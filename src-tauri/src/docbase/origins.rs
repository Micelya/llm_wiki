//! Origins: the folders the user incorporated into the space.
//!
//! Incorporating the first one creates the space's general project,
//! which covers every origin from then on.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use uuid::Uuid;

use super::{now_ms, sql_err, DocBase, Visible};

pub const LOCAL_FOLDER: &str = "local-folder";
/// Technical name of the general project and of its folder in the app's
/// workspace. What the user reads instead comes from the vocabulary of
/// their profession ("Archivo" for a law firm).
pub const GENERAL_PROJECT_NAME: &str = "default";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Origin {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub location: String,
    /// The name of each folder level, outermost first.
    pub convention: Vec<String>,
    pub added_at: i64,
    pub last_scan_at: Option<i64>,
    /// `None` until the first sweep.
    pub reachable: Option<bool>,
}

impl Origin {
    pub fn root(&self) -> PathBuf {
        PathBuf::from(&self.location)
    }
}

impl DocBase {
    pub fn add_origin(
        &self,
        name: &str,
        folder: &Path,
        convention: &[String],
    ) -> Result<Origin, String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("El origen necesita un nombre".to_string());
        }
        // The name becomes a folder of the general project's wiki.
        let unusable = name.starts_with('.')
            || name.ends_with('.')
            || name.chars().any(|c| c.is_control() || r#"/\<>:"|?*"#.contains(c));
        if unusable {
            return Err(format!("'{name}' no sirve como nombre de origen"));
        }
        if !folder.is_absolute() || !folder.is_dir() {
            return Err(format!(
                "'{}' no es una carpeta que se pueda leer",
                folder.display()
            ));
        }
        let convention = clean_convention(convention);
        let location = folder.to_string_lossy().to_string();

        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        for other in list(&tx, &Visible::All)? {
            if other.name.to_lowercase() == name.to_lowercase() {
                return Err(format!("Ya hay un origen llamado '{}'", other.name));
            }
            let other_root = other.root();
            if contains(&other_root, folder) || contains(folder, &other_root) {
                return Err(format!(
                    "La carpeta se superpone con el origen '{}' ({})",
                    other.name, other.location
                ));
            }
        }
        let origin = Origin {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            provider: LOCAL_FOLDER.to_string(),
            location,
            convention,
            added_at: now_ms(),
            last_scan_at: None,
            reachable: None,
        };
        tx.execute(
            "INSERT INTO origin (id, name, provider, location, convention, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                origin.id,
                origin.name,
                origin.provider,
                origin.location,
                serde_json::to_string(&origin.convention).unwrap_or_else(|_| "[]".into()),
                origin.added_at
            ],
        )
        .map_err(sql_err)?;
        let general = ensure_general_project(&tx)?;
        tx.execute(
            "INSERT OR IGNORE INTO project_origin (project_id, origin_id) VALUES (?1, ?2)",
            params![general, origin.id],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        Ok(origin)
    }

    /// Forgets an origin and what was known about its files. The folder
    /// itself is never touched.
    pub fn remove_origin(&self, id: &str) -> Result<(), String> {
        let removed = self
            .conn()
            .execute("DELETE FROM origin WHERE id = ?1", [id])
            .map_err(sql_err)?;
        if removed == 0 {
            return Err("Ese origen ya no existe".to_string());
        }
        self.purge_orphans()?;
        Ok(())
    }

    pub fn set_origin_convention(&self, id: &str, convention: &[String]) -> Result<(), String> {
        let json = serde_json::to_string(&clean_convention(convention))
            .unwrap_or_else(|_| "[]".to_string());
        let updated = self
            .conn()
            .execute("UPDATE origin SET convention = ?1 WHERE id = ?2", params![json, id])
            .map_err(sql_err)?;
        if updated == 0 {
            return Err("Ese origen ya no existe".to_string());
        }
        Ok(())
    }

    pub fn origins(&self, visible: &Visible) -> Result<Vec<Origin>, String> {
        list(&self.conn(), visible)
    }

    pub fn origin(&self, id: &str) -> Result<Option<Origin>, String> {
        Ok(list(&self.conn(), &Visible::Origins(vec![id.to_string()]))?
            .into_iter()
            .next())
    }
}

pub(crate) fn list(conn: &Connection, visible: &Visible) -> Result<Vec<Origin>, String> {
    let (condition, bound) = visible.condition("id");
    let mut statement = conn
        .prepare(&format!(
            "SELECT id, name, provider, location, convention, added_at, last_scan_at, reachable
             FROM origin WHERE {condition} ORDER BY name COLLATE NOCASE"
        ))
        .map_err(sql_err)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(bound.iter()), |row| {
            let convention: String = row.get(4)?;
            let reachable: Option<i64> = row.get(7)?;
            Ok(Origin {
                id: row.get(0)?,
                name: row.get(1)?,
                provider: row.get(2)?,
                location: row.get(3)?,
                convention: serde_json::from_str(&convention).unwrap_or_default(),
                added_at: row.get(5)?,
                last_scan_at: row.get(6)?,
                reachable: reachable.map(|value| value != 0),
            })
        })
        .map_err(sql_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)
}

/// The id of the general project, creating it the first time.
pub(crate) fn ensure_general_project(conn: &Connection) -> Result<String, String> {
    let existing: Option<String> = conn
        .query_row("SELECT id FROM project WHERE kind = 'general'", [], |row| {
            row.get(0)
        })
        .optional()
        .map_err(sql_err)?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO project (id, name, kind, created_at) VALUES (?1, ?2, 'general', ?3)",
        params![id, GENERAL_PROJECT_NAME, now_ms()],
    )
    .map_err(sql_err)?;
    Ok(id)
}

/// The labels of a file: one per folder level the user named, taken from
/// the folder it is in, with the values the user corrected on top.
pub fn labels_for(
    folder: &str,
    convention: &[String],
    overrides: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    let levels = folder.split('/').filter(|part| !part.is_empty());
    for (name, value) in convention.iter().zip(levels) {
        labels.insert(name.clone(), value.to_string());
    }
    for (name, value) in overrides {
        if value.trim().is_empty() {
            labels.remove(name);
        } else {
            labels.insert(name.clone(), value.trim().to_string());
        }
    }
    labels
}

fn clean_convention(convention: &[String]) -> Vec<String> {
    convention
        .iter()
        .map(|level| level.trim().to_string())
        .filter(|level| !level.is_empty())
        .collect()
}

/// Whether `inner` is `outer` or sits somewhere under it, comparing the
/// way Windows does.
fn contains(outer: &Path, inner: &Path) -> bool {
    let normalize = |path: &Path| -> Vec<String> {
        path.components()
            .filter_map(|component| match component {
                std::path::Component::Normal(part) => Some(part.to_string_lossy().to_lowercase()),
                std::path::Component::Prefix(prefix) => {
                    Some(prefix.as_os_str().to_string_lossy().to_lowercase())
                }
                _ => None,
            })
            .collect()
    };
    let outer = normalize(outer);
    let inner = normalize(inner);
    inner.len() >= outer.len() && inner[..outer.len()] == outer[..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    #[test]
    fn the_first_origin_creates_the_general_project_and_later_ones_join_it() {
        let dir = TempDir::new();
        let base = DocBase::in_memory().unwrap();
        base.add_origin("Casos", &dir.folder("casos"), &[]).unwrap();
        base.add_origin("Administración", &dir.folder("admin"), &[]).unwrap();

        let conn = base.conn();
        let (projects, covered): (i64, i64) = (
            conn.query_row("SELECT count(*) FROM project WHERE kind = 'general' AND members = 'open'", [], |r| r.get(0))
                .unwrap(),
            conn.query_row("SELECT count(*) FROM project_origin", [], |r| r.get(0))
                .unwrap(),
        );
        assert_eq!((projects, covered), (1, 2));
    }

    #[test]
    fn rejects_a_repeated_name_a_missing_folder_and_overlapping_folders() {
        let dir = TempDir::new();
        let base = DocBase::in_memory().unwrap();
        let casos = dir.folder("casos");
        base.add_origin("Casos", &casos, &[]).unwrap();

        assert!(base.add_origin("casos", &dir.folder("otra"), &[]).is_err());
        assert!(base.add_origin("Fantasma", &dir.path().join("no-existe"), &[]).is_err());
        assert!(base.add_origin("Adentro", &dir.folder("casos/2026"), &[]).is_err());
        assert!(base.add_origin("Afuera", dir.path(), &[]).is_err());
        assert!(base.add_origin("  ", &dir.folder("vacio"), &[]).is_err());
        assert!(base.add_origin("Casos: 2026", &dir.folder("otra2"), &[]).is_err());
        assert!(base.add_origin("¿Dudas?", &dir.folder("otra3"), &[]).is_err());
        assert_eq!(base.origins(&Visible::All).unwrap().len(), 1);
    }

    #[test]
    fn visibility_limits_which_origins_are_listed() {
        let dir = TempDir::new();
        let base = DocBase::in_memory().unwrap();
        let a = base.add_origin("A", &dir.folder("a"), &[]).unwrap();
        base.add_origin("B", &dir.folder("b"), &[]).unwrap();

        let seen = base.origins(&Visible::Origins(vec![a.id.clone()])).unwrap();
        assert_eq!(seen.iter().map(|o| o.name.as_str()).collect::<Vec<_>>(), ["A"]);
        assert!(base.origins(&Visible::Origins(vec![])).unwrap().is_empty());
    }

    #[test]
    fn labels_come_from_the_folder_levels_and_can_be_corrected() {
        let convention = vec!["cliente".to_string(), "caso".to_string()];
        let none = BTreeMap::new();

        let labels = labels_for("BGH/Reclamo facturas/Anexos", &convention, &none);
        assert_eq!(labels.get("cliente").map(String::as_str), Some("BGH"));
        assert_eq!(labels.get("caso").map(String::as_str), Some("Reclamo facturas"));

        // A file left at the client level has no case.
        assert_eq!(labels_for("BGH", &convention, &none).len(), 1);
        assert!(labels_for("", &convention, &none).is_empty());

        // A misfiled document: the user corrects the case and clears nothing else.
        let mut fixed = BTreeMap::new();
        fixed.insert("caso".to_string(), "Otro caso".to_string());
        let labels = labels_for("BGH/Reclamo facturas", &convention, &fixed);
        assert_eq!(labels.get("caso").map(String::as_str), Some("Otro caso"));
        assert_eq!(labels.get("cliente").map(String::as_str), Some("BGH"));
    }
}
