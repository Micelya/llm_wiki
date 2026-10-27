//! The engine that reads the printed text of scanned pages on this
//! computer (D-558): RapidOCR (detection, orientation and recognition of
//! each line) over ONNX Runtime, with the models the `rapidocr` package
//! carries. Nothing leaves the computer and no model in the cloud is
//! asked, so the base runs it in the background even with the automatic
//! reading off (`docbase/recognize.rs`).
//!
//! The models and the ONNX Runtime library are not in the program nor in
//! the repository: `scripts/fetch-ocr-models.mjs` puts them in an `ocr/`
//! folder (`models/` and the library), which the app looks for in its
//! resources, beside the program, in `MICELYA_OCR_DIR`, and, in a
//! development build, in `src-tauri/ocr`. Without them the engine is not
//! available: it says so (`unavailable`) and the app goes on.
//!
//! RapidOCR says how sure it is of each line: a line below the catalog's
//! `minLineConfidence` (`resources/catalog/local-recognition.json`) does
//! not enter the text, so handwriting read badly is not taken for text
//! (D-518). Handwriting is not transcribed here: it is asked of a provider.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use rapidocr_core::model::{model_set_by_name, ModelCache, ModelDownloadMode};
use rapidocr_core::RapidOcr;
use serde::Deserialize;

use super::recognition::{TextRecognizer, UNREADABLE};

/// The name of the engine in the registry of AI services and in the
/// marker of what it read.
pub const ENGINE: &str = "rapidocr-onnx";
/// The folder, in the resources and beside the program, with the models
/// (`models/`) and the ONNX Runtime library.
pub const OCR_DIR: &str = "ocr";
const CATALOG: &str = include_str!("../../resources/catalog/local-recognition.json");

#[cfg(target_os = "windows")]
const RUNTIME_FILE: &str = "onnxruntime.dll";
#[cfg(target_os = "macos")]
const RUNTIME_FILE: &str = "libonnxruntime.dylib";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const RUNTIME_FILE: &str = "libonnxruntime.so";

/// What the catalog says of the local reading.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalCatalog {
    pub model_set: String,
    pub long_side_px: u32,
    pub min_line_confidence: f32,
}

pub fn catalog() -> &'static LocalCatalog {
    static PARSED: OnceLock<LocalCatalog> = OnceLock::new();
    PARSED.get_or_init(|| serde_json::from_str(CATALOG).expect("the catalog of the local reading is valid"))
}

/// One line as the engine read it, with how sure it is (0 to 1).
#[derive(Debug, Clone, PartialEq)]
pub struct ReadLine {
    pub text: String,
    pub score: f32,
}

/// The folders where the models and the library may be, in order.
fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("MICELYA_OCR_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    if let Some(resources) = crate::commands::fs::resource_dir_hint() {
        dirs.push(resources.join(OCR_DIR));
    }
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf)) {
        dirs.push(exe_dir.join(OCR_DIR));
        dirs.push(exe_dir.join("resources").join(OCR_DIR));
    }
    // The copy the script leaves in the repository, for a development
    // build; not in the tests, so every test sees the engine as the
    // continuous integration does (without models) unless it asks.
    if cfg!(debug_assertions) && !cfg!(test) {
        dirs.push(development_dir());
    }
    dirs
}

/// Where `scripts/fetch-ocr-models.mjs` leaves the models.
pub fn development_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(OCR_DIR)
}

/// Why the folder `dir` cannot be read with, or `None`: the library and
/// every model of the catalog's set are there (each checked against its
/// sha256 when the engine starts).
pub fn missing_in(dir: &Path) -> Option<String> {
    let Some(set) = model_set_by_name(&catalog().model_set) else {
        return Some(format!("Unknown set of models '{}'", catalog().model_set));
    };
    if !dir.join(RUNTIME_FILE).is_file() {
        return Some(format!("{RUNTIME_FILE} is missing"));
    }
    let missing = ModelCache::new(dir.join("models")).missing_assets(set);
    if !missing.is_empty() {
        let names: Vec<&str> = missing.iter().map(|asset| asset.filename).collect();
        return Some(format!("its models are missing ({})", names.join(", ")));
    }
    None
}

/// The folder the engine reads with: the first one that has everything.
pub fn ocr_dir() -> Result<PathBuf, String> {
    let dirs = candidate_dirs();
    if let Some(dir) = dirs.iter().find(|dir| missing_in(dir).is_none()) {
        return Ok(dir.clone());
    }
    let why = dirs.first().and_then(|dir| missing_in(dir)).unwrap_or_else(|| "no folder to look in".to_string());
    Err(format!("The engine of this computer is not available: {why} (scripts/fetch-ocr-models.mjs)"))
}

/// Why the engine of this computer cannot read now, or `None`. Cheap: it
/// does not start the engine, but tells why it failed to start if it did.
pub fn unavailable() -> Option<String> {
    if let Some(Err(error)) = ENGINE_LOADED.get() {
        return Some(error.clone());
    }
    ocr_dir().err()
}

/// ONNX Runtime opened once per process, from the library in `dir`.
fn open_runtime(dir: &Path) -> Result<(), String> {
    static OPENED: OnceLock<Result<(), String>> = OnceLock::new();
    OPENED
        .get_or_init(|| {
            let library = dir.join(RUNTIME_FILE);
            let builder = ort::init_from(&library)
                .map_err(|error| format!("ONNX Runtime could not be opened from '{}': {error}", library.display()))?;
            builder.with_name("micelya-ocr").commit();
            Ok(())
        })
        .clone()
}

/// The engine, started from the models in `dir`.
pub fn start_in(dir: &Path) -> Result<RapidOcr, String> {
    if let Some(why) = missing_in(dir) {
        return Err(format!("The engine of this computer is not available: {why}"));
    }
    open_runtime(dir)?;
    let set = model_set_by_name(&catalog().model_set).ok_or_else(|| format!("Unknown set of models '{}'", catalog().model_set))?;
    let cache = ModelCache::new(dir.join("models"));
    cache
        .ensure_model_set(set, ModelDownloadMode::Never)
        .map_err(|error| format!("The models of the engine of this computer are not right: {error:#}"))?;
    let mut config = cache.config_for(set);
    // Every line comes back with its confidence: which ones enter is the
    // catalog's threshold (`text_of`).
    config.text_score = 0.0;
    config.inference.intra_threads = threads();
    RapidOcr::new(config).map_err(|error| format!("The engine of this computer could not start: {error:#}"))
}

/// Threads of the engine: half the processors, up to four, so the app
/// stays responsive while it reads.
fn threads() -> usize {
    std::thread::available_parallelism().map_or(1, |n| n.get() / 2).clamp(1, 4)
}

/// The engine of the process, started on the first page it reads and kept:
/// starting it again for every page would cost more than reading it.
static ENGINE_LOADED: OnceLock<Result<Mutex<RapidOcr>, String>> = OnceLock::new();

fn engine() -> Result<MutexGuard<'static, RapidOcr>, String> {
    let loaded = ENGINE_LOADED.get_or_init(|| ocr_dir().and_then(|dir| start_in(&dir)).map(Mutex::new));
    match loaded {
        // A page that panicked half-way leaves the engine as it was.
        Ok(engine) => Ok(engine.lock().unwrap_or_else(|poisoned| poisoned.into_inner())),
        Err(error) => Err(error.clone()),
    }
}

/// The lines of the image at `image`, top to bottom, each with its
/// confidence; an image larger than the catalog's long side is scaled
/// down first.
pub fn read_lines(ocr: &mut RapidOcr, image: &Path) -> Result<Vec<ReadLine>, String> {
    let picture = image::open(image).map_err(|error| format!("The image '{}' could not be opened: {error}", image.display()))?;
    let long_side = catalog().long_side_px.max(1);
    let picture = if picture.width().max(picture.height()) > long_side {
        picture.resize(long_side, long_side, image::imageops::FilterType::Triangle)
    } else {
        picture
    };
    let output = ocr.run_image(&picture.to_rgb8()).map_err(|error| format!("The engine of this computer failed: {error:#}"))?;
    Ok(output.lines.into_iter().map(|line| ReadLine { text: line.text, score: line.score }).collect())
}

/// The text of the lines whose confidence reaches `threshold`, one per
/// row. A page with lines but none sure enough says `[ilegible]`, so the
/// screens offer it to another engine; a page without any line is blank.
pub fn text_of(lines: &[ReadLine], threshold: f32) -> String {
    let kept: Vec<&str> = lines.iter().filter(|line| line.score >= threshold).map(|line| line.text.trim()).filter(|text| !text.is_empty()).collect();
    if kept.is_empty() && !lines.is_empty() {
        return UNREADABLE.to_string();
    }
    kept.join("\n")
}

/// Reads with the engine of this computer, one page at a time.
pub struct LocalRecognizer;

impl LocalRecognizer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LocalRecognizer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextRecognizer for LocalRecognizer {
    fn engine_id(&self) -> String {
        format!("{ENGINE}:{}", catalog().model_set)
    }

    fn recognize(&self, image: &Path) -> Result<String, String> {
        let mut ocr = engine()?;
        let lines = read_lines(&mut ocr, image)?;
        Ok(text_of(&lines, catalog().min_line_confidence))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/source_volume/fixtures").join(name)
    }

    fn line(text: &str, score: f32) -> ReadLine {
        ReadLine { text: text.into(), score }
    }

    #[test]
    fn only_the_lines_sure_enough_enter_the_text() {
        let lines = [line("PRIMERA LINEA", 0.98), line("garabato", 0.31), line("SEGUNDA LINEA", 0.91)];
        assert_eq!(text_of(&lines, 0.5), "PRIMERA LINEA\nSEGUNDA LINEA");
        assert_eq!(text_of(&lines, 0.0), "PRIMERA LINEA\ngarabato\nSEGUNDA LINEA");
        // Lines, none sure enough: unreadable, to be offered to another engine.
        assert_eq!(text_of(&[line("garabato", 0.2)], 0.5), UNREADABLE);
        // No line at all: a blank page.
        assert_eq!(text_of(&[], 0.5), "");
    }

    #[test]
    fn the_catalog_says_how_to_read() {
        let catalog = catalog();
        assert!(model_set_by_name(&catalog.model_set).is_some());
        assert!((0.0..=1.0).contains(&catalog.min_line_confidence));
        assert!(catalog.long_side_px >= 1000);
        assert_eq!(LocalRecognizer::new().engine_id(), format!("rapidocr-onnx:{}", catalog.model_set));
    }

    #[test]
    fn without_its_models_the_engine_says_it_is_not_available() {
        let empty = std::env::temp_dir().join(format!("micelya-ocr-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&empty).unwrap();
        let why = missing_in(&empty).unwrap();
        assert!(why.contains(RUNTIME_FILE), "{why}");
        let error = start_in(&empty).err().unwrap();
        assert!(error.contains("not available"), "{error}");
        let _ = std::fs::remove_dir_all(&empty);
    }

    /// Reads an invented scanned page (`fixtures/pagina-prueba.png`). Skipped
    /// when the models were not downloaded (the continuous integration only
    /// builds): `node scripts/fetch-ocr-models.mjs`.
    #[test]
    fn with_its_models_it_reads_a_printed_page() {
        let dir = development_dir();
        if missing_in(&dir).is_some() {
            eprintln!("skipped: the models of the engine of this computer are not downloaded");
            return;
        }
        let mut ocr = start_in(&dir).unwrap();
        let lines = read_lines(&mut ocr, &fixture("pagina-prueba.png")).unwrap();
        let text = text_of(&lines, catalog().min_line_confidence);
        assert!(text.contains("CONTRATO DE PRUEBA"), "{text}");
        assert!(text.contains("1.284.650,75"), "{text}");
        assert!(text.contains("03/11/2026"), "{text}");
        assert!(lines.iter().all(|line| (0.0..=1.0).contains(&line.score)));
    }
}
