//! The loop of the process (E4 of 2 nov, in place of A2): the tests of A2,
//! the six of the barrier redone, and those of E4. Invented PDFs (R-24) and
//! stand-in models: no answer of any run, no real model.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use rusqlite::params;
use serde_json::json;

use super::super::failures::{self, About as FailureAbout, Failure};
use super::super::knowledge::extract::RereadTarget;
use super::super::knowledge::tests::{is_knowledge, type_answer};
use super::super::testing::TempDir;
use super::super::thresholds::Thresholds;
use super::super::DocBase;
use super::run::{Ask, Turn, Wake, BACK_LIMIT};
use super::{Class, Scope, Step};
use crate::source_volume::asking::{AskConfig, DocumentAsker};
use crate::source_volume::page_shape::test_pdf::{build, page, Page};

/// A model that answers the question of the cut (the next page goes on the
/// same document), the windows of the cut by zones (a document starts at
/// the third page), the type (a contract) and the reading (nothing). It may
/// answer the type with another shape, or fail it, and it may drop a new
/// file in the folder while it answers.
#[derive(Default)]
struct Model {
    images: AtomicUsize,
    texts: AtomicUsize,
    /// The answer to the type questions instead of a contract: `Err` fails.
    types: Option<Result<&'static str, &'static str>>,
    /// A file written while the model answers (a new file mid-operation).
    drops: Mutex<Option<(std::path::PathBuf, Vec<u8>)>>,
}

impl Model {
    fn asked(&self) -> usize {
        self.images.load(Ordering::SeqCst) + self.texts.load(Ordering::SeqCst)
    }
}

impl DocumentAsker for Model {
    fn engine_id(&self) -> String {
        "falso:proceso".into()
    }

    fn ask(&self, prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
        self.texts.fetch_add(1, Ordering::SeqCst);
        if let Some((path, bytes)) = self.drops.lock().unwrap().take() {
            std::fs::write(path, bytes).unwrap();
        }
        if is_knowledge(prompt) {
            return Ok(json!({ "menciones": [], "afirmaciones": [] }).to_string());
        }
        match self.types {
            Some(Ok(answer)) => Ok(answer.to_string()),
            Some(Err(error)) => Err(error.to_string()),
            None => Ok(type_answer("contrato")),
        }
    }

    fn sees_images(&self) -> bool {
        true
    }

    fn ask_with_images(&self, prompt: &str, _schema: &serde_json::Value, images: &[std::path::PathBuf]) -> Result<String, String> {
        self.images.fetch_add(1, Ordering::SeqCst);
        if prompt.contains("=== Pistas de los saltos ===") {
            let pages: Vec<_> = (1..=images.len() as u32)
                .map(|p| json!({"pagina": p, "empieza_documento": p == 1 || p == 3, "seguridad": "alta"}))
                .collect();
            return Ok(json!({"paginas": pages, "documentos": [{"primera_pagina": 1, "tipo": "other", "seguridad": "alta"}]}).to_string());
        }
        Ok(r#"{"second_starts_new_document": false, "first_page_type": "other", "second_page_type": "other", "confidence": "high"}"#.into())
    }
}

fn prose(words: &str) -> Page {
    page(
        &[
            &format!("Texto inventado de la prueba del bucle, {words}, con bastantes letras"),
            "para que la extraccion lo lea como una pagina digital y no como un escaneo,",
            "sin formulas de apertura ni de cierre y sin numeros de pagina ni codigos.",
        ],
        0.0,
    )
}

fn opener() -> Page {
    page(&["Senor Juez:", "Texto inventado de otro escrito que empieza en esta pagina de la prueba."], 0.0)
}

/// What a person sets in Ajustes to let the base ask: the automatic
/// reading and the engine of «Preguntar al modelo». Nothing of the cut.
fn reading_on(base: &DocBase) {
    base.set_background_reading(true).unwrap();
    base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "m".into(), ..Default::default() })).unwrap();
}

/// A base with these PDFs, swept: everything else is for the loop.
fn with_files(dir: &TempDir, files: &[(&str, Vec<Page>)]) -> (DocBase, String) {
    for (name, pages) in files {
        dir.write(&format!("o/{name}"), &build(pages));
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    (base, origin.id)
}

/// One turn of the loop with a step: what has no step of its own (the
/// pages of a scan to count, a text of an older extraction) goes before.
fn turn(base: &DocBase, model: &Model) -> Turn {
    for _ in 0..20 {
        let done = base.process_turn(Some(model)).unwrap();
        if done.step.is_some() || !done.worked {
            return done;
        }
    }
    panic!("only work without a step")
}

/// Turns of the loop until it does nothing.
fn until_idle(base: &DocBase, model: &Model) -> Vec<Turn> {
    let mut turns = Vec::new();
    for _ in 0..600 {
        let done = turn(base, model);
        if !done.worked {
            return turns;
        }
        turns.push(done);
    }
    panic!("the loop never stopped: {:?}", turns.iter().rev().take(5).collect::<Vec<_>>());
}

/// The first incomplete step the loop sees now (with what it set aside).
fn first(base: &DocBase) -> Option<Step> {
    base.process_view().unwrap().state.first_incomplete
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

fn fail_scan(base: &DocBase, scan: i64, class_code: &str) {
    let mut failure = Failure::new("recognition", FailureAbout::Version(scan), "prueba");
    failure.cause = failures::cause(class_code);
    failures::record(&base.conn(), &failure).unwrap();
}

fn versions(base: &DocBase) -> Vec<i64> {
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT version_id FROM similar_key_data ORDER BY version_id").unwrap();
    let rows = statement.query_map([], |row| row.get(0)).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

/// A reading that names a party in the document: its key data change.
fn names_a_party(base: &DocBase, version: i64, who: &str) {
    base.conn()
        .execute(
            "INSERT INTO knowledge_mention (version_id, ref, kind, written, normalized, role, quote)
             VALUES (?1, ?2, 'organizacion', ?2, ?3, 'parte', ?2)",
            params![version, who, who.to_lowercase()],
        )
        .unwrap();
}

// ---- A2: the loop ----

#[test]
fn the_loop_does_one_operation_at_a_time_and_no_question_goes_out_with_a_step_before_incomplete() {
    let dir = TempDir::new();
    let (base, _) = with_files(
        &dir,
        &[
            ("escrito.pdf", vec![prose("primera hoja"), prose("segunda hoja"), opener()]),
            ("uno.pdf", vec![prose("una hoja suelta")]),
            ("dos.pdf", vec![prose("otra hoja"), opener(), prose("la ultima hoja")]),
        ],
    );
    reading_on(&base);
    let model = Model::default();
    let mut questions = 0;
    for _ in 0..600 {
        let before = first(&base);
        let asked = model.asked();
        let done = turn(&base, &model);
        if !done.worked {
            break;
        }
        if model.asked() > asked {
            questions += 1;
            let step = done.step.expect("a question is an operation of a step");
            assert_eq!(step.about().class, Class::Asks, "{step:?} asked");
            // The step asked was the first incomplete one: every step
            // before was complete (or set aside with its reason).
            assert!(before.is_none_or(|b| b == step), "asked at {step:?} with {before:?} incomplete");
        }
    }
    assert!(questions > 0, "the cut and the type were asked");
    assert_eq!(first(&base), None, "everything complete");
    assert_eq!(base.process_status().state, "stopped");
    assert!(base.process_status().turns > 5);
}

#[test]
fn the_gap_an_answer_that_changes_the_parties_of_several_documents_makes_the_next_operation_measure() {
    let dir = TempDir::new();
    let (base, _) = with_files(
        &dir,
        &[
            ("alfa.pdf", vec![prose("la hoja alfa con su texto propio")]),
            ("beta.pdf", vec![prose("la hoja beta con otro texto")]),
            ("gama.pdf", vec![prose("la hoja gama con un texto mas")]),
        ],
    );
    reading_on(&base);
    let model = Model::default();
    until_idle(&base, &model);
    let measured = versions(&base);
    assert_eq!(measured.len(), 3);
    // The answer about identities is kept: each document names a party
    // now, and nothing else runs in between.
    for (index, version) in measured.iter().enumerate() {
        names_a_party(&base, *version, &format!("Sociedad Inventada {index}"));
    }
    let asked = model.asked();
    let next = turn(&base, &model);
    assert_eq!(next.step, Some(Step::Similar), "the next operation measures, it never asks");
    assert_eq!(model.asked(), asked);
    assert!(base.process_status().back.is_some_and(|why| why.contains("paso 11")), "«En curso» says why it went back");
    until_idle(&base, &model);
    assert_eq!(first(&base), None);
}

#[test]
fn a_persons_ask_waits_only_for_its_own_document_not_for_the_other_files() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    reading_on(&base);
    let model = Model::default();
    until_idle(&base, &model);
    // The reading off: nothing goes out by itself any more.
    base.set_background_reading(false).unwrap();
    // A new file arrives at the folder: work without AI for it.
    dir.write("o/nuevo.pdf", &build(&[prose("otra hoja nueva"), opener()]));
    base.scan_origin(&origin).unwrap();
    assert_eq!(first(&base), Some(Step::FileHash));
    // A person asks to read the first document again.
    assert_eq!(base.knowledge_request_reread(&RereadTarget::default()).unwrap(), 1);
    let asked = model.asked();
    let next = turn(&base, &model);
    assert_eq!(next.step, Some(Step::Reading), "the ask goes first, before the work of the other file");
    assert!(model.asked() > asked);
    assert!(base.process_state(&Scope::All, false).unwrap().pending(Step::FileHash) > 0, "the new file still waits");
    assert!(base.process_status().scope.is_some_and(|s| s.contains("document:")));
}

#[test]
fn a_new_file_mid_operation_the_operation_ends_and_the_process_starts_again_from_the_beginning() {
    let dir = TempDir::new();
    let (base, _) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    reading_on(&base);
    let model = Model::default();
    // While the model answers its first question, a file lands in the folder.
    *model.drops.lock().unwrap() = Some((dir.path().join("o/nuevo.pdf"), build(&[prose("hoja nueva")])));
    let mut asked_at = None;
    for _ in 0..200 {
        let done = turn(&base, &model);
        if model.texts.load(Ordering::SeqCst) > 0 {
            asked_at = done.step;
            break;
        }
    }
    assert_eq!(asked_at, Some(Step::Type), "the question of the type was asked");
    assert!(dir.path().join("o/nuevo.pdf").exists());
    // The operation ended: its answer is kept and applied.
    assert_eq!(base.document_types_all().unwrap().len(), 1);
    // The wake of the folder: the sweep, then back to the first steps.
    base.sweep_origins(None);
    base.process_woken();
    assert_eq!(turn(&base, &model).step, Some(Step::FileHash));
    until_idle(&base, &model);
    assert_eq!(base.document_types_all().unwrap().len(), 2);
}

#[test]
fn everything_complete_the_loop_stops_and_a_wake_starts_it_again() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    reading_on(&base);
    let model = Model::default();
    until_idle(&base, &model);
    assert!(!turn(&base, &model).worked);
    assert_eq!(base.process_status().state, "stopped");
    // A wake ends the sleep at once.
    let wake = Wake::default();
    wake.request();
    let started = Instant::now();
    wake.wait(Duration::from_secs(5));
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(wake.take(), (true, false));
    // A new file and the sweep: the loop works again.
    dir.write("o/nuevo.pdf", &build(&[prose("hoja nueva")]));
    base.scan_origin(&origin).unwrap();
    base.process_woken();
    assert_eq!(turn(&base, &model).step, Some(Step::FileHash));
}

// ---- The six tests of the barrier, redone over the loop ----

#[test]
fn with_a_scan_waiting_no_question_goes_out_until_it_is_read_or_set_aside() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("escrito.pdf", vec![prose("primera hoja"), prose("segunda hoja"), opener()])]);
    let model = Model::default();
    until_idle(&base, &model);
    reading_on(&base);
    base.process_woken();
    assert_eq!(first(&base), Some(Step::CutQuestion));
    let scan = add_scan(&base, &origin);
    // The scan is the first step incomplete: the loop reads it before any
    // question, the one of the cut included.
    assert_eq!(first(&base), Some(Step::Scans));
    // Its reading fails, of any class: set aside, the questions go on.
    fail_scan(&base, scan, "models-missing");
    let next = turn(&base, &model);
    assert_eq!(next.step, Some(Step::CutQuestion));
    assert_eq!((model.images.load(Ordering::SeqCst), model.texts.load(Ordering::SeqCst)), (1, 0));
    until_idle(&base, &model);
    assert!(model.texts.load(Ordering::SeqCst) > 0, "the type goes out");
}

#[test]
fn what_is_marked_as_a_failure_is_set_aside_until_it_is_resolved() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    let model = Model::default();
    until_idle(&base, &model);
    let scan = add_scan(&base, &origin);
    assert_eq!(first(&base), Some(Step::Scans));
    // A transient one too, while it waits for its next attempt.
    fail_scan(&base, scan, "file-locked");
    let scans = base.process_state(&Scope::All, false).unwrap().step(Step::Scans).unwrap().count;
    assert_eq!((scans.pending, scans.set_aside), (0, 1));
    assert_ne!(first(&base), Some(Step::Scans));
    // Resolved, it is pending again.
    failures::resolve(&base.conn(), "recognition", &FailureAbout::Version(scan)).unwrap();
    assert_eq!(first(&base), Some(Step::Scans));
}

#[test]
fn a_new_file_does_its_work_without_ai_before_any_question_of_the_base() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    reading_on(&base);
    let model = Model::default();
    until_idle(&base, &model);
    let before = model.asked();
    dir.write("o/nuevo.pdf", &build(&[prose("otra hoja nueva")]));
    base.scan_origin(&origin).unwrap();
    base.process_woken();
    let mut local = 0;
    loop {
        let asked = model.asked();
        let done = turn(&base, &model);
        assert!(done.worked, "the loop stopped before asking");
        if model.asked() > asked {
            break;
        }
        local += 1;
    }
    assert!(local >= 4, "bytes, text, ficha, signature and similar first ({local} operations)");
    assert!(model.asked() > before);
    let state = base.process_state(&Scope::All, false).unwrap();
    for step in [Step::FileHash, Step::Text, Step::Shapes, Step::Signature, Step::Similar] {
        assert_eq!(state.pending(step), 0, "{step:?} complete before the question");
    }
}

#[test]
fn without_the_models_of_this_computer_the_scan_waits_with_its_failure() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    let scan = add_scan(&base, &origin);
    // No way to download the models: permanent, once.
    assert_eq!(base.scans_without_models("its models are missing", false).unwrap(), 1);
    assert_eq!(base.scans_without_models("its models are missing", false).unwrap(), 0);
    let class: String = base
        .conn()
        .query_row("SELECT class FROM failure WHERE item = '900' AND state <> 'resolved'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(class, "permanent");
    let scans = base.process_state(&Scope::All, false).unwrap().step(Step::Scans).unwrap().count;
    assert_eq!((scans.pending, scans.set_aside), (0, 1));
    // The models arrive: the scan is pending again.
    base.scans_with_models().unwrap();
    assert_eq!(base.process_state(&Scope::All, false).unwrap().pending(Step::Scans), 1);
    // Being downloaded: transient.
    base.scans_without_models("downloading", true).unwrap();
    let class: String = base
        .conn()
        .query_row("SELECT class FROM failure WHERE item = ?1 AND state <> 'resolved'", [scan.to_string()], |r| r.get(0))
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

    let mine = Thresholds { low: 0.85, high: 0.98, quality: 0.6, low_confidence_type: 0.25, indicator_attempts: 5, indicator_hours: 12, round_cap: 20, retry_minutes: vec![1, 2], name_letters: 3, name_share: 0.2 };
    base.set_thresholds(&mine).unwrap();
    assert_eq!(base.thresholds().unwrap(), mine);
    assert_eq!(failures::indicator_threshold(&base.conn()).unwrap(), (5, 12));
    let mut failure = Failure::new("sweep", FailureAbout::Origin("o".into()), "database is locked");
    failure.cause = failures::cause("db-busy");
    let first = failures::record_at(&base.conn(), &failure, 0).unwrap();
    assert_eq!(first.next_at, Some(60_000));
    let third = {
        failures::record_at(&base.conn(), &failure, 0).unwrap();
        failures::record_at(&base.conn(), &failure, 0).unwrap()
    };
    assert_eq!(third.next_at, Some(120_000));
    assert_eq!(base.progress(&Default::default()).unwrap().auto_questions.cap, 20);
    assert!(base.set_thresholds(&Thresholds { low: 0.98, ..mine.clone() }).is_err());
    assert!(base.set_thresholds(&Thresholds { retry_minutes: vec![], ..mine.clone() }).is_err());
    assert_eq!(base.thresholds().unwrap(), mine);
}

// ---- E4 ----

#[test]
fn in_a_new_base_without_touching_any_setting_of_the_cut_a_doubtful_jump_of_the_cut_by_zones_is_asked_by_itself() {
    let text = |w: &str| page(&[&format!("Texto inventado de la prueba del corte por zonas, {w}, con bastantes letras"), "para que se lea como una pagina digital."], 0.0);
    let dir = TempDir::new();
    let (base, _) = with_files(&dir, &[("zonas.pdf", vec![text("primera"), text("segunda"), text("tercera")])]);
    let model = Model::default();
    until_idle(&base, &model);
    let hash: String = base.conn().query_row("SELECT file_hash FROM location WHERE rel_path = 'zonas.pdf'", [], |row| row.get(0)).unwrap();
    // Its reading by zones, every step doubtful.
    base.put_zones(&hash, &crate::docbase::cut::by_zones::tests::doubtful_file(3), "prueba").unwrap();
    // Only what a person sets to let the base ask at all.
    reading_on(&base);
    base.process_woken();
    let cut_settings: i64 =
        base.conn().query_row("SELECT count(*) FROM docbase_setting WHERE key LIKE 'cut:%'", [], |row| row.get(0)).unwrap();
    assert_eq!(cut_settings, 0, "no setting of the cut was touched");
    until_idle(&base, &model);
    assert!(model.images.load(Ordering::SeqCst) >= 1, "the window was asked by itself");
    assert_eq!(base.cut_questions().unwrap().by_itself, 0, "nothing of the cut left to ask");
    let parts: Vec<(u32, u32)> = base.strip_parts(&hash).unwrap().iter().map(|p| (p.first_page, p.last_page)).collect();
    assert_eq!(parts, vec![(1, 2), (3, 3)], "the answer cut the file");
}

#[test]
fn the_question_of_the_cut_waits_for_the_work_without_ai_of_its_file() {
    let dir = TempDir::new();
    let (base, _) = with_files(&dir, &[("escrito.pdf", vec![prose("primera hoja"), prose("segunda hoja"), opener()])]);
    let model = Model::default();
    until_idle(&base, &model);
    let (hash, version): (String, i64) = base.conn().query_row("SELECT hash, version_id FROM file", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    // Its ficha of the pages is to be made again: work without AI of its file.
    base.conn().execute("DELETE FROM derivative WHERE version_id = ?1 AND kind = ?2", params![version, crate::docbase::page_shapes::SHAPES_KIND]).unwrap();
    reading_on(&base);
    // A person asks the model about the cut of this very file, too.
    base.process_ask(Ask::Cut, &Scope::File(hash.clone())).unwrap();
    assert_eq!(base.process_state(&Scope::File(hash.clone()), true).unwrap().first_incomplete, Some(Step::Shapes));
    let next = turn(&base, &model);
    assert_eq!(next.step, Some(Step::Shapes), "the work without AI of its file first");
    assert_eq!(model.images.load(Ordering::SeqCst), 0, "no question of the cut while its file has local work");
    let mut asked_at = None;
    for _ in 0..50 {
        let done = turn(&base, &model);
        if model.images.load(Ordering::SeqCst) > 0 {
            asked_at = done.step;
            break;
        }
    }
    assert_eq!(asked_at, Some(Step::CutQuestion));
    assert_eq!(base.process_state(&Scope::File(hash), false).unwrap().pending(Step::Shapes), 0);
}

#[test]
fn a_permanent_failure_of_a_type_question_stays_over_its_document_and_is_not_tried_again() {
    let dir = TempDir::new();
    let (base, _) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    reading_on(&base);
    let model = Model { types: Some(Err("413: the question is too large for the service")), ..Model::default() };
    until_idle(&base, &model);
    let asked = model.texts.load(Ordering::SeqCst);
    assert_eq!(asked, 1, "asked once");
    let (item_kind, class, next_at): (String, String, Option<i64>) = base
        .conn()
        .query_row("SELECT item_kind, class, next_at FROM failure WHERE stage = 'types' AND state <> 'resolved'", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!((item_kind.as_str(), class.as_str(), next_at), ("version", "permanent", None), "over its document, not retried");
    let types = base.process_state(&Scope::All, false).unwrap().step(Step::Type).unwrap().count;
    assert_eq!((types.pending, types.set_aside), (0, 1), "set aside: it holds nothing");
    // Neither later nor at a wake.
    base.process_woken();
    until_idle(&base, &model);
    assert_eq!(model.texts.load(Ordering::SeqCst), asked);
    // What «Atención» says is what happens: no retry.
    let view = base.failures_open().unwrap().into_iter().find(|f| f.stage == "types").unwrap();
    assert_eq!((view.class.as_str(), view.next_at), ("permanent", None));
}

#[test]
fn a_transient_failure_of_a_type_question_waits_for_its_next_attempt() {
    let dir = TempDir::new();
    let (base, _) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    reading_on(&base);
    // An answer of another shape: transient (D-714).
    let model = Model { types: Some(Ok("esto no tiene la forma pedida")), ..Model::default() };
    until_idle(&base, &model);
    assert_eq!(model.texts.load(Ordering::SeqCst), 1);
    let (class, next_at, cause): (String, Option<i64>, String) = base
        .conn()
        .query_row("SELECT class, next_at, cause FROM failure WHERE stage = 'types' AND state <> 'resolved'", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .unwrap();
    assert_eq!((class.as_str(), cause.as_str()), ("transient", "bad-answer"));
    let wait = next_at.unwrap() - crate::docbase::now_ms();
    assert!((9 * 60_000..=10 * 60_000).contains(&wait), "the first wait is of 10 minutes ({wait} ms)");
    // Before its time, not asked again.
    base.process_woken();
    until_idle(&base, &model);
    assert_eq!(model.texts.load(Ordering::SeqCst), 1);
    // Its time comes: asked again, and the next wait is longer.
    base.conn().execute("UPDATE failure SET next_at = 1 WHERE stage = 'types'", []).unwrap();
    base.process_woken();
    until_idle(&base, &model);
    assert_eq!(model.texts.load(Ordering::SeqCst), 2);
    let (attempt, next_at): (i64, Option<i64>) = base
        .conn()
        .query_row("SELECT attempt, next_at FROM failure WHERE stage = 'types' AND state <> 'resolved'", [], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!(attempt, 2);
    assert!(next_at.unwrap() - crate::docbase::now_ms() > 50 * 60_000, "an hour the second time");
}

#[test]
fn an_element_that_goes_back_three_times_is_set_aside_for_a_person() {
    let dir = TempDir::new();
    let (base, _) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja de un acuerdo")])]);
    reading_on(&base);
    let model = Model::default();
    until_idle(&base, &model);
    let version = versions(&base)[0];
    for time in 1..=BACK_LIMIT + 1 {
        // An answer changes its parties: it goes back to the similar
        // documents.
        names_a_party(&base, version, &format!("Sociedad Inventada {time}"));
        until_idle(&base, &model);
        let open: i64 = base
            .conn()
            .query_row("SELECT count(*) FROM failure WHERE stage = ?1 AND state <> 'resolved'", [failures::BACK_STAGE], |r| r.get(0))
            .unwrap();
        assert_eq!(open, i64::from(time > BACK_LIMIT), "after {time} times");
    }
    let (item, class): (String, String) = base
        .conn()
        .query_row("SELECT item, class FROM failure WHERE stage = ?1", [failures::BACK_STAGE], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap();
    assert_eq!((item, class.as_str()), (version.to_string(), "unknown"), "over the document, waiting for a person");
    // Once more: it no longer goes back, it waits.
    names_a_party(&base, version, "Sociedad Inventada Otra");
    let similar = base.process_state(&Scope::All, false).unwrap().step(Step::Similar).unwrap().count;
    assert_eq!((similar.pending, similar.set_aside), (0, 1));
}

#[test]
fn the_harness_of_the_run_changes_no_setting_of_the_app() {
    let harness = include_str!("../worker_control.rs");
    let code: String = harness.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    for setting in ["round_cap", "set_thresholds", "windows-by-itself", "WINDOWS_BY_ITSELF", "set_background_reading"] {
        assert!(!code.contains(setting), "the harness touches {setting}");
    }
    assert!(code.contains("process_turn"), "it runs the same loop as the app");
    assert!(code.contains("MICELYA_CONTROL_SETTINGS"), "what a run needs different is in its script");
}

// ---- A3: the watcher, «Volver a procesar» and the counts of «En curso» ----

/// Waits, a little at a time, until the watch has a change that calmed down.
fn heard_and_calmed(watch: &super::super::watch::Watch) -> bool {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        if !watch.calmed().is_empty() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    false
}

#[test]
fn a_new_file_in_a_watched_folder_wakes_the_process_which_starts_again_after_the_operation_going_on() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    reading_on(&base);
    let wake = std::sync::Arc::new(Wake::default());
    let mut watch = super::super::watch::Watch::start(wake.clone());
    watch.follow(&[(origin.clone(), dir.path().join("o"))]);
    std::thread::sleep(Duration::from_millis(300));
    let model = Model::default();
    // While the model answers the question of the type, a file is copied
    // into the folder.
    *model.drops.lock().unwrap() = Some((dir.path().join("o/nuevo.pdf"), build(&[prose("hoja nueva")])));
    let mut asked_at = None;
    for _ in 0..200 {
        let done = turn(&base, &model);
        if model.texts.load(Ordering::SeqCst) > 0 {
            asked_at = done.step;
            break;
        }
    }
    assert_eq!(asked_at, Some(Step::Type));
    // The operation going on ended: its answer is kept.
    assert_eq!(base.document_types_all().unwrap().len(), 1);
    // The watcher heard the copy; once it calmed down, the loop writes it
    // down at the start of its turn and goes back to the first step.
    assert!(heard_and_calmed(&watch), "the watcher hears the new file");
    base.origin_changed(&origin).unwrap();
    assert_eq!(first(&base), Some(Step::Inventory));
    assert_eq!(turn(&base, &model).step, Some(Step::Inventory));
    let back = base.process_status().back.unwrap_or_default();
    assert!(back.contains("volvió al paso 1") && back.contains("un archivo nuevo o cambiado en «O»"), "{back}");
    assert_eq!(turn(&base, &model).step, Some(Step::FileHash));
    until_idle(&base, &model);
    assert_eq!(base.document_types_all().unwrap().len(), 2);
    // The inventory is complete again: the change is older than the sweep.
    assert_eq!(base.process_state(&Scope::All, false).unwrap().pending(Step::Inventory), 0);
}

#[test]
fn processing_a_folder_again_leaves_its_inventory_incomplete_and_asks_nothing_again() {
    let dir = TempDir::new();
    let (base, origin) = with_files(&dir, &[("a/uno.pdf", vec![prose("una hoja")]), ("b/dos.pdf", vec![prose("otra hoja")])]);
    reading_on(&base);
    let model = Model::default();
    until_idle(&base, &model);
    let asked = model.asked();
    let types = base.document_types_all().unwrap().len();
    assert_eq!(base.process_state(&Scope::All, false).unwrap().first_incomplete, None);

    let again = super::run::Again { origin_id: Some(origin.clone()), folder: Some("a".into()), rel_path: None };
    assert_eq!(base.process_again(&again).unwrap(), 1);
    let state = base.process_state(&Scope::All, false).unwrap();
    assert_eq!(state.first_incomplete, Some(Step::Inventory));
    assert_eq!(state.pending(Step::Inventory), 1);
    assert_eq!(state.pending(Step::FileHash), 1, "only the file of the folder");
    let pending: Vec<String> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT rel_path FROM location WHERE hash_state = 'pending'").unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(pending, vec!["a/uno.pdf".to_string()]);

    // The same loop takes it, in its order; nothing changed, so nothing is
    // asked again and every step is complete again.
    assert_eq!(turn(&base, &model).step, Some(Step::Inventory));
    assert!(base.process_status().back.unwrap_or_default().contains("pediste volver a procesar la carpeta «a»"));
    until_idle(&base, &model);
    assert_eq!(model.asked(), asked);
    assert_eq!(base.document_types_all().unwrap().len(), types);
    assert_eq!(base.process_state(&Scope::All, false).unwrap().first_incomplete, None);

    // A file, and everything.
    let file = super::run::Again { origin_id: Some(origin.clone()), folder: None, rel_path: Some("b/dos.pdf".into()) };
    assert_eq!(base.process_again(&file).unwrap(), 1);
    until_idle(&base, &model);
    assert_eq!(base.process_again(&super::run::Again::default()).unwrap(), 2);
    assert_eq!(base.process_state(&Scope::All, false).unwrap().pending(Step::FileHash), 2);
    until_idle(&base, &model);
    assert_eq!(model.asked(), asked);
    // An origin that is not there.
    let gone = super::run::Again { origin_id: Some("no-existe".into()), ..Default::default() };
    assert!(base.process_again(&gone).is_err());
}

#[test]
fn a_file_of_three_parts_counts_three_documents_in_inicio_and_in_en_curso() {
    let dir = TempDir::new();
    let (base, _) = with_files(
        &dir,
        &[("compuesto.pdf", vec![prose("primera parte"), prose("segunda parte"), prose("tercera parte")]), ("suelto.pdf", vec![prose("una hoja")])],
    );
    // The bytes and the text of the files, without asking.
    for _ in 0..10 {
        base.hash_pending(8).unwrap();
        base.extract_pending(8).unwrap();
    }
    // The composite file cut in three parts (data made for the test): the
    // whole text of the file stays a version, and is not a document.
    {
        let conn = base.conn();
        let hash: String = conn
            .query_row("SELECT file_hash FROM location WHERE rel_path = 'compuesto.pdf'", [], |row| row.get(0))
            .unwrap();
        conn.execute("DELETE FROM appearance WHERE resource_hash = ?1 AND selector = 'all'", [&hash]).unwrap();
        for part in 1..=3 {
            conn.execute("INSERT INTO version (id, state, first_seen_at) VALUES (?1, 'processed', 1)", [800 + part]).unwrap();
            conn.execute(
                "INSERT INTO appearance (version_id, resource_hash, selector, position, cut_by, cut_state, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'rule', 'confirmed', 1)",
                params![800 + part, hash, format!("pages:{part}-{part}"), part],
            )
            .unwrap();
        }
    }
    let health = base.health(&super::super::Visible::All).unwrap();
    assert_eq!((health.documents, health.files, health.places), (4, 2, 2), "3 parts and the single file");
    let view = base.process_view().unwrap();
    assert_eq!(view.totals.documents, 4);
    assert_eq!(view.totals.documents, health.documents, "Inicio and «En curso» say the same");
    // A copy of the composite in another folder is the same 3 documents.
    let bytes = std::fs::read(dir.path().join("o/compuesto.pdf")).unwrap();
    dir.write("o/copia/compuesto.pdf", &bytes);
    let origin: String = base.conn().query_row("SELECT id FROM origin", [], |row| row.get(0)).unwrap();
    base.scan_origin(&origin).unwrap();
    base.hash_pending(8).unwrap();
    let health = base.health(&super::super::Visible::All).unwrap();
    assert_eq!((health.documents, health.files, health.places), (4, 2, 3));
}
