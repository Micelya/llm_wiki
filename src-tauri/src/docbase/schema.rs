//! Tables of the document base and how they are brought up to date.
//!
//! The schema version is SQLite's `user_version`. Each entry of
//! `MIGRATIONS` takes the database one version forward; a database is
//! never edited in place outside of them.

use rusqlite::Connection;

use super::{now_ms, sql_err};

pub const CURRENT_VERSION: i64 = MIGRATIONS.len() as i64;

const MIGRATIONS: &[&str] = &[V1];

const V1: &str = "
-- The space this database belongs to: exactly one row.
CREATE TABLE space (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('private', 'shared')),
    created_at  INTEGER NOT NULL
);

-- A folder the user incorporated. `convention` is how the user orders
-- it: a JSON array with the name of each folder level, e.g.
-- [\"cliente\", \"caso\"]. `reachable` is NULL until the first sweep.
CREATE TABLE origin (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL UNIQUE,
    provider      TEXT NOT NULL,
    location      TEXT NOT NULL,
    convention    TEXT NOT NULL DEFAULT '[]',
    added_at      INTEGER NOT NULL,
    last_scan_at  INTEGER,
    reachable     INTEGER
);

-- One document. `text_hash` is the fingerprint of its normalized text,
-- when it has enough text to tell documents apart.
CREATE TABLE content (
    id             INTEGER PRIMARY KEY,
    state          TEXT NOT NULL,
    text_hash      TEXT,
    word_count     INTEGER,
    error          TEXT,
    first_seen_at  INTEGER NOT NULL,
    processed_at   INTEGER
);
CREATE UNIQUE INDEX content_by_text ON content (text_hash) WHERE text_hash IS NOT NULL;
CREATE INDEX content_by_state ON content (state);

-- An exact sequence of bytes, and the document it is.
CREATE TABLE file (
    hash        TEXT PRIMARY KEY,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    size        INTEGER NOT NULL
);
CREATE INDEX file_by_content ON file (content_id);

-- A file at a path inside an origin. `kind` is 'file', or 'link' for the
-- shortcuts a mounted cloud drive leaves in place of its native
-- documents: detected, never processed. `label_overrides` holds the
-- label values the user corrected by hand.
CREATE TABLE location (
    id               INTEGER PRIMARY KEY,
    origin_id        TEXT NOT NULL REFERENCES origin (id) ON DELETE CASCADE,
    rel_path         TEXT NOT NULL,
    folder           TEXT NOT NULL,
    name             TEXT NOT NULL,
    ext              TEXT NOT NULL,
    size             INTEGER NOT NULL,
    mtime_ms         INTEGER NOT NULL,
    kind             TEXT NOT NULL DEFAULT 'file',
    file_hash        TEXT,
    hash_state       TEXT NOT NULL DEFAULT 'pending',
    hash_error       TEXT,
    label_overrides  TEXT NOT NULL DEFAULT '{}',
    first_seen_at    INTEGER NOT NULL,
    last_seen_at     INTEGER NOT NULL,
    UNIQUE (origin_id, rel_path)
);
CREATE INDEX location_by_hash ON location (file_hash);
CREATE INDEX location_by_hash_state ON location (hash_state);

-- What was obtained from a document: its text today; summary, type and
-- fields later. Produced once per content.
CREATE TABLE derivative (
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    engine      TEXT NOT NULL,
    body        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (content_id, kind)
);

-- Word search over the text of each content; rowid is the content id.
CREATE VIRTUAL TABLE content_fts USING fts5 (
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

-- A wiki project. The general project covers every origin of the space;
-- the others are points of view over part of them. `members` is 'open'
-- until someone reserves the project.
CREATE TABLE project (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('general', 'view')),
    path        TEXT,
    members     TEXT NOT NULL DEFAULT 'open',
    created_at  INTEGER NOT NULL
);
CREATE UNIQUE INDEX one_general_project ON project (kind) WHERE kind = 'general';

CREATE TABLE project_origin (
    project_id  TEXT NOT NULL REFERENCES project (id) ON DELETE CASCADE,
    origin_id   TEXT NOT NULL REFERENCES origin (id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, origin_id)
);

-- What happened to the locations, oldest first: the answer to 'what
-- changed since…'. `kind` is added, modified, removed or moved.
CREATE TABLE change (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    at         INTEGER NOT NULL,
    kind       TEXT NOT NULL,
    origin_id  TEXT NOT NULL,
    rel_path   TEXT NOT NULL,
    file_hash  TEXT,
    detail     TEXT
);
";

pub fn migrate(conn: &mut Connection) -> Result<(), String> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(sql_err)?;
    if version > CURRENT_VERSION {
        return Err(format!(
            "The document base was written by a newer version of the app (schema {version}, this app knows {CURRENT_VERSION})"
        ));
    }
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        let tx = conn.transaction().map_err(sql_err)?;
        tx.execute_batch(migration).map_err(sql_err)?;
        tx.pragma_update(None, "user_version", index as i64 + 1)
            .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
    }
    let spaces: i64 = conn
        .query_row("SELECT count(*) FROM space", [], |row| row.get(0))
        .map_err(sql_err)?;
    if spaces == 0 {
        conn.execute(
            "INSERT INTO space (id, kind, created_at) VALUES (?1, 'private', ?2)",
            rusqlite::params![uuid::Uuid::new_v4().to_string(), now_ms()],
        )
        .map_err(sql_err)?;
    }
    Ok(())
}
