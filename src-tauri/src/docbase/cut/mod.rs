//! The cutter of the PDFs that join several documents (D-556): rules
//! first, and the model only in the doubtful steps.
//!
//! For every PDF of two pages or more that has its ficha of shape and the
//! text of its pages, the background worker keeps what the cutter says of
//! each step from one page to the next (`cut_step`): the verdict
//! (`continue`, `new` or `doubtful`), the rule that gave it and the type
//! the next page suggests. The rules ask nothing, so they always run; a
//! file is cut again when the rules, the catalog (`cut.json`) or the text
//! of its pages change. A doubtful step counts as `continue` until the
//! model answers; it is asked only by the thread that asks the model,
//! while the automatic reading is on, with the two pages as images. The
//! answer is kept in `model_answer` (task `cut`) by the version of the
//! instruction and the exact prints of the two pages: the same pages in
//! another file are not asked again.
//!
//! The steps are turned into versions and appearances in `parts.rs`.

mod ask;
mod catalog;
pub mod parts;
mod rules;
mod signals;

#[cfg(test)]
pub(crate) mod check;
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

use super::page_shapes::SHAPES_KIND;
use super::similar::classify::{cached_answer, stops_the_run, ASKING};
use super::text::EXTRACTION_ENGINE;
use super::{now_ms, sql_err, DocBase};

pub use ask::{CutAnswer, CUT_TASK};
pub use rules::Verdict;

/// The version of the rules. With the catalog's, what the kept steps were
/// decided with: another one decides them again.
pub const RULES_VERSION: &str = "cut-rules/1";

pub fn rules_version() -> String {
    format!("{RULES_VERSION}+cut.json/{}", catalog::catalog().version)
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
    /// The type the vocabulary finds in the next page, and the one the
    /// model gave.
    pub rule_type: Option<String>,
    pub model_type: Option<String>,
}

impl CutStep {
    /// Whether a document starts at the next page: as the rules say, or as
    /// the model says in a doubtful step; doubtful without an answer
    /// counts as continue.
    pub fn starts(&self) -> bool {
        match self.verdict {
            Verdict::New => true,
            Verdict::Continue => false,
            Verdict::Doubtful => self.model_says == Some(true),
        }
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
    /// Doubtful steps still without an answer.
    pub left: usize,
    pub seconds: f64,
    pub errors: Vec<String>,
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
        "UPDATE cut_step SET model_says = ?1, model_type = coalesce(?2, model_type) WHERE question = ?3",
        params![answer.second_starts_new_document, CutAnswer::known(&answer.second_page_type), key],
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

impl DocBase {
    /// Cuts up to `limit` PDF files whose steps are missing or were decided
    /// with other rules, another catalog or another text of their pages.
    /// Only the rules: a doubtful step takes an answer already kept for
    /// its two pages, and nothing is asked. Returns how many were cut.
    pub fn cut_pending(&self, limit: usize) -> Result<usize, String> {
        let version = rules_version();
        type Job = (String, i64, String, String, String, i64);
        let jobs: Vec<Job> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(&format!(
                    "SELECT f.hash, f.version_id, o.location, min(l.rel_path), d.body,
                            (SELECT max(t.created_at) FROM resource_text t WHERE t.resource_hash = f.hash) AS stamp
                     FROM file f JOIN version c ON c.id = f.version_id
                     JOIN derivative d ON d.version_id = c.id AND d.kind = ?1 AND d.engine = ?2
                     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done' AND lower(l.ext) = 'pdf'
                     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                     WHERE c.state IN ('processed', 'needs-recognition') AND NOT {}
                       AND json_array_length(d.body, '$.pages') >= 2
                       AND EXISTS (SELECT 1 FROM resource_text t WHERE t.resource_hash = f.hash)
                       AND NOT EXISTS (SELECT 1 FROM cut_step s WHERE s.resource_hash = f.hash AND s.after_page = 0
                           AND s.rules_version = ?3
                           AND s.text_stamp = (SELECT max(t.created_at) FROM resource_text t WHERE t.resource_hash = f.hash))
                     GROUP BY f.hash ORDER BY f.hash LIMIT ?4",
                    super::excluded_sql!()
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params![SHAPES_KIND, SHAPES_ENGINE, version, limit as i64], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
                })
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (hash, version_id, root, rel_path, body, stamp) in &jobs {
            let shapes: PdfShapes = serde_json::from_str(body).unwrap_or_default();
            let path = LocalFolderProvider::new(root.clone()).local_path(rel_path).unwrap_or_default();
            // A PDF pdfium cannot open is cut by its text alone.
            let scans = run_guarded("docbase_cut", || signals::scan_pages(&path)).unwrap_or_default();
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
                rule_type: pages.first().and_then(rules::type_of),
                model_type: None,
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
                    rule_type: rules::type_of(&pair[1]),
                    model_type: None,
                };
                if verdict == Verdict::Doubtful {
                    if let (Some(a), Some(b)) = (prints.get(&pair[0].page), prints.get(&pair[1].page)) {
                        let key = ask::key(a, b);
                        let kept = cached_answer(&self.conn(), &key, engine.as_deref())?;
                        if let Some(answer) = kept.and_then(|a| CutAnswer::parse(&a).ok()) {
                            s.model_says = Some(answer.second_starts_new_document);
                            s.model_type = CutAnswer::known(&answer.second_page_type);
                            answers.push((s.after, answer));
                        }
                        s.question = Some(key);
                    }
                }
                steps.push(s);
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
                                           rules_version, text_stamp, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![hash, s.after, s.verdict.as_str(), s.rule, s.question, s.model_says, s.rule_type, s.model_type, version, stamp, now],
                )
                .map_err(sql_err)?;
            }
            tx.commit().map_err(sql_err)?;
            let _ = version_id;
        }
        Ok(jobs.len())
    }

    /// What the cutter says of each step of a file, in order (the start of
    /// the file first).
    pub fn cut_steps(&self, hash: &str) -> Result<Vec<CutStep>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(
                "SELECT after_page, verdict, rule, question, model_says, rule_type, model_type FROM cut_step
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
                })
            })
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// The questions about the doubtful steps, for the thread that asks
    /// the model: nothing while the automatic reading is off.
    pub fn cut_background(&self, asker: &dyn DocumentAsker, limit: usize) -> Result<CutAskReport, String> {
        if !self.background_reading()? {
            return Ok(CutAskReport { left: self.cut_doubtful_left(None)?, ..CutAskReport::default() });
        }
        self.cut_ask_run(asker, limit, None)
    }

    fn cut_doubtful_left(&self, only: Option<&BTreeSet<String>>) -> Result<usize, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare("SELECT resource_hash FROM cut_step WHERE verdict = 'doubtful' AND model_says IS NULL AND question IS NOT NULL")
            .map_err(sql_err)?;
        let hashes: Vec<String> =
            statement.query_map([], |row| row.get(0)).map_err(sql_err)?.collect::<Result<_, _>>().map_err(sql_err)?;
        Ok(hashes.iter().filter(|h| only.is_none_or(|o| o.contains(*h))).count())
    }

    /// Asks the model about up to `limit` doubtful steps (of the files in
    /// `only`, when given), one at a time, with the two pages as images. A
    /// step whose pages were already answered takes that answer without
    /// asking. An engine that cannot look at images asks nothing.
    pub fn cut_ask_run(&self, asker: &dyn DocumentAsker, limit: usize, only: Option<&BTreeSet<String>>) -> Result<CutAskReport, String> {
        let mut report = CutAskReport::default();
        if !asker.sees_images() {
            report.left = self.cut_doubtful_left(only)?;
            return Ok(report);
        }
        let _one_at_a_time = ASKING.lock().unwrap_or_else(|p| p.into_inner());
        let engine = asker.engine_id();
        let rows: Vec<(String, u32, String, Option<i64>)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(
                    "SELECT s.resource_hash, s.after_page, s.question, f.version_id FROM cut_step s
                     JOIN file f ON f.hash = s.resource_hash
                     WHERE s.verdict = 'doubtful' AND s.model_says IS NULL AND s.question IS NOT NULL
                     ORDER BY s.resource_hash, s.after_page",
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
            if let Some(answer) = kept {
                apply_answer(&self.conn(), &key, &answer)?;
                report.kept += 1;
                continue;
            }
            if report.asked + report.failed >= limit {
                continue;
            }
            let Some(path) = file_path(&self.conn(), &hash)? else {
                continue;
            };
            let scratch = std::env::temp_dir().join(format!("micelya-cut-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir_all(&scratch).map_err(|e| e.to_string())?;
            let started = Instant::now();
            let answer = run_guarded("docbase_cut_render", || ask::render_pages(&path, &[after, after + 1], &scratch))
                .and_then(|images| asker.ask_with_images(&prompt, &schema, &images));
            report.seconds += started.elapsed().as_secs_f64();
            let _ = std::fs::remove_dir_all(&scratch);
            match answer.and_then(|text| CutAnswer::parse(&text).map(|parsed| (text, parsed))) {
                Ok((text, parsed)) => {
                    let conn = self.conn();
                    keep_cut_answer(&conn, &key, &engine, version_id, &text)?;
                    apply_answer(&conn, &key, &parsed)?;
                    report.asked += 1;
                }
                Err(error) => {
                    report.failed += 1;
                    let stop = stops_the_run(&error);
                    report.errors.push(error);
                    if stop {
                        break;
                    }
                }
            }
        }
        report.left = self.cut_doubtful_left(only)?;
        Ok(report)
    }
}
