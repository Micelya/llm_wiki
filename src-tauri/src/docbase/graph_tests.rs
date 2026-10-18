//! The graph from the base, with invented documents and people.

use rusqlite::params;

use super::*;

fn document(base: &DocBase, id: i64, origin: &str) {
    let conn = base.conn();
    conn.execute(
        "INSERT OR IGNORE INTO origin (id, name, provider, location, added_at) VALUES (?1, ?1, 'local', ?1, 0)",
        [origin],
    )
    .unwrap();
    conn.execute("INSERT INTO content (id, state, first_seen_at) VALUES (?1, 'done', 0)", [id]).unwrap();
    conn.execute("INSERT INTO file (hash, content_id, size) VALUES (?1, ?2, 1)", params![format!("h{id}"), id]).unwrap();
    conn.execute(
        "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, first_seen_at, last_seen_at)
         VALUES (?1, ?2, '', ?3, 'pdf', 1, 0, ?4, 0, 0)",
        params![origin, format!("doc{id}.pdf"), format!("Documento {id}"), format!("h{id}")],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO knowledge_page (entity_id, path, fingerprint, written_at) VALUES (?1, ?2, '', 0)",
        params![format!("doc-{id}"), format!("wiki/sources/documento-{id}.md")],
    )
    .unwrap();
}

fn person(base: &DocBase, id: &str, name: &str) {
    let conn = base.conn();
    conn.execute(
        "INSERT INTO knowledge_entity (id, kind, name, created_at, updated_at) VALUES (?1, 'persona', ?2, 0, 0)",
        params![id, name],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO knowledge_page (entity_id, path, fingerprint, written_at) VALUES (?1, ?2, '', 0)",
        params![id, format!("wiki/entities/{id}.md")],
    )
    .unwrap();
}

fn mention(base: &DocBase, content: i64, entity: &str, role: &str) -> i64 {
    let conn = base.conn();
    conn.execute(
        "INSERT INTO knowledge_mention (content_id, ref, kind, written, normalized, role, quote, entity_id)
         VALUES (?1, ?2, 'persona', ?3, ?3, ?4, ?5, ?3)",
        params![content, format!("{entity}-{content}-{role}"), entity, role, format!("cita de {entity}")],
    )
    .unwrap();
    conn.last_insert_rowid()
}

#[test]
fn edges_are_the_relations_of_the_base_with_type_quote_and_state() {
    let base = DocBase::in_memory().unwrap();
    document(&base, 1, "o1");
    document(&base, 2, "o1");
    document(&base, 3, "o2");
    document(&base, 4, "o1");
    person(&base, "ent-a", "Ana Prueba");
    person(&base, "ent-b", "Beto Ensayo");
    let a1 = mention(&base, 1, "ent-a", "parte");
    let b1 = mention(&base, 1, "ent-b", "apoderado");
    mention(&base, 3, "ent-a", "parte");
    {
        let conn = base.conn();
        conn.execute(
            "INSERT INTO knowledge_claim (id, subject, predicate, object_entity, computed_at) VALUES (1, 'ent-b', 'representa_a', 'ent-a', 0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO knowledge_evidence (content_id, subject_mention, predicate, object_mention, quote, claim_id)
             VALUES (1, ?1, 'representa_a', ?2, 'Beto en nombre de Ana', 1)",
            params![b1, a1],
        )
        .unwrap();
        for (a, b, relation, state) in [(1, 2, "same-template", "proposed"), (1, 4, "same-template", "discarded")] {
            conn.execute(
                "INSERT INTO similar_pair (a, b, relation, certainty, jaccard, containment, cover, similarity, description, evidence, evidence_hash, engine, computed_at, state)
                 VALUES (?1, ?2, ?3, 'probable', 0, 0, 0, 0, '', '', '', 'test', 0, ?4)",
                params![a, b, relation, state],
            )
            .unwrap();
        }
    }
    let graph = base.graph(&Visible::Origins(vec!["o1".to_string()])).unwrap();
    let ids: Vec<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    // Document 3 is not visible; the people have pages.
    assert!(!ids.contains(&"documento-3"), "{ids:?}");
    assert!(ids.contains(&"documento-1") && ids.contains(&"ent-a") && ids.contains(&"ent-b"));
    let one = graph.nodes.iter().find(|n| n.id == "documento-1").unwrap();
    assert_eq!((one.page_type.as_str(), one.label.as_str()), ("source", "Documento 1"));
    let edge = |family: &str, kind: &str| graph.edges.iter().find(|e| e.family == family && e.kind == kind).cloned();
    let party = edge("mention", "parte").unwrap();
    assert_eq!((party.source.as_str(), party.target.as_str()), ("documento-1", "ent-a"));
    assert_eq!(party.quote.as_deref(), Some("cita de ent-a"));
    let says = edge("claim", "representa_a").unwrap();
    assert_eq!((says.source.as_str(), says.target.as_str()), ("ent-b", "ent-a"));
    assert_eq!(says.quote.as_deref(), Some("Beto en nombre de Ana"));
    assert!(edge("similar", "same-template").is_some());
    // The discarded relation is not an edge; nothing joins 1 and 4.
    assert!(!graph.edges.iter().any(|e| e.target == "documento-4" || e.source == "documento-4"));
    // Only what is visible: the mention in document 3 gives no edge.
    assert_eq!(graph.edges.iter().filter(|e| e.family == "mention").count(), 2);
}
