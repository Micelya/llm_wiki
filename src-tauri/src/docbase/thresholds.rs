//! The thresholds of the base, in its configuration and not in the code
//! (D-722): settings of the base (`docbase_setting`), like the registry of
//! the engines (`services.rs`), each with its value by default.
//!
//! - Of the reading: the low threshold (0,90, D-706), the high one of the
//!   bands (0,97, D-668) and the quality: the share of firm text below
//!   which a document is «no legible» (D-705, P-520). The bands and the
//!   quality are used by the reading by bands, when it enters the app.
//! - Of the failures: the threshold of the indicator, attempts and hours
//!   (D-714), kept where the first stage of 1 nov left them.
//! - Of the round of questions that go by themselves: its cap (D-484).
//! - Of the retries: the waits of a transient failure, in minutes; past
//!   the last, the last again, without end.
//!
//! The least words to type (10) and to compare (30) stay in the code: the
//! owner did not ask for them.

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::{failures, sql_err, DocBase};

pub const LOW: &str = "reading:low-threshold";
pub const HIGH: &str = "reading:high-threshold";
pub const QUALITY: &str = "reading:quality-threshold";
pub const ROUND_CAP: &str = "auto-questions:cap";
pub const RETRY_MINUTES: &str = "failures:retry-minutes";

/// The share of firm text below which a document is «no legible», by
/// default (P-520): provisional, assumed in the unattended run of 1 nov
/// until its value is measured on the readings of the test folder (D-741).
pub const DEFAULT_QUALITY: f64 = 0.5;

/// Every threshold, with its value now.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    pub low: f64,
    pub high: f64,
    pub quality: f64,
    pub indicator_attempts: i64,
    pub indicator_hours: i64,
    pub round_cap: usize,
    pub retry_minutes: Vec<i64>,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            low: 0.90,
            high: 0.97,
            quality: DEFAULT_QUALITY,
            indicator_attempts: 3,
            indicator_hours: 6,
            round_cap: super::auto_questions::CAP,
            retry_minutes: vec![10, 60, 6 * 60, 24 * 60],
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
        indicator_attempts,
        indicator_hours,
        round_cap: round_cap(conn)?,
        retry_minutes: retry_minutes(conn)?,
    })
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
    if !(0.0..=1.0).contains(&t.quality) {
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
            (failures::INDICATOR_ATTEMPTS, t.indicator_attempts.to_string()),
            (failures::INDICATOR_HOURS, t.indicator_hours.to_string()),
            (ROUND_CAP, t.round_cap.to_string()),
            (RETRY_MINUTES, retry),
        ];
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        for (key, value) in values {
            tx.execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![key, value]).map_err(sql_err)?;
        }
        tx.commit().map_err(sql_err)
    }
}
