//! The watcher (A3 of 2 nov): what it hears, how it waits for a folder to
//! calm down, and the real watch of the system over a folder of the test.
//! Invented files only (R-24).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::super::process::run::Wake;
use super::super::testing::TempDir;
use super::{Heard, Watch, QUIET};

fn heard(roots: &[(&str, &str)]) -> Heard {
    let mut heard = Heard::default();
    let roots: Vec<(String, PathBuf)> = roots.iter().map(|(id, root)| (id.to_string(), PathBuf::from(root))).collect();
    heard.set_roots(&roots);
    heard
}

#[test]
fn a_copy_of_many_files_is_taken_at_once_when_the_folder_calms_down() {
    let mut heard = heard(&[("o", "C:/Archivo/Clientes")]);
    let start = Instant::now();
    // 500 files land over two seconds: only the first one wakes the loop.
    let mut woke = 0;
    for n in 0..500u64 {
        let at = start + Duration::from_millis(n * 4);
        woke += usize::from(heard.note(&PathBuf::from(format!("C:/Archivo/Clientes/caso/hoja-{n}.pdf")), at));
    }
    assert_eq!(woke, 1);
    let last = start + Duration::from_millis(499 * 4);
    // While the copy goes on, or before it is quiet for long enough, nothing.
    assert!(heard.calmed(last + QUIET - Duration::from_millis(1)).is_empty());
    assert_eq!(heard.next_calm(last), Some(QUIET));
    // Quiet: taken once, all of it.
    assert_eq!(heard.calmed(last + QUIET), vec!["o".to_string()]);
    assert!(heard.calmed(last + QUIET * 2).is_empty());
    assert_eq!(heard.next_calm(last + QUIET), None);
}

#[test]
fn what_the_inventory_leaves_out_and_what_is_outside_every_origin_is_not_heard() {
    let mut heard = heard(&[("o", "C:/Archivo")]);
    let now = Instant::now();
    for path in ["C:/Archivo/~$carta.docx", "C:/Archivo/.git/index", "C:/Archivo/desktop.ini", "C:/Archivo/a.pdf.crdownload", "C:/Otra/a.pdf"] {
        assert!(!heard.note(&PathBuf::from(path), now), "{path}");
    }
    assert_eq!(heard.next_calm(now), None);
    // A file of the user is; after it, its origin is already waiting.
    assert!(heard.note(&PathBuf::from("C:/Archivo/caso/carta.pdf"), now));
    assert!(!heard.note(&PathBuf::from("C:/Archivo/caso"), now));
    assert_eq!(heard.calmed(now + QUIET), vec!["o".to_string()]);
}

#[test]
fn an_origin_inside_another_one_takes_its_own_changes() {
    let mut heard = heard(&[("todo", "C:/Archivo"), ("caso", "C:/Archivo/caso")]);
    let now = Instant::now();
    assert!(heard.note(&PathBuf::from("C:/Archivo/caso/carta.pdf"), now));
    assert_eq!(heard.calmed(now + QUIET), vec!["caso".to_string()]);
}

#[cfg(windows)]
#[test]
fn in_windows_the_case_and_the_slashes_of_a_path_do_not_matter() {
    let mut heard = heard(&[("o", "C:\\Archivo\\Clientes\\")]);
    assert!(heard.note(&PathBuf::from("c:\\archivo\\clientes\\nuevo.pdf"), Instant::now()));
}

#[test]
fn a_notice_lost_or_a_watch_that_failed_is_a_change_of_every_origin() {
    let mut heard = heard(&[("a", "C:/A"), ("b", "C:/B")]);
    let now = Instant::now();
    assert!(heard.note_all(now));
    let mut calm = heard.calmed(now + QUIET);
    calm.sort();
    assert_eq!(calm, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn an_origin_removed_is_no_longer_heard() {
    let mut heard = heard(&[("a", "C:/A"), ("b", "C:/B")]);
    let now = Instant::now();
    heard.note(&PathBuf::from("C:/B/x.pdf"), now);
    heard.set_roots(&[("a".to_string(), PathBuf::from("C:/A"))]);
    assert!(heard.calmed(now + QUIET).is_empty());
    assert!(!heard.note(&PathBuf::from("C:/B/y.pdf"), now));
}

/// Waits, a little at a time, until the watch has calmed origins.
fn until_calmed(watch: &Watch, limit: Duration) -> Vec<String> {
    let start = Instant::now();
    while start.elapsed() < limit {
        let calm = watch.calmed();
        if !calm.is_empty() {
            return calm;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    Vec::new()
}

#[test]
fn the_watch_of_the_system_hears_a_new_file_in_a_watched_folder_and_wakes_the_loop() {
    let dir = TempDir::new();
    dir.write("o/antes.txt", b"ya estaba");
    let wake = Arc::new(Wake::default());
    let mut watch = Watch::start(wake.clone());
    watch.follow(&[("o".to_string(), dir.path().join("o"))]);
    assert!(watch.watches("o"));
    // Give the system a moment to start telling.
    std::thread::sleep(Duration::from_millis(300));
    let copied = Instant::now();
    dir.write("o/caso/nuevo.txt", b"un archivo nuevo de la prueba");
    // The loop is woken by the first change...
    let woke = (0..50).any(|_| {
        std::thread::sleep(Duration::from_millis(100));
        wake.take().0
    });
    assert!(woke, "the first change wakes the loop");
    // ... and the change is taken once the folder is quiet.
    assert_eq!(until_calmed(&watch, Duration::from_secs(20)), vec!["o".to_string()]);
    assert!(copied.elapsed() >= QUIET);
    // A folder that went away is no longer watched; the sweep sees it.
    watch.follow(&[]);
    assert!(!watch.watches("o"));
}
