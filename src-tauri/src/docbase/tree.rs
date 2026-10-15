//! An origin seen as a tree of folders, and what the user left out.
//!
//! Excluding a file or a folder touches nothing on disk. It stays in the
//! inventory, in sight, but the base stops reading it, does not extract
//! its text and does not offer it to the wiki. It can be undone.

use std::collections::BTreeMap;

use rusqlite::params;
use rusqlite::types::Value;
use serde::Serialize;

use super::query::{read_rows, rows_sql, DocumentRow};
use super::{now_ms, sql_err, DocBase, Visible};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    pub name: String,
    /// Path inside the origin, with forward slashes.
    pub rel_path: String,
    /// Files anywhere under it.
    pub files: i64,
    /// Left out, itself or through a folder above it.
    pub excluded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderListing {
    pub origin_id: String,
    /// The folder listed; empty for the root of the origin.
    pub folder: String,
    pub excluded: bool,
    pub folders: Vec<FolderNode>,
    pub files: Vec<DocumentRow>,
}

/// What was left out of one origin.
pub(crate) struct Exclusions {
    files: Vec<String>,
    folders: Vec<String>,
}

impl Exclusions {
    pub(crate) fn load(base: &DocBase, origin_id: &str) -> Result<Self, String> {
        let conn = base.conn();
        let mut statement = conn
            .prepare("SELECT rel_path, is_folder FROM exclusion WHERE origin_id = ?1")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([origin_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))
            .map_err(sql_err)?;
        let mut found = Exclusions {
            files: Vec::new(),
            folders: Vec::new(),
        };
        for row in rows {
            let (rel_path, is_folder) = row.map_err(sql_err)?;
            if is_folder != 0 {
                found.folders.push(rel_path);
            } else {
                found.files.push(rel_path);
            }
        }
        Ok(found)
    }

    pub(crate) fn covers_folder(&self, folder: &str) -> bool {
        self.folders
            .iter()
            .any(|excluded| excluded.is_empty() || folder == excluded || is_under(folder, excluded))
    }

    pub(crate) fn covers_file(&self, rel_path: &str) -> bool {
        self.files.iter().any(|excluded| excluded == rel_path)
            || self
                .folders
                .iter()
                .any(|excluded| excluded.is_empty() || is_under(rel_path, excluded))
    }
}

fn is_under(path: &str, folder: &str) -> bool {
    path.len() > folder.len() && path.starts_with(folder) && path.as_bytes()[folder.len()] == b'/'
}

impl DocBase {
    /// The folders and files directly inside one folder of an origin.
    pub fn list_folder(&self, visible: &Visible, origin_id: &str, folder: &str) -> Result<FolderListing, String> {
        let folder = folder.trim_matches('/');
        if !matches!(visible, Visible::All) && !matches!(visible, Visible::Origins(ids) if ids.iter().any(|id| id == origin_id)) {
            return Err("Ese origen ya no existe".to_string());
        }
        let exclusions = Exclusions::load(self, origin_id)?;
        let conn = self.conn();

        // Every folder under this one that holds files, with how many.
        let mut children: BTreeMap<String, i64> = BTreeMap::new();
        {
            let mut statement = conn
                .prepare(
                    "SELECT folder, count(*) FROM location
                     WHERE origin_id = ?1 AND folder != ?2
                       AND (?2 = '' OR substr(folder, 1, length(?2) + 1) = ?2 || '/')
                     GROUP BY folder",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params![origin_id, folder], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (deep, count) = row.map_err(sql_err)?;
                let rest = if folder.is_empty() { deep.as_str() } else { &deep[folder.len() + 1..] };
                let child = rest.split('/').next().unwrap_or(rest).to_string();
                *children.entry(child).or_default() += count;
            }
        }
        let mut folders: Vec<FolderNode> = children
            .into_iter()
            .map(|(name, files)| {
                let rel_path = if folder.is_empty() { name.clone() } else { format!("{folder}/{name}") };
                FolderNode {
                    excluded: exclusions.covers_folder(&rel_path),
                    name,
                    rel_path,
                    files,
                }
            })
            .collect();
        folders.sort_by_key(|node| node.name.to_lowercase());

        let (sql, mut bound) = rows_sql(visible, None);
        bound.push(Value::Text(origin_id.to_string()));
        bound.push(Value::Text(folder.to_string()));
        let files = read_rows(
            &conn,
            &format!("SELECT * FROM ({sql}) WHERE origin_id = ? AND folder = ? ORDER BY name COLLATE NOCASE"),
            bound,
        )?;

        Ok(FolderListing {
            origin_id: origin_id.to_string(),
            excluded: exclusions.covers_folder(folder),
            folder: folder.to_string(),
            folders,
            files,
        })
    }

    /// Leaves a file or a folder out of the base, or brings it back.
    pub fn set_excluded(&self, origin_id: &str, rel_path: &str, is_folder: bool, excluded: bool) -> Result<(), String> {
        let rel_path = rel_path.trim_matches('/');
        let conn = self.conn();
        if excluded {
            conn.execute(
                "INSERT OR REPLACE INTO exclusion (origin_id, rel_path, is_folder, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![origin_id, rel_path, is_folder as i64, now_ms()],
            )
            .map_err(sql_err)?;
        } else {
            conn.execute(
                "DELETE FROM exclusion WHERE origin_id = ?1 AND rel_path = ?2",
                params![origin_id, rel_path],
            )
            .map_err(sql_err)?;
        }
        Ok(())
    }

    /// Where a folder of an origin is on disk.
    pub fn folder_path(&self, origin_id: &str, folder: &str) -> Result<Option<String>, String> {
        Ok(self.origin(origin_id)?.map(|origin| {
            let mut path = origin.root();
            for part in folder.split('/').filter(|part| !part.is_empty() && *part != "..") {
                path.push(part);
            }
            path.to_string_lossy().to_string()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    fn sample() -> (TempDir, DocBase, String) {
        let dir = TempDir::new();
        dir.write("o/BOREAL/Reclamo/carta.md", b"una carta");
        dir.write("o/BOREAL/Reclamo/Anexos/factura.md", b"una factura");
        dir.write("o/BOREAL/contrato.md", b"un contrato");
        dir.write("o/Sovra/acuerdo.md", b"un acuerdo");
        dir.write("o/suelto.md", b"suelto");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        (dir, base, origin.id)
    }

    fn settle(base: &DocBase) {
        while base.hash_pending(50).unwrap() > 0 {}
        while base.extract_pending(50).unwrap() > 0 {}
    }

    #[test]
    fn a_folder_lists_its_subfolders_with_their_files_and_its_own_files() {
        let (_dir, base, origin) = sample();
        let root = base.list_folder(&Visible::All, &origin, "").unwrap();
        assert_eq!(
            root.folders.iter().map(|f| (f.name.as_str(), f.files)).collect::<Vec<_>>(),
            [("BOREAL", 3), ("Sovra", 1)]
        );
        assert_eq!(root.files.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(), ["suelto.md"]);

        let boreal = base.list_folder(&Visible::All, &origin, "BOREAL").unwrap();
        assert_eq!(boreal.folders.len(), 1);
        assert_eq!((boreal.folders[0].rel_path.as_str(), boreal.folders[0].files), ("BOREAL/Reclamo", 2));
        assert_eq!(boreal.files.len(), 1);

        assert!(base.list_folder(&Visible::Origins(vec![]), &origin, "").is_err());
    }

    #[test]
    fn an_excluded_file_is_kept_in_sight_and_never_read() {
        let (_dir, base, origin) = sample();
        base.set_excluded(&origin, "BOREAL/contrato.md", false, true).unwrap();
        settle(&base);

        let boreal = base.list_folder(&Visible::All, &origin, "BOREAL").unwrap();
        assert_eq!(boreal.files[0].state, "excluded");
        let unread: i64 = base
            .conn()
            .query_row("SELECT count(*) FROM location WHERE hash_state = 'pending'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(unread, 1);

        // Brought back, it is read like any other.
        base.set_excluded(&origin, "BOREAL/contrato.md", false, false).unwrap();
        settle(&base);
        let boreal = base.list_folder(&Visible::All, &origin, "BOREAL").unwrap();
        assert_eq!(boreal.files[0].state, "processed");
    }

    #[test]
    fn excluding_a_folder_leaves_out_everything_under_it_and_nothing_beside_it() {
        let (dir, base, origin) = sample();
        dir.write("o/BOREAL-otro/nota.md", b"otra carpeta de nombre parecido");
        base.scan_origin(&origin).unwrap();
        base.set_excluded(&origin, "BOREAL", true, true).unwrap();
        settle(&base);

        let root = base.list_folder(&Visible::All, &origin, "").unwrap();
        let excluded: Vec<&str> = root.folders.iter().filter(|f| f.excluded).map(|f| f.name.as_str()).collect();
        assert_eq!(excluded, ["BOREAL"]);
        let deep = base.list_folder(&Visible::All, &origin, "BOREAL/Reclamo/Anexos").unwrap();
        assert!(deep.excluded);
        assert_eq!(deep.files[0].state, "excluded");
        let other = base.list_folder(&Visible::All, &origin, "BOREAL-otro").unwrap();
        assert_eq!(other.files[0].state, "processed");

        let health = base.health(&Visible::All).unwrap();
        assert_eq!(health.by_state.get("excluded"), Some(&3));
    }

    #[test]
    fn what_is_excluded_is_not_offered_to_the_wiki() {
        let (dir, base, origin) = sample();
        dir.folder("General/wiki");
        dir.folder("General/raw/sources");
        dir.write("General/schema.md", b"# schema");
        base.link_general_project(&dir.path().join("General"), "default").unwrap();
        base.set_excluded(&origin, "BOREAL", true, true).unwrap();
        base.set_excluded(&origin, "suelto.md", false, true).unwrap();

        let offered: Vec<String> = base.wiki_sources().unwrap().into_iter().map(|s| s.identity).collect();
        assert_eq!(offered, ["O/Sovra/acuerdo.md"]);

        let asked = vec![
            "raw/sources/O/BOREAL/contrato.md".to_string(),
            "raw/sources/O/Sovra/acuerdo.md".to_string(),
            "raw/sources/otra-cosa/x.md".to_string(),
        ];
        let project = dir.path().join("General");
        assert_eq!(base.excluded_sources(&project, &asked).unwrap(), ["raw/sources/O/BOREAL/contrato.md"]);
        // Another project is none of the base's business.
        assert!(base.excluded_sources(&dir.path().join("Otro"), &asked).unwrap().is_empty());
    }
}
