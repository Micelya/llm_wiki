//! The passes over the archive, kept in the database and done little by
//! little in the background (R-15: only what changed and its neighbours).
//!
//! 1. **Sign** each content with text, once per text and per catalog:
//!    its shingles, MinHash and buckets, and its marks (pass 2). Signing
//!    counts its shingles into `similar_common` (pass 0) and leaves the
//!    content *dirty*.
//! 2. **Measure the neighbourhood** of each dirty content (pass 1): its
//!    candidates by bucket, the exact measures without the routine text,
//!    the alignment, what changes, and the proposal.
//! 3. **Regroup** (pass 3) once nothing is dirty: families against a
//!    principal document.
//!
//! Computed rows are rebuilt from the texts whenever they are missing or
//! stale; a person's decision is never overwritten (R-08, R-09, R-12): a
//! pair that was discarded stays discarded however often it is measured.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use crate::panic_guard::run_guarded;
use crate::source_volume::{LocalFolderProvider, SourceProvider};

use super::super::{now_ms, sql_err, DocBase, Visible};
use super::align;
use super::changes::{self, Changes};
use super::key_data::{self, KeyData, TextIndex};
use super::marks::{self, Mark, PdfSignature};
use super::format;
use super::minhash;
use super::propose::{self, Measures, Side};
use super::text::{self, Prepared, SHINGLE_WORDS};
use super::types::{self, RelationDef};

/// Version of the computation. Changing it, or the catalog, computes
/// everything again. `similar-2`: the key data of the two documents decide
/// whether a version or a similar pair are another document (D-497).
/// `similar-3`: a version shares much in both directions. `similar-4`:
/// the text is compared without what its format adds (D-521).
pub const ENGINE: &str = "similar-4";
/// Where the base keeps how the key data looked when it last compared
/// them (`similar_key_data_check`).
const KEY_DATA_SETTING: &str = "similar-key-data";
/// Below this many words a text says too little to compare (the same
/// limit as the text fingerprint).
pub const MIN_WORDS: usize = 30;
/// Pass 0: a shingle is routine text (letterhead, a signature block, the
/// firm's own details) when it appears in at least this share of the
/// documents with text, and in at least `ROUTINE_MIN_DOCS` of them. High
/// on purpose: the thirteen invoices of one supplier must not make their
/// own template routine.
pub const ROUTINE_SHARE: f64 = 0.5;
pub const ROUTINE_MIN_DOCS: i64 = 10;
/// Neighbours by bucket whose estimated containment is below this are not
/// measured; at most `MAX_NEIGHBOURS` are, the closest first (R-16).
pub const CANDIDATE_MIN: f64 = 0.15;
pub const MAX_NEIGHBOURS: usize = 100;
/// Exact shingle containment below which a pair is not aligned.
pub const MEASURE_MIN: f64 = 0.25;
/// Relations that put two documents in the same family: those that make
/// them one document. The same template is a relation between different
/// documents (D-497), so it never joins a family.
pub const GROUP_RELATIONS: &[&str] = &["near-identical", "signed-copy", "preview", "version"];
/// Relations that put two documents in the same unit when typing them and
/// when building a template: the families' and the same template.
pub const UNIT_RELATIONS: &[&str] = &["near-identical", "signed-copy", "preview", "version", "same-template"];
/// The links of each document: the other document, the relation and the cover.
pub(crate) type Links = BTreeMap<i64, BTreeMap<i64, (String, f64)>>;
/// A family bigger than this is marked for review.
pub const GROUP_LARGE: usize = 30;

pub(crate) fn engine_id() -> String {
    format!("{ENGINE}+{}", types::catalog().digest)
}

fn json_err(e: serde_json::Error) -> String {
    format!("Document base error: {e}")
}

/// What was computed for one content, before it is saved.
struct Signed {
    content_id: i64,
    text_at: i64,
    words: usize,
    shingles: Vec<u64>,
    minhash: Vec<u64>,
    marks: Vec<Mark>,
    signatures: Vec<PdfSignature>,
    skip_pages: Vec<usize>,
    drop_phrases: Vec<String>,
}

/// A content ready to be compared.
struct Doc {
    id: i64,
    prepared: Prepared,
    shingles: Vec<u64>,
    /// Words covered by routine shingles.
    routine: Vec<bool>,
    own_words: usize,
    recognized: bool,
    marks: Vec<String>,
    fields: Vec<(String, String)>,
    /// Its parties, matter, identifiers, amounts and dates, as the base
    /// knows them, and its text to look them up in (D-497).
    key: KeyData,
    index: TextIndex,
}

/// A document of the base as the screens show it: one of its locations.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocRef {
    pub content_id: i64,
    pub location_id: i64,
    pub origin_name: String,
    pub rel_path: String,
    pub name: String,
    /// Locations the document has.
    pub copies: i64,
}

/// A document similar to the one asked about.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarDocument {
    pub content_id: i64,
    pub other: DocRef,
    /// Proposed relation: an id of the catalog's relations.
    pub relation: String,
    /// For a directed relation, which of the two is the relation of the
    /// other (the signed copy, the preview, the one contained).
    pub subject_content_id: Option<i64>,
    pub certainty: String,
    pub description: String,
    pub jaccard: f64,
    pub containment: f64,
    pub cover: f64,
    pub similarity: f64,
    pub suggested_source: Option<i64>,
    pub evidence: serde_json::Value,
    pub evidence_hash: String,
    pub computed_at: i64,
    /// "proposed", "joined" or "discarded".
    pub state: String,
    pub chosen_relation: Option<String>,
    /// The document left as the wiki's source when both were joined as
    /// one document.
    pub wiki_source: Option<i64>,
    pub decided_at: Option<i64>,
    /// Two documents of one template where no datum changes (D-440): made
    /// with the template they would wait for no one, but they may be one
    /// same document, so they wait for a person (D-497).
    pub possibly_same: bool,
}

/// What waits for a person, for Inicio.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarPending {
    /// Pairs with no decision whose proposed relation waits for a person
    /// (`asks_person`): a copy, a version, one inside the other, a delivery
    /// receipt or a signing certificate. The same template and the only
    /// similar wait for no one (D-497).
    pub pairs: i64,
    /// Families with at least one such pair.
    pub groups: i64,
    /// Of `pairs`, those whose proposed relation makes both one document
    /// (practically the same, the signed copy, a version): deciding them
    /// changes which one feeds the wiki.
    pub same_document_pairs: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupMember {
    pub doc: DocRef,
    /// Its relation with the principal (the proposed one, or the one a
    /// person chose).
    pub relation: Option<String>,
    pub cover: Option<f64>,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimilarGroup {
    pub principal: DocRef,
    pub members: Vec<GroupMember>,
    /// "large" (grew too much) or "dispersed" (part of what is similar to
    /// its members is not similar to the principal).
    pub flags: Vec<String>,
    /// Members whose pair with the principal waits for a decision: one
    /// whose relation asks a person (`asks_person`).
    pub pending: i64,
    /// Of `pending`, those whose proposed relation makes both one document.
    pub pending_same_document: i64,
    /// How many fields its template has, when it shares one.
    pub template_fields: Option<i64>,
    /// Pairs of its template's documents where nothing changes but a few
    /// words, still undecided: possibly the same document (P-218).
    pub possibly_same: i64,
}

/// One decision of `decide_similar_many`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PairDecision {
    pub content_id: i64,
    pub other_content_id: i64,
    pub decision: Decision,
    /// The fingerprint of the evidence the person saw (R-08).
    #[serde(default)]
    pub evidence_hash: Option<String>,
}

/// A pair that waits for a decision outside the families' principals.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenPair {
    /// The document the pair is seen from (`similar.content_id`).
    pub doc: DocRef,
    pub similar: SimilarDocument,
}

/// The columns `pair_from_row` reads, in its order.
const PAIR_COLUMNS: &str = "a, b, relation, subject, certainty, description, jaccard, containment, cover, similarity, \
     suggested_source, evidence, evidence_hash, computed_at, state, chosen_relation, wiki_source, decided_at";

/// A row of `similar_pair` (`PAIR_COLUMNS`) seen from `seen_from`, with
/// the other document still to be filled in.
fn pair_from_row(row: &rusqlite::Row<'_>, seen_from: i64) -> rusqlite::Result<(i64, i64, SimilarDocument)> {
    let a: i64 = row.get(0)?;
    let b: i64 = row.get(1)?;
    let evidence: String = row.get(11)?;
    Ok((
        a,
        b,
        SimilarDocument {
            content_id: seen_from,
            other: DocRef { content_id: 0, location_id: 0, origin_name: String::new(), rel_path: String::new(), name: String::new(), copies: 0 },
            relation: row.get(2)?,
            subject_content_id: row.get(3)?,
            certainty: row.get(4)?,
            description: row.get(5)?,
            jaccard: row.get(6)?,
            containment: row.get(7)?,
            cover: row.get(8)?,
            similarity: row.get(9)?,
            suggested_source: row.get(10)?,
            evidence: serde_json::from_str(&evidence).unwrap_or(serde_json::Value::Null),
            evidence_hash: row.get(12)?,
            computed_at: row.get(13)?,
            state: row.get(14)?,
            chosen_relation: row.get(15)?,
            wiki_source: row.get(16)?,
            decided_at: row.get(17)?,
            possibly_same: false,
        },
    ))
}

/// What a person decided about a pair.
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum Decision {
    /// Joined with this relation; for one that makes them one document,
    /// `wiki_source` is the one that keeps feeding the wiki.
    Join { relation: String, wiki_source: Option<i64> },
    Discard,
    /// Back to proposed.
    Undo,
}

impl DocBase {
    /// One small piece of the similar-documents work. Returns how much was
    /// done (0: nothing pending).
    pub fn similar_step(&self, limit: usize) -> Result<usize, String> {
        let forgotten = self.similar_forget_orphans()?;
        let signed = self.similar_sign_pending(limit)?;
        if signed > 0 {
            return Ok(forgotten + signed);
        }
        self.similar_key_data_check()?;
        let measured = self.similar_measure_dirty(limit)?;
        let dirty_left: i64 = self
            .conn()
            .query_row("SELECT count(*) FROM similar_dirty", [], |row| row.get(0))
            .map_err(sql_err)?;
        if (measured > 0 || forgotten > 0) && dirty_left == 0 {
            // A new measurement undoes what the rules of the type said
            // (pass 6), so they apply again before the families.
            self.apply_type_rules()?;
            self.similar_regroup()?;
            self.types_refresh()?;
        }
        if measured > 0 || dirty_left > 0 {
            return Ok(forgotten + measured);
        }
        // A base that comes from before the templates (V21) builds them
        // once, without waiting for its families to change.
        Ok(forgotten + self.similar_templates_refresh()?)
    }

    /// Forgets everything computed (never a decision) so it is computed
    /// again from the texts.
    #[cfg(test)]
    pub fn similar_reset(&self) -> Result<(), String> {
        self.conn()
            .execute_batch(
                "DELETE FROM similar_signature; DELETE FROM similar_common; DELETE FROM document_marks;
                 DELETE FROM similar_dirty; DELETE FROM similar_group;
                 DELETE FROM similar_template_value; DELETE FROM similar_template_field;
                 DELETE FROM similar_template_document; DELETE FROM similar_template;
                 DELETE FROM similar_key_data; DELETE FROM docbase_setting WHERE key = 'similar-key-data';
                 DELETE FROM similar_pair WHERE state = 'proposed';
                 DELETE FROM document_type; DELETE FROM document_data;",
            )
            .map_err(sql_err)
    }

    /// Uncounts and drops the signatures of contents that went away.
    fn similar_forget_orphans(&self) -> Result<usize, String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        let gone: Vec<(i64, Vec<u8>)> = {
            let mut statement = tx
                .prepare("SELECT content_id, shingles FROM similar_signature WHERE content_id NOT IN (SELECT id FROM content)")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        for (content_id, shingles) in &gone {
            uncount(&tx, &minhash::from_blob(shingles))?;
            tx.execute("DELETE FROM similar_signature WHERE content_id = ?1", [content_id])
                .map_err(sql_err)?;
            tx.execute("DELETE FROM similar_dirty WHERE content_id = ?1", [content_id])
                .map_err(sql_err)?;
        }
        tx.commit().map_err(sql_err)?;
        Ok(gone.len())
    }

    /// Signs one content again as if its PDFs carried these signatures:
    /// the tests have no signed PDF to read.
    #[cfg(test)]
    pub(crate) fn similar_sign_with(&self, content_id: i64, signatures: Vec<PdfSignature>) -> Result<(), String> {
        let (body, text_at, fields): (String, i64, Option<String>) = self
            .conn()
            .query_row(
                "SELECT d.body, d.created_at, fd.body FROM derivative d
                 LEFT JOIN derivative fd ON fd.content_id = d.content_id AND fd.kind = 'fields'
                 WHERE d.content_id = ?1 AND d.kind = 'text'",
                [content_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(sql_err)?;
        let signed = sign(content_id, &body, text_at, fields.is_some(), signatures);
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        save_signature(&tx, &signed, &engine_id())?;
        tx.commit().map_err(sql_err)
    }

    /// Signs up to `limit` contents whose signature is missing or stale.
    fn similar_sign_pending(&self, limit: usize) -> Result<usize, String> {
        let engine = engine_id();
        let jobs: Vec<(i64, String, i64, Option<String>, Vec<(String, PathBuf)>)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(
                    "SELECT c.id, d.body, d.created_at, fd.body
                     FROM content c
                     JOIN derivative d ON d.content_id = c.id AND d.kind = 'text'
                     LEFT JOIN derivative fd ON fd.content_id = c.id AND fd.kind = 'fields'
                     LEFT JOIN similar_signature s ON s.content_id = c.id
                     WHERE c.state = 'processed'
                       AND (s.content_id IS NULL OR s.text_at != d.created_at OR s.engine != ?1)
                     ORDER BY c.id
                     LIMIT ?2",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map(params![engine, limit as i64], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
                })
                .map_err(sql_err)?
                .collect::<Result<Vec<(i64, String, i64, Option<String>)>, _>>()
                .map_err(sql_err)?;
            let mut jobs = Vec::new();
            for (content_id, body, text_at, fields) in rows {
                let paths = pdf_paths(&conn, content_id)?;
                jobs.push((content_id, body, text_at, fields, paths));
            }
            jobs
        };
        for (content_id, body, text_at, fields, paths) in &jobs {
            let mut signatures = Vec::new();
            for (file, path) in paths {
                match run_guarded("docbase_pdf_signatures", || marks::pdf_signatures(path, Some(file))) {
                    Ok(found) => {
                        for signature in found {
                            if !signatures.contains(&signature) {
                                signatures.push(signature);
                            }
                        }
                    }
                    Err(error) => eprintln!("[docbase] signatures of '{}': {error}", path.display()),
                }
            }
            let signed = sign(*content_id, body, *text_at, fields.is_some(), signatures);
            let mut conn = self.conn();
            let tx = conn.transaction().map_err(sql_err)?;
            save_signature(&tx, &signed, &engine)?;
            tx.commit().map_err(sql_err)?;
        }
        Ok(jobs.len())
    }

    /// Measures the neighbourhood of up to `limit` dirty contents.
    fn similar_measure_dirty(&self, limit: usize) -> Result<usize, String> {
        let dirty: Vec<(i64, i64)> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare("SELECT content_id, since FROM similar_dirty ORDER BY since, content_id LIMIT ?1")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([limit as i64], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        if dirty.is_empty() {
            return Ok(0);
        }
        let routine = {
            let conn = self.conn();
            routine_shingles(&conn)?
        };
        for (content_id, since) in &dirty {
            self.measure_one(*content_id, &routine)?;
            self.conn()
                .execute("DELETE FROM similar_dirty WHERE content_id = ?1 AND since = ?2", params![content_id, since])
                .map_err(sql_err)?;
        }
        Ok(dirty.len())
    }

    fn measure_one(&self, content_id: i64, routine: &HashSet<u64>) -> Result<(), String> {
        // Candidates, with the lock held only for the queries.
        let (me, neighbours) = {
            let conn = self.conn();
            let Some(me) = load_doc(&conn, content_id, routine)? else {
                return Ok(());
            };
            let mine = signature_of(&conn, content_id)?;
            let mut candidates: BTreeSet<i64> = BTreeSet::new();
            {
                let mut statement = conn
                    .prepare(
                        "SELECT DISTINCT o.content_id FROM similar_band m
                         JOIN similar_band o ON o.band = m.band AND o.bucket = m.bucket AND o.content_id != m.content_id
                         WHERE m.content_id = ?1",
                    )
                    .map_err(sql_err)?;
                let rows = statement
                    .query_map([content_id], |row| row.get::<_, i64>(0))
                    .map_err(sql_err)?;
                for row in rows {
                    candidates.insert(row.map_err(sql_err)?);
                }
            }
            let mut ranked: Vec<(f64, i64)> = Vec::new();
            for other in &candidates {
                let Some(theirs) = signature_of(&conn, *other)? else {
                    continue;
                };
                let estimate = estimated_containment(&mine, &theirs);
                if estimate >= CANDIDATE_MIN && theirs.0 >= MIN_WORDS {
                    ranked.push((estimate, *other));
                }
            }
            ranked.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)));
            ranked.truncate(MAX_NEIGHBOURS);
            let mut neighbours: BTreeSet<i64> = ranked.into_iter().map(|(_, id)| id).collect();
            // Pairs it already has are measured again, so they are updated or dropped.
            let mut statement = conn
                .prepare("SELECT CASE WHEN a = ?1 THEN b ELSE a END FROM similar_pair WHERE a = ?1 OR b = ?1")
                .map_err(sql_err)?;
            let existing = statement
                .query_map([content_id], |row| row.get::<_, i64>(0))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            neighbours.extend(existing);
            (me, neighbours)
        };
        // A document generated from a template never pairs with the
        // documents of its template: it is made with it, not similar to
        // them (D-497).
        let generated = {
            let conn = self.conn();
            super::generate::generated_families(&conn)?
        };
        let made_with = |x: i64, y: i64| generated.get(&x).is_some_and(|theirs| theirs.contains(&y));

        let mut results: Vec<(i64, i64, Option<(propose::Proposal, Measures, serde_json::Value)>)> = Vec::new();
        for other_id in neighbours {
            let (lo, hi) = if content_id < other_id { (content_id, other_id) } else { (other_id, content_id) };
            if made_with(content_id, other_id) || made_with(other_id, content_id) {
                results.push((lo, hi, None));
                continue;
            }
            let other = {
                let conn = self.conn();
                load_doc(&conn, other_id, routine)?
            };
            let outcome = match &other {
                Some(other) if me.prepared.word_count() >= MIN_WORDS && other.prepared.word_count() >= MIN_WORDS => {
                    let (a, b) = if me.id < other.id { (&me, other) } else { (other, &me) };
                    compare(a, b, routine)
                }
                _ => None,
            };
            results.push((lo, hi, outcome));
        }

        let engine = engine_id();
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        for (a, b, outcome) in results {
            save_pair(&tx, a, b, outcome, &engine)?;
        }
        // The key data its pairs were measured with.
        tx.execute(
            "INSERT OR REPLACE INTO similar_key_data (content_id, print, computed_at) VALUES (?1, ?2, ?3)",
            params![content_id, me.key.print(), now_ms()],
        )
        .map_err(sql_err)?;
        tx.commit().map_err(sql_err)
    }

    /// The key data of the documents change after they were measured: the
    /// reading names their parties and amounts later, a person merges two
    /// entities. The documents whose key data are not the ones their pairs
    /// were measured with are measured again, with their neighbours (R-15).
    /// It looks only when what the base knows changed since the last time
    /// (a few counts, so a step of two documents does not read every key
    /// datum of the archive each time). Returns how many it left to measure.
    fn similar_key_data_check(&self) -> Result<usize, String> {
        let conn = self.conn();
        let now: String = conn
            .query_row(
                "SELECT (SELECT count(*) || ':' || coalesce(max(id), 0) || ':' || count(entity_id) || ':' || coalesce(max(entity_id), '')
                         FROM knowledge_mention)
                     || '|' || (SELECT count(*) FROM knowledge_identifier)
                     || '|' || (SELECT count(*) || ':' || coalesce(max(id), 0) FROM knowledge_evidence)
                     || '|' || (SELECT count(*) || ':' || coalesce(max(computed_at), 0) FROM document_data)
                     || '|' || (SELECT count(*) || ':' || count(merged_into) || ':' || coalesce(max(updated_at), 0) FROM knowledge_entity)",
                [],
                |row| row.get(0),
            )
            .map_err(sql_err)?;
        let then: Option<String> = conn
            .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [KEY_DATA_SETTING], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        if then.as_deref() == Some(now.as_str()) {
            return Ok(0);
        }
        let measured: Vec<(i64, String)> = {
            let mut statement = conn
                .prepare(
                    "SELECT k.content_id, k.print FROM similar_key_data k
                     WHERE k.content_id NOT IN (SELECT content_id FROM similar_dirty)",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let current = key_data::load_all(&conn, None)?;
        let empty = KeyData::default().print();
        let at = now_ms();
        let mut stale = 0;
        for (content_id, print) in measured {
            let print_now = current.get(&content_id).map(KeyData::print).unwrap_or_else(|| empty.clone());
            if print_now != print {
                conn.execute("INSERT OR REPLACE INTO similar_dirty (content_id, since) VALUES (?1, ?2)", params![content_id, at])
                    .map_err(sql_err)?;
                stale += 1;
            }
        }
        conn.execute("DELETE FROM similar_key_data WHERE content_id NOT IN (SELECT id FROM content)", [])
            .map_err(sql_err)?;
        conn.execute(
            "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
            params![KEY_DATA_SETTING, now],
        )
        .map_err(sql_err)?;
        Ok(stale)
    }

    /// Pass 3: families against a principal document (R-14), from the
    /// pairs no one discarded, and the template of each family of the
    /// same template (`template`), built again where its family changed.
    pub fn similar_regroup(&self) -> Result<(), String> {
        self.regroup_families()?;
        self.similar_templates_refresh().map(|_| ())
    }

    fn regroup_families(&self) -> Result<(), String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        // A document generated from a template has no pair with the
        // documents it came from (D-497), and is never the principal of a
        // family (D-488).
        super::generate::generated_pairs_dropped(&tx)?;
        let (groups, adjacent) = grouped(&tx, GROUP_RELATIONS)?;

        tx.execute("DELETE FROM similar_group", []).map_err(sql_err)?;
        let now = now_ms();
        for group in &groups {
            tx.execute(
                "INSERT INTO similar_group (principal, members, flags, computed_at) VALUES (?1, ?2, ?3, ?4)",
                params![group.principal, group.members.len() as i64, serde_json::to_string(&group.flags).map_err(json_err)?, now],
            )
            .map_err(sql_err)?;
            for member in &group.members {
                let link = adjacent.get(&group.principal).and_then(|links| links.get(member));
                tx.execute(
                    "INSERT INTO similar_group_member (principal, content_id, relation, cover) VALUES (?1, ?2, ?3, ?4)",
                    params![group.principal, member, link.map(|(r, _)| r.clone()), link.map(|(_, c)| *c)],
                )
                .map_err(sql_err)?;
            }
        }
        tx.commit().map_err(sql_err)
    }

    /// The documents similar to the one at this location.
    pub fn similar_of(&self, visible: &Visible, location_id: i64) -> Result<Vec<SimilarDocument>, String> {
        let conn = self.conn();
        let Some(content_id) = content_of_location(&conn, location_id)? else {
            return Ok(Vec::new());
        };
        let mut statement = conn
            .prepare(&format!(
                "SELECT {PAIR_COLUMNS} FROM similar_pair WHERE a = ?1 OR b = ?1
                 ORDER BY (state = 'discarded'), (relation = 'similar'), cover DESC"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map([content_id], |row| {
                let (a, b, similar) = pair_from_row(row, content_id)?;
                Ok((if a == content_id { b } else { a }, similar))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        drop(statement);
        let possibly_same = super::template::possibly_same_pairs(&conn)?;
        let mut out = Vec::new();
        for (other, mut similar) in rows {
            // A relation is seen only if both documents are (design, shared space).
            if let Some(doc) = doc_ref(&conn, visible, other)? {
                similar.other = doc;
                similar.possibly_same = possibly_same.contains(&(content_id.min(other), content_id.max(other)));
                out.push(similar);
            }
        }
        Ok(out)
    }

    /// Records what a person decided about the pair of these two
    /// documents. `shown_evidence` is the fingerprint of the evidence the
    /// person saw (R-08); the current one when not given.
    pub fn decide_similar(
        &self,
        content_id: i64,
        other_content_id: i64,
        decision: &Decision,
        shown_evidence: Option<&str>,
    ) -> Result<(), String> {
        self.record_decision(content_id, other_content_id, decision, shown_evidence)?;
        self.similar_regroup()
    }

    /// Several decisions at once, as one: a family confirmed in one click.
    /// They are recorded in order and the families are made again once.
    /// When one fails, the ones before it stay recorded and the error
    /// says how many were saved.
    pub fn decide_similar_many(&self, decisions: &[PairDecision]) -> Result<usize, String> {
        let mut done = 0;
        let mut failure = None;
        for item in decisions {
            match self.record_decision(item.content_id, item.other_content_id, &item.decision, item.evidence_hash.as_deref()) {
                Ok(()) => done += 1,
                Err(error) => {
                    failure = Some(error);
                    break;
                }
            }
        }
        if done > 0 {
            self.similar_regroup()?;
        }
        match failure {
            Some(error) => Err(format!("Se guardaron {done} de {} decisiones. {error}", decisions.len())),
            None => Ok(done),
        }
    }

    /// The pairs that wait for a decision and are not between a family's
    /// principal and one of its members (those are resolved from the
    /// family): pairs outside families, and pairs between two members.
    /// Each is seen from its smaller content (`similar.content_id`), whose
    /// document is `doc`. Only those that wait for a person: the same
    /// template and plain "similar" are different documents (D-497),
    /// unless no datum changes between two of one template (possibly the
    /// same document, D-440).
    pub fn similar_open_pairs(&self, visible: &Visible) -> Result<Vec<OpenPair>, String> {
        let conn = self.conn();
        let possibly_same = super::template::possibly_same_pairs(&conn)?;
        let mut statement = conn
            .prepare(&format!(
                "SELECT {PAIR_COLUMNS} FROM similar_pair p
                 WHERE p.state = 'proposed' AND p.relation != 'similar' AND NOT EXISTS (
                    SELECT 1 FROM similar_group_member m
                    WHERE m.principal != m.content_id
                      AND ((m.principal = p.a AND m.content_id = p.b) OR (m.principal = p.b AND m.content_id = p.a)))
                 ORDER BY p.cover DESC, p.a, p.b"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                let a: i64 = row.get(0)?;
                pair_from_row(row, a)
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        drop(statement);
        let mut out = Vec::new();
        for (a, b, mut similar) in rows {
            similar.possibly_same = possibly_same.contains(&(a, b));
            if !asks_person(&similar.relation) && !similar.possibly_same {
                continue;
            }
            let (Some(doc), Some(other)) = (doc_ref(&conn, visible, a)?, doc_ref(&conn, visible, b)?) else {
                continue;
            };
            similar.other = other;
            out.push(OpenPair { doc, similar });
        }
        Ok(out)
    }

    fn record_decision(
        &self,
        content_id: i64,
        other_content_id: i64,
        decision: &Decision,
        shown_evidence: Option<&str>,
    ) -> Result<(), String> {
        let (a, b) = if content_id < other_content_id { (content_id, other_content_id) } else { (other_content_id, content_id) };
        {
            let conn = self.conn();
            let current: Option<String> = conn
                .query_row("SELECT evidence_hash FROM similar_pair WHERE a = ?1 AND b = ?2", params![a, b], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            let Some(current) = current else {
                return Err("Esos dos documentos no figuran como parecidos".to_string());
            };
            let shown = shown_evidence.map(str::to_string).unwrap_or(current);
            let now = now_ms();
            match decision {
                Decision::Join { relation, wiki_source } => {
                    let Some(definition) = types::catalog().relation(relation) else {
                        return Err(format!("'{relation}' no es una relación conocida"));
                    };
                    let source = match (definition.same_document, wiki_source) {
                        (true, Some(source)) if *source == a || *source == b => Some(*source),
                        (true, _) => return Err("Hay que elegir cuál de los dos documentos queda como fuente del wiki".to_string()),
                        (false, None) => None,
                        (false, Some(_)) => return Err("Esa relación no deja un solo documento como fuente del wiki".to_string()),
                    };
                    if let Some(source) = source {
                        if let Some(joined) = joined_without_source(&conn, a, b, source)? {
                            return Err(format!(
                                "Con esa elección ninguno de los {joined} documentos unidos como el mismo quedaría como \
                                 fuente del wiki: cada uno tiene otro elegido en lugar de él. Elegí el otro documento \
                                 como fuente o deshacé una de esas uniones."
                            ));
                        }
                    }
                    conn.execute(
                        "UPDATE similar_pair SET state = 'joined', chosen_relation = ?3, wiki_source = ?4,
                         shown_evidence_hash = ?5, decided_at = ?6 WHERE a = ?1 AND b = ?2",
                        params![a, b, relation, source, shown, now],
                    )
                    .map_err(sql_err)?;
                }
                Decision::Discard => {
                    conn.execute(
                        "UPDATE similar_pair SET state = 'discarded', chosen_relation = NULL, wiki_source = NULL,
                         shown_evidence_hash = ?3, decided_at = ?4 WHERE a = ?1 AND b = ?2",
                        params![a, b, shown, now],
                    )
                    .map_err(sql_err)?;
                }
                Decision::Undo => {
                    conn.execute(
                        "UPDATE similar_pair SET state = 'proposed', chosen_relation = NULL, wiki_source = NULL,
                         shown_evidence_hash = NULL, decided_at = NULL WHERE a = ?1 AND b = ?2",
                        params![a, b],
                    )
                    .map_err(sql_err)?;
                }
            }
        }
        Ok(())
    }

    /// What waits for a decision: only the pairs that may be one same
    /// document (D-497).
    pub fn similar_pending(&self, visible: &Visible) -> Result<SimilarPending, String> {
        let conn = self.conn();
        let asks = asks_person_sql("p.relation");
        let (seen_a, mut bound) = visible_content(visible, "p.a");
        let (seen_b, bound_b) = visible_content(visible, "p.b");
        bound.extend(bound_b);
        let pairs: i64 = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM similar_pair p
                     WHERE p.state = 'proposed' AND {asks} AND {seen_a} AND {seen_b}"
                ),
                params_from_iter(bound.iter()),
                |row| row.get(0),
            )
            .map_err(sql_err)?;
        let (seen_principal, bound) = visible_content(visible, "g.principal");
        let groups: i64 = conn
            .query_row(
                &format!(
                    "SELECT count(*) FROM similar_group g WHERE {seen_principal} AND EXISTS (
                        SELECT 1 FROM similar_group_member m JOIN similar_pair p
                          ON p.a = min(m.principal, m.content_id) AND p.b = max(m.principal, m.content_id)
                        WHERE m.principal = g.principal AND p.state = 'proposed' AND {asks})"
                ),
                params_from_iter(bound.iter()),
                |row| row.get(0),
            )
            .map_err(sql_err)?;
        let one = same_document_sql("p.relation");
        let (seen_a, mut bound) = visible_content(visible, "p.a");
        let (seen_b, bound_b) = visible_content(visible, "p.b");
        bound.extend(bound_b);
        let same_document_pairs: i64 = conn
            .query_row(
                &format!("SELECT count(*) FROM similar_pair p WHERE p.state = 'proposed' AND {one} AND {seen_a} AND {seen_b}"),
                params_from_iter(bound.iter()),
                |row| row.get(0),
            )
            .map_err(sql_err)?;
        Ok(SimilarPending { pairs, groups, same_document_pairs })
    }

    /// The provisional families, largest first.
    pub fn similar_groups(&self, visible: &Visible) -> Result<Vec<SimilarGroup>, String> {
        let conn = self.conn();
        let heads: Vec<(i64, String)> = {
            let mut statement = conn
                .prepare("SELECT principal, flags FROM similar_group ORDER BY members DESC, principal")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let mut out = Vec::new();
        for (principal, flags) in heads {
            let Some(head) = doc_ref(&conn, visible, principal)? else {
                continue;
            };
            let members: Vec<(i64, Option<String>, Option<f64>, Option<String>)> = {
                let mut statement = conn
                    .prepare(
                        "SELECT m.content_id, m.relation, m.cover, p.state FROM similar_group_member m
                         LEFT JOIN similar_pair p ON p.a = min(m.principal, m.content_id) AND p.b = max(m.principal, m.content_id)
                         WHERE m.principal = ?1 ORDER BY m.content_id = m.principal DESC, m.cover DESC, m.content_id",
                    )
                    .map_err(sql_err)?;
                let rows = statement
                    .query_map([principal], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
                    .map_err(sql_err)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(sql_err)?;
                rows
            };
            let mut list = Vec::new();
            let mut pending = 0;
            let mut pending_same_document = 0;
            for (content_id, relation, cover, state) in members {
                if state.as_deref() == Some("proposed") && relation.as_deref().is_some_and(asks_person) {
                    pending += 1;
                    if relation.as_deref().is_some_and(makes_one_document) {
                        pending_same_document += 1;
                    }
                }
                if let Some(doc) = doc_ref(&conn, visible, content_id)? {
                    list.push(GroupMember { doc, relation, cover, state });
                }
            }
            let template = super::template::summary(&conn, principal)?;
            out.push(SimilarGroup {
                principal: head,
                members: list,
                flags: serde_json::from_str(&flags).unwrap_or_default(),
                pending,
                pending_same_document,
                template_fields: template.map(|(fields, _)| fields),
                possibly_same: template.map(|(_, same)| same).unwrap_or(0),
            });
        }
        Ok(out)
    }
}

/// Whether joining `a` and `b` as one same document, with `source` as
/// the wiki's source, would leave no source at all for the documents
/// joined that way with them: the choices close a circle (A gives way to
/// B, B to C, C to A). A document stops feeding the wiki when, in any
/// pair joined as the same document, the other one was chosen
/// (`passed_over_sql`), so the set needs at least one document chosen in
/// every such pair it is in. A set without circles always has one.
/// Returns how many documents the set has when it would have none.
fn joined_without_source(conn: &Connection, a: i64, b: i64, source: i64) -> Result<Option<usize>, String> {
    let mut statement = conn
        .prepare(
            "SELECT a, b, wiki_source FROM similar_pair
             WHERE state = 'joined' AND wiki_source IS NOT NULL AND NOT (a = ?1 AND b = ?2)",
        )
        .map_err(sql_err)?;
    let mut pairs = statement
        .query_map(params![a, b], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)))
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    pairs.push((a, b, source));
    let mut neighbours: HashMap<i64, Vec<i64>> = HashMap::new();
    for &(x, y, _) in &pairs {
        neighbours.entry(x).or_default().push(y);
        neighbours.entry(y).or_default().push(x);
    }
    let mut joined: HashSet<i64> = HashSet::from([a]);
    let mut pending = vec![a];
    while let Some(next) = pending.pop() {
        for &other in neighbours.get(&next).into_iter().flatten() {
            if joined.insert(other) {
                pending.push(other);
            }
        }
    }
    let passed_over: HashSet<i64> = pairs
        .iter()
        .filter(|(x, _, _)| joined.contains(x))
        .map(|&(x, y, chosen)| if chosen == x { y } else { x })
        .collect();
    Ok((passed_over.len() == joined.len()).then_some(joined.len()))
}

/// Whether joining with this relation leaves only one of the two as the
/// wiki's source (catalog: `sameDocument`). The automatic one is done by
/// the base itself and never waits for a person.
pub fn makes_one_document(relation: &str) -> bool {
    types::catalog().relation(relation).is_some_and(|r| r.same_document && !r.automatic)
}

/// SQL condition: the relation in `column` makes both one document.
fn same_document_sql(column: &str) -> String {
    relation_sql(column, |r| r.same_document && !r.automatic)
}

/// Whether a proposal of this relation waits for a person (catalog:
/// `asksPerson`): the two may be one same document. The same template
/// with other data and the only similar are different documents and wait
/// for no one (D-497).
pub fn asks_person(relation: &str) -> bool {
    types::catalog().relation(relation).is_some_and(|r| r.asks_person && !r.automatic)
}

/// SQL condition: the relation in `column` waits for a person.
pub(crate) fn asks_person_sql(column: &str) -> String {
    relation_sql(column, |r| r.asks_person && !r.automatic)
}

fn relation_sql(column: &str, keep: impl Fn(&RelationDef) -> bool) -> String {
    let ids: Vec<String> = types::catalog()
        .relations
        .iter()
        .filter(|r| keep(r))
        .map(|r| format!("'{}'", r.id.replace('\'', "''")))
        .collect();
    if ids.is_empty() {
        "0".to_string()
    } else {
        format!("{column} IN ({})", ids.join(", "))
    }
}

/// The relations a person can choose from, as the catalog lists them.
pub fn relations() -> Vec<RelationDef> {
    types::catalog().relations.clone()
}

/// Computes what is stored for one content.
fn sign(content_id: i64, body: &str, text_at: i64, has_fields: bool, signatures: Vec<PdfSignature>) -> Signed {
    let body = format::comparable(body);
    let pages = text::pages(&body);
    let reading = marks::read(types::catalog(), &marks::Facts { pages: &pages, has_fields, signatures: &signatures });
    let prepared = Prepared::new(&pages, &reading.skip_pages, &reading.drop_phrases);
    let shingles = if prepared.word_count() >= MIN_WORDS { prepared.shingles() } else { Vec::new() };
    let minhash = minhash::signature(&shingles);
    Signed {
        content_id,
        text_at,
        words: prepared.word_count(),
        shingles,
        minhash,
        marks: reading.marks,
        signatures,
        skip_pages: reading.skip_pages,
        drop_phrases: reading.drop_phrases,
    }
}

fn save_signature(tx: &Transaction<'_>, signed: &Signed, engine: &str) -> Result<(), String> {
    let old: Option<Vec<u8>> = tx
        .query_row("SELECT shingles FROM similar_signature WHERE content_id = ?1", [signed.content_id], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    if let Some(old) = old {
        uncount(tx, &minhash::from_blob(&old))?;
        tx.execute("DELETE FROM similar_signature WHERE content_id = ?1", [signed.content_id])
            .map_err(sql_err)?;
    }
    let now = now_ms();
    tx.execute(
        "INSERT INTO similar_signature (content_id, engine, text_at, words, shingles, minhash, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            signed.content_id,
            engine,
            signed.text_at,
            signed.words as i64,
            minhash::to_blob(&signed.shingles),
            minhash::to_blob(&signed.minhash),
            now
        ],
    )
    .map_err(sql_err)?;
    {
        let mut statement = tx
            .prepare_cached(
                "INSERT INTO similar_common (shingle, docs) VALUES (?1, 1)
                 ON CONFLICT (shingle) DO UPDATE SET docs = docs + 1",
            )
            .map_err(sql_err)?;
        for shingle in &signed.shingles {
            statement.execute([*shingle as i64]).map_err(sql_err)?;
        }
        let mut statement = tx
            .prepare_cached("INSERT OR IGNORE INTO similar_band (band, bucket, content_id) VALUES (?1, ?2, ?3)")
            .map_err(sql_err)?;
        for (band, bucket) in minhash::buckets(&signed.minhash).into_iter().enumerate() {
            statement.execute(params![band as i64, bucket, signed.content_id]).map_err(sql_err)?;
        }
    }
    tx.execute(
        "INSERT OR REPLACE INTO document_marks (content_id, engine, marks, signatures, skip_pages, drop_phrases, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            signed.content_id,
            engine,
            serde_json::to_string(&signed.marks).map_err(json_err)?,
            serde_json::to_string(&signed.signatures).map_err(json_err)?,
            serde_json::to_string(&signed.skip_pages).map_err(json_err)?,
            serde_json::to_string(&signed.drop_phrases).map_err(json_err)?,
            now
        ],
    )
    .map_err(sql_err)?;
    tx.execute(
        "INSERT OR REPLACE INTO similar_dirty (content_id, since) VALUES (?1, ?2)",
        params![signed.content_id, now],
    )
    .map_err(sql_err)?;
    Ok(())
}

fn uncount(tx: &Transaction<'_>, shingles: &[u64]) -> Result<(), String> {
    let mut statement = tx
        .prepare_cached("UPDATE similar_common SET docs = docs - 1 WHERE shingle = ?1")
        .map_err(sql_err)?;
    for shingle in shingles {
        statement.execute([*shingle as i64]).map_err(sql_err)?;
    }
    tx.execute("DELETE FROM similar_common WHERE docs <= 0", []).map_err(sql_err)?;
    Ok(())
}

/// The shingles that are routine text in the archive today (pass 0).
fn routine_shingles(conn: &Connection) -> Result<HashSet<u64>, String> {
    let documents: i64 = conn
        .query_row("SELECT count(*) FROM similar_signature WHERE length(shingles) > 0", [], |row| row.get(0))
        .map_err(sql_err)?;
    let threshold = ROUTINE_MIN_DOCS.max((ROUTINE_SHARE * documents as f64).ceil() as i64);
    let mut statement = conn
        .prepare("SELECT shingle FROM similar_common WHERE docs >= ?1")
        .map_err(sql_err)?;
    let rows = statement
        .query_map([threshold], |row| row.get::<_, i64>(0))
        .map_err(sql_err)?;
    let mut out = HashSet::new();
    for row in rows {
        out.insert(row.map_err(sql_err)? as u64);
    }
    Ok(out)
}

/// Words, shingle count and MinHash of a content.
fn signature_of(conn: &Connection, content_id: i64) -> Result<Option<(usize, usize, Vec<u64>)>, String> {
    conn.query_row(
        "SELECT words, length(shingles) / 8, minhash FROM similar_signature WHERE content_id = ?1",
        [content_id],
        |row| Ok((row.get::<_, i64>(0)? as usize, row.get::<_, i64>(1)? as usize, minhash::from_blob(&row.get::<_, Vec<u8>>(2)?))),
    )
    .optional()
    .map_err(sql_err)
}

/// Containment estimated from two signatures and their sizes.
fn estimated_containment(a: &Option<(usize, usize, Vec<u64>)>, b: &(usize, usize, Vec<u64>)) -> f64 {
    let Some(a) = a else {
        return 0.0;
    };
    let jaccard = minhash::estimate(&a.2, &b.2);
    let smaller = a.1.min(b.1);
    if smaller == 0 {
        return 0.0;
    }
    (jaccard * (a.1 + b.1) as f64 / ((1.0 + jaccard) * smaller as f64)).min(1.0)
}

/// The PDF files of a content, where they can be read now.
fn pdf_paths(conn: &Connection, content_id: i64) -> Result<Vec<(String, PathBuf)>, String> {
    let mut statement = conn
        .prepare(
            "SELECT f.hash, o.location, min(l.rel_path) FROM file f
             JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done' AND l.ext = 'pdf'
             JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
             WHERE f.content_id = ?1
             GROUP BY f.hash",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([content_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    Ok(rows
        .into_iter()
        .filter_map(|(hash, root, rel_path)| LocalFolderProvider::new(root).local_path(&rel_path).map(|path| (hash, path)))
        .collect())
}

/// A content ready to be compared, from what is stored.
fn load_doc(conn: &Connection, content_id: i64, routine: &HashSet<u64>) -> Result<Option<Doc>, String> {
    let row: Option<(String, String, Option<String>, String, String, String, Vec<u8>)> = conn
        .query_row(
            "SELECT d.body, d.engine, fd.body, m.marks, m.skip_pages, m.drop_phrases, s.shingles
             FROM similar_signature s
             JOIN derivative d ON d.content_id = s.content_id AND d.kind = 'text'
             JOIN document_marks m ON m.content_id = s.content_id
             LEFT JOIN derivative fd ON fd.content_id = s.content_id AND fd.kind = 'fields'
             WHERE s.content_id = ?1",
            [content_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((body, engine, fields, marks, skip_pages, drop_phrases, shingles)) = row else {
        return Ok(None);
    };
    let skip: Vec<usize> = serde_json::from_str(&skip_pages).unwrap_or_default();
    let drop: Vec<String> = serde_json::from_str(&drop_phrases).unwrap_or_default();
    let marks: Vec<Mark> = serde_json::from_str(&marks).unwrap_or_default();
    let body = format::comparable(&body);
    let prepared = Prepared::new(&text::pages(&body), &skip, &drop);
    let mut routine_words = vec![false; prepared.words.len()];
    if !routine.is_empty() && prepared.words.len() >= SHINGLE_WORDS {
        for start in 0..=prepared.words.len() - SHINGLE_WORDS {
            let window = &prepared.words[start..start + SHINGLE_WORDS];
            if routine.contains(&text::shingle_hash(window.iter().map(|w| w.key.as_str()))) {
                routine_words[start..start + SHINGLE_WORDS].iter_mut().for_each(|w| *w = true);
            }
        }
    }
    let own_words = routine_words.iter().filter(|r| !**r).count();
    let fields: Vec<(String, String)> = fields
        .and_then(|body| serde_json::from_str::<Vec<serde_json::Value>>(&body).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|field| {
            (
                field["name"].as_str().unwrap_or_default().to_string(),
                field["value"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();
    let mut kinds: Vec<String> = marks.iter().map(|m| m.kind.clone()).collect();
    kinds.sort();
    kinds.dedup();
    let index = TextIndex::new(&prepared);
    Ok(Some(Doc {
        id: content_id,
        recognized: text::is_recognized(&body) || engine != crate::docbase::text::EXTRACTION_ENGINE,
        prepared,
        shingles: minhash::from_blob(&shingles),
        routine: routine_words,
        own_words,
        marks: kinds,
        fields,
        key: key_data::load(conn, content_id)?,
        index,
    }))
}

/// Measures, aligns and proposes for one pair (`a.id < b.id`).
fn compare(a: &Doc, b: &Doc, routine: &HashSet<u64>) -> Option<(propose::Proposal, Measures, serde_json::Value)> {
    let shingles = minhash::overlap(&a.shingles, &b.shingles, &|s| routine.contains(&s));
    if shingles.0 == 0 || (shingles.0 as f64) < MEASURE_MIN * shingles.1.min(shingles.2) as f64 {
        return None;
    }
    let (ids_a, ids_b) = align::intern(
        a.prepared.words.iter().map(|w| w.key.as_str()),
        b.prepared.words.iter().map(|w| w.key.as_str()),
    );
    let ops = align::align(&ids_a, &ids_b);
    let mut found: Changes = changes::summarize(&a.prepared, &b.prepared, &ops);
    changes::add_fields(&mut found, &a.fields, &b.fields);
    let (mut matched_a, mut matched_b) = (0, 0);
    for op in &ops {
        if let align::Op::Same { a: ra, b: rb } = op {
            matched_a += ra.clone().filter(|i| !a.routine[*i]).count();
            matched_b += rb.clone().filter(|j| !b.routine[*j]).count();
        }
    }
    let side_a = Side { content_id: a.id, own_words: a.own_words, matched: matched_a, recognized: a.recognized, marks: a.marks.clone() };
    let side_b = Side { content_id: b.id, own_words: b.own_words, matched: matched_b, recognized: b.recognized, marks: b.marks.clone() };
    let measures = propose::measures(&side_a, &side_b, shingles);
    let proposal = propose::propose(&side_a, &side_b, &measures, &found)?;
    // Before it is a version of the other, or only similar: their key data (D-497).
    let differing = if matches!(proposal.relation.as_str(), "version" | "similar") {
        key_data::differing((&a.key, &a.index), (&b.key, &b.index))
    } else {
        Vec::new()
    };
    let proposal = propose::with_key_data(proposal, &differing)?;
    let mut evidence = serde_json::json!({
        "engine": ENGINE,
        "rule": proposal.rule,
        "measures": measures,
        "words": { "a": a.prepared.word_count(), "b": b.prepared.word_count() },
        "ownWords": { "a": a.own_words, "b": b.own_words },
        "recognized": { "a": a.recognized, "b": b.recognized },
        "marks": { "a": a.marks, "b": b.marks },
        "changes": found,
    });
    if !differing.is_empty() {
        evidence["keyData"] = serde_json::json!({ "differ": differing });
    }
    Some((proposal, measures, evidence))
}

fn save_pair(
    tx: &Transaction<'_>,
    a: i64,
    b: i64,
    outcome: Option<(propose::Proposal, Measures, serde_json::Value)>,
    engine: &str,
) -> Result<(), String> {
    let Some((proposal, measures, evidence)) = outcome else {
        // A pair a rule of the type proposed was never measured: it stays.
        tx.execute(
            "DELETE FROM similar_pair WHERE a = ?1 AND b = ?2 AND state = 'proposed' AND origin = 'measured'",
            params![a, b],
        )
            .map_err(sql_err)?;
        return Ok(());
    };
    let evidence = serde_json::to_string(&evidence).map_err(json_err)?;
    let hash = format!("{:016x}", text::fnv1a(evidence.as_bytes()));
    tx.execute(
        "INSERT INTO similar_pair (a, b, relation, subject, certainty, jaccard, containment, cover, similarity,
                                   description, suggested_source, evidence, evidence_hash, engine, computed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT (a, b) DO UPDATE SET
            relation = excluded.relation, subject = excluded.subject, certainty = excluded.certainty,
            jaccard = excluded.jaccard, containment = excluded.containment, cover = excluded.cover,
            similarity = excluded.similarity, description = excluded.description,
            suggested_source = excluded.suggested_source, evidence = excluded.evidence,
            evidence_hash = excluded.evidence_hash, engine = excluded.engine, computed_at = excluded.computed_at,
            origin = 'measured', measured = NULL, type_rule = NULL",
        params![
            a,
            b,
            proposal.relation,
            proposal.subject,
            proposal.certainty,
            measures.jaccard,
            measures.containment,
            measures.cover,
            measures.similarity,
            proposal.description,
            proposal.suggested_source,
            evidence,
            hash,
            engine,
            now_ms()
        ],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Documents linked by `relations`, as families against a principal
/// (`families`), with the links between them: the families shown
/// (`GROUP_RELATIONS`) or the units for types and templates
/// (`UNIT_RELATIONS`).
pub(crate) fn grouped(conn: &Connection, relations: &[&str]) -> Result<(Vec<Family>, Links), String> {
    let generated = super::generate::generated_contents(conn)?;
    let mut adjacent: Links = BTreeMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT a, b, CASE WHEN state = 'joined' THEN chosen_relation ELSE relation END, cover
                 FROM similar_pair WHERE state != 'discarded'",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, f64>(3)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (a, b, relation, cover) = row.map_err(sql_err)?;
            if !relations.contains(&relation.as_str()) {
                continue;
            }
            adjacent.entry(a).or_default().insert(b, (relation.clone(), cover));
            adjacent.entry(b).or_default().insert(a, (relation, cover));
        }
    }
    // How complete each document is: a party's signature first, then
    // the amount of text, then the oldest content.
    let mut completeness: HashMap<i64, (bool, i64)> = HashMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT s.content_id, s.words, coalesce(m.marks, '[]') FROM similar_signature s
                 LEFT JOIN document_marks m ON m.content_id = s.content_id",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (id, words, marks) = row.map_err(sql_err)?;
            let marks: Vec<Mark> = serde_json::from_str(&marks).unwrap_or_default();
            completeness.insert(id, (marks.iter().any(|m| m.kind == "party-signature"), words));
        }
    }
    let groups = families(&adjacent, &|id| {
        if generated.contains(&id) {
            (false, -1)
        } else {
            completeness.get(&id).copied().unwrap_or((false, 0))
        }
    });
    Ok((groups, adjacent))
}

/// A provisional family.
#[derive(Debug, Clone, PartialEq)]
pub struct Family {
    pub principal: i64,
    pub members: Vec<i64>,
    pub flags: Vec<String>,
}

/// Families against a principal: in each set of linked documents, the
/// most complete one is the principal and only the documents linked to
/// it join; the rest form families of their own the same way.
pub fn families(
    adjacent: &BTreeMap<i64, BTreeMap<i64, (String, f64)>>,
    completeness: &dyn Fn(i64) -> (bool, i64),
) -> Vec<Family> {
    let mut seen: HashSet<i64> = HashSet::new();
    let mut out = Vec::new();
    for start in adjacent.keys() {
        if seen.contains(start) {
            continue;
        }
        let mut component: BTreeSet<i64> = BTreeSet::new();
        let mut queue = vec![*start];
        while let Some(node) = queue.pop() {
            if !component.insert(node) {
                continue;
            }
            seen.insert(node);
            if let Some(links) = adjacent.get(&node) {
                queue.extend(links.keys().filter(|n| !component.contains(n)));
            }
        }
        let size = component.len();
        let mut remaining = component;
        while remaining.len() >= 2 {
            let principal = *remaining
                .iter()
                .max_by(|x, y| {
                    let (cx, cy) = (completeness(**x), completeness(**y));
                    cx.cmp(&cy).then(y.cmp(x))
                })
                .expect("not empty");
            let mut members = vec![principal];
            if let Some(links) = adjacent.get(&principal) {
                members.extend(links.keys().filter(|n| remaining.contains(n)));
            }
            for member in &members {
                remaining.remove(member);
            }
            if members.len() >= 2 {
                let mut flags = Vec::new();
                if members.len() > GROUP_LARGE {
                    flags.push("large".to_string());
                }
                if size > members.len() {
                    flags.push("dispersed".to_string());
                }
                members.sort_unstable();
                out.push(Family { principal, members, flags });
            }
        }
    }
    out
}

pub(super) fn content_of_location(conn: &Connection, location_id: i64) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.id = ?1",
        [location_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_err)
}

/// SQL condition: the content in `column` has a location the caller sees.
fn visible_content(visible: &Visible, column: &str) -> (String, Vec<Value>) {
    let (condition, bound) = visible.condition("l.origin_id");
    (
        format!(
            "EXISTS (SELECT 1 FROM file f JOIN location l ON l.file_hash = f.hash
                     WHERE f.content_id = {column} AND {condition} AND NOT {})",
            super::super::excluded_sql!()
        ),
        bound.into_iter().map(Value::Text).collect(),
    )
}

/// The location that stands for a content: the first visible one.
pub(super) fn doc_ref(conn: &Connection, visible: &Visible, content_id: i64) -> Result<Option<DocRef>, String> {
    let (condition, bound) = visible.condition("l.origin_id");
    let bound: Vec<Value> = std::iter::once(Value::Integer(content_id))
        .chain(bound.into_iter().map(Value::Text))
        .collect();
    conn.query_row(
        &format!(
            "SELECT l.id, o.name, l.rel_path, l.name,
                    (SELECT count(*) FROM location l2 JOIN file f2 ON f2.hash = l2.file_hash WHERE f2.content_id = f.content_id)
             FROM location l
             JOIN origin o ON o.id = l.origin_id
             JOIN file f ON f.hash = l.file_hash
             WHERE f.content_id = ? AND {condition} AND NOT {}
             ORDER BY o.name COLLATE NOCASE, l.rel_path COLLATE NOCASE
             LIMIT 1",
            super::super::excluded_sql!()
        ),
        params_from_iter(bound),
        |row| {
            Ok(DocRef {
                content_id,
                location_id: row.get(0)?,
                origin_name: row.get(1)?,
                rel_path: row.get(2)?,
                name: row.get(3)?,
                copies: row.get(4)?,
            })
        },
    )
    .optional()
    .map_err(sql_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(i64, i64)]) -> BTreeMap<i64, BTreeMap<i64, (String, f64)>> {
        let mut adjacent: BTreeMap<i64, BTreeMap<i64, (String, f64)>> = BTreeMap::new();
        for (a, b) in edges {
            adjacent.entry(*a).or_default().insert(*b, ("same-template".into(), 0.9));
            adjacent.entry(*b).or_default().insert(*a, ("same-template".into(), 0.9));
        }
        adjacent
    }

    #[test]
    fn a_chain_does_not_make_a_family() {
        // 1 ~ 2 ~ 3, and 3 is not similar to 1, the most complete.
        let words = |id: i64| (false, match id { 1 => 900, 2 => 500, _ => 400 });
        let found = families(&graph(&[(1, 2), (2, 3)]), &words);
        assert_eq!(found, vec![Family { principal: 1, members: vec![1, 2], flags: vec!["dispersed".into()] }]);
    }

    #[test]
    fn the_signed_copy_is_the_principal_and_all_join_it() {
        let complete = |id: i64| (id == 3, 100);
        let found = families(&graph(&[(1, 2), (1, 3), (2, 3)]), &complete);
        assert_eq!(found, vec![Family { principal: 3, members: vec![1, 2, 3], flags: vec![] }]);
    }

    /// What waits for a person in a copy of a real base, before and after
    /// this engine computes it again (D-497). Numbers only: no name, no
    /// text. The copy is opened in a scratch folder; its files are only
    /// read (the signatures of its PDFs).
    ///
    /// ```text
    /// MICELYA_DOCBASE_COPY="C:\copia\private.sqlite3" \
    ///     cargo test --lib pairs_waiting_for_a_person_in_a_copy_of_the_base -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "reads a copy of a real base"]
    fn pairs_waiting_for_a_person_in_a_copy_of_the_base() {
        if std::env::var("MICELYA_DOCBASE_COPY").is_err() {
            println!("MICELYA_DOCBASE_COPY is not set: nothing to measure");
            return;
        }
        let Some((_scratch, base)) = crate::docbase::knowledge::check::open_base() else { return };
        let count = |base: &DocBase| -> BTreeMap<(String, String, bool), i64> {
            let conn = base.conn();
            let mut statement = conn.prepare("SELECT relation, state, count(*) FROM similar_pair GROUP BY 1, 2").unwrap();
            let rows = statement
                .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            rows.into_iter().map(|(relation, state, n)| ((relation.clone(), state, asks_person(&relation)), n)).collect()
        };
        let before = count(&base);
        let started = std::time::Instant::now();
        let mut steps = 0;
        while base.similar_step(50).unwrap() > 0 {
            steps += 1;
            assert!(steps < 100_000, "the engine does not settle");
        }
        println!("computed again in {:.1} s ({steps} steps)", started.elapsed().as_secs_f32());
        let after = count(&base);
        for (label, counts) in [("before", &before), ("after", &after)] {
            let total: i64 = counts.values().sum();
            let asking: i64 = counts.iter().filter(|((_, state, asks), _)| state == "proposed" && *asks).map(|(_, n)| n).sum();
            println!("{label}: {total} pairs; waiting for a person with the new rule: {asking}");
            for ((relation, state, asks), n) in counts {
                println!("   {relation:<24} {state:<10} {:<12} {n}", if *asks { "asks" } else { "information" });
            }
        }
        let conn = base.conn();
        let rules: Vec<(String, i64)> = {
            let mut statement = conn
                .prepare("SELECT coalesce(json_extract(evidence, '$.rule'), '-'), count(*) FROM similar_pair GROUP BY 1")
                .unwrap();
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
            rows
        };
        println!("by rule: {rules:?}");
        let differ: Vec<(String, i64)> = {
            let mut statement = conn
                .prepare(
                    "SELECT j.value, count(*) FROM similar_pair, json_each(json_extract(evidence, '$.keyData.differ')) j GROUP BY 1",
                )
                .unwrap();
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
            rows
        };
        println!("pairs taken down by their key data, by key: {differ:?}");
        let families: i64 = conn.query_row("SELECT count(*) FROM similar_group", [], |row| row.get(0)).unwrap();
        let templates: i64 = conn.query_row("SELECT count(*) FROM similar_template", [], |row| row.get(0)).unwrap();
        println!("families: {families}; templates: {templates}");
        drop(conn);
        let pending = base.similar_pending(&Visible::All).unwrap();
        println!(
            "Atención: {} pairs wait ({} of one same document), in {} families",
            pending.pairs, pending.same_document_pairs, pending.groups
        );
    }
}
