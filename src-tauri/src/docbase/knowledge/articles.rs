//! What the screens, the search and the chat read of the pages kept in the
//! base (D-402, D-404): the article of an object, the object a logical path
//! or a link names, the notes of a person, the overview of the archive and
//! the saved queries. Nothing here reads or writes a file.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::pages::{notes_of_object, stem, title_of, with_notes, OVERVIEW_PATH};
use super::vocabulary::vocabulary;
use crate::docbase::{now_ms, sql_err, DocBase};

/// The notes of a person on an object, or kept apart.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeNote {
    pub id: i64,
    /// `ent-…`, `mat-…` or `doc-…`; none for a note of the old wiki whose
    /// page matched no object (kept apart, never lost).
    pub object_id: Option<String>,
    /// The title of the page of the old wiki it came from, if it did.
    pub title: Option<String>,
    pub origin_path: Option<String>,
    pub text: String,
    pub updated_at: i64,
}

/// The article of an object: its page as the projection last wrote it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeArticle {
    pub object_id: String,
    /// `entity`, `matter` or `document`.
    pub kind: String,
    /// Its logical path (`wiki/entities/…`): what the links between pages say.
    pub path: String,
    pub title: String,
    /// Markdown, with its header; without the notes of a person.
    pub body: String,
    /// `projected`, or `kept`: its object left, the page stays for its notes.
    pub state: String,
    pub written_at: i64,
    pub notes: Option<KnowledgeNote>,
}

/// The object a logical path or a link names.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageObject {
    /// `ent-…`, `mat-…`, `doc-…`, or `overview`.
    pub object_id: String,
    /// `entity`, `matter`, `document` or `overview`.
    pub kind: String,
    pub path: String,
    /// For a document: the document the page is named by, and one place of
    /// it, to open its screen.
    pub content_id: Option<i64>,
    pub location_id: Option<i64>,
}

/// The overview of the archive.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeOverview {
    pub path: String,
    pub body: String,
    pub written_at: i64,
}

/// A question of the chat a person saved, with its answer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedQuery {
    pub id: i64,
    pub title: String,
    pub question: String,
    pub answer: String,
    /// The pages or objects it cites (logical paths, link names or ids).
    pub refs: Vec<String>,
    pub created_at: i64,
}

/// What kind of object an id is.
pub fn kind_of(id: &str) -> Option<&'static str> {
    match id.split('-').next() {
        Some("ent") => Some("entity"),
        Some("mat") => Some("matter"),
        Some("doc") => Some("document"),
        _ => None,
    }
}

fn note_row(row: &rusqlite::Row) -> rusqlite::Result<KnowledgeNote> {
    Ok(KnowledgeNote {
        id: row.get(0)?,
        object_id: row.get(1)?,
        title: row.get(2)?,
        origin_path: row.get(3)?,
        text: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

const NOTE_COLUMNS: &str = "id, object_id, title, origin_path, text, updated_at";

fn note_of(conn: &Connection, object_id: &str) -> Result<Option<KnowledgeNote>, String> {
    conn.query_row(&format!("SELECT {NOTE_COLUMNS} FROM knowledge_note WHERE object_id = ?1"), [object_id], note_row)
        .optional()
        .map_err(sql_err)
}

/// A reference to a page as the logical path it may be: a logical path, an
/// absolute path of the old wiki's file, or a link (`name`, `name.md`),
/// without its anchor.
fn logical(reference: &str) -> String {
    let clean = reference.replace('\\', "/");
    let clean = clean.split('#').next().unwrap_or_default().trim().to_string();
    let lower = clean.to_lowercase();
    if lower.starts_with("wiki/") {
        return clean;
    }
    match lower.rfind("/wiki/") {
        Some(at) => clean[at + 1..].to_string(),
        None => clean,
    }
}

impl DocBase {
    /// The article of object `id` (`ent-…`, `mat-…` or `doc-…`) as the
    /// projection last wrote it in the base, with its notes; none while it
    /// has no page.
    pub fn knowledge_article(&self, id: &str) -> Result<Option<KnowledgeArticle>, String> {
        let Some(kind) = kind_of(id) else { return Ok(None) };
        let conn = self.conn();
        let row: Option<(String, String, String, i64)> = conn
            .query_row(
                "SELECT path, state, body, written_at FROM knowledge_page WHERE entity_id = ?1 AND body IS NOT NULL",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((path, state, body, written_at)) = row else { return Ok(None) };
        let title = title_of(&body).unwrap_or_else(|| stem(&path).replace('-', " "));
        Ok(Some(KnowledgeArticle {
            object_id: id.to_string(),
            kind: kind.to_string(),
            path,
            title,
            body,
            state,
            written_at,
            notes: note_of(&conn, id)?,
        }))
    }

    /// The article of the page of a document (the page of its family of
    /// versions, D-340).
    pub fn knowledge_document_article(&self, content_id: i64) -> Result<Option<KnowledgeArticle>, String> {
        let Some(path) = self.document_pages()?.remove(&content_id) else { return Ok(None) };
        let id: Option<String> = self
            .conn()
            .query_row("SELECT entity_id FROM knowledge_page WHERE path = ?1", [&path], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        match id {
            Some(id) => self.knowledge_article(&id),
            None => Ok(None),
        }
    }

    /// The object a page names: by its logical path, the absolute path of
    /// the old wiki's file, or a link (`[[name]]`), so that every link of
    /// an article opens the screen of what it names. The overview is one
    /// too.
    pub fn knowledge_object_of_page(&self, reference: &str) -> Result<Option<PageObject>, String> {
        let wanted = logical(reference);
        if wanted.is_empty() {
            return Ok(None);
        }
        let wanted_stem = stem(&wanted).to_lowercase();
        if wanted.eq_ignore_ascii_case(OVERVIEW_PATH) || wanted_stem == stem(OVERVIEW_PATH) {
            return Ok(Some(PageObject {
                object_id: "overview".into(),
                kind: "overview".into(),
                path: OVERVIEW_PATH.into(),
                content_id: None,
                location_id: None,
            }));
        }
        let conn = self.conn();
        let pages: Vec<(String, String)> = {
            let mut statement = conn.prepare("SELECT entity_id, path FROM knowledge_page ORDER BY path").map_err(sql_err)?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let found = pages
            .iter()
            .find(|(_, path)| path.eq_ignore_ascii_case(&wanted))
            .or_else(|| pages.iter().find(|(_, path)| stem(path).to_lowercase() == wanted_stem));
        let Some((id, path)) = found else { return Ok(None) };
        let Some(kind) = kind_of(id) else { return Ok(None) };
        let content_id = id.strip_prefix("doc-").and_then(|n| n.parse::<i64>().ok());
        let location_id = match content_id {
            Some(content) => conn
                .query_row(
                    "SELECT l.id FROM location l JOIN file f ON f.hash = l.file_hash WHERE f.content_id = ?1 AND l.kind = 'file'
                     ORDER BY l.id LIMIT 1",
                    [content],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_err)?,
            None => None,
        };
        Ok(Some(PageObject { object_id: id.clone(), kind: kind.into(), path: path.clone(), content_id, location_id }))
    }

    /// A page whole, as the old wiki's file had it (what the app wrote, then
    /// the notes of a person), with its title: for the search and the chat.
    pub fn knowledge_page_text(&self, path: &str) -> Result<Option<(String, String)>, String> {
        if path.eq_ignore_ascii_case(OVERVIEW_PATH) {
            return Ok(self.knowledge_overview()?.map(|o| (title_of(&o.body).unwrap_or_default(), o.body)));
        }
        let row: Option<(String, String)> = self
            .conn()
            .query_row("SELECT entity_id, path FROM knowledge_page WHERE path = ?1 COLLATE NOCASE", [path], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .map_err(sql_err)?;
        let Some((id, _)) = row else { return Ok(None) };
        Ok(self.knowledge_article(&id)?.map(|article| {
            let notes = article.notes.map(|n| n.text).unwrap_or_default();
            (article.title, with_notes(vocabulary(), &article.body, &notes))
        }))
    }

    /// The notes of a person on object `id`.
    pub fn knowledge_notes(&self, id: &str) -> Result<Option<KnowledgeNote>, String> {
        note_of(&self.conn(), id)
    }

    /// Keeps the notes of a person on object `id` (`ent-…`, `mat-…`,
    /// `doc-…`); empty, they go. A page kept only for its notes follows on
    /// the next projection.
    pub fn knowledge_set_notes(&self, id: &str, text: &str) -> Result<Option<KnowledgeNote>, String> {
        if kind_of(id).is_none() {
            return Err(format!("'{id}' no es una persona, organización, asunto ni documento"));
        }
        let conn = self.conn();
        let before = notes_of_object(&conn, id)?;
        if text.trim().is_empty() {
            conn.execute("DELETE FROM knowledge_note WHERE object_id = ?1", [id]).map_err(sql_err)?;
        } else if before != text {
            conn.execute(
                "INSERT INTO knowledge_note (object_id, text, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (object_id) DO UPDATE SET text = excluded.text, updated_at = excluded.updated_at",
                params![id, text, now_ms()],
            )
            .map_err(sql_err)?;
        }
        if before.trim().is_empty() != text.trim().is_empty() {
            super::mark_dirty(&conn, "project", id)?;
        }
        note_of(&conn, id)
    }

    /// The notes of the old wiki whose page matched no object, kept apart.
    pub fn knowledge_orphan_notes(&self) -> Result<Vec<KnowledgeNote>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!("SELECT {NOTE_COLUMNS} FROM knowledge_note WHERE object_id IS NULL ORDER BY title, id"))
            .map_err(sql_err)?;
        let rows = statement.query_map([], note_row).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// Removes a note by its number (a note kept apart, after a person read it).
    pub fn knowledge_delete_note(&self, note_id: i64) -> Result<(), String> {
        let conn = self.conn();
        let object: Option<Option<String>> =
            conn.query_row("SELECT object_id FROM knowledge_note WHERE id = ?1", [note_id], |row| row.get(0)).optional().map_err(sql_err)?;
        conn.execute("DELETE FROM knowledge_note WHERE id = ?1", [note_id]).map_err(sql_err)?;
        if let Some(Some(object)) = object {
            super::mark_dirty(&conn, "project", &object)?;
        }
        Ok(())
    }

    /// The overview of the archive, as the projection last wrote it.
    pub fn knowledge_overview(&self) -> Result<Option<KnowledgeOverview>, String> {
        self.conn()
            .query_row("SELECT path, body, written_at FROM knowledge_overview WHERE id = 1", [], |row| {
                Ok(KnowledgeOverview { path: row.get(0)?, body: row.get(1)?, written_at: row.get(2)? })
            })
            .optional()
            .map_err(sql_err)
    }

    /// The saved queries, the latest first.
    pub fn saved_queries(&self) -> Result<Vec<SavedQuery>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare("SELECT id, title, question, answer, refs, created_at FROM knowledge_query ORDER BY created_at DESC, id DESC")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok(SavedQuery {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    question: row.get(2)?,
                    answer: row.get(3)?,
                    refs: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
                    created_at: row.get(5)?,
                })
            })
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// Saves a question of the chat with its answer and what it cites.
    pub fn save_query(&self, title: &str, question: &str, answer: &str, refs: &[String]) -> Result<SavedQuery, String> {
        let title = title.trim();
        if title.is_empty() || answer.trim().is_empty() {
            return Err("Una consulta guardada necesita un título y una respuesta".into());
        }
        let created_at = now_ms();
        let conn = self.conn();
        conn.execute(
            "INSERT INTO knowledge_query (title, question, answer, refs, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![title, question, answer, serde_json::to_string(refs).map_err(|e| e.to_string())?, created_at],
        )
        .map_err(sql_err)?;
        Ok(SavedQuery {
            id: conn.last_insert_rowid(),
            title: title.to_string(),
            question: question.to_string(),
            answer: answer.to_string(),
            refs: refs.to_vec(),
            created_at,
        })
    }

    pub fn delete_saved_query(&self, id: i64) -> Result<(), String> {
        self.conn().execute("DELETE FROM knowledge_query WHERE id = ?1", [id]).map(|_| ()).map_err(sql_err)
    }
}
