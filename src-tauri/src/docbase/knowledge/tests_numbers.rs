//! The identity with the owner's rules, general for any identifier (E5 of
//! 2 nov, audit 1C): the number in its common form, with the reader's
//! confusions and the check digit; a near number joins only with a name
//! within the caps of the base's configuration; a number without its label
//! never keeps two apart. Invented names, and numbers written the ways the
//! scans of the test folder write them (R-24: no name or number of anyone).

use super::resolve::{resolve, write, Asking, ResolveReport};
use super::tests::{answer, base_with, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use super::tests_identity::run;
use crate::docbase::thresholds::Thresholds;
use crate::docbase::DocBase;

// A lawyer's registration, as a seal read by a scan writes it: the "°"
// read as a digit, a digit changed, a letter for a digit.
const A1: &str = "DOC-A1. Escrito firmado por el abogado Persona Uno Ejemplar, T° 12, F° 345, en representación de la parte actora.";
const A2: &str = "DOC-A2. Sello del letrado Persona Uno Ejemplar T212-F2345 C.S.J.N. al pie de la presentación.";
const A3: &str = "DOC-A3. Firma de Personna Uno Ejemplor, Tomo 12, Folio 348, abogado de la parte.";
const A4: &str = "DOC-A4. Nota de Persana Una Ejemplor, T21 2 - F3 3 4 s, con copia al expediente.";
// A booklet number read three ways: an "s" for a 5, a label misread.
const B1: &str = "DOC-B1. Partida de nacimiento. Padre: Persona Dos Ejemplar, Doc. Ident. LE 1234 565, argentino.";
const B2: &str = "DOC-B2. Declaración de Persona Dos Ejemplar, LE 1234 56s, sobre el inmueble.";
const B3: &str = "DOC-B3. Recibo de Persona Dos Ejemplar, ∠E 1234 565, por el alquiler.";
// A tax number right, and the same one with a digit misread (its check digit fails).
const C1: &str = "DOC-C1. Factura a Persona Tres Ejemplar, CUIT 20-12345678-6, por honorarios.";
const C2: &str = "DOC-C2. Recibo de Persona Tres Ejemplar, CUIT 20-12345618-6, por honorarios.";
// The same number given to two people: the mother read with the father's.
const D1: &str = "DOC-D1. Constancia de Persona Cuatro Ejemplar, DNI 33.333.333, domiciliado en Calle Uno 100.";
const D2: &str = "DOC-D2. Partida. Madre: Otra Gente Distinta, DNI 33.333.333, ama de casa.";
// The same name with two tax numbers right and different: two people.
const E1: &str = "DOC-E1. Contrato de Persona Cinco Ejemplar, CUIT 27-22222222-8, locataria.";
const E2: &str = "DOC-E2. Denuncia de Persona Cinco Ejemplar, CUIT 27-33333333-9, vecina.";
// A number without its label taken for a registration.
const F1: &str = "DOC-F1. Poder otorgado por Persona Seis Ejemplar, CUIT 20-23456789-7, matrícula T° 40 F° 500.";
const F2: &str = "DOC-F2. Comprobante de pago 0361705 de Persona Seis Ejemplar, CUIT 20-23456789-7.";

const FILES: &[(&str, &str)] = &[
    ("a1.txt", A1),
    ("a2.txt", A2),
    ("a3.txt", A3),
    ("a4.txt", A4),
    ("b1.txt", B1),
    ("b2.txt", B2),
    ("b3.txt", B3),
    ("c1.txt", C1),
    ("c2.txt", C2),
    ("d1.txt", D1),
    ("d2.txt", D2),
    ("e1.txt", E1),
    ("e2.txt", E2),
    ("f1.txt", F1),
    ("f2.txt", F2),
];

fn ids(list: &[(&str, &str, &str)]) -> serde_json::Value {
    serde_json::Value::Array(list.iter().map(|(kind, value, quote)| serde_json::json!({ "clase": kind, "valor": value, "cita": quote })).collect())
}

fn person(written: &str, quote: &str, list: &[(&str, &str, &str)]) -> String {
    answer(vec![mention("p", "persona", written, "parte", quote, ids(list))], vec![])
}

/// What a careful model reads in each document.
fn reading(prompt: &str) -> String {
    let said: &[(&str, &str, &str, &[(&str, &str, &str)])] = &[
        ("DOC-A1.", "Persona Uno Ejemplar", "abogado Persona Uno Ejemplar", &[("matricula", "T° 12, F° 345", "Persona Uno Ejemplar, T° 12, F° 345")]),
        ("DOC-A2.", "Persona Uno Ejemplar", "letrado Persona Uno Ejemplar", &[("matricula", "T212-F2345", "Persona Uno Ejemplar T212-F2345")]),
        ("DOC-A3.", "Personna Uno Ejemplor", "Firma de Personna Uno Ejemplor", &[("matricula", "Tomo 12, Folio 348", "Personna Uno Ejemplor, Tomo 12, Folio 348")]),
        ("DOC-A4.", "Persana Una Ejemplor", "Nota de Persana Una Ejemplor", &[("matricula", "T21 2 - F3 3 4 s", "Persana Una Ejemplor, T21 2 - F3 3 4 s")]),
        ("DOC-B1.", "Persona Dos Ejemplar", "Padre: Persona Dos Ejemplar", &[("dni", "LE 1234 565", "Persona Dos Ejemplar, Doc. Ident. LE 1234 565")]),
        ("DOC-B2.", "Persona Dos Ejemplar", "Declaración de Persona Dos Ejemplar", &[("dni", "LE 1234 56s", "Persona Dos Ejemplar, LE 1234 56s")]),
        ("DOC-B3.", "Persona Dos Ejemplar", "Recibo de Persona Dos Ejemplar", &[("otro", "∠E 1234 565", "Persona Dos Ejemplar, ∠E 1234 565")]),
        ("DOC-C1.", "Persona Tres Ejemplar", "Factura a Persona Tres Ejemplar", &[("cuit", "20-12345678-6", "Persona Tres Ejemplar, CUIT 20-12345678-6")]),
        ("DOC-C2.", "Persona Tres Ejemplar", "Recibo de Persona Tres Ejemplar", &[("cuit", "20-12345618-6", "Persona Tres Ejemplar, CUIT 20-12345618-6")]),
        ("DOC-D1.", "Persona Cuatro Ejemplar", "Constancia de Persona Cuatro Ejemplar", &[("dni", "33.333.333", "Persona Cuatro Ejemplar, DNI 33.333.333")]),
        ("DOC-D2.", "Otra Gente Distinta", "Madre: Otra Gente Distinta", &[("dni", "33.333.333", "Otra Gente Distinta, DNI 33.333.333")]),
        ("DOC-E1.", "Persona Cinco Ejemplar", "Contrato de Persona Cinco Ejemplar", &[("cuit", "27-22222222-8", "Persona Cinco Ejemplar, CUIT 27-22222222-8")]),
        ("DOC-E2.", "Persona Cinco Ejemplar", "Denuncia de Persona Cinco Ejemplar", &[("cuit", "27-33333333-9", "Persona Cinco Ejemplar, CUIT 27-33333333-9")]),
        (
            "DOC-F1.",
            "Persona Seis Ejemplar",
            "Poder otorgado por Persona Seis Ejemplar",
            &[("cuit", "20-23456789-7", "Persona Seis Ejemplar, CUIT 20-23456789-7"), ("matricula", "T° 40 F° 500", "matrícula T° 40 F° 500")],
        ),
        (
            "DOC-F2.",
            "Persona Seis Ejemplar",
            "de Persona Seis Ejemplar",
            &[("cuit", "20-23456789-7", "Persona Seis Ejemplar, CUIT 20-23456789-7"), ("matricula", "0361705", "Comprobante de pago 0361705")],
        ),
    ];
    for (mark, written, quote, list) in said {
        if prompt.contains(mark) {
            return person(written, quote, list);
        }
    }
    empty_answer()
}

fn is_identity(prompt: &str) -> bool {
    prompt.contains("Hay que decidir si cada candidato es la misma entidad")
}

/// The stand-in model reads, and never knows who is who.
fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_identity(prompt) {
            let candidates = prompt.split("Candidatos:\n").nth(1).unwrap_or_default();
            let answers: Vec<serde_json::Value> = candidates
                .split("\n[")
                .enumerate()
                .map(|(index, _)| serde_json::json!({ "candidato": index + 1, "decision": "dudosa", "certeza": "baja", "razon": "no se sabe" }))
                .collect();
            serde_json::json!({ "respuestas": answers }).to_string()
        } else if is_knowledge(prompt) {
            reading(prompt)
        } else {
            type_answer("contrato")
        }
    })
}

/// The entity of the mention of this document.
fn entity(base: &DocBase, document: &str) -> Option<String> {
    base.conn()
        .query_row(
            "SELECT m.entity_id FROM knowledge_mention m JOIN file f ON f.version_id = m.version_id
             JOIN location l ON l.file_hash = f.hash WHERE l.name = ?1",
            [document],
            |row| row.get(0),
        )
        .unwrap()
}

fn resolved_by(base: &DocBase, document: &str) -> Option<String> {
    base.conn()
        .query_row(
            "SELECT m.resolved_by FROM knowledge_mention m JOIN file f ON f.version_id = m.version_id
             JOIN location l ON l.file_hash = f.hash WHERE l.name = ?1",
            [document],
            |row| row.get(0),
        )
        .unwrap()
}

fn doubts(base: &DocBase) -> Vec<String> {
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT reason FROM knowledge_doubt ORDER BY reason").unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
}

fn identity_calls(model: &FakeModel) -> usize {
    model.asked.lock().unwrap().iter().filter(|q| is_identity(q)).count()
}

#[test]
fn a_number_written_otherwise_or_misread_joins_with_a_name_within_the_caps_and_not_beyond() {
    let (_dir, base) = base_with(FILES);
    let asker = model();
    run(&base, &asker);
    let same = |a: &str, b: &str| entity(&base, a).is_some() && entity(&base, a) == entity(&base, b);

    // The registration: the "°" read as a digit joins the same name; a
    // digit changed joins a name 2 letters away (of 19, 11 %).
    assert!(same("a1.txt", "a2.txt"), "the «°» read as a 2");
    assert_eq!(resolved_by(&base, "a2.txt").as_deref(), Some("identificador-parecido"));
    assert!(same("a1.txt", "a3.txt"), "a name 2 letters away");
    // A name 3 letters away does not join, though the number is near.
    assert!(!same("a1.txt", "a4.txt"), "a name 3 letters away");

    // The booklet: an "s" for a 5 is the same number; a label misread
    // ("∠E"), given as another number, is that DNI.
    assert!(same("b1.txt", "b2.txt"), "an s for a 5");
    assert!(same("b1.txt", "b3.txt"), "another number that is the DNI");
    assert_eq!(resolved_by(&base, "b2.txt").as_deref(), Some("identificador"));

    // The check digit: one right, one broken by a misread digit: near,
    // and the same name joins.
    assert!(same("c1.txt", "c2.txt"), "a tax number with a digit misread");
    // Two right and different: two people, though the same name.
    assert!(!same("e1.txt", "e2.txt"), "two tax numbers right and different");

    // The same number given to two people with names far apart: not one.
    assert!(!same("d1.txt", "d2.txt"), "the mother with the father's number");
    let reasons = doubts(&base);
    assert!(reasons.iter().any(|r| r == "identificador-otro-nombre"), "{reasons:?}");

    // A number without its label does not keep apart: the same tax number joins.
    assert!(same("f1.txt", "f2.txt"), "a number without its label taken for a registration");
    assert!(!reasons.iter().any(|r| r == "mismo-nombre-otro-identificador"), "{reasons:?}");
    // Nothing of this was asked to the model.
    assert_eq!(identity_calls(&asker), 0);
}

#[test]
fn the_pairs_whose_names_are_beyond_the_caps_are_listed_with_how_far() {
    let (_dir, base) = base_with(FILES);
    run(&base, &model());
    let mut report = ResolveReport::default();
    let mut asking = Asking { asker: None, engine: None, budget: 0, collect: None };
    let outcome = resolve(&base, &mut asking, &mut report).unwrap();
    let names = |i: usize| outcome.rows[outcome.clusters[i].principal].written.clone();
    let apart: Vec<(String, String, bool, Option<usize>)> =
        outcome.numbers_apart.iter().map(|p| (names(p.a), names(p.b), p.near, p.far.map(|f| f.0))).collect();
    assert!(
        apart.iter().any(|(a, b, near, far)| (a.contains("Persana") || b.contains("Persana")) && *near && *far == Some(3)),
        "the name 3 letters away: {apart:?}"
    );
    assert!(apart.iter().any(|(a, b, near, _)| (a.contains("Otra Gente") || b.contains("Otra Gente")) && !*near), "{apart:?}");
    write(&base, &outcome, &mut report).unwrap();
}

#[test]
fn the_caps_of_the_names_are_in_the_configuration_and_changing_them_computes_the_identity_again() {
    let (_dir, base) = base_with(FILES);
    let asker = model();
    let by_default = base.thresholds().unwrap();
    assert_eq!((by_default.name_letters, by_default.name_share), (2, 0.15));
    run(&base, &asker);
    assert_ne!(entity(&base, "a1.txt"), entity(&base, "a4.txt"));
    let asked = asker.asked.lock().unwrap().len();

    // 3 letters and 20 %: the name 3 letters away (16 %) joins, without
    // asking anything, and the configuration keeps them.
    base.set_thresholds(&Thresholds { name_letters: 3, name_share: 0.2, ..by_default.clone() }).unwrap();
    assert_eq!(base.thresholds().unwrap().name_letters, 3);
    let again = base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert!(!again.skipped, "a change of the caps is work to do");
    assert_eq!(entity(&base, "a1.txt"), entity(&base, "a4.txt"));
    assert_eq!(asker.asked.lock().unwrap().len(), asked);
    // A share beyond 1 is refused.
    assert!(base.set_thresholds(&Thresholds { name_share: 1.5, ..by_default }).is_err());
}

#[test]
fn an_identifier_read_only_in_approximate_text_gives_the_identity_nothing() {
    use super::extract::{verify, Answer};
    use super::vocabulary::Vocabulary;
    let body = "## Page 1\n\nPODER ESPECIAL\nComparece Persona Siete Ejemplar por derecho propio.\n\
[aproximado] Persona Siete Ejemplar, DNI 44.444.444\n";
    let said: Answer = serde_json::from_value(serde_json::json!({
        "menciones": [mention("a", "persona", "Persona Siete Ejemplar", "parte", "Comparece Persona Siete Ejemplar",
            ids(&[("dni", "44.444.444", "Persona Siete Ejemplar, DNI 44.444.444")]))]
    }))
    .unwrap();
    let reading = verify(&Vocabulary::builtin(), body, &[], &[said]);
    assert_eq!(reading.mentions.len(), 1, "{:?}", reading.discarded);
    assert!(reading.mentions[0].identifiers.is_empty(), "{:?}", reading.mentions[0].identifiers);
}
