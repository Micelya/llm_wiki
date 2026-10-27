//! Stage 6 of the knowledge layer: what the screens and the attention
//! view read of the record, besides the entities and doubts of
//! `entities.rs`.
//!
//! - What needs a person (`knowledge_attention`): readings that failed,
//!   readings that lost many quotes (a badly recognized scan), claims in
//!   conflict, recent joins by identifier of different names, the matter
//!   the model proposed for loose documents, pages kept by their notes and
//!   entities that left with the documents left out, and how far the
//!   reading, the resolution and the projection went. Data, not words: the
//!   screen writes them with its vocabulary (D-156).
//! - The matters of the archive and one matter with its documents, why
//!   each is in it, its parties and the decisions about it.
//! - The entities a document names, with their roles, and its matter.
//! - What an entity's detail adds to `knowledge_entity`: its page, the
//!   matter of each of its documents, the decisions about it and the
//!   claims of other entities about it.
//!
//! Every query takes what `visible` lets see.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};
use serde::Serialize;

use super::decide::{DecisionView, IdentityDecision};
use super::entities::document;
use super::matter_records::{MatterQuestion, MatterRecord};
use super::matters::{content_key, Reason};
use super::resolve::cluster_key;
use super::vocabulary::vocabulary;
use super::{changes, extract, visible_content, Discarded, KnowledgeStatus};
use crate::docbase::{now_ms, sql_err, DocBase, Visible};

/// A document, as a line of a screen shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRef {
    pub content_id: i64,
    /// A location of it the caller can see.
    pub location_id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailedReading {
    pub document: DocumentRef,
    pub error: Option<String>,
}

/// A reading whose quotes were discarded in a large share.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoisyReading {
    pub document: DocumentRef,
    /// Pieces kept (mentions and evidence) and discarded for their quote.
    pub kept: usize,
    pub discarded: usize,
}

/// The claims of an entity about one thing whose values differ.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictGroup {
    pub entity_id: String,
    pub entity_name: String,
    pub kind: String,
    pub predicate: String,
    /// The entity the values are said within (a company), when there is one.
    pub within: Option<String>,
    pub values: Vec<String>,
    pub claims: Vec<i64>,
}

/// An entity a strong identifier joined although its names differ.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentifierJoin {
    pub entity_id: String,
    pub name: String,
    pub kind: String,
    /// Every way its documents write it.
    pub names: Vec<String>,
    /// The strong identifiers that joined it, as written.
    pub identifiers: Vec<String>,
    pub documents: usize,
    /// When the latest document that brought a join by identifier was read.
    pub read_at: i64,
}

/// A matter the grouping proposed, for a person to confirm or discard
/// (D-381), with why its documents are together.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterToConfirm {
    pub id: String,
    pub name: String,
    pub documents: usize,
    /// In how many of its documents each kind of reason is ("numero",
    /// "asunto"...).
    pub reasons: BTreeMap<String, usize>,
}

/// A loose document the model proposed for a matter.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedMatter {
    pub document: DocumentRef,
    pub matter_id: String,
    pub matter_name: String,
    pub reason: Option<String>,
}

/// A page of an entity that is no longer projected and stays for its notes
/// or for the pages that link it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeptPage {
    pub entity_id: String,
    pub path: String,
    pub name: Option<String>,
    /// Where the entity went, when it was joined with another.
    pub merged_into: Option<String>,
}

/// An entity that left the record because the documents that named it were
/// left out.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftWithDocuments {
    pub entity_id: String,
    pub name: String,
    pub kind: String,
    pub documents: Vec<DocumentRef>,
}

/// A decision of a person that no longer finds what it named in the
/// documents read again (D-321): it is not applied to a guess, and it is
/// not dropped in silence.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionReview {
    pub decision: DecisionView,
    /// The document it was about, when it names one the caller can see.
    pub document: Option<DocumentRef>,
    /// "no-encontrada" or "varias".
    pub reason: String,
}

/// What the layer brings to the attention view.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeAttention {
    pub status: KnowledgeStatus,
    pub failed: Vec<FailedReading>,
    pub noisy: Vec<NoisyReading>,
    pub conflicts: Vec<ConflictGroup>,
    pub identifier_joins: Vec<IdentifierJoin>,
    pub proposed_matters: Vec<ProposedMatter>,
    /// The matters the grouping proposed that no person confirmed yet.
    pub matters_to_confirm: Vec<MatterToConfirm>,
    /// Pages whose synthesis is still to be written, and sentences of the
    /// syntheses the computation discarded (no valid reference).
    pub synthesis_pending: usize,
    pub discarded_sentences: usize,
    pub kept_pages: Vec<KeptPage>,
    pub left_with_documents: Vec<LeftWithDocuments>,
    /// Decisions of a person to review: what they named is not in the
    /// documents as they read now.
    pub decisions_to_review: Vec<DecisionReview>,
    /// Whether a matter that looks like the one of a record is the same.
    pub matters_to_join: Vec<MatterQuestion>,
}

/// A matter in the list of matters.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterSummary {
    pub id: String,
    pub name: String,
    /// "open", "closed" or "archived" (D-475): a closed or archived one is
    /// hidden by the screen of matters unless asked for.
    pub status: String,
    /// "proposed" (by the grouping) or "person".
    pub origin: String,
    /// "proposed" or "confirmed".
    pub confirmation: String,
    pub documents: usize,
    /// Loose documents the model proposed for it.
    pub proposed: usize,
    pub page_path: Option<String>,
}

/// Why a document is in its matter: one link with another document.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterReason {
    /// "persona", "documento", "asunto", "numero" or "importe".
    pub kind: String,
    /// The relation, the shared reference (normalized), the number or the amount.
    pub value: String,
    /// How the documents write the shared reference, for "asunto".
    pub value_name: Option<String>,
    pub with: Option<DocumentRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterDocument {
    pub document: DocumentRef,
    /// "firm" (evidence or a person put it there) or "proposed" (the model).
    pub state: String,
    pub reasons: Vec<MatterReason>,
    /// What the model said, for a proposed one.
    pub proposal_reason: Option<String>,
}

/// A party or a person of a matter.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterParty {
    pub entity_id: String,
    pub kind: String,
    pub name: String,
    pub roles: Vec<String>,
    pub documents: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterDetail {
    pub id: String,
    pub name: String,
    /// "active" or "retired".
    pub state: String,
    pub status: String,
    pub origin: String,
    /// "proposed", "confirmed" or "discarded".
    pub confirmation: String,
    pub page_path: Option<String>,
    pub documents: Vec<MatterDocument>,
    pub parties: Vec<MatterParty>,
    /// The decisions of a person about its documents, active or undone.
    pub decisions: Vec<DecisionView>,
    /// Its records of a court, with their official titles.
    pub records: Vec<MatterRecord>,
}

/// The matter of one document, firm or proposed.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMatter {
    pub content_id: i64,
    pub matter_id: String,
    pub name: String,
    pub proposed: bool,
    pub page_path: Option<String>,
}

/// An entity a document names.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEntity {
    /// None while its mentions are not resolved.
    pub entity_id: Option<String>,
    pub kind: String,
    pub name: String,
    /// How this document writes it.
    pub written: Vec<String>,
    pub roles: Vec<String>,
    pub page_path: Option<String>,
}

/// What the layer knows of a document, for its detail.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEntities {
    pub content_id: Option<i64>,
    /// "done", "failed", "stale" or "pending".
    pub state: String,
    /// Read with an earlier version of the instructions (D-432).
    pub older: bool,
    /// A person asked to read it again, and it waits for it.
    pub reread: bool,
    pub error: Option<String>,
    pub entities: Vec<DocumentEntity>,
    /// The record numbers, files and acts it refers to: data of the
    /// document, not entities (D-377).
    pub references: Vec<super::references::DocumentReference>,
    pub matters: Vec<DocumentMatter>,
    /// Pieces of the reading discarded by the check of the quotes.
    pub discarded: usize,
}

/// A claim of another entity about this one ("represents", "partner of").
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingClaim {
    pub claim_id: i64,
    pub subject_id: String,
    pub subject_name: String,
    pub subject_kind: String,
    pub predicate: String,
    pub state: String,
    pub documents: usize,
}

fn with_params(first: Vec<Value>, params: Vec<String>) -> Vec<Value> {
    let mut out = first;
    out.extend(params.into_iter().map(Value::from));
    out
}

type Names = HashMap<i64, (Option<i64>, String)>;

fn document_ref(conn: &Connection, visible: &Visible, content_id: i64, cache: &mut Names) -> Result<DocumentRef, String> {
    let (location_id, name) = document(conn, visible, content_id, cache)?;
    Ok(DocumentRef { content_id, location_id, name })
}

/// The path of each page of the layer (entities and matters).
pub(crate) fn page_paths(conn: &Connection) -> Result<HashMap<String, String>, String> {
    let mut statement = conn.prepare("SELECT entity_id, path FROM knowledge_page WHERE state = 'projected'").map_err(sql_err)?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// The decisions of a person about matters (keys `content:<id>`), with the
/// contents on each side.
fn matter_decisions(conn: &Connection) -> Result<Vec<(DecisionView, i64, Option<i64>)>, String> {
    let all = decisions(conn)?;
    let successors = changes::successors(conn)?;
    let content = |key: &str| -> Option<i64> { changes::translate(key, &successors).strip_prefix("content:")?.parse().ok() };
    Ok(all
        .into_iter()
        .filter_map(|d| {
            let a = content(&d.a)?;
            let b = d.b.as_deref().and_then(content);
            Some((d, a, b))
        })
        .collect())
}

fn decisions(conn: &Connection) -> Result<Vec<DecisionView>, String> {
    let mut statement = conn
        .prepare(
            "SELECT id, kind, a, b, note, shown_evidence, state, decided_at, undone_at, review, applied_as FROM knowledge_decision
             ORDER BY decided_at DESC, id DESC",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| {
            Ok(DecisionView {
                id: row.get(0)?,
                kind: row.get(1)?,
                a: row.get(2)?,
                b: row.get(3)?,
                note: row.get(4)?,
                shown_evidence: row.get(5)?,
                state: row.get(6)?,
                decided_at: row.get(7)?,
                undone_at: row.get(8)?,
                review: row.get(9)?,
                applied_as: row.get(10)?,
            })
        })
        .map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// The proposals of the model the person did not answer yet: a proposal
/// for a matter is gone when the person kept the document apart from one of
/// its documents (or put it in).
fn open_proposals(conn: &Connection) -> Result<BTreeMap<i64, Vec<(String, Option<String>)>>, String> {
    super::matters::proposals(conn)
}

impl DocBase {
    /// What the layer brings to the attention view, over what `visible`
    /// lets see.
    pub fn knowledge_attention(&self, visible: &Visible) -> Result<KnowledgeAttention, String> {
        let status = self.knowledge_status(visible)?;
        let conn = self.conn();
        let rules = &vocabulary().attention;
        let mut names: Names = HashMap::new();

        // Readings that failed, and readings that lost many quotes.
        let mut failed = Vec::new();
        let mut noisy = Vec::new();
        for standing in extract::standings(&conn, visible, None)? {
            let Some(reading) = standing.reading else { continue };
            if !reading.current {
                continue;
            }
            let row: Option<(Option<String>, i64, i64)> = conn
                .query_row(
                    "SELECT error, mentions, evidence FROM knowledge_extraction WHERE version_id = ?1",
                    [standing.content_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()
                .map_err(sql_err)?;
            let Some((error, mentions, evidence)) = row else { continue };
            if reading.state == "failed" {
                failed.push(FailedReading { document: document_ref(&conn, visible, standing.content_id, &mut names)?, error });
                continue;
            }
            let discarded: Vec<Discarded> = serde_json::from_str(&reading.discarded).unwrap_or_default();
            let lost = discarded.iter().filter(|d| rules.noisy_reasons.iter().any(|r| r == &d.reason)).count();
            let kept = (mentions + evidence).max(0) as usize;
            let said = kept + discarded.len();
            if lost >= rules.noisy_min && said > 0 && lost as f64 > rules.noisy_share * said as f64 {
                noisy.push(NoisyReading { document: document_ref(&conn, visible, standing.content_id, &mut names)?, kept, discarded: lost });
            }
        }

        // Decisions of a person that no longer find what they named.
        let mut decisions_to_review = Vec::new();
        {
            let successors = changes::successors(&conn)?;
            for decision in decisions(&conn)? {
                let Some(reason) = decision.review.clone().filter(|_| decision.state == "active") else { continue };
                let key = changes::translate(&decision.a, &successors);
                let content = key
                    .strip_prefix("cluster:")
                    .or_else(|| key.strip_prefix("evidence:"))
                    .and_then(|rest| rest.split(':').next())
                    .and_then(|c| c.parse::<i64>().ok());
                let document = match content {
                    Some(content) => {
                        let found = document_ref(&conn, visible, content, &mut names)?;
                        if found.location_id.is_none() {
                            // A document the caller cannot see: nor its decision.
                            continue;
                        }
                        Some(found)
                    }
                    None => None,
                };
                decisions_to_review.push(DecisionReview { decision, document, reason });
            }
        }

        // Claims in conflict, by entity and what they are about.
        let mut conflicts: BTreeMap<(String, String, Option<String>, Option<String>), ConflictGroup> = BTreeMap::new();
        {
            let (condition, params) = visible_content("e.version_id", visible);
            let mut statement = conn
                .prepare(&format!(
                    "SELECT DISTINCT c.id, c.subject, s.name, s.kind, c.predicate, c.context_entity, w.name, c.qualifier,
                            coalesce(o.name, c.object_value)
                     FROM knowledge_claim c JOIN knowledge_entity s ON s.id = c.subject
                     LEFT JOIN knowledge_entity o ON o.id = c.object_entity LEFT JOIN knowledge_entity w ON w.id = c.context_entity
                     JOIN knowledge_evidence e ON e.claim_id = c.id
                     WHERE c.state = 'conflict' AND s.state = 'active' AND {condition}
                     ORDER BY s.name, c.predicate, c.id"
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params_from_iter(params.iter()), |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, Option<String>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                        row.get::<_, Option<String>>(7)?,
                        row.get::<_, Option<String>>(8)?,
                    ))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (id, subject, name, kind, predicate, context, within, qualifier, value) = row.map_err(sql_err)?;
                let group = conflicts.entry((subject.clone(), predicate.clone(), context, qualifier)).or_insert_with(|| ConflictGroup {
                    entity_id: subject,
                    entity_name: name,
                    kind,
                    predicate,
                    within,
                    values: Vec::new(),
                    claims: Vec::new(),
                });
                group.claims.push(id);
                if let Some(value) = value {
                    if !group.values.contains(&value) {
                        group.values.push(value);
                    }
                }
            }
        }

        // Joins by a strong identifier of different names, recent.
        let mut identifier_joins = Vec::new();
        {
            let since = now_ms() - rules.recent_join_days * 24 * 60 * 60 * 1000;
            let (condition, params) = visible_content("m.version_id", visible);
            let mut statement = conn
                .prepare(&format!(
                    "SELECT e.id, e.name, e.kind, max(k.extracted_at) FROM knowledge_mention m
                     JOIN knowledge_entity e ON e.id = m.entity_id JOIN knowledge_extraction k ON k.version_id = m.version_id
                     WHERE m.resolved_by = 'identificador' AND e.state = 'active' AND {condition}
                     GROUP BY e.id HAVING max(k.extracted_at) >= ? ORDER BY max(k.extracted_at) DESC, e.name"
                ))
                .map_err(sql_err)?;
            let mut all: Vec<Value> = params.into_iter().map(Value::from).collect();
            all.push(since.into());
            let rows: Vec<(String, String, String, i64)> = statement
                .query_map(params_from_iter(all.iter()), |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                .map_err(sql_err)?
                .collect::<Result<_, _>>()
                .map_err(sql_err)?;
            drop(statement);
            let (condition, params) = visible_content("m.version_id", visible);
            for (id, name, kind, read_at) in rows {
                let mut statement = conn
                    .prepare(&format!(
                        "SELECT m.written, m.normalized, m.version_id FROM knowledge_mention m WHERE m.entity_id = ? AND {condition}
                         ORDER BY m.id"
                    ))
                    .map_err(sql_err)?;
                let mentions: Vec<(String, String, i64)> = statement
                    .query_map(params_from_iter(with_params(vec![id.clone().into()], params.clone()).iter()), |row| {
                        Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                    })
                    .map_err(sql_err)?
                    .collect::<Result<_, _>>()
                    .map_err(sql_err)?;
                let normalized: BTreeSet<&str> = mentions.iter().map(|(_, n, _)| n.as_str()).collect();
                if normalized.len() < 2 {
                    continue;
                }
                let mut written: Vec<String> = Vec::new();
                for (w, _, _) in &mentions {
                    if !written.contains(w) {
                        written.push(w.clone());
                    }
                }
                let documents: BTreeSet<i64> = mentions.iter().map(|(_, _, c)| *c).collect();
                let mut statement = conn
                    .prepare(
                        "SELECT DISTINCT i.kind, i.value FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id
                         WHERE m.entity_id = ?1 ORDER BY i.kind, i.value",
                    )
                    .map_err(sql_err)?;
                let identifiers: Vec<String> = statement
                    .query_map([&id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
                    .map_err(sql_err)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(sql_err)?
                    .into_iter()
                    .filter(|(kind, _)| vocabulary().is_strong(kind))
                    .map(|(_, value)| value)
                    .collect();
                identifier_joins.push(IdentifierJoin { entity_id: id, name, kind, names: written, identifiers, documents: documents.len(), read_at });
            }
        }

        // What the model proposed for loose documents.
        let mut proposed_matters = Vec::new();
        {
            let visible_contents = visible_contents(&conn, visible)?;
            let matter_names: HashMap<String, String> = {
                // A closed or archived matter leaves Atención (D-475).
                let mut statement = conn.prepare("SELECT id, name FROM knowledge_matter WHERE state = 'active' AND status = 'open'").map_err(sql_err)?;
                let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
                rows.collect::<Result<_, _>>().map_err(sql_err)?
            };
            for (content, list) in open_proposals(&conn)? {
                if !visible_contents.contains(&content) {
                    continue;
                }
                for (matter_id, reason) in list {
                    let Some(matter_name) = matter_names.get(&matter_id).cloned() else { continue };
                    proposed_matters.push(ProposedMatter { document: document_ref(&conn, visible, content, &mut names)?, matter_id, matter_name, reason });
                }
            }
        }

        // The matters the grouping proposed, for a person to confirm.
        let matters_to_confirm = {
            let visible_contents = visible_contents(&conn, visible)?;
            let mut statement = conn
                .prepare(
                    "SELECT m.id, m.name, d.version_id, d.reasons FROM knowledge_matter m JOIN knowledge_matter_document d ON d.matter_id = m.id
                     WHERE m.state = 'active' AND m.status = 'open' AND m.confirmation = 'proposed' AND d.state = 'firm' ORDER BY m.id, d.version_id",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?)))
                .map_err(sql_err)?;
            let mut by_id: BTreeMap<String, MatterToConfirm> = BTreeMap::new();
            for row in rows {
                let (id, name, content, reasons) = row.map_err(sql_err)?;
                if !visible_contents.contains(&content) {
                    continue;
                }
                let entry = by_id.entry(id.clone()).or_insert_with(|| MatterToConfirm { id, name, documents: 0, reasons: BTreeMap::new() });
                entry.documents += 1;
                let kinds: BTreeSet<String> = serde_json::from_str::<Vec<Reason>>(&reasons).unwrap_or_default().into_iter().map(|r| r.kind).collect();
                for kind in kinds {
                    *entry.reasons.entry(kind).or_default() += 1;
                }
            }
            let mut list: Vec<MatterToConfirm> = by_id.into_values().filter(|m| m.documents >= 2).collect();
            list.sort_by(|a, b| b.documents.cmp(&a.documents).then_with(|| a.name.cmp(&b.name)));
            list
        };

        // Only a page with claims waits for a synthesis: one whose entity
        // the documents say nothing more of (or a matter's) never gets one.
        let (synthesis_pending, discarded_sentences): (i64, i64) = conn
            .query_row(
                "SELECT coalesce(sum(CASE WHEN p.synthesis IS NULL AND EXISTS (
                            SELECT 1 FROM knowledge_claim c WHERE c.subject = p.entity_id AND c.state IN ('supported', 'conflict')
                        ) THEN 1 ELSE 0 END), 0), coalesce(sum(p.discarded), 0)
                 FROM knowledge_page p WHERE p.state = 'projected'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sql_err)?;

        let kept_pages = {
            let mut statement = conn
                .prepare(
                    "SELECT p.entity_id, p.path, e.name, e.merged_into FROM knowledge_page p LEFT JOIN knowledge_entity e ON e.id = p.entity_id
                     WHERE p.state = 'kept' ORDER BY p.path",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok(KeptPage { entity_id: row.get(0)?, path: row.get(1)?, name: row.get(2)?, merged_into: row.get(3)? }))
                .map_err(sql_err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?
        };

        // Entities that left with the documents left out.
        let mut left_with_documents = Vec::new();
        {
            let left = changes::left_out(&conn)?;
            if !left.is_empty() {
                let list = left.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
                let mut statement = conn
                    .prepare(&format!(
                        "SELECT e.id, e.name, e.kind, g.version_id FROM knowledge_gone_mention g JOIN knowledge_entity e ON e.id = g.entity_id
                         WHERE e.state = 'retired' AND g.version_id IN ({list}) ORDER BY e.name, g.version_id"
                    ))
                    .map_err(sql_err)?;
                let rows: Vec<(String, String, String, i64)> = statement
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                    .map_err(sql_err)?
                    .collect::<Result<_, _>>()
                    .map_err(sql_err)?;
                drop(statement);
                let mut by_entity: BTreeMap<String, LeftWithDocuments> = BTreeMap::new();
                for (id, name, kind, content) in rows {
                    let document = document_ref(&conn, &Visible::All, content, &mut names)?;
                    let entry = by_entity.entry(id.clone()).or_insert_with(|| LeftWithDocuments { entity_id: id, name, kind, documents: Vec::new() });
                    if !entry.documents.iter().any(|d| d.content_id == content) {
                        entry.documents.push(document);
                    }
                }
                left_with_documents = by_entity.into_values().collect();
            }
        }

        let matters_to_join = super::matter_records::open_questions(&conn, &visible_contents(&conn, visible)?)?;

        Ok(KnowledgeAttention {
            status,
            failed,
            noisy,
            conflicts: conflicts.into_values().filter(|g| g.claims.len() > 1).collect(),
            identifier_joins,
            proposed_matters,
            matters_to_confirm,
            synthesis_pending: synthesis_pending as usize,
            discarded_sentences: discarded_sentences as usize,
            kept_pages,
            left_with_documents,
            decisions_to_review,
            matters_to_join,
        })
    }

    /// The active matters with documents `visible` lets see, the largest
    /// first.
    pub fn knowledge_matters(&self, visible: &Visible) -> Result<Vec<MatterSummary>, String> {
        let conn = self.conn();
        let (condition, params) = visible_content("d.version_id", visible);
        let mut statement = conn
            .prepare(&format!(
                "SELECT m.id, m.name, count(d.version_id), m.status, m.origin, m.confirmation FROM knowledge_matter m
                 LEFT JOIN knowledge_matter_document d ON d.matter_id = m.id
                 WHERE m.state = 'active' AND ((d.version_id IS NULL AND m.origin = 'person') OR {condition})
                 GROUP BY m.id ORDER BY count(d.version_id) DESC, m.name"
            ))
            .map_err(sql_err)?;
        let rows: Vec<(String, String, i64, String, String, String)> = statement
            .query_map(params_from_iter(params.iter()), |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)))
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        drop(statement);
        let paths = page_paths(&conn)?;
        let mut proposed: HashMap<String, usize> = HashMap::new();
        let visible_contents = visible_contents(&conn, visible)?;
        for (content, list) in open_proposals(&conn)? {
            if visible_contents.contains(&content) {
                for (matter, _) in list {
                    *proposed.entry(matter).or_default() += 1;
                }
            }
        }
        Ok(rows
            .into_iter()
            .map(|(id, name, documents, status, origin, confirmation)| MatterSummary {
                page_path: paths.get(&id).cloned(),
                proposed: proposed.get(&id).copied().unwrap_or(0),
                id,
                name,
                status,
                origin,
                confirmation,
                documents: documents as usize,
            })
            .collect())
    }

    /// One matter with its documents (and the ones the model proposed),
    /// why each is in it, its parties and the decisions about it.
    pub fn knowledge_matter(&self, visible: &Visible, id: &str) -> Result<Option<MatterDetail>, String> {
        let conn = self.conn();
        let found: Option<(String, String, String, String, String)> = conn
            .query_row("SELECT name, state, status, origin, confirmation FROM knowledge_matter WHERE id = ?1", [id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .optional()
            .map_err(sql_err)?;
        let Some((name, state, status, origin, confirmation)) = found else { return Ok(None) };
        let mut names: Names = HashMap::new();
        let (condition, params) = visible_content("d.version_id", visible);
        let mut statement = conn
            .prepare(&format!(
                "SELECT d.version_id, d.state, d.reasons FROM knowledge_matter_document d WHERE d.matter_id = ? AND {condition}
                 ORDER BY d.version_id"
            ))
            .map_err(sql_err)?;
        let rows: Vec<(i64, String, String)> = statement
            .query_map(params_from_iter(with_params(vec![id.to_string().into()], params).iter()), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        drop(statement);
        if rows.is_empty() && state == "active" && origin != "person" {
            return Ok(None);
        }
        let entity_names: HashMap<String, String> = {
            let mut statement = conn.prepare("SELECT id, name FROM knowledge_entity").map_err(sql_err)?;
            let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let visible_contents = visible_contents(&conn, visible)?;
        let mut documents = Vec::new();
        for (content, document_state, reasons) in &rows {
            let parsed: Vec<Reason> = serde_json::from_str(reasons).unwrap_or_default();
            let mut shown = Vec::new();
            for reason in parsed {
                let with = if visible_contents.contains(&reason.with) { Some(document_ref(&conn, visible, reason.with, &mut names)?) } else { None };
                // A shared reference: its normalized form (D-377); an entity
                // id from before it, until the matters are grouped again.
                let value_name = if reason.kind == "asunto" {
                    match entity_names.get(&reason.value) {
                        Some(name) => Some(name.clone()),
                        None => super::references::written_form(&conn, vocabulary(), &reason.value)?,
                    }
                } else {
                    None
                };
                shown.push(MatterReason { kind: reason.kind, value: reason.value, value_name, with });
            }
            documents.push(MatterDocument {
                document: document_ref(&conn, visible, *content, &mut names)?,
                state: document_state.clone(),
                reasons: shown,
                proposal_reason: None,
            });
        }
        for (content, list) in open_proposals(&conn)? {
            if !visible_contents.contains(&content) {
                continue;
            }
            for (matter, reason) in list {
                if matter == id {
                    documents.push(MatterDocument {
                        document: document_ref(&conn, visible, content, &mut names)?,
                        state: "proposed".into(),
                        reasons: Vec::new(),
                        proposal_reason: reason,
                    });
                }
            }
        }
        // Its parties: who its firm documents name, but the matters.
        let firm: Vec<i64> = rows.iter().map(|(c, _, _)| *c).collect();
        let mut parties: BTreeMap<String, (MatterParty, BTreeSet<i64>)> = BTreeMap::new();
        if !firm.is_empty() {
            let list = firm.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
            let mut statement = conn
                .prepare(&format!(
                    "SELECT m.version_id, m.role, e.id, e.kind, e.name FROM knowledge_mention m JOIN knowledge_entity e ON e.id = m.entity_id
                     WHERE m.version_id IN ({list}) AND e.state = 'active' AND e.kind != ?1 ORDER BY e.name, m.id"
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map([&vocabulary().matters.kind], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (content, role, entity, kind, entity_name) = row.map_err(sql_err)?;
                let entry = parties.entry(entity.clone()).or_insert_with(|| {
                    (MatterParty { entity_id: entity, kind, name: entity_name, roles: Vec::new(), documents: 0 }, BTreeSet::new())
                });
                if !entry.0.roles.contains(&role) {
                    entry.0.roles.push(role);
                }
                entry.1.insert(content);
            }
        }
        let fallback = vocabulary().fallback_role().to_string();
        let mut parties: Vec<MatterParty> = parties
            .into_values()
            .map(|(mut party, docs)| {
                party.documents = docs.len();
                party
            })
            // Who is only named in passing in one document is not a party.
            .filter(|p| p.documents > 1 || p.roles.iter().any(|r| r != &fallback))
            .collect();
        parties.sort_by(|a, b| b.documents.cmp(&a.documents).then_with(|| a.name.cmp(&b.name)));
        let contents: BTreeSet<i64> = documents.iter().map(|d| d.document.content_id).collect();
        let decisions = matter_decisions(&conn)?
            .into_iter()
            .filter(|(_, a, b)| contents.contains(a) || b.is_some_and(|b| contents.contains(&b)))
            .map(|(d, _, _)| d)
            .collect();
        let page_path = page_paths(&conn)?.remove(id);
        let records = super::matter_records::records_of(&conn, id)?;
        Ok(Some(MatterDetail { id: id.to_string(), name, state, status, origin, confirmation, page_path, documents, parties, decisions, records }))
    }

    /// The entities a document names, with their roles, and its matter.
    pub fn knowledge_document_entities(&self, visible: &Visible, location_id: i64) -> Result<DocumentEntities, String> {
        let known = self.knowledge_of(visible, location_id)?;
        let mut out = DocumentEntities {
            content_id: known.content_id,
            state: known.state.clone(),
            older: known.older,
            reread: known.reread,
            error: known.error.clone(),
            entities: Vec::new(),
            references: Vec::new(),
            matters: Vec::new(),
            discarded: known.discarded.len(),
        };
        let Some(content) = known.content_id else { return Ok(out) };
        let conn = self.conn();
        let paths = page_paths(&conn)?;
        let order = |kind: &str| vocabulary().entity_types.iter().position(|t| t.id == kind).unwrap_or(usize::MAX);
        let mut by_entity: BTreeMap<String, DocumentEntity> = BTreeMap::new();
        let entity_rows: HashMap<String, (String, String, String)> = {
            let mut statement = conn.prepare("SELECT id, kind, name, state FROM knowledge_entity").map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, (row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let vocabulary = vocabulary();
        for mention in &known.mentions {
            if vocabulary.is_reference(&mention.kind) {
                continue;
            }
            let active = mention.entity_id.as_ref().and_then(|id| entity_rows.get(id).map(|row| (id, row))).filter(|(_, row)| row.2 == "active");
            let key = match active {
                Some((id, _)) => id.clone(),
                None => format!("~{}:{}", mention.kind, mention.written),
            };
            let entry = by_entity.entry(key).or_insert_with(|| match active {
                Some((id, (kind, name, _))) => DocumentEntity {
                    entity_id: Some(id.clone()),
                    kind: kind.clone(),
                    name: name.clone(),
                    written: Vec::new(),
                    roles: Vec::new(),
                    page_path: paths.get(id).cloned(),
                },
                None => DocumentEntity {
                    entity_id: None,
                    kind: mention.kind.clone(),
                    name: mention.written.clone(),
                    written: Vec::new(),
                    roles: Vec::new(),
                    page_path: None,
                },
            });
            if !entry.written.contains(&mention.written) {
                entry.written.push(mention.written.clone());
            }
            if !entry.roles.contains(&mention.role) {
                entry.roles.push(mention.role.clone());
            }
        }
        let mut entities: Vec<DocumentEntity> = by_entity.into_values().collect();
        entities.sort_by(|a, b| order(&a.kind).cmp(&order(&b.kind)).then_with(|| a.name.cmp(&b.name)));
        out.entities = entities;
        let identifiers: BTreeMap<i64, Vec<super::IdentifierView>> =
            known.mentions.iter().map(|m| (m.id, m.identifiers.clone())).collect();
        out.references = super::references::shown_references(&conn, vocabulary, content, &identifiers)?;
        out.matters = document_matters(&conn, &[content], &paths)?;
        Ok(out)
    }

    /// Takes a document out of a matter: kept apart from each of its other
    /// documents (a person's separation is never crossed), in one go and
    /// with one resolution. Also how a proposal of the model is turned
    /// down. Returns the ids of the decisions.
    pub fn knowledge_leave_matter(&self, document: &str, matter_id: &str, note: Option<&str>) -> Result<Vec<i64>, String> {
        let content = self.content_of_reference(document)?;
        let others: Vec<i64> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare("SELECT version_id FROM knowledge_matter_document WHERE matter_id = ?1 AND version_id != ?2 ORDER BY version_id")
                .map_err(sql_err)?;
            let rows = statement.query_map(rusqlite::params![matter_id, content], |row| row.get(0)).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        if others.is_empty() {
            return Err("Ese asunto no tiene otros documentos".to_string());
        }
        let decisions: Vec<IdentityDecision> = others
            .iter()
            .map(|other| IdentityDecision::OtherMatter { a: content_key(content), b: content_key(*other) })
            .collect();
        self.knowledge_decide_many(&decisions, note)
    }

    /// What a person decides about a matter the grouping proposed (D-381):
    /// `confirm` it, `discard` it (it goes away and stays away while the
    /// grouping finds the same documents), or `reopen` it as a proposal.
    /// Kept on the matter: a grouping never overwrites it. The matters are
    /// grouped again at once.
    pub fn knowledge_decide_matter(&self, matter_id: &str, decision: &str) -> Result<(), String> {
        let conn = self.conn();
        let state: Option<String> = conn
            .query_row("SELECT state FROM knowledge_matter WHERE id = ?1", [matter_id], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        let Some(state) = state else { return Err("Ese asunto ya no existe".to_string()) };
        let now = now_ms();
        match decision {
            "confirm" => {
                if state != "active" {
                    return Err("Ese asunto ya no está activo".to_string());
                }
                conn.execute(
                    "UPDATE knowledge_matter SET confirmation = 'confirmed', decided_at = ?2, decided_documents = NULL, updated_at = ?2 WHERE id = ?1",
                    rusqlite::params![matter_id, now],
                )
                .map_err(sql_err)?;
            }
            "discard" => {
                let documents: Vec<i64> = {
                    let mut statement =
                        conn.prepare("SELECT version_id FROM knowledge_matter_document WHERE matter_id = ?1 ORDER BY version_id").map_err(sql_err)?;
                    let rows = statement.query_map([matter_id], |row| row.get(0)).map_err(sql_err)?;
                    rows.collect::<Result<_, _>>().map_err(sql_err)?
                };
                let list = serde_json::to_string(&documents).map_err(|e| e.to_string())?;
                conn.execute(
                    "UPDATE knowledge_matter SET confirmation = 'discarded', state = 'retired', decided_at = ?2, decided_documents = ?3, updated_at = ?2
                     WHERE id = ?1",
                    rusqlite::params![matter_id, now, list],
                )
                .map_err(sql_err)?;
                conn.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1", [matter_id]).map_err(sql_err)?;
            }
            "reopen" => {
                conn.execute(
                    "UPDATE knowledge_matter SET confirmation = 'proposed', decided_at = NULL, decided_documents = NULL, updated_at = ?2 WHERE id = ?1",
                    rusqlite::params![matter_id, now],
                )
                .map_err(sql_err)?;
            }
            other => return Err(format!("'{other}' no es una decisión sobre un asunto")),
        }
        super::mark_dirty(&conn, "project", "matters")?;
        let decided = super::resolve::load_decided(&conn)?;
        super::matters::group(&conn, vocabulary(), &decided.joins, &decided.separations)?;
        Ok(())
    }

    /// Puts a document in a matter: of the same matter as its first
    /// document. How a proposal of the model is confirmed.
    pub fn knowledge_join_matter(&self, document: &str, matter_id: &str, note: Option<&str>) -> Result<i64, String> {
        let content = self.content_of_reference(document)?;
        let first: Option<i64> = self
            .conn()
            .query_row(
                "SELECT version_id FROM knowledge_matter_document WHERE matter_id = ?1 AND version_id != ?2 ORDER BY version_id LIMIT 1",
                rusqlite::params![matter_id, content],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        let first = first.ok_or_else(|| "Ese asunto ya no tiene documentos".to_string())?;
        let ids = self.knowledge_decide_many(&[IdentityDecision::SameMatter { a: content_key(content), b: content_key(first) }], note)?;
        Ok(ids[0])
    }

    pub(crate) fn content_of_reference(&self, reference: &str) -> Result<i64, String> {
        if let Some(id) = reference.strip_prefix("content:").and_then(|s| s.parse().ok()) {
            return Ok(id);
        }
        if let Some(id) = reference.strip_prefix("location:").and_then(|s| s.parse::<i64>().ok()) {
            let content: Option<i64> = self
                .conn()
                .query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.id = ?1", [id], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            return content.ok_or_else(|| "Ese documento ya no existe".to_string());
        }
        Err(format!("'{reference}' no es un documento"))
    }
}

/// The contents `visible` lets see.
fn visible_contents(conn: &Connection, visible: &Visible) -> Result<BTreeSet<i64>, String> {
    let (condition, params) = visible_content("c.id", visible);
    let mut statement = conn.prepare(&format!("SELECT c.id FROM version c WHERE {condition}")).map_err(sql_err)?;
    let rows = statement.query_map(params_from_iter(params.iter()), |row| row.get(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// The matters of these documents, firm and proposed.
pub(crate) fn document_matters(conn: &Connection, contents: &[i64], paths: &HashMap<String, String>) -> Result<Vec<DocumentMatter>, String> {
    let mut out = Vec::new();
    if contents.is_empty() {
        return Ok(out);
    }
    let list = contents.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
    let mut statement = conn
        .prepare(&format!(
            "SELECT d.version_id, m.id, m.name FROM knowledge_matter_document d JOIN knowledge_matter m ON m.id = d.matter_id
             WHERE m.state = 'active' AND d.version_id IN ({list}) ORDER BY d.version_id, m.name"
        ))
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
        .map_err(sql_err)?;
    for row in rows {
        let (content_id, matter_id, name) = row.map_err(sql_err)?;
        out.push(DocumentMatter { content_id, page_path: paths.get(&matter_id).cloned(), matter_id, name, proposed: false });
    }
    let wanted: BTreeSet<i64> = contents.iter().copied().collect();
    let names: HashMap<String, String> = {
        let mut statement = conn.prepare("SELECT id, name FROM knowledge_matter WHERE state = 'active'").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    for (content_id, list) in open_proposals(conn)? {
        if !wanted.contains(&content_id) {
            continue;
        }
        for (matter_id, _) in list {
            if let Some(name) = names.get(&matter_id) {
                out.push(DocumentMatter { content_id, page_path: paths.get(&matter_id).cloned(), name: name.clone(), matter_id, proposed: true });
            }
        }
    }
    Ok(out)
}

/// The decisions of a person about an entity: those on its groups of
/// mentions, its claims and its evidence, the latest first.
pub(crate) fn entity_decisions(conn: &Connection, entity: &str) -> Result<Vec<DecisionView>, String> {
    let mut statement = conn.prepare("SELECT version_id, kind, normalized FROM knowledge_mention WHERE entity_id = ?1").map_err(sql_err)?;
    let rows: Vec<(i64, String, String)> = statement
        .query_map([entity], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    let clusters: BTreeSet<String> = rows.iter().map(|(c, k, n)| cluster_key(*c, k, n)).collect();
    let evidence: Vec<String> = rows.iter().map(|(c, _, n)| format!("evidence:{c}:{n}:")).collect();
    let claim = format!("claim:{entity}:");
    let successors = changes::successors(conn)?;
    let about = |key: &str| {
        let key = changes::translate(key, &successors);
        clusters.contains(&key) || key.starts_with(&claim) || evidence.iter().any(|e| key.starts_with(e.as_str()))
    };
    Ok(decisions(conn)?.into_iter().filter(|d| about(&d.a) || d.b.as_deref().is_some_and(about)).collect())
}

/// The claims of other entities whose object is this one.
pub(crate) fn incoming_claims(conn: &Connection, visible: &Visible, entity: &str) -> Result<Vec<IncomingClaim>, String> {
    let (condition, params) = visible_content("e.version_id", visible);
    let mut statement = conn
        .prepare(&format!(
            "SELECT c.id, s.id, s.name, s.kind, c.predicate, c.state, count(DISTINCT e.version_id) FROM knowledge_claim c
             JOIN knowledge_entity s ON s.id = c.subject JOIN knowledge_evidence e ON e.claim_id = c.id
             WHERE c.object_entity = ? AND c.state != 'retired' AND s.state = 'active' AND {condition}
             GROUP BY c.id ORDER BY c.predicate, s.name"
        ))
        .map_err(sql_err)?;
    let rows = statement
        .query_map(params_from_iter(with_params(vec![entity.to_string().into()], params).iter()), |row| {
            Ok(IncomingClaim {
                claim_id: row.get(0)?,
                subject_id: row.get(1)?,
                subject_name: row.get(2)?,
                subject_kind: row.get(3)?,
                predicate: row.get(4)?,
                state: row.get(5)?,
                documents: row.get::<_, i64>(6)? as usize,
            })
        })
        .map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

