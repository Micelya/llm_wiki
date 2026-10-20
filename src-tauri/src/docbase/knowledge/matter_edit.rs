//! A matter made and edited by hand (D-386): a person creates one (empty
//! or with a document), names it, and puts documents in it or takes them
//! out. What a person does wins over what the grouping proposes and
//! survives it: the name stays (`name_by_person`), and each document put
//! in or taken out is kept in `knowledge_matter_choice`, which the grouping
//! applies over what it finds (`matters::group`). Putting a document in a
//! proposed matter, or taking one out, is deciding about it: it becomes
//! confirmed. A document can be in several matters.

use rusqlite::{params, OptionalExtension};

use super::vocabulary::vocabulary;
use crate::docbase::{now_ms, sql_err, DocBase};

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
                        "SELECT m.origin, (SELECT count(*) FROM knowledge_matter_document d WHERE d.matter_id = m.id AND d.content_id != ?2)
                         FROM knowledge_matter m WHERE m.id = ?1",
                        params![matter_id, content],
                        |row| Ok((row.get(0)?, row.get(1)?)),
                    )
                    .map_err(sql_err)?;
                if origin == "proposed" && others == 0 {
                    return Err("Es su último documento: para que no quede, descartalo".to_string());
                }
                conn.execute(
                    "INSERT INTO knowledge_matter_choice (matter_id, content_id, choice, decided_at) VALUES (?1, ?2, 'out', ?3)
                     ON CONFLICT (matter_id, content_id) DO UPDATE SET choice = 'out', decided_at = excluded.decided_at",
                    params![matter_id, content, now],
                )
                .map_err(sql_err)?;
                conn.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1 AND content_id = ?2", params![matter_id, content])
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
fn put(conn: &rusqlite::Connection, matter_id: &str, content: i64, now: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO knowledge_matter_choice (matter_id, content_id, choice, decided_at) VALUES (?1, ?2, 'in', ?3)
         ON CONFLICT (matter_id, content_id) DO UPDATE SET choice = 'in', decided_at = excluded.decided_at",
        params![matter_id, content, now],
    )
    .map_err(sql_err)?;
    conn.execute(
        "INSERT INTO knowledge_matter_document (matter_id, content_id, state, reasons) VALUES (?1, ?2, 'firm', '[]')
         ON CONFLICT (matter_id, content_id) DO UPDATE SET state = 'firm'",
        params![matter_id, content],
    )
    .map_err(sql_err)?;
    Ok(())
}
