//! Where recognized text is kept: `raw/recognized/<sha-256 of the file>/`
//! inside the project.
//!
//! Recognized text is not a disposable cache. Producing it costs engine
//! calls, and reading the same image twice does not give the same text
//! twice. So it is stored as project material, next to `raw/sources`,
//! and keyed by what the file *is* rather than where it is: a file that
//! is moved, renamed, saved again without changes, or present in two
//! folders is read once.
//!
//! Each part is one small JSON file — `page-<n>.json` for a PDF page,
//! `image.json` for a standalone image — holding the text and the engine
//! that read it. Nothing is ever written next to the original file.
//!
//! Text recognized before this store existed is taken over on first use,
//! so nothing already read is read again: PDF pages from the per-path
//! folder beside the extracted-text cache, and standalone images from
//! the extracted-text cache itself, which for an image is exactly the
//! recognized text.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use super::file_hash::file_sha256;
use super::recognition::{page_cache_dir, RecognizedPage};

/// Project-relative folder of the store.
pub const RECOGNIZED_DIR: &str = "raw/recognized";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// 1-based page of a PDF.
    Page(u32),
    /// The whole of a standalone image.
    Image,
}

#[derive(Serialize, Deserialize)]
struct StoredPart {
    engine: String,
    text: String,
}

/// Text recognized earlier for the same source path, in the old layout.
struct Legacy {
    dir: PathBuf,
    /// Extracted-text cache of the source (`.cache/<name>.txt`).
    text_cache: PathBuf,
    source_modified: Option<SystemTime>,
}

pub struct RecognizedStore {
    dir: PathBuf,
    legacy: Option<Legacy>,
}

impl RecognizedStore {
    /// A store kept in `dir`, with nothing to take over.
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            legacy: None,
        }
    }

    /// The store of a source. `requested` is the path the app asked for
    /// (logical for a mounted source) and `real` is where the file is.
    pub fn for_source(requested: &Path, real: &Path) -> Result<Self, String> {
        let legacy_dir = page_cache_dir(requested);
        let hash = file_sha256(real)?;
        let dir = match (super::project_root_of(requested), &hash) {
            (Some(project), Some(hash)) => project.join(RECOGNIZED_DIR).join(hash),
            // Outside a project, or too large to hash: keep the text by
            // path, as before.
            _ => return Ok(Self::at(legacy_dir)),
        };
        Ok(Self {
            dir,
            legacy: Some(Legacy {
                dir: legacy_dir,
                text_cache: text_cache_file(requested),
                source_modified: fs::metadata(real).and_then(|m| m.modified()).ok(),
            }),
        })
    }

    pub fn get(&self, part: Part) -> Option<RecognizedPage> {
        if let Some(found) = self.read(part) {
            return Some(found);
        }
        let taken_over = self.legacy.as_ref().and_then(|legacy| legacy.read(part))?;
        // Best effort: if it cannot be saved it is simply taken over
        // again next time.
        let _ = self.put(part, &taken_over);
        Some(taken_over)
    }

    pub fn put(&self, part: Part, entry: &RecognizedPage) -> Result<(), String> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| format!("Failed to create '{}': {e}", self.dir.display()))?;
        let stored = StoredPart {
            engine: entry.engine_id.clone(),
            text: entry.text.clone(),
        };
        let json = serde_json::to_string_pretty(&stored).map_err(|e| e.to_string())?;
        let path = self.part_file(part);
        fs::write(&path, json).map_err(|e| format!("Failed to write '{}': {e}", path.display()))
    }

    fn read(&self, part: Part) -> Option<RecognizedPage> {
        let raw = fs::read_to_string(self.part_file(part)).ok()?;
        let stored: StoredPart = serde_json::from_str(&raw).ok()?;
        Some(RecognizedPage {
            engine_id: stored.engine,
            text: stored.text,
        })
    }

    fn part_file(&self, part: Part) -> PathBuf {
        match part {
            Part::Page(page) => self.dir.join(format!("page-{page}.json")),
            Part::Image => self.dir.join("image.json"),
        }
    }
}

impl Legacy {
    /// Either old file is valid only if written after the source was
    /// last modified.
    fn read(&self, part: Part) -> Option<RecognizedPage> {
        match part {
            // `p0001.txt`, with the engine on the first line.
            Part::Page(page) => {
                let raw = self.read_if_current(&self.dir.join(format!("p{page:04}.txt")))?;
                let (engine_id, text) = raw.split_once('\n')?;
                Some(RecognizedPage {
                    engine_id: engine_id.strip_prefix("motor=")?.to_string(),
                    text: text.to_string(),
                })
            }
            // The text cache of an image: the recognition marker on the
            // first line, then the text.
            Part::Image => {
                let raw = self.read_if_current(&self.text_cache)?;
                let (first_line, text) = raw.split_once('\n')?;
                let engine_id = first_line
                    .trim()
                    .strip_prefix("<!-- texto-reconocido motor=\"")?
                    .strip_suffix("\" -->")?;
                Some(RecognizedPage {
                    engine_id: engine_id.to_string(),
                    text: text.trim().to_string(),
                })
            }
        }
    }

    fn read_if_current(&self, path: &Path) -> Option<String> {
        let written_at = fs::metadata(path).ok()?.modified().ok()?;
        if written_at < self.source_modified? {
            return None;
        }
        fs::read_to_string(path).ok()
    }
}

fn text_cache_file(requested_source: &Path) -> PathBuf {
    let parent = requested_source.parent().unwrap_or(Path::new("."));
    let name = requested_source
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    parent.join(".cache").join(format!("{name}.txt"))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        base: PathBuf,
        project: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!("micelya-recognized-{}", uuid::Uuid::new_v4()));
            let project = base.join("project");
            fs::create_dir_all(project.join("raw/sources/Docs")).unwrap();
            Self { base, project }
        }

        fn source(&self, rel: &str, bytes: &[u8]) -> PathBuf {
            let path = self.project.join("raw/sources").join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, bytes).unwrap();
            path
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    fn page(text: &str) -> RecognizedPage {
        RecognizedPage {
            engine_id: "fake/v1".to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn text_is_kept_in_the_project_under_the_hash_of_the_file() {
        let fx = Fixture::new();
        let scan = fx.source("Docs/scan.pdf", b"pdf bytes");
        let store = RecognizedStore::for_source(&scan, &scan).unwrap();

        store.put(Part::Page(2), &page("segunda")).unwrap();

        let hash = file_sha256(&scan).unwrap().unwrap();
        let file = fx.project.join("raw/recognized").join(hash).join("page-2.json");
        assert!(file.is_file());
        assert_eq!(store.get(Part::Page(2)), Some(page("segunda")));
        assert_eq!(store.get(Part::Page(1)), None);
    }

    #[test]
    fn a_copy_or_a_renamed_file_finds_the_text_already_read() {
        let fx = Fixture::new();
        let photo = fx.source("Docs/carta.jpeg", b"jpeg bytes");
        RecognizedStore::for_source(&photo, &photo)
            .unwrap()
            .put(Part::Image, &page("texto de la carta"))
            .unwrap();

        let copy = fx.source("Otra carpeta/carta (copia).jpeg", b"jpeg bytes");
        let other = fx.source("Docs/otra.jpeg", b"other bytes");

        assert_eq!(
            RecognizedStore::for_source(&copy, &copy).unwrap().get(Part::Image),
            Some(page("texto de la carta"))
        );
        assert_eq!(RecognizedStore::for_source(&other, &other).unwrap().get(Part::Image), None);
    }

    #[test]
    fn a_mounted_source_keeps_its_text_in_the_project_not_in_the_origin() {
        let fx = Fixture::new();
        let origin = fx.base.join("origin");
        fs::create_dir_all(&origin).unwrap();
        fs::write(origin.join("scan.pdf"), b"pdf bytes").unwrap();
        let logical = fx.project.join("raw/sources/Mounted/scan.pdf");

        RecognizedStore::for_source(&logical, &origin.join("scan.pdf"))
            .unwrap()
            .put(Part::Page(1), &page("uno"))
            .unwrap();

        assert_eq!(fs::read_dir(&origin).unwrap().count(), 1);
        assert!(fx.project.join("raw/recognized").is_dir());
    }

    #[test]
    fn pages_read_before_the_store_existed_are_taken_over_without_reading_again() {
        let fx = Fixture::new();
        let scan = fx.source("Docs/scan.pdf", b"pdf bytes");
        let old = page_cache_dir(&scan);
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("p0003.txt"), "motor=codex-cli/gpt\ntercera página").unwrap();

        let store = RecognizedStore::for_source(&scan, &scan).unwrap();
        let taken = store.get(Part::Page(3)).unwrap();

        assert_eq!(taken.engine_id, "codex-cli/gpt");
        assert_eq!(taken.text, "tercera página");
        // Now stored by hash: it survives the removal of the old cache.
        fs::remove_dir_all(&old).unwrap();
        assert_eq!(store.get(Part::Page(3)), Some(taken));
    }

    #[test]
    fn an_image_read_before_the_store_existed_is_taken_over_from_its_text_cache() {
        let fx = Fixture::new();
        let photo = fx.source("Docs/carta.jpeg", b"jpeg bytes");
        let cache = fx.project.join("raw/sources/Docs/.cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(
            cache.join("carta.jpeg.txt"),
            format!("{}\nCARTA DOCUMENTO\nBuenos Aires\n", crate::source_volume::recognition::marker("codex-cli/gpt")),
        )
        .unwrap();

        let store = RecognizedStore::for_source(&photo, &photo).unwrap();

        assert_eq!(
            store.get(Part::Image),
            Some(RecognizedPage {
                engine_id: "codex-cli/gpt".to_string(),
                text: "CARTA DOCUMENTO\nBuenos Aires".to_string(),
            })
        );
    }

    #[test]
    fn a_text_cache_that_is_not_recognized_text_is_not_taken_for_an_image() {
        let fx = Fixture::new();
        let photo = fx.source("Docs/carta.jpeg", b"jpeg bytes");
        let cache = fx.project.join("raw/sources/Docs/.cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("carta.jpeg.txt"), "no preprocessing needed").unwrap();

        assert_eq!(RecognizedStore::for_source(&photo, &photo).unwrap().get(Part::Image), None);
    }

    #[test]
    fn old_text_older_than_the_file_is_not_taken_over() {
        let fx = Fixture::new();
        let scan = fx.source("Docs/scan.pdf", b"old bytes");
        let old = page_cache_dir(&scan);
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("p0001.txt"), "motor=codex-cli/gpt\nvieja").unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(&scan, b"new bytes").unwrap();

        assert_eq!(RecognizedStore::for_source(&scan, &scan).unwrap().get(Part::Page(1)), None);
    }
}
