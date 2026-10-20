//! The search over the documents of the base (N5, D-350 to D-353).
//!
//! The text index of the base (`content_fts`) always goes over everything
//! that is visible; the graph of the base orders and widens what it finds,
//! and never cuts it (D-293). Only a set a person chose on purpose (for
//! example the documents of a matter) narrows the search.
//!
//! Four ranked lists are fused by reciprocal rank:
//!
//! - the words of the question in the text of each document (bm25), long
//!   words also by their stem (the word without its last two letters, so
//!   "contratistas" finds "contratista" and "tribunales" "tribunal");
//! - the numbers and identifiers of the question in every way they are
//!   written: `content_search.numbers` keeps each run of digits written
//!   with dots, dashes, slashes or letters in between as its digits alone
//!   ("12.345.678" and "12345678" are one number; "A00999-00012345" and
//!   "00999A00012345" too);
//! - the words of the question in the summary of each document
//!   (`content_search.summary`), which is written in the language of the
//!   archive: a question in Spanish finds a contract written in English;
//!   when the question has a number, only among the documents that have it;
//! - the graph: the people, organizations and matters the question names
//!   (by the names the record knows them by) raise the documents they appear
//!   in, and add them when the text did not find them; a name that appears
//!   in many more documents than its entity ("acuerdo", "cliente") does not
//!   count, nor an entity present in half the archive.
//!
//! Then the first documents found bring, with a small weight, the
//! documents the base relates to them (versions, same template, contained
//! in). Every list only holds documents with a visible location.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use serde::Serialize;

use super::knowledge::quote;
use super::knowledge::vocabulary::vocabulary;
use super::{sql_err, DocBase, Visible};

/// Reciprocal rank constant: small, because the lists are short and the
/// first places are the ones that matter.
const RANK_K: f64 = 10.0;
/// How many documents each list contributes.
const LIST_LIMIT: i64 = 400;
/// The graph widens from this many first documents.
const WIDEN_FROM: usize = 5;
/// The share of a document's score its related documents receive.
const WIDEN_WEIGHT: f64 = 0.03;
/// An entity in more documents than this raises them less (it orders less).
const SPECIFIC_DOCUMENTS: f64 = 8.0;
/// An entity in more than this share of the archive does not order at all.
const ENTITY_MAX_SHARE: f64 = 0.5;
/// Longest name looked up, in words.
const MAX_NAME_WORDS: usize = 6;
/// Words this long or longer are also searched by their stem.
const STEM_FROM_CHARS: usize = 6;
/// What the ids of the references the question names start with (D-377):
/// a reference is data of its documents, not an entity.
pub const REFERENCE_PREFIX: &str = "ref:";

/// A document found, with why.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentHit {
    pub content_id: i64,
    pub score: f64,
    /// The passage of the text that matched, or the topic of its summary.
    pub snippet: String,
    /// Where it was found: "text", "number", "summary", "graph", "related".
    pub found_by: Vec<String>,
    /// The entities named in the question that appear in it.
    pub entities: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSearch {
    pub hits: Vec<DocumentHit>,
    /// Entities (and matters) the question names, by id, the most
    /// specific (in fewest documents) first.
    pub named: Vec<String>,
    /// Their names, by id.
    pub names: BTreeMap<String, String>,
}

/// Every way a number or identifier of `text` can be written, as its
/// digits alone: the runs of digits joined by dots, dashes, slashes, a
/// thousands comma or up to two letters ("12.345.678" -> 12345678,
/// "A00999-00012345" -> 0099900012345). Plain runs of digits are already
/// words of the text index and are left out; an amount with cents is also
/// kept without them ("37,500.00" -> 37500).
pub fn number_forms(text: &str) -> BTreeSet<String> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let alnum = |i: usize| chars[i].is_ascii_alphanumeric();
    let letters = |mut i: usize| {
        let start = i;
        while i < n && i - start < 2 && chars[i].is_ascii_alphabetic() {
            i += 1;
        }
        i
    };
    let digits = |mut i: usize| {
        while i < n && chars[i].is_ascii_digit() {
            i += 1;
        }
        i
    };
    let mut out = BTreeSet::new();
    let mut i = 0;
    while i < n {
        if i > 0 && alnum(i - 1) {
            i += 1;
            continue;
        }
        let after_letters = letters(i);
        let end_digits = digits(after_letters);
        if end_digits == after_letters {
            i += 1;
            continue;
        }
        let start = i;
        let mut end = end_digits;
        loop {
            let mut j = end;
            if j < n && matches!(chars[j], '.' | '-' | '/') {
                j += 1;
            } else if j < n && chars[j] == ',' {
                let k = digits(j + 1);
                if k - (j + 1) == 3 && (k >= n || !chars[k].is_ascii_digit()) {
                    j += 1;
                } else {
                    break;
                }
            }
            let after = letters(j);
            let next = digits(after);
            if next == after {
                break;
            }
            end = next;
        }
        let run: String = chars[start..end].iter().collect();
        let joined = run.chars().any(|c| !c.is_ascii_digit());
        if joined {
            let only: String = run.chars().filter(char::is_ascii_digit).collect();
            if only.len() >= 4 {
                out.insert(only.clone());
            }
            let tail: Vec<char> = run.chars().rev().take(3).collect();
            let cents = tail.len() == 3 && tail[0].is_ascii_digit() && tail[1].is_ascii_digit() && matches!(tail[2], '.' | ',');
            if cents && only.len() >= 6 {
                out.insert(only[..only.len() - 2].to_string());
            }
        }
        i = end.max(i + 1);
    }
    out
}

/// What the search index keeps of one document besides its text: its
/// numbers in every form and its summary. Called whenever the text or the
/// summary of a document changes.
pub(crate) fn index_content(conn: &Connection, content_id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM content_search WHERE rowid = ?1", [content_id]).map_err(sql_err)?;
    let body: Option<String> = conn
        .query_row("SELECT body FROM content_fts WHERE rowid = ?1", [content_id], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    let summary: Option<(Option<String>, String)> = conn
        .query_row("SELECT topic, points FROM knowledge_summary WHERE content_id = ?1", [content_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(sql_err)?;
    if body.is_none() && summary.is_none() {
        return Ok(());
    }
    let numbers = body.as_deref().map(number_forms).unwrap_or_default().into_iter().collect::<Vec<_>>().join(" ");
    let summary = summary.map(|(topic, points)| summary_text(topic.as_deref(), &points)).unwrap_or_default();
    conn.execute(
        "INSERT INTO content_search (rowid, numbers, summary) VALUES (?1, ?2, ?3)",
        params![content_id, numbers, summary],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Forgets what the search index keeps of documents that are gone.
pub(crate) fn forget_gone(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM content_search WHERE rowid NOT IN (SELECT id FROM content)", [])
        .map_err(sql_err)?;
    Ok(())
}

/// Indexes the documents the search index does not have yet (a base
/// brought up from before the index, or one written by an older app).
pub(crate) fn fill_missing(conn: &Connection) -> Result<usize, String> {
    let missing: Vec<i64> = {
        let mut statement = conn
            .prepare(
                "SELECT rowid FROM content_fts WHERE rowid NOT IN (SELECT rowid FROM content_search)
                 UNION SELECT content_id FROM knowledge_summary WHERE content_id NOT IN (SELECT rowid FROM content_search)",
            )
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get(0)).map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?
    };
    for content in &missing {
        index_content(conn, *content)?;
    }
    Ok(missing.len())
}

fn summary_text(topic: Option<&str>, points: &str) -> String {
    let mut parts: Vec<String> = topic.map(|t| vec![t.to_string()]).unwrap_or_default();
    if let Ok(serde_json::Value::Array(points)) = serde_json::from_str::<serde_json::Value>(points) {
        for point in points {
            if let Some(text) = point.get("text").and_then(|t| t.as_str()) {
                parts.push(text.to_string());
            }
        }
    }
    parts.join("\n")
}

/// The terms of the question for the text index: each word, long words
/// by their stem, and every form of its numbers.
fn query_terms(query: &str) -> (Vec<String>, Vec<String>) {
    let mut words = Vec::new();
    for token in crate::commands::search::tokenize_query(query) {
        let chars: Vec<char> = token.chars().collect();
        if chars.len() >= STEM_FROM_CHARS && !chars.iter().all(char::is_ascii_digit) {
            let stem: String = chars[..chars.len() - 2].iter().collect();
            words.push(format!("\"{}\"*", stem.replace('"', "\"\"")));
        } else {
            words.push(format!("\"{}\"", token.replace('"', "\"\"")));
        }
    }
    // The numbers of the question in every form, plain runs too: "12345678"
    // finds the "12.345.678" of a text.
    let mut forms = number_forms(query);
    forms.extend(super::knowledge::pages::digit_runs(query).into_iter().filter(|d| d.len() >= 4));
    let numbers: Vec<String> = forms.into_iter().map(|d| format!("\"{d}\"")).collect();
    words.extend(numbers.iter().cloned());
    (words, numbers)
}

/// The condition that keeps a document among the visible ones (and, when
/// a person chose a set, among those), on a content id column.
fn visible_condition(visible: &Visible, within: Option<&[i64]>, column: &str) -> (String, Vec<Value>) {
    let (condition, bound) = visible.condition("l.origin_id");
    let mut sql = format!(
        "{column} IN (SELECT f.content_id FROM file f JOIN location l ON l.file_hash = f.hash WHERE {condition})"
    );
    let mut values: Vec<Value> = bound.into_iter().map(Value::Text).collect();
    if let Some(within) = within {
        let marks = vec!["?"; within.len()].join(", ");
        if within.is_empty() {
            sql.push_str(" AND 1 = 0");
        } else {
            sql.push_str(&format!(" AND {column} IN ({marks})"));
            values.extend(within.iter().map(|c| Value::Integer(*c)));
        }
    }
    (sql, values)
}

fn ranked(conn: &Connection, table: &str, matching: &str, keep: &(String, Vec<Value>)) -> Result<Vec<i64>, String> {
    if matching.is_empty() {
        return Ok(Vec::new());
    }
    let sql = format!(
        "SELECT rowid FROM {table} WHERE {table} MATCH ? AND {} ORDER BY bm25({table}) LIMIT {LIST_LIMIT}",
        keep.0
    );
    let mut values = vec![Value::Text(matching.to_string())];
    values.extend(keep.1.iter().cloned());
    let mut statement = conn.prepare(&sql).map_err(sql_err)?;
    let rows = statement.query_map(params_from_iter(values), |row| row.get(0));
    match rows {
        Ok(rows) => rows.collect::<Result<Vec<_>, _>>().map_err(sql_err),
        // A question the index cannot parse finds nothing in this list.
        Err(_) => Ok(Vec::new()),
    }
}

/// The entities (and matters) the question names, each with its visible
/// documents.
fn named_entities(
    conn: &Connection,
    query: &str,
    keep: &(String, Vec<Value>),
) -> Result<BTreeMap<String, BTreeSet<i64>>, String> {
    let words = quote::words(query);
    let total: i64 = conn.query_row("SELECT count(*) FROM content", [], |row| row.get(0)).map_err(sql_err)?;
    let max_documents = ((total as f64) * ENTITY_MAX_SHARE).max(3.0) as usize;
    let merged: HashMap<String, String> = {
        let mut statement = conn
            .prepare("SELECT id, merged_into FROM knowledge_entity WHERE state = 'merged' AND merged_into IS NOT NULL")
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let root = |id: &str| {
        let mut id = id.to_string();
        for _ in 0..10 {
            match merged.get(&id) {
                Some(next) => id = next.clone(),
                None => break,
            }
        }
        id
    };
    let matters: Vec<(String, String)> = {
        let mut statement =
            conn.prepare("SELECT id, name FROM knowledge_matter WHERE state = 'active'").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
            .map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?
            .into_iter()
            .map(|(id, name)| (id, quote::words(&name).join(" ")))
            .collect()
    };
    let mut found: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
    let mut document_count: HashMap<String, i64> = HashMap::new();
    for i in 0..words.len() {
        for j in (i + 1)..=(words.len().min(i + MAX_NAME_WORDS)) {
            let name = words[i..j].join(" ");
            if name.chars().count() < 4 || (j - i == 1 && crate::commands::search::tokenize_query(&name).is_empty()) {
                continue;
            }
            let mut ids: Vec<String> = {
                let mut statement =
                    conn.prepare_cached("SELECT entity_id FROM knowledge_alias WHERE normalized = ?1").map_err(sql_err)?;
                let rows = statement.query_map([&name], |row| row.get::<_, String>(0)).map_err(sql_err)?;
                rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?.iter().map(|id| root(id)).collect()
            };
            ids.extend(matters.iter().filter(|(_, n)| *n == name).map(|(id, _)| id.clone()));
            // A reference of the documents (D-377): a record, a file, an act.
            let is_reference: bool = conn
                .prepare_cached("SELECT EXISTS (SELECT 1 FROM knowledge_mention WHERE kind = ?1 AND defined = 0 AND normalized = ?2)")
                .and_then(|mut s| s.query_row(params![vocabulary().matters.kind, name], |row| row.get(0)))
                .map_err(sql_err)?;
            if is_reference {
                ids.push(format!("{REFERENCE_PREFIX}{name}"));
            }
            if ids.is_empty() {
                continue;
            }
            // How many documents the name is written in.
            let written_in: i64 = conn
                .query_row(
                    "SELECT count(*) FROM content_fts WHERE content_fts MATCH ?1",
                    [format!("\"{}\"", name.replace('"', "\"\""))],
                    |row| row.get(0),
                )
                .unwrap_or(i64::MAX);
            for id in ids {
                let documents = documents_of(conn, &id, &merged, keep)?;
                let all = *document_count.entry(id.clone()).or_insert_with(|| documents.len() as i64);
                if written_in > 2 * all + 2 || documents.is_empty() || documents.len() > max_documents {
                    continue;
                }
                found.insert(id, documents);
            }
        }
    }
    Ok(found)
}

fn documents_of(
    conn: &Connection,
    id: &str,
    merged: &HashMap<String, String>,
    keep: &(String, Vec<Value>),
) -> Result<BTreeSet<i64>, String> {
    if let Some(normalized) = id.strip_prefix(REFERENCE_PREFIX) {
        let sql = format!(
            "SELECT DISTINCT content_id FROM knowledge_mention WHERE kind = ? AND defined = 0 AND normalized = ? AND {keep}",
            keep = keep.0
        );
        let mut values: Vec<Value> = vec![Value::Text(vocabulary().matters.kind.clone()), Value::Text(normalized.to_string())];
        values.extend(keep.1.iter().cloned());
        let mut statement = conn.prepare(&sql).map_err(sql_err)?;
        let rows = statement.query_map(params_from_iter(values), |row| row.get(0)).map_err(sql_err)?;
        return rows.collect::<Result<_, _>>().map_err(sql_err);
    }
    let mut ids: Vec<String> = vec![id.to_string()];
    // The entities merged into this one, at any depth.
    let mut grew = true;
    while grew {
        grew = false;
        for (from, into) in merged {
            if ids.contains(into) && !ids.contains(from) {
                ids.push(from.clone());
                grew = true;
            }
        }
    }
    let marks = vec!["?"; ids.len()].join(", ");
    let sql = format!(
        "SELECT content_id FROM knowledge_mention WHERE entity_id IN ({marks}) AND {keep}
         UNION SELECT content_id FROM knowledge_matter_document WHERE matter_id = ? AND {keep}",
        keep = keep.0
    );
    let mut values: Vec<Value> = ids.iter().cloned().map(Value::Text).collect();
    values.extend(keep.1.iter().cloned());
    values.push(Value::Text(id.to_string()));
    values.extend(keep.1.iter().cloned());
    let mut statement = conn.prepare(&sql).map_err(sql_err)?;
    let rows = statement.query_map(params_from_iter(values), |row| row.get(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

impl DocBase {
    /// Searches the visible documents (D-350). `within`, when given, is a
    /// set a person chose on purpose (a matter, a selection): only then is
    /// the search narrowed. The graph never takes a document out.
    pub fn search_documents(
        &self,
        visible: &Visible,
        query: &str,
        limit: usize,
        within: Option<&[i64]>,
    ) -> Result<DocumentSearch, String> {
        let conn = self.conn();
        let (words, numbers) = query_terms(query);
        let keep_rowid = visible_condition(visible, within, "rowid");
        let keep_content = visible_condition(visible, within, "content_id");
        let text = ranked(&conn, "content_fts", &words.join(" OR "), &keep_rowid)?;
        let by_number = if numbers.is_empty() {
            Vec::new()
        } else {
            ranked(&conn, "content_search", &format!("numbers : ({})", numbers.join(" OR ")), &keep_rowid)?
        };
        let mut by_summary = if words.is_empty() {
            Vec::new()
        } else {
            ranked(&conn, "content_search", &format!("summary : ({})", words.join(" OR ")), &keep_rowid)?
        };
        // A question with a number asks for the documents that have it: the
        // summary only orders those. Its other words ("número", "cliente")
        // are in the summaries of many documents that do not have it, and
        // would push out the ones the number finds.
        if !numbers.is_empty() {
            let with_number: HashSet<i64> = by_number.iter().copied().collect();
            by_summary.retain(|content| with_number.contains(content));
        }
        let mut score: HashMap<i64, f64> = HashMap::new();
        let mut found_by: HashMap<i64, Vec<String>> = HashMap::new();
        for (list, why) in [(&text, "text"), (&by_number, "number"), (&by_summary, "summary")] {
            for (rank, content) in list.iter().enumerate() {
                *score.entry(*content).or_default() += 1.0 / (RANK_K + rank as f64 + 1.0);
                found_by.entry(*content).or_default().push(why.to_string());
            }
        }
        // The graph orders and widens: what the question names.
        let named = named_entities(&conn, query, &keep_content)?;
        let mut entities_of: HashMap<i64, Vec<String>> = HashMap::new();
        if !named.is_empty() {
            let mut documents: BTreeSet<i64> = BTreeSet::new();
            for set in named.values() {
                documents.extend(set.iter().copied());
            }
            for content in documents {
                let mut specific = 0.0;
                for (id, set) in &named {
                    if set.contains(&content) {
                        specific += (SPECIFIC_DOCUMENTS / set.len() as f64).min(1.0);
                        entities_of.entry(content).or_default().push(id.clone());
                    }
                }
                *score.entry(content).or_default() += specific / named.len() as f64 / (RANK_K + 1.0);
                found_by.entry(content).or_default().push("graph".to_string());
            }
        }
        // ...and the first documents bring the ones related to them.
        let mut first: Vec<(i64, f64)> = score.iter().map(|(c, s)| (*c, *s)).collect();
        first.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        first.truncate(WIDEN_FROM);
        let visible_set: HashSet<i64> = {
            let keep_id = visible_condition(visible, within, "id");
            let mut statement = conn.prepare(&format!("SELECT id FROM content WHERE {}", keep_id.0)).map_err(sql_err)?;
            let rows = statement.query_map(params_from_iter(keep_id.1), |row| row.get(0)).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (content, _) in first {
            let current = score.get(&content).copied().unwrap_or_default();
            let mut statement = conn
                .prepare_cached(
                    "SELECT CASE WHEN a = ?1 THEN b ELSE a END FROM similar_pair
                     WHERE (a = ?1 OR b = ?1) AND state != 'discarded'",
                )
                .map_err(sql_err)?;
            let related: Vec<i64> = statement
                .query_map([content], |row| row.get(0))
                .map_err(sql_err)?
                .collect::<Result<_, _>>()
                .map_err(sql_err)?;
            for other in related {
                if !visible_set.contains(&other) {
                    continue;
                }
                *score.entry(other).or_default() += WIDEN_WEIGHT * current;
                found_by.entry(other).or_default().push("related".to_string());
            }
        }
        let mut order: Vec<(i64, f64)> = score.into_iter().collect();
        order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal).then(a.0.cmp(&b.0)));
        order.truncate(limit);
        let mut hits = Vec::new();
        for (content, score) in order {
            let snippet = snippet_of(&conn, content, &words)?;
            let mut why = found_by.remove(&content).unwrap_or_default();
            why.dedup();
            hits.push(DocumentHit {
                content_id: content,
                score,
                snippet,
                found_by: why,
                entities: entities_of.remove(&content).unwrap_or_default(),
            });
        }
        let mut names = BTreeMap::new();
        for id in named.keys() {
            if let Some(normalized) = id.strip_prefix(REFERENCE_PREFIX) {
                let written = super::knowledge::references::written_form(&conn, vocabulary(), normalized)?;
                names.insert(id.clone(), written.unwrap_or_else(|| normalized.to_string()));
                continue;
            }
            let name: Option<String> = conn
                .query_row(
                    "SELECT name FROM knowledge_entity WHERE id = ?1 UNION ALL SELECT name FROM knowledge_matter WHERE id = ?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(sql_err)?;
            names.insert(id.clone(), name.unwrap_or_else(|| id.clone()));
        }
        // The most specific first: the one in fewest documents.
        let mut named: Vec<(String, usize)> = named.into_iter().map(|(id, set)| (id, set.len())).collect();
        named.sort_by(|a, b| a.1.cmp(&b.1).then(a.0.cmp(&b.0)));
        Ok(DocumentSearch { hits, named: named.into_iter().map(|(id, _)| id).collect(), names })
    }

    /// The page of each document in the wiki of the archive: the page of
    /// its family of versions (D-340), as the projection last wrote it.
    pub fn document_pages(&self) -> Result<HashMap<i64, String>, String> {
        let conn = self.conn();
        let words = &vocabulary().pages;
        let written: HashMap<String, String> = {
            let mut statement = conn
                .prepare("SELECT entity_id, path FROM knowledge_page WHERE entity_id LIKE 'doc-%'")
                .map_err(sql_err)?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let mut parent: HashMap<i64, i64> = HashMap::new();
        fn find(parent: &mut HashMap<i64, i64>, x: i64) -> i64 {
            let mut root = x;
            while let Some(&p) = parent.get(&root) {
                if p == root {
                    break;
                }
                root = p;
            }
            root
        }
        {
            let mut statement = conn
                .prepare(
                    "SELECT a, b, coalesce(chosen_relation, relation), state, wiki_source
                     FROM similar_pair WHERE state != 'discarded'",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<i64>>(4)?,
                    ))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (a, b, relation, state, source) = row.map_err(sql_err)?;
                if !(words.version_relations.contains(&relation) || (state == "joined" && source.is_some())) {
                    continue;
                }
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
                if ra != rb {
                    parent.insert(ra.max(rb), ra.min(rb));
                }
            }
        }
        let contents: Vec<i64> = {
            let mut statement = conn.prepare("SELECT id FROM content").map_err(sql_err)?;
            let rows = statement.query_map([], |row| row.get(0)).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let mut members: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
        for content in contents {
            let root = find(&mut parent, content);
            members.entry(root).or_default().push(content);
        }
        let mut out = HashMap::new();
        for family in members.values() {
            // The page is named by its smallest member; when that one has
            // no page (it is not a source of the wiki), by another one.
            let path = family.iter().find_map(|m| written.get(&format!("doc-{m}")));
            if let Some(path) = path {
                for m in family {
                    out.insert(*m, path.clone());
                }
            }
        }
        Ok(out)
    }

    /// The page of the entity or matter known by this name (normalized as
    /// the record keeps names), when exactly one has a page.
    pub fn page_named(&self, normalized: &str) -> Result<Option<String>, String> {
        if normalized.is_empty() {
            return Ok(None);
        }
        let conn = self.conn();
        let mut statement = conn
            .prepare(
                "SELECT DISTINCT p.path FROM knowledge_alias a
                 JOIN knowledge_entity e ON e.id = a.entity_id
                 JOIN knowledge_page p ON p.entity_id = coalesce(e.merged_into, e.id)
                 WHERE a.normalized = ?1",
            )
            .map_err(sql_err)?;
        let paths: Vec<String> = statement
            .query_map([normalized], |row| row.get(0))
            .map_err(sql_err)?
            .collect::<Result<_, _>>()
            .map_err(sql_err)?;
        Ok(if paths.len() == 1 { paths.into_iter().next() } else { None })
    }

    /// The pages of entities and matters, by id.
    pub fn entity_pages(&self, ids: &[String]) -> Result<BTreeMap<String, String>, String> {
        let conn = self.conn();
        let mut out = BTreeMap::new();
        for id in ids {
            let path: Option<String> = conn
                .query_row("SELECT path FROM knowledge_page WHERE entity_id = ?1", [id], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            if let Some(path) = path {
                out.insert(id.clone(), path);
            }
        }
        Ok(out)
    }
}

/// The passage of the document that matched, or the topic of its summary.
fn snippet_of(conn: &Connection, content: i64, words: &[String]) -> Result<String, String> {
    if !words.is_empty() {
        let found: Option<String> = conn
            .query_row(
                "SELECT snippet(content_fts, 0, '', '', '…', 24) FROM content_fts WHERE content_fts MATCH ?1 AND rowid = ?2",
                params![words.join(" OR "), content],
                |row| row.get(0),
            )
            .optional()
            .unwrap_or(None);
        if let Some(found) = found.filter(|s| !s.trim().is_empty()) {
            return Ok(found.split_whitespace().collect::<Vec<_>>().join(" "));
        }
    }
    let topic: Option<Option<String>> = conn
        .query_row("SELECT topic FROM knowledge_summary WHERE content_id = ?1", [content], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(topic.flatten().unwrap_or_default())
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
