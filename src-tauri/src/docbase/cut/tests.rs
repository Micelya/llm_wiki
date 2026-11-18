//! The cutter over invented PDFs (R-24): the rules always run, a doubtful
//! step is asked only with the automatic reading on, the same two pages
//! are never asked twice, and an even file goes on without asking.

use std::sync::atomic::{AtomicUsize, Ordering};

use crate::docbase::progress::Scope;
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::page_shape::test_pdf::{build, page, Page};

use super::{rules_version, Verdict};

struct FakeEyes {
    calls: AtomicUsize,
}

impl DocumentAsker for FakeEyes {
    fn engine_id(&self) -> String {
        "falso:ojos".into()
    }

    fn ask(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
        Err("this fake only looks at images".into())
    }

    fn sees_images(&self) -> bool {
        true
    }

    fn ask_with_images(&self, _prompt: &str, _schema: &serde_json::Value, images: &[std::path::PathBuf]) -> Result<String, String> {
        assert_eq!(images.len(), 2);
        assert!(images.iter().all(|i| i.is_absolute() && i.exists()));
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(r#"{"second_starts_new_document": true, "first_page_type": "other", "second_page_type": "recibo", "confidence": "high"}"#.into())
    }
}

fn prose(words: &str) -> Page {
    page(
        &[
            &format!("Texto inventado de la prueba del cortador, {words}, con bastantes letras"),
            "para que la extraccion lo lea como una pagina digital y no como un escaneo,",
            "sin formulas de apertura ni de cierre y sin numeros de pagina ni codigos.",
        ],
        0.0,
    )
}

fn settle(base: &DocBase, origin: &str) {
    base.scan_origin(origin).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    while base.profile_pending(10).unwrap() > 0 {}
    while base.cut_pending(10).unwrap() > 0 {}
}

fn hash_of(base: &DocBase, name: &str) -> String {
    base.conn().query_row("SELECT file_hash FROM location WHERE rel_path = ?1", [name], |row| row.get(0)).unwrap()
}

#[test]
fn a_doubtful_step_is_asked_only_with_the_reading_on_and_the_same_pages_never_twice() {
    let (x, y) = (prose("primera hoja"), prose("segunda hoja"));
    let opener = page(&["Senor Juez:", "Texto inventado de otro escrito que empieza en esta pagina de la prueba."], 0.0);
    let dir = TempDir::new();
    dir.write("o/uno.pdf", &build(&[x.clone(), y.clone()]));
    dir.write("o/dos.pdf", &build(&[x.clone(), y.clone(), opener.clone()]));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    settle(&base, &origin.id);

    let (uno, dos) = (hash_of(&base, "uno.pdf"), hash_of(&base, "dos.pdf"));
    // An even file without a cut: its step goes on, and is not asked.
    let steps = base.cut_steps(&uno).unwrap();
    assert_eq!(steps.iter().map(|s| (s.after, s.verdict)).collect::<Vec<_>>(), vec![(0, Verdict::New), (1, Verdict::Continue)]);
    assert_eq!(steps[1].rule, "uniform");
    assert!(steps[1].question.is_some());
    assert!(!steps[1].starts());
    // An even file with a cut of a rule keeps its doubtful steps.
    let steps = base.cut_steps(&dos).unwrap();
    assert_eq!((steps[2].verdict, steps[2].rule.as_str()), (Verdict::New, "opener"));
    assert_eq!((steps[1].verdict, steps[1].rule.as_str()), (Verdict::Doubtful, "none"));
    assert!(!steps[1].starts(), "doubtful counts as continue");
    assert_eq!(steps[1].question, base.cut_steps(&uno).unwrap()[1].question, "the same pages, the same question");

    // With the automatic reading off, nothing is asked.
    let eyes = FakeEyes { calls: AtomicUsize::new(0) };
    let report = base.cut_background(&eyes, 5).unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked, report.left), (0, 0, 1));
    assert!(base.cut_steps(&uno).unwrap()[1].model_says.is_none());

    // With it on, once: the doubtful step. The answer about the same
    // pages decides the even file too: it weighs more than the rule.
    base.set_background_reading(true).unwrap();
    let report = base.cut_background(&eyes, 5).unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked, report.left), (1, 1, 0));
    for hash in [&uno, &dos] {
        let step = &base.cut_steps(hash).unwrap()[1];
        assert!(step.starts());
        assert_eq!(step.suggested_type(), Some("recibo"));
    }

    // Another file with the same two pages takes the answer kept, while
    // its rules run, without asking; the step with the answer keeps it,
    // and in this even file its other step goes on.
    dir.write("o/tres.pdf", &build(&[opener, x, y]));
    settle(&base, &origin.id);
    let steps = base.cut_steps(&hash_of(&base, "tres.pdf")).unwrap();
    assert_eq!((steps[2].verdict, steps[2].model_says), (Verdict::Doubtful, Some(true)));
    assert!(steps[2].starts());
    assert_eq!((steps[1].verdict, steps[1].rule.as_str()), (Verdict::Continue, "uniform"));
    let report = base.cut_background(&eyes, 5).unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked, report.left), (1, 0, 0));
}

/// A PDF whose last page is of another paper: same length, so the
/// cross-reference table stays right.
fn with_last_page_taller(mut pdf: Vec<u8>) -> Vec<u8> {
    let (from, to) = (b"/MediaBox [0 0 612 792]", b"/MediaBox [0 0 612 999]");
    let at = pdf.windows(from.len()).rposition(|w| w == from).unwrap();
    pdf[at..at + to.len()].copy_from_slice(to);
    pdf
}

#[test]
fn a_file_with_pages_of_two_sizes_keeps_its_doubtful_steps() {
    let dir = TempDir::new();
    dir.write("o/dos.pdf", &with_last_page_taller(build(&[prose("una"), prose("otra")])));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    settle(&base, &origin.id);
    let steps = base.cut_steps(&hash_of(&base, "dos.pdf")).unwrap();
    assert_eq!((steps[1].verdict, steps[1].rule.as_str()), (Verdict::Doubtful, "none"));
}

#[test]
fn in_an_even_file_the_step_after_a_signed_page_stays_doubtful() {
    let signed = page(
        &[
            "Texto inventado de la prueba del cortador, un recibo, con bastantes letras",
            "para que la extraccion lo lea como una pagina digital y no como un escaneo,",
            "y al final un renglon para firmar:",
            "______________________________",
            "Firma",
        ],
        0.0,
    );
    let dir = TempDir::new();
    dir.write("o/tres.pdf", &build(&[prose("una"), signed, prose("otra")]));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    settle(&base, &origin.id);
    let steps = base.cut_steps(&hash_of(&base, "tres.pdf")).unwrap();
    let said: Vec<_> = steps.iter().skip(1).map(|s| (s.verdict, s.rule.as_str())).collect();
    assert_eq!(said, vec![(Verdict::Continue, "uniform"), (Verdict::Doubtful, "none")]);
}

#[test]
fn a_file_is_cut_again_when_the_rules_change_and_one_page_is_not_cut() {
    let dir = TempDir::new();
    dir.write("o/dos.pdf", &build(&[prose("una"), prose("otra")]));
    dir.write("o/sola.pdf", &build(&[prose("sola")]));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    settle(&base, &origin.id);
    assert!(base.cut_steps(&hash_of(&base, "sola.pdf")).unwrap().is_empty());
    let hash = hash_of(&base, "dos.pdf");
    assert_eq!(base.cut_steps(&hash).unwrap().len(), 2);
    assert_eq!(base.cut_pending(10).unwrap(), 0);
    base.conn().execute("UPDATE cut_step SET rules_version = 'cut-rules/0' WHERE resource_hash = ?1", [&hash]).unwrap();
    assert_eq!(base.cut_pending(10).unwrap(), 1);
    let version: String =
        base.conn().query_row("SELECT DISTINCT rules_version FROM cut_step WHERE resource_hash = ?1", [&hash], |row| row.get(0)).unwrap();
    assert_eq!(version, rules_version());
}

/// A compound file (a rule cuts it) with a doubtful step, and a PDF of one
/// document with another.
fn compound_and_single(dir: &TempDir) -> (DocBase, String, String, String) {
    let opener = page(&["Senor Juez:", "Texto inventado de otro escrito que empieza en esta pagina de la prueba."], 0.0);
    dir.write("o/dos.pdf", &build(&[prose("primera hoja"), prose("segunda hoja"), opener.clone()]));
    dir.write("o/cuatro.pdf", &build(&[prose("tercera hoja"), prose("cuarta hoja"), opener]));
    dir.write("o/uno.pdf", &with_last_page_taller(build(&[prose("una"), prose("otra")])));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    settle(&base, &origin.id);
    let (dos, cuatro, uno) = (hash_of(&base, "dos.pdf"), hash_of(&base, "cuatro.pdf"), hash_of(&base, "uno.pdf"));
    (base, dos, cuatro, uno)
}

#[test]
fn by_itself_the_model_is_asked_only_in_the_files_a_rule_cut_and_in_the_round_with_its_cap() {
    let dir = TempDir::new();
    let (base, _dos, _cuatro, uno) = compound_and_single(&dir);
    assert_eq!(base.cut_questions().unwrap(), super::CutQuestions { by_itself: 2, on_request: 1 });
    base.set_background_reading(true).unwrap();
    let eyes = FakeEyes { calls: AtomicUsize::new(0) };

    // The round already asked 50: two more pass its cap, so they wait for
    // a person, and «En curso» says so.
    let state = base.engine_state();
    state.auto_asked(50);
    assert!(base.cut_round_turn(&eyes, 1).unwrap().is_none());
    assert_eq!(eyes.calls.load(Ordering::SeqCst), 0);
    let shown = base.progress(&Scope::default()).unwrap().auto_questions;
    assert_eq!((shown.pending, shown.cut_pending, shown.held), (2, 2, Some("cut")));
    // A stage with nothing to ask leaves that wait as it is.
    state.ran("resolve", 0);
    assert!(base.auto_questions_judge("resolve").unwrap());
    assert_eq!(state.snapshot().auto_held, Some("cut"));

    // A person lets them go: one at a time, counted in the round.
    base.auto_questions_decide(true).unwrap();
    let report = base.cut_round_turn(&eyes, 1).unwrap().unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked, report.left), (1, 1, 1));
    let report = base.cut_round_turn(&eyes, 1).unwrap().unwrap();
    assert_eq!((report.asked, report.left), (1, 0));
    assert_eq!(state.snapshot().auto_asked, 52);
    // The PDF of one document is not asked by itself: it goes on uncertain.
    assert!(base.cut_steps(&uno).unwrap()[1].uncertain());
    assert_eq!(base.cut_questions().unwrap(), super::CutQuestions { by_itself: 0, on_request: 1 });
    let report = base.cut_round_turn(&eyes, 5).unwrap().unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked), (2, 0));
}

#[test]
fn a_person_asks_about_one_pdf_with_the_reading_off_and_it_is_counted() {
    let dir = TempDir::new();
    let (base, _dos, _cuatro, uno) = compound_and_single(&dir);
    let eyes = FakeEyes { calls: AtomicUsize::new(0) };
    assert!(!base.background_reading().unwrap());
    let parts = base.cut_ask_file(&eyes, &uno).unwrap();
    // Only that file; the answer says another document starts: two parts.
    assert_eq!(eyes.calls.load(Ordering::SeqCst), 1);
    assert_eq!(parts.iter().map(|p| (p.first_page, p.last_page)).collect::<Vec<_>>(), vec![(1, 1), (2, 2)]);
    assert!(!base.cut_steps(&uno).unwrap()[1].uncertain());
    assert_eq!(base.cut_questions().unwrap(), super::CutQuestions { by_itself: 2, on_request: 0 });
    // «En curso» says what it was for and what it did.
    let shown = base.progress(&Scope::default()).unwrap().auto_questions;
    assert!(shown.asked_for.iter().any(|a| a.task == super::CUT_TASK && a.asked == 1));
    assert_eq!(shown.cut_effect, crate::docbase::auto_questions::CutEffect { starts: 1, backed: 0, uncertain: 0 });
    // Asked again, the answer is kept: nothing more is sent.
    base.cut_ask_file(&eyes, &uno).unwrap();
    assert_eq!(eyes.calls.load(Ordering::SeqCst), 1);
    // An engine that does not look at images says so.
    struct Blind;
    impl DocumentAsker for Blind {
        fn engine_id(&self) -> String {
            "falso:ciego".into()
        }
        fn ask(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
            Err("blind".into())
        }
    }
    assert!(base.cut_ask_file(&Blind, &uno).is_err());
}

// ── Failures (D-714, D-717), simulated: no real model, no real fault ────

use crate::docbase::failures;

/// An engine that looks at images and answers by a script: one answer or
/// error per call, in order; past the script, a good answer.
struct Scripted {
    id: &'static str,
    script: Vec<Result<&'static str, &'static str>>,
    calls: AtomicUsize,
}

const GOOD: &str = r#"{"second_starts_new_document": false, "first_page_type": "other", "second_page_type": "other", "confidence": "high"}"#;

impl Scripted {
    fn new(id: &'static str, script: Vec<Result<&'static str, &'static str>>) -> Self {
        Scripted { id, script, calls: AtomicUsize::new(0) }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl DocumentAsker for Scripted {
    fn engine_id(&self) -> String {
        self.id.into()
    }

    fn ask(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
        Err("this fake only looks at images".into())
    }

    fn sees_images(&self) -> bool {
        true
    }

    fn ask_with_images(&self, _prompt: &str, _schema: &serde_json::Value, _images: &[std::path::PathBuf]) -> Result<String, String> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        match self.script.get(call) {
            Some(Ok(answer)) => Ok(answer.to_string()),
            Some(Err(error)) => Err(error.to_string()),
            None => Ok(GOOD.into()),
        }
    }
}

/// A base with one PDF whose rules cut before its last page and leave
/// `doubtful` doubtful steps before it, which the thread asks by itself,
/// with the automatic reading on.
fn with_doubtful_steps(doubtful: usize) -> (TempDir, DocBase, String) {
    let words = ["primera hoja", "segunda hoja", "tercera hoja", "cuarta hoja"];
    let mut pages: Vec<Page> = words[..=doubtful].iter().map(|w| prose(w)).collect();
    pages.push(page(&["Senor Juez:", "Texto inventado de otro escrito que empieza en esta pagina de la prueba."], 0.0));
    let dir = TempDir::new();
    dir.write("o/escrito.pdf", &build(&pages));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    settle(&base, &origin.id);
    let hash = hash_of(&base, "escrito.pdf");
    base.set_background_reading(true).unwrap();
    assert_eq!(base.cut_questions().unwrap().by_itself, doubtful);
    (dir, base, hash)
}

fn failure_of(base: &DocBase, item: &str) -> Option<(String, String, String, i64)> {
    base.conn()
        .query_row(
            "SELECT cause, class, scope, attempt FROM failure WHERE item = ?1 AND state <> 'resolved'",
            [item],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .ok()
}

#[test]
fn a_429_pauses_only_that_engine_and_its_files_wait() {
    let (_dir, base, _hash) = with_doubtful_steps(1);
    let limited = Scripted::new("motor:a", vec![Err("stream error: 429 Too Many Requests")]);
    let report = base.cut_background(&limited, 5).unwrap();
    assert_eq!((report.asked, report.failed), (0, 1));
    assert!(report.paused_until.is_some());
    // Written down on the engine, transient, of the whole engine.
    assert_eq!(failure_of(&base, "motor:a"), Some(("rate-limit".into(), "transient".into(), "engine".into(), 1)));
    // The file waits for the answer: its reading does not go on.
    assert!(!base.waiting_for_cut_answers(true).unwrap().is_empty());
    // That engine is not asked again until its next attempt.
    let report = base.cut_background(&limited, 5).unwrap();
    assert_eq!((limited.calls(), report.asked), (1, 0));
    assert!(report.paused_until.is_some());
    // Another engine is.
    let other = Scripted::new("motor:b", vec![]);
    let report = base.cut_background(&other, 5).unwrap();
    assert_eq!((other.calls(), report.asked, report.left), (1, 1, 0));
    assert!(base.waiting_for_cut_answers(true).unwrap().is_empty());
    // The pause of the first is over at its next attempt.
    base.conn().execute("UPDATE failure SET next_at = 0 WHERE item = 'motor:a'", []).unwrap();
    assert_eq!(failures::engine_paused_until(&base.conn(), "motor:a").unwrap(), None);
}

#[test]
fn a_stretch_that_fails_does_not_hold_back_the_next() {
    let (_dir, base, hash) = with_doubtful_steps(2);
    // The first question is too large for the service: permanent, of its
    // stretch alone. The next stretch is asked in the same run.
    let asker = Scripted::new("motor:a", vec![Err("HTTP 413 Request Entity Too Large")]);
    let report = base.cut_background(&asker, 5).unwrap();
    assert_eq!((asker.calls(), report.asked, report.failed), (2, 1, 1));
    assert!(report.paused_until.is_none());
    assert_eq!(failure_of(&base, &format!("{hash}#1")), Some(("too-large".into(), "permanent".into(), "item".into(), 1)));
    let steps = base.cut_steps(&hash).unwrap();
    assert_eq!((steps[1].model_says, steps[2].model_says), (None, Some(false)));
    // Permanent: the thread does not ask it again, and the file waits.
    let report = base.cut_background(&asker, 5).unwrap();
    assert_eq!((asker.calls(), report.asked, report.left), (2, 0, 1));
    assert!(!base.waiting_for_cut_answers(true).unwrap().is_empty());
    // A person who asks for the file asks it again; it is resolved.
    base.cut_ask_file(&asker, &hash).unwrap();
    assert_eq!(asker.calls(), 3);
    assert_eq!(failure_of(&base, &format!("{hash}#1")), None);
}

#[test]
fn a_transient_failure_of_a_stretch_takes_its_next_attempt_and_an_answer_of_another_shape_counts() {
    let (_dir, base, hash) = with_doubtful_steps(1);
    let span = format!("{hash}#1");
    let asker = Scripted::new("motor:a", vec![Ok("no es json"), Ok("no es json"), Ok("no es json")]);
    for attempt in 1..=3 {
        let report = base.cut_background(&asker, 5).unwrap();
        assert_eq!((report.asked, report.failed), (0, 1));
        let class = if attempt < 3 { "transient" } else { "unknown" };
        assert_eq!(failure_of(&base, &span), Some(("bad-answer".into(), class.into(), "item".into(), attempt)));
        // Before its next attempt, it is not asked again.
        let report = base.cut_background(&asker, 5).unwrap();
        assert_eq!((asker.calls(), report.asked + report.failed), (attempt as usize, 0));
        base.conn().execute("UPDATE failure SET next_at = 0 WHERE item = ?1 AND next_at IS NOT NULL", [&span]).unwrap();
    }
    // Unknown after the third equal answer: never again by itself.
    base.cut_background(&asker, 5).unwrap();
    assert_eq!(asker.calls(), 3);
}

#[test]
fn an_engine_that_does_not_look_at_images_is_a_permanent_failure_and_the_files_wait() {
    let (_dir, base, _hash) = with_doubtful_steps(1);
    struct Blind;
    impl DocumentAsker for Blind {
        fn engine_id(&self) -> String {
            "falso:ciego".into()
        }
        fn ask(&self, _prompt: &str, _schema: &serde_json::Value) -> Result<String, String> {
            Err("blind".into())
        }
    }
    base.cut_background(&Blind, 5).unwrap();
    base.cut_background(&Blind, 5).unwrap();
    assert_eq!(failure_of(&base, "falso:ciego"), Some(("engine-no-images".into(), "permanent".into(), "item".into(), 1)));
    assert!(!base.waiting_for_cut_answers(false).unwrap().is_empty());
    // It does not pause that engine for the questions without images.
    assert_eq!(failures::engine_paused_until(&base.conn(), "falso:ciego").unwrap(), None);
    // An engine that looks at images: it is gone.
    base.cut_background(&Scripted::new("motor:a", vec![]), 5).unwrap();
    assert_eq!(failure_of(&base, "falso:ciego"), None);
}

/// A base with one PDF of three pages, read, before its ficha.
fn before_the_ficha() -> (TempDir, DocBase, String, i64) {
    let opener = page(&["Senor Juez:", "Texto inventado de otro escrito que empieza en esta pagina de la prueba."], 0.0);
    let dir = TempDir::new();
    dir.write("o/escrito.pdf", &build(&[prose("primera hoja"), prose("segunda hoja"), opener]));
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    let hash = hash_of(&base, "escrito.pdf");
    let version: i64 = base.conn().query_row("SELECT version_id FROM file WHERE hash = ?1", [&hash], |r| r.get(0)).unwrap();
    (dir, base, hash, version)
}

fn with_fault<T>(fault: &str, f: impl FnOnce() -> T) -> T {
    crate::docbase::page_shapes::SHAPES_FAULT.with(|s| *s.borrow_mut() = Some(fault.into()));
    let out = f();
    crate::docbase::page_shapes::SHAPES_FAULT.with(|s| *s.borrow_mut() = None);
    out
}

fn has_ficha(base: &DocBase, version: i64) -> bool {
    base.conn()
        .query_row("SELECT EXISTS (SELECT 1 FROM derivative WHERE version_id = ?1 AND kind = 'page-shapes')", [version], |r| r.get(0))
        .unwrap()
}

#[test]
fn the_empty_ficha_of_an_encrypted_pdf_is_permanent_and_the_file_is_not_read_whole() {
    let (_dir, base, hash, version) = before_the_ficha();
    let attended = with_fault("Failed to open PDF 'escrito.pdf': PdfiumLibraryInternalError(PasswordError)", || base.profile_pending(10).unwrap());
    assert_eq!(attended, 1);
    assert_eq!(failure_of(&base, &hash), Some(("pdf-encrypted".into(), "permanent".into(), "item".into(), 1)));
    // No empty ficha: it is not tried again, not cut, and it waits; it is
    // never read whole (P-528).
    assert!(!has_ficha(&base, version));
    assert_eq!(base.profile_pending(10).unwrap(), 0);
    assert_eq!(base.cut_pending(10).unwrap(), 0);
    assert!(base.waiting_for_cut().unwrap().contains(&version));
}

#[test]
fn the_empty_ficha_of_a_pdf_in_use_is_transient_and_comes_out_at_its_next_attempt() {
    let (_dir, base, hash, version) = before_the_ficha();
    let locked = "Failed to open PDF 'escrito.pdf': PdfiumLibraryInternalError(FileError) \
                  (The process cannot access the file because it is being used by another process. (os error 32))";
    with_fault(locked, || base.profile_pending(10).unwrap());
    assert_eq!(failure_of(&base, &hash), Some(("file-locked".into(), "transient".into(), "item".into(), 1)));
    let next: i64 = base.conn().query_row("SELECT next_at - last_at FROM failure WHERE item = ?1", [&hash], |r| r.get(0)).unwrap();
    assert_eq!(next, 10 * 60 * 1000);
    assert!(!has_ficha(&base, version));
    assert!(base.waiting_for_cut().unwrap().contains(&version));
    // Before its next attempt nothing is tried; at it, the ficha comes out,
    // the failure is resolved and the file is cut.
    assert_eq!(base.profile_pending(10).unwrap(), 0);
    base.conn().execute("UPDATE failure SET next_at = 0", []).unwrap();
    assert_eq!(base.profile_pending(10).unwrap(), 1);
    assert!(has_ficha(&base, version));
    assert_eq!(failure_of(&base, &hash), None);
    assert_eq!(base.cut_pending(10).unwrap(), 1);
}
