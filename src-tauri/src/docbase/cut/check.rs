//! Measures the cutter over the PDFs of the parts sheet, as the prototype
//! was measured (`docs/investigacion/corte/guiones/medir.py`): it writes
//! the decisions of variant A (rules only) and of variant B (the model in
//! the doubtful steps) in the format that script reads.
//!
//! Not part of the normal test run: it reads real folders and, with
//! `MICELYA_CUT_ASK`, costs the quota of the provider. The base is a
//! scratch one, removed at the end; the folders are only read.
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\carpeta" MICELYA_CUT_SHEET="C:\hoja\partes.json" MICELYA_CUT_OUT="C:\tmp\corte" \
//!     cargo test --lib docbase::cut::check -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - `MICELYA_DOCBASE_CHECK`, `MICELYA_DOCBASE_START`,
//!   `MICELYA_RECOGNIZED_SEED`: as in `similar/check_types.rs`.
//! - `MICELYA_CUT_SHEET`: the parts sheet (`files` with `name`, `sha256`,
//!   `pages`); its files are found in the base by their hash.
//! - `MICELYA_CUT_OUT`: the folder where `cortes-a.json` and
//!   `cortes-b.json` are written (they name the files: keep it outside the
//!   repository).
//! - `MICELYA_ANSWER_SEED`: files of `model_answer` rows (a JSON list,
//!   separated by `;`) put in the base before cutting, never over an
//!   answer it has: the answers kept are not asked again.
//! - `MICELYA_CUT_ASK`: how many new questions the doubtful steps of the
//!   sheet's files may take, with Codex (`MICELYA_ASK_MODEL`,
//!   `MICELYA_ASK_REASONING`), one at a time.
//! - `MICELYA_ANSWER_SEED_WRITE`: where the answers about cuts the base
//!   keeps at the end are written, as `model_answer` rows.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

use crate::docbase::similar::check_types::prepare;
use crate::docbase::DocBase;
use crate::source_volume::asking::{asker_for, AskConfig};

use super::{CutAskReport, Verdict, CUT_TASK};

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

fn written_answers(base: &DocBase) -> Value {
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

/// The decisions of a file as `medir.py` reads them: with the model's
/// answers (variant B) or the rules only (variant A).
fn decisions(base: &DocBase, hash: &str, name: &str, pages: u64, with_model: bool) -> Value {
    let steps = base.cut_steps(hash).unwrap();
    let mut out = Vec::new();
    let mut types = BTreeMap::new();
    for s in &steps {
        let answered = with_model && s.verdict == Verdict::Doubtful && s.model_says.is_some();
        let (decision, rule) = if answered {
            (if s.starts() { "new" } else { "continue" }, "codex")
        } else {
            (s.verdict.as_str(), s.rule.as_str())
        };
        let ty = if with_model { s.suggested_type().map(str::to_string) } else { s.rule_type.clone() };
        if s.after == 0 || decision == "new" {
            types.insert((s.after + 1).to_string(), ty);
        }
        if s.after > 0 {
            out.push(json!({"after": s.after, "decision": decision, "rule": rule}));
        }
    }
    json!({"file": name, "pages": pages, "steps": out, "types": types})
}

#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK"]
fn the_cutter_over_the_parts_sheet() {
    let (Ok(folders), Ok(sheet), Ok(out)) =
        (std::env::var("MICELYA_DOCBASE_CHECK"), std::env::var("MICELYA_CUT_SHEET"), std::env::var("MICELYA_CUT_OUT"))
    else {
        println!("MICELYA_DOCBASE_CHECK, MICELYA_CUT_SHEET or MICELYA_CUT_OUT is not set: nothing to measure");
        return;
    };
    let sheet: Value = serde_json::from_str(&std::fs::read_to_string(&sheet).unwrap()).unwrap();
    let started = std::time::Instant::now();
    let (_scratch, base) = prepare(&folders);
    if let Ok(seeds) = std::env::var("MICELYA_ANSWER_SEED") {
        println!("answers seeded: {}", seed_answers(&base, &seeds));
    }
    while base.profile_pending(16).unwrap() > 0 {}
    let cut_started = std::time::Instant::now();
    let mut cut = 0;
    loop {
        let n = base.cut_pending(16).unwrap();
        if n == 0 {
            break;
        }
        cut += n;
    }
    println!(
        "base ready in {:.0} s; {cut} PDF cut by the rules in {:.1} s",
        cut_started.duration_since(started).as_secs_f64(),
        cut_started.elapsed().as_secs_f64()
    );

    // The sheet's files, by their hash.
    let mut sheet_files: Vec<(String, String, u64)> = Vec::new();
    for f in sheet["files"].as_array().unwrap() {
        let hash = f["sha256"].as_str().unwrap().to_lowercase();
        let known: i64 = base.conn().query_row("SELECT count(*) FROM file WHERE hash = ?1", [&hash], |row| row.get(0)).unwrap();
        if known == 1 {
            sheet_files.push((hash, f["name"].as_str().unwrap().to_string(), f["pages"].as_u64().unwrap()));
        }
    }
    println!("files of the sheet found: {} of {}", sheet_files.len(), sheet["files"].as_array().unwrap().len());
    let only: BTreeSet<String> = sheet_files.iter().map(|(h, _, _)| h.clone()).collect();
    let write = |file: &str, value: Value| {
        std::fs::create_dir_all(&out).unwrap();
        std::fs::write(std::path::Path::new(&out).join(file), serde_json::to_string_pretty(&value).unwrap()).unwrap();
    };
    let a: Vec<Value> = sheet_files.iter().map(|(h, n, p)| decisions(&base, h, n, *p, false)).collect();
    write("cortes-a.json", Value::Array(a));

    let mut report = CutAskReport::default();
    if let Ok(limit) = std::env::var("MICELYA_CUT_ASK") {
        let config = AskConfig {
            engine: "codex-cli".into(),
            model: std::env::var("MICELYA_ASK_MODEL").unwrap_or_default(),
            reasoning: std::env::var("MICELYA_ASK_REASONING").unwrap_or_default(),
        };
        let asker = asker_for(&config).unwrap();
        report = base.cut_ask_run(asker.as_ref(), limit.trim().parse().unwrap(), Some(&only)).unwrap();
        println!(
            "Codex: {} asked, {} failed, {} from the answers kept, {:.0} s; doubtful left without an answer: {}",
            report.asked, report.failed, report.kept, report.seconds, report.left
        );
        for e in report.errors.iter().take(3) {
            println!("  error: {}", e.chars().take(200).collect::<String>());
        }
    }
    let answered: usize = sheet_files
        .iter()
        .map(|(h, _, _)| base.cut_steps(h).unwrap().iter().filter(|s| s.verdict == Verdict::Doubtful && s.model_says.is_some()).count())
        .sum();
    let b: Vec<Value> = sheet_files.iter().map(|(h, n, p)| decisions(&base, h, n, *p, true)).collect();
    let meta = json!({"codex_calls": answered + report.failed, "codex_seconds": report.seconds, "codex_failed": report.failed,
                      "codex_new": report.asked});
    write("cortes-b.json", json!({"meta": meta, "files": b}));

    // The other PDFs, rules only: the risk of cutting what is not joined.
    let others: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT DISTINCT resource_hash FROM cut_step ORDER BY 1").unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.map(|r| r.unwrap()).filter(|h: &String| !only.contains(h)).collect()
    };
    let (mut with_cut, mut cuts, mut doubtful, mut steps) = (0, 0, 0, 0);
    let mut by_rule: BTreeMap<String, usize> = BTreeMap::new();
    for h in &others {
        let s = base.cut_steps(h).unwrap();
        let new: Vec<_> = s.iter().filter(|s| s.after > 0 && s.verdict == Verdict::New).collect();
        for n in &new {
            *by_rule.entry(n.rule.clone()).or_default() += 1;
        }
        with_cut += usize::from(!new.is_empty());
        cuts += new.len();
        doubtful += s.iter().filter(|s| s.verdict == Verdict::Doubtful).count();
        steps += s.iter().filter(|s| s.after > 0).count();
    }
    println!(
        "other PDFs of two pages or more: {}; with some cut: {with_cut}; cuts: {cuts} {by_rule:?}; steps: {steps}; doubtful: {doubtful}",
        others.len()
    );
    if let Ok(path) = std::env::var("MICELYA_ANSWER_SEED_WRITE") {
        let answers = written_answers(&base);
        println!("answers about cuts written: {}", answers.as_array().unwrap().len());
        std::fs::write(path, serde_json::to_string_pretty(&answers).unwrap()).unwrap();
    }
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
        count("SELECT count(*) FROM appearance WHERE selector <> 'file'"),
        count("SELECT count(*) FROM appearance WHERE selector <> 'file' AND cut_state = 'proposed'"),
        count("SELECT count(*) FROM appearance WHERE selector <> 'file' AND cut_state = 'confirmed'"),
    );
    println!(
        "joined files {} · included versions {} (of them, twins of another version {}) · versions read {} (included {})",
        count("SELECT count(DISTINCT resource_hash) FROM appearance WHERE selector <> 'file'"),
        count("SELECT count(DISTINCT version_id) FROM appearance WHERE selector <> 'file'"),
        count(
            "SELECT count(DISTINCT a.version_id) FROM appearance a WHERE a.selector <> 'file'
             AND (SELECT count(*) FROM appearance b WHERE b.version_id = a.version_id) > 1"
        ),
        count("SELECT count(*) FROM knowledge_extraction WHERE state = 'done'"),
        count("SELECT count(*) FROM knowledge_extraction k WHERE state = 'done' AND EXISTS (SELECT 1 FROM appearance a WHERE a.version_id = k.version_id AND a.selector <> 'file')"),
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
    // doubtful steps once the automatic reading is on (`cut_background`).
    println!("   cut, doubtful steps without an answer kept: {}", base.cut_doubtful_left(None).unwrap());
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
