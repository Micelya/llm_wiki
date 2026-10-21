//! A scratch folder for tests, removed when the test ends.

use std::fs;
use std::path::{Path, PathBuf};

use super::DocBase;

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

/// Points the general project at the folder of an old wiki, as the bases
/// made before the wiki was retired (W6) did; only the one-time import of
/// its files and of the chat's files reads it.
pub fn point_general_project_at(base: &DocBase, folder: &Path) {
    base.conn()
        .execute(
            "UPDATE project SET path = ?1 WHERE kind = 'general'",
            [folder.to_string_lossy().replace('\\', "/")],
        )
        .unwrap();
}

/// The folder of the old wiki the general project points at, if any.
pub fn general_project_path(base: &DocBase) -> Option<String> {
    base.conn()
        .query_row("SELECT path FROM project WHERE kind = 'general'", [], |row| row.get::<_, Option<String>>(0))
        .ok()
        .flatten()
}
