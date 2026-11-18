//! The control run (stage 7 of 1 nov, redone in E4 and A4 of 2 nov): the
//! app as it is, from an empty base, with THE LOOP OF THE APP itself
//! (`run_looked`, the same function the app runs, with its watcher, its
//! sweeps and its sleeps), not a chain of tests and not waits of its own.
//! Nothing given: no cuts, no readings, no answers of other runs, and no
//! answers kept by the harness either (A4 of 2 nov: the rule «desde cero»).
//!
//! ```text
//! MICELYA_CONTROL_BASE="C:\corrida\base.sqlite3" MICELYA_CONTROL_RUN=<id> MICELYA_DOCBASE_CHECK="C:\carpeta;C:\llegan" \
//!     MICELYA_CONTROL_SETTINGS="background-reading=1" MICELYA_CONTROL_READER=local \
//!     MICELYA_ASK_MODEL=... MICELYA_ASK_REASONING=high \
//!     cargo test --lib docbase::process::run::control::the_app_from_zero -- --exact --ignored --nocapture --test-threads=1
//! ```
//!
//! - **It changes no setting of the app** (E4 of 2 nov): not the windows of
//!   the cut, which are asked by themselves in every base, nor the cap of
//!   the round. When the round waits for a person, the harness does what a
//!   person does on the screen, «Seguir» (`auto_questions_decide`), and
//!   writes it down. What a run needs different is written in its script,
//!   in sight: `MICELYA_CONTROL_SETTINGS` (`key=value` separated by `;`),
//!   kept in the base and written at the start of the log; the engine of
//!   «Preguntar al modelo», the one a person chooses in Ajustes (codex-cli
//!   with `MICELYA_ASK_MODEL` and `MICELYA_ASK_REASONING`); and, with
//!   `MICELYA_CONTROL_READER=local`, the reader of the scans of Ajustes of
//!   the base set to the one of this computer (point 3b of the order of the
//!   owner of 2 nov), the same choice a person makes there. The reading by
//!   zones, that a test build leaves off, is put as the app comes
//!   (`ZONES_READING_IN_THE_APP`), and the log says so.
//! - **No answers of other runs** (A4 of 2 nov): the questions go to the
//!   engine of Ajustes, counted and written down, never kept nor taken from
//!   a file. It refuses to start with a variable of the tests that brings
//!   answers, readings, cuts or bases (`FROM_OTHER_RUNS`), a new base with
//!   readings beside it, or a base of another run (`control:run`).
//! - `MICELYA_CONTROL_BASE`: the base of the run. Made empty when it is not
//!   there; when it is, it is opened again and the run goes on (D-678).
//! - `MICELYA_CONTROL_RUN`: the name of the run, kept in the base.
//! - `MICELYA_CONTROL_LOG`: the events, one JSON per line (appended): each
//!   question (`ask`), each operation (`op`: the step, the start and the
//!   end, the questions, whether the steps before were complete, over the
//!   base and over the document of each answer), what the watcher heard,
//!   what a person did.
//! - `MICELYA_CONTROL_ASK_LIMIT`: at most this many new questions (a trial).
//! - `MICELYA_CONTROL_MAX_WAIT`: seconds; a wait longer than this (a failure
//!   whose next attempt is far) ends the run, which is taken up again later.
//! - `MICELYA_CONTROL_STOPPED_MARK` and `MICELYA_CONTROL_STOPPED_WAIT`: the
//!   first time the process stops with nothing left, the file is written and
//!   the loop goes on as the app does, asleep, for at most that many seconds
//!   (the script copies files into a watched origin then: the watcher and
//!   the type of a family, decision 3 of 2 nov).

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use super::{run_looked, Looker, Turn, Wake};
use crate::docbase::process::Scope;
use crate::docbase::{now_ms, DocBase};
use crate::source_volume::asking::{AskConfig, DocumentAsker};

fn log(event: Value) {
    let line = event.to_string();
    println!("{line}");
    if let Ok(path) = std::env::var("MICELYA_CONTROL_LOG") {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn env_number(name: &str) -> Option<u64> {
    std::env::var(name).ok().and_then(|l| l.parse().ok())
}

/// What each step has left, by its number: [pending, set aside].
fn left_of(base: &DocBase, scope: &Scope) -> Vec<(u8, usize, usize)> {
    base.process_state(scope, false)
        .map(|state| state.steps.iter().map(|s| (s.about.number, s.count.pending, s.count.set_aside)).collect())
        .unwrap_or_default()
}

fn left_json(left: &[(u8, usize, usize)]) -> Value {
    json!(left.iter().map(|(n, p, a)| (n.to_string(), json!([p, a]))).collect::<serde_json::Map<_, _>>())
}

/// The steps before `step` with something pending.
fn incomplete_before(left: &[(u8, usize, usize)], step: u8) -> Vec<u8> {
    left.iter().filter(|(n, pending, _)| *n < step && *pending > 0).map(|(n, _, _)| *n).collect()
}

/// The variables of the tests that bring answers, readings, cuts or bases
/// of other runs (auditoría 1A of 2 nov, section 4): a control run never
/// starts with one of them.
pub(crate) const FROM_OTHER_RUNS: [&str; 12] = [
    "MICELYA_ASK_CACHE",
    "MICELYA_ASK_OFFLINE",
    "MICELYA_ANSWER_SEED",
    "MICELYA_RECOGNIZED_SEED",
    "MICELYA_RECOGNIZED_CACHE",
    "MICELYA_DOCBASE_RECOGNIZED",
    "MICELYA_DOCBASE_START",
    "MICELYA_DOCBASE_COPY",
    "MICELYA_CUTS_GIVEN",
    "MICELYA_ZONES_ANSWERS",
    "MICELYA_ENTITY_ANSWERS",
    "MICELYA_KNOWLEDGE_SAVE",
];

fn refuse_other_runs() {
    let set: Vec<&str> = FROM_OTHER_RUNS.iter().copied().filter(|v| std::env::var_os(v).is_some()).collect();
    assert!(set.is_empty(), "a control run starts from zero: these variables bring things of other runs: {set:?}");
}

/// Settings the script may never name: the reader of the scans is chosen
/// only with `MICELYA_CONTROL_READER`, as Ajustes chooses it.
const NOT_FROM_THE_SCRIPT: [&str; 3] = ["ai-services", "recognition-engine", "recognition:"];

/// The mark of the run in its base.
const RUN_SETTING: &str = "control:run";

/// The settings the script of the run names, written in the base and in the log.
fn settings_of_the_script(base: &DocBase) -> Vec<(String, String)> {
    // The reading by zones as the app comes (`ZONES_READING_IN_THE_APP`,
    // `cut/by_zones/mod.rs`): this run is compiled as a test, where it is
    // off for the unit tests of the cut of before. The value is the app's,
    // never one of the script.
    use crate::docbase::cut::by_zones::{ZONES_READING, ZONES_READING_IN_THE_APP};
    let mut named: Vec<(String, String)> = vec![(ZONES_READING.to_string(), ZONES_READING_IN_THE_APP.to_string())];
    named.push(("vectors".into(), "como la app: Micelya Core del llavero (una compilación de prueba los deja apagados)".into()));
    for pair in std::env::var("MICELYA_CONTROL_SETTINGS").unwrap_or_default().split(';') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = key.trim();
            assert!(
                !NOT_FROM_THE_SCRIPT.iter().any(|not| key.starts_with(not)) && key != RUN_SETTING,
                "MICELYA_CONTROL_SETTINGS names '{key}': not a setting a script may name"
            );
            named.push((key.to_string(), value.trim().to_string()));
        }
    }
    for (key, value) in &named {
        base.conn()
            .execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", rusqlite::params![key, value])
            .unwrap();
    }
    let model = std::env::var("MICELYA_ASK_MODEL").unwrap_or_default();
    let reasoning = std::env::var("MICELYA_ASK_REASONING").unwrap_or_default();
    if model.is_empty() {
        // A trial without models: no engine chosen, as an app where nobody
        // chose one; the questions wait and nothing is asked.
        base.set_ask_engine(None).unwrap();
        named.push(("ask-engine".into(), "none".into()));
    } else {
        base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: model.clone(), reasoning: reasoning.clone(), ..Default::default() }))
            .unwrap();
        named.push(("ask-engine".into(), format!("codex-cli {model} {reasoning}")));
    }
    // Point 3b of the order of the owner of 2 nov: Ajustes of the base of
    // the run points to the reader of this computer, as a person chooses it.
    match std::env::var("MICELYA_CONTROL_READER").as_deref() {
        Ok("local") => {
            let engine = crate::source_volume::recognition_local::ENGINE.to_string();
            let model = crate::source_volume::recognition_local::catalog().model_set.clone();
            base.set_recognition_engine(Some(&crate::source_volume::recognition::RecognitionConfig { engine, model })).unwrap();
            named.push(("reader".into(), "local (Ajustes de la base)".into()));
        }
        Ok(other) => panic!("MICELYA_CONTROL_READER={other}: only «local» (point 3b of the order of 2 nov)"),
        Err(_) => {}
    }
    named
}

/// The reader of the scans the base uses, as Ajustes of the base says.
fn reader_of_the_base(base: &DocBase) -> String {
    match crate::docbase::services::recognition_choice(&base.conn()) {
        Ok(Some(config)) => crate::source_volume::recognition::engine_id(&config).unwrap_or_else(|e| e),
        Ok(None) => "none".into(),
        Err(error) => error,
    }
}

/// The engine of Ajustes, its questions counted and written down: never
/// kept, never taken from a file (no answers of other runs).
struct Counted {
    inner: Box<dyn DocumentAsker>,
    asked: Arc<AtomicUsize>,
}

impl Counted {
    fn counted(&self, prompt: &str, images: usize, ask: impl Fn() -> Result<String, String>) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        let n = self.asked.fetch_add(1, Ordering::SeqCst) + 1;
        let t0 = now_ms();
        let answer = ask();
        log(json!({"t": now_ms(), "event": "ask", "n": n, "t0": t0, "t1": now_ms(), "engine": self.inner.engine_id(),
                   "ok": answer.is_ok(), "error": answer.as_ref().err(), "images": images, "prompt_chars": prompt.chars().count(),
                   "prompt_sha": format!("{:x}", Sha256::digest(prompt.as_bytes())),
                   "answer_sha": answer.as_ref().ok().map(|a| format!("{:x}", Sha256::digest(a.as_bytes())))}));
        answer
    }
}

impl DocumentAsker for Counted {
    fn engine_id(&self) -> String {
        self.inner.engine_id()
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        self.counted(prompt, 0, || self.inner.ask(prompt, schema))
    }

    fn sees_images(&self) -> bool {
        self.inner.sees_images()
    }

    fn ask_with_images(&self, prompt: &str, schema: &serde_json::Value, images: &[PathBuf]) -> Result<String, String> {
        self.counted(prompt, images.len(), || self.inner.ask_with_images(prompt, schema, images))
    }
}

/// The control run looking at the loop of the app.
struct Control {
    wake: Arc<Wake>,
    asked: Arc<AtomicUsize>,
    limit: usize,
    max_wait: Duration,
    stopped_mark: Option<PathBuf>,
    stopped_wait: Duration,
    stopped_since: Option<Instant>,
    /// At the start of the turn going on: the time, the questions, what each step had left.
    t0: i64,
    asked_before: usize,
    left_before: Vec<(u8, usize, usize)>,
    last_answer: i64,
    last_step: Option<u8>,
    ops: usize,
    backs: usize,
    seguir_without_work: usize,
}

impl Control {
    fn end(&self, why: &str, base: &DocBase) {
        log(json!({"t": now_ms(), "event": "end", "why": why, "asked": self.asked.load(Ordering::SeqCst), "ops": self.ops,
                   "backs": self.backs, "status": base.process_status()}));
    }

    /// The answers written since the last operation, with the steps before
    /// `step` incomplete over their document right after it (complete after
    /// is complete before: an operation does only the work of its step).
    fn answers(&mut self, base: &DocBase, step: u8) -> Vec<Value> {
        let rows: Vec<(i64, String, Option<i64>, i64)> = {
            let conn = base.conn();
            let mut statement =
                conn.prepare("SELECT rowid, task, version_id, asked_at FROM model_answer WHERE rowid > ?1 ORDER BY rowid").unwrap();
            let rows = statement.query_map([self.last_answer], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap();
            rows.collect::<Result<_, _>>().unwrap()
        };
        rows.into_iter()
            .map(|(rowid, task, version, asked_at)| {
                self.last_answer = self.last_answer.max(rowid);
                let before = version.map(|v| incomplete_before(&left_of(base, &Scope::Document(v)), step));
                json!({"task": task, "version": version, "asked_at": asked_at, "prior_incomplete_doc": before})
            })
            .collect()
    }
}

impl Looker for Control {
    fn asker(&mut self, asker: Option<Box<dyn DocumentAsker>>) -> Option<Box<dyn DocumentAsker>> {
        asker.map(|inner| Box::new(Counted { inner, asked: self.asked.clone() }) as Box<dyn DocumentAsker>)
    }

    fn before(&mut self, base: &DocBase) {
        self.t0 = now_ms();
        self.asked_before = self.asked.load(Ordering::SeqCst);
        self.left_before = left_of(base, &Scope::All);
    }

    fn after(&mut self, base: &DocBase, turn: &Result<Turn, String>) -> bool {
        let t1 = now_ms();
        let asked = self.asked.load(Ordering::SeqCst);
        match turn {
            Err(error) => log(json!({"t": t1, "event": "error", "t0": self.t0, "error": error})),
            Ok(turn) if turn.worked => {
                self.ops += 1;
                self.stopped_since = None;
                self.seguir_without_work = 0;
                let step = turn.step.map(|s| s.number());
                let back = matches!((step, self.last_step), (Some(now), Some(last)) if now < last);
                self.backs += usize::from(back);
                let answers = step.map(|s| self.answers(base, s)).unwrap_or_default();
                log(json!({"t": t1, "event": "op", "step": step, "t0": self.t0, "t1": t1, "asked": asked - self.asked_before,
                           "prior_incomplete": step.map(|s| incomplete_before(&self.left_before, s)),
                           "left_before": left_json(&self.left_before), "answers": answers, "back": back}));
                if step.is_some() {
                    self.last_step = step;
                }
            }
            Ok(_) => {}
        }
        if asked >= self.limit {
            self.end("ask-limit", base);
            return false;
        }
        true
    }

    fn heard(&mut self, base: &DocBase) {
        log(json!({"t": now_ms(), "event": "heard", "status": base.process_status()}));
    }

    fn idle(&mut self, base: &DocBase, sleep: Duration) -> bool {
        let status = base.process_status();
        if status.state == "waiting-person" {
            // What a person does on the screen when the round asks: «Seguir»;
            // twice without any work between, it is not the round: the run ends.
            if self.seguir_without_work >= 2 {
                self.end("waiting-person", base);
                return false;
            }
            self.seguir_without_work += 1;
            base.auto_questions_decide(true).unwrap();
            log(json!({"t": now_ms(), "event": "person", "what": "Seguir", "status": status}));
            self.wake.request();
            return true;
        }
        let state = base.process_state(&Scope::All, true).unwrap();
        if state.first_incomplete.is_none() && status.state == "stopped" {
            if let Some(mark) = self.stopped_mark.as_ref().filter(|m| !m.exists()) {
                std::fs::write(mark, format!("{}\n", now_ms())).unwrap();
                log(json!({"t": now_ms(), "event": "stopped-first", "left": left_json(&left_of(base, &Scope::All))}));
                self.stopped_since = Some(Instant::now());
                return true;
            }
            if self.stopped_since.is_some_and(|since| since.elapsed() < self.stopped_wait) {
                return true;
            }
            self.end("done", base);
            return false;
        }
        let wait = crate::docbase::failures::next_attempt(&base.conn())
            .unwrap()
            .map(|at| Duration::from_millis(at.saturating_sub(now_ms()).max(0) as u64))
            .unwrap_or(self.max_wait + Duration::from_secs(1));
        if wait > self.max_wait && self.stopped_since.is_none() {
            log(json!({"t": now_ms(), "event": "waiting", "wait_s": wait.as_secs(), "sleep_s": sleep.as_secs(), "first_incomplete": state.first_incomplete.map(|s| s.number())}));
            self.end("long-wait", base);
            return false;
        }
        true
    }
}

/// Opens the base of the run: a new one only beside no readings, an old
/// one only of this same run.
fn open_the_base(path: &str, run: &str) -> (DocBase, bool) {
    let new = !Path::new(path).exists();
    let beside = Path::new(path).parent().unwrap_or(Path::new(".")).join(crate::docbase::recognize::RECOGNIZED_DIR);
    if new {
        let readings = std::fs::read_dir(&beside).map(|d| d.count()).unwrap_or(0);
        assert_eq!(readings, 0, "a new base of a control run starts with no readings beside it ({})", beside.display());
    }
    let base = DocBase::open(Path::new(path)).unwrap();
    let kept: Option<String> = {
        use rusqlite::OptionalExtension;
        base.conn()
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [RUN_SETTING], |r| r.get(0))
            .optional()
            .unwrap()
    };
    match kept {
        None if new => {
            base.conn().execute("INSERT INTO docbase_setting (key, value) VALUES (?1, ?2)", rusqlite::params![RUN_SETTING, run]).unwrap();
        }
        Some(kept) if kept == run => {}
        other => panic!("the base {path} is not of the run {run} (it says {other:?}): a control run starts from zero"),
    }
    (base, new)
}

#[test]
#[ignore = "runs the app on the real folders named in MICELYA_DOCBASE_CHECK and asks the real model"]
fn the_app_from_zero() {
    let (Ok(path), Ok(folders)) = (std::env::var("MICELYA_CONTROL_BASE"), std::env::var("MICELYA_DOCBASE_CHECK")) else {
        println!("MICELYA_CONTROL_BASE or MICELYA_DOCBASE_CHECK is not set: nothing to run");
        return;
    };
    refuse_other_runs();
    // The vectors as the app comes (C1 of 2 nov, D-805: on, with Micelya
    // Core of the keychain): a test build leaves them off unless a test
    // names its Core. The value is the app's, never one of the script.
    crate::docbase::vectors::TEST_CORE.with(|core| *core.borrow_mut() = Some(crate::docbase::vectors::TestCore::Real));
    let run = std::env::var("MICELYA_CONTROL_RUN").expect("MICELYA_CONTROL_RUN: the name of the run, kept in its base");
    let (base, new) = open_the_base(&path, &run);
    let named = settings_of_the_script(&base);
    if new {
        for (index, folder) in folders.split(';').filter(|f| !f.trim().is_empty()).enumerate() {
            // Named by its folder, as the screen of Orígenes names it: not «1» (F4, row 7).
            let folder = Path::new(folder.trim());
            let name = folder.file_name().map(|n| n.to_string_lossy().to_string()).filter(|n| !n.is_empty()).unwrap_or_else(|| "Origen".into());
            let name = if index == 0 { name } else { format!("{name} ({})", index + 1) };
            base.add_origin(&name, folder, &[]).unwrap();
        }
    }
    let settings: Vec<(String, String)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT key, value FROM docbase_setting ORDER BY key").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let last_answer: i64 = base.conn().query_row("SELECT coalesce(max(rowid), 0) FROM model_answer", [], |r| r.get(0)).unwrap();
    log(json!({"t": now_ms(), "event": "open", "run": run, "new": new, "named_by_the_script": named,
               "reader_from_the_base": reader_of_the_base(&base), "settings": settings, "answers_in_the_base": last_answer}));

    let wake = Arc::new(Wake::default());
    let mut control = Control {
        wake: wake.clone(),
        asked: Arc::new(AtomicUsize::new(0)),
        limit: env_number("MICELYA_CONTROL_ASK_LIMIT").map(|l| l as usize).unwrap_or(usize::MAX),
        max_wait: Duration::from_secs(env_number("MICELYA_CONTROL_MAX_WAIT").unwrap_or(900)),
        stopped_mark: std::env::var("MICELYA_CONTROL_STOPPED_MARK").ok().map(PathBuf::from),
        stopped_wait: Duration::from_secs(env_number("MICELYA_CONTROL_STOPPED_WAIT").unwrap_or(1800)),
        stopped_since: None,
        t0: 0,
        asked_before: 0,
        left_before: Vec::new(),
        last_answer,
        last_step: None,
        ops: 0,
        backs: 0,
        seguir_without_work: 0,
    };
    let started = Instant::now();
    run_looked(&base, &wake, &|| {}, &|| {}, &mut control);
    log(json!({"t": now_ms(), "event": "closed", "seconds": started.elapsed().as_secs(), "left": left_json(&left_of(&base, &Scope::All))}));
}

/// The measure of the hint of the type (C2 of 2 nov, D-805): over the base
/// of a control run, the question of the type of each document typed by the
/// model asked AGAIN WITHOUT THE HINT (the question of before, `tipo-2`,
/// each document alone, never in a group), with the engine of Ajustes of
/// the run, counted and never kept nor taken from a file, and kept apart
/// (`type_control_answer`): the process never reads it, and no type changes.
///
/// ```text
/// MICELYA_CONTROL_BASE="C:\corrida\base.sqlite3" MICELYA_CONTROL_LOG="C:\corrida\sin-pista.jsonl" \
///     [MICELYA_CONTROL_NO_HINT_ALL=1] [MICELYA_CONTROL_ASK_LIMIT=n] \
///     cargo test --lib docbase::process::run::control::the_type_without_the_hint -- --exact --ignored --nocapture --test-threads=1
/// ```
///
/// - Run it once the control run is over (or stopped): it opens the same base.
/// - The documents: those without a family whose type the model gave about
///   them; with `MICELYA_CONTROL_NO_HINT_ALL=1`, also the principals of a
///   family. Not the ones whose question had no hint nor group (`tipo-2`):
///   both questions would be the same one. Never those of low confidence.
/// - One line per document in the log (`event: "no-hint"`): the type with
///   the hint (the one kept), the one without it, whether they are the same,
///   whether it went in a group (then the difference mixes the group and the
///   hint) and whether it was asked now. Taken up again, what is already
///   kept apart is not asked again (D-678).
#[test]
#[ignore = "asks the real model the type of the documents of a control run without the hint"]
fn the_type_without_the_hint() {
    let Ok(path) = std::env::var("MICELYA_CONTROL_BASE") else {
        println!("MICELYA_CONTROL_BASE is not set: nothing to run");
        return;
    };
    refuse_other_runs();
    assert!(Path::new(&path).exists(), "the base of the control run is not there: {path}");
    let all = std::env::var("MICELYA_CONTROL_NO_HINT_ALL").as_deref() == Ok("1");
    let limit: usize = env_number("MICELYA_CONTROL_ASK_LIMIT").map(|l| l as usize).unwrap_or(usize::MAX);
    let base = DocBase::open(Path::new(&path)).unwrap();
    let engine = crate::docbase::services::ask_engine(&base.conn()).unwrap().expect("the run chose the engine of «Preguntar al modelo»");
    let asked_counter = Arc::new(AtomicUsize::new(0));
    let asker = Counted { inner: crate::source_volume::asking::asker_for(&engine).unwrap(), asked: asked_counter.clone() };
    let documents = base.typed_by_the_model(!all).unwrap();
    let measured: Vec<_> = documents.iter().filter(|d| d.question_version != crate::docbase::similar::classify::TYPE_PROMPT).collect();
    log(json!({"t": now_ms(), "event": "no-hint-open", "documents": documents.len(), "measured": measured.len(),
               "with_family": all, "hint_on": base.type_hint().unwrap()}));
    let (mut same, mut done) = (0usize, 0usize);
    for document in measured {
        if asked_counter.load(Ordering::SeqCst) >= limit {
            log(json!({"t": now_ms(), "event": "end", "why": "ask-limit", "asked": asked_counter.load(Ordering::SeqCst)}));
            break;
        }
        match base.type_without_hint(&asker, document.version_id) {
            Ok(control) => {
                done += 1;
                same += usize::from(control.doc_type == document.doc_type);
                log(json!({"t": now_ms(), "event": "no-hint", "version": document.version_id, "with_hint": document.doc_type,
                           "without_hint": control.doc_type, "same": control.doc_type == document.doc_type,
                           "in_group": document.in_group, "hint_in_question": document.with_hint, "asked_now": control.asked}));
            }
            Err(error) => log(json!({"t": now_ms(), "event": "error", "version": document.version_id, "error": error})),
        }
    }
    log(json!({"t": now_ms(), "event": "no-hint-closed", "measured": done, "same": same, "asked": asked_counter.load(Ordering::SeqCst)}));
}
