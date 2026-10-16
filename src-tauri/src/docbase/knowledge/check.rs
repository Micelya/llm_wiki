//! Runs the extraction of the knowledge layer over real documents, with
//! the real model, and measures it against the entity answer sheet
//! (R-17).
//!
//! Not part of the normal test run: it costs the quota of the configured
//! provider. The documents and the base are only read; the database the
//! run writes is a copy in a scratch folder removed at the end.
//!
//! ```text
//! MICELYA_DOCBASE_COPY="C:\tmp\private.sqlite3" MICELYA_ENTITY_ANSWERS="C:\hoja\entidades.json" \
//! MICELYA_ASK_MODEL="" MICELYA_ASK_CACHE="C:\tmp\cache-modelo.json" MICELYA_ASK_PARALLEL=3 \
//!     cargo test --lib docbase::knowledge::check -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - `MICELYA_DOCBASE_COPY`: a copy of an app's base (with its `-wal` and
//!   `-shm` next to it). It is copied again into the scratch folder and
//!   brought up to date there: the text of scans already recognized, and
//!   the type and data of each document, come with it. Without it,
//!   `MICELYA_DOCBASE_CHECK` names the folders to read, as in
//!   `similar/check_types.rs` (and then the types are asked first).
//! - `MICELYA_ENTITY_ANSWERS`: the answer sheet (outside the repository:
//!   it names real people), `{"root", "entities": [{"id", "type", "name",
//!   "aliases", "identifiers", "mentions": [{"path", "as_written",
//!   "role"}], "claims": [{"predicate", "object", "path", "quote"}]}]}`,
//!   with paths relative to `root`.
//! - `MICELYA_ASK_CACHE`: a file where the model's answers are kept
//!   between runs, by engine and question: a second run asks nothing
//!   already asked. `MICELYA_ASK_PARALLEL`: questions asked at the same
//!   time while warming that cache (1 when not said). `MICELYA_ASK_LIMIT`:
//!   a trial run that asks only this many new questions.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

use serde::Deserialize;

use super::extract::KnowledgeReport;
use super::quote::{self, Located};
use crate::docbase::similar::check_types::{is_limit, key, paths_by_content, prepare};
use crate::docbase::similar::classify;
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible, PRIVATE_DB_FILE};
use crate::source_volume::asking::{AskConfig, DocumentAsker};

#[derive(Debug, Deserialize)]
pub(crate) struct Sheet {
    pub root: String,
    #[serde(default)]
    pub entities: Vec<SheetEntity>,
    #[serde(default)]
    pub false_mentions: Vec<SheetFalseMention>,
    /// Pairs of entities that look the same and are not.
    #[serde(default)]
    pub distinct: Vec<SheetPair>,
    /// Pairs the layer should leave in doubt.
    #[serde(default)]
    pub doubtful: Vec<SheetPair>,
    /// Mentions that cannot be told to be anyone.
    #[serde(default)]
    pub unresolvable: Vec<SheetFalseMention>,
    /// The matters: groups of documents.
    #[serde(default)]
    pub matters: Vec<SheetMatter>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SheetMatter {
    pub id: String,
    #[serde(default)]
    pub documents: Vec<String>,
    #[serde(default)]
    pub entities: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SheetPair {
    pub a: String,
    pub b: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SheetFalseMention {
    pub path: String,
    pub as_written: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SheetEntity {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default)]
    pub identifiers: Vec<SheetIdentifier>,
    #[serde(default)]
    pub mentions: Vec<SheetMention>,
    #[serde(default)]
    pub claims: Vec<SheetClaim>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SheetIdentifier {
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SheetMention {
    pub path: String,
    pub as_written: String,
    #[serde(default)]
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct SheetClaim {
    pub predicate: String,
    #[serde(default)]
    pub object: serde_json::Value,
    pub path: String,
    #[serde(default)]
    pub quote: String,
}

/// The entity answer sheet named in `MICELYA_ENTITY_ANSWERS`.
pub(crate) fn sheet() -> Option<Sheet> {
    let path = std::env::var("MICELYA_ENTITY_ANSWERS").ok()?;
    let text = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str(&text) {
        Ok(sheet) => Some(sheet),
        Err(error) => {
            println!("the entity answer sheet {path} is not readable: {error}");
            None
        }
    }
}

/// The model, with its answers kept in `MICELYA_ASK_CACHE` by engine and
/// question; answers from an earlier run are not counted.
pub(crate) struct Recorded {
    inner: Box<dyn DocumentAsker>,
    file: Option<PathBuf>,
    saved: Mutex<BTreeMap<String, String>>,
    pub asked: AtomicUsize,
}

impl Recorded {
    fn key(&self, prompt: &str) -> String {
        classify::question_key("check", &format!("{}\u{0}{prompt}", self.inner.engine_id()))
    }

    pub(crate) fn has(&self, prompt: &str) -> bool {
        self.saved.lock().unwrap().contains_key(&self.key(prompt))
    }
}

impl DocumentAsker for Recorded {
    fn engine_id(&self) -> String {
        self.inner.engine_id()
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        let key = self.key(prompt);
        if let Some(answer) = self.saved.lock().unwrap().get(&key) {
            return Ok(answer.clone());
        }
        // A provider at its usage limit is waited for, a few times.
        let mut attempt = 0;
        let answer = loop {
            self.asked.fetch_add(1, Ordering::SeqCst);
            match self.inner.ask(prompt, schema) {
                Ok(answer) => break answer,
                Err(error) if attempt < 6 && is_limit(&error) => {
                    attempt += 1;
                    println!("the provider is at its limit ({error}); waiting before trying again");
                    std::thread::sleep(std::time::Duration::from_secs(300));
                }
                Err(error) => return Err(error),
            }
        };
        let mut saved = self.saved.lock().unwrap();
        saved.insert(key, answer.clone());
        if let Some(file) = &self.file {
            let _ = std::fs::write(file, serde_json::to_string_pretty(&*saved).unwrap());
        }
        Ok(answer)
    }
}

/// A scratch copy of the base in `MICELYA_DOCBASE_COPY`, or the folders
/// of `MICELYA_DOCBASE_CHECK` read into a scratch base.
pub(crate) fn open_base() -> Option<(TempDir, DocBase)> {
    if let Ok(copy) = std::env::var("MICELYA_DOCBASE_COPY") {
        let scratch = TempDir::new();
        let target = scratch.path().join(PRIVATE_DB_FILE);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        for suffix in ["", "-wal", "-shm"] {
            let from = PathBuf::from(format!("{copy}{suffix}"));
            if from.exists() {
                std::fs::copy(&from, PathBuf::from(format!("{}{suffix}", target.display()))).unwrap();
            }
        }
        // A base another branch already took to its own next version (the
        // attention view adds a table) is taken back one version on the
        // copy, so this branch's migration runs; the other tables stay.
        {
            let conn = rusqlite::Connection::open(&target).unwrap();
            let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
            let ours: i64 = conn
                .query_row("SELECT count(*) FROM sqlite_master WHERE name = 'knowledge_extraction'", [], |row| row.get(0))
                .unwrap();
            // The knowledge layer starts at schema 8 on this branch.
            if ours == 0 && version >= 8 {
                println!("the copy is at schema {version} without this branch's tables: taken back to 7");
                conn.pragma_update(None, "user_version", 7).unwrap();
            }
        }
        let base = DocBase::open(&target).unwrap();
        return Some((scratch, base));
    }
    let folders = std::env::var("MICELYA_DOCBASE_CHECK").ok()?;
    Some(prepare(&folders))
}

/// Keeps a copy of the scratch base, after the run, in the file named in
/// `MICELYA_KNOWLEDGE_SAVE` (outside the repository: it holds real
/// documents), to look into it by hand.
pub(crate) fn save_copy(base: &DocBase) {
    let Ok(path) = std::env::var("MICELYA_KNOWLEDGE_SAVE") else { return };
    let _ = std::fs::remove_file(&path);
    match base.conn().execute("VACUUM INTO ?1", [&path]) {
        Ok(_) => println!("   a copy of the base after the run: {path}"),
        Err(error) => println!("   could not keep a copy of the base in {path}: {error}"),
    }
}

/// Whether two names, normalized, are the same one: equal, or the words
/// of one all among those of the other (two words at least).
pub(crate) fn names_match(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let a: BTreeSet<&str> = a.split(' ').filter(|w| !w.is_empty()).collect();
    let b: BTreeSet<&str> = b.split(' ').filter(|w| !w.is_empty()).collect();
    let (short, long) = if a.len() <= b.len() { (&a, &b) } else { (&b, &a) };
    short.len() >= 2 && short.is_subset(long)
}

pub(crate) fn normalized(text: &str) -> String {
    quote::words(text).join(" ")
}

#[derive(Debug, Clone)]
struct Found {
    id: i64,
    kind: String,
    normalized: String,
    role: String,
}

fn ratio(a: usize, b: usize) -> String {
    if b == 0 {
        "-".to_string()
    } else {
        format!("{:.0} %", 100.0 * a as f64 / b as f64)
    }
}

/// The model of the evaluations: Codex CLI with `MICELYA_ASK_MODEL`, its
/// answers kept in `MICELYA_ASK_CACHE`.
pub(crate) fn recorded() -> Recorded {
    let file = std::env::var("MICELYA_ASK_CACHE").ok().map(PathBuf::from);
    let saved: BTreeMap<String, String> = file
        .as_ref()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    Recorded {
        inner: crate::source_volume::asking::asker_for(&AskConfig {
            engine: "codex-cli".into(),
            model: std::env::var("MICELYA_ASK_MODEL").unwrap_or_default(),
        })
        .unwrap(),
        file,
        saved: Mutex::new(saved),
        asked: AtomicUsize::new(0),
    }
}

/// Questions asked at the same time while warming the cache.
pub(crate) fn parallel() -> usize {
    std::env::var("MICELYA_ASK_PARALLEL").ok().and_then(|p| p.parse().ok()).unwrap_or(1).max(1)
}

/// A trial run: only this many new questions.
pub(crate) fn limit() -> Option<usize> {
    std::env::var("MICELYA_ASK_LIMIT").ok().and_then(|l| l.parse().ok())
}

/// Asks the questions not yet in the cache, a few at a time. Returns the
/// failures.
pub(crate) fn ask_in_parallel(asker: &Recorded, mut questions: Vec<classify::Question>, what: &str) -> Vec<String> {
    questions.retain(|q| !asker.has(&q.prompt));
    let mut seen = BTreeSet::new();
    questions.retain(|q| seen.insert(q.key.clone()));
    if let Some(limit) = limit() {
        questions.truncate(limit);
    }
    let parallel = parallel();
    println!("{what}: {} questions not yet in the cache; asking {parallel} at a time", questions.len());
    let next = AtomicUsize::new(0);
    let failures = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for _ in 0..parallel {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some(question) = questions.get(index) else { break };
                let one = Instant::now();
                match asker.ask(&question.prompt, &question.schema) {
                    Ok(_) => println!(
                        "   answered {}/{} (document {}) in {:.0}s",
                        index + 1,
                        questions.len(),
                        question.content_id,
                        one.elapsed().as_secs_f32()
                    ),
                    Err(error) => {
                        println!("   failed {} (document {}): {error}", index + 1, question.content_id);
                        failures.lock().unwrap().push(error);
                    }
                }
            });
        }
    });
    failures.into_inner().unwrap()
}

/// Types first (a hint of the reading), then the reading of every
/// document, with the questions warmed in the cache a few at a time.
pub(crate) fn read_all(base: &DocBase, asker: &Recorded) -> KnowledgeReport {
    let started = Instant::now();
    loop {
        let report = base.types_run(asker, 100).unwrap();
        if report.asked + report.data_asked == 0 {
            break;
        }
    }
    let types_asked = asker.asked.load(Ordering::SeqCst);
    println!("types: {:.1}s, {types_asked} questions to the model", started.elapsed().as_secs_f32());
    let started = Instant::now();
    let failures = ask_in_parallel(asker, base.knowledge_questions().unwrap(), "knowledge");
    let mut total = KnowledgeReport::default();
    loop {
        let report = base.knowledge_run(asker, 1000).unwrap();
        let more = report.extracted > 0;
        total.add(&report);
        if !more {
            break;
        }
    }
    println!(
        "knowledge: {:.1}s; questions to the model in this run: {} (answers kept from earlier runs not counted); failures: {}",
        started.elapsed().as_secs_f32(),
        asker.asked.load(Ordering::SeqCst) - types_asked,
        failures.len()
    );
    println!("   {total:?}");
    total
}

#[test]
#[ignore = "reads a real base or real folders and asks the real model"]
fn extraction_against_the_entity_sheet() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY or MICELYA_DOCBASE_CHECK is not set: nothing to check");
        return;
    };
    let asker = recorded();
    read_all(&base, &asker);
    let status = base.knowledge_status(&Visible::All).unwrap();
    println!("\n== status ==\n   {status:?}");
    save_copy(&base);

    let (by_path, by_content) = paths_by_content(&base);
    let name = |content: i64| by_content.get(&content).map(|p| p.join(" = ")).unwrap_or_else(|| format!("#{content}"));
    let found: BTreeMap<i64, Vec<Found>> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT content_id, id, kind, normalized, role FROM knowledge_mention ORDER BY id").unwrap();
        let rows: Vec<(i64, Found)> = statement
            .query_map([], |row| {
                Ok((row.get(0)?, Found { id: row.get(1)?, kind: row.get(2)?, normalized: row.get(3)?, role: row.get(4)? }))
            })
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut out: BTreeMap<i64, Vec<Found>> = BTreeMap::new();
        for (content, mention) in rows {
            out.entry(content).or_default().push(mention);
        }
        out
    };
    // Evidence by document: (subject, predicate, object mention).
    let evidence: BTreeMap<i64, Vec<(i64, String, Option<i64>)>> = {
        let conn = base.conn();
        let mut statement =
            conn.prepare("SELECT content_id, subject_mention, predicate, object_mention FROM knowledge_evidence").unwrap();
        let rows: Vec<(i64, i64, String, Option<i64>)> = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        let mut out: BTreeMap<i64, Vec<(i64, String, Option<i64>)>> = BTreeMap::new();
        for (content, subject, predicate, object) in rows {
            out.entry(content).or_default().push((subject, predicate, object));
        }
        out
    };
    // Identifiers by mention, normalized.
    let identifiers: HashMap<i64, Vec<String>> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT mention_id, normalized FROM knowledge_identifier").unwrap();
        let rows: Vec<(i64, String)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        let mut out: HashMap<i64, Vec<String>> = HashMap::new();
        for (mention, value) in rows {
            out.entry(mention).or_default().push(value);
        }
        out
    };
    println!("\n== discarded by the check ==");
    {
        let conn = base.conn();
        let mut statement =
            conn.prepare("SELECT content_id, discarded FROM knowledge_extraction WHERE discarded != '[]' ORDER BY content_id").unwrap();
        let rows: Vec<(i64, String)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        for (content, discarded) in rows {
            let items: Vec<super::Discarded> = serde_json::from_str(&discarded).unwrap_or_default();
            println!("   {} ({} discarded)", name(content), items.len());
            for item in items {
                println!("      [{}] {} {:?}: «{}»", item.what, item.reason, item.said, item.quote);
            }
        }
    }

    let Some(sheet) = sheet() else {
        println!("\nMICELYA_ENTITY_ANSWERS is not set or not readable: no comparison with the answer sheet");
        return;
    };
    let content_of = |path: &str| -> Option<i64> { by_path.get(&key(&format!("{}/{}", sheet.root, path))).copied() };

    // Expected mentions by document: (entity, type, name, role).
    let mut expected: BTreeMap<i64, Vec<(&SheetEntity, String, String)>> = BTreeMap::new();
    let mut missing_docs: BTreeSet<&str> = BTreeSet::new();
    for entity in &sheet.entities {
        for mention in &entity.mentions {
            match content_of(&mention.path) {
                Some(content) => expected.entry(content).or_default().push((entity, normalized(&mention.as_written), mention.role.clone())),
                None => {
                    missing_docs.insert(mention.path.as_str());
                }
            }
        }
    }
    for path in &missing_docs {
        println!("   not in the base: {path}");
    }

    println!("\n== mentions by document ==");
    println!("   {:>4} {:>5} {:>5} {:>6} {:>5} {:>5}  document", "exp", "found", "hit", "strict", "role", "extra");
    let (mut all_expected, mut all_hits, mut all_strict, mut all_roles, mut all_found, mut all_found_hits) = (0, 0, 0, 0, 0, 0);
    let mut misses = Vec::new();
    let mut extras = Vec::new();
    let mut by_type: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let documents: BTreeSet<i64> = expected.keys().chain(found.keys()).copied().collect();
    for content in documents {
        let want = expected.get(&content).cloned().unwrap_or_default();
        let got = found.get(&content).cloned().unwrap_or_default();
        let (mut hits, mut strict, mut roles) = (0, 0, 0);
        for (entity, written, role) in &want {
            let names: Vec<String> =
                std::iter::once(written.clone()).chain(std::iter::once(normalized(&entity.name))).collect();
            let matching: Vec<&Found> = got
                .iter()
                .filter(|f| f.kind == entity.kind && names.iter().any(|n| names_match(n, &f.normalized)))
                .collect();
            let entry = by_type.entry(entity.kind.clone()).or_default();
            entry.0 += 1;
            if matching.is_empty() {
                misses.push(format!("   {} «{}» ({}, {role}) in {}", entity.id, written, entity.kind, name(content)));
                continue;
            }
            entry.1 += 1;
            hits += 1;
            if matching.iter().any(|f| &f.normalized == written) {
                strict += 1;
            }
            if matching.iter().any(|f| &f.role == role) {
                roles += 1;
            }
        }
        let mut found_hits = 0;
        for f in &got {
            let hit = want.iter().any(|(entity, written, _)| {
                f.kind == entity.kind
                    && (names_match(written, &f.normalized)
                        || names_match(&normalized(&entity.name), &f.normalized)
                        || entity.aliases.iter().any(|a| names_match(&normalized(a), &f.normalized)))
            });
            if hit {
                found_hits += 1;
            } else {
                extras.push(format!("   «{}» ({}, {}) in {}", f.normalized, f.kind, f.role, name(content)));
            }
        }
        println!(
            "   {:>4} {:>5} {:>5} {:>6} {:>5} {:>5}  {}",
            want.len(),
            got.len(),
            hits,
            strict,
            roles,
            got.len() - found_hits,
            name(content)
        );
        all_expected += want.len();
        all_hits += hits;
        all_strict += strict;
        all_roles += roles;
        all_found += got.len();
        all_found_hits += found_hits;
    }
    println!(
        "\n   mentions of the sheet found: {all_hits} of {all_expected} ({}); written exactly the same: {all_strict}; with the same role: {all_roles} ({} of those found)",
        ratio(all_hits, all_expected),
        ratio(all_roles, all_hits)
    );
    println!(
        "   mentions found that the sheet has: {all_found_hits} of {all_found} ({}); the rest are not in the sheet (true or not)",
        ratio(all_found_hits, all_found)
    );
    for (kind, (want, hit)) in &by_type {
        println!("   {kind}: {hit} of {want} ({})", ratio(*hit, *want));
    }
    println!("\n   mentions of the sheet not found: {}", misses.len());
    for line in &misses {
        println!("   {line}");
    }
    println!("\n   mentions found that the sheet does not have: {}", extras.len());
    for line in &extras {
        println!("   {line}");
    }

    // Claims: the sheet's predicates are free words; each is compared with
    // the predicates of the vocabulary that say the same. An identifier
    // the sheet gives as a claim is found among the mention's identifiers.
    println!("\n== claims ==");
    let texts: HashMap<i64, Located> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT content_id, body FROM derivative WHERE kind = 'text'").unwrap();
        let rows: Vec<(i64, String)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
        rows.into_iter().map(|(id, body)| (id, Located::new(&body))).collect()
    };
    let (mut claims, mut comparable, mut claim_hits, mut id_claims, mut id_hits, mut sheet_quotes_found) = (0, 0, 0, 0, 0, 0);
    let mut claim_misses = Vec::new();
    let mut not_in_vocabulary: BTreeMap<String, usize> = BTreeMap::new();
    let mut per_predicate: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for entity in &sheet.entities {
        let mut names: Vec<String> = vec![normalized(&entity.name)];
        names.extend(entity.aliases.iter().map(|a| normalized(a)));
        names.extend(entity.mentions.iter().map(|m| normalized(&m.as_written)));
        for claim in &entity.claims {
            let Some(content) = content_of(&claim.path) else { continue };
            claims += 1;
            if texts.get(&content).is_some_and(|t| t.locate(&claim.quote, None).is_some()) {
                sheet_quotes_found += 1;
            }
            let mentions: Vec<i64> = found
                .get(&content)
                .map(|list| {
                    list.iter()
                        .filter(|f| f.kind == entity.kind && names.iter().any(|n| names_match(n, &f.normalized)))
                        .map(|f| f.id)
                        .collect()
                })
                .unwrap_or_default();
            if IDENTIFIER_PREDICATES.contains(&claim.predicate.as_str()) {
                id_claims += 1;
                let value = normalized(claim.object.as_str().unwrap_or_default());
                let hit = mentions.iter().any(|m| identifiers.get(m).is_some_and(|ids| ids.iter().any(|i| i == &value)));
                if hit {
                    id_hits += 1;
                } else {
                    claim_misses.push(format!("   {} [identifier] {} = {} in {}", entity.id, claim.predicate, claim.object, name(content)));
                }
                continue;
            }
            let Some(ours) = vocabulary_predicates(&claim.predicate) else {
                *not_in_vocabulary.entry(claim.predicate.clone()).or_default() += 1;
                continue;
            };
            comparable += 1;
            let hit = evidence.get(&content).is_some_and(|list| {
                list.iter().any(|(subject, predicate, object)| {
                    ours.contains(&predicate.as_str())
                        && (mentions.contains(subject) || object.is_some_and(|o| mentions.contains(&o)))
                })
            });
            let entry = per_predicate.entry(claim.predicate.clone()).or_default();
            entry.0 += 1;
            if hit {
                entry.1 += 1;
                claim_hits += 1;
            } else {
                claim_misses.push(format!("   {} {} = {} in {}", entity.id, claim.predicate, claim.object, name(content)));
            }
        }
    }
    let found_evidence: usize = evidence.values().map(Vec::len).sum();
    println!(
        "   claims of the sheet: {claims}; with a predicate of the vocabulary: {comparable}, found {claim_hits} ({}); identifiers given as claims: {id_claims}, found {id_hits} ({})",
        ratio(claim_hits, comparable),
        ratio(id_hits, id_claims)
    );
    println!("   evidence kept in all: {found_evidence}; quotes of the sheet that the check finds in the text: {sheet_quotes_found} of {claims}");
    for (predicate, (want, hit)) in &per_predicate {
        println!("      {predicate}: {hit} of {want}");
    }
    println!(
        "   predicates of the sheet with no equivalent in the vocabulary: {}",
        not_in_vocabulary.iter().map(|(p, n)| format!("{p} ({n})")).collect::<Vec<_>>().join(", ")
    );
    println!("\n   claims of the sheet not found: {}", claim_misses.len());
    for line in &claim_misses {
        println!("   {line}");
    }

    // Strings that look like an entity and are not one.
    let mut false_found = Vec::new();
    for item in &sheet.false_mentions {
        let Some(content) = content_of(&item.path) else { continue };
        let written = normalized(&item.as_written);
        if found.get(&content).is_some_and(|list| list.iter().any(|f| f.normalized == written)) {
            false_found.push(format!("   «{}» in {}", item.as_written, name(content)));
        }
    }
    println!("\n== false mentions of the sheet extracted: {} of {} ==", false_found.len(), sheet.false_mentions.len());
    for line in &false_found {
        println!("   {line}");
    }
}

/// The sheet's predicates that are an identifier of the entity.
const IDENTIFIER_PREDICATES: &[&str] = &["dni", "cuit", "cuil", "registro", "matricula", "expediente"];

/// The predicates of the vocabulary that say what a predicate of the
/// sheet says, when there are such.
pub(crate) fn vocabulary_predicates(sheet: &str) -> Option<&'static [&'static str]> {
    Some(match sheet {
        "cargo" => &["cargo"],
        "domicilio" | "domicilio_constituido" | "domicilio_constituido_por" => &["domicilio"],
        "representa_a" | "representada_por" | "apoderado_de" => &["representa_a"],
        "socio_de" => &["socio_de"],
        "empleado_de" | "trabaja_en" | "contratista_de" | "integra_equipo_de" | "pertenece_a" => &["empleado_de"],
        "deudor_de" => &["deudor_de", "acreedor_de"],
        "acreedor_de" => &["acreedor_de", "deudor_de"],
        "monto" | "monto_reclamado" | "saldo_pendiente" | "monto_adeudado" => &["monto_reclamado", "monto_adeudado"],
        "parte_en" => &["parte_en"],
        "fecha" | "plazo" | "inicio_de_vinculo" | "fin_de_vinculo" => &["inicio_de_vinculo", "fin_de_vinculo"],
        "profesion" => &["profesion"],
        "nacionalidad" => &["nacionalidad"],
        "estado_civil" => &["estado_civil"],
        "casilla" | "contacto" | "correo" | "telefono" => &["contacto"],
        _ => return None,
    })
}
