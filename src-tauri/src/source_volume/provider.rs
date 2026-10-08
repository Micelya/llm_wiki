//! Where a mounted source set physically lives.
//!
//! A provider answers the questions the volume cannot answer by itself:
//! is the origin reachable, what does it contain, and (when the origin is
//! a real directory) where on disk is a given entry. Local folders are
//! the only implementation today; cloud drives are expected to implement
//! the same trait without the rest of the app noticing.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProviderKind {
    LocalFolder,
}

/// One file offered by a provider, addressed relative to the mount root
/// with forward slashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEntry {
    pub rel_path: String,
    pub size: u64,
    pub mtime_ms: i64,
}

pub trait SourceProvider {
    fn kind(&self) -> ProviderKind;

    /// False when the origin cannot be reached right now (unplugged
    /// drive, offline share). Callers must treat that as "unknown", never
    /// as "every file was deleted".
    fn is_available(&self) -> bool;

    /// Every file under the origin. Errors when the origin is unavailable.
    fn list(&self) -> Result<Vec<SourceEntry>, String>;

    /// On-disk path for an entry, for providers backed by a real
    /// directory. `None` means the content has to be fetched instead.
    fn local_path(&self, rel_path: &str) -> Option<PathBuf>;
}

pub struct LocalFolderProvider {
    root: PathBuf,
}

impl LocalFolderProvider {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl SourceProvider for LocalFolderProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::LocalFolder
    }

    fn is_available(&self) -> bool {
        self.root.is_dir()
    }

    fn list(&self) -> Result<Vec<SourceEntry>, String> {
        if !self.is_available() {
            return Err(format!(
                "Source folder is not available: '{}'",
                self.root.display()
            ));
        }
        let mut entries = Vec::new();
        for entry in WalkDir::new(&self.root).into_iter().filter_map(Result::ok) {
            if !entry.file_type().is_file() {
                continue;
            }
            let Ok(rel) = entry.path().strip_prefix(&self.root) else {
                continue;
            };
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let mtime_ms = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0);
            entries.push(SourceEntry {
                rel_path: rel.to_string_lossy().replace('\\', "/"),
                size: meta.len(),
                mtime_ms,
            });
        }
        entries.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        Ok(entries)
    }

    fn local_path(&self, rel_path: &str) -> Option<PathBuf> {
        let mut path = self.root.clone();
        for part in rel_path.split('/').filter(|part| !part.is_empty()) {
            if part == "." || part == ".." {
                return None;
            }
            path.push(part);
        }
        Some(path)
    }
}
