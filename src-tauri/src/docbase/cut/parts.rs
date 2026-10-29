//! From what the cutter says to the documents (D-560): when the steps of
//! a PDF give two parts or more, each part is a version of its own, with
//! the text of its pages joined as the text of the whole file is (the
//! `## Page N` headings keep the numbers of the file), and an appearance
//! `pages:first-last` on the file. The version of the whole file stays,
//! with its appearance `file`: the joined PDF is a document of its own.
//!
//! - A cut of the rules or of the model is `proposed`; one a person made
//!   (`cut_decision`), `confirmed`. What a person says wins over the rules
//!   and the model, and is applied again when the file is cut again.
//! - A cut goes on without waiting for a person (D-603): a part with a
//!   step that nothing backs (`CutStep::uncertain`) is `uncertain`, and
//!   so is what comes out of it (copies and versions, families, matters).
//!   The appearance `file` of a cut PDF is uncertain when any of its
//!   steps is, also for a PDF of one part.
//! - A part with the text of a version that already exists is another
//!   appearance of that version.
//! - When the cut changes, the parts with the same pages and the same text
//!   keep their version (and its number); those that go away go as a
//!   version goes (`knowledge_gone`); the new ones are made.
//! - A PDF with one part changes nothing.
//!
//! The parts are made once the file has its text, also while some of its
//! pages wait for recognition: a part with too little text waits with them
//! (`needs-recognition`), and when they are read the parts whose text
//! changed are made again.

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::source_volume::catalog::{fingerprint, is_page_heading};

use super::CutStep;
use crate::docbase::{now_ms, sql_err, DocBase};

/// What a person says of a page: it starts another document, or it goes
/// on with the one before it.
pub const STARTS: &str = "starts";
pub const CONTINUES: &str = "continues";

/// A document a joined PDF includes, as the detail of that PDF shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IncludedPart {
    pub position: i64,
    pub version_id: i64,
    pub document_id: i64,
    pub first_page: u32,
    pub last_page: u32,
    /// `rule`, `model` or `person`.
    pub cut_by: String,
    /// `proposed` or `confirmed`.
    pub cut_state: String,
    /// The pages of the part whose step from the page before nothing
    /// backs and no person decided: for a person to look at.
    pub doubtful_pages: Vec<u32>,
    /// Whether a step that bounds the part, or one inside it, has nothing
    /// to back it (D-603): what decides how its cut is shown.
    pub uncertain: bool,
    /// The whole file, for a PDF of one part with uncertain steps: there
    /// is no other document to open.
    pub whole: bool,
}

/// A part of a file as the steps and a person's word give it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Part {
    pub first: u32,
    pub last: u32,
    pub by: &'static str,
    pub confirmed: bool,
    pub uncertain: bool,
}

/// Whether `page` starts a document, who says so, and whether nothing
/// backs it (a step not made yet is not backed either).
fn said(steps: &BTreeMap<u32, &CutStep>, decisions: &BTreeMap<u32, bool>, page: u32) -> (bool, &'static str, bool) {
    if let Some(&starts) = decisions.get(&page) {
        return (starts, "person", false);
    }
    match steps.get(&(page - 1)) {
        Some(s) if s.by_model() => (s.starts(), "model", s.uncertain()),
        Some(s) => (s.starts(), "rule", s.uncertain()),
        None => (false, "rule", true),
    }
}

/// The parts of a file of `pages` pages. A part is cut by whoever decided
/// the steps that make it (its first page, the pages inside, the page
/// after it): a person when a person decided them all, and then it is
/// confirmed; the model when the model decided one; the rules otherwise.
/// It is uncertain when one of those steps has nothing to back it.
pub(crate) fn partition(pages: u32, steps: &[CutStep], decisions: &BTreeMap<u32, bool>) -> Vec<Part> {
    let by_after: BTreeMap<u32, &CutStep> = steps.iter().map(|s| (s.after, s)).collect();
    let says: BTreeMap<u32, (bool, &'static str, bool)> = (2..=pages).map(|p| (p, said(&by_after, decisions, p))).collect();
    let mut starts = vec![1];
    starts.extend(says.iter().filter(|(_, (starts, ..))| *starts).map(|(page, _)| *page));
    starts
        .iter()
        .enumerate()
        .map(|(index, &first)| {
            let last = starts.get(index + 1).map(|next| next - 1).unwrap_or(pages);
            let bounds = first.max(2)..=(last + 1).min(pages);
            let who: Vec<&str> = bounds.clone().map(|p| says[&p].1).collect();
            let uncertain = bounds.into_iter().any(|p| says[&p].2);
            let confirmed = !who.is_empty() && who.iter().all(|w| *w == "person");
            let by = if confirmed {
                "person"
            } else if who.contains(&"model") {
                "model"
            } else {
                "rule"
            };
            Part { first, last, by, confirmed, uncertain }
        })
        .collect()
}

/// The text of pages `first` to `last` of a text joined by pages, as the
/// text of a whole file is: each page with its heading and its number.
pub(crate) fn pages_text(text: &str, first: u32, last: u32) -> String {
    let mut page: Option<u32> = None;
    let mut out: Vec<&str> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if is_page_heading(trimmed) {
            page = trimmed["## Page ".len()..].parse().ok();
        }
        if page.is_some_and(|p| p >= first && p <= last) {
            out.push(line);
        }
    }
    out.join("\n").trim_end().to_string()
}

/// SQL: the version in `column` is the version of a whole file that has
/// included parts. It is not read whole: its parts are (D-560).
pub(crate) fn has_included_sql(column: &str) -> String {
    format!(
        "EXISTS (SELECT 1 FROM appearance wa JOIN appearance wp ON wp.resource_hash = wa.resource_hash AND wp.selector <> 'file'
                 WHERE wa.version_id = {column} AND wa.selector = 'file')"
    )
}

/// The versions that appear only in uncertain cuts. What comes out of them
/// (copies and versions, families, matters, templates) goes on without
/// waiting for a person, marked as uncertain (D-603): not being able to
/// tell is never the same as knowing.
pub(crate) fn only_uncertain(conn: &Connection) -> Result<BTreeSet<i64>, String> {
    let mut statement = conn.prepare(&format!("SELECT version_id FROM ({UNCERTAIN_VERSIONS})")).map_err(sql_err)?;
    let rows = statement.query_map([], |row| row.get(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// SQL: the versions that appear only in uncertain cuts.
pub(crate) const UNCERTAIN_VERSIONS: &str = "SELECT version_id FROM appearance GROUP BY version_id HAVING min(uncertain) = 1";

/// SQL: the version in `column` appears only in uncertain cuts.
pub(crate) fn only_uncertain_sql(column: &str) -> String {
    format!("{column} IN ({UNCERTAIN_VERSIONS})")
}

/// What the parts of a file are made from: its version and the time of
/// its text, its steps and the model's answers, and a person's word (both
/// tables are read in the order of their key).
const BASIS_SQL: &str = "c.id || ':' || d.created_at
    || ':' || (SELECT max(s.created_at) FROM cut_step s WHERE s.resource_hash = f.hash)
    || ':' || coalesce((SELECT group_concat(s.after_page || '=' || coalesce(s.model_says, '') || '/' || coalesce(s.model_confidence, ''), ',') FROM cut_step s
                        WHERE s.resource_hash = f.hash AND (s.verdict = 'doubtful' OR s.rule = 'uniform')), '')
    || ':' || coalesce((SELECT group_concat(k.page || k.decision, ',') FROM cut_decision k WHERE k.resource_hash = f.hash), '')";

/// The pages for a person to look at: those whose step from the page
/// before nothing backs, and no person decided (D-603).
fn to_look_at(steps: &[CutStep], decided: &BTreeMap<u32, bool>) -> BTreeSet<u32> {
    steps
        .iter()
        .filter(|step| step.after > 0 && step.uncertain())
        .map(|step| step.after + 1)
        .filter(|page| !decided.contains_key(page))
        .collect()
}

fn decisions(conn: &Connection, hash: &str) -> Result<BTreeMap<u32, bool>, String> {
    let mut statement = conn.prepare("SELECT page, decision FROM cut_decision WHERE resource_hash = ?1").map_err(sql_err)?;
    let rows = statement
        .query_map([hash], |row| Ok((row.get::<_, u32>(0)?, row.get::<_, String>(1)? == STARTS)))
        .map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

struct Existing {
    id: i64,
    version_id: i64,
    selector: String,
    text_hash: Option<String>,
    body: Option<String>,
}

impl DocBase {
    /// Makes again the versions of the parts of up to `limit` cut files
    /// whose version, text, steps or a person's word changed since they
    /// were last made. Returns how many files were looked at.
    pub fn cut_parts_pending(&self, limit: usize) -> Result<usize, String> {
        let jobs: Vec<(String, String)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(&format!(
                    "SELECT f.hash, {BASIS_SQL} AS basis FROM file f
                     JOIN version c ON c.id = f.version_id AND c.state IN ('processed', 'needs-recognition')
                     JOIN derivative d ON d.version_id = c.id AND d.kind = 'text'
                     WHERE EXISTS (SELECT 1 FROM cut_step s WHERE s.resource_hash = f.hash)
                       AND basis IS NOT (SELECT a.basis FROM cut_applied a WHERE a.resource_hash = f.hash)
                     ORDER BY f.hash LIMIT ?1"
                ))
                .map_err(sql_err)?;
            let rows = statement.query_map([limit as i64], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (hash, basis) in &jobs {
            self.make_parts(hash, basis)?;
        }
        Ok(jobs.len())
    }

    fn make_parts(&self, hash: &str, basis: &str) -> Result<(), String> {
        let steps = self.cut_steps(hash)?;
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        let whole: Option<(i64, String, String, String)> = tx
            .query_row(
                "SELECT c.id, d.body, d.engine, c.state FROM file f JOIN version c ON c.id = f.version_id
                 JOIN derivative d ON d.version_id = c.id AND d.kind = 'text' WHERE f.hash = ?1",
                [hash],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((whole_id, text, engine, whole_state)) = whole else {
            return Ok(());
        };
        let pages = steps.iter().map(|s| s.after).max().unwrap_or(0) + 1;
        let mut parts = partition(pages, &steps, &decisions(&tx, hash)?);
        // The whole file is uncertain when any of its steps is: for a PDF
        // of one part, what is said of its only document.
        tx.execute(
            "UPDATE appearance SET uncertain = ?1 WHERE resource_hash = ?2 AND selector = 'file'",
            params![parts.iter().any(|p| p.uncertain), hash],
        )
        .map_err(sql_err)?;
        if parts.len() < 2 {
            parts.clear();
        }
        let existing: Vec<Existing> = {
            let mut statement = tx
                .prepare(
                    "SELECT a.id, a.version_id, a.selector, v.text_hash, d.body FROM appearance a
                     JOIN version v ON v.id = a.version_id
                     LEFT JOIN derivative d ON d.version_id = a.version_id AND d.kind = 'text'
                     WHERE a.resource_hash = ?1 AND a.selector <> 'file'",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([hash], |row| {
                    Ok(Existing { id: row.get(0)?, version_id: row.get(1)?, selector: row.get(2)?, text_hash: row.get(3)?, body: row.get(4)? })
                })
                .map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        let now = now_ms();
        let state = |part: &Part| if part.confirmed { "confirmed" } else { "proposed" };
        let mut kept: BTreeSet<i64> = BTreeSet::new();
        let mut to_make = Vec::new();
        for (index, part) in parts.iter().enumerate() {
            let selector = format!("pages:{}-{}", part.first, part.last);
            let body = pages_text(&text, part.first, part.last);
            let identity = fingerprint(&body).map(|(hash, _)| hash);
            let same = existing.iter().find(|e| {
                e.selector == selector
                    && ((e.text_hash.is_some() && e.text_hash == identity) || e.body.as_deref() == Some(body.as_str()))
            });
            match same {
                Some(e) => {
                    kept.insert(e.id);
                    tx.execute(
                        "UPDATE appearance SET position = ?1, cut_by = ?2, cut_state = ?3, uncertain = ?4 WHERE id = ?5",
                        params![index as i64 + 1, part.by, state(part), part.uncertain, e.id],
                    )
                    .map_err(sql_err)?;
                }
                None => to_make.push((index, part, selector, body, identity)),
            }
        }
        let mut left: BTreeSet<i64> = BTreeSet::new();
        for e in existing.iter().filter(|e| !kept.contains(&e.id)) {
            tx.execute("DELETE FROM appearance WHERE id = ?1", [e.id]).map_err(sql_err)?;
            left.insert(e.version_id);
        }
        for (index, part, selector, body, identity) in to_make {
            let twin: Option<i64> = match &identity {
                Some(identity) => tx
                    .query_row(
                        "SELECT id FROM version WHERE text_hash = ?1 AND id != ?2 ORDER BY id LIMIT 1",
                        params![identity, whole_id],
                        |row| row.get(0),
                    )
                    .optional()
                    .map_err(sql_err)?,
                None => None,
            };
            let version_id = match twin {
                Some(twin) => twin,
                None => {
                    // An id is never given twice (`identity.rs`).
                    tx.execute(
                        "INSERT INTO version (id, state, first_seen_at) VALUES (
                             (SELECT max(n) + 1 FROM (SELECT coalesce(max(id), 0) AS n FROM version
                                                      UNION ALL SELECT coalesce(max(version_id), 0) FROM knowledge_gone)),
                             'pending', ?1)",
                        [now],
                    )
                    .map_err(sql_err)?;
                    let id = tx.last_insert_rowid();
                    // Some pages of the file wait for recognition: a part
                    // with too little text to tell it waits with them.
                    let read = whole_state == "processed" || identity.is_some();
                    crate::docbase::text::store(&tx, id, &body, &engine, if read { "processed" } else { "needs-recognition" })?
                }
            };
            tx.execute(
                "INSERT INTO appearance (version_id, resource_hash, selector, position, cut_by, cut_state, uncertain, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![version_id, hash, selector, index as i64 + 1, part.by, state(part), part.uncertain, now],
            )
            .map_err(sql_err)?;
            left.remove(&version_id);
        }
        for version_id in left {
            let used: bool = tx
                .query_row(
                    "SELECT EXISTS (SELECT 1 FROM appearance WHERE version_id = ?1) OR EXISTS (SELECT 1 FROM file WHERE version_id = ?1)",
                    [version_id],
                    |row| row.get(0),
                )
                .map_err(sql_err)?;
            if !used {
                tx.execute("DELETE FROM content_fts WHERE rowid = ?1", [version_id]).map_err(sql_err)?;
                tx.execute("DELETE FROM version WHERE id = ?1", [version_id]).map_err(sql_err)?;
            }
        }
        crate::docbase::search::forget_gone(&tx)?;
        tx.execute("INSERT OR REPLACE INTO cut_applied (resource_hash, basis) VALUES (?1, ?2)", params![hash, basis])
            .map_err(sql_err)?;
        tx.commit().map_err(sql_err)
    }

    /// The documents a joined PDF includes, in order, with their pages,
    /// the state of their cut and who cut it; none for a file of one part.
    pub fn included_parts(&self, hash: &str) -> Result<Vec<IncludedPart>, String> {
        let steps = self.cut_steps(hash)?;
        let conn = self.conn();
        let decided = decisions(&conn, hash)?;
        let doubtful = to_look_at(&steps, &decided);
        let mut statement = conn
            .prepare(
                "SELECT a.position, a.version_id, v.document_id, a.selector, a.cut_by, a.cut_state, a.uncertain FROM appearance a
                 JOIN version v ON v.id = a.version_id
                 WHERE a.resource_hash = ?1 AND a.selector <> 'file' ORDER BY a.position",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([hash], |row| {
                let selector: String = row.get(3)?;
                let (first, last) = selector
                    .strip_prefix("pages:")
                    .and_then(|r| r.split_once('-'))
                    .map(|(a, b)| (a.parse().unwrap_or(0), b.parse().unwrap_or(0)))
                    .unwrap_or((0, 0));
                Ok(IncludedPart {
                    position: row.get(0)?,
                    version_id: row.get(1)?,
                    document_id: row.get(2)?,
                    first_page: first,
                    last_page: last,
                    cut_by: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                    cut_state: row.get(5)?,
                    doubtful_pages: Vec::new(),
                    uncertain: row.get(6)?,
                    whole: false,
                })
            })
            .map_err(sql_err)?;
        let mut parts: Vec<IncludedPart> = rows.collect::<Result<_, _>>().map_err(sql_err)?;
        for part in &mut parts {
            part.doubtful_pages = doubtful.range(part.first_page..=part.last_page).copied().collect();
        }
        Ok(parts)
    }

    /// What the strip of pages of a file shows: its parts, or, for a PDF
    /// of one part with uncertain steps, the whole file as its only part,
    /// with those pages to look at (D-603). Its appearance stays
    /// `confirmed` by its constraint; the mark decides how it is shown.
    pub fn strip_parts(&self, hash: &str) -> Result<Vec<IncludedPart>, String> {
        let parts = self.included_parts(hash)?;
        if !parts.is_empty() {
            return Ok(parts);
        }
        let Some(pages) = self.cut_pages(hash)? else {
            return Ok(parts);
        };
        let whole: Option<(i64, i64)> = self
            .conn()
            .query_row(
                "SELECT a.version_id, v.document_id FROM appearance a JOIN version v ON v.id = a.version_id
                 WHERE a.resource_hash = ?1 AND a.selector = 'file' AND a.uncertain = 1",
                [hash],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((version_id, document_id)) = whole else {
            return Ok(parts);
        };
        let steps = self.cut_steps(hash)?;
        let decided = decisions(&self.conn(), hash)?;
        let by = partition(pages, &steps, &decided).first().map(|p| p.by).unwrap_or("rule");
        Ok(vec![IncludedPart {
            position: 1,
            version_id,
            document_id,
            first_page: 1,
            last_page: pages,
            cut_by: by.into(),
            cut_state: "proposed".into(),
            doubtful_pages: to_look_at(&steps, &decided).into_iter().collect(),
            uncertain: true,
            whole: true,
        }])
    }

    /// The number of pages of a cut file: none while it is not cut.
    fn cut_pages(&self, hash: &str) -> Result<Option<u32>, String> {
        self.conn()
            .query_row("SELECT max(after_page) + 1 FROM cut_step WHERE resource_hash = ?1", [hash], |row| row.get(0))
            .map_err(sql_err)
    }

    fn decide(&self, hash: &str, page: u32, decision: &str) -> Result<(), String> {
        let pages = self.cut_pages(hash)?.ok_or("Este archivo todavía no está cortado.")?;
        if page < 2 || page > pages {
            return Err(format!("La página {page} no está entre la 2 y la {pages}."));
        }
        self.conn()
            .execute(
                "INSERT OR REPLACE INTO cut_decision (resource_hash, page, decision, decided_at) VALUES (?1, ?2, ?3, ?4)",
                params![hash, page, decision, now_ms()],
            )
            .map_err(sql_err)?;
        self.cut_parts_of(hash)
    }

    /// Makes the parts of one file again now, if anything changed.
    pub(crate) fn cut_parts_of(&self, hash: &str) -> Result<(), String> {
        let basis: Option<String> = self
            .conn()
            .query_row(
                &format!(
                    "SELECT {BASIS_SQL} FROM file f JOIN version c ON c.id = f.version_id AND c.state IN ('processed', 'needs-recognition')
                     JOIN derivative d ON d.version_id = c.id AND d.kind = 'text' WHERE f.hash = ?1"
                ),
                [hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?
            .flatten();
        match basis {
            Some(basis) => self.make_parts(hash, &basis)?,
            None => return Ok(()),
        }
        // A person's word: what waited for the cut (families) comes now,
        // without waiting for another measurement (D-556).
        self.similar_regroup()
    }

    /// A person says another document starts at `page` of the file.
    pub fn cut_split_at(&self, hash: &str, page: u32) -> Result<Vec<IncludedPart>, String> {
        self.decide(hash, page, STARTS)?;
        self.strip_parts(hash)
    }

    /// A person joins the part that starts at `page` to the one before it.
    pub fn cut_join_with_previous(&self, hash: &str, page: u32) -> Result<Vec<IncludedPart>, String> {
        self.decide(hash, page, CONTINUES)?;
        self.strip_parts(hash)
    }

    /// A person confirms every cut of the file as it is now: each page
    /// keeps, in a person's word, whether it starts a document.
    pub fn cut_confirm_all(&self, hash: &str) -> Result<Vec<IncludedPart>, String> {
        let pages = self.cut_pages(hash)?.ok_or("Este archivo todavía no está cortado.")?;
        let steps = self.cut_steps(hash)?;
        let starts: BTreeSet<u32> = {
            let conn = self.conn();
            partition(pages, &steps, &decisions(&conn, hash)?).iter().map(|p| p.first).collect()
        };
        {
            let mut conn = self.conn();
            let tx = conn.transaction().map_err(sql_err)?;
            let now = now_ms();
            for page in 2..=pages {
                let decision = if starts.contains(&page) { STARTS } else { CONTINUES };
                tx.execute(
                    "INSERT OR IGNORE INTO cut_decision (resource_hash, page, decision, decided_at) VALUES (?1, ?2, ?3, ?4)",
                    params![hash, page, decision, now],
                )
                .map_err(sql_err)?;
            }
            tx.commit().map_err(sql_err)?;
        }
        self.cut_parts_of(hash)?;
        self.strip_parts(hash)
    }
}

/// Over invented texts (R-24), without PDFs: the steps are written as the
/// cutter writes them.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::testing::TempDir;
    use crate::docbase::{Visible, PRIVATE_DB_FILE};

    /// A page of invented words, different for each seed.
    fn page_words(seed: u32) -> String {
        (0..40).map(|i| format!("palabra{seed}x{i}")).collect::<Vec<_>>().join(" ")
    }

    fn joined(pages: &[u32]) -> String {
        pages.iter().enumerate().map(|(i, seed)| format!("## Page {}\n\n{}", i + 1, page_words(*seed))).collect::<Vec<_>>().join("\n\n")
    }

    fn open() -> (TempDir, DocBase) {
        let dir = TempDir::new();
        let base = DocBase::open(&dir.path().join(PRIVATE_DB_FILE)).unwrap();
        base.conn()
            .execute_batch(
                "INSERT INTO origin (id, name, provider, location, added_at, reachable) VALUES ('o', 'Origen', 'local', 'C:/nada', 1, 1);",
            )
            .unwrap();
        (dir, base)
    }

    /// A file with this text, at a path of the origin.
    fn add_file(base: &DocBase, id: i64, hash: &str, text: &str) {
        let mut conn = base.conn();
        let tx = conn.transaction().unwrap();
        tx.execute("INSERT INTO version (id, state, first_seen_at) VALUES (?1, 'pending', 1)", [id]).unwrap();
        tx.execute("INSERT INTO file (hash, version_id, size) VALUES (?1, ?2, 10)", params![hash, id]).unwrap();
        tx.execute(
            "INSERT INTO location (origin_id, rel_path, folder, name, ext, size, mtime_ms, file_hash, hash_state, first_seen_at, last_seen_at)
             VALUES ('o', ?1, '', ?1, 'pdf', 10, 1, ?2, 'done', 1, 1)",
            params![format!("{hash}.pdf"), hash],
        )
        .unwrap();
        crate::docbase::text::store(&tx, id, text, "extraction", "processed").unwrap();
        tx.commit().unwrap();
    }

    /// The steps of a file of `pages` pages: `new` before the pages in
    /// `starts`, `doubtful` before those in `doubtful`, `continue` before
    /// the others.
    fn cut(base: &DocBase, hash: &str, pages: u32, starts: &[u32], doubtful: &[u32], stamp: i64) {
        let conn = base.conn();
        conn.execute("DELETE FROM cut_step WHERE resource_hash = ?1", [hash]).unwrap();
        for after in 0..pages {
            let next = after + 1;
            let verdict = if after == 0 || starts.contains(&next) {
                "new"
            } else if doubtful.contains(&next) {
                "doubtful"
            } else {
                "continue"
            };
            conn.execute(
                "INSERT INTO cut_step (resource_hash, after_page, verdict, rule, question, rules_version, text_stamp, created_at)
                 VALUES (?1, ?2, ?3, 'r', ?4, 'v', 1, ?5)",
                params![hash, after, verdict, (verdict == "doubtful").then(|| format!("q{next}")), stamp],
            )
            .unwrap();
        }
    }

    fn count(base: &DocBase, sql: &str) -> i64 {
        base.conn().query_row(sql, [], |row| row.get(0)).unwrap()
    }

    fn spans(parts: &[IncludedPart]) -> Vec<(u32, u32, &str, &str)> {
        parts.iter().map(|p| (p.first_page, p.last_page, p.cut_by.as_str(), p.cut_state.as_str())).collect()
    }

    #[test]
    fn a_pdf_of_three_parts_gives_four_versions_and_three_appearances_of_pages() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3, 4, 5]));
        cut(&base, "h1", 5, &[3, 5], &[], 1);
        assert_eq!(base.cut_parts_pending(10).unwrap(), 1);
        assert_eq!(count(&base, "SELECT count(*) FROM version"), 4);
        assert_eq!(count(&base, "SELECT count(*) FROM document"), 4);
        assert_eq!(count(&base, "SELECT count(*) FROM appearance WHERE selector GLOB 'pages:*'"), 3);
        let parts = base.included_parts("h1").unwrap();
        assert_eq!(spans(&parts), vec![(1, 2, "rule", "proposed"), (3, 4, "rule", "proposed"), (5, 5, "rule", "proposed")]);
        // The text of a part is that of its pages, with the numbers of the file.
        let text: String = base
            .conn()
            .query_row("SELECT body FROM derivative WHERE version_id = ?1 AND kind = 'text'", [parts[1].version_id], |r| r.get(0))
            .unwrap();
        assert!(text.starts_with("## Page 3") && text.contains("## Page 4") && !text.contains("## Page 5"));
        assert!(text.contains(&page_words(3)) && !text.contains(&page_words(2)));
        // Nothing changed: nothing is made again.
        assert_eq!(base.cut_parts_pending(10).unwrap(), 0);
        // A PDF without cuts changes nothing.
        add_file(&base, 9, "h9", &joined(&[91, 92]));
        cut(&base, "h9", 2, &[], &[], 1);
        base.cut_parts_pending(10).unwrap();
        assert!(base.included_parts("h9").unwrap().is_empty());
        assert_eq!(count(&base, "SELECT count(*) FROM version"), 5);
    }

    #[test]
    fn a_part_twin_of_another_file_is_an_appearance_of_the_same_version() {
        let (_dir, base) = open();
        add_file(&base, 1, "suelto", &joined(&[2, 3]));
        add_file(&base, 2, "h1", &joined(&[1, 2, 3]));
        cut(&base, "h1", 3, &[2], &[], 1);
        base.cut_parts_pending(10).unwrap();
        let parts = base.included_parts("h1").unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1].version_id, 1);
        assert_eq!(count(&base, "SELECT count(*) FROM appearance WHERE version_id = 1"), 2);
        assert_eq!(count(&base, "SELECT count(*) FROM version"), 3);
    }

    #[test]
    fn split_and_join_change_only_the_parts_touched_and_keep_the_numbers_of_the_others() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3, 4, 5, 6]));
        cut(&base, "h1", 6, &[3, 5], &[], 1);
        base.cut_parts_pending(10).unwrap();
        let before = base.included_parts("h1").unwrap();
        // A person splits the first part at page 2.
        let split = base.cut_split_at("h1", 2).unwrap();
        assert_eq!(
            spans(&split),
            vec![(1, 1, "person", "confirmed"), (2, 2, "rule", "proposed"), (3, 4, "rule", "proposed"), (5, 6, "rule", "proposed")]
        );
        assert_eq!((split[2].version_id, split[3].version_id), (before[1].version_id, before[2].version_id));
        assert!(split[0].version_id > before[2].version_id && split[1].version_id > before[2].version_id);
        // The version of pages 1 to 2 went as a version goes.
        assert_eq!(count(&base, &format!("SELECT count(*) FROM knowledge_gone WHERE version_id = {}", before[0].version_id)), 1);
        // A person joins pages 5 and 6 to the part before them.
        let joined_parts = base.cut_join_with_previous("h1", 5).unwrap();
        assert_eq!(spans(&joined_parts)[2], (3, 6, "rule", "proposed"));
        assert_eq!((joined_parts[0].version_id, joined_parts[1].version_id), (split[0].version_id, split[1].version_id));
        // The numbers are never given again.
        let ids: BTreeSet<i64> = joined_parts.iter().map(|p| p.version_id).collect();
        assert!(!ids.contains(&before[0].version_id) && !ids.contains(&before[2].version_id));
        // Confirming makes every cut a person's.
        let confirmed = base.cut_confirm_all("h1").unwrap();
        assert!(confirmed.iter().all(|p| p.cut_by == "person" && p.cut_state == "confirmed"));
        assert_eq!(confirmed.iter().map(|p| p.version_id).collect::<Vec<_>>(), joined_parts.iter().map(|p| p.version_id).collect::<Vec<_>>());
        assert!(base.cut_split_at("h1", 1).is_err() && base.cut_split_at("h1", 7).is_err());
    }

    #[test]
    fn what_a_person_said_survives_the_file_being_cut_again() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3, 4]));
        cut(&base, "h1", 4, &[3], &[2], 1);
        base.cut_parts_pending(10).unwrap();
        base.cut_split_at("h1", 2).unwrap();
        let parts = base.included_parts("h1").unwrap();
        assert_eq!(spans(&parts), vec![(1, 1, "person", "confirmed"), (2, 2, "rule", "proposed"), (3, 4, "rule", "proposed")]);
        // The cutter cuts it again with other rules: the person's cut stays.
        cut(&base, "h1", 4, &[], &[2, 3], 2);
        base.cut_parts_pending(10).unwrap();
        let again = base.included_parts("h1").unwrap();
        assert_eq!(spans(&again), vec![(1, 1, "person", "confirmed"), (2, 4, "rule", "proposed")]);
        assert_eq!(again[0].version_id, parts[0].version_id);
        // The model answers a doubtful step: the parts it makes are the model's.
        base.conn().execute("UPDATE cut_step SET model_says = 1 WHERE resource_hash = 'h1' AND after_page = 2", []).unwrap();
        base.cut_parts_pending(10).unwrap();
        assert_eq!(
            spans(&base.included_parts("h1").unwrap()),
            vec![(1, 1, "person", "confirmed"), (2, 2, "model", "proposed"), (3, 4, "model", "proposed")]
        );
        // The file goes and comes back: the person's word is still there.
        base.conn().execute("DELETE FROM file WHERE hash = 'h1'", []).unwrap();
        assert_eq!(count(&base, "SELECT count(*) FROM cut_decision WHERE resource_hash = 'h1'"), 1);
    }

    #[test]
    fn the_whole_file_with_parts_is_not_read_and_its_parts_go_on_without_a_person() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3]));
        add_file(&base, 2, "h2", &joined(&[7, 8]));
        let read = |base: &DocBase| -> Vec<i64> {
            crate::docbase::knowledge::extract::standings(&base.conn(), &Visible::All, None).unwrap().iter().map(|s| s.content_id).collect()
        };
        assert_eq!(read(&base), vec![1, 2]);
        // The step into page 3 is doubtful and nobody answered it.
        cut(&base, "h1", 3, &[2], &[3], 1);
        base.cut_parts_pending(10).unwrap();
        let parts: Vec<i64> = base.included_parts("h1").unwrap().iter().map(|p| p.version_id).collect();
        // The parts are read, each with its text; the whole file is not.
        assert_eq!(read(&base), [vec![2], parts.clone()].concat());
        // The second part goes on, marked: the step inside it is uncertain.
        assert_eq!(only_uncertain(&base.conn()).unwrap(), BTreeSet::from([parts[1], 1]));
        base.cut_confirm_all("h1").unwrap();
        assert!(only_uncertain(&base.conn()).unwrap().is_empty());
        // When the file goes, its parts go with it.
        base.conn().execute_batch("DELETE FROM location;").unwrap();
        base.purge_orphans().unwrap();
        assert_eq!(count(&base, "SELECT count(*) FROM version"), 0);
    }

    #[test]
    fn copies_and_versions_compare_each_part_not_the_whole_file_and_mark_an_uncertain_cut() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3]));
        // Another file with the text of pages 2 and 3, and one page more.
        add_file(&base, 2, "h2", &joined(&[2, 3, 4]));
        while base.similar_step(16).unwrap() > 0 {}
        let signed = |base: &DocBase, id: i64| count(base, &format!("SELECT count(*) FROM similar_signature WHERE version_id = {id}"));
        let pairs_of = |base: &DocBase, id: i64| count(base, &format!("SELECT count(*) FROM similar_pair WHERE a = {id} OR b = {id}"));
        assert_eq!((signed(&base, 1), pairs_of(&base, 1)), (1, 1));
        cut(&base, "h1", 3, &[2], &[3], 1);
        base.cut_parts_pending(10).unwrap();
        let parts: Vec<i64> = base.included_parts("h1").unwrap().iter().map(|p| p.version_id).collect();
        while base.similar_step(16).unwrap() > 0 {}
        // The whole file is not compared any more, and what it had goes;
        // each part is compared with its own text.
        assert_eq!((signed(&base, 1), pairs_of(&base, 1)), (0, 0));
        assert!(parts.iter().all(|id| signed(&base, *id) == 1));
        let second = parts[1];
        assert_eq!(
            count(&base, &format!("SELECT count(*) FROM similar_pair WHERE a = min({second}, 2) AND b = max({second}, 2)")),
            1
        );
        assert_eq!(pairs_of(&base, parts[0]), 0);
        // Its proposal does not wait for the cut: it is made, marked as
        // uncertain, and the mark goes when a person confirms the cut.
        let uncertain = |base: &DocBase| {
            count(base, &format!("SELECT count(*) FROM similar_pair p WHERE {}", crate::docbase::similar::store::uncertain_pair_sql("p")))
        };
        assert_eq!(uncertain(&base), 1);
        base.cut_confirm_all("h1").unwrap();
        assert_eq!(uncertain(&base), 0);
        // Joined again into one document, the whole file is compared again.
        base.cut_join_with_previous("h1", 2).unwrap();
        assert!(base.included_parts("h1").unwrap().is_empty());
        while base.similar_step(16).unwrap() > 0 {}
        assert_eq!(signed(&base, 1), 1);
    }

    #[test]
    fn a_file_waiting_for_recognition_is_cut_and_the_part_without_text_waits() {
        let (_dir, base) = open();
        let text = format!("{}

## Page 3

", joined(&[1, 2]));
        add_file(&base, 1, "h1", &text);
        base.conn().execute("UPDATE version SET state = 'needs-recognition' WHERE id = 1", []).unwrap();
        cut(&base, "h1", 3, &[2, 3], &[], 1);
        base.cut_parts_pending(10).unwrap();
        let parts = base.included_parts("h1").unwrap();
        assert_eq!(parts.len(), 3);
        let state = |id: i64| -> String { base.conn().query_row("SELECT state FROM version WHERE id = ?1", [id], |r| r.get(0)).unwrap() };
        assert_eq!((state(parts[0].version_id), state(parts[2].version_id)), ("processed".to_string(), "needs-recognition".to_string()));
    }

    #[test]
    fn the_doubtful_steps_no_person_decided_are_marked_on_their_pages() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3, 4, 5]));
        cut(&base, "h1", 5, &[4], &[2, 3], 1);
        base.cut_parts_pending(10).unwrap();
        let parts = base.included_parts("h1").unwrap();
        assert_eq!(parts.iter().map(|p| p.doubtful_pages.clone()).collect::<Vec<_>>(), vec![vec![2, 3], vec![]]);
        // A person decides one of them: it is no longer for looking at.
        let split = base.cut_split_at("h1", 3).unwrap();
        assert_eq!(split.iter().map(|p| p.doubtful_pages.clone()).collect::<Vec<_>>(), vec![vec![2], vec![], vec![]]);
        // Confirming leaves none.
        assert!(base.cut_confirm_all("h1").unwrap().iter().all(|p| p.doubtful_pages.is_empty()));
    }

    /// What backs each step (D-603): an answer of high or medium
    /// confidence backs it; no answer, or one of low confidence, leaves it
    /// uncertain, and so the parts it bounds or holds.
    #[test]
    fn an_answer_of_low_confidence_or_none_leaves_the_step_uncertain() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3, 4, 5, 6]));
        // Doubtful steps into pages 2, 4 and 6; another document at 5.
        cut(&base, "h1", 6, &[5], &[2, 4, 6], 1);
        let answer = |after: u32, says: bool, confidence: &str| {
            base.conn()
                .execute(
                    "UPDATE cut_step SET model_says = ?1, model_confidence = ?2 WHERE resource_hash = 'h1' AND after_page = ?3",
                    params![says, confidence, after],
                )
                .unwrap();
        };
        answer(1, true, "high");
        answer(3, false, "low");
        base.cut_parts_pending(10).unwrap();
        let parts = base.included_parts("h1").unwrap();
        // Page 2 starts by an answer of high confidence: backed. Page 4
        // goes on by one of low confidence, page 6 with no answer.
        assert_eq!(
            parts.iter().map(|p| (p.first_page, p.last_page, p.uncertain, p.doubtful_pages.clone())).collect::<Vec<_>>(),
            vec![(1, 1, false, vec![]), (2, 4, true, vec![4]), (5, 6, true, vec![6])]
        );
        answer(3, false, "medium");
        answer(5, false, "medium");
        base.cut_parts_pending(10).unwrap();
        assert!(base.included_parts("h1").unwrap().iter().all(|p| !p.uncertain && p.doubtful_pages.is_empty()));
        assert_eq!(count(&base, "SELECT uncertain FROM appearance WHERE selector = 'file'"), 0);
    }

    /// A PDF of one document with an uncertain step stays one document,
    /// marked: its strip shows the whole file with that page to look at,
    /// and a person's word takes the mark away.
    #[test]
    fn a_pdf_of_one_document_with_an_uncertain_step_shows_its_strip() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3]));
        cut(&base, "h1", 3, &[], &[3], 1);
        base.cut_parts_pending(10).unwrap();
        assert!(base.included_parts("h1").unwrap().is_empty());
        assert_eq!(count(&base, "SELECT uncertain FROM appearance WHERE selector = 'file'"), 1);
        let strip = base.strip_parts("h1").unwrap();
        assert_eq!(
            strip.iter().map(|p| (p.first_page, p.last_page, p.uncertain, p.whole, p.doubtful_pages.clone())).collect::<Vec<_>>(),
            vec![(1, 3, true, true, vec![3])]
        );
        let rows = base.documents(&Visible::All, &Default::default()).unwrap().rows;
        assert_eq!((rows[0].included, rows[0].unconfirmed_cuts), (0, 1));
        assert!(base.cut_confirm_all("h1").unwrap().is_empty());
        assert_eq!(count(&base, "SELECT uncertain FROM appearance WHERE selector = 'file'"), 0);
        // A file with nothing doubtful has no strip.
        add_file(&base, 2, "h2", &joined(&[7, 8]));
        cut(&base, "h2", 2, &[], &[], 1);
        base.cut_parts_pending(10).unwrap();
        assert!(base.strip_parts("h2").unwrap().is_empty());
    }

    #[test]
    fn a_joined_file_says_what_it_includes_and_each_part_opens_as_a_document() {
        let (_dir, base) = open();
        add_file(&base, 1, "h1", &joined(&[1, 2, 3, 4]));
        // The step into page 2 is doubtful: the first part is uncertain.
        cut(&base, "h1", 4, &[3], &[2], 1);
        base.cut_parts_pending(10).unwrap();
        let rows = base.documents(&crate::docbase::Visible::All, &Default::default()).unwrap().rows;
        assert_eq!((rows[0].included, rows[0].unconfirmed_cuts), (2, 1));
        let location = rows[0].location_id;
        let parts = base.included_parts("h1").unwrap();
        // The second part: its own text and its pages, inside the file.
        let detail = base.document_part(&crate::docbase::Visible::All, location, parts[1].version_id).unwrap().unwrap();
        assert_eq!(detail.row.content_id, Some(parts[1].version_id));
        assert_eq!(detail.file_hash.as_deref(), Some("h1"));
        let part = detail.part.unwrap();
        assert_eq!((part.ordinal, part.count, part.first_page, part.last_page), (2, 2, 3, 4));
        assert!(detail.text.unwrap().contains(&page_words(3)));
        // The file itself is not one of its parts.
        assert!(base.document_part(&crate::docbase::Visible::All, location, 1).unwrap().is_none());
        // Its page opens inside the file.
        base.conn()
            .execute(
                "INSERT INTO knowledge_page (entity_id, path, fingerprint, written_at) VALUES (?1, 'wiki/documents/parte.md', 'f', 1)",
                [format!("doc-{}", parts[1].version_id)],
            )
            .unwrap();
        let object = base.knowledge_object_of_page("wiki/documents/parte.md").unwrap().unwrap();
        assert_eq!((object.location_id, object.included), (Some(location), true));
        // Confirmed, nothing is left uncertain.
        base.cut_confirm_all("h1").unwrap();
        let rows = base.documents(&crate::docbase::Visible::All, &Default::default()).unwrap().rows;
        assert_eq!((rows[0].included, rows[0].unconfirmed_cuts), (2, 0));
    }
}
