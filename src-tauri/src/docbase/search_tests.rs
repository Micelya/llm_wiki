//! The search over the documents of the base, with invented documents.

use super::*;

fn base() -> DocBase {
    let base = DocBase::in_memory().unwrap();
    {
        let conn = base.conn();
        for origin in ["o1", "o2"] {
            conn.execute(
                "INSERT INTO origin (id, name, provider, location, added_at) VALUES (?1, ?1, 'local', ?1, 0)",
                [origin],
            )
            .unwrap();
        }
    }
    base
}

/// A document with its text, in an origin, with an optional summary.
fn document(base: &DocBase, id: i64, origin: &str, text: &str, summary: Option<&str>) {
    let conn = base.conn();
    conn.execute("INSERT INTO content (id, state, first_seen_at) VALUES (?1, 'done', 0)", [id]).unwrap();
    let hash = format!("h{id}");
    conn.execute("INSERT INTO file (hash, content_id, size) VALUES (?1, ?2, 1)", params![hash, id]).unwrap();
    conn.execute(
        "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, first_seen_at, last_seen_at)
         VALUES (?1, ?2, '', ?2, 'pdf', 1, 0, ?3, 0, 0)",
        params![origin, format!("doc{id}.pdf"), hash],
    )
    .unwrap();
    conn.execute("INSERT INTO content_fts (rowid, body) VALUES (?1, ?2)", params![id, text]).unwrap();
    if let Some(topic) = summary {
        conn.execute(
            "INSERT INTO knowledge_summary (content_id, topic, points) VALUES (?1, ?2, '[]')",
            params![id, topic],
        )
        .unwrap();
    }
    index_content(&conn, id).unwrap();
}

/// An entity with one name, mentioned in some documents.
fn entity(base: &DocBase, id: &str, name: &str, normalized: &str, documents: &[i64]) {
    let conn = base.conn();
    conn.execute(
        "INSERT INTO knowledge_entity (id, kind, name, created_at, updated_at) VALUES (?1, 'persona', ?2, 0, 0)",
        params![id, name],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO knowledge_alias (entity_id, normalized, written) VALUES (?1, ?2, ?3)",
        params![id, normalized, name],
    )
    .unwrap();
    for (n, content) in documents.iter().enumerate() {
        conn.execute(
            "INSERT INTO knowledge_mention (content_id, ref, kind, written, normalized, role, quote, entity_id)
             VALUES (?1, ?2, 'persona', ?3, ?4, 'parte', ?3, ?5)",
            params![content, format!("{id}-{n}"), name, normalized, id],
        )
        .unwrap();
    }
}

fn found(base: &DocBase, visible: &Visible, query: &str) -> Vec<i64> {
    base.search_documents(visible, query, 10, None).unwrap().hits.iter().map(|h| h.content_id).collect()
}

#[test]
fn numbers_are_one_however_they_are_written() {
    let forms = number_forms("DNI 12.345.678, factura A00999-00012345 por USD 37,500.00");
    assert!(forms.contains("12345678"));
    assert!(forms.contains("0099900012345"));
    assert!(forms.contains("37500"));
    assert_eq!(number_forms("factura 00999A00012345"), BTreeSet::from(["0099900012345".to_string()]));
    // Plain runs of digits are already words of the text.
    assert!(number_forms("cuotas 1 y 2 del año 2026").is_empty());
}

#[test]
fn a_number_with_dots_finds_it_written_without_them_and_back() {
    let base = base();
    document(&base, 1, "o1", "El documento nacional de identidad 12.345.678 del firmante.", None);
    document(&base, 2, "o1", "Cuenta del cliente 12345678 al día de hoy.", None);
    document(&base, 3, "o1", "Otro texto sin números de nadie.", None);
    let with_dots = found(&base, &Visible::All, "DNI 12.345.678");
    assert!(with_dots.contains(&1) && with_dots.contains(&2), "{with_dots:?}");
    let without = found(&base, &Visible::All, "12345678");
    assert_eq!(&without[..2].iter().copied().collect::<BTreeSet<_>>(), &BTreeSet::from([1, 2]));
}

#[test]
fn a_document_in_another_language_is_found_by_its_summary() {
    let base = base();
    document(
        &base,
        1,
        "o1",
        "Mutual non-disclosure terms between the parties regarding confidential information.",
        Some("Acuerdo de confidencialidad entre dos sociedades"),
    );
    document(&base, 2, "o1", "Recibo de pago de una cuota.", Some("Recibo de pago"));
    assert_eq!(found(&base, &Visible::All, "acuerdo de confidencialidad")[0], 1);
}

#[test]
fn long_words_also_find_their_singular_and_plural() {
    let base = base();
    document(&base, 1, "o1", "Son competentes los tribunal ordinarios de la ciudad.", None);
    document(&base, 2, "o1", "Recibo de pago.", None);
    assert_eq!(found(&base, &Visible::All, "¿Qué tribunales?")[0], 1);
}

#[test]
fn the_graph_orders_and_adds_but_never_takes_a_document_out() {
    let base = base();
    // The text of 1 does not have the name; the record knows it appears there.
    document(&base, 1, "o1", "Contrato de locación firmado por el inquilino.", None);
    document(&base, 2, "o1", "Contrato de locación de un depósito.", None);
    document(&base, 3, "o1", "Carta de Zoe Quintanar sobre otro tema.", None);
    for id in 4..10 {
        document(&base, id, "o1", "Factura de servicios.", None);
    }
    entity(&base, "ent-zq", "Zoe Quintanar", "zoe quintanar", &[1, 3]);
    let search = base.search_documents(&Visible::All, "contrato de locación de Zoe Quintanar", 10, None).unwrap();
    let order: Vec<i64> = search.hits.iter().map(|h| h.content_id).collect();
    assert_eq!(search.named, vec!["ent-zq".to_string()]);
    // 1 has the text and the entity: above 2, which has only the text and is still there.
    let at = |c: i64| order.iter().position(|x| *x == c).unwrap();
    assert!(at(1) < at(2), "{order:?}");
    assert!(order.contains(&3), "{order:?}");
    let one = search.hits.iter().find(|h| h.content_id == 1).unwrap();
    assert!(one.found_by.contains(&"graph".to_string()) && one.entities == vec!["ent-zq".to_string()]);
}

#[test]
fn a_name_written_everywhere_does_not_name_its_entity() {
    let base = base();
    for id in 1..=6 {
        document(&base, id, "o1", "Acuerdo entre las partes.", None);
    }
    for id in 7..=20 {
        document(&base, id, "o1", "Factura.", None);
    }
    entity(&base, "ent-ac", "Acuerdo", "acuerdo", &[1]);
    let search = base.search_documents(&Visible::All, "acuerdo", 10, None).unwrap();
    assert!(search.named.is_empty());
}

#[test]
fn only_what_is_visible_and_only_the_chosen_set_when_there_is_one() {
    let base = base();
    document(&base, 1, "o1", "Pagaré por la compraventa del rodado.", None);
    document(&base, 2, "o2", "Pagaré por la compraventa del inmueble.", None);
    document(&base, 3, "o1", "Pagaré del préstamo.", None);
    entity(&base, "ent-x", "Rodado Inmueble", "compraventa", &[2]);
    let mine = Visible::Origins(vec!["o1".to_string()]);
    let order = found(&base, &mine, "pagaré compraventa");
    assert!(!order.contains(&2), "{order:?}");
    assert!(order.contains(&1) && order.contains(&3));
    let chosen = base.search_documents(&Visible::All, "pagaré", 10, Some(&[3])).unwrap();
    assert_eq!(chosen.hits.iter().map(|h| h.content_id).collect::<Vec<_>>(), vec![3]);
}

#[test]
fn the_index_follows_the_text_and_the_summary() {
    let base = base();
    document(&base, 1, "o1", "Texto con el número 98.765.432.", None);
    {
        let conn = base.conn();
        conn.execute("DELETE FROM content_search", []).unwrap();
        assert_eq!(fill_missing(&conn).unwrap(), 1);
        let numbers: String =
            conn.query_row("SELECT numbers FROM content_search WHERE rowid = 1", [], |r| r.get(0)).unwrap();
        assert_eq!(numbers, "98765432");
    }
    assert_eq!(found(&base, &Visible::All, "98765432"), vec![1]);
}
