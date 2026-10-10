//! The source volume: the single owner of "where do a project's sources
//! live".
//!
//! The rest of the app addresses sources by their *logical* path,
//! `raw/sources/<name>/…`, exactly as it did when every source was copied
//! into the project. A project may *mount* an external origin under a
//! name; the volume then translates logical paths under that name to the
//! real location and back. Files that were copied into `raw/sources`
//! keep working untouched: a path that matches no mount resolves to
//! itself.
//!
//! Mounted origins are read-only from the app's point of view. Anything
//! the app derives from them (extracted text, indexes) belongs inside
//! the project, never next to the originals.

// The volume is introduced ahead of its callers; fs and file_sync are
// routed through it in follow-up changes.
#![allow(dead_code, unused_imports)]

pub mod catalog;
pub mod file_hash;
mod provider;
pub mod recognition;
pub mod recognition_progress;
pub mod recognized_store;
mod recognition_codex;
pub mod watch;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use provider::{LocalFolderProvider, ProviderKind, SourceEntry, SourceProvider};

pub const SOURCES_PREFIX: &str = "raw/sources";
const MOUNTS_FILE: &str = ".llm-wiki/source-mounts.json";
const MOUNT_TABLE_VERSION: u32 = 1;
/// Folder the app keeps per-directory derived data in.
const DERIVED_DIR: &str = ".cache";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mount {
    pub id: String,
    /// Folder name the origin appears under in `raw/sources`.
    pub name: String,
    pub provider: ProviderKind,
    /// Provider-specific address; an absolute directory for local folders.
    pub location: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MountTable {
    version: u32,
    mounts: Vec<Mount>,
}

/// Outcome of translating a path the app asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// Not under any mount: use the path as given.
    Direct(PathBuf),
    /// Under a mount: `path` is the real location of the entry.
    Mounted { mount_id: String, path: PathBuf },
}

impl Resolved {
    pub fn path(&self) -> &Path {
        match self {
            Resolved::Direct(path) | Resolved::Mounted { path, .. } => path,
        }
    }

    pub fn is_mounted(&self) -> bool {
        matches!(self, Resolved::Mounted { .. })
    }
}

pub struct SourceVolume {
    project_root: PathBuf,
    mounts: Vec<Mount>,
}

impl SourceVolume {
    /// Load the project's mount table. A project without one has no
    /// mounts; a table that exists but cannot be read is an error, so a
    /// corrupt file is never mistaken for "nothing is mounted".
    pub fn open(project_root: impl Into<PathBuf>) -> Result<Self, String> {
        let project_root = project_root.into();
        let table_path = project_root.join(MOUNTS_FILE);
        let mounts = if table_path.exists() {
            let raw = fs::read_to_string(&table_path)
                .map_err(|e| format!("Failed to read '{}': {e}", table_path.display()))?;
            serde_json::from_str::<MountTable>(&raw)
                .map_err(|e| format!("Invalid mount table '{}': {e}", table_path.display()))?
                .mounts
        } else {
            Vec::new()
        };
        Ok(Self {
            project_root,
            mounts,
        })
    }

    pub fn mounts(&self) -> &[Mount] {
        &self.mounts
    }

    /// Mount a local folder so it appears as `raw/sources/<name>`.
    pub fn add_local_mount(&mut self, name: &str, folder: &Path) -> Result<Mount, String> {
        let name = name.trim();
        validate_mount_name(name)?;
        if self.mounts.iter().any(|m| same_name(&m.name, name)) {
            return Err(format!("A source named '{name}' is already mounted"));
        }
        if has_own_content(&self.sources_root().join(name)) {
            return Err(format!(
                "'{SOURCES_PREFIX}/{name}' already exists in the project and would be hidden by the mount"
            ));
        }
        if !folder.is_absolute() {
            return Err(format!("Source folder must be an absolute path: '{}'", folder.display()));
        }
        if !folder.is_dir() {
            return Err(format!("Source folder does not exist: '{}'", folder.display()));
        }
        let folder_parts = parts(folder);
        let project_parts = parts(&self.project_root);
        if strip_parts(&folder_parts, &project_parts).is_some()
            || strip_parts(&project_parts, &folder_parts).is_some()
        {
            return Err("Cannot mount the project folder, a folder inside it, or a folder that contains it".to_string());
        }

        let mount = Mount {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            provider: ProviderKind::LocalFolder,
            location: folder.to_string_lossy().replace('\\', "/"),
        };
        self.mounts.push(mount.clone());
        if let Err(err) = self.save() {
            self.mounts.pop();
            return Err(err);
        }
        Ok(mount)
    }

    /// Forget a mount. The origin itself is never touched.
    pub fn remove_mount(&mut self, id: &str) -> Result<Mount, String> {
        let idx = self
            .mounts
            .iter()
            .position(|m| m.id == id)
            .ok_or_else(|| format!("Unknown source mount: {id}"))?;
        let removed = self.mounts.remove(idx);
        if let Err(err) = self.save() {
            self.mounts.insert(idx, removed);
            return Err(err);
        }
        Ok(removed)
    }

    /// Translate a path the app asked for into where it really is.
    /// Accepts an absolute path or one relative to the project root.
    pub fn resolve(&self, path: &Path) -> Result<Resolved, String> {
        let requested = parts(path);
        let rel = if path.is_absolute() {
            match strip_parts(&requested, &parts(&self.project_root)) {
                Some(rel) => rel.to_vec(),
                None => return Ok(Resolved::Direct(path.to_path_buf())),
            }
        } else {
            requested
        };
        if rel.iter().any(|part| part == "..") {
            return Err(format!("Path escapes the project: '{}'", path.display()));
        }

        let prefix = parts(Path::new(SOURCES_PREFIX));
        if let Some([name, rest @ ..]) = strip_parts(&rel, &prefix) {
            // Derived data (extracted-text caches) keeps its logical
            // address but lives in the project, so the app never writes
            // next to the originals.
            let derived = rest.iter().any(|part| part == DERIVED_DIR);
            let mount = self.mounts.iter().find(|m| same_name(&m.name, name));
            if let (Some(mount), false) = (mount, derived) {
                let mut real = PathBuf::from(&mount.location);
                real.extend(rest);
                return Ok(Resolved::Mounted {
                    mount_id: mount.id.clone(),
                    path: real,
                });
            }
        }

        let mut direct = self.project_root.clone();
        direct.extend(&rel);
        Ok(Resolved::Direct(direct))
    }

    /// Project-relative logical path for a real path, with forward
    /// slashes. `None` when the path belongs neither to a mount nor to
    /// the project.
    pub fn to_logical(&self, real: &Path) -> Option<String> {
        let real_parts = parts(real);
        // Longest origin wins so a mount nested inside another one maps
        // to its own name.
        let mounted = self
            .mounts
            .iter()
            .filter_map(|mount| {
                let location = parts(Path::new(&mount.location));
                let rest = strip_parts(&real_parts, &location)?.to_vec();
                Some((location.len(), mount, rest))
            })
            .max_by_key(|(depth, _, _)| *depth);
        if let Some((_, mount, rest)) = mounted {
            let mut logical = vec![SOURCES_PREFIX.to_string(), mount.name.clone()];
            logical.extend(rest);
            return Some(logical.join("/"));
        }
        strip_parts(&real_parts, &parts(&self.project_root)).map(|rel| rel.join("/"))
    }

    pub fn provider_for(&self, mount: &Mount) -> Box<dyn SourceProvider> {
        match mount.provider {
            ProviderKind::LocalFolder => Box::new(LocalFolderProvider::new(&mount.location)),
        }
    }

    fn sources_root(&self) -> PathBuf {
        self.project_root.join(SOURCES_PREFIX)
    }

    fn save(&self) -> Result<(), String> {
        let table_path = self.project_root.join(MOUNTS_FILE);
        if let Some(parent) = table_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create '{}': {e}", parent.display()))?;
        }
        let table = MountTable {
            version: MOUNT_TABLE_VERSION,
            mounts: self.mounts.clone(),
        };
        let json = serde_json::to_string_pretty(&table).map_err(|e| e.to_string())?;
        let tmp_path = table_path.with_extension("json.tmp");
        fs::write(&tmp_path, json)
            .map_err(|e| format!("Failed to write '{}': {e}", tmp_path.display()))?;
        fs::rename(&tmp_path, &table_path)
            .map_err(|e| format!("Failed to replace '{}': {e}", table_path.display()))
    }
}

/// Real location of a path, for callers that only have the path and not
/// the project it belongs to. Paths outside `raw/sources`, and projects
/// without mounts, come back unchanged.
///
/// An unreadable mount table is an error rather than a silent fallback:
/// answering with the in-project path would make every mounted source
/// look deleted.
pub fn resolve_path(path: &Path) -> Result<PathBuf, String> {
    match volume_for(path)? {
        Some(volume) => Ok(volume.resolve(path)?.path().to_path_buf()),
        None => Ok(path.to_path_buf()),
    }
}

/// The volume of the project `path` belongs to, when `path` is under
/// that project's `raw/sources` and the project has a mount table.
pub fn volume_for(path: &Path) -> Result<Option<SourceVolume>, String> {
    for ancestor in path.ancestors() {
        if !is_sources_root(ancestor) {
            continue;
        }
        let Some(project_root) = ancestor.parent().and_then(Path::parent) else {
            continue;
        };
        if project_root.join(MOUNTS_FILE).is_file() {
            return SourceVolume::open(project_root).map(Some);
        }
    }
    Ok(None)
}

/// Root of the project `path` belongs to, when `path` is inside that
/// project's `raw/sources`.
pub fn project_root_of(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| is_sources_root(ancestor))
        .and_then(|sources| sources.parent()?.parent().map(Path::to_path_buf))
}

fn is_sources_root(path: &Path) -> bool {
    let named = |p: &Path, expected: &str| {
        p.file_name()
            .is_some_and(|name| same_part(&name.to_string_lossy(), expected))
    };
    named(path, "sources") && path.parent().is_some_and(|parent| named(parent, "raw"))
}

/// Mounts of the project whose `raw/sources` folder is `dir`, as
/// (name, real location). Empty for any other directory.
pub fn mounts_under(dir: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    if !is_sources_root(dir) {
        return Ok(Vec::new());
    }
    Ok(volume_for(dir)?
        .map(|volume| {
            volume
                .mounts
                .iter()
                .map(|m| (m.name.clone(), PathBuf::from(&m.location)))
                .collect()
        })
        .unwrap_or_default())
}

/// `resolve_path` for callers that carry paths as strings. The input is
/// returned untouched when nothing is mounted there.
pub fn resolve_str(path: &str) -> Result<String, String> {
    let resolved = resolve_path(Path::new(path))?;
    if resolved == Path::new(path) {
        Ok(path.to_string())
    } else {
        Ok(resolved.to_string_lossy().replace('\\', "/"))
    }
}

/// True when `dir` holds anything besides derived data, i.e. files a
/// mount of the same name would hide.
fn has_own_content(dir: &Path) -> bool {
    if !dir.exists() {
        return false;
    }
    if !dir.is_dir() {
        return true;
    }
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .any(|entry| {
            let rel = entry.path().strip_prefix(dir).unwrap_or(entry.path());
            !parts(rel).iter().any(|part| part == DERIVED_DIR)
        })
}

/// Handle a delete request for a path the volume owns. Returns false
/// when the path is not under a mount and the caller should delete it
/// as usual.
///
/// Originals are never deleted. Asking to delete the mounted folder
/// itself unmounts it and drops the data derived from it; asking to
/// delete something inside a mount is accepted and does nothing on disk,
/// so the app can still forget what it generated from that file.
pub fn delete_mounted(path: &Path) -> Result<bool, String> {
    let Some(mut volume) = volume_for(path)? else {
        return Ok(false);
    };
    let Resolved::Mounted { mount_id, path: real } = volume.resolve(path)? else {
        return Ok(false);
    };
    let Some(mount) = volume.mounts.iter().find(|m| m.id == mount_id).cloned() else {
        return Ok(false);
    };
    let is_mount_root = strip_parts(&parts(&real), &parts(Path::new(&mount.location)))
        .is_some_and(|rest| rest.is_empty());
    if is_mount_root {
        volume.remove_mount(&mount_id)?;
        let derived = volume.sources_root().join(&mount.name);
        if derived.is_dir() && !has_own_content(&derived) {
            let _ = fs::remove_dir_all(&derived);
        }
    }
    Ok(true)
}

#[tauri::command]
pub async fn list_source_mounts(project_path: String) -> Result<Vec<Mount>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::panic_guard::run_guarded("list_source_mounts", || {
            Ok(SourceVolume::open(project_path)?.mounts().to_vec())
        })
    })
    .await
    .map_err(|e| format!("list_source_mounts blocking task join error: {e}"))?
}

#[tauri::command]
pub async fn add_source_mount(
    project_path: String,
    name: String,
    folder: String,
) -> Result<Mount, String> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::panic_guard::run_guarded("add_source_mount", || {
            SourceVolume::open(project_path)?.add_local_mount(&name, Path::new(&folder))
        })
    })
    .await
    .map_err(|e| format!("add_source_mount blocking task join error: {e}"))?
}

fn validate_mount_name(name: &str) -> Result<(), String> {
    let invalid = name.is_empty()
        || name.starts_with('.')
        || name.ends_with(['.', ' '])
        || name.chars().any(|c| c.is_control() || r#"/\<>:"|?*"#.contains(c));
    if invalid {
        return Err(format!("Invalid source name: '{name}'"));
    }
    Ok(())
}

/// Mount names share a namespace with folders on case-insensitive
/// filesystems, so they are compared without case everywhere.
fn same_name(a: &str, b: &str) -> bool {
    a.to_lowercase() == b.to_lowercase()
}

fn same_part(a: &str, b: &str) -> bool {
    if cfg!(windows) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

/// Path split into comparable segments: separators unified, `.` and
/// empty segments dropped, Windows verbatim prefix removed. `..` is kept
/// so callers can reject it.
fn parts(path: &Path) -> Vec<String> {
    let normalized = path.to_string_lossy().replace('\\', "/");
    let normalized = normalized.strip_prefix("//?/").unwrap_or(&normalized);
    normalized
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .map(str::to_string)
        .collect()
}

fn strip_parts<'a>(path: &'a [String], base: &[String]) -> Option<&'a [String]> {
    if base.is_empty() || path.len() < base.len() {
        return None;
    }
    path.iter()
        .zip(base)
        .all(|(a, b)| same_part(a, b))
        .then(|| &path[base.len()..])
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        base: PathBuf,
        project: PathBuf,
        external: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!("llm-wiki-volume-{}", Uuid::new_v4()));
            let project = base.join("project");
            let external = base.join("external").join("Contratos");
            fs::create_dir_all(project.join(SOURCES_PREFIX)).unwrap();
            fs::create_dir_all(external.join("2025")).unwrap();
            fs::write(external.join("2025").join("acuerdo.pdf"), b"pdf").unwrap();
            fs::write(external.join("notas.md"), b"notas").unwrap();
            Self {
                base,
                project,
                external,
            }
        }

        fn volume(&self) -> SourceVolume {
            SourceVolume::open(&self.project).unwrap()
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn project_without_mount_table_has_no_mounts() {
        let fx = Fixture::new();
        assert!(fx.volume().mounts().is_empty());
    }

    #[test]
    fn mounts_persist_across_open() {
        let fx = Fixture::new();
        let added = fx.volume().add_local_mount("Contratos", &fx.external).unwrap();
        assert_eq!(fx.volume().mounts(), &[added]);
    }

    #[test]
    fn corrupt_mount_table_is_an_error_not_an_empty_volume() {
        let fx = Fixture::new();
        fs::create_dir_all(fx.project.join(".llm-wiki")).unwrap();
        fs::write(fx.project.join(MOUNTS_FILE), "{ not json").unwrap();
        assert!(SourceVolume::open(&fx.project).is_err());
    }

    #[test]
    fn resolves_logical_paths_under_a_mount_to_the_origin() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        let mount = volume.add_local_mount("Contratos", &fx.external).unwrap();

        let relative = volume
            .resolve(Path::new("raw/sources/Contratos/2025/acuerdo.pdf"))
            .unwrap();
        let absolute = volume
            .resolve(&fx.project.join("raw/sources/contratos/2025/acuerdo.pdf"))
            .unwrap();

        for resolved in [relative, absolute] {
            assert_eq!(
                resolved,
                Resolved::Mounted {
                    mount_id: mount.id.clone(),
                    path: fx.external.join("2025").join("acuerdo.pdf"),
                }
            );
            assert!(resolved.path().is_file());
        }
    }

    #[test]
    fn mount_root_itself_resolves_to_the_origin_folder() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        volume.add_local_mount("Contratos", &fx.external).unwrap();
        let resolved = volume.resolve(Path::new("raw/sources/Contratos")).unwrap();
        assert!(resolved.is_mounted());
        assert_eq!(parts(resolved.path()), parts(&fx.external));
    }

    #[test]
    fn paths_outside_any_mount_resolve_to_themselves() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        volume.add_local_mount("Contratos", &fx.external).unwrap();

        let copied = volume.resolve(Path::new("raw/sources/Copiados/a.md")).unwrap();
        assert_eq!(copied, Resolved::Direct(fx.project.join("raw/sources/Copiados/a.md")));

        let wiki = volume.resolve(Path::new("wiki/index.md")).unwrap();
        assert_eq!(wiki, Resolved::Direct(fx.project.join("wiki/index.md")));

        let outside = fx.base.join("elsewhere.txt");
        assert_eq!(volume.resolve(&outside).unwrap(), Resolved::Direct(outside));
    }

    #[test]
    fn resolve_rejects_parent_traversal() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        volume.add_local_mount("Contratos", &fx.external).unwrap();
        assert!(volume
            .resolve(Path::new("raw/sources/Contratos/../../secret.txt"))
            .is_err());
    }

    #[test]
    fn to_logical_round_trips_with_resolve() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        volume.add_local_mount("Contratos", &fx.external).unwrap();

        let real = fx.external.join("2025").join("acuerdo.pdf");
        let logical = volume.to_logical(&real).unwrap();
        assert_eq!(logical, "raw/sources/Contratos/2025/acuerdo.pdf");
        assert_eq!(volume.resolve(Path::new(&logical)).unwrap().path(), real);

        assert_eq!(
            volume.to_logical(&fx.project.join("wiki").join("index.md")).unwrap(),
            "wiki/index.md"
        );
        assert_eq!(volume.to_logical(&fx.base.join("elsewhere.txt")), None);
    }

    #[test]
    fn to_logical_prefers_the_deepest_mount() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        volume.add_local_mount("Contratos", &fx.external).unwrap();
        volume.add_local_mount("Anio2025", &fx.external.join("2025")).unwrap();
        assert_eq!(
            volume
                .to_logical(&fx.external.join("2025").join("acuerdo.pdf"))
                .unwrap(),
            "raw/sources/Anio2025/acuerdo.pdf"
        );
    }

    #[test]
    fn rejects_invalid_or_conflicting_mounts() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        volume.add_local_mount("Contratos", &fx.external).unwrap();

        // Same name, different case.
        assert!(volume.add_local_mount("contratos", &fx.external).is_err());
        // Names that are not a single safe folder name.
        for name in ["", "a/b", "..", ".cache", "a:b", "fin."] {
            assert!(volume.add_local_mount(name, &fx.external).is_err(), "{name}");
        }
        // Would hide a folder that already exists in the project.
        let copied = fx.project.join(SOURCES_PREFIX).join("Copiados");
        fs::create_dir_all(&copied).unwrap();
        fs::write(copied.join("a.md"), "a").unwrap();
        assert!(volume.add_local_mount("Copiados", &fx.external).is_err());
        // Origin must exist and must not overlap the project.
        assert!(volume.add_local_mount("Nada", &fx.base.join("missing")).is_err());
        assert!(volume.add_local_mount("Proyecto", &fx.project).is_err());
        assert!(volume.add_local_mount("Wiki", &fx.project.join("raw")).is_err());
        assert!(volume.add_local_mount("Base", &fx.base).is_err());

        assert_eq!(volume.mounts().len(), 1);
    }

    #[test]
    fn removing_a_mount_leaves_the_origin_untouched() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        let mount = volume.add_local_mount("Contratos", &fx.external).unwrap();

        volume.remove_mount(&mount.id).unwrap();

        assert!(fx.volume().mounts().is_empty());
        assert!(fx.external.join("notas.md").is_file());
        assert!(volume.remove_mount(&mount.id).is_err());
    }

    #[test]
    fn resolve_path_finds_the_project_from_the_path_alone() {
        let fx = Fixture::new();
        let logical = fx.project.join("raw/sources/Contratos/notas.md");
        let copied = fx.project.join("raw/sources/Copiados/a.md");

        // No mount table yet: nothing is translated.
        assert_eq!(resolve_path(&logical).unwrap(), logical);

        fx.volume().add_local_mount("Contratos", &fx.external).unwrap();

        assert_eq!(resolve_path(&logical).unwrap(), fx.external.join("notas.md"));
        assert_eq!(resolve_path(&copied).unwrap(), copied);
        let wiki = fx.project.join("wiki/index.md");
        assert_eq!(resolve_path(&wiki).unwrap(), wiki);
    }

    #[test]
    fn resolve_path_fails_loudly_on_a_corrupt_mount_table() {
        let fx = Fixture::new();
        fx.volume().add_local_mount("Contratos", &fx.external).unwrap();
        fs::write(fx.project.join(MOUNTS_FILE), "{ not json").unwrap();

        assert!(resolve_path(&fx.project.join("raw/sources/Contratos/notas.md")).is_err());
        // Paths outside raw/sources never consult the table.
        let wiki = fx.project.join("wiki/index.md");
        assert_eq!(resolve_path(&wiki).unwrap(), wiki);
    }

    #[test]
    fn derived_data_under_a_mount_stays_in_the_project() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        volume.add_local_mount("Contratos", &fx.external).unwrap();

        let cache = "raw/sources/Contratos/2025/.cache/acuerdo.pdf.txt";
        assert_eq!(
            volume.resolve(Path::new(cache)).unwrap(),
            Resolved::Direct(fx.project.join(cache))
        );
    }

    #[test]
    fn a_folder_with_only_derived_data_does_not_block_a_mount() {
        let fx = Fixture::new();
        let shadow = fx.project.join(SOURCES_PREFIX).join("Contratos");
        fs::create_dir_all(shadow.join("2025").join(".cache")).unwrap();
        fs::write(shadow.join("2025").join(".cache").join("acuerdo.pdf.txt"), "x").unwrap();

        let mut volume = fx.volume();
        assert!(volume.add_local_mount("Contratos", &fx.external).is_ok());
    }

    #[test]
    fn mounts_under_only_answers_for_the_sources_root() {
        let fx = Fixture::new();
        fx.volume().add_local_mount("Contratos", &fx.external).unwrap();

        let listed = mounts_under(&fx.project.join(SOURCES_PREFIX)).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].0, "Contratos");
        assert_eq!(parts(&listed[0].1), parts(&fx.external));
        assert!(mounts_under(&fx.project).unwrap().is_empty());
        assert!(mounts_under(&fx.project.join("wiki")).unwrap().is_empty());
    }

    #[test]
    fn resolve_str_keeps_untranslated_paths_byte_identical() {
        let fx = Fixture::new();
        fx.volume().add_local_mount("Contratos", &fx.external).unwrap();

        let sep = std::path::MAIN_SEPARATOR;
        let wiki = format!("{}{sep}wiki{sep}index.md", fx.project.display());
        assert_eq!(resolve_str(&wiki).unwrap(), wiki);
        let mounted = format!("{}/raw/sources/Contratos/notas.md", fx.project.display());
        assert!(Path::new(&resolve_str(&mounted).unwrap()).is_file());
    }

    #[test]
    fn deleting_a_mount_unmounts_it_and_never_touches_the_origin() {
        let fx = Fixture::new();
        fx.volume().add_local_mount("Contratos", &fx.external).unwrap();
        let logical_root = fx.project.join("raw/sources/Contratos");
        let cache = logical_root.join("2025").join(".cache");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("acuerdo.pdf.txt"), "texto").unwrap();

        // A file inside the mount: accepted, nothing happens on disk.
        assert!(delete_mounted(&logical_root.join("2025/acuerdo.pdf")).unwrap());
        assert!(fx.external.join("2025").join("acuerdo.pdf").is_file());
        assert_eq!(fx.volume().mounts().len(), 1);

        // The mounted folder itself: unmounted, derived data dropped.
        assert!(delete_mounted(&logical_root).unwrap());
        assert!(fx.volume().mounts().is_empty());
        assert!(!logical_root.exists());
        assert!(fx.external.join("2025").join("acuerdo.pdf").is_file());
        assert!(fx.external.join("notas.md").is_file());

        // Anything else is left to the caller.
        assert!(!delete_mounted(&fx.project.join("raw/sources/Copiados/a.md")).unwrap());
        assert!(!delete_mounted(&fx.project.join("wiki/index.md")).unwrap());
    }

    #[test]
    fn local_provider_lists_files_and_reports_availability() {
        let fx = Fixture::new();
        let mut volume = fx.volume();
        let mount = volume.add_local_mount("Contratos", &fx.external).unwrap();
        let provider = volume.provider_for(&mount);

        assert!(provider.is_available());
        let listed = provider
            .list()
            .unwrap()
            .into_iter()
            .map(|entry| (entry.rel_path, entry.size))
            .collect::<Vec<_>>();
        assert_eq!(
            listed,
            vec![("2025/acuerdo.pdf".to_string(), 3), ("notas.md".to_string(), 5)]
        );
        assert_eq!(
            provider.local_path("2025/acuerdo.pdf").unwrap(),
            fx.external.join("2025").join("acuerdo.pdf")
        );
        assert_eq!(provider.local_path("../x"), None);

        fs::remove_dir_all(&fx.external).unwrap();
        assert!(!provider.is_available());
        assert!(provider.list().is_err());
    }
}
