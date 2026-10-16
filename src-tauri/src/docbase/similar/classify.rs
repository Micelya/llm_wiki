//! Passes 4 and 5: what type of document each one is, in what stage, and
//! the data its type cares about. Pass 6 is in `by_type.rs`.
//!
//! - **4 · Type and stage.** First grouped, then asked (R-21): one
//!   question per provisional family, about its principal document, and
//!   one per document left alone; none when the verified marks already
//!   say it. The question carries the closed list of the catalog's types
//!   and stages and a bounded excerpt of the text; the model chooses from
//!   that list or proposes a new type marked as such (R-22), which stays
//!   a proposal until a person confirms it.
//! - **5 · Data by type.** The rules of the type (`data.rs`); the model
//!   only for the required data they did not find.
//!
//! Every answer is kept with its question, the prompt version and the
//! engine that gave it, by the fingerprint of the question: the same
//! question is never asked twice (R-20). Asking costs the provider's
//! quota, so it only happens in `types_run`, which the app calls on an
//! explicit action of the user or while the archive's wiki is active;
//! everything else here (marks, answers already given, rules) costs
//! nothing and runs with the other passes.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Mutex;

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::super::{now_ms, sql_err, DocBase};
use super::data::{self, Datum};
use super::marks::Mark;
use super::types::{self, Catalog, DataRule, DocType};
use crate::source_volume::asking::{AskConfig, DocumentAsker};

/// Versions of the questions. Changing a prompt changes its version, and
/// every question is asked again.
pub const TYPE_PROMPT: &str = "tipo-2";
pub const DATA_PROMPT: &str = "datos-1";
/// A document with fewer words than this says too little to be typed.
pub const MIN_TYPE_WORDS: i64 = 10;
/// The excerpt of a long text: its first and last words.
pub const EXCERPT_HEAD: usize = 1200;
pub const EXCERPT_TAIL: usize = 300;
/// Members of a family described in its question; the rest take the
/// principal's answer.
pub const MAX_MEMBERS_IN_QUESTION: usize = 30;
/// Relations after which a member is in the principal's stage, unless
/// its own marks say another one.
pub const SAME_STAGE: &[&str] = &["near-identical", "same-template", "version"];
/// Setting with the engine that answers while the archive's wiki is active.
const ASK_ENGINE_SETTING: &str = "ask-engine";

/// One run that asks at a time: the background one and the one a person
/// starts never ask the same question together.
static ASKING: Mutex<()> = Mutex::new(());

/// The fingerprint a question is kept by: its version and its whole text.
pub fn question_key(version: &str, prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(version.as_bytes());
    hasher.update([0]);
    hasher.update(prompt.as_bytes());
    let digest = hasher.finalize();
    format!("{version}:{}", digest.iter().take(16).map(|b| format!("{b:02x}")).collect::<String>())
}

/// What a document is, as the screens show it.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentType {
    pub content_id: i64,
    /// A type of the catalog, or the id the model proposed (`proposed`).
    pub doc_type: String,
    pub stage: Option<String>,
    /// A new type the model proposed: it is not in the catalog and waits
    /// for a person to confirm it.
    pub proposed: bool,
    pub proposed_name: Option<String>,
    pub proposed_description: Option<String>,
    /// "marks" (verified without a model), "model" (asked about this
    /// document) or "family" (the answer about its family's principal).
    pub source: String,
    /// The document the model was asked about.
    pub principal: Option<i64>,
    pub confidence: Option<f64>,
    pub reason: Option<String>,
    pub computed_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProposedType {
    pub id: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub documents: i64,
}

/// What a run would do, for the notice shown before it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypesPlan {
    /// Documents with enough text to be typed.
    pub documents: usize,
    /// Of those, how many have a type now.
    pub typed: usize,
    /// Questions about type and stage still to ask.
    pub type_questions: usize,
    /// Questions about data still to ask (more may appear once the
    /// pending types are known).
    pub data_questions: usize,
    pub proposed_types: Vec<ProposedType>,
    /// The engine that answers while the archive's wiki is active.
    pub engine: Option<AskConfig>,
}

/// What a run did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    /// Questions about type and stage sent to the model.
    pub asked: usize,
    pub failed: usize,
    /// Documents whose type or stage was written in this run.
    pub typed: usize,
    pub data_asked: usize,
    pub data_failed: usize,
    /// Pairs whose relation a rule of the type set or took back.
    pub corrected: usize,
    /// Questions left for another run (limit reached, or the provider
    /// stopped answering).
    pub left: usize,
    pub errors: Vec<String>,
}

impl RunReport {
    pub fn add(&mut self, other: &RunReport) {
        self.asked += other.asked;
        self.failed += other.failed;
        self.typed += other.typed;
        self.data_asked += other.data_asked;
        self.data_failed += other.data_failed;
        self.corrected += other.corrected;
        self.left = other.left;
        self.errors.extend(other.errors.iter().cloned());
    }
}

/// A document as the passes see it.
#[derive(Debug, Clone)]
pub(super) struct DocInfo {
    pub text_at: i64,
    pub marks: Vec<Mark>,
    pub marks_json: String,
}

/// A family (its principal and its members with their relation to it),
/// or a document alone.
#[derive(Debug, Clone)]
struct Unit {
    principal: i64,
    members: Vec<(i64, Option<String>)>,
}

/// A row of `document_type` before it is written.
#[derive(Debug, Clone, PartialEq)]
struct TypeRow {
    content_id: i64,
    doc_type: String,
    stage: Option<String>,
    proposed: bool,
    proposed_name: Option<String>,
    proposed_description: Option<String>,
    source: &'static str,
    principal: Option<i64>,
    confidence: Option<f64>,
    reason: Option<String>,
    question: Option<String>,
}

/// A question still to ask.
#[derive(Debug, Clone)]
pub(super) struct Question {
    pub key: String,
    pub version: &'static str,
    pub task: &'static str,
    pub content_id: i64,
    pub prompt: String,
    pub schema: serde_json::Value,
}

/// The type the verified marks give, and its stage; `decisive` when
/// nothing is left to ask about it.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct ByMarks {
    pub doc_type: String,
    pub stage: Option<String>,
    pub decisive: bool,
}

/// What the marks of a document say about its type. Carriers (a signing
/// service, a form) never give the type; marks of two types say nothing.
pub(super) fn type_by_marks(catalog: &Catalog, marks: &[Mark]) -> Option<ByMarks> {
    let found: BTreeSet<&str> = marks
        .iter()
        .map(|m| m.doc_type.as_str())
        .filter(|t| catalog.doc_type(t).is_some_and(|t| !t.carrier))
        .collect();
    if found.len() != 1 {
        return None;
    }
    let doc_type = found.into_iter().next().expect("one");
    let stages: BTreeSet<&str> =
        marks.iter().filter(|m| m.doc_type == doc_type).filter_map(|m| m.stage.as_deref()).collect();
    let has_stages = catalog.doc_type(doc_type).is_some_and(|t| !t.stages.is_empty());
    Some(match stages.len() {
        1 => ByMarks { doc_type: doc_type.to_string(), stage: stages.into_iter().next().map(str::to_string), decisive: true },
        0 => ByMarks { doc_type: doc_type.to_string(), stage: None, decisive: !has_stages },
        // Marks of two stages contradict each other: the model reads it.
        _ => ByMarks { doc_type: doc_type.to_string(), stage: None, decisive: false },
    })
}

/// The stage a document's own marks give within `doc_type`.
fn own_stage(catalog: &Catalog, marks: &[Mark], doc_type: &str) -> Option<String> {
    type_by_marks(catalog, marks).filter(|m| m.doc_type == doc_type).and_then(|m| m.stage)
}

fn fingerprint(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hasher.finalize().iter().take(12).map(|b| format!("{b:02x}")).collect()
}

/// The documents with enough text to be typed.
pub(super) fn load_docs(conn: &Connection) -> Result<BTreeMap<i64, DocInfo>, String> {
    let mut statement = conn
        .prepare(
            "SELECT s.content_id, s.text_at, coalesce(m.marks, '[]') FROM similar_signature s
             JOIN content c ON c.id = s.content_id AND c.state = 'processed'
             LEFT JOIN document_marks m ON m.content_id = s.content_id
             WHERE s.words >= ?1",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([MIN_TYPE_WORDS], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))
        .map_err(sql_err)?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (id, text_at, marks_json) = row.map_err(sql_err)?;
        let marks: Vec<Mark> = serde_json::from_str(&marks_json).unwrap_or_default();
        out.insert(id, DocInfo { text_at, marks, marks_json });
    }
    Ok(out)
}

/// Families against their principal, then every document left alone.
fn load_units(conn: &Connection, docs: &BTreeMap<i64, DocInfo>) -> Result<Vec<Unit>, String> {
    let mut families: BTreeMap<i64, Vec<(i64, Option<String>)>> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare("SELECT principal, content_id, relation FROM similar_group_member ORDER BY principal, content_id")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, Option<String>>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (principal, member, relation) = row.map_err(sql_err)?;
            let entry = families.entry(principal).or_default();
            if member != principal {
                entry.push((member, relation));
            }
        }
    }
    let mut placed: BTreeSet<i64> = BTreeSet::new();
    let mut units = Vec::new();
    for (principal, members) in families {
        if !docs.contains_key(&principal) {
            continue;
        }
        let members: Vec<(i64, Option<String>)> = members.into_iter().filter(|(m, _)| docs.contains_key(m)).collect();
        placed.insert(principal);
        placed.extend(members.iter().map(|(m, _)| *m));
        units.push(Unit { principal, members });
    }
    for id in docs.keys() {
        if !placed.contains(id) {
            units.push(Unit { principal: *id, members: Vec::new() });
        }
    }
    Ok(units)
}

/// What a unit is decided on, without reading any text: when it is the
/// same, nothing is decided again.
fn unit_basis(catalog: &Catalog, unit: &Unit, docs: &BTreeMap<i64, DocInfo>) -> String {
    let mut parts: Vec<String> = vec![TYPE_PROMPT.to_string(), catalog.digest.clone()];
    let mut add = |id: i64, relation: &str| {
        let doc = &docs[&id];
        parts.push(format!("{id}|{relation}|{}|{}", doc.text_at, doc.marks_json));
    };
    add(unit.principal, "principal");
    for (member, relation) in &unit.members {
        add(*member, relation.as_deref().unwrap_or(""));
    }
    let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    fingerprint(&refs)
}

/// The type rows of a unit decided by its marks, when they decide it.
fn rows_by_marks(catalog: &Catalog, unit: &Unit, docs: &BTreeMap<i64, DocInfo>) -> Option<Vec<TypeRow>> {
    let head = type_by_marks(catalog, &docs[&unit.principal].marks)?;
    if !head.decisive {
        return None;
    }
    let has_stages = catalog.doc_type(&head.doc_type).is_some_and(|t| !t.stages.is_empty());
    let row = |content_id: i64, stage: Option<String>, source: &'static str| TypeRow {
        content_id,
        doc_type: head.doc_type.clone(),
        stage,
        proposed: false,
        proposed_name: None,
        proposed_description: None,
        source,
        principal: (content_id != unit.principal).then_some(unit.principal),
        confidence: None,
        reason: None,
        question: None,
    };
    let mut rows = vec![row(unit.principal, head.stage.clone(), "marks")];
    for (member, relation) in &unit.members {
        let stage = match own_stage(catalog, &docs[member].marks, &head.doc_type) {
            Some(stage) => Some(stage),
            None if !has_stages => None,
            None if relation.as_deref().is_some_and(|r| SAME_STAGE.contains(&r)) => head.stage.clone(),
            // Another stage, and nothing verified says which: the model reads it.
            None => return None,
        };
        rows.push(row(*member, stage, "marks"));
    }
    Some(rows)
}

/// The text of a document, cut to its first and last words when long.
pub fn excerpt(body: &str) -> (String, bool) {
    let lines: Vec<&str> = body
        .lines()
        .filter(|line| !line.trim_start().starts_with(crate::source_volume::catalog::RECOGNIZED_MARKER_START))
        .collect();
    let words = |line: &str| line.split_whitespace().count();
    let total: usize = lines.iter().map(|l| words(l)).sum();
    if total <= EXCERPT_HEAD + EXCERPT_TAIL {
        return (lines.join("\n").trim().to_string(), false);
    }
    let mut head = Vec::new();
    let mut count = 0;
    for line in &lines {
        if count >= EXCERPT_HEAD {
            break;
        }
        count += words(line);
        head.push(*line);
    }
    let mut tail = Vec::new();
    let mut tail_count = 0;
    for line in lines[head.len()..].iter().rev() {
        if tail_count >= EXCERPT_TAIL {
            break;
        }
        tail_count += words(line);
        tail.push(*line);
    }
    tail.reverse();
    let skipped = total.saturating_sub(count + tail_count);
    (format!("{}\n\n[… se omiten unas {skipped} palabras …]\n\n{}", head.join("\n").trim(), tail.join("\n").trim()), true)
}

/// A mark in words, for the question.
fn mark_line(catalog: &Catalog, mark: &Mark) -> String {
    let issuer = mark
        .variant
        .as_deref()
        .and_then(|v| catalog.types.iter().flat_map(|t| t.variants.iter()).find(|variant| variant.id == v))
        .map(|variant| format!(" ({})", variant.issuer))
        .unwrap_or_default();
    let page = mark.page.map(|p| format!(", página {p}")).unwrap_or_default();
    let value = match (&mark.value, mark.kind.as_str()) {
        // A certificate's names say who sealed it; they are already the issuer.
        (Some(_), "service-seal" | "party-signature") => String::new(),
        (Some(value), _) => format!(": {value}"),
        (None, _) => String::new(),
    };
    format!("{}{issuer}{value}{page}", mark.signal.replace('-', " "))
}

fn marks_text(catalog: &Catalog, marks: &[Mark]) -> String {
    let mut lines: Vec<String> = marks.iter().map(|m| mark_line(catalog, m)).collect();
    lines.dedup();
    if lines.is_empty() {
        "nada".to_string()
    } else {
        lines.join("; ")
    }
}

fn type_list(catalog: &Catalog) -> String {
    catalog
        .document_types()
        .map(|t| {
            let stages = if t.stages.is_empty() {
                "no tiene".to_string()
            } else {
                t.stages
                    .iter()
                    .map(|s| match t.stage_descriptions.get(s) {
                        Some(d) => format!("{s} ({})", d.trim()),
                        None => s.clone(),
                    })
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            format!("- {}: {} Etapas: {stages}.", t.id, t.description.trim())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn type_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["tipo", "etapa", "nuevo", "nombre", "descripcion", "confianza", "razon", "otros"],
        "properties": {
            "tipo": { "type": "string" },
            "etapa": { "type": ["string", "null"] },
            "nuevo": { "type": "boolean" },
            "nombre": { "type": ["string", "null"] },
            "descripcion": { "type": ["string", "null"] },
            "confianza": { "type": "number" },
            "razon": { "type": "string" },
            "otros": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["documento", "etapa"],
                    "properties": {
                        "documento": { "type": "integer" },
                        "etapa": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

/// What a family member is to its principal, for the question.
struct MemberView {
    name: String,
    description: String,
    marks: String,
}

/// The question about a unit's type and stage.
fn type_prompt(catalog: &Catalog, name: &str, body: &str, marks: &[Mark], members: &[MemberView]) -> String {
    let (text, cut) = excerpt(body);
    let mut prompt = format!(
        "Clasificá un documento de un archivo profesional según la lista de tipos de abajo.\n\n\
         Tipos de documento (elegí uno de esta lista siempre que corresponda):\n{}\n\n\
         Si ningún tipo de la lista corresponde, proponé uno nuevo: en \"tipo\" un identificador corto en minúsculas con guiones, \
         en \"nombre\" cómo se llama y en \"descripcion\" qué es, en términos generales (sin nombres de personas, empresas ni números), \
         y \"nuevo\": true. Si elegís uno de la lista, \"nuevo\" es false y \"nombre\" y \"descripcion\" van en null.\n\
         La etapa es una de las del tipo elegido, o null si el tipo no tiene etapas o el documento no permite saberla.\n\n\
         Lo que ya se verificó en el documento sin el modelo: {}.\n",
        type_list(catalog),
        marks_text(catalog, marks)
    );
    if !members.is_empty() {
        prompt.push_str(
            "\nEste documento es el principal de un grupo de documentos que comparten su texto de base; \
             el tipo que elijas vale para todos. En \"otros\" indicá la etapa de cada uno de los demás, \
             leyendo en qué se diferencia del principal (null solo si no se puede saber):\n",
        );
        for (index, member) in members.iter().enumerate() {
            prompt.push_str(&format!(
                "- Documento {} (archivo «{}»): {} Verificado: {}.\n",
                index + 2,
                member.name,
                member.description,
                member.marks
            ));
        }
    }
    prompt.push_str(&format!(
        "\nEl nombre del archivo es una pista, no una prueba: si el contenido dice otra cosa, manda el contenido.\n\
         Archivo: «{name}»\n\n\
         Texto del documento{}:\n\"\"\"\n{text}\n\"\"\"\n\n\
         Respondé solo con el JSON pedido. \"confianza\" va de 0 a 1; \"razon\" es una frase corta que dice en qué te basaste.",
        if cut { " (extracto: el principio y el final)" } else { "" }
    ));
    prompt
}

#[derive(Debug, Deserialize)]
struct TypeAnswer {
    tipo: String,
    etapa: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    nuevo: bool,
    nombre: Option<String>,
    descripcion: Option<String>,
    #[serde(default)]
    confianza: Option<f64>,
    #[serde(default)]
    razon: Option<String>,
    #[serde(default)]
    otros: Vec<MemberStage>,
}

#[derive(Debug, Deserialize)]
struct MemberStage {
    documento: i64,
    etapa: Option<String>,
}

/// The JSON of an answer, even when the engine wrapped it in a code block.
fn parse_json<T: for<'de> Deserialize<'de>>(answer: &str) -> Result<T, String> {
    let trimmed = answer.trim();
    if let Ok(value) = serde_json::from_str(trimmed) {
        return Ok(value);
    }
    let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) else {
        return Err("La respuesta del modelo no es JSON".to_string());
    };
    serde_json::from_str(&trimmed[start..=end]).map_err(|e| format!("La respuesta del modelo no tiene la forma pedida: {e}"))
}

/// A proposed type's id: lowercase letters, digits and dashes.
fn clean_id(id: &str) -> String {
    let words = super::text::normalized(id);
    let id = words.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-");
    if id.is_empty() {
        "sin-tipo".to_string()
    } else {
        id
    }
}

/// The rows a type answer gives a unit.
fn rows_from_answer(
    catalog: &Catalog,
    unit: &Unit,
    docs: &BTreeMap<i64, DocInfo>,
    answer: &str,
    question: &str,
) -> Result<Vec<TypeRow>, String> {
    let answer: TypeAnswer = parse_json(answer)?;
    let id = clean_id(&answer.tipo);
    let known: Option<&DocType> = catalog.doc_type(&id).filter(|t| !t.carrier);
    let proposed = known.is_none();
    let valid_stage = |stage: Option<String>| -> Option<String> {
        let stage = stage.map(|s| clean_id(&s))?;
        match known {
            Some(t) => t.stages.contains(&stage).then_some(stage),
            // A new type has no list of stages yet: its stage is kept as said.
            None => Some(stage),
        }
    };
    let principal_stage = own_stage(catalog, &docs[&unit.principal].marks, &id).or_else(|| valid_stage(answer.etapa.clone()));
    let confidence = answer.confianza.map(|c| c.clamp(0.0, 1.0));
    let reason = answer.razon.map(|r| r.trim().to_string()).filter(|r| !r.is_empty());
    let row = |content_id: i64, stage: Option<String>, source: &'static str| TypeRow {
        content_id,
        doc_type: id.clone(),
        stage,
        proposed,
        proposed_name: if proposed { answer.nombre.clone().filter(|n| !n.trim().is_empty()) } else { None },
        proposed_description: if proposed { answer.descripcion.clone().filter(|d| !d.trim().is_empty()) } else { None },
        source,
        principal: Some(unit.principal),
        confidence,
        reason: reason.clone(),
        question: Some(question.to_string()),
    };
    let mut rows = vec![row(unit.principal, principal_stage.clone(), "model")];
    let said: HashMap<i64, Option<String>> = answer.otros.into_iter().map(|m| (m.documento, m.etapa)).collect();
    for (index, (member, relation)) in unit.members.iter().enumerate() {
        let asked_about = index < MAX_MEMBERS_IN_QUESTION;
        let answered = if asked_about { said.get(&(index as i64 + 2)).cloned().flatten() } else { None };
        let stage = own_stage(catalog, &docs[member].marks, &id)
            .or_else(|| valid_stage(answered))
            .or_else(|| {
                // Not said: the principal's, only for a member in the same stage.
                relation.as_deref().is_some_and(|r| SAME_STAGE.contains(&r)).then(|| principal_stage.clone()).flatten()
            });
        rows.push(row(*member, stage, "family"));
    }
    Ok(rows)
}

fn write_type_rows(conn: &Connection, rows: &[TypeRow], basis: &str) -> Result<usize, String> {
    let now = now_ms();
    let mut changed = 0;
    for row in rows {
        let before: Option<(String, Option<String>, String)> = conn
            .query_row(
                "SELECT doc_type, stage, basis FROM document_type WHERE content_id = ?1",
                [row.content_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()
            .map_err(sql_err)?;
        if before.as_ref().is_some_and(|(t, s, b)| t == &row.doc_type && s == &row.stage && b == basis) {
            continue;
        }
        conn.execute(
            "INSERT OR REPLACE INTO document_type
                (content_id, doc_type, stage, proposed, proposed_name, proposed_description, source, principal,
                 confidence, reason, question, basis, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                row.content_id,
                row.doc_type,
                row.stage,
                row.proposed,
                row.proposed_name,
                row.proposed_description,
                row.source,
                row.principal,
                row.confidence,
                row.reason,
                row.question,
                basis,
                now
            ],
        )
        .map_err(sql_err)?;
        changed += 1;
    }
    Ok(changed)
}

fn read_proposed_types(conn: &Connection) -> Result<Vec<ProposedType>, String> {
    let mut statement = conn
        .prepare(
            "SELECT doc_type, max(proposed_name), max(proposed_description), count(*) FROM document_type
             WHERE proposed = 1 GROUP BY doc_type ORDER BY count(*) DESC, doc_type",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok(ProposedType { id: row.get(0)?, name: row.get(1)?, description: row.get(2)?, documents: row.get(3)? }))
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    Ok(rows)
}

fn cached_answer(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT answer FROM model_answer WHERE question = ?1", [key], |row| row.get(0))
        .optional()
        .map_err(sql_err)
}

fn body_of(conn: &Connection, content_id: i64) -> Result<String, String> {
    conn.query_row("SELECT body FROM derivative WHERE content_id = ?1 AND kind = 'text'", [content_id], |row| row.get(0))
        .optional()
        .map_err(sql_err)
        .map(Option::unwrap_or_default)
}

fn name_of(conn: &Connection, content_id: i64) -> Result<String, String> {
    conn.query_row(
        "SELECT l.name FROM location l JOIN file f ON f.hash = l.file_hash WHERE f.content_id = ?1
         ORDER BY l.rel_path COLLATE NOCASE LIMIT 1",
        [content_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_err)
    .map(Option::unwrap_or_default)
}

/// The type rows a pending type question will write, once answered.
struct PendingType {
    unit: Unit,
    basis: String,
    question: Question,
}

/// The data a pending data question will complete, once answered.
struct PendingData {
    content_id: i64,
    question: Question,
}

/// The data rules that apply to a document: those of its type and those
/// of the carriers its marks show (a signing service, a form).
pub(super) fn data_rules<'a>(catalog: &'a Catalog, doc_type: Option<&str>, marks: &[Mark]) -> Vec<&'a DataRule> {
    let mut rules: Vec<&DataRule> = Vec::new();
    if let Some(t) = doc_type.and_then(|t| catalog.doc_type(t)) {
        rules.extend(t.data.iter());
    }
    let carriers: BTreeSet<&str> = marks
        .iter()
        .map(|m| m.doc_type.as_str())
        .filter(|t| catalog.doc_type(t).is_some_and(|t| t.carrier))
        .collect();
    for carrier in carriers {
        for rule in &catalog.doc_type(carrier).expect("listed").data {
            if !rules.iter().any(|r| r.id == rule.id) {
                rules.push(rule);
            }
        }
    }
    rules
}

fn data_schema(fields: &[&DataRule]) -> serde_json::Value {
    let properties: serde_json::Map<String, serde_json::Value> = fields
        .iter()
        .map(|f| (f.id.clone(), serde_json::json!({ "type": "array", "items": { "type": "string" } })))
        .collect();
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": fields.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
        "properties": properties,
    })
}

fn data_prompt(doc_type: &DocType, fields: &[&DataRule], name: &str, body: &str) -> String {
    let (text, cut) = excerpt(body);
    let list = fields
        .iter()
        .map(|f| format!("- {}: {}{}", f.id, f.description.trim(), if f.many { " (puede haber varios)" } else { "" }))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Este documento es del tipo «{}»: {}\n\n\
         Copiá de su texto estos datos, cada valor tal como aparece escrito:\n{list}\n\n\
         Si un dato no está en el texto, devolvé una lista vacía para ese dato. No inventes ni completes valores.\n\n\
         Archivo: «{name}»\n\n\
         Texto del documento{}:\n\"\"\"\n{text}\n\"\"\"\n\n\
         Respondé solo con el JSON pedido.",
        doc_type.id,
        doc_type.description.trim(),
        if cut { " (extracto: el principio y el final)" } else { "" }
    )
}

fn data_from_answer(fields: &[&DataRule], answer: &str) -> Result<Vec<Datum>, String> {
    let value: serde_json::Value = parse_json(answer)?;
    let mut out = Vec::new();
    for field in fields {
        let values: Vec<String> = match &value[&field.id] {
            serde_json::Value::Array(items) => items.iter().filter_map(|v| v.as_str()).map(|v| v.trim().to_string()).collect(),
            serde_json::Value::String(one) => vec![one.trim().to_string()],
            _ => Vec::new(),
        };
        for v in values.into_iter().filter(|v| !v.is_empty()) {
            if !out.iter().any(|d: &Datum| d.field == field.id && data::comparable(&d.value) == data::comparable(&v)) {
                out.push(Datum { field: field.id.clone(), value: v, source: "model".into() });
            }
            if !field.many {
                break;
            }
        }
    }
    Ok(out)
}

impl DocBase {
    /// Pass 4 without the model: types the marks decide, and answers the
    /// model already gave. Returns the rows written and the questions
    /// still to ask.
    fn pass4_refresh(&self) -> Result<(usize, Vec<PendingType>), String> {
        let catalog = types::catalog();
        let conn = self.conn();
        let docs = load_docs(&conn)?;
        let units = load_units(&conn, &docs)?;
        let current: HashMap<i64, String> = {
            let mut statement = conn.prepare("SELECT content_id, basis FROM document_type").map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?
                .collect::<Result<HashMap<_, _>, _>>()
                .map_err(sql_err)?;
            rows
        };
        // Documents that no longer have enough text lose their type.
        let gone: Vec<i64> = current.keys().filter(|id| !docs.contains_key(id)).copied().collect();
        let mut written = 0;
        for id in gone {
            conn.execute("DELETE FROM document_type WHERE content_id = ?1", [id]).map_err(sql_err)?;
            conn.execute("DELETE FROM document_data WHERE content_id = ?1", [id]).map_err(sql_err)?;
            written += 1;
        }
        let mut pending = Vec::new();
        for unit in units {
            let basis = unit_basis(catalog, &unit, &docs);
            let all = std::iter::once(unit.principal).chain(unit.members.iter().map(|(m, _)| *m));
            if all.clone().all(|id| current.get(&id) == Some(&basis)) {
                continue;
            }
            if let Some(rows) = rows_by_marks(catalog, &unit, &docs) {
                written += write_type_rows(&conn, &rows, &basis)?;
                continue;
            }
            let question = type_question(&conn, catalog, &unit, &docs)?;
            match cached_answer(&conn, &question.key)? {
                Some(answer) => match rows_from_answer(catalog, &unit, &docs, &answer, &question.key) {
                    Ok(rows) => written += write_type_rows(&conn, &rows, &basis)?,
                    Err(error) => eprintln!("[docbase] stored answer {}: {error}", question.key),
                },
                None => pending.push(PendingType { unit, basis, question }),
            }
        }
        Ok((written, pending))
    }

    /// Pass 5 without the model: the data the rules find, completed with
    /// answers the model already gave. Returns the rows written and the
    /// questions still to ask.
    fn pass5_refresh(&self) -> Result<(usize, Vec<PendingData>), String> {
        let catalog = types::catalog();
        let conn = self.conn();
        let docs = load_docs(&conn)?;
        let typed: Vec<(i64, String, bool)> = {
            let mut statement = conn.prepare("SELECT content_id, doc_type, proposed FROM document_type").map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let current: HashMap<i64, (String, Option<String>)> = {
            let mut statement = conn.prepare("SELECT content_id, basis, question FROM document_data").map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, (row.get(1)?, row.get(2)?))))
                .map_err(sql_err)?
                .collect::<Result<HashMap<_, _>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let mut written = 0;
        let mut pending = Vec::new();
        for (content_id, doc_type, proposed) in typed {
            let Some(doc) = docs.get(&content_id) else { continue };
            let own = (!proposed).then_some(doc_type.as_str());
            let rules = data_rules(catalog, own, &doc.marks);
            let basis = fingerprint(&[DATA_PROMPT, &catalog.digest, &doc_type, &doc.text_at.to_string(), &doc.marks_json]);
            if let Some((old, question)) = current.get(&content_id) {
                if old == &basis {
                    // Unchanged; it may still wait for its question.
                    if let Some(key) = question {
                        if cached_answer(&conn, key)?.is_none() {
                            if let Some(question) = data_question(&conn, catalog, content_id, own, &doc.marks, &rules)? {
                                pending.push(PendingData { content_id, question });
                            }
                        }
                    }
                    continue;
                }
            }
            let body = body_of(&conn, content_id)?;
            let (mut found, missing) = data::extract(&rules, &body, &doc.marks);
            let mut question_key = None;
            if !missing.is_empty() {
                if let Some(question) = data_question(&conn, catalog, content_id, own, &doc.marks, &rules)? {
                    question_key = Some(question.key.clone());
                    match cached_answer(&conn, &question.key)? {
                        Some(answer) => {
                            let fields: Vec<&DataRule> = rules.iter().copied().filter(|r| missing.contains(&r.id)).collect();
                            match data_from_answer(&fields, &answer) {
                                Ok(more) => found.extend(more),
                                Err(error) => eprintln!("[docbase] stored answer {}: {error}", question.key),
                            }
                        }
                        None => pending.push(PendingData { content_id, question }),
                    }
                }
            }
            let still_missing: Vec<&String> = missing.iter().filter(|m| !found.iter().any(|d| &d.field == *m)).collect();
            conn.execute(
                "INSERT OR REPLACE INTO document_data (content_id, doc_type, data, missing, question, basis, computed_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    content_id,
                    doc_type,
                    serde_json::to_string(&found).map_err(|e| e.to_string())?,
                    serde_json::to_string(&still_missing).map_err(|e| e.to_string())?,
                    question_key,
                    basis,
                    now_ms()
                ],
            )
            .map_err(sql_err)?;
            written += 1;
        }
        // Data of documents that lost their type.
        written += conn
            .execute("DELETE FROM document_data WHERE content_id NOT IN (SELECT content_id FROM document_type)", [])
            .map_err(sql_err)?;
        Ok((written, pending))
    }

    /// Everything of passes 4 to 6 that costs nothing: marks, answers
    /// already given, data rules, relations by type. Returns how much
    /// changed.
    pub fn types_refresh(&self) -> Result<usize, String> {
        let (typed, _) = self.pass4_refresh()?;
        let (data, _) = self.pass5_refresh()?;
        let corrected = self.apply_type_rules()?;
        if corrected > 0 {
            self.similar_regroup()?;
        }
        Ok(typed + data + corrected)
    }

    /// What a run would ask now.
    pub fn types_plan(&self) -> Result<TypesPlan, String> {
        let (_, pending_types) = self.pass4_refresh()?;
        let (_, pending_data) = self.pass5_refresh()?;
        if self.apply_type_rules()? > 0 {
            self.similar_regroup()?;
        }
        let conn = self.conn();
        let documents = load_docs(&conn)?.len();
        let typed: i64 = conn.query_row("SELECT count(*) FROM document_type", [], |row| row.get(0)).map_err(sql_err)?;
        let proposed_types = read_proposed_types(&conn)?;
        let mut keys: BTreeSet<&str> = BTreeSet::new();
        let data_questions = pending_data.iter().filter(|p| keys.insert(p.question.key.as_str())).count();
        Ok(TypesPlan {
            documents,
            typed: typed as usize,
            type_questions: pending_types.len(),
            data_questions,
            proposed_types,
            engine: ask_engine(&conn)?,
        })
    }

    /// The types the model proposed that no person confirmed yet, with how
    /// many documents have each one. Reads what is stored; costs nothing.
    pub fn proposed_types(&self) -> Result<Vec<ProposedType>, String> {
        read_proposed_types(&self.conn())
    }

    /// Asks the model what is pending, at most `max_questions` questions,
    /// and applies the answers. Then runs pass 6.
    pub fn types_run(&self, asker: &dyn DocumentAsker, max_questions: usize) -> Result<RunReport, String> {
        let _one_at_a_time = ASKING.lock().unwrap_or_else(|p| p.into_inner());
        let mut report = RunReport::default();
        let catalog = types::catalog();
        let mut budget = max_questions;
        let mut stopped = false;

        let (written, pending) = self.pass4_refresh()?;
        report.typed += written;
        for item in pending {
            if budget == 0 || stopped {
                report.left += 1;
                continue;
            }
            budget -= 1;
            report.asked += 1;
            match self.ask(asker, &item.question) {
                Ok(answer) => {
                    let conn = self.conn();
                    let docs = load_docs(&conn)?;
                    if !std::iter::once(item.unit.principal).chain(item.unit.members.iter().map(|(m, _)| *m)).all(|id| docs.contains_key(&id)) {
                        continue;
                    }
                    match rows_from_answer(catalog, &item.unit, &docs, &answer, &item.question.key) {
                        Ok(rows) => report.typed += write_type_rows(&conn, &rows, &item.basis)?,
                        Err(error) => {
                            report.failed += 1;
                            report.errors.push(error);
                        }
                    }
                }
                Err(error) => {
                    report.failed += 1;
                    stopped = stops_the_run(&error);
                    report.errors.push(error);
                }
            }
        }

        let (_, pending) = self.pass5_refresh()?;
        let mut asked_keys: BTreeSet<String> = BTreeSet::new();
        for item in pending {
            if asked_keys.contains(&item.question.key) {
                continue;
            }
            if budget == 0 || stopped {
                report.left += 1;
                continue;
            }
            budget -= 1;
            report.data_asked += 1;
            asked_keys.insert(item.question.key.clone());
            if let Err(error) = self.ask(asker, &item.question) {
                report.data_failed += 1;
                stopped = stops_the_run(&error);
                report.errors.push(format!("{} ({})", error, item.content_id));
            }
        }
        // The answers just saved complete the data.
        if report.data_asked > 0 {
            self.conn()
                .execute("UPDATE document_data SET basis = '' WHERE question IN (SELECT question FROM model_answer)", [])
                .map_err(sql_err)?;
            self.pass5_refresh()?;
        }

        report.corrected = self.apply_type_rules()?;
        if report.corrected > 0 {
            self.similar_regroup()?;
        }
        Ok(report)
    }

    /// Sends one question and keeps its answer.
    fn ask(&self, asker: &dyn DocumentAsker, question: &Question) -> Result<String, String> {
        if let Some(answer) = cached_answer(&self.conn(), &question.key)? {
            return Ok(answer);
        }
        let answer = asker.ask(&question.prompt, &question.schema)?;
        // Only an answer of the shape asked for is kept.
        parse_json::<serde_json::Value>(&answer)?;
        self.conn()
            .execute(
                "INSERT OR REPLACE INTO model_answer (question, task, prompt_version, engine, content_id, prompt, answer, asked_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    question.key,
                    question.task,
                    question.version,
                    asker.engine_id(),
                    question.content_id,
                    question.prompt,
                    answer,
                    now_ms()
                ],
            )
            .map_err(sql_err)?;
        Ok(answer)
    }

    /// The type of the document at this location, and its data.
    pub fn document_type_of(&self, location_id: i64) -> Result<(Option<DocumentType>, Vec<Datum>), String> {
        let conn = self.conn();
        let content: Option<i64> = conn
            .query_row(
                "SELECT f.content_id FROM location l JOIN file f ON f.hash = l.file_hash WHERE l.id = ?1",
                [location_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?;
        let Some(content) = content else {
            return Ok((None, Vec::new()));
        };
        let found = read_types(&conn, Some(content))?.remove(&content);
        let data = read_data(&conn, Some(content))?.remove(&content).flatten().unwrap_or_default();
        Ok((found, data))
    }

    #[cfg(test)]
    pub fn document_types_all(&self) -> Result<BTreeMap<i64, DocumentType>, String> {
        read_types(&self.conn(), None)
    }

    #[cfg(test)]
    pub fn document_data_all(&self) -> Result<BTreeMap<i64, Vec<Datum>>, String> {
        Ok(read_data(&self.conn(), None)?.into_iter().map(|(k, v)| (k, v.unwrap_or_default())).collect())
    }

    /// The engine that answers while the archive's wiki is active; none
    /// clears it.
    pub fn set_ask_engine(&self, engine: Option<&AskConfig>) -> Result<(), String> {
        let conn = self.conn();
        match engine {
            Some(engine) => conn
                .execute(
                    "INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)",
                    params![ASK_ENGINE_SETTING, serde_json::to_string(engine).map_err(|e| e.to_string())?],
                )
                .map_err(sql_err)?,
            None => conn.execute("DELETE FROM docbase_setting WHERE key = ?1", [ASK_ENGINE_SETTING]).map_err(sql_err)?,
        };
        Ok(())
    }

    /// The engine to ask with in the background: only while the archive's
    /// wiki is active and someone said which one.
    pub fn background_ask_engine(&self) -> Result<Option<AskConfig>, String> {
        let conn = self.conn();
        let active: Option<i64> = conn
            .query_row("SELECT wiki_active FROM project WHERE kind = 'general'", [], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        if active != Some(1) {
            return Ok(None);
        }
        ask_engine(&conn)
    }
}

fn ask_engine(conn: &Connection) -> Result<Option<AskConfig>, String> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM docbase_setting WHERE key = ?1", [ASK_ENGINE_SETTING], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    Ok(value.and_then(|v| serde_json::from_str(&v).ok()))
}

/// A failure after which asking more in the same run only wastes time:
/// the provider is at its limit or cannot be reached.
pub fn stops_the_run(error: &str) -> bool {
    let error = error.to_lowercase();
    ["limit", "429", "quota", "not found on path", "failed to start", "did not answer within"]
        .iter()
        .any(|w| error.contains(w))
}

fn type_question(conn: &Connection, catalog: &Catalog, unit: &Unit, docs: &BTreeMap<i64, DocInfo>) -> Result<Question, String> {
    let body = body_of(conn, unit.principal)?;
    let name = name_of(conn, unit.principal)?;
    let mut members = Vec::new();
    for (member, _) in unit.members.iter().take(MAX_MEMBERS_IN_QUESTION) {
        let (a, b) = (unit.principal.min(*member), unit.principal.max(*member));
        let description: String = conn
            .query_row("SELECT description FROM similar_pair WHERE a = ?1 AND b = ?2", params![a, b], |row| row.get(0))
            .optional()
            .map_err(sql_err)?
            .unwrap_or_default();
        members.push(MemberView {
            name: name_of(conn, *member)?,
            description,
            marks: marks_text(catalog, &docs[member].marks),
        });
    }
    let prompt = type_prompt(catalog, &name, &body, &docs[&unit.principal].marks, &members);
    Ok(Question {
        key: question_key(TYPE_PROMPT, &prompt),
        version: TYPE_PROMPT,
        task: "type",
        content_id: unit.principal,
        prompt,
        schema: type_schema(),
    })
}

/// The question about the required data the rules did not find, when
/// there are such data.
fn data_question(
    conn: &Connection,
    catalog: &Catalog,
    content_id: i64,
    doc_type: Option<&str>,
    marks: &[Mark],
    rules: &[&DataRule],
) -> Result<Option<Question>, String> {
    let body = body_of(conn, content_id)?;
    let (_, missing) = data::extract(rules, &body, marks);
    let fields: Vec<&DataRule> = rules.iter().copied().filter(|r| missing.contains(&r.id)).collect();
    if fields.is_empty() {
        return Ok(None);
    }
    // The type the question speaks of: the document's own, or the carrier
    // whose data are missing.
    let Some(about) = doc_type
        .and_then(|t| catalog.doc_type(t))
        .filter(|t| t.data.iter().any(|d| missing.contains(&d.id)))
        .or_else(|| catalog.types.iter().find(|t| t.carrier && t.data.iter().any(|d| missing.contains(&d.id))))
    else {
        return Ok(None);
    };
    let prompt = data_prompt(about, &fields, &name_of(conn, content_id)?, &body);
    Ok(Some(Question {
        key: question_key(DATA_PROMPT, &prompt),
        version: DATA_PROMPT,
        task: "data",
        content_id,
        prompt,
        schema: data_schema(&fields),
    }))
}

pub(super) fn read_types(conn: &Connection, only: Option<i64>) -> Result<BTreeMap<i64, DocumentType>, String> {
    let mut statement = conn
        .prepare(
            "SELECT content_id, doc_type, stage, proposed, proposed_name, proposed_description, source, principal,
                    confidence, reason, computed_at
             FROM document_type WHERE ?1 IS NULL OR content_id = ?1",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([only], |row| {
            Ok(DocumentType {
                content_id: row.get(0)?,
                doc_type: row.get(1)?,
                stage: row.get(2)?,
                proposed: row.get(3)?,
                proposed_name: row.get(4)?,
                proposed_description: row.get(5)?,
                source: row.get(6)?,
                principal: row.get(7)?,
                confidence: row.get(8)?,
                reason: row.get(9)?,
                computed_at: row.get(10)?,
            })
        })
        .map_err(sql_err)?;
    let mut out = BTreeMap::new();
    for row in rows {
        let row = row.map_err(sql_err)?;
        out.insert(row.content_id, row);
    }
    Ok(out)
}

pub(super) fn read_data(conn: &Connection, only: Option<i64>) -> Result<BTreeMap<i64, Option<Vec<Datum>>>, String> {
    let mut statement = conn
        .prepare("SELECT content_id, data FROM document_data WHERE ?1 IS NULL OR content_id = ?1")
        .map_err(sql_err)?;
    let rows = statement
        .query_map([only], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
        .map_err(sql_err)?;
    let mut out = BTreeMap::new();
    for row in rows {
        let (id, data) = row.map_err(sql_err)?;
        out.insert(id, serde_json::from_str(&data).ok());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(doc_type: &str, kind: &str, stage: Option<&str>) -> Mark {
        Mark {
            kind: kind.into(),
            doc_type: doc_type.into(),
            variant: None,
            stage: stage.map(str::to_string),
            signal: "s".into(),
            page: None,
            value: None,
        }
    }

    #[test]
    fn marks_give_the_type_only_when_they_are_of_one_type_and_not_a_carrier() {
        let catalog = Catalog::builtin();
        let preview = type_by_marks(&catalog, &[mark("carta-documento", "preview", Some("vista-previa"))]).unwrap();
        assert_eq!(preview, ByMarks { doc_type: "carta-documento".into(), stage: Some("vista-previa".into()), decisive: true });
        // A type with stages and no stage mark: the stage is asked.
        let form = type_by_marks(&catalog, &[mark("carta-documento", "postal-form", None)]).unwrap();
        assert!(!form.decisive);
        // A carrier never gives the type.
        assert_eq!(type_by_marks(&catalog, &[mark("pdf-servicio-de-firma", "service-seal", None)]), None);
        // Two types: nothing.
        assert_eq!(
            type_by_marks(
                &catalog,
                &[mark("carta-documento", "postal-form", None), mark("acuse-de-recibo", "delivery-receipt", None)]
            ),
            None
        );
    }

    #[test]
    fn a_long_text_is_cut_to_its_beginning_and_end() {
        let body: String = (0..5000).map(|i| format!("palabra{i}\n")).collect();
        let (text, cut) = excerpt(&body);
        assert!(cut);
        assert!(text.contains("palabra0\n"));
        assert!(text.contains("palabra4999"));
        assert!(!text.contains("palabra2500\n"));
        assert!(text.contains("se omiten unas"));
        let (short, cut) = excerpt("uno dos tres");
        assert_eq!((short.as_str(), cut), ("uno dos tres", false));
    }

    #[test]
    fn the_question_key_depends_on_the_version_and_the_text() {
        assert_eq!(question_key("v1", "a"), question_key("v1", "a"));
        assert_ne!(question_key("v1", "a"), question_key("v2", "a"));
        assert_ne!(question_key("v1", "a"), question_key("v1", "b"));
    }

    #[test]
    fn an_answer_wrapped_in_a_code_block_is_still_read() {
        let answer = "```json\n{\"tipo\": \"factura\", \"etapa\": null, \"nuevo\": false, \"nombre\": null, \"descripcion\": null, \"confianza\": 0.9, \"razon\": \"x\", \"otros\": []}\n```";
        let parsed: TypeAnswer = parse_json(answer).unwrap();
        assert_eq!(parsed.tipo, "factura");
        assert!(parse_json::<TypeAnswer>("no sé").is_err());
    }

    #[test]
    fn a_proposed_id_is_made_of_plain_words() {
        assert_eq!(clean_id("Contrato de Préstamo"), "contrato-de-prestamo");
        assert_eq!(clean_id("  "), "sin-tipo");
    }
}
