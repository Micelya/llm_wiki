//! The knowledge layer: the people, organizations and matters the
//! documents name, how each document names them and with what role, and
//! what the documents say of them, every piece with the quote that backs
//! it, verified in the text by a computation (D-171).
//!
//! It lives in the space's base, between the documents and the wiki:
//! the wiki's entity pages are a projection of this record (D-175).
//! Stages (design: `docs/design/conocimiento-propuesta.md`):
//!
//! 1. **Extraction** (`extract.rs`): one reading of each document by the
//!    model, in strict JSON, with every quote checked (`quote.rs`).
//! 2. **Resolution of identities** (`resolve.rs`, with the names read by
//!    `names.rs`), the decisions of a person (`decide.rs`) and the claims
//!    consolidated from the evidence (`claims.rs`); what the screens read
//!    of the record (`entities.rs`).
//! 3. **Projection** (`pages.rs`): the page of each entity in the
//!    archive's wiki, the block of entities of each document's page and
//!    the overview, from the record.
//! 4. **Changes and deletions** (`changes.rs`): what a document that goes
//!    away, comes back, is replaced or is left out takes with it, and what
//!    goes on to the document that takes its place.
//! 5. Relations, 6. the attention view: later stages, on the tables
//!    already here (`schema.rs`, V8 to V10).
//!
//! The types of entity, roles, identifiers and predicates are data
//! (`vocabulary.rs`, R-26).

#[cfg(test)]
pub(crate) mod check;
#[cfg(test)]
mod check_identity;
#[cfg(test)]
mod check_pages;
pub(crate) mod changes;
pub(crate) mod claims;
pub mod decide;
pub mod entities;
pub mod extract;
pub mod names;
pub mod pages;
pub mod quote;
pub mod resolve;
#[cfg(test)]
pub(crate) mod tests;
#[cfg(test)]
mod tests_changes;
#[cfg(test)]
mod tests_identity;
#[cfg(test)]
pub(crate) mod tests_pages;
pub mod vocabulary;

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{sql_err, DocBase, Visible};
use crate::source_volume::asking::AskConfig;

/// SQL condition: the content `{column}` has a location the caller can
/// see. Its parameters come from `Visible::condition`.
fn visible_content(column: &str, visible: &Visible) -> (String, Vec<String>) {
    let (condition, params) = visible.condition("vl.origin_id");
    (
        format!(
            "EXISTS (SELECT 1 FROM file vf JOIN location vl ON vl.file_hash = vf.hash
                     WHERE vf.content_id = {column} AND {condition})"
        ),
        params,
    )
}

/// How far the reading of the archive went, for the screens and the
/// attention view.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeStatus {
    /// Documents with enough text to be read.
    pub documents: usize,
    /// Read with the current question and vocabulary, over their current text.
    pub extracted: usize,
    /// Read, but over another text, question or vocabulary: read again.
    pub stale: usize,
    /// Still to read (never read, or stale).
    pub pending: usize,
    /// The model's answer could not be used.
    pub failed: usize,
    pub mentions: usize,
    pub identifiers: usize,
    pub evidence: usize,
    /// What the model said and the computation discarded, by reason
    /// (a quote not found in the text, a name outside its quote…).
    pub discarded: usize,
    pub discarded_by_reason: BTreeMap<String, usize>,
    /// Answers of the model kept for this layer (readings and identities).
    pub answers: usize,
    /// Active entities, by kind.
    pub entities: usize,
    pub entities_by_kind: BTreeMap<String, usize>,
    /// Pairs left for a person to decide.
    pub doubts: usize,
    pub claims: usize,
    /// Claims with another value for the same time.
    pub conflicts: usize,
    /// Readings or decisions not resolved yet.
    pub to_resolve: usize,
    /// Entities with a page in the archive's wiki, and entities whose page
    /// is to be written again.
    pub pages: usize,
    pub to_project: usize,
    /// The engine that reads while the archive's wiki is active.
    pub engine: Option<AskConfig>,
}

/// One thing the computation discarded from an answer, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discarded {
    /// "mention", "identifier" or "claim".
    pub what: String,
    pub reason: String,
    /// The name, value or predicate the model gave.
    pub said: String,
    pub quote: String,
}

/// A mention as the screens show it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MentionView {
    pub id: i64,
    pub kind: String,
    pub written: String,
    pub role: String,
    pub quote: String,
    pub page: Option<i64>,
    pub same_as: Option<i64>,
    pub entity_id: Option<String>,
    pub identifiers: Vec<IdentifierView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifierView {
    pub kind: String,
    pub value: String,
    pub quote: String,
    pub page: Option<i64>,
    /// The base had already recognized it by its shape, without a model.
    pub by_shape: bool,
}

/// What a document says of one of its mentions.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceView {
    pub id: i64,
    pub subject: i64,
    pub predicate: String,
    pub object_mention: Option<i64>,
    pub object_value: Option<String>,
    pub quote: String,
    pub page: Option<i64>,
    pub fact_time: Option<String>,
}

/// What the layer knows of one document.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentKnowledge {
    pub content_id: Option<i64>,
    /// "done", "failed", "stale" or "pending".
    pub state: String,
    pub engine: Option<String>,
    pub extracted_at: Option<i64>,
    pub mentions: Vec<MentionView>,
    pub evidence: Vec<EvidenceView>,
    pub discarded: Vec<Discarded>,
    pub error: Option<String>,
}

impl DocBase {
    /// How far the reading of the archive went, over what `visible` lets see.
    pub fn knowledge_status(&self, visible: &Visible) -> Result<KnowledgeStatus, String> {
        let standings = extract::standings(&self.conn(), visible, None)?;
        let conn = self.conn();
        let mut status = KnowledgeStatus { documents: standings.len(), ..Default::default() };
        let mut read = 0;
        for standing in standings {
            let Some(reading) = standing.reading else { continue };
            read += 1;
            if !reading.current {
                status.stale += 1;
                continue;
            }
            match reading.state.as_str() {
                "done" => status.extracted += 1,
                _ => status.failed += 1,
            }
            let discarded: Vec<Discarded> = serde_json::from_str(&reading.discarded).unwrap_or_default();
            status.discarded += discarded.len();
            for item in discarded {
                *status.discarded_by_reason.entry(item.reason).or_default() += 1;
            }
        }
        status.pending = status.documents - read + status.stale;
        let count = |sql: &str| -> Result<usize, String> {
            let (condition, params) = visible_content("m.content_id", visible);
            conn.query_row(&format!("{sql} WHERE {condition}"), rusqlite::params_from_iter(params.iter()), |row| {
                row.get::<_, i64>(0)
            })
            .map(|n| n as usize)
            .map_err(sql_err)
        };
        status.mentions = count("SELECT count(*) FROM knowledge_mention m")?;
        status.identifiers =
            count("SELECT count(*) FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id")?;
        status.evidence = count("SELECT count(*) FROM knowledge_evidence m")?;
        status.answers = conn
            .query_row("SELECT count(*) FROM model_answer WHERE task IN (?1, ?2, ?3)", [extract::TASK, resolve::TASK, pages::TASK], |row| row.get::<_, i64>(0))
            .map_err(sql_err)? as usize;
        {
            let (condition, params) = visible_content("m.content_id", visible);
            let mut statement = conn
                .prepare(&format!(
                    "SELECT e.kind, count(DISTINCT e.id) FROM knowledge_entity e JOIN knowledge_mention m ON m.entity_id = e.id
                     WHERE e.state = 'active' AND {condition} GROUP BY e.kind"
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(params.iter()), |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (kind, n) = row.map_err(sql_err)?;
                status.entities += n as usize;
                status.entities_by_kind.insert(kind, n as usize);
            }
        }
        drop(conn);
        status.doubts = self.knowledge_doubts(visible)?.len();
        let conn = self.conn();
        let (condition, params) = visible_content("e.content_id", visible);
        let claims: Vec<(String, i64)> = {
            let mut statement = conn
                .prepare(&format!(
                    "SELECT c.state, count(DISTINCT c.id) FROM knowledge_claim c JOIN knowledge_evidence e ON e.claim_id = c.id
                     WHERE {condition} GROUP BY c.state"
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(params.iter()), |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (state, n) in claims {
            if state != "retired" {
                status.claims += n as usize;
            }
            if state == "conflict" {
                status.conflicts += n as usize;
            }
        }
        status.to_resolve = conn
            .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')", [], |row| row.get::<_, i64>(0))
            .map_err(sql_err)? as usize;
        status.pages = conn
            .query_row("SELECT count(*) FROM knowledge_page WHERE state = 'projected'", [], |row| row.get::<_, i64>(0))
            .map_err(sql_err)? as usize;
        status.to_project = conn
            .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope = 'project'", [], |row| row.get::<_, i64>(0))
            .map_err(sql_err)? as usize;
        status.engine = super::similar::classify::ask_engine(&conn)?;
        Ok(status)
    }

    /// What the layer knows of the document at this location.
    pub fn knowledge_of(&self, visible: &Visible, location_id: i64) -> Result<DocumentKnowledge, String> {
        let conn = self.conn();
        let (condition, params) = visible.condition("l.origin_id");
        let mut values: Vec<rusqlite::types::Value> = vec![location_id.into()];
        values.extend(params.into_iter().map(rusqlite::types::Value::from));
        let content: Option<i64> = conn
            .query_row(
                &format!(
                    "SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.id = ? AND {condition}"
                ),
                rusqlite::params_from_iter(values.iter()),
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        let Some(content_id) = content else {
            return Ok(DocumentKnowledge { state: "pending".into(), ..Default::default() });
        };
        knowledge_of_content(&conn, content_id)
    }
}

/// What the layer knows of one content.
pub(crate) fn knowledge_of_content(conn: &Connection, content_id: i64) -> Result<DocumentKnowledge, String> {
    let mut out = DocumentKnowledge { content_id: Some(content_id), state: "pending".into(), ..Default::default() };
    let row: Option<(String, Option<String>, i64, String, Option<String>)> = conn
        .query_row(
            "SELECT state, engine, extracted_at, discarded, error FROM knowledge_extraction WHERE content_id = ?1",
            [content_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(sql_err)?;
    if let Some((state, engine, at, discarded, error)) = row {
        let current = extract::standings(conn, &Visible::All, Some(content_id))?
            .first()
            .and_then(|s| s.reading.as_ref())
            .is_some_and(|r| r.current);
        out.state = if current { state } else { "stale".into() };
        out.engine = engine;
        out.extracted_at = Some(at);
        out.discarded = serde_json::from_str(&discarded).unwrap_or_default();
        out.error = error;
    }
    let mut identifiers: BTreeMap<i64, Vec<IdentifierView>> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT i.mention_id, i.kind, i.value, i.quote, i.page, i.by_shape FROM knowledge_identifier i
                 JOIN knowledge_mention m ON m.id = i.mention_id WHERE m.content_id = ?1 ORDER BY i.mention_id, i.kind, i.value",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([content_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    IdentifierView {
                        kind: row.get(1)?,
                        value: row.get(2)?,
                        quote: row.get(3)?,
                        page: row.get(4)?,
                        by_shape: row.get(5)?,
                    },
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (mention, identifier) = row.map_err(sql_err)?;
            identifiers.entry(mention).or_default().push(identifier);
        }
    }
    {
        let mut statement = conn
            .prepare(
                "SELECT m.id, m.kind, m.written, m.role, m.quote, m.page, s.id, m.entity_id FROM knowledge_mention m
                 LEFT JOIN knowledge_mention s ON s.content_id = m.content_id AND s.ref = m.same_as
                 WHERE m.content_id = ?1 ORDER BY m.id",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([content_id], |row| {
                Ok(MentionView {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    written: row.get(2)?,
                    role: row.get(3)?,
                    quote: row.get(4)?,
                    page: row.get(5)?,
                    same_as: row.get(6)?,
                    entity_id: row.get(7)?,
                    identifiers: Vec::new(),
                })
            })
            .map_err(sql_err)?;
        for row in rows {
            let mut mention = row.map_err(sql_err)?;
            mention.identifiers = identifiers.remove(&mention.id).unwrap_or_default();
            out.mentions.push(mention);
        }
    }
    let mut statement = conn
        .prepare(
            "SELECT id, subject_mention, predicate, object_mention, object_value, quote, page, fact_time
             FROM knowledge_evidence WHERE content_id = ?1 ORDER BY id",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([content_id], |row| {
            Ok(EvidenceView {
                id: row.get(0)?,
                subject: row.get(1)?,
                predicate: row.get(2)?,
                object_mention: row.get(3)?,
                object_value: row.get(4)?,
                quote: row.get(5)?,
                page: row.get(6)?,
                fact_time: row.get(7)?,
            })
        })
        .map_err(sql_err)?;
    for row in rows {
        out.evidence.push(row.map_err(sql_err)?);
    }
    Ok(out)
}

/// Marks work left for a later pass (resolution, retiring evidence,
/// projecting a page).
pub(crate) fn mark_dirty(conn: &Connection, scope: &str, reference: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO knowledge_dirty (scope, ref, since) VALUES (?1, ?2, ?3)",
        params![scope, reference, super::now_ms()],
    )
    .map_err(sql_err)?;
    Ok(())
}
