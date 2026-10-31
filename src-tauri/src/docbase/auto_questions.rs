//! The questions about the doubtful steps of the cutter (D-606), of «quién
//! es quién» and of the summaries of the pages that follow a change in
//! what was read (D-484): they go by themselves while the automatic
//! reading is on, «En curso» says how many they are, and when a round of
//! them passes `CAP` the app waits for a person before going on («Seguir»
//! or «Dejarlas para después»).
//!
//! A round lasts until no stage has questions left. How many were asked in
//! it lives in memory (`EngineState`); what a person said of it is kept in
//! the base, so that «para después» survives closing the app. «En curso»
//! also says what every question to the model since the round began was
//! for, as the base keeps them (`model_answer.task` and `asked_at`), and
//! what the answers about cuts did to their steps.

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::progress::EngineNow;
use super::{sql_err, DocBase};

/// Past this many questions in a round, a person says whether to go on.
pub const CAP: usize = 50;

/// The stages whose questions make the round, in the order they ask: the
/// doubtful steps of the cutter, the identities and the pages.
pub const STAGES: [&str; 3] = ["cut", "resolve", "pages"];

/// What a person said of the round (`docbase_setting`).
const SETTING: &str = "auto_questions";

/// What a person said of a round over the cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Permit {
    /// Go on with the round as it was then, `total` questions; if it grows
    /// past that by more than `CAP`, the app asks again.
    Allowed { total: usize },
    /// Left for later: nothing of it is asked until a person says go on.
    Later,
}

pub(crate) fn permit(conn: &Connection) -> Result<Option<Permit>, String> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [SETTING], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(value.as_deref().and_then(parse))
}

fn parse(value: &str) -> Option<Permit> {
    if value == "later" {
        return Some(Permit::Later);
    }
    value.strip_prefix("allowed:")?.parse().ok().map(|total| Permit::Allowed { total })
}

/// The questions the round has left, as each of its stages last counted them.
pub(crate) fn pending(now: &EngineNow) -> usize {
    STAGES.iter().map(|stage| now.left.get(stage).copied().unwrap_or(0)).sum()
}

/// Whether a round with `asked` questions asked and `pending` left may go on.
pub(crate) fn may_go(asked: usize, pending: usize, permit: Option<Permit>) -> bool {
    if pending == 0 {
        return true;
    }
    let total = asked + pending;
    match permit {
        None => total <= CAP,
        Some(Permit::Allowed { total: allowed }) => total <= allowed + CAP,
        Some(Permit::Later) => false,
    }
}

/// The questions to the model of one task since the round began, by
/// themselves or because a person asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskedFor {
    /// `cut`, `type`, `knowledge`, `identity`, `synthesis`…
    pub task: String,
    pub asked: usize,
}

/// What the answers about cuts asked since the round began did to their
/// steps (D-606).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CutEffect {
    /// Another document starts there: a new cut.
    pub starts: usize,
    /// It goes on, and the answer backs it: no longer uncertain.
    pub backed: usize,
    /// It goes on, with an answer of low confidence: still uncertain.
    pub uncertain: usize,
}

/// The round as «En curso» shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoQuestions {
    /// Questions left, as last counted.
    pub pending: usize,
    /// Of them, those about the doubtful steps of the cutter.
    pub cut_pending: usize,
    /// Asked since the round began.
    pub asked: usize,
    pub cap: usize,
    /// The stage whose questions wait for a person, when one does.
    pub held: Option<&'static str>,
    /// What a person said of the round: `allowed`, `later` or nothing.
    pub decision: Option<&'static str>,
    /// Every question to the model since the round began, by task, the
    /// most first.
    pub asked_for: Vec<AskedFor>,
    pub cut_effect: CutEffect,
}

impl AutoQuestions {
    pub(crate) fn of(conn: &Connection, now: &EngineNow, permit: Option<Permit>) -> Result<Self, String> {
        let since = now.round_since;
        let mut statement = conn
            .prepare("SELECT task, count(*) FROM model_answer WHERE asked_at > ?1 GROUP BY task ORDER BY count(*) DESC, task")
            .map_err(sql_err)?;
        let asked_for = statement
            .query_map([since], |row| Ok(AskedFor { task: row.get(0)?, asked: row.get::<_, i64>(1)? as usize }))
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        let low = super::cut::LOW_CONFIDENCE;
        let cut_effect = conn
            .query_row(
                &format!(
                    "SELECT coalesce(sum(s.model_says = 1), 0), coalesce(sum(s.model_says = 0 AND coalesce(s.model_confidence, '') <> '{low}'), 0),
                            coalesce(sum(s.model_says = 0 AND s.model_confidence = '{low}'), 0)
                     FROM cut_step s WHERE s.verdict = 'doubtful' AND s.model_says IS NOT NULL
                       AND EXISTS (SELECT 1 FROM model_answer a WHERE a.question = s.question AND a.task = ?2 AND a.asked_at > ?1)"
                ),
                params![since, super::cut::CUT_TASK],
                |row| {
                    Ok(CutEffect {
                        starts: row.get::<_, i64>(0)? as usize,
                        backed: row.get::<_, i64>(1)? as usize,
                        uncertain: row.get::<_, i64>(2)? as usize,
                    })
                },
            )
            .map_err(sql_err)?;
        Ok(Self {
            pending: pending(now),
            cut_pending: now.left.get("cut").copied().unwrap_or(0),
            asked: now.auto_asked,
            cap: CAP,
            held: now.auto_held,
            decision: permit.map(|permit| match permit {
                Permit::Allowed { .. } => "allowed",
                Permit::Later => "later",
            }),
            asked_for,
            cut_effect,
        })
    }
}

impl DocBase {
    /// Whether the questions of `stage` (`cut`, `resolve` or `pages`) may be asked
    /// now (D-484). Unless a person allowed the round and it did not grow
    /// past that, it counts them first, asking nothing: with the answers
    /// kept, what the resolution or the pages would still ask. Says which
    /// stage waits for a person.
    pub(crate) fn auto_questions_go(&self, stage: &'static str) -> Result<bool, String> {
        let state = self.engine_state();
        let permit = permit(&self.conn())?;
        let now = state.snapshot();
        let known = now.left.get(stage).copied().unwrap_or(0);
        let recount = match permit {
            Some(Permit::Allowed { total }) => now.auto_asked + pending(&now) > total + CAP,
            // Left for later: counted again only once what was counted is gone.
            Some(Permit::Later) => known == 0,
            None => true,
        };
        if recount {
            if stage == "cut" {
                state.ran(stage, self.cut_questions()?.by_itself);
            } else if stage == "resolve" {
                let report = self.knowledge_resolve(None, 0)?;
                state.ran(stage, report.left);
            } else {
                let report = self.knowledge_project(None, 0)?;
                // Waiting for the resolution: nothing to count yet, nor to ask.
                if report.reason.as_deref() == Some("resolving") {
                    return Ok(true);
                }
                state.ran(stage, report.left);
            }
        }
        self.auto_questions_judge(stage)
    }

    /// With the counts of now, whether `stage` may ask; says which waits.
    pub(crate) fn auto_questions_judge(&self, stage: &'static str) -> Result<bool, String> {
        let state = self.engine_state();
        let now = state.snapshot();
        let own = now.left.get(stage).copied().unwrap_or(0);
        let round_goes = may_go(now.auto_asked, pending(&now), permit(&self.conn())?);
        let go = own == 0 || round_goes;
        // A stage with nothing to ask leaves another's wait as it is.
        if !go {
            state.auto_held(Some(stage));
        } else if round_goes || now.auto_held == Some(stage) {
            state.auto_held(None);
        }
        Ok(go)
    }

    /// What a person says of the round (D-484): `go`, its questions are
    /// asked now, as many as there are; otherwise they wait until a person
    /// says go on.
    pub fn auto_questions_decide(&self, go: bool) -> Result<(), String> {
        let state = self.engine_state();
        let now = state.snapshot();
        let value = if go { format!("allowed:{}", now.auto_asked + pending(&now)) } else { "later".to_string() };
        self.conn()
            .execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![SETTING, value])
            .map_err(sql_err)?;
        if go {
            state.auto_held(None);
        }
        Ok(())
    }

    /// Nothing left to ask in any stage: the round is over, and what a
    /// person said of it goes with it.
    pub(crate) fn auto_questions_round_over(&self) -> Result<(), String> {
        self.engine_state().auto_round_over();
        self.conn().execute("DELETE FROM docbase_setting WHERE key = ?1", [SETTING]).map_err(sql_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::knowledge::tests::{base_with, is_knowledge, type_answer, FakeModel};
    use crate::docbase::progress::Scope;
    use crate::source_volume::asking::AskConfig;

    // Invented documents (R-24: nothing here comes from the test folder).
    const LETTER: &str = "CARTA DE PRUEBA\n\nPersona Uno Ejemplo le escribe a Persona Dos Ejemplo por el inmueble del domicilio inventado, \
para dejar constancia de la entrega de las llaves el día del vencimiento.";
    const RECEIPT: &str = "RECIBO DE PRUEBA\n\nPersona Dos Ejemplo recibió de Persona Uno Ejemplo la suma inventada por el inmueble del \
domicilio inventado, sin observaciones de ninguna de las partes.";

    fn model() -> FakeModel {
        FakeModel::new("codex-cli:m", |prompt| {
            if is_knowledge(prompt) {
                serde_json::json!({ "menciones": [], "afirmaciones": [], "importes": [] }).to_string()
            } else {
                type_answer("carta")
            }
        })
    }

    /// Two documents read, their names still to join, with the reading on.
    fn read_base() -> (crate::docbase::testing::TempDir, DocBase) {
        let (dir, base) = base_with(&[("carta.txt", LETTER), ("recibo.txt", RECEIPT)]);
        base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "m".into(), ..Default::default() })).unwrap();
        base.set_background_reading(true).unwrap();
        let asker = model();
        base.types_run(&asker, 50).unwrap();
        base.knowledge_run(&asker, 50).unwrap();
        (dir, base)
    }

    fn resolve_stage(base: &DocBase) -> crate::docbase::progress::Stage {
        base.progress(&Scope::default()).unwrap().stages.into_iter().find(|s| s.id == "resolve").unwrap()
    }

    #[test]
    fn a_round_goes_by_itself_up_to_the_cap_and_past_it_only_when_a_person_allows_it() {
        assert!(may_go(0, 0, Some(Permit::Later)));
        assert!(may_go(0, 50, None));
        assert!(!may_go(0, 51, None));
        // What was asked in the round counts too.
        assert!(!may_go(30, 21, None));
        // Allowed: the round as it was, and up to 50 more that come up.
        assert!(may_go(10, 90, Some(Permit::Allowed { total: 60 })));
        assert!(!may_go(10, 101, Some(Permit::Allowed { total: 60 })));
        assert!(!may_go(0, 1, Some(Permit::Later)));
        assert_eq!(parse("allowed:60"), Some(Permit::Allowed { total: 60 }));
        assert_eq!(parse("later"), Some(Permit::Later));
        assert_eq!(parse("otra cosa"), None);
    }

    #[test]
    fn with_nothing_to_ask_the_round_goes_on_and_counting_asks_nothing() {
        let (_dir, base) = read_base();
        // The documents name nobody: joining them asks the model nothing.
        assert!(base.auto_questions_go("resolve").unwrap());
        assert_eq!(base.engine_state().snapshot().left.get("resolve"), Some(&0));
        assert_eq!(base.engine_state().snapshot().auto_held, None);
        let shown = base.progress(&Scope::default()).unwrap().auto_questions;
        assert_eq!(shown, AutoQuestions { pending: 0, asked: 0, cap: CAP, held: None, decision: None, ..shown.clone() });
    }

    #[test]
    fn past_the_cap_it_waits_for_a_person_who_lets_it_go_on_or_leaves_it_for_later() {
        let (_dir, base) = read_base();
        let state = base.engine_state();
        assert_eq!(resolve_stage(&base).state, "running");

        // 30 to ask: they go by themselves.
        state.ran("resolve", 30);
        assert!(base.auto_questions_judge("resolve").unwrap());
        // 60: «En curso» says how many and that they wait for a person.
        state.ran("resolve", 60);
        assert!(!base.auto_questions_judge("resolve").unwrap());
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(progress.auto_questions, AutoQuestions { pending: 60, asked: 0, cap: CAP, held: Some("resolve"), decision: None, ..progress.auto_questions.clone() });
        let stage = resolve_stage(&base);
        assert_eq!((stage.state, stage.extra.get("held")), ("paused", Some(&1)));

        // A person lets it go on: the 60, and some more that come up.
        base.auto_questions_decide(true).unwrap();
        assert_eq!(resolve_stage(&base).state, "running");
        assert!(base.auto_questions_judge("resolve").unwrap());
        state.auto_asked(40);
        state.ran("resolve", 20);
        state.ran("pages", 45);
        assert!(base.auto_questions_judge("pages").unwrap());
        let shown = base.progress(&Scope::default()).unwrap().auto_questions;
        assert_eq!((shown.pending, shown.asked, shown.decision), (65, 40, Some("allowed")));
        // Many more than were allowed: it asks again.
        state.ran("pages", 100);
        assert!(!base.auto_questions_judge("pages").unwrap());

        // The round ends: what was said goes with it.
        state.ran("resolve", 0);
        state.ran("pages", 0);
        base.auto_questions_round_over().unwrap();
        let shown = base.progress(&Scope::default()).unwrap().auto_questions;
        assert_eq!(shown, AutoQuestions { pending: 0, asked: 0, cap: CAP, held: None, decision: None, ..shown.clone() });
        // A new round: the questions asked before are not its own (D-606).
        assert!(shown.asked_for.is_empty(), "{:?}", shown.asked_for);

        // Left for later: even a few wait, until a person says go on; it
        // survives closing the app.
        base.auto_questions_decide(false).unwrap();
        state.ran("resolve", 5);
        assert!(!base.auto_questions_go("resolve").unwrap());
        assert_eq!(permit(&base.conn()).unwrap(), Some(Permit::Later));
        assert_eq!(base.progress(&Scope::default()).unwrap().auto_questions.decision, Some("later"));
        base.auto_questions_decide(true).unwrap();
        assert!(base.auto_questions_judge("resolve").unwrap());
    }
}
