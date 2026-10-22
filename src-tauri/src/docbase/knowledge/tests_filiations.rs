//! The question that asks for every filiation (D-459): one claim per parent
//! of each person, with a quote that names both and skips nothing; and what
//! the check does with answers in that shape (D-460), over a reading alone or a
//! base, with invented documents and a stand-in model (R-24: nothing here
//! comes from the test folder).

use super::extract::{prompt, verify, Answer, Hints, Reading, RereadTarget, CORRECTION, KNOWLEDGE_PROMPT};
use super::tests::{answer, base_with, claim, is_knowledge, mention, type_answer, FakeModel};
use super::vocabulary::Vocabulary;
use crate::docbase::Visible;

fn none() -> serde_json::Value {
    serde_json::json!([])
}

fn person(reference: &str, written: &str, quote: &str) -> serde_json::Value {
    mention(reference, "persona", written, "otro", quote, none())
}

fn reading_of(body: &str, said: String) -> Reading {
    let answer: Answer = serde_json::from_str(&said).unwrap();
    verify(&Vocabulary::builtin(), body, &[], &[answer])
}

/// The relations kept, by the names of their sides, in order.
fn kept(reading: &Reading) -> Vec<(String, String, String)> {
    let name = |reference: &str| reading.mentions.iter().find(|m| m.reference == reference).map(|m| m.written.clone()).unwrap_or_default();
    let mut out: Vec<(String, String, String)> = reading
        .evidence
        .iter()
        .filter(|e| e.object_mention.is_some())
        .map(|e| (name(&e.subject), e.predicate.clone(), name(e.object_mention.as_deref().unwrap())))
        .collect();
    out.sort();
    out
}

fn reasons(reading: &Reading, what: &str) -> Vec<String> {
    let mut out: Vec<String> = reading.discarded.iter().filter(|d| d.what == what).map(|d| d.reason.clone()).collect();
    out.sort();
    out
}

fn parent(of: &str, child: &str) -> (String, String, String) {
    (of.to_string(), "progenitor_de".to_string(), child.to_string())
}

#[test]
fn the_question_asks_for_each_parent_of_each_person_with_a_quote_that_names_both() {
    assert_eq!(KNOWLEDGE_PROMPT, "conocimiento-7");
    let vocabulary = Vocabulary::builtin();
    let question = prompt(&vocabulary, &Hints::default(), "Texto.", 0, 1);
    // What the question says of every relation.
    for rule in [
        "nunca saltees palabras con «...» o «…»",
        "nunca la misma en los dos lados",
        "Una frase que une a varias personas da una afirmación por cada par",
        "no solo los de su titular",
    ] {
        assert!(question.contains(rule), "the question does not say: {rule}");
    }
    // What it says of each kinship comes from the catalog, with an invented
    // example: one claim per parent, both with the same quote.
    let filiation = vocabulary.predicate("progenitor_de").unwrap();
    assert!(question.contains(&format!("- progenitor_de: {}", filiation.description.trim())));
    for words in ["una afirmación por cada padre y por cada madre", "son dos afirmaciones", "las dos con esa misma cita", "no solo la del titular"] {
        assert!(filiation.description.contains(words), "{words}");
    }
    for id in ["hermano_de", "abuelo_de", "nieto_de", "tio_de", "sobrino_de"] {
        let description = &vocabulary.predicate(id).unwrap().description;
        assert!(description.contains("por el papel que tiene en el documento"), "{id}");
        assert!(question.contains(&format!("- {id}: {}", description.trim())), "{id}");
    }
}

const MARRIAGE: &str = "DOC-ACTA. Contraen matrimonio Persona Uno Ejemplo, hija de Persona Dos Ejemplo y de Persona Tres Ejemplo, \
y Persona Cuatro Ejemplo, hijo de Persona Cinco Ejemplo y de Persona Seis Ejemplo.";
const BRIDE: &str = "Persona Uno Ejemplo, hija de Persona Dos Ejemplo y de Persona Tres Ejemplo";
const GROOM: &str = "Persona Cuatro Ejemplo, hijo de Persona Cinco Ejemplo y de Persona Seis Ejemplo";

/// The answer the new question asks for: each parent of each person of the
/// act, one claim each, the two of a person with the same quote.
fn marriage_answer() -> String {
    answer(
        vec![
            person("p1", "Persona Uno Ejemplo", "matrimonio Persona Uno Ejemplo"),
            person("p2", "Persona Dos Ejemplo", "hija de Persona Dos Ejemplo"),
            person("p3", "Persona Tres Ejemplo", "y de Persona Tres Ejemplo"),
            person("p4", "Persona Cuatro Ejemplo", "y Persona Cuatro Ejemplo"),
            person("p5", "Persona Cinco Ejemplo", "hijo de Persona Cinco Ejemplo"),
            person("p6", "Persona Seis Ejemplo", "y de Persona Seis Ejemplo"),
        ],
        vec![
            claim("p2", "progenitor_de", Some("p1"), None, BRIDE, None),
            claim("p3", "progenitor_de", Some("p1"), None, BRIDE, None),
            claim("p5", "progenitor_de", Some("p4"), None, GROOM, None),
            claim("p6", "progenitor_de", Some("p4"), None, GROOM, None),
        ],
    )
}

#[test]
fn a_child_of_two_parents_gives_two_claims_with_the_same_quote_for_each_person_of_the_act() {
    let reading = reading_of(MARRIAGE, marriage_answer());
    assert_eq!(
        kept(&reading),
        [
            parent("Persona Cinco Ejemplo", "Persona Cuatro Ejemplo"),
            parent("Persona Dos Ejemplo", "Persona Uno Ejemplo"),
            parent("Persona Seis Ejemplo", "Persona Cuatro Ejemplo"),
            parent("Persona Tres Ejemplo", "Persona Uno Ejemplo"),
        ]
    );
    assert!(reading.discarded.is_empty(), "{:?}", reading.discarded);
}

#[test]
fn a_filiation_given_badly_is_corrected_when_the_text_says_it_and_discarded_when_not() {
    let body = "DOC-ACTAS. Persona Uno Ejemplo, hija de Persona Dos Ejemplo. Persona Tres Ejemplo, hijo de Persona Cuatro Ejemplo. \
Persona Cinco Ejemplo, hijo de Persona Seis Ejemplo. Persona Siete Ejemplo, hija de Persona Ocho Ejemplo y de Persona Nueve Ejemplo. \
Padre: Persona Diez Ejemplo. Persona Once Ejemplo, hijo de Persona Doce Ejemplo.";
    let said = answer(
        vec![
            person("p1", "Persona Uno Ejemplo", "Persona Uno Ejemplo, hija"),
            person("p2", "Persona Dos Ejemplo", "hija de Persona Dos Ejemplo"),
            person("p3", "Persona Tres Ejemplo", "Persona Tres Ejemplo, hijo"),
            person("p4", "Persona Cuatro Ejemplo", "hijo de Persona Cuatro Ejemplo"),
            person("p5", "Persona Cinco Ejemplo", "Persona Cinco Ejemplo, hijo"),
            person("p6", "Persona Seis Ejemplo", "hijo de Persona Seis Ejemplo"),
            person("p7", "Persona Siete Ejemplo", "Persona Siete Ejemplo, hija"),
            person("p8", "Persona Ocho Ejemplo", "hija de Persona Ocho Ejemplo"),
            person("p9", "Persona Nueve Ejemplo", "y de Persona Nueve Ejemplo"),
            person("p10", "Persona Diez Ejemplo", "Padre: Persona Diez Ejemplo"),
            person("p11", "Persona Once Ejemplo", "Persona Once Ejemplo, hijo"),
            person("p12", "Persona Doce Ejemplo", "hijo de Persona Doce Ejemplo"),
        ],
        vec![
            // The parent on both sides: the child is who the text says.
            claim("p2", "progenitor_de", Some("p2"), None, "Persona Uno Ejemplo, hija de Persona Dos Ejemplo", None),
            // The parent without the child: the same.
            claim("p4", "progenitor_de", None, None, "Persona Tres Ejemplo, hijo de Persona Cuatro Ejemplo", None),
            // The child given as the parent: turned round.
            claim("p5", "progenitor_de", Some("p6"), None, "Persona Cinco Ejemplo, hijo de Persona Seis Ejemplo", None),
            // Said from the child, with the parent as the child: turned round.
            claim("p12", "hijo_de", Some("p11"), None, "Persona Once Ejemplo, hijo de Persona Doce Ejemplo", None),
            // Well given: kept as they are.
            claim("p8", "progenitor_de", Some("p7"), None, "Persona Siete Ejemplo, hija de Persona Ocho Ejemplo y de Persona Nueve Ejemplo", None),
            claim("p9", "progenitor_de", Some("p7"), None, "Persona Siete Ejemplo, hija de Persona Ocho Ejemplo y de Persona Nueve Ejemplo", None),
            // The child on both sides, of two parents: which one is not known.
            claim("p7", "progenitor_de", Some("p7"), None, "Persona Siete Ejemplo, hija de Persona Ocho Ejemplo y de Persona Nueve Ejemplo", None),
            // The parent on both sides, and nothing says whose: discarded.
            claim("p10", "progenitor_de", Some("p10"), None, "Padre: Persona Diez Ejemplo", None),
        ],
    );
    let reading = reading_of(body, said);
    assert_eq!(
        kept(&reading),
        [
            parent("Persona Cuatro Ejemplo", "Persona Tres Ejemplo"),
            parent("Persona Doce Ejemplo", "Persona Once Ejemplo"),
            parent("Persona Dos Ejemplo", "Persona Uno Ejemplo"),
            parent("Persona Nueve Ejemplo", "Persona Siete Ejemplo"),
            parent("Persona Ocho Ejemplo", "Persona Siete Ejemplo"),
            parent("Persona Seis Ejemplo", "Persona Cinco Ejemplo"),
        ]
    );
    assert_eq!(reasons(&reading, CORRECTION), ["filiacion-dada-vuelta", "filiacion-dada-vuelta", "hijo-de-la-filiacion", "hijo-de-la-filiacion"]);
    assert_eq!(reasons(&reading, "claim"), ["objeto-desconocido", "objeto-desconocido"]);
}

#[test]
fn a_quote_that_skips_words_is_discarded_with_its_reason() {
    let body = "DOC-UNION. Persona Uno Ejemplo y Persona Dos Ejemplo contrajeron matrimonio. De dicha unión nacieron: Persona Tres \
Ejemplo, el 1 de enero, y Persona Cuatro Ejemplo, el 2 de febrero.";
    let said = answer(
        vec![
            person("p1", "Persona Uno Ejemplo", "Persona Uno Ejemplo y Persona Dos Ejemplo"),
            person("p2", "Persona Dos Ejemplo", "y Persona Dos Ejemplo contrajeron"),
            person("p3", "Persona Tres Ejemplo", "nacieron: Persona Tres Ejemplo"),
            person("p4", "Persona Cuatro Ejemplo", "y Persona Cuatro Ejemplo"),
        ],
        vec![
            claim("p1", "progenitor_de", Some("p4"), None, "De dicha unión nacieron: ... Persona Cuatro Ejemplo", None),
            claim("p2", "progenitor_de", Some("p4"), None, "De dicha unión nacieron: Persona Tres Ejemplo… y Persona Cuatro Ejemplo", None),
            // Another word: not in the text.
            claim("p1", "progenitor_de", Some("p3"), None, "De dicha unión nació Persona Tres Ejemplo", None),
            // In the text, but it names the child only.
            claim("p2", "progenitor_de", Some("p3"), None, "De dicha unión nacieron: Persona Tres Ejemplo", None),
            // From the parents' names: kept.
            claim(
                "p1",
                "progenitor_de",
                Some("p3"),
                None,
                "Persona Uno Ejemplo y Persona Dos Ejemplo contrajeron matrimonio. De dicha unión nacieron: Persona Tres Ejemplo",
                None,
            ),
            // The points the text has itself are not words left out.
            claim("p1", "conyuge_de", Some("p2"), None, "Persona Uno Ejemplo y Persona Dos Ejemplo contrajeron matrimonio...", None),
        ],
    );
    let reading = reading_of(body, said);
    assert_eq!(
        kept(&reading),
        [
            ("Persona Uno Ejemplo".to_string(), "conyuge_de".to_string(), "Persona Dos Ejemplo".to_string()),
            parent("Persona Uno Ejemplo", "Persona Tres Ejemplo"),
        ]
    );
    assert_eq!(
        reasons(&reading, "claim"),
        ["cita-con-puntos-suspensivos", "cita-con-puntos-suspensivos", "cita-no-encontrada", "sujeto-fuera-de-la-cita"]
    );
}

#[test]
fn a_base_keeps_each_parent_with_the_same_quote_and_the_new_question_reads_nothing_by_itself() {
    let (_dir, base) = base_with(&[("acta.txt", MARRIAGE)]);
    let model = FakeModel::new("falso:1", |prompt| if is_knowledge(prompt) { marriage_answer() } else { type_answer("acta") });
    base.types_run(&model, 50).unwrap();
    assert_eq!(base.knowledge_run(&model, 50).unwrap().asked, 1);
    let conn = base.conn();
    let mut statement =
        conn.prepare("SELECT quote, count(*) FROM knowledge_evidence WHERE predicate = 'progenitor_de' GROUP BY quote ORDER BY quote").unwrap();
    let rows: Vec<(String, i64)> = statement.query_map([], |r| Ok((r.get(0)?, r.get(1)?))).unwrap().collect::<Result<_, _>>().unwrap();
    assert_eq!(rows, [(GROOM.to_string(), 2), (BRIDE.to_string(), 2)]);
    drop(statement);

    // A reading of the question before stays as it is, of an earlier
    // version: nothing is asked until a person asks to read it again
    // (D-432), and then it takes one question.
    conn.execute("UPDATE knowledge_extraction SET prompt_version = 'conocimiento-6'", []).unwrap();
    drop(conn);
    let status = base.knowledge_status(&Visible::All).unwrap();
    assert_eq!((status.extracted, status.older, status.stale, status.pending), (1, 1, 0, 0), "{status:?}");
    assert_eq!(base.knowledge_plan().unwrap().documents, 0);
    let forbidden = FakeModel::new("falso:1", |prompt| {
        assert!(!is_knowledge(prompt), "nothing should be read with the model");
        type_answer("acta")
    });
    assert_eq!(base.knowledge_run(&forbidden, 50).unwrap().asked, 0);
    let plan = base.knowledge_reread_plan(&RereadTarget::default()).unwrap();
    assert_eq!((plan.documents, plan.questions, plan.requested), (1, 1, 0));
}
