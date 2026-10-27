//! The references of a document (D-377): the number of a record, a file,
//! "the contract of X". They are not entities: they are data of the
//! document, as it writes them, with the quote and the page they are in,
//! searchable, and evidence to group documents into matters.
//!
//! They come from the reading that already exists: the mentions of the
//! type `matters.kind` of the vocabulary. No rule of its own finds them.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;
use serde::Serialize;

use super::vocabulary::Vocabulary;
use super::IdentifierView;
use crate::docbase::sql_err;

/// One mention of a reference in a document.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Reference {
    pub content_id: i64,
    pub mention_id: i64,
    pub written: String,
    pub normalized: String,
    pub role: String,
    pub quote: String,
    pub page: Option<i64>,
}

/// A reference as the detail of a document shows it: one per normalized
/// form, with every way the document writes it and every quote.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentReference {
    pub written: String,
    pub normalized: String,
    pub also_written: Vec<String>,
    pub role: String,
    pub quotes: Vec<ReferenceQuote>,
    /// The numbers the reading found in it (a record, an invoice).
    pub identifiers: Vec<IdentifierView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceQuote {
    pub quote: String,
    pub page: Option<i64>,
}

/// The references of one document, or of all (`None`), in the order the
/// documents were read. A term the document defines and a way of writing
/// that is not a name (only common words, a file) are not references.
pub(crate) fn document_references(conn: &Connection, vocabulary: &Vocabulary, content: Option<i64>) -> Result<Vec<Reference>, String> {
    let mut statement = conn
        .prepare(
            "SELECT version_id, id, written, normalized, role, quote, page FROM knowledge_mention
             WHERE kind = ?1 AND defined = 0 AND (?2 IS NULL OR version_id = ?2) ORDER BY version_id, id",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map(rusqlite::params![vocabulary.matters.kind, content], |row| {
            Ok(Reference {
                content_id: row.get(0)?,
                mention_id: row.get(1)?,
                written: row.get(2)?,
                normalized: row.get(3)?,
                role: row.get(4)?,
                quote: row.get(5)?,
                page: row.get(6)?,
            })
        })
        .map_err(sql_err)?;
    let mut out = Vec::new();
    for row in rows {
        let reference = row.map_err(sql_err)?;
        if !reference.normalized.trim().is_empty() && vocabulary.is_a_name(&vocabulary.matters.kind, &reference.written) {
            out.push(reference);
        }
    }
    Ok(out)
}

/// The documents whose references cite a record by its number and its
/// year, by their normalized form (the same filters as
/// `document_references`): the number, as one group of digits or as several
/// ("12.345"), followed by the year in four digits or two. Citing a record
/// is not being of it: another, related record is cited too.
pub(crate) fn documents_citing(conn: &Connection, vocabulary: &Vocabulary, number: u64, year: u32) -> Result<BTreeSet<i64>, String> {
    let short = format!("%{:02}%", year % 100);
    let mut statement = conn
        .prepare(
            "SELECT version_id, written, normalized FROM knowledge_mention
             WHERE kind = ?1 AND defined = 0 AND (normalized LIKE ?2 OR written LIKE ?2)",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map(rusqlite::params![vocabulary.matters.kind, short], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .map_err(sql_err)?;
    let mut out = BTreeSet::new();
    for row in rows {
        let (content, written, normalized) = row.map_err(sql_err)?;
        if normalized.trim().is_empty() || !vocabulary.is_a_name(&vocabulary.matters.kind, &written) {
            continue;
        }
        if cites(&normalized, number, year) || cites(&written, number, year) {
            out.insert(content);
        }
    }
    Ok(out)
}

/// Whether a text writes a record's number followed by its year.
pub(crate) fn cites(text: &str, number: u64, year: u32) -> bool {
    let tokens: Vec<&str> = text.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
    let is_year = |t: &str| match t.len() {
        4 => t.parse::<u32>().ok() == Some(year),
        2 => t.parse::<u32>().ok() == Some(year % 100),
        _ => false,
    };
    for start in 0..tokens.len() {
        let mut digits = String::new();
        for (end, token) in tokens.iter().enumerate().skip(start) {
            if !token.chars().all(|c| c.is_ascii_digit()) {
                break;
            }
            if end > start && digits.parse::<u64>().ok() == Some(number) && is_year(token) {
                return true;
            }
            digits.push_str(token);
            if digits.len() > 18 {
                break;
            }
        }
    }
    false
}

/// The record a reference cites by number and year (P-330): a number,
/// with or without thousands dots, then a slash or a dash and a year of
/// four digits or two ("1.234/2020", "1234-20"). None when it cites none
/// or more than one, or when the number goes on (a date, an incident:
/// "12/05/2020", "1234/2020/1"): the same record written several ways is
/// then said once.
pub(crate) fn record_cited(text: &str) -> Option<(u64, u32)> {
    let text = text.replace(" / ", "/").replace("/ ", "/").replace(" /", "/");
    let chars: Vec<char> = text.chars().collect();
    let mut found: Option<(u64, u32)> = None;
    let mut at = 0;
    while at < chars.len() {
        if !chars[at].is_ascii_digit() || (at > 0 && chars[at - 1].is_ascii_alphanumeric()) {
            at += 1;
            continue;
        }
        // A run of digits and the separators between them.
        let mut end = at;
        while end < chars.len()
            && (chars[end].is_ascii_digit() || (matches!(chars[end], '.' | '/' | '-') && chars.get(end + 1).is_some_and(|c| c.is_ascii_digit())))
        {
            end += 1;
        }
        let run: String = chars[at..end].iter().collect();
        at = end;
        if chars.get(end).is_some_and(|c| c.is_ascii_alphanumeric()) {
            continue;
        }
        let parts: Vec<&str> = run.split(['/', '-']).collect();
        let [number, year] = parts[..] else { continue };
        let groups: Vec<&str> = number.split('.').collect();
        let grouped = groups.len() == 1 || ((1..=3).contains(&groups[0].len()) && groups[1..].iter().all(|g| g.len() == 3));
        let digits: String = groups.concat();
        let year_fits = match year.len() {
            4 => year.parse::<u32>().is_ok_and(|y| (1900..=2099).contains(&y)),
            2 => true,
            _ => false,
        };
        if !grouped || !year_fits || digits.is_empty() || digits.len() > 18 {
            continue;
        }
        let (Ok(number), Ok(year)) = (digits.parse::<u64>(), year.parse::<u32>()) else { continue };
        match found {
            Some(seen) if !same_record(seen, (number, year)) => return None,
            Some(_) => {}
            None => found = Some((number, year)),
        }
    }
    found
}

/// Whether two records cited are the same: the same number and year, a
/// year of two digits being its last two.
pub(crate) fn same_record(a: (u64, u32), b: (u64, u32)) -> bool {
    a.0 == b.0 && (a.1 == b.1 || (a.1.min(b.1) < 100 && a.1 % 100 == b.1 % 100))
}

/// The references of one document, one per normalized form, with the
/// identifiers read in them.
pub(crate) fn shown_references(
    conn: &Connection,
    vocabulary: &Vocabulary,
    content: i64,
    identifiers: &BTreeMap<i64, Vec<IdentifierView>>,
) -> Result<Vec<DocumentReference>, String> {
    let mut by_form: BTreeMap<String, DocumentReference> = BTreeMap::new();
    let mut order: Vec<String> = Vec::new();
    for reference in document_references(conn, vocabulary, Some(content))? {
        let entry = by_form.entry(reference.normalized.clone()).or_insert_with(|| {
            order.push(reference.normalized.clone());
            DocumentReference {
                written: reference.written.clone(),
                normalized: reference.normalized.clone(),
                also_written: Vec::new(),
                role: reference.role.clone(),
                quotes: Vec::new(),
                identifiers: Vec::new(),
            }
        });
        if reference.written != entry.written && !entry.also_written.contains(&reference.written) {
            entry.also_written.push(reference.written.clone());
        }
        if !entry.quotes.iter().any(|q| q.quote == reference.quote) {
            entry.quotes.push(ReferenceQuote { quote: reference.quote.clone(), page: reference.page });
        }
        for identifier in identifiers.get(&reference.mention_id).into_iter().flatten() {
            if !entry.identifiers.iter().any(|i| i.kind == identifier.kind && i.value == identifier.value) {
                entry.identifiers.push(identifier.clone());
            }
        }
    }
    Ok(order.into_iter().filter_map(|form| by_form.remove(&form)).collect())
}

/// The way the documents write a reference most, in the form of a name
/// when they write it in capitals (as an entity's name was, D-239).
pub(crate) fn written_form(conn: &Connection, vocabulary: &Vocabulary, normalized: &str) -> Result<Option<String>, String> {
    let mut statement = conn
        .prepare_cached(
            "SELECT written FROM knowledge_mention WHERE kind = ?1 AND normalized = ?2
             GROUP BY written ORDER BY count(*) DESC, length(written) DESC, written",
        )
        .map_err(sql_err)?;
    let forms: Vec<String> = statement
        .query_map(rusqlite::params![vocabulary.matters.kind, normalized], |row| row.get(0))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    Ok(forms.first().map(|first| super::names::shown_name(first, forms.iter().map(String::as_str))))
}
