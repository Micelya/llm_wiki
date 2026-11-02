//! The loop of the process (E4 of 2 nov, in place of A2): the only thing
//! that does the work of the base. There is no barrier and no other thread.
//!
//! ```text
//! while the app is open:
//!     take the wakes (a change in a folder, a person, a retry that came due)
//!     step = the first step with something pending (`process_state`)
//!     if there is one: do ONE small operation of it, and keep it
//!     if not:          sleep until the next wake
//! ```
//!
//! - **One small operation per turn,** and back to the first step: a few
//!   files of bytes or text, one PDF read by zones, one question. No
//!   operation is cut in half: the wakes are read only at the start of a
//!   turn, and what they ask is in the base.
//! - **The gap is closed by construction:** the next operation is chosen
//!   after the last one was kept, from the base. An answer that changes the
//!   key data of some documents leaves the similar documents incomplete,
//!   and the next operation measures them, never a question.
//! - **Every start is the same** (change B of 2 nov): the first load, a new
//!   folder, the sweep and a person. What a person asks for that touches
//!   the process (read again, ask the model about the cut of one PDF, the
//!   run of the types of the screen) is kept in the base as an ask with its
//!   scope; the loop looks at the asks first, with the same steps and the
//!   same operations over that scope: it waits only for the operation going
//!   on and the steps before of its own document (decision 2 of 2 nov).
//! - **The questions** go out only when every step before is complete
//!   (D-774, point 1, D-781): the question of the cut waits for the work
//!   without AI of its file. The round with a cap (D-484, D-606) is their
//!   permission in the steps 8, 17 and 19: past its cap, the step is set
//!   aside waiting for a person (in the 17 also the 18 and the 19), and the
//!   rest goes on. The vectors (step 11, C1 of 2 nov) are no question: they
//!   are not in the round.
//! - **The failures** stay over their element with their class and their
//!   waits (D-714, P-533): what failed is set aside until its next attempt,
//!   or until a person; a failure of the whole engine sets aside every
//!   question to it, and the steps without AI go on.
//! - **Going back is counted** (D-774, point 2): a document whose key data
//!   change again after it was measured, or a file whose parts are made
//!   again, goes back to an earlier step; past two times it is set aside
//!   for a person, with its failure (`failures::BACK_STAGE`).
//! - **It sleeps** until a wake: a person, a change of the settings, the
//!   sweep of backup, or the next attempt of a failure or of a paused
//!   engine.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;

use super::steps::{Class, Scope, Step, STEPS};
use super::{Looking, ProcessState};
use crate::docbase::failures::{self, About, Failure};
use crate::docbase::watch::Watch;
use rusqlite::OptionalExtension;
use crate::docbase::{sql_err, DocBase, Visible};
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::recognition::TextRecognizer;

const HASH_BATCH: usize = 8;
const TEXT_BATCH: usize = 2;
const SIMILAR_BATCH: usize = 2;
/// Readings checked again from the answers kept, per operation.
const RECHECK_BATCH: usize = 4;
/// Documents waiting for recognition whose pages are counted at a time.
const SCAN_PAGES_BATCH: usize = 2;
/// PDFs whose ficha of shape is computed at a time.
const PROFILE_BATCH: usize = 4;
/// PDFs cut by the rules, or whose parts are made, at a time (D-556).
const CUT_BATCH: usize = 4;
/// The sweep of backup of every folder, for the folders of the network and
/// the cloud that do not tell (the watcher, `watch.rs`, A3 of 2 nov).
pub const SWEEP_EVERY: Duration = Duration::from_secs(5 * 60);
/// How often the screens are told that something changed, at most.
const NOTIFY_EVERY: Duration = Duration::from_millis(800);
/// After an error of the base in a turn, how long before the next one.
const AFTER_ERROR: Duration = Duration::from_secs(30);
/// When the scans of this computer try again after their engine stopped.
const RECOGNITION_RETRY: Duration = Duration::from_secs(5 * 60);
/// How many times an element may go back to an earlier step; one more and
/// it is set aside for a person (D-774, point 2).
pub const BACK_LIMIT: u32 = 2;
/// The asks of a person kept in the base (`docbase_setting`), one row each.
const ASK_PREFIX: &str = "process:ask:";

/// How the rest of the app wakes the loop up.
#[derive(Default)]
pub struct Wake {
    /// (something happened, a sweep is asked for).
    flags: Mutex<(bool, bool)>,
    signal: Condvar,
}

impl Wake {
    /// A sweep of every folder now (a folder added, the portals, the server).
    pub fn request_sweep(&self) {
        let mut flags = self.flags.lock().unwrap_or_else(|p| p.into_inner());
        *flags = (true, true);
        self.signal.notify_all();
    }

    /// Something a person did or a setting changed: the loop looks again.
    pub fn request(&self) {
        self.flags.lock().unwrap_or_else(|p| p.into_inner()).0 = true;
        self.signal.notify_all();
    }

    pub(crate) fn take(&self) -> (bool, bool) {
        std::mem::take(&mut *self.flags.lock().unwrap_or_else(|p| p.into_inner()))
    }

    pub(crate) fn wait(&self, timeout: Duration) {
        let guard = self.flags.lock().unwrap_or_else(|p| p.into_inner());
        if !guard.0 {
            let _ = self.signal.wait_timeout(guard, timeout);
        }
    }
}

/// What a person asks for that touches the process: one more start of it,
/// with its scope (change B of 2 nov).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// «Preguntar al modelo» where each document of a PDF starts.
    Cut,
    /// The run of the types of the screen of the types: their type and data.
    Types,
    /// Read again what a person asked to read again (D-432).
    Reading,
    /// Recognize the text of a scan a person asked for, with the reader
    /// chosen in Ajustes then or the one of «Leer con IA» (E2 of 2 nov).
    /// Kept in `recognition_request`, with its reader.
    Recognition,
}

impl Ask {
    fn key(self) -> &'static str {
        match self {
            Ask::Cut => "cut",
            Ask::Types => "types",
            Ask::Reading => "reading",
            Ask::Recognition => "recognition",
        }
    }

    fn parse(key: &str) -> Option<Ask> {
        [Ask::Cut, Ask::Types, Ask::Reading, Ask::Recognition].into_iter().find(|a| a.key() == key)
    }

    /// The steps whose questions it lets go out, with the reading off.
    fn steps(self) -> &'static [Step] {
        match self {
            Ask::Cut => &[Step::CutQuestion],
            Ask::Types => &[Step::Type, Step::TypeData],
            Ask::Reading => &[Step::Reading],
            Ask::Recognition => &[Step::Scans],
        }
    }
}

/// An ask kept in the base, with its scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub ask: Ask,
    pub scope: Scope,
}

fn scope_key(scope: &Scope) -> String {
    match scope {
        Scope::All => "all".into(),
        Scope::Document(version) => format!("document:{version}"),
        Scope::File(hash) => format!("file:{hash}"),
        Scope::Folder { origin, folder } => format!("folder:{origin}|{folder}"),
    }
}

fn parse_scope(key: &str) -> Option<Scope> {
    let (kind, rest) = key.split_once(':').unwrap_or((key, ""));
    match kind {
        "all" => Some(Scope::All),
        "document" => rest.parse().ok().map(Scope::Document),
        "file" => Some(Scope::File(rest.to_string())),
        "folder" => rest.split_once('|').map(|(o, f)| Scope::Folder { origin: o.into(), folder: f.into() }),
        _ => None,
    }
}

/// How the loop stands, for «En curso» (A3 draws it).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoopStatus {
    /// `working`, `stopped` (everything complete), `waiting-person` (the
    /// round past its cap) or `waiting` (only what waits for a retry or a
    /// paused engine is left).
    pub state: &'static str,
    /// The number of the step of the last operation.
    pub step: Option<u8>,
    /// What the last operation did, in words.
    pub operation: Option<String>,
    /// The scope of the last operation: «todo», or the ask of a person.
    pub scope: Option<String>,
    /// Why it went back to an earlier step the last time.
    pub back: Option<String>,
    /// Why it waits, when it does.
    pub waiting_for: Option<String>,
    /// Operations done since the app opened.
    pub turns: u64,
}

/// What the loop keeps in memory between turns.
#[derive(Default)]
pub(crate) struct Memory {
    status: LoopStatus,
    /// The step of the last operation over the whole base.
    last_step: Option<Step>,
    /// Steps set aside as a whole now, with why: the round past its cap,
    /// or a step whose operation could not go on.
    aside: BTreeMap<Step, String>,
    /// How many times each element went back to an earlier step.
    back: BTreeMap<String, u32>,
    /// The pass over the steps that know what they have left only by going
    /// over them (identities, matters, summaries, D-452): the next one to go
    /// over, and whether a whole pass did nothing.
    pass: usize,
    settled: bool,
    /// Why the process has to start again from the first step: a change the
    /// watcher heard, or a person who asked to process something again.
    /// «En curso» says it with the next going back.
    cause: Option<String>,
}

/// What one turn did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Turn {
    /// The step of its operation.
    pub step: Option<Step>,
    /// It did something: the next turn comes at once.
    pub worked: bool,
    /// Pages of the wiki were written.
    pub pages: bool,
}

/// What an operation did.
#[derive(Debug, Clone, Copy, Default)]
struct Done {
    worked: bool,
    pages: bool,
}

impl Done {
    fn when(worked: bool) -> Done {
        Done { worked, pages: false }
    }
}

/// The steps that know what they have left only by going over them.
const PASSES: [Step; 3] = [Step::Identity, Step::Matters, Step::Pages];

impl DocBase {
    fn memory(&self) -> std::sync::MutexGuard<'_, Memory> {
        self.process.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// How the loop stands now, for «En curso».
    pub fn process_status(&self) -> LoopStatus {
        self.memory().status.clone()
    }

    /// Something happened outside the loop (a person, a setting, a sweep):
    /// what was set aside for now is looked at again, and the steps that
    /// know what they have left only by going over them are gone over again.
    pub(crate) fn process_woken(&self) {
        let mut memory = self.memory();
        memory.aside.clear();
        memory.pass = 0;
        memory.settled = false;
    }

    /// A person asks the process for `ask` over `scope`: kept in the base
    /// until its steps are complete in that scope.
    pub fn process_ask(&self, ask: Ask, scope: &Scope) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT OR IGNORE INTO docbase_setting (key, value) VALUES (?1, ?2)",
                rusqlite::params![format!("{ASK_PREFIX}{}:{}", ask.key(), scope_key(scope)), crate::docbase::now_ms().to_string()],
            )
            .map_err(sql_err)?;
        self.process_woken();
        Ok(())
    }

    /// The asks of a person waiting, the oldest first: those kept as asks,
    /// and the documents a person asked to read again.
    pub fn process_asks(&self) -> Result<Vec<Asked>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!("SELECT key FROM docbase_setting WHERE key LIKE '{ASK_PREFIX}%' ORDER BY CAST(value AS INTEGER), key"))
            .map_err(sql_err)?;
        let keys: Vec<String> = statement.query_map([], |row| row.get(0)).map_err(sql_err)?.collect::<Result<_, _>>().map_err(sql_err)?;
        let mut asks: Vec<Asked> = keys
            .iter()
            .filter_map(|key| {
                let (ask, scope) = key.strip_prefix(ASK_PREFIX)?.split_once(':')?;
                Some(Asked { ask: Ask::parse(ask)?, scope: parse_scope(scope)? })
            })
            .collect();
        for standing in crate::docbase::knowledge::extract::standings(&conn, &Visible::All, None)? {
            if standing.reading.as_ref().is_some_and(|r| r.reread) {
                asks.push(Asked { ask: Ask::Reading, scope: Scope::Document(standing.content_id) });
            }
        }
        // The scans a person asked to read: each waits only for its own
        // document (E2 of 2 nov; the ask of a person, decision 2 of 2 nov).
        for content in crate::docbase::recognize::requested_recognitions(&conn)? {
            asks.push(Asked { ask: Ask::Recognition, scope: Scope::Document(content) });
        }
        Ok(asks)
    }

    fn ask_done(&self, asked: &Asked) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM docbase_setting WHERE key = ?1", [format!("{ASK_PREFIX}{}:{}", asked.ask.key(), scope_key(&asked.scope))])
            .map_err(sql_err)?;
        Ok(())
    }

    /// The state of the process over `scope` as the loop looks at it: the
    /// steps set aside for now, and the steps an ask lets go out.
    fn looking_at(&self, scope: &Scope, allowed: &[Step]) -> Result<ProcessState, String> {
        let aside: BTreeSet<Step> = self.memory().aside.keys().copied().collect();
        self.process_state_looking(scope, &Looking { allowed: allowed.to_vec(), aside, first_only: true })
    }

    /// One turn of the loop: ONE small operation of the first incomplete
    /// step, the asks of a person first. `asker`: the engine of «Preguntar
    /// al modelo» (none: the questions wait, set aside).
    pub fn process_turn(&self, asker: Option<&dyn DocumentAsker>) -> Result<Turn, String> {
        // What has no step of its own and costs nothing (a document made
        // from a template, a text of an older extraction, what is left of
        // a file that went away, the pages of the scans to count).
        let tidy = self.generated_settle().unwrap_or_else(|error| {
            eprintln!("[docbase] generated documents: {error}");
            0
        }) + self.refresh_extraction(TEXT_BATCH)?
            + self.purge_orphans()?
            + self.count_scan_pages(SCAN_PAGES_BATCH)?
            // The scans that waited for the models of this computer when
            // they are not needed any more (another reader chosen, or the
            // models came): back to their step.
            + self.scans_models_settle()?;
        if tidy > 0 {
            return Ok(Turn { step: None, worked: true, pages: false });
        }

        // The asks of a person, each over its scope.
        for asked in self.process_asks()? {
            let allowed = asked.ask.steps();
            let state = self.looking_at(&asked.scope, allowed)?;
            let last = *allowed.last().unwrap_or(&Step::Inventory);
            let mut first = state.first_incomplete;
            // «Preguntar al modelo» about one PDF takes also the doubtful
            // steps the app does not ask by itself (the cut of before, D-622),
            // once the steps before of its file are complete.
            if asked.ask == Ask::Cut && first.is_none_or(|s| s > Step::CutQuestion) {
                let files = self.files_of(&asked.scope)?;
                if self.cut_doubtful_left(Some(&files))? > 0 {
                    first = Some(Step::CutQuestion);
                }
            }
            match first {
                Some(step) if step <= last => {
                    let cut = (step == Step::CutQuestion).then(|| self.files_of(&asked.scope)).transpose()?;
                    let left = |files: &Option<BTreeSet<String>>| files.as_ref().map(|f| self.cut_doubtful_left(Some(f))).transpose();
                    let before = left(&cut)?;
                    let done = self.operate(step, &asked.scope, Some(asked.ask), asker)?;
                    // A question a person asked for that failed is asked
                    // once: the failure is in Atención, and asking again is
                    // the person's (no spin over a model that keeps failing).
                    if before.is_some() && left(&cut)? >= before {
                        self.ask_done(&asked)?;
                    }
                    if done.worked {
                        self.note_turn(step, &format!("lo pedido por una persona ({})", scope_key(&asked.scope)));
                        return Ok(Turn { step: Some(step), worked: true, pages: done.pages });
                    }
                }
                // Its steps are complete in its scope: the ask is done.
                _ => self.ask_done(&asked)?,
            }
        }

        // The whole base. Each step that cannot go on is set aside for this
        // turn, so the loop ends.
        for _ in 0..=STEPS.len() * 2 {
            let state = self.looking_at(&Scope::All, &[])?;
            let Some(step) = state.first_incomplete.or(self.next_pass()?) else {
                return self.nothing_left();
            };
            self.went_back(step);
            let done = self.operate(step, &Scope::All, None, asker)?;
            if done.worked {
                self.note_turn(step, "todo");
                return Ok(Turn { step: Some(step), worked: true, pages: done.pages });
            }
            // It could not go on: set aside for now, and the steps after go
            // on (what it waits for is in «En curso»).
            self.set_aside(step, state.first_incomplete == Some(step));
        }
        self.nothing_left()
    }

    /// The next step of the pass over the identities, the matters and the
    /// summaries, when everything counted is complete and a pass is due.
    fn next_pass(&self) -> Result<Option<Step>, String> {
        {
            let memory = self.memory();
            if memory.settled {
                return Ok(None);
            }
        }
        // Not while the questions are held (the reading off, no engine, the
        // engine paused): nothing of a pass may go out then.
        let reading_on = self.background_reading()?;
        if super::steps::asking_held(&self.conn(), reading_on)?.is_some() {
            return Ok(None);
        }
        let memory = self.memory();
        Ok(PASSES.iter().skip(memory.pass).find(|s| !memory.aside.contains_key(s)).copied())
    }

    /// A step did nothing: in the pass, the pass goes to the next step; a
    /// counted step is set aside for now (the round may have set it aside
    /// already, waiting for a person).
    fn set_aside(&self, step: Step, counted: bool) {
        let mut memory = self.memory();
        if !counted {
            memory.pass = PASSES.iter().position(|s| *s == step).map(|p| p + 1).unwrap_or(PASSES.len());
            if memory.pass >= PASSES.len() {
                memory.settled = true;
            }
            return;
        }
        if memory.aside.contains_key(&step) {
            return;
        }
        memory.aside.insert(step, "no pudo seguir por ahora".into());
        drop(memory);
        if step.about().class == Class::Local {
            // A step without AI that counts something left and cannot do
            // it: told in Atención, it does not stop the rest.
            let mut failure = Failure::new(
                "thread",
                About::Thread(format!("process/{}", step.about().key)),
                "A step without AI counts work left that its operation does not take.",
            );
            failure.cause = failures::cause("program-error");
            failures::note_once(&self.conn(), &failure).unwrap_or_else(|error| eprintln!("[docbase] {error}"));
        }
    }

    /// Nothing more can be done now: the round of questions is over when
    /// none is left, and «En curso» says why it stopped.
    fn nothing_left(&self) -> Result<Turn, String> {
        let aside: Vec<String> = self.memory().aside.values().cloned().collect();
        let person = aside.iter().any(|why| why.contains("persona"));
        if aside.is_empty() && crate::docbase::auto_questions::pending(&self.engine_state().snapshot()) == 0 {
            self.auto_questions_round_over()?;
        }
        let waiting = failures::next_attempt(&self.conn())?.is_some();
        let mut memory = self.memory();
        memory.status.state = if person {
            "waiting-person"
        } else if waiting || !aside.is_empty() {
            "waiting"
        } else {
            "stopped"
        };
        memory.status.waiting_for = (!aside.is_empty()).then(|| aside.join("; "));
        Ok(Turn::default())
    }

    fn note_turn(&self, step: Step, scope: &str) {
        let mut memory = self.memory();
        if step <= Step::Reading {
            memory.pass = 0;
            memory.settled = false;
        }
        // What was set aside for now is looked at again after any work.
        memory.aside.retain(|_, why| why.contains("persona"));
        // A step without AI that could not go on and now did: its notice
        // in Atención is resolved.
        let _ = failures::resolve(&self.conn(), "thread", &About::Thread(format!("process/{}", step.about().key)));
        if scope == "todo" {
            memory.last_step = Some(step);
        }
        memory.status.state = "working";
        memory.status.step = Some(step.number());
        memory.status.operation = Some(format!("paso {} de {}, {}", step.number(), STEPS.len(), step.about().name.to_lowercase()));
        memory.status.scope = Some(scope.to_string());
        memory.status.waiting_for = None;
        memory.status.turns += 1;
    }

    /// The loop goes to `step`, earlier than the one of its last operation:
    /// «En curso» says why.
    fn went_back(&self, step: Step) {
        let mut memory = self.memory();
        let because = |memory: &mut Memory| memory.cause.take().map(|why| format!(": {why}")).unwrap_or_default();
        if let Some(last) = memory.last_step.filter(|last| step < *last) {
            let why = if step == Step::Inventory { because(&mut memory) } else { String::new() };
            memory.status.back = Some(format!(
                "volvió al paso {} ({}) después del paso {} ({}){why}",
                step.number(),
                step.about().name.to_lowercase(),
                last.number(),
                last.about().name.to_lowercase()
            ));
        } else if step == Step::Inventory && memory.cause.is_some() {
            // It was at the start already, or stopped: the cause is told all
            // the same.
            let why = because(&mut memory);
            memory.status.back = Some(format!("empezó de nuevo por el paso 1 ({}){why}", step.about().name.to_lowercase()));
        }
    }

    /// The elements that go back to `step` now, by what they are in the
    /// record of failures: the documents measured whose key data changed
    /// since (the similar documents), and the files whose parts were made
    /// and are to be made again.
    fn going_back(&self, step: Step) -> Result<Vec<About>, String> {
        Ok(match step {
            Step::Similar => crate::docbase::similar::store::stale_by_key_data_now(&self.conn())?.into_iter().map(About::Version).collect(),
            Step::Parts => {
                let conn = self.conn();
                let mut statement = conn
                    .prepare(&format!(
                        "SELECT f.hash FROM {} WHERE {} AND EXISTS (SELECT 1 FROM cut_applied ca WHERE ca.resource_hash = f.hash)",
                        crate::docbase::cut::parts::PARTS_FROM,
                        crate::docbase::cut::parts::parts_pending_sql()
                    ))
                    .map_err(sql_err)?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(sql_err)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?.into_iter().map(About::File).collect()
            }
            _ => Vec::new(),
        })
    }

    /// Counts the elements that went back to `step` with this operation
    /// (they were going back before it and are not after): past
    /// `BACK_LIMIT` times, each is set aside for a person, with its failure.
    fn count_back(&self, step: Step, before: Vec<About>) -> Result<(), String> {
        if before.is_empty() {
            return Ok(());
        }
        let after = self.going_back(step)?;
        for element in before.into_iter().filter(|e| !after.contains(e)) {
            let key = format!("{}:{element:?}", step.about().key);
            let times = {
                let mut memory = self.memory();
                let times = memory.back.entry(key).or_default();
                *times += 1;
                *times
            };
            if times > BACK_LIMIT {
                let message = format!(
                    "Volvió {times} veces al paso {} ({}): se aparta para que lo mire una persona.",
                    step.number(),
                    step.about().name.to_lowercase()
                );
                let mut failure = Failure::new(failures::BACK_STAGE, element, &message);
                failure.cause = failures::cause("goes-back");
                failures::note_once(&self.conn(), &failure)?;
            }
        }
        Ok(())
    }

    /// ONE small operation of `step` over `scope`. `ask`: a person asked
    /// for it (its questions go out with the reading off).
    fn operate(&self, step: Step, scope: &Scope, ask: Option<Ask>, asker: Option<&dyn DocumentAsker>) -> Result<Done, String> {
        let state = self.engine_state();
        if step.about().class == Class::Asks && asker.is_none() {
            return Ok(Done::default());
        }
        let back = self.going_back(step)?;
        // The marks of the steps that close by taking them away, before the
        // round counts (its count may take them away too).
        let marked = match step {
            Step::Identity => self.marks("scope IN ('resolve', 'retire')")?,
            Step::Pages => self.marks("scope = 'project'")?,
            _ => 0,
        };
        let done = match step {
            Step::Inventory => Done::when(match scope {
                Scope::Folder { origin, .. } => self.sweep_origins(Some(origin.as_str())),
                // The origins whose inventory is incomplete: never swept, or
                // changed after their last sweep (the watcher, «Volver a
                // procesar»). The sweep of backup goes over all of them.
                _ => {
                    let mut swept = false;
                    for origin in self.origins_to_sweep()? {
                        swept |= self.sweep_origins(Some(origin.as_str()));
                    }
                    swept
                }
            }),
            Step::FileHash => Done::when(self.hash_pending(HASH_BATCH)? > 0),
            Step::Text => Done::when(self.extract_pending(TEXT_BATCH)? > 0),
            Step::Scans => Done::when(self.scans_operation(scope, ask)?),
            Step::Shapes => Done::when(self.profile_pending(PROFILE_BATCH)? > 0),
            Step::Zones => Done::when(self.zones_operation()?),
            Step::CutRules => Done::when(self.local_or_note("cut", self.cut_pending(CUT_BATCH))),
            Step::Parts => Done::when(self.local_or_note("parts", self.cut_parts_pending(CUT_BATCH))),
            Step::Signature | Step::Similar => Done::when(self.similar_step(SIMILAR_BATCH)? > 0),
            // Up to 96 documents, one call to Micelya Core (C1 of 2 nov).
            Step::Vectors => Done::when(self.vectors_operation()?),
            Step::CutQuestion => {
                let asker = asker.expect("checked above");
                // The round is the permission of what goes by itself; what a
                // person asks for goes out (it is counted all the same).
                if ask.is_none() && !self.round_lets(step, "cut")? {
                    return Ok(Done::default());
                }
                state.at_stage(Some("types"));
                let only: Option<BTreeSet<String>> = match scope {
                    Scope::All => None,
                    _ => Some(self.files_of(scope)?),
                };
                let report = self.cut_ask_run(asker, 1, only.as_ref())?;
                if only.is_none() {
                    state.ran("cut", report.left);
                }
                state.auto_asked(report.asked);
                Done::when(report.asked + report.kept + report.failed > 0 || report.paused_until.is_some())
            }
            Step::Type | Step::TypeData => {
                state.at_stage(Some("types"));
                let report = self.types_run(asker.expect("checked above"), 1)?;
                state.ran("types", report.left);
                Done::when(report.asked + report.data_asked + report.typed + report.corrected + report.failed + report.data_failed > 0)
            }
            Step::TemplateFields => {
                state.at_stage(Some("templates"));
                let report = self.templates_naming_run(asker.expect("checked above"), 1)?;
                state.ran("templates", report.left);
                Done::when(report.asked + report.named + report.failed > 0)
            }
            Step::Reading => {
                state.at_stage(Some("reading"));
                if self.knowledge_recheck(RECHECK_BATCH)? > 0 {
                    return Ok(Done::when(true));
                }
                let asker = asker.expect("checked above");
                let requested = ask == Some(Ask::Reading);
                let report = if requested { self.knowledge_run_requested(asker, 1)? } else { self.knowledge_run(asker, 1)? };
                state.ran("reading", report.left);
                // What a person asked to read again is resolved once it is
                // all read, without asking.
                if requested && report.extracted + report.unusable > 0 && self.knowledge_rereads_waiting()? == 0 {
                    self.knowledge_resolve(None, 0)?;
                }
                Done::when(report.asked + report.extracted + report.unusable + report.failed > 0)
            }
            Step::Identity => {
                if !self.round_lets(step, "resolve")? {
                    return Ok(Done::default());
                }
                state.at_stage(Some("resolve"));
                let report = self.knowledge_resolve(asker, 1)?;
                state.ran("resolve", report.left);
                state.auto_asked(report.asked);
                // A resolution that left nothing takes its marks away: the
                // step is complete, that is work too.
                Done::when(report.asked + report.changed + report.failed > 0 || self.marks("scope IN ('resolve', 'retire')")? < marked)
            }
            Step::Matters => {
                state.at_stage(Some("matters"));
                let report = self.knowledge_propose_matters(asker, 1)?;
                if !report.skipped {
                    state.ran("matters", report.left);
                }
                Done::when(report.asked + report.failed > 0)
            }
            Step::Pages => {
                if !self.round_lets(step, "pages")? {
                    return Ok(Done::default());
                }
                state.at_stage(Some("pages"));
                let report = self.knowledge_project(asker, 1)?;
                if report.reason.as_deref() != Some("resolving") {
                    state.ran("pages", report.left);
                }
                state.auto_asked(report.asked);
                let cleared = self.marks("scope = 'project'")? < marked;
                Done { worked: report.wrote() || report.asked + report.failed > 0 || cleared, pages: report.wrote() }
            }
        };
        if done.worked {
            self.count_back(step, back)?;
        }
        Ok(done)
    }

    /// The round with a cap (D-484, D-606) as the permission of the steps
    /// 8, 17 and 19: past its cap, the step waits for a person (the 17 also
    /// holds the 18 and the 19), and the rest goes on.
    fn round_lets(&self, step: Step, stage: &'static str) -> Result<bool, String> {
        if self.auto_questions_go(stage)? {
            return Ok(true);
        }
        let why = "esperando a una persona: la ronda de preguntas llegó a su tope (¿seguir?)".to_string();
        let mut memory = self.memory();
        memory.aside.insert(step, why.clone());
        if step == Step::Identity {
            memory.aside.insert(Step::Matters, why.clone());
            memory.aside.insert(Step::Pages, why);
        }
        Ok(false)
    }

    /// How many marks of the knowledge layer of `which` (an SQL condition)
    /// wait (`knowledge_dirty`).
    fn marks(&self, which: &str) -> Result<i64, String> {
        self.conn().query_row(&format!("SELECT count(*) FROM knowledge_dirty WHERE {which}"), [], |row| row.get(0)).map_err(sql_err)
    }

    /// The files of a scope, by their hash.
    fn files_of(&self, scope: &Scope) -> Result<BTreeSet<String>, String> {
        let conn = self.conn();
        let mut statement = conn.prepare(&format!("SELECT DISTINCT f.hash FROM file f WHERE {}", scope.file_sql("f.hash"))).map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// A step without AI whose error went only to the console: written down.
    fn local_or_note(&self, stage: &str, result: Result<usize, String>) -> bool {
        match result {
            Ok(done) => done > 0,
            Err(error) => {
                eprintln!("[docbase] {stage}: {error}");
                self.note(&Failure::new(stage, About::Thread(format!("step/{stage}")), &error));
                false
            }
        }
    }

    /// One scan read: WHERE THE READER IS USED (E2 of 2 nov, D-764). The
    /// reader is the one the request of a person was made with, or the one
    /// chosen in Ajustes (`services::recognition_choice`, the one of this
    /// computer by default); nothing here names one. With the one of this
    /// computer it is local work; with one in the cloud it is a question,
    /// with its failures (over the document, with their class and waits,
    /// D-714), its round (`round_lets`) and its quota (a limit of the
    /// provider is a transient failure). The text it gives is the text of
    /// the document; the shape of the pages (zones and lines, which the cut
    /// needs) is read on this computer in the step of the zones, whatever
    /// reader is chosen (`zones_operation`).
    fn scans_operation(&self, scope: &Scope, ask: Option<Ask>) -> Result<bool, String> {
        use crate::docbase::recognize::{scan_without_models, RecognitionStep, RECOGNITION_STAGE};
        use crate::docbase::services::is_local;
        let only = match (ask, scope) {
            (Some(Ask::Recognition), Scope::Document(id)) => Some(*id),
            _ => None,
        };
        let Some(content) = self.next_recognition_of(only)? else {
            return Ok(false);
        };
        let requested = self.reader_of_request(content)?.is_some();
        let Some(reader) = self.reader_for(content)? else {
            return Ok(false);
        };
        let local = is_local(&reader.engine);
        let engine = crate::source_volume::recognition::engine_id(&reader)?;
        let fake = test_reader(&engine);
        if local {
            // Without its models the scans wait with their failure; none
            // goes to the cloud instead (D-775).
            if let (None, Some(why)) = (&fake, crate::source_volume::recognition_local::unavailable()) {
                let noted = self.scans_without_models(&why, false)?;
                return Ok(noted > 0 || scan_without_models(&self.conn(), content, &why, false)?);
            }
        } else if !requested && !self.round_lets(Step::Scans, "recognition")? {
            // What goes by itself in the cloud needs the round; what a
            // person asked for goes out (it is counted all the same).
            return Ok(false);
        }
        let real = if fake.is_none() { Some(crate::source_volume::recognition::recognizer_for(&reader)?) } else { None };
        let recognizer: &dyn TextRecognizer = match (&fake, &real) {
            (Some(fake), _) => fake.as_ref(),
            (None, Some(real)) => real.as_ref(),
            (None, None) => return Ok(false),
        };
        let state = self.engine_state();
        let pages = state.snapshot().scan_pages.get(&content).copied().unwrap_or(1).max(1);
        let step = self.recognition_step_of(content, recognizer, RECOGNITION_RETRY)?;
        if !local {
            if !matches!(step, RecognitionStep::Skipped(_) | RecognitionStep::Idle) {
                state.auto_asked(pages);
            }
            state.ran("recognition", self.scan_questions_left()?);
        }
        Ok(match step {
            RecognitionStep::Idle => false,
            RecognitionStep::Failed(content) => {
                // Marked until a person asks again, with its reason.
                let error: Option<String> =
                    self.conn().query_row("SELECT error FROM version WHERE id = ?1", [content], |row| row.get(0)).map_err(sql_err)?;
                let message = error.unwrap_or_else(|| "The text of the document could not be recognized.".to_string());
                let mut failure = Failure::new(RECOGNITION_STAGE, About::Version(content), &message);
                if failure.cause.class == failures::Class::Transient {
                    failure.cause = failures::cause("unknown");
                }
                failure.engine = Some(&engine);
                self.note(&failure);
                true
            }
            RecognitionStep::Stopped(error) => {
                // The provider failed (its limit, no connection, no answer):
                // the scan is set aside until its next attempt (D-714).
                let mut failure = Failure::new(RECOGNITION_STAGE, About::Version(content), &error);
                failure.engine = Some(&engine);
                self.note(&failure);
                true
            }
            _ => true,
        })
    }

    /// One PDF read by zones on this computer. Without the model of zones,
    /// each PDF waiting says so with its failure and waits (D-717).
    fn zones_operation(&self) -> Result<bool, String> {
        if let Some(why) = crate::source_volume::layout_local::unavailable() {
            return Ok(self.zones_without_models(&why)? > 0);
        }
        self.zones_with_models()?;
        self.read_zones_step()
    }

    /// Sweeps every folder (or one), the inventory of the step 1. Returns
    /// whether anything may look different.
    pub(crate) fn sweep_origins(&self, only: Option<&str>) -> bool {
        let origins = match self.origins(&Visible::All) {
            Ok(origins) => origins,
            Err(error) => {
                eprintln!("[docbase] {error}");
                return false;
            }
        };
        let mut swept = false;
        for origin in origins.iter().filter(|o| only.is_none_or(|id| o.id == id)) {
            self.engine_state().sweeping(Some(&origin.id));
            match self.scan_origin(&origin.id) {
                Ok(_) => {
                    let _ = failures::resolve(&self.conn(), "sweep", &About::Origin(origin.id.clone()));
                }
                Err(error) => {
                    eprintln!("[docbase] sweep of '{}' failed: {error}", origin.name);
                    self.note(&Failure::new("sweep", About::Origin(origin.id.clone()), &error));
                }
            }
            swept = true;
        }
        self.engine_state().sweeping(None);
        swept
    }

    /// The origins whose inventory is incomplete now (`steps::INVENTORY_LEFT`).
    fn origins_to_sweep(&self) -> Result<Vec<String>, String> {
        let conn = self.conn();
        let mut statement =
            conn.prepare(&format!("SELECT o.id FROM origin o WHERE {}", super::steps::INVENTORY_LEFT)).map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)
    }

    /// Something changed in the folder of `origin` (the watcher heard it and
    /// it calmed down): its inventory is incomplete until a sweep newer than
    /// the change, and the loop starts again from the first step after the
    /// operation going on. «En curso» says why.
    pub fn origin_changed(&self, origin: &str) -> Result<(), String> {
        let name: Option<String> = self
            .conn()
            .query_row("SELECT name FROM origin WHERE id = ?1", [origin], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        let Some(name) = name else {
            return Ok(());
        };
        self.conn()
            .execute("UPDATE origin SET changed_at = ?1 WHERE id = ?2", rusqlite::params![crate::docbase::now_ms(), origin])
            .map_err(sql_err)?;
        self.memory().cause = Some(format!("un archivo nuevo o cambiado en «{name}»"));
        self.process_woken();
        Ok(())
    }

    /// «Volver a procesar» (A3 of 2 nov): a person asks to process again a
    /// file, a folder or everything. It is one more start of the same
    /// process, with no way of its own: the inventory of that scope is left
    /// incomplete (the origins, to be swept again, and their files, to have
    /// their bytes read again instead of trusting their size and date), and
    /// the loop does the rest in its order. A step after it is done again
    /// only where what it was made with changed (its seal); reading again
    /// with the AI is «Leer de nuevo», with its cost. Returns how many files
    /// are to be read again.
    pub fn process_again(&self, again: &Again) -> Result<usize, String> {
        let now = crate::docbase::now_ms();
        let conn = self.conn();
        let (files, origins) = match (&again.origin_id, &again.rel_path) {
            (None, _) => (
                conn.execute("UPDATE location SET hash_state = 'pending', hash_error = NULL WHERE kind = 'file'", []).map_err(sql_err)?,
                conn.execute("UPDATE origin SET changed_at = ?1", [now]).map_err(sql_err)?,
            ),
            (Some(origin), Some(rel_path)) => (
                conn.execute(
                    "UPDATE location SET hash_state = 'pending', hash_error = NULL WHERE kind = 'file' AND origin_id = ?1 AND rel_path = ?2",
                    rusqlite::params![origin, rel_path],
                )
                .map_err(sql_err)?,
                conn.execute("UPDATE origin SET changed_at = ?1 WHERE id = ?2", rusqlite::params![now, origin]).map_err(sql_err)?,
            ),
            (Some(origin), None) => {
                let folder = again.folder.as_deref().unwrap_or("").trim_matches('/');
                (
                    conn.execute(
                        "UPDATE location SET hash_state = 'pending', hash_error = NULL
                         WHERE kind = 'file' AND origin_id = ?1 AND (?2 = '' OR substr(rel_path, 1, length(?2) + 1) = ?2 || '/')",
                        rusqlite::params![origin, folder],
                    )
                    .map_err(sql_err)?,
                    conn.execute("UPDATE origin SET changed_at = ?1 WHERE id = ?2", rusqlite::params![now, origin]).map_err(sql_err)?,
                )
            }
        };
        drop(conn);
        if origins == 0 {
            return Err("Ese origen ya no existe".to_string());
        }
        let folder = again.folder.as_deref().unwrap_or("").trim_matches('/');
        let what = match (&again.origin_id, &again.rel_path) {
            (None, _) => "todo".to_string(),
            (Some(_), Some(rel_path)) => format!("el archivo «{}»", rel_path.rsplit('/').next().unwrap_or(rel_path)),
            (Some(_), None) if !folder.is_empty() => format!("la carpeta «{folder}»"),
            (Some(origin), None) => {
                let name: Option<String> =
                    self.conn().query_row("SELECT name FROM origin WHERE id = ?1", [origin], |row| row.get(0)).optional().map_err(sql_err)?;
                format!("la carpeta «{}»", name.unwrap_or_default())
            }
        };
        self.memory().cause = Some(format!("pediste volver a procesar {what}"));
        self.process_woken();
        Ok(files)
    }
}

/// What a person asks to process again: everything (no origin), a folder
/// of an origin (`folder`, empty for all of it) or a file (`rel_path`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Again {
    pub origin_id: Option<String>,
    pub folder: Option<String>,
    pub rel_path: Option<String>,
}

#[cfg(test)]
thread_local! {
    /// The readers of the tests, by the name they keep their text with: the
    /// step of the scans reads with these instead of the real ones (stand-in
    /// readers, no real model and no models of this computer).
    pub(crate) static TEST_READERS: std::cell::RefCell<Vec<std::rc::Rc<dyn TextRecognizer>>> = Default::default();
}

/// The reader of a test with this name, if one was given.
fn test_reader(engine: &str) -> Option<std::rc::Rc<dyn TextRecognizer>> {
    #[cfg(test)]
    {
        TEST_READERS.with(|readers| readers.borrow().iter().find(|r| r.engine_id() == engine).cloned())
    }
    #[cfg(not(test))]
    {
        let _ = engine;
        None
    }
}

/// The engine of «Preguntar al modelo», as an asker, when one is chosen.
fn asker_now(base: &DocBase) -> Option<Box<dyn DocumentAsker>> {
    let engine = crate::docbase::services::ask_engine(&base.conn()).ok().flatten()?;
    crate::source_volume::asking::asker_for(&engine).ok()
}

/// How long the loop may sleep when nothing is left: until the sweep of
/// backup, or the next attempt of a failure, whatever comes first.
fn sleep_for(base: &DocBase, last_sweep: Instant, watch: &Watch) -> Duration {
    let sweep = SWEEP_EVERY.saturating_sub(last_sweep.elapsed());
    let retry = failures::next_attempt(&base.conn())
        .ok()
        .flatten()
        .map(|at| Duration::from_millis(at.saturating_sub(crate::docbase::now_ms()).max(0) as u64));
    let calm = watch.next_calm();
    [retry, calm].into_iter().flatten().fold(sweep, Duration::min).max(Duration::from_millis(200))
}

/// The folders of the origins, for the watcher.
fn origin_folders(base: &DocBase) -> Vec<(String, std::path::PathBuf)> {
    base.origins(&Visible::All).map(|origins| origins.iter().map(|o| (o.id.clone(), o.root())).collect()).unwrap_or_default()
}

/// The changes the watcher heard that calmed down: written down in the base,
/// and the loop starts again from the first step. Whether there was one.
pub(crate) fn take_calmed(base: &DocBase, watch: &Watch) -> bool {
    let calmed = watch.calmed();
    for origin in &calmed {
        if let Err(error) = base.origin_changed(origin) {
            eprintln!("[docbase] {error}");
        }
    }
    !calmed.is_empty()
}

/// Starts the loop, the only thread of the work of the base. `on_change`
/// is called, at most a few times per second, when something the screens
/// show may have changed; `on_pages`, when pages of the wiki were written.
pub fn spawn(base: Arc<DocBase>, wake: Arc<Wake>, on_change: impl Fn() + Send + 'static, on_pages: impl Fn() + Send + 'static) {
    let for_note = base.clone();
    let spawned = std::thread::Builder::new().name("docbase-process".to_string()).spawn(move || run(&base, &wake, &on_change, &on_pages));
    if let Err(error) = spawned {
        eprintln!("[docbase] could not start the process: {error}");
        note_thread(&for_note, &error.to_string());
    }
}

fn run(base: &DocBase, wake: &Arc<Wake>, on_change: &dyn Fn(), on_pages: &dyn Fn()) {
    run_looked(base, wake, on_change, on_pages, &mut NoLooker);
}

/// What looks at the loop from outside: the control run (A4 of 2 nov)
/// writes down each turn and counts the questions; the app looks with
/// `NoLooker`. It never changes what a turn does: it can only count the
/// questions of the engine of Ajustes, end the loop (`after`, `idle`), and
/// do what a person does on the screen.
pub(crate) trait Looker {
    /// The engine of «Preguntar al modelo» of this turn, as Ajustes says.
    fn asker(&mut self, asker: Option<Box<dyn DocumentAsker>>) -> Option<Box<dyn DocumentAsker>> {
        asker
    }
    /// Just before a turn.
    fn before(&mut self, _base: &DocBase) {}
    /// Just after a turn; false ends the loop.
    fn after(&mut self, _base: &DocBase, _turn: &Result<Turn, String>) -> bool {
        true
    }
    /// The watcher heard a change that calmed down.
    fn heard(&mut self, _base: &DocBase) {}
    /// Nothing to do now: the loop is to sleep `sleep`; false ends it.
    fn idle(&mut self, _base: &DocBase, _sleep: Duration) -> bool {
        true
    }
}

/// The app: nobody looks.
struct NoLooker;

impl Looker for NoLooker {}

/// The loop of the app, looked at by `looker`.
pub(crate) fn run_looked(base: &DocBase, wake: &Arc<Wake>, on_change: &dyn Fn(), on_pages: &dyn Fn(), looker: &mut dyn Looker) {
    let mut last_sweep: Option<Instant> = None;
    let mut last_notice = Instant::now();
    let mut unannounced = false;
    // The watcher (A3 of 2 nov): its notices only write down when a folder
    // changed, and wake this loop; the change is taken here, at the start
    // of a turn, once the folder calmed down.
    let mut watch = Watch::start(wake.clone());
    loop {
        let (woken, sweep) = wake.take();
        if woken {
            base.process_woken();
        }
        if take_calmed(base, &watch) {
            looker.heard(base);
            unannounced = true;
        }
        if sweep || last_sweep.is_none_or(|at| at.elapsed() >= SWEEP_EVERY) {
            unannounced |= base.sweep_origins(None);
            last_sweep = Some(Instant::now());
            // The folders of the origins, watched again after each sweep: a
            // new origin, one removed, one that came back.
            watch.follow(&origin_folders(base));
            base.process_woken();
            // The readings of this computer left in Micelya Core are tried
            // again with the sweep (P-294); nothing is sent when none.
            unannounced |= crate::micelya_core::ledger::retry_left_readings(false);
        }
        // A panic in one operation does not end the loop: it is written
        // down as an error of the program, and the loop waits a little.
        let asker = looker.asker(asker_now(base));
        looker.before(base);
        let turn = crate::panic_guard::run_guarded("docbase_process_turn", || base.process_turn(asker.as_deref()));
        if !looker.after(base, &turn) {
            return;
        }
        let worked = match turn {
            Ok(turn) => {
                if turn.pages {
                    on_pages();
                }
                turn.worked
            }
            Err(error) => {
                eprintln!("[docbase] {error}");
                note_thread(base, &error);
                unannounced |= true;
                wake.wait(AFTER_ERROR);
                continue;
            }
        };
        unannounced |= worked;
        if unannounced && (!worked || last_notice.elapsed() >= NOTIFY_EVERY) {
            on_change();
            last_notice = Instant::now();
            unannounced = false;
        }
        if !worked {
            let sleep = sleep_for(base, last_sweep.unwrap_or_else(Instant::now), &watch);
            if !looker.idle(base, sleep) {
                return;
            }
            wake.wait(sleep);
        }
    }
}

/// Writes down a turn that failed or panicked: an error of the program,
/// unless its message says otherwise.
fn note_thread(base: &DocBase, error: &str) {
    let mut failure = Failure::new("thread", About::Thread("docbase-process".to_string()), error);
    if failure.cause.code == "unknown" {
        failure.cause = failures::cause("program-error");
    }
    base.note(&failure);
}

/// Every step of the process over the whole base, with how the loop
/// stands: what «En curso» needs (A3 draws it).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessView {
    pub status: LoopStatus,
    pub state: ProcessState,
    /// The places, files and documents of the base, by the same rule as
    /// Inicio (`query::totals`): a file of 3 parts is 3 documents.
    pub totals: crate::docbase::query::Totals,
    /// Why each step set aside as a whole now is (the round past its cap
    /// waits for a person), by its number.
    pub aside: BTreeMap<u8, String>,
}

impl DocBase {
    /// The 19 steps over the whole base and the loop. Reads only.
    pub fn process_view(&self) -> Result<ProcessView, String> {
        let reasons: BTreeMap<u8, String> = self.memory().aside.iter().map(|(step, why)| (step.number(), why.clone())).collect();
        let aside: BTreeSet<Step> = self.memory().aside.keys().copied().collect();
        let state = self.process_state_looking(&Scope::All, &Looking { aside, ..Looking::default() })?;
        debug_assert_eq!(state.steps.len(), STEPS.len());
        let totals = crate::docbase::query::totals(&self.conn(), &Visible::All)?;
        Ok(ProcessView { status: self.process_status(), state, totals, aside: reasons })
    }
}

#[cfg(test)]
#[path = "../worker_control.rs"]
mod control;
