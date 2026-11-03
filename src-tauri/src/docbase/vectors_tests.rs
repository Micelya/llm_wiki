//! The step of the vectors (C1 of 2 nov), with a Micelya Core of the test
//! that gives repeatable vectors (`micelya_core::fake`): a copy asks
//! nothing; a text that changed asks again; another model asks everything
//! again; a 503 leaves the step waiting with its transient failure; with
//! «Ninguno» the step is complete; they go 96 at a time. Every text and
//! every answer is made up for the test. The last one, ignored, asks the
//! real Core (the test of the stage, fewer than 5 calls).

use rusqlite::params;

use super::process::{Class, Scope, Step};
use super::services::{EngineChoice, Service, NO_VECTORS};
use super::testing::TempDir;
use super::vectors::{self, TestCore, TEST_CORE};
use super::DocBase;
use crate::micelya_core::fake::{fake_vector, FakeCore};
use crate::source_volume::page_shape::test_pdf::{build, page, Page};

const MODEL: &str = "us.cohere.embed-v4:0";

fn prose(words: &str) -> Page {
    page(
        &[
            &format!("Texto inventado de la prueba de los vectores, {words}, con bastantes letras"),
            "para que la extraccion lo lea como una pagina digital y no como un escaneo,",
            "sin formulas de apertura ni de cierre y sin numeros de pagina ni codigos.",
        ],
        0.0,
    )
}

/// Everything the base does without AI, until nothing is left.
fn local(base: &DocBase) {
    loop {
        let mut done = base.hash_pending(200).unwrap() + base.extract_pending(200).unwrap() + base.purge_orphans().unwrap();
        done += base.profile_pending(200).unwrap() + base.cut_pending(200).unwrap() + base.cut_parts_pending(200).unwrap();
        done += base.similar_step(200).unwrap();
        if done == 0 {
            return;
        }
    }
}

/// A base with these PDFs, its work without AI done and the automatic
/// reading on.
fn base_with(dir: &TempDir, files: &[(&str, Vec<Page>)]) -> (DocBase, String) {
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

/// The Core of the test, set for this thread while it lives.
struct Core(FakeCore);

impl Core {
    fn start() -> Core {
        let fake = FakeCore::start();
        TEST_CORE.with(|core| *core.borrow_mut() = Some(TestCore::Fake(fake.connection())));
        Core(fake)
    }

    /// How many texts each call of vectors carried.
    fn calls(&self) -> Vec<usize> {
        self.0.with(|state| state.embedded.clone())
    }
}

impl Drop for Core {
    fn drop(&mut self) {
        TEST_CORE.with(|core| *core.borrow_mut() = None);
    }
}

/// The operations of the step, until it does nothing more.
fn run(base: &DocBase) -> usize {
    let mut operations = 0;
    while base.vectors_operation().unwrap() {
        operations += 1;
        assert!(operations < 50, "the step does not end");
    }
    operations
}

/// The step of the vectors as «En curso» counts it: left, set aside, for a person.
fn step(base: &DocBase) -> (usize, usize, usize) {
    let state = base.process_state(&Scope::All, false).unwrap();
    let count = state.step(Step::Vectors).unwrap().count;
    (count.pending, count.set_aside, count.person)
}

/// The documents, by version.
fn documents(base: &DocBase) -> Vec<i64> {
    let conn = base.conn();
    let mut statement = conn
        .prepare(&format!(
            "SELECT c.id FROM version c WHERE c.state = 'processed' AND {} ORDER BY c.id",
            super::cut::parts::is_document_sql("c.id")
        ))
        .unwrap();
    let rows = statement.query_map([], |row| row.get(0)).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

/// The rows of the table of vectors: version, model, numbers, print.
fn rows(base: &DocBase) -> Vec<(i64, String, i64, String)> {
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT version_id, model, dimension, text_hash FROM version_vector ORDER BY version_id").unwrap();
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn text_of(base: &DocBase, version: i64) -> String {
    let body: String = base
        .conn()
        .query_row("SELECT body FROM derivative WHERE version_id = ?1 AND kind = 'text'", [version], |row| row.get(0))
        .unwrap();
    vectors::text_for_vector(&body)
}

fn choose(base: &DocBase, engine: &str, model: &str) {
    let choice = EngineChoice { engine: engine.into(), connection: "micelya-core".into(), model: model.into(), ..Default::default() };
    base.set_engine_of(Service::Embed, Some(choice)).unwrap();
}

#[test]
fn a_copy_asks_nothing_and_the_loop_does_the_vectors_after_the_signature_and_before_the_families() {
    let core = Core::start();
    let dir = TempDir::new();
    let (base, origin) = base_with(&dir, &[("uno.pdf", vec![prose("primera hoja")]), ("dos.pdf", vec![prose("segunda hoja")])]);
    // The same bytes in another folder: the same version.
    dir.write("o/copias/uno.pdf", &build(&[prose("primera hoja")]));
    base.scan_origin(&origin).unwrap();
    local(&base);
    let docs = documents(&base);
    assert_eq!(docs.len(), 2, "the copy is the same version");
    assert_eq!(step(&base), (2, 0, 0));
    assert_eq!(super::process::steps::class_now(&base.conn(), Step::Vectors).unwrap(), Class::Asks);

    // The loop: the signature and the families are done, the vectors are
    // the first step left; one operation, one call with both texts.
    let state = base.process_state(&Scope::All, true).unwrap();
    assert_eq!(state.first_incomplete, Some(Step::Vectors));
    // (What has no step of its own and costs nothing goes first.)
    let turn = (0..5).map(|_| base.process_turn(None).unwrap()).find(|turn| turn.step.is_some()).unwrap();
    assert_eq!(turn.step, Some(Step::Vectors));
    assert_eq!(core.calls(), vec![2]);
    assert_eq!(step(&base), (0, 0, 0));
    let kept = rows(&base);
    assert_eq!(kept.iter().map(|r| (r.0, r.1.as_str(), r.2)).collect::<Vec<_>>(), vec![(docs[0], MODEL, 1024), (docs[1], MODEL, 1024)]);
    for (version, _, _, print) in &kept {
        let text = text_of(&base, *version);
        assert!(text.contains("Texto inventado") && !text.contains("<!--"), "{text}");
        assert_eq!(print, &vectors::print_of(&text));
        assert_eq!(vectors::vector_of(&base.conn(), *version).unwrap(), Some(Some(fake_vector(MODEL, &text, 1024))));
    }

    // Two copies more: nothing is asked.
    dir.write("o/otra/uno.pdf", &build(&[prose("primera hoja")]));
    dir.write("o/otra/dos.pdf", &build(&[prose("segunda hoja")]));
    base.scan_origin(&origin).unwrap();
    local(&base);
    assert_eq!(step(&base), (0, 0, 0));
    assert_eq!(run(&base), 0);
    assert_eq!(core.calls(), vec![2]);
}

#[test]
fn a_changed_text_asks_again_the_same_text_does_not_and_another_model_asks_everything() {
    let core = Core::start();
    let dir = TempDir::new();
    let (base, _) = base_with(&dir, &[("uno.pdf", vec![prose("primera hoja")]), ("dos.pdf", vec![prose("segunda hoja")])]);
    run(&base);
    assert_eq!(core.calls(), vec![2]);
    let docs = documents(&base);

    // A text read again that says something else: only that one is asked.
    base.conn()
        .execute(
            "UPDATE derivative SET body = body || ' y una linea nueva', created_at = created_at + 1 WHERE version_id = ?1 AND kind = 'text'",
            [docs[0]],
        )
        .unwrap();
    assert_eq!(step(&base), (1, 0, 0));
    run(&base);
    assert_eq!(core.calls(), vec![2, 1]);
    let text = text_of(&base, docs[0]);
    assert!(text.ends_with("y una linea nueva"));
    assert_eq!(vectors::vector_of(&base.conn(), docs[0]).unwrap(), Some(Some(fake_vector(MODEL, &text, 1024))));

    // Read again with the same words: its print is the same, nothing is asked.
    base.conn().execute("UPDATE derivative SET created_at = created_at + 1 WHERE version_id = ?1 AND kind = 'text'", [docs[1]]).unwrap();
    assert_eq!(step(&base), (1, 0, 0));
    run(&base);
    assert_eq!(core.calls(), vec![2, 1]);
    assert_eq!(step(&base), (0, 0, 0));

    // Another model in Ajustes: everything is asked again, with it.
    choose(&base, "core", "global.cohere.embed-v4:0");
    assert_eq!(step(&base), (2, 0, 0));
    run(&base);
    assert_eq!(core.calls(), vec![2, 1, 2]);
    assert!(rows(&base).iter().all(|r| r.1 == "global.cohere.embed-v4:0"));
    assert_eq!(step(&base), (0, 0, 0));
}

#[test]
fn a_503_leaves_the_step_waiting_with_its_transient_failure_and_atencion_tells_it_past_the_threshold() {
    let core = Core::start();
    let dir = TempDir::new();
    let (base, _) = base_with(&dir, &[("uno.pdf", vec![prose("primera hoja")]), ("dos.pdf", vec![prose("segunda hoja")])]);
    let failure = |base: &DocBase| -> (String, String, String, Option<i64>, i64, bool) {
        base.conn()
            .query_row(
                "SELECT cause, class, state, next_at, attempt, indicator FROM failure WHERE stage = 'vectors' ORDER BY id DESC LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
            )
            .unwrap()
    };
    let due = |base: &DocBase| base.conn().execute("UPDATE failure SET next_at = 1 WHERE stage = 'vectors'", []).unwrap();

    for attempt in 1..=3 {
        // Each 503 is repeated by the client; the third is the step's.
        core.0.with(|state| state.embed_failures = vec![503, 503, 503]);
        assert!(base.vectors_operation().unwrap(), "a failure written down is an operation");
        let (cause, class, state, next_at, attempts, indicator) = failure(&base);
        assert_eq!((cause.as_str(), class.as_str(), state.as_str(), attempts), ("no-connection", "transient", "open", attempt));
        assert!(next_at.is_some_and(|at| at > super::now_ms()));
        // Atención tells it louder from the third attempt (D-722).
        assert_eq!(indicator, attempt >= 3, "attempt {attempt}");
        // The step waits: set aside, not for a person, nothing asked again
        // before its next attempt, and the questions to the model go on.
        assert_eq!(step(&base), (0, 2, 0));
        assert!(!base.vectors_operation().unwrap());
        assert_eq!(super::process::steps::asking_held(&base.conn(), true).unwrap(), Some("falta elegir el motor de IA"));
        due(&base);
        assert_eq!(step(&base), (2, 0, 0));
    }
    assert!(core.calls().is_empty());
    run(&base);
    assert_eq!(core.calls(), vec![2]);
    assert_eq!(failure(&base).2, "resolved");
    assert_eq!(step(&base), (0, 0, 0));

    // Without a connection with Core: the same, transient.
    TEST_CORE.with(|c| *c.borrow_mut() = Some(TestCore::Missing));
    choose(&base, "core", "otro-modelo");
    assert!(base.vectors_operation().unwrap());
    let (cause, class, ..) = failure(&base);
    assert_eq!((cause.as_str(), class.as_str()), ("no-connection", "transient"));
    assert_eq!(step(&base), (0, 2, 0));
    // A person who chooses «Ninguno» takes the failure away: complete.
    choose(&base, NO_VECTORS, "");
    assert_eq!(failure(&base).2, "resolved");
    assert_eq!(step(&base), (0, 0, 0));
}

#[test]
fn with_none_the_step_is_complete_and_nothing_is_asked() {
    let core = Core::start();
    let dir = TempDir::new();
    let (base, _) = base_with(&dir, &[("uno.pdf", vec![prose("primera hoja")])]);
    choose(&base, NO_VECTORS, "");
    assert_eq!(step(&base), (0, 0, 0));
    assert_eq!(super::process::steps::class_now(&base.conn(), Step::Vectors).unwrap(), Class::Local);
    assert!(!base.vectors_operation().unwrap());
    assert!(core.calls().is_empty() && rows(&base).is_empty());
    // With the automatic reading off, Core waits for it: set aside.
    choose(&base, "core", "");
    base.set_background_reading(false).unwrap();
    let state = base.process_state(&Scope::All, false).unwrap();
    let vectors = state.step(Step::Vectors).unwrap();
    assert_eq!((vectors.count.pending, vectors.count.set_aside), (0, 1));
    assert_eq!(vectors.held, Some("la lectura automática está apagada"));
}

#[test]
fn they_go_96_at_a_time() {
    let core = Core::start();
    let dir = TempDir::new();
    let files: Vec<(String, Vec<Page>)> = (0..97).map(|n| (format!("hoja-{n}.pdf"), vec![prose(&format!("hoja numero {n}"))])).collect();
    let files: Vec<(&str, Vec<Page>)> = files.iter().map(|(name, pages)| (name.as_str(), pages.clone())).collect();
    let (base, _) = base_with(&dir, &files);
    assert_eq!(step(&base), (97, 0, 0));
    assert_eq!(run(&base), 2);
    assert_eq!(core.calls(), vec![96, 1]);
    assert_eq!(rows(&base).len(), 97);
    let no_vector: i64 = base.conn().query_row("SELECT count(*) FROM version_vector WHERE vector IS NULL", [], |r| r.get(0)).unwrap();
    assert_eq!(no_vector, 0);
    // A version that goes takes its vector with it.
    let first = documents(&base)[0];
    base.conn().execute("DELETE FROM version WHERE id = ?1", params![first]).ok();
    let left: i64 = base.conn().query_row("SELECT count(*) FROM version_vector WHERE version_id = ?1", [first], |r| r.get(0)).unwrap();
    let still: i64 = base.conn().query_row("SELECT count(*) FROM version WHERE id = ?1", [first], |r| r.get(0)).unwrap();
    assert_eq!(left, still);
}

#[test]
fn the_text_is_the_one_with_its_bands_without_the_comments_of_the_machine() {
    // As the reading by zones keeps a scan (E1 of 2 nov): the firm lines,
    // the approximate ones with their mark; the discarded are not there.
    let body = "## Page 1\n\n<!-- texto-reconocido motor=\"local por zonas\" -->\nPARTIDA INVENTADA\n[aproximado] Hijo de Persona Tres\n";
    let expected = "## Page 1\n\n\nPARTIDA INVENTADA\n[aproximado] Hijo de Persona Tres";
    assert_eq!(vectors::text_for_vector(body), expected);
    assert_eq!(vectors::text_for_vector("  <!-- solo un comentario -->  "), "");
    assert_eq!(vectors::print_of("a"), vectors::print_of("a"));
    assert_ne!(vectors::print_of("a"), vectors::print_of("b"));
}

/// THE TEST WITH THE REAL MICELYA CORE (C1 of 2 nov): ignored; run it by
/// hand with `cargo test real_core -- --ignored --nocapture`. Two texts
/// made up for the test, never documents of a folder: their dimension,
/// that asking the same text twice gives the same vector, and that a copy
/// asks nothing. Two calls of vectors (and the permit). The connection and
/// the permit are never printed.
#[test]
#[ignore]
fn real_core_vectors_of_two_made_up_texts() {
    use std::sync::atomic::Ordering;
    let connection = crate::micelya_core::saved_connection().unwrap().expect("the connection with Micelya Core in the keychain");
    assert!(connection.environment.to_lowercase().contains("dev"), "only the environment of development");
    TEST_CORE.with(|core| *core.borrow_mut() = Some(TestCore::Real));
    let dir = TempDir::new();
    let contract = page(
        &[
            "Contrato inventado de locacion de un deposito entre dos sociedades de la prueba,",
            "por veinticuatro meses, con un precio mensual que se actualiza cada seis meses,",
            "y la obligacion de devolver el deposito en el mismo estado en que se recibio.",
        ],
        0.0,
    );
    let letter = page(
        &[
            "Carta inventada de una persona de la prueba a su banco, que pide el cierre",
            "de una cuenta corriente y la devolucion del saldo por transferencia,",
            "con la firma al pie y la fecha del dia en que se escribio.",
        ],
        0.0,
    );
    let (base, origin) = base_with(&dir, &[("contrato.pdf", vec![contract.clone()]), ("carta.pdf", vec![letter])]);
    dir.write("o/copia/contrato.pdf", &build(&[contract]));
    base.scan_origin(&origin).unwrap();
    local(&base);
    let docs = documents(&base);
    assert_eq!(docs.len(), 2, "the copy is the same version");
    let before = vectors::CALLS.load(Ordering::SeqCst);
    run(&base);
    let step_calls = vectors::CALLS.load(Ordering::SeqCst) - before;
    assert_eq!(step(&base), (0, 0, 0));
    let mut kept = Vec::new();
    for version in &docs {
        let vector = vectors::vector_of(&base.conn(), *version).unwrap().flatten().expect("its vector");
        let norm = vector.iter().map(|n| n * n).sum::<f32>().sqrt();
        println!("documento inventado: {} numeros, norma {norm:.6}", vector.len());
        assert_eq!(vector.len(), 1024);
        assert!((norm - 1.0).abs() < 1e-3);
        kept.push(vector);
    }
    // The same text asked again, by itself.
    let client = crate::micelya_core::client::CoreClient::new(connection, crate::micelya_core::client::Timing::default()).unwrap();
    let again = client.embed(MODEL, "document", &[text_of(&base, docs[0])], 1024).unwrap().remove(0);
    let cosine: f32 = again.iter().zip(&kept[0]).map(|(a, b)| a * b).sum();
    let between: f32 = kept[0].iter().zip(&kept[1]).map(|(a, b)| a * b).sum();
    let largest_gap = again.iter().zip(&kept[0]).map(|(a, b)| (a - b).abs()).fold(0.0_f32, f32::max);
    println!("llamadas del paso: {step_calls}; mismo texto dos veces: parecido {cosine:.8}, mayor diferencia {largest_gap:.2e}; entre los dos textos: {between:.4}");
    assert_eq!(step_calls, 1, "one call for the two texts; the copy asked nothing");
    // Measured on 2 nov (the only run): 0,99988, the largest gap 0,0015, and
    // 0,50 between the two texts. Asked alone or with another text, the same
    // text does not give the very same numbers: why the step never asks again
    // a text whose print it has. The threshold was set after that run.
    assert!(cosine > 0.999, "{cosine}");
    TEST_CORE.with(|core| *core.borrow_mut() = None);
}
