//! The vectors of the documents (C1 of 2 nov): the step «Vectores» of the
//! process, after the signature and before the families and the type.
//!
//! - **What:** one vector per version that is a document (D-718), never a
//!   whole file of several documents. The text is the one the app read,
//!   with its bands (E1 of 2 nov): the firm lines, the approximate ones
//!   with their mark, never the discarded ones (they are not in the text),
//!   and without the comments of the machine (`<!-- … -->`).
//! - **Its condition:** every document has its row in `version_vector` with
//!   the model chosen in Ajustes, its numbers and the print of its text.
//!   A new document, a text that changed and another model in Ajustes leave
//!   it incomplete. A copy asks nothing: it is the same version. A text the
//!   base already has a vector of, with the same model, takes it without
//!   asking (the print is the same).
//! - **The documents of a type of low confidence** have their vector too:
//!   it serves for queries. What uses the vectors as a signal (C2) does not
//!   take them as examples (`process::steps::low_confidence`).
//! - **«Ninguno»** in Ajustes: the step is complete, nothing is asked.
//! - **Micelya Core** gives them (Cohere Embed V4, 1.024 numbers), 96 texts
//!   per call (`CoreClient::embed`), one call per operation of the loop.
//!   They wait for the automatic reading, as everything that goes to the
//!   cloud by itself. When Core fails, the step waits like any transient
//!   failure (D-714): its failure, with its class, holds it until the next
//!   attempt, and Atención tells it louder past the threshold of attempts
//!   or hours. A failure that is not transient waits for a person; choosing
//!   another engine in Ajustes takes it away (`services::set_ai_services`).
//! - **Queries per document:** about 0,013 calls to Core (one every 96
//!   documents), no question to a language model.

use std::collections::HashMap;

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use super::failures::{self, About, Failure};
use super::services::{self, VectorsConfig};
use super::{now_ms, sql_err, DocBase};
use crate::micelya_core::client::{CoreClient, Timing, EMBED_BATCH};
use crate::micelya_core::CoreConnection;

/// The stage of the record of failures of the step.
pub(crate) const STAGE: &str = "vectors";
/// What its failures are about: the step as a whole (a failure of Core is
/// of every document waiting, and never pauses the questions to the model).
pub(crate) fn about() -> About {
    About::Thread(STAGE.to_string())
}

/// The most characters of one text sent. The longest document of the
/// folder of the test has 86.073; Core takes up to 200.000.
const MAX_CHARS: usize = 180_000;
/// Core's kind of text for what is kept (`query` is what a person asks).
const INPUT_TYPE: &str = "document";
/// Said when there is no connection with Core: a transient failure, tried
/// again like one without connection.
pub(crate) const NO_CONNECTION: &str = "There is no connection with Micelya Core for the vectors: load it in Ajustes → Conexiones, or choose «Ninguno» in Ajustes → Servicios de IA → Calcular vectores";

/// The documents with their text: a version `c` and its text `d`.
const DOCUMENTS_FROM: &str = "version c JOIN derivative d ON d.version_id = c.id AND d.kind = 'text'";

/// SQL over `DOCUMENTS_FROM`: a document whose vector is missing or was
/// made with another model (`?1`), other numbers (`?2`) or another text.
fn left_where() -> String {
    format!(
        "c.state = 'processed' AND {document}
         AND NOT EXISTS (SELECT 1 FROM version_vector v WHERE v.version_id = c.id AND v.model = ?1 AND v.dimension = ?2
                           AND v.text_at = d.created_at)",
        document = super::cut::parts::is_document_sql("c.id"),
    )
}

/// The text of a document as it is turned into a vector.
pub(crate) fn text_for_vector(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        match rest[start..].find("-->") {
            Some(end) => rest = &rest[start + end + 3..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    let text = out.trim();
    match text.char_indices().nth(MAX_CHARS) {
        Some((cut, _)) => text[..cut].to_string(),
        None => text.to_string(),
    }
}

/// The print of a text: its SHA-256.
pub(crate) fn print_of(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

fn to_blob(vector: &[f32]) -> Vec<u8> {
    vector.iter().flat_map(|n| n.to_le_bytes()).collect()
}

pub(crate) fn from_blob(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect()
}

/// The engine of the vectors now: the one of Ajustes (`services`). In the
/// tests, only when a test sets its Core (`TEST_CORE`): the others work as
/// with «Ninguno», and nothing ever reads the keychain of this computer.
pub(crate) fn chosen(conn: &Connection) -> Result<Option<VectorsConfig>, String> {
    #[cfg(test)]
    if TEST_CORE.with(|core| core.borrow().is_none()) {
        return Ok(None);
    }
    services::vectors_choice(conn)
}

/// The Core of a test: a fake one (`micelya_core::fake`), none, or the
/// real one of the keychain (only the test of the real Core, ignored).
#[cfg(test)]
#[derive(Clone)]
pub(crate) enum TestCore {
    Fake(CoreConnection),
    Missing,
    Real,
}

#[cfg(test)]
thread_local! {
    pub(crate) static TEST_CORE: std::cell::RefCell<Option<TestCore>> = const { std::cell::RefCell::new(None) };
}

/// The calls of vectors made, for the tests.
#[cfg(test)]
pub(crate) static CALLS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// The connection with Core kept in the keychain, when it is complete.
fn saved_connection() -> Result<Option<(CoreConnection, Timing)>, String> {
    Ok(crate::micelya_core::saved_connection()?
        .filter(|c| !c.api.is_empty() && !c.issuer.is_empty() && !c.client_secret.is_empty())
        .map(|c| (c, Timing::default())))
}

#[cfg(not(test))]
fn connection_now() -> Result<Option<(CoreConnection, Timing)>, String> {
    saved_connection()
}

#[cfg(test)]
fn connection_now() -> Result<Option<(CoreConnection, Timing)>, String> {
    match TEST_CORE.with(|core| core.borrow().clone()) {
        Some(TestCore::Fake(connection)) => Ok(Some((connection, crate::micelya_core::fake::FakeCore::timing()))),
        Some(TestCore::Real) => saved_connection(),
        _ => Ok(None),
    }
}

/// How the step stands: its documents left (`free`, `versions`: SQL over
/// the version `c`, the failures and the scope), and those set aside: with
/// a failure of their own, or all of them while a failure of the step
/// holds. The third number: of the set aside, those that wait for a person
/// (a failure that is not tried again by itself).
pub(crate) fn left(conn: &Connection, free: &str, versions: &str) -> Result<(usize, usize, usize), String> {
    let Some(config) = chosen(conn)? else {
        return Ok((0, 0, 0));
    };
    let count = |held: &str| -> Result<usize, String> {
        conn.query_row(
            &format!("SELECT count(*) FROM {DOCUMENTS_FROM} WHERE {} AND {held} AND {versions}", left_where()),
            params![config.model, config.dimension as i64],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n.max(0) as usize)
        .map_err(sql_err)
    };
    let (pending, aside) = (count(free)?, count(&format!("NOT {free}"))?);
    if failures::holds(conn, STAGE, &about())? {
        let person = if waits_for_person(conn)? { pending + aside } else { 0 };
        return Ok((0, pending + aside, person));
    }
    Ok((pending, aside, 0))
}

/// Whether the failure of the step is not tried again by itself.
fn waits_for_person(conn: &Connection) -> Result<bool, String> {
    let found: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM failure WHERE stage = ?1 AND item_kind = 'thread' AND item = ?1 AND state = 'person' LIMIT 1",
            [STAGE],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?;
    Ok(found.is_some())
}

/// The vector kept of a version with the model chosen now, if it has one
/// made from its text of now; `Some(None)` for a document without text.
pub(crate) fn vector_of(conn: &Connection, version: i64) -> Result<Option<Option<Vec<f32>>>, String> {
    let Some(config) = chosen(conn)? else {
        return Ok(None);
    };
    let blob: Option<Option<Vec<u8>>> = conn
        .query_row(
            "SELECT v.vector FROM version_vector v JOIN derivative d ON d.version_id = v.version_id AND d.kind = 'text'
             WHERE v.version_id = ?1 AND v.model = ?2 AND v.dimension = ?3 AND v.text_at = d.created_at",
            params![version, config.model, config.dimension as i64],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?;
    Ok(blob.map(|b| b.map(|b| from_blob(&b))))
}

/// One document waiting for its vector.
struct Waiting {
    version: i64,
    text_at: i64,
    text: String,
    print: String,
}

impl DocBase {
    /// ONE operation of the step of the vectors: up to 96 documents, with
    /// at most one call to Core. Returns whether it did something (kept
    /// vectors, or wrote down a failure).
    pub(crate) fn vectors_operation(&self) -> Result<bool, String> {
        let (config, waiting, mut known) = {
            let conn = self.conn();
            let Some(config) = chosen(&conn)? else {
                return Ok(false);
            };
            if failures::holds(&conn, STAGE, &about())? {
                return Ok(false);
            }
            let free = super::process::steps::free_sql("c.id");
            let mut statement = conn
                .prepare(&format!(
                    "SELECT c.id, d.body, d.created_at FROM {DOCUMENTS_FROM} WHERE {} AND {free} ORDER BY c.id LIMIT ?3",
                    left_where()
                ))
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params![config.model, config.dimension as i64, EMBED_BATCH as i64], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
                })
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            let waiting: Vec<Waiting> = rows
                .into_iter()
                .map(|(version, body, text_at)| {
                    let text = text_for_vector(&body);
                    Waiting { version, text_at, print: print_of(&text), text }
                })
                .collect();
            // What the base already has for the same text and model: taken
            // without asking. A document without text has no vector.
            let mut known: HashMap<String, Option<Vec<u8>>> = HashMap::new();
            for item in &waiting {
                if known.contains_key(&item.print) {
                    continue;
                }
                if item.text.is_empty() {
                    known.insert(item.print.clone(), None);
                    continue;
                }
                let kept: Option<Option<Vec<u8>>> = conn
                    .query_row(
                        "SELECT vector FROM version_vector WHERE text_hash = ?1 AND model = ?2 AND dimension = ?3 AND vector IS NOT NULL LIMIT 1",
                        params![item.print, config.model, config.dimension as i64],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(sql_err)?;
                if let Some(Some(blob)) = kept {
                    known.insert(item.print.clone(), Some(blob));
                }
            }
            (config, waiting, known)
        };
        if waiting.is_empty() {
            return Ok(false);
        }

        // The texts the base has no vector of, each once.
        let mut asked: Vec<&Waiting> = Vec::new();
        for item in &waiting {
            if !known.contains_key(&item.print) && !asked.iter().any(|a| a.print == item.print) {
                asked.push(item);
            }
        }
        if !asked.is_empty() {
            let texts: Vec<String> = asked.iter().map(|item| item.text.clone()).collect();
            let answer = match connection_now()? {
                None => Err(NO_CONNECTION.to_string()),
                Some((connection, timing)) => CoreClient::new(connection, timing).map_err(|e| e.message()).and_then(|client| {
                    #[cfg(test)]
                    CALLS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    client.embed(&config.model, INPUT_TYPE, &texts, config.dimension).map_err(|e| e.message())
                }),
            };
            match answer {
                Ok(vectors) => {
                    for (item, vector) in asked.iter().zip(vectors) {
                        known.insert(item.print.clone(), Some(to_blob(&vector)));
                    }
                }
                Err(message) => {
                    // The step waits, with its class and its next attempt.
                    let mut failure = Failure::new(STAGE, about(), &message);
                    failure.engine = Some(config.engine.as_str());
                    failures::record(&self.conn(), &failure)?;
                    return Ok(true);
                }
            }
        }

        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        let now = now_ms();
        for item in &waiting {
            let vector = known.get(&item.print).cloned().flatten();
            tx.execute(
                "INSERT OR REPLACE INTO version_vector (version_id, model, dimension, text_hash, text_at, vector, computed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![item.version, config.model, config.dimension as i64, item.print, item.text_at, vector, now],
            )
            .map_err(sql_err)?;
        }
        tx.commit().map_err(sql_err)?;
        failures::resolve(&conn, STAGE, &about())?;
        Ok(true)
    }
}
