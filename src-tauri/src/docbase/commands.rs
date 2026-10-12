//! What the screens can ask the document base.
//!
//! In the private space everything is visible, so every command passes
//! `Visible::All`. The shared space will compute it per person here,
//! without the queries below changing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use super::origins::Origin;
use super::query::OriginSummary;
use super::worker::{self, Wake};
use super::{DocBase, Visible, PRIVATE_DB_FILE};

/// Event the screens listen to in order to refresh.
pub const CHANGED_EVENT: &str = "docbase://changed";

/// The base of the private space, or why it could not be opened.
pub struct DocBaseState {
    base: Result<Arc<DocBase>, String>,
    wake: Arc<Wake>,
}

impl DocBaseState {
    fn base(&self) -> Result<&Arc<DocBase>, String> {
        self.base
            .as_ref()
            .map_err(|error| format!("La base documental no está disponible: {error}"))
    }
}

/// Opens the private space's base in the app's local data folder and
/// starts its background worker.
pub fn start(app: &tauri::AppHandle) {
    let wake = Arc::new(Wake::default());
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("no se encontró la carpeta de datos de la app: {e}"))
        .and_then(|dir| DocBase::open(&dir.join(PRIVATE_DB_FILE)))
        .map(Arc::new);
    match &base {
        Ok(base) => {
            let handle = app.clone();
            worker::spawn(base.clone(), wake.clone(), move || {
                let _ = handle.emit(CHANGED_EVENT, ());
            });
        }
        Err(error) => eprintln!("[docbase] {error}"),
    }
    app.manage(DocBaseState { base, wake });
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderPreview {
    /// Suggested origin name: the folder's own name.
    pub name: String,
    /// A few example paths, one folder level per element, to help the
    /// user say what each level means.
    pub examples: Vec<Vec<String>>,
}

#[tauri::command(async)]
pub fn docbase_list_origins(state: State<'_, DocBaseState>) -> Result<Vec<OriginSummary>, String> {
    state.base()?.origin_summaries(&Visible::All)
}

#[tauri::command(async)]
pub fn docbase_preview_folder(folder: String) -> Result<FolderPreview, String> {
    let root = PathBuf::from(&folder);
    if !root.is_dir() {
        return Err(format!("'{folder}' no es una carpeta que se pueda leer"));
    }
    let name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| folder.clone());
    Ok(FolderPreview {
        name,
        examples: example_paths(&root, 3, 4),
    })
}

#[tauri::command(async)]
pub fn docbase_add_origin(
    state: State<'_, DocBaseState>,
    name: String,
    folder: String,
    convention: Vec<String>,
) -> Result<Origin, String> {
    let origin = state
        .base()?
        .add_origin(&name, Path::new(&folder), &convention)?;
    state.wake.request_sweep();
    Ok(origin)
}

#[tauri::command(async)]
pub fn docbase_set_origin_convention(
    state: State<'_, DocBaseState>,
    origin_id: String,
    convention: Vec<String>,
) -> Result<(), String> {
    state.base()?.set_origin_convention(&origin_id, &convention)
}

#[tauri::command(async)]
pub fn docbase_remove_origin(state: State<'_, DocBaseState>, origin_id: String) -> Result<(), String> {
    state.base()?.remove_origin(&origin_id)
}

/// Asks for a sweep of every origin now; the result arrives as
/// `docbase://changed` events.
#[tauri::command(async)]
pub fn docbase_rescan(state: State<'_, DocBaseState>) -> Result<(), String> {
    state.base()?;
    state.wake.request_sweep();
    Ok(())
}

/// Up to `count` paths of nested folders under `root`, `depth` levels
/// deep at most, following the first folders in name order.
fn example_paths(root: &Path, count: usize, depth: usize) -> Vec<Vec<String>> {
    fn subfolders(dir: &Path) -> Vec<(String, PathBuf)> {
        let mut found: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
            .map(|entry| (entry.file_name().to_string_lossy().to_string(), entry.path()))
            .filter(|(name, _)| !name.starts_with('.') && !name.starts_with('$'))
            .collect();
        found.sort_by_key(|(name, _)| name.to_lowercase());
        found
    }

    let mut examples = Vec::new();
    for (name, path) in subfolders(root).into_iter().take(count) {
        let mut levels = vec![name];
        let mut current = path;
        while levels.len() < depth {
            let Some((name, path)) = subfolders(&current).into_iter().next() else {
                break;
            };
            levels.push(name);
            current = path;
        }
        examples.push(levels);
    }
    examples
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    #[test]
    fn folder_examples_follow_the_first_folders_down() {
        let dir = TempDir::new();
        dir.folder("BGH/Reclamo facturas/Anexos");
        dir.folder("Aeros/Contrato");
        dir.folder(".oculta/x");
        dir.write("suelto.txt", b"x");

        let examples = example_paths(dir.path(), 3, 4);
        assert_eq!(
            examples,
            vec![
                vec!["Aeros".to_string(), "Contrato".to_string()],
                vec!["BGH".to_string(), "Reclamo facturas".to_string(), "Anexos".to_string()],
            ]
        );
    }
}
