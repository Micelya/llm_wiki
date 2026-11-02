//! The thresholds of the base, in its configuration and not in the code
//! (D-722): settings of the base (`docbase_setting`), like the registry of
//! the engines (`services.rs`), each with its value by default.
//!
//! - Of the reading: the low threshold (0,90, D-706), the high one of the
//!   bands (0,97, D-668) and the quality: the share of safe words below
//!   which a document is «no legible» (D-705, P-520).
//! - Of the type: the share of safe words below which a document is of a
//!   type of low confidence (D-769), 30 % provisional. A document below
//!   either of the two is of low confidence (`safe_words.rs`, E3 of 2 nov).
//! - Of the failures: the threshold of the indicator, attempts and hours
//!   (D-714), kept where the first stage of 1 nov left them.
//! - Of the round of questions that go by themselves: its cap (D-484).
//! - Of the retries: the waits of a transient failure, in minutes; past
//!   the last, the last again, without end.
//! - Of the identity: how far two names may be to join two mentions whose
//!   number is the same or near (E5 of 2 nov, note 2 of the coordinator):
//!   at most so many letters and at most so much of the name, both
//!   provisional (2 letters and 15 %, audit 1C).
//!
//! The least words to compare (30) stay in the code: the owner did not ask
//! for them. The 10 words to type and to read are gone (E3 of 2 nov): a
//! document with one word is asked (`safe_words::ASKED_WORDS`).

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{failures, sql_err, DocBase};

pub const LOW: &str = "reading:low-threshold";
pub const HIGH: &str = "reading:high-threshold";
pub const QUALITY: &str = "reading:quality-threshold";
pub const LOW_CONFIDENCE_TYPE: &str = "types:low-confidence-threshold";
pub const ROUND_CAP: &str = "auto-questions:cap";
pub const RETRY_MINUTES: &str = "failures:retry-minutes";
pub const NAME_LETTERS: &str = "identity:name-letters";
pub const NAME_SHARE: &str = "identity:name-share";

/// The share of firm text below which a document is «no legible», by
/// default (P-520): provisional, assumed in the unattended run of 1 nov
/// until its value is measured on the readings of the test folder (D-741).
pub const DEFAULT_QUALITY: f64 = 0.5;
/// The share of safe words below which a document is of a type of low
/// confidence, by default: provisional, until it is measured with the
/// sheet of types (decision 5 of 2 nov, D-769).
pub const DEFAULT_LOW_CONFIDENCE_TYPE: f64 = 0.30;

fn default_low_confidence_type() -> f64 {
    DEFAULT_LOW_CONFIDENCE_TYPE
}

/// How many letters two names may differ to join two mentions with the
/// same number or a near one, by default: provisional (audit 1C, E5 of 2
/// nov), until the long run of control measures it again.
pub const DEFAULT_NAME_LETTERS: usize = 2;
/// The share of the name those letters may be at most, by default: so a
/// short name does not take two changes. Provisional, as the letters.
pub const DEFAULT_NAME_SHARE: f64 = 0.15;

fn default_name_letters() -> usize {
    DEFAULT_NAME_LETTERS
}

fn default_name_share() -> f64 {
    DEFAULT_NAME_SHARE
}

/// Every threshold, with its value now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    pub low: f64,
    pub high: f64,
    pub quality: f64,
    /// The threshold of the type of low confidence (D-769).
    #[serde(default = "default_low_confidence_type")]
    pub low_confidence_type: f64,
    pub indicator_attempts: i64,
    pub indicator_hours: i64,
    pub round_cap: usize,
    pub retry_minutes: Vec<i64>,
    /// The caps of the names of the identity (E5 of 2 nov).
    #[serde(default = "default_name_letters")]
    pub name_letters: usize,
    #[serde(default = "default_name_share")]
    pub name_share: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            low: 0.90,
            high: 0.97,
            quality: DEFAULT_QUALITY,
            low_confidence_type: DEFAULT_LOW_CONFIDENCE_TYPE,
            indicator_attempts: 3,
            indicator_hours: 6,
            round_cap: super::auto_questions::CAP,
            retry_minutes: vec![10, 60, 6 * 60, 24 * 60],
            name_letters: DEFAULT_NAME_LETTERS,
            name_share: DEFAULT_NAME_SHARE,
        }
    }
}

fn setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [key], |row| row.get(0))
        .optional()
        .map_err(sql_err)
}

fn share(conn: &Connection, key: &str, default: f64) -> Result<f64, String> {
    Ok(setting(conn, key)?.and_then(|v| v.trim().parse().ok()).filter(|n: &f64| (0.0..=1.0).contains(n)).unwrap_or(default))
}

fn minutes(value: &str) -> Option<Vec<i64>> {
    let waits: Vec<i64> = value.split(',').map(|w| w.trim().parse().ok().filter(|n: &i64| *n > 0)).collect::<Option<_>>()?;
    (!waits.is_empty()).then_some(waits)
}

/// The thresholds as the configuration has them; each one missing or that
/// cannot be read, by default.
pub fn thresholds(conn: &Connection) -> Result<Thresholds, String> {
    let default = Thresholds::default();
    let (indicator_attempts, indicator_hours) = failures::indicator_threshold(conn)?;
    Ok(Thresholds {
        low: share(conn, LOW, default.low)?,
        high: share(conn, HIGH, default.high)?,
        quality: share(conn, QUALITY, default.quality)?,
        low_confidence_type: share(conn, LOW_CONFIDENCE_TYPE, default.low_confidence_type)?,
        indicator_attempts,
        indicator_hours,
        round_cap: round_cap(conn)?,
        retry_minutes: retry_minutes(conn)?,
        name_letters: name_letters(conn)?,
        name_share: share(conn, NAME_SHARE, default.name_share)?,
    })
}

/// How many letters two names may differ to join by a number.
pub fn name_letters(conn: &Connection) -> Result<usize, String> {
    Ok(setting(conn, NAME_LETTERS)?.and_then(|v| v.trim().parse().ok()).unwrap_or(DEFAULT_NAME_LETTERS))
}

/// The caps of the names of the identity: (letters, share of the name).
pub fn name_caps(conn: &Connection) -> Result<(usize, f64), String> {
    Ok((name_letters(conn)?, share(conn, NAME_SHARE, DEFAULT_NAME_SHARE)?))
}

/// The cap of the round of questions that go by themselves.
pub fn round_cap(conn: &Connection) -> Result<usize, String> {
    Ok(setting(conn, ROUND_CAP)?.and_then(|v| v.trim().parse().ok()).filter(|n: &usize| *n > 0).unwrap_or(super::auto_questions::CAP))
}

/// The waits of a transient failure, in minutes.
pub fn retry_minutes(conn: &Connection) -> Result<Vec<i64>, String> {
    Ok(setting(conn, RETRY_MINUTES)?.as_deref().and_then(minutes).unwrap_or_else(|| Thresholds::default().retry_minutes))
}

/// Why these thresholds cannot be kept, or `None`.
fn invalid(t: &Thresholds) -> Option<&'static str> {
    if !(0.0 < t.low && t.low < t.high && t.high <= 1.0) {
        return Some("El umbral bajo tiene que ser mayor que 0 y menor que el alto, y el alto, a lo sumo 1.");
    }
    if !(0.0..=1.0).contains(&t.quality) || !(0.0..=1.0).contains(&t.low_confidence_type) || !(0.0..=1.0).contains(&t.name_share) {
        return Some("La calidad es una proporción: entre 0 y 1.");
    }
    if t.indicator_attempts < 1 || t.indicator_hours < 1 || t.round_cap < 1 {
        return Some("Los intentos, las horas y el tope de la ronda tienen que ser 1 o más.");
    }
    if t.retry_minutes.is_empty() || t.retry_minutes.iter().any(|m| *m < 1) {
        return Some("Las esperas de los reintentos son minutos, 1 o más, separados por comas.");
    }
    None
}

impl DocBase {
    /// The thresholds of the base (D-722).
    pub fn thresholds(&self) -> Result<Thresholds, String> {
        thresholds(&self.conn())
    }

    /// Keeps the thresholds; the next step that uses one reads it.
    pub fn set_thresholds(&self, t: &Thresholds) -> Result<(), String> {
        if let Some(why) = invalid(t) {
            return Err(why.to_string());
        }
        let retry = t.retry_minutes.iter().map(|m| m.to_string()).collect::<Vec<_>>().join(",");
        let values = [
            (LOW, t.low.to_string()),
            (HIGH, t.high.to_string()),
            (QUALITY, t.quality.to_string()),
            (LOW_CONFIDENCE_TYPE, t.low_confidence_type.to_string()),
            (failures::INDICATOR_ATTEMPTS, t.indicator_attempts.to_string()),
            (failures::INDICATOR_HOURS, t.indicator_hours.to_string()),
            (ROUND_CAP, t.round_cap.to_string()),
            (RETRY_MINUTES, retry),
            (NAME_LETTERS, t.name_letters.to_string()),
            (NAME_SHARE, t.name_share.to_string()),
        ];
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        for (key, value) in values {
            tx.execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![key, value]).map_err(sql_err)?;
        }
        tx.commit().map_err(sql_err)
    }
}
