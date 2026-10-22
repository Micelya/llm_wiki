//! Runs passes 4 to 6 (type and stage, data by type, relations by type)
//! over real folders, with the real model, and measures them against the
//! answer sheet (R-17).
//!
//! Not part of the normal test run: it costs the quota of the configured
//! provider. The folders are only read; the database goes to a scratch
//! folder removed at the end.
//!
//! ```text
//! MICELYA_DOCBASE_CHECK="C:\casos" MICELYA_SIMILAR_ANSWERS="C:\hoja\respuestas.json" \
//! MICELYA_RECOGNIZED_CACHE="C:\tmp\reconocido" MICELYA_RECOGNIZED_SEED="C:\proyecto\raw\recognized" \
//! MICELYA_RECOGNIZE=1 MICELYA_ASK_MODEL="" \
//!     cargo test --lib docbase::similar::check_types -- --ignored --nocapture --test-threads=1
//! ```
//!
//! - `MICELYA_RECOGNIZED_CACHE`: where the text read from images is kept
//!   between runs, one folder per file fingerprint (the layout of a
//!   project's `raw/recognized`).
//! - `MICELYA_RECOGNIZED_SEED`: folders with that same layout whose text
//!   is copied (never moved) into the cache before reading anything.
//! - `MICELYA_RECOGNIZE=1`: read with Codex the pages still missing;
//!   without it, documents with missing pages stay without text.
//! - `MICELYA_ASK_CACHE`: a file where the model's answers are kept
//!   between runs, so a second run asks nothing already asked.
//! - `MICELYA_TEXTS_OUT`: a folder where the text of every document is
//!   written, to read while writing the catalog's rules.

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::Instant;

use serde::Deserialize;

use super::classify::{self, RunReport};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, PRIVATE_DB_FILE};
use crate::source_volume::asking::{AskConfig, DocumentAsker};
use crate::source_volume::recognition::{self, RecognitionConfig, TextRecognizer};
use crate::source_volume::recognized_store::RecognizedStore;

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
    #[serde(default)]
    measurable: bool,
}

#[derive(Debug, Deserialize)]
struct AnswerDocument {
    path: String,
    #[serde(rename = "type")]
    doc_type: Option<String>,
    stage: Option<String>,
}

pub(crate) fn key(path: &str) -> String {
    path.replace('\\', "/").trim_end_matches('/').to_lowercase()
}

/// Counts the pages handed to the engine.
struct Counting<'a> {
    inner: Option<&'a dyn TextRecognizer>,
    calls: Cell<usize>,
}

impl TextRecognizer for Counting<'_> {
    fn engine_id(&self) -> String {
        self.inner.map(|r| r.engine_id()).unwrap_or_else(|| "codex-cli".to_string())
    }

    fn recognize(&self, image: &Path) -> Result<String, String> {
        let Some(inner) = self.inner else {
            return Err("recognition is off (MICELYA_RECOGNIZE)".to_string());
        };
        self.calls.set(self.calls.get() + 1);
        inner.recognize(image)
    }

    fn recognize_page(&self, image: &Path, page: u32) -> Result<recognition::PageReading, String> {
        let Some(inner) = self.inner else {
            return Err("recognition is off (MICELYA_RECOGNIZE)".to_string());
        };
        self.calls.set(self.calls.get() + 1);
        inner.recognize_page(image, page)
    }
}

fn copy_seed(seeds: &str, cache: &Path, hash: &str) {
    for seed in seeds.split(';').filter(|s| !s.trim().is_empty()) {
        let from = Path::new(seed.trim()).join(hash);
        let Ok(entries) = std::fs::read_dir(&from) else {
            continue;
        };
        let to = cache.join(hash);
        let _ = std::fs::create_dir_all(&to);
        for entry in entries.flatten() {
            let target = to.join(entry.file_name());
            if !target.exists() {
                let _ = std::fs::copy(entry.path(), target);
            }
        }
    }
}

/// Reads the folders into a scratch base, with the text of images.
pub(crate) fn prepare(folders: &str) -> (TempDir, DocBase) {
    let scratch = TempDir::new();
    let base = DocBase::open(&scratch.path().join(PRIVATE_DB_FILE)).unwrap();
    for (index, folder) in folders.split(';').filter(|f| !f.trim().is_empty()).enumerate() {
        let origin = base.add_origin(&format!("{}", index + 1), Path::new(folder.trim()), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
    }
    while base.hash_pending(32).unwrap() > 0 {}
    while base.extract_pending(8).unwrap() > 0 {}
    base.purge_orphans().unwrap();

    // Text read from images: from the cache, from the seeds, and from
    // Codex when asked to.
    let cache = std::env::var("MICELYA_RECOGNIZED_CACHE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| scratch.path().join("recognized"));
    let seeds = std::env::var("MICELYA_RECOGNIZED_SEED").unwrap_or_default();
    let engine = (std::env::var("MICELYA_RECOGNIZE").as_deref() == Ok("1")).then(|| {
        recognition::recognizer_for(&RecognitionConfig {
            engine: "codex-cli".into(),
            model: std::env::var("MICELYA_ASK_MODEL").unwrap_or_default(),
        })
        .unwrap()
    });
    let counting = Counting { inner: engine.as_deref(), calls: Cell::new(0) };
    let waiting: Vec<(String, String, String)> = {
        let conn = base.conn();
        let mut statement = conn
            .prepare(
                "SELECT f.hash, min(o.location || '/' || l.rel_path), min(l.ext) FROM content c
                 JOIN file f ON f.content_id = c.id JOIN location l ON l.file_hash = f.hash
                 JOIN origin o ON o.id = l.origin_id
                 WHERE c.state = 'needs-recognition' GROUP BY f.hash",
            )
            .unwrap();
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows
    };
    let mut recognized = 0;
    for (hash, path, ext) in &waiting {
        copy_seed(&seeds, &cache, hash);
        let store = RecognizedStore::at(cache.join(hash));
        let path = Path::new(path);
        let text = if ext == "pdf" {
            crate::commands::fs::extract_text_without_recognition(path)
                .and_then(|extracted| recognition::complete_pdf_text(path, &extracted.unwrap_or_default(), &store, &counting))
        } else {
            recognition::recognize_image_source(path, &store, &counting)
        };
        match text {
            Ok(text) => {
                if base.store_recognized_text(hash, &text, &counting.engine_id()).unwrap() {
                    recognized += 1;
                }
            }
            Err(error) => println!("not recognized: {} ({error})", path.display()),
        }
    }
    println!(
        "text read from images: {recognized} of {} documents; pages read by Codex now: {}",
        waiting.len(),
        counting.calls.get()
    );
    while base.similar_step(16).unwrap() > 0 {}
    (scratch, base)
}

pub(crate) fn paths_by_content(base: &DocBase) -> (HashMap<String, i64>, BTreeMap<i64, Vec<String>>) {
    let conn = base.conn();
    let mut statement = conn
        .prepare(
            "SELECT o.location, l.rel_path, f.content_id FROM location l
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
        by_path.insert(key(&format!("{}/{}", root, rel_path)), content);
        by_content.entry(content).or_default().push(rel_path);
    }
    for list in by_content.values_mut() {
        list.sort();
    }
    (by_path, by_content)
}

/// Writes the text of every document to `MICELYA_TEXTS_OUT`.
#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK"]
fn texts_of_real_folders() {
    let (Ok(folders), Ok(out)) = (std::env::var("MICELYA_DOCBASE_CHECK"), std::env::var("MICELYA_TEXTS_OUT")) else {
        println!("MICELYA_DOCBASE_CHECK or MICELYA_TEXTS_OUT is not set: nothing to do");
        return;
    };
    let (_scratch, base) = prepare(&folders);
    let (_, by_content) = paths_by_content(&base);
    let texts: Vec<(i64, String)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT content_id, body FROM derivative WHERE kind = 'text'").unwrap();
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows
    };
    std::fs::create_dir_all(&out).unwrap();
    for (content, body) in texts {
        let name = by_content.get(&content).and_then(|p| p.first()).cloned().unwrap_or_default();
        let file = format!("{content:03} {}.txt", name.replace(['/', '\\', ':'], "_"));
        std::fs::write(Path::new(&out).join(file), format!("{}\n\n{body}", by_content.get(&content).map(|p| p.join(" = ")).unwrap_or_default())).unwrap();
    }
}

/// The model, counting what it is asked; answers already given in an
/// earlier run come from `MICELYA_ASK_CACHE` and are not counted.
struct Recorded {
    inner: Box<dyn DocumentAsker>,
    file: Option<PathBuf>,
    saved: std::sync::Mutex<BTreeMap<String, String>>,
    asked: std::sync::atomic::AtomicUsize,
}

impl DocumentAsker for Recorded {
    fn engine_id(&self) -> String {
        self.inner.engine_id()
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        let key = classify::question_key("check", prompt);
        if let Some(answer) = self.saved.lock().unwrap().get(&key) {
            return Ok(answer.clone());
        }
        // A provider that hit its usage limit is waited for, a few times.
        let mut attempt = 0;
        let answer = loop {
            self.asked.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            match self.inner.ask(prompt, schema) {
                Ok(answer) => break answer,
                Err(error) if attempt < 4 && is_limit(&error) => {
                    attempt += 1;
                    println!("the provider is at its limit ({error}); waiting before trying again");
                    std::thread::sleep(std::time::Duration::from_secs(240));
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

pub(crate) fn is_limit(error: &str) -> bool {
    let error = error.to_lowercase();
    ["rate limit", "usage limit", "429", "too many requests", "quota"].iter().any(|w| error.contains(w))
}

#[test]
#[ignore = "reads the real folders named in MICELYA_DOCBASE_CHECK and asks the real model"]
fn types_in_real_folders() {
    let Ok(folders) = std::env::var("MICELYA_DOCBASE_CHECK") else {
        println!("MICELYA_DOCBASE_CHECK is not set: nothing to check");
        return;
    };
    let started = Instant::now();
    let (_scratch, base) = prepare(&folders);
    println!("reading and passes 0 to 3: {:.1}s", started.elapsed().as_secs_f32());
    let (by_path, by_content) = paths_by_content(&base);
    let name = |content: i64| -> String {
        by_content.get(&content).map(|paths| paths.join(" = ")).unwrap_or_else(|| format!("#{content}"))
    };

    let file = std::env::var("MICELYA_ASK_CACHE").ok().map(PathBuf::from);
    let saved: BTreeMap<String, String> = file
        .as_ref()
        .and_then(|f| std::fs::read_to_string(f).ok())
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default();
    let asker = Recorded {
        inner: crate::source_volume::asking::asker_for(&AskConfig {
            engine: "codex-cli".into(),
            model: std::env::var("MICELYA_ASK_MODEL").unwrap_or_default(),
            reasoning: std::env::var("MICELYA_ASK_REASONING").unwrap_or_default(),
        })
        .unwrap(),
        file,
        saved: std::sync::Mutex::new(saved),
        asked: Default::default(),
    };

    let plan = base.types_plan().unwrap();
    println!("\n== plan ==\n   {plan:?}");
    let started = Instant::now();
    let mut total = RunReport::default();
    loop {
        let report = base.types_run(&asker, 100).unwrap();
        let more = report.asked > 0 || report.data_asked > 0;
        total.add(&report);
        if !more {
            break;
        }
    }
    println!(
        "passes 4 to 6: {:.1}s; questions in this run: {} to the model (answers kept from earlier runs not counted)",
        started.elapsed().as_secs_f32(),
        asker.asked.load(std::sync::atomic::Ordering::SeqCst)
    );
    println!("   {total:?}");

    // What was found, document by document.
    let types = base.document_types_all().unwrap();
    println!("\n== type and stage (pass 4) ==");
    for (content, found) in &types {
        println!(
            "   {:<26} {:<22} {:<6} {:<5} {}",
            format!("{}{}", found.doc_type, if found.proposed { " (nuevo)" } else { "" }),
            found.stage.clone().unwrap_or_default(),
            found.source,
            found.confidence.map(|c| format!("{c:.2}")).unwrap_or_default(),
            name(*content)
        );
        if let Some(reason) = &found.reason {
            println!("         {reason}");
        }
    }
    println!("\n== data (pass 5) ==");
    for (content, data) in base.document_data_all().unwrap() {
        if data.is_empty() {
            continue;
        }
        println!("   {}", name(content));
        for datum in data {
            println!("      {:<22} {:<6} {}", datum.field, datum.source, datum.value);
        }
    }

    let Some(answers) = std::env::var("MICELYA_SIMILAR_ANSWERS")
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str::<Answers>(&text).ok())
    else {
        println!("\nMICELYA_SIMILAR_ANSWERS is not set or not readable: no comparison with the answer sheet");
        return;
    };
    let content_of = |path: &str| -> Option<i64> { by_path.get(&key(&format!("{}/{}", answers.root, path))).copied() };

    println!("\n== types against the answer sheet ==");
    let (mut expected, mut type_hits, mut stage_expected, mut stage_hits, mut untyped) = (0, 0, 0, 0, 0);
    for document in &answers.documents {
        let Some(want) = &document.doc_type else { continue };
        let Some(content) = content_of(&document.path) else {
            println!("   not in the base: {}", document.path);
            continue;
        };
        expected += 1;
        let found = types.get(&content);
        let got = found.map(|f| f.doc_type.clone());
        let ok = got.as_deref() == Some(want.as_str());
        if ok {
            type_hits += 1;
        }
        if got.is_none() {
            untyped += 1;
        }
        let mut stage_note = String::new();
        if let Some(stage) = &document.stage {
            stage_expected += 1;
            let got_stage = found.and_then(|f| f.stage.clone());
            if got_stage.as_deref() == Some(stage.as_str()) {
                stage_hits += 1;
            } else {
                stage_note = format!(" · stage expected {stage}, found {}", got_stage.unwrap_or_else(|| "none".into()));
            }
        }
        if !ok || !stage_note.is_empty() {
            println!(
                "   {} expected {want}, found {}{stage_note}: {}",
                if ok { "  " } else { "XX" },
                got.unwrap_or_else(|| "nothing".into()),
                document.path
            );
        }
    }
    println!(
        "   types: {type_hits} of {expected} right ({untyped} without a type) · stages: {stage_hits} of {stage_expected} right"
    );

    // Relations: what pass 6 changed, and how the pairs stand against the sheet.
    println!("\n== relations by type (pass 6) ==");
    let corrected = base.similar_corrections().unwrap();
    for (a, b, measured, now, rule) in &corrected {
        println!("   [{rule}] {} -> {now}: {} ~ {}", measured.clone().unwrap_or_else(|| "nothing".into()), name(*a), name(*b));
    }
    let pairs: HashMap<(i64, i64), String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT a, b, relation FROM similar_pair").unwrap();
        let rows = statement
            .query_map([], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))
            .unwrap()
            .collect::<Result<HashMap<_, _>, _>>()
            .unwrap();
        rows
    };
    let changed: HashMap<(i64, i64), (Option<String>, String)> =
        corrected.iter().map(|(a, b, measured, now, _)| ((*a, *b), (measured.clone(), now.clone()))).collect();
    let (mut judged, mut right, mut wrong, mut fixed, mut broken) = (0, 0, 0, 0, 0);
    let mut lines = Vec::new();
    for pair in &answers.pairs {
        let Some(want) = engine_relation(&pair.relation) else { continue };
        let (Some(ca), Some(cb)) = (content_of(&pair.a), content_of(&pair.b)) else { continue };
        if ca == cb {
            continue;
        }
        let k = (ca.min(cb), ca.max(cb));
        let found = pairs.get(&k).cloned();
        judged += 1;
        let ok = match (want, found.as_deref()) {
            ("none", None) | ("none", Some("similar")) => true,
            (_, Some(found)) => found == want,
            _ => false,
        };
        let proposed_wrong = match (want, found.as_deref()) {
            (_, None) | (_, Some("similar")) => false,
            ("none", Some(_)) => true,
            (want, Some(found)) => found != want,
        };
        if ok {
            right += 1;
        }
        if proposed_wrong {
            wrong += 1;
            lines.push(format!(
                "   WRONG expected {want} ({}{}), found {}: {} ~ {}",
                pair.relation,
                if pair.measurable { "" } else { ", inferred" },
                found.clone().unwrap_or_default(),
                pair.a,
                pair.b
            ));
        }
        if let Some((measured, now)) = changed.get(&k) {
            let before_ok = match (want, measured.as_deref()) {
                ("none", None) | ("none", Some("similar")) => true,
                (_, Some(m)) => m == want,
                _ => false,
            };
            match (before_ok, ok) {
                (false, true) => fixed += 1,
                (true, false) => broken += 1,
                _ => {}
            }
            lines.push(format!(
                "   {} expected {want} ({}{}), measured {}, now {now}: {} ~ {}",
                if ok { "ok" } else { "XX" },
                pair.relation,
                if pair.measurable { "" } else { ", inferred" },
                measured.clone().unwrap_or_else(|| "nothing".into()),
                pair.a,
                pair.b
            ));
        } else if !ok && !pair.measurable {
            lines.push(format!(
                "   -- expected {want} ({}, inferred), found {}: {} ~ {}",
                pair.relation,
                found.unwrap_or_else(|| "nothing".into()),
                pair.a,
                pair.b
            ));
        }
    }
    for line in &lines {
        println!("{line}");
    }
    println!(
        "   pairs of the sheet: {judged}; right now {right}; wrong proposals now {wrong}; corrected by type rules: {fixed} fixed, {broken} broken"
    );
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
        "acuse-de-recibo" => "delivery-receipt-of",
        "constancia-de-firma" => "signing-certificate-of",
        "respuesta-a" => "reply-to",
        "traduccion" => "translation-of",
        "mismo-asunto" => "same-matter",
        "misma-persona-otro-asunto" => "same-person-other-matter",
        _ => return None,
    })
}
