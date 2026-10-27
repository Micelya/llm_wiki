//! Measures the engine of this computer (D-558, `recognition_local`) over
//! real folders, against the answer sheets of printed passages, of a form
//! and of handwritten values.
//!
//! Not part of the normal test run; it needs the models
//! (`node scripts/fetch-ocr-models.mjs`). The folders are only read; the
//! images of the pages go to a scratch folder removed at the end. It
//! prints numbers only, never names nor texts (R-24).
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\casos" MICELYA_OCR_SHEETS="C:\hoja" [MICELYA_OCR_MAX_PAGES=400] \
//!     cargo test --lib docbase::similar::check_local_reading -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - The sheets (`MICELYA_OCR_SHEETS`): `partes.json` names the files and
//!   their sha256; `impreso.json` has passages typed by eye (`04-p14`: file
//!   4, page 14), measured as the character error rate of the best window
//!   of the page's text, spaces and dashes left out (as `cer.py` of the
//!   Docling study); `formulario-01.json`, the values of the form of file
//!   1, counted when they are in its text; `manuscrito.json`, handwritten
//!   values by page, counted when they enter the text exactly or nearly
//!   (a quarter of their letters wrong at most). Each one at several
//!   confidence thresholds, from what the engine read once.
//! - The whole folder: the pages of its PDFs the ficha of shape sends to
//!   recognition, how many came out with text at the catalog's threshold,
//!   the seconds per page and the peak memory of the process. With
//!   `MICELYA_OCR_MAX_PAGES`, that many pages spread over all of them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rapidocr_core::RapidOcr;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::docbase::testing::TempDir;
use crate::source_volume::page_shape::profile_pdf;
use crate::source_volume::recognition::{render_page, UNREADABLE};
use crate::source_volume::recognition_local::{catalog, development_dir, read_lines, start_in, text_of, ReadLine};

const THRESHOLDS: [f32; 9] = [0.0, 0.5, 0.7, 0.8, 0.85, 0.9, 0.93, 0.95, 0.97];

#[derive(Deserialize)]
struct Parts {
    files: Vec<PartFile>,
}

#[derive(Deserialize)]
struct PartFile {
    sha256: String,
}

#[derive(Deserialize)]
struct Form {
    pairs: Vec<Vec<String>>,
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> T {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))).unwrap()
}

fn pdfs_under(root: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("pdf")))
        .collect();
    found.sort();
    found
}

fn sha256_of(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap();
    Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
}

/// `cer.py`'s normal form: spaces, dashes and table marks left out.
fn cer_norm(text: &str) -> Vec<char> {
    text.chars().filter(|c| !(c.is_whitespace() || "-–—_|#*".contains(*c))).collect()
}

/// Letters and digits only, in lower case: to find a value in a text.
fn value_norm(text: &str) -> Vec<char> {
    text.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// The fewest edits that turn `needle` into some window of `hay`.
fn best_window_distance(needle: &[char], hay: &[char]) -> usize {
    let n = needle.len();
    let mut column: Vec<usize> = (0..=n).collect();
    let mut best = column[n];
    for h in hay {
        let mut diagonal = column[0];
        column[0] = 0;
        for i in 1..=n {
            let up = column[i];
            column[i] = (up + 1).min(column[i - 1] + 1).min(diagonal + usize::from(needle[i - 1] != *h));
            diagonal = up;
        }
        best = best.min(column[n]);
    }
    best
}

fn cer_pct(reference: &str, text: &str) -> f64 {
    let reference = cer_norm(reference);
    let text = cer_norm(text);
    if text.is_empty() {
        return 100.0;
    }
    (1000.0 * best_window_distance(&reference, &text) as f64 / reference.len() as f64).round() / 10.0
}

/// Whether `value` is in `text`: exactly, and nearly.
fn found(value: &str, text: &str) -> (bool, bool) {
    let value = value_norm(value);
    let text = value_norm(text);
    if value.is_empty() {
        return (false, false);
    }
    let distance = best_window_distance(&value, &text);
    (distance == 0, distance * 4 <= value.len())
}

struct Reader {
    ocr: RapidOcr,
    scratch: TempDir,
}

impl Reader {
    /// The lines of a page, and the seconds it took to draw and to read it.
    fn page(&mut self, pdf: &Path, page: u32) -> Result<(Vec<ReadLine>, f64, f64), String> {
        let at = Instant::now();
        let image = render_page(pdf, page, self.scratch.path())?;
        let drawn = at.elapsed().as_secs_f64();
        let at = Instant::now();
        let lines = read_lines(&mut self.ocr, &image);
        let read = at.elapsed().as_secs_f64();
        let _ = std::fs::remove_file(&image);
        Ok((lines?, drawn, read))
    }
}

fn peak_memory_mb() -> Option<f64> {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!("(Get-Process -Id {}).PeakWorkingSet64", std::process::id())])
        .output()
        .ok()?;
    let bytes: f64 = String::from_utf8_lossy(&output.stdout).trim().parse().ok()?;
    Some((bytes / 1_048_576.0).round())
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[test]
#[ignore]
fn the_engine_of_this_computer_against_the_sheets_and_the_folder() {
    let (Some(folder), Some(sheets)) = (std::env::var_os("MICELYA_DOCBASE_CHECK"), std::env::var_os("MICELYA_OCR_SHEETS")) else {
        eprintln!("skipped: MICELYA_DOCBASE_CHECK and MICELYA_OCR_SHEETS");
        return;
    };
    let (folder, sheets) = (PathBuf::from(folder), PathBuf::from(sheets));
    let dir = std::env::var_os("MICELYA_OCR_DIR").map_or_else(development_dir, PathBuf::from);
    let at = Instant::now();
    let ocr = start_in(&dir).unwrap();
    let start_s = round2(at.elapsed().as_secs_f64());
    let mut reader = Reader { ocr, scratch: TempDir::new() };
    let pdfs = pdfs_under(&folder);

    // The six files of the sheets, by their sha256.
    let parts: Parts = read_json(&sheets.join("partes.json"));
    let mut files: BTreeMap<usize, PathBuf> = BTreeMap::new();
    for pdf in &pdfs {
        let hash = sha256_of(pdf);
        if let Some(index) = parts.files.iter().position(|part| part.sha256 == hash) {
            files.entry(index + 1).or_insert_with(|| pdf.clone());
        }
    }
    let file = |n: usize| files.get(&n).unwrap_or_else(|| panic!("file {n} of the sheet not found"));
    let lines_of = |reader: &mut Reader, n: usize, page: u32| reader.page(file(n), page).unwrap().0;
    let key = |k: &str| -> (usize, u32) {
        let (n, page) = k.split_once("-p").unwrap();
        (n.parse().unwrap(), page.parse().unwrap())
    };

    // Printed passages: the error of characters at each threshold.
    let printed: BTreeMap<String, String> = read_json(&sheets.join("impreso.json"));
    let mut printed_cer = BTreeMap::new();
    for (k, reference) in &printed {
        let (n, page) = key(k);
        let lines = lines_of(&mut reader, n, page);
        let by: Vec<(String, f64)> = THRESHOLDS.iter().map(|t| (format!("{t}"), cer_pct(reference, &text_of(&lines, *t)))).collect();
        printed_cer.insert(k.clone(), serde_json::json!({ "caracteres": cer_norm(reference).len(), "lineas": lines.len(), "cer_pct_por_umbral": by }));
    }

    // The form of file 1: its values found in its text.
    let form: Form = read_json(&sheets.join("formulario-01.json"));
    let pages = profile_pdf(file(1)).unwrap().pages.len() as u32;
    let form_lines: Vec<ReadLine> = (1..=pages).flat_map(|page| lines_of(&mut reader, 1, page)).collect();
    let form_found: Vec<(String, usize, usize)> = THRESHOLDS
        .iter()
        .map(|t| {
            let text = text_of(&form_lines, *t);
            let hits: Vec<(bool, bool)> = form.pairs.iter().map(|pair| found(&pair[1], &text)).collect();
            (format!("{t}"), hits.iter().filter(|h| h.0).count(), hits.iter().filter(|h| h.1).count())
        })
        .collect();

    // Handwritten values: how many enter the text, exactly or nearly.
    let handwritten: BTreeMap<String, Vec<String>> = read_json(&sheets.join("manuscrito.json"));
    let mut hand_lines: Vec<(Vec<ReadLine>, &Vec<String>)> = Vec::new();
    for (k, values) in &handwritten {
        let (n, page) = key(k);
        hand_lines.push((lines_of(&mut reader, n, page), values));
    }
    let total_values: usize = handwritten.values().map(Vec::len).sum();
    let hand_found: Vec<serde_json::Value> = THRESHOLDS
        .iter()
        .map(|t| {
            let (mut exact, mut near, mut kept, mut lines) = (0, 0, 0, 0);
            for (page_lines, values) in &hand_lines {
                let text = text_of(page_lines, *t);
                for value in values.iter() {
                    let (e, n) = found(value, &text);
                    exact += usize::from(e);
                    near += usize::from(n);
                }
                lines += page_lines.len();
                kept += page_lines.iter().filter(|line| line.score >= *t).count();
            }
            serde_json::json!({ "umbral": t, "exactos": exact, "casi": near, "lineas": lines, "lineas_que_entran": kept })
        })
        .collect();

    // The whole folder: the pages its fichas of shape send to recognition.
    let max_pages: usize = std::env::var("MICELYA_OCR_MAX_PAGES").ok().and_then(|v| v.parse().ok()).unwrap_or(usize::MAX);
    let mut to_read: Vec<(usize, u32)> = Vec::new();
    let mut unreadable_pdfs = 0;
    for (index, pdf) in pdfs.iter().enumerate() {
        match profile_pdf(pdf) {
            Ok(shapes) => to_read.extend(shapes.to_read().into_iter().map(|page| (index, page))),
            Err(_) => unreadable_pdfs += 1,
        }
    }
    let total = to_read.len();
    let step = total.div_ceil(max_pages.max(1)).max(1);
    let sample: Vec<(usize, u32)> = to_read.iter().copied().step_by(step).take(max_pages).collect();
    let threshold = catalog().min_line_confidence;
    let (mut with_text, mut unreadable, mut blank, mut failed) = (0, 0, 0, 0);
    let (mut drawn_s, mut read_s, mut slowest) = (0.0, 0.0, 0.0f64);
    // At each threshold: the lines that enter, and the pages left with none.
    let mut all_lines = 0;
    let mut by_threshold = [(0usize, 0usize); THRESHOLDS.len()];
    let at = Instant::now();
    for (index, page) in &sample {
        match reader.page(&pdfs[*index], *page) {
            Ok((lines, drawn, read)) => {
                drawn_s += drawn;
                read_s += read;
                slowest = slowest.max(read);
                all_lines += lines.len();
                for (at_threshold, t) in by_threshold.iter_mut().zip(THRESHOLDS) {
                    let kept = lines.iter().filter(|line| line.score >= t).count();
                    at_threshold.0 += kept;
                    at_threshold.1 += usize::from(kept == 0 && !lines.is_empty());
                }
                match text_of(&lines, threshold).as_str() {
                    "" => blank += 1,
                    text if text == UNREADABLE => unreadable += 1,
                    _ => with_text += 1,
                }
            }
            Err(_) => failed += 1,
        }
    }
    let read_pages = sample.len().max(1) as f64;
    let report = serde_json::json!({
        "version": if cfg!(debug_assertions) { "depuracion (dependencias del motor optimizadas)" } else { "publicacion" },
        "arranque_motor_s": start_s,
        "hilos": std::thread::available_parallelism().map_or(1, |n| n.get() / 2).clamp(1, 4),
        "archivos_de_la_hoja_encontrados": files.len(),
        "impreso": printed_cer,
        "formulario_01": { "valores": form.pairs.len(), "lineas": form_lines.len(), "encontrados_por_umbral(umbral, exactos, casi)": form_found },
        "manuscrito": { "valores": total_values, "por_umbral": hand_found },
        "carpeta": {
            "pdf": pdfs.len(),
            "pdf_que_no_abren": unreadable_pdfs,
            "paginas_que_la_ficha_manda_a_leer": total,
            "paginas_leidas": sample.len(),
            "umbral": threshold,
            "con_texto": with_text,
            "ilegibles": unreadable,
            "en_blanco": blank,
            "fallaron": failed,
            "lineas": all_lines,
            "por_umbral(umbral, lineas_que_entran, paginas_ilegibles)": THRESHOLDS.iter().zip(by_threshold).map(|(t, (kept, none))| (format!("{t}"), kept, none)).collect::<Vec<_>>(),
            "s_por_pagina_dibujo": round2(drawn_s / read_pages),
            "s_por_pagina_lectura": round2(read_s / read_pages),
            "s_pagina_mas_lenta": round2(slowest),
            "minutos_total": round2(at.elapsed().as_secs_f64() / 60.0),
        },
        "memoria_maxima_mb": peak_memory_mb(),
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
}
