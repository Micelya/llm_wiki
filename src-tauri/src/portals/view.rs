//! The portal's view inside the main window, and the commands of the
//! «Portales» page.
//!
//! - The view is a webview child of the main window (`Window::add_child`),
//!   placed over the area the page leaves for it, with its own profile
//!   (`<app local data>/portales/<portal>/perfil`): the portal's session
//!   stays there, as in any browser. The app never asks for nor keeps a
//!   password.
//! - It has no permission of the app: the capabilities are for the app's
//!   own webview, the portal's pages are remote (Tauri refuses the app's
//!   commands to a remote page that no capability names, and every app
//!   command is refused to any webview but the app's own, `lib.rs`), and
//!   it can only go to the portal's addresses. Nothing is downloaded but
//!   what the app asks for.
//! - The app reads it with its own scripts and `eval_with_callback`
//!   (`scripts.rs`): the page never sends anything to the app.
//! - One runner, one record at a time, from a queue. The page follows it
//!   by the `portal://status` event, sent only to the app's webview.

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, WebviewBuilder};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, Url, Webview, WebviewUrl};

use super::page::Snapshot;
use super::store::{self, Followed, KnownFiles, NoneKnown};
use super::walk::{Browser, Fetched, Progress, Stop, Walk};
use super::{connector, expedientes_folder, scripts, CaseKey, Connector, APP_WEBVIEW};
use crate::docbase::commands::{DocBaseState, CHANGED_EVENT};
use crate::docbase::DocBase;

/// The event the page follows the runner by.
pub const STATUS_EVENT: &str = "portal://status";

/// What the lock says while nothing is being done.
const IDLE_NOTE: &str = "Micelya maneja esta ventana: pedí un expediente en la página y se habilita sola cuando el portal te necesite";

fn view_label(connector: &dyn Connector) -> String {
    format!("portal-{}", connector.id())
}

#[derive(Default)]
pub struct PortalState {
    shared: Arc<Shared>,
}

#[derive(Default)]
struct Shared {
    control: Mutex<Control>,
    signal: Condvar,
    download: Mutex<Slot>,
    download_signal: Condvar,
}

impl Shared {
    fn control(&self) -> MutexGuard<'_, Control> {
        self.control.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn slot(&self) -> MutexGuard<'_, Slot> {
        self.download.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

#[derive(Default)]
struct Control {
    queue: VecDeque<Job>,
    running: Option<Job>,
    runner: bool,
    cancel: bool,
    go_on: bool,
    manual: bool,
    progress: Progress,
}

#[derive(Debug, Clone, PartialEq)]
struct Job {
    portal: &'static str,
    key: CaseKey,
}

/// The download the app asked for: where it goes, and how it went.
#[derive(Default)]
struct Slot {
    target: Option<PathBuf>,
    started: Option<String>,
    finished: Option<bool>,
}

/// What the page shows of the runner.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortalStatus {
    pub running: Option<String>,
    pub queue: Vec<String>,
    pub manual: bool,
    pub progress: Progress,
}

fn status_of(control: &Control) -> PortalStatus {
    PortalStatus {
        running: control.running.as_ref().map(|job| job.key.display()),
        queue: control.queue.iter().map(|job| job.key.display()).collect(),
        manual: control.manual,
        progress: control.progress.clone(),
    }
}

fn emit_status(app: &AppHandle, shared: &Shared) {
    let status = status_of(&shared.control());
    let _ = app.emit_to(APP_WEBVIEW, STATUS_EVENT, status);
}

/// A download of the view: only the one the app asked for goes, to where
/// the app said; any other is refused.
fn on_download(shared: &Shared, event: DownloadEvent<'_>) -> bool {
    match event {
        DownloadEvent::Requested { url, destination } => {
            let mut slot = shared.slot();
            match (&slot.target, &slot.started) {
                (Some(target), None) => {
                    *destination = target.clone();
                    slot.started = Some(url.to_string());
                    shared.download_signal.notify_all();
                    true
                }
                _ => false,
            }
        }
        DownloadEvent::Finished { url, success, .. } => {
            let mut slot = shared.slot();
            if slot.started.as_deref() == Some(url.as_str()) {
                slot.finished = Some(success);
                shared.download_signal.notify_all();
            }
            true
        }
        _ => true,
    }
}

/// Creates the view the first time, over the area the page gives it.
fn ensure_view(app: &AppHandle, shared: &Arc<Shared>, connector: &'static dyn Connector, position: LogicalPosition<f64>, size: LogicalSize<f64>) -> Result<Webview, String> {
    let label = view_label(connector);
    if let Some(view) = app.get_webview(&label) {
        return Ok(view);
    }
    let window = app.get_window(APP_WEBVIEW).ok_or("No se encontró la ventana de la app")?;
    let profile = app
        .path()
        .app_local_data_dir()
        .map_err(|e| format!("No se encontró la carpeta de datos de la app: {e}"))?
        .join("portales")
        .join(connector.id())
        .join("perfil");
    fs::create_dir_all(&profile).map_err(|e| format!("No se pudo crear {}: {e}", profile.display()))?;
    let start = Url::parse(connector.start_url()).map_err(|e| e.to_string())?;
    let downloads = shared.clone();
    let loads = shared.clone();
    let builder = WebviewBuilder::new(label, WebviewUrl::External(start))
        .data_directory(profile)
        .initialization_script(scripts::INIT)
        .on_navigation(move |url| connector.allows(url))
        .on_new_window(|_, _| NewWindowResponse::Deny)
        .on_download(move |_, event| on_download(&downloads, event))
        .on_page_load(move |view, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            // A new page starts locked: unlock it again if it is the
            // person's turn, or say what the lock is for.
            let control = loads.control();
            let person = control.running.is_some() && control.progress.state == "needs-person";
            if control.manual || person {
                let _ = view.eval(scripts::lock(false, ""));
            } else if control.running.is_none() {
                let _ = view.eval(scripts::lock(true, IDLE_NOTE));
            }
        });
    window.add_child(builder, position, size).map_err(|e| format!("No se pudo abrir el portal: {e}"))
}

/// The view, as the walk drives it.
struct ViewBrowser {
    app: AppHandle,
    shared: Arc<Shared>,
    view: Webview,
}

fn eval(view: &Webview, script: String, timeout: Duration) -> Result<serde_json::Value, String> {
    let (sender, receiver) = std::sync::mpsc::channel::<String>();
    view.eval_with_callback(script, move |result| {
        let _ = sender.send(result);
    })
    .map_err(|e| format!("No se pudo leer el portal: {e}"))?;
    let raw = receiver.recv_timeout(timeout).map_err(|_| "El portal no respondió a tiempo".to_string())?;
    serde_json::from_str(&raw).map_err(|e| format!("Respuesta ilegible del portal: {e}"))
}

impl Browser for ViewBrowser {
    fn navigate(&mut self, url: &str) -> Result<(), String> {
        let url = Url::parse(url).map_err(|e| e.to_string())?;
        self.view.navigate(url).map_err(|e| format!("No se pudo abrir el portal: {e}"))
    }

    fn snapshot(&mut self, act: bool) -> Result<Snapshot, String> {
        let value = eval(&self.view, scripts::snapshot(act), Duration::from_secs(20))?;
        serde_json::from_value(value).map_err(|e| format!("Respuesta ilegible del portal: {e}"))
    }

    fn click(&mut self, mark: &str) -> Result<bool, String> {
        Ok(eval(&self.view, scripts::click(mark), Duration::from_secs(10))?.as_str() == Some("ok"))
    }

    fn fill(&mut self, mark: &str, value: &str) -> Result<bool, String> {
        match eval(&self.view, scripts::fill(mark, value), Duration::from_secs(10))?.as_str() {
            Some("ok") => Ok(true),
            Some("no-option") => Err("Esa opción no está en la lista del portal".to_string()),
            _ => Ok(false),
        }
    }

    fn download(&mut self, mark: &str, destination: &Path) -> Result<Fetched, String> {
        *self.shared.slot() = Slot { target: Some(destination.to_path_buf()), started: None, finished: None };
        let asked = eval(&self.view, scripts::download(mark), Duration::from_secs(10));
        if !matches!(asked.as_ref().map(|v| v.as_str()), Ok(Some("link" | "click"))) {
            *self.shared.slot() = Slot::default();
            return match asked {
                Ok(_) => Ok(Fetched::Missing),
                Err(e) => Err(e),
            };
        }
        let started_by = Instant::now() + Duration::from_secs(30);
        let finished_by = Instant::now() + Duration::from_secs(15 * 60);
        let mut slot = self.shared.slot();
        loop {
            if slot.finished.is_some() {
                break;
            }
            let now = Instant::now();
            let limit = if slot.started.is_none() { started_by } else { finished_by };
            if now >= limit || self.cancelled() {
                let started = slot.started.is_some();
                *slot = Slot::default();
                return Ok(if self.cancelled() {
                    Fetched::Failed("Detenido durante la descarga".to_string())
                } else if started {
                    Fetched::Failed("La descarga tardó demasiado".to_string())
                } else {
                    Fetched::NotStarted
                });
            }
            let wait = (limit - now).min(Duration::from_millis(500));
            slot = self.shared.download_signal.wait_timeout(slot, wait).map(|(s, _)| s).unwrap_or_else(|p| p.into_inner().0);
        }
        let success = slot.finished == Some(true);
        *slot = Slot::default();
        Ok(if success { Fetched::Done } else { Fetched::Failed("La descarga se cortó".to_string()) })
    }

    fn lock(&mut self, locked: bool, note: &str) {
        let _ = self.view.eval(scripts::lock(locked, note));
    }

    fn wait(&mut self, time: Duration) {
        let until = Instant::now() + time;
        while Instant::now() < until && !self.cancelled() {
            std::thread::sleep((until - Instant::now()).min(Duration::from_millis(100)));
        }
    }

    fn cancelled(&self) -> bool {
        self.shared.control().cancel
    }

    fn take_go_on(&mut self) -> bool {
        std::mem::take(&mut self.shared.control().go_on)
    }

    fn report(&mut self, progress: &Progress) {
        self.shared.control().progress = progress.clone();
        emit_status(&self.app, &self.shared);
    }
}

fn start_runner(app: &AppHandle, shared: &Arc<Shared>) {
    {
        let mut control = shared.control();
        if control.runner {
            shared.signal.notify_all();
            return;
        }
        control.runner = true;
    }
    let app = app.clone();
    let shared = shared.clone();
    let spawned = std::thread::Builder::new().name("portal-runner".to_string()).spawn(move || runner(app, shared));
    if let Err(error) = spawned {
        eprintln!("[portals] the runner could not start: {error}");
    }
}

fn runner(app: AppHandle, shared: Arc<Shared>) {
    loop {
        let job = {
            let mut control = shared.control();
            loop {
                if let Some(job) = control.queue.pop_front() {
                    control.running = Some(job.clone());
                    control.cancel = false;
                    control.go_on = false;
                    control.manual = false;
                    break job;
                }
                control = shared.signal.wait(control).unwrap_or_else(|p| p.into_inner());
            }
        };
        emit_status(&app, &shared);
        let result = run_job(&app, &shared, &job);
        {
            let mut control = shared.control();
            let mut progress = control.progress.clone();
            match result {
                Ok(done) => progress = done,
                Err(Stop::Cancelled) => {
                    progress.state = "stopped".to_string();
                    progress.person = None;
                    progress.message = "Detenido. Lo que se bajó queda guardado y la próxima actualización sigue desde ahí.".to_string();
                    control.queue.clear();
                }
                Err(Stop::Failed(message)) => {
                    progress.state = "error".to_string();
                    progress.person = None;
                    progress.message = message;
                }
            }
            control.progress = progress;
            control.running = None;
            control.cancel = false;
        }
        if let Some(view) = connector(job.portal).ok().and_then(|c| app.get_webview(&view_label(c))) {
            let _ = view.eval(scripts::lock(true, IDLE_NOTE));
        }
        emit_status(&app, &shared);
    }
}

fn run_job(app: &AppHandle, shared: &Arc<Shared>, job: &Job) -> Result<Progress, Stop> {
    let connector = connector(job.portal).map_err(Stop::Failed)?;
    let view = app
        .get_webview(&view_label(connector))
        .ok_or_else(|| Stop::Failed("Abrí la página «Portales» para que la app pueda usar el portal".to_string()))?;
    let expedientes = expedientes_folder(app).map_err(Stop::Failed)?;
    let docbase = app.state::<DocBaseState>();
    let base = docbase.shared().ok();
    let known: &dyn KnownFiles = match &base {
        Some(base) => base.as_ref(),
        None => &NoneKnown,
    };
    let walk = Walk::new(connector, expedientes.join(connector.folder()), known);
    let mut browser = ViewBrowser { app: app.clone(), shared: shared.clone(), view };
    let progress = walk.run(&mut browser, &job.key);
    // Whatever happened, what was brought goes to the base.
    if let Some(base) = &base {
        if let Err(error) = integrate(base, &expedientes, connector, &job.key) {
            eprintln!("[portals] the record could not go into the base: {error}");
        }
        docbase.request_sweep();
        let _ = app.emit(CHANGED_EVENT, ());
    }
    progress
}

/// The record's folder in the base (its origin), and its confirmed matter
/// with the documents the base already inventoried.
fn integrate(base: &DocBase, expedientes: &Path, connector: &dyn Connector, key: &CaseKey) -> Result<(), String> {
    let folder = store::record_folder(&expedientes.join(connector.folder()), key);
    let Some(mut record) = store::load(&folder)? else {
        return Ok(());
    };
    if record.updated_at.is_none() && !record.entries.iter().any(|e| e.file.is_some()) {
        return Ok(());
    }
    let (origin, inside) = store::ensure_origin(base, expedientes)?;
    base.scan_origin(&origin)?;
    store::ensure_matter(base, &mut record, &origin, &store::rel_inside(&inside, connector.folder(), key))?;
    store::save(&folder, &record)
}

fn enqueue(app: &AppHandle, state: &PortalState, jobs: Vec<Job>) -> PortalStatus {
    {
        let mut control = state.shared.control();
        for job in jobs {
            if control.running.as_ref() != Some(&job) && !control.queue.contains(&job) {
                control.queue.push_back(job);
            }
        }
    }
    start_runner(app, &state.shared);
    state.shared.signal.notify_all();
    emit_status(app, &state.shared);
    status_of(&state.shared.control())
}

/// Shows the view over the area the page leaves for it (logical pixels of
/// the window), creating it the first time.
#[tauri::command(async)]
pub fn portal_show(app: AppHandle, state: State<'_, PortalState>, portal: String, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let connector = connector(&portal)?;
    if width < 20.0 || height < 20.0 {
        return portal_hide(app, portal);
    }
    let position = LogicalPosition::new(x.max(0.0), y.max(0.0));
    let size = LogicalSize::new(width, height);
    let view = ensure_view(&app, &state.shared, connector, position, size)?;
    view.set_position(position).map_err(|e| e.to_string())?;
    view.set_size(size).map_err(|e| e.to_string())?;
    view.show().map_err(|e| e.to_string())
}

/// Hides the view (the page left); a walk in course goes on.
#[tauri::command(async)]
pub fn portal_hide(app: AppHandle, portal: String) -> Result<(), String> {
    let connector = connector(&portal)?;
    if let Some(view) = app.get_webview(&view_label(connector)) {
        view.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Brings a record (or what is new of one already followed).
#[tauri::command(async)]
pub fn portal_fetch(app: AppHandle, state: State<'_, PortalState>, portal: String, jurisdiction: String, number: String, year: String) -> Result<PortalStatus, String> {
    let connector = connector(&portal)?;
    let key = CaseKey::new(&jurisdiction, &number, &year)?;
    Ok(enqueue(&app, &state, vec![Job { portal: connector.id(), key }]))
}

/// What is new of every record followed in a portal, one after the other.
#[tauri::command(async)]
pub fn portal_update_all(app: AppHandle, state: State<'_, PortalState>, portal: String) -> Result<PortalStatus, String> {
    let connector = connector(&portal)?;
    let folder = expedientes_folder(&app)?.join(connector.folder());
    let jobs = store::followed(&folder).into_iter().map(|(_, record)| Job { portal: connector.id(), key: record.key }).collect();
    Ok(enqueue(&app, &state, jobs))
}

/// Stops the walk in course and empties the queue.
#[tauri::command(async)]
pub fn portal_stop(app: AppHandle, state: State<'_, PortalState>) -> Result<PortalStatus, String> {
    {
        let mut control = state.shared.control();
        control.queue.clear();
        if control.running.is_some() {
            control.cancel = true;
        }
    }
    state.shared.download_signal.notify_all();
    emit_status(&app, &state.shared);
    Ok(status_of(&state.shared.control()))
}

/// The person says they did what the portal asked.
#[tauri::command(async)]
pub fn portal_go_on(state: State<'_, PortalState>) -> Result<(), String> {
    state.shared.control().go_on = true;
    Ok(())
}

/// Lets the person use the view by themselves (when nothing runs), or
/// takes it back.
#[tauri::command(async)]
pub fn portal_manual(app: AppHandle, state: State<'_, PortalState>, portal: String, enabled: bool) -> Result<PortalStatus, String> {
    let connector = connector(&portal)?;
    {
        let mut control = state.shared.control();
        if enabled && control.running.is_some() {
            return Err("Esperá a que termine lo que está haciendo la app, o detenelo".to_string());
        }
        control.manual = enabled;
    }
    if let Some(view) = app.get_webview(&view_label(connector)) {
        let _ = view.eval(if enabled { scripts::lock(false, "") } else { scripts::lock(true, IDLE_NOTE) });
    }
    emit_status(&app, &state.shared);
    Ok(status_of(&state.shared.control()))
}

#[tauri::command(async)]
pub fn portal_status(state: State<'_, PortalState>) -> PortalStatus {
    status_of(&state.shared.control())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedList {
    /// The portal's name, for a person.
    pub name: String,
    pub folder: String,
    pub records: Vec<Followed>,
}

/// The records followed in a portal. Puts in their matters the documents
/// the base inventoried since.
#[tauri::command(async)]
pub fn portal_followed(app: AppHandle, docbase: State<'_, DocBaseState>, portal: String) -> Result<FollowedList, String> {
    let connector = connector(&portal)?;
    let expedientes = expedientes_folder(&app)?;
    let folder = expedientes.join(connector.folder());
    let list = store::followed(&folder);
    if let Ok(base) = docbase.shared() {
        for (_, record) in &list {
            if record.updated_at.is_some() {
                if let Err(error) = integrate_documents(&base, &expedientes, connector, &record.key) {
                    eprintln!("[portals] {error}");
                }
            }
        }
    }
    let list = store::followed(&folder);
    Ok(FollowedList { name: connector.name().to_string(), folder: folder.to_string_lossy().to_string(), records: store::followed_view(&list) })
}

/// Puts in the matter the documents the base inventoried, without
/// sweeping again.
fn integrate_documents(base: &DocBase, expedientes: &Path, connector: &dyn Connector, key: &CaseKey) -> Result<(), String> {
    let folder = store::record_folder(&expedientes.join(connector.folder()), key);
    let Some(mut record) = store::load(&folder)? else {
        return Ok(());
    };
    let (origin, inside) = store::ensure_origin(base, expedientes)?;
    let before = record.matter_id.clone();
    let added = store::ensure_matter(base, &mut record, &origin, &store::rel_inside(&inside, connector.folder(), key))?;
    if added > 0 || before != record.matter_id {
        store::save(&folder, &record)?;
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct Jurisdiction {
    pub code: String,
    pub name: String,
}

#[tauri::command(async)]
pub fn portal_jurisdictions(portal: String) -> Result<Vec<Jurisdiction>, String> {
    let connector = connector(&portal)?;
    Ok(connector.jurisdictions().into_iter().map(|(code, name)| Jurisdiction { code: code.to_string(), name: name.to_string() }).collect())
}

/// Opens the folder of a portal, or of one of its records.
#[tauri::command(async)]
pub fn portal_open_folder(app: AppHandle, portal: String, record: Option<String>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let connector = connector(&portal)?;
    let mut folder = expedientes_folder(&app)?.join(connector.folder());
    if let Some(name) = record {
        if name.contains(['/', '\\']) || name.contains("..") {
            return Err("Esa carpeta no es de un expediente".to_string());
        }
        folder = folder.join(name);
    }
    fs::create_dir_all(&folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
    app.opener()
        .open_path(folder.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("No se pudo abrir la carpeta: {e}"))
}
