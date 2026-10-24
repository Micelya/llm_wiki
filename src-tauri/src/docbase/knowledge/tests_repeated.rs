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
const A2: &str = "DOC-A2. Autos: EJEMPLO, PERSONA UNO s/ sucesion. Expediente 1234/2020 EJEMPLO, PERSONA UNO s/ sucesion ab intestato, de Persona Uno Ejemplo, DNI 11.111.111.";
const A3: &str = "DOC-A3. Autos: EJEMPLO, PERSONA UNO s/ sucesion. En el expte. 1234/20 consta la defuncion de Persona Uno Ejemplo, DNI 11.111.111.";
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
            let other = "Expediente 1234/2020 EJEMPLO, PERSONA UNO s/ sucesion ab intestato";
            answer(
                vec![autos, mention("y", "asunto", other, "referido", other, serde_json::json!([])), person("Persona Uno Ejemplo, DNI 11.111.111")],
                vec![claim("p", "causante", Some("y"), None, "sucesion ab intestato, de Persona Uno Ejemplo", None)],
            )
        } else if prompt.contains("DOC-A3.") {
            answer(
                vec![
                    autos,
                    mention("z", "asunto", "expte. 1234/20", "referido", "En el expte. 1234/20", serde_json::json!([])),
                    person("Persona Uno Ejemplo, DNI 11.111.111"),
                ],
                vec![claim("p", "causante", Some("z"), None, "consta la defuncion de Persona Uno Ejemplo", None)],
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
    let section = text.split("## Lo que dicen los documentos del asunto\n\n").nth(1).unwrap_or_default();
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
    // write it, and with more words): one line; the number alone is not
    // known to be of it yet.
    let (text, lines) = facts_of_the_matter(&base);
    assert_eq!(lines.len(), 2, "{text}");
    assert!(lines.iter().any(|l| l.contains("Causante de: este asunto")), "{text}");
    assert!(lines.iter().any(|l| l.contains("Causante de: expte. 1234/20")), "{text}");
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
    let note = super::tests_pages::note_after(&text, "Causante de: este asunto");
    for written in ["«EJEMPLO, PERSONA UNO s/ sucesion»", "«Expediente 1234/2020 EJEMPLO, PERSONA UNO s/ sucesion ab intestato»", "«expte. 1234/20»"] {
        assert!(note.contains(written), "{note}");
    }
    assert!(note.contains("Escrito como"), "{note}");
}
