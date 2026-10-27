//! The resolution of identities over a real base, with invented
//! documents and a stand-in model (R-24: nothing here comes from the test
//! folder).

use std::collections::{BTreeMap, BTreeSet};

use super::decide::IdentityDecision;
use super::tests::{answer, base_with, claim, empty_answer, is_knowledge, mention, refresh, type_answer, FakeModel};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};

const A: &str = "DOC-A. Contrato entre Ejemplo Uno S.A., CUIT 30-11111111-1, y Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100.";
const B: &str = "DOC-B. Recibo firmado por DOS, Persona, DNI 22.222.222, a favor de Ejemplo Uno SA, con domicilio en Calle Uno 100.";
const C: &str = "DOC-C. Constancia de Persona Dos, DNI 44.444.444, vecina de otra ciudad, con domicilio en Calle Dos 200.";
const D: &str = "DOC-D. Nota de Persona Dos, DNI 22.222.223, sobre el mismo contrato y sus pagos pendientes.";
const E: &str = "DOC-E. Acuerdo entre Ejemplo Uno S.A. y Ejemplo Uno LLC, firmado por Persona Tres, soltera en 2020, en nombre de ambas sociedades.";
const F: &str = "DOC-F. Factura de EJEMPLO UNO para Persona Tres Cuatro, casada en 2024, por servicios prestados durante marzo.";

pub(crate) const FILES: &[(&str, &str)] = &[("a.txt", A), ("b.txt", B), ("c.txt", C), ("d.txt", D), ("e.txt", E), ("f.txt", F)];

fn ids(kind: &str, value: &str, quote: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": kind, "valor": value, "cita": quote }])
}

/// What a careful model reads in each document.
fn reading(prompt: &str) -> String {
    let none = serde_json::json!([]);
    let (mentions, claims) = if prompt.contains("DOC-A.") {
        (
            vec![
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A., CUIT 30-11111111-1", ids("cuit", "30-11111111-1", "CUIT 30-11111111-1")),
                mention("p", "persona", "Persona Dos", "parte", "Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
            ],
            vec![claim("p", "domicilio", None, Some("Calle Uno 100"), "Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100", None)],
        )
    } else if prompt.contains("DOC-B.") {
        (
            vec![
                mention("p", "persona", "DOS, Persona", "firmante", "DOS, Persona, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
                mention("o", "organizacion", "Ejemplo Uno SA", "destinatario", "a favor de Ejemplo Uno SA", none.clone()),
            ],
            vec![claim("p", "domicilio", None, Some("Calle Uno 100"), "a favor de Ejemplo Uno SA, con domicilio en Calle Uno 100", None)],
        )
    } else if prompt.contains("DOC-C.") {
        (
            vec![mention("p", "persona", "Persona Dos", "parte", "Constancia de Persona Dos, DNI 44.444.444", ids("dni", "44.444.444", "DNI 44.444.444"))],
            vec![claim("p", "domicilio", None, Some("Calle Dos 200"), "con domicilio en Calle Dos 200", None)],
        )
    } else if prompt.contains("DOC-D.") {
        (
            vec![mention("p", "persona", "Persona Dos", "remitente", "Nota de Persona Dos, DNI 22.222.223", ids("dni", "22.222.223", "DNI 22.222.223"))],
            vec![],
        )
    } else if prompt.contains("DOC-E.") {
        (
            vec![
                mention("s", "organizacion", "Ejemplo Uno S.A.", "parte", "Acuerdo entre Ejemplo Uno S.A.", none.clone()),
                mention("l", "organizacion", "Ejemplo Uno LLC", "parte", "y Ejemplo Uno LLC", none.clone()),
                mention("p", "persona", "Persona Tres", "firmante", "firmado por Persona Tres, soltera en 2020", none.clone()),
            ],
            vec![claim("p", "estado_civil", None, Some("soltera"), "Persona Tres, soltera en 2020", Some("en 2020"))],
        )
    } else if prompt.contains("DOC-F.") {
        (
            vec![
                mention("o", "organizacion", "EJEMPLO UNO", "emisor", "Factura de EJEMPLO UNO", none.clone()),
                mention("p", "persona", "Persona Tres Cuatro", "destinatario", "para Persona Tres Cuatro, casada en 2024", none.clone()),
            ],
            vec![claim("p", "estado_civil", None, Some("casada"), "Persona Tres Cuatro, casada en 2024", Some("en 2024"))],
        )
    } else {
        return empty_answer();
    };
    answer(mentions, claims)
}

fn is_identity(prompt: &str) -> bool {
    prompt.contains("Hay que decidir si cada candidato es la misma entidad")
}

/// The stand-in model's verdicts: a candidate named "Persona Tres" is the
/// same; anything else, doubtful.
fn verdicts(prompt: &str) -> String {
    let candidates = prompt.split("Candidatos:\n").nth(1).unwrap_or_default();
    let mut answers = Vec::new();
    for (index, block) in candidates.split("\n[").enumerate() {
        let names = block.lines().find(|l| l.starts_with("- Nombres:")).unwrap_or_default();
        let decision = if names.contains("Persona Tres") { "misma" } else { "dudosa" };
        answers.push(serde_json::json!({ "candidato": index + 1, "decision": decision, "certeza": "alta", "razon": "por el nombre" }));
    }
    serde_json::json!({ "respuestas": answers }).to_string()
}

pub(crate) fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_identity(prompt) {
            verdicts(prompt)
        } else if is_knowledge(prompt) {
            reading(prompt)
        } else {
            type_answer("contrato")
        }
    })
}

fn identity_calls(model: &FakeModel) -> usize {
    model.asked.lock().unwrap().iter().filter(|q| is_identity(q)).count()
}

/// Reads everything and resolves it.
pub(crate) fn run(base: &DocBase, asker: &FakeModel) {
    base.types_run(asker, 50).unwrap();
    while base.knowledge_run(asker, 50).unwrap().extracted > 0 {}
    base.knowledge_resolve(Some(asker), 50).unwrap();
}

/// The entities, as sets of (document, name written), and the doubts as
/// pairs of such sets: what does not depend on ids or the order of reading.
fn partition(base: &DocBase) -> (BTreeSet<BTreeSet<(String, String)>>, BTreeSet<(String, String)>) {
    let conn = base.conn();
    let mut statement = conn
        .prepare(
            "SELECT m.entity_id, l.name, m.written FROM knowledge_mention m
             JOIN file f ON f.version_id = m.version_id JOIN location l ON l.file_hash = f.hash WHERE m.entity_id IS NOT NULL",
        )
        .unwrap();
    let rows: Vec<(String, String, String)> =
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap().collect::<Result<_, _>>().unwrap();
    let mut by_entity: BTreeMap<String, BTreeSet<(String, String)>> = BTreeMap::new();
    for (entity, document, written) in rows {
        by_entity.entry(entity).or_default().insert((document, written));
    }
    let mut statement = conn.prepare("SELECT entity_a, entity_b, reason FROM knowledge_doubt").unwrap();
    let doubts: Vec<(String, String, String)> =
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap().collect::<Result<_, _>>().unwrap();
    let name = |id: &str| format!("{:?}", by_entity.get(id));
    let doubts = doubts
        .into_iter()
        .map(|(a, b, reason)| {
            let (x, y) = (name(&a), name(&b));
            (if x <= y { format!("{x} / {y}") } else { format!("{y} / {x}") }, reason)
        })
        .collect();
    (by_entity.into_values().collect(), doubts)
}

/// The entity of the mention written so in this document.
fn entity(base: &DocBase, document: &str, written: &str) -> Option<String> {
    base.conn()
        .query_row(
            "SELECT m.entity_id FROM knowledge_mention m JOIN file f ON f.version_id = m.version_id
             JOIN location l ON l.file_hash = f.hash WHERE l.name = ?1 AND m.written = ?2",
            [document, written],
            |row| row.get(0),
        )
        .unwrap()
}

fn mention_id(base: &DocBase, document: &str, written: &str) -> i64 {
    base.conn()
        .query_row(
            "SELECT m.id FROM knowledge_mention m JOIN file f ON f.version_id = m.version_id
             JOIN location l ON l.file_hash = f.hash WHERE l.name = ?1 AND m.written = ?2",
            [document, written],
            |row| row.get(0),
        )
        .unwrap()
}

fn resolved_by(base: &DocBase, document: &str, written: &str) -> Option<String> {
    let id = mention_id(base, document, written);
    base.conn().query_row("SELECT resolved_by FROM knowledge_mention WHERE id = ?1", [id], |row| row.get(0)).unwrap()
}

#[test]
fn identifiers_names_and_the_model_in_cascade() {
    let (_dir, base) = base_with(FILES);
    let asker = model();
    run(&base, &asker);

    // The same DNI joins two ways of writing a name; another DNI keeps
    // the same name apart; one digit of difference is a doubt.
    let two = entity(&base, "a.txt", "Persona Dos").unwrap();
    assert_eq!(entity(&base, "b.txt", "DOS, Persona").as_ref(), Some(&two));
    assert_eq!(resolved_by(&base, "b.txt", "DOS, Persona").as_deref(), Some("identificador"));
    assert_ne!(entity(&base, "c.txt", "Persona Dos").as_ref(), Some(&two));
    assert_ne!(entity(&base, "d.txt", "Persona Dos").as_ref(), Some(&two));
    let (_, doubts) = partition(&base);
    assert!(doubts.iter().any(|(_, reason)| reason == "identificador-parecido"), "{doubts:?}");

    // The same name and legal form joins; another legal form never; the
    // name without a form, when two forms share it, is asked once against
    // both: the model cannot tell, so it is a doubt against each.
    let company = entity(&base, "a.txt", "Ejemplo Uno S.A.").unwrap();
    assert_eq!(entity(&base, "b.txt", "Ejemplo Uno SA").as_ref(), Some(&company));
    assert_eq!(entity(&base, "e.txt", "Ejemplo Uno S.A.").as_ref(), Some(&company));
    let other = entity(&base, "e.txt", "Ejemplo Uno LLC").unwrap();
    assert_ne!(other, company);
    let bare = entity(&base, "f.txt", "EJEMPLO UNO").unwrap();
    assert!(bare != company && bare != other);
    assert_eq!(doubts.iter().filter(|(_, reason)| reason == "sin-forma-varias-sociedades").count(), 2, "{doubts:?}");

    // A partial name: the model said it is the same.
    let three = entity(&base, "e.txt", "Persona Tres").unwrap();
    assert_eq!(entity(&base, "f.txt", "Persona Tres Cuatro").as_ref(), Some(&three));
    assert_eq!(resolved_by(&base, "f.txt", "Persona Tres Cuatro").as_deref(), Some("modelo"));
    let asked = identity_calls(&asker);
    assert_eq!(asked, 2, "one question about the partial name, one about the name without a form");

    // Resolving again asks nothing and changes nothing (R-13, R-20).
    let before = partition(&base);
    super::mark_dirty(&base.conn(), "resolve", "otra vez").unwrap();
    let again = base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!((again.asked, again.changed, again.left), (0, 0, 0), "{again:?}");
    assert_eq!(partition(&base), before);
    assert_eq!(identity_calls(&asker), asked);
    // Nothing to do: nothing is computed.
    assert!(base.knowledge_resolve(Some(&asker), 50).unwrap().skipped);
    let status = base.knowledge_status(&Visible::All).unwrap();
    assert_eq!(status.entities_by_kind["persona"], 4);
    assert_eq!(status.entities_by_kind["organizacion"], 3);
    assert!(status.doubts >= 2);
    assert_eq!(status.to_resolve, 0);
}

#[test]
fn a_verdict_said_to_another_version_of_the_question_is_asked_again() {
    let (_dir, base) = base_with(FILES);
    let asker = model();
    run(&base, &asker);
    let asked = identity_calls(&asker);
    let before = partition(&base);
    // The verdicts kept were said to an older version of the question.
    let conn = base.conn();
    conn.execute("UPDATE model_answer SET question = question || '-vieja', prompt_version = 'identidad-0' WHERE task = 'identity'", []).unwrap();
    conn.execute("UPDATE knowledge_verdict SET question = question || '-vieja'", []).unwrap();
    drop(conn);
    super::mark_dirty(&base.conn(), "resolve", "otra versión").unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!(identity_calls(&asker), 2 * asked, "each question is asked again, once");
    assert_eq!(partition(&base), before, "the same answers give the same entities");
    // And kept again: one more resolution asks nothing.
    super::mark_dirty(&base.conn(), "resolve", "otra vez").unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!(identity_calls(&asker), 2 * asked);
}

#[test]
fn the_question_says_another_position_is_not_another_person() {
    let (_dir, base) = base_with(FILES);
    let asker = model();
    run(&base, &asker);
    let questions: Vec<String> = asker.asked.lock().unwrap().iter().filter(|q| is_identity(q)).cloned().collect();
    assert!(questions.iter().all(|q| q.contains("otro cargo u otra organización no alcanza")), "{questions:?}");
}

#[test]
fn the_order_of_reading_does_not_change_the_entities() {
    let (_dir, all) = base_with(FILES);
    run(&all, &model());

    // The same documents, one at a time, the other way round.
    let dir = TempDir::new();
    let base = DocBase::in_memory().unwrap();
    std::fs::create_dir_all(dir.path().join("o")).unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    let asker = model();
    for (name, text) in FILES.iter().rev() {
        dir.write(&format!("o/{name}"), text.as_bytes());
        refresh(&base, &origin.id);
        run(&base, &asker);
    }
    assert_eq!(partition(&base), partition(&all));
}

#[test]
fn what_a_person_decides_is_firm_and_can_be_undone() {
    let (_dir, base) = base_with(FILES);
    let asker = model();
    run(&base, &asker);
    let two = entity(&base, "a.txt", "Persona Dos").unwrap();

    // Taking a mention out of its entity: it stays out on every pass.
    let out = base
        .knowledge_decide(
            &IdentityDecision::Separate { a: format!("mention:{}", mention_id(&base, "b.txt", "DOS, Persona")), b: format!("entity:{two}") },
            Some("no es la misma"),
            None,
        )
        .unwrap();
    assert_ne!(entity(&base, "b.txt", "DOS, Persona"), entity(&base, "a.txt", "Persona Dos"));
    assert_eq!(entity(&base, "a.txt", "Persona Dos").as_ref(), Some(&two), "the bigger part keeps the id");
    super::mark_dirty(&base.conn(), "resolve", "otra vez").unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_ne!(entity(&base, "b.txt", "DOS, Persona"), entity(&base, "a.txt", "Persona Dos"));
    // No doubt is raised about what a person already decided.
    let doubts = base.knowledge_doubts(&Visible::All).unwrap();
    assert!(!doubts.iter().any(|d| d.reason == "identificador-otro-nombre"));

    // Undone: joined again by the identifier.
    base.knowledge_undo(out).unwrap();
    assert_eq!(entity(&base, "b.txt", "DOS, Persona").as_ref(), Some(&two));
    assert!(base.knowledge_undo(out).is_err());

    // A person's join wins over another identifier.
    let other = entity(&base, "c.txt", "Persona Dos").unwrap();
    base.knowledge_decide(&IdentityDecision::Join { a: format!("entity:{other}"), b: format!("entity:{two}") }, None, None).unwrap();
    assert_eq!(entity(&base, "c.txt", "Persona Dos"), entity(&base, "a.txt", "Persona Dos"));
    assert_eq!(resolved_by(&base, "c.txt", "Persona Dos").as_deref(), Some("persona"));
    // Two domiciles for the same person now, with no time: two values, not
    // a conflict (a conflict needs the same time, D-252).
    let joined = entity(&base, "a.txt", "Persona Dos").unwrap();
    let detail = base.knowledge_entity(&Visible::All, &joined).unwrap().unwrap();
    let domiciles: Vec<(&str, usize)> =
        detail.claims.iter().filter(|c| c.predicate == "domicilio").map(|c| (c.state.as_str(), c.evidence.len())).collect();
    assert_eq!(domiciles.len(), 2, "{domiciles:?}");
    assert!(domiciles.iter().all(|(state, _)| *state == "supported"));
    assert!(domiciles.contains(&("supported", 2)), "the same domicile in two documents is one claim");

    // A mention marked wrong is no entity; a claim marked wrong is retired.
    base.knowledge_decide(&IdentityDecision::Wrong { a: format!("mention:{}", mention_id(&base, "d.txt", "Persona Dos")) }, None, None)
        .unwrap();
    assert_eq!(entity(&base, "d.txt", "Persona Dos"), None);
    assert_eq!(resolved_by(&base, "d.txt", "Persona Dos").as_deref(), Some("incorrecta"));
    let claim = detail.claims.iter().find(|c| c.predicate == "domicilio" && c.evidence.len() == 1).unwrap().id;
    base.knowledge_decide(&IdentityDecision::Wrong { a: format!("claim:{claim}") }, None, None).unwrap();
    let detail = base.knowledge_entity(&Visible::All, &joined).unwrap().unwrap();
    let states: BTreeSet<&str> = detail.claims.iter().filter(|c| c.predicate == "domicilio").map(|c| c.state.as_str()).collect();
    assert!(states.contains("retired"), "{states:?}");

    let decisions = base.knowledge_decisions().unwrap();
    assert_eq!(decisions.len(), 4);
    assert_eq!(decisions.iter().filter(|d| d.state == "undone").count(), 1);
    assert!(decisions.iter().all(|d| !d.shown_evidence.is_empty()));
    // Two different kinds are never decided together.
    let company = entity(&base, "a.txt", "Ejemplo Uno S.A.").unwrap();
    assert!(base.knowledge_decide(&IdentityDecision::Join { a: format!("entity:{company}"), b: format!("entity:{joined}") }, None, None).is_err());
}

#[test]
fn claims_with_their_own_time_are_a_history() {
    let (_dir, base) = base_with(FILES);
    run(&base, &model());
    let three = entity(&base, "e.txt", "Persona Tres").unwrap();
    let detail = base.knowledge_entity(&Visible::All, &three).unwrap().unwrap();
    let civil: BTreeSet<(&str, Option<&str>, &str)> = detail
        .claims
        .iter()
        .filter(|c| c.predicate == "estado_civil")
        .map(|c| (c.object_value.as_deref().unwrap_or_default(), c.valid_from.as_deref(), c.state.as_str()))
        .collect();
    assert_eq!(civil, BTreeSet::from([("soltera", Some("en 2020"), "supported"), ("casada", Some("en 2024"), "supported")]));
    // The page of the entity shows its documents, names and identifiers.
    let two = entity(&base, "a.txt", "Persona Dos").unwrap();
    let detail = base.knowledge_entity(&Visible::All, &two).unwrap().unwrap();
    assert_eq!(detail.mentions.len(), 2);
    assert!(detail.mentions.iter().all(|m| m.location_id.is_some()));
    assert_eq!(detail.identifiers.len(), 1);
    assert_eq!(detail.identifiers[0].documents, 2);
    let listed = base.knowledge_entities(&Visible::All, Some("persona"), Some("dos")).unwrap();
    assert!(listed.iter().any(|e| e.id == two && e.documents == 2));
    assert!(base.knowledge_entities(&Visible::Origins(vec![]), None, None).unwrap().is_empty());
}

/// The model names one of the companies a name without a form may be.
fn naming(company: &'static str) -> FakeModel {
    FakeModel::new("falso:1", move |prompt| {
        if is_identity(prompt) && prompt.contains("a lo sumo uno puede ser la misma") {
            let candidates = prompt.split("Candidatos:\n").nth(1).unwrap_or_default();
            let answers: Vec<serde_json::Value> = candidates
                .split("\n[")
                .enumerate()
                .map(|(index, block)| {
                    let names = block.lines().find(|l| l.starts_with("- Nombres:")).unwrap_or_default();
                    let decision = if names.contains(company) { "misma" } else { "distinta" };
                    serde_json::json!({ "candidato": index + 1, "decision": decision, "certeza": "media", "razon": "por el contexto" })
                })
                .collect();
            serde_json::json!({ "respuestas": answers }).to_string()
        } else if is_identity(prompt) {
            verdicts(prompt)
        } else if is_knowledge(prompt) {
            reading(prompt)
        } else {
            type_answer("contrato")
        }
    })
}

#[test]
fn a_name_without_a_form_joins_the_one_company_the_model_names() {
    let (_dir, base) = base_with(FILES);
    run(&base, &naming("LLC"));
    let other = entity(&base, "e.txt", "Ejemplo Uno LLC").unwrap();
    assert_eq!(entity(&base, "f.txt", "EJEMPLO UNO").as_ref(), Some(&other));
    let (_, doubts) = partition(&base);
    assert!(!doubts.iter().any(|(_, reason)| reason == "sin-forma-varias-sociedades"), "{doubts:?}");

    // Naming both is not knowing which: a doubt against each.
    let (_dir, base) = base_with(FILES);
    run(&base, &naming("Ejemplo Uno"));
    let bare = entity(&base, "f.txt", "EJEMPLO UNO").unwrap();
    assert_ne!(Some(&bare), entity(&base, "e.txt", "Ejemplo Uno LLC").as_ref());
    assert_ne!(Some(&bare), entity(&base, "a.txt", "Ejemplo Uno S.A.").as_ref());
    let (_, doubts) = partition(&base);
    assert_eq!(doubts.iter().filter(|(_, reason)| reason == "sin-forma-varias-sociedades").count(), 2, "{doubts:?}");
}

/// Two documents of one family: a person in each, the same surname and a
/// first name one letter apart, nothing else in common.
const G: &str = "DOC-G. Correo de Marina Ejemplar, del equipo de operaciones, sobre los pagos pendientes de marzo.";
const H: &str = "DOC-H. Anexo con la dotación del proyecto: MARIANA EJEMPLAR, DNI 33.333.333, analista.";

/// The model reads G and H and says the two are the same, as sure as told.
fn relatives(certainty: &'static str) -> FakeModel {
    FakeModel::new("falso:1", move |prompt| {
        if is_identity(prompt) {
            let candidates = prompt.split("Candidatos:\n").nth(1).unwrap_or_default();
            let answers: Vec<serde_json::Value> = candidates
                .split("\n[")
                .enumerate()
                .map(|(index, _)| {
                    serde_json::json!({ "candidato": index + 1, "decision": "misma", "certeza": certainty, "razon": "puede ser un error de escritura" })
                })
                .collect();
            serde_json::json!({ "respuestas": answers }).to_string()
        } else if is_knowledge(prompt) && prompt.contains("DOC-G.") {
            answer(vec![mention("p", "persona", "Marina Ejemplar", "remitente", "Correo de Marina Ejemplar", serde_json::json!([]))], vec![])
        } else if is_knowledge(prompt) && prompt.contains("DOC-H.") {
            answer(
                vec![mention("p", "persona", "MARIANA EJEMPLAR", "otro", "MARIANA EJEMPLAR, DNI 33.333.333", ids("dni", "33.333.333", "DNI 33.333.333"))],
                vec![],
            )
        } else if is_knowledge(prompt) {
            empty_answer()
        } else {
            type_answer("correo")
        }
    })
}

#[test]
fn people_whose_first_names_differ_by_a_letter_join_only_when_the_model_is_sure() {
    let files: &[(&str, &str)] = &[("g.txt", G), ("h.txt", H)];
    // Not sure: they may be relatives; a doubt, not one person.
    let (_dir, base) = base_with(files);
    run(&base, &relatives("media"));
    let one = entity(&base, "g.txt", "Marina Ejemplar").unwrap();
    assert_ne!(entity(&base, "h.txt", "MARIANA EJEMPLAR").as_ref(), Some(&one));
    let (_, doubts) = partition(&base);
    assert!(doubts.iter().any(|(_, reason)| reason == "modelo-misma-nombre-distinto"), "{doubts:?}");
    // Kept and resolved again: still apart, nothing asked.
    super::mark_dirty(&base.conn(), "resolve", "otra vez").unwrap();
    let again = base.knowledge_resolve(None, 50).unwrap();
    assert_eq!(again.asked, 0);
    assert_ne!(entity(&base, "h.txt", "MARIANA EJEMPLAR"), entity(&base, "g.txt", "Marina Ejemplar"));

    // Sure: one person.
    let (_dir, base) = base_with(files);
    run(&base, &relatives("alta"));
    assert_eq!(entity(&base, "h.txt", "MARIANA EJEMPLAR"), entity(&base, "g.txt", "Marina Ejemplar"));
    let by = [resolved_by(&base, "g.txt", "Marina Ejemplar"), resolved_by(&base, "h.txt", "MARIANA EJEMPLAR")];
    assert!(by.iter().any(|b| b.as_deref() == Some("modelo")), "{by:?}");
}
