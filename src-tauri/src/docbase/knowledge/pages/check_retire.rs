//! Measures the base without the wiki's folder (D-402, D-403) over a copy
//! of a real base and a copy of its wiki, offline: the same pages as the
//! wiki had, no file written, no question to the model, and every note of
//! a person and saved query of the wiki in the base.
//!
//! Not part of the normal test run. Nothing of the real data is kept in
//! the repository: it only prints counts.
//!
//! ```text
//! MICELYA_DOCBASE_COPY=<copy of the base> MICELYA_WIKI_COPY=<copy of the wiki project> \
//!     cargo test --lib check_retire -- --ignored --nocapture --test-threads=1
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::docbase::knowledge::check::open_base;
use crate::docbase::testing::TempDir;

/// Every file under `root`: path, size, modification time and bytes.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, (u64, Option<std::time::SystemTime>, Vec<u8>)> {
    fn walk(dir: &Path, out: &mut BTreeMap<PathBuf, (u64, Option<std::time::SystemTime>, Vec<u8>)>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if let Ok(meta) = entry.metadata() {
                out.insert(path.clone(), (meta.len(), meta.modified().ok(), std::fs::read(&path).unwrap_or_default()));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, &mut out);
    out
}

/// A model that answers nothing and counts what it is asked.
struct Offline {
    engine: String,
    asked: AtomicUsize,
}

impl DocumentAsker for Offline {
    fn engine_id(&self) -> String {
        self.engine.clone()
    }

    fn ask(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
        self.asked.fetch_add(1, Ordering::SeqCst);
        Err("offline: no question may be asked".into())
    }
}

fn kind(id: &str) -> &str {
    id.split('-').next().unwrap_or_default()
}

/// A page's text without the date it was written.
fn undated(text: &str) -> String {
    text.replace("\r\n", "\n").lines().filter(|l| !l.starts_with("updated:")).collect::<Vec<_>>().join("\n").trim_end().to_string()
}

#[test]
#[ignore = "reads a copy of a real base and of its wiki"]
fn the_base_keeps_the_pages_without_the_folder() {
    let (Ok(copy), Ok(wiki)) = (std::env::var("MICELYA_DOCBASE_COPY"), std::env::var("MICELYA_WIKI_COPY")) else {
        println!("MICELYA_DOCBASE_COPY and MICELYA_WIKI_COPY are not set: nothing to measure");
        return;
    };
    let wiki = PathBuf::from(wiki);
    // What the base had before: its pages, by kind.
    let before: BTreeMap<String, String> = {
        let conn = rusqlite::Connection::open_with_flags(&copy, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut statement = conn.prepare("SELECT entity_id, path FROM knowledge_page WHERE state = 'projected'").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let by_kind = |pages: &BTreeMap<String, String>| {
        let mut out: BTreeMap<String, usize> = BTreeMap::new();
        for id in pages.keys() {
            *out.entry(kind(id).to_string()).or_default() += 1;
        }
        out
    };
    println!("== before ==\n   pages of the base: {} {:?}", before.len(), by_kind(&before));

    let Some((_scratch, base)) = open_base() else { return };
    let version: i64 = base.conn().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    println!("   schema after opening: {version}; background reading: {}", base.background_reading().unwrap());

    // The notes and queries of the wiki's copy, counted on their own.
    let words = vocabulary();
    let mut found_notes = 0;
    let mut pages_read = 0;
    for folder in [words.pages.folder(), words.pages.documents_folder()] {
        for file in import::markdown_files(&wiki.join(folder)) {
            pages_read += 1;
            let text = std::fs::read_to_string(&file).unwrap_or_default();
            if !notes_of(&place_block(&text, None), words.pages.word("notes")).trim().is_empty() {
                found_notes += 1;
            }
        }
    }
    let found_queries = import::markdown_files(&wiki.join("wiki/queries")).len();
    println!("   in the wiki's copy: {pages_read} pages, {found_notes} with notes of a person, {found_queries} saved queries");

    // The import reads the copy of the wiki, never the real folder.
    let wiki_before = snapshot(&wiki);
    base.conn().execute("UPDATE project SET path = ?1 WHERE kind = 'general'", [wiki.to_string_lossy().replace('\\', "/")]).unwrap();
    let imported = base.import_wiki_files().unwrap();
    println!("\n== import ==\n   {imported:?}");
    let notes_in_base: i64 = base.conn().query_row("SELECT count(*) FROM knowledge_note", [], |row| row.get(0)).unwrap();
    let queries_in_base: i64 = base.conn().query_row("SELECT count(*) FROM knowledge_query", [], |row| row.get(0)).unwrap();
    println!(
        "   notes found {found_notes}, imported {} (to objects {}, kept apart {}); rows of notes {notes_in_base}; queries found {found_queries}, in the base {queries_in_base}",
        imported.to_objects + imported.orphans,
        imported.to_objects,
        imported.orphans
    );
    println!("   again: skipped {}", base.import_wiki_files().unwrap().skipped);

    // The projection, with the general project on an empty folder: it
    // needs none, and writes in none.
    let empty = TempDir::new();
    base.conn().execute("UPDATE project SET path = ?1 WHERE kind = 'general'", [empty.path().to_string_lossy().replace('\\', "/")]).unwrap();
    let engine = classify::ask_engine(&base.conn()).unwrap().map(|c| crate::source_volume::asking::engine_id(&c)).unwrap_or_default();
    let pending = base.synthesis_questions(&engine).unwrap();
    let offline = Offline { engine: engine.clone(), asked: AtomicUsize::new(0) };
    let report = base.knowledge_project(Some(&offline), 10_000).unwrap();
    println!("\n== projection ==\n   questions not kept for {engine}: {}; asked: {}", pending.len(), offline.asked.load(Ordering::SeqCst));
    println!("   {report:?}");
    let again = base.knowledge_project(Some(&offline), 10_000).unwrap();
    println!("   again: skipped {} written {} documents {} asked {}", again.skipped, again.written, again.blocks_written, offline.asked.load(Ordering::SeqCst));
    // And without any wiki.
    base.conn().execute("UPDATE project SET path = NULL WHERE kind = 'general'", []).unwrap();
    super::super::mark_dirty(&base.conn(), "project", "check").unwrap();
    let without = base.knowledge_project(Some(&offline), 10_000).unwrap();
    println!(
        "   without a wiki: skipped {} written {} documents {} asked {}",
        without.skipped,
        without.written,
        without.blocks_written,
        offline.asked.load(Ordering::SeqCst)
    );

    let after: BTreeMap<String, String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT entity_id, path FROM knowledge_page WHERE state = 'projected' AND body IS NOT NULL").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let same: BTreeSet<&String> = before.iter().filter(|(id, path)| after.get(*id) == Some(*path)).map(|(id, _)| id).collect();
    println!("\n== after ==\n   pages of the base with their body: {} {:?}", after.len(), by_kind(&after));
    println!(
        "   the same object and path as before: {}; only before: {:?}; only after: {:?}",
        same.len(),
        before.keys().filter(|id| !same.contains(id)).map(|id| kind(id)).collect::<Vec<_>>(),
        after.keys().filter(|id| !before.contains_key(*id)).map(|id| kind(id)).collect::<Vec<_>>()
    );
    // Each page as one text, against the file the wiki had.
    let mut equal: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for (id, path) in &after {
        let text = base.knowledge_page_text(path).unwrap().map(|(_, t)| t).unwrap_or_default();
        let file = std::fs::read_to_string(wiki.join(path)).unwrap_or_default();
        let entry = equal.entry(kind(id).to_string()).or_default();
        entry.1 += 1;
        if undated(&text) == undated(&file) {
            entry.0 += 1;
        }
    }
    println!("   pages whose text is the wiki file's, by kind (equal, of): {equal:?}");
    let overview = base.knowledge_overview().unwrap();
    let overview_file = std::fs::read_to_string(wiki.join(OVERVIEW_PATH)).unwrap_or_default();
    println!(
        "   overview in the base: {}; the same as the wiki's file: {}",
        overview.is_some(),
        overview.as_ref().is_some_and(|o| undated(&o.body) == undated(&overview_file))
    );

    // Not a file written: neither in the empty folder nor in the wiki's copy.
    let written_in_empty = snapshot(empty.path()).len();
    let wiki_after = snapshot(&wiki);
    let changed = wiki_before.iter().filter(|(path, before)| wiki_after.get(*path) != Some(*before)).count()
        + wiki_after.keys().filter(|path| !wiki_before.contains_key(*path)).count();
    println!("\n== files ==\n   written in the empty folder: {written_in_empty}; changed or new in the wiki's copy: {changed} (of {})", wiki_before.len());
    println!("   questions asked to the model in all: {}", offline.asked.load(Ordering::SeqCst));
}
