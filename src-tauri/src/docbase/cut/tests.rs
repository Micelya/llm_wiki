//! The cutter over invented PDFs (R-24): the rules always run, a doubtful
//! step is asked only with the automatic reading on, the same two pages
//! are never asked twice, and an even file goes on without asking.

use std::sync::atomic::{AtomicUsize, Ordering};

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
