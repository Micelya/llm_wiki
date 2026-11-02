//! Two levels, file and document (D-718): every file has a structure of 1
//! to n documents, written the same way whatever n is; a file of one
//! document is a file and a document; the same rule of exact copy holds
//! between files (the same print) and between documents (the same text);
//! a «documento compuesto» is only a file of two or more, and a whole file,
//! as such, never goes to the steps of documents. Invented texts (R-24),
//! without PDFs: the cutter's steps are written as it writes them, and a
//! stand-in model answers.

use rusqlite::params;

use super::tests::{is_knowledge, mention, type_answer, FakeModel};
use crate::docbase::similar::classify::name_of;
use crate::docbase::testing::TempDir;
use crate::docbase::{now_ms, DocBase, PRIVATE_DB_FILE};

/// What the contract of these tests says on its first page.
const PARTY: &str = "Persona Seis, DNI 66.666.666, en adelante la cedente";

/// A page of invented words, different for each seed; the contract's first
/// page (seed 50) names its party.
fn page_words(seed: u32) -> String {
    let words = (0..40).map(|i| format!("palabra{seed}x{i}")).collect::<Vec<_>>().join(" ");
    if seed == 50 {
        format!("{words}\n{PARTY}.")
    } else {
        words
    }
}

/// Whether a question carries the page of this seed.
fn has(prompt: &str, seed: u32) -> bool {
    prompt.contains(&format!("palabra{seed}x5 "))
}

fn open() -> (TempDir, DocBase) {
    let dir = TempDir::new();
    let base = DocBase::open(&dir.path().join(PRIVATE_DB_FILE)).unwrap();
    base.conn()
        .execute_batch("INSERT INTO origin (id, name, provider, location, added_at, reachable) VALUES ('o', 'Origen', 'local', 'C:/nada', 1, 1);")
        .unwrap();
    (dir, base)
}

/// A file of the origin with this extension, one page per seed, with its
/// text and the text of each page as the extraction leaves them.
fn add_file(base: &DocBase, id: i64, hash: &str, ext: &str, seeds: &[u32]) {
    let text = seeds.iter().enumerate().map(|(i, s)| format!("## Page {}\n\n{}", i + 1, page_words(*s))).collect::<Vec<_>>().join("\n\n");
    let mut conn = base.conn();
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO version (id, state, first_seen_at) VALUES (?1, 'pending', 1)", [id]).unwrap();
    tx.execute("INSERT INTO file (hash, version_id, size) VALUES (?1, ?2, 10)", params![hash, id]).unwrap();
    tx.execute(
        "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, hash_state, first_seen_at, last_seen_at)
         VALUES ('o', ?1, '', ?1, ?2, 10, 1, ?3, 'done', 1, 1)",
        params![format!("{hash}.{ext}"), ext, hash],
    )
    .unwrap();
    crate::docbase::text::store(&tx, id, &text, "extraction", "processed").unwrap();
    for (i, seed) in seeds.iter().enumerate() {
        tx.execute(
            "INSERT INTO resource_text (resource_hash, page_no, engine, body, created_at) VALUES (?1, ?2, ?3, ?4, 5)",
            params![hash, i as i64 + 1, crate::docbase::text::EXTRACTION_ENGINE, page_words(*seed)],
        )
        .unwrap();
    }
    tx.commit().unwrap();
}

/// The same bytes at another place, found later: an exact copy of a file.
fn add_place(base: &DocBase, hash: &str, rel_path: &str) {
    base.conn()
        .execute(
            "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, hash_state, first_seen_at, last_seen_at)
             VALUES ('o', ?1, '', ?1, 'pdf', 10, 1, ?2, 'done', 2, 2)",
            params![rel_path, hash],
        )
        .unwrap();
}

/// The steps of a file of `pages` pages as the cutter leaves them, with
/// the rules in use and the text its pages have: `new` before the pages
/// in `starts`, `continue` before the others.
fn cut(base: &DocBase, hash: &str, pages: u32, starts: &[u32]) {
    let conn = base.conn();
    let stamp: i64 = conn.query_row("SELECT max(created_at) FROM resource_text WHERE resource_hash = ?1", [hash], |r| r.get(0)).unwrap();
    for after in 0..pages {
        let next = after + 1;
        let verdict = if after == 0 || starts.contains(&next) { "new" } else { "continue" };
        conn.execute(
            "INSERT INTO cut_step (resource_hash, after_page, verdict, rule, rules_version, text_stamp, created_at)
             VALUES (?1, ?2, ?3, 'r', ?4, ?5, ?6)",
            params![hash, after, verdict, crate::docbase::cut::rules_version(), stamp, now_ms()],
        )
        .unwrap();
    }
}

/// The model of these tests: a type for every document, and a reading
/// that finds the party of the contract where the text names it.
fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let mentions: Vec<serde_json::Value> = if prompt.contains(PARTY) {
            vec![mention("a", "persona", "Persona Seis", "parte", PARTY, serde_json::json!([]))]
        } else {
            Vec::new()
        };
        serde_json::json!({ "menciones": mentions, "afirmaciones": [] }).to_string()
    })
}

/// The work without the model, then one turn of the thread that asks it.
fn settle(base: &DocBase, asker: &FakeModel) {
    while base.cut_parts_pending(10).unwrap() > 0 {}
    while base.similar_step(10).unwrap() > 0 {}
    base.types_run(asker, 50).unwrap();
    base.knowledge_run(asker, 50).unwrap();
}

fn count(base: &DocBase, sql: &str) -> i64 {
    base.conn().query_row(sql, [], |r| r.get(0)).unwrap()
}

fn texts(base: &DocBase, sql: &str) -> Vec<String> {
    let conn = base.conn();
    let mut statement = conn.prepare(sql).unwrap();
    let rows = statement.query_map([], |r| r.get(0)).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

/// The structure of a file: the version and the place of each document.
fn structure(base: &DocBase, hash: &str) -> Vec<(i64, String)> {
    let conn = base.conn();
    let mut statement = conn
        .prepare("SELECT version_id, selector FROM appearance WHERE resource_hash = ?1 AND selector <> 'file' ORDER BY position, id")
        .unwrap();
    let rows = statement.query_map([hash], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn version_of(base: &DocBase, hash: &str) -> i64 {
    count(base, &format!("SELECT version_id FROM file WHERE hash = '{hash}'"))
}

/// The questions about the pages of this seed.
fn asked_about(model: &FakeModel, seed: u32) -> usize {
    model.asked.lock().unwrap().iter().filter(|q| has(q, seed)).count()
}

#[test]
fn a_file_of_one_document_has_a_structure_of_one_part_with_its_version() {
    let (_dir, base) = open();
    let model = model();
    // A Word, a PDF of one page and a PDF of three pages the cutter cuts
    // into one document: each one is a file and a document.
    add_file(&base, 1, "word", "docx", &[1, 2]);
    add_file(&base, 2, "una", "pdf", &[3]);
    add_file(&base, 3, "tres", "pdf", &[4, 5, 6]);
    cut(&base, "tres", 3, &[]);
    settle(&base, &model);
    for hash in ["word", "una", "tres"] {
        assert_eq!(structure(&base, hash), vec![(version_of(&base, hash), "all".to_string())], "{hash}");
        assert!(base.included_parts(hash).unwrap().is_empty(), "{hash} is not a «documento compuesto»");
    }
    // Each document is typed and read once, as what it is.
    assert_eq!(count(&base, "SELECT count(*) FROM document_type"), 3);
    assert_eq!(count(&base, "SELECT count(*) FROM knowledge_extraction WHERE state = 'done'"), 3);
    assert_eq!(asked_about(&model, 4), 2, "the type and the reading of the PDF of one document");
}

#[test]
fn a_cut_file_has_a_structure_of_n_parts_and_is_a_composite_only_with_two_or_more() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "h1", "pdf", &[1, 2, 3]);
    let whole = version_of(&base, "h1");
    // Before its cut: the one document the file is born with, which waits.
    assert_eq!(structure(&base, "h1"), vec![(whole, "all".to_string())]);
    cut(&base, "h1", 3, &[3]);
    settle(&base, &model);
    let places: Vec<String> = structure(&base, "h1").into_iter().map(|(_, s)| s).collect();
    assert_eq!(places, vec!["pages:1-2", "pages:3-3"]);
    assert_eq!(base.included_parts("h1").unwrap().len(), 2);
    // The whole file is only structure: no signature, no type, no reading,
    // no question with its whole text.
    for table in ["similar_signature", "document_type", "knowledge_extraction"] {
        assert_eq!(count(&base, &format!("SELECT count(*) FROM {table} WHERE version_id = {whole}")), 0, "{table}");
    }
    assert!(!model.asked.lock().unwrap().iter().any(|q| has(q, 1) && has(q, 3)), "the whole file is never asked about");
    assert_eq!(count(&base, "SELECT count(*) FROM document_type"), 2);
    // A person joins the two: one document again, with the version of the
    // file; it is no longer a composite.
    base.cut_join_with_previous("h1", 3).unwrap();
    while base.cut_parts_pending(10).unwrap() > 0 {}
    assert_eq!(structure(&base, "h1"), vec![(whole, "all".to_string())]);
    assert!(base.included_parts("h1").unwrap().is_empty());
}

#[test]
fn a_whole_file_without_its_structure_is_neither_typed_nor_read() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "suelto", "docx", &[1]);
    add_file(&base, 2, "otro", "docx", &[2]);
    base.conn().execute("DELETE FROM appearance WHERE resource_hash = 'otro' AND selector = 'all'", []).unwrap();
    settle(&base, &model);
    assert_eq!(asked_about(&model, 1), 2, "the document is typed and read");
    assert_eq!(asked_about(&model, 2), 0, "the file without a document is not");
    assert_eq!(count(&base, "SELECT count(*) FROM similar_signature WHERE version_id = 2"), 0);
}

#[test]
fn an_exact_copy_of_a_file_found_later_repeats_nothing_and_keeps_its_name() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "contrato", "pdf", &[50, 51]);
    cut(&base, "contrato", 2, &[]);
    settle(&base, &model);
    let asked = model.asked.lock().unwrap().len();
    let name = name_of(&base.conn(), 1).unwrap();
    assert_eq!(name, "contrato.pdf");
    // The same bytes at another place, whose name comes first.
    add_place(&base, "contrato", "a-copia.pdf");
    settle(&base, &model);
    assert_eq!(model.asked.lock().unwrap().len(), asked, "nothing is asked again");
    assert_eq!(name_of(&base.conn(), 1).unwrap(), name, "the copy does not change the name, nor the questions (P-517)");
    assert_eq!(count(&base, "SELECT count(*) FROM version"), 1);
}

/// The contract alone (a file of one document) and the same contract as
/// the annex of an agreement (a part of a composite), in this order or in
/// the other: the type, the entities and the reading of the contract, and
/// the questions about it.
fn contract_and_annex(contract_first: bool) -> (String, Vec<String>, i64, usize) {
    let (_dir, base) = open();
    let model = model();
    let contract = |base: &DocBase| {
        add_file(base, 1, "contrato", "pdf", &[50, 51]);
        cut(base, "contrato", 2, &[]);
    };
    let agreement = |base: &DocBase| {
        add_file(base, 2, "acuerdo", "pdf", &[60, 61, 50, 51]);
        cut(base, "acuerdo", 4, &[3]);
    };
    if contract_first {
        contract(&base);
        settle(&base, &model);
        agreement(&base);
    } else {
        agreement(&base);
        settle(&base, &model);
        let annex = structure(&base, "acuerdo")[1].0;
        let name = name_of(&base.conn(), annex).unwrap();
        let asked = model.asked.lock().unwrap().len();
        contract(&base);
        settle(&base, &model);
        assert_eq!(model.asked.lock().unwrap().len(), asked, "the contract that comes later repeats nothing (P-517)");
        assert_eq!(name_of(&base.conn(), annex).unwrap(), name, "nor changes its name");
    }
    settle(&base, &model);
    // One document, seen from both files.
    let alone = structure(&base, "contrato");
    let annex = structure(&base, "acuerdo");
    assert_eq!(alone.len(), 1);
    assert_eq!(annex.len(), 2);
    assert_eq!(alone[0].0, annex[1].0, "the annex is an exact copy: the same version");
    let version = alone[0].0;
    let doc_type = texts(&base, &format!("SELECT doc_type FROM document_type WHERE version_id = {version}")).concat();
    let entities = texts(
        &base,
        &format!(
            "SELECT DISTINCT m.written FROM knowledge_mention m
             WHERE m.version_id = {version} ORDER BY 1"
        ),
    );
    let readings = count(&base, &format!("SELECT count(*) FROM knowledge_extraction WHERE version_id = {version} AND state = 'done'"));
    (doc_type, entities, readings, asked_about(&model, 50))
}

#[test]
fn the_annex_that_is_a_copy_of_the_contract_alone_gives_the_same_from_either_side() {
    let first = contract_and_annex(true);
    let later = contract_and_annex(false);
    assert_eq!(first.0, "contrato");
    assert!(first.1.iter().any(|e| e.contains("Seis")), "{:?}", first.1);
    assert_eq!(first.2, 1, "read once");
    // Its type and its reading, once each: never again for the copy.
    assert_eq!(first.3, 2);
    assert_eq!(first, later);
}

/// Opens a copy of a base with this version (`MICELYA_LEVELS_COPY`), for
/// the check of the stage 2a of 1 nov: the migration runs, then the work
/// without the model (the parts, the search of similar documents and the
/// types from the answers kept). Asks nothing: the asker it is given fails.
#[test]
#[ignore]
fn open_a_copy_for_the_two_levels() {
    let Ok(path) = std::env::var("MICELYA_LEVELS_COPY") else { return };
    let base = DocBase::open(std::path::Path::new(&path)).unwrap();
    while base.cut_parts_pending(64).unwrap() > 0 {}
    while base.similar_step(64).unwrap() > 0 {}
    let refuses = FakeModel::new("nadie:0", |_| panic!("no question is asked"));
    let types = base.types_run(&refuses, 0).unwrap();
    let standings = super::extract::standings(&base.conn(), &crate::docbase::Visible::All, None).unwrap();
    let to_read = standings.iter().filter(|s| !s.reading.as_ref().is_some_and(|r| r.current)).count();
    println!(
        "documents to read: {}; of them not read now: {to_read}; types written from the answers kept: {}; type questions left: {}; asked: {}",
        standings.len(),
        types.typed,
        types.left,
        refuses.asked.lock().unwrap().len()
    );
}

/// The page of a file of several documents lists each one with its pages
/// and its type, from what the base already has (P-591, etapa P): the link
/// «archivo, pág. N» of a person leads to something useful, without a
/// page written by the model for each part.
#[test]
fn the_page_of_a_cut_file_lists_its_documents_with_their_pages_and_types() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "h1", "pdf", &[50, 2, 3]);
    let whole = version_of(&base, "h1");
    cut(&base, "h1", 3, &[3]);
    settle(&base, &model);
    base.knowledge_resolve(Some(&model), 50).unwrap();
    base.knowledge_project(Some(&model), 50).unwrap();
    let page = base.knowledge_document_article(whole).unwrap().expect("the file has its page");
    assert!(page.body.contains("## Los documentos de este archivo"), "{}", page.body);
    assert!(page.body.contains("- **Págs. 1–2** · Contrato"), "{}", page.body);
    assert!(page.body.contains("- **Pág. 3** · Contrato"), "{}", page.body);
    assert!(!page.body.contains("todavía no se leyó"), "{}", page.body);
}
