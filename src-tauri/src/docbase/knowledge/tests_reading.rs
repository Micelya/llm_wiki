//! The second question of the reading and its checks (D-244 to D-252):
//! money as a relation, defined terms, domiciles with their class, time
//! and holder, relations that name whom, forms, identifiers by their label
//! and the record of a signing service. Invented documents and a stand-in
//! model (R-24: nothing here comes from the test folder).

use super::extract::{verify, Answer, CORRECTION};
use super::tests::{amount, answer, base_with, claim, is_knowledge, mention, type_answer, FakeModel};
use super::vocabulary::Vocabulary;
use crate::docbase::Visible;

fn read(body: &str, said: serde_json::Value) -> super::extract::Reading {
    let answer: Answer = serde_json::from_value(said).unwrap();
    verify(&Vocabulary::builtin(), body, &[], &[answer])
}

fn ids(kind: &str, value: &str, quote: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": kind, "valor": value, "cita": quote, "rotulo": null }])
}

fn reasons(reading: &super::extract::Reading) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = reading.discarded.iter().map(|d| (d.what.clone(), d.reason.clone())).collect();
    out.sort();
    out
}

const LOAN: &str = "Ejemplo Uno S.A. (en adelante, la «Empresa») y Persona Dos (en adelante, la «Prestamista») acuerdan: \
la Prestamista le adeuda a la Empresa la suma de USD 5.000 en concepto de saldo. Percepción IVA $ 100,00.";

#[test]
fn an_amount_is_between_two_parties_and_its_direction_is_the_quotes() {
    let reading = read(
        LOAN,
        serde_json::json!({
            "menciones": [
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A. (en adelante, la «Empresa»)", serde_json::json!([])),
                mention("p", "persona", "Persona Dos", "parte", "y Persona Dos (en adelante, la «Prestamista»)", serde_json::json!([])),
                mention("e", "organizacion", "Empresa", "parte", "le adeuda a la Empresa la suma de USD 5.000", serde_json::json!([])),
                mention("q", "persona", "Prestamista", "parte", "la Prestamista le adeuda a la Empresa", serde_json::json!([])),
            ],
            "importes": [
                // The model put the company as the debtor: the quote says the
                // lender owes the company.
                amount("e", "q", "USD 5.000", "saldo", "la Prestamista le adeuda a la Empresa la suma de USD 5.000 en concepto de saldo", None),
                // A tax is not an amount between parties.
                amount("q", "e", "$ 100,00", "saldo", "Percepción IVA $ 100,00", None),
                // Without the other party it does not go.
                serde_json::json!({ "deudor": "q", "acreedor": null, "importe": "USD 5.000", "moneda": null, "tipo": "saldo",
                    "concepto": null, "fecha": null, "cita": "la suma de USD 5.000", "pagina": null }),
            ]
        }),
    );
    // The defined terms are local aliases of what they define.
    let by_ref = |r: &str| reading.mentions.iter().find(|m| m.reference == r).unwrap();
    let (company, lender) = (by_ref("m3"), by_ref("m4"));
    assert!(company.defined && lender.defined);
    assert_eq!((company.same_as.as_deref(), lender.same_as.as_deref()), (Some("m1"), Some("m2")));
    assert!(!by_ref("m1").defined);
    assert_eq!(reading.evidence.len(), 1, "{:?}", reading.discarded);
    let money = &reading.evidence[0];
    assert_eq!((money.predicate.as_str(), money.subject.as_str(), money.object_mention.as_deref()), ("importe", "m4", Some("m3")));
    assert_eq!(money.amount.as_deref(), Some("USD 5.000"));
    assert_eq!(money.detail.as_ref().unwrap()["tipo"], "saldo");
    assert_eq!(money.detail.as_ref().unwrap()["moneda"], "usd");
    assert!(reasons(&reading).contains(&(CORRECTION.to_string(), "direccion-invertida".to_string())));
    assert!(reasons(&reading).contains(&("amount".to_string(), "es-un-impuesto".to_string())));
    assert!(reasons(&reading).contains(&("amount".to_string(), "sin-contraparte".to_string())));
}

const FORM: &str = "FICHA\nPROVINCIA: Ejemplo\nApellidos: DOS\nNombres: PERSONA\nDomicilio particular:\n\nCalle Uno 100\n\
Cónyuge: Persona Cinco DNI 55.555.555\nEmpleador: Otra LLC\nEIN: 12-3456789\nCD Nro. 123456789 AR\nNúmero de cliente 4444";

#[test]
fn a_form_keeps_each_value_with_its_field_and_its_person() {
    let reading = read(
        FORM,
        serde_json::json!({
            "menciones": [
                mention("prov", "organizacion", "PROVINCIA", "otro", "PROVINCIA: Ejemplo", serde_json::json!([])),
                // The name in two fields of the form.
                mention("p", "persona", "Persona Dos", "parte", "Apellidos: DOS Nombres: PERSONA",
                    serde_json::json!([
                        { "clase": "dni", "valor": "55.555.555", "cita": "Cónyuge: Persona Cinco DNI 55.555.555", "rotulo": null },
                        { "clase": "otro", "valor": "123456789", "cita": "CD Nro. 123456789 AR", "rotulo": null },
                        { "clase": "otro", "valor": "4444", "cita": "Número de cliente 4444", "rotulo": null },
                    ])),
                mention("c", "persona", "Persona Cinco", "otro", "Cónyuge: Persona Cinco", serde_json::json!([])),
                mention("o", "organizacion", "Otra LLC", "otro", "Empleador: Otra LLC", ids("cuit", "12-3456789", "EIN: 12-3456789")),
            ],
            "afirmaciones": [
                // The label and the value apart.
                { "sujeto": "p", "predicado": "domicilio", "objeto_ref": null, "objeto_valor": "Calle Uno 100", "clase": "real",
                  "cita": "Calle Uno 100", "rotulo": "Domicilio particular:", "pagina": null, "tiempo": null },
            ]
        }),
    );
    let names: Vec<&str> = reading.mentions.iter().map(|m| m.written.as_str()).collect();
    assert_eq!(names, ["Persona Dos", "Persona Cinco", "Otra LLC"], "{:?}", reading.discarded);
    let of = |name: &str| reading.mentions.iter().find(|m| m.written == name).unwrap();
    let kinds = |name: &str| of(name).identifiers.iter().map(|i| (i.kind.clone(), i.normalized.clone())).collect::<Vec<_>>();
    // The spouse's DNI is the spouse's; a shipment number identifies no one;
    // the number of a client is one; the EIN is an EIN by its label.
    assert_eq!(kinds("Persona Dos"), [("cliente".to_string(), "4444".to_string())]);
    assert_eq!(kinds("Persona Cinco"), [("dni".to_string(), "55555555".to_string())]);
    assert_eq!(kinds("Otra LLC"), [("ein".to_string(), "123456789".to_string())]);
    assert_eq!(reading.evidence.len(), 1, "{:?}", reading.discarded);
    assert_eq!(reading.evidence[0].detail.as_ref().unwrap()["clase"], "real");
    let all = reasons(&reading);
    for reason in ["rotulo-de-formulario", "no-identifica", "de-otra-persona", "clase-por-su-rotulo"] {
        assert!(all.iter().any(|(_, r)| r == reason), "{reason}: {all:?}");
    }
}

const LETTER: &str = "## Page 1\n\nPersona Tres, por Ejemplo Uno S.A., con domicilio en Calle Dos 200, representa a la sociedad. \
Persona Tres es apoderada.\n\n## Page 2\n\nCertificate Of Completion\nEnvelope Summary Events\nSigner Events\n\
Persona Tres firmó el sobre.";

#[test]
fn a_signature_block_and_a_signing_record_say_what_they_say() {
    let reading = read(
        LETTER,
        serde_json::json!({
            "menciones": [
                mention("p", "persona", "Persona Tres", "apoderado", "Persona Tres, por Ejemplo Uno S.A.", serde_json::json!([])),
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "por Ejemplo Uno S.A., con domicilio", serde_json::json!([])),
                mention("s", "persona", "Persona Tres", "parte", "Persona Tres firmó el sobre", serde_json::json!([])),
            ],
            "afirmaciones": [
                claim("p", "domicilio", None, Some("Calle Dos 200"), "Persona Tres, por Ejemplo Uno S.A., con domicilio en Calle Dos 200", None),
                // Represents whom? The quote does not say.
                claim("p", "representa_a", Some("o"), None, "Persona Tres es apoderada", None),
                claim("p", "representa_a", Some("o"), None, "Persona Tres, por Ejemplo Uno S.A.", None),
            ]
        }),
    );
    // The address of the signature block is the company's.
    let domicile = reading.evidence.iter().find(|e| e.predicate == "domicilio").unwrap();
    assert_eq!(domicile.subject, "m2");
    assert_eq!(reading.evidence.iter().filter(|e| e.predicate == "representa_a").count(), 1);
    // On the page of the signing record, the party is a signer.
    let roles: Vec<&str> = reading.mentions.iter().filter(|m| m.written == "Persona Tres").map(|m| m.role.as_str()).collect();
    assert_eq!(roles, ["apoderado", "firmante"]);
    let all = reasons(&reading);
    for reason in ["domicilio-de-la-sociedad", "objeto-fuera-de-la-cita", "papel-de-la-constancia-de-firma"] {
        assert!(all.iter().any(|(_, r)| r == reason), "{reason}: {all:?}");
    }
}

const D1: &str = "DOC-D1. Nota de Persona Seis, DNI 33.333.333, con domicilio en Calle Tres 300. Buenos Aires, 1 de marzo de 2020.";
const D2: &str = "DOC-D2. Nota de Persona Seis, DNI 33.333.333, con domicilio en Calle Cuatro 400. Buenos Aires, 5 de mayo de 2023.";
const D3: &str = "DOC-D3. Persona Seis, DNI 33.333.333, constituye domicilio en Calle Cinco 500. Buenos Aires, 5 de mayo de 2023.";

fn dated_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let (date, street, verb) = if prompt.contains("DOC-D1.") {
            ("1 de marzo de 2020", "Calle Tres 300", "con domicilio en")
        } else if prompt.contains("DOC-D2.") {
            ("5 de mayo de 2023", "Calle Cuatro 400", "con domicilio en")
        } else {
            ("5 de mayo de 2023", "Calle Cinco 500", "constituye domicilio en")
        };
        let mut said: serde_json::Value = serde_json::from_str(&answer(
            vec![mention("p", "persona", "Persona Seis", "parte", "Persona Seis, DNI 33.333.333", ids("dni", "33.333.333", "DNI 33.333.333"))],
            vec![claim("p", "domicilio", None, Some(street), &format!("{verb} {street}"), None)],
        ))
        .unwrap();
        said["fecha_documento"] = date.into();
        said.to_string()
    })
}

#[test]
fn domiciles_at_different_dates_or_of_different_classes_are_not_a_conflict() {
    let (_dir, base) = base_with(&[("d1.txt", D1), ("d2.txt", D2), ("d3.txt", D3)]);
    let asker = dated_model();
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    let report = base.knowledge_resolve(Some(&asker), 50).unwrap();
    assert_eq!(report.conflicts, 0, "{report:?}");
    let conn = base.conn();
    let mut statement = conn
        .prepare("SELECT object_value, valid_from, coalesce(qualifier, '') FROM knowledge_claim WHERE predicate = 'domicilio' ORDER BY 1")
        .unwrap();
    let rows: Vec<(String, Option<String>, String)> =
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(
        rows,
        [
            ("Calle Cinco 500".to_string(), Some("5 de mayo de 2023".to_string()), "constituido".to_string()),
            ("Calle Cuatro 400".to_string(), Some("5 de mayo de 2023".to_string()), String::new()),
            ("Calle Tres 300".to_string(), Some("1 de marzo de 2020".to_string()), String::new()),
        ]
    );
    // The entity's detail shows them, none in conflict.
    let entity: String = conn.query_row("SELECT subject FROM knowledge_claim LIMIT 1", [], |row| row.get(0)).unwrap();
    drop(statement);
    drop(conn);
    let detail = base.knowledge_entity(&Visible::All, &entity).unwrap().unwrap();
    assert!(detail.claims.iter().all(|c| c.state == "supported"));
}
