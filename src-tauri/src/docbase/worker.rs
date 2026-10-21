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
/// of each document, then its reading for the knowledge layer, then the
/// identities of what was read (D-192), then the pages of the archive's
/// wiki that are a projection of the record (D-199). One
/// question at a time, but one after the other while there is work, so a
/// whole archive is read in one go; only while the archive's wiki is
/// active and an engine was chosen: like feeding the wiki, it costs the
/// provider's quota, and turning the wiki off stops it. When the provider
/// stops answering (its usage limit), it waits and tries again.
/// `on_pages` is called when pages of the wiki were written.
pub fn spawn_asking(base: Arc<DocBase>, on_change: impl Fn() + Send + 'static, on_pages: impl Fn() + Send + 'static) {
    let spawned = std::thread::Builder::new()
        .name("docbase-asking".to_string())
        .spawn(move || loop {
            let wait = ask_step(&base, &on_change, &on_pages);
            if !wait.is_zero() {
                std::thread::sleep(wait);
            }
        });
    if let Err(error) = spawned {
        eprintln!("[docbase] could not start the questions to the model: {error}");
    }
}

/// One question at most. Returns how long to wait before the next.
///
/// It tells the engine's state (`progress.rs`) which stage it is at, how
/// many questions each stage had left and the last failure of the model,
/// with when it tries again: what «En curso» shows (D-424).
fn ask_step(base: &DocBase, on_change: &dyn Fn(), on_pages: &dyn Fn()) -> Duration {
    let state = base.engine_state();
    let engine = match base.background_ask_engine() {
        Ok(Some(engine)) => engine,
        Ok(None) => {
            state.at_stage(None);
            return ASK_IDLE;
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            return ASK_IDLE;
        }
    };
    let asker = match crate::source_volume::asking::asker_for(&engine) {
        Ok(asker) => asker,
        Err(error) => {
            eprintln!("[docbase] {error}");
            state.failed(None, &error, Some(ASK_BACKOFF));
            return ASK_BACKOFF;
        }
    };
    let failed = |stage: &'static str, errors: &[String]| {
        state.failed(Some(stage), &errors.join("; "), Some(ASK_BACKOFF));
        ASK_BACKOFF
    };
    state.at_stage(Some("types"));
    match base.types_run(asker.as_ref(), 1) {
        Ok(report) => {
            state.ran("types", report.left);
            if report.asked + report.data_asked + report.typed + report.corrected > 0 {
                on_change();
            }
            if report.failed + report.data_failed > 0 {
                eprintln!("[docbase] questions to the model: {}", report.errors.join("; "));
                return failed("types", &report.errors);
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
    state.at_stage(Some("reading"));
    match base.knowledge_run(asker.as_ref(), 1) {
        Ok(report) => {
            state.ran("reading", report.left);
            if report.extracted + report.unusable > 0 {
                on_change();
            }
            if report.failed > 0 {
                eprintln!("[docbase] reading for the knowledge layer: {}", report.errors.join("; "));
                return failed("reading", &report.errors);
            }
            if report.asked > 0 || report.extracted > 0 {
                return Duration::ZERO;
            }
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            return ASK_IDLE;
        }
    }
    // Nothing left to read: the identities of what was read, one question
    // at a time too.
    state.at_stage(Some("resolve"));
    match base.knowledge_resolve(Some(asker.as_ref()), 1) {
        Ok(report) => {
            state.ran("resolve", report.left);
            if report.changed > 0 {
                on_change();
            }
            if report.failed > 0 {
                eprintln!("[docbase] identities of the knowledge layer: {}", report.errors.join("; "));
                return failed("resolve", &report.errors);
            }
            if report.asked > 0 || report.left > 0 {
                return Duration::ZERO;
            }
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            return ASK_IDLE;
        }
    }
    // Every identity resolved: the matter of the documents no evidence puts
    // in one, proposed by the model, one question at a time (D-213).
    state.at_stage(Some("matters"));
    match base.knowledge_propose_matters(Some(asker.as_ref()), 1) {
        Ok(report) => {
            state.ran("matters", report.left);
            if report.failed > 0 {
                eprintln!("[docbase] matters of the knowledge layer: {}", report.errors.join("; "));
                return failed("matters", &report.errors);
            }
            if report.asked > 0 {
                on_change();
                return Duration::ZERO;
            }
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            return ASK_IDLE;
        }
    }
    // The pages that changed, one question at a
    // time for what the documents say. Whatever the order in which the
    // wiki wrote its pages of documents, this leaves every page of entity
    // up to date with the whole archive.
    state.at_stage(Some("pages"));
    match base.knowledge_project(Some(asker.as_ref()), 1) {
        Ok(report) => {
            state.ran("pages", report.left);
            if report.wrote() {
                on_change();
                on_pages();
            }
            if report.failed > 0 {
                eprintln!("[docbase] pages of the knowledge layer: {}", report.errors.join("; "));
                return failed("pages", &report.errors);
            }
            if report.asked > 0 || report.left > 0 {
                Duration::ZERO
            } else {
                // Nothing left to ask anywhere.
                state.at_stage(None);
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
        base.engine_state().sweeping(Some(&origin.id));
        if let Err(error) = base.scan_origin(&origin.id) {
            eprintln!("[docbase] sweep of '{}' failed: {error}", origin.name);
        }
    }
    base.engine_state().sweeping(None);
    !origins.is_empty()
}

/// One small piece of pending work. Returns whether there was any.
fn step(base: &DocBase) -> Result<bool, String> {
    if base.hash_pending(HASH_BATCH)? > 0 {
        return Ok(true);
    }
    // Texts an older extraction wrote that the current one reads better
    // (a form's ticked boxes) go back to be extracted, once per version.
    if base.refresh_extraction(TEXT_BATCH)? > 0 {
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
