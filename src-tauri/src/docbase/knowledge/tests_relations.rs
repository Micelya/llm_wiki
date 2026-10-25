//! Matters, positions and relations, over a real base, with invented
//! documents and a stand-in model (R-24: nothing here comes from the test
//! folder).

use std::path::PathBuf;

use super::decide::IdentityDecision;
use super::tests::{answer, amount, base_with, claim, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use super::tests_pages::{entity_pages, is_synthesis, note_after, object_in, page_of, read, synthesis};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};

const L: &str = "DOC-L. Carta documento de Ejemplo Uno S.A. a Persona Dos por la factura 0001-00012345, firmada por \
Persona Nueve, presidente de Ejemplo Uno S.A.";
const F: &str = "DOC-F. Factura 0001-00012345 de Ejemplo Uno S.A. a Persona Dos por $ 1.234,56 por servicios.";
const R: &str = "DOC-R. Recibo de Persona Dos a Ejemplo Uno S.A. por $ 1.234,56 en efectivo.";
const O: &str = "DOC-O. Nota de Persona Dos a Ejemplo Uno S.A. sobre otro tema, por $ 10.000 de anticipo.";
const G: &str = "DOC-G. Acta de Otra Empresa S.R.L. firmada por Persona Nueve, presidente de Otra Empresa S.R.L.";

const P: &str = "DOC-P. Correo de Persona Dos a Ejemplo Uno S.A. para coordinar el pago pendiente que reclama la carta.";

const FILES: &[(&str, &str)] = &[("l.txt", L), ("f.txt", F), ("r.txt", R), ("o.txt", O), ("g.txt", G), ("p.txt", P)];

fn ids(kind: &str, value: &str, quote: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": kind, "valor": value, "cita": quote }])
}

fn reading(prompt: &str) -> String {
    let none = serde_json::json!([]);
    let company = |role: &str, quote: &str| mention("o", "organizacion", "Ejemplo Uno S.A.", role, quote, serde_json::json!([]));
    let two = |role: &str, quote: &str| mention("p", "persona", "Persona Dos", role, quote, serde_json::json!([]));
    let (mentions, claims) = if prompt.contains("DOC-L.") {
        (
            vec![
                company("remitente", "Carta documento de Ejemplo Uno S.A."),
                two("destinatario", "a Persona Dos por la factura"),
                mention("x", "asunto", "factura 0001-00012345", "referido", "por la factura 0001-00012345", ids("otro", "0001-00012345", "factura 0001-00012345")),
                mention("n", "persona", "Persona Nueve", "firmante", "firmada por Persona Nueve", none.clone()),
            ],
            vec![
                claim("n", "cargo", None, Some("presidente"), "Persona Nueve, presidente de Ejemplo Uno S.A.", None),
                claim("n", "representa_a", Some("o"), None, "firmada por Persona Nueve, presidente de Ejemplo Uno S.A.", None),
            ],
        )
    } else if prompt.contains("DOC-F.") {
        (
            vec![
                mention("x", "asunto", "Factura 0001-00012345", "referido", "Factura 0001-00012345 de Ejemplo Uno S.A.", ids("otro", "0001-00012345", "Factura 0001-00012345")),
                company("emisor", "de Ejemplo Uno S.A. a Persona Dos"),
                two("destinatario", "a Persona Dos por $ 1.234,56"),
            ],
            vec![amount("p", "o", "$ 1.234,56", "factura", "a Persona Dos por $ 1.234,56", None)],
        )
    } else if prompt.contains("DOC-R.") {
        (
            vec![two("emisor", "Recibo de Persona Dos"), company("destinatario", "a Ejemplo Uno S.A. por $ 1.234,56")],
            vec![amount("p", "o", "$ 1.234,56", "cuota", "a Ejemplo Uno S.A. por $ 1.234,56", None)],
        )
    } else if prompt.contains("DOC-O.") {
        (
            vec![two("remitente", "Nota de Persona Dos"), company("destinatario", "a Ejemplo Uno S.A. sobre otro tema")],
            vec![amount("p", "o", "$ 10.000", "cuota", "por $ 10.000 de anticipo", None)],
        )
    } else if prompt.contains("DOC-P.") {
        (
            vec![two("remitente", "Correo de Persona Dos"), company("destinatario", "a Ejemplo Uno S.A. para coordinar")],
            // The same tie said from both sides.
            vec![
                claim("p", "deudor_de", Some("o"), None, "Persona Dos a Ejemplo Uno S.A. para coordinar el pago pendiente", None),
                claim("p", "deudor_de", Some("o"), None, "Correo de Persona Dos a Ejemplo Uno S.A. para coordinar el pago pendiente", None),
            ],
        )
    } else if prompt.contains("DOC-G.") {
        (
            vec![
                mention("e", "organizacion", "Otra Empresa S.R.L.", "parte", "Acta de Otra Empresa S.R.L.", none.clone()),
                mention("n", "persona", "Persona Nueve", "firmante", "firmada por Persona Nueve", none.clone()),
            ],
            vec![claim("n", "cargo", None, Some("presidente"), "Persona Nueve, presidente de Otra Empresa S.R.L.", None)],
        )
    } else {
        return empty_answer();
    };
    answer(mentions, claims)
}

pub(crate) fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            synthesis(prompt)
        } else if prompt.contains("Hay que decidir si un documento del archivo es parte de alguno de estos asuntos") {
            // The mail is about the letter's claim; the note is not.
            let chosen = if prompt.contains("Empieza así: «DOC-P.") { 1 } else { 0 };
            serde_json::json!({ "asunto": chosen, "certeza": "media", "razon": "por los hechos" }).to_string()
        } else if is_knowledge(prompt) {
            reading(prompt)
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

pub(crate) fn archive() -> (TempDir, DocBase, PathBuf, FakeModel) {
    let (dir, base) = base_with(FILES);
    dir.folder("General/wiki/sources");
    dir.write("General/schema.md", b"# schema");
    let project = dir.path().join("General");
    crate::docbase::testing::point_general_project_at(&base, &project);
    let asker = model();
    read(&base, &asker);
    // The model proposes loose documents only for a matter a person
    // confirmed (D-381).
    let ids: Vec<String> = base.knowledge_matters(&crate::docbase::Visible::All).unwrap().into_iter().map(|m| m.id).collect();
    for id in ids {
        base.knowledge_decide_matter(&id, "confirm").unwrap();
    }
    base.knowledge_propose_matters(Some(&asker), 50).unwrap();
    base.knowledge_project(Some(&asker), 50).unwrap();
    (dir, base, project, asker)
}

pub(crate) fn content(base: &DocBase, file: &str) -> i64 {
    base.knowledge_of(&Visible::All, super::tests::location(base, file)).unwrap().content_id.unwrap()
}

/// The matters, as sets of file names.
pub(crate) fn matters(base: &DocBase) -> Vec<Vec<String>> {
    let conn = base.conn();
    let mut statement = conn
        .prepare(
            "SELECT d.matter_id, l.name FROM knowledge_matter_document d JOIN knowledge_matter m ON m.id = d.matter_id
             JOIN file f ON f.content_id = d.content_id JOIN location l ON l.file_hash = f.hash
             WHERE m.state = 'active' ORDER BY d.matter_id, l.name",
        )
        .unwrap();
    let rows: Vec<(String, String)> = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
    let mut out: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for (matter, name) in rows {
        out.entry(matter).or_default().push(name);
    }
    let mut list: Vec<Vec<String>> = out.into_values().collect();
    list.sort();
    list
}

#[test]
fn documents_of_one_matter_go_together_by_what_is_specific_to_it() {
    let (_dir, base, _project, _asker) = archive();
    // The letter names the invoice by its number; the receipt pays its
    // amount. Sharing the person and the company is not a matter (R-05);
    // nor is a round amount.
    assert_eq!(matters(&base), vec![vec!["f.txt".to_string(), "l.txt".to_string(), "r.txt".to_string()]]);
    let pages = entity_pages(&base);
    let matter = pages.iter().find(|(_, t)| t.contains("entity_id: mat-")).map(|(_, t)| t.as_str()).expect("a page of the matter");
    assert!(matter.contains("entity_kind: asunto") && matter.contains("## Documentos (3)"), "{matter}");
    assert!(matter.contains("## Partes y personas") && matter.contains("Persona Dos") && matter.contains("Ejemplo Uno S.A."), "{matter}");
    // Why they go together, in one line; each document with its reasons is
    // in «Datos y correcciones», and the documents are listed once (D-441).
    assert!(matter.contains("*Por qué estos documentos van juntos* — 3 documentos: "), "{matter}");
    assert!(matter.contains("comparten el importe (2)") && matter.contains("comparten el número (2)"), "{matter}");
    assert!(!matter.contains("comparten el número 000100012345") && !matter.contains("> [!info]-"), "{matter}");
    let listed = &matter[..matter.find("\n[^1]: ").unwrap_or(matter.len())];
    for file in ["f.txt", "l.txt", "r.txt"] {
        assert_eq!(listed.matches(&format!("|{file}]]")).count(), 1, "{file} more than once: {matter}");
    }
    let id = object_in(matter);
    let detail = base.knowledge_matter(&Visible::All, &id).unwrap().expect("its data");
    let reasons: Vec<String> = detail.documents.iter().flat_map(|d| d.reasons.iter().map(|r| format!("{} {}", r.kind, r.value))).collect();
    assert!(reasons.iter().any(|r| r == "numero 000100012345") && reasons.iter().any(|r| r.starts_with("importe ")), "{reasons:?}");
    // What its documents say, each with its quote as a note.
    assert!(matter.contains("$ 1.234,56"), "{matter}");
    assert!(note_after(matter, "$ 1.234,56").contains("([["), "{matter}");
    // The person's documents, by matter; the note of another subject apart.
    let person = page_of(&pages, "Persona Dos");
    // Named by its parties, never by the invoice (D-267); its documents
    // are all of one kind here, which names it (D-288).
    assert!(person.contains("### [[contrato-entre-ejemplo-uno-s-a-y-persona-dos-"), "{person}");
    assert!(person.contains("### Sin caso identificado") && person.contains("o.txt"), "{person}");
    // No page for a matter named in one document only.
    assert!(!pages.iter().any(|(_, t)| t.contains("entity_kind: asunto") && t.contains("entity_id: ent-")));
}

#[test]
fn what_a_person_decides_about_matters_holds() {
    let (_dir, base, _project, _asker) = archive();
    let (f, r, o) = (content(&base, "f.txt"), content(&base, "r.txt"), content(&base, "o.txt"));
    // The receipt is not of that matter: it stays out, whatever it shares.
    let apart = base
        .knowledge_decide(&IdentityDecision::OtherMatter { a: format!("content:{r}"), b: format!("content:{f}") }, None, None)
        .unwrap();
    assert_eq!(matters(&base), vec![vec!["f.txt".to_string(), "l.txt".to_string()]]);
    // The note is: it goes in.
    base.knowledge_decide(&IdentityDecision::SameMatter { a: format!("content:{o}"), b: format!("content:{f}") }, None, None).unwrap();
    assert_eq!(matters(&base), vec![vec!["f.txt".to_string(), "l.txt".to_string(), "o.txt".to_string()]]);
    // Undoing the separation lets the receipt back in.
    base.knowledge_undo(apart).unwrap();
    assert_eq!(matters(&base)[0].len(), 4);
}

#[test]
fn a_position_in_two_companies_is_two_claims_and_no_conflict() {
    let (_dir, base, _project, _asker) = archive();
    let pages = entity_pages(&base);
    let nine = page_of(&pages, "Persona Nueve");
    assert!(nine.contains("presidente (en Ejemplo Uno S.A.)") && nine.contains("presidente (en Otra Empresa S.R.L.)"), "{nine}");
    assert!(!nine.contains("Datos en los que los documentos difieren"), "{nine}");
    let conflicts = base.knowledge_status(&Visible::All).unwrap().conflicts;
    assert_eq!(conflicts, 0);
}

#[test]
fn a_relation_is_shown_on_both_sides_with_its_quote() {
    let (_dir, base, _project, _asker) = archive();
    let pages = entity_pages(&base);
    let nine = page_of(&pages, "Persona Nueve");
    assert!(nine.contains("## Vínculos") && nine.contains("Representa a [[ejemplo-uno-s-a-"), "{nine}");
    let company = page_of(&pages, "Ejemplo Uno S.A.");
    assert!(company.contains("## Vínculos") && company.contains("Representada por [[persona-nueve-"), "{company}");
    assert!(company.contains("«firmada por Persona Nueve, presidente de Ejemplo Uno S.A.»"), "{company}");
    // A tie and its inverse from the other side are one line (D-239).
    let two = page_of(&pages, "Persona Dos");
    assert_eq!(two.matches("- Deudor de [[ejemplo-uno-s-a-").count(), 1, "{two}");
    assert_eq!(company.matches("- Acreedor de [[persona-dos-").count(), 1, "{company}");
}

#[test]
fn the_model_proposes_the_matter_of_a_loose_document_and_it_stays_a_proposal() {
    let (_dir, base, _project, asker) = archive();
    // The mail shares the parties and no number: it is not grouped…
    assert_eq!(matters(&base), vec![vec!["f.txt".to_string(), "l.txt".to_string(), "r.txt".to_string()]]);
    // …but the model, asked once, proposes it; the note it does not.
    let asked = |a: &FakeModel| a.asked.lock().unwrap().iter().filter(|q| q.contains("es parte de alguno de estos asuntos")).count();
    assert_eq!(asked(&asker), 2);
    let pages = entity_pages(&base);
    let matter = pages.iter().find(|(_, t)| t.contains("entity_id: mat-")).map(|(_, t)| t.as_str()).unwrap();
    assert!(matter.contains("### Posiblemente de este caso") && matter.contains("|p.txt]] — certeza media: por los hechos"), "{matter}");
    assert!(matter.contains("## Documentos (3)"), "{matter}");
    // Proposed, without a word about the program (P-208).
    assert!(matter.contains("*sin confirmar: una persona los confirma o los descarta en «Datos y correcciones»*") && !matter.contains("el modelo"), "{matter}");
    // A shared reference as the documents write it, never its normalized
    // form (D-381): in «Datos y correcciones», with each document.
    let id = object_in(matter);
    let detail = base.knowledge_matter(&Visible::All, &id).unwrap().expect("its data");
    let names: Vec<String> = detail.documents.iter().flat_map(|d| d.reasons.iter().filter_map(|r| r.value_name.clone())).collect();
    assert!(names.iter().any(|n| n.eq_ignore_ascii_case("Factura 0001-00012345")), "{names:?}");
    let person = page_of(&pages, "Persona Dos");
    assert!(person.contains("|p.txt]] — remitente · contrato (propuesto)"), "{person}");
    // Asking again asks nothing.
    base.knowledge_propose_matters(Some(&asker), 50).unwrap();
    assert_eq!(asked(&asker), 2);
}
