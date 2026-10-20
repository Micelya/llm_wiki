//! Identity across names of the same organization: the form without the
//! company within a document, areas and general mailboxes, groups of
//! companies and matters of the same name between other parties
//! (D-260 to D-264). Invented documents only (R-24).

use super::tests::{answer, base_with, claim, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use super::tests_identity::run;
use crate::docbase::DocBase;

const G1: &str = "DOC-G1. Contrato entre Ejemplar S.A. y un cliente, firmado por Persona Cinco, apoderado de Ejemplar S.A. Sobre enviado por Legales Ejemplar (legal@ejemplar.com).";
const G2: &str = "DOC-G2. Acuerdo de Ejemplar LLC firmado por Persona Cinco en representación de Ejemplar LLC. Sobre enviado por Legales Ejemplar (legal@ejemplar.com).";
const G3: &str = "DOC-G3. Correo de EJEMPLAR sobre una deuda pendiente con un proveedor. Para: Cobranzas. Copia: legal@ejemplar.com.";
const G4: &str = "DOC-G4. Carta de Ejemplar S.A. a un proveedor. Firma EJEMPLAR, por su apoderado.";
const G5: &str = "DOC-G5. Factura de Muestra Tecno S.A., empresa de Grupo Muestra. Denuncias: Línea Ética de Grupo Muestra.";
const G6: &str = "DOC-G6. Acuerdo de terminación entre Ejemplar LLC y Persona Seis, con un pago final.";
const G7: &str = "DOC-G7. Acuerdo de terminación entre Ejemplar LLC y Persona Siete, con un pago final.";
const G8: &str = "DOC-G8. Aviso de envío de noreply@muestratecno.com sobre una entrega de equipos pendiente para el cliente, con su número de seguimiento.";

const FILES: &[(&str, &str)] =
    &[("g1.txt", G1), ("g2.txt", G2), ("g3.txt", G3), ("g4.txt", G4), ("g5.txt", G5), ("g6.txt", G6), ("g7.txt", G7), ("g8.txt", G8)];

fn none() -> serde_json::Value {
    serde_json::json!([])
}

fn reading(prompt: &str) -> String {
    let (mentions, claims) = if prompt.contains("DOC-G1.") {
        (
            vec![
                mention("s", "organizacion", "Ejemplar S.A.", "parte", "Contrato entre Ejemplar S.A.", none()),
                mention("p", "persona", "Persona Cinco", "apoderado", "Persona Cinco, apoderado de Ejemplar S.A.", none()),
                mention("l", "organizacion", "Legales Ejemplar (legal@ejemplar.com)", "otro", "Legales Ejemplar (legal@ejemplar.com)", none()),
            ],
            vec![claim("p", "representa_a", Some("s"), None, "Persona Cinco, apoderado de Ejemplar S.A.", None)],
        )
    } else if prompt.contains("DOC-G2.") {
        (
            vec![
                mention("s", "organizacion", "Ejemplar LLC", "parte", "Acuerdo de Ejemplar LLC", none()),
                mention("p", "persona", "Persona Cinco", "apoderado", "firmado por Persona Cinco en representación de Ejemplar LLC", none()),
                mention("l", "organizacion", "Legales Ejemplar (legal@ejemplar.com)", "otro", "Legales Ejemplar (legal@ejemplar.com)", none()),
            ],
            vec![claim("p", "representa_a", Some("s"), None, "Persona Cinco en representación de Ejemplar LLC", None)],
        )
    } else if prompt.contains("DOC-G3.") {
        (
            vec![
                mention("o", "organizacion", "EJEMPLAR", "otro", "Correo de EJEMPLAR", none()),
                mention("c", "organizacion", "Cobranzas", "destinatario", "Para: Cobranzas", none()),
                mention("m", "persona", "legal@ejemplar.com", "destinatario", "Copia: legal@ejemplar.com", none()),
            ],
            vec![],
        )
    } else if prompt.contains("DOC-G4.") {
        (
            vec![
                mention("s", "organizacion", "Ejemplar S.A.", "remitente", "Carta de Ejemplar S.A.", none()),
                mention("o", "organizacion", "EJEMPLAR", "firmante", "Firma EJEMPLAR, por su apoderado", none()),
            ],
            vec![],
        )
    } else if prompt.contains("DOC-G5.") {
        (
            vec![
                mention("t", "organizacion", "Muestra Tecno S.A.", "emisor", "Factura de Muestra Tecno S.A.", none()),
                mention("g", "organizacion", "Grupo Muestra", "otro", "empresa de Grupo Muestra", none()),
                mention("e", "organizacion", "Línea Ética de Grupo Muestra", "otro", "Denuncias: Línea Ética de Grupo Muestra", none()),
            ],
            vec![],
        )
    } else if prompt.contains("DOC-G6.") || prompt.contains("DOC-G7.") {
        let person = if prompt.contains("DOC-G6.") { "Persona Seis" } else { "Persona Siete" };
        (
            vec![
                mention("a", "asunto", "Acuerdo de terminación", "referido", "Acuerdo de terminación entre Ejemplar LLC", none()),
                mention("s", "organizacion", "Ejemplar LLC", "parte", "entre Ejemplar LLC", none()),
                mention("p", "persona", person, "parte", &format!("y {person}"), none()),
            ],
            vec![],
        )
    } else if prompt.contains("DOC-G8.") {
        (vec![mention("n", "organizacion", "noreply@muestratecno.com", "remitente", "noreply@muestratecno.com", none())], vec![])
    } else {
        return empty_answer();
    };
    answer(mentions, claims)
}

/// A model that says "the same" of every pair: what keeps things apart
/// here is the program, not the model.
fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            let candidates = prompt.split("Candidatos:\n").nth(1).unwrap_or_default();
            let count = candidates.matches("\n[").count() + 1;
            let answers: Vec<serde_json::Value> = (1..=count)
                .map(|i| serde_json::json!({ "candidato": i, "decision": "misma", "certeza": "alta", "razon": "el nombre" }))
                .collect();
            serde_json::json!({ "respuestas": answers }).to_string()
        } else if is_knowledge(prompt) {
            reading(prompt)
        } else {
            type_answer("contrato")
        }
    })
}

fn entity(base: &DocBase, document: &str, written: &str) -> Option<String> {
    base.conn()
        .query_row(
            "SELECT m.entity_id FROM knowledge_mention m JOIN file f ON f.content_id = m.content_id
             JOIN location l ON l.file_hash = f.hash WHERE l.name = ?1 AND m.written = ?2",
            [document, written],
            |row| row.get(0),
        )
        .unwrap()
}

fn name(base: &DocBase, id: &str) -> String {
    base.conn().query_row("SELECT name FROM knowledge_entity WHERE id = ?1", [id], |row| row.get(0)).unwrap()
}

#[test]
fn companies_of_one_name_areas_and_their_group() {
    let (_dir, base) = base_with(FILES);
    let asker = model();
    run(&base, &asker);

    let sa = entity(&base, "g1.txt", "Ejemplar S.A.").unwrap();
    let llc = entity(&base, "g2.txt", "Ejemplar LLC").unwrap();
    assert_ne!(sa, llc, "another legal form is another company");
    // Within a document, the name without its form is the one complete
    // form the document writes (D-261).
    assert_eq!(entity(&base, "g4.txt", "EJEMPLAR").as_ref(), Some(&sa));
    // The same mailbox in documents of the two companies, and the same
    // person representing both: a group. What has no form and no context
    // is the group's, and so is the mailbox, never one company's (D-260,
    // D-262).
    let lone = entity(&base, "g3.txt", "EJEMPLAR").unwrap();
    let mailbox = entity(&base, "g1.txt", "Legales Ejemplar (legal@ejemplar.com)").unwrap();
    assert_eq!(entity(&base, "g2.txt", "Legales Ejemplar (legal@ejemplar.com)").as_ref(), Some(&mailbox));
    assert_eq!(mailbox, lone);
    assert!(mailbox != sa && mailbox != llc);
    assert_eq!(name(&base, &lone), "Ejemplar", "the group is named by the shared name, not by its mailbox");
    // A general mailbox read as a person is no one; an area that names no
    // organization is not an entity (D-262).
    assert_eq!(entity(&base, "g3.txt", "legal@ejemplar.com"), None);
    assert_eq!(entity(&base, "g3.txt", "Cobranzas"), None);
    let members: Vec<(String, String)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT member_id, reasons FROM knowledge_group WHERE group_id = ?1 ORDER BY member_id").unwrap();
        let rows = statement.query_map([&lone], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let ids: Vec<&String> = members.iter().map(|(m, _)| m).collect();
    assert!(ids.contains(&&sa) && ids.contains(&&llc), "{members:?}");
    assert!(members.iter().all(|(_, r)| r.contains("mismas-personas") && r.contains("misma-casilla")), "{members:?}");

    // An area of a group the document names is that group; the group
    // has the companies that start with its name and appear with it.
    let group = entity(&base, "g5.txt", "Grupo Muestra").unwrap();
    assert_eq!(entity(&base, "g5.txt", "Línea Ética de Grupo Muestra").as_ref(), Some(&group));
    assert_eq!(name(&base, &group), "Grupo Muestra");
    let tecno = entity(&base, "g5.txt", "Muestra Tecno S.A.").unwrap();
    let in_group: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_group WHERE group_id = ?1 AND member_id = ?2", [&group, &tecno], |row| row.get(0))
        .unwrap();
    assert_eq!(in_group, 1);
    // A general mailbox is of the company its domain names.
    assert_eq!(entity(&base, "g8.txt", "noreply@muestratecno.com").as_ref(), Some(&tecno));

    // A reference is data of its document, not an entity (D-377); two
    // agreements of the same name between other parties are not one
    // matter (D-263, D-378).
    assert_eq!(entity(&base, "g6.txt", "Acuerdo de terminación"), None);
    assert_eq!(entity(&base, "g7.txt", "Acuerdo de terminación"), None);
    let together: i64 = base
        .conn()
        .query_row(
            "SELECT count(*) FROM knowledge_matter_document a JOIN knowledge_matter_document b ON a.matter_id = b.matter_id
             WHERE a.content_id = (SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = 'g6.txt')
               AND b.content_id = (SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = 'g7.txt')",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(together, 0);
}

const M1: &str = "DOC-M1. Carta documento: Muestra Uno S.A. reclama a Muestra Dos S.R.L. el pago de la factura 0001-00099999 por $ 12.345,67 dentro de las 48 horas.";
const M2: &str = "DOC-M2. Factura 0001-00099999 emitida por Muestra Uno S.A. a Muestra Dos S.R.L. por servicios de mantenimiento. Total $ 12.345,67.";
const M3: &str = "DOC-M3. Cronología interna del reclamo: el primero de marzo se envió la factura 0001-00099999 y no hubo respuesta del cliente hasta hoy.";

fn matter_reading(prompt: &str) -> String {
    let ids = |value: &str, quote: &str| serde_json::json!([{ "clase": "factura", "valor": value, "cita": quote }]);
    let (mentions, claims) = if prompt.contains("DOC-M1.") {
        (
            vec![
                mention("u", "organizacion", "Muestra Uno S.A.", "remitente", "Muestra Uno S.A. reclama", none()),
                mention("d", "organizacion", "Muestra Dos S.R.L.", "destinatario", "a Muestra Dos S.R.L.", none()),
                mention("f", "asunto", "la factura 0001-00099999", "referido", "la factura 0001-00099999", ids("0001-00099999", "factura 0001-00099999")),
            ],
            vec![super::tests::amount("d", "u", "$ 12.345,67", "reclamo", "Muestra Uno S.A. reclama a Muestra Dos S.R.L. el pago de la factura 0001-00099999 por $ 12.345,67", None)],
        )
    } else if prompt.contains("DOC-M2.") {
        (
            vec![
                mention("u", "organizacion", "Muestra Uno S.A.", "emisor", "emitida por Muestra Uno S.A.", none()),
                mention("d", "organizacion", "Muestra Dos S.R.L.", "destinatario", "a Muestra Dos S.R.L.", none()),
                mention("f", "asunto", "Factura 0001-00099999", "referido", "Factura 0001-00099999", ids("0001-00099999", "Factura 0001-00099999")),
            ],
            vec![],
        )
    } else {
        // The chronology is read with nothing in it: its number is in the
        // text all the same.
        return empty_answer();
    };
    answer(mentions, claims)
}

#[test]
fn a_matter_is_named_by_the_claim_between_its_parties_and_a_number_in_the_text_joins_it() {
    // Other documents, so that the number is not in most of the archive.
    let mut files: Vec<(&str, &str)> = vec![("m1.txt", M1), ("m2.txt", M2), ("m3.txt", M3)];
    files.extend_from_slice(FILES);
    let (_dir, base) = base_with(&files);
    let asker = FakeModel::new("falso:1", |prompt| {
        if is_knowledge(prompt) && prompt.contains("DOC-G") {
            reading(prompt)
        } else if is_knowledge(prompt) {
            matter_reading(prompt)
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    });
    run(&base, &asker);
    let conn = base.conn();
    let (name, documents): (String, i64) = conn
        .query_row(
            "SELECT m.name, count(d.content_id) FROM knowledge_matter m JOIN knowledge_matter_document d ON d.matter_id = m.id
             WHERE m.state = 'active' AND m.name LIKE '%Muestra Dos%' GROUP BY m.id",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(documents, 3, "the chronology cites the invoice by its number (D-268)");
    assert_eq!(name, "Reclamo de Muestra Uno S.A. a Muestra Dos S.R.L.", "by the claim, never by the invoice (D-267)");
}

#[test]
fn an_abbreviation_without_its_last_dot_is_shown_with_it() {
    use super::resolve::most_written;
    let forms: std::collections::BTreeMap<&str, usize> = [("Ejemplo S.A", 5), ("Ejemplo S.A.", 2), ("EJEMPLO SA", 1)].into_iter().collect();
    assert_eq!(most_written(&forms), "Ejemplo S.A.");
    // Without the complete form, the most used one stays.
    let forms: std::collections::BTreeMap<&str, usize> = [("Ejemplo S.A", 5), ("EJEMPLO SA", 6)].into_iter().collect();
    assert_eq!(most_written(&forms), "EJEMPLO SA");
    let forms: std::collections::BTreeMap<&str, usize> = [("Ejemplo S.A", 5), ("EJEMPLO SA", 1)].into_iter().collect();
    assert_eq!(most_written(&forms), "Ejemplo S.A");
}

const T1: &str ="DOC-T1. Propuesta de servicios de brillo (Fabrica Ejemplo S.A.) para Ejemplar S.A. Firma EJEMPLAR, por su apoderado.";

#[test]
fn a_trade_name_with_another_company_in_parentheses_is_not_that_company() {
    let with_same = |mut m: serde_json::Value, same: &str| {
        m["misma_que"] = serde_json::json!(same);
        m
    };
    let reading = move |prompt: &str| -> String {
        if !prompt.contains("DOC-T1.") {
            return empty_answer();
        }
        answer(
            vec![
                mention("b", "organizacion", "brillo", "remitente", "Propuesta de servicios de brillo", none()),
                // The model says the company in parentheses is the trade name.
                with_same(mention("f", "organizacion", "Fabrica Ejemplo S.A.", "otro", "brillo (Fabrica Ejemplo S.A.)", none()), "b"),
                mention("s", "organizacion", "Ejemplar S.A.", "destinatario", "para Ejemplar S.A.", none()),
                // A name that fits the complete form still joins.
                with_same(mention("o", "organizacion", "EJEMPLAR", "firmante", "Firma EJEMPLAR", none()), "s"),
            ],
            vec![],
        )
    };
    let asker = FakeModel::new("falso:1", move |prompt| if is_knowledge(prompt) { reading(prompt) } else { type_answer("contrato") });
    let (_dir, base) = base_with(&[("t1.txt", T1)]);
    run(&base, &asker);
    let trade = entity(&base, "t1.txt", "brillo").unwrap();
    let company = entity(&base, "t1.txt", "Fabrica Ejemplo S.A.").unwrap();
    assert_ne!(trade, company, "names with nothing in common are not joined by the model's word (D-281)");
    assert_eq!(entity(&base, "t1.txt", "EJEMPLAR"), entity(&base, "t1.txt", "Ejemplar S.A."));
}

const S1: &str = "DOC-S1. Acuerdo de servicios entre las partes.\nPor la empresa: Ejemplar Norte S.A.\nNombre: Persona Ocho\nCargo: Apoderado\nPor el cliente: Persona Nueve\nFecha: hoy, en dos ejemplares.";

#[test]
fn a_signature_block_names_the_company_on_a_line_of_its_own() {
    let reading = |prompt: &str| -> String {
        if !prompt.contains("DOC-S1.") {
            return empty_answer();
        }
        answer(
            vec![
                mention("s", "organizacion", "Ejemplar Norte S.A.", "parte", "Por la empresa: Ejemplar Norte S.A.", none()),
                mention("p", "persona", "Persona Ocho", "firmante", "Nombre: Persona Ocho", none()),
                mention("c", "persona", "Persona Nueve", "parte", "Por el cliente: Persona Nueve", none()),
            ],
            vec![
                // The company is on its own line, before the name and the
                // position: it is the one represented (P-122, D-269).
                claim("p", "representa_a", Some("s"), None, "Nombre: Persona Ocho\nCargo: Apoderado", None),
                // Nothing in this quote says she acts for the company.
                claim("c", "representa_a", Some("s"), None, "Por el cliente: Persona Nueve", None),
            ],
        )
    };
    let asker = FakeModel::new("falso:1", move |prompt| if is_knowledge(prompt) { reading(prompt) } else { type_answer("contrato") });
    let (_dir, base) = base_with(&[("s1.txt", S1)]);
    run(&base, &asker);
    let kept: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT s.written FROM knowledge_evidence e JOIN knowledge_mention s ON s.id = e.subject_mention WHERE e.predicate = 'representa_a'")
            .unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(kept, vec!["Persona Ocho".to_string()]);
}

fn matter_asker() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_knowledge(prompt) && prompt.contains("DOC-G") {
            reading(prompt)
        } else if is_knowledge(prompt) {
            matter_reading(prompt)
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

fn content_of(base: &DocBase, file: &str) -> i64 {
    base.conn()
        .query_row("SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [file], |row| row.get(0))
        .unwrap()
}

/// The active matters: (id, documents, confirmation).
fn active_matters(base: &DocBase) -> Vec<(String, i64, String)> {
    let conn = base.conn();
    let mut statement = conn
        .prepare(
            "SELECT m.id, count(d.content_id), m.confirmation FROM knowledge_matter m JOIN knowledge_matter_document d ON d.matter_id = m.id
             WHERE m.state = 'active' GROUP BY m.id ORDER BY m.id",
        )
        .unwrap();
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn regroup(base: &DocBase) -> super::matters::Grouped {
    let conn = base.conn();
    let decided = super::resolve::load_decided(&conn).unwrap();
    super::matters::group(&conn, super::vocabulary::vocabulary(), &decided.joins, &decided.separations).unwrap()
}

/// A matter counts families of versions, not copies (D-381): a letter
/// and its signed copy are one document, never a matter by themselves.
#[test]
fn one_document_with_its_versions_is_not_a_matter() {
    let mut files: Vec<(&str, &str)> = vec![("m1.txt", M1), ("m2.txt", M2)];
    files.extend_from_slice(FILES);
    let (_dir, base) = base_with(&files);
    run(&base, &matter_asker());
    let matters = active_matters(&base);
    assert_eq!(matters.len(), 1, "two documents that share an invoice: {matters:?}");
    assert_eq!(matters[0].2, "proposed", "the grouping proposes; a person confirms (D-373)");
    let status: String = base.conn().query_row("SELECT status FROM knowledge_matter WHERE id = ?1", [&matters[0].0], |row| row.get(0)).unwrap();
    assert_eq!(status, "open");
    let (a, b) = {
        let (x, y) = (content_of(&base, "m1.txt"), content_of(&base, "m2.txt"));
        (x.min(y), x.max(y))
    };
    base.conn()
        .execute(
            "INSERT OR REPLACE INTO similar_pair (a, b, relation, certainty, jaccard, containment, cover, similarity, description, evidence, evidence_hash, engine, computed_at)
             VALUES (?1, ?2, 'version', 'alta', 0.9, 0.9, 0.9, 0.9, '', '[]', '', 'test', 0)",
            [a, b],
        )
        .unwrap();
    let grouped = regroup(&base);
    assert_eq!(grouped.one_family, 1);
    assert!(active_matters(&base).is_empty(), "one family is not a matter");
}

/// Confirming or discarding a proposed matter is a decision of a person
/// that survives the grouping (D-381).
#[test]
fn a_confirmed_or_discarded_matter_stays_so_when_grouped_again() {
    let mut files: Vec<(&str, &str)> = vec![("m1.txt", M1), ("m2.txt", M2), ("m3.txt", M3)];
    files.extend_from_slice(FILES);
    let (_dir, base) = base_with(&files);
    run(&base, &matter_asker());
    let matters = active_matters(&base);
    assert_eq!(matters.len(), 1, "{matters:?}");
    let id = matters[0].0.clone();
    let attention = base.knowledge_attention(&crate::docbase::Visible::All).unwrap();
    assert_eq!(attention.matters_to_confirm.len(), 1);
    assert_eq!(attention.matters_to_confirm[0].documents, 3);

    base.knowledge_decide_matter(&id, "discard").unwrap();
    assert!(active_matters(&base).is_empty(), "a discarded matter goes away");
    let grouped = regroup(&base);
    assert_eq!(grouped.discarded, 1);
    assert!(active_matters(&base).is_empty(), "and stays away with the same documents");
    assert!(base.knowledge_attention(&crate::docbase::Visible::All).unwrap().matters_to_confirm.is_empty());

    base.knowledge_decide_matter(&id, "reopen").unwrap();
    assert_eq!(active_matters(&base), vec![(id.clone(), 3, "proposed".to_string())], "reopened, it is a proposal again");

    base.knowledge_decide_matter(&id, "confirm").unwrap();
    regroup(&base);
    assert_eq!(active_matters(&base), vec![(id.clone(), 3, "confirmed".to_string())]);
    let list = base.knowledge_matters(&crate::docbase::Visible::All).unwrap();
    assert_eq!((list[0].status.as_str(), list[0].origin.as_str(), list[0].confirmation.as_str()), ("open", "proposed", "confirmed"));
    assert!(base.knowledge_attention(&crate::docbase::Visible::All).unwrap().matters_to_confirm.is_empty());

    // A name a person gave stays.
    base.conn().execute("UPDATE knowledge_matter SET name = 'Nombre puesto a mano', name_by_person = 1 WHERE id = ?1", [&id]).unwrap();
    regroup(&base);
    let name: String = base.conn().query_row("SELECT name FROM knowledge_matter WHERE id = ?1", [&id], |row| row.get(0)).unwrap();
    assert_eq!(name, "Nombre puesto a mano");
}

fn documents_of(base: &DocBase, id: &str) -> Vec<i64> {
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT content_id FROM knowledge_matter_document WHERE matter_id = ?1 ORDER BY content_id").unwrap();
    let rows = statement.query_map([id], |row| row.get(0)).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

/// What a person does to a matter wins over what the grouping proposes
/// and survives it (D-386): the name, a document put in, a document taken
/// out; and putting a document in a proposed matter confirms it.
#[test]
fn what_a_person_does_to_a_matter_survives_the_grouping() {
    let mut files: Vec<(&str, &str)> = vec![("m1.txt", M1), ("m2.txt", M2), ("m3.txt", M3)];
    files.extend_from_slice(FILES);
    let (_dir, base) = base_with(&files);
    run(&base, &matter_asker());
    let matters = active_matters(&base);
    assert_eq!(matters.len(), 1, "{matters:?}");
    let id = matters[0].0.clone();
    let (m2, m3, g6) = (content_of(&base, "m2.txt"), content_of(&base, "m3.txt"), content_of(&base, "g6.txt"));

    base.knowledge_choose_matter_document(&id, &format!("content:{g6}"), "in").unwrap();
    base.knowledge_choose_matter_document(&id, &format!("content:{m3}"), "out").unwrap();
    base.knowledge_rename_matter(&id, "  Reclamo   puesto a mano ").unwrap();
    let confirmation: String = base.conn().query_row("SELECT confirmation FROM knowledge_matter WHERE id = ?1", [&id], |row| row.get(0)).unwrap();
    assert_eq!(confirmation, "confirmed", "putting a document in a proposed matter is deciding about it");
    regroup(&base);
    run(&base, &matter_asker());
    let docs = documents_of(&base, &id);
    assert!(docs.contains(&g6), "a document put in by hand stays: {docs:?}");
    assert!(docs.contains(&m2));
    assert!(!docs.contains(&m3), "a document taken out by hand stays out: {docs:?}");
    let name: String = base.conn().query_row("SELECT name FROM knowledge_matter WHERE id = ?1", [&id], |row| row.get(0)).unwrap();
    assert_eq!(name, "Reclamo puesto a mano");

    // A matter made by hand, empty and then with documents that are also
    // in the other one: a document can be in several matters.
    let own = base.knowledge_create_matter("Asunto propio", &[]).unwrap();
    let list = base.knowledge_matters(&crate::docbase::Visible::All).unwrap();
    let mine = list.iter().find(|m| m.id == own).expect("an empty matter made by hand is listed");
    assert_eq!((mine.origin.as_str(), mine.confirmation.as_str(), mine.documents), ("person", "confirmed", 0));
    base.knowledge_choose_matter_document(&own, &format!("content:{m2}"), "in").unwrap();
    base.knowledge_choose_matter_document(&own, &format!("content:{g6}"), "in").unwrap();
    base.knowledge_choose_matter_document(&own, &format!("content:{g6}"), "out").unwrap();
    regroup(&base);
    assert_eq!(documents_of(&base, &own), vec![m2], "the grouping never touches a matter made by hand");
    assert!(documents_of(&base, &id).contains(&m2), "and the document is still in the proposed one");
    assert!(base.knowledge_rename_matter(&own, " ").is_err(), "a matter made by hand needs a name");
    assert!(base.knowledge_attention(&crate::docbase::Visible::All).unwrap().matters_to_confirm.is_empty());
}
