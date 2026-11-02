//! The one rule of the steps that ask and the documents of a type of low
//! confidence (E3 of 2 nov; D-769, D-775): a document with one word is
//! asked, and what comes out of one of low confidence is set aside, with its
//! reason, from every step and use that makes information. Since E6 of 2
//! nov it is read too, and its reading kept, marked: only for queries. Invented texts
//! and a stand-in model (R-24): nothing here comes from the test folder or
//! from an answer kept by another run.

use super::tests::{base_with, empty_answer, is_knowledge, mention, refresh, type_answer, FakeModel};
use crate::docbase::process::steps::{count_step, Scope, Step};
use crate::docbase::thresholds::Thresholds;
use crate::docbase::DocBase;

/// Five firm words.
const RECEIPT: &str = "Recibo de pago Persona Siete";

/// 9 firm words and 18 read with low confidence: 33 % of safe words, below
/// «no legible» (50 %) and above the type of low confidence (30 %).
const ACT: &str = "ACTA DE PRUEBA\nComparece Persona Cuatro por derecho propio.\n\
[aproximado] borroso uno dos tres cuatro cinco seis siete ocho nueve diez\n\
[aproximado] once doce trece catorce quince dieciseis diecisiete\n";

/// Another one, of 4 firm words and 16 approximate: 20 %.
const NOTE: &str = "NOTA DE PRUEBA Ocho\n[aproximado] mancha alfa beta gamma delta epsilon zeta eta theta\n\
[aproximado] iota kappa lambda mu nu xi omicron\n";

const LEASE: &str = "CONTRATO DE LOCACIÓN DE PRUEBA\n\nEntre Ejemplo Ocho S.A., en adelante la locadora, y Persona Nueve, \
en adelante el locatario, se celebra el presente contrato de locación de un depósito inventado.";

fn model() -> FakeModel {
    FakeModel::new("prueba:modelo", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        if prompt.contains("Comparece Persona Cuatro") {
            super::tests::answer(
                vec![mention("a", "persona", "Persona Cuatro", "parte", "Comparece Persona Cuatro por derecho propio", serde_json::json!([]))],
                vec![],
            )
        } else if prompt.contains("Recibo de pago") {
            super::tests::answer(vec![mention("a", "persona", "Persona Siete", "parte", "Recibo de pago Persona Siete", serde_json::json!([]))], vec![])
        } else {
            empty_answer()
        }
    })
}

fn version_of(base: &DocBase, name: &str) -> i64 {
    base.conn()
        .query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| row.get(0))
        .unwrap()
}

fn set_shares(base: &DocBase, low_confidence_type: f64, quality: f64) {
    base.set_thresholds(&Thresholds { low_confidence_type, quality, ..Thresholds::default() }).unwrap();
}

fn asked_about(model: &FakeModel, words: &str, knowledge: bool) -> usize {
    model.asked.lock().unwrap().iter().filter(|q| is_knowledge(q) == knowledge && q.contains(words)).count()
}

#[test]
fn a_document_of_five_words_is_asked_its_type_and_read_and_en_curso_counts_it_the_same() {
    let (_dir, base) = base_with(&[("recibo.txt", RECEIPT)]);
    let receipt = version_of(&base, "recibo.txt");
    let words: i64 = base.conn().query_row("SELECT word_count FROM version WHERE id = ?1", [receipt], |row| row.get(0)).unwrap();
    assert_eq!(words, 5);
    // «En curso»: the step of the type has it (before, it needed 10 words of
    // its signature).
    let count = count_step(&base.conn(), Step::Type, &Scope::All).unwrap();
    assert_eq!(count.pending + count.set_aside, 1);
    let types = base.progress(&Default::default()).unwrap().stages.into_iter().find(|s| s.id == "types").unwrap();
    assert_eq!(types.total, 1);
    let model = model();
    base.types_run(&model, 10).unwrap();
    assert_eq!(asked_about(&model, "Recibo de pago Persona Siete", false), 1, "its type is asked");
    let count = count_step(&base.conn(), Step::Reading, &Scope::All).unwrap();
    assert_eq!(count.pending + count.set_aside, 1, "the reading has it too");
    base.knowledge_run(&model, 10).unwrap();
    assert_eq!(asked_about(&model, "Recibo de pago Persona Siete", true), 1, "and it is read");
    let reading = base.progress(&Default::default()).unwrap().stages.into_iter().find(|s| s.id == "reading").unwrap();
    assert_eq!((reading.total, reading.done, reading.out), (1, 1, 0));
}

#[test]
fn a_document_below_no_legible_is_of_low_confidence_and_one_above_both_is_not() {
    let (_dir, base) = base_with(&[("acta.txt", ACT), ("contrato.txt", LEASE)]);
    let (act, lease) = (version_of(&base, "acta.txt"), version_of(&base, "contrato.txt"));
    let words = crate::docbase::safe_words::of(&base.conn(), act).unwrap().unwrap();
    assert_eq!((words.firm, words.approximate, words.discarded), (9, 18, 0));
    // By default (30 % and 50 %): 33 % is above the type of low confidence
    // but below «no legible», which is of low confidence too.
    let low = crate::docbase::process::steps::low_confidence(&base.conn()).unwrap();
    assert!(low.contains(&act) && !low.contains(&lease), "{low:?}");
    let why = crate::docbase::safe_words::reasons(&base.conn()).unwrap();
    assert!(why[&act].starts_with("no legible: 33 % de palabras seguras (umbral 50 %)"), "{why:?}");
    // Below «no legible» only when it is the higher one; the threshold is
    // read when looking: moving it asks nothing again.
    set_shares(&base, 0.30, 0.20);
    assert!(crate::docbase::process::steps::low_confidence(&base.conn()).unwrap().is_empty());
    set_shares(&base, 0.40, 0.20);
    let why = crate::docbase::safe_words::reasons(&base.conn()).unwrap();
    assert!(why[&act].starts_with("tipo de confianza baja: 33 %"), "{why:?}");
}

#[test]
fn a_document_of_low_confidence_is_set_aside_from_each_step_with_its_reason() {
    let (dir, base) = base_with(&[("acta.txt", ACT), ("contrato.txt", LEASE)]);
    let act = version_of(&base, "acta.txt");
    let model = model();
    // Read while the thresholds let it in: what it said joins the record.
    set_shares(&base, 0.0, 0.0);
    base.types_run(&model, 10).unwrap();
    base.knowledge_run(&model, 10).unwrap();
    base.knowledge_resolve(None, 0).unwrap();
    let entity_of = |base: &DocBase| -> Option<String> {
        base.conn()
            .query_row(
                "SELECT e.state FROM knowledge_mention m JOIN knowledge_entity e ON e.id = m.entity_id WHERE m.version_id = ?1",
                [act],
                |row| row.get(0),
            )
            .ok()
    };
    assert!(entity_of(&base).is_some(), "joined while it was not of low confidence");
    let families = crate::docbase::knowledge::pages::document_families(&base).unwrap();
    assert!(families.iter().any(|(_, members)| members.contains(&act)));

    // The thresholds by default: it is of low confidence («no legible»).
    set_shares(&base, 0.30, 0.50);
    // The identity: its mentions join nobody, as a document left out.
    base.knowledge_resolve(None, 0).unwrap();
    assert_eq!(entity_of(&base), None, "its mention left the record");
    let resolved_by: String =
        base.conn().query_row("SELECT resolved_by FROM knowledge_mention WHERE version_id = ?1", [act], |row| row.get(0)).unwrap();
    assert_eq!(resolved_by, "excluido");
    let retired: String = base.conn().query_row("SELECT state FROM knowledge_entity WHERE name = 'Persona Cuatro'", [], |row| row.get(0)).unwrap();
    assert_eq!(retired, "retired");
    // The matters: it is in none.
    let in_matter: i64 =
        base.conn().query_row("SELECT count(*) FROM knowledge_matter_document WHERE version_id = ?1", [act], |row| row.get(0)).unwrap();
    assert_eq!(in_matter, 0);
    // The pages: it has no page of its own.
    let families = crate::docbase::knowledge::pages::document_families(&base).unwrap();
    assert!(!families.iter().any(|(_, members)| members.contains(&act)), "{families:?}");

    // A new one of low confidence: its type is asked, with its approximate
    // words; its data and its reading are too, and kept, marked (E6 of 2
    // nov, over E3: the model is never skipped). It is only for queries:
    // none of its entities joins the identities nor has a page.
    dir.write("o/nota.txt", NOTE.as_bytes());
    let origin: String = base.conn().query_row("SELECT id FROM origin", [], |row| row.get(0)).unwrap();
    refresh(&base, &origin);
    let note = version_of(&base, "nota.txt");
    base.types_run(&model, 10).unwrap();
    let typed = model.asked.lock().unwrap().iter().filter(|q| !is_knowledge(q) && q.contains("[aproximado] mancha alfa")).count();
    assert_eq!(typed, 1, "its type is asked with its approximate words");
    let count = count_step(&base.conn(), Step::TypeData, &Scope::Document(note)).unwrap();
    assert_eq!(count.set_aside, 0, "the data of its type are looked for");
    let count = count_step(&base.conn(), Step::Reading, &Scope::Document(note)).unwrap();
    assert_eq!((count.pending, count.set_aside), (1, 0), "it is read like the others");
    base.knowledge_run(&model, 10).unwrap();
    assert_eq!(asked_about(&model, "NOTA DE PRUEBA Ocho", true), 1, "it is read");
    let read: String = base.conn().query_row("SELECT state FROM knowledge_extraction WHERE version_id = ?1", [note], |row| row.get(0)).unwrap();
    assert_eq!(read, "done", "its reading is kept");
    let items = base.progress_items("reading", &Default::default(), 50).unwrap().items;
    let item = items.iter().find(|i| i.content_id == Some(note)).unwrap();
    assert_eq!(item.status, "done");
    assert!(item.detail.as_deref().unwrap().starts_with("no legible: 20 % de palabras seguras"), "{:?}", item.detail);
    base.knowledge_resolve(None, 0).unwrap();
    let joined: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_mention WHERE version_id = ?1 AND entity_id IS NOT NULL", [note], |row| row.get(0))
        .unwrap();
    assert_eq!(joined, 0, "its mentions join no identity");
    let families = crate::docbase::knowledge::pages::document_families(&base).unwrap();
    assert!(!families.iter().any(|(_, members)| members.contains(&note)), "no page of its own");
    // The queries show it marked: the sheet of the document and the search.
    let location: i64 = base.conn().query_row("SELECT id FROM location WHERE name = 'nota.txt'", [], |row| row.get(0)).unwrap();
    let detail = base.document(&crate::docbase::Visible::All, location).unwrap().unwrap();
    assert!(detail.low_confidence.as_deref().is_some_and(|m| m.starts_with("no legible: 20 %")), "{:?}", detail.low_confidence);
    let found = base.search_documents(&crate::docbase::Visible::All, "NOTA DE PRUEBA Ocho", 10, None).unwrap();
    let hit = found.hits.iter().find(|h| h.content_id == note).expect("it is found");
    assert!(hit.low_confidence.is_some(), "and shown marked");

    // Lowering the threshold lets them in: no mark, and nothing asked again.
    set_shares(&base, 0.10, 0.10);
    let count = count_step(&base.conn(), Step::Reading, &Scope::Document(note)).unwrap();
    assert_eq!((count.pending, count.set_aside), (0, 0));
    let detail = base.document(&crate::docbase::Visible::All, location).unwrap().unwrap();
    assert_eq!(detail.low_confidence, None);
}

/// The same act, 50 words: two firm copies and one read with low confidence.
fn act_of(name: &str, marked: bool) -> String {
    let lines = [
        "ACTA DE ASAMBLEA DE PRUEBA",
        "En la ciudad inventada, a los diez dias del mes de marzo, se reunen los socios de Ejemplo Diez S.A.",
        "para tratar el orden del dia: aprobacion de los estados contables y eleccion de autoridades.",
        "Se aprueba por unanimidad la gestion del directorio y se designa presidente a Persona Once.",
        "No habiendo mas asuntos que tratar se levanta la sesion firmando los presentes.",
    ];
    let mut out = String::new();
    for (index, line) in lines.iter().enumerate() {
        if marked && index > 0 {
            out.push_str("[aproximado] ");
        }
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(name);
    out
}

#[test]
fn a_family_takes_no_type_from_a_document_of_low_confidence_nor_describes_it() {
    let (_dir, base) =
        base_with(&[("a.txt", &act_of("Copia A", true)), ("b.txt", &act_of("Copia B", false)), ("c.txt", &act_of("Copia C", false))]);
    let (a, b, c) = (version_of(&base, "a.txt"), version_of(&base, "b.txt"), version_of(&base, "c.txt"));
    assert!(crate::docbase::process::steps::low_confidence(&base.conn()).unwrap().contains(&a));
    // The three are one family by their text (the approximate lines count as
    // a signal).
    let related: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM similar_pair WHERE (a = ?1 OR b = ?1) AND state != 'discarded'", [a], |row| row.get(0))
        .unwrap();
    assert!(related > 0, "the low one is related to the copies");
    let model = model();
    base.types_run(&model, 10).unwrap();
    let principal_of = |id: i64| -> Option<i64> {
        base.conn().query_row("SELECT principal FROM document_type WHERE version_id = ?1", [id], |row| row.get(0)).unwrap()
    };
    // The low one is typed alone, about itself; no other takes its answer.
    assert_eq!(principal_of(a), Some(a));
    assert_ne!(principal_of(b), Some(a));
    assert_ne!(principal_of(c), Some(a));
    // Nor is it described in the question of the copies.
    let questions = model.asked.lock().unwrap().clone();
    assert!(questions.iter().filter(|q| q.contains("Copia B") || q.contains("Copia C")).all(|q| !q.contains("Copia A")), "{questions:?}");
}
