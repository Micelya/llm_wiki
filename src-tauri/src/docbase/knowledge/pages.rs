//! Stage 3 of the knowledge layer: the pages of the archive's wiki that
//! are a projection of the record (D-175).
//!
//! - **The page of each entity** that has one (`wiki/entities/`): its
//!   header (type, name, aliases, id, every document that names it), what
//!   the documents say of it, the list of its documents grouped by matter
//!   with its role in each, the entities related to it and a section of
//!   notes the app never writes over. Which entities have a page is
//!   decided over the whole archive, by the rules of the vocabulary
//!   (`pages.kinds`, D-196): never by one document.
//! - **What the documents say** is written by the model from the verified
//!   claims of the entity alone, numbered; every sentence names the
//!   claims that back it, and a computation throws away a sentence that
//!   names none, names one that does not exist, or carries a number that
//!   none of its claims carries (D-197). Claims in conflict are listed as
//!   such, all values in view. The answer is kept like any other
//!   (`model_answer`): the same claims ask nothing again.
//! - **The block of entities of each document's page** (the pages the
//!   wiki writes from each document): who and what the document names,
//!   with its role and a link to its page. Written by the app, without the
//!   model, between two marks; a page the wiki writes again gets its
//!   block back (D-198).
//! - **The overview** of the archive (`wiki/overview.md`), from the
//!   record, in the words of the vocabulary.
//!
//! A page's file name is fixed the first time (a readable name plus the
//! entity's id) and kept when the entity is renamed: links never break
//! (D-195). Everything is computed again from the record each time the
//! record or the wiki's pages of documents change, and only what changed
//! is written (R-13, R-15): the order in which documents were read,
//! resolved or written into the wiki does not matter.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::quote;
use super::vocabulary::{vocabulary, Vocabulary};
use crate::docbase::similar::classify::{self as classify, ask_and_keep, cached_answer, parse_json, stops_the_run, Question, ASKING};
use crate::docbase::{now_ms, sql_err, DocBase, Visible};
use crate::source_volume::asking::DocumentAsker;

/// Version of the question that writes what the documents say.
pub const SYNTHESIS_PROMPT: &str = "sintesis-1";
/// The task its answers are kept under in `model_answer`.
pub const TASK: &str = "synthesis";
/// The words of the vocabulary (`pages.words`) a page is written with.
#[cfg_attr(not(test), allow(dead_code))]
pub const WORDS: &[&str] = &[
    "says",
    "saysNothing",
    "saysPending",
    "conflicts",
    "references",
    "documents",
    "withoutMatter",
    "related",
    "inCommon",
    "notes",
    "notesMark",
    "aliases",
    "page",
    "documentWord",
    "documentsWord",
    "kept",
    "keptMerged",
    "sourceBlock",
    "sourceBlockMark",
    "overviewTitle",
    "overviewIntro",
    "overviewDocuments",
    "overviewEntities",
    "overviewMost",
    "overviewMatters",
];
/// Claims of one entity described in its question; the rest are left
/// out of the synthesis (the list of documents is always complete).
pub const MAX_CLAIMS: usize = 120;
/// Quotes shown for one claim.
const QUOTES_PER_CLAIM: usize = 2;
const MAX_RELATED: usize = 12;
const MOST_NAMED: usize = 10;
/// Where the language of the wiki is kept (`docbase_setting`).
const WIKI_LANGUAGE_SETTING: &str = "wiki-language";
/// What the last complete projection was computed on.
const BASIS_SETTING: &str = "knowledge-pages-basis";
const OVERVIEW_SETTING: &str = "knowledge-overview";
const OVERVIEW_PATH: &str = "wiki/overview.md";
/// The marks around what the app writes inside a page it does not own,
/// and before the notes of a person. They are found by these words alone.
pub const BLOCK_START: &str = "<!-- micelya:entidades:inicio";
pub const BLOCK_END: &str = "<!-- micelya:entidades:fin -->";
pub const NOTES_MARK: &str = "<!-- micelya:notas";
/// Marks a page written by the app from the record.
pub const GENERATED_BY: &str = "micelya-conocimiento";

/// One projection at a time.
static PROJECTING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// What a projection did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReport {
    /// Nothing to do: no general project, the resolution is not done, or
    /// nothing changed since the last projection.
    pub skipped: bool,
    pub reason: Option<String>,
    /// Entities with a page.
    pub pages: usize,
    pub written: usize,
    pub unchanged: usize,
    /// Pages removed (their entity has no page any more and they held no
    /// notes), and pages kept only for their notes.
    pub removed: usize,
    pub kept: usize,
    /// Questions sent to the model, failed, and left for another run.
    pub asked: usize,
    pub failed: usize,
    pub left: usize,
    /// Sentences of the syntheses in the pages, and thrown away by the check.
    pub sentences: usize,
    pub discarded: usize,
    /// Pages of documents, and how many got their block written.
    pub source_pages: usize,
    pub blocks_written: usize,
    pub overview_written: bool,
    pub errors: Vec<String>,
}

impl ProjectReport {
    /// Whether a file of the wiki changed.
    pub fn wrote(&self) -> bool {
        self.written + self.removed + self.blocks_written > 0 || self.overview_written
    }
}

#[derive(Debug, Clone)]
struct Claim {
    id: i64,
    predicate: String,
    object_entity: Option<String>,
    object_value: Option<String>,
    state: String,
    valid_from: Option<String>,
    /// (content, quote, page)
    evidence: Vec<(i64, String, Option<i64>)>,
}

#[derive(Debug, Clone, Default)]
struct Entity {
    id: String,
    kind: String,
    name: String,
    state: String,
    merged_into: Option<String>,
    /// Ways it is written, with how many mentions.
    written: BTreeMap<String, usize>,
    /// Its documents, with its roles in each.
    documents: BTreeMap<i64, BTreeSet<String>>,
    /// (kind, normalized) -> (value as shown, documents)
    identifiers: BTreeMap<(String, String), (String, BTreeSet<i64>)>,
    claims: Vec<Claim>,
}

#[derive(Debug, Clone, Default)]
struct Document {
    name: String,
    /// What the wiki calls it (`<origin>/<path inside it>`).
    identity: Option<String>,
    /// The page the wiki wrote from it.
    page: Option<String>,
    doc_type: Option<String>,
}

#[derive(Debug, Clone)]
struct PageRow {
    path: String,
    state: String,
    fingerprint: String,
}

/// The record, as the pages need it.
struct Record {
    entities: BTreeMap<String, Entity>,
    documents: BTreeMap<i64, Document>,
    pages: BTreeMap<String, PageRow>,
    /// Source pages: (path, content).
    source_pages: Vec<(String, i64)>,
}

/// A sentence of the synthesis, as the model answers it.
#[derive(Debug, Deserialize)]
struct Answer {
    #[serde(default)]
    oraciones: Vec<Sentence>,
}

#[derive(Debug, Deserialize)]
struct Sentence {
    texto: String,
    #[serde(default)]
    afirmaciones: Vec<i64>,
}

fn hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hasher.finalize().iter().take(12).map(|b| format!("{b:02x}")).collect()
}

/// The readable part of a page's file name: the words of the name, in
/// lower case without accents, joined by hyphens.
pub fn slug(name: &str, fallback: &str) -> String {
    let mut out = String::new();
    for word in quote::words(name) {
        let clean: String = word.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if clean.is_empty() {
            continue;
        }
        if !out.is_empty() && out.len() + clean.len() + 1 > 60 {
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(&clean);
    }
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

/// The path of a new page: the folder, the readable name and the short
/// form of the entity's id, which no other entity shares.
pub fn page_path(folder: &str, id: &str, name: &str, kind: &str) -> String {
    let short: String = id.trim_start_matches("ent-").chars().take(8).collect();
    format!("{folder}/{}-{short}.md", slug(name, kind))
}

/// What a wiki link names a page by: its file name without `.md`.
fn stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

fn link(path: &str, text: &str) -> String {
    let text = text.replace(['[', ']', '|'], " ");
    format!("[[{}|{}]]", stem(path), text.trim())
}

fn yaml_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// The digits of a text, by runs: "$ 100.000" has "100" and "000".
fn digit_runs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            run.push(c);
        } else if !run.is_empty() {
            out.insert(std::mem::take(&mut run));
        }
    }
    out
}

/// Takes out references written by the model inside a sentence ("[2]").
fn without_markers(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let after = &rest[start + 1..];
        match after.find(']') {
            Some(end) if !after[..end].is_empty() && after[..end].chars().all(|c| c.is_ascii_digit() || c == ',' || c == ' ') => {
                out.push_str(rest[..start].trim_end());
                rest = &after[end + 1..];
            }
            _ => {
                out.push_str(&rest[..start + 1]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Entity {
    /// The most used way of writing it, after its name.
    fn aliases(&self) -> Vec<String> {
        let mut list: Vec<(&String, &usize)> = self.written.iter().collect();
        list.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let own = quote::words(&self.name).join(" ");
        let mut seen = BTreeSet::from([own]);
        list.into_iter()
            .filter(|(w, _)| seen.insert(quote::words(w).join(" ")))
            .map(|(w, _)| w.clone())
            .collect()
    }

    fn has_strong_identifier(&self, vocabulary: &Vocabulary) -> bool {
        self.identifiers.keys().any(|(kind, _)| vocabulary.is_strong(kind))
    }
}

/// Whether an entity has a page, by the rule of its type (D-196): it
/// appears in enough documents, has a role of the list in one of them, or
/// (if the rule says so) a document gives it a strong identifier.
fn has_page(vocabulary: &Vocabulary, entity: &Entity) -> bool {
    if entity.state != "active" || entity.documents.is_empty() {
        return false;
    }
    let Some(rule) = vocabulary.pages.rule(&entity.kind) else { return false };
    if rule.min_documents > 0 && entity.documents.len() >= rule.min_documents {
        return true;
    }
    if entity.documents.values().flatten().any(|role| rule.roles.iter().any(|r| r == role)) {
        return true;
    }
    rule.strong_identifier && entity.has_strong_identifier(vocabulary)
}

fn load(conn: &Connection, project_id: &str, identities: &HashMap<i64, String>) -> Result<Record, String> {
    let mut entities: BTreeMap<String, Entity> = BTreeMap::new();
    {
        let mut statement = conn.prepare("SELECT id, kind, name, state, merged_into FROM knowledge_entity").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    name: row.get(2)?,
                    state: row.get(3)?,
                    merged_into: row.get(4)?,
                    ..Default::default()
                })
            })
            .map_err(sql_err)?;
        for row in rows {
            let entity = row.map_err(sql_err)?;
            entities.insert(entity.id.clone(), entity);
        }
    }
    // Only documents that still have a file somewhere count.
    let (visible, params) = super::visible_content("m.content_id", &Visible::All);
    {
        let mut statement = conn
            .prepare(&format!(
                "SELECT m.entity_id, m.content_id, m.role, m.written FROM knowledge_mention m
                 WHERE m.entity_id IS NOT NULL AND {visible}"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (entity, content, role, written) = row.map_err(sql_err)?;
            if let Some(entity) = entities.get_mut(&entity) {
                entity.documents.entry(content).or_default().insert(role);
                *entity.written.entry(written).or_default() += 1;
            }
        }
    }
    {
        let mut statement = conn
            .prepare(&format!(
                "SELECT m.entity_id, i.kind, i.value, i.normalized, m.content_id FROM knowledge_identifier i
                 JOIN knowledge_mention m ON m.id = i.mention_id WHERE m.entity_id IS NOT NULL AND {visible}"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (entity, kind, value, normalized, content) = row.map_err(sql_err)?;
            if let Some(entity) = entities.get_mut(&entity) {
                entity.identifiers.entry((kind, normalized)).or_insert_with(|| (value, BTreeSet::new())).1.insert(content);
            }
        }
    }
    {
        let mut evidence: HashMap<i64, Vec<(i64, String, Option<i64>)>> = HashMap::new();
        let (visible_e, params_e) = super::visible_content("e.content_id", &Visible::All);
        let mut statement = conn
            .prepare(&format!(
                "SELECT e.claim_id, e.content_id, e.quote, e.page FROM knowledge_evidence e
                 WHERE e.claim_id IS NOT NULL AND {visible_e} ORDER BY e.content_id, e.id"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params_e.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<i64>>(3)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (claim, content, quote, page) = row.map_err(sql_err)?;
            evidence.entry(claim).or_default().push((content, quote, page));
        }
        let mut statement = conn
            .prepare(
                "SELECT id, subject, predicate, object_entity, object_value, state, valid_from FROM knowledge_claim
                 WHERE state != 'retired' ORDER BY subject, predicate, coalesce(valid_from, ''), coalesce(object_normalized, object_entity, ''), id",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    Claim {
                        id: row.get(0)?,
                        predicate: row.get(2)?,
                        object_entity: row.get(3)?,
                        object_value: row.get(4)?,
                        state: row.get(5)?,
                        valid_from: row.get(6)?,
                        evidence: Vec::new(),
                    },
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (subject, mut claim) = row.map_err(sql_err)?;
            claim.evidence = evidence.remove(&claim.id).unwrap_or_default();
            if claim.evidence.is_empty() {
                continue;
            }
            if let Some(entity) = entities.get_mut(&subject) {
                entity.claims.push(claim);
            }
        }
    }
    let mut documents: BTreeMap<i64, Document> = BTreeMap::new();
    let contents: BTreeSet<i64> = entities.values().flat_map(|e| e.documents.keys().copied()).collect();
    for content in &contents {
        documents.insert(
            *content,
            Document { name: classify::name_of(conn, *content)?, identity: identities.get(content).cloned(), ..Default::default() },
        );
    }
    {
        let mut statement = conn.prepare("SELECT content_id, doc_type FROM document_type").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (content, doc_type) = row.map_err(sql_err)?;
            if let Some(document) = documents.get_mut(&content) {
                document.doc_type = Some(doc_type);
            }
        }
    }
    let mut source_pages = Vec::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT DISTINCT p.path, f.content_id FROM wiki_page p
                 JOIN wiki_page_source s ON s.page_id = p.id JOIN location l ON l.id = s.location_id
                 JOIN file f ON f.hash = l.file_hash
                 WHERE p.project_id = ?1 AND p.type = 'source' AND f.content_id IS NOT NULL ORDER BY p.path, f.content_id",
            )
            .map_err(sql_err)?;
        let rows = statement.query_map([project_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (path, content) = row.map_err(sql_err)?;
            if let Some(document) = documents.get_mut(&content) {
                if document.page.is_none() {
                    document.page = Some(path.clone());
                }
            }
            source_pages.push((path, content));
        }
    }
    let mut pages = BTreeMap::new();
    {
        let mut statement = conn.prepare("SELECT entity_id, path, state, fingerprint FROM knowledge_page").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, PageRow { path: row.get(1)?, state: row.get(2)?, fingerprint: row.get(3)? }))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (id, page) = row.map_err(sql_err)?;
            pages.insert(id, page);
        }
    }
    Ok(Record { entities, documents, pages, source_pages })
}

/// The language the wiki is written in, as the wiki said it; none when it
/// did not say.
pub(crate) fn wiki_language(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [WIKI_LANGUAGE_SETTING], |row| row.get::<_, String>(0))
        .optional()
        .map_err(sql_err)
        .map(|v| v.filter(|l| !l.trim().is_empty() && l != "auto"))
}

fn setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [key], |row| row.get(0)).optional().map_err(sql_err)
}

fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![key, value]).map_err(sql_err)?;
    Ok(())
}

/// The claims of an entity in the order they are numbered, at most
/// `MAX_CLAIMS`.
fn numbered(vocabulary: &Vocabulary, entity: &Entity) -> Vec<Claim> {
    let order = |predicate: &str| vocabulary.predicates.iter().position(|p| p.id == predicate).unwrap_or(usize::MAX);
    let mut claims = entity.claims.clone();
    claims.sort_by(|a, b| {
        order(&a.predicate)
            .cmp(&order(&b.predicate))
            .then_with(|| a.valid_from.cmp(&b.valid_from))
            .then_with(|| a.object_value.cmp(&b.object_value))
            .then_with(|| a.object_entity.cmp(&b.object_entity))
    });
    claims.truncate(MAX_CLAIMS);
    claims
}

fn object_text(record: &Record, claim: &Claim) -> String {
    match (&claim.object_value, &claim.object_entity) {
        (Some(value), _) => value.clone(),
        (None, Some(id)) => record.entities.get(id).map(|e| e.name.clone()).unwrap_or_else(|| id.clone()),
        (None, None) => String::new(),
    }
}

/// The question that writes what the documents say of an entity.
fn synthesis_question(vocabulary: &Vocabulary, record: &Record, entity: &Entity, claims: &[Claim], language: Option<&str>) -> Question {
    let kind = vocabulary.type_label(&entity.kind);
    let mut listed = Vec::new();
    for (index, claim) in claims.iter().enumerate() {
        let mut line = format!("[{}] {}: {}", index + 1, vocabulary.predicate_label(&claim.predicate), object_text(record, claim));
        if let Some(from) = &claim.valid_from {
            line.push_str(&format!(" (tiempo: {from})"));
        }
        if claim.state == "conflict" {
            line.push_str(" — EN CONFLICTO con otra afirmación del mismo dato");
        }
        let documents: BTreeSet<i64> = claim.evidence.iter().map(|(c, ..)| *c).collect();
        line.push_str(&format!(" — en {} documento(s)", documents.len()));
        for (content, quote, _) in claim.evidence.iter().take(QUOTES_PER_CLAIM) {
            let name = record.documents.get(content).map(|d| d.name.as_str()).unwrap_or_default();
            line.push_str(&format!("\n    Cita: «{}» ({name})", quote.trim()));
        }
        listed.push(line);
    }
    let language = match language {
        Some(language) => format!("Escribí en este idioma: {language}."),
        None => "Escribí en el idioma en que están escritas las citas.".to_string(),
    };
    let prompt = format!(
        "Sos asistente de un estudio jurídico. Redactá lo que dicen los documentos del archivo sobre {kind} «{name}», \
para su ficha en el wiki del estudio. Usá solo las afirmaciones numeradas de abajo: cada una ya fue comprobada contra el \
texto de su documento.\n\n\
Reglas:\n\
- No agregues nada que no esté en las afirmaciones: ni conocimiento general, ni suposiciones, ni conclusiones que combinen \
dos afirmaciones (si una persona representa a una sociedad y la sociedad firmó un contrato, no digas que la persona lo firmó).\n\
- Oraciones breves y concretas, como las escribiría un abogado: partes, roles, cargos, domicilios, importes, fechas, \
obligaciones y plazos. Agrupá lo que se relaciona; no repitas.\n\
- Cada oración lleva los números de las afirmaciones que la respaldan (al menos uno). No escribas los números dentro del texto.\n\
- Copiá importes, fechas, números y nombres tal como están en las afirmaciones.\n\
- Si una afirmación dice un tiempo, decilo. Si las afirmaciones de un mismo dato dicen tiempos distintos, contalo como una historia.\n\
- Las afirmaciones EN CONFLICTO no se resuelven: decí que los documentos difieren y nombrá cada valor con su número.\n\
- {language}\n\n\
{kind}: {name}\n\
Afirmaciones:\n{claims}\n\n\
Respondé solo con el JSON pedido: {{\"oraciones\": [{{\"texto\": \"...\", \"afirmaciones\": [1, 2]}}]}}.",
        name = entity.name,
        claims = listed.join("\n"),
    );
    Question {
        key: classify::question_key(SYNTHESIS_PROMPT, &prompt),
        version: SYNTHESIS_PROMPT,
        task: TASK,
        content_id: 0,
        prompt,
        schema: schema(),
    }
}

pub fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["oraciones"],
        "properties": {
            "oraciones": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["texto", "afirmaciones"],
                    "properties": {
                        "texto": { "type": "string" },
                        "afirmaciones": { "type": "array", "items": { "type": "integer" } }
                    }
                }
            }
        }
    })
}

/// The sentences of an answer the check keeps, each with the claims
/// (indexes into `claims`) that back it; and how many it threw away.
fn checked(record: &Record, entity: &Entity, claims: &[Claim], answer: &str) -> (Vec<(String, Vec<usize>)>, usize) {
    let Ok(answer) = parse_json::<Answer>(answer) else { return (Vec::new(), 1) };
    let mut kept = Vec::new();
    let mut discarded = 0;
    for sentence in answer.oraciones {
        let text = without_markers(&sentence.texto);
        let mut refs: Vec<usize> = Vec::new();
        let mut bad = text.is_empty() || sentence.afirmaciones.is_empty();
        for n in &sentence.afirmaciones {
            match usize::try_from(*n).ok().filter(|n| *n >= 1 && *n <= claims.len()) {
                Some(n) if !refs.contains(&(n - 1)) => refs.push(n - 1),
                Some(_) => {}
                None => bad = true,
            }
        }
        if !bad {
            // Every number of the sentence is in one of its claims.
            let mut backed = digit_runs(&entity.name);
            for &r in &refs {
                let claim = &claims[r];
                backed.extend(digit_runs(&object_text(record, claim)));
                backed.extend(digit_runs(claim.valid_from.as_deref().unwrap_or_default()));
                for (_, quote, _) in &claim.evidence {
                    backed.extend(digit_runs(quote));
                }
            }
            bad = digit_runs(&text).iter().any(|run| !backed.contains(run));
        }
        if bad {
            discarded += 1;
        } else {
            refs.sort_unstable();
            kept.push((text, refs));
        }
    }
    (kept, discarded)
}

/// What the page of an entity says, but its notes; and what the
/// synthesis left out.
struct Rendered {
    text: String,
    synthesis: Option<String>,
    sentences: usize,
    discarded: usize,
}

fn document_link(record: &Record, content: i64) -> String {
    match record.documents.get(&content) {
        Some(Document { page: Some(page), name, .. }) => link(page, name),
        Some(document) => document.name.clone(),
        None => format!("#{content}"),
    }
}

/// The entities related to one: the ones with a page that share documents
/// or claims with it, the closest first.
fn related<'a>(record: &'a Record, paths: &BTreeMap<String, String>, entity: &Entity) -> Vec<(&'a Entity, usize)> {
    let mut score: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for other in record.entities.values() {
        if other.id == entity.id || !paths.contains_key(&other.id) {
            continue;
        }
        let shared = entity.documents.keys().filter(|c| other.documents.contains_key(c)).count();
        let claimed = entity.claims.iter().filter(|c| c.object_entity.as_deref() == Some(other.id.as_str())).count()
            + other.claims.iter().filter(|c| c.object_entity.as_deref() == Some(entity.id.as_str())).count();
        if shared + claimed > 0 {
            score.insert(other.id.as_str(), (claimed, shared));
        }
    }
    let mut list: Vec<(&Entity, (usize, usize))> =
        score.into_iter().filter_map(|(id, s)| record.entities.get(id).map(|e| (e, s))).collect();
    list.sort_by(|a, b| (b.1 .0 * 2 + b.1 .1).cmp(&(a.1 .0 * 2 + a.1 .1)).then_with(|| a.0.name.cmp(&b.0.name)));
    list.truncate(MAX_RELATED);
    list.into_iter().map(|(e, (_, shared))| (e, shared)).collect()
}

fn documents_word<'a>(vocabulary: &'a Vocabulary, n: usize) -> &'a str {
    vocabulary.pages.word(if n == 1 { "documentWord" } else { "documentsWord" })
}

fn render(vocabulary: &Vocabulary, record: &Record, paths: &BTreeMap<String, String>, entity: &Entity, answer: Option<(&str, &str)>, claims: &[Claim]) -> Rendered {
    let words = &vocabulary.pages;
    let mut out = String::new();
    let aliases = entity.aliases();
    let mut sources: Vec<String> = entity.documents.keys().filter_map(|c| record.documents.get(c)?.identity.clone()).collect();
    sources.sort();
    sources.dedup();
    let related = related(record, paths, entity);
    out.push_str("---\ntype: entity\n");
    out.push_str(&format!("title: {}\n", yaml_string(&entity.name)));
    out.push_str(&format!("entity_kind: {}\n", entity.kind));
    out.push_str(&format!("entity_id: {}\n", entity.id));
    out.push_str(&format!("aliases: [{}]\n", aliases.iter().map(|a| yaml_string(a)).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("sources: [{}]\n", sources.iter().map(|s| yaml_string(s)).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!(
        "related: [{}]\n",
        related.iter().filter_map(|(e, _)| paths.get(&e.id)).map(|p| stem(p)).collect::<Vec<_>>().join(", ")
    ));
    out.push_str(&format!("generated_by: {GENERATED_BY}\n"));
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n", entity.name));
    let mut facts = vec![format!("*{}*", vocabulary.type_label(&entity.kind))];
    if !aliases.is_empty() {
        facts.push(format!("{}: {}", words.word("aliases"), aliases.join(" · ")));
    }
    for ((kind, _), (value, documents)) in &entity.identifiers {
        facts.push(format!("{} {value} ({} {})", vocabulary.identifier_label(kind), documents.len(), documents_word(vocabulary, documents.len())));
    }
    out.push_str(&facts.join(" · "));
    out.push_str("\n\n");

    // What the documents say.
    let mut synthesis = None;
    let mut sentences = 0;
    let mut discarded = 0;
    out.push_str(&format!("## {}\n\n", words.word("says")));
    let mut used: BTreeSet<usize> = BTreeSet::new();
    if claims.is_empty() {
        out.push_str(words.word("saysNothing"));
        out.push_str("\n\n");
    } else {
        match answer {
            Some((key, text)) => {
                synthesis = Some(key.to_string());
                let (kept, thrown) = checked(record, entity, claims, text);
                discarded = thrown;
                sentences = kept.len();
                let lines: Vec<String> = kept
                    .iter()
                    .map(|(text, refs)| {
                        used.extend(refs.iter().copied());
                        let numbers: Vec<String> = refs.iter().map(|r| (r + 1).to_string()).collect();
                        format!("{text} [{}]", numbers.join(", "))
                    })
                    .collect();
                if lines.is_empty() {
                    out.push_str(words.word("saysPending"));
                } else {
                    out.push_str(&lines.join(" "));
                }
                out.push_str("\n\n");
            }
            None => {
                out.push_str(words.word("saysPending"));
                out.push_str("\n\n");
            }
        }
        // Conflicts are always in view, all their values.
        let mut conflicts: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (index, claim) in claims.iter().enumerate() {
            if claim.state == "conflict" {
                conflicts.entry(claim.predicate.as_str()).or_default().push(index);
            }
        }
        if !conflicts.is_empty() {
            out.push_str(&format!("**{}**\n\n", words.word("conflicts")));
            for (predicate, indexes) in &conflicts {
                let values: Vec<String> =
                    indexes.iter().map(|i| format!("«{}» [{}]", object_text(record, &claims[*i]), i + 1)).collect();
                used.extend(indexes.iter().copied());
                out.push_str(&format!("- {}: {}\n", vocabulary.predicate_label(predicate), values.join(" · ")));
            }
            out.push('\n');
        }
        if !used.is_empty() {
            out.push_str(&format!("### {}\n\n", words.word("references")));
            for index in &used {
                let claim = &claims[*index];
                let quotes: Vec<String> = claim
                    .evidence
                    .iter()
                    .map(|(content, quote, page)| {
                        let at = page.map(|p| format!(", {} {p}", words.word("page"))).unwrap_or_default();
                        format!("«{}» ({}{at})", quote.trim(), document_link(record, *content))
                    })
                    .collect();
                out.push_str(&format!(
                    "- [{}] {}: {} — {}\n",
                    index + 1,
                    vocabulary.predicate_label(&claim.predicate),
                    object_text(record, claim),
                    quotes.join("; ")
                ));
            }
            out.push('\n');
        }
    }

    // Its documents, by matter.
    out.push_str(&format!("## {} ({})\n\n", words.word("documents"), entity.documents.len()));
    let line = |content: i64, roles: &BTreeSet<String>| {
        let roles: Vec<&str> = roles.iter().map(|r| vocabulary.role_label(r)).collect();
        let kind = record
            .documents
            .get(&content)
            .and_then(|d| d.doc_type.as_deref())
            .map(|t| format!(" · {}", t.replace('-', " ")))
            .unwrap_or_default();
        format!("- {} — {}{kind}\n", document_link(record, content), roles.join(", "))
    };
    if entity.kind == "asunto" {
        for (content, roles) in &entity.documents {
            out.push_str(&line(*content, roles));
        }
        out.push('\n');
    } else {
        let mut by_matter: BTreeMap<String, (String, Vec<i64>)> = BTreeMap::new();
        let mut alone = Vec::new();
        for content in entity.documents.keys() {
            let matters: Vec<&Entity> = record
                .entities
                .values()
                .filter(|m| m.kind == "asunto" && paths.contains_key(&m.id) && m.documents.contains_key(content))
                .collect();
            if matters.is_empty() {
                alone.push(*content);
            }
            for matter in matters {
                by_matter
                    .entry(matter.id.clone())
                    .or_insert_with(|| (link(&paths[&matter.id], &matter.name), Vec::new()))
                    .1
                    .push(*content);
            }
        }
        let mut groups: Vec<(String, Vec<i64>)> = by_matter.into_values().collect();
        groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
        let several = !groups.is_empty();
        for (title, contents) in groups {
            out.push_str(&format!("### {title}\n\n"));
            for content in contents {
                out.push_str(&line(content, &entity.documents[&content]));
            }
            out.push('\n');
        }
        if !alone.is_empty() {
            if several {
                out.push_str(&format!("### {}\n\n", words.word("withoutMatter")));
            }
            for content in alone {
                out.push_str(&line(content, &entity.documents[&content]));
            }
            out.push('\n');
        }
    }

    if !related.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("related")));
        for (other, shared) in &related {
            let together =
                if *shared > 0 { format!(" — {shared} {}", words.word("inCommon")) } else { String::new() };
            out.push_str(&format!("- {} · {}{together}\n", link(&paths[&other.id], &other.name), vocabulary.type_label(&other.kind)));
        }
        out.push('\n');
    }
    Rendered { text: out, synthesis, sentences, discarded }
}

/// The notes of a person in a page on disk: what follows the notes mark,
/// or the section of notes when the mark was taken out.
fn notes_of(text: &str, heading: &str) -> String {
    let text = text.replace("\r\n", "\n");
    if let Some(at) = text.find(NOTES_MARK) {
        let after = &text[at..];
        let line_end = after.find('\n').map(|n| n + 1).unwrap_or(after.len());
        return after[line_end..].trim_matches('\n').to_string();
    }
    let wanted = format!("## {heading}");
    if let Some(at) = text.lines().position(|l| l.trim() == wanted) {
        return text.lines().skip(at + 1).collect::<Vec<_>>().join("\n").trim_matches('\n').to_string();
    }
    String::new()
}

fn with_notes(vocabulary: &Vocabulary, generated: &str, notes: &str) -> String {
    let words = &vocabulary.pages;
    let mut out = generated.trim_end().to_string();
    out.push_str(&format!("\n\n## {}\n\n{NOTES_MARK}: {} -->\n", words.word("notes"), words.word("notesMark")));
    if !notes.trim().is_empty() {
        out.push_str(notes.trim_matches('\n'));
        out.push('\n');
    }
    out
}

/// Puts the date the page was written under its header.
fn dated(text: &str) -> String {
    text.replacen(&format!("generated_by: {GENERATED_BY}\n"), &format!("generated_by: {GENERATED_BY}\nupdated: {}\n", today()), 1)
}

/// Writes a file whole, through a file next to it.
fn write_file(path: &Path, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("No se pudo crear '{}': {e}", parent.display()))?;
    }
    let temporary = path.with_extension("md.tmp");
    std::fs::write(&temporary, text).map_err(|e| format!("No se pudo escribir '{}': {e}", temporary.display()))?;
    std::fs::rename(&temporary, path).map_err(|e| format!("No se pudo escribir '{}': {e}", path.display()))
}

fn read_file(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// The block of entities a document's page ends with.
fn source_block(vocabulary: &Vocabulary, record: &Record, paths: &BTreeMap<String, String>, content: i64) -> Option<String> {
    let order = |kind: &str| vocabulary.entity_types.iter().position(|t| t.id == kind).unwrap_or(usize::MAX);
    let mut named: Vec<(&Entity, &BTreeSet<String>)> = record
        .entities
        .values()
        .filter(|e| e.state == "active")
        .filter_map(|e| e.documents.get(&content).map(|roles| (e, roles)))
        // Who has no page and was only named in passing is left out.
        .filter(|(e, roles)| paths.contains_key(&e.id) || roles.iter().any(|r| r != vocabulary.fallback_role()))
        .collect();
    if named.is_empty() {
        return None;
    }
    named.sort_by(|a, b| order(&a.0.kind).cmp(&order(&b.0.kind)).then_with(|| a.0.name.cmp(&b.0.name)));
    let words = &vocabulary.pages;
    let mut out = format!("{BLOCK_START}: {} -->\n## {}\n\n", words.word("sourceBlockMark"), words.word("sourceBlock"));
    for (entity, roles) in named {
        let name = match paths.get(&entity.id) {
            Some(path) => link(path, &entity.name),
            None => entity.name.clone(),
        };
        let roles: Vec<&str> = roles.iter().map(|r| vocabulary.role_label(r)).collect();
        out.push_str(&format!("- {name} · {} — {}\n", vocabulary.type_label(&entity.kind), roles.join(", ")));
    }
    out.push_str(BLOCK_END);
    out.push('\n');
    Some(out)
}

/// The page with its block of entities in place of the one it had (or at
/// its end); with none, without any.
pub(crate) fn with_block(text: &str, block: Option<&str>) -> String {
    let mut rest = text.replace("\r\n", "\n");
    // Every block there is goes, so a page merged twice keeps one.
    while let Some(start) = rest.find(BLOCK_START) {
        let end = rest[start..].find(BLOCK_END).map(|e| start + e + BLOCK_END.len()).unwrap_or(rest.len());
        let after = rest[end..].trim_start_matches('\n').to_string();
        rest = format!("{}{}{after}", rest[..start].trim_end_matches('\n'), if after.is_empty() { "" } else { "\n\n" });
    }
    let mut out = rest.trim_end().to_string();
    if let Some(block) = block {
        out.push_str("\n\n");
        out.push_str(block.trim_end());
    }
    out.push('\n');
    out
}

fn file_meta(path: &Path) -> (i64, i64) {
    match std::fs::metadata(path) {
        Ok(meta) => (
            meta.len() as i64,
            meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        ),
        Err(_) => (-1, -1),
    }
}

/// The overview of the archive, from the record.
fn overview(vocabulary: &Vocabulary, record: &Record, paths: &BTreeMap<String, String>, documents: usize) -> String {
    let words = &vocabulary.pages;
    let mut out = format!(
        "---\ntype: overview\ntitle: {}\ngenerated_by: {GENERATED_BY}\n---\n\n# {}\n\n{}\n\n",
        yaml_string(words.word("overviewTitle")),
        words.word("overviewTitle"),
        words.word("overviewIntro")
    );
    out.push_str(&format!("- {}: {documents}\n", words.word("overviewDocuments")));
    let mut by_kind: BTreeMap<usize, (&str, usize)> = BTreeMap::new();
    for entity in record.entities.values().filter(|e| paths.contains_key(&e.id)) {
        let order = vocabulary.entity_types.iter().position(|t| t.id == entity.kind).unwrap_or(usize::MAX);
        by_kind.entry(order).or_insert((vocabulary.type_label(&entity.kind), 0)).1 += 1;
    }
    let counts: Vec<String> = by_kind.values().map(|(label, n)| format!("{label}: {n}")).collect();
    out.push_str(&format!("- {}: {}\n\n", words.word("overviewEntities"), counts.join(" · ")));
    let mut most: Vec<&Entity> =
        record.entities.values().filter(|e| paths.contains_key(&e.id) && e.kind != "asunto").collect();
    most.sort_by(|a, b| b.documents.len().cmp(&a.documents.len()).then_with(|| a.name.cmp(&b.name)));
    if !most.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("overviewMost")));
        for entity in most.into_iter().take(MOST_NAMED) {
            let n = entity.documents.len();
            out.push_str(&format!(
                "- {} · {} — {n} {}\n",
                link(&paths[&entity.id], &entity.name),
                vocabulary.type_label(&entity.kind),
                documents_word(vocabulary, n)
            ));
        }
        out.push('\n');
    }
    let mut matters: Vec<&Entity> = record.entities.values().filter(|e| paths.contains_key(&e.id) && e.kind == "asunto").collect();
    matters.sort_by(|a, b| b.documents.len().cmp(&a.documents.len()).then_with(|| a.name.cmp(&b.name)));
    if !matters.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("overviewMatters")));
        for entity in matters {
            let n = entity.documents.len();
            out.push_str(&format!("- {} — {n} {}\n", link(&paths[&entity.id], &entity.name), documents_word(vocabulary, n)));
        }
        out.push('\n');
    }
    out
}

/// The entity a merged one went into, following the chain.
fn survivor<'a>(record: &'a Record, id: &str) -> Option<&'a Entity> {
    let mut current = record.entities.get(id)?;
    for _ in 0..16 {
        match (&current.state[..], &current.merged_into) {
            ("merged", Some(next)) => current = record.entities.get(next)?,
            ("active", _) => return Some(current),
            _ => return None,
        }
    }
    None
}

/// What a projection is computed on besides the record: the pages of
/// documents of the wiki (path, size, date), the vocabulary, the language
/// and the folder of the wiki.
fn basis_of(base: &DocBase, project_id: &str, vocabulary: &Vocabulary, language: Option<&str>, root: &Path) -> Result<String, String> {
    let conn = base.conn();
    let mut statement = conn
        .prepare("SELECT path, size, mtime_ms FROM wiki_page WHERE project_id = ?1 AND type = 'source' ORDER BY path")
        .map_err(sql_err)?;
    let rows: Vec<String> = statement
        .query_map([project_id], |row| Ok(format!("{}|{}|{}", row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, i64>(2)?)))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    Ok(hash(&[&vocabulary.digest, language.unwrap_or(""), &root.to_string_lossy(), &rows.join("
")]))
}

/// How the projection reaches the model.
struct Asking<'a> {
    asker: Option<&'a dyn DocumentAsker>,
    engine: Option<String>,
    budget: usize,
    collect: Option<Vec<Question>>,
}

impl DocBase {
    /// The language the archive's wiki is written in; none (or "auto")
    /// lets the pages follow the language of the documents.
    pub fn set_wiki_language(&self, language: Option<&str>) -> Result<(), String> {
        let conn = self.conn();
        match language.map(str::trim).filter(|l| !l.is_empty()) {
            Some(language) => set_setting(&conn, WIKI_LANGUAGE_SETTING, language),
            None => conn.execute("DELETE FROM docbase_setting WHERE key = ?1", [WIKI_LANGUAGE_SETTING]).map(|_| ()).map_err(sql_err),
        }
    }

    /// The questions a projection would ask now (for the evaluation, which
    /// warms its cache with them first). Writes nothing.
    #[cfg(test)]
    pub(crate) fn synthesis_questions(&self, engine: &str) -> Result<Vec<Question>, String> {
        let mut report = ProjectReport::default();
        let mut asking = Asking { asker: None, engine: Some(engine.to_string()), budget: 0, collect: Some(Vec::new()) };
        project(self, &mut asking, &mut report, true)?;
        Ok(asking.collect.unwrap_or_default())
    }

    /// Projects the record into the general project's wiki: the page of
    /// each entity that has one, the block of entities of each document's
    /// page and the overview; up to `max_questions` questions to `asker`
    /// for what the documents say (none without one: the pages are
    /// written all the same, with the synthesis pending). Waits for the
    /// resolution of identities to finish, and does nothing when neither
    /// the record nor the wiki's pages of documents changed since the last
    /// complete projection.
    pub fn knowledge_project(&self, asker: Option<&dyn DocumentAsker>, max_questions: usize) -> Result<ProjectReport, String> {
        let _one = PROJECTING.lock().unwrap_or_else(|p| p.into_inner());
        let _asking = asker.map(|_| ASKING.lock().unwrap_or_else(|p| p.into_inner()));
        let engine = match asker {
            Some(asker) => Some(asker.engine_id()),
            None => classify::ask_engine(&self.conn())?.map(|c| crate::source_volume::asking::engine_id(&c)),
        };
        let mut report = ProjectReport::default();
        let mut asking = Asking { asker, engine, budget: max_questions, collect: None };
        project(self, &mut asking, &mut report, false)?;
        Ok(report)
    }
}

fn project(base: &DocBase, asking: &mut Asking, report: &mut ProjectReport, dry: bool) -> Result<(), String> {
    let vocabulary = vocabulary();
    let started = now_ms();
    let Some(general) = base.general_project()? else {
        report.skipped = true;
        report.reason = Some("no-project".into());
        return Ok(());
    };
    let Some(root) = general.path.clone() else {
        report.skipped = true;
        report.reason = Some("no-project".into());
        return Ok(());
    };
    let root = PathBuf::from(root);
    let to_resolve: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')", [], |row| row.get(0))
        .map_err(sql_err)?;
    if to_resolve > 0 {
        report.skipped = true;
        report.reason = Some("resolving".into());
        return Ok(());
    }
    base.index_wiki()?;
    let identities: HashMap<i64, String> = {
        let mut out = HashMap::new();
        for source in base.wiki_sources()? {
            if let Some(content) = source.content_id {
                out.entry(content).or_insert(source.identity);
            }
        }
        out
    };
    let (record, language, basis_before, dirty) = {
        let conn = base.conn();
        let record = load(&conn, &general.id, &identities)?;
        let dirty: i64 =
            conn.query_row("SELECT count(*) FROM knowledge_dirty WHERE scope = 'project'", [], |row| row.get(0)).map_err(sql_err)?;
        (record, wiki_language(&conn)?, setting(&conn, BASIS_SETTING)?, dirty)
    };
    // What the pages are computed on, besides the record: the pages of
    // documents the wiki wrote, the vocabulary and the language.
    let basis = basis_of(base, &general.id, vocabulary, language.as_deref(), &root)?;
    if !dry && dirty == 0 && basis_before.as_deref() == Some(basis.as_str()) {
        report.skipped = true;
        report.reason = Some("unchanged".into());
        return Ok(());
    }

    // Which entities have a page, and where.
    let folder = vocabulary.pages.folder().to_string();
    let mut paths: BTreeMap<String, String> = BTreeMap::new();
    let mut taken: BTreeSet<String> = record.pages.values().map(|p| p.path.to_lowercase()).collect();
    for entity in record.entities.values() {
        if !has_page(vocabulary, entity) {
            continue;
        }
        let path = match record.pages.get(&entity.id) {
            Some(page) => page.path.clone(),
            None => {
                let mut path = page_path(&folder, &entity.id, &entity.name, &entity.kind);
                if taken.contains(&path.to_lowercase()) {
                    path = format!("{folder}/{}-{}.md", slug(&entity.name, &entity.kind), entity.id.trim_start_matches("ent-"));
                }
                taken.insert(path.to_lowercase());
                path
            }
        };
        paths.insert(entity.id.clone(), path);
    }
    report.pages = paths.len();

    // The page of each entity.
    for (id, path) in &paths {
        let entity = &record.entities[id];
        let claims = numbered(vocabulary, entity);
        let mut answer: Option<(String, String)> = None;
        if !claims.is_empty() {
            let question = synthesis_question(vocabulary, &record, entity, &claims, language.as_deref());
            let cached = cached_answer(&base.conn(), &question.key, asking.engine.as_deref())?;
            match cached {
                Some(text) => answer = Some((question.key.clone(), text)),
                None => {
                    if let Some(collect) = &mut asking.collect {
                        collect.push(question);
                    } else if let (Some(asker), true) = (asking.asker, asking.budget > 0) {
                        asking.budget -= 1;
                        report.asked += 1;
                        match ask_and_keep(base, asker, &question) {
                            Ok(text) => answer = Some((question.key.clone(), text)),
                            Err(error) => {
                                report.failed += 1;
                                report.errors.push(format!("{}: {error}", entity.name));
                                if stops_the_run(&error) {
                                    asking.budget = 0;
                                }
                            }
                        }
                    } else {
                        report.left += 1;
                    }
                }
            }
        }
        let rendered = render(vocabulary, &record, &paths, entity, answer.as_ref().map(|(k, t)| (k.as_str(), t.as_str())), &claims);
        report.sentences += rendered.sentences;
        report.discarded += rendered.discarded;
        if dry {
            continue;
        }
        let fingerprint = hash(&[&rendered.text]);
        let file = root.join(path);
        let known = record.pages.get(id);
        if known.is_some_and(|p| p.fingerprint == fingerprint && p.state == "projected") && file.is_file() {
            report.unchanged += 1;
            continue;
        }
        let notes = read_file(&file).map(|t| notes_of(&t, vocabulary.pages.word("notes"))).unwrap_or_default();
        write_file(&file, &with_notes(vocabulary, &dated(&rendered.text), &notes))?;
        base.conn()
            .execute(
                "INSERT OR REPLACE INTO knowledge_page (entity_id, path, state, fingerprint, synthesis, sentences, discarded, written_at)
                 VALUES (?1, ?2, 'projected', ?3, ?4, ?5, ?6, ?7)",
                params![id, path, fingerprint, rendered.synthesis, rendered.sentences as i64, rendered.discarded as i64, now_ms()],
            )
            .map_err(sql_err)?;
        report.written += 1;
    }
    if dry {
        return Ok(());
    }

    // Pages whose entity has no page any more: gone, unless a person wrote
    // notes in them (R-09).
    for (id, page) in &record.pages {
        if paths.contains_key(id) {
            continue;
        }
        let file = root.join(&page.path);
        let notes = read_file(&file).map(|t| notes_of(&t, vocabulary.pages.word("notes"))).unwrap_or_default();
        if notes.trim().is_empty() {
            if file.is_file() {
                std::fs::remove_file(&file).map_err(|e| format!("No se pudo borrar '{}': {e}", file.display()))?;
            }
            base.conn().execute("DELETE FROM knowledge_page WHERE entity_id = ?1", [id]).map_err(sql_err)?;
            report.removed += 1;
            continue;
        }
        let entity = record.entities.get(id);
        let name = entity.map(|e| e.name.clone()).unwrap_or_else(|| id.clone());
        let mut text = format!(
            "---\ntype: entity\ntitle: {}\nentity_id: {id}\nstate: kept\ngenerated_by: {GENERATED_BY}\n---\n\n# {name}\n\n",
            yaml_string(&name)
        );
        match survivor(&record, id).filter(|s| s.id != *id).and_then(|s| paths.get(&s.id).map(|p| (s, p))) {
            Some((into, path)) => text.push_str(&format!("{} {}.\n", vocabulary.pages.word("keptMerged"), link(path, &into.name))),
            None => text.push_str(&format!("{}\n", vocabulary.pages.word("kept"))),
        }
        let fingerprint = hash(&[&text]);
        if page.state == "kept" && page.fingerprint == fingerprint {
            continue;
        }
        write_file(&file, &with_notes(vocabulary, &text, &notes))?;
        base.conn()
            .execute(
                "UPDATE knowledge_page SET state = 'kept', fingerprint = ?2, synthesis = NULL, written_at = ?3 WHERE entity_id = ?1",
                params![id, fingerprint, now_ms()],
            )
            .map_err(sql_err)?;
        report.kept += 1;
    }

    // The block of entities of each document's page.
    let blocks: BTreeMap<String, (String, i64, i64)> = {
        let conn = base.conn();
        let mut statement = conn.prepare("SELECT path, block_hash, size, mtime_ms FROM knowledge_source_block").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, (row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, i64>(3)?))))
            .map_err(sql_err)?;
        rows.collect::<Result<_, _>>().map_err(sql_err)?
    };
    let mut by_page: BTreeMap<&str, Vec<i64>> = BTreeMap::new();
    for (path, content) in &record.source_pages {
        by_page.entry(path.as_str()).or_default().push(*content);
    }
    report.source_pages = by_page.len();
    for (path, contents) in &by_page {
        let block = source_block(vocabulary, &record, &paths, contents[0]);
        let block_hash = hash(&[block.as_deref().unwrap_or("")]);
        let file = root.join(path);
        let (size, mtime) = file_meta(&file);
        if blocks.get(*path).is_some_and(|(h, s, m)| *h == block_hash && *s == size && *m == mtime) {
            continue;
        }
        let Some(text) = read_file(&file) else { continue };
        let next = with_block(&text, block.as_deref());
        if next != text {
            write_file(&file, &next)?;
            report.blocks_written += 1;
        }
        let (size, mtime) = file_meta(&file);
        base.conn()
            .execute(
                "INSERT OR REPLACE INTO knowledge_source_block (path, block_hash, size, mtime_ms) VALUES (?1, ?2, ?3, ?4)",
                params![path, block_hash, size, mtime],
            )
            .map_err(sql_err)?;
    }

    // The overview.
    {
        let documents = record.entities.values().flat_map(|e| e.documents.keys()).collect::<BTreeSet<_>>().len();
        let text = overview(vocabulary, &record, &paths, documents);
        let fingerprint = hash(&[&text]);
        let file = root.join(OVERVIEW_PATH);
        let before = setting(&base.conn(), OVERVIEW_SETTING)?;
        if before.as_deref() != Some(fingerprint.as_str()) || !file.is_file() {
            write_file(&file, &dated(&text))?;
            set_setting(&base.conn(), OVERVIEW_SETTING, &fingerprint)?;
            report.overview_written = true;
        }
    }

    if report.left == 0 && report.failed == 0 {
        base.conn().execute("DELETE FROM knowledge_dirty WHERE scope = 'project' AND since <= ?1", [started]).map_err(sql_err)?;
        // What the next projection compares with: the pages of documents
        // as they are now, blocks included.
        base.index_wiki()?;
        let basis = basis_of(base, &general.id, vocabulary, language.as_deref(), &root)?;
        set_setting(&base.conn(), BASIS_SETTING, &basis)?;
    } else if report.wrote() {
        base.index_wiki()?;
    }
    Ok(())
}
