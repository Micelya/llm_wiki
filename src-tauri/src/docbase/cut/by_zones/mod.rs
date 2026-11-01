//! The cut over the reading by zones (chain N, D-668): it runs where a PDF
//! has its reading by zones (derivative `zones`); a file without it keeps
//! the cut of before (`rules.rs`), unchanged.
//!
//! The four rules that decide only say «goes on» (`cut.json`, `zones.finals`);
//! the others are hints. The parts are the runs between two cuts of a rule
//! (with those rules, the whole file); a part with some doubtful step is
//! asked in windows of up to ten pages that share two (`window.rs`): one
//! question each, with the images, the firm text by zones and the hints.
//! The windows are kept in `cut_window`, with their key; the answers in
//! `model_answer` (task `cut-window`), by that key. A page two windows see
//! takes the least sure answer, and a step the model did not answer with
//! «alta» is uncertain: a person sees it (D-603). A person's word wins
//! over all of it when the parts are made (`parts.rs`).
//!
//! The model is asked about the windows only when a person asks
//! (`cut_ask_file`), or by itself when `WINDOWS_BY_ITSELF` is on, which it
//! is not until the owner decides it (decision 3 of the plan of the cut).

pub mod page;
pub(crate) mod py;
pub mod rules;
pub mod scan;
pub mod window;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::Instant;

use rusqlite::{params, Connection};
use serde_json::Value;

use crate::panic_guard::run_guarded;
use crate::source_volume::asking::DocumentAsker;

use super::catalog::catalog;
use super::{ask, file_path, CutAskReport, CutStep, Verdict};
use crate::docbase::similar::classify::{cached_answer, stops_the_run};
use crate::docbase::{now_ms, sql_err, DocBase};

pub use page::PageReading;
pub use scan::PageScan;
pub use window::WINDOW_TASK;

/// The kind of the derivative that keeps the reading by zones of a
/// version: its pages, in order (JSON list of `PageReading`).
#[cfg_attr(not(test), allow(dead_code))]
pub const ZONES_KIND: &str = "zones";

/// The setting that says a reading by zones is coming for the PDFs: while
/// it is `on`, the cut of a PDF without it waits for it, or for its
/// failure (`ZONES_STAGE`), and the extraction waits with it (D-710).
pub const ZONES_READING: &str = "cut:zones-reading";
/// What `note_failure` writes down when the reading by zones of a file
/// failed: that file waits (D-717).
#[cfg_attr(not(test), allow(dead_code))]
pub const ZONES_STAGE: &str = "zones";

/// The setting that lets the thread that asks the model take the windows
/// by itself, in the round with a cap (D-606). Off: decision 3 of the plan.
pub const WINDOWS_BY_ITSELF: &str = "cut:windows-by-itself";

/// The version the steps of the cut by zones are decided with.
pub fn zones_version() -> String {
    format!("{}+cut.json/{}", catalog().zones.version, catalog().version)
}

/// SQL: the windows are asked by themselves.
pub(crate) fn by_itself_sql() -> String {
    format!("coalesce((SELECT value FROM docbase_setting WHERE key = '{WINDOWS_BY_ITSELF}'), '') = '1'")
}

/// Whether a reading by zones is coming when the setting is not written:
/// the app reads every PDF by zones (`docbase/structure`, D-746); the
/// unit tests of the cut of before, made without the model of zones, say
/// so when they need it.
const ZONES_READING_BY_DEFAULT: &str = if cfg!(test) { "off" } else { "on" };

/// SQL over a file `f` without its reading by zones: one is coming
/// (`ZONES_READING`). Also when its reading failed: the file waits with its
/// failure written down, and is not cut with the cut of before (D-717).
pub(crate) fn zones_waiting_sql() -> String {
    format!(
        "(coalesce((SELECT value FROM docbase_setting WHERE key = '{ZONES_READING}'), '{ZONES_READING_BY_DEFAULT}') = 'on')"
    )
}

/// SQL: a window `w` still without an answer.
pub(crate) const WINDOW_WITHOUT_ANSWER: &str = "NOT EXISTS (SELECT 1 FROM model_answer m WHERE m.question = w.question)";

/// What the cut by zones says of a file, before any answer.
#[derive(Debug, Clone)]
pub struct ZonesCut {
    pub decisions: Vec<rules::Decision>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub parts: Vec<rules::Part>,
    pub windows: Vec<window::Window>,
}

/// The rules, the parts and the windows of a file, from the reading of
/// its pages and what pdfium tells of them (a page pdfium did not read
/// has nothing from it).
pub fn cut(readings: &[PageReading], scans: &[PageScan]) -> ZonesCut {
    let mut readings: Vec<&PageReading> = readings.iter().collect();
    readings.sort_by_key(|r| r.page);
    let features: Vec<page::Features> = readings
        .iter()
        .enumerate()
        .map(|(i, r)| page::features(r, &scans.get(i).cloned().unwrap_or_default()))
        .collect();
    let decisions: Vec<rules::Decision> =
        features.windows(2).map(|pair| rules::decide(pair[0].page, rules::rules(&pair[0], &pair[1]))).collect();
    let pages = readings.len() as u32;
    let parts = rules::parts(pages, &decisions);
    let by_page: BTreeMap<u32, &PageReading> = readings.iter().map(|r| (r.page, *r)).collect();
    let windows = window::windows(pages, &parts, &decisions, &by_page);
    ZonesCut { decisions, parts, windows }
}

/// A type the model gave, when it is one of the list.
fn known_type(t: &str) -> Option<String> {
    (t != ask::OTHER && catalog().ask.types.contains_key(t)).then(|| t.to_string())
}

fn confidence(sure: &str) -> Option<String> {
    match sure {
        "alta" => Some("high".into()),
        "media" => Some("medium".into()),
        "baja" => Some(super::LOW_CONFIDENCE.into()),
        _ => None,
    }
}

/// The steps of a file: what the rules decided, and in a doubtful step
/// what the windows answered (`answers`, by window, in order).
pub fn steps(cut: &ZonesCut, answers: &[Option<Value>]) -> Vec<CutStep> {
    let pairs: Vec<(&window::Window, Option<Value>)> = cut.windows.iter().zip(answers.iter().cloned()).collect();
    let (pages, types) = window::merged(&pairs);
    let type_at = |page: u32| types.get(&page).and_then(|(t, _)| known_type(t));
    let mut out = vec![CutStep {
        after: 0,
        verdict: Verdict::New,
        rule: "start".into(),
        question: None,
        model_says: None,
        model_confidence: None,
        rule_type: None,
        model_type: type_at(1),
        zones: true,
        hints: None,
    }];
    for d in &cut.decisions {
        let verdict = match d.decision {
            rules::CUTS => Verdict::New,
            rules::JOINS => Verdict::Continue,
            _ => Verdict::Doubtful,
        };
        let answer = (verdict == Verdict::Doubtful).then(|| pages.get(&(d.after + 1))).flatten();
        out.push(CutStep {
            after: d.after,
            verdict,
            rule: d.rule.clone(),
            question: None,
            model_says: answer.map(|a| a.starts),
            model_confidence: answer.and_then(|a| confidence(&a.sure)),
            rule_type: None,
            model_type: type_at(d.after + 1),
            zones: true,
            hints: Some(serde_json::to_string(&d.hints).unwrap_or_default()),
        });
    }
    out
}

/// The reading by zones of a version, when it has one.
pub(crate) fn readings_of(body: &str) -> Result<Vec<PageReading>, String> {
    serde_json::from_str(body).map_err(|e| format!("The reading by zones is not of the shape expected: {e}"))
}

/// A window as `cut_window` keeps it.
fn window_rows(conn: &Connection, hash: &str) -> Result<Vec<window::Window>, String> {
    let mut statement = conn
        .prepare(
            "SELECT first_page, last_page, part_first, part_last, window_no, windows, prompt, question FROM cut_window
             WHERE resource_hash = ?1 ORDER BY first_page",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([hash], |row| {
            Ok(window::Window {
                first: row.get(0)?,
                last: row.get(1)?,
                part: (row.get(2)?, row.get(3)?),
                number: row.get(4)?,
                windows: row.get(5)?,
                prompt: row.get(6)?,
                key: row.get(7)?,
            })
        })
        .map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

impl DocBase {
    /// Keeps the reading by zones of the pages of a file (by `engine`), for
    /// its version: its cut is made again with it. The app reads it itself
    /// (`docbase/structure`); this is for the tests, which give it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn put_zones(&self, hash: &str, readings: &[PageReading], engine: &str) -> Result<(), String> {
        let body = serde_json::to_string(readings).map_err(|e| e.to_string())?;
        self.conn()
            .execute(
                "INSERT OR REPLACE INTO derivative (version_id, kind, engine, body, created_at)
                 SELECT version_id, ?2, ?3, ?4, ?5 FROM file WHERE hash = ?1",
                params![hash, ZONES_KIND, engine, body, now_ms()],
            )
            .map_err(sql_err)?;
        Ok(())
    }

    /// Cuts a file by the reading by zones of its pages (`body`, made at
    /// `stamp`): its steps and its windows, with the answers already kept
    /// for them (`engine`, any when not given). Asks nothing.
    pub(crate) fn cut_by_zones(&self, hash: &str, pdf: &Path, body: &str, stamp: i64, engine: Option<&str>) -> Result<(), String> {
        let readings = readings_of(body)?;
        // A PDF pdfium cannot open is cut by its reading alone.
        let scans = run_guarded("docbase_cut_zones", || scan::scan_pdf(pdf)).unwrap_or_default();
        let cut = cut(&readings, &scans);
        let answers: Vec<Option<Value>> = {
            let conn = self.conn();
            cut.windows
                .iter()
                .map(|w| Ok(cached_answer(&conn, &w.key, engine)?.and_then(|a| window::valid(&a).ok())))
                .collect::<Result<_, String>>()?
        };
        let steps = steps(&cut, &answers);
        let version = zones_version();
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        tx.execute("DELETE FROM cut_step WHERE resource_hash = ?1", [hash]).map_err(sql_err)?;
        tx.execute("DELETE FROM cut_window WHERE resource_hash = ?1", [hash]).map_err(sql_err)?;
        let now = now_ms();
        for w in &cut.windows {
            tx.execute(
                "INSERT INTO cut_window (resource_hash, first_page, last_page, part_first, part_last, window_no, windows, question, prompt, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![hash, w.first, w.last, w.part.0, w.part.1, w.number, w.windows, w.key, w.prompt, now],
            )
            .map_err(sql_err)?;
        }
        for s in &steps {
            tx.execute(
                "INSERT INTO cut_step (resource_hash, after_page, verdict, rule, question, model_says, rule_type, model_type,
                                       rules_version, text_stamp, created_at, model_confidence, hints)
                 VALUES (?1, ?2, ?3, ?4, NULL, ?5, NULL, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![hash, s.after, s.verdict.as_str(), s.rule, s.model_says, s.model_type, version, stamp, now, s.model_confidence, s.hints],
            )
            .map_err(sql_err)?;
        }
        tx.commit().map_err(sql_err)
    }

    /// Puts in the doubtful steps of a file cut by zones what its windows
    /// answered (`engine`, any when not given). Returns how many windows
    /// have an answer.
    pub(crate) fn apply_windows(&self, hash: &str, engine: Option<&str>) -> Result<usize, String> {
        let conn = self.conn();
        let windows = window_rows(&conn, hash)?;
        if windows.is_empty() {
            return Ok(0);
        }
        let answers: Vec<Option<Value>> = windows
            .iter()
            .map(|w| Ok(cached_answer(&conn, &w.key, engine)?.and_then(|a| window::valid(&a).ok())))
            .collect::<Result<_, String>>()?;
        let pairs: Vec<(&window::Window, Option<Value>)> = windows.iter().zip(answers.iter().cloned()).collect();
        let (pages, types) = window::merged(&pairs);
        let mut statement = conn
            .prepare("SELECT after_page, verdict FROM cut_step WHERE resource_hash = ?1 ORDER BY after_page")
            .map_err(sql_err)?;
        let rows: Vec<(u32, String)> = statement
            .query_map([hash], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        for (after, verdict) in rows {
            let answer = (verdict == Verdict::Doubtful.as_str()).then(|| pages.get(&(after + 1))).flatten();
            let ty = types.get(&(after + 1)).and_then(|(t, _)| known_type(t));
            conn.execute(
                "UPDATE cut_step SET model_says = ?3, model_confidence = ?4, model_type = ?5 WHERE resource_hash = ?1 AND after_page = ?2",
                params![hash, after, answer.map(|a| a.starts), answer.and_then(|a| confidence(&a.sure)), ty],
            )
            .map_err(sql_err)?;
        }
        Ok(answers.iter().filter(|a| a.is_some()).count())
    }

    /// The windows still without an answer, of the files in `only` or,
    /// without it, of all when they are asked by themselves.
    pub(crate) fn windows_left(&self, only: Option<&BTreeSet<String>>) -> Result<usize, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT w.resource_hash FROM cut_window w WHERE {WINDOW_WITHOUT_ANSWER} AND ({})",
                if only.is_some() { "1".to_string() } else { by_itself_sql() }
            ))
            .map_err(sql_err)?;
        let hashes: Vec<String> =
            statement.query_map([], |row| row.get(0)).map_err(sql_err)?.collect::<Result<_, _>>().map_err(sql_err)?;
        Ok(hashes.iter().filter(|h| only.is_none_or(|o| o.contains(*h))).count())
    }

    /// Asks the model about the windows still without an answer (of the
    /// files in `only`; without it, only when they are asked by
    /// themselves), one at a time, counted in `report` with the limit of
    /// the run: each window is one question.
    pub(crate) fn cut_windows_ask(
        &self,
        asker: &dyn DocumentAsker,
        limit: usize,
        only: Option<&BTreeSet<String>>,
        report: &mut CutAskReport,
    ) -> Result<(), String> {
        let engine = asker.engine_id();
        let rows: Vec<(String, u32, u32, String, String)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(&format!(
                    "SELECT w.resource_hash, w.first_page, w.last_page, w.question, w.prompt FROM cut_window w
                     WHERE ({}) ORDER BY w.resource_hash, w.first_page",
                    if only.is_some() { "1".to_string() } else { by_itself_sql() }
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let schema = window::schema();
        let mut touched: BTreeSet<String> = BTreeSet::new();
        for (hash, first, last, key, prompt) in rows {
            if only.is_some_and(|o| !o.contains(&hash)) {
                continue;
            }
            if cached_answer(&self.conn(), &key, Some(&engine))?.is_some_and(|a| window::valid(&a).is_ok()) {
                report.kept += 1;
                touched.insert(hash);
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
            let pages: Vec<u32> = (first..=last).collect();
            let answer = run_guarded("docbase_cut_render", || ask::render_pages(&path, &pages, &scratch))
                .and_then(|images| asker.ask_with_images(&prompt, &schema, &images));
            report.seconds += started.elapsed().as_secs_f64();
            let _ = std::fs::remove_dir_all(&scratch);
            match answer.and_then(|text| window::valid(&text).map(|_| text)) {
                Ok(text) => {
                    self.conn()
                        .execute(
                            "INSERT OR REPLACE INTO model_answer (question, task, prompt_version, engine, version_id, prompt, answer, asked_at)
                             VALUES (?1, ?2, ?3, ?4, (SELECT version_id FROM file WHERE hash = ?5), ?6, ?7, ?8)",
                            params![key, WINDOW_TASK, catalog().zones.version, engine, hash, prompt, text, now_ms()],
                        )
                        .map_err(sql_err)?;
                    report.asked += 1;
                    touched.insert(hash);
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
        for hash in touched {
            self.apply_windows(&hash, Some(&engine))?;
        }
        Ok(())
    }
}
