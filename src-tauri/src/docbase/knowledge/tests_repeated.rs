//! What is said more than once said once (D-516): a marital status from a
//! closed list, a value without a class joined with the same one with a
//! class, with invented documents and a stand-in model (R-24: nothing here
//! comes from the test folder).

use super::tests::{answer, base_with, claim, is_knowledge, mention, type_answer, FakeModel};
use crate::docbase::DocBase;

fn dni(value: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": "dni", "valor": value, "cita": format!("DNI {value}") }])
}

/// (value, class) of the claims of a predicate, in order.
fn claims(base: &DocBase, predicate: &str) -> Vec<(String, String, String)> {
    let conn = base.conn();
    let mut statement = conn
        .prepare("SELECT object_value, coalesce(qualifier, ''), state FROM knowledge_claim WHERE predicate = ?1 AND state != 'retired' ORDER BY 1")
        .unwrap();
    let rows = statement.query_map([predicate], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn t(a: &str, b: &str, c: &str) -> (String, String, String) {
    (a.to_string(), b.to_string(), c.to_string())
}

const M1: &str = "DOC-M1. Persona Ocho, DNI 44.444.444, soltera y sin descendencia, con domicilio real en Calle Siete 700.";
const M2: &str = "DOC-M2. Comparece Persona Ocho, DNI 44.444.444, de estado civil soltero, domiciliada en Calle Siete 700.";
const M3: &str = "DOC-M3. Persona Ocho, DNI 44.444.444, quien dice ser de profesión docente y sin hijos.";

fn status_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let person = |quote: &str| mention("p", "persona", "Persona Ocho", "parte", quote, dni("44.444.444"));
        if prompt.contains("DOC-M1.") {
            let mut real = claim("p", "domicilio", None, Some("Calle Siete 700"), "con domicilio real en Calle Siete 700", None);
            real["clase"] = "real".into();
            answer(
                vec![person("Persona Ocho, DNI 44.444.444")],
                vec![
                    // What the model split in two: only the first is a
                    // marital status.
                    claim("p", "estado_civil", None, Some("soltera"), "Persona Ocho, DNI 44.444.444, soltera y sin descendencia", None),
                    claim("p", "estado_civil", None, Some("sin descendencia"), "Persona Ocho, DNI 44.444.444, soltera y sin descendencia", None),
                    real,
                ],
            )
        } else if prompt.contains("DOC-M2.") {
            answer(
                vec![person("Comparece Persona Ocho, DNI 44.444.444")],
                vec![
                    claim("p", "estado_civil", None, Some("soltero"), "Persona Ocho, DNI 44.444.444, de estado civil soltero", None),
                    // The same domicile, without its class.
                    claim("p", "domicilio", None, Some("Calle Siete 700"), "domiciliada en Calle Siete 700", None),
                ],
            )
        } else if prompt.contains("DOC-M3.") {
            answer(
                vec![person("Persona Ocho, DNI 44.444.444")],
                vec![claim("p", "estado_civil", None, Some("sin hijos"), "docente y sin hijos", None)],
            )
        } else {
            answer(vec![], vec![])
        }
    })
}

#[test]
fn a_marital_status_is_one_of_its_list_and_a_value_without_class_joins_the_one_with_it() {
    let (_dir, base) = base_with(&[("m1.txt", M1), ("m2.txt", M2), ("m3.txt", M3)]);
    let model = status_model();
    base.types_run(&model, 50).unwrap();
    base.knowledge_run(&model, 50).unwrap();
    let report = base.knowledge_resolve(None, 0).unwrap();
    // «Soltera» and «soltero» are one value of the list, not a conflict;
    // «sin descendencia» and «sin hijos» are not a marital status.
    assert_eq!(claims(&base, "estado_civil"), [t("soltera", "", "supported")]);
    assert_eq!(report.conflicts, 0, "{report:?}");
    // The domicile said without its class is the one said with it.
    assert_eq!(claims(&base, "domicilio"), [t("Calle Siete 700", "real", "supported")]);
}

const A1: &str = "DOC-A1. Autos: EJEMPLO, PERSONA UNO s/ sucesion. Se informa el fallecimiento de Persona Uno Ejemplo, DNI 11.111.111.";
const A2: &str = "DOC-A2. Autos: EJEMPLO, PERSONA UNO s/ sucesion. Expediente 1234/2020 EJEMPLO, PERSONA UNO s/ sucesion y otros, de Persona Uno Ejemplo, DNI 11.111.111.";
const A3: &str = "DOC-A3. Autos: EJEMPLO, PERSONA UNO s/ sucesion. En el expte. 1234/20 consta la muerte de Persona Uno Ejemplo, DNI 11.111.111.";
const F1: &str = "DOC-F1. Nota de Persona Diez Otro sobre la entrega de llaves.";
const F2: &str = "DOC-F2. Nota de Persona Once Otro sobre la entrega de planos.";
const F3: &str = "DOC-F3. Nota de Persona Doce Otro sobre la entrega de fotos.";

fn matter_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let autos = mention("x", "asunto", "EJEMPLO, PERSONA UNO s/ sucesion", "referido", "Autos: EJEMPLO, PERSONA UNO s/ sucesion", serde_json::json!([]));
        let person = |quote: &str| mention("p", "persona", "Persona Uno Ejemplo", "otro", quote, dni("11.111.111"));
        if prompt.contains("DOC-A1.") {
            answer(
                vec![autos, person("Persona Uno Ejemplo, DNI 11.111.111")],
                vec![claim("p", "causante", Some("x"), None, "el fallecimiento de Persona Uno Ejemplo", None)],
            )
        } else if prompt.contains("DOC-A2.") {
            let other = "Expediente 1234/2020 EJEMPLO, PERSONA UNO s/ sucesion y otros";
            answer(
                vec![autos, mention("y", "asunto", other, "referido", other, serde_json::json!([])), person("Persona Uno Ejemplo, DNI 11.111.111")],
                vec![claim("p", "causante", Some("y"), None, "sucesion y otros, de Persona Uno Ejemplo", None)],
            )
        } else if prompt.contains("DOC-A3.") {
            answer(
                vec![
                    autos,
                    mention("z", "asunto", "expte. 1234/20", "referido", "En el expte. 1234/20", serde_json::json!([])),
                    person("Persona Uno Ejemplo, DNI 11.111.111"),
                ],
                vec![claim("p", "causante", Some("z"), None, "consta la muerte de Persona Uno Ejemplo", None)],
            )
        } else if let Some(name) = ["Persona Diez Otro", "Persona Once Otro", "Persona Doce Otro"].iter().find(|n| prompt.contains(**n)) {
            answer(vec![mention("o", "persona", name, "emisor", &format!("Nota de {name}"), serde_json::json!([]))], vec![])
        } else {
            answer(vec![], vec![])
        }
    })
}

/// The lines of what the documents of a matter say, on its page.
fn facts_of_the_matter(base: &DocBase) -> (String, Vec<String>) {
    let pages = super::tests_pages::entity_pages(base);
    let (_, text) = pages.iter().find(|(_, text)| text.contains("entity_kind: asunto")).expect("the page of the matter");
    let section = text.split("## Lo que dicen los documentos del caso\n\n").nth(1).unwrap_or_default();
    let lines = section.lines().take_while(|l| l.starts_with("- ")).map(str::to_string).collect();
    (text.clone(), lines)
}

#[test]
fn what_points_to_its_own_matter_is_said_once_on_its_page() {
    let (dir, base) = base_with(&[("a1.txt", A1), ("a2.txt", A2), ("a3.txt", A3), ("f1.txt", F1), ("f2.txt", F2), ("f3.txt", F3)]);
    dir.folder("General/wiki/sources");
    crate::docbase::testing::point_general_project_at(&base, &dir.path().join("General"));
    let model = matter_model();
    super::tests_pages::read(&base, &model);
    base.knowledge_project(None, 0).unwrap();
    // Two ways of writing the matter itself (as most of its documents
    // write it, and with more words), and its number written another way:
    // one line. The number is of it because a document writes it with the
    // words of the matter (P-330): the two ways of writing the number are
    // one claim of the person.
    let (text, lines) = facts_of_the_matter(&base);
    assert_eq!(lines.len(), 1, "{text}");
    assert!(lines.iter().any(|l| l.contains("Causante de: este caso")), "{text}");
    assert_eq!(claims(&base, "causante").len(), 2, "{text}");
    // With its record, the number is of it too: said once, every way of
    // writing it in the note.
    let matter: String = base.conn().query_row("SELECT id FROM knowledge_matter WHERE state = 'active'", [], |r| r.get(0)).unwrap();
    base.conn()
        .execute(
            "INSERT INTO knowledge_matter_record (matter_id, portal, jurisdiction, number, year, title, source, added_at)
             VALUES (?1, 'portal', 'ABC', 1234, 2020, NULL, 'integration', 1)",
            [&matter],
        )
        .unwrap();
    super::mark_dirty(&base.conn(), "project", "matters").unwrap();
    base.knowledge_project(None, 0).unwrap();
    let (text, lines) = facts_of_the_matter(&base);
    assert_eq!(lines.len(), 1, "{text}");
    let note = super::tests_pages::note_after(&text, "Causante de: este caso");
    for written in ["«EJEMPLO, PERSONA UNO s/ sucesion»", "«Expediente 1234/2020 EJEMPLO, PERSONA UNO s/ sucesion y otros»", "«expte. 1234/20»"] {
        assert!(note.contains(written), "{note}");
    }
    assert!(note.contains("Escrito como"), "{note}");
}

#[test]
fn a_person_is_named_as_most_of_its_documents_write_it_and_an_organization_as_before() {
    use super::names::{chosen_name, ORGANIZATION, PERSON};
    let v = super::vocabulary::Vocabulary::builtin();
    // Written in four documents, one of them with a word more.
    let written_in = [("Ejemplo Uno", 1), ("Ejemplo Uno", 2), ("Ejemplo Uno", 3), ("Ejemplo Uno Diez", 4)];
    let forms = [("Ejemplo Uno", 3), ("Ejemplo Uno Diez", 1)];
    let candidates = [("ejemplo uno", "Ejemplo Uno", 3), ("ejemplo uno diez", "Ejemplo Uno Diez", 1)];
    assert_eq!(chosen_name(&v, PERSON, &candidates, &forms, &written_in), Some("ejemplo uno"));
    // An organization keeps its most complete name (D-259).
    assert_eq!(chosen_name(&v, ORGANIZATION, &candidates, &forms, &written_in), Some("ejemplo uno diez"));
    // In two documents, one each: nothing is few yet.
    assert_eq!(chosen_name(&v, PERSON, &candidates, &forms, &[("Ejemplo Uno", 1), ("Ejemplo Uno Diez", 2)]), Some("ejemplo uno diez"));
}

const N1: &str = "DOC-N1. Comparece Persona Uno Dos Tres, DNI 55.555.555, por derecho propio.";
const N2: &str = "DOC-N2. Recibo firmado por Persona Uno Dos Tres, DNI 55.555.555.";
const N3: &str = "DOC-N3. Nota de Persona Uno Tres, DNI 55.555.555, sobre el pago.";
const N4: &str = "DOC-N4. Acta: PERSONA UNO DOS TRES Y CUATRO, DNI 55.555.555, presente.";

fn names_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let said = [
            ("DOC-N1.", "Persona Uno Dos Tres", "Comparece Persona Uno Dos Tres, DNI 55.555.555"),
            ("DOC-N2.", "Persona Uno Dos Tres", "firmado por Persona Uno Dos Tres, DNI 55.555.555"),
            ("DOC-N3.", "Persona Uno Tres", "Nota de Persona Uno Tres, DNI 55.555.555"),
            ("DOC-N4.", "PERSONA UNO DOS TRES Y CUATRO", "Acta: PERSONA UNO DOS TRES Y CUATRO, DNI 55.555.555"),
        ];
        match said.iter().find(|(mark, ..)| prompt.contains(*mark)) {
            Some((_, written, quote)) => answer(vec![mention("p", "persona", written, "parte", quote, dni("55.555.555"))], vec![]),
            None => answer(vec![], vec![]),
        }
    })
}

fn people(base: &DocBase) -> Vec<(String, String)> {
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT id, name FROM knowledge_entity WHERE kind = 'persona' AND state = 'active' ORDER BY id").unwrap();
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

#[test]
fn a_person_s_name_is_what_most_documents_write_and_a_person_can_choose_it_and_undo_it() {
    use super::decide::IdentityDecision;
    let (_dir, base) = base_with(&[("n1.txt", N1), ("n2.txt", N2), ("n3.txt", N3), ("n4.txt", N4)]);
    let model = names_model();
    base.types_run(&model, 50).unwrap();
    base.knowledge_run(&model, 50).unwrap();
    base.knowledge_resolve(None, 0).unwrap();
    let found = people(&base);
    assert_eq!(found.len(), 1, "{found:?}");
    let (id, name) = found[0].clone();
    // The surname of one document is not added; the whole form stays among
    // the ways it is written.
    assert_eq!(name, "Persona Uno Dos Tres");
    let aliases: Vec<String> = base.knowledge_entity(&crate::docbase::Visible::All, &id).unwrap().unwrap().aliases.into_iter().map(|a| a.written).collect();
    assert!(aliases.iter().any(|a| a == "PERSONA UNO DOS TRES Y CUATRO"), "{aliases:?}");
    // A person chooses another one, written by hand: it stands, also when
    // the archive is resolved again.
    let decision = base.knowledge_decide(&IdentityDecision::Name { a: format!("entity:{id}"), name: " Persona  Uno Tres ".into() }, None, None).unwrap();
    assert_eq!(people(&base), [(id.clone(), "Persona Uno Tres".to_string())]);
    super::mark_dirty(&base.conn(), "resolve", "test").unwrap();
    base.knowledge_resolve(None, 0).unwrap();
    assert_eq!(people(&base), [(id.clone(), "Persona Uno Tres".to_string())]);
    let decisions = base.knowledge_entity(&crate::docbase::Visible::All, &id).unwrap().unwrap().decisions;
    assert!(decisions.iter().any(|d| d.kind == "name" && d.b.as_deref() == Some("name:Persona Uno Tres")), "{decisions:?}");
    // Undone, the documents name it again.
    base.knowledge_undo(decision).unwrap();
    assert_eq!(people(&base), [(id, "Persona Uno Dos Tres".to_string())]);
    assert!(base.knowledge_decide(&IdentityDecision::Name { a: "entity:nadie".into(), name: "  ".into() }, None, None).is_err());
}

const B1: &str = "DOC-B1. Autos: MODELO, PERSONA DOS s/ sucesion. Se cita el Expte. 5678/2019 MODELO, PERSONA TRES s/ sucesion, de Persona Tres Modelo, DNI 22.222.222.";
const B2: &str = "DOC-B2. Autos: MODELO, PERSONA DOS s/ sucesion. En el expediente N° 5.678/19 consta la muerte de Persona Tres Modelo, DNI 22.222.222.";
const B3: &str = "DOC-B3. Autos: MODELO, PERSONA DOS s/ sucesion. Se agrega la partida del 12/05/2019 de Persona Tres Modelo, DNI 22.222.222.";

fn other_record_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let autos = mention("x", "asunto", "MODELO, PERSONA DOS s/ sucesion", "referido", "Autos: MODELO, PERSONA DOS s/ sucesion", serde_json::json!([]));
        let person = |quote: &str| mention("p", "persona", "Persona Tres Modelo", "otro", quote, dni("22.222.222"));
        let cited = |written: &str, quote: &str, claim_quote: &str| {
            answer(
                vec![autos.clone(), mention("y", "asunto", written, "otro", quote, serde_json::json!([])), person("Persona Tres Modelo, DNI 22.222.222")],
                vec![claim("p", "causante", Some("y"), None, claim_quote, None)],
            )
        };
        if prompt.contains("DOC-B1.") {
            cited("Expte. 5678/2019 MODELO, PERSONA TRES s/ sucesion", "el Expte. 5678/2019 MODELO, PERSONA TRES s/ sucesion", "sucesion, de Persona Tres Modelo")
        } else if prompt.contains("DOC-B2.") {
            cited("expediente N° 5.678/19", "En el expediente N° 5.678/19", "consta la muerte de Persona Tres Modelo")
        } else if prompt.contains("DOC-B3.") {
            cited("partida del 12/05/2019", "la partida del 12/05/2019", "partida del 12/05/2019 de Persona Tres Modelo")
        } else if let Some(name) = ["Persona Diez Otro", "Persona Once Otro", "Persona Doce Otro"].iter().find(|n| prompt.contains(**n)) {
            answer(vec![mention("o", "persona", name, "emisor", &format!("Nota de {name}"), serde_json::json!([]))], vec![])
        } else {
            answer(vec![], vec![])
        }
    })
}

#[test]
fn another_record_written_several_ways_is_said_once_by_number_and_year() {
    let (dir, base) = base_with(&[("b1.txt", B1), ("b2.txt", B2), ("b3.txt", B3), ("f1.txt", F1), ("f2.txt", F2), ("f3.txt", F3)]);
    dir.folder("General/wiki/sources");
    crate::docbase::testing::point_general_project_at(&base, &dir.path().join("General"));
    let model = other_record_model();
    super::tests_pages::read(&base, &model);
    base.knowledge_project(None, 0).unwrap();
    // The record of another succession, written two ways: one claim, shown
    // with its fullest writing and every quote; a date is not a record.
    let values: Vec<String> = claims(&base, "causante").into_iter().map(|(value, ..)| value).collect();
    assert_eq!(values, ["Expte. 5678/2019 MODELO, PERSONA TRES s/ sucesion", "partida del 12/05/2019"]);
    let (text, lines) = facts_of_the_matter(&base);
    assert_eq!(lines.len(), 2, "{text}");
    let note = super::tests_pages::note_after(&text, "Causante de: Expte. 5678/2019");
    assert!(note.contains("b1.txt") && note.contains("b2.txt"), "{note}");
}

#[test]
fn a_record_is_cited_by_number_and_year_however_it_is_written() {
    use super::references::{record_cited, same_record};
    assert_eq!(record_cited("Expte. N° 1.234/2020 EJEMPLO s/ sucesion"), Some((1234, 2020)));
    assert_eq!(record_cited("expediente 1234-20"), Some((1234, 20)));
    assert_eq!(record_cited("causa 001234 / 2020"), Some((1234, 2020)));
    // A date, an incident, two records or none: no record.
    assert_eq!(record_cited("partida del 12/05/2020"), None);
    assert_eq!(record_cited("expte. 1234/2020/1"), None);
    assert_eq!(record_cited("expte. 1234/2020 y 5678/2019"), None);
    assert_eq!(record_cited("EJEMPLO s/ sucesion"), None);
    assert_eq!(record_cited("Ley 1.23/2020"), None);
    assert!(same_record((1234, 2020), (1234, 20)) && !same_record((1234, 2020), (1234, 2021)) && !same_record((1234, 2020), (1235, 2020)));
}
