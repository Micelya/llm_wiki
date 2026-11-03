//! Pass 6: the relations of two documents, by the rules of their type.
//!
//! With the type, the stage, the marks and the data of each document,
//! the catalog's rules (`types::RelationRule`) confirm or correct what
//! pass 1 measured:
//!
//! - **different data, different documents:** two letters that share the
//!   postal form but claim different invoices are two letters, however
//!   much text they share. A relation that makes them one document is
//!   taken down to the rule's (`same-template`) when they share as much
//!   text as a template's documents do (`SHARE_MUCH`), and to `similar`
//!   when they share less (P-485).
//! - **stage pairs:** the preview of a letter and the letter as sent,
//!   with the same data, are `preview`, even when the preview's watermark
//!   could not be read.
//! - **shared data:** the delivery receipt and the letter with the same
//!   shipment number; the signing certificate and the document with the
//!   same signing process. Proposed even when their texts share nothing.
//!   The value shared is each one's own (the first in reading order, of
//!   its form or its heading), not a number one of them cites (P-485).
//!
//! A stage needs its data (`stage_data` of the type, P-485): a letter the
//! model says was sent but without a shipment number is not sent yet, and
//! is taken as the stage before, unless a mark of the document shows the
//! stage it was said to be in.
//!
//! What the general rule already resolved is left alone: a rule only
//! acts on the relations it names. A decision of a person is never
//! touched (R-08), and a rule that stops applying gives back what was
//! measured (R-09).

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use super::super::{now_ms, sql_err, DocBase};
use super::classify::{load_docs, read_data, read_types};
use super::data::comparable;
use super::propose::{RECOGNIZED_TOLERANCE, SHARE_MUCH};
use super::text;
use super::types::{self, Catalog, RelationRule};

/// How close two documents are after each measured relation, closest
/// first. `different-data` only takes a relation down to its own, never
/// up: one document, then a stage of the other, then one inside the
/// other, then the same template, then only similar.
const CLOSENESS: &[&str] = &[
    "same-document",
    "near-identical",
    "signed-copy",
    "version",
    "preview",
    "contained-in",
    "same-template",
    "similar",
];

fn closeness(relation: &str) -> usize {
    CLOSENESS.iter().position(|r| *r == relation).unwrap_or(CLOSENESS.len())
}

/// Relations a `stage-pair` rule turns into its own: the two share their
/// base text (or one holds the other, as a preview holds the letter) and
/// the general rule found no stage in them.
const STAGE_PAIR_FROM: &[&str] = &["near-identical", "version", "contained-in", "same-template"];
/// Measured relations a `shared-data` rule replaces: they only say the
/// texts look alike.
const SHARED_DATA_OVER: &[&str] = &["similar", "same-template"];

/// What the measurement proposed, kept when a rule changes it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Measured {
    relation: String,
    subject: Option<i64>,
    certainty: String,
    description: String,
    suggested_source: Option<i64>,
    evidence: String,
    evidence_hash: String,
}

/// What a rule says of a pair.
#[derive(Debug, Clone, PartialEq)]
struct Verdict {
    relation: String,
    subject: Option<i64>,
    rule: String,
    certainty: Option<&'static str>,
    description: String,
    evidence: serde_json::Value,
}

/// What pass 6 knows of a document.
#[derive(Debug, Clone, Default)]
struct Known {
    /// Its own type, unless it is only proposed.
    own: Option<String>,
    stage: Option<String>,
    /// Its own type and the carriers its marks show.
    kinds: BTreeSet<String>,
    /// Comparable values of each datum, and the first one as written.
    data: BTreeMap<String, (BTreeSet<String>, String)>,
    /// The parts of its file that certify it: (version, relation, rule).
    certified_by: Vec<(i64, String, String)>,
}

fn words(id: &str) -> String {
    id.replace('-', " ")
}

fn load_known(conn: &Connection, catalog: &Catalog) -> Result<HashMap<i64, Known>, String> {
    let docs = load_docs(conn)?;
    let found = read_types(conn, None)?;
    let data = read_data(conn, None)?;
    let mut out = HashMap::new();
    for (id, doc) in docs {
        let mut known = Known::default();
        if let Some(found) = found.get(&id) {
            if !found.proposed {
                known.own = Some(found.doc_type.clone());
                known.stage = found.stage.clone();
                known.kinds.insert(found.doc_type.clone());
            }
        }
        for mark in &doc.marks {
            if catalog.doc_type(&mark.doc_type).is_some_and(|t| t.carrier) {
                known.kinds.insert(mark.doc_type.clone());
            }
            let within = catalog.within_file.iter().find(|w| w.id == mark.signal);
            if let (Some(rule), Some(certifier)) = (within, mark.value.as_deref().and_then(|v| v.parse::<i64>().ok())) {
                known.certified_by.push((certifier, rule.relation.clone(), rule.id.clone()));
            }
        }
        for datum in data.get(&id).cloned().flatten().unwrap_or_default() {
            let entry = known.data.entry(datum.field.clone()).or_insert_with(|| (BTreeSet::new(), datum.value.clone()));
            entry.0.insert(comparable(&datum.value));
        }
        let shown: Vec<&str> = doc.marks.iter().filter_map(|m| m.stage.as_deref()).collect();
        settle_stage(catalog, &mut known, &shown);
        out.insert(id, known);
    }
    Ok(out)
}

/// A stage needs its data (P-485): the stage the model said, unless the
/// type says it needs a datum the document does not have; then the latest
/// earlier stage whose data it has. A stage a mark of the document shows
/// (`shown`) is verified and stays.
fn settle_stage(catalog: &Catalog, known: &mut Known, shown: &[&str]) {
    let Some(doc_type) = known.own.as_deref().and_then(|t| catalog.doc_type(t)) else { return };
    let Some(said) = known.stage.as_deref() else { return };
    if shown.contains(&said) {
        return;
    }
    let data = &known.data;
    known.stage = doc_type.stage_with_data(said, |field| data.get(field).is_some_and(|(values, _)| !values.is_empty()));
}

/// The part of the shorter document found in the other, as the general
/// rule weighs it: text read from images gets its slack (`propose.rs`).
fn shared_part(cover: f64, evidence: &str) -> f64 {
    let evidence: serde_json::Value = serde_json::from_str(evidence).unwrap_or_default();
    let recognized = ["a", "b"].iter().any(|side| evidence["recognized"][side].as_bool() == Some(true));
    cover + if recognized { RECOGNIZED_TOLERANCE } else { 0.0 }
}

fn rules_of<'a>(catalog: &'a Catalog, doc_type: &str, kind: &'a str) -> impl Iterator<Item = &'a RelationRule> + 'a {
    catalog.doc_type(doc_type).into_iter().flat_map(move |t| t.rules.iter().filter(move |r| r.kind == kind))
}

/// What the rules of the types say of `a` and `b`, given what was
/// measured (`None`: nothing) and the part of the text they share
/// (`shared_part`).
fn verdict(catalog: &Catalog, a: (i64, &Known), b: (i64, &Known), measured: Option<&str>, share: f64) -> Option<Verdict> {
    let shared: Vec<&String> = a.1.kinds.intersection(&b.1.kinds).collect();

    // A part of the same file that certifies the other (`attest.rs`).
    if measured.is_none_or(|m| SHARED_DATA_OVER.contains(&m)) {
        for (certified, certifier) in [(a, b), (b, a)] {
            let Some((_, relation, rule)) = certified.1.certified_by.iter().find(|(id, _, _)| *id == certifier.0) else { continue };
            return Some(Verdict {
                relation: relation.clone(),
                subject: Some(certifier.0),
                rule: rule.clone(),
                certainty: Some("high"),
                description: "Va en el mismo archivo y da fe de su firma.".to_string(),
                evidence: serde_json::json!({ "withinFile": rule, "measured": measured }),
            });
        }
    }

    // Different data: different documents. Sharing as much text as a
    // template's documents do, they are still of the same template; with
    // less, only similar (P-485).
    if let Some(measured) = measured {
        for doc_type in &shared {
            for rule in rules_of(catalog, doc_type, "different-data") {
                let relation =
                    if share < SHARE_MUCH && closeness(&rule.relation) < closeness("similar") { "similar" } else { rule.relation.as_str() };
                if closeness(measured) >= closeness(relation) {
                    continue;
                }
                for field in &rule.data {
                    let (Some((va, wa)), Some((vb, wb))) = (a.1.data.get(field), b.1.data.get(field)) else { continue };
                    if va.is_empty() || vb.is_empty() || va == vb {
                        continue;
                    }
                    return Some(Verdict {
                        relation: relation.to_string(),
                        subject: None,
                        rule: rule.id.clone(),
                        certainty: None,
                        description: format!(
                            "Comparten el texto de base, pero son documentos distintos: como {}, cambia {} ({wa} y {wb}).",
                            words(doc_type),
                            words(field)
                        ),
                        evidence: serde_json::json!({
                            "type": doc_type, "field": field, "values": [wa, wb], "measured": measured, "share": share
                        }),
                    });
                }
            }
        }
    }

    // A stage and the next one of the same document.
    if let Some(measured) = measured.filter(|m| STAGE_PAIR_FROM.contains(m)) {
        if let (Some(ta), Some(tb)) = (&a.1.own, &b.1.own) {
            if ta == tb {
                for rule in rules_of(catalog, ta, "stage-pair") {
                    let Some(subject_stage) = &rule.subject_stage else { continue };
                    let subject = if a.1.stage.as_ref() == Some(subject_stage)
                        && b.1.stage.as_ref().is_some_and(|s| rule.other_stages.contains(s))
                    {
                        a
                    } else if b.1.stage.as_ref() == Some(subject_stage)
                        && a.1.stage.as_ref().is_some_and(|s| rule.other_stages.contains(s))
                    {
                        b
                    } else {
                        continue;
                    };
                    // The data the rule names must be there in both, and equal.
                    let agree = rule.data.iter().all(|field| match (a.1.data.get(field), b.1.data.get(field)) {
                        (Some((va, _)), Some((vb, _))) => !va.is_empty() && va == vb,
                        _ => false,
                    });
                    if !agree {
                        continue;
                    }
                    let other = if subject.0 == a.0 { b } else { a };
                    return Some(Verdict {
                        relation: rule.relation.clone(),
                        subject: Some(subject.0),
                        rule: rule.id.clone(),
                        certainty: Some("low"),
                        description: format!(
                            "Según su tipo ({}), uno está en la etapa «{}» y el otro en «{}», con {} iguales.",
                            words(ta),
                            words(subject_stage),
                            words(other.1.stage.as_deref().unwrap_or_default()),
                            if rule.data.is_empty() {
                                "el texto de base".to_string()
                            } else {
                                rule.data.iter().map(|d| words(d)).collect::<Vec<_>>().join(", ")
                            }
                        ),
                        evidence: serde_json::json!({
                            "type": ta, "stages": [a.1.stage, b.1.stage], "fields": rule.data, "measured": measured
                        }),
                    });
                }
            }
        }
    }

    // The same datum in a document of this type and one of the other.
    if measured.is_none_or(|m| SHARED_DATA_OVER.contains(&m)) {
        for (subject, other) in [(a, b), (b, a)] {
            for doc_type in &subject.1.kinds {
                for rule in rules_of(catalog, doc_type, "shared-data") {
                    let Some(other_type) = &rule.other_type else { continue };
                    if !other.1.kinds.contains(other_type) || other.1.kinds.contains(doc_type) {
                        continue;
                    }
                    let mut values = Vec::new();
                    // Each one's own value: the first, of its form or its
                    // heading; a number one of them cites is a reference.
                    let same = !rule.data.is_empty()
                        && rule.data.iter().all(|field| match (subject.1.data.get(field), other.1.data.get(field)) {
                            (Some((_, written)), Some((_, other_written))) if comparable(written) == comparable(other_written) => {
                                values.push(written.clone());
                                true
                            }
                            _ => false,
                        });
                    if !same {
                        continue;
                    }
                    return Some(Verdict {
                        relation: rule.relation.clone(),
                        subject: Some(subject.0),
                        rule: rule.id.clone(),
                        certainty: Some(if rule.certainty.as_deref() == Some("high") { "high" } else { "low" }),
                        description: format!(
                            "Según su tipo, comparten {}: {}.",
                            rule.data.iter().map(|d| words(d)).collect::<Vec<_>>().join(", "),
                            values.join(", ")
                        ),
                        evidence: serde_json::json!({
                            "type": doc_type, "otherType": other_type, "fields": rule.data, "values": values, "measured": measured
                        }),
                    });
                }
            }
        }
    }
    None
}

/// Pairs a `shared-data` rule may join that pass 1 did not measure: by
/// datum value, a document of the rule's type and one of the other.
fn shared_data_candidates(catalog: &Catalog, known: &HashMap<i64, Known>) -> BTreeSet<(i64, i64)> {
    let mut by_value: HashMap<(&str, &str, &str), Vec<i64>> = HashMap::new();
    for (id, doc) in known {
        for kind in &doc.kinds {
            for (field, (values, _)) in &doc.data {
                for value in values {
                    by_value.entry((kind.as_str(), field.as_str(), value.as_str())).or_default().push(*id);
                }
            }
        }
    }
    let mut out = BTreeSet::new();
    for doc_type in &catalog.types {
        for rule in doc_type.rules.iter().filter(|r| r.kind == "shared-data") {
            let (Some(other_type), Some(field)) = (&rule.other_type, rule.data.first()) else { continue };
            for ((kind, f, value), ids) in &by_value {
                if *kind != doc_type.id || f != field {
                    continue;
                }
                let Some(others) = by_value.get(&(other_type.as_str(), field.as_str(), *value)) else { continue };
                for a in ids {
                    for b in others {
                        if a != b {
                            out.insert((*a.min(b), *a.max(b)));
                        }
                    }
                }
            }
        }
    }
    out
}

/// A proposed pair as stored.
struct Stored {
    a: i64,
    b: i64,
    relation: String,
    subject: Option<i64>,
    origin: String,
    measured: Option<Measured>,
    rule: Option<String>,
    current: Measured,
    /// The part of the shorter one found in the other, as measured.
    cover: f64,
}

impl DocBase {
    /// Pass 6. Returns how many pairs changed.
    pub fn apply_type_rules(&self) -> Result<usize, String> {
        let catalog = types::catalog();
        let mut conn = self.conn();
        let known = load_known(&conn, catalog)?;
        let stored: Vec<Stored> = {
            let mut statement = conn
                .prepare(
                    "SELECT a, b, relation, subject, origin, measured, type_rule, certainty, description, suggested_source,
                            evidence, evidence_hash, cover
                     FROM similar_pair WHERE state = 'proposed'",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| {
                    let measured: Option<String> = row.get(5)?;
                    Ok(Stored {
                        a: row.get(0)?,
                        b: row.get(1)?,
                        relation: row.get(2)?,
                        subject: row.get(3)?,
                        origin: row.get(4)?,
                        measured: measured.and_then(|m| serde_json::from_str(&m).ok()),
                        rule: row.get(6)?,
                        current: Measured {
                            relation: row.get(2)?,
                            subject: row.get(3)?,
                            certainty: row.get(7)?,
                            description: row.get(8)?,
                            suggested_source: row.get(9)?,
                            evidence: row.get(10)?,
                            evidence_hash: row.get(11)?,
                        },
                        cover: row.get::<_, Option<f64>>(12)?.unwrap_or(0.0),
                    })
                })
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let decided: BTreeSet<(i64, i64)> = {
            let mut statement = conn.prepare("SELECT a, b FROM similar_pair WHERE state != 'proposed'").map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?
                .collect::<Result<BTreeSet<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let empty = Known::default();
        let tx = conn.transaction().map_err(sql_err)?;
        let now = now_ms();
        let mut changed = 0;
        let mut seen: BTreeSet<(i64, i64)> = BTreeSet::new();

        for pair in &stored {
            seen.insert((pair.a, pair.b));
            // What was measured, before any rule.
            let base: Option<Measured> = match (&pair.rule, pair.origin.as_str()) {
                (None, _) => Some(pair.current.clone()),
                (Some(_), "type") => None,
                (Some(_), _) => pair.measured.clone(),
            };
            let share = base.as_ref().map(|m| shared_part(pair.cover, &m.evidence)).unwrap_or(0.0);
            let found = verdict(
                catalog,
                (pair.a, known.get(&pair.a).unwrap_or(&empty)),
                (pair.b, known.get(&pair.b).unwrap_or(&empty)),
                base.as_ref().map(|m| m.relation.as_str()),
                share,
            );
            match (found, base) {
                (Some(v), base) => {
                    if pair.rule.as_deref() == Some(v.rule.as_str()) && pair.relation == v.relation && pair.subject == v.subject {
                        continue;
                    }
                    let base_evidence = base.as_ref().map(|m| m.evidence.clone()).unwrap_or_else(|| "{}".into());
                    let mut evidence: serde_json::Value = serde_json::from_str(&base_evidence).unwrap_or(serde_json::json!({}));
                    evidence["typeRule"] = serde_json::json!({ "id": v.rule, "detail": v.evidence });
                    let evidence = evidence.to_string();
                    let hash = format!("{:016x}", text::fnv1a(evidence.as_bytes()));
                    let certainty = v.certainty.map(str::to_string).or_else(|| base.as_ref().map(|m| m.certainty.clone())).unwrap_or_else(|| "low".into());
                    let keeps_one = catalog.relation(&v.relation).is_some_and(|r| r.same_document);
                    let source = if keeps_one { base.as_ref().and_then(|m| m.suggested_source) } else { None };
                    tx.execute(
                        "UPDATE similar_pair SET relation = ?3, subject = ?4, certainty = ?5, description = ?6, suggested_source = ?7,
                             evidence = ?8, evidence_hash = ?9, type_rule = ?10, measured = ?11, computed_at = ?12
                         WHERE a = ?1 AND b = ?2",
                        params![
                            pair.a,
                            pair.b,
                            v.relation,
                            v.subject,
                            certainty,
                            v.description,
                            source,
                            evidence,
                            hash,
                            v.rule,
                            base.map(|m| serde_json::to_string(&m).unwrap_or_default()),
                            now
                        ],
                    )
                    .map_err(sql_err)?;
                    changed += 1;
                }
                (None, Some(base)) if pair.rule.is_some() => {
                    // The rule no longer applies: back to what was measured.
                    tx.execute(
                        "UPDATE similar_pair SET relation = ?3, subject = ?4, certainty = ?5, description = ?6, suggested_source = ?7,
                             evidence = ?8, evidence_hash = ?9, type_rule = NULL, measured = NULL, computed_at = ?10
                         WHERE a = ?1 AND b = ?2",
                        params![
                            pair.a,
                            pair.b,
                            base.relation,
                            base.subject,
                            base.certainty,
                            base.description,
                            base.suggested_source,
                            base.evidence,
                            base.evidence_hash,
                            now
                        ],
                    )
                    .map_err(sql_err)?;
                    changed += 1;
                }
                (None, None) if pair.origin == "type" => {
                    tx.execute("DELETE FROM similar_pair WHERE a = ?1 AND b = ?2 AND state = 'proposed'", params![pair.a, pair.b])
                        .map_err(sql_err)?;
                    changed += 1;
                }
                _ => {}
            }
        }

        // Pairs no measurement found that a rule of the type proposes.
        let certified: BTreeSet<(i64, i64)> = known
            .iter()
            .flat_map(|(id, doc)| doc.certified_by.iter().map(move |(by, _, _)| (*id.min(by), *id.max(by))))
            .filter(|(a, b)| a != b && known.contains_key(a) && known.contains_key(b))
            .collect();
        let candidates: BTreeSet<(i64, i64)> = shared_data_candidates(catalog, &known).into_iter().chain(certified).collect();
        for (a, b) in candidates {
            if seen.contains(&(a, b)) || decided.contains(&(a, b)) {
                continue;
            }
            let Some(v) = verdict(catalog, (a, &known[&a]), (b, &known[&b]), None, 0.0) else { continue };
            let evidence = serde_json::json!({ "typeRule": { "id": v.rule, "detail": v.evidence } }).to_string();
            let hash = format!("{:016x}", text::fnv1a(evidence.as_bytes()));
            tx.execute(
                "INSERT INTO similar_pair (a, b, relation, subject, certainty, jaccard, containment, cover, similarity,
                                           description, suggested_source, evidence, evidence_hash, engine, computed_at,
                                           origin, type_rule)
                 VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 0, ?6, NULL, ?7, ?8, ?9, ?10, 'type', ?11)",
                params![a, b, v.relation, v.subject, v.certainty.unwrap_or("low"), v.description, evidence, hash, "types", now, v.rule],
            )
            .map_err(sql_err)?;
            changed += 1;
        }
        tx.commit().map_err(sql_err)?;
        Ok(changed)
    }

    /// The pairs a rule of the type set: (a, b, measured relation, relation
    /// now, rule).
    #[cfg(test)]
    pub fn similar_corrections(&self) -> Result<Vec<(i64, i64, Option<String>, String, String)>, String> {
        let conn = self.conn();
        let mut statement = conn
            .prepare("SELECT a, b, measured, relation, type_rule FROM similar_pair WHERE type_rule IS NOT NULL ORDER BY type_rule, a, b")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                let measured: Option<String> = row.get(2)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    measured.and_then(|m| serde_json::from_str::<Measured>(&m).ok()).map(|m| m.relation),
                    row.get(3)?,
                    row.get(4)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// All of the text in common.
    const ALL: f64 = 1.0;

    fn known(own: &str, stage: Option<&str>, data: &[(&str, &str)]) -> Known {
        let mut k = Known { own: Some(own.into()), stage: stage.map(str::to_string), ..Default::default() };
        k.kinds.insert(own.into());
        for (field, value) in data {
            let entry = k.data.entry(field.to_string()).or_insert_with(|| (BTreeSet::new(), value.to_string()));
            entry.0.insert(comparable(value));
        }
        k
    }

    #[test]
    fn two_letters_with_different_invoices_are_different_documents() {
        let catalog = Catalog::builtin();
        let a = known("carta-documento", Some("enviada"), &[("facturas", "0003-00001111")]);
        let b = known("carta-documento", Some("enviada"), &[("facturas", "0003-00002222")]);
        // Sharing as much text as a template's documents do, they are two
        // letters of the same template (P-485)…
        let v = verdict(&catalog, (1, &a), (2, &b), Some("near-identical"), ALL).unwrap();
        assert_eq!(v.relation, "same-template");
        assert_eq!(verdict(&catalog, (1, &a), (2, &b), Some("contained-in"), 0.86).unwrap().relation, "same-template");
        // …and a shared template is already as far.
        assert_eq!(verdict(&catalog, (1, &a), (2, &b), Some("same-template"), ALL), None);
        // Sharing less, they are only similar.
        assert_eq!(verdict(&catalog, (1, &a), (2, &b), Some("contained-in"), 0.79).unwrap().relation, "similar");
        assert_eq!(verdict(&catalog, (1, &a), (2, &b), Some("same-template"), 0.79).unwrap().relation, "similar");
        // A relation never goes up, and one already as far is not touched.
        assert_eq!(verdict(&catalog, (1, &a), (2, &b), Some("similar"), ALL), None);
        let invoice_a = known("factura", None, &[("numero", "0003-00001111")]);
        let invoice_b = known("factura", None, &[("numero", "0003-00002222")]);
        assert_eq!(verdict(&catalog, (1, &invoice_a), (2, &invoice_b), Some("same-template"), ALL), None);
        // Same invoices: nothing to correct.
        let c = known("carta-documento", Some("enviada"), &[("facturas", "0003-00001111")]);
        assert_eq!(verdict(&catalog, (1, &a), (2, &c), Some("near-identical"), ALL), None);
    }

    #[test]
    fn a_preview_and_the_letter_sent_with_the_same_data_are_a_preview() {
        let catalog = Catalog::builtin();
        let preview = known("carta-documento", Some("vista-previa"), &[("facturas", "0003-00001111")]);
        let sent = known("carta-documento", Some("enviada"), &[("facturas", "0003-00001111")]);
        let v = verdict(&catalog, (1, &sent), (2, &preview), Some("version"), ALL).unwrap();
        assert_eq!((v.relation.as_str(), v.subject), ("preview", Some(2)));
        // Without the data to compare, nothing is said.
        let bare = known("carta-documento", Some("vista-previa"), &[]);
        assert_eq!(verdict(&catalog, (1, &sent), (2, &bare), Some("version"), ALL), None);
    }

    #[test]
    fn a_delivery_receipt_goes_with_the_letter_of_its_shipment_number() {
        let catalog = Catalog::builtin();
        let receipt = known("acuse-de-recibo", None, &[("numero-de-envio", "XY123456789AR")]);
        let letter = known("carta-documento", Some("enviada"), &[("numero-de-envio", "XY123456789AR")]);
        let v = verdict(&catalog, (1, &letter), (2, &receipt), None, ALL).unwrap();
        assert_eq!((v.relation.as_str(), v.subject), ("delivery-receipt-of", Some(2)));
        let other = known("carta-documento", Some("enviada"), &[("numero-de-envio", "XY999999999AR")]);
        assert_eq!(verdict(&catalog, (1, &other), (2, &receipt), None, ALL), None);
        // A relation the measurement already gave is not replaced.
        assert_eq!(verdict(&catalog, (1, &letter), (2, &receipt), Some("version"), ALL), None);
    }

    #[test]
    fn a_part_certified_by_another_of_its_file_has_it_as_its_signing_certificate() {
        let catalog = Catalog::builtin();
        let mut agreement = Known::default();
        agreement.certified_by.push((7, "signing-certificate-of".into(), "firmado-segun-su-constancia".into()));
        let certificate = Known::default();
        let v = verdict(&catalog, (3, &agreement), (7, &certificate), None, ALL).unwrap();
        assert_eq!((v.relation.as_str(), v.subject, v.certainty), ("signing-certificate-of", Some(7), Some("high")));
        // Another part of the file is not its certificate.
        assert_eq!(verdict(&catalog, (3, &agreement), (8, &certificate), None, ALL), None);
    }

    #[test]
    fn a_shared_datum_is_each_ones_own_and_not_a_number_cited() {
        // P-485: the letter's own shipment number is the first in reading
        // order; the other one is cited in its body.
        let catalog = Catalog::builtin();
        let receipt = known("acuse-de-recibo", None, &[("numero-de-envio", "XY123456789AR")]);
        let citing = known(
            "carta-documento",
            Some("enviada"),
            &[("numero-de-envio", "Q12345ZZ678"), ("numero-de-envio", "XY123456789AR")],
        );
        assert_eq!(verdict(&catalog, (1, &citing), (2, &receipt), None, 0.0), None);
        let own = known("carta-documento", Some("enviada"), &[("numero-de-envio", "XY123456789AR"), ("numero-de-envio", "Q12345ZZ678")]);
        let v = verdict(&catalog, (1, &own), (2, &receipt), None, 0.0).unwrap();
        assert_eq!((v.relation.as_str(), v.subject), ("delivery-receipt-of", Some(2)));
    }

    #[test]
    fn a_letter_said_sent_without_its_shipment_number_is_not_sent_and_can_be_the_preview() {
        // P-485: a stage needs its data. The model said "sent", but the
        // letter has no shipment number and no mark shows it was sent.
        let catalog = Catalog::builtin();
        let mut said_sent = known("carta-documento", Some("enviada"), &[("facturas", "0003-00001111")]);
        settle_stage(&catalog, &mut said_sent, &[]);
        assert_eq!(said_sent.stage.as_deref(), Some("vista-previa"));
        let draft = known("carta-documento", Some("borrador"), &[("facturas", "0003-00001111")]);
        let v = verdict(&catalog, (1, &draft), (2, &said_sent), Some("version"), ALL).unwrap();
        assert_eq!((v.relation.as_str(), v.subject), ("preview", Some(2)));

        // With its shipment number it was sent: no stage pair with a draft.
        let mut sent = known("carta-documento", Some("enviada"), &[("facturas", "0003-00001111"), ("numero-de-envio", "XY123456789AR")]);
        settle_stage(&catalog, &mut sent, &[]);
        assert_eq!(sent.stage.as_deref(), Some("enviada"));
        assert_eq!(verdict(&catalog, (1, &draft), (2, &sent), Some("version"), ALL), None);
        // A mark that shows it was sent wins over the missing number.
        let mut marked = known("carta-documento", Some("enviada"), &[("facturas", "0003-00001111")]);
        settle_stage(&catalog, &mut marked, &["enviada"]);
        assert_eq!(marked.stage.as_deref(), Some("enviada"));
        // A stage that needs nothing stays as said.
        let mut received = known("carta-documento", Some("recibida"), &[]);
        settle_stage(&catalog, &mut received, &[]);
        assert_eq!(received.stage.as_deref(), Some("recibida"));
    }
}
