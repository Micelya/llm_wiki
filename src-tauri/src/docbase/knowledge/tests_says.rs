//! A claim counts only when its quote says it, beyond being in the text:
//! over a real base, with an invented document and a stand-in model (R-24:
//! nothing here comes from the test folder).

use super::tests::{amount, answer, base_with, claim, is_knowledge, mention, type_answer, FakeModel};

const N: &str = "DOC-N. Nota de Ejemplo Uno S.A., CUIT 30-11111111-1, firmada por Persona Dos, DNI 22.222.222, su apoderada. \
Fecha: 01.02.2020. Persona Dos adeuda a Ejemplo Uno S.A. la suma de $ 5.000. TOTAL Percepciones 1.234,56. Persona Tres \
vive en otra ciudad. Correo: persona.dos@ejemplo.com. Con efecto a partir del 1 de marzo de 2020 Persona Dos deja su cargo.";

fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let ids = |kind: &str, v: &str, q: &str| serde_json::json!([{ "clase": kind, "valor": v, "cita": q }]);
        answer(
            vec![
                mention("o", "organizacion", "Ejemplo Uno S.A.", "remitente", "Nota de Ejemplo Uno S.A., CUIT 30-11111111-1", ids("cuit", "30-11111111-1", "CUIT 30-11111111-1")),
                mention("p", "persona", "Persona Dos", "firmante", "firmada por Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
                mention("t", "persona", "Persona Tres", "otro", "Persona Tres vive en otra ciudad", serde_json::json!([])),
            ],
            vec![
                // Said by their quotes: kept.
                claim("p", "representa_a", Some("o"), None, "firmada por Persona Dos, DNI 22.222.222, su apoderada", None),
                claim("p", "deudor_de", Some("o"), None, "Persona Dos adeuda a Ejemplo Uno S.A. la suma de $ 5.000", None),
                amount("p", "o", "$ 5.000", "saldo", "Persona Dos adeuda a Ejemplo Uno S.A. la suma de $ 5.000", None),
                claim("p", "contacto", None, Some("persona.dos@ejemplo.com"), "Correo: persona.dos@ejemplo.com", None),
                claim("p", "fin_de_vinculo", None, Some("1 de marzo de 2020"), "Con efecto a partir del 1 de marzo de 2020", None),
                // In the text, but their quotes do not say them: dropped.
                claim("t", "representa_a", Some("o"), None, "vive en otra ciudad", None),
                claim("o", "deudor_de", Some("t"), None, "Persona Tres vive en otra ciudad", None),
                amount("t", "o", "1.234,56", "reclamo", "TOTAL Percepciones 1.234,56", None),
                claim("p", "contacto", None, Some("22.222.222"), "DNI 22.222.222", None),
                claim("p", "contacto", None, Some("DNI 22.222.222"), "DNI 22.222.222", None),
                claim("o", "inicio_de_vinculo", None, Some("01.02.2020"), "Fecha: 01.02.2020", None),
            ]
        )
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
            "Persona Dos importe Ejemplo Uno S.A.",
            // "Nota de Ejemplo Uno S.A., ..., firmada por Persona Dos, ...,
            // su apoderada": the company is the one named right before
            // (P-122, D-269).
            "Persona Dos representa_a Ejemplo Uno S.A.",
        ],
        "{report:?}"
    );
    // What was dropped stays in the reading, counted as not fit. A relation
    // with legal consequences whose quote neither names whom nor says how
    // ("vive en otra ciudad") and an amount next to the word of a tax do
    // not even get there: the reading drops them (D-244, D-250).
    let evidence: i64 = conn.query_row("SELECT count(*) FROM knowledge_evidence", [], |row| row.get(0)).unwrap();
    assert_eq!(evidence, 9);
    assert!(report.unfit >= 4, "{report:?}");
    let dropped: String = conn.query_row("SELECT discarded FROM knowledge_extraction", [], |row| row.get(0)).unwrap();
    assert!(dropped.contains("objeto-fuera-de-la-cita") && dropped.contains("es-un-impuesto"), "{dropped}");
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

const S1: &str = "DOC-S1. Contrato de Ejemplo Uno LLC, con domicilio en 100 Ejemplo Ave, Suite 555-B, Ciudad Ejemplo, ZZ 99999-1234, \
representada por Persona Dos.";
const S2: &str = "DOC-S2. Acuerdo de Ejemplo Uno LLC, registration number 7654321, firmado por Persona Dos.";

fn address_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let ids = |list: &[(&str, &str, &str)]| {
            serde_json::Value::Array(list.iter().map(|(k, v, q)| serde_json::json!({ "clase": k, "valor": v, "cita": q })).collect())
        };
        if prompt.contains("DOC-S1.") {
            serde_json::json!({
                "menciones": [mention("o", "organizacion", "Ejemplo Uno LLC", "parte", "Contrato de Ejemplo Uno LLC",
                    ids(&[("matricula", "555-B", "Suite 555-B"), ("otro", "99999-1234", "ZZ 99999-1234")]))],
                "afirmaciones": [claim("o", "domicilio", None, Some("100 Ejemplo Ave, Suite 555-B, Ciudad Ejemplo, ZZ 99999-1234"),
                    "con domicilio en 100 Ejemplo Ave, Suite 555-B, Ciudad Ejemplo, ZZ 99999-1234", None)]
            })
            .to_string()
        } else {
            serde_json::json!({
                "menciones": [mention("o", "organizacion", "Ejemplo Uno LLC", "parte", "Acuerdo de Ejemplo Uno LLC",
                    ids(&[("matricula", "7654321", "registration number 7654321")]))],
                "afirmaciones": []
            })
            .to_string()
        }
    })
}

#[test]
fn a_part_of_an_address_is_not_an_identifier_and_does_not_keep_a_company_apart() {
    let (_dir, base) = base_with(&[("s1.txt", S1), ("s2.txt", S2)]);
    let asker = address_model();
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    let conn = base.conn();
    let values: Vec<String> = conn
        .prepare("SELECT value FROM knowledge_identifier ORDER BY value")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(values, ["7654321"]);
    let companies: i64 = conn
        .query_row("SELECT count(DISTINCT entity_id) FROM knowledge_mention WHERE kind = 'organizacion'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(companies, 1, "the same name, nothing keeps them apart");
}

const K1: &str = "DOC-K1. Constancia de Persona Dos, DNI 22.222.222, para el trámite.";
const K2: &str = "DOC-K2. Planilla del servicio con el alta de Persona Dos 22222222 en la lista de este año, sin otro dato.";
const K3: &str = "DOC-K3. Nota de Persona Dos, DNI 33.333.333, vecina de otra ciudad.";

fn kind_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let ids = |k: &str, v: &str, q: &str| serde_json::json!([{ "clase": k, "valor": v, "cita": q }]);
        let m = if prompt.contains("DOC-K1.") {
            mention("p", "persona", "Persona Dos", "parte", "Constancia de Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222"))
        } else if prompt.contains("DOC-K2.") {
            mention("p", "persona", "Persona Dos", "parte", "Persona Dos 22222222", ids("otro", "22222222", "Persona Dos 22222222"))
        } else {
            mention("p", "persona", "Persona Dos", "parte", "Nota de Persona Dos, DNI 33.333.333", ids("dni", "33.333.333", "DNI 33.333.333"))
        };
        serde_json::json!({ "menciones": [m], "afirmaciones": [] }).to_string()
    })
}

#[test]
fn the_same_number_read_as_another_kind_is_that_identifier() {
    let (_dir, base) = base_with(&[("k1.txt", K1), ("k2.txt", K2), ("k3.txt", K3)]);
    let asker = kind_model();
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    let entity_of = |content: &str| -> String {
        base.conn()
            .query_row(
                "SELECT m.entity_id FROM knowledge_mention m JOIN file f ON f.version_id = m.version_id JOIN location l ON l.file_hash = f.hash
                 WHERE l.name = ?1",
                [content],
                |row| row.get(0),
            )
            .unwrap()
    };
    assert_eq!(entity_of("k1.txt"), entity_of("k2.txt"), "the number of the planilla is the DNI");
    assert_ne!(entity_of("k1.txt"), entity_of("k3.txt"), "another DNI is another person");
}

const P1: &str = "DOC-P1. Ficha de cliente. Nombre: Persona Dos Tres. Cónyuge: Persona Cinco Seis DNI 55.555.555 ama de casa. \
Firma: Persona Dos Tres.";

#[test]
fn a_number_written_after_another_persons_name_is_not_hers() {
    let model = FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let ids = serde_json::json!([{ "clase": "dni", "valor": "55.555.555", "cita": "DNI 55.555.555" }]);
        serde_json::json!({
            "menciones": [
                mention("p", "persona", "Persona Dos Tres", "parte", "Nombre: Persona Dos Tres", ids),
                mention("c", "persona", "Persona Cinco Seis", "otro", "Cónyuge: Persona Cinco Seis", serde_json::json!([])),
            ],
            "afirmaciones": []
        })
        .to_string()
    });
    let (_dir, base) = base_with(&[("p1.txt", P1)]);
    base.types_run(&model, 50).unwrap();
    base.knowledge_run(&model, 50).unwrap();
    let kept: Vec<String> = base
        .conn()
        .prepare("SELECT m.written FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(kept, ["Persona Cinco Seis"], "the spouse's DNI is not the client's: it is the spouse's");
}

const V1: &str = "DOC-V1. Nota firmada por Persona Dos Tres, de la oficina central, sobre el pedido de este mes.";
const V2: &str = "DOC-V2. Respuesta de Persona Tres a la nota de este mes, sin más datos de ella.";
const V3: &str = "DOC-V3. Recibo de Persona Tres, correo persona.tres@ejemplo.com, de la oficina central.";

fn doubtful_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            return serde_json::json!({ "respuestas": [{ "candidato": 1, "decision": "dudosa", "certeza": "media", "razon": "no alcanza" }] })
                .to_string();
        }
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let none = serde_json::json!([]);
        let (m, claims) = if prompt.contains("DOC-V1.") {
            (mention("p", "persona", "Persona Dos Tres", "firmante", "firmada por Persona Dos Tres", none), vec![])
        } else if prompt.contains("DOC-V2.") {
            (mention("p", "persona", "Persona Tres", "remitente", "Respuesta de Persona Tres", none), vec![])
        } else {
            (
                mention("p", "persona", "Persona Tres", "emisor", "Recibo de Persona Tres", none),
                vec![claim("p", "contacto", None, Some("persona.tres@ejemplo.com"), "correo persona.tres@ejemplo.com", None)],
            )
        };
        serde_json::json!({ "menciones": [m], "afirmaciones": claims }).to_string()
    })
}

#[test]
fn a_verdict_that_kept_two_apart_is_asked_again_when_they_show_more() {
    let (dir, base) = base_with(&[("v1.txt", V1), ("v2.txt", V2)]);
    let asker = doubtful_model();
    let identity = |m: &FakeModel| m.asked.lock().unwrap().iter().filter(|q| q.contains("Hay que decidir si cada candidato")).count();
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!(identity(&asker), 1);
    super::mark_dirty(&base.conn(), "resolve", "otra vez").unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!(identity(&asker), 1, "nothing new: nothing is asked");
    // A new document shows more of one side: the doubt is asked again.
    dir.write("o/v3.txt", V3.as_bytes());
    super::tests::refresh(&base, &base.origins(&crate::docbase::Visible::All).unwrap()[0].id);
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!(identity(&asker), 2);
    super::mark_dirty(&base.conn(), "resolve", "otra vez").unwrap();
    base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!(identity(&asker), 2);
}
