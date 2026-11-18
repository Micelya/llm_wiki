//! What the reading does with the lines read with low confidence (D-668,
//! E1 of 2 nov): they go in the question, marked, and nothing that comes
//! only from them is kept as a datum. Invented texts (R-24).

use super::extract::{identifiers_by_shape, prompt, verify, Answer, Hints, APPROXIMATE_QUOTE};
use super::tests::mention;
use super::vocabulary::Vocabulary;
use crate::source_volume::bands::NOTE;

const BODY: &str = "## Page 1\n\nACTA DE PRUEBA\nComparece Ana Gómez, DNI 30.123.456, por derecho propio.\n\
[aproximado] Hijo de Xandro Quimbel, DNI 7.607.905\n";

fn ids(kind: &str, value: &str, quote: &str) -> serde_json::Value {
    serde_json::json!([{ "clase": kind, "valor": value, "cita": quote, "rotulo": null }])
}

#[test]
fn a_mention_that_comes_only_from_approximate_text_is_not_a_datum() {
    let answer: Answer = serde_json::from_value(serde_json::json!({
        "menciones": [
            mention("a", "persona", "Ana Gómez", "parte", "Comparece Ana Gómez, DNI 30.123.456", ids("dni", "30.123.456", "Ana Gómez, DNI 30.123.456")),
            mention("b", "persona", "Xandro Quimbel", "otro", "Hijo de Xandro Quimbel, DNI 7.607.905", ids("dni", "7.607.905", "Xandro Quimbel, DNI 7.607.905")),
        ]
    }))
    .unwrap();
    let reading = verify(&Vocabulary::builtin(), BODY, &[], &[answer]);
    let kept: Vec<&str> = reading.mentions.iter().map(|m| m.written.as_str()).collect();
    assert_eq!(kept, ["Ana Gómez"], "{:?}", reading.discarded);
    let left: Vec<(&str, &str)> = reading.discarded.iter().map(|d| (d.said.as_str(), d.reason.as_str())).collect();
    assert!(left.contains(&("Xandro Quimbel", APPROXIMATE_QUOTE)), "{left:?}");
    // The number of the firm text is a hint of the reading; the one read
    // with low confidence is not.
    let hints = identifiers_by_shape(BODY);
    assert!(!hints.iter().any(|h| h.contains("7.607.905") || h.contains("7607905")), "{hints:?}");
}

#[test]
fn the_question_shows_the_approximate_lines_marked_with_a_note_and_a_firm_text_has_the_question_it_had() {
    let vocabulary = Vocabulary::builtin();
    let marked = prompt(&vocabulary, &Hints::default(), BODY, 0, 1);
    assert!(marked.contains(NOTE) && marked.contains("[aproximado] Hijo de Xandro Quimbel"));
    let firm = crate::source_volume::bands::firm(BODY);
    assert!(!prompt(&vocabulary, &Hints::default(), &firm, 0, 1).contains(NOTE));
}
