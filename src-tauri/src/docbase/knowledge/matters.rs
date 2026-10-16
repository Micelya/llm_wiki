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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::changes;
use super::names;
use super::quote;
use super::values;
use super::vocabulary::Vocabulary;
use crate::docbase::{now_ms, sql_err};

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
