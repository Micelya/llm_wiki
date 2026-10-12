//! The text of each document, obtained once per content.
//!
//! Only deterministic extraction runs here: it costs nothing and never
//! invents. A document whose text is only pixels (a scan, a photo) is
//! marked as needing recognition instead of being sent to an engine
//! behind the user's back; its text arrives when something recognises
//! it and hands it to `store_recognized_text`.
//!
//! Two files with the same text are the same document: the Word file and
//! the PDF made from it end up as one content with two files.

use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension, Transaction};

use crate::panic_guard::run_guarded;
use crate::source_volume::catalog::fingerprint;
use crate::source_volume::recognition::{is_recognizable_image, textless_pages};
use crate::source_volume::{LocalFolderProvider, SourceProvider};

use super::{now_ms, sql_err, DocBase};

/// Name stored as the engine of text that came from plain extraction.
pub const EXTRACTION_ENGINE: &str = "extraction";

enum Outcome {
    /// Its whole text.
    Text(String),
    /// What could be extracted; the rest is pixels.
    Partial(String),
    NeedsRecognition,
    Unsupported,
}

impl DocBase {
    /// Obtains the text of up to `limit` documents that do not have it
    /// yet. Returns how many were attended.
    pub fn extract_pending(&self, limit: usize) -> Result<usize, String> {
        let jobs = self.next_text_jobs(limit)?;
        for (content_id, path) in &jobs {
            let outcome = run_guarded("docbase_extract_text", || extract(path));
            self.apply_text(*content_id, outcome)?;
        }
        Ok(jobs.len())
    }

    fn next_text_jobs(&self, limit: usize) -> Result<Vec<(i64, PathBuf)>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(&format!(
                "SELECT c.id, o.location, min(l.rel_path)
                 FROM content c
                 JOIN file f ON f.content_id = c.id
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
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(sql_err)?;
        let mut jobs = Vec::new();
        for row in rows {
            let (content_id, root, rel_path) = row.map_err(sql_err)?;
            if let Some(path) = LocalFolderProvider::new(root).local_path(&rel_path) {
                jobs.push((content_id, path));
            }
        }
        Ok(jobs)
    }

    fn apply_text(&self, content_id: i64, outcome: Result<Outcome, String>) -> Result<(), String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        // Dropped or already attended while the file was being read.
        let pending: Option<i64> = tx
            .query_row("SELECT id FROM content WHERE id = ?1 AND state = 'pending'", [content_id], |row| {
                row.get(0)
            })
            .optional()
            .map_err(sql_err)?;
        if pending.is_none() {
            return Ok(());
        }
        match outcome {
            Ok(Outcome::Text(text)) => store(&tx, content_id, &text, EXTRACTION_ENGINE, "processed")?,
            Ok(Outcome::Partial(text)) => {
                store(&tx, content_id, &text, EXTRACTION_ENGINE, "needs-recognition")?
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
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        let content_id: Option<i64> = tx
            .query_row("SELECT content_id FROM file WHERE hash = ?1", [file_hash], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        let Some(content_id) = content_id else {
            return Ok(false);
        };
        store(&tx, content_id, text, engine, "processed")?;
        tx.commit().map_err(sql_err)?;
        Ok(true)
    }

    /// The text of the document with this file hash, when the base has
    /// it whole (a document still waiting for recognition does not count).
    pub fn text_of_file(&self, file_hash: &str) -> Result<Option<String>, String> {
        self.conn()
            .query_row(
                "SELECT d.body FROM file f
                 JOIN content c ON c.id = f.content_id AND c.state = 'processed'
                 JOIN derivative d ON d.content_id = c.id AND d.kind = 'text'
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
/// and this one goes away.
fn store(tx: &Transaction<'_>, content_id: i64, text: &str, engine: &str, state: &str) -> Result<(), String> {
    let now = now_ms();
    let identity = fingerprint(text);
    if let Some((text_hash, _)) = &identity {
        let twin: Option<i64> = tx
            .query_row(
                "SELECT id FROM content WHERE text_hash = ?1 AND id != ?2",
                params![text_hash, content_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        if let Some(twin) = twin {
            tx.execute("UPDATE file SET content_id = ?1 WHERE content_id = ?2", params![twin, content_id])
                .map_err(sql_err)?;
            tx.execute("DELETE FROM content_fts WHERE rowid = ?1", [content_id])
                .map_err(sql_err)?;
            tx.execute("DELETE FROM content WHERE id = ?1", [content_id])
                .map_err(sql_err)?;
            return Ok(());
        }
    }
    let words = match &identity {
        Some((_, words)) => *words as i64,
        None => text.split_whitespace().count() as i64,
    };
    tx.execute(
        "UPDATE content SET state = ?1, text_hash = ?2, word_count = ?3, error = NULL, processed_at = ?4 WHERE id = ?5",
        params![state, identity.map(|(hash, _)| hash), words, now, content_id],
    )
    .map_err(sql_err)?;
    tx.execute(
        "INSERT INTO derivative (content_id, kind, engine, body, created_at) VALUES (?1, 'text', ?2, ?3, ?4)
         ON CONFLICT (content_id, kind) DO UPDATE SET engine = excluded.engine, body = excluded.body, created_at = excluded.created_at",
        params![content_id, engine, text, now],
    )
    .map_err(sql_err)?;
    tx.execute("DELETE FROM content_fts WHERE rowid = ?1", [content_id])
        .map_err(sql_err)?;
    tx.execute("INSERT INTO content_fts (rowid, body) VALUES (?1, ?2)", params![content_id, text])
        .map_err(sql_err)?;
    Ok(())
}

fn set_state(tx: &Transaction<'_>, content_id: i64, state: &str, error: Option<&str>) -> Result<(), String> {
    tx.execute(
        "UPDATE content SET state = ?1, error = ?2, processed_at = ?3 WHERE id = ?4",
        params![state, error, now_ms(), content_id],
    )
    .map_err(sql_err)?;
    Ok(())
}

fn extract(path: &Path) -> Result<Outcome, String> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
        .to_lowercase();
    match crate::commands::fs::extract_text_without_recognition(path)? {
        Some(text) if ext == "pdf" => {
            // A page with no text layer is a scanned page.
            if textless_pages(path).map(|pages| pages.is_empty()).unwrap_or(true) {
                Ok(Outcome::Text(text))
            } else if fingerprint(&text).is_some() {
                Ok(Outcome::Partial(text))
            } else {
                Ok(Outcome::NeedsRecognition)
            }
        }
        Some(text) => Ok(Outcome::Text(text)),
        None if is_recognizable_image(&ext) => Ok(Outcome::NeedsRecognition),
        None => Ok(Outcome::Unsupported),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::query::DocumentFilter;
    use crate::docbase::testing::TempDir;
    use crate::docbase::Visible;

    const CONTRACT: &str = "Entre Aeros Sociedad Anónima, con domicilio en la calle Florida 123 de la Ciudad de Buenos Aires, \
        en adelante el proveedor, y BGH Sociedad Anónima, en adelante el cliente, se celebra el presente contrato de \
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

        assert_eq!(count(&base, "SELECT count(*) FROM content WHERE state = 'processed'"), 1);
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
    fn the_same_text_in_another_file_is_the_same_document() {
        let dir = TempDir::new();
        dir.write("o/contrato.md", CONTRACT.as_bytes());
        // Same words, other bytes: another format, other line breaks.
        dir.write("o/contrato.txt", CONTRACT.replace(", ", ",\r\n").as_bytes());
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);

        assert_eq!(count(&base, "SELECT count(*) FROM file"), 2);
        assert_eq!(count(&base, "SELECT count(*) FROM content"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM content_fts"), 1);

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
        assert_eq!(count(&base, "SELECT count(*) FROM content WHERE text_hash IS NOT NULL"), 0);
    }

    #[test]
    fn recognised_text_reaches_every_copy_of_the_file() {
        let dir = TempDir::new();
        dir.write("o/escaneo/doc0042.jpg", b"\xff\xd8 la misma foto");
        dir.write("o/socio/Carta BGH.jpg", b"\xff\xd8 la misma foto");
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
        assert_eq!(state_of(&base, "Carta BGH.jpg"), "processed");
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
}
