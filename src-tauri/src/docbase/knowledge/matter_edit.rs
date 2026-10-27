//! A matter made and edited by hand (D-386): a person creates one (empty
//! or with a document), names it, and puts documents in it or takes them
//! out. What a person does wins over what the grouping proposes and
//! survives it: the name stays (`name_by_person`), and each document put
//! in or taken out is kept in `knowledge_matter_choice`, which the grouping
//! applies over what it finds (`matters::group`). Putting a document in a
//! proposed matter, or taking one out, is deciding about it: it becomes
//! confirmed. A document can be in several matters.
//!
//! A matter has a status (D-475): `open`, `closed` or `archived`. A closed
//! or archived one leaves the lists and Atención, stays in the search with
//! its mark and gets no new proposals; it can be reopened.
//!
//! What a matter and the choices about some of its documents were can be
//! kept and put back (`MatterSnapshot`): how the operations of the
//! contract undo what a person confirmed (`operations/apply.rs`).

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::vocabulary::vocabulary;
use crate::docbase::{now_ms, sql_err, DocBase};

/// A document of a matter, as it was.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterDocumentSnapshot {
    pub content_id: i64,
    pub state: String,
    pub reasons: String,
}

/// What a person had chosen about a document of a matter, if anything.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChoiceSnapshot {
    pub content_id: i64,
    /// `in`, `out`, or none.
    pub choice: Option<String>,
    pub decided_at: Option<i64>,
    /// How it was in the matter: none when it was not.
    pub member: Option<MatterDocumentSnapshot>,
}

/// A matter as it was, to put it back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterSnapshot {
    pub id: String,
    pub name: String,
    pub name_by_person: bool,
    pub state: String,
    pub status: String,
    pub origin: String,
    pub confirmation: String,
    pub decided_at: Option<i64>,
    pub decided_documents: Option<String>,
    /// Every document it had.
    pub documents: Vec<MatterDocumentSnapshot>,
    /// The documents it was taken for, with the choices about them.
    pub choices: Vec<ChoiceSnapshot>,
}

/// The status words a matter can have (D-475).
pub const MATTER_STATUSES: &[&str] = &["open", "closed", "archived"];

impl DocBase {
    /// Creates a matter by hand, with a name and the documents given
    /// (`content:<id>` or `location:<id>`; none for an empty one). Returns
    /// its id.
    pub fn knowledge_create_matter(&self, name: &str, documents: &[String]) -> Result<String, String> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            return Err("Falta el nombre".to_string());
        }
        let contents: Vec<i64> = documents.iter().map(|d| self.content_of_reference(d)).collect::<Result<_, _>>()?;
        let conn = self.conn();
        let now = now_ms();
        let mut id = format!("mat-p-{now:x}");
        let mut n = 2;
        while conn.query_row("SELECT 1 FROM knowledge_matter WHERE id = ?1", [&id], |_| Ok(())).optional().map_err(sql_err)?.is_some() {
            id = format!("mat-p-{now:x}-{n}");
            n += 1;
        }
        conn.execute(
            "INSERT INTO knowledge_matter (id, name, state, created_at, updated_at, status, origin, confirmation, name_by_person, decided_at)
             VALUES (?1, ?2, 'active', ?3, ?3, 'open', 'person', 'confirmed', 1, ?3)",
            params![id, name, now],
        )
        .map_err(sql_err)?;
        for content in contents {
            put(&conn, &id, content, now)?;
        }
        super::mark_dirty(&conn, "project", "matters")?;
        Ok(id)
    }

    /// Names a matter by hand: the grouping keeps the name. An empty name
    /// gives a proposed matter back the name the app makes for it.
    pub fn knowledge_rename_matter(&self, matter_id: &str, name: &str) -> Result<(), String> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        let conn = self.conn();
        let origin = origin_of(&conn, matter_id)?;
        let now = now_ms();
        if name.is_empty() {
            if origin == "person" {
                return Err("Falta el nombre".to_string());
            }
            conn.execute("UPDATE knowledge_matter SET name_by_person = 0, updated_at = ?2 WHERE id = ?1", params![matter_id, now]).map_err(sql_err)?;
            let decided = super::resolve::load_decided(&conn)?;
            super::matters::group(&conn, vocabulary(), &decided.joins, &decided.separations)?;
        } else {
            conn.execute(
                "UPDATE knowledge_matter SET name = ?2, name_by_person = 1, updated_at = ?3 WHERE id = ?1",
                params![matter_id, name, now],
            )
            .map_err(sql_err)?;
        }
        super::mark_dirty(&conn, "project", "matters")
    }

    /// Puts a document (`content:<id>` or `location:<id>`) in a matter
    /// (`in`) or takes it out (`out`), by hand. A proposed matter becomes
    /// confirmed: a person decided about it.
    pub fn knowledge_choose_matter_document(&self, matter_id: &str, document: &str, choice: &str) -> Result<(), String> {
        let content = self.content_of_reference(document)?;
        let conn = self.conn();
        origin_of(&conn, matter_id)?;
        let now = now_ms();
        match choice {
            "in" => put(&conn, matter_id, content, now)?,
            "out" => {
                let (origin, others): (String, i64) = conn
                    .query_row(
                        "SELECT m.origin, (SELECT count(*) FROM knowledge_matter_document d WHERE d.matter_id = m.id AND d.version_id != ?2)
                         FROM knowledge_matter m WHERE m.id = ?1",
                        params![matter_id, content],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(sql_err)?;
                if origin == "proposed" && others == 0 {
                    return Err("Es su último documento: para que no quede, descartalo".to_string());
                }
                conn.execute(
                    "INSERT INTO knowledge_matter_choice (matter_id, version_id, choice, decided_at) VALUES (?1, ?2, 'out', ?3)
                     ON CONFLICT (matter_id, version_id) DO UPDATE SET choice = 'out', decided_at = excluded.decided_at",
                    params![matter_id, content, now],
                )
                .map_err(sql_err)?;
                conn.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1 AND version_id = ?2", params![matter_id, content])
                    .map_err(sql_err)?;
            }
            other => return Err(format!("'{other}' no es una elección sobre un documento")),
        }
        conn.execute(
            "UPDATE knowledge_matter SET confirmation = 'confirmed', decided_at = ?2, updated_at = ?2 WHERE id = ?1 AND confirmation = 'proposed'",
            params![matter_id, now],
        )
        .map_err(sql_err)?;
        super::mark_dirty(&conn, "project", "matters")
    }
}

impl DocBase {
    /// A matter as it is now, with the choices about these documents.
    pub fn knowledge_matter_snapshot(&self, matter_id: &str, contents: &[i64]) -> Result<MatterSnapshot, String> {
        let conn = self.conn();
        type Row = (String, i64, String, String, String, String, Option<i64>, Option<String>);
        let row: Option<Row> = conn
            .query_row(
                "SELECT name, name_by_person, state, status, origin, confirmation, decided_at, decided_documents FROM knowledge_matter WHERE id = ?1",
                [matter_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((name, name_by_person, state, status, origin, confirmation, decided_at, decided_documents)) = row else {
            return Err("Ese asunto ya no existe".to_string());
        };
        let member = |content: i64| -> Result<Option<MatterDocumentSnapshot>, String> {
            conn.query_row(
                "SELECT state, reasons FROM knowledge_matter_document WHERE matter_id = ?1 AND version_id = ?2",
                params![matter_id, content],
                |row| Ok(MatterDocumentSnapshot { content_id: content, state: row.get(0)?, reasons: row.get(1)? }),
            )
            .optional()
            .map_err(sql_err)
        };
        let documents: Vec<MatterDocumentSnapshot> = {
            let mut statement = conn
                .prepare("SELECT version_id, state, reasons FROM knowledge_matter_document WHERE matter_id = ?1 ORDER BY version_id")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([matter_id], |row| Ok(MatterDocumentSnapshot { content_id: row.get(0)?, state: row.get(1)?, reasons: row.get(2)? }))
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let mut choices = Vec::new();
        for &content in contents {
            let chosen: Option<(String, i64)> = conn
                .query_row(
                    "SELECT choice, decided_at FROM knowledge_matter_choice WHERE matter_id = ?1 AND version_id = ?2",
                    params![matter_id, content],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(sql_err)?;
            choices.push(ChoiceSnapshot {
                content_id: content,
                choice: chosen.as_ref().map(|c| c.0.clone()),
                decided_at: chosen.map(|c| c.1),
                member: member(content)?,
            });
        }
        Ok(MatterSnapshot {
            id: matter_id.to_string(),
            name,
            name_by_person: name_by_person != 0,
            state,
            status,
            origin,
            confirmation,
            decided_at,
            decided_documents,
            documents,
            choices,
        })
    }

    /// Puts back what a snapshot kept: the choices about its documents and
    /// how each was in the matter, and its confirmation. `whole`: also its
    /// state, status, name and every document it had, and the matters are
    /// grouped again (how a decision about the matter is undone).
    pub fn knowledge_restore_matter(&self, snapshot: &MatterSnapshot, whole: bool) -> Result<(), String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        let exists: bool = tx.query_row("SELECT 1 FROM knowledge_matter WHERE id = ?1", [&snapshot.id], |_| Ok(())).optional().map_err(sql_err)?.is_some();
        if !exists {
            return Err("Ese asunto ya no existe".to_string());
        }
        let now = now_ms();
        for choice in &snapshot.choices {
            match (&choice.choice, choice.decided_at) {
                (Some(kind), Some(at)) => {
                    tx.execute(
                        "INSERT INTO knowledge_matter_choice (matter_id, version_id, choice, decided_at) VALUES (?1, ?2, ?3, ?4)
                         ON CONFLICT (matter_id, version_id) DO UPDATE SET choice = excluded.choice, decided_at = excluded.decided_at",
                        params![snapshot.id, choice.content_id, kind, at],
                    )
                    .map_err(sql_err)?;
                }
                _ => {
                    tx.execute("DELETE FROM knowledge_matter_choice WHERE matter_id = ?1 AND version_id = ?2", params![snapshot.id, choice.content_id])
                        .map_err(sql_err)?;
                }
            }
            if !whole {
                match &choice.member {
                    Some(member) => {
                        tx.execute(
                            "INSERT INTO knowledge_matter_document (matter_id, version_id, state, reasons) VALUES (?1, ?2, ?3, ?4)
                             ON CONFLICT (matter_id, version_id) DO UPDATE SET state = excluded.state, reasons = excluded.reasons",
                            params![snapshot.id, member.content_id, member.state, member.reasons],
                        )
                        .map_err(sql_err)?;
                    }
                    None => {
                        tx.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1 AND version_id = ?2", params![snapshot.id, choice.content_id])
                            .map_err(sql_err)?;
                    }
                }
            }
        }
        if whole {
            tx.execute(
                "UPDATE knowledge_matter SET name = ?2, name_by_person = ?3, state = ?4, status = ?5, confirmation = ?6, decided_at = ?7,
                        decided_documents = ?8, updated_at = ?9 WHERE id = ?1",
                params![
                    snapshot.id,
                    snapshot.name,
                    snapshot.name_by_person as i64,
                    snapshot.state,
                    snapshot.status,
                    snapshot.confirmation,
                    snapshot.decided_at,
                    snapshot.decided_documents,
                    now
                ],
            )
            .map_err(sql_err)?;
            tx.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1", [&snapshot.id]).map_err(sql_err)?;
            for document in &snapshot.documents {
                let present: bool =
                    tx.query_row("SELECT 1 FROM version WHERE id = ?1", [document.content_id], |_| Ok(())).optional().map_err(sql_err)?.is_some();
                if present {
                    tx.execute(
                        "INSERT INTO knowledge_matter_document (matter_id, version_id, state, reasons) VALUES (?1, ?2, ?3, ?4)",
                        params![snapshot.id, document.content_id, document.state, document.reasons],
                    )
                    .map_err(sql_err)?;
                }
            }
        } else {
            tx.execute(
                "UPDATE knowledge_matter SET confirmation = ?2, decided_at = ?3, updated_at = ?4 WHERE id = ?1",
                params![snapshot.id, snapshot.confirmation, snapshot.decided_at, now],
            )
            .map_err(sql_err)?;
        }
        super::mark_dirty(&tx, "project", "matters")?;
        if whole {
            let decided = super::resolve::load_decided(&tx)?;
            super::matters::group(&tx, vocabulary(), &decided.joins, &decided.separations)?;
        }
        tx.commit().map_err(sql_err)
    }

    /// Closes, archives or reopens a matter (D-475): `open`, `closed` or
    /// `archived`. Kept on the matter: a grouping never changes it.
    pub fn knowledge_set_matter_status(&self, matter_id: &str, status: &str) -> Result<(), String> {
        if !MATTER_STATUSES.contains(&status) {
            return Err(format!("'{status}' no es un estado de un asunto"));
        }
        let conn = self.conn();
        origin_of(&conn, matter_id)?;
        conn.execute("UPDATE knowledge_matter SET status = ?2, updated_at = ?3 WHERE id = ?1", params![matter_id, status, now_ms()])
            .map_err(sql_err)?;
        super::mark_dirty(&conn, "project", "matters")
    }
}

/// The origin of an active matter, or why it cannot be edited.
fn origin_of(conn: &rusqlite::Connection, matter_id: &str) -> Result<String, String> {
    let found: Option<(String, String)> = conn
        .query_row("SELECT origin, state FROM knowledge_matter WHERE id = ?1", [matter_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(sql_err)?;
    match found {
        None => Err("Ya no existe: se agrupó de otra manera".to_string()),
        Some((_, state)) if state != "active" => Err("Ya no está activo".to_string()),
        Some((origin, _)) => Ok(origin),
    }
}

/// Puts a document in a matter, as a person's choice.
pub(super) fn put(conn: &rusqlite::Connection, matter_id: &str, content: i64, now: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO knowledge_matter_choice (matter_id, version_id, choice, decided_at) VALUES (?1, ?2, 'in', ?3)
         ON CONFLICT (matter_id, version_id) DO UPDATE SET choice = 'in', decided_at = excluded.decided_at",
        params![matter_id, content, now],
    )
    .map_err(sql_err)?;
    conn.execute(
        "INSERT INTO knowledge_matter_document (matter_id, version_id, state, reasons) VALUES (?1, ?2, 'firm', '[]')
         ON CONFLICT (matter_id, version_id) DO UPDATE SET state = 'firm'",
        params![matter_id, content],
    )
    .map_err(sql_err)?;
    Ok(())
}
