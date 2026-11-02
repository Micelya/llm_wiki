//! The record of failures (D-714): every failure of the background work,
//! local or of the model, in one table (`failure`), with the step where it
//! happened, what it is about, its cause and message, its class, its
//! reach, the attempt and its times.
//!
//! - **The class** (`classify`, the only function that gives it):
//!   transient (it is tried again), permanent (it is not, and is told as
//!   an error to correct) or unknown (it is not, and is told).
//! - **The reach:** one item, or a whole engine. A failure of the whole
//!   engine (its limit, its quota, a 429, no connection, the credential)
//!   stops the questions to that engine until its next attempt.
//! - **The attempts:** a transient one is tried again without end, after
//!   10 minutes, an hour, 6 hours and then once a day (`retry_after`).
//!   Past the threshold of attempts or time of the settings (3 or 6 hours
//!   by default, D-722), it is marked for the indicator, and the attempts
//!   go on.
//! - **The answer of another shape** is transient, and turns unknown after
//!   3 equal answers of the same engine to the same question.
//!
//! A failure stays open (`open`, or `person` when it waits for a person)
//! until the work it is about comes out well (`resolve`). What failed is
//! not done again over the same `basis` while it holds (`holds_sql`), and
//! a file with a failure of its ficha, its cut or its parts waits: it is
//! neither typed nor read (`file_waits_sql`, D-717).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::{now_ms, sql_err, DocBase};

/// The steps whose failure leaves a file waiting (D-717): its ficha of
/// shape, its cut (also by zones) and its parts.
pub const SHAPES_STAGE: &str = "shapes";
pub const CUT_QUESTION_STAGE: &str = "cut-question";
pub const DIGITAL_SIGNATURES_STAGE: &str = "digital-signatures";
/// An element that went back to an earlier step more times than the
/// process allows (D-774, point 2): it waits for a person.
pub const BACK_STAGE: &str = "process-back";
const FILE_WAITING_STAGES: &str = "'shapes', 'cut', 'parts', 'zones'";

/// The settings of the threshold of the indicator (D-722).
pub const INDICATOR_ATTEMPTS: &str = "failures:indicator-attempts";
pub const INDICATOR_HOURS: &str = "failures:indicator-hours";
const DEFAULT_ATTEMPTS: i64 = 3;
const DEFAULT_HOURS: i64 = 6;

/// How many equal answers of another shape turn the failure unknown.
const SAME_ANSWERS: i64 = 3;

/// The time now in SQL, in milliseconds.
pub const NOW_SQL: &str = "(CAST(strftime('%s', 'now') AS INTEGER) * 1000)";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    Transient,
    Permanent,
    Unknown,
}

impl Class {
    pub fn as_str(self) -> &'static str {
        match self {
            Class::Transient => "transient",
            Class::Permanent => "permanent",
            Class::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Only its item: a file, a document, a stretch of the cut.
    Item,
    /// The whole engine: it is not asked until the next attempt.
    Engine,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Item => "item",
            Scope::Engine => "engine",
        }
    }
}

/// A cause: its code, its class and its reach.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cause {
    pub code: &'static str,
    pub class: Class,
    pub scope: Scope,
}

const fn c(code: &'static str, class: Class, scope: Scope) -> Cause {
    Cause { code, class, scope }
}

use Class::{Permanent, Transient, Unknown};
use Scope::{Engine, Item};

/// Every cause the app knows (the table of A2 and the owner's four).
pub const CAUSES: &[Cause] = &[
    // The base does not answer: busy, full (with the indicator at once) or damaged.
    c("db-busy", Transient, Item),
    c("disk-full", Transient, Item),
    c("db-damaged", Permanent, Item),
    // A PDF pdfium cannot open: encrypted or broken; in use by another
    // program, a file of the cloud not downloaded, or not there now.
    c("pdf-encrypted", Permanent, Item),
    c("pdf-broken", Permanent, Item),
    c("file-locked", Transient, Item),
    c("cloud-not-downloaded", Transient, Item),
    c("file-unavailable", Transient, Item),
    // The reading by zones: another shape (a program or version error),
    // no memory, its model missing.
    c("zones-shape", Permanent, Item),
    c("out-of-memory", Transient, Item),
    c("zones-model-missing", Permanent, Item),
    // The scan waits for the models of the engine of this computer (E of
    // 1 nov): being downloaded, or with no way to download them.
    c("models-downloading", Transient, Item),
    c("models-missing", Permanent, Item),
    // Not a failure: the reader of the scans went back once to the one of
    // this computer (D-775, point 1); Atención tells it until a person
    // sees it.
    c("reader-back-to-local", Unknown, Item),
    // The model: its limit, quota or 429; no connection or no answer in
    // time; the engine not installed or not starting; the credential.
    c("rate-limit", Transient, Engine),
    c("no-connection", Transient, Engine),
    c("timeout", Transient, Engine),
    c("engine-missing", Permanent, Engine),
    c("credential", Permanent, Engine),
    // The engine that does not look at images: of the question of the cut
    // alone, it does not pause the engine for the others.
    c("engine-no-images", Permanent, Item),
    // One question: too large for the service, an answer of another shape.
    c("too-large", Permanent, Item),
    c("bad-answer", Transient, Item),
    // The folder of the file is not connected.
    c("folder-disconnected", Transient, Item),
    // Written before this record: tried again to learn its cause.
    c("before-record", Transient, Item),
    // An element that went back to an earlier step more than twice (D-774,
    // point 2): it waits for a person.
    c("goes-back", Unknown, Item),
    // An error of the program (a panic) and anything else.
    c("program-error", Unknown, Item),
    c("unknown", Unknown, Item),
];

/// The cause of a code; an unknown code is `unknown`.
pub fn cause(code: &str) -> Cause {
    CAUSES.iter().copied().find(|c| c.code == code).unwrap_or(c("unknown", Unknown, Item))
}

/// The class of a failure from its message: the only function that
/// classifies (D-714). The words are those of the providers, of SQLite, of
/// pdfium and of Windows; the order matters (a question too large before
/// the limit, the engine missing before a file not found).
pub fn classify(message: &str) -> Cause {
    let m = message.to_lowercase();
    let any = |words: &[&str]| words.iter().any(|w| m.contains(w));
    let code = if any(&["413", "too large", "too long", "context length", "context_length", "maximum context", "context window"]) {
        "too-large"
    } else if any(&["usage limit", "hit your limit", "rate limit", "rate_limit", "429", "quota", "too many requests"]) {
        "rate-limit"
    } else if any(&[
        "401",
        "403",
        "unauthorized",
        "forbidden",
        "api key",
        "api_key",
        "invalid key",
        "authentication",
        "not logged in",
        "log in",
        "login",
        "credential",
    ]) {
        "credential"
    } else if any(&["not found on path", "failed to start", "not installed", "no está instalado"]) {
        "engine-missing"
    } else if any(&["did not answer within", "timed out", "timeout"]) {
        "timeout"
    } else if any(&[
        "no connection",
        "connection",
        "network",
        "dns",
        "could not resolve",
        "error sending request",
        "offline",
        "unreachable",
        "stream disconnected",
    ]) {
        "no-connection"
    } else if any(&["database is locked", "database is busy", "database table is locked", "sqlite_busy"]) {
        "db-busy"
    } else if any(&["database or disk is full", "disk full", "no space left", "os error 112"]) {
        "disk-full"
    } else if any(&["malformed", "not a database", "database disk image", "corrupt"]) {
        "db-damaged"
    } else if any(&["passworderror", "password"]) {
        "pdf-encrypted"
    } else if any(&["os error 32", "os error 33", "used by another process", "sharing violation"]) {
        "file-locked"
    } else if any(&["os error 362", "cloud file", "cloud operation", "0x8007016a", "not downloaded"]) {
        "cloud-not-downloaded"
    } else if any(&["formaterror", "securityerror", "pageerror", "not a pdf", "invalid pdf"]) {
        "pdf-broken"
    } else if any(&["fileerror", "os error 2)", "os error 3)", "cannot find the", "no such file", "not found"]) {
        "file-unavailable"
    } else if any(&["out of memory", "memory allocation"]) {
        "out-of-memory"
    } else if any(&["not of the shape expected"]) {
        "zones-shape"
    } else if any(&["internal error in", "panicked"]) {
        "program-error"
    } else {
        "unknown"
    };
    cause(code)
}

/// How long after its attempt `attempt` (1 the first) a transient failure
/// is tried again, in milliseconds: 10 minutes, an hour, 6 hours, then
/// once a day, without end.
#[cfg(test)]
pub fn retry_after(attempt: i64) -> i64 {
    retry_after_with(&super::thresholds::Thresholds::default().retry_minutes, attempt)
}

/// The same, with the waits of the configuration (`thresholds.rs`), in
/// minutes: past the last, the last again.
pub fn retry_after_with(minutes: &[i64], attempt: i64) -> i64 {
    let index = (attempt.max(1) - 1) as usize;
    minutes.get(index).or(minutes.last()).copied().unwrap_or(24 * 60) * 60 * 1000
}

/// The error of pdfium over a file, with what Windows says of it when
/// pdfium only says it could not open it: in use by another program, a
/// file of the cloud not downloaded, or not there. Then `classify` tells
/// them apart from a broken or encrypted PDF.
pub fn pdf_error_detail(path: &std::path::Path, error: String) -> String {
    let lower = error.to_lowercase();
    if !(lower.contains("fileerror") || lower.contains("failed to open")) || lower.contains("passworderror") || lower.contains("formaterror") {
        return error;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Offline, recall on open, recall on data access: a placeholder of
        // a folder synchronized with the cloud.
        const CLOUD: u32 = 0x0000_1000 | 0x0004_0000 | 0x0040_0000;
        if std::fs::metadata(path).is_ok_and(|m| m.file_attributes() & CLOUD != 0) {
            return format!("{error} (cloud file not downloaded)");
        }
    }
    match std::fs::File::open(path) {
        Err(io) => format!("{error} ({io})"),
        Ok(_) => error,
    }
}

/// What a failure is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum About {
    /// A file, by its print (hash).
    File(String),
    /// A document or version.
    Version(i64),
    /// A stretch of the question of the cut: a file and the page after
    /// which its step is.
    Span { file: String, after: u32 },
    /// A window of the cut by zones: a file and its first page (P-535).
    Window { file: String, first: u32 },
    /// One question to the model about no document (the summary of a page
    /// of an entity), by its key (P-533).
    Question(String),
    /// A whole engine, by its id.
    Engine(String),
    /// A folder (origin) of the base.
    Origin(String),
    /// A thread of the background work, or the step of a thread as a whole.
    Thread(String),
}

impl About {
    fn kind(&self) -> &'static str {
        match self {
            About::File(_) => "file",
            About::Version(_) => "version",
            About::Span { .. } | About::Window { .. } => "span",
            About::Question(_) => "question",
            About::Engine(_) => "engine",
            About::Origin(_) => "origin",
            About::Thread(_) => "thread",
        }
    }

    fn item(&self) -> String {
        match self {
            About::File(hash) => hash.clone(),
            About::Version(id) => id.to_string(),
            About::Span { file, after } => format!("{file}#{after}"),
            About::Window { file, first } => format!("{file}#v{first}"),
            About::Engine(id) | About::Origin(id) | About::Thread(id) | About::Question(id) => id.clone(),
        }
    }

    fn file(&self) -> Option<&str> {
        match self {
            About::File(hash) | About::Span { file: hash, .. } | About::Window { file: hash, .. } => Some(hash),
            _ => None,
        }
    }
}

/// A failure to write down.
#[derive(Debug, Clone)]
pub struct Failure<'a> {
    pub stage: &'a str,
    pub about: About,
    pub cause: Cause,
    pub message: &'a str,
    /// What the work was done over: it is not done again over it while the
    /// failure holds.
    pub basis: Option<&'a str>,
    /// The engine asked, for a failure of the model.
    pub engine: Option<&'a str>,
    /// The answer of another shape, to count the equal ones.
    pub answer: Option<&'a str>,
}

impl<'a> Failure<'a> {
    /// A failure classified by its message.
    pub fn new(stage: &'a str, about: About, message: &'a str) -> Self {
        Failure { stage, about, cause: classify(message), message, basis: None, engine: None, answer: None }
    }
}

/// A failure as it was written down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    pub id: i64,
    pub class: Class,
    pub scope: Scope,
    pub attempt: i64,
    pub next_at: Option<i64>,
}

fn setting_number(conn: &Connection, key: &str, default: i64) -> Result<i64, String> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [key], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(value.and_then(|v| v.trim().parse().ok()).filter(|n: &i64| *n > 0).unwrap_or(default))
}

/// The threshold of the indicator: attempts and hours (D-722).
pub fn indicator_threshold(conn: &Connection) -> Result<(i64, i64), String> {
    Ok((setting_number(conn, INDICATOR_ATTEMPTS, DEFAULT_ATTEMPTS)?, setting_number(conn, INDICATOR_HOURS, DEFAULT_HOURS)?))
}

/// Writes down a failure now.
pub fn record(conn: &Connection, failure: &Failure<'_>) -> Result<Recorded, String> {
    record_at(conn, failure, now_ms())
}

/// Writes down a failure at `now`: a new one, or one more attempt of the
/// one still open about the same thing in the same step.
pub fn record_at(conn: &Connection, failure: &Failure<'_>, now: i64) -> Result<Recorded, String> {
    let kind = failure.about.kind();
    let item = failure.about.item();
    let answer_hash = failure.answer.map(|a| {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(a.trim().as_bytes()))
    });
    let open: Option<(i64, i64, i64, Option<String>, i64)> = conn
        .query_row(
            "SELECT id, attempt, first_at, answer_hash, same_answers FROM failure
             WHERE stage = ?1 AND item_kind = ?2 AND item = ?3 AND state <> 'resolved'",
            params![failure.stage, kind, item],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let (attempt, first_at, same) = match &open {
        Some((_, attempt, first_at, kept, same)) => {
            let same = match &answer_hash {
                Some(hash) if kept.as_ref() == Some(hash) => same + 1,
                Some(_) => 1,
                None => 0,
            };
            (attempt + 1, *first_at, same)
        }
        None => (1, now, i64::from(answer_hash.is_some())),
    };
    let mut cause = failure.cause;
    if cause.code == "bad-answer" && same >= SAME_ANSWERS {
        cause.class = Class::Unknown;
    }
    let waits = super::thresholds::retry_minutes(conn)?;
    let next_at = (cause.class == Class::Transient).then(|| now + retry_after_with(&waits, attempt));
    let (attempts, hours) = indicator_threshold(conn)?;
    let indicator = cause.class != Class::Transient
        || cause.code == "disk-full"
        || attempt >= attempts
        || now - first_at >= hours * 3600 * 1000;
    let state = if cause.class == Class::Transient { "open" } else { "person" };
    let to_diagnose = cause.class != Class::Transient;
    let version_id = match &failure.about {
        About::Version(id) => {
            let exists: bool = conn
                .query_row("SELECT EXISTS (SELECT 1 FROM version WHERE id = ?1)", [id], |row| row.get(0))
                .map_err(sql_err)?;
            exists.then_some(*id)
        }
        _ => None,
    };
    let file_hash = match failure.about.file() {
        Some(hash) => {
            let exists: bool =
                conn.query_row("SELECT EXISTS (SELECT 1 FROM file WHERE hash = ?1)", [hash], |row| row.get(0)).map_err(sql_err)?;
            exists.then(|| hash.to_string())
        }
        None => None,
    };
    let message: String = failure.message.chars().take(2000).collect();
    let id = match open {
        Some((id, ..)) => {
            conn.execute(
                "UPDATE failure SET cause = ?2, message = ?3, class = ?4, scope = ?5, engine = coalesce(?6, engine), attempt = ?7,
                        answer_hash = ?8, same_answers = ?9, last_at = ?10, next_at = ?11, state = ?12, indicator = ?13,
                        to_diagnose = ?14, basis = ?15
                 WHERE id = ?1",
                params![
                    id,
                    cause.code,
                    message,
                    cause.class.as_str(),
                    cause.scope.as_str(),
                    failure.engine,
                    attempt,
                    answer_hash,
                    same,
                    now,
                    next_at,
                    state,
                    indicator,
                    to_diagnose,
                    failure.basis
                ],
            )
            .map_err(sql_err)?;
            id
        }
        None => {
            conn.execute(
                "INSERT INTO failure (stage, item_kind, item, file_hash, version_id, basis, cause, message, class, scope, engine,
                                      attempt, answer_hash, same_answers, first_at, last_at, next_at, state, indicator, to_diagnose)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12, ?13, ?14, ?14, ?15, ?16, ?17, ?18)",
                params![
                    failure.stage,
                    kind,
                    item,
                    file_hash,
                    version_id,
                    failure.basis,
                    cause.code,
                    message,
                    cause.class.as_str(),
                    cause.scope.as_str(),
                    failure.engine,
                    answer_hash,
                    same,
                    now,
                    next_at,
                    state,
                    indicator,
                    to_diagnose
                ],
            )
            .map_err(sql_err)?;
            conn.last_insert_rowid()
        }
    };
    Ok(Recorded { id, class: cause.class, scope: cause.scope, attempt, next_at })
}

/// The work of `stage` over `about` came out well: its failure is resolved.
pub fn resolve(conn: &Connection, stage: &str, about: &About) -> Result<(), String> {
    conn.execute(
        "UPDATE failure SET state = 'resolved', next_at = NULL WHERE stage = ?1 AND item_kind = ?2 AND item = ?3 AND state <> 'resolved'",
        params![stage, about.kind(), about.item()],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Whether a failure of `stage` over `about` is still open.
pub fn is_open(conn: &Connection, stage: &str, about: &About) -> Result<bool, String> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM failure WHERE stage = ?1 AND item_kind = ?2 AND item = ?3 AND state <> 'resolved')",
        params![stage, about.kind(), about.item()],
        |row| row.get(0),
    )
    .map_err(sql_err)
}

/// Writes down a failure only when none is open about the same thing in
/// the same step: for what is looked at again and again without trying
/// anything new.
pub fn note_once(conn: &Connection, failure: &Failure<'_>) -> Result<(), String> {
    if !is_open(conn, failure.stage, &failure.about)? {
        record(conn, failure)?;
    }
    Ok(())
}

/// What kept every failure of one cause is gone (an engine that looks at
/// images was chosen): they are resolved.
pub fn resolve_cause(conn: &Connection, cause: &str) -> Result<(), String> {
    conn.execute("UPDATE failure SET state = 'resolved', next_at = NULL WHERE cause = ?1 AND state <> 'resolved'", [cause])
        .map_err(sql_err)?;
    Ok(())
}

/// What kept the failures of one cause in one step is gone (the models
/// of this computer are there now): they are resolved.
pub fn resolve_stage_cause(conn: &Connection, stage: &str, cause: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE failure SET state = 'resolved', next_at = NULL WHERE stage = ?1 AND cause = ?2 AND state <> 'resolved'",
        params![stage, cause],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// An engine answered: its failures of the whole engine are resolved.
pub fn resolve_engine(conn: &Connection, engine: &str) -> Result<(), String> {
    conn.execute(
        "UPDATE failure SET state = 'resolved', next_at = NULL
         WHERE item_kind = 'engine' AND item = ?1 AND scope = 'engine' AND state <> 'resolved'",
        [engine],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Until when an engine is not asked: its next attempt after a failure of
/// the whole engine, `i64::MAX` while a permanent one waits for a person;
/// `None` when it can be asked.
pub fn engine_paused_until(conn: &Connection, engine: &str) -> Result<Option<i64>, String> {
    engine_paused_at(conn, engine, now_ms())
}

pub fn engine_paused_at(conn: &Connection, engine: &str, now: i64) -> Result<Option<i64>, String> {
    let found: Option<Option<i64>> = conn
        .query_row(
            "SELECT max(coalesce(next_at, 9223372036854775807)) FROM failure
             WHERE item_kind = 'engine' AND item = ?1 AND scope = 'engine' AND state <> 'resolved'",
            [engine],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?;
    Ok(found.flatten().filter(|until| *until > now))
}

/// SQL: the work of `stage` over `item` (an SQL text) failed over `basis`
/// (an SQL value) and its failure holds: permanent or unknown, or a
/// transient one whose next attempt has not come. It is not done again.
pub fn holds_sql(stage: &str, item: &str, basis: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM failure fx WHERE fx.stage = '{stage}' AND fx.item = {item} AND fx.state <> 'resolved'
                 AND fx.basis = {basis} AND (fx.next_at IS NULL OR fx.next_at > {NOW_SQL}))"
    )
}

/// SQL: a failure of `stage` over `item` (an SQL text), over whatever
/// basis, holds: it is not tried again until its next attempt.
pub fn holds_any_sql(stage: &str, item: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM failure fy WHERE fy.stage = '{stage}' AND fy.item = {item} AND fy.state <> 'resolved'
                 AND (fy.next_at IS NULL OR fy.next_at > {NOW_SQL}))"
    )
}

/// SQL: the file `hash` (an SQL text) waits for a failure of its ficha,
/// its cut or its parts (D-717): it is neither typed nor read, whole or in
/// parts.
pub fn file_waits_sql(hash: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM failure fw WHERE fw.file_hash = {hash} AND fw.state <> 'resolved'
                 AND fw.stage IN ({FILE_WAITING_STAGES}))"
    )
}

/// The stage of the record of a question to the model, by its task: the
/// seven steps of the questions (P-533). The question of the cut keeps its
/// own (`CUT_QUESTION_STAGE`).
pub fn question_stage(task: &str) -> &'static str {
    match task {
        "type" => "types",
        "data" => "data",
        super::similar::naming::FIELDS_TASK => "templates",
        "knowledge" => "reading",
        "identity" => "resolve",
        "matter" => "matters",
        _ => "pages",
    }
}

/// What a question is about in the record: its document, or the question
/// itself when it is about none (the summary of a page of an entity).
pub(crate) fn question_about(content_id: i64, key: &str) -> About {
    if content_id > 0 {
        About::Version(content_id)
    } else {
        About::Question(key.to_string())
    }
}

/// Whether a failure of `stage` over `about` holds now: permanent or
/// unknown (they are not tried again by themselves, D-714), or transient
/// before its next attempt. A document set aside for going back too many
/// times (`BACK_STAGE`) is held in every step.
pub fn holds(conn: &Connection, stage: &str, about: &About) -> Result<bool, String> {
    conn.query_row(
        &format!(
            "SELECT EXISTS (SELECT 1 FROM failure WHERE state <> 'resolved' AND item_kind = ?2 AND item = ?3
                 AND ((stage = ?1 AND (next_at IS NULL OR next_at > {NOW_SQL})) OR stage = '{BACK_STAGE}'))"
        ),
        params![stage, about.kind(), about.item()],
        |row| row.get(0),
    )
    .map_err(sql_err)
}

/// The documents a failure of `stage` holds now (`holds`), with those set
/// aside for going back too many times.
pub fn held_versions(conn: &Connection, stage: &str) -> Result<std::collections::BTreeSet<i64>, String> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT DISTINCT CAST(item AS INTEGER) FROM failure WHERE state <> 'resolved' AND item_kind = 'version'
               AND ((stage = ?1 AND (next_at IS NULL OR next_at > {NOW_SQL})) OR stage = '{BACK_STAGE}')"
        ))
        .map_err(sql_err)?;
    let rows = statement.query_map([stage], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// The earliest next attempt still to come of any failure, or of a pause
/// of an engine: when the process wakes up by itself.
pub fn next_attempt(conn: &Connection) -> Result<Option<i64>, String> {
    conn.query_row(
        &format!("SELECT min(next_at) FROM failure WHERE state <> 'resolved' AND next_at IS NOT NULL AND next_at > {NOW_SQL}"),
        [],
        |row| row.get(0),
    )
    .map_err(sql_err)
}

/// A failure still open, as «Atención» shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FailureView {
    pub id: i64,
    pub stage: String,
    pub item_kind: String,
    pub item: String,
    pub cause: String,
    pub message: String,
    pub class: String,
    pub scope: String,
    pub engine: Option<String>,
    pub attempt: i64,
    pub first_at: i64,
    pub last_at: i64,
    pub next_at: Option<i64>,
    pub state: String,
    pub indicator: bool,
    pub to_diagnose: bool,
    /// The file it is about, by its first place: its path in its folder.
    pub path: Option<String>,
    pub location_id: Option<i64>,
}

impl DocBase {
    /// The failures still open, the newest first.
    pub fn failures_open(&self) -> Result<Vec<FailureView>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(
                "SELECT x.id, x.stage, x.item_kind, x.item, x.cause, x.message, x.class, x.scope, x.engine, x.attempt,
                        x.first_at, x.last_at, x.next_at, x.state, x.indicator, x.to_diagnose,
                        (SELECT l.rel_path FROM location l WHERE l.file_hash = coalesce(x.file_hash, f.hash) ORDER BY l.rel_path LIMIT 1),
                        (SELECT l.id FROM location l WHERE l.file_hash = coalesce(x.file_hash, f.hash) ORDER BY l.rel_path LIMIT 1)
                 FROM failure x LEFT JOIN file f ON x.file_hash IS NULL AND f.version_id = x.version_id
                 WHERE x.state <> 'resolved'
                 GROUP BY x.id ORDER BY x.last_at DESC, x.id DESC",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok(FailureView {
                    id: row.get(0)?,
                    stage: row.get(1)?,
                    item_kind: row.get(2)?,
                    item: row.get(3)?,
                    cause: row.get(4)?,
                    message: row.get(5)?,
                    class: row.get(6)?,
                    scope: row.get(7)?,
                    engine: row.get(8)?,
                    attempt: row.get(9)?,
                    first_at: row.get(10)?,
                    last_at: row.get(11)?,
                    next_at: row.get(12)?,
                    state: row.get(13)?,
                    indicator: row.get(14)?,
                    to_diagnose: row.get(15)?,
                    path: row.get(16)?,
                    location_id: row.get(17)?,
                })
            })
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)
    }

    /// Writes down a failure of the background work that otherwise went
    /// only to the console, which goes on as it was. A failure of the
    /// record itself goes to the console too: it never stops the work.
    pub(crate) fn note(&self, failure: &Failure<'_>) -> Option<Recorded> {
        match record(&self.conn(), failure) {
            Ok(recorded) => Some(recorded),
            Err(error) => {
                eprintln!("[docbase] the failure could not be written down: {error}");
                None
            }
        }
    }

    /// A question of one of the seven steps of the questions failed
    /// (P-533, D-714): written down over its document (or over the
    /// question, when it is about none) with its class and its waits; one
    /// of the whole engine pauses that engine. `answer`: the answer of
    /// another shape, which counts the equal ones.
    pub(crate) fn question_failed(&self, task: &str, content_id: i64, key: &str, engine: &str, error: &str, answer: Option<&str>) {
        let stage = question_stage(task);
        let mut failure = Failure::new(stage, question_about(content_id, key), error);
        if answer.is_some() {
            failure.cause = cause("bad-answer");
            failure.answer = answer;
        }
        if failure.cause.scope == Scope::Engine {
            failure.about = About::Engine(engine.to_string());
        }
        failure.engine = Some(engine);
        self.note(&failure);
    }

    /// A question of those steps was answered: its failure, and those of
    /// its engine, are resolved.
    pub(crate) fn question_answered(&self, task: &str, content_id: i64, key: &str, engine: &str) {
        let conn = self.conn();
        let about = question_about(content_id, key);
        if let Err(error) = resolve(&conn, question_stage(task), &about).and_then(|_| resolve_engine(&conn, engine)) {
            eprintln!("[docbase] {error}");
        }
    }

    /// Whether the question is held by a failure now (`holds`): it is not
    /// asked, and does not take a place in the run.
    pub(crate) fn question_held(&self, task: &str, content_id: i64, key: &str) -> bool {
        holds(&self.conn(), question_stage(task), &question_about(content_id, key)).unwrap_or(false)
    }
}

#[cfg(test)]
#[path = "failures_tests.rs"]
mod tests;
