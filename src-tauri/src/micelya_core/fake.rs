//! A stand-in Micelya Core for the tests: a local server that answers as
//! the issuer and the API do, as each test sets it, and keeps every request
//! it got. It never talks to the real Core. Every value is made up.

use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tiny_http::{Header, Response, Server};

use super::client::Timing;
use super::CoreConnection;

pub(crate) const TENANT: &str = "organizacion-de-prueba";
const CLIENT: &str = "cliente-de-prueba";
const SECRET: &str = "secreto-de-prueba";

/// One request the server got.
#[derive(Debug, Clone)]
pub(crate) struct Seen {
    pub method: String,
    /// With its query.
    pub path: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Seen {
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.clone())
    }

    /// Everything it carried, as text, to look for what must never go.
    pub fn everything(&self) -> String {
        let headers: Vec<String> = self.headers.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        format!("{} {}\n{}\n{}", self.method, self.path, headers.join("\n"), String::from_utf8_lossy(&self.body))
    }
}

/// How the server answers, and what it got.
pub(crate) struct FakeState {
    pub seen: Vec<Seen>,
    /// How many permits it gave (`permiso-1`, `permiso-2`…).
    pub permits_given: usize,
    /// How long the next permit lasts, in seconds.
    pub permit_lasts: u64,
    /// When each permit was given, and for how long.
    issued: Vec<(Instant, u64)>,
    /// The API refuses the permits numbered below this.
    pub refuse_permits_given_before: usize,
    /// How many readings of others the list shows.
    pub listed: usize,
    /// Every list answers this error.
    pub list_status: Option<u16>,
    /// The next lists answer these errors, one each.
    pub list_failures: Vec<u16>,
    /// Every reading asked for, deleted or not.
    pub readings: Vec<FakeReading>,
    /// How the next readings end.
    pub outcome: Outcome,
    /// How many looks a reading in progress takes to end.
    pub looks_to_end: usize,
    /// What the result says read it.
    pub model_version: String,
    /// The next requests for a reading answer these errors, one each, after
    /// registering it (the answer is lost on the way).
    pub request_failures: Vec<u16>,
    /// The next deletions answer these errors, one each.
    pub delete_failures: Vec<u16>,
}

/// How a reading ends.
#[derive(Debug, Clone)]
pub(crate) enum Outcome {
    /// It is read: these lines, each with its words and their confidence.
    Read(Vec<Vec<(&'static str, f64)>>),
    /// It fails, with this reason.
    Fails(&'static str),
    /// The file never arrived in time.
    Expires,
    /// It never ends.
    NeverEnds,
}

/// A reading the fake keeps.
#[derive(Debug, Clone)]
pub(crate) struct FakeReading {
    pub id: String,
    pub filename: String,
    pub sha256: String,
    pub status: String,
    pub stage: Option<String>,
    pub failure: Option<String>,
    pub looks: usize,
    pub uploaded: Option<Vec<u8>>,
    pub outcome: Outcome,
    pub deleted: bool,
}

impl Default for FakeState {
    fn default() -> Self {
        Self {
            seen: Vec::new(),
            permits_given: 0,
            permit_lasts: 3600,
            issued: Vec::new(),
            refuse_permits_given_before: 0,
            listed: 0,
            list_status: None,
            list_failures: Vec::new(),
            readings: Vec::new(),
            outcome: Outcome::Read(vec![vec![("Texto", 0.99), ("de", 0.99), ("prueba", 0.95)]]),
            looks_to_end: 2,
            model_version: "1.0".into(),
            request_failures: Vec::new(),
            delete_failures: Vec::new(),
        }
    }
}

impl FakeState {
    /// The readings not deleted.
    pub fn kept(&self) -> Vec<&FakeReading> {
        self.readings.iter().filter(|r| !r.deleted).collect()
    }
}

pub(crate) struct FakeCore {
    /// `http://127.0.0.1:<port>`.
    pub url: String,
    state: Arc<Mutex<FakeState>>,
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
}

impl FakeCore {
    pub fn start() -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").expect("a local port for the fake Core"));
        let port = server.server_addr().to_ip().expect("an address of this computer").port();
        let url = format!("http://127.0.0.1:{port}");
        let state = Arc::new(Mutex::new(FakeState::default()));
        let thread = {
            let (server, state, url) = (server.clone(), state.clone(), url.clone());
            std::thread::spawn(move || {
                for mut request in server.incoming_requests() {
                    let mut body = Vec::new();
                    let _ = request.as_reader().read_to_end(&mut body);
                    let seen = Seen {
                        method: request.method().as_str().to_string(),
                        path: request.url().to_string(),
                        headers: request.headers().iter().map(|h| (h.field.as_str().to_string(), h.value.as_str().to_string())).collect(),
                        body,
                    };
                    let (status, answer) = {
                        let mut state = state.lock().unwrap();
                        state.seen.push(seen.clone());
                        handle(&mut state, &url, &seen)
                    };
                    let text = match &answer {
                        Value::Null => String::new(),
                        other => other.to_string(),
                    };
                    let response = Response::from_string(text)
                        .with_status_code(status)
                        .with_header(Header::from_bytes("Content-Type", "application/json").unwrap());
                    let _ = request.respond(response);
                }
            })
        };
        Self { url, state, server, thread: Some(thread) }
    }

    /// The connection with this server.
    pub fn connection(&self) -> CoreConnection {
        CoreConnection {
            environment: "ambiente-de-prueba".into(),
            api: format!("{}/v1", self.url),
            issuer: format!("{}/realms/prueba", self.url),
            tenant: TENANT.into(),
            client_id: CLIENT.into(),
            client_secret: SECRET.into(),
        }
    }

    /// Waits of a few milliseconds, so the tests do not wait.
    pub fn timing() -> Timing {
        Timing {
            call: Duration::from_secs(5),
            upload: Duration::from_secs(5),
            poll_every: Duration::from_millis(5),
            reading_cap: Duration::from_secs(5),
            retry_waits: vec![Duration::from_millis(1), Duration::from_millis(1)],
        }
    }

    pub fn with<T>(&self, change: impl FnOnce(&mut FakeState) -> T) -> T {
        change(&mut self.state.lock().unwrap())
    }

    pub fn seen(&self) -> Vec<Seen> {
        self.with(|state| state.seen.clone())
    }
}

impl Drop for FakeCore {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn problem(status: u16, code: &str, detail: &str) -> (u16, Value) {
    (status, json!({ "type": "about:blank", "title": "Error de prueba", "status": status, "code": code, "detail": detail, "traceId": "rastro-de-prueba" }))
}

/// The fields of a form (`a=b&c=d`), decoded.
fn form(body: &[u8]) -> Vec<(String, String)> {
    let decode = |text: &str| {
        let bytes = text.replace('+', " ").into_bytes();
        let mut out = Vec::new();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] == b'%' && index + 2 < bytes.len() {
                if let Ok(byte) = u8::from_str_radix(&String::from_utf8_lossy(&bytes[index + 1..index + 3]), 16) {
                    out.push(byte);
                    index += 3;
                    continue;
                }
            }
            out.push(bytes[index]);
            index += 1;
        }
        String::from_utf8_lossy(&out).to_string()
    };
    String::from_utf8_lossy(body)
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .map(|(key, value)| (decode(key), decode(value)))
        .collect()
}

fn request_json(reading: &FakeReading) -> Value {
    let done = reading.status == "succeeded";
    json!({ "id": reading.id, "status": reading.status, "stage": reading.stage, "requestedAt": chrono::Utc::now().to_rfc3339(),
            "failure": reading.failure, "filename": reading.filename, "requestedBy": "persona-de-prueba", "ownership": "personal",
            "pages": if done { json!(1) } else { Value::Null }, "characters": if done { json!(10) } else { Value::Null },
            "cancellable": false, "analysis": null })
}

fn upload_json(url: &str, reading: &FakeReading) -> Value {
    json!({ "url": format!("{url}/subidas/{}?firma=firma-de-prueba", reading.id), "method": "PUT",
            "headers": { "Content-Type": "image/jpeg", "x-amz-meta-prueba": "1" }, "expiresAt": "2099-01-01T00:00:00Z" })
}

fn result_json(reading: &FakeReading, model_version: &str) -> Value {
    let Outcome::Read(lines) = &reading.outcome else {
        return Value::Null;
    };
    let lines: Vec<Value> = lines
        .iter()
        .enumerate()
        .map(|(index, words)| {
            let text = words.iter().map(|(w, _)| *w).collect::<Vec<_>>().join(" ");
            let confidence = words.iter().map(|(_, c)| *c).fold(1.0_f64, f64::min);
            json!({ "page": 1, "text": text, "confidence": confidence,
                    "boundingBox": { "left": 0.1, "top": 0.05 * index as f64, "width": 0.8, "height": 0.04 },
                    "words": words.iter().map(|(w, c)| json!({ "text": w, "confidence": c })).collect::<Vec<_>>() })
        })
        .collect();
    let text = lines.iter().map(|l| l["text"].as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join("\n");
    json!({ "id": reading.id, "filename": reading.filename, "requestedBy": "persona-de-prueba", "ownership": "personal",
            "pages": 1, "text": text, "lines": lines, "requestedAt": "2026-01-01T00:00:00Z", "completedAt": "2026-01-01T00:00:01Z",
            "provider": "textract", "modelVersion": model_version, "analysis": null })
}

fn handle(state: &mut FakeState, url: &str, seen: &Seen) -> (u16, Value) {
    let (path, _query) = seen.path.split_once('?').unwrap_or((&seen.path, ""));
    // The upload goes straight to the storage, without the permit.
    if let Some(id) = path.strip_prefix("/subidas/") {
        if seen.method != "PUT" || seen.header("authorization").is_some() {
            return (400, Value::Null);
        }
        return match state.readings.iter_mut().find(|r| r.id == id && !r.deleted) {
            Some(reading) => {
                reading.uploaded = Some(seen.body.clone());
                (200, Value::Null)
            }
            None => (404, Value::Null),
        };
    }
    if path == "/realms/prueba/protocol/openid-connect/token" && seen.method == "POST" {
        let fields = form(&seen.body);
        let field = |name: &str| fields.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()).unwrap_or("");
        if field("grant_type") != "client_credentials" || field("client_id") != CLIENT || field("client_secret") != SECRET {
            return (401, json!({ "error": "invalid_client", "error_description": "Invalid client or Invalid client credentials" }));
        }
        if field("scope") != format!("organization:{TENANT}") {
            return (400, json!({ "error": "invalid_scope", "error_description": "Invalid scopes: organization" }));
        }
        state.permits_given += 1;
        state.issued.push((Instant::now(), state.permit_lasts));
        return (200, json!({ "access_token": format!("permiso-{}", state.permits_given), "expires_in": state.permit_lasts, "token_type": "Bearer" }));
    }
    let Some(route) = path.strip_prefix("/v1/") else {
        return problem(404, "not_found", "No existe");
    };
    // Every call to the API: a permit it gave that has not ended, and the organization.
    let number = seen
        .header("authorization")
        .and_then(|value| value.strip_prefix("Bearer permiso-").and_then(|n| n.parse::<usize>().ok()))
        .unwrap_or(0);
    let valid = number >= 1
        && number >= state.refuse_permits_given_before
        && state.issued.get(number - 1).is_some_and(|(at, lasts)| at.elapsed() < Duration::from_secs(*lasts));
    if !valid {
        return problem(401, "UNAUTHENTICATED", "El permiso no sirve");
    }
    if seen.header("x-tenant-id").as_deref() != Some(TENANT) {
        return problem(403, "PERMISSION_DENIED", "Otra organización");
    }
    match (seen.method.as_str(), route) {
        ("GET", "tenant/document-readings") => {
            if let Some(status) = state.list_status {
                return problem(status, "DEPENDENCY_UNAVAILABLE", "No disponible");
            }
            if !state.list_failures.is_empty() {
                let status = state.list_failures.remove(0);
                return problem(status, "DEPENDENCY_UNAVAILABLE", "No disponible por un momento");
            }
            let items: Vec<Value> = (0..state.listed)
                .map(|n| {
                    json!({ "id": format!("ajena-{n}"), "status": "succeeded", "stage": null, "requestedAt": "2026-01-01T00:00:00Z",
                            "failure": null, "filename": "otra.pdf", "requestedBy": "persona-de-prueba", "ownership": "personal",
                            "pages": 1, "characters": 10, "cancellable": false, "analysis": null })
                })
                .collect();
            let mut items = items;
            items.splice(0..0, state.readings.iter().rev().filter(|r| !r.deleted).map(request_json));
            (200, json!({ "items": items, "nextCursor": null }))
        }
        ("POST", "tenant/document-readings") => {
            let asked: Value = serde_json::from_slice(&seen.body).unwrap_or(Value::Null);
            let filename = asked["filename"].as_str().unwrap_or("").to_string();
            if !filename.ends_with(".jpg") && !filename.ends_with(".png") && !filename.ends_with(".pdf") {
                return problem(422, "document_reading.filename_not_allowed", "El nombre no se admite");
            }
            let sha256 = asked["contentSha256"].as_str().unwrap_or("").to_string();
            // The same file, in progress or read: that reading, not another.
            if let Some(found) = state.readings.iter().find(|r| {
                !r.deleted && !sha256.is_empty() && r.sha256 == sha256 && (r.status == "in_progress" || r.status == "succeeded")
            }) {
                let upload = if found.stage.as_deref() == Some("awaiting_input") { upload_json(url, found) } else { Value::Null };
                return (200, json!({ "request": request_json(found), "upload": upload }));
            }
            let reading = FakeReading {
                id: format!("lectura-{}", state.readings.len() + 1),
                filename,
                sha256,
                status: "in_progress".into(),
                stage: Some("awaiting_input".into()),
                failure: None,
                looks: 0,
                uploaded: None,
                outcome: state.outcome.clone(),
                deleted: false,
            };
            let answer = json!({ "request": request_json(&reading), "upload": upload_json(url, &reading) });
            state.readings.push(reading);
            if !state.request_failures.is_empty() {
                let status = state.request_failures.remove(0);
                return problem(status, "DEPENDENCY_UNAVAILABLE", "La respuesta se perdió");
            }
            (202, answer)
        }
        (method, route) => {
            let Some(rest) = route.strip_prefix("tenant/document-readings/") else {
                return problem(404, "not_found", "No existe");
            };
            let (id, action) = rest.split_once('/').unwrap_or((rest, ""));
            if method == "DELETE" && !state.delete_failures.is_empty() {
                let status = state.delete_failures.remove(0);
                return problem(status, "DEPENDENCY_UNAVAILABLE", "No se pudo borrar");
            }
            let (looks_to_end, model_version) = (state.looks_to_end, state.model_version.clone());
            let Some(reading) = state.readings.iter_mut().find(|r| r.id == id && !r.deleted) else {
                return problem(404, "not_found", "No existe");
            };
            match (method, action) {
                ("POST", "uploaded") => {
                    if reading.uploaded.is_none() {
                        return problem(409, "document_reading.upload_not_found", "No llegó el documento");
                    }
                    if reading.stage.as_deref() == Some("awaiting_input") {
                        reading.stage = Some("queued".into());
                    }
                    (200, request_json(reading))
                }
                ("GET", "") => {
                    if reading.status == "in_progress" {
                        reading.looks += 1;
                        let started = reading.stage.as_deref() != Some("awaiting_input");
                        if reading.looks >= looks_to_end && (started || matches!(reading.outcome, Outcome::Expires)) {
                            match reading.outcome.clone() {
                                Outcome::Read(_) => reading.status = "succeeded".into(),
                                Outcome::Fails(reason) => {
                                    reading.status = "failed".into();
                                    reading.failure = Some(reason.into());
                                }
                                Outcome::Expires => {
                                    reading.status = "expired".into();
                                    reading.failure = Some("El documento no llegó a tiempo.".into());
                                }
                                Outcome::NeverEnds => {}
                            }
                            if reading.status != "in_progress" {
                                reading.stage = None;
                            }
                        } else if started {
                            reading.stage = Some("awaiting_provider".into());
                        }
                    }
                    (200, request_json(reading))
                }
                ("GET", "result") => {
                    if reading.status != "succeeded" {
                        return problem(409, "document_reading.not_finished", "No terminó");
                    }
                    (200, result_json(reading, &model_version))
                }
                ("DELETE", "") => {
                    reading.deleted = true;
                    (204, Value::Null)
                }
                _ => problem(404, "not_found", "No existe"),
            }
        }
    }
}
