//! The one import of the old wiki's files into the base (D-403): what a
//! person wrote there and nothing else is, the notes of each page and the
//! saved queries. The pages themselves are not read: the projection writes
//! them in the base from the record (D-402).
//!
//! - The notes of a page (after its notes mark, or under its heading of
//!   notes) go to the object of the page: the entity or matter of its
//!   path in `knowledge_page` or of its header, or, for a page of a
//!   document, the document's family. Notes whose page matches no object
//!   are kept apart, with the page's title (`knowledge_note` without an
//!   object): never lost.
//! - Each file of `wiki/queries` is a saved query: its title, its text as
//!   the answer, the pages it links as its references, its date.
//!
//! Once per base (`wiki-files-imported` in `docbase_setting`, with what it
//! found), and only while the general project still has its folder.
//! Nothing in the folder is written or removed.

use std::path::{Path, PathBuf};

use super::*;

const IMPORTED_SETTING: &str = "wiki-files-imported";
const QUERIES_FOLDER: &str = "wiki/queries";

/// What the import found and kept.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiImport {
    /// Done before, or no folder to read: nothing was read.
    pub skipped: bool,
    /// Pages read, and those with notes of a person.
    pub pages: usize,
    pub with_notes: usize,
    /// Notes kept on their object, and kept apart (no object matched).
    pub to_objects: usize,
    pub orphans: usize,
    /// Saved queries.
    pub queries: usize,
}

impl DocBase {
    /// The import, once: from the folder of the general project's wiki when
    /// it is still there; nothing otherwise (and it will look again).
    pub fn import_wiki_files(&self) -> Result<WikiImport, String> {
        if setting(&self.conn(), IMPORTED_SETTING)?.is_some() {
            return Ok(WikiImport { skipped: true, ..Default::default() });
        }
        let root: Option<String> = self
            .conn()
            .query_row("SELECT path FROM project WHERE kind = 'general'", [], |row| row.get::<_, Option<String>>(0))
            .optional()
            .map_err(sql_err)?
            .flatten();
        match root.map(PathBuf::from).filter(|root| root.join("wiki").is_dir()) {
            Some(root) => self.import_wiki_files_from(&root),
            None => Ok(WikiImport { skipped: true, ..Default::default() }),
        }
    }

    /// The import from the wiki project at `root` (its `wiki/` inside),
    /// once per base.
    pub fn import_wiki_files_from(&self, root: &Path) -> Result<WikiImport, String> {
        if setting(&self.conn(), IMPORTED_SETTING)?.is_some() {
            return Ok(WikiImport { skipped: true, ..Default::default() });
        }
        let vocabulary = vocabulary();
        let notes_word = vocabulary.pages.word("notes");
        let families = family::families(self, vocabulary)?;
        let mut report = WikiImport::default();
        let conn = self.conn();
        let by_path: HashMap<String, String> = {
            let mut statement = conn.prepare("SELECT entity_id, path FROM knowledge_page").map_err(sql_err)?;
            let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
            rows.map(|row| row.map(|(id, path)| (path.to_lowercase(), id))).collect::<Result<_, _>>().map_err(sql_err)?
        };
        let tx = conn.unchecked_transaction().map_err(sql_err)?;
        for folder in [vocabulary.pages.folder(), vocabulary.pages.documents_folder()] {
            for file in markdown_files(&root.join(folder)) {
                let Some(name) = file.file_name().map(|n| n.to_string_lossy().to_string()) else { continue };
                let Ok(text) = std::fs::read_to_string(&file) else { continue };
                report.pages += 1;
                let notes = notes_of(&place_block(&text, None), notes_word);
                if notes.trim().is_empty() {
                    continue;
                }
                report.with_notes += 1;
                let logical = format!("{folder}/{name}");
                match object_of(&tx, &families, &by_path, &logical, &text)? {
                    Some(id) => {
                        add_notes(&tx, &id, &notes)?;
                        report.to_objects += 1;
                    }
                    None => {
                        let title = title_of(&text).unwrap_or_else(|| readable_stem(&logical));
                        tx.execute(
                            "INSERT INTO knowledge_note (object_id, title, origin_path, text, updated_at) VALUES (NULL, ?1, ?2, ?3, ?4)",
                            params![title, logical, notes, modified_ms(&file)],
                        )
                        .map_err(sql_err)?;
                        report.orphans += 1;
                    }
                }
            }
        }
        for file in markdown_files(&root.join(QUERIES_FOLDER)) {
            let Some(name) = file.file_name().map(|n| n.to_string_lossy().to_string()) else { continue };
            let Ok(text) = std::fs::read_to_string(&file) else { continue };
            let logical = format!("{QUERIES_FOLDER}/{name}");
            let query = saved_query(&text, &logical, modified_ms(&file));
            report.queries += tx
                .execute(
                    "INSERT OR IGNORE INTO knowledge_query (title, question, answer, refs, origin_path, created_at) VALUES (?1, '', ?2, ?3, ?4, ?5)",
                    params![query.title, query.answer, serde_json::to_string(&query.refs).unwrap_or_else(|_| "[]".into()), logical, query.created_at],
                )
                .map_err(sql_err)?;
        }
        set_setting(&tx, IMPORTED_SETTING, &serde_json::to_string(&report).unwrap_or_default())?;
        tx.commit().map_err(sql_err)?;
        Ok(report)
    }
}

/// The markdown files right inside `dir`, in order.
pub(super) fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md")))
        .collect();
    out.sort();
    out
}

fn modified_ms(file: &Path) -> i64 {
    std::fs::metadata(file)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as i64)
        .unwrap_or_else(now_ms)
}

/// A value of the header of a page (`key: value`), unquoted.
fn header_value(text: &str, key: &str) -> Option<String> {
    let text = text.replace("\r\n", "\n");
    let header = text.strip_prefix("---\n")?.split("\n---").next()?;
    let line = header.lines().find(|l| l.starts_with(&format!("{key}:")))?;
    let value = line[key.len() + 1..].trim();
    Some(value.trim_matches('"').trim_matches('\'').to_string()).filter(|v| !v.is_empty())
}

/// The object a page of the old wiki was about: the one of its path in the
/// base, or of its header, or (for a page of a document) the family of the
/// document the old wiki wrote it from.
fn object_of(
    conn: &Connection,
    families: &family::Families,
    by_path: &HashMap<String, String>,
    logical: &str,
    text: &str,
) -> Result<Option<String>, String> {
    // A document's page is its family's page now (the family may have
    // grown or split since).
    let of_document = |id: &str| -> Option<String> {
        let content = id.strip_prefix("doc-")?.parse::<i64>().ok()?;
        Some(families.family_of(content).map(|f| f.id.clone()).unwrap_or_else(|| id.to_string()))
    };
    let known = |id: &str| -> Result<bool, String> {
        let table = match id.split('-').next() {
            Some("ent") => "knowledge_entity",
            Some("mat") => "knowledge_matter",
            Some("doc") => return Ok(true),
            _ => return Ok(false),
        };
        conn.query_row(&format!("SELECT 1 FROM {table} WHERE id = ?1"), [id], |_| Ok(()))
            .optional()
            .map(|found| found.is_some())
            .map_err(sql_err)
    };
    if let Some(id) = by_path.get(&logical.to_lowercase()) {
        return Ok(Some(of_document(id).unwrap_or_else(|| id.clone())));
    }
    for key in ["entity_id", "document_id", "matter_id"] {
        if let Some(id) = header_value(text, key) {
            if known(&id)? {
                return Ok(Some(of_document(&id).unwrap_or(id)));
            }
        }
    }
    // A page the old ingest wrote of a document: the documents it was
    // written from, as the old index of the wiki kept them, or as its
    // header names them.
    let mut contents: Vec<i64> = {
        let found = conn.prepare(
            "SELECT DISTINCT f.version_id FROM wiki_page p
             JOIN wiki_page_source s ON s.page_id = p.id JOIN location l ON l.id = s.location_id
             JOIN file f ON f.hash = l.file_hash
             WHERE lower(p.path) = lower(?1) AND f.version_id IS NOT NULL ORDER BY f.version_id",
        );
        match found {
            Ok(mut statement) => statement
                .query_map([logical], |row| row.get::<_, i64>(0))
                .map_err(sql_err)?
                .collect::<Result<_, _>>()
                .map_err(sql_err)?,
            Err(_) => Vec::new(),
        }
    };
    if let Some(sources) = header_value(text, "sources") {
        let named: Vec<String> = sources
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        for (content, names) in &families.identities {
            if names.iter().any(|n| named.contains(&n.to_lowercase())) {
                contents.push(*content);
            }
        }
    }
    Ok(contents.into_iter().find_map(|c| families.family_of(c)).map(|f| f.id.clone()))
}

/// A saved query, as a file of `wiki/queries` has it.
#[derive(Debug, PartialEq)]
struct SavedQueryFile {
    title: String,
    answer: String,
    refs: Vec<String>,
    created_at: i64,
}

fn saved_query(text: &str, logical: &str, modified: i64) -> SavedQueryFile {
    let clean = text.replace("\r\n", "\n");
    let body = match clean.strip_prefix("---\n").and_then(|rest| rest.split_once("\n---")) {
        Some((_, after)) => after.trim_start_matches('-').trim().to_string(),
        None => clean.trim().to_string(),
    };
    let created_at = header_value(&clean, "created")
        .and_then(|date| chrono::NaiveDate::parse_from_str(date.get(..10).unwrap_or(&date), "%Y-%m-%d").ok())
        .and_then(|date| date.and_hms_opt(12, 0, 0))
        .map(|at| at.and_utc().timestamp_millis())
        .unwrap_or(modified);
    let mut refs: Vec<String> = Vec::new();
    let mut rest = body.as_str();
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        let target = after[..end].split('|').next().unwrap_or_default().split('#').next().unwrap_or_default().trim().to_string();
        if !target.is_empty() && !refs.contains(&target) {
            refs.push(target);
        }
        rest = &after[end + 2..];
    }
    SavedQueryFile { title: title_of(&clean).unwrap_or_else(|| readable_stem(logical)), answer: body, refs, created_at }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_query_keeps_its_title_answer_links_and_date() {
        let text = "---\ntype: query\ntitle: \"Plazo del contrato\"\ncreated: 2025-03-04\ntags: []\n---\n\nEl plazo es de doce meses, según [[contrato-uno|el contrato]] y [[persona-dos-1a2b3c4d]].\n";
        let query = saved_query(text, "wiki/queries/plazo.md", 7);
        assert_eq!(query.title, "Plazo del contrato");
        assert!(query.answer.starts_with("El plazo es de doce meses"), "{query:?}");
        assert_eq!(query.refs, vec!["contrato-uno".to_string(), "persona-dos-1a2b3c4d".to_string()]);
        assert!(query.created_at > 7);
        // Without a header: its file name and the date of the file.
        let bare = saved_query("Solo texto.", "wiki/queries/una-consulta.md", 7);
        assert_eq!((bare.title.as_str(), bare.created_at, bare.answer.as_str()), ("una consulta", 7, "Solo texto."));
    }
}
