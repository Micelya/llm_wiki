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

const MIGRATIONS: &[&str] = &[V1, V2, V3, V4];

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

/// Document, version and appearance (D-551, D-554).
///
/// `content` is now `version`, and every `content_id` is `version_id`
/// (SQLite carries the change to the indexes, the triggers and the foreign
/// keys); the numbers stay. `content_fts` and `content_search` keep their
/// names, their `rowid` the number of the version.
///
/// `document`: what stays the same across its versions; `uid` (a UUID of
/// version 7, from the moment the version was first seen) is its stable
/// identifier. `prevailing_version_id` is one of its versions, and
/// `prevailing_reason` says why (`only` while it has one). Each version
/// gets its own document, born with its number (version numbers are never
/// reused): joining versions in one document comes later.
///
/// `appearance`: a presence of a version in a file (`resource_hash`): the
/// whole file (`selector` `file`, nobody cut it, `confirmed`) or its pages
/// `pages:<from>-<to>`, with its `position` in the file, who cut it
/// (`rule`, `model`, `person`) and whether the cut is `proposed` or
/// `confirmed` (D-556). The Word and its PDF with the same text are two
/// appearances of one version.
///
/// They keep themselves: a new version gets its document, a new file its
/// whole-file appearance; a file moved to its twin by the text takes its
/// appearance along; a version that goes takes its document; a file that
/// goes takes its appearances.
const V3: &str = "
ALTER TABLE content RENAME TO version;
ALTER TABLE file RENAME COLUMN content_id TO version_id;
ALTER TABLE derivative RENAME COLUMN content_id TO version_id;
ALTER TABLE similar_signature RENAME COLUMN content_id TO version_id;
ALTER TABLE similar_band RENAME COLUMN content_id TO version_id;
ALTER TABLE document_marks RENAME COLUMN content_id TO version_id;
ALTER TABLE similar_dirty RENAME COLUMN content_id TO version_id;
ALTER TABLE similar_group_member RENAME COLUMN content_id TO version_id;
ALTER TABLE document_type RENAME COLUMN content_id TO version_id;
ALTER TABLE document_data RENAME COLUMN content_id TO version_id;
ALTER TABLE model_answer RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_extraction RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_mention RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_evidence RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_matter_document RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_matter_verdict RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_gone RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_gone_file RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_gone_mention RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_summary RENAME COLUMN content_id TO version_id;
ALTER TABLE knowledge_matter_choice RENAME COLUMN content_id TO version_id;
ALTER TABLE similar_template_document RENAME COLUMN content_id TO version_id;
ALTER TABLE similar_template_value RENAME COLUMN content_id TO version_id;
ALTER TABLE recognition_request RENAME COLUMN content_id TO version_id;
ALTER TABLE generated_document RENAME COLUMN content_id TO version_id;
ALTER TABLE similar_key_data RENAME COLUMN content_id TO version_id;

CREATE TABLE document (
    id                     INTEGER PRIMARY KEY,
    uid                    TEXT NOT NULL UNIQUE,
    prevailing_version_id  INTEGER REFERENCES version (id) ON DELETE SET NULL,
    prevailing_reason      TEXT NOT NULL CHECK (prevailing_reason IN ('only', 'rule', 'model', 'person')),
    created_at             INTEGER NOT NULL
);
ALTER TABLE version ADD COLUMN document_id INTEGER REFERENCES document (id);
CREATE INDEX version_by_document ON version (document_id);

CREATE TABLE appearance (
    id             INTEGER PRIMARY KEY,
    version_id     INTEGER NOT NULL REFERENCES version (id) ON DELETE CASCADE,
    resource_hash  TEXT NOT NULL REFERENCES file (hash) ON DELETE CASCADE,
    selector       TEXT NOT NULL CHECK (selector = 'file'
                       OR (selector GLOB 'pages:[1-9]*-[1-9]*' AND substr(selector, 7) NOT GLOB '*[^0-9-]*')),
    position       INTEGER NOT NULL,
    cut_by         TEXT CHECK (cut_by IN ('rule', 'model', 'person')),
    cut_state      TEXT NOT NULL CHECK (cut_state IN ('proposed', 'confirmed')),
    created_at     INTEGER NOT NULL,
    UNIQUE (resource_hash, selector),
    CHECK ((selector = 'file') = (cut_by IS NULL)),
    CHECK (selector <> 'file' OR cut_state = 'confirmed')
);
CREATE INDEX appearance_by_version ON appearance (version_id);

INSERT INTO document (id, uid, prevailing_version_id, prevailing_reason, created_at)
    SELECT id,
           lower(substr(printf('%012x', first_seen_at), 1, 8) || '-' || substr(printf('%012x', first_seen_at), 9, 4)
                 || '-7' || substr(hex(randomblob(2)), 2, 3) || '-' || substr('89ab', 1 + (random() & 3), 1)
                 || substr(hex(randomblob(2)), 2, 3) || '-' || hex(randomblob(6))),
           id, 'only', first_seen_at
    FROM version;
UPDATE version SET document_id = id;
INSERT INTO appearance (version_id, resource_hash, selector, position, cut_by, cut_state, created_at)
    SELECT version_id, hash, 'file', 1, NULL, 'confirmed', CAST(strftime('%s', 'now') AS INTEGER) * 1000 FROM file;

CREATE TRIGGER version_has_document AFTER INSERT ON version WHEN NEW.document_id IS NULL BEGIN
    INSERT INTO document (id, uid, prevailing_version_id, prevailing_reason, created_at)
        VALUES (NEW.id,
                lower(substr(printf('%012x', NEW.first_seen_at), 1, 8) || '-' || substr(printf('%012x', NEW.first_seen_at), 9, 4)
                      || '-7' || substr(hex(randomblob(2)), 2, 3) || '-' || substr('89ab', 1 + (random() & 3), 1)
                      || substr(hex(randomblob(2)), 2, 3) || '-' || hex(randomblob(6))),
                NEW.id, 'only', NEW.first_seen_at);
    UPDATE version SET document_id = NEW.id WHERE id = NEW.id;
END;

CREATE TRIGGER version_gone_document AFTER DELETE ON version BEGIN
    DELETE FROM document WHERE id = OLD.document_id
        AND NOT EXISTS (SELECT 1 FROM version WHERE document_id = OLD.document_id);
END;

CREATE TRIGGER document_prevails_own BEFORE UPDATE OF prevailing_version_id ON document
    WHEN NEW.prevailing_version_id IS NOT NULL
        AND NOT EXISTS (SELECT 1 FROM version WHERE id = NEW.prevailing_version_id AND document_id = NEW.id)
BEGIN
    SELECT RAISE(ABORT, 'the prevailing version is not one of the versions of the document');
END;

CREATE TRIGGER file_appears AFTER INSERT ON file BEGIN
    INSERT OR IGNORE INTO appearance (version_id, resource_hash, selector, position, cut_by, cut_state, created_at)
        VALUES (NEW.version_id, NEW.hash, 'file', 1, NULL, 'confirmed', CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;

CREATE TRIGGER file_moves AFTER UPDATE OF version_id ON file BEGIN
    UPDATE appearance SET version_id = NEW.version_id WHERE resource_hash = NEW.hash AND selector = 'file';
END;
";

/// What the cutter says of each step of a PDF (D-556, `docbase/cut`):
/// `after_page` is the step from that page to the next, 0 the start of
/// the file. `verdict` is the rules' (`doubtful` counts as continue until
/// the model answers), `rule` the one that gave it; `question` the key of
/// the question about the two pages (`model_answer`, task `cut`) and
/// `model_says` its answer (1, the next page starts another document);
/// `rule_type` and `model_type` the type the next page suggests. Steps of
/// other rules, another catalog or another text (`text_stamp`, the latest
/// `resource_text` of the file) are decided again. They go with their file.
const V4: &str = "
CREATE TABLE cut_step (
    resource_hash  TEXT NOT NULL REFERENCES file (hash) ON DELETE CASCADE,
    after_page     INTEGER NOT NULL CHECK (after_page >= 0),
    verdict        TEXT NOT NULL CHECK (verdict IN ('continue', 'new', 'doubtful')),
    rule           TEXT NOT NULL,
    question       TEXT,
    model_says     INTEGER CHECK (model_says IN (0, 1)),
    rule_type      TEXT,
    model_type     TEXT,
    rules_version  TEXT NOT NULL,
    text_stamp     INTEGER NOT NULL,
    created_at     INTEGER NOT NULL,
    PRIMARY KEY (resource_hash, after_page)
) WITHOUT ROWID;
CREATE INDEX cut_step_by_question ON cut_step (question) WHERE question IS NOT NULL;
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
            "INSERT INTO version (id, state, first_seen_at) VALUES (1, 'processed', 1);
             INSERT INTO file (hash, version_id, size) VALUES ('h1', 1, 10), ('h2', 1, 10);
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

    /// A base at version 3 with invented data keeps it and gains the steps
    /// of the cutter, which go with their file.
    #[test]
    fn version_4_keeps_the_cuts_of_each_file() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        build_at(&mut conn, 3);
        conn.execute_batch(
            "INSERT INTO version (id, state, first_seen_at) VALUES (1, 'processed', 1);
             INSERT INTO file (hash, version_id, size) VALUES ('h1', 1, 10);",
        )
        .unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn).unwrap(), CURRENT_VERSION);
        let appearances: i64 = conn.query_row("SELECT count(*) FROM appearance", [], |row| row.get(0)).unwrap();
        assert_eq!(appearances, 1);
        conn.execute_batch(
            "INSERT INTO cut_step (resource_hash, after_page, verdict, rule, rules_version, text_stamp, created_at)
                VALUES ('h1', 0, 'new', 'start', 'r', 1, 1), ('h1', 1, 'doubtful', 'none', 'r', 1, 1);",
        )
        .unwrap();
        let bad = conn.execute(
            "INSERT INTO cut_step (resource_hash, after_page, verdict, rule, rules_version, text_stamp, created_at) VALUES ('h1', 2, 'quizas', 'x', 'r', 1, 1)",
            [],
        );
        assert!(bad.is_err());
        conn.execute("DELETE FROM file WHERE hash = 'h1'", []).unwrap();
        let left: i64 = conn.query_row("SELECT count(*) FROM cut_step", [], |row| row.get(0)).unwrap();
        assert_eq!(left, 0);
    }

    #[test]
    fn a_base_of_version_2_without_the_reading_setting_keeps_the_reading_off() {
        let dir = crate::docbase::testing::TempDir::new();
        let path = dir.path().join(crate::docbase::PRIVATE_DB_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        {
            let mut conn = Connection::open(&path).unwrap();
            build_at(&mut conn, 2);
        }
        let base = crate::docbase::DocBase::open(&path).unwrap();
        assert_eq!(version(&base.conn()).unwrap(), CURRENT_VERSION);
        assert!(!base.background_reading().unwrap());
    }

    /// A base at version 2 with invented data: the versions keep their
    /// numbers and everything that hangs from them, each one gets its
    /// document and each file its whole-file appearance; the reading stays
    /// off as it was.
    #[test]
    fn version_3_gives_each_version_its_document_and_each_file_its_appearance() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        build_at(&mut conn, 2);
        conn.execute_batch(
            "INSERT INTO content (id, state, text_hash, first_seen_at) VALUES (4, 'processed', 't4', 1761530000000), (9, 'processed', 't9', 1761530000001);
             INSERT INTO file (hash, content_id, size) VALUES ('h1', 4, 10), ('h2', 4, 12), ('h3', 9, 10);
             INSERT INTO derivative (content_id, kind, engine, body, created_at) VALUES (4, 'text', 'extraction', 'uno', 1);
             INSERT INTO model_answer (question, task, prompt_version, engine, content_id, prompt, answer, asked_at)
                VALUES ('q1', 'type', '1', 'motor', 9, 'p', 'a', 1);",
        )
        .unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn).unwrap(), CURRENT_VERSION);
        let ints = |sql: &str| -> Vec<i64> {
            let mut statement = conn.prepare(sql).unwrap();
            let rows = statement.query_map([], |row| row.get(0)).unwrap();
            rows.collect::<Result<_, _>>().unwrap()
        };
        assert_eq!(ints("SELECT id FROM version ORDER BY id"), vec![4, 9]);
        assert_eq!(ints("SELECT version_id FROM file ORDER BY hash"), vec![4, 4, 9]);
        assert_eq!(ints("SELECT version_id FROM derivative"), vec![4]);
        assert_eq!(ints("SELECT version_id FROM model_answer"), vec![9]);
        assert_eq!(ints("SELECT document_id FROM version ORDER BY id"), vec![4, 9]);
        assert_eq!(ints("SELECT prevailing_version_id FROM document ORDER BY id"), vec![4, 9]);
        let uids: Vec<String> = {
            let mut statement = conn.prepare("SELECT uid FROM document ORDER BY id").unwrap();
            let rows = statement.query_map([], |row| row.get(0)).unwrap();
            rows.collect::<Result<_, _>>().unwrap()
        };
        for uid in &uids {
            let parsed = uuid::Uuid::parse_str(uid).unwrap();
            assert_eq!(parsed.get_version_num(), 7, "{uid}");
            assert_eq!(parsed.get_variant(), uuid::Variant::RFC4122, "{uid}");
        }
        assert!(uids[0] < uids[1], "the uid follows when the version was first seen");
        assert_eq!(
            ints("SELECT version_id FROM appearance WHERE selector = 'file' AND cut_by IS NULL AND cut_state = 'confirmed' ORDER BY resource_hash"),
            vec![4, 4, 9]
        );
        let broken: i64 = conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| row.get(0)).unwrap();
        assert_eq!(broken, 0);
        let settings: i64 = conn.query_row("SELECT count(*) FROM docbase_setting", [], |row| row.get(0)).unwrap();
        assert_eq!(settings, 0);

        // A new version and its file keep themselves; a file that goes
        // takes its appearance; a version that goes takes its document.
        conn.execute_batch(
            "INSERT INTO version (id, state, first_seen_at) VALUES (12, 'pending', 1761530000500);
             INSERT INTO file (hash, version_id, size) VALUES ('h4', 12, 10);",
        )
        .unwrap();
        assert_eq!(ints("SELECT document_id FROM version WHERE id = 12"), vec![12]);
        assert_eq!(ints("SELECT version_id FROM appearance WHERE resource_hash = 'h4'"), vec![12]);
        conn.execute_batch("UPDATE file SET version_id = 9 WHERE version_id = 12; DELETE FROM version WHERE id = 12;").unwrap();
        assert_eq!(ints("SELECT version_id FROM appearance WHERE resource_hash = 'h4'"), vec![9]);
        assert_eq!(ints("SELECT id FROM document ORDER BY id"), vec![4, 9]);
        conn.execute("DELETE FROM file WHERE hash = 'h2'", []).unwrap();
        assert_eq!(ints("SELECT count(*) FROM appearance"), vec![3]);
        // The prevailing version is one of the document's; a cut of pages
        // says who made it, the whole file does not.
        assert!(conn.execute("UPDATE document SET prevailing_version_id = 9 WHERE id = 4", []).is_err());
        assert!(conn
            .execute("INSERT INTO appearance (version_id, resource_hash, selector, position, cut_by, cut_state, created_at) VALUES (4, 'h1', 'pages:2-3', 2, NULL, 'proposed', 1)", [])
            .is_err());
        assert!(conn
            .execute("INSERT INTO appearance (version_id, resource_hash, selector, position, cut_by, cut_state, created_at) VALUES (4, 'h1', 'pages:2-3', 2, 'rule', 'proposed', 1)", [])
            .is_ok());
        assert!(conn
            .execute("INSERT INTO appearance (version_id, resource_hash, selector, position, cut_by, cut_state, created_at) VALUES (4, 'h1', 'paginas', 3, 'rule', 'proposed', 1)", [])
            .is_err());
    }
}
