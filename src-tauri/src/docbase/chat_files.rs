//! The chat's history and preferences live in the app's data folder
//! (D-415), not in the folder of a wiki project. Once per base, while the
//! general project still has its folder, its chat files (`.llm-wiki/`:
//! `conversations.json`, `chats/*.json`, `chat-preferences.json` and the
//! old `chat-history.json`) are copied there. Nothing is removed from the
//! project's folder, and a file already in the data folder is never
//! replaced.

use std::path::{Path, PathBuf};

use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use super::{sql_err, DocBase};

const COPIED_SETTING: &str = "chat-files-copied";
/// The folder of the chat inside the app's data folder.
pub const CHAT_FOLDER: &str = "chat";
const LOOSE_FILES: [&str; 3] = ["conversations.json", "chat-preferences.json", "chat-history.json"];

/// What the copy found and copied.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatCopy {
    /// Done before, or no folder to read: nothing was read.
    pub skipped: bool,
    /// Conversations (files of `chats/`) copied.
    pub conversations: usize,
    /// Every file copied (conversations, their list, the preferences).
    pub files: usize,
    /// Files left as they were because the data folder already had them.
    pub kept: usize,
}

/// Copies the chat files of `project/.llm-wiki` into `to`, without
/// replacing what `to` already has.
pub fn copy_chat_files(project: &Path, to: &Path) -> Result<ChatCopy, String> {
    let from = project.join(".llm-wiki");
    let mut report = ChatCopy::default();
    let mut copy = |source: PathBuf, target: PathBuf, conversation: bool| -> Result<(), String> {
        if !source.is_file() {
            return Ok(());
        }
        if target.exists() {
            report.kept += 1;
            return Ok(());
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("no se pudo crear '{}': {e}", parent.display()))?;
        }
        std::fs::copy(&source, &target).map_err(|e| format!("no se pudo copiar '{}': {e}", source.display()))?;
        report.files += 1;
        if conversation {
            report.conversations += 1;
        }
        Ok(())
    };
    for name in LOOSE_FILES {
        copy(from.join(name), to.join(name), false)?;
    }
    if let Ok(entries) = std::fs::read_dir(from.join("chats")) {
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.to_lowercase().ends_with(".json"))
            .collect();
        names.sort();
        for name in names {
            copy(from.join("chats").join(&name), to.join("chats").join(&name), true)?;
        }
    }
    Ok(report)
}

impl DocBase {
    /// The copy, once: from the folder of the general project when it is
    /// still there; nothing otherwise (and it will look again).
    pub fn copy_chat_files_once(&self, to: &Path) -> Result<ChatCopy, String> {
        let done: Option<String> = self
            .conn()
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [COPIED_SETTING], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        if done.is_some() {
            return Ok(ChatCopy { skipped: true, ..Default::default() });
        }
        let root: Option<String> = self
            .conn()
            .query_row("SELECT path FROM project WHERE kind = 'general'", [], |row| row.get::<_, Option<String>>(0))
            .optional()
            .map_err(sql_err)?
            .flatten();
        let Some(root) = root.map(PathBuf::from).filter(|root| root.join(".llm-wiki").is_dir()) else {
            return Ok(ChatCopy { skipped: true, ..Default::default() });
        };
        let report = copy_chat_files(&root, to)?;
        self.conn()
            .execute(
                "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
                params![COPIED_SETTING, serde_json::to_string(&report).map_err(|e| e.to_string())?],
            )
            .map_err(sql_err)?;
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("micelya-chat-{name}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn project_with_chats() -> PathBuf {
        let project = scratch("project");
        let meta = project.join(".llm-wiki");
        std::fs::create_dir_all(meta.join("chats")).unwrap();
        std::fs::write(meta.join("conversations.json"), r#"[{"id":"c1","title":"Prueba"}]"#).unwrap();
        std::fs::write(meta.join("chat-preferences.json"), r#"{"agentMode":"deep"}"#).unwrap();
        std::fs::write(meta.join("chats").join("c1.json"), "[]").unwrap();
        std::fs::write(meta.join("chats").join("c2.json"), "[]").unwrap();
        std::fs::write(meta.join("chats").join("nota.txt"), "no").unwrap();
        std::fs::write(meta.join("review.json"), "[]").unwrap();
        project
    }

    #[test]
    fn the_chat_files_are_copied_once_and_never_replace_or_remove_anything() {
        let project = project_with_chats();
        let to = scratch("data").join(CHAT_FOLDER);
        std::fs::create_dir_all(to.join("chats")).unwrap();
        std::fs::write(to.join("chats").join("c2.json"), "[1]").unwrap();
        let base = DocBase::in_memory().unwrap();
        base.conn()
            .execute(
                "INSERT INTO project (id, name, kind, path, created_at) VALUES ('g', 'General', 'general', ?1, 0)",
                [project.to_string_lossy().to_string()],
            )
            .unwrap();
        let report = base.copy_chat_files_once(&to).unwrap();
        assert_eq!(report, ChatCopy { skipped: false, conversations: 1, files: 3, kept: 1 });
        assert_eq!(std::fs::read_to_string(to.join("chats").join("c2.json")).unwrap(), "[1]");
        assert!(to.join("chat-preferences.json").is_file());
        assert!(!to.join("review.json").exists());
        assert!(project.join(".llm-wiki").join("chats").join("c1.json").is_file());
        // Once: a second time reads nothing.
        std::fs::remove_file(to.join("conversations.json")).unwrap();
        assert!(base.copy_chat_files_once(&to).unwrap().skipped);
        assert!(!to.join("conversations.json").exists());
    }

    #[test]
    fn without_the_project_folder_nothing_is_marked_and_it_looks_again() {
        let base = DocBase::in_memory().unwrap();
        let to = scratch("empty");
        assert!(base.copy_chat_files_once(&to).unwrap().skipped);
        let marked: Option<String> = base
            .conn()
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [COPIED_SETTING], |row| row.get(0))
            .optional()
            .unwrap();
        assert!(marked.is_none());
    }

    /// The copy from a copy of a real wiki folder (`MICELYA_CHAT_FROM`):
    /// every chat file there is in the data folder after it.
    #[test]
    #[ignore]
    fn the_chat_files_of_a_real_folder_are_all_copied() {
        let Ok(from) = std::env::var("MICELYA_CHAT_FROM") else { return };
        let from = PathBuf::from(from);
        let to = scratch("real").join(CHAT_FOLDER);
        let report = copy_chat_files(&from, &to).unwrap();
        let in_source = std::fs::read_dir(from.join(".llm-wiki").join("chats"))
            .map(|entries| entries.filter_map(Result::ok).filter(|e| e.file_name().to_string_lossy().ends_with(".json")).count())
            .unwrap_or(0);
        let loose = LOOSE_FILES.iter().filter(|name| from.join(".llm-wiki").join(name).is_file()).count();
        eprintln!("chat copy: {report:?}; conversations in the source: {in_source}; loose files: {loose}");
        assert_eq!(report.conversations, in_source);
        assert_eq!(report.files, in_source + loose);
        assert_eq!(report.kept, 0);
    }
}
