//! How sure the record is of each claim, and decisions of a person that
//! survive reading a document again (D-320 to D-322), over a real base,
//! with invented documents and a stand-in model (R-24: nothing here comes
//! from the test folder).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::decide::IdentityDecision;
use super::tests::{answer, base_with, claim, empty_answer, is_knowledge, mention, refresh, type_answer, FakeModel};
use super::tests_pages::{entity_pages, is_synthesis, page_of, read, synthesis};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};

const D: &str = "DOC-D. Declaración de Persona Nueve Ejemplo, DNI 99.999.999, con domicilio en Calle Siete 700 piso 3, \
firmada ante Persona Diez Ejemplo.";
const E: &str = "DOC-E. Nota de Persona Once Ejemplo, con domicilio en Calle Ocho 800 PB.";
const F: &str = "DOC-F. Recibo de Persona Nueve Ejemplo, DNI 99.999.999, por la entrega de las llaves.";
const G: &str = "DOC-G. Ficha de Persona Doce Ejemplo. Su cónyuge Persona Trece Ejemplo vive en Calle Trece 1300.";
const H: &str = "DOC-H. Nota de Persona Doce Ejemplo, con domicilio en Calle Doce 1200.";
const K: &str = "DOC-K. Carta de Persona Doce Ejemplo, con domicilio en Calle Doce 1200.";

fn ids(kind: &str, value: &str, quote: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": kind, "valor": value, "cita": quote }])
}

/// The reading of each document. `variant` is how a second reading (with
/// another question or another model) writes the same things: 0, as the
/// first one; 1, the names and values written another way; 2, the witness
/// of D left out.
fn reading(prompt: &str, variant: usize) -> String {
    let none = serde_json::json!([]);
    let (mentions, claims) = if prompt.contains("DOC-D.") {
        let nine = if variant == 0 { "Persona Nueve Ejemplo" } else { "Nueve Ejemplo" };
        let domicile = if variant == 0 { "Calle Siete 700 piso 3" } else { "Calle Siete 700" };
        let mut mentions = vec![mention("n", "persona", nine, "parte", "Persona Nueve Ejemplo, DNI 99.999.999", ids("dni", "99.999.999", "DNI 99.999.999"))];
        if variant != 2 {
            mentions.push(mention("t", "persona", "Persona Diez Ejemplo", "testigo", "firmada ante Persona Diez Ejemplo", none.clone()));
        }
        (mentions, vec![claim("n", "domicilio", None, Some(domicile), "con domicilio en Calle Siete 700 piso 3", None)])
    } else if prompt.contains("DOC-E.") {
        let domicile = if variant == 0 { "Calle Ocho 800" } else { "Calle Ocho 800 PB" };
        (
            vec![mention("o", "persona", "Persona Once Ejemplo", "parte", "Nota de Persona Once Ejemplo", none.clone())],
            vec![claim("o", "domicilio", None, Some(domicile), "con domicilio en Calle Ocho 800 PB", None)],
        )
    } else if prompt.contains("DOC-F.") {
        (vec![mention("n", "persona", "Persona Nueve Ejemplo", "parte", "Recibo de Persona Nueve Ejemplo, DNI 99.999.999", ids("dni", "99.999.999", "DNI 99.999.999"))], vec![])
    } else if prompt.contains("DOC-G.") {
        (
            vec![
                mention("d", "persona", "Persona Doce Ejemplo", "parte", "Ficha de Persona Doce Ejemplo", none.clone()),
                mention("t", "persona", "Persona Trece Ejemplo", "otro", "Su cónyuge Persona Trece Ejemplo", none.clone()),
            ],
            // The domicile of the spouse, read as the holder's.
            vec![claim("d", "domicilio", None, Some("Calle Trece 1300"), "Persona Trece Ejemplo vive en Calle Trece 1300", None)],
        )
    } else if prompt.contains("DOC-H.") {
        (
            vec![mention("d", "persona", "Persona Doce Ejemplo", "parte", "Nota de Persona Doce Ejemplo", none.clone())],
            vec![claim("d", "domicilio", None, Some("Calle Doce 1200"), "Persona Doce Ejemplo, con domicilio en Calle Doce 1200", None)],
        )
    } else if prompt.contains("DOC-K.") {
        (
            vec![mention("d", "persona", "Persona Doce Ejemplo", "parte", "Carta de Persona Doce Ejemplo", none.clone())],
            vec![claim("d", "domicilio", None, Some("Calle Doce 1200"), "Persona Doce Ejemplo, con domicilio en Calle Doce 1200", None)],
        )
    } else {
        return empty_answer();
    };
    answer(mentions, claims)
}

fn model(variant: Arc<AtomicUsize>) -> FakeModel {
    FakeModel::new("falso:1", move |prompt| {
        if is_synthesis(prompt) {
            synthesis(prompt)
        } else if is_knowledge(prompt) {
            reading(prompt, variant.load(Ordering::SeqCst))
        } else if prompt.contains("Hay que decidir si cada candidato es la misma entidad") {
            serde_json::json!({ "respuestas": [] }).to_string()
        } else {
            type_answer("contrato")
        }
    })
}

struct Archive {
    dir: TempDir,
    base: DocBase,
    _project: PathBuf,
    model: FakeModel,
    variant: Arc<AtomicUsize>,
    origin: String,
}

impl Archive {
    fn new(files: &[(&str, &str)]) -> Self {
        let (dir, base) = base_with(files);
        dir.folder("General/wiki/sources");
        dir.write("General/schema.md", b"# schema");
        let project = dir.path().join("General");
        crate::docbase::testing::point_general_project_at(&base, &project);
        let variant = Arc::new(AtomicUsize::new(0));
        let model = model(variant.clone());
        read(&base, &model);
        let origin = base.origins(&Visible::All).unwrap()[0].id.clone();
        let archive = Self { dir, base, _project: project, model, variant, origin };
        archive.project();
        archive
    }

    fn project(&self) {
        self.base.knowledge_project(Some(&self.model), 50).unwrap();
    }

    fn content(&self, file: &str) -> i64 {
        self.base
            .conn()
            .query_row("SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1", [file], |r| r.get(0))
            .unwrap_or_else(|_| panic!("no document {file}"))
    }

    /// The document read again because a person asked (D-432): the same
    /// text, and the model asked again (its earlier answers forgotten).
    fn read_again(&self, file: &str, variant: usize) {
        let content = self.content(file);
        self.variant.store(variant, Ordering::SeqCst);
        let location: i64 = self
            .base
            .conn()
            .query_row("SELECT l.id FROM location l JOIN file f ON f.hash = l.file_hash WHERE f.version_id = ?1 LIMIT 1", [content], |r| r.get(0))
            .unwrap();
        let target = super::extract::RereadTarget { location_id: Some(location), ..Default::default() };
        assert_eq!(self.base.knowledge_request_reread(&target).unwrap(), 1);
        let conn = self.base.conn();
        conn.execute("DELETE FROM model_answer WHERE version_id = ?1", [content]).unwrap();
        drop(conn);
        read(&self.base, &self.model);
        self.project();
    }

    fn mention(&self, file: &str, written: &str) -> i64 {
        let content = self.content(file);
        self.base
            .conn()
            .query_row("SELECT id FROM knowledge_mention WHERE version_id = ?1 AND written = ?2", rusqlite::params![content, written], |r| r.get(0))
            .unwrap_or_else(|_| panic!("no mention {written} in {file}"))
    }

    fn entity_of(&self, file: &str, written: &str) -> Option<String> {
        let content = self.content(file);
        self.base
            .conn()
            .query_row(
                "SELECT entity_id FROM knowledge_mention WHERE version_id = ?1 AND written = ?2",
                rusqlite::params![content, written],
                |r| r.get(0),
            )
            .unwrap_or_else(|_| panic!("no mention {written} in {file}"))
    }

    fn evidence(&self, file: &str, predicate: &str) -> i64 {
        let content = self.content(file);
        self.base
            .conn()
            .query_row("SELECT id FROM knowledge_evidence WHERE version_id = ?1 AND predicate = ?2", rusqlite::params![content, predicate], |r| {
                r.get(0)
            })
            .unwrap_or_else(|_| panic!("no {predicate} in {file}"))
    }

    /// The live claims of the entity a mention is in: "predicate:value".
    fn claims_of(&self, file: &str, written: &str) -> Vec<String> {
        let Some(entity) = self.entity_of(file, written) else { return Vec::new() };
        let conn = self.base.conn();
        let mut statement = conn
            .prepare(
                "SELECT predicate || ':' || coalesce(object_value, object_entity) FROM knowledge_claim WHERE subject = ?1 AND state != 'retired' ORDER BY 1",
            )
            .unwrap();
        let rows = statement.query_map([entity], |row| row.get(0)).unwrap();
        let out = rows.collect::<Result<_, _>>().unwrap();
        out
    }

    fn certainty(&self, value: &str) -> serde_json::Value {
        let text: String = self
            .base
            .conn()
            .query_row("SELECT certainty FROM knowledge_claim WHERE object_value = ?1 AND state != 'retired'", [value], |r| r.get(0))
            .unwrap_or_else(|_| panic!("no claim {value}"));
        serde_json::from_str(&text).unwrap()
    }

    fn to_review(&self) -> Vec<(String, Option<String>)> {
        self.base
            .knowledge_attention(&Visible::All)
            .unwrap()
            .decisions_to_review
            .into_iter()
            .map(|r| (r.decision.kind, r.document.map(|d| d.name)))
            .collect()
    }

    fn page(&self, title: &str) -> String {
        page_of(&entity_pages(&self.base), title).to_string()
    }
}

#[test]
fn decisions_of_a_person_survive_reading_the_documents_again() {
    let archive = Archive::new(&[("d.txt", D), ("e.txt", E), ("f.txt", F)]);
    // Joined by their identifier at first.
    assert_eq!(archive.entity_of("d.txt", "Persona Nueve Ejemplo"), archive.entity_of("f.txt", "Persona Nueve Ejemplo"));
    let decide = |decision: IdentityDecision| {
        archive.base.knowledge_decide(&decision, None, None).unwrap();
    };
    // Four decisions: the witness of D and the party of E are the same; the
    // party of D is not the one of F; the domicile read in D is wrong; the
    // domicile of E is wrong.
    decide(IdentityDecision::Join {
        a: format!("mention:{}", archive.mention("d.txt", "Persona Diez Ejemplo")),
        b: format!("mention:{}", archive.mention("e.txt", "Persona Once Ejemplo")),
    });
    let nine_f = archive.entity_of("f.txt", "Persona Nueve Ejemplo").unwrap();
    decide(IdentityDecision::Separate { a: format!("mention:{}", archive.mention("d.txt", "Persona Nueve Ejemplo")), b: format!("entity:{nine_f}") });
    decide(IdentityDecision::Wrong { a: format!("evidence:{}", archive.evidence("d.txt", "domicilio")) });
    let e_claim: i64 = archive.base.conn().query_row("SELECT id FROM knowledge_claim WHERE object_value = 'Calle Ocho 800'", [], |r| r.get(0)).unwrap();
    decide(IdentityDecision::Wrong { a: format!("claim:{e_claim}") });
    let check = |nine: &str| {
        assert_eq!(archive.entity_of("d.txt", "Persona Diez Ejemplo"), archive.entity_of("e.txt", "Persona Once Ejemplo"));
        assert_ne!(archive.entity_of("d.txt", nine), archive.entity_of("f.txt", "Persona Nueve Ejemplo"));
        assert!(archive.claims_of("d.txt", nine).iter().all(|c| !c.starts_with("domicilio")), "{:?}", archive.claims_of("d.txt", nine));
        assert!(archive.claims_of("e.txt", "Persona Once Ejemplo").iter().all(|c| !c.starts_with("domicilio")));
    };
    check("Persona Nueve Ejemplo");
    assert!(archive.to_review().is_empty());

    // Read again with another question: the same things, the same way.
    archive.read_again("d.txt", 0);
    check("Persona Nueve Ejemplo");
    assert!(archive.to_review().is_empty());

    // Read again with another question: the names and values written
    // another way. Every decision goes on to what they name now.
    archive.read_again("d.txt", 1);
    archive.read_again("e.txt", 1);
    check("Nueve Ejemplo");
    assert!(archive.to_review().is_empty(), "{:?}", archive.to_review());
    let moved: i64 =
        archive.base.conn().query_row("SELECT count(*) FROM knowledge_decision WHERE applied_as IS NOT NULL", [], |r| r.get(0)).unwrap();
    assert_eq!(moved, 3, "the separation, the piece and the claim moved; the join did not");

    // A new version of D, another text: the decisions go on to it.
    archive.dir.write("o/d.txt", format!("{D} Se firma un ejemplar.").as_bytes());
    refresh(&archive.base, &archive.origin);
    read(&archive.base, &archive.model);
    archive.project();
    check("Nueve Ejemplo");
    assert!(archive.to_review().is_empty(), "{:?}", archive.to_review());

    // Read again, and the witness of D is no longer read: the join cannot
    // be applied and is not lost in silence; it waits for a person.
    archive.read_again("d.txt", 2);
    assert_eq!(archive.to_review(), vec![("join".to_string(), Some("d.txt".to_string()))]);
    let decisions = archive.base.knowledge_decisions().unwrap();
    assert_eq!(decisions.iter().filter(|d| d.state == "active").count(), 4);
    // The others still apply.
    assert_ne!(archive.entity_of("d.txt", "Nueve Ejemplo"), archive.entity_of("f.txt", "Persona Nueve Ejemplo"));
    assert!(archive.claims_of("d.txt", "Nueve Ejemplo").iter().all(|c| !c.starts_with("domicilio")));

    // The witness comes back: the join applies again and leaves the list.
    archive.read_again("d.txt", 1);
    assert!(archive.to_review().is_empty());
    assert_eq!(archive.entity_of("d.txt", "Persona Diez Ejemplo"), archive.entity_of("e.txt", "Persona Once Ejemplo"));
}

#[test]
fn every_claim_says_how_sure_it_is_and_a_doubtful_one_is_never_written_as_a_fact() {
    let archive = Archive::new(&[("g.txt", G), ("h.txt", H), ("k.txt", K)]);
    // A quote that names someone else and not the subject: doubtful.
    let spouse = archive.certainty("Calle Trece 1300");
    assert_eq!(spouse["level"], "doubtful", "{spouse}");
    assert_eq!(spouse["reasons"], serde_json::json!(["attribution"]));
    // With its quote and nobody's word on it.
    assert_eq!(archive.certainty("Calle Doce 1200")["level"], "quoted");
    let page = archive.page("Persona Doce Ejemplo");
    assert!(page.contains("Cada dato tiene su cita, comprobada en el texto del documento"), "{page}");
    assert!(page.contains("Calle Trece 1300 *(sin confirmar: la cita nombra a otra persona u organización)*"), "{page}");
    // The synthesis was told, and its sentence on it says so.
    let asked = archive.model.asked.lock().unwrap().clone();
    let question = asked.iter().rev().find(|q| is_synthesis(q) && q.contains("Persona Doce Ejemplo")).unwrap();
    assert!(question.contains("Calle Trece 1300 — DUDOSA (la cita nombra a otra persona u organización)"), "{question}");
    assert!(question.contains("Las afirmaciones marcadas DUDOSA no son hechos firmes"));
    let sentence = page.lines().find(|l| l.contains("Según los documentos, Domicilio: Calle Trece 1300")).expect("its sentence");
    assert!(sentence.contains("*(sin confirmar)*"), "{sentence}");
    assert!(!page.contains("verificad"));
    // A question with no doubtful claim does not change.
    let plain = asked.iter().filter(|q| is_synthesis(q) && !q.contains("Persona Doce Ejemplo")).all(|q| !q.contains("DUDOSA"));
    assert!(plain);

    // A person confirms it: confirmed, and the page says who.
    let claim: i64 =
        archive.base.conn().query_row("SELECT id FROM knowledge_claim WHERE object_value = 'Calle Trece 1300'", [], |r| r.get(0)).unwrap();
    archive.base.knowledge_decide(&IdentityDecision::Confirm { a: format!("claim:{claim}") }, None, None).unwrap();
    archive.project();
    assert_eq!(archive.certainty("Calle Trece 1300")["level"], "confirmed");
    assert!(archive.page("Persona Doce Ejemplo").contains("Calle Trece 1300 *(confirmado por una persona)*"));
    // Only claims are confirmed.
    let mention = archive.mention("g.txt", "Persona Doce Ejemplo");
    assert!(archive.base.knowledge_decide(&IdentityDecision::Confirm { a: format!("mention:{mention}") }, None, None).is_err());

    // Said by two documents and marked wrong in one: corrected.
    archive.base.knowledge_decide(&IdentityDecision::Wrong { a: format!("evidence:{}", archive.evidence("k.txt", "domicilio")) }, None, None).unwrap();
    archive.project();
    assert_eq!(archive.certainty("Calle Doce 1200")["level"], "corrected");
    assert!(archive.page("Persona Doce Ejemplo").contains("Calle Doce 1200 *(corregido por una persona)*"));
    // The screen of the entity says it too.
    let detail = archive.base.knowledge_entity(&Visible::All, &archive.entity_of("h.txt", "Persona Doce Ejemplo").unwrap()).unwrap().unwrap();
    let level = |value: &str| detail.claims.iter().find(|c| c.object_value.as_deref() == Some(value)).map(|c| c.certainty.clone());
    assert_eq!(level("Calle Trece 1300").as_deref(), Some("confirmed"));
    assert_eq!(level("Calle Doce 1200").as_deref(), Some("corrected"));
}
