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
    let proposed = base.knowledge_propose_matters(Some(&asker), 10_000).unwrap();
    println!("matters proposed by the model: {proposed:?}");

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
                "SELECT p.entity_id, p.path, coalesce(e.kind, 'asunto (grupo)'),
                        coalesce((SELECT count(DISTINCT m.content_id) FROM knowledge_mention m WHERE m.entity_id = e.id),
                                 (SELECT count(*) FROM knowledge_matter_document d WHERE d.matter_id = p.entity_id))
                 FROM knowledge_page p LEFT JOIN knowledge_entity e ON e.id = p.entity_id WHERE p.state = 'projected'",
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
    let mut main_of: BTreeMap<String, String> = BTreeMap::new();
    for entity in sheet.entities.iter().filter(|e| e.kind != "asunto") {
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
        if let Some(main) = &main {
            main_of.insert(entity.id.clone(), main.clone());
        }
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

    matters_against_the_sheet(&base, &sheet, &content_of, &pages);
    relations_against_the_sheet(&base, &sheet, &main_of);

    if let Ok(sample) = std::env::var("MICELYA_PAGES_SAMPLE") {
        let sample = std::path::PathBuf::from(sample);
        std::fs::create_dir_all(&sample).unwrap();
        let mut chosen: Vec<(&String, &(String, String, usize))> = pages.iter().collect();
        chosen.sort_by(|a, b| b.1 .2.cmp(&a.1 .2).then_with(|| a.0.cmp(b.0)));
        // The three named in most documents, a matter and a person further down.
        let mut picked: Vec<&String> = chosen.iter().take(3).map(|(id, _)| *id).collect();
        if let Some((id, _)) = chosen.iter().find(|(id, p)| p.1.starts_with("asunto") && !picked.contains(id)) {
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
        // All the pages the app wrote, to measure how they read.
        if std::env::var("MICELYA_PAGES_COPY_ALL").is_ok() {
            let all = sample.join("entidades");
            let copied = copy_dir(&project.path().join("wiki/entities"), &all);
            let _ = std::fs::copy(project.path().join("wiki/overview.md"), all.join("overview.md"));
            println!("   all the pages of entities ({copied}): {}", all.display());
        }
    }
    changes(&base, &asker, project.path());
}

/// The matters (groups of documents) against the matters of the sheet:
/// pairs of documents of one matter of the sheet grouped by the layer
/// (found), and pairs it grouped that the sheet has in different matters
/// or in none (false groupings, the worse error: R-07).
fn matters_against_the_sheet(
    base: &crate::docbase::DocBase,
    sheet: &super::check::Sheet,
    content_of: &dyn Fn(&str) -> Option<i64>,
    pages: &BTreeMap<String, (String, String, usize)>,
) {
    let mut sheet_of: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    for matter in &sheet.matters {
        for path in &matter.documents {
            if let Some(content) = content_of(path) {
                sheet_of.entry(content).or_default().insert(matter.id.clone());
            }
        }
    }
    let ours: Vec<(String, i64)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT d.matter_id, d.content_id FROM knowledge_matter_document d JOIN knowledge_matter m ON m.id = d.matter_id WHERE m.state = 'active'")
            .unwrap();
        let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let mut our_of: BTreeMap<i64, String> = BTreeMap::new();
    let mut groups: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
    for (matter, content) in &ours {
        our_of.insert(*content, matter.clone());
        groups.entry(matter.clone()).or_default().insert(*content);
    }
    let documents: Vec<i64> = sheet_of.keys().chain(our_of.keys()).copied().collect::<BTreeSet<_>>().into_iter().collect();
    let (mut in_sheet, mut grouped, mut found) = (0usize, 0usize, 0usize);
    for (i, a) in documents.iter().enumerate() {
        for b in &documents[i + 1..] {
            let same_sheet = sheet_of.get(a).zip(sheet_of.get(b)).is_some_and(|(x, y)| !x.is_disjoint(y));
            let same_ours = our_of.get(a).is_some() && our_of.get(a) == our_of.get(b);
            in_sheet += same_sheet as usize;
            grouped += same_ours as usize;
            found += (same_sheet && same_ours) as usize;
        }
    }
    println!("\n== matters ==");
    println!(
        "   matters (groups of documents): {}, with {} documents; with a page: {}",
        groups.len(),
        our_of.len(),
        groups.keys().filter(|g| pages.contains_key(*g)).count()
    );
    println!("   pairs of documents of one matter of the sheet: {in_sheet}; grouped: {found} ({})", ratio(found, in_sheet));
    println!("   pairs grouped: {grouped}; false groupings (not one matter in the sheet): {} ({})", grouped - found, ratio(grouped - found, grouped));
    // What the model proposed for documents no evidence put in a matter:
    // right when the sheet puts the document in the matter most of that
    // group's documents are in.
    let proposals = super::matters::proposals(&base.conn()).unwrap();
    let (mut right, mut wrong) = (0usize, 0usize);
    for (content, list) in &proposals {
        for (matter, _) in list {
            let mut counts: BTreeMap<&String, usize> = BTreeMap::new();
            for doc in groups.get(matter).into_iter().flatten() {
                for id in sheet_of.get(doc).into_iter().flatten() {
                    *counts.entry(id).or_default() += 1;
                }
            }
            let majority = counts.iter().max_by_key(|(_, n)| **n).map(|(id, _)| (*id).clone());
            if majority.is_some_and(|m| sheet_of.get(content).is_some_and(|s| s.contains(&m))) {
                right += 1;
            } else {
                wrong += 1;
            }
        }
    }
    println!("   documents the model proposed for a matter: {}; right by the sheet: {right}; wrong: {wrong}", right + wrong);
    for matter in &sheet.matters {
        let docs: BTreeSet<i64> = matter.documents.iter().filter_map(|p| content_of(p)).collect();
        let mut best: BTreeMap<&String, usize> = BTreeMap::new();
        for content in &docs {
            if let Some(ours) = our_of.get(content) {
                *best.entry(ours).or_default() += 1;
            }
        }
        let top = best.iter().max_by_key(|(_, n)| **n).map(|(id, n)| (*id, *n));
        println!(
            "      {} ({} documents): {}",
            matter.id,
            docs.len(),
            match top {
                Some((id, n)) => format!("{n} in one of ours ({} documents{})", groups[id].len(), if pages.contains_key(id) { ", with a page" } else { "" }),
                None => "in none of ours".to_string(),
            }
        );
    }
}

/// The relations between entities against the sheet's (stage 5): for each
/// claim of the sheet between two of its entities with a predicate of the
/// vocabulary, whether the record has a claim of that predicate between
/// the entities each one went to (either way: the sheet says "represented
/// by" where the vocabulary says "represents").
fn relations_against_the_sheet(base: &crate::docbase::DocBase, sheet: &super::check::Sheet, main_of: &BTreeMap<String, String>) {
    let ours: BTreeSet<(String, String, String)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT subject, predicate, object_entity FROM knowledge_claim WHERE object_entity IS NOT NULL AND state != 'retired'")
            .unwrap();
        let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let entity_named = |object: &serde_json::Value| -> Option<&super::check::SheetEntity> {
        let text = object.as_str()?;
        // The sheet names the object by its id, before what it adds ("E04 (…)").
        let first = text.split_whitespace().next().unwrap_or_default().split('/').next().unwrap_or_default();
        if let Some(found) = sheet.entities.iter().find(|e| e.id == first) {
            return Some(found);
        }
        let wanted = normalized(text);
        sheet.entities.iter().find(|e| {
            e.kind != "asunto" && (names_match(&normalized(&e.name), &wanted) || e.aliases.iter().any(|a| names_match(&normalized(a), &wanted)))
        })
    };
    let (mut total, mut found, mut unresolved) = (0usize, 0usize, 0usize);
    let mut by_predicate: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for entity in &sheet.entities {
        for claim in &entity.claims {
            let Some(predicates) = super::check::vocabulary_predicates(&claim.predicate) else { continue };
            let Some(object) = entity_named(&claim.object) else { continue };
            if object.kind == "asunto" || !predicates.iter().any(|p| super::vocabulary::vocabulary().predicate(p).is_some_and(|d| d.takes_entity())) {
                continue;
            }
            total += 1;
            let entry = by_predicate.entry(claim.predicate.as_str()).or_default();
            entry.0 += 1;
            let (Some(s), Some(o)) = (main_of.get(&entity.id), main_of.get(&object.id)) else {
                unresolved += 1;
                continue;
            };
            let hit = predicates.iter().any(|p| {
                ours.contains(&(s.clone(), p.to_string(), o.clone())) || ours.contains(&(o.clone(), p.to_string(), s.clone()))
            });
            if hit {
                found += 1;
                entry.1 += 1;
            }
        }
    }
    println!("\n== relations ==");
    println!(
        "   relations of the sheet between two of its entities: {total}; in the record between the same entities: {found} ({}); with an entity not found: {unresolved}",
        ratio(found, total)
    );
    for (predicate, (want, hit)) in by_predicate {
        println!("      {predicate}: {hit} of {want}");
    }
    println!("   relations in the record between two entities: {}", ours.len());
}

/// The pages on disk, without the date they were written.
fn page_texts(root: &std::path::Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for folder in ["wiki/entities", "wiki/sources"] {
        for entry in std::fs::read_dir(root.join(folder)).into_iter().flatten().flatten() {
            let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
            let text: Vec<&str> = text.lines().filter(|l| !l.starts_with("updated:")).collect();
            out.insert(format!("{folder}/{}", entry.file_name().to_string_lossy()), text.join("
"));
        }
    }
    out
}

fn entity_states(base: &crate::docbase::DocBase) -> BTreeMap<String, String> {
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT id, state FROM knowledge_entity").unwrap();
    let rows = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

/// Changes (stage 4): the document that names most entities is left out
/// and taken back, then deleted. Leaving it out must retire only what it
/// alone said and write only the pages it touches; taking it back must
/// leave everything as it was, asking nothing.
fn changes(base: &crate::docbase::DocBase, asker: &super::check::Recorded, root: &std::path::Path) {
    use std::sync::atomic::Ordering;
    let (content, named): (i64, i64) = base
        .conn()
        .query_row(
            "SELECT content_id, count(DISTINCT entity_id) AS n FROM knowledge_mention WHERE entity_id IS NOT NULL
             GROUP BY content_id ORDER BY n DESC, content_id LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    let only_there: i64 = base
        .conn()
        .query_row(
            "SELECT count(DISTINCT entity_id) FROM knowledge_mention m WHERE m.content_id = ?1 AND m.entity_id IS NOT NULL
             AND NOT EXISTS (SELECT 1 FROM knowledge_mention o WHERE o.entity_id = m.entity_id AND o.content_id != ?1)",
            [content],
            |r| r.get(0),
        )
        .unwrap();
    let locations: Vec<(String, String)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT l.origin_id, l.rel_path FROM location l JOIN file f ON f.hash = l.file_hash WHERE f.content_id = ?1")
            .unwrap();
        let rows = statement.query_map([content], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let states = entity_states(base);
    let pages = page_texts(root);
    let asked = asker.asked.load(Ordering::SeqCst);
    println!(
        "
== changes ==
   the document that names most entities: {named} entities, {only_there} named only there; {} locations",
        locations.len()
    );
    for (origin, path) in &locations {
        base.set_excluded(origin, path, false, true).unwrap();
    }
    let resolved = base.knowledge_resolve(Some(asker), 30).unwrap();
    let projected = base.knowledge_project(Some(asker), 30).unwrap();
    let after = entity_states(base);
    let retired = after.iter().filter(|(id, s)| *s == "retired" && states.get(*id).is_some_and(|b| b != "retired")).count();
    let texts = page_texts(root);
    let changed = texts.iter().filter(|(p, t)| pages.get(*p) != Some(*t)).count() + pages.keys().filter(|p| !texts.contains_key(*p)).count();
    println!(
        "   left out: {retired} entities retired (should be {only_there}); entities changed {}; pages written {}, removed {}, kept {}, blocks {}; files that changed: {changed}; questions left: {}",
        resolved.changed, projected.written, projected.removed, projected.kept, projected.blocks_written, resolved.left + projected.left
    );
    let asked_left_out = asker.asked.load(Ordering::SeqCst) - asked;
    println!("   questions to the model while it was left out: {asked_left_out}");
    let asked = asker.asked.load(Ordering::SeqCst);
    for (origin, path) in &locations {
        base.set_excluded(origin, path, false, false).unwrap();
    }
    let resolved = base.knowledge_resolve(Some(asker), 30).unwrap();
    let projected = base.knowledge_project(Some(asker), 30).unwrap();
    let back = entity_states(base);
    let texts = page_texts(root);
    let differ = texts.iter().filter(|(p, t)| pages.get(*p) != Some(*t)).count() + pages.keys().filter(|p| !texts.contains_key(*p)).count();
    let states_differ = back.iter().filter(|(id, s)| states.get(*id) != Some(*s)).count();
    println!(
        "   taken back: entities changed {}, pages written {}; entities whose state differs from before: {states_differ}; files that differ from before: {differ}; questions to the model: {} (should be 0); left {}",
        resolved.changed,
        projected.written,
        asker.asked.load(Ordering::SeqCst) - asked,
        resolved.left + projected.left
    );
}
