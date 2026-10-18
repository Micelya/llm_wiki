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

const NOTICES: &str = "Ejemplo Uno LLC (en adelante, «Alfa») y Otra Asociación (en adelante, «Beta») acuerdan lo siguiente. \
Notificaciones. Si a Beta: Otra Asociación, Calle Uno 100. Si a Alfa: Ejemplo Uno LLC, Calle Dos 200.";

#[test]
fn a_defined_term_names_one_thing_in_its_document() {
    // The model made the notice's "Beta" the same as the first party.
    let link = |mut m: serde_json::Value, to: &str| {
        m["misma_que"] = serde_json::Value::String(to.to_string());
        m
    };
    for with_definitions in [true, false] {
        let mut said = serde_json::json!({
            "menciones": [
                mention("a", "organizacion", "Ejemplo Uno LLC", "parte", "Ejemplo Uno LLC (en adelante, «Alfa»)", serde_json::json!([])),
                mention("b", "organizacion", "Otra Asociación", "parte", "y Otra Asociación (en adelante, «Beta»)", serde_json::json!([])),
                link(mention("c", "organizacion", "Alfa", "parte", "Ejemplo Uno LLC (en adelante, «Alfa»)", serde_json::json!([])), "a"),
                link(mention("d", "organizacion", "Beta", "parte", "Otra Asociación (en adelante, «Beta»)", serde_json::json!([])), "b"),
                link(mention("e", "organizacion", "Beta", "destinatario", "Si a Beta: Otra Asociación", serde_json::json!([])), "a"),
                link(mention("f", "organizacion", "Alfa", "destinatario", "Si a Alfa: Ejemplo Uno LLC", serde_json::json!([])), "a"),
            ]
        });
        if with_definitions {
            said["definiciones"] = serde_json::json!([
                { "termino": "Alfa", "ref": "a", "cita": "Ejemplo Uno LLC (en adelante, «Alfa»)" },
                { "termino": "Beta", "ref": "b", "cita": "Otra Asociación (en adelante, «Beta»)" }
            ]);
        }
        let reading = read(NOTICES, said);
        let by_ref = |r: &str| reading.mentions.iter().find(|m| m.reference == r).unwrap();
        assert!(by_ref("m5").defined, "{:?}", reading.mentions);
        assert_ne!(by_ref("m5").same_as.as_deref(), Some("m1"), "a term names one thing: {:?}", reading.mentions);
        // What agrees with the term's first mention stays.
        assert_eq!(by_ref("m3").same_as.as_deref(), Some("m1"));
        assert_eq!(by_ref("m4").same_as.as_deref(), Some("m2"));
        assert_eq!(by_ref("m6").same_as.as_deref(), Some("m1"));
    }
}

const BRAND: &str = "CARTA DE ACEPTACIÓN\nSres.\nmarcainventada (Ejemplo Uno S.A.)\nPRESENTE\nAceptamos la propuesta.";

#[test]
fn a_company_name_in_parentheses_is_not_a_defined_term() {
    let reading = read(
        BRAND,
        serde_json::json!({
            "menciones": [
                mention("a", "organizacion", "marcainventada", "destinatario", "marcainventada (Ejemplo Uno S.A.)", serde_json::json!([])),
                mention("b", "organizacion", "Ejemplo Uno S.A.", "parte", "marcainventada (Ejemplo Uno S.A.)", serde_json::json!([])),
            ],
            "definiciones": [
                { "termino": "Ejemplo Uno S.A.", "ref": "a", "cita": "marcainventada (Ejemplo Uno S.A.)" }
            ]
        }),
    );
    assert!(reading.mentions.iter().all(|m| !m.defined), "{:?}", reading.mentions);
    assert!(reasons(&reading).contains(&("definition".to_string(), "termino-con-forma-juridica".to_string())));
}

const INSTALMENTS: &str = "Buenos Aires, 10 de marzo de 2025. Ejemplo Uno S.A. (en adelante, la «Empresa») y Persona Dos \
acuerdan que la Empresa pagará a Persona Dos la suma de USD 3.000 en dos cuotas: USD 1.500 con vencimiento el 30 de abril de 2025 \
y USD 1.500 el 31 de mayo de 2025. Persona Dos recibió de la Empresa USD 700 el 1 de marzo de 2025. A modo de ejemplo, si el \
consumo fuera de USD 9.000, la Empresa le facturaría a Persona Dos USD 720.";

fn with_state(mut amount: serde_json::Value, state: &str) -> serde_json::Value {
    amount["estado"] = serde_json::Value::String(state.to_string());
    amount
}

#[test]
fn an_amount_keeps_its_state_and_a_payment_due_after_the_document_is_not_paid() {
    let reading = read(
        INSTALMENTS,
        serde_json::json!({
            "fecha_documento": "10 de marzo de 2025",
            "menciones": [
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A. (en adelante, la «Empresa»)", serde_json::json!([])),
                mention("p", "persona", "Persona Dos", "parte", "y Persona Dos acuerdan", serde_json::json!([])),
            ],
            "importes": [
                with_state(amount("o", "p", "USD 3.000", "saldo", "la Empresa pagará a Persona Dos la suma de USD 3.000 en dos cuotas", None), "pendiente"),
                // The model said it was paid: it falls due after the document.
                with_state(amount("o", "p", "USD 1.500", "cuota", "USD 1.500 con vencimiento el 30 de abril de 2025", Some("30 de abril de 2025")), "pagado"),
                // Paid before the document: it stays paid.
                with_state(amount("o", "p", "USD 700", "cuota", "Persona Dos recibió de la Empresa USD 700 el 1 de marzo de 2025", Some("1 de marzo de 2025")), "pagado"),
                // An example says nothing of the parties.
                with_state(amount("p", "o", "USD 720", "factura", "la Empresa le facturaría a Persona Dos USD 720", None), "ejemplo"),
            ]
        }),
    );
    let state_of = |value: &str| {
        reading
            .evidence
            .iter()
            .find(|e| e.amount.as_deref() == Some(value))
            .map(|e| e.detail.as_ref().unwrap().get("estado").and_then(|s| s.as_str()).unwrap_or("").to_string())
    };
    assert_eq!(state_of("USD 3.000").as_deref(), Some("pendiente"));
    assert_eq!(state_of("USD 1.500").as_deref(), Some("pendiente"));
    assert_eq!(state_of("USD 700").as_deref(), Some("pagado"));
    assert_eq!(state_of("USD 720"), None, "an example is not kept");
    assert!(reasons(&reading).contains(&(CORRECTION.to_string(), "vence-despues-del-documento".to_string())));
    assert!(reasons(&reading).contains(&("amount".to_string(), "no-es-un-hecho".to_string())));
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

#[test]
fn a_number_too_short_identifies_no_one() {
    // D-288: "Registro N° 2" would join everyone with a registration 2.
    let text = "Persona Seis, inscripta en el Registro N° 2, DNI 66.666.666.";
    let reading = read(
        text,
        serde_json::json!({
            "menciones": [
                mention("p", "persona", "Persona Seis", "parte", "Persona Seis, inscripta en el Registro N° 2",
                    serde_json::json!([
                        { "clase": "matricula", "valor": "2", "cita": "Registro N° 2", "rotulo": null },
                        { "clase": "dni", "valor": "66.666.666", "cita": "DNI 66.666.666", "rotulo": null },
                    ])),
            ],
            "afirmaciones": []
        }),
    );
    let ids: Vec<String> = reading.mentions[0].identifiers.iter().map(|i| i.normalized.clone()).collect();
    assert_eq!(ids, ["66666666"], "{:?}", reading.discarded);
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
                // The company is named right before and the quote says she
                // acts for it (P-122, D-269): kept.
                claim("p", "representa_a", Some("o"), None, "Persona Tres es apoderada", None),
                claim("p", "representa_a", Some("o"), None, "Persona Tres, por Ejemplo Uno S.A.", None),
                // Represents whom? The quote neither says it nor says how.
                claim("p", "representa_a", Some("o"), None, "representa a la sociedad", None),
            ]
        }),
    );
    // The address of the signature block is the company's.
    let domicile = reading.evidence.iter().find(|e| e.predicate == "domicilio").unwrap();
    assert_eq!(domicile.subject, "m2");
    // The two that say the same are one piece of evidence.
    assert_eq!(reading.evidence.iter().filter(|e| e.predicate == "representa_a").count(), 1, "{:?}", reading.discarded);
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

const INVOICE: &str = "FACTURA\nCUIT: Ingresos brutos: Inicio de actividades:\n30-11111111-1 901-123456 01/01/2010\nEjemplo Uno S.A.";

#[test]
fn a_label_and_its_value_apart_in_the_text_are_one_quote() {
    let reading = read(
        INVOICE,
        serde_json::json!({
            "menciones": [mention("o", "organizacion", "Ejemplo Uno S.A.", "emisor", "Ejemplo Uno S.A.",
                ids("cuit", "30-11111111-1", "CUIT: 30-11111111-1"))],
            "afirmaciones": [
                claim("o", "inicio_de_vinculo", None, Some("01/01/2010"), "Inicio de actividades: 01/01/2010", None),
                // Too far apart, or not in the text: not found.
                claim("o", "inicio_de_vinculo", None, Some("01/01/2011"), "Inicio de actividades: 01/01/2011", None),
            ]
        }),
    );
    let ids: Vec<&str> = reading.mentions[0].identifiers.iter().map(|i| i.value.as_str()).collect();
    assert_eq!(ids, ["30-11111111-1"], "{:?}", reading.discarded);
    assert_eq!(reading.evidence.len(), 1, "{:?}", reading.discarded);
}

const POWER: &str = "Ejemplo Uno S.A., CUIT 30-11111111-1, representada por Persona Tres, apoderada, DNI 22.222.222, CUIT 27-22222222-4.";

#[test]
fn a_number_with_the_prefix_of_a_person_given_to_a_company_is_the_person_s() {
    let reading = read(
        POWER,
        serde_json::json!({
            "menciones": [
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A., CUIT 30-11111111-1",
                    serde_json::json!([
                        { "clase": "cuit", "valor": "30-11111111-1", "cita": "Ejemplo Uno S.A., CUIT 30-11111111-1", "rotulo": null },
                        { "clase": "cuit", "valor": "27-22222222-4", "cita": "DNI 22.222.222, CUIT 27-22222222-4", "rotulo": null },
                    ])),
                mention("p", "persona", "Persona Tres", "apoderado", "representada por Persona Tres, apoderada",
                    ids("dni", "22.222.222", "DNI 22.222.222")),
            ]
        }),
    );
    let of = |name: &str| reading.mentions.iter().find(|m| m.written == name).unwrap();
    let numbers = |name: &str| of(name).identifiers.iter().map(|i| i.value.clone()).collect::<Vec<_>>();
    assert_eq!(numbers("Ejemplo Uno S.A."), ["30-11111111-1"]);
    assert_eq!(numbers("Persona Tres"), ["22.222.222", "27-22222222-4"]);
    assert!(reasons(&reading).contains(&(CORRECTION.to_string(), "de-otra-persona".to_string())));

    // With no person it could be, it is nobody's.
    let alone = read(
        "Ejemplo Uno S.A., CUIT 20-33333333-3.",
        serde_json::json!({
            "menciones": [
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A., CUIT 20-33333333-3",
                    ids("cuit", "20-33333333-3", "Ejemplo Uno S.A., CUIT 20-33333333-3")),
            ]
        }),
    );
    assert!(alone.mentions[0].identifiers.is_empty());
    assert!(reasons(&alone).contains(&("identifier".to_string(), "de-otra-persona".to_string())));
}

/// The summary of a document comes with its reading (D-330): a point
/// stays when its quote is in the text and its numbers are in its quote;
/// its mentions are the kept ones.
#[test]
fn the_summary_comes_with_the_reading_and_each_point_with_its_quote() {
    let reading = read(
        LOAN,
        serde_json::json!({
            "menciones": [
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A. (en adelante, la «Empresa»)", serde_json::json!([])),
                mention("p", "persona", "Persona Dos", "parte", "y Persona Dos (en adelante, la «Prestamista»)", serde_json::json!([])),
            ],
            "afirmaciones": [],
            "importes": [],
            "resumen": {
                "tema": "Acuerdo entre Ejemplo Uno S.A. y Persona Dos sobre un saldo.",
                "puntos": [
                    { "texto": "La prestamista le debe a la empresa USD 5.000 de saldo.", "menciones": ["p", "o", "x"],
                      "cita": "la Prestamista le adeuda a la Empresa la suma de USD 5.000 en concepto de saldo", "pagina": null },
                    // A number its quote does not have.
                    { "texto": "La deuda vence en 2031.", "menciones": [],
                      "cita": "la Prestamista le adeuda a la Empresa la suma de USD 5.000", "pagina": null },
                    // A quote that is not in the text.
                    { "texto": "Las partes se reconcilian.", "menciones": [], "cita": "las partes se reconcilian", "pagina": null },
                ]
            }
        }),
    );
    assert_eq!(reading.summary.topic.as_deref(), Some("Acuerdo entre Ejemplo Uno S.A. y Persona Dos sobre un saldo."));
    assert_eq!(reading.summary.points.len(), 1);
    let point = &reading.summary.points[0];
    assert_eq!(point.mentions.len(), 2, "the unknown ref goes");
    let reasons = reasons(&reading);
    assert!(reasons.contains(&("summary".to_string(), "resumen-numero-fuera-de-la-cita".to_string())));
    assert!(reasons.contains(&("summary".to_string(), "resumen-cita-no-encontrada".to_string())));
    // A topic with a number the text does not have is left out.
    let reading = read(LOAN, serde_json::json!({ "menciones": [], "resumen": { "tema": "Préstamo de 1999.", "puntos": [] } }));
    assert_eq!(reading.summary.topic, None);
}

/// The summary is kept in the base with the reading, its points pointing
/// at the evidence of the same passage; reading the document again
/// replaces it, and nothing else asks the model about the document.
#[test]
fn the_summary_is_kept_with_the_reading_and_points_at_its_evidence() {
    let body = "CONTRATO DE MUTUO\n\nEjemplo Uno S.A. presta a Persona Dos la suma de $ 50.000, que Persona Dos devolverá en diez cuotas.";
    let (_dir, base) = base_with(&[("mutuo.txt", body)]);
    let asker = FakeModel::new("fake:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        serde_json::json!({
            "menciones": [
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A. presta a Persona Dos", serde_json::json!([])),
                mention("p", "persona", "Persona Dos", "parte", "presta a Persona Dos la suma de $ 50.000", serde_json::json!([])),
            ],
            "afirmaciones": [],
            "importes": [amount("p", "o", "$ 50.000", "precio", "Ejemplo Uno S.A. presta a Persona Dos la suma de $ 50.000", None)],
            "resumen": {
                "tema": "Contrato de mutuo.",
                "puntos": [{ "texto": "Ejemplo Uno S.A. le presta $ 50.000 a Persona Dos.", "menciones": ["o", "p"],
                             "cita": "Ejemplo Uno S.A. presta a Persona Dos la suma de $ 50.000", "pagina": null }]
            }
        })
        .to_string()
    });
    base.types_run(&asker, 50).unwrap();
    base.knowledge_run(&asker, 50).unwrap();
    let knowledge_questions = asker.asked.lock().unwrap().iter().filter(|q| is_knowledge(q)).count();
    assert_eq!(knowledge_questions, 1, "one reading per document");
    let conn = base.conn();
    let (topic, points): (Option<String>, String) =
        conn.query_row("SELECT topic, points FROM knowledge_summary", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(topic.as_deref(), Some("Contrato de mutuo."));
    let points: Vec<super::extract::SummaryPoint> = serde_json::from_str(&points).unwrap();
    assert_eq!(points.len(), 1);
    let evidence: i64 = conn.query_row("SELECT id FROM knowledge_evidence WHERE amount IS NOT NULL", [], |row| row.get(0)).unwrap();
    assert_eq!(points[0].evidence, vec![evidence]);
    let mentions: i64 = conn
        .query_row(&format!("SELECT count(*) FROM knowledge_mention WHERE id IN ({})", points[0].mentions.join(",")), [], |row| row.get(0))
        .unwrap();
    assert_eq!(mentions, 2);
}
