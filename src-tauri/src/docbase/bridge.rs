//! The bridge between the base and the wiki of the general project.
//!
//! The wiki asks the base three things: which sources exist, what the
//! text of one is, and what changed. The bridge answers the first two:
//!
//! - *which sources exist*: every origin of the space is mounted in the
//!   general project's wiki, so the wiki sees the origins as its sources
//!   without copying a single file;
//! - *the text of one*: the wiki takes the text the base already has,
//!   and a scan recognised for the wiki goes back to the base, so it is
//!   never recognised twice in the space.
//!
//! What changed is still noticed by the wiki project's own watcher,
//! which already covers mounted folders.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::source_volume::{SourceVolume, SOURCES_PREFIX};

use super::origins::{self, ensure_general_project, Origin};
use super::{sql_err, DocBase, Visible};

/// The base of the private space, for the parts of the app that read
/// sources and have no handle to the app state.
static SHARED: OnceLock<Arc<DocBase>> = OnceLock::new();

pub fn share(base: Arc<DocBase>) {
    let _ = SHARED.set(base);
}

fn shared() -> Option<&'static Arc<DocBase>> {
    SHARED.get()
}

/// An origin as the general project's wiki sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgedOrigin {
    pub origin_id: String,
    pub name: String,
    /// The folder under `raw/sources` it appears as, when it is mounted.
    pub mounted_as: Option<String>,
    /// Why it could not be mounted, when that is the case.
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneralProject {
    pub id: String,
    pub name: String,
    /// The wiki project folder, once one was assigned.
    pub path: Option<String>,
    pub members: String,
    pub origins: Vec<BridgedOrigin>,
}

/// A file of the base as a source of the general project's wiki.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WikiSource {
    pub origin_id: String,
    /// Path inside the wiki project: `raw/sources/<origin>/<file>`.
    pub source_path: String,
    /// What the wiki calls the source: the path under `raw/sources`.
    pub identity: String,
    pub ext: String,
}

impl DocBase {
    /// The general project of the space; `None` before the first origin.
    pub fn general_project(&self) -> Result<Option<GeneralProject>, String> {
        let (project, origins) = {
            let conn = self.conn();
            let project: Option<(String, String, Option<String>, String)> = conn
                .query_row(
                    "SELECT id, name, path, members FROM project WHERE kind = 'general'",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
                .optional()
                .map_err(sql_err)?;
            (project, origins::list(&conn, &Visible::All)?)
        };
        let Some((id, name, path, members)) = project else {
            return Ok(None);
        };
        let mounts = match &path {
            Some(path) => SourceVolume::open(path).map(|volume| volume.mounts().to_vec()),
            None => Ok(Vec::new()),
        };
        let bridged = origins
            .iter()
            .map(|origin| {
                let (mounted_as, problem) = match (&path, &mounts) {
                    (None, _) => (None, None),
                    (Some(_), Err(error)) => (None, Some(error.clone())),
                    (Some(_), Ok(mounts)) => (
                        mounts
                            .iter()
                            .find(|mount| same_folder(&mount.location, &origin.location))
                            .map(|mount| mount.name.clone()),
                        None,
                    ),
                };
                BridgedOrigin {
                    origin_id: origin.id.clone(),
                    name: origin.name.clone(),
                    mounted_as,
                    problem,
                }
            })
            .collect();
        Ok(Some(GeneralProject {
            id,
            name,
            path,
            members,
            origins: bridged,
        }))
    }

    /// Makes the wiki project at `project_path` the general project of
    /// the space and mounts every origin in it.
    pub fn link_general_project(&self, project_path: &Path, name: &str) -> Result<GeneralProject, String> {
        if !project_path.join("wiki").is_dir() || !project_path.join("schema.md").is_file() {
            return Err(format!("'{}' no es un proyecto de wiki", project_path.display()));
        }
        {
            let conn = self.conn();
            let id = ensure_general_project(&conn)?;
            conn.execute(
                "UPDATE project SET path = ?1, name = ?2 WHERE id = ?3",
                params![project_path.to_string_lossy().replace('\\', "/"), name.trim(), id],
            )
            .map_err(sql_err)?;
        }
        self.sync_general_mounts()
    }

    /// Stops using a wiki project as the general project. The folders
    /// mounted for the base are unmounted from it; its pages stay.
    pub fn unlink_general_project(&self) -> Result<(), String> {
        let Some(project) = self.general_project()? else {
            return Ok(());
        };
        if let Some(path) = &project.path {
            let origins = self.origins(&Visible::All)?;
            let mut volume = SourceVolume::open(path)?;
            for origin in &origins {
                unmount(&mut volume, origin)?;
            }
        }
        self.conn()
            .execute("UPDATE project SET path = NULL WHERE kind = 'general'", [])
            .map_err(sql_err)?;
        Ok(())
    }

    /// Mounts in the general project's wiki every origin that is not
    /// there yet. An origin that cannot be mounted is reported with the
    /// reason; the others are mounted all the same.
    pub fn sync_general_mounts(&self) -> Result<GeneralProject, String> {
        let project = self
            .general_project()?
            .ok_or_else(|| "Todavía no hay carpetas incorporadas".to_string())?;
        let Some(path) = project.path.clone() else {
            return Ok(project);
        };
        let origins = self.origins(&Visible::All)?;
        let mut volume = SourceVolume::open(&path)?;
        let mut problems: Vec<(String, String)> = Vec::new();
        for origin in &origins {
            let mounted = volume
                .mounts()
                .iter()
                .any(|mount| same_folder(&mount.location, &origin.location));
            if mounted {
                continue;
            }
            if let Err(error) = volume.add_local_mount(&origin.name, &origin.root()) {
                problems.push((origin.id.clone(), error));
            }
        }
        let mut project = self
            .general_project()?
            .ok_or_else(|| "Todavía no hay carpetas incorporadas".to_string())?;
        for bridged in &mut project.origins {
            if let Some((_, error)) = problems.iter().find(|(id, _)| *id == bridged.origin_id) {
                bridged.problem = Some(error.clone());
            }
        }
        Ok(project)
    }

    /// Unmounts an origin from the general project's wiki, when there is
    /// one. Used before the base forgets the origin.
    pub fn unmount_from_general(&self, origin_id: &str) -> Result<(), String> {
        let Some(origin) = self.origin(origin_id)? else {
            return Ok(());
        };
        let Some(path) = self.general_project()?.and_then(|project| project.path) else {
            return Ok(());
        };
        unmount(&mut SourceVolume::open(&path)?, &origin)
    }

    /// Which sources the general project's wiki has: every file of the
    /// origins mounted in it that the wiki can ingest. Cloud shortcuts
    /// are left out: they hold no document.
    pub fn wiki_sources(&self) -> Result<Vec<WikiSource>, String> {
        let Some(project) = self.general_project()? else {
            return Ok(Vec::new());
        };
        let mut sources = Vec::new();
        let conn = self.conn();
        for origin in project.origins.iter().filter(|origin| origin.mounted_as.is_some()) {
            let mount = origin.mounted_as.as_deref().unwrap_or_default();
            let mut statement = conn
                .prepare("SELECT rel_path, ext FROM location WHERE origin_id = ?1 AND kind = 'file' ORDER BY rel_path")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([&origin.origin_id], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (rel_path, ext) = row.map_err(sql_err)?;
                let identity = format!("{mount}/{rel_path}");
                sources.push(WikiSource {
                    origin_id: origin.origin_id.clone(),
                    source_path: format!("{SOURCES_PREFIX}/{identity}"),
                    identity,
                    ext,
                });
            }
        }
        Ok(sources)
    }

    /// The hash and the state of the document at a path on disk, when
    /// that path is a file of some origin and the base read it as it is
    /// now (same size and modification time).
    fn known_file(&self, real: &Path) -> Result<Option<(String, String)>, String> {
        let Ok(meta) = std::fs::metadata(real) else {
            return Ok(None);
        };
        let mtime_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        let conn = self.conn();
        for origin in origins::list(&conn, &Visible::All)? {
            let Some(rel_path) = relative_to(&origin.root(), real) else {
                continue;
            };
            let found: Option<(String, String)> = conn
                .query_row(
                    "SELECT l.file_hash, c.state
                     FROM location l
                     JOIN file f ON f.hash = l.file_hash
                     JOIN content c ON c.id = f.content_id
                     WHERE l.origin_id = ?1 AND l.rel_path = ?2 COLLATE NOCASE
                       AND l.hash_state = 'done' AND l.size = ?3 AND l.mtime_ms = ?4",
                    params![origin.id, rel_path, meta.len() as i64, mtime_ms],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(sql_err)?;
            return Ok(found);
        }
        Ok(None)
    }
}

/// The whole text of the file at `real`, when the base already has it.
/// Never fails: without an answer the caller extracts the text itself.
pub fn text_for_source(real: &Path) -> Option<String> {
    let base = shared()?;
    let (hash, state) = base.known_file(real).ok().flatten()?;
    if state != "processed" {
        return None;
    }
    base.text_of_file(&hash).ok().flatten()
}

/// Hands the base the text just recognised for the file at `real`, if
/// that file is a document the base is still waiting to have recognised.
pub fn offer_recognized_text(real: &Path, text: &str, engine: &str) {
    let Some(base) = shared() else {
        return;
    };
    match base.known_file(real) {
        Ok(Some((hash, state))) if state == "needs-recognition" => {
            if let Err(error) = base.store_recognized_text(&hash, text, engine) {
                eprintln!("[docbase] could not keep recognised text: {error}");
            }
        }
        Ok(_) => {}
        Err(error) => eprintln!("[docbase] {error}"),
    }
}

fn unmount(volume: &mut SourceVolume, origin: &Origin) -> Result<(), String> {
    let mount_id = volume
        .mounts()
        .iter()
        .find(|mount| same_folder(&mount.location, &origin.location))
        .map(|mount| mount.id.clone());
    if let Some(id) = mount_id {
        volume.remove_mount(&id)?;
    }
    Ok(())
}

fn folder_parts(path: &str) -> Vec<String> {
    path.replace('\\', "/")
        .split('/')
        .filter(|part| !part.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Whether two folder paths name the same folder, the way Windows
/// compares them.
fn same_folder(a: &str, b: &str) -> bool {
    folder_parts(a) == folder_parts(b)
}

/// `path` relative to `root` with forward slashes, when it is under it.
fn relative_to(root: &Path, path: &Path) -> Option<String> {
    let clean = |p: &Path| -> PathBuf {
        // Canonical paths on Windows carry a `\\?\` prefix the origin's
        // own path does not have.
        PathBuf::from(p.to_string_lossy().trim_start_matches(r"\\?\").to_string())
    };
    let root_text = clean(root).to_string_lossy().replace('\\', "/");
    let path_text = clean(path).to_string_lossy().replace('\\', "/");
    let root_parts = folder_parts(&root_text);
    let path_parts: Vec<&str> = path_text.split('/').filter(|part| !part.is_empty()).collect();
    if path_parts.len() <= root_parts.len() {
        return None;
    }
    let under = path_parts
        .iter()
        .zip(&root_parts)
        .all(|(part, root_part)| part.to_lowercase() == *root_part);
    under.then(|| path_parts[root_parts.len()..].join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    const LETTER: &str = "Por la presente intimamos a ustedes al pago de las facturas adeudadas dentro del plazo \
        de diez días hábiles, bajo apercibimiento de iniciar las acciones legales que correspondan y de reclamar \
        los intereses devengados desde la fecha de vencimiento de cada una de ellas.";

    fn wiki_project(dir: &TempDir, name: &str) -> PathBuf {
        let root = dir.folder(name);
        dir.folder(&format!("{name}/wiki"));
        dir.folder(&format!("{name}/raw/sources"));
        dir.write(&format!("{name}/schema.md"), b"# schema");
        root
    }

    fn settle(base: &DocBase) {
        for origin in base.origins(&Visible::All).unwrap() {
            base.scan_origin(&origin.id).unwrap();
        }
        while base.hash_pending(50).unwrap() > 0 {}
        while base.extract_pending(50).unwrap() > 0 {}
    }

    #[test]
    fn linking_a_wiki_mounts_every_origin_without_copying_anything() {
        let dir = TempDir::new();
        dir.write("casos/BGH/carta.md", LETTER.as_bytes());
        dir.write("admin/factura.pdf", b"%PDF-");
        let project = wiki_project(&dir, "General");
        let base = DocBase::in_memory().unwrap();
        base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();
        base.add_origin("Administración", &dir.path().join("admin"), &[]).unwrap();

        // Before a wiki is assigned there is a general project without one.
        let before = base.general_project().unwrap().unwrap();
        assert!(before.path.is_none());
        assert!(before.origins.iter().all(|origin| origin.mounted_as.is_none()));

        let linked = base.link_general_project(&project, "General").unwrap();
        assert_eq!(linked.members, "open");
        assert_eq!(
            linked.origins.iter().map(|o| o.mounted_as.as_deref()).collect::<Vec<_>>(),
            [Some("Administración"), Some("Casos")]
        );
        // Nothing was copied into the project.
        assert_eq!(std::fs::read_dir(project.join("raw/sources")).unwrap().count(), 0);
        // The wiki reads the original through its logical path.
        let logical = project.join("raw/sources/Casos/BGH/carta.md");
        assert_eq!(
            crate::source_volume::resolve_path(&logical).unwrap(),
            dir.path().join("casos").join("BGH").join("carta.md")
        );
    }

    #[test]
    fn an_origin_added_later_is_mounted_and_a_removed_one_unmounted() {
        let dir = TempDir::new();
        dir.folder("casos");
        dir.folder("nube");
        let project = wiki_project(&dir, "General");
        let base = DocBase::in_memory().unwrap();
        base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();
        base.link_general_project(&project, "General").unwrap();

        let nube = base.add_origin("Nube", &dir.path().join("nube"), &[]).unwrap();
        let synced = base.sync_general_mounts().unwrap();
        assert!(synced.origins.iter().all(|origin| origin.mounted_as.is_some()));

        base.unmount_from_general(&nube.id).unwrap();
        base.remove_origin(&nube.id).unwrap();
        let mounts = SourceVolume::open(&project).unwrap().mounts().to_vec();
        assert_eq!(mounts.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), ["Casos"]);
    }

    #[test]
    fn an_origin_that_cannot_be_mounted_is_reported_and_the_rest_go_on() {
        let dir = TempDir::new();
        dir.folder("casos");
        dir.folder("nube");
        let project = wiki_project(&dir, "General");
        // The wiki already has its own copied folder with that name.
        dir.write("General/raw/sources/Casos/viejo.md", b"copiado antes");
        let base = DocBase::in_memory().unwrap();
        base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();
        base.add_origin("Nube", &dir.path().join("nube"), &[]).unwrap();

        let linked = base.link_general_project(&project, "General").unwrap();
        let casos = linked.origins.iter().find(|o| o.name == "Casos").unwrap();
        assert!(casos.mounted_as.is_none() && casos.problem.is_some());
        let nube = linked.origins.iter().find(|o| o.name == "Nube").unwrap();
        assert_eq!(nube.mounted_as.as_deref(), Some("Nube"));

        assert!(base.link_general_project(&dir.folder("no-es-wiki"), "X").is_err());
    }

    #[test]
    fn the_wiki_sources_are_the_files_of_the_mounted_origins() {
        let dir = TempDir::new();
        dir.write("casos/BGH/carta.md", LETTER.as_bytes());
        dir.write("casos/BGH/notas.gdoc", b"{}");
        let project = wiki_project(&dir, "General");
        let base = DocBase::in_memory().unwrap();
        base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();
        settle(&base);
        assert!(base.wiki_sources().unwrap().is_empty());

        base.link_general_project(&project, "General").unwrap();
        let sources = base.wiki_sources().unwrap();
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].source_path, "raw/sources/Casos/BGH/carta.md");
        assert_eq!(sources[0].identity, "Casos/BGH/carta.md");
    }

    #[test]
    fn the_base_answers_with_the_text_only_for_the_file_as_it_read_it() {
        let dir = TempDir::new();
        let file = dir.write("casos/BGH/carta.md", LETTER.as_bytes());
        let base = DocBase::in_memory().unwrap();
        base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();
        settle(&base);

        let (hash, state) = base.known_file(&file).unwrap().unwrap();
        assert_eq!(state, "processed");
        assert_eq!(base.text_of_file(&hash).unwrap().as_deref(), Some(LETTER));
        // Case and separators do not matter on Windows paths.
        let odd = PathBuf::from(file.to_string_lossy().replace("BGH", "bgh"));
        assert!(base.known_file(&odd).unwrap().is_some() || !cfg!(windows));

        // Edited since: what the base has is about another file.
        dir.write("casos/BGH/carta.md", format!("{LETTER} Posdata.").as_bytes());
        assert!(base.known_file(&file).unwrap().is_none());
        assert!(base.known_file(&dir.path().join("fuera.md")).unwrap().is_none());
    }

    /// The only test that installs the shared base: the lock is set once
    /// per process, and a base that knows none of their folders changes
    /// nothing for the other tests.
    #[test]
    fn a_scan_recognised_once_is_text_for_every_wiki_that_reads_it() {
        let dir = TempDir::new();
        let photo = dir.write("casos/BGH/foto de la carta.jpg", b"\xff\xd8 una foto");
        dir.write("casos/copias/la misma foto.jpg", b"\xff\xd8 una foto");
        let project = wiki_project(&dir, "General");
        let base = Arc::new(DocBase::in_memory().unwrap());
        base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();
        base.link_general_project(&project, "General").unwrap();
        settle(&base);
        share(base.clone());
        let read = |logical: &str| {
            let path = project.join(logical).to_string_lossy().to_string();
            tauri::async_runtime::block_on(crate::commands::fs::read_file(path, Some(false))).unwrap()
        };

        // Not recognised yet: the wiki gets the usual placeholder.
        assert!(text_for_source(&photo).is_none());
        assert!(read("raw/sources/Casos/BGH/foto de la carta.jpg").starts_with("[Image:"));

        // The wiki recognises it once, through its own ingest.
        offer_recognized_text(&photo, LETTER, "motor-de-prueba");

        assert_eq!(read("raw/sources/Casos/BGH/foto de la carta.jpg"), LETTER);
        // The copy under another name never goes to the engine.
        assert_eq!(read("raw/sources/Casos/copias/la misma foto.jpg"), LETTER);
        // Offering again, or for a document that is already whole, changes nothing.
        offer_recognized_text(&photo, "otro texto", "motor-de-prueba");
        assert_eq!(read("raw/sources/Casos/BGH/foto de la carta.jpg"), LETTER);
    }

    #[test]
    fn paths_are_matched_to_their_origin_without_case_or_prefix() {
        let root = Path::new(r"C:\Casos\Estudio");
        assert_eq!(relative_to(root, Path::new(r"c:\casos\estudio\BGH\Carta.pdf")).as_deref(), Some("BGH/Carta.pdf"));
        assert_eq!(relative_to(root, Path::new(r"\\?\C:\Casos\Estudio\a.pdf")).as_deref(), Some("a.pdf"));
        assert_eq!(relative_to(root, Path::new(r"C:\Casos\Estudio")), None);
        assert_eq!(relative_to(root, Path::new(r"C:\Casos\Otro\a.pdf")), None);
        assert!(same_folder("C:/Casos/Estudio/", r"c:\casos\estudio"));
    }
}
