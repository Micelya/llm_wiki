//! The text of scans and photos, recognized by the base itself (P-242).
//!
//! A document whose text is only pixels (a scanned PDF, a photo, or the
//! pages without text of a mixed PDF) waits in `needs-recognition` after
//! the extraction (`text.rs`). Here an engine behind the `TextRecognizer`
//! contract reads it, one document at a time:
//!
//! - by itself, in the background, with the engine of this computer
//!   (D-558, `recognition_local`), the automatic reading on or off: it
//!   asks no model in the cloud. Only while its models are there; without
//!   them, as before:
//! - by itself, in the background, while the automatic reading is on (the
//!   same switch as the questions to the model: each page is a question);
//! - when a person asks for it, even with the automatic reading off. What
//!   she asked for is kept in the base (`recognition_request`, in the order
//!   asked) until it is read or fails for good, so closing the app does not
//!   lose it.
//!
//! Once per document and engine: the pages already read are kept beside
//! the base (`recognized/<file hash>/page-N.json`), so a failure half-way
//! resumes where it stopped, and the whole text goes to
//! `store_recognized_text` with the engine that read it, so every copy of
//! the file has it and no copy is read again. A document read by another
//! engine is read again only when a person asks. Then the document follows
//! the usual path: copies, type, reading, matters, search.
//!
//! A failure of the provider (its usage limit, no connection) leaves the
//! document waiting and the engine tries again later; any other failure,
//! or the provider failing on the same document again and again, marks the
//! document with the reason until a person asks again.

use std::path::PathBuf;
use std::time::Duration;

use rusqlite::{params, Connection, OptionalExtension};

use crate::panic_guard::run_guarded;
use crate::source_volume::recognition::{self, is_recognizable_image, textless_pages, RecognitionConfig, TextRecognizer};
use crate::source_volume::recognized_store::RecognizedStore;
use crate::source_volume::{LocalFolderProvider, SourceProvider};

use super::similar::classify::stops_the_run;
use super::text::EXTRACTION_ENGINE;
use super::{now_ms, sql_err, DocBase};

/// Folder, beside the database, where the recognized pages are kept.
pub const RECOGNIZED_DIR: &str = "recognized";
/// After this many failures of the provider on the same document, the
/// document is marked as failed: a page that never gets an answer must not
/// hold the others back for ever.
const MAX_PROVIDER_TRIES: usize = 3;
/// What a failure says when no copy of the file can be reached now.
pub const UNREACHABLE: &str = "The file cannot be reached: its folder is not connected";

/// What one step of the recognition did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionStep {
    /// Nothing to recognize now.
    Idle,
    /// The text of this document was recognized and kept.
    Recognized(i64),
    /// Nothing to do for it: its text is not only pixels, or this engine
    /// already read it.
    Skipped(i64),
    /// It could not be recognized: it is marked with the reason.
    Failed(i64),
    /// The provider failed (its limit, no connection): try again later.
    Stopped(String),
}

/// Which documents a person asks to recognize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecognitionRequest {
    /// These documents: waiting, failed, or read by another engine.
    These(Vec<i64>),
    /// Every document waiting, the failed ones too.
    AllWaiting,
    /// The documents whose recognition failed.
    Failed,
}

/// The engine that recognizes text: the one of «Reconocer escaneos» in the
/// registry of AI services, when the base can run it (`services.rs`).
pub(crate) fn recognition_engine(conn: &Connection) -> Result<Option<RecognitionConfig>, String> {
    super::services::recognition_engine(conn)
}

/// SQL condition: the content `c` has a copy that can be read now.
const REACHABLE_SQL: &str = "EXISTS (SELECT 1 FROM file f
     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done'
     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
     WHERE f.content_id = c.id AND NOT {excluded})";

fn reachable_sql() -> String {
    REACHABLE_SQL.replace("{excluded}", super::excluded_sql!())
}

/// The documents a person asked to recognize and are still to do, in the
/// order asked (kept in the base: they survive closing the app).
pub(crate) fn requested_recognitions(conn: &Connection) -> Result<Vec<i64>, String> {
    let mut statement = conn
        .prepare("SELECT content_id FROM recognition_request ORDER BY position, content_id")
        .map_err(sql_err)?;
    let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

impl DocBase {
    pub fn recognition_engine(&self) -> Result<Option<RecognitionConfig>, String> {
        recognition_engine(&self.conn())
    }

    /// A person asks to recognize documents, with the automatic reading on
    /// or off: they go first, one after the other, and a failed one is
    /// tried again. Only documents still waiting count, or, among `These`,
    /// one read by an engine other than the one chosen now. Returns how
    /// many were asked for.
    pub fn request_recognition(&self, which: &RecognitionRequest) -> Result<usize, String> {
        let current = match self.recognition_engine()? {
            Some(config) => recognition::engine_id(&config)?,
            None => return Err("No engine to recognize the text of scans was chosen".to_string()),
        };
        let ids: Vec<i64> = {
            let conn = self.conn();
            let mut ids = Vec::new();
            match which {
                RecognitionRequest::These(wanted) => {
                    for id in wanted {
                        let found: Option<(String, Option<String>)> = conn
                            .query_row(
                                "SELECT c.state, d.engine FROM content c
                                 LEFT JOIN derivative d ON d.content_id = c.id AND d.kind = 'text' WHERE c.id = ?1",
                                [id],
                                |row| Ok((row.get(0)?, row.get(1)?)),
                            )
                            .optional()
                            .map_err(sql_err)?;
                        if let Some((state, engine)) = found {
                            let wanted = needs_engine(&state, engine.as_deref(), &current) || has_unread_scans(&conn, &self.recognized, &state, *id)?;
                            if wanted && !ids.contains(id) {
                                ids.push(*id);
                            }
                        }
                    }
                }
                RecognitionRequest::AllWaiting | RecognitionRequest::Failed => {
                    let failed_only = matches!(which, RecognitionRequest::Failed);
                    let mut statement = conn
                        .prepare(&format!(
                            "SELECT c.id FROM content c WHERE c.state = 'needs-recognition' AND (?1 = 0 OR c.error IS NOT NULL)
                             AND {} ORDER BY c.id",
                            reachable_sql()
                        ))
                        .map_err(sql_err)?;
                    let rows = statement.query_map([failed_only as i64], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
                    for row in rows {
                        ids.push(row.map_err(sql_err)?);
                    }
                }
            }
            // After the ones already asked for, each once.
            let last: i64 = conn
                .query_row("SELECT coalesce(max(position), 0) FROM recognition_request", [], |row| row.get(0))
                .map_err(sql_err)?;
            let at = now_ms();
            for (index, id) in ids.iter().enumerate() {
                conn.execute("UPDATE content SET error = NULL WHERE id = ?1 AND state = 'needs-recognition'", [id])
                    .map_err(sql_err)?;
                conn.execute(
                    "INSERT OR IGNORE INTO recognition_request (content_id, position, asked_at) VALUES (?1, ?2, ?3)",
                    params![id, last + 1 + index as i64, at],
                )
                .map_err(sql_err)?;
            }
            ids
        };
        self.engine_state().forget_recognition_tries(&ids);
        Ok(ids.len())
    }

    /// The recognition of a document a person may have asked for ended,
    /// well or for good: it is no longer asked for.
    fn recognition_ended(&self, content: i64) -> Result<(), String> {
        self.conn()
            .execute("DELETE FROM recognition_request WHERE content_id = ?1", [content])
            .map_err(sql_err)?;
        self.engine_state().forget_recognition_tries(&[content]);
        Ok(())
    }

    /// The document to recognize next: the first one a person asked for,
    /// or, while the automatic reading is on, the next one waiting that did
    /// not fail and can be read now. While the engine of this computer can
    /// read, the ones waiting are its own (`next_local_recognition`).
    pub fn next_recognition(&self) -> Result<Option<i64>, String> {
        if let Some(id) = requested_recognitions(&self.conn())?.first().copied() {
            return Ok(Some(id));
        }
        if super::services::local_recognition_engine().is_some() || !self.background_reading()? {
            return Ok(None);
        }
        self.next_waiting()
    }

    /// The document the engine of this computer reads next (D-558): the
    /// next one waiting that did not fail, can be read now and nobody asked
    /// for, the automatic reading on or off. It asks no model in the cloud.
    pub fn next_local_recognition(&self) -> Result<Option<i64>, String> {
        self.next_waiting()
    }

    fn next_waiting(&self) -> Result<Option<i64>, String> {
        self.conn()
            .query_row(
                &format!(
                    "SELECT c.id FROM content c WHERE c.state = 'needs-recognition' AND c.error IS NULL AND {}
                     AND NOT EXISTS (SELECT 1 FROM recognition_request r WHERE r.content_id = c.id)
                     ORDER BY c.id LIMIT 1",
                    reachable_sql()
                ),
                [],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)
    }

    /// Recognizes the next document (`next_recognition`) with `recognizer`,
    /// telling the engine's state the document and the page it is at. When
    /// the provider fails, the document keeps waiting and the stage shows
    /// the error with when it tries again (`retry_in`).
    pub fn recognition_step(&self, recognizer: &dyn TextRecognizer, retry_in: Duration) -> Result<RecognitionStep, String> {
        match self.next_recognition()? {
            Some(content) => self.recognition_step_of(content, recognizer, retry_in),
            None => Ok(RecognitionStep::Idle),
        }
    }

    /// The same, for the next document of the engine of this computer
    /// (`next_local_recognition`).
    pub fn local_recognition_step(&self, recognizer: &dyn TextRecognizer, retry_in: Duration) -> Result<RecognitionStep, String> {
        match self.next_local_recognition()? {
            Some(content) => self.recognition_step_of(content, recognizer, retry_in),
            None => Ok(RecognitionStep::Idle),
        }
    }

    fn recognition_step_of(&self, content: i64, recognizer: &dyn TextRecognizer, retry_in: Duration) -> Result<RecognitionStep, String> {
        let state = self.engine_state();
        state.recognizing(Some(content));
        let result = self.recognize_content(content, recognizer);
        state.recognizing(None);
        match result {
            Ok(true) => {
                self.recognition_ended(content)?;
                state.recognized();
                Ok(RecognitionStep::Recognized(content))
            }
            Ok(false) => {
                self.recognition_ended(content)?;
                Ok(RecognitionStep::Skipped(content))
            }
            Err(error) if stops_the_run(&error) && state.recognition_tried(content) < MAX_PROVIDER_TRIES => {
                state.failed(Some("recognition"), &error, Some(retry_in));
                Ok(RecognitionStep::Stopped(error))
            }
            Err(error) => {
                self.recognition_ended(content)?;
                // The failure is of the document: a failure of the provider
                // before it is behind.
                state.forget_error_of("recognition");
                let marked = self
                    .conn()
                    .execute(
                        "UPDATE content SET error = ?1 WHERE id = ?2 AND state = 'needs-recognition'",
                        params![error, content],
                    )
                    .map_err(sql_err)?;
                // A document read again with another engine keeps its text:
                // the failure shows on the stage.
                if marked == 0 {
                    state.failed(Some("recognition"), &error, None);
                }
                Ok(RecognitionStep::Failed(content))
            }
        }
    }

    /// Recognizes the text of one document and keeps it, for every copy of
    /// its file. False when there is nothing to do: its text is not only
    /// pixels, or this engine already read it.
    pub fn recognize_content(&self, content_id: i64, recognizer: &dyn TextRecognizer) -> Result<bool, String> {
        let engine = recognizer.engine_id();
        let file: Option<(String, String, String, String)> = {
            let conn = self.conn();
            let found: Option<(String, Option<String>)> = conn
                .query_row(
                    "SELECT c.state, d.engine FROM content c
                     LEFT JOIN derivative d ON d.content_id = c.id AND d.kind = 'text' WHERE c.id = ?1",
                    [content_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(sql_err)?;
            match found {
                Some((state, text_engine)) if needs_engine(&state, text_engine.as_deref(), &engine) => {}
                Some((state, _)) if has_unread_scans(&conn, &self.recognized, &state, content_id)? => {}
                _ => return Ok(false),
            }
            conn.query_row(
                &format!(
                    "SELECT f.hash, o.location, l.rel_path, l.ext FROM file f
                     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done'
                     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                     WHERE f.content_id = ?1 AND NOT {}
                     ORDER BY l.id LIMIT 1",
                    super::excluded_sql!()
                ),
                [content_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(sql_err)?
        };
        let Some((hash, root, rel_path, ext)) = file else {
            return Err(UNREACHABLE.to_string());
        };
        let path = LocalFolderProvider::new(root).local_path(&rel_path).ok_or_else(|| UNREACHABLE.to_string())?;
        if !path.is_file() {
            return Err(UNREACHABLE.to_string());
        }
        let store = RecognizedStore::at(self.recognized.join(&hash)).for_engine(&engine);
        let state = self.engine_state();
        let on_page = |page: usize, total: usize| state.recognizing_page(page, total);
        let ext = ext.to_lowercase();
        // The text, and each page read with the engine that read it.
        let (text, pages) = run_guarded("docbase_recognize_text", || {
            if ext == "pdf" {
                let extracted = crate::commands::fs::extract_text_without_recognition(&path)?.unwrap_or_default();
                let done = recognition::complete_pdf_text_kept(&path, &extracted, &store, recognizer, &on_page)?;
                let pages = done.pages.into_iter().map(|(page, read)| (page, read.engine_id, read.text)).collect();
                Ok((done.text, pages))
            } else if is_recognizable_image(&ext) {
                on_page(1, 1);
                let text = recognition::recognize_image_source(&path, &store, recognizer)?;
                Ok((text.clone(), vec![(1, engine.clone(), text)]))
            } else {
                Err(format!("A .{ext} file is not an image whose text can be recognized"))
            }
        })?;
        let pages: Vec<(u32, &str, &str)> = pages.iter().map(|(page, engine, body)| (*page, engine.as_str(), body.as_str())).collect();
        self.store_recognized_pages(&hash, &text, &engine, &pages)
    }

    /// Counts the pages each document waiting for recognition needs read,
    /// up to `limit` documents not counted yet, so «En curso» can say the
    /// size of the work before it starts (each page is a question to the
    /// model). Kept in memory. Returns how many were counted.
    pub fn count_scan_pages(&self, limit: usize) -> Result<usize, String> {
        let known = self.engine_state().snapshot().scan_pages;
        let jobs: Vec<(i64, PathBuf, String)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(&format!(
                    "SELECT c.id, o.location, l.rel_path, l.ext FROM content c
                     JOIN file f ON f.content_id = c.id
                     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done'
                     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                     WHERE c.state = 'needs-recognition' AND NOT {}
                     ORDER BY c.id, l.id",
                    super::excluded_sql!()
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))
                })
                .map_err(sql_err)?;
            let mut jobs: Vec<(i64, PathBuf, String)> = Vec::new();
            for row in rows {
                let (content, root, rel_path, ext) = row.map_err(sql_err)?;
                if known.contains_key(&content) || jobs.iter().any(|(c, ..)| *c == content) {
                    continue;
                }
                if let Some(path) = LocalFolderProvider::new(root).local_path(&rel_path) {
                    jobs.push((content, path, ext.to_lowercase()));
                }
                if jobs.len() >= limit {
                    break;
                }
            }
            jobs
        };
        for (content, path, ext) in &jobs {
            // A PDF that cannot be opened counts no page: its recognition
            // will say why.
            let pages = if ext == "pdf" {
                run_guarded("docbase_scan_pages", || textless_pages(path)).map(|pages| pages.len()).unwrap_or(0)
            } else {
                1
            };
            self.engine_state().scan_pages(*content, pages);
        }
        Ok(jobs.len())
    }
}

/// Whether a document already read has scanned pages its ficha of shape
/// says need reading and were not read (P-320): read only when asked.
fn has_unread_scans(conn: &Connection, recognized: &std::path::Path, state: &str, content: i64) -> Result<bool, String> {
    Ok(state == "processed" && !super::page_shapes::unread_of(conn, recognized, content)?.is_empty())
}

/// Whether a document in `state`, whose text `text_engine` produced, has
/// something for `engine` to read: its text is only pixels, or another
/// engine recognized it.
fn needs_engine(state: &str, text_engine: Option<&str>, engine: &str) -> bool {
    state == "needs-recognition" || (state == "processed" && text_engine.is_some_and(|e| e != EXTRACTION_ENGINE && e != engine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::knowledge::tests::{is_knowledge, type_answer, FakeModel};
    use crate::docbase::progress::Scope;
    use crate::docbase::query::DocumentFilter;
    use crate::docbase::testing::TempDir;
    use crate::docbase::Visible;
    use crate::source_volume::asking::AskConfig;
    use crate::source_volume::recognition::FakeRecognizer;

    // Invented texts (R-24).
    const PAGE_ONE: &str = "CONTRATO DE LOCACIÓN\n\nEntre Ejemplo Uno S.A., CUIT 30-11111111-1, y Persona Tres, DNI 33.333.333, \
en adelante el locatario, se celebra el presente contrato de locación por el inmueble del domicilio inventado.";
    const PAGE_TWO: &str = "El canon mensual es de $ 100.000 a partir del 1 de marzo, pagadero del uno al diez de cada mes \
en el domicilio de la locadora, y el plazo es de dos años desde la entrega de las llaves.";
    const NOTE: &str = "NOTA DE PRUEBA\n\nPor la presente se deja constancia de que Persona Tres retiró los papeles \
del inmueble del domicilio inventado el día del vencimiento, sin observaciones de ninguna de las partes.";
    const RECEIPT: &str = "RECIBO DE PRUEBA\n\nRecibí de Persona Tres la suma de pesos cien mil en concepto de canon \
del mes de abril por el inmueble del domicilio inventado, sin otra deuda pendiente a la fecha.";
    const OTHER_NOTE: &str = "OTRA NOTA DE PRUEBA\n\nSe deja constancia de que la llave del portón quedó en la administración \
del edificio inventado hasta que alguna de las partes la retire, con aviso de un día.";
    const SLOW: Duration = Duration::from_secs(600);

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/source_volume/fixtures").join(name)).unwrap()
    }

    /// A PDF of two pages: the first with its own text, the second a scan
    /// without a text layer (an image covers it).
    fn mixed_pdf() -> Vec<u8> {
        use crate::source_volume::page_shape::test_pdf::{build, page};
        let lines = [
            "Primera pagina con texto propio del documento de prueba,",
            "escrita para que la extraccion la lea entera y no la tome",
            "por un escaneo: tiene bastante mas de ochenta letras y",
            "treinta palabras distintas entre si, en cuatro renglones.",
        ];
        build(&[page(&lines, 0.0), page(&[], 1.0)])
    }

    fn settle(base: &DocBase, origin: &str) {
        base.scan_origin(origin).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
        while base.extract_pending(50).unwrap() > 0 {}
        base.purge_orphans().unwrap();
    }

    /// A base over a folder with these files.
    fn base_of(files: &[(&str, Vec<u8>)]) -> (TempDir, DocBase, String) {
        let dir = TempDir::new();
        for (name, bytes) in files {
            dir.write(&format!("o/{name}"), bytes);
        }
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        (dir, base, origin.id)
    }

    fn engine(model: &str) -> RecognitionConfig {
        RecognitionConfig { engine: "codex-cli".into(), model: model.into() }
    }

    /// The recognizer of the engine chosen as `model`, with these answers.
    fn fake(model: &str, answers: Vec<Result<&str, &str>>) -> FakeRecognizer {
        FakeRecognizer::of(&format!("codex-cli:{model}"), answers)
    }

    fn content_of(base: &DocBase, name: &str) -> i64 {
        base.conn()
            .query_row(
                "SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = ?1",
                [name],
                |row| row.get(0),
            )
            .unwrap()
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

    fn text_of(base: &DocBase, name: &str) -> (String, String) {
        base.conn()
            .query_row(
                "SELECT d.body, d.engine FROM location l JOIN file f ON f.hash = l.file_hash
                 JOIN derivative d ON d.content_id = f.content_id AND d.kind = 'text' WHERE l.name = ?1",
                [name],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }

    /// Recognizes until there is nothing left; returns the steps.
    fn run(base: &DocBase, recognizer: &dyn TextRecognizer) -> Vec<RecognitionStep> {
        let mut steps = Vec::new();
        loop {
            match base.recognition_step(recognizer, SLOW).unwrap() {
                RecognitionStep::Idle => return steps,
                step @ RecognitionStep::Stopped(_) => {
                    steps.push(step);
                    return steps;
                }
                step => steps.push(step),
            }
        }
    }

    fn turn_reading_on(base: &DocBase) {
        base.set_ask_engine(Some(&AskConfig { engine: "codex-cli".into(), model: "m".into(), ..Default::default() })).unwrap();
        base.set_background_reading(true).unwrap();
    }

    #[test]
    fn scans_photos_and_the_blank_pages_of_a_mixed_pdf_are_recognized_and_nothing_else() {
        let (_dir, base, _) = base_of(&[
            ("escaneo.pdf", fixture("escaneo-2-paginas.pdf")),
            ("foto.png", fixture("pagina-prueba.png")),
            ("mixto.pdf", mixed_pdf()),
            ("nota.txt", NOTE.as_bytes().to_vec()),
        ]);
        assert_eq!(state_of(&base, "mixto.pdf"), "needs-recognition");
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        turn_reading_on(&base);

        // Two pages of the scan, the photo, and only the blank page of the
        // mixed PDF: four pages, in whatever order the documents come.
        let recognizer = fake("m", vec![Ok(PAGE_ONE), Ok(PAGE_TWO), Ok(RECEIPT), Ok(OTHER_NOTE)]);
        let steps = run(&base, &recognizer);
        assert_eq!(steps.len(), 3, "{steps:?}");
        assert!(steps.iter().all(|step| matches!(step, RecognitionStep::Recognized(_))), "{steps:?}");
        assert_eq!(recognizer.calls(), 4);
        for name in ["escaneo.pdf", "foto.png", "mixto.pdf", "nota.txt"] {
            assert_eq!(state_of(&base, name), "processed", "{name}");
        }
        let marker = recognition::marker("codex-cli:m");
        let (text, by) = text_of(&base, "escaneo.pdf");
        assert_eq!(by, "codex-cli:m");
        assert_eq!(text.matches(&marker).count(), 2, "{text}");
        assert_eq!(text_of(&base, "foto.png").0.matches(&marker).count(), 1);
        // The mixed PDF keeps its own text and gets the recognized page.
        let (mixed, _) = text_of(&base, "mixto.pdf");
        assert!(mixed.contains("Primera pagina con texto propio"), "{mixed}");
        assert_eq!(mixed.matches(&marker).count(), 1, "{mixed}");
        assert_eq!(text_of(&base, "nota.txt").1, EXTRACTION_ENGINE);
    }

    #[test]
    fn a_document_is_recognized_once_per_engine_and_its_copies_are_not_read_again() {
        let image = fixture("pagina-prueba.png");
        let (_dir, base, _) = base_of(&[("carta.png", image.clone()), ("copia/carta escaneada.png", image)]);
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        turn_reading_on(&base);
        let first = fake("m", vec![Ok(NOTE)]);
        assert_eq!(run(&base, &first).len(), 1);
        assert_eq!(first.calls(), 1);
        assert_eq!(state_of(&base, "carta escaneada.png"), "processed");

        // Nothing is waiting: asking again with the same engine reads nothing.
        let content = content_of(&base, "carta.png");
        assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![content])).unwrap(), 0);
        let again = fake("m", vec![]);
        assert!(run(&base, &again).is_empty());
        assert_eq!(again.calls(), 0);

        // Another engine does not read again by itself...
        base.set_recognition_engine(Some(&engine("otro"))).unwrap();
        let other = fake("otro", vec![Ok(PAGE_ONE)]);
        assert!(run(&base, &other).is_empty());
        assert_eq!(text_of(&base, "carta.png").1, "codex-cli:m");
        // ...only when a person asks, and its text replaces the other.
        assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![content])).unwrap(), 1);
        assert_eq!(run(&base, &other), vec![RecognitionStep::Recognized(content)]);
        assert_eq!(other.calls(), 1);
        let (text, by) = text_of(&base, "carta escaneada.png");
        assert_eq!(by, "codex-cli:otro");
        assert!(text.contains("CONTRATO DE LOCACIÓN"));
    }

    #[test]
    fn with_the_reading_off_only_what_a_person_asks_for_is_recognized() {
        let (_dir, base, _) = base_of(&[("uno.png", fixture("pagina-prueba.png")), ("dos.pdf", fixture("escaneo-2-paginas.pdf"))]);
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        let recognizer = fake("m", vec![Ok(NOTE)]);
        assert!(run(&base, &recognizer).is_empty());
        assert_eq!(recognizer.calls(), 0);

        let one = content_of(&base, "uno.png");
        assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![one])).unwrap(), 1);
        assert_eq!(run(&base, &recognizer), vec![RecognitionStep::Recognized(one)]);
        assert_eq!(state_of(&base, "uno.png"), "processed");
        assert_eq!(state_of(&base, "dos.pdf"), "needs-recognition");

        // «For all»: the rest.
        assert_eq!(base.request_recognition(&RecognitionRequest::AllWaiting).unwrap(), 1);
        let rest = fake("m", vec![Ok(PAGE_ONE), Ok(PAGE_TWO)]);
        assert_eq!(run(&base, &rest).len(), 1);
        assert_eq!(state_of(&base, "dos.pdf"), "processed");
    }

    #[test]
    fn the_engine_of_this_computer_reads_the_new_scans_with_the_reading_off_and_leaves_the_asked_ones() {
        let (_dir, base, _) = base_of(&[("uno.png", fixture("pagina-prueba.png")), ("dos.pdf", fixture("escaneo-2-paginas.pdf"))]);
        assert!(!base.background_reading().unwrap());
        // A person asks for one, with the engine chosen in Ajustes: that one
        // is not the local engine's.
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        let two = content_of(&base, "dos.pdf");
        assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![two])).unwrap(), 1);

        let local = FakeRecognizer::of("rapidocr-onnx:ppocrv6-small", vec![Ok(NOTE)]);
        let one = content_of(&base, "uno.png");
        assert_eq!(base.local_recognition_step(&local, SLOW).unwrap(), RecognitionStep::Recognized(one));
        assert_eq!(base.local_recognition_step(&local, SLOW).unwrap(), RecognitionStep::Idle);
        assert_eq!(local.calls(), 1);
        assert_eq!(text_of(&base, "uno.png").1, "rapidocr-onnx:ppocrv6-small");
        assert_eq!(state_of(&base, "dos.pdf"), "needs-recognition");
        assert_eq!(base.next_recognition().unwrap(), Some(two));

        // A document read by the local engine is not read again by itself.
        let chosen = fake("m", vec![Ok(PAGE_ONE), Ok(PAGE_TWO)]);
        assert_eq!(run(&base, &chosen), vec![RecognitionStep::Recognized(two)]);
        assert_eq!(text_of(&base, "uno.png").1, "rapidocr-onnx:ppocrv6-small");
    }

    #[test]
    fn without_an_engine_nothing_is_asked_for() {
        let (_dir, base, _) = base_of(&[("uno.png", fixture("pagina-prueba.png"))]);
        assert!(base.request_recognition(&RecognitionRequest::AllWaiting).is_err());
        let progress = base.progress(&Scope::default()).unwrap();
        let stage = progress.stages.iter().find(|s| s.id == "recognition").unwrap();
        assert_eq!((stage.state, stage.pending), ("paused", 1));
        assert!(progress.recognition_engine.is_none());
        assert_eq!(progress.paused, 1);
    }

    #[test]
    fn a_failure_of_the_engine_marks_the_document_and_one_of_the_provider_waits() {
        let (_dir, base, _) = base_of(&[("uno.png", fixture("pagina-prueba.png")), ("dos.pdf", fixture("escaneo-2-paginas.pdf"))]);
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        turn_reading_on(&base);
        let one = content_of(&base, "uno.png");
        let two = content_of(&base, "dos.pdf");
        // In this order: asked for, they go before the others.
        assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![one, two])).unwrap(), 2);

        // The provider at its limit: nothing is marked, the stage says so.
        let limited = fake("m", vec![Err("You've hit your usage limit")]);
        assert!(matches!(run(&base, &limited)[..], [RecognitionStep::Stopped(_)]));
        let progress = base.progress(&Scope::default()).unwrap();
        let stage = progress.stages.iter().find(|s| s.id == "recognition").unwrap();
        assert_eq!((stage.state, stage.failed, stage.pending), ("error", 0, 2));
        assert_eq!(stage.error.as_ref().unwrap().kind, "limit");
        assert!(stage.error.as_ref().unwrap().retry_at.is_some());

        // Any other failure marks the document with its reason; the second
        // page of the scan fails, and the first one is kept.
        let broken = fake("m", vec![Err("la imagen no se pudo leer"), Ok(PAGE_ONE), Err("respuesta rara")]);
        let steps = run(&base, &broken);
        assert_eq!(steps, vec![RecognitionStep::Failed(one), RecognitionStep::Failed(two)]);
        assert_eq!(state_of(&base, "uno.png"), "needs-recognition");
        let progress = base.progress(&Scope::default()).unwrap();
        let stage = progress.stages.iter().find(|s| s.id == "recognition").unwrap();
        assert_eq!((stage.state, stage.failed, stage.pending), ("error", 2, 0));
        assert!(stage.error.is_none(), "an answer leaves the error of the provider behind");
        let items = base.progress_items("recognition", &Scope::default(), 10).unwrap();
        assert_eq!(items.items[0].status, "failed");
        assert!(items.items.iter().any(|i| i.detail.as_deref().is_some_and(|d| d.contains("la imagen no se pudo leer"))));
        // Not tried again by itself.
        assert!(run(&base, &fake("m", vec![])).is_empty());

        // «Reintentar»: the photo, and only the page of the scan that failed.
        assert_eq!(base.request_recognition(&RecognitionRequest::Failed).unwrap(), 2);
        let retry = fake("m", vec![Ok(NOTE), Ok(PAGE_TWO)]);
        let steps = run(&base, &retry);
        assert!(steps.contains(&RecognitionStep::Recognized(one)) && steps.contains(&RecognitionStep::Recognized(two)), "{steps:?}");
        assert_eq!(retry.calls(), 2);
        assert!(text_of(&base, "dos.pdf").0.contains("CONTRATO DE LOCACIÓN"));
    }

    #[test]
    fn what_a_person_asked_to_recognize_survives_closing_the_app() {
        let dir = TempDir::new();
        dir.write("o/uno.png", &fixture("pagina-prueba.png"));
        dir.write("o/dos.pdf", &fixture("escaneo-2-paginas.pdf"));
        let path = dir.path().join("base").join("private.sqlite3");
        let (one, two) = {
            let base = DocBase::open(&path).unwrap();
            let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
            settle(&base, &origin.id);
            base.set_recognition_engine(Some(&engine("m"))).unwrap();
            let (one, two) = (content_of(&base, "uno.png"), content_of(&base, "dos.pdf"));
            // Asked in this order, the reading off; the app closes before.
            assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![two, one])).unwrap(), 2);
            (one, two)
        };
        let base = DocBase::open(&path).unwrap();
        // Opened again, the requests are there, in their order, and the
        // stage says they were asked for.
        assert_eq!(base.next_recognition().unwrap(), Some(two));
        let stage = base.progress(&Scope::default()).unwrap().stages.into_iter().find(|s| s.id == "recognition").unwrap();
        assert_eq!((stage.state, stage.extra["requested"]), ("running", 2));
        // Asked again, nothing is repeated.
        assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![one])).unwrap(), 1);
        assert_eq!(requested_recognitions(&base.conn()).unwrap(), vec![two, one]);
        // Read, they are no longer asked for; with the reading off nothing else is read.
        let recognizer = fake("m", vec![Ok(PAGE_ONE), Ok(PAGE_TWO), Ok(NOTE)]);
        assert_eq!(run(&base, &recognizer), vec![RecognitionStep::Recognized(two), RecognitionStep::Recognized(one)]);
        assert!(requested_recognitions(&base.conn()).unwrap().is_empty());
        assert_eq!(base.next_recognition().unwrap(), None);
    }

    #[test]
    fn the_provider_failing_again_and_again_on_a_document_marks_it() {
        let (_dir, base, _) = base_of(&[("uno.png", fixture("pagina-prueba.png"))]);
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        turn_reading_on(&base);
        let timing_out = fake("m", vec![Err("Codex did not answer within 300 seconds"); MAX_PROVIDER_TRIES]);
        for _ in 1..MAX_PROVIDER_TRIES {
            assert!(matches!(run(&base, &timing_out)[..], [RecognitionStep::Stopped(_)]));
        }
        assert!(matches!(run(&base, &timing_out)[..], [RecognitionStep::Failed(_)]));
        let error: Option<String> = base.conn().query_row("SELECT error FROM content", [], |row| row.get(0)).unwrap();
        assert!(error.unwrap().contains("did not answer"));
    }

    #[test]
    fn after_its_recognition_a_scan_follows_the_usual_path() {
        let (_dir, base, _) = base_of(&[("escaneo.pdf", fixture("escaneo-2-paginas.pdf"))]);
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        turn_reading_on(&base);
        let asker = FakeModel::new("codex-cli:m", |prompt| {
            if is_knowledge(prompt) {
                serde_json::json!({ "menciones": [], "afirmaciones": [], "importes": [] }).to_string()
            } else {
                type_answer("contrato")
            }
        });
        // Before: nothing to compare, type or read.
        while base.similar_step(10).unwrap() > 0 {}
        assert_eq!(base.types_run(&asker, 10).unwrap().asked, 0);

        run(&base, &fake("m", vec![Ok(PAGE_ONE), Ok(PAGE_TWO)]));
        while base.similar_step(10).unwrap() > 0 {}
        assert_eq!(base.types_run(&asker, 10).unwrap().typed, 1);
        base.knowledge_run(&asker, 10).unwrap();
        let progress = base.progress(&Scope::default()).unwrap();
        let figures = |id: &str| {
            let stage = progress.stages.iter().find(|s| s.id == id).unwrap();
            (stage.state, stage.done, stage.total)
        };
        assert_eq!(figures("recognition"), ("done", 1, 1));
        assert_eq!(figures("types"), ("done", 1, 1));
        assert_eq!(figures("reading"), ("done", 1, 1));
        // Found by the words of its recognized text.
        let filter = DocumentFilter { query: Some("canon mensual".to_string()), ..Default::default() };
        assert_eq!(base.documents(&Visible::All, &filter).unwrap().rows.len(), 1);
    }

    #[test]
    fn a_scan_is_read_by_micelya_core_page_by_page_and_nothing_is_left_in_core() {
        use crate::micelya_core::fake::{FakeCore, Outcome};
        use crate::source_volume::recognition_core::CoreRecognizer;
        use crate::source_volume::recognized_store::Part;

        let (_dir, base, _) = base_of(&[("escaneo.pdf", fixture("escaneo-2-paginas.pdf"))]);
        base.set_recognition_engine(Some(&RecognitionConfig { engine: "core".into(), model: "textract-1.0".into() })).unwrap();
        turn_reading_on(&base);
        let core = FakeCore::start();
        core.with(|state| state.outcome = Outcome::Read(vec![vec![("CONTRATO", 0.99), ("DE", 0.99), ("LOCACIÓN", 0.71)]]));
        let recognizer = CoreRecognizer::with_connection("textract-1.0", core.connection(), FakeCore::timing());
        let content = content_of(&base, "escaneo.pdf");
        assert_eq!(run(&base, &recognizer), vec![RecognitionStep::Recognized(content)]);

        let (text, by) = text_of(&base, "escaneo.pdf");
        assert_eq!(by, "core:textract-1.0");
        assert_eq!(text.matches(&recognition::marker("core:textract-1.0")).count(), 2, "{text}");
        assert!(text.contains("CONTRATO DE LOCACIÓN"));
        // One reading per page, each named by its page (both at once, in
        // any order), every one deleted.
        let mut names: Vec<String> = core.with(|state| state.readings.iter().map(|r| r.filename.clone()).collect());
        names.sort();
        assert_eq!(names, ["pagina-1.jpg", "pagina-2.jpg"]);
        assert!(core.with(|state| state.kept().is_empty()));
        // The doubtful words are kept beside the text, which they do not change.
        let hash: String = base
            .conn()
            .query_row("SELECT f.hash FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.name = 'escaneo.pdf'", [], |row| row.get(0))
            .unwrap();
        let page = RecognizedStore::at(base.recognized.join(&hash)).get(Part::Page(1)).unwrap();
        assert_eq!(page.doubtful.len(), 1);
        assert_eq!((page.doubtful[0].word.as_str(), page.doubtful[0].percent), ("LOCACIÓN", 71));
        assert_eq!(page.read_by.as_deref(), Some("textract-1.0"));

        // Core cannot be reached: the scan keeps waiting, the stage says so.
        let (_dir, other, _) = base_of(&[("otro.png", fixture("pagina-prueba.png"))]);
        other.set_recognition_engine(Some(&RecognitionConfig { engine: "core".into(), model: String::new() })).unwrap();
        turn_reading_on(&other);
        let mut gone = core.connection();
        gone.issuer = "http://127.0.0.1:9/realms/prueba".into();
        let unreachable = CoreRecognizer::with_connection("", gone, FakeCore::timing());
        assert!(matches!(run(&other, &unreachable)[..], [RecognitionStep::Stopped(_)]));
        assert_eq!(state_of(&other, "otro.png"), "needs-recognition");
        let stage = other.progress(&Scope::default()).unwrap().stages.into_iter().find(|s| s.id == "recognition").unwrap();
        assert_eq!(stage.error.as_ref().unwrap().kind, "connection");
    }

    #[test]
    fn the_stage_tells_the_pages_that_wait_and_the_page_being_read() {
        let (_dir, base, _) = base_of(&[("uno.png", fixture("pagina-prueba.png")), ("dos.pdf", fixture("escaneo-2-paginas.pdf"))]);
        let stage = |base: &DocBase| base.progress(&Scope::default()).unwrap().stages.into_iter().find(|s| s.id == "recognition").unwrap();
        assert_eq!((stage(&base).extra["pages"], stage(&base).extra["uncounted"]), (0, 2));
        while base.count_scan_pages(1).unwrap() > 0 {}
        assert_eq!((stage(&base).extra["pages"], stage(&base).extra["uncounted"]), (3, 0));
        let items = base.progress_items("recognition", &Scope::default(), 10).unwrap();
        let pdf = items.items.iter().find(|i| i.name == "dos.pdf").unwrap();
        assert_eq!(pdf.pages, Some(2));

        // With the reading off it is paused; with an engine and the reading
        // on, it runs, and names the document and the page.
        base.set_recognition_engine(Some(&engine("m"))).unwrap();
        assert_eq!(stage(&base).state, "paused");
        turn_reading_on(&base);
        assert_eq!(stage(&base).state, "running");
        let two = content_of(&base, "dos.pdf");
        base.engine_state().recognizing(Some(two));
        base.engine_state().recognizing_page(2, 2);
        let current = stage(&base).current.unwrap();
        assert_eq!((current.name.as_str(), current.page, current.pages), ("dos.pdf", Some(2), Some(2)));
        let progress = base.progress(&Scope::default()).unwrap();
        assert_eq!(progress.recognition_engine.as_deref(), Some("codex-cli:m"));
        assert_eq!(progress.in_process, 2);
    }
}
