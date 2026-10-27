//! What the chat's agent reads and keeps, all of it in the base (D-413):
//! the objects it finds (documents, people and organizations, matters)
//! with their article, the passages of the documents, the relations of
//! the graph, the overview of the archive, and the queries a person saves.
//! Nothing here reads or writes a file of the old wiki or of its sources.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::docbase::knowledge::quote::words;
use crate::docbase::{DocBase, Visible};

use super::types::{AgentKnowledgeContext, AgentReference};

/// The most of an article or a page the agent reads at once.
const MAX_READ_CHARS: usize = 60_000;
/// The most of a page a search result carries when its content is asked for.
const MAX_SEARCH_CONTENT_CHARS: usize = 4_000;
/// Relations of an object the graph search tells, at most.
const MAX_RELATIONS: usize = 12;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseSearchToolOutput {
    pub mode: String,
    pub token_hits: usize,
    pub vector_hits: usize,
    pub graph_hits: usize,
    pub references: Vec<AgentReference>,
}

/// A query of the chat the agent saved in the base.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQueryOutput {
    pub id: i64,
    pub title: String,
}

/// What a page names: `document`, `entity`, `matter`, or `overview`.
fn kind_of_page(base: &DocBase, path: &str) -> String {
    base.knowledge_object_of_page(path)
        .ok()
        .flatten()
        .map(|object| object.kind)
        .unwrap_or_else(|| "document".to_string())
}

/// The objects the search of the base finds for `query`: the pages of the
/// documents and of whom the question names, each one by its logical path
/// (the screen it opens), its title and the passage that matched; with
/// `include_content`, the page whole instead of the passage.
pub fn base_search(
    base: &DocBase,
    query: &str,
    top_k: usize,
    include_content: bool,
) -> Result<BaseSearchToolOutput, String> {
    let found = crate::commands::search::search_base(base, query, top_k)?;
    let references = found
        .results
        .iter()
        .map(|result| {
            let snippet = if include_content {
                base.knowledge_page_text(&result.path)
                    .ok()
                    .flatten()
                    .map(|(_, text)| trim(&text, MAX_SEARCH_CONTENT_CHARS))
            } else {
                Some(result.snippet.clone())
            };
            AgentReference {
                title: result.title.clone(),
                path: result.path.clone(),
                kind: kind_of_page(base, &result.path),
                snippet: snippet.filter(|s| !s.trim().is_empty()),
                score: Some(result.score),
                knowledge_context: (!result.graph_related_to.is_empty()).then(|| AgentKnowledgeContext {
                    related_to: result.graph_related_to.clone(),
                    tags: Vec::new(),
                    outgoing_links: Vec::new(),
                    backlinks: Vec::new(),
                    link_count: result.graph_related_to.len(),
                    latest_version: None,
                }),
            }
        })
        .collect();
    Ok(BaseSearchToolOutput {
        mode: found.mode,
        token_hits: found.token_hits,
        vector_hits: found.vector_hits,
        graph_hits: found.graph_hits,
        references,
    })
}

/// One object read from the base: by its id (`ent-…`, `mat-…`, `doc-…`),
/// its logical path, or the name of a link. Its article as the base keeps
/// it, with the notes of a person.
pub fn base_read(base: &DocBase, reference: &str) -> Result<Value, String> {
    let reference = reference.trim();
    if reference.is_empty() {
        return Err("base.read requires path".to_string());
    }
    let path = if crate::docbase::knowledge::articles::kind_of(reference).is_some() && !reference.contains('/') {
        base.knowledge_article(reference)?.map(|article| article.path)
    } else {
        base.knowledge_object_of_page(reference)?.map(|object| object.path)
    };
    let Some(path) = path else {
        return Err(format!("base.read: the base has no page for '{reference}'"));
    };
    let Some((title, text)) = base.knowledge_page_text(&path)? else {
        return Err(format!("base.read: the page '{path}' has no text yet"));
    };
    let object = base.knowledge_object_of_page(&path)?;
    Ok(json!({
        "path": path,
        "title": title,
        "kind": object.as_ref().map(|o| o.kind.clone()),
        "objectId": object.map(|o| o.object_id),
        "content": trim(&text, MAX_READ_CHARS),
    }))
}

/// Keeps a question of the chat with its answer in the base (what the old
/// tool wrote as a page of `wiki/queries`).
pub fn base_save_query(
    base: &DocBase,
    title: &str,
    content: &str,
    question: &str,
    refs: &[String],
) -> Result<SavedQueryOutput, String> {
    let saved = base.save_query(title, question, content, refs)?;
    Ok(SavedQueryOutput { id: saved.id, title: saved.title })
}

/// The passages of the documents that match `query`, from their text in
/// the base (what the wiki's copies of the sources were for): each one by the page of its
/// document, so it opens the document's screen.
pub fn base_source_search(base: &DocBase, query: &str, top_k: usize) -> Result<Vec<AgentReference>, String> {
    let query = query.trim();
    if query.is_empty() {
        return Err("source.search query is required".to_string());
    }
    let found = base.search_documents(&Visible::All, query, top_k.clamp(1, 10), None)?;
    let pages = base.document_pages()?;
    let conn = base.conn();
    let mut out = Vec::new();
    for hit in found.hits {
        let name: Option<String> = conn
            .query_row(
                "SELECT l.name FROM location l JOIN file f ON f.hash = l.file_hash
                 WHERE f.version_id = ?1 ORDER BY l.id LIMIT 1",
                [hit.content_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(crate::docbase::sql_err)?;
        out.push(AgentReference {
            title: name.unwrap_or_else(|| format!("doc-{}", hit.content_id)),
            path: pages.get(&hit.content_id).cloned().unwrap_or_else(|| format!("doc-{}", hit.content_id)),
            kind: "source".to_string(),
            snippet: Some(hit.snippet).filter(|s| !s.trim().is_empty()),
            score: Some(hit.score),
            knowledge_context: None,
        });
    }
    Ok(out)
}

/// The relations the graph of the base keeps of the objects whose name
/// `query` says: each one with its neighbours and what relates them.
pub fn base_graph_search(base: &DocBase, query: &str, top_k: usize) -> Result<Vec<AgentReference>, String> {
    let wanted: BTreeSet<String> = words(query).into_iter().filter(|w| w.chars().count() > 2).collect();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let graph = base.graph(&Visible::All)?;
    let labels: BTreeMap<&str, &str> = graph.nodes.iter().map(|n| (n.id.as_str(), n.label.as_str())).collect();
    let mut matched: Vec<(usize, &crate::docbase::graph::BaseGraphNode)> = graph
        .nodes
        .iter()
        .filter_map(|node| {
            let label = words(&node.label);
            let hits = label.iter().filter(|w| wanted.contains(*w)).count();
            (hits > 0).then_some((hits, node))
        })
        .collect();
    matched.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.label.cmp(&b.1.label)));
    let mut out = Vec::new();
    for (hits, node) in matched.into_iter().take(top_k.clamp(1, 10)) {
        let relations: Vec<(String, String)> = graph
            .edges
            .iter()
            .filter_map(|edge| {
                let other = if edge.source == node.id {
                    &edge.target
                } else if edge.target == node.id {
                    &edge.source
                } else {
                    return None;
                };
                let name = labels.get(other.as_str())?.to_string();
                let label = if edge.label.trim().is_empty() { edge.kind.clone() } else { edge.label.clone() };
                Some((name, label))
            })
            .collect();
        let snippet = relations
            .iter()
            .take(MAX_RELATIONS)
            .map(|(name, label)| format!("{label}: {name}"))
            .collect::<Vec<_>>()
            .join("; ");
        out.push(AgentReference {
            title: node.label.clone(),
            path: node.path.clone().unwrap_or_else(|| node.id.clone()),
            kind: "graph".to_string(),
            snippet: Some(format!("{} ({} relaciones). {snippet}", node.kind, relations.len())),
            score: Some(hits as f64 * 1_000.0 + relations.len() as f64),
            knowledge_context: Some(AgentKnowledgeContext {
                related_to: relations.iter().take(MAX_RELATIONS).map(|(name, _)| name.clone()).collect(),
                tags: Vec::new(),
                outgoing_links: Vec::new(),
                backlinks: Vec::new(),
                link_count: relations.len(),
                latest_version: None,
            }),
        });
    }
    Ok(out)
}

/// The most documents a person can attach to one message of the chat.
pub const MAX_ATTACHED_DOCUMENTS: usize = 8;

/// A document of the base a person attached to a message of the chat
/// (P-243): its name and where it is, the page that opens it, and its text
/// as the base keeps it (`None` while it has none, as a scan not yet
/// recognized).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedDocument {
    pub location_id: i64,
    pub content_id: Option<i64>,
    pub name: String,
    /// "<origin> › <folder>", or the origin alone at its root.
    pub place: String,
    /// The logical path of its page (`wiki/…`), or `doc-<id>` without one.
    pub path: String,
    pub text: Option<String>,
}

/// The documents attached to a message, by the id of their location, read
/// from the base (never from a file on disk): the first
/// `MAX_ATTACHED_DOCUMENTS`, each once; one that is no longer in the base
/// is left out.
pub fn attached_documents(base: &DocBase, location_ids: &[i64]) -> Result<Vec<AttachedDocument>, String> {
    let pages = base.document_pages()?;
    let conn = base.conn();
    let mut out: Vec<AttachedDocument> = Vec::new();
    for id in location_ids {
        if out.len() >= MAX_ATTACHED_DOCUMENTS {
            break;
        }
        if out.iter().any(|d| d.location_id == *id) {
            continue;
        }
        let found: Option<(String, String, String, Option<i64>, Option<String>)> = conn
            .query_row(
                "SELECT l.name, o.name, l.folder, f.version_id, d.body FROM location l
                 JOIN origin o ON o.id = l.origin_id
                 LEFT JOIN file f ON f.hash = l.file_hash
                 LEFT JOIN derivative d ON d.version_id = f.version_id AND d.kind = 'text'
                 WHERE l.id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(crate::docbase::sql_err)?;
        let Some((name, origin, folder, content_id, body)) = found else { continue };
        // Two files of the same document are one document.
        if content_id.is_some() && out.iter().any(|d| d.content_id == content_id) {
            continue;
        }
        let path = content_id
            .and_then(|content| pages.get(&content).cloned())
            .or_else(|| content_id.map(|content| format!("doc-{content}")))
            .unwrap_or_else(|| format!("location-{id}"));
        let text = body.map(|body| without_recognition_marks(&body)).filter(|t| !t.trim().is_empty());
        let place = if folder.is_empty() { origin } else { format!("{origin} › {folder}") };
        out.push(AttachedDocument { location_id: *id, content_id, name, place, path, text });
    }
    Ok(out)
}

/// How an attached document is cited among the references of the answer.
pub fn attached_reference(document: &AttachedDocument) -> AgentReference {
    AgentReference {
        title: document.name.clone(),
        path: document.path.clone(),
        kind: "document".to_string(),
        snippet: Some(format!("Adjuntado al mensaje · {}", document.place)),
        score: None,
        knowledge_context: None,
    }
}

/// The text of a document without the marks the recognition of a scan
/// leaves before each page it read.
fn without_recognition_marks(text: &str) -> String {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("<!-- texto-reconocido"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The overview of the archive, as the base keeps it: the project context
/// of every turn.
pub fn base_overview(base: &DocBase) -> Option<String> {
    base.knowledge_overview().ok().flatten().map(|o| o.body).filter(|b| !b.trim().is_empty())
}

fn trim(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max).collect();
    out.push_str("\n[…]");
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use rusqlite::params;

    use super::*;

    /// A base with one invented document, its page, one person with a page
    /// and the overview.
    pub(crate) fn base_with_pages() -> DocBase {
        let base = DocBase::in_memory().unwrap();
        {
            let conn = base.conn();
            for sql in [
                "INSERT INTO origin (id, name, provider, location, added_at) VALUES ('o1', 'o1', 'local', 'o1', 0)",
                "INSERT INTO version (id, state, first_seen_at) VALUES (1, 'done', 0)",
                "INSERT INTO file (hash, version_id, size) VALUES ('h1', 1, 1)",
                "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, first_seen_at, last_seen_at)
                 VALUES ('o1', 'contrato-prueba.pdf', '', 'contrato-prueba.pdf', 'pdf', 1, 0, 'h1', 0, 0)",
                "INSERT INTO content_fts (rowid, body) VALUES (1, 'Contrato de locación entre Ana Prueba y Ejemplo SA por el depósito de la calle Falsa')",
                "INSERT INTO knowledge_entity (id, kind, name, created_at, updated_at) VALUES ('ent-a', 'persona', 'Ana Prueba', 0, 0)",
                "INSERT INTO knowledge_alias (entity_id, normalized, written) VALUES ('ent-a', 'ana prueba', 'Ana Prueba')",
                "INSERT INTO knowledge_mention (version_id, ref, kind, written, normalized, role, quote, entity_id)
                 VALUES (1, 'm1', 'persona', 'Ana Prueba', 'ana prueba', 'parte', 'Ana Prueba, locataria', 'ent-a')",
            ] {
                conn.execute(sql, []).unwrap();
            }
            for (id, path, body) in [
                (
                    "doc-1",
                    "wiki/sources/contrato-prueba.md",
                    "---\ntitle: Contrato de prueba\n---\n# Contrato de prueba\n\nLocación del depósito de la calle Falsa.",
                ),
                ("ent-a", "wiki/entities/ana-prueba.md", "---\ntitle: Ana Prueba\n---\n# Ana Prueba\n\nLocataria del depósito."),
            ] {
                conn.execute(
                    "INSERT INTO knowledge_page (entity_id, path, fingerprint, written_at, body) VALUES (?1, ?2, '', 0, ?3)",
                    params![id, path, body],
                )
                .unwrap();
            }
            conn.execute(
                "INSERT INTO knowledge_overview (id, path, body, fingerprint, written_at) VALUES (1, 'wiki/overview.md', '# Resumen\n\nUn archivo de prueba.', '', 0)",
                [],
            )
            .unwrap();
            crate::docbase::search::index_content(&conn, 1).unwrap();
        }
        base
    }

    #[test]
    fn the_search_finds_objects_of_the_base_with_their_text() {
        let base = base_with_pages();
        let found = base_search(&base, "depósito Falsa", 5, false).unwrap();
        assert_eq!(found.mode, "base");
        let first = &found.references[0];
        assert_eq!(first.path, "wiki/sources/contrato-prueba.md");
        assert_eq!(first.kind, "document");
        let whole = base_search(&base, "depósito Falsa", 5, true).unwrap();
        assert!(whole.references[0].snippet.as_deref().unwrap().contains("Locación del depósito"));
    }

    #[test]
    fn an_object_is_read_by_its_id_its_path_or_its_link() {
        let base = base_with_pages();
        for reference in ["ent-a", "wiki/entities/ana-prueba.md", "ana-prueba"] {
            let read = base_read(&base, reference).unwrap();
            assert_eq!(read["path"], "wiki/entities/ana-prueba.md");
            assert_eq!(read["kind"], "entity");
            assert!(read["content"].as_str().unwrap().contains("Locataria"));
        }
        assert!(base_read(&base, "nadie").is_err());
        assert!(base_read(&base, "../wiki/secret.md").is_err());
    }

    #[test]
    fn a_saved_query_lives_in_the_base() {
        let base = base_with_pages();
        let saved = base_save_query(&base, "Quién alquila", "La locataria es Ana Prueba.", "¿Quién alquila?", &["ent-a".into()]).unwrap();
        let queries = base.saved_queries().unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(queries[0].id, saved.id);
        assert_eq!(queries[0].refs, vec!["ent-a".to_string()]);
        assert!(base_save_query(&base, " ", "x", "", &[]).is_err());
    }

    #[test]
    fn the_source_search_reads_the_text_of_the_documents() {
        let base = base_with_pages();
        let found = base_source_search(&base, "calle Falsa", 5).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].title, "contrato-prueba.pdf");
        assert_eq!(found[0].path, "wiki/sources/contrato-prueba.md");
        assert!(found[0].snippet.as_deref().unwrap().contains("Falsa"));
        assert!(base_source_search(&base, "  ", 5).is_err());
    }

    #[test]
    fn an_attached_document_is_read_from_the_base_with_its_name_and_place() {
        let base = base_with_pages();
        {
            let conn = base.conn();
            conn.execute(
                "INSERT INTO derivative (version_id, kind, body, engine, created_at) VALUES (1, 'text', ?1, 'prueba', 0)",
                ["<!-- texto-reconocido motor=\"prueba\" -->\nContrato de locación del depósito de la calle Falsa."],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, first_seen_at, last_seen_at)
                 VALUES ('o1', 'Casos/copia.pdf', 'Casos', 'copia.pdf', 'pdf', 1, 0, 'h1', 0, 0)",
                [],
            )
            .unwrap();
        }
        let ids: Vec<i64> = {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT id FROM location ORDER BY id").unwrap();
            let rows = statement.query_map([], |row| row.get(0)).unwrap();
            rows.map(Result::unwrap).collect()
        };
        // The second file is the same document: it is attached once; one
        // that is not in the base is left out.
        let found = attached_documents(&base, &[ids[0], ids[1], 9_999]).unwrap();
        assert_eq!(found.len(), 1);
        let document = &found[0];
        assert_eq!((document.name.as_str(), document.place.as_str()), ("contrato-prueba.pdf", "o1"));
        assert_eq!(document.path, "wiki/sources/contrato-prueba.md");
        assert_eq!(document.text.as_deref(), Some("Contrato de locación del depósito de la calle Falsa."));
        let copy = attached_documents(&base, &[ids[1]]).unwrap();
        assert_eq!(copy[0].place, "o1 › Casos");
        let reference = attached_reference(document);
        assert_eq!((reference.kind.as_str(), reference.path.as_str()), ("document", "wiki/sources/contrato-prueba.md"));
        assert!(reference.snippet.unwrap().contains("o1"));
        let many = vec![ids[0]; 20];
        assert_eq!(attached_documents(&base, &many).unwrap().len(), 1);
    }

    #[test]
    fn the_overview_comes_from_the_base() {
        let base = base_with_pages();
        assert!(base_overview(&base).unwrap().contains("Un archivo de prueba"));
        assert!(base_overview(&DocBase::in_memory().unwrap()).is_none());
    }
}
