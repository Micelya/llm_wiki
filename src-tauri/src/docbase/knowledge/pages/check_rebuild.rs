//! Measures, over a copy of a real base and offline, what writing every
//! page again costs and what the pages look like after it: the steps of
//! the background work in their order (types, reading, identities,
//! matters, pages), each with how many questions it would send to the
//! model, and then what the pages of the base say (sections that should
//! not be there, notes, dates and titles of documents).
//!
//! Not part of the normal test run. Nothing of the real data is kept in
//! the repository: it prints counts, and copies a few pages to a folder
//! outside it when asked.
//!
//! ```text
//! MICELYA_DOCBASE_COPY=<copy of the base> MICELYA_PAGES_SAMPLE=<folder outside the repository> \
//!     MICELYA_KNOWLEDGE_SAVE=<a file outside the repository, for the base after the run> \
//!     cargo test --lib check_rebuild -- --ignored --nocapture --test-threads=1
//! ```

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::docbase::knowledge::check::open_base;

/// A model that answers nothing and counts what it is asked; with
/// `identities`, a stand-in: a question about identities gets "no
/// decision", so the resolution can finish, and one of a synthesis gets no
/// sentence (both counted apart). Never the model.
struct Offline {
    engine: String,
    identities: bool,
    asked: AtomicUsize,
    stand_in: AtomicUsize,
}

impl DocumentAsker for Offline {
    fn engine_id(&self) -> String {
        self.engine.clone()
    }

    fn ask(&self, _prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        if self.identities && schema.pointer("/properties/respuestas").is_some() {
            self.stand_in.fetch_add(1, Ordering::SeqCst);
            return Ok(serde_json::json!({ "respuestas": [] }).to_string());
        }
        // And, to see a whole rebuild from answers kept, a synthesis gets
        // no sentence.
        if self.identities && schema.pointer("/properties/oraciones").is_some() {
            self.stand_in.fetch_add(1, Ordering::SeqCst);
            return Ok(serde_json::json!({ "oraciones": [] }).to_string());
        }
        self.asked.fetch_add(1, Ordering::SeqCst);
        Err("offline: no question may be asked".into())
    }
}

/// Whether a page has a section under `## heading` or `### heading`.
fn has_section(body: &str, heading: &str) -> bool {
    body.lines().any(|line| line.starts_with("##") && line.trim_start_matches('#').trim() == heading)
}

/// Whether a text writes a date with an English month ("March 3, 2021").
fn english_date(text: &str) -> bool {
    let tokens: Vec<String> = text.split_whitespace().map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase()).collect();
    tokens.windows(2).any(|w| {
        let month = |t: &str| ENGLISH_MONTHS.iter().any(|m| *m == t || (t.len() == 3 && m.starts_with(t) && t != "may"));
        let day = |t: &str| t.parse::<u32>().is_ok_and(|d| (1..=31).contains(&d));
        (month(&w[0]) && day(&w[1])) || (day(&w[0]) && month(&w[1]))
    })
}

/// A page without its quotes (what a document writes is kept as written).
fn without_quotes(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '«' => depth += 1,
            '»' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

#[test]
#[ignore = "reads a copy of a real base"]
fn writing_every_page_again_asks_nothing() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY is not set: nothing to measure");
        return;
    };
    let version: i64 = base.conn().query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
    let engine = classify::ask_engine(&base.conn()).unwrap().map(|c| crate::source_volume::asking::engine_id(&c)).unwrap_or_default();
    println!("== the copy ==\n   schema after opening: {version}; engine of the base: {}", if engine.is_empty() { "none" } else { "set" });
    let offline = Offline { engine: engine.clone(), identities: false, asked: AtomicUsize::new(0), stand_in: AtomicUsize::new(0) };
    let asked = || offline.asked.load(Ordering::SeqCst);

    // The background work, step by step, as `worker.rs` runs it.
    let started = std::time::Instant::now();
    let types = base.types_run(&offline, 10_000).unwrap();
    println!("\n== the steps ==\n   types: asked {} data asked {} left {} (questions so far {})", types.asked, types.data_asked, types.left, asked());
    let reading = base.knowledge_run(&offline, 10_000).unwrap();
    println!("   reading: asked {} left {} rechecked {} (questions so far {})", reading.asked, reading.left, reading.rechecked, asked());
    let resolve = base.knowledge_resolve(Some(&offline), 10_000).unwrap();
    println!("   identities: skipped {} asked {} left {} changed {} (questions so far {})", resolve.skipped, resolve.asked, resolve.left, resolve.changed, asked());
    if resolve.left > 0 {
        // The questions about identities this copy still has (not of the
        // pages): answered "no decision" by a stand-in, never by the model.
        let stand_in = Offline { engine: engine.clone(), identities: true, asked: AtomicUsize::new(0), stand_in: AtomicUsize::new(0) };
        let again = base.knowledge_resolve(Some(&stand_in), 10_000).unwrap();
        println!(
            "   identities with a stand-in: asked {} left {} changed {}; answered by the stand-in {}, other questions {}",
            again.asked,
            again.left,
            again.changed,
            stand_in.stand_in.load(Ordering::SeqCst),
            stand_in.asked.load(Ordering::SeqCst)
        );
    }
    let matters = base.knowledge_propose_matters(Some(&offline), 10_000).unwrap();
    println!("   matters: asked {} left {} (questions so far {})", matters.asked, matters.left, asked());
    // The questions of what the documents say that no answer kept covers:
    // the same ones before and after a change of the pages alone.
    let pending = base.synthesis_questions(&engine).unwrap();
    let mut keys: Vec<&str> = pending.iter().map(|q| q.key.as_str()).collect();
    keys.sort_unstable();
    println!("   questions of the syntheses without a kept answer: {} (their keys: {})", pending.len(), hash(&keys));
    let before_pages = asked();
    let report = base.knowledge_project(Some(&offline), 10_000).unwrap();
    println!(
        "   pages: {:.1}s; skipped {} written {} unchanged {} documents {} asked {} left {} failed {} sentences {} discarded {}",
        started.elapsed().as_secs_f32(),
        report.skipped,
        report.written,
        report.unchanged,
        report.blocks_written,
        report.asked,
        report.left,
        report.failed,
        report.sentences,
        report.discarded
    );
    println!("   questions to the model while writing the pages: {}", asked() - before_pages);
    let again = base.knowledge_project(Some(&offline), 10_000).unwrap();
    println!("   again: skipped {} written {} documents {} asked {}", again.skipped, again.written, again.blocks_written, again.asked);
    println!("   questions to the model in all: {}", asked());

    // Once those syntheses have an answer kept (a stand-in's, in this
    // scratch copy only), every page written again from scratch asks
    // nothing: what a new version of the pages does.
    let stand_in = Offline { engine: engine.clone(), identities: true, asked: AtomicUsize::new(0), stand_in: AtomicUsize::new(0) };
    let answered = base.knowledge_project(Some(&stand_in), 10_000).unwrap();
    println!("   with the syntheses answered by a stand-in: answered {} asked {} failed {}", stand_in.stand_in.load(Ordering::SeqCst), answered.asked, answered.failed);
    let asked_before = asked();
    base.conn().execute("DELETE FROM docbase_setting WHERE key = ?1", [BASIS_SETTING]).unwrap();
    base.conn().execute("UPDATE knowledge_page SET fingerprint = ''", []).unwrap();
    let rebuilt = base.knowledge_project(Some(&offline), 10_000).unwrap();
    println!(
        "   every page written again: skipped {} written {} documents {} asked {} left {} failed {}; questions to the model {}",
        rebuilt.skipped,
        rebuilt.written,
        rebuilt.blocks_written,
        rebuilt.asked,
        rebuilt.left,
        rebuilt.failed,
        asked() - asked_before
    );

    // What the pages say now.
    let pages: Vec<(String, String, String)> = {
        let conn = base.conn();
        let mut statement =
            conn.prepare("SELECT entity_id, path, body FROM knowledge_page WHERE state = 'projected' AND body IS NOT NULL ORDER BY entity_id").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut sample: BTreeMap<String, (String, String)> = BTreeMap::new();
    for (id, path, body) in &pages {
        let kind = if id.starts_with("doc-") {
            "document"
        } else if id.starts_with("mat-") {
            "matter"
        } else {
            "entity"
        };
        *counts.entry(kind.to_string()).or_default() += 1;
        // The headings the pages had before D-441, in the words of the
        // vocabulary of then.
        for (label, heading) in [("references", "Referencias"), ("said list", "Lo que dicen, dato por dato"), ("said others", "Otros datos de los documentos")] {
            if has_section(body, heading) {
                *counts.entry(format!("{kind} with «{label}»")).or_default() += 1;
            }
        }
        let noted = body.contains("[^");
        if noted {
            *counts.entry(format!("{kind} with notes")).or_default() += 1;
        }
        // Below its header (which names the files) and outside its quotes and
        // links (which write a file's name).
        let text = body.splitn(3, "---\n").nth(2).unwrap_or(body);
        let text = without_quotes(text);
        let text: String = text.split("[[").map(|part| part.split_once("]]").map(|(_, after)| after).unwrap_or(part)).collect();
        if english_date(&text) {
            *counts.entry(format!("{kind} with an English date outside quotes and links")).or_default() += 1;
        }
        // The phrases about the program the pages had before (P-208).
        for phrase in ["el modelo", "cálculo de parecidos"] {
            if body.contains(phrase) {
                *counts.entry(format!("{kind} with «{phrase}»")).or_default() += 1;
            }
        }
        if kind == "document" {
            let content: i64 = id.trim_start_matches("doc-").parse().unwrap_or_default();
            let file = family::document_title(&base.conn(), content).unwrap_or_default();
            if title_of(body).is_some_and(|t| t == file) {
                *counts.entry("document titled by its file".to_string()).or_default() += 1;
            }
        }
        let key = format!("{kind}-{}", if noted { "noted" } else { "plain" });
        let longer = sample.get(&key).is_none_or(|(_, b)| body.len() > b.len() && body.len() < 12_000);
        if longer {
            sample.insert(key, (path.clone(), body.clone()));
        }
    }
    println!("\n== the pages ==");
    for (what, n) in &counts {
        println!("   {what}: {n}");
    }
    if let Ok(folder) = std::env::var("MICELYA_PAGES_SAMPLE") {
        std::fs::create_dir_all(&folder).unwrap();
        for (key, (path, body)) in &sample {
            let name = format!("{key}-{}", path.rsplit('/').next().unwrap_or("page.md"));
            std::fs::write(std::path::Path::new(&folder).join(name), body).unwrap();
        }
        println!("   {} pages copied to the sample folder", sample.len());
    }
    crate::docbase::knowledge::check::save_copy(&base);
}
