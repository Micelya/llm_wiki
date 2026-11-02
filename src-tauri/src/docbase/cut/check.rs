//! The cutter over the folders of a test, as the prototype was measured:
//! the parts of the joined PDFs over a whole folder, the cuts given by a
//! sheet (`apply_given_cuts`) and the answers put in a base or written out
//! (`seed_answers`, `written_answers`), for the other checks. The variants
//! of the cut of before (A, the rules only; B, the model in its doubtful
//! steps of two pages) and their count of questions are gone with it (3b
//! of 2 nov): the rules of the cut by zones are measured against the
//! labels in `check_rules.rs`.
//!
//! Not part of the normal test run: it reads real folders and, with
//! `MICELYA_CUT_ASK`, costs the quota of the provider; the folders are only
//! read.
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\carpeta" MICELYA_CUT_SHEET="C:\hoja\partes.json" \
//!     cargo test --lib docbase::cut::check -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - `MICELYA_DOCBASE_CHECK`, `MICELYA_DOCBASE_START`,
//!   `MICELYA_RECOGNIZED_SEED`: as in `similar/check_types.rs`.
//! - `MICELYA_CUT_SHEET`: the parts sheet (`files` with `name`, `sha256`,
//!   `pages`); its files are found in the base by their hash.
//! - `MICELYA_ANSWER_SEED`: files of `model_answer` rows (a JSON list,
//!   separated by `;`) put in the base before cutting, never over an
//!   answer it has: the answers kept are not asked again.
//! - `MICELYA_CUT_ASK`: how many new questions the doubtful steps of the
//!   sheet's files may take, with Codex (`MICELYA_ASK_MODEL`,
//!   `MICELYA_ASK_REASONING`), one at a time.
//! - `MICELYA_ANSWER_SEED_WRITE`: where the answers about cuts the base
//!   keeps at the end are written, as `model_answer` rows.
//! - `MICELYA_KNOWLEDGE_SAVE`: where a copy of the base is kept at the end.

use std::collections::BTreeSet;

use serde_json::{json, Value};

use crate::docbase::DocBase;
use crate::source_volume::asking::{asker_for, AskConfig};

use super::CUT_TASK;

/// What `apply_given_cuts` did.
#[derive(Debug, Default)]
pub(crate) struct GivenCuts {
    /// Files of the cuts given that the base has, already cut.
    pub files: usize,
    /// Pages decided as a person would (from the second of each file).
    pub pages: usize,
    /// Files of the cuts given the base does not have, or has not cut.
    pub missing: usize,
    /// Files whose number of pages is not the one of the cuts given.
    pub other_pages: usize,
    /// Steps of those files that no person decided after it (should be 0).
    pub undecided: i64,
}

/// `MICELYA_CUTS_GIVEN`: the cuts of a file in the shape of the parts
/// sheet (`files[].sha256`, `pages`, `documents[].first_page`), decided
/// after `cut_pending` as a person would decide them, page by page from
/// the second one, with `cut_split_at` and `cut_join_with_previous`: the
/// cut asks the model nothing about those files. Only for the tests (the
/// chain N of the trial of the structure, P-484).
pub(crate) fn apply_given_cuts(base: &DocBase) -> Option<GivenCuts> {
    let path = std::env::var("MICELYA_CUTS_GIVEN").ok()?;
    let sheet: Value = serde_json::from_str(&std::fs::read_to_string(path.trim()).unwrap()).unwrap();
    let mut done = GivenCuts::default();
    for file in sheet["files"].as_array().into_iter().flatten() {
        let hash = file["sha256"].as_str().unwrap_or_default().to_lowercase();
        if file["pages"].as_u64() == Some(1) {
            continue;
        }
        let pages: Option<u32> = base
            .conn()
            .query_row("SELECT max(after_page) + 1 FROM cut_step WHERE resource_hash = ?1", [&hash], |row| row.get(0))
            .unwrap();
        let Some(pages) = pages else {
            done.missing += 1;
            continue;
        };
        if file["pages"].as_u64().is_some_and(|p| p != u64::from(pages)) {
            done.other_pages += 1;
            continue;
        }
        let starts: BTreeSet<u32> = file["documents"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|d| d["first_page"].as_u64().map(|p| p as u32))
            .collect();
        for page in 2..=pages {
            // A page already decided the same way stays as it is: its
            // documents are not made again when a run is taken up again.
            let wanted = if starts.contains(&page) { super::parts::STARTS } else { super::parts::CONTINUES };
            let kept: Option<String> = base
                .conn()
                .query_row(
                    "SELECT decision FROM cut_decision WHERE resource_hash = ?1 AND page = ?2",
                    rusqlite::params![hash, page],
                    |row| row.get(0),
                )
                .ok();
            if kept.as_deref() == Some(wanted) {
                continue;
            }
            if starts.contains(&page) {
                base.cut_split_at(&hash, page).unwrap();
            } else {
                base.cut_join_with_previous(&hash, page).unwrap();
            }
            done.pages += 1;
        }
        done.undecided += base
            .conn()
            .query_row(
                "SELECT count(*) FROM cut_step s WHERE s.resource_hash = ?1 AND s.after_page >= 1
                   AND NOT EXISTS (SELECT 1 FROM cut_decision d WHERE d.resource_hash = s.resource_hash AND d.page = s.after_page + 1)",
                [&hash],
                |row| row.get::<_, i64>(0),
            )
            .unwrap();
        done.files += 1;
    }
    println!(
        "cuts given (MICELYA_CUTS_GIVEN): {} files decided page by page ({} pages); not in the base or not cut: {}; with other pages: {}; steps left without a person's word: {}",
        done.files, done.pages, done.missing, done.other_pages, done.undecided
    );
    Some(done)
}

pub(crate) fn seed_answers(base: &DocBase, files: &str) -> usize {
    let mut put = 0;
    for file in files.split(';').filter(|f| !f.trim().is_empty()) {
        let rows: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(file.trim()).unwrap()).unwrap();
        for r in rows {
            let s = |k: &str| r[k].as_str().unwrap_or_default().to_string();
            put += base
                .conn()
                .execute(
                    "INSERT OR IGNORE INTO model_answer (question, task, prompt_version, engine, version_id, prompt, answer, asked_at)
                     VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, ?7)",
                    rusqlite::params![s("question"), s("task"), s("prompt_version"), s("engine"), s("prompt"), s("answer"), r["asked_at"].as_i64().unwrap_or(0)],
                )
                .unwrap();
        }
    }
    put
}

pub(super) fn written_answers(base: &DocBase) -> Value {
    let conn = base.conn();
    let mut statement = conn
        .prepare("SELECT question, task, prompt_version, engine, prompt, answer, asked_at FROM model_answer WHERE task = ?1 ORDER BY question, engine")
        .unwrap();
    let rows = statement
        .query_map([CUT_TASK], |row| {
            Ok(json!({
                "question": row.get::<_, String>(0)?, "task": row.get::<_, String>(1)?, "prompt_version": row.get::<_, String>(2)?,
                "engine": row.get::<_, String>(3)?, "prompt": row.get::<_, String>(4)?, "answer": row.get::<_, String>(5)?,
                "asked_at": row.get::<_, i64>(6)?,
            }))
        })
        .unwrap();
    Value::Array(rows.map(|r| r.unwrap()).collect())
}

/// Makes an empty base of this version at `MICELYA_EMPTY_BASE`, for
/// `a-la-base-nueva.py` to bring the answers of the earlier base into
/// (then it is the `MICELYA_DOCBASE_START` of the run below).
#[test]
#[ignore = "writes the base named in MICELYA_EMPTY_BASE"]
fn an_empty_base_to_start_from() {
    let Ok(path) = std::env::var("MICELYA_EMPTY_BASE") else {
        println!("MICELYA_EMPTY_BASE is not set: nothing to make");
        return;
    };
    let path = std::path::PathBuf::from(path);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    drop(DocBase::open(&path).unwrap());
}

/// How the parts stand: versions, documents and appearances; the joined
/// files and their parts, against the parts sheet; the versions read.
fn parts_report(base: &DocBase, sheet: &Value, when: &str) {
    let conn = base.conn();
    let count = |sql: &str| -> i64 { conn.query_row(sql, [], |row| row.get(0)).unwrap() };
    println!("--- {when}");
    println!(
        "versions {} · documents {} · appearances {} (whole files {}, of pages {}, proposed {}, confirmed {})",
        count("SELECT count(*) FROM version"),
        count("SELECT count(*) FROM document"),
        count("SELECT count(*) FROM appearance"),
        count("SELECT count(*) FROM appearance WHERE selector = 'file'"),
        count("SELECT count(*) FROM appearance WHERE selector GLOB 'pages:*'"),
        count("SELECT count(*) FROM appearance WHERE selector GLOB 'pages:*' AND cut_state = 'proposed'"),
        count("SELECT count(*) FROM appearance WHERE selector GLOB 'pages:*' AND cut_state = 'confirmed'"),
    );
    println!(
        "joined files {} · included versions {} (of them, twins of another version {}) · versions read {} (included {})",
        count("SELECT count(DISTINCT resource_hash) FROM appearance WHERE selector GLOB 'pages:*'"),
        count("SELECT count(DISTINCT version_id) FROM appearance WHERE selector GLOB 'pages:*'"),
        count(
            "SELECT count(DISTINCT a.version_id) FROM appearance a WHERE a.selector GLOB 'pages:*'
             AND (SELECT count(*) FROM appearance b WHERE b.version_id = a.version_id) > 1"
        ),
        count("SELECT count(*) FROM knowledge_extraction WHERE state = 'done'"),
        count("SELECT count(*) FROM knowledge_extraction k WHERE state = 'done' AND EXISTS (SELECT 1 FROM appearance a WHERE a.version_id = k.version_id AND a.selector GLOB 'pages:*')"),
    );
    drop(conn);
    let (mut expected, mut found, mut exact, mut files) = (0, 0, 0, 0);
    for file in sheet["files"].as_array().into_iter().flatten() {
        let hash = file["sha256"].as_str().unwrap_or_default();
        let truth: BTreeSet<(u64, u64)> = file["documents"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|d| (d["first_page"].as_u64().unwrap_or(0), d["last_page"].as_u64().unwrap_or(0)))
            .collect();
        let parts: BTreeSet<(u64, u64)> =
            base.included_parts(hash).unwrap().iter().map(|p| (p.first_page as u64, p.last_page as u64)).collect();
        files += usize::from(!parts.is_empty());
        expected += truth.len();
        found += parts.len();
        exact += truth.intersection(&parts).count();
    }
    println!("parts sheet: {files} files cut · {found} included documents, {exact} with the pages of the sheet, of {expected}");
}

/// The parts of the joined PDFs over the whole folder (step 1b): from the
/// base of `MICELYA_DOCBASE_START` (the answers brought from the earlier
/// base) and `MICELYA_ANSWER_SEED` (those about cuts), the reading with
/// only the answers kept (`MICELYA_ASK_OFFLINE`), and then, with
/// `MICELYA_CUT_ASK`, Codex on the doubtful steps of the PDFs outside the
/// sheet: first those the rules already cut, then the others.
#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK"]
fn the_parts_over_the_whole_folder() {
    let Ok(sheet) = std::env::var("MICELYA_CUT_SHEET") else {
        println!("MICELYA_CUT_SHEET is not set: nothing to measure");
        return;
    };
    let sheet: Value = serde_json::from_str(&std::fs::read_to_string(&sheet).unwrap()).unwrap();
    let Some((_scratch, base)) = crate::docbase::knowledge::check::open_base() else {
        println!("MICELYA_DOCBASE_CHECK is not set: nothing to measure");
        return;
    };
    if let Ok(seeds) = std::env::var("MICELYA_ANSWER_SEED") {
        println!("answers seeded: {}", seed_answers(&base, &seeds));
    }
    while base.refresh_extraction(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    while base.profile_pending(16).unwrap() > 0 {}
    while base.cut_pending(16).unwrap() > 0 {}
    while base.cut_parts_pending(64).unwrap() > 0 {}
    parts_report(&base, &sheet, "the cuts, with the answers kept");
    let started = std::time::Instant::now();
    let asker = crate::docbase::knowledge::check::recorded();
    let reading = crate::docbase::knowledge::check::read_all(&base, &asker);
    println!(
        "reading, only the answers kept: {} asked, {} failed, {} read, {} left, {:.0} s",
        reading.asked,
        reading.failed,
        reading.extracted,
        reading.left,
        started.elapsed().as_secs_f32()
    );
    if std::env::var("MICELYA_LATER_PASSES").is_ok() {
        crate::docbase::knowledge::check::later_passes(&base, &asker);
    }
    crate::docbase::knowledge::check::print_cache_hits(&base);
    // The questions the thread that asks the model would take about the
    // doubtful steps once the automatic reading is on (`cut_background`),
    // and those that wait for a person to ask (D-622).
    let questions = base.cut_questions().unwrap();
    println!(
        "   cut, doubtful steps without an answer kept: {} asked by itself, {} only when a person asks",
        questions.by_itself, questions.on_request
    );
    parts_report(&base, &sheet, "after the reading");
    crate::docbase::knowledge::check::save_copy(&base);
    let Ok(limit) = std::env::var("MICELYA_CUT_ASK") else {
        return;
    };
    let mut limit: usize = limit.trim().parse().unwrap();
    let in_sheet: BTreeSet<String> =
        sheet["files"].as_array().into_iter().flatten().filter_map(|f| f["sha256"].as_str().map(str::to_string)).collect();
    let (ruled, others): (BTreeSet<String>, BTreeSet<String>) = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT resource_hash, max(after_page > 0 AND verdict = 'new') FROM cut_step GROUP BY resource_hash HAVING max(after_page) >= 1")
            .unwrap();
        let rows: Vec<(String, bool)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().map(|r| r.unwrap()).collect();
        let outside: Vec<(String, bool)> = rows.into_iter().filter(|(h, _)| !in_sheet.contains(h)).collect();
        (
            outside.iter().filter(|(_, r)| *r).map(|(h, _)| h.clone()).collect(),
            outside.iter().filter(|(_, r)| !*r).map(|(h, _)| h.clone()).collect(),
        )
    };
    let config = AskConfig {
        engine: "codex-cli".into(),
        model: std::env::var("MICELYA_ASK_MODEL").unwrap_or_default(),
        reasoning: std::env::var("MICELYA_ASK_REASONING").unwrap_or_default(),
    };
    let codex = asker_for(&config).unwrap();
    for (what, only) in [("PDFs the rules cut", &ruled), ("the other PDFs", &others)] {
        let report = base.cut_ask_run(codex.as_ref(), limit, Some(only)).unwrap();
        limit = limit.saturating_sub(report.asked + report.failed);
        println!(
            "Codex, {what} ({} files): {} asked, {} failed, {} kept, {:.0} s; doubtful left: {}",
            only.len(),
            report.asked,
            report.failed,
            report.kept,
            report.seconds,
            report.left
        );
        for e in report.errors.iter().take(3) {
            println!("  error: {}", e.chars().take(200).collect::<String>());
        }
    }
    while base.cut_parts_pending(64).unwrap() > 0 {}
    let cut_outside = ruled.iter().chain(&others).filter(|h| !base.included_parts(h).unwrap().is_empty()).count();
    println!("PDFs outside the sheet: {} · cut in parts now: {cut_outside}", ruled.len() + others.len());
    parts_report(&base, &sheet, "after Codex");
    if let Ok(path) = std::env::var("MICELYA_ANSWER_SEED_WRITE") {
        std::fs::write(&path, serde_json::to_string_pretty(&written_answers(&base)).unwrap()).unwrap();
        println!("answers about cuts written: {path}");
    }
}
