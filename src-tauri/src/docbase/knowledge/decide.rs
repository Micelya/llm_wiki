//! What a person decides about identities (D-189): "these two are the
//! same", "these two are not", "this datum is wrong", "this datum is right"
//! (a claim, D-320), and undoing any of
//! them, the automatic joins included (a separation undoes them). A person
//! can also choose how an entity is named (D-516).
//!
//! A decision is kept with the evidence that was shown and its digest, as
//! the decisions about similar documents (`similar/store.rs`), and refers
//! to what does not change when the archive is read again: a group of
//! mentions of one document (`cluster:<content>:<kind>:<name>`), a piece
//! of evidence (`evidence:<content>:<subject>:<predicate>:<object>`) or a
//! claim (`claim:<entity>:<predicate>:<object>`). The screens name an
//! entity, a mention, a piece of evidence or a claim by the ids they show
//! (`entity:<id>`, `mention:<id>`, `evidence:<id>`, `claim:<id>`); they are
//! turned into those keys here. Every resolution reads the decisions
//! again: a computation never overwrites them (R-08, R-09). A join is
//! firm; a separation wins over every other evidence, a person's join
//! included.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::claims::{claim_key, evidence_key};
use super::mark_dirty;
use super::resolve::{cluster_key, parse_cluster_key};
use crate::docbase::{now_ms, sql_err, DocBase};

/// A decision of a person about identities.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum IdentityDecision {
    /// `a` and `b` are the same entity (entities, mentions or groups).
    Join { a: String, b: String },
    /// `a` and `b` are not the same: kept apart in every resolution. To
    /// take a mention out of its entity, `a` is the mention and `b` the
    /// entity.
    Separate { a: String, b: String },
    /// `a` is wrong: a mention that is not an entity, a piece of evidence
    /// or a claim that the document does not say.
    Wrong { a: String },
    /// `a`, a claim, is right: a person read it in its document (D-320).
    Confirm { a: String },
    /// Two documents (`content:<id>` or `location:<id>`) are of the same
    /// matter.
    SameMatter { a: String, b: String },
    /// Two documents are not of the same matter: never grouped together.
    OtherMatter { a: String, b: String },
    /// `a`, an entity, is named `name` (D-516): one of the ways its
    /// documents write it, or one a person writes. It stands over what the
    /// documents say until it is undone.
    Name { a: String, name: String },
}

/// The key of a document a person decides matters by.
fn document_key(conn: &Connection, reference: &str) -> Result<String, String> {
    if let Some(id) = id_of(reference, "content:") {
        return Ok(super::matters::content_key(id));
    }
    if let Some(id) = id_of(reference, "location:") {
        let content: Option<i64> = conn
            .query_row("SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.id = ?1", [id], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        return content.map(super::matters::content_key).ok_or_else(|| "Ese documento ya no existe".to_string());
    }
    Err(format!("'{reference}' no es un documento"))
}

/// A decision as the screens show it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DecisionView {
    pub id: i64,
    /// "join", "separate" or "wrong".
    pub kind: String,
    pub a: String,
    pub b: Option<String>,
    pub note: Option<String>,
    pub shown_evidence: String,
    /// "active" or "undone".
    pub state: String,
    pub decided_at: i64,
    pub undone_at: Option<i64>,
    /// Why an active decision no longer finds what it named in the
    /// documents read again ("no-encontrada", "varias"): for a person to
    /// review (D-321).
    pub review: Option<String>,
    /// The keys it applies to now, when what it named is written another
    /// way (JSON `{"a", "b"}`).
    pub applied_as: Option<String>,
}

/// The group of mentions that stands for an entity: its most used name,
/// in the first document that writes it so, other than `avoid`.
fn representative(conn: &Connection, entity: &str, avoid: Option<&str>) -> Result<String, String> {
    let mut statement = conn
        .prepare(
            "SELECT content_id, kind, normalized, count(*) OVER (PARTITION BY normalized) AS uses FROM knowledge_mention
             WHERE entity_id = ?1 ORDER BY uses DESC, normalized, content_id",
        )
        .map_err(sql_err)?;
    let rows: Vec<(i64, String, String)> = statement
        .query_map([entity], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    let avoided = avoid.and_then(parse_cluster_key);
    rows.into_iter()
        .find(|(content, kind, normalized)| {
            avoided.as_ref().is_none_or(|(c, k, _)| !(c == content && k == kind))
                && avoid != Some(cluster_key(*content, kind, normalized).as_str())
        })
        .map(|(content, kind, normalized)| cluster_key(content, &kind, &normalized))
        .ok_or_else(|| "Esa entidad no tiene menciones que se puedan usar para decidir".to_string())
}

fn id_of(reference: &str, prefix: &str) -> Option<i64> {
    reference.strip_prefix(prefix)?.parse().ok()
}

/// The key of a group of mentions, from an entity, a mention or a key.
fn group_key(conn: &Connection, reference: &str, avoid: Option<&str>) -> Result<String, String> {
    if let Some(entity) = reference.strip_prefix("entity:") {
        return representative(conn, entity, avoid);
    }
    if let Some(id) = id_of(reference, "mention:") {
        let found: Option<(i64, String, String)> = conn
            .query_row("SELECT content_id, kind, normalized FROM knowledge_mention WHERE id = ?1", [id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()
            .map_err(sql_err)?;
        let (content, kind, normalized) = found.ok_or_else(|| "Esa mención ya no existe".to_string())?;
        return Ok(cluster_key(content, &kind, &normalized));
    }
    if parse_cluster_key(reference).is_some() {
        return Ok(reference.to_string());
    }
    Err(format!("'{reference}' no es una entidad ni una mención"))
}

/// The key of what can be wrong: a mention, a piece of evidence or a claim.
fn wrong_key(conn: &Connection, reference: &str) -> Result<String, String> {
    if let Some(id) = id_of(reference, "evidence:") {
        let found: Option<(i64, String, String, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT e.content_id, s.normalized, e.predicate, o.normalized, e.object_normalized FROM knowledge_evidence e
                 JOIN knowledge_mention s ON s.id = e.subject_mention LEFT JOIN knowledge_mention o ON o.id = e.object_mention
                 WHERE e.id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let (content, subject, predicate, object, value) = found.ok_or_else(|| "Ese dato ya no existe".to_string())?;
        let object = match object {
            Some(name) => format!("m:{name}"),
            None => value.unwrap_or_default(),
        };
        return Ok(evidence_key(content, &subject, &predicate, &object));
    }
    if let Some(id) = id_of(reference, "claim:") {
        let found: Option<(String, String, Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT subject, predicate, object_entity, object_normalized FROM knowledge_claim WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let (subject, predicate, entity, value) = found.ok_or_else(|| "Esa afirmación ya no existe".to_string())?;
        return Ok(claim_key(&subject, &predicate, entity.as_deref(), value.as_deref()));
    }
    if reference.starts_with("evidence:") || reference.starts_with("claim:") {
        return Ok(reference.to_string());
    }
    group_key(conn, reference, None)
}

/// What a person saw, when the screen does not say: the names of both sides.
fn shown_by_default(conn: &Connection, keys: &[&str]) -> Result<String, String> {
    let mut sides = Vec::new();
    for key in keys {
        let names: Vec<String> = match parse_cluster_key(key) {
            Some((content, kind, normalized)) => {
                let mut statement = conn
                    .prepare("SELECT DISTINCT written FROM knowledge_mention WHERE content_id = ?1 AND kind = ?2 AND normalized = ?3")
                    .map_err(sql_err)?;
                let rows = statement.query_map(params![content, kind, normalized], |row| row.get(0)).map_err(sql_err)?;
                rows.collect::<Result<_, _>>().map_err(sql_err)?
            }
            None => Vec::new(),
        };
        sides.push(serde_json::json!({ "key": key, "names": names }));
    }
    Ok(serde_json::Value::Array(sides).to_string())
}

/// Keeps a decision of a person about identities, with the evidence that
/// was shown (or, when the screen does not say, the names of both sides),
/// and marks the resolution to be done again. Returns its id.
fn record(conn: &Connection, decision: &IdentityDecision, note: Option<&str>, shown_evidence: Option<&str>) -> Result<i64, String> {
    let (kind, a, b) = match decision {
        IdentityDecision::Join { a, b } | IdentityDecision::Separate { a, b } => {
            let first = group_key(conn, a, None)?;
            // Taking a mention out of its entity: the entity stands
            // for itself by a mention of another document.
            let avoid = matches!(decision, IdentityDecision::Separate { .. }).then_some(first.as_str());
            let second = group_key(conn, b, avoid)?;
            let (ka, kb) = (parse_cluster_key(&first), parse_cluster_key(&second));
            if ka.as_ref().map(|k| &k.1) != kb.as_ref().map(|k| &k.1) {
                return Err("Solo se puede decidir sobre dos entidades del mismo tipo".to_string());
            }
            if first == second {
                return Err("Las dos partes son la misma mención".to_string());
            }
            let kind = if matches!(decision, IdentityDecision::Join { .. }) { "join" } else { "separate" };
            (kind, first, Some(second))
        }
        IdentityDecision::Wrong { a } => ("wrong", wrong_key(conn, a)?, None),
        IdentityDecision::Confirm { a } => {
            let key = wrong_key(conn, a)?;
            if !key.starts_with("claim:") {
                return Err("Solo se puede confirmar una afirmación".to_string());
            }
            ("confirm", key, None)
        }
        IdentityDecision::Name { a, name } => {
            let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
            if name.is_empty() || name.chars().count() > 200 {
                return Err("El nombre no puede estar vacío ni pasar de 200 letras".to_string());
            }
            ("name", group_key(conn, a, None)?, Some(format!("{}{name}", super::resolve::NAME_PREFIX)))
        }
        IdentityDecision::SameMatter { a, b } | IdentityDecision::OtherMatter { a, b } => {
            let (first, second) = (document_key(conn, a)?, document_key(conn, b)?);
            if first == second {
                return Err("Las dos partes son el mismo documento".to_string());
            }
            let kind = if matches!(decision, IdentityDecision::SameMatter { .. }) { "join" } else { "separate" };
            (kind, first, Some(second))
        }
    };
    let shown = match shown_evidence {
        Some(text) => text.to_string(),
        None => shown_by_default(conn, &[a.as_str(), b.as_deref().unwrap_or_default()])?,
    };
    let digest: String = Sha256::digest(shown.as_bytes()).iter().take(16).map(|b| format!("{b:02x}")).collect();
    conn.execute(
        "INSERT INTO knowledge_decision (kind, a, b, note, shown_evidence, shown_evidence_hash, state, decided_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7)",
        params![kind, a, b, note, shown, digest, now_ms()],
    )
    .map_err(sql_err)?;
    let id = conn.last_insert_rowid();
    mark_dirty(conn, "resolve", &format!("decision:{id}"))?;
    Ok(id)
}

impl DocBase {
    /// Keeps a decision of a person about identities, with the evidence
    /// that was shown (or, when the screen does not say, the names of both
    /// sides), and applies it: the next resolution, which this one starts
    /// without asking the model, reads it. Returns its id.
    pub fn knowledge_decide(&self, decision: &IdentityDecision, note: Option<&str>, shown_evidence: Option<&str>) -> Result<i64, String> {
        let id = record(&self.conn(), decision, note, shown_evidence)?;
        self.knowledge_resolve(None, 0)?;
        Ok(id)
    }

    /// Keeps several decisions of a person at once (taking a document out
    /// of a matter keeps it apart from each of its documents), with what
    /// the screen showed of each by default, and applies them with one
    /// resolution. Nothing is kept when one of them cannot be.
    pub fn knowledge_decide_many(&self, decisions: &[IdentityDecision], note: Option<&str>) -> Result<Vec<i64>, String> {
        let ids = {
            let mut conn = self.conn();
            let tx = conn.transaction().map_err(sql_err)?;
            let mut ids = Vec::new();
            for decision in decisions {
                ids.push(record(&tx, decision, note, None)?);
            }
            tx.commit().map_err(sql_err)?;
            ids
        };
        self.knowledge_resolve(None, 0)?;
        Ok(ids)
    }

    /// Undoes a decision: it is kept, marked undone, and stops counting.
    pub fn knowledge_undo(&self, id: i64) -> Result<(), String> {
        {
            let conn = self.conn();
            let changed = conn
                .execute("UPDATE knowledge_decision SET state = 'undone', undone_at = ?2 WHERE id = ?1 AND state = 'active'", params![id, now_ms()])
                .map_err(sql_err)?;
            if changed == 0 {
                return Err("Esa decisión no existe o ya se deshizo".to_string());
            }
            mark_dirty(&conn, "resolve", &format!("decision:{id}"))?;
        }
        self.knowledge_resolve(None, 0)?;
        Ok(())
    }

    /// Every decision about identities, the latest first.
    pub fn knowledge_decisions(&self) -> Result<Vec<DecisionView>, String> {
        let conn = self.conn();
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
}
