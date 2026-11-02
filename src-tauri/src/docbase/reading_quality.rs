//! «Calidad de la lectura» (E2 of 2 nov, P-544; etapa P): the documents
//! whose reading has few safe words, to choose the thresholds with data.
//!
//! - **Every document** by the one rule of what a document is
//!   (`query::totals`): each part of a file of several, or the file itself
//!   when it is one.
//! - **Its share of safe words** is the one the step of the signature kept
//!   (`safe_words.rs`): the firm words over every word read. A document
//!   below «no legible» or below the type of low confidence is of low
//!   confidence (`safe_words::limit`): only for queries.
//! - **The rule to choose a threshold:** how many documents and pages stay
//!   below each value, and what moving the two thresholds of the
//!   confidence would change (`confidence_effect`) before it is kept.
//!
//! Looking writes nothing (`look_writes_nothing.rs`). Production questions
//! per document (D-672): 0.

use std::collections::BTreeSet;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::cut::by_zones::ZONES_KIND;
use super::safe_words::{self, SafeWords};
use super::{excluded_sql, sql_err, DocBase};

/// A document as the screen lists it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityDocument {
    /// Its version: what «Leer con IA» and its screen open.
    pub version_id: i64,
    /// A place of its file, to open it; none when every place is excluded.
    pub location_id: Option<i64>,
    /// The name of its file.
    pub name: String,
    /// Where its file is: `<origin>/<path inside it>`.
    pub path: String,
    /// A part of a file of several: its pages in the file.
    pub first_page: Option<u32>,
    pub last_page: Option<u32>,
    /// How many pages it has, when the base knows.
    pub pages: Option<u32>,
    pub firm: usize,
    pub approximate: usize,
    pub discarded: usize,
    /// The firm words over every word read.
    pub share: f64,
    /// With the thresholds of now: `no-legible`, `low-confidence` or none.
    pub verdict: Option<String>,
}

/// How many documents and pages stay below a value.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QualityStep {
    pub value: f64,
    pub documents: usize,
    pub pages: usize,
}

/// What «Calidad de la lectura» shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingQuality {
    /// «No legible» and the type of low confidence, now.
    pub quality: f64,
    pub low_confidence_type: f64,
    /// Every document of the base, by the one rule.
    pub documents: usize,
    /// Of them, those with their share kept.
    pub measured: usize,
    /// The measured ones with a share below 1, the worst first.
    pub below_one: Vec<QualityDocument>,
    /// How many stay below 10 %, 20 %… 100 %, and below each threshold of now.
    pub steps: Vec<QualityStep>,
    /// Documents with words but without their share yet (the step of the
    /// signature did not reach them).
    pub unmeasured: Vec<QualityDocument>,
    /// Documents whose reading gave no word.
    pub without_text: Vec<QualityDocument>,
}

/// What moving the two thresholds of the confidence would change, by the
/// shares kept now: nothing is asked again to look.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidenceEffect {
    /// Documents that become of low confidence, and their pages.
    pub to_low: usize,
    pub to_low_pages: usize,
    /// Documents that stop being of low confidence, and their pages.
    pub from_low: usize,
    pub from_low_pages: usize,
    /// At most how many pages of people, organizations and matters are
    /// written again: what those documents name changes, and each page whose
    /// facts change is a question to «Preguntar al modelo» (its summary).
    pub pages_written_again: usize,
}

/// The verdict of a share with these thresholds.
pub fn verdict(share: f64, quality: f64, low_confidence_type: f64) -> Option<&'static str> {
    if share < quality {
        Some("no-legible")
    } else if share < low_confidence_type {
        Some("low-confidence")
    } else {
        None
    }
}

/// How many documents and pages have a share below `value`.
pub fn below(documents: &[QualityDocument], value: f64) -> QualityStep {
    let under: Vec<&QualityDocument> = documents.iter().filter(|d| d.share < value).collect();
    QualityStep { value, documents: under.len(), pages: under.iter().map(|d| d.pages.unwrap_or(1) as usize).sum() }
}

/// Every document by the one rule, with its words by band when kept.
fn documents(conn: &Connection) -> Result<Vec<(QualityDocument, Option<SafeWords>, Option<i64>)>, String> {
    let places = format!("SELECT l.file_hash FROM location l WHERE l.kind = 'file' AND NOT {}", excluded_sql!());
    let rows: Vec<(i64, String, String, Option<i64>, Option<String>)> = {
        let mut statement = conn
            .prepare(&format!(
                "SELECT a.version_id, min(a.resource_hash), min(a.selector), v.word_count,
                        (SELECT body FROM derivative d WHERE d.version_id = a.version_id AND d.kind = ?1)
                 FROM appearance a JOIN version v ON v.id = a.version_id
                 WHERE a.selector <> 'file' AND a.resource_hash IN ({places})
                 GROUP BY a.version_id ORDER BY a.version_id"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map([safe_words::KIND], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut out = Vec::with_capacity(rows.len());
    for (version, hash, selector, words, body) in rows {
        let place: Option<(i64, String, String, String)> = conn
            .query_row(
                &format!(
                    "SELECT l.id, l.name, o.name, l.rel_path FROM location l JOIN origin o ON o.id = l.origin_id
                     WHERE l.file_hash = ?1 AND l.kind = 'file' AND NOT {} ORDER BY l.id LIMIT 1",
                    excluded_sql!()
                ),
                [&hash],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let (first, last) = selector
            .strip_prefix("pages:")
            .and_then(|r| r.split_once('-'))
            .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)))
            .map_or((None, None), |(a, b)| (Some(a), Some(b)));
        let pages = match (first, last) {
            (Some(a), Some(b)) => Some(b.saturating_sub(a) + 1),
            _ => conn
                .query_row(
                    "SELECT json_array_length(z.body) FROM derivative z JOIN file f ON f.version_id = z.version_id
                     WHERE f.hash = ?1 AND z.kind = ?2 AND json_valid(z.body)",
                    params![hash, ZONES_KIND],
                    |row| row.get::<_, Option<i64>>(0),
                )
                .optional()
                .map_err(sql_err)?
                .flatten()
                .filter(|n| *n > 0)
                .map(|n| n as u32),
        };
        let kept: Option<SafeWords> = body.and_then(|b| serde_json::from_str(&b).ok());
        let words_by_band = kept.unwrap_or_default();
        let (location_id, name, path) = match place {
            Some((id, name, origin, rel)) => (Some(id), name, format!("{origin}/{rel}")),
            None => (None, String::new(), String::new()),
        };
        out.push((
            QualityDocument {
                version_id: version,
                location_id,
                name,
                path,
                first_page: first,
                last_page: last,
                pages,
                firm: words_by_band.firm,
                approximate: words_by_band.approximate,
                discarded: words_by_band.discarded,
                share: kept.and_then(|w| w.share()).unwrap_or(1.0),
                verdict: None,
            },
            kept,
            words,
        ));
    }
    Ok(out)
}

/// The values the rule lists: every tenth, and the thresholds of now.
fn values(quality: f64, low_confidence_type: f64) -> Vec<f64> {
    let mut list: Vec<f64> = (1..=10).map(|n| n as f64 / 10.0).collect();
    for value in [quality, low_confidence_type] {
        if value > 0.0 && !list.iter().any(|v| (v - value).abs() < 1e-9) {
            list.push(value);
        }
    }
    list.sort_by(|a, b| a.total_cmp(b));
    list
}

/// What «Calidad de la lectura» shows, with the thresholds of now.
pub fn reading_quality(conn: &Connection) -> Result<ReadingQuality, String> {
    let t = super::thresholds::thresholds(conn)?;
    let all = documents(conn)?;
    let total = all.len();
    let mut measured = 0;
    let mut below_one = Vec::new();
    let mut unmeasured = Vec::new();
    let mut without_text = Vec::new();
    for (mut document, kept, words) in all {
        match kept {
            Some(w) if w.total() > 0 => {
                measured += 1;
                if document.share < 1.0 {
                    document.verdict = verdict(document.share, t.quality, t.low_confidence_type).map(String::from);
                    below_one.push(document);
                }
            }
            Some(_) => without_text.push(document),
            None if words == Some(0) => without_text.push(document),
            None => unmeasured.push(document),
        }
    }
    below_one.sort_by(|a, b| a.share.total_cmp(&b.share).then_with(|| a.path.cmp(&b.path)).then(a.first_page.cmp(&b.first_page)));
    let steps = values(t.quality, t.low_confidence_type).into_iter().map(|v| below(&below_one, v)).collect();
    Ok(ReadingQuality {
        quality: t.quality,
        low_confidence_type: t.low_confidence_type,
        documents: total,
        measured,
        below_one,
        steps,
        unmeasured,
        without_text,
    })
}

/// What keeping `quality` and `low_confidence_type` would change, by the
/// shares kept now. A document is of low confidence below the higher of
/// the two (`safe_words::limit`).
pub fn confidence_effect(conn: &Connection, quality: f64, low_confidence_type: f64) -> Result<ConfidenceEffect, String> {
    let t = super::thresholds::thresholds(conn)?;
    let now = t.quality.max(t.low_confidence_type);
    let then = quality.max(low_confidence_type);
    let mut effect = ConfidenceEffect::default();
    let mut moving: BTreeSet<i64> = BTreeSet::new();
    for (document, kept, _) in documents(conn)? {
        if !kept.is_some_and(|w| w.total() > 0) {
            continue;
        }
        let (was, will) = (document.share < now, document.share < then);
        let pages = document.pages.unwrap_or(1) as usize;
        if will && !was {
            effect.to_low += 1;
            effect.to_low_pages += pages;
            moving.insert(document.version_id);
        } else if was && !will {
            effect.from_low += 1;
            effect.from_low_pages += pages;
            moving.insert(document.version_id);
        }
    }
    if !moving.is_empty() {
        let list = moving.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(",");
        effect.pages_written_again = conn
            .query_row(
                &format!(
                    "SELECT count(DISTINCT coalesce(m.entity_id, m.kind || ':' || m.normalized)) FROM knowledge_mention m
                     WHERE m.version_id IN ({list})"
                ),
                [],
                |row| row.get::<_, i64>(0),
            )
            .map_err(sql_err)? as usize;
    }
    Ok(effect)
}

impl DocBase {
    /// «Calidad de la lectura» (P-544).
    pub fn reading_quality(&self) -> Result<ReadingQuality, String> {
        reading_quality(&self.conn())
    }

    /// What keeping these two thresholds would change (P-542).
    pub fn confidence_effect(&self, quality: f64, low_confidence_type: f64) -> Result<ConfidenceEffect, String> {
        confidence_effect(&self.conn(), quality, low_confidence_type)
    }
}

#[cfg(test)]
#[path = "reading_quality_tests.rs"]
mod tests;
