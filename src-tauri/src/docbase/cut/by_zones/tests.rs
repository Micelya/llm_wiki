//! The cut by zones over invented pages (R-24).

use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::json;

use super::page::{Line, PageReading, Shape, Zone};
use super::rules::{CUTS, DOUBTFUL, JOINS};
use super::scan::PageScan;
use super::window::{self, merge, PageAnswer};
use super::{cut, steps};
use crate::docbase::cut::{CutStep, Verdict};
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::page_shape::test_pdf::{build, page as pdf_page};

const H: f64 = 1400.0;

fn line(text: &str, y: f64, safe: bool) -> Line {
    Line { texto: text.into(), recuadro: Some(vec![100.0, y, 900.0, y + 20.0]), seguro: Some(safe) }
}

fn zone(id: &str, class: &str, lines: Vec<Line>) -> Zone {
    let top = lines.first().map(|l| l.recuadro.as_ref().unwrap()[1]).unwrap_or(0.0);
    let bottom = lines.last().map(|l| l.recuadro.as_ref().unwrap()[3]).unwrap_or(0.0);
    let texto = lines.iter().filter(|l| l.seguro == Some(true)).map(|l| l.texto.clone()).collect::<Vec<_>>().join("\n");
    Zone { id: id.into(), clase: class.into(), recuadro: Some(vec![100.0, top, 900.0, bottom]), texto, renglones: lines }
}

fn reading(page: u32, forma: &str, zonas: Vec<Zone>) -> PageReading {
    let texto_seguro = zonas
        .iter()
        .flat_map(|z| z.renglones.iter())
        .filter(|l| l.seguro == Some(true))
        .map(|l| l.texto.clone())
        .collect::<Vec<_>>()
        .join("\n");
    PageReading {
        page,
        tamano_px: Some(vec![1000.0, H]),
        forma: Shape { forma: forma.into(), capa: "digital".into(), firma: "none".into() },
        zonas,
        evidencia: Vec::new(),
        texto_seguro,
    }
}

/// A digital page of prose with these firm lines.
fn prose(page: u32, lines: &[&str]) -> PageReading {
    let ls = lines.iter().enumerate().map(|(i, t)| line(t, 200.0 + 40.0 * i as f64, true)).collect();
    reading(page, "digital", vec![zone("z1", "text", ls)])
}

fn with_footer(mut r: PageReading, footer: &str, y: f64) -> PageReading {
    r.zonas.push(zone("z9", "footer", vec![line(footer, y, true)]));
    r.texto_seguro = format!("{}\n{footer}", r.texto_seguro);
    r
}

fn scans(n: usize) -> Vec<PageScan> {
    (0..n).map(|_| PageScan { w: 595, h: 842, shape: "digital_prose".into(), ..PageScan::default() }).collect()
}

fn decide(pages: Vec<PageReading>) -> (&'static str, String, Vec<String>) {
    let c = cut(&pages, &scans(pages.len()));
    let d = &c.decisions[0];
    (d.decision, d.rule.clone(), d.hints.iter().map(|h| h.regla.clone()).collect())
}

#[test]
fn the_four_safe_rules_only_say_it_goes_on() {
    // The sentence goes on in the next page.
    let (d, rule, _) = decide(vec![
        prose(1, &["Primer renglón inventado de la prueba.", "y por lo tanto el contrato inventado que firman las partes"]),
        prose(2, &["obliga a las dos partes inventadas a cumplirlo.", "Otro renglón."]),
    ]);
    assert_eq!((d, rule.as_str()), (JOINS, "frase_sigue"));
    // The page ends on a function word.
    let (d, rule, _) = decide(vec![
        prose(1, &["Renglón inventado.", "Las partes inventadas acuerdan el pago de"]),
        prose(2, &["Mil pesos inventados por mes.", "Otro renglón."]),
    ]);
    assert_eq!((d, rule.as_str()), (JOINS, "termina_en_nexo"));
    // «Página x de y» goes on.
    let (d, rule, _) = decide(vec![
        with_footer(prose(1, &["Primer punto inventado.", "Fin del primer punto."]), "Página 1 de 2", 1300.0),
        with_footer(prose(2, &["Segundo punto inventado.", "Fin."]), "Página 2 de 2", 1300.0),
    ]);
    assert_eq!((d, rule.as_str()), (JOINS, "pagina_de_sigue"));
    // A blank page goes with the one before.
    let (d, rule, _) =
        decide(vec![prose(1, &["Un escrito inventado.", "Termina acá."]), reading(2, "blank_or_footer", Vec::new())]);
    assert_eq!((d, rule.as_str()), (JOINS, "pagina_blanca"));
    // A page number that goes on is a hint, not a decision.
    let (d, rule, hints) = decide(vec![
        with_footer(prose(1, &["Un escrito inventado.", "Termina acá."]), "1", 50.0),
        with_footer(prose(2, &["Otro escrito inventado.", "Termina allá."]), "2", 50.0),
    ]);
    assert_eq!((d, rule.as_str()), (DOUBTFUL, "ninguna"));
    assert_eq!(hints, vec!["numero_sigue".to_string()]);
    // A doubtful last line does not found «la frase sigue»: it is a hint.
    let mut first = prose(1, &["Renglón inventado.", "y por lo tanto el contrato inventado que firman las partes"]);
    first.zonas[0].renglones[1].seguro = Some(false);
    let (d, _, hints) = decide(vec![first, prose(2, &["obliga a las dos partes a cumplirlo.", "Otro."])]);
    assert_eq!(d, DOUBTFUL);
    assert_eq!(hints, vec!["frase_sigue_dudoso".to_string()]);
    // No rule of the cut by zones ever cuts.
    assert!(!super::super::catalog::catalog().zones.finals.iter().any(|f| f.ends_with(CUTS)));
}

/// A file of `n` pages, every step doubtful.
fn doubtful_file(n: u32) -> Vec<PageReading> {
    (1..=n).map(|p| prose(p, &[&format!("Escrito inventado número {p}."), "Termina acá."])).collect()
}

#[test]
fn windows_of_ten_pages_share_two() {
    assert_eq!(window::spans(1, 22), vec![(1, 10), (9, 18), (17, 22)]);
    assert_eq!(window::spans(1, 10), vec![(1, 10)]);
    assert_eq!(window::spans(4, 5), vec![(4, 5)]);
    let c = cut(&doubtful_file(12), &scans(12));
    assert_eq!(c.parts.len(), 1);
    let w: Vec<(u32, u32, u32)> = c.windows.iter().map(|w| (w.first, w.last, w.number)).collect();
    assert_eq!(w, vec![(1, 10, 1), (9, 12, 2)]);
    assert!(c.windows[0].prompt.contains("La página 1 es el principio de una parte: ahí empieza un documento. La parte sigue después de la página 10."));
    assert!(c.windows[1].prompt.contains("Este es el tramo 2 de 2 de una parte más larga. La página 9 sigue algo que no ves"));
    // A file whose steps the rules decided is not asked.
    let c = cut(
        &[
            prose(1, &["Renglón inventado.", "Las partes inventadas acuerdan el pago de"]),
            prose(2, &["Mil pesos inventados por mes.", "Otro renglón."]),
        ],
        &scans(2),
    );
    assert!(c.windows.is_empty());
}

#[test]
fn the_question_and_its_key_come_from_its_text_only() {
    let pages = doubtful_file(3);
    let a = cut(&pages, &scans(3));
    let w = &a.windows[0];
    assert_eq!(w.key, window::key(&w.prompt));
    for placeholder in ["{k}", "{desde}", "{hasta}", "{total}", "{borde}", "{criterios}", "{tipos}", "{texto}", "{pistas}"] {
        assert!(!w.prompt.contains(placeholder), "{placeholder}");
    }
    assert!(w.prompt.contains("--- Página 2 (imagen 2) · digital ---\n[text] Escrito inventado número 2."));
    assert!(w.prompt.contains("- De la 1 a la 2: dudoso. Sin pistas."));
    // The same text, the same key; another text, another key. The images
    // (here, what pdfium tells of the paper) are not in it.
    assert_eq!(cut(&pages, &scans(3)).windows[0].key, w.key);
    let mut other_paper = scans(3);
    other_paper[2].w = 842;
    other_paper[2].h = 595;
    assert_eq!(cut(&pages, &other_paper).windows[0].key, w.key);
    let mut changed = pages.clone();
    changed[1] = prose(2, &["Otro escrito inventado.", "Termina acá."]);
    assert_ne!(cut(&changed, &scans(3)).windows[0].key, w.key);
    // A doubtful line is not transcribed: a warning stands for it.
    let mut doubtful = pages.clone();
    doubtful[2].zonas[0].renglones[1].seguro = Some(false);
    doubtful[2].zonas[0].texto = "Escrito inventado número 3.".into();
    let p = cut(&doubtful, &scans(3)).windows[0].prompt.clone();
    assert!(p.contains("[text] [dudoso: 1 renglón(es) sin transcribir; miralo en la imagen]"));
}

#[test]
fn a_page_two_windows_see_takes_the_least_sure_answer() {
    let s = |x: &str| x.to_string();
    assert_eq!(merge(&[(true, s("alta"), 1), (true, s("media"), 2)]), Some(PageAnswer { starts: true, sure: s("media") }));
    assert_eq!(merge(&[(true, s("media"), 1), (false, s("media"), 2)]), Some(PageAnswer { starts: true, sure: s("baja") }));
    assert_eq!(merge(&[(false, s("alta"), 2), (true, s("alta"), 1)]), Some(PageAnswer { starts: true, sure: s("baja") }));
    assert_eq!(merge(&[]), None);
    // Twelve pages, two windows: page 9 opens the second one, which does
    // not cut there; page 10 is seen by both.
    let c = cut(&doubtful_file(12), &scans(12));
    let answer = |first: u32, last: u32, starts: &[u32], sure: &str| {
        let pages: Vec<_> = (first..=last)
            .map(|p| json!({"pagina": p, "empieza_documento": p == first && first == 1 || starts.contains(&p), "seguridad": sure}))
            .collect();
        Some(json!({"paginas": pages, "documentos": [{"primera_pagina": first, "tipo": "other", "seguridad": "alta"}]}))
    };
    let answers = vec![answer(1, 10, &[10], "alta"), answer(9, 12, &[9, 10], "media")];
    let s = steps(&c, &answers);
    let at = |after: u32| s.iter().find(|x| x.after == after).unwrap();
    assert_eq!((at(8).model_says, at(8).model_confidence.as_deref()), (Some(false), Some("high")));
    assert_eq!((at(9).model_says, at(9).model_confidence.as_deref()), (Some(true), Some("medium")));
    assert!(at(9).uncertain() && !at(8).uncertain());
    // The answers of a model that numbered the images and not the pages.
    let moved = json!({"paginas": [{"pagina": 1, "empieza_documento": false, "seguridad": "alta"},
                                   {"pagina": 2, "empieza_documento": true, "seguridad": "alta"}], "documentos": []});
    let (pages, _) = window::answer_pages(9, 10, &moved);
    assert_eq!(pages.keys().copied().collect::<Vec<_>>(), vec![9, 10]);
}

#[test]
fn a_step_of_medium_confidence_stays_uncertain() {
    let step = |zones: bool, confidence: &str| CutStep {
        after: 3,
        verdict: Verdict::Doubtful,
        rule: "ninguna".into(),
        question: None,
        model_says: Some(true),
        model_confidence: Some(confidence.into()),
        rule_type: None,
        model_type: None,
        zones,
        hints: None,
    };
    assert!(step(true, "medium").uncertain());
    assert!(step(true, "low").uncertain());
    assert!(!step(true, "high").uncertain());
    // The cut of before keeps its rule: only a low one.
    assert!(!step(false, "medium").uncertain());
}

/// Answers every window: a document starts at the page of `starts`, with
/// `sure`; the others go on with «alta».
struct Windows {
    calls: AtomicUsize,
    starts: u32,
    sure: &'static str,
}

impl DocumentAsker for Windows {
    fn engine_id(&self) -> String {
        "falso:ventanas".into()
    }

    fn ask(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
        Err("this fake only looks at images".into())
    }

    fn sees_images(&self) -> bool {
        true
    }

    fn ask_with_images(&self, prompt: &str, _schema: &serde_json::Value, images: &[std::path::PathBuf]) -> Result<String, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert!(prompt.contains("=== Pistas de los saltos ==="));
        let pages: Vec<_> = (1..=images.len() as u32)
            .map(|p| {
                let sure = if p == self.starts { self.sure } else { "alta" };
                json!({"pagina": p, "empieza_documento": p == 1 || p == self.starts, "seguridad": sure})
            })
            .collect();
        Ok(json!({"paginas": pages, "documentos": [{"primera_pagina": 1, "tipo": "other", "seguridad": "alta"}]}).to_string())
    }
}

#[test]
fn the_cut_by_zones_runs_where_there_is_a_reading_and_a_person_wins() {
    let text = |w: &str| pdf_page(&[&format!("Texto inventado de la prueba del corte por zonas, {w}, con bastantes letras"), "para que se lea como una pagina digital."], 0.0);
    let dir = TempDir::new();
    let pages = [text("primera"), text("segunda"), text("tercera")];
    dir.write("o/zonas.pdf", &build(&pages));
    dir.write("o/sin-zonas.pdf", &build(&[text("una"), text("otra")]));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    while base.profile_pending(10).unwrap() > 0 {}
    let hash = |name: &str| -> String {
        base.conn().query_row("SELECT file_hash FROM location WHERE rel_path = ?1", [name], |row| row.get(0)).unwrap()
    };
    let (zoned, plain) = (hash("zonas.pdf"), hash("sin-zonas.pdf"));
    base.put_zones(&zoned, &doubtful_file(3), "prueba").unwrap();
    while base.cut_pending(10).unwrap() > 0 {}
    while base.cut_parts_pending(10).unwrap() > 0 {}

    // A file without a reading by zones keeps the cut of before.
    let before = base.cut_steps(&plain).unwrap();
    assert!(!before.is_empty() && before.iter().all(|s| !s.zones));
    let steps = base.cut_steps(&zoned).unwrap();
    assert!(steps.iter().all(|s| s.zones));
    assert_eq!(steps.iter().map(|s| s.verdict).collect::<Vec<_>>(), vec![Verdict::New, Verdict::Doubtful, Verdict::Doubtful]);
    assert_eq!(steps[1].hints.as_deref(), Some("[]"));

    // One window, never asked by itself (decision 3 of the plan): it waits
    // for a person, with the automatic reading on.
    let q = base.cut_questions().unwrap();
    assert_eq!((q.by_itself, q.on_request), (0, 1 + before.iter().filter(|s| s.verdict == Verdict::Doubtful && s.model_says.is_none() && s.question.is_some()).count()));
    base.set_background_reading(true).unwrap();
    let eyes = Windows { calls: AtomicUsize::new(0), starts: 3, sure: "alta" };
    base.cut_background(&eyes, 5).unwrap();
    assert_eq!(eyes.calls.load(Ordering::SeqCst), 0);
    assert!(base.waiting_for_cut_answers(true).unwrap().is_empty());

    // A person asks: one question for the window, and the parts follow it.
    let parts = base.cut_ask_file(&eyes, &zoned).unwrap();
    assert_eq!(eyes.calls.load(Ordering::SeqCst), 1);
    assert_eq!(parts.iter().map(|p| (p.first_page, p.last_page)).collect::<Vec<_>>(), vec![(1, 2), (3, 3)]);
    // Asked again, the answer kept is taken.
    base.cut_ask_file(&eyes, &zoned).unwrap();
    assert_eq!(eyes.calls.load(Ordering::SeqCst), 1);

    // A person's word wins over the model: one part, the whole file, with
    // nothing left to look at.
    let parts = base.cut_join_with_previous(&zoned, 3).unwrap();
    assert!(parts.is_empty(), "{parts:?}");
    assert!(base.cut_steps(&zoned).unwrap()[2].starts(), "the model's answer stays; the person's word wins over it");

    // An answer of medium confidence leaves the page for a person to look at.
    let dir2 = TempDir::new();
    dir2.write("p/zonas.pdf", &build(&pages));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("P", &dir2.path().join("p"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    while base.profile_pending(10).unwrap() > 0 {}
    let zoned: String = base.conn().query_row("SELECT file_hash FROM location", [], |row| row.get(0)).unwrap();
    base.put_zones(&zoned, &doubtful_file(3), "prueba").unwrap();
    while base.cut_pending(10).unwrap() > 0 {}
    let eyes = Windows { calls: AtomicUsize::new(0), starts: 3, sure: "media" };
    let parts = base.cut_ask_file(&eyes, &zoned).unwrap();
    assert_eq!(parts.iter().map(|p| (p.first_page, p.last_page)).collect::<Vec<_>>(), vec![(1, 2), (3, 3)]);
    assert_eq!(parts[1].doubtful_pages, vec![3]);
}

#[test]
fn while_a_reading_by_zones_is_coming_the_cut_waits_for_it() {
    let dir = TempDir::new();
    let text = |w: &str| pdf_page(&[&format!("Texto inventado de la espera de la lectura por zonas, {w}, con letras"), "para que se lea como digital."], 0.0);
    dir.write("o/a.pdf", &build(&[text("uno"), text("dos")]));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    while base.profile_pending(10).unwrap() > 0 {}
    base.conn().execute("INSERT INTO docbase_setting (key, value) VALUES (?1, 'on')", [super::ZONES_READING]).unwrap();
    assert_eq!(base.cut_pending(10).unwrap(), 0);
    assert_eq!(base.waiting_for_cut().unwrap().len(), 1);
    // The reading arrives: the file is cut by it, and nothing waits.
    let hash: String = base.conn().query_row("SELECT file_hash FROM location", [], |row| row.get(0)).unwrap();
    base.put_zones(&hash, &doubtful_file(2), "prueba").unwrap();
    assert_eq!(base.cut_pending(10).unwrap(), 1);
    while base.cut_parts_pending(10).unwrap() > 0 {}
    assert!(base.waiting_for_cut().unwrap().is_empty());
    assert!(base.cut_steps(&hash).unwrap()[0].zones);
}
