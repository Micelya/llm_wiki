//! Where recognized text is kept: a folder per file, named by the
//! SHA-256 of the file.
//!
//! Recognized text is not a disposable cache. Producing it costs engine
//! calls, and reading the same image twice does not give the same text
//! twice. So it is kept, keyed by what the file *is* rather than where
//! it is: a file that
//! is moved, renamed, saved again without changes, or present in two
//! folders is read once.
//!
//! Each part is one small JSON file — `page-<n>.json` for a PDF page,
//! `image.json` for a standalone image — holding the text and the engine
//! that read it, and, when the engine says so, the words it doubts and the
//! model that read it. Nothing is ever written next to the original file.
//! The base keeps its store beside the database (`docbase::recognize`).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::recognition::{DoubtfulWord, RecognizedPage};

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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    doubtful: Vec<DoubtfulWord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    read_by: Option<String>,
}

pub struct RecognizedStore {
    dir: PathBuf,
    /// Only what this engine read counts: the text of another engine is
    /// read again (P-242: once per document and engine).
    engine: Option<String>,
}

impl RecognizedStore {
    /// A store kept in `dir`.
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: dir.into(),
            engine: None,
        }
    }

    /// The same store, where only what `engine` read is found.
    pub fn for_engine(mut self, engine: &str) -> Self {
        self.engine = Some(engine.to_string());
        self
    }

    pub fn get(&self, part: Part) -> Option<RecognizedPage> {
        let wanted = |page: &RecognizedPage| self.engine.as_deref().is_none_or(|engine| page.engine_id == engine);
        self.read(part).filter(wanted)
    }

    pub fn put(&self, part: Part, entry: &RecognizedPage) -> Result<(), String> {
        fs::create_dir_all(&self.dir)
            .map_err(|e| format!("Failed to create '{}': {e}", self.dir.display()))?;
        let stored = StoredPart {
            engine: entry.engine_id.clone(),
            text: entry.text.clone(),
            doubtful: entry.doubtful.clone(),
            read_by: entry.read_by.clone(),
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
            doubtful: stored.doubtful,
            read_by: stored.read_by,
        })
    }

    fn part_file(&self, part: Part) -> PathBuf {
        match part {
            Part::Page(page) => self.dir.join(format!("page-{page}.json")),
            Part::Image => self.dir.join("image.json"),
        }
    }
}
