//! The matter of a record brought from a portal: looked for before it is
//! made (`docbase/knowledge/matter_records.rs`). Invented data only.

use std::fs;
use std::path::PathBuf;

use super::*;
use crate::docbase::knowledge::matter_records::{records_of, MatterQuestion};
use crate::docbase::Visible;

/// A record brought into a base of its own: the work folder, its origin,
/// the record's folder inside it and its documents inventoried.
struct Brought {
    _dir: TempDir,
    work: PathBuf,
    base: DocBase,
    origin: String,
    rel: String,
    folder: PathBuf,
}

impl Brought {
    fn new(title: &str) -> Brought {
        let dir = TempDir::new();
        let work = dir.path().join("trabajo");
        let expedientes = work.join("Expedientes");
        let mut portal = FakePortal::new(REFERENCE, title, entries(3, 20));
        walk_with(&expedientes, &store::NoneKnown).run(&mut portal, &key()).unwrap();
        let base = DocBase::open(&dir.path().join("base.sqlite3")).unwrap();
        let (origin, inside) = store::ensure_origin(&base, &work, &expedientes).unwrap();
        base.scan_origin(&origin).unwrap();
        base.hash_pending(100).unwrap();
        let folder = store::record_folder(&expedientes.join("PJN"), &key());
        let rel = store::rel_inside(&inside, "PJN", &key());
        Brought { _dir: dir, work, base, origin, rel, folder }
    }

    fn record(&self) -> RecordFile {
        store::load(&self.folder).unwrap().unwrap()
    }

    fn integrate(&self, record: &mut RecordFile) -> usize {
        store::ensure_matter(&self.base, record, &self.origin, &self.rel).unwrap()
    }

    /// Copies of the record's first `copies` documents and a document of
    /// its own in a folder of a person, inventoried: their contents.
    fn person_folder(&self, copies: usize) -> (Vec<i64>, i64) {
        let own = self.work.join("Clientes").join("Reclamo inventado");
        fs::create_dir_all(&own).unwrap();
        let mut pdfs: Vec<PathBuf> =
            fs::read_dir(&self.folder).unwrap().flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "pdf")).collect();
        pdfs.sort();
        for (n, pdf) in pdfs.iter().take(copies).enumerate() {
            fs::copy(pdf, own.join(format!("copia {n}.pdf"))).unwrap();
        }
        fs::write(own.join("nota propia.pdf"), b"%PDF-1.4 una nota inventada").unwrap();
        self.base.scan_origin(&self.origin).unwrap();
        self.base.hash_pending(100).unwrap();
        let conn = self.base.conn();
        let content = |name: &str| -> i64 {
            conn.query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [name], |r| r.get(0))
                .unwrap()
        };
        ((0..copies).map(|n| content(&format!("copia {n}.pdf"))).collect(), content("nota propia.pdf"))
    }

    /// Its name, state, whether a person named it, and its confirmation.
    fn matter(&self, id: &str) -> (String, String, i64, String) {
        self.base
            .conn()
            .query_row("SELECT name, state, name_by_person, confirmation FROM knowledge_matter WHERE id = ?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap()
    }

    fn records_of(&self, id: &str) -> Vec<String> {
        records_of(&self.base.conn(), id).unwrap().into_iter().map(|r| r.reference).collect()
    }

    fn questions(&self) -> Vec<MatterQuestion> {
        self.base.knowledge_attention(&Visible::All).unwrap().matters_to_join
    }

    fn active_matters(&self) -> i64 {
        self.base.conn().query_row("SELECT count(*) FROM knowledge_matter WHERE state = 'active'", [], |r| r.get(0)).unwrap()
    }
}

fn row(name: &str, state: &str, by_person: i64, confirmation: &str) -> (String, String, i64, String) {
    (name.to_string(), state.to_string(), by_person, confirmation.to_string())
}

#[test]
fn a_record_with_no_matter_gets_one_named_by_its_official_title_and_not_by_a_person() {
    let world = Brought::new(TITLE);
    let mut record = world.record();
    record.data.title = None;
    assert_eq!(world.integrate(&mut record), 3);
    let matter = record.matter_id.clone().unwrap();
    // No official title yet: its number.
    assert_eq!(world.matter(&matter), row("ZZZ 4321/2011", "active", 0, "confirmed"));
    assert_eq!(world.records_of(&matter), ["ZZZ 4321/2011"]);
    assert!(world.questions().is_empty(), "nothing looks like it");
    // The title comes: the name follows it.
    record.data.title = Some("  QUIMERA C/ ONDARRETA S/ INVENTADO ".into());
    assert_eq!(world.integrate(&mut record), 0);
    assert_eq!(world.matter(&matter).0, "QUIMERA C/ ONDARRETA S/ INVENTADO");
    // A name a person gives stays, whatever the title does.
    world.base.knowledge_rename_matter(&matter, "Reclamo de Quimera").unwrap();
    record.data.title = Some("OTRO TITULO INVENTADO".into());
    world.integrate(&mut record);
    assert_eq!(world.matter(&matter).0, "Reclamo de Quimera");
    assert_eq!(world.active_matters(), 1);
}

#[test]
fn a_matter_that_has_the_record_takes_its_documents_without_asking() {
    let world = Brought::new(TITLE);
    let mut record = world.record();
    world.integrate(&mut record);
    let matter = record.matter_id.clone().unwrap();
    // The record's file lost its matter: the base knows it by the record.
    let mut again = world.record();
    again.matter_id = None;
    assert_eq!(world.integrate(&mut again), 0);
    assert_eq!(again.matter_id.as_deref(), Some(matter.as_str()));
    assert_eq!(world.active_matters(), 1);
    assert!(world.questions().is_empty());
}

#[test]
fn a_persons_matter_that_shares_files_is_asked_about_once_and_yes_joins_them_into_it() {
    let world = Brought::new(TITLE);
    let (copies, own) = world.person_folder(2);
    let documents = [format!("content:{}", copies[0]), format!("content:{}", copies[1]), format!("content:{own}")];
    let person = world.base.knowledge_create_matter("Reclamo inventado", &documents).unwrap();
    let mut record = world.record();
    world.integrate(&mut record);
    let made = record.matter_id.clone().unwrap();
    assert_ne!(made, person, "it is made as before, and asked about");
    let questions = world.questions();
    assert_eq!(questions.len(), 1, "{questions:?}");
    let question = &questions[0];
    assert_eq!((question.record_matter_id.as_str(), question.other_matter_id.as_str()), (made.as_str(), person.as_str()));
    assert_eq!((question.shared, question.citing), (2, 0));
    assert_eq!(question.records.iter().map(|r| r.reference.as_str()).collect::<Vec<_>>(), ["ZZZ 4321/2011"]);
    // One question per pair, however many times it is integrated.
    world.integrate(&mut record);
    assert_eq!(world.questions().len(), 1);
    // Yes: one matter, the person's, with its name, the record and every document.
    world.base.knowledge_answer_matter_question(&made, &person, true).unwrap();
    assert_eq!(world.matter(&made).1, "retired");
    assert_eq!(world.matter(&person), row("Reclamo inventado", "active", 1, "confirmed"));
    assert_eq!(world.records_of(&person), ["ZZZ 4321/2011"]);
    let detail = world.base.knowledge_matter(&Visible::All, &person).unwrap().unwrap();
    assert_eq!(detail.documents.len(), 4, "its own, the two copies and the record's third");
    assert_eq!(detail.records.len(), 1);
    assert!(world.questions().is_empty());
    // The next integration goes to it.
    assert_eq!(world.integrate(&mut record), 0);
    assert_eq!(record.matter_id.as_deref(), Some(person.as_str()));
    assert!(world.questions().is_empty());
    assert!(world.base.knowledge_answer_matter_question(&made, &person, true).is_err(), "already answered");
}

#[test]
fn no_is_remembered_and_not_asked_again() {
    let world = Brought::new(TITLE);
    let (copies, _) = world.person_folder(1);
    let person = world.base.knowledge_create_matter("Otro reclamo", &[format!("content:{}", copies[0])]).unwrap();
    let mut record = world.record();
    world.integrate(&mut record);
    let made = record.matter_id.clone().unwrap();
    assert_eq!(world.questions().len(), 1);
    world.base.knowledge_answer_matter_question(&made, &person, false).unwrap();
    world.integrate(&mut record);
    assert!(world.questions().is_empty());
    assert_eq!(world.active_matters(), 2);
    assert_eq!(world.matter(&made).0, TITLE);
}

#[test]
fn a_proposed_matter_that_only_cites_the_number_is_asked_about_and_takes_the_title_when_joined() {
    use crate::docbase::knowledge::vocabulary::vocabulary;
    let world = Brought::new(TITLE);
    let (_, own) = world.person_folder(0);
    {
        let conn = world.base.conn();
        conn.execute(
            "INSERT INTO knowledge_matter (id, name, state, created_at, updated_at) VALUES ('mat-propuesto', 'Cobro inventado', 'active', 1, 1)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO knowledge_matter_document (matter_id, version_id, state, reasons) VALUES ('mat-propuesto', ?1, 'firm', '[]')", [own])
            .unwrap();
        // The note cites the record, and a related one.
        for (n, written) in [(1, "expediente N° 4.321/2011"), (2, "expediente ZZZ 4322/2011")] {
            conn.execute(
                "INSERT INTO knowledge_mention (version_id, ref, kind, written, normalized, role, quote) VALUES (?1, ?2, ?3, ?4, ?5, 'objeto', ?4)",
                rusqlite::params![own, format!("m{n}"), vocabulary().matters.kind, written, crate::docbase::knowledge::quote::words(written).join(" ")],
            )
            .unwrap();
        }
    }
    let mut record = world.record();
    world.integrate(&mut record);
    let made = record.matter_id.clone().unwrap();
    let questions = world.questions();
    assert_eq!(questions.len(), 1, "{questions:?}");
    assert_eq!((questions[0].other_matter_id.as_str(), questions[0].shared, questions[0].citing), ("mat-propuesto", 0, 1));
    assert_eq!(questions[0].other_origin, "proposed");
    world.base.knowledge_answer_matter_question(&made, "mat-propuesto", true).unwrap();
    // The proposal is confirmed by the answer, and named by its record.
    assert_eq!(world.matter("mat-propuesto"), row(TITLE, "active", 0, "confirmed"));
    assert_eq!(world.records_of("mat-propuesto"), ["ZZZ 4321/2011"]);
    assert_eq!(world.matter(&made).1, "retired");
}

#[test]
fn two_integrations_of_the_same_record_at_once_make_one_matter() {
    let world = Brought::new(TITLE);
    let ids: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2)
            .map(|_| {
                let world = &world;
                scope.spawn(move || {
                    let mut record = world.record();
                    world.integrate(&mut record);
                    record.matter_id.unwrap()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert_eq!(ids[0], ids[1]);
    assert_eq!(world.active_matters(), 1);
    let chosen: i64 = world.base.conn().query_row("SELECT count(*) FROM knowledge_matter_choice", [], |r| r.get(0)).unwrap();
    assert_eq!(chosen, 3);
}

#[test]
fn what_the_first_version_left_is_repaired_by_general_rules() {
    let world = Brought::new(TITLE);
    let mut record = world.record();
    let contents: Vec<String> = {
        let conn = world.base.conn();
        let mut statement = conn.prepare("SELECT DISTINCT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash ORDER BY f.version_id").unwrap();
        statement.query_map([], |r| r.get::<_, i64>(0)).unwrap().map(|c| format!("content:{}", c.unwrap())).collect()
    };
    // Two integrations at once, as the first version made them: both as a
    // person's, named with the reference; the second, the one the record's
    // file names, with part of the documents.
    let whole = world.base.knowledge_create_matter(&format!("{TITLE} ({})", record.reference), &contents).unwrap();
    let part = world.base.knowledge_create_matter(&record.reference, &contents[..2]).unwrap();
    // A third a person touched (a document taken out): it stays.
    let touched = world.base.knowledge_create_matter(&record.reference, &contents[..2]).unwrap();
    world.base.knowledge_choose_matter_document(&touched, &contents[1], "out").unwrap();
    // A matter a person named alike, without the record's documents: not the record's.
    let unrelated = world.base.knowledge_create_matter(&record.reference, &[]).unwrap();
    record.matter_id = Some(part.clone());
    world.integrate(&mut record);
    assert_eq!(world.matter(&part).1, "retired", "a copy nobody decided about, all of it in the other");
    assert_eq!(record.matter_id.as_deref(), Some(whole.as_str()));
    assert_eq!(world.matter(&whole), row(TITLE, "active", 0, "confirmed"));
    assert_eq!(world.records_of(&whole), ["ZZZ 4321/2011"]);
    assert_eq!(world.matter(&touched).1, "active");
    assert_eq!(world.matter(&unrelated), row(&record.reference, "active", 1, "confirmed"));
    let questions = world.questions();
    let pairs: Vec<(&str, &str)> = questions.iter().map(|q| (q.record_matter_id.as_str(), q.other_matter_id.as_str())).collect();
    assert_eq!(pairs, [(whole.as_str(), touched.as_str())]);
    // Again: nothing more.
    world.integrate(&mut record);
    assert_eq!(world.questions().len(), 1);
}

#[test]
fn a_reference_cites_a_record_by_its_number_and_year_however_it_is_written() {
    use crate::docbase::knowledge::references::cites;
    for text in ["expte n 4 321 2011", "ZZZ 004321/2011", "4321/11", "causa 7 4321 2011"] {
        assert!(cites(text, 4321, 2011), "{text}");
    }
    for text in ["4321 2012", "14321/2011", "4321", "2011 4321", "43 21x 2011"] {
        assert!(!cites(text, 4321, 2011), "{text}");
    }
}
