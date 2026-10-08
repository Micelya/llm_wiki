//! What change detection needs to know about mounted origins.
//!
//! File sync compares what is on disk with a snapshot and turns every
//! path that has disappeared into a "deleted" change, which in turn
//! removes the wiki pages generated from that source. A mounted origin
//! can vanish as a whole for reasons that have nothing to do with its
//! files: a cloud drive that is not signed in, an unplugged disk, a
//! network share that is down. This module is where that difference is
//! decided, so file sync never has to reason about mounts itself:
//!
//! - `locate` answers where a logical path really is, or `Unreachable`
//!   when nothing can be said about it right now. Callers must skip an
//!   unreachable path: no change, no snapshot update.
//! - `reachable_origins` lists the origins that can be walked and
//!   watched right now.
//!
//! An origin whose folder exists but is completely empty counts as
//! unreachable. Cloud drives show an empty folder while they are
//! starting or signed out, and wrongly deleting a whole project's pages
//! costs far more than ignoring a folder the user really emptied (they
//! can remove the mount instead).

use std::fs;
use std::path::{Path, PathBuf};

use super::{SourceVolume, MOUNTS_FILE, SOURCES_PREFIX};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// The path to look at. It may or may not exist.
    At(PathBuf),
    /// Under a mounted origin that cannot be reached, or the mount table
    /// could not be read. Unknown, never "deleted".
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    /// Logical folder of the mount, e.g. `raw/sources/Contratos`.
    pub logical_prefix: String,
    pub real_root: PathBuf,
}

/// Real location of the project-relative logical path `rel`.
pub fn locate(project_root: &Path, rel: &str) -> Location {
    let direct = project_root.join(rel);
    if !is_source_rel(rel) || !has_mount_table(project_root) {
        return Location::At(direct);
    }
    let Ok(volume) = SourceVolume::open(project_root) else {
        // Without the table a missing path may well be a mounted one.
        return if direct.exists() {
            Location::At(direct)
        } else {
            Location::Unreachable
        };
    };
    match volume.resolve(Path::new(rel)) {
        Ok(resolved) if resolved.is_mounted() => {
            let reachable = volume
                .mounts()
                .iter()
                .find(|mount| is_under(rel, &logical_prefix(&mount.name)))
                .is_some_and(|mount| is_reachable(Path::new(&mount.location)));
            if reachable {
                Location::At(resolved.path().to_path_buf())
            } else {
                Location::Unreachable
            }
        }
        Ok(resolved) => Location::At(resolved.path().to_path_buf()),
        Err(_) => Location::At(direct),
    }
}

/// Mounted origins that can be read right now.
pub fn reachable_origins(project_root: &Path) -> Vec<Origin> {
    if !has_mount_table(project_root) {
        return Vec::new();
    }
    let Ok(volume) = SourceVolume::open(project_root) else {
        return Vec::new();
    };
    volume
        .mounts()
        .iter()
        .filter(|mount| is_reachable(Path::new(&mount.location)))
        .map(|mount| Origin {
            logical_prefix: logical_prefix(&mount.name),
            real_root: PathBuf::from(&mount.location),
        })
        .collect()
}

/// True when `real` is inside one of the project's mounted origins,
/// reachable or not.
pub fn is_in_mounted_origin(project_root: &Path, real: &Path) -> bool {
    if real.starts_with(project_root) || !has_mount_table(project_root) {
        return false;
    }
    SourceVolume::open(project_root).is_ok_and(|volume| {
        volume
            .to_logical(real)
            .is_some_and(|logical| is_source_rel(&logical))
    })
}

fn has_mount_table(project_root: &Path) -> bool {
    project_root.join(MOUNTS_FILE).is_file()
}

fn logical_prefix(mount_name: &str) -> String {
    format!("{SOURCES_PREFIX}/{mount_name}")
}

fn is_source_rel(rel: &str) -> bool {
    is_under(rel, SOURCES_PREFIX)
}

fn is_under(rel: &str, prefix: &str) -> bool {
    let rel = rel.replace('\\', "/").to_lowercase();
    let prefix = prefix.to_lowercase();
    rel == prefix || rel.starts_with(&format!("{prefix}/"))
}

/// The folder can be listed and holds at least one entry.
fn is_reachable(origin: &Path) -> bool {
    fs::read_dir(origin).is_ok_and(|mut entries| entries.next().is_some_and(|entry| entry.is_ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        base: PathBuf,
        project: PathBuf,
        origin: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let base = std::env::temp_dir().join(format!("micelya-watch-{}", uuid::Uuid::new_v4()));
            let project = base.join("project");
            let origin = base.join("origin");
            fs::create_dir_all(project.join("raw/sources")).unwrap();
            fs::create_dir_all(&origin).unwrap();
            fs::write(origin.join("a.pdf"), b"a").unwrap();
            SourceVolume::open(&project)
                .unwrap()
                .add_local_mount("Docs", &origin)
                .unwrap();
            Self {
                base,
                project,
                origin,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.base);
        }
    }

    #[test]
    fn a_mounted_path_is_located_in_its_origin() {
        let fx = Fixture::new();
        assert_eq!(
            locate(&fx.project, "raw/sources/Docs/a.pdf"),
            Location::At(fx.origin.join("a.pdf"))
        );
    }

    #[test]
    fn a_file_missing_from_a_reachable_origin_is_located_so_it_can_be_seen_as_deleted() {
        let fx = Fixture::new();
        let Location::At(path) = locate(&fx.project, "raw/sources/Docs/gone.pdf") else {
            panic!("a reachable origin must give a location");
        };
        assert!(!path.exists());
    }

    #[test]
    fn nothing_is_known_about_a_path_whose_origin_disappeared() {
        let fx = Fixture::new();
        fs::remove_dir_all(&fx.origin).unwrap();
        assert_eq!(
            locate(&fx.project, "raw/sources/Docs/a.pdf"),
            Location::Unreachable
        );
        assert!(reachable_origins(&fx.project).is_empty());
    }

    #[test]
    fn an_origin_that_shows_up_empty_is_treated_as_unreachable() {
        let fx = Fixture::new();
        fs::remove_file(fx.origin.join("a.pdf")).unwrap();
        assert_eq!(
            locate(&fx.project, "raw/sources/Docs/a.pdf"),
            Location::Unreachable
        );
    }

    #[test]
    fn an_unreadable_mount_table_never_makes_missing_sources_look_deleted() {
        let fx = Fixture::new();
        fs::write(fx.project.join(MOUNTS_FILE), "{ not json").unwrap();
        assert_eq!(
            locate(&fx.project, "raw/sources/Docs/a.pdf"),
            Location::Unreachable
        );
        fs::write(fx.project.join("raw/sources/own.md"), "x").unwrap();
        assert_eq!(
            locate(&fx.project, "raw/sources/own.md"),
            Location::At(fx.project.join("raw/sources/own.md"))
        );
    }

    #[test]
    fn paths_outside_sources_and_copied_sources_stay_in_the_project() {
        let fx = Fixture::new();
        assert_eq!(
            locate(&fx.project, "wiki/index.md"),
            Location::At(fx.project.join("wiki/index.md"))
        );
        assert_eq!(
            locate(&fx.project, "raw/sources/Copied/b.pdf"),
            Location::At(fx.project.join("raw/sources/Copied/b.pdf"))
        );
    }

    #[test]
    fn derived_data_of_a_mount_stays_in_the_project() {
        let fx = Fixture::new();
        assert_eq!(
            locate(&fx.project, "raw/sources/Docs/.cache/a.pdf.txt"),
            Location::At(fx.project.join("raw/sources/Docs/.cache/a.pdf.txt"))
        );
    }

    #[test]
    fn reachable_origins_carry_their_logical_folder() {
        let fx = Fixture::new();
        assert_eq!(
            reachable_origins(&fx.project),
            vec![Origin {
                logical_prefix: "raw/sources/Docs".to_string(),
                real_root: PathBuf::from(fx.origin.to_string_lossy().replace('\\', "/")),
            }]
        );
        assert!(is_in_mounted_origin(&fx.project, &fx.origin.join("sub/x.pdf")));
        assert!(!is_in_mounted_origin(&fx.project, &fx.project.join("raw/sources/x.pdf")));
    }
}
