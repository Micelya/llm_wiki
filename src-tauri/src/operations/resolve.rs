//! From what the model named to what the base has (2.2: «el modelo nombra,
//! el programa resuelve»). A thing is named by the identifier the model
//! saw when it searched or read (`doc-41` a document, `ent-…` a record of
//! a person or an organization, `mat-…` a matter) or, without one, by its
//! name: one found is taken; several are shown for a person to choose
//! («¿Cuál?», up to five); none is «no lo encontré». What a card shows of
//! each comes from the base, never from what the model wrote.

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::docbase::knowledge::quote::words;
use crate::docbase::query::DocumentFilter;
use crate::docbase::{sql_err, DocBase, Visible};

use super::MAX_CANDIDATES;

/// A document as a card shows it: its name, where it is and how to open it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocLine {
    /// `doc-<content>`.
    pub key: String,
    pub content_id: i64,
    pub location_id: Option<i64>,
    pub name: String,
    /// «origen › carpeta».
    pub place: String,
    /// The date the document's text says, when the reading found one.
    pub date: Option<String>,
}

/// A matter as a card shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterLine {
    pub key: String,
    pub name: String,
    /// `active` or `retired`.
    pub state: String,
    /// `open`, `closed` or `archived`.
    pub status: String,
    /// `proposed` or `person`.
    pub origin: String,
    /// `proposed`, `confirmed` or `discarded`.
    pub confirmation: String,
    pub documents: usize,
}

/// A record of a person or an organization, as a card shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityLine {
    pub key: String,
    pub name: String,
    pub kind: String,
    pub state: String,
    pub merged_into: Option<String>,
}

/// What a name found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found<T> {
    One(T),
    /// Several: a person chooses (up to five).
    Many(Vec<T>),
    None,
}

/// The words that say «the documents attached to the message».
pub fn names_the_attached(named: &str) -> bool {
    let wanted = words(named);
    let joined = wanted.join(" ");
    matches!(
        joined.as_str(),
        "este documento" | "estos documentos" | "este" | "estos" | "esta" | "estas" | "el adjunto" | "los adjuntos" | "adjunto" | "adjuntos"
            | "el documento adjunto" | "los documentos adjuntos" | "este archivo" | "estos archivos"
    )
}

/// One document of the base by its content, with the first place of it.
pub fn document_line(conn: &Connection, content_id: i64) -> Result<Option<DocLine>, String> {
    let found: Option<(i64, String, String, String)> = conn
        .query_row(
            "SELECT l.id, l.name, l.folder, o.name FROM location l JOIN file f ON f.hash = l.file_hash JOIN origin o ON o.id = l.origin_id
             WHERE f.version_id = ?1 ORDER BY l.rel_path COLLATE NOCASE LIMIT 1",
            [content_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let exists: bool = found.is_some()
        || conn.query_row("SELECT 1 FROM version WHERE id = ?1", [content_id], |_| Ok(())).optional().map_err(sql_err)?.is_some();
    if !exists {
        return Ok(None);
    }
    let date: Option<String> = conn
        .query_row("SELECT doc_date FROM knowledge_extraction WHERE version_id = ?1", [content_id], |row| row.get(0))
        .optional()
        .map_err(sql_err)?
        .flatten();
    Ok(Some(match found {
        Some((location_id, name, folder, origin)) => DocLine {
            key: format!("doc-{content_id}"),
            content_id,
            location_id: Some(location_id),
            name,
            place: if folder.is_empty() { origin } else { format!("{origin} › {}", folder.replace('/', " › ")) },
            date,
        },
        None => DocLine { key: format!("doc-{content_id}"), content_id, location_id: None, name: format!("doc-{content_id}"), place: String::new(), date },
    }))
}

/// The content of a location.
fn content_of_location(conn: &Connection, location_id: i64) -> Result<Option<i64>, String> {
    conn.query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.id = ?1", [location_id], |row| row.get(0))
        .optional()
        .map_err(sql_err)
        .map(Option::flatten)
}

/// The content a document's identifier names: `doc-N`, `content:N`,
/// `loc-N` or `location:N`.
pub fn content_of_key(conn: &Connection, key: &str) -> Result<Option<i64>, String> {
    let key = key.trim();
    let number = |prefix: &str| key.strip_prefix(prefix).and_then(|n| n.trim().parse::<i64>().ok());
    if let Some(id) = number("doc-").or_else(|| number("content:")) {
        return Ok(Some(id));
    }
    if let Some(id) = number("loc-").or_else(|| number("location:")) {
        return content_of_location(conn, id);
    }
    Ok(None)
}

fn looks_like_document_key(key: &str) -> bool {
    let key = key.trim();
    ["doc-", "content:", "loc-", "location:"].iter().any(|p| key.strip_prefix(p).is_some_and(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit())))
}

/// A document the model named. `attached`: the documents attached to the
/// message, by the id of their location.
pub fn find_document(base: &DocBase, named: &str, attached: &[i64]) -> Result<Found<DocLine>, String> {
    let named = named.trim();
    if names_the_attached(named) {
        let conn = base.conn();
        let mut lines = Vec::new();
        for location in attached {
            if let Some(content) = content_of_location(&conn, *location)? {
                if let Some(mut line) = document_line(&conn, content)? {
                    line.location_id = Some(*location);
                    if !lines.iter().any(|l: &DocLine| l.content_id == content) {
                        lines.push(line);
                    }
                }
            }
        }
        return Ok(match lines.len() {
            0 => Found::None,
            1 => Found::One(lines.remove(0)),
            _ => Found::Many(lines),
        });
    }
    if looks_like_document_key(named) {
        let conn = base.conn();
        let Some(content) = content_of_key(&conn, named)? else { return Ok(Found::None) };
        return Ok(match document_line(&conn, content)? {
            Some(line) => Found::One(line),
            None => Found::None,
        });
    }
    let wanted = words(named);
    if wanted.is_empty() {
        return Ok(Found::None);
    }
    let page = base.documents(
        &Visible::All,
        &DocumentFilter { query: Some(named.to_string()), one_per_document: true, limit: Some(50), sort: Some("name".into()), ..Default::default() },
    )?;
    // By its name first: the words of the name are all in the file's name.
    // A document found only by its text is never taken alone: a person
    // chooses it.
    let mut by_name = Vec::new();
    let mut by_text = Vec::new();
    {
        let conn = base.conn();
        for row in page.rows {
            let Some(content) = row.content_id else { continue };
            let Some(mut line) = document_line(&conn, content)? else { continue };
            line.location_id = Some(row.location_id);
            line.name = row.name.clone();
            let place = if row.folder.is_empty() { row.origin_name.clone() } else { format!("{} › {}", row.origin_name, row.folder.replace('/', " › ")) };
            line.place = place;
            let name_words = words(&row.name);
            let exact = name_words == wanted || words(stem(&row.name)) == wanted;
            let all = wanted.iter().all(|w| name_words.iter().any(|n| n.starts_with(w.as_str())));
            if exact {
                by_name.insert(0, line);
            } else if all {
                by_name.push(line);
            } else {
                by_text.push(line);
            }
        }
    }
    if let Some(first) = by_name.first() {
        let exact_count = by_name.iter().filter(|l| words(&l.name) == wanted || words(stem(&l.name)) == wanted).count();
        if by_name.len() == 1 || exact_count == 1 && words(&first.name) == wanted {
            return Ok(Found::One(by_name.remove(0)));
        }
        by_name.truncate(MAX_CANDIDATES);
        return Ok(Found::Many(by_name));
    }
    if by_text.is_empty() {
        return Ok(Found::None);
    }
    by_text.truncate(MAX_CANDIDATES);
    Ok(Found::Many(by_text))
}

fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(dot) if dot > 0 => &name[..dot],
        _ => name,
    }
}

/// One matter by its id, as it is now (active or not).
pub fn matter_line(conn: &Connection, id: &str) -> Result<Option<MatterLine>, String> {
    conn.query_row(
        "SELECT m.id, m.name, m.state, m.status, m.origin, m.confirmation,
                (SELECT count(*) FROM knowledge_matter_document d WHERE d.matter_id = m.id)
         FROM knowledge_matter m WHERE m.id = ?1",
        [id.trim()],
        |row| {
            Ok(MatterLine {
                key: row.get(0)?,
                name: row.get(1)?,
                state: row.get(2)?,
                status: row.get(3)?,
                origin: row.get(4)?,
                confirmation: row.get(5)?,
                documents: row.get::<_, i64>(6)? as usize,
            })
        },
    )
    .optional()
    .map_err(sql_err)
}

/// A matter the model named: by its id, or by the words of its name.
pub fn find_matter(base: &DocBase, named: &str) -> Result<Found<MatterLine>, String> {
    let named = named.trim();
    if named.starts_with("mat-") {
        let conn = base.conn();
        return Ok(match matter_line(&conn, named)? {
            Some(line) if line.state == "active" => Found::One(line),
            _ => Found::None,
        });
    }
    let wanted = words(named);
    if wanted.is_empty() {
        return Ok(Found::None);
    }
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT id FROM knowledge_matter WHERE state = 'active' ORDER BY name").map_err(sql_err)?;
    let ids: Vec<String> = statement.query_map([], |row| row.get(0)).map_err(sql_err)?.collect::<Result<_, _>>().map_err(sql_err)?;
    drop(statement);
    let mut exact = Vec::new();
    let mut close = Vec::new();
    for id in ids {
        let Some(line) = matter_line(&conn, &id)? else { continue };
        let name_words = words(&line.name);
        if name_words == wanted {
            exact.push(line);
        } else if wanted.iter().all(|w| name_words.iter().any(|n| n.starts_with(w.as_str()))) {
            close.push(line);
        }
    }
    Ok(pick(exact, close))
}

fn pick<T>(mut exact: Vec<T>, mut close: Vec<T>) -> Found<T> {
    if exact.len() == 1 {
        return Found::One(exact.remove(0));
    }
    if exact.len() > 1 {
        exact.truncate(MAX_CANDIDATES);
        return Found::Many(exact);
    }
    match close.len() {
        0 => Found::None,
        1 => Found::One(close.remove(0)),
        _ => {
            close.truncate(MAX_CANDIDATES);
            Found::Many(close)
        }
    }
}

/// One record by its id, as it is now.
pub fn entity_line(conn: &Connection, id: &str) -> Result<Option<EntityLine>, String> {
    conn.query_row("SELECT id, name, kind, state, merged_into FROM knowledge_entity WHERE id = ?1", [id.trim()], |row| {
        Ok(EntityLine { key: row.get(0)?, name: row.get(1)?, kind: row.get(2)?, state: row.get(3)?, merged_into: row.get(4)? })
    })
    .optional()
    .map_err(sql_err)
}

/// A record the model named: by its id (a merged one goes to the one it
/// went into), or by its name or one of its ways of writing it.
pub fn find_entity(base: &DocBase, named: &str, kind: Option<&str>) -> Result<Found<EntityLine>, String> {
    let named = named.trim();
    if named.starts_with("ent-") {
        let conn = base.conn();
        let mut current = entity_line(&conn, named)?;
        let mut hops = 0;
        while let Some(line) = current.clone() {
            match (&line.state[..], &line.merged_into) {
                ("active", _) => break,
                (_, Some(next)) if hops < 8 => {
                    current = entity_line(&conn, next)?;
                    hops += 1;
                }
                _ => {
                    current = None;
                }
            }
        }
        return Ok(match current {
            Some(line) if line.state == "active" && kind.is_none_or(|k| k == line.kind) => Found::One(line),
            _ => Found::None,
        });
    }
    let wanted = words(named);
    if wanted.is_empty() {
        return Ok(Found::None);
    }
    let found = base.knowledge_entities(&Visible::All, kind, Some(named))?;
    let mut exact = Vec::new();
    let mut close = Vec::new();
    for summary in found {
        let line = EntityLine { key: summary.id.clone(), name: summary.name.clone(), kind: summary.kind.clone(), state: "active".into(), merged_into: None };
        if words(&summary.name) == wanted {
            exact.push(line);
        } else {
            close.push(line);
        }
    }
    Ok(pick(exact, close))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_attached_are_named_with_this_or_these() {
        assert!(names_the_attached("este documento"));
        assert!(names_the_attached("Estos"));
        assert!(names_the_attached("los adjuntos"));
        assert!(!names_the_attached("el contrato"));
    }

    #[test]
    fn a_document_key_is_doc_or_loc_and_a_number() {
        assert!(looks_like_document_key("doc-41"));
        assert!(looks_like_document_key("location:7"));
        assert!(!looks_like_document_key("doc-carta"));
        assert!(!looks_like_document_key("carta documento"));
    }
}
