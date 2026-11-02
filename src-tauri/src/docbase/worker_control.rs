//! The control run (stage 7 of 1 nov, redone in E4 of 2 nov): the app as it
//! is, from an empty base, with the same loop of the process the app runs
//! (`process_turn`), not a chain of tests and not threads of its own.
//! Nothing given: no cuts, no readings, no answers of other runs.
//!
//! ```text
//! MICELYA_CONTROL_BASE="C:\corrida\base.sqlite3" MICELYA_DOCBASE_CHECK="C:\carpeta" MICELYA_CONTROL_UNTIL=cut \
//!     MICELYA_CONTROL_SETTINGS="background-reading=1" MICELYA_ASK_MODEL=... MICELYA_ASK_REASONING=high \
//!     cargo test --lib docbase::process::run::control -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - **It changes no setting of the app** (E4 of 2 nov): not the windows of
//!   the cut, which are asked by themselves in every base, nor the cap of
//!   the round. What a run needs different is written in its script, in
//!   sight: `MICELYA_CONTROL_SETTINGS`, `key=value` separated by `;`, kept
//!   in the base of the run and written at the start of the log. The
//!   engine of «Preguntar al modelo» is the one a person chooses in Ajustes:
//!   codex-cli with `MICELYA_ASK_MODEL` and `MICELYA_ASK_REASONING`, also
//!   written in the log. The reading by zones, that a test build leaves off,
//!   is put as the app comes (`ZONES_READING_IN_THE_APP`, the value of the
//!   app, not one of the script), and the log says so. The reader of the
//!   scans is never named by the script (E2 of 2 nov): it is the one of
//!   Ajustes of the base, the one of this computer by default, and the log
//!   says which (`reader_from_the_base`).
//! - `MICELYA_CONTROL_BASE`: the base of the run. Made empty when it is not
//!   there; when it is, it is opened again and the run goes on (D-678).
//! - `MICELYA_CONTROL_UNTIL`: `cut` ends once the steps up to the parts are
//!   complete; `all` once nothing is left.
//! - `MICELYA_CONTROL_LOG`: the events, one JSON per line (appended).
//! - `MICELYA_CONTROL_ASK_LIMIT`: at most this many new questions (a trial).
//! - `MICELYA_CONTROL_MAX_WAIT`: seconds; a wait longer than this (a failure
//!   whose next attempt is far, the round waiting for a person) ends the
//!   run, which is taken up again later.
//!
//! What stage A4 adds (the scripts before and after, without answers of
//! other runs) goes around this.

use std::io::Write as _;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::docbase::process::{Scope, Step};
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

/// What each step has left, by its number.
fn left(base: &DocBase) -> Value {
    match base.process_state(&Scope::All, false) {
        Ok(state) => json!(state
            .steps
            .iter()
            .map(|s| (s.about.number.to_string(), json!([s.count.pending, s.count.set_aside])))
            .collect::<serde_json::Map<_, _>>()),
        Err(error) => json!({"error": error}),
    }
}

/// Settings the script may never name: the reader of the scans comes from
/// Ajustes of the base (E2 of 2 nov; the reader of this computer by
/// default), never from the script of the run.
const NOT_FROM_THE_SCRIPT: [&str; 3] = ["ai-services", "recognition-engine", "recognition:"];

/// The settings the script of the run names, written in the base and in the log.
fn settings_of_the_script(base: &DocBase) -> Vec<(String, String)> {
    // The reading by zones as the app comes (`ZONES_READING_IN_THE_APP`,
    // `cut/by_zones/mod.rs`): this run is compiled as a test, where it is
    // off for the unit tests of the cut of before. The value is the app's,
    // never one of the script.
    use crate::docbase::cut::by_zones::{ZONES_READING, ZONES_READING_IN_THE_APP};
    let mut named: Vec<(String, String)> = vec![(ZONES_READING.to_string(), ZONES_READING_IN_THE_APP.to_string())];
    for pair in std::env::var("MICELYA_CONTROL_SETTINGS").unwrap_or_default().split(';') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = key.trim();
            assert!(
                !NOT_FROM_THE_SCRIPT.iter().any(|not| key.starts_with(not)),
                "MICELYA_CONTROL_SETTINGS names '{key}': the reader of the scans comes from Ajustes of the base, not from the script"
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
    base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: model.clone(), reasoning: reasoning.clone(), ..Default::default() }))
        .unwrap();
    named.push(("ask-engine".into(), format!("codex-cli {model} {reasoning}")));
    named
}

/// The reader of the scans the base uses, as Ajustes of the base says (it
/// is not named by the script): written in the log.
fn reader_of_the_base(base: &DocBase) -> String {
    match crate::docbase::services::recognition_choice(&base.conn()) {
        Ok(Some(config)) => crate::source_volume::recognition::engine_id(&config).unwrap_or_else(|e| e),
        Ok(None) => "none".into(),
        Err(error) => error,
    }
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
    log(json!({"t": now_ms(), "event": "open", "new": new, "until": if until_all { "all" } else { "cut" },
               "named_by_the_script": named, "reader_from_the_base": reader_of_the_base(&base), "settings": settings}));

    let asker = crate::docbase::knowledge::check::recorded();
    let started = Instant::now();
    let mut last_sweep = Instant::now();
    base.sweep_origins(None);
    let mut last = Value::Null;
    loop {
        let asked = asker.asked.load(Ordering::SeqCst);
        if asked >= limit {
            log(json!({"t": now_ms(), "event": "end", "why": "ask-limit", "asked": asked}));
            break;
        }
        if last_sweep.elapsed() >= super::SWEEP_EVERY {
            base.sweep_origins(None);
            base.process_woken();
            last_sweep = Instant::now();
        }
        let turn = base.process_turn(Some(&asker)).unwrap_or_else(|error| {
            log(json!({"t": now_ms(), "event": "error", "error": error}));
            super::Turn::default()
        });
        let now = left(&base);
        if now != last || turn.worked && asker.asked.load(Ordering::SeqCst) != asked {
            log(json!({"t": now_ms(), "event": "turn", "step": turn.step.map(|s| s.number()), "asked": asker.asked.load(Ordering::SeqCst), "left": now}));
            last = now;
        }
        if turn.worked {
            continue;
        }
        let state = base.process_state(&Scope::All, true).unwrap();
        let cut_done = state.first_incomplete.is_none_or(|s| s > Step::Parts);
        if !until_all && cut_done || until_all && state.first_incomplete.is_none() && base.process_status().state == "stopped" {
            log(json!({"t": now_ms(), "event": "end", "why": "done", "status": base.process_status()}));
            break;
        }
        let wait = crate::docbase::failures::next_attempt(&base.conn())
            .unwrap()
            .map(|at| Duration::from_millis(at.saturating_sub(now_ms()).max(0) as u64))
            .unwrap_or(max_wait + Duration::from_secs(1));
        if wait > max_wait {
            log(json!({"t": now_ms(), "event": "end", "why": "long-wait", "wait_s": wait.as_secs(), "status": base.process_status()}));
            break;
        }
        std::thread::sleep(wait.max(Duration::from_secs(1)));
        base.process_woken();
    }
    log(json!({"t": now_ms(), "event": "closed", "seconds": started.elapsed().as_secs(), "left": left(&base)}));
}
