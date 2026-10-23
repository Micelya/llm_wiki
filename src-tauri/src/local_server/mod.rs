//! The one server of the app inside the machine (D-487, D-495, P-227;
//! `docs/design/contrato-de-operaciones.md`, 6 and 11.8). It listens on
//! `127.0.0.1:19827`, the address the web clipper always used, so the
//! extension already installed keeps working without installing it again.
//! It answers:
//!
//! - `GET /status` and `POST /clip`: the web clipper, as before, without a
//!   token (a page of a web site cannot pass for an extension).
//! - `/api/v1/…`: the local API, for programs and scripts (`api.rs`).
//! - `POST /mcp`: the MCP, for assistants such as Claude Code (`mcp.rs`).
//!
//! Both offer the same operations the chat uses, from the one list of the
//! contract (`operations`): by default a program only reads; with the
//! permission to ask for changes, what it asks waits in Atención for a
//! person (`calls.rs`). Without the token of a program nothing but the
//! clipper is answered.
//!
//! It never answers a web page: only the browser's extensions and programs
//! of the machine, which send no `Origin` (`auth.rs`).
//!
//! - `clip.rs`: saving a web clip.
//! - `auth.rs`: who may talk to it.
//! - `calls.rs`: what a program asks of the contract.
//! - `programs.rs`: the programs a person lets in, with their tokens.
//! - `commands.rs`: what Ajustes asks of it.

pub mod api;
pub mod auth;
pub mod calls;
pub mod clip;
pub mod commands;
pub mod mcp;
pub mod programs;

use std::collections::VecDeque;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};
use tiny_http::{Header, Method, Request, Response, Server};

use crate::docbase::DocBase;

/// The port of the web clipper since its first version.
pub const PORT: u16 = 19827;
const BIND_ATTEMPTS: u32 = 3;
/// A clip larger than this is not a page.
const MAX_CLIP_BYTES: u64 = 8 * 1024 * 1024;
/// The most a request of a program carries.
const MAX_BODY_BYTES: u64 = 1024 * 1024;
/// The most requests in a second, and at the same time (the limits of the
/// local API of the original program).
const RATE_WINDOW: Duration = Duration::from_secs(1);
const MAX_PER_WINDOW: usize = 120;
const MAX_IN_FLIGHT: usize = 64;

/// 0 starting, 1 listening, 2 the port is taken, 3 stopped.
static STATE: AtomicU8 = AtomicU8::new(0);
/// Whether programs may use the server (the switch of Ajustes, read from
/// the base when the server starts).
static PROGRAMS_ON: AtomicBool = AtomicBool::new(true);

pub fn programs_on() -> bool {
    PROGRAMS_ON.load(Ordering::Relaxed)
}

pub fn set_programs_on(on: bool) {
    PROGRAMS_ON.store(on, Ordering::Relaxed);
}

/// How the server is, for Ajustes: `starting`, `running`, `port-in-use`
/// or `stopped`.
pub fn state() -> &'static str {
    match STATE.load(Ordering::Relaxed) {
        0 => "starting",
        1 => "running",
        2 => "port-in-use",
        _ => "stopped",
    }
}

/// The address of the server, as a program writes it.
pub fn address() -> String {
    format!("http://127.0.0.1:{PORT}")
}

/// What the server needs from the app; the tests give their own.
pub struct Context {
    /// The port it listens on: every request has to be addressed to it.
    pub port: u16,
    /// The base, when it is open.
    pub base: Box<dyn Fn() -> Result<Arc<DocBase>, String> + Send + Sync>,
    /// Where the clips go, asked on every clip (the work folder can change).
    pub clips: Box<dyn Fn() -> Result<PathBuf, String> + Send + Sync>,
    /// Told of every clip saved.
    pub clip_saved: Box<dyn Fn(&Path) + Send + Sync>,
    /// Whether programs may use it (the switch of Ajustes).
    pub programs_on: Box<dyn Fn() -> bool + Send + Sync>,
    /// Told when a program left a change waiting for a person.
    pub proposed: Box<dyn Fn() + Send + Sync>,
}

/// How many requests came in the last second, and how many are being
/// answered.
#[derive(Default)]
struct Limits {
    recent: Mutex<VecDeque<Instant>>,
    in_flight: AtomicUsize,
}

impl Limits {
    fn allow(&self) -> bool {
        let now = Instant::now();
        let mut recent = self.recent.lock().unwrap_or_else(|p| p.into_inner());
        while recent.front().is_some_and(|t| now.duration_since(*t) > RATE_WINDOW) {
            recent.pop_front();
        }
        if recent.len() >= MAX_PER_WINDOW {
            return false;
        }
        recent.push_back(now);
        true
    }

    fn slot(self: &Arc<Self>) -> Option<Slot> {
        let mut current = self.in_flight.load(Ordering::Relaxed);
        loop {
            if current >= MAX_IN_FLIGHT {
                return None;
            }
            match self.in_flight.compare_exchange_weak(current, current + 1, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => return Some(Slot(self.clone())),
                Err(next) => current = next,
            }
        }
    }
}

struct Slot(Arc<Limits>);

impl Drop for Slot {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Where it listens: the machine itself, never every address.
pub fn bind_address() -> String {
    format!("127.0.0.1:{PORT}")
}

/// Starts the server in its own thread, on the machine only.
pub fn start(app: AppHandle) {
    thread::spawn(move || {
        let address = bind_address();
        let mut server = None;
        for attempt in 1..=BIND_ATTEMPTS {
            match Server::http(&address) {
                Ok(bound) => {
                    server = Some(bound);
                    break;
                }
                Err(error) => {
                    eprintln!("[local-server] {address} is not available ({attempt}/{BIND_ATTEMPTS}): {error}");
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
        let Some(server) = server else {
            STATE.store(2, Ordering::Relaxed);
            return;
        };
        if let Some(base) = crate::docbase::bridge::shared_base() {
            match programs::enabled(&base) {
                Ok(on) => set_programs_on(on),
                Err(error) => eprintln!("[local-server] the switch of the programs could not be read: {error}"),
            }
        }
        STATE.store(1, Ordering::Relaxed);
        eprintln!("[local-server] listening on http://{address}");
        if let Some(base) = crate::docbase::bridge::shared_base() {
            if let Err(error) = crate::operations::log::expire_program_requests(&base, crate::docbase::now_ms()) {
                eprintln!("[local-server] the old requests could not expire: {error}");
            }
        }
        let folder = app.clone();
        let sweep = app.clone();
        let changed = app.clone();
        let context = Context {
            port: PORT,
            base: Box::new(|| crate::docbase::bridge::shared_base().ok_or_else(|| "La base documental no está abierta".to_string())),
            clips: Box::new(move || crate::work_folder::clips_folder(&folder)),
            clip_saved: Box::new(move |path| {
                eprintln!("[local-server] clip saved: {}", path.display());
                sweep.state::<crate::docbase::commands::DocBaseState>().request_sweep();
                let _ = sweep.emit(crate::docbase::commands::CHANGED_EVENT, ());
            }),
            programs_on: Box::new(programs_on),
            proposed: Box::new(move || {
                let _ = changed.emit(crate::docbase::commands::CHANGED_EVENT, ());
            }),
        };
        serve(&server, Arc::new(context));
        STATE.store(3, Ordering::Relaxed);
    });
}

/// Answers every request of `server`, each in its own thread, within the
/// limits, until the server is unblocked.
pub fn serve(server: &Server, context: Arc<Context>) {
    let limits = Arc::new(Limits::default());
    for request in server.incoming_requests() {
        if !limits.allow() {
            answer(request, 429, error_json("Demasiados pedidos: probá de nuevo en un momento"), None, &[]);
            continue;
        }
        let Some(slot) = limits.slot() else {
            answer(request, 503, error_json("El servidor está ocupado: probá de nuevo en un momento"), None, &[]);
            continue;
        };
        let context = context.clone();
        thread::spawn(move || {
            let _slot = slot;
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle(request, &context)));
            if outcome.is_err() {
                eprintln!("[local-server] a request failed");
            }
        });
    }
}

/// Answers one request.
pub fn handle(mut request: Request, context: &Context) {
    if !auth::host_is_ours(header(&request, "Host").as_deref(), context.port) {
        return answer(request, 421, error_json("Este pedido no es para Micelya"), None, &[]);
    }
    let origin = header(&request, "Origin");
    if origin.as_deref().is_some_and(|o| !auth::is_extension(o)) {
        return answer(request, 403, error_json("Una página web no puede usar Micelya"), None, &[]);
    }
    let origin = origin.as_deref();
    if request.method() == &Method::Options {
        return answer(request, 204, String::new(), origin, &[]);
    }
    let path = request.url().split('?').next().unwrap_or("").to_string();
    match (request.method().clone(), path.as_str()) {
        (Method::Get, "/status") => answer(request, 200, json!({ "ok": true, "app": "Micelya" }).to_string(), origin, &[]),
        (Method::Post, "/clip") => {
            if !is_json(&request) {
                return answer(request, 415, error_json("El recorte tiene que llegar como JSON"), origin, &[]);
            }
            let body = match read_body(&mut request, MAX_CLIP_BYTES) {
                Ok(body) => body,
                Err((status, error)) => return answer(request, status, error_json(&error), origin, &[]),
            };
            let result = serde_json::from_str::<clip::Clip>(&body)
                .map_err(|error| format!("El pedido no es un recorte: {error}"))
                .and_then(|clip| {
                    let folder = (context.clips)()?;
                    clip::save_clip(&folder, &clip, Local::now())
                });
            match result {
                Ok(path) => {
                    (context.clip_saved)(&path);
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    answer(request, 200, json!({ "ok": true, "file": name }).to_string(), origin, &[])
                }
                Err(error) => answer(request, 400, error_json(&error), origin, &[]),
            }
        }
        (_, p) if p == "/api" || p.starts_with("/api/") || p == "/mcp" => for_programs(request, context, origin, &path),
        _ => answer(request, 404, error_json("No existe"), origin, &[]),
    }
}

/// The local API and the MCP: only with the token of a program, while the
/// programs are on.
fn for_programs(mut request: Request, context: &Context, origin: Option<&str>, path: &str) {
    if !(context.programs_on)() {
        return answer(request, 503, error_json("Los programas conectados están apagados en Micelya (Ajustes → Programas conectados)"), origin, &[]);
    }
    let base = match (context.base)() {
        Ok(base) => base,
        Err(error) => return answer(request, 503, error_json(&error), origin, &[]),
    };
    let token = header(&request, "Authorization");
    let program = match auth::bearer(token.as_deref()).map(|token| programs::by_token(&base, token)) {
        Some(Ok(Some(program))) => program,
        Some(Err(error)) => return answer(request, 500, error_json(&error), origin, &[]),
        _ => {
            let why = "Falta el token del programa o no es el correcto: se da en Micelya, Ajustes → Programas conectados";
            return answer(request, 401, error_json(why), origin, &[("WWW-Authenticate", "Bearer realm=\"micelya\"")]);
        }
    };
    let method = request.method().as_str().to_ascii_uppercase();
    if path == "/mcp" && method != "POST" {
        // No stream of events: everything is answered to its POST.
        return answer(request, 405, error_json("El MCP de Micelya se usa con POST"), origin, &[("Allow", "POST")]);
    }
    let mut body = String::new();
    if method == "POST" {
        if !is_json(&request) {
            return answer(request, 415, error_json("El pedido tiene que llegar como JSON (Content-Type: application/json)"), origin, &[]);
        }
        body = match read_body(&mut request, MAX_BODY_BYTES) {
            Ok(body) => body,
            Err((status, error)) => return answer(request, status, error_json(&error), origin, &[]),
        };
    }
    if path == "/mcp" {
        let (status, text) = mcp::route(&body, &base, &program, &*context.proposed);
        return answer(request, status, text.unwrap_or_default(), origin, &[]);
    }
    let (status, text) = api::route(&method, path, &body, &base, &program, &*context.proposed);
    answer(request, status, text, origin, &[])
}

pub(crate) fn error_json(message: &str) -> String {
    json!({ "ok": false, "error": message }).to_string()
}

fn header(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str().to_string())
}

fn is_json(request: &Request) -> bool {
    header(request, "Content-Type").is_some_and(|value| value.trim().to_ascii_lowercase().starts_with("application/json"))
}

/// The body of a request, up to `limit` bytes.
fn read_body(request: &mut Request, limit: u64) -> Result<String, (u16, String)> {
    let mut body = String::new();
    request
        .as_reader()
        .take(limit + 1)
        .read_to_string(&mut body)
        .map_err(|error| (400, format!("No se pudo leer el pedido: {error}")))?;
    if body.len() as u64 > limit {
        return Err((413, "El pedido es demasiado grande".to_string()));
    }
    Ok(body)
}

fn answer(request: Request, status: u16, body: String, origin: Option<&str>, extra: &[(&str, &str)]) {
    let mut response = Response::from_string(body).with_status_code(status);
    let mut headers: Vec<(&str, String)> = vec![
        ("Content-Type", "application/json; charset=utf-8".to_string()),
        ("Cache-Control", "no-store".to_string()),
    ];
    // Only an extension of the browser reads what the server answers.
    if let Some(origin) = origin.filter(|origin| auth::is_extension(origin)) {
        headers.push(("Access-Control-Allow-Origin", origin.to_string()));
        headers.push(("Access-Control-Allow-Methods", "GET, POST, OPTIONS".to_string()));
        headers.push(("Access-Control-Allow-Headers", "Content-Type, Authorization".to_string()));
        headers.push(("Access-Control-Max-Age", "600".to_string()));
        headers.push(("Access-Control-Allow-Private-Network", "true".to_string()));
        headers.push(("Vary", "Origin".to_string()));
    }
    for (name, value) in extra {
        headers.push((name, value.to_string()));
    }
    for (name, value) in headers {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response.add_header(header);
        }
    }
    let _ = request.respond(response);
}

#[cfg(test)]
mod tests;
