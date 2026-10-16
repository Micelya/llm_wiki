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
    let path = std::env::var("MICELYA_SIMILAR_ANSWERS").ok()?;
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
    let Ok(folders) = std::env::var("MICELYA_DOCBASE_CHECK") else {
        println!("MICELYA_DOCBASE_CHECK is not set: nothing to check");
        return;
    };
    let (_scratch, base) = run(&folders);
    let (by_path, by_content) = paths_by_content(&base);
    let name = |content: i64| -> String {
        by_content
            .get(&content)
            .map(|paths| paths.join(" = "))
            .unwrap_or_else(|| format!("#{content}"))
    };

    // What there is.
    let count = |sql: &str| -> i64 { base.conn().query_row(sql, [], |row| row.get(0)).unwrap() };
    println!("\ndocuments: {}", count("SELECT count(*) FROM content"));
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
                "SELECT c.id, c.state FROM content c
                 WHERE c.id NOT IN (SELECT content_id FROM similar_signature WHERE length(shingles) > 0) ORDER BY c.state, c.id",
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
            .prepare("SELECT content_id, marks, signatures, skip_pages FROM document_marks ORDER BY content_id")
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
                println!("      embedded signature: {}", signature.names.join(" | "));
            }
            if skip != "[]" {
                println!("      pages left out of the comparison (from 0): {skip}");
                // To write the catalog's signals: what those pages say.
                if std::env::var("MICELYA_SIMILAR_SHOW_PAGES").is_ok() {
                    let body: String = base
                        .conn()
                        .query_row("SELECT body FROM derivative WHERE content_id = ?1 AND kind = 'text'", [content], |row| row.get(0))
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
        println!("\nMICELYA_SIMILAR_ANSWERS is not set or not readable: no comparison with the answer sheet");
        return;
    };
    let proposed: HashMap<(i64, i64), (String, Option<i64>)> =
        pairs.iter().map(|p| ((p.0, p.1), (p.2.clone(), p.3))).collect();
    let content_of = |path: &str| -> Option<i64> { by_path.get(&key(&format!("{}/{}", answers.root, path))).copied() };

    #[derive(Default)]
    struct Tally {
        expected: usize,
        hits: usize,
        wrong: usize,
        missed: usize,
        as_similar: usize,
    }
    let mut tally: BTreeMap<String, Tally> = BTreeMap::new();
    let mut wrong: Vec<String> = Vec::new();
    let mut missed: Vec<String> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    let mut judged: BTreeSet<(i64, i64)> = BTreeSet::new();
    // Members of a "same template" family, pairwise: the sheet writes
    // them against the principal only.
    let mut template_family: HashMap<i64, i64> = HashMap::new();

    for answer in answers.pairs.iter().filter(|p| p.measurable) {
        let Some(expected) = engine_relation(&answer.relation) else {
            unknown.push(format!("relation '{}' ({} ~ {})", answer.relation, answer.a, answer.b));
            continue;
        };
        let (Some(ca), Some(cb)) = (content_of(&answer.a), content_of(&answer.b)) else {
            unknown.push(format!("not in the base: {} ~ {}", answer.a, answer.b));
            continue;
        };
        if expected == "same-template" {
            let family = *template_family.get(&ca).or(template_family.get(&cb)).unwrap_or(&ca.min(cb));
            template_family.insert(ca, family);
            template_family.insert(cb, family);
        }
        let entry = tally.entry(expected.to_string()).or_default();
        entry.expected += 1;
        let (lo, hi) = (ca.min(cb), ca.max(cb));
        judged.insert((lo, hi));
        let found = if ca == cb {
            Some("same-document".to_string())
        } else {
            proposed.get(&(lo, hi)).map(|(relation, _)| relation.clone())
        };
        let line = format!(
            "expected {} ({}), proposed {}: {} ~ {}{}",
            expected,
            answer.relation,
            found.clone().unwrap_or_else(|| "nothing".into()),
            answer.a,
            answer.b,
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
                wrong.push(line);
            }
            (_, Some(found)) if found == expected => entry.hits += 1,
            (_, None) => {
                entry.missed += 1;
                missed.push(line);
            }
            (_, Some("similar")) => {
                entry.missed += 1;
                entry.as_similar += 1;
                missed.push(line);
            }
            (_, Some(_)) => {
                entry.wrong += 1;
                wrong.push(line);
            }
        }
    }
    // Proposals the sheet does not judge directly.
    let mut inferred = Tally::default();
    let mut unjudged: Vec<String> = Vec::new();
    for ((a, b), (relation, _)) in &proposed {
        if judged.contains(&(*a, *b)) || relation == "similar" {
            continue;
        }
        match (template_family.get(a), template_family.get(b)) {
            (Some(x), Some(y)) if x == y => {
                inferred.expected += 1;
                if relation == "same-template" {
                    inferred.hits += 1;
                } else {
                    inferred.wrong += 1;
                    wrong.push(format!(
                        "expected same-template (same family in the sheet), proposed {relation}: {} ~ {}",
                        name(*a),
                        name(*b)
                    ));
                }
            }
            _ => unjudged.push(format!("[{relation}] {} ~ {}", name(*a), name(*b))),
        }
    }

    println!("\n== against the answer sheet (measurable pairs) ==");
    println!("   {:<16} {:>8} {:>6} {:>6} {:>7} {:>11}", "relation", "expected", "hits", "WRONG", "missed", "as similar");
    let mut total = Tally::default();
    for (relation, t) in &tally {
        println!("   {:<16} {:>8} {:>6} {:>6} {:>7} {:>11}", relation, t.expected, t.hits, t.wrong, t.missed, t.as_similar);
        total.expected += t.expected;
        total.hits += t.hits;
        total.wrong += t.wrong;
        total.missed += t.missed;
        total.as_similar += t.as_similar;
    }
    println!(
        "   {:<16} {:>8} {:>6} {:>6} {:>7} {:>11}",
        "total", total.expected, total.hits, total.wrong, total.missed, total.as_similar
    );
    println!(
        "   inside a same-template family of the sheet: {} pairs, {} right, {} wrong",
        inferred.expected, inferred.hits, inferred.wrong
    );
    println!("\n   WRONG proposals (the worst outcome, R-07): {}", wrong.len());
    for line in &wrong {
        println!("      {line}");
    }
    println!("\n   missed relations: {}", missed.len());
    for line in &missed {
        println!("      {line}");
    }
    println!("\n   proposals the sheet does not judge: {}", unjudged.len());
    for line in &unjudged {
        println!("      {line}");
    }
    if !unknown.is_empty() {
        println!("\n   answers that could not be used: {}", unknown.len());
        for line in &unknown {
            println!("      {line}");
        }
    }
}

/// Words that say nothing about a case even though they appear in the
/// names of the test files: generic words of documents, in the two
/// languages of the test folder.
const GENERIC: &[&str] = &[
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
    let mut generic: BTreeSet<String> = GENERIC.iter().map(|w| w.to_string()).collect();
    for doc in ["reglas-de-procesamiento.md", "catalogo-de-tipos.md", "parecidos-propuesta.md", "conocimiento-propuesta.md"] {
        let text = std::fs::read_to_string(manifest.join("../docs/design").join(doc)).unwrap_or_default();
        generic.extend(tokens(&text));
    }
    for issuer in catalog.issuers() {
        generic.extend(tokens(&issuer));
    }
    generic
}

fn case_terms(
    answers: &Answers,
    entities: Option<&crate::docbase::knowledge::check::Sheet>,
    catalog: &Catalog,
) -> BTreeMap<String, String> {
    let generic = generic_words(catalog);
    let mut names: Vec<String> = Vec::new();
    for pair in &answers.pairs {
        names.push(pair.a.clone());
        names.push(pair.b.clone());
    }
    names.extend(answers.documents.iter().map(|d| d.path.clone()));
    // Of an identifier, only its numbers: its words ("number") are generic.
    for entity in entities.map(|s| s.entities.as_slice()).unwrap_or_default() {
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
fn entity_phrases(entities: &crate::docbase::knowledge::check::Sheet, catalog: &Catalog) -> BTreeMap<String, String> {
    let generic = generic_words(catalog);
    let mut phrases = BTreeMap::new();
    for entity in entities.entities.iter().filter(|e| e.kind != "asunto") {
        let names = std::iter::once(&entity.name).chain(&entity.aliases).chain(entity.mentions.iter().map(|m| &m.as_written));
        for name in names {
            let words = tokens(name);
            if words.len() >= 2 && words.iter().any(|w| w.len() >= 3 && !generic.contains(w)) {
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
            .is_some_and(|e| matches!(e, "rs" | "ts" | "tsx" | "json"));
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
    let entities = crate::docbase::knowledge::check::sheet();
    if entities.is_none() {
        println!("MICELYA_ENTITY_ANSWERS is not set or not readable: the entity sheet is not checked");
    }
    let terms = case_terms(&answers, entities.as_ref(), &catalog);
    let phrases = entities.as_ref().map(|e| entity_phrases(e, &catalog)).unwrap_or_default();
    println!(
        "case terms looked for: {}: {}",
        terms.len(),
        terms.keys().cloned().collect::<Vec<_>>().join(", ")
    );

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    files_under(&manifest.join("src/docbase"), &mut files);
    files_under(&manifest.join("../src/components/docbase"), &mut files);
    files_under(&manifest.join("../src/vocabulary"), &mut files);
    files.push(manifest.join("resources/catalog/document-types.json"));
    files.push(manifest.join("resources/catalog/knowledge.json"));

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
