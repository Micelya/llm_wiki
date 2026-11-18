//! The vectors as a signal of the type (C2 of 2 nov), with invented texts,
//! vectors made for the test and a stand-in model (R-24): nothing here
//! comes from the test folder, from a vector of another run or from an
//! answer kept by another run. The vectors are written in the base by the
//! test: no Micelya Core is asked.

use std::sync::Mutex;

use rusqlite::params;

use crate::docbase::services::{EngineChoice, Service, NO_VECTORS};
use crate::docbase::testing::TempDir;
use crate::docbase::vectors::{TestCore, TEST_CORE};
use crate::docbase::DocBase;
use crate::source_volume::asking::DocumentAsker;

use super::hint::HINT_HEADER;

const MODEL: &str = "us.cohere.embed-v4:0";
const GROUP: &str = "Clasificá cada uno de los";
const ALONE: &str = "Clasificá un documento";

/// Invented texts, each of its own words, so no two make a family.
const TEXTS: [(&str, &str); 6] = [
    ("ajedrez.md", "Crónica inventada del torneo de ajedrez del club barrial: hubo doce mesas, dos relojes rotos y una final muy pareja que terminó en tablas."),
    ("huerta.md", "Apuntes de una huerta urbana imaginaria: tomates en hilera, albahaca junto a la medianera y riego por goteo los martes y viernes temprano."),
    ("faro.md", "Relato breve sobre un faro abandonado en una costa ficticia, su escalera de caracol oxidada y las gaviotas que anidan en la linterna."),
    ("bicicleta.md", "Guía casera para arreglar una bicicleta vieja: cambiar la cadena, centrar la rueda trasera y ajustar los frenos de varilla."),
    ("receta.md", "Receta de una sopa de calabaza inventada para la prueba, con jengibre rallado, caldo de verduras y un chorrito de crema al final."),
    ("viaje.md", "Diario de un viaje en tren por pueblos de nombres inventados, con estaciones de ladrillo, mate en el andén y un paisaje de trigales."),
];

/// Four firm words and sixteen read with low confidence (20 %): of a type
/// of low confidence (below 30 %).
const BLURRED: &str = "NOTA BORROSA DE PRUEBA\n[aproximado] mancha alfa beta gamma delta epsilon zeta eta theta\n\
[aproximado] iota kappa lambda mu nu xi omicron\n";

/// Another one, of other words: 3 firm and 14 approximate (18 %).
const BLURRED_TOO: &str = "SEGUNDA NOTA DIFUSA
[aproximado] borron uno dos tres cuatro cinco seis siete
[aproximado] ocho nueve diez once doce trece
";

/// A stand-in model: the type of each document by a word of its text, and
/// every question it was asked.
struct Model {
    answer: Box<dyn Fn(&str) -> Result<String, String> + Send + Sync>,
    asked: Mutex<Vec<String>>,
}

impl Model {
    fn new(answer: impl Fn(&str) -> Result<String, String> + Send + Sync + 'static) -> Self {
        Model { answer: Box::new(answer), asked: Mutex::new(Vec::new()) }
    }

    /// The questions of the type it was asked.
    fn types_asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().iter().filter(|q| q.starts_with("Clasificá")).cloned().collect()
    }
}

impl DocumentAsker for Model {
    fn engine_id(&self) -> String {
        "prueba:modelo".into()
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        assert_eq!(schema["additionalProperties"], false, "the answer's shape is strict");
        self.asked.lock().unwrap().push(prompt.to_string());
        if !prompt.starts_with("Clasificá") {
            // The data of the type: none.
            return Ok("{}".into());
        }
        (self.answer)(prompt)
    }
}

fn one(tipo: &str) -> serde_json::Value {
    serde_json::json!({"tipo": tipo, "etapa": null, "nuevo": false, "nombre": null, "descripcion": null, "confianza": 0.9, "razon": "por el texto", "otros": []})
}

/// The answer to a question alone or of a group: `pick` gives the type of
/// each document by its text.
fn answer_by(prompt: &str, pick: &dyn Fn(&str) -> &'static str) -> String {
    if !prompt.starts_with(GROUP) {
        return one(pick(prompt)).to_string();
    }
    let parts: Vec<&str> = prompt.split("\n--- Documento ").skip(1).collect();
    let documentos: Vec<serde_json::Value> = parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let mut answer = one(pick(part));
            let object = answer.as_object_mut().unwrap();
            object.remove("otros");
            object.insert("documento".into(), serde_json::json!(i + 1));
            answer
        })
        .collect();
    serde_json::json!({ "documentos": documentos }).to_string()
}

/// Vectors take part only while a Core of the test is set (`vectors::chosen`).
struct Vectors;

impl Vectors {
    fn on() -> Vectors {
        TEST_CORE.with(|core| *core.borrow_mut() = Some(TestCore::Missing));
        Vectors
    }
}

impl Drop for Vectors {
    fn drop(&mut self) {
        TEST_CORE.with(|core| *core.borrow_mut() = None);
    }
}

fn base_with(files: &[(&str, &str)]) -> (TempDir, DocBase, String) {
    let dir = TempDir::new();
    for (name, text) in files {
        dir.write(&format!("o/{name}"), text.as_bytes());
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    refresh(&base, &origin.id);
    (dir, base, origin.id)
}

fn refresh(base: &DocBase, origin: &str) {
    base.scan_origin(origin).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    while base.similar_step(10).unwrap() > 0 {}
}

fn version(base: &DocBase, name: &str) -> i64 {
    base.conn()
        .query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| row.get(0))
        .unwrap()
}

/// A vector of 1.024 numbers: these places with these values.
fn vector(parts: &[(usize, f32)]) -> Vec<f32> {
    let mut v = vec![0.0f32; 1024];
    for (place, value) in parts {
        v[*place] = *value;
    }
    v
}

/// Near the first axis (0,96 between two of them), each with its own tilt.
fn near(tilt: usize) -> Vec<f32> {
    vector(&[(0, 1.0), (tilt, 0.2)])
}

fn put_vector(base: &DocBase, name: &str, v: &[f32]) {
    let blob: Vec<u8> = v.iter().flat_map(|n| n.to_le_bytes()).collect();
    let id = version(base, name);
    let changed = base
        .conn()
        .execute(
            "INSERT OR REPLACE INTO version_vector (version_id, model, dimension, text_hash, text_at, vector, computed_at)
             SELECT ?1, ?2, 1024, 'de la prueba', d.created_at, ?3, 0 FROM derivative d WHERE d.version_id = ?1 AND d.kind = 'text'",
            params![id, MODEL, blob],
        )
        .unwrap();
    assert_eq!(changed, 1, "{name} has its text");
}

fn type_of(base: &DocBase, name: &str) -> Option<(String, String, Option<i64>)> {
    base.document_types_all().unwrap().remove(&version(base, name)).map(|t| (t.doc_type, t.source, t.principal))
}

fn by_word(text: &str) -> &'static str {
    if text.contains("ajedrez") {
        "acuerdo"
    } else if text.contains("huerta") {
        "recibo"
    } else if text.contains("faro") {
        "boleta"
    } else {
        "factura"
    }
}

#[test]
fn a_group_of_three_gets_three_types_in_one_question() {
    let _vectors = Vectors::on();
    let (_dir, base, _) = base_with(&TEXTS[..4]);
    put_vector(&base, "ajedrez.md", &near(1));
    put_vector(&base, "huerta.md", &near(2));
    put_vector(&base, "faro.md", &near(3));
    put_vector(&base, "bicicleta.md", &vector(&[(5, 1.0)]));
    // Three alone near one another and one far: two questions, not four.
    assert_eq!(base.types_plan().unwrap().type_questions, 2);
    let model = Model::new(|prompt| Ok(answer_by(prompt, &by_word)));
    base.types_run(&model, 10).unwrap();
    let asked = model.types_asked();
    assert_eq!(asked.len(), 2, "{asked:#?}");
    let group = asked.iter().find(|q| q.starts_with(GROUP)).expect("the question of the group");
    assert!(group.starts_with("Clasificá cada uno de los 3 documentos"));
    for word in ["ajedrez", "huerta", "faro"] {
        assert!(group.contains(word), "the text of each one goes in the question");
    }
    assert!(!group.contains("bicicleta"));
    // Each one its own type, asked about itself: it is not a family.
    for (name, tipo) in [("ajedrez.md", "acuerdo"), ("huerta.md", "recibo"), ("faro.md", "boleta"), ("bicicleta.md", "factura")] {
        let id = version(&base, name);
        assert_eq!(type_of(&base, name), Some((tipo.to_string(), "model".to_string(), Some(id))), "{name}");
    }
    assert_eq!(base.types_plan().unwrap().type_questions, 0);
    assert_eq!(base.types_run(&model, 10).unwrap().asked, 0, "never asked twice");
}

/// Three documents typed as invoices, then a fourth very close to the three.
/// The vectors of the test must be on (`Vectors::on`) in the test that calls it.
fn three_typed_and_one_more(answer_of_the_fourth: &'static str) -> (TempDir, DocBase, Model) {
    let (dir, base, origin) = base_with(&TEXTS[..3]);
    for (name, tilt) in [("ajedrez.md", 1), ("huerta.md", 2), ("faro.md", 3)] {
        put_vector(&base, name, &near(tilt));
    }
    let model = Model::new(move |prompt| {
        Ok(answer_by(prompt, &|text: &str| if text.contains("bicicleta") { answer_of_the_fourth } else { "factura" }))
    });
    base.types_run(&model, 10).unwrap();
    dir.write("o/bicicleta.md", TEXTS[3].1.as_bytes());
    refresh(&base, &origin);
    put_vector(&base, "bicicleta.md", &near(4));
    base.types_run(&model, 10).unwrap();
    (dir, base, model)
}

#[test]
fn the_question_carries_the_three_nearest_neighbours_with_their_type_and_likeness() {
    let _vectors = Vectors::on();
    let (_dir, base, model) = three_typed_and_one_more("factura");
    let asked = model.types_asked();
    assert_eq!(asked.len(), 2, "the group of three, then the fourth alone");
    let fourth = asked.last().unwrap();
    assert!(fourth.starts_with(ALONE));
    assert!(fourth.contains(HINT_HEADER), "{fourth}");
    assert_eq!(fourth.matches("- factura (del catálogo): parecido 0,96").count(), 3, "{fourth}");
    assert!(fourth.contains("Pesan más los tipos del catálogo o confirmados por una persona"));
    // The first question had no neighbour typed: no hint.
    assert!(!asked[0].contains(HINT_HEADER));
    let fourth = version(&base, "bicicleta.md");
    let key: String = base.conn().query_row("SELECT question FROM document_type WHERE version_id = ?1", [fourth], |r| r.get(0)).unwrap();
    assert!(key.starts_with("tipo-3:"), "{key}");
    assert!(base.types_by_similarity().unwrap().doubtful.is_empty(), "it agrees with its neighbours");
}

#[test]
fn an_answer_against_three_very_close_neighbours_is_marked_doubtful_and_not_asked_again() {
    let _vectors = Vectors::on();
    let (_dir, base, model) = three_typed_and_one_more("recibo");
    assert_eq!(type_of(&base, "bicicleta.md").unwrap().0, "recibo", "the answer of the model stays");
    let found = base.types_by_similarity().unwrap();
    assert_eq!(found.doubtful.len(), 1);
    let doubt = &found.doubtful[0];
    assert_eq!((doubt.version_id, doubt.doc_type.as_str(), doubt.neighbour_type.as_str()), (version(&base, "bicicleta.md"), "recibo", "factura"));
    assert_eq!(doubt.likeness.len(), 3);
    assert!(doubt.likeness.iter().all(|s| *s > 0.95));
    assert_eq!(doubt.name, "bicicleta.md");
    let before = model.types_asked().len();
    assert_eq!(base.types_run(&model, 10).unwrap().asked, 0);
    assert_eq!(model.types_asked().len(), before, "not asked again");
}

#[test]
fn a_document_of_low_confidence_is_no_neighbour_no_member_of_a_group_and_gets_no_hint() {
    let _vectors = Vectors::on();
    let (dir, base, origin) = base_with(&[("borrosa.md", BLURRED), TEXTS[1]]);
    put_vector(&base, "borrosa.md", &near(1));
    put_vector(&base, "huerta.md", &vector(&[(0, 0.5), (7, 1.0)]));
    let pick = |text: &str| if text.contains("BORROSA") || text.contains("DIFUSA") { "factura" } else { "recibo" };
    let model = Model::new(move |prompt| Ok(answer_by(prompt, &pick)));
    base.types_run(&model, 10).unwrap();
    assert_eq!(type_of(&base, "borrosa.md").unwrap().0, "factura", "it is asked its type, as every document");
    // A new one very close to the blurred one (0,96), and another blurred
    // one very close to the new one, with the two typed before.
    dir.write("o/faro.md", TEXTS[2].1.as_bytes());
    dir.write("o/difusa.md", BLURRED_TOO.as_bytes());
    refresh(&base, &origin);
    put_vector(&base, "faro.md", &near(2));
    put_vector(&base, "difusa.md", &near(5));
    base.types_run(&model, 10).unwrap();
    let asked = model.types_asked();
    assert_eq!(asked.len(), 4, "{asked:#?}");
    assert!(asked.iter().all(|q| !q.starts_with(GROUP)), "the blurred one goes in no group");
    let blurred = asked.iter().find(|q| q.contains("DIFUSA")).unwrap();
    assert!(!blurred.contains(HINT_HEADER), "no hint for one of low confidence");
    let lighthouse = asked.iter().find(|q| q.contains("faro")).unwrap();
    assert!(lighthouse.contains(HINT_HEADER));
    assert!(lighthouse.contains("- recibo (del catálogo)"), "{lighthouse}");
    assert!(!lighthouse.contains("- factura (del catálogo)"), "the one of low confidence is no neighbour: {lighthouse}");
}

#[test]
fn with_none_the_question_of_the_type_is_the_one_of_before() {
    let _vectors = Vectors::on();
    let (_dir, base, _) = base_with(&TEXTS[..3]);
    let choice = EngineChoice { engine: NO_VECTORS.into(), connection: "micelya-core".into(), ..Default::default() };
    base.set_engine_of(Service::Embed, Some(choice)).unwrap();
    for (name, tilt) in [("ajedrez.md", 1), ("huerta.md", 2), ("faro.md", 3)] {
        put_vector(&base, name, &near(tilt));
    }
    let model = Model::new(|prompt| Ok(answer_by(prompt, &by_word)));
    base.types_run(&model, 10).unwrap();
    let asked = model.types_asked();
    assert_eq!(asked.len(), 3, "one question each, no group");
    assert!(asked.iter().all(|q| q.starts_with(ALONE) && !q.contains(HINT_HEADER)));
    let keys: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT question FROM document_type ORDER BY version_id").unwrap();
        let rows = statement.query_map([], |r| r.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    assert!(keys.iter().all(|k| k.starts_with("tipo-2:")), "{keys:?}");
    assert!(base.types_by_similarity().unwrap() == Default::default());
}

#[test]
fn with_the_hint_off_the_question_has_no_neighbours_but_the_groups_the_mark_and_the_rescue_go_on() {
    let _vectors = Vectors::on();
    let (dir, base, origin) = base_with(&TEXTS[..3]);
    base.set_type_hint(false).unwrap();
    assert!(!base.type_hint().unwrap());
    for (name, tilt) in [("ajedrez.md", 1), ("huerta.md", 2), ("faro.md", 3)] {
        put_vector(&base, name, &near(tilt));
    }
    let model = Model::new(|prompt| {
        if prompt.contains("receta") {
            return Err("el modelo no pudo leer este documento".into());
        }
        Ok(answer_by(prompt, &|text: &str| if text.contains("bicicleta") { "recibo" } else { "factura" }))
    });
    base.types_run(&model, 10).unwrap();
    assert!(model.types_asked()[0].starts_with("Clasificá cada uno de los 3 documentos"), "the group goes on");
    // Two more: one against its neighbours, one whose question fails.
    dir.write("o/bicicleta.md", TEXTS[3].1.as_bytes());
    dir.write("o/receta.md", TEXTS[4].1.as_bytes());
    refresh(&base, &origin);
    put_vector(&base, "bicicleta.md", &near(4));
    put_vector(&base, "receta.md", &vector(&[(0, 1.0), (11, 0.6)]));
    base.types_run(&model, 10).unwrap();
    let asked = model.types_asked();
    assert!(asked.iter().all(|q| !q.contains(HINT_HEADER)), "no question carries the neighbours");
    assert!(asked.iter().filter(|q| q.contains("bicicleta")).all(|q| q.starts_with(ALONE)));
    let found = base.types_by_similarity().unwrap();
    assert_eq!(found.doubtful.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["bicicleta.md"], "the mark goes on");
    // The rescue: the document whose question failed has no type, and its
    // neighbours suggest one.
    assert_eq!(type_of(&base, "receta.md"), None);
    assert_eq!(found.suggested.len(), 1);
    let rescue = &found.suggested[0];
    assert_eq!((rescue.name.as_str(), rescue.doc_type.as_str()), ("receta.md", "factura"));
    assert_eq!(rescue.neighbours.len(), 3);
    assert_eq!(rescue.why, "la pregunta de su tipo falló");
    assert!(rescue.likeness > 0.8 && rescue.likeness < 0.9, "{}", rescue.likeness);
}

#[test]
fn with_the_hint_on_the_type_waits_for_its_vector_while_core_fails() {
    let _vectors = Vectors::on();
    let (_dir, base, _) = base_with(&TEXTS[..2]);
    put_vector(&base, "ajedrez.md", &near(1));
    // The other one has no vector, and Core cannot be reached.
    base.set_background_reading(true).unwrap();
    assert!(base.vectors_operation().unwrap());
    let model = Model::new(|prompt| Ok(answer_by(prompt, &by_word)));
    base.types_run(&model, 10).unwrap();
    assert_eq!(model.types_asked().len(), 1);
    assert_eq!(type_of(&base, "huerta.md"), None, "it waits for its vector");
    let left = super::classify::types_left(&base.conn(), None).unwrap();
    assert_eq!((left.units, left.held), (0, 1));
    assert!(base.contents_waiting_for_type().unwrap().contains(&version(&base, "huerta.md")));
    // With the hint off, it does not wait.
    base.set_type_hint(false).unwrap();
    base.types_run(&model, 10).unwrap();
    assert_eq!(type_of(&base, "huerta.md").unwrap().0, "recibo");
}

#[test]
fn the_question_without_the_hint_to_measure_changes_no_type_and_is_kept_apart() {
    let _vectors = Vectors::on();
    let (_dir, base, _model) = three_typed_and_one_more("recibo");
    let fourth = version(&base, "bicicleta.md");
    let answers = |base: &DocBase| -> i64 { base.conn().query_row("SELECT count(*) FROM model_answer", [], |r| r.get(0)).unwrap() };
    let before = (type_of(&base, "bicicleta.md"), answers(&base));
    let typed = base.typed_by_the_model(true).unwrap();
    let measured: Vec<_> = typed.iter().filter(|t| t.question_version == "tipo-3").collect();
    assert_eq!(measured.len(), 4, "the group of three and the fourth with the hint: {typed:?}");
    assert!(measured.iter().find(|t| t.version_id == fourth).is_some_and(|t| t.with_hint && !t.in_group));
    assert!(measured.iter().filter(|t| t.version_id != fourth).all(|t| t.in_group && !t.with_hint));

    let other = Model::new(|prompt| Ok(answer_by(prompt, &|_| "acuerdo")));
    let control = base.type_without_hint(&other, fourth).unwrap();
    assert_eq!((control.doc_type.as_str(), control.asked), ("acuerdo", true));
    assert!(control.question.starts_with("tipo-2:"));
    let asked = other.types_asked();
    assert_eq!(asked.len(), 1);
    assert!(asked[0].starts_with(ALONE) && !asked[0].contains(HINT_HEADER), "the question of before");
    // Nothing of the process changed, and it is kept apart.
    assert_eq!((type_of(&base, "bicicleta.md"), answers(&base)), before);
    let apart: i64 = base.conn().query_row("SELECT count(*) FROM type_control_answer WHERE version_id = ?1", [fourth], |r| r.get(0)).unwrap();
    assert_eq!(apart, 1);
    assert_eq!(base.types_run(&other, 10).unwrap().asked, 0, "the process does not take it");
    assert_eq!(type_of(&base, "bicicleta.md"), before.0);
    // Asked again: the same question of the same engine is not sent again.
    assert!(!base.type_without_hint(&other, fourth).unwrap().asked);
    assert_eq!(other.types_asked().len(), 1);
}
