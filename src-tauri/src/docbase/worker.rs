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
    Ok(base.purge_orphans()? > 0)
}
