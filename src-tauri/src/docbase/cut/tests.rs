//! The cutter over invented PDFs (R-24): the rules always run, a doubtful
//! step is asked only with the automatic reading on, and the same two
//! pages are never asked twice.

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
    let steps = base.cut_steps(&uno).unwrap();
    assert_eq!(steps.iter().map(|s| (s.after, s.verdict)).collect::<Vec<_>>(), vec![(0, Verdict::New), (1, Verdict::Doubtful)]);
    assert!(steps[1].question.is_some());
    assert!(!steps[1].starts(), "doubtful counts as continue");
    let steps = base.cut_steps(&dos).unwrap();
    assert_eq!((steps[2].verdict, steps[2].rule.as_str()), (Verdict::New, "opener"));
    assert_eq!(steps[1].question, base.cut_steps(&uno).unwrap()[1].question, "the same pages, the same question");

    // With the automatic reading off, nothing is asked.
    let eyes = FakeEyes { calls: AtomicUsize::new(0) };
    let report = base.cut_background(&eyes, 5).unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked, report.left), (0, 0, 2));
    assert!(base.cut_steps(&uno).unwrap()[1].model_says.is_none());

    // With it on, once for both files.
    base.set_background_reading(true).unwrap();
    let report = base.cut_background(&eyes, 5).unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked, report.left), (1, 1, 0));
    for hash in [&uno, &dos] {
        let step = &base.cut_steps(hash).unwrap()[1];
        assert!(step.starts());
        assert_eq!(step.suggested_type(), Some("recibo"));
    }

    // Another file with the same two pages takes the answer kept, while
    // its rules run, without asking.
    dir.write("o/tres.pdf", &build(&[opener, x, y]));
    settle(&base, &origin.id);
    let steps = base.cut_steps(&hash_of(&base, "tres.pdf")).unwrap();
    assert_eq!(steps[2].model_says, Some(true));
    // Only its other doubtful step (from its first page) is asked.
    let report = base.cut_background(&eyes, 5).unwrap();
    assert_eq!((eyes.calls.load(Ordering::SeqCst), report.asked, report.left), (2, 1, 0));
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
