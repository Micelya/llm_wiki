//! Tables of the document base and how they are brought up to date.
//!
//! The schema version is SQLite's `user_version`. Each entry of
//! `MIGRATIONS` takes the database one version forward; a database is
//! never edited in place outside of them.

use rusqlite::Connection;

use super::{now_ms, sql_err};

pub const CURRENT_VERSION: i64 = MIGRATIONS.len() as i64;

const MIGRATIONS: &[&str] = &[V1, V2, V3, V4, V5, V6, V7, V8];

/// What a person decided about the document types the model proposed
/// (R-22). `confirmed`: the type joins the closed list the model chooses
/// from, with the `name` (and `description`) the person gave it.
/// `rejected`: the type is not used; its documents are typed again, with
/// the model told not to propose it. A decision is undone by deleting it.
const V8: &str = "
CREATE TABLE type_decision (
    type_id      TEXT PRIMARY KEY,
    decision     TEXT NOT NULL CHECK (decision IN ('confirmed', 'rejected')),
    name         TEXT,
    description  TEXT,
    decided_at   INTEGER NOT NULL
);
";

/// Type, stage and data of each document, and the relations the type
/// corrects (passes 4 to 6 of `similar/`).
///
/// - `model_answer`: every answer of the model, by the fingerprint of the
///   question (its prompt version and its whole text), with the engine
///   and model that gave it (R-20). The same question is never asked
///   twice. No foreign key: an answer outlives the document it was about,
///   so the same text coming back costs nothing.
/// - `document_type`: what each document is and in what stage, where
///   that came from (`marks`, `model`, or `family`: the answer about the
///   principal of its family), and on what it was decided (`basis`):
///   when that changes it is decided again. `proposed` marks a type the
///   model proposed that is not in the catalog: a person confirms it.
/// - `document_data`: the data of each document by the rules of its type
///   and, where they found nothing, by the model.
/// - `similar_pair.origin`: `measured` (pass 1) or `type` (proposed by a
///   rule of the type, pass 6, without a measurement). `measured` keeps
///   the measured proposal when a type rule changed it, and `type_rule`
///   names that rule.
/// - `docbase_setting`: small settings of the base (the engine that
///   answers while the archive's wiki is active).
const V7: &str = "
CREATE TABLE model_answer (
    question        TEXT PRIMARY KEY,
    task            TEXT NOT NULL,
    prompt_version  TEXT NOT NULL,
    engine          TEXT NOT NULL,
    content_id      INTEGER,
    prompt          TEXT NOT NULL,
    answer          TEXT NOT NULL,
    asked_at        INTEGER NOT NULL
);

CREATE TABLE document_type (
    content_id            INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    doc_type              TEXT NOT NULL,
    stage                 TEXT,
    proposed              INTEGER NOT NULL DEFAULT 0,
    proposed_name         TEXT,
    proposed_description  TEXT,
    source                TEXT NOT NULL CHECK (source IN ('marks', 'model', 'family')),
    principal             INTEGER,
    confidence            REAL,
    reason                TEXT,
    question              TEXT,
    basis                 TEXT NOT NULL,
    computed_at           INTEGER NOT NULL
);
CREATE INDEX document_type_by_type ON document_type (doc_type);

CREATE TABLE document_data (
    content_id   INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    doc_type     TEXT NOT NULL,
    data         TEXT NOT NULL DEFAULT '[]',
    missing      TEXT NOT NULL DEFAULT '[]',
    question     TEXT,
    basis        TEXT NOT NULL,
    computed_at  INTEGER NOT NULL
);

ALTER TABLE similar_pair ADD COLUMN origin TEXT NOT NULL DEFAULT 'measured';
ALTER TABLE similar_pair ADD COLUMN measured TEXT;
ALTER TABLE similar_pair ADD COLUMN type_rule TEXT;

CREATE TABLE docbase_setting (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);
";

/// Similar documents (`similar/`). Everything here but the decisions of
/// a person is computed again from the texts whenever it is missing or
/// stale.
///
/// - `similar_signature`: the shingles and MinHash of each content with
///   text. It has no foreign key on purpose: when a content goes away its
///   shingles are first uncounted from `similar_common`, then the row is
///   dropped (`similar::store`).
/// - `similar_band`: the LSH buckets that find a content's neighbours.
/// - `similar_common`: in how many documents each shingle appears (pass
///   0); the most common ones are routine text and do not count.
/// - `document_marks`: what was verified without a model (pass 2), and
///   what the catalog said to leave out of the comparison.
/// - `similar_dirty`: contents whose neighbourhood must be measured again.
/// - `similar_pair`: one row per pair shown (`a` < `b`): measures,
///   proposal and evidence (computed), and the decision of a person
///   (`state`, `chosen_relation`, `wiki_source`, `shown_evidence_hash`,
///   `decided_at`), which a new computation never overwrites.
/// - `similar_group` / `similar_group_member`: provisional families, each
///   against its principal document (pass 3).
const V6: &str = "
CREATE TABLE similar_signature (
    content_id   INTEGER PRIMARY KEY,
    engine       TEXT NOT NULL,
    text_at      INTEGER NOT NULL,
    words        INTEGER NOT NULL,
    shingles     BLOB NOT NULL,
    minhash      BLOB NOT NULL,
    computed_at  INTEGER NOT NULL
);

CREATE TABLE similar_band (
    band        INTEGER NOT NULL,
    bucket      INTEGER NOT NULL,
    content_id  INTEGER NOT NULL REFERENCES similar_signature (content_id) ON DELETE CASCADE,
    PRIMARY KEY (band, bucket, content_id)
) WITHOUT ROWID;
CREATE INDEX similar_band_by_content ON similar_band (content_id);

CREATE TABLE similar_common (
    shingle  INTEGER PRIMARY KEY,
    docs     INTEGER NOT NULL
);
CREATE INDEX similar_common_by_docs ON similar_common (docs);

CREATE TABLE document_marks (
    content_id   INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    engine       TEXT NOT NULL,
    marks        TEXT NOT NULL DEFAULT '[]',
    signatures   TEXT NOT NULL DEFAULT '[]',
    skip_pages   TEXT NOT NULL DEFAULT '[]',
    drop_phrases TEXT NOT NULL DEFAULT '[]',
    computed_at  INTEGER NOT NULL
);

CREATE TABLE similar_dirty (
    content_id  INTEGER PRIMARY KEY,
    since       INTEGER NOT NULL
);

CREATE TABLE similar_pair (
    a                    INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    b                    INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    relation             TEXT NOT NULL,
    subject              INTEGER,
    certainty            TEXT NOT NULL,
    jaccard              REAL NOT NULL,
    containment          REAL NOT NULL,
    cover                REAL NOT NULL,
    similarity           REAL NOT NULL,
    description          TEXT NOT NULL,
    suggested_source     INTEGER,
    evidence             TEXT NOT NULL,
    evidence_hash        TEXT NOT NULL,
    engine               TEXT NOT NULL,
    computed_at          INTEGER NOT NULL,
    state                TEXT NOT NULL DEFAULT 'proposed' CHECK (state IN ('proposed', 'joined', 'discarded')),
    chosen_relation      TEXT,
    wiki_source          INTEGER REFERENCES content (id) ON DELETE SET NULL,
    shown_evidence_hash  TEXT,
    decided_at           INTEGER,
    PRIMARY KEY (a, b),
    CHECK (a < b)
);
CREATE INDEX similar_pair_by_b ON similar_pair (b);
CREATE INDEX similar_pair_by_state ON similar_pair (state);

CREATE TABLE similar_group (
    principal    INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    members      INTEGER NOT NULL,
    flags        TEXT NOT NULL DEFAULT '[]',
    computed_at  INTEGER NOT NULL
);

CREATE TABLE similar_group_member (
    principal   INTEGER NOT NULL REFERENCES similar_group (principal) ON DELETE CASCADE,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    relation    TEXT,
    cover       REAL,
    PRIMARY KEY (principal, content_id)
);
CREATE INDEX similar_group_member_by_content ON similar_group_member (content_id);
";

/// Index of a project's wiki, rebuilt from its pages: the pages, the
/// links between them (`to_page` is NULL for a broken link) and the
/// source documents each page names (`location_id` when the base knows
/// that file). `path` is relative to the project, with forward slashes.
const V5: &str = "
CREATE TABLE wiki_page (
    id          INTEGER PRIMARY KEY,
    project_id  TEXT NOT NULL REFERENCES project (id) ON DELETE CASCADE,
    path        TEXT NOT NULL,
    slug        TEXT NOT NULL,
    title       TEXT NOT NULL,
    type        TEXT NOT NULL,
    size        INTEGER NOT NULL,
    mtime_ms    INTEGER NOT NULL,
    UNIQUE (project_id, path)
);

CREATE TABLE wiki_link (
    from_page  INTEGER NOT NULL REFERENCES wiki_page (id) ON DELETE CASCADE,
    target     TEXT NOT NULL,
    kind       TEXT NOT NULL,
    to_page    INTEGER REFERENCES wiki_page (id) ON DELETE SET NULL,
    PRIMARY KEY (from_page, target, kind)
);
CREATE INDEX wiki_link_by_target ON wiki_link (to_page);

CREATE TABLE wiki_page_source (
    page_id      INTEGER NOT NULL REFERENCES wiki_page (id) ON DELETE CASCADE,
    identity     TEXT NOT NULL,
    location_id  INTEGER REFERENCES location (id) ON DELETE SET NULL,
    PRIMARY KEY (page_id, identity)
);
CREATE INDEX wiki_page_source_by_location ON wiki_page_source (location_id);
";

/// Label groups, labels, the folder rules that assign them, what the
/// user put or removed by hand, and what each file ends up with.
/// `label.key` is the text a label is found by (the folder name it came
/// from); `label.name` is what the user reads and can rename.
const V4: &str = "
CREATE TABLE label_group (
    id          INTEGER PRIMARY KEY,
    name        TEXT NOT NULL COLLATE NOCASE UNIQUE,
    created_at  INTEGER NOT NULL
);

CREATE TABLE label (
    id          INTEGER PRIMARY KEY,
    group_id    INTEGER REFERENCES label_group (id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    key         TEXT NOT NULL COLLATE NOCASE,
    color       TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);
CREATE UNIQUE INDEX label_by_key ON label (coalesce(group_id, 0), key);

CREATE TABLE label_rule (
    id          INTEGER PRIMARY KEY,
    origin_id   TEXT NOT NULL REFERENCES origin (id) ON DELETE CASCADE,
    folder      TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('fixed', 'levels')),
    label_id    INTEGER REFERENCES label (id) ON DELETE CASCADE,
    groups      TEXT NOT NULL DEFAULT '[]',
    created_at  INTEGER NOT NULL
);

CREATE TABLE label_manual (
    location_id  INTEGER NOT NULL REFERENCES location (id) ON DELETE CASCADE,
    label_id     INTEGER NOT NULL REFERENCES label (id) ON DELETE CASCADE,
    mode         TEXT NOT NULL CHECK (mode IN ('add', 'remove')),
    PRIMARY KEY (location_id, label_id)
);

CREATE TABLE location_label (
    location_id  INTEGER NOT NULL REFERENCES location (id) ON DELETE CASCADE,
    label_id     INTEGER NOT NULL REFERENCES label (id) ON DELETE CASCADE,
    PRIMARY KEY (location_id, label_id)
);
CREATE INDEX location_label_by_label ON location_label (label_id);
";

/// Files and folders the user left out of the base. Nothing on disk is
/// touched; `rel_path` is the path inside the origin.
const V3: &str = "
CREATE TABLE exclusion (
    origin_id   TEXT NOT NULL REFERENCES origin (id) ON DELETE CASCADE,
    rel_path    TEXT NOT NULL,
    is_folder   INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (origin_id, rel_path)
);
";

/// The general project's wiki is fed from the base only once the user
/// turns it on: feeding it uses the model.
const V2: &str = "
ALTER TABLE project ADD COLUMN wiki_active INTEGER NOT NULL DEFAULT 0;
";

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
