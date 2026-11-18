//! The share of safe words of each document, and the documents of a type
//! of low confidence (E3 of 2 nov; D-769, D-775).
//!
//! - **One rule for every step that asks** (the type, the reading, «En
//!   curso»): a document with at least one word is asked (`ASKED_WORDS`,
//!   over `version.word_count`, the words of its text: firm and
//!   approximate). There is no other count: before, the type counted the
//!   words of the signature (without the fields of a form nor the pages the
//!   marks skip) and the reading every word of the text, both from 10.
//! - **The share of safe words:** the firm words over every word read, of
//!   any band (D-775, point 2): the firm and the approximate ones of its
//!   text with bands (E1 of 2 nov), and the ones the reader discarded on
//!   the pages that gave its text, from the reading by zones. A document
//!   without a scanned page has 1. It is kept with the signature (the step
//!   of the signature, `similar/store.rs`), as the derivative `safe-words`
//!   of the version: work of this computer, no question.
//! - **A document of a type of low confidence** has a share below the
//!   threshold of the type of low confidence, or below the one of «no
//!   legible», which is also treated as of low confidence (the assumption
//!   on the two thresholds of the plan of 2 nov). Told when looking, with
//!   the thresholds of now (`thresholds.rs`): moving them asks nothing
//!   again by itself. It is still asked its type, with its approximate
//!   words; what comes out of it is used in no process and for no derived
//!   information (D-769): it is only for queries (search it, open it, see
//!   it). Where it is set aside: `process::steps::low_confidence`.
//!
//! Production questions per document (D-672): 0 here. The rule of one word
//! adds the type and the reading of the documents with 1 to 9 words that
//! are not of low confidence.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use super::sql_err;
use crate::source_volume::bands;
use crate::source_volume::catalog::{is_page_heading, normalized_words, RECOGNIZED_MARKER_START};

/// The derivative of a version with its count of words by band.
pub(crate) const KIND: &str = "safe-words";
/// A document with at least this many words is asked (the type and the
/// reading), with its approximate words when it has few safe ones.
pub const ASKED_WORDS: i64 = 1;
/// What the marker of a text read by zones says (`recognize::zones_text_engine`):
/// its discarded lines are in its reading by zones.
const BY_ZONES: &str = " por zonas";

/// The words of a document by band.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafeWords {
    pub firm: usize,
    pub approximate: usize,
    pub discarded: usize,
}

impl SafeWords {
    pub fn total(&self) -> usize {
        self.firm + self.approximate + self.discarded
    }

    /// The firm words over every word read; `None` without words.
    pub fn share(&self) -> Option<f64> {
        let total = self.total();
        (total > 0).then(|| self.firm as f64 / total as f64)
    }
}

/// The words of a text that count as its words (`version.word_count`):
/// every word of it, firm or approximate, without its headings and markers.
pub fn words_of(text: &str) -> usize {
    normalized_words(text).len()
}

/// The pages of a text whose text a reader by zones gave: their discarded
/// lines count. A text without page headings (an image) is page 1.
fn pages_by_zones(text: &str) -> BTreeSet<u32> {
    let mut page = 1;
    let mut out = BTreeSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if is_page_heading(trimmed) {
            page = trimmed["## Page ".len()..].parse().unwrap_or(page);
        } else if trimmed.starts_with(RECOGNIZED_MARKER_START) && trimmed.contains(BY_ZONES) {
            out.insert(page);
        }
    }
    out
}

/// The words of a text by band: the firm and the approximate ones of the
/// text, and the discarded ones of its pages read by zones
/// (`discarded_on`, by page).
pub fn count(text: &str, discarded_on: &BTreeMap<u32, usize>) -> SafeWords {
    let all = words_of(text);
    let firm = if bands::has_approximate(text) { words_of(&bands::firm(text)) } else { all };
    let discarded = pages_by_zones(text).iter().filter_map(|page| discarded_on.get(page)).sum();
    SafeWords { firm, approximate: all.saturating_sub(firm), discarded }
}

/// The discarded words of each page that gave its text, in a reading by
/// zones (`structure`: the JSON list of its pages).
pub fn discarded_by_page(zones: &str) -> BTreeMap<u32, usize> {
    let pages: Vec<serde_json::Value> = serde_json::from_str(zones).unwrap_or_default();
    let mut out = BTreeMap::new();
    for page in pages {
        if page["forma"]["se_lee"].as_bool() != Some(true) {
            continue;
        }
        let number = page["page"].as_u64().unwrap_or(1) as u32;
        let words: usize = page["zonas"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|zone| zone["renglones"].as_array().into_iter().flatten())
            .filter(|row| row["franja"].as_str() == Some("descartada"))
            .map(|row| normalized_words(row["texto"].as_str().unwrap_or_default()).len())
            .sum();
        out.insert(number, words);
    }
    out
}

/// The words by band of a version with this text: its discarded words come
/// from the reading by zones of a file it is in (a part, of the file it is
/// cut from), when it has one.
pub(crate) fn measure(conn: &Connection, version: i64, text: &str) -> Result<SafeWords, String> {
    let zones: Option<String> = conn
        .query_row(
            "SELECT z.body FROM derivative z JOIN file f ON f.version_id = z.version_id
             WHERE z.kind = ?2 AND f.hash IN (SELECT resource_hash FROM appearance WHERE version_id = ?1
                                              UNION SELECT hash FROM file WHERE version_id = ?1)
             ORDER BY z.created_at DESC LIMIT 1",
            params![version, crate::docbase::cut::by_zones::ZONES_KIND],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?;
    Ok(count(text, &zones.as_deref().map(discarded_by_page).unwrap_or_default()))
}

/// Keeps the words by band of a version.
pub(crate) fn save(tx: &Transaction<'_>, version: i64, words: &SafeWords, engine: &str) -> Result<(), String> {
    let body = serde_json::to_string(words).map_err(|e| e.to_string())?;
    tx.execute(
        "INSERT INTO derivative (version_id, kind, engine, body, created_at) VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT (version_id, kind) DO UPDATE SET engine = excluded.engine, body = excluded.body, created_at = excluded.created_at",
        params![version, KIND, engine, body, super::now_ms()],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// The words by band kept for a version.
pub fn of(conn: &Connection, version: i64) -> Result<Option<SafeWords>, String> {
    let body: Option<String> = conn
        .query_row("SELECT body FROM derivative WHERE version_id = ?1 AND kind = ?2", params![version, KIND], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(body.and_then(|b| serde_json::from_str(&b).ok()))
}

/// The share below which a document is of a type of low confidence now:
/// the threshold of the type of low confidence, or the one of «no
/// legible» when it is higher (a document that cannot be read is of low
/// confidence too).
pub(crate) fn limit(conn: &Connection) -> Result<f64, String> {
    let t = super::thresholds::thresholds(conn)?;
    Ok(t.low_confidence_type.max(t.quality))
}

/// SQL condition: the version `column` is of a type of low confidence with
/// the share `limit`.
pub(crate) fn low_sql(column: &str, limit: f64) -> String {
    let field = |name: &str| format!("coalesce(json_extract(sw.body, '$.{name}'), 0)");
    format!(
        "EXISTS (SELECT 1 FROM derivative sw WHERE sw.version_id = {column} AND sw.kind = '{KIND}'
                 AND {firm} < {limit:.6} * ({firm} + {approximate} + {discarded}))",
        firm = field("firm"),
        approximate = field("approximate"),
        discarded = field("discarded"),
    )
}

/// The documents of a type of low confidence now.
pub(crate) fn low_confidence(conn: &Connection) -> Result<BTreeSet<i64>, String> {
    let limit = limit(conn)?;
    let mut statement = conn
        .prepare(&format!("SELECT v.id FROM version v WHERE {}", low_sql("v.id", limit)))
        .map_err(sql_err)?;
    let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// Why a document is set aside, as «En curso» says it: «no legible» or of a
/// type of low confidence, with its share and the threshold.
pub(crate) fn reason(words: &SafeWords, low_confidence_type: f64, quality: f64) -> Option<String> {
    let share = words.share()?;
    let percent = |x: f64| (x * 100.0).round() as i64;
    if share < quality {
        Some(format!("no legible: {} % de palabras seguras (umbral {} %); sirve solo para consultas", percent(share), percent(quality)))
    } else if share < low_confidence_type {
        Some(format!(
            "tipo de confianza baja: {} % de palabras seguras (umbral {} %); sirve solo para consultas",
            percent(share),
            percent(low_confidence_type)
        ))
    } else {
        None
    }
}

/// The mark of one document of a type of low confidence now, with its
/// reason; `None` when it is not one. What the queries show (the search,
/// the sheet of the document): its reading exists and is only for queries
/// (E6 of 2 nov).
pub fn mark_of(conn: &Connection, version: i64) -> Result<Option<String>, String> {
    let t = super::thresholds::thresholds(conn)?;
    Ok(of(conn, version)?.and_then(|words| reason(&words, t.low_confidence_type, t.quality)))
}

/// The reason of each document of a type of low confidence now.
pub(crate) fn reasons(conn: &Connection) -> Result<BTreeMap<i64, String>, String> {
    let t = super::thresholds::thresholds(conn)?;
    let mut out = BTreeMap::new();
    for version in low_confidence(conn)? {
        if let Some(words) = of(conn, version)? {
            if let Some(why) = reason(&words, t.low_confidence_type, t.quality) {
                out.insert(version, why);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ZONES: &str = r#"[
        {"page": 1, "forma": {"se_lee": true}, "zonas": [{"renglones": [
            {"texto": "PARTIDA DE NACIMIENTO", "franja": "firme"},
            {"texto": "Hijo de Xandro", "franja": "aproximada"},
            {"texto": "ilegible borroso texto", "franja": "descartada"}]}]},
        {"page": 2, "forma": {"se_lee": false}, "zonas": [{"renglones": [
            {"texto": "no se leyo con el lector", "franja": "descartada"}]}]}
    ]"#;

    #[test]
    fn the_share_is_the_firm_words_over_every_word_read() {
        let text = "## Page 1\n\n<!-- texto-reconocido motor=\"rapidocr por zonas, bajo 0.9, alto 0.97\" -->\nPARTIDA DE NACIMIENTO\n[aproximado] Hijo de Xandro\n\n## Page 2\n\nTexto digital de la hoja\n";
        let discarded = discarded_by_page(ZONES);
        assert_eq!(discarded, [(1, 3)].into_iter().collect());
        let words = count(text, &discarded);
        // Firm: 3 of the scan and 5 digital; approximate 3; discarded 3 of
        // the page that gave its text (page 2 was not read by the reader).
        assert_eq!(words, SafeWords { firm: 8, approximate: 3, discarded: 3 });
        assert_eq!(words.share(), Some(8.0 / 14.0));
        // A digital text: everything is safe.
        assert_eq!(count("## Page 1\n\nUn contrato", &discarded).share(), Some(1.0));
        assert_eq!(count("", &discarded).share(), None);
    }

    #[test]
    fn the_reason_says_which_threshold() {
        let words = SafeWords { firm: 1, approximate: 3, discarded: 6 };
        assert!(reason(&words, 0.3, 0.5).unwrap().starts_with("no legible: 10 %"));
        let words = SafeWords { firm: 4, approximate: 6, discarded: 0 };
        assert!(reason(&words, 0.5, 0.3).unwrap().starts_with("tipo de confianza baja: 40 %"));
        assert_eq!(reason(&SafeWords { firm: 9, approximate: 1, discarded: 0 }, 0.3, 0.5), None);
    }
}
