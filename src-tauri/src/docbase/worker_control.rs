//! The control run (stage 7 of 1 nov): the app as it is, from an empty
//! base, with its two threads of work, not a chain of tests. One thread does
//! what the app's worker and its reader of this computer do (`step`,
//! `local_recognition_step`: bytes, text, scans, the reading by zones, the
//! ficha, the cut with rules, the parts, the signatures and the similar
//! documents); the other asks as the app's thread that asks (`ask_with`,
//! with its barrier: only the question of the cut while work without AI is
//! left). Nothing given: no cuts, no readings, no answers of other runs.
//!
//! ```text
//! MICELYA_CONTROL_BASE="C:\corrida\base.sqlite3" MICELYA_DOCBASE_CHECK="C:\carpeta" MICELYA_CONTROL_UNTIL=cut \
//!     MICELYA_OCR_DIR=... MICELYA_ASK_CACHE=... MICELYA_ASK_MODEL=... MICELYA_ASK_REASONING=high \
//!     cargo test --lib docbase::worker::control -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - `MICELYA_CONTROL_BASE`: the base of the run. Made empty when it is not
//!   there; when it is, it is opened again and the run goes on (D-678), as
//!   the app does when it opens again.
//! - On every open it turns on, in this base only: the automatic reading,
//!   the engine (codex-cli, `MICELYA_ASK_MODEL`, `MICELYA_ASK_REASONING`),
//!   the reading by zones (`cut:zones-reading`, which a test build leaves
//!   off), the windows of the cut by zones asked by themselves
//!   (`cut:windows-by-itself`) and a round cap no one has to lift (no
//!   person presses «Seguir» at night).
//! - `MICELYA_CONTROL_UNTIL`: `cut` ends once nothing without AI is left and
//!   no question of the cut waits; `all` once nothing is left to ask.
//! - `MICELYA_CONTROL_LOG`: the events, one JSON per line (appended): the
//!   work without AI left, by kind, when it changes; every turn of the
//!   thread that asks that asked or changed its wait.
//! - `MICELYA_CONTROL_ASK_LIMIT`: at most this many new questions (a trial).
//! - `MICELYA_CONTROL_MAX_WAIT`: seconds; a wait longer than this (a failure
//!   whose next attempt is far) ends the run, which is taken up again later.

use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::docbase::{now_ms, DocBase};
use crate::source_volume::asking::AskConfig;

fn log(event: Value) {
    let line = event.to_string();
    println!("{line}");
    if let Ok(path) = std::env::var("MICELYA_CONTROL_LOG") {
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

fn barrier_value(base: &DocBase) -> Value {
    match base.barrier() {
        Ok(b) => json!({"text": b.text, "scans": b.scans, "shapes": b.shapes, "cut": b.cut, "similar": b.similar}),
        Err(error) => json!({"error": error}),
    }
}

/// The settings of the run, in its base only.
fn turn_on(base: &DocBase) {
    let model = std::env::var("MICELYA_ASK_MODEL").unwrap_or_default();
    let reasoning = std::env::var("MICELYA_ASK_REASONING").unwrap_or_default();
    base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model, reasoning, ..Default::default() })).unwrap();
    base.set_background_reading(true).unwrap();
    for (key, value) in [(crate::docbase::cut::by_zones::ZONES_READING, "on"), (crate::docbase::cut::by_zones::WINDOWS_BY_ITSELF, "1")] {
        base.conn()
            .execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", rusqlite::params![key, value])
            .unwrap();
    }
    let mut thresholds = base.thresholds().unwrap();
    thresholds.round_cap = 100_000;
    base.set_thresholds(&thresholds).unwrap();
}

#[test]
#[ignore = "runs the app on the real folders named in MICELYA_DOCBASE_CHECK and asks the real model"]
fn the_app_from_zero() {
    let (Ok(path), Ok(folders)) = (std::env::var("MICELYA_CONTROL_BASE"), std::env::var("MICELYA_DOCBASE_CHECK")) else {
        println!("MICELYA_CONTROL_BASE or MICELYA_DOCBASE_CHECK is not set: nothing to run");
        return;
    };
    let until_all = std::env::var("MICELYA_CONTROL_UNTIL").as_deref() == Ok("all");
    let limit: usize = std::env::var("MICELYA_CONTROL_ASK_LIMIT").ok().and_then(|l| l.parse().ok()).unwrap_or(usize::MAX);
    let max_wait = Duration::from_secs(std::env::var("MICELYA_CONTROL_MAX_WAIT").ok().and_then(|l| l.parse().ok()).unwrap_or(900));
    let new = !Path::new(&path).exists();
    let base = DocBase::open(Path::new(&path)).unwrap();
    turn_on(&base);
    if new {
        for (index, folder) in folders.split(';').filter(|f| !f.trim().is_empty()).enumerate() {
            base.add_origin(&format!("{}", index + 1), Path::new(folder.trim()), &[]).unwrap();
        }
    }
    let settings: Vec<(String, String)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT key, value FROM docbase_setting ORDER BY key").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    log(json!({"t": now_ms(), "event": "open", "new": new, "until": if until_all { "all" } else { "cut" }, "settings": settings}));
    super::sweep(&base);

    let asker = crate::docbase::knowledge::check::recorded();
    let local_idle = AtomicBool::new(false);
    let stop = AtomicBool::new(false);
    let started = Instant::now();
    std::thread::scope(|scope| {
        // The worker and the reader of this computer, one after the other.
        scope.spawn(|| {
            let mut last = Value::Null;
            while !stop.load(Ordering::SeqCst) {
                let worked = super::step(&base).unwrap_or_else(|error| {
                    log(json!({"t": now_ms(), "event": "local-error", "error": error}));
                    false
                });
                let read = super::local_recognition_step(&base, &|| {}).is_zero();
                let now = barrier_value(&base);
                if now != last {
                    log(json!({"t": now_ms(), "event": "local", "left": now}));
                    last = now;
                }
                local_idle.store(!worked && !read, Ordering::SeqCst);
                if !worked && !read {
                    std::thread::sleep(Duration::from_secs(2));
                }
            }
        });
        // The thread that asks, one question at a time.
        let mut last = (false, usize::MAX, Duration::MAX);
        let mut idle_turns = 0;
        loop {
            let asked_before = asker.asked.load(Ordering::SeqCst);
            if asked_before >= limit {
                log(json!({"t": now_ms(), "event": "end", "why": "ask-limit", "asked": asked_before}));
                break;
            }
            let holds = base.barrier_holds();
            // `cut`: once the barrier is free, only what is left of the cut
            // goes out; the rest is the next point.
            let cut_only_left = !until_all && !holds && base.cut_questions().map(|q| q.by_itself).unwrap_or(1) == 0;
            let wait = if cut_only_left { super::ASK_IDLE } else { super::ask_with(&base, &asker, &|| {}, &|| {}) };
            let asked = asker.asked.load(Ordering::SeqCst);
            if (holds, asked, wait) != last {
                log(json!({"t": now_ms(), "event": "ask", "holds": holds, "asked": asked, "wait_s": wait.as_secs()}));
                last = (holds, asked, wait);
            }
            if wait.is_zero() {
                idle_turns = 0;
                continue;
            }
            let idle = local_idle.load(Ordering::SeqCst) && !base.barrier_holds();
            let cut_left = base.cut_questions().map(|q| q.by_itself).unwrap_or(1);
            idle_turns = if idle { idle_turns + 1 } else { 0 };
            // Twice idle in a row: the work of the other thread may have
            // brought more (the parts made again after an answer).
            if idle && cut_left == 0 && (!until_all || wait == super::ASK_IDLE) && idle_turns >= 2 {
                log(json!({"t": now_ms(), "event": "end", "why": "done", "asked": asked}));
                break;
            }
            if wait > max_wait {
                log(json!({"t": now_ms(), "event": "end", "why": "long-wait", "wait_s": wait.as_secs(), "asked": asked}));
                break;
            }
            std::thread::sleep(wait.min(Duration::from_secs(if holds || !idle { 5 } else { 15 })));
        }
        stop.store(true, Ordering::SeqCst);
    });
    log(json!({"t": now_ms(), "event": "closed", "seconds": started.elapsed().as_secs(), "left": barrier_value(&base), "cut_questions_left": base.cut_questions().map(|q| q.by_itself).ok()}));
}
