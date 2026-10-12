//! A scratch folder for tests, removed when the test ends.

use std::fs;
use std::path::{Path, PathBuf};

pub struct TempDir {
    root: PathBuf,
}

impl TempDir {
    pub fn new() -> Self {
        let root = std::env::temp_dir().join(format!("micelya-docbase-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Creates a folder and returns its path.
    pub fn folder(&self, rel: &str) -> PathBuf {
        let path = self.root.join(rel);
        fs::create_dir_all(&path).unwrap();
        path
    }

    pub fn write(&self, rel: &str, bytes: &[u8]) -> PathBuf {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }

    pub fn remove(&self, rel: &str) {
        fs::remove_file(self.root.join(rel)).unwrap();
    }

    pub fn remove_dir(&self, rel: &str) {
        fs::remove_dir_all(self.root.join(rel)).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
