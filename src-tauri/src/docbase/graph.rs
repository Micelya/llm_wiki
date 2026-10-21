//! The graph of the archive, from the base (N5, D-351, D-410).
//!
//! Nodes are the objects of the base, whether the projection wrote a page
//! for them or not: the active people and organizations (an organization
//! with companies in its group is a group), each visible document with its
//! versions (one node per family, D-340), and the active matters with what
//! a person decided of them (proposed or confirmed) and their status.
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
//! Nothing is read from files: an edge exists only when the relation
//! exists in the base.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params_from_iter, types::Value, OptionalExtension};
use serde::Serialize;

use super::knowledge::pages::{document_families, document_title};
use super::knowledge::vocabulary::vocabulary;
use super::{sql_err, DocBase, Visible};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BaseGraphNode {
    /// The object: `ent-…`, `mat-…`, or `doc-…` (a document with its
    /// versions), the id its screen, its article and its notes go by.
    pub id: String,
    pub label: String,
    /// What it is: the kind of entity (`persona`, `organizacion`...),
    /// `grupo`, `documento` or `asunto`.
    pub kind: String,
    /// The logical path of its page in the base, while it has one.
    pub path: Option<String>,
    /// For a document: one place of it, to open its screen.
    pub location_id: Option<i64>,
    /// For a matter: `proposed` (by the app) or `confirmed` (by a person).
    pub confirmation: Option<String>,
    /// For a matter: its status (`open`...).
    pub status: Option<String>,
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

impl DocBase {
    /// The graph of the visible documents, the active entities and the
    /// active matters.
    pub fn graph(&self, visible: &Visible) -> Result<BaseGraph, String> {
        // Before taking the connection: the families read the base too.
        let families = document_families(self)?;
        let conn = self.conn();
        let words = vocabulary();
        let (condition, bound) = visible.condition("l.origin_id");
        let visible_contents: BTreeSet<i64> = {
            let sql = format!(
                "SELECT DISTINCT f.content_id FROM file f JOIN location l ON l.file_hash = f.hash
                 WHERE l.kind = 'file' AND f.content_id IS NOT NULL AND {condition}"
            );
            let mut statement = conn.prepare(&sql).map_err(sql_err)?;
            let rows = statement
                .query_map(params_from_iter(bound.iter().cloned().map(Value::Text)), |row| row.get(0))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let pages: HashMap<String, String> = {
            let mut statement = conn.prepare("SELECT entity_id, path FROM knowledge_page").map_err(sql_err)?;
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

        // Documents: one node per family with a visible version, named
        // after the version its page is named after.
        let mut family_of: HashMap<i64, String> = HashMap::new();
        for (id, members) in &families {
            let seen: Vec<i64> = members.iter().copied().filter(|m| visible_contents.contains(m)).collect();
            if seen.is_empty() {
                continue;
            }
            let mut location_id: Option<i64> = None;
            for member in &seen {
                let sql = format!(
                    "SELECT l.id FROM location l JOIN file f ON f.hash = l.file_hash
                     WHERE f.content_id = ?1 AND l.kind = 'file' AND {condition} ORDER BY l.id LIMIT 1"
                );
                let mut values = vec![Value::Integer(*member)];
                values.extend(bound.iter().cloned().map(Value::Text));
                location_id = conn.query_row(&sql, params_from_iter(values), |row| row.get(0)).optional().map_err(sql_err)?;
                if location_id.is_some() {
                    break;
                }
            }
            for member in &seen {
                family_of.insert(*member, id.clone());
            }
            nodes.insert(
                id.clone(),
                BaseGraphNode {
                    id: id.clone(),
                    label: document_title(&conn, members[0])?,
                    kind: "documento".into(),
                    path: pages.get(id).cloned(),
                    location_id,
                    confirmation: None,
                    status: None,
                },
            );
        }
        // People and organizations; a reference is data of its document,
        // never a node (D-377).
        let groups: BTreeSet<String> = {
            let mut statement = conn.prepare("SELECT DISTINCT group_id FROM knowledge_group").map_err(sql_err)?;
            let rows = statement.query_map([], |row| row.get(0)).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        {
            let mut statement = conn
                .prepare("SELECT id, kind, name FROM knowledge_entity WHERE state = 'active' ORDER BY id")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (id, kind, name) = row.map_err(sql_err)?;
                if words.is_reference(&kind) {
                    continue;
                }
                let kind = if groups.contains(&id) { "grupo".to_string() } else { kind };
                let path = pages.get(&id).cloned();
                nodes.insert(id.clone(), BaseGraphNode { id, label: name, kind, path, location_id: None, confirmation: None, status: None });
            }
        }
        // Matters a person has not discarded.
        {
            let mut statement = conn
                .prepare("SELECT id, name, confirmation, status FROM knowledge_matter WHERE state = 'active' ORDER BY id")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (id, name, confirmation, status) = row.map_err(sql_err)?;
                let path = pages.get(&id).cloned();
                nodes.insert(
                    id.clone(),
                    BaseGraphNode {
                        id,
                        label: name,
                        kind: "asunto".into(),
                        path,
                        location_id: None,
                        confirmation: Some(confirmation),
                        status: Some(status),
                    },
                );
            }
        }
        let node = |id: &str| -> Option<String> {
            let id = root(id);
            nodes.contains_key(&id).then_some(id)
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
                let Some(document) = family_of.get(&content).cloned() else { continue };
                let Some(target) = node(&entity) else { continue };
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
                    .any(|c| family_of.contains_key(&c));
                if !seen {
                    continue;
                }
                let (Some(source), Some(target)) = (node(&subject), node(&object)) else { continue };
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
        // Documents the computation of similar documents relates; two
        // versions of one document are one node.
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
                let (Some(source), Some(target)) = (family_of.get(&a).cloned(), family_of.get(&b).cloned()) else { continue };
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
                let Some(document) = family_of.get(&content).cloned() else { continue };
                let Some(source) = node(&matter) else { continue };
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
            let label = words.pages.words.get("groupOf").cloned().unwrap_or_else(|| words.type_label("grupo").to_string());
            let mut statement = conn.prepare("SELECT group_id, member_id FROM knowledge_group").map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (group, member) = row.map_err(sql_err)?;
                let (Some(source), Some(target)) = (node(&group), node(&member)) else { continue };
                add_edge(BaseGraphEdge {
                    source,
                    target,
                    family: "group".into(),
                    kind: "group".into(),
                    label: label.clone(),
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
