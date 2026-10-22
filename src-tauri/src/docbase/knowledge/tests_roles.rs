//! A side of a relation named by the role it has in its document (D-456),
//! the child of a filiation (D-457), and what the model said of a group
//! of a document read again (D-458), over a reading alone or a real base,
//! with invented documents and a stand-in model (R-24: nothing here comes
//! from the test folder).

use super::extract::{verify, Answer, Reading, CORRECTION};
use super::tests::{answer, base_with, claim, is_knowledge, mention, type_answer, FakeModel};
use super::vocabulary::Vocabulary;
use crate::docbase::DocBase;

fn none() -> serde_json::Value {
    serde_json::json!([])
}

fn person(reference: &str, written: &str, role: &str, quote: &str) -> serde_json::Value {
    mention(reference, "persona", written, role, quote, none())
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

fn reasons(reading: &Reading, what: &str) -> Vec<String> {
    let mut out: Vec<String> = reading.discarded.iter().filter(|d| d.what == what).map(|d| d.reason.clone()).collect();
    out.sort();
    out
}

fn t(s: &str, p: &str, o: &str) -> (String, String, String) {
    (s.to_string(), p.to_string(), o.to_string())
}

#[test]
fn a_side_named_by_the_role_it_has_in_the_document_is_named() {
    let body = "DOC-SUC. Autos: EJEMPLO, PERSONA UNO s/ sucesión. Consta el fallecimiento de Persona Uno Ejemplo. \
Que el causante contrajo matrimonio con Persona Dos Ejemplo. En mi carácter de apoderado de Persona Tres Ejemplo, firma \
Persona Cuatro Ejemplo. Que mis mandantes son hermanos de Persona Cinco Ejemplo.";
    let said = answer(
        vec![
            mention("x", "asunto", "EJEMPLO, PERSONA UNO s/ sucesión", "referido", "Autos: EJEMPLO, PERSONA UNO s/ sucesión", none()),
            person("p1", "Persona Uno Ejemplo", "otro", "el fallecimiento de Persona Uno Ejemplo"),
            person("p2", "Persona Dos Ejemplo", "otro", "matrimonio con Persona Dos Ejemplo"),
            person("p3", "Persona Tres Ejemplo", "parte", "apoderado de Persona Tres Ejemplo"),
            person("p4", "Persona Cuatro Ejemplo", "apoderado", "firma Persona Cuatro Ejemplo"),
            person("p5", "Persona Cinco Ejemplo", "otro", "hermanos de Persona Cinco Ejemplo"),
        ],
        vec![
            claim("p1", "causante", Some("x"), None, "Consta el fallecimiento de Persona Uno Ejemplo", None),
            // "El causante" is the one person whose succession it is.
            claim("p1", "conyuge_de", Some("p2"), None, "Que el causante contrajo matrimonio con Persona Dos Ejemplo", None),
            // Someone else is not.
            claim("p5", "conyuge_de", Some("p2"), None, "Que el causante contrajo matrimonio con Persona Dos Ejemplo", None),
            claim("p4", "representa_a", Some("p3"), None, "En mi carácter de apoderado de Persona Tres Ejemplo", None),
            // "Mis mandantes" are the people the signer represents.
            claim("p3", "hermano_de", Some("p5"), None, "Que mis mandantes son hermanos de Persona Cinco Ejemplo", None),
            claim("p2", "hermano_de", Some("p5"), None, "Que mis mandantes son hermanos de Persona Cinco Ejemplo", None),
        ],
    );
    let reading = reading_of(body, said);
    assert_eq!(
        kept(&reading),
        [
            t("Persona Uno Ejemplo", "causante", "EJEMPLO, PERSONA UNO s/ sucesión"),
            t("Persona Cuatro Ejemplo", "representa_a", "Persona Tres Ejemplo"),
            t("Persona Uno Ejemplo", "conyuge_de", "Persona Dos Ejemplo"),
            t("Persona Tres Ejemplo", "hermano_de", "Persona Cinco Ejemplo"),
        ]
    );
    assert_eq!(reasons(&reading, "claim"), ["sujeto-fuera-de-la-cita", "sujeto-fuera-de-la-cita"]);
}

#[test]
fn a_role_of_several_people_does_not_name_one() {
    let body = "DOC-DOS. Autos: EJEMPLO, PERSONA UNO Y OTRA s/ sucesión. Consta el fallecimiento de Persona Uno Ejemplo \
y de Persona Seis Ejemplo. Que el causante contrajo matrimonio con Persona Dos Ejemplo.";
    let said = answer(
        vec![
            mention("x", "asunto", "EJEMPLO, PERSONA UNO Y OTRA s/ sucesión", "referido", "Autos: EJEMPLO, PERSONA UNO Y OTRA s/ sucesión", none()),
            person("p1", "Persona Uno Ejemplo", "otro", "el fallecimiento de Persona Uno Ejemplo"),
            person("p6", "Persona Seis Ejemplo", "otro", "y de Persona Seis Ejemplo"),
            person("p2", "Persona Dos Ejemplo", "otro", "matrimonio con Persona Dos Ejemplo"),
        ],
        vec![
            claim("p1", "causante", Some("x"), None, "Consta el fallecimiento de Persona Uno Ejemplo y de Persona Seis Ejemplo", None),
            claim("p6", "causante", Some("x"), None, "Consta el fallecimiento de Persona Uno Ejemplo y de Persona Seis Ejemplo", None),
            claim("p1", "conyuge_de", Some("p2"), None, "Que el causante contrajo matrimonio con Persona Dos Ejemplo", None),
        ],
    );
    let reading = reading_of(body, said);
    assert_eq!(kept(&reading).len(), 2, "{:?}", kept(&reading));
    assert_eq!(reasons(&reading, "claim"), ["sujeto-fuera-de-la-cita"]);
}

#[test]
fn the_child_of_a_filiation_is_who_the_text_names_right_before_it() {
    let body = "Se inscribe a Persona Seis Ejemplo, nacido el 1 de enero.\nHijo de Persona Siete Ejemplo\n\
y de Persona Ocho Ejemplo\nTestigo: Persona Nueve Ejemplo.\nPartida de Persona Diez Ejemplo, hijo de Persona Once Ejemplo.\n\
Persona Doce Ejemplo\nHija de Persona Trece Ejemplo.\nPersona Catorce Ejemplo, DNI 1.234.567, hijo de Persona Quince Ejemplo.
Acta de Persona Dieciséis Ejemplo, casado con Persona Diecisiete Ejemplo, hijo de Persona Dieciocho Ejemplo.";
    let said = answer(
        vec![
            person("p6", "Persona Seis Ejemplo", "parte", "inscribe a Persona Seis Ejemplo"),
            person("p7", "Persona Siete Ejemplo", "otro", "Hijo de Persona Siete Ejemplo"),
            person("p8", "Persona Ocho Ejemplo", "otro", "y de Persona Ocho Ejemplo"),
            person("p9", "Persona Nueve Ejemplo", "testigo", "Testigo: Persona Nueve Ejemplo"),
            person("p10", "Persona Diez Ejemplo", "parte", "Partida de Persona Diez Ejemplo"),
            person("p11", "Persona Once Ejemplo", "otro", "hijo de Persona Once Ejemplo"),
            person("p12", "Persona Doce Ejemplo", "parte", "Persona Doce Ejemplo"),
            person("p13", "Persona Trece Ejemplo", "otro", "Hija de Persona Trece Ejemplo"),
            person("p14", "Persona Catorce Ejemplo", "parte", "Persona Catorce Ejemplo, DNI"),
            person("p15", "Persona Quince Ejemplo", "otro", "hijo de Persona Quince Ejemplo"),
            person("p16", "Persona Dieciséis Ejemplo", "parte", "Acta de Persona Dieciséis Ejemplo"),
            person("p17", "Persona Diecisiete Ejemplo", "otro", "casado con Persona Diecisiete Ejemplo"),
            person("p18", "Persona Dieciocho Ejemplo", "otro", "hijo de Persona Dieciocho Ejemplo"),
        ],
        vec![
            // The quote names the parent; the child is named right before.
            claim("p7", "progenitor_de", Some("p6"), None, "Hijo de Persona Siete Ejemplo", None),
            // The list of parents goes on.
            claim("p8", "progenitor_de", Some("p6"), None, "y de Persona Ocho Ejemplo", None),
            // Someone named after it is not the child.
            claim("p7", "progenitor_de", Some("p9"), None, "Hijo de Persona Siete Ejemplo", None),
            // The parent given as both sides: the child is who the text says.
            claim("p11", "progenitor_de", Some("p11"), None, "hijo de Persona Once Ejemplo", None),
            // Said from the child.
            claim("p12", "hijo_de", Some("p13"), None, "Hija de Persona Trece Ejemplo", None),
            // Nothing before the words in the quote names anyone: the text
            // before the quote does.
            claim("p15", "progenitor_de", Some("p14"), None, "DNI 1.234.567, hijo de Persona Quince Ejemplo", None),
            // The child given as both sides is not a parent of anyone.
            claim("p14", "progenitor_de", Some("p14"), None, "Persona Catorce Ejemplo, DNI 1.234.567, hijo de Persona Quince Ejemplo", None),
            // The spouse a certificate names before the parents is not the child.
            claim("p18", "progenitor_de", Some("p18"), None, "hijo de Persona Dieciocho Ejemplo", None),
        ],
    );
    let reading = reading_of(body, said);
    assert_eq!(
        kept(&reading),
        [
            t("Persona Siete Ejemplo", "progenitor_de", "Persona Seis Ejemplo"),
            t("Persona Ocho Ejemplo", "progenitor_de", "Persona Seis Ejemplo"),
            t("Persona Once Ejemplo", "progenitor_de", "Persona Diez Ejemplo"),
            t("Persona Trece Ejemplo", "progenitor_de", "Persona Doce Ejemplo"),
            t("Persona Quince Ejemplo", "progenitor_de", "Persona Catorce Ejemplo"),
            t("Persona Dieciocho Ejemplo", "progenitor_de", "Persona Dieciséis Ejemplo"),
        ]
    );
    assert_eq!(reasons(&reading, "claim"), ["objeto-desconocido", "objeto-fuera-de-la-cita"]);
    assert_eq!(reasons(&reading, CORRECTION), ["hijo-de-la-filiacion", "hijo-de-la-filiacion"]);
}

#[test]
fn a_relation_with_the_matter_takes_the_one_named_after_its_subject() {
    let body = "DOC-MAT. Autos: EJEMPLO, PERSONA UNO s/ sucesión. Otros autos: OTRO, PERSONA DOS s/ cobro. \
Por fallecimiento de Persona Uno Ejemplo lo suceden sus hijos.";
    let said = answer(
        vec![
            mention("x", "asunto", "EJEMPLO, PERSONA UNO s/ sucesión", "referido", "Autos: EJEMPLO, PERSONA UNO s/ sucesión", none()),
            mention("y", "asunto", "OTRO, PERSONA DOS s/ cobro", "referido", "Otros autos: OTRO, PERSONA DOS s/ cobro", none()),
            person("p1", "Persona Uno Ejemplo", "otro", "Por fallecimiento de Persona Uno Ejemplo"),
        ],
        vec![
            // No matter given: the matter named after the person.
            claim("p1", "causante", Some("p1"), None, "Por fallecimiento de Persona Uno Ejemplo", None),
            // The two sides the other way round.
            claim("x", "causante", Some("p1"), None, "Autos: EJEMPLO, PERSONA UNO s/ sucesión", None),
        ],
    );
    let reading = reading_of(body, said);
    assert_eq!(kept(&reading), [t("Persona Uno Ejemplo", "causante", "EJEMPLO, PERSONA UNO s/ sucesión")]);
    assert_eq!(reasons(&reading, CORRECTION), ["asunto-del-documento", "lados-dados-vuelta"]);
    assert!(reasons(&reading, "claim").is_empty(), "{:?}", reading.discarded);
}

const WRIT: &str = "DOC-ESCRITO. Persona Cuatro Ejemplo, apoderado, firma este escrito por Persona Tres Ejemplo, parte. Que mis dos \
representados han sido declarados herederos de Persona Uno Ejemplo, en representación de Persona Dos Ejemplo, padre prefallecido \
de mis mandantes.";
const NOTE: &str = "DOC-NOTA. Persona Seis Ejemplo, apoderado, firma esta nota por Persona Cinco Ejemplo, parte. Fueron declarados \
herederos de Persona Uno Ejemplo, en representación de Persona Dos Ejemplo.";

fn writ_model() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let (lawyer, party, quote) = if prompt.contains("DOC-ESCRITO.") {
            (
                ("Persona Cuatro Ejemplo", "Persona Cuatro Ejemplo, apoderado"),
                ("Persona Tres Ejemplo", "por Persona Tres Ejemplo, parte"),
                "Que mis dos representados han sido declarados herederos de Persona Uno Ejemplo, en representación de Persona Dos Ejemplo",
            )
        } else if prompt.contains("DOC-NOTA.") {
            (
                ("Persona Seis Ejemplo", "Persona Seis Ejemplo, apoderado"),
                ("Persona Cinco Ejemplo", "por Persona Cinco Ejemplo, parte"),
                "Fueron declarados herederos de Persona Uno Ejemplo, en representación de Persona Dos Ejemplo",
            )
        } else {
            return answer(vec![], vec![]);
        };
        answer(
            vec![
                person("a", lawyer.0, "apoderado", lawyer.1),
                person("b", party.0, "parte", party.1),
                person("c", "Persona Uno Ejemplo", "otro", "herederos de Persona Uno Ejemplo"),
                person("d", "Persona Dos Ejemplo", "otro", "en representación de Persona Dos Ejemplo"),
            ],
            vec![claim("b", "heredero_por_representacion_de", Some("c"), None, quote, None)],
        )
    })
}

fn level_of(base: &DocBase, predicate: &str, subject: &str) -> Option<serde_json::Value> {
    base.conn()
        .query_row(
            "SELECT c.certainty FROM knowledge_claim c JOIN knowledge_entity e ON e.id = c.subject
             WHERE c.predicate = ?1 AND e.name = ?2 AND c.state != 'retired'",
            [predicate, subject],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
        .map(|c| c["level"].clone())
}

#[test]
fn whom_a_lawyer_writes_for_is_named_by_my_clients() {
    let (_dir, base) = base_with(&[("escrito.txt", WRIT), ("nota.txt", NOTE)]);
    let model = writ_model();
    base.types_run(&model, 50).unwrap();
    base.knowledge_run(&model, 50).unwrap();
    base.knowledge_resolve(None, 0).unwrap();
    // "Mis representados" are the parties of the writ: the quote names its
    // subject, though it names another person too (D-434, D-456).
    assert_eq!(level_of(&base, "heredero_por_representacion_de", "Persona Tres Ejemplo"), Some(serde_json::json!("quoted")));
    // Without them, the quote names another person and not the subject.
    assert_eq!(level_of(&base, "heredero_por_representacion_de", "Persona Cinco Ejemplo"), Some(serde_json::json!("doubtful")));
}

#[test]
fn an_acronym_written_letter_by_letter_is_an_acronym() {
    use super::names::{compare, key, Likeness, ORGANIZATION};
    let v = Vocabulary::builtin();
    let full = key(&v, ORGANIZATION, "Colegio Ejemplo de Notarios");
    assert_eq!(compare(ORGANIZATION, &key(&v, ORGANIZATION, "C.E.N."), &full), Likeness::Maybe("sigla"));
    assert_eq!(compare(ORGANIZATION, &key(&v, ORGANIZATION, "CEN"), &full), Likeness::Maybe("sigla"));
    assert_eq!(compare(ORGANIZATION, &key(&v, ORGANIZATION, "C.E.X."), &full), Likeness::Unlike);
}

const S1: &str = "DOC-S1. Constancia del C.E.N. sobre el trámite.";
const S2: &str = "DOC-S2. Otra constancia del C.E.N. para el expediente.";
const S3: &str = "DOC-S3. Nota del Colegio Ejemplo de Notarios sobre el trámite.";

fn notaries() -> FakeModel {
    FakeModel::new("falso:1", |prompt| {
        if !is_knowledge(prompt) {
            return type_answer("contrato");
        }
        let org = |written: &str, quote: &str| mention("o", "organizacion", written, "emisor", quote, none());
        if prompt.contains("DOC-S1.") {
            answer(vec![org("C.E.N.", "Constancia del C.E.N.")], vec![])
        } else if prompt.contains("DOC-S2.") {
            answer(vec![org("C.E.N.", "constancia del C.E.N.")], vec![])
        } else if prompt.contains("DOC-S3.") {
            answer(vec![org("Colegio Ejemplo de Notarios", "Nota del Colegio Ejemplo de Notarios")], vec![])
        } else {
            answer(vec![], vec![])
        }
    })
}

#[test]
fn what_the_model_said_of_a_group_read_again_holds_for_the_group_that_names_it_now() {
    let (_dir, base) = base_with(&[("s1.txt", S1), ("s2.txt", S2), ("s3.txt", S3)]);
    let model = notaries();
    base.types_run(&model, 50).unwrap();
    base.knowledge_run(&model, 50).unwrap();
    let full = super::tests_relations::content(&base, "s3.txt");
    // The model said once that the acronym, in a document read again since
    // (it does not name it any more), is the college.
    base.conn()
        .execute(
            "INSERT INTO knowledge_verdict (a, b, engine, verdict, reason, question, asked_at) VALUES (?1, ?2, 'falso:1', 'same', 'la sigla', 'q-vieja', 1)",
            ["cluster:9999:organizacion:c e n".to_string(), format!("cluster:{full}:organizacion:colegio ejemplo de notarios")],
        )
        .unwrap();
    let report = base.knowledge_resolve(None, 0).unwrap();
    assert_eq!(report.left, 0, "{report:?}");
    let entities: i64 = base
        .conn()
        .query_row("SELECT count(DISTINCT entity_id) FROM knowledge_mention WHERE kind = 'organizacion'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(entities, 1);
}
