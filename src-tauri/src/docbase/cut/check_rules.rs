//! Measures the rules of the cut by zones against the labels (3b of 2 nov)
//! with the text the app reads today, in a new base: the files are copied
//! to an empty folder, read by zones with the reader of this computer and
//! cut with the rules, asking nothing. It writes, for each file, what each
//! step says (the rule that decided it or `dudoso`, and its hints), the
//! windows the app would ask and a little of each page; the labels are
//! compared outside (`etapa3b\guiones\medir.py`): they never come in here.
//!
//! Not part of the normal test run: it reads real files and writes their
//! text; keep the output outside the repository. It can be stopped and run
//! again: the base keeps what it read, and only reads what is missing
//! (`MICELYA_CUT_MINUTES` of reading each time, 9 by default).
//!
//! ```text
//! MICELYA_OCR_DIR="C:\...\src-tauri\ocr" MICELYA_CUT_FILES="C:\lista.txt" MICELYA_CUT_DIR="C:\base-nueva" \
//! MICELYA_CUT_OUT="C:\salida.json" cargo test --lib docbase::cut::check_rules -- --ignored --nocapture
//! ```
//!
//! - `MICELYA_CUT_FILES`: a text file with the path of each file, one by line.
//! - `MICELYA_CUT_DIR`: the folder of the new base; it must not exist, or be
//!   one this test made (then it goes on reading).

use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::by_zones::{self, page, scan, ZONES_READING};
use crate::docbase::DocBase;
use crate::source_volume::recognition_local::LocalRecognizer;

/// A little of the page: how it is made and the start of its own text.
fn page_line(f: &page::Features, r: &by_zones::PageReading) -> Value {
    let own: Vec<String> = r
        .zonas
        .iter()
        .flat_map(|z| z.renglones.iter().map(move |l| format!("{}{}:{}", if l.seguro == Some(true) { "" } else { "?" }, z.clase, l.texto)))
        .collect();
    json!({
        "pagina": f.page, "forma": f.forma, "hechura": f.scan.shape, "capa_propia": f.scan.own_chars,
        "renglones": own.len(), "palabras_firmes": f.body.iter().map(|l| l.split_whitespace().count()).sum::<usize>(),
        "firma": f.sign_cues, "texto": own.iter().take(8).map(|t| t.chars().take(70).collect::<String>()).collect::<Vec<_>>(),
    })
}

#[test]
#[ignore = "reads the real files named in MICELYA_CUT_FILES"]
fn measure_the_rules_of_the_cut() {
    let (Ok(list), Ok(dir), Ok(out)) =
        (std::env::var("MICELYA_CUT_FILES"), std::env::var("MICELYA_CUT_DIR"), std::env::var("MICELYA_CUT_OUT"))
    else {
        println!("MICELYA_CUT_FILES, MICELYA_CUT_DIR or MICELYA_CUT_OUT is not set: nothing to measure");
        return;
    };
    let minutes: u64 = std::env::var("MICELYA_CUT_MINUTES").ok().and_then(|m| m.parse().ok()).unwrap_or(9);
    let dir = Path::new(&dir);
    let files = dir.join("archivos");
    let base_path = dir.join("base.sqlite3");
    let new = !base_path.exists();
    if new {
        // From zero: an empty folder, no base, no readings of before.
        assert!(!dir.exists() || std::fs::read_dir(dir).unwrap().next().is_none(), "{} must be empty", dir.display());
        std::fs::create_dir_all(&files).unwrap();
        for (i, line) in std::fs::read_to_string(&list).unwrap().lines().filter(|l| !l.trim().is_empty()).enumerate() {
            std::fs::copy(line.trim(), files.join(format!("a{:03}.pdf", i + 1))).unwrap();
        }
    }
    let base = DocBase::open(&base_path).unwrap();
    if new {
        let origin = base.add_origin("O", &files, &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        base.conn().execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, 'on')", [ZONES_READING]).unwrap();
    }
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    while base.profile_pending(10).unwrap() > 0 {}
    let reader = LocalRecognizer::new();
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(minutes * 60) && base.read_zones_step_with(&reader).unwrap() {}
    let left = base.zones_waiting(10_000).unwrap().len();
    let rows: Vec<(String, String, String)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare(
                "SELECT f.hash, min(l.rel_path), z.body FROM file f JOIN location l ON l.file_hash = f.hash
                 JOIN derivative z ON z.version_id = f.version_id AND z.kind = 'zones' GROUP BY f.hash ORDER BY min(l.rel_path)",
            )
            .unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let mut all = serde_json::Map::new();
    for (hash, rel, body) in rows {
        let mut readings = by_zones::readings_of(&body).unwrap();
        readings.sort_by_key(|r| r.page);
        let scans = scan::scan_pdf(&files.join(&rel)).unwrap_or_default();
        let cut = by_zones::cut(&readings, &scans);
        let pages: Vec<Value> = readings
            .iter()
            .enumerate()
            .map(|(i, r)| page_line(&page::features(r, &scans.get(i).cloned().unwrap_or_default()), r))
            .collect();
        let steps: Vec<Value> = cut
            .decisions
            .iter()
            .map(|d| json!({ "despues": d.after, "decision": d.decision, "regla": d.rule, "pistas": d.hints }))
            .collect();
        let windows: Vec<Value> = cut.windows.iter().map(|w| json!([w.first, w.last])).collect();
        all.insert(hash, json!({ "archivo": rel, "paginas": pages, "saltos": steps, "tramos": windows }));
    }
    let n = all.len();
    std::fs::write(&out, serde_json::to_string_pretty(&json!({ "sin_leer": left, "archivos": all })).unwrap()).unwrap();
    println!("read by zones: {n}; still to read: {left}; out: {out}");
}
