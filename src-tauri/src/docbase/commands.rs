//! What the screens can ask the document base.
//!
//! In the private space everything is visible, so every command passes
//! `Visible::All`. The shared space will compute it per person here,
//! without the queries below changing.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::Serialize;
use tauri::{Emitter, Manager, State};

use super::bridge::{self, GeneralProject, WikiSource};
use super::labels::{Label, LabelGroup, LabelRule};
use super::origins::Origin;
use super::query::{Activity, DocumentDetail, DocumentFilter, DocumentPage, Health, OriginSummary};
use super::tree::FolderListing;
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
            bridge::share(base.clone());
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
    let base = state.base()?;
    let origin = base.add_origin(&name, Path::new(&folder), &convention)?;
    // The general project's wiki, when there is one, gets the new origin
    // as a source too. Failing to mount it does not undo the origin: the
    // Orígenes screen shows the reason.
    if let Err(error) = base.sync_general_mounts() {
        eprintln!("[docbase] {error}");
    }
    state.wake.request_sweep();
    Ok(origin)
}

#[tauri::command(async)]
pub fn docbase_remove_origin(state: State<'_, DocBaseState>, origin_id: String) -> Result<(), String> {
    let base = state.base()?;
    base.unmount_from_general(&origin_id)?;
    base.remove_origin(&origin_id)
}

#[tauri::command(async)]
pub fn docbase_general_project(state: State<'_, DocBaseState>) -> Result<Option<GeneralProject>, String> {
    state.base()?.general_project()
}

/// Makes a wiki project the general project of the space: every origin
/// becomes a source of it, without copying anything. Nothing is ingested
/// by this.
#[tauri::command(async)]
pub fn docbase_link_general_project(
    state: State<'_, DocBaseState>,
    project_path: String,
    name: String,
) -> Result<GeneralProject, String> {
    state
        .base()?
        .link_general_project(Path::new(&project_path), &name)
}

/// Makes sure the general project has its wiki in the app's workspace
/// and returns it. Creates nothing but the empty project the first time.
#[tauri::command(async)]
pub fn docbase_ensure_default_project(
    state: State<'_, DocBaseState>,
    workspace: String,
) -> Result<GeneralProject, String> {
    state.base()?.ensure_default_project(Path::new(&workspace))
}

#[tauri::command(async)]
pub fn docbase_set_wiki_active(state: State<'_, DocBaseState>, active: bool) -> Result<(), String> {
    state.base()?.set_wiki_active(active)
}

#[tauri::command(async)]
pub fn docbase_unlink_general_project(state: State<'_, DocBaseState>) -> Result<(), String> {
    state.base()?.unlink_general_project()
}

/// The files of the base as sources of the general project's wiki.
#[tauri::command(async)]
pub fn docbase_wiki_sources(state: State<'_, DocBaseState>) -> Result<Vec<WikiSource>, String> {
    state.base()?.wiki_sources()
}

/// Asks for a sweep of every origin now; the result arrives as
/// `docbase://changed` events.
#[tauri::command(async)]
pub fn docbase_rescan(state: State<'_, DocBaseState>) -> Result<(), String> {
    state.base()?;
    state.wake.request_sweep();
    Ok(())
}

/// What the Inicio screen shows of the base.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Home {
    pub health: Health,
    pub origins: Vec<OriginSummary>,
    pub activity: Vec<Activity>,
}

#[tauri::command(async)]
pub fn docbase_home(state: State<'_, DocBaseState>) -> Result<Home, String> {
    let base = state.base()?;
    Ok(Home {
        health: base.health(&Visible::All)?,
        origins: base.origin_summaries(&Visible::All)?,
        activity: base.activity(&Visible::All, 12)?,
    })
}

#[tauri::command(async)]
pub fn docbase_list_documents(
    state: State<'_, DocBaseState>,
    filter: Option<DocumentFilter>,
) -> Result<DocumentPage, String> {
    state
        .base()?
        .documents(&Visible::All, &filter.unwrap_or_default())
}

#[tauri::command(async)]
pub fn docbase_get_document(
    state: State<'_, DocBaseState>,
    location_id: i64,
) -> Result<Option<DocumentDetail>, String> {
    state.base()?.document(&Visible::All, location_id)
}

#[tauri::command(async)]
pub fn docbase_label_groups(state: State<'_, DocBaseState>) -> Result<Vec<LabelGroup>, String> {
    state.base()?.label_groups()
}

/// The label with that name in that group (none: it stands alone),
/// created when it does not exist yet.
#[tauri::command(async)]
pub fn docbase_ensure_label(
    state: State<'_, DocBaseState>,
    group: Option<String>,
    name: String,
) -> Result<Label, String> {
    state.base()?.ensure_label(group.as_deref(), &name)
}

#[tauri::command(async)]
pub fn docbase_rename_label(state: State<'_, DocBaseState>, label_id: i64, name: String) -> Result<(), String> {
    state.base()?.rename_label(label_id, &name)
}

#[tauri::command(async)]
pub fn docbase_delete_label(state: State<'_, DocBaseState>, label_id: i64) -> Result<(), String> {
    state.base()?.delete_label(label_id)
}

#[tauri::command(async)]
pub fn docbase_rename_group(state: State<'_, DocBaseState>, group_id: i64, name: String) -> Result<(), String> {
    state.base()?.rename_group(group_id, &name)
}

#[tauri::command(async)]
pub fn docbase_delete_group(state: State<'_, DocBaseState>, group_id: i64) -> Result<(), String> {
    state.base()?.delete_group(group_id)
}

#[tauri::command(async)]
pub fn docbase_label_rules(state: State<'_, DocBaseState>, origin_id: String) -> Result<Vec<LabelRule>, String> {
    state.base()?.label_rules(&origin_id)
}

/// "Everything under this folder carries this label."
#[tauri::command(async)]
pub fn docbase_add_fixed_rule(
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
    label_id: i64,
) -> Result<(), String> {
    state.base()?.add_fixed_rule(&origin_id, &folder, label_id)
}

/// "Each subfolder of this folder is a … (group), level by level."
/// An empty list removes that rule from the folder.
#[tauri::command(async)]
pub fn docbase_set_levels_rule(
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
    groups: Vec<String>,
) -> Result<(), String> {
    state.base()?.set_levels_rule(&origin_id, &folder, &groups)
}

#[tauri::command(async)]
pub fn docbase_remove_rule(state: State<'_, DocBaseState>, rule_id: i64) -> Result<(), String> {
    state.base()?.remove_rule(rule_id)
}

/// Puts a label on one file or takes it off, whatever the rules say.
#[tauri::command(async)]
pub fn docbase_set_location_label(
    state: State<'_, DocBaseState>,
    location_id: i64,
    label_id: i64,
    on: bool,
) -> Result<(), String> {
    state.base()?.set_location_label(location_id, label_id, on)
}

/// The folders and files directly inside one folder of an origin.
#[tauri::command(async)]
pub fn docbase_list_folder(
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
) -> Result<FolderListing, String> {
    state.base()?.list_folder(&Visible::All, &origin_id, &folder)
}

/// Leaves a file or a folder out of the base, or brings it back. Nothing
/// on disk is touched.
#[tauri::command(async)]
pub fn docbase_set_excluded(
    state: State<'_, DocBaseState>,
    origin_id: String,
    rel_path: String,
    is_folder: bool,
    excluded: bool,
) -> Result<(), String> {
    state
        .base()?
        .set_excluded(&origin_id, &rel_path, is_folder, excluded)?;
    // Bringing something back gives the worker files to read.
    state.wake.request_sweep();
    Ok(())
}

/// Which of these sources of a wiki project the user left out of the base.
#[tauri::command(async)]
pub fn docbase_excluded_sources(
    state: State<'_, DocBaseState>,
    project_path: String,
    source_paths: Vec<String>,
) -> Result<Vec<String>, String> {
    state
        .base()?
        .excluded_sources(Path::new(&project_path), &source_paths)
}

/// Shows a folder of an origin in the system's file manager.
#[tauri::command(async)]
pub fn docbase_open_folder(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    origin_id: String,
    folder: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = state
        .base()?
        .folder_path(&origin_id, &folder)?
        .ok_or_else(|| "Ese origen ya no existe".to_string())?;
    if !Path::new(&path).is_dir() {
        return Err(format!("No se puede abrir '{path}': la carpeta no está disponible ahora"));
    }
    app.opener()
        .open_path(path.clone(), None::<&str>)
        .map_err(|e| format!("No se pudo abrir '{path}': {e}"))
}

/// Opens a file of the inventory with the program the system uses for
/// it, or shows it in its folder. Only locations the base knows can be
/// opened this way.
#[tauri::command(async)]
pub fn docbase_open_location(
    app: tauri::AppHandle,
    state: State<'_, DocBaseState>,
    location_id: i64,
    reveal: bool,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let path = state
        .base()?
        .location_path(&Visible::All, location_id)?
        .ok_or_else(|| "Ese archivo ya no está en el inventario".to_string())?;
    if !Path::new(&path).exists() {
        return Err(format!("No se puede abrir '{path}': el archivo no está disponible ahora"));
    }
    let opened = if reveal {
        app.opener().reveal_item_in_dir(&path)
    } else {
        app.opener().open_path(path.clone(), None::<&str>)
    };
    opened.map_err(|e| format!("No se pudo abrir '{path}': {e}"))
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
