//! The claims of the knowledge layer: what the documents say of each
//! entity, once its mentions are resolved (D-191).
//!
//! - The evidence that says the same (the same subject, predicate and
//!   object, the object being another entity or a value, compared by its
//!   normalized words) is **one claim** with several pieces of evidence.
//! - A predicate that takes one value at a time (`single` in the
//!   vocabulary: a domicile, a marital status) with different values is a
//!   **conflict** when the times are the same or not said; when every
//!   value has its own time it is a **history**, each claim from its time.
//!   A conflict is kept, never resolved: both are shown.
//! - What a person marked as wrong (a piece of evidence, or a whole claim)
//!   does not count; a claim marked wrong is kept, retired.
//! - A value without the shape its predicate asks for (a number as a
//!   position) does not count; the same value written two ways (a domicile
//!   with or without its floor) is one claim, shown with the fullest
//!   writing (`values.rs`, D-209).
//! - A value said within another entity (a position held in a company,
//!   `within` in the vocabulary) is a claim within that entity: the same
//!   position in two companies is two claims, and values within different
//!   entities are never in conflict (D-210). The same goes for a value of
//!   another kind, told by the words of its quote (`qualifiers`: a
//!   domicile constituted for a proceeding is not the real one).
//!
//! Computed again from the evidence on every resolution; only what
//! changed is written.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rusqlite::{params, Connection};

use super::quote;
use super::values;
use super::vocabulary::Vocabulary;
use crate::docbase::{now_ms, sql_err};

/// What a consolidation left.
#[derive(Debug, Default)]
pub(crate) struct Consolidated {
    pub claims: usize,
    pub conflicts: usize,
    /// Entities whose claims changed.
    pub touched: BTreeSet<String>,
    /// Pieces of evidence whose value has not the shape of its predicate.
    pub unfit: usize,
}

/// The key a person marks a piece of evidence as wrong by: its document,
/// the name of its subject, the predicate and the object. It survives
/// reading the document again.
pub(crate) fn evidence_key(content_id: i64, subject: &str, predicate: &str, object: &str) -> String {
    format!("evidence:{content_id}:{subject}:{predicate}:{object}")
}

/// The key of a claim: its subject entity, predicate and object.
/// An amount between two parties has both: the other party and the
/// amount (D-244).
pub(crate) fn claim_key(subject: &str, predicate: &str, object_entity: Option<&str>, object_normalized: Option<&str>) -> String {
    match (object_entity, object_normalized.filter(|n| !n.is_empty())) {
        (Some(entity), Some(amount)) => format!("claim:{subject}:{predicate}:entity:{entity}:value:{amount}"),
        (Some(entity), None) => format!("claim:{subject}:{predicate}:entity:{entity}"),
        (None, normalized) => format!("claim:{subject}:{predicate}:value:{}", normalized.unwrap_or_default()),
    }
}

/// The key of a claim said within another entity (a position in a
/// company). A person marks such a claim wrong by its key without the
/// entity: wrong in every company.
fn draft_key(
    subject: &str,
    predicate: &str,
    object_entity: Option<&str>,
    object_normalized: Option<&str>,
    context: Option<&str>,
    qualifier: Option<&str>,
) -> String {
    let mut key = claim_key(subject, predicate, object_entity, object_normalized);
    if let Some(context) = context {
        key.push_str(&format!(":within:{context}"));
    }
    if let Some(qualifier) = qualifier {
        key.push_str(&format!(":as:{qualifier}"));
    }
    key
}

/// The kind of value the quote says, by the words of the vocabulary.
fn qualifier_of(qualifiers: &[super::vocabulary::Qualifier], quote_text: &str) -> Option<String> {
    let words = quote::words(quote_text);
    qualifiers
        .iter()
        .find(|q| q.words.iter().any(|w| quote::contains(&words, &quote::words(w))))
        .map(|q| q.id.clone())
}

/// The resolved mentions of each document, as the context of a value
/// needs them: `content -> [(mention, kind, entity, words of its name)]`.
type Named = HashMap<i64, Vec<(i64, String, String, Vec<String>)>>;

/// The entity of `types` a value is said within: the one whose name is in
/// the quote (the only one, or the only one the subject is related to in
/// the document); else the one the subject is related to in the
/// document, when there is only one. None when it cannot be told.
fn context_of(named: &Named, related: &HashMap<i64, BTreeSet<String>>, types: &[String], content: i64, subject_mention: i64, quote_text: &str) -> Option<String> {
    let quote_words = quote::words(quote_text);
    let in_quote: BTreeSet<&String> = named
        .get(&content)
        .into_iter()
        .flatten()
        .filter(|(id, kind, _, words)| *id != subject_mention && types.contains(kind) && quote::contains(&quote_words, words))
        .map(|(_, _, entity, _)| entity)
        .collect();
    let of_type: BTreeSet<&String> = named
        .get(&content)
        .into_iter()
        .flatten()
        .filter(|(_, kind, _, _)| types.contains(kind))
        .map(|(_, _, entity, _)| entity)
        .collect();
    let linked: BTreeSet<&String> =
        related.get(&subject_mention).into_iter().flatten().filter(|e| of_type.contains(e)).collect();
    match in_quote.len() {
        1 => in_quote.into_iter().next().cloned(),
        0 => (linked.len() == 1).then(|| linked.into_iter().next().cloned()).flatten(),
        _ => {
            let both: Vec<&String> = in_quote.into_iter().filter(|c| linked.contains(c)).collect();
            (both.len() == 1).then(|| both[0].clone())
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Draft {
    subject: String,
    predicate: String,
    context: Option<String>,
    qualifier: Option<String>,
    object_entity: Option<String>,
    object_value: Option<String>,
    object_normalized: Option<String>,
    /// The amount of an amount between two parties, and what else its
    /// reading says (kind, currency, concept), from its first evidence.
    amount: Option<String>,
    detail: Option<String>,
    evidence: BTreeSet<i64>,
    times: BTreeMap<String, String>,
    valid_from: Option<String>,
    state: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
struct Stored {
    id: i64,
    object_value: Option<String>,
    valid_from: Option<String>,
    state: String,
    evidence: BTreeSet<i64>,
}

struct Piece {
    id: i64,
    content: i64,
    subject_mention: i64,
    quote: String,
    subject_entity: Option<String>,
    subject_name: String,
    predicate: String,
    object_entity: Option<String>,
    object_name: Option<String>,
    value: Option<String>,
    normalized: Option<String>,
    time: Option<String>,
    amount: Option<String>,
    detail: Option<serde_json::Value>,
    /// The date of its document, as its text writes it.
    doc_date: Option<String>,
}

/// The values given as identifiers in any document (normalized): a
/// number of a document is not someone's telephone.
fn identifier_values(conn: &Connection) -> Result<HashSet<String>, String> {
    let mut statement = conn.prepare("SELECT DISTINCT normalized FROM knowledge_identifier").map_err(sql_err)?;
    let values = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(sql_err)?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(sql_err)?;
    Ok(values)
}

/// Whether the quote of a piece says what the piece claims, beyond being
/// in the text (R-02): a relation between two entities names one of them
/// in its quote; a predicate with words of its own (`cues`: a debt says
/// "adeuda", "saldo", "a pagar"…) has one of them, or the start of one, in
/// its quote, and none of the words that say it is about something else
/// (`not_cues`: "percepciones" next to an amount); a contact is not a
/// number the documents give as an identifier. What fails is kept in the reading but is not a claim.
fn says(vocabulary: &Vocabulary, piece: &Piece, identifiers: &HashSet<String>) -> bool {
    let quote = quote::words(&piece.quote);
    let Some(definition) = vocabulary.predicate(&piece.predicate) else { return true };
    let has = |list: &[String]| quote.iter().any(|w| list.iter().any(|c| w.starts_with(c.as_str())));
    // The words of a tax next to an amount between parties are checked
    // next to its amount, when it is read (D-244).
    let not_cues = if definition.is_money() { &[][..] } else { &definition.not_cues[..] };
    if (!definition.cues.is_empty() && !has(&definition.cues)) || has(not_cues) {
        return false;
    }
    // An amount between two parties names them by the reading of the whole
    // document ("Ud. adeuda la suma de…": the debtor is whom the letter is
    // addressed to); its direction was checked against its quote (D-244).
    if let Some(object) = piece.object_name.as_ref().filter(|_| !definition.is_money()) {
        let named = |name: &str| {
            let words: Vec<String> = name.split_whitespace().map(str::to_string).collect();
            quote::contains(&quote, &words)
                || words.iter().any(|w| w.chars().count() >= 4 && !vocabulary.is_generic(w) && quote.contains(w))
        };
        if !named(&piece.subject_name) && !named(object) {
            return false;
        }
    }
    if definition.shape == "contact" {
        let value = piece.normalized.clone().unwrap_or_else(|| quote::words(piece.value.as_deref().unwrap_or_default()).join(" "));
        let compact = super::names::identifier_value(&value);
        let is_identifier = identifiers.iter().any(|id| {
            let id = super::names::identifier_value(id);
            id.chars().filter(char::is_ascii_digit).count() >= 6 && compact.contains(&id)
        });
        if is_identifier {
            return false;
        }
    }
    true
}

fn load_pieces(conn: &Connection) -> Result<Vec<Piece>, String> {
    let mut statement = conn
        .prepare(
            "SELECT e.id, e.content_id, s.entity_id, s.normalized, e.predicate, o.entity_id, o.normalized,
                    e.object_value, e.object_normalized, e.fact_time, e.subject_mention, e.quote, e.amount, e.detail, k.doc_date
             FROM knowledge_evidence e
             JOIN knowledge_mention s ON s.id = e.subject_mention
             LEFT JOIN knowledge_mention o ON o.id = e.object_mention
             LEFT JOIN knowledge_extraction k ON k.content_id = e.content_id
             ORDER BY e.content_id, e.id",
        )
        .map_err(sql_err)?;
    let pieces = statement
        .query_map([], |row| {
            Ok(Piece {
                id: row.get(0)?,
                content: row.get(1)?,
                subject_entity: row.get(2)?,
                subject_name: row.get(3)?,
                predicate: row.get(4)?,
                object_entity: row.get(5)?,
                object_name: row.get(6)?,
                value: row.get(7)?,
                normalized: row.get(8)?,
                time: row.get(9)?,
                subject_mention: row.get(10)?,
                quote: row.get(11)?,
                amount: row.get(12)?,
                detail: row.get::<_, Option<String>>(13)?.and_then(|d| serde_json::from_str(&d).ok()),
                doc_date: row.get(14)?,
            })
        })
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    Ok(pieces)
}

/// What the context of a value is found with: the resolved mentions of
/// each document, and the entities each subject is related to in it.
fn load_context(conn: &Connection) -> Result<(Named, HashMap<i64, BTreeSet<String>>), String> {
    let mut named: Named = HashMap::new();
    {
        let mut statement = conn
            .prepare("SELECT id, content_id, kind, entity_id, written FROM knowledge_mention WHERE entity_id IS NOT NULL ORDER BY id")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (id, content, kind, entity, written) = row.map_err(sql_err)?;
            named.entry(content).or_default().push((id, kind, entity, quote::words(&written)));
        }
    }
    let mut related: HashMap<i64, BTreeSet<String>> = HashMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT e.subject_mention, o.entity_id FROM knowledge_evidence e JOIN knowledge_mention o ON o.id = e.object_mention
                 WHERE o.entity_id IS NOT NULL",
            )
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (subject, object) = row.map_err(sql_err)?;
            related.entry(subject).or_default().insert(object);
        }
    }
    Ok((named, related))
}

pub(crate) fn consolidate(conn: &Connection, vocabulary: &Vocabulary, wrong: &BTreeSet<String>) -> Result<Consolidated, String> {
    let pieces = load_pieces(conn)?;
    let (named, related) = load_context(conn)?;

    // The context of each piece, and its value once its shape is checked
    // and its ways of writing are put together, by (subject, predicate,
    // context).
    let mut unfit = 0;
    let identifiers = identifier_values(conn)?;
    let mut unsaid: HashSet<i64> = HashSet::new();
    let mut contexts: HashMap<i64, (Option<String>, Option<String>)> = HashMap::new();
    let mut by_value: BTreeMap<(String, String, Option<String>, Option<String>), Vec<usize>> = BTreeMap::new();
    for (index, piece) in pieces.iter().enumerate() {
        let Some(subject) = &piece.subject_entity else { continue };
        if !says(vocabulary, piece, &identifiers) {
            unfit += 1;
            unsaid.insert(piece.id);
            continue;
        }
        let definition = vocabulary.predicate(&piece.predicate);
        let within = definition.map(|p| p.within.as_slice()).unwrap_or_default();
        let context = (!within.is_empty())
            .then(|| context_of(&named, &related, within, piece.content, piece.subject_mention, &piece.quote))
            .flatten()
            .filter(|c| c != subject);
        // The kind of value: what the words of the quote say (a domicile
        // "constituido"), else the class the reading gave ("real",
        // "sede"); the kind of an amount (D-244, D-247).
        let said_class = |key: &str| piece.detail.as_ref().and_then(|d| d.get(key)).and_then(|c| c.as_str()).map(str::to_string);
        let qualifier = if definition.is_some_and(|p| p.is_money()) {
            said_class("tipo")
        } else {
            definition
                .and_then(|p| qualifier_of(&p.qualifiers, &piece.quote))
                .or_else(|| said_class("clase").filter(|c| definition.is_some_and(|p| p.qualifiers.iter().any(|q| &q.id == c))))
        };
        contexts.insert(piece.id, (context.clone(), qualifier.clone()));
        if piece.object_name.is_none() {
            let shape = definition.map(|p| p.shape.as_str()).unwrap_or_default();
            if !values::fits(vocabulary, shape, piece.value.as_deref().unwrap_or_default(), &piece.subject_name) {
                unfit += 1;
                continue;
            }
            by_value.entry((subject.clone(), piece.predicate.clone(), context, qualifier)).or_default().push(index);
        }
    }
    // (value shown, normalized) each piece with a value counts with.
    let mut written_as: HashMap<i64, (String, String)> = HashMap::new();
    for ((_, predicate, _, _), list) in &by_value {
        let shape = vocabulary.predicate(predicate).map(|p| p.shape.as_str()).unwrap_or_default();
        let value = |i: usize| pieces[list[i]].value.as_deref().unwrap_or_default();
        // Pieces written the same are one value; then the ways of writing
        // the same value, against the first of each group.
        let mut group: Vec<usize> = (0..list.len()).collect();
        for i in 0..list.len() {
            for j in 0..i {
                if group[j] != j {
                    continue;
                }
                let same_words = pieces[list[i]].normalized == pieces[list[j]].normalized;
                if same_words || values::same(vocabulary, shape, value(i), value(j)) {
                    group[i] = j;
                    break;
                }
            }
        }
        let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (i, g) in group.iter().enumerate() {
            members.entry(*g).or_default().push(i);
        }
        for list_of in members.values() {
            let shown = values::fullest(list_of.iter().map(|&i| value(i))).unwrap_or_default().trim().to_string();
            let normalized = quote::words(&shown).join(" ");
            for &i in list_of {
                written_as.insert(pieces[list[i]].id, (shown.clone(), normalized.clone()));
            }
        }
    }

    let mut drafts: BTreeMap<String, Draft> = BTreeMap::new();
    let mut evidence_claim: BTreeMap<i64, String> = BTreeMap::new();
    for piece in &pieces {
        let Some(subject) = &piece.subject_entity else { continue };
        if unsaid.contains(&piece.id) {
            continue;
        }
        let object_word = match (&piece.object_name, &piece.normalized) {
            (Some(name), _) => format!("m:{name}"),
            (None, Some(value)) => value.clone(),
            _ => String::new(),
        };
        if wrong.contains(&evidence_key(piece.content, &piece.subject_name, &piece.predicate, &object_word)) {
            continue;
        }
        let object_entity = match (&piece.object_name, &piece.object_entity) {
            (Some(_), Some(entity)) if entity != subject => Some(entity.clone()),
            (Some(_), _) => continue,
            (None, _) => None,
        };
        let (value, normalized) = match (&object_entity, written_as.get(&piece.id)) {
            (None, Some((value, normalized))) => (Some(value.clone()), Some(normalized.clone())),
            (None, None) => continue,
            // An amount between two parties keys on its amount too.
            (Some(_), _) => (None, piece.amount.as_deref().map(|a| values::digit_runs(a).join(" ")).filter(|a| !a.is_empty())),
        };
        if object_entity.is_none() && normalized.as_deref().map_or(true, str::is_empty) {
            continue;
        }
        let (context, qualifier) = contexts.get(&piece.id).cloned().unwrap_or_default();
        let plain = claim_key(subject, &piece.predicate, object_entity.as_deref(), normalized.as_deref());
        let key = draft_key(subject, &piece.predicate, object_entity.as_deref(), normalized.as_deref(), context.as_deref(), qualifier.as_deref());
        let draft = drafts.entry(key.clone()).or_insert_with(|| Draft {
            subject: subject.clone(),
            predicate: piece.predicate.clone(),
            context: context.clone(),
            qualifier: qualifier.clone(),
            object_value: value.clone(),
            object_entity: object_entity.clone(),
            object_normalized: normalized.clone(),
            amount: piece.amount.clone(),
            detail: piece.detail.as_ref().map(|d| d.to_string()),
            evidence: BTreeSet::new(),
            times: BTreeMap::new(),
            valid_from: None,
            state: "supported",
        });
        draft.evidence.insert(piece.id);
        if wrong.contains(&plain) {
            draft.state = "retired";
        }
        // Its time: the one the quote says, or the date of its document for
        // a value whose time is its document's (a domicile, D-247).
        let from_document = vocabulary.predicate(&piece.predicate).is_some_and(|p| p.time_from_document);
        let time = piece
            .time
            .as_deref()
            .filter(|t| !t.trim().is_empty())
            .or_else(|| from_document.then_some(piece.doc_date.as_deref()).flatten().filter(|t| !t.trim().is_empty()));
        if let Some(time) = time {
            draft.times.entry(quote::words(time).join(" ")).or_insert_with(|| time.trim().to_string());
        }
        evidence_claim.insert(piece.id, key);
    }
    for draft in drafts.values_mut() {
        if draft.times.len() == 1 {
            draft.valid_from = draft.times.values().next().cloned();
        }
    }
    // One value at a time: different values of the same kind, for the
    // same holder and the same time, are a conflict; each with its own
    // time, a history; without a time, values that are not compared
    // (D-252). Values within different entities are not compared.
    let mut by_subject: BTreeMap<(String, String, Option<String>, Option<String>), Vec<String>> = BTreeMap::new();
    for (key, draft) in &drafts {
        if draft.object_entity.is_none() && draft.state != "retired" && vocabulary.predicate(&draft.predicate).is_some_and(|p| p.single) {
            by_subject
                .entry((draft.subject.clone(), draft.predicate.clone(), draft.context.clone(), draft.qualifier.clone()))
                .or_default()
                .push(key.clone());
        }
    }
    for keys in by_subject.values() {
        if keys.len() < 2 {
            continue;
        }
        let mut times: HashMap<String, usize> = HashMap::new();
        for key in keys {
            if let Some(time) = &drafts[key].valid_from {
                *times.entry(quote::words(time).join(" ")).or_default() += 1;
            }
        }
        for key in keys {
            let draft = drafts.get_mut(key).expect("listed");
            let shared = draft.valid_from.as_ref().is_some_and(|t| times[&quote::words(t).join(" ")] > 1);
            if shared {
                draft.state = "conflict";
            }
        }
    }

    // What is stored, to write only what changed.
    let mut stored: BTreeMap<String, Stored> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT id, subject, predicate, object_entity, object_normalized, valid_from, state, context_entity, object_value, qualifier
                 FROM knowledge_claim",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, Option<String>>(9)?,
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (id, subject, predicate, entity, normalized, valid_from, state, context, object_value, qualifier) = row.map_err(sql_err)?;
            let key = draft_key(&subject, &predicate, entity.as_deref(), normalized.as_deref(), context.as_deref(), qualifier.as_deref());
            stored.insert(key, Stored { id, object_value, valid_from, state, evidence: BTreeSet::new() });
        }
        let mut statement = conn.prepare("SELECT id, claim_id FROM knowledge_evidence WHERE claim_id IS NOT NULL").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))).map_err(sql_err)?;
        let by_id: HashMap<i64, String> = stored.iter().map(|(k, s)| (s.id, k.clone())).collect();
        for row in rows {
            let (evidence, claim) = row.map_err(sql_err)?;
            if let Some(key) = by_id.get(&claim) {
                stored.get_mut(key).expect("listed").evidence.insert(evidence);
            }
        }
    }
    let now = now_ms();
    let mut out = Consolidated { unfit, ..Default::default() };
    let mut ids: HashMap<String, i64> = HashMap::new();
    // What goes first, so a claim that takes the place of another one
    // never meets it in the unique index.
    for (key, old) in &stored {
        if !drafts.contains_key(key) {
            let subject: Option<String> = conn
                .query_row("SELECT subject FROM knowledge_claim WHERE id = ?1", [old.id], |row| row.get(0))
                .map_err(sql_err)
                .ok();
            conn.execute("DELETE FROM knowledge_claim WHERE id = ?1", [old.id]).map_err(sql_err)?;
            out.touched.extend(subject);
        }
    }
    for (key, draft) in &drafts {
        match stored.get(key) {
            Some(old) => {
                ids.insert(key.clone(), old.id);
                let value_changed = draft.object_value.is_some() && old.object_value != draft.object_value;
                if old.valid_from != draft.valid_from || old.state != draft.state || old.evidence != draft.evidence || value_changed {
                    conn.execute(
                        "UPDATE knowledge_claim SET valid_from = ?2, state = ?3, object_value = coalesce(?4, object_value), computed_at = ?5,
                         amount = ?6, detail = ?7
                         WHERE id = ?1",
                        params![old.id, draft.valid_from, draft.state, draft.object_value, now, draft.amount, draft.detail],
                    )
                    .map_err(sql_err)?;
                    out.touched.insert(draft.subject.clone());
                    out.touched.extend(draft.object_entity.clone());
                }
            }
            None => {
                conn.execute(
                    "INSERT INTO knowledge_claim
                        (subject, predicate, object_entity, object_value, object_normalized, valid_from, state, computed_at, context_entity,
                         qualifier, amount, detail)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        draft.subject,
                        draft.predicate,
                        draft.object_entity,
                        draft.object_value,
                        draft.object_normalized,
                        draft.valid_from,
                        draft.state,
                        now,
                        draft.context,
                        draft.qualifier,
                        draft.amount,
                        draft.detail
                    ],
                )
                .map_err(sql_err)?;
                ids.insert(key.clone(), conn.last_insert_rowid());
                out.touched.insert(draft.subject.clone());
                out.touched.extend(draft.object_entity.clone());
            }
        }
    }
    for piece in &pieces {
        let claim = evidence_claim.get(&piece.id).and_then(|k| ids.get(k)).copied();
        conn.execute("UPDATE knowledge_evidence SET claim_id = ?2 WHERE id = ?1 AND claim_id IS NOT ?2", params![piece.id, claim])
            .map_err(sql_err)?;
    }
    out.claims = drafts.values().filter(|d| d.state != "retired").count();
    out.conflicts = drafts.values().filter(|d| d.state == "conflict").count();
    Ok(out)
}
