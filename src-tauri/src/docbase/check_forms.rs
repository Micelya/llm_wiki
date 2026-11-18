//! Reads real scans with the reader of this computer, in a new base and
//! without a question, and checks the text of the pages of a form (E3 of 2
//! nov): what the reader read after the fields is text of its page in the
//! three places that cut the text by pages (the signature, the quotes and
//! the page numbers). It also counts the share of safe words of each page
//! and each file, below the thresholds of the type of low confidence and
//! «no legible». Not part of the normal run: it reads real files (only
//! reads them: a copy goes to a temporary folder); keep the output outside
//! the repository.
//!
//! ```text
//! MICELYA_OCR_DIR="C:\...\src-tauri\ocr" MICELYA_TEXT_FILES="C:\a.pdf;C:\b.pdf" MICELYA_TEXT_OUT="C:\salida" \
//!     cargo test --lib docbase::check_forms -- --ignored --nocapture
//! ```

use std::collections::BTreeMap;
use std::path::Path;

use crate::commands::extract_images::FORM_FIELDS_HEADING;
use crate::docbase::cut::by_zones::ZONES_READING;
use crate::docbase::safe_words::{self, SafeWords};
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;
use crate::source_volume::catalog::{normalized_words, RECOGNIZED_MARKER_START};
use crate::source_volume::recognition_local::LocalRecognizer;

const SHARES: [f64; 5] = [0.10, 0.20, 0.30, 0.40, 0.50];

/// The words a reader read after the fields of a form, page by page.
fn after_the_fields(text: &str) -> BTreeMap<u32, Vec<String>> {
    let mut out: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    let (mut page, mut fields, mut read) = (1u32, false, false);
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(n) = trimmed.strip_prefix("## Page ").and_then(|n| n.parse().ok()) {
            (page, fields, read) = (n, false, false);
        } else if trimmed == FORM_FIELDS_HEADING {
            fields = true;
        } else if trimmed.starts_with(RECOGNIZED_MARKER_START) {
            read = fields;
        } else if read {
            out.entry(page).or_default().extend(normalized_words(crate::source_volume::bands::without_mark(trimmed)));
        }
    }
    out
}

#[test]
#[ignore = "reads the real files named in MICELYA_TEXT_FILES"]
fn the_pages_of_a_form_keep_their_scanned_text_and_the_shares_are_counted() {
    let (Ok(files), Ok(out)) = (std::env::var("MICELYA_TEXT_FILES"), std::env::var("MICELYA_TEXT_OUT")) else {
        println!("MICELYA_TEXT_FILES or MICELYA_TEXT_OUT is not set: nothing to read");
        return;
    };
    let out = Path::new(&out);
    std::fs::create_dir_all(out).unwrap();
    let dir = TempDir::new();
    let names: Vec<String> = files
        .split(';')
        .filter(|f| !f.trim().is_empty())
        .enumerate()
        .map(|(i, f)| {
            let path = Path::new(f.trim());
            let ext = path.extension().map(|x| x.to_string_lossy().to_lowercase()).unwrap_or_default();
            let name = format!("documento-{}.{ext}", i + 1);
            dir.write(&format!("o/{name}"), &std::fs::read(path).unwrap());
            println!("{name}: {}", path.display());
            name
        })
        .collect();
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    let reader = LocalRecognizer::new();
    let of = |name: &str| -> (i64, String) {
        base.conn()
            .query_row("SELECT f.version_id, f.hash FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap()
    };
    for name in &names {
        base.recognize_content(of(name).0, &reader).unwrap();
    }
    base.conn().execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, 'on')", [ZONES_READING]).unwrap();
    while base.profile_pending(10).unwrap() > 0 {}
    while base.read_zones_step_with(&reader).unwrap() {}
    let mut summary = Vec::new();
    let mut below_pages = [0usize; 5];
    let mut below_files = [0usize; 5];
    let mut pages_counted = 0;
    for name in &names {
        let (version, hash) = of(name);
        let body: String =
            base.conn().query_row("SELECT body FROM derivative WHERE version_id = ?1 AND kind = 'text'", [version], |row| row.get(0)).unwrap();
        std::fs::write(out.join(format!("{name}.txt")), &body).unwrap();
        // The forms: what the old code threw away, now in the three places.
        let lost = after_the_fields(&body);
        let signature_pages = crate::docbase::similar::text::pages(&body);
        let signature: Vec<String> = signature_pages.iter().flatten().flat_map(|line| normalized_words(line)).collect();
        let located = crate::docbase::knowledge::quote::Located::new(&body);
        let comparable = crate::docbase::similar::format::comparable(&body);
        let mut wrong = Vec::new();
        let mut words_after_fields = 0;
        for (page, words) in &lost {
            words_after_fields += words.len();
            let first: Vec<&String> = words.iter().take(6).collect();
            if !first.iter().all(|w| signature.contains(w)) {
                wrong.push(format!("page {page}: not in the signature: {first:?}"));
            }
            let quote = words.iter().take(6).cloned().collect::<Vec<_>>().join(" ");
            if !quote.is_empty() && located.locate(&quote, None).is_some_and(|at| at != Some(i64::from(*page))) {
                wrong.push(format!("page {page}: the quote «{quote}» is on page {:?}", located.locate(&quote, None)));
            }
            if !first.iter().all(|w| normalized_words(&comparable).contains(w)) {
                wrong.push(format!("page {page}: not in the comparable text: {first:?}"));
            }
        }
        // The shares, of the file and of each page that gave its text.
        let zones = base.zones_of(&hash).unwrap().unwrap_or_default();
        let discarded = safe_words::discarded_by_page(&zones);
        let whole = safe_words::measure(&base.conn(), version, &body).unwrap();
        let mut pages = Vec::new();
        for page in discarded.keys() {
            let section = if name.ends_with(".pdf") { crate::docbase::cut::parts::pages_text(&body, *page, *page) } else { body.clone() };
            let words: SafeWords = safe_words::count(&section, &discarded);
            if let Some(share) = words.share() {
                pages_counted += 1;
                for (i, limit) in SHARES.iter().enumerate() {
                    below_pages[i] += usize::from(share < *limit);
                }
                pages.push(serde_json::json!({ "pagina": page, "firmes": words.firm, "aproximadas": words.approximate,
                    "descartadas": words.discarded, "proporcion": (share * 100.0).round() / 100.0 }));
            }
        }
        if let Some(share) = whole.share() {
            for (i, limit) in SHARES.iter().enumerate() {
                below_files[i] += usize::from(share < *limit);
            }
        }
        let line = serde_json::json!({
            "documento": name, "paginas_con_formulario_y_escaneo": lost.len(), "palabras_despues_de_los_campos": words_after_fields,
            "errores": wrong, "archivo": { "firmes": whole.firm, "aproximadas": whole.approximate, "descartadas": whole.discarded,
            "proporcion": whole.share().map(|s| (s * 100.0).round() / 100.0) }, "paginas": pages,
        });
        println!("{line}");
        summary.push(line);
    }
    let below: Vec<serde_json::Value> = SHARES
        .iter()
        .enumerate()
        .map(|(i, limit)| serde_json::json!({ "umbral": limit, "paginas_debajo": below_pages[i], "archivos_debajo": below_files[i] }))
        .collect();
    let all = serde_json::json!({ "paginas_medidas": pages_counted, "debajo": below, "documentos": summary });
    std::fs::write(out.join("resumen.json"), serde_json::to_string_pretty(&all).unwrap()).unwrap();
    println!("{}", serde_json::to_string_pretty(&all["debajo"]).unwrap());
    for document in &summary {
        assert!(document["errores"].as_array().unwrap().is_empty(), "{document}");
    }
}
