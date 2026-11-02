//! The step of the scans with the reader chosen in Ajustes (E2 of 2 nov):
//! the loop of the process with stand-in readers (`TEST_READERS`) and the
//! invented image and PDF of the fixtures (R-24). No real model, no models
//! of this computer, no answer, reading or base of any run.

use std::rc::Rc;

use super::super::failures::Class as FailureClass;
use super::super::recognize::{scan_without_models, RecognitionRequest, MISSING_AI_READER};
use super::super::services::{AiServices, EngineChoice, ServiceChoice};
use super::super::testing::TempDir;
use super::super::DocBase;
use super::run::TEST_READERS;
use super::steps::class_now;
use super::{Class, Scope, Step};
use crate::source_volume::recognition::{engine_id, FakeRecognizer, RecognitionConfig, TextRecognizer};
use crate::source_volume::recognition_local;

// Invented texts (R-24).
const NOTE: &str = "NOTA DE PRUEBA\n\nSe deja constancia de que Persona Tres retiró los papeles del inmueble del domicilio \
inventado el día del vencimiento, sin observaciones de ninguna de las partes.";
const OTHER: &str = "OTRA NOTA DE PRUEBA\n\nSe deja constancia de que la llave del portón quedó en la administración del \
edificio inventado hasta que alguna de las partes la retire, con aviso de un día.";

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/source_volume/fixtures").join(name)).unwrap()
}

/// A base over a folder with these files (name in the folder, fixture).
fn base_with(files: &[(&str, &str)]) -> (TempDir, DocBase) {
    TEST_READERS.with(|readers| readers.borrow_mut().clear());
    let dir = TempDir::new();
    for (name, from) in files {
        dir.write(&format!("o/{name}"), &fixture(from));
    }
    let base = DocBase::in_memory().unwrap();
    base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    (dir, base)
}

fn reader(engine: &str, model: &str) -> EngineChoice {
    let connection = if engine == "core" { crate::micelya_core::CONNECTION_ID } else { engine };
    EngineChoice { engine: engine.into(), connection: connection.into(), model: model.into(), reasoning: String::new() }
}

/// Ajustes → Servicios de IA → Reconocer escaneos: the reader, and the one of «Leer con IA».
fn choose(base: &DocBase, principal: Option<EngineChoice>, for_ai: Option<EngineChoice>) {
    let services = AiServices { recognize: ServiceChoice { engine: principal, alternative: for_ai }, ..Default::default() };
    base.set_ai_services(services).unwrap();
}

/// A stand-in reader the step of the scans reads with, by its name.
fn stand_in(id: &str, answers: Vec<Result<&str, &str>>) -> Rc<FakeRecognizer> {
    let fake = Rc::new(FakeRecognizer::of(id, answers));
    TEST_READERS.with(|readers| readers.borrow_mut().push(fake.clone() as Rc<dyn TextRecognizer>));
    fake
}

/// The name the reader of this computer keeps its text with.
fn local_id() -> String {
    engine_id(&RecognitionConfig { engine: recognition_local::ENGINE.into(), model: recognition_local::catalog().model_set.clone() }).unwrap()
}

/// Turns of the loop, without an engine of «Preguntar al modelo», until it
/// has nothing to do.
fn work(base: &DocBase) {
    let mut steps = Vec::new();
    for _ in 0..200 {
        let turn = base.process_turn(None).unwrap();
        if !turn.worked {
            return;
        }
        steps.push(turn.step);
    }
    panic!("the loop did not stop: {:?}", &steps[steps.len() - 10..]);
}

fn content_of(base: &DocBase, name: &str) -> i64 {
    base.conn()
        .query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |row| row.get(0))
        .unwrap()
}

fn text_engine(base: &DocBase, name: &str) -> Option<String> {
    base.conn()
        .query_row(
            "SELECT d.engine FROM location l JOIN file f ON f.hash = l.file_hash
             JOIN derivative d ON d.version_id = f.version_id AND d.kind = 'text' WHERE l.name = ?1",
            [name],
            |row| row.get(0),
        )
        .ok()
}

/// The failures of the recognition still open over a document: (cause, class, next attempt).
fn open_failures(base: &DocBase, content: i64) -> Vec<(String, String, Option<i64>)> {
    let conn = base.conn();
    let mut statement = conn
        .prepare("SELECT cause, class, next_at FROM failure WHERE stage = 'recognition' AND item = ?1 AND state <> 'resolved'")
        .unwrap();
    let rows = statement.query_map([content.to_string()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn scans(base: &DocBase) -> (usize, usize) {
    let count = base.process_state(&Scope::All, false).unwrap().step(Step::Scans).unwrap().count;
    (count.pending, count.set_aside)
}

#[test]
fn with_the_reader_of_this_computer_chosen_the_step_of_the_scans_is_local_and_asks_nothing() {
    let (_dir, base) = base_with(&[("uno.png", "pagina-prueba.png")]);
    // Nothing chosen in Ajustes: the reader of this computer, by default.
    assert_eq!(class_now(&base.conn(), Step::Scans).unwrap(), Class::Local);
    let local = stand_in(&local_id(), vec![Ok(NOTE)]);
    let codex = stand_in("codex-cli:m", vec![Ok(OTHER)]);
    // The automatic reading off: the reader of this computer reads anyway.
    assert!(!base.background_reading().unwrap());
    work(&base);
    assert_eq!((local.calls(), codex.calls()), (1, 0));
    assert_eq!(text_engine(&base, "uno.png"), Some(local_id()));
    // No question: the round counted none.
    assert_eq!(base.engine_state().snapshot().auto_asked, 0);
    assert_eq!(scans(&base), (0, 0));

    // Chosen by name, the same.
    choose(&base, Some(reader(recognition_local::ENGINE, "")), None);
    assert_eq!(class_now(&base.conn(), Step::Scans).unwrap(), Class::Local);
}

#[test]
fn with_codex_or_core_chosen_the_step_asks_and_a_failure_sets_the_scan_aside_with_its_retry() {
    for (engine, model, id) in [("codex-cli", "m", "codex-cli:m"), ("core", "textract-1.0", "core:textract-1.0")] {
        let (_dir, base) = base_with(&[("uno.png", "pagina-prueba.png")]);
        choose(&base, Some(reader(engine, model)), None);
        assert_eq!(class_now(&base.conn(), Step::Scans).unwrap(), Class::Asks, "{engine}");
        let cloud = stand_in(id, vec![Err("You've hit your usage limit"), Ok(NOTE)]);
        let local = stand_in(&local_id(), vec![Ok(OTHER)]);

        // The reading off: a reader in the cloud does not read by itself
        // (D-775), and the reader of this computer does not take its scans.
        work(&base);
        let one = content_of(&base, "uno.png");
        assert_eq!((cloud.calls(), local.calls()), (0, 0), "{engine}");
        assert_eq!(scans(&base), (0, 1), "{engine}: set aside, the reading off");

        // The reading on: it asks. The provider at its limit: the scan is
        // set aside with its failure and its next attempt (D-714).
        base.set_background_reading(true).unwrap();
        work(&base);
        assert_eq!(cloud.calls(), 1, "{engine}");
        let failures = open_failures(&base, one);
        assert_eq!(failures.len(), 1, "{engine}: {failures:?}");
        assert_eq!((failures[0].0.as_str(), failures[0].1.as_str()), ("rate-limit", FailureClass::Transient.as_str()));
        assert!(failures[0].2.is_some(), "{engine}: it has its next attempt");
        assert_eq!(scans(&base), (0, 1), "{engine}");
        // It is a question of the round.
        assert!(base.engine_state().snapshot().auto_asked >= 1, "{engine}");
        // Not tried again before its next attempt.
        work(&base);
        assert_eq!(cloud.calls(), 1, "{engine}");

        // Its next attempt comes: read, and its failure is behind.
        base.conn().execute("UPDATE failure SET next_at = 1 WHERE stage = 'recognition'", []).unwrap();
        assert_eq!(scans(&base), (1, 0), "{engine}");
        work(&base);
        assert_eq!(cloud.calls(), 2, "{engine}");
        assert_eq!(text_engine(&base, "uno.png").as_deref(), Some(id));
        assert!(open_failures(&base, one).is_empty(), "{engine}");
        assert_eq!(local.calls(), 0, "{engine}");
    }
}

#[test]
fn what_a_person_asks_for_is_read_with_the_reader_chosen_and_never_dropped() {
    let (_dir, base) = base_with(&[("uno.png", "pagina-prueba.png"), ("dos.pdf", "escaneo-2-paginas.pdf")]);
    choose(&base, Some(reader("core", "textract-1.0")), None);
    let core = stand_in("core:textract-1.0", vec![Ok(NOTE), Ok(NOTE), Ok(OTHER)]);
    let local = stand_in(&local_id(), vec![Ok(OTHER), Ok(OTHER)]);
    work(&base);
    let (one, two) = (content_of(&base, "uno.png"), content_of(&base, "dos.pdf"));

    // A person asks for both, the reading off: they are read with the
    // reader chosen in Ajustes, Micelya Core.
    assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![one])).unwrap(), 1);
    work(&base);
    assert_eq!(text_engine(&base, "uno.png").as_deref(), Some("core:textract-1.0"));
    assert_eq!(core.calls(), 1);

    // Asked with Core, and then Ajustes changes to the reader of this
    // computer: the request keeps its reader. It is not taken by another
    // one and dropped (the audit of 2 nov, 2.2).
    assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![two])).unwrap(), 1);
    choose(&base, None, None);
    work(&base);
    assert_eq!(text_engine(&base, "dos.pdf").as_deref(), Some("core:textract-1.0"));
    assert_eq!(core.calls(), 3);
    assert_eq!(local.calls(), 0);
    assert!(super::super::recognize::requested_recognitions(&base.conn()).unwrap().is_empty());
}

#[test]
fn without_the_models_of_this_computer_and_core_chosen_the_scan_does_not_fail() {
    let (_dir, base) = base_with(&[("uno.png", "pagina-prueba.png")]);
    // Its bytes and its text, without the step of the scans.
    let origin = base.origins(&super::super::Visible::All).unwrap()[0].id.clone();
    base.scan_origin(&origin).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    let one = content_of(&base, "uno.png");
    // Before: the reader by default had no models, and the scan waited
    // with that failure.
    assert!(scan_without_models(&base.conn(), one, "faltan sus modelos", false).unwrap());
    assert_eq!(open_failures(&base, one)[0].0, "models-missing");
    assert_eq!(scans(&base), (0, 1));

    // Micelya Core chosen: the failure of the models does not apply; the
    // scan goes back to its step and Core reads it.
    choose(&base, Some(reader("core", "")), None);
    base.set_background_reading(true).unwrap();
    let core = stand_in("core:textract-1.0", vec![Ok(NOTE)]);
    work(&base);
    assert_eq!(core.calls(), 1);
    assert_eq!(text_engine(&base, "uno.png").as_deref(), Some("core:textract-1.0"));
    assert!(open_failures(&base, one).is_empty());
    let models: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM failure WHERE cause IN ('models-missing', 'models-downloading') AND state <> 'resolved'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(models, 0);
}

#[test]
fn reading_with_ai_is_an_ask_like_any_other_with_the_reader_of_ai_and_its_cost_first() {
    let (_dir, base) = base_with(&[("uno.png", "pagina-prueba.png")]);
    let local = stand_in(&local_id(), vec![Ok(OTHER)]);
    work(&base);
    let one = content_of(&base, "uno.png");
    assert_eq!(text_engine(&base, "uno.png"), Some(local_id()));

    // No reader of AI chosen: the screen says where to choose it.
    assert_eq!(base.request_ai_reading(one).unwrap_err(), MISSING_AI_READER);
    assert!(base.ai_reading_cost(one).is_err());

    // Codex as the reader of «Leer con IA»; the reader of the scans stays
    // the one of this computer.
    choose(&base, None, Some(reader("codex-cli", "m")));
    assert_eq!(class_now(&base.conn(), Step::Scans).unwrap(), Class::Local);
    let cost = base.ai_reading_cost(one).unwrap();
    assert_eq!((cost.engine.as_str(), cost.pages), ("codex-cli:m", 1));
    let codex = stand_in("codex-cli:m", vec![Ok(NOTE)]);
    // The reading off: what a person asks goes out.
    assert_eq!(base.request_ai_reading(one).unwrap(), 1);
    let asks = base.process_asks().unwrap();
    assert!(asks.iter().any(|a| a.scope == Scope::Document(one)), "{asks:?}");
    work(&base);
    assert_eq!((codex.calls(), local.calls()), (1, 1));
    assert_eq!(text_engine(&base, "uno.png").as_deref(), Some("codex-cli:m"));
    // Read: asking again with the same reader reads nothing.
    assert_eq!(base.request_ai_reading(one).unwrap(), 0);
}
