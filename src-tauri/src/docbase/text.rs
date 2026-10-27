//! The text of each document, obtained once per content.
//!
//! Only deterministic extraction runs here: it costs nothing and never
//! invents. A document whose text is only pixels (a scan, a photo) is
//! marked as needing recognition instead of being sent to an engine
//! behind the user's back; its text arrives when the base recognises it
//! (`recognize.rs`: in the background while the automatic reading is on,
//! or when a person asks) and hands it to `store_recognized_text`.
//!
//! Two files with the same text are the same document: the Word file and
//! the PDF made from it end up as one content with two files.
//!
//! A fillable PDF form also keeps what was typed into its fields, both
//! inside the text and apart, as the `fields` derivative: the same blank
//! form filled for two people is two documents, and its fields are the
//! data that tell them apart.

use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension, Transaction};

use crate::commands::extract_images::{join_pdf_pages, pdf_pages_text, PdfPageContent};
use crate::panic_guard::run_guarded;
use crate::source_volume::catalog::fingerprint;
use crate::source_volume::recognition::{is_recognizable_image, textless_pages};
use crate::source_volume::{LocalFolderProvider, SourceProvider};

use super::{now_ms, sql_err, DocBase};

/// Name stored as the engine of text that came from plain extraction.
pub const EXTRACTION_ENGINE: &str = "extraction";

/// Kind of the derivative that holds the filled-in fields of a form.
pub const FIELDS_KIND: &str = "fields";

/// Version of what plain extraction reads of a file. Raised when it reads
/// more than before; the documents it would now read differently are then
/// extracted again (`refresh_extraction`). 2: which box of a form is
/// ticked, and the tick glyphs of a flattened form (P-135).
pub const EXTRACTION_VERSION: i64 = 2;
/// Where the version the texts of the base were extracted with is kept
/// (`docbase_setting`), and how far the check of the older ones went.
const EXTRACTION_VERSION_SETTING: &str = "text-extraction-version";
const EXTRACTION_REFRESH_SETTING: &str = "text-extraction-refreshed-to";

enum Outcome {
    /// Its whole text.
    Text(String),
    /// What could be extracted; the rest is pixels.
    Partial(String),
    NeedsRecognition,
    Unsupported,
}

/// What plain extraction read of a file, with its pages: each as it is in
/// the text, page 1 first; one for a file without pages, none when nothing
/// was read.
struct Read {
    outcome: Outcome,
    pages: Vec<String>,
}

impl DocBase {
    /// Obtains the text of up to `limit` documents that do not have it
    /// yet. Returns how many were attended.
    pub fn extract_pending(&self, limit: usize) -> Result<usize, String> {
        let jobs = self.next_text_jobs(limit)?;
        for (content_id, path, file_hash) in &jobs {
            self.engine_state().reading_file(Some(super::progress::Key::Content(*content_id)));
            let read = run_guarded("docbase_extract_text", || extract(path));
            let fields = run_guarded("docbase_form_fields", || form_fields(path)).unwrap_or_default();
            self.engine_state().reading_file(None);
            self.apply_text(*content_id, file_hash, read, &fields)?;
        }
        Ok(jobs.len())
    }

    /// Looks again, up to `limit` at a time, at the documents whose text an
    /// older version of the extraction wrote: a PDF whose form now reads
    /// differently (a ticked box it did not see) is marked to be extracted
    /// again, and its text changes with it; the others are left alone.
    /// Once all were looked at, the base keeps the current version and this
    /// does nothing. A base with no text yet is at the current version.
    /// Returns how many were looked at.
    pub fn refresh_extraction(&self, limit: usize) -> Result<usize, String> {
        self.refresh_extraction_with(limit, |path| form_fields(path).unwrap_or_default())
    }

    fn refresh_extraction_with(&self, limit: usize, read_fields: impl Fn(&Path) -> Vec<(String, String)>) -> Result<usize, String> {
        let setting = |conn: &rusqlite::Connection, key: &str| -> Result<Option<i64>, String> {
            conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [key], |row| row.get::<_, String>(0))
                .optional()
                .map_err(sql_err)
                .map(|v| v.and_then(|v| v.parse().ok()))
        };
        let jobs: Vec<(i64, PathBuf, Option<String>)> = {
            let conn = self.conn();
            if setting(&conn, EXTRACTION_VERSION_SETTING)?.unwrap_or(1) >= EXTRACTION_VERSION {
                return Ok(0);
            }
            let from = setting(&conn, EXTRACTION_REFRESH_SETTING)?.unwrap_or(0);
            let mut statement = conn
                .prepare(&format!(
                    "SELECT c.id, o.location, min(l.rel_path), fd.body
                     FROM version c
                     JOIN derivative d ON d.version_id = c.id AND d.kind = 'text' AND d.engine = ?3
                     LEFT JOIN derivative fd ON fd.version_id = c.id AND fd.kind = ?4
                     JOIN file f ON f.version_id = c.id
                     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done'
                     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                     WHERE c.state = 'processed' AND c.id > ?1 AND NOT {}
                     GROUP BY c.id
                     ORDER BY c.id
                     LIMIT ?2",
                    super::excluded_sql!()
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params![from, limit as i64, EXTRACTION_ENGINE, FIELDS_KIND], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?))
                })
                .map_err(sql_err)?;
            let mut jobs = Vec::new();
            for row in rows {
                let (content_id, root, rel_path, fields) = row.map_err(sql_err)?;
                // A path that cannot be reached now is looked at as none.
                let path = LocalFolderProvider::new(root).local_path(&rel_path).unwrap_or_default();
                jobs.push((content_id, path, fields));
            }
            jobs
        };
        let mut marked = Vec::new();
        for (content_id, path, kept) in &jobs {
            let fields = run_guarded("docbase_form_fields", || Ok::<_, String>(read_fields(path))).unwrap_or_default();
            let kept: Vec<(String, String)> = kept
                .as_deref()
                .and_then(|body| serde_json::from_str::<Vec<serde_json::Value>>(body).ok())
                .unwrap_or_default()
                .iter()
                .map(|f| (f["name"].as_str().unwrap_or_default().to_string(), f["value"].as_str().unwrap_or_default().to_string()))
                .collect();
            if fields != kept {
                marked.push(*content_id);
            }
        }
        let conn = self.conn();
        for content_id in &marked {
            conn.execute("UPDATE version SET state = 'pending' WHERE id = ?1 AND state = 'processed'", [content_id])
                .map_err(sql_err)?;
        }
        let last = jobs.last().map(|(id, ..)| *id);
        match last {
            Some(last) if jobs.len() >= limit => {
                conn.execute(
                    "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
                    params![EXTRACTION_REFRESH_SETTING, last.to_string()],
                )
                .map_err(sql_err)?;
            }
            _ => {
                conn.execute(
                    "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
                    params![EXTRACTION_VERSION_SETTING, EXTRACTION_VERSION.to_string()],
                )
                .map_err(sql_err)?;
                conn.execute("DELETE FROM docbase_setting WHERE key = ?1", [EXTRACTION_REFRESH_SETTING]).map_err(sql_err)?;
            }
        }
        Ok(jobs.len())
    }

    /// The contents to read, with the file read for each (its first path).
    fn next_text_jobs(&self, limit: usize) -> Result<Vec<(i64, PathBuf, String)>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                // With one min(), SQLite takes the other columns from its row.
                "SELECT c.id, o.location, min(l.rel_path), l.file_hash
                 FROM version c
                 JOIN file f ON f.version_id = c.id
                 JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done'
                 JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                 WHERE c.state = 'pending' AND NOT {}
                 GROUP BY c.id
                 ORDER BY c.id
                 LIMIT ?1",
                super::excluded_sql!()
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map([limit as i64], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))
            })
            .map_err(sql_err)?;
        let mut jobs = Vec::new();
        for row in rows {
            let (content_id, root, rel_path, file_hash) = row.map_err(sql_err)?;
            if let Some(path) = LocalFolderProvider::new(root).local_path(&rel_path) {
                jobs.push((content_id, path, file_hash));
            }
        }
        Ok(jobs)
    }

    fn apply_text(
        &self,
        content_id: i64,
        file_hash: &str,
        read: Result<Read, String>,
        fields: &[(String, String)],
    ) -> Result<(), String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        // Dropped or already attended while the file was being read.
        let pending: Option<i64> = tx
            .query_row("SELECT id FROM version WHERE id = ?1 AND state = 'pending'", [content_id], |row| {
                row.get(0)
            })
            .optional()
            .map_err(sql_err)?;
        if pending.is_none() {
            return Ok(());
        }
        let outcome = match read {
            Ok(read) => {
                let pages: Vec<(u32, &str, &str)> = read
                    .pages
                    .iter()
                    .enumerate()
                    .map(|(index, body)| (index as u32 + 1, EXTRACTION_ENGINE, body.as_str()))
                    .collect();
                store_pages(&tx, file_hash, &pages)?;
                Ok(read.outcome)
            }
            Err(error) => Err(error),
        };
        match outcome {
            Ok(Outcome::Text(text)) => {
                let kept = store(&tx, content_id, &text, EXTRACTION_ENGINE, "processed")?;
                store_fields(&tx, kept, fields)?;
            }
            Ok(Outcome::Partial(text)) => {
                let kept = store(&tx, content_id, &text, EXTRACTION_ENGINE, "needs-recognition")?;
                store_fields(&tx, kept, fields)?;
            }
            Ok(Outcome::NeedsRecognition) => set_state(&tx, content_id, "needs-recognition", None)?,
            Ok(Outcome::Unsupported) => set_state(&tx, content_id, "unsupported", None)?,
            Err(error) => set_state(&tx, content_id, "error", Some(&error))?,
        }
        tx.commit().map_err(sql_err)
    }

    /// Hands the base the text a recognition engine read from the file
    /// with this hash, so no other copy of it is ever recognised again.
    /// Returns false when the base does not know that file.
    pub fn store_recognized_text(&self, file_hash: &str, text: &str, engine: &str) -> Result<bool, String> {
        self.store_recognized_pages(file_hash, text, engine, &[])
    }

    /// Same as `store_recognized_text`, with the pages read, each with its
    /// number and the engine that read it (`resource_text`).
    pub fn store_recognized_pages(
        &self,
        file_hash: &str,
        text: &str,
        engine: &str,
        pages: &[(u32, &str, &str)],
    ) -> Result<bool, String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        let content_id: Option<i64> = tx
            .query_row("SELECT version_id FROM file WHERE hash = ?1", [file_hash], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        let Some(content_id) = content_id else {
            return Ok(false);
        };
        store_pages(&tx, file_hash, pages)?;
        store(&tx, content_id, text, engine, "processed")?;
        tx.commit().map_err(sql_err)?;
        Ok(true)
    }

    /// The text of the document with this file hash, when the base has
    /// it whole (a document still waiting for recognition does not count).
    #[cfg_attr(not(test), allow(dead_code))] // Only the tests ask it today.
    pub fn text_of_file(&self, file_hash: &str) -> Result<Option<String>, String> {
        self.conn()
            .query_row(
                "SELECT d.body FROM file f
                 JOIN version c ON c.id = f.version_id AND c.state = 'processed'
                 JOIN derivative d ON d.version_id = c.id AND d.kind = 'text'
                 WHERE f.hash = ?1",
                [file_hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)
    }
}

/// Saves the text of a content. If another content already has this
/// same text, the two are one document: this one's files join the other
/// and this one goes away. Returns the content that keeps the text.
///
/// A document generated from a template (D-488) is never another one with
/// the same text (the one of the template whose data it started with): it
/// stays a document of its own, without the identity of its text.
fn store(tx: &Transaction<'_>, content_id: i64, text: &str, engine: &str, state: &str) -> Result<i64, String> {
    let now = now_ms();
    let identity = fingerprint(text);
    if identity.is_some() && super::similar::generate::is_generated(tx, content_id)? {
        let taken: bool = tx
            .query_row(
                "SELECT count(*) FROM version WHERE text_hash = ?1 AND id != ?2",
                params![identity.as_ref().map(|(hash, _)| hash.clone()), content_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sql_err)?
            > 0;
        if taken {
            let words = identity.map(|(_, words)| words).unwrap_or_default();
            return store_alone(tx, content_id, text, engine, state, words as i64, now);
        }
    }
    if let Some((text_hash, _)) = &identity {
        let twin: Option<i64> = tx
            .query_row(
                "SELECT id FROM version WHERE text_hash = ?1 AND id != ?2",
                params![text_hash, content_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        if let Some(twin) = twin {
            tx.execute("UPDATE file SET version_id = ?1 WHERE version_id = ?2", params![twin, content_id])
                .map_err(sql_err)?;
            // Its ficha of shape is made again with every file it now has,
            // so each one gets the prints of its pages (`page_shapes.rs`).
            tx.execute(
                "DELETE FROM derivative WHERE version_id = ?1 AND kind = ?2",
                params![twin, super::page_shapes::SHAPES_KIND],
            )
            .map_err(sql_err)?;
            tx.execute("DELETE FROM content_fts WHERE rowid = ?1", [content_id])
                .map_err(sql_err)?;
            tx.execute("DELETE FROM content_search WHERE rowid = ?1", [content_id])
                .map_err(sql_err)?;
            tx.execute("DELETE FROM version WHERE id = ?1", [content_id])
                .map_err(sql_err)?;
            return Ok(twin);
        }
    }
    let words = match &identity {
        Some((_, words)) => *words as i64,
        None => text.split_whitespace().count() as i64,
    };
    tx.execute(
        "UPDATE version SET state = ?1, text_hash = ?2, word_count = ?3, error = NULL, processed_at = ?4 WHERE id = ?5",
        params![state, identity.map(|(hash, _)| hash), words, now, content_id],
    )
    .map_err(sql_err)?;
    tx.execute(
        "INSERT INTO derivative (version_id, kind, engine, body, created_at) VALUES (?1, 'text', ?2, ?3, ?4)
         ON CONFLICT (version_id, kind) DO UPDATE SET engine = excluded.engine, body = excluded.body, created_at = excluded.created_at",
        params![content_id, engine, text, now],
    )
    .map_err(sql_err)?;
    tx.execute("DELETE FROM content_fts WHERE rowid = ?1", [content_id])
        .map_err(sql_err)?;
    tx.execute("INSERT INTO content_fts (rowid, body) VALUES (?1, ?2)", params![content_id, text])
        .map_err(sql_err)?;
    super::search::index_content(tx, content_id)?;
    Ok(content_id)
}

/// Keeps the text of each page of a file as an engine read it
/// (`resource_text`): (page, engine, text), replacing what that engine had
/// read of that page.
fn store_pages(tx: &Transaction<'_>, file_hash: &str, pages: &[(u32, &str, &str)]) -> Result<(), String> {
    let now = now_ms();
    for (page, engine, body) in pages {
        tx.execute(
            "INSERT OR REPLACE INTO resource_text (resource_hash, page_no, engine, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![file_hash, page, engine, body, now],
        )
        .map_err(sql_err)?;
    }
    Ok(())
}

/// Saves the text of a content that keeps no identity of its text.
fn store_alone(tx: &Transaction<'_>, content_id: i64, text: &str, engine: &str, state: &str, words: i64, now: i64) -> Result<i64, String> {
    tx.execute(
        "UPDATE version SET state = ?1, text_hash = NULL, word_count = ?2, error = NULL, processed_at = ?3 WHERE id = ?4",
        params![state, words, now, content_id],
    )
    .map_err(sql_err)?;
    tx.execute(
        "INSERT INTO derivative (version_id, kind, engine, body, created_at) VALUES (?1, 'text', ?2, ?3, ?4)
         ON CONFLICT (version_id, kind) DO UPDATE SET engine = excluded.engine, body = excluded.body, created_at = excluded.created_at",
        params![content_id, engine, text, now],
    )
    .map_err(sql_err)?;
    tx.execute("DELETE FROM content_fts WHERE rowid = ?1", [content_id]).map_err(sql_err)?;
    tx.execute("INSERT INTO content_fts (rowid, body) VALUES (?1, ?2)", params![content_id, text]).map_err(sql_err)?;
    super::search::index_content(tx, content_id)?;
    Ok(content_id)
}

/// Keeps the filled-in fields of a form apart from its text, as a list
/// of {"name", "value"} in page order. Nothing is stored for a document
/// that is not a form.
fn store_fields(tx: &Transaction<'_>, content_id: i64, fields: &[(String, String)]) -> Result<(), String> {
    if fields.is_empty() {
        return Ok(());
    }
    let body = serde_json::to_string(
        &fields
            .iter()
            .map(|(name, value)| serde_json::json!({ "name": name, "value": value }))
            .collect::<Vec<_>>(),
    )
    .map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO derivative (version_id, kind, engine, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (version_id, kind) DO UPDATE SET engine = excluded.engine, body = excluded.body, created_at = excluded.created_at",
        params![content_id, FIELDS_KIND, EXTRACTION_ENGINE, body, now_ms()],
    )
    .map_err(sql_err)?;
    Ok(())
}

fn set_state(tx: &Transaction<'_>, content_id: i64, state: &str, error: Option<&str>) -> Result<(), String> {
    tx.execute(
        "UPDATE version SET state = ?1, error = ?2, processed_at = ?3 WHERE id = ?4",
        params![state, error, now_ms(), content_id],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// The filled-in fields of a fillable PDF; empty for anything else.
fn form_fields(path: &Path) -> Result<Vec<(String, String)>, String> {
    let is_pdf = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
    if !is_pdf {
        return Ok(Vec::new());
    }
    crate::commands::pdf_form::pdf_form_fields(&path.to_string_lossy())
}

fn extract(path: &Path) -> Result<Read, String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext == "pdf" {
        // The text of a PDF is its pages joined; each page is kept as it
        // is in the text.
        let pages: Vec<String> = pdf_pages_text(&path.to_string_lossy())?.iter().map(PdfPageContent::body).collect();
        let text = join_pdf_pages(&pages);
        // A page with no text layer is a scanned page.
        let outcome = if textless_pages(path).map(|pages| pages.is_empty()).unwrap_or(true) {
            Outcome::Text(text)
        } else if fingerprint(&text).is_some() {
            Outcome::Partial(text)
        } else {
            Outcome::NeedsRecognition
        };
        return Ok(Read { outcome, pages });
    }
    Ok(match crate::commands::fs::extract_text_without_recognition(path)? {
        Some(text) => Read { pages: vec![text.clone()], outcome: Outcome::Text(text) },
        None if is_recognizable_image(&ext) => Read { outcome: Outcome::NeedsRecognition, pages: Vec::new() },
        None => Read { outcome: Outcome::Unsupported, pages: Vec::new() },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::query::DocumentFilter;
    use crate::docbase::testing::TempDir;
    use crate::docbase::Visible;

    const CONTRACT: &str = "Entre Andina Sociedad Anónima, con domicilio en la calle Florida 123 de la Ciudad de Buenos Aires, \
        en adelante el proveedor, y BOREAL Sociedad Anónima, en adelante el cliente, se celebra el presente contrato de \
        prestación de servicios de mantenimiento, sujeto a las cláusulas y condiciones que se detallan a continuación.";

    fn count(base: &DocBase, sql: &str) -> i64 {
        base.conn().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn settle(base: &DocBase, origin_id: &str) {
        base.scan_origin(origin_id).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
        while base.extract_pending(50).unwrap() > 0 {}
        base.purge_orphans().unwrap();
    }

    fn state_of(base: &DocBase, name: &str) -> String {
        base.documents(&Visible::All, &DocumentFilter::default())
            .unwrap()
            .rows
            .into_iter()
            .find(|row| row.name == name)
            .unwrap()
            .state
    }

    #[test]
    fn the_text_of_a_document_is_obtained_once_however_many_copies_exist() {
        let dir = TempDir::new();
        dir.write("o/contrato.md", CONTRACT.as_bytes());
        dir.write("o/copia/contrato firmado.md", CONTRACT.as_bytes());
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);

        assert_eq!(count(&base, "SELECT count(*) FROM version WHERE state = 'processed'"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM derivative WHERE kind = 'text'"), 1);
        assert_eq!(state_of(&base, "contrato.md"), "processed");
        assert_eq!(state_of(&base, "contrato firmado.md"), "processed");

        let rows = base.documents(&Visible::All, &DocumentFilter::default()).unwrap().rows;
        let detail = base.document(&Visible::All, rows[0].location_id).unwrap().unwrap();
        assert_eq!(detail.text.as_deref(), Some(CONTRACT));
        assert_eq!(detail.text_engine.as_deref(), Some(EXTRACTION_ENGINE));
        assert!(detail.word_count.unwrap() > 30);
    }

    #[test]
    fn a_form_read_better_by_a_newer_extraction_is_extracted_again() {
        let dir = TempDir::new();
        dir.write("o/ficha.md", CONTRACT.as_bytes());
        dir.write("o/otro.md", "Un texto distinto de cualquier otro, con palabras suficientes para ser un documento entero.".as_bytes());
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let read = |path: &Path| -> Vec<(String, String)> {
            if path.ends_with("ficha.md") {
                vec![("Pregunta impresa".to_string(), "SI (marcada)".to_string())]
            } else {
                Vec::new()
            }
        };
        // One at a time: the form is marked, the other is left alone.
        let mut looked = 0;
        loop {
            let n = base.refresh_extraction_with(1, read).unwrap();
            if n == 0 {
                break;
            }
            looked += n;
        }
        assert_eq!(looked, 2);
        assert_eq!(count(&base, "SELECT count(*) FROM version WHERE state = 'pending'"), 1);
        assert_eq!(state_of(&base, "otro.md"), "processed");
        // Once done, the base is at the current version: nothing else.
        assert_eq!(base.refresh_extraction_with(10, |_: &Path| vec![("x".to_string(), "y".to_string())]).unwrap(), 0);
        while base.extract_pending(50).unwrap() > 0 {}
        assert_eq!(state_of(&base, "ficha.md"), "processed");
    }

    #[test]
    fn a_base_with_no_text_yet_is_at_the_current_extraction() {
        let base = DocBase::in_memory().unwrap();
        assert_eq!(base.refresh_extraction(10).unwrap(), 0);
        let version: String = base
            .conn()
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [EXTRACTION_VERSION_SETTING], |row| row.get(0))
            .unwrap();
        assert_eq!(version, EXTRACTION_VERSION.to_string());
    }

    #[test]
    fn the_same_text_in_another_file_is_the_same_document() {
        let dir = TempDir::new();
        dir.write("o/contrato.md", CONTRACT.as_bytes());
        // Same words, other bytes: another format, other line breaks.
        dir.write("o/contrato.txt", CONTRACT.replace(", ", ",\r\n").as_bytes());
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);

        assert_eq!(count(&base, "SELECT count(*) FROM file"), 2);
        assert_eq!(count(&base, "SELECT count(*) FROM version"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM content_fts"), 1);
        // The two files are two appearances of that one version, which has
        // one document; the one that went took its own document (D-551).
        assert_eq!(
            count(&base, "SELECT count(*) FROM appearance a JOIN version v ON v.id = a.version_id WHERE a.selector = 'file' AND a.cut_state = 'confirmed'"),
            2
        );
        assert_eq!(count(&base, "SELECT count(*) FROM document"), 1);
        assert_eq!(
            count(&base, "SELECT count(*) FROM document d JOIN version v ON v.document_id = d.id AND d.prevailing_version_id = v.id"),
            1
        );

        let rows = base.documents(&Visible::All, &DocumentFilter::default()).unwrap().rows;
        assert!(rows.iter().all(|row| row.copies == 2));
        let detail = base.document(&Visible::All, rows[0].location_id).unwrap().unwrap();
        assert_eq!(detail.locations.iter().filter(|location| !location.same_file).count(), 1);
    }

    #[test]
    fn images_wait_for_recognition_and_media_has_no_text() {
        let dir = TempDir::new();
        dir.write("o/foto de la carta.jpg", b"\xff\xd8\xff\xe0 no es texto");
        dir.write("o/audiencia.mp3", b"ID3 audio");
        dir.write("o/plano.dwg", b"\x00\x01\x02\xfe\xff binario");
        dir.write("o/nota.txt", b"breve");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);

        assert_eq!(state_of(&base, "foto de la carta.jpg"), "needs-recognition");
        assert_eq!(state_of(&base, "audiencia.mp3"), "unsupported");
        assert_eq!(state_of(&base, "plano.dwg"), "unsupported");
        // Too short to tell documents apart by text, but still text.
        assert_eq!(state_of(&base, "nota.txt"), "processed");
        assert_eq!(count(&base, "SELECT count(*) FROM version WHERE text_hash IS NOT NULL"), 0);
    }

    #[test]
    fn recognised_text_reaches_every_copy_of_the_file() {
        let dir = TempDir::new();
        dir.write("o/escaneo/doc0042.jpg", b"\xff\xd8 la misma foto");
        dir.write("o/socio/Carta BOREAL.jpg", b"\xff\xd8 la misma foto");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let hash: String = base
            .conn()
            .query_row("SELECT hash FROM file", [], |row| row.get(0))
            .unwrap();
        assert!(base.text_of_file(&hash).unwrap().is_none());

        assert!(base.store_recognized_text(&hash, CONTRACT, "codex-cli").unwrap());
        assert_eq!(base.text_of_file(&hash).unwrap().as_deref(), Some(CONTRACT));
        assert_eq!(state_of(&base, "doc0042.jpg"), "processed");
        assert_eq!(state_of(&base, "Carta BOREAL.jpg"), "processed");
        assert!(!base.store_recognized_text("no-such-hash", CONTRACT, "codex-cli").unwrap());
    }

    #[test]
    fn documents_are_found_by_the_words_of_their_text() {
        let dir = TempDir::new();
        dir.write("o/a.md", CONTRACT.as_bytes());
        dir.write("o/rescisión.md", "Carta documento por la rescisión unilateral del acuerdo.".as_bytes());
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let find = |query: &str| -> Vec<String> {
            let filter = DocumentFilter { query: Some(query.to_string()), ..Default::default() };
            let mut names: Vec<String> = base
                .documents(&Visible::All, &filter)
                .unwrap()
                .rows
                .into_iter()
                .map(|row| row.name)
                .collect();
            names.sort();
            names
        };

        // In the text only, without accents, by the start of a word.
        assert_eq!(find("mantenimiento"), ["a.md"]);
        assert_eq!(find("clausulas"), ["a.md"]);
        assert_eq!(find("manten"), ["a.md"]);
        // One word from the name and one from the text.
        assert_eq!(find("rescision unilateral"), ["rescisión.md"]);
        // Every word has to be somewhere.
        assert!(find("mantenimiento unilateral").is_empty());
        // Signs that mean something to the search engine are just text.
        assert!(find("\"*) OR").is_empty());
    }
    /// A one-page fillable PDF: the page says `page`, and its only text
    /// field, "Nombre", holds `name`.
    fn filled_form(page: &str, name: &str) -> Vec<u8> {
        let content = format!("BT /F1 10 Tf 40 760 Td ({page}) Tj ET");
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [5 0 R] >> >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
             /Resources << /Font << /F1 6 0 R >> >> /Annots [5 0 R] >>"
                .to_string(),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
            format!("<< /Type /Annot /Subtype /Widget /FT /Tx /T (Nombre) /V ({name}) /Rect [40 700 300 720] /P 3 0 R /F 4 >>"),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];
        let mut pdf = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
        }
        let xref = pdf.len();
        pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1));
        for offset in offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1));
        pdf.into_bytes()
    }

    /// The text of a PDF as it was put together before it was made from
    /// its pages: the reference the text of a document must keep, byte by
    /// byte (the answers kept of the model are found by it, D-555).
    fn joined_as_before(path: &Path) -> String {
        let _guard = crate::commands::fs::lock_pdfium();
        let pdfium = crate::commands::fs::pdfium().unwrap();
        let doc = pdfium.load_pdf_from_file(&path.to_string_lossy().to_string(), None).unwrap();
        let mut out = String::new();
        for (page_idx, page) in doc.pages().iter().enumerate() {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str(&format!("## Page {}\n\n", page_idx + 1));
            out.push_str(&page.text().unwrap().all());
            out.push('\n');
            let fields = crate::commands::pdf_form::page_form_fields(&page);
            if !fields.is_empty() {
                out.push_str(&format!("\n{}\n\n", crate::commands::pdf_form::FORM_FIELDS_HEADING));
                for (name, value) in &fields {
                    out.push_str(&format!("- {name}: {value}\n"));
                }
            }
        }
        out
    }

    fn pages_of(base: &DocBase, hash: &str) -> Vec<(i64, String, String)> {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT page_no, engine, body FROM resource_text WHERE resource_hash = ?1 ORDER BY page_no, engine")
            .unwrap();
        let rows = statement.query_map([hash], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap();
        rows.map(|row| row.unwrap()).collect()
    }

    fn hash_of(base: &DocBase, name: &str) -> String {
        base.conn()
            .query_row("SELECT file_hash FROM location WHERE rel_path = ?1", [name], |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn the_text_made_from_the_pages_is_the_same_as_before_and_each_page_is_kept() {
        use crate::source_volume::page_shape::test_pdf;
        let form = "Ficha de cliente. Complete todos los campos con los datos de la persona o de la sociedad, \
                    indique domicilio y actividad, y firme al pie.";
        let dir = TempDir::new();
        let filled = dir.write("o/ficha.pdf", &filled_form(form, "Ana Gomez"));
        let long = dir.write(
            "o/nota.pdf",
            &test_pdf::build(&[
                test_pdf::page(&["Primera hoja de la nota de prueba", "con dos renglones"], 0.0),
                test_pdf::page(&["Segunda hoja"], 0.0),
                test_pdf::page(&[], 0.0),
            ]),
        );
        dir.write("o/carta.txt", CONTRACT.as_bytes());
        for path in [&filled, &long] {
            let before = joined_as_before(path);
            let now = crate::commands::extract_images::extract_pdf_markdown(
                &path.to_string_lossy(),
                None,
                "",
                &crate::commands::extract_images::ExtractOptions::default(),
            )
            .unwrap();
            assert_eq!(now, before);
        }
        assert!(joined_as_before(&filled).contains("Nombre: Ana Gomez"));

        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        for (name, path) in [("ficha.pdf", &filled), ("nota.pdf", &long)] {
            let hash = hash_of(&base, name);
            let text = base.text_of_file(&hash).unwrap().unwrap();
            assert_eq!(text, joined_as_before(path), "{name}");
            let pages = pages_of(&base, &hash);
            assert!(pages.iter().all(|(_, engine, _)| engine == EXTRACTION_ENGINE));
            let numbers: Vec<i64> = pages.iter().map(|(page, _, _)| *page).collect();
            assert_eq!(numbers, (1..=numbers.len() as i64).collect::<Vec<_>>());
            // The pages joined are the text of the document.
            assert_eq!(join_pdf_pages(pages.iter().map(|(_, _, body)| body.as_str())), text, "{name}");
        }
        assert_eq!(pages_of(&base, &hash_of(&base, "nota.pdf")).len(), 3);
        // A file without pages has one, the whole text.
        let letter = pages_of(&base, &hash_of(&base, "carta.txt"));
        assert_eq!(letter, vec![(1, EXTRACTION_ENGINE.to_string(), base.text_of_file(&hash_of(&base, "carta.txt")).unwrap().unwrap())]);
    }

    #[test]
    fn the_pages_recognized_are_kept_with_their_engine() {
        let dir = TempDir::new();
        dir.write("o/escrito.png", b"not really an image");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let hash = hash_of(&base, "escrito.png");
        assert!(pages_of(&base, &hash).is_empty());
        assert!(base.store_recognized_pages(&hash, CONTRACT, "motor-a", &[(1, "motor-a", CONTRACT)]).unwrap());
        assert_eq!(pages_of(&base, &hash), vec![(1, "motor-a".to_string(), CONTRACT.to_string())]);
        assert_eq!(base.text_of_file(&hash).unwrap().as_deref(), Some(CONTRACT));
    }

    #[test]
    fn the_same_form_filled_for_two_people_is_two_documents() {
        let form = "Ficha de cliente. Complete todos los campos con los datos de la persona o de la sociedad, \
                    indique domicilio, actividad, origen de los fondos y si reviste la condicion de persona \
                    expuesta politicamente, y firme al pie de cada hoja.";
        let dir = TempDir::new();
        dir.write("o/ficha uno.pdf", &filled_form(form, "Ana Gomez"));
        dir.write("o/ficha dos.pdf", &filled_form(form, "Bruno Vidal"));
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);

        // Before, only the page was read: both read alike and became one.
        assert_eq!(count(&base, "SELECT count(*) FROM version WHERE state = 'processed'"), 2);
        let rows = base.documents(&Visible::All, &DocumentFilter::default()).unwrap().rows;
        assert!(rows.iter().all(|row| row.copies == 1));

        // What was typed is part of the text, so it can be searched...
        let one = rows.iter().find(|row| row.name == "ficha uno.pdf").unwrap();
        let detail = base.document(&Visible::All, one.location_id).unwrap().unwrap();
        let text = detail.text.unwrap();
        assert!(text.contains("Ficha de cliente"));
        assert!(text.contains("Nombre: Ana Gomez"));
        assert!(!text.contains("Bruno"));

        // ...and kept apart, field by field.
        let fields: String = base
            .conn()
            .query_row(
                "SELECT d.body FROM derivative d JOIN file f ON f.version_id = d.version_id
                 JOIN location l ON l.file_hash = f.hash
                 WHERE d.kind = 'fields' AND l.name = 'ficha dos.pdf'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fields, r#"[{"name":"Nombre","value":"Bruno Vidal"}]"#);
        assert_eq!(count(&base, "SELECT count(*) FROM derivative WHERE kind = 'fields'"), 2);
    }

    /// A one-page PDF form. `texts` are (x, y, words) printed on the page;
    /// `objects` are the form's own objects, numbered from 7 on; `annots`
    /// and `fields` are the numbers of the page's widgets and of the
    /// form's top fields. Object 6 is an empty appearance for any button.
    fn form_pdf(texts: &[(u32, u32, &str)], objects: &[String], annots: &[usize], fields: &[usize]) -> Vec<u8> {
        // Enough printed text for the page not to pass for a scan.
        let preamble = [
            (40, 770, "Formulario de alta. Complete los datos de la persona y marque las opciones que correspondan."),
            (40, 758, "La informacion se presenta con caracter de declaracion jurada y queda reservada al estudio."),
        ];
        let content = preamble
            .iter()
            .chain(texts.iter())
            .map(|(x, y, words)| format!("BT /F1 10 Tf {x} {y} Td ({words}) Tj ET"))
            .collect::<Vec<_>>()
            .join("\n");
        let refs = |numbers: &[usize]| numbers.iter().map(|n| format!("{n} 0 R")).collect::<Vec<_>>().join(" ");
        let mut all = vec![
            format!("<< /Type /Catalog /Pages 2 0 R /AcroForm << /Fields [{}] >> >>", refs(fields)),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R \
                 /Resources << /Font << /F1 5 0 R >> >> /Annots [{}] >>",
                refs(annots)
            ),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
            "<< /Type /XObject /Subtype /Form /BBox [0 0 12 12] /Length 0 >>\nstream\n\nendstream".to_string(),
        ];
        all.extend(objects.iter().cloned());
        let mut pdf = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        for (index, object) in all.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.push_str(&format!("{} 0 obj\n{object}\nendobj\n", index + 1));
        }
        let xref = pdf.len();
        pdf.push_str(&format!("xref\n0 {}\n0000000000 65535 f \n", all.len() + 1));
        for offset in offsets {
            pdf.push_str(&format!("{offset:010} 00000 n \n"));
        }
        pdf.push_str(&format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", all.len() + 1));
        pdf.into_bytes()
    }

    /// A tick box or option button at `rect` whose "on" state is `on`,
    /// currently showing `state`. `own` is its own field part, if it is a
    /// field by itself (`/FT /Btn /T (...)`), or its `/Parent n 0 R`.
    fn button(own: &str, rect: &str, on: &str, state: &str) -> String {
        format!(
            "<< /Type /Annot /Subtype /Widget {own} /Rect [{rect}] /AS /{state} \
             /AP << /N << /{on} 6 0 R /Off 6 0 R >> >> /P 3 0 R /F 4 >>"
        )
    }

    /// The text and the `fields` derivative of the only document in `pdf`.
    fn read_form(pdf: &[u8]) -> (String, String) {
        let dir = TempDir::new();
        dir.write("o/formulario.pdf", pdf);
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let rows = base.documents(&Visible::All, &DocumentFilter::default()).unwrap().rows;
        let detail = base.document(&Visible::All, rows[0].location_id).unwrap().unwrap();
        let text = detail.text.unwrap_or_else(|| panic!("no text: {} {:?}", rows[0].state, detail.error));
        let fields: String = base
            .conn()
            .query_row("SELECT body FROM derivative WHERE kind = 'fields'", [], |row| row.get(0))
            .unwrap();
        (text, fields)
    }

    #[test]
    fn an_option_group_says_which_option_was_chosen() {
        // One field, "Beneficiario final", with two buttons: SI and NO.
        let pdf = form_pdf(
            &[(40, 740, "Declaracion jurada de la persona que contrata el servicio")],
            &[
                "<< /FT /Btn /Ff 49152 /T (Beneficiario final) /V /NO /Kids [8 0 R 9 0 R] >>".to_string(),
                button("/Parent 7 0 R", "100 600 112 612", "SI", "Off"),
                button("/Parent 7 0 R", "160 600 172 612", "NO", "NO"),
            ],
            &[8, 9],
            &[7],
        );
        let (text, fields) = read_form(&pdf);
        assert!(text.contains("- Beneficiario final: NO (marcada)"), "{text}");
        assert!(!text.contains("SI"), "{text}");
        assert_eq!(fields, r#"[{"name":"Beneficiario final","value":"NO (marcada)"}]"#);
    }

    #[test]
    fn a_ticked_box_is_marked_and_an_unticked_one_is_left_out() {
        // The second box is "on" with a word other than "Yes".
        let pdf = form_pdf(
            &[(40, 740, "Preferencias de contacto del cliente para avisos y resumenes")],
            &[
                button("/FT /Btn /T (Acepta notificaciones)", "100 600 112 612", "Yes", "Yes"),
                button("/FT /Btn /T (Recibe resumen mensual)", "100 560 112 572", "Si", "Si"),
                button("/FT /Btn /T (Autoriza debito)", "100 520 112 532", "Yes", "Off"),
            ],
            &[7, 8, 9],
            &[7, 8, 9],
        );
        let (text, fields) = read_form(&pdf);
        assert!(text.contains("- Acepta notificaciones: marcado"), "{text}");
        assert!(text.contains("- Recibe resumen mensual: marcado"), "{text}");
        assert!(!text.contains("Autoriza"), "{text}");
        assert_eq!(
            fields,
            r#"[{"name":"Acepta notificaciones","value":"marcado"},{"name":"Recibe resumen mensual","value":"marcado"}]"#
        );
    }

    #[test]
    fn unnamed_boxes_take_their_question_and_label_from_the_page() {
        // As a form editor leaves them: one box per answer, named "Check
        // Box", each with the same "on" word; what tells them apart is the
        // text printed on their row. Labels are before their boxes here.
        let pdf = form_pdf(
            &[
                (40, 600, "Es cliente habitual del estudio?"),
                (300, 600, "SI"),
                (340, 600, "NO"),
                (40, 560, "Tiene deudas pendientes?"),
                (300, 560, "SI"),
                (340, 560, "NO"),
            ],
            &[
                button("/FT /Btn /T (Check Box1)", "315 596 327 610", "Si", "Off"),
                button("/FT /Btn /T (Check Box2)", "360 596 372 610", "Si", "Si"),
                button("/FT /Btn /T (Check Box3)", "315 556 327 570", "Si", "Si"),
                button("/FT /Btn /T (Check Box4)", "360 556 372 570", "Si", "Off"),
            ],
            &[7, 8, 9, 10],
            &[7, 8, 9, 10],
        );
        let (text, fields) = read_form(&pdf);
        assert!(text.contains("- Es cliente habitual del estudio?: NO (marcada)"), "{text}");
        assert!(text.contains("- Tiene deudas pendientes?: SI (marcada)"), "{text}");
        assert!(!text.contains("Check Box"), "{text}");
        assert_eq!(
            fields,
            r#"[{"name":"Es cliente habitual del estudio?","value":"NO (marcada)"},{"name":"Tiene deudas pendientes?","value":"SI (marcada)"}]"#
        );
    }

    #[test]
    fn an_option_group_with_placeholder_states_takes_the_printed_labels() {
        // States "Choice1" and "Choice2" say nothing: the printed label does.
        let pdf = form_pdf(
            &[(40, 600, "Condicion frente al impuesto"), (220, 600, "Inscripto"), (320, 600, "Exento")],
            &[
                "<< /FT /Btn /Ff 49152 /T (Group1) /V /Choice2 /Kids [8 0 R 9 0 R] >>".to_string(),
                button("/Parent 7 0 R", "205 596 217 610", "Choice1", "Off"),
                button("/Parent 7 0 R", "305 596 317 610", "Choice2", "Choice2"),
            ],
            &[8, 9],
            &[7],
        );
        let (text, _) = read_form(&pdf);
        assert!(text.contains("- Condicion frente al impuesto: Exento"), "{text}");
    }
}
