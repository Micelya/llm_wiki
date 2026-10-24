//! Tables of the document base and how they are brought up to date.
//!
//! The schema version is SQLite's `user_version`. Each entry of
//! `MIGRATIONS` takes the database one version forward; a database is
//! never edited in place outside of them.

use rusqlite::Connection;

use super::{now_ms, sql_err};

pub const CURRENT_VERSION: i64 = MIGRATIONS.len() as i64;

const MIGRATIONS: &[&str] = &[V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12, V13, V14, V15, V16, V17, V18, V19, V20, V21, V22, V23, V24, V25, V26, V27, V28, V29];

/// A person can choose how an entity is named (D-516): `knowledge_decision`
/// admits `name`, `a` being a group of mentions of the entity and `b` the
/// name chosen (`name:<the name>`). Every decision is kept as it was.
const V29: &str = "
CREATE TABLE knowledge_decision_v29 (
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
INSERT INTO knowledge_decision_v29 (id, kind, a, b, note, shown_evidence, shown_evidence_hash, state, decided_at, undone_at, review, applied_as)
    SELECT id, kind, a, b, note, shown_evidence, shown_evidence_hash, state, decided_at, undone_at, review, applied_as FROM knowledge_decision;
DROP TABLE knowledge_decision;
ALTER TABLE knowledge_decision_v29 RENAME TO knowledge_decision;
CREATE INDEX knowledge_decision_by_a ON knowledge_decision (a);
CREATE INDEX knowledge_decision_by_b ON knowledge_decision (b);
";

/// The records of a court a matter has (`knowledge/matter_records.rs`): a
/// record is data of a matter, not the matter. A matter can have none, one
/// or several; the integration of a record brought from a portal looks for
/// the matter that has it before making one.
///
/// - `knowledge_matter_record`: the portal, the jurisdiction, the number and
///   the year of the record; `title`, its official title as the portal
///   gives it; `source`, where it came from (`integration`: a record brought
///   from a portal).
/// - `knowledge_matter_question`: whether a matter that looks like the one
///   of a record is the same (shared documents, `shared`; documents that cite
///   its number, `citing`): one question for a person per pair. `answer` is
///   `same` (they were joined) or `different` (not asked again).
const V28: &str = "
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
CREATE INDEX knowledge_matter_record_by_key ON knowledge_matter_record (portal, jurisdiction, number, year);
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
";

/// The programs a person let use the base through the server inside the
/// machine (`local_server/programs.rs`, D-495, P-227): an assistant such as
/// Claude Code, or a script. Each has its own token, kept in the system
/// keychain; here only its SHA-256, to know it without the keychain.
///
/// - `permission`: `read` (the default: only the readings) or `change`
///   (it can also ask for changes, which wait for a person in Atención).
/// - `revoked_at`: when a person removed it; its token no longer works and
///   the row stays, so the record of operations can still name it.
/// - `local_program_use`: how many times a day it used each operation,
///   without what it read.
const V27: &str = "
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
";

/// The record of operations of the contract (`operations/log.rs`, D-498):
/// each change the chat (or, later, another program) proposed, with the
/// request of the person, the card as it was seen, what was left ticked,
/// who proposed it, when, whether it was confirmed, cancelled or undone,
/// and what undoing it needs. A decision of a person, kept as such.
///
/// - `caller`: `chat`, `screen`, `clipper` or `program:<id>`.
/// - `request`: the message of the person (chat), or nothing.
/// - `conversation`: the conversation and the turn of the chat.
/// - `engine`: the engine and model that proposed it.
/// - `input`, `card`, `chosen`, `undo`: JSON.
const V26: &str = "
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
CREATE INDEX operation_log_by_state ON operation_log (state, proposed_at);
";

/// The key data each document's pairs were measured with (D-497): a
/// fingerprint of its parties, matter, identifiers, amounts and dates, as
/// the base knew them then. When what the base knows of a document changes
/// (the reading names its parties later, two entities are merged), its
/// pairs are measured again. Computed; nothing of a person.
const V25: &str = "
CREATE TABLE similar_key_data (
    content_id   INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    print        TEXT NOT NULL,
    computed_at  INTEGER NOT NULL
);
";

/// The documents generated from a template (D-488): a Word written in the
/// folder «Generados» of the work folder, with the template's text and the
/// values a person gave. The record does not hang from the template, which
/// is built again whenever its family changes, nor from the matter: it
/// says what was done, and survives both.
///
/// - `naming`: the lasting identity of the template (`template_naming`);
///   `principal`, `template_name`, `template_engine` and `template_built_at`:
///   the template as it was (its principal, its name, its version).
/// - `matter_id`, `matter_name`: the matter it was generated for, if any.
/// - `from_document`: the document whose data it started with, if any.
/// - `origin_id`, `rel_path`: where the file was written, inside the work
///   folder's origin; `file_hash`: the SHA-256 of what was written.
/// - `field_values`: each field, with its name, its value and where the value
///   came from (JSON).
/// - `content_id`: the document the base found at that place (it changes
///   when a person edits the Word); `matter_content`: the one put in the
///   matter, as a person's choice.
const V24: &str = "
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
CREATE INDEX generated_document_by_naming ON generated_document (naming);
CREATE INDEX generated_document_by_hash ON generated_document (file_hash);
CREATE INDEX generated_document_by_place ON generated_document (origin_id, rel_path);
CREATE INDEX generated_document_by_content ON generated_document (content_id);
";

/// The names of the templates and of their fields (D-488), apart from the
/// templates, which are built again whenever their family changes. A
/// template is not joined to a matter nor to a person and keeps none of
/// their data: only its shared text, its fields and the documents it came
/// from.
///
/// - `template_naming`: the lasting identity of a template, across its
///   rebuilds and a change of its principal: the documents it had when it
///   was last built (JSON list of content ids) and the name a person gave
///   it. A template being built takes the one of its previous build, or the
///   one no other template has that shares with it most of the documents.
/// - `similar_template.naming`: the one each template took.
/// - `similar_template_field.anchor`: the words of the template around the
///   field, by which what was said about it is found again when the
///   template is built again; `label`: the name its text gives it (its name
///   in the form, the label before it), computed with the template.
/// - `template_field_choice`: what a person decided about a field: its
///   name, what it is (a person, a date…), whether it is fixed and with
///   what value. The program never writes it.
/// - `template_field_answer`: what the model said of a field (its name and
///   what it is) and the question it answered (`model_answer`).
const V23: &str = "
CREATE TABLE template_naming (
    id          INTEGER PRIMARY KEY,
    documents   TEXT NOT NULL DEFAULT '[]',
    name        TEXT,
    named_at    INTEGER,
    built_at    INTEGER NOT NULL
);

ALTER TABLE similar_template ADD COLUMN naming INTEGER REFERENCES template_naming (id) ON DELETE SET NULL;
ALTER TABLE similar_template_field ADD COLUMN anchor TEXT NOT NULL DEFAULT '';
ALTER TABLE similar_template_field ADD COLUMN label TEXT;

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
";

/// What a person asked to recognize (P-242, D-452): the documents whose
/// text is only pixels that she asked the base to read, in the order she
/// asked (`position`), until each one is read or fails for good. In the
/// base, so closing the app does not lose them; the failures of the
/// provider on each one are counted in memory.
const V22: &str = "
CREATE TABLE recognition_request (
    content_id  INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    asked_at    INTEGER NOT NULL
);
";

/// The template of each family of the same template (P-247, D-438): the
/// text its documents share, with the fields that change, and the value
/// of each document in each field. Computed from the texts, without a
/// model, also for the families nobody confirmed; built again whenever
/// its family changes (`fingerprint`: its documents, their texts and the
/// engine). Nothing here is a decision of a person.
///
/// - `similar_template`: one per family, by its principal. `segments`:
///   the text, as a JSON list of `{"text"}` and `{"field": n}`;
///   `same_documents`: pairs of documents (JSON `[a, b]`) where no field
///   changes but a few words, possibly the same document (P-218).
/// - `similar_template_document`: its documents, the principal first.
/// - `similar_template_field`: its fields in order (`position`), with
///   the kind of thing each is (`kind`: identifier, amount, date, name…),
///   its order among those of its kind (`ordinal`: "Fecha 2"), for a
///   field of a form its name there (`form_name`), and for the same field
///   said again (the number of a receipt on each stub) the position of its
///   first place (`same_as`).
/// - `similar_template_value`: what each document says in each field
///   (empty: nothing there).
const V21: &str = "
CREATE TABLE similar_template (
    principal       INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    fingerprint     TEXT NOT NULL,
    segments        TEXT NOT NULL DEFAULT '[]',
    same_documents  TEXT NOT NULL DEFAULT '[]',
    computed_at     INTEGER NOT NULL
);

CREATE TABLE similar_template_document (
    principal   INTEGER NOT NULL REFERENCES similar_template (principal) ON DELETE CASCADE,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    PRIMARY KEY (principal, content_id)
);
CREATE INDEX similar_template_document_by_content ON similar_template_document (content_id);

CREATE TABLE similar_template_field (
    principal   INTEGER NOT NULL REFERENCES similar_template (principal) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    ordinal     INTEGER NOT NULL,
    form_name   TEXT,
    same_as     INTEGER,
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
";

/// The base without the wiki's folder (D-396, D-402): what the files of
/// the wiki held lives here.
///
/// - `knowledge_page.body`: the page as the projection writes it (markdown
///   with its header), without the notes of a person; `path` stays its
///   logical name, what the links between pages say. NULL until the
///   projection writes it again.
/// - `knowledge_overview`: the overview of the archive (one row).
/// - `knowledge_note`: the notes of a person on an entity, a matter or a
///   document (`object_id`: `ent-…`, `mat-…`, `doc-…`). A note whose page
///   matched no object when it was imported from the wiki keeps no object
///   (`object_id` NULL) and is shown apart with its `title`, so it is
///   never lost. `origin_path` is the page it came from, if any.
/// - `knowledge_query`: a saved question of the chat, with its answer and
///   the pages it cites (`refs`, a JSON list of logical paths or ids).
///   `origin_path`: the file of `wiki/queries` it was imported from.
/// - `knowledge_research`: a research on the web (W5), of an object or
///   loose (`object_id` NULL), with its sources (JSON list).
/// - `background-reading` in `docbase_setting`: whether the base reads and
///   resolves on its own; it takes the value the general project's
///   switch of the wiki had.
const V20: &str = "
ALTER TABLE knowledge_page ADD COLUMN body TEXT;

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
CREATE INDEX knowledge_research_object ON knowledge_research (object_id);

INSERT OR REPLACE INTO docbase_setting (key, value)
    SELECT 'background-reading', CASE WHEN wiki_active != 0 THEN '1' ELSE '0' END FROM project WHERE kind = 'general' LIMIT 1;
";

/// What a person chose for the documents of a matter (D-386): `in` (put
/// in by hand) or `out` (taken out by hand). The grouping applies it over
/// what it finds, so it survives every grouping; a matter a person created
/// (`origin = 'person'`) has only the documents a person put in it.
const V19: &str = "
CREATE TABLE knowledge_matter_choice (
    matter_id   TEXT NOT NULL REFERENCES knowledge_matter (id) ON DELETE CASCADE,
    content_id  INTEGER NOT NULL,
    choice      TEXT NOT NULL CHECK (choice IN ('in', 'out')),
    decided_at  INTEGER NOT NULL,
    PRIMARY KEY (matter_id, content_id)
);
";

/// One concept of matter (D-373, D-381): what the grouping proposes is a
/// proposal until a person confirms it, and every matter has a status.
///
/// - `status`: `open` for all of them for now (its life cycle is P-213).
/// - `origin`: `proposed` (the grouping of `knowledge/matters.rs`) or
///   `person` (created by a person; the grouping never touches it).
/// - `confirmation`: `proposed`, `confirmed` or `discarded`, what a
///   person decided, which a grouping never overwrites. A discarded
///   matter is retired and keeps in `decided_documents` (JSON list of
///   content ids) the documents it had when it was discarded: the same
///   documents grouped again stay discarded; a group with a new document
///   is proposed again.
/// - `name_by_person`: 1 when a person named it; the grouping keeps it.
///
/// The matters there are stay proposals, open: nobody had decided about
/// them; what a person decided about their documents is in
/// `knowledge_decision`, untouched.
const V18: &str = "
ALTER TABLE knowledge_matter ADD COLUMN status TEXT NOT NULL DEFAULT 'open';
ALTER TABLE knowledge_matter ADD COLUMN origin TEXT NOT NULL DEFAULT 'proposed' CHECK (origin IN ('proposed', 'person'));
ALTER TABLE knowledge_matter ADD COLUMN confirmation TEXT NOT NULL DEFAULT 'proposed' CHECK (confirmation IN ('proposed', 'confirmed', 'discarded'));
ALTER TABLE knowledge_matter ADD COLUMN name_by_person INTEGER NOT NULL DEFAULT 0;
ALTER TABLE knowledge_matter ADD COLUMN decided_at INTEGER;
ALTER TABLE knowledge_matter ADD COLUMN decided_documents TEXT;
";

/// What the search keeps of each document besides its text (D-350): its
/// numbers and identifiers as digits alone (`numbers`, so "12.345.678"
/// and "12345678" are one) and its summary in the language of the archive
/// (`summary`). The rowid is the content id; `search::fill_missing` fills
/// it on opening and `search::index_content` keeps it up to date.
const V17: &str = "
CREATE VIRTUAL TABLE content_search USING fts5 (
    numbers,
    summary,
    tokenize = 'unicode61 remove_diacritics 2'
);
";

/// The summary of each document (D-330): one more thing the one reading
/// of the knowledge layer gives, written and replaced with it. `points` is
/// JSON: `[{text, quote, page, mentions, evidence}]`, `mentions` being ids
/// of `knowledge_mention` and `evidence` ids of `knowledge_evidence` whose
/// quote is the point's passage.
const V16: &str = "
CREATE TABLE knowledge_summary (
    content_id  INTEGER PRIMARY KEY REFERENCES content (id) ON DELETE CASCADE,
    topic       TEXT,
    points      TEXT NOT NULL DEFAULT '[]'
);
";

/// How sure the record is of each claim, and decisions that survive a
/// reading again (D-320, D-321):
///
/// - `knowledge_claim.certainty`: computed on every resolution from what is
///   already kept (its state, the doubts, the decisions): `{"level",
///   "reasons"}`, the level being `quoted` (its quote is in the document,
///   nobody confirmed it), `confirmed`, `doubtful` or `corrected`.
/// - `knowledge_decision`: a person can also confirm a claim (`confirm`);
///   `review` says why an active decision no longer finds what it named
///   (for a person to review, never dropped in silence) and `applied_as`
///   the keys it applies to when what it named is written another way.
const V15: &str = "
ALTER TABLE knowledge_claim ADD COLUMN certainty TEXT;
CREATE TABLE knowledge_decision_v15 (
    id                   INTEGER PRIMARY KEY,
    kind                 TEXT NOT NULL CHECK (kind IN ('join', 'separate', 'wrong', 'confirm')),
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
INSERT INTO knowledge_decision_v15 (id, kind, a, b, note, shown_evidence, shown_evidence_hash, state, decided_at, undone_at)
    SELECT id, kind, a, b, note, shown_evidence, shown_evidence_hash, state, decided_at, undone_at FROM knowledge_decision;
DROP TABLE knowledge_decision;
ALTER TABLE knowledge_decision_v15 RENAME TO knowledge_decision;
CREATE INDEX knowledge_decision_by_a ON knowledge_decision (a);
CREATE INDEX knowledge_decision_by_b ON knowledge_decision (b);
";

/// Groups of companies (D-260): `knowledge_group` says which entities are
/// the companies of a group entity (several companies with the same name
/// and another legal form, and another sign that links them; or a group
/// the documents name), with the reasons. Computed again on every
/// resolution, like the entities.
const V14: &str = "
CREATE TABLE knowledge_group (
    group_id   TEXT NOT NULL REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    member_id  TEXT NOT NULL REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    reasons    TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (group_id, member_id)
);
CREATE INDEX knowledge_group_by_member ON knowledge_group (member_id);
";

/// What the second question of the reading brings (D-244 to D-252):
///
/// - `amount` and `detail` of a piece of evidence and of a claim: an amount
///   between two parties is a relation (the debtor is the subject, the
///   creditor the object entity) with the amount as written, and its kind,
///   currency and concept in `detail` (JSON). The kind of amount is the
///   claim's `qualifier` and its normalized amount its `object_normalized`,
///   so that two amounts between the same parties are two claims.
/// - `knowledge_mention.defined`: the mention is a term the document
///   defines ("en adelante «la Empresa»"): a local alias of the entity it
///   defines, which gives it neither name nor alias.
/// - `knowledge_extraction.doc_date`: the date of the document as its text
///   writes it, the time of a value whose quote says none.
const V13: &str = "
ALTER TABLE knowledge_evidence ADD COLUMN amount TEXT;
ALTER TABLE knowledge_evidence ADD COLUMN detail TEXT;
ALTER TABLE knowledge_claim ADD COLUMN amount TEXT;
ALTER TABLE knowledge_claim ADD COLUMN detail TEXT;
ALTER TABLE knowledge_mention ADD COLUMN defined INTEGER NOT NULL DEFAULT 0;
ALTER TABLE knowledge_extraction ADD COLUMN doc_date TEXT;
";

/// What the model was shown of the two sides when it said a verdict on a
/// pair of groups (`knowledge_verdict.evidence`, a fingerprint): a verdict
/// that kept them apart is asked again when what they show changed (R-08:
/// new and different evidence). Verdicts kept before have none.
const V12: &str = "
ALTER TABLE knowledge_verdict ADD COLUMN evidence TEXT;
";

/// Changes and deletions in the knowledge layer (`knowledge/changes.rs`):
/// what a document that went away leaves behind, so that what a person
/// decided about it, and the ids of its entities, go on to the document
/// that takes its place (the same text back, or the new version of the
/// file at its path).
///
/// - `knowledge_gone`: a content that went away, with the fingerprint of
///   its text. Its id is never given to another content.
/// - `knowledge_gone_file`: the files it had (the change log names the
///   path where each one was replaced by a new version).
/// - `knowledge_gone_mention`: the entity each name of a document had
///   when its mentions went away (the document went, or was read again),
///   so the same name of the document that takes its place keeps it.
/// - `knowledge_claim.context_entity`: the entity a value is said within
///   (a position held in a company): the same value within two entities
///   is two claims, not a conflict. `qualifier`: what kind of value the
///   quote says it is (a domicile constituted for a proceeding): values of
///   different kinds are not in conflict either.
/// - What the model said of the groups of a content that goes away is
///   forgotten (the answers stay in `model_answer`: the same question
///   costs nothing).
/// - `knowledge_matter` / `knowledge_matter_document`: the matters of the
///   archive as groups of documents (`knowledge/matters.rs`), with a
///   stable id, a name and, for each document, the reasons it is in the
///   matter. Computed again on every resolution; a person's decisions
///   about them are in `knowledge_decision` (`content:<id>` keys).
///   `knowledge_matter_verdict`: what the model said of a document that
///   no evidence puts in a matter (does it belong to this one?): a
///   proposal for a person, never a link between documents (R-03).
const V11: &str = "
CREATE TABLE knowledge_matter (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    state       TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'retired')),
    created_at  INTEGER NOT NULL,
    updated_at  INTEGER NOT NULL
);

CREATE TABLE knowledge_matter_document (
    matter_id   TEXT NOT NULL REFERENCES knowledge_matter (id) ON DELETE CASCADE,
    content_id  INTEGER NOT NULL REFERENCES content (id) ON DELETE CASCADE,
    state       TEXT NOT NULL DEFAULT 'firm' CHECK (state IN ('firm', 'proposed')),
    reasons     TEXT NOT NULL DEFAULT '[]',
    PRIMARY KEY (matter_id, content_id)
);
CREATE INDEX knowledge_matter_document_by_content ON knowledge_matter_document (content_id);

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
CREATE INDEX knowledge_gone_file_by_content ON knowledge_gone_file (content_id);

CREATE TABLE knowledge_gone_mention (
    content_id  INTEGER NOT NULL,
    kind        TEXT NOT NULL,
    normalized  TEXT NOT NULL,
    entity_id   TEXT NOT NULL,
    gone_at     INTEGER NOT NULL,
    PRIMARY KEY (content_id, kind, normalized)
);

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

ALTER TABLE knowledge_claim ADD COLUMN context_entity TEXT REFERENCES knowledge_entity (id) ON DELETE SET NULL;
ALTER TABLE knowledge_claim ADD COLUMN qualifier TEXT;
DROP INDEX knowledge_claim_key;
CREATE UNIQUE INDEX knowledge_claim_key
    ON knowledge_claim (subject, predicate, coalesce(object_entity, ''), coalesce(object_normalized, ''), coalesce(context_entity, ''),
                        coalesce(qualifier, ''));
";

/// The projection of the knowledge layer into the general project's wiki
/// (`knowledge/pages.rs`).
///
/// - `knowledge_page`: the page of each entity that has one. `path` is
///   fixed the first time (a readable name plus the entity's id) and kept
///   when the entity is renamed, so links never break. `fingerprint` is
///   that of what the app wrote (everything but the notes): the file is
///   written again only when it changes. `synthesis` is the key of the
///   model's answer the page uses (`model_answer`), or none while it is
///   being asked; `discarded` counts the sentences of that answer the
///   computation threw away. `state` is `projected`, or `kept` for a page
///   whose entity left the record but that holds notes of a person.
/// - `knowledge_source_block`: the block of entities the app keeps at the
///   end of each document's page, with what the file looked like after it
///   was written (size and date), so a page written again by the wiki gets
///   its block back.
const V10: &str = "
CREATE TABLE knowledge_page (
    entity_id    TEXT PRIMARY KEY,
    path         TEXT NOT NULL UNIQUE,
    state        TEXT NOT NULL DEFAULT 'projected' CHECK (state IN ('projected', 'kept')),
    fingerprint  TEXT NOT NULL,
    synthesis    TEXT,
    sentences    INTEGER NOT NULL DEFAULT 0,
    discarded    INTEGER NOT NULL DEFAULT 0,
    written_at   INTEGER NOT NULL
);

CREATE TABLE knowledge_source_block (
    path        TEXT PRIMARY KEY,
    block_hash  TEXT NOT NULL,
    size        INTEGER NOT NULL,
    mtime_ms    INTEGER NOT NULL
);
";

/// The knowledge layer (`knowledge/`): people, organizations and matters
/// the documents name, how each document names them, and what the
/// documents say of them, every piece with the quote that backs it.
///
/// - `model_answer` is keyed by the question *and* the engine and model
///   that answered it: another model asks again. The answers kept so far
///   keep their key and their engine.
/// - `knowledge_extraction`: one reading per document, with what it was
///   decided on (`basis`: prompt version, vocabulary and text), the
///   engine, the questions it took and what was discarded and why.
/// - `knowledge_mention`: a way a document names an entity, with its role
///   and the quote that holds it (verified in the text); `ref` and
///   `same_as` are the model's names for mentions inside one document.
///   `entity_id` is set by the resolution of identities.
/// - `knowledge_identifier`: identifiers (DNI, CUIT…) of a mention.
/// - `knowledge_evidence`: what a document says of a mention (subject,
///   predicate, object mention or value, quote, page, time of the fact).
///   `claim_id` groups it with the evidence that says the same.
/// - `knowledge_entity`, `knowledge_alias`, `knowledge_claim`: the
///   canonical record, filled by the resolution of identities. An entity
///   id never changes; a merged entity points to the one it went into.
/// - `knowledge_decision`: what a person decided about identities (join,
///   separate, wrong datum), with the evidence shown. A computation never
///   overwrites it; undoing it keeps it, marked undone.
/// - `knowledge_doubt`: pairs of mentions the resolution could not tell
///   apart (the same name with an identifier misread, the model
///   abstaining…), for a person to decide; computed again on every
///   resolution, never joined by it. `a` and `b` are the keys of groups
///   of mentions of one document (`cluster:<content>:<kind>:<name>`).
/// - `knowledge_verdict`: what the model said of two groups of mentions
///   (the same, different, it cannot tell), by the keys of the groups and
///   the engine that said it: evidence kept for every later resolution,
///   so a pair is never asked twice (R-20). `question` is the key of the
///   answer in `model_answer`.
/// - `knowledge_dirty`: work left for the incremental passes: contents to
///   resolve again, contents whose evidence must be retired, pages to
///   project again. A content that goes away marks itself (trigger).
const V9: &str = "
CREATE TABLE model_answer_by_engine (
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
INSERT INTO model_answer_by_engine (question, task, prompt_version, engine, content_id, prompt, answer, asked_at)
    SELECT question, task, prompt_version, engine, content_id, prompt, answer, asked_at FROM model_answer;
DROP TABLE model_answer;
ALTER TABLE model_answer_by_engine RENAME TO model_answer;

CREATE TABLE knowledge_entity (
    id           TEXT PRIMARY KEY,
    kind         TEXT NOT NULL,
    name         TEXT NOT NULL,
    state        TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'merged', 'retired')),
    merged_into  TEXT REFERENCES knowledge_entity (id) ON DELETE SET NULL,
    created_at   INTEGER NOT NULL,
    updated_at   INTEGER NOT NULL
);
CREATE INDEX knowledge_entity_by_kind ON knowledge_entity (kind, state);

CREATE TABLE knowledge_alias (
    entity_id   TEXT NOT NULL REFERENCES knowledge_entity (id) ON DELETE CASCADE,
    normalized  TEXT NOT NULL,
    written     TEXT NOT NULL,
    mentions    INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_id, normalized)
);
CREATE INDEX knowledge_alias_by_name ON knowledge_alias (normalized);

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
);

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
    resolved_by  TEXT,
    UNIQUE (content_id, ref)
);
CREATE INDEX knowledge_mention_by_name ON knowledge_mention (normalized);
CREATE INDEX knowledge_mention_by_entity ON knowledge_mention (entity_id);

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
CREATE INDEX knowledge_identifier_by_value ON knowledge_identifier (kind, normalized);

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
    computed_at        INTEGER NOT NULL,
    CHECK ((object_entity IS NULL) != (object_value IS NULL))
);
CREATE UNIQUE INDEX knowledge_claim_key
    ON knowledge_claim (subject, predicate, coalesce(object_entity, ''), coalesce(object_normalized, ''));

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
    claim_id           INTEGER REFERENCES knowledge_claim (id) ON DELETE SET NULL,
    CHECK ((object_mention IS NULL) != (object_value IS NULL))
);
CREATE INDEX knowledge_evidence_by_content ON knowledge_evidence (content_id);
CREATE INDEX knowledge_evidence_by_claim ON knowledge_evidence (claim_id);

CREATE TABLE knowledge_decision (
    id                   INTEGER PRIMARY KEY,
    kind                 TEXT NOT NULL CHECK (kind IN ('join', 'separate', 'wrong')),
    a                    TEXT NOT NULL,
    b                    TEXT,
    note                 TEXT,
    shown_evidence       TEXT NOT NULL,
    shown_evidence_hash  TEXT NOT NULL,
    state                TEXT NOT NULL DEFAULT 'active' CHECK (state IN ('active', 'undone')),
    decided_at           INTEGER NOT NULL,
    undone_at            INTEGER
);
CREATE INDEX knowledge_decision_by_a ON knowledge_decision (a);
CREATE INDEX knowledge_decision_by_b ON knowledge_decision (b);

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
CREATE INDEX knowledge_doubt_by_entity_a ON knowledge_doubt (entity_a);
CREATE INDEX knowledge_doubt_by_entity_b ON knowledge_doubt (entity_b);

CREATE TABLE knowledge_verdict (
    a          TEXT NOT NULL,
    b          TEXT NOT NULL,
    engine     TEXT NOT NULL,
    verdict    TEXT NOT NULL CHECK (verdict IN ('same', 'different', 'doubtful')),
    reason     TEXT,
    question   TEXT NOT NULL,
    asked_at   INTEGER NOT NULL,
    PRIMARY KEY (a, b, engine)
);

CREATE TABLE knowledge_dirty (
    scope  TEXT NOT NULL CHECK (scope IN ('resolve', 'retire', 'project')),
    ref    TEXT NOT NULL,
    since  INTEGER NOT NULL,
    PRIMARY KEY (scope, ref)
);

CREATE TRIGGER knowledge_extraction_gone AFTER DELETE ON knowledge_extraction BEGIN
    INSERT OR REPLACE INTO knowledge_dirty (scope, ref, since)
        VALUES ('retire', CAST(OLD.content_id AS TEXT), CAST(strftime('%s', 'now') AS INTEGER) * 1000);
END;
";

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

    #[test]
    fn answers_kept_before_keep_their_question_and_engine() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(7) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        let insert = "INSERT INTO model_answer (question, task, prompt_version, engine, content_id, prompt, answer, asked_at)
                      VALUES ('tipo-2:x', 'type', 'tipo-2', ?1, NULL, 'p', '{}', 1)";
        conn.execute(insert, ["codex-cli:m"]).unwrap();
        migrate(&mut conn).unwrap();
        let kept: (String, String) =
            conn.query_row("SELECT question, engine FROM model_answer", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(kept, ("tipo-2:x".to_string(), "codex-cli:m".to_string()));
        // Another model keeps its own answer to the same question; the
        // same model, only one.
        conn.execute(insert, ["codex-cli:n"]).unwrap();
        assert!(conn.execute(insert, ["codex-cli:m"]).is_err());
    }

    /// V15 rebuilds `knowledge_decision` to admit `confirm`: every decision
    /// a person took before (join, separate, wrong; active and undone)
    /// stays as it was, field by field, with its id.
    #[test]
    fn the_decisions_taken_before_v15_are_all_kept() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(14) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        type Row = (i64, String, String, Option<String>, Option<String>, String, String, String, i64, Option<i64>);
        let rows: Vec<Row> = vec![
            (3, "join".into(), "ent-a".into(), Some("ent-b".into()), Some("la misma".into()), "[1,2]".into(), "h1".into(), "active".into(), 10, None),
            (5, "separate".into(), "cluster:1:persona:uno".into(), Some("cluster:2:persona:uno".into()), None, "[]".into(), "h2".into(), "active".into(), 20, None),
            (7, "wrong".into(), "claim:9".into(), None, Some("no es su domicilio".into()), "[3]".into(), "h3".into(), "active".into(), 30, None),
            (8, "join".into(), "ent-c".into(), Some("ent-d".into()), None, "[]".into(), "h4".into(), "undone".into(), 40, Some(50)),
            (9, "wrong".into(), "content:4".into(), None, None, "[]".into(), "h5".into(), "undone".into(), 60, Some(70)),
        ];
        for r in &rows {
            conn.execute(
                "INSERT INTO knowledge_decision (id, kind, a, b, note, shown_evidence, shown_evidence_hash, state, decided_at, undone_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![r.0, r.1, r.2, r.3, r.4, r.5, r.6, r.7, r.8, r.9],
            )
            .unwrap();
        }
        migrate(&mut conn).unwrap();
        let mut statement = conn
            .prepare(
                "SELECT id, kind, a, b, note, shown_evidence, shown_evidence_hash, state, decided_at, undone_at, review, applied_as
                 FROM knowledge_decision ORDER BY id",
            )
            .unwrap();
        let kept: Vec<(Row, Option<String>, Option<String>)> = statement
            .query_map([], |row| {
                Ok((
                    (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?),
                    row.get(10)?,
                    row.get(11)?,
                ))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(kept.len(), rows.len());
        for ((row, review, applied), before) in kept.iter().zip(&rows) {
            assert_eq!(row, before);
            assert!(review.is_none() && applied.is_none());
        }
        // The new kind is admitted, and the indexes are there.
        conn.execute(
            "INSERT INTO knowledge_decision (kind, a, shown_evidence, shown_evidence_hash, decided_at) VALUES ('confirm', 'claim:1', '[]', 'h', 1)",
            [],
        )
        .unwrap();
        let indexes: i64 = conn
            .query_row("SELECT count(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'knowledge_decision'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(indexes, 2);
    }

    /// A base at version 20 gets the templates' tables empty, and keeps
    /// its families: the templates are built from them afterwards.
    #[test]
    fn a_base_at_version_20_gets_the_templates_and_keeps_its_families() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(20) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO content (id, state, first_seen_at) VALUES (1, 'processed', 1), (2, 'processed', 1);
             INSERT INTO similar_group (principal, members, computed_at) VALUES (1, 2, 1);
             INSERT INTO similar_group_member (principal, content_id, relation, cover) VALUES (1, 1, NULL, NULL), (1, 2, 'same-template', 0.9);",
        )
        .unwrap();
        migrate(&mut conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, CURRENT_VERSION);
        let members: i64 = conn.query_row("SELECT count(*) FROM similar_group_member", [], |row| row.get(0)).unwrap();
        assert_eq!(members, 2);
        for table in ["similar_template", "similar_template_document", "similar_template_field", "similar_template_value"] {
            let rows: i64 = conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap();
            assert_eq!(rows, 0, "{table}");
        }
        // A template goes away with its principal, and its fields and values with it.
        conn.execute_batch(
            "INSERT INTO similar_template (principal, fingerprint, computed_at) VALUES (1, 'x', 1);
             INSERT INTO similar_template_document (principal, content_id, position) VALUES (1, 1, 0), (1, 2, 1);
             INSERT INTO similar_template_field (principal, position, kind, ordinal) VALUES (1, 0, 'date', 1);
             INSERT INTO similar_template_value (principal, position, content_id, value) VALUES (1, 0, 1, 'a'), (1, 0, 2, 'b');
             DELETE FROM content WHERE id = 1;",
        )
        .unwrap();
        let left: i64 = conn
            .query_row(
                "SELECT (SELECT count(*) FROM similar_template) + (SELECT count(*) FROM similar_template_field)
                      + (SELECT count(*) FROM similar_template_value) + (SELECT count(*) FROM similar_template_document)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(left, 0);
    }

    /// A base at version 21 gets the table of the recognitions asked for,
    /// empty, and a request goes away with its document.
    #[test]
    fn a_base_at_version_21_gets_the_recognition_requests() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(21) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        conn.execute("INSERT INTO content (id, state, first_seen_at) VALUES (1, 'needs-recognition', 1)", []).unwrap();
        migrate(&mut conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, CURRENT_VERSION);
        let count = |conn: &Connection| -> i64 { conn.query_row("SELECT count(*) FROM recognition_request", [], |row| row.get(0)).unwrap() };
        assert_eq!(count(&conn), 0);
        conn.execute("INSERT INTO recognition_request (content_id, position, asked_at) VALUES (1, 1, 1)", []).unwrap();
        conn.execute("DELETE FROM content WHERE id = 1", []).unwrap();
        assert_eq!(count(&conn), 0);
    }

    /// A base at version 22 keeps its templates as they are, with no anchor
    /// nor label yet (they are built again with them), and gets the names
    /// kept apart, empty; what a person decided goes away only with the
    /// identity of its template, never with a rebuild of the template.
    #[test]
    fn a_base_at_version_22_keeps_its_templates_and_gets_the_names_apart() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(22) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO content (id, state, first_seen_at) VALUES (1, 'processed', 1), (2, 'processed', 1);
             INSERT INTO similar_template (principal, fingerprint, segments, computed_at) VALUES (1, 'x', '[{\"field\":0}]', 1);
             INSERT INTO similar_template_document (principal, content_id, position) VALUES (1, 1, 0), (1, 2, 1);
             INSERT INTO similar_template_field (principal, position, kind, ordinal) VALUES (1, 0, 'date', 1);
             INSERT INTO similar_template_value (principal, position, content_id, value) VALUES (1, 0, 1, 'a'), (1, 0, 2, 'b');",
        )
        .unwrap();
        migrate(&mut conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, CURRENT_VERSION);
        let field: (String, Option<String>, Option<i64>) = conn
            .query_row(
                "SELECT f.anchor, f.label, t.naming FROM similar_template_field f JOIN similar_template t ON t.principal = f.principal",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(field, (String::new(), None, None));
        let values: i64 = conn.query_row("SELECT count(*) FROM similar_template_value", [], |row| row.get(0)).unwrap();
        assert_eq!(values, 2);
        for table in ["template_naming", "template_field_choice", "template_field_answer"] {
            let rows: i64 = conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap();
            assert_eq!(rows, 0, "{table}");
        }
        // A rebuild of the template leaves what was decided about its fields.
        conn.execute_batch(
            "INSERT INTO template_naming (id, documents, name, built_at) VALUES (7, '[1,2]', 'Mi modelo', 1);
             UPDATE similar_template SET naming = 7;
             INSERT INTO template_field_choice (naming, anchor, name, decided_at) VALUES (7, 'recibi de|la suma', 'Quien paga', 1);
             INSERT INTO template_field_answer (naming, anchor, name, question, answered_at) VALUES (7, 'el dia|en concepto', 'Fecha', 'campos-1:x', 1);
             DELETE FROM similar_template;",
        )
        .unwrap();
        let kept: i64 = conn
            .query_row(
                "SELECT (SELECT count(*) FROM template_naming) + (SELECT count(*) FROM template_field_choice)
                      + (SELECT count(*) FROM template_field_answer)",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(kept, 3);
        conn.execute("DELETE FROM template_naming", []).unwrap();
        let left: i64 = conn
            .query_row("SELECT (SELECT count(*) FROM template_field_choice) + (SELECT count(*) FROM template_field_answer)", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(left, 0);
    }

    /// A base at version 23 gets the record of the generated documents,
    /// empty, and keeps what it had; the record survives its document.
    #[test]
    fn a_base_at_version_23_gets_the_record_of_the_generated_documents() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(23) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        conn.execute_batch(
            "INSERT INTO content (id, state, first_seen_at) VALUES (1, 'processed', 1);
             INSERT INTO template_naming (id, documents, name, built_at) VALUES (7, '[1]', 'Mi modelo', 1);",
        )
        .unwrap();
        migrate(&mut conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, CURRENT_VERSION);
        let rows: i64 = conn.query_row("SELECT count(*) FROM generated_document", [], |row| row.get(0)).unwrap();
        assert_eq!(rows, 0);
        let names: i64 = conn.query_row("SELECT count(*) FROM template_naming", [], |row| row.get(0)).unwrap();
        assert_eq!(names, 1);
        conn.execute_batch(
            "INSERT INTO generated_document (naming, template_name, template_engine, template_built_at, origin_id, rel_path,
                                             file_hash, generated_at, content_id)
             VALUES (7, 'Mi modelo', 'template-3', 1, 'o', 'Generados/a.docx', 'h', 1, 1);
             DELETE FROM content;",
        )
        .unwrap();
        let left: (i64, Option<i64>) = conn
            .query_row("SELECT count(*), max(content_id) FROM generated_document", [], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        assert_eq!(left, (1, None), "the record stays when its document goes");
    }

    /// The owner's base is at version 8 (the type decisions of the
    /// attention view): the layer's tables come after it, and what was
    /// decided about types stays.
    #[test]
    fn a_base_at_the_attention_views_version_gets_the_layer_and_keeps_its_type_decisions() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(8) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        conn.execute("INSERT INTO type_decision (type_id, decision, name, decided_at) VALUES ('receta', 'confirmed', 'Receta', 1)", [])
            .unwrap();
        migrate(&mut conn).unwrap();
        let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        assert_eq!(version, CURRENT_VERSION);
        for table in ["type_decision", "knowledge_entity", "knowledge_page", "knowledge_matter"] {
            let found: i64 = conn
                .query_row("SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = ?1", [table], |row| row.get(0))
                .unwrap();
            assert_eq!(found, 1, "{table}");
        }
        let kept: String = conn.query_row("SELECT name FROM type_decision", [], |row| row.get(0)).unwrap();
        assert_eq!(kept, "Receta");
    }

    /// The records of a matter come after the programs of the machine: the
    /// matters there are stay as they were, with no record yet.
    #[test]
    fn a_base_before_the_records_of_a_matter_gets_them_and_keeps_its_matters() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(27) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        conn.execute(
            "INSERT INTO knowledge_matter (id, name, state, created_at, updated_at, origin, confirmation, name_by_person)
             VALUES ('mat-p-1', 'XYZ 1/2001', 'active', 1, 1, 'person', 'confirmed', 1)",
            [],
        )
        .unwrap();
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn).unwrap(), CURRENT_VERSION);
        let kept: (String, i64) = conn.query_row("SELECT name, name_by_person FROM knowledge_matter", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        assert_eq!(kept, ("XYZ 1/2001".to_string(), 1), "the update does not repair: the integration does, with the record in hand");
        for table in ["knowledge_matter_record", "knowledge_matter_question"] {
            let rows: i64 = conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get(0)).unwrap();
            assert_eq!(rows, 0, "{table}");
        }
        conn.execute(
            "INSERT INTO knowledge_matter_record (matter_id, portal, jurisdiction, number, year, title, source, added_at)
             VALUES ('mat-p-1', 'pjn', 'XYZ', 1, 2001, NULL, 'integration', 1)",
            [],
        )
        .unwrap();
        conn.execute("DELETE FROM knowledge_matter WHERE id = 'mat-p-1'", []).unwrap();
        let left: i64 = conn.query_row("SELECT count(*) FROM knowledge_matter_record", [], |row| row.get(0)).unwrap();
        assert_eq!(left, 0, "a record goes with its matter");
    }

    /// A person can choose a name (V29): every decision there was stays, as
    /// it was, and a decision of a name is admitted.
    #[test]
    fn a_base_before_the_names_chosen_keeps_its_decisions_and_admits_one() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for (index, migration) in MIGRATIONS.iter().enumerate().take(28) {
            conn.execute_batch(migration).unwrap();
            conn.pragma_update(None, "user_version", index as i64 + 1).unwrap();
        }
        conn.execute(
            "INSERT INTO knowledge_decision (kind, a, b, shown_evidence, shown_evidence_hash, state, decided_at, review, applied_as)
             VALUES ('separate', 'cluster:1:persona:a', 'cluster:2:persona:b', '[]', 'h', 'undone', 5, 'varias', '{}')",
            [],
        )
        .unwrap();
        assert!(conn
            .execute("INSERT INTO knowledge_decision (kind, a, b, shown_evidence, shown_evidence_hash, decided_at) VALUES ('name', 'x', 'name:y', '[]', 'h', 1)", [])
            .is_err());
        migrate(&mut conn).unwrap();
        assert_eq!(version(&conn).unwrap(), CURRENT_VERSION);
        let kept: (String, String, String, i64, String, String) = conn
            .query_row("SELECT kind, a, state, decided_at, review, applied_as FROM knowledge_decision", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
            })
            .unwrap();
        assert_eq!(kept, ("separate".into(), "cluster:1:persona:a".into(), "undone".into(), 5, "varias".into(), "{}".into()));
        conn.execute("INSERT INTO knowledge_decision (kind, a, b, shown_evidence, shown_evidence_hash, decided_at) VALUES ('name', 'x', 'name:y', '[]', 'h', 1)", [])
            .unwrap();
        let indexes: i64 = conn
            .query_row("SELECT count(*) FROM sqlite_master WHERE type = 'index' AND tbl_name = 'knowledge_decision'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(indexes, 2);
    }
}
