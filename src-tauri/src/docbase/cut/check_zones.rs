//! Measures the cut by zones against the scripts of chain N (the plan of
//! the cut, section 4): with the readings by zones and the answers N kept,
//! the app must make the same questions (the same keys), find every
//! answer and ask nothing. And counts what the cut of before would ask by
//! itself over the same base, asking nothing.
//!
//! Not part of the normal test run: it reads real folders and writes the
//! names of their files; keep the output outside the repository.
//!
//! ```text
//! MICELYA_ZONES_BASE="C:\corrida\base-1-corte.sqlite3" MICELYA_ZONES_READING="C:\lectura\estructura" \
//! MICELYA_ZONES_ANSWERS="C:\N-ia\respuestas" MICELYA_ZONES_OUT="C:\salida" \
//!     cargo test --lib docbase::cut::check_zones -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - `MICELYA_ZONES_BASE`: a base to start from, copied to the output
//!   folder and never written; without it, a new one over the folders of
//!   `MICELYA_DOCBASE_CHECK` (as in `similar/check_types.rs`).
//! - `MICELYA_ZONES_READING`: the readings by zones (`<sha256>/pagina-<n>.json`).
//! - `MICELYA_ZONES_ANSWERS`: folders of the answers of N (`*.json` with
//!   `pregunta` and `respuesta`), separated by `;`. Each is kept with the
//!   key the app gives its question.
//! - `MICELYA_ZONES_SIGNALS`: optional, the signals pdfium gave N
//!   (`<sha256>.json`), to compare the paper, shape and image of each page.
//! - `MICELYA_ZONES_OUT`: where the base and the results are written.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde_json::{json, Value};

use super::by_zones::{self, window, PageReading, ZONES_READING, ZONES_STAGE};
use super::{note_failure, Verdict};
use crate::docbase::similar::check_types::prepare;
use crate::docbase::DocBase;
use crate::source_volume::asking::{engine_id, AskConfig, DocumentAsker};

/// The engine the answers of N were given by.
fn engine() -> String {
    engine_id(&AskConfig { engine: "codex-cli".into(), model: "gpt-6-luna".into(), reasoning: "high".into() })
}

/// Fails whenever it is asked: no question is allowed. Keeps what it was
/// asked, to tell which window did not find its answer.
struct NoQuestions {
    asked: Mutex<Vec<String>>,
}

impl DocumentAsker for NoQuestions {
    fn engine_id(&self) -> String {
        engine()
    }

    fn ask(&self, prompt: &str, _schema: &Value) -> Result<String, String> {
        self.asked.lock().unwrap().push(prompt.to_string());
        Err("no question is allowed in this run".into())
    }

    fn sees_images(&self) -> bool {
        true
    }

    fn ask_with_images(&self, prompt: &str, schema: &Value, _images: &[PathBuf]) -> Result<String, String> {
        self.ask(prompt, schema)
    }
}

/// The base: a copy of `MICELYA_ZONES_BASE` in `out`, or a new one.
fn base_in(out: &Path, name: &str) -> (Option<crate::docbase::testing::TempDir>, DocBase) {
    if let Ok(start) = std::env::var("MICELYA_ZONES_BASE") {
        let target = out.join(name);
        let _ = std::fs::remove_file(&target);
        std::fs::copy(&start, &target).unwrap();
        return (None, DocBase::open(&target).unwrap());
    }
    let folders = std::env::var("MICELYA_DOCBASE_CHECK").expect("MICELYA_ZONES_BASE or MICELYA_DOCBASE_CHECK");
    let (scratch, base) = prepare(&folders);
    while base.profile_pending(16).unwrap() > 0 {}
    (Some(scratch), base)
}

/// Empties what the cut keeps, for it to run from the start.
fn empty_the_cut(base: &DocBase) {
    base.conn()
        .execute_batch(
            "DELETE FROM cut_step; DELETE FROM cut_window; DELETE FROM cut_decision; DELETE FROM cut_applied;
             DELETE FROM model_answer WHERE task IN ('cut', 'cut-window');
             DELETE FROM failure WHERE stage IN ('cut', 'parts', 'zones');",
        )
        .unwrap();
}

fn pdfs(base: &DocBase) -> Vec<(String, i64)> {
    let conn = base.conn();
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT f.hash, f.version_id FROM file f JOIN location l ON l.file_hash = f.hash AND lower(l.ext) = 'pdf'
             ORDER BY f.hash",
        )
        .unwrap();
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    rows.map(|r| r.unwrap()).collect()
}

fn readings_of(folder: &Path) -> Vec<PageReading> {
    let mut out: Vec<PageReading> = std::fs::read_dir(folder)
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("pagina-"))
        .map(|e| serde_json::from_str(&std::fs::read_to_string(e.path()).unwrap()).unwrap())
        .collect();
    out.sort_by_key(|r| r.page);
    out
}

fn decision_of(v: Verdict) -> &'static str {
    match v {
        Verdict::Continue => by_zones::rules::JOINS,
        Verdict::New => by_zones::rules::CUTS,
        Verdict::Doubtful => by_zones::rules::DOUBTFUL,
    }
}

fn sure_of(confidence: Option<&str>) -> Value {
    match confidence {
        Some("high") => json!("alta"),
        Some("medium") => json!("media"),
        Some("low") => json!("baja"),
        _ => Value::Null,
    }
}

/// The first line where two questions differ.
fn first_difference(a: &str, b: &str) -> String {
    for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
        if x != y {
            return format!("renglón {}: app «{}» · N «{}»", i + 1, x.chars().take(160).collect::<String>(), y.chars().take(160).collect::<String>());
        }
    }
    format!("una es más larga: app {} renglones, N {}", a.lines().count(), b.lines().count())
}

#[test]
#[ignore = "reads the real folders named in MICELYA_ZONES_READING"]
fn the_cut_by_zones_against_the_scripts_of_n() {
    let (Ok(reading), Ok(out)) = (std::env::var("MICELYA_ZONES_READING"), std::env::var("MICELYA_ZONES_OUT")) else {
        println!("MICELYA_ZONES_READING or MICELYA_ZONES_OUT is not set: nothing to measure");
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let started = std::time::Instant::now();
    let (_scratch, base) = base_in(&out, "base-corte.sqlite3");
    empty_the_cut(&base);
    base.conn()
        .execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, 'on')", [ZONES_READING])
        .unwrap();

    // The readings by zones, as a reader would leave them.
    let (mut given, mut missing) = (BTreeSet::new(), 0);
    for (hash, version_id) in pdfs(&base) {
        let folder = Path::new(&reading).join(&hash);
        if folder.is_dir() {
            base.put_zones(&hash, &readings_of(&folder), "zones-given").unwrap();
            given.insert(hash);
        } else {
            note_failure(&base.conn(), ZONES_STAGE, &hash, &format!("v{version_id}"), "no reading by zones given").unwrap();
            missing += 1;
        }
    }
    println!("readings by zones given: {} files; without one: {missing}", given.len());

    // The answers of N, by the key the app gives their question.
    let mut kept: BTreeMap<String, String> = BTreeMap::new();
    let mut seeded = 0;
    for folder in std::env::var("MICELYA_ZONES_ANSWERS").unwrap_or_default().split(';').filter(|f| !f.trim().is_empty()) {
        for entry in std::fs::read_dir(folder.trim()).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".json") {
                continue;
            }
            let r: Value = serde_json::from_str(&std::fs::read_to_string(entry.path()).unwrap()).unwrap();
            let (Some(question), answer) = (r["pregunta"].as_str(), &r["respuesta"]) else { continue };
            kept.insert(name.trim_end_matches(".json").to_string(), question.to_string());
            if answer.is_null() {
                continue;
            }
            seeded += base
                .conn()
                .execute(
                    "INSERT OR IGNORE INTO model_answer (question, task, prompt_version, engine, version_id, prompt, answer, asked_at)
                     VALUES (?1, ?2, ?3, ?4, NULL, ?5, ?6, 0)",
                    rusqlite::params![window::key(question), window::WINDOW_TASK, "cut-zones/1", engine(), question, answer.to_string()],
                )
                .unwrap();
        }
    }
    println!("answers of N kept in the base: {seeded}");

    // The cut, asking nothing.
    while base.cut_pending(16).unwrap() > 0 {}
    let asker = NoQuestions { asked: Mutex::new(Vec::new()) };
    let report = base.cut_ask_run(&asker, usize::MAX, Some(&given)).unwrap();
    println!(
        "windows: {} found their answer, {} did not (asked and refused); left without an answer: {}",
        report.kept, report.failed, report.left
    );
    while base.cut_parts_pending(16).unwrap() > 0 {}

    // What did not find its answer, and how its question differs.
    let mut lines = Vec::new();
    for prompt in asker.asked.lock().unwrap().iter() {
        let windows: Vec<(String, u32, u32)> = {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT resource_hash, first_page, last_page FROM cut_window WHERE prompt = ?1").unwrap();
            let rows = statement.query_map([prompt], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
            rows.map(|r| r.unwrap()).collect()
        };
        // The question as the app makes it, to ask it apart when its key
        // changed (the reading by zones of the app, part 2).
        let asked = out.join("preguntas");
        std::fs::create_dir_all(&asked).unwrap();
        for (hash, first, last) in windows {
            std::fs::write(asked.join(format!("{hash}-{first:03}-{last:03}.txt")), prompt).unwrap();
            let prefix = format!("{}-{first:03}-{last:03}-", &hash[..12]);
            let theirs = kept.iter().find(|(k, _)| k.starts_with(&prefix)).map(|(_, q)| q.clone());
            let why = match theirs {
                Some(q) => first_difference(prompt, &q),
                None => "N no tiene una pregunta con esas páginas".to_string(),
            };
            lines.push(format!("{} páginas {first}-{last}: {why}", &hash[..12]));
        }
    }
    std::fs::write(out.join("faltan.txt"), lines.join("\n")).unwrap();
    println!("windows without their answer: {} (faltan.txt)", lines.len());

    // The steps, as the scripts of N write them.
    let mut decisions = serde_json::Map::new();
    let mut steps = serde_json::Map::new();
    let (mut uncertain, mut windows_count) = (0, 0);
    for hash in &given {
        let s = base.cut_steps(hash).unwrap();
        if s.is_empty() || !s[0].zones {
            continue;
        }
        let mut d = Vec::new();
        let mut by_step = serde_json::Map::new();
        for step in s.iter().filter(|x| x.after > 0) {
            let hints: Value = step.hints.as_deref().map(|h| serde_json::from_str(h).unwrap()).unwrap_or(json!([]));
            d.push(json!({"after_page": step.after, "decision": decision_of(step.verdict), "regla": step.rule, "pistas": hints}));
            let v = if step.verdict != Verdict::Doubtful {
                json!([decision_of(step.verdict), "alta", "regla"])
            } else if let Some(says) = step.model_says {
                json!([if says { "corta" } else { "une" }, sure_of(step.model_confidence.as_deref()), "ia"])
            } else {
                json!([null, null, "sin respuesta"])
            };
            uncertain += step.uncertain() as usize;
            by_step.insert(step.after.to_string(), v);
        }
        decisions.insert(hash.clone(), Value::Array(d));
        steps.insert(hash.clone(), Value::Object(by_step));
        windows_count += base
            .conn()
            .query_row("SELECT count(*) FROM cut_window WHERE resource_hash = ?1", [hash], |row| row.get::<_, i64>(0))
            .unwrap();
    }
    std::fs::write(out.join("decisiones-app.json"), serde_json::to_string_pretty(&json!({"archivos": decisions})).unwrap()).unwrap();
    std::fs::write(out.join("saltos-app.json"), serde_json::to_string_pretty(&Value::Object(steps)).unwrap()).unwrap();
    let questions = base.cut_questions().unwrap();
    println!(
        "files cut by zones: {}; windows: {windows_count}; uncertain steps: {uncertain}; questions left by itself {} and on request {}",
        decisions.len(),
        questions.by_itself,
        questions.on_request
    );

    // The paper, shape and image pdfium gives, against those N had.
    if let Ok(signals) = std::env::var("MICELYA_ZONES_SIGNALS") {
        let (mut pages, mut differ, mut shown) = (0, 0, 0);
        for hash in &given {
            let path = Path::new(&signals).join(format!("{hash}.json"));
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let theirs: Value = serde_json::from_str(&text).unwrap();
            let Some(pdf) = super::file_path(&base.conn(), hash).unwrap() else { continue };
            let ours = by_zones::scan::scan_pdf(&pdf).unwrap_or_default();
            for (i, t) in theirs["pages"].as_array().unwrap().iter().enumerate() {
                pages += 1;
                let o = ours.get(i).cloned().unwrap_or_default();
                let image = o.image.as_ref().map(|m| json!([m.px.0, m.px.1, m.dpi, m.bpp, m.cs]));
                let their_image = t["main_image"].as_object().map(|m| json!([m["px"][0], m["px"][1], m["dpi"], m["bpp"], m["cs"]]));
                let same = json!([o.w, o.h, o.shape]) == json!([t["w"], t["h"], t["shape"]]) && image == their_image;
                if !same {
                    differ += 1;
                    if shown < 12 {
                        shown += 1;
                        println!(
                            "  {} page {}: app {} {} {} {:?} own {} · N {} {} {} {} own {}",
                            &hash[..12],
                            i + 1,
                            o.w,
                            o.h,
                            o.shape,
                            image,
                            o.own_chars,
                            t["w"],
                            t["h"],
                            t["shape"],
                            their_image.unwrap_or(Value::Null),
                            t["own_chars"]
                        );
                    }
                }
            }
        }
        println!("pdfium signals: {differ} of {pages} pages differ from N's");
    }
    base.conn().execute("VACUUM INTO ?1", [out.join("base-final.sqlite3").to_string_lossy().to_string()]).ok();
    println!("done in {:.0} s", started.elapsed().as_secs_f64());
}

#[test]
#[ignore = "reads the base named in MICELYA_ZONES_BASE or the folders of MICELYA_DOCBASE_CHECK"]
fn what_the_cut_of_before_would_ask_by_itself() {
    let Ok(out) = std::env::var("MICELYA_ZONES_OUT") else {
        println!("MICELYA_ZONES_OUT is not set: nothing to count");
        return;
    };
    let out = PathBuf::from(out);
    std::fs::create_dir_all(&out).unwrap();
    let (_scratch, base) = base_in(&out, "base-corte-de-hoy.sqlite3");
    empty_the_cut(&base);
    while base.cut_pending(16).unwrap() > 0 {}
    let q = base.cut_questions().unwrap();
    let conn = base.conn();
    let count = |sql: &str| conn.query_row(sql, [], |row| row.get::<_, i64>(0)).unwrap();
    let files_by_itself = count(&format!(
        "SELECT count(DISTINCT s.resource_hash) FROM cut_step s WHERE {} AND {}",
        super::WITHOUT_ANSWER,
        super::ASKED_BY_ITSELF
    ));
    let cut_files = count("SELECT count(DISTINCT resource_hash) FROM cut_step");
    let doubtful = count("SELECT count(*) FROM cut_step WHERE verdict = 'doubtful'");
    let by_rule = count("SELECT count(*) FROM cut_step WHERE verdict = 'new' AND after_page > 0");
    let result = json!({
        "archivos_cortados": cut_files,
        "saltos_dudosos": doubtful,
        "cortes_de_una_regla": by_rule,
        "preguntas_solas": q.by_itself,
        "archivos_con_preguntas_solas": files_by_itself,
        "en_la_primera_ronda": q.by_itself.min(crate::docbase::auto_questions::CAP),
        "esperan_a_una_persona": q.on_request,
    });
    println!("{result}");
    std::fs::write(out.join("corte-de-hoy.json"), serde_json::to_string_pretty(&result).unwrap()).unwrap();
}
