//! A claim counts only when its quote says it, beyond being in the text:
//! over a real base, with an invented document and a stand-in model (R-24:
//! nothing here comes from the test folder).

use super::tests::{base_with, claim, is_knowledge, mention, type_answer, FakeModel};

const N: &str = "DOC-N. Nota de Ejemplo Uno S.A., CUIT 30-11111111-1, firmada por Persona Dos, DNI 22.222.222, su apoderada. \
Fecha: 01.02.2020. Persona Dos adeuda a Ejemplo Uno S.A. la suma de $ 5.000. TOTAL Percepciones 1.234,56. Persona Tres \
vive en otra ciudad. Correo: persona.dos@ejemplo.com. Con efecto a partir del 1 de marzo de 2020 Persona Dos deja su cargo.";

fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let ids = |kind: &str, v: &str, q: &str| serde_json::json!([{ "clase": kind, "valor": v, "cita": q }]);
        serde_json::json!({
            "menciones": [
                mention("o", "organizacion", "Ejemplo Uno S.A.", "remitente", "Nota de Ejemplo Uno S.A., CUIT 30-11111111-1", ids("cuit", "30-11111111-1", "CUIT 30-11111111-1")),
                mention("p", "persona", "Persona Dos", "firmante", "firmada por Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
                mention("t", "persona", "Persona Tres", "otro", "Persona Tres vive en otra ciudad", serde_json::json!([])),
            ],
            "afirmaciones": [
                // Said by their quotes: kept.
                claim("p", "representa_a", Some("o"), None, "firmada por Persona Dos, DNI 22.222.222, su apoderada", None),
                claim("p", "deudor_de", Some("o"), None, "Persona Dos adeuda a Ejemplo Uno S.A. la suma de $ 5.000", None),
                claim("p", "monto_adeudado", None, Some("$ 5.000"), "Persona Dos adeuda a Ejemplo Uno S.A. la suma de $ 5.000", None),
                claim("p", "contacto", None, Some("persona.dos@ejemplo.com"), "Correo: persona.dos@ejemplo.com", None),
                claim("p", "fin_de_vinculo", None, Some("1 de marzo de 2020"), "Con efecto a partir del 1 de marzo de 2020", None),
                // In the text, but their quotes do not say them: dropped.
                claim("t", "representa_a", Some("o"), None, "vive en otra ciudad", None),
                claim("o", "acreedor_de", Some("t"), None, "Persona Tres vive en otra ciudad", None),
                claim("o", "monto_reclamado", None, Some("1.234,56"), "TOTAL Percepciones 1.234,56", None),
                claim("p", "contacto", None, Some("22.222.222"), "DNI 22.222.222", None),
                claim("o", "inicio_de_vinculo", None, Some("01.02.2020"), "Fecha: 01.02.2020", None),
            ]
        })
        .to_string()
    })
}

#[test]
fn a_claim_counts_only_when_its_quote_says_it() {
    let (_dir, base) = base_with(&[("n.txt", N)]);
    let asker = model();
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    let report = base.knowledge_resolve(Some(&asker), 50).unwrap();
    let conn = base.conn();
    let mut statement = conn
        .prepare(
            "SELECT e.name || ' ' || c.predicate || ' ' || coalesce(c.object_value, o.name) FROM knowledge_claim c
             JOIN knowledge_entity e ON e.id = c.subject LEFT JOIN knowledge_entity o ON o.id = c.object_entity
             WHERE c.state != 'retired' ORDER BY 1",
        )
        .unwrap();
    let claims: Vec<String> = statement.query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(
        claims,
        [
            "Persona Dos contacto persona.dos@ejemplo.com",
            "Persona Dos deudor_de Ejemplo Uno S.A.",
            "Persona Dos fin_de_vinculo 1 de marzo de 2020",
            "Persona Dos monto_adeudado $ 5.000",
            "Persona Dos representa_a Ejemplo Uno S.A.",
        ],
        "{report:?}"
    );
    // What was dropped stays in the reading, counted as not fit.
    let evidence: i64 = conn.query_row("SELECT count(*) FROM knowledge_evidence", [], |row| row.get(0)).unwrap();
    assert_eq!(evidence, 10);
    assert!(report.unfit >= 5, "{report:?}");
}

#[test]
fn other_rules_compute_the_record_again_without_asking() {
    let (_dir, base) = base_with(&[("n.txt", N)]);
    let asker = model();
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    assert!(!base.knowledge_resolve(Some(&asker), 50).unwrap().skipped);
    assert!(base.knowledge_resolve(Some(&asker), 50).unwrap().skipped, "nothing changed");
    // The record was computed with other rules: it is computed again.
    base.conn().execute("UPDATE docbase_setting SET value = 'otras' WHERE key = 'knowledge-resolution'", []).unwrap();
    let asked_before = asker.asked.lock().unwrap().len();
    let again = base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert!(!again.skipped, "{again:?}");
    assert_eq!(asker.asked.lock().unwrap().len(), asked_before, "nothing more is asked");
    assert!(base.knowledge_resolve(Some(&asker), 50).unwrap().skipped);
}
