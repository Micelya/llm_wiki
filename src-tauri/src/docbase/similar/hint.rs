//! The vectors as a signal of the type (C2 of 2 nov). They propose, they
//! never decide: the catalog gives the names and the model is still asked.
//!
//! - **The hint of the neighbours** (`tipo-3`): the question of the type of
//!   a document carries the 3 documents already typed whose vectors are the
//!   nearest to its own, with their type and their likeness. The types of
//!   the catalog and those a person confirmed weigh more than the ones the
//!   model proposed: the question says which is which, and the suggestion
//!   and the doubtful mark count them so. A setting of its own, «Pista del
//!   tipo por parecido» (`HINT_SETTING`), on by default (D-805): off, the
//!   question carries no neighbour, and the groups, the suggestion for the
//!   documents without a type and the doubtful mark go on.
//! - **The groups by likeness:** the documents alone (no family) still to
//!   ask whose vectors are all near one another, from the threshold of the
//!   groups (a setting of the base, 0,85 by default, C0 of 2 nov), at most
//!   `GROUP_MAX`, go in ONE question with one answer for each: each one gets
//!   its own type. It is not a family, where the members take the type of
//!   the principal.
//! - **The doubtful mark:** when the answer of the model goes against its 3
//!   nearest neighbours, all from the threshold of the very close ones and
//!   of one same type of the catalog or confirmed, the type is kept marked
//!   as doubtful (`type_doubt`) and shown in Atención. It is not asked again.
//! - **The rescue of the documents without a type:** the type its 3 nearest
//!   neighbours give, weighed, for a document the step of the type leaves
//!   without one (its question failed, or it has no word to ask with). Shown
//!   to a person; nothing is written as its type.
//! - **A document of a type of low confidence** (decision 5 of 2 nov, E3):
//!   it takes no part. It is no neighbour, no example, it goes in no group,
//!   its question is the one of before (no hint), it gets no mark and no
//!   suggestion: what comes out of it is only for queries.
//! - **With «Ninguno» in «Calcular vectores»** there are no signals: the
//!   question of the type is the one of before, without hint nor groups.
//! - **The type waits for its vector** only with the hint on and while a
//!   failure of Micelya Core holds the step of the vectors (the AI is never
//!   skipped, P-612); with the hint off it does not wait.
//!
//! Production questions per document (D-672): the groups take away about
//! 0,05 questions of type per document (C0 of 2 nov, with 0,85); the hint
//! and the mark add none; the vectors are the 0,013 calls to Core of C1.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::super::{now_ms, sql_err};
use super::classify::TypeDecisions;
use super::types::Catalog;

/// The setting of the hint: «0» turns it off; on by default (D-805).
pub(crate) const HINT_SETTING: &str = "types:vector-hint";
/// The most documents of one group by likeness (C2 of 2 nov).
pub const GROUP_MAX: usize = 10;
/// The neighbours of the hint, of the mark and of the suggestion.
pub const NEIGHBOURS: usize = 3;
/// What a type the model proposed and nobody confirmed weighs, against 1
/// for one of the catalog or confirmed by a person (provisional, assumed
/// D-500: the long run of control measures the hint, D-805).
pub const PROPOSED_WEIGHT: f64 = 0.5;
/// How the hint starts in the question: the control run looks for it.
pub(crate) const HINT_HEADER: &str = "Documentos ya clasificados que más se parecen a este por su texto";

/// Whether the hint goes in the questions of the type.
pub(crate) fn hint_on(conn: &Connection) -> Result<bool, String> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [HINT_SETTING], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(value.as_deref().map(str::trim) != Some("0"))
}

pub(crate) fn set_hint_on(conn: &Connection, on: bool) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
        params![HINT_SETTING, if on { "1" } else { "0" }],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// Where the type of a neighbour comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// A type of the catalog.
    Catalog,
    /// A type the model proposed and a person confirmed.
    Confirmed,
    /// A type the model proposed that nobody confirmed yet: it weighs less.
    Proposed,
}

impl Origin {
    pub fn weight(self) -> f64 {
        match self {
            Origin::Catalog | Origin::Confirmed => 1.0,
            Origin::Proposed => PROPOSED_WEIGHT,
        }
    }
}

/// A document already typed near another one.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Neighbour {
    pub version_id: i64,
    pub doc_type: String,
    /// The name of a type the model proposed or a person confirmed.
    pub name: Option<String>,
    pub origin: Origin,
    /// The cosine of the two vectors, from 0 to 1.
    pub similarity: f64,
}

struct Typed {
    version: i64,
    doc_type: String,
    name: Option<String>,
    origin: Origin,
}

/// What the vectors say now, for the step of the type: `None` with
/// «Ninguno» (`load`).
pub(super) struct Signals {
    /// The hint goes in the question.
    pub hint: bool,
    pub group_threshold: f64,
    pub close_threshold: f64,
    /// The vectors of the text of now, normalized; none for a document
    /// without text.
    vectors: HashMap<i64, Vec<f32>>,
    /// The documents with a row of the model of now (with or without a vector).
    with_row: BTreeSet<i64>,
    /// The documents typed that may be neighbours: with a vector, not of
    /// low confidence and not doubtful.
    typed: Vec<Typed>,
    low: BTreeSet<i64>,
    /// A failure of Micelya Core holds the step of the vectors.
    core_held: bool,
}

fn normalized(vector: Vec<f32>) -> Option<Vec<f32>> {
    let norm = vector.iter().map(|n| f64::from(*n) * f64::from(*n)).sum::<f64>().sqrt();
    (norm > 0.0).then(|| vector.into_iter().map(|n| (f64::from(n) / norm) as f32).collect())
}

fn dot(a: &[f32], b: &[f32]) -> f64 {
    a.iter().zip(b).map(|(x, y)| f64::from(*x) * f64::from(*y)).sum()
}

/// The signals of the vectors now, read from the base in one go (P-612,
/// point 3); `None` when «Calcular vectores» is «Ninguno».
pub(super) fn load(conn: &Connection, catalog: &Catalog, decisions: &TypeDecisions) -> Result<Option<Signals>, String> {
    let Some(config) = crate::docbase::vectors::chosen(conn)? else {
        return Ok(None);
    };
    let thresholds = crate::docbase::thresholds::thresholds(conn)?;
    let mut vectors = HashMap::new();
    let mut with_row = BTreeSet::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT v.version_id, v.vector FROM version_vector v
                 JOIN derivative d ON d.version_id = v.version_id AND d.kind = 'text'
                 WHERE v.model = ?1 AND v.dimension = ?2 AND v.text_at = d.created_at",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map(params![config.model, config.dimension as i64], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<Vec<u8>>>(1)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (version, blob) = row.map_err(sql_err)?;
            with_row.insert(version);
            if let Some(vector) = blob.map(|b| crate::docbase::vectors::from_blob(&b)).and_then(normalized) {
                vectors.insert(version, vector);
            }
        }
    }
    let low = crate::docbase::safe_words::low_confidence(conn)?;
    let doubtful: BTreeSet<i64> = {
        let mut statement = conn.prepare("SELECT version_id FROM type_doubt").map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut typed = Vec::new();
    {
        let mut statement = conn
            .prepare("SELECT version_id, doc_type, proposed, proposed_name FROM document_type ORDER BY version_id")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?, row.get::<_, Option<String>>(3)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (version, doc_type, proposed, name) = row.map_err(sql_err)?;
            if !vectors.contains_key(&version) || low.contains(&version) || doubtful.contains(&version) {
                continue;
            }
            let origin = if proposed {
                Origin::Proposed
            } else if catalog.doc_type(&doc_type).is_some() {
                Origin::Catalog
            } else {
                Origin::Confirmed
            };
            let name = match origin {
                Origin::Catalog => None,
                Origin::Confirmed => decisions.confirmed.get(&doc_type).map(|(n, _)| n.clone()).or(name),
                Origin::Proposed => name,
            };
            typed.push(Typed { version, doc_type, name, origin });
        }
    }
    let core_held = crate::docbase::failures::holds(conn, crate::docbase::vectors::STAGE, &crate::docbase::vectors::about())?;
    Ok(Some(Signals {
        hint: hint_on(conn)?,
        group_threshold: thresholds.group_similarity,
        close_threshold: thresholds.close_similarity,
        vectors,
        with_row,
        typed,
        low,
        core_held,
    }))
}

impl Signals {
    fn similarity(&self, a: i64, b: i64) -> Option<f64> {
        Some(dot(self.vectors.get(&a)?, self.vectors.get(&b)?))
    }

    /// Whether a document takes part in the signals: with a vector and not
    /// of low confidence.
    pub(crate) fn takes_part(&self, version: i64) -> bool {
        self.vectors.contains_key(&version) && !self.low.contains(&version)
    }

    /// The 3 typed documents nearest to `of`, outside `apart` (its own
    /// family or group), the nearest first.
    pub(crate) fn neighbours(&self, of: i64, apart: &BTreeSet<i64>) -> Vec<Neighbour> {
        if !self.takes_part(of) {
            return Vec::new();
        }
        let vector = &self.vectors[&of];
        let mut near: Vec<Neighbour> = self
            .typed
            .iter()
            .filter(|t| t.version != of && !apart.contains(&t.version))
            .map(|t| Neighbour {
                version_id: t.version,
                doc_type: t.doc_type.clone(),
                name: t.name.clone(),
                origin: t.origin,
                similarity: dot(vector, &self.vectors[&t.version]).clamp(-1.0, 1.0),
            })
            .collect();
        near.sort_by(|a, b| b.similarity.total_cmp(&a.similarity).then(a.version_id.cmp(&b.version_id)));
        near.truncate(NEIGHBOURS);
        near
    }

    /// The hint of the question of `of`, or none (the hint off, no neighbour).
    pub(crate) fn hint_for(&self, of: i64, apart: &BTreeSet<i64>) -> Option<String> {
        if !self.hint {
            return None;
        }
        let near = self.neighbours(of, apart);
        (!near.is_empty()).then(|| hint_text(&near))
    }

    /// Whether the question of a unit waits for the vector of its principal:
    /// with the hint on, while a failure of Core holds the step of the vectors.
    pub(crate) fn waits(&self, principal: i64) -> bool {
        self.hint && self.core_held && !self.low.contains(&principal) && !self.with_row.contains(&principal)
    }

    /// The groups by likeness among `alone` (documents alone still to ask):
    /// every two of a group are near from the threshold of the groups (not a
    /// chain), at most `GROUP_MAX` each, the nearest pairs first. Only the
    /// groups of two or more, each by its documents in order.
    pub(crate) fn groups(&self, alone: &[i64]) -> Vec<Vec<i64>> {
        let ids: Vec<i64> = alone.iter().copied().filter(|id| self.takes_part(*id)).collect();
        let mut pairs: Vec<(f64, i64, i64)> = Vec::new();
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                let similarity = self.similarity(*a, *b).unwrap_or(0.0);
                if similarity >= self.group_threshold {
                    pairs.push((similarity, (*a).min(*b), (*a).max(*b)));
                }
            }
        }
        pairs.sort_by(|x, y| y.0.total_cmp(&x.0).then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2)));
        let mut group_of: BTreeMap<i64, usize> = ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
        let mut groups: Vec<Vec<i64>> = ids.iter().map(|id| vec![*id]).collect();
        for (_, a, b) in pairs {
            let (ga, gb) = (group_of[&a], group_of[&b]);
            if ga == gb || groups[ga].len() + groups[gb].len() > GROUP_MAX {
                continue;
            }
            let all_near = groups[ga]
                .iter()
                .all(|x| groups[gb].iter().all(|y| self.similarity(*x, *y).unwrap_or(0.0) >= self.group_threshold));
            if !all_near {
                continue;
            }
            let moved = std::mem::take(&mut groups[gb]);
            for id in &moved {
                group_of.insert(*id, ga);
            }
            groups[ga].extend(moved);
        }
        let mut out: Vec<Vec<i64>> = groups
            .into_iter()
            .filter(|g| g.len() >= 2)
            .map(|mut g| {
                g.sort();
                g
            })
            .collect();
        out.sort();
        out
    }

    /// The doubtful mark of `version` answered `answered`: its 3 nearest
    /// neighbours outside `apart`, when all are very close, share a type of
    /// the catalog or confirmed, and the answer is another one.
    pub(crate) fn doubt(&self, version: i64, answered: &str, apart: &BTreeSet<i64>) -> Option<Vec<Neighbour>> {
        let near = self.neighbours(version, apart);
        if near.len() < NEIGHBOURS || near.iter().any(|n| n.similarity < self.close_threshold || n.origin == Origin::Proposed) {
            return None;
        }
        let first = &near[0].doc_type;
        (near.iter().all(|n| &n.doc_type == first) && first != answered).then_some(near)
    }

    /// The type the 3 nearest neighbours of `version` give, each weighed by
    /// its likeness and its origin, with them; none without neighbours.
    pub(crate) fn suggestion(&self, version: i64) -> Option<(String, f64, Vec<Neighbour>)> {
        let near = self.neighbours(version, &BTreeSet::new());
        let mut score: BTreeMap<&str, f64> = BTreeMap::new();
        for n in &near {
            *score.entry(n.doc_type.as_str()).or_default() += n.similarity.max(0.0) * n.origin.weight();
        }
        let (best, _) = score.into_iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(a.0)))?;
        let best = best.to_string();
        let likeness = near.iter().filter(|n| n.doc_type == best).map(|n| n.similarity).fold(0.0, f64::max);
        Some((best, likeness, near))
    }
}

/// A likeness as the questions and the screens write it: «0,91».
pub(crate) fn likeness(similarity: f64) -> String {
    format!("{similarity:.2}").replace('.', ",")
}

/// The hint, in words, for the question of the type.
pub(crate) fn hint_text(near: &[Neighbour]) -> String {
    let lines: Vec<String> = near
        .iter()
        .map(|n| {
            let origin = match (n.origin, n.name.as_deref().map(str::trim).filter(|s| !s.is_empty())) {
                (Origin::Catalog, _) => "del catálogo".to_string(),
                (Origin::Confirmed, Some(name)) => format!("confirmado por una persona: «{name}»"),
                (Origin::Confirmed, None) => "confirmado por una persona".to_string(),
                (Origin::Proposed, Some(name)) => format!("propuesto por el modelo, sin confirmar: «{name}»"),
                (Origin::Proposed, None) => "propuesto por el modelo, sin confirmar".to_string(),
            };
            format!("- {} ({origin}): parecido {}", n.doc_type, likeness(n.similarity))
        })
        .collect();
    format!(
        "{HINT_HEADER} (el parecido va de 0 a 1):\n{}\n\
         Es una pista, no una prueba: si el contenido dice otra cosa, manda el contenido. Pesan más los tipos del catálogo \
         o confirmados por una persona que los propuestos por el modelo sin confirmar. Si el documento es del mismo tipo que \
         alguno de ellos, usá ese mismo identificador en vez de proponer otro con un nombre parecido.\n",
        lines.join("\n")
    )
}

/// Keeps or takes away the doubtful mark of a document just answered.
pub(crate) fn note_doubt(conn: &Connection, version: i64, answered: &str, near: Option<&[Neighbour]>) -> Result<(), String> {
    let Some(near) = near else {
        conn.execute("DELETE FROM type_doubt WHERE version_id = ?1", [version]).map_err(sql_err)?;
        return Ok(());
    };
    let neighbours = serde_json::to_string(&near.iter().map(|n| (n.version_id, n.similarity)).collect::<Vec<_>>()).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO type_doubt (version_id, doc_type, neighbour_type, neighbours, computed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![version, answered, near[0].doc_type, neighbours, now_ms()],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// A document with its doubtful type, for Atención.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DoubtfulType {
    pub version_id: i64,
    pub location_id: Option<i64>,
    pub name: String,
    pub path: String,
    pub first_page: Option<u32>,
    pub last_page: Option<u32>,
    /// The type the model gave.
    pub doc_type: String,
    /// The type its 3 very close neighbours share.
    pub neighbour_type: String,
    /// The likeness of each of the 3.
    pub likeness: Vec<f64>,
}

/// A document without a type, with the type its neighbours suggest.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedType {
    pub version_id: i64,
    pub location_id: Option<i64>,
    pub name: String,
    pub path: String,
    pub first_page: Option<u32>,
    pub last_page: Option<u32>,
    pub doc_type: String,
    /// The likeness of the nearest neighbour of that type.
    pub likeness: f64,
    pub neighbours: Vec<Neighbour>,
    /// Why it has no type: its question failed, or it has no word to ask with.
    pub why: String,
}

/// What the vectors say of the types, for Atención: the doubtful ones and
/// the suggestions for the documents without a type. Reads only.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypesBySimilarity {
    pub doubtful: Vec<DoubtfulType>,
    pub suggested: Vec<SuggestedType>,
}

/// Where a version is: its location, the name of its file, `<origin>/<path>`
/// and its pages when it is a part of a file.
fn place(conn: &Connection, version: i64) -> Result<(Option<i64>, String, String, Option<u32>, Option<u32>), String> {
    let found: Option<(i64, String, String, String, String)> = conn
        .query_row(
            "SELECT l.id, l.name, o.name, l.rel_path, a.selector FROM appearance a
             JOIN location l ON l.file_hash = a.resource_hash JOIN origin o ON o.id = l.origin_id
             WHERE a.version_id = ?1 AND l.kind = 'file' ORDER BY a.id, l.id LIMIT 1",
            [version],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((id, name, origin, rel, selector)) = found else {
        return Ok((None, String::new(), String::new(), None, None));
    };
    let pages = selector
        .strip_prefix("pages:")
        .and_then(|r| r.split_once('-'))
        .and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?)));
    Ok((Some(id), name, format!("{origin}/{rel}"), pages.map(|p| p.0), pages.map(|p| p.1)))
}

/// The doubtful types and the suggestions now. Reads only.
pub(crate) fn types_by_similarity(conn: &Connection) -> Result<TypesBySimilarity, String> {
    let catalog = super::types::catalog();
    let decisions = super::classify::read_type_decisions(conn)?;
    let mut out = TypesBySimilarity::default();
    let doubts: Vec<(i64, String, String, String)> = {
        let mut statement = conn
            .prepare(
                "SELECT d.version_id, d.doc_type, d.neighbour_type, d.neighbours FROM type_doubt d
                 JOIN document_type t ON t.version_id = d.version_id AND t.doc_type = d.doc_type ORDER BY d.version_id",
            )
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    for (version, doc_type, neighbour_type, neighbours) in doubts {
        let near: Vec<(i64, f64)> = serde_json::from_str(&neighbours).unwrap_or_default();
        let (location_id, name, path, first_page, last_page) = place(conn, version)?;
        out.doubtful.push(DoubtfulType {
            version_id: version,
            location_id,
            name,
            path,
            first_page,
            last_page,
            doc_type,
            neighbour_type,
            likeness: near.into_iter().map(|(_, s)| s).collect(),
        });
    }
    let Some(signals) = load(conn, catalog, &decisions)? else {
        return Ok(out);
    };
    // Without a type, and the step of the type leaves them so: their
    // question held by a failure, or no word to ask with.
    let held = crate::docbase::failures::held_versions(conn, "types")?;
    let without: Vec<(i64, i64)> = {
        let mut statement = conn
            .prepare(&format!(
                "SELECT c.id, coalesce(c.word_count, 0) FROM version c WHERE c.state = 'processed' AND {}
                   AND NOT EXISTS (SELECT 1 FROM document_type t WHERE t.version_id = c.id) ORDER BY c.id",
                crate::docbase::cut::parts::is_document_sql("c.id")
            ))
            .map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    for (version, words) in without {
        let why = if words < crate::docbase::safe_words::ASKED_WORDS {
            "no tiene palabras para preguntar su tipo"
        } else if held.contains(&version) {
            "la pregunta de su tipo falló"
        } else {
            continue;
        };
        let Some((doc_type, likeness, neighbours)) = signals.suggestion(version) else { continue };
        let (location_id, name, path, first_page, last_page) = place(conn, version)?;
        out.suggested.push(SuggestedType {
            version_id: version,
            location_id,
            name,
            path,
            first_page,
            last_page,
            doc_type,
            likeness,
            neighbours,
            why: why.to_string(),
        });
    }
    Ok(out)
}

impl super::super::DocBase {
    /// Whether the questions of the type carry the hint of the neighbours.
    pub fn type_hint(&self) -> Result<bool, String> {
        hint_on(&self.conn())
    }

    /// Turns the hint on or off; the next question uses it. Nothing typed is
    /// asked again (the form of a question is not part of its basis).
    pub fn set_type_hint(&self, on: bool) -> Result<(), String> {
        set_hint_on(&self.conn(), on)
    }

    /// The doubtful types and the suggestions for the documents without a
    /// type, for Atención. Reads only.
    pub fn types_by_similarity(&self) -> Result<TypesBySimilarity, String> {
        types_by_similarity(&self.conn())
    }
}
