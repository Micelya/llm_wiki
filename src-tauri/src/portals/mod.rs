//! Judicial portals inside the app (P-248, D-446 to D-449): a browser view
//! of the portal in the «Portales» page, which the app drives by itself and
//! hands to the person only when the portal asks for a person (its
//! anti-robot challenge, or signing in). The app walks the record's
//! entries, downloads each new document one at a time and keeps them in
//! `<carpeta de trabajo>\Expedientes\<portal>\<record>\`, with their data
//! next to them; the document base inventories that folder and each record
//! becomes a confirmed matter.
//!
//! What every portal shares lives here: the view and its lock (`view`),
//! the walk over the record (`walk`), the files and their data (`store`).
//! What only one portal knows (its labels, its table of entries, its pager,
//! how it downloads) is its connector (`pjn`).
//!
//! Design: `docs/design/portales-judiciales-propuesta.md`.

pub mod page;
pub mod pjn;
pub mod scripts;
pub mod store;
pub mod view;
pub mod walk;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use page::Page;

/// The label of the app's own webview: the only one that may call the
/// app's commands (the portal's view never can, `lib.rs`).
pub const APP_WEBVIEW: &str = "main";

/// Whether a webview is the app's own and may call its commands.
pub fn is_app_webview(label: &str) -> bool {
    label == APP_WEBVIEW
}

/// The folder where the records brought from the portals are kept:
/// `<carpeta de trabajo>\Expedientes` (`work_folder`, D-450). It is part of
/// the work folder's origin, never an origin of its own (D-451).
pub fn expedientes_folder(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::work_folder::records_folder(app)
}

/// The work folder that holds the records' folder.
pub fn work_folder(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::work_folder::folder(app)
}

/// One record of a portal: its jurisdiction (the portal's code, "CIV"),
/// its number and its year.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CaseKey {
    #[serde(rename = "jurisdiccion")]
    pub jurisdiction: String,
    #[serde(rename = "numero")]
    pub number: u64,
    #[serde(rename = "anio")]
    pub year: u32,
}

impl CaseKey {
    pub fn new(jurisdiction: &str, number: &str, year: &str) -> Result<CaseKey, String> {
        let jurisdiction = jurisdiction.trim().to_uppercase();
        if jurisdiction.is_empty() || jurisdiction.len() > 6 || !jurisdiction.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err("Elegí la jurisdicción".to_string());
        }
        let digits: String = number.chars().filter(|c| !c.is_whitespace() && *c != '.').collect();
        let number: u64 = match digits.parse() {
            Ok(n) if n > 0 && digits.chars().all(|c| c.is_ascii_digit()) => n,
            _ => return Err("El número tiene que ser solo dígitos".to_string()),
        };
        let year: u32 = match year.trim().parse() {
            Ok(y) if (1900..=2100).contains(&y) => y,
            _ => return Err("El año tiene que tener cuatro dígitos".to_string()),
        };
        Ok(CaseKey { jurisdiction, number, year })
    }

    /// As people write it: "CIV 1234/2020".
    pub fn display(&self) -> String {
        format!("{} {}/{}", self.jurisdiction, self.number, self.year)
    }

    /// Its folder: "CIV 1234-2020".
    pub fn folder_name(&self) -> String {
        format!("{} {}-{}", self.jurisdiction, self.number, self.year)
    }

    /// The start of the name of each of its documents: "CIV1234-2020".
    pub fn file_prefix(&self) -> String {
        format!("{}{}-{}", self.jurisdiction, self.number, self.year)
    }
}

/// The references to records written in a text ("CIV 012345/2019"): two to
/// four capital letters, the number and, after a slash, the year.
pub fn find_case_keys(text: &str) -> Vec<CaseKey> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let start_ok = i == 0 || !chars[i - 1].is_alphanumeric();
        if !(start_ok && chars[i].is_ascii_uppercase()) {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len() && chars[j].is_ascii_uppercase() {
            j += 1;
        }
        let letters: String = chars[i..j].iter().collect();
        let mut k = j;
        while k < chars.len() && chars[k] == ' ' {
            k += 1;
        }
        let number_start = k;
        while k < chars.len() && chars[k].is_ascii_digit() {
            k += 1;
        }
        let number: String = chars[number_start..k].iter().collect();
        if (2..=4).contains(&letters.len()) && !number.is_empty() && number.len() <= 8 && k < chars.len() && chars[k] == '/' {
            let year_start = k + 1;
            let mut m = year_start;
            while m < chars.len() && chars[m].is_ascii_digit() {
                m += 1;
            }
            let year: String = chars[year_start..m].iter().collect();
            if year.len() == 4 {
                if let Ok(key) = CaseKey::new(&letters, &number, &year) {
                    if !found.contains(&key) {
                        found.push(key);
                    }
                }
                i = m;
                continue;
            }
        }
        i = j.max(i + 1);
    }
    found
}

/// Which screen of the portal the view shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Screen {
    /// It asks to sign in: only a person does that.
    Login,
    /// It shows its anti-robot challenge: only a person solves it.
    Challenge,
    /// The form to look up a record; `challenge` when it asks for the
    /// challenge before looking up.
    Form { challenge: bool },
    /// A record, with its table of entries.
    Record,
    /// Something the connector does not know.
    Other,
}

impl Screen {
    /// Whether only a person can go on from it.
    pub fn needs_person(&self) -> bool {
        matches!(self, Screen::Login | Screen::Challenge | Screen::Form { challenge: true })
    }
}

/// How to look up a record in the form: what to put in each field (by the
/// mark of the field) and what to press.
#[derive(Debug, Clone, PartialEq)]
pub struct FormPlan {
    pub fills: Vec<(String, String)>,
    pub submit: Option<String>,
    pub challenge: bool,
}

/// What the record page says of the record.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordData {
    /// The record it shows, when it says it.
    #[serde(skip)]
    pub shown: Option<CaseKey>,
    #[serde(rename = "caratula", default)]
    pub title: Option<String>,
    #[serde(rename = "jurisdiccion", default)]
    pub jurisdiction: Option<String>,
    #[serde(rename = "dependencia", default)]
    pub office: Option<String>,
    #[serde(rename = "situacion", default)]
    pub situation: Option<String>,
}

/// One entry of the record's table.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// "dd/mm/yyyy" ("05/03/2024"; the portal may write "5/3/2024").
    pub date: String,
    pub kind: String,
    pub detail: String,
    /// As the portal writes it ("146 / 148").
    pub folios: String,
    pub office: Option<String>,
    /// Its document, when it has one: the mark of its link and where the
    /// link points.
    pub download: Option<DownloadLink>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadLink {
    pub mark: String,
    pub href: Option<String>,
}

/// A record related to this one (accumulated, incident…).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Linked {
    #[serde(rename = "expediente")]
    pub key: CaseKey,
    #[serde(rename = "referencia")]
    pub reference: String,
    #[serde(rename = "detalle", default)]
    pub detail: Option<String>,
}

/// The rows of a page of the table of entries that could not be read as
/// an entry (no date found in them): told, never left out in silence.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Unread {
    pub rows: usize,
    /// Of them, those with a document.
    pub with_document: usize,
}

/// What the app reads of one page of a record.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordPage {
    pub data: RecordData,
    pub rows: Vec<Row>,
    pub unread: Unread,
    /// The mark of its "next page" link, when there is one to press.
    pub next: Option<String>,
    /// The number of the page it shows, as the pager says it.
    pub page_number: Option<u32>,
    /// What tells this page from the next one.
    pub signature: String,
    pub linked: Vec<Linked>,
}

/// What a connector knows of its portal.
pub trait Connector: Send + Sync {
    /// Its id ("pjn").
    fn id(&self) -> &'static str;
    /// Its folder inside `Expedientes` ("PJN").
    fn folder(&self) -> &'static str;
    /// Its name for a person.
    fn name(&self) -> &'static str;
    /// The page where a record is looked up.
    fn start_url(&self) -> &'static str;
    /// Whether the view may go to this address (the portal and its own
    /// challenge and sign-in pages, nothing else).
    fn allows(&self, url: &tauri::Url) -> bool;
    /// Which screen this page is.
    fn screen(&self, page: &Page) -> Screen;
    /// How to look up the record in the form of this page.
    fn form(&self, page: &Page, key: &CaseKey) -> Result<FormPlan, String>;
    /// What the page of a record says.
    fn record(&self, page: &Page) -> RecordPage;
    /// A message the portal shows (a record that does not exist…).
    fn message(&self, page: &Page) -> Option<String>;
    /// Whether the portal is still working on a query: it shows its own
    /// sign of a query in course («Consulta en proceso»).
    fn busy(&self, page: &Page) -> bool;
    /// How many entries a full page of the record has, when it is fixed.
    fn rows_per_page(&self) -> Option<usize>;
    /// A text of an entry kept by an earlier version of the connector, as
    /// this one reads it now, so the same entry keeps its key.
    fn tidy(&self, text: &str) -> String {
        text.to_string()
    }
    /// Its jurisdictions, by code and name, for the form of the app.
    fn jurisdictions(&self) -> Vec<(&'static str, &'static str)>;
}

/// What the view does with an address it is about to go to.
#[derive(Debug, Clone, PartialEq)]
pub enum Navigation {
    /// It goes.
    Allow,
    /// An address of the portal without encryption (`http`), as the PJN
    /// redirects to after «Consultar»: the view does not go there, and opens
    /// the same address with `https` instead. Nothing is ever loaded
    /// without encryption.
    Upgrade(tauri::Url),
    /// It does not go.
    Refuse,
}

/// Decides where the view may go: the portal's own addresses; the same
/// ones without encryption are taken to `https` (only with the default
/// port, and only when the `https` one is the portal's); anything else is
/// refused.
pub fn navigation(connector: &dyn Connector, url: &tauri::Url) -> Navigation {
    if connector.allows(url) {
        return Navigation::Allow;
    }
    if url.scheme() == "http" && url.port().is_none() && url.username().is_empty() && url.password().is_none() {
        let mut secure = url.clone();
        if secure.set_scheme("https").is_ok() && connector.allows(&secure) {
            return Navigation::Upgrade(secure);
        }
    }
    Navigation::Refuse
}

/// The addresses taken to `https` lately, so a portal that keeps sending
/// the view back to `http` (a loop) is not followed forever.
#[derive(Debug, Default)]
pub struct Upgrades {
    recent: std::collections::VecDeque<(String, std::time::Instant)>,
}

impl Upgrades {
    /// How many times the same address may be taken to `https` in
    /// `WINDOW`.
    pub const MAX_SAME: usize = 3;
    pub const WINDOW: std::time::Duration = std::time::Duration::from_secs(30);

    /// Whether this address may be taken to `https` now; remembers it.
    pub fn admit(&mut self, url: &tauri::Url, now: std::time::Instant) -> bool {
        self.recent.retain(|(_, at)| now.saturating_duration_since(*at) < Self::WINDOW);
        let same = self.recent.iter().filter(|(seen, _)| seen == url.as_str()).count();
        if same >= Self::MAX_SAME {
            return false;
        }
        self.recent.push_back((url.as_str().to_string(), now));
        true
    }
}

/// The connector of a portal, by its id.
pub fn connector(id: &str) -> Result<&'static dyn Connector, String> {
    match id {
        "pjn" => Ok(&pjn::Pjn),
        other => Err(format!("No hay un conector para el portal «{other}»")),
    }
}
