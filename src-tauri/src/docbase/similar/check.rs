//! Runs the similar-documents passes over real folders and measures them
//! against an answer sheet (R-17), and checks that no case of the answer
//! sheet leaked into the program (R-24).
//!
//! Not part of the normal test run. The folders are only read; the
//! database goes to a scratch folder removed at the end.
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\casos" MICELYA_SIMILAR_ANSWERS="C:\hoja\respuestas.json" \
//!     cargo test --lib docbase::similar::check -- --ignored --nocapture --test-threads=1
//! ```
//!
//! The answer sheet is a JSON file outside the repository (it names real
//! files and people): `{"root", "pairs": [{"a", "b", "relation",
//! "direction", "measurable", "note"}], "documents": [...]}`, with paths
//! relative to `root` and the relations in Spanish ("misma-plantilla").

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Deserialize;

use super::marks::{Mark, PdfSignature};
use super::types::Catalog;
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible, PRIVATE_DB_FILE};

#[derive(Debug, Deserialize)]
struct Answers {
    root: String,
    #[serde(default)]
    pairs: Vec<AnswerPair>,
    #[serde(default)]
    documents: Vec<AnswerDocument>,
}

#[derive(Debug, Deserialize)]
struct AnswerPair {
    a: String,
    b: String,
    relation: String,
    /// The pages of the file `a` the pair is about (`pages:3-5`), when
    /// the file holds several documents; the whole file when not said.
    #[serde(default)]
    a_selector: Option<String>,
    #[serde(default)]
    b_selector: Option<String>,
    #[serde(default)]
    measurable: bool,
    #[serde(default)]
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnswerDocument {
    path: String,
}

/// The engine's id for each relation of the answer sheet.
fn engine_relation(answer: &str) -> Option<&'static str> {
    Some(match answer {
        "mismo-documento" => "same-document",
        "practicamente-el-mismo" => "near-identical",
        "ejemplar-firmado" => "signed-copy",
        "vista-previa" => "preview",
        "version" => "version",
        "contenido-en" => "contained-in",
        "misma-plantilla" => "same-template",
        "sin-relacion" => "none",
        _ => return None,
    })
}

fn answers() -> Option<Answers> {
    // Several sheets go separated by `;`: the first one is used here.
    let paths = std::env::var("MICELYA_SIMILAR_ANSWERS").ok()?;
    let path = paths.split(';').next()?.trim().to_string();
    let text = std::fs::read_to_string(&path).map_err(|e| println!("cannot read {path}: {e}")).ok()?;
    serde_json::from_str(&text).map_err(|e| println!("{path} is not a valid answer sheet: {e}")).ok()
}

fn key(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Runs everything over the folders and returns the base.
fn run(folders: &str) -> (TempDir, DocBase) {
    let scratch = TempDir::new();
    let base = DocBase::open(&scratch.path().join(PRIVATE_DB_FILE)).unwrap();
    for (index, folder) in folders.split(';').filter(|f| !f.trim().is_empty()).enumerate() {
        let folder = Path::new(folder.trim());
        let origin = base.add_origin(&format!("{}", index + 1), folder, &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
    }
    let started = Instant::now();
    while base.hash_pending(32).unwrap() > 0 {}
    while base.extract_pending(8).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    println!("reading: {:.1}s", started.elapsed().as_secs_f32());
    let started = Instant::now();
    while base.similar_step(16).unwrap() > 0 {}
    println!("similar documents: {:.1}s", started.elapsed().as_secs_f32());
    (scratch, base)
}

/// Absolute path (normalized key) of every location, by content.
fn paths_by_content(base: &DocBase) -> (HashMap<String, i64>, BTreeMap<i64, Vec<String>>) {
    let conn = base.conn();
    let mut statement = conn
        .prepare(
            "SELECT o.location, l.rel_path, f.version_id FROM location l
             JOIN origin o ON o.id = l.origin_id JOIN file f ON f.hash = l.file_hash",
        )
        .unwrap();
    let rows: Vec<(String, String, i64)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let mut by_path = HashMap::new();
    let mut by_content: BTreeMap<i64, Vec<String>> = BTreeMap::new();
    for (root, rel_path, content) in rows {
        let absolute = key(&format!("{}/{}", root, rel_path));
        by_path.insert(absolute.clone(), content);
        by_content.entry(content).or_default().push(rel_path);
    }
    for list in by_content.values_mut() {
        list.sort();
    }
    (by_path, by_content)
}

#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK"]
fn similar_documents_in_real_folders() {
    // Over a copy of a base (`MICELYA_DOCBASE_COPY`, of this numbering or
    // of the earlier one) instead of folders: the same measurement of what
    // that base already has.
    let (_scratch, base) = if std::env::var("MICELYA_DOCBASE_COPY").is_ok() {
        let (scratch, base) = crate::docbase::knowledge::check::open_base().unwrap();
        while base.similar_step(16).unwrap() > 0 {}
        (scratch, base)
    } else {
        let Ok(folders) = std::env::var("MICELYA_DOCBASE_CHECK") else {
            println!("MICELYA_DOCBASE_CHECK is not set: nothing to check");
            return;
        };
        run(&folders)
    };
    let (_, by_content) = paths_by_content(&base);
    let name = |content: i64| -> String {
        by_content
            .get(&content)
            .map(|paths| paths.join(" = "))
            .unwrap_or_else(|| format!("#{content}"))
    };

    // What there is.
    let count = |sql: &str| -> i64 { base.conn().query_row(sql, [], |row| row.get(0)).unwrap() };
    println!("\ndocuments: {}", count("SELECT count(*) FROM version"));
    println!("with text, signed: {}", count("SELECT count(*) FROM similar_signature"));
    println!(
        "with enough text to compare: {}",
        count("SELECT count(*) FROM similar_signature WHERE length(shingles) > 0")
    );
    println!("shingles in the archive: {}", count("SELECT count(*) FROM similar_common"));
    println!(
        "routine shingles (pass 0, share {} / at least {} documents): {}",
        super::store::ROUTINE_SHARE,
        super::store::ROUTINE_MIN_DOCS,
        count(&format!(
            "SELECT count(*) FROM similar_common WHERE docs >= max({}, (SELECT count(*) FROM similar_signature WHERE length(shingles) > 0) * {})",
            super::store::ROUTINE_MIN_DOCS,
            super::store::ROUTINE_SHARE
        ))
    );

    // What cannot be compared yet.
    {
        let conn = base.conn();
        let mut statement = conn
            .prepare(
                "SELECT c.id, c.state FROM version c
                 WHERE c.id NOT IN (SELECT version_id FROM similar_signature WHERE length(shingles) > 0) ORDER BY c.state, c.id",
            )
            .unwrap();
        let rows: Vec<(i64, String)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        drop(statement);
        drop(conn);
        println!("
not compared (no text yet, or too little): {}", rows.len());
        for (content, state) in rows {
            println!("   {state:<18} {}", name(content));
        }
    }

    // Pass 2: marks.
    println!("\n== marks (pass 2) ==");
    {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT version_id, marks, signatures, skip_pages FROM document_marks ORDER BY version_id")
            .unwrap();
        let rows: Vec<(i64, String, String, String)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        drop(statement);
        drop(conn);
        for (content, marks, signatures, skip) in rows {
            let marks: Vec<Mark> = serde_json::from_str(&marks).unwrap();
            let signatures: Vec<PdfSignature> = serde_json::from_str(&signatures).unwrap();
            if marks.is_empty() && signatures.is_empty() {
                continue;
            }
            println!("   {}", name(content));
            for mark in marks {
                println!(
                    "      {:<16} {} {} {} {}{}",
                    mark.kind,
                    mark.doc_type,
                    mark.variant.unwrap_or_default(),
                    mark.stage.unwrap_or_default(),
                    mark.signal,
                    mark.page.map(|p| format!(" (page {p})")).unwrap_or_default()
                );
            }
            for signature in signatures {
                let check = signature
                    .check
                    .as_ref()
                    .map(|c| format!(" [{} {}{}]", c.state, c.covers, c.problem.as_deref().map(|p| format!(" {p}")).unwrap_or_default()))
                    .unwrap_or_default();
                println!("      embedded signature: {}{check}", signature.names.join(" | "));
            }
            if skip != "[]" {
                println!("      pages left out of the comparison (from 0): {skip}");
                // To write the catalog's signals: what those pages say.
                if std::env::var("MICELYA_SIMILAR_SHOW_PAGES").is_ok() {
                    let body: String = base
                        .conn()
                        .query_row("SELECT body FROM derivative WHERE version_id = ?1 AND kind = 'text'", [content], |row| row.get(0))
                        .unwrap();
                    let pages = super::text::pages(&body);
                    let skip: Vec<usize> = serde_json::from_str(&skip).unwrap();
                    for page in skip {
                        println!("      --- page {} ---", page + 1);
                        for line in &pages[page] {
                            println!("      | {line}");
                        }
                    }
                }
            }
        }
    }

    // Pass 1: pairs.
    let pairs: Vec<(i64, i64, String, Option<i64>, f64, f64, f64, f64, String, String, String)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare(
                "SELECT a, b, relation, subject, cover, similarity, jaccard, containment, description, certainty, evidence
                 FROM similar_pair ORDER BY relation, cover DESC",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                ))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows
    };
    let mut by_relation: BTreeMap<&str, usize> = BTreeMap::new();
    for pair in &pairs {
        *by_relation.entry(pair.2.as_str()).or_default() += 1;
    }
    println!("\n== pairs (pass 1): {} ==", pairs.len());
    for (relation, n) in &by_relation {
        println!("   {relation:<16} {n}");
    }
    for (a, b, relation, subject, cover, similarity, jaccard, containment, description, certainty, evidence) in &pairs {
        let evidence: serde_json::Value = serde_json::from_str(evidence).unwrap();
        println!(
            "\n   [{relation}] cover {:.2} · similarity {:.2} · jaccard {:.2} · containment {:.2} · {certainty} · rule {}",
            cover,
            similarity,
            jaccard,
            containment,
            evidence["rule"].as_str().unwrap_or_default()
        );
        println!("      a: {}", name(*a));
        println!("      b: {}", name(*b));
        if let Some(subject) = subject {
            println!("      subject: {}", if subject == a { "a" } else { "b" });
        }
        println!("      {description}");
        println!(
            "      changes: {} hunks, data {} / wording {} tokens (replaced: {} / {}), items {}, swaps {}, repeated {}, marks a {} b {}",
            evidence["changes"]["hunks"],
            evidence["changes"]["dataTokens"],
            evidence["changes"]["wordingTokens"],
            evidence["changes"]["replacedDataTokens"],
            evidence["changes"]["replacedWordingTokens"],
            evidence["changes"]["items"],
            evidence["changes"]["swaps"],
            evidence["changes"]["repeated"].as_array().map(|r| r.len()).unwrap_or(0),
            evidence["marks"]["a"],
            evidence["marks"]["b"],
        );
    }

    // Pass 3: families.
    let groups = base.similar_groups(&Visible::All).unwrap();
    println!("\n== families (pass 3): {} ==", groups.len());
    for group in &groups {
        println!("   principal: {}  {:?}", name(group.principal.content_id), group.flags);
        for member in group.members.iter().skip(1) {
            println!(
                "      {} [{}] {:.2}",
                name(member.doc.content_id),
                member.relation.clone().unwrap_or_default(),
                member.cover.unwrap_or_default()
            );
        }
    }

    // Against the answer sheet.
    let Some(answers) = answers() else {
        println!("
MICELYA_SIMILAR_ANSWERS is not set or not readable: no comparison with the answer sheet");
        return;
    };
    against_the_sheet(&base, &answers, true);
}

#[derive(Default)]
struct Tally {
    expected: usize,
    hits: usize,
    wrong: usize,
    missed: usize,
    as_similar: usize,
}

/// What a measurement against a sheet covers (the owner, 27 oct): only the
/// pairs whose two documents the base has with exactly the boundaries the
/// sheet gives are evaluated; each of the others goes, by its number (from
/// 1), to the reason it was not.
#[derive(Default)]
struct Coverage {
    pairs: usize,
    evaluated: Vec<usize>,
    /// Excluded: the sheet does not measure the pair (`measurable: false`).
    not_measurable: Vec<usize>,
    /// Excluded: a file of the pair is not in the base.
    not_in_base: Vec<usize>,
    /// Excluded: a relation the engine does not have.
    unknown_relation: Vec<usize>,
    /// The sheet names a whole file and the base cut it: the sheet does
    /// not say which of its documents the pair is about.
    not_enough_reference: Vec<usize>,
    /// The sheet says which pages and the base has no document with them:
    /// it cut otherwise, cut more, or did not cut.
    cut_errors: Vec<usize>,
}

/// A measurement against a sheet.
struct Measured {
    coverage: Coverage,
    /// The evaluated pairs only: both sides with exact boundaries.
    exact: Tally,
    /// Every pair of a known relation with its two files in the base, each
    /// side by the part with most pages in common: for diagnosis only, it
    /// does not count.
    approximate: Tally,
    /// The versions the evaluated pairs are about.
    named: BTreeSet<i64>,
}

/// What appears in each file of the base: the version of the whole file
/// and, when it was cut, the versions of its parts with their pages.
struct Appearances {
    hash_of: HashMap<String, String>,
    paths_of: HashMap<String, Vec<String>>,
    in_file: HashMap<String, Vec<(i64, Option<(u32, u32)>)>>,
    /// The pages of each file, when the base knows them.
    pages: HashMap<String, u32>,
}

/// What a reference of the sheet (a file, and maybe some of its pages)
/// stands for in the base.
struct Resolved {
    /// The document with exactly the boundaries of the reference when
    /// `exact`; otherwise an approximation, for diagnosis only: the whole
    /// file when it was not cut, or the part with most pages in common.
    version: i64,
    /// The base has a document with exactly the pages the sheet says: the
    /// file not cut, when the sheet names the whole file (no pages, or
    /// all of them); or a part with those very pages.
    exact: bool,
    /// The parts the base has of the file, by pages, when it was cut.
    parts: Vec<(u32, u32)>,
}

fn pages_of(selector: &str) -> Option<(u32, u32)> {
    let (first, last) = selector.strip_prefix("pages:")?.split_once('-')?;
    Some((first.parse().ok()?, last.parse().ok()?))
}

impl Appearances {
    fn of(base: &DocBase) -> Self {
        let conn = base.conn();
        let mut hash_of = HashMap::new();
        let mut paths_of: HashMap<String, Vec<String>> = HashMap::new();
        let mut statement = conn
            .prepare(
                "SELECT o.location, l.rel_path, l.file_hash FROM location l JOIN origin o ON o.id = l.origin_id
                 WHERE l.file_hash IS NOT NULL ORDER BY l.rel_path",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .unwrap();
        for row in rows {
            let (root, rel_path, hash) = row.unwrap();
            hash_of.insert(key(&format!("{root}/{rel_path}")), hash.clone());
            paths_of.entry(hash).or_default().push(rel_path);
        }
        let mut in_file: HashMap<String, Vec<(i64, Option<(u32, u32)>)>> = HashMap::new();
        let mut statement = conn
            .prepare("SELECT resource_hash, version_id, selector FROM appearance ORDER BY resource_hash, version_id")
            .unwrap();
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))
            .unwrap();
        for row in rows {
            let (hash, version, selector) = row.unwrap();
            in_file.entry(hash).or_default().push((version, pages_of(&selector)));
        }
        let mut statement = conn
            .prepare(
                "SELECT resource_hash, max(page_no) FROM (SELECT resource_hash, page_no FROM resource_page
                 UNION ALL SELECT resource_hash, page_no FROM resource_text) GROUP BY resource_hash",
            )
            .unwrap();
        let pages = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, u32>(1)?)))
            .unwrap()
            .collect::<Result<HashMap<_, _>, _>>()
            .unwrap();
        Appearances { hash_of, paths_of, in_file, pages }
    }

    /// What `selector` (pages of the file; the whole file when not said) of
    /// the file at `path` stands for in the base (see `Resolved`); the
    /// approximation takes the first part on a tie.
    fn resolve(&self, root: &str, path: &str, selector: Option<&str>) -> Option<Resolved> {
        let hash = self.hash_of.get(&key(&format!("{root}/{path}")))?;
        let all = self.in_file.get(hash)?;
        let pages = self.pages.get(hash).map(|n| (1, *n));
        // A selector that cannot be read names no pages the base can have.
        let wanted = selector.map(|s| pages_of(s).unwrap_or((0, 0)));
        let mut parts: Vec<(u32, u32, i64)> = all.iter().filter_map(|(v, p)| p.map(|(a, b)| (a, b, *v))).collect();
        parts.sort();
        let ranges: Vec<(u32, u32)> = parts.iter().map(|(a, b, _)| (*a, *b)).collect();
        if parts.is_empty() {
            let whole = all.iter().find(|(_, p)| p.is_none())?.0;
            let exact = wanted.is_none() || wanted == pages;
            return Some(Resolved { version: whole, exact, parts: ranges });
        }
        let target = wanted.or(pages);
        if let Some(part) = parts.iter().find(|(a, b, _)| Some((*a, *b)) == target) {
            return Some(Resolved { version: part.2, exact: true, parts: ranges });
        }
        let wanted = wanted.unwrap_or((1, u32::MAX));
        let common = |a: u32, b: u32| (b.min(wanted.1) + 1).saturating_sub(a.max(wanted.0));
        let best = parts.iter().max_by_key(|(a, b, _)| (common(*a, *b), std::cmp::Reverse(*a)))?;
        Some(Resolved { version: best.2, exact: false, parts: ranges })
    }

    /// The paths of a version, with the pages when it is a part.
    fn name(&self, version: i64) -> String {
        let mut names = Vec::new();
        for (hash, list) in &self.in_file {
            for (v, pages) in list {
                if *v != version {
                    continue;
                }
                for path in self.paths_of.get(hash).into_iter().flatten() {
                    names.push(match pages {
                        Some((a, b)) => format!("{path} [pages {a}-{b}]"),
                        None => path.clone(),
                    });
                }
            }
        }
        names.sort();
        if names.is_empty() {
            format!("#{version}")
        } else {
            names.join(" = ")
        }
    }
}

fn ranges(parts: &[(u32, u32)]) -> String {
    parts.iter().map(|(a, b)| format!("{a}-{b}")).collect::<Vec<_>>().join(", ")
}

/// A pair of the sheet to judge: its number (from 1), the answer, the
/// relation the engine should propose and the two versions.
type ToJudge<'a> = (usize, &'a AnswerPair, &'static str, i64, i64);

/// The pairs of a measurement, judged against what the base proposes.
#[derive(Default)]
struct Judged {
    by_relation: BTreeMap<String, Tally>,
    total: Tally,
    /// Proposals inside a "same template" family of the sheet.
    inferred: Tally,
    wrong: Vec<String>,
    missed: Vec<String>,
    unjudged: Vec<String>,
}

fn judge(pairs: &[ToJudge], proposed: &HashMap<(i64, i64), String>, seen: &Appearances) -> Judged {
    let mut judged = Judged::default();
    let mut said: BTreeSet<(i64, i64)> = BTreeSet::new();
    // Members of a "same template" family, pairwise: the sheet writes
    // them against the principal only.
    let mut template_family: HashMap<i64, i64> = HashMap::new();
    for (number, answer, expected, ca, cb) in pairs {
        let (expected, ca, cb) = (*expected, *ca, *cb);
        if expected == "same-template" {
            let family = *template_family.get(&ca).or(template_family.get(&cb)).unwrap_or(&ca.min(cb));
            template_family.insert(ca, family);
            template_family.insert(cb, family);
        }
        let entry = judged.by_relation.entry(expected.to_string()).or_default();
        entry.expected += 1;
        let (lo, hi) = (ca.min(cb), ca.max(cb));
        said.insert((lo, hi));
        let found = if ca == cb { Some("same-document".to_string()) } else { proposed.get(&(lo, hi)).cloned() };
        let line = format!(
            "pair {number}: expected {} ({}), proposed {}: {} ~ {}{}",
            expected,
            answer.relation,
            found.clone().unwrap_or_else(|| "nothing".into()),
            seen.name(ca),
            seen.name(cb),
            answer.note.as_ref().map(|n| format!(" — {n}")).unwrap_or_default()
        );
        match (expected, found.as_deref()) {
            ("none", None) => entry.hits += 1,
            ("none", Some("similar")) => {
                entry.hits += 1;
                entry.as_similar += 1;
            }
            ("none", Some(_)) => {
                entry.wrong += 1;
                judged.wrong.push(line);
            }
            (_, Some(found)) if found == expected => entry.hits += 1,
            (_, None) => {
                entry.missed += 1;
                judged.missed.push(line);
            }
            (_, Some("similar")) => {
                entry.missed += 1;
                entry.as_similar += 1;
                judged.missed.push(line);
            }
            (_, Some(_)) => {
                entry.wrong += 1;
                judged.wrong.push(line);
            }
        }
    }
    for t in judged.by_relation.values() {
        judged.total.expected += t.expected;
        judged.total.hits += t.hits;
        judged.total.wrong += t.wrong;
        judged.total.missed += t.missed;
        judged.total.as_similar += t.as_similar;
    }
    // Proposals the sheet does not judge directly.
    for ((a, b), relation) in proposed {
        if said.contains(&(*a, *b)) || relation == "similar" {
            continue;
        }
        match (template_family.get(a), template_family.get(b)) {
            (Some(x), Some(y)) if x == y => {
                judged.inferred.expected += 1;
                if relation == "same-template" {
                    judged.inferred.hits += 1;
                } else {
                    judged.inferred.wrong += 1;
                    judged.wrong.push(format!(
                        "expected same-template (same family in the sheet), proposed {relation}: {} ~ {}",
                        seen.name(*a),
                        seen.name(*b)
                    ));
                }
            }
            _ => judged.unjudged.push(format!("[{relation}] {} ~ {}", seen.name(*a), seen.name(*b))),
        }
    }
    judged
}

fn print_tally(judged: &Judged) {
    println!("   {:<16} {:>8} {:>6} {:>6} {:>7} {:>11}", "relation", "expected", "hits", "WRONG", "missed", "as similar");
    for (relation, t) in &judged.by_relation {
        println!("   {:<16} {:>8} {:>6} {:>6} {:>7} {:>11}", relation, t.expected, t.hits, t.wrong, t.missed, t.as_similar);
    }
    let t = &judged.total;
    println!("   {:<16} {:>8} {:>6} {:>6} {:>7} {:>11}", "total", t.expected, t.hits, t.wrong, t.missed, t.as_similar);
    println!(
        "   inside a same-template family of the sheet: {} pairs, {} right, {} wrong",
        judged.inferred.expected, judged.inferred.hits, judged.inferred.wrong
    );
}

fn numbers(list: &[usize]) -> String {
    if list.is_empty() {
        "none".into()
    } else {
        list.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
    }
}

/// How one side of a pair stands: the line that says it when it is not
/// exact, and whether it is a cut error (the sheet says which pages).
fn side_line(number: usize, side: &str, resolved: &Resolved, selector: Option<&str>) -> Option<(String, bool)> {
    if resolved.exact {
        return None;
    }
    let base = if resolved.parts.is_empty() {
        "the file not cut".to_string()
    } else {
        format!("{} parts ({})", resolved.parts.len(), ranges(&resolved.parts))
    };
    Some((
        format!("pair {number} side {side}: the base has {base}; the sheet: {}", selector.unwrap_or("the whole file")),
        selector.is_some(),
    ))
}

/// Measures the pairs of the base against an answer sheet, each pair of
/// the sheet between the documents its files and pages stand for. Only
/// exact boundaries count (the owner, 27 oct): a pair with a side the base
/// does not have exactly is a cut error or lacks a reference, and goes to
/// the coverage, not to the hits, wrong and missed; the earlier count,
/// with the part with most pages in common, is printed after it as a
/// diagnosis. The proposals that wait for a cut to be confirmed (D-556)
/// count as not made: what a person sees.
fn against_the_sheet(base: &DocBase, answers: &Answers, list_unjudged: bool) -> Measured {
    let seen = Appearances::of(base);
    let proposed: HashMap<(i64, i64), String> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare(&format!("SELECT p.a, p.b, p.relation FROM similar_pair p WHERE NOT {}", super::store::waiting_sql("p")))
            .unwrap();
        let rows = statement
            .query_map([], |row| Ok(((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?), row.get::<_, String>(2)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows.into_iter().collect()
    };
    let waiting: i64 = base
        .conn()
        .query_row(&format!("SELECT count(*) FROM similar_pair p WHERE {}", super::store::waiting_sql("p")), [], |row| row.get(0))
        .unwrap();

    let mut coverage = Coverage { pairs: answers.pairs.len(), ..Default::default() };
    let mut exact_pairs: Vec<ToJudge> = Vec::new();
    let mut approximate_pairs: Vec<ToJudge> = Vec::new();
    let mut named: BTreeSet<i64> = BTreeSet::new();
    let mut unknown: Vec<String> = Vec::new();
    // The sides not exact: numbers and pages only.
    let mut cut_lines: Vec<String> = Vec::new();
    let mut not_said_lines: Vec<String> = Vec::new();
    for (index, answer) in answers.pairs.iter().enumerate() {
        let number = index + 1;
        if !answer.measurable {
            coverage.not_measurable.push(number);
            continue;
        }
        let Some(expected) = engine_relation(&answer.relation) else {
            coverage.unknown_relation.push(number);
            unknown.push(format!("relation '{}' (pair {number})", answer.relation));
            continue;
        };
        let a = seen.resolve(&answers.root, &answer.a, answer.a_selector.as_deref());
        let b = seen.resolve(&answers.root, &answer.b, answer.b_selector.as_deref());
        let (Some(a), Some(b)) = (a, b) else {
            coverage.not_in_base.push(number);
            continue;
        };
        let (mut cut_error, mut not_said) = (false, false);
        for (side, resolved, selector) in [("a", &a, &answer.a_selector), ("b", &b, &answer.b_selector)] {
            let Some((line, said)) = side_line(number, side, resolved, selector.as_deref()) else { continue };
            if said {
                cut_error = true;
                cut_lines.push(line);
            } else {
                not_said = true;
                not_said_lines.push(line);
            }
        }
        approximate_pairs.push((number, answer, expected, a.version, b.version));
        if cut_error {
            coverage.cut_errors.push(number);
        } else if not_said {
            coverage.not_enough_reference.push(number);
        } else {
            coverage.evaluated.push(number);
            named.insert(a.version);
            named.insert(b.version);
            exact_pairs.push((number, answer, expected, a.version, b.version));
        }
    }
    let exact = judge(&exact_pairs, &proposed, &seen);
    let approximate = judge(&approximate_pairs, &proposed, &seen);

    let c = &coverage;
    let excluded = c.not_measurable.len() + c.not_in_base.len() + c.unknown_relation.len();
    println!("\n== against the answer sheet: coverage ==");
    println!("   pairs of the sheet: {}", c.pairs);
    println!("   evaluated with exact boundaries: {}", c.evaluated.len());
    println!("   excluded: {excluded}");
    println!("      the sheet does not measure them (measurable: false): {} [{}]", c.not_measurable.len(), numbers(&c.not_measurable));
    println!("      a file the base does not have: {} [{}]", c.not_in_base.len(), numbers(&c.not_in_base));
    println!("      a relation the engine does not have: {} [{}]", c.unknown_relation.len(), numbers(&c.unknown_relation));
    println!(
        "   not enough reference (the sheet names a whole file the base cut): {} [{}]",
        c.not_enough_reference.len(),
        numbers(&c.not_enough_reference)
    );
    println!(
        "   cut errors (the sheet says which pages, the base has no such document): {} [{}]",
        c.cut_errors.len(),
        numbers(&c.cut_errors)
    );
    println!(
        "   of the evaluated: {} hits, {} wrong (another relation), {} missed",
        exact.total.hits, exact.total.wrong, exact.total.missed
    );

    println!("\n== against the answer sheet (evaluated pairs, exact boundaries) ==");
    print_tally(&exact);
    println!("   proposals waiting for a cut to be confirmed (not counted): {waiting}");
    println!("\n   WRONG proposals (the worst outcome, R-07): {}", exact.wrong.len());
    for line in &exact.wrong {
        println!("      {line}");
    }
    println!("\n   missed relations: {}", exact.missed.len());
    for line in &exact.missed {
        println!("      {line}");
    }
    println!("\n   proposals the sheet does not judge: {}", exact.unjudged.len());
    if list_unjudged {
        for line in &exact.unjudged {
            println!("      {line}");
        }
    }
    println!("\n   sides of a cut error: {}", cut_lines.len());
    for line in &cut_lines {
        println!("      {line}");
    }
    println!("\n   sides without enough reference: {}", not_said_lines.len());
    for line in &not_said_lines {
        println!("      {line}");
    }
    if !unknown.is_empty() {
        println!("\n   answers that could not be used: {}", unknown.len());
        for line in &unknown {
            println!("      {line}");
        }
    }

    println!("\n== diagnostic, not counted: each side by the part with most pages in common ==");
    print_tally(&approximate);
    let only_here: Vec<usize> = c.cut_errors.iter().chain(&c.not_enough_reference).copied().collect::<BTreeSet<_>>().into_iter().collect();
    println!("   pairs only in this count: {} [{}]", only_here.len(), numbers(&only_here));

    Measured { exact: exact.total, approximate: approximate.total, named, coverage }
}

#[test]
fn a_side_counts_only_with_exact_boundaries() {
    let file = |name: &str| (key(&format!("root/{name}")), name.to_string());
    let seen = Appearances {
        hash_of: [file("whole"), file("cut")].into_iter().collect(),
        paths_of: HashMap::new(),
        in_file: [
            ("whole".to_string(), vec![(1, None)]),
            ("cut".to_string(), vec![(2, None), (3, Some((1, 1))), (4, Some((2, 4)))]),
        ]
        .into_iter()
        .collect(),
        pages: [("whole".to_string(), 3), ("cut".to_string(), 4)].into_iter().collect(),
    };
    let resolve = |path: &str, selector: Option<&str>| seen.resolve("root", path, selector).unwrap();
    let fate = |path: &str, selector: Option<&str>| side_line(1, "a", &resolve(path, selector), selector).map(|(_, cut)| cut);
    // A file not cut: the whole file, with its pages said or not, is
    // exact; a stretch of it is a cut the base did not make.
    assert!(resolve("whole", None).exact);
    assert!(resolve("whole", Some("pages:1-3")).exact);
    assert_eq!(fate("whole", Some("pages:1-2")), Some(true));
    // A cut file: only a part with those very pages is exact; the part
    // with most pages in common stands for it in the diagnosis only.
    let part = resolve("cut", Some("pages:2-4"));
    assert!(part.exact && part.version == 4);
    let one_document = resolve("cut", Some("pages:1-4"));
    assert!(!one_document.exact && one_document.version == 4);
    assert_eq!(fate("cut", Some("pages:1-4")), Some(true));
    assert_eq!(fate("cut", Some("pages:1-3")), Some(true));
    // The sheet names the whole file and the base cut it: no reference.
    assert_eq!(fate("cut", None), Some(false));
    assert_eq!(fate("whole", None), None);
    assert!(seen.resolve("root", "missing", None).is_none());
}

/// The copies and versions over the whole folder with the joined PDFs cut,
/// each included document on its own: from the base of
/// `MICELYA_DOCBASE_START` and the answers about cuts of
/// `MICELYA_ANSWER_SEED`, the reading with only the answers kept
/// (`MICELYA_ASK_OFFLINE`), measured against every sheet of
/// `MICELYA_SIMILAR_ANSWERS` (separated by `;`) twice: as it is, with the
/// cuts proposed and their proposals waiting, and with every cut confirmed
/// as if a person had looked at them. Each time it also says how much of
/// the cuts has a reference: the joined PDFs the parts sheet of
/// `MICELYA_CUT_SHEET` has, and the included documents no reference
/// covers. `MICELYA_KNOWLEDGE_SAVE`: where a copy of the base is kept at
/// the end.
#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK"]
fn similar_documents_of_the_included_documents() {
    let Some((_scratch, base)) = crate::docbase::knowledge::check::open_base() else {
        println!("MICELYA_DOCBASE_CHECK is not set: nothing to measure");
        return;
    };
    if let Ok(seeds) = std::env::var("MICELYA_ANSWER_SEED") {
        println!("answers seeded: {}", crate::docbase::cut::check::seed_answers(&base, &seeds));
    }
    while base.refresh_extraction(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    while base.profile_pending(16).unwrap() > 0 {}
    while base.cut_pending(16).unwrap() > 0 {}
    while base.cut_parts_pending(64).unwrap() > 0 {}
    let asker = crate::docbase::knowledge::check::recorded();
    let reading = crate::docbase::knowledge::check::read_all(&base, &asker);
    println!("reading, only the answers kept: {} asked, {} read, {} left", reading.asked, reading.extracted, reading.left);
    let started = Instant::now();
    while base.similar_step(16).unwrap() > 0 {}
    println!("similar documents: {:.1}s", started.elapsed().as_secs_f32());
    let sheets: Vec<(String, Answers)> = std::env::var("MICELYA_SIMILAR_ANSWERS")
        .unwrap_or_default()
        .split(';')
        .filter(|p| !p.trim().is_empty())
        .filter_map(|p| {
            let text = std::fs::read_to_string(p.trim()).ok()?;
            let name = Path::new(p.trim()).file_name()?.to_string_lossy().to_string();
            Some((name, serde_json::from_str(&text).map_err(|e| println!("{p}: {e}")).ok()?))
        })
        .collect();
    // The fingerprints of the files of the parts sheet (`files`, `sha256`).
    let parts_sheet: Option<BTreeSet<String>> = std::env::var("MICELYA_CUT_SHEET").ok().and_then(|path| {
        let sheet: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
        Some(sheet["files"].as_array()?.iter().filter_map(|f| Some(f["sha256"].as_str()?.to_lowercase())).collect())
    });
    let report = |when: &str| {
        let count = |sql: &str| -> i64 { base.conn().query_row(sql, [], |row| row.get(0)).unwrap() };
        println!("\n==== {when} ====");
        println!(
            "versions {} · compared {} · cut files {} · parts {} · versions waiting for a cut {} · pairs {}",
            count("SELECT count(*) FROM version"),
            count("SELECT count(*) FROM similar_signature WHERE length(shingles) > 0"),
            count("SELECT count(DISTINCT resource_hash) FROM appearance WHERE selector <> 'file'"),
            count("SELECT count(*) FROM appearance WHERE selector <> 'file'"),
            count(&format!("SELECT count(*) FROM version v WHERE {}", crate::docbase::cut::parts::only_proposed_sql("v.id"))),
            count("SELECT count(*) FROM similar_pair"),
        );
        // The parts of each cut file, by its hash.
        let parts: Vec<(String, i64)> = {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT resource_hash, version_id FROM appearance WHERE selector <> 'file'").unwrap();
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
            rows
        };
        let cut_files: BTreeSet<&str> = parts.iter().map(|(h, _)| h.as_str()).collect();
        match &parts_sheet {
            Some(sheet) => {
                let in_base = sheet
                    .iter()
                    .filter(|h| count(&format!("SELECT count(*) FROM file WHERE hash = '{}'", h.replace('\'', ""))) > 0)
                    .count();
                let cut_in_sheet = sheet.iter().filter(|h| cut_files.contains(h.as_str())).count();
                let referenced = parts.iter().filter(|(h, _)| sheet.contains(h)).count();
                println!(
                    "joined PDFs the base cut: {} · with a reference in the parts sheet: {} · without: {}",
                    cut_files.len(),
                    cut_in_sheet,
                    cut_files.len() - cut_in_sheet
                );
                println!(
                    "files of the parts sheet: {} · cut by the base: {} · in the base and not cut: {} · not in the base: {}",
                    sheet.len(),
                    cut_in_sheet,
                    in_base - cut_in_sheet,
                    sheet.len() - in_base
                );
                println!(
                    "included documents: {} · in a PDF of the parts sheet: {} · without a reference for their boundaries: {}",
                    parts.len(),
                    referenced,
                    parts.len() - referenced
                );
            }
            None => println!("MICELYA_CUT_SHEET is not set or not readable: no count of the cuts with a reference"),
        }
        for (name, sheet) in &sheets {
            println!("\n---- sheet {name} ----");
            let m = against_the_sheet(&base, sheet, false);
            let c = &m.coverage;
            println!(
                "summary {when} · {name}: {} pairs · {} evaluated: {} hits, {} wrong, {} missed · excluded {} (not measurable {}, not in the base {}, unknown relation {}) · not enough reference {} · cut errors {}",
                c.pairs,
                c.evaluated.len(),
                m.exact.hits,
                m.exact.wrong,
                m.exact.missed,
                c.not_measurable.len() + c.not_in_base.len() + c.unknown_relation.len(),
                c.not_measurable.len(),
                c.not_in_base.len(),
                c.unknown_relation.len(),
                c.not_enough_reference.len(),
                c.cut_errors.len()
            );
            println!(
                "   diagnostic, not counted: {} expected, {} hits, {} wrong, {} missed",
                m.approximate.expected, m.approximate.hits, m.approximate.wrong, m.approximate.missed
            );
            let included: BTreeSet<i64> = parts.iter().map(|(_, v)| *v).collect();
            println!(
                "   included documents an evaluated pair is about: {} of {}",
                included.intersection(&m.named).count(),
                included.len()
            );
        }
    };
    report("as it is: cuts proposed, their proposals waiting");
    let cut: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT DISTINCT resource_hash FROM appearance WHERE selector <> 'file'").unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
        rows
    };
    for hash in &cut {
        base.cut_confirm_all(hash).unwrap();
    }
    while base.cut_parts_pending(64).unwrap() > 0 {}
    while base.similar_step(16).unwrap() > 0 {}
    report("every cut confirmed");
    crate::docbase::knowledge::check::save_copy(&base);
}

/// Words that say nothing about a case even though they appear in the
/// names of the test files: generic words of documents, in the two
/// languages of the test folder.
const GENERIC: &[&str] = &[
    // Words of the formulas and kinds of court documents the cutter looks for (cut.json).
    "tengo", "cedula", "solicitud", "provea", "hagase", "dictamen",
    "agreement", "assignment", "exhibit", "termination", "notice", "partnership", "template", "summary", "screenshot",
    "draft", "final", "copia", "copy", "dup", "duplicados", "movidos", "consumos", "facturas", "impagas", "rechazo",
    "rechaza", "factura", "intimacion", "pago", "bloqueo", "consulta", "subscripcion", "anual", "propuesta",
    "facturacion", "local", "servicios", "resumen", "cronologia", "perfil", "cliente", "prestamo", "recibo", "cuota",
    "cancelacion", "total", "teletrabajo", "contrato", "digital", "memorandum", "understanding", "ltd", "llc",
    "firmado", "sin", "firmar", "acuerdo", "terminacion", "non", "disclosure", "one", "way", "docx", "pdf", "jpeg",
    "jpg", "png", "xlsx", "ini", "desktop", "carta", "documento", "correo", "boleta", "abril", "mayo", "junio",
    "julio", "agosto", "septiembre", "octubre", "noviembre", "diciembre", "enero", "febrero", "marzo", "feb", "jun",
    "oct", "nov", "dic", "abr", "del", "los", "las", "and", "the", "with", "por", "para", "con", "sus", "una", "uno",
    "dos", "tres", "todas", "mes", "dia", "sociedad", "anonima", "cloud", "safe",
    // Words of court filings and of addresses (21 oct: a new folder of the set).
    "link", "calle", "social", "nro", "inscripcion", "tribunal", "juzgado", "declara", "presentacion", "letrado",
    "inicia", "despacho", "remision", "pase", "notificado", "notifica", "efecto", "destino", "tasa", "resuelva",
    "ordene", "informe", "herederos", "externo", "contable", "admite", "adjunta",
    // Words of a succession (22 oct: the sheet of a judicial file), which the
    // vocabulary of kinship and inheritance uses.
    "causante", "matrimonio", "oficio", "derecho", "sucesion", "sucesorio", "declaratoria",
    // A word of any written text (22 oct: "the documentation of Tauri" in a
    // design note).
    "documentacion",
    // Words of civil records and of any court filing (25 oct: the design of
    // composite documents speaks of birth and death records in general).
    "partida", "partidas", "nacimiento", "defuncion", "certificacion", "apertura", "cumple", "digitalizacion",
];

/// Words the program uses for its own reasons that a case happens to share:
/// the vendors and models it talks to (a search provider, the model of the
/// reading), ordinary English words of the tests brought from the original
/// project, and a letter of the Greek alphabet. They say nothing about a
/// case.
const NOT_OF_CASES: &[&str] = &["google", "luna", "tea", "foundation", "grant", "iota"];

/// Words of the names of public bodies (courts, registries, the official
/// gazette, public banks, the bar), common words of the trade: an
/// organization of a sheet without a legal form named only with them (and
/// with numbers) is vocabulary, not a case. A person or a company is
/// always a case.
const PUBLIC_BODY_WORDS: &[&str] = &[
    "juzgado", "juzgados", "nacional", "nacion", "primera", "instancia", "inst", "1ra", "civil", "comercial", "fuero", "colegio",
    "publico", "publica", "publicos", "abogados", "capital", "federal", "archivo", "general", "poder", "judicial", "registro", "reg",
    "estado", "capacidad", "personas", "buenos", "aires", "ciudad", "autonoma", "camara", "apelaciones", "ministerio", "fiscal",
    "fiscalia", "agente", "propiedad", "inmueble", "horizontal", "juicios", "universales", "csj", "boletin", "oficial", "republica",
    "argentina", "automotor", "seccional", "secc", "departamento", "dpto", "policia", "division", "delegaciones", "administracion",
    "gubernamental", "ingresos", "direccion", "rentas", "escribanos", "actos", "ultima", "voluntad", "banco", "bancociudad", "consejo",
    "profesional", "ciencias", "economicas", "sistema", "integrado", "previsional", "inspeccion", "justicia", "corte", "suprema",
    "cpacf", "cpce", "agip", "gcba", "igj", "sipa", "informe", "testamento", "nac", "nro", "secretaria", "secretario",
];

/// Words in a text, normalized as everywhere else.
fn tokens(text: &str) -> Vec<String> {
    super::text::normalized(text).split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect()
}

/// What a case can leak into the program: words of the file names and
/// of the sheet's notes that are not generic, long numbers, and the
/// identifiers of the entity sheet.
/// Words that say nothing about a case: the generic list, the words of
/// the design texts written in general terms (R-24), and the companies
/// that issue forms, which may be named.
fn generic_words(catalog: &Catalog) -> BTreeSet<String> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut generic: BTreeSet<String> = GENERIC.iter().chain(NOT_OF_CASES).map(|w| w.to_string()).collect();
    for doc in ["reglas-de-procesamiento.md", "catalogo-de-tipos.md", "parecidos-propuesta.md", "conocimiento-propuesta.md"] {
        let text = std::fs::read_to_string(manifest.join("../docs/design").join(doc)).unwrap_or_default();
        generic.extend(tokens(&text));
    }
    for issuer in catalog.issuers() {
        generic.extend(tokens(&issuer));
    }
    generic
}

fn case_terms(answers: &Answers, entities: &[crate::docbase::knowledge::check::Sheet], catalog: &Catalog) -> BTreeMap<String, String> {
    // A word of a file name that is a common word of the names of public
    // bodies (a court, a registry) says nothing of a case either.
    let mut generic = generic_words(catalog);
    generic.extend(PUBLIC_BODY_WORDS.iter().map(|w| w.to_string()));
    let mut names: Vec<String> = Vec::new();
    for pair in &answers.pairs {
        names.push(pair.a.clone());
        names.push(pair.b.clone());
    }
    names.extend(answers.documents.iter().map(|d| d.path.clone()));
    // Of an identifier, only its numbers: its words ("number") are generic.
    for entity in entities.iter().flat_map(|s| s.entities.iter()) {
        for identifier in &entity.identifiers {
            names.extend(tokens(&identifier.value).into_iter().filter(|w| w.chars().any(|c| c.is_ascii_digit())));
        }
    }
    for entry in walkdir::WalkDir::new(&answers.root).into_iter().flatten() {
        if let Ok(relative) = entry.path().strip_prefix(&answers.root) {
            names.push(relative.to_string_lossy().to_string());
        }
    }
    let mut terms: BTreeMap<String, String> = BTreeMap::new();
    let mut add = |word: &str, from: &str| {
        let digits = word.chars().filter(|c| c.is_ascii_digit()).count();
        let letters = word.len() - digits;
        let is_term = if digits == 0 {
            word.len() >= 3 && !generic.contains(word)
        } else if letters == 0 {
            digits >= 7
        } else {
            digits >= 4 && word.len() >= 6
        };
        if is_term {
            terms.entry(word.to_string()).or_insert_with(|| from.to_string());
        }
    };
    for name in &names {
        for word in tokens(name) {
            add(&word, name);
        }
    }
    for note in answers.pairs.iter().filter_map(|p| p.note.as_ref()) {
        // Only what looks like a name or a number in free text.
        for raw in note.split_whitespace() {
            let trimmed = raw.trim_matches(|c: char| !c.is_alphanumeric());
            if trimmed.chars().next().is_some_and(|c| c.is_uppercase() || c.is_ascii_digit()) {
                for word in tokens(trimmed) {
                    add(&word, note);
                }
            }
        }
    }
    terms
}

/// The names of people and organizations of the entity sheet, as phrases
/// of two words or more with one that is not generic: a single word of a
/// name is often an ordinary word (a plan, a city), a whole name is not.
fn entity_phrases(sheets: &[crate::docbase::knowledge::check::Sheet], catalog: &Catalog) -> BTreeMap<String, String> {
    let generic = generic_words(catalog);
    let vocabulary = crate::docbase::knowledge::vocabulary::vocabulary();
    let mut phrases = BTreeMap::new();
    for entity in sheets.iter().flat_map(|s| s.entities.iter()).filter(|e| e.kind != "asunto") {
        let names: Vec<&String> =
            std::iter::once(&entity.name).chain(&entity.aliases).chain(entity.mentions.iter().map(|m| &m.as_written)).collect();
        // A public body (an organization without a legal form) is named with
        // the common words of the trade too.
        let public_body = entity.kind == "organizacion"
            && names.iter().all(|n| crate::docbase::knowledge::names::key(vocabulary, "organizacion", n).form.is_none());
        let common = |w: &String| generic.contains(w) || (public_body && PUBLIC_BODY_WORDS.contains(&w.as_str()));
        for name in names {
            let words = tokens(name);
            // Of a public body, its numbers (of a court, of a section) are
            // not a case either.
            let counts = |w: &String| w.len() >= 3 && !common(w) && !(public_body && w.chars().all(|c| c.is_ascii_digit()));
            if words.len() >= 2 && words.iter().any(counts) {
                phrases.entry(format!(" {} ", words.join(" "))).or_insert_with(|| entity.id.clone());
            }
        }
    }
    phrases
}

fn files_under(root: &Path, out: &mut Vec<PathBuf>) {
    for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        let path = entry.path();
        let wanted = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e, "rs" | "ts" | "tsx" | "json" | "md"));
        if entry.file_type().is_file() && wanted {
            out.push(path.to_path_buf());
        }
    }
}

#[test]
#[ignore = "reads the answer sheet named in MICELYA_SIMILAR_ANSWERS"]
fn no_case_of_the_answer_sheet_is_in_the_program() {
    let Some(answers) = answers() else {
        println!("MICELYA_SIMILAR_ANSWERS is not set or not readable: nothing to check");
        return;
    };
    let catalog = Catalog::builtin();
    // Every entity sheet (MICELYA_ENTITY_ANSWERS names one or several,
    // separated by `;`).
    let entities = crate::docbase::knowledge::check::sheets();
    if entities.is_empty() {
        println!("MICELYA_ENTITY_ANSWERS is not set or not readable: the entity sheets are not checked");
    }
    println!("entity sheets read: {}", entities.len());
    let terms = case_terms(&answers, &entities, &catalog);
    let phrases = entity_phrases(&entities, &catalog);
    println!(
        "case terms looked for: {}: {}",
        terms.len(),
        terms.keys().cloned().collect::<Vec<_>>().join(", ")
    );

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    // The whole program, its tests and its catalogs, and the design texts.
    // Not the records of what happened (`docs/sessions/`, the register of
    // decisions, the pending list): they are history and are not rewritten.
    let mut files = Vec::new();
    files_under(&manifest.join("src"), &mut files);
    files_under(&manifest.join("resources"), &mut files);
    files_under(&manifest.join("../src"), &mut files);
    files_under(&manifest.join("../docs/design"), &mut files);

    let mut found = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        for (number, line) in text.lines().enumerate() {
            for word in tokens(line) {
                if let Some(source) = terms.get(&word) {
                    found.push(format!("{}:{}: '{word}' (from: {source})", file.display(), number + 1));
                }
            }
            let padded = super::text::padded(line);
            for (phrase, entity) in &phrases {
                if padded.contains(phrase.as_str()) {
                    found.push(format!("{}:{}: '{}' (a name of entity {entity})", file.display(), number + 1, phrase.trim()));
                }
            }
        }
    }
    for line in &found {
        println!("   {line}");
    }
    assert!(found.is_empty(), "{} mentions of a case of the answer sheet (R-24)", found.len());
}

/// Computes the similar documents again over a copy of an app's base
/// (`MICELYA_DOCBASE_COPY`, one file, as `sqlite3.backup()` leaves it),
/// copied first to `MICELYA_SIMILAR_OUT`, where it stays to be compared
/// with the copy; prints the families and the pairs that wait.
#[test]
#[ignore = "reads the copy of a base named in MICELYA_DOCBASE_COPY"]
fn similar_documents_over_a_copy_of_a_base() {
    let (Ok(copy), Ok(out)) = (std::env::var("MICELYA_DOCBASE_COPY"), std::env::var("MICELYA_SIMILAR_OUT")) else {
        println!("MICELYA_DOCBASE_COPY or MICELYA_SIMILAR_OUT is not set: nothing to compute");
        return;
    };
    std::fs::copy(&copy, &out).unwrap();
    let base = DocBase::open(Path::new(&out)).unwrap();
    let started = Instant::now();
    while base.similar_step(16).unwrap() > 0 {}
    println!("similar documents: {:.1}s", started.elapsed().as_secs_f32());
    let groups = base.similar_groups(&Visible::All).unwrap();
    let inside: i64 = groups.iter().map(|g| g.pending).sum();
    let deciding = groups.iter().filter(|g| g.pending > 0).count();
    let open = base.similar_open_pairs(&Visible::All).unwrap();
    let signed_out = open.iter().filter(|p| p.similar.relation == "signed-copy").count();
    println!(
        "families: {} (with something to decide: {deciding}); waiting inside: {inside}, outside: {}; signed copies outside: {signed_out}",
        groups.len(),
        open.len()
    );
}
