//! Micelya's work folder (D-450, D-451): where the app keeps what it brings
//! by itself. The web clips go to «Recortes web» and the records of the
//! judicial portals to «Expedientes». The folder is an origin of the base,
//! so what is saved there is inventoried like any document; its folders
//! are part of it, never origins of their own.
//!
//! It is chosen in Ajustes → Carpeta de trabajo and kept in `app-state.json`
//! under `workspaceDir` (the key the old workspace of projects used, so
//! the folder already there stays). Without one, `<Documentos>\Micelya`.
//! Every part of the app asks for these folders here, and only here.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_store::StoreExt;

use crate::docbase::commands::{DocBaseState, CHANGED_EVENT};
use crate::docbase::work_origin::{NewFiles, WorkOrigin};
use crate::docbase::DocBase;

/// The app's settings file, the same one the screens use.
const STORE_FILE: &str = "app-state.json";
/// Where the folder is kept in it.
const STORE_KEY: &str = "workspaceDir";

/// The web clips (`clip_server`).
pub const CLIPS: &str = "Recortes web";
/// The records brought from the portals (`portals`).
pub const RECORDS: &str = "Expedientes";
/// The app's own folders inside the work folder.
pub const APP_FOLDERS: &[&str] = &[CLIPS, RECORDS];
/// The names its origin can take, the first free one.
const ORIGIN_NAMES: &[&str] = &["Carpeta de trabajo", "Carpeta de trabajo de Micelya", "Micelya"];
/// When the person last saw the new clips (`docbase_setting`).
const CLIPS_SEEN: &str = "clips-seen-at";

/// The work folder: the one chosen, or the one by default.
pub fn folder(app: &AppHandle) -> Result<PathBuf, String> {
    match chosen(app) {
        Some(folder) => Ok(folder),
        None => default_folder(app),
    }
}

/// `<carpeta de trabajo>\Expedientes`.
pub fn records_folder(app: &AppHandle) -> Result<PathBuf, String> {
    folder(app).map(|work| work.join(RECORDS))
}

/// `<carpeta de trabajo>\Recortes web`.
pub fn clips_folder(app: &AppHandle) -> Result<PathBuf, String> {
    folder(app).map(|work| work.join(CLIPS))
}

fn chosen(app: &AppHandle) -> Option<PathBuf> {
    let store = app.store(STORE_FILE).ok()?;
    let value = store.get(STORE_KEY)?;
    let text = value.as_str()?.trim();
    (!text.is_empty()).then(|| normalize(text))
}

fn default_folder(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .document_dir()
        .map(|documents| documents.join("Micelya"))
        .or_else(|_| app.path().app_data_dir().map(|data| data.join("Micelya")))
        .map_err(|e| format!("No se encontró dónde poner la carpeta de trabajo: {e}"))
}

/// The folder as Windows writes it (the screens keep paths with `/`).
fn normalize(text: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(text.replace('/', "\\"))
    } else {
        PathBuf::from(text)
    }
}

fn keep(app: &AppHandle, folder: &Path) -> Result<(), String> {
    let store = app.store(STORE_FILE).map_err(|e| format!("No se pudo abrir los ajustes: {e}"))?;
    store.set(STORE_KEY, serde_json::Value::String(folder.to_string_lossy().replace('\\', "/")));
    store.save().map_err(|e| format!("No se pudieron guardar los ajustes: {e}"))
}

/// Creates the folder and the app's folders inside it.
pub fn prepare(folder: &Path) -> Result<(), String> {
    for sub in APP_FOLDERS {
        let path = folder.join(sub);
        fs::create_dir_all(&path).map_err(|e| format!("No se pudo crear {}: {e}", path.display()))?;
    }
    Ok(())
}

/// The work folder's origin in the base, making it one if needed.
pub fn register(base: &DocBase, folder: &Path) -> Result<WorkOrigin, String> {
    base.ensure_work_origin(ORIGIN_NAMES, folder, APP_FOLDERS)
}

/// What Ajustes shows of the work folder.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkFolderStatus {
    pub path: String,
    pub clips: String,
    pub records: String,
    /// The origin of the base that holds it.
    pub origin_name: Option<String>,
    /// That origin is the folder itself (not one that holds it).
    pub origin_own: bool,
    /// Why it is not ready, in words.
    pub error: Option<String>,
}

/// Makes sure the folder exists with its folders and is an origin, and
/// says how it is. Asks for a sweep when its origin is new or changed.
fn ensure_at(app: &AppHandle, docbase: &DocBaseState, folder: &Path) -> Result<WorkOrigin, String> {
    fs::create_dir_all(folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
    let base = docbase.shared()?;
    let before = base.origins(&crate::docbase::Visible::All)?;
    // Its folders are made once the base takes it: a folder it does not
    // take is left as it was.
    let work = register(&base, folder)?;
    prepare(folder)?;
    let changed = !before
        .iter()
        .any(|o| o.id == work.origin.id && o.location == work.origin.location)
        || !work.joined.is_empty();
    if changed {
        docbase.request_sweep();
        let _ = app.emit(CHANGED_EVENT, ());
    }
    Ok(work)
}

fn status_of(folder: &Path, work: Result<WorkOrigin, String>) -> WorkFolderStatus {
    let (origin_name, origin_own, error) = match work {
        Ok(work) => (Some(work.origin.name), work.own, None),
        Err(error) => (None, false, Some(error)),
    };
    WorkFolderStatus {
        path: folder.to_string_lossy().to_string(),
        clips: folder.join(CLIPS).to_string_lossy().to_string(),
        records: folder.join(RECORDS).to_string_lossy().to_string(),
        origin_name,
        origin_own,
        error,
    }
}

/// When the app opens: the folder exists, with its folders, and is an
/// origin. The first time, the folder by default is kept as the chosen one.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    std::thread::spawn(move || {
        let folder = match folder(&app) {
            Ok(folder) => folder,
            Err(error) => return eprintln!("[work-folder] {error}"),
        };
        if chosen(&app).is_none() {
            if let Err(error) = keep(&app, &folder) {
                eprintln!("[work-folder] {error}");
            }
        }
        let docbase = app.state::<DocBaseState>();
        match ensure_at(&app, &docbase, &folder) {
            Ok(work) if !work.joined.is_empty() => {
                eprintln!("[work-folder] {} joined the work folder's origin", work.joined.join(", "))
            }
            Ok(_) => {}
            Err(error) => eprintln!("[work-folder] {error}"),
        }
    });
}

/// The work folder and how it is in the base. It also makes sure it is
/// there: the folders, and the origin.
#[tauri::command(async)]
pub fn work_folder_status(app: AppHandle, docbase: State<'_, DocBaseState>) -> Result<WorkFolderStatus, String> {
    let folder = folder(&app)?;
    let work = ensure_at(&app, &docbase, &folder);
    Ok(status_of(&folder, work))
}

/// Changes the work folder. It is kept only if the base can take it as an
/// origin; otherwise the reason comes back and nothing changes.
#[tauri::command(async)]
pub fn work_folder_set(app: AppHandle, docbase: State<'_, DocBaseState>, path: String) -> Result<WorkFolderStatus, String> {
    let folder = normalize(path.trim());
    if !folder.is_absolute() {
        return Err("Elegí una carpeta con su ruta completa".to_string());
    }
    let work = ensure_at(&app, &docbase, &folder)?;
    keep(&app, &folder)?;
    Ok(status_of(&folder, Ok(work)))
}

/// Opens the work folder, or one of its folders (`clips`, `records`).
#[tauri::command(async)]
pub fn work_folder_open(app: AppHandle, which: Option<String>) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    let work = folder(&app)?;
    let target = match which.as_deref() {
        None | Some("") => work,
        Some("clips") => work.join(CLIPS),
        Some("records") => work.join(RECORDS),
        Some(other) => return Err(format!("No hay una carpeta «{other}» en la carpeta de trabajo")),
    };
    fs::create_dir_all(&target).map_err(|e| format!("No se pudo crear {}: {e}", target.display()))?;
    app.opener()
        .open_path(target.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| format!("No se pudo abrir la carpeta: {e}"))
}

/// The clips the base inventoried since the person last saw them.
#[tauri::command(async)]
pub fn work_folder_new_clips(app: AppHandle, docbase: State<'_, DocBaseState>) -> Result<NewFiles, String> {
    docbase.shared()?.files_new_since(&clips_folder(&app)?, CLIPS_SEEN, 20)
}

/// The person saw the new clips: they are no longer new.
#[tauri::command(async)]
pub fn work_folder_clips_seen(app: AppHandle, docbase: State<'_, DocBaseState>) -> Result<(), String> {
    docbase.shared()?.mark_seen(CLIPS_SEEN)?;
    let _ = app.emit(CHANGED_EVENT, ());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;
    use crate::docbase::Visible;

    #[test]
    fn the_folder_is_made_with_its_folders_and_is_one_origin_with_them_inside() {
        let dir = TempDir::new();
        let work = dir.path().join("trabajo");
        prepare(&work).unwrap();
        assert!(work.join(CLIPS).is_dir() && work.join(RECORDS).is_dir());
        let base = DocBase::in_memory().unwrap();
        let origin = register(&base, &work).unwrap();
        assert_eq!(origin.origin.name, "Carpeta de trabajo");
        // The records' folder and the clips' folder are inside it: no
        // origin of their own, and no other origin can be made there.
        assert!(base.add_origin("Expedientes", &work.join(RECORDS), &[]).is_err());
        assert!(base.add_origin("Recortes web", &work.join(CLIPS), &[]).is_err());
        assert_eq!(base.origins(&Visible::All).unwrap().len(), 1);
        assert_eq!(normalize("C:/Usuarios/alguien/carpeta").to_string_lossy().contains('/'), !cfg!(windows));
    }
}
