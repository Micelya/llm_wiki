//! The reading by zones with invented pages (the repository is public).

use super::page::{band, classify, flags, structure, Bands, Line, ModelZone};
use super::reader::{char_lines, layer, page_out, placed_chars, profiles, Profile};
use super::{basis, zones_left};
use crate::docbase::cut::by_zones::{page::question_text, PageReading, ZONES_READING, ZONES_STAGE};
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;
use crate::source_volume::page_shape::test_pdf::{build, page as pdf_page};

const BANDS: Bands = Bands { high: 0.97, low: 0.90 };

fn line(b: [i64; 4], text: &str, score: Option<f64>) -> Line {
    Line { bbox: b, text: text.into(), score }
}

fn zone(b: [i64; 4], class: &str, score: f64) -> ModelZone {
    ModelZone { bbox: b, class: class.into(), score }
}

#[test]
fn each_line_has_its_band_and_why() {
    let c = |text: &str, score: Option<f64>, class: &str, old: Option<&str>| {
        let (possible, why, _) = classify(&line([0, 0, 10, 10], text, score), class, BANDS, old);
        (band(possible, score, BANDS.low), why)
    };
    // pdfium's text is firm.
    assert_eq!(c("Texto de la capa", None, "text", None), ("firme", "digital".into()));
    // From the high threshold up, firm when it looks like words.
    assert_eq!(c("El contrato de locación", Some(0.98), "text", None), ("firme", "confianza".into()));
    // Initials and acronyms are words (V1): not sunk.
    assert_eq!(c("Empresa S.A. CUIT", Some(0.99), "text", None).0, "firme");
    assert_eq!(c("xqzt wvbk prst", Some(0.99), "text", None), ("aproximada", "no parece palabras".into()));
    // In a seal or an image, at most approximate; below the low threshold, discarded.
    assert_eq!(c("Escribano Público", Some(0.99), "seal", None), ("aproximada", "zona de seal".into()));
    assert_eq!(c("Escribano Público", Some(0.5), "image", None), ("descartada", "zona de image".into()));
    // Below the high threshold: the old layer that says the same rescues it...
    assert_eq!(c("Calle Inventada 1234", Some(0.6), "text", Some("xxcalleinventada1234pisoxx")), ("firme", "capa_vieja".into()));
    // ...and so does the lexicon, from the low threshold up.
    assert_eq!(c("Señores de la reunión", Some(0.93), "text", None), ("firme", "lexico".into()));
    assert_eq!(c("Señores de la reunión", Some(0.85), "text", None), ("descartada", "confianza baja".into()));
    assert_eq!(c("Señores de la reunión 7", Some(0.93), "text", None), ("aproximada", "confianza baja".into()));
}

#[test]
fn every_line_goes_to_the_smallest_zone_that_holds_it() {
    let zones = [
        zone([0, 0, 1000, 1000], "text", 0.9),
        zone([100, 100, 500, 200], "doc_title", 0.8),
        // Below the least score of a zone: not a zone.
        zone([0, 1500, 1000, 1600], "footer", 0.4),
    ];
    let lines = [
        line([120, 120, 480, 160], "TÍTULO INVENTADO", Some(0.99)),
        line([50, 300, 900, 340], "Primer renglón del cuerpo inventado", Some(0.98)),
        line([50, 400, 900, 440], "renglón dudoso", Some(0.5)),
        // Outside every zone: a zone of its own for each block.
        line([50, 1510, 400, 1540], "Pie de página", None),
        line([50, 1550, 400, 1580], "segundo renglón del pie", None),
        line([50, 1800, 400, 1830], "lejos del pie", None),
    ];
    let s = structure(&lines, &zones, BANDS, None, ("modelo", "lector"));
    let got: Vec<(&str, &str, usize)> = s.zones.iter().map(|z| (z.id.as_str(), z.clase.as_str(), z.renglones.len())).collect();
    assert_eq!(got, vec![("z1", "text", 2), ("z2", "doc_title", 1), ("z3", "sin_zona", 2), ("z4", "sin_zona", 1)]);
    assert_eq!(s.zones[0].texto, "Primer renglón del cuerpo inventado");
    assert!(!s.zones[0].segura && s.zones[1].segura);
    assert_eq!(s.zones[2].recuadro, [50, 1510, 400, 1580]);
    assert_eq!((s.zones[2].motor_zonas.as_deref(), s.zones[2].motor_texto.as_deref()), (None, Some("pdfium")));
    assert_eq!(s.zones[0].motor_texto.as_deref(), Some("lector"));
    assert_eq!(s.evidence.len(), 1);
    assert_eq!((s.evidence[0].zona.as_str(), s.evidence[0].franja), ("z1", "descartada"));
    assert_eq!(s.firm.lines().next(), Some("Primer renglón del cuerpo inventado"));
    assert!(s.approximate.is_empty());
    // The words of a broken logo stay as the reader gave them (stage 3b).
    let logo = [line([10, 10, 60, 30], "Inven", Some(0.99)), line([10, 32, 60, 52], "tada", Some(0.99))];
    let s = structure(&logo, &[zone([0, 0, 100, 100], "header_image", 0.9)], BANDS, None, ("m", "l"));
    assert_eq!(s.zones[0].renglones.len(), 2);
}

#[test]
fn the_marks_of_a_page_its_signature_and_a_blank_page() {
    let read = [line([0, 0, 10, 10], "Firmado digitalmente por un sistema inventado", Some(0.99))];
    assert_eq!(flags(&read, &[], 0.9, true, 0).1, "system");
    let hand = [line([0, 0, 10, 10], "Firma y aclaración del firmante inventado", Some(0.99))];
    assert_eq!(flags(&hand, &[], 0.9, false, 500).1, "system", "«firmante» is the mark of a system");
    let hand = [line([0, 0, 10, 10], "Firma y aclaración", Some(0.99)), line([0, 20, 10, 30], "texto de relleno largo de la hoja", Some(0.99))];
    let (_, signature, blank) = flags(&hand, &[zone([0, 0, 50, 50], "seal", 0.8)], 0.9, true, 0);
    assert_eq!((signature, blank), ("handwritten", false));
    // Not read: only pdfium's letters of its own say it is blank.
    let (f, signature, blank) = flags(&hand, &[zone([0, 0, 50, 50], "seal", 0.8)], 0.9, false, 10);
    assert_eq!((signature, blank, f.seal), ("none", true, true));
    let (_, _, blank) = flags(&[line([0, 0, 1, 1], "3", Some(0.99))], &[], 0.9, true, 0);
    assert!(blank);
    let low = [line([0, 0, 1, 1], "renglón leído mal", Some(0.4)), line([0, 2, 1, 3], "otro renglón leído", Some(0.5))];
    assert!(flags(&low, &[], 0.9, true, 0).0.lectura_baja_confianza);
}

#[test]
fn which_pages_are_read() {
    let words = "Texto propio de la página, con palabras que parecen palabras y bastantes letras en total. Segunda frase inventada.";
    let header = "ENCABEZADO INVENTADO QUE SE REPITE";
    let page = |own: &str, cover: f64, paths: usize| (format!("{header}\r\n{own}"), cover, 1, paths);
    let p = profiles(&[page(words, 0.0, 0), page("", 0.9, 0), page(words, 0.9, 0), page("", 0.0, 250), page("", 0.0, 0)]);
    // The line on most pages is not the page's own.
    assert_eq!(p[1].own_chars, 0);
    assert!(p[1].chars > 0);
    let layers: Vec<(&str, bool)> = p.iter().map(layer).collect();
    assert_eq!(
        layers,
        vec![("digital", false), ("footer_only", true), ("old_ocr", true), ("footer_only", true), ("footer_only", false)]
    );
    let noisy = Profile { chars: 900, own_chars: 900, dudosas: 0.5, palabras: 40, ..Default::default() };
    assert_eq!(layer(&noisy), ("old_ocr", true));
    assert_eq!(layer(&Profile::default()), ("none", false));
}

#[test]
fn the_letters_of_the_layer_make_lines() {
    let ch = |c: &str, l: f64, b: f64, r: f64, t: f64| (c.to_string(), l, b, r, t);
    let raw = vec![
        ch("H", 10.0, 700.0, 16.0, 712.0),
        ch("o", 16.0, 700.0, 22.0, 712.0),
        ch(" ", 0.0, 0.0, 0.0, 0.0),
        ch("y", 26.0, 700.0, 32.0, 712.0),
        ch("\r", 0.0, 0.0, 0.0, 0.0),
        ch("\n", 0.0, 0.0, 0.0, 0.0),
        ch("A", 10.0, 680.0, 16.0, 692.0),
        // Far to the right: two spaces that become one.
        ch("B", 60.0, 680.0, 66.0, 692.0),
    ];
    let lines = char_lines(&placed_chars(&raw, 800.0, 1.0));
    let got: Vec<(&str, [i64; 4])> = lines.iter().map(|l| (l.text.as_str(), l.bbox)).collect();
    assert_eq!(got, vec![("Ho y", [10, 88, 32, 100]), ("A B", [10, 108, 66, 120])]);
    assert!(lines.iter().all(|l| l.score.is_none()));
}

#[test]
fn the_reading_of_a_page_is_what_the_cut_reads() {
    let p = Profile { chars: 2000, own_chars: 1900, ..Default::default() };
    let lines = [line([100, 100, 900, 140], "CONTRATO INVENTADO", None), line([100, 200, 900, 240], "Página 1 de 2", None)];
    let out = page_out(1, (1414, 2000), &p, &lines, &[zone([90, 90, 910, 150], "doc_title", 0.9)], BANDS, None);
    assert_eq!((out.forma.forma, out.forma.capa.as_str(), out.forma.se_lee), ("digital", "digital", false));
    assert_eq!(out.receta, "n-zonas-rust:pp_doc_layoutv2+rapidocr-onnx:ppocrv6-small|alto 0.97|bajo 0.9");
    let body = serde_json::to_string(&[&out]).unwrap();
    let back: Vec<PageReading> = serde_json::from_str(&body).unwrap();
    assert_eq!(back[0].zonas.len(), 2);
    assert_eq!(back[0].zonas[0].renglones[0].seguro, Some(true));
    assert_eq!(back[0].tamano_px.as_deref(), Some(&[1414.0, 2000.0][..]));
    let (text, forma) = question_text(back.first(), 3500);
    assert_eq!(forma, "digital");
    assert!(text.starts_with("[doc_title] CONTRATO INVENTADO\n[sin_zona] Página 1 de 2"), "{text}");
    assert_eq!(serde_json::from_str::<serde_json::Value>(&body).unwrap()[0]["franjas"]["firme"], 2);
}

#[test]
fn without_the_model_of_zones_the_pdfs_wait_with_their_failure() {
    let dir = TempDir::new();
    let text = |w: &str| pdf_page(&[&format!("Texto inventado de la lectura por zonas, {w}, con letras"), "para que se lea como digital."], 0.0);
    dir.write("o/a.pdf", &build(&[text("uno"), text("dos")]));
    dir.write("o/b.pdf", &build(&[text("sola")]));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    while base.profile_pending(10).unwrap() > 0 {}
    let free = "1 = 1";
    // In the tests no reading by zones is coming unless the setting says so.
    assert_eq!(zones_left(&base.conn(), free, BANDS).unwrap(), 0);
    base.conn().execute("INSERT INTO docbase_setting (key, value) VALUES (?1, 'on')", [ZONES_READING]).unwrap();
    // Only the PDF of two pages or more waits for it; the cut waits too.
    assert_eq!(zones_left(&base.conn(), free, BANDS).unwrap(), 1);
    assert_eq!(base.cut_pending(10).unwrap(), 0);
    let process = base.process_state(&crate::docbase::process::Scope::All, false).unwrap();
    assert!(process.pending(crate::docbase::process::Step::Zones) >= 1);
    let waiting = base.zones_waiting(10).unwrap();
    assert_eq!(waiting.len(), 1);
    assert!(waiting[0].1.ends_with("a.pdf"));
    // Without the model: written once, and the file waits; not cut with
    // the cut of before (D-717).
    assert_eq!(base.zones_without_models("The model of zones of this computer is not installed").unwrap(), 1);
    assert_eq!(base.zones_without_models("again").unwrap(), 0);
    assert_eq!(zones_left(&base.conn(), free, BANDS).unwrap(), 0);
    assert!(base.zones_waiting(10).unwrap().is_empty());
    assert_eq!(base.cut_pending(10).unwrap(), 0);
    let (stage, cause, basis_kept): (String, String, String) =
        base.conn().query_row("SELECT stage, cause, basis FROM failure", [], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
    assert_eq!((stage.as_str(), cause.as_str()), (ZONES_STAGE, "models-missing"));
    assert_eq!(basis_kept, basis(BANDS));
    // The model is there: the file is read again (here it fails: the
    // tests have no model, and the failure is written with its class).
    base.zones_with_models().unwrap();
    assert_eq!(base.zones_waiting(10).unwrap().len(), 1);
    // The scans' models do not resolve the failures of the reading by zones.
    base.zones_without_models("not installed").unwrap();
    base.scans_with_models().unwrap();
    assert!(base.zones_waiting(10).unwrap().is_empty());
    // A reading kept: the file is cut by it.
    let hash = &waiting[0].0;
    let p = Profile { chars: 200, own_chars: 200, ..Default::default() };
    let page = |n: u32, t: &str| page_out(n, (1414, 2000), &p, &[line([100, 100, 900, 140], t, None)], &[], BANDS, None);
    let body = serde_json::to_string(&[page(1, "Texto inventado uno"), page(2, "Texto inventado dos")]).unwrap();
    base.put_zones_body(hash, &body, &basis(BANDS)).unwrap();
    assert!(base.zones_of(hash).unwrap().is_some());
    assert_eq!(base.cut_pending(10).unwrap(), 1);
    assert!(base.cut_steps(hash).unwrap()[0].zones);
}
