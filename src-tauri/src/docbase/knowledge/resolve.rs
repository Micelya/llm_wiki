//! Stage 2 of the knowledge layer: which mentions name the same entity.
//!
//! The mentions of one document that are the same entity (the same name
//! with several roles, or what the model linked with "misma_que") form a
//! **group of the document** (`Cluster`), the unit that is resolved. Its
//! key, `cluster:<content>:<kind>:<name>`, does not depend on the order in
//! which the archive was read: decisions and verdicts refer to it.
//!
//! The groups are joined into entities in cascade, from the surest to the
//! least sure (R-11), with three outcomes: the same, different, doubtful
//! (D-187):
//!
//! 0. What a person decided: a join is firm; a separation keeps two groups
//!    apart in every pass (R-08).
//! 1. **Strong identifiers** (DNI, CUIT, registration, file number): the
//!    same one joins by itself (D-174); another one of the same kind keeps
//!    two groups apart forever; one character of difference with
//!    compatible names is doubtful (a misread scan). An identifier the
//!    model gave to mentions of two kinds, or to two groups of one
//!    document, is not used: it was attributed to the wrong mention.
//! 2. **The same name**: the same words (any order for a person, two words
//!    at least; for an organization, the same legal form), unless an
//!    identifier keeps them apart. Matters are never joined by their name.
//! 3. **The model**, for the names that may be the same (a partial name,
//!    an initial, a typo, an acronym, an organization without its form):
//!    one question per principal entity and its candidates (R-21), with
//!    the names, identifiers, facts and quotes of each, and the option to
//!    abstain. Its answer is kept as a verdict on the pair (R-20) and is
//!    evidence for every later resolution: the same pair is not asked again.
//!
//! Families are built against a principal, not by chain (R-14): the model
//! is asked about a candidate against the principal's own groups, and a
//! candidate joined by the model never brings in its own candidates.
//! When in doubt, nothing is joined (R-07): what cannot be told apart is
//! kept as a **doubt** (`knowledge_doubt`) for a person.
//!
//! Candidates are found through indexes of names and identifiers (words,
//! first letters, initials, mail addresses), never all against all. The
//! whole partition is computed again from the mentions, the decisions and
//! the verdicts kept, in an order that depends only on the texts (R-13):
//! reading the archive in another order gives the same entities. What is
//! written is only what changed, and each entity that changed is marked
//! for its page to be projected again (R-15).

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{changes, claims};
use super::names::{self, Likeness, NameKey, ORGANIZATION, PERSON};
use super::vocabulary::{vocabulary, Vocabulary};
use crate::docbase::similar::classify::{self as classify, ask_and_keep, cached_answer, parse_json, stops_the_run, Question, ASKING};
use crate::docbase::{now_ms, sql_err, DocBase};
use crate::source_volume::asking::DocumentAsker;

/// Version of the question about identities.
pub const IDENTITY_PROMPT: &str = "identidad-4";

/// The rules the record was last computed with (`docbase_setting`): when
/// the program's or the vocabulary's change, the record is computed again.
const RESOLUTION_SETTING: &str = "knowledge-resolution";
/// Raised when a change in the program changes what a resolution gives.
const RESOLUTION_VERSION: &str = "resolucion-9";

fn resolution_basis() -> String {
    format!("{RESOLUTION_VERSION}|{IDENTITY_PROMPT}|{}", vocabulary().digest)
}
/// The task its answers are kept under in `model_answer`.
pub const TASK: &str = "identity";
/// Candidates described in one question; more take another question.
pub const MAX_CANDIDATES: usize = 8;
/// A word or key shared by more groups than this is too common to find
/// candidates by (R-16).
const BLOCK_CAP: usize = 200;
/// Appearances of an entity described in a question.
const APPEARANCES: usize = 5;
/// Shortest value of a strong identifier that is used to join.
const MIN_STRONG: usize = 5;

/// One resolution can run at a time.
static RESOLVING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// What a resolution did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveReport {
    /// Nothing had changed since the last resolution.
    pub skipped: bool,
    /// Questions sent to the model.
    pub asked: usize,
    /// Questions the model did not answer.
    pub failed: usize,
    /// Questions left for another run (the limit, or the provider stopped).
    pub left: usize,
    /// Groups of mentions of one document, and the entities they make.
    pub groups: usize,
    pub entities: usize,
    /// Joins, by what made them.
    pub by_person: usize,
    pub by_identifier: usize,
    pub by_name: usize,
    pub by_model: usize,
    /// Pairs left for a person.
    pub doubts: usize,
    /// Entities whose mentions, name or claims changed (pages to project).
    pub changed: usize,
    pub claims: usize,
    pub conflicts: usize,
    /// Pieces of evidence whose value has not the shape of its predicate.
    pub unfit: usize,
    /// Matters (groups of documents) and the documents in them.
    pub matters: usize,
    pub matter_documents: usize,
    /// Keys too common to look for candidates by.
    pub common_keys: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct Row {
    pub id: i64,
    pub content_id: i64,
    pub reference: String,
    pub same_as: Option<String>,
    pub kind: String,
    pub written: String,
    pub normalized: String,
    pub role: String,
    pub quote: String,
    pub entity: Option<String>,
    /// A term the document defines ("la Empresa"): never what names its
    /// group (D-245).
    pub defined: bool,
    /// The entity this name of the document had before its mentions went
    /// away (read again, or the document it took the place of).
    pub hint: Option<String>,
}

/// The mentions of one document that are the same entity.
#[derive(Debug, Clone)]
pub(crate) struct Cluster {
    pub content_id: i64,
    pub kind: String,
    pub rows: Vec<usize>,
    pub principal: usize,
    pub name: NameKey,
    /// `cluster:<content>:<kind>:<normalized name>`: what decisions and
    /// verdicts refer to.
    pub key: String,
    /// What it is ordered by: the text of its document and its name, never
    /// the order in which the archive was read.
    pub canon: String,
    /// The normalized names of its mentions.
    pub names: BTreeSet<String>,
    /// Strong identifiers by kind.
    pub strong: BTreeMap<String, BTreeSet<String>>,
    /// Left out of the entities: why.
    pub excluded: Option<&'static str>,
    /// The names of its mentions as compared (the principal's first),
    /// without the terms the document defines: the long name and the short
    /// one the document gives it are both its names (D-264).
    pub keys: Vec<NameKey>,
    /// The roles of its mentions.
    pub roles: BTreeSet<String>,
    /// An area or a mailbox of an organization: joined to the organization
    /// its document names, never by its name (D-262).
    pub area: Option<names::Area>,
}

pub(crate) fn cluster_key(content_id: i64, kind: &str, normalized: &str) -> String {
    format!("cluster:{content_id}:{kind}:{normalized}")
}

/// `(content, kind, normalized)` of a cluster key.
pub(crate) fn parse_cluster_key(key: &str) -> Option<(i64, String, String)> {
    let rest = key.strip_prefix("cluster:")?;
    let (content, rest) = rest.split_once(':')?;
    let (kind, normalized) = rest.split_once(':')?;
    Some((content.parse().ok()?, kind.to_string(), normalized.to_string()))
}

fn hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hasher.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
}

pub(crate) fn load_rows(conn: &Connection) -> Result<Vec<Row>, String> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT m.id, m.content_id, m.ref, m.same_as, m.kind, m.written, m.normalized, m.role, m.quote, m.entity_id, m.defined
             FROM knowledge_mention m JOIN knowledge_extraction k ON k.content_id = m.content_id AND k.state = 'done'
             WHERE {}
             ORDER BY m.content_id, m.id",
            changes::active_content("m.content_id")
        ))
        .map_err(sql_err)?;
    let hints = changes::entity_hints(conn)?;
    let rows = statement
        .query_map([], |row| {
            Ok(Row {
                id: row.get(0)?,
                content_id: row.get(1)?,
                reference: row.get(2)?,
                same_as: row.get(3)?,
                kind: row.get(4)?,
                written: row.get(5)?,
                normalized: row.get(6)?,
                role: row.get(7)?,
                quote: row.get(8)?,
                entity: row.get(9)?,
                defined: row.get(10)?,
                hint: None,
            })
        })
        .map_err(sql_err)?;
    let mut rows = rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)?;
    for row in rows.iter_mut().filter(|r| r.entity.is_none()) {
        row.hint = hints.get(&(row.content_id, row.kind.clone(), row.normalized.clone())).cloned();
    }
    Ok(rows)
}

fn load_identifiers(conn: &Connection) -> Result<HashMap<i64, Vec<(String, String, String)>>, String> {
    let mut statement = conn.prepare("SELECT mention_id, kind, value, quote FROM knowledge_identifier ORDER BY kind, value").map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))
        .map_err(sql_err)?;
    let mut out: HashMap<i64, Vec<(String, String, String)>> = HashMap::new();
    for row in rows {
        let (mention, kind, value, quote) = row.map_err(sql_err)?;
        out.entry(mention).or_default().push((kind, value, quote));
    }
    Ok(out)
}

fn load_hashes(conn: &Connection) -> Result<HashMap<i64, String>, String> {
    let mut statement = conn.prepare("SELECT content_id, text_hash FROM knowledge_extraction").map_err(sql_err)?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
    rows.collect::<Result<HashMap<_, _>, _>>().map_err(sql_err)
}

/// A small union-find over indices.
struct Sets {
    parent: Vec<usize>,
}

impl Sets {
    fn new(n: usize) -> Self {
        Self { parent: (0..n).collect() }
    }
    fn find(&mut self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        let mut x = x;
        while self.parent[x] != root {
            let next = self.parent[x];
            self.parent[x] = root;
            x = next;
        }
        root
    }
    fn union(&mut self, a: usize, b: usize) -> usize {
        let (a, b) = (self.find(a), self.find(b));
        let (keep, gone) = if a <= b { (a, b) } else { (b, a) };
        self.parent[gone] = keep;
        keep
    }
}

/// Whether two names of one document cannot be the same entity: two
/// legal forms, or two strong identifiers of the same kind.
fn apart_in_document(vocabulary: &Vocabulary, a: &NameKey, b: &NameKey, ids_a: &[(String, String)], ids_b: &[(String, String)]) -> bool {
    if matches!((&a.form, &b.form), (Some(x), Some(y)) if x != y) {
        return true;
    }
    ids_a.iter().any(|(kind, value)| {
        vocabulary.is_strong(kind) && ids_b.iter().any(|(k, _)| k == kind) && !ids_b.iter().any(|(k, v)| k == kind && v == value)
    })
}

/// The way of writing a name the most mentions use. An abbreviation left
/// without its last dot ("Ejemplo S.A") counts for the same name with it
/// ("Ejemplo S.A."), which is the one shown (D-282).
pub(crate) fn most_written(forms: &BTreeMap<&str, usize>) -> String {
    let completed = |w: &str| -> Option<String> {
        let last = w.split_whitespace().last()?;
        (!w.ends_with('.') && last.contains('.') && last.chars().last().is_some_and(char::is_alphabetic)).then(|| format!("{w}."))
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for (written, count) in forms {
        let target = completed(written).filter(|c| forms.contains_key(c.as_str())).unwrap_or_else(|| written.to_string());
        *counts.entry(target).or_default() += count;
    }
    counts.into_iter().max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(&a.0))).map(|(w, _)| w).unwrap_or_default()
}

/// Whether the model's "the same" within a document joins the complete
/// name of a company with an organization's name that has nothing in
/// common with it (P-134, D-281): a trade name written with another
/// company's name in parentheses ("Marca (Sociedad S.A.)") is at most
/// related to it, never that company. Names that share words, an acronym
/// or a typo still join.
fn unrelated_company_names(kind: &str, a: &NameKey, b: &NameKey) -> bool {
    kind == ORGANIZATION
        && (a.form.is_some() || b.form.is_some())
        && a.has_name()
        && b.has_name()
        && names::compare(kind, a, b) == Likeness::Unlike
}

/// The groups of mentions of each document, in canonical order.
pub(crate) fn clusters(
    vocabulary: &Vocabulary,
    rows: &[Row],
    identifiers: &HashMap<i64, Vec<(String, String, String)>>,
    hashes: &HashMap<i64, String>,
) -> Vec<Cluster> {
    let keys: Vec<NameKey> = rows.iter().map(|r| names::key(vocabulary, &r.kind, &r.written)).collect();
    let mut ids: Vec<Vec<(String, String)>> = rows
        .iter()
        .map(|r| {
            identifiers
                .get(&r.id)
                .map(|list| list.iter().map(|(k, v, _)| (k.clone(), names::identifier_value(v))).collect())
                .unwrap_or_default()
        })
        .collect();
    // The same number read as another kind ("otro número" in one document,
    // the DNI in others) is that strong identifier: the reading did not
    // say which kind it is, the number says who.
    let applies = |kind: &str, entity: &str| vocabulary.identifier(kind).is_some_and(|t| t.strong && (t.types.is_empty() || t.types.iter().any(|x| x == entity)));
    let mut strong_kind: HashMap<(String, String), BTreeSet<String>> = HashMap::new();
    for (row, list) in rows.iter().zip(&ids) {
        for (kind, value) in list {
            if applies(kind, &row.kind) && value.chars().filter(|c| c.is_ascii_digit()).count() >= 6 {
                strong_kind.entry((row.kind.clone(), value.clone())).or_default().insert(kind.clone());
            }
        }
    }
    for (row, list) in rows.iter().zip(ids.iter_mut()) {
        for (kind, value) in list.iter_mut() {
            if applies(kind, &row.kind) {
                continue;
            }
            if let Some(kinds) = strong_kind.get(&(row.kind.clone(), value.clone())).filter(|k| k.len() == 1) {
                *kind = kinds.iter().next().unwrap().clone();
            }
        }
    }
    let mut sets = Sets::new(rows.len());
    let mut start = 0;
    while start < rows.len() {
        let content = rows[start].content_id;
        let end = (start..rows.len()).find(|&i| rows[i].content_id != content).unwrap_or(rows.len());
        let mut by_name: HashMap<(&str, &str), usize> = HashMap::new();
        let mut by_ref: HashMap<&str, usize> = HashMap::new();
        for i in start..end {
            by_ref.insert(rows[i].reference.as_str(), i);
            match by_name.get(&(rows[i].kind.as_str(), rows[i].normalized.as_str())) {
                Some(&j) => {
                    sets.union(i, j);
                }
                None => {
                    by_name.insert((rows[i].kind.as_str(), rows[i].normalized.as_str()), i);
                }
            }
        }
        for i in start..end {
            let Some(j) = rows[i].same_as.as_deref().and_then(|s| by_ref.get(s)).copied() else { continue };
            // What a defined term stands for is said by the definition; any
            // other "the same" of the model has to fit the names (D-281).
            let by_definition = rows[i].defined || rows[j].defined;
            if rows[i].kind == rows[j].kind
                && !apart_in_document(vocabulary, &keys[i], &keys[j], &ids[i], &ids[j])
                && (by_definition || !unrelated_company_names(&rows[i].kind, &keys[i], &keys[j]))
            {
                sets.union(i, j);
            }
        }
        start = end;
    }
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..rows.len() {
        let root = sets.find(i);
        members.entry(root).or_default().push(i);
    }
    let mut out: Vec<Cluster> = Vec::new();
    for (_, list) in members {
        let count = |n: &str| list.iter().filter(|&&i| rows[i].normalized == n).count();
        let principal = *list
            .iter()
            .max_by(|&&a, &&b| {
                (!rows[a].defined)
                    .cmp(&!rows[b].defined)
                    .then_with(|| keys[a].weight().cmp(&keys[b].weight()))
                    .then_with(|| count(&rows[a].normalized).cmp(&count(&rows[b].normalized)))
                    .then_with(|| rows[b].normalized.cmp(&rows[a].normalized))
                    .then_with(|| rows[b].id.cmp(&rows[a].id))
            })
            .expect("a group has mentions");
        let first = &rows[principal];
        let mut name = keys[principal].clone();
        if name.email.is_none() {
            name.email = list.iter().find_map(|&i| keys[i].email.clone());
        }
        let mut strong: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for &i in &list {
            for (kind, value) in &ids[i] {
                let applies = vocabulary.identifier(kind).is_some_and(|t| t.strong && (t.types.is_empty() || t.types.contains(&first.kind)));
                if applies && value.chars().count() >= MIN_STRONG {
                    strong.entry(kind.clone()).or_default().insert(value.clone());
                }
            }
        }
        let mut cluster_keys: Vec<NameKey> = vec![name.clone()];
        for &i in &list {
            if !rows[i].defined && keys[i].has_name() && !cluster_keys.iter().any(|k| k.exact_key(&first.kind) == keys[i].exact_key(&first.kind)) {
                cluster_keys.push(keys[i].clone());
            }
        }
        let area = if first.defined { None } else { names::area(vocabulary, &first.kind, &name) };
        let mailbox_person = first.kind == PERSON && area.is_some();
        out.push(Cluster {
            keys: cluster_keys,
            roles: list.iter().map(|&i| rows[i].role.clone()).collect(),
            area,
            content_id: first.content_id,
            kind: first.kind.clone(),
            principal,
            key: cluster_key(first.content_id, &first.kind, &first.normalized),
            canon: format!("{}|{}|{}", hashes.get(&first.content_id).map(String::as_str).unwrap_or(""), first.kind, first.normalized),
            names: list.iter().map(|&i| rows[i].normalized.clone()).collect(),
            excluded: if first.defined {
                // Only terms the document defines, without what they define.
                Some("termino-definido")
            } else if mailbox_person {
                // A general mailbox read as a person is no one (D-262).
                Some("casilla")
            } else {
                (!name.has_name() && name.email.is_none()).then_some("sin-nombre")
            },
            name,
            strong,
            rows: list,
        });
    }
    // Within a document, a name of an organization without its legal form
    // is the one complete form of it the document writes (D-261): the same
    // words with a form, or a longer name that starts with them, when no
    // other organization of the document has those words.
    coreference(vocabulary, &mut out);
    // An identifier given to mentions of two kinds, or to two groups of
    // one document, was attributed to the wrong mention: it does not join.
    let mut kinds: HashMap<(String, String), BTreeSet<String>> = HashMap::new();
    let mut per_document: HashMap<(i64, String, String), usize> = HashMap::new();
    for cluster in &out {
        for (kind, values) in &cluster.strong {
            for value in values {
                kinds.entry((kind.clone(), value.clone())).or_default().insert(cluster.kind.clone());
                *per_document.entry((cluster.content_id, kind.clone(), value.clone())).or_default() += 1;
            }
        }
    }
    for cluster in &mut out {
        let content = cluster.content_id;
        for (kind, values) in cluster.strong.iter_mut() {
            values.retain(|v| {
                kinds[&(kind.clone(), v.clone())].len() == 1 && per_document[&(content, kind.clone(), v.clone())] == 1
            });
        }
        cluster.strong.retain(|_, v| !v.is_empty());
    }
    out.sort_by(|a, b| a.canon.cmp(&b.canon).then_with(|| a.key.cmp(&b.key)));
    out
}

/// Joins, within each document, a group of an organization named without
/// its legal form to the one complete form of the same name (D-261).
fn coreference(vocabulary: &Vocabulary, out: &mut Vec<Cluster>) {
    let mut by_content: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (i, cluster) in out.iter().enumerate() {
        if cluster.kind == ORGANIZATION && cluster.excluded.is_none() {
            by_content.entry(cluster.content_id).or_default().push(i);
        }
    }
    let mut gone: BTreeSet<usize> = BTreeSet::new();
    for list in by_content.values() {
        for &short in list {
            let name = &out[short].name;
            if gone.contains(&short) || name.form.is_some() || name.words.is_empty() || out[short].area.is_some() {
                continue;
            }
            let words = &name.words;
            // The organizations of the document whose name holds these
            // words (any of their names).
            let holders: Vec<usize> = list
                .iter()
                .copied()
                .filter(|&o| o != short && !gone.contains(&o) && out[o].area.is_none())
                .filter(|&o| out[o].keys.iter().any(|k| k.words.len() >= words.len() && k.words.windows(words.len()).any(|w| w == words.as_slice())))
                .collect();
            let complete: Vec<usize> = holders
                .iter()
                .copied()
                .filter(|&o| out[o].keys.iter().any(|k| (k.form.is_some() && &k.words == words) || (k.words.len() > words.len() && k.words.starts_with(words))))
                .collect();
            // Two ways of writing the same complete name are one form.
            let distinct = |list: &[usize]| list.iter().map(|&o| out[o].name.exact_key(ORGANIZATION)).collect::<BTreeSet<_>>().len();
            if complete.is_empty() || distinct(&holders) != 1 || distinct(&complete) != 1 {
                continue;
            }
            let target = complete[0];
            let apart = out[short].strong.iter().any(|(kind, values)| out[target].strong.get(kind).is_some_and(|other| values.is_disjoint(other)));
            if apart || !vocabulary.is_a_name(ORGANIZATION, &out[target].name.joined()) {
                continue;
            }
            let moved = out[short].clone();
            let into = &mut out[target];
            into.rows.extend(moved.rows);
            into.names.extend(moved.names);
            for (kind, values) in moved.strong {
                into.strong.entry(kind).or_default().extend(values);
            }
            for key in moved.keys {
                if !into.keys.iter().any(|k| k.exact_key(ORGANIZATION) == key.exact_key(ORGANIZATION)) {
                    into.keys.push(key);
                }
            }
            into.roles.extend(moved.roles);
            gone.insert(short);
        }
    }
    let mut index = 0;
    out.retain(|_| {
        let keep = !gone.contains(&index);
        index += 1;
        keep
    });
}

/// The group of each name of a document: what a key of a group refers to.
pub(crate) fn names_index(clusters: &[Cluster]) -> HashMap<(i64, String, String), usize> {
    let mut index = HashMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        for name in &cluster.names {
            index.insert((cluster.content_id, cluster.kind.clone(), name.clone()), i);
        }
    }
    index
}

/// What a person decided, on the keys of groups of mentions.
#[derive(Debug, Clone, Default)]
pub(crate) struct Decided {
    pub joins: Vec<(String, String)>,
    pub separations: Vec<(String, String)>,
    /// Keys of groups, evidence or claims marked as wrong.
    pub wrong: BTreeSet<String>,
    /// Keys of claims a person confirmed.
    pub confirmed: BTreeSet<String>,
    /// Every active decision, placed on what the documents say now
    /// (`anchors.rs`): the keys above are the ones placed.
    pub anchored: Vec<super::anchors::Anchored>,
}

pub(crate) fn load_decided(conn: &Connection) -> Result<Decided, String> {
    let anchored = super::anchors::place_all(conn, vocabulary())?;
    let mut out = Decided::default();
    for decision in &anchored {
        let a = decision.a.key().to_string();
        let b = decision.b.as_ref().map(|b| b.key().to_string());
        match (decision.kind.as_str(), a, b) {
            ("join", a, Some(b)) => out.joins.push((a, b)),
            ("separate", a, Some(b)) => out.separations.push((a, b)),
            ("wrong", a, _) => {
                out.wrong.insert(a);
            }
            ("confirm", a, _) => {
                out.confirmed.insert(a);
            }
            _ => {}
        }
    }
    out.anchored = anchored;
    Ok(out)
}

/// What the model said of a pair of groups.
#[derive(Debug, Clone, PartialEq)]
struct Verdict {
    verdict: String,
    reason: Option<String>,
    /// The fingerprint of what the two sides showed, when it was said.
    evidence: Option<String>,
}

/// The verdicts kept: with an engine, only its own; without one, the
/// latest of any engine (D-177).
fn load_verdicts(conn: &Connection, engine: Option<&str>) -> Result<HashMap<(String, String), Verdict>, String> {
    // A verdict said to another version of the question is not kept: the
    // question changed, so it is asked again (R-20).
    let mut statement = conn
        .prepare(
            "SELECT v.a, v.b, v.engine, v.verdict, v.reason, v.evidence FROM knowledge_verdict v
             WHERE NOT EXISTS (SELECT 1 FROM model_answer m WHERE m.question = v.question AND m.prompt_version != ?1)
             ORDER BY v.asked_at, v.engine",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([IDENTITY_PROMPT], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, Option<String>>(5)?,
            ))
        })
        .map_err(sql_err)?;
    let mut out = HashMap::new();
    for row in rows {
        let (a, b, by, verdict, reason, evidence) = row.map_err(sql_err)?;
        if engine.is_some_and(|e| e != by) {
            continue;
        }
        out.insert((a, b), Verdict { verdict, reason, evidence });
    }
    Ok(out)
}

fn pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.to_string(), b.to_string())
    } else {
        (b.to_string(), a.to_string())
    }
}

/// A pair the resolution could not tell apart.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Doubt {
    pub reason: String,
    pub detail: Option<String>,
}

/// The working state of one resolution.
struct Resolver<'a> {
    clusters: &'a [Cluster],
    sets: Sets,
    strong: Vec<BTreeMap<String, BTreeSet<String>>>,
    forms: Vec<BTreeSet<String>>,
    size: Vec<usize>,
    /// Pairs of clusters a person kept apart.
    separated: Vec<(usize, usize)>,
    /// Pairs a person decided on: never a doubt.
    decided: HashSet<(usize, usize)>,
    /// The joins made: (cluster, cluster, by what).
    links: Vec<(usize, usize, &'static str)>,
    doubts: BTreeMap<(usize, usize), Doubt>,
}

impl<'a> Resolver<'a> {
    fn new(clusters: &'a [Cluster]) -> Self {
        Self {
            clusters,
            sets: Sets::new(clusters.len()),
            strong: clusters.iter().map(|c| c.strong.clone()).collect(),
            forms: clusters.iter().map(|c| c.name.form.iter().cloned().collect()).collect(),
            size: clusters.iter().map(|c| c.rows.len()).collect(),
            separated: Vec::new(),
            decided: HashSet::new(),
            links: Vec::new(),
            doubts: BTreeMap::new(),
        }
    }

    fn find(&mut self, x: usize) -> usize {
        self.sets.find(x)
    }

    /// What keeps two groups apart, if anything: a person's separation,
    /// another strong identifier of the same kind, another legal form.
    fn apart(&mut self, a: usize, b: usize, by_person: bool) -> Option<&'static str> {
        let (a, b) = (self.find(a), self.find(b));
        for (x, y) in self.separated.clone() {
            let (x, y) = (self.find(x), self.find(y));
            if (x == a && y == b) || (x == b && y == a) {
                return Some("separadas-por-una-persona");
            }
        }
        if by_person {
            return None;
        }
        for (kind, values) in &self.strong[a] {
            if self.strong[b].get(kind).is_some_and(|other| values.is_disjoint(other)) {
                return Some("otro-identificador");
            }
        }
        if !self.forms[a].is_empty() && !self.forms[b].is_empty() && self.forms[a].is_disjoint(&self.forms[b]) {
            return Some("otra-forma");
        }
        None
    }

    fn join(&mut self, x: usize, y: usize, by: &'static str) -> Result<bool, &'static str> {
        let (a, b) = (self.find(x), self.find(y));
        if a == b {
            return Ok(false);
        }
        // A person and an organization are never one entity.
        if self.clusters[x].kind != self.clusters[y].kind {
            return Err("otro-tipo");
        }
        if let Some(why) = self.apart(a, b, by == "persona") {
            return Err(why);
        }
        let keep = self.sets.union(a, b);
        let gone = if keep == a { b } else { a };
        let strong = std::mem::take(&mut self.strong[gone]);
        for (kind, values) in strong {
            self.strong[keep].entry(kind).or_default().extend(values);
        }
        let forms = std::mem::take(&mut self.forms[gone]);
        self.forms[keep].extend(forms);
        self.size[keep] += self.size[gone];
        self.links.push((x, y, by));
        Ok(true)
    }

    fn doubt(&mut self, x: usize, y: usize, reason: &str, detail: Option<String>) {
        let key = (x.min(y), x.max(y));
        if x == y || self.decided.contains(&key) {
            return;
        }
        self.doubts.entry(key).or_insert(Doubt { reason: reason.to_string(), detail });
    }

    /// 0. What a person decided.
    fn persons(&mut self, decided: &Decided) {
        let index = names_index(self.clusters);
        let find = |key: &str| -> Option<usize> {
            let (content, kind, normalized) = parse_cluster_key(key)?;
            index.get(&(content, kind, normalized)).copied()
        };
        for (a, b) in &decided.separations {
            if let (Some(x), Some(y)) = (find(a), find(b)) {
                self.separated.push((x, y));
                self.decided.insert((x.min(y), x.max(y)));
            }
        }
        for (a, b) in &decided.joins {
            if let (Some(x), Some(y)) = (find(a), find(b)) {
                self.decided.insert((x.min(y), x.max(y)));
                if self.clusters[x].kind == self.clusters[y].kind && self.clusters[x].excluded.is_none() && self.clusters[y].excluded.is_none() {
                    let _ = self.join(x, y, "persona");
                }
            }
        }
    }

    /// 1. Strong identifiers.
    fn identifiers(&mut self) {
        let mut index: BTreeMap<(String, String, String), Vec<usize>> = BTreeMap::new();
        for (i, cluster) in self.clusters.iter().enumerate() {
            if cluster.excluded.is_some() {
                continue;
            }
            for (kind, values) in &cluster.strong {
                for value in values {
                    index.entry((cluster.kind.clone(), kind.clone(), value.clone())).or_default().push(i);
                }
            }
        }
        for ((kind, _, _), list) in &index {
            let principal = list[0];
            for &other in &list[1..] {
                let (a, b) = (&self.clusters[principal].name, &self.clusters[other].name);
                match names::compare(kind, a, b) {
                    Likeness::Unlike if a.has_name() && b.has_name() => self.doubt(principal, other, "identificador-otro-nombre", None),
                    Likeness::OtherForm => self.doubt(principal, other, "identificador-otra-forma", None),
                    _ => {
                        if let Err(why) = self.join(principal, other, "identificador") {
                            self.doubt(principal, other, why, None);
                        }
                    }
                }
            }
        }
        // One character of difference, with names that may be the same:
        // perhaps a misread scan.
        let mut values: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for cluster in self.clusters.iter().filter(|c| c.excluded.is_none()) {
            for (kind, list) in &cluster.strong {
                values.entry((cluster.kind.clone(), kind.clone())).or_default().extend(list.iter().cloned());
            }
        }
        for ((entity_kind, kind), list) in values {
            let mut deletions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for value in &list {
                let chars: Vec<char> = value.chars().collect();
                deletions.entry(value.clone()).or_default().insert(value.clone());
                for skip in 0..chars.len() {
                    let key: String = chars.iter().enumerate().filter(|(i, _)| *i != skip).map(|(_, c)| c).collect();
                    deletions.entry(key).or_default().insert(value.clone());
                }
            }
            let mut near: BTreeSet<(String, String)> = BTreeSet::new();
            for group in deletions.values() {
                let group: Vec<&String> = group.iter().collect();
                for i in 0..group.len() {
                    for j in i + 1..group.len() {
                        if names::misread(group[i], group[j]) {
                            near.insert((group[i].clone(), group[j].clone()));
                        }
                    }
                }
            }
            for (va, vb) in near {
                let with = |v: &str| -> Vec<usize> {
                    index.get(&(entity_kind.clone(), kind.clone(), v.to_string())).cloned().unwrap_or_default()
                };
                let (xa, xb) = (with(&va), with(&vb));
                for &x in &xa {
                    for &y in &xb {
                        if self.find(x) == self.find(y) {
                            continue;
                        }
                        match names::compare(&entity_kind, &self.clusters[x].name, &self.clusters[y].name) {
                            Likeness::Exact | Likeness::Maybe(_) => {
                                self.doubt(x, y, "identificador-parecido", Some(format!("{kind}: {va} / {vb}")))
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    /// 2. The same name.
    fn same_names(&mut self) {
        let mut index: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, cluster) in self.clusters.iter().enumerate() {
            if cluster.excluded.is_some() || cluster.area.is_some() {
                continue;
            }
            // Every name of the group (the long name and the short one the
            // document gives it), not only its principal one (D-264).
            for name in &cluster.keys {
                let exact = match cluster.kind.as_str() {
                    PERSON => name.words.len() >= 2,
                    ORGANIZATION => name.words.len() >= 2 || (name.form.is_some() && !name.words.is_empty()),
                    _ => false,
                };
                if exact && name.has_name() {
                    let list = index.entry(name.exact_key(&cluster.kind)).or_default();
                    if !list.contains(&i) {
                        list.push(i);
                    }
                }
            }
            let name = &cluster.name;
            if let Some(email) = &name.email {
                if cluster.kind != PERSON && cluster.kind != ORGANIZATION {
                    continue;
                }
                index.entry(format!("{}|@{email}", cluster.kind)).or_default().push(i);
            }
        }
        for (key, list) in index {
            if list.len() < 2 {
                continue;
            }
            let by_address = key.contains("|@");
            let mut roots: Vec<usize> = Vec::new();
            for &i in &list {
                let root = self.find(i);
                if !roots.contains(&root) {
                    roots.push(root);
                }
            }
            // Identifiers that keep some of them apart: only those without
            // any identifier are in doubt, against each of the others.
            let mut conflict = false;
            for i in 0..roots.len() {
                for j in i + 1..roots.len() {
                    if self.apart(roots[i], roots[j], false) == Some("otro-identificador") {
                        conflict = true;
                    }
                }
            }
            if conflict {
                let identified: Vec<usize> = list.iter().copied().filter(|&i| !self.strong[self.sets.parent_of(i)].is_empty()).collect();
                for &i in &list {
                    let root = self.find(i);
                    if self.strong[root].is_empty() {
                        for &j in &identified {
                            self.doubt(i, j, "mismo-nombre-otro-identificador", None);
                        }
                    }
                }
                continue;
            }
            let principal = list[0];
            for &other in &list[1..] {
                if by_address && names::compare(&self.clusters[principal].kind, &self.clusters[principal].name, &self.clusters[other].name) != Likeness::Exact {
                    continue;
                }
                if let Err(why) = self.join(principal, other, "nombre") {
                    self.doubt(principal, other, why, None);
                }
            }
        }
    }
}

impl<'a> Resolver<'a> {
    /// 2b. Areas and general mailboxes (D-262): each is a contact of the
    /// organization its own document names (the words left without the
    /// area's, its initials, its mail domain), or of the one organization
    /// of the archive with the name of its domain. The same area in several
    /// documents is one: joined to its organization when all of them say
    /// the same one; when they say several, it is theirs in common (their
    /// group, D-260), and left apart here. Returns the areas left without
    /// an organization, by their key, with the organizations they name.
    fn areas(&mut self) -> BTreeMap<String, (Vec<usize>, BTreeSet<usize>)> {
        let clusters = self.clusters;
        let mut by_content: HashMap<i64, Vec<usize>> = HashMap::new();
        for (i, cluster) in clusters.iter().enumerate() {
            if cluster.excluded.is_none() && cluster.area.is_none() && cluster.kind == ORGANIZATION {
                by_content.entry(cluster.content_id).or_default().push(i);
            }
        }
        let mut areas: BTreeMap<String, (Vec<usize>, BTreeSet<usize>)> = BTreeMap::new();
        for (i, cluster) in clusters.iter().enumerate() {
            let Some(area) = cluster.area.as_ref().filter(|_| cluster.excluded.is_none()) else { continue };
            let key = match &cluster.name.email {
                Some(email) => format!("{}|@{email}", cluster.kind),
                None => cluster.name.exact_key(&cluster.kind),
            };
            let mut named: BTreeSet<usize> = BTreeSet::new();
            for &o in by_content.get(&cluster.content_id).into_iter().flatten() {
                if clusters[o].keys.iter().any(|k| names::area_names(area, k)) {
                    named.insert(o);
                }
            }
            if named.is_empty() && area.domain.is_some() {
                // The domain of the mailbox names one organization of the
                // archive (its words joined).
                let domain = names::Area { inner: Vec::new(), domain: area.domain.clone() };
                for (o, other) in clusters.iter().enumerate() {
                    if other.kind == ORGANIZATION && other.excluded.is_none() && other.area.is_none() && other.keys.iter().any(|k| names::area_names(&domain, k)) {
                        named.insert(o);
                    }
                }
            }
            let entry = areas.entry(key).or_default();
            entry.0.push(i);
            entry.1.extend(named);
        }
        let mut left: BTreeMap<String, (Vec<usize>, BTreeSet<usize>)> = BTreeMap::new();
        for (key, (list, named)) in areas {
            let roots: BTreeSet<usize> = named.iter().map(|&o| self.find(o)).collect();
            // One name of one organization (written with its form or not,
            // not yet joined): it is that one.
            let words: BTreeSet<&Vec<String>> = named.iter().map(|&o| &clusters[o].name.words).collect();
            let forms: BTreeSet<&String> = named.iter().filter_map(|&o| clusters[o].name.form.as_ref()).collect();
            let one_name = words.len() == 1 && forms.len() <= 1;
            if roots.len() == 1 || one_name {
                let target = named
                    .iter()
                    .copied()
                    .find(|&o| clusters[o].name.form.is_some())
                    .unwrap_or_else(|| *named.iter().next().unwrap());
                for &i in &list {
                    if let Err(why) = self.join(target, i, "area") {
                        self.doubt(target, i, why, None);
                    }
                }
                continue;
            }
            // The same area in several documents is one, whoever it is of.
            for &i in &list[1..] {
                let _ = self.join(list[0], i, "nombre");
            }
            left.insert(key, (list, named));
        }
        left
    }
}

impl Sets {
    /// The root of `x` without compressing (for a shared borrow).
    fn parent_of(&self, x: usize) -> usize {
        let mut root = x;
        while self.parent[root] != root {
            root = self.parent[root];
        }
        root
    }
}

/// The candidates for the model: pairs of entities (after the joins by
/// identifier and by name) whose names may be the same, with the reason.
fn candidates(resolver: &mut Resolver, common: &mut usize, formless: &mut BTreeMap<usize, BTreeSet<usize>>) -> BTreeMap<(usize, usize), &'static str> {
    let clusters = resolver.clusters;
    let mut index: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        if cluster.excluded.is_some() || cluster.area.is_some() {
            continue;
        }
        let mut keys: BTreeSet<String> = BTreeSet::new();
        for name in &cluster.keys {
            keys.extend(names::block_keys(name));
        }
        for key in keys {
            index.entry(format!("{}|{key}", cluster.kind)).or_default().push(i);
        }
    }
    // The parties of each document (its groups with a party's role), to
    // tell two matters of the same name apart: a contract is between its
    // parties (D-263).
    let party_roles = &vocabulary().matters.party_roles;
    let mut parties: HashMap<i64, BTreeSet<usize>> = HashMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        if cluster.excluded.is_none() && cluster.kind != "asunto" && cluster.area.is_none() && cluster.roles.iter().any(|r| party_roles.contains(r)) {
            parties.entry(cluster.content_id).or_default().insert(i);
        }
    }
    // Names of organizations with their legal form: a name without a form
    // that more than one form shares (the same name as a local company
    // and as a foreign one) cannot be told to be either.
    let mut formed: BTreeSet<(Vec<String>, String)> = BTreeSet::new();
    for cluster in clusters.iter().filter(|c| c.excluded.is_none() && c.kind == ORGANIZATION) {
        if let Some(form) = &cluster.name.form {
            let mut words = cluster.name.words.clone();
            words.sort();
            formed.insert((words, form.clone()));
        }
    }
    let mut ambiguous: HashMap<usize, bool> = HashMap::new();
    let mut is_ambiguous = |i: usize| -> bool {
        *ambiguous.entry(i).or_insert_with(|| {
            let cluster = &clusters[i];
            if cluster.kind != ORGANIZATION || cluster.name.form.is_some() || cluster.name.words.is_empty() {
                return false;
            }
            let own: BTreeSet<&String> = cluster.name.words.iter().collect();
            let forms: BTreeSet<&String> = formed
                .iter()
                .filter(|(words, _)| {
                    let theirs: BTreeSet<&String> = words.iter().collect();
                    !theirs.is_empty() && (theirs.is_subset(&own) || own.is_subset(&theirs))
                })
                .map(|(_, form)| form)
                .collect();
            forms.len() >= 2
        })
    };
    let mut seen: HashSet<(usize, usize)> = HashSet::new();
    let mut edges: BTreeMap<(usize, usize), &'static str> = BTreeMap::new();
    // Names of one word that may be a person's partial name, by partner.
    let mut single: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    for list in index.values() {
        if list.len() < 2 {
            continue;
        }
        if list.len() > BLOCK_CAP {
            *common += 1;
            continue;
        }
        for i in 0..list.len() {
            for j in i + 1..list.len() {
                let (x, y) = (list[i], list[j]);
                if !seen.insert((x, y)) {
                    continue;
                }
                let (rx, ry) = (resolver.find(x), resolver.find(y));
                if rx == ry {
                    continue;
                }
                let Likeness::Maybe(reason) = compare_groups(&clusters[x], &clusters[y]) else {
                    continue;
                };
                if resolver.apart(rx, ry, false).is_some() {
                    continue;
                }
                if clusters[x].kind == "asunto" && clusters[x].content_id != clusters[y].content_id {
                    // Two documents with two parties or more each, neither
                    // within the other: another contract between others.
                    let mut of = |c: usize| -> BTreeSet<usize> {
                        parties.get(&clusters[c].content_id).into_iter().flatten().map(|&p| resolver.find(p)).collect()
                    };
                    let (px, py) = (of(x), of(y));
                    if px.len() >= 2 && py.len() >= 2 && !px.is_subset(&py) && !py.is_subset(&px) {
                        continue;
                    }
                }
                let key = (rx.min(ry), rx.max(ry));
                let formless_against_formed = |a: usize, b: usize| clusters[a].name.form.is_none() && clusters[b].name.form.is_some();
                if formless_against_formed(x, y) && is_ambiguous(x) {
                    // A name without a legal form that several companies
                    // share: one question for all of them, later.
                    formless.entry(rx).or_default().insert(ry);
                    continue;
                }
                if formless_against_formed(y, x) && is_ambiguous(y) {
                    formless.entry(ry).or_default().insert(rx);
                    continue;
                }
                if clusters[x].kind == PERSON && reason == "una-palabra" {
                    let (sx, sy) = (clusters[x].name.words.len() == 1, clusters[y].name.words.len() == 1);
                    if sx && sy {
                        continue;
                    }
                    let (lone, partner) = if sx { (rx, ry) } else { (ry, rx) };
                    single.entry(lone).or_default().insert(partner);
                    continue;
                }
                edges.entry(key).or_insert(reason);
            }
        }
    }
    // A one-word name of a person is a candidate only for the one entity
    // whose name has that word.
    for (lone, partners) in single {
        if partners.len() == 1 {
            let partner = *partners.iter().next().unwrap();
            edges.entry((lone.min(partner), lone.max(partner))).or_insert("una-palabra");
        }
    }
    edges
}

/// How two groups of mentions compare: their principal names, or else the
/// best of their other names (D-264).
fn compare_groups(a: &Cluster, b: &Cluster) -> Likeness {
    let principal = names::compare(&a.kind, &a.name, &b.name);
    if principal != Likeness::Unlike {
        return principal;
    }
    for x in &a.keys {
        for y in &b.keys {
            match names::compare(&a.kind, x, y) {
                Likeness::Exact => return Likeness::Maybe("otro-nombre"),
                Likeness::Maybe(reason) => return Likeness::Maybe(reason),
                _ => {}
            }
        }
    }
    Likeness::Unlike
}

/// What the question shows of the documents.
struct Context {
    documents: HashMap<i64, String>,
    /// What each mention's document says of it, in words.
    facts: HashMap<i64, Vec<String>>,
    /// Groups by document.
    by_content: HashMap<i64, Vec<usize>>,
    /// The quote of each identifier, by mention.
    identifiers: HashMap<i64, Vec<(String, String, String)>>,
}

fn context(conn: &Connection, rows: &[Row], clusters: &[Cluster], identifiers: HashMap<i64, Vec<(String, String, String)>>) -> Result<Context, String> {
    let mut documents = HashMap::new();
    for cluster in clusters {
        if let std::collections::hash_map::Entry::Vacant(entry) = documents.entry(cluster.content_id) {
            entry.insert(classify::name_of(conn, cluster.content_id)?);
        }
    }
    let written: HashMap<i64, &str> = rows.iter().map(|r| (r.id, r.written.as_str())).collect();
    let mut facts: HashMap<i64, Vec<String>> = HashMap::new();
    let mut statement =
        conn.prepare("SELECT subject_mention, predicate, object_mention, object_value FROM knowledge_evidence ORDER BY id").map_err(sql_err)?;
    let found = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<i64>>(2)?, row.get::<_, Option<String>>(3)?)))
        .map_err(sql_err)?;
    for item in found {
        let (subject, predicate, object, value) = item.map_err(sql_err)?;
        let object = match (object, value) {
            (Some(m), _) => written.get(&m).map(|w| format!("«{w}»")),
            (None, Some(v)) => Some(v),
            _ => None,
        };
        if let Some(object) = object {
            facts.entry(subject).or_default().push(format!("{}: {object}", predicate.replace('_', " ")));
        }
    }
    let mut by_content: HashMap<i64, Vec<usize>> = HashMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        by_content.entry(cluster.content_id).or_default().push(i);
    }
    Ok(Context { documents, facts, by_content, identifiers })
}

/// The fingerprint of what a question shows of two sides.
fn shown_fingerprint(principal: &str, candidate: &str) -> String {
    let digest = Sha256::digest(format!("{principal}\u{0}{candidate}").as_bytes());
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn cut(text: &str, words: usize) -> String {
    let all: Vec<&str> = text.split_whitespace().collect();
    if all.len() <= words {
        all.join(" ")
    } else {
        format!("{} …", all[..words].join(" "))
    }
}

/// An entity as a question describes it: its own groups of mentions.
fn describe(rows: &[Row], clusters: &[Cluster], members: &[usize], context: &Context) -> String {
    let mut names: BTreeMap<&str, usize> = BTreeMap::new();
    let mut written: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    let mut identifiers: BTreeSet<String> = BTreeSet::new();
    let mut facts: BTreeMap<String, usize> = BTreeMap::new();
    let mut documents: BTreeSet<i64> = BTreeSet::new();
    for &c in members {
        documents.insert(clusters[c].content_id);
        for &r in &clusters[c].rows {
            let row = &rows[r];
            *names.entry(row.normalized.as_str()).or_default() += 1;
            *written.entry(row.normalized.as_str()).or_default().entry(row.written.as_str()).or_default() += 1;
            for (kind, value, _) in context.identifiers.get(&row.id).into_iter().flatten() {
                let applies = vocabulary().identifier(kind).is_some_and(|t| t.types.is_empty() || t.types.contains(&row.kind));
                if applies {
                    identifiers.insert(format!("{kind} {value}"));
                }
            }
            for fact in context.facts.get(&row.id).into_iter().flatten() {
                *facts.entry(fact.clone()).or_default() += 1;
            }
        }
    }
    let ranked = |map: &BTreeMap<String, usize>, max: usize| -> Vec<String> {
        let mut list: Vec<(&String, &usize)> = map.iter().collect();
        list.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        list.into_iter().take(max).map(|(k, _)| k.clone()).collect()
    };
    let mut name_list: Vec<(&str, usize)> = names.into_iter().collect();
    name_list.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let shown_names: Vec<String> = name_list
        .iter()
        .take(6)
        .map(|(n, count)| {
            let forms = &written[n];
            let form = forms.iter().max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0))).map(|(w, _)| *w).unwrap_or(n);
            format!("«{form}» ({count})")
        })
        .collect();
    let mut out = format!("- Nombres: {}\n", shown_names.join(", "));
    if !identifiers.is_empty() {
        out.push_str(&format!("- Identificadores: {}\n", identifiers.into_iter().take(6).collect::<Vec<_>>().join("; ")));
    }
    // Mail addresses, all of them: the most telling of the facts.
    let mails: BTreeSet<String> = facts
        .keys()
        .filter_map(|f| f.split_once(": ").map(|(_, v)| v.trim().to_lowercase()))
        .filter(|v| v.contains('@') && !v.contains(' '))
        .take(4)
        .collect();
    if !mails.is_empty() {
        out.push_str(&format!("- Correos: {}\n", mails.into_iter().collect::<Vec<_>>().join(", ")));
    }
    let facts = ranked(&facts, 6);
    if !facts.is_empty() {
        out.push_str(&format!("- Lo que dicen los documentos: {}\n", facts.join("; ")));
    }
    // Appearances: one per document, in the order of the documents' names.
    let mut shown: Vec<(String, String)> = Vec::new();
    let mut by_document: BTreeMap<(String, i64), Vec<usize>> = BTreeMap::new();
    for &c in members {
        let document = context.documents.get(&clusters[c].content_id).cloned().unwrap_or_default();
        by_document.entry((document, clusters[c].content_id)).or_default().push(c);
    }
    for ((document, _), list) in by_document.iter().take(APPEARANCES) {
        let c = list[0];
        let roles: BTreeSet<&str> = clusters[c].rows.iter().map(|&r| rows[r].role.as_str()).collect();
        let quote = &rows[clusters[c].principal].quote;
        shown.push((document.clone(), format!("como {}: «{}»", roles.into_iter().collect::<Vec<_>>().join(", "), cut(quote, 40))));
    }
    out.push_str(&format!("- Aparece en {} documento(s); por ejemplo:\n", documents.len()));
    for (document, text) in shown {
        out.push_str(&format!("  - «{document}», {text}\n"));
    }
    // Who else the same documents name.
    let own: BTreeSet<usize> = members.iter().copied().collect();
    let mut around: BTreeMap<String, usize> = BTreeMap::new();
    for content in &documents {
        for &other in context.by_content.get(content).into_iter().flatten() {
            if !own.contains(&other) && clusters[other].excluded.is_none() {
                *around.entry(rows[clusters[other].principal].written.clone()).or_default() += 1;
            }
        }
    }
    let around = ranked(&around, 6);
    if !around.is_empty() {
        out.push_str(&format!(
            "- Nombrados en los mismos documentos: {}\n",
            around.iter().map(|n| format!("«{n}»")).collect::<Vec<_>>().join(", ")
        ));
    }
    out
}

/// The question about a principal entity and its candidates.
fn question(vocabulary: &Vocabulary, kind: &str, principal: &str, candidates: &[String], content_id: i64, one_of: bool) -> Question {
    let about = vocabulary.entity_type(kind).map(|t| t.description.trim().to_string()).unwrap_or_default();
    let listed: Vec<String> = candidates.iter().enumerate().map(|(i, c)| format!("[{}]\n{c}", i + 1)).collect();
    let prompt = format!(
        "Sos asistente de un estudio jurídico. Varios documentos del archivo nombran entidades de tipo «{kind}» ({about}). \
Hay que decidir si cada candidato es la misma entidad que la principal. Usá solo lo que muestran estos nombres, datos y \
citas: no agregues nada que sepas por otro lado.\n\n\
Reglas:\n\
- \"misma\" solo si los documentos lo muestran con claridad: el mismo nombre completo, o una variante (otro orden, una inicial, \
una abreviatura, un error de tipeo evidente) con un contexto que coincide (la misma organización, el mismo cargo, el mismo \
domicilio, las mismas personas alrededor, el mismo asunto).\n\
- \"distinta\" si algo las separa: otro nombre de pila u otro apellido, otro identificador, otra forma societaria, otro país, un \
contexto que no puede ser el mismo. Una marca, un grupo empresario o un área no es la sociedad. Compartir un nombre de pila, un \
apellido, un teléfono o un domicilio no alcanza para que sean la misma.\n\
- Una persona puede tener cargos en varias organizaciones (directora de una sociedad y presidenta de otra del mismo grupo) y \
escribirse con o sin su segundo nombre: otro cargo u otra organización no alcanza para decir \"distinta\". La misma dirección \
de correo personal en las dos (no una casilla general como legales@, info@ o cobranzas@) es un dato fuerte de que es la misma \
persona.\n\
- \"dudosa\" si con esto no se puede saber. Una letra distinta en un nombre puede ser un error o una persona distinta: si el \
contexto no lo resuelve, \"dudosa\". En una organización, una razón social con la misma forma societaria que difiere en una o \
dos letras (una letra que falta o sobra, una sílaba repetida), en documentos donde aparecen las mismas otras partes (la misma \
contraparte, los mismos firmantes, el mismo asunto), es la misma: el error es del documento; si alguno de los documentos muestra \
otra organización con ese nombre, \"dudosa\". Ante la duda, \"dudosa\": unir dos entidades distintas es mucho peor que dejarlas separadas.\n\
- Si es \"misma\", en \"certeza\" poné \"alta\" solo si un dato de los documentos lo deja fuera de duda (el mismo \
identificador, el mismo nombre completo con la misma organización o el mismo cargo, una cita que dice que son la misma); si \
no, \"media\". Para \"distinta\" y \"dudosa\", la que corresponda.\n\
- En \"razon\", en una oración, qué dato de los documentos lo decide.\n{one_of}\n\
Principal:\n{principal}\n\
Candidatos:\n{}\n\
Respondé solo con el JSON pedido: una respuesta por candidato, con su número.",
        listed.join("\n"),
        one_of = if one_of {
            "- La principal se nombra sin su forma societaria y los candidatos son sociedades distintas entre sí (cada una con \
otra forma): a lo sumo uno puede ser la misma. Decí \"misma\" solo si los documentos de la principal muestran cuál es (el mismo \
identificador, el mismo domicilio, el mismo asunto, las mismas personas); si no se puede saber, \"dudosa\" para todos.\n"
        } else {
            ""
        }
    );
    Question {
        key: classify::question_key(IDENTITY_PROMPT, &prompt),
        version: IDENTITY_PROMPT,
        task: TASK,
        content_id,
        prompt,
        schema: schema(),
    }
}

pub fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["respuestas"],
        "properties": {
            "respuestas": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["candidato", "decision", "certeza", "razon"],
                    "properties": {
                        "candidato": { "type": "integer" },
                        "decision": { "type": "string", "enum": ["misma", "distinta", "dudosa"] },
                        "certeza": { "type": "string", "enum": ["alta", "media", "baja"] },
                        "razon": { "type": "string" }
                    }
                }
            }
        }
    })
}

#[derive(Debug, Deserialize)]
struct Answer {
    #[serde(default)]
    respuestas: Vec<AnswerItem>,
}

#[derive(Debug, Deserialize)]
struct AnswerItem {
    candidato: i64,
    decision: String,
    #[serde(default)]
    certeza: Option<String>,
    #[serde(default)]
    razon: Option<String>,
}

/// The outcome of a resolution, before it is written.
pub(crate) struct Outcome {
    pub rows: Vec<Row>,
    pub clusters: Vec<Cluster>,
    /// Groups of companies (D-260): the group (a group of mentions of it)
    /// and its companies, each with the reasons.
    pub groups: Vec<Group>,
    /// The entity (index of its first group) of each group.
    pub root: Vec<usize>,
    /// What joined each group to its entity ("sola" when alone).
    pub by: Vec<&'static str>,
    pub doubts: BTreeMap<(usize, usize), Doubt>,
}

/// How the model is reached during a resolution.
pub(crate) struct Asking<'a> {
    pub asker: Option<&'a dyn DocumentAsker>,
    pub engine: Option<String>,
    pub budget: usize,
    /// Only collect the questions not answered yet, without asking (the
    /// evaluation asks them a few at a time first).
    pub collect: Option<Vec<Question>>,
}

/// Resolves every group of mentions: the cascade, then the model for the
/// candidates left. `report` counts what was asked and joined.
pub(crate) fn resolve(base: &DocBase, asking: &mut Asking, report: &mut ResolveReport) -> Result<Outcome, String> {
    let vocabulary = vocabulary();
    let (rows, identifiers, hashes, decided, kept_verdicts) = {
        let conn = base.conn();
        (load_rows(&conn)?, load_identifiers(&conn)?, load_hashes(&conn)?, load_decided(&conn)?, load_verdicts(&conn, asking.engine.as_deref())?)
    };
    let mut clusters = clusters(vocabulary, &rows, &identifiers, &hashes);
    for cluster in &mut clusters {
        if cluster.excluded.is_none() && cluster.names.iter().any(|n| decided.wrong.contains(&cluster_key(cluster.content_id, &cluster.kind, n))) {
            cluster.excluded = Some("incorrecta");
        }
    }
    let context = {
        let conn = base.conn();
        context(&conn, &rows, &clusters, identifiers)?
    };
    // The verdicts kept, on the groups they were said of: a group is found
    // by any of the names of its mentions, so a verdict outlives a change
    // of the group's principal name.
    let index = names_index(&clusters);
    let group_of = |key: &str| -> Option<usize> {
        let (content, kind, normalized) = parse_cluster_key(key)?;
        index.get(&(content, kind, normalized)).copied()
    };
    let mut verdicts: HashMap<(usize, usize), Verdict> = HashMap::new();
    for ((a, b), verdict) in kept_verdicts {
        if let (Some(x), Some(y)) = (group_of(&a), group_of(&b)) {
            if x != y {
                verdicts.insert((x.min(y), x.max(y)), verdict);
            }
        }
    }
    let mut resolver = Resolver::new(&clusters);
    resolver.persons(&decided);
    resolver.identifiers();
    resolver.same_names();
    let loose_areas = resolver.areas();

    // 3. The model, for the candidates left, against each principal.
    let mut common = 0;
    let mut formless: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
    let edges = candidates(&mut resolver, &mut common, &mut formless);
    report.common_keys = common;
    let core: Vec<usize> = (0..clusters.len()).map(|i| resolver.find(i)).collect();
    let mut members: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, &root) in core.iter().enumerate() {
        if clusters[i].excluded.is_none() {
            members.entry(root).or_default().push(i);
        }
    }
    let mut partners: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for &(a, b) in edges.keys() {
        partners.entry(a).or_default().push(b);
        partners.entry(b).or_default().push(a);
    }
    let size = |root: usize| members.get(&root).map(|m| m.iter().map(|&c| clusters[c].rows.len()).sum::<usize>()).unwrap_or(0);
    let mut ranking: Vec<usize> = partners.keys().copied().collect();
    ranking.sort_by(|&a, &b| size(b).cmp(&size(a)).then_with(|| a.cmp(&b)));
    let rank: HashMap<usize, usize> = ranking.iter().enumerate().map(|(i, &r)| (r, i)).collect();
    let mut absorbed: HashSet<usize> = HashSet::new();
    let mut stopped = false;
    for &principal in &ranking {
        if absorbed.contains(&principal) {
            continue;
        }
        let mut list: Vec<usize> = partners[&principal].iter().copied().filter(|c| rank[c] > rank[&principal] && !absorbed.contains(c)).collect();
        list.sort_by_key(|c| rank[c]);
        let mut to_ask: Vec<usize> = Vec::new();
        let principal_shown = describe(&rows, &clusters, &members[&principal], &context);
        for candidate in list {
            // What the model already said of this pair, on any of their own
            // groups. A verdict that kept them apart, said when the two
            // sides showed something else, is asked again: the evidence is
            // new (R-08); one that joined them holds.
            let shown = shown_fingerprint(&principal_shown, &describe(&rows, &clusters, &members[&candidate], &context));
            let mut said: Vec<&Verdict> = Vec::new();
            for &x in &members[&principal] {
                for &y in &members[&candidate] {
                    if let Some(v) = verdicts.get(&(x.min(y), x.max(y))) {
                        if v.verdict == "same" || v.evidence.as_deref() == Some(shown.as_str()) {
                            said.push(v);
                        }
                    }
                }
            }
            if said.is_empty() {
                to_ask.push(candidate);
                continue;
            }
            let same = said.iter().filter(|v| v.verdict == "same").count();
            let different = said.iter().filter(|v| v.verdict == "different").count();
            let verdict = if same == said.len() {
                "same"
            } else if different == said.len() {
                "different"
            } else {
                "doubtful"
            };
            let reason = said.iter().find(|v| v.verdict == verdict).and_then(|v| v.reason.clone());
            apply(&mut resolver, &mut absorbed, principal, candidate, verdict, reason, report);
        }
        for batch in to_ask.chunks(MAX_CANDIDATES) {
            if stopped || (asking.asker.is_some() && asking.budget == 0) {
                report.left += 1;
                continue;
            }
            let principal_text = &principal_shown;
            let texts: Vec<String> = batch.iter().map(|c| describe(&rows, &clusters, &members[c], &context)).collect();
            let kind = clusters[principal].kind.clone();
            let question = question(vocabulary, &kind, principal_text, &texts, clusters[principal].content_id, false);
            let Some(said) = ask_question(base, asking, report, &question, batch.len(), &mut stopped)? else { continue };
            for ((&candidate, (verdict, reason)), text) in batch.iter().zip(said).zip(&texts) {
                let shown = shown_fingerprint(principal_text, text);
                keep_verdict(base, &clusters[principal].key, &clusters[candidate].key, &verdict, &reason, &question.key, Some(&shown))?;
                verdicts.insert(
                    (principal.min(candidate), principal.max(candidate)),
                    Verdict { verdict: verdict.to_string(), reason: reason.clone(), evidence: Some(shown) },
                );
                apply(&mut resolver, &mut absorbed, principal, candidate, verdict, reason, report);
            }
        }
    }

    // 3b. A name without a legal form that several companies share (D-212):
    // one question with all of them, the model told that at most one can
    // be the same. Joined only when it names exactly one; else, a doubt
    // against each.
    for (&lone, partners) in &formless {
        let Some(own) = members.get(&lone) else { continue };
        let mut against: Vec<usize> = Vec::new();
        for &partner in partners {
            if resolver.find(partner) != resolver.find(lone) && members.contains_key(&partner) && !against.contains(&partner) {
                against.push(partner);
            }
        }
        against.sort_by_key(|&c| clusters[c].canon.clone());
        if against.is_empty() || absorbed.contains(&lone) {
            continue;
        }
        let mut said: Vec<Option<Verdict>> = Vec::new();
        for &candidate in &against {
            let mut found: Option<Verdict> = None;
            for &x in own {
                for &y in &members[&candidate] {
                    if let Some(v) = verdicts.get(&(x.min(y), x.max(y))) {
                        found = Some(v.clone());
                    }
                }
            }
            said.push(found);
        }
        if said.iter().any(Option::is_none) && against.len() <= MAX_CANDIDATES {
            if stopped || (asking.asker.is_some() && asking.budget == 0) {
                report.left += 1;
            } else {
                let principal_text = describe(&rows, &clusters, own, &context);
                let texts: Vec<String> = against.iter().map(|c| describe(&rows, &clusters, &members[c], &context)).collect();
                let kind = clusters[lone].kind.clone();
                let question = question(vocabulary, &kind, &principal_text, &texts, clusters[lone].content_id, true);
                if let Some(answers) = ask_question(base, asking, report, &question, against.len(), &mut stopped)? {
                    said.clear();
                    for (&candidate, (verdict, reason)) in against.iter().zip(answers) {
                        keep_verdict(base, &clusters[lone].key, &clusters[candidate].key, &verdict, &reason, &question.key, None)?;
                        let v = Verdict { verdict: verdict.to_string(), reason, evidence: None };
                        verdicts.insert((lone.min(candidate), lone.max(candidate)), v.clone());
                        said.push(Some(v));
                    }
                }
            }
        }
        let same: Vec<usize> =
            against.iter().zip(&said).filter(|(_, v)| v.as_ref().is_some_and(|v| v.verdict == "same")).map(|(&c, _)| c).collect();
        let complete = said.iter().all(Option::is_some);
        if complete && same.len() == 1 {
            let reason = said.iter().flatten().find(|v| v.verdict == "same").and_then(|v| v.reason.clone());
            match resolver.join(same[0], lone, "modelo") {
                Ok(_) => continue,
                Err(why) => resolver.doubt(same[0], lone, &format!("modelo-misma-{why}"), reason),
            }
            continue;
        }
        for &candidate in &against {
            resolver.doubt(lone, candidate, "sin-forma-varias-sociedades", None);
        }
    }

    // 4. Groups of companies (D-260).
    let evidence = {
        let conn = base.conn();
        load_links(&conn)?
    };
    let groups = groups(vocabulary, &mut resolver, &rows, &evidence, &loose_areas);

    let root: Vec<usize> = (0..clusters.len()).map(|i| resolver.find(i)).collect();
    let mut by: Vec<&'static str> = vec!["sola"; clusters.len()];
    let rank_of = |by: &str| match by {
        "persona" => 4,
        "identificador" => 3,
        "nombre" | "area" => 2,
        "modelo" | "grupo" => 1,
        _ => 0,
    };
    for &(x, y, how) in &resolver.links {
        for c in [x, y] {
            if rank_of(how) > rank_of(by[c]) {
                by[c] = how;
            }
        }
        match how {
            "persona" => report.by_person += 1,
            "identificador" => report.by_identifier += 1,
            "nombre" | "area" => report.by_name += 1,
            _ => report.by_model += 1,
        }
    }
    // An area that names no organization of its documents ("Cobranzas"
    // alone) is a contact of no one known: not an entity (D-262).
    let mut alone: BTreeSet<usize> = BTreeSet::new();
    for (list, named) in loose_areas.values() {
        if named.is_empty() {
            for &i in list {
                if (0..clusters.len()).filter(|&j| root[j] == root[i]).all(|j| clusters[j].area.is_some() || clusters[j].excluded.is_some()) {
                    alone.insert(i);
                }
            }
        }
    }
    let mut doubts = std::mem::take(&mut resolver.doubts);
    drop(resolver);
    for &i in &alone {
        clusters[i].excluded = Some("casilla");
    }
    doubts.retain(|&(x, y), _| root[x] != root[y] && clusters[x].excluded.is_none() && clusters[y].excluded.is_none());
    Ok(Outcome { rows, clusters, root, by, doubts, groups })
}

/// A group of companies: one of its groups of mentions, and its companies
/// (a group of mentions of each), with the reasons.
#[derive(Debug, Clone)]
pub(crate) struct Group {
    pub group: usize,
    pub members: Vec<(usize, Vec<String>)>,
    /// The shared name of its companies, as one of them writes it (a group
    /// of companies of one name), which names the group (D-260).
    pub name: Option<String>,
}

/// The words of a written name up to its shared words ("Ejemplo Uno S.A."
/// and "ejemplo uno" give "Ejemplo Uno").
fn shared_written(written: &str, shared: &[String]) -> Option<String> {
    let mut taken: Vec<&str> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    for token in written.split_whitespace() {
        let folded = super::quote::words(token);
        if folded.is_empty() {
            continue;
        }
        taken.push(token);
        words.extend(folded);
        if words == shared {
            return Some(taken.join(" ").trim_end_matches([',', '.', ';']).to_string());
        }
        if words.len() >= shared.len() {
            break;
        }
    }
    None
}

/// The relations between two mentions: (subject, predicate, object).
fn load_links(conn: &Connection) -> Result<Vec<(i64, String, i64)>, String> {
    let mut statement = conn
        .prepare("SELECT subject_mention, predicate, object_mention FROM knowledge_evidence WHERE object_mention IS NOT NULL ORDER BY id")
        .map_err(sql_err)?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).map_err(sql_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)
}

/// 4. Groups of companies (D-260). Several organizations with the same
/// name and another legal form are the companies of one group when
/// something else links them: the same person as the representative,
/// partner or employee of two of them (`groupLinks`), or as the attorney in
/// documents of each; the same general mailbox in documents of several; a
/// document that names two of them and says "group" or "affiliate"
/// (`affiliateWords`). What is named without a form and could not be told
/// to be one of them, and that mailbox, are the group's. The shared name
/// alone is not enough (R-07): it stays a doubt for a person. A group the
/// documents name ("Grupo Ejemplo") exists by its quote: its companies are
/// the ones whose name starts with the rest of its name and that appear
/// with it in a document.
fn groups(
    vocabulary: &Vocabulary,
    resolver: &mut Resolver,
    rows: &[Row],
    links: &[(i64, String, i64)],
    loose_areas: &BTreeMap<String, (Vec<usize>, BTreeSet<usize>)>,
) -> Vec<Group> {
    let clusters = resolver.clusters;
    let words = &vocabulary.identity;
    let organization = |c: &Cluster| c.kind == ORGANIZATION && c.excluded.is_none() && c.area.is_none() && !c.name.words.is_empty();
    // The group of mentions of each mention.
    let mut cluster_of: HashMap<i64, usize> = HashMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        for &r in &cluster.rows {
            cluster_of.insert(rows[r].id, i);
        }
    }
    // The organizations of each document, by root.
    let mut in_document: BTreeMap<i64, BTreeSet<usize>> = BTreeMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        if organization(cluster) {
            in_document.entry(cluster.content_id).or_default().insert(resolver.find(i));
        }
    }
    let mut out: Vec<Group> = Vec::new();

    // Companies of the same name with another legal form.
    let mut formed: BTreeMap<Vec<String>, BTreeMap<String, BTreeSet<usize>>> = BTreeMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        if let (true, Some(form)) = (organization(cluster), &cluster.name.form) {
            formed.entry(cluster.name.words.clone()).or_default().entry(form.clone()).or_default().insert(resolver.find(i));
        }
    }
    for (name, by_form) in formed {
        if by_form.len() < 2 {
            continue;
        }
        let members: BTreeSet<usize> = by_form.values().flatten().copied().collect();
        let mut reasons: BTreeMap<usize, BTreeSet<String>> = BTreeMap::new();
        // The same person linked to two of them.
        let mut people: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        for (subject, predicate, object) in links {
            if !words.group_links.iter().any(|p| p == predicate) {
                continue;
            }
            let (Some(&s), Some(&o)) = (cluster_of.get(subject), cluster_of.get(object)) else { continue };
            let (s, o) = (resolver.find(s), resolver.find(o));
            if clusters[s].kind == PERSON && members.contains(&o) {
                people.entry(s).or_default().insert(o);
            }
        }
        // The attorney of a document where only one of them appears.
        for (i, cluster) in clusters.iter().enumerate() {
            if cluster.kind != PERSON || cluster.excluded.is_some() || !cluster.roles.contains("apoderado") {
                continue;
            }
            let theirs: Vec<usize> = in_document.get(&cluster.content_id).into_iter().flatten().copied().filter(|r| members.contains(r)).collect();
            if theirs.len() == 1 {
                let person = resolver.find(i);
                people.entry(person).or_default().insert(theirs[0]);
            }
        }
        for linked in people.values().filter(|l| l.len() >= 2) {
            for &m in linked {
                reasons.entry(m).or_default().insert("mismas-personas".into());
            }
        }
        // The same general mailbox in documents of several.
        for (list, named) in loose_areas.values() {
            let theirs: BTreeSet<usize> = named.iter().map(|&o| resolver.find(o)).filter(|r| members.contains(r)).collect();
            if theirs.len() >= 2 && !list.is_empty() {
                for &m in &theirs {
                    reasons.entry(m).or_default().insert("misma-casilla".into());
                }
            }
        }
        // A document that names two of them and says they are of a group.
        for (content, present) in &in_document {
            let theirs: Vec<usize> = present.iter().copied().filter(|r| members.contains(r)).collect();
            if theirs.len() < 2 {
                continue;
            }
            let says = clusters.iter().filter(|c| c.content_id == *content).flat_map(|c| c.rows.iter()).any(|&r| {
                let quote = super::quote::words(&rows[r].quote).join(" ");
                words.affiliate_words.iter().any(|a| {
                    let a = super::quote::words(a).join(" ");
                    quote.split(' ').any(|w| w.starts_with(a.as_str())) || (a.contains(' ') && quote.contains(a.as_str()))
                })
            });
            if says {
                for &m in &theirs {
                    reasons.entry(m).or_default().insert("lo-dice-un-documento".into());
                }
            }
        }
        if reasons.len() < 2 {
            continue;
        }
        // The group: what is named so without a form and is no company's,
        // and the mailboxes of several of them.
        let lone: Vec<usize> = (0..clusters.len())
            .filter(|&i| organization(&clusters[i]) && clusters[i].name.form.is_none() && clusters[i].name.words == name)
            .filter(|&i| !members.contains(&resolver.find(i)))
            .collect();
        let mut gathered: Vec<usize> = lone.clone();
        for (list, named) in loose_areas.values() {
            let theirs: BTreeSet<usize> = named.iter().map(|&o| resolver.find(o)).collect();
            if !theirs.is_empty() && theirs.iter().all(|r| members.contains(r)) && theirs.len() >= 2 {
                gathered.extend(list.iter().copied());
            }
        }
        let Some(&first) = gathered.first() else { continue };
        for &other in &gathered[1..] {
            let _ = resolver.join(first, other, "grupo");
        }
        let shared = (0..clusters.len())
            .filter(|&i| organization(&clusters[i]) && clusters[i].name.form.is_some() && clusters[i].name.words == name)
            .find_map(|i| shared_written(&rows[clusters[i].principal].written, &name));
        let members: Vec<(usize, Vec<String>)> = members
            .iter()
            .map(|&m| (m, reasons.get(&m).map(|r| r.iter().cloned().collect()).unwrap_or_else(|| vec!["mismo-nombre".into()])))
            .collect();
        out.push(Group { group: first, members, name: shared });
    }

    // A group the documents name.
    let is_group_word = |w: &str| words.group_words.iter().any(|g| g == w);
    for (i, cluster) in clusters.iter().enumerate() {
        if !organization(cluster) || cluster.name.form.is_some() || cluster.name.words.len() < 2 || !is_group_word(&cluster.name.words[0]) {
            continue;
        }
        let group = resolver.find(i);
        if out.iter().any(|g| resolver.find(g.group) == group) {
            continue;
        }
        let inner = &cluster.name.words[1..];
        let mut members: BTreeSet<usize> = BTreeSet::new();
        for (j, other) in clusters.iter().enumerate() {
            if j == i || !organization(other) || other.name.form.is_none() || !other.name.words.starts_with(inner) || other.name.words.len() <= inner.len() {
                continue;
            }
            let root = resolver.find(j);
            if root == group {
                continue;
            }
            let together = clusters
                .iter()
                .enumerate()
                .any(|(g, c)| resolver.sets.parent_of(g) == group && in_document.get(&c.content_id).is_some_and(|present| present.contains(&root)));
            if together {
                members.insert(root);
            }
        }
        if members.is_empty() {
            continue;
        }
        let members: Vec<(usize, Vec<String>)> = members.into_iter().map(|m| (m, vec!["nombrado-como-grupo".to_string()])).collect();
        out.push(Group { group: i, members, name: None });
    }
    out
}

/// Asks a question about identities (or takes its answer kept): the
/// verdict on each candidate, in order, with its reason. None when it was
/// not answered (left for another run, or only collected).
fn ask_question(
    base: &DocBase,
    asking: &mut Asking,
    report: &mut ResolveReport,
    question: &Question,
    count: usize,
    stopped: &mut bool,
) -> Result<Option<Vec<(&'static str, Option<String>)>>, String> {
    if let Some(collected) = asking.collect.as_mut() {
        if cached_answer(&base.conn(), &question.key, asking.engine.as_deref())?.is_none() {
            collected.push(question.clone());
            report.left += 1;
            return Ok(None);
        }
    }
    let answer = match asking.asker {
        Some(asker) => {
            let cached = cached_answer(&base.conn(), &question.key, Some(&asker.engine_id()))?;
            if cached.is_none() {
                asking.budget = asking.budget.saturating_sub(1);
                report.asked += 1;
            }
            match ask_and_keep(base, asker, question) {
                Ok(answer) => Some(answer),
                Err(error) => {
                    report.failed += 1;
                    report.left += 1;
                    *stopped = stops_the_run(&error);
                    report.errors.push(error);
                    None
                }
            }
        }
        None => cached_answer(&base.conn(), &question.key, asking.engine.as_deref())?,
    };
    let Some(answer) = answer else {
        if asking.asker.is_none() {
            report.left += 1;
        }
        return Ok(None);
    };
    let parsed: Option<Answer> = parse_json(&answer).ok();
    let mut out = Vec::new();
    for index in 0..count {
        let item = parsed.as_ref().and_then(|a| a.respuestas.iter().find(|r| r.candidato == index as i64 + 1));
        out.push(match item.map(|i| i.decision.as_str()) {
            // The certainty is kept with the reason: measured, a "the
            // same" with medium certainty was right as often.
            Some("misma") => (
                "same",
                item.map(|i| format!("certeza {}: {}", i.certeza.as_deref().unwrap_or("-"), i.razon.as_deref().unwrap_or_default())),
            ),
            Some("distinta") => ("different", item.and_then(|i| i.razon.clone())),
            Some(_) => ("doubtful", item.and_then(|i| i.razon.clone())),
            None => ("doubtful", Some("el modelo no respondió por este candidato".to_string())),
        });
    }
    Ok(Some(out))
}

/// Keeps what the model said of a pair of groups, for every later
/// resolution (R-20).
fn keep_verdict(base: &DocBase, x: &str, y: &str, verdict: &str, reason: &Option<String>, question: &str, evidence: Option<&str>) -> Result<(), String> {
    let engine = base
        .conn()
        .query_row("SELECT engine FROM model_answer WHERE question = ?1 ORDER BY asked_at DESC LIMIT 1", [question], |row| row.get::<_, String>(0))
        .unwrap_or_default();
    let (a, b) = pair(x, y);
    base.conn()
        .execute(
            "INSERT OR REPLACE INTO knowledge_verdict (a, b, engine, verdict, reason, question, asked_at, evidence)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![a, b, engine, verdict, reason, question, now_ms(), evidence],
        )
        .map_err(sql_err)?;
    Ok(())
}

/// Applies what the model said of a candidate: the same joins it to the
/// principal (unless something keeps them apart); doubtful is a doubt.
fn apply(
    resolver: &mut Resolver,
    absorbed: &mut HashSet<usize>,
    principal: usize,
    candidate: usize,
    verdict: &str,
    reason: Option<String>,
    _report: &mut ResolveReport,
) {
    match verdict {
        "same" => match resolver.join(principal, candidate, "modelo") {
            Ok(_) => {
                absorbed.insert(candidate);
            }
            Err(why) => resolver.doubt(principal, candidate, &format!("modelo-misma-{why}"), reason),
        },
        "doubtful" => resolver.doubt(principal, candidate, "modelo-dudosa", reason),
        _ => {}
    }
}

/// An existing entity, as the writing finds it.
struct Existing {
    kind: String,
    name: String,
    state: String,
    merged_into: Option<String>,
    created_at: i64,
}

fn entity_id(kind: &str, canon: &str, taken: &HashSet<String>, existing: &HashMap<String, Existing>) -> String {
    let base = format!("ent-{}", hash(&[kind, canon]));
    let mut id = base.clone();
    let mut n = 2;
    while taken.contains(&id) || existing.get(&id).is_some_and(|e| e.state == "active") {
        id = format!("{base}-{n}");
        n += 1;
    }
    id
}

/// Writes the outcome: entities (an id is kept by the entity that keeps
/// most of its mentions), aliases, the entity of every mention, the
/// doubts and the claims; marks every entity that changed for its page.
pub(crate) fn write(base: &DocBase, outcome: &Outcome, report: &mut ResolveReport) -> Result<(), String> {
    let vocabulary = vocabulary();
    let mut conn = base.conn();
    let decided = load_decided(&conn)?;
    let rows = &outcome.rows;
    let clusters = &outcome.clusters;
    // The mentions are the ones resolved: the reading cannot change while
    // the resolution runs (both take the lock of the questions).
    let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (i, cluster) in clusters.iter().enumerate() {
        if cluster.excluded.is_none() {
            groups.entry(outcome.root[i]).or_default().push(i);
        }
    }
    let mention_ids = |list: &[usize]| -> BTreeSet<i64> {
        list.iter().flat_map(|&c| clusters[c].rows.iter().map(|&r| rows[r].id)).collect()
    };
    let existing: HashMap<String, Existing> = {
        let mut statement = conn.prepare("SELECT id, kind, name, state, merged_into, created_at FROM knowledge_entity").map_err(sql_err)?;
        let found = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    Existing { kind: row.get(1)?, name: row.get(2)?, state: row.get(3)?, merged_into: row.get(4)?, created_at: row.get(5)? },
                ))
            })
            .map_err(sql_err)?;
        found.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut before: HashMap<String, BTreeSet<i64>> = HashMap::new();
    for row in rows.iter() {
        if let Some(entity) = &row.entity {
            before.entry(entity.clone()).or_default().insert(row.id);
        }
    }
    // Who keeps which id: the biggest entities choose first.
    let mut order: Vec<(usize, Vec<usize>)> = groups.into_iter().collect();
    order.sort_by(|a, b| {
        let size = |g: &Vec<usize>| g.iter().map(|&c| clusters[c].rows.len()).sum::<usize>();
        size(&b.1).cmp(&size(&a.1)).then_with(|| clusters[a.1[0]].canon.cmp(&clusters[b.1[0]].canon))
    });
    let mut taken: HashSet<String> = HashSet::new();
    let mut assigned: Vec<(String, Vec<usize>)> = Vec::new();
    for (_, list) in order {
        let mut overlap: BTreeMap<String, usize> = BTreeMap::new();
        for &c in &list {
            for &r in &clusters[c].rows {
                // The entity it has, or the one its name had before the
                // document was read again or took another one's place.
                if let Some(entity) = rows[r].entity.as_ref().or(rows[r].hint.as_ref()) {
                    if !taken.contains(entity) && existing.get(entity).is_some_and(|e| e.kind == clusters[c].kind) {
                        *overlap.entry(entity.clone()).or_default() += 1;
                    }
                }
            }
        }
        let best = overlap
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| existing[&b.0].created_at.cmp(&existing[&a.0].created_at)).then_with(|| b.0.cmp(&a.0)))
            .map(|(id, _)| id);
        let id = best.unwrap_or_else(|| entity_id(&clusters[list[0]].kind, &clusters[list[0]].canon, &taken, &existing));
        taken.insert(id.clone());
        assigned.push((id, list));
    }
    let now = now_ms();
    let tx = conn.transaction().map_err(sql_err)?;
    let mut changed: BTreeSet<String> = BTreeSet::new();
    let mut entity_of_row: HashMap<i64, (String, &'static str)> = HashMap::new();
    for (id, list) in &assigned {
        let kind = &clusters[list[0]].kind;
        // Aliases: each normalized name with its most used way of writing.
        let mut aliases: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
        let mut proper: BTreeSet<&str> = BTreeSet::new();
        // An area or a mailbox never names the organization it is of, nor
        // its group (D-262), unless there is nothing else.
        let named_by_area = list.iter().all(|&c| clusters[c].area.is_some());
        for &c in list {
            if named_by_area || clusters[c].area.is_none() {
                proper.insert(rows[clusters[c].principal].normalized.as_str());
            }
            for &r in &clusters[c].rows {
                *aliases.entry(rows[r].normalized.as_str()).or_default().entry(rows[r].written.as_str()).or_default() += 1;
                entity_of_row.insert(rows[r].id, (id.clone(), outcome.by[c]));
            }
        }
        let count = |n: &str| aliases[n].values().sum::<usize>();
        let written_of = |n: &str| -> String { most_written(&aliases[n]) };
        // A group of companies of one name is named by that name (D-260).
        let group_name = outcome
            .groups
            .iter()
            .filter(|g| list.iter().any(|&c| outcome.root[c] == outcome.root[g.group]))
            .find_map(|g| g.name.clone());
        // By specificity: the most complete form that is a proper name,
        // not the most repeated one (D-259).
        let shown: Vec<(String, String, usize)> = proper.iter().map(|n| (n.to_string(), written_of(n), count(n))).collect();
        let candidates: Vec<(&str, &str, usize)> = shown.iter().map(|(n, w, c)| (n.as_str(), w.as_str(), *c)).collect();
        let forms: Vec<(&str, usize)> = aliases.values().flat_map(|forms| forms.iter().map(|(w, c)| (*w, *c))).collect();
        let chosen = names::most_specific_with(vocabulary, kind, &candidates, &forms).unwrap_or_default().to_string();
        let chosen = chosen.as_str();
        // On one line, and in the form of a proper name when the documents
        // write it some other way than in capitals (D-239).
        let name = super::names::shown_name(&group_name.unwrap_or_else(|| written_of(chosen)), aliases.values().flat_map(|forms| forms.keys().copied()));
        let name = if kind == names::PERSON { names::person_case(&name) } else { name };
        let now_members = mention_ids(list);
        match existing.get(id) {
            Some(old) => {
                if old.name != name || old.state != "active" || old.merged_into.is_some() || before.get(id) != Some(&now_members) {
                    tx.execute(
                        "UPDATE knowledge_entity SET name = ?2, state = 'active', merged_into = NULL, updated_at = ?3 WHERE id = ?1",
                        params![id, name, now],
                    )
                    .map_err(sql_err)?;
                    changed.insert(id.clone());
                }
            }
            None => {
                tx.execute(
                    "INSERT INTO knowledge_entity (id, kind, name, state, created_at, updated_at) VALUES (?1, ?2, ?3, 'active', ?4, ?4)",
                    params![id, kind, name, now],
                )
                .map_err(sql_err)?;
                changed.insert(id.clone());
            }
        }
        if changed.contains(id) {
            tx.execute("DELETE FROM knowledge_alias WHERE entity_id = ?1", [id]).map_err(sql_err)?;
            for normalized in aliases.keys() {
                tx.execute(
                    "INSERT INTO knowledge_alias (entity_id, normalized, written, mentions) VALUES (?1, ?2, ?3, ?4)",
                    params![id, normalized, written_of(normalized), count(normalized) as i64],
                )
                .map_err(sql_err)?;
            }
        }
    }
    // Entities no group kept: merged into the one that took their
    // mentions, or retired (kept, with the decisions about them).
    let kept: HashSet<&String> = assigned.iter().map(|(id, _)| id).collect();
    for (id, old) in &existing {
        if kept.contains(id) {
            continue;
        }
        if before.get(id).is_none() {
            // No mention left at all (its documents went away): retired.
            if old.state == "active" {
                tx.execute("UPDATE knowledge_entity SET state = 'retired', updated_at = ?2 WHERE id = ?1", params![id, now])
                    .map_err(sql_err)?;
                changed.insert(id.clone());
            }
            continue;
        }
        let mut into: BTreeMap<&String, usize> = BTreeMap::new();
        for mention in before.get(id).into_iter().flatten() {
            if let Some((new, _)) = entity_of_row.get(mention) {
                *into.entry(new).or_default() += 1;
            }
        }
        let target = into.into_iter().max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0))).map(|(id, _)| id.clone());
        let state = if target.is_some() { "merged" } else { "retired" };
        if old.state != state || old.merged_into != target {
            tx.execute(
                "UPDATE knowledge_entity SET state = ?2, merged_into = ?3, updated_at = ?4 WHERE id = ?1",
                params![id, state, target, now],
            )
            .map_err(sql_err)?;
            changed.insert(id.clone());
        }
    }
    // The entity of every mention.
    for cluster in clusters.iter() {
        for &r in &cluster.rows {
            let row = &rows[r];
            let (entity, by) = match cluster.excluded {
                Some(why) => (None, Some(why)),
                None => {
                    let (id, by) = &entity_of_row[&row.id];
                    (Some(id.clone()), Some(*by))
                }
            };
            tx.execute(
                "UPDATE knowledge_mention SET entity_id = ?2, resolved_by = ?3
                 WHERE id = ?1 AND (entity_id IS NOT ?2 OR resolved_by IS NOT ?3)",
                params![row.id, entity, by],
            )
            .map_err(sql_err)?;
        }
    }
    // Doubts, computed again.
    tx.execute("DELETE FROM knowledge_doubt", []).map_err(sql_err)?;
    for (&(x, y), doubt) in &outcome.doubts {
        let entity = |c: usize| entity_of_row.get(&rows[clusters[c].principal].id).map(|(id, _)| id.clone());
        let evidence = hash(&[&clusters[x].key, &clusters[y].key, &doubt.reason, doubt.detail.as_deref().unwrap_or("")]);
        let (a, b) = if clusters[x].key <= clusters[y].key { (x, y) } else { (y, x) };
        tx.execute(
            "INSERT OR REPLACE INTO knowledge_doubt (a, b, kind, entity_a, entity_b, reason, detail, evidence_hash, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![clusters[a].key, clusters[b].key, clusters[a].kind, entity(a), entity(b), doubt.reason, doubt.detail, evidence, now],
        )
        .map_err(sql_err)?;
    }
    // Groups of companies (D-260), written again; a change marks the
    // pages of the group and of its companies.
    let mut before_groups: BTreeSet<(String, String, String)> = BTreeSet::new();
    {
        let mut statement = tx.prepare("SELECT group_id, member_id, reasons FROM knowledge_group").map_err(sql_err)?;
        let found = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).map_err(sql_err)?;
        for item in found {
            before_groups.insert(item.map_err(sql_err)?);
        }
    }
    let mut now_groups: BTreeSet<(String, String, String)> = BTreeSet::new();
    for group in &outcome.groups {
        let entity = |c: usize| entity_of_row.get(&rows[clusters[c].principal].id).map(|(id, _)| id.clone());
        let Some(group_id) = entity(group.group) else { continue };
        for (member, reasons) in &group.members {
            if let Some(member_id) = entity(*member).filter(|m| *m != group_id) {
                now_groups.insert((group_id.clone(), member_id, serde_json::to_string(reasons).unwrap_or_else(|_| "[]".into())));
            }
        }
    }
    if before_groups != now_groups {
        tx.execute("DELETE FROM knowledge_group", []).map_err(sql_err)?;
        for (group, member, reasons) in &now_groups {
            tx.execute("INSERT INTO knowledge_group (group_id, member_id, reasons) VALUES (?1, ?2, ?3)", params![group, member, reasons])
                .map_err(sql_err)?;
        }
        for (group, member, _) in before_groups.symmetric_difference(&now_groups) {
            changed.insert(group.clone());
            changed.insert(member.clone());
        }
    }
    report.doubts = outcome.doubts.len();
    report.groups = clusters.iter().filter(|c| c.excluded.is_none()).count();
    report.entities = assigned.len();
    // The documents left out: their mentions out of the record.
    let left_out = changes::left_out(&tx)?;
    changed.extend(changes::leave_out(&tx, &left_out)?);
    // Claims: the evidence in terms of entities.
    let consolidated = claims::consolidate(&tx, vocabulary, &decided)?;
    // What each decision of a person applies to now, and those left for a
    // person to review (never dropped in silence).
    super::anchors::keep(&tx, &decided.anchored, &consolidated.placed)?;
    report.claims = consolidated.claims;
    report.conflicts = consolidated.conflicts;
    report.unfit = consolidated.unfit;
    changed.extend(consolidated.touched);
    // Matters: the documents grouped by what is specific to one matter.
    let grouped = super::matters::group(&tx, vocabulary, &decided.joins, &decided.separations)?;
    report.matters = grouped.matters;
    report.matter_documents = grouped.documents;
    for id in &changed {
        super::mark_dirty(&tx, "project", id)?;
    }
    report.changed = changed.len();
    changes::forget_used_hints(&tx)?;
    tx.commit().map_err(sql_err)
}

impl DocBase {
    /// The questions a resolution would ask now (for the evaluation, which
    /// warms its cache with them first). Writes nothing.
    #[cfg(test)]
    pub(crate) fn identity_questions(&self, engine: &str) -> Result<Vec<Question>, String> {
        let mut report = ResolveReport::default();
        let mut asking = Asking { asker: None, engine: Some(engine.to_string()), budget: 0, collect: Some(Vec::new()) };
        resolve(self, &mut asking, &mut report)?;
        Ok(asking.collect.unwrap_or_default())
    }

    /// Resolves the identities of the mentions read: the cascade without a
    /// model, the verdicts kept, and up to `max_questions` new questions
    /// to `asker` (none without one). Does nothing when no reading or
    /// decision changed since the last complete resolution.
    pub fn knowledge_resolve(&self, asker: Option<&dyn DocumentAsker>, max_questions: usize) -> Result<ResolveReport, String> {
        let _one = RESOLVING.lock().unwrap_or_else(|p| p.into_inner());
        let _asking = asker.map(|_| ASKING.lock().unwrap_or_else(|p| p.into_inner()));
        // A change in what is left out is work to do, marked before the
        // resolution starts so that finishing it clears the mark.
        changes::notice_left_out(&self.conn())?;
        let started = now_ms();
        let dirty: i64 = self
            .conn()
            .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')", [], |row| row.get(0))
            .map_err(sql_err)?;
        // Other rules (a new version of the program's or of the
        // vocabulary's) are work to do too: the record is computed again
        // from the readings and the verdicts kept, without asking more.
        let basis = resolution_basis();
        let current = self
            .conn()
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [RESOLUTION_SETTING], |row| row.get::<_, String>(0))
            .ok();
        if dirty == 0 && current.as_deref() == Some(basis.as_str()) {
            return Ok(ResolveReport { skipped: true, ..Default::default() });
        }
        let any_reading: i64 = self
            .conn()
            .query_row("SELECT count(*) FROM knowledge_extraction", [], |row| row.get(0))
            .map_err(sql_err)?;
        if dirty == 0 && any_reading == 0 {
            return Ok(ResolveReport { skipped: true, ..Default::default() });
        }
        let engine = match asker {
            Some(asker) => Some(asker.engine_id()),
            None => classify::ask_engine(&self.conn())?.map(|c| crate::source_volume::asking::engine_id(&c)),
        };
        let mut report = ResolveReport::default();
        let mut asking = Asking { asker, engine, budget: max_questions, collect: None };
        let outcome = resolve(self, &mut asking, &mut report)?;
        write(self, &outcome, &mut report)?;
        if report.left == 0 {
            self.conn()
                .execute("DELETE FROM knowledge_dirty WHERE scope IN ('resolve', 'retire') AND since <= ?1", [started])
                .map_err(sql_err)?;
            self.conn()
                .execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![RESOLUTION_SETTING, basis])
                .map_err(sql_err)?;
        }
        Ok(report)
    }
}
