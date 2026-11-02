//! The data given the other way round, and the copy that takes the reading
//! of its original (E6 of 2 nov): the labels of a parent in a form, two
//! relations that contradict each other, a value far from its subject, the
//! title of a document with one party, the role of whoever sends it, and a
//! copy found by the search of similar documents that is not read again.
//! Invented texts and a stand-in model (R-24): nothing here comes from the
//! test folder or from an answer kept by another run.

use super::extract::{verify, Answer, Reading, CORRECTION, FAR_FROM_SUBJECT};
use super::tests::{answer, base_with, claim, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use super::tests_pages::{entity_pages, page_of, read};
use super::vocabulary::Vocabulary;
use crate::docbase::Visible;

fn none() -> serde_json::Value {
    serde_json::json!([])
}

fn reading_of(body: &str, said: String) -> Reading {
    let answer: Answer = serde_json::from_str(&said).unwrap();
    verify(&Vocabulary::builtin(), body, &[], &[answer])
}

fn reasons(reading: &Reading) -> Vec<String> {
    reading.discarded.iter().map(|d| d.reason.clone()).collect()
}

fn name_of(reading: &Reading, reference: &str) -> String {
    reading.mentions.iter().find(|m| m.reference == reference).map(|m| m.written.clone()).unwrap_or_default()
}

const FORM: &str = "FICHA DE PRUEBA\nNombre: PERSONA UNO EJEMPLO\nDNI 11.111.111\nApellido y Nombre del Padre: EJEMPLO PERSONA DOS\n\
DNI del padre: 22.222.222\nFecha: 1 de marzo de 2020\n";

/// «Nombre del padre: X» makes X the parent, though the quote does not say
/// «hijo de», and the number under «DNI del padre:» is his; the number
/// before the label stays with the person of the form.
#[test]
fn a_label_of_a_parent_makes_the_name_under_it_the_parent_and_its_data_his() {
    let reading = reading_of(
        FORM,
        answer(
            vec![
                mention(
                    "a",
                    "persona",
                    "PERSONA UNO EJEMPLO",
                    "parte",
                    "Nombre: PERSONA UNO EJEMPLO",
                    serde_json::json!([
                        { "clase": "dni", "valor": "11.111.111", "cita": "Nombre: PERSONA UNO EJEMPLO\nDNI 11.111.111\nApellido y Nombre del Padre: EJEMPLO PERSONA DOS", "rotulo": null },
                        { "clase": "dni", "valor": "22.222.222", "cita": "Apellido y Nombre del Padre: EJEMPLO PERSONA DOS\nDNI del padre: 22.222.222", "rotulo": null },
                    ]),
                ),
                mention("b", "persona", "EJEMPLO PERSONA DOS", "otro", "Apellido y Nombre del Padre: EJEMPLO PERSONA DOS", none()),
            ],
            // The model gave the person of the form as the parent of her father.
            vec![claim("a", "progenitor_de", Some("b"), None, "Nombre: PERSONA UNO EJEMPLO\nDNI 11.111.111\nApellido y Nombre del Padre: EJEMPLO PERSONA DOS", None)],
        ),
    );
    let kept: Vec<(String, String, String)> = reading
        .evidence
        .iter()
        .filter(|e| e.object_mention.is_some())
        .map(|e| (name_of(&reading, &e.subject), e.predicate.clone(), name_of(&reading, e.object_mention.as_deref().unwrap())))
        .collect();
    assert_eq!(
        kept,
        [("EJEMPLO PERSONA DOS".to_string(), "progenitor_de".to_string(), "PERSONA UNO EJEMPLO".to_string())],
        "{:?}",
        reading.discarded
    );
    assert!(reading.discarded.iter().any(|d| d.what == CORRECTION && d.reason == "filiacion-por-rotulo"), "{:?}", reading.discarded);
    let numbers = |written: &str| -> Vec<String> {
        reading.mentions.iter().find(|m| m.written == written).unwrap().identifiers.iter().map(|i| i.normalized.clone()).collect()
    };
    assert_eq!(numbers("PERSONA UNO EJEMPLO"), ["11111111"]);
    assert_eq!(numbers("EJEMPLO PERSONA DOS"), ["22222222"]);
}

const INVOICE: &str = "FACTURA DE PRUEBA\nEjemplo Uno S.A. emite la presente factura.\nCliente: Ejemplo Dos S.R.L.\n\
Domicilio: Calle Inventada 123\nImporte total: diez pesos.\n";

/// A value whose subject the text does not name near its quote is kept in
/// doubt, not as a fact: the address under the client given to the company
/// that wrote the invoice. The same address given to the client is a fact.
#[test]
fn a_value_whose_subject_is_not_near_its_quote_is_in_doubt() {
    let said = |subject: &str| {
        answer(
            vec![
                mention("e", "organizacion", "Ejemplo Uno S.A.", "emisor", "Ejemplo Uno S.A. emite la presente factura", none()),
                mention("c", "organizacion", "Ejemplo Dos S.R.L.", "destinatario", "Cliente: Ejemplo Dos S.R.L.", none()),
            ],
            vec![claim(subject, "domicilio", None, Some("Calle Inventada 123"), "Domicilio: Calle Inventada 123", None)],
        )
    };
    let far = reading_of(INVOICE, said("e"));
    assert_eq!(far.evidence.len(), 1, "kept: {:?}", far.discarded);
    let detail = far.evidence[0].detail.clone().unwrap_or_default();
    assert_eq!(detail.get(FAR_FROM_SUBJECT).and_then(|v| v.as_bool()), Some(true), "{detail:?}");
    assert!(reasons(&far).contains(&"sujeto-lejos".to_string()));
    let near = reading_of(INVOICE, said("c"));
    assert_eq!(near.evidence.len(), 1);
    assert!(near.evidence[0].detail.as_ref().is_none_or(|d| d.get(FAR_FROM_SUBJECT).is_none()), "{:?}", near.evidence[0].detail);

    // In a base, the claim is doubtful for that reason, and its page says so.
    let (_dir, base) = base_with(&[("factura.txt", INVOICE)]);
    let model = FakeModel::new("prueba:modelo", move |prompt| if is_knowledge(prompt) { said("e") } else { type_answer("factura") });
    base.types_run(&model, 10).unwrap();
    base.knowledge_run(&model, 10).unwrap();
    base.knowledge_resolve(None, 0).unwrap();
    let certainty: String = base
        .conn()
        .query_row("SELECT certainty FROM knowledge_claim WHERE predicate = 'domicilio'", [], |row| row.get(0))
        .unwrap();
    assert!(certainty.contains("\"doubtful\"") && certainty.contains("\"far\""), "{certainty}");
}

const SON: &str = "ESCRITO DE PRUEBA\nPersona Tres Ejemplo, hijo de Persona Cuatro Ejemplo, se presenta por derecho propio.\n";
const FATHER: &str = "NOTA DE PRUEBA\nPersona Cuatro Ejemplo, hijo de Persona Tres Ejemplo, firma la presente.\n";

fn kinship_model() -> FakeModel {
    FakeModel::new("prueba:modelo", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("escrito");
        }
        let people = |quote3: &str, quote4: &str| {
            vec![
                mention("p3", "persona", "Persona Tres Ejemplo", "parte", quote3, serde_json::json!([])),
                mention("p4", "persona", "Persona Cuatro Ejemplo", "otro", quote4, serde_json::json!([])),
            ]
        };
        if prompt.contains("se presenta por derecho propio") {
            let quote = "Persona Tres Ejemplo, hijo de Persona Cuatro Ejemplo";
            answer(people(quote, quote), vec![claim("p4", "progenitor_de", Some("p3"), None, quote, None)])
        } else if prompt.contains("firma la presente") {
            let quote = "Persona Cuatro Ejemplo, hijo de Persona Tres Ejemplo";
            answer(people(quote, quote), vec![claim("p3", "progenitor_de", Some("p4"), None, quote, None)])
        } else {
            empty_answer()
        }
    })
}

/// A parent of B in one document and B parent of A in another: both claims
/// are in conflict, the pages list them apart and never as a fact, and
/// Atención shows them as one decision for a person.
#[test]
fn two_relations_said_the_other_way_round_are_in_conflict_and_no_fact_of_the_pages() {
    let (dir, base) = base_with(&[("hijo.txt", SON), ("padre.txt", FATHER)]);
    dir.folder("General/wiki/sources");
    dir.write("General/schema.md", b"# schema");
    let project = dir.path().join("General");
    crate::docbase::testing::point_general_project_at(&base, &project);
    let model = kinship_model();
    read(&base, &model);
    let states: Vec<(String, String)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT c.state, c.certainty FROM knowledge_claim c WHERE c.predicate = 'progenitor_de' ORDER BY c.id")
            .unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(states.len(), 2, "{states:?}");
    assert!(states.iter().all(|(state, certainty)| state == "conflict" && certainty.contains("backwards")), "{states:?}");
    base.knowledge_project(Some(&model), 50).unwrap();
    let pages = entity_pages(&base);
    let page = page_of(&pages, "Persona Tres Ejemplo");
    let relations = page.split("## Vínculos").nth(1).expect("its relations").split("\n## ").next().unwrap();
    let (facts, contested) = relations.split_once("**En conflicto").expect("listed apart");
    assert!(!facts.contains("Padre o madre de") && !facts.contains("Hijo o hija de"), "{relations}");
    assert!(contested.contains("Padre o madre de") && contested.contains("Hijo o hija de"), "{relations}");
    let attention = base.knowledge_attention(&Visible::All).unwrap();
    assert_eq!(attention.conflicts.len(), 1, "{:?}", attention.conflicts);
    assert_eq!(attention.conflicts[0].values.len(), 2, "{:?}", attention.conflicts);
}

/// A copy the search of similar documents finds (almost the same text) is
/// not read again: it takes the reading of its original, checked against
/// its own text. When a person says they are not the same, it is read.
#[test]
fn a_copy_by_similarity_takes_the_reading_of_its_original_and_asks_nothing() {
    let lease = "CONTRATO DE LOCACIÓN DE PRUEBA\n\nEntre Ejemplo Ocho S.A., en adelante la locadora, y Persona Nueve, en adelante el \
locatario, se celebra el presente contrato de locación de un depósito inventado, por el plazo de dos años, con un precio \
mensual que las partes acuerdan por separado y que se actualiza cada seis meses según el índice pactado.";
    let copy = format!("{lease}\nCopia escaneada.");
    let (_dir, base) = base_with(&[("a.txt", lease), ("b.txt", &copy)]);
    let version = |name: &str| -> i64 {
        base.conn()
            .query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| row.get(0))
            .unwrap()
    };
    let (a, b) = (version("a.txt"), version("b.txt"));
    // The search of similar documents proposes them as almost the same.
    base.conn()
        .execute("UPDATE similar_pair SET relation = 'near-identical' WHERE (a = ?1 AND b = ?2) OR (a = ?2 AND b = ?1)", [a.min(b), a.max(b)])
        .unwrap();
    let pairs: i64 = base.conn().query_row("SELECT count(*) FROM similar_pair WHERE relation = 'near-identical'", [], |row| row.get(0)).unwrap();
    if pairs == 0 {
        base.conn()
            .execute(
                "INSERT INTO similar_pair (a, b, relation, certainty, jaccard, containment, cover, similarity, description, evidence,
                     evidence_hash, engine, computed_at)
                 VALUES (?1, ?2, 'near-identical', 'alta', 0.95, 0.95, 0.95, 0.95, 'prueba', '[]', 'h', 'prueba', 0)",
                [a.min(b), a.max(b)],
            )
            .unwrap();
    }
    let model = FakeModel::new("prueba:modelo", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        answer(vec![mention("p", "persona", "Persona Nueve", "parte", "y Persona Nueve, en adelante el", serde_json::json!([]))], vec![])
    });
    base.types_run(&model, 10).unwrap();
    let report = base.knowledge_run(&model, 10).unwrap();
    assert_eq!(model.knowledge_calls(), 1, "only the original is read");
    assert_eq!(report.inherited, 1);
    let read_as: (String, String) =
        base.conn().query_row("SELECT state, basis FROM knowledge_extraction WHERE version_id = ?1", [b], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(read_as.0, "done");
    assert!(read_as.1.starts_with(&format!("copia:{a}|")), "{read_as:?}");
    let mentions: i64 =
        base.conn().query_row("SELECT count(*) FROM knowledge_mention WHERE version_id = ?1", [b], |row| row.get(0)).unwrap();
    assert_eq!(mentions, 1, "the copy has the reading of its original");
    // Nothing more to ask.
    assert_eq!(base.knowledge_plan().unwrap().questions, 0);
    base.knowledge_run(&model, 10).unwrap();
    assert_eq!(model.knowledge_calls(), 1);
    // A person says they are not the same: the copy is read as itself.
    base.conn().execute("UPDATE similar_pair SET state = 'discarded' WHERE relation = 'near-identical'", []).unwrap();
    base.types_run(&model, 10).unwrap();
    let again = base.knowledge_run(&model, 10).unwrap();
    assert_eq!(again.waiting_for_type, 0, "{again:?}");
    assert_eq!(model.knowledge_calls(), 2, "read as itself once it is not a copy");
}

/// One party alone is not who a document is from; a sender the quote does
/// not say is in doubt.
#[test]
fn a_document_with_one_party_is_not_titled_as_from_it_and_a_sender_needs_its_quote() {
    let letter = "CARTA DE PRUEBA\nSeñores Ejemplo Tres S.A.\nPor la presente rechazamos la factura inventada.\nRemitente: Ejemplo Cuatro S.A.\n";
    let reading = reading_of(
        letter,
        answer(
            vec![
                mention("d", "organizacion", "Ejemplo Tres S.A.", "remitente", "Señores Ejemplo Tres S.A.", none()),
                mention("r", "organizacion", "Ejemplo Cuatro S.A.", "remitente", "Remitente: Ejemplo Cuatro S.A.", none()),
            ],
            vec![],
        ),
    );
    let role = |written: &str| reading.mentions.iter().find(|m| m.written == written).map(|m| m.role.clone()).unwrap();
    assert_eq!(role("Ejemplo Tres S.A."), "otro", "the quote does not say it sends it");
    assert_eq!(role("Ejemplo Cuatro S.A."), "remitente", "under «Remitente:»");
    assert!(reasons(&reading).contains(&"remitente-sin-cita".to_string()));
}
