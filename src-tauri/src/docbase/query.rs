//! Read-only views of the base for the screens.

use std::collections::BTreeMap;

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::labels::{labels_of, Label};
use super::origins::{self, Origin};
use super::{sql_err, DocBase, Visible};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginSummary {
    #[serde(flatten)]
    pub origin: Origin,
    /// Files in the inventory, shortcuts included.
    pub files: i64,
    /// Files still waiting to be read.
    pub pending: i64,
    /// Files that could not be read.
    pub errors: i64,
    /// Cloud shortcuts: detected, not processable.
    pub links: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DocumentFilter {
    /// Words to look for in the name and path of the file and in the
    /// text of the document. Every word has to be found somewhere.
    pub query: Option<String>,
    pub origin_id: Option<String>,
    /// One of the states of `DocumentRow::state`.
    pub state: Option<String>,
    /// Only documents with more than one visible location.
    pub duplicates_only: bool,
    /// Only files carrying this label.
    pub label_id: Option<i64>,
    /// Only documents with a similar document that waits for a decision
    /// (a proposed relation other than plain `similar`, as Inicio counts).
    pub similar_pending: bool,
    /// Only the members of the family whose principal is this content.
    pub similar_group: Option<i64>,
    /// Only documents of this type (pass 4); `"none"`: without a type.
    pub doc_type: Option<String>,
    /// One row per document instead of one per location: of the
    /// locations holding the same document that match the filter, only
    /// the most recently modified is listed (its `copies` tells how many
    /// there are). `total` then counts documents.
    pub one_per_document: bool,
    /// `recent` (default), `name` or `folder`.
    pub sort: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

/// One location, with what is known of the document it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRow {
    pub location_id: i64,
    pub origin_id: String,
    pub origin_name: String,
    pub origin_reachable: Option<bool>,
    pub rel_path: String,
    pub folder: String,
    pub name: String,
    pub ext: String,
    pub size: i64,
    pub mtime_ms: i64,
    /// Where the document stands, from the inventory to its text:
    /// `link` (cloud shortcut: detected, not processable), `unread`
    /// (waiting to be read), `excluded` (left out by the user),
    /// `unreadable`, `too-large`, `unprocessed`
    /// (read, text not extracted yet), `processed`, `needs-recognition`
    /// (its text is only pixels), `unsupported` or `error`.
    pub state: String,
    pub content_id: Option<i64>,
    /// Visible locations holding this same document, this one included.
    pub copies: i64,
    pub labels: Vec<Label>,
    /// What the document is (pass 4): a type of the catalog, or one the
    /// model proposed (`doc_type_proposed`).
    pub doc_type: Option<String>,
    pub doc_stage: Option<String>,
    pub doc_type_proposed: bool,
    /// The name of a type outside the catalog: the one the model proposed,
    /// or the one a person gave it when confirming it.
    pub doc_type_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentPage {
    pub rows: Vec<DocumentRow>,
    /// Rows matching the filter, before `limit` and `offset`.
    pub total: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationRef {
    pub location_id: i64,
    pub origin_name: String,
    pub origin_reachable: Option<bool>,
    pub rel_path: String,
    /// Where the file is on disk.
    pub path: String,
    pub mtime_ms: i64,
    /// False when it is the same document in another file (another
    /// format, another scan of the same paper).
    pub same_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDetail {
    #[serde(flatten)]
    pub row: DocumentRow,
    /// Where the file is on disk.
    pub path: String,
    pub file_hash: Option<String>,
    /// Why the file or its text could not be read, when that is the case.
    pub error: Option<String>,
    pub word_count: Option<i64>,
    pub processed_at: Option<i64>,
    pub first_seen_at: i64,
    pub text: Option<String>,
    /// What produced the text: an extractor, or a recognition engine.
    pub text_engine: Option<String>,
    /// When that text was obtained.
    pub text_at: Option<i64>,
    /// Every visible location of this document, this one first.
    pub locations: Vec<LocationRef>,
    /// When it was generated from a template: when, from which and for
    /// which matter (D-488).
    pub generated: Option<super::similar::generate::GeneratedLine>,
}

/// How the archive is doing, in numbers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub origins: i64,
    pub unreachable_origins: i64,
    /// Files in the inventory, cloud shortcuts included.
    pub locations: i64,
    /// Distinct documents among the files already read.
    pub documents: i64,
    /// Locations by `DocumentRow::state`.
    pub by_state: BTreeMap<String, i64>,
    /// Locations whose document is also somewhere else.
    pub copies: i64,
    pub words: i64,
    pub last_scan_at: Option<i64>,
    /// Scanned pages of documents already read that their ficha of shape
    /// says need reading and were not read (P-320), and those documents.
    pub unread_scan_pages: i64,
    pub unread_scan_documents: Vec<i64>,
}

/// Something that happened to the files of an origin. Changes found by
/// the same sweep are told together: "183 files were added".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Activity {
    pub at: i64,
    /// added, modified, removed or moved.
    pub kind: String,
    pub origin_name: String,
    pub count: i64,
    /// One of the files, as an example; the only one when `count` is 1.
    pub rel_path: String,
    /// For a move, the path the file was at before.
    pub from_path: Option<String>,
}

const STATE_SQL: &str = concat!(
    "CASE WHEN ",
    super::excluded_sql!(),
    " THEN 'excluded'
        WHEN l.kind = 'link' THEN 'link'
        WHEN l.hash_state = 'pending' AND l.file_hash IS NULL THEN 'unread'
        WHEN l.hash_state = 'error' THEN 'unreadable'
        WHEN l.hash_state = 'too-large' THEN 'too-large'
        WHEN c.state = 'pending' THEN 'unprocessed'
        ELSE COALESCE(c.state, 'unread')
    END"
);

impl DocBase {
    pub fn origin_summaries(&self, visible: &Visible) -> Result<Vec<OriginSummary>, String> {
        let conn = self.conn();
        let origins = origins::list(&conn, visible)?;
        let (condition, bound) = visible.condition("origin_id");
        let mut statement = conn
            .prepare(&format!(
                "SELECT origin_id,
                        count(*),
                        sum(hash_state = 'pending'),
                        sum(hash_state = 'error'),
                        sum(kind = 'link')
                 FROM location WHERE {condition} GROUP BY origin_id"
            ))
            .map_err(sql_err)?;
        let counts = statement
            .query_map(params_from_iter(bound.iter()), |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;

        Ok(origins
            .into_iter()
            .map(|origin| {
                let (files, pending, errors, links) = counts
                    .iter()
                    .find(|(id, ..)| *id == origin.id)
                    .map(|(_, files, pending, errors, links)| (*files, *pending, *errors, *links))
                    .unwrap_or((0, 0, 0, 0));
                OriginSummary {
                    origin,
                    files,
                    pending,
                    errors,
                    links,
                }
            })
            .collect())
    }

    pub fn health(&self, visible: &Visible) -> Result<Health, String> {
        let conn = self.conn();
        let (sql, bound) = rows_sql(visible, None);
        let mut health = Health::default();

        let mut statement = conn
            .prepare(&format!(
                "SELECT state, count(*), sum(copies > 1) FROM ({sql}) GROUP BY state"
            ))
            .map_err(sql_err)?;
        let groups = statement
            .query_map(params_from_iter(bound.clone()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        for (state, count, copies) in groups {
            health.locations += count;
            health.copies += copies;
            health.by_state.insert(state, count);
        }

        let (documents, words): (i64, i64) = conn
            .query_row(
                &format!(
                    "SELECT count(*), coalesce(sum(word_count), 0) FROM version
                     WHERE id IN (SELECT version_id FROM ({sql}) WHERE version_id IS NOT NULL)"
                ),
                params_from_iter(bound.clone()),
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sql_err)?;
        health.documents = documents;
        health.words = words;
        let unread = super::page_shapes::unread_scans(&conn, &self.recognized, &sql, &bound)?;
        health.unread_scan_pages = unread.pages;
        health.unread_scan_documents = unread.documents;

        for origin in origins::list(&conn, visible)? {
            health.origins += 1;
            if origin.reachable == Some(false) {
                health.unreachable_origins += 1;
            }
            health.last_scan_at = health.last_scan_at.max(origin.last_scan_at);
        }
        Ok(health)
    }

    /// The latest things that happened to the files, newest first.
    pub fn activity(&self, visible: &Visible, limit: i64) -> Result<Vec<Activity>, String> {
        let conn = self.conn();
        let (condition, bound) = visible.condition("ch.origin_id");
        let mut bound: Vec<Value> = bound.into_iter().map(Value::Text).collect();
        bound.push(Value::Integer(limit.clamp(1, 200)));
        // A move also leaves an 'added' and a 'removed' behind: those are
        // the same event told twice, so they are left out.
        let mut statement = conn
            .prepare(&format!(
                "SELECT ch.at, ch.kind, o.name, count(*), min(ch.rel_path),
                        CASE WHEN ch.kind = 'moved' THEN min(ch.detail) END
                 FROM change ch JOIN origin o ON o.id = ch.origin_id
                 WHERE {condition}
                   AND NOT (ch.kind = 'removed' AND ch.detail IS NOT NULL)
                   AND NOT (ch.kind = 'added' AND EXISTS (
                        SELECT 1 FROM change m
                        WHERE m.kind = 'moved' AND m.origin_id = ch.origin_id AND m.rel_path = ch.rel_path))
                 GROUP BY ch.at, ch.kind, ch.origin_id,
                          CASE WHEN ch.kind = 'moved' THEN ch.id END
                 ORDER BY ch.at DESC, max(ch.id) DESC
                 LIMIT ?"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(params_from_iter(bound), |row| {
                let detail: Option<String> = row.get(5)?;
                let from_path = detail
                    .and_then(|detail| serde_json::from_str::<serde_json::Value>(&detail).ok())
                    .and_then(|from| from.get("relPath").and_then(|p| p.as_str()).map(str::to_string));
                Ok(Activity {
                    at: row.get(0)?,
                    kind: row.get(1)?,
                    origin_name: row.get(2)?,
                    count: row.get(3)?,
                    rel_path: row.get(4)?,
                    from_path,
                })
            })
            .map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)
    }

    /// The inventory, one row per location.
    pub fn documents(&self, visible: &Visible, filter: &DocumentFilter) -> Result<DocumentPage, String> {
        let conn = self.conn();
        let mut conditions = Vec::new();
        let mut bound: Vec<Value> = Vec::new();

        if let Some(origin_id) = filter.origin_id.as_deref().filter(|id| !id.is_empty()) {
            conditions.push("origin_id = ?".to_string());
            bound.push(Value::Text(origin_id.to_string()));
        }
        if let Some(state) = filter.state.as_deref().filter(|state| !state.is_empty()) {
            conditions.push("state = ?".to_string());
            bound.push(Value::Text(state.to_string()));
        }
        if filter.duplicates_only {
            conditions.push("copies > 1".to_string());
        }
        if let Some(label_id) = filter.label_id {
            conditions.push("location_id IN (SELECT location_id FROM location_label WHERE label_id = ?)".to_string());
            bound.push(Value::Integer(label_id));
        }
        if filter.similar_pending {
            // Only what waits for a person: a copy, a version… (D-497).
            let asks = super::similar::store::asks_person_sql("relation");
            conditions.push(format!(
                "version_id IN (SELECT a FROM similar_pair WHERE state = 'proposed' AND {asks}
                                UNION SELECT b FROM similar_pair WHERE state = 'proposed' AND {asks})"
            ));
        }
        match filter.doc_type.as_deref().filter(|t| !t.is_empty()) {
            Some("none") => conditions.push("doc_type IS NULL".to_string()),
            Some(doc_type) => {
                conditions.push("doc_type = ?".to_string());
                bound.push(Value::Text(doc_type.to_string()));
            }
            None => {}
        }
        if let Some(principal) = filter.similar_group {
            conditions.push("version_id IN (SELECT version_id FROM similar_group_member WHERE principal = ?)".to_string());
            bound.push(Value::Integer(principal));
        }
        for word in filter
            .query
            .as_deref()
            .unwrap_or("")
            .split_whitespace()
            .take(8)
        {
            // Each word has to be in the path or in the text.
            let like = Value::Text(format!("%{}%", escape_like(word)));
            match fts_prefix(word) {
                Some(prefix) => {
                    conditions.push(
                        "(rel_path LIKE ? ESCAPE '\\' OR version_id IN (SELECT rowid FROM content_fts WHERE content_fts MATCH ?))"
                            .to_string(),
                    );
                    bound.push(like);
                    bound.push(Value::Text(prefix));
                }
                None => {
                    conditions.push("rel_path LIKE ? ESCAPE '\\'".to_string());
                    bound.push(like);
                }
            }
        }
        let filter_sql = if conditions.is_empty() {
            "1 = 1".to_string()
        } else {
            conditions.join(" AND ")
        };
        let order = match filter.sort.as_deref() {
            Some("name") => "name COLLATE NOCASE, rel_path COLLATE NOCASE",
            Some("folder") => "origin_name COLLATE NOCASE, rel_path COLLATE NOCASE",
            _ => "mtime_ms DESC, rel_path COLLATE NOCASE",
        };

        let (base_sql, base_bound) = rows_sql(visible, None);
        let all_bound = |extra: &[Value]| -> Vec<Value> {
            base_bound
                .iter()
                .cloned()
                .chain(bound.iter().cloned())
                .chain(extra.iter().cloned())
                .collect()
        };

        // A location without a document yet (unread, a link) stands alone.
        let matching = if filter.one_per_document {
            format!(
                "SELECT * FROM (
                     SELECT *, row_number() OVER (
                         PARTITION BY coalesce(version_id, -location_id)
                         ORDER BY mtime_ms DESC, location_id
                     ) AS document_rank
                     FROM ({base_sql}) WHERE {filter_sql}
                 ) WHERE document_rank = 1"
            )
        } else {
            format!("SELECT * FROM ({base_sql}) WHERE {filter_sql}")
        };

        let total: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM ({matching})"),
                params_from_iter(all_bound(&[])),
                |row| row.get(0),
            )
            .map_err(sql_err)?;

        let limit = filter.limit.unwrap_or(200).clamp(1, 1000);
        let offset = filter.offset.unwrap_or(0).max(0);
        let rows = read_rows(
            &conn,
            &format!("SELECT * FROM ({matching}) ORDER BY {order} LIMIT ? OFFSET ?"),
            all_bound(&[Value::Integer(limit), Value::Integer(offset)]),
        )?;
        Ok(DocumentPage { rows, total })
    }

    /// Everything known about one location and the document it holds.
    /// `None` when it does not exist or is not visible.
    pub fn document(&self, visible: &Visible, location_id: i64) -> Result<Option<DocumentDetail>, String> {
        let conn = self.conn();
        let (sql, bound) = rows_sql(visible, Some(location_id));
        let Some(row) = read_rows(&conn, &sql, bound)?.into_iter().next() else {
            return Ok(None);
        };

        let (root, file_hash, hash_error, first_seen_at): (String, Option<String>, Option<String>, i64) = conn
            .query_row(
                "SELECT o.location, l.file_hash, l.hash_error, l.first_seen_at
                 FROM location l JOIN origin o ON o.id = l.origin_id WHERE l.id = ?1",
                [location_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .map_err(sql_err)?;

        let content: Option<(Option<i64>, Option<i64>, Option<String>)> = match row.content_id {
            Some(content_id) => conn
                .query_row(
                    "SELECT word_count, processed_at, error FROM version WHERE id = ?1",
                    [content_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(sql_err)?,
            None => None,
        };
        let (word_count, processed_at, content_error) = content.unwrap_or((None, None, None));

        let text: Option<(String, String, i64)> = match row.content_id {
            Some(content_id) => conn
                .query_row(
                    "SELECT body, engine, created_at FROM derivative WHERE version_id = ?1 AND kind = 'text'",
                    [content_id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
                .map_err(sql_err)?,
            None => None,
        };

        let locations = match row.content_id {
            Some(content_id) => locations_of(&conn, visible, content_id, location_id, file_hash.as_deref())?,
            None => Vec::new(),
        };
        let generated = match row.content_id {
            Some(content_id) => super::similar::generate::generated_line(&conn, content_id)?,
            None => None,
        };

        Ok(Some(DocumentDetail {
            path: absolute(&root, &row.rel_path),
            file_hash,
            error: hash_error.or(content_error),
            word_count,
            processed_at,
            first_seen_at,
            text: text.as_ref().map(|(body, ..)| body.clone()),
            text_engine: text.as_ref().map(|(_, engine, _)| engine.clone()),
            text_at: text.map(|(.., at)| at),
            locations,
            generated,
            row,
        }))
    }

    /// Every visible location of the document this location holds, this
    /// one first; just this one when it holds no document yet. Empty when
    /// it does not exist or is not visible.
    pub fn copies(&self, visible: &Visible, location_id: i64) -> Result<Vec<LocationRef>, String> {
        let conn = self.conn();
        let (condition, visible_bound) = visible.condition("l.origin_id");
        let bound: Vec<Value> = std::iter::once(Value::Integer(location_id))
            .chain(visible_bound.into_iter().map(Value::Text))
            .collect();
        let found: Option<(Option<i64>, Option<String>, String, String, Option<i64>, String, i64)> = conn
            .query_row(
                &format!(
                    "SELECT f.version_id, l.file_hash, o.name, o.location, o.reachable, l.rel_path, l.mtime_ms
                     FROM location l
                     JOIN origin o ON o.id = l.origin_id
                     LEFT JOIN file f ON f.hash = l.file_hash
                     WHERE l.id = ? AND {condition}"
                ),
                params_from_iter(bound),
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((content_id, file_hash, origin_name, root, reachable, rel_path, mtime_ms)) = found else {
            return Ok(Vec::new());
        };
        match content_id {
            Some(content_id) => locations_of(&conn, visible, content_id, location_id, file_hash.as_deref()),
            None => Ok(vec![LocationRef {
                location_id,
                origin_name,
                origin_reachable: reachable.map(|value| value != 0),
                path: absolute(&root, &rel_path),
                rel_path,
                mtime_ms,
                same_file: true,
            }]),
        }
    }

    /// Where a location is on disk, when it is visible.
    pub fn location_path(&self, visible: &Visible, location_id: i64) -> Result<Option<String>, String> {
        let (condition, bound) = visible.condition("l.origin_id");
        let bound: Vec<Value> = std::iter::once(Value::Integer(location_id))
            .chain(bound.into_iter().map(Value::Text))
            .collect();
        self.conn()
            .query_row(
                &format!(
                    "SELECT o.location, l.rel_path FROM location l JOIN origin o ON o.id = l.origin_id
                     WHERE l.id = ? AND {condition}"
                ),
                params_from_iter(bound),
                |row| Ok(absolute(&row.get::<_, String>(0)?, &row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(sql_err)
    }
}

/// The query behind every document row: one row per visible location,
/// or just `only` when given. Copies are counted among what is visible:
/// a copy the caller cannot see does not exist for them.
pub(crate) fn rows_sql(visible: &Visible, only: Option<i64>) -> (String, Vec<Value>) {
    let (visible_l, bound_l) = visible.condition("l.origin_id");
    let (visible_l2, bound_l2) = visible.condition("l2.origin_id");
    let mut bound: Vec<Value> = bound_l2.into_iter().map(Value::Text).collect();
    bound.extend(bound_l.into_iter().map(Value::Text));
    let only_sql = match only {
        Some(id) => {
            bound.push(Value::Integer(id));
            "AND l.id = ?"
        }
        None => "",
    };
    let sql = format!(
        "SELECT l.id AS location_id, l.origin_id AS origin_id, o.name AS origin_name,
                o.reachable AS origin_reachable, l.rel_path AS rel_path, l.folder AS folder,
                l.name AS name, l.ext AS ext, l.size AS size, l.mtime_ms AS mtime_ms,
                {STATE_SQL} AS state,
                f.version_id AS version_id,
                CASE WHEN f.version_id IS NULL THEN 1 ELSE (
                    SELECT count(*) FROM location l2 JOIN file f2 ON f2.hash = l2.file_hash
                    WHERE f2.version_id = f.version_id AND {visible_l2}
                ) END AS copies,
                dt.doc_type AS doc_type, dt.stage AS doc_stage, coalesce(dt.proposed, 0) AS doc_type_proposed,
                dt.proposed_name AS doc_type_name
         FROM location l
         JOIN origin o ON o.id = l.origin_id
         LEFT JOIN file f ON f.hash = l.file_hash
         LEFT JOIN version c ON c.id = f.version_id
         LEFT JOIN document_type dt ON dt.version_id = f.version_id
         WHERE {visible_l} {only_sql}"
    );
    (sql, bound)
}

/// Every visible location holding `content_id`, `first` at the top; each
/// says whether it is the same file as `file_hash` or another file with
/// the same document.
fn locations_of(
    conn: &Connection,
    visible: &Visible,
    content_id: i64,
    first: i64,
    file_hash: Option<&str>,
) -> Result<Vec<LocationRef>, String> {
    let (condition, visible_bound) = visible.condition("l.origin_id");
    let mut statement = conn
        .prepare(&format!(
            "SELECT l.id, o.name, o.reachable, l.rel_path, o.location, l.mtime_ms, l.file_hash
             FROM location l
             JOIN origin o ON o.id = l.origin_id
             JOIN file f ON f.hash = l.file_hash
             WHERE f.version_id = ? AND {condition}
             ORDER BY (l.id = ?) DESC, o.name COLLATE NOCASE, l.rel_path COLLATE NOCASE"
        ))
        .map_err(sql_err)?;
    let bound: Vec<Value> = std::iter::once(Value::Integer(content_id))
        .chain(visible_bound.into_iter().map(Value::Text))
        .chain(std::iter::once(Value::Integer(first)))
        .collect();
    let found = statement
        .query_map(params_from_iter(bound), |r| {
            let reachable: Option<i64> = r.get(2)?;
            let root: String = r.get(4)?;
            let rel_path: String = r.get(3)?;
            let hash: Option<String> = r.get(6)?;
            Ok(LocationRef {
                location_id: r.get(0)?,
                origin_name: r.get(1)?,
                origin_reachable: reachable.map(|value| value != 0),
                path: absolute(&root, &rel_path),
                rel_path,
                mtime_ms: r.get(5)?,
                same_file: hash.as_deref() == file_hash,
            })
        })
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    Ok(found)
}

pub(crate) fn read_rows(conn: &Connection, sql: &str, bound: Vec<Value>) -> Result<Vec<DocumentRow>, String> {
    let mut statement = conn.prepare(sql).map_err(sql_err)?;
    let rows = statement
        .query_map(params_from_iter(bound), |row| {
            let reachable: Option<i64> = row.get("origin_reachable")?;
            Ok(DocumentRow {
                location_id: row.get("location_id")?,
                origin_id: row.get("origin_id")?,
                origin_name: row.get("origin_name")?,
                origin_reachable: reachable.map(|value| value != 0),
                rel_path: row.get("rel_path")?,
                labels: Vec::new(),
                folder: row.get("folder")?,
                name: row.get("name")?,
                ext: row.get("ext")?,
                size: row.get("size")?,
                mtime_ms: row.get("mtime_ms")?,
                state: row.get("state")?,
                content_id: row.get("version_id")?,
                copies: row.get("copies")?,
                doc_type: row.get("doc_type")?,
                doc_stage: row.get("doc_stage")?,
                doc_type_proposed: row.get("doc_type_proposed")?,
                doc_type_name: row.get("doc_type_name")?,
            })
        })
        .map_err(sql_err)?;
    let mut rows = rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?;
    let ids: Vec<i64> = rows.iter().map(|row| row.location_id).collect();
    let mut labels = labels_of(conn, &ids)?;
    for row in &mut rows {
        row.labels = labels.remove(&row.location_id).unwrap_or_default();
    }
    Ok(rows)
}

/// A word as a full-text query matching the words that start with it,
/// or `None` when it has nothing a text index would hold. Quoting keeps
/// whatever the user typed from being read as query syntax.
fn fts_prefix(word: &str) -> Option<String> {
    let clean: String = word
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.is_empty() {
        None
    } else {
        Some(format!("\"{clean}\"*"))
    }
}

fn escape_like(word: &str) -> String {
    word.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn absolute(root: &str, rel_path: &str) -> String {
    let mut path = std::path::PathBuf::from(root);
    for part in rel_path.split('/').filter(|part| !part.is_empty()) {
        path.push(part);
    }
    path.to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    fn settle(base: &DocBase, origin_id: &str) {
        base.scan_origin(origin_id).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
    }

    #[test]
    fn an_origin_summary_counts_its_files_by_state() {
        let dir = TempDir::new();
        dir.write("o/a.pdf", b"a");
        dir.write("o/b.pdf", b"b");
        dir.write("o/notas.gdoc", b"{}");
        dir.folder("vacio");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.add_origin("Vacío", &dir.path().join("vacio"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        base.hash_pending(1).unwrap();

        let summaries = base.origin_summaries(&Visible::All).unwrap();
        let o = summaries.iter().find(|s| s.origin.name == "O").unwrap();
        assert_eq!((o.files, o.pending, o.errors, o.links), (3, 1, 0, 1));
        let empty = summaries.iter().find(|s| s.origin.name == "Vacío").unwrap();
        assert_eq!((empty.files, empty.origin.reachable), (0, None));
    }

    fn sample() -> (TempDir, DocBase, String, String) {
        let dir = TempDir::new();
        dir.write("casos/BOREAL/Reclamo/carta documento.pdf", b"carta");
        dir.write("casos/BOREAL/Reclamo/copia de la carta.pdf", b"carta");
        dir.write("casos/Andina/Contrato/contrato.docx", b"contrato");
        dir.write("casos/Andina/notas.gdoc", b"{}");
        dir.write("nube/carta escaneada.pdf", b"carta");
        let base = DocBase::in_memory().unwrap();
        let convention = vec!["cliente".to_string(), "caso".to_string()];
        let casos = base.add_origin("Casos", &dir.path().join("casos"), &convention).unwrap();
        let nube = base.add_origin("Nube", &dir.path().join("nube"), &[]).unwrap();
        settle(&base, &casos.id);
        settle(&base, &nube.id);
        (dir, base, casos.id, nube.id)
    }

    fn names(page: &DocumentPage) -> Vec<&str> {
        let mut names: Vec<&str> = page.rows.iter().map(|row| row.name.as_str()).collect();
        names.sort();
        names
    }

    #[test]
    fn the_inventory_lists_every_location_with_state_copies_and_labels() {
        let (_dir, base, _, _) = sample();
        let page = base.documents(&Visible::All, &DocumentFilter::default()).unwrap();
        assert_eq!(page.total, 5);

        let carta = page.rows.iter().find(|row| row.name == "carta documento.pdf").unwrap();
        assert_eq!(carta.state, "unprocessed");
        assert_eq!(carta.copies, 3);
        let named = |row: &DocumentRow| -> Vec<String> {
            row.labels
                .iter()
                .map(|label| format!("{}: {}", label.group.clone().unwrap_or_default(), label.name))
                .collect()
        };
        assert_eq!(named(carta), ["caso: Reclamo", "cliente: BOREAL"]);

        let link = page.rows.iter().find(|row| row.name == "notas.gdoc").unwrap();
        assert_eq!((link.state.as_str(), link.copies, link.content_id), ("link", 1, None));
        // Left at the client level: it has a client and no case.
        assert_eq!(named(link), ["cliente: Andina"]);
    }

    #[test]
    fn the_inventory_filters_by_words_origin_state_and_duplicates() {
        let (_dir, base, _, nube) = sample();
        let find = |filter: DocumentFilter| base.documents(&Visible::All, &filter).unwrap();

        let by_words = find(DocumentFilter { query: Some("carta boreal".into()), ..Default::default() });
        assert_eq!(names(&by_words), ["carta documento.pdf", "copia de la carta.pdf"]);

        let by_origin = find(DocumentFilter { origin_id: Some(nube), ..Default::default() });
        assert_eq!(names(&by_origin), ["carta escaneada.pdf"]);

        let by_state = find(DocumentFilter { state: Some("link".into()), ..Default::default() });
        assert_eq!(names(&by_state), ["notas.gdoc"]);

        let duplicates = find(DocumentFilter { duplicates_only: true, ..Default::default() });
        assert_eq!(duplicates.total, 3);

        let paged = find(DocumentFilter { sort: Some("name".into()), limit: Some(2), offset: Some(1), ..Default::default() });
        assert_eq!((paged.total, paged.rows.len()), (5, 2));
        assert_eq!(paged.rows[0].name, "carta escaneada.pdf");

        // A literal % is a character, not a wildcard.
        assert_eq!(find(DocumentFilter { query: Some("%".into()), ..Default::default() }).total, 0);
    }

    #[test]
    fn one_row_per_document_lists_each_document_once_with_its_copies() {
        let (_dir, base, casos, _) = sample();
        let one = |filter: DocumentFilter| {
            base.documents(&Visible::All, &DocumentFilter { one_per_document: true, ..filter }).unwrap()
        };

        // The letter is in three files: it is one document.
        let all = one(DocumentFilter::default());
        assert_eq!((all.total, all.rows.len()), (3, 3));
        let listed = names(&all);
        assert!(listed.contains(&"contrato.docx") && listed.contains(&"notas.gdoc"));
        let carta = all.rows.iter().find(|row| row.copies == 3).unwrap();

        // Its copies, the listed one first.
        let copies = base.copies(&Visible::All, carta.location_id).unwrap();
        assert_eq!(copies.len(), 3);
        assert_eq!(copies[0].location_id, carta.location_id);

        // A copy's own name still finds the document, shown by that copy.
        let found = one(DocumentFilter { query: Some("escaneada".into()), ..Default::default() });
        assert_eq!(names(&found), ["carta escaneada.pdf"]);
        assert_eq!(found.rows[0].copies, 3);

        // What the caller cannot see is neither listed nor counted.
        let visible = Visible::Origins(vec![casos]);
        let page = base.documents(&visible, &DocumentFilter { one_per_document: true, ..Default::default() }).unwrap();
        assert_eq!(page.total, 3);
        let carta = page.rows.iter().find(|row| row.copies == 2).unwrap();
        assert_eq!(base.copies(&visible, carta.location_id).unwrap().len(), 2);

        // A file without a document yet is its own row and its own copy.
        let link = all.rows.iter().find(|row| row.name == "notas.gdoc").unwrap();
        let alone = base.copies(&Visible::All, link.location_id).unwrap();
        assert_eq!((alone.len(), alone[0].location_id), (1, link.location_id));
        assert!(base.copies(&Visible::All, 9999).unwrap().is_empty());
    }

    #[test]
    fn copies_in_an_origin_the_caller_cannot_see_do_not_count() {
        let (_dir, base, casos, _) = sample();
        let visible = Visible::Origins(vec![casos]);
        let page = base.documents(&visible, &DocumentFilter::default()).unwrap();
        assert_eq!(page.total, 4);
        let carta = page.rows.iter().find(|row| row.name == "carta documento.pdf").unwrap();
        assert_eq!(carta.copies, 2);

        let detail = base.document(&visible, carta.location_id).unwrap().unwrap();
        assert_eq!(detail.locations.len(), 2);
    }

    #[test]
    fn the_detail_of_a_document_lists_all_its_locations_this_one_first() {
        let (dir, base, _, _) = sample();
        let page = base.documents(&Visible::All, &DocumentFilter::default()).unwrap();
        let copy = page.rows.iter().find(|row| row.name == "copia de la carta.pdf").unwrap();

        let detail = base.document(&Visible::All, copy.location_id).unwrap().unwrap();
        assert_eq!(detail.locations.len(), 3);
        assert_eq!(detail.locations[0].location_id, copy.location_id);
        assert!(detail.locations.iter().all(|location| location.same_file));
        assert_eq!(
            std::path::PathBuf::from(&detail.path),
            dir.path().join("casos").join("BOREAL").join("Reclamo").join("copia de la carta.pdf")
        );
        assert!(detail.file_hash.is_some());
        assert!(detail.text.is_none());

        assert!(base.document(&Visible::All, 9999).unwrap().is_none());
        assert!(base.document(&Visible::Origins(vec![]), copy.location_id).unwrap().is_none());
    }

    #[test]
    fn health_counts_locations_documents_states_and_copies() {
        let (dir, base, casos, _) = sample();
        let health = base.health(&Visible::All).unwrap();
        assert_eq!((health.origins, health.unreachable_origins), (2, 0));
        assert_eq!((health.locations, health.documents, health.copies), (5, 2, 3));
        assert_eq!(health.by_state.get("unprocessed"), Some(&4));
        assert_eq!(health.by_state.get("link"), Some(&1));
        assert!(health.last_scan_at.is_some());

        // What cannot be seen is not counted either.
        let partial = base.health(&Visible::Origins(vec![casos.clone()])).unwrap();
        assert_eq!((partial.origins, partial.locations, partial.copies), (1, 4, 2));

        dir.remove_dir("nube");
        let nube = base
            .origins(&Visible::All)
            .unwrap()
            .into_iter()
            .find(|origin| origin.name == "Nube")
            .unwrap();
        base.scan_origin(&nube.id).unwrap();
        let health = base.health(&Visible::All).unwrap();
        assert_eq!((health.unreachable_origins, health.locations), (1, 5));
    }

    #[test]
    fn activity_tells_each_sweep_together_and_a_move_once() {
        let (dir, base, casos, _) = sample();
        let first = base.activity(&Visible::All, 20).unwrap();
        // One line per origin: its first sweep.
        assert_eq!(first.len(), 2);
        let casos_line = first.iter().find(|a| a.origin_name == "Casos").unwrap();
        assert_eq!((casos_line.kind.as_str(), casos_line.count), ("added", 4));

        std::fs::create_dir_all(dir.path().join("casos/Andina/Firmados")).unwrap();
        std::fs::rename(
            dir.path().join("casos/Andina/Contrato/contrato.docx"),
            dir.path().join("casos/Andina/Firmados/contrato firmado.docx"),
        )
        .unwrap();
        settle(&base, &casos);

        let latest = base.activity(&Visible::All, 20).unwrap();
        assert_eq!(latest.len(), 3);
        assert_eq!(latest[0].kind, "moved");
        assert_eq!(latest[0].rel_path, "Andina/Firmados/contrato firmado.docx");
        assert_eq!(latest[0].from_path.as_deref(), Some("Andina/Contrato/contrato.docx"));
    }
}
