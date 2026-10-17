//! Identity across names of the same organization: the form without the
//! company within a document, areas and general mailboxes, groups of
//! companies and matters of the same name between other parties
//! (D-260 to D-264). Invented documents only (R-24).

use super::tests::{answer, base_with, claim, empty_answer, is_knowledge, mention, type_answer, FakeModel};
use super::tests_identity::run;
use crate::docbase::DocBase;

const G1: &str = "DOC-G1. Contrato entre Ejemplar S.A. y un cliente, firmado por Persona Cinco, apoderado de Ejemplar S.A. Sobre enviado por Legales Ejemplar (legal@ejemplar.com).";
const G2: &str = "DOC-G2. Acuerdo de Ejemplar LLC firmado por Persona Cinco en representación de Ejemplar LLC. Sobre enviado por Legales Ejemplar (legal@ejemplar.com).";
const G3: &str = "DOC-G3. Correo de EJEMPLAR sobre una deuda pendiente con un proveedor.";
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
        (vec![mention("o", "organizacion", "EJEMPLAR", "otro", "Correo de EJEMPLAR", none())], vec![])
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

    // Two agreements of the same name between other parties are two
    // matters, even when the model would say they are the same (D-263).
    let first = entity(&base, "g6.txt", "Acuerdo de terminación").unwrap();
    let second = entity(&base, "g7.txt", "Acuerdo de terminación").unwrap();
    assert_ne!(first, second);
}
