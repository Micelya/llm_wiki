//! The readings this computer asked for in Micelya Core and that are not
//! deleted yet (P-294), by their identifiers, kept in a file of the app's
//! data folder (`LEDGER_FILE`). With it:
//!
//! - only what this computer asked for is ever deleted in Core: a
//!   credential may be shared by several computers, and the generic name of
//!   a page (`pagina-N.jpg`) says nothing of whose it is;
//! - a reading left in Core (the app closed half-way, Core not answering,
//!   or refusing to delete it, as when the credential lacks the
//!   permission) is deleted later, even after the app is opened again
//!   (`retry_left_readings`, from the worker every `RETRY_LEFT_EVERY`, or
//!   when a person asks in Atención);
//! - a reading Core refused to delete for something that is not a cut is
//!   shown in Atención until it is deleted.
//!
//! It keeps only identifiers, the generic names and the reasons Core gave:
//! nothing of the documents.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use super::client::{CoreClient, CoreError, Timing};
use super::saved_connection;

/// Its file, in the app's data folder.
pub const LEDGER_FILE: &str = "micelya-core-lecturas.json";
/// How often the readings left in Core are tried again.
pub const RETRY_LEFT_EVERY: Duration = Duration::from_secs(10 * 60);
/// How long a reading whose page was not kept is left to its walk: until
/// then the page may be asked again, and Core gives the same reading back
/// without reading it again (and the walk deletes it). After that it is
/// deleted from outside.
pub const LEFT_BEHIND: Duration = Duration::from_secs(60 * 60);

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// A reading of this computer not deleted yet in Core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeftReading {
    pub id: String,
    /// The name it was asked with (`pagina-N.jpg`).
    pub name: String,
    /// When it was asked, in milliseconds since 1970.
    pub asked_at: i64,
    /// Its text was kept by the base: only deleting it is left. Otherwise
    /// its page is asked again, and Core gives it back without reading it
    /// again.
    #[serde(default)]
    pub kept: bool,
    /// Why Core refused to delete it the last time it was not a cut (no
    /// answer, Core not available): what Atención shows.
    #[serde(default)]
    pub refused: Option<String>,
    /// When deleting it was last tried.
    #[serde(default)]
    pub tried_at: Option<i64>,
}

#[derive(Default)]
struct Inner {
    loaded: bool,
    left: Vec<LeftReading>,
    /// The readings a walk of this process (or a retry) is on now: nobody
    /// else deletes them.
    walking: HashSet<String>,
}

pub struct Ledger {
    file: Option<PathBuf>,
    inner: Mutex<Inner>,
}

impl Ledger {
    /// Kept in `file`.
    pub fn at(file: impl Into<PathBuf>) -> Self {
        Self { file: Some(file.into()), inner: Mutex::new(Inner::default()) }
    }

    /// Kept only while the app runs (the tests).
    pub fn in_memory() -> Self {
        Self { file: None, inner: Mutex::new(Inner::default()) }
    }

    fn with<T>(&self, change: impl FnOnce(&mut Inner) -> T) -> T {
        let mut inner = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if !inner.loaded {
            inner.loaded = true;
            if let Some(file) = &self.file {
                inner.left = load(file);
            }
        }
        change(&mut inner)
    }

    /// Changes the readings and keeps them in the file.
    fn change<T>(&self, change: impl FnOnce(&mut Inner) -> T) -> T {
        self.with(|inner| {
            let before = inner.left.clone();
            let out = change(inner);
            if inner.left != before {
                if let Some(file) = &self.file {
                    if let Err(error) = save(file, &inner.left) {
                        eprintln!("[core] the readings left in Micelya Core could not be written: {error}");
                    }
                }
            }
            out
        })
    }

    /// A walk was given this reading by Core for this computer: it is this
    /// computer's, and the walk is on it until it tells how it ended.
    pub fn asked(&self, id: &str, name: &str) {
        self.change(|inner| {
            inner.walking.insert(id.to_string());
            if !inner.left.iter().any(|left| left.id == id) {
                inner.left.push(LeftReading {
                    id: id.to_string(),
                    name: name.to_string(),
                    asked_at: now_ms(),
                    kept: false,
                    refused: None,
                    tried_at: None,
                });
            }
        });
    }

    /// Whether this computer asked for it and has not deleted it yet.
    pub fn is_ours(&self, id: &str) -> bool {
        self.with(|inner| inner.left.iter().any(|left| left.id == id))
    }

    /// A reading of this computer that no walk is on: the walk takes it.
    pub fn adopt(&self, id: &str) -> bool {
        self.with(|inner| {
            let free = inner.left.iter().any(|left| left.id == id) && !inner.walking.contains(id);
            if free {
                inner.walking.insert(id.to_string());
            }
            free
        })
    }

    /// Deleted in Core (or no longer there).
    pub fn deleted(&self, id: &str) {
        self.change(|inner| {
            inner.walking.remove(id);
            inner.left.retain(|left| left.id != id);
        });
    }

    /// Not deleted: `kept` says whether its text was kept by the base;
    /// `refused`, why Core refused when it was not a cut (`None` keeps the
    /// reason it had: a cut says nothing new).
    pub fn not_deleted(&self, id: &str, kept: bool, refused: Option<String>) {
        self.change(|inner| {
            inner.walking.remove(id);
            if let Some(left) = inner.left.iter_mut().find(|left| left.id == id) {
                left.kept |= kept;
                if refused.is_some() {
                    left.refused = refused;
                }
                left.tried_at = Some(now_ms());
            }
        });
    }

    /// No walk is on it any more, and nothing was tried.
    pub fn release(&self, id: &str) {
        self.with(|inner| {
            inner.walking.remove(id);
        });
    }

    fn is_due(inner: &Inner, left: &LeftReading, now: i64) -> bool {
        !inner.walking.contains(&left.id)
            && (left.refused.is_some() || left.kept || now - left.asked_at >= LEFT_BEHIND.as_millis() as i64)
    }

    /// Whether some reading is to be deleted now (`due`).
    pub fn any_due(&self, now: i64) -> bool {
        self.with(|inner| inner.left.iter().any(|left| Self::is_due(inner, left, now)))
    }

    /// The readings to delete now, taken (no walk is on them until each is
    /// told): Core refused to delete them, or their text was kept, or they
    /// were asked longer ago than `LEFT_BEHIND`.
    pub fn due(&self, now: i64) -> Vec<LeftReading> {
        self.with(|inner| {
            let due: Vec<LeftReading> = inner.left.iter().filter(|left| Self::is_due(inner, left, now)).cloned().collect();
            for left in &due {
                inner.walking.insert(left.id.clone());
            }
            due
        })
    }

    /// The readings Core refused to delete, for Atención.
    pub fn refused(&self) -> Vec<LeftReading> {
        self.with(|inner| inner.left.iter().filter(|left| left.refused.is_some()).cloned().collect())
    }

    /// Every reading left, for the tests.
    #[cfg(test)]
    pub fn left(&self) -> Vec<LeftReading> {
        self.with(|inner| inner.left.clone())
    }
}

fn load(file: &Path) -> Vec<LeftReading> {
    let Ok(text) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    match serde_json::from_str(&text) {
        Ok(left) => left,
        Err(error) => {
            // Put aside, never written over: it may hold readings to delete.
            let aside = file.with_extension("json.ilegible");
            eprintln!("[core] {} cannot be read ({error}); it is kept as {}", file.display(), aside.display());
            let _ = std::fs::rename(file, aside);
            Vec::new()
        }
    }
}

fn save(file: &Path, left: &[LeftReading]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(left).map_err(|e| e.to_string())?;
    let partial = file.with_extension("json.parcial");
    std::fs::write(&partial, text).map_err(|e| e.to_string())?;
    std::fs::rename(&partial, file).map_err(|e| e.to_string())
}

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// The ledger is kept in `data_dir` (the app's data folder). Called once,
/// when the app starts, before anything reads with Core.
pub fn install(data_dir: &Path) {
    let _ = FILE.set(data_dir.join(LEDGER_FILE));
}

/// The ledger of the app: in its data folder once `install` was called,
/// only in memory otherwise (the tests).
pub fn shared() -> Arc<Ledger> {
    static SHARED: OnceLock<Arc<Ledger>> = OnceLock::new();
    SHARED
        .get_or_init(|| {
            Arc::new(match FILE.get() {
                Some(file) => Ledger::at(file),
                None => Ledger::in_memory(),
            })
        })
        .clone()
}

/// Deletes in Core the readings of `ledger` due now (`Ledger::due`), and
/// stops at the first cut: Core is tried again later. Returns how many were
/// deleted.
pub fn delete_left(client: &CoreClient, ledger: &Ledger) -> usize {
    let mut deleted = 0;
    let mut cut: Option<CoreError> = None;
    for left in ledger.due(now_ms()) {
        if cut.is_some() {
            ledger.release(&left.id);
            continue;
        }
        match client.delete_reading(&left.id) {
            Ok(()) => {
                ledger.deleted(&left.id);
                deleted += 1;
            }
            Err(error) if error.is_transient() => {
                ledger.not_deleted(&left.id, false, None);
                cut = Some(error);
            }
            Err(error) => {
                eprintln!("[core] a reading left in Micelya Core could not be deleted yet: {}", error.message());
                ledger.not_deleted(&left.id, false, Some(error.message()));
            }
        }
    }
    if let Some(error) = cut {
        eprintln!("[core] the readings left in Micelya Core are tried again later: {}", error.message());
    }
    deleted
}

/// Tries again to delete in Core the readings of this computer left there
/// (P-294): at most every `RETRY_LEFT_EVERY`, or now when `now` (a person
/// asked), with the connection of the keychain. Nothing is read from the
/// keychain nor sent when there is nothing to delete. Returns whether it
/// tried (what Atención shows may have changed).
pub fn retry_left_readings(now: bool) -> bool {
    let ledger = shared();
    if !ledger.any_due(now_ms()) {
        return false;
    }
    static LAST: Mutex<Option<Instant>> = Mutex::new(None);
    {
        let mut last = LAST.lock().unwrap_or_else(|p| p.into_inner());
        if !now && last.is_some_and(|at| at.elapsed() < RETRY_LEFT_EVERY) {
            return false;
        }
        *last = Some(Instant::now());
    }
    let Ok(Some(connection)) = saved_connection() else {
        return false;
    };
    // Once each, and soon: the worker waits for it.
    let timing = Timing { call: Duration::from_secs(15), retry_waits: Vec::new(), ..Timing::default() };
    let Ok(client) = CoreClient::new(connection, timing) else {
        return false;
    };
    delete_left(&client, &ledger);
    true
}

#[cfg(test)]
mod tests {
    use super::super::fake::FakeCore;
    use super::*;

    fn scratch_file() -> PathBuf {
        std::env::temp_dir().join(format!("micelya-ledger-test-{}", uuid::Uuid::new_v4())).join(LEDGER_FILE)
    }

    #[test]
    fn the_readings_left_are_kept_in_their_file_and_read_again() {
        let file = scratch_file();
        let ledger = Ledger::at(&file);
        ledger.asked("lectura-1", "pagina-1.jpg");
        ledger.asked("lectura-2", "pagina-2.jpg");
        ledger.not_deleted("lectura-2", true, Some("Micelya Core rejected the credential (API, 403)".into()));
        ledger.deleted("lectura-1");

        let again = Ledger::at(&file);
        let left = again.left();
        assert_eq!(left.len(), 1);
        assert_eq!((left[0].id.as_str(), left[0].name.as_str(), left[0].kept), ("lectura-2", "pagina-2.jpg", true));
        assert!(left[0].refused.as_deref().unwrap().contains("403"));
        assert!(again.is_ours("lectura-2") && !again.is_ours("lectura-1"));
        // Nothing of the documents: identifiers, names and reasons.
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains("lectura-2") && !text.contains("lectura-1"), "{text}");

        // A file that cannot be read is put aside, not written over.
        std::fs::write(&file, "no es json").unwrap();
        assert!(Ledger::at(&file).left().is_empty());
        assert!(file.with_extension("json.ilegible").is_file());
        let _ = std::fs::remove_dir_all(file.parent().unwrap());
    }

    #[test]
    fn only_what_no_walk_is_on_is_due_and_a_page_not_kept_waits_to_be_given_back() {
        let ledger = Ledger::in_memory();
        ledger.asked("en-curso", "pagina-1.jpg");
        ledger.asked("rechazada", "pagina-2.jpg");
        ledger.not_deleted("rechazada", true, Some("403".into()));
        ledger.asked("cortada", "pagina-3.jpg");
        ledger.not_deleted("cortada", false, None);
        let now = now_ms();
        // Under way: never; refused: yes; a page not kept: once it is left behind.
        let due: Vec<String> = ledger.due(now).into_iter().map(|left| left.id).collect();
        assert_eq!(due, ["rechazada"]);
        // Taken: not given twice until it is told.
        assert!(ledger.due(now).is_empty());
        ledger.release("rechazada");
        let later = now + LEFT_BEHIND.as_millis() as i64 + 1;
        let due: Vec<String> = ledger.due(later).into_iter().map(|left| left.id).collect();
        assert_eq!(due, ["rechazada", "cortada"]);
        // A walk takes only what is its computer's and free.
        ledger.release("cortada");
        assert!(ledger.adopt("cortada") && !ledger.adopt("cortada") && !ledger.adopt("de-otra-computadora"));
    }

    #[test]
    fn what_core_refused_to_delete_is_deleted_later_and_a_cut_stops_the_retry() {
        let core = FakeCore::start();
        let client = CoreClient::new(core.connection(), FakeCore::timing()).unwrap();
        let ledger = Ledger::in_memory();
        for n in 1..=3 {
            ledger.asked(&format!("lectura-{n}"), &format!("pagina-{n}.jpg"));
            ledger.not_deleted(&format!("lectura-{n}"), true, Some("403".into()));
        }
        // Core is not available: the first fails, the rest wait for later.
        core.with(|state| state.delete_failures = vec![503, 503, 503]);
        assert_eq!(delete_left(&client, &ledger), 0);
        assert_eq!(ledger.left().len(), 3);
        assert!(ledger.left().iter().all(|left| left.refused.as_deref() == Some("403")));
        // Still refused for one, deleted the others (already gone counts as deleted).
        core.with(|state| state.delete_failures = vec![403]);
        assert_eq!(delete_left(&client, &ledger), 2);
        let left = ledger.refused();
        assert_eq!(left.len(), 1);
        assert!(left[0].refused.as_deref().unwrap().contains("rejected the credential"), "{left:?}");
        assert_eq!(delete_left(&client, &ledger), 1);
        assert!(ledger.left().is_empty());
        // Only what this computer asked for was ever deleted.
        let deletes: Vec<String> = core.seen().into_iter().filter(|s| s.method == "DELETE").map(|s| s.path).collect();
        assert!(deletes.iter().all(|path| path.starts_with("/v1/tenant/document-readings/lectura-")), "{deletes:?}");
    }
}
