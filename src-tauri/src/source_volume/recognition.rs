//! Text recognition for sources that carry their text as pixels: scanned
//! PDF pages and photos of documents.
//!
//! The rest of the app keeps reading a source's text from the usual
//! extracted-text cache. This module only decides *which* parts of a
//! source need recognition and hands the pixels to a `TextRecognizer`;
//! which engine does the reading is a detail behind that trait, so
//! engines can be added or swapped without touching callers.
//!
//! Recognized text is machine-read and can be wrong in ways extracted
//! text cannot (a `0` for an `O` in a code, a digit in an amount), so
//! every recognized block is preceded by a marker naming the engine.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::recognition_codex::CodexRecognizer;
use super::recognized_store::{Part, RecognizedStore};

/// A PDF page with fewer non-whitespace characters than this is treated
/// as having no text of its own. Same threshold the image extractor uses
/// to tell a text page from a scan with only a page number or watermark.
const MIN_PAGE_TEXT_CHARS: usize = 80;
/// Long side, in pixels, of a page rendered for recognition.
const RENDER_LONG_SIDE_PX: i32 = 2000;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionConfig {
    pub engine: String,
    #[serde(default)]
    pub model: String,
}

pub trait TextRecognizer {
    /// Stable name of the engine and model, written into the marker that
    /// precedes everything this recognizer produced.
    fn engine_id(&self) -> String;

    /// Full text visible in the image, or an error. An engine must fail
    /// rather than return a partial or empty answer, so a page is never
    /// cached as blank because of a transient problem.
    fn recognize(&self, image: &Path) -> Result<String, String>;
}

pub fn recognizer_for(config: &RecognitionConfig) -> Result<Box<dyn TextRecognizer>, String> {
    match config.engine.as_str() {
        "codex-cli" => Ok(Box::new(CodexRecognizer::new(&config.model))),
        other => Err(format!("Unknown text recognition engine: '{other}'")),
    }
}

/// Image formats a standalone source can be recognized from.
pub fn is_recognizable_image(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg")
}

pub fn marker(engine_id: &str) -> String {
    format!("<!-- texto-reconocido motor=\"{engine_id}\" -->")
}

/// Where recognized pages of `source` were kept before they moved to
/// the recognized store: next to its extracted-text cache, addressed by
/// the path the app asked for. Only read now, to take that text over.
pub fn page_cache_dir(requested_source: &Path) -> PathBuf {
    let parent = requested_source.parent().unwrap_or(Path::new("."));
    let name = requested_source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    parent.join(".cache").join(format!("{name}.recognized"))
}

/// Text of a standalone image source. An image already read is not read
/// again.
pub fn recognize_image_source(
    image: &Path,
    store: &RecognizedStore,
    recognizer: &dyn TextRecognizer,
) -> Result<String, String> {
    let entry = match store.get(Part::Image) {
        Some(entry) => entry,
        None => {
            let entry = RecognizedPage {
                engine_id: recognizer.engine_id(),
                text: recognizer.recognize(image)?.trim().to_string(),
            };
            store.put(Part::Image, &entry)?;
            entry
        }
    };
    Ok(format!("{}\n{}\n", marker(&entry.engine_id), entry.text))
}

/// `extracted` (the text PDFium found, one `## Page N` section per page)
/// with the pages that have no text of their own filled in by
/// `recognizer`.
///
/// Each recognized page is saved in `store` as soon as it is read,
/// so a failure half-way through a long scan does not throw away the
/// pages already done: the error is returned, nothing is spliced, and
/// the next attempt resumes where this one stopped.
pub fn complete_pdf_text(
    pdf: &Path,
    extracted: &str,
    store: &RecognizedStore,
    recognizer: &dyn TextRecognizer,
) -> Result<String, String> {
    complete_pdf_text_reporting(pdf, extracted, store, recognizer, &|_, _| {})
}

/// Same as `complete_pdf_text`; `on_page(position, total)` is called for
/// each page that needs recognition, before it is read, counting from 1.
pub fn complete_pdf_text_reporting(
    pdf: &Path,
    extracted: &str,
    store: &RecognizedStore,
    recognizer: &dyn TextRecognizer,
    on_page: &dyn Fn(usize, usize),
) -> Result<String, String> {
    let pages = textless_pages(pdf)?;
    if pages.is_empty() {
        return Ok(extracted.to_string());
    }
    let total = pages.len();

    let mut recognized = BTreeMap::new();
    let mut scratch: Option<PathBuf> = None;
    let mut failure = None;
    for (index, page) in pages.into_iter().enumerate() {
        on_page(index + 1, total);
        if let Some(hit) = store.get(Part::Page(page)) {
            recognized.insert(page, hit);
            continue;
        }
        let dir = match &scratch {
            Some(dir) => dir.clone(),
            None => {
                let dir = std::env::temp_dir()
                    .join(format!("micelya-recognition-{}", uuid::Uuid::new_v4()));
                fs::create_dir_all(&dir)
                    .map_err(|e| format!("Failed to create '{}': {e}", dir.display()))?;
                scratch = Some(dir.clone());
                dir
            }
        };
        let result = render_page(pdf, page, &dir).and_then(|image| {
            let text = recognizer.recognize(&image)?;
            let entry = RecognizedPage {
                engine_id: recognizer.engine_id(),
                text: text.trim().to_string(),
            };
            store.put(Part::Page(page), &entry)?;
            Ok(entry)
        });
        match result {
            Ok(entry) => {
                recognized.insert(page, entry);
            }
            Err(err) => {
                failure = Some(format!("Text recognition failed on page {page}: {err}"));
                break;
            }
        }
    }
    if let Some(dir) = scratch {
        let _ = fs::remove_dir_all(dir);
    }
    if let Some(err) = failure {
        return Err(err);
    }
    Ok(splice_recognized_pages(extracted, &recognized))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecognizedPage {
    pub engine_id: String,
    pub text: String,
}

/// Insert each recognized page at the end of its `## Page N` section.
/// A page whose section cannot be found is appended as a new section, so
/// recognized text is never dropped.
pub fn splice_recognized_pages(extracted: &str, pages: &BTreeMap<u32, RecognizedPage>) -> String {
    let mut out = extracted.to_string();
    let mut orphans = Vec::new();
    // Highest page first, so earlier insert positions stay valid.
    for (page, entry) in pages.iter().rev() {
        let block = format!("{}\n{}\n", marker(&entry.engine_id), entry.text);
        match section_end(&out, *page) {
            Some(end) => out.insert_str(end, &block),
            None => orphans.push((*page, block)),
        }
    }
    for (page, block) in orphans.into_iter().rev() {
        if !out.is_empty() {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("\n\n");
        }
        out.push_str(&format!("## Page {page}\n\n{block}"));
    }
    out
}

/// Byte offset just past the body of the `## Page {page}` section.
fn section_end(text: &str, page: u32) -> Option<usize> {
    let heading = format!("## Page {page}\n");
    let start = if text.starts_with(&heading) {
        0
    } else {
        text.find(&format!("\n{heading}"))? + 1
    };
    let body = start + heading.len();
    match text[body..].find("\n\n## Page ") {
        Some(next) => Some(body + next),
        None => Some(text.len()),
    }
}

/// 1-based numbers of the pages that have no usable text of their own.
/// Decided by how much text the page has, not by whether it holds one
/// page-sized image: scans split into strips have many small images.
pub fn textless_pages(pdf: &Path) -> Result<Vec<u32>, String> {
    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(pdf, None)
        .map_err(|e| format!("Failed to open PDF '{}': {e}", pdf.display()))?;
    let mut pages = Vec::new();
    for (index, page) in doc.pages().iter().enumerate() {
        let chars = page
            .text()
            .map(|text| text.all().chars().filter(|c| !c.is_whitespace()).count())
            .unwrap_or(0);
        if chars < MIN_PAGE_TEXT_CHARS {
            pages.push(index as u32 + 1);
        }
    }
    Ok(pages)
}

/// Render one page to a PNG in `dest_dir` and return its path.
fn render_page(pdf: &Path, page: u32, dest_dir: &Path) -> Result<PathBuf, String> {
    use pdfium_render::prelude::*;

    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(pdf, None)
        .map_err(|e| format!("Failed to open PDF '{}': {e}", pdf.display()))?;
    let pdf_page = doc
        .pages()
        .get((page - 1) as i32)
        .map_err(|e| format!("Page {page} not found: {e}"))?;
    let config = PdfRenderConfig::new()
        .set_target_width(RENDER_LONG_SIDE_PX)
        .set_maximum_height(RENDER_LONG_SIDE_PX);
    let image = pdf_page
        .render_with_config(&config)
        .map_err(|e| format!("Page {page} could not be rendered: {e}"))?
        .as_image()
        .map_err(|e| format!("Page {page} could not be converted to an image: {e}"))?;
    let path = dest_dir.join(format!("p{page:04}.png"));
    image
        .to_luma8()
        .save_with_format(&path, image::ImageFormat::Png)
        .map_err(|e| format!("Page {page} could not be saved: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    /// Answers with a canned text per call and records what it was shown.
    struct FakeRecognizer {
        answers: RefCell<Vec<Result<String, String>>>,
        seen: RefCell<Vec<PathBuf>>,
    }

    impl FakeRecognizer {
        fn new(answers: Vec<Result<&str, &str>>) -> Self {
            Self {
                answers: RefCell::new(
                    answers
                        .into_iter()
                        .rev()
                        .map(|a| a.map(str::to_string).map_err(str::to_string))
                        .collect(),
                ),
                seen: RefCell::new(Vec::new()),
            }
        }

        fn calls(&self) -> usize {
            self.seen.borrow().len()
        }
    }

    impl TextRecognizer for FakeRecognizer {
        fn engine_id(&self) -> String {
            "falso:1".to_string()
        }

        fn recognize(&self, image: &Path) -> Result<String, String> {
            assert!(image.is_file(), "recognizer must be handed a real image");
            self.seen.borrow_mut().push(image.to_path_buf());
            self.answers
                .borrow_mut()
                .pop()
                .unwrap_or_else(|| Err("no more answers".to_string()))
        }
    }

    fn page(text: &str) -> RecognizedPage {
        RecognizedPage {
            engine_id: "falso:1".to_string(),
            text: text.to_string(),
        }
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/source_volume/fixtures")
            .join(name)
    }

    fn scratch() -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("micelya-recognition-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const M: &str = "<!-- texto-reconocido motor=\"falso:1\" -->";

    #[test]
    fn splices_recognized_text_into_the_matching_page_sections() {
        let extracted = "## Page 1\n\n\n\n\n## Page 2\n\ntexto real\n\n\n## Page 3\n\n\n";
        let pages = BTreeMap::from([(1, page("uno")), (3, page("tres"))]);
        assert_eq!(
            splice_recognized_pages(extracted, &pages),
            format!(
                "## Page 1\n\n\n{M}\nuno\n\n\n## Page 2\n\ntexto real\n\n\n## Page 3\n\n\n{M}\ntres\n"
            )
        );
    }

    #[test]
    fn splice_leaves_pages_with_text_untouched_and_keeps_orphans() {
        let extracted = "## Page 1\n\ntexto real\n";
        let pages = BTreeMap::from([(4, page("cuatro"))]);
        assert_eq!(
            splice_recognized_pages(extracted, &pages),
            format!("## Page 1\n\ntexto real\n\n\n## Page 4\n\n{M}\ncuatro\n")
        );
        assert_eq!(splice_recognized_pages(extracted, &BTreeMap::new()), extracted);
    }

    #[test]
    fn splice_does_not_confuse_page_1_with_page_10() {
        let extracted = "## Page 10\n\n\n";
        let pages = BTreeMap::from([(1, page("uno"))]);
        assert_eq!(
            splice_recognized_pages(extracted, &pages),
            format!("## Page 10\n\n\n\n\n## Page 1\n\n{M}\nuno\n")
        );
    }

    #[test]
    fn image_source_text_is_preceded_by_the_engine_marker() {
        let recognizer = FakeRecognizer::new(vec![Ok("  CONTRATO\nlinea 2  ")]);
        let text = recognize_image_source(
            &fixture("escaneo-2-paginas.pdf"),
            &RecognizedStore::at(scratch()),
            &recognizer,
        ).unwrap();
        assert_eq!(text, format!("{M}\nCONTRATO\nlinea 2\n"));
    }

    #[test]
    fn unknown_engine_is_rejected() {
        let config = RecognitionConfig {
            engine: "nada".to_string(),
            model: String::new(),
        };
        assert!(recognizer_for(&config).is_err());
    }

    #[test]
    fn page_cache_lives_beside_the_text_cache_of_the_requested_path() {
        assert_eq!(
            page_cache_dir(Path::new("/p/raw/sources/Docs/a b.pdf")),
            Path::new("/p/raw/sources/Docs/.cache/a b.pdf.recognized")
        );
    }

    #[test]
    fn an_image_already_read_is_not_read_again() {
        let image = fixture("pagina-prueba.png");
        let store = RecognizedStore::at(scratch());
        let first = FakeRecognizer::new(vec![Ok("texto de la foto")]);
        let text = recognize_image_source(&image, &store, &first).unwrap();
        assert_eq!(first.calls(), 1);

        let second = FakeRecognizer::new(vec![]);
        assert_eq!(recognize_image_source(&image, &store, &second).unwrap(), text);
        assert_eq!(second.calls(), 0);
    }

    #[test]
    fn each_page_needing_recognition_is_reported_in_order() {
        let pdf = fixture("escaneo-2-paginas.pdf");
        let recognizer = FakeRecognizer::new(vec![Ok("primera"), Ok("segunda")]);
        let seen = std::cell::RefCell::new(Vec::new());

        complete_pdf_text_reporting(
            &pdf,
            "## Page 1




## Page 2


",
            &RecognizedStore::at(scratch()),
            &recognizer,
            &|page, total| seen.borrow_mut().push((page, total)),
        )
        .unwrap();

        assert_eq!(*seen.borrow(), vec![(1, 2), (2, 2)]);
    }

    #[test]
    fn scanned_pdf_pages_are_detected_rendered_and_recognized() {
        let pdf = fixture("escaneo-2-paginas.pdf");
        assert_eq!(textless_pages(&pdf).unwrap(), vec![1, 2]);

        let cache_dir = scratch();
        let cache = RecognizedStore::at(&cache_dir);
        let recognizer = FakeRecognizer::new(vec![Ok("primera"), Ok("segunda")]);
        let extracted = "## Page 1\n\n\n\n\n## Page 2\n\n\n";

        let text = complete_pdf_text(&pdf, extracted, &cache, &recognizer).unwrap();

        assert_eq!(
            text,
            format!("## Page 1\n\n\n{M}\nprimera\n\n\n## Page 2\n\n\n{M}\nsegunda\n")
        );
        assert_eq!(recognizer.calls(), 2);

        // A second run is served from the per-page cache.
        let again = FakeRecognizer::new(vec![]);
        assert_eq!(
            complete_pdf_text(&pdf, extracted, &cache, &again).unwrap(),
            text
        );
        assert_eq!(again.calls(), 0);
        let _ = fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn a_failed_page_fails_the_document_but_keeps_finished_pages() {
        let pdf = fixture("escaneo-2-paginas.pdf");
        let cache_dir = scratch();
        let cache = RecognizedStore::at(&cache_dir);
        let extracted = "## Page 1\n\n\n\n\n## Page 2\n\n\n";

        let failing = FakeRecognizer::new(vec![Ok("primera"), Err("limite de uso")]);
        let err = complete_pdf_text(&pdf, extracted, &cache, &failing).unwrap_err();
        assert!(err.contains("page 2") && err.contains("limite de uso"), "{err}");

        // The retry only has to read the page that failed.
        let retry = FakeRecognizer::new(vec![Ok("segunda")]);
        let text = complete_pdf_text(&pdf, extracted, &cache, &retry).unwrap();
        assert!(text.contains("primera") && text.contains("segunda"));
        assert_eq!(retry.calls(), 1);
        let _ = fs::remove_dir_all(cache_dir);
    }

    #[test]
    fn pdf_with_its_own_text_is_returned_unchanged_without_recognition() {
        let pdf = fixture("con-texto.pdf");
        assert!(textless_pages(&pdf).unwrap().is_empty());
        let recognizer = FakeRecognizer::new(vec![]);
        let extracted = "## Page 1\n\ncualquier cosa\n";
        assert_eq!(
            complete_pdf_text(&pdf, extracted, &RecognizedStore::at(scratch()), &recognizer).unwrap(),
            extracted
        );
        assert_eq!(recognizer.calls(), 0);
    }
}
