//! Passes 4 to 6 over a real base, with invented documents and a model
//! that is a stand-in: it never calls a real engine (R-24: nothing here
//! comes from the test folder).

use std::sync::Mutex;

use crate::docbase::query::DocumentFilter;
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};
use crate::source_volume::asking::{AskConfig, DocumentAsker};

/// Answers each question with what `answer` says for it, and remembers
/// every question it was asked.
struct FakeModel {
    answer: Box<dyn Fn(&str) -> String + Send + Sync>,
    asked: Mutex<Vec<String>>,
}

impl FakeModel {
    fn new(answer: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
        Self { answer: Box::new(answer), asked: Mutex::new(Vec::new()) }
    }

    fn calls(&self) -> usize {
        self.asked.lock().unwrap().len()
    }

    fn questions(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

impl DocumentAsker for FakeModel {
    fn engine_id(&self) -> String {
        "falso:1".to_string()
    }

    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        assert_eq!(schema["additionalProperties"], false, "the answer's shape is strict");
        self.asked.lock().unwrap().push(prompt.to_string());
        Ok((self.answer)(prompt))
    }
}

fn type_answer(tipo: &str, etapa: Option<&str>, others: &[(i64, &str)]) -> String {
    serde_json::json!({
        "tipo": tipo,
        "etapa": etapa,
        "nuevo": false,
        "nombre": null,
        "descripcion": null,
        "confianza": 0.9,
        "razon": "por el texto",
        "otros": others.iter().map(|(n, s)| serde_json::json!({"documento": n, "etapa": s})).collect::<Vec<_>>(),
    })
    .to_string()
}

fn invoice(number: &str, issued: &str, total: &str, with_total: bool) -> String {
    format!(
        "FACTURA B N° {number}\nFecha de emisión: {issued}\nCliente: Ejemplo Tres S.A., CUIT 30-11111111-1.\n\
         Detalle: servicio mensual de alojamiento y soporte técnico de la plataforma, según contrato vigente, \
         con atención de incidentes en horario hábil y copias de seguridad diarias de la base de datos.\n\
         {}Medio de pago: transferencia bancaria a la cuenta informada en el pie de este comprobante.",
        if with_total { format!("Total a pagar: $ {total}\n") } else { String::new() }
    )
}

const LETTER: &str = "Intimo a usted para que en el plazo de cuarenta y ocho horas abone la factura 0003-00004321 \
adeudada por los servicios prestados durante el último trimestre, bajo apercibimiento de iniciar las acciones legales \
correspondientes y de suspender la prestación sin más aviso. Queda usted debidamente notificado.";

const RECIPE: &str = "Bizcochuelo de vainilla. Batir cuatro huevos con una taza de azúcar hasta que dupliquen su volumen, \
agregar una taza de harina tamizada de a poco, mezclar con movimientos envolventes, volcar en un molde enmantecado y \
hornear a temperatura moderada durante cuarenta minutos sin abrir la puerta del horno.";

fn base_with(files: &[(&str, String)]) -> (TempDir, DocBase) {
    let dir = TempDir::new();
    for (name, text) in files {
        dir.write(&format!("o/{name}"), text.as_bytes());
    }
    let base = DocBase::in_memory().unwrap();
    let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
    base.scan_origin(&origin.id).unwrap();
    while base.hash_pending(50).unwrap() > 0 {}
    while base.extract_pending(50).unwrap() > 0 {}
    base.purge_orphans().unwrap();
    while base.similar_step(10).unwrap() > 0 {}
    (dir, base)
}

fn location(base: &DocBase, name: &str) -> i64 {
    base.documents(&Visible::All, &DocumentFilter::default())
        .unwrap()
        .rows
        .into_iter()
        .find(|row| row.name == name)
        .unwrap_or_else(|| panic!("no document named {name}"))
        .location_id
}

fn type_of(base: &DocBase, name: &str) -> Option<super::classify::DocumentType> {
    base.document_type_of(location(base, name)).unwrap().0
}

fn data_of(base: &DocBase, name: &str, field: &str) -> Vec<String> {
    base.document_type_of(location(base, name))
        .unwrap()
        .1
        .into_iter()
        .filter(|d| d.field == field)
        .map(|d| d.value)
        .collect()
}

fn relation(base: &DocBase, a: &str, b: &str) -> Option<super::SimilarDocument> {
    base.similar_of(&Visible::All, location(base, a)).unwrap().into_iter().find(|s| s.other.name == b)
}

fn three_invoices() -> Vec<(&'static str, String)> {
    vec![
        ("enero.md", invoice("0003-00001001", "15/01/2026", "120.000,00", true)),
        ("febrero.md", invoice("0003-00001002", "14/02/2026", "131.500,00", true)),
        ("marzo.md", invoice("0003-00001003", "16/03/2026", "128.250,00", true)),
    ]
}

#[test]
fn one_question_per_family_and_it_is_never_asked_twice() {
    let (_dir, base) = base_with(&three_invoices());
    assert_eq!(base.similar_groups(&Visible::All).unwrap().len(), 1, "the three invoices are one family");
    assert_eq!(base.types_plan().unwrap().type_questions, 1);

    let model = FakeModel::new(|_| type_answer("factura", None, &[]));
    let report = base.types_run(&model, 10).unwrap();
    assert_eq!(model.calls(), 1, "one question for the whole family");
    assert_eq!(report.asked, 1);
    // The question carries the closed list and the text.
    let question = &model.questions()[0];
    assert!(question.contains("- carta-documento:") && question.contains("- factura:"), "{question}");
    assert!(question.contains("Documento 2"), "{question}");
    assert!(question.contains("FACTURA B"), "{question}");

    for name in ["enero.md", "febrero.md", "marzo.md"] {
        let found = type_of(&base, name).unwrap();
        assert_eq!(found.doc_type, "factura", "{name}");
        assert!(!found.proposed);
    }
    let principal = type_of(&base, "enero.md").unwrap().principal.unwrap();
    let sources: Vec<String> = ["enero.md", "febrero.md", "marzo.md"].iter().map(|n| type_of(&base, n).unwrap().source).collect();
    assert_eq!(sources.iter().filter(|s| *s == "model").count(), 1);
    assert_eq!(sources.iter().filter(|s| *s == "family").count(), 2);
    assert!(type_of(&base, "enero.md").unwrap().principal == Some(principal));

    // Asked again: nothing to ask.
    assert_eq!(base.types_plan().unwrap().type_questions, 0);
    let again = FakeModel::new(|_| panic!("nothing should be asked"));
    assert_eq!(base.types_run(&again, 10).unwrap().asked, 0);

    // Everything computed forgotten: the answer kept is used, not asked again.
    base.conn().execute_batch("DELETE FROM document_type; DELETE FROM document_data;").unwrap();
    assert_eq!(base.types_plan().unwrap().type_questions, 0);
    assert_eq!(type_of(&base, "febrero.md").unwrap().doc_type, "factura");
    let stored: (String, String) = base
        .conn()
        .query_row("SELECT engine, prompt_version FROM model_answer", [], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    assert_eq!(stored, ("falso:1".to_string(), super::classify::TYPE_PROMPT.to_string()));
}

#[test]
fn marks_that_say_it_spare_the_question() {
    let preview = format!("CARTA DOCUMENTO\nVISTA PREVIA - SIN VALOR LEGAL\nREMITENTE\nDESTINATARIO\n{LETTER}");
    let (_dir, base) = base_with(&[("vista previa.md", preview)]);
    assert_eq!(base.types_plan().unwrap().type_questions, 0);
    let found = type_of(&base, "vista previa.md").unwrap();
    assert_eq!((found.doc_type.as_str(), found.stage.as_deref(), found.source.as_str()), ("carta-documento", Some("vista-previa"), "marks"));
    // Its data come from the rules of its type.
    assert_eq!(data_of(&base, "vista previa.md", "facturas"), ["0003-00004321"]);
}

#[test]
fn a_type_outside_the_list_is_kept_as_a_proposal() {
    let (_dir, base) = base_with(&[("receta.md", RECIPE.to_string())]);
    let model = FakeModel::new(|_| {
        serde_json::json!({
            "tipo": "Receta de cocina", "etapa": null, "nuevo": true, "nombre": "Receta de cocina",
            "descripcion": "Instrucciones para preparar una comida.", "confianza": 0.7, "razon": "ingredientes y pasos", "otros": []
        })
        .to_string()
    });
    base.types_run(&model, 10).unwrap();
    let found = type_of(&base, "receta.md").unwrap();
    assert_eq!(found.doc_type, "receta-de-cocina");
    assert!(found.proposed);
    assert_eq!(found.proposed_name.as_deref(), Some("Receta de cocina"));
    let plan = base.types_plan().unwrap();
    assert_eq!(plan.proposed_types.len(), 1);
    assert_eq!(plan.proposed_types[0].documents, 1);
    // A proposed type has no rules: nothing else is asked about it.
    assert_eq!(plan.data_questions, 0);
}

#[test]
fn a_type_of_the_list_said_as_new_is_the_one_of_the_list() {
    let (_dir, base) = base_with(&[("enero.md", invoice("0003-00001001", "15/01/2026", "120.000,00", true))]);
    let model = FakeModel::new(|_| {
        serde_json::json!({
            "tipo": "factura", "etapa": "inventada", "nuevo": true, "nombre": "Factura", "descripcion": "x",
            "confianza": 2.0, "razon": "x", "otros": []
        })
        .to_string()
    });
    base.types_run(&model, 10).unwrap();
    let found = type_of(&base, "enero.md").unwrap();
    assert_eq!(found.doc_type, "factura");
    assert!(!found.proposed);
    // A stage the type does not have is not kept; the confidence stays in range.
    assert_eq!(found.stage, None);
    assert_eq!(found.confidence, Some(1.0));
}

#[test]
fn data_come_from_the_rules_and_the_model_only_completes_them() {
    let (_dir, base) = base_with(&[
        ("con total.md", invoice("0003-00001001", "15/01/2026", "120.000,00", true)),
        ("sin total.md", RECIPE.replace("Bizcochuelo", "Comprobante") + " Fecha de emisión: 02/03/2026. Factura N° 0007-00000099."),
    ]);
    let model = FakeModel::new(|prompt| {
        if prompt.contains("Copiá de su texto estos datos") {
            serde_json::json!({ "total": ["$ 999,00"] }).to_string()
        } else {
            type_answer("factura", None, &[])
        }
    });
    let report = base.types_run(&model, 10).unwrap();
    assert_eq!(report.asked, 2, "two documents alone, two type questions");
    assert_eq!(report.data_asked, 1, "only the one whose total the rules did not find");
    assert_eq!(data_of(&base, "con total.md", "numero"), ["0003-00001001"]);
    assert_eq!(data_of(&base, "con total.md", "total"), ["$ 120.000,00"]);
    assert_eq!(data_of(&base, "con total.md", "fecha-de-emision"), ["15/01/2026"]);
    assert_eq!(data_of(&base, "sin total.md", "total"), ["$ 999,00"]);
    let question = model.questions().into_iter().find(|q| q.contains("Copiá de su texto")).unwrap();
    assert!(question.contains("- total:") && !question.contains("- numero:"), "{question}");
    // Asked once: a new run asks nothing.
    let again = FakeModel::new(|_| panic!("nothing should be asked"));
    let report = base.types_run(&again, 10).unwrap();
    assert_eq!((report.asked, report.data_asked), (0, 0));
}

#[test]
fn the_limit_of_questions_leaves_the_rest_for_later() {
    let (_dir, base) = base_with(&[("receta.md", RECIPE.to_string()), ("carta.md", LETTER.to_string())]);
    let model = FakeModel::new(|_| type_answer("acuerdo", Some("borrador"), &[]));
    let report = base.types_run(&model, 1).unwrap();
    assert_eq!((report.asked, report.left), (1, 1));
    assert_eq!(base.types_plan().unwrap().type_questions, 1);
}

#[test]
fn the_type_and_stage_correct_a_measured_relation() {
    // The letter as sent (with its shipment number) and a copy without
    // it whose watermark could not be read: measured as versions.
    let sent = format!("CARTA DOCUMENTO\nEnvío XY123456789AR\n{LETTER}");
    let unmarked = format!("CARTA DOCUMENTO\n{LETTER}");
    let (_dir, base) = base_with(&[("enviada.md", sent), ("sin marca.md", unmarked)]);
    let measured = relation(&base, "enviada.md", "sin marca.md").expect("measured");
    assert_eq!(measured.relation, "version");

    // One question, about the principal; the model says the other is the preview.
    let model = FakeModel::new(|_| type_answer("carta-documento", Some("enviada"), &[(2, "vista-previa")]));
    let report = base.types_run(&model, 10).unwrap();
    assert_eq!(report.asked, 1);
    assert_eq!(type_of(&base, "sin marca.md").unwrap().stage.as_deref(), Some("vista-previa"));
    assert_eq!(type_of(&base, "enviada.md").unwrap().stage.as_deref(), Some("enviada"));
    assert_eq!(data_of(&base, "enviada.md", "numero-de-envio"), ["XY123456789AR"]);

    let corrected = relation(&base, "enviada.md", "sin marca.md").unwrap();
    assert_eq!(corrected.relation, "preview");
    assert_eq!(corrected.subject_content_id, Some(type_of(&base, "sin marca.md").unwrap().content_id));
    assert_eq!(corrected.evidence["typeRule"]["id"], "vista-previa-de-la-carta");
    assert!(corrected.description.contains("vista previa"), "{}", corrected.description);

    // Measured again from the texts: the rule applies again, and a
    // person's decision is never touched.
    base.similar_reset().unwrap();
    base.conn().execute_batch("DELETE FROM document_type; DELETE FROM document_data;").ok();
    while base.similar_step(10).unwrap() > 0 {}
    assert_eq!(relation(&base, "enviada.md", "sin marca.md").unwrap().relation, "preview");
}

#[test]
fn a_delivery_receipt_is_joined_to_its_letter_by_the_shipment_number() {
    let sent = format!("CARTA DOCUMENTO\nEnvío XY123456789AR\nLA PRESENTE ES COPIA FIEL DEL ORIGINAL\n{LETTER}");
    let receipt = "A.R. AVISO DE RECIBO. Pieza XY123456789AR. RECIBÍ CONFORME EL ENVÍO REFERENTE A ESTE AVISO. \
                   Fecha 12/05/2026 hora 10:30. Firma y aclaración de quien recibe en el domicilio indicado por el remitente."
        .to_string();
    let (_dir, base) = base_with(&[("enviada.md", sent), ("acuse.md", receipt)]);
    assert!(relation(&base, "enviada.md", "acuse.md").is_none(), "their texts share nothing");

    let model = FakeModel::new(|prompt| {
        if prompt.contains("RECIBÍ CONFORME") {
            type_answer("acuse-de-recibo", None, &[])
        } else {
            type_answer("carta-documento", Some("enviada"), &[])
        }
    });
    base.types_run(&model, 10).unwrap();
    // The letter's marks decided it alone; the receipt was asked about.
    assert_eq!(type_of(&base, "enviada.md").unwrap().source, "marks");
    assert_eq!(model.calls(), 1);
    let found = relation(&base, "enviada.md", "acuse.md").expect("proposed by the type");
    assert_eq!(found.relation, "delivery-receipt-of");
    assert_eq!(found.subject_content_id, Some(type_of(&base, "acuse.md").unwrap().content_id));
    assert_eq!(found.state, "proposed");
}

fn proposing(name: &'static str) -> FakeModel {
    FakeModel::new(move |prompt| {
        // A model that proposes the same new type until it is told not to.
        let rejected = prompt.contains("rechazó");
        serde_json::json!({
            "tipo": if rejected { "otro-tipo" } else { name }, "etapa": null, "nuevo": true,
            "nombre": if rejected { "Otro tipo" } else { "Receta de cocina" },
            "descripcion": "Instrucciones para preparar una comida.", "confianza": 0.7, "razon": "x", "otros": []
        })
        .to_string()
    })
}

#[test]
fn a_confirmed_type_joins_the_closed_list_with_its_name_and_the_answers_stay() {
    let (_dir, base) = base_with(&[("receta.md", RECIPE.to_string())]);
    let model = proposing("receta-de-cocina");
    base.types_run(&model, 10).unwrap();
    assert_eq!(base.proposed_types().unwrap().len(), 1);

    let confirm = super::classify::TypeDecision::Confirm { name: "Receta".into(), description: None };
    base.decide_type("receta-de-cocina", &confirm).unwrap();
    let found = type_of(&base, "receta.md").unwrap();
    assert!(!found.proposed);
    assert_eq!(found.proposed_name.as_deref(), Some("Receta"));
    assert!(base.proposed_types().unwrap().is_empty());
    let row = base.documents(&Visible::All, &DocumentFilter::default()).unwrap().rows.into_iter().find(|r| r.name == "receta.md").unwrap();
    assert_eq!((row.doc_type_proposed, row.doc_type_name.as_deref()), (false, Some("Receta")));

    // Nothing is asked again: the question already answered is the same.
    base.types_run(&model, 10).unwrap();
    assert_eq!(model.calls(), 1);
    assert_eq!(base.types_plan().unwrap().type_questions, 0);

    // Undone, it is a proposal again.
    base.decide_type("receta-de-cocina", &super::classify::TypeDecision::Undo).unwrap();
    assert!(type_of(&base, "receta.md").unwrap().proposed);
    assert_eq!(base.proposed_types().unwrap().len(), 1);
}

#[test]
fn a_rejected_type_is_asked_again_telling_the_model_not_to_propose_it() {
    let (_dir, base) = base_with(&[("receta.md", RECIPE.to_string())]);
    let model = proposing("receta-de-cocina");
    base.types_run(&model, 10).unwrap();
    base.decide_type("receta-de-cocina", &super::classify::TypeDecision::Reject).unwrap();
    assert!(type_of(&base, "receta.md").is_none(), "its documents lose the rejected type");
    assert_eq!(base.types_plan().unwrap().type_questions, 1, "and wait for a new question");

    base.types_run(&model, 10).unwrap();
    assert_eq!(model.calls(), 2);
    assert!(model.questions()[1].contains("receta-de-cocina (Receta de cocina)"));
    assert_eq!(type_of(&base, "receta.md").unwrap().doc_type, "otro-tipo");
    let decided = base.decided_types().unwrap();
    assert_eq!((decided[0].id.as_str(), decided[0].decision.as_str(), decided[0].documents), ("receta-de-cocina", "rejected", 0));
}

#[test]
fn only_proposed_types_can_be_decided_and_a_name_is_needed() {
    let (_dir, base) = base_with(&[("enero.md", invoice("0003-00001001", "15/01/2026", "120.000,00", true))]);
    let confirm = |name: &str| super::classify::TypeDecision::Confirm { name: name.into(), description: None };
    assert!(base.decide_type("factura", &confirm("Factura")).unwrap_err().contains("catálogo"));
    assert!(base.decide_type("no-existe", &confirm("X")).is_err());
    assert!(base.decide_type("no-existe", &super::classify::TypeDecision::Undo).is_err());
    let model = proposing("receta-de-cocina");
    let (_dir2, other) = base_with(&[("receta.md", RECIPE.to_string())]);
    other.types_run(&model, 10).unwrap();
    assert!(other.decide_type("receta-de-cocina", &confirm("  ")).is_err());
}

/// A fake model that answers under the name of an engine the base can
/// choose (the registry of AI services only takes those, D-443).
struct Named<'a>(&'a FakeModel, &'static str);

impl DocumentAsker for Named<'_> {
    fn engine_id(&self) -> String {
        self.1.to_string()
    }
    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String> {
        self.0.ask(prompt, schema)
    }
}

#[test]
fn once_an_engine_is_chosen_only_its_answers_count() {
    let (_dir, base) = base_with(&[("receta.md", RECIPE.to_string())]);
    let model = FakeModel::new(|_| type_answer("factura", None, &[]));
    base.types_run(&Named(&model, "codex-cli:1"), 10).unwrap();
    let forget = || base.conn().execute_batch("DELETE FROM document_type; DELETE FROM document_data;").unwrap();
    // No engine chosen yet: any answer kept counts.
    forget();
    assert_eq!(base.types_plan().unwrap().type_questions, 0);
    // The engine and model that answered it.
    base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "1".into(), ..Default::default() })).unwrap();
    forget();
    assert_eq!(base.types_plan().unwrap().type_questions, 0);
    // Another model: the question is asked again, and its answer kept apart.
    base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "2".into(), ..Default::default() })).unwrap();
    forget();
    assert_eq!(base.types_plan().unwrap().type_questions, 1);
    let other = FakeModel::new(|_| type_answer("factura", None, &[]));
    assert_eq!(base.types_run(&Named(&other, "codex-cli:2"), 10).unwrap().asked, 1);
    let answers: i64 = base.conn().query_row("SELECT count(*) FROM model_answer WHERE task = 'type'", [], |row| row.get(0)).unwrap();
    assert_eq!(answers, 2);
}

/// What the type of a document and its relation to another are after
/// one more round of the type pass and the measurement.
fn round(base: &DocBase, model: &FakeModel, a: &str, b: &str) -> (Option<String>, Option<String>) {
    base.types_run(model, 10).unwrap();
    while base.similar_step(10).unwrap() > 0 {}
    (type_of(base, b).map(|t| t.doc_type), relation(base, a, b).map(|r| r.relation))
}

#[test]
fn a_member_of_the_template_whose_rule_parts_the_family_is_typed_alone_and_it_settles() {
    // A letter and the postal form of another shipment printed over the
    // same template: the model reads the second, alone, as the delivery
    // receipt; asked as a member of the letter's family it would be
    // given the letter's type, which the rule of different shipment
    // numbers then uses to take the family apart.
    let letter = format!("CARTA DOCUMENTO\nEnvío XY111111111AR\nRemitente: Ejemplo Uno S.A.\n{LETTER}");
    let other = format!(
        "CARTA DOCUMENTO\nEnvío XY222222222AR\nRemitente: Ejemplo Uno S.A.\n{LETTER}\n\
         Entregado en el domicilio indicado, según consta en el correo."
    );
    let (_dir, base) = base_with(&[("acuse.md", letter), ("carta.md", other)]);
    assert_eq!(relation(&base, "carta.md", "acuse.md").map(|r| r.relation).as_deref(), Some("same-template"));
    let model = FakeModel::new(|prompt| {
        // The model reads the text of the document asked about (the
        // principal): its file name closes the question.
        if prompt.contains("Archivo: «acuse.md»") {
            type_answer("acuse-de-recibo", None, &[])
        } else {
            type_answer("carta-documento", Some("enviada"), &[])
        }
    });
    let mut seen = Vec::new();
    for _ in 0..6 {
        seen.push(round(&base, &model, "carta.md", "acuse.md"));
    }
    assert_eq!(seen[4], seen[5], "the type and the relation settle: {seen:?}");
    assert_eq!(seen[3], seen[5], "and stay: {seen:?}");
    assert_eq!(seen[5].0.as_deref(), Some("acuse-de-recibo"), "{seen:?}");
}
