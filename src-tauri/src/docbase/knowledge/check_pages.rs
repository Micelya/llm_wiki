//! Runs the projection of the record into a wiki over real documents,
//! with the real model, and measures it against the entity answer sheet
//! (R-17): which entities of the sheet have a page, how many of their
//! documents the page lists, and how many sentences of the syntheses the
//! check kept or threw away.
//!
//! Not part of the normal test run. Same variables as `check.rs`, plus:
//!
//! - `MICELYA_PAGES_WIKI`: a folder with pages of documents written by the
//!   wiki before (`wiki/sources/*.md`), copied into the scratch project so
//!   the pages link to them and get their block. Only read.
//! - `MICELYA_PAGES_SAMPLE`: a folder (outside the repository: the pages
//!   name real people) where the pages of the entities named in most
//!   documents, and the overview, are copied after the run.
//!
//! ```text
//! MICELYA_DOCBASE_COPY=… MICELYA_ENTITY_ANSWERS=… MICELYA_ASK_MODEL=… MICELYA_ASK_CACHE=… MICELYA_ASK_PARALLEL=3 \
//!     cargo test --lib docbase::knowledge::check_pages -- --ignored --nocapture --test-threads=1
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use super::check::{ask_in_parallel, limit, names_match, normalized, open_base, read_all, recorded, save_copy, sheet};
use crate::docbase::similar::check_types::{key, paths_by_content};
use crate::docbase::testing::TempDir;
use crate::source_volume::asking::DocumentAsker;

fn ratio(a: usize, b: usize) -> String {
    if b == 0 {
        "-".to_string()
    } else {
        format!("{:.1} %", 100.0 * a as f64 / b as f64)
    }
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> usize {
    let mut copied = 0;
    let Ok(entries) = std::fs::read_dir(from) else { return 0 };
    std::fs::create_dir_all(to).unwrap();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") && std::fs::copy(&path, to.join(entry.file_name())).is_ok() {
            copied += 1;
        }
    }
    copied
}

#[test]
#[ignore = "reads a real base or real folders and asks the real model"]
fn pages_against_the_entity_sheet() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY or MICELYA_DOCBASE_CHECK is not set: nothing to check");
        return;
    };
    let asker = recorded();
    read_all(&base, &asker);
    let questions = base.identity_questions(&asker.engine_id()).unwrap();
    ask_in_parallel(&asker, questions, "identity");
    let resolved = base.knowledge_resolve(Some(&asker), 10_000).unwrap();
    println!("identity: {resolved:?}");

    // A scratch wiki project as the general project of the copy.
    let project = TempDir::new();
    project.folder("wiki/sources");
    project.write("schema.md", b"# schema");
    if let Ok(wiki) = std::env::var("MICELYA_PAGES_WIKI") {
        let copied = copy_dir(&std::path::Path::new(&wiki).join("wiki/sources"), &project.path().join("wiki/sources"));
        println!("pages of documents copied from {wiki}: {copied}");
    }
    base.link_general_project(project.path(), "default").unwrap();
    base.set_wiki_language(Some("Spanish")).unwrap();

    let started = Instant::now();
    let engine = asker.engine_id();
    let questions = base.synthesis_questions(&engine).unwrap();
    let total_questions = questions.len();
    let failures = ask_in_parallel(&asker, questions, "synthesis");
    let report = base.knowledge_project(Some(&asker), limit().unwrap_or(10_000)).unwrap();
    println!(
        "projection: {:.1}s; synthesis questions: {total_questions}; failures: {}\n   {report:?}",
        started.elapsed().as_secs_f32(),
        failures.len()
    );
    // Projecting again changes nothing (R-13).
    super::mark_dirty(&base.conn(), "project", "check").unwrap();
    let again = base.knowledge_project(Some(&asker), 0).unwrap();
    println!("   projecting again: written {}, asked {}, blocks {}, overview {}", again.written, again.asked, again.blocks_written, again.overview_written);
    save_copy(&base);

    // The pages on disk.
    let mut pages: BTreeMap<String, (String, String, usize)> = BTreeMap::new(); // entity -> (path, kind, documents)
    {
        let conn = base.conn();
        let mut statement = conn
            .prepare(
                "SELECT p.entity_id, p.path, e.kind, (SELECT count(DISTINCT m.content_id) FROM knowledge_mention m WHERE m.entity_id = e.id)
                 FROM knowledge_page p JOIN knowledge_entity e ON e.id = p.entity_id WHERE p.state = 'projected'",
            )
            .unwrap();
        let rows: Vec<(String, String, String, i64)> =
            statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))).unwrap().collect::<Result<_, _>>().unwrap();
        for (id, path, kind, docs) in rows {
            pages.insert(id, (path, kind, docs as usize));
        }
    }
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    let mut single = 0;
    for (_, kind, docs) in pages.values() {
        *by_kind.entry(kind.as_str()).or_default() += 1;
        if *docs == 1 {
            single += 1;
        }
    }
    println!("\n== pages ==\n   pages of entities: {} ({by_kind:?}); with a single document: {single}", pages.len());
    let (sentences, discarded): (i64, i64) =
        base.conn().query_row("SELECT coalesce(sum(sentences), 0), coalesce(sum(discarded), 0) FROM knowledge_page", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    println!(
        "   sentences of the syntheses kept: {sentences}; thrown away by the check: {discarded} ({})",
        ratio(discarded as usize, (sentences + discarded) as usize)
    );
    // Every sentence of a synthesis ends with its references.
    let with_synthesis: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT path FROM knowledge_page WHERE synthesis IS NOT NULL AND sentences > 0").unwrap();
        let rows = statement.query_map([], |r| r.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let without_reference = with_synthesis
        .iter()
        .map(|path| std::fs::read_to_string(project.path().join(path)).unwrap_or_default())
        .map(|text| {
            let says = text.split("## Lo que dicen los documentos

").nth(1).unwrap_or("");
            let paragraph = says.split("

").next().unwrap_or("");
            // A sentence ends with "[n]" or "[n, m]" before the next one starts.
            paragraph.split("] ").filter(|piece| !piece.contains('[')).count()
        })
        .sum::<usize>();
    println!("   sentences of the syntheses without a reference (should be 0): {without_reference}");

    let Some(sheet) = sheet() else {
        println!("\nMICELYA_ENTITY_ANSWERS is not set: no comparison with the answer sheet");
        return;
    };
    let (by_path, _) = paths_by_content(&base);
    let content_of = |path: &str| -> Option<i64> { by_path.get(&key(&format!("{}/{}", sheet.root, path))).copied() };
    // Mentions read, by document.
    let read: Vec<(i64, i64, String, String, Option<String>)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT id, content_id, kind, normalized, entity_id FROM knowledge_mention").unwrap();
        let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let documents_of: BTreeMap<String, BTreeSet<i64>> = {
        let mut out: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
        for (_, content, _, _, entity) in &read {
            if let Some(entity) = entity {
                out.entry(entity.clone()).or_default().insert(*content);
            }
        }
        out
    };
    println!("\n== against the sheet ==");
    let (mut with_page, mut docs_total, mut docs_listed, mut entities) = (0usize, 0usize, 0usize, 0usize);
    let mut rows = Vec::new();
    for entity in &sheet.entities {
        let mut names: Vec<String> = vec![normalized(&entity.name)];
        names.extend(entity.aliases.iter().map(|a| normalized(a)));
        let docs: BTreeSet<i64> = entity.mentions.iter().filter_map(|m| content_of(&m.path)).collect();
        if docs.is_empty() {
            continue;
        }
        entities += 1;
        // Our entities its mentions went to, by how many documents.
        let mut ours: BTreeMap<String, usize> = BTreeMap::new();
        for mention in &entity.mentions {
            let Some(content) = content_of(&mention.path) else { continue };
            let written = normalized(&mention.as_written);
            for (_, c, kind, norm, id) in &read {
                if *c == content && *kind == entity.kind && (names_match(&written, norm) || names.iter().any(|n| names_match(n, norm))) {
                    if let Some(id) = id {
                        *ours.entry(id.clone()).or_default() += 1;
                    }
                }
            }
        }
        let main = ours.iter().max_by_key(|(_, n)| **n).map(|(id, _)| id.clone());
        let page = main.as_ref().and_then(|id| pages.get(id));
        let listed = main.as_ref().and_then(|id| documents_of.get(id)).map(|d| d.intersection(&docs).count()).unwrap_or(0);
        docs_total += docs.len();
        if page.is_some() {
            with_page += 1;
            docs_listed += listed;
        }
        rows.push((docs.len(), entity.id.clone(), entity.kind.clone(), page.is_some(), listed, ours.len()));
    }
    println!("   entities of the sheet: {entities}; with a page: {with_page} ({})", ratio(with_page, entities));
    println!(
        "   documents of the sheet's entities listed in their page: {docs_listed} of {docs_total} ({})",
        ratio(docs_listed, docs_total)
    );
    rows.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    println!("   the sheet's entities with most documents (documents of the sheet / listed in the page / our entities it is in):");
    for (docs, id, kind, page, listed, pieces) in rows.iter().take(15) {
        println!("      {id} ({kind}): {docs} / {} / {pieces}", if *page { listed.to_string() } else { "sin página".into() });
    }
    for (docs, id, kind, page, _, _) in rows.iter().filter(|r| !r.3) {
        println!("      without a page: {id} ({kind}, {docs} documents)");
        let _ = page;
    }

    if let Ok(sample) = std::env::var("MICELYA_PAGES_SAMPLE") {
        let sample = std::path::PathBuf::from(sample);
        std::fs::create_dir_all(&sample).unwrap();
        let mut chosen: Vec<(&String, &(String, String, usize))> = pages.iter().collect();
        chosen.sort_by(|a, b| b.1 .2.cmp(&a.1 .2).then_with(|| a.0.cmp(b.0)));
        // The three named in most documents, a matter and a person further down.
        let mut picked: Vec<&String> = chosen.iter().take(3).map(|(id, _)| *id).collect();
        if let Some((id, _)) = chosen.iter().find(|(id, p)| p.1 == "asunto" && !picked.contains(id)) {
            picked.push(id);
        }
        if let Some((id, _)) = chosen.iter().rev().find(|(id, p)| p.1 == "persona" && !picked.contains(id)) {
            picked.push(id);
        }
        for id in picked {
            let path = &pages[id].0;
            let name = path.rsplit('/').next().unwrap();
            let _ = std::fs::copy(project.path().join(path), sample.join(name));
        }
        let _ = std::fs::copy(project.path().join("wiki/overview.md"), sample.join("overview.md"));
        let with_block = std::fs::read_dir(project.path().join("wiki/sources"))
            .into_iter()
            .flatten()
            .flatten()
            .find(|e| std::fs::read_to_string(e.path()).is_ok_and(|t| t.contains(super::pages::BLOCK_START)));
        if let Some(source) = with_block {
            let _ = std::fs::copy(source.path(), sample.join(format!("documento-{}", source.file_name().to_string_lossy())));
        }
        println!("   a sample of the pages: {}", sample.display());
    }
}
