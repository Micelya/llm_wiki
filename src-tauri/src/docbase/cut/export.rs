//! Writes the signals of every step from one page to the next, for a small
//! classifier trained outside the app (the trial of D-610): one JSON with
//! only numbers and categories, never a name, a path or a text, kept
//! outside the repository. Not part of the normal test run.
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\carpeta" MICELYA_CUT_SIGNALS="C:\tmp\senales.json" MICELYA_ASK_OFFLINE=1 \
//!     cargo test --lib docbase::cut::export -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - `MICELYA_CUT_BASE`: a copy of a base to work on (it is changed: keep
//!   it outside the repository). Without it, `MICELYA_DOCBASE_CHECK` names
//!   the folders read into a scratch base, as in `similar/check_types.rs`
//!   (`MICELYA_DOCBASE_START`, `MICELYA_RECOGNIZED_SEED`,
//!   `MICELYA_RECOGNIZE`).
//! - `MICELYA_ANSWER_SEED`: answers put in the base first, as in `check.rs`.
//! - `MICELYA_CUT_ONLY`: JSON lists of objects with a `sha256` (files
//!   separated by `;`): the files whose questions are counted and asked.
//! - `MICELYA_CUT_ASK`: how many new questions Codex may take
//!   (`MICELYA_ASK_MODEL`, `MICELYA_ASK_REASONING`), one at a time, about
//!   every step of those files no rule decided, the even file's too: the
//!   variant «the rules and the model in every doubtful step». Without it
//!   nothing is asked, and the run says how many it would ask.
//! - `MICELYA_CUT_SIGNALS`: the JSON written: by file, its pages and steps.
//! - `MICELYA_ANSWER_SEED_WRITE`, `MICELYA_KNOWLEDGE_SAVE`: as in `check.rs`.

use std::collections::{BTreeSet, HashSet};
use std::path::Path;
use std::time::Instant;

use serde_json::{json, Value};

use crate::docbase::page_shapes::SHAPES_KIND;
use crate::docbase::similar::check_types::prepare;
use crate::docbase::DocBase;
use crate::source_volume::page_shape::{normalized_line, repeated_lines, visible_chars, PdfShapes, SHAPES_ENGINE};

use super::catalog::catalog;
use super::rules::{self, Verdict};
use super::signals::{self, Making, PageSignals};
use super::{CutAnswer, CUT_TASK};

fn making(m: Making) -> &'static str {
    match m {
        Making::Scan => "scan",
        Making::Digital => "digital",
        Making::Blank => "blank",
    }
}

/// The smaller over the larger: 1 when equal or both nothing.
fn ratio(a: f64, b: f64) -> f64 {
    if a.max(b) <= 0.0 {
        1.0
    } else {
        a.min(b) / a.max(b)
    }
}

/// Bits that differ between two perceptual prints of the same length.
fn distance(a: &str, b: &str) -> Option<u32> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    a.chars().zip(b.chars()).map(|(x, y)| Some((x.to_digit(16)? ^ y.to_digit(16)?).count_ones())).sum()
}

fn upper_share(text: &str) -> f64 {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    if letters.is_empty() {
        return 0.0;
    }
    letters.iter().filter(|c| c.is_uppercase()).count() as f64 / letters.len() as f64
}

fn words(lines: &[String]) -> HashSet<&str> {
    lines.iter().flat_map(|l| l.split(|c: char| !c.is_alphanumeric())).filter(|w| w.chars().count() >= 4).collect()
}

fn overlap(a: &HashSet<&str>, b: &HashSet<&str>) -> f64 {
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        a.intersection(b).count() as f64 / union as f64
    }
}

/// The last kept answer about a step's two pages, from any engine.
fn answer_of(base: &DocBase, key: &str) -> Option<CutAnswer> {
    let text: Option<String> = base
        .conn()
        .query_row(
            "SELECT answer FROM model_answer WHERE question = ?1 AND task = ?2 ORDER BY asked_at DESC LIMIT 1",
            rusqlite::params![key, CUT_TASK],
            |row| row.get(0),
        )
        .ok();
    text.and_then(|t| CutAnswer::parse(&t).ok())
}

/// What one page shows, in numbers and categories.
fn page_value(p: &PageSignals, shapes: &PdfShapes, index: usize, own: &[&str], print: Option<&(String, String, Option<i64>, Option<i64>)>) -> Value {
    let c = catalog();
    let t = &c.thresholds;
    let profile = shapes.pages.get(index);
    let tail = p.lines[p.lines.len().saturating_sub(t.tail_lines)..].join(" ");
    let head = p.lines.iter().take(t.head_lines).cloned().collect::<Vec<_>>().join(" ");
    let first_letter = own.first().and_then(|l| l.chars().find(|c| c.is_alphabetic()));
    let last_char = own.last().and_then(|l| l.trim_end().chars().last());
    let image = p.image.clone().unwrap_or_default();
    json!({
        "page": p.page,
        "making": making(p.making),
        "shape": profile.map(|f| format!("{:?}", f.shape)),
        "layer": profile.map(|f| format!("{:?}", f.text_layer)),
        "read": profile.map(|f| f.read),
        "chars": profile.map(|f| f.chars),
        "own_chars": profile.map(|f| f.own_chars),
        "own_lines": profile.map(|f| f.own_lines),
        "short_lines": profile.map(|f| f.short_lines),
        "single_letters": profile.map(|f| f.single_letters),
        "largest_image": profile.map(|f| f.largest_image),
        "images_cover": profile.map(|f| f.images_cover),
        "group_values": profile.map(|f| f.group_values),
        "fields": profile.map(|f| f.fields),
        "drawn": profile.map(|f| f.drawn),
        "width": p.size.0,
        "height": p.size.1,
        "dpi": image.dpi,
        "pixels": image.pixels.0 as u64 * image.pixels.1 as u64,
        "color": image.color,
        "page_of_x": p.page_of.first().map(|(x, _)| *x),
        "page_of_y": p.page_of.first().map(|(_, y)| *y),
        "page_of_last": p.page_of.last().is_some_and(|(x, y)| x == y && *y > 1),
        "codes": p.codes.len(),
        "serials": p.serials.len(),
        "machine_zone": p.machine_zone,
        "has_type": rules::type_of(p).is_some(),
        "signed_at_the_end": rules::signed_at_the_end(p),
        "closer": c.closers.iter().any(|x| tail.contains(x.as_str())),
        "opener": c.openers.iter().any(|x| head.contains(x.as_str())),
        "starts_lower": first_letter.is_some_and(char::is_lowercase),
        "ends_stop": last_char.is_some_and(|c| matches!(c, '.' | '!' | '?')),
        "ends_open": last_char.is_some_and(|c| c.is_alphanumeric() || matches!(c, ',' | ';' | '-')),
        "head_upper": upper_share(&own.iter().take(2).cloned().collect::<Vec<_>>().join(" ")),
        "has_signature": print.and_then(|p| p.2),
        "has_handwriting": print.and_then(|p| p.3),
    })
}

/// The pages and the steps of one file, and how long they took.
fn file_value(base: &DocBase, hash: &str) -> Option<Value> {
    let (body, version, document): (String, i64, Option<i64>) = base
        .conn()
        .query_row(
            "SELECT d.body, f.version_id, v.document_id FROM file f JOIN version v ON v.id = f.version_id
             JOIN derivative d ON d.version_id = f.version_id AND d.kind = ?2 AND d.engine = ?3 WHERE f.hash = ?1",
            rusqlite::params![hash, SHAPES_KIND, SHAPES_ENGINE],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .ok()?;
    let shapes: PdfShapes = serde_json::from_str(&body).unwrap_or_default();
    let path = super::file_path(&base.conn(), hash).ok().flatten()?;
    let started = Instant::now();
    let scans = signals::scan_pages(&path).unwrap_or_default();
    let scan_micros = started.elapsed().as_micros() as u64;
    let started = Instant::now();
    let texts = super::page_texts(&base.conn(), hash, &shapes).ok()?;
    let pages = signals::signals(&shapes, &scans, &texts);
    let prints: Vec<(u32, (String, String, Option<i64>, Option<i64>))> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT page_no, exact_hash, phash, has_signature, has_handwriting FROM resource_page WHERE resource_hash = ?1")
            .ok()?;
        let rows = statement
            .query_map([hash], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))))
            .ok()?;
        rows.filter_map(Result::ok).collect()
    };
    let print = |page: u32| prints.iter().find(|(p, _)| *p == page).map(|(_, v)| v);
    let repeated = repeated_lines(&texts);
    let own: Vec<Vec<&str>> = texts
        .iter()
        .map(|t| t.lines().filter(|l| visible_chars(l) > 0 && !repeated.contains(&normalized_line(l))).collect())
        .collect();
    let empty: Vec<&str> = Vec::new();
    let page_values: Vec<Value> = pages
        .iter()
        .enumerate()
        .map(|(i, p)| page_value(p, &shapes, i, own.get(i).unwrap_or(&empty), print(p.page)))
        .collect();
    let ruled: Vec<(Verdict, &'static str)> = pages.windows(2).map(|w| rules::step(&w[0], &w[1])).collect();
    let verdicts: Vec<Verdict> = ruled.iter().map(|(v, _)| *v).collect();
    let even = rules::uniform(&pages, &verdicts);
    let kept = base.cut_steps(hash).unwrap_or_default();
    let t = &catalog().thresholds;
    let mut steps = Vec::new();
    for (pair, (verdict, rule)) in pages.windows(2).zip(&ruled) {
        let (a, b) = (&pair[0], &pair[1]);
        let signed = rules::signed_at_the_end(a);
        let (pa, pb) = (print(a.page), print(b.page));
        let key = match (pa, pb) {
            (Some(x), Some(y)) => Some(super::ask::key(&x.0, &y.0)),
            _ => None,
        };
        let answer = key.as_deref().and_then(|k| answer_of(base, k));
        let app = kept.iter().find(|s| s.after == a.page);
        let (ia, ib) = (a.image.clone().unwrap_or_default(), b.image.clone().unwrap_or_default());
        steps.push(json!({
            "after_page": a.page,
            "rule_verdict": verdict.as_str(),
            "rule": rule,
            "app_verdict": app.map(|s| s.verdict.as_str()),
            "app_rule": app.map(|s| s.rule.clone()),
            "even_file": even,
            "even_step": even && *verdict == Verdict::Doubtful && *rule == rules::NONE && !signed,
            "signed_before": signed,
            "phash_distance": match (pa, pb) { (Some(x), Some(y)) => distance(&x.1, &y.1), _ => None },
            "same_print": matches!((pa, pb), (Some(x), Some(y)) if x.0 == y.0),
            "width_ratio": ratio(a.size.0, b.size.0),
            "height_ratio": ratio(a.size.1, b.size.1),
            "turned": (a.size.0 > a.size.1) != (b.size.0 > b.size.1),
            "dpi_ratio": ratio(ia.dpi as f64, ib.dpi as f64),
            "same_color": ia.color == ib.color,
            "same_pixels": ia.pixels == ib.pixels && ia.pixels != (0, 0),
            "shared_codes": a.codes.intersection(&b.codes).next().is_some(),
            "serial_next": a.serials.iter().any(|(la, na)| b.serials.iter().any(|(lb, nb)| la == lb && *nb == na + 1)),
            "word_overlap": overlap(&words(&a.lines), &words(&b.lines)),
            "tail_head_overlap": overlap(
                &words(&a.lines[a.lines.len().saturating_sub(t.tail_lines)..]),
                &words(&b.lines[..b.lines.len().min(t.head_lines)])
            ),
            "question": key.is_some(),
            "model_says": answer.as_ref().map(|x| x.second_starts_new_document),
            "model_confidence": answer.as_ref().map(|x| x.confidence.clone()),
        }));
    }
    let signal_micros = started.elapsed().as_micros() as u64;
    Some(json!({
        "sha256": hash,
        "pages": pages.len(),
        "version": version,
        "document": document,
        "scan_micros": scan_micros,
        "signal_micros": signal_micros,
        "page_signals": page_values,
        "steps": steps,
    }))
}

/// The hashes named in the label files of `MICELYA_CUT_ONLY`.
fn only_files() -> Option<BTreeSet<String>> {
    let files = std::env::var("MICELYA_CUT_ONLY").ok()?;
    let mut only = BTreeSet::new();
    for file in files.split(';').filter(|f| !f.trim().is_empty()) {
        let rows: Vec<Value> = serde_json::from_str(&std::fs::read_to_string(file.trim()).unwrap()).unwrap();
        only.extend(rows.iter().filter_map(|r| r["sha256"].as_str().map(str::to_lowercase)));
    }
    Some(only)
}

#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK or the copy in MICELYA_CUT_BASE"]
fn the_signals_of_every_step() {
    let Ok(out) = std::env::var("MICELYA_CUT_SIGNALS") else {
        println!("MICELYA_CUT_SIGNALS is not set: nothing to write");
        return;
    };
    let (_scratch, base) = if let Ok(copy) = std::env::var("MICELYA_CUT_BASE") {
        (None, DocBase::open(Path::new(&copy)).unwrap())
    } else if let Ok(folders) = std::env::var("MICELYA_DOCBASE_CHECK") {
        let (scratch, base) = prepare(&folders);
        (Some(scratch), base)
    } else {
        println!("MICELYA_CUT_BASE or MICELYA_DOCBASE_CHECK is not set: nothing to read");
        return;
    };
    if let Ok(seeds) = std::env::var("MICELYA_ANSWER_SEED") {
        println!("answers seeded: {}", super::check::seed_answers(&base, &seeds));
    }
    while base.profile_pending(16).unwrap() > 0 {}
    while base.cut_pending(16).unwrap() > 0 {}
    // The cuts given (chain N): decided as a person would, nothing asked.
    let given = super::check::apply_given_cuts(&base);
    let hashes: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT DISTINCT resource_hash FROM cut_step ORDER BY 1").unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.map(|r| r.unwrap()).collect()
    };
    let only = only_files();
    let counted: Vec<&String> = hashes.iter().filter(|h| only.as_ref().is_none_or(|o| o.contains(*h))).collect();
    // Every step no rule decided, the even file's too, without a kept answer.
    let missing = |base: &DocBase| -> i64 {
        counted
            .iter()
            .map(|h| {
                base.conn()
                    .query_row(
                        "SELECT count(DISTINCT s.question) FROM cut_step s
                         WHERE s.resource_hash = ?1 AND (s.verdict = 'doubtful' OR s.rule = 'uniform') AND s.question IS NOT NULL
                           AND NOT EXISTS (SELECT 1 FROM model_answer m WHERE m.question = s.question AND m.task = ?2)",
                        rusqlite::params![h, CUT_TASK],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap()
            })
            .sum()
    };
    println!("files cut: {} · counted: {} · steps no rule decided without an answer kept: {}", hashes.len(), counted.len(), missing(&base));
    let mut cut_asked = 0;
    if let Ok(limit) = std::env::var("MICELYA_CUT_ASK") {
        let only = only.as_ref().expect("MICELYA_CUT_ASK needs MICELYA_CUT_ONLY");
        for h in only {
            base.conn()
                .execute("UPDATE cut_step SET verdict = 'doubtful', rule = 'none' WHERE resource_hash = ?1 AND rule = ?2", rusqlite::params![h, rules::UNIFORM])
                .unwrap();
        }
        // The model with its answers kept as they come, in `MICELYA_ASK_CACHE`:
        // a stage cut short and run again asks only what it did not get (D-678).
        let asker = crate::docbase::knowledge::check::recorded();
        let report = base.cut_ask_run(&asker, limit.trim().parse().unwrap(), Some(only)).unwrap();
        println!(
            "Codex: {} asked, {} failed, {} from the answers kept, {:.0} s; left without an answer: {}",
            report.asked,
            report.failed,
            report.kept,
            report.seconds,
            missing(&base)
        );
        for e in report.errors.iter().take(3) {
            println!("  error: {}", e.chars().take(200).collect::<String>());
        }
        cut_asked = asker.asked.load(std::sync::atomic::Ordering::SeqCst);
    }
    // The questions of this run by kind (D-672): only the cut asks here.
    println!(
        "questions to the model in this run, by kind: cut {cut_asked}, type 0, reading 0, entities 0, identity 0, pages 0{}",
        if given.is_some() { " (cuts given: the cut asks nothing)" } else { "" }
    );
    let files: Vec<Value> = hashes.iter().filter_map(|h| file_value(&base, h)).collect();
    let relations: Vec<Value> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT a, b, relation FROM similar_pair ORDER BY a, b").unwrap();
        let rows = statement
            .query_map([], |row| Ok(json!({"a": row.get::<_, i64>(0)?, "b": row.get::<_, i64>(1)?, "relation": row.get::<_, String>(2)?})))
            .unwrap();
        rows.filter_map(Result::ok).collect()
    };
    let steps: usize = files.iter().map(|f| f["steps"].as_array().map_or(0, Vec::len)).sum();
    println!("signals written: {} files, {steps} steps, {} relations", files.len(), relations.len());
    std::fs::write(&out, serde_json::to_string_pretty(&json!({"files": files, "relations": relations})).unwrap()).unwrap();
    if let Ok(path) = std::env::var("MICELYA_ANSWER_SEED_WRITE") {
        let answers = super::check::written_answers(&base);
        println!("answers about cuts written: {}", answers.as_array().map_or(0, Vec::len));
        std::fs::write(path, serde_json::to_string_pretty(&answers).unwrap()).unwrap();
    }
    crate::docbase::knowledge::check::save_copy(&base);
}
