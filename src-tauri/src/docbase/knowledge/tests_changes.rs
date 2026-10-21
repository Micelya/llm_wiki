//! Changes and deletions in the knowledge layer, over a real base, with
//! invented documents and a stand-in model (R-24: nothing here comes from
//! the test folder).

use std::path::PathBuf;

use super::decide::IdentityDecision;
use super::tests::{answer, amount, base_with, claim, empty_answer, is_knowledge, mention, refresh, type_answer, FakeModel};
use super::tests_pages::{entity_pages, has_page, is_synthesis, page_of, read, synthesis};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};

const A: &str = "DOC-A. Contrato entre Ejemplo Uno S.A., CUIT 30-11111111-1, y Persona Dos, DNI 22.222.222, \
con domicilio en Calle Uno 100, ante el escribano Persona Ocho.";
const B: &str = "DOC-B. Recibo firmado por Persona Dos, DNI 22.222.222, a favor de Ejemplo Uno S.A. por $ 5.000, \
ante Persona Siete como testigo.";
const C: &str = "DOC-C. Nota de Persona Seis, DNI 66.666.666, con domicilio en Calle Seis 600, parte del acuerdo con Ejemplo Uno S.A.";

fn ids(kind: &str, value: &str, quote: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": kind, "valor": value, "cita": quote }])
}

fn reading(prompt: &str) -> String {
    let none = serde_json::json!([]);
    let (mentions, claims) = if prompt.contains("DOC-A.") {
        (
            vec![
                mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A., CUIT 30-11111111-1", ids("cuit", "30-11111111-1", "CUIT 30-11111111-1")),
                mention("p", "persona", "Persona Dos", "parte", "Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
                mention("e", "persona", "Persona Ocho", "testigo", "ante el escribano Persona Ocho", none.clone()),
            ],
            vec![claim("p", "domicilio", None, Some("Calle Uno 100"), "Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100", None)],
        )
    } else if prompt.contains("DOC-B.") {
        (
            vec![
                mention("p", "persona", "Persona Dos", "firmante", "Recibo firmado por Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222")),
                mention("o", "organizacion", "Ejemplo Uno S.A.", "destinatario", "a favor de Ejemplo Uno S.A.", none.clone()),
                mention("t", "persona", "Persona Siete", "testigo", "ante Persona Siete como testigo", none.clone()),
            ],
            vec![amount("p", "o", "$ 5.000", "cuota", "a favor de Ejemplo Uno S.A. por $ 5.000", None)],
        )
    } else if prompt.contains("DOC-C.") {
        (
            vec![
                mention("s", "persona", "Persona Seis", "parte", "Nota de Persona Seis, DNI 66.666.666", ids("dni", "66.666.666", "DNI 66.666.666")),
                mention("o", "organizacion", "Ejemplo Uno S.A.", "otro", "parte del acuerdo con Ejemplo Uno S.A.", none.clone()),
            ],
            vec![claim("s", "domicilio", None, Some("Calle Seis 600"), "con domicilio en Calle Seis 600", None)],
        )
    } else {
        return empty_answer();
    };
    answer(mentions, claims)
}

fn model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if is_synthesis(prompt) {
            synthesis(prompt)
        } else if is_knowledge(prompt) {
            reading(prompt)
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

fn readings(model: &FakeModel) -> usize {
    model.asked.lock().unwrap().iter().filter(|q| is_knowledge(q)).count()
}

fn syntheses(model: &FakeModel) -> usize {
    model.asked.lock().unwrap().iter().filter(|q| is_synthesis(q)).count()
}

struct Archive {
    dir: TempDir,
    base: DocBase,
    _project: PathBuf,
    model: FakeModel,
    origin: String,
}

impl Archive {
    fn new(files: &[(&str, &str)]) -> Self {
        let (dir, base) = base_with(files);
        dir.folder("General/wiki/sources");
        dir.write("General/schema.md", b"# schema");
        let project = dir.path().join("General");
        base.link_general_project(&project, "General").unwrap();
        let model = model();
        read(&base, &model);
        let origin = base.origins(&Visible::All).unwrap()[0].id.clone();
        let archive = Self { dir, base, _project: project, model, origin };
        archive.project();
        archive
    }

    fn project(&self) -> super::pages::ProjectReport {
        self.base.knowledge_project(Some(&self.model), 50).unwrap()
    }

    /// The files of the origin changed on disk: the base sees it, and the
    /// layer reads and resolves what changed.
    fn refresh(&self) {
        refresh(&self.base, &self.origin);
        read(&self.base, &self.model);
    }

    fn entity(&self, name: &str) -> (String, String) {
        self.base
            .conn()
            .query_row("SELECT id, state FROM knowledge_entity WHERE name = ?1", [name], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap_or_else(|_| panic!("no entity named {name}"))
    }

    fn page_file(&self, id: &str) -> Option<String> {
        self.base.conn().query_row("SELECT path FROM knowledge_page WHERE entity_id = ?1", [id], |row| row.get(0)).ok()
    }

    fn page(&self, title: &str) -> String {
        page_of(&entity_pages(&self.base), title).to_string()
    }

    fn evidence(&self, file: &str, predicate: &str) -> i64 {
        let location = super::tests::location(&self.base, file);
        let known = self.base.knowledge_of(&Visible::All, location).unwrap();
        known.evidence.iter().find(|e| e.predicate == predicate).unwrap_or_else(|| panic!("no {predicate} in {file}")).id
    }

    fn claims_of(&self, id: &str) -> Vec<String> {
        let conn = self.base.conn();
        let mut statement = conn
            .prepare("SELECT predicate || ':' || coalesce(object_value, object_entity) FROM knowledge_claim WHERE subject = ?1 AND state != 'retired' ORDER BY 1")
            .unwrap();
        let rows = statement.query_map([id], |row| row.get(0)).unwrap();
        let out = rows.collect::<Result<_, _>>().unwrap();
        out
    }
}

/// A page without the date it was written.
fn undated(text: &str) -> String {
    text.lines().filter(|l| !l.starts_with("updated:")).collect::<Vec<_>>().join("\n")
}

#[test]
fn a_deleted_document_takes_what_it_said_and_gives_it_back_when_it_returns() {
    let archive = Archive::new(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let (six, _) = archive.entity("Persona Seis");
    let six_page = archive.page_file(&six).expect("a party has a page");
    assert!(archive.page("Ejemplo Uno S.A.").contains("## Documentos (3)"));
    // A person says the domicile read in C is wrong.
    let wrong = archive.evidence("c.txt", "domicilio");
    archive.base.knowledge_decide(&IdentityDecision::Wrong { a: format!("evidence:{wrong}") }, None, None).unwrap();
    read(&archive.base, &archive.model);
    archive.project();
    assert!(archive.claims_of(&six).is_empty());
    let person_before = undated(&archive.page("Persona Dos"));
    let syntheses_before = syntheses(&archive.model);

    // C goes away.
    std::fs::remove_file(archive.dir.path().join("o/c.txt")).unwrap();
    archive.refresh();
    let report = archive.project();
    // Its entity is retired, not deleted; its page goes (no notes, no link to it).
    assert_eq!(archive.entity("Persona Seis"), (six.clone(), "retired".to_string()));
    assert_eq!(archive.page_file(&six), None);
    assert!(!has_page(&archive.base, &six_page));
    // Its page and the page of the document go.
    assert_eq!(report.removed, 2, "{report:?}");
    // Only the pages that change are written: the company loses a document;
    // the person is not touched and no synthesis is asked again.
    assert_eq!(report.written, 1, "{report:?}");
    assert!(archive.page("Ejemplo Uno S.A.").contains("## Documentos (2)"));
    assert_eq!(undated(&archive.page("Persona Dos")), person_before);
    assert_eq!(syntheses(&archive.model), syntheses_before);
    // What the person decided is kept, for when it comes back.
    let decisions = archive.base.knowledge_decisions().unwrap();
    assert_eq!(decisions.iter().filter(|d| d.state == "active").count(), 1);

    // C comes back, with the same text: the same entity, the same page, the
    // decision applies again, and the reading costs nothing.
    let readings_before = readings(&archive.model);
    archive.dir.write("o/c.txt", C.as_bytes());
    archive.refresh();
    archive.project();
    assert_eq!(archive.entity("Persona Seis"), (six.clone(), "active".to_string()));
    assert_eq!(archive.page_file(&six).as_deref(), Some(six_page.as_str()));
    assert!(archive.claims_of(&six).is_empty(), "{:?}", archive.claims_of(&six));
    assert_eq!(readings(&archive.model), readings_before);
    assert!(archive.page("Ejemplo Uno S.A.").contains("## Documentos (3)"));
}

#[test]
fn a_claim_with_other_evidence_stays_when_one_document_goes() {
    let b2 = "DOC-B. Recibo firmado por Persona Dos, DNI 22.222.222, con domicilio en Calle Uno 100, a favor de Ejemplo Uno S.A.";
    let model_b2 = |prompt: &str| -> Option<String> {
        prompt.contains("DOC-B. Recibo firmado por Persona Dos, DNI 22.222.222, con domicilio").then(|| {
            serde_json::json!({
                "menciones": [mention("p", "persona", "Persona Dos", "firmante", "Recibo firmado por Persona Dos, DNI 22.222.222", ids("dni", "22.222.222", "DNI 22.222.222"))],
                "afirmaciones": [claim("p", "domicilio", None, Some("Calle Uno 100"), "con domicilio en Calle Uno 100", None)]
            })
            .to_string()
        })
    };
    let (dir, base) = base_with(&[("a.txt", A), ("b.txt", b2)]);
    let asker = FakeModel::new("falso:1", move |prompt| {
        if is_knowledge(prompt) {
            model_b2(prompt).unwrap_or_else(|| reading(prompt))
        } else {
            type_answer("contrato")
        }
    });
    read(&base, &asker);
    let (two, _): (String, String) =
        base.conn().query_row("SELECT id, state FROM knowledge_entity WHERE name = 'Persona Dos'", [], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
    let evidence = |id: &str| -> i64 {
        base.conn()
            .query_row(
                "SELECT count(*) FROM knowledge_evidence e JOIN knowledge_claim c ON c.id = e.claim_id WHERE c.subject = ?1 AND c.predicate = 'domicilio'",
                [id],
                |r| r.get(0),
            )
            .unwrap()
    };
    assert_eq!(evidence(&two), 2);
    std::fs::remove_file(dir.path().join("o/a.txt")).unwrap();
    let origin = base.origins(&Visible::All).unwrap()[0].id.clone();
    refresh(&base, &origin);
    read(&base, &asker);
    // The claim stays, with the evidence that is left.
    assert_eq!(evidence(&two), 1);
    // The entity only A named is retired.
    let eight: String = base.conn().query_row("SELECT state FROM knowledge_entity WHERE name = 'Persona Ocho'", [], |r| r.get(0)).unwrap();
    assert_eq!(eight, "retired");
}

#[test]
fn a_new_version_keeps_its_entities_and_what_a_person_decided() {
    let archive = Archive::new(&[("a.txt", A), ("b.txt", B)]);
    let (two, _) = archive.entity("Persona Dos");
    let (eight, _) = archive.entity("Persona Ocho");
    let eight_page = archive.page_file(&eight).expect("a witness has a page");
    let wrong = archive.evidence("a.txt", "domicilio");
    archive.base.knowledge_decide(&IdentityDecision::Wrong { a: format!("evidence:{wrong}") }, None, None).unwrap();
    read(&archive.base, &archive.model);
    assert!(!archive.claims_of(&two).iter().any(|c| c.starts_with("domicilio")));
    let readings_before = readings(&archive.model);

    // A new version of A: another text, read again.
    archive.dir.write("o/a.txt", format!("{A} Se firman dos ejemplares.").as_bytes());
    archive.refresh();
    archive.project();
    assert_eq!(readings(&archive.model), readings_before + 1);
    // The entity only A names keeps its id and its page.
    assert_eq!(archive.entity("Persona Ocho"), (eight.clone(), "active".to_string()));
    assert_eq!(archive.page_file(&eight).as_deref(), Some(eight_page.as_str()));
    assert_eq!(archive.entity("Persona Dos").0, two);
    // The decision about the old version holds for the new one.
    assert!(!archive.claims_of(&two).iter().any(|c| c.starts_with("domicilio")), "{:?}", archive.claims_of(&two));
    assert!(archive.claims_of(&two).iter().any(|c| c.starts_with("importe")));
    // Nothing of the old version is left.
    let old_mentions: i64 = archive
        .base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_mention WHERE content_id NOT IN (SELECT id FROM content)", [], |r| r.get(0))
        .unwrap();
    assert_eq!(old_mentions, 0);
}

#[test]
fn an_excluded_document_does_not_count_and_comes_back_without_reading_it_again() {
    let archive = Archive::new(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let (six, _) = archive.entity("Persona Seis");
    let before = archive.base.knowledge_status(&Visible::All).unwrap();
    archive.base.set_excluded(&archive.origin, "c.txt", false, true).unwrap();
    read(&archive.base, &archive.model);
    archive.project();
    assert_eq!(archive.entity("Persona Seis").1, "retired");
    assert!(archive.page("Ejemplo Uno S.A.").contains("## Documentos (2)"));
    let status = archive.base.knowledge_status(&Visible::All).unwrap();
    assert_eq!(status.documents, before.documents - 1);

    let readings_before = readings(&archive.model);
    archive.base.set_excluded(&archive.origin, "c.txt", false, false).unwrap();
    read(&archive.base, &archive.model);
    archive.project();
    assert_eq!(archive.entity("Persona Seis"), (six, "active".to_string()));
    assert!(archive.page("Ejemplo Uno S.A.").contains("## Documentos (3)"));
    assert_eq!(readings(&archive.model), readings_before);
}

#[test]
fn a_copy_joined_with_the_chosen_one_counts_but_the_page_names_the_chosen_one() {
    let copy = format!("{A} Copia simple.");
    let archive = Archive::new(&[("a.txt", A), ("b.txt", B), ("copia.txt", &copy)]);
    assert!(archive.page("Persona Dos").contains("## Documentos (3)"));
    let content = |file: &str| -> i64 {
        archive.base.knowledge_of(&Visible::All, super::tests::location(&archive.base, file)).unwrap().content_id.unwrap()
    };
    let (a, copy) = (content("a.txt"), content("copia.txt"));
    archive
        .base
        .conn()
        .execute(
            "INSERT OR REPLACE INTO similar_pair (a, b, relation, certainty, jaccard, containment, cover, similarity, description,
                 evidence, evidence_hash, engine, computed_at, state, chosen_relation, wiki_source, decided_at)
             VALUES (?1, ?2, 'version', 'alta', 0.9, 0.9, 0.9, 0.9, '', '[]', 'h', 'test', 1, 'joined', 'version', ?3, 1)",
            rusqlite::params![a.min(copy), a.max(copy), a],
        )
        .unwrap();
    super::mark_dirty(&archive.base.conn(), "project", "test").unwrap();
    archive.project();
    let person = archive.page("Persona Dos");
    // Listed once, as the chosen one; its evidence still counts.
    assert!(person.contains("## Documentos (2)"), "{person}");
    assert!(person.contains("|a.txt]] — parte") && !person.contains("copia.txt — parte"), "{person}");
    assert!(person.contains("copia.txt → "), "{person}");
}

#[test]
fn processing_again_without_changes_changes_nothing() {
    let archive = Archive::new(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let pages = entity_pages(&archive.base);
    let asked = archive.model.asked.lock().unwrap().len();
    // Nothing changed: nothing to do.
    assert!(archive.base.knowledge_resolve(Some(&archive.model), 50).unwrap().skipped);
    let report = archive.project();
    assert!(report.skipped, "{report:?}");
    // Forced to work again: the same record, the same pages, nothing asked.
    super::mark_dirty(&archive.base.conn(), "resolve", "test").unwrap();
    let resolved = archive.base.knowledge_resolve(Some(&archive.model), 50).unwrap();
    assert_eq!(resolved.changed, 0, "{resolved:?}");
    super::mark_dirty(&archive.base.conn(), "project", "test").unwrap();
    let report = archive.project();
    assert_eq!((report.written, report.removed, report.blocks_written, report.asked), (0, 0, 0, 0), "{report:?}");
    // Scanning the folder again changes nothing either.
    archive.refresh();
    assert!(archive.project().skipped);
    assert_eq!(entity_pages(&archive.base), pages);
    assert_eq!(archive.model.asked.lock().unwrap().len(), asked);
}

#[test]
fn a_page_whose_entity_leaves_goes_and_the_links_to_it_become_text() {
    let archive = Archive::new(&[("a.txt", A), ("b.txt", B), ("c.txt", C)]);
    let (six, _) = archive.entity("Persona Seis");
    let path = archive.page_file(&six).unwrap();
    let stem = path.rsplit('/').next().unwrap().trim_end_matches(".md").to_string();
    // A text the app does not write links to it: a saved query.
    archive.base.save_query("Nota", "", &format!("Ver [[{stem}|la persona]] y [[{stem}]].\n"), &[]).unwrap();
    std::fs::remove_file(archive.dir.path().join("o/c.txt")).unwrap();
    archive.refresh();
    let report = archive.project();
    // Without notes, the page goes even if another page links to it: the
    // link is left as its text (D-239).
    // (And the page of the document that went.)
    assert_eq!((report.removed, report.kept), (2, 0), "{report:?}");
    assert!(!has_page(&archive.base, &path));
    let note = archive.base.saved_queries().unwrap()[0].answer.clone();
    assert!(note.contains("Ver la persona y Persona Seis."), "{note}");
}
