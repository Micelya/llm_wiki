//! The extraction of the knowledge layer over a real base, with invented
//! documents and a stand-in model that never calls a real engine (R-24:
//! nothing here comes from the test folder).

use std::sync::Mutex;

use super::extract::{KnowledgeReport, KNOWLEDGE_PROMPT};
use crate::docbase::query::DocumentFilter;
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};
use crate::source_volume::asking::DocumentAsker;

/// Answers each question with what `answer` says for it, and remembers
/// every question it was asked.
pub(crate) struct FakeModel {
    engine: String,
    answer: Box<dyn Fn(&str) -> String + Send + Sync>,
    pub(crate) asked: Mutex<Vec<String>>,
}

impl FakeModel {
    pub(crate) fn new(engine: &str, answer: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        Self { engine: engine.into(), answer: Box::new(answer), asked: Mutex::new(Vec::new()) }
    }

    fn knowledge_calls(&self) -> usize {
        self.asked.lock().unwrap().iter().filter(|q| is_knowledge(q)).count()
    }
}

impl DocumentAsker for FakeModel {
    fn engine_id(&self) -> String {
        self.engine.clone()
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        assert_eq!(schema["additionalProperties"], false, "the answer's shape is strict");
        self.asked.lock().unwrap().push(prompt.to_string());
        Ok((self.answer)(prompt))
    }
}

pub(crate) fn is_knowledge(prompt: &str) -> bool {
    prompt.starts_with("Sos asistente de un estudio jurídico. Leé el documento")
}

const LEASE: &str = "CONTRATO DE LOCACIÓN\n\nEntre Ejemplo Uno S.A., CUIT 30-11111111-1, representada por su presidente \
Persona Dos, DNI 22.222.222, en adelante la locadora, y Persona Tres, DNI 33.333.333, en adelante el locatario, \
se celebra el presente contrato de locación.\nEl locatario constituye domicilio en Calle Inventada 456.\n\
El canon mensual es de $ 100.000 a partir del 1 de marzo.";

const RECIPE: &str = "Bizcochuelo de vainilla. Batir cuatro huevos con una taza de azúcar hasta que dupliquen su volumen, \
agregar una taza de harina tamizada de a poco y hornear a temperatura moderada durante cuarenta minutos.";

pub(crate) fn mention(reference: &str, kind: &str, written: &str, role: &str, quote: &str, ids: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "ref": reference, "misma_que": null, "tipo": kind, "escrito": written, "rol": role,
        "cita": quote, "pagina": null, "identificadores": ids
    })
}

pub(crate) fn claim(subject: &str, predicate: &str, object_ref: Option<&str>, value: Option<&str>, quote: &str, time: Option<&str>) -> serde_json::Value {
    serde_json::json!({
        "sujeto": subject, "predicado": predicate, "objeto_ref": object_ref, "objeto_valor": value,
        "cita": quote, "pagina": null, "tiempo": time
    })
}

/// What a careful model would say of the lease, plus five things the
/// check must throw away.
fn lease_answer() -> String {
    serde_json::json!({
        "menciones": [
            mention("a", "organizacion", "Ejemplo Uno S.A.", "parte", "Entre Ejemplo Uno S.A., CUIT 30-11111111-1",
                serde_json::json!([{"clase": "cuit", "valor": "30-11111111-1", "cita": "Ejemplo Uno S.A., CUIT 30-11111111-1"}])),
            mention("b", "persona", "Persona Dos", "apoderado", "representada por su presidente Persona Dos, DNI 22.222.222",
                serde_json::json!([{"clase": "dni", "valor": "22.222.222", "cita": "Persona Dos, DNI 22.222.222"}])),
            mention("c", "persona", "Persona Tres", "parte", "y Persona Tres, DNI 33.333.333, en adelante el locatario",
                serde_json::json!([{"clase": "dni", "valor": "33.333.334", "cita": "Persona Tres, DNI 33.333.333"}])),
            // Not in the text.
            mention("d", "persona", "Persona Cuatro", "testigo", "firma como testigo Persona Cuatro", serde_json::json!([])),
            // The name is not inside its quote.
            mention("e", "persona", "Persona Cinco", "parte", "en adelante la locadora", serde_json::json!([])),
        ],
        "afirmaciones": [
            claim("b", "cargo", None, Some("presidente"), "representada por su presidente Persona Dos", None),
            claim("b", "representa_a", Some("a"), None, "Ejemplo Uno S.A., CUIT 30-11111111-1, representada por su presidente Persona Dos", None),
            claim("c", "domicilio", None, Some("Calle Inventada 456"), "El locatario constituye domicilio en Calle Inventada 456", None),
            claim("c", "monto_adeudado", None, Some("$ 100.000"), "El canon mensual es de $ 100.000 a partir del 1 de marzo", Some("a partir del 1 de marzo")),
            // Said twice: kept once.
            claim("c", "domicilio", None, Some("calle inventada 456"), "constituye domicilio en Calle Inventada 456", None),
            // A position is said of a person, not of a company.
            claim("a", "cargo", None, Some("locadora"), "en adelante la locadora", None),
            // The value is not in the quote.
            claim("c", "domicilio", None, Some("Calle Falsa 1"), "El locatario constituye domicilio en Calle Inventada 456", None),
            // The subject is a mention that was discarded.
            claim("d", "cargo", None, Some("testigo"), "en adelante el locatario", None),
        ]
    })
    .to_string()
}

pub(crate) fn empty_answer() -> String {
    serde_json::json!({ "menciones": [], "afirmaciones": [] }).to_string()
}

pub(crate) fn type_answer(tipo: &str) -> String {
    serde_json::json!({
        "tipo": tipo, "etapa": null, "nuevo": false, "nombre": null, "descripcion": null,
        "confianza": 0.9, "razon": "por el texto", "otros": []
    })
    .to_string()
}

/// The model of these tests: types, and the reading of each document.
fn model(engine: &str) -> FakeModel {
    FakeModel::new(engine, |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        if prompt.contains("CONTRATO DE LOCACIÓN") {
            lease_answer()
        } else {
            empty_answer()
        }
    })
}

pub(crate) fn base_with(files: &[(&str, &str)]) -> (TempDir, DocBase) {
    let dir = TempDir::new();
    for (name, text) in files {
        dir.write(&format!("o/{name}"), text.as_bytes());
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    refresh(&base, &origin.id);
    (dir, base)
}

pub(crate) fn refresh(base: &DocBase, origin: &str) {
    base.scan_origin(origin).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    while base.similar_step(10).unwrap() > 0 {}
}

pub(crate) fn location(base: &DocBase, name: &str) -> i64 {
    base.documents(&Visible::All, &DocumentFilter::default())
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("no document named {name}"))
        .location_id
}

fn run(base: &DocBase, asker: &FakeModel) -> KnowledgeReport {
    base.types_run(asker, 50).unwrap();
    base.knowledge_run(asker, 50).unwrap()
}

#[test]
fn only_what_is_in_the_text_is_kept() {
    let (_dir, base) = base_with(&[("contrato.txt", LEASE), ("receta.txt", RECIPE)]);
    let asker = model("falso:1");
    let report = run(&base, &asker);
    assert_eq!((report.asked, report.extracted, report.failed, report.left), (2, 2, 0, 0), "{report:?}");

    let found = base.knowledge_of(&Visible::All, location(&base, "contrato.txt")).unwrap();
    assert_eq!(found.state, "done");
    let names: Vec<(&str, &str, &str)> =
        found.mentions.iter().map(|m| (m.kind.as_str(), m.written.as_str(), m.role.as_str())).collect();
    assert_eq!(
        names,
        [("organizacion", "Ejemplo Uno S.A.", "parte"), ("persona", "Persona Dos", "apoderado"), ("persona", "Persona Tres", "parte")]
    );
    // Identifiers: a value that is not in its quote is dropped; one the
    // base recognized by its shape says so.
    let ids: Vec<(&str, &str, bool)> = found
        .mentions
        .iter()
        .flat_map(|m| m.identifiers.iter().map(|i| (i.kind.as_str(), i.value.as_str(), i.by_shape)))
        .collect();
    assert_eq!(ids, [("cuit", "30-11111111-1", true), ("dni", "22.222.222", true)]);

    let said: Vec<(&str, Option<&str>, Option<i64>)> =
        found.evidence.iter().map(|e| (e.predicate.as_str(), e.object_value.as_deref(), e.object_mention)).collect();
    let company = found.mentions[0].id;
    assert_eq!(
        said,
        [
            ("cargo", Some("presidente"), None),
            ("representa_a", None, Some(company)),
            ("domicilio", Some("Calle Inventada 456"), None),
            ("monto_adeudado", Some("$ 100.000"), None),
        ]
    );
    assert_eq!(found.evidence[3].fact_time.as_deref(), Some("a partir del 1 de marzo"));

    let mut reasons: Vec<(&str, &str)> = found.discarded.iter().map(|d| (d.what.as_str(), d.reason.as_str())).collect();
    reasons.sort();
    assert_eq!(
        reasons,
        [
            ("claim", "no-corresponde-al-tipo"),
            ("claim", "sujeto-desconocido"),
            ("claim", "valor-fuera-de-la-cita"),
            ("identifier", "valor-fuera-de-la-cita"),
            ("mention", "cita-no-encontrada"),
            ("mention", "nombre-fuera-de-la-cita"),
        ]
    );

    let status = base.knowledge_status(&Visible::All).unwrap();
    assert_eq!((status.documents, status.extracted, status.pending, status.failed), (2, 2, 0, 0));
    assert_eq!((status.mentions, status.identifiers, status.evidence, status.discarded), (3, 2, 4, 6));
    assert_eq!(status.discarded_by_reason["valor-fuera-de-la-cita"], 2);
    assert_eq!(status.answers, 2);
    // Nothing is seen through no origin.
    assert_eq!(base.knowledge_status(&Visible::Origins(vec![])).unwrap().documents, 0);
}

#[test]
fn reading_again_asks_nothing_and_changes_nothing() {
    let (_dir, base) = base_with(&[("contrato.txt", LEASE)]);
    let asker = model("falso:1");
    run(&base, &asker);
    let before = base.knowledge_of(&Visible::All, location(&base, "contrato.txt")).unwrap();

    let again = FakeModel::new("falso:1", |prompt| {
        assert!(!is_knowledge(prompt), "nothing should be read again");
        type_answer("contrato")
    });
    let report = run(&base, &again);
    assert_eq!((report.asked, report.extracted), (0, 0));
    assert_eq!(base.knowledge_of(&Visible::All, location(&base, "contrato.txt")).unwrap(), before);

    // The reading forgotten: the answer kept is used, not asked again.
    base.conn().execute("DELETE FROM knowledge_extraction", []).unwrap();
    let report = run(&base, &again);
    assert_eq!((report.asked, report.extracted), (0, 1));
    let after = base.knowledge_of(&Visible::All, location(&base, "contrato.txt")).unwrap();
    assert_eq!(after.mentions.len(), before.mentions.len());
    assert_eq!(after.evidence.len(), before.evidence.len());
}

#[test]
fn another_model_asks_again_but_the_reading_stays_until_the_text_changes() {
    let (dir, base) = base_with(&[("contrato.txt", LEASE)]);
    run(&base, &model("falso:1"));

    // Another model: the reading on the same text, question and
    // vocabulary is not redone (D-180)…
    let other = model("falso:2");
    assert_eq!(run(&base, &other).asked, 0);
    // …but a question it was never asked is not answered with the old one.
    base.conn().execute("DELETE FROM knowledge_extraction", []).unwrap();
    assert_eq!(run(&base, &other).asked, 1);
    assert_eq!(other.knowledge_calls(), 1);
    let engines: i64 = base
        .conn()
        .query_row("SELECT count(DISTINCT engine) FROM model_answer WHERE prompt_version = ?1", [KNOWLEDGE_PROMPT], |row| row.get(0))
        .unwrap();
    assert_eq!(engines, 2);

    // The text changes: a new document, read again; the old one goes and
    // leaves a mark to retire what it said.
    let old = base.knowledge_of(&Visible::All, location(&base, "contrato.txt")).unwrap().content_id.unwrap();
    dir.write("o/contrato.txt", format!("{LEASE}\nSe firman dos ejemplares.").as_bytes());
    let origin: String = base.conn().query_row("SELECT id FROM origin", [], |row| row.get(0)).unwrap();
    refresh(&base, &origin);
    let status = base.knowledge_status(&Visible::All).unwrap();
    assert_eq!(status.pending, 1);
    let report = run(&base, &other);
    assert_eq!((report.asked, report.extracted), (1, 1));
    let retire: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope = 'retire' AND ref = ?1", [old.to_string()], |row| row.get(0))
        .unwrap();
    assert_eq!(retire, 1);
    let left: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_mention WHERE content_id = ?1", [old], |row| row.get(0))
        .unwrap();
    assert_eq!(left, 0);
}

#[test]
fn a_document_waits_for_its_type_and_a_budget_leaves_the_rest() {
    let (_dir, base) = base_with(&[("contrato.txt", LEASE), ("receta.txt", RECIPE)]);
    let asker = model("falso:1");
    // No type asked yet: the reading waits for it.
    let report = base.knowledge_run(&asker, 10).unwrap();
    assert_eq!((report.asked, report.waiting_for_type), (0, 2));
    base.types_run(&asker, 50).unwrap();
    let report = base.knowledge_run(&asker, 1).unwrap();
    assert_eq!((report.asked, report.extracted, report.left), (1, 1, 1));
    let report = base.knowledge_run(&asker, 1).unwrap();
    assert_eq!((report.asked, report.extracted, report.left), (1, 1, 0));
}

#[test]
fn an_answer_of_another_shape_is_a_failure_and_not_retried() {
    let (_dir, base) = base_with(&[("contrato.txt", LEASE)]);
    let asker = FakeModel::new("falso:1", |prompt| {
        if is_knowledge(prompt) {
            serde_json::json!({ "otra": "cosa", "menciones": "no es una lista" }).to_string()
        } else {
            type_answer("contrato")
        }
    });
    let report = run(&base, &asker);
    assert_eq!((report.asked, report.unusable), (1, 1));
    let found = base.knowledge_of(&Visible::All, location(&base, "contrato.txt")).unwrap();
    assert_eq!(found.state, "failed");
    assert!(found.error.is_some());
    assert_eq!(base.knowledge_status(&Visible::All).unwrap().failed, 1);
    assert_eq!(run(&base, &asker).asked, 0, "the same answer would fail again");
}

#[test]
fn the_same_mention_in_two_parts_is_one() {
    use super::extract::{verify, Answer};
    use super::vocabulary::Vocabulary;
    let body = "## Page 1\nFirma Persona Dos, DNI 22.222.222.\n## Page 2\nFirma Persona Dos, en carácter de apoderada.";
    let part = |quote: &str, page: i64, same: Option<&str>| -> Answer {
        serde_json::from_value(serde_json::json!({
            "menciones": [
                {"ref": "x", "misma_que": null, "tipo": "persona", "escrito": "Persona Dos", "rol": "firmante",
                 "cita": quote, "pagina": page, "identificadores": []},
                {"ref": "y", "misma_que": same, "tipo": "persona", "escrito": "Persona Dos", "rol": "apoderado",
                 "cita": quote, "pagina": page, "identificadores": []}
            ],
            "afirmaciones": []
        }))
        .unwrap()
    };
    let reading = verify(
        &Vocabulary::builtin(),
        body,
        &[],
        &[part("Firma Persona Dos, DNI 22.222.222", 1, Some("x")), part("Firma Persona Dos, en carácter de apoderada", 2, None)],
    );
    let kept: Vec<(&str, &str, Option<i64>, Option<&str>)> = reading
        .mentions
        .iter()
        .map(|m| (m.reference.as_str(), m.role.as_str(), m.page, m.same_as.as_deref()))
        .collect();
    assert_eq!(kept, [("m1", "firmante", Some(1), None), ("m2", "apoderado", Some(1), Some("m1"))]);
}
