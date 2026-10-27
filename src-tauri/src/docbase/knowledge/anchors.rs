//! What a person decided, on what the archive says now (P-142, D-321).
//!
//! A decision refers to what does not change when a document is read again
//! (`decide.rs`): a group of mentions of one document by its name, a piece
//! of evidence by its document, subject, predicate and object. A reading
//! with another question (or another model) may write the same name or
//! value another way ("Persona A. Dos" for "Persona Dos", "Calle Seis N°
//! 600" for "Calle Seis 600"); a new version of the file may drop it.
//!
//! Before every resolution each active decision is placed on what the
//! documents say now:
//!
//! - **applies**: what it names is there, as it was (or in the document
//!   that took the place of its document, `changes.rs`);
//! - **moved**: it is not there as it was, but one thing of its document,
//!   and only one, is the same written another way (the same kind and a
//!   name that may be the same; the same predicate and a value that is the
//!   same): the decision goes on to it;
//! - **waiting**: its document is gone or not read yet: nothing to say
//!   until it is (a document that comes back gets its decision back);
//! - **to review**: its document is read and nothing in it, or more than
//!   one thing, is what it named. It is never applied to a guess and never
//!   dropped in silence: the attention view lists it for a person.
//!
//! Decisions about claims (`claim:<entity>:…`) are placed by the
//! consolidation of the claims (`claims.rs`), which knows them.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection};

use super::names::{self, Likeness};
use super::resolve::{cluster_key, parse_cluster_key};
use super::values;
use super::vocabulary::Vocabulary;
use super::{changes, quote};
use crate::docbase::sql_err;

/// Why a decision is left for a person to review.
pub(crate) const NOT_FOUND: &str = "no-encontrada";
pub(crate) const SEVERAL: &str = "varias";

/// Where a key of a decision stands now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Placed {
    /// The key itself (perhaps on the document that took the place of its own).
    Applies(String),
    /// Another key of the same document: the same thing written another way.
    Moved(String),
    /// Its document is gone or not read yet.
    Waiting(String),
    /// Read, and not found (or found more than once): for a person.
    Review(String, &'static str),
}

impl Placed {
    /// The key the decision is applied with.
    pub(crate) fn key(&self) -> &str {
        match self {
            Placed::Applies(k) | Placed::Moved(k) | Placed::Waiting(k) | Placed::Review(k, _) => k,
        }
    }

    pub(crate) fn review(&self) -> Option<&'static str> {
        match self {
            Placed::Review(_, why) => Some(why),
            _ => None,
        }
    }
}

/// An active decision, placed.
#[derive(Debug, Clone)]
pub(crate) struct Anchored {
    pub id: i64,
    pub kind: String,
    pub a: Placed,
    pub b: Option<Placed>,
}

impl Anchored {
    /// Why it is to be reviewed, if it is.
    pub(crate) fn review(&self) -> Option<&'static str> {
        self.a.review().or_else(|| self.b.as_ref().and_then(Placed::review))
    }

    /// The keys it is applied with now, when one of them moved.
    pub(crate) fn moved(&self) -> Option<serde_json::Value> {
        let moved = matches!(self.a, Placed::Moved(_)) || matches!(self.b, Some(Placed::Moved(_)));
        moved.then(|| serde_json::json!({ "a": self.a.key(), "b": self.b.as_ref().map(Placed::key) }))
    }
}

/// What the documents say now, as the placing needs it.
struct Now {
    /// Contents that exist.
    contents: BTreeSet<i64>,
    /// Contents whose reading is done and current enough to judge.
    read: BTreeSet<i64>,
    /// (content, kind) -> normalized names of its mentions.
    names: HashMap<(i64, String), BTreeSet<String>>,
    /// content -> (subject, predicate, object) of its evidence, as the keys
    /// write them (`m:<name>` for an object mention).
    evidence: HashMap<i64, Vec<(String, String, String)>>,
    /// The kind of the mentions of a content by their name, to read a
    /// subject's name with the rules of its kind.
    kind_of: HashMap<(i64, String), String>,
}

fn load(conn: &Connection) -> Result<Now, String> {
    let mut now = Now {
        contents: BTreeSet::new(),
        read: BTreeSet::new(),
        names: HashMap::new(),
        evidence: HashMap::new(),
        kind_of: HashMap::new(),
    };
    {
        let mut statement = conn.prepare("SELECT id FROM version").map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
        for row in rows {
            now.contents.insert(row.map_err(sql_err)?);
        }
    }
    {
        let mut statement = conn.prepare("SELECT version_id FROM knowledge_extraction WHERE state = 'done'").map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
        for row in rows {
            now.read.insert(row.map_err(sql_err)?);
        }
    }
    {
        let mut statement = conn.prepare("SELECT version_id, kind, normalized FROM knowledge_mention").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, kind, normalized) = row.map_err(sql_err)?;
            now.kind_of.entry((content, normalized.clone())).or_insert_with(|| kind.clone());
            now.names.entry((content, kind)).or_default().insert(normalized);
        }
    }
    {
        let mut statement = conn
            .prepare(
                "SELECT e.version_id, s.normalized, e.predicate, o.normalized, e.object_normalized FROM knowledge_evidence e
                 JOIN knowledge_mention s ON s.id = e.subject_mention LEFT JOIN knowledge_mention o ON o.id = e.object_mention",
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
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (content, subject, predicate, object, value) = row.map_err(sql_err)?;
            let object = match object {
                Some(name) => format!("m:{name}"),
                None => value.unwrap_or_default(),
            };
            now.evidence.entry(content).or_default().push((subject, predicate, object));
        }
    }
    Ok(now)
}

/// Whether two names of one document, of the same kind, may be the same
/// entity written another way.
fn may_be(vocabulary: &Vocabulary, kind: &str, a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (x, y) = (names::key(vocabulary, kind, a), names::key(vocabulary, kind, b));
    matches!(names::compare(kind, &x, &y), Likeness::Exact | Likeness::Maybe(_))
}

/// The only element of `found`, or why there is none.
fn only(found: BTreeSet<String>, waiting: String) -> Placed {
    match found.len() {
        1 => Placed::Moved(found.into_iter().next().expect("one")),
        0 => Placed::Review(waiting, NOT_FOUND),
        _ => Placed::Review(waiting, SEVERAL),
    }
}

fn place_cluster(vocabulary: &Vocabulary, now: &Now, key: String) -> Placed {
    let Some((content, kind, name)) = parse_cluster_key(&key) else { return Placed::Applies(key) };
    if !now.contents.contains(&content) || !now.read.contains(&content) {
        return Placed::Waiting(key);
    }
    let names = now.names.get(&(content, kind.clone()));
    if names.is_some_and(|n| n.contains(&name)) {
        return Placed::Applies(key);
    }
    let found: BTreeSet<String> = names
        .into_iter()
        .flatten()
        .filter(|candidate| may_be(vocabulary, &kind, candidate, &name))
        .map(|candidate| cluster_key(content, &kind, candidate))
        .collect();
    only(found, key)
}

/// `evidence:<content>:<subject>:<predicate>:<object>`.
fn parse_evidence_key(key: &str) -> Option<(i64, String, String, String)> {
    let mut parts = key.strip_prefix("evidence:")?.splitn(4, ':');
    let content = parts.next()?.parse().ok()?;
    Some((content, parts.next()?.to_string(), parts.next()?.to_string(), parts.next()?.to_string()))
}

fn place_evidence(vocabulary: &Vocabulary, now: &Now, key: String) -> Placed {
    let Some((content, subject, predicate, object)) = parse_evidence_key(&key) else { return Placed::Applies(key) };
    if !now.contents.contains(&content) || !now.read.contains(&content) {
        return Placed::Waiting(key);
    }
    let pieces = now.evidence.get(&content).map(Vec::as_slice).unwrap_or_default();
    if pieces.iter().any(|(s, p, o)| *s == subject && *p == predicate && *o == object) {
        return Placed::Applies(key);
    }
    let shape = vocabulary.predicate(&predicate).map(|p| p.shape.as_str()).unwrap_or_default();
    let same_subject = |s: &str| {
        let kind = now.kind_of.get(&(content, s.to_string())).map(String::as_str).unwrap_or(names::PERSON);
        may_be(vocabulary, kind, s, &subject)
    };
    let same_object = |o: &str| match (o.strip_prefix("m:"), object.strip_prefix("m:")) {
        (Some(x), Some(y)) => {
            let kind = now.kind_of.get(&(content, x.to_string())).map(String::as_str).unwrap_or(names::PERSON);
            may_be(vocabulary, kind, x, y)
        }
        (None, None) => o == object || values::same(vocabulary, shape, o, &object) || same_digits(o, &object),
        _ => false,
    };
    let found: BTreeSet<String> = pieces
        .iter()
        .filter(|(s, p, o)| *p == predicate && same_subject(s) && same_object(o))
        .map(|(s, p, o)| super::claims::evidence_key(content, s, p, o))
        .collect();
    only(found, key)
}

/// Two values with the same figures and words but for the small ones
/// ("calle seis n 600", "calle seis 600").
pub(crate) fn same_digits(a: &str, b: &str) -> bool {
    let digits = |t: &str| values::digit_runs(t);
    let long = |t: &str| quote::words(t).into_iter().filter(|w| w.chars().count() >= 3).collect::<Vec<_>>();
    let (da, db) = (digits(a), digits(b));
    !da.is_empty() && da == db && long(a) == long(b)
}

/// Every active decision, placed on what the documents say now. Keys of
/// claims and of documents (`content:`) are only carried to their
/// successors here.
pub(crate) fn place_all(conn: &Connection, vocabulary: &Vocabulary) -> Result<Vec<Anchored>, String> {
    let rows: Vec<(i64, String, String, Option<String>)> = {
        let mut statement =
            conn.prepare("SELECT id, kind, a, b FROM knowledge_decision WHERE state = 'active' ORDER BY decided_at, id").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    if rows.is_empty() {
        return Ok(Vec::new());
    }
    let successors = changes::successors(conn)?;
    let now = load(conn)?;
    let place = |key: String| -> Placed {
        let key = changes::translate(&key, &successors);
        if key.starts_with("cluster:") {
            place_cluster(vocabulary, &now, key)
        } else if key.starts_with("evidence:") {
            place_evidence(vocabulary, &now, key)
        } else {
            Placed::Applies(key)
        }
    };
    Ok(rows
        .into_iter()
        .map(|(id, kind, a, b)| Anchored { id, kind, a: place(a), b: b.map(place) })
        .collect())
}

/// Keeps, with each active decision, whether it is to be reviewed and the
/// keys it applies with when they moved. `claims` adds what the
/// consolidation of the claims found of the keys of claims (`None`: not
/// found). Only what changed is written.
pub(crate) fn keep(conn: &Connection, anchored: &[Anchored], claims: &BTreeMap<String, Option<String>>) -> Result<(), String> {
    for decision in anchored {
        let (mut review, mut moved) = (decision.review().map(str::to_string), decision.moved());
        if let Some(found) = claims.get(decision.a.key()) {
            match found {
                None => review = Some(NOT_FOUND.to_string()),
                Some(other) if other != decision.a.key() => moved = Some(serde_json::json!({ "a": other, "b": null })),
                Some(_) => {}
            }
        }
        let moved = moved.map(|m| m.to_string());
        conn.execute(
            "UPDATE knowledge_decision SET review = ?2, applied_as = ?3 WHERE id = ?1 AND (review IS NOT ?2 OR applied_as IS NOT ?3)",
            params![decision.id, review, moved],
        )
        .map_err(sql_err)?;
    }
    Ok(())
}
