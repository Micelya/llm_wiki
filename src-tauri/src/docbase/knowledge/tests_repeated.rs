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
