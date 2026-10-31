//! The reading waits for what goes before it (D-666, P-519): the cut of a
//! PDF of several pages, its parts, the answers the model is still to give
//! about its doubtful steps and the search of similar documents. It never
//! waits for what failed, for a person, or for a file the cutter does not
//! take. Invented texts (R-24), without PDFs: the cutter's steps are
//! written as it writes them, and a stand-in model answers.

use rusqlite::params;

use super::tests::{is_knowledge, type_answer, FakeModel};
use crate::docbase::testing::TempDir;
use crate::docbase::{now_ms, DocBase, PRIVATE_DB_FILE};
use crate::source_volume::asking::DocumentAsker;

/// A page of invented words, different for each seed.
fn page_words(seed: u32) -> String {
    (0..40).map(|i| format!("palabra{seed}x{i}")).collect::<Vec<_>>().join(" ")
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

/// The steps of a file of `pages` pages as the cutter leaves them, with
/// the rules in use and the text its pages have: `new` before the pages
/// in `starts`, `doubtful` (question `q<page>`) before those in
/// `doubtful`, `continue` before the others.
fn cut(base: &DocBase, hash: &str, pages: u32, starts: &[u32], doubtful: &[u32]) {
    let conn = base.conn();
    let stamp: i64 = conn.query_row("SELECT max(created_at) FROM resource_text WHERE resource_hash = ?1", [hash], |r| r.get(0)).unwrap();
    for after in 0..pages {
        let next = after + 1;
        let verdict = if after == 0 || starts.contains(&next) {
            "new"
        } else if doubtful.contains(&next) {
            "doubtful"
        } else {
            "continue"
        };
        conn.execute(
            "INSERT INTO cut_step (resource_hash, after_page, verdict, rule, question, rules_version, text_stamp, created_at)
             VALUES (?1, ?2, ?3, 'r', ?4, ?5, ?6, ?7)",
            params![hash, after, verdict, (verdict == "doubtful").then(|| format!("q{next}")), crate::docbase::cut::rules_version(), stamp, now_ms()],
        )
        .unwrap();
    }
}

/// The model of these tests: a type for every document, and a reading
/// with nothing in it.
fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_knowledge(prompt) {
            serde_json::json!({ "menciones": [], "afirmaciones": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

/// The same model, with an engine that looks at images: the one that
/// asks by itself about the doubtful steps of the cuts.
struct Seeing(FakeModel);

impl DocumentAsker for Seeing {
    fn engine_id(&self) -> String {
        self.0.engine_id()
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        self.0.ask(prompt, schema)
    }

    fn sees_images(&self) -> bool {
        true
    }
}

/// One turn of the thread that asks the model: the types, then the reading.
fn ask_turn(base: &DocBase, asker: &dyn DocumentAsker) {
    base.types_run(asker, 50).unwrap();
    base.knowledge_run(asker, 50).unwrap();
}

/// The questions of the reading the model was asked.
fn readings(model: &FakeModel) -> Vec<String> {
    model.asked.lock().unwrap().iter().filter(|q| is_knowledge(q)).cloned().collect()
}

/// The thread of this computer, after the text: the documents of the cut
/// files and the search of similar documents.
fn local_after_cut(base: &DocBase) {
    while base.cut_parts_pending(10).unwrap() > 0 {}
    while base.similar_step(10).unwrap() > 0 {}
}

#[test]
fn the_two_threads_taking_turns_never_read_the_whole_file() {
    let (_dir, base) = open();
    let model = model();
    // The thread of this computer reads the text of a PDF of three pages,
    // which join two documents (pages 1-2 and 3).
    add_file(&base, 1, "h1", "pdf", &[1, 2, 3]);
    // The thread that asks takes its turn before the cut: nothing to read.
    ask_turn(&base, &model);
    assert!(readings(&model).is_empty());
    // Cut, without its documents yet: still nothing.
    cut(&base, "h1", 3, &[3], &[]);
    ask_turn(&base, &model);
    assert!(readings(&model).is_empty());
    // Its documents, without the search of similar documents: still nothing.
    while base.cut_parts_pending(10).unwrap() > 0 {}
    ask_turn(&base, &model);
    assert!(readings(&model).is_empty());
    // The search of similar documents: each document is read, never the
    // whole file.
    while base.similar_step(10).unwrap() > 0 {}
    ask_turn(&base, &model);
    let asked = readings(&model);
    assert!(asked.iter().any(|p| has(p, 1) && has(p, 2)), "the first document is read");
    assert!(asked.iter().any(|p| has(p, 3)), "the second document is read");
    assert!(!asked.iter().any(|p| has(p, 1) && has(p, 3)), "the whole file is never read");
}

#[test]
fn a_pdf_of_one_page_and_a_file_that_is_not_a_pdf_are_read_without_waiting_for_a_cut() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "h1", "pdf", &[1]);
    // Two pages of text, but not a PDF: the cutter does not take it.
    add_file(&base, 2, "h2", "docx", &[2, 3]);
    assert!(base.waiting_for_cut().unwrap().is_empty());
    while base.similar_step(10).unwrap() > 0 {}
    ask_turn(&base, &model);
    let asked = readings(&model);
    assert!(asked.iter().any(|p| has(p, 1)) && asked.iter().any(|p| has(p, 2) && has(p, 3)));
    let steps: i64 = base.conn().query_row("SELECT count(*) FROM cut_step", [], |r| r.get(0)).unwrap();
    assert_eq!(steps, 0);
}

#[test]
fn a_cut_that_fails_does_not_hold_the_reading_back() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "h1", "pdf", &[1, 2, 3]);
    {
        let conn = base.conn();
        // Its ficha of three pages, and the text of a page the cutter cannot
        // take: its cut fails.
        conn.execute(
            "INSERT INTO derivative (version_id, kind, engine, body, created_at) VALUES (1, ?1, ?2, '{\"pages\":[{},{},{}]}', 6)",
            params![crate::docbase::page_shapes::SHAPES_KIND, crate::source_volume::page_shape::SHAPES_ENGINE],
        )
        .unwrap();
        conn.execute("INSERT INTO resource_text (resource_hash, page_no, engine, body, created_at) VALUES ('h1', -1, 'extraction', 'x', 5)", [])
            .unwrap();
    }
    while base.similar_step(10).unwrap() > 0 {}
    // Signed from before, the whole file is not typed nor read: it waits.
    ask_turn(&base, &model);
    assert!(model.asked.lock().unwrap().is_empty());
    // The cut fails: written down, and not tried again with the same text
    // and rules.
    assert_eq!(base.cut_pending(10).unwrap(), 1);
    assert_eq!(base.cut_pending(10).unwrap(), 0);
    assert!(base.waiting_for_cut().unwrap().is_empty());
    // The file is read as before the cut existed: whole.
    ask_turn(&base, &model);
    assert!(readings(&model).iter().any(|p| has(p, 1) && has(p, 3)));
}

#[test]
fn the_reading_waits_for_the_signature_of_the_search_of_similar_documents() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "h1", "docx", &[1, 2]);
    // Without its signature it is in no unit to type, and it is not read.
    let report = base.knowledge_run(&model, 50).unwrap();
    assert_eq!((report.asked, report.waiting_for_type), (0, 1));
    assert_eq!(base.waiting_for_similar().unwrap().into_iter().collect::<Vec<_>>(), vec![1]);
    while base.similar_step(10).unwrap() > 0 {}
    assert!(base.waiting_for_similar().unwrap().is_empty());
    ask_turn(&base, &model);
    assert!(readings(&model).iter().any(|p| has(p, 1) && has(p, 2)));
}

#[test]
fn a_whole_file_signed_before_its_cut_is_not_typed_whole() {
    let (_dir, base) = open();
    let model = model();
    add_file(&base, 1, "h1", "pdf", &[1, 2, 3]);
    // Signed before its cut (a cut that came back: another text of its
    // pages, other rules): its type is not asked as a whole.
    while base.similar_step(10).unwrap() > 0 {}
    assert_eq!(base.types_run(&model, 50).unwrap().asked, 0);
    cut(&base, "h1", 3, &[3], &[]);
    local_after_cut(&base);
    // Its two documents are typed, each one.
    assert_eq!(base.types_run(&model, 50).unwrap().asked, 2);
}

#[test]
fn the_parts_wait_for_the_answers_the_model_will_give_about_the_cut_and_never_for_a_person() {
    let (_dir, base) = open();
    let seeing = Seeing(model());
    add_file(&base, 1, "h1", "pdf", &[1, 2, 3]);
    // A rule says page 3 starts another document; whether page 2 does is
    // doubtful, and the thread asks it by itself while the reading is on.
    cut(&base, "h1", 3, &[3], &[2]);
    local_after_cut(&base);
    base.set_background_reading(true).unwrap();
    ask_turn(&base, &seeing);
    assert!(readings(&seeing.0).is_empty());
    // Nothing waits when that answer does not come by itself: an engine
    // that does not look at images, the round waiting for a person, the
    // questions about the cuts failing, the reading off.
    assert!(base.waiting_for_cut_answers(false).unwrap().is_empty());
    base.engine_state().auto_held(Some("cut"));
    assert!(base.waiting_for_cut_answers(true).unwrap().is_empty());
    base.engine_state().auto_held(None);
    base.engine_state().cut_asking_failed(true);
    assert!(base.waiting_for_cut_answers(true).unwrap().is_empty());
    base.engine_state().cut_asking_failed(false);
    base.set_background_reading(false).unwrap();
    assert!(base.waiting_for_cut_answers(true).unwrap().is_empty());
    base.set_background_reading(true).unwrap();
    assert!(!base.waiting_for_cut_answers(true).unwrap().is_empty());
    // The answer: page 2 starts another document. The parts are made
    // again, and each one is read once.
    base.conn().execute("UPDATE cut_step SET model_says = 1, model_confidence = 'high' WHERE question = 'q2'", []).unwrap();
    local_after_cut(&base);
    assert!(base.waiting_for_cut_answers(true).unwrap().is_empty());
    ask_turn(&base, &seeing);
    let asked = readings(&seeing.0);
    for seed in 1..=3 {
        assert!(asked.iter().any(|p| has(p, seed)), "page {seed} is read");
    }
    assert!(!asked.iter().any(|p| has(p, 1) && has(p, 2)), "the part that the answer split is never read");
}
