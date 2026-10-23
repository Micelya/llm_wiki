//! The chat on the contract of operations (3.1, D-498): in each turn the
//! model answers one block of text in one of four shapes, in Spanish —
//! **usar** up to three readings of the contract, **proponer** one change
//! of the contract, **preguntar** the person with a form, or **responder**.
//! The program checks the block (a wrong one is said to the model, which
//! gets one more chance), runs the readings, and for a proposal finds what
//! it names in the base, builds the card, keeps it in the record and ends
//! the turn: the change is made by «Confirmar» on the card, by a person,
//! never by the model. The agent gets no permission to change the base.
//!
//! The turns are visible and their cap is extensible (4.1): each turn says
//! «vuelta i de n»; at the cap the model answers with what it found and
//! «Seguir buscando» goes on from where it stopped (`continuation`).
//!
//! The same loop serves every engine of only text: an engine with an API,
//! Codex (`text_engine.rs`) and, later, Core.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};
use uuid::Uuid;

use crate::docbase::DocBase;
use crate::operations::log::Origin;
use crate::operations::prompt::{self, ChatTools, MAX_READS_PER_TURN};
use crate::operations::read::{self, ReadContext};
use crate::operations::{find, flow, Caller, Kind, Proposal};

use super::base_tools::AttachedDocument;
use super::cancel::AgentCancellationToken;
use super::context::{collapse_whitespace, trim_chars};
use super::events::AgentEvent;
use super::provider::AgentLlmProvider;
use super::runtime::AgentEventSink;
use super::tools::{self, BuiltinToolRegistry, ToolRegistry};
use super::types::{
    AgentBudget, AgentChatRequest, AgentChatResponse, AgentReference, AgentRetrievalMode, AgentToolEvent, AgentUsage, AgentUserInputRequest,
};

/// The most of one observation the model sees again.
const MAX_OBSERVATION_CHARS: usize = 14_000;
/// What an old observation keeps when they no longer fit (5.4).
const SUMMARY_CHARS: usize = 1_500;
const MAX_HISTORY_CHARS: usize = 12_000;
const MAX_DOCUMENT_CHARS: usize = 20_000;
const MAX_DOCUMENTS_CHARS: usize = 48_000;
/// The continuations kept in memory, and for how long (4.1).
const MAX_CONTINUATIONS: usize = 32;
const CONTINUATION_TTL: Duration = Duration::from_secs(60 * 60);

/// What the model observed, as it sees it again in the next turn.
#[derive(Debug, Clone, PartialEq)]
struct Observed {
    what: String,
    text: String,
    summarized: bool,
}

/// Where a message's turns are: what «Seguir buscando» goes on from.
#[derive(Debug, Clone)]
struct LoopState {
    observed: Vec<Observed>,
    references: Vec<AgentReference>,
    signatures: BTreeSet<String>,
    used: usize,
    kept_at: Instant,
}

impl LoopState {
    fn new() -> Self {
        Self { observed: Vec::new(), references: Vec::new(), signatures: BTreeSet::new(), used: 0, kept_at: Instant::now() }
    }
}

fn continuations() -> &'static Mutex<HashMap<String, LoopState>> {
    static STORE: OnceLock<Mutex<HashMap<String, LoopState>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn keep_continuation(state: &LoopState) -> String {
    let id = format!("seguir-{}", Uuid::new_v4());
    let mut store = continuations().lock().unwrap_or_else(|p| p.into_inner());
    store.retain(|_, s| s.kept_at.elapsed() < CONTINUATION_TTL);
    while store.len() >= MAX_CONTINUATIONS {
        let oldest = store.iter().min_by_key(|(_, s)| s.kept_at).map(|(k, _)| k.clone());
        match oldest {
            Some(key) => {
                store.remove(&key);
            }
            None => break,
        }
    }
    let mut kept = state.clone();
    kept.kept_at = Instant::now();
    store.insert(id.clone(), kept);
    id
}

fn take_continuation(id: &str) -> Option<LoopState> {
    let mut store = continuations().lock().unwrap_or_else(|p| p.into_inner());
    let state = store.remove(id)?;
    (state.kept_at.elapsed() < CONTINUATION_TTL).then_some(state)
}

/// One step the model asked for.
#[derive(Debug, Clone, PartialEq)]
enum Step {
    Use(Vec<(String, Value)>),
    Propose { operation: String, data: Value, reason: Option<String> },
    Ask { title: Option<String>, description: Option<String>, fields: Option<Value> },
    Answer(String),
    /// Not a block: plain text (some models answer so).
    Text(String),
    /// A block that is wrong: what to tell the model.
    Invalid(String),
}

fn json_object(raw: &str) -> Option<Map<String, Value>> {
    let trimmed = raw.trim();
    let unfenced = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|rest| rest.trim().trim_end_matches("```").trim())
        .unwrap_or(trimmed);
    if let Ok(Value::Object(map)) = serde_json::from_str::<Value>(unfenced) {
        return Some(map);
    }
    let start = unfenced.find('{')?;
    let end = unfenced.rfind('}')?;
    (end > start).then(|| serde_json::from_str::<Value>(&unfenced[start..=end]).ok()).flatten().and_then(|v| match v {
        Value::Object(map) => Some(map),
        _ => None,
    })
}

fn string(map: &Map<String, Value>, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|k| map.get(*k).and_then(Value::as_str)).map(str::to_string)
}

/// The operations of the old protocol, said in the new one.
fn old_tool(map: &Map<String, Value>, tool: &str) -> Option<Step> {
    let query = string(map, &["query"]).unwrap_or_default();
    let path = string(map, &["path"]).unwrap_or_default();
    Some(match tool {
        "base.search" => Step::Use(vec![("buscar".into(), json!({ "consulta": query }))]),
        "base.read" => Step::Use(vec![("leer".into(), json!({ "id": path }))]),
        "source.search" => Step::Use(vec![("buscar_pasajes".into(), json!({ "consulta": query }))]),
        "graph.search" => Step::Use(vec![("relaciones".into(), json!({ "nombre": query }))]),
        "web.search" => Step::Use(vec![("buscar_en_internet".into(), json!({ "consulta": query }))]),
        "anytxt.search" => Step::Use(vec![("buscar_en_anytxt".into(), json!({ "consulta": query }))]),
        "base.save_query" => Step::Propose {
            operation: "consulta_guardar".into(),
            data: json!({ "titulo": string(map, &["title"]).unwrap_or(path), "respuesta": string(map, &["content"]).unwrap_or_default() }),
            reason: None,
        },
        "user.ask" | "user_input.ask" | "askUserQuestion" | "AskUserQuestion" | "ask_user_question" => Step::Ask {
            title: string(map, &["title"]),
            description: string(map, &["description"]),
            fields: map.get("fields").or_else(|| map.get("questions")).cloned(),
        },
        _ => return None,
    })
}

fn is_change(name: &str) -> bool {
    find(name).is_some_and(|op| op.kind == Kind::Change)
}

/// Reads the block the model answered, even with text around it.
fn parse_step(raw: &str) -> Step {
    let Some(map) = json_object(raw) else {
        let trimmed = raw.trim();
        if trimmed.starts_with('{') || trimmed.starts_with("```") {
            return Step::Invalid("El bloque no es un JSON válido o vino cortado. Devolvé un solo objeto JSON completo.".into());
        }
        return Step::Text(trimmed.to_string());
    };
    let action = string(&map, &["accion", "acción", "action"]).unwrap_or_default().trim().to_lowercase();
    let data = |m: &Map<String, Value>| m.get("datos").or_else(|| m.get("data")).cloned().unwrap_or(Value::Object(Map::new()));
    match action.as_str() {
        "usar" | "use" => {
            let mut calls = Vec::new();
            if let Some(list) = map.get("operaciones").and_then(Value::as_array) {
                for call in list {
                    if let Some(call) = call.as_object() {
                        let name = string(call, &["operacion", "operación", "operation"]).unwrap_or_default();
                        calls.push((name, data(call)));
                    }
                }
            } else if let Some(name) = string(&map, &["operacion", "operación"]) {
                calls.push((name, data(&map)));
            }
            if calls.is_empty() {
                return Step::Invalid("«usar» necesita «operaciones»: una lista de {\"operacion\":…,\"datos\":{…}}.".into());
            }
            Step::Use(calls)
        }
        "proponer" | "propose" => match string(&map, &["operacion", "operación", "operation"]) {
            Some(operation) => Step::Propose { operation, data: data(&map), reason: string(&map, &["motivo"]) },
            None => Step::Invalid("«proponer» necesita «operacion» y «datos».".into()),
        },
        "preguntar" | "ask" => Step::Ask {
            title: string(&map, &["titulo", "title"]),
            description: string(&map, &["descripcion", "descripción", "description"]),
            fields: map.get("campos").or_else(|| map.get("fields")).or_else(|| map.get("questions")).cloned(),
        },
        "responder" | "final" | "answer" => match string(&map, &["respuesta", "answer"]) {
            Some(answer) if !answer.trim().is_empty() => Step::Answer(answer),
            _ => Step::Invalid("«responder» necesita «respuesta» con el texto.".into()),
        },
        "tool" => match string(&map, &["tool"]).and_then(|tool| old_tool(&map, &tool)) {
            Some(step) => step,
            None => Step::Invalid("Esa herramienta no existe. Usá las operaciones de la lista con «usar» o «proponer».".into()),
        },
        "" => match string(&map, &["operacion", "operación"]) {
            Some(name) if is_change(&name) => Step::Propose { operation: name, data: data(&map), reason: string(&map, &["motivo"]) },
            Some(name) => Step::Use(vec![(name, data(&map))]),
            None => match string(&map, &["respuesta", "answer"]) {
                Some(answer) => Step::Answer(answer),
                None => Step::Invalid("Falta «accion»: usar, proponer, preguntar o responder.".into()),
            },
        },
        // An operation's name where the action goes.
        other if is_change(other) => Step::Propose { operation: other.to_string(), data: data(&map), reason: string(&map, &["motivo"]) },
        other if find(other).is_some() || other.starts_with("buscar_en_") => Step::Use(vec![(other.to_string(), data(&map))]),
        other => match old_tool(&map, other) {
            Some(step) => step,
            None => Step::Invalid(format!("«{other}» no es una acción: usá usar, proponer, preguntar o responder.")),
        },
    }
}

/// Content of a part, so it cannot close the part or open another.
fn escape_parts(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' {
            let mut j = i + 1;
            if j < chars.len() && chars[j] == '/' {
                j += 1;
            }
            let start = j;
            while j < chars.len() && (chars[j].is_ascii_uppercase() || chars[j] == '_' || chars[j] == 'Ó' || chars[j] == 'Í') {
                j += 1;
            }
            if j > start && j < chars.len() && chars[j] == '>' {
                out.push_str("&lt;");
                out.extend(&chars[i + 1..j]);
                out.push_str("&gt;");
                i = j + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// A screen where what the model asked for is done, when the chat cannot.
fn screen_for(name: &str) -> Option<&'static str> {
    let name = name.to_lowercase();
    if name.contains("borrar") || name.contains("eliminar") || name.contains("delete") {
        return Some("Nada se borra desde el chat: un caso se descarta y un documento se saca del caso.");
    }
    if name.contains("etiqueta") || name.contains("label") {
        return Some("Las etiquetas se ponen en la pantalla de Documentos.");
    }
    if name.contains("origen") || name.contains("carpeta") || name.contains("excluir") || name.contains("ajuste") {
        return Some("Los orígenes, las carpetas y las exclusiones se cambian en Ajustes.");
    }
    if name.contains("copia") || name.contains("version") {
        return Some("Las copias y versiones se deciden en «Copias y versiones».");
    }
    if name.contains("expediente") || name.contains("portal") {
        return Some("Los expedientes se actualizan en «Portales».");
    }
    if name.contains("tipo") {
        return Some("Los tipos de documento se deciden en su pantalla.");
    }
    if name.contains("grupo") {
        return Some("El grupo de sociedades todavía no se arma desde el chat.");
    }
    None
}

/// The turn of the chat, with what the runtime already did.
pub(super) struct Turn<'a> {
    pub request: &'a AgentChatRequest,
    pub message: &'a str,
    pub session_id: String,
    pub project_id: String,
    pub project_path: String,
    pub base: Option<Arc<DocBase>>,
    pub engine: Arc<dyn AgentLlmProvider>,
    /// How much text one turn of this engine takes (5.5).
    pub context_chars: usize,
    /// The engine takes no images (Codex).
    pub text_only: bool,
    pub web: Option<tools::WebSearchConfig>,
    pub anytxt: Option<tools::AnyTxtConfig>,
    pub references: Vec<AgentReference>,
    pub tool_events: Vec<AgentToolEvent>,
    pub events: Vec<AgentEvent>,
    pub event_sink: Option<AgentEventSink>,
    pub cancellation: Option<&'a AgentCancellationToken>,
    pub attached: Vec<AttachedDocument>,
}

fn emit(events: &mut Vec<AgentEvent>, sink: &Option<AgentEventSink>, event: AgentEvent) {
    if let Some(sink) = sink {
        sink(event.clone());
    }
    events.push(event);
}

fn check_cancel(cancellation: Option<&AgentCancellationToken>) -> Result<(), String> {
    match cancellation {
        Some(token) => token.check(),
        None => Ok(()),
    }
}

/// What a reading is doing, in words for the chat («buscando …»).
fn doing(name: &str, data: &Value) -> String {
    let text = |k: &str| data.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
    let part = data.get("parte").and_then(Value::as_i64).map(|p| format!(", parte {p}")).unwrap_or_default();
    match name {
        "buscar" => format!("buscando «{}»", text("consulta")),
        "buscar_pasajes" => format!("buscando el pasaje «{}»", text("consulta")),
        "leer" => format!("leyendo {}{part}", text("id")),
        "leer_texto" => format!("leyendo el texto de {}{part}", text("documento")),
        "relaciones" => format!("viendo las relaciones de {}", text("nombre")),
        "ver_documento" => format!("viendo el documento {}", text("documento")),
        "ver_ficha" => format!("viendo la ficha {}", text("ficha")),
        "ver_caso" => format!("viendo el caso {}", text("caso")),
        "listar_casos" => "listando los casos".to_string(),
        "listar_plantillas" => "listando las plantillas".to_string(),
        "buscar_en_internet" => format!("buscando en internet «{}»", text("consulta")),
        "buscar_en_anytxt" => format!("buscando en AnyTXT «{}»", text("consulta")),
        other => other.to_string(),
    }
}

/// What a reading found, in a few words for the chat.
fn found_words(result: &Value) -> String {
    for key in ["resultados", "pasajes", "casos", "plantillas"] {
        if let Some(list) = result.get(key).and_then(Value::as_array) {
            return if list.len() == 1 { "1 resultado".to_string() } else { format!("{} resultados", list.len()) };
        }
    }
    if let (Some(part), Some(parts)) = (result.get("parte").and_then(Value::as_u64), result.get("partes").and_then(Value::as_u64)) {
        let name = result.get("titulo").or_else(|| result.get("nombre")).and_then(Value::as_str).unwrap_or_default();
        return format!("{name}, parte {part} de {parts}");
    }
    result.get("nombre").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| "listo".to_string())
}

/// The objects a reading names, as references the answer cites.
fn references_of(result: &Value, out: &mut Vec<AgentReference>) {
    let mut add = |item: &Value| {
        let Some(path) = item.get("ruta").and_then(Value::as_str).filter(|p| !p.is_empty()) else { return };
        let kind = match item.get("tipo").and_then(Value::as_str) {
            Some("ficha") => "entity",
            Some("caso") => "matter",
            _ => match item.get("id").and_then(Value::as_str).and_then(crate::docbase::knowledge::articles::kind_of) {
                Some(kind) => kind,
                None => "document",
            },
        };
        let title = item.get("titulo").or_else(|| item.get("nombre")).and_then(Value::as_str).unwrap_or(path).to_string();
        if out.iter().any(|r| r.path == path) {
            return;
        }
        out.push(AgentReference {
            title,
            path: path.to_string(),
            kind: kind.to_string(),
            snippet: item.get("pasaje").and_then(Value::as_str).map(|s| trim_chars(&collapse_whitespace(s), 400)),
            score: None,
            knowledge_context: None,
        });
    };
    add(result);
    for key in ["resultados", "pasajes", "casos"] {
        if let Some(list) = result.get(key).and_then(Value::as_array) {
            for item in list {
                add(item);
            }
        }
    }
}

struct Loop<'a> {
    turn: Turn<'a>,
    state: LoopState,
    tools: ChatTools,
    instructions: String,
    invalid_in_a_row: usize,
    unknown: usize,
    last_prompt_chars: usize,
}

impl<'a> Loop<'a> {
    fn render_history(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        let mut total = 0;
        for message in self.turn.request.history.iter().rev() {
            let who = if message.role == "user" { "persona" } else { "asistente" };
            let line = format!("{who}: {}", trim_chars(message.content.trim(), 4_000));
            total += line.chars().count();
            if total > MAX_HISTORY_CHARS {
                break;
            }
            lines.push(line);
        }
        lines.reverse();
        escape_parts(&lines.join("\n\n"))
    }

    fn render_documents(&self, budget: usize) -> String {
        let mut out = String::new();
        let mut remaining = budget.min(MAX_DOCUMENTS_CHARS);
        for document in &self.turn.attached {
            let key = document.content_id.map(|c| format!("doc-{c}")).unwrap_or_else(|| format!("loc-{}", document.location_id));
            let head = format!("[{key}] «{}» ({})\n", document.name, document.place);
            let body = match document.text.as_deref() {
                Some(text) => {
                    let room = remaining.saturating_sub(head.chars().count()).min(MAX_DOCUMENT_CHARS);
                    if room < 200 {
                        format!("(no entra el texto: leelo con leer_texto {key})")
                    } else if text.chars().count() > room {
                        format!("{}\n[el texto sigue: leelo por partes con leer_texto]", trim_chars(text.trim(), room))
                    } else {
                        text.trim().to_string()
                    }
                }
                None => "(este documento todavía no tiene texto en la base)".to_string(),
            };
            remaining = remaining.saturating_sub(head.chars().count() + body.chars().count());
            out.push_str(&head);
            out.push_str(&escape_parts(&body));
            out.push_str("\n\n");
        }
        out
    }

    fn render_observed(&self) -> String {
        self.state
            .observed
            .iter()
            .enumerate()
            .map(|(i, o)| format!("{}. {}{}:\n{}", i + 1, o.what, if o.summarized { " (resumido)" } else { "" }, escape_parts(&o.text)))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// The user part of a turn, fitted to the engine: the oldest
    /// observations are summarized first, then the conversation is cut;
    /// the instructions and the request are never touched (5.4).
    fn user_text(&mut self, max: usize, continuing: bool) -> String {
        loop {
            let history = self.render_history();
            let documents = self.render_documents(MAX_DOCUMENTS_CHARS);
            let mut out = String::new();
            if !history.is_empty() {
                out.push_str(&format!("<CONVERSACION>\n{history}\n</CONVERSACION>\n\n"));
            }
            if !documents.is_empty() {
                out.push_str(&format!("<DOCUMENTOS>\nLos documentos adjuntos al mensaje («este documento», «estos»):\n\n{documents}</DOCUMENTOS>\n\n"));
            }
            let observed = self.render_observed();
            out.push_str(&format!("<OBSERVADO>\n{}\n</OBSERVADO>\n\n", if observed.is_empty() { "Nada todavía.".to_string() } else { observed }));
            if self.turn.text_only && !self.turn.request.images.is_empty() {
                out.push_str("(La persona adjuntó imágenes que este motor no puede ver: decíselo si hacen falta.)\n\n");
            }
            out.push_str(&format!("<PEDIDO>\n{}\n</PEDIDO>\n\n", escape_parts(self.turn.message.trim())));
            if continuing {
                out.push_str("La persona pidió seguir buscando: seguí desde lo observado, sin repetir lo que ya leíste.\n");
            }
            out.push_str(&prompt::budget_line(self.state.used, max));
            out.push_str("\nDevolvé ahora el bloque JSON del paso siguiente.");
            let total = self.instructions.chars().count() + out.chars().count();
            if total <= self.turn.context_chars {
                return out;
            }
            // Summarize the oldest observation not summarized yet.
            if let Some(oldest) = self.state.observed.iter_mut().find(|o| !o.summarized && o.text.chars().count() > SUMMARY_CHARS) {
                oldest.text = trim_chars(&oldest.text, SUMMARY_CHARS);
                oldest.summarized = true;
                continue;
            }
            return trim_chars(&out, self.turn.context_chars.saturating_sub(self.instructions.chars().count()).max(4_000));
        }
    }

    fn observe(&mut self, what: String, text: String) {
        self.state.observed.push(Observed { what, text: trim_chars(&text, MAX_OBSERVATION_CHARS), summarized: false });
    }

    fn reject(&mut self, what: &str, why: String) {
        self.turn.tool_events.push(AgentToolEvent { tool: what.to_string(), status: "failed".into(), detail: Some(why.clone()) });
        emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::tool_end(what, Some(format!("failed: {why}"))));
        self.observe(what.to_string(), format!("rechazado: {why}"));
    }

    fn engine_label(&self) -> String {
        format!("{}:{}", self.turn.engine.provider_name(), self.turn.engine.model_name())
    }

    async fn generate(&mut self, user: &str) -> Result<String, String> {
        let images = if self.state.used == 1 && !self.turn.text_only { self.turn.request.images.clone() } else { Vec::new() };
        self.last_prompt_chars = self.instructions.chars().count() + user.chars().count();
        self.turn.tool_events.push(AgentToolEvent { tool: "llm.generate".into(), status: "started".into(), detail: Some(self.engine_label()) });
        let engine = self.turn.engine.clone();
        let future = engine.generate_text(&self.instructions, user, &images);
        let result = match self.turn.cancellation {
            Some(token) => tokio::select! {
                biased;
                _ = token.cancelled() => Err("Agent turn cancelled".to_string()),
                result = future => result,
            },
            None => future.await,
        };
        match &result {
            Ok(_) => self.turn.tool_events.push(AgentToolEvent { tool: "llm.generate".into(), status: "completed".into(), detail: None }),
            Err(error) => {
                self.turn.tool_events.push(AgentToolEvent { tool: "llm.generate".into(), status: "failed".into(), detail: Some(error.clone()) });
                emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::Error { message: error.clone() });
            }
        }
        result
    }

    /// Runs the readings of one turn (up to three).
    async fn use_readings(&mut self, calls: Vec<(String, Value)>) -> Result<(), String> {
        let extra = calls.len().saturating_sub(MAX_READS_PER_TURN);
        for (name, data) in calls.into_iter().take(MAX_READS_PER_TURN) {
            check_cancel(self.turn.cancellation)?;
            let name = name.trim().to_string();
            if name == "buscar_en_internet" || name == "buscar_en_anytxt" {
                self.chat_tool(&name, &data).await;
                continue;
            }
            let Some(op) = find(&name) else {
                self.unknown_operation(&name);
                continue;
            };
            if op.kind == Kind::Change {
                self.reject(&name, format!("«{name}» cambia algo: no se usa, se propone sola con {{\"accion\":\"proponer\",…}}."));
                continue;
            }
            if !op.ready || !op.callers.contains(&Caller::Chat) || !prompt::chat_readings(&self.tools).iter().any(|r| r.name == op.name) {
                self.reject(&name, format!("«{name}» no se puede usar desde el chat en este modo."));
                continue;
            }
            let data = match op.check(&data) {
                Ok(data) => data,
                Err(why) => {
                    self.reject(&name, why);
                    continue;
                }
            };
            let signature = format!("{name}:{data}");
            let what = format!("{name} {data}");
            if !self.state.signatures.insert(signature) {
                self.observe(what, "Ya lo leíste: usá lo observado arriba.".to_string());
                continue;
            }
            let doing = doing(&name, &data);
            emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::tool_start(name.clone(), Some(doing.clone())));
            self.turn.tool_events.push(AgentToolEvent { tool: name.clone(), status: "started".into(), detail: Some(doing) });
            let Some(base) = self.turn.base.clone() else {
                self.reject(&name, "La base documental no está abierta.".to_string());
                continue;
            };
            let context = ReadContext { attached: self.turn.request.context_documents.clone() };
            let (task_name, task_data) = (name.clone(), data.clone());
            let result = tokio::task::spawn_blocking(move || read::read(&base, &task_name, &task_data, &context))
                .await
                .map_err(|e| format!("La lectura se interrumpió: {e}"))?;
            match result {
                Ok(value) => {
                    let mut found = Vec::new();
                    references_of(&value, &mut found);
                    for reference in found {
                        if !self.turn.references.iter().any(|r| r.path == reference.path) {
                            emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::ReferenceAdded { reference: reference.clone() });
                            self.turn.references.push(reference.clone());
                        }
                        if !self.state.references.iter().any(|r| r.path == reference.path) {
                            self.state.references.push(reference);
                        }
                    }
                    let words = found_words(&value);
                    self.turn.tool_events.push(AgentToolEvent { tool: name.clone(), status: "completed".into(), detail: Some(words.clone()) });
                    emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::tool_end(name.clone(), Some(words)));
                    self.observe(what, value.to_string());
                }
                Err(why) => self.reject(&name, why),
            }
        }
        if extra > 0 {
            self.observe("aviso".into(), format!("Pediste {} lecturas de más: se hacen hasta {MAX_READS_PER_TURN} por vuelta.", extra));
        }
        Ok(())
    }

    /// Searching the internet or AnyTXT: the chat's own tools, outside the
    /// contract.
    async fn chat_tool(&mut self, name: &str, data: &Value) {
        let (enabled, tool) = match name {
            "buscar_en_internet" => (self.tools.web && !self.tools.faithful, "web.search"),
            _ => (self.tools.anytxt && !self.tools.faithful, "anytxt.search"),
        };
        if !enabled {
            self.reject(name, format!("«{name}» no está encendido en este mensaje."));
            return;
        }
        let query = data.get("consulta").and_then(Value::as_str).unwrap_or_default().trim().to_string();
        if query.is_empty() {
            self.reject(name, format!("A {name} le falta «consulta»"));
            return;
        }
        let doing = doing(name, data);
        emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::tool_start(tool, Some(doing.clone())));
        let context = tools::ToolContext {
            project_path: &self.turn.project_path,
            base: self.turn.base.clone(),
            web_search_config: self.turn.web.clone(),
            anytxt_config: self.turn.anytxt.clone(),
        };
        let registry = BuiltinToolRegistry;
        let result = registry.execute(tool, json!({ "query": query, "topK": 5 }), context).await;
        match result.and_then(|v| serde_json::from_value::<Vec<AgentReference>>(v).map_err(|e| e.to_string())) {
            Ok(found) => {
                let summary: Vec<Value> =
                    found.iter().map(|r| json!({ "titulo": r.title, "direccion": r.path, "pasaje": r.snippet })).collect();
                for reference in found {
                    if !self.turn.references.iter().any(|r| r.path == reference.path) {
                        emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::ReferenceAdded { reference: reference.clone() });
                        self.turn.references.push(reference);
                    }
                }
                emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::tool_end(tool, Some(format!("{} resultados", summary.len()))));
                self.observe(format!("{name} {data}"), json!({ "resultados": summary }).to_string());
            }
            Err(why) => self.reject(name, why),
        }
    }

    fn unknown_operation(&mut self, name: &str) {
        self.unknown += 1;
        let readings: Vec<&str> = prompt::chat_readings(&self.tools).iter().map(|op| op.name).collect();
        let changes: Vec<&str> = prompt::chat_changes().iter().map(|op| op.name).collect();
        self.reject(
            name,
            format!(
                "«{name}» no es una operación del contrato. Para leer: {}. Para proponer: {}. Si lo que pide la persona no está, decile que no se puede hacer desde el chat.",
                readings.join(", "),
                changes.join(", ")
            ),
        );
    }

    fn done(&mut self, answer: String, max: usize, proposal: Option<crate::operations::Card>, form: Option<AgentUserInputRequest>) -> AgentChatResponse {
        let reached = self.state.used >= max && proposal.is_none() && form.is_none();
        let continuation = reached.then(|| {
            self.state.references = self.turn.references.clone();
            keep_continuation(&self.state)
        });
        if self.turn.event_sink.is_some() && !answer.is_empty() {
            emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::MessageDelta { text: answer.clone() });
        }
        let budget = AgentBudget { used: self.state.used, max, reached, continuation };
        emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::Budget { budget: budget.clone() });
        emit(&mut self.turn.events, &self.turn.event_sink, AgentEvent::Done { session_id: self.turn.session_id.clone() });
        let usage = AgentUsage {
            prompt_chars: self.last_prompt_chars,
            completion_chars: answer.len(),
            reference_count: self.turn.references.len(),
            tool_event_count: self.turn.tool_events.len(),
        };
        AgentChatResponse {
            ok: true,
            project_id: self.turn.project_id.clone(),
            session_id: self.turn.session_id.clone(),
            mode: self.turn.request.mode,
            message: answer,
            references: std::mem::take(&mut self.turn.references),
            tool_events: std::mem::take(&mut self.turn.tool_events),
            events: std::mem::take(&mut self.turn.events),
            user_input_request: form,
            operation_proposal: proposal,
            budget: Some(budget),
            usage: Some(usage),
        }
    }

    /// What the chat says when the model did not answer in its last turn:
    /// what it found, not an error.
    fn forced_answer(&self) -> String {
        let references = &self.turn.references;
        if references.is_empty() {
            return "No llegué a encontrar en la base lo necesario para contestar con seguridad.".to_string();
        }
        let mut answer = String::from("No llegué a terminar. Esto es lo que encontré en la base:\n");
        for reference in references.iter().take(8) {
            answer.push_str(&format!("- {} ({})", reference.title, reference.path));
            if let Some(snippet) = reference.snippet.as_deref() {
                answer.push_str(&format!(": {}", trim_chars(&collapse_whitespace(snippet), 320)));
            }
            answer.push('\n');
        }
        answer
    }
}

/// Runs the turn of the chat on the contract.
pub(super) async fn run(turn: Turn<'_>) -> Result<AgentChatResponse, String> {
    let request = turn.request;
    let tools = ChatTools { web: request.tools.web, anytxt: request.tools.anytxt, faithful: request.retrieval_mode == AgentRetrievalMode::Faithful };
    let instructions = prompt::chat_instructions(crate::operations::words::words(), &tools);
    let cap = super::runtime::chat_turns(request.mode);
    let continued = request.continuation.as_deref().and_then(take_continuation);
    let continuing = continued.is_some();
    let state = continued.unwrap_or_else(LoopState::new);
    let max = if continuing { state.used + request.extra_steps.unwrap_or(cap).clamp(1, 24) } else { cap };
    let mut chat = Loop { turn, state, tools, instructions, invalid_in_a_row: 0, unknown: 0, last_prompt_chars: 0 };
    // What the turns before found is cited again.
    for reference in chat.state.references.clone() {
        if !chat.turn.references.iter().any(|r| r.path == reference.path) {
            chat.turn.references.push(reference);
        }
    }
    loop {
        check_cancel(chat.turn.cancellation)?;
        chat.state.used += 1;
        let used = chat.state.used;
        let last = used >= max;
        emit(&mut chat.turn.events, &chat.turn.event_sink, AgentEvent::Step { used, max });
        let user = chat.user_text(max, continuing);
        let raw = chat.generate(&user).await?;
        let step = parse_step(&raw);
        if !matches!(step, Step::Invalid(_) | Step::Text(_)) {
            chat.invalid_in_a_row = 0;
        }
        match step {
            Step::Answer(answer) => return Ok(chat.done(answer, max, None, None)),
            Step::Ask { title, description, fields } => match super::runtime::user_form(title, description, fields) {
                Ok(form) => {
                    emit(&mut chat.turn.events, &chat.turn.event_sink, AgentEvent::UserInputRequired { request: form.clone() });
                    let text = form.description.clone().unwrap_or_else(|| "Necesito estos datos para seguir.".to_string());
                    return Ok(chat.done(text, max, None, Some(form)));
                }
                Err(why) if !last => chat.reject("preguntar", why),
                Err(_) => return Ok({
                    let text = chat.forced_answer();
                    chat.done(text, max, None, None)
                }),
            },
            Step::Text(text) => {
                // A model that answers in plain text: once it is told the
                // shape; the second time its text is the answer.
                chat.invalid_in_a_row += 1;
                if chat.invalid_in_a_row >= 2 || last {
                    return Ok(chat.done(text, max, None, None));
                }
                chat.reject("bloque", "Contestá con un solo objeto JSON: usar, proponer, preguntar o responder.".to_string());
            }
            Step::Invalid(why) => {
                chat.invalid_in_a_row += 1;
                if chat.invalid_in_a_row >= 2 || last {
                    let text = chat.forced_answer();
                    return Ok(chat.done(text, max, None, None));
                }
                chat.reject("bloque", why);
            }
            Step::Use(_) | Step::Propose { .. } if last => {
                let text = chat.forced_answer();
                return Ok(chat.done(text, max, None, None));
            }
            Step::Use(calls) => {
                let before = chat.unknown;
                chat.use_readings(calls).await?;
                if chat.unknown > before && chat.unknown >= 2 {
                    let name = chat.state.observed.last().map(|o| o.what.clone()).unwrap_or_default();
                    return Ok(cannot(&mut chat, &name, max));
                }
            }
            Step::Propose { operation, data, reason } => {
                let operation = operation.trim().to_string();
                let Some(op) = find(&operation) else {
                    chat.unknown_operation(&operation);
                    if chat.unknown >= 2 {
                        return Ok(cannot(&mut chat, &operation, max));
                    }
                    continue;
                };
                if op.kind == Kind::Read {
                    chat.use_readings(vec![(operation, data)]).await?;
                    continue;
                }
                if !op.ready || !op.callers.contains(&Caller::Chat) {
                    chat.unknown += 1;
                    if chat.unknown >= 2 {
                        return Ok(cannot(&mut chat, &operation, max));
                    }
                    chat.reject(&operation, format!("«{operation}» todavía no se puede pedir desde el chat. Decíselo a la persona."));
                    continue;
                }
                let Some(base) = chat.turn.base.clone() else {
                    return Ok(chat.done("La base documental no está abierta: no puedo preparar ese cambio.".to_string(), max, None, None));
                };
                let origin = Origin {
                    caller: "chat".into(),
                    request: Some(chat.turn.message.to_string()),
                    conversation: Some(format!("{}:{}", chat.turn.session_id, request.run_id.clone().unwrap_or_default())),
                    engine: Some(chat.engine_label()),
                };
                let attached = request.context_documents.clone();
                let name = operation.clone();
                emit(&mut chat.turn.events, &chat.turn.event_sink, AgentEvent::tool_start(name.clone(), Some(format!("preparando la tarjeta: {name}"))));
                let proposal = tokio::task::spawn_blocking(move || flow::propose(&base, &name, &data, &origin, &attached))
                    .await
                    .map_err(|e| format!("La propuesta se interrumpió: {e}"))?;
                match proposal {
                    Ok(Proposal::Card(card)) => {
                        emit(&mut chat.turn.events, &chat.turn.event_sink, AgentEvent::tool_end(operation.clone(), Some(card.title.clone())));
                        chat.turn.tool_events.push(AgentToolEvent { tool: operation.clone(), status: "completed".into(), detail: Some(card.title.clone()) });
                        emit(&mut chat.turn.events, &chat.turn.event_sink, AgentEvent::OperationProposed { card: Box::new(card.clone()) });
                        let mut text = String::new();
                        if let Some(reason) = reason.filter(|r| !r.trim().is_empty()) {
                            text.push_str(reason.trim());
                            text.push_str("\n\n");
                        }
                        text.push_str("Te preparé la tarjeta para que la revises: todavía no se hizo nada.");
                        return Ok(chat.done(text, max, Some(card), None));
                    }
                    Ok(Proposal::Nothing(observation)) => {
                        emit(&mut chat.turn.events, &chat.turn.event_sink, AgentEvent::tool_end(operation.clone(), Some("no hay nada que hacer".into())));
                        chat.observe(format!("proponer {operation}"), observation);
                    }
                    Err(why) => {
                        chat.invalid_in_a_row += 1;
                        chat.reject(&operation, why);
                    }
                }
            }
        }
    }
}

/// The chat says it cannot do what was asked, and where it is done.
fn cannot(chat: &mut Loop<'_>, name: &str, max: usize) -> AgentChatResponse {
    let name = name.split_whitespace().next().unwrap_or(name).trim();
    let mut text = "Eso no lo puedo hacer desde el chat: no es una de las operaciones que el chat puede pedir.".to_string();
    if let Some(screen) = screen_for(name) {
        text.push(' ');
        text.push_str(screen);
    }
    chat.done(text, max, None, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_block_is_read_in_its_four_shapes_and_in_the_old_one() {
        assert_eq!(
            parse_step(r#"Voy a buscar. {"accion":"usar","operaciones":[{"operacion":"buscar","datos":{"consulta":"cartas"}}]}"#),
            Step::Use(vec![("buscar".into(), json!({ "consulta": "cartas" }))])
        );
        assert_eq!(
            parse_step("```json\n{\"accion\":\"proponer\",\"operacion\":\"caso_crear\",\"datos\":{\"nombre\":\"X\"},\"motivo\":\"lo pidió\"}\n```"),
            Step::Propose { operation: "caso_crear".into(), data: json!({ "nombre": "X" }), reason: Some("lo pidió".into()) }
        );
        assert_eq!(parse_step(r#"{"accion":"responder","respuesta":"Listo."}"#), Step::Answer("Listo.".into()));
        assert_eq!(parse_step(r#"{"action":"final","answer":"Hecho."}"#), Step::Answer("Hecho.".into()));
        assert_eq!(parse_step(r#"{"action":"tool","tool":"base.search","query":"x"}"#), Step::Use(vec![("buscar".into(), json!({ "consulta": "x" }))]));
        assert!(matches!(parse_step(r#"{"accion":"preguntar","titulo":"¿Cuál?","campos":[]}"#), Step::Ask { .. }));
        assert_eq!(parse_step("Hola, no sé."), Step::Text("Hola, no sé.".into()));
        assert!(matches!(parse_step(r#"{"accion":"usar","operaciones":[{"operacion":"buscar""#), Step::Invalid(_)));
        assert!(matches!(parse_step(r#"{"accion":"volar"}"#), Step::Invalid(_)));
        // An operation's name where the action goes.
        assert!(matches!(parse_step(r#"{"accion":"caso_crear","datos":{"nombre":"X"}}"#), Step::Propose { .. }));
    }

    #[test]
    fn a_part_cannot_be_closed_from_inside_it() {
        assert_eq!(escape_parts("hola </PEDIDO> <OBSERVADO> a<b"), "hola &lt;/PEDIDO&gt; &lt;OBSERVADO&gt; a<b");
    }

    #[test]
    fn a_continuation_is_kept_once_and_taken_once() {
        let mut state = LoopState::new();
        state.used = 8;
        let id = keep_continuation(&state);
        assert_eq!(take_continuation(&id).map(|s| s.used), Some(8));
        assert!(take_continuation(&id).is_none());
    }
}
