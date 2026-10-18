//! Runs the resolution of identities over real documents, with the real
//! model, and measures it against the entity answer sheet (R-17): false
//! joins and false separations apart, the sheet's entities split in
//! several and the entities that mix several, and the pairs the sheet says
//! are distinct, doubtful or cannot be resolved.
//!
//! Not part of the normal test run. Same variables as `check.rs`:
//!
//! ```text
//! MICELYA_DOCBASE_COPY=… MICELYA_ENTITY_ANSWERS=… MICELYA_ASK_MODEL=… MICELYA_ASK_CACHE=… MICELYA_ASK_PARALLEL=3 \
//!     cargo test --lib docbase::knowledge::check_identity -- --ignored --nocapture --test-threads=1
//! ```
//!
//! A mention of the sheet is matched to the mentions read in its document
//! as the extraction is measured (same type, the same name or the words
//! of one inside the other); a mention read that matches two entities of
//! the sheet is left out. Pairs are counted between the mentions matched.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::Ordering;
use std::time::Instant;

use super::check::{ask_in_parallel, limit, names_match, normalized, open_base, read_all, recorded, save_copy, sheet};
use crate::docbase::similar::check_types::{key, paths_by_content};
use crate::docbase::Visible;
use crate::source_volume::asking::DocumentAsker;

fn ratio(a: usize, b: usize) -> String {
    if b == 0 {
        "-".to_string()
    } else {
        format!("{:.1} %", 100.0 * a as f64 / b as f64)
    }
}

#[derive(Debug, Clone)]
struct Read {
    id: i64,
    content: i64,
    kind: String,
    normalized: String,
    written: String,
    entity: Option<String>,
    by: Option<String>,
}

#[test]
#[ignore = "reads a real base or real folders and asks the real model"]
fn identity_against_the_entity_sheet() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY or MICELYA_DOCBASE_CHECK is not set: nothing to check");
        return;
    };
    let asker = recorded();
    read_all(&base, &asker);

    // The resolution: the questions warmed in the cache a few at a time,
    // then the run, which asks one after the other what changed with the
    // answers before.
    let started = Instant::now();
    let before = asker.asked.load(Ordering::SeqCst);
    let engine = asker.engine_id();
    let questions = base.identity_questions(&engine).unwrap();
    let total_questions = questions.len();
    if let Ok(path) = std::env::var("MICELYA_IDENTITY_DUMP") {
        let text: Vec<String> = questions.iter().map(|q| format!("==== {}
{}", q.key, q.prompt)).collect();
        let _ = std::fs::write(&path, text.join("

"));
    }
    let failures = ask_in_parallel(&asker, questions, "identity");
    let report = base.knowledge_resolve(Some(&asker), limit().unwrap_or(10_000)).unwrap();
    println!(
        "identity: {:.1}s; questions in the first plan: {total_questions}; questions to the model in this run: {}; failures: {}",
        started.elapsed().as_secs_f32(),
        asker.asked.load(Ordering::SeqCst) - before,
        failures.len()
    );
    println!("   {report:?}");
    let identity_answers: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM model_answer WHERE task = ?1", [super::resolve::TASK], |row| row.get(0))
        .unwrap();
    println!("   identity questions kept in the base: {identity_answers}");
    // Resolving again without changes changes nothing (R-13).
    super::mark_dirty(&base.conn(), "resolve", "check").unwrap();
    let again = base.knowledge_resolve(Some(&asker), 0).unwrap();
    println!("   resolving again: {} entities changed, {} questions asked, {} left", again.changed, again.asked, again.left);
    save_copy(&base);

    let (by_path, by_content) = paths_by_content(&base);
    let name = |content: i64| by_content.get(&content).map(|p| p.join(" = ")).unwrap_or_else(|| format!("#{content}"));
    let read: Vec<Read> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT id, content_id, kind, normalized, written, entity_id, resolved_by FROM knowledge_mention ORDER BY id")
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok(Read {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    kind: row.get(2)?,
                    normalized: row.get(3)?,
                    written: row.get(4)?,
                    entity: row.get(5)?,
                    by: row.get(6)?,
                })
            })
            .unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    let entity_names: HashMap<String, String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT id, name FROM knowledge_entity").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };

    println!("\n== the record ==");
    {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT kind, state, count(*) FROM knowledge_entity GROUP BY kind, state ORDER BY kind, state").unwrap();
        let rows: Vec<(String, String, i64)> =
            statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap().collect::<Result<_, _>>().unwrap();
        for (kind, state, n) in rows {
            println!("   entities {kind} {state}: {n}");
        }
        let mut statement = conn.prepare("SELECT coalesce(resolved_by, '-'), count(*) FROM knowledge_mention GROUP BY 1 ORDER BY 1").unwrap();
        let rows: Vec<(String, i64)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        println!("   mentions by what resolved them: {}", rows.iter().map(|(b, n)| format!("{b} {n}")).collect::<Vec<_>>().join(", "));
        let mut statement = conn.prepare("SELECT reason, count(*) FROM knowledge_doubt GROUP BY 1 ORDER BY 1").unwrap();
        let rows: Vec<(String, i64)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        println!("   doubts by reason: {}", rows.iter().map(|(b, n)| format!("{b} {n}")).collect::<Vec<_>>().join(", "));
        let mut statement = conn.prepare("SELECT state, count(*) FROM knowledge_claim GROUP BY 1 ORDER BY 1").unwrap();
        let rows: Vec<(String, i64)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        println!("   claims by state: {}", rows.iter().map(|(b, n)| format!("{b} {n}")).collect::<Vec<_>>().join(", "));
        let several: i64 = conn
            .query_row("SELECT count(*) FROM (SELECT claim_id FROM knowledge_evidence WHERE claim_id IS NOT NULL GROUP BY claim_id HAVING count(*) > 1)", [], |row| row.get(0))
            .unwrap();
        println!("   claims with more than one piece of evidence: {several}");
        let mut statement = conn.prepare("SELECT verdict, count(*) FROM knowledge_verdict GROUP BY 1 ORDER BY 1").unwrap();
        let rows: Vec<(String, i64)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        println!("   verdicts of the model: {}", rows.iter().map(|(b, n)| format!("{b} {n}")).collect::<Vec<_>>().join(", "));
    }

    let Some(sheet) = sheet() else {
        println!("\nMICELYA_ENTITY_ANSWERS is not set or not readable: no comparison with the answer sheet");
        return;
    };
    let content_of = |path: &str| -> Option<i64> { by_path.get(&key(&format!("{}/{}", sheet.root, path))).copied() };
    let by_doc: BTreeMap<i64, Vec<&Read>> = {
        let mut out: BTreeMap<i64, Vec<&Read>> = BTreeMap::new();
        for r in &read {
            out.entry(r.content).or_default().push(r);
        }
        out
    };
    // Each mention read, to the entity of the sheet it is (when only one).
    let mut matched: BTreeMap<i64, BTreeSet<&str>> = BTreeMap::new();
    let sheet_kind: HashMap<&str, &str> = sheet.entities.iter().map(|e| (e.id.as_str(), e.kind.as_str())).collect();
    for entity in &sheet.entities {
        let mut names: Vec<String> = vec![normalized(&entity.name)];
        names.extend(entity.aliases.iter().map(|a| normalized(a)));
        for mention in &entity.mentions {
            let Some(content) = content_of(&mention.path) else { continue };
            let written = normalized(&mention.as_written);
            for r in by_doc.get(&content).into_iter().flatten() {
                if r.kind == entity.kind
                    && (names_match(&written, &r.normalized) || names.iter().any(|n| names_match(n, &r.normalized)))
                {
                    matched.entry(r.id).or_default().insert(entity.id.as_str());
                }
            }
        }
    }
    let ambiguous = matched.values().filter(|s| s.len() > 1).count();
    let mapped: Vec<(&Read, &str)> = read
        .iter()
        .filter_map(|r| matched.get(&r.id).filter(|s| s.len() == 1).map(|s| (r, *s.iter().next().unwrap())))
        .collect();
    println!(
        "\n== identity against the sheet ==\n   mentions read matched to one entity of the sheet: {} (of {} read; {ambiguous} match more than one and are left out)",
        mapped.len(),
        read.len()
    );
    let unresolved = mapped.iter().filter(|(r, _)| r.entity.is_none()).count();
    println!("   of those, without an entity: {unresolved}");

    // Pairs.
    let (mut same_same, mut false_split, mut false_join, mut apart) = (0usize, 0usize, 0usize, 0usize);
    let (mut x_same_same, mut x_false_split, mut x_false_join) = (0usize, 0usize, 0usize);
    let mut join_examples: BTreeMap<(String, String), usize> = BTreeMap::new();
    for i in 0..mapped.len() {
        for j in i + 1..mapped.len() {
            let ((a, ea), (b, eb)) = (mapped[i], mapped[j]);
            if a.kind != b.kind {
                continue;
            }
            let ours = a.entity.is_some() && a.entity == b.entity;
            let theirs = ea == eb;
            let cross = a.content != b.content;
            match (theirs, ours) {
                (true, true) => {
                    same_same += 1;
                    if cross {
                        x_same_same += 1;
                    }
                }
                (true, false) => {
                    false_split += 1;
                    if cross {
                        x_false_split += 1;
                    }
                }
                (false, true) => {
                    false_join += 1;
                    if cross {
                        x_false_join += 1;
                    }
                    let (p, q) = if ea <= eb { (ea, eb) } else { (eb, ea) };
                    *join_examples.entry((p.to_string(), q.to_string())).or_default() += 1;
                }
                (false, false) => apart += 1,
            }
        }
    }
    let should_join = same_same + false_split;
    println!("   pairs of mentions of the same entity of the sheet: {should_join}; joined: {same_same} ({}); false separations: {false_split} ({})", ratio(same_same, should_join), ratio(false_split, should_join));
    println!("   pairs of mentions of different entities: {}; false joins: {false_join} ({})", false_join + apart, ratio(false_join, false_join + apart));
    println!(
        "   only pairs across documents: joined {x_same_same} of {} ({}); false separations {x_false_split}; false joins {x_false_join}",
        x_same_same + x_false_split,
        ratio(x_same_same, x_same_same + x_false_split)
    );
    println!("   precision of the joins (pairs): {}", ratio(same_same, same_same + false_join));
    println!("\n   entities of the sheet mixed in one of ours (pairs of mentions joined):");
    for ((p, q), n) in &join_examples {
        let names: BTreeSet<&str> = mapped
            .iter()
            .filter(|(_, e)| e == p || e == q)
            .filter_map(|(r, _)| r.entity.as_deref())
            .filter(|id| mapped.iter().any(|(r2, e2)| r2.entity.as_deref() == Some(id) && e2 == p) && mapped.iter().any(|(r2, e2)| r2.entity.as_deref() == Some(id) && e2 == q))
            .map(|id| entity_names.get(id).map(String::as_str).unwrap_or(id))
            .collect();
        println!("      {p} + {q}: {n} pairs, in «{}»", names.into_iter().collect::<Vec<_>>().join("», «"));
    }

    // Entities split and entities mixed.
    let mut pieces: BTreeMap<&str, BTreeMap<String, usize>> = BTreeMap::new();
    let mut mixes: BTreeMap<String, BTreeMap<&str, usize>> = BTreeMap::new();
    for (r, e) in &mapped {
        let ours = r.entity.clone().unwrap_or_else(|| format!("(sin entidad) {}", r.id));
        *pieces.entry(e).or_default().entry(ours.clone()).or_default() += 1;
        *mixes.entry(ours).or_default().entry(e).or_default() += 1;
    }
    let split: Vec<(&&str, &BTreeMap<String, usize>)> = pieces.iter().filter(|(_, p)| p.len() > 1).collect();
    let mixed: Vec<(&String, &BTreeMap<&str, usize>)> = mixes.iter().filter(|(_, m)| m.len() > 1).collect();
    let by_kind = |kind: &str| -> (usize, usize) {
        let all = pieces.keys().filter(|e| sheet_kind.get(*e) == Some(&kind)).count();
        let split = split.iter().filter(|(e, _)| sheet_kind.get(**e) == Some(&kind)).count();
        (split, all)
    };
    println!("\n   entities of the sheet matched: {}; split in more than one of ours: {}", pieces.len(), split.len());
    for kind in ["persona", "organizacion", "asunto"] {
        let (s, a) = by_kind(kind);
        println!("      {kind}: {s} of {a} split");
    }
    println!("   entities of ours that mix more than one of the sheet: {}", mixed.len());
    for (ours, list) in &mixed {
        println!(
            "      «{}»: {}",
            entity_names.get(*ours).map(String::as_str).unwrap_or(ours),
            list.iter().map(|(e, n)| format!("{e} ({n})")).collect::<Vec<_>>().join(", ")
        );
    }
    println!("\n   entities of the sheet split (pieces: mentions):");
    for (e, list) in &split {
        let shown: Vec<String> = list
            .iter()
            .map(|(id, n)| format!("«{}» {n}", entity_names.get(id).map(String::as_str).unwrap_or(id)))
            .collect();
        println!("      {e}: {}", shown.join(", "));
    }

    // The sheet's distinct pairs, doubtful pairs and unresolvable mentions.
    let ours_of = |e: &str| -> BTreeSet<String> { mapped.iter().filter(|(_, x)| *x == e).filter_map(|(r, _)| r.entity.clone()).collect() };
    let mut broken = Vec::new();
    for pair in &sheet.distinct {
        if !ours_of(&pair.a).is_disjoint(&ours_of(&pair.b)) {
            broken.push(format!("{} + {}", pair.a, pair.b));
        }
    }
    println!("\n   distinct pairs of the sheet joined: {} of {} {}", broken.len(), sheet.distinct.len(), broken.join("; "));
    let doubts: Vec<(Option<String>, Option<String>, String)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT entity_a, entity_b, reason FROM knowledge_doubt").unwrap();
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    for pair in &sheet.doubtful {
        let (a, b) = (ours_of(&pair.a), ours_of(&pair.b));
        let joined = !a.is_disjoint(&b);
        let doubted = doubts.iter().find(|(x, y, _)| {
            let (x, y) = (x.clone().unwrap_or_default(), y.clone().unwrap_or_default());
            (a.contains(&x) && b.contains(&y)) || (a.contains(&y) && b.contains(&x))
        });
        println!(
            "   doubtful pair {} + {}: {}",
            pair.a,
            pair.b,
            if joined {
                "joined (wrong)".to_string()
            } else if let Some((_, _, reason)) = doubted {
                format!("left in doubt ({reason})")
            } else {
                "kept apart, with no doubt raised".to_string()
            }
        );
    }
    let mut wrongly = 0;
    for item in &sheet.unresolvable {
        let Some(content) = content_of(&item.path) else { continue };
        let written = normalized(&item.as_written);
        for r in by_doc.get(&content).into_iter().flatten() {
            if r.normalized == written || names_match(&written, &r.normalized) {
                let entity = r.entity.clone();
                let with: BTreeSet<&str> =
                    mapped.iter().filter(|(m, _)| entity.is_some() && m.entity == entity && m.id != r.id).map(|(_, e)| *e).collect();
                if !with.is_empty() {
                    wrongly += 1;
                    println!("   unresolvable «{}» in {} joined to {}", item.as_written, name(content), with.into_iter().collect::<Vec<_>>().join(", "));
                }
            }
        }
    }
    println!("   unresolvable mentions joined to an entity of the sheet: {wrongly} (of {} listed)", sheet.unresolvable.len());

    println!("\n== our entities with more than one document ==");
    let mut docs: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
    for r in &read {
        if let Some(e) = &r.entity {
            docs.entry(e.clone()).or_default().insert(r.content);
        }
    }
    let mut list: Vec<(&String, &BTreeSet<i64>)> = docs.iter().filter(|(_, d)| d.len() > 1).collect();
    list.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(b.0)));
    for (id, d) in list {
        let written: BTreeSet<&str> = read.iter().filter(|r| r.entity.as_ref() == Some(id)).map(|r| r.written.as_str()).collect();
        let by: BTreeSet<&str> = read.iter().filter(|r| r.entity.as_ref() == Some(id)).filter_map(|r| r.by.as_deref()).collect();
        println!(
            "   «{}» {} documents [{}]: {}",
            entity_names.get(id).map(String::as_str).unwrap_or(id),
            d.len(),
            by.into_iter().collect::<Vec<_>>().join(","),
            written.into_iter().take(8).collect::<Vec<_>>().join(" | ")
        );
    }
    println!("\n== doubts ==");
    {
        let conn = base.conn();
        let mut statement = conn
            .prepare(
                "SELECT d.reason, coalesce(ea.name, d.a), coalesce(eb.name, d.b), coalesce(d.detail, '') FROM knowledge_doubt d
                 LEFT JOIN knowledge_entity ea ON ea.id = d.entity_a LEFT JOIN knowledge_entity eb ON eb.id = d.entity_b ORDER BY 1, 2",
            )
            .unwrap();
        let rows: Vec<(String, String, String, String)> =
            statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap().collect::<Result<_, _>>().unwrap();
        for (reason, a, b, detail) in rows {
            println!("   [{reason}] «{a}» / «{b}» {detail}");
        }
    }
    let _ = Visible::All;
}
