//! The references of a document (D-377): the number of a record, a file,
//! "the contract of X". They are not entities: they are data of the
//! document, as it writes them, with the quote and the page they are in,
//! searchable, and evidence to group documents into matters.
//!
//! They come from the reading that already exists: the mentions of the
//! type `matters.kind` of the vocabulary. No rule of its own finds them.

use std::collections::BTreeMap;

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
            "SELECT content_id, id, written, normalized, role, quote, page FROM knowledge_mention
             WHERE kind = ?1 AND defined = 0 AND (?2 IS NULL OR content_id = ?2) ORDER BY content_id, id",
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
