//! Stage 4 of the knowledge layer: what changes when a document is
//! deleted, replaced by a new version, read again or left out (D-205 to
//! D-208).
//!
//! The record is computed again from the mentions on every resolution
//! (`resolve.rs`), so what a document said goes away with its mentions:
//! its evidence goes, a claim with other evidence stays, an entity left
//! without mentions is retired (kept, with what a person decided about
//! it), and only the pages whose text changes are written again
//! (`pages.rs`). What this module adds is what must survive the change:
//!
//! - **What a person decided** names a document by its id
//!   (`cluster:<content>:…`, `evidence:<content>:…`). When the document
//!   goes away and another takes its place, the decision goes on to that
//!   one: the same text back (by its fingerprint), or the new version of
//!   the file at the same path (by the change log). A content id is never
//!   given twice (`identity.rs`), so an old decision never lands on an
//!   unrelated document.
//! - **The id of each entity**, and so the path of its page: the entity
//!   each name of a document had when its mentions went away (deleted,
//!   or read again) is a hint for the same name of the document that
//!   takes its place. An entity seen only in that document keeps its id
//!   and its page.
//! - **What is left out**: a document whose every location is excluded is
//!   not processed (`base-documental.md`): its mentions do not count, but
//!   its reading is kept, so taking the exclusion back costs nothing. A
//!   document joined as the same document with another one chosen as the
//!   wiki's source still counts (its text is real evidence); its pages
//!   name the chosen one (`pages.rs`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection};

use crate::docbase::{excluded_sql, now_ms, sql_err};

/// Where the set of documents left out was last seen (`docbase_setting`).
const LEFT_OUT_SETTING: &str = "knowledge-left-out";

/// SQL condition: the content `{column}` has a location that is not
/// excluded. A document with none is not processed by the layer.
pub(crate) fn active_content(column: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM file af JOIN location l ON l.file_hash = af.hash
                 WHERE af.version_id = {column} AND NOT {})",
        excluded_sql!()
    )
}

/// The contents read by the layer that are left out now: every location
/// they have is excluded.
pub(crate) fn left_out(conn: &Connection) -> Result<BTreeSet<i64>, String> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT k.version_id FROM knowledge_extraction k
             WHERE EXISTS (SELECT 1 FROM file f JOIN location l ON l.file_hash = f.hash WHERE f.version_id = k.version_id)
               AND NOT {}",
            active_content("k.version_id")
        ))
        .map_err(sql_err)?;
    let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// Whether the documents left out, or the documents a person joined,
/// changed since the last time this was asked; marks the resolution and
/// the pages to be done again when they did.
pub(crate) fn notice_left_out(conn: &Connection) -> Result<bool, String> {
    let now: Vec<String> = left_out(conn)?.into_iter().map(|c| c.to_string()).collect();
    // Documents a person joined (they go into one matter, and the page
    // names the one chosen) count as a change too.
    let joined: Vec<String> = {
        let mut statement = conn
            .prepare(
                "SELECT a || '-' || b || '-' || coalesce(chosen_relation, relation) || '-' || coalesce(wiki_source, '')
                 FROM similar_pair WHERE state = 'joined' ORDER BY a, b",
            )
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let now = format!("{}|{}", now.join(","), joined.join(","));
    let before: Option<String> = conn
        .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [LEFT_OUT_SETTING], |row| row.get(0))
        .ok();
    if before.as_deref().unwrap_or("|") == now {
        return Ok(false);
    }
    conn.execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![LEFT_OUT_SETTING, now])
        .map_err(sql_err)?;
    super::mark_dirty(conn, "resolve", "left-out")?;
    super::mark_dirty(conn, "project", "left-out")?;
    Ok(true)
}

/// The document that took the place of each one that went away: the same
/// text back (or the same bytes), or else the new version of a file of it at the path where
/// it was replaced (the latest one). Only contents that exist now.
pub(crate) fn successors(conn: &Connection) -> Result<HashMap<i64, i64>, String> {
    let gone: Vec<(i64, Option<String>)> = {
        let mut statement = conn
            .prepare("SELECT g.version_id, g.text_hash FROM knowledge_gone g WHERE NOT EXISTS (SELECT 1 FROM version c WHERE c.id = g.version_id)")
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut out = HashMap::new();
    for (content, text_hash) in gone {
        let same_text: Option<i64> = match &text_hash {
            Some(hash) => conn
                .query_row("SELECT id FROM version WHERE text_hash = ?1 AND id != ?2", params![hash, content], |row| row.get(0))
                .ok(),
            None => None,
        };
        // The same bytes back (a text too short for a fingerprint).
        let same_file = || -> Option<i64> {
            conn.query_row(
                "SELECT f.version_id FROM knowledge_gone_file g JOIN file f ON f.hash = g.hash
                 WHERE g.version_id = ?1 AND f.version_id != ?1 ORDER BY f.version_id DESC LIMIT 1",
                [content],
                |row| row.get(0),
            )
            .ok()
        };
        let next = match same_text.or_else(same_file) {
            Some(next) => Some(next),
            None => conn
                .query_row(
                    "SELECT f.version_id FROM knowledge_gone_file g
                     JOIN change ch ON ch.file_hash = g.hash AND ch.kind = 'modified'
                     JOIN location l ON l.origin_id = ch.origin_id AND l.rel_path = ch.rel_path
                     JOIN file f ON f.hash = l.file_hash
                     WHERE g.version_id = ?1 AND f.version_id != ?1
                     ORDER BY ch.id DESC LIMIT 1",
                    [content],
                    |row| row.get(0),
                )
                .ok(),
        };
        if let Some(next) = next {
            out.insert(content, next);
        }
    }
    Ok(out)
}

/// A key of a decision on the document that took the place of the one it
/// names, if that one went away.
pub(crate) fn translate(key: &str, successors: &HashMap<i64, i64>) -> String {
    if let Some(Ok(mut content)) = key.strip_prefix("content:").map(str::parse::<i64>) {
        for _ in 0..8 {
            match successors.get(&content) {
                Some(&next) => content = next,
                None => break,
            }
        }
        return format!("content:{content}");
    }
    for prefix in ["cluster:", "evidence:"] {
        let Some(rest) = key.strip_prefix(prefix) else { continue };
        let Some((content, tail)) = rest.split_once(':') else { continue };
        let Ok(mut content) = content.parse::<i64>() else { continue };
        let mut moved = false;
        // A chain (a version replaced by another) is followed, a few steps.
        for _ in 0..8 {
            match successors.get(&content) {
                Some(&next) => {
                    content = next;
                    moved = true;
                }
                None => break,
            }
        }
        if moved {
            return format!("{prefix}{content}:{tail}");
        }
    }
    key.to_string()
}

/// For each content, the contents it took the place of.
fn predecessors(successors: &HashMap<i64, i64>) -> BTreeMap<i64, Vec<i64>> {
    let mut out: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for (&gone, _) in successors {
        let mut target = gone;
        for _ in 0..8 {
            match successors.get(&target) {
                Some(&next) => target = next,
                None => break,
            }
        }
        out.entry(target).or_default().push(gone);
    }
    for list in out.values_mut() {
        list.sort_unstable_by(|a, b| b.cmp(a));
    }
    out
}

/// The entity each name of a document had before its mentions went away:
/// `(content, kind, normalized) -> entity`, for the contents that exist
/// now (their own earlier reading first, then the documents they took the
/// place of, the latest first).
pub(crate) fn entity_hints(conn: &Connection) -> Result<HashMap<(i64, String, String), String>, String> {
    let mut kept: HashMap<i64, Vec<(String, String, String)>> = HashMap::new();
    {
        let mut statement = conn.prepare("SELECT version_id, kind, normalized, entity_id FROM knowledge_gone_mention").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, kind, normalized, entity) = row.map_err(sql_err)?;
            kept.entry(content).or_default().push((kind, normalized, entity));
        }
    }
    if kept.is_empty() {
        return Ok(HashMap::new());
    }
    let successors = successors(conn)?;
    let before = predecessors(&successors);
    let mut out = HashMap::new();
    let existing: BTreeSet<i64> = {
        let mut statement = conn.prepare("SELECT id FROM version").map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    for &content in &existing {
        let mut sources = vec![content];
        sources.extend(before.get(&content).into_iter().flatten().copied());
        for source in sources {
            for (kind, normalized, entity) in kept.get(&source).into_iter().flatten() {
                out.entry((content, kind.clone(), normalized.clone())).or_insert_with(|| entity.clone());
            }
        }
    }
    Ok(out)
}

/// After a resolution wrote the record: the hints of the documents that
/// exist and were resolved are not needed any more. Those of documents
/// that went away are kept, for the day they come back.
pub(crate) fn forget_used_hints(conn: &Connection) -> Result<(), String> {
    conn.execute(
        &format!(
            "DELETE FROM knowledge_gone_mention WHERE version_id IN (
                 SELECT k.version_id FROM knowledge_extraction k WHERE k.state = 'done' AND {})",
            active_content("k.version_id")
        ),
        [],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Takes the mentions of the documents left out out of the record: no
/// entity, marked as left out; the entity they had is kept as a hint, for
/// when the exclusion is taken back.
pub(crate) fn leave_out(conn: &Connection, contents: &BTreeSet<i64>) -> Result<BTreeSet<String>, String> {
    let mut touched = BTreeSet::new();
    let now = now_ms();
    for content in contents {
        let mut statement = conn
            .prepare("SELECT kind, normalized, entity_id FROM knowledge_mention WHERE version_id = ?1 AND entity_id IS NOT NULL")
            .map_err(sql_err)?;
        let rows: Vec<(String, String, String)> = statement
            .query_map([content], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        for (kind, normalized, entity) in rows {
            conn.execute(
                "INSERT OR REPLACE INTO knowledge_gone_mention (version_id, kind, normalized, entity_id, gone_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![content, kind, normalized, entity, now],
            )
            .map_err(sql_err)?;
            touched.insert(entity);
        }
        conn.execute(
            "UPDATE knowledge_mention SET entity_id = NULL, resolved_by = 'excluido'
             WHERE version_id = ?1 AND (entity_id IS NOT NULL OR resolved_by IS NOT 'excluido')",
            [content],
        )
        .map_err(sql_err)?;
    }
    Ok(touched)
}
