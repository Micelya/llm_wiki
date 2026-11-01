//! Reads real PDFs by zones with the reader of the app, to compare its
//! reading page by page with the readings of chain N (the plan of the cut,
//! section 4, step 5). Not part of the normal test run: it reads real
//! folders; keep the output outside the repository.
//!
//! ```text
//! MICELYA_OCR_DIR="C:\...\src-tauri\ocr" MICELYA_STRUCTURE_PDFS="C:\carpeta;C:\examen" \
//! MICELYA_STRUCTURE_N="C:\paso6\lectura\estructura;C:\paso6\examen\lectura\estructura" \
//! MICELYA_STRUCTURE_OUT="C:\salida" MICELYA_STRUCTURE_MAX_S=540 \
//!     cargo test --lib docbase::structure::check_reading -- --ignored --nocapture
//! ```
//!
//! - `MICELYA_STRUCTURE_PDFS`: folders with the PDFs, separated by `;`.
//! - `MICELYA_STRUCTURE_N`: folders of the readings of N
//!   (`<sha256>/pagina-<n>.json`): only their PDFs are read.
//! - `MICELYA_STRUCTURE_OUT`: where each reading goes, as N writes it
//!   (`<sha256>/pagina-<n>.json`); a PDF already read is skipped.
//! - `MICELYA_STRUCTURE_MAX_S`: stops after the PDF that passes these
//!   seconds, to go on in the next run.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::page::Bands;
use super::reader::read_pdf;
use crate::docbase::thresholds::Thresholds;

fn pdfs_in(folder: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for e in entries.flatten() {
        let path = e.path();
        if path.is_dir() {
            pdfs_in(&path, out);
        } else if path.extension().is_some_and(|x| x.eq_ignore_ascii_case("pdf")) {
            out.push(path);
        }
    }
}

#[test]
#[ignore = "reads the real PDFs named in MICELYA_STRUCTURE_PDFS"]
fn read_real_pdfs_by_zones() {
    let (Ok(folders), Ok(out)) = (std::env::var("MICELYA_STRUCTURE_PDFS"), std::env::var("MICELYA_STRUCTURE_OUT")) else {
        println!("MICELYA_STRUCTURE_PDFS or MICELYA_STRUCTURE_OUT is not set: nothing to read");
        return;
    };
    let max_s: f64 = std::env::var("MICELYA_STRUCTURE_MAX_S").ok().and_then(|v| v.parse().ok()).unwrap_or(540.0);
    let wanted: BTreeSet<String> = std::env::var("MICELYA_STRUCTURE_N")
        .unwrap_or_default()
        .split(';')
        .filter(|f| !f.trim().is_empty())
        .flat_map(|f| std::fs::read_dir(f).into_iter().flatten().flatten())
        .filter(|e| e.path().join("pagina-1.json").is_file())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    let mut pdfs = Vec::new();
    for f in folders.split(';').filter(|f| !f.trim().is_empty()) {
        pdfs_in(Path::new(f), &mut pdfs);
    }
    pdfs.sort();
    let defaults = Thresholds::default();
    let bands = Bands { high: defaults.high, low: defaults.low };
    let started = Instant::now();
    let (mut read, mut seen, mut failed) = (0, BTreeSet::new(), 0);
    for pdf in pdfs {
        let Ok(Some(hash)) = crate::source_volume::file_hash::file_sha256(&pdf) else {
            continue;
        };
        if (!wanted.is_empty() && !wanted.contains(&hash)) || !seen.insert(hash.clone()) {
            continue;
        }
        let dir = Path::new(&out).join(&hash);
        if dir.join("pdf.json").is_file() {
            continue;
        }
        if started.elapsed().as_secs_f64() > max_s {
            println!("stopped by time; read {read}; run again to go on");
            return;
        }
        let at = Instant::now();
        std::fs::create_dir_all(&dir).unwrap();
        match read_pdf(&pdf, bands) {
            Ok(pages) => {
                for p in &pages {
                    std::fs::write(dir.join(format!("pagina-{}.json", p.page)), serde_json::to_string(p).unwrap()).unwrap();
                }
                let summary = serde_json::json!({"sha256": hash, "paginas": pages.len(), "segundos": at.elapsed().as_secs_f64()});
                std::fs::write(dir.join("pdf.json"), summary.to_string()).unwrap();
                read += 1;
                println!("{} {} pages in {:.1} s", &hash[..8], pages.len(), at.elapsed().as_secs_f64());
            }
            Err(error) => {
                failed += 1;
                let summary = serde_json::json!({"sha256": hash, "error": error});
                std::fs::write(dir.join("pdf.json"), summary.to_string()).unwrap();
                println!("{} failed: {error}", &hash[..8]);
            }
        }
    }
    println!("done; read {read}, failed {failed}");
}
