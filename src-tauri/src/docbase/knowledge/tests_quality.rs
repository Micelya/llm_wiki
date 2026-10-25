//! The quality of the people and organizations the reading makes: what
//! names no one (a misread, an office alone, a rule taken for an
//! organization, a domicile taken for a person), a definition by another
//! term, a second number of another person, what the lists show and what
//! a doubt shows. Invented documents and a stand-in model (R-24: nothing
//! here comes from the test folder).

use super::extract::{verify, Answer};
use super::names;
use super::tests::{base_with, is_knowledge, mention, type_answer, FakeModel};
use super::vocabulary::Vocabulary;
use crate::docbase::{DocBase, Visible};

fn read(body: &str, said: serde_json::Value) -> super::extract::Reading {
    let answer: Answer = serde_json::from_value(said).unwrap();
    verify(&Vocabulary::builtin(), body, &[], &[answer])
}

fn ids(values: &[(&str, &str)], quote: &str) -> serde_json::Value {
    serde_json::Value::Array(values.iter().map(|(kind, value)| serde_json::json!({ "clase": kind, "valor": value, "cita": quote, "rotulo": null })).collect())
}

fn none() -> serde_json::Value {
    serde_json::json!([])
}

#[test]
fn a_name_that_names_no_one_says_why() {
    let v = Vocabulary::builtin();
    let (p, o) = ("persona", "organizacion");
    assert_eq!(v.not_named(p, "Ana [ilegible]"), Some("nombre-ilegible"));
    assert_eq!(v.not_named(p, "Ana Gó?"), Some("nombre-ilegible"));
    assert_eq!(v.not_named(p, "C a r l o s Pérez"), Some("letras-sueltas"));
    assert_eq!(v.not_named(p, "J U A N"), Some("letras-sueltas"));
    assert_eq!(v.not_named(p, "ana@ejemplo.com"), Some("correo-o-web"));
    assert_eq!(v.not_named(o, "www.ejemplo.com.ar"), Some("correo-o-web"));
    assert_eq!(v.not_named(p, "Actuaria"), Some("cargo-sin-nombre"));
    assert_eq!(v.not_named(p, "la Secretaria"), Some("cargo-sin-nombre"));
    assert_eq!(v.not_named(p, "Sr. Juez"), Some("cargo-sin-nombre"));
    assert_eq!(v.not_named(o, "Ley de Ejemplo 1234"), Some("norma-no-organizacion"));
    assert_eq!(v.not_named(o, "Norma Técnica ABC"), Some("norma-no-organizacion"));
    assert_eq!(v.not_named(o, "Estándar Ejemplo"), Some("norma-no-organizacion"));
    // What does name someone: initials, a legal form written apart, an
    // office with a name, a person called like a rule, an acronym of an
    // organization, an office that is an organization.
    for (kind, name) in [
        (p, "J. R. R. Ejemplo"),
        (p, "J R R Ejemplo"),
        (o, "Ejemplo Uno S A U"),
        (p, "la secretaria Ana Gómez"),
        (p, "Secretaria Ana Gómez"),
        (p, "Norma Ejemplo"),
        (o, "ABCD"),
        (o, "Secretaría de Ejemplo"),
        (o, "Código Ejemplo S.A."),
    ] {
        assert_eq!(v.not_named(kind, name), None, "{name}");
    }
    // The pages and the screens see the same.
    assert!(!v.is_a_name(p, "Actuaria") && !v.is_a_name(p, "C a r l o s Pérez") && !v.is_a_name(o, "Ley de Ejemplo"));
    assert!(v.is_a_name(p, "Secretaria Ana Gómez") && v.is_a_name(o, "ABCD") && v.is_a_name(p, "Norma Ejemplo"));
    // An office before a name is like a title: the same person.
    assert_eq!(names::key(&v, p, "Jueza Ana Gómez").words, names::key(&v, p, "Ana Gómez").words);
    assert_ne!(names::key(&v, o, "Secretaría de Ejemplo").words, names::key(&v, o, "Ejemplo").words);
}

const ACT: &str = "ACTA. En la ciudad de Ejemplo, ante la Actuaria, comparece Ana Gómez, DNI 30.123.456, con domicilio en \
calle Bruno Sosa 100, y la secretaria Carla Ruiz. Firma: C a r l o s Pérez. Testigo: Dora [ilegible]. Se aplica la Ley de \
Ejemplo 1234. Contacto: ana@ejemplo.com. También comparece Bruno Sosa, vecino. Firma: Eva Gó[ilegible], DNI 30.999.888.";

#[test]
fn the_reading_drops_who_is_no_one() {
    let reading = read(
        ACT,
        serde_json::json!({
            "menciones": [
                mention("a", "persona", "Actuaria", "otro", "ante la Actuaria, comparece Ana Gómez", none()),
                mention("b", "persona", "Ana Gómez", "parte", "comparece Ana Gómez, DNI 30.123.456", ids(&[("dni", "30.123.456")], "Ana Gómez, DNI 30.123.456")),
                mention("c", "persona", "Bruno Sosa", "otro", "con domicilio en calle Bruno Sosa 100", none()),
                mention("d", "persona", "Carla Ruiz", "otro", "y la secretaria Carla Ruiz", none()),
                mention("e", "persona", "C a r l o s Pérez", "firmante", "Firma: C a r l o s Pérez", none()),
                mention("f", "persona", "Dora [ilegible]", "otro", "Testigo: Dora [ilegible]", none()),
                mention("g", "organizacion", "Ley de Ejemplo 1234", "otro", "Se aplica la Ley de Ejemplo 1234", none()),
                mention("h", "persona", "ana@ejemplo.com", "otro", "Contacto: ana@ejemplo.com", none()),
                mention("i", "persona", "Bruno Sosa", "parte", "También comparece Bruno Sosa, vecino", none()),
                mention("j", "persona", "Eva Gó[ilegible]", "firmante", "Firma: Eva Gó[ilegible], DNI 30.999.888", ids(&[("dni", "30.999.888")], "Eva Gó[ilegible], DNI 30.999.888")),
            ]
        }),
    );
    let kept: Vec<&str> = reading.mentions.iter().map(|m| m.written.as_str()).collect();
    // A person named like a street keeps her name where the text writes it
    // apart; a mail address still says whose contact it is; a misread name
    // with a DNI is still who the DNI says.
    assert_eq!(kept, vec!["Ana Gómez", "Carla Ruiz", "ana@ejemplo.com", "Bruno Sosa", "Eva Gó[ilegible]"], "{:?}", reading.discarded);
    let mut reasons: Vec<&str> = reading.discarded.iter().filter(|d| d.what == "mention").map(|d| d.reason.as_str()).collect();
    reasons.sort();
    assert_eq!(reasons, vec!["cargo-sin-nombre", "domicilio-como-persona", "letras-sueltas", "nombre-ilegible", "norma-no-organizacion"]);
}

const SERVICES: &str = "CONTRATO. Ejemplo Uno S.A. (en adelante, el «Cliente») y Ejemplo Dos S.R.L. (el «Proveedor») acuerdan: \
«Grupo» significa el Cliente y sus subsidiarias. «Personal» significa los empleados del Proveedor. El Proveedor presta servicios al Grupo \
con su Personal.";

#[test]
fn a_definition_by_another_term_follows_it_to_its_name() {
    let reading = read(
        SERVICES,
        serde_json::json!({
            "menciones": [
                mention("a", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A. (en adelante, el «Cliente»)", none()),
                mention("b", "organizacion", "Ejemplo Dos S.R.L.", "parte", "Ejemplo Dos S.R.L. (el «Proveedor»)", none()),
                mention("c", "organizacion", "Grupo", "otro", "El Proveedor presta servicios al Grupo", none()),
                mention("d", "organizacion", "Personal", "otro", "presta servicios al Grupo con su Personal", none()),
            ],
            "definiciones": [
                { "termino": "Cliente", "ref": "a", "cita": "Ejemplo Uno S.A. (en adelante, el «Cliente»)" },
                { "termino": "Proveedor", "ref": "b", "cita": "Ejemplo Dos S.R.L. (el «Proveedor»)" },
                // What the quote names by another term: the Cliente.
                { "termino": "Grupo", "ref": "a", "cita": "«Grupo» significa el Cliente y sus subsidiarias" },
                // The model said the Cliente; the quote names the Proveedor.
                { "termino": "Personal", "ref": "a", "cita": "«Personal» significa los empleados del Proveedor" }
            ]
        }),
    );
    let by_written = |w: &str| reading.mentions.iter().find(|m| m.written == w).unwrap();
    let group = by_written("Grupo");
    assert!(group.defined, "{:?}", reading.mentions);
    assert_eq!(group.same_as.as_deref(), Some(by_written("Ejemplo Uno S.A.").reference.as_str()), "{:?}", reading.discarded);
    assert_ne!(by_written("Personal").same_as.as_deref(), Some(by_written("Ejemplo Uno S.A.").reference.as_str()));
    let unnamed: Vec<&str> = reading.discarded.iter().filter(|d| d.reason == "definicion-sin-nombre").map(|d| d.said.as_str()).collect();
    assert_eq!(unnamed, vec!["Personal"]);
}

const FORM: &str = "DOC-P. Formulario de alta. Titular: Alba Ruiz. Cotitular: Bruno Sosa DNI 22.222.222 DNI 11.111.111. Firma Alba Ruiz.";
const RECORD: &str = "DOC-Q. Constancia de Alba Ruiz, DNI 11.111.111, vecina de la ciudad.";
const RECEIPT: &str = "DOC-R. Recibo de Carla Prueba, DNI 33.333.333, por el pago de marzo.";
const NOTE: &str = "DOC-S. Nota de Dora Paz, DNI 33.333.333, sobre el pago de marzo.";

fn people_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        let person = |r: &str, name: &str, quote: &str, ids: serde_json::Value| mention(r, "persona", name, "parte", quote, ids);
        if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            let candidates = prompt.split("Candidatos:\n").nth(1).unwrap_or_default();
            let answers: Vec<serde_json::Value> = (0..candidates.split("\n[").count())
                .map(|i| serde_json::json!({ "candidato": i + 1, "decision": "dudosa", "certeza": "alta", "razon": "no se sabe" }))
                .collect();
            return serde_json::json!({ "respuestas": answers }).to_string();
        }
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let mentions = if prompt.contains("DOC-P.") {
            // A form read without its structure: the holder's DNI given to
            // the co-holder too.
            vec![
                person("a", "Alba Ruiz", "Titular: Alba Ruiz", none()),
                person(
                    "b",
                    "Bruno Sosa",
                    "Cotitular: Bruno Sosa DNI 22.222.222 DNI 11.111.111",
                    ids(&[("dni", "22.222.222"), ("dni", "11.111.111")], "Bruno Sosa DNI 22.222.222 DNI 11.111.111"),
                ),
            ]
        } else if prompt.contains("DOC-Q.") {
            vec![person("a", "Alba Ruiz", "Constancia de Alba Ruiz, DNI 11.111.111", ids(&[("dni", "11.111.111")], "Alba Ruiz, DNI 11.111.111"))]
        } else if prompt.contains("DOC-R.") {
            vec![person("a", "Carla Prueba", "Recibo de Carla Prueba, DNI 33.333.333", ids(&[("dni", "33.333.333")], "Carla Prueba, DNI 33.333.333"))]
        } else if prompt.contains("DOC-S.") {
            vec![person("a", "Dora Paz", "Nota de Dora Paz, DNI 33.333.333", ids(&[("dni", "33.333.333")], "Dora Paz, DNI 33.333.333"))]
        } else {
            Vec::new()
        };
        serde_json::json!({ "menciones": mentions, "afirmaciones": [] }).to_string()
    })
}

fn run(base: &DocBase, asker: &FakeModel) {
    base.types_run(asker, 50).unwrap();
    while base.knowledge_run(asker, 50).unwrap().extracted > 0 {}
    base.knowledge_resolve(Some(asker), 50).unwrap();
}

fn entity_named(base: &DocBase, written: &str) -> String {
    base.conn().query_row("SELECT entity_id FROM knowledge_mention WHERE written = ?1 LIMIT 1", [written], |row| row.get(0)).unwrap()
}

#[test]
fn a_second_number_of_another_person_opens_no_doubt_and_a_doubt_shows_its_evidence() {
    let (_dir, base) = base_with(&[("p.txt", FORM), ("q.txt", RECORD), ("r.txt", RECEIPT), ("s.txt", NOTE)]);
    run(&base, &people_model());
    let doubts = base.knowledge_doubts(&Visible::All).unwrap();
    // Bruno Sosa had Alba Ruiz's DNI too: it is taken off him, as doubtful,
    // and the two are not put in doubt.
    let bruno = entity_named(&base, "Bruno Sosa");
    assert!(doubts.iter().all(|d| d.entity_a.as_deref() != Some(bruno.as_str()) && d.entity_b.as_deref() != Some(bruno.as_str())), "{doubts:?}");
    let alba: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT DISTINCT entity_id FROM knowledge_mention WHERE written = 'Alba Ruiz'").unwrap();
        statement.query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(alba.len(), 1, "the holder is one person");
    let detail = base.knowledge_entity(&Visible::All, &bruno).unwrap().unwrap();
    let shown: Vec<(String, bool)> = detail.identifiers.iter().map(|i| (i.value.clone(), i.doubtful)).collect();
    assert_eq!(shown, vec![("11.111.111".to_string(), true), ("22.222.222".to_string(), false)]);
    // Two names with one DNI stay in doubt, with the DNI and the documents.
    let shared: Vec<_> = doubts.iter().filter(|d| d.reason == "identificador-otro-nombre").collect();
    assert_eq!(shared.len(), 1, "{doubts:?}");
    assert_eq!(shared[0].detail.as_deref(), Some("dni: 33333333"));
    let mut documents = shared[0].documents.clone();
    documents.sort();
    assert_eq!(documents, vec!["r.txt".to_string(), "s.txt".to_string()]);
}

const HIRING: &str = "DOC-T. Ejemplo Uno S.A. (en adelante, la «Empresa») contrata a Eva Gó[ilegible], DNI 44.444.444. La Empresa \
paga el sueldo a Eva Gó[ilegible].";
const INVOICE: &str = "DOC-U. Factura de EJEMPLO UNO SA a Ana Gómez por servicios.";
const LETTER: &str = "DOC-V. Carta de Ejemplo Uno S.A. a Ana Gómez sobre la factura.";

fn list_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let org = |r: &str, name: &str, quote: &str| mention(r, "organizacion", name, "parte", quote, none());
        let said = if prompt.contains("DOC-T.") {
            serde_json::json!({
                "menciones": [
                    org("a", "Ejemplo Uno S.A.", "Ejemplo Uno S.A. (en adelante, la «Empresa»)"),
                    org("b", "Empresa", "La Empresa paga el sueldo"),
                    mention("c", "persona", "Eva Gó[ilegible]", "parte", "contrata a Eva Gó[ilegible], DNI 44.444.444",
                        ids(&[("dni", "44.444.444")], "Eva Gó[ilegible], DNI 44.444.444")),
                ],
                "definiciones": [{ "termino": "Empresa", "ref": "a", "cita": "Ejemplo Uno S.A. (en adelante, la «Empresa»)" }],
                "afirmaciones": []
            })
        } else if prompt.contains("DOC-U.") {
            serde_json::json!({ "menciones": [
                org("a", "EJEMPLO UNO SA", "Factura de EJEMPLO UNO SA a Ana Gómez"),
                mention("b", "persona", "Ana Gómez", "parte", "a Ana Gómez por servicios", none()),
            ], "afirmaciones": [] })
        } else if prompt.contains("DOC-V.") {
            serde_json::json!({ "menciones": [
                org("a", "Ejemplo Uno S.A.", "Carta de Ejemplo Uno S.A. a Ana Gómez"),
                mention("b", "persona", "Ana Gómez", "parte", "a Ana Gómez sobre la factura", none()),
            ], "afirmaciones": [] })
        } else {
            serde_json::json!({ "menciones": [], "afirmaciones": [] })
        };
        said.to_string()
    })
}

#[test]
fn the_lists_show_what_the_pages_name_and_what_has_a_page_first() {
    let (_dir, base) = base_with(&[("t.txt", HIRING), ("u.txt", INVOICE), ("v.txt", LETTER)]);
    let model = list_model();
    base.types_run(&model, 50).unwrap();
    while base.knowledge_run(&model, 50).unwrap().extracted > 0 {}
    base.knowledge_resolve(Some(&model), 50).unwrap();
    let company = entity_named(&base, "Ejemplo Uno S.A.");
    let misread = entity_named(&base, "Eva Gó[ilegible]");
    let listed = base.knowledge_entities(&Visible::All, None, None).unwrap();
    let names: Vec<&str> = listed.iter().map(|e| e.name.as_str()).collect();
    // A misread name is in the record, not in the list.
    assert!(listed.iter().all(|e| e.id != misread), "{names:?}");
    // The defined term is a local alias, not another name to show.
    let summary = listed.iter().find(|e| e.id == company).expect("the company is listed");
    assert!(!summary.aliases.iter().any(|a| a == "Empresa"), "{:?}", summary.aliases);
    let detail = base.knowledge_entity(&Visible::All, &company).unwrap().unwrap();
    assert!(detail.mentions.iter().any(|m| m.written == "Empresa"), "the term is a mention of the company");
    assert!(!detail.aliases.iter().any(|a| a.written == "Empresa"), "{:?}", detail.aliases);
    // The company, in more documents, goes first; with a page, Ana Gómez does.
    let ana = entity_named(&base, "Ana Gómez");
    assert_eq!(listed.first().map(|e| e.id.as_str()), Some(company.as_str()), "{names:?}");
    base.conn()
        .execute("INSERT INTO knowledge_page (entity_id, path, state, fingerprint, written_at) VALUES (?1, 'wiki/ana.md', 'projected', 'f', 1)", [&ana])
        .unwrap();
    let listed = base.knowledge_entities(&Visible::All, None, None).unwrap();
    assert_eq!(listed.first().map(|e| e.id.as_str()), Some(ana.as_str()));
}
