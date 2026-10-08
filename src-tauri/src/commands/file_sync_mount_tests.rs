//! Change detection over mounted origins: their files are tracked under
//! logical `raw/sources/<mount>/…` paths, and an origin that cannot be
//! reached never turns into deletions.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use super::*;
use crate::source_volume::SourceVolume;

const A: &str = "raw/sources/Docs/a.md";
const B: &str = "raw/sources/Docs/sub/b.md";

struct Fixture {
    base: PathBuf,
    project: PathBuf,
    origin: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let base = std::env::temp_dir().join(format!(
            "micelya-file-sync-mount-{}",
            uuid::Uuid::new_v4()
        ));
        let project = base.join("project");
        let origin = base.join("origin");
        fs::create_dir_all(project.join("raw/sources")).unwrap();
        fs::create_dir_all(origin.join("sub")).unwrap();
        fs::write(origin.join("a.md"), "alpha").unwrap();
        fs::write(origin.join("sub/b.md"), "beta").unwrap();
        SourceVolume::open(&project)
            .unwrap()
            .add_local_mount("Docs", &origin)
            .unwrap();
        ensure_sync_dir(&project).unwrap();
        Self {
            base,
            project,
            origin,
        }
    }

    fn rescan(&self) {
        enqueue_rescan_changes_for_prefixes(
            &self.project,
            "p1",
            &["raw/sources"],
            &SourceWatchConfig::default(),
        )
        .unwrap();
    }

    /// Pending changes as (path, kind), then applied to the snapshot.
    fn take_changes(&self) -> Vec<(String, FileChangeKind)> {
        let changes = read_queue(&self.project)
            .unwrap()
            .tasks
            .into_iter()
            .map(|task| (task.path, task.kind))
            .collect();
        process_queue_inner(&self.project, "p1", |_| {}, |_| {}).unwrap();
        changes
    }

    fn tracked(&self) -> Vec<String> {
        read_snapshot(&self.project)
            .unwrap()
            .files
            .into_keys()
            .collect()
    }

    fn origin_files(&self) -> usize {
        walkdir::WalkDir::new(&self.origin)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .count()
    }

    fn disconnect_origin(&self) -> PathBuf {
        let parked = self.base.join("origin-disconnected");
        fs::rename(&self.origin, &parked).unwrap();
        parked
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

fn created(path: &str) -> (String, FileChangeKind) {
    (path.to_string(), FileChangeKind::Created)
}

#[test]
fn mounted_files_are_tracked_under_their_logical_paths() {
    let fx = Fixture::new();

    fx.rescan();

    assert_eq!(fx.take_changes(), vec![created(A), created(B)]);
    assert_eq!(fx.tracked(), vec![A.to_string(), B.to_string()]);
    assert_eq!(fx.origin_files(), 2, "nothing may be written to the origin");
}

#[test]
fn an_unchanged_origin_produces_no_changes() {
    let fx = Fixture::new();
    fx.rescan();
    fx.take_changes();

    fx.rescan();

    assert!(fx.take_changes().is_empty());
}

#[test]
fn edits_and_removals_inside_a_reachable_origin_are_detected() {
    let fx = Fixture::new();
    fx.rescan();
    fx.take_changes();

    fs::write(fx.origin.join("a.md"), "alpha, edited and longer").unwrap();
    fs::remove_file(fx.origin.join("sub/b.md")).unwrap();
    fx.rescan();

    assert_eq!(
        fx.take_changes(),
        vec![
            (A.to_string(), FileChangeKind::Modified),
            (B.to_string(), FileChangeKind::Deleted),
        ]
    );
    assert_eq!(fx.tracked(), vec![A.to_string()]);
}

#[test]
fn a_disconnected_origin_deletes_nothing_and_resumes_when_it_returns() {
    let fx = Fixture::new();
    fx.rescan();
    fx.take_changes();

    let parked = fx.disconnect_origin();
    fx.rescan();
    enqueue_rescan_changes(&fx.project, "p1", &SourceWatchConfig::default()).unwrap();
    enqueue_paths(&fx.project, "p1", BTreeSet::from([A.to_string(), B.to_string()])).unwrap();

    assert!(fx.take_changes().is_empty());
    assert_eq!(fx.tracked(), vec![A.to_string(), B.to_string()]);

    fs::rename(&parked, &fx.origin).unwrap();
    fx.rescan();
    assert!(fx.take_changes().is_empty());
}

#[test]
fn an_origin_that_shows_up_empty_deletes_nothing() {
    let fx = Fixture::new();
    fx.rescan();
    fx.take_changes();

    fs::remove_dir_all(fx.origin.join("sub")).unwrap();
    fs::remove_file(fx.origin.join("a.md")).unwrap();
    fx.rescan();

    assert!(fx.take_changes().is_empty());
    assert_eq!(fx.tracked(), vec![A.to_string(), B.to_string()]);
}

#[test]
fn an_unreadable_mount_table_deletes_nothing() {
    let fx = Fixture::new();
    fx.rescan();
    fx.take_changes();

    fs::write(fx.project.join(".llm-wiki/source-mounts.json"), "{ not json").unwrap();
    fx.rescan();

    assert!(fx.take_changes().is_empty());
    assert_eq!(fx.tracked(), vec![A.to_string(), B.to_string()]);
}

#[test]
fn a_full_rescan_tracks_mounted_and_copied_sources_together() {
    let fx = Fixture::new();
    fs::write(fx.project.join("raw/sources/copied.md"), "copied").unwrap();

    enqueue_rescan_changes(&fx.project, "p1", &SourceWatchConfig::default()).unwrap();

    assert_eq!(
        fx.take_changes(),
        vec![created(A), created(B), created("raw/sources/copied.md")]
    );
}

#[test]
fn copied_sources_are_still_reported_deleted_when_removed() {
    let fx = Fixture::new();
    fs::write(fx.project.join("raw/sources/copied.md"), "copied").unwrap();
    fx.rescan();
    fx.take_changes();

    fs::remove_file(fx.project.join("raw/sources/copied.md")).unwrap();
    fx.rescan();

    assert_eq!(
        fx.take_changes(),
        vec![("raw/sources/copied.md".to_string(), FileChangeKind::Deleted)]
    );
}
