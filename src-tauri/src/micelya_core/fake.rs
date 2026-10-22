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
        }
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

fn handle(state: &mut FakeState, _url: &str, seen: &Seen) -> (u16, Value) {
    let (path, _query) = seen.path.split_once('?').unwrap_or((&seen.path, ""));
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
            (200, json!({ "items": items, "nextCursor": null }))
        }
        _ => problem(404, "not_found", "No existe"),
    }
}
