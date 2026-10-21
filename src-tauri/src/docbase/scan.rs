//! The sweep: compare what an origin holds now with what the base knew.
//!
//! An origin that cannot be read is not an empty origin. If the folder
//! is missing, or shows up empty when files were known in it, nothing is
//! concluded: what was known is kept until the origin can be read again.

use std::collections::HashMap;

use rusqlite::params;
use serde::Serialize;

use crate::source_volume::{LocalFolderProvider, SourceEntry, SourceProvider};

use super::{now_ms, sql_err, DocBase};

/// Extensions of the shortcuts a mounted cloud drive leaves in place of
/// its own document formats. They hold a link, not the document.
const CLOUD_LINK_EXTS: &[&str] = &[
    "gdoc", "gsheet", "gslides", "gdraw", "gform", "gmap", "gsite", "gjam", "gscript", "gtable",
];

/// Folders that never hold the user's documents.
const SKIPPED_DIRS: &[&str] = &[
    "$recycle.bin",
    "system volume information",
    "node_modules",
    "__macosx",
];

/// Files the system or an editor leaves around.
const SKIPPED_FILES: &[&str] = &["desktop.ini", "thumbs.db", "ehthumbs.db", ".ds_store"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanReport {
    pub origin_id: String,
    pub reachable: bool,
    pub total: usize,
    pub added: usize,
    pub modified: usize,
    pub removed: usize,
}

struct Known {
    id: i64,
    size: i64,
    mtime_ms: i64,
    file_hash: Option<String>,
}

impl DocBase {
    pub fn scan_origin(&self, origin_id: &str) -> Result<ScanReport, String> {
        let report = self.sweep(origin_id)?;
        // New files inherit the label rules of their folders.
        if report.added > 0 {
            self.relabel_origin(origin_id)?;
        }
        Ok(report)
    }

    fn sweep(&self, origin_id: &str) -> Result<ScanReport, String> {
        let origin = self
            .origin(origin_id)?
            .ok_or_else(|| "Ese origen ya no existe".to_string())?;

        // Listing can take long on a network or cloud folder: no lock here.
        let listed = LocalFolderProvider::new(origin.root())
            .list()
            .map(|entries| {
                entries
                    .into_iter()
                    .filter(|entry| is_inventoried(&entry.rel_path))
                    .collect::<Vec<_>>()
            });

        let now = now_ms();
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;

        let mut known: HashMap<String, Known> = HashMap::new();
        {
            let mut statement = tx
                .prepare("SELECT id, rel_path, size, mtime_ms, file_hash FROM location WHERE origin_id = ?1")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([origin_id], |row| {
                    Ok((
                        row.get::<_, String>(1)?,
                        Known {
                            id: row.get(0)?,
                            size: row.get(2)?,
                            mtime_ms: row.get(3)?,
                            file_hash: row.get(4)?,
                        },
                    ))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (rel_path, entry) = row.map_err(sql_err)?;
                known.insert(rel_path, entry);
            }
        }

        let entries = match listed {
            Ok(entries) if !(entries.is_empty() && !known.is_empty()) => entries,
            _ => {
                tx.execute(
                    "UPDATE origin SET reachable = 0, last_scan_at = ?1 WHERE id = ?2",
                    params![now, origin_id],
                )
                .map_err(sql_err)?;
                tx.commit().map_err(sql_err)?;
                return Ok(ScanReport {
                    origin_id: origin_id.to_string(),
                    reachable: false,
                    total: known.len(),
                    ..ScanReport::default()
                });
            }
        };

        let mut report = ScanReport {
            origin_id: origin_id.to_string(),
            reachable: true,
            total: entries.len(),
            ..ScanReport::default()
        };

        for entry in &entries {
            let size = entry.size as i64;
            match known.remove(&entry.rel_path) {
                None => {
                    let (folder, name, ext) = split(&entry.rel_path);
                    let link = CLOUD_LINK_EXTS.contains(&ext.as_str());
                    tx.execute(
                        "INSERT INTO location
                             (origin_id, rel_path, folder, name, ext, size, mtime_ms, kind, hash_state, first_seen_at, last_seen_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                        params![
                            origin_id,
                            entry.rel_path,
                            folder,
                            name,
                            ext,
                            size,
                            entry.mtime_ms,
                            if link { "link" } else { "file" },
                            if link { "skipped" } else { "pending" },
                            now
                        ],
                    )
                    .map_err(sql_err)?;
                    record_change(&tx, now, "added", origin_id, entry, None)?;
                    report.added += 1;
                }
                Some(before) if before.size != size || before.mtime_ms != entry.mtime_ms => {
                    // The old hash stays until the file is read again, so
                    // the document it was does not vanish in between.
                    tx.execute(
                        "UPDATE location
                         SET size = ?1, mtime_ms = ?2, hash_error = NULL,
                             hash_state = CASE kind WHEN 'link' THEN 'skipped' ELSE 'pending' END
                         WHERE id = ?3",
                        params![size, entry.mtime_ms, before.id],
                    )
                    .map_err(sql_err)?;
                    record_change(&tx, now, "modified", origin_id, entry, before.file_hash.as_deref())?;
                    report.modified += 1;
                }
                Some(_) => {}
            }
        }

        // Whatever is left was known and is no longer there.
        for (rel_path, gone) in &known {
            tx.execute("DELETE FROM location WHERE id = ?1", [gone.id])
                .map_err(sql_err)?;
            tx.execute(
                "INSERT INTO change (at, kind, origin_id, rel_path, file_hash) VALUES (?1, 'removed', ?2, ?3, ?4)",
                params![now, origin_id, rel_path, gone.file_hash],
            )
            .map_err(sql_err)?;
            report.removed += 1;
        }

        // A file that could not be read gets another try on every sweep.
        tx.execute(
            "UPDATE location SET hash_state = 'pending', hash_error = NULL
             WHERE origin_id = ?1 AND hash_state = 'error'",
            [origin_id],
        )
        .map_err(sql_err)?;
        tx.execute(
            "UPDATE location SET last_seen_at = ?1 WHERE origin_id = ?2",
            params![now, origin_id],
        )
        .map_err(sql_err)?;
        tx.execute(
            "UPDATE origin SET reachable = 1, last_scan_at = ?1 WHERE id = ?2",
            params![now, origin_id],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        Ok(report)
    }
}

fn record_change(
    tx: &rusqlite::Transaction<'_>,
    at: i64,
    kind: &str,
    origin_id: &str,
    entry: &SourceEntry,
    file_hash: Option<&str>,
) -> Result<(), String> {
    tx.execute(
        "INSERT INTO change (at, kind, origin_id, rel_path, file_hash) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![at, kind, origin_id, entry.rel_path, file_hash],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Folder, file name and lowercase extension of a forward-slash path.
pub(crate) fn split(rel_path: &str) -> (String, String, String) {
    let (folder, name) = match rel_path.rsplit_once('/') {
        Some((folder, name)) => (folder.to_string(), name.to_string()),
        None => (String::new(), rel_path.to_string()),
    };
    let ext = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => ext.to_lowercase(),
        _ => String::new(),
    };
    (folder, name, ext)
}

/// Whether a file belongs in the inventory. Everything the user could
/// have put there does; what the system, an editor or the app itself
/// leaves around does not.
fn is_inventoried(rel_path: &str) -> bool {
    let mut parts: Vec<&str> = rel_path.split('/').collect();
    let Some(name) = parts.pop() else {
        return false;
    };
    for dir in parts {
        let lower = dir.to_lowercase();
        if dir.starts_with('.') || SKIPPED_DIRS.contains(&lower.as_str()) {
            return false;
        }
    }
    let lower = name.to_lowercase();
    !(name.starts_with('.')
        || name.starts_with("~$")
        || SKIPPED_FILES.contains(&lower.as_str())
        || lower.ends_with(".tmp")
        || lower.ends_with(".crdownload")
        || lower.ends_with(".partial"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    fn count(base: &DocBase, sql: &str) -> i64 {
        base.conn().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    #[test]
    fn the_first_sweep_registers_every_file_with_its_folder() {
        let dir = TempDir::new();
        dir.write("casos/BOREAL/Reclamo/carta.pdf", b"carta");
        dir.write("casos/BOREAL/contrato.docx", b"contrato");
        dir.write("casos/suelto.txt", b"suelto");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("Casos", &dir.path().join("casos"), &[]).unwrap();

        let report = base.scan_origin(&origin.id).unwrap();
        assert_eq!((report.reachable, report.total, report.added), (true, 3, 3));

        let (folder, name, ext, state): (String, String, String, String) = base
            .conn()
            .query_row(
                "SELECT folder, name, ext, hash_state FROM location WHERE rel_path = 'BOREAL/Reclamo/carta.pdf'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!((folder.as_str(), name.as_str(), ext.as_str(), state.as_str()), ("BOREAL/Reclamo", "carta.pdf", "pdf", "pending"));
        assert_eq!(base.origin(&origin.id).unwrap().unwrap().reachable, Some(true));
    }

    #[test]
    fn a_second_sweep_sees_what_was_added_edited_and_deleted() {
        let dir = TempDir::new();
        dir.write("o/queda.txt", b"queda");
        dir.write("o/cambia.txt", b"antes");
        dir.write("o/se-va.txt", b"se va");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        base.conn()
            .execute("UPDATE location SET hash_state = 'done', file_hash = 'h-' || name", [])
            .unwrap();

        dir.write("o/cambia.txt", b"despues, mas largo");
        dir.write("o/nuevo.txt", b"nuevo");
        dir.remove("o/se-va.txt");
        let report = base.scan_origin(&origin.id).unwrap();

        assert_eq!((report.added, report.modified, report.removed), (1, 1, 1));
        assert_eq!(count(&base, "SELECT count(*) FROM location"), 3);
        // Only the edited and the new file have to be read again.
        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE hash_state = 'pending'"), 2);
        // The edited file keeps its old identity until it is read.
        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE name = 'cambia.txt' AND file_hash = 'h-cambia.txt'"), 1);
        // The deletion remembers which file it was, to recognise a move.
        assert_eq!(count(&base, "SELECT count(*) FROM change WHERE kind = 'removed' AND file_hash = 'h-se-va.txt'"), 1);
    }

    #[test]
    fn an_origin_that_disappears_or_shows_up_empty_keeps_what_was_known() {
        let dir = TempDir::new();
        dir.write("o/a.txt", b"a");
        dir.write("o/b.txt", b"b");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();

        // A cloud drive that is not signed in shows its folder empty.
        dir.remove("o/a.txt");
        dir.remove("o/b.txt");
        let report = base.scan_origin(&origin.id).unwrap();
        assert!(!report.reachable);
        assert_eq!(report.removed, 0);
        assert_eq!(count(&base, "SELECT count(*) FROM location"), 2);
        assert_eq!(base.origin(&origin.id).unwrap().unwrap().reachable, Some(false));

        // An unplugged disk: the folder is not there at all.
        dir.remove_dir("o");
        assert!(!base.scan_origin(&origin.id).unwrap().reachable);
        assert_eq!(count(&base, "SELECT count(*) FROM location"), 2);
        assert_eq!(count(&base, "SELECT count(*) FROM change WHERE kind = 'removed'"), 0);

        // Back again with one file less: now the deletion is real.
        dir.write("o/a.txt", b"a");
        let report = base.scan_origin(&origin.id).unwrap();
        assert_eq!((report.reachable, report.removed), (true, 1));
    }

    #[test]
    fn cloud_shortcuts_are_registered_as_detected_and_never_read() {
        let dir = TempDir::new();
        dir.write("o/Estrategia.gdoc", b"{\"doc_id\":\"x\"}");
        dir.write("o/Presupuesto.gsheet", b"{}");
        dir.write("o/real.pdf", b"pdf");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();

        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE kind = 'link' AND hash_state = 'skipped'"), 2);
        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE kind = 'file' AND hash_state = 'pending'"), 1);
    }

    #[test]
    fn system_leftovers_and_hidden_folders_stay_out_of_the_inventory() {
        assert!(is_inventoried("BOREAL/carta.pdf"));
        assert!(is_inventoried("sin extension"));
        assert!(!is_inventoried("desktop.ini"));
        assert!(!is_inventoried("BOREAL/Thumbs.db"));
        assert!(!is_inventoried("BOREAL/~$carta.docx"));
        assert!(!is_inventoried(".git/config"));
        assert!(!is_inventoried("proyecto/.oculta/cache.json"));
        assert!(!is_inventoried("fuentes/.cache/carta.pdf.txt"));
        assert!(!is_inventoried("$RECYCLE.BIN/x.pdf"));
        assert!(!is_inventoried("descarga.pdf.crdownload"));
    }

    #[test]
    fn paths_split_into_folder_name_and_extension() {
        assert_eq!(split("a/b/C.PDF"), ("a/b".into(), "C.PDF".into(), "pdf".into()));
        assert_eq!(split("suelto"), (String::new(), "suelto".into(), String::new()));
        assert_eq!(split(".oculto"), (String::new(), ".oculto".into(), String::new()));
    }
}
