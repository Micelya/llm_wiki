//! The record of entities as the screens (and the projection of pages)
//! read it: the list of entities, one entity with its aliases,
//! identifiers, mentions, claims and doubts, and the doubts left for a
//! person. Only what `visible` lets see: an entity is seen through the
//! documents that name it.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};
use serde::Serialize;

use super::{quote, visible_content};
use crate::docbase::{sql_err, DocBase, Visible};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitySummary {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub mentions: usize,
    pub documents: usize,
    /// Other ways it is written, the most used first.
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AliasView {
    pub written: String,
    pub mentions: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityIdentifier {
    pub kind: String,
    pub value: String,
    pub documents: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityMention {
    pub mention_id: i64,
    pub content_id: i64,
    /// A location of its document the caller can see.
    pub location_id: Option<i64>,
    pub document: String,
    pub role: String,
    pub written: String,
    pub quote: String,
    pub page: Option<i64>,
    /// What joined it to the entity: "persona", "identificador",
    /// "nombre", "modelo" or "sola".
    pub resolved_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimEvidence {
    pub evidence_id: i64,
    pub content_id: i64,
    pub document: String,
    pub quote: String,
    pub page: Option<i64>,
    pub fact_time: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaimView {
    pub id: i64,
    pub predicate: String,
    pub object_entity: Option<String>,
    pub object_name: Option<String>,
    pub object_value: Option<String>,
    /// "supported", "conflict" or "retired".
    pub state: String,
    pub valid_from: Option<String>,
    pub evidence: Vec<ClaimEvidence>,
    /// How sure the record is of it (D-320): "quoted" (its quote is in the
    /// document, nobody confirmed it), "confirmed", "doubtful" or
    /// "corrected"; and why ("conflict", "identity", "attribution",
    /// "states", "by-person").
    pub certainty: String,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoubtView {
    /// Keys of the two groups of mentions (what a decision refers to).
    pub a: String,
    pub b: String,
    pub kind: String,
    pub entity_a: Option<String>,
    pub entity_b: Option<String>,
    pub name_a: Option<String>,
    pub name_b: Option<String>,
    /// Why it is in doubt ("identificador-parecido", "modelo-dudosa"…).
    pub reason: String,
    pub detail: Option<String>,
    pub evidence_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityDetail {
    pub id: String,
    pub kind: String,
    pub name: String,
    /// "active", "merged" or "retired".
    pub state: String,
    pub merged_into: Option<String>,
    pub aliases: Vec<AliasView>,
    pub identifiers: Vec<EntityIdentifier>,
    pub mentions: Vec<EntityMention>,
    pub claims: Vec<ClaimView>,
    pub doubts: Vec<DoubtView>,
    /// Its page in the archive's wiki, when it has one.
    pub page_path: Option<String>,
    /// The matter of each of its documents (firm or proposed by the model).
    pub matters: Vec<super::screens::DocumentMatter>,
    /// What a person decided about it, active or undone, the latest first.
    pub decisions: Vec<super::decide::DecisionView>,
    /// What other entities' claims say of it ("represents it").
    pub incoming: Vec<super::screens::IncomingClaim>,
}

fn values(first: Vec<Value>, params: Vec<String>) -> Vec<Value> {
    let mut out = first;
    out.extend(params.into_iter().map(Value::from));
    out
}

/// The name of a document and a location of it the caller can see.
pub(crate) fn document(conn: &Connection, visible: &Visible, content_id: i64, cache: &mut HashMap<i64, (Option<i64>, String)>) -> Result<(Option<i64>, String), String> {
    if let Some(found) = cache.get(&content_id) {
        return Ok(found.clone());
    }
    let (condition, params) = visible.condition("l.origin_id");
    let found: Option<(i64, String)> = conn
        .query_row(
            &format!(
                "SELECT l.id, l.name FROM location l JOIN file f ON f.hash = l.file_hash WHERE f.content_id = ? AND {condition}
                 ORDER BY l.rel_path COLLATE NOCASE LIMIT 1"
            ),
            params_from_iter(values(vec![content_id.into()], params).iter()),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let out = match found {
        Some((id, name)) => (Some(id), name),
        None => (None, String::new()),
    };
    cache.insert(content_id, out.clone());
    Ok(out)
}

fn doubts(conn: &Connection, visible: &Visible, entity: Option<&str>) -> Result<Vec<DoubtView>, String> {
    let (ca, pa) = visible_content("ma.content_id", visible);
    let (cb, pb) = visible_content("mb.content_id", visible);
    let mut statement = conn
        .prepare(&format!(
            "SELECT d.a, d.b, d.kind, d.entity_a, d.entity_b, ea.name, eb.name, d.reason, d.detail, d.evidence_hash
             FROM knowledge_doubt d
             LEFT JOIN knowledge_entity ea ON ea.id = d.entity_a LEFT JOIN knowledge_entity eb ON eb.id = d.entity_b
             WHERE (? IS NULL OR d.entity_a = ? OR d.entity_b = ?)
               AND EXISTS (SELECT 1 FROM knowledge_mention ma WHERE ma.entity_id = d.entity_a AND {ca})
               AND EXISTS (SELECT 1 FROM knowledge_mention mb WHERE mb.entity_id = d.entity_b AND {cb})
             ORDER BY d.kind, ea.name, eb.name"
        ))
        .map_err(sql_err)?;
    let mut all: Vec<Value> = vec![entity.map(str::to_string).into(), entity.map(str::to_string).into(), entity.map(str::to_string).into()];
    all.extend(pa.into_iter().map(Value::from));
    all.extend(pb.into_iter().map(Value::from));
    let rows = statement
        .query_map(params_from_iter(all.iter()), |row| {
            Ok(DoubtView {
                a: row.get(0)?,
                b: row.get(1)?,
                kind: row.get(2)?,
                entity_a: row.get(3)?,
                entity_b: row.get(4)?,
                name_a: row.get(5)?,
                name_b: row.get(6)?,
                reason: row.get(7)?,
                detail: row.get(8)?,
                evidence_hash: row.get(9)?,
            })
        })
        .map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

impl DocBase {
    /// The active entities seen through `visible`, of one kind or all,
    /// whose names contain the words of `query`; the most named first.
    pub fn knowledge_entities(&self, visible: &Visible, kind: Option<&str>, query: Option<&str>) -> Result<Vec<EntitySummary>, String> {
        let conn = self.conn();
        let (condition, params) = visible_content("m.content_id", visible);
        let mut statement = conn
            .prepare(&format!(
                "SELECT e.id, e.kind, e.name, count(m.id), count(DISTINCT m.content_id) FROM knowledge_entity e
                 JOIN knowledge_mention m ON m.entity_id = e.id
                 WHERE e.state = 'active' AND (? IS NULL OR e.kind = ?) AND e.kind != '{reference}' AND {condition}
                 GROUP BY e.id ORDER BY count(DISTINCT m.content_id) DESC, e.name",
                // References are not entities (D-377), even in a base not
                // resolved again yet.
                reference = super::vocabulary::vocabulary().matters.kind.replace('\'', "")
            ))
            .map_err(sql_err)?;
        let first: Vec<Value> = vec![kind.map(str::to_string).into(), kind.map(str::to_string).into()];
        let rows: Vec<(String, String, String, i64, i64)> = statement
            .query_map(params_from_iter(values(first, params).iter()), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        let mut aliases: HashMap<String, Vec<(String, String, i64)>> = HashMap::new();
        {
            let mut statement = conn.prepare("SELECT entity_id, normalized, written, mentions FROM knowledge_alias ORDER BY mentions DESC, written").map_err(sql_err)?;
            let found = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?)))
                .map_err(sql_err)?;
            for row in found {
                let (entity, normalized, written, mentions) = row.map_err(sql_err)?;
                aliases.entry(entity).or_default().push((normalized, written, mentions));
            }
        }
        let wanted: Vec<String> = query.map(quote::words).unwrap_or_default();
        let mut out = Vec::new();
        for (id, kind, name, mentions, documents) in rows {
            let list = aliases.remove(&id).unwrap_or_default();
            if !wanted.is_empty() {
                let found = list.iter().any(|(normalized, _, _)| {
                    let words: BTreeSet<&str> = normalized.split(' ').collect();
                    wanted.iter().all(|w| words.iter().any(|x| x.starts_with(w.as_str())))
                });
                if !found {
                    continue;
                }
            }
            let others: Vec<String> = list.into_iter().map(|(_, written, _)| written).filter(|w| w != &name).take(5).collect();
            out.push(EntitySummary { id, kind, name, mentions: mentions as usize, documents: documents as usize, aliases: others });
        }
        Ok(out)
    }

    /// One entity as `visible` lets see it, with everything the record
    /// knows of it. A merged entity says which one it went into.
    pub fn knowledge_entity(&self, visible: &Visible, id: &str) -> Result<Option<EntityDetail>, String> {
        let conn = self.conn();
        let found: Option<(String, String, String, Option<String>)> = conn
            .query_row("SELECT kind, name, state, merged_into FROM knowledge_entity WHERE id = ?1", [id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .optional()
            .map_err(sql_err)?;
        let Some((kind, name, state, merged_into)) = found else { return Ok(None) };
        let mut cache = HashMap::new();
        let (condition, params) = visible_content("m.content_id", visible);
        let mut statement = conn
            .prepare(&format!(
                "SELECT m.id, m.content_id, m.role, m.written, m.quote, m.page, m.resolved_by FROM knowledge_mention m
                 WHERE m.entity_id = ? AND {condition} ORDER BY m.content_id, m.id"
            ))
            .map_err(sql_err)?;
        let rows: Vec<(i64, i64, String, String, String, Option<i64>, Option<String>)> = statement
            .query_map(params_from_iter(values(vec![id.to_string().into()], params.clone()).iter()), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?))
            })
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        drop(statement);
        if rows.is_empty() && state == "active" {
            return Ok(None);
        }
        let mut mentions = Vec::new();
        let mut written: BTreeMap<String, usize> = BTreeMap::new();
        for (mention_id, content_id, role, text, quote, page, resolved_by) in rows {
            let (location_id, document) = document(&conn, visible, content_id, &mut cache)?;
            *written.entry(text.clone()).or_default() += 1;
            mentions.push(EntityMention { mention_id, content_id, location_id, document, role, written: text, quote, page, resolved_by });
        }
        let mut aliases: Vec<AliasView> = written.into_iter().map(|(written, mentions)| AliasView { written, mentions }).collect();
        aliases.sort_by(|a, b| b.mentions.cmp(&a.mentions).then_with(|| a.written.cmp(&b.written)));
        // Identifiers, by kind and value, with how many documents give them.
        let mut identifiers: BTreeMap<(String, String), BTreeSet<i64>> = BTreeMap::new();
        {
            let mut statement = conn
                .prepare(&format!(
                    "SELECT i.kind, i.value, i.normalized, m.content_id FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id
                     WHERE m.entity_id = ? AND {condition}"
                ))
                .map_err(sql_err)?;
            let found = statement
                .query_map(params_from_iter(values(vec![id.to_string().into()], params.clone()).iter()), |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?))
                })
                .map_err(sql_err)?;
            let mut shown: HashMap<(String, String), String> = HashMap::new();
            for row in found {
                let (kind, value, normalized, content) = row.map_err(sql_err)?;
                shown.entry((kind.clone(), normalized.clone())).or_insert(value);
                identifiers.entry((kind, normalized)).or_default().insert(content);
            }
            let identifiers: Vec<EntityIdentifier> = identifiers
                .into_iter()
                .map(|((kind, normalized), docs)| EntityIdentifier {
                    value: shown.remove(&(kind.clone(), normalized)).unwrap_or_default(),
                    kind,
                    documents: docs.len(),
                })
                .collect();
            let claims = claims_of(&conn, visible, id, &mut cache)?;
            let doubts = doubts(&conn, visible, Some(id))?;
            let paths = super::screens::page_paths(&conn)?;
            let contents: Vec<i64> = mentions.iter().map(|m| m.content_id).collect::<BTreeSet<_>>().into_iter().collect();
            let matters = super::screens::document_matters(&conn, &contents, &paths)?;
            let decisions = super::screens::entity_decisions(&conn, id)?;
            let incoming = super::screens::incoming_claims(&conn, visible, id)?;
            return Ok(Some(EntityDetail {
                id: id.to_string(),
                kind,
                name,
                state,
                merged_into,
                aliases,
                identifiers,
                mentions,
                claims,
                doubts,
                page_path: paths.get(id).cloned(),
                matters,
                decisions,
                incoming,
            }));
        }
    }

    /// The pairs left for a person to decide.
    pub fn knowledge_doubts(&self, visible: &Visible) -> Result<Vec<DoubtView>, String> {
        doubts(&self.conn(), visible, None)
    }
}

/// What the documents say of an entity: its claims as subject, each with
/// the evidence `visible` lets see.
fn claims_of(conn: &Connection, visible: &Visible, id: &str, cache: &mut HashMap<i64, (Option<i64>, String)>) -> Result<Vec<ClaimView>, String> {
    let mut statement = conn
        .prepare(
            "SELECT c.id, c.predicate, c.object_entity, o.name, coalesce(c.object_value, c.amount), c.state, c.valid_from, c.certainty
             FROM knowledge_claim c
             LEFT JOIN knowledge_entity o ON o.id = c.object_entity WHERE c.subject = ?1 ORDER BY c.predicate, c.id",
        )
        .map_err(sql_err)?;
    let claims: Vec<ClaimView> = statement
        .query_map([id], |row| {
            Ok(ClaimView {
                id: row.get(0)?,
                predicate: row.get(1)?,
                object_entity: row.get(2)?,
                object_name: row.get(3)?,
                object_value: row.get(4)?,
                state: row.get(5)?,
                valid_from: row.get(6)?,
                evidence: Vec::new(),
                certainty: String::new(),
                reasons: Vec::new(),
            })
            .map(|mut claim| {
                let value: serde_json::Value =
                    row.get::<_, Option<String>>(7).ok().flatten().and_then(|j| serde_json::from_str(&j).ok()).unwrap_or_default();
                claim.certainty = value.get("level").and_then(|l| l.as_str()).unwrap_or("quoted").to_string();
                claim.reasons = value
                    .get("reasons")
                    .and_then(|r| r.as_array())
                    .map(|list| list.iter().filter_map(|r| r.as_str().map(str::to_string)).collect())
                    .unwrap_or_default();
                claim
            })
        })
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    let (condition, params) = visible_content("e.content_id", visible);
    let mut statement = conn
        .prepare(&format!(
            "SELECT e.id, e.content_id, e.quote, e.page, e.fact_time FROM knowledge_evidence e WHERE e.claim_id = ? AND {condition}
             ORDER BY e.content_id, e.id"
        ))
        .map_err(sql_err)?;
    let mut out = Vec::new();
    for mut claim in claims {
        let rows: Vec<(i64, i64, String, Option<i64>, Option<String>)> = statement
            .query_map(params_from_iter(values(vec![claim.id.into()], params.clone()).iter()), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
            })
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        for (evidence_id, content_id, quote, page, fact_time) in rows {
            let (_, document) = document(conn, visible, content_id, cache)?;
            claim.evidence.push(ClaimEvidence { evidence_id, content_id, document, quote, page, fact_time });
        }
        if !claim.evidence.is_empty() {
            out.push(claim);
        }
    }
    Ok(out)
}
