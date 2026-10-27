//! Tables of the document base and how they are brought up to date.
//!
//! The schema version is SQLite's `user_version`. Each entry of
//! `MIGRATIONS` takes the database one version forward; a database is
//! never edited in place outside of them.
//!
//! The numbering starts again with this base's own file (D-548): the base
//! of the earlier numbering is never opened nor brought over. From its
//! first version on, every change of shape keeps the data, with its
//! migration and its backup (D-496, D-560).

use rusqlite::Connection;

use super::{now_ms, sql_err};

pub const CURRENT_VERSION: i64 = MIGRATIONS.len() as i64;

const MIGRATIONS: &[&str] = &[V1, V2];

/// The shape of the base at version 29 of the earlier numbering, in one
/// migration, generated from that base (`schema_v1.sql`). It seeds no row:
/// without the `background-reading` setting the automatic reading is off
/// until the person turns it on.
const V1: &str = include_str!("schema_v1.sql");

/// The text and the prints of each page of a file (D-560, D-534).
///
/// `resource_text`: the text of a file by page and by engine: `extraction`
/// for its text layer, a recognizer's engine for what it read. A file
/// without pages (a Word file, a plain text) has one row, page 1. The text
/// of a document (`derivative`, kind `text`) stays as it was: these rows
/// are its pages, not another text.
///
/// `resource_page`: the prints of each page of a PDF, from the page drawn
/// 256 pixels high in grays: `exact_hash` (SHA-256 of its pixels), `phash`
/// (64 bits, 16 hex digits) and `text_hash` (SHA-256 of its normalized
/// text, only from 40 characters on). `has_handwriting` and `has_signature`
/// are null while nothing can tell.
///
/// Both go with their file (`ON DELETE CASCADE`).
const V2: &str = "
CREATE TABLE resource_text (
    resource_hash  TEXT NOT NULL REFERENCES file (hash) ON DELETE CASCADE,
    page_no        INTEGER NOT NULL,
    engine         TEXT NOT NULL,
    body           TEXT NOT NULL,
    created_at     INTEGER NOT NULL,
    PRIMARY KEY (resource_hash, page_no, engine)
) WITHOUT ROWID;

CREATE TABLE resource_page (
    resource_hash    TEXT NOT NULL REFERENCES file (hash) ON DELETE CASCADE,
    page_no          INTEGER NOT NULL,
    exact_hash       TEXT NOT NULL,
    phash            TEXT NOT NULL,
    text_hash        TEXT,
    has_handwriting  INTEGER CHECK (has_handwriting IN (0, 1)),
    has_signature    INTEGER CHECK (has_signature IN (0, 1)),
    created_at       INTEGER NOT NULL,
    PRIMARY KEY (resource_hash, page_no)
) WITHOUT ROWID;
CREATE INDEX resource_page_by_exact ON resource_page (exact_hash);
CREATE INDEX resource_page_by_text ON resource_page (text_hash) WHERE text_hash IS NOT NULL;
";

/// The version of a base: 0 for one just created.
pub fn version(conn: &Connection) -> Result<i64, String> {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(sql_err)
}

/// Takes an empty base to `version`, as an app of that version left it: the
/// bases of the past, for the tests of an update.
#[cfg(test)]
pub(crate) fn build_at(conn: &mut Connection, version: i64) {
    for (index, migration) in MIGRATIONS.iter().enumerate().take(version as usize) {
        let tx = conn.transaction().unwrap();
        tx.execute_batch(migration).unwrap();
        tx.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        tx.commit().unwrap();
    }
}

pub fn migrate(conn: &mut Connection) -> Result<(), String> {
    let version = self::version(conn)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate(&mut conn).unwrap();
        conn
    }

    #[test]
    fn a_new_base_reads_nothing_by_itself() {
        let conn = fresh();
        assert_eq!(version(&conn).unwrap(), CURRENT_VERSION);
        let settings: i64 = conn.query_row("SELECT count(*) FROM docbase_setting", [], |row| row.get(0)).unwrap();
        assert_eq!(settings, 0);
        let spaces: i64 = conn.query_row("SELECT count(*) FROM space", [], |row| row.get(0)).unwrap();
        assert_eq!(spaces, 1);
        let dir = crate::docbase::testing::TempDir::new();
        let base = crate::docbase::DocBase::open(&dir.path().join(crate::docbase::PRIVATE_DB_FILE)).unwrap();
        assert!(!base.background_reading().unwrap());
    }

    #[test]
    fn the_pages_of_a_file_go_with_it() {
        let conn = fresh();
        conn.execute_batch(
            "INSERT INTO content (id, state, first_seen_at) VALUES (1, 'processed', 1);
             INSERT INTO file (hash, content_id, size) VALUES ('h1', 1, 10), ('h2', 1, 10);
             INSERT INTO resource_text (resource_hash, page_no, engine, body, created_at)
                VALUES ('h1', 1, 'extraction', 'uno', 1), ('h1', 2, 'extraction', 'dos', 1), ('h2', 1, 'extraction', 'uno', 1);
             INSERT INTO resource_page (resource_hash, page_no, exact_hash, phash, created_at)
                VALUES ('h1', 1, 'e1', '0000000000000000', 1), ('h2', 1, 'e1', '0000000000000000', 1);
             DELETE FROM file WHERE hash = 'h1';",
        )
        .unwrap();
        let left = |table: &str| -> i64 {
            conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap()
        };
        assert_eq!((left("resource_text"), left("resource_page")), (1, 1));
        let unknown = conn.execute(
            "INSERT INTO resource_page (resource_hash, page_no, exact_hash, phash, created_at) VALUES ('nada', 1, 'e', 'p', 1)",
            [],
        );
        assert!(unknown.is_err());
    }
}
