//! The background worker: sweeps the origins and reads what is pending.
//!
//! Watching is a sweep at start, another every few minutes, and one
//! whenever the user or the app asks. Between sweeps it reads pending
//! files a few at a time, so the screens stay responsive and a large
//! cloud folder is downloaded little by little.

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use super::{DocBase, Visible};

const SWEEP_EVERY: Duration = Duration::from_secs(5 * 60);
const IDLE_WAIT: Duration = Duration::from_secs(20);
const HASH_BATCH: usize = 8;
const TEXT_BATCH: usize = 2;
const SIMILAR_BATCH: usize = 2;
const NOTIFY_EVERY: Duration = Duration::from_millis(800);

/// How the rest of the app asks the worker to sweep now.
#[derive(Default)]
pub struct Wake {
    sweep_requested: Mutex<bool>,
    signal: Condvar,
}

impl Wake {
    pub fn request_sweep(&self) {
        *self.sweep_requested.lock().unwrap_or_else(|p| p.into_inner()) = true;
        self.signal.notify_all();
    }

    fn take_request(&self) -> bool {
        std::mem::take(&mut *self.sweep_requested.lock().unwrap_or_else(|p| p.into_inner()))
    }

    fn wait(&self, timeout: Duration) {
        let guard = self.sweep_requested.lock().unwrap_or_else(|p| p.into_inner());
        if !*guard {
            let _ = self.signal.wait_timeout(guard, timeout);
        }
    }
}

/// Starts the worker thread. `on_change` is called, at most a few times
/// per second, whenever something the screens show may have changed.
pub fn spawn(base: Arc<DocBase>, wake: Arc<Wake>, on_change: impl Fn() + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("docbase-worker".to_string())
        .spawn(move || run(&base, &wake, &on_change));
    if let Err(error) = spawned {
        eprintln!("[docbase] could not start the background worker: {error}");
    }
}

fn run(base: &DocBase, wake: &Wake, on_change: &dyn Fn()) {
    let mut last_sweep: Option<Instant> = None;
    let mut last_notice = Instant::now();
    let mut unannounced = false;
    loop {
        let requested = wake.take_request();
        if requested || last_sweep.is_none_or(|at| at.elapsed() >= SWEEP_EVERY) {
            unannounced |= sweep(base);
            last_sweep = Some(Instant::now());
        }

        let worked = match step(base) {
            Ok(worked) => worked,
            Err(error) => {
                eprintln!("[docbase] {error}");
                false
            }
        };
        unannounced |= worked;

        if unannounced && (!worked || last_notice.elapsed() >= NOTIFY_EVERY) {
            on_change();
            last_notice = Instant::now();
            unannounced = false;
        }
        if !worked {
            wake.wait(IDLE_WAIT);
        }
    }
}

/// How long to wait before looking again when there is nothing to ask,
/// and after the provider failed (it may be at its usage limit).
const ASK_IDLE: Duration = Duration::from_secs(30);
const ASK_BACKOFF: Duration = Duration::from_secs(10 * 60);

/// Starts the thread that asks the model: first the type, stage and data
/// of each document, then its reading for the knowledge layer. One
/// question at a time, but one after the other while there is work, so a
/// whole archive is read in one go; only while the archive's wiki is
/// active and an engine was chosen: like feeding the wiki, it costs the
/// provider's quota, and turning the wiki off stops it. When the provider
/// stops answering (its usage limit), it waits and tries again.
pub fn spawn_asking(base: Arc<DocBase>, on_change: impl Fn() + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("docbase-asking".to_string())
        .spawn(move || loop {
            let wait = ask_step(&base, &on_change);
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
        });
    if let Err(error) = spawned {
        eprintln!("[docbase] could not start the questions to the model: {error}");
    }
}

/// One question at most. Returns how long to wait before the next.
fn ask_step(base: &DocBase, on_change: &dyn Fn()) -> Duration {
    let engine = match base.background_ask_engine() {
        Ok(Some(engine)) => engine,
        Ok(None) => return ASK_IDLE,
        Err(error) => {
            eprintln!("[docbase] {error}");
            return ASK_IDLE;
        }
    };
    let asker = match crate::source_volume::asking::asker_for(&engine) {
        Ok(asker) => asker,
        Err(error) => {
            eprintln!("[docbase] {error}");
            return ASK_BACKOFF;
        }
    };
    match base.types_run(asker.as_ref(), 1) {
        Ok(report) => {
            if report.asked + report.data_asked + report.typed + report.corrected > 0 {
                on_change();
            }
            if report.failed + report.data_failed > 0 {
                eprintln!("[docbase] questions to the model: {}", report.errors.join("; "));
                return ASK_BACKOFF;
            }
            if report.asked + report.data_asked > 0 {
                return Duration::ZERO;
            }
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            return ASK_IDLE;
        }
    }
    match base.knowledge_run(asker.as_ref(), 1) {
        Ok(report) => {
            if report.extracted + report.unusable > 0 {
                on_change();
            }
            if report.failed > 0 {
                eprintln!("[docbase] reading for the knowledge layer: {}", report.errors.join("; "));
                return ASK_BACKOFF;
            }
            if report.asked > 0 || report.extracted > 0 {
                Duration::ZERO
            } else {
                ASK_IDLE
            }
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            ASK_IDLE
        }
    }
}

/// Sweeps every origin. Returns whether anything may look different.
fn sweep(base: &DocBase) -> bool {
    let origins = match base.origins(&Visible::All) {
        Ok(origins) => origins,
        Err(error) => {
            eprintln!("[docbase] {error}");
            return false;
        }
    };
    for origin in &origins {
        if let Err(error) = base.scan_origin(&origin.id) {
            eprintln!("[docbase] sweep of '{}' failed: {error}", origin.name);
        }
    }
    !origins.is_empty()
}

/// One small piece of pending work. Returns whether there was any.
fn step(base: &DocBase) -> Result<bool, String> {
    if base.hash_pending(HASH_BATCH)? > 0 {
        return Ok(true);
    }
    if base.extract_pending(TEXT_BATCH)? > 0 {
        return Ok(true);
    }
    if base.purge_orphans()? > 0 {
        return Ok(true);
    }
    // Similar documents: only once every text is read, a little at a
    // time, so it never holds the reading back.
    Ok(base.similar_step(SIMILAR_BATCH)? > 0)
}
