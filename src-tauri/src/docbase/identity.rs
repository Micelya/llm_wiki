//! Identity: reading a file to learn which document it is.
//!
//! A location first gets the SHA-256 of its bytes. Two locations with
//! the same hash are the same file, and so the same content. Reading is
//! done a few files at a time, in the background: on a cloud drive it is
//! what downloads the file the first time.

use std::path::PathBuf;

use rusqlite::{params, OptionalExtension};

use crate::source_volume::file_hash::file_sha256;
use crate::source_volume::{LocalFolderProvider, SourceProvider};

use super::{now_ms, sql_err, DocBase};

struct Job {
    location_id: i64,
    origin_id: String,
    rel_path: String,
    path: PathBuf,
    size: i64,
    mtime_ms: i64,
}

impl DocBase {
    /// Reads up to `limit` locations whose bytes are still unknown.
    /// Returns how many were attended.
    pub fn hash_pending(&self, limit: usize) -> Result<usize, String> {
        let jobs = self.next_hash_jobs(limit)?;
        for job in &jobs {
            // An origin that went away mid-run is not a reading error:
            // leave the file pending for when it is back.
            if !job.path.exists() {
                let root_gone = self
                    .origin(&job.origin_id)?
                    .map(|origin| !origin.root().is_dir())
                    .unwrap_or(true);
                if root_gone {
                    self.conn()
                        .execute("UPDATE origin SET reachable = 0 WHERE id = ?1", [&job.origin_id])
                        .map_err(sql_err)?;
                    continue;
                }
            }
            let outcome = file_sha256(&job.path);
            self.apply_hash(job, outcome)?;
        }
        Ok(jobs.len())
    }

    fn next_hash_jobs(&self, limit: usize) -> Result<Vec<Job>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare(
                "SELECT l.id, l.origin_id, l.rel_path, o.location, l.size, l.mtime_ms
                 FROM location l JOIN origin o ON o.id = l.origin_id
                 WHERE l.hash_state = 'pending' AND l.kind = 'file' AND o.reachable = 1
                 ORDER BY l.size
                 LIMIT ?1",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([limit as i64], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                ))
            })
            .map_err(sql_err)?;
        let mut jobs = Vec::new();
        for row in rows {
            let (location_id, origin_id, rel_path, root, size, mtime_ms) = row.map_err(sql_err)?;
            let Some(path) = LocalFolderProvider::new(root).local_path(&rel_path) else {
                continue;
            };
            jobs.push(Job {
                location_id,
                origin_id,
                rel_path,
                path,
                size,
                mtime_ms,
            });
        }
        Ok(jobs)
    }

    fn apply_hash(&self, job: &Job, outcome: Result<Option<String>, String>) -> Result<(), String> {
        let now = now_ms();
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;

        // The sweep may have seen the file change, or go, while it was
        // being read: then this reading says nothing about what is there.
        let current: Option<(i64, i64, Option<String>)> = tx
            .query_row(
                "SELECT size, mtime_ms, file_hash FROM location WHERE id = ?1 AND hash_state = 'pending'",
                [job.location_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((size, mtime_ms, previous_hash)) = current else {
            return Ok(());
        };
        if size != job.size || mtime_ms != job.mtime_ms {
            return Ok(());
        }

        match outcome {
            Ok(Some(hash)) => {
                let known: Option<i64> = tx
                    .query_row("SELECT content_id FROM file WHERE hash = ?1", [&hash], |row| row.get(0))
                    .optional()
                    .map_err(sql_err)?;
                if known.is_none() {
                    tx.execute(
                        "INSERT INTO content (state, first_seen_at) VALUES ('pending', ?1)",
                        [now],
                    )
                    .map_err(sql_err)?;
                    tx.execute(
                        "INSERT INTO file (hash, content_id, size) VALUES (?1, ?2, ?3)",
                        params![hash, tx.last_insert_rowid(), size],
                    )
                    .map_err(sql_err)?;
                }
                tx.execute(
                    "UPDATE location SET file_hash = ?1, hash_state = 'done', hash_error = NULL WHERE id = ?2",
                    params![hash, job.location_id],
                )
                .map_err(sql_err)?;

                // A file that vanished from one path and shows up at
                // another with the same bytes was moved, not replaced.
                if previous_hash.is_none() {
                    let removed: Option<(i64, String, String)> = tx
                        .query_row(
                            "SELECT id, origin_id, rel_path FROM change
                             WHERE kind = 'removed' AND file_hash = ?1 AND detail IS NULL
                             ORDER BY id DESC LIMIT 1",
                            [&hash],
                            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                        )
                        .optional()
                        .map_err(sql_err)?;
                    if let Some((change_id, from_origin, from_path)) = removed {
                        tx.execute("UPDATE change SET detail = 'moved' WHERE id = ?1", [change_id])
                            .map_err(sql_err)?;
                        let from = serde_json::json!({ "originId": from_origin, "relPath": from_path });
                        tx.execute(
                            "INSERT INTO change (at, kind, origin_id, rel_path, file_hash, detail)
                             VALUES (?1, 'moved', ?2, ?3, ?4, ?5)",
                            params![now, job.origin_id, job.rel_path, hash, from.to_string()],
                        )
                        .map_err(sql_err)?;
                    }
                }
            }
            Ok(None) => {
                tx.execute(
                    "UPDATE location SET file_hash = NULL, hash_state = 'too-large', hash_error = NULL WHERE id = ?1",
                    [job.location_id],
                )
                .map_err(sql_err)?;
            }
            Err(error) => {
                tx.execute(
                    "UPDATE location SET hash_state = 'error', hash_error = ?1 WHERE id = ?2",
                    params![error, job.location_id],
                )
                .map_err(sql_err)?;
            }
        }
        tx.commit().map_err(sql_err)
    }

    /// Whether some file in a reachable origin is still waiting to be read.
    pub fn has_pending_hashes(&self) -> Result<bool, String> {
        self.conn()
            .query_row(
                "SELECT EXISTS (
                     SELECT 1 FROM location l JOIN origin o ON o.id = l.origin_id
                     WHERE l.hash_state = 'pending' AND l.kind = 'file' AND o.reachable = 1)",
                [],
                |row| row.get::<_, i64>(0),
            )
            .map(|exists| exists != 0)
            .map_err(sql_err)
    }

    /// Drops the documents that no location points to any more, and with
    /// them everything derived from them. Returns how many were dropped.
    ///
    /// Does nothing while files are still waiting to be read: one of them
    /// may be a document that was only moved.
    pub fn purge_orphans(&self) -> Result<usize, String> {
        if self.has_pending_hashes()? {
            return Ok(0);
        }
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        tx.execute(
            "DELETE FROM file WHERE hash NOT IN (SELECT file_hash FROM location WHERE file_hash IS NOT NULL)",
            [],
        )
        .map_err(sql_err)?;
        tx.execute(
            "DELETE FROM content_fts WHERE rowid IN (SELECT id FROM content WHERE id NOT IN (SELECT content_id FROM file))",
            [],
        )
        .map_err(sql_err)?;
        let dropped = tx
            .execute("DELETE FROM content WHERE id NOT IN (SELECT content_id FROM file)", [])
            .map_err(sql_err)?;
        tx.commit().map_err(sql_err)?;
        Ok(dropped)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;

    fn count(base: &DocBase, sql: &str) -> i64 {
        base.conn().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn settle(base: &DocBase, origin_id: &str) {
        base.scan_origin(origin_id).unwrap();
        while base.hash_pending(50).unwrap() > 0 {}
        base.purge_orphans().unwrap();
    }

    #[test]
    fn copies_with_other_names_and_folders_are_one_document() {
        let dir = TempDir::new();
        dir.write("o/escaneo/doc0042.pdf", b"la misma escritura");
        dir.write("o/socio/Escritura Perez.pdf", b"la misma escritura");
        dir.write("o/socio/otra.pdf", b"otra escritura");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);

        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE hash_state = 'done'"), 3);
        assert_eq!(count(&base, "SELECT count(*) FROM file"), 2);
        assert_eq!(count(&base, "SELECT count(*) FROM content"), 2);
        assert_eq!(count(&base, "SELECT count(DISTINCT file_hash) FROM location WHERE name IN ('doc0042.pdf', 'Escritura Perez.pdf')"), 1);
    }

    #[test]
    fn the_same_file_in_two_origins_is_still_one_document() {
        let dir = TempDir::new();
        dir.write("servidor/contrato.docx", b"contrato");
        dir.write("nube/copia de contrato.docx", b"contrato");
        let base = DocBase::in_memory().unwrap();
        let a = base.add_origin("Servidor", &dir.path().join("servidor"), &[]).unwrap();
        let b = base.add_origin("Nube", &dir.path().join("nube"), &[]).unwrap();
        settle(&base, &a.id);
        settle(&base, &b.id);

        assert_eq!(count(&base, "SELECT count(*) FROM content"), 1);
    }

    #[test]
    fn a_moved_file_keeps_its_document_and_is_logged_as_moved() {
        let dir = TempDir::new();
        dir.write("o/pendiente/carta.pdf", b"carta documento");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let content_before = count(&base, "SELECT id FROM content");
        base.conn()
            .execute("UPDATE content SET state = 'processed'", [])
            .unwrap();

        dir.write("o/BGH/carta documento 13-02.pdf", b"carta documento");
        dir.remove("o/pendiente/carta.pdf");
        settle(&base, &origin.id);

        assert_eq!(count(&base, "SELECT count(*) FROM location"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM content"), 1);
        // Same document, already processed: nothing is redone.
        assert_eq!(count(&base, "SELECT id FROM content WHERE state = 'processed'"), content_before);
        assert_eq!(count(&base, "SELECT count(*) FROM change WHERE kind = 'moved' AND rel_path = 'BGH/carta documento 13-02.pdf'"), 1);
    }

    #[test]
    fn a_document_without_locations_is_dropped_with_what_was_derived_from_it() {
        let dir = TempDir::new();
        dir.write("o/a.txt", b"documento a");
        dir.write("o/b.txt", b"documento b");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        {
            let conn = base.conn();
            conn.execute(
                "INSERT INTO derivative (content_id, kind, engine, body, created_at)
                 SELECT id, 'text', 'test', 'texto', 1 FROM content",
                [],
            )
            .unwrap();
            conn.execute("INSERT INTO content_fts (rowid, body) SELECT id, 'texto' FROM content", [])
                .unwrap();
        }

        dir.remove("o/a.txt");
        settle(&base, &origin.id);

        assert_eq!(count(&base, "SELECT count(*) FROM content"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM derivative"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM content_fts"), 1);
    }

    #[test]
    fn an_edited_file_becomes_another_document() {
        let dir = TempDir::new();
        dir.write("o/borrador.txt", b"version uno");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);
        let before: String = base
            .conn()
            .query_row("SELECT file_hash FROM location", [], |row| row.get(0))
            .unwrap();

        dir.write("o/borrador.txt", b"version dos, mas larga");
        settle(&base, &origin.id);

        let after: String = base
            .conn()
            .query_row("SELECT file_hash FROM location", [], |row| row.get(0))
            .unwrap();
        assert_ne!(before, after);
        assert_eq!(count(&base, "SELECT count(*) FROM content"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM file"), 1);
    }

    #[test]
    fn nothing_is_dropped_while_an_origin_is_unreachable() {
        let dir = TempDir::new();
        dir.write("o/a.txt", b"documento a");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        settle(&base, &origin.id);

        dir.remove_dir("o");
        settle(&base, &origin.id);

        assert_eq!(count(&base, "SELECT count(*) FROM location"), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM content"), 1);
    }

    #[test]
    fn a_file_that_cannot_be_read_is_marked_and_retried_on_the_next_sweep() {
        let dir = TempDir::new();
        dir.write("o/a.txt", b"documento a");
        let base = DocBase::in_memory().unwrap();
        let origin = base.add_origin("O", &dir.path().join("o"), &[]).unwrap();
        base.scan_origin(&origin.id).unwrap();
        // Gone between the sweep and the reading, with the origin still there.
        dir.remove("o/a.txt");
        dir.write("o/otro.txt", b"otro");
        base.hash_pending(50).unwrap();
        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE hash_state = 'error' AND hash_error IS NOT NULL"), 1);

        dir.write("o/a.txt", b"documento a");
        settle(&base, &origin.id);
        assert_eq!(count(&base, "SELECT count(*) FROM location WHERE hash_state = 'done'"), 2);
    }
}
