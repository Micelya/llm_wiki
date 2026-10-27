//! The document base: what files exist in a space, where each copy is,
//! and which copies are the same document.
//!
//! One database per space. It keeps three levels apart:
//!
//! - a *location* is a file at a path inside an origin: what appears,
//!   changes, moves or disappears;
//! - a *file* is an exact sequence of bytes, named by its SHA-256;
//! - a *content* is one document. Several files can be the same content
//!   (the same text saved in two formats), and everything derived from a
//!   document (its text, later its summary and type) hangs from the
//!   content, so it is produced once per space however many copies exist.
//!
//! Origins are read-only: nothing here writes next to the originals. The
//! database lives in the app's own local data folder.
//!
//! Design: `docs/design/base-documental.md`.

pub mod auto_questions;
pub mod bridge;
pub mod chat_files;
pub mod commands;
#[cfg(test)]
mod field_check;
mod identity;
pub mod graph;
pub mod knowledge;
pub mod labels;
pub mod origins;
mod page_shapes;
pub mod progress;
pub mod query;
pub mod questions;
pub mod recognize;
pub mod search;
pub mod services;
mod scan;
mod schema;
pub mod similar;
#[cfg(test)]
pub(crate) mod testing;
pub mod text;
pub mod tree;
pub mod upgrade;
pub mod work_origin;
mod worker;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;

/// SQL condition: the location `l` was left out by the user, itself or
/// through a folder above it.
macro_rules! excluded_sql {
    () => {
        "EXISTS (SELECT 1 FROM exclusion e WHERE e.origin_id = l.origin_id AND (
            (e.is_folder = 0 AND e.rel_path = l.rel_path)
            OR (e.is_folder = 1 AND (e.rel_path = '' OR substr(l.rel_path, 1, length(e.rel_path) + 1) = e.rel_path || '/'))))"
    };
}
pub(crate) use excluded_sql;

/// File name of the private space's database inside the app data folder.
/// A new file (D-548): the base of the earlier numbering, which was
/// `docbase/private.sqlite3`, is never opened nor brought over.
pub const PRIVATE_DB_FILE: &str = "docbase/base.sqlite3";

/// The version of the base's shape this app writes (`schema.rs`).
pub const SCHEMA_VERSION: i64 = schema::CURRENT_VERSION;

/// What the caller is allowed to see. Every query takes one, so the
/// shared space can narrow it later without touching the queries; in the
/// private space it is always `All`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visible {
    All,
    Origins(Vec<String>),
}

impl Visible {
    /// SQL condition on an origin id column, with its parameters.
    pub(crate) fn condition(&self, column: &str) -> (String, Vec<String>) {
        match self {
            Visible::All => ("1 = 1".to_string(), Vec::new()),
            Visible::Origins(ids) if ids.is_empty() => ("1 = 0".to_string(), Vec::new()),
            Visible::Origins(ids) => {
                let marks = vec!["?"; ids.len()].join(", ");
                (format!("{column} IN ({marks})"), ids.clone())
            }
        }
    }
}

/// The database of one space.
pub struct DocBase {
    conn: Mutex<Connection>,
    /// What the engine is doing now and the last error of the model (in memory).
    engine: progress::EngineState,
    /// Where the text recognized from scans is kept page by page, beside
    /// the database (`recognize.rs`).
    recognized: PathBuf,
}

impl std::fmt::Debug for DocBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DocBase")
    }
}

impl DocBase {
    /// Opens (creating it if needed) the database at `path` and brings
    /// its schema up to date, without backups: for the tests and the checks
    /// over copies of a base. The app opens its own with
    /// `open_in_data_folder`.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn open(path: &Path) -> Result<Self, String> {
        let conn = Self::connect(path)?;
        Self::prepare(conn, Self::recognized_beside(path))
    }

    /// The base of the app, in its data folder (`PRIVATE_DB_FILE`). Before
    /// the first change of its shape it keeps a backup of the data, and
    /// without one it changes nothing (`upgrade.rs`, D-496). Says what it
    /// found and did, also when the base could not be opened.
    pub fn open_in_data_folder(data_dir: &Path) -> (Result<Self, String>, upgrade::Opening) {
        let mut opening = upgrade::Opening::default();
        let base = Self::open_keeping_backups(data_dir, &mut opening);
        if let Err(problem) = &base {
            opening.problem = Some(problem.clone());
        }
        (base, opening)
    }

    fn open_keeping_backups(data_dir: &Path, opening: &mut upgrade::Opening) -> Result<Self, String> {
        let path = data_dir.join(PRIVATE_DB_FILE);
        let conn = Self::connect(&path)?;
        let found = schema::version(&conn)?;
        let current = schema::CURRENT_VERSION;
        opening.found_version = Some(found);
        if found > current {
            opening.newer = true;
            return Err(upgrade::newer_message(found, current));
        }
        if found > 0 && found < current {
            let name = upgrade::take(&conn, data_dir, found, current)
                .map_err(|cause| upgrade::no_backup_message(found, current, &cause))?;
            eprintln!("[docbase] backup before taking the base from version {found} to {current}: {name}");
            opening.backup = Some(name);
        }
        let base = Self::prepare(conn, Self::recognized_beside(&path)).map_err(|cause| match &opening.backup {
            Some(name) => upgrade::update_failed_message(found, current, name, &cause),
            None => cause,
        })?;
        if let Err(error) = upgrade::note_app_version(&base.conn()) {
            eprintln!("[docbase] the version of the app could not be noted in the base: {error}");
        }
        if opening.backup.is_some() {
            // Only after an update that went well.
            opening.pruned = upgrade::prune(data_dir, upgrade::KEEP);
        }
        Ok(base)
    }

    fn connect(path: &Path) -> Result<Connection, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create '{}': {e}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .map_err(|e| format!("Failed to open the document base '{}': {e}", path.display()))?;
        // WAL lets the screens read while the background worker writes.
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| format!("Failed to set the journal mode: {e}"))?;
        Ok(conn)
    }

    fn recognized_beside(path: &Path) -> PathBuf {
        path.parent().unwrap_or(Path::new(".")).join(recognize::RECOGNIZED_DIR)
    }

    /// The version of the base's shape.
    pub fn schema_version(&self) -> Result<i64, String> {
        schema::version(&self.conn())
    }

    /// A database that lives only in memory, for tests.
    #[cfg(test)]
    pub fn in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open an in-memory document base: {e}"))?;
        let recognized = std::env::temp_dir().join(format!("micelya-recognized-{}", uuid::Uuid::new_v4()));
        Self::prepare(conn, recognized)
    }

    fn prepare(mut conn: Connection, recognized: PathBuf) -> Result<Self, String> {
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| format!("Failed to enable foreign keys: {e}"))?;
        schema::migrate(&mut conn)?;
        search::fill_missing(&conn)?;
        let base = Self {
            conn: Mutex::new(conn),
            engine: progress::EngineState::default(),
            recognized,
        };
        labels::adopt_conventions(&base)?;
        Ok(base)
    }

    /// The connection. Hold it only for the statements themselves: reading
    /// or hashing a file must happen with the lock released.
    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        // A panic while holding the lock leaves the connection usable:
        // SQLite rolls back whatever transaction was open.
        self.conn.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

pub(crate) fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

pub(crate) fn sql_err(e: rusqlite::Error) -> String {
    format!("Document base error: {e}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_base_has_its_private_space() {
        let base = DocBase::in_memory().unwrap();
        let kind: String = base
            .conn()
            .query_row("SELECT kind FROM space", [], |row| row.get(0))
            .unwrap();
        assert_eq!(kind, "private");
    }

    #[test]
    fn reopening_keeps_the_data_and_the_schema_version() {
        let dir = std::env::temp_dir().join(format!("micelya-docbase-{}", uuid::Uuid::new_v4()));
        let path = dir.join(PRIVATE_DB_FILE);
        {
            let base = DocBase::open(&path).unwrap();
            base.conn()
                .execute(
                    "INSERT INTO origin (id, name, provider, location, added_at)
                     VALUES ('o1', 'Casos', 'local-folder', 'C:/casos', 1)",
                    [],
                )
                .unwrap();
        }
        let base = DocBase::open(&path).unwrap();
        let (origins, version): (i64, i64) = {
            let conn = base.conn();
            (
                conn.query_row("SELECT count(*) FROM origin", [], |row| row.get(0))
                    .unwrap(),
                conn.query_row("PRAGMA user_version", [], |row| row.get(0))
                    .unwrap(),
            )
        };
        assert_eq!(origins, 1);
        assert_eq!(version, schema::CURRENT_VERSION);
        drop(base);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn word_search_ignores_accents_and_case() {
        let base = DocBase::in_memory().unwrap();
        let conn = base.conn();
        conn.execute(
            "INSERT INTO content_fts (rowid, body) VALUES (7, 'Carta documento por la RESCISIÓN del contrato')",
            [],
        )
        .unwrap();
        let hit: i64 = conn
            .query_row(
                "SELECT rowid FROM content_fts WHERE content_fts MATCH 'rescision'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(hit, 7);
    }

    #[test]
    fn a_space_has_at_most_one_general_project() {
        let base = DocBase::in_memory().unwrap();
        let conn = base.conn();
        let insert = "INSERT INTO project (id, name, kind, created_at) VALUES (?1, ?2, 'general', 1)";
        conn.execute(insert, ["p1", "General"]).unwrap();
        assert!(conn.execute(insert, ["p2", "Otro general"]).is_err());
    }

    #[test]
    fn visibility_becomes_a_condition_on_the_origin() {
        assert_eq!(Visible::All.condition("l.origin_id").0, "1 = 1");
        assert_eq!(Visible::Origins(vec![]).condition("l.origin_id").0, "1 = 0");
        let (sql, params) =
            Visible::Origins(vec!["a".into(), "b".into()]).condition("l.origin_id");
        assert_eq!(sql, "l.origin_id IN (?, ?)");
        assert_eq!(params, vec!["a".to_string(), "b".to_string()]);
    }
}
