//! The cutter of the PDFs that join several documents (D-556): rules
//! first, and the model only in the doubtful steps.
//!
//! For every PDF of two pages or more that has its ficha of shape and the
//! text of its pages, the background worker keeps what the cutter says of
//! each step from one page to the next (`cut_step`): the verdict
//! (`continue`, `new` or `doubtful`), the rule that gave it and the type
//! the next page suggests. The rules ask nothing, so they always run; a
//! file is cut again when the rules, the catalog (`cut.json`) or the text
//! of its pages change. In an even file (`rules::uniform`) a step no rule
//! decided goes on and is not asked. A doubtful step counts as `continue`
//! until the model answers, with the two pages as images: by itself, while
//! the automatic reading is on, only in a file where a rule already says
//! another document starts (D-622), in the round of questions with a cap
//! (`auto_questions.rs`); in the others only when a person asks
//! (`cut_ask_file`), with the reading on or off. The answer is kept in `model_answer` (task `cut`) by the version of the
//! instruction and the exact prints of the two pages: the same pages in
//! another file are not asked again.
//!
//! The steps are turned into versions and appearances in `parts.rs`.

mod ask;
pub(crate) mod by_zones;
mod catalog;
pub mod parts;
mod rules;
mod signals;

#[cfg(test)]
pub(crate) mod check;
#[cfg(test)]
mod check_zones;
#[cfg(test)]
mod check_rules;
#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Instant;

use rusqlite::{params, Connection, OptionalExtension};

use crate::panic_guard::run_guarded;
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::page_shape::{PdfShapes, TextLayer, SHAPES_ENGINE};
use crate::source_volume::{LocalFolderProvider, SourceProvider};

use super::failures;
use super::page_shapes::SHAPES_KIND;
use super::similar::classify::{cached_answer, stops_the_run, ASKING};
use super::text::EXTRACTION_ENGINE;
use super::{now_ms, sql_err, DocBase};

pub use ask::{CutAnswer, CUT_TASK};
pub use rules::Verdict;

/// The version of the rules. With the catalog's, what the kept steps were
/// decided with: another one decides them again.
pub const RULES_VERSION: &str = "cut-rules/2";

pub fn rules_version() -> String {
    format!("{RULES_VERSION}+cut.json/{}", catalog::catalog().version)
}

/// The confidence of an answer that does not back a step.
pub const LOW_CONFIDENCE: &str = "low";

/// The confidence of an answer as the base keeps it: one of the three the
/// question allows, nothing otherwise.
fn known_confidence(confidence: &str) -> Option<String> {
    ["high", "medium", LOW_CONFIDENCE].contains(&confidence).then(|| confidence.to_string())
}

/// What the cutter says of one step.
#[derive(Debug, Clone, PartialEq)]
pub struct CutStep {
    /// The step from page `after` to the next; 0 is the start of the file.
    pub after: u32,
    pub verdict: Verdict,
    pub rule: String,
    /// The key of the question about the two pages, in a doubtful step
    /// whose pages have prints.
    pub question: Option<String>,
    /// What the model said: whether the next page starts another document.
    pub model_says: Option<bool>,
    /// How sure the model said it was: `high`, `medium` or `low`.
    pub model_confidence: Option<String>,
    /// The type the vocabulary finds in the next page, and the one the
    /// model gave.
    pub rule_type: Option<String>,
    pub model_type: Option<String>,
    /// Decided by the cut by zones (`by_zones`): over windows of pages,
    /// where only an answer of high confidence backs a step.
    pub zones: bool,
    /// The hints of the rules of the cut by zones (JSON), in its steps.
    pub hints: Option<String>,
}

impl CutStep {
    /// Whether the model's answer decides the step: in a doubtful step, and
    /// in one the even file let go on when an answer about its two pages
    /// comes later (an answer weighs more than that rule, D-615).
    pub fn by_model(&self) -> bool {
        self.model_says.is_some() && (self.verdict == Verdict::Doubtful || self.rule == rules::UNIFORM)
    }

    /// Whether a document starts at the next page: as the model says when
    /// its answer decides, as the rules say otherwise; doubtful without an
    /// answer counts as continue.
    pub fn starts(&self) -> bool {
        if self.by_model() {
            return self.model_says == Some(true);
        }
        self.verdict == Verdict::New
    }

    /// Whether nothing backs what the step says (D-603): a doubtful step
    /// the model did not answer, or answered with low confidence. A rule
    /// other than `none` (the even file too) or an answer of high or
    /// medium confidence backs it; a person's word backs any step, and
    /// wins over it when the parts are made (`parts.rs`).
    /// In the cut by zones only a high one backs it: a medium one is
    /// uncertain too.
    pub fn uncertain(&self) -> bool {
        if self.by_model() {
            if self.zones {
                return self.model_confidence.as_deref() != Some("high");
            }
            return self.model_confidence.as_deref() == Some(LOW_CONFIDENCE);
        }
        self.verdict == Verdict::Doubtful
    }

    pub fn suggested_type(&self) -> Option<&str> {
        self.model_type.as_deref().or(self.rule_type.as_deref())
    }
}

/// What a run of questions about the doubtful steps did.
#[derive(Debug, Clone, Default)]
pub struct CutAskReport {
    pub asked: usize,
    /// Answered from an answer kept (the same pages in another file).
    pub kept: usize,
    pub failed: usize,
    /// Doubtful steps still without an answer, of the files the run went
    /// over (by itself: those of `ASKED_BY_ITSELF`).
    pub left: usize,
    pub seconds: f64,
    pub errors: Vec<String>,
    /// Until when the engine is not asked: a failure of the whole engine
    /// (its limit, a 429, no connection, the credential), D-714.
    pub paused_until: Option<i64>,
}

/// The files whose doubtful steps the thread that asks the model takes by
/// itself (D-622): those where a rule already says another document starts
/// after the first page. There it is known that the file joins several
/// documents, and the doubt is where each one ends. In the others a
/// doubtful step goes on uncertain (D-603) and is asked only when a person
/// asks (`cut_ask_file`). `s` is the step.
pub(crate) const ASKED_BY_ITSELF: &str =
    "EXISTS (SELECT 1 FROM cut_step r WHERE r.resource_hash = s.resource_hash AND r.verdict = 'new' AND r.after_page > 0)";

/// A doubtful step still without an answer, with the key of its question,
/// that no person decided: a step a person decided is never asked again
/// (the step of the question of the cut, A1 of 2 nov). A person decides the
/// page after the step (`cut_decision`).
pub(crate) const WITHOUT_ANSWER: &str = "s.verdict = 'doubtful' AND s.model_says IS NULL AND s.question IS NOT NULL
     AND NOT EXISTS (SELECT 1 FROM cut_decision k WHERE k.resource_hash = s.resource_hash AND k.page = s.after_page + 1)";

/// What `note_failure` writes down for the cut of a file and for its parts.
const CUT_STAGE: &str = "cut";
const PARTS_STAGE: &str = "parts";

/// The time of the text of the pages of the file `f`: the file is cut
/// again when it changes.
const STAMP_SQL: &str = "(SELECT max(t.created_at) FROM resource_text t WHERE t.resource_hash = f.hash)";

/// The files the cutter reads: `f`, with its version `c`, at a location
/// `l` (a PDF) of a reachable origin `o`; with `cuttable_where`.
/// `z` is its reading by zones, when it has one (`by_zones`).
const CUTTABLE_FROM: &str = "file f JOIN version c ON c.id = f.version_id
     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done' AND lower(l.ext) = 'pdf'
     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
     LEFT JOIN derivative z ON z.version_id = c.id AND z.kind = 'zones'";

/// SQL: the file of `CUTTABLE_FROM` is read, at a location not excluded,
/// and has the text of its pages or its reading by zones.
fn cuttable_where() -> String {
    format!(
        "c.state IN ('processed', 'needs-recognition') AND NOT {}
         AND (z.body IS NOT NULL OR EXISTS (SELECT 1 FROM resource_text t WHERE t.resource_hash = f.hash))",
        super::excluded_sql!()
    )
}

/// SQL: the steps of the file `f` were decided with the rules `rules` (an
/// SQL value) and the text its pages have now, or its cut failed with
/// them (`note_failure`): it is not cut again until they change.
/// A file with its reading by zones `z` is cut by it (`by_zones`), with
/// its version and the time of that reading.
fn cut_settled_sql(rules: &str) -> String {
    let zones = format!("'{}'", by_zones::zones_version());
    format!(
        "(CASE WHEN z.created_at IS NOT NULL THEN
             (EXISTS (SELECT 1 FROM cut_step s WHERE s.resource_hash = f.hash AND s.after_page = 0
                  AND s.rules_version = {zones} AND s.text_stamp = z.created_at)
              OR {failed_zones})
          ELSE
             (EXISTS (SELECT 1 FROM cut_step s WHERE s.resource_hash = f.hash AND s.after_page = 0
                  AND s.rules_version = {rules} AND s.text_stamp = {STAMP_SQL})
              OR {failed}) END)",
        failed = failed_sql(CUT_STAGE, "f.hash", &format!("{rules} || '|' || {STAMP_SQL}")),
        failed_zones = failed_sql(CUT_STAGE, "f.hash", &format!("{zones} || '|' || z.created_at"))
    )
}

/// A piece of the background work over one file or document that failed
/// (the cut of a file, its parts, the signature of a document): written
/// down in the record of failures with its cause and class and with what
/// it was done over (`basis`, D-714). It is not done again over that
/// basis while the failure holds (a transient one, until its next
/// attempt). D-488 still holds: a failure never holds the rest back. But
/// the file waits (D-717): it is not read whole, nor cut with the cut of
/// before (`failures::file_waits_sql`).
pub(crate) fn note_failure(conn: &Connection, stage: &str, item: &str, basis: &str, error: &str) -> Result<(), String> {
    eprintln!("[docbase] {stage} of {item}: {error}");
    let about = match item.parse::<i64>() {
        Ok(id) if stage == SIGN_STAGE => failures::About::Version(id),
        _ => failures::About::File(item.to_string()),
    };
    let mut failure = failures::Failure::new(stage, about, error);
    failure.basis = Some(basis);
    failures::record(conn, &failure)?;
    Ok(())
}

/// The step of the signature of a document (`similar::store`): its item
/// is a version.
const SIGN_STAGE: &str = "similar";

/// The work of `stage` over `item` came out well: its failure, if it had
/// one, is resolved.
pub(crate) fn note_success(conn: &Connection, stage: &str, item: &str) -> Result<(), String> {
    let about = match item.parse::<i64>() {
        Ok(id) if stage == SIGN_STAGE => failures::About::Version(id),
        _ => failures::About::File(item.to_string()),
    };
    failures::resolve(conn, stage, &about)
}

/// SQL: the work of `stage` over `item` failed over `basis` (both SQL
/// values), as `note_failure` wrote it down, and the failure holds.
pub(crate) fn failed_sql(stage: &str, item: &str, basis: &str) -> String {
    failures::holds_sql(stage, &format!("CAST({item} AS TEXT)"), basis)
}

/// The doubtful steps still without an answer (D-606).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CutQuestions {
    /// The thread that asks the model takes them by itself.
    pub by_itself: usize,
    /// They wait for a person to ask (`cut_ask_file`).
    pub on_request: usize,
}

/// The text of each page as the cutter reads it: a recognizer's when the
/// ficha sends the page to recognition or its layer is not the page's
/// own, the extraction otherwise.
fn page_texts(conn: &Connection, hash: &str, shapes: &PdfShapes) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare("SELECT page_no, engine, body FROM resource_text WHERE resource_hash = ?1 ORDER BY page_no, created_at")
        .map_err(sql_err)?;
    let mut extracted: BTreeMap<u32, String> = BTreeMap::new();
    let mut recognized: BTreeMap<u32, String> = BTreeMap::new();
    let rows = statement
        .query_map([hash], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
        .map_err(sql_err)?;
    for row in rows {
        let (page, engine, body) = row.map_err(sql_err)?;
        if engine == EXTRACTION_ENGINE {
            extracted.insert(page, body);
        } else {
            recognized.insert(page, body);
        }
    }
    Ok(shapes
        .pages
        .iter()
        .map(|p| {
            let prefer_read = p.read || p.text_layer != TextLayer::Digital;
            let read = recognized.get(&p.page).filter(|_| prefer_read);
            read.or(extracted.get(&p.page)).cloned().unwrap_or_default()
        })
        .collect())
}

/// Where a file can be read, from its first location in a reachable origin.
fn file_path(conn: &Connection, hash: &str) -> Result<Option<PathBuf>, String> {
    let found: Option<(String, String)> = conn
        .query_row(
            &format!(
                "SELECT o.location, min(l.rel_path) FROM location l JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                 WHERE l.file_hash = ?1 AND l.hash_state = 'done' AND NOT {} GROUP BY l.file_hash",
                super::excluded_sql!()
            ),
            [hash],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(sql_err)?;
    Ok(found.and_then(|(root, rel)| LocalFolderProvider::new(root).local_path(&rel)))
}

fn keep_cut_answer(conn: &Connection, key: &str, engine: &str, version_id: Option<i64>, answer: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO model_answer (question, task, prompt_version, engine, version_id, prompt, answer, asked_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![key, CUT_TASK, catalog::catalog().ask.version, engine, version_id, ask::prompt(), answer, now_ms()],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Puts an answer in every step asked with its key: what the model says,
/// the type of the next page and, for the page before when nothing gave
/// it, the type of the first page.
fn apply_answer(conn: &Connection, key: &str, answer: &CutAnswer) -> Result<(), String> {
    conn.execute(
        "UPDATE cut_step SET model_says = ?1, model_type = coalesce(?2, model_type), model_confidence = ?4 WHERE question = ?3",
        params![answer.second_starts_new_document, CutAnswer::known(&answer.second_page_type), key, known_confidence(&answer.confidence)],
    )
    .map_err(sql_err)?;
    if let Some(first) = CutAnswer::known(&answer.first_page_type) {
        conn.execute(
            "UPDATE cut_step SET model_type = ?1
             WHERE model_type IS NULL AND (resource_hash, after_page + 1) IN (SELECT resource_hash, after_page FROM cut_step WHERE question = ?2)",
            params![first, key],
        )
        .map_err(sql_err)?;
    }
    Ok(())
}

/// How many files are still to cut with the rules (`cut_pending`) and
/// `free` (an SQL condition over the version `c`) says do not hold back
/// what is set aside (`process::steps::free_sql`).
pub(crate) fn rules_left(conn: &rusqlite::Connection, free: &str) -> Result<usize, String> {
    conn.query_row(
        &format!(
            "SELECT count(DISTINCT f.hash) FROM {CUTTABLE_FROM}
             LEFT JOIN derivative d ON d.version_id = c.id AND d.kind = ?1 AND d.engine = ?2
             WHERE {cuttable} AND NOT {settled} AND {free}
               AND CASE WHEN z.body IS NOT NULL THEN json_array_length(z.body) >= 2
                   ELSE d.body IS NOT NULL AND json_array_length(d.body, '$.pages') >= 2 AND NOT {waiting} END",
            cuttable = cuttable_where(),
            settled = cut_settled_sql("?3"),
            waiting = by_zones::zones_waiting_sql(),
        ),
        params![SHAPES_KIND, SHAPES_ENGINE, rules_version()],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n as usize)
    .map_err(sql_err)
}

impl DocBase {
    /// Cuts up to `limit` PDF files whose steps are missing or were decided
    /// with other rules, another catalog or another text of their pages.
    /// Only the rules: a doubtful step takes an answer already kept for
    /// its two pages, and nothing is asked. Returns how many were cut.
    pub fn cut_pending(&self, limit: usize) -> Result<usize, String> {
        let version = rules_version();
        type Job = (String, i64, String, String, Option<String>, Option<i64>, Option<String>, Option<i64>);
        let jobs: Vec<Job> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(&format!(
                    "SELECT f.hash, f.version_id, o.location, min(l.rel_path), d.body, {STAMP_SQL} AS stamp, z.body, z.created_at
                     FROM {CUTTABLE_FROM}
                     LEFT JOIN derivative d ON d.version_id = c.id AND d.kind = ?1 AND d.engine = ?2
                     WHERE {cuttable} AND NOT {settled}
                       AND CASE WHEN z.body IS NOT NULL THEN json_array_length(z.body) >= 2
                           ELSE d.body IS NOT NULL AND json_array_length(d.body, '$.pages') >= 2 AND NOT {waiting} END
                     GROUP BY f.hash ORDER BY f.hash LIMIT ?4",
                    cuttable = cuttable_where(),
                    settled = cut_settled_sql("?3"),
                    waiting = by_zones::zones_waiting_sql(),
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params![SHAPES_KIND, SHAPES_ENGINE, version, limit as i64], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?))
                })
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (hash, version_id, root, rel_path, body, stamp, zones, zones_stamp) in &jobs {
            // A file with its reading by zones is cut by it (D-668).
            if let (Some(zones), Some(zones_stamp)) = (zones, zones_stamp) {
                let path = LocalFolderProvider::new(root.clone()).local_path(rel_path).unwrap_or_default();
                let engine = super::services::ask_engine(&self.conn())?.map(|c| crate::source_volume::asking::engine_id(&c));
                match self.cut_by_zones(hash, &path, zones, *zones_stamp, engine.as_deref()) {
                    Ok(()) => note_success(&self.conn(), CUT_STAGE, hash)?,
                    Err(error) => {
                        note_failure(&self.conn(), CUT_STAGE, hash, &format!("{}|{zones_stamp}", by_zones::zones_version()), &error)?
                    }
                }
                continue;
            }
            let body = body.clone().unwrap_or_default();
            let stamp = stamp.unwrap_or_default();
            // One file at a time: a cut that fails is written down and the
            // others go on (D-488); that file waits (D-717), it is not read
            // whole.
            let cut = || -> Result<(), String> {
                let shapes: PdfShapes = serde_json::from_str(&body).unwrap_or_default();
                let path = LocalFolderProvider::new(root.clone()).local_path(rel_path).unwrap_or_default();
                // A PDF pdfium cannot open is cut by its text alone; one in
                // use by another program or in the cloud without being
                // downloaded waits for it (a transient failure, D-714).
                let scans = match run_guarded("docbase_cut", || signals::scan_pages(&path)) {
                    Ok(scans) => scans,
                    Err(error) if path.exists() => {
                        let error = failures::pdf_error_detail(&path, error);
                        if ["file-locked", "cloud-not-downloaded"].contains(&failures::classify(&error).code) {
                            return Err(error);
                        }
                        Vec::new()
                    }
                    Err(_) => Vec::new(),
                };
                let (texts, prints, engine) = {
                    let conn = self.conn();
                    let texts = page_texts(&conn, hash, &shapes)?;
                    let mut statement =
                        conn.prepare("SELECT page_no, exact_hash FROM resource_page WHERE resource_hash = ?1").map_err(sql_err)?;
                    let prints: BTreeMap<u32, String> = statement
                        .query_map([hash], |row| Ok((row.get(0)?, row.get(1)?)))
                        .map_err(sql_err)?
                        .collect::<Result<_, _>>()
                        .map_err(sql_err)?;
                    let engine = super::services::ask_engine(&conn)?.map(|c| crate::source_volume::asking::engine_id(&c));
                    (texts, prints, engine)
                };
                let pages = signals::signals(&shapes, &scans, &texts);
                let mut steps = vec![CutStep {
                    after: 0,
                    verdict: Verdict::New,
                    rule: "start".into(),
                    question: None,
                    model_says: None,
                    model_confidence: None,
                    rule_type: pages.first().and_then(rules::type_of),
                    model_type: None,
                    zones: false,
                    hints: None,
                }];
                let mut answers: Vec<(u32, CutAnswer)> = Vec::new();
                for pair in pages.windows(2) {
                    let (verdict, rule) = rules::step(&pair[0], &pair[1]);
                    let mut s = CutStep {
                        after: pair[0].page,
                        verdict,
                        rule: rule.into(),
                        question: None,
                        model_says: None,
                        model_confidence: None,
                        rule_type: rules::type_of(&pair[1]),
                        model_type: None,
                        zones: false,
                        hints: None,
                    };
                    if verdict == Verdict::Doubtful {
                        if let (Some(a), Some(b)) = (prints.get(&pair[0].page), prints.get(&pair[1].page)) {
                            let key = ask::key(a, b);
                            let kept = cached_answer(&self.conn(), &key, engine.as_deref())?;
                            if let Some(answer) = kept.and_then(|a| CutAnswer::parse(&a).ok()) {
                                s.model_says = Some(answer.second_starts_new_document);
                                s.model_confidence = known_confidence(&answer.confidence);
                                s.model_type = CutAnswer::known(&answer.second_page_type);
                                answers.push((s.after, answer));
                            }
                            s.question = Some(key);
                        }
                    }
                    steps.push(s);
                }
                // The even file sees the whole file: a step no rule decided and
                // the model did not answer goes on (D-603), unless the page
                // before ends with a signature (D-616). It keeps its question,
                // for an answer that comes later. A person's word is not looked
                // at here: it wins over any step when the parts are made
                // (`parts.rs`).
                let verdicts: Vec<Verdict> = steps.iter().skip(1).map(|s| s.verdict).collect();
                if rules::uniform(&pages, &verdicts) {
                    for s in steps.iter_mut().skip(1) {
                        let signed = pages.iter().find(|p| p.page == s.after).is_some_and(rules::signed_at_the_end);
                        if s.verdict == Verdict::Doubtful && s.rule == rules::NONE && s.model_says.is_none() && !signed {
                            s.verdict = Verdict::Continue;
                            s.rule = rules::UNIFORM.into();
                        }
                    }
                }
                for (after, answer) in answers {
                    if let Some(before) = steps.iter_mut().find(|s| s.after + 1 == after) {
                        if before.model_type.is_none() {
                            before.model_type = CutAnswer::known(&answer.first_page_type);
                        }
                    }
                }
                let mut conn = self.conn();
                let tx = conn.transaction().map_err(sql_err)?;
                tx.execute("DELETE FROM cut_step WHERE resource_hash = ?1", [hash]).map_err(sql_err)?;
                let now = now_ms();
                for s in &steps {
                    tx.execute(
                        "INSERT INTO cut_step (resource_hash, after_page, verdict, rule, question, model_says, rule_type, model_type,
                                               rules_version, text_stamp, created_at, model_confidence)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                        params![hash, s.after, s.verdict.as_str(), s.rule, s.question, s.model_says, s.rule_type, s.model_type, version, stamp, now, s.model_confidence],
                    )
                    .map_err(sql_err)?;
                }
                tx.commit().map_err(sql_err)
            };
            match cut() {
                Ok(()) => note_success(&self.conn(), CUT_STAGE, hash)?,
                Err(error) => note_failure(&self.conn(), CUT_STAGE, hash, &format!("{version}|{stamp}"), &error)?,
            }
            let _ = version_id;
        }
        Ok(jobs.len())
    }

    /// The versions of whole files that wait for the cutter before they are
    /// read, typed or named (D-666): those of a PDF of two pages or more (by
    /// its ficha, or by the text of its pages while the ficha is still to be
    /// made) whose cut is still to be made with its text and the rules in
    /// use (`cut_pending`), or whose parts are still to be made from its
    /// steps (`cut_parts_pending`). A file whose ficha, cut or parts failed
    /// waits too, with the parts of a cut before, as they were (D-717): it
    /// is not read whole. A file the cutter does not take is not waited
    /// for: a PDF of one page, an image, any other file. Asks nothing.
    pub(crate) fn waiting_for_cut(&self) -> Result<BTreeSet<i64>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT f.version_id FROM {CUTTABLE_FROM}
                 LEFT JOIN derivative d ON d.version_id = c.id AND d.kind = ?1 AND d.engine = ?2
                 WHERE {cuttable} AND NOT {settled}
                   AND coalesce(json_array_length(z.body), json_array_length(d.body, '$.pages'),
                                (SELECT count(DISTINCT t.page_no) FROM resource_text t WHERE t.resource_hash = f.hash)) >= 2
                 UNION
                 SELECT f.version_id FROM {parts_from} WHERE {parts}
                 UNION
                 SELECT f.version_id FROM file f WHERE {file_waits}
                 UNION
                 SELECT a.version_id FROM appearance a WHERE {part_waits}",
                cuttable = cuttable_where(),
                settled = cut_settled_sql("?3"),
                parts_from = parts::PARTS_FROM,
                parts = parts::parts_pending_sql(),
                file_waits = failures::file_waits_sql("f.hash"),
                part_waits = failures::file_waits_sql("a.resource_hash"),
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(params![SHAPES_KIND, SHAPES_ENGINE, rules_version()], |row| row.get(0))
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// The versions of the files whose doubtful steps the thread that asks
    /// the model is still to ask by itself (`ASKED_BY_ITSELF`): they wait
    /// for the answer, whatever keeps it from coming (D-717, «la IA nunca
    /// se saltea»): the round at its cap, the automatic reading off, an
    /// engine that does not look at images, the folder not connected, the
    /// model failing. None of them goes on with its uncertain cut: that is
    /// left only for what a person looks at (the steps asked only when a
    /// person asks, `cut_ask_file`). The answer may make their parts again:
    /// they are read after it, once. `_sees_images` no longer matters.
    pub(crate) fn waiting_for_cut_answers(&self, _sees_images: bool) -> Result<BTreeSet<i64>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT DISTINCT a.version_id FROM appearance a
                 WHERE (EXISTS (SELECT 1 FROM cut_step s WHERE s.resource_hash = a.resource_hash AND {WITHOUT_ANSWER} AND {ASKED_BY_ITSELF})
                        OR EXISTS (SELECT 1 FROM cut_window w WHERE w.resource_hash = a.resource_hash AND {window_without}))
                   AND EXISTS (SELECT 1 FROM location l WHERE l.file_hash = a.resource_hash AND l.hash_state = 'done' AND NOT {excluded})",
                excluded = super::excluded_sql!(),
                window_without = by_zones::WINDOW_WITHOUT_ANSWER,
            ))
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// What the cutter says of each step of a file, in order (the start of
    /// the file first).
    pub fn cut_steps(&self, hash: &str) -> Result<Vec<CutStep>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(
                "SELECT after_page, verdict, rule, question, model_says, rule_type, model_type, model_confidence,
                        rules_version LIKE 'cut-zones/%', hints FROM cut_step
                 WHERE resource_hash = ?1 ORDER BY after_page",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([hash], |row| {
                Ok(CutStep {
                    after: row.get(0)?,
                    verdict: Verdict::parse(&row.get::<_, String>(1)?),
                    rule: row.get(2)?,
                    question: row.get(3)?,
                    model_says: row.get(4)?,
                    rule_type: row.get(5)?,
                    model_type: row.get(6)?,
                    model_confidence: row.get(7)?,
                    zones: row.get(8)?,
                    hints: row.get(9)?,
                })
            })
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// The questions about the doubtful steps, for the thread that asks
    /// the model: nothing while the automatic reading is off, and only in
    /// the files of `ASKED_BY_ITSELF`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn cut_background(&self, asker: &dyn DocumentAsker, limit: usize) -> Result<CutAskReport, String> {
        if !self.background_reading()? {
            return Ok(CutAskReport { left: self.cut_questions()?.by_itself, ..CutAskReport::default() });
        }
        self.cut_ask_run(asker, limit, None)
    }

    /// The worker's turn for the questions about the cuts (D-606): when the
    /// round of questions with a cap lets them go, up to `limit` more,
    /// counted in the round; `None` while they wait for a person.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn cut_round_turn(&self, asker: &dyn DocumentAsker, limit: usize) -> Result<Option<CutAskReport>, String> {
        if !self.auto_questions_go("cut")? {
            return Ok(None);
        }
        let report = self.cut_background(asker, limit)?;
        let state = self.engine_state();
        state.ran("cut", report.left);
        state.auto_asked(report.asked);
        Ok(Some(report))
    }

    /// How many doubtful steps are still without an answer: those the
    /// thread asks by itself and those that wait for a person. Asks nothing.
    pub fn cut_questions(&self) -> Result<CutQuestions, String> {
        let (by_itself, all): (i64, i64) = self
            .conn()
            .query_row(
                &format!("SELECT coalesce(sum({ASKED_BY_ITSELF}), 0), count(*) FROM cut_step s WHERE {WITHOUT_ANSWER}"),
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sql_err)?;
        // Each window of the cut by zones still without an answer is one
        // question more, always asked by itself (D-720).
        let windows: i64 = self
            .conn()
            .query_row(&format!("SELECT count(*) FROM cut_window w WHERE {}", by_zones::WINDOW_WITHOUT_ANSWER), [], |row| row.get(0))
            .map_err(sql_err)?;
        Ok(CutQuestions { by_itself: (by_itself + windows) as usize, on_request: (all - by_itself) as usize })
    }

    /// Doubtful steps without an answer in the files of `only`, or, without
    /// it, in those the thread asks by itself.
    pub(crate) fn cut_doubtful_left(&self, only: Option<&BTreeSet<String>>) -> Result<usize, String> {
        let Some(only) = only else {
            return Ok(self.cut_questions()?.by_itself);
        };
        let windows = self.windows_left(Some(only))?;
        let conn = self.conn();
        let mut statement = conn.prepare(&format!("SELECT s.resource_hash FROM cut_step s WHERE {WITHOUT_ANSWER}")).map_err(sql_err)?;
        let hashes: Vec<String> =
            statement.query_map([], |row| row.get(0)).map_err(sql_err)?.collect::<Result<_, _>>().map_err(sql_err)?;
        Ok(hashes.iter().filter(|h| only.contains(*h)).count() + windows)
    }

    /// A person asks the model where each document of one PDF starts
    /// (D-606): every doubtful step of it still without an answer, one at
    /// a time, with the automatic reading on or off; the answers are kept
    /// and counted like any other. Its parts are made again with them.
    /// Returns the parts as they are now.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn cut_ask_file(&self, asker: &dyn DocumentAsker, hash: &str) -> Result<Vec<parts::IncludedPart>, String> {
        if !asker.sees_images() {
            return Err("El motor de «Preguntar al modelo» no mira imágenes: no puede decir dónde empieza cada documento.".into());
        }
        let only = BTreeSet::from([hash.to_string()]);
        let report = self.cut_ask_run(asker, usize::MAX, Some(&only))?;
        self.cut_parts_of(hash)?;
        if report.asked + report.kept == 0 {
            if let Some(error) = report.errors.first() {
                return Err(error.clone());
            }
        }
        self.strip_parts(hash)
    }

    /// Asks the model about up to `limit` doubtful steps (of the files in
    /// `only`, when given; otherwise of those of `ASKED_BY_ITSELF`), one at
    /// a time, with the two pages as images. A step whose pages were
    /// already answered takes that answer without asking.
    ///
    /// Each failure is written down with its class (P-523, D-714). One of
    /// a stretch (one step) does not stop the others: that stretch takes
    /// its next attempt and the run goes on with the next. One of the whole
    /// engine pauses only that engine, until its next attempt. An engine
    /// that cannot look at images asks nothing: a permanent failure, and
    /// the files wait (D-717).
    pub fn cut_ask_run(&self, asker: &dyn DocumentAsker, limit: usize, only: Option<&BTreeSet<String>>) -> Result<CutAskReport, String> {
        let mut report = CutAskReport::default();
        let engine = asker.engine_id();
        if !asker.sees_images() {
            report.left = self.cut_doubtful_left(only)?;
            if only.is_none() && report.left > 0 {
                let about = failures::About::Engine(engine.clone());
                if !failures::is_open(&self.conn(), failures::CUT_QUESTION_STAGE, &about)? {
                    let mut failure = failures::Failure::new(
                        failures::CUT_QUESTION_STAGE,
                        about,
                        "The engine does not look at images: it cannot say where each document starts.",
                    );
                    failure.cause = failures::cause("engine-no-images");
                    failure.engine = Some(&engine);
                    self.note(&failure);
                }
            }
            return Ok(report);
        }
        failures::resolve_cause(&self.conn(), "engine-no-images")?;
        if only.is_none() {
            let paused = failures::engine_paused_until(&self.conn(), &engine)?;
            if let Some(until) = paused {
                report.paused_until = Some(until);
                report.left = self.cut_doubtful_left(only)?;
                return Ok(report);
            }
        }
        let _one_at_a_time = ASKING.lock().unwrap_or_else(|p| p.into_inner());
        let rows: Vec<(String, u32, String, Option<i64>)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(
                    &format!(
                        "SELECT s.resource_hash, s.after_page, s.question, f.version_id FROM cut_step s
                         JOIN file f ON f.hash = s.resource_hash
                         WHERE {WITHOUT_ANSWER} AND ({by_itself})
                         ORDER BY s.resource_hash, s.after_page",
                        // The files asked for go whole, also the stretches
                        // that failed; without them, only those the thread
                        // asks by itself, but not a stretch whose failure
                        // holds (its next attempt has not come).
                        by_itself = if only.is_some() {
                            "1".to_string()
                        } else {
                            format!(
                                "{ASKED_BY_ITSELF} AND NOT {}",
                                failures::holds_any_sql(failures::CUT_QUESTION_STAGE, "s.resource_hash || '#' || s.after_page")
                            )
                        }
                    ),
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let (prompt, schema) = (ask::prompt(), ask::schema());
        for (hash, after, key, version_id) in rows {
            if only.is_some_and(|o| !o.contains(&hash)) {
                continue;
            }
            let kept = cached_answer(&self.conn(), &key, Some(&engine))?.and_then(|a| CutAnswer::parse(&a).ok());
            let span = failures::About::Span { file: hash.clone(), after };
            if let Some(answer) = kept {
                apply_answer(&self.conn(), &key, &answer)?;
                failures::resolve(&self.conn(), failures::CUT_QUESTION_STAGE, &span)?;
                report.kept += 1;
                continue;
            }
            if report.asked + report.failed >= limit {
                continue;
            }
            // The folder is not connected: a transient failure of the
            // stretch; the file waits (D-717).
            let Some(path) = file_path(&self.conn(), &hash)? else {
                let mut failure =
                    failures::Failure::new(failures::CUT_QUESTION_STAGE, span, "The folder of the file is not connected.");
                failure.cause = failures::cause("folder-disconnected");
                self.note(&failure);
                continue;
            };
            let scratch = std::env::temp_dir().join(format!("micelya-cut-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
            let started = Instant::now();
            let outcome = ask_one(asker, &path, after, &scratch, &prompt, &schema);
            report.seconds += started.elapsed().as_secs_f64();
            let _ = std::fs::remove_dir_all(&scratch);
            match outcome {
                Ok((text, parsed)) => {
                    let conn = self.conn();
                    keep_cut_answer(&conn, &key, &engine, version_id, &text)?;
                    apply_answer(&conn, &key, &parsed)?;
                    failures::resolve(&conn, failures::CUT_QUESTION_STAGE, &span)?;
                    failures::resolve_engine(&conn, &engine)?;
                    report.asked += 1;
                }
                Err(failed) => {
                    report.failed += 1;
                    report.errors.push(failed.message.clone());
                    let whole_engine = failed.cause.scope == failures::Scope::Engine;
                    let about = if whole_engine { failures::About::Engine(engine.clone()) } else { span };
                    let mut failure = failures::Failure::new(failures::CUT_QUESTION_STAGE, about, &failed.message);
                    failure.cause = failed.cause;
                    failure.engine = Some(&engine);
                    failure.answer = failed.answer.as_deref();
                    let recorded = self.note(&failure);
                    if whole_engine {
                        // Only this engine stops, until its next attempt.
                        report.paused_until = Some(recorded.and_then(|r| r.next_at).unwrap_or(i64::MAX));
                        break;
                    }
                    // The next stretch goes on (P-523).
                }
            }
        }
        // The windows of the cut by zones: one question each.
        if report.paused_until.is_none() && !report.errors.last().is_some_and(|e| stops_the_run(e)) {
            self.cut_windows_ask(asker, limit, only, &mut report)?;
        }
        // The reading waits for these answers only while they come
        // (`waiting_for_cut_answers`).
        if only.is_none() && report.asked + report.failed > 0 {
            self.engine_state().cut_asking_failed(report.failed > 0);
        }
        report.left = self.cut_doubtful_left(only)?;
        Ok(report)
    }
}

/// What failed in one question about a step.
struct AskFailed {
    message: String,
    cause: failures::Cause,
    /// The answer of another shape, to count the equal ones.
    answer: Option<String>,
}

/// One question about the step after page `after` of the PDF at `path`:
/// the two pages as images, the answer read. A failure says its cause: the
/// image that cannot be made (as the PDF that does not open), the model's
/// error, or an answer of another shape (D-714).
fn ask_one(
    asker: &dyn DocumentAsker,
    path: &std::path::Path,
    after: u32,
    scratch: &std::path::Path,
    prompt: &str,
    schema: &serde_json::Value,
) -> Result<(String, CutAnswer), AskFailed> {
    let images = run_guarded("docbase_cut_render", || ask::render_pages(path, &[after, after + 1], scratch)).map_err(|error| {
        let message = failures::pdf_error_detail(path, error);
        AskFailed { cause: failures::classify(&message), message, answer: None }
    })?;
    let text = asker
        .ask_with_images(prompt, schema, &images)
        .map_err(|message| AskFailed { cause: failures::classify(&message), message, answer: None })?;
    match CutAnswer::parse(&text) {
        Ok(parsed) => Ok((text, parsed)),
        Err(message) => Err(AskFailed { message, cause: failures::cause("bad-answer"), answer: Some(text) }),
    }
}
