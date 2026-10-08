//! The source catalog: which documents a project holds, apart from where
//! their files are.
//!
//! A *content* is one document, identified by a fingerprint of its text.
//! A *location* is a source file, identified the way the rest of the app
//! identifies sources: its path under `raw/sources`. The same agreement
//! saved as Word and as PDF, or copied into two folders, is one content
//! with several locations, and only needs to be ingested once.
//!
//! Only an exact match of the normalized text makes two files the same
//! content. Similar documents — versions, or forms filled in with
//! different data — have different fingerprints and are never joined
//! here.
//!
//! The catalog also remembers which location a content was ingested
//! from and which wiki pages that produced, so a later location of the
//! same content can be attached to those pages instead of being sent to
//! the model again.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::watch::{self, Location};
use super::SOURCES_PREFIX;

const CATALOG_FILE: &str = ".llm-wiki/source-catalog.json";
const CATALOG_VERSION: u32 = 1;
/// Below this many words a text says too little to call two files the
/// same document (an unread scan, a cover page, a photo).
const MIN_FINGERPRINT_WORDS: usize = 30;
/// Start of the marker `recognition.rs` writes before machine-read text.
const RECOGNIZED_MARKER_START: &str = "<!-- texto-reconocido";

/// One read-modify-write of the catalog file at a time.
static CATALOG_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ingested {
    /// Location the content was ingested from.
    pub identity: String,
    /// Wiki pages that ingest wrote, relative to the project.
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Content {
    /// Fingerprint of the normalized text.
    pub id: String,
    /// Part of the text was machine-read from pixels, so the match is
    /// less certain than one between extracted texts.
    pub recognized: bool,
    pub word_count: usize,
    /// Source identities (paths under `raw/sources`) holding this content.
    pub locations: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ingested: Option<Ingested>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogFile {
    version: u32,
    contents: Vec<Content>,
}

/// Fingerprint of a source text, or `None` when the text is too short to
/// identify a document.
pub fn fingerprint(text: &str) -> Option<(String, usize)> {
    let words = normalized_words(text);
    if words.len() < MIN_FINGERPRINT_WORDS {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(words.join(" ").as_bytes());
    let id = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Some((id, words.len()))
}

pub fn is_recognized(text: &str) -> bool {
    text.contains(RECOGNIZED_MARKER_START)
}

/// Words of `text` as compared between files: lowercase, without
/// accents, letters and digits only. Formatting that depends on the file
/// format — punctuation, line breaks, page headings, recognition
/// markers — does not take part.
fn normalized_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(RECOGNIZED_MARKER_START) || is_page_heading(trimmed) {
            continue;
        }
        let mut word = String::new();
        for c in trimmed.chars().flat_map(char::to_lowercase) {
            match unaccented(c) {
                Some(c) if c.is_ascii_alphanumeric() => word.push(c),
                _ => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                }
            }
        }
        if !word.is_empty() {
            words.push(word);
        }
    }
    words
}

/// The `## Page N` headings the PDF extractor writes.
fn is_page_heading(line: &str) -> bool {
    line.strip_prefix("## Page ")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

/// `c` without its accent, for the accented letters of Western European
/// languages. Characters outside ASCII that are not listed are dropped
/// by the caller.
fn unaccented(c: char) -> Option<char> {
    Some(match c {
        'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        c if c.is_ascii() => c,
        _ => return None,
    })
}

/// Record that the source `identity` holds `text` and return the content
/// it belongs to, with every location known for it. `None` when the text
/// is too short to be fingerprinted; such a source is not catalogued.
///
/// A source appears under one content only: registering it again with
/// different text moves it. Locations whose file no longer exists are
/// dropped; locations in an unreachable mounted origin are kept.
pub fn register(project_root: &Path, identity: &str, text: &str) -> Result<Option<Content>, String> {
    let _guard = lock();
    let mut catalog = read(project_root)?;
    let Some((id, word_count)) = fingerprint(text) else {
        forget_location(&mut catalog, identity, None);
        write(project_root, &catalog)?;
        return Ok(None);
    };
    forget_location(&mut catalog, identity, Some(&id));

    let recognized = is_recognized(text);
    let position = match catalog.contents.iter().position(|content| content.id == id) {
        Some(position) => position,
        None => {
            catalog.contents.push(Content {
                id,
                recognized,
                word_count,
                locations: Vec::new(),
                ingested: None,
            });
            catalog.contents.len() - 1
        }
    };
    let content = &mut catalog.contents[position];
    content
        .locations
        .retain(|location| location_may_exist(project_root, location));
    // The pages of an ingest list the location they came from. If that
    // location is gone they now belong to the oldest one left, or to
    // nothing when no other location ever shared them.
    let orphaned = content.ingested.as_ref().is_some_and(|ingested| {
        !same_identity(&ingested.identity, identity)
            && !content
                .locations
                .iter()
                .any(|location| same_identity(location, &ingested.identity))
    });
    if orphaned {
        content.ingested = match (content.locations.first(), content.ingested.take()) {
            (Some(heir), Some(ingested)) => Some(Ingested {
                identity: heir.clone(),
                files: ingested.files,
            }),
            _ => None,
        };
    }
    content.locations.push(identity.to_string());
    // Every text that produced this fingerprint must be extracted text
    // for the match to count as certain.
    content.recognized = content.recognized || recognized;
    let result = content.clone();
    write(project_root, &catalog)?;
    Ok(Some(result))
}

/// Record the wiki pages produced by ingesting `identity`.
pub fn record_ingest(project_root: &Path, identity: &str, files: Vec<String>) -> Result<(), String> {
    let _guard = lock();
    let mut catalog = read(project_root)?;
    let Some(content) = catalog
        .contents
        .iter_mut()
        .find(|content| content.locations.iter().any(|l| same_identity(l, identity)))
    else {
        return Ok(());
    };
    content.ingested = Some(Ingested {
        identity: identity.to_string(),
        files,
    });
    write(project_root, &catalog)
}

/// Contents that have more than one location.
pub fn duplicates(project_root: &Path) -> Result<Vec<Content>, String> {
    let _guard = lock();
    Ok(read(project_root)?
        .contents
        .into_iter()
        .filter(|content| content.locations.len() > 1)
        .collect())
}

/// Remove `identity` from every content. Contents left without a
/// location are dropped, except `keep`, which is about to get it back.
fn forget_location(catalog: &mut CatalogFile, identity: &str, keep: Option<&str>) {
    for content in &mut catalog.contents {
        content
            .locations
            .retain(|location| !same_identity(location, identity));
    }
    catalog
        .contents
        .retain(|content| !content.locations.is_empty() || keep == Some(content.id.as_str()));
}

/// False only when the source is known to be gone. An unreachable
/// origin says nothing about its files.
fn location_may_exist(project_root: &Path, identity: &str) -> bool {
    match watch::locate(project_root, &format!("{SOURCES_PREFIX}/{identity}")) {
        Location::At(path) => path.exists(),
        Location::Unreachable => true,
    }
}

/// Source identities are compared without case, as the rest of the app
/// does when it matches a page's `sources` entries.
fn same_identity(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    CATALOG_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn catalog_path(project_root: &Path) -> PathBuf {
    project_root.join(CATALOG_FILE)
}

fn read(project_root: &Path) -> Result<CatalogFile, String> {
    let path = catalog_path(project_root);
    if !path.exists() {
        return Ok(CatalogFile {
            version: CATALOG_VERSION,
            contents: Vec::new(),
        });
    }
    let raw = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read '{}': {e}", path.display()))?;
    serde_json::from_str(&raw).map_err(|e| format!("Invalid source catalog '{}': {e}", path.display()))
}

fn write(project_root: &Path, catalog: &CatalogFile) -> Result<(), String> {
    let path = catalog_path(project_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create '{}': {e}", parent.display()))?;
    }
    // Stable order keeps the file readable and its diffs small.
    let mut ordered: BTreeMap<&str, &Content> = BTreeMap::new();
    for content in &catalog.contents {
        ordered.insert(&content.id, content);
    }
    let file = serde_json::json!({
        "version": CATALOG_VERSION,
        "contents": ordered.values().collect::<Vec<_>>(),
    });
    let json = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, json).map_err(|e| format!("Failed to write '{}': {e}", tmp.display()))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Failed to replace '{}': {e}", path.display()))
}

#[tauri::command]
pub async fn catalog_register_source(
    project_path: String,
    identity: String,
    text: String,
) -> Result<Option<Content>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::panic_guard::run_guarded("catalog_register_source", || {
            register(Path::new(&project_path), &identity, &text)
        })
    })
    .await
    .map_err(|e| format!("catalog_register_source blocking task join error: {e}"))?
}

#[tauri::command]
pub async fn catalog_record_ingest(
    project_path: String,
    identity: String,
    files: Vec<String>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::panic_guard::run_guarded("catalog_record_ingest", || {
            record_ingest(Path::new(&project_path), &identity, files)
        })
    })
    .await
    .map_err(|e| format!("catalog_record_ingest blocking task join error: {e}"))?
}

#[tauri::command]
pub async fn catalog_list_duplicates(project_path: String) -> Result<Vec<Content>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::panic_guard::run_guarded("catalog_list_duplicates", || {
            duplicates(Path::new(&project_path))
        })
    })
    .await
    .map_err(|e| format!("catalog_list_duplicates blocking task join error: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGREEMENT: &str = "Acuerdo de colaboración entre las partes. La primera parte se obliga a \
        prestar los servicios descriptos en el anexo, y la segunda parte se obliga a pagar el precio \
        convenido dentro de los treinta días de recibida cada factura, en la cuenta que se indique.";

    struct Project(PathBuf);

    impl Project {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("micelya-catalog-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(root.join("raw/sources")).unwrap();
            Self(root)
        }

        fn source(&self, identity: &str) {
            let path = self.0.join("raw/sources").join(identity);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "x").unwrap();
        }

        fn register(&self, identity: &str, text: &str) -> Option<Content> {
            self.source(identity);
            register(&self.0, identity, text).unwrap()
        }
    }

    impl Drop for Project {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn the_same_text_in_another_format_has_the_same_fingerprint() {
        let as_pdf = format!("## Page 1\n{}\n", AGREEMENT.replace(". ", ".\n"));
        let as_docx = AGREEMENT.to_uppercase().replace("COLABORACIÓN", "colaboracion");
        assert_eq!(fingerprint(AGREEMENT), fingerprint(&as_pdf));
        assert_eq!(fingerprint(AGREEMENT), fingerprint(&as_docx));
    }

    #[test]
    fn a_different_figure_is_a_different_document() {
        let other = AGREEMENT.replace("treinta", "sesenta");
        assert_ne!(fingerprint(AGREEMENT), fingerprint(&other));
        let amount_a = format!("{AGREEMENT} Importe total: USD 4.114,80.");
        let amount_b = format!("{AGREEMENT} Importe total: USD 4.114,90.");
        assert_ne!(fingerprint(&amount_a), fingerprint(&amount_b));
    }

    #[test]
    fn short_texts_are_not_fingerprinted() {
        assert_eq!(fingerprint("Factura A 00121-00030080"), None);
        let project = Project::new();
        assert_eq!(project.register("portada.pdf", "Solo una portada"), None);
    }

    #[test]
    fn recognition_markers_do_not_change_the_fingerprint_but_are_remembered() {
        let recognized = format!("<!-- texto-reconocido motor=\"codex-cli/gpt\" -->\n{AGREEMENT}");
        assert_eq!(fingerprint(AGREEMENT), fingerprint(&recognized));

        let project = Project::new();
        assert!(!project.register("a.docx", AGREEMENT).unwrap().recognized);
        assert!(project.register("a-escaneado.pdf", &recognized).unwrap().recognized);
    }

    #[test]
    fn a_second_file_with_the_same_text_is_another_location_of_the_content() {
        let project = Project::new();
        let first = project.register("Acuerdos/acuerdo.docx", AGREEMENT).unwrap();
        assert_eq!(first.locations, vec!["Acuerdos/acuerdo.docx"]);

        let second = project.register("Acuerdos/acuerdo.pdf", AGREEMENT).unwrap();
        assert_eq!(second.id, first.id);
        assert_eq!(second.locations, vec!["Acuerdos/acuerdo.docx", "Acuerdos/acuerdo.pdf"]);
        assert_eq!(duplicates(&project.0).unwrap().len(), 1);
    }

    #[test]
    fn registering_a_source_twice_does_not_duplicate_its_location() {
        let project = Project::new();
        project.register("a.docx", AGREEMENT);
        let again = project.register("a.docx", AGREEMENT).unwrap();
        assert_eq!(again.locations, vec!["a.docx"]);
    }

    #[test]
    fn a_source_whose_text_changed_moves_to_its_new_content() {
        let project = Project::new();
        project.register("a.docx", AGREEMENT);
        project.register("b.docx", AGREEMENT);

        let edited = project
            .register("b.docx", &AGREEMENT.replace("treinta", "sesenta"))
            .unwrap();

        assert_eq!(edited.locations, vec!["b.docx"]);
        assert!(duplicates(&project.0).unwrap().is_empty());
    }

    #[test]
    fn locations_whose_file_is_gone_are_dropped() {
        let project = Project::new();
        project.register("a.docx", AGREEMENT);
        fs::remove_file(project.0.join("raw/sources/a.docx")).unwrap();

        let content = project.register("b.docx", AGREEMENT).unwrap();

        assert_eq!(content.locations, vec!["b.docx"]);
    }

    #[test]
    fn the_pages_of_an_ingest_are_remembered_for_later_locations() {
        let project = Project::new();
        project.register("a.docx", AGREEMENT);
        record_ingest(&project.0, "a.docx", vec!["wiki/sources/a.md".to_string()]).unwrap();

        let later = project.register("copia/a.pdf", AGREEMENT).unwrap();

        assert_eq!(
            later.ingested,
            Some(Ingested {
                identity: "a.docx".to_string(),
                files: vec!["wiki/sources/a.md".to_string()],
            })
        );
    }

    #[test]
    fn pages_stay_with_the_content_when_the_ingested_location_is_deleted() {
        let project = Project::new();
        project.register("a.docx", AGREEMENT);
        record_ingest(&project.0, "a.docx", vec!["wiki/sources/a.md".to_string()]).unwrap();
        project.register("b.pdf", AGREEMENT);
        fs::remove_file(project.0.join("raw/sources/a.docx")).unwrap();

        let content = project.register("c.pdf", AGREEMENT).unwrap();

        assert_eq!(content.locations, vec!["b.pdf", "c.pdf"]);
        assert_eq!(
            content.ingested,
            Some(Ingested {
                identity: "b.pdf".to_string(),
                files: vec!["wiki/sources/a.md".to_string()],
            })
        );
    }

    #[test]
    fn an_ingest_is_forgotten_when_its_only_location_is_deleted() {
        let project = Project::new();
        project.register("a.docx", AGREEMENT);
        record_ingest(&project.0, "a.docx", vec!["wiki/sources/a.md".to_string()]).unwrap();
        fs::remove_file(project.0.join("raw/sources/a.docx")).unwrap();

        let content = project.register("b.pdf", AGREEMENT).unwrap();

        assert_eq!(content.locations, vec!["b.pdf"]);
        assert_eq!(content.ingested, None);
    }

    #[test]
    fn re_registering_the_ingested_source_keeps_its_pages() {
        let project = Project::new();
        project.register("a.docx", AGREEMENT);
        record_ingest(&project.0, "a.docx", vec!["wiki/sources/a.md".to_string()]).unwrap();

        let again = project.register("a.docx", AGREEMENT).unwrap();

        assert_eq!(again.ingested.unwrap().identity, "a.docx");
    }
}
