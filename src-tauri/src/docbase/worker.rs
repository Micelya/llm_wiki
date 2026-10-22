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
/// Readings checked again from the answers kept per step.
const RECHECK_BATCH: usize = 4;
/// Documents waiting for recognition whose pages are counted per step.
const SCAN_PAGES_BATCH: usize = 2;
const NOTIFY_EVERY: Duration = Duration::from_millis(800);

/// How the rest of the app asks the worker to sweep now.
#[derive(Default)]
pub struct Wake {
    sweep_requested: Mutex<bool>,
    signal: Condvar,
}

impl Wake {
    pub fn request_sweep(&self) {
        self.request();
    }

    /// Ends the wait of the thread now (for the questions to the model: a
    /// person asked to recognize a scan).
    pub fn request(&self) {
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

/// Starts the thread that asks the model: first the text of the scans
/// (P-242), then the type, stage and data of each document, then the
/// names of the fields of the templates (D-488), then its
/// reading for the knowledge layer, then the identities of what was read
/// (D-192), then the pages of the archive's wiki that are a projection of
/// the record (D-199). One
/// question at a time, but one after the other while there is work, so a
/// whole archive is read in one go; only while the automatic reading is
/// on and an engine was chosen: it costs the provider's quota, and turning
/// it off stops it. A scan a person asked to recognize is read even with
/// the reading off; `wake` ends the wait when one is asked for. When the
/// provider stops answering (its usage limit), it waits and tries again.
/// `on_pages` is called when pages of the wiki were written.
pub fn spawn_asking(
    base: Arc<DocBase>,
    wake: Arc<Wake>,
    on_change: impl Fn() + Send + 'static,
    on_pages: impl Fn() + Send + 'static,
) {
    let spawned = std::thread::Builder::new()
        .name("docbase-asking".to_string())
        .spawn(move || loop {
            let wait = ask_step(&base, &on_change, &on_pages);
            if !wait.is_zero() {
                wake.wait(wait);
            }
            wake.take_request();
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
    if let Some(wait) = recognition_step(base, on_change) {
        return wait;
    }
    if let Some(wait) = requested_reading_step(base, on_change) {
        return wait;
    }
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
    // The names of the fields of the templates that their text does not
    // give, one template at a time (D-488).
    state.at_stage(Some("templates"));
    match base.templates_naming_run(asker.as_ref(), 1) {
        Ok(report) => {
            state.ran("templates", report.left);
            if report.asked + report.named > 0 {
                on_change();
            }
            if report.failed > 0 {
                eprintln!("[docbase] names of the fields of the templates: {}", report.errors.join("; "));
                return failed("templates", &report.errors);
            }
            if report.asked > 0 {
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
            if report.extracted + report.unusable + report.rechecked > 0 {
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
            // Skipped while identities are being resolved: not a pass (D-452).
            if !report.skipped {
                state.ran("matters", report.left);
            }
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
            if report.reason.as_deref() != Some("resolving") {
                state.ran("pages", report.left);
            }
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

/// The documents a person asked to read again (D-432), with the automatic
/// reading on or off, before the rest: one question at a time. Once none is
/// left, what they named is resolved again without asking. `None` when
/// there is none, or no engine: the questions go on.
fn requested_reading_step(base: &DocBase, on_change: &dyn Fn()) -> Option<Duration> {
    match base.knowledge_rereads_waiting() {
        Ok(0) => return None,
        Ok(_) => {}
        Err(error) => {
            eprintln!("[docbase] {error}");
            return None;
        }
    }
    let engine = match super::similar::classify::ask_engine(&base.conn()) {
        Ok(Some(engine)) => engine,
        Ok(None) => return None,
        Err(error) => {
            eprintln!("[docbase] {error}");
            return None;
        }
    };
    let asker = crate::source_volume::asking::asker_for(&engine).ok()?;
    let state = base.engine_state();
    state.at_stage(Some("reading"));
    match base.knowledge_run_requested(asker.as_ref(), 1) {
        Ok(report) => {
            if report.extracted + report.unusable > 0 {
                on_change();
            }
            if report.failed > 0 {
                eprintln!("[docbase] reading again what was asked: {}", report.errors.join("; "));
                state.failed(Some("reading"), &report.errors.join("; "), Some(ASK_BACKOFF));
                return Some(ASK_BACKOFF);
            }
            if report.asked == 0 && report.extracted == 0 {
                // Nothing could be asked (the type of the document waits):
                // the other questions go on.
                return None;
            }
            if base.knowledge_rereads_waiting().unwrap_or(0) == 0 {
                let _ = base.knowledge_resolve(None, 0);
                on_change();
            }
            Some(Duration::ZERO)
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            None
        }
    }
}

/// The text of one scan, before any other question (`recognize.rs`): the
/// first one a person asked for, or, while the automatic reading is on,
/// the next one waiting. `None` when there is none, or no engine to read
/// it: the questions go on.
fn recognition_step(base: &DocBase, on_change: &dyn Fn()) -> Option<Duration> {
    use super::recognize::RecognitionStep;
    let config = match base.recognition_engine() {
        Ok(Some(config)) => config,
        Ok(None) => return None,
        Err(error) => {
            eprintln!("[docbase] {error}");
            return None;
        }
    };
    match base.next_recognition() {
        Ok(Some(_)) => {}
        Ok(None) => return None,
        Err(error) => {
            eprintln!("[docbase] {error}");
            return None;
        }
    }
    let recognizer = match crate::source_volume::recognition::recognizer_for(&config) {
        Ok(recognizer) => recognizer,
        Err(error) => {
            eprintln!("[docbase] {error}");
            return None;
        }
    };
    base.engine_state().at_stage(Some("recognition"));
    on_change();
    match base.recognition_step(recognizer.as_ref(), ASK_BACKOFF) {
        Ok(RecognitionStep::Idle) => None,
        Ok(RecognitionStep::Stopped(error)) => {
            eprintln!("[docbase] text recognition: {error}");
            on_change();
            Some(ASK_BACKOFF)
        }
        Ok(RecognitionStep::Failed(content)) => {
            eprintln!("[docbase] the text of document {content} could not be recognized");
            on_change();
            Some(Duration::ZERO)
        }
        Ok(_) => {
            on_change();
            Some(Duration::ZERO)
        }
        Err(error) => {
            eprintln!("[docbase] {error}");
            None
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
    // A document generated from a template follows its file, before its
    // text is read (D-488). A failure there never holds the rest back.
    match base.generated_settle() {
        Ok(changed) if changed > 0 => return Ok(true),
        Ok(_) => {}
        Err(error) => eprintln!("[docbase] generated documents: {error}"),
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
    // Readings checked with an earlier version of the program's check are
    // checked again from the answers kept, without the model (D-432).
    if base.knowledge_recheck(RECHECK_BATCH)? > 0 {
        return Ok(true);
    }
    // Similar documents: only once every text is read, a little at a
    // time, so it never holds the reading back.
    if base.similar_step(SIMILAR_BATCH)? > 0 {
        return Ok(true);
    }
    // The pages the scans need recognized: the size of that work.
    Ok(base.count_scan_pages(SCAN_PAGES_BATCH)? > 0)
}
