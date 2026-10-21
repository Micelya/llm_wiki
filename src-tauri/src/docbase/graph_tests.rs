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
    // Document 3 is not visible.
    assert!(!ids.contains(&"doc-3"), "{ids:?}");
    assert!(ids.contains(&"doc-1") && ids.contains(&"ent-a") && ids.contains(&"ent-b"));
    let one = graph.nodes.iter().find(|n| n.id == "doc-1").unwrap();
    assert_eq!((one.kind.as_str(), one.label.as_str()), ("documento", "Documento 1"));
    assert_eq!(one.path.as_deref(), Some("wiki/sources/documento-1.md"));
    assert!(one.location_id.is_some());
    let edge = |family: &str, kind: &str| graph.edges.iter().find(|e| e.family == family && e.kind == kind).cloned();
    let party = edge("mention", "parte").unwrap();
    assert_eq!((party.source.as_str(), party.target.as_str()), ("doc-1", "ent-a"));
    assert_eq!(party.quote.as_deref(), Some("cita de ent-a"));
    let says = edge("claim", "representa_a").unwrap();
    assert_eq!((says.source.as_str(), says.target.as_str()), ("ent-b", "ent-a"));
    assert_eq!(says.quote.as_deref(), Some("Beto en nombre de Ana"));
    assert!(edge("similar", "same-template").is_some());
    // The discarded relation is not an edge; nothing joins 1 and 4.
    assert!(!graph.edges.iter().any(|e| e.target == "doc-4" || e.source == "doc-4"));
    // Only what is visible: the mention in document 3 gives no edge.
    assert_eq!(graph.edges.iter().filter(|e| e.family == "mention").count(), 2);
}

#[test]
fn nodes_are_the_objects_of_the_base_with_or_without_a_page() {
    let base = DocBase::in_memory().unwrap();
    document(&base, 1, "o1");
    document(&base, 2, "o1");
    document(&base, 5, "o1");
    person(&base, "ent-a", "Ana Prueba");
    {
        let conn = base.conn();
        // Without a page: an organization, a group, a reference, and
        // what is merged or retired.
        for (id, kind, state) in [
            ("ent-c", "organizacion", "active"),
            ("ent-g", "organizacion", "active"),
            ("ent-r", "asunto", "active"),
            ("ent-x", "persona", "retired"),
        ] {
            conn.execute(
                "INSERT INTO knowledge_entity (id, kind, name, state, created_at, updated_at) VALUES (?1, ?2, ?1, ?3, 0, 0)",
                params![id, kind, state],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO knowledge_entity (id, kind, name, state, merged_into, created_at, updated_at) VALUES ('ent-m', 'persona', 'M', 'merged', 'ent-a', 0, 0)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO knowledge_group (group_id, member_id) VALUES ('ent-g', 'ent-c')", []).unwrap();
        // Two versions of one document: one node.
        conn.execute(
            "INSERT INTO similar_pair (a, b, relation, certainty, jaccard, containment, cover, similarity, description, evidence, evidence_hash, engine, computed_at, state)
             VALUES (1, 2, 'version', 'probable', 0, 0, 0, 0, '', '', '', 'test', 0, 'proposed')",
            [],
        )
        .unwrap();
        for (id, confirmation, state) in [("mat-p", "proposed", "active"), ("mat-c", "confirmed", "active"), ("mat-d", "discarded", "retired")] {
            conn.execute(
                "INSERT INTO knowledge_matter (id, name, state, confirmation, created_at, updated_at) VALUES (?1, ?1, ?2, ?3, 0, 0)",
                params![id, state, confirmation],
            )
            .unwrap();
        }
        conn.execute("INSERT INTO knowledge_matter_document (matter_id, content_id, state) VALUES ('mat-p', 2, 'proposed')", []).unwrap();
        conn.execute("INSERT INTO knowledge_matter_document (matter_id, content_id) VALUES ('mat-d', 5)", []).unwrap();
    }
    mention(&base, 2, "ent-m", "parte");
    mention(&base, 5, "ent-c", "emisor");
    let graph = base.graph(&Visible::All).unwrap();
    let kinds: BTreeMap<&str, &str> = graph.nodes.iter().map(|n| (n.id.as_str(), n.kind.as_str())).collect();
    assert_eq!(
        kinds,
        BTreeMap::from([
            ("doc-1", "documento"),
            ("doc-5", "documento"),
            ("ent-a", "persona"),
            ("ent-c", "organizacion"),
            ("ent-g", "grupo"),
            ("mat-c", "asunto"),
            ("mat-p", "asunto"),
        ])
    );
    let matter = graph.nodes.iter().find(|n| n.id == "mat-p").unwrap();
    assert_eq!((matter.confirmation.as_deref(), matter.status.as_deref()), (Some("proposed"), Some("open")));
    assert_eq!(matter.path, None);
    // The version's edges are the family's; a merged entity's, its root's.
    let pairs: BTreeSet<(&str, &str, &str, &str)> =
        graph.edges.iter().map(|e| (e.source.as_str(), e.target.as_str(), e.family.as_str(), e.state.as_str())).collect();
    assert_eq!(
        pairs,
        BTreeSet::from([
            ("doc-1", "ent-a", "mention", "read"),
            ("doc-5", "ent-c", "mention", "read"),
            ("mat-p", "doc-1", "matter", "proposed"),
            ("ent-g", "ent-c", "group", "proposed"),
        ])
    );
}

/// Counts the graph of a copy of a real base: nodes by kind, edges by
/// family and by state. Only numbers are printed.
#[test]
#[ignore = "reads a copy of a real base"]
fn graph_of_a_copy_of_the_base() {
    let Ok(copy) = std::env::var("MICELYA_DOCBASE_COPY") else {
        println!("MICELYA_DOCBASE_COPY is not set: nothing to measure");
        return;
    };
    let base = DocBase::open(std::path::Path::new(&copy)).unwrap();
    let graph = base.graph(&Visible::All).unwrap();
    let mut nodes: BTreeMap<String, usize> = BTreeMap::new();
    for node in &graph.nodes {
        *nodes.entry(node.kind.clone()).or_default() += 1;
    }
    let mut edges: BTreeMap<String, usize> = BTreeMap::new();
    let mut states: BTreeMap<String, usize> = BTreeMap::new();
    for edge in &graph.edges {
        *edges.entry(edge.family.clone()).or_default() += 1;
        *states.entry(format!("{}:{}", edge.family, edge.state)).or_default() += 1;
    }
    println!("nodes: {} {nodes:?}", graph.nodes.len());
    println!("edges: {} {edges:?}", graph.edges.len());
    println!("edges by state: {states:?}");
}
