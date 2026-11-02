//! The steps of the process (A1 of 2 nov): their order, the counts against
//! the barrier of today, the gap the barrier does not see, a scope, the
//! steps a person decided and the type a new member of a family takes.
//! Invented PDFs (R-24) and a stand-in model.

use std::sync::atomic::{AtomicUsize, Ordering};

use rusqlite::params;

use super::super::barrier::Barrier;
use super::super::failures::{self, About as FailureAbout, Failure};
use super::super::knowledge::tests::{is_knowledge, type_answer};
use super::super::testing::TempDir;
use super::super::worker::ask_with;
use super::super::DocBase;
use super::{Class, Level, Scope, Step, STEPS};
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::page_shape::test_pdf::{build, page, Page};

/// A model that answers the question of the cut (the next page goes on
/// the same document), the type (a contract) and the reading (nothing).
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
        "falso:proceso".into()
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
            &format!("Texto inventado de la prueba del proceso, {words}, con bastantes letras"),
            "para que la extraccion lo lea como una pagina digital y no como un escaneo,",
            "sin formulas de apertura ni de cierre y sin numeros de pagina ni codigos.",
        ],
        0.0,
    )
}

fn opener() -> Page {
    page(&["Senor Juez:", "Texto inventado de otro escrito que empieza en esta pagina de la prueba."], 0.0)
}

/// A long page of an invented agreement: two of them with one word
/// changed are the same document, near identical.
fn agreement(change: &str) -> Page {
    page(
        &[
            "Acuerdo inventado de servicios entre dos sociedades de la prueba del proceso,",
            &format!("que se obligan a prestar los servicios descritos en el anexo con la diligencia {change},"),
            "a guardar la confidencialidad de la informacion recibida durante toda la vigencia,",
            "a pagar el precio en las fechas convenidas y a resolver sus diferencias de buena fe,",
            "con domicilio constituido en la ciudad inventada de la prueba para todos los efectos,",
            "y a someterse a la jurisdiccion de los tribunales ordinarios de esa misma ciudad.",
        ],
        0.0,
    )
}

/// Everything the thread of this computer does, until nothing is left.
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

fn with_files(dir: &TempDir, files: &[(&str, Vec<Page>)]) -> (DocBase, String) {
    for (name, pages) in files {
        dir.write(&format!("o/{name}"), &build(pages));
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    (base, origin.id)
}

/// A base with these PDFs and all its work without AI done.
fn settled(dir: &TempDir, files: &[(&str, Vec<Page>)]) -> (DocBase, String) {
    let (base, origin) = with_files(dir, files);
    local(&base);
    (base, origin)
}

fn turn(base: &DocBase, model: &Model) {
    ask_with(base, model, &|| {}, &|| {});
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

fn all(base: &DocBase) -> super::ProcessState {
    base.process_state(&Scope::All, false).unwrap()
}

/// The steps without AI that the barrier counts, side by side with it.
/// The barrier does not see the key data that changed after a document was
/// measured (the gap): that is `stale`, the only difference allowed.
fn compare(base: &DocBase, at: &str) {
    let state = all(base);
    let barrier: Barrier = base.barrier().unwrap();
    let p = |step: Step| state.pending(step);
    let stale = super::super::similar::store::stale_by_key_data_now(&base.conn()).unwrap().len();
    let steps = Barrier {
        text: p(Step::FileHash) + p(Step::Text),
        scans: p(Step::Scans),
        shapes: p(Step::Shapes),
        cut: p(Step::Zones) + p(Step::CutRules) + p(Step::Parts),
        similar: p(Step::Signature) + p(Step::Similar) - stale,
    };
    assert_eq!(steps, barrier, "{at}: the steps count what the barrier counts");
    assert_eq!(stale, 0, "{at}: no reading changed any key datum here");
}

#[test]
fn the_eighteen_steps_go_in_order_nine_of_the_file_and_nine_of_the_document() {
    assert_eq!(STEPS.len(), 18);
    for (index, step) in STEPS.iter().enumerate() {
        let about = step.about();
        assert_eq!(about.number as usize, index + 1);
        assert_eq!(Step::from_number(about.number), Some(*step));
        assert_eq!(about.level, if index < 9 { Level::File } else { Level::Document }, "{}", about.name);
        assert_eq!(about.after_the_type, index + 1 > 12, "{}", about.name);
        assert!(!about.name.is_empty() && !about.seal.is_empty() && !about.undone_by.is_empty());
    }
    assert_eq!(Step::from_number(0), None);
    assert_eq!(Step::from_number(19), None);
    let asks: Vec<u8> = STEPS.iter().filter(|s| s.about().class == Class::Asks).map(|s| s.number()).collect();
    assert_eq!(asks, vec![8, 12, 13, 14, 15, 16, 17, 18]);
    assert_eq!(Step::Scans.about().class, Class::BySettings);
}

#[test]
fn the_steps_without_ai_count_what_the_barrier_counts_at_every_moment() {
    let dir = TempDir::new();
    let (base, origin) = with_files(
        &dir,
        &[
            ("escrito.pdf", vec![prose("primera hoja"), prose("segunda hoja"), opener()]),
            ("uno.pdf", vec![prose("una hoja suelta")]),
            ("dos.pdf", vec![prose("otra hoja"), opener(), prose("la ultima hoja")]),
        ],
    );
    compare(&base, "after the sweep");
    assert!(all(&base).pending(Step::FileHash) > 0);
    // One small piece at a time, comparing after each.
    let pieces: [(&str, &dyn Fn() -> usize); 6] = [
        ("hash", &|| base.hash_pending(1).unwrap()),
        ("text", &|| base.extract_pending(1).unwrap()),
        ("ficha", &|| base.profile_pending(1).unwrap()),
        ("cut", &|| base.cut_pending(1).unwrap()),
        ("parts", &|| base.cut_parts_pending(1).unwrap()),
        ("similar", &|| base.similar_step(1).unwrap()),
    ];
    let mut moments = 0;
    for (name, piece) in pieces {
        while piece() > 0 {
            moments += 1;
            compare(&base, name);
        }
    }
    local(&base);
    compare(&base, "settled");
    assert!(moments >= 10, "it compared at {moments} moments");
    assert_eq!(base.barrier().unwrap().left(), 0);

    // A scan waiting, then failed: set aside, as the barrier lets it go.
    let scan = add_scan(&base, &origin);
    compare(&base, "a scan waits");
    assert_eq!(all(&base).pending(Step::Scans), 1);
    let mut failure = Failure::new("recognition", FailureAbout::Version(scan), "prueba");
    failure.cause = failures::cause("file-locked");
    failures::record(&base.conn(), &failure).unwrap();
    compare(&base, "the scan failed");
    let scans = all(&base).step(Step::Scans).unwrap().count;
    assert_eq!((scans.pending, scans.set_aside), (0, 1));

    // A new file in the folder already processed.
    dir.write("o/nuevo.pdf", &build(&[prose("hoja nueva"), opener()]));
    base.scan_origin(&origin).unwrap();
    compare(&base, "a new file");
    local(&base);
    compare(&base, "the new file settled");
}

#[test]
fn an_answer_that_changes_the_key_data_leaves_the_similar_documents_incomplete_in_the_same_moment() {
    let dir = TempDir::new();
    let (base, _) = settled(
        &dir,
        &[
            ("alfa.pdf", vec![prose("la hoja alfa con su texto propio")]),
            ("beta.pdf", vec![prose("la hoja beta con otro texto")]),
            ("gama.pdf", vec![prose("la hoja gama con un texto mas")]),
        ],
    );
    let measured: Vec<i64> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT version_id FROM similar_key_data ORDER BY version_id").unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(measured.len(), 3, "the three documents were measured");
    assert_eq!(all(&base).pending(Step::Similar), 0);

    // The reading of each is kept: it names a party in each one, which
    // changes their key data. Nothing else runs in between.
    for (index, version) in measured.iter().enumerate() {
        base.conn()
            .execute(
                "INSERT INTO knowledge_mention (version_id, ref, kind, written, normalized, role, quote)
                 VALUES (?1, ?2, 'organizacion', ?2, ?3, 'parte', ?2)",
                params![version, format!("Sociedad Inventada {index}"), format!("sociedad inventada {index}")],
            )
            .unwrap();
    }
    // The barrier does not see it: nothing marked the documents yet.
    assert_eq!(base.barrier().unwrap().similar, 0, "the gap of the barrier");
    // The step does, in the same moment, from the base.
    let state = base.process_state(&Scope::All, true).unwrap();
    assert_eq!(state.pending(Step::Similar), 3);
    assert_eq!(state.first_incomplete, Some(Step::Similar), "the next operation measures, it never asks");

    while base.similar_step(10).unwrap() > 0 {}
    let state = all(&base);
    assert_eq!(state.pending(Step::Similar), 0);
    assert!(state.first_incomplete.is_none_or(|s| s > Step::Similar));
}

#[test]
fn a_step_of_the_cut_a_person_decided_is_not_asked_again() {
    let dir = TempDir::new();
    let (base, _) = settled(&dir, &[("escrito.pdf", vec![prose("primera hoja"), prose("segunda hoja"), opener()])]);
    base.set_background_reading(true).unwrap();
    assert_eq!(all(&base).pending(Step::CutQuestion), 1);
    assert_eq!(all(&base).first_incomplete, Some(Step::CutQuestion));

    // A person says the second page goes on the same document.
    let (hash, after): (String, u32) = base
        .conn()
        .query_row("SELECT resource_hash, after_page FROM cut_step WHERE verdict = 'doubtful'", [], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    base.cut_join_with_previous(&hash, after + 1).unwrap();
    assert_eq!(all(&base).pending(Step::CutQuestion), 0, "decided, not pending");
    assert_eq!(base.cut_questions().unwrap(), Default::default());
    let model = Model::default();
    turn(&base, &model);
    assert_eq!(model.images(), 0, "the step a person decided is not asked");
}

#[test]
fn a_scope_counts_only_its_own_and_a_document_takes_the_steps_of_its_file() {
    let dir = TempDir::new();
    let (base, origin) = settled(&dir, &[("uno.pdf", vec![prose("una hoja")])]);
    let (hash, version): (String, i64) =
        base.conn().query_row("SELECT hash, version_id FROM file", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    // The reading off: the questions are set aside, they hold nothing.
    let type_step = all(&base).step(Step::Type).unwrap().clone();
    assert_eq!((type_step.count.pending, type_step.count.set_aside, type_step.held.is_some()), (0, 1, true));

    // A new file, in a folder of its own.
    dir.write("o/nueva/otro.pdf", &build(&[prose("otra hoja"), opener()]));
    base.scan_origin(&origin).unwrap();
    assert_eq!(all(&base).first_incomplete, Some(Step::FileHash));
    let folder = Scope::Folder { origin: origin.clone(), folder: "nueva".into() };
    assert_eq!(base.process_state(&folder, true).unwrap().first_incomplete, Some(Step::FileHash));
    let whole = Scope::Folder { origin: origin.clone(), folder: String::new() };
    assert_eq!(base.process_state(&whole, true).unwrap().first_incomplete, Some(Step::FileHash));
    // The document already there waits for nothing of the new file.
    for scope in [Scope::Document(version), Scope::File(hash.clone())] {
        let state = base.process_state(&scope, false).unwrap();
        assert_eq!(state.first_incomplete, None, "{scope:?}");
        assert_eq!(state.steps.len(), 18);
    }
    // `first_only` stops at the first incomplete step.
    assert_eq!(base.process_state(&Scope::All, true).unwrap().steps.len(), 2);

    // A document of a file of several takes the steps of its file.
    local(&base);
    let parts: Vec<i64> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT a.version_id FROM appearance a WHERE a.selector GLOB 'pages:*'").unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(parts.len(), 2, "the new file has two documents");
    base.conn().execute("DELETE FROM cut_applied", []).unwrap();
    let state = base.process_state(&Scope::Document(parts[0]), true).unwrap();
    assert_eq!(state.first_incomplete, Some(Step::Parts), "its file's parts are its own steps before");
    assert_eq!(base.process_state(&Scope::Document(version), true).unwrap().first_incomplete, None);
}

#[test]
fn a_document_new_to_a_family_already_asked_takes_its_type_without_asking() {
    let dir = TempDir::new();
    let (base, origin) = settled(&dir, &[("acuerdo.pdf", vec![agreement("debida")]), ("acuerdo bis.pdf", vec![agreement("esperada")])]);
    let families: i64 = base.conn().query_row("SELECT count(*) FROM similar_group", [], |row| row.get(0)).unwrap();
    assert_eq!(families, 1, "the two agreements are one family");
    base.set_background_reading(true).unwrap();
    let model = Model::default();
    for _ in 0..6 {
        turn(&base, &model);
    }
    let asked = model.texts();
    assert!(asked > 0, "the family was asked");
    let types = base.document_types_all().unwrap();
    assert_eq!(types.len(), 2);

    // A third agreement joins the family.
    dir.write("o/acuerdo ter.pdf", &build(&[agreement("habitual")]));
    base.scan_origin(&origin).unwrap();
    // Its work without AI types it already: the family's type, no question.
    local(&base);
    let members: i64 = base.conn().query_row("SELECT count(*) FROM similar_group_member", [], |row| row.get(0)).unwrap();
    assert_eq!(members, 3, "the new one is in the family");
    assert_eq!(base.document_types_all().unwrap().len(), 3);
    assert_eq!(all(&base).pending(Step::Type), 0);
    for _ in 0..4 {
        turn(&base, &model);
    }
    let types = base.document_types_all().unwrap();
    assert_eq!(types.len(), 3);
    assert!(types.values().all(|t| t.doc_type == types.values().next().unwrap().doc_type));
    let source: String = base
        .conn()
        .query_row("SELECT source FROM document_type ORDER BY computed_at DESC, version_id DESC LIMIT 1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(source, "family");
    let typed_again: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM model_answer WHERE task = 'type'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(typed_again, 1, "one question of the type, for the family before");
    assert_eq!(all(&base).pending(Step::Type), 0);
}
