//! The barrier (D-719): while the base has work left that needs no AI, the
//! thread that asks the model asks only the question of the cut.
//!
//! The work without AI is the text (the bytes and the text of each file),
//! the scans the engine of this computer reads, the ficha of the pages, the
//! cut with rules and the parts, and the signatures, the similar documents
//! and the families. It holds the barrier for the whole base, also when a
//! new file arrives at a folder already processed (D-2 of 1 nov): until
//! that file did its own work without AI, nothing else is asked. The
//! reading a person asks for waits too (D-3 of 1 nov).
//!
//! What does not hold it: what waits for a person, what has a failure
//! written down, of any class (`failures.rs`), and what waits for the
//! question of the cut, which is the one that still goes out.
//!
//! One cheap query per piece of work, at the start of every turn of the
//! thread that asks; «En curso» shows the same counts.

use rusqlite::{params, Connection};
use serde::Serialize;

use super::{sql_err, DocBase};
use crate::source_volume::page_shape::SHAPES_ENGINE;

/// What work without AI is left, by kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Barrier {
    /// Files whose bytes or text are still to read.
    pub text: usize,
    /// Scans the engine of this computer is still to read.
    pub scans: usize,
    /// PDFs without the ficha of their pages.
    pub shapes: usize,
    /// Files still to cut with the rules, or whose parts are still to make.
    pub cut: usize,
    /// Documents still to sign, or to measure against the others.
    pub similar: usize,
}

impl Barrier {
    /// How much work without AI is left, all kinds together.
    pub fn left(&self) -> usize {
        self.text + self.scans + self.shapes + self.cut + self.similar
    }

    /// Whether the questions to the model, but the one of the cut, wait.
    pub fn holds(&self) -> bool {
        self.left() > 0
    }
}

/// SQL: no failure is open, of any class, about the version `version` (an
/// SQL value) or about a file it is in: what failed does not hold the
/// barrier.
pub(crate) fn free_sql(version: &str) -> String {
    format!(
        "NOT EXISTS (SELECT 1 FROM failure fb WHERE fb.state <> 'resolved' AND (
             (fb.item_kind = 'version' AND fb.item = CAST({version} AS TEXT))
             OR fb.file_hash IN (SELECT fc.hash FROM file fc WHERE fc.version_id = {version})
             OR fb.file_hash IN (SELECT ab.resource_hash FROM appearance ab WHERE ab.version_id = {version})))"
    )
}

fn count(conn: &Connection, sql: &str) -> Result<usize, String> {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0)).map(|n| n.max(0) as usize).map_err(sql_err)
}

/// The work without AI left in the base now.
pub(crate) fn barrier(conn: &Connection) -> Result<Barrier, String> {
    let excluded = super::excluded_sql!();
    let free = free_sql("c.id");
    let unread = count(
        conn,
        &format!(
            "SELECT count(*) FROM location l JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
             WHERE l.hash_state = 'pending' AND l.kind = 'file' AND NOT {excluded}"
        ),
    )?;
    let without_text = count(
        conn,
        &format!(
            "SELECT count(DISTINCT c.id) FROM version c JOIN file f ON f.version_id = c.id
             JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done'
             JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
             WHERE c.state = 'pending' AND NOT {excluded} AND {free}"
        ),
    )?;
    // A scan whose recognition failed (`error`) waits for a person.
    let scans = count(
        conn,
        &format!(
            "SELECT count(*) FROM version c WHERE c.state = 'needs-recognition' AND c.error IS NULL AND {} AND {free}",
            super::recognize::reachable_sql()
        ),
    )?;
    let shapes: i64 = conn
        .query_row(
            &format!(
                "SELECT count(DISTINCT c.id) FROM version c JOIN file f ON f.version_id = c.id
                 JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done' AND lower(l.ext) = 'pdf'
                 JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                 WHERE c.state IN ('processed', 'needs-recognition') AND NOT {excluded} AND {free}
                   AND NOT EXISTS (SELECT 1 FROM derivative d WHERE d.version_id = c.id AND d.kind = ?1 AND d.engine = ?2)"
            ),
            params![super::page_shapes::SHAPES_KIND, SHAPES_ENGINE],
            |row| row.get(0),
        )
        .map_err(sql_err)?;
    let cut = super::cut::rules_left(conn, &free)? + super::cut::parts::parts_left(conn, &free)?;
    let similar = super::similar::store::similar_left(conn, &free)?;
    Ok(Barrier { text: unread + without_text, scans, shapes: shapes.max(0) as usize, cut, similar })
}

impl DocBase {
    /// The work without AI left in the base now (D-719).
    pub fn barrier(&self) -> Result<Barrier, String> {
        barrier(&self.conn())
    }

    /// Whether the questions to the model, but the one of the cut, wait.
    /// A base that cannot say holds them: the order is never skipped.
    pub(crate) fn barrier_holds(&self) -> bool {
        match self.barrier() {
            Ok(barrier) => barrier.holds(),
            Err(error) => {
                eprintln!("[docbase] the work without AI left: {error}");
                true
            }
        }
    }

    /// For a question a person asks to the model that is not the one of the
    /// cut (D-3 of 1 nov): an error that says why it waits.
    pub(crate) fn barrier_check(&self) -> Result<(), String> {
        let barrier = self.barrier()?;
        if barrier.holds() {
            return Err(format!(
                "Las preguntas a la IA esperan: la base todavía hace trabajo sin IA ({} pendientes). Salen solas cuando termine.",
                barrier.left()
            ));
        }
        Ok(())
    }
}
