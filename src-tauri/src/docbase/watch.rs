//! The watcher (A3 of 2 nov, change A of the approval): it tells the
//! process that something changed in a folder connected as an origin.
//!
//! **It is the watcher of the wiki, recovered,** not a new one. It left with
//! the wiki on 21 oct (commit 0a43023f): `commands/file_sync.rs`, which
//! watched with `notify` and gathered the paths of the changes, and
//! `source_volume/watch.rs`, which told an origin that cannot be reached
//! from one that was emptied. What it keeps, adapted to the base of
//! documents and to the loop of the process (E4 of 2 nov):
//!
//! - **The same dependency and the same watch:** `notify`, recursive, over
//!   the real folder of each origin (in Windows, the notices of the system).
//!   An origin that cannot be watched now is not an error: the sweep of
//!   backup still sees its changes, and the watch is tried again after
//!   every sweep (`follow`), as the old one did with a mounted origin.
//! - **It waits for the folder to calm down** before telling, as the old
//!   one gathered the paths until 700 ms passed without a notice. Here the
//!   quiet is counted per origin and is longer (`QUIET`): a copy of many
//!   files is taken at once, when it ends.
//! - **A notice lost or failed is a change of everything,** as the old one
//!   sent its root to look again at all of it: an overflow of the notices
//!   or an error of the watch marks every origin.
//! - **The same files are left out** as the inventory leaves them
//!   (`scan::is_inventoried`): what the system, an editor or a download
//!   leaves around, like the old filters (`~$*`, `.tmp`, `.crdownload`...).
//! - **The sweep of backup stays** for the folders of the network and the
//!   cloud that do not tell: every 5 minutes (`process::run::SWEEP_EVERY`),
//!   in place of the old reconciliation every minute.
//!
//! What changes:
//!
//! - **It only writes down that an origin changed** (`origin.changed_at`)
//!   and wakes the loop. The old one kept a snapshot and a queue of its own
//!   in files of the project, and ran the ingest itself; here the inventory
//!   (the step 1) is incomplete while a change is newer than the last sweep,
//!   and the loop sweeps that origin as one more operation, after the one
//!   going on, never in the middle of it.
//! - **It has no thread of its own** (there is only the loop, E4): the
//!   thread of the notices of `notify` only writes down when it heard the
//!   last change of each origin, and wakes the loop on the first one; the
//!   loop counts the quiet when it looks at its wakes (`calmed`), and sleeps
//!   until the next origin calms down (`next_calm`).
//! - **No snapshot, no queue, no generations, no writes of the app to
//!   ignore:** the base is the snapshot (the inventory compares the size and
//!   the date of each file), a change of the app in a folder costs one
//!   sweep more, and there is one watch for the life of the app.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use super::process::run::Wake;

/// How long an origin must stay without a notice before its change is
/// written down: a folder being copied is taken when it calms down.
pub const QUIET: Duration = Duration::from_secs(3);

/// A path as the watcher compares it: forward slashes, no slash at the
/// end, and in lower case on Windows, where the case does not matter.
fn compared(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.trim_end_matches('/').to_string();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

/// What the watcher heard and did not write down yet: the folders it
/// watches, and when it heard the last change of each origin. The thread of
/// the notices writes here; the loop takes what calmed down.
#[derive(Default)]
pub(crate) struct Heard {
    /// (origin, its folder as compared).
    roots: Vec<(String, String)>,
    /// The last change heard of each origin not written down yet.
    last: BTreeMap<String, Instant>,
}

impl Heard {
    pub(crate) fn set_roots(&mut self, roots: &[(String, PathBuf)]) {
        self.roots = roots.iter().map(|(id, root)| (id.clone(), compared(root))).collect();
        // The longest first: an origin inside another one takes its own.
        self.roots.sort_by_key(|(_, root)| std::cmp::Reverse(root.len()));
        let ids: Vec<&String> = roots.iter().map(|(id, _)| id).collect();
        self.last.retain(|id, _| ids.contains(&id));
    }

    /// The origin of `path`, and the path inside it.
    fn origin_of(&self, path: &Path) -> Option<(String, String)> {
        let path = compared(path);
        self.roots.iter().find_map(|(id, root)| {
            if path == *root {
                Some((id.clone(), String::new()))
            } else {
                path.strip_prefix(&format!("{root}/")).map(|rel| (id.clone(), rel.to_string()))
            }
        })
    }

    /// A change at `path`, heard at `at`. It counts only when the inventory
    /// would look at that path. Returns whether its origin was quiet until
    /// now: then the loop is woken to count its quiet.
    pub(crate) fn note(&mut self, path: &Path, at: Instant) -> bool {
        let Some((id, rel)) = self.origin_of(path) else {
            return false;
        };
        if !rel.is_empty() && !super::scan::is_inventoried(&rel) {
            return false;
        }
        self.last.insert(id, at).is_none()
    }

    /// Every origin changed: a notice was lost or the watch failed.
    pub(crate) fn note_all(&mut self, at: Instant) -> bool {
        let mut first = false;
        for (id, _) in &self.roots {
            first |= self.last.insert(id.clone(), at).is_none();
        }
        first
    }

    /// The origins that stayed quiet for `QUIET` until `now`: taken away,
    /// to be written down.
    pub(crate) fn calmed(&mut self, now: Instant) -> Vec<String> {
        let calm: Vec<String> =
            self.last.iter().filter(|(_, at)| now.saturating_duration_since(**at) >= QUIET).map(|(id, _)| id.clone()).collect();
        for id in &calm {
            self.last.remove(id);
        }
        calm
    }

    /// How long until the next origin calms down, when one is waiting.
    pub(crate) fn next_calm(&self, now: Instant) -> Option<Duration> {
        self.last.values().map(|at| QUIET.saturating_sub(now.saturating_duration_since(*at))).min()
    }
}

fn lock(heard: &Mutex<Heard>) -> MutexGuard<'_, Heard> {
    heard.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The watch over the folders of the origins, for the life of the loop.
pub struct Watch {
    heard: Arc<Mutex<Heard>>,
    watcher: Option<RecommendedWatcher>,
    /// The folders watched now, by origin.
    watched: BTreeMap<String, PathBuf>,
}

impl Watch {
    /// Starts the watch; it watches nothing until `follow`. Without a watch
    /// of the system (it could not start), only the sweep of backup sees the
    /// changes.
    pub fn start(wake: Arc<Wake>) -> Watch {
        let heard = Arc::new(Mutex::new(Heard::default()));
        let for_notices = heard.clone();
        let watcher = RecommendedWatcher::new(
            move |notice: notify::Result<Event>| {
                let now = Instant::now();
                let first = {
                    let mut heard = lock(&for_notices);
                    match notice {
                        // Reading a file is not changing it (the app reads
                        // them to know their bytes).
                        Ok(event) if matches!(event.kind, EventKind::Access(_)) => false,
                        Ok(event) if event.need_rescan() => heard.note_all(now),
                        Ok(event) => event.paths.iter().fold(false, |first, path| heard.note(path, now) | first),
                        Err(error) => {
                            eprintln!("[docbase] the watch of the folders failed; every origin is swept: {error}");
                            heard.note_all(now)
                        }
                    }
                };
                if first {
                    wake.request();
                }
            },
            Config::default(),
        );
        let watcher = match watcher {
            Ok(watcher) => Some(watcher),
            Err(error) => {
                eprintln!("[docbase] the watch of the folders could not start; only the sweep of backup sees changes: {error}");
                None
            }
        };
        Watch { heard, watcher, watched: BTreeMap::new() }
    }

    /// Watches the folder of each origin of `origins` (id, folder) that can
    /// be read now, and stops watching the others. A folder that cannot be
    /// watched now (not connected, a network share down) is left to the
    /// sweep of backup, and tried again the next time.
    pub fn follow(&mut self, origins: &[(String, PathBuf)]) {
        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };
        let gone: Vec<String> = self
            .watched
            .iter()
            .filter(|(id, root)| !root.is_dir() || !origins.iter().any(|(other, path)| other == *id && path == *root))
            .map(|(id, _)| id.clone())
            .collect();
        for id in gone {
            if let Some(root) = self.watched.remove(&id) {
                let _ = watcher.unwatch(&root);
            }
        }
        for (id, root) in origins {
            if self.watched.contains_key(id) || !root.is_dir() {
                continue;
            }
            match watcher.watch(root, RecursiveMode::Recursive) {
                Ok(()) => {
                    self.watched.insert(id.clone(), root.clone());
                }
                Err(error) => eprintln!("[docbase] the folder '{}' cannot be watched now; the sweep of backup sees it: {error}", root.display()),
            }
        }
        lock(&self.heard).set_roots(origins);
    }

    /// The origins whose change calmed down: to be written down now.
    pub fn calmed(&self) -> Vec<String> {
        lock(&self.heard).calmed(Instant::now())
    }

    /// How long until the next origin calms down, when one is waiting.
    pub fn next_calm(&self) -> Option<Duration> {
        lock(&self.heard).next_calm(Instant::now())
    }

    /// Whether the system watches the folder of `origin` now.
    #[cfg(test)]
    pub(crate) fn watches(&self, origin: &str) -> bool {
        self.watched.contains_key(origin)
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
