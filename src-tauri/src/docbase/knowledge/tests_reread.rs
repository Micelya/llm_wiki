//! Reading again only when a person asks (D-432), and the checks of the
//! relations between people (D-433 to D-437), over a real base or over a
//! reading alone, with invented documents and a stand-in model (R-24:
//! nothing here comes from the test folder).

use super::extract::{verify, Answer, Reading, RereadTarget};
use super::tests::{answer, base_with, claim, is_knowledge, location, mention, type_answer, FakeModel};
use super::vocabulary::Vocabulary;
use crate::docbase::{DocBase, Visible};

const ACT: &str = "DOC-ACTA. Persona Uno Ejemplo y Persona Dos Ejemplo contrajeron matrimonio el 1 de marzo.";

fn none() -> serde_json::Value {
    serde_json::json!([])
}

fn act_answer() -> String {
    answer(
        vec![
            mention("a", "persona", "Persona Uno Ejemplo", "parte", "Persona Uno Ejemplo y Persona Dos Ejemplo contrajeron", none()),
            mention("b", "persona", "Persona Dos Ejemplo", "parte", "y Persona Dos Ejemplo contrajeron matrimonio", none()),
        ],
        vec![claim("a", "conyuge_de", Some("b"), None, "Persona Uno Ejemplo y Persona Dos Ejemplo contrajeron matrimonio", None)],
    )
}

/// The model of these tests: the act, and the type of any document.
fn careful() -> FakeModel {
    FakeModel::new("falso:1", |prompt| if is_knowledge(prompt) { act_answer() } else { type_answer("acta") })
}

/// A model that must not be asked to read anything.
fn forbidden() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        assert!(!is_knowledge(prompt), "nothing should be read with the model");
        type_answer("acta")
    })
}

fn read(base: &DocBase, model: &FakeModel) -> super::extract::KnowledgeReport {
    base.types_run(model, 50).unwrap();
    base.knowledge_run(model, 50).unwrap()
}

fn evidence_count(base: &DocBase) -> i64 {
    base.conn().query_row("SELECT count(*) FROM knowledge_evidence", [], |r| r.get(0)).unwrap()
}

/// What the base would keep of a reading made with another catalog: its
/// questions were others (so their answers are kept under other keys),
/// and it was checked with another version of the check.
fn as_if_read_with_another_catalog(base: &DocBase) {
    let conn = base.conn();
    let keys: String = conn.query_row("SELECT questions FROM knowledge_extraction", [], |r| r.get(0)).unwrap();
    let keys: Vec<String> = serde_json::from_str(&keys).unwrap();
    let old: Vec<String> = keys.iter().map(|k| format!("{k}-vieja")).collect();
    for (key, old) in keys.iter().zip(&old) {
        conn.execute("UPDATE model_answer SET question = ?2 WHERE question = ?1", [key, old]).unwrap();
    }
    conn.execute(
        "UPDATE knowledge_extraction SET questions = ?1, basis = 'de-antes', prompt_version = 'conocimiento-0', vocabulary = 'r-de-antes'",
        [serde_json::to_string(&old).unwrap()],
    )
    .unwrap();
    // What the earlier check kept of it is gone: the check of now writes it.
    conn.execute("DELETE FROM knowledge_evidence", []).unwrap();
}

#[test]
fn another_catalog_asks_the_model_nothing_and_checks_the_readings_again_from_the_answers_kept() {
    let (_dir, base) = base_with(&[("acta.txt", ACT)]);
    assert_eq!(read(&base, &careful()).asked, 1);
    as_if_read_with_another_catalog(&base);
    assert_eq!(evidence_count(&base), 0);

    // Nothing to ask: the reading stands, of an earlier version.
    let status = base.knowledge_status(&Visible::All).unwrap();
    assert_eq!((status.extracted, status.older, status.stale, status.pending), (1, 1, 0, 0), "{status:?}");
    assert_eq!(base.knowledge_plan().unwrap().documents, 0);

    // The background work checks it again from the answer kept, asking
    // nothing; then there is nothing left to check.
    let report = read(&base, &forbidden());
    assert_eq!((report.asked, report.rechecked, report.extracted), (0, 1, 0), "{report:?}");
    assert_eq!(evidence_count(&base), 1);
    assert_eq!(base.knowledge_recheck(10).unwrap(), 0);
    let found = base.knowledge_of(&Visible::All, location(&base, "acta.txt")).unwrap();
    assert_eq!((found.state.as_str(), found.older, found.reread), ("done", true, false));
}

#[test]
fn a_person_asks_to_read_again_and_the_model_is_asked_anew_once() {
    let (_dir, base) = base_with(&[("acta.txt", ACT)]);
    read(&base, &careful());
    std::thread::sleep(std::time::Duration::from_millis(5));

    let document = RereadTarget { location_id: Some(location(&base, "acta.txt")), ..Default::default() };
    let plan = base.knowledge_reread_plan(&document).unwrap();
    assert_eq!((plan.documents, plan.questions, plan.requested), (1, 1, 0));
    assert_eq!(base.knowledge_request_reread(&document).unwrap(), 1);
    // The request is kept in the base, not in memory.
    let basis: String = base.conn().query_row("SELECT basis FROM knowledge_extraction", [], |r| r.get(0)).unwrap();
    assert!(basis.starts_with("releer:"), "{basis}");
    assert_eq!(base.knowledge_rereads_waiting().unwrap(), 1);
    assert_eq!(base.knowledge_reread_plan(&RereadTarget::default()).unwrap().requested, 1);
    let found = base.knowledge_of(&Visible::All, location(&base, "acta.txt")).unwrap();
    assert!(found.reread);

    // The model is asked again, even for the question it answered before.
    std::thread::sleep(std::time::Duration::from_millis(5));
    let report = base.knowledge_run_requested(&careful(), 50).unwrap();
    assert_eq!((report.asked, report.extracted), (1, 1), "{report:?}");
    let found = base.knowledge_of(&Visible::All, location(&base, "acta.txt")).unwrap();
    assert_eq!((found.state.as_str(), found.older, found.reread), ("done", false, false));
    assert_eq!(base.knowledge_rereads_waiting().unwrap(), 0);
    // Once.
    assert_eq!(read(&base, &forbidden()).asked, 0);
    // A folder or the whole archive is asked the same way.
    assert_eq!(base.knowledge_request_reread(&RereadTarget::default()).unwrap(), 1);
    assert_eq!(base.knowledge_rereads_waiting().unwrap(), 1);
}

fn reading_of(body: &str, said: String) -> Reading {
    let answer: Answer = serde_json::from_str(&said).unwrap();
    verify(&Vocabulary::builtin(), body, &[], &[answer])
}

fn kept(reading: &Reading) -> Vec<(String, String, String)> {
    let name = |reference: &str| reading.mentions.iter().find(|m| m.reference == reference).map(|m| m.written.clone()).unwrap_or_default();
    reading
        .evidence
        .iter()
        .filter(|e| e.object_mention.is_some())
        .map(|e| (name(&e.subject), e.predicate.clone(), name(e.object_mention.as_deref().unwrap())))
        .collect()
}

fn reasons(reading: &Reading) -> Vec<String> {
    reading.discarded.iter().filter(|d| d.what == "claim").map(|d| d.reason.clone()).collect()
}

#[test]
fn a_relation_between_two_people_names_both_and_is_kept_once() {
    let body = "Acta de matrimonio de Persona Tres Ejemplo y Persona Cuatro Ejemplo, padre y madre de Persona Cinco Ejemplo. \
De la unión de Persona Tres Ejemplo y Persona Cuatro Ejemplo nacieron dos hijos, a saber: Persona Cinco Ejemplo y otro. \
En mi carácter de cónyuge de Persona Seis Ejemplo, firma Persona Ocho Ejemplo.";
    let p = |r: &str, written: &str, role: &str, quote: &str| mention(r, "persona", written, role, quote, none());
    let said = answer(
        vec![
            p("p3", "Persona Tres Ejemplo", "otro", "Acta de matrimonio de Persona Tres Ejemplo"),
            p("p4", "Persona Cuatro Ejemplo", "otro", "y Persona Cuatro Ejemplo, padre y madre"),
            p("p5", "Persona Cinco Ejemplo", "otro", "padre y madre de Persona Cinco Ejemplo"),
            p("p6", "Persona Seis Ejemplo", "otro", "cónyuge de Persona Seis Ejemplo"),
            p("p8", "Persona Ocho Ejemplo", "firmante", "firma Persona Ocho Ejemplo"),
        ],
        vec![
            claim("p3", "conyuge_de", Some("p4"), None, "Acta de matrimonio de Persona Tres Ejemplo y Persona Cuatro Ejemplo", None),
            // The people moved one place: the quote does not name the subject.
            claim("p5", "conyuge_de", Some("p3"), None, "Acta de matrimonio de Persona Tres Ejemplo y Persona Cuatro Ejemplo", None),
            // A child named after the union of the parents.
            claim(
                "p5",
                "conyuge_de",
                Some("p4"),
                None,
                "De la unión de Persona Tres Ejemplo y Persona Cuatro Ejemplo nacieron dos hijos, a saber: Persona Cinco Ejemplo",
                None,
            ),
            // The first person is whoever signs.
            claim("p8", "conyuge_de", Some("p6"), None, "En mi carácter de cónyuge de Persona Seis Ejemplo", None),
            // The other side of a kinship is kept as the kinship, once.
            claim(
                "p5",
                "hijo_de",
                Some("p3"),
                None,
                "Acta de matrimonio de Persona Tres Ejemplo y Persona Cuatro Ejemplo, padre y madre de Persona Cinco Ejemplo",
                None,
            ),
        ],
    );
    let reading = reading_of(body, said);
    let t = |s: &str, p: &str, o: &str| (s.to_string(), p.to_string(), o.to_string());
    assert_eq!(
        kept(&reading),
        [
            t("Persona Tres Ejemplo", "conyuge_de", "Persona Cuatro Ejemplo"),
            t("Persona Ocho Ejemplo", "conyuge_de", "Persona Seis Ejemplo"),
            t("Persona Tres Ejemplo", "progenitor_de", "Persona Cinco Ejemplo"),
        ]
    );
    let mut why = reasons(&reading);
    why.sort();
    assert_eq!(why, ["sujeto-fuera-de-la-cita", "vinculo-de-otros"]);
}

#[test]
fn representing_is_said_where_the_words_of_it_are() {
    let body = "Persona Uno Ejemplo (T° 1 F° 2, Colegio de Ejemplo), en representación de Persona Dos Ejemplo, se presenta. \
El Juzgado Uno de Ejemplo libra oficio al Juzgado Dos de Ejemplo a fin de que informe. \
Ejemplo Uno S.A., representada por Persona Tres Ejemplo.";
    let said = answer(
        vec![
            mention("p1", "persona", "Persona Uno Ejemplo", "apoderado", "Persona Uno Ejemplo (T° 1 F° 2", none()),
            mention("c", "organizacion", "Colegio de Ejemplo", "otro", "F° 2, Colegio de Ejemplo)", none()),
            mention("p2", "persona", "Persona Dos Ejemplo", "parte", "en representación de Persona Dos Ejemplo", none()),
            mention("j1", "organizacion", "Juzgado Uno de Ejemplo", "otro", "El Juzgado Uno de Ejemplo libra oficio", none()),
            mention("j2", "organizacion", "Juzgado Dos de Ejemplo", "otro", "al Juzgado Dos de Ejemplo a fin", none()),
            mention("o", "organizacion", "Ejemplo Uno S.A.", "parte", "Ejemplo Uno S.A., representada por", none()),
            mention("p3", "persona", "Persona Tres Ejemplo", "apoderado", "representada por Persona Tres Ejemplo", none()),
        ],
        vec![
            claim("p1", "representa_a", Some("p2"), None, "Persona Uno Ejemplo (T° 1 F° 2, Colegio de Ejemplo), en representación de Persona Dos Ejemplo", None),
            // Its registration names the bar before "en representación de".
            claim("p1", "representa_a", Some("c"), None, "Persona Uno Ejemplo (T° 1 F° 2, Colegio de Ejemplo), en representación de Persona Dos Ejemplo", None),
            // A court that asks another for a report does not represent it.
            claim("j1", "representa_a", Some("j2"), None, "El Juzgado Uno de Ejemplo libra oficio al Juzgado Dos de Ejemplo", None),
            claim("p3", "representa_a", Some("o"), None, "Ejemplo Uno S.A., representada por Persona Tres Ejemplo", None),
        ],
    );
    let reading = reading_of(body, said);
    let t = |s: &str, p: &str, o: &str| (s.to_string(), p.to_string(), o.to_string());
    assert_eq!(
        kept(&reading),
        [t("Persona Uno Ejemplo", "representa_a", "Persona Dos Ejemplo"), t("Persona Tres Ejemplo", "representa_a", "Ejemplo Uno S.A.")]
    );
    let mut why = reasons(&reading);
    why.sort();
    assert_eq!(why, ["no-corresponde-al-nombre", "vinculo-fuera-de-la-cita"]);
}

#[test]
fn an_item_of_a_list_is_found_where_the_list_has_it() {
    let body = "Se rechazan las facturas A0001-00000001 con fecha 1 de enero, A0001-00000002 con fecha 2 de enero, \
A0001-00000003 con fecha 3 de enero, recibidas por correo.";
    let said = answer(
        vec![
            mention("x", "asunto", "factura A0001-00000001", "referido", "facturas A0001-00000001 con fecha 1 de enero", none()),
            mention("y", "asunto", "factura A0001-00000003", "referido", "facturas A0001-00000003 con fecha 3 de enero", none()),
            // A quote of words the text does not have stays out.
            mention("z", "asunto", "factura A0001-00000009", "referido", "facturas A0001-00000009 con fecha 9 de enero", none()),
        ],
        vec![],
    );
    let reading = reading_of(body, said);
    let names: Vec<&str> = reading.mentions.iter().map(|m| m.written.as_str()).collect();
    assert_eq!(names, ["factura A0001-00000001", "factura A0001-00000003"]);
    assert_eq!(reading.discarded.len(), 1);
}

const SINGLE: &str = "DOC-SOLTERO. Persona Uno Ejemplo, soltero, se presenta. Persona Uno Ejemplo contrajo matrimonio con Persona Dos Ejemplo.";
const PARTNERS: &str = "DOC-SOCIOS. Persona Tres Ejemplo y Persona Cuatro Ejemplo son socios de Ejemplo Uno S.A.; firma Persona Cinco Ejemplo. \
En mi carácter de apoderado de Persona Seis Ejemplo y de Persona Siete Ejemplo, firma Persona Ocho Ejemplo. \
Persona Seis Ejemplo, en nombre y representación de su padre prefallecido Persona Siete Ejemplo.";

fn doubts_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("acta");
        }
        let p = |r: &str, written: &str, role: &str, quote: &str| mention(r, "persona", written, role, quote, none());
        if prompt.contains("DOC-SOLTERO.") {
            answer(
                vec![p("a", "Persona Uno Ejemplo", "parte", "Persona Uno Ejemplo, soltero"), p("b", "Persona Dos Ejemplo", "otro", "con Persona Dos Ejemplo")],
                vec![
                    claim("a", "estado_civil", None, Some("soltero"), "Persona Uno Ejemplo, soltero", None),
                    claim("a", "conyuge_de", Some("b"), None, "Persona Uno Ejemplo contrajo matrimonio con Persona Dos Ejemplo", None),
                ],
            )
        } else if prompt.contains("DOC-SOCIOS.") {
            answer(
                vec![
                    p("c", "Persona Tres Ejemplo", "otro", "Persona Tres Ejemplo y Persona Cuatro"),
                    p("d", "Persona Cuatro Ejemplo", "otro", "y Persona Cuatro Ejemplo son socios"),
                    mention("o", "organizacion", "Ejemplo Uno S.A.", "otro", "son socios de Ejemplo Uno S.A.", none()),
                    p("e", "Persona Cinco Ejemplo", "firmante", "firma Persona Cinco Ejemplo"),
                    p("f", "Persona Seis Ejemplo", "parte", "apoderado de Persona Seis Ejemplo"),
                    p("g", "Persona Siete Ejemplo", "parte", "y de Persona Siete Ejemplo, firma"),
                    p("h", "Persona Ocho Ejemplo", "apoderado", "firma Persona Ocho Ejemplo"),
                ],
                vec![
                    // The quote names two others and not the subject.
                    claim("e", "socio_de", Some("o"), None, "Persona Tres Ejemplo y Persona Cuatro Ejemplo son socios de Ejemplo Uno S.A.", None),
                    claim("c", "socio_de", Some("o"), None, "Persona Tres Ejemplo y Persona Cuatro Ejemplo son socios de Ejemplo Uno S.A.", None),
                    // "mi" is whoever signs.
                    claim("h", "representa_a", Some("f"), None, "En mi carácter de apoderado de Persona Seis Ejemplo y de Persona Siete Ejemplo", None),
                    // Inheriting in place of a parent is not representing.
                    claim("f", "representa_a", Some("g"), None, "Persona Seis Ejemplo, en nombre y representación de su padre prefallecido Persona Siete Ejemplo", None),
                ],
            )
        } else {
            answer(vec![], vec![])
        }
    })
}

fn certainty_of(base: &DocBase, predicate: &str, subject: &str) -> Option<serde_json::Value> {
    base.conn()
        .query_row(
            "SELECT c.certainty FROM knowledge_claim c JOIN knowledge_entity e ON e.id = c.subject
             WHERE c.predicate = ?1 AND e.name = ?2 AND c.state != 'retired'",
            [predicate, subject],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|c| serde_json::from_str(&c).ok())
}

#[test]
fn a_relation_is_doubtful_when_its_quote_names_others_or_its_document_contradicts_it() {
    let (_dir, base) = base_with(&[("soltero.txt", SINGLE), ("socios.txt", PARTNERS)]);
    read(&base, &doubts_model());
    base.knowledge_resolve(None, 0).unwrap();
    let spouse = certainty_of(&base, "conyuge_de", "Persona Uno Ejemplo").expect("a claim");
    assert_eq!(spouse["level"], "doubtful", "{spouse}");
    assert_eq!(spouse["reasons"], serde_json::json!(["contradicted"]));
    let partner = certainty_of(&base, "socio_de", "Persona Cinco Ejemplo").expect("a claim");
    assert_eq!((partner["level"].clone(), partner["reasons"].clone()), (serde_json::json!("doubtful"), serde_json::json!(["attribution"])));
    assert_eq!(certainty_of(&base, "socio_de", "Persona Tres Ejemplo").expect("a claim")["level"], "quoted");
    assert_eq!(certainty_of(&base, "representa_a", "Persona Ocho Ejemplo").expect("a claim")["level"], "quoted");
    assert!(certainty_of(&base, "representa_a", "Persona Seis Ejemplo").is_none(), "an heir by representation represents nobody");
}

#[test]
fn an_identifier_written_with_more_or_fewer_characters_does_not_keep_two_apart() {
    use super::names::{identifier_value, written_otherwise};
    assert!(written_otherwise(&identifier_value("20-12345678-9"), &identifier_value("12345678-9")));
    assert!(!written_otherwise(&identifier_value("T° 1 F° 23 CPA"), &identifier_value("T° 1 F° 234")));
    assert!(written_otherwise(&identifier_value("M 12.345.678"), &identifier_value("12.345.678")));
    assert_eq!(identifier_value("Tº 12 Fº 345"), identifier_value("T 12 F 345"));
    // Too short to say anything, or another number.
    assert!(!written_otherwise("12345", "912345"));
    assert!(!written_otherwise("12345678", "12345679"));
}

#[test]
fn a_new_check_or_a_new_question_change_their_own_digest() {
    let text = include_str!("../../../resources/catalog/knowledge.json");
    let base = Vocabulary::parse(text).unwrap();
    // A word the program checks with: checked again, nothing is asked.
    let checked = Vocabulary::parse(&text.replacen("\"firstPerson\": [\"mi\",", "\"firstPerson\": [\"mio\", \"mi\",", 1)).unwrap();
    assert_ne!(base.reading_digest, checked.reading_digest);
    assert_eq!(base.asking_digest, checked.asking_digest);
    // What the question says: an earlier version.
    let asked = Vocabulary::parse(&text.replacen("Su profesión u ocupación.", "Su profesión, oficio u ocupación.", 1)).unwrap();
    assert_ne!(base.asking_digest, asked.asking_digest);
    assert_ne!(base.reading_digest, asked.reading_digest);
}
