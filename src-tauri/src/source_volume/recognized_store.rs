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

use super::recognition::RecognizedPage;

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
