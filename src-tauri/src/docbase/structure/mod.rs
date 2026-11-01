//! The reading by zones inside the app (D-725, the plan of the cut, part
//! 2): the zones of each page of a PDF by the model of this computer, its
//! lines with their box by the reader of this computer or by pdfium, and
//! each line with its band, with the thresholds of the configuration
//! (D-722). It is the reading the cut by zones (`cut/by_zones`) is made
//! with, kept as the derivative `zones` of the version: the reading and
//! the ficha of shape of each page (`forma`).
//!
//! While the setting `cut:zones-reading` is on (by default, D-746) the
//! cut of a PDF of two pages or more waits for its reading. A reading that
//! fails is a failure of the record (D-714) and the file waits: it is
//! never cut with the cut of before (D-717). Without the model of zones
//! or the reader, every PDF waiting says so and waits.
//!
//! Production questions per document (D-672): 0. Everything here runs on
//! this computer.

pub(crate) mod page;
pub(crate) mod reader;
pub(crate) mod words;

#[cfg(test)]
mod check_reading;
#[cfg(test)]
mod tests;

use std::path::PathBuf;

use rusqlite::{params, Connection, OptionalExtension};

use super::cut::by_zones::{zones_waiting_sql, ZONES_KIND, ZONES_STAGE};
use super::cut::{failed_sql, note_failure, note_success};
use super::failures;
use super::page_shapes::SHAPES_KIND;
use super::{sql_err, DocBase};
use crate::source_volume::page_shape::SHAPES_ENGINE;
use crate::source_volume::{LocalFolderProvider, SourceProvider};
use page::Bands;

/// The basis of a reading by zones: how it is read and with which
/// thresholds. A failure holds over its basis.
pub fn basis(bands: Bands) -> String {
    format!("{}|alto {}|bajo {}", reader::RECIPE, bands.high, bands.low)
}

/// The PDFs `f` (version `c`, location `l`, origin `o`) with the ficha of
/// their pages `d` and their reading by zones `z`, when they have one.
const FROM: &str = "file f JOIN version c ON c.id = f.version_id
     JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done' AND lower(l.ext) = 'pdf'
     JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
     JOIN derivative d ON d.version_id = c.id AND d.kind = ?1 AND d.engine = ?2
     LEFT JOIN derivative z ON z.version_id = c.id AND z.kind = ?3";

/// SQL: a PDF of `FROM` of two pages or more waits for its reading by
/// zones (the setting is on and it has none), and it did not fail over
/// `?4`, the basis.
fn waiting_where() -> String {
    format!(
        "c.state IN ('processed', 'needs-recognition') AND NOT {excluded} AND z.body IS NULL
         AND json_array_length(d.body, '$.pages') >= 2 AND {waiting}",
        excluded = super::excluded_sql!(),
        waiting = zones_waiting_sql(),
    )
}

/// How many PDFs wait for their reading by zones and `free` (an SQL
/// condition over the version `c`) says hold the barrier (`barrier.rs`).
pub(crate) fn zones_left(conn: &Connection, free: &str, bands: Bands) -> Result<usize, String> {
    conn.query_row(
        &format!(
            "SELECT count(DISTINCT f.hash) FROM {FROM} WHERE {} AND NOT {} AND {free}",
            waiting_where(),
            failed_sql(ZONES_STAGE, "f.hash", "?4")
        ),
        params![SHAPES_KIND, SHAPES_ENGINE, ZONES_KIND, basis(bands)],
        |row| row.get::<_, i64>(0),
    )
    .map(|n| n.max(0) as usize)
    .map_err(sql_err)
}

fn bands_of(conn: &Connection) -> Result<Bands, String> {
    let t = super::thresholds::thresholds(conn)?;
    Ok(Bands { high: t.high, low: t.low })
}

impl DocBase {
    /// The thresholds the pages are read with now.
    pub(crate) fn reading_bands(&self) -> Result<Bands, String> {
        bands_of(&self.conn())
    }

    /// The PDFs waiting for their reading by zones that did not fail.
    pub(crate) fn zones_waiting(&self, limit: usize) -> Result<Vec<(String, PathBuf)>, String> {
        let conn = self.conn();
        let bands = bands_of(&conn)?;
        let mut statement = conn
            .prepare(&format!(
                "SELECT f.hash, o.location, min(l.rel_path) FROM {FROM} WHERE {} AND NOT {}
                 GROUP BY f.hash ORDER BY f.hash LIMIT ?5",
                waiting_where(),
                failed_sql(ZONES_STAGE, "f.hash", "?4")
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(params![SHAPES_KIND, SHAPES_ENGINE, ZONES_KIND, basis(bands), limit as i64], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
            })
            .map_err(sql_err)?;
        let rows: Vec<(String, String, String)> = rows.collect::<Result<_, _>>().map_err(sql_err)?;
        Ok(rows
            .into_iter()
            .map(|(hash, root, rel)| {
                let path = LocalFolderProvider::new(root).local_path(&rel).unwrap_or_default();
                (hash, path)
            })
            .collect())
    }

    /// Keeps the reading by zones of a file (the JSON list of its pages),
    /// made with `engine`, for its version.
    pub(crate) fn put_zones_body(&self, hash: &str, body: &str, engine: &str) -> Result<(), String> {
        self.conn()
            .execute(
                "INSERT OR REPLACE INTO derivative (version_id, kind, engine, body, created_at)
                 SELECT version_id, ?2, ?3, ?4, ?5 FROM file WHERE hash = ?1",
                params![hash, ZONES_KIND, engine, body, super::now_ms()],
            )
            .map_err(sql_err)?;
        Ok(())
    }

    /// Reads by zones the next PDF waiting, and keeps its reading; or
    /// writes down why it failed, and the file waits (D-717). Returns
    /// whether there was one. Asks no model in the cloud.
    pub(crate) fn read_zones_step(&self) -> Result<bool, String> {
        let bands = self.reading_bands()?;
        let Some((hash, path)) = self.zones_waiting(1)?.into_iter().next() else {
            return Ok(false);
        };
        let basis = basis(bands);
        let read = crate::panic_guard::run_guarded("docbase_read_zones", || reader::read_pdf(&path, bands));
        let body = read.and_then(|pages| serde_json::to_string(&pages).map_err(|e| e.to_string()));
        match body {
            Ok(body) => {
                self.put_zones_body(&hash, &body, &basis)?;
                note_success(&self.conn(), ZONES_STAGE, &hash)?;
            }
            Err(error) => note_failure(&self.conn(), ZONES_STAGE, &hash, &basis, &format!("The reading by zones failed: {error}"))?,
        }
        Ok(true)
    }

    /// Without the model of zones or the reader of this computer, every PDF
    /// waiting for its reading waits with its failure (`why`), written
    /// once. Returns how many were written.
    pub(crate) fn zones_without_models(&self, why: &str) -> Result<usize, String> {
        let waiting: Vec<String> = self.zones_waiting(10_000)?.into_iter().map(|(hash, _)| hash).collect();
        let conn = self.conn();
        let basis = basis(bands_of(&conn)?);
        let mut noted = 0;
        for hash in &waiting {
            let about = failures::About::File(hash.clone());
            if failures::is_open(&conn, ZONES_STAGE, &about)? {
                continue;
            }
            let mut failure = failures::Failure::new(ZONES_STAGE, about, why);
            failure.cause = failures::cause("models-missing");
            failure.basis = Some(&basis);
            failures::record(&conn, &failure)?;
            noted += 1;
        }
        Ok(noted)
    }

    /// The model of zones and the reader are there: the PDFs that waited
    /// for them are read.
    pub(crate) fn zones_with_models(&self) -> Result<(), String> {
        failures::resolve_stage_cause(&self.conn(), ZONES_STAGE, "models-missing")
    }

    /// The reading by zones of a file, when it has one (the JSON list of
    /// its pages).
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn zones_of(&self, hash: &str) -> Result<Option<String>, String> {
        self.conn()
            .query_row(
                "SELECT z.body FROM derivative z JOIN file f ON f.version_id = z.version_id WHERE f.hash = ?1 AND z.kind = ?2",
                params![hash, ZONES_KIND],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)
    }
}
