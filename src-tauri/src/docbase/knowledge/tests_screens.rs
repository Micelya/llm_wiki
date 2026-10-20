//! What the screens and the attention view read of the record, over a real
//! base, with invented documents and a stand-in model (R-24: nothing here
//! comes from the test folder).

use super::decide::IdentityDecision;
use super::tests::{base_with, claim, is_knowledge, location, mention, type_answer, FakeModel};
use super::tests_relations::{archive, content, matters};
use crate::docbase::Visible;

#[test]
fn the_attention_view_gets_doubts_joins_by_identifier_and_progress() {
    let (_dir, base) = base_with(super::tests_identity::FILES);
    let asker = super::tests_identity::model();
    super::tests_identity::run(&base, &asker);
    let attention = base.knowledge_attention(&Visible::All).unwrap();
    assert_eq!(attention.status.documents, 6);
    assert_eq!(attention.status.pending, 0);
    assert!(attention.status.doubts > 0, "a DNI that differs by one digit is a doubt");
    // The DNI joined two ways of writing the person, read just now.
    let joined = attention.identifier_joins.iter().find(|j| j.kind == "persona").expect("a join by identifier");
    assert!(joined.names.len() >= 2 && joined.identifiers.iter().any(|i| i.contains("22.222.222")), "{joined:?}");
    assert!(attention.failed.is_empty() && attention.noisy.is_empty() && attention.conflicts.is_empty());
}

const H: &str = "DOC-H. Constancia de Persona Seis, DNI 33.333.333, con domicilio en Calle Tres 300, para el trámite de \
inscripción que corresponde a este año.";
const J: &str = "DOC-J. Nota firmada por Persona Seis, DNI 33.333.333, con domicilio en Calle Cuatro 400, para el mismo \
trámite de inscripción de este año.";
const K: &str = "DOC-K. Un escaneo con el texto mal reconocido de una carta que casi no se puede leer en ninguna de \
sus partes ni en su firma.";
const M: &str = "DOC-M. Un documento cualquiera para el que el modelo responde algo que no tiene la forma pedida en \
la pregunta de lectura.";

fn problems_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let none = serde_json::json!([]);
        let ids = |v: &str| serde_json::json!([{ "clase": "dni", "valor": v, "cita": format!("DNI {v}") }]);
        if prompt.contains("DOC-H.") {
            serde_json::json!({
                "menciones": [mention("p", "persona", "Persona Seis", "parte", "Constancia de Persona Seis, DNI 33.333.333", ids("33.333.333"))],
                "afirmaciones": [claim("p", "domicilio", None, Some("Calle Tres 300"), "con domicilio en Calle Tres 300, para el trámite de inscripción que corresponde a este año", Some("este año"))]
            })
            .to_string()
        } else if prompt.contains("DOC-J.") {
            serde_json::json!({
                "menciones": [mention("p", "persona", "Persona Seis", "firmante", "firmada por Persona Seis, DNI 33.333.333", ids("33.333.333"))],
                "afirmaciones": [claim("p", "domicilio", None, Some("Calle Cuatro 400"), "con domicilio en Calle Cuatro 400, para el mismo trámite de inscripción de este año", Some("este año"))]
            })
            .to_string()
        } else if prompt.contains("DOC-K.") {
            // Quotes the text does not have: a badly recognized scan.
            let lost: Vec<serde_json::Value> = (0..5)
                .map(|i| mention(&format!("x{i}"), "persona", &format!("Nombre Raro {i}"), "parte", &format!("firmado por Nombre Raro {i}"), none.clone()))
                .collect();
            let mut all = lost;
            all.push(mention("c", "organizacion", "Una Carta", "otro", "texto mal reconocido de una carta", none.clone()));
            serde_json::json!({ "menciones": all, "afirmaciones": [] }).to_string()
        } else if prompt.contains("DOC-M.") {
            serde_json::json!({ "otra": "cosa", "menciones": "no es una lista" }).to_string()
        } else {
            super::tests::empty_answer()
        }
    })
}

#[test]
fn failed_and_noisy_readings_and_conflicts_reach_the_attention_view() {
    let (_dir, base) = base_with(&[("h.txt", H), ("j.txt", J), ("k.txt", K), ("m.txt", M)]);
    let asker = problems_model();
    super::tests_pages::read(&base, &asker);
    let attention = base.knowledge_attention(&Visible::All).unwrap();
    assert_eq!(attention.failed.len(), 1, "{:?}", attention.failed);
    assert_eq!(attention.failed[0].document.name, "m.txt");
    assert!(attention.failed[0].error.is_some());
    assert_eq!(attention.noisy.len(), 1, "{:?}", attention.noisy);
    assert_eq!((attention.noisy[0].document.name.as_str(), attention.noisy[0].discarded), ("k.txt", 5));
    // Two domiciles for the same time, from the person the DNI joined.
    assert_eq!(attention.conflicts.len(), 1, "{:?}", attention.conflicts);
    let conflict = &attention.conflicts[0];
    assert_eq!((conflict.entity_name.as_str(), conflict.predicate.as_str(), conflict.claims.len()), ("Persona Seis", "domicilio", 2));
    assert!(conflict.values.iter().any(|v| v.contains("Tres")) && conflict.values.iter().any(|v| v.contains("Cuatro")));

    // "This datum is wrong" answers the conflict.
    base.knowledge_decide(&IdentityDecision::Wrong { a: format!("claim:{}", conflict.claims[0]) }, None, None).unwrap();
    assert!(base.knowledge_attention(&Visible::All).unwrap().conflicts.is_empty());

    // The entity's detail shows the decision, to undo it.
    let detail = base.knowledge_entity(&Visible::All, &conflict.entity_id).unwrap().unwrap();
    assert_eq!(detail.decisions.len(), 1);
    assert_eq!(detail.decisions[0].kind, "wrong");
    base.knowledge_undo(detail.decisions[0].id).unwrap();
    assert_eq!(base.knowledge_attention(&Visible::All).unwrap().conflicts.len(), 1);
}

#[test]
fn a_matter_shows_its_documents_why_its_parties_and_the_proposal() {
    let (_dir, base, _project, _asker) = archive();
    let list = base.knowledge_matters(&Visible::All).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!((list[0].documents, list[0].proposed), (3, 1));
    assert!(list[0].page_path.is_some());
    let detail = base.knowledge_matter(&Visible::All, &list[0].id).unwrap().unwrap();
    let mut firm: Vec<&str> = detail.documents.iter().filter(|d| d.state == "firm").map(|d| d.document.name.as_str()).collect();
    firm.sort();
    assert_eq!(firm, vec!["f.txt", "l.txt", "r.txt"]);
    let proposed: Vec<&str> = detail.documents.iter().filter(|d| d.state == "proposed").map(|d| d.document.name.as_str()).collect();
    assert_eq!(proposed, vec!["p.txt"]);
    let kinds: Vec<&str> = detail.documents.iter().flat_map(|d| d.reasons.iter().map(|r| r.kind.as_str())).collect();
    assert!(kinds.contains(&"numero") && kinds.contains(&"importe"), "{kinds:?}");
    assert!(detail.documents.iter().flat_map(|d| &d.reasons).all(|r| r.with.is_some()));
    let parties: Vec<&str> = detail.parties.iter().map(|p| p.name.as_str()).collect();
    assert!(parties.contains(&"Persona Dos") && parties.contains(&"Ejemplo Uno S.A."), "{parties:?}");
    assert!(!detail.parties.iter().any(|p| p.kind == "asunto"));

    let attention = base.knowledge_attention(&Visible::All).unwrap();
    assert_eq!(attention.proposed_matters.len(), 1);
    assert_eq!(attention.proposed_matters[0].document.name, "p.txt");
}

#[test]
fn a_proposal_is_confirmed_or_turned_down_and_a_document_taken_out() {
    let (_dir, base, _project, _asker) = archive();
    let matter = base.knowledge_matters(&Visible::All).unwrap()[0].id.clone();
    let p = location(&base, "p.txt");
    // Confirmed: the mail is in the matter, and no longer a proposal.
    let joined = base.knowledge_join_matter(&format!("location:{p}"), &matter, None).unwrap();
    assert_eq!(matters(&base)[0].len(), 4);
    assert!(base.knowledge_attention(&Visible::All).unwrap().proposed_matters.is_empty());
    // Undone, it is a proposal again.
    base.knowledge_undo(joined).unwrap();
    assert_eq!(matters(&base)[0].len(), 3);
    assert_eq!(base.knowledge_attention(&Visible::All).unwrap().proposed_matters.len(), 1);
    // Turned down: kept apart from the matter, the proposal is answered.
    let apart = base.knowledge_leave_matter(&format!("location:{p}"), &matter, Some("no es de este asunto")).unwrap();
    assert_eq!(apart.len(), 3);
    assert!(base.knowledge_attention(&Visible::All).unwrap().proposed_matters.is_empty());
    assert_eq!(base.knowledge_matters(&Visible::All).unwrap()[0].proposed, 0);
    for id in apart {
        base.knowledge_undo(id).unwrap();
    }
    // A document taken out of the matter stays out, whatever it shares.
    let r = content(&base, "r.txt");
    base.knowledge_leave_matter(&format!("content:{r}"), &matter, None).unwrap();
    assert_eq!(matters(&base), vec![vec!["f.txt".to_string(), "l.txt".to_string()]]);
    let detail = base.knowledge_matter(&Visible::All, &matters_id(&base)).unwrap().unwrap();
    assert_eq!(detail.decisions.iter().filter(|d| d.state == "active").count(), 2);
}

fn matters_id(base: &crate::docbase::DocBase) -> String {
    base.knowledge_matters(&Visible::All).unwrap()[0].id.clone()
}

#[test]
fn a_document_shows_whom_it_names_with_their_roles_and_its_matter() {
    let (_dir, base, _project, _asker) = archive();
    let found = base.knowledge_document_entities(&Visible::All, location(&base, "l.txt")).unwrap();
    assert_eq!(found.state, "done");
    let named: Vec<(&str, &str)> = found.entities.iter().map(|e| (e.name.as_str(), e.roles[0].as_str())).collect();
    assert!(named.contains(&("Persona Dos", "destinatario")), "{named:?}");
    assert!(named.contains(&("Ejemplo Uno S.A.", "remitente")), "{named:?}");
    assert!(named.contains(&("Persona Nueve", "firmante")), "{named:?}");
    assert!(found.entities.iter().all(|e| e.entity_id.is_some()));
    // The invoice it refers to is a reference of the document, with its
    // quote and its number, not an entity (D-377).
    assert!(!found.entities.iter().any(|e| e.kind == "asunto"), "{:?}", found.entities);
    assert_eq!(found.references.len(), 1, "{:?}", found.references);
    let invoice = &found.references[0];
    assert_eq!(invoice.written, "factura 0001-00012345");
    assert_eq!(invoice.quotes[0].quote, "por la factura 0001-00012345");
    assert!(invoice.identifiers.iter().any(|i| i.value == "0001-00012345"), "{:?}", invoice.identifiers);
    assert!(base.knowledge_entities(&Visible::All, None, None).unwrap().iter().all(|e| e.kind != "asunto"));
    assert_eq!(found.matters.len(), 1);
    assert!(!found.matters[0].proposed && found.matters[0].page_path.is_some());
    // The mail's matter is a proposal.
    let mail = base.knowledge_document_entities(&Visible::All, location(&base, "p.txt")).unwrap();
    assert_eq!(mail.matters.len(), 1);
    assert!(mail.matters[0].proposed);
}

#[test]
fn an_entity_shows_its_page_its_matters_and_who_speaks_of_it() {
    let (_dir, base, project, _asker) = archive();
    let company = base.knowledge_entities(&Visible::All, Some("organizacion"), Some("ejemplo uno")).unwrap();
    let detail = base.knowledge_entity(&Visible::All, &company[0].id).unwrap().unwrap();
    assert!(detail.page_path.as_deref().is_some_and(|p| p.starts_with("wiki/entities/")), "{:?}", detail.page_path);
    // Its article is its page as the projection wrote it, and a link to
    // that page leads to the entity (D-389).
    let article = base.knowledge_entity_article(&detail.id).unwrap().unwrap();
    assert!(article.text.contains("title: \"Ejemplo Uno S.A.\""), "{}", article.text);
    let file = project.join(detail.page_path.as_deref().unwrap());
    assert_eq!(base.knowledge_entity_of_page(&file.to_string_lossy()).unwrap().as_deref(), Some(detail.id.as_str()));
    assert_eq!(base.knowledge_entity_of_page(&project.join("wiki/overview.md").to_string_lossy()).unwrap(), None);
    assert_eq!(detail.matters.iter().filter(|m| !m.proposed).count(), 3);
    assert!(detail.incoming.iter().any(|c| c.predicate == "representa_a" && c.subject_name == "Persona Nueve"), "{:?}", detail.incoming);
    // A mention taken out of it is a decision on it, to undo.
    let mention = detail.mentions.iter().find(|m| m.document == "o.txt").unwrap().mention_id;
    let id = base
        .knowledge_decide(&IdentityDecision::Separate { a: format!("mention:{mention}"), b: format!("entity:{}", detail.id) }, None, None)
        .unwrap();
    let after = base.knowledge_entity(&Visible::All, &detail.id).unwrap().unwrap();
    assert!(!after.mentions.iter().any(|m| m.document == "o.txt"));
    assert_eq!(after.decisions.iter().map(|d| d.id).collect::<Vec<_>>(), vec![id]);
}

#[test]
fn a_reference_resolved_into_an_entity_before_lets_go_of_it() {
    let (_dir, base, _project, _asker) = archive();
    {
        let conn = base.conn();
        conn.execute(
            "INSERT INTO knowledge_entity (id, kind, name, state, created_at, updated_at) VALUES ('ent-vieja', 'asunto', 'Factura vieja', 'active', 0, 0)",
            [],
        )
        .unwrap();
        conn.execute("UPDATE knowledge_mention SET entity_id = 'ent-vieja', resolved_by = 'nombre' WHERE kind = 'asunto'", []).unwrap();
        // A base resolved by the program of before (D-377).
        conn.execute("DELETE FROM docbase_setting WHERE key = 'knowledge-resolution'", []).unwrap();
    }
    base.knowledge_resolve(None, 0).unwrap();
    let conn = base.conn();
    let state: String = conn.query_row("SELECT state FROM knowledge_entity WHERE id = 'ent-vieja'", [], |row| row.get(0)).unwrap();
    assert_eq!(state, "retired");
    let still: i64 = conn
        .query_row("SELECT count(*) FROM knowledge_mention WHERE kind = 'asunto' AND entity_id IS NOT NULL", [], |row| row.get(0))
        .unwrap();
    assert_eq!(still, 0);
    let people: i64 = conn
        .query_row("SELECT count(*) FROM knowledge_mention WHERE kind != 'asunto' AND entity_id IS NOT NULL", [], |row| row.get(0))
        .unwrap();
    assert!(people > 0);
}
