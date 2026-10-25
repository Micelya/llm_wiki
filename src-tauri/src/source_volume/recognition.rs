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

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

use super::recognition_codex::CodexRecognizer;
use super::recognition_core::CoreRecognizer;
use super::recognized_store::{Part, RecognizedStore};

/// Long side, in pixels, of a page rendered for recognition.
pub(crate) const RENDER_LONG_SIDE_PX: i32 = 2000;
/// What a page an engine could not read at all says, as Codex marks what
/// it cannot read: the screens offer it to another engine.
pub const UNREADABLE: &str = "[ilegible]";

/// Which engine recognizes text, and with which model.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionConfig {
    pub engine: String,
    #[serde(default)]
    pub model: String,
}

/// A word an engine is not sure it read well, as it says (Micelya Core
/// gives the confidence of each word).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DoubtfulWord {
    pub word: String,
    /// Its line in the page, counting from 1.
    pub line: usize,
    /// How sure the engine is, from 0 to 100.
    pub percent: u8,
}

/// What an engine read on one page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PageReading {
    pub text: String,
    /// The words it doubts, when it says how sure it is.
    pub doubtful: Vec<DoubtfulWord>,
    /// The model that read it, as its provider says (`textract-1.0`).
    pub read_by: Option<String>,
}

pub trait TextRecognizer {
    /// Stable name of the engine and model, written into the marker that
    /// precedes everything this recognizer produced.
    fn engine_id(&self) -> String;

    /// Full text visible in the image, or an error. An engine must fail
    /// rather than return a partial or empty answer, so a page is never
    /// cached as blank because of a transient problem.
    fn recognize(&self, image: &Path) -> Result<String, String>;

    /// Page `page` of a document (1 for a photo), with what the engine
    /// says beyond the text. By default, the text of `recognize`.
    fn recognize_page(&self, image: &Path, page: u32) -> Result<PageReading, String> {
        let _ = page;
        self.recognize(image).map(|text| PageReading { text, ..Default::default() })
    }

    /// The pages `pages` of a document, each read from the image
    /// `image(page)` makes of it: `done` is told of each one as it ends, in
    /// the order they end, and says whether to go on. After a page fails no
    /// other page starts. By default one page after the other; an engine
    /// that reads several at once says so here (Micelya Core, P-292).
    fn recognize_pages(&self, pages: &[u32], image: &PageImage<'_>, done: &mut dyn FnMut(u32, Result<PageReading, String>) -> bool) {
        for &page in pages {
            let reading = image(page).and_then(|image| self.recognize_page(&image, page));
            let failed = reading.is_err();
            if !done(page, reading) || failed {
                break;
            }
        }
    }
}

/// Makes the image of a page of a document, to be read; an engine that
/// reads several pages at once calls it from several threads.
pub type PageImage<'a> = dyn Fn(u32) -> Result<PathBuf, String> + Sync + 'a;

/// Reads `pages` with `read`, up to `at_once` at the same time, each on a
/// thread of its own, and tells `done`, on the calling thread, of each page
/// as it ends, in the order they end; `done` says whether to go on. After a
/// page fails, or `done` says to stop, no other page starts; those under
/// way end and are told, so nothing already read (and paid) is lost.
pub(crate) fn pages_at_once<T: Send>(
    at_once: usize,
    pages: &[u32],
    read: &(dyn Fn(u32) -> Result<T, String> + Sync),
    done: &mut dyn FnMut(u32, Result<T, String>) -> bool,
) {
    let stop = AtomicBool::new(false);
    let next = AtomicUsize::new(0);
    let (sender, received) = std::sync::mpsc::channel();
    std::thread::scope(|scope| {
        for _ in 0..at_once.clamp(1, pages.len().max(1)) {
            let sender = sender.clone();
            let (stop, next) = (&stop, &next);
            scope.spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let Some(&page) = pages.get(next.fetch_add(1, Ordering::SeqCst)) else {
                        break;
                    };
                    let result = read(page);
                    if result.is_err() {
                        stop.store(true, Ordering::SeqCst);
                    }
                    if sender.send((page, result)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(sender);
        for (page, result) in received {
            if !done(page, result) {
                stop.store(true, Ordering::SeqCst);
            }
        }
    });
}

pub fn recognizer_for(config: &RecognitionConfig) -> Result<Box<dyn TextRecognizer>, String> {
    match config.engine.as_str() {
        "codex-cli" => Ok(Box::new(CodexRecognizer::new(&config.model))),
        // Its connection is read from the keychain when it reads its first page.
        "core" => Ok(Box::new(CoreRecognizer::new(&config.model))),
        other => Err(format!("Unknown text recognition engine: '{other}'")),
    }
}

/// The name an engine and model save their text with (what `engine_id`
/// of its recognizer returns), without starting the engine.
pub fn engine_id(config: &RecognitionConfig) -> Result<String, String> {
    recognizer_for(config).map(|recognizer| recognizer.engine_id())
}

/// Image formats a standalone source can be recognized from.
pub fn is_recognizable_image(ext: &str) -> bool {
    matches!(ext, "png" | "jpg" | "jpeg")
}

pub fn marker(engine_id: &str) -> String {
    format!("<!-- texto-reconocido motor=\"{engine_id}\" -->")
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
            let entry = RecognizedPage::of(recognizer.engine_id(), recognizer.recognize_page(image, 1)?);
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
/// each page that needs recognition, before it is read, counting from 1
/// (from the engine's threads when it reads several pages at once).
///
/// The engine reads the pages not read yet (`recognize_pages`): one after
/// the other, or several at once, each kept as soon as it is read, in its
/// own section whatever the order they end in. The failure told is the one
/// of the first page that failed.
pub fn complete_pdf_text_reporting(
    pdf: &Path,
    extracted: &str,
    store: &RecognizedStore,
    recognizer: &dyn TextRecognizer,
    on_page: &(dyn Fn(usize, usize) + Sync),
) -> Result<String, String> {
    let shapes = super::page_shape::profile_pdf(pdf)?;
    complete_pdf_pages(pdf, &shapes.to_read(), &shapes.old_layers(), extracted, store, recognizer, on_page)
}

/// Same as `complete_pdf_text_reporting`, for these pages; the text of
/// the ones in `replaced` (an old recognition layer) gives way to what is
/// read now.
pub fn complete_pdf_pages(
    pdf: &Path,
    pages: &[u32],
    replaced: &BTreeSet<u32>,
    extracted: &str,
    store: &RecognizedStore,
    recognizer: &dyn TextRecognizer,
    on_page: &(dyn Fn(usize, usize) + Sync),
) -> Result<String, String> {
    if pages.is_empty() {
        return Ok(extracted.to_string());
    }
    let total = pages.len();
    let position = |page: u32| pages.iter().position(|p| *p == page).map_or(0, |index| index + 1);

    let mut recognized = BTreeMap::new();
    let mut to_read = Vec::new();
    for (index, page) in pages.iter().enumerate() {
        match store.get(Part::Page(*page)) {
            Some(hit) => {
                on_page(index + 1, total);
                recognized.insert(*page, hit);
            }
            None => to_read.push(*page),
        }
    }
    if to_read.is_empty() {
        return Ok(splice_replacing(extracted, &recognized, replaced));
    }
    let dir = std::env::temp_dir().join(format!("micelya-recognition-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create '{}': {e}", dir.display()))?;
    let engine = recognizer.engine_id();
    let mut failure: Option<(u32, String)> = None;
    let image = |page: u32| {
        on_page(position(page), total);
        render_page(pdf, page, &dir)
    };
    recognizer.recognize_pages(&to_read, &image, &mut |page, reading| {
        let kept = reading.and_then(|reading| {
            let entry = RecognizedPage::of(engine.clone(), reading);
            store.put(Part::Page(page), &entry)?;
            Ok(entry)
        });
        match kept {
            Ok(entry) => {
                recognized.insert(page, entry);
                true
            }
            Err(err) => {
                if failure.as_ref().is_none_or(|(first, _)| page < *first) {
                    failure = Some((page, err));
                }
                false
            }
        }
    });
    let _ = fs::remove_dir_all(&dir);
    if let Some((page, err)) = failure {
        return Err(format!("Text recognition failed on page {page}: {err}"));
    }
    Ok(splice_replacing(extracted, &recognized, replaced))
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecognizedPage {
    pub engine_id: String,
    pub text: String,
    /// The words the engine doubts (kept beside the text, which they do
    /// not change).
    pub doubtful: Vec<DoubtfulWord>,
    /// The model that read it, as its provider says.
    pub read_by: Option<String>,
}

impl RecognizedPage {
    fn of(engine_id: String, reading: PageReading) -> Self {
        Self { engine_id, text: reading.text.trim().to_string(), doubtful: reading.doubtful, read_by: reading.read_by }
    }
}

/// Insert each recognized page at the end of its `## Page N` section.
/// A page whose section cannot be found is appended as a new section, so
/// recognized text is never dropped.
pub fn splice_recognized_pages(extracted: &str, pages: &BTreeMap<u32, RecognizedPage>) -> String {
    splice_replacing(extracted, pages, &BTreeSet::new())
}

/// Same as `splice_recognized_pages`; the body of the sections of the
/// pages in `replaced` goes away first (an old recognition layer, read
/// again).
pub fn splice_replacing(extracted: &str, pages: &BTreeMap<u32, RecognizedPage>, replaced: &BTreeSet<u32>) -> String {
    let mut out = extracted.to_string();
    for page in replaced.iter().rev().filter(|page| pages.contains_key(page)) {
        if let Some(end) = section_end(&out, *page) {
            let heading = format!("## Page {page}
");
            let start = if out.starts_with(&heading) { 0 } else { out.find(&format!("
{heading}")).map_or(end, |at| at + 1) };
            let body = (start + heading.len()).min(end);
            out.replace_range(body..end, "
");
        }
    }
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

/// 1-based numbers of the pages that need recognition, as their ficha of
/// shape says (`page_shape`, P-320): scans without text of their own, and
/// scans whose old recognition layer is noisy.
pub fn textless_pages(pdf: &Path) -> Result<Vec<u32>, String> {
    Ok(super::page_shape::profile_pdf(pdf)?.to_read())
}

/// Render one page to a PNG in `dest_dir` and return its path.
pub(crate) fn render_page(pdf: &Path, page: u32, dest_dir: &Path) -> Result<PathBuf, String> {
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

/// A stand-in engine for the tests: answers with a canned text per call
/// and records what it was shown. It never calls a real engine.
#[cfg(test)]
pub(crate) struct FakeRecognizer {
    engine: String,
    answers: std::cell::RefCell<Vec<Result<String, String>>>,
    seen: std::cell::RefCell<Vec<PathBuf>>,
}

#[cfg(test)]
impl FakeRecognizer {
    pub(crate) fn new(answers: Vec<Result<&str, &str>>) -> Self {
        Self::of("falso:1", answers)
    }

    /// One that says it is `engine`.
    pub(crate) fn of(engine: &str, answers: Vec<Result<&str, &str>>) -> Self {
        Self {
            engine: engine.to_string(),
            answers: std::cell::RefCell::new(
                answers
                    .into_iter()
                    .rev()
                    .map(|a| a.map(str::to_string).map_err(str::to_string))
                    .collect(),
            ),
            seen: std::cell::RefCell::new(Vec::new()),
        }
    }

    pub(crate) fn calls(&self) -> usize {
        self.seen.borrow().len()
    }
}

#[cfg(test)]
impl TextRecognizer for FakeRecognizer {
    fn engine_id(&self) -> String {
        self.engine.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn page(text: &str) -> RecognizedPage {
        RecognizedPage {
            engine_id: "falso:1".to_string(),
            text: text.to_string(),
            ..Default::default()
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
        let codex = RecognitionConfig { engine: "codex-cli".to_string(), model: "m".to_string() };
        assert_eq!(engine_id(&codex).unwrap(), "codex-cli:m");
        // Core's needs nothing to be named: its connection is read later.
        let core = RecognitionConfig { engine: "core".to_string(), model: String::new() };
        assert_eq!(engine_id(&core).unwrap(), "core:textract-1.0");
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
        let seen = std::sync::Mutex::new(Vec::new());

        complete_pdf_text_reporting(
            &pdf,
            "## Page 1




## Page 2


",
            &RecognizedStore::at(scratch()),
            &recognizer,
            &|page, total| seen.lock().unwrap().push((page, total)),
        )
        .unwrap();

        assert_eq!(*seen.lock().unwrap(), vec![(1, 2), (2, 2)]);
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
    fn pages_go_at_once_up_to_the_limit_and_none_starts_after_one_fails() {
        let reading_now = AtomicUsize::new(0);
        let most = AtomicUsize::new(0);
        let read = |page: u32| {
            let now = reading_now.fetch_add(1, Ordering::SeqCst) + 1;
            most.fetch_max(now, Ordering::SeqCst);
            // Page 3 fails soon; the others take longer.
            std::thread::sleep(std::time::Duration::from_millis(if page == 3 { 40 } else { 200 }));
            reading_now.fetch_sub(1, Ordering::SeqCst);
            if page == 3 {
                Err("falla de prueba".to_string())
            } else {
                Ok(page * 10)
            }
        };
        let all: Vec<u32> = (1..=10).collect();

        let mut ended = BTreeMap::new();
        pages_at_once(4, &all[..2].iter().chain(&all[3..]).copied().collect::<Vec<_>>(), &read, &mut |page, result| {
            ended.insert(page, result.unwrap());
            true
        });
        assert_eq!(ended.len(), 9);
        assert!(ended.iter().all(|(page, value)| *value == page * 10));
        assert_eq!(most.load(Ordering::SeqCst), 4);

        // Page 3 fails: the pages under way end and are told, no other starts.
        let mut ended = BTreeMap::new();
        pages_at_once(4, &all, &read, &mut |page, result| {
            ended.insert(page, result);
            true
        });
        assert_eq!(ended.keys().copied().collect::<Vec<_>>(), [1, 2, 3, 4]);
        assert!(ended[&3].is_err() && ended[&4].is_ok());

        // One at a time, in order.
        let mut order = Vec::new();
        most.store(0, Ordering::SeqCst);
        pages_at_once(1, &[5, 6, 7], &read, &mut |page, _| {
            order.push(page);
            true
        });
        assert_eq!((order, most.load(Ordering::SeqCst)), (vec![5, 6, 7], 1));
    }

    #[test]
    fn a_noisy_old_layer_read_again_gives_way_to_the_new_reading() {
        use crate::source_volume::page_shape::test_pdf::{build, page};
        let noisy = [
            "C O N T R A T O de l o c a c i o n e n t r e l a s p a r t e s q u e s u s c r i b e n",
            "e l p r e s e n t e e n l a C i u d a d I n v e n t a d a a l o s d i a s d e l m e s",
        ];
        let good = "Segunda pagina con su texto propio, escrita en renglones largos como un escrito comun del estudio.";
        let dir = scratch();
        let pdf = dir.join("capa-vieja.pdf");
        std::fs::write(&pdf, build(&[page(&noisy, 1.0), page(&[good], 0.0)])).unwrap();
        let recognizer = FakeRecognizer::new(vec![Ok("CONTRATO de locacion entre las partes")]);
        let extracted = format!("## Page 1

{}
{}

## Page 2

{good}
", noisy[0], noisy[1]);

        let text = complete_pdf_text(&pdf, &extracted, &RecognizedStore::at(dir.join("leidas")), &recognizer).unwrap();

        assert_eq!(text, format!("## Page 1

{M}
CONTRATO de locacion entre las partes


## Page 2

{good}
"));
        assert_eq!(recognizer.calls(), 1);
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
