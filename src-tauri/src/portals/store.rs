//! Where what comes from a portal is kept, and what is known of it.
//!
//! `Expedientes\<portal>\<jurisdicción> <número>-<año>\` holds, for each
//! record:
//!
//! - its documents, named `<JUR><número>-<año>_fs<fojas>_<fecha>_<TIPO>_<detalle>.pdf`;
//! - next to each one, its data (`<same name>.micelya.json`: date, folios,
//!   kind, detail, office, the address it came from, when it was
//!   downloaded);
//! - `expediente.micelya.json`: the record (title, jurisdiction, office,
//!   situation, its related records) and each entry with its state. It is
//!   what makes the walk incremental and resumable: what was seen and
//!   what was downloaded.
//!
//! The data files end in `.micelya.json` so the base does not take them
//! for documents (`docbase/scan.rs`). A download lands first in
//! `Expedientes\<portal>\.descargas` (left out of the inventory) and only a
//! checked PDF is moved into the record's folder.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::params;
use serde::{Deserialize, Serialize};

use super::{CaseKey, Linked, RecordData, Row};
use crate::docbase::origins::path_inside;
use crate::docbase::{sql_err, DocBase, Visible};

/// The file of each record.
pub const RECORD_FILE: &str = "expediente.micelya.json";
/// The ending of the data files.
pub const DATA_SUFFIX: &str = ".micelya.json";
/// The folder where downloads land before they are checked.
pub const STAGING: &str = ".descargas";

/// What an entry is, for the walk.
pub mod state {
    /// It has no document (a "MOVIMIENTO").
    pub const NO_DOCUMENT: &str = "sin-documento";
    /// Its document is waiting to be downloaded.
    pub const PENDING: &str = "pendiente";
    /// Downloaded by the app, in the record's folder.
    pub const DOWNLOADED: &str = "bajado";
    /// The base already had it: not downloaded again.
    pub const ALREADY: &str = "ya-estaba";
    /// Its download failed: tried again on the next update.
    pub const FAILED: &str = "fallo";
}

/// One entry of the record, with its state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    #[serde(rename = "fecha")]
    pub date: String,
    #[serde(rename = "tipo")]
    pub kind: String,
    #[serde(rename = "detalle")]
    pub detail: String,
    #[serde(rename = "fojas")]
    pub folios: String,
    #[serde(rename = "oficina", default, skip_serializing_if = "Option::is_none")]
    pub office: Option<String>,
    #[serde(rename = "con_documento")]
    pub has_document: bool,
    #[serde(rename = "estado")]
    pub state: String,
    /// Its file: the name in the record's folder, or where the base had it.
    #[serde(rename = "archivo", default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(rename = "tamano", default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(rename = "fecha_descarga", default, skip_serializing_if = "Option::is_none")]
    pub downloaded_at: Option<String>,
    #[serde(rename = "direccion_origen", default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(rename = "error", default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Entry {
    pub fn of(row: &Row) -> Entry {
        Entry {
            date: row.date.clone(),
            kind: row.kind.clone(),
            detail: row.detail.clone(),
            folios: row.folios.clone(),
            office: row.office.clone(),
            has_document: row.download.is_some(),
            state: if row.download.is_some() { state::PENDING } else { state::NO_DOCUMENT }.to_string(),
            ..Entry::default()
        }
    }

    /// What identifies it in the record: folios, date, kind and detail.
    pub fn key(&self) -> EntryKey {
        EntryKey::new(&self.folios, &self.date, &self.kind, &self.detail)
    }
}

/// An entry as it is compared: its folios, its date (yyyymmdd), its kind
/// and its detail, normalized.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EntryKey {
    pub folios: Vec<u64>,
    pub date: String,
    pub kind: String,
    pub detail: String,
}

impl EntryKey {
    pub fn new(folios: &str, date: &str, kind: &str, detail: &str) -> EntryKey {
        EntryKey { folios: numbers(folios), date: canonical_date(date).unwrap_or_default(), kind: compact(kind), detail: slug(detail, 60).to_lowercase() }
    }

    pub fn of(row: &Row) -> EntryKey {
        EntryKey::new(&row.folios, &row.date, &row.kind, &row.detail)
    }

    /// Folios, date and kind: what the base is searched by.
    fn same_document(&self, other: &EntryKey) -> bool {
        let folios = self.folios == other.folios
            || (!self.folios.is_empty() && self.folios.first() == other.folios.first());
        folios && self.date == other.date && self.kind == other.kind
    }
}

/// The record's file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RecordFile {
    pub portal: String,
    #[serde(rename = "expediente")]
    pub key: CaseKey,
    #[serde(rename = "referencia")]
    pub reference: String,
    #[serde(flatten)]
    pub data: RecordData,
    #[serde(rename = "actuaciones", default)]
    pub entries: Vec<Entry>,
    #[serde(rename = "vinculados", default)]
    pub linked: Vec<Linked>,
    /// The matter of the base made for it.
    #[serde(rename = "caso", default, skip_serializing_if = "Option::is_none")]
    pub matter_id: Option<String>,
    #[serde(rename = "seguido_desde")]
    pub followed_at: String,
    #[serde(rename = "ultima_actualizacion", default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(rename = "ultimo_resultado", default, skip_serializing_if = "Option::is_none")]
    pub last_result: Option<String>,
    /// The last walk went through the table up to its last page, or up to
    /// a page it already knew whole: the file holds every entry of the
    /// record. Only then may the next update stop at the first page it
    /// already knows (the portal only adds entries, newest first). A walk
    /// cut or stopped leaves it off, and the next one goes through it all.
    #[serde(rename = "recorrido_completo", default)]
    pub complete: bool,
}

impl Default for CaseKey {
    fn default() -> Self {
        CaseKey { jurisdiction: String::new(), number: 0, year: 0 }
    }
}

impl RecordFile {
    pub fn new(portal: &str, key: &CaseKey) -> RecordFile {
        RecordFile {
            portal: portal.to_string(),
            key: key.clone(),
            reference: key.display(),
            followed_at: now_text(),
            ..RecordFile::default()
        }
    }

    pub fn entry_index(&self, key: &EntryKey) -> Option<usize> {
        self.entries.iter().position(|e| &e.key() == key)
    }

    /// How many entries share folios, date and kind with this one: more
    /// than one, and the detail tells them apart.
    pub fn twins(&self, key: &EntryKey) -> usize {
        self.entries.iter().filter(|e| e.key().same_document(key)).count()
    }

    pub fn count(&self, state: &str) -> usize {
        self.entries.iter().filter(|e| e.state == state).count()
    }
}

impl Entry {
    /// Nothing is left to do with it: it has no document, or its document
    /// is in the folder or in the base.
    pub fn settled(&self) -> bool {
        match self.state.as_str() {
            state::NO_DOCUMENT => !self.has_document,
            state::DOWNLOADED | state::ALREADY => self.file.is_some(),
            _ => false,
        }
    }

    /// Its document is still to be brought (waiting, or it failed).
    pub fn unfinished(&self) -> bool {
        self.has_document && matches!(self.state.as_str(), state::PENDING | state::FAILED)
    }
}

/// The folder of a record inside the portal's folder.
pub fn record_folder(portal_folder: &Path, key: &CaseKey) -> PathBuf {
    portal_folder.join(key.folder_name())
}

pub fn load(folder: &Path) -> Result<Option<RecordFile>, String> {
    let path = folder.join(RECORD_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("No se pudo leer {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("No se pudo leer {}: {e}", path.display())),
    }
}

/// Writes the record's file whole, through a temporary one: a cut never
/// leaves half a file.
pub fn save(folder: &Path, record: &RecordFile) -> Result<(), String> {
    fs::create_dir_all(folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
    write_json(&folder.join(RECORD_FILE), record)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text).map_err(|e| format!("No se pudo escribir {}: {e}", path.display()))?;
    fs::rename(&temporary, path).map_err(|e| format!("No se pudo escribir {}: {e}", path.display()))
}

/// The data kept next to a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocumentData {
    pub portal: String,
    #[serde(rename = "expediente")]
    pub reference: String,
    #[serde(rename = "fecha")]
    pub date: String,
    #[serde(rename = "fojas")]
    pub folios: String,
    #[serde(rename = "tipo")]
    pub kind: String,
    #[serde(rename = "detalle")]
    pub detail: String,
    #[serde(rename = "oficina", default)]
    pub office: Option<String>,
    #[serde(rename = "direccion_origen", default)]
    pub source: Option<String>,
    #[serde(rename = "fecha_descarga")]
    pub downloaded_at: String,
    #[serde(rename = "archivo")]
    pub file: String,
    #[serde(rename = "tamano")]
    pub size: u64,
}

/// The data file of a document: its name with `.micelya.json` in place of
/// its extension.
pub fn data_path(document: &Path) -> PathBuf {
    let stem = document.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    document.with_file_name(format!("{stem}{DATA_SUFFIX}"))
}

pub fn write_document_data(document: &Path, data: &DocumentData) -> Result<(), String> {
    write_json(&data_path(document), data)
}

/// The name of an entry's document:
/// `<JUR><número>-<año>_fs<fojas>_<fecha>_<TIPO>_<detalle>.pdf`, with the
/// folios as their numbers ("146 / 148" is `fs146-148`), the date as
/// yyyy-mm-dd, and kind and detail without spaces nor signs.
pub fn document_name(key: &CaseKey, entry: &Entry) -> String {
    let folios = numbers(&entry.folios).iter().map(u64::to_string).collect::<Vec<_>>().join("-");
    let folios = if folios.is_empty() { "0".to_string() } else { folios };
    let date = canonical_date(&entry.date)
        .map(|d| format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]))
        .unwrap_or_else(|| "sin-fecha".to_string());
    let kind = slug(&entry.kind, 30).to_uppercase();
    let kind = if kind.is_empty() { "SIN-TIPO".to_string() } else { kind };
    let detail = slug(&entry.detail, 60);
    let mut name = format!("{}_fs{folios}_{date}_{kind}", key.file_prefix());
    if !detail.is_empty() {
        name.push('_');
        name.push_str(&detail);
    }
    name.push_str(".pdf");
    name
}

/// A name not taken in the folder nor by another entry: `_2`, `_3`… before
/// the extension when it is.
pub fn free_name(folder: &Path, name: &str, taken: &[String]) -> String {
    let (stem, ext) = name.rsplit_once('.').unwrap_or((name, ""));
    let mut candidate = name.to_string();
    let mut n = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&candidate)) || folder.join(&candidate).exists() {
        candidate = format!("{stem}_{n}.{ext}");
        n += 1;
    }
    candidate
}

/// The numbers in a text, in order.
pub fn numbers(text: &str) -> Vec<u64> {
    text.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).filter_map(|s| s.parse().ok()).collect()
}

/// A date as yyyymmdd, from "dd/mm/yyyy", "yyyy-mm-dd", "yyyymmdd" or
/// "ddmmyyyy".
pub fn canonical_date(text: &str) -> Option<String> {
    let groups: Vec<&str> = text.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).collect();
    let (y, m, d) = match groups.as_slice() {
        [a, b, c] if a.len() == 4 => (*a, *b, *c),
        [a, b, c] if c.len() == 4 => (*c, *b, *a),
        [one] if one.len() == 8 => {
            let starts_with_year = one.starts_with("19") || one.starts_with("20");
            let month_after_year: u32 = one[4..6].parse().ok()?;
            if starts_with_year && (1..=12).contains(&month_after_year) {
                (&one[0..4], &one[4..6], &one[6..8])
            } else {
                (&one[4..8], &one[2..4], &one[0..2])
            }
        }
        _ => return None,
    };
    let (y, m, d): (u32, u32, u32) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || !(1900..=2100).contains(&y) {
        return None;
    }
    Some(format!("{y:04}{m:02}{d:02}"))
}

/// Letters and digits without accents, every other run of signs one `-`,
/// at most `limit` characters.
pub fn slug(text: &str, limit: usize) -> String {
    let plain = ascii(text);
    let mut out = String::new();
    for c in plain.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let mut out: String = out.chars().take(limit).collect();
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn ascii(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'Á' | 'À' | 'Ä' | 'Â' => 'A',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'É' | 'È' | 'Ë' | 'Ê' => 'E',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'Í' | 'Ì' | 'Ï' | 'Î' => 'I',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'Ó' | 'Ò' | 'Ö' | 'Ô' => 'O',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'Ú' | 'Ù' | 'Ü' | 'Û' => 'U',
            'ñ' => 'n',
            'Ñ' => 'N',
            other => other,
        })
        .collect()
}

fn compact(text: &str) -> String {
    ascii(text).chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_uppercase()
}

pub fn now_text() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// What a document name says, when it follows the naming of the records.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDocument {
    pub jurisdiction: String,
    pub number: u64,
    pub year: u32,
    pub entry: EntryKey,
}

/// Reads `<JUR><número>-<año>_fs<fojas>_<fecha>_<TIPO>_<detalle>.<ext>`.
pub fn parse_document_name(name: &str) -> Option<NamedDocument> {
    let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
    let mut parts = stem.split('_');
    let record = parts.next()?;
    let folios = parts.next()?.strip_prefix("fs")?;
    let date = parts.next()?;
    let kind = parts.next()?;
    let detail: Vec<&str> = parts.collect();
    let letters: String = record.chars().take_while(char::is_ascii_alphabetic).collect();
    let (number, year) = record[letters.len()..].split_once('-')?;
    Some(NamedDocument {
        jurisdiction: letters.to_uppercase(),
        number: number.trim().parse().ok()?,
        year: year.trim().parse().ok()?,
        entry: EntryKey { folios: numbers(folios), date: canonical_date(date)?, kind: compact(kind), detail: detail.join("_").to_lowercase() },
    })
}

/// A document the base already has.
#[derive(Debug, Clone, PartialEq)]
pub struct Known {
    pub path: PathBuf,
    pub size: u64,
}

/// Where the walk asks whether a document is already in the base.
pub trait KnownFiles {
    /// The document of this entry of this record that the base already
    /// has, by its name. `by_detail`: other entries share its folios, date
    /// and kind, so the detail must match too.
    fn find(&self, key: &CaseKey, entry: &EntryKey, by_detail: bool) -> Option<Known>;
}

/// The base knows none (a first test).
pub struct NoneKnown;

impl KnownFiles for NoneKnown {
    fn find(&self, _: &CaseKey, _: &EntryKey, _: bool) -> Option<Known> {
        None
    }
}

impl KnownFiles for DocBase {
    fn find(&self, key: &CaseKey, entry: &EntryKey, by_detail: bool) -> Option<Known> {
        let conn = self.conn();
        let pattern = format!("{}%\\_fs%", key.jurisdiction);
        let mut statement = conn
            .prepare(
                "SELECT l.name, l.size, l.rel_path, o.location FROM location l JOIN origin o ON o.id = l.origin_id
                 WHERE l.kind = 'file' AND l.name LIKE ?1 ESCAPE '\\' AND l.size > 0",
            )
            .ok()?;
        let rows = statement
            .query_map(params![pattern], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))
            })
            .ok()?;
        for (name, size, rel_path, location) in rows.flatten() {
            if name.to_lowercase().ends_with(DATA_SUFFIX) {
                continue;
            }
            let Some(named) = parse_document_name(&name) else { continue };
            let same_record = named.jurisdiction == key.jurisdiction && named.number == key.number && named.year == key.year;
            if !same_record || !named.entry.same_document(entry) {
                continue;
            }
            if by_detail && !(named.entry.detail.starts_with(&entry.detail) || entry.detail.starts_with(&named.entry.detail)) {
                continue;
            }
            let path = Path::new(&location).join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
            return Some(Known { path, size: size.max(0) as u64 });
        }
        None
    }
}

/// A record followed, as the page lists it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Followed {
    pub folder: String,
    pub reference: String,
    pub jurisdiction: String,
    pub number: u64,
    pub year: u32,
    pub title: Option<String>,
    pub office: Option<String>,
    pub situation: Option<String>,
    pub followed_at: String,
    pub updated_at: Option<String>,
    pub last_result: Option<String>,
    pub entries: usize,
    pub with_document: usize,
    pub downloaded: usize,
    pub already: usize,
    pub failed: usize,
    pub pending: usize,
    pub matter_id: Option<String>,
    pub linked: Vec<FollowedLink>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedLink {
    pub reference: String,
    pub jurisdiction: String,
    pub number: u64,
    pub year: u32,
    pub detail: Option<String>,
    /// Already followed.
    pub followed: bool,
}

/// The records followed in a portal's folder, the latest updated first.
pub fn followed(portal_folder: &Path) -> Vec<(PathBuf, RecordFile)> {
    let Ok(read) = fs::read_dir(portal_folder) else { return Vec::new() };
    let mut out: Vec<(PathBuf, RecordFile)> = read
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| load(&e.path()).ok().flatten().map(|record| (e.path(), record)))
        .collect();
    out.sort_by(|a, b| b.1.updated_at.cmp(&a.1.updated_at).then_with(|| a.1.reference.cmp(&b.1.reference)));
    out
}

pub fn followed_view(list: &[(PathBuf, RecordFile)]) -> Vec<Followed> {
    list.iter()
        .map(|(folder, record)| Followed {
            folder: folder.to_string_lossy().to_string(),
            reference: record.reference.clone(),
            jurisdiction: record.key.jurisdiction.clone(),
            number: record.key.number,
            year: record.key.year,
            title: record.data.title.clone(),
            office: record.data.office.clone(),
            situation: record.data.situation.clone(),
            followed_at: record.followed_at.clone(),
            updated_at: record.updated_at.clone(),
            last_result: record.last_result.clone(),
            entries: record.entries.len(),
            with_document: record.entries.iter().filter(|e| e.has_document).count(),
            downloaded: record.count(state::DOWNLOADED),
            already: record.count(state::ALREADY),
            failed: record.count(state::FAILED),
            pending: record.count(state::PENDING),
            matter_id: record.matter_id.clone(),
            linked: record
                .linked
                .iter()
                .map(|l| FollowedLink {
                    reference: l.reference.clone(),
                    jurisdiction: l.key.jurisdiction.clone(),
                    number: l.key.number,
                    year: l.key.year,
                    detail: l.detail.clone(),
                    followed: list.iter().any(|(_, r)| r.key == l.key),
                })
                .collect(),
        })
        .collect()
}

/// The origin of the base that holds the records' folder, `folder`, inside
/// the work folder `work`: the work folder's own, made now if needed, or
/// one that holds it already (D-451). Never an origin for the records'
/// folder alone: the base takes no origin inside another, and one made
/// before by the first version of the portals joins the work folder's.
/// Returns its id and the path of the folder inside it.
pub fn ensure_origin(base: &DocBase, work: &Path, folder: &Path) -> Result<(String, String), String> {
    fs::create_dir_all(folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
    crate::work_folder::prepare(work)?;
    let registered = crate::work_folder::register(base, work);
    for origin in base.origins(&Visible::All)? {
        if let Some(rest) = path_inside(&origin.root(), folder) {
            return Ok((origin.id, rest));
        }
    }
    Err(match registered {
        Err(error) => error,
        Ok(_) => format!("La carpeta {} no está dentro de la carpeta de trabajo", folder.display()),
    })
}

/// The matter of the record: made confirmed, by a person's request, with
/// its reference in its name; and every document of its folder the base
/// already inventoried, in it. Returns how many documents went in now.
pub fn ensure_matter(base: &DocBase, record: &mut RecordFile, origin_id: &str, rel_folder: &str) -> Result<usize, String> {
    let exists = match &record.matter_id {
        Some(id) => base
            .conn()
            .query_row("SELECT state FROM knowledge_matter WHERE id = ?1", [id], |row| row.get::<_, String>(0))
            .map(|state| state == "active")
            .unwrap_or(false),
        None => false,
    };
    if !exists {
        let name = match record.data.title.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
            Some(title) => format!("{title} ({})", record.reference),
            None => record.reference.clone(),
        };
        record.matter_id = Some(base.knowledge_create_matter(&name, &[])?);
    }
    let matter = record.matter_id.clone().unwrap_or_default();
    let contents: Vec<i64> = {
        let conn = base.conn();
        let prefix = if rel_folder.is_empty() { String::new() } else { format!("{rel_folder}/") };
        let mut statement = conn
            .prepare(
                "SELECT DISTINCT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash
                 WHERE l.origin_id = ?1 AND substr(l.rel_path, 1, length(?2)) = ?2 AND l.ext = 'pdf'
                   AND f.content_id NOT IN (SELECT content_id FROM knowledge_matter_choice WHERE matter_id = ?3)",
            )
            .map_err(sql_err)?;
        let rows = statement.query_map(params![origin_id, prefix, matter], |row| row.get(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    for content in &contents {
        base.knowledge_choose_matter_document(&matter, &format!("content:{content}"), "in")?;
    }
    Ok(contents.len())
}

/// The path of a record's folder inside its origin, with `/`.
pub fn rel_inside(origin_rel: &str, portal: &str, key: &CaseKey) -> String {
    let own = format!("{portal}/{}", key.folder_name());
    if origin_rel.is_empty() {
        own
    } else {
        format!("{origin_rel}/{own}")
    }
}
