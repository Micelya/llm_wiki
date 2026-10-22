//! The web clipper's local server (P-227, D-450, D-451). The «Micelya
//! Clipper» browser extension (`extension/`) sends here the page it
//! clipped; the server saves it as a markdown file in `<carpeta de
//! trabajo>\Recortes web\`, with the page's address and the date at the
//! top, and asks the base for a sweep: the base inventories it like any
//! document, and Atención says there are new clips.
//!
//! Only `GET /status` and `POST /clip`. It listens on the machine itself
//! (127.0.0.1:19827). A request from anywhere else needs the token of
//! `MICELYA_CLIP_TOKEN`; a page of a web site cannot use it (its `Origin`
//! is refused, and a clip has to come as JSON, which a site cannot send
//! across origins without the browser asking first).

use std::io::Read;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Local};
use serde::Deserialize;
use tauri::{AppHandle, Manager};
use tiny_http::{Header, Method, Request, Response, Server};

pub const PORT: u16 = 19827;
/// A clip larger than this is not a page.
const MAX_BODY_BYTES: u64 = 8 * 1024 * 1024;
const TOKEN_ENV: &str = "MICELYA_CLIP_TOKEN";
const BIND_ATTEMPTS: u32 = 3;

/// 0 starting, 1 listening, 2 the port is taken, 3 stopped.
static STATE: AtomicU8 = AtomicU8::new(0);

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

/// What the server needs from the app.
pub struct Clipper {
    /// Where the clips go, asked on every clip (the work folder can change).
    pub folder: Box<dyn Fn() -> Result<PathBuf, String> + Send + Sync>,
    /// The token a request from another machine needs, if there is one.
    pub token: Option<String>,
    /// Told of every clip saved.
    pub saved: Box<dyn Fn(&Path) + Send + Sync>,
}

/// Starts the server in its own thread.
pub fn start(app: AppHandle) {
    thread::spawn(move || {
        let address = format!("127.0.0.1:{PORT}");
        let mut server = None;
        for attempt in 1..=BIND_ATTEMPTS {
            match Server::http(&address) {
                Ok(bound) => {
                    server = Some(bound);
                    break;
                }
                Err(error) => {
                    eprintln!("[clip-server] {address} is not available ({attempt}/{BIND_ATTEMPTS}): {error}");
                    thread::sleep(Duration::from_secs(2));
                }
            }
        }
        let Some(server) = server else {
            STATE.store(2, Ordering::Relaxed);
            return;
        };
        STATE.store(1, Ordering::Relaxed);
        let handle = app.clone();
        let clipper = Clipper {
            folder: Box::new(move || crate::work_folder::clips_folder(&handle)),
            token: std::env::var(TOKEN_ENV).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty()),
            saved: Box::new(move |path| {
                eprintln!("[clip-server] saved {}", path.display());
                let docbase = app.state::<crate::docbase::commands::DocBaseState>();
                docbase.request_sweep();
            }),
        };
        for request in server.incoming_requests() {
            handle_request(request, &clipper);
        }
        STATE.store(3, Ordering::Relaxed);
    });
}

/// What the extension sends.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Clip {
    pub title: String,
    pub url: String,
    pub content: String,
}

/// Answers one request.
pub fn handle_request(mut request: Request, clipper: &Clipper) {
    let origin = header(&request, "Origin");
    if let Some(origin) = origin.as_deref() {
        if !is_allowed_origin(origin) {
            return answer(request, 403, r#"{"ok":false,"error":"Este origen no puede usar el recortador"}"#.to_string(), None);
        }
    }
    let origin = origin.as_deref();
    if request.method() == &Method::Options {
        return answer(request, 204, String::new(), origin);
    }
    if !is_loopback(request.remote_addr()) && !has_token(&request, clipper.token.as_deref()) {
        return answer(request, 401, r#"{"ok":false,"error":"Falta el token o no es el correcto"}"#.to_string(), origin);
    }
    let path = request.url().split('?').next().unwrap_or("").to_string();
    match (request.method().clone(), path.as_str()) {
        (Method::Get, "/status") => answer(request, 200, r#"{"ok":true,"app":"Micelya"}"#.to_string(), origin),
        (Method::Post, "/clip") => {
            let json = header(&request, "Content-Type").is_some_and(|value| value.to_lowercase().starts_with("application/json"));
            if !json {
                return answer(request, 415, r#"{"ok":false,"error":"El recorte tiene que llegar como JSON"}"#.to_string(), origin);
            }
            let mut body = String::new();
            if let Err(error) = request.as_reader().take(MAX_BODY_BYTES + 1).read_to_string(&mut body) {
                return answer(request, 400, error_json(&format!("No se pudo leer el pedido: {error}")), origin);
            }
            if body.len() as u64 > MAX_BODY_BYTES {
                return answer(request, 413, r#"{"ok":false,"error":"El recorte es demasiado grande"}"#.to_string(), origin);
            }
            let result = serde_json::from_str::<Clip>(&body)
                .map_err(|error| format!("El pedido no es un recorte: {error}"))
                .and_then(|clip| {
                    let folder = (clipper.folder)()?;
                    save_clip(&folder, &clip, Local::now())
                });
            match result {
                Ok(path) => {
                    (clipper.saved)(&path);
                    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    answer(request, 200, serde_json::json!({ "ok": true, "file": name }).to_string(), origin)
                }
                Err(error) => answer(request, 400, error_json(&error), origin),
            }
        }
        _ => answer(request, 404, r#"{"ok":false,"error":"No existe"}"#.to_string(), origin),
    }
}

fn error_json(message: &str) -> String {
    serde_json::json!({ "ok": false, "error": message }).to_string()
}

fn header(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.as_str().as_str().eq_ignore_ascii_case(name))
        .map(|header| header.value.as_str().to_string())
}

fn answer(request: Request, status: u16, body: String, origin: Option<&str>) {
    let mut response = Response::from_string(body).with_status_code(status);
    let mut headers = vec![
        ("Content-Type", "application/json; charset=utf-8".to_string()),
        ("Access-Control-Allow-Methods", "GET, POST, OPTIONS".to_string()),
        ("Access-Control-Allow-Headers", "Content-Type, Authorization".to_string()),
        ("Access-Control-Max-Age", "600".to_string()),
    ];
    if let Some(origin) = origin.filter(|origin| is_allowed_origin(origin)) {
        headers.push(("Access-Control-Allow-Origin", origin.to_string()));
        headers.push(("Vary", "Origin".to_string()));
        headers.push(("Access-Control-Allow-Private-Network", "true".to_string()));
    }
    for (name, value) in headers {
        if let Ok(header) = Header::from_bytes(name.as_bytes(), value.as_bytes()) {
            response.add_header(header);
        }
    }
    let _ = request.respond(response);
}

/// The browser extensions, and pages of the machine itself.
pub fn is_allowed_origin(origin: &str) -> bool {
    origin.starts_with("chrome-extension://")
        || origin.starts_with("moz-extension://")
        || origin == "http://localhost"
        || origin.starts_with("http://localhost:")
        || origin == "http://127.0.0.1"
        || origin.starts_with("http://127.0.0.1:")
}

fn is_loopback(address: Option<&SocketAddr>) -> bool {
    address.is_some_and(|address| address.ip().is_loopback())
}

fn has_token(request: &Request, token: Option<&str>) -> bool {
    let Some(token) = token else {
        return false;
    };
    header(request, "Authorization")
        .and_then(|value| value.strip_prefix("Bearer ").map(str::to_string))
        .is_some_and(|given| same(given.as_bytes(), token.as_bytes()))
}

/// Compares without telling how much matched.
fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Saves a clip in `folder` as `<aaaa-mm-dd> <título>.md`, with the page's
/// address and the date at the top. Never overwrites: a name already there
/// gets ` (2)`, ` (3)`…
pub fn save_clip(folder: &Path, clip: &Clip, now: DateTime<Local>) -> Result<PathBuf, String> {
    let content = clip.content.trim();
    if content.is_empty() {
        return Err("El recorte llegó vacío".to_string());
    }
    std::fs::create_dir_all(folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
    let title = one_line(&clip.title);
    let title = if title.is_empty() { "Recorte sin título".to_string() } else { title };
    let stem = format!("{} {}", now.format("%Y-%m-%d"), file_title(&title));
    let mut path = folder.join(format!("{stem}.md"));
    let mut n = 2;
    while path.exists() {
        path = folder.join(format!("{stem} ({n}).md"));
        n += 1;
    }
    let url = one_line(&clip.url);
    let markdown = format!(
        "Dirección: {}\nFecha del recorte: {}\n\n# {}\n\n{}\n",
        if url.is_empty() { "(sin dirección)" } else { url.as_str() },
        now.format("%Y-%m-%d %H:%M"),
        title,
        content
    );
    std::fs::write(&path, markdown).map_err(|e| format!("No se pudo guardar el recorte: {e}"))?;
    Ok(path)
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The title as a file name: without the signs Windows does not take, up
/// to 80 letters.
fn file_title(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { ' ' } else { c })
        .collect();
    let clean = one_line(&clean);
    let short: String = clean.chars().take(80).collect();
    let short = short.trim_end_matches(['.', ' ']).to_string();
    if short.is_empty() {
        "Recorte".to_string()
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;
    use crate::docbase::DocBase;
    use chrono::TimeZone;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::{Arc, Mutex};

    fn moment() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 3, 4, 9, 30, 0).unwrap()
    }

    #[test]
    fn a_clip_is_a_markdown_file_with_the_address_and_the_date_at_the_top() {
        let dir = TempDir::new();
        let folder = dir.path().join("Recortes web");
        let clip = Clip {
            title: "Una página: de prueba / inventada?".to_string(),
            url: "https://ejemplo.invalid/nota".to_string(),
            content: "Texto de la página inventada.".to_string(),
        };
        let path = save_clip(&folder, &clip, moment()).unwrap();
        assert_eq!(path.file_name().unwrap().to_string_lossy(), "2026-03-04 Una página de prueba inventada.md");
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("Dirección: https://ejemplo.invalid/nota\nFecha del recorte: 2026-03-04 09:30\n\n# Una página: de prueba / inventada?\n\n"), "{text}");
        assert!(text.contains("Texto de la página inventada."));
        // The same page again is another file, never the same one written over.
        let again = save_clip(&folder, &clip, moment()).unwrap();
        assert_eq!(again.file_name().unwrap().to_string_lossy(), "2026-03-04 Una página de prueba inventada (2).md");
        assert!(save_clip(&folder, &Clip { content: "  ".to_string(), ..clip }, moment()).is_err());
    }

    /// Sends one request to the server and returns the status and the body.
    fn send(port: u16, request: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let body = response.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }

    fn post(path: &str, origin: Option<&str>, content_type: &str, body: &str) -> String {
        let origin = origin.map(|o| format!("Origin: {o}\r\n")).unwrap_or_default();
        format!(
            "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n{origin}Content-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    }

    #[test]
    fn a_clip_sent_to_the_server_is_saved_and_the_base_takes_it() {
        let dir = TempDir::new();
        let work = dir.path().join("trabajo");
        crate::work_folder::prepare(&work).unwrap();
        let clips = work.join(crate::work_folder::CLIPS);
        let saved: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let told = saved.clone();
        let target = clips.clone();
        let clipper = Clipper {
            folder: Box::new(move || Ok(target.clone())),
            token: None,
            saved: Box::new(move |path| told.lock().unwrap().push(path.to_path_buf())),
        };
        let server = Server::http("127.0.0.1:0").unwrap();
        let port = server.server_addr().to_ip().unwrap().port();
        let requests = 6;
        let serving = thread::spawn(move || {
            for _ in 0..requests {
                let request = server.recv().unwrap();
                handle_request(request, &clipper);
            }
        });

        let (status, body) = send(port, "GET /status HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        assert_eq!(status, 200, "{body}");
        let clip = r#"{"title":"Nota inventada","url":"https://ejemplo.invalid/a","content":"Lo que dice la nota inventada, con palabras suficientes para la base."}"#;
        let (status, body) = send(port, &post("/clip", Some("chrome-extension://abcdef"), "application/json", clip));
        assert_eq!(status, 200, "{body}");
        assert!(body.contains("Nota inventada.md"), "{body}");
        // A web site cannot write clips, not even without asking first.
        let (status, _) = send(port, &post("/clip", Some("https://sitio.invalid"), "application/json", clip));
        assert_eq!(status, 403);
        let (status, _) = send(port, &post("/clip", None, "text/plain", clip));
        assert_eq!(status, 415);
        let (status, _) = send(port, &post("/clip", None, "application/json", r#"{"title":"Vacío","content":""}"#));
        assert_eq!(status, 400);
        let (status, _) = send(port, "GET /projects HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
        assert_eq!(status, 404);
        serving.join().unwrap();

        let saved = saved.lock().unwrap().clone();
        assert_eq!(saved.len(), 1);
        let files: Vec<_> = std::fs::read_dir(&clips).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(files.len(), 1);

        // The base inventories it like any document, with its text.
        let base = DocBase::in_memory().unwrap();
        let origin = crate::work_folder::register(&base, &work).unwrap().origin;
        base.scan_origin(&origin.id).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
        while base.extract_pending(50).unwrap() > 0 {}
        let (folder, state): (String, String) = base
            .conn()
            .query_row(
                "SELECT l.folder, c.state FROM location l JOIN file f ON f.hash = l.file_hash JOIN content c ON c.id = f.content_id",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(folder, "Recortes web");
        assert_eq!(state, "processed");
        let text: String = base.conn().query_row("SELECT body FROM derivative WHERE kind = 'text'", [], |row| row.get(0)).unwrap();
        assert!(text.contains("https://ejemplo.invalid/a"), "{text}");
        let new = base.files_new_since(&clips, "clips-seen-at", 10).unwrap();
        assert_eq!(new.total, 1);
    }

    #[test]
    fn only_the_extensions_and_the_machine_itself_are_allowed() {
        for origin in ["chrome-extension://abc", "moz-extension://abc", "http://localhost:5173", "http://127.0.0.1:8080"] {
            assert!(is_allowed_origin(origin), "{origin}");
        }
        for origin in ["", "https://sitio.invalid", "http://localhost.sitio.invalid", "http://127.0.0.1.sitio.invalid", "null"] {
            assert!(!is_allowed_origin(origin), "{origin}");
        }
        let lan: SocketAddr = "192.168.1.20:5000".parse().unwrap();
        let local: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        assert!(is_loopback(Some(&local)) && !is_loopback(Some(&lan)) && !is_loopback(None));
        assert!(same(b"abc", b"abc") && !same(b"abc", b"abd") && !same(b"abc", b"ab"));
    }
}
