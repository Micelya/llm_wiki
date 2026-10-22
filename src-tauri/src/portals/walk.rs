//! The walk over one record, the same for every portal (D-449).
//!
//! 1. Reach the record: if the view already shows it, stay; otherwise go
//!    to the portal's form, fill it, and press «Consultar». When the
//!    portal asks for a person (its challenge, signing in), unlock the
//!    view, say so, and only look until the page changes (or the person
//!    says to go on); then lock it again and go on.
//! 2. Walk its pages: read the table of entries, take each new document,
//!    press «Siguiente», wait until the table changes; until there is no
//!    «Siguiente» or the page does not change.
//! 3. For each entry with a document: if the app already downloaded it
//!    (and it is still there with its size), or the base already has it
//!    by its name, leave it; otherwise download it, check that it is a
//!    PDF, move it to the record's folder with its data next to it.
//!    One request at a time, with a pause between two.
//! 4. Check what was downloaded (that each file is there with its size)
//!    and keep the record's file after every document: a cut leaves what
//!    was done, and the next update goes on from there.
//!
//! The walk drives a `Browser`: the real view (`view.rs`) or, in the
//! tests, a portal made up from invented pages.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use super::page::{Page, Snapshot, VALUE};
use super::store::{self, state, DocumentData, Entry, EntryKey, KnownFiles, RecordFile, STAGING};
use super::{CaseKey, Connector, RecordPage, Row, Screen};

/// The pause between two requests to the portal (D-446).
pub const PAUSE: Duration = Duration::from_millis(700);

/// What the lock says while the app works.
pub const WORKING_NOTE: &str = "Micelya está usando esta ventana: esperá a que termine";

/// The result of asking for a document.
#[derive(Debug, Clone, PartialEq)]
pub enum Fetched {
    /// Saved where it was asked.
    Done,
    /// No download started (the link took the view somewhere else).
    NotStarted,
    /// The link is not on the page any more.
    Missing,
    Failed(String),
}

/// What the walk needs from a view of the portal.
pub trait Browser {
    fn navigate(&mut self, url: &str) -> Result<(), String>;
    /// The page; `act` marks what the app may press or fill.
    fn snapshot(&mut self, act: bool) -> Result<Snapshot, String>;
    /// Presses an element; `false` when it is not on the page any more.
    fn click(&mut self, mark: &str) -> Result<bool, String>;
    /// Fills a field; `false` when it is not on the page any more.
    fn fill(&mut self, mark: &str, value: &str) -> Result<bool, String>;
    /// Asks for the document of a link and waits until it is saved at
    /// `destination`.
    fn download(&mut self, mark: &str, destination: &Path) -> Result<Fetched, String>;
    fn lock(&mut self, locked: bool, note: &str);
    fn wait(&mut self, time: Duration);
    fn cancelled(&self) -> bool;
    /// Whether the person said they finished what the portal asked.
    fn take_go_on(&mut self) -> bool;
    fn report(&mut self, progress: &Progress);
}

/// How the walk goes, for the page.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// `working`, `needs-person`, `done`, `stopped` or `error`.
    pub state: String,
    pub message: String,
    pub reference: Option<String>,
    /// What the portal asks a person: `challenge` or `login`.
    pub person: Option<String>,
    pub page: Option<u32>,
    /// Entries seen, with a document, downloaded now, already there,
    /// failed.
    pub seen: usize,
    pub with_document: usize,
    pub downloaded: usize,
    pub already: usize,
    pub failed: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub enum Stop {
    Cancelled,
    Failed(String),
}

/// The walk's settings: the portal, where its records are kept, what the
/// base already has and the times to wait.
pub struct Walk<'a> {
    pub connector: &'a dyn Connector,
    pub portal_folder: PathBuf,
    pub known: &'a dyn KnownFiles,
    /// Between two requests to the portal.
    pub pause: Duration,
    /// Between two looks while waiting for a page to change.
    pub poll: Duration,
    /// How many looks before a page that does not change is taken as
    /// final.
    pub looks: usize,
    /// Between two looks while a person does what the portal asks.
    pub person_poll: Duration,
}

impl<'a> Walk<'a> {
    pub fn new(connector: &'a dyn Connector, portal_folder: PathBuf, known: &'a dyn KnownFiles) -> Self {
        Walk {
            connector,
            portal_folder,
            known,
            pause: PAUSE,
            poll: Duration::from_millis(300),
            looks: 100,
            person_poll: Duration::from_millis(1500),
        }
    }
}

#[derive(Default)]
struct Run {
    progress: Progress,
    seen: HashSet<EntryKey>,
    downloaded: HashSet<EntryKey>,
    already: HashSet<EntryKey>,
    failed: HashSet<EntryKey>,
    attempts: HashMap<EntryKey, u32>,
}

impl Run {
    fn count(&mut self) {
        self.progress.downloaded = self.downloaded.len();
        self.progress.already = self.already.len();
        self.progress.failed = self.failed.len();
    }
}

enum End {
    Done,
    /// The record is not on the view any more (a challenge, the session
    /// ended): reach it again.
    Again,
}

enum Step {
    Next,
    Again,
}

#[derive(PartialEq)]
enum Waited {
    PageChanged,
    GoOn,
}

impl Walk<'_> {
    /// Brings what is new of a record.
    pub fn run(&self, browser: &mut dyn Browser, key: &CaseKey) -> Result<Progress, Stop> {
        let folder = store::record_folder(&self.portal_folder, key);
        let mut record = store::load(&folder).map_err(Stop::Failed)?.unwrap_or_else(|| RecordFile::new(self.connector.id(), key));
        store::save(&folder, &record).map_err(Stop::Failed)?;
        let mut run = Run::default();
        run.progress.state = "working".to_string();
        run.progress.reference = Some(key.display());
        browser.lock(true, WORKING_NOTE);
        self.say(browser, &mut run, format!("Buscando el expediente {} en el portal…", key.display()));
        let mut rounds = 0;
        loop {
            rounds += 1;
            if rounds > 8 {
                return Err(Stop::Failed("El portal interrumpió el recorrido demasiadas veces: probá más tarde".to_string()));
            }
            self.reach(browser, &mut run, key)?;
            match self.pages(browser, &mut run, key, &folder, &mut record)? {
                End::Done => break,
                End::Again => continue,
            }
        }
        self.verify(&folder, &mut record, &mut run);
        let summary = summary(&record, &run);
        record.updated_at = Some(store::now_text());
        record.last_result = Some(summary.clone());
        store::save(&folder, &record).map_err(Stop::Failed)?;
        run.progress.state = "done".to_string();
        run.progress.message = summary;
        run.progress.person = None;
        browser.report(&run.progress);
        Ok(run.progress)
    }

    fn say(&self, browser: &mut dyn Browser, run: &mut Run, message: String) {
        run.count();
        run.progress.message = message;
        browser.report(&run.progress);
    }

    fn check(&self, browser: &dyn Browser) -> Result<(), Stop> {
        if browser.cancelled() {
            Err(Stop::Cancelled)
        } else {
            Ok(())
        }
    }

    /// The page of the view, read again until it is there.
    fn page(&self, browser: &mut dyn Browser, act: bool) -> Result<Page, Stop> {
        for attempt in 1..=6u32 {
            self.check(browser)?;
            match browser.snapshot(act) {
                Ok(snapshot) if snapshot.error.is_none() && !snapshot.html.is_empty() && snapshot.ready != "loading" => {
                    return Ok(Page::parse(&snapshot.url, &snapshot.html));
                }
                _ => browser.wait(self.poll * attempt),
            }
        }
        Err(Stop::Failed("El portal no responde: revisá la conexión y probá de nuevo".to_string()))
    }

    fn go_start(&self, browser: &mut dyn Browser) -> Result<(), Stop> {
        self.check(browser)?;
        browser.navigate(self.connector.start_url()).map_err(Stop::Failed)?;
        browser.wait(self.poll);
        Ok(())
    }

    /// Takes the view to the record.
    fn reach(&self, browser: &mut dyn Browser, run: &mut Run, key: &CaseKey) -> Result<(), Stop> {
        let mut navigated = false;
        let mut submits = 0;
        let mut fills = 0;
        let mut unknown = 0;
        let mut person_done = false;
        loop {
            let page = self.page(browser, true)?;
            match self.connector.screen(&page) {
                Screen::Record => match self.connector.record(&page).data.shown {
                    Some(shown) if &shown != key => {
                        if submits > 0 {
                            return Err(Stop::Failed(format!(
                                "El portal muestra el expediente {} y no el pedido ({})",
                                shown.display(),
                                key.display()
                            )));
                        }
                        if navigated {
                            return Err(Stop::Failed("El portal no vuelve al formulario de consulta".to_string()));
                        }
                        self.go_start(browser)?;
                        navigated = true;
                    }
                    _ => return Ok(()),
                },
                screen @ (Screen::Login | Screen::Challenge) => {
                    self.wait_person(browser, run, screen)?;
                }
                Screen::Form { challenge } => {
                    if submits >= 3 {
                        let message = self.connector.message(&page).unwrap_or_else(|| "El portal no mostró el expediente pedido".to_string());
                        return Err(Stop::Failed(message));
                    }
                    let mut press = true;
                    if !person_done {
                        self.say(browser, run, "Completando el formulario del portal…".to_string());
                        let plan = self.connector.form(&page, key).map_err(Stop::Failed)?;
                        let mut gone = false;
                        for (mark, value) in &plan.fills {
                            // A field that already says it is left alone:
                            // a list that redraws the form when it changes
                            // is changed once.
                            let now = page.els.iter().find(|e| e.mark() == Some(mark.as_str())).and_then(|e| e.attr(VALUE));
                            if now == Some(value.as_str()) {
                                continue;
                            }
                            browser.wait(self.pause.min(Duration::from_millis(200)));
                            if !browser.fill(mark, value).map_err(Stop::Failed)? {
                                gone = true;
                                break;
                            }
                        }
                        if gone {
                            fills += 1;
                            if fills > 5 {
                                return Err(Stop::Failed("El formulario del portal cambia mientras se completa: probá de nuevo".to_string()));
                            }
                            continue;
                        }
                        // A form that did not move on after «Consultar»
                        // asks for something only a person sees.
                        if challenge || submits > 0 {
                            person_done = true;
                            press = self.wait_person(browser, run, Screen::Form { challenge: true })? == Waited::GoOn;
                        }
                    }
                    if !press {
                        // The person pressed «Consultar», or the page
                        // changed: read it again.
                        continue;
                    }
                    let page = self.page(browser, true)?;
                    if !matches!(self.connector.screen(&page), Screen::Form { .. }) {
                        continue;
                    }
                    let plan = self.connector.form(&page, key).map_err(Stop::Failed)?;
                    let Some(submit) = plan.submit else {
                        return Err(Stop::Failed("No encontré el botón «Consultar» del portal".to_string()));
                    };
                    self.say(browser, run, "Consultando el expediente…".to_string());
                    browser.wait(self.pause);
                    if !browser.click(&submit).map_err(Stop::Failed)? {
                        continue;
                    }
                    submits += 1;
                    person_done = false;
                    let before = self.connector.message(&page);
                    for _ in 0..self.looks {
                        browser.wait(self.poll);
                        let after = self.page(browser, false)?;
                        let screen = self.connector.screen(&after);
                        if !matches!(screen, Screen::Form { .. }) || after.url != page.url || self.connector.message(&after) != before {
                            break;
                        }
                    }
                }
                Screen::Other => {
                    if !navigated {
                        self.go_start(browser)?;
                        navigated = true;
                        continue;
                    }
                    unknown += 1;
                    if unknown > self.looks {
                        return Err(Stop::Failed(
                            "No reconozco esta pantalla del portal. Con «Usar la ventana yo» podés llevarla al formulario de consulta y volver a intentar.".to_string(),
                        ));
                    }
                    browser.wait(self.poll);
                }
            }
        }
    }

    /// Unlocks the view for what only a person does, and only looks at it
    /// until the page changes or the person says to go on.
    fn wait_person(&self, browser: &mut dyn Browser, run: &mut Run, screen: Screen) -> Result<Waited, Stop> {
        let tell = |run: &mut Run, screen: &Screen| {
            let (person, message) = match screen {
                Screen::Login => ("login", "El portal pide iniciar sesión: hacelo en la ventana del portal. Micelya sigue sola cuando entres."),
                _ => (
                    "challenge",
                    "El portal pide resolver un desafío: resolvelo en la ventana del portal y tocá «Consultar». Micelya sigue sola cuando la página cambie.",
                ),
            };
            run.progress.state = "needs-person".to_string();
            run.progress.person = Some(person.to_string());
            run.progress.message = message.to_string();
        };
        tell(run, &screen);
        browser.report(&run.progress);
        let mut start = self.page(browser, false)?;
        let mut current = self.connector.screen(&start);
        let result = loop {
            if browser.cancelled() {
                browser.lock(true, WORKING_NOTE);
                return Err(Stop::Cancelled);
            }
            // Every turn: a new page of the portal starts locked.
            browser.lock(false, "");
            if browser.take_go_on() {
                break Waited::GoOn;
            }
            browser.wait(self.person_poll);
            let page = self.page(browser, false)?;
            let now = self.connector.screen(&page);
            if page.url == start.url && now == current {
                continue;
            }
            if now.needs_person() {
                // Something else for the person (signing in after the
                // challenge...): go on waiting.
                tell(run, &now);
                browser.report(&run.progress);
                current = now;
                start = page;
                continue;
            }
            break Waited::PageChanged;
        };
        browser.lock(true, WORKING_NOTE);
        run.progress.state = "working".to_string();
        run.progress.person = None;
        self.say(browser, run, "Gracias: Micelya sigue sola.".to_string());
        Ok(result)
    }

    /// Walks the pages of the record from the one the view shows.
    fn pages(&self, browser: &mut dyn Browser, run: &mut Run, key: &CaseKey, folder: &Path, record: &mut RecordFile) -> Result<End, Stop> {
        let mut signatures: HashSet<String> = HashSet::new();
        let mut retries = 0;
        loop {
            let page = self.page(browser, true)?;
            if self.connector.screen(&page) != Screen::Record {
                return Ok(End::Again);
            }
            let shown = self.connector.record(&page);
            if !signatures.insert(shown.signature.clone()) {
                return Ok(End::Done);
            }
            run.progress.page = shown.page_number;
            take_data(record, &shown);
            if let Some(full) = self.connector.rows_per_page() {
                if shown.next.is_some() && shown.rows.len() != full {
                    let warning = format!(
                        "La página {} mostró {} actuaciones y no {full}",
                        shown.page_number.map(|n| n.to_string()).unwrap_or_else(|| "?".to_string()),
                        shown.rows.len()
                    );
                    if !run.progress.warnings.contains(&warning) {
                        run.progress.warnings.push(warning);
                    }
                }
            }
            for row in &shown.rows {
                if let Step::Again = self.entry(browser, run, key, folder, record, row)? {
                    return Ok(End::Again);
                }
            }
            store::save(folder, record).map_err(Stop::Failed)?;
            let Some(next) = shown.next.clone() else {
                return Ok(End::Done);
            };
            let following = shown.page_number.map(|n| format!(" {}", n + 1)).unwrap_or_default();
            self.say(browser, run, format!("Pasando a la página{following}…"));
            browser.wait(self.pause);
            if !browser.click(&next).map_err(Stop::Failed)? {
                retries += 1;
                if retries > 3 {
                    return Err(Stop::Failed("La página del portal cambia sola: probá de nuevo".to_string()));
                }
                signatures.remove(&shown.signature);
                continue;
            }
            let mut changed = false;
            for _ in 0..self.looks {
                browser.wait(self.poll);
                let after = self.page(browser, false)?;
                if self.connector.screen(&after) != Screen::Record {
                    return Ok(End::Again);
                }
                if self.connector.record(&after).signature != shown.signature {
                    changed = true;
                    break;
                }
            }
            if !changed {
                run.progress.warnings.push(format!(
                    "La página{} no cambió al pedir la siguiente: se tomó como la última",
                    shown.page_number.map(|n| format!(" {n}")).unwrap_or_default()
                ));
                return Ok(End::Done);
            }
        }
    }

    /// One entry of the table.
    fn entry(&self, browser: &mut dyn Browser, run: &mut Run, key: &CaseKey, folder: &Path, record: &mut RecordFile, row: &Row) -> Result<Step, Stop> {
        self.check(browser)?;
        let entry_key = EntryKey::of(row);
        let index = match record.entry_index(&entry_key) {
            Some(index) => {
                let entry = &mut record.entries[index];
                if entry.office.is_none() {
                    entry.office = row.office.clone();
                }
                if row.download.is_some() && !entry.has_document {
                    entry.has_document = true;
                    entry.state = state::PENDING.to_string();
                }
                index
            }
            None => {
                record.entries.push(Entry::of(row));
                record.entries.len() - 1
            }
        };
        if run.seen.insert(entry_key.clone()) {
            run.progress.seen += 1;
            if row.download.is_some() {
                run.progress.with_document += 1;
            }
        }
        let Some(link) = &row.download else {
            record.entries[index].state = state::NO_DOCUMENT.to_string();
            return Ok(Step::Next);
        };
        // Downloaded before, and still there with its size.
        let entry = &record.entries[index];
        let kept = match entry.state.as_str() {
            state::DOWNLOADED => entry.file.as_ref().is_some_and(|f| same_size(&folder.join(f), entry.size)),
            state::ALREADY => entry.file.as_ref().is_some_and(|f| Path::new(f).exists()),
            _ => false,
        };
        if kept {
            if !run.downloaded.contains(&entry_key) {
                run.already.insert(entry_key);
            }
            return Ok(Step::Next);
        }
        // Already in the base, by its name.
        let twins = record.twins(&entry_key) > 1;
        if let Some(found) = self.known.find(key, &entry_key, twins) {
            let entry = &mut record.entries[index];
            match found.path.strip_prefix(folder).ok().filter(|_| found.path.parent() == Some(folder)) {
                Some(own) => {
                    entry.state = state::DOWNLOADED.to_string();
                    entry.file = Some(own.to_string_lossy().to_string());
                }
                None => {
                    entry.state = state::ALREADY.to_string();
                    entry.file = Some(found.path.to_string_lossy().to_string());
                }
            }
            entry.size = Some(found.size);
            entry.error = None;
            run.already.insert(entry_key);
            store::save(folder, record).map_err(Stop::Failed)?;
            return Ok(Step::Next);
        }
        if run.attempts.get(&entry_key).copied().unwrap_or(0) >= 2 {
            return Ok(Step::Next);
        }
        // Download it.
        let taken: Vec<String> = record.entries.iter().filter_map(|e| e.file.clone()).collect();
        let name = store::free_name(folder, &store::document_name(key, &record.entries[index]), &taken);
        self.say(browser, run, format!("Bajando {name}"));
        let staging = self.portal_folder.join(STAGING);
        fs::create_dir_all(&staging).map_err(|e| Stop::Failed(format!("No se pudo crear {}: {e}", staging.display())))?;
        let temporary = staging.join(format!("{}.pdf", uuid::Uuid::new_v4()));
        browser.wait(self.pause);
        let fetched = browser.download(&link.mark, &temporary).map_err(Stop::Failed)?;
        *run.attempts.entry(entry_key.clone()).or_insert(0) += u32::from(fetched != Fetched::Missing);
        let checked = match fetched {
            Fetched::Done => pdf_size(&temporary),
            Fetched::Missing => {
                // The page changed under the walk: read it again.
                let _ = fs::remove_file(&temporary);
                return Ok(Step::Again);
            }
            Fetched::NotStarted => Err("La descarga no empezó".to_string()),
            Fetched::Failed(why) => Err(why),
        };
        match checked {
            Ok(size) => {
                fs::create_dir_all(folder).map_err(|e| Stop::Failed(e.to_string()))?;
                let destination = folder.join(&name);
                fs::rename(&temporary, &destination)
                    .or_else(|_| fs::copy(&temporary, &destination).map(|_| ()).and_then(|_| fs::remove_file(&temporary)))
                    .map_err(|e| Stop::Failed(format!("No se pudo guardar {name}: {e}")))?;
                let entry = &mut record.entries[index];
                entry.state = state::DOWNLOADED.to_string();
                entry.file = Some(name.clone());
                entry.size = Some(size);
                entry.downloaded_at = Some(store::now_text());
                entry.source = link.href.clone();
                entry.error = None;
                let data = DocumentData {
                    portal: self.connector.folder().to_string(),
                    reference: key.display(),
                    date: entry.date.clone(),
                    folios: entry.folios.clone(),
                    kind: entry.kind.clone(),
                    detail: entry.detail.clone(),
                    office: entry.office.clone(),
                    source: entry.source.clone(),
                    downloaded_at: entry.downloaded_at.clone().unwrap_or_default(),
                    file: name,
                    size,
                };
                store::write_document_data(&destination, &data).map_err(Stop::Failed)?;
                run.failed.remove(&entry_key);
                run.downloaded.insert(entry_key);
                store::save(folder, record).map_err(Stop::Failed)?;
                Ok(Step::Next)
            }
            Err(why) => {
                let _ = fs::remove_file(&temporary);
                let entry = &mut record.entries[index];
                entry.state = state::FAILED.to_string();
                entry.error = Some(why);
                run.failed.insert(entry_key);
                store::save(folder, record).map_err(Stop::Failed)?;
                // A failure can be the session that ended: look at the
                // page (only look: marking it again would lose the marks
                // of the other entries).
                let page = self.page(browser, false)?;
                if self.connector.screen(&page) != Screen::Record {
                    return Ok(Step::Again);
                }
                Ok(Step::Next)
            }
        }
    }

    /// Every document downloaded is in the folder with its size.
    fn verify(&self, folder: &Path, record: &mut RecordFile, run: &mut Run) {
        for entry in &mut record.entries {
            if entry.state != state::DOWNLOADED {
                continue;
            }
            let there = entry.file.as_ref().is_some_and(|f| same_size(&folder.join(f), entry.size));
            if !there {
                entry.state = state::FAILED.to_string();
                entry.error = Some("El archivo no está en la carpeta o cambió de tamaño: se baja de nuevo en la próxima actualización".to_string());
                run.failed.insert(entry.key());
                run.downloaded.remove(&entry.key());
            }
        }
        run.count();
        let missing = run.progress.with_document.saturating_sub(run.progress.downloaded + run.progress.already + run.progress.failed);
        if missing > 0 {
            run.progress.warnings.push(format!("{missing} documentos quedaron sin bajar: se intentan en la próxima actualización"));
        }
    }
}

fn take_data(record: &mut RecordFile, shown: &RecordPage) {
    let data = &shown.data;
    if data.title.is_some() {
        record.data.title = data.title.clone();
    }
    if data.jurisdiction.is_some() {
        record.data.jurisdiction = data.jurisdiction.clone();
    }
    if data.office.is_some() {
        record.data.office = data.office.clone();
    }
    if data.situation.is_some() {
        record.data.situation = data.situation.clone();
    }
    for linked in &shown.linked {
        if !record.linked.iter().any(|l| l.key == linked.key) && linked.key != record.key {
            record.linked.push(linked.clone());
        }
    }
}

fn same_size(path: &Path, size: Option<u64>) -> bool {
    fs::metadata(path).map(|m| Some(m.len()) == size && m.len() > 0).unwrap_or(false)
}

/// The size of a downloaded file, if it is a PDF.
fn pdf_size(path: &Path) -> Result<u64, String> {
    let size = fs::metadata(path).map(|m| m.len()).map_err(|_| "El documento no llegó a guardarse".to_string())?;
    if size == 0 {
        return Err("El documento llegó vacío".to_string());
    }
    let mut head = Vec::new();
    fs::File::open(path)
        .and_then(|f| f.take(1024).read_to_end(&mut head))
        .map_err(|e| format!("No se pudo leer el documento: {e}"))?;
    if !head.windows(5).any(|w| w == b"%PDF-") {
        return Err("El portal devolvió otra cosa que un PDF (puede que la sesión haya vencido)".to_string());
    }
    Ok(size)
}

fn summary(record: &RecordFile, run: &Run) -> String {
    let p = &run.progress;
    let without = record.count(state::NO_DOCUMENT);
    let mut parts = vec![format!(
        "{}: {} {} nuevos, {} ya estaban",
        record.reference,
        p.downloaded,
        if p.downloaded == 1 { "documento" } else { "documentos" },
        p.already
    )];
    if without > 0 {
        parts.push(format!("{without} actuaciones sin documento"));
    }
    if p.failed > 0 {
        parts.push(format!("{} no se pudieron bajar", p.failed));
    }
    format!("{}.", parts.join("; "))
}
