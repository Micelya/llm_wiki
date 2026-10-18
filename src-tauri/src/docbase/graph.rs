//! The graph of the wiki of the archive, from the base (N5, D-351).
//!
//! Nodes are the pages the base writes: each document (or family of
//! versions), each person, organization, group and matter with a page.
//! Edges are the relations the base keeps, each with its type, its label
//! from the vocabulary, a quote when it has one and its state:
//!
//! - a document and the entities it names (`role`: party, signatory...);
//! - what the documents say between two entities (`predicate`: represents,
//!   owes...), only when a visible document says it;
//! - two documents the computation of similar documents relates (same
//!   template, signing certificate of...), never the discarded ones;
//! - a matter and its documents; a group and its companies.
//!
//! The links written in the files are not read: an edge exists only when
//! the relation exists in the base.

use std::collections::{BTreeMap, HashMap};

use rusqlite::{params_from_iter, types::Value};
use serde::Serialize;

use super::knowledge::vocabulary::vocabulary;
use super::{sql_err, DocBase, Visible};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseGraphNode {
    /// The page's file name without `.md`, as the graph of pages names it.
    pub id: String,
    pub label: String,
    /// The page type: `source` for documents, `entity` for the rest.
    pub page_type: String,
    /// What it is: `document`, or the kind of entity (`persona`...).
    pub kind: String,
    /// The page, relative to the project.
    pub path: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseGraphEdge {
    pub source: String,
    pub target: String,
    /// `mention`, `claim`, `similar`, `matter` or `group`.
    pub family: String,
    /// The type inside the family: the role, the predicate, the relation.
    pub kind: String,
    /// How the vocabulary names it.
    pub label: String,
    pub quote: Option<String>,
    /// `proposed`, `confirmed`, `joined`, `firm`, the claim's certainty...
    pub state: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaseGraph {
    pub nodes: Vec<BaseGraphNode>,
    pub edges: Vec<BaseGraphEdge>,
}

fn stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

impl DocBase {
    /// The graph of the visible documents and the entities with a page.
    pub fn graph(&self, visible: &Visible) -> Result<BaseGraph, String> {
        let document_pages = self.document_pages()?;
        let conn = self.conn();
        let words = vocabulary();
        let (condition, bound) = visible.condition("l.origin_id");
        let visible_contents: Vec<i64> = {
            let sql = format!(
                "SELECT DISTINCT f.content_id FROM file f JOIN location l ON l.file_hash = f.hash WHERE {condition}"
            );
            let mut statement = conn.prepare(&sql).map_err(sql_err)?;
            let rows = statement
                .query_map(params_from_iter(bound.iter().cloned().map(Value::Text)), |row| row.get(0))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let page_of_content: HashMap<i64, String> = visible_contents
            .iter()
            .filter_map(|c| document_pages.get(c).map(|p| (*c, p.clone())))
            .collect();
        let entity_pages: HashMap<String, String> = {
            let mut statement = conn
                .prepare("SELECT entity_id, path FROM knowledge_page WHERE entity_id NOT LIKE 'doc-%'")
                .map_err(sql_err)?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let merged: HashMap<String, String> = {
            let mut statement = conn
                .prepare("SELECT id, merged_into FROM knowledge_entity WHERE state = 'merged' AND merged_into IS NOT NULL")
                .map_err(sql_err)?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let root = |id: &str| {
            let mut id = id.to_string();
            for _ in 0..10 {
                match merged.get(&id) {
                    Some(next) => id = next.clone(),
                    None => break,
                }
            }
            id
        };

        let mut nodes: BTreeMap<String, BaseGraphNode> = BTreeMap::new();
        let mut edges: BTreeMap<(String, String, String, String), BaseGraphEdge> = BTreeMap::new();
        let mut add_edge = |edge: BaseGraphEdge| {
            if edge.source == edge.target {
                return;
            }
            let key = (edge.source.clone(), edge.target.clone(), edge.family.clone(), edge.kind.clone());
            edges.entry(key).or_insert(edge);
        };

        // Documents.
        let mut contents: Vec<(&i64, &String)> = page_of_content.iter().collect();
        contents.sort();
        for (content, path) in contents {
            let id = stem(path);
            if nodes.contains_key(&id) {
                continue;
            }
            // The title the wiki index read from the page, or the name of
            // the document's first file.
            let label: Option<String> = conn
                .query_row("SELECT title FROM wiki_page WHERE path = ?1 ORDER BY id LIMIT 1", [path], |row| row.get(0))
                .ok()
                .or_else(|| {
                    conn.query_row(
                        "SELECT l.name FROM file f JOIN location l ON l.file_hash = f.hash WHERE f.content_id = ?1 ORDER BY l.id LIMIT 1",
                        [content],
                        |row| row.get(0),
                    )
                    .ok()
                });
            nodes.entry(id.clone()).or_insert(BaseGraphNode {
                id: id.clone(),
                label: label.unwrap_or_else(|| id.clone()),
                page_type: "source".into(),
                kind: "document".into(),
                path: path.clone(),
            });
        }
        // Entities and matters with a page.
        let entity_node = |id: &str, nodes: &mut BTreeMap<String, BaseGraphNode>| -> Option<String> {
            let path = entity_pages.get(id)?;
            let node_id = stem(path);
            if !nodes.contains_key(&node_id) {
                let (name, kind): (String, String) = conn
                    .query_row(
                        "SELECT name, kind FROM knowledge_entity WHERE id = ?1
                         UNION ALL SELECT name, 'asunto' FROM knowledge_matter WHERE id = ?1",
                        [id],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .unwrap_or_else(|_| (id.to_string(), "entity".to_string()));
                nodes.insert(
                    node_id.clone(),
                    BaseGraphNode { id: node_id.clone(), label: name, page_type: "entity".into(), kind, path: path.clone() },
                );
            }
            Some(node_id)
        };

        // A document and the entities it names, one edge per role.
        {
            let mut statement = conn
                .prepare(
                    "SELECT content_id, entity_id, role, min(quote) FROM knowledge_mention
                     WHERE entity_id IS NOT NULL GROUP BY content_id, entity_id, role",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (content, entity, role, quote) = row.map_err(sql_err)?;
                let Some(document) = page_of_content.get(&content).map(|p| stem(p)) else { continue };
                let Some(target) = entity_node(&root(&entity), &mut nodes) else { continue };
                add_edge(BaseGraphEdge {
                    source: document,
                    target,
                    family: "mention".into(),
                    label: words.role_label(&role).to_string(),
                    kind: role,
                    quote: Some(quote).filter(|q| !q.trim().is_empty()),
                    state: "read".into(),
                });
            }
        }
        // What visible documents say between two entities.
        {
            let mut statement = conn
                .prepare(
                    "SELECT c.subject, c.object_entity, c.predicate, c.state, c.certainty,
                            (SELECT e.quote FROM knowledge_evidence e WHERE e.claim_id = c.id ORDER BY e.id LIMIT 1),
                            (SELECT group_concat(DISTINCT e.content_id) FROM knowledge_evidence e WHERE e.claim_id = c.id)
                     FROM knowledge_claim c WHERE c.object_entity IS NOT NULL AND c.state != 'retired'",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (subject, object, predicate, state, certainty, quote, contents) = row.map_err(sql_err)?;
                let seen = contents
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|c| c.trim().parse::<i64>().ok())
                    .any(|c| page_of_content.contains_key(&c));
                if !seen {
                    continue;
                }
                let (Some(source), Some(target)) = (entity_node(&root(&subject), &mut nodes), entity_node(&root(&object), &mut nodes)) else {
                    continue;
                };
                let level = certainty
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                    .and_then(|v| v.get("level").and_then(|l| l.as_str()).map(str::to_string));
                add_edge(BaseGraphEdge {
                    source,
                    target,
                    family: "claim".into(),
                    label: words.predicate_label(&predicate).to_string(),
                    kind: predicate,
                    quote,
                    state: level.unwrap_or(state),
                });
            }
        }
        // Documents the computation of similar documents relates.
        {
            let mut statement = conn
                .prepare(
                    "SELECT a, b, coalesce(chosen_relation, relation), state FROM similar_pair WHERE state != 'discarded'",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (a, b, relation, state) = row.map_err(sql_err)?;
                let (Some(source), Some(target)) = (page_of_content.get(&a).map(|p| stem(p)), page_of_content.get(&b).map(|p| stem(p))) else {
                    continue;
                };
                let label = words.pages.words.get(&format!("rel-{relation}")).cloned().unwrap_or_else(|| relation.replace('-', " "));
                add_edge(BaseGraphEdge { source, target, family: "similar".into(), kind: relation, label, quote: None, state });
            }
        }
        // A matter and its documents.
        {
            let mut statement = conn
                .prepare("SELECT matter_id, content_id, state FROM knowledge_matter_document")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (matter, content, state) = row.map_err(sql_err)?;
                let Some(document) = page_of_content.get(&content).map(|p| stem(p)) else { continue };
                let Some(source) = entity_node(&matter, &mut nodes) else { continue };
                add_edge(BaseGraphEdge {
                    source,
                    target: document,
                    family: "matter".into(),
                    kind: "matter".into(),
                    label: words.type_label("asunto").to_string(),
                    quote: None,
                    state,
                });
            }
        }
        // A group and its companies.
        {
            let mut statement = conn.prepare("SELECT group_id, member_id FROM knowledge_group").map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (group, member) = row.map_err(sql_err)?;
                let (Some(source), Some(target)) = (entity_node(&root(&group), &mut nodes), entity_node(&root(&member), &mut nodes)) else {
                    continue;
                };
                add_edge(BaseGraphEdge {
                    source,
                    target,
                    family: "group".into(),
                    kind: "group".into(),
                    label: words.type_label("grupo").to_string(),
                    quote: None,
                    state: "proposed".into(),
                });
            }
        }
        Ok(BaseGraph { nodes: nodes.into_values().collect(), edges: edges.into_values().collect() })
    }
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
