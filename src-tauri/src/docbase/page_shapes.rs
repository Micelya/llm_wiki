//! The ficha of shape of each page of the PDFs of the base (P-320), kept
//! beside the text of each document (`derivative`, kind `page-shapes`).
//!
//! A new reading decides with it which pages are recognized
//! (`source_volume::page_shape`). What was read before it is not read
//! again by itself: its ficha is computed in the background (pdfium only,
//! no model), and the scanned pages it finds unread are counted for
//! «Atención», where a person asks for them to be read.
//!
//! A page counts as read when the base keeps its recognition
//! (`recognized/<file hash>/page-N.json`), or when its document was
//! recognized and the test used before the ficha sent that page to
//! recognition (some of those readings left no file per page).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rusqlite::types::Value;
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};

use crate::panic_guard::run_guarded;
use crate::source_volume::page_shape::{profile_pdf, PdfShapes, SHAPES_ENGINE};
use crate::source_volume::LocalFolderProvider;
use crate::source_volume::SourceProvider;

use super::text::EXTRACTION_ENGINE;
use super::{now_ms, sql_err, DocBase};

pub const SHAPES_KIND: &str = "page-shapes";

/// The scanned pages of what was already read that are still unread.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UnreadScans {
    pub pages: i64,
    /// The documents (contents) that have them.
    pub documents: Vec<i64>,
}

impl DocBase {
    /// Computes the ficha of up to `limit` PDF documents that do not have
    /// one of this version. Returns how many were attended.
    pub fn profile_pending(&self, limit: usize) -> Result<usize, String> {
        let jobs: Vec<(i64, PathBuf)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(&format!(
                    "SELECT c.id, o.location, min(l.rel_path) FROM content c
                     JOIN file f ON f.content_id = c.id
                     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done' AND lower(l.ext) = 'pdf'
                     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                     WHERE c.state IN ('processed', 'needs-recognition') AND NOT {}
                       AND NOT EXISTS (SELECT 1 FROM derivative d WHERE d.content_id = c.id AND d.kind = ?1 AND d.engine = ?2)
                     GROUP BY c.id ORDER BY c.id LIMIT ?3",
                    super::excluded_sql!()
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params![SHAPES_KIND, SHAPES_ENGINE, limit as i64], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(sql_err)?;
            let mut jobs = Vec::new();
            for row in rows {
                let (content, root, rel_path) = row.map_err(sql_err)?;
                jobs.push((content, LocalFolderProvider::new(root).local_path(&rel_path).unwrap_or_default()));
            }
            jobs
        };
        for (content, path) in &jobs {
            // A PDF pdfium cannot open keeps an empty ficha: it is not tried
            // again until the next version of the ficha.
            let shapes = run_guarded("docbase_page_shapes", || profile_pdf(path)).unwrap_or_default();
            let body = serde_json::to_string(&shapes).map_err(|e| e.to_string())?;
            self.conn()
                .execute(
                    "INSERT INTO derivative (content_id, kind, engine, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT (content_id, kind) DO UPDATE SET engine = excluded.engine, body = excluded.body, created_at = excluded.created_at",
                    params![content, SHAPES_KIND, SHAPES_ENGINE, body, now_ms()],
                )
                .map_err(sql_err)?;
        }
        Ok(jobs.len())
    }
}

/// The pages of a processed document its ficha sends to recognition and
/// that were not read.
pub(super) fn unread_of(conn: &Connection, recognized: &Path, content: i64) -> Result<Vec<u32>, String> {
    let found: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT d.body, t.engine FROM content c
             JOIN derivative d ON d.content_id = c.id AND d.kind = ?2 AND d.engine = ?3
             LEFT JOIN derivative t ON t.content_id = c.id AND t.kind = 'text'
             WHERE c.id = ?1 AND c.state = 'processed'",
            params![content, SHAPES_KIND, SHAPES_ENGINE],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((body, text_engine)) = found else {
        return Ok(Vec::new());
    };
    let Ok(shapes) = serde_json::from_str::<PdfShapes>(&body) else {
        return Ok(Vec::new());
    };
    let wanted = shapes.to_read();
    if wanted.is_empty() {
        return Ok(Vec::new());
    }
    let mut read = recognized_pages(conn, recognized, content)?;
    if text_engine.as_deref().is_some_and(|engine| engine != EXTRACTION_ENGINE) {
        read.extend(shapes.read_by_the_earlier_test());
    }
    Ok(wanted.into_iter().filter(|page| !read.contains(page)).collect())
}

/// The pages of any file of this content whose recognition the base keeps.
fn recognized_pages(conn: &Connection, recognized: &Path, content: i64) -> Result<BTreeSet<u32>, String> {
    let mut statement = conn.prepare("SELECT hash FROM file WHERE content_id = ?1").map_err(sql_err)?;
    let hashes = statement
        .query_map([content], |row| row.get::<_, String>(0))
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    let mut pages = BTreeSet::new();
    for hash in hashes {
        let Ok(entries) = std::fs::read_dir(recognized.join(&hash)) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(page) = name.strip_prefix("page-").and_then(|rest| rest.strip_suffix(".json")).and_then(|n| n.parse().ok()) {
                pages.insert(page);
            }
        }
    }
    Ok(pages)
}

/// The unread scanned pages of the processed documents among the rows of
/// `rows_sql` (with its `bound` values).
pub(crate) fn unread_scans(conn: &Connection, recognized: &Path, rows_sql: &str, bound: &[Value]) -> Result<UnreadScans, String> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT DISTINCT c.id FROM content c
             JOIN derivative d ON d.content_id = c.id AND d.kind = '{SHAPES_KIND}'
             WHERE c.state = 'processed'
               AND c.id IN (SELECT content_id FROM ({rows_sql}) WHERE content_id IS NOT NULL)
             ORDER BY c.id"
        ))
        .map_err(sql_err)?;
    let ids = statement
        .query_map(params_from_iter(bound.iter()), |row| row.get::<_, i64>(0))
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    let mut unread = UnreadScans::default();
    for id in ids {
        let pages = unread_of(conn, recognized, id)?.len() as i64;
        if pages > 0 {
            unread.pages += pages;
            unread.documents.push(id);
        }
    }
    Ok(unread)
}

#[cfg(test)]
mod tests {
    use crate::docbase::query::DocumentFilter;
    use crate::docbase::recognize::RecognitionRequest;
    use crate::docbase::testing::TempDir;
    use crate::docbase::{DocBase, Visible};
    use crate::source_volume::page_shape::test_pdf::{build, page};
    use crate::source_volume::recognition::{FakeRecognizer, RecognitionConfig};

    const PROSE: [&str; 3] = [
        "Primera pagina con texto propio del documento de prueba, escrita para que",
        "la extraccion la lea entera y no la tome por un escaneo: tiene bastante",
        "mas de ochenta letras y treinta palabras distintas entre si, repartidas.",
    ];
    fn footer(n: usize) -> String {
        format!("Sistema Inventado de Expedientes - firmado digitalmente el {n:02}/03/2026 - hoja {n} de 3 - codigo verificador")
    }

    fn settle(base: &DocBase, origin: &str) {
        base.scan_origin(origin).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
        while base.extract_pending(50).unwrap() > 0 {}
        base.purge_orphans().unwrap();
    }

    #[test]
    fn a_scan_hidden_by_a_footer_waits_for_recognition_only_in_a_new_reading() {
        let f: Vec<String> = (1..=3).map(footer).collect();
        let pdf = build(&[
            page(&[PROSE[0], PROSE[1], PROSE[2], &f[0]], 0.0),
            page(&[&f[1]], 1.0),
            page(&[PROSE[0], PROSE[1], PROSE[2], &f[2]], 0.0),
        ]);
        let dir = TempDir::new();
        dir.write("o/oficio.pdf", &pdf);
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let rows = base.documents(&Visible::All, &DocumentFilter::default()).unwrap().rows;
        assert_eq!(rows[0].state, "needs-recognition");
    }

    #[test]
    fn a_scan_already_read_as_text_is_counted_and_read_when_asked() {
        let f: Vec<String> = (1..=3).map(footer).collect();
        let pdf = build(&[
            page(&[PROSE[0], PROSE[1], PROSE[2], &f[0]], 0.0),
            page(&[&f[1]], 1.0),
            page(&[PROSE[0], PROSE[1], PROSE[2], &f[2]], 0.0),
        ]);
        let dir = TempDir::new();
        dir.write("o/oficio.pdf", &pdf);
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        // As the test of the 80 characters read it before: whole text.
        let content: i64 = base.conn().query_row("SELECT id FROM content", [], |row| row.get(0)).unwrap();
        base.conn().execute("UPDATE content SET state = 'processed' WHERE id = ?1", [content]).unwrap();
        base.conn()
            .execute(
                "INSERT INTO derivative (content_id, kind, engine, body, created_at) VALUES (?1, 'text', 'extraction', 'texto', 0)
                 ON CONFLICT (content_id, kind) DO NOTHING",
                [content],
            )
            .unwrap();
        assert_eq!(base.health(&Visible::All).unwrap().unread_scan_pages, 0);

        while base.profile_pending(10).unwrap() > 0 {}
        let health = base.health(&Visible::All).unwrap();
        assert_eq!((health.unread_scan_pages, health.unread_scan_documents.clone()), (1, vec![content]));

        // Asked for, it is read; nothing else is.
        base.set_recognition_engine(Some(&RecognitionConfig { engine: "codex-cli".into(), model: "m".into() })).unwrap();
        assert_eq!(base.request_recognition(&RecognitionRequest::These(vec![content])).unwrap(), 1);
        let recognizer = FakeRecognizer::of("codex-cli:m", vec![Ok("la hoja escaneada")]);
        assert!(base.recognize_content(content, &recognizer).unwrap());
        assert_eq!(recognizer.calls(), 1);
        assert_eq!(base.health(&Visible::All).unwrap().unread_scan_pages, 0);
        assert!(!base.recognize_content(content, &recognizer).unwrap());
    }

    /// What the ficha marks on a real base (only read; a copy), without a
    /// model: the scanned pages already read that it would read now.
    ///
    /// ```text
    /// MICELYA_DOCBASE_COPY=C:\tmp\private.sqlite3 MICELYA_DOCBASE_RECOGNIZED=C:\datos\recognized \
    ///     cargo test --lib docbase::page_shapes -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads a copy of a real base and its files"]
    fn unread_scans_of_a_real_base() {
        let Some((_scratch, base)) = crate::docbase::knowledge::check::open_base() else {
            println!("MICELYA_DOCBASE_COPY is not set: nothing to measure");
            return;
        };
        if let Ok(from) = std::env::var("MICELYA_DOCBASE_RECOGNIZED") {
            for entry in std::fs::read_dir(&from).unwrap().flatten() {
                let to = base.recognized.join(entry.file_name());
                std::fs::create_dir_all(&to).unwrap();
                for page in std::fs::read_dir(entry.path()).unwrap().flatten() {
                    std::fs::copy(page.path(), to.join(page.file_name())).unwrap();
                }
            }
        }
        while base.profile_pending(20).unwrap() > 0 {}
        let bodies: Vec<String> = {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT body FROM derivative WHERE kind = 'page-shapes'").unwrap();
            let rows = statement.query_map([], |row| row.get::<_, String>(0)).unwrap();
            rows.map(|row| row.unwrap()).collect()
        };
        let mut shapes = std::collections::BTreeMap::new();
        let (mut pages, mut read, mut earlier) = (0, 0, 0);
        for body in &bodies {
            let ficha: super::PdfShapes = serde_json::from_str(body).unwrap();
            pages += ficha.pages.len();
            read += ficha.to_read().len();
            earlier += ficha.read_by_the_earlier_test().len();
            for page in &ficha.pages {
                *shapes.entry(format!("{:?} · {:?}", page.shape, page.text_layer)).or_insert(0) += 1;
            }
        }
        println!("documentos PDF con ficha: {} · páginas: {pages}", bodies.len());
        for (shape, count) in &shapes {
            println!("  {count} · {shape}");
        }
        println!("páginas que la ficha reconoce: {read} (la prueba de antes: {earlier})");
        let health = base.health(&Visible::All).unwrap();
        println!(
            "ya leídas, escaneadas sin leer: {} páginas en {} documentos",
            health.unread_scan_pages,
            health.unread_scan_documents.len()
        );
    }
}
