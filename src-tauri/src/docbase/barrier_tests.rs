//! The barrier (D-719), the scans in the configuration (D-721) and the
//! thresholds in the configuration (D-722). Invented PDFs (R-24) and a
//! stand-in model that answers the question of the cut and the others.

use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::params;

use super::failures::{self, About, Failure};
use super::knowledge::extract::RereadTarget;
use super::knowledge::tests::{is_knowledge, type_answer};
use super::testing::TempDir;
use super::thresholds::Thresholds;
use super::worker::{ask_with, requested_reading_with};
use super::DocBase;
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::page_shape::test_pdf::{build, page, Page};

/// A model that looks at images: it answers the question of the cut (the
/// next page goes on the same document) and every other question.
#[derive(Default)]
struct Model {
    images: AtomicUsize,
    texts: AtomicUsize,
}

impl Model {
    fn images(&self) -> usize {
        self.images.load(Ordering::SeqCst)
    }

    fn texts(&self) -> usize {
        self.texts.load(Ordering::SeqCst)
    }
}

impl DocumentAsker for Model {
    fn engine_id(&self) -> String {
        "falso:barrera".into()
    }

    fn ask(&self, prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
        self.texts.fetch_add(1, Ordering::SeqCst);
        Ok(if is_knowledge(prompt) { serde_json::json!({ "menciones": [], "afirmaciones": [] }).to_string() } else { type_answer("contrato") })
    }

    fn sees_images(&self) -> bool {
        true
    }

    fn ask_with_images(&self, _prompt: &str, _schema: &serde_json::Value, _images: &[std::path::PathBuf]) -> Result<String, String> {
        self.images.fetch_add(1, Ordering::SeqCst);
        Ok(r#"{"second_starts_new_document": false, "first_page_type": "other", "second_page_type": "other", "confidence": "high"}"#.into())
    }
}

fn prose(words: &str) -> Page {
    page(
        &[
            &format!("Texto inventado de la prueba de la barrera, {words}, con bastantes letras"),
            "para que la extraccion lo lea como una pagina digital y no como un escaneo,",
            "sin formulas de apertura ni de cierre y sin numeros de pagina ni codigos.",
        ],
        0.0,
    )
}

fn opener() -> Page {
    page(&["Senor Juez:", "Texto inventado de otro escrito que empieza en esta pagina de la prueba."], 0.0)
}

/// Everything the thread of this computer does after the sweep, until
/// nothing is left.
fn local(base: &DocBase) {
    loop {
        let mut done = base.hash_pending(50).unwrap() + base.extract_pending(50).unwrap() + base.purge_orphans().unwrap();
        done += base.profile_pending(10).unwrap() + base.cut_pending(10).unwrap() + base.cut_parts_pending(10).unwrap();
        done += base.similar_step(10).unwrap();
        if done == 0 {
            return;
        }
    }
}

/// A base with these PDFs, all its work without AI done, the reading on.
fn settled(dir: &TempDir, files: &[(&str, Vec<Page>)]) -> (DocBase, String) {
    for (name, pages) in files {
        dir.write(&format!("o/{name}"), &build(pages));
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    local(&base);
    base.set_background_reading(true).unwrap();
    (base, origin.id)
}

/// A scan the engine of this computer is still to read.
fn add_scan(base: &DocBase, origin: &str) -> i64 {
    let conn = base.conn();
    conn.execute("INSERT INTO version (id, state, first_seen_at) VALUES (900, 'needs-recognition', 1)", []).unwrap();
    conn.execute("INSERT INTO file (hash, version_id, size) VALUES ('escaneo900', 900, 10)", []).unwrap();
    conn.execute(
        "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, hash_state, first_seen_at, last_seen_at)
         VALUES (?1, 'escaneo.png', '', 'escaneo.png', 'png', 10, 1, 'escaneo900', 'done', 1, 1)",
        params![origin],
    )
    .unwrap();
    900
}

fn turn(base: &DocBase, model: &Model) {
    ask_with(base, model, &|| {}, &|| {});
}

fn fail_scan(base: &DocBase, scan: i64, class_code: &str) {
    let mut failure = Failure::new("recognition", About::Version(scan), "prueba");
    failure.cause = failures::cause(class_code);
    failures::record(&base.conn(), &failure).unwrap();
}

#[test]
fn with_a_scan_waiting_neither_the_type_nor_the_reading_go_out_and_the_question_of_the_cut_does() {
    let dir = TempDir::new();
    let (base, origin) = settled(&dir, &[("escrito.pdf", vec![prose("primera hoja"), prose("segunda hoja"), opener()])]);
    assert_eq!(base.cut_questions().unwrap().by_itself, 1);
    assert!(!base.barrier().unwrap().holds());
    let scan = add_scan(&base, &origin);
    let barrier = base.barrier().unwrap();
    assert_eq!((barrier.scans, barrier.left()), (1, 1));

    let model = Model::default();
    turn(&base, &model);
    assert_eq!((model.images(), model.texts()), (1, 0), "only the question of the cut goes out");
    local(&base);
    for _ in 0..3 {
        turn(&base, &model);
    }
    assert_eq!(model.texts(), 0, "neither the type nor the reading while the scan waits");
    // «En curso» says why, and how much is left.
    assert_eq!(base.progress(&Default::default()).unwrap().barrier.scans, 1);

    // The scan fails, of any class: it no longer holds the barrier.
    fail_scan(&base, scan, "models-missing");
    assert!(!base.barrier().unwrap().holds());
    turn(&base, &model);
    assert!(model.texts() > 0, "the type goes out");
}

#[test]
fn what_is_marked_as_a_failure_does_not_hold_the_barrier() {
    let dir = TempDir::new();
    let (base, origin) = settled(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    let scan = add_scan(&base, &origin);
    assert!(base.barrier().unwrap().holds());
    // A transient one too, while it waits for its next attempt.
    fail_scan(&base, scan, "file-locked");
    assert_eq!(base.barrier().unwrap(), Default::default());
    // Resolved, it holds again.
    failures::resolve(&base.conn(), "recognition", &About::Version(scan)).unwrap();
    assert!(base.barrier().unwrap().holds());
}

#[test]
fn a_new_file_holds_the_whole_base_until_it_did_its_own_work_without_ai() {
    let dir = TempDir::new();
    let (base, origin) = settled(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    let model = Model::default();
    turn(&base, &model);
    assert!(model.texts() > 0, "the base already processed asks");
    let before = model.texts();

    // A new file arrives at the folder already processed.
    dir.write("o/nuevo.pdf", &build(&[prose("otra hoja nueva")]));
    base.scan_origin(&origin).unwrap();
    assert!(base.barrier().unwrap().text > 0);
    for _ in 0..3 {
        turn(&base, &model);
    }
    assert_eq!(model.texts(), before, "nothing of the whole base is asked");
    // Its text, but not its ficha nor its signature: it still holds.
    while base.hash_pending(50).unwrap() + base.extract_pending(50).unwrap() > 0 {}
    assert!(base.barrier().unwrap().holds());
    turn(&base, &model);
    assert_eq!(model.texts(), before);

    // Its work without AI done, the base goes on.
    local(&base);
    assert!(!base.barrier().unwrap().holds());
    turn(&base, &model);
    assert!(model.texts() > before);
}

#[test]
fn a_reading_a_person_asks_for_waits_for_the_barrier() {
    let dir = TempDir::new();
    let (base, origin) = settled(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    let model = Model::default();
    for _ in 0..10 {
        turn(&base, &model);
    }
    assert_eq!(base.knowledge_request_reread(&RereadTarget::default()).unwrap(), 1);
    let scan = add_scan(&base, &origin);
    let before = model.texts();
    assert!(requested_reading_with(&base, &model, &|| {}).is_none());
    assert_eq!(model.texts(), before, "it waits for the scan");
    fail_scan(&base, scan, "models-missing");
    assert!(requested_reading_with(&base, &model, &|| {}).is_some());
    assert!(model.texts() > before);
}

#[test]
fn without_the_models_of_this_computer_the_scan_does_not_go_to_the_cloud_and_waits_with_its_failure() {
    let dir = TempDir::new();
    let (base, origin) = settled(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    // A cloud engine chosen for the scans before the configuration (D-721).
    base.set_recognition_engine(Some(&crate::source_volume::recognition::RecognitionConfig { engine: "codex-cli".into(), model: "m".into() }))
        .unwrap();
    let scan = add_scan(&base, &origin);
    let model = Model::default();
    turn(&base, &model);
    let state: String = base.conn().query_row("SELECT state FROM version WHERE id = ?1", [scan], |r| r.get(0)).unwrap();
    assert_eq!(state, "needs-recognition", "the thread that asks reads no scan");

    // No way to download the models: permanent, once.
    assert_eq!(base.scans_without_models("its models are missing", false).unwrap(), 1);
    assert_eq!(base.scans_without_models("its models are missing", false).unwrap(), 0);
    let class: String = base
        .conn()
        .query_row("SELECT class FROM failure WHERE item = '900' AND state <> 'resolved'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(class, "permanent");
    assert!(!base.barrier().unwrap().holds());
    // The models arrive: the scan is read again, and holds the barrier.
    base.scans_with_models().unwrap();
    assert!(base.barrier().unwrap().holds());
    // Being downloaded: transient.
    base.scans_without_models("downloading", true).unwrap();
    let class: String = base
        .conn()
        .query_row("SELECT class FROM failure WHERE item = '900' AND state <> 'resolved'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(class, "transient");
}

#[test]
fn the_thresholds_are_read_from_the_configuration_with_their_values_by_default() {
    let base = DocBase::in_memory().unwrap();
    let by_default = base.thresholds().unwrap();
    assert_eq!(by_default, Thresholds::default());
    assert_eq!((by_default.low, by_default.high, by_default.round_cap), (0.90, 0.97, 50));
    assert_eq!((by_default.indicator_attempts, by_default.indicator_hours), (3, 6));
    assert_eq!(by_default.retry_minutes, vec![10, 60, 360, 1440]);

    let mine = Thresholds { low: 0.85, high: 0.98, quality: 0.6, indicator_attempts: 5, indicator_hours: 12, round_cap: 20, retry_minutes: vec![1, 2] };
    base.set_thresholds(&mine).unwrap();
    assert_eq!(base.thresholds().unwrap(), mine);
    assert_eq!(failures::indicator_threshold(&base.conn()).unwrap(), (5, 12));
    // The waits of the retries: the first, then the last again.
    let mut failure = Failure::new("sweep", About::Origin("o".into()), "database is locked");
    failure.cause = failures::cause("db-busy");
    let first = failures::record_at(&base.conn(), &failure, 0).unwrap();
    assert_eq!(first.next_at, Some(60_000));
    let third = {
        failures::record_at(&base.conn(), &failure, 0).unwrap();
        failures::record_at(&base.conn(), &failure, 0).unwrap()
    };
    assert_eq!(third.next_at, Some(120_000));
    // The cap of the round, as «En curso» shows it.
    assert_eq!(base.progress(&Default::default()).unwrap().auto_questions.cap, 20);

    // What cannot be a threshold is not kept.
    assert!(base.set_thresholds(&Thresholds { low: 0.98, ..mine.clone() }).is_err());
    assert!(base.set_thresholds(&Thresholds { retry_minutes: vec![], ..mine.clone() }).is_err());
    assert_eq!(base.thresholds().unwrap(), mine);
}
