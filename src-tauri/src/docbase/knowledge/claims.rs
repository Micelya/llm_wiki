//! The claims of the knowledge layer: what the documents say of each
//! entity, once its mentions are resolved (D-191).
//!
//! - The evidence that says the same (the same subject, predicate and
//!   object, the object being another entity or a value, compared by its
//!   normalized words) is **one claim** with several pieces of evidence.
//! - A predicate that takes one value at a time (`single` in the
//!   vocabulary: a domicile, a marital status) with different values is a
//!   **conflict** when the times are the same or not said; when every
//!   value has its own time it is a **history**, each claim from its time.
//!   A conflict is kept, never resolved: both are shown.
//! - What a person marked as wrong (a piece of evidence, or a whole claim)
//!   does not count; a claim marked wrong is kept, retired.
//!
//! Computed again from the evidence on every resolution; only what
//! changed is written.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection};

use super::quote;
use super::vocabulary::Vocabulary;
use crate::docbase::{now_ms, sql_err};

/// What a consolidation left.
#[derive(Debug, Default)]
pub(crate) struct Consolidated {
    pub claims: usize,
    pub conflicts: usize,
    /// Entities whose claims changed.
    pub touched: BTreeSet<String>,
}

/// The key a person marks a piece of evidence as wrong by: its document,
/// the name of its subject, the predicate and the object. It survives
/// reading the document again.
pub(crate) fn evidence_key(content_id: i64, subject: &str, predicate: &str, object: &str) -> String {
    format!("evidence:{content_id}:{subject}:{predicate}:{object}")
}

/// The key of a claim: its subject entity, predicate and object.
pub(crate) fn claim_key(subject: &str, predicate: &str, object_entity: Option<&str>, object_normalized: Option<&str>) -> String {
    match object_entity {
        Some(entity) => format!("claim:{subject}:{predicate}:entity:{entity}"),
        None => format!("claim:{subject}:{predicate}:value:{}", object_normalized.unwrap_or_default()),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Draft {
    subject: String,
    predicate: String,
    object_entity: Option<String>,
    object_value: Option<String>,
    object_normalized: Option<String>,
    evidence: BTreeSet<i64>,
    times: BTreeMap<String, String>,
    valid_from: Option<String>,
    state: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct Stored {
    id: i64,
    valid_from: Option<String>,
    state: String,
    evidence: BTreeSet<i64>,
}

pub(crate) fn consolidate(conn: &Connection, vocabulary: &Vocabulary, wrong: &BTreeSet<String>) -> Result<Consolidated, String> {
    struct Piece {
        id: i64,
        content: i64,
        subject_entity: Option<String>,
        subject_name: String,
        predicate: String,
        object_entity: Option<String>,
        object_name: Option<String>,
        value: Option<String>,
        normalized: Option<String>,
        time: Option<String>,
    }
    let mut statement = conn
        .prepare(
            "SELECT e.id, e.content_id, s.entity_id, s.normalized, e.predicate, o.entity_id, o.normalized,
                    e.object_value, e.object_normalized, e.fact_time
             FROM knowledge_evidence e
             JOIN knowledge_mention s ON s.id = e.subject_mention
             LEFT JOIN knowledge_mention o ON o.id = e.object_mention
             ORDER BY e.content_id, e.id",
        )
        .map_err(sql_err)?;
    let pieces: Vec<Piece> = statement
        .query_map([], |row| {
            Ok(Piece {
                id: row.get(0)?,
                content: row.get(1)?,
                subject_entity: row.get(2)?,
                subject_name: row.get(3)?,
                predicate: row.get(4)?,
                object_entity: row.get(5)?,
                object_name: row.get(6)?,
                value: row.get(7)?,
                normalized: row.get(8)?,
                time: row.get(9)?,
            })
        })
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    drop(statement);

    let mut drafts: BTreeMap<String, Draft> = BTreeMap::new();
    let mut evidence_claim: BTreeMap<i64, String> = BTreeMap::new();
    for piece in &pieces {
        let Some(subject) = &piece.subject_entity else { continue };
        let object_word = match (&piece.object_name, &piece.normalized) {
            (Some(name), _) => format!("m:{name}"),
            (None, Some(value)) => value.clone(),
            _ => String::new(),
        };
        if wrong.contains(&evidence_key(piece.content, &piece.subject_name, &piece.predicate, &object_word)) {
            continue;
        }
        let object_entity = match (&piece.object_name, &piece.object_entity) {
            (Some(_), Some(entity)) if entity != subject => Some(entity.clone()),
            (Some(_), _) => continue,
            (None, _) => None,
        };
        let normalized = object_entity.is_none().then(|| piece.normalized.clone()).flatten();
        if object_entity.is_none() && normalized.as_deref().map_or(true, str::is_empty) {
            continue;
        }
        let key = claim_key(subject, &piece.predicate, object_entity.as_deref(), normalized.as_deref());
        let draft = drafts.entry(key.clone()).or_insert_with(|| Draft {
            subject: subject.clone(),
            predicate: piece.predicate.clone(),
            object_value: if object_entity.is_none() { piece.value.clone() } else { None },
            object_entity: object_entity.clone(),
            object_normalized: normalized.clone(),
            evidence: BTreeSet::new(),
            times: BTreeMap::new(),
            valid_from: None,
            state: "supported",
        });
        draft.evidence.insert(piece.id);
        if let Some(time) = piece.time.as_deref().filter(|t| !t.trim().is_empty()) {
            draft.times.entry(quote::words(time).join(" ")).or_insert_with(|| time.trim().to_string());
        }
        evidence_claim.insert(piece.id, key);
    }
    for draft in drafts.values_mut() {
        if draft.times.len() == 1 {
            draft.valid_from = draft.times.values().next().cloned();
        }
    }
    // One value at a time: different values for the same time (or no
    // time) are a conflict; each with its own time, a history.
    let mut by_subject: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for (key, draft) in &drafts {
        if draft.object_entity.is_none() && vocabulary.predicate(&draft.predicate).is_some_and(|p| p.single) {
            by_subject.entry((draft.subject.clone(), draft.predicate.clone())).or_default().push(key.clone());
        }
    }
    for keys in by_subject.values() {
        if keys.len() < 2 {
            continue;
        }
        let mut times: HashMap<String, usize> = HashMap::new();
        for key in keys {
            if let Some(time) = &drafts[key].valid_from {
                *times.entry(quote::words(time).join(" ")).or_default() += 1;
            }
        }
        for key in keys {
            let draft = drafts.get_mut(key).expect("listed");
            let alone = draft.valid_from.as_ref().is_some_and(|t| times[&quote::words(t).join(" ")] == 1);
            if !alone {
                draft.state = "conflict";
            }
        }
    }
    for (key, draft) in drafts.iter_mut() {
        if wrong.contains(key) {
            draft.state = "retired";
        }
    }

    // What is stored, to write only what changed.
    let mut stored: BTreeMap<String, Stored> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare("SELECT id, subject, predicate, object_entity, object_normalized, valid_from, state FROM knowledge_claim")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (id, subject, predicate, entity, normalized, valid_from, state) = row.map_err(sql_err)?;
            let key = claim_key(&subject, &predicate, entity.as_deref(), normalized.as_deref());
            stored.insert(key, Stored { id, valid_from, state, evidence: BTreeSet::new() });
        }
        let mut statement = conn.prepare("SELECT id, claim_id FROM knowledge_evidence WHERE claim_id IS NOT NULL").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).map_err(sql_err)?;
        let by_id: HashMap<i64, String> = stored.iter().map(|(k, s)| (s.id, k.clone())).collect();
        for row in rows {
            let (evidence, claim) = row.map_err(sql_err)?;
            if let Some(key) = by_id.get(&claim) {
                stored.get_mut(key).expect("listed").evidence.insert(evidence);
            }
        }
    }
    let now = now_ms();
    let mut out = Consolidated::default();
    let mut ids: HashMap<String, i64> = HashMap::new();
    for (key, draft) in &drafts {
        match stored.get(key) {
            Some(old) => {
                ids.insert(key.clone(), old.id);
                if old.valid_from != draft.valid_from || old.state != draft.state || old.evidence != draft.evidence {
                    conn.execute(
                        "UPDATE knowledge_claim SET valid_from = ?2, state = ?3, object_value = coalesce(?4, object_value), computed_at = ?5
                         WHERE id = ?1",
                        params![old.id, draft.valid_from, draft.state, draft.object_value, now],
                    )
                    .map_err(sql_err)?;
                    out.touched.insert(draft.subject.clone());
                    if let Some(object) = &draft.object_entity {
                        out.touched.insert(object.clone());
                    }
                }
            }
            None => {
                conn.execute(
                    "INSERT INTO knowledge_claim (subject, predicate, object_entity, object_value, object_normalized, valid_from, state, computed_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        draft.subject,
                        draft.predicate,
                        draft.object_entity,
                        draft.object_value,
                        draft.object_normalized,
                        draft.valid_from,
                        draft.state,
                        now
                    ],
                )
                .map_err(sql_err)?;
                ids.insert(key.clone(), conn.last_insert_rowid());
                out.touched.insert(draft.subject.clone());
                if let Some(object) = &draft.object_entity {
                    out.touched.insert(object.clone());
                }
            }
        }
    }
    for (key, old) in &stored {
        if !drafts.contains_key(key) {
            let subject: Option<String> = conn
                .query_row("SELECT subject FROM knowledge_claim WHERE id = ?1", [old.id], |row| row.get(0))
                .map_err(sql_err)
                .ok();
            conn.execute("DELETE FROM knowledge_claim WHERE id = ?1", [old.id]).map_err(sql_err)?;
            out.touched.extend(subject);
        }
    }
    for piece in &pieces {
        let claim = evidence_claim.get(&piece.id).and_then(|k| ids.get(k)).copied();
        conn.execute("UPDATE knowledge_evidence SET claim_id = ?2 WHERE id = ?1 AND claim_id IS NOT ?2", params![piece.id, claim])
            .map_err(sql_err)?;
    }
    out.claims = drafts.values().filter(|d| d.state != "retired").count();
    out.conflicts = drafts.values().filter(|d| d.state == "conflict").count();
    Ok(out)
}
