//! The text with its bands for every step (E1 of 2 nov): a scan read by the
//! reader of this computer has as its text its reading by zones, the firm
//! lines as text, the approximate ones marked and the discarded ones out;
//! the PDFs of one page and the images are read by zones too; the step of
//! the zones does not read again a page the step of the scans read; the
//! thresholds of Ajustes make the text; and a base with texts of the simple
//! scan reads them again. A stand-in reader of zones and invented pages
//! (R-24: nothing here comes from the test folder).

use std::cell::Cell;
use std::path::Path;

use crate::docbase::cut::by_zones::ZONES_READING;
use crate::docbase::process::{Scope, Step};
use crate::docbase::recognize::{simple_scans_read_again_once, zones_text_engine, RecognitionRequest};
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;
use crate::source_volume::layout_local::Zone;
use crate::source_volume::page_shape::test_pdf::{build, page};
use crate::source_volume::recognition::{marker, TextRecognizer};
use crate::source_volume::recognition_local::{catalog, PageByZones, ReadLine, ENGINE};

use super::page::Bands;

/// A reader of zones that sees the same invented page in every picture,
/// and counts the pages it reads.
struct Stand {
    read: Cell<usize>,
}

impl Stand {
    fn new() -> Self {
        Stand { read: Cell::new(0) }
    }
}

fn line(top: i64, text: &str, score: f32) -> ReadLine {
    ReadLine { text: text.into(), score, bbox: [40, top, 900, top + 30] }
}

impl PageByZones for Stand {
    fn zones(&self, _: &image::RgbImage) -> Result<Vec<Zone>, String> {
        Ok(vec![
            Zone { bbox: [20, 0, 960, 700], class: "text".into(), score: 0.9 },
            Zone { bbox: [20, 800, 960, 1000], class: "seal".into(), score: 0.9 },
        ])
    }

    fn lines(&self, _: &image::RgbImage) -> Result<Vec<ReadLine>, String> {
        self.read.set(self.read.get() + 1);
        Ok(vec![
            line(10, "CONTRATO DE LOCACION DE PRUEBA", 0.99),
            line(60, "Locatario Persona Tres", 0.99),
            // Below the high threshold, words the lexicon does not know.
            line(110, "Hijo de Xandro Quimbel", 0.93),
            line(160, "garabato zzq", 0.40),
            // Sure, but in the seal: at most approximate.
            line(850, "Matricula Tomo 99 Folio 999", 0.99),
        ])
    }
}

impl TextRecognizer for Stand {
    fn engine_id(&self) -> String {
        format!("{ENGINE}:{}", catalog().model_set)
    }

    fn recognize(&self, _: &Path) -> Result<String, String> {
        Err("it reads by zones".into())
    }

    fn by_zones(&self) -> Option<&dyn PageByZones> {
        Some(self)
    }
}

fn base_of(files: &[(&str, Vec<u8>)]) -> (TempDir, DocBase) {
    let dir = TempDir::new();
    for (name, bytes) in files {
        dir.write(&format!("o/{name}"), bytes);
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    (dir, base)
}

fn photo() -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/source_volume/fixtures/pagina-prueba.png")).unwrap()
}

/// A PDF of one page, a scan (a picture covers it, no text of its own).
fn scan_of_one_page(cover: f64) -> Vec<u8> {
    build(&[page(&[], cover)])
}

fn of(base: &DocBase, name: &str) -> (i64, String) {
    base.conn()
        .query_row("SELECT f.version_id, f.hash FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap()
}

fn text(base: &DocBase, version: i64) -> (String, String) {
    base.conn()
        .query_row("SELECT body, engine FROM derivative WHERE version_id = ?1 AND kind = 'text'", [version], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
}

const DEFAULT: Bands = Bands { high: 0.97, low: 0.90 };
const MARKED: &str = "CONTRATO DE LOCACION DE PRUEBA\nLocatario Persona Tres\n[aproximado] Hijo de Xandro Quimbel\n[aproximado] Matricula Tomo 99 Folio 999";

#[test]
fn a_scanned_pdf_of_one_page_and_an_image_are_read_by_zones_with_their_bands_and_never_twice() {
    let (_dir, base) = base_of(&[("una.pdf", scan_of_one_page(1.0)), ("foto.png", photo())]);
    let stand = Stand::new();
    let local = zones_text_engine(&stand.engine_id(), DEFAULT);
    for name in ["una.pdf", "foto.png"] {
        let (version, _) = of(&base, name);
        assert!(base.recognize_content(version, &stand).unwrap(), "{name}");
        let (body, engine) = text(&base, version);
        // The firm lines as text, the approximate ones marked (the seal
        // too), the discarded one out.
        assert!(body.contains(MARKED), "{name}: {body}");
        assert!(!body.contains("garabato"), "{name}: {body}");
        assert!(body.contains(&marker(&local)), "{name}: {body}");
        assert_eq!(engine, local);
        // Every word of its text counts as its words, firm (8) and
        // approximate (9): the one rule of the steps that ask (E3 of 2 nov).
        let words: i64 = base.conn().query_row("SELECT word_count FROM version WHERE id = ?1", [version], |row| row.get(0)).unwrap();
        assert_eq!(words, 17, "{name}");
    }
    assert_eq!(stand.read.get(), 2, "one page read for each document");

    // The step of the zones reads both (before E1, a PDF of one page and an
    // image had no reading by zones), and reads no page again.
    base.conn().execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, 'on')", [ZONES_READING]).unwrap();
    while base.profile_pending(10).unwrap() > 0 {}
    let mut steps = 0;
    while base.read_zones_step_with(&stand).unwrap() {
        steps += 1;
        assert!(steps <= 2);
    }
    assert_eq!(steps, 2);
    assert_eq!(stand.read.get(), 2, "the step of the zones took the pages the step of the scans read");
    for name in ["una.pdf", "foto.png"] {
        let (_, hash) = of(&base, name);
        let body: serde_json::Value = serde_json::from_str(&base.zones_of(&hash).unwrap().unwrap()).unwrap();
        assert_eq!(body.as_array().unwrap().len(), 1, "{name}");
        let bands = &body[0]["franjas"];
        assert_eq!((bands["firme"].as_u64(), bands["aproximada"].as_u64(), bands["descartada"].as_u64()), (Some(2), Some(2), Some(1)), "{name}");
        assert_eq!(super::reader::marked_text(&body[0]), MARKED);
    }

    // The share of safe words (E3 of 2 nov), kept with the signature: the
    // firm words over every word read, the discarded ones of the reading by
    // zones too, for the PDF of one page and for the image.
    while base.similar_step(10).unwrap() > 0 {}
    for name in ["una.pdf", "foto.png"] {
        let (version, _) = of(&base, name);
        let words = crate::docbase::safe_words::of(&base.conn(), version).unwrap().unwrap();
        assert_eq!(words, crate::docbase::safe_words::SafeWords { firm: 8, approximate: 9, discarded: 2 }, "{name}");
    }
}

#[test]
fn the_thresholds_of_ajustes_make_the_text_and_a_person_may_read_it_again_with_new_ones() {
    let (_dir, base) = base_of(&[("uno.pdf", scan_of_one_page(1.0)), ("dos.png", photo())]);
    let stand = Stand::new();
    let (uno, _) = of(&base, "uno.pdf");
    let (dos, _) = of(&base, "dos.png");
    assert!(base.recognize_content(uno, &stand).unwrap());
    assert!(text(&base, uno).0.contains("[aproximado] Hijo de Xandro Quimbel"));
    // The low threshold goes up in Ajustes: the line read at 0,93 is
    // discarded now; the seal stays approximate.
    let mut thresholds = base.thresholds().unwrap();
    thresholds.low = 0.95;
    base.set_thresholds(&thresholds).unwrap();
    assert!(base.recognize_content(dos, &stand).unwrap());
    let (body, engine) = text(&base, dos);
    assert!(!body.contains("Xandro"), "{body}");
    assert!(body.contains("CONTRATO DE LOCACION DE PRUEBA\nLocatario Persona Tres\n[aproximado] Matricula Tomo 99 Folio 999"), "{body}");
    assert_eq!(engine, zones_text_engine(&stand.engine_id(), Bands { high: 0.97, low: 0.95 }));
    // What was read is not read again by itself (D-747); a person may ask.
    assert!(text(&base, uno).0.contains("Xandro"));
    assert!(!base.recognize_content(dos, &stand).unwrap(), "read with these thresholds already");
    assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![uno])).unwrap(), 1);
    assert!(base.recognize_content(uno, &stand).unwrap());
    assert!(!text(&base, uno).0.contains("Xandro"));
    assert_eq!(stand.read.get(), 3, "the page read with other thresholds is read again, once");
}

#[test]
fn a_base_with_texts_of_the_simple_scan_reads_them_again_by_zones_once() {
    let (_dir, base) = base_of(&[("vieja.png", photo())]);
    let (version, hash) = of(&base, "vieja.png");
    // As the simple scan left it: every line of 0,90 or more, unmarked.
    let simple = format!("{ENGINE}:{}", catalog().model_set);
    let old = format!("{}\nCONTRATO DE LOCACION DE PRUEBA\nHijo de Xandro Quimbel\nMatricula Tomo 99 Folio 999\n", marker(&simple));
    assert!(base.store_recognized_text(&hash, &old, &simple).unwrap());
    // The base was opened before that text: it is opened again here.
    base.conn().execute("DELETE FROM docbase_setting WHERE key = 'recognition:simple-scans-read-again'", []).unwrap();
    assert_eq!(simple_scans_read_again_once(&base.conn()).unwrap(), 1);
    assert_eq!(simple_scans_read_again_once(&base.conn()).unwrap(), 0, "once per base");
    // It waits in the step of the scans, with the reader of this computer
    // (no question to the cloud), whatever Ajustes says.
    assert_eq!(crate::docbase::recognize::requested_recognitions(&base.conn()).unwrap(), vec![version]);
    assert_eq!(base.reader_of_request(version).unwrap().map(|r| r.engine), Some(ENGINE.to_string()));
    assert!(base.process_state(&Scope::All, false).unwrap().pending(Step::Scans) >= 1);
    let stand = Stand::new();
    assert!(base.recognize_content(version, &stand).unwrap());
    let (body, engine) = text(&base, version);
    assert!(body.contains(MARKED), "{body}");
    assert_eq!(engine, zones_text_engine(&simple, DEFAULT));
}
