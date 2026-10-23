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
//!   it can only go to the portal's addresses (one of them without
//!   encryption, as the PJN redirects to, is opened with `https`; nothing
//!   is loaded without encryption). It opens no window of its own. Nothing
//!   is downloaded but what the app asks for; WebView2's permission to
//!   download several files in a row is granted by the app itself, only
//!   to the portal's pages, so the person is not asked for it.
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
use tauri::webview::{DownloadEvent, NewWindowResponse, PageLoadEvent, PageLoadPayload, WebviewBuilder};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, State, Url, Webview, WebviewUrl};

use super::page::Snapshot;
use super::store::{self, Followed, KnownFiles, NoneKnown};
use super::walk::{Browser, Fetched, Progress, Stop, Walk};
#[cfg(windows)]
use super::{grants_by_itself, Permission};
use super::{connector, expedientes_folder, navigation, scripts, work_folder, CaseKey, Connector, Navigation, Upgrades, APP_WEBVIEW};
use crate::docbase::commands::{DocBaseState, CHANGED_EVENT};
use crate::docbase::DocBase;

/// The event the page follows the runner by.
pub const STATUS_EVENT: &str = "portal://status";

/// What the lock says while nothing is being done.
const IDLE_NOTE: &str = "Micelya maneja esta ventana: pedí un expediente en la página y se habilita sola cuando el portal te necesite";

/// The start of the label of every portal's view.
const VIEW_PREFIX: &str = "portal-";

fn view_label(connector: &dyn Connector) -> String {
    format!("{VIEW_PREFIX}{}", connector.id())
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
    /// One show or hide of the view at a time (`show_view`).
    views: Mutex<()>,
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
    /// Go through every page of the record («Revisar todo el expediente»).
    full: bool,
}

impl Job {
    fn same_record(&self, other: &Job) -> bool {
        self.portal == other.portal && self.key == other.key
    }
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

/// An area of the app's window, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Place {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Where the portals' views are: the app's window, or a fake in the tests.
pub(crate) trait Views {
    type View;
    fn find(&self, label: &str) -> Option<Self::View>;
    fn create(&self, label: &str, place: Place) -> Result<Self::View, String>;
    /// Puts the view over `place` and shows it.
    fn place(&self, view: &Self::View, place: Place) -> Result<(), String>;
    fn hide(&self, view: &Self::View) -> Result<(), String>;
}

/// Shows the view over `place`. The view outlives the page (it leaves, it
/// reloads, and in development React mounts it twice), so one that exists
/// is placed and shown again, never created a second time; the gate keeps
/// two requests from creating it at once.
pub(crate) fn show_view<V: Views>(views: &V, gate: &Mutex<()>, label: &str, place: Place) -> Result<(), String> {
    let _one_at_a_time = gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let view = match views.find(label) {
        Some(view) => view,
        None => match views.create(label, place) {
            Ok(view) => view,
            // Made meanwhile by something that does not pass the gate.
            Err(error) => views.find(label).ok_or(error)?,
        },
    };
    views.place(&view, place)
}

/// Hides the view, when there is one, so it does not cover other pages.
pub(crate) fn hide_view<V: Views>(views: &V, gate: &Mutex<()>, label: &str) -> Result<(), String> {
    let _one_at_a_time = gate.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match views.find(label) {
        Some(view) => views.hide(&view),
        None => Ok(()),
    }
}

/// The views of a portal inside the app's window.
struct AppViews<'a> {
    app: &'a AppHandle,
    shared: &'a Arc<Shared>,
    connector: &'static dyn Connector,
}

impl Views for AppViews<'_> {
    type View = Webview;

    fn find(&self, label: &str) -> Option<Webview> {
        self.app.get_webview(label)
    }

    fn create(&self, label: &str, place: Place) -> Result<Webview, String> {
        create_view(self.app, self.shared, self.connector, label, place)
    }

    fn place(&self, view: &Webview, place: Place) -> Result<(), String> {
        view.set_position(LogicalPosition::new(place.x, place.y)).map_err(|e| e.to_string())?;
        view.set_size(LogicalSize::new(place.width, place.height)).map_err(|e| e.to_string())?;
        view.show().map_err(|e| e.to_string())
    }

    fn hide(&self, view: &Webview) -> Result<(), String> {
        view.hide().map_err(|e| e.to_string())
    }
}

/// When the app's own page loads again (a reload), hides the portals'
/// views: they outlive it and would cover whatever the new page shows
/// until the «Portales» page asks for them again. It does not wait for the
/// gate: this runs on the main thread, which a view being created waits
/// for.
pub fn hide_on_app_reload<R: tauri::Runtime>(webview: &Webview<R>, payload: &PageLoadPayload<'_>) {
    if payload.event() != PageLoadEvent::Started || !super::is_app_webview(webview.label()) {
        return;
    }
    for (label, view) in webview.app_handle().webviews() {
        if label.starts_with(VIEW_PREFIX) {
            let _ = view.hide();
        }
    }
}

/// Creates the view over the area the page gives it.
fn create_view(app: &AppHandle, shared: &Arc<Shared>, connector: &'static dyn Connector, label: &str, place: Place) -> Result<Webview, String> {
    let position = LogicalPosition::new(place.x, place.y);
    let size = LogicalSize::new(place.width, place.height);
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
    let upgrades = Mutex::new(Upgrades::default());
    let (navigator, own_label) = (app.clone(), label.to_string());
    let builder = WebviewBuilder::new(label.to_string(), WebviewUrl::External(start))
        .data_directory(profile)
        .initialization_script(scripts::INIT)
        // Also called for the server's redirects (WebView2's
        // NavigationStarting, `wry` 0.55.1), as the PJN's after «Consultar»
        // to `http://…/expediente.seam`: that one is cancelled and the same
        // address opened with `https`.
        .on_navigation(move |url| match navigation(connector, url) {
            Navigation::Allow => true,
            Navigation::Upgrade(secure) => {
                let admitted = upgrades.lock().unwrap_or_else(|p| p.into_inner()).admit(&secure, Instant::now());
                if admitted {
                    open_later(&navigator, &own_label, secure);
                } else {
                    eprintln!("[portals] the portal keeps sending the view to http: {url} is not followed again");
                }
                false
            }
            Navigation::Refuse => false,
        })
        // No window of its own: the challenge and the query are in the
        // page. A request of the portal is only told (for the real test).
        .on_new_window(move |url, _| {
            if !matches!(navigation(connector, &url), Navigation::Refuse) {
                eprintln!("[portals] the portal asked to open a window (refused): {url}");
            }
            NewWindowResponse::Deny
        })
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
    let view = window.add_child(builder, position, size).map_err(|e| format!("No se pudo abrir el portal: {e}"))?;
    grant_downloads(&view, connector);
    Ok(view)
}

/// Grants by itself, only to the portal's pages, WebView2's permission to
/// download several files in a row (`grants_by_itself`): without it,
/// WebView2 asks the person to accept that the site downloads documents
/// once the app asks for the second one. Neither Tauri nor `wry` 0.55.1 hand
/// this event, so it is taken from WebView2 itself (`with_webview`, on the
/// main thread); every other permission keeps WebView2's own answer.
#[cfg(windows)]
fn grant_downloads(view: &Webview, connector: &'static dyn Connector) {
    use webview2_com::Microsoft::Web::WebView2::Win32::{
        COREWEBVIEW2_PERMISSION_KIND, COREWEBVIEW2_PERMISSION_KIND_MULTIPLE_AUTOMATIC_DOWNLOADS, COREWEBVIEW2_PERMISSION_STATE_ALLOW,
    };
    use webview2_com::{take_pwstr, PermissionRequestedEventHandler};
    let asked = view.with_webview(move |platform| {
        let handler = PermissionRequestedEventHandler::create(Box::new(move |_, args| {
            let Some(args) = args else { return Ok(()) };
            let mut kind = COREWEBVIEW2_PERMISSION_KIND::default();
            unsafe { args.PermissionKind(&mut kind)? };
            if kind != COREWEBVIEW2_PERMISSION_KIND_MULTIPLE_AUTOMATIC_DOWNLOADS {
                return Ok(());
            }
            let mut origin = windows_core::PWSTR::null();
            unsafe { args.Uri(&mut origin)? };
            let origin = take_pwstr(origin);
            if grants_by_itself(connector, Permission::MultipleDownloads, &origin) {
                unsafe { args.SetState(COREWEBVIEW2_PERMISSION_STATE_ALLOW)? };
            } else {
                eprintln!("[portals] a page out of the portal asked to download several files: {origin} (left to WebView2)");
            }
            Ok(())
        }));
        let mut token = 0i64;
        let registered = unsafe { platform.controller().CoreWebView2().and_then(|core| core.add_PermissionRequested(&handler, &mut token)) };
        if let Err(error) = registered {
            eprintln!("[portals] the permission to download several documents could not be granted: {error}");
        }
    });
    if let Err(error) = asked {
        eprintln!("[portals] the permission to download several documents could not be granted: {error}");
    }
}

#[cfg(not(windows))]
fn grant_downloads(_view: &Webview, _connector: &'static dyn Connector) {}

/// Opens an address in the view once the navigation being decided is
/// over. The decision runs on the main thread, where Tauri would carry out
/// a navigation at once, inside WebView2's own event (`send_user_message`
/// of `tauri-runtime-wry`); from another thread it is queued to the event
/// loop instead, and nothing waits for it.
fn open_later(app: &AppHandle, label: &str, url: Url) {
    let (app, label) = (app.clone(), label.to_string());
    let spawned = std::thread::Builder::new().name("portal-https".to_string()).spawn(move || {
        if let Some(view) = app.get_webview(&label) {
            if let Err(error) = view.navigate(url) {
                eprintln!("[portals] the https address could not be opened: {error}");
            }
        }
    });
    if let Err(error) = spawned {
        eprintln!("[portals] the https address could not be opened: {error}");
    }
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

    fn focus(&mut self, on: bool) {
        let _ = self.view.eval(scripts::focus(on));
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
    let work = work_folder(app).map_err(Stop::Failed)?;
    let expedientes = expedientes_folder(app).map_err(Stop::Failed)?;
    let docbase = app.state::<DocBaseState>();
    let base = docbase.shared().ok();
    let known: &dyn KnownFiles = match &base {
        Some(base) => base.as_ref(),
        None => &NoneKnown,
    };
    let mut walk = Walk::new(connector, expedientes.join(connector.folder()), known);
    walk.full = job.full;
    let mut browser = ViewBrowser { app: app.clone(), shared: shared.clone(), view };
    let progress = walk.run(&mut browser, &job.key);
    // Whatever happened, what was brought goes to the base.
    if let Some(base) = &base {
        if let Err(error) = integrate(base, &work, &expedientes, connector, &job.key) {
            eprintln!("[portals] the record could not go into the base: {error}");
        }
        docbase.request_sweep();
        let _ = app.emit(CHANGED_EVENT, ());
    }
    progress
}

/// The record's folder in the base (its origin), and its confirmed matter
/// with the documents the base already inventoried.
fn integrate(base: &DocBase, work: &Path, expedientes: &Path, connector: &dyn Connector, key: &CaseKey) -> Result<(), String> {
    let folder = store::record_folder(&expedientes.join(connector.folder()), key);
    let Some(mut record) = store::load(&folder)? else {
        return Ok(());
    };
    if record.updated_at.is_none() && !record.entries.iter().any(|e| e.file.is_some()) {
        return Ok(());
    }
    let (origin, inside) = store::ensure_origin(base, work, expedientes)?;
    base.scan_origin(&origin)?;
    store::ensure_matter(base, &mut record, &origin, &store::rel_inside(&inside, connector.folder(), key))?;
    store::save(&folder, &record)
}

/// Puts a record in the queue once: one already waiting is asked to go
/// through it all if this one asks; one running now is queued again only
/// when this one goes through more of it.
fn queue_job(control: &mut Control, job: Job) {
    if let Some(waiting) = control.queue.iter_mut().find(|waiting| waiting.same_record(&job)) {
        waiting.full |= job.full;
        return;
    }
    let running = control.running.as_ref().is_some_and(|running| running.same_record(&job) && (running.full || !job.full));
    if !running {
        control.queue.push_back(job);
    }
}

fn enqueue(app: &AppHandle, state: &PortalState, jobs: Vec<Job>) -> PortalStatus {
    {
        let mut control = state.shared.control();
        for job in jobs {
            queue_job(&mut control, job);
        }
    }
    start_runner(app, &state.shared);
    state.shared.signal.notify_all();
    emit_status(app, &state.shared);
    status_of(&state.shared.control())
}

/// Shows the view over the area the page leaves for it (logical pixels of
/// the window), creating it only if there is none.
#[tauri::command(async)]
pub fn portal_show(app: AppHandle, state: State<'_, PortalState>, portal: String, x: f64, y: f64, width: f64, height: f64) -> Result<(), String> {
    let connector = connector(&portal)?;
    let views = AppViews { app: &app, shared: &state.shared, connector };
    if width < 20.0 || height < 20.0 {
        return hide_view(&views, &state.shared.views, &view_label(connector));
    }
    let place = Place { x: x.max(0.0), y: y.max(0.0), width, height };
    show_view(&views, &state.shared.views, &view_label(connector), place)
}

/// Hides the view (the page left); a walk in course goes on.
#[tauri::command(async)]
pub fn portal_hide(app: AppHandle, state: State<'_, PortalState>, portal: String) -> Result<(), String> {
    let connector = connector(&portal)?;
    let views = AppViews { app: &app, shared: &state.shared, connector };
    hide_view(&views, &state.shared.views, &view_label(connector))
}

/// Brings a record (or what is new of one already followed); `full`: goes
/// through every page of it, also those the app already has whole.
#[tauri::command(async)]
pub fn portal_fetch(
    app: AppHandle,
    state: State<'_, PortalState>,
    portal: String,
    jurisdiction: String,
    number: String,
    year: String,
    full: Option<bool>,
) -> Result<PortalStatus, String> {
    let connector = connector(&portal)?;
    let key = CaseKey::new(&jurisdiction, &number, &year)?;
    Ok(enqueue(&app, &state, vec![Job { portal: connector.id(), key, full: full.unwrap_or(false) }]))
}

/// What is new of every record followed in a portal, one after the other.
#[tauri::command(async)]
pub fn portal_update_all(app: AppHandle, state: State<'_, PortalState>, portal: String) -> Result<PortalStatus, String> {
    let connector = connector(&portal)?;
    let folder = expedientes_folder(&app)?.join(connector.folder());
    let jobs = store::followed(&folder).into_iter().map(|(_, record)| Job { portal: connector.id(), key: record.key, full: false }).collect();
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
    let work = work_folder(&app)?;
    let expedientes = expedientes_folder(&app)?;
    let folder = expedientes.join(connector.folder());
    let list = store::followed(&folder);
    if let Ok(base) = docbase.shared() {
        for (_, record) in &list {
            if record.updated_at.is_some() {
                if let Err(error) = integrate_documents(&base, &work, &expedientes, connector, &record.key) {
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
fn integrate_documents(base: &DocBase, work: &Path, expedientes: &Path, connector: &dyn Connector, key: &CaseKey) -> Result<(), String> {
    let folder = store::record_folder(&expedientes.join(connector.folder()), key);
    let Some(mut record) = store::load(&folder)? else {
        return Ok(());
    };
    let (origin, inside) = store::ensure_origin(base, work, expedientes)?;
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

#[cfg(test)]
mod tests {
    //! Showing and hiding the portal's view, with the views of a fake
    //! window: the page mounts twice, leaves, comes back, or asks twice at
    //! once.

    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::{hide_view, show_view, Place, Views};

    const LABEL: &str = "portal-zzz";
    const AREA: Place = Place { x: 400.0, y: 120.0, width: 800.0, height: 600.0 };
    const MOVED: Place = Place { x: 460.0, y: 120.0, width: 740.0, height: 600.0 };

    #[derive(Debug, Clone, PartialEq)]
    struct Shown {
        visible: bool,
        place: Place,
    }

    /// A window that makes its views as Tauri does: it checks the label,
    /// takes a while to make the view, and only then keeps it.
    #[derive(Default)]
    struct FakeWindow {
        views: Mutex<HashMap<String, Shown>>,
        created: AtomicUsize,
        slow: Duration,
        /// Making the view fails with this.
        refuse: Option<&'static str>,
        /// Someone else makes the view while this one fails.
        made_meanwhile: bool,
    }

    impl FakeWindow {
        fn shown(&self) -> Option<Shown> {
            self.views.lock().unwrap().get(LABEL).cloned()
        }
    }

    impl Views for FakeWindow {
        type View = String;

        fn find(&self, label: &str) -> Option<String> {
            self.views.lock().unwrap().contains_key(label).then(|| label.to_string())
        }

        fn create(&self, label: &str, place: Place) -> Result<String, String> {
            if self.views.lock().unwrap().contains_key(label) {
                return Err(format!("a webview with label `{label}` already exists"));
            }
            std::thread::sleep(self.slow);
            if let Some(problem) = self.refuse {
                if self.made_meanwhile {
                    self.views.lock().unwrap().insert(label.to_string(), Shown { visible: true, place });
                }
                return Err(problem.to_string());
            }
            self.created.fetch_add(1, Ordering::SeqCst);
            self.views.lock().unwrap().insert(label.to_string(), Shown { visible: true, place });
            Ok(label.to_string())
        }

        fn place(&self, view: &String, place: Place) -> Result<(), String> {
            let mut views = self.views.lock().unwrap();
            let shown = views.get_mut(view).ok_or("no view")?;
            shown.visible = true;
            shown.place = place;
            Ok(())
        }

        fn hide(&self, view: &String) -> Result<(), String> {
            let mut views = self.views.lock().unwrap();
            views.get_mut(view).ok_or("no view")?.visible = false;
            Ok(())
        }
    }

    #[test]
    fn a_record_is_queued_once_and_a_whole_review_is_not_lost() {
        use super::{queue_job, Control, Job};
        let key = crate::portals::CaseKey::new("ZZZ", "4321", "2011").unwrap();
        let other = crate::portals::CaseKey::new("ZZZ", "1", "2012").unwrap();
        let job = |key: &crate::portals::CaseKey, full: bool| Job { portal: "pjn", key: key.clone(), full };
        let mut control = Control::default();
        queue_job(&mut control, job(&key, false));
        queue_job(&mut control, job(&key, false));
        queue_job(&mut control, job(&other, false));
        assert_eq!(control.queue.len(), 2);
        // Asked to go through it all while it waits: the one waiting does.
        queue_job(&mut control, job(&key, true));
        assert_eq!(control.queue.len(), 2);
        assert!(control.queue[0].full);
        // While it runs: again only if this one goes through more of it.
        let mut control = Control { running: Some(job(&key, false)), ..Control::default() };
        queue_job(&mut control, job(&key, false));
        assert!(control.queue.is_empty());
        queue_job(&mut control, job(&key, true));
        assert_eq!(control.queue.len(), 1);
        let mut control = Control { running: Some(job(&key, true)), ..Control::default() };
        queue_job(&mut control, job(&key, false));
        assert!(control.queue.is_empty());
    }

    #[test]
    fn the_view_is_made_once_and_then_placed_again() {
        let window = FakeWindow::default();
        let gate = Mutex::new(());
        show_view(&window, &gate, LABEL, AREA).unwrap();
        show_view(&window, &gate, LABEL, AREA).unwrap();
        show_view(&window, &gate, LABEL, MOVED).unwrap();
        assert_eq!(window.created.load(Ordering::SeqCst), 1);
        assert_eq!(window.shown(), Some(Shown { visible: true, place: MOVED }));
    }

    #[test]
    fn leaving_the_page_hides_it_and_coming_back_shows_it_again() {
        let window = FakeWindow::default();
        let gate = Mutex::new(());
        show_view(&window, &gate, LABEL, AREA).unwrap();
        hide_view(&window, &gate, LABEL).unwrap();
        assert_eq!(window.shown().map(|s| s.visible), Some(false));
        show_view(&window, &gate, LABEL, AREA).unwrap();
        assert_eq!(window.shown(), Some(Shown { visible: true, place: AREA }));
        assert_eq!(window.created.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn hiding_before_there_is_a_view_does_nothing() {
        let window = FakeWindow::default();
        hide_view(&window, &Mutex::new(()), LABEL).unwrap();
        assert_eq!(window.shown(), None);
        assert_eq!(window.created.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn two_requests_at_once_make_one_view_and_neither_fails() {
        let window = Arc::new(FakeWindow { slow: Duration::from_millis(60), ..FakeWindow::default() });
        let gate = Arc::new(Mutex::new(()));
        let requests: Vec<_> = [AREA, MOVED]
            .into_iter()
            .map(|place| {
                let (window, gate) = (window.clone(), gate.clone());
                std::thread::spawn(move || show_view(window.as_ref(), &gate, LABEL, place))
            })
            .collect();
        for request in requests {
            request.join().unwrap().unwrap();
        }
        assert_eq!(window.created.load(Ordering::SeqCst), 1);
        assert_eq!(window.shown().map(|s| s.visible), Some(true));
    }

    #[test]
    fn a_view_made_meanwhile_is_used_instead_of_failing() {
        let window = FakeWindow { refuse: Some("a webview with label `portal-zzz` already exists"), made_meanwhile: true, ..FakeWindow::default() };
        show_view(&window, &Mutex::new(()), LABEL, MOVED).unwrap();
        assert_eq!(window.shown(), Some(Shown { visible: true, place: MOVED }));
    }

    #[test]
    fn a_view_that_cannot_be_made_says_why() {
        let window = FakeWindow { refuse: Some("No se pudo abrir el portal"), ..FakeWindow::default() };
        let problem = show_view(&window, &Mutex::new(()), LABEL, AREA).unwrap_err();
        assert_eq!(problem, "No se pudo abrir el portal");
        assert_eq!(window.shown(), None);
    }
}
