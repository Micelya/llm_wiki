//! Stage 5 of the knowledge layer: which documents belong to the same
//! matter (D-213).
//!
//! A matter of the archive (a claim and its letters, a contract and its
//! payments) is more than the matters the documents name: a reply to a
//! letter document, the invoices it claims and the e-mails about them are
//! the same matter even when none of them names it the same way. A matter
//! here is a **group of documents** joined by evidence that is specific to
//! one matter (R-05: sharing a person or a company is not sharing a
//! matter):
//!
//! 1. what a person decided ("these two documents are the same matter",
//!    or "they are not": a separation is never crossed);
//! 2. two documents a person joined with a relation that keeps them in one
//!    matter (the same document, a version, a reply, a delivery receipt:
//!    `matters.relations` in the vocabulary); only what a person
//!    confirmed, never a proposal (R-03);
//! 3. the same matter named by both, once the identities are resolved (an
//!    expediente, a contract identified by the model);
//! 4. the same specific number in both: on a matter they name (an invoice,
//!    a letter, a project, a file), or in a datum of their type that is an
//!    identifier (the invoices a letter claims, the number of a shipment).
//!    Numbers are compared by their digits; a number that identifies a
//!    person or a company (a DNI, a CUIT) is not a matter's, and a number
//!    too common in the archive does not count (R-16);
//! 5. the same amount in both, with its decimals or thousands (an amount
//!    claimed, the total of an invoice); not a round one.
//!
//! The links are applied from the surest to the least sure (R-11), in an
//! order that depends only on the texts (R-13), and none joins two groups
//! a person kept apart. A group of two documents or more is a matter, with
//! a stable id (the one of the matter that keeps most of its documents)
//! and the reasons each document is in it, so a person can see why.
//!
//! A document that no evidence puts in a matter, but shares parties with
//! one, is read by the model against those matters (one question per
//! document, with abstention): what it says is a **proposal** for a
//! person, shown as such in the pages, never a link that groups other
//! documents (R-03, `knowledge_propose_matters`).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::changes;
use super::names;
use super::quote;
use super::values;
use super::vocabulary::Vocabulary;
use crate::docbase::similar::classify::Question;
use crate::docbase::{now_ms, sql_err};
use crate::source_volume::asking::DocumentAsker;

/// What a grouping left.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Grouped {
    /// Matters (groups of two documents or more), and the documents in them.
    pub matters: usize,
    pub documents: usize,
    /// Links by what made them.
    pub by_reason: BTreeMap<String, usize>,
    /// Links a person's separation did not let through.
    pub kept_apart: usize,
    /// Matters whose documents or name changed.
    pub changed: usize,
}

/// A reason a document is in its matter, as the pages show it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reason {
    /// "persona", "documento", "asunto", "numero" or "importe".
    pub kind: String,
    /// The relation, the matter's entity, the number or the amount.
    pub value: String,
    /// The other document of the link.
    pub with: i64,
}

fn hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hasher.finalize().iter().take(8).map(|b| format!("{b:02x}")).collect()
}

struct Sets {
    parent: Vec<usize>,
}

impl Sets {
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
}

/// A link between two documents, and its strength (lower is surer).
struct Link {
    strength: u8,
    a: usize,
    b: usize,
    reason: (String, String),
}

/// The documents read by the layer that count (not left out), with the
/// fingerprint of their text, in an order that depends only on the texts.
fn documents(conn: &Connection) -> Result<Vec<(i64, String)>, String> {
    let mut statement = conn
        .prepare(&format!(
            "SELECT k.content_id, k.text_hash FROM knowledge_extraction k WHERE k.state = 'done' AND {}",
            changes::active_content("k.content_id")
        ))
        .map_err(sql_err)?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
    let mut out: Vec<(i64, String)> = rows.collect::<Result<_, _>>().map_err(sql_err)?;
    out.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    Ok(out)
}

/// The digits of a number, which is how two writings of it compare
/// ("A00121-00028550" and "Factura 00121-00028550").
fn digits_of(value: &str) -> String {
    value.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// How an amount is compared: its runs of digits, when it has decimals or
/// thousands and digits enough. A round amount ("10.000", "1.500,00") is
/// in many matters: it says nothing.
fn amount_key(rules: &super::vocabulary::MatterRules, value: &str) -> Option<String> {
    let runs = values::digit_runs(value);
    let digits: usize = runs.iter().map(|r| r.len()).sum();
    let round = runs.last().is_some_and(|last| last.chars().all(|c| c == '0'));
    (runs.len() >= 2 && digits >= rules.amount_digits && !round).then(|| runs.join("."))
}

/// A key a person decides matters by: "content:<id>".
pub(crate) fn content_key(content: i64) -> String {
    format!("content:{content}")
}

fn content_of(key: &str) -> Option<i64> {
    key.strip_prefix("content:")?.parse().ok()
}

/// Groups the documents into matters and writes what changed.
pub(crate) fn group(conn: &Connection, vocabulary: &Vocabulary, joins: &[(String, String)], separations: &[(String, String)]) -> Result<Grouped, String> {
    let rules = &vocabulary.matters;
    let docs = documents(conn)?;
    let index: HashMap<i64, usize> = docs.iter().enumerate().map(|(i, (c, _))| (*c, i)).collect();
    let mut links: Vec<Link> = Vec::new();
    let mut out = Grouped::default();

    // 1. What a person decided.
    let successors = changes::successors(conn)?;
    let at = |key: &str| -> Option<usize> {
        let key = changes::translate(key, &successors);
        content_of(&key).and_then(|c| index.get(&c).copied())
    };
    for (a, b) in joins {
        if let (Some(x), Some(y)) = (at(a), at(b)) {
            links.push(Link { strength: 0, a: x, b: y, reason: ("persona".into(), String::new()) });
        }
    }
    let apart: Vec<(usize, usize)> = separations.iter().filter_map(|(a, b)| Some((at(a)?, at(b)?))).collect();

    // 2. Documents a person joined with a relation of one matter.
    {
        let mut statement = conn
            .prepare("SELECT a, b, coalesce(chosen_relation, relation) FROM similar_pair WHERE state = 'joined'")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (a, b, relation) = row.map_err(sql_err)?;
            if !rules.relations.iter().any(|r| r == &relation) {
                continue;
            }
            if let (Some(&x), Some(&y)) = (index.get(&a), index.get(&b)) {
                links.push(Link { strength: 1, a: x, b: y, reason: ("documento".into(), relation) });
            }
        }
    }

    // 3 to 5: the same matter, number or amount in several documents, each
    // linked to the first of them (by the order of the texts).
    let mut shared: BTreeMap<(u8, String, String), BTreeSet<usize>> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare("SELECT content_id, entity_id FROM knowledge_mention WHERE kind = ?1 AND entity_id IS NOT NULL")
            .map_err(sql_err)?;
        let rows = statement.query_map([&rules.kind], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (content, entity) = row.map_err(sql_err)?;
            if let Some(&i) = index.get(&content) {
                shared.entry((2, "asunto".into(), entity)).or_default().insert(i);
            }
        }
    }
    // The numbers of the parties (a DNI, a CUIT): never a matter's.
    let parties: BTreeSet<String> = {
        let mut statement = conn
            .prepare("SELECT i.kind, i.value FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id WHERE m.kind != ?1")
            .map_err(sql_err)?;
        let rows = statement.query_map([&rules.kind], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        let mut out = BTreeSet::new();
        for row in rows {
            let (kind, value) = row.map_err(sql_err)?;
            if vocabulary.is_strong(&kind) {
                out.insert(digits_of(&value));
            }
        }
        out
    };
    let number = |value: &str| -> Option<String> {
        let digits = digits_of(value);
        let long = names::identifier_value(value).chars().count() >= rules.min_length;
        (long && digits.len() >= rules.min_digits && !parties.contains(&digits)).then_some(digits)
    };
    {
        let mut statement = conn
            .prepare(
                "SELECT m.content_id, i.kind, i.value FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id
                 WHERE m.kind = ?1",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([&rules.kind], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, kind, value) = row.map_err(sql_err)?;
            if !rules.identifier_kinds.iter().any(|k| k == &kind) {
                continue;
            }
            if let (Some(digits), Some(&i)) = (number(&value), index.get(&content)) {
                shared.entry((3, "numero".into(), digits)).or_default().insert(i);
            }
        }
    }
    // The data of each document by its type: identifiers and amounts.
    {
        let catalog = crate::docbase::similar::types::catalog();
        let mut statement = conn.prepare("SELECT content_id, doc_type, data FROM document_data").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, doc_type, data) = row.map_err(sql_err)?;
            let Some(&i) = index.get(&content) else { continue };
            let Some(definition) = catalog.types.iter().find(|t| t.id == doc_type) else { continue };
            let items: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
            for item in items {
                let (Some(field), Some(value)) = (item["field"].as_str(), item["value"].as_str()) else { continue };
                match definition.data.iter().find(|d| d.id == field).map(|d| d.kind.as_str()) {
                    Some("identifier") => {
                        if let Some(digits) = number(value) {
                            shared.entry((3, "numero".into(), digits)).or_default().insert(i);
                        }
                    }
                    Some("amount") => {
                        if let Some(key) = amount_key(rules, value) {
                            shared.entry((4, "importe".into(), key)).or_default().insert(i);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    {
        let mut statement = conn
            .prepare(
                "SELECT e.content_id, e.predicate, e.object_value, s.normalized FROM knowledge_evidence e
                 JOIN knowledge_mention s ON s.id = e.subject_mention WHERE e.object_value IS NOT NULL",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, predicate, value, subject) = row.map_err(sql_err)?;
            if vocabulary.predicate(&predicate).is_none_or(|p| p.shape != "amount") || !values::fits(vocabulary, "amount", &value, &subject) {
                continue;
            }
            if let (Some(key), Some(&i)) = (amount_key(rules, &value), index.get(&content)) {
                shared.entry((4, "importe".into(), key)).or_default().insert(i);
            }
        }
    }
    let cap = ((docs.len() as f64) * rules.max_share).ceil().max(2.0) as usize;
    for ((strength, kind, value), list) in &shared {
        if list.len() < 2 {
            continue;
        }
        if list.len() > cap {
            *out.by_reason.entry(format!("{kind}-comun")).or_default() += 1;
            continue;
        }
        let first = *list.iter().next().expect("two or more");
        for &other in list.iter().skip(1) {
            links.push(Link { strength: *strength, a: first, b: other, reason: (kind.clone(), value.clone()) });
        }
    }
    links.sort_by(|x, y| {
        x.strength
            .cmp(&y.strength)
            .then_with(|| x.a.min(x.b).cmp(&y.a.min(y.b)))
            .then_with(|| x.a.max(x.b).cmp(&y.a.max(y.b)))
            .then_with(|| x.reason.cmp(&y.reason))
    });

    // The links, from the surest, never across a separation.
    let mut sets = Sets { parent: (0..docs.len()).collect() };
    let mut members: Vec<BTreeSet<usize>> = (0..docs.len()).map(|i| BTreeSet::from([i])).collect();
    let mut reasons: Vec<BTreeSet<Reason>> = vec![BTreeSet::new(); docs.len()];
    for link in &links {
        let (ra, rb) = (sets.find(link.a), sets.find(link.b));
        let reason = |with: usize| Reason { kind: link.reason.0.clone(), value: link.reason.1.clone(), with: docs[with].0 };
        if ra != rb {
            let crosses = apart.iter().any(|&(x, y)| {
                (members[ra].contains(&x) && members[rb].contains(&y)) || (members[ra].contains(&y) && members[rb].contains(&x))
            });
            if crosses {
                out.kept_apart += 1;
                continue;
            }
            let (keep, gone) = if ra <= rb { (ra, rb) } else { (rb, ra) };
            sets.parent[gone] = keep;
            let moved = std::mem::take(&mut members[gone]);
            members[keep].extend(moved);
            *out.by_reason.entry(link.reason.0.clone()).or_default() += 1;
        }
        // The reason is kept for both documents, also when they already
        // were in the same matter: it is evidence all the same.
        if sets.find(link.a) == sets.find(link.b) {
            reasons[link.a].insert(reason(link.b));
            reasons[link.b].insert(reason(link.a));
        }
    }
    let groups: Vec<BTreeSet<usize>> = members.into_iter().filter(|m| m.len() >= 2).collect();

    // Who keeps which id: the biggest groups choose first, by overlap.
    let existing: HashMap<String, (String, BTreeSet<i64>)> = {
        let mut out: HashMap<String, (String, BTreeSet<i64>)> = HashMap::new();
        let mut statement = conn.prepare("SELECT id, state FROM knowledge_matter").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (id, state) = row.map_err(sql_err)?;
            out.insert(id, (state, BTreeSet::new()));
        }
        let mut statement = conn.prepare("SELECT matter_id, content_id FROM knowledge_matter_document").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (id, content) = row.map_err(sql_err)?;
            if let Some(entry) = out.get_mut(&id) {
                entry.1.insert(content);
            }
        }
        out
    };
    let mut order: Vec<&BTreeSet<usize>> = groups.iter().collect();
    order.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.iter().next().cmp(&b.iter().next())));
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let mut assigned: Vec<(String, &BTreeSet<usize>)> = Vec::new();
    for group in order {
        let contents: BTreeSet<i64> = group.iter().map(|&i| docs[i].0).collect();
        let best = existing
            .iter()
            .filter(|(id, _)| !taken.contains(*id))
            .map(|(id, (_, before))| (id, before.intersection(&contents).count()))
            .filter(|(_, n)| *n > 0)
            .max_by(|a, b| a.1.cmp(&b.1).then_with(|| b.0.cmp(a.0)))
            .map(|(id, _)| id.clone());
        let id = best.unwrap_or_else(|| {
            let first = &docs[*group.iter().next().expect("two or more")].1;
            let mut id = format!("mat-{}", hash(&[first]));
            let mut n = 2;
            while taken.contains(&id) || existing.contains_key(&id) && existing[&id].0 == "active" {
                id = format!("mat-{}-{n}", hash(&[first]));
                n += 1;
            }
            id
        });
        taken.insert(id.clone());
        assigned.push((id, group));
    }

    // The name of each matter: the matter its documents name most, with
    // its parties.
    let now = now_ms();
    for (id, group) in &assigned {
        let contents: Vec<i64> = group.iter().map(|&i| docs[i].0).collect();
        let name = matter_name(conn, vocabulary, &contents)?;
        let before = existing.get(id);
        let same_docs = before.is_some_and(|(state, b)| state == "active" && *b == contents.iter().copied().collect());
        let stored_name: Option<String> = conn.query_row("SELECT name FROM knowledge_matter WHERE id = ?1", [id], |row| row.get(0)).ok();
        let stored_reasons: BTreeMap<i64, String> = {
            let mut statement = conn.prepare("SELECT content_id, reasons FROM knowledge_matter_document WHERE matter_id = ?1").map_err(sql_err)?;
            let rows = statement.query_map([id], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let now_reasons: BTreeMap<i64, String> = group
            .iter()
            .map(|&i| (docs[i].0, serde_json::to_string(&reasons[i].iter().collect::<Vec<_>>()).unwrap_or_else(|_| "[]".into())))
            .collect();
        if same_docs && stored_name.as_deref() == Some(name.as_str()) && stored_reasons == now_reasons {
            continue;
        }
        conn.execute(
            "INSERT INTO knowledge_matter (id, name, state, created_at, updated_at) VALUES (?1, ?2, 'active', ?3, ?3)
             ON CONFLICT (id) DO UPDATE SET name = excluded.name, state = 'active', updated_at = excluded.updated_at",
            params![id, name, now],
        )
        .map_err(sql_err)?;
        conn.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1", [id]).map_err(sql_err)?;
        for (content, reasons) in &now_reasons {
            conn.execute(
                "INSERT OR REPLACE INTO knowledge_matter_document (matter_id, content_id, state, reasons) VALUES (?1, ?2, 'firm', ?3)",
                params![id, content, reasons],
            )
            .map_err(sql_err)?;
        }
        out.changed += 1;
    }
    let kept: BTreeSet<&String> = assigned.iter().map(|(id, _)| id).collect();
    for (id, (state, _)) in &existing {
        if !kept.contains(id) && state == "active" {
            conn.execute("UPDATE knowledge_matter SET state = 'retired', updated_at = ?2 WHERE id = ?1", params![id, now]).map_err(sql_err)?;
            conn.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1", [id]).map_err(sql_err)?;
            out.changed += 1;
        }
    }
    out.matters = assigned.len();
    out.documents = assigned.iter().map(|(_, g)| g.len()).sum();
    if out.changed > 0 {
        super::mark_dirty(conn, "project", "matters")?;
    }
    Ok(out)
}

/// The name of a matter: the matter its documents name in most documents
/// (as the document's subject first), and its parties; with no matter
/// named, its parties alone.
fn matter_name(conn: &Connection, vocabulary: &Vocabulary, contents: &[i64]) -> Result<String, String> {
    let rules = &vocabulary.matters;
    let list = contents.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
    let mut named: BTreeMap<String, (usize, usize, String)> = BTreeMap::new();
    let mut parties: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let mut statement = conn
        .prepare(&format!(
            "SELECT DISTINCT m.content_id, m.kind, m.role, e.id, e.name FROM knowledge_mention m JOIN knowledge_entity e ON e.id = m.entity_id
             WHERE m.content_id IN ({list}) AND e.state = 'active'"
        ))
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))
        })
        .map_err(sql_err)?;
    let mut seen: BTreeSet<(i64, String)> = BTreeSet::new();
    for row in rows {
        let (content, kind, role, id, name) = row.map_err(sql_err)?;
        if kind == rules.kind {
            if vocabulary.is_generic(&quote::words(&name).join(" ")) {
                continue;
            }
            let entry = named.entry(id).or_insert((0, 0, name));
            if seen.insert((content, entry.2.clone())) {
                entry.0 += 1;
            }
            if rules.subject_roles.iter().any(|r| r == &role) {
                entry.1 += 1;
            }
        } else if rules.party_roles.iter().any(|r| r == &role) && super::pages::name_like(&kind, &name) {
            let entry = parties.entry(id).or_insert((0, name));
            if seen.insert((content, format!("party:{}", entry.1))) {
                entry.0 += 1;
            }
        }
    }
    let mut parties: Vec<(usize, String)> = parties.into_values().collect();
    parties.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    let parties: Vec<String> = parties.into_iter().take(rules.parties_in_name).map(|(_, n)| one_line(&n)).collect();
    let mut named: Vec<(usize, usize, String)> = named.into_values().collect();
    named.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.cmp(&a.0)).then_with(|| b.2.len().cmp(&a.2.len())).then_with(|| a.2.cmp(&b.2)));
    let words = &vocabulary.pages;
    Ok(match (named.first(), parties.is_empty()) {
        (Some((_, _, name)), true) => one_line(name),
        (Some((_, _, name)), false) => format!("{} — {}", one_line(name), parties.join(", ")),
        (None, false) => format!("{} {}", words.word("matterBetween"), parties.join(", ")),
        (None, true) => words.word("matterUnnamed").to_string(),
    })
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Version of the question about the matter of a document.
pub const MATTER_PROMPT: &str = "asunto-1";
/// The task its answers are kept under in `model_answer`.
pub const TASK: &str = "matter";
/// Words of a document's text shown in its question.
const OPENING_WORDS: usize = 250;
/// Documents of a matter described in a question.
const MATTER_DOCUMENTS: usize = 10;
/// What its documents say, described in a question.
const FACTS: usize = 15;

/// What a pass of proposals did.
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposeReport {
    /// Nothing to do: the identities are still being resolved.
    pub skipped: bool,
    /// Documents asked about, questions sent, failed and left for later.
    pub candidates: usize,
    pub asked: usize,
    pub failed: usize,
    pub left: usize,
    /// Documents proposed for a matter now.
    pub proposed: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct Choice {
    asunto: i64,
    #[serde(default)]
    certeza: Option<String>,
    #[serde(default)]
    razon: Option<String>,
}

pub fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["asunto", "certeza", "razon"],
        "properties": {
            "asunto": { "type": "integer" },
            "certeza": { "type": "string", "enum": ["alta", "media", "baja"] },
            "razon": { "type": "string" }
        }
    })
}

/// The parties of each document: entities other than matters, with a
/// role that is not the fallback one, whose name is a name.
fn parties_by_document(conn: &Connection, vocabulary: &Vocabulary) -> Result<BTreeMap<i64, BTreeSet<String>>, String> {
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT m.content_id, m.role, e.id, e.kind, e.name FROM knowledge_mention m JOIN knowledge_entity e ON e.id = m.entity_id
             WHERE e.state = 'active' AND e.kind != ?1",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([&vocabulary.matters.kind], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, String>(4)?))
        })
        .map_err(sql_err)?;
    let mut out: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    for row in rows {
        let (content, role, id, kind, name) = row.map_err(sql_err)?;
        if role != vocabulary.fallback_role() && super::pages::name_like(&kind, &name) {
            out.entry(content).or_default().insert(id);
        }
    }
    Ok(out)
}

/// A document, as a question shows it: its name and type, who appears in
/// it and with what role, what it says of them, and how it starts.
fn describe_document(conn: &Connection, vocabulary: &Vocabulary, content: i64) -> Result<String, String> {
    let name = crate::docbase::similar::classify::name_of(conn, content)?;
    let doc_type: Option<String> = conn.query_row("SELECT doc_type FROM document_type WHERE content_id = ?1", [content], |row| row.get(0)).ok();
    let mut out = format!("Documento: {name}");
    if let Some(doc_type) = doc_type {
        out.push_str(&format!(" ({})", doc_type.replace('-', " ")));
    }
    out.push('\n');
    let mut statement = conn
        .prepare(
            "SELECT DISTINCT e.name, e.kind, m.role FROM knowledge_mention m JOIN knowledge_entity e ON e.id = m.entity_id
             WHERE m.content_id = ?1 AND e.state = 'active' ORDER BY e.kind, e.name, m.role",
        )
        .map_err(sql_err)?;
    let rows: Vec<(String, String, String)> = statement
        .query_map([content], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    if !rows.is_empty() {
        let named: Vec<String> = rows
            .iter()
            .map(|(name, kind, role)| format!("{} ({}, {})", one_line(name), vocabulary.type_label(kind), vocabulary.role_label(role)))
            .collect();
        out.push_str(&format!("- Nombra a: {}\n", named.join("; ")));
    }
    let mut statement = conn
        .prepare(
            "SELECT e.predicate, coalesce(e.object_value, o.written), e.quote FROM knowledge_evidence e
             LEFT JOIN knowledge_mention o ON o.id = e.object_mention WHERE e.content_id = ?1 ORDER BY e.id LIMIT ?2",
        )
        .map_err(sql_err)?;
    let facts: Vec<(String, String, String)> = statement
        .query_map(params![content, FACTS as i64], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    for (predicate, object, quote_text) in facts {
        out.push_str(&format!("- {}: {} («{}»)\n", vocabulary.predicate_label(&predicate), one_line(&object), one_line(&quote_text)));
    }
    let body: Option<String> =
        conn.query_row("SELECT body FROM derivative WHERE content_id = ?1 AND kind = 'text'", [content], |row| row.get(0)).ok();
    if let Some(body) = body {
        let opening: Vec<&str> = body.split_whitespace().filter(|w| !w.starts_with("##")).take(OPENING_WORDS).collect();
        out.push_str(&format!("- Empieza así: «{}»\n", opening.join(" ")));
    }
    Ok(out)
}

/// A matter, as a question shows it: its name, its documents and parties.
fn describe_matter(conn: &Connection, vocabulary: &Vocabulary, name: &str, documents: &BTreeSet<i64>, parties: &BTreeSet<String>) -> Result<String, String> {
    let mut out = format!("Asunto: {}\n", one_line(name));
    for content in documents.iter().take(MATTER_DOCUMENTS) {
        let doc_name = crate::docbase::similar::classify::name_of(conn, *content)?;
        let doc_type: Option<String> = conn.query_row("SELECT doc_type FROM document_type WHERE content_id = ?1", [content], |row| row.get(0)).ok();
        out.push_str(&format!("- Documento: {doc_name}{}\n", doc_type.map(|t| format!(" ({})", t.replace('-', " "))).unwrap_or_default()));
    }
    if documents.len() > MATTER_DOCUMENTS {
        out.push_str(&format!("- y {} documentos más\n", documents.len() - MATTER_DOCUMENTS));
    }
    let mut names: Vec<String> = Vec::new();
    for id in parties {
        if let Ok((entity, kind)) = conn.query_row("SELECT name, kind FROM knowledge_entity WHERE id = ?1", [id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))) {
            names.push(format!("{} ({})", one_line(&entity), vocabulary.type_label(&kind)));
        }
    }
    names.sort();
    if !names.is_empty() {
        out.push_str(&format!("- Partes y personas: {}\n", names.join("; ")));
    }
    Ok(out)
}

fn matter_question(document: &str, matters: &[String], content_id: i64) -> Question {
    let listed: Vec<String> = matters.iter().enumerate().map(|(i, m)| format!("[{}]\n{m}", i + 1)).collect();
    let prompt = format!(
        "Sos asistente de un estudio jurídico. Hay que decidir si un documento del archivo es parte de alguno de estos asuntos \
(un asunto es un reclamo, un contrato, un trámite: un conjunto de documentos sobre los mismos hechos). Usá solo lo que se \
muestra: no agregues nada que sepas por otro lado.\n\n\
Reglas:\n\
- Que el documento nombre a las mismas personas u organizaciones no alcanza: pueden tener varios asuntos entre ellas.\n\
- Elegí un asunto solo si el documento trata de los mismos hechos: el mismo contrato, reclamo, deuda, trámite o \
intercambio, o se refiere a uno de sus documentos.\n\
- Si no es de ninguno, o no se puede saber, respondé 0.\n\
- En \"certeza\": \"alta\" si el documento lo dice (cita el contrato, la factura, la carta); \"media\" si los hechos coinciden \
sin que lo diga; \"baja\" si es una suposición.\n\
- En \"razon\", en una oración, qué dato del documento lo decide.\n\n\
{document}\n\
Asuntos:\n{}\n\n\
Respondé solo con el JSON pedido: {{\"asunto\": número del asunto o 0, \"certeza\": ..., \"razon\": ...}}.",
        listed.join("\n")
    );
    Question {
        key: crate::docbase::similar::classify::question_key(MATTER_PROMPT, &prompt),
        version: MATTER_PROMPT,
        task: TASK,
        content_id,
        prompt,
        schema: schema(),
    }
}

impl crate::docbase::DocBase {
    /// Proposes a matter for the documents no evidence puts in one, when
    /// they share parties with it (D-213): the model reads the document
    /// and the matters and says which, if any. Up to `max_questions`
    /// questions; an answer is kept and never asked again (R-20). What it
    /// says is a proposal for a person, shown as such, never a link
    /// between documents (R-03). Waits for the resolution to finish.
    pub fn knowledge_propose_matters(&self, asker: Option<&dyn DocumentAsker>, max_questions: usize) -> Result<ProposeReport, String> {
        let _asking = asker.map(|_| crate::docbase::similar::classify::ASKING.lock().unwrap_or_else(|p| p.into_inner()));
        let vocabulary = super::vocabulary::vocabulary();
        let rules = &vocabulary.matters;
        let mut report = ProposeReport::default();
        let to_resolve: i64 = self
            .conn()
            .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')", [], |row| row.get(0))
            .map_err(sql_err)?;
        if to_resolve > 0 {
            report.skipped = true;
            return Ok(report);
        }
        let engine = match asker {
            Some(asker) => Some(asker.engine_id()),
            None => crate::docbase::similar::classify::ask_engine(&self.conn())?.map(|c| crate::source_volume::asking::engine_id(&c)),
        };
        let (matters, grouped, parties, separated) = {
            let conn = self.conn();
            let mut matters: BTreeMap<String, (String, BTreeSet<i64>)> = BTreeMap::new();
            let mut statement = conn
                .prepare(
                    "SELECT m.id, m.name, d.content_id FROM knowledge_matter m JOIN knowledge_matter_document d ON d.matter_id = m.id
                     WHERE m.state = 'active' AND d.state = 'firm'",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))
                .map_err(sql_err)?;
            for row in rows {
                let (id, name, content) = row.map_err(sql_err)?;
                matters.entry(id).or_insert_with(|| (name, BTreeSet::new())).1.insert(content);
            }
            let grouped: BTreeSet<i64> = matters.values().flat_map(|(_, d)| d.iter().copied()).collect();
            let parties = parties_by_document(&conn, vocabulary)?;
            let decided = super::resolve::load_decided(&conn)?;
            let successors = changes::successors(&conn)?;
            let separated: BTreeSet<(i64, i64)> = decided
                .separations
                .iter()
                .filter_map(|(a, b)| {
                    let (a, b) = (changes::translate(a, &successors), changes::translate(b, &successors));
                    Some((content_of(&a)?, content_of(&b)?))
                })
                .flat_map(|(a, b)| [(a, b), (b, a)])
                .collect();
            (matters, grouped, parties, separated)
        };
        let loose: Vec<i64> = documents(&self.conn())?.into_iter().map(|(c, _)| c).filter(|c| !grouped.contains(c)).collect();
        let mut changed = false;
        let mut stopped = false;
        for content in loose {
            let Some(own) = parties.get(&content) else { continue };
            let mut candidates: Vec<(usize, &String)> = matters
                .iter()
                .filter(|(_, (_, docs))| !docs.iter().any(|d| separated.contains(&(content, *d))))
                .map(|(id, (_, docs))| {
                    let theirs: BTreeSet<&String> = docs.iter().flat_map(|d| parties.get(d).into_iter().flatten()).collect();
                    (own.iter().filter(|p| theirs.contains(p)).count(), id)
                })
                .filter(|(shared, _)| *shared >= rules.propose_with_shared)
                .collect();
            if candidates.is_empty() {
                continue;
            }
            candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
            candidates.truncate(rules.propose_candidates);
            report.candidates += 1;
            let ids: Vec<&String> = candidates.iter().map(|(_, id)| *id).collect();
            let known: usize = {
                let conn = self.conn();
                let mut n = 0;
                for id in &ids {
                    let found: Option<String> = conn
                        .query_row(
                            "SELECT verdict FROM knowledge_matter_verdict WHERE content_id = ?1 AND matter_id = ?2 AND engine = ?3",
                            params![content, id, engine.clone().unwrap_or_default()],
                            |row| row.get(0),
                        )
                        .ok();
                    n += found.is_some() as usize;
                }
                n
            };
            if known == ids.len() {
                continue;
            }
            let question = {
                let conn = self.conn();
                let document = describe_document(&conn, vocabulary, content)?;
                let mut texts = Vec::new();
                for id in &ids {
                    let (name, docs) = &matters[*id];
                    let theirs: BTreeSet<String> = docs.iter().flat_map(|d| parties.get(d).into_iter().flatten().cloned()).collect();
                    texts.push(describe_matter(&conn, vocabulary, name, docs, &theirs)?);
                }
                matter_question(&document, &texts, content)
            };
            let cached = crate::docbase::similar::classify::cached_answer(&self.conn(), &question.key, engine.as_deref())?;
            let answer = match (cached, asker) {
                (Some(answer), _) => Some(answer),
                (None, Some(asker)) if !stopped && report.asked < max_questions => {
                    report.asked += 1;
                    match crate::docbase::similar::classify::ask_and_keep(self, asker, &question) {
                        Ok(answer) => Some(answer),
                        Err(error) => {
                            report.failed += 1;
                            stopped = crate::docbase::similar::classify::stops_the_run(&error);
                            report.errors.push(error);
                            None
                        }
                    }
                }
                _ => None,
            };
            let Some(answer) = answer else {
                report.left += 1;
                continue;
            };
            let choice: Option<Choice> = crate::docbase::similar::classify::parse_json(&answer).ok();
            let now = now_ms();
            for (index, id) in ids.iter().enumerate() {
                let (verdict, reason) = match &choice {
                    Some(c) if c.asunto == index as i64 + 1 && c.certeza.as_deref() != Some("baja") => {
                        ("same", Some(format!("certeza {}: {}", c.certeza.as_deref().unwrap_or("-"), c.razon.as_deref().unwrap_or_default())))
                    }
                    Some(c) if c.asunto == index as i64 + 1 => ("doubtful", c.razon.clone()),
                    Some(c) => ("different", c.razon.clone()),
                    None => ("doubtful", Some("el modelo no respondió en la forma pedida".to_string())),
                };
                self.conn()
                    .execute(
                        "INSERT OR REPLACE INTO knowledge_matter_verdict (content_id, matter_id, engine, verdict, reason, question, asked_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![content, id, engine.clone().unwrap_or_default(), verdict, reason, question.key, now],
                    )
                    .map_err(sql_err)?;
                if verdict == "same" {
                    report.proposed += 1;
                }
                changed = true;
            }
        }
        if changed {
            super::mark_dirty(&self.conn(), "project", "matter-proposals")?;
        }
        Ok(report)
    }
}

/// What the model proposed: the matter of each document no evidence puts
/// in one, with its reason, for the engine in use (or any, without one).
pub(crate) fn proposals(conn: &Connection) -> Result<BTreeMap<i64, Vec<(String, Option<String>)>>, String> {
    let engine = crate::docbase::similar::classify::ask_engine(conn)?.map(|c| crate::source_volume::asking::engine_id(&c));
    let mut statement = conn
        .prepare(
            "SELECT v.content_id, v.matter_id, v.reason, v.engine FROM knowledge_matter_verdict v
             JOIN knowledge_matter m ON m.id = v.matter_id AND m.state = 'active'
             WHERE v.verdict = 'same' AND NOT EXISTS (SELECT 1 FROM knowledge_matter_document d WHERE d.content_id = v.content_id)
             ORDER BY v.content_id, v.matter_id",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<String>>(2)?, row.get::<_, String>(3)?)))
        .map_err(sql_err)?;
    let mut out: BTreeMap<i64, Vec<(String, Option<String>)>> = BTreeMap::new();
    let found: Vec<(i64, String, Option<String>, String)> = rows.collect::<Result<_, _>>().map_err(sql_err)?;
    drop(statement);
    if found.is_empty() {
        return Ok(out);
    }
    // A person who kept the document apart from a document of the matter
    // answered the proposal (D-218).
    let apart = separated_documents(conn)?;
    let mut members: HashMap<String, Vec<i64>> = HashMap::new();
    {
        let mut statement = conn.prepare("SELECT matter_id, content_id FROM knowledge_matter_document").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (matter, content) = row.map_err(sql_err)?;
            members.entry(matter).or_default().push(content);
        }
    }
    for (content, matter, reason, by) in found {
        if engine.as_deref().is_some_and(|e| e != by) {
            continue;
        }
        let answered = members.get(&matter).into_iter().flatten().any(|other| apart.contains(&(content.min(*other), content.max(*other))));
        if answered {
            continue;
        }
        out.entry(content).or_default().push((matter, reason));
    }
    Ok(out)
}

/// Pairs of documents a person kept apart ("not the same matter"), each
/// with the smaller id first, after the documents that took the place of
/// others.
fn separated_documents(conn: &Connection) -> Result<BTreeSet<(i64, i64)>, String> {
    let successors = changes::successors(conn)?;
    let mut statement = conn
        .prepare("SELECT a, b FROM knowledge_decision WHERE state = 'active' AND kind = 'separate' AND a LIKE 'content:%'")
        .map_err(sql_err)?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))).map_err(sql_err)?;
    let mut out = BTreeSet::new();
    for row in rows {
        let (a, b) = row.map_err(sql_err)?;
        let a = content_of(&changes::translate(&a, &successors));
        let b = b.and_then(|b| content_of(&changes::translate(&b, &successors)));
        if let (Some(a), Some(b)) = (a, b) {
            out.insert((a.min(b), a.max(b)));
        }
    }
    Ok(out)
}
