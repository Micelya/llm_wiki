//! How far the processing of the archive went, stage by stage (D-424):
//! what the attention view shows in «En curso», the line at the foot of
//! the window and the figure «En curso» of Inicio.
//!
//! The figures are counted again on every request from what the base
//! already keeps, without writing anything. What the engine is doing now
//! (the origin it sweeps, the file it reads, the stage its questions to
//! the model are at and the document of the question in flight) and the
//! last error of the model live in memory (`EngineState`): they only
//! matter while the app runs.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::time::Duration;

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::query::rows_sql;
use super::{now_ms, origins, sql_err, DocBase, Visible};

/// The stages, in the order a document goes through them; `templates`
/// goes over the templates of the families, naming their fields (D-488).
pub const STAGES: [&str; 10] = ["origins", "text", "recognition", "similar", "types", "templates", "reading", "resolve", "matters", "pages"];

/// The stages that ask the model, in the order the engine asks
/// (`worker::ask_step`); they run only while the automatic reading is on.
pub const MODEL_STAGES: [&str; 6] = ["types", "templates", "reading", "resolve", "matters", "pages"];

/// The stages whose work left the base cannot count by itself: the
/// matters the model proposes and the pages it writes depend on all the
/// stages before, and how many questions they have left is known only
/// when the engine goes through them. Until the engine's first pass
/// through each since the app opened, it is not shown as done: it waits
/// for that first pass (D-452).
pub const FIRST_LAP_STAGES: [&str; 2] = ["matters", "pages"];

/// The stage a question to the model belongs to, by its task.
pub fn stage_of_task(task: &str) -> &'static str {
    match task {
        "type" | "data" => "types",
        super::similar::naming::FIELDS_TASK => "templates",
        "knowledge" => "reading",
        "identity" => "resolve",
        "matter" => "matters",
        _ => "pages",
    }
}

/// What kind of failure an error of the model is, from its message:
/// `limit` (the provider's usage limit), `key` (the key or the session was
/// rejected), `connection` (no connection, or no answer in time) or
/// `other`.
pub fn error_kind(message: &str) -> &'static str {
    let message = message.to_lowercase();
    let any = |words: &[&str]| words.iter().any(|w| message.contains(w));
    if any(&["usage limit", "hit your limit", "rate limit", "rate_limit", "429", "quota", "too many requests"]) {
        "limit"
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
        "key"
    } else if any(&[
        "connection",
        "network",
        "timed out",
        "timeout",
        "dns",
        "could not resolve",
        "error sending request",
        "offline",
        "unreachable",
        "did not answer within",
        "stream disconnected",
    ]) {
        "connection"
    } else {
        "other"
    }
}

/// Longest message of an error kept for the screens.
const ERROR_CHARS: usize = 400;

/// The last error of the model, as the screens show it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelError {
    /// The stage that asked; `None` when the model could not even be started.
    pub stage: Option<String>,
    /// `limit`, `key`, `connection` or `other` (`error_kind`).
    pub kind: String,
    pub message: String,
    pub at: i64,
    /// When the engine tries again.
    pub retry_at: Option<i64>,
}

/// Something a stage goes over: an origin, a file, a document or a page.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Key {
    Origin(String),
    Location(i64),
    Content(i64),
    Page(String),
}

/// What the engine is doing now, as last told.
#[derive(Debug, Clone, Default)]
pub(crate) struct EngineNow {
    /// The origin being swept.
    pub sweeping: Option<String>,
    /// The file whose bytes, or the document whose text, is being read.
    pub reading_file: Option<Key>,
    /// The stage the questions to the model are at; `None` while there is
    /// nothing to ask (the automatic reading is off, or no model).
    pub model_stage: Option<&'static str>,
    /// The document of the question in flight, when it is about one.
    pub asking: Option<i64>,
    /// Questions each stage had left at its last run.
    pub left: BTreeMap<&'static str, usize>,
    pub last_error: Option<ModelError>,
    /// Questions of «quién es quién» and of the summaries asked by
    /// themselves since their round began (`auto_questions.rs`, D-484).
    pub auto_asked: usize,
    /// The stage whose questions wait for a person: the round passed its
    /// cap, or a person left them for later (D-484).
    pub auto_held: Option<&'static str>,
    /// The last question the thread asked by itself about a doubtful step
    /// of a cut failed: the reading does not wait for those answers until
    /// one comes (`cut::waiting_for_cut_answers`, D-488).
    pub cut_asking_failed: bool,
    /// When the round began: the last round's end, or the app's opening.
    /// The questions the base keeps from then on are the round's (D-606).
    pub round_since: i64,
    /// The document whose text is being recognized now (`recognize.rs`).
    /// What a person asked to recognize is kept in the base
    /// (`recognition_request`), so it survives closing the app.
    pub recognizing: Option<Recognizing>,
    /// How many times the provider failed on each document (its limit, no
    /// connection) since it was last asked for.
    pub recognition_tries: BTreeMap<i64, usize>,
    /// How many pages each document waiting for recognition needs read,
    /// as the worker counted them: the size of the work, before it starts.
    pub scan_pages: BTreeMap<i64, usize>,
}

/// The document being recognized and the page it is at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Recognizing {
    pub content: i64,
    /// 1-based, among the pages that need recognition; 0 until known.
    pub page: usize,
    pub total: usize,
}

/// What the engine is doing now and the last error of the model, kept in
/// memory by the threads of the worker (`worker.rs`) and the questions to
/// the model (`classify::ask_and_keep`).
#[derive(Debug)]
pub struct EngineState {
    now: Mutex<EngineNow>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self { now: Mutex::new(EngineNow { round_since: now_ms(), ..EngineNow::default() }) }
    }
}

impl EngineState {
    fn with<T>(&self, change: impl FnOnce(&mut EngineNow) -> T) -> T {
        let mut now = self.now.lock().unwrap_or_else(|p| p.into_inner());
        change(&mut now)
    }

    pub(crate) fn snapshot(&self) -> EngineNow {
        self.with(|now| now.clone())
    }

    /// The origin the worker sweeps now; `None` when the sweep ends.
    pub(crate) fn sweeping(&self, origin_id: Option<&str>) {
        self.with(|now| now.sweeping = origin_id.map(str::to_string));
    }

    /// The file (or document) whose bytes or text the worker reads now.
    pub(crate) fn reading_file(&self, item: Option<Key>) {
        self.with(|now| now.reading_file = item);
    }

    /// The stage the questions to the model went on to; `None`: nothing to ask.
    pub(crate) fn at_stage(&self, stage: Option<&'static str>) {
        self.with(|now| {
            now.model_stage = stage;
            if stage.is_none() {
                now.asking = None;
            }
        });
    }

    /// A question to the model is about to be sent.
    pub(crate) fn asking(&self, task: &str, content_id: i64) {
        self.with(|now| {
            now.model_stage = Some(stage_of_task(task));
            now.asking = (content_id > 0).then_some(content_id);
        });
    }

    /// The question in flight came back; an answer leaves the last error behind.
    pub(crate) fn asked(&self, answered: bool) {
        self.with(|now| {
            now.asking = None;
            if answered {
                now.last_error = None;
            }
        });
    }

    /// How many questions a stage had left when its run ended.
    pub(crate) fn ran(&self, stage: &'static str, left: usize) {
        self.with(|now| {
            now.left.insert(stage, left);
        });
    }

    /// Questions of the round of automatic questions just asked (D-484).
    pub(crate) fn auto_asked(&self, asked: usize) {
        self.with(|now| now.auto_asked += asked);
    }

    /// The stage whose questions wait for a person; `None`: none waits.
    pub(crate) fn auto_held(&self, stage: Option<&'static str>) {
        self.with(|now| now.auto_held = stage);
    }

    /// Whether the last question about a doubtful step of a cut, asked by
    /// the thread by itself, failed.
    pub(crate) fn cut_asking_failed(&self, failed: bool) {
        self.with(|now| now.cut_asking_failed = failed);
    }

    /// The round of automatic questions ended: nothing left to ask.
    pub(crate) fn auto_round_over(&self) {
        self.with(|now| {
            now.auto_asked = 0;
            now.auto_held = None;
            now.round_since = now_ms();
        });
    }

    /// The document whose text is recognized now; `None` when it ends.
    pub(crate) fn recognizing(&self, content: Option<i64>) {
        self.with(|now| now.recognizing = content.map(|content| Recognizing { content, page: 0, total: 0 }));
    }

    /// The page of the document being recognized the engine reads now.
    pub(crate) fn recognizing_page(&self, page: usize, total: usize) {
        self.with(|now| {
            if let Some(recognizing) = now.recognizing.as_mut() {
                recognizing.page = page;
                recognizing.total = total;
            }
        });
    }

    /// A person asked again to recognize these documents: the failures of
    /// the provider on them are forgotten.
    pub(crate) fn forget_recognition_tries(&self, contents: &[i64]) {
        self.with(|now| {
            for content in contents {
                now.recognition_tries.remove(content);
            }
        });
    }

    /// The provider failed on a document once more; how many times now.
    pub(crate) fn recognition_tried(&self, content: i64) -> usize {
        self.with(|now| {
            let tries = now.recognition_tries.entry(content).or_default();
            *tries += 1;
            *tries
        })
    }

    /// The engine read a document: the last error of the model is behind.
    pub(crate) fn recognized(&self) {
        self.with(|now| now.last_error = None);
    }

    /// The last error of the model is forgotten if `stage` asked it.
    pub(crate) fn forget_error_of(&self, stage: &str) {
        self.with(|now| {
            if now.last_error.as_ref().is_some_and(|error| error.stage.as_deref() == Some(stage)) {
                now.last_error = None;
            }
        });
    }

    /// How many pages of a document wait for recognition.
    pub(crate) fn scan_pages(&self, content: i64, pages: usize) {
        self.with(|now| {
            now.scan_pages.insert(content, pages);
        });
    }

    /// The model failed in `stage` (`None`: it could not be started); the
    /// engine tries again after `retry_in`.
    pub(crate) fn failed(&self, stage: Option<&'static str>, message: &str, retry_in: Option<Duration>) {
        let at = now_ms();
        let message: String = message.trim().chars().take(ERROR_CHARS).collect();
        self.with(|now| {
            now.asking = None;
            now.last_error = Some(ModelError {
                stage: stage.map(str::to_string),
                kind: error_kind(&message).to_string(),
                message,
                at,
                retry_at: retry_in.map(|wait| at + wait.as_millis() as i64),
            });
        });
    }
}

/// What part of the archive the figures are about: one origin, or one
/// folder of it; nothing, the whole archive.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Scope {
    pub origin_id: Option<String>,
    /// A folder inside the origin, with forward slashes; empty, the whole origin.
    pub folder: Option<String>,
}

impl Scope {
    fn origin(&self) -> Option<&str> {
        self.origin_id.as_deref().filter(|id| !id.is_empty())
    }

    fn folder(&self) -> &str {
        self.folder.as_deref().map(|f| f.trim_matches('/')).unwrap_or("")
    }

    pub fn is_whole(&self) -> bool {
        self.origin().is_none()
    }

    /// The documents of the scope (its files not left out).
    pub(crate) fn contents(&self, conn: &Connection) -> Result<BTreeSet<i64>, String> {
        let (condition, bound) = self.location_sql("l.origin_id", "l.folder");
        let mut statement = conn
            .prepare(&format!(
                "SELECT DISTINCT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash
                 WHERE NOT {} AND {condition}",
                super::excluded_sql!()
            ))
            .map_err(sql_err)?;
        let rows = statement.query_map(params_from_iter(bound), |row| row.get::<_, i64>(0)).map_err(sql_err)?;
        rows.collect::<Result<BTreeSet<i64>, _>>().map_err(sql_err)
    }

    /// SQL condition on the origin and folder columns of a location, with
    /// its parameters.
    fn location_sql(&self, origin_column: &str, folder_column: &str) -> (String, Vec<Value>) {
        let Some(origin) = self.origin() else {
            return ("1 = 1".to_string(), Vec::new());
        };
        let folder = self.folder();
        if folder.is_empty() {
            return (format!("{origin_column} = ?"), vec![Value::Text(origin.to_string())]);
        }
        (
            format!("{origin_column} = ? AND ({folder_column} = ? OR substr({folder_column}, 1, length(?) + 1) = ? || '/')"),
            vec![
                Value::Text(origin.to_string()),
                Value::Text(folder.to_string()),
                Value::Text(folder.to_string()),
                Value::Text(folder.to_string()),
            ],
        )
    }
}

/// One stage of the processing, with its figures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub id: &'static str,
    /// `done`, `running`, `waiting` (its turn), `paused` (the automatic
    /// reading is off, or no model or engine was chosen) or `error`.
    pub state: &'static str,
    /// What it counts: `origins`, `files`, `documents`, `templates` or `pages`.
    pub unit: &'static str,
    /// What it goes over: done, pending and failed.
    pub total: usize,
    pub done: usize,
    pub pending: usize,
    pub failed: usize,
    /// Left out of the stage (too large, a format with no text, a cloud
    /// shortcut; a document of no matter), not counted in `total`.
    pub out: usize,
    /// Work to do that no document stands for: a decision to apply, the
    /// pages to compute again.
    pub other_work: bool,
    /// What the engine is on now, when it is known.
    pub current: Option<StageItem>,
    /// The last error of the model, on the stage it happened in.
    pub error: Option<ModelError>,
    /// Figures of its own: `questions` left at its last run, `grouped` and
    /// `loose` documents of the matters; for the scans, the `pages` that
    /// wait, the documents whose pages are not counted yet (`uncounted`)
    /// and those a person asked for (`requested`); `firstLap` (1) while a
    /// stage of `FIRST_LAP_STAGES` waits for the engine's first pass.
    pub extra: BTreeMap<&'static str, usize>,
}

/// One thing a stage goes over, as its list shows it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageItem {
    /// `pending`, `failed`, `done` or `out`.
    pub status: &'static str,
    pub location_id: Option<i64>,
    pub content_id: Option<i64>,
    pub origin_id: Option<String>,
    /// For a page: its path, to open it.
    pub page_path: Option<String>,
    pub name: String,
    /// Where it is: the origin and the folder, or the origin's folder on disk.
    pub place: String,
    /// The reason of a failure, why it is out, or the matter of a document.
    pub detail: Option<String>,
    /// Of a scan being recognized: the page the engine reads now, among
    /// those that need it.
    pub page: Option<usize>,
    /// Of a scan: how many pages need recognition, once counted.
    pub pages: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StageItems {
    pub stage: String,
    /// Failed first, then pending, then done, then out; at most the limit.
    pub items: Vec<StageItem>,
    pub total: usize,
}

/// Every stage and what it means for the screens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stages: Vec<Stage>,
    /// Documents with something still to do in a stage that runs or waits
    /// its turn; a file not read yet counts as one.
    pub in_process: usize,
    /// Documents waiting because the automatic reading is paused or no
    /// model (or no engine for the scans) was chosen.
    pub paused: usize,
    pub background_reading: bool,
    pub engine_chosen: bool,
    /// The engine that recognizes the text of scans, as it names what it
    /// reads; `None` when none was chosen.
    pub recognition_engine: Option<String>,
    pub last_error: Option<ModelError>,
    /// The questions about the cuts, of «quién es quién» and of the
    /// summaries that go by themselves: how many, whether they wait for a
    /// person (D-484), and what every question since the round began was
    /// for (D-606).
    pub auto_questions: super::auto_questions::AutoQuestions,
}

/// A folder of an origin and how many files it holds, its subfolders included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderCount {
    pub folder: String,
    pub files: usize,
}

const PENDING: &str = "pending";
const FAILED: &str = "failed";
const DONE: &str = "done";
const OUT: &str = "out";

/// What one stage goes over, each thing with how it stands in it.
#[derive(Debug, Default)]
struct Members {
    unit: &'static str,
    items: Vec<(Key, &'static str, Option<String>)>,
    other_work: bool,
    extra: BTreeMap<&'static str, usize>,
}

impl Members {
    fn new(unit: &'static str) -> Self {
        Self { unit, ..Default::default() }
    }

    fn push(&mut self, key: Key, status: &'static str) {
        self.items.push((key, status, None));
    }

    fn count(&self, status: &str) -> usize {
        self.items.iter().filter(|(_, s, _)| *s == status).count()
    }

    fn keys<'a>(&'a self, status: &'a str) -> impl Iterator<Item = &'a Key> + 'a {
        self.items.iter().filter(move |(_, s, _)| *s == status).map(|(key, _, _)| key)
    }
}

/// What every stage is computed from, read once.
struct Facts<'a> {
    scope: &'a Scope,
    /// The documents with a location in the scope that is not left out.
    active: BTreeSet<i64>,
    /// The name the engine in use saves its answers with.
    answering: Option<String>,
    reading_on: bool,
    engine_chosen: bool,
    /// The name the engine that recognizes scans saves its text with.
    recognizer: Option<String>,
    now: EngineNow,
    /// What a person said of the round of automatic questions (D-484).
    permit: Option<super::auto_questions::Permit>,
}

impl DocBase {
    /// What the engine is doing now and its last error (in memory).
    pub(crate) fn engine_state(&self) -> &EngineState {
        &self.engine
    }

    /// Every stage of the processing over `scope`, with its figures and
    /// how it stands. Reads only.
    pub fn progress(&self, scope: &Scope) -> Result<Progress, String> {
        let reading_on = self.background_reading()?;
        let now = self.engine_state().snapshot();
        let conn = self.conn();
        let facts = Facts::gather(&conn, scope, reading_on, now)?;
        let members = all_members(&conn, &facts)?;
        summarize(&conn, &facts, members)
    }

    /// The things one stage goes over in `scope`: what failed first, then
    /// what is pending, then what is done; at most `limit`. Reads only.
    pub fn progress_items(&self, stage: &str, scope: &Scope, limit: usize) -> Result<StageItems, String> {
        let Some(stage) = STAGES.iter().copied().find(|s| *s == stage) else {
            return Err(format!("Etapa desconocida: '{stage}'"));
        };
        let reading_on = self.background_reading()?;
        let now = self.engine_state().snapshot();
        let conn = self.conn();
        let facts = Facts::gather(&conn, scope, reading_on, now)?;
        let mut members = all_members(&conn, &facts)?;
        let Some(index) = STAGES.iter().position(|s| *s == stage) else {
            return Err(format!("Etapa desconocida: '{stage}'"));
        };
        let mut chosen = std::mem::take(&mut members[index]);
        let total = chosen.items.len();
        let rank = |status: &str| match status {
            FAILED => 0,
            PENDING => 1,
            DONE => 2,
            _ => 3,
        };
        chosen.items.sort_by(|a, b| rank(a.1).cmp(&rank(b.1)).then_with(|| a.0.cmp(&b.0)));
        let mut items = Vec::new();
        for (key, status, detail) in chosen.items.into_iter().take(limit.clamp(1, 500)) {
            items.push(describe(&conn, &facts, stage, &key, status, detail)?);
        }
        Ok(StageItems { stage: stage.to_string(), items, total })
    }

    /// The folders of an origin, each with the files under it, to choose
    /// the scope of the figures. Reads only.
    pub fn progress_folders(&self, origin_id: &str) -> Result<Vec<FolderCount>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT l.folder, count(*) FROM location l
                 WHERE l.origin_id = ?1 AND l.kind = 'file' AND NOT {} GROUP BY l.folder",
                super::excluded_sql!()
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map([origin_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(sql_err)?;
        let mut folders: BTreeMap<String, usize> = BTreeMap::new();
        for row in rows {
            let (folder, files) = row.map_err(sql_err)?;
            let mut path = String::new();
            for part in folder.split('/').filter(|p| !p.is_empty()) {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(part);
                *folders.entry(path.clone()).or_default() += files as usize;
            }
        }
        let mut out: Vec<FolderCount> = folders.into_iter().map(|(folder, files)| FolderCount { folder, files }).collect();
        out.sort_by(|a, b| a.folder.to_lowercase().cmp(&b.folder.to_lowercase()));
        Ok(out)
    }
}

impl<'a> Facts<'a> {
    fn gather(conn: &Connection, scope: &'a Scope, reading_on: bool, now: EngineNow) -> Result<Self, String> {
        let active = scope.contents(conn)?;
        let engine = super::similar::classify::ask_engine(conn)?;
        let recognizer = super::recognize::recognition_engine(conn)?
            .and_then(|config| crate::source_volume::recognition::engine_id(&config).ok());
        Ok(Self {
            scope,
            active,
            answering: engine.as_ref().map(crate::source_volume::asking::engine_id),
            reading_on,
            engine_chosen: engine.is_some(),
            recognizer,
            now,
            permit: super::auto_questions::permit(conn)?,
        })
    }
}

/// The members of every stage, in the order of `STAGES`.
fn all_members(conn: &Connection, facts: &Facts) -> Result<Vec<Members>, String> {
    let standings = super::knowledge::extract::standings(conn, &Visible::All, None)?;
    let read: Vec<i64> = standings
        .iter()
        .filter(|s| facts.active.contains(&s.content_id))
        .filter(|s| s.reading.as_ref().is_some_and(|r| r.current && r.state == "done"))
        .map(|s| s.content_id)
        .collect();
    let resolve = resolve_members(conn, facts, &read)?;
    let waiting_resolution: BTreeSet<i64> = resolve
        .keys(PENDING)
        .filter_map(|key| match key {
            Key::Content(id) => Some(*id),
            _ => None,
        })
        .collect();
    let resolving = resolve.other_work || !waiting_resolution.is_empty();
    Ok(vec![
        origin_members(conn, facts)?,
        text_members(conn, facts)?,
        recognition_members(conn, facts)?,
        similar_members(conn, facts)?,
        types_members(conn, facts)?,
        templates_members(conn, facts)?,
        reading_members(facts, &standings)?,
        resolve,
        matters_members(conn, facts, &read, &waiting_resolution, resolving)?,
        pages_members(conn, facts, resolving)?,
    ])
}

fn origin_members(conn: &Connection, facts: &Facts) -> Result<Members, String> {
    let mut members = Members::new("origins");
    for origin in origins::list(conn, &Visible::All)? {
        if facts.scope.origin().is_some_and(|id| id != origin.id) {
            continue;
        }
        let status = if origin.reachable == Some(false) {
            FAILED
        } else if origin.last_scan_at.is_none() {
            PENDING
        } else {
            DONE
        };
        members.push(Key::Origin(origin.id), status);
    }
    Ok(members)
}

/// Each file of the inventory: its bytes, and then the text of its document.
fn text_members(conn: &Connection, facts: &Facts) -> Result<Members, String> {
    let (rows, mut bound) = rows_sql(&Visible::All, None);
    let (condition, scope_bound) = facts.scope.location_sql("origin_id", "folder");
    bound.extend(scope_bound);
    let mut statement = conn
        .prepare(&format!("SELECT location_id, state FROM ({rows}) WHERE {condition}"))
        .map_err(sql_err)?;
    let found = statement
        .query_map(params_from_iter(bound), |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
        .map_err(sql_err)?;
    let mut members = Members::new("files");
    for row in found {
        let (location, state) = row.map_err(sql_err)?;
        let status = match state.as_str() {
            "unread" | "unprocessed" => PENDING,
            "unreadable" | "error" => FAILED,
            "processed" | "needs-recognition" => DONE,
            "excluded" => continue,
            _ => OUT,
        };
        members.items.push((Key::Location(location), status, (status == OUT).then_some(state)));
    }
    Ok(members)
}

/// The documents whose text is only pixels, those whose recognition
/// failed (with the reason) and those whose text a recognition already
/// read; with the pages that wait (P-242).
fn recognition_members(conn: &Connection, facts: &Facts) -> Result<Members, String> {
    let mut statement = conn
        .prepare(
            "SELECT c.id, c.state, c.error FROM version c LEFT JOIN derivative d ON d.version_id = c.id AND d.kind = 'text'
             WHERE c.state = 'needs-recognition' OR (c.state = 'processed' AND d.engine IS NOT NULL AND d.engine != ?1)",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([super::text::EXTRACTION_ENGINE], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?))
        })
        .map_err(sql_err)?;
    let mut members = Members::new("documents");
    let (mut pages, mut uncounted) = (0, 0);
    for row in rows {
        let (content, state, error) = row.map_err(sql_err)?;
        if !facts.active.contains(&content) {
            continue;
        }
        if state != "needs-recognition" {
            members.push(Key::Content(content), DONE);
            continue;
        }
        match facts.now.scan_pages.get(&content) {
            Some(count) => pages += count,
            None => uncounted += 1,
        }
        match error {
            Some(error) => members.items.push((Key::Content(content), FAILED, Some(error))),
            None => members.push(Key::Content(content), PENDING),
        }
    }
    let requested = super::recognize::requested_recognitions(conn)?
        .into_iter()
        .filter(|c| facts.active.contains(c))
        .count();
    members.extra.insert("pages", pages);
    members.extra.insert("uncounted", uncounted);
    members.extra.insert("requested", requested);
    Ok(members)
}

/// How the recognition of scans stands: it runs while a document is read,
/// when a person asked for some or while the automatic reading is on; it
/// is paused with the reading off, or with no engine chosen.
fn recognition_state(facts: &Facts, members: &Members, error: bool) -> &'static str {
    let reading_one = facts.now.recognizing.is_some_and(|r| facts.active.contains(&r.content));
    let requested = members.extra.get("requested").copied().unwrap_or(0) > 0;
    if reading_one {
        "running"
    } else if members.count(PENDING) == 0 && !requested {
        if members.count(FAILED) > 0 { "error" } else { "done" }
    } else if facts.recognizer.is_none() {
        "paused"
    } else if error {
        "error"
    } else if requested || facts.reading_on || super::services::local_recognition_engine().is_some() {
        // The engine of this computer reads with the reading off (D-558).
        "running"
    } else {
        "paused"
    }
}

/// Each document with text, compared with the others: copies, versions
/// and documents of the same template.
fn similar_members(conn: &Connection, facts: &Facts) -> Result<Members, String> {
    let mut statement = conn
        .prepare(
            "SELECT c.id,
                    s.version_id IS NOT NULL AND s.text_at = d.created_at AND s.engine = ?1
                    AND NOT EXISTS (SELECT 1 FROM similar_dirty sd WHERE sd.version_id = c.id)
             FROM version c JOIN derivative d ON d.version_id = c.id AND d.kind = 'text'
             LEFT JOIN similar_signature s ON s.version_id = c.id
             WHERE c.state = 'processed'",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([super::similar::store::engine_id()], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?)))
        .map_err(sql_err)?;
    let mut members = Members::new("documents");
    for row in rows {
        let (content, compared) = row.map_err(sql_err)?;
        if facts.active.contains(&content) {
            members.push(Key::Content(content), if compared { DONE } else { PENDING });
        }
    }
    Ok(members)
}

/// Each document with text enough: its type and stage, and the data the
/// rules did not find, asked to the model.
fn types_members(conn: &Connection, facts: &Facts) -> Result<Members, String> {
    let mut statement = conn
        .prepare(
            "SELECT s.version_id,
                    EXISTS (SELECT 1 FROM document_type t WHERE t.version_id = s.version_id),
                    EXISTS (SELECT 1 FROM document_data dd WHERE dd.version_id = s.version_id AND dd.question IS NOT NULL
                            AND NOT EXISTS (SELECT 1 FROM model_answer a WHERE a.question = dd.question AND (?1 IS NULL OR a.engine = ?1)))
             FROM similar_signature s JOIN version c ON c.id = s.version_id AND c.state = 'processed'
             WHERE s.words >= ?2",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map(
            rusqlite::params![facts.answering, super::similar::classify::MIN_TYPE_WORDS],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, bool>(1)?, row.get::<_, bool>(2)?)),
        )
        .map_err(sql_err)?;
    let mut members = Members::new("documents");
    for row in rows {
        let (content, typed, data_waiting) = row.map_err(sql_err)?;
        if facts.active.contains(&content) {
            members.push(Key::Content(content), if typed && !data_waiting { DONE } else { PENDING });
        }
    }
    members.extra.insert("questions", facts.now.left.get("types").copied().unwrap_or(0));
    Ok(members)
}

/// Each template of a family, by its principal: its fields that its text
/// does not name wait for the model (D-488).
fn templates_members(conn: &Connection, facts: &Facts) -> Result<Members, String> {
    let mut members = Members::new("templates");
    for (principal, waiting) in super::similar::naming::waiting_templates(conn)? {
        if !facts.active.contains(&principal) {
            continue;
        }
        if waiting == 0 {
            members.push(Key::Content(principal), DONE);
        } else {
            let detail = if waiting == 1 { "1 campo sin nombre".to_string() } else { format!("{waiting} campos sin nombre") };
            members.items.push((Key::Content(principal), PENDING, Some(detail)));
        }
    }
    members.extra.insert("questions", facts.now.left.get("templates").copied().unwrap_or(0));
    Ok(members)
}

/// Each document read by the model to know whom it names.
fn reading_members(facts: &Facts, standings: &[super::knowledge::extract::Standing]) -> Result<Members, String> {
    let mut members = Members::new("documents");
    let (mut outdated, mut requested) = (0, 0);
    for standing in standings.iter().filter(|s| facts.active.contains(&s.content_id)) {
        let status = match &standing.reading {
            Some(reading) if reading.current && reading.state == "done" => DONE,
            Some(reading) if reading.current => FAILED,
            _ => PENDING,
        };
        // Read with another version of the question or another model, kept
        // as they are (D-465); and asked by a person to read again (D-432).
        if let Some(reading) = &standing.reading {
            outdated += usize::from(reading.outdated(facts.answering.as_deref()));
            requested += usize::from(reading.reread);
        }
        members.push(Key::Content(standing.content_id), status);
    }
    members.extra.insert("outdated", outdated);
    members.extra.insert("requested", requested);
    Ok(members)
}

/// Each document read: its names joined with those of the others.
fn resolve_members(conn: &Connection, facts: &Facts, read: &[i64]) -> Result<Members, String> {
    let marks: Vec<String> = {
        let mut statement = conn
            .prepare("SELECT ref FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')")
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let dirty: BTreeSet<i64> = marks.iter().filter_map(|m| m.parse::<i64>().ok()).collect();
    let mut members = Members::new("documents");
    for content in read {
        members.push(Key::Content(*content), if dirty.contains(content) { PENDING } else { DONE });
    }
    // A decision to apply, a document that left: the resolution is of the
    // whole archive, so it only counts for the whole archive.
    let read: BTreeSet<i64> = read.iter().copied().collect();
    members.other_work = facts.scope.is_whole() && marks.iter().any(|m| m.parse::<i64>().map_or(true, |id| !read.contains(&id)));
    members.extra.insert("questions", facts.now.left.get("resolve").copied().unwrap_or(0));
    Ok(members)
}

/// Each document read, in its matter: the evidence groups them, and the
/// model proposes one for the loose ones once every identity is resolved.
fn matters_members(
    conn: &Connection,
    facts: &Facts,
    read: &[i64],
    waiting_resolution: &BTreeSet<i64>,
    resolving: bool,
) -> Result<Members, String> {
    let grouped: BTreeSet<i64> = {
        let mut statement = conn
            .prepare(
                "SELECT DISTINCT d.version_id FROM knowledge_matter_document d
                 JOIN knowledge_matter m ON m.id = d.matter_id WHERE m.state = 'active'",
            )
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut members = Members::new("documents");
    let (mut in_matter, mut loose) = (0, 0);
    for content in read {
        if waiting_resolution.contains(content) {
            members.push(Key::Content(*content), PENDING);
            continue;
        }
        if grouped.contains(content) {
            in_matter += 1;
        } else {
            loose += 1;
        }
        members.push(Key::Content(*content), DONE);
    }
    let questions = facts.now.left.get("matters").copied().unwrap_or(0);
    members.other_work = facts.scope.is_whole() && (questions > 0 || (resolving && !read.is_empty()));
    members.extra.insert("grouped", in_matter);
    members.extra.insert("loose", loose);
    members.extra.insert("questions", questions);
    Ok(members)
}

/// Each page (article) of the base: written from the record, and what the
/// documents say of its entity, written by the model.
fn pages_members(conn: &Connection, facts: &Facts, resolving: bool) -> Result<Members, String> {
    let in_scope: Option<BTreeSet<String>> = if facts.scope.is_whole() {
        None
    } else {
        let list = facts.active.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
        let mut ids: BTreeSet<String> = facts.active.iter().map(|c| format!("doc-{c}")).collect();
        if !list.is_empty() {
            for sql in [
                format!("SELECT DISTINCT entity_id FROM knowledge_mention WHERE entity_id IS NOT NULL AND version_id IN ({list})"),
                format!("SELECT DISTINCT matter_id FROM knowledge_matter_document WHERE version_id IN ({list})"),
            ] {
                let mut statement = conn.prepare(&sql).map_err(sql_err)?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(sql_err)?;
                for row in rows {
                    ids.insert(row.map_err(sql_err)?);
                }
            }
        }
        Some(ids)
    };
    let mut statement = conn
        .prepare(
            "SELECT p.entity_id,
                    p.synthesis IS NULL AND EXISTS (
                        SELECT 1 FROM knowledge_claim c WHERE c.subject = p.entity_id AND c.state IN ('supported', 'conflict'))
             FROM knowledge_page p WHERE p.state = 'projected'",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, bool>(1)?)))
        .map_err(sql_err)?;
    let mut members = Members::new("pages");
    for row in rows {
        let (id, waiting) = row.map_err(sql_err)?;
        if in_scope.as_ref().is_some_and(|ids| !ids.contains(&id)) {
            continue;
        }
        members.push(Key::Page(id), if waiting { PENDING } else { DONE });
    }
    let to_project: i64 = conn
        .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope = 'project'", [], |row| row.get(0))
        .map_err(sql_err)?;
    let questions = facts.now.left.get("pages").copied().unwrap_or(0);
    members.other_work = facts.scope.is_whole() && (to_project > 0 || resolving || questions > 0);
    members.extra.insert("questions", questions);
    Ok(members)
}

/// The figures and the state of every stage.
fn summarize(conn: &Connection, facts: &Facts, members: Vec<Members>) -> Result<Progress, String> {
    let index = |id: &str| STAGES.iter().position(|s| *s == id).unwrap_or(0);
    // Before the engine's first pass through it, a stage of
    // `FIRST_LAP_STAGES` may have work nobody counted yet: once something
    // was read, or it has things of its own, it waits for that pass.
    let read_any = members[index("reading")].count(DONE) > 0;
    let first_lap: Vec<bool> = STAGES
        .iter()
        .zip(&members)
        .map(|(id, m)| FIRST_LAP_STAGES.contains(id) && !facts.now.left.contains_key(*id) && (read_any || !m.items.is_empty()))
        .collect();
    let work = |i: usize| members[i].count(PENDING) > 0 || members[i].other_work || first_lap[i];
    // The stage the questions to the model are at: the one the engine said,
    // while it has work; otherwise the first one with work, in its order.
    let active_model = facts
        .now
        .model_stage
        .filter(|stage| work(index(stage)))
        .or_else(|| MODEL_STAGES.iter().copied().find(|stage| work(index(stage))));
    let error_stage: Option<&str> = facts.now.last_error.as_ref().map(|error| match error.stage.as_deref() {
        Some(stage) => STAGES.iter().copied().find(|s| *s == stage).unwrap_or("types"),
        None => active_model.unwrap_or("types"),
    });
    let can_ask = facts.reading_on && facts.engine_chosen;
    let text_pending = members[index("text")].count(PENDING) > 0;

    let mut stages = Vec::new();
    let mut in_process: BTreeSet<Key> = BTreeSet::new();
    let mut paused: BTreeSet<Key> = BTreeSet::new();
    for (i, (id, mut members)) in STAGES.iter().copied().zip(members).enumerate() {
        let pending = members.count(PENDING);
        let failed = members.count(FAILED);
        let has_work = pending > 0 || members.other_work || first_lap[i];
        if first_lap[i] {
            members.extra.insert("firstLap", 1);
        }
        let error = facts.now.last_error.clone().filter(|_| error_stage == Some(id));
        let model = MODEL_STAGES.contains(&id);
        // Its questions wait for a person (D-484).
        let held = model && has_work && can_ask && facts.now.auto_held == Some(id);
        if held {
            members.extra.insert("held", 1);
        }
        let state = if id == "recognition" {
            recognition_state(facts, &members, error.is_some())
        } else if model {
            if !has_work {
                if failed > 0 { "error" } else { "done" }
            } else if !can_ask || held {
                "paused"
            } else if error.is_some() {
                "error"
            } else if active_model == Some(id) {
                "running"
            } else {
                "waiting"
            }
        } else {
            match id {
                "origins" if facts.now.sweeping.as_deref().is_some_and(|o| facts.scope.origin().is_none_or(|s| s == o)) => "running",
                "origins" if pending > 0 => "waiting",
                "similar" if pending > 0 && text_pending => "waiting",
                _ if pending > 0 => "running",
                _ if failed > 0 => "error",
                _ => "done",
            }
        };
        let counted = matches!(members.unit, "files" | "documents");
        if counted {
            let target = if state == "paused" { &mut paused } else { &mut in_process };
            target.extend(members.keys(PENDING).cloned());
        }
        let current = current_item(conn, facts, id, state)?;
        stages.push(Stage {
            id,
            state,
            unit: members.unit,
            total: members.items.len() - members.count(OUT),
            done: members.count(DONE),
            pending,
            failed,
            out: members.count(OUT),
            other_work: members.other_work,
            current,
            error,
            extra: members.extra,
        });
    }
    // A document waiting in a stage that runs is in process, even if a
    // later stage that waits for the model is paused.
    paused.retain(|key| !in_process.contains(key));
    Ok(Progress {
        stages,
        in_process: in_process.len(),
        paused: paused.len(),
        background_reading: facts.reading_on,
        engine_chosen: facts.engine_chosen,
        recognition_engine: facts.recognizer.clone(),
        last_error: facts.now.last_error.clone(),
        auto_questions: super::auto_questions::AutoQuestions::of(conn, &facts.now, facts.permit)?,
    })
}

/// What the engine is on now in a stage that runs, when it is known and
/// inside the scope.
fn current_item(conn: &Connection, facts: &Facts, stage: &'static str, state: &str) -> Result<Option<StageItem>, String> {
    if state != "running" {
        return Ok(None);
    }
    let key = match stage {
        "origins" => facts.now.sweeping.clone().map(Key::Origin),
        "text" => facts.now.reading_file.clone(),
        "recognition" => facts.now.recognizing.map(|r| Key::Content(r.content)),
        _ if MODEL_STAGES.contains(&stage) && facts.now.model_stage == Some(stage) => facts.now.asking.map(Key::Content),
        _ => None,
    };
    let Some(key) = key else { return Ok(None) };
    let inside = match &key {
        Key::Origin(id) => facts.scope.origin().is_none_or(|o| o == id),
        Key::Content(id) => facts.active.contains(id),
        Key::Location(id) => {
            let (condition, mut bound) = facts.scope.location_sql("l.origin_id", "l.folder");
            bound.insert(0, Value::Integer(*id));
            conn.query_row(
                &format!("SELECT 1 FROM location l WHERE l.id = ? AND {condition}"),
                params_from_iter(bound),
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_err)?
            .is_some()
        }
        Key::Page(_) => true,
    };
    if !inside {
        return Ok(None);
    }
    let mut item = describe(conn, facts, stage, &key, PENDING, None)?;
    if let (Some(reading), "recognition") = (facts.now.recognizing, stage) {
        item.page = (reading.page > 0).then_some(reading.page);
        item.pages = (reading.total > 0).then_some(reading.total).or(item.pages);
    }
    Ok(Some(item))
}

/// One thing of a stage, with its name, where it is and its reason.
fn describe(
    conn: &Connection,
    facts: &Facts,
    stage: &str,
    key: &Key,
    status: &'static str,
    detail: Option<String>,
) -> Result<StageItem, String> {
    let mut item = StageItem {
        status,
        location_id: None,
        content_id: None,
        origin_id: None,
        page_path: None,
        name: String::new(),
        place: String::new(),
        detail,
        page: None,
        pages: None,
    };
    match key {
        Key::Origin(id) => {
            let found: Option<(String, String)> = conn
                .query_row("SELECT name, location FROM origin WHERE id = ?1", [id], |row| Ok((row.get(0)?, row.get(1)?)))
                .optional()
                .map_err(sql_err)?;
            let (name, location) = found.unwrap_or_else(|| (id.clone(), String::new()));
            item.origin_id = Some(id.clone());
            item.name = name;
            item.place = location;
        }
        Key::Location(id) => {
            let found: Option<(String, String, String, String, Option<i64>, Option<String>, Option<String>)> = conn
                .query_row(
                    "SELECT l.name, l.origin_id, o.name, l.folder, f.version_id, l.hash_error, c.error
                     FROM location l JOIN origin o ON o.id = l.origin_id
                     LEFT JOIN file f ON f.hash = l.file_hash LEFT JOIN version c ON c.id = f.version_id
                     WHERE l.id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
                )
                .optional()
                .map_err(sql_err)?;
            if let Some((name, origin_id, origin, folder, content, hash_error, content_error)) = found {
                item.location_id = Some(*id);
                item.content_id = content;
                item.origin_id = Some(origin_id);
                item.name = name;
                item.place = place(&origin, &folder);
                if status == FAILED {
                    item.detail = hash_error.or(content_error);
                }
            }
        }
        Key::Content(id) => {
            let (condition, mut bound) = facts.scope.location_sql("l.origin_id", "l.folder");
            bound.insert(0, Value::Integer(*id));
            let found: Option<(i64, String, String, String, String)> = conn
                .query_row(
                    &format!(
                        "SELECT l.id, l.name, l.origin_id, o.name, l.folder FROM location l
                         JOIN file f ON f.hash = l.file_hash JOIN origin o ON o.id = l.origin_id
                         WHERE f.version_id = ? AND NOT {} AND {condition}
                         ORDER BY l.mtime_ms DESC, l.id LIMIT 1",
                        super::excluded_sql!()
                    ),
                    params_from_iter(bound),
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
                .optional()
                .map_err(sql_err)?;
            item.content_id = Some(*id);
            match found {
                Some((location, name, origin_id, origin, folder)) => {
                    item.location_id = Some(location);
                    item.origin_id = Some(origin_id);
                    item.name = name;
                    item.place = place(&origin, &folder);
                }
                None => item.name = format!("Documento {id}"),
            }
            if stage == "recognition" && status != DONE {
                item.pages = facts.now.scan_pages.get(id).copied();
            }
            if stage == "reading" && status == FAILED {
                item.detail = conn
                    .query_row("SELECT error FROM knowledge_extraction WHERE version_id = ?1", [id], |row| row.get(0))
                    .optional()
                    .map_err(sql_err)?
                    .flatten();
            }
            if stage == "matters" && status == DONE {
                let mut statement = conn
                    .prepare(
                        "SELECT m.name FROM knowledge_matter_document d JOIN knowledge_matter m ON m.id = d.matter_id
                         WHERE d.version_id = ?1 AND m.state = 'active' ORDER BY m.name",
                    )
                    .map_err(sql_err)?;
                let rows = statement.query_map([id], |row| row.get::<_, String>(0)).map_err(sql_err)?;
                let names: Vec<String> = rows.collect::<Result<_, _>>().map_err(sql_err)?;
                item.detail = (!names.is_empty()).then(|| names.join(" · "));
            }
        }
        Key::Page(id) => {
            let path: Option<String> = conn
                .query_row("SELECT path FROM knowledge_page WHERE entity_id = ?1", [id], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            let named: Option<String> = conn
                .query_row(
                    "SELECT name FROM knowledge_entity WHERE id = ?1 UNION ALL SELECT name FROM knowledge_matter WHERE id = ?1 LIMIT 1",
                    [id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_err)?;
            let document = id.strip_prefix("doc-").and_then(|c| c.parse::<i64>().ok());
            let name = match (named, document) {
                (Some(name), _) => name,
                (None, Some(content)) => super::similar::classify::name_of(conn, content).unwrap_or_default(),
                (None, None) => String::new(),
            };
            let stem = path.as_deref().and_then(|p| p.rsplit('/').next()).map(|f| f.trim_end_matches(".md").to_string());
            item.name = if name.is_empty() { stem.unwrap_or_else(|| id.clone()) } else { name };
            item.content_id = document;
            item.page_path = path;
        }
    }
    Ok(item)
}

/// "<origin> › <folder>", or the origin alone at its root.
fn place(origin: &str, folder: &str) -> String {
    if folder.is_empty() {
        origin.to_string()
    } else {
        format!("{origin} › {folder}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::knowledge::tests::{base_with, is_knowledge, refresh, type_answer, FakeModel};
    use crate::docbase::testing::TempDir;
    use crate::source_volume::asking::AskConfig;

    // Invented documents (R-24).
    const LEASE: &str = "CONTRATO DE LOCACIÓN\n\nEntre Ejemplo Uno S.A., CUIT 30-11111111-1, y Persona Tres, DNI 33.333.333, \
en adelante el locatario, se celebra el presente contrato de locación por el inmueble del domicilio inventado. \
El canon mensual es de $ 100.000 a partir del 1 de marzo.";
    const NOTE: &str = "NOTA DE PRUEBA\n\nPor la presente se deja constancia de que Persona Tres retiró los papeles \
del inmueble del domicilio inventado el día del vencimiento, sin observaciones de ninguna de las partes.";

    fn stage<'a>(progress: &'a Progress, id: &str) -> &'a Stage {
        progress.stages.iter().find(|s| s.id == id).unwrap()
    }

    fn states(progress: &Progress) -> Vec<(&'static str, &'static str)> {
        progress.stages.iter().map(|s| (s.id, s.state)).collect()
    }

    fn figures(progress: &Progress, id: &str) -> (usize, usize, usize, usize) {
        let s = stage(progress, id);
        (s.done, s.pending, s.failed, s.total)
    }

    /// The model of these tests: a type for every document, and nobody named.
    fn model() -> FakeModel {
        FakeModel::new("codex-cli:m", |prompt| {
            if is_knowledge(prompt) {
                serde_json::json!({ "menciones": [], "afirmaciones": [], "importes": [] }).to_string()
            } else {
                type_answer("contrato")
            }
        })
    }

    fn turn_reading_on(base: &DocBase) {
        base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "m".into(), ..Default::default() })).unwrap();
        base.set_background_reading(true).unwrap();
    }

    #[test]
    fn every_stage_is_listed_in_order_and_a_finished_one_stays_as_done() {
        let (_dir, base) = base_with(&[("contrato.txt", LEASE), ("nueva/nota.txt", NOTE)]);
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(progress.stages.iter().map(|s| s.id).collect::<Vec<_>>(), STAGES);
        // The files are read and compared; the model has not been asked
        // anything because the automatic reading is off.
        assert_eq!(figures(&progress, "text"), (2, 0, 0, 2));
        assert_eq!(stage(&progress, "text").unit, "files");
        assert_eq!(figures(&progress, "similar"), (2, 0, 0, 2));
        assert_eq!(figures(&progress, "types"), (0, 2, 0, 2));
        assert_eq!(
            states(&progress)[..7],
            [
                ("origins", "done"),
                ("text", "done"),
                ("recognition", "done"),
                ("similar", "done"),
                ("types", "paused"),
                // No template: nothing to name.
                ("templates", "done"),
                ("reading", "paused"),
            ]
        );
        assert_eq!(stage(&progress, "templates").unit, "templates");
        assert_eq!((progress.in_process, progress.paused), (0, 2));
        assert!(!progress.background_reading);

        // With the reading on, the types go first and the reading waits its turn.
        turn_reading_on(&base);
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(stage(&progress, "types").state, "running");
        assert_eq!(stage(&progress, "reading").state, "waiting");
        assert_eq!((progress.in_process, progress.paused), (2, 0));

        let asker = model();
        base.types_run(&asker, 50).unwrap();
        let progress = base.progress(&Scope::default()).unwrap();
        // Done, it does not go away: it keeps its figures.
        assert_eq!(stage(&progress, "types").state, "done");
        assert_eq!(figures(&progress, "types"), (2, 0, 0, 2));
        assert_eq!(stage(&progress, "reading").state, "running");

        base.knowledge_run(&asker, 50).unwrap();
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(figures(&progress, "reading"), (2, 0, 0, 2));
        assert_eq!(stage(&progress, "reading").state, "done");
        // What was read waits for its names to be joined with the others.
        assert_eq!(stage(&progress, "resolve").state, "running");
        assert_eq!(stage(&progress, "resolve").pending, 2);
        assert_eq!(stage(&progress, "matters").state, "waiting");

        base.knowledge_resolve(Some(&asker), 50).unwrap();
        base.knowledge_propose_matters(Some(&asker), 50).unwrap();
        base.knowledge_project(Some(&asker), 50).unwrap();
        // The engine went through the matters and the pages, with nothing left.
        base.engine_state().ran("matters", 0);
        base.engine_state().ran("pages", 0);
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(stage(&progress, "resolve").state, "done");
        assert_eq!(figures(&progress, "resolve"), (2, 0, 0, 2));
        assert_eq!(stage(&progress, "matters").extra["loose"], 2);
        assert!(progress.stages.iter().all(|s| s.state == "done"), "{:?}", states(&progress));
        assert_eq!((progress.in_process, progress.paused), (0, 0));
    }

    #[test]
    fn the_matters_and_the_pages_wait_for_the_first_pass_of_the_engine() {
        let (_dir, base) = base_with(&[("contrato.txt", LEASE), ("nueva/nota.txt", NOTE)]);
        let asker = model();
        base.types_run(&asker, 50).unwrap();
        base.knowledge_run(&asker, 50).unwrap();
        base.knowledge_resolve(Some(&asker), 50).unwrap();
        base.knowledge_propose_matters(Some(&asker), 50).unwrap();
        base.knowledge_project(Some(&asker), 50).unwrap();
        // Read with an earlier version of the question (D-432): kept as read.
        base.conn().execute("UPDATE knowledge_extraction SET prompt_version = 'conocimiento-0'", []).unwrap();

        // The app just opened, with the automatic reading off: nothing tells
        // what the matters and the pages have left, so they are not done.
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(stage(&progress, "reading").state, "done");
        assert_eq!(stage(&progress, "reading").extra["outdated"], 2);
        for id in ["matters", "pages"] {
            assert_eq!(stage(&progress, id).state, "paused", "{id}");
            assert_eq!(stage(&progress, id).extra.get("firstLap"), Some(&1), "{id}");
        }
        // No document of theirs is counted as waiting.
        assert_eq!((progress.in_process, progress.paused), (0, 0));
        // The same in a folder.
        let folder = Scope { origin_id: base.origins(&Visible::All).unwrap().first().map(|o| o.id.clone()), folder: Some("nueva".into()) };
        assert_eq!(stage(&base.progress(&folder).unwrap(), "matters").state, "paused");

        // With the reading on, the engine goes to the matters, then the pages.
        turn_reading_on(&base);
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(stage(&progress, "matters").state, "running");
        assert_eq!(stage(&progress, "pages").state, "waiting");
        // Its pass made, with nothing left, both are done; what was read
        // with the earlier version still says so.
        base.engine_state().ran("matters", 0);
        base.engine_state().ran("pages", 0);
        let progress = base.progress(&Scope::default()).unwrap();
        assert!(progress.stages.iter().all(|s| s.state == "done"), "{:?}", states(&progress));
        assert!(progress.stages.iter().all(|s| !s.extra.contains_key("firstLap")));
        assert_eq!(stage(&progress, "reading").extra["outdated"], 2);
    }

    #[test]
    fn with_nothing_read_the_matters_and_the_pages_have_nothing_to_wait_for() {
        let (_dir, base) = base_with(&[("contrato.txt", LEASE)]);
        let progress = base.progress(&Scope::default()).unwrap();
        for id in ["matters", "pages"] {
            assert_eq!((stage(&progress, id).state, stage(&progress, id).total), ("done", 0), "{id}");
        }
    }

    #[test]
    fn files_not_read_yet_are_in_process_and_the_scope_narrows_every_figure() {
        let dir = TempDir::new();
        dir.write("o/contrato.txt", LEASE.as_bytes());
        dir.write("o/nueva/nota.txt", NOTE.as_bytes());
        dir.write("o/nueva/foto.jpg", b"\xff\xd8\xff\xe0 no es texto");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();

        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(stage(&progress, "origins").state, "done");
        assert_eq!(figures(&progress, "text"), (0, 3, 0, 3));
        assert_eq!(stage(&progress, "text").state, "running");
        assert_eq!(progress.in_process, 3);

        refresh(&base, &origin.id);
        let whole = base.progress(&Scope::default()).unwrap();
        // The image waits for its recognition: with no engine chosen it is
        // paused, and counts neither as in process nor as a failure.
        assert_eq!(figures(&whole, "recognition"), (0, 1, 0, 1));
        assert_eq!(stage(&whole, "recognition").state, "paused");
        assert_eq!(whole.in_process, 0);
        assert_eq!(whole.paused, 3);

        let folder = Scope { origin_id: Some(origin.id.clone()), folder: Some("nueva".into()) };
        let narrow = base.progress(&folder).unwrap();
        assert_eq!(figures(&narrow, "text"), (2, 0, 0, 2));
        assert_eq!(figures(&narrow, "types"), (0, 1, 0, 1));
        let elsewhere = Scope { origin_id: Some("otro".into()), folder: None };
        assert_eq!(figures(&base.progress(&elsewhere).unwrap(), "text"), (0, 0, 0, 0));

        // The list of a stage, with each document to open.
        let items = base.progress_items("types", &folder, 50).unwrap();
        assert_eq!(items.total, 1);
        assert_eq!(items.items[0].name, "nota.txt");
        assert_eq!(items.items[0].place, "O › nueva");
        assert_eq!(items.items[0].status, "pending");
        assert!(items.items[0].location_id.is_some());
        let text = base.progress_items("text", &Scope::default(), 50).unwrap();
        assert_eq!(text.items.iter().map(|i| i.status).collect::<Vec<_>>(), ["done", "done", "done"]);
        assert!(base.progress_items("nada", &Scope::default(), 50).is_err());
    }

    #[test]
    fn a_file_that_cannot_be_read_comes_first_in_its_list_with_its_reason() {
        let dir = TempDir::new();
        dir.write("o/contrato.txt", LEASE.as_bytes());
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        refresh(&base, &origin.id);
        base.conn()
            .execute("UPDATE version SET state = 'error', error = 'archivo dañado'", [])
            .unwrap();
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(figures(&progress, "text"), (0, 0, 1, 1));
        assert_eq!(stage(&progress, "text").state, "error");
        let items = base.progress_items("text", &Scope::default(), 50).unwrap();
        assert_eq!((items.items[0].status, items.items[0].detail.as_deref()), ("failed", Some("archivo dañado")));
    }

    #[test]
    fn the_last_error_of_the_model_shows_in_words_on_its_stage_until_an_answer_comes() {
        let (_dir, base) = base_with(&[("contrato.txt", LEASE)]);
        turn_reading_on(&base);
        base.engine_state().at_stage(Some("types"));
        base.engine_state().failed(
            Some("types"),
            "Codex exited with exit code: 1: You've hit your usage limit. Try again later.",
            Some(Duration::from_secs(600)),
        );
        let progress = base.progress(&Scope::default()).unwrap();
        let types = stage(&progress, "types");
        assert_eq!(types.state, "error");
        let error = types.error.as_ref().unwrap();
        assert_eq!(error.kind, "limit");
        assert_eq!(error.retry_at, Some(error.at + 600_000));
        assert!(stage(&progress, "reading").error.is_none());

        // An error with no stage (the model could not be started) goes to
        // the stage that would ask now.
        base.engine_state().failed(None, "Failed to start codex: program not found", Some(Duration::from_secs(600)));
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(stage(&progress, "types").error.as_ref().unwrap().kind, "other");

        // With the reading paused it says so, and keeps the error in view.
        base.set_background_reading(false).unwrap();
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(stage(&progress, "types").state, "paused");
        assert!(stage(&progress, "types").error.is_some());

        base.set_background_reading(true).unwrap();
        base.engine_state().asking("type", 1);
        base.engine_state().asked(true);
        let progress = base.progress(&Scope::default()).unwrap();
        assert!(progress.last_error.is_none());
        assert_eq!(stage(&progress, "types").state, "running");
    }

    #[test]
    fn the_document_being_read_is_named_on_its_stage() {
        let (_dir, base) = base_with(&[("contrato.txt", LEASE)]);
        turn_reading_on(&base);
        let content: i64 = base.conn().query_row("SELECT id FROM version", [], |row| row.get(0)).unwrap();
        base.engine_state().asking("type", content);
        let progress = base.progress(&Scope::default()).unwrap();
        let current = stage(&progress, "types").current.as_ref().unwrap();
        assert_eq!(current.name, "contrato.txt");
        // Only on the stage that runs.
        assert!(stage(&progress, "reading").current.is_none());
    }

    #[test]
    fn errors_are_told_by_their_kind() {
        assert_eq!(error_kind("Codex exited with exit code: 1: You've hit your usage limit"), "limit");
        assert_eq!(error_kind("stream error: 429 Too Many Requests"), "limit");
        assert_eq!(error_kind("401 Unauthorized: invalid api key"), "key");
        assert_eq!(error_kind("Codex did not answer within 600 seconds"), "connection");
        assert_eq!(error_kind("error sending request for url"), "connection");
        assert_eq!(error_kind("La respuesta no tiene la forma pedida"), "other");
    }

    #[test]
    fn each_question_belongs_to_its_stage() {
        use crate::docbase::knowledge::{extract, matters, pages, resolve};
        assert_eq!(stage_of_task("type"), "types");
        assert_eq!(stage_of_task("data"), "types");
        assert_eq!(stage_of_task(extract::TASK), "reading");
        assert_eq!(stage_of_task(resolve::TASK), "resolve");
        assert_eq!(stage_of_task(matters::TASK), "matters");
        assert_eq!(stage_of_task(pages::TASK), "pages");
    }

    #[test]
    fn the_folders_of_an_origin_count_the_files_under_them() {
        let dir = TempDir::new();
        dir.write("o/raiz.txt", b"uno");
        dir.write("o/Casos/A/uno.txt", b"uno");
        dir.write("o/Casos/A/dos.txt", b"dos");
        dir.write("o/Casos/B/tres.txt", b"tres");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        let folders = base.progress_folders(&origin.id).unwrap();
        let found: Vec<(&str, usize)> = folders.iter().map(|f| (f.folder.as_str(), f.files)).collect();
        assert_eq!(found, [("Casos", 3), ("Casos/A", 2), ("Casos/B", 1)]);
    }

    #[test]
    fn counting_writes_nothing() {
        let (_dir, base) = base_with(&[("contrato.txt", LEASE)]);
        let before = base.conn().query_row("SELECT total_changes()", [], |row| row.get::<_, i64>(0)).unwrap();
        base.progress(&Scope::default()).unwrap();
        for stage in STAGES {
            base.progress_items(stage, &Scope::default(), 10).unwrap();
        }
        let after = base.conn().query_row("SELECT total_changes()", [], |row| row.get::<_, i64>(0)).unwrap();
        assert_eq!(before, after);
    }

    /// How long counting takes on a copy of a real base (never the base in
    /// use): `MICELYA_PROGRESS_DB=<copy of private.sqlite3> cargo test --lib
    /// counting_a_copy_of_a_real_base_is_quick -- --ignored --nocapture`.
    /// It is asked every few seconds while the app is open.
    #[test]
    #[ignore]
    fn counting_a_copy_of_a_real_base_is_quick() {
        let Ok(path) = std::env::var("MICELYA_PROGRESS_DB") else { return };
        let base = DocBase::open(std::path::Path::new(&path)).unwrap();
        let started = std::time::Instant::now();
        let progress = base.progress(&Scope::default()).unwrap();
        let whole = started.elapsed();
        let origin = base.origins(&Visible::All).unwrap().into_iter().next().map(|o| o.id);
        let started = std::time::Instant::now();
        base.progress(&Scope { origin_id: origin, folder: None }).unwrap();
        let narrow = started.elapsed();
        let started = std::time::Instant::now();
        for stage in STAGES {
            base.progress_items(stage, &Scope::default(), 50).unwrap();
        }
        let lists = started.elapsed();
        println!("whole {whole:?} · one origin {narrow:?} · the nine lists {lists:?}");
        for stage in &progress.stages {
            println!("{} {} {}/{} pending {} failed {} out {}", stage.id, stage.state, stage.done, stage.total, stage.pending, stage.failed, stage.out);
        }
        assert!(whole < Duration::from_millis(1_500), "{whole:?}");
    }

    #[test]
    fn the_engine_keeps_what_it_is_on_and_forgets_it_when_done() {
        let engine = EngineState::default();
        engine.sweeping(Some("o1"));
        engine.asking("knowledge", 7);
        let now = engine.snapshot();
        assert_eq!((now.sweeping.as_deref(), now.model_stage, now.asking), (Some("o1"), Some("reading"), Some(7)));
        engine.sweeping(None);
        engine.asked(false);
        engine.ran("reading", 3);
        engine.at_stage(None);
        let now = engine.snapshot();
        assert_eq!((now.sweeping, now.model_stage, now.asking), (None, None, None));
        assert_eq!(now.left.get("reading"), Some(&3));
        // A long message is cut for the screens.
        engine.failed(Some("reading"), &"x".repeat(2_000), None);
        assert_eq!(engine.snapshot().last_error.unwrap().message.chars().count(), ERROR_CHARS);
    }
}
