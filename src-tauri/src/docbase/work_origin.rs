//! The app's work folder in the base (D-450, D-451): one origin that holds
//! what the app brings by itself (the web clips, the records of the
//! portals), each in its own folder inside it.
//!
//! The base takes no overlapping origins, so the work folder is a single
//! origin and its folders are part of it, never origins of their own. An
//! origin that already holds the folder serves as it. An origin inside it
//! that is one of the app's own folders (the records' folder, which was an
//! origin by itself before) becomes part of it, keeping everything known
//! about its files: the same locations, with their labels, exclusions and
//! history. When the work folder changes, its origin goes with it.

use std::path::Path;

use rusqlite::{params, OptionalExtension, Transaction};
use serde::Serialize;
use uuid::Uuid;

use super::origins::{self, path_inside, Origin, LOCAL_FOLDER};
use super::{now_ms, sql_err, DocBase, Visible};

/// Where the base keeps which origin is the work folder (`docbase_setting`).
const WORK_ORIGIN_SETTING: &str = "work-folder-origin";

/// What the work folder is in the base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkOrigin {
    pub origin: Origin,
    /// The origin is the work folder itself; if not, an origin that holds it.
    pub own: bool,
    /// The origins that became part of it now, by name.
    pub joined: Vec<String>,
}

/// A file a folder holds since some moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewFile {
    pub location_id: i64,
    pub name: String,
    /// Its folder inside its origin, with `/`.
    pub folder: String,
    pub origin_name: String,
    pub first_seen_at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewFiles {
    pub total: usize,
    /// The most recent first, up to the limit asked.
    pub files: Vec<NewFile>,
}

impl DocBase {
    /// The origin of the work folder, making it one when no origin holds it.
    /// `names` are the names it can take, the first free one; `app_folders`
    /// the app's own folders inside it («Expedientes»): an origin in one of
    /// them becomes part of it. Any other origin inside the folder stops it,
    /// with nothing changed: it is a folder of the person's, and turning it
    /// into part of another one is not the app's call.
    ///
    /// When the work folder changed, the origin it had moves to the new
    /// folder: what is there with the same path keeps everything known
    /// about it; what is not leaves the base (the files are not touched).
    pub fn ensure_work_origin(&self, names: &[&str], folder: &Path, app_folders: &[&str]) -> Result<WorkOrigin, String> {
        if !folder.is_absolute() || !folder.is_dir() {
            return Err(format!("'{}' no es una carpeta que se pueda leer", folder.display()));
        }
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        let all = origins::list(&tx, &Visible::All)?;
        let remembered: Option<Origin> = tx
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [WORK_ORIGIN_SETTING], |row| row.get::<_, String>(0))
            .optional()
            .map_err(sql_err)?
            .and_then(|id| all.iter().find(|origin| origin.id == id).cloned());

        // An origin that holds the folder already is it.
        if let Some((cover, rel)) = all.iter().find_map(|o| path_inside(&o.root(), folder).map(|rel| (o, rel))) {
            let own = rel.is_empty();
            if own {
                remember(&tx, &cover.id)?;
            } else {
                // The work folder is not an origin of its own: the one it
                // had stays as any other origin, never moved again.
                tx.execute("DELETE FROM docbase_setting WHERE key = ?1", [WORK_ORIGIN_SETTING]).map_err(sql_err)?;
            }
            tx.commit().map_err(sql_err)?;
            return Ok(WorkOrigin { origin: cover.clone(), own, joined: Vec::new() });
        }

        // The origins inside it: the app's, or the one it had.
        let inner: Vec<(Origin, String)> = all
            .iter()
            .filter_map(|o| path_inside(folder, &o.root()).map(|rel| (o.clone(), rel)))
            .collect();
        for (origin, rel) in &inner {
            let had = remembered.as_ref().is_some_and(|r| r.id == origin.id);
            if !had && !in_app_folder(rel, app_folders) {
                return Err(format!(
                    "La carpeta {} contiene el origen «{}» ({}), y la base no admite un origen dentro de otro. Elegí otra carpeta, o quitá ese origen en «Orígenes» y volvé a intentarlo.",
                    folder.display(),
                    origin.name,
                    origin.location
                ));
            }
        }

        let location = folder.to_string_lossy().to_string();
        // The origin it had moves here, unless it is inside the folder: then
        // it is one more to join, into a new one.
        let moved = remembered.filter(|r| !inner.iter().any(|(o, _)| o.id == r.id));
        let target = match &moved {
            Some(origin) => {
                tx.execute(
                    "UPDATE origin SET location = ?1, reachable = NULL WHERE id = ?2",
                    params![location, origin.id],
                )
                .map_err(sql_err)?;
                origin.id.clone()
            }
            None => {
                let id = Uuid::new_v4().to_string();
                // Its name comes at the end, once the ones it joins are gone.
                tx.execute(
                    "INSERT INTO origin (id, name, provider, location, added_at) VALUES (?1, ?1, ?2, ?3, ?4)",
                    params![id, LOCAL_FOLDER, location, now_ms()],
                )
                .map_err(sql_err)?;
                let general = origins::ensure_general_project(&tx)?;
                tx.execute(
                    "INSERT OR IGNORE INTO project_origin (project_id, origin_id) VALUES (?1, ?2)",
                    params![general, id],
                )
                .map_err(sql_err)?;
                id
            }
        };

        let mut joined = Vec::new();
        for (origin, rel) in &inner {
            join(&tx, &origin.id, &target, rel)?;
            joined.push(origin.name.clone());
        }
        if moved.is_some() {
            forget_missing(&tx, &target, folder)?;
        } else {
            let taken: Vec<String> = origins::list(&tx, &Visible::All)?
                .into_iter()
                .filter(|o| o.id != target)
                .map(|o| o.name.to_lowercase())
                .collect();
            let name = names
                .iter()
                .find(|name| !taken.contains(&name.to_lowercase()))
                .ok_or("Ya hay orígenes con los nombres que usa la app para su carpeta de trabajo")?;
            tx.execute("UPDATE origin SET name = ?1 WHERE id = ?2", params![name, target]).map_err(sql_err)?;
        }
        remember(&tx, &target)?;
        tx.commit().map_err(sql_err)?;
        drop(conn);

        if !joined.is_empty() {
            self.relabel_origin(&target)?;
        }
        let origin = self.origin(&target)?.ok_or("El origen de la carpeta de trabajo no quedó")?;
        Ok(WorkOrigin { origin, own: true, joined })
    }

    /// The files `folder` holds (it and what is under it) that the base
    /// first saw after the moment kept in `seen_setting`, or every one if
    /// none was kept. Nothing when no origin holds the folder.
    pub fn files_new_since(&self, folder: &Path, seen_setting: &str, limit: usize) -> Result<NewFiles, String> {
        let conn = self.conn();
        let seen: i64 = conn
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [seen_setting], |row| row.get::<_, String>(0))
            .optional()
            .map_err(sql_err)?
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let Some((origin, rel)) = origins::list(&conn, &Visible::All)?
            .into_iter()
            .find_map(|o| path_inside(&o.root(), folder).map(|rel| (o, rel)))
        else {
            return Ok(NewFiles { total: 0, files: Vec::new() });
        };
        let mut statement = conn
            .prepare(
                "SELECT id, name, folder, first_seen_at FROM location
                 WHERE origin_id = ?1 AND kind = 'file' AND first_seen_at > ?2
                   AND (?3 = '' OR lower(folder) = lower(?3) OR lower(substr(folder, 1, length(?3) + 1)) = lower(?3 || '/'))
                 ORDER BY first_seen_at DESC, id DESC",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map(params![origin.id, seen, rel], |row| {
                Ok(NewFile {
                    location_id: row.get(0)?,
                    name: row.get(1)?,
                    folder: row.get(2)?,
                    origin_name: origin.name.clone(),
                    first_seen_at: row.get(3)?,
                })
            })
            .map_err(sql_err)?;
        let all: Vec<NewFile> = rows.collect::<Result<_, _>>().map_err(sql_err)?;
        Ok(NewFiles { total: all.len(), files: all.into_iter().take(limit).collect() })
    }

    /// Keeps now as the moment of `seen_setting`: what came before is no
    /// longer new.
    pub fn mark_seen(&self, seen_setting: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
                params![seen_setting, now_ms().to_string()],
            )
            .map_err(sql_err)?;
        Ok(())
    }
}

fn remember(tx: &Transaction<'_>, origin_id: &str) -> Result<(), String> {
    tx.execute(
        "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
        params![WORK_ORIGIN_SETTING, origin_id],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Whether a path inside the work folder is one of the app's folders or
/// sits under one.
fn in_app_folder(rel: &str, app_folders: &[&str]) -> bool {
    let rel = rel.to_lowercase();
    app_folders.iter().any(|folder| {
        let folder = folder.to_lowercase();
        rel == folder || rel.starts_with(&format!("{folder}/"))
    })
}

/// `from`, an origin at `rel` inside `to`'s folder, becomes part of `to`:
/// its locations (the same ids, so everything that hangs from them stays),
/// exclusions, label rules and history, with their paths under `rel`.
fn join(tx: &Transaction<'_>, from: &str, to: &str, rel: &str) -> Result<(), String> {
    let prefixed = |column: &str| format!("CASE {column} WHEN '' THEN ?3 ELSE ?3 || '/' || {column} END");
    // A path the work folder's origin still had from before is the same file.
    tx.execute(
        "DELETE FROM location WHERE origin_id = ?2
           AND rel_path IN (SELECT ?3 || '/' || rel_path FROM location WHERE origin_id = ?1)",
        params![from, to, rel],
    )
    .map_err(sql_err)?;
    tx.execute(
        &format!(
            "UPDATE location SET origin_id = ?2, rel_path = ?3 || '/' || rel_path, folder = {} WHERE origin_id = ?1",
            prefixed("folder")
        ),
        params![from, to, rel],
    )
    .map_err(sql_err)?;
    tx.execute(
        &format!("UPDATE OR IGNORE exclusion SET origin_id = ?2, rel_path = {} WHERE origin_id = ?1", prefixed("rel_path")),
        params![from, to, rel],
    )
    .map_err(sql_err)?;
    tx.execute(
        &format!("UPDATE label_rule SET origin_id = ?2, folder = {} WHERE origin_id = ?1", prefixed("folder")),
        params![from, to, rel],
    )
    .map_err(sql_err)?;
    tx.execute(
        "UPDATE change SET origin_id = ?2, rel_path = ?3 || '/' || rel_path WHERE origin_id = ?1",
        params![from, to, rel],
    )
    .map_err(sql_err)?;
    tx.execute("DELETE FROM origin WHERE id = ?1", [from]).map_err(sql_err)?;
    Ok(())
}

/// After moving an origin to another folder: what is not there leaves the
/// base, as a sweep would say. A sweep cannot say it for a new folder with
/// nothing in it yet: it takes an empty folder for one it cannot read.
fn forget_missing(tx: &Transaction<'_>, origin_id: &str, folder: &Path) -> Result<(), String> {
    let known: Vec<(i64, String, Option<String>)> = {
        let mut statement = tx
            .prepare("SELECT id, rel_path, file_hash FROM location WHERE origin_id = ?1")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([origin_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let now = now_ms();
    for (id, rel_path, hash) in known {
        let path = rel_path.split('/').fold(folder.to_path_buf(), |path, part| path.join(part));
        if path.is_file() {
            continue;
        }
        tx.execute("DELETE FROM location WHERE id = ?1", [id]).map_err(sql_err)?;
        tx.execute(
            "INSERT INTO change (at, kind, origin_id, rel_path, file_hash) VALUES (?1, 'removed', ?2, ?3, ?4)",
            params![now, origin_id, rel_path, hash],
        )
        .map_err(sql_err)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    const NAMES: &[&str] = &["Carpeta de trabajo", "Carpeta de trabajo de la app"];
    const APP_FOLDERS: &[&str] = &["Expedientes", "Recortes web"];

    fn count(base: &DocBase, sql: &str) -> i64 {
        base.conn().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn settle(base: &DocBase, origin_id: &str) {
        base.scan_origin(origin_id).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
        while base.extract_pending(50).unwrap() > 0 {}
    }

    fn paths(base: &DocBase) -> Vec<(String, String)> {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT origin_id, rel_path FROM location ORDER BY rel_path").unwrap();
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().map(Result::unwrap).collect()
    }

    #[test]
    fn the_work_folder_becomes_an_origin_once() {
        let dir = TempDir::new();
        let work = dir.folder("trabajo");
        let base = DocBase::in_memory().unwrap();
        let first = base.ensure_work_origin(NAMES, &work, APP_FOLDERS).unwrap();
        assert!(first.own);
        assert_eq!(first.origin.name, "Carpeta de trabajo");
        assert_eq!(first.origin.root(), work);
        let again = base.ensure_work_origin(NAMES, &work, APP_FOLDERS).unwrap();
        assert_eq!(again.origin.id, first.origin.id);
        assert_eq!(base.origins(&Visible::All).unwrap().len(), 1);
        // It is part of the general project, like any origin.
        assert_eq!(count(&base, "SELECT count(*) FROM project_origin"), 1);
    }

    #[test]
    fn an_origin_that_holds_the_folder_already_is_it() {
        let dir = TempDir::new();
        let base = DocBase::in_memory().unwrap();
        let all = base.add_origin("Todo", &dir.folder("todo"), &[]).unwrap();
        let found = base.ensure_work_origin(NAMES, &dir.folder("todo/trabajo"), APP_FOLDERS).unwrap();
        assert_eq!((found.origin.id, found.own), (all.id, false));
        assert_eq!(base.origins(&Visible::All).unwrap().len(), 1);
    }

    #[test]
    fn the_records_origin_inside_it_becomes_part_of_it_with_everything_known() {
        let dir = TempDir::new();
        dir.write("trabajo/Expedientes/PJN/ZZZ 1-2020/escrito.txt", "Escrito de prueba con texto suficiente para la base.".as_bytes());
        dir.write("trabajo/Expedientes/PJN/ZZZ 1-2020/otro.txt", "Otro escrito inventado, distinto del primero.".as_bytes());
        let base = DocBase::in_memory().unwrap();
        let records = base.add_origin("Expedientes", &dir.path().join("trabajo/Expedientes"), &[]).unwrap();
        settle(&base, &records.id);
        let label = base.ensure_label(None, "revisado").unwrap();
        base.add_fixed_rule(&records.id, "PJN", label.id).unwrap();
        base.set_excluded(&records.id, "PJN/ZZZ 1-2020/otro.txt", false, true).unwrap();
        let before: Vec<i64> = {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT id FROM location ORDER BY id").unwrap();
            statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
        };
        let contents = count(&base, "SELECT count(*) FROM content");
        let texts = count(&base, "SELECT count(*) FROM derivative");

        let work = base.ensure_work_origin(NAMES, &dir.path().join("trabajo"), APP_FOLDERS).unwrap();
        assert_eq!(work.joined, vec!["Expedientes".to_string()]);
        assert_eq!(work.origin.name, "Carpeta de trabajo");
        let origins = base.origins(&Visible::All).unwrap();
        assert_eq!(origins.len(), 1, "no origin inside another");
        // The same locations, under the records' folder.
        assert_eq!(
            paths(&base),
            vec![
                (work.origin.id.clone(), "Expedientes/PJN/ZZZ 1-2020/escrito.txt".to_string()),
                (work.origin.id.clone(), "Expedientes/PJN/ZZZ 1-2020/otro.txt".to_string()),
            ]
        );
        let after: Vec<i64> = {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT id FROM location ORDER BY id").unwrap();
            statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
        };
        assert_eq!(after, before);
        assert_eq!(count(&base, "SELECT count(*) FROM content"), contents);
        assert_eq!(count(&base, "SELECT count(*) FROM derivative"), texts);
        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE folder = 'Expedientes/PJN/ZZZ 1-2020'"), 2);
        // Its rule, its exclusion and its history went with it.
        let rule: String = base.conn().query_row("SELECT folder FROM label_rule", [], |row| row.get(0)).unwrap();
        assert_eq!(rule, "Expedientes/PJN");
        assert_eq!(count(&base, "SELECT count(*) FROM location_label"), 2);
        let excluded: String = base.conn().query_row("SELECT rel_path FROM exclusion", [], |row| row.get(0)).unwrap();
        assert_eq!(excluded, "Expedientes/PJN/ZZZ 1-2020/otro.txt");
        assert_eq!(count(&base, "SELECT count(*) FROM change WHERE rel_path NOT LIKE 'Expedientes/%'"), 0);
        // A sweep finds nothing new and nothing gone.
        let report = base.scan_origin(&work.origin.id).unwrap();
        assert_eq!((report.added, report.removed, report.modified), (0, 0, 0));
    }

    #[test]
    fn an_origin_of_the_person_inside_it_stops_it_with_nothing_changed() {
        let dir = TempDir::new();
        let base = DocBase::in_memory().unwrap();
        base.add_origin("Casos", &dir.folder("trabajo/casos"), &[]).unwrap();
        let error = base.ensure_work_origin(NAMES, &dir.path().join("trabajo"), APP_FOLDERS).unwrap_err();
        assert!(error.contains("«Casos»"), "{error}");
        let origins = base.origins(&Visible::All).unwrap();
        assert_eq!(origins.iter().map(|o| o.name.as_str()).collect::<Vec<_>>(), ["Casos"]);
    }

    #[test]
    fn when_the_folder_changes_its_origin_goes_with_it() {
        let dir = TempDir::new();
        dir.write("vieja/Recortes web/2026-01-02 Nota.md", "Una nota inventada que se movió con la carpeta.".as_bytes());
        dir.write("vieja/Recortes web/2026-01-03 Quedó.md", "Una nota inventada que quedó en la carpeta vieja.".as_bytes());
        let base = DocBase::in_memory().unwrap();
        let old = base.ensure_work_origin(NAMES, &dir.path().join("vieja"), APP_FOLDERS).unwrap();
        settle(&base, &old.origin.id);
        let moved_id: i64 = base
            .conn()
            .query_row("SELECT id FROM location WHERE name = '2026-01-02 Nota.md'", [], |row| row.get(0))
            .unwrap();
        // The person moves one file to the new folder and chooses it.
        dir.write("nueva/Recortes web/2026-01-02 Nota.md", &std::fs::read(dir.path().join("vieja/Recortes web/2026-01-02 Nota.md")).unwrap());
        let new = base.ensure_work_origin(NAMES, &dir.path().join("nueva"), APP_FOLDERS).unwrap();
        assert_eq!(new.origin.id, old.origin.id);
        assert_eq!(new.origin.root(), dir.path().join("nueva"));
        assert_eq!(base.origins(&Visible::All).unwrap().len(), 1);
        let left: Vec<i64> = {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT id FROM location").unwrap();
            statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
        };
        assert_eq!(left, vec![moved_id], "what is still there keeps its place; the rest leaves");
        assert_eq!(count(&base, "SELECT count(*) FROM change WHERE kind = 'removed'"), 1);
        let report = base.scan_origin(&new.origin.id).unwrap();
        assert!(report.reachable);
        assert_eq!((report.added, report.removed), (0, 0));
    }

    #[test]
    fn a_new_folder_that_holds_the_old_one_takes_it_in() {
        let dir = TempDir::new();
        dir.write("arriba/micelya/Recortes web/2026-01-02 Nota.md", "Una nota inventada para la prueba.".as_bytes());
        let base = DocBase::in_memory().unwrap();
        let old = base.ensure_work_origin(NAMES, &dir.path().join("arriba/micelya"), APP_FOLDERS).unwrap();
        settle(&base, &old.origin.id);
        let new = base.ensure_work_origin(NAMES, &dir.path().join("arriba"), APP_FOLDERS).unwrap();
        assert_eq!(new.joined, vec!["Carpeta de trabajo".to_string()]);
        assert_eq!(new.origin.name, "Carpeta de trabajo");
        assert_eq!(base.origins(&Visible::All).unwrap().len(), 1);
        assert_eq!(paths(&base), vec![(new.origin.id.clone(), "micelya/Recortes web/2026-01-02 Nota.md".to_string())]);
    }

    #[test]
    fn the_new_files_of_a_folder_are_the_ones_after_it_was_seen() {
        let dir = TempDir::new();
        dir.write("trabajo/Recortes web/2026-01-02 Nota.md", "Una nota inventada para la prueba.".as_bytes());
        dir.write("trabajo/Expedientes/otro.txt", "Algo que no es un recorte.".as_bytes());
        let base = DocBase::in_memory().unwrap();
        let work = base.ensure_work_origin(NAMES, &dir.path().join("trabajo"), APP_FOLDERS).unwrap();
        settle(&base, &work.origin.id);
        let clips = dir.path().join("trabajo/Recortes web");
        let new = base.files_new_since(&clips, "clips-seen-at", 10).unwrap();
        assert_eq!(new.total, 1);
        assert_eq!(new.files[0].name, "2026-01-02 Nota.md");
        assert_eq!(new.files[0].origin_name, "Carpeta de trabajo");
        base.mark_seen("clips-seen-at").unwrap();
        assert_eq!(base.files_new_since(&clips, "clips-seen-at", 10).unwrap().total, 0);
        // A folder no origin holds has nothing new.
        assert_eq!(base.files_new_since(&dir.folder("fuera"), "clips-seen-at", 10).unwrap().total, 0);
    }
}
