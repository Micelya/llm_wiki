//! The records of a court a matter has: a record is data of a matter, not
//! the matter (`knowledge_matter_record`). A matter can have none, one or
//! several.
//!
//! When a record brought from a portal goes into the base, its matter is
//! looked for before one is made (`knowledge_integrate_record`), all of it
//! inside one immediate transaction: two integrations of the same record
//! never run at once.
//!
//! - An active matter that already has the record takes its documents,
//!   without asking.
//! - Otherwise one is made, as before, but not as named by a person: it is
//!   named by the record's official title, or by its number while there is
//!   none, and the name follows the title when it comes or changes. A name a
//!   person gave is never touched; a matter with several records is not
//!   renamed by itself.
//! - An active matter that looks like the record's, whatever its origin,
//!   becomes one question for a person per pair
//!   (`knowledge_matter_question`): «is it the same matter?», with the
//!   evidence. Two signs: it shares documents with what was brought (the
//!   same content, firm), or its documents cite the record's number (weak:
//!   citing is not being). Yes joins them into the matter that existed,
//!   which keeps a name a person gave and gets the record; no is
//!   remembered and not asked again.
//!
//! What the first version of the integration left is repaired by general
//! rules, with the record in hand: a matter it made bears the record's
//! reference as its name, or after a title in parentheses, marked as a
//! person's, and holds documents of the record; it gets the record and its
//! name stops being a person's. One of them with no decision of a person
//! whose documents are all in another matter of the same record is retired.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;

use super::matter_edit::put;
use super::vocabulary::vocabulary;
use crate::docbase::{now_ms, sql_err, DocBase};

/// Where a record came from: the integration of one brought from a portal.
pub const FROM_INTEGRATION: &str = "integration";

/// A record as the integration gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordInput {
    pub portal: String,
    pub jurisdiction: String,
    pub number: u64,
    pub year: u32,
    /// How the record's file writes it, when it differs from how the app
    /// does (`reference`).
    pub written: Option<String>,
    /// Its official title, when the portal gave one.
    pub title: Option<String>,
}

impl RecordInput {
    /// How the app writes it: "ABC 123/2020".
    pub fn reference(&self) -> String {
        reference(&self.jurisdiction, self.number as i64, self.year as i64)
    }

    fn title(&self) -> Option<String> {
        self.title.as_deref().map(collapse).filter(|t| !t.is_empty())
    }
}

/// A record of a matter, as its page shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterRecord {
    pub portal: String,
    pub jurisdiction: String,
    pub number: i64,
    pub year: i64,
    /// "ABC 123/2020".
    pub reference: String,
    pub title: Option<String>,
}

impl MatterRecord {
    /// The name of a matter that has only this record.
    fn matter_name(&self) -> String {
        self.title.clone().unwrap_or_else(|| self.reference.clone())
    }
}

/// A question for a person: whether a matter that looks like the one of a
/// record is the same.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterQuestion {
    pub record_matter_id: String,
    pub record_matter_name: String,
    /// The records of the first.
    pub records: Vec<MatterRecord>,
    pub other_matter_id: String,
    pub other_matter_name: String,
    /// "proposed" (by the grouping) or "person".
    pub other_origin: String,
    /// Documents of the record that are in the other matter too.
    pub shared: usize,
    /// Documents of the other matter that cite the record's number.
    pub citing: usize,
}

/// What an integration did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RecordOutcome {
    pub matter_id: String,
    /// The matter was made now.
    pub created: bool,
    /// Documents put in it now.
    pub added: usize,
    /// Matters of the same record retired, being copies of another.
    pub retired: Vec<String>,
    /// New questions for a person.
    pub questions: usize,
}

impl DocBase {
    /// The matter of a record brought from a portal, looked for before it is
    /// made, with the documents of the record (`contents`) put in it; the
    /// copies the first version left, retired; and a question for each
    /// matter that looks like it. `known` is the matter the record's file
    /// names.
    pub fn knowledge_integrate_record(&self, record: &RecordInput, known: Option<&str>, contents: &[i64]) -> Result<RecordOutcome, String> {
        let mut conn = self.conn();
        // The lock: a second integration of the same record waits here
        // until the first has its matter.
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(sql_err)?;
        let now = now_ms();
        let contents: BTreeSet<i64> = contents.iter().copied().collect();
        adopt(&tx, record, known, &contents, now)?;
        let retired = retire_copies(&tx, record, known, &contents, now)?;
        let (matter, created) = match own_matter(&tx, record, known)? {
            Some(id) => (id, false),
            None => (create(&tx, record, now)?, true),
        };
        attach(&tx, &matter, record, now)?;
        let chosen: BTreeSet<i64> = ids(&tx, "SELECT version_id FROM knowledge_matter_choice WHERE matter_id = ?1", &matter)?;
        let mut added = 0;
        for content in contents.iter().filter(|c| !chosen.contains(c)) {
            put(&tx, &matter, *content, now)?;
            added += 1;
        }
        if added > 0 {
            tx.execute(
                "UPDATE knowledge_matter SET confirmation = 'confirmed', decided_at = ?2, updated_at = ?2 WHERE id = ?1 AND confirmation = 'proposed'",
                params![matter, now],
            )
            .map_err(sql_err)?;
        }
        name_by_records(&tx, &matter, now)?;
        let questions = ask(&tx, record, &matter, &contents, now)?;
        if created || added > 0 || !retired.is_empty() || questions > 0 {
            super::mark_dirty(&tx, "project", "matters")?;
        }
        tx.commit().map_err(sql_err)?;
        Ok(RecordOutcome { matter_id: matter, created, added, retired, questions })
    }

    /// The answer of a person to whether a matter that looks like the one
    /// of a record is the same: yes joins the record's matter into the
    /// other, which keeps a name a person gave and gets the record; no is
    /// remembered.
    pub fn knowledge_answer_matter_question(&self, record_matter: &str, other_matter: &str, same: bool) -> Result<(), String> {
        let conn = self.conn();
        let answer: Option<Option<String>> = conn
            .query_row(
                "SELECT answer FROM knowledge_matter_question WHERE record_matter = ?1 AND other_matter = ?2",
                params![record_matter, other_matter],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        match answer {
            None => return Err("Esa pregunta ya no existe".to_string()),
            Some(Some(_)) => return Err("Esa pregunta ya tiene respuesta".to_string()),
            Some(None) => {}
        }
        let now = now_ms();
        let tx = conn.unchecked_transaction().map_err(sql_err)?;
        if same {
            for id in [record_matter, other_matter] {
                if !active(&tx, id)? {
                    return Err("Uno de los dos ya no está activo".to_string());
                }
            }
        }
        tx.execute(
            "UPDATE knowledge_matter_question SET answer = ?3, answered_at = ?4 WHERE record_matter = ?1 AND other_matter = ?2",
            params![record_matter, other_matter, if same { "same" } else { "different" }, now],
        )
        .map_err(sql_err)?;
        if same {
            join(&tx, record_matter, other_matter, now)?;
            super::mark_dirty(&tx, "project", "matters")?;
        }
        tx.commit().map_err(sql_err)?;
        if same {
            let decided = super::resolve::load_decided(&conn)?;
            super::matters::group(&conn, vocabulary(), &decided.joins, &decided.separations)?;
        }
        Ok(())
    }
}

/// The records of a matter.
pub(crate) fn records_of(conn: &Connection, matter: &str) -> Result<Vec<MatterRecord>, String> {
    let mut statement = conn
        .prepare_cached(
            "SELECT portal, jurisdiction, number, year, title FROM knowledge_matter_record WHERE matter_id = ?1
             ORDER BY added_at, jurisdiction, year, number",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([matter], |row| {
            let (portal, jurisdiction, number, year, title): (String, String, i64, i64, Option<String>) =
                (row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?);
            Ok(MatterRecord {
                reference: reference(&jurisdiction, number, year),
                title: title.map(|t| collapse(&t)).filter(|t| !t.is_empty()),
                portal,
                jurisdiction,
                number,
                year,
            })
        })
        .map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// The name the records give a matter: the title of its only record (or
/// its number while there is none); the name it has when it has several
/// (it is not renamed by itself); none when it has none.
pub(crate) fn name_from_records(conn: &Connection, matter: &str) -> Result<Option<String>, String> {
    let records = records_of(conn, matter)?;
    match records.len() {
        0 => Ok(None),
        1 => Ok(Some(records[0].matter_name())),
        _ => conn.query_row("SELECT name FROM knowledge_matter WHERE id = ?1", [matter], |row| row.get(0)).optional().map_err(sql_err),
    }
}

/// The questions no person answered yet, between two active and open
/// matters, when one of them has a document `visible` lets see.
pub(crate) fn open_questions(conn: &Connection, visible: &BTreeSet<i64>) -> Result<Vec<MatterQuestion>, String> {
    let rows: Vec<(String, String, String, String, String, i64, i64)> = {
        let mut statement = conn
            .prepare(
                "SELECT q.record_matter, a.name, q.other_matter, b.name, b.origin, q.shared, q.citing
                 FROM knowledge_matter_question q
                 JOIN knowledge_matter a ON a.id = q.record_matter
                 JOIN knowledge_matter b ON b.id = q.other_matter
                 WHERE q.answer IS NULL AND a.state = 'active' AND b.state = 'active' AND a.status = 'open' AND b.status = 'open'
                   AND a.confirmation != 'discarded' AND b.confirmation != 'discarded'
                 ORDER BY q.shared DESC, q.citing DESC, q.asked_at, q.record_matter, q.other_matter",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)))
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut out = Vec::new();
    for (record_matter, record_name, other, other_name, other_origin, shared, citing) in rows {
        let seen = [&record_matter, &other]
            .into_iter()
            .map(|m| documents_of(conn, m))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|docs| docs.iter().any(|d| visible.contains(d)));
        if !seen {
            continue;
        }
        out.push(MatterQuestion {
            records: records_of(conn, &record_matter)?,
            record_matter_id: record_matter,
            record_matter_name: record_name,
            other_matter_id: other,
            other_matter_name: other_name,
            other_origin,
            shared: shared as usize,
            citing: citing as usize,
        });
    }
    Ok(out)
}

fn reference(jurisdiction: &str, number: i64, year: i64) -> String {
    format!("{jurisdiction} {number}/{year}")
}

fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn ids(conn: &Connection, sql: &str, matter: &str) -> Result<BTreeSet<i64>, String> {
    let mut statement = conn.prepare_cached(sql).map_err(sql_err)?;
    let rows = statement.query_map([matter], |row| row.get(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// Every document of a matter: those it has and those a person put in.
fn documents_of(conn: &Connection, matter: &str) -> Result<BTreeSet<i64>, String> {
    ids(
        conn,
        "SELECT version_id FROM knowledge_matter_document WHERE matter_id = ?1
         UNION SELECT version_id FROM knowledge_matter_choice WHERE matter_id = ?1 AND choice = 'in'",
        matter,
    )
}

fn active(conn: &Connection, matter: &str) -> Result<bool, String> {
    let state: Option<String> = conn
        .query_row("SELECT state FROM knowledge_matter WHERE id = ?1", [matter], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(state.as_deref() == Some("active"))
}

/// The active matters that have the record, the oldest first.
fn matters_with(conn: &Connection, record: &RecordInput) -> Result<Vec<String>, String> {
    let mut statement = conn
        .prepare_cached(
            "SELECT m.id FROM knowledge_matter m JOIN knowledge_matter_record r ON r.matter_id = m.id
             WHERE m.state = 'active' AND r.portal = ?1 AND r.jurisdiction = ?2 AND r.number = ?3 AND r.year = ?4
             ORDER BY m.created_at, m.id",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map(params![record.portal, record.jurisdiction, record.number as i64, record.year as i64], |row| row.get(0))
        .map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// Gives a matter the record, and keeps its official title up to date.
fn attach(conn: &Connection, matter: &str, record: &RecordInput, now: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO knowledge_matter_record (matter_id, portal, jurisdiction, number, year, title, source, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT (matter_id, portal, jurisdiction, number, year) DO UPDATE SET title = coalesce(excluded.title, title)",
        params![matter, record.portal, record.jurisdiction, record.number as i64, record.year as i64, record.title(), FROM_INTEGRATION, now],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// The matters the first version of the integration made for this record:
/// made as a person's, named by the record's reference or by a title with
/// the reference in parentheses, with documents of the record (or the one
/// its file names). They get the record, and their name stops being a
/// person's.
fn adopt(conn: &Connection, record: &RecordInput, known: Option<&str>, contents: &BTreeSet<i64>, now: i64) -> Result<(), String> {
    let forms: Vec<String> = std::iter::once(record.reference()).chain(record.written.iter().map(|w| collapse(w))).collect();
    let named: Vec<String> = {
        let mut statement = conn
            .prepare("SELECT id, name FROM knowledge_matter WHERE state = 'active' AND origin = 'person' AND name_by_person = 1")
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        let mut out = Vec::new();
        for row in rows {
            let (id, name) = row.map_err(sql_err)?;
            if forms.iter().any(|form| name == *form || name.ends_with(&format!(" ({form})"))) {
                out.push(id);
            }
        }
        out
    };
    for id in named {
        let holds = known == Some(id.as_str()) || documents_of(conn, &id)?.iter().any(|c| contents.contains(c));
        if !holds {
            continue;
        }
        attach(conn, &id, record, now)?;
        conn.execute("UPDATE knowledge_matter SET name_by_person = 0, updated_at = ?2 WHERE id = ?1", params![id, now]).map_err(sql_err)?;
    }
    Ok(())
}

/// Whether nothing of a matter of the record is a person's decision: made
/// by the integration and left as it was (no other record, no name, no
/// status, no document taken out, no document put in but the record's).
fn untouched(conn: &Connection, matter: &str, contents: &BTreeSet<i64>) -> Result<bool, String> {
    let (origin, name_by_person, status, confirmation): (String, i64, String, String) = conn
        .query_row("SELECT origin, name_by_person, status, confirmation FROM knowledge_matter WHERE id = ?1", [matter], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(sql_err)?;
    if origin != "person" || name_by_person != 0 || status != "open" || confirmation != "confirmed" {
        return Ok(false);
    }
    let records: i64 = conn
        .query_row("SELECT count(*) FROM knowledge_matter_record WHERE matter_id = ?1", [matter], |row| row.get(0))
        .map_err(sql_err)?;
    let out: i64 = conn
        .query_row("SELECT count(*) FROM knowledge_matter_choice WHERE matter_id = ?1 AND choice = 'out'", [matter], |row| row.get(0))
        .map_err(sql_err)?;
    let put_in = ids(conn, "SELECT version_id FROM knowledge_matter_choice WHERE matter_id = ?1 AND choice = 'in'", matter)?;
    let answered: i64 = conn
        .query_row(
            "SELECT count(*) FROM knowledge_matter_question WHERE answer IS NOT NULL AND (record_matter = ?1 OR other_matter = ?1)",
            [matter],
            |row| row.get(0),
        )
        .map_err(sql_err)?;
    Ok(records == 1 && out == 0 && answered == 0 && put_in.iter().all(|c| contents.contains(c)))
}

/// Retires each matter of the record nobody decided about whose documents
/// are all in another matter of the same record: a copy two integrations
/// made at once. Of two with the same documents, the one the record's file
/// names stays, or else the oldest. One a person touched stays, and becomes
/// a question.
fn retire_copies(conn: &Connection, record: &RecordInput, known: Option<&str>, contents: &BTreeSet<i64>, now: i64) -> Result<Vec<String>, String> {
    let matters = matters_with(conn, record)?;
    if matters.len() < 2 {
        return Ok(Vec::new());
    }
    let documents: BTreeMap<String, BTreeSet<i64>> = matters.iter().map(|m| Ok((m.clone(), documents_of(conn, m)?))).collect::<Result<_, String>>()?;
    let rank = |m: &str| (known != Some(m), matters.iter().position(|x| x == m).unwrap_or(usize::MAX));
    let mut retired: Vec<String> = Vec::new();
    // The smallest and the least kept go first.
    let mut order = matters.clone();
    order.sort_by(|a, b| documents[a].len().cmp(&documents[b].len()).then_with(|| rank(b).cmp(&rank(a))));
    for matter in &order {
        let mine = &documents[matter];
        let within = matters.iter().filter(|o| *o != matter && !retired.contains(o)).any(|other| {
            let theirs = &documents[other];
            mine.is_subset(theirs) && (mine.len() < theirs.len() || rank(other) < rank(matter))
        });
        if within && untouched(conn, matter, contents)? {
            conn.execute("UPDATE knowledge_matter SET state = 'retired', updated_at = ?2 WHERE id = ?1", params![matter, now]).map_err(sql_err)?;
            conn.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1", [matter]).map_err(sql_err)?;
            conn.execute("DELETE FROM knowledge_matter_record WHERE matter_id = ?1", [matter]).map_err(sql_err)?;
            conn.execute("DELETE FROM knowledge_matter_question WHERE answer IS NULL AND (record_matter = ?1 OR other_matter = ?1)", [matter])
                .map_err(sql_err)?;
            retired.push(matter.clone());
        }
    }
    Ok(retired)
}

/// The matter of the record: the one its file names while it is active,
/// or else the active one that has it with the most documents (the oldest
/// of them).
fn own_matter(conn: &Connection, record: &RecordInput, known: Option<&str>) -> Result<Option<String>, String> {
    if let Some(id) = known {
        if active(conn, id)? {
            return Ok(Some(id.to_string()));
        }
    }
    let mut best: Option<(usize, String)> = None;
    for matter in matters_with(conn, record)? {
        let size = documents_of(conn, &matter)?.len();
        if best.as_ref().is_none_or(|(n, _)| size > *n) {
            best = Some((size, matter));
        }
    }
    Ok(best.map(|(_, id)| id))
}

/// Makes the record's matter: confirmed, made at a person's request (to
/// follow the record), named by the record and not by a person.
fn create(conn: &Connection, record: &RecordInput, now: i64) -> Result<String, String> {
    let mut id = format!("mat-p-{now:x}");
    let mut n = 2;
    while conn.query_row("SELECT 1 FROM knowledge_matter WHERE id = ?1", [&id], |_| Ok(())).optional().map_err(sql_err)?.is_some() {
        id = format!("mat-p-{now:x}-{n}");
        n += 1;
    }
    let name = record.title().unwrap_or_else(|| record.reference());
    conn.execute(
        "INSERT INTO knowledge_matter (id, name, state, created_at, updated_at, status, origin, confirmation, name_by_person, decided_at)
         VALUES (?1, ?2, 'active', ?3, ?3, 'open', 'person', 'confirmed', 0, ?3)",
        params![id, name, now],
    )
    .map_err(sql_err)?;
    Ok(id)
}

/// Names a matter by its records, unless a person named it.
fn name_by_records(conn: &Connection, matter: &str, now: i64) -> Result<(), String> {
    let (name, by_person): (String, i64) = conn
        .query_row("SELECT name, name_by_person FROM knowledge_matter WHERE id = ?1", [matter], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(sql_err)?;
    if by_person != 0 {
        return Ok(());
    }
    if let Some(wanted) = name_from_records(conn, matter)? {
        if wanted != name {
            conn.execute("UPDATE knowledge_matter SET name = ?2, updated_at = ?3 WHERE id = ?1", params![matter, wanted, now]).map_err(sql_err)?;
        }
    }
    Ok(())
}

/// One question per pair for each active matter that looks like the
/// record's: it shares documents with what was brought, or its documents
/// cite the record's number. Returns how many are new; one already
/// answered is never asked again, one still open gets its evidence again.
fn ask(conn: &Connection, record: &RecordInput, own: &str, contents: &BTreeSet<i64>, now: i64) -> Result<usize, String> {
    let mut shared: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
    let mut citing: BTreeMap<String, BTreeSet<i64>> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare_cached(
                "SELECT matter_id FROM knowledge_matter_document WHERE version_id = ?1
                 UNION SELECT matter_id FROM knowledge_matter_choice WHERE version_id = ?1 AND choice = 'in'",
            )
            .map_err(sql_err)?;
        for content in contents {
            let rows = statement.query_map([content], |row| row.get::<_, String>(0)).map_err(sql_err)?;
            for row in rows {
                shared.entry(row.map_err(sql_err)?).or_default().insert(*content);
            }
        }
        let cite = super::references::documents_citing(conn, vocabulary(), record.number, record.year)?;
        let mut statement = conn.prepare_cached("SELECT matter_id FROM knowledge_matter_document WHERE version_id = ?1").map_err(sql_err)?;
        for content in &cite {
            let rows = statement.query_map([content], |row| row.get::<_, String>(0)).map_err(sql_err)?;
            for row in rows {
                citing.entry(row.map_err(sql_err)?).or_default().insert(*content);
            }
        }
    }
    let candidates: BTreeSet<&String> = shared.keys().chain(citing.keys()).filter(|m| m.as_str() != own).collect();
    let mut new = 0;
    for other in candidates {
        let looks: Option<(String, String, String)> = conn
            .query_row("SELECT state, status, confirmation FROM knowledge_matter WHERE id = ?1", [other], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .optional()
            .map_err(sql_err)?;
        if !looks.is_some_and(|(state, status, confirmation)| state == "active" && status == "open" && confirmation != "discarded") {
            continue;
        }
        let (n_shared, n_citing) = (shared.get(other).map_or(0, |s| s.len()) as i64, citing.get(other).map_or(0, |s| s.len()) as i64);
        let asked: Option<(String, String, Option<String>)> = conn
            .query_row(
                "SELECT record_matter, other_matter, answer FROM knowledge_matter_question
                 WHERE (record_matter = ?1 AND other_matter = ?2) OR (record_matter = ?2 AND other_matter = ?1)",
                params![own, other],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(sql_err)?;
        match asked {
            None => {
                conn.execute(
                    "INSERT INTO knowledge_matter_question (record_matter, other_matter, shared, citing, answer, asked_at) VALUES (?1, ?2, ?3, ?4, NULL, ?5)",
                    params![own, other, n_shared, n_citing, now],
                )
                .map_err(sql_err)?;
                new += 1;
            }
            Some((a, b, None)) => {
                conn.execute(
                    "UPDATE knowledge_matter_question SET shared = ?3, citing = ?4 WHERE record_matter = ?1 AND other_matter = ?2",
                    params![a, b, n_shared, n_citing],
                )
                .map_err(sql_err)?;
            }
            Some(_) => {}
        }
    }
    Ok(new)
}

/// Joins the record's matter into the other: its documents (but those a
/// person took out of the other) and its records go to the other, which is
/// confirmed, and it is retired. The other keeps a name a person gave; else
/// it is named by its records.
fn join(conn: &Connection, from: &str, into: &str, now: i64) -> Result<(), String> {
    let taken_out = ids(conn, "SELECT version_id FROM knowledge_matter_choice WHERE matter_id = ?1 AND choice = 'out'", into)?;
    for content in documents_of(conn, from)? {
        if !taken_out.contains(&content) {
            put(conn, into, content, now)?;
        }
    }
    conn.execute(
        "UPDATE knowledge_matter SET confirmation = 'confirmed', decided_at = ?2, decided_documents = NULL, updated_at = ?2 WHERE id = ?1 AND confirmation != 'confirmed'",
        params![into, now],
    )
    .map_err(sql_err)?;
    conn.execute(
        "INSERT OR IGNORE INTO knowledge_matter_record (matter_id, portal, jurisdiction, number, year, title, source, added_at)
         SELECT ?2, portal, jurisdiction, number, year, title, source, added_at FROM knowledge_matter_record WHERE matter_id = ?1",
        params![from, into],
    )
    .map_err(sql_err)?;
    conn.execute("DELETE FROM knowledge_matter_record WHERE matter_id = ?1", [from]).map_err(sql_err)?;
    conn.execute("UPDATE knowledge_matter SET state = 'retired', updated_at = ?2 WHERE id = ?1", params![from, now]).map_err(sql_err)?;
    conn.execute("DELETE FROM knowledge_matter_document WHERE matter_id = ?1", [from]).map_err(sql_err)?;
    // The questions still open about the one retired are about the other now.
    conn.execute(
        "UPDATE OR IGNORE knowledge_matter_question SET record_matter = ?2 WHERE record_matter = ?1 AND answer IS NULL AND other_matter != ?2",
        params![from, into],
    )
    .map_err(sql_err)?;
    conn.execute(
        "UPDATE OR IGNORE knowledge_matter_question SET other_matter = ?2 WHERE other_matter = ?1 AND answer IS NULL AND record_matter != ?2",
        params![from, into],
    )
    .map_err(sql_err)?;
    conn.execute("DELETE FROM knowledge_matter_question WHERE answer IS NULL AND (record_matter = ?1 OR other_matter = ?1)", [from])
        .map_err(sql_err)?;
    name_by_records(conn, into, now)
}
