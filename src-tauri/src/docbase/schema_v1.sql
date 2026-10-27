-- The shape of the base at version 29 of the earlier numbering, in one
-- migration (D-548). Generated from that base's sqlite_master (tables,
-- then indexes, then triggers, each in the order they were created);
-- the inner tables of the search tables are made by SQLite itself.

CREATE TABLE space (
    id          TEXT PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('private', 'shared')),
    created_at  INTEGER NOT NULL
);

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

CREATE TABLE content (
    id             INTEGER PRIMARY KEY,
    state          TEXT NOT NULL,
    text_hash      TEXT,
    word_count     INTEGER,
    error          TEXT,
    first_seen_at  INTEGER NOT NULL,
    processed_at   INTEGER
);

CREATE TABLE file (
    hash        TEXT PRIMARY KEY,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    size        INTEGER NOT NULL
);

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

CREATE TABLE derivative (
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    engine      TEXT NOT NULL,
    body        TEXT NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (content_id, kind)
);

CREATE VIRTUAL TABLE content_fts USING fts5 (
    body,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TABLE project (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    kind        TEXT NOT NULL CHECK (kind IN ('general', 'view')),
    path        TEXT,
    members     TEXT NOT NULL DEFAULT 'open',
    created_at  INTEGER NOT NULL
, wiki_active INTEGER NOT NULL DEFAULT 0);

CREATE TABLE project_origin (
    project_id  TEXT NOT NULL REFERENCES project (id) ON DELETE CASCADE,
    origin_id   TEXT NOT NULL REFERENCES origin (id) ON DELETE CASCADE,
    PRIMARY KEY (project_id, origin_id)
);

CREATE TABLE change (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    at         INTEGER NOT NULL,
    kind       TEXT NOT NULL,
    origin_id  TEXT NOT NULL,
    rel_path   TEXT NOT NULL,
    file_hash  TEXT,
    detail     TEXT
);

CREATE TABLE exclusion (
    origin_id   TEXT NOT NULL REFERENCES origin (id) ON DELETE CASCADE,
    rel_path    TEXT NOT NULL,
    is_folder   INTEGER NOT NULL,
    created_at  INTEGER NOT NULL,
    PRIMARY KEY (origin_id, rel_path)
);

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

CREATE TABLE wiki_page_source (
    page_id      INTEGER NOT NULL REFERENCES wiki_page (id) ON DELETE CASCADE,
    identity     TEXT NOT NULL,
    location_id  INTEGER REFERENCES location (id) ON DELETE SET NULL,
    PRIMARY KEY (page_id, identity)
);

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

CREATE TABLE similar_common (
    shingle  INTEGER PRIMARY KEY,
    docs     INTEGER NOT NULL
);

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
    decided_at           INTEGER, origin TEXT NOT NULL DEFAULT 'measured', measured TEXT, type_rule TEXT,
    PRIMARY KEY (a, b),
    CHECK (a < b)
);

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

CREATE TABLE document_data (
    content_id   INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    doc_type     TEXT NOT NULL,
    data         TEXT NOT NULL DEFAULT '[]',
    missing      TEXT NOT NULL DEFAULT '[]',
    question     TEXT,
    basis        TEXT NOT NULL,
    computed_at  INTEGER NOT NULL
);

CREATE TABLE docbase_setting (
    key    TEXT PRIMARY KEY,
    value  TEXT NOT NULL
);

CREATE TABLE type_decision (
    type_id      TEXT PRIMARY KEY,
    decision     TEXT NOT NULL CHECK (decision IN ('confirmed', 'rejected')),
    name         TEXT,
    description  TEXT,
    decided_at   INTEGER NOT NULL
);

CREATE TABLE "model_answer" (
    question        TEXT NOT NULL,
    task            TEXT NOT NULL,
    prompt_version  TEXT NOT NULL,
    engine          TEXT NOT NULL,
    content_id      INTEGER,
    prompt          TEXT NOT NULL,
    answer          TEXT NOT NULL,
    asked_at        INTEGER NOT NULL,
    PRIMARY KEY (question, engine)
);

CREATE TABLE knowledge_entity (
    id           TEXT PRIMARY KEY,
    kind         TEXT NOT NULL,
    name         TEXT NOT NULL,
    state        TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'merged', 'retired')),
    merged_into  TEXT REFERENCES knowledge_entity (id) ON DELETE SET NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE TABLE knowledge_alias (
    entity_id   TEXT NOT NULL REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    normalized  TEXT NOT NULL,
    written     TEXT NOT NULL,
    mentions    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_id, normalized)
);

CREATE TABLE knowledge_extraction (
    content_id      INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    state           TEXT NOT NULL CHECK (state IN ('done', 'failed')),
    basis           TEXT NOT NULL,
    prompt_version  TEXT NOT NULL,
    vocabulary      TEXT NOT NULL,
    text_hash       TEXT NOT NULL,
    text_at         INTEGER NOT NULL,
    engine          TEXT,
    questions       TEXT NOT NULL DEFAULT '[]',
    parts           INTEGER NOT NULL DEFAULT 0,
    mentions        INTEGER NOT NULL DEFAULT 0,
    evidence        INTEGER NOT NULL DEFAULT 0,
    discarded       TEXT NOT NULL DEFAULT '[]',
    error           TEXT,
    extracted_at    INTEGER NOT NULL
, doc_date TEXT);

CREATE TABLE knowledge_mention (
    id           INTEGER PRIMARY KEY,
    content_id   INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    ref          TEXT NOT NULL,
    same_as      TEXT,
    kind         TEXT NOT NULL,
    written      TEXT NOT NULL,
    normalized   TEXT NOT NULL,
    role         TEXT NOT NULL,
    quote        TEXT NOT NULL,
    page         INTEGER,
    entity_id    TEXT REFERENCES knowledge_entity (id) ON DELETE SET NULL,
    resolved_by  TEXT, defined INTEGER NOT NULL DEFAULT 0,
    UNIQUE (content_id, ref)
);

CREATE TABLE knowledge_identifier (
    mention_id  INTEGER NOT NULL REFERENCES knowledge_mention (id) ON DELETE CASCADE,
    kind        TEXT NOT NULL,
    value       TEXT NOT NULL,
    normalized  TEXT NOT NULL,
    quote       TEXT NOT NULL,
    page        INTEGER,
    by_shape    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (mention_id, kind, normalized)
);

CREATE TABLE knowledge_claim (
    id                 INTEGER PRIMARY KEY,
    subject            TEXT NOT NULL REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    predicate          TEXT NOT NULL,
    object_entity      TEXT REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    object_value       TEXT,
    object_normalized  TEXT,
    valid_from         TEXT,
    valid_to           TEXT,
    state              TEXT NOT NULL DEFAULT 'supported' CHECK (state IN ('supported', 'conflict', 'retired', 'pending')),
    computed_at        INTEGER NOT NULL, context_entity TEXT REFERENCES knowledge_entity (id) ON DELETE SET NULL, qualifier TEXT, amount TEXT, detail TEXT, certainty TEXT,
    CHECK ((object_entity IS NULL) != (object_value IS NULL))
);

CREATE TABLE knowledge_evidence (
    id                 INTEGER PRIMARY KEY,
    content_id         INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    subject_mention    INTEGER NOT NULL REFERENCES knowledge_mention (id) ON DELETE CASCADE,
    predicate          TEXT NOT NULL,
    object_mention     INTEGER REFERENCES knowledge_mention (id) ON DELETE CASCADE,
    object_value       TEXT,
    object_normalized  TEXT,
    quote              TEXT NOT NULL,
    page               INTEGER,
    fact_time          TEXT,
    claim_id           INTEGER REFERENCES knowledge_claim (id) ON DELETE SET NULL, amount TEXT, detail TEXT,
    CHECK ((object_mention IS NULL) != (object_value IS NULL))
);

CREATE TABLE knowledge_doubt (
    a              TEXT NOT NULL,
    b              TEXT NOT NULL,
    kind           TEXT NOT NULL,
    entity_a       TEXT,
    entity_b       TEXT,
    reason         TEXT NOT NULL,
    detail         TEXT,
    evidence_hash  TEXT NOT NULL,
    computed_at    INTEGER NOT NULL,
    PRIMARY KEY (a, b)
);

CREATE TABLE knowledge_verdict (
    a          TEXT NOT NULL,
    b          TEXT NOT NULL,
    engine     TEXT NOT NULL,
    verdict    TEXT NOT NULL CHECK (verdict IN ('same', 'different', 'doubtful')),
    reason     TEXT,
    question   TEXT NOT NULL,
    asked_at   INTEGER NOT NULL, evidence TEXT,
    PRIMARY KEY (a, b, engine)
);

CREATE TABLE knowledge_dirty (
    scope  TEXT NOT NULL CHECK (scope IN ('resolve', 'retire', 'project')),
    ref    TEXT NOT NULL,
    since  INTEGER NOT NULL,
    PRIMARY KEY (scope, ref)
);

CREATE TABLE knowledge_page (
    entity_id    TEXT PRIMARY KEY,
    path         TEXT NOT NULL UNIQUE,
    state        TEXT NOT NULL DEFAULT 'projected' CHECK (state IN ('projected', 'kept')),
    fingerprint  TEXT NOT NULL,
    synthesis    TEXT,
    sentences    INTEGER NOT NULL DEFAULT 0,
    discarded    INTEGER NOT NULL DEFAULT 0,
    written_at   INTEGER NOT NULL
, body TEXT);

CREATE TABLE knowledge_source_block (
    path        TEXT PRIMARY KEY,
    block_hash  TEXT NOT NULL,
    size        INTEGER NOT NULL,
    mtime_ms    INTEGER NOT NULL
);

CREATE TABLE knowledge_matter (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'retired')),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
, status TEXT NOT NULL DEFAULT 'open', origin TEXT NOT NULL DEFAULT 'proposed' CHECK (origin IN ('proposed', 'person')), confirmation TEXT NOT NULL DEFAULT 'proposed' CHECK (confirmation IN ('proposed', 'confirmed', 'discarded')), name_by_person INTEGER NOT NULL DEFAULT 0, decided_at INTEGER, decided_documents TEXT);

CREATE TABLE knowledge_matter_document (
    matter_id   TEXT NOT NULL REFERENCES knowledge_matter (id) ON DELETE CASCADE,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    state       TEXT NOT NULL DEFAULT 'firm' CHECK (state IN ('firm', 'proposed')),
    reasons     TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (matter_id, content_id)
);

CREATE TABLE knowledge_matter_verdict (
    content_id  INTEGER NOT NULL,
    matter_id   TEXT NOT NULL,
    engine      TEXT NOT NULL,
    verdict     TEXT NOT NULL CHECK (verdict IN ('same', 'different', 'doubtful')),
    reason      TEXT,
    question    TEXT NOT NULL,
    asked_at    INTEGER NOT NULL,
    PRIMARY KEY (content_id, matter_id, engine)
);

CREATE TABLE knowledge_gone (
    content_id  INTEGER PRIMARY KEY,
    text_hash   TEXT,
    gone_at     INTEGER NOT NULL
);

CREATE TABLE knowledge_gone_file (
    hash        TEXT PRIMARY KEY,
    content_id  INTEGER NOT NULL,
    gone_at     INTEGER NOT NULL
);

CREATE TABLE knowledge_gone_mention (
    content_id  INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    normalized  TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    gone_at     INTEGER NOT NULL,
    PRIMARY KEY (content_id, kind, normalized)
);

CREATE TABLE knowledge_group (
    group_id   TEXT NOT NULL REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    member_id  TEXT NOT NULL REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    reasons    TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (group_id, member_id)
);

CREATE TABLE knowledge_summary (
    content_id  INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    topic       TEXT,
    points      TEXT NOT NULL DEFAULT '[]'
);

CREATE VIRTUAL TABLE content_search USING fts5 (
    numbers,
    summary,
    tokenize = 'unicode61 remove_diacritics 2'
);

CREATE TABLE knowledge_matter_choice (
    matter_id   TEXT NOT NULL REFERENCES knowledge_matter (id) ON DELETE CASCADE,
    content_id  INTEGER NOT NULL,
    choice      TEXT NOT NULL CHECK (choice IN ('in', 'out')),
    decided_at  INTEGER NOT NULL,
    PRIMARY KEY (matter_id, content_id)
);

CREATE TABLE knowledge_overview (
    id           INTEGER PRIMARY KEY CHECK (id = 1),
    path         TEXT NOT NULL,
    body         TEXT NOT NULL,
    fingerprint  TEXT NOT NULL,
    written_at   INTEGER NOT NULL
);

CREATE TABLE knowledge_note (
    id           INTEGER PRIMARY KEY,
    object_id    TEXT UNIQUE,
    title        TEXT,
    origin_path  TEXT,
    text         TEXT NOT NULL,
    updated_at   INTEGER NOT NULL
);

CREATE TABLE knowledge_query (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    question     TEXT NOT NULL DEFAULT '',
    answer       TEXT NOT NULL,
    refs         TEXT NOT NULL DEFAULT '[]',
    origin_path  TEXT UNIQUE,
    created_at   INTEGER NOT NULL
);

CREATE TABLE knowledge_research (
    id           INTEGER PRIMARY KEY,
    object_id    TEXT,
    topic        TEXT NOT NULL,
    body         TEXT NOT NULL,
    sources      TEXT NOT NULL DEFAULT '[]',
    created_at   INTEGER NOT NULL
);

CREATE TABLE similar_template (
    principal       INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    fingerprint     TEXT NOT NULL,
    segments        TEXT NOT NULL DEFAULT '[]',
    same_documents  TEXT NOT NULL DEFAULT '[]',
    computed_at     INTEGER NOT NULL
, naming INTEGER REFERENCES template_naming (id) ON DELETE SET NULL);

CREATE TABLE similar_template_document (
    principal   INTEGER NOT NULL REFERENCES similar_template (principal) ON DELETE CASCADE,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    PRIMARY KEY (principal, content_id)
);

CREATE TABLE similar_template_field (
    principal   INTEGER NOT NULL REFERENCES similar_template (principal) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    ordinal     INTEGER NOT NULL,
    form_name   TEXT,
    same_as     INTEGER, anchor TEXT NOT NULL DEFAULT '', label TEXT,
    PRIMARY KEY (principal, position)
);

CREATE TABLE similar_template_value (
    principal   INTEGER NOT NULL,
    position    INTEGER NOT NULL,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    value       TEXT NOT NULL,
    PRIMARY KEY (principal, position, content_id),
    FOREIGN KEY (principal, position) REFERENCES similar_template_field (principal, position) ON DELETE CASCADE
);

CREATE TABLE recognition_request (
    content_id  INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    asked_at    INTEGER NOT NULL
);

CREATE TABLE template_naming (
    id          INTEGER PRIMARY KEY,
    documents   TEXT NOT NULL DEFAULT '[]',
    name        TEXT,
    named_at    INTEGER,
    built_at    INTEGER NOT NULL
);

CREATE TABLE template_field_choice (
    naming       INTEGER NOT NULL REFERENCES template_naming (id) ON DELETE CASCADE,
    anchor       TEXT NOT NULL,
    name         TEXT,
    what         TEXT,
    fixed        INTEGER NOT NULL DEFAULT 0,
    fixed_value  TEXT,
    decided_at   INTEGER NOT NULL,
    PRIMARY KEY (naming, anchor)
);

CREATE TABLE template_field_answer (
    naming       INTEGER NOT NULL REFERENCES template_naming (id) ON DELETE CASCADE,
    anchor       TEXT NOT NULL,
    name         TEXT,
    what         TEXT,
    question     TEXT NOT NULL,
    answered_at  INTEGER NOT NULL,
    PRIMARY KEY (naming, anchor)
);

CREATE TABLE generated_document (
    id                 INTEGER PRIMARY KEY,
    naming             INTEGER,
    principal          INTEGER,
    template_name      TEXT NOT NULL,
    template_engine    TEXT NOT NULL,
    template_built_at  INTEGER NOT NULL,
    matter_id          TEXT,
    matter_name        TEXT,
    from_document      INTEGER,
    origin_id          TEXT NOT NULL,
    rel_path           TEXT NOT NULL,
    file_hash          TEXT NOT NULL,
    field_values       TEXT NOT NULL DEFAULT '[]',
    generated_at       INTEGER NOT NULL,
    content_id         INTEGER REFERENCES content (id) ON DELETE SET NULL,
    matter_content     INTEGER
);

CREATE TABLE similar_key_data (
    content_id   INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    print        TEXT NOT NULL,
    computed_at  INTEGER NOT NULL
);

CREATE TABLE operation_log (
    id            INTEGER PRIMARY KEY,
    operation     TEXT NOT NULL,
    caller        TEXT NOT NULL,
    request       TEXT,
    conversation  TEXT,
    engine        TEXT,
    input         TEXT NOT NULL,
    card          TEXT NOT NULL,
    chosen        TEXT,
    state         TEXT NOT NULL CHECK (state IN ('proposed', 'applied', 'cancelled', 'undone', 'failed', 'expired')),
    undo          TEXT,
    error         TEXT,
    proposed_at   INTEGER NOT NULL,
    decided_at    INTEGER,
    undone_at     INTEGER
);

CREATE TABLE local_program (
    id            TEXT PRIMARY KEY,
    name          TEXT NOT NULL,
    permission    TEXT NOT NULL CHECK (permission IN ('read', 'change')),
    token_hash    TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    last_used_at  INTEGER,
    revoked_at    INTEGER
);

CREATE TABLE local_program_use (
    program_id  TEXT NOT NULL REFERENCES local_program (id) ON DELETE CASCADE,
    day         TEXT NOT NULL,
    operation   TEXT NOT NULL,
    count       INTEGER NOT NULL,
    PRIMARY KEY (program_id, day, operation)
);

CREATE TABLE knowledge_matter_record (
    matter_id     TEXT NOT NULL REFERENCES knowledge_matter (id) ON DELETE CASCADE,
    portal        TEXT NOT NULL,
    jurisdiction  TEXT NOT NULL,
    number        INTEGER NOT NULL,
    year          INTEGER NOT NULL,
    title         TEXT,
    source        TEXT NOT NULL,
    added_at      INTEGER NOT NULL,
    PRIMARY KEY (matter_id, portal, jurisdiction, number, year)
);

CREATE TABLE knowledge_matter_question (
    record_matter  TEXT NOT NULL,
    other_matter   TEXT NOT NULL,
    shared         INTEGER NOT NULL DEFAULT 0,
    citing         INTEGER NOT NULL DEFAULT 0,
    answer         TEXT CHECK (answer IN ('same', 'different')),
    asked_at       INTEGER NOT NULL,
    answered_at    INTEGER,
    PRIMARY KEY (record_matter, other_matter)
);

CREATE TABLE "knowledge_decision" (
    id                   INTEGER PRIMARY KEY,
    kind                 TEXT NOT NULL CHECK (kind IN ('join', 'separate', 'wrong', 'confirm', 'name')),
    a                    TEXT NOT NULL,
    b                    TEXT,
    note                 TEXT,
    shown_evidence       TEXT NOT NULL,
    shown_evidence_hash  TEXT NOT NULL,
    state                TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'undone')),
    decided_at           INTEGER NOT NULL,
    undone_at            INTEGER,
    review               TEXT,
    applied_as           TEXT
);

CREATE UNIQUE INDEX content_by_text ON content (text_hash) WHERE text_hash IS NOT NULL;

CREATE INDEX content_by_state ON content (state);

CREATE INDEX file_by_content ON file (content_id);

CREATE INDEX location_by_hash ON location (file_hash);

CREATE INDEX location_by_hash_state ON location (hash_state);

CREATE UNIQUE INDEX one_general_project ON project (kind) WHERE kind = 'general';

CREATE UNIQUE INDEX label_by_key ON label (coalesce(group_id, 0), key);

CREATE INDEX location_label_by_label ON location_label (label_id);

CREATE INDEX wiki_link_by_target ON wiki_link (to_page);

CREATE INDEX wiki_page_source_by_location ON wiki_page_source (location_id);

CREATE INDEX similar_band_by_content ON similar_band (content_id);

CREATE INDEX similar_common_by_docs ON similar_common (docs);

CREATE INDEX similar_pair_by_b ON similar_pair (b);

CREATE INDEX similar_pair_by_state ON similar_pair (state);

CREATE INDEX similar_group_member_by_content ON similar_group_member (content_id);

CREATE INDEX document_type_by_type ON document_type (doc_type);

CREATE INDEX knowledge_entity_by_kind ON knowledge_entity (kind, state);

CREATE INDEX knowledge_alias_by_name ON knowledge_alias (normalized);

CREATE INDEX knowledge_mention_by_name ON knowledge_mention (normalized);

CREATE INDEX knowledge_mention_by_entity ON knowledge_mention (entity_id);

CREATE INDEX knowledge_identifier_by_value ON knowledge_identifier (kind, normalized);

CREATE INDEX knowledge_evidence_by_content ON knowledge_evidence (content_id);

CREATE INDEX knowledge_evidence_by_claim ON knowledge_evidence (claim_id);

CREATE INDEX knowledge_doubt_by_entity_a ON knowledge_doubt (entity_a);

CREATE INDEX knowledge_doubt_by_entity_b ON knowledge_doubt (entity_b);

CREATE INDEX knowledge_matter_document_by_content ON knowledge_matter_document (content_id);

CREATE INDEX knowledge_gone_file_by_content ON knowledge_gone_file (content_id);

CREATE UNIQUE INDEX knowledge_claim_key
    ON knowledge_claim (subject, predicate, coalesce(object_entity, ''), coalesce(object_normalized, ''), coalesce(context_entity, ''),
                        coalesce(qualifier, ''));

CREATE INDEX knowledge_group_by_member ON knowledge_group (member_id);

CREATE INDEX knowledge_research_object ON knowledge_research (object_id);

CREATE INDEX similar_template_document_by_content ON similar_template_document (content_id);

CREATE INDEX generated_document_by_naming ON generated_document (naming);

CREATE INDEX generated_document_by_hash ON generated_document (file_hash);

CREATE INDEX generated_document_by_place ON generated_document (origin_id, rel_path);

CREATE INDEX generated_document_by_content ON generated_document (content_id);

CREATE INDEX operation_log_by_state ON operation_log (state, proposed_at);

CREATE INDEX knowledge_matter_record_by_key ON knowledge_matter_record (portal, jurisdiction, number, year);

CREATE INDEX knowledge_decision_by_a ON knowledge_decision (a);

CREATE INDEX knowledge_decision_by_b ON knowledge_decision (b);

CREATE TRIGGER knowledge_extraction_gone AFTER DELETE ON knowledge_extraction BEGIN
    INSERT OR REPLACE INTO knowledge_dirty (scope, ref, since)
        VALUES ('retire', CAST(OLD.content_id AS TEXT), CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;

CREATE TRIGGER knowledge_content_gone AFTER DELETE ON content BEGIN
    INSERT OR REPLACE INTO knowledge_gone (content_id, text_hash, gone_at)
        VALUES (OLD.id, OLD.text_hash, CAST(strftime('%s', 'now') AS INTEGER) * 1000);
    DELETE FROM knowledge_verdict
        WHERE a LIKE 'cluster:' || OLD.id || ':%' OR b LIKE 'cluster:' || OLD.id || ':%';
    DELETE FROM knowledge_matter_verdict WHERE content_id = OLD.id;
END;

CREATE TRIGGER knowledge_file_gone AFTER DELETE ON file BEGIN
    INSERT OR REPLACE INTO knowledge_gone_file (hash, content_id, gone_at)
        VALUES (OLD.hash, OLD.content_id, CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;

CREATE TRIGGER knowledge_mention_gone AFTER DELETE ON knowledge_mention WHEN OLD.entity_id IS NOT NULL BEGIN
    INSERT OR REPLACE INTO knowledge_gone_mention (content_id, kind, normalized, entity_id, gone_at)
        VALUES (OLD.content_id, OLD.kind, OLD.normalized, OLD.entity_id, CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;
