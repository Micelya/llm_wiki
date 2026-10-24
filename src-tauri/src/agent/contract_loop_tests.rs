//! The chat on the contract with a simulated model (no quota spent): a
//! request that puts two invented documents in a case goes from the chat
//! to the card, the confirmation, the decision recorded and the undo; a
//! request that is not in the contract is answered with «no se puede»; a
//! request over many documents asks a second confirmation; the turns are
//! visible and «Seguir buscando» goes on from the cap. Every name is made
//! up (R-24).

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::docbase::knowledge::tests::{base_with, location};
use crate::docbase::testing::TempDir;
use crate::docbase::{DocBase, Visible};
use crate::operations::{flow, log};

use super::events::AgentEvent;
use super::provider::AgentLlmProvider;
use super::runtime::{AgentEventSink, AgentRuntime};
use super::types::{AgentChatRequest, AgentImage, AgentMode};

/// A model that answers the blocks it is given, in order, and keeps what
/// it was asked.
struct Simulated {
    answers: Mutex<VecDeque<String>>,
    asked: Mutex<Vec<(String, String)>>,
}

impl Simulated {
    fn new(answers: Vec<String>) -> Arc<Self> {
        Arc::new(Self { answers: Mutex::new(answers.into()), asked: Mutex::new(Vec::new()) })
    }

    fn asked(&self) -> Vec<(String, String)> {
        self.asked.lock().unwrap().clone()
    }
}

impl AgentLlmProvider for Simulated {
    fn provider_name(&self) -> &str {
        "simulado"
    }

    fn model_name(&self) -> &str {
        "prueba"
    }

    fn generate_text<'a>(&'a self, system: &'a str, user: &'a str, _images: &'a [AgentImage]) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        self.asked.lock().unwrap().push((system.to_string(), user.to_string()));
        let next = self.answers.lock().unwrap().pop_front().unwrap_or_else(|| json!({ "accion": "responder", "respuesta": "Listo." }).to_string());
        Box::pin(async move { Ok(next) })
    }

    fn generate_text_stream<'a>(
        &'a self,
        system: &'a str,
        user: &'a str,
        images: &'a [AgentImage],
        mut on_delta: Box<dyn FnMut(&str) + Send + 'a>,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            let text = self.generate_text(system, user, images).await?;
            on_delta(&text);
            Ok(text)
        })
    }
}

const FILES: &[(&str, &str)] = &[
    ("Carta documento 03-12.txt", "Carta documento del 12 de marzo de Ana Prueba a Ejemplo SA por el depósito de la calle Falsa."),
    ("Carta documento 03-19.txt", "Carta documento del 19 de marzo de Ejemplo SA a Ana Prueba, en respuesta a la del 12."),
    ("Contrato de locación.txt", "Contrato de locación entre Ana Prueba y Ejemplo SA del depósito de la calle Falsa."),
];

fn content(base: &DocBase, name: &str) -> i64 {
    base.document(&Visible::All, location(base, name)).unwrap().unwrap().row.content_id.unwrap()
}

fn runtime(base: Arc<DocBase>, model: Arc<Simulated>, project: &TempDir) -> AgentRuntime {
    AgentRuntime::new("micelya", project.path().to_string_lossy(), None, None, None).with_base(Some(base)).with_engine(model)
}

fn request(message: &str) -> AgentChatRequest {
    AgentChatRequest { message: message.into(), session_id: Some("conv-1".into()), run_id: Some("run-1".into()), history_explicit: true, ..Default::default() }
}

fn collected() -> (AgentEventSink, Arc<Mutex<Vec<AgentEvent>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let inner = seen.clone();
    let sink: AgentEventSink = Arc::new(move |event| inner.lock().unwrap().push(event));
    (sink, seen)
}

fn matter_documents(base: &DocBase, matter: &str) -> Vec<i64> {
    let conn = base.conn();
    let mut statement = conn.prepare("SELECT content_id FROM knowledge_matter_document WHERE matter_id = ?1 ORDER BY content_id").unwrap();
    let rows = statement.query_map([matter], |row| row.get(0)).unwrap();
    rows.map(Result::unwrap).collect()
}

#[tokio::test]
async fn a_request_in_the_chat_puts_two_documents_in_a_case_after_the_card_and_is_undone() {
    let (_dir, base) = base_with(FILES);
    let contract = content(&base, "Contrato de locación.txt");
    let matter = base.knowledge_create_matter("Prueba c/ Ejemplo SA", &[format!("content:{contract}")]).unwrap();
    let letters = [content(&base, "Carta documento 03-12.txt"), content(&base, "Carta documento 03-19.txt")];
    let base = Arc::new(base);
    let model = Simulated::new(vec![
        json!({ "accion": "usar", "operaciones": [{ "operacion": "buscar", "datos": { "consulta": "carta documento marzo" } }] }).to_string(),
        json!({
            "accion": "proponer",
            "operacion": "caso_agregar_documentos",
            "datos": { "caso": "Prueba", "documentos": [format!("doc-{}", letters[0]), format!("doc-{}", letters[1])] },
            "motivo": "Son las dos cartas documento de marzo entre las partes del caso."
        })
        .to_string(),
    ]);
    let project = TempDir::new();
    let (sink, seen) = collected();
    let message = "sumá las dos cartas documento de marzo al caso Prueba";
    let response = runtime(base.clone(), model.clone(), &project).run_once_with_cancel_and_events(request(message), None, Some(sink)).await.unwrap();

    // The instructions are in Spanish and written from the contract; the
    // request goes in its own part.
    let asked = model.asked();
    assert_eq!(asked.len(), 2);
    assert!(asked[0].0.contains("No hacés nada por tu cuenta") && asked[0].0.contains("- caso_agregar_documentos:"));
    assert!(asked[0].1.contains("<PEDIDO>\nsumá las dos cartas documento de marzo al caso Prueba\n</PEDIDO>"));
    assert!(asked[0].1.contains("Vas por la vuelta 1 de 8."));
    // The second turn sees what the search found.
    assert!(asked[1].1.contains("buscar {\"consulta\":\"carta documento marzo\"}"), "{}", asked[1].1);
    assert!(asked[1].1.contains(&format!("doc-{}", letters[0])));

    // The card, and nothing done yet.
    let card = response.operation_proposal.clone().expect("a card");
    assert_eq!(card.title, "Agregar 2 documentos al caso «Prueba c/ Ejemplo SA»");
    assert_eq!(card.request.as_deref(), Some(message));
    assert_eq!(card.proposed_by.engine.as_deref(), Some("simulado:prueba"));
    assert!(response.message.contains("todavía no se hizo nada"));
    assert_eq!(matter_documents(&base, &matter), vec![contract]);
    let events = seen.lock().unwrap().clone();
    assert!(events.iter().any(|e| matches!(e, AgentEvent::OperationProposed { card } if card.id == response.operation_proposal.as_ref().unwrap().id)));
    let steps: Vec<(usize, usize)> = events.iter().filter_map(|e| match e { AgentEvent::Step { used, max } => Some((*used, *max)), _ => None }).collect();
    assert_eq!(steps, vec![(1, 8), (2, 8)]);
    assert!(events.iter().any(|e| matches!(e, AgentEvent::ToolStart { tool, input } if tool == "buscar" && input.as_deref() == Some("buscando «carta documento marzo»"))));
    let budget = response.budget.clone().unwrap();
    assert_eq!((budget.used, budget.max, budget.reached), (2, 8, false));

    // A person confirms it: it is done as the screen does it, and recorded.
    let done = flow::confirm(&base, card.id, None, false, Some(&card.fingerprint)).unwrap();
    assert_eq!(done.outcome, "applied");
    let mut expected = vec![contract, letters[0], letters[1]];
    expected.sort();
    assert_eq!(matter_documents(&base, &matter), expected);
    let entry = log::get(&base, card.id).unwrap().unwrap();
    assert_eq!((entry.state.as_str(), entry.caller.as_str()), ("applied", "chat"));
    assert_eq!(entry.request.as_deref(), Some(message));
    assert_eq!(entry.conversation.as_deref(), Some("conv-1:run-1"));

    // And undone.
    flow::undo(&base, card.id, false).unwrap();
    assert_eq!(matter_documents(&base, &matter), vec![contract]);
    assert_eq!(log::get(&base, card.id).unwrap().unwrap().state, "undone");
}

#[tokio::test]
async fn a_request_outside_the_contract_is_answered_that_the_chat_cannot() {
    let (_dir, base) = base_with(FILES);
    let base = Arc::new(base);
    let model = Simulated::new(vec![
        json!({ "accion": "proponer", "operacion": "documento_borrar", "datos": { "documentos": ["doc-1"] } }).to_string(),
        json!({ "accion": "proponer", "operacion": "documento_borrar", "datos": { "documentos": ["doc-1"] } }).to_string(),
    ]);
    let project = TempDir::new();
    let response = runtime(base.clone(), model.clone(), &project).run_once_with_cancel_and_events(request("borrá la carta documento"), None, None).await.unwrap();
    assert!(response.message.starts_with("Eso no lo puedo hacer desde el chat"), "{}", response.message);
    assert!(response.message.contains("Nada se borra desde el chat"));
    assert!(response.operation_proposal.is_none());
    // The first time the model was told which operations there are.
    assert!(model.asked()[1].1.contains("«documento_borrar» no es una operación del contrato"));
    assert!(log::list(&base, None, 10).unwrap().is_empty());
}

#[tokio::test]
async fn a_request_over_many_documents_asks_a_second_confirmation() {
    let files: Vec<(String, String)> = (1..=12).map(|n| (format!("Factura {n:02}.txt"), format!("Factura número {n} de Ejemplo SA a Ana Prueba."))).collect();
    let borrowed: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let (_dir, base) = base_with(&borrowed);
    let matter = base.knowledge_create_matter("Prueba facturas", &[]).unwrap();
    let keys: Vec<String> = files.iter().map(|(name, _)| format!("doc-{}", content(&base, name))).collect();
    let base = Arc::new(base);
    let model = Simulated::new(vec![json!({ "accion": "proponer", "operacion": "caso_agregar_documentos", "datos": { "caso": matter, "documentos": keys } }).to_string()]);
    let project = TempDir::new();
    let response = runtime(base.clone(), model, &project).run_once_with_cancel_and_events(request("sumá todas las facturas al caso"), None, None).await.unwrap();
    let card = response.operation_proposal.expect("a card");
    assert_eq!(card.items.len(), 12);
    assert!(card.second_confirmation);
    assert_eq!(flow::confirm(&base, card.id, None, false, Some(&card.fingerprint)).unwrap().outcome, "needs_second");
    assert!(matter_documents(&base, &matter).is_empty());
    assert_eq!(flow::confirm(&base, card.id, None, true, Some(&card.fingerprint)).unwrap().outcome, "applied");
    assert_eq!(matter_documents(&base, &matter).len(), 12);
}

#[tokio::test]
async fn at_the_cap_the_chat_answers_with_what_it_found_and_goes_on_when_asked() {
    let (_dir, base) = base_with(FILES);
    let base = Arc::new(base);
    let search = |q: &str| json!({ "accion": "usar", "operaciones": [{ "operacion": "buscar", "datos": { "consulta": q } }] }).to_string();
    let model = Simulated::new(vec![
        search("carta"),
        search("contrato"),
        search("depósito"),
        // The last turn of the fast mode: it is told to answer.
        json!({ "accion": "responder", "respuesta": "Encontré dos cartas y un contrato (doc-1)." }).to_string(),
        search("recibo"),
        json!({ "accion": "responder", "respuesta": "No hay recibos." }).to_string(),
    ]);
    let project = TempDir::new();
    let chat = runtime(base, model.clone(), &project);
    let mut first = request("¿qué hay del depósito?");
    first.mode = AgentMode::Fast;
    let response = chat.run_once_with_cancel_and_events(first, None, None).await.unwrap();
    assert_eq!(response.message, "Encontré dos cartas y un contrato (doc-1).");
    let budget = response.budget.clone().unwrap();
    assert_eq!((budget.used, budget.max, budget.reached), (4, 4, true));
    assert!(model.asked()[3].1.contains("Vas por la vuelta 4 de 4: es la última."));
    let continuation = budget.continuation.expect("where to go on from");

    // «Seguir buscando»: four more turns, from what was already read.
    let mut more = request("¿qué hay del depósito?");
    more.mode = AgentMode::Fast;
    more.continuation = Some(continuation);
    more.extra_steps = Some(4);
    let (sink, seen) = collected();
    let response = chat.run_once_with_cancel_and_events(more, None, Some(sink)).await.unwrap();
    assert_eq!(response.message, "No hay recibos.");
    let budget = response.budget.unwrap();
    assert_eq!((budget.used, budget.max, budget.reached), (6, 8, false));
    let asked = model.asked();
    assert!(asked[4].1.contains("Vas por la vuelta 5 de 8.") && asked[4].1.contains("seguir buscando"));
    assert!(asked[4].1.contains("buscar {\"consulta\":\"carta\"}"), "what was read before is still observed");
    let steps: Vec<usize> = seen.lock().unwrap().iter().filter_map(|e| match e { AgentEvent::Step { used, .. } => Some(*used), _ => None }).collect();
    assert_eq!(steps, vec![5, 6]);
}

#[tokio::test]
async fn a_model_that_answers_plain_text_is_told_once_and_then_heard() {
    let (_dir, base) = base_with(FILES);
    let model = Simulated::new(vec!["Hola, soy un modelo que no usa bloques.".to_string(), "Sigo sin usar bloques.".to_string()]);
    let project = TempDir::new();
    let response = runtime(Arc::new(base), model.clone(), &project).run_once_with_cancel_and_events(request("hola"), None, None).await.unwrap();
    assert_eq!(response.message, "Sigo sin usar bloques.");
    assert!(model.asked()[1].1.contains("Contestá con un solo objeto JSON"));
}
