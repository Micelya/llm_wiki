//! Reads real scans with the reader of this computer, in a new base, and
//! checks their text (E1 of 2 nov): the firm lines as text, the approximate
//! ones marked, nothing of the discarded ones; and that the step of the
//! zones reads no page the step of the scans read. Not part of the normal
//! run: it reads real files (only reads them: a copy goes to a temporary
//! folder); keep the output outside the repository.
//!
//! ```text
//! MICELYA_OCR_DIR="C:\...\src-tauri\ocr" MICELYA_TEXT_FILES="C:\a.pdf;C:\b.pdf" MICELYA_TEXT_OUT="C:\salida" \
//!     cargo test --lib docbase::structure::check_text -- --ignored --nocapture
//! ```

use std::cell::Cell;
use std::path::Path;

use crate::docbase::cut::by_zones::ZONES_READING;
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;
use crate::source_volume::layout_local::Zone;
use crate::source_volume::recognition::TextRecognizer;
use crate::source_volume::recognition_local::{LocalRecognizer, PageByZones, ReadLine};

/// The reader of this computer, counting the pages it reads.
struct Counting {
    inner: LocalRecognizer,
    read: Cell<usize>,
}

impl PageByZones for Counting {
    fn zones(&self, picture: &image::RgbImage) -> Result<Vec<Zone>, String> {
        self.inner.zones(picture)
    }

    fn lines(&self, picture: &image::RgbImage) -> Result<Vec<ReadLine>, String> {
        self.read.set(self.read.get() + 1);
        self.inner.lines(picture)
    }
}

impl TextRecognizer for Counting {
    fn engine_id(&self) -> String {
        self.inner.engine_id()
    }

    fn recognize(&self, image: &Path) -> Result<String, String> {
        self.inner.recognize(image)
    }

    fn by_zones(&self) -> Option<&dyn PageByZones> {
        Some(self)
    }
}

#[test]
#[ignore = "reads the real files named in MICELYA_TEXT_FILES"]
fn read_real_scans_with_their_bands() {
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
    let reader = Counting { inner: LocalRecognizer::new(), read: Cell::new(0) };
    let of = |name: &str| -> (i64, String) {
        base.conn()
            .query_row("SELECT f.version_id, f.hash FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .unwrap()
    };
    for name in &names {
        let at = std::time::Instant::now();
        let read = base.recognize_content(of(name).0, &reader).unwrap();
        println!("{name}: read {read} in {:.1} s", at.elapsed().as_secs_f64());
    }
    let by_the_scans = reader.read.get();
    base.conn().execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, 'on')", [ZONES_READING]).unwrap();
    while base.profile_pending(10).unwrap() > 0 {}
    while base.read_zones_step_with(&reader).unwrap() {}
    let by_the_zones = reader.read.get() - by_the_scans;
    let mut summary = Vec::new();
    for name in &names {
        let (version, hash) = of(name);
        let (body, engine): (String, String) = base
            .conn()
            .query_row("SELECT body, engine FROM derivative WHERE version_id = ?1 AND kind = 'text'", [version], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        let zones: serde_json::Value = serde_json::from_str(&base.zones_of(&hash).unwrap().expect("its reading by zones")).unwrap();
        let (mut firm, mut approximate, mut discarded, mut wrong) = (0, 0, 0, Vec::new());
        let is_pdf = name.ends_with(".pdf");
        for page in zones.as_array().unwrap().iter().filter(|p| p["forma"]["se_lee"].as_bool() == Some(true)) {
            // Each page against its own section of the text.
            let n = page["page"].as_u64().unwrap() as u32;
            let section = if is_pdf { crate::docbase::cut::parts::pages_text(&body, n, n) } else { body.clone() };
            let lines: Vec<&str> = section.lines().map(str::trim).collect();
            for row in page["zonas"].as_array().unwrap().iter().flat_map(|z| z["renglones"].as_array().unwrap()) {
                let text = row["texto"].as_str().unwrap().trim();
                let marked = crate::source_volume::bands::approximate_line(text);
                match row["franja"].as_str().unwrap() {
                    "firme" => {
                        firm += 1;
                        if !lines.contains(&text) {
                            wrong.push(format!("firm line missing: {text}"));
                        }
                    }
                    "aproximada" => {
                        approximate += 1;
                        if !lines.contains(&marked.as_str()) {
                            wrong.push(format!("approximate line not marked: {text}"));
                        }
                    }
                    _ => {
                        discarded += 1;
                        // A discarded line may say what a firm one says
                        // elsewhere; it must not be there as itself only.
                        let firm_same = page["zonas"].as_array().unwrap().iter().flat_map(|z| z["renglones"].as_array().unwrap()).any(|r| {
                            r["franja"].as_str() != Some("descartada") && r["texto"].as_str().map(str::trim) == Some(text)
                        });
                        if !firm_same && (lines.contains(&text) || lines.contains(&marked.as_str())) {
                            wrong.push(format!("discarded line in the text: {text}"));
                        }
                    }
                }
            }
        }
        std::fs::write(out.join(format!("{name}.txt")), &body).unwrap();
        let line = serde_json::json!({
            "documento": name, "motor": engine, "paginas": zones.as_array().unwrap().len(),
            "firmes": firm, "aproximadas": approximate, "descartadas": discarded, "errores": wrong,
        });
        println!("{line}");
        summary.push(line);
    }
    let all = serde_json::json!({ "leidas_por_el_escaneo": by_the_scans, "leidas_de_nuevo_por_las_zonas": by_the_zones, "documentos": summary });
    std::fs::write(out.join("resumen.json"), serde_json::to_string_pretty(&all).unwrap()).unwrap();
    println!("pages read by the scans: {by_the_scans}; read again by the zones: {by_the_zones}");
    for document in &summary {
        assert!(document["errores"].as_array().unwrap().is_empty(), "{document}");
        assert!(document["firmes"].as_u64().unwrap() > 0, "{document}");
    }
    assert_eq!(by_the_zones, 0, "the step of the zones read again a page the step of the scans read");
}
