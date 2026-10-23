//! The server answered for real, on a port the system chooses (never the
//! app's own 19827), with invented data.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::*;
use crate::docbase::testing::TempDir;
use crate::docbase::DocBase;

/// A server of the tests: its port, and how to stop it.
pub(super) struct Running {
    pub port: u16,
    server: Arc<Server>,
    serving: Option<JoinHandle<()>>,
}

impl Running {
    pub fn start(context: Context) -> Self {
        let server = Arc::new(Server::http("127.0.0.1:0").unwrap());
        let port = server.server_addr().to_ip().unwrap().port();
        let context = Context { port, ..context };
        let serving_server = server.clone();
        let serving = thread::spawn(move || serve(&serving_server, Arc::new(context)));
        Self { port, server, serving: Some(serving) }
    }

    /// Sends one request, as written, and returns the status and the body.
    pub fn send(&self, request: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let status = response.split(' ').nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let body = response.split_once("\r\n\r\n").map(|(_, b)| b.to_string()).unwrap_or_default();
        (status, body)
    }

    pub fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    pub fn get(&self, path: &str, headers: &str) -> (u16, String) {
        self.send(&format!("GET {path} HTTP/1.1\r\nHost: {}\r\n{headers}Connection: close\r\n\r\n", self.host()))
    }

    pub fn post(&self, path: &str, headers: &str, content_type: &str, body: &str) -> (u16, String) {
        self.send(&format!(
            "POST {path} HTTP/1.1\r\nHost: {}\r\n{headers}Content-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            self.host(),
            body.len()
        ))
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.server.unblock();
        if let Some(serving) = self.serving.take() {
            let _ = serving.join();
        }
    }
}

/// A context over `base`, with the clips in `clips`; `saved` is told of
/// each clip.
pub(super) fn context(base: Arc<DocBase>, clips: PathBuf, saved: Arc<Mutex<Vec<PathBuf>>>) -> Context {
    Context {
        port: 0,
        base: Box::new(move || Ok(base.clone())),
        clips: Box::new(move || Ok(clips.clone())),
        clip_saved: Box::new(move |path| saved.lock().unwrap().push(path.to_path_buf())),
    }
}

#[test]
fn a_clip_sent_to_the_server_is_saved_and_the_base_takes_it() {
    let dir = TempDir::new();
    let work = dir.path().join("trabajo");
    crate::work_folder::prepare(&work).unwrap();
    let clips = work.join(crate::work_folder::CLIPS);
    let saved: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
    let base = Arc::new(DocBase::in_memory().unwrap());
    let server = Running::start(context(base.clone(), clips.clone(), saved.clone()));

    // What the installed extension sends, as it always did.
    let extension = "Origin: chrome-extension://abcdefghijklmnop\r\n";
    let (status, body) = server.get("/status", extension);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains(r#""ok":true"#), "{body}");
    let clip = r#"{"title":"Nota inventada","url":"https://ejemplo.invalid/a","content":"Lo que dice la nota inventada, con palabras suficientes para la base."}"#;
    let (status, body) = server.post("/clip", extension, "application/json", clip);
    assert_eq!(status, 200, "{body}");
    assert!(body.contains("Nota inventada.md"), "{body}");
    // Asked before by the browser, for the extension.
    let (status, _) = server.send(&format!(
        "OPTIONS /clip HTTP/1.1\r\nHost: {}\r\n{extension}Access-Control-Request-Method: POST\r\nConnection: close\r\n\r\n",
        server.host()
    ));
    assert_eq!(status, 204);
    // A web site cannot write clips, not even without asking first.
    let (status, _) = server.post("/clip", "Origin: https://sitio.invalid\r\n", "application/json", clip);
    assert_eq!(status, 403);
    let (status, _) = server.post("/clip", "", "text/plain", clip);
    assert_eq!(status, 415);
    let (status, _) = server.post("/clip", "", "application/json", r#"{"title":"Vacío","content":""}"#);
    assert_eq!(status, 400);
    let (status, _) = server.get("/projects", "");
    assert_eq!(status, 404);

    let saved = saved.lock().unwrap().clone();
    assert_eq!(saved.len(), 1);
    let files: Vec<_> = std::fs::read_dir(&clips).unwrap().map(|e| e.unwrap().file_name()).collect();
    assert_eq!(files.len(), 1);

    // The base inventories it like any document, with its text.
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
fn the_server_answers_only_what_is_addressed_to_it_on_the_machine_itself() {
    let dir = TempDir::new();
    let saved: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
    let base = Arc::new(DocBase::in_memory().unwrap());
    let server = Running::start(context(base, dir.path().join("Recortes web"), saved));

    let (status, _) = server.send(&format!("GET /status HTTP/1.1\r\nHost: localhost:{}\r\nConnection: close\r\n\r\n", server.port));
    assert_eq!(status, 200);
    // Another name for the machine: a page that renamed itself to this
    // address («DNS rebinding»).
    for host in ["sitio.invalid".to_string(), format!("sitio.invalid:{}", server.port), "127.0.0.1".to_string(), "127.0.0.1:19827".to_string()] {
        let (status, _) = server.send(&format!("GET /status HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n"));
        assert_eq!(status, 421, "{host}");
    }
    // A web page, also one of the machine itself.
    for origin in ["https://sitio.invalid", "http://localhost:5173", "null"] {
        let (status, _) = server.get("/status", &format!("Origin: {origin}\r\n"));
        assert_eq!(status, 403, "{origin}");
    }
    // It listens on the machine only, at the clipper's address.
    assert_eq!(bind_address(), "127.0.0.1:19827");
}
