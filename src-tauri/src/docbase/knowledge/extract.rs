//! Stage 1 of the knowledge layer: one reading of each document by the
//! model, and the check of every quote it gives.
//!
//! - **What is asked.** The mentions of people, organizations and matters
//!   (type, how it is written, role, quote, page), the identifiers of
//!   each one and the claims (subject, predicate of the list, object,
//!   quote, page, time of the fact when the text says it), in strict JSON
//!   with closed lists from the vocabulary. The instructions speak of the
//!   documents of a law firm and forbid adding anything the document
//!   does not say. As a hint, what the base already knows without a
//!   model: the identifiers it recognized by their shape, the type and
//!   stage of the document and its data by type. A long document is read
//!   in parts of whole pages.
//! - **What is kept.** Only what the computation verifies (`quote.rs`): a
//!   quote that is in the text, with the name of the mention, the value of
//!   the identifier or of the claim inside it. Everything else is
//!   discarded and counted, with its reason.
//! - **When.** A document is read again only when its text, the version of
//!   the question or the vocabulary changed (`basis`). Every answer is
//!   kept with the engine and model that gave it: the same question is not
//!   asked twice (R-20). A document whose type is still to be asked waits
//!   for it: the type goes into the question.

use std::collections::{BTreeSet, HashMap};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::quote::{self, Located};
use super::vocabulary::{vocabulary, Vocabulary};
use super::{mark_dirty, visible_content, Discarded};
use crate::docbase::similar::changes::{classify as kind_of_token, Kind};
use crate::docbase::similar::classify::{
    self as classify, ask_and_keep, cached_answer, parse_json, stops_the_run, Question, ASKING, MIN_TYPE_WORDS,
};
use crate::docbase::similar::data::Datum;
use crate::docbase::similar::text::{pages, Prepared};
use crate::docbase::{now_ms, sql_err, DocBase, Visible};
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::catalog::{is_page_heading, RECOGNIZED_MARKER_START};

/// Version of the question. Changing the prompt changes it, and every
/// document is read again.
pub const KNOWLEDGE_PROMPT: &str = "conocimiento-1";
/// The task the answers are kept under in `model_answer`.
pub const TASK: &str = "knowledge";
/// Words of text in one question; a longer document is read in parts of
/// whole pages.
pub const PART_WORDS: usize = 4000;

/// The type of entity of a person (`knowledge.json`), and how many words
/// before a number are looked at to tell whose it is.
const PERSON_KIND: &str = "persona";
const OTHER_PERSON_WINDOW: usize = 6;
/// Identifiers recognized by their shape handed over as a hint.
const MAX_SHAPE_HINTS: usize = 60;

/// What a run did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeReport {
    /// Questions sent to the model.
    pub asked: usize,
    /// Questions the model did not answer (or answered in another shape).
    pub failed: usize,
    /// Documents whose reading was written in this run.
    pub extracted: usize,
    /// Documents whose answer could not be used.
    pub unusable: usize,
    /// Documents left for another run (limit reached, or the provider
    /// stopped answering).
    pub left: usize,
    /// Documents waiting for their type to be asked first.
    pub waiting_for_type: usize,
    pub mentions: usize,
    pub evidence: usize,
    pub discarded: usize,
    pub errors: Vec<String>,
}

impl KnowledgeReport {
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn add(&mut self, other: &KnowledgeReport) {
        self.asked += other.asked;
        self.failed += other.failed;
        self.extracted += other.extracted;
        self.unusable += other.unusable;
        self.left = other.left;
        self.waiting_for_type = other.waiting_for_type;
        self.mentions += other.mentions;
        self.evidence += other.evidence;
        self.discarded += other.discarded;
        self.errors.extend(other.errors.iter().cloned());
    }
}

/// What a run would do, for the notice shown before it.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgePlan {
    /// Documents still to read.
    pub documents: usize,
    /// Questions that takes.
    pub questions: usize,
    /// Documents that wait for their type to be asked first.
    pub waiting_for_type: usize,
    /// The engine that reads while the archive's wiki is active.
    pub engine: Option<crate::source_volume::asking::AskConfig>,
}

fn fingerprint(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hasher.finalize().iter().take(12).map(|b| format!("{b:02x}")).collect()
}

fn text_hash(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().take(16).map(|b| format!("{b:02x}")).collect()
}

/// What a reading is decided on: when it is the same, nothing is read again.
fn basis(vocabulary: &Vocabulary, text_hash: &str) -> String {
    fingerprint(&[KNOWLEDGE_PROMPT, &vocabulary.reading_digest, text_hash])
}

/// How the reading of one document stands.
#[derive(Debug, Clone)]
pub(crate) struct Standing {
    pub content_id: i64,
    /// When its text was written (`derivative.created_at`).
    pub text_at: i64,
    pub reading: Option<ReadingRow>,
}

#[derive(Debug, Clone)]
pub(crate) struct ReadingRow {
    pub state: String,
    pub basis: String,
    /// Read on the current text, question and vocabulary.
    pub current: bool,
    pub discarded: String,
}

/// The documents with enough text to be read (all of them, or `only`),
/// over what `visible` lets see, and how their reading stands. Whether a
/// reading is current is told without reading the texts: the same text
/// (by when it was written), question and vocabulary.
pub(crate) fn standings(conn: &Connection, visible: &Visible, only: Option<i64>) -> Result<Vec<Standing>, String> {
    let vocabulary = vocabulary();
    let (condition, params) = visible_content("c.id", visible);
    let mut values: Vec<rusqlite::types::Value> = vec![MIN_TYPE_WORDS.into(), only.into()];
    values.extend(params.into_iter().map(rusqlite::types::Value::from));
    let mut statement = conn
        .prepare(&format!(
            "SELECT c.id, d.created_at, k.state, k.basis, k.text_at, k.prompt_version, k.vocabulary, k.discarded
             FROM content c JOIN derivative d ON d.content_id = c.id AND d.kind = 'text'
             LEFT JOIN knowledge_extraction k ON k.content_id = c.id
             WHERE c.state = 'processed' AND coalesce(c.word_count, 0) >= ?1 AND (?2 IS NULL OR c.id = ?2) AND {condition}
               AND {active}
             ORDER BY c.id",
            active = super::changes::active_content("c.id")
        ))
        .map_err(sql_err)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(values.iter()), |row| {
            let content_id: i64 = row.get(0)?;
            let text_at: i64 = row.get(1)?;
            let state: Option<String> = row.get(2)?;
            let reading = match state {
                Some(state) => {
                    let read_at: i64 = row.get(4)?;
                    let prompt: String = row.get(5)?;
                    let vocabulary_digest: String = row.get(6)?;
                    Some(ReadingRow {
                        state,
                        basis: row.get(3)?,
                        current: read_at == text_at && prompt == KNOWLEDGE_PROMPT && vocabulary_digest == vocabulary.reading_digest,
                        discarded: row.get(7)?,
                    })
                }
                None => None,
            };
            Ok(Standing { content_id, text_at, reading })
        })
        .map_err(sql_err)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(sql_err)
}

/// The parts a text is read in: whole pages, up to `PART_WORDS` words
/// each; a single page longer than that is cut by lines.
pub fn parts(body: &str) -> Vec<String> {
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim_start().starts_with(RECOGNIZED_MARKER_START)).collect();
    // Blocks: what comes before the first page heading, then one per page.
    let mut blocks: Vec<Vec<&str>> = vec![Vec::new()];
    for line in lines {
        if is_page_heading(line.trim()) && blocks.last().is_some_and(|b| !b.is_empty()) {
            blocks.push(Vec::new());
        }
        blocks.last_mut().expect("one").push(line);
    }
    let words = |lines: &[&str]| lines.iter().map(|l| l.split_whitespace().count()).sum::<usize>();
    let mut pieces: Vec<Vec<&str>> = Vec::new();
    for block in blocks {
        if words(&block) <= PART_WORDS {
            pieces.push(block);
            continue;
        }
        let mut current: Vec<&str> = Vec::new();
        for line in block {
            if !current.is_empty() && words(&current) + line.split_whitespace().count() > PART_WORDS {
                pieces.push(std::mem::take(&mut current));
            }
            current.push(line);
        }
        pieces.push(current);
    }
    let mut out: Vec<String> = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for piece in pieces {
        if !current.is_empty() && words(&current) + words(&piece) > PART_WORDS {
            out.push(current.join("\n").trim().to_string());
            current.clear();
        }
        current.extend(piece);
    }
    if !current.is_empty() || out.is_empty() {
        out.push(current.join("\n").trim().to_string());
    }
    out.retain(|p| !p.is_empty());
    out
}

/// The identifiers the base recognizes by their shape, without a model.
pub fn identifiers_by_shape(body: &str) -> Vec<String> {
    let prepared = Prepared::new(&pages(body), &[], &[]);
    let mut out: Vec<String> = Vec::new();
    for raw in &prepared.raws {
        if kind_of_token(raw) == Kind::Identifier && !out.contains(&raw.text) {
            out.push(raw.text.clone());
            if out.len() >= MAX_SHAPE_HINTS {
                break;
            }
        }
    }
    out
}

/// What the base knows of a document before the model reads it.
#[derive(Debug, Clone, Default)]
pub struct Hints {
    pub name: String,
    pub doc_type: Option<String>,
    pub stage: Option<String>,
    pub data: Vec<Datum>,
    pub shapes: Vec<String>,
}

fn hints(conn: &Connection, content_id: i64, body: &str) -> Result<Hints, String> {
    let (doc_type, stage): (Option<String>, Option<String>) = conn
        .query_row("SELECT doc_type, stage FROM document_type WHERE content_id = ?1", [content_id], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(sql_err)?
        .unwrap_or((None, None));
    let data: Vec<Datum> = conn
        .query_row("SELECT data FROM document_data WHERE content_id = ?1", [content_id], |row| row.get::<_, String>(0))
        .optional()
        .map_err(sql_err)?
        .and_then(|d| serde_json::from_str(&d).ok())
        .unwrap_or_default();
    Ok(Hints { name: classify::name_of(conn, content_id)?, doc_type, stage, data, shapes: identifiers_by_shape(body) })
}

fn list(terms: impl Iterator<Item = (String, String)>) -> String {
    terms.map(|(id, description)| format!("- {id}: {}", description.trim())).collect::<Vec<_>>().join("\n")
}

fn with_types(description: &str, types: &[String]) -> String {
    if types.is_empty() {
        description.trim().to_string()
    } else {
        format!("{} (para: {})", description.trim(), types.join(", "))
    }
}

/// The question about one part of a document.
pub fn prompt(vocabulary: &Vocabulary, hints: &Hints, part: &str, index: usize, total: usize) -> String {
    let types = list(vocabulary.entity_types.iter().map(|t| (t.id.clone(), t.description.clone())));
    let roles = list(vocabulary.roles.iter().map(|t| (t.id.clone(), with_types(&t.description, &t.types))));
    let identifiers = list(vocabulary.identifiers.iter().map(|t| (t.id.clone(), with_types(&t.description, &t.types))));
    let predicates = list(vocabulary.predicates.iter().map(|p| {
        let object = if p.takes_entity() {
            format!("el objeto es otra mención del documento{}", if p.object_types.is_empty() { String::new() } else { format!(" ({})", p.object_types.join(", ")) })
        } else {
            "el objeto es un valor copiado del texto".to_string()
        };
        (p.id.clone(), format!("{} Sujeto: {}; {object}.", p.description.trim(), if p.subject.is_empty() { "cualquiera".to_string() } else { p.subject.join(", ") }))
    }));
    let mut known = Vec::new();
    if let Some(doc_type) = &hints.doc_type {
        known.push(format!(
            "tipo de documento: {doc_type}{}",
            hints.stage.as_ref().map(|s| format!(", etapa: {s}")).unwrap_or_default()
        ));
    }
    if !hints.data.is_empty() {
        let data: Vec<String> = hints.data.iter().map(|d| format!("{}: {}", d.field, d.value)).collect();
        known.push(format!("datos del documento: {}", data.join("; ")));
    }
    if !hints.shapes.is_empty() {
        known.push(format!("números con forma de identificador: {}", hints.shapes.join(", ")));
    }
    let known = if known.is_empty() { "nada".to_string() } else { known.join("\n- ") };
    let part_note = if total > 1 {
        format!(" Es la parte {} de {total} del documento: respondé solo por lo que está en esta parte.", index + 1)
    } else {
        String::new()
    };
    format!(
        "Sos asistente de un estudio jurídico. Leé el documento de abajo y registrá las personas, organizaciones y asuntos \
que nombra, con qué papel aparece cada uno, y lo que el documento dice de ellos.{part_note}\n\n\
Reglas:\n\
- Solo lo que dice este documento. No agregues nada que sepas por otro lado ni completes datos que el texto no trae.\n\
- Cada cita es un fragmento copiado literalmente del texto, continuo, de no más de 40 palabras, sin puntos suspensivos ni cambios. \
Se va a comprobar que esté en el texto: lo que no esté, se descarta.\n\
- Una mención por cada forma en que el documento escribe a una entidad y por cada papel que tiene. \"escrito\" es el nombre \
tal como aparece, y su cita lo contiene. Si dos menciones de este documento son la misma entidad (\"el locatario\" y su nombre, \
la sigla y la razón social), en \"misma_que\" de la segunda poné la \"ref\" de la primera; si no, null. No registres un pronombre \
ni un papel sin nombre (\"el locatario\") si el documento no dice quién es.\n\
- Los identificadores (DNI, CUIT, matrícula, número de expediente) van en la mención a la que el texto los asocia, con el valor \
tal como está escrito y una cita que lo contiene.\n\
- Las afirmaciones dicen algo del sujeto (la \"ref\" de una mención) con un predicado de la lista. Si el predicado lleva otra \
mención como objeto, va en \"objeto_ref\" y \"objeto_valor\" es null; si lleva un valor, va en \"objeto_valor\" tal como está \
escrito, dentro de la cita, y \"objeto_ref\" es null. \"tiempo\" es la fecha o el período del hecho si el documento lo dice, \
tal como está escrito; si no, null.\n\
- \"pagina\" es el número de la página (los encabezados \"## Page N\" del texto) donde está la cita, o null si el texto no tiene páginas.\n\
- La sección \"### Form fields\", si está, trae los campos completados de un formulario: son parte del texto.\n\
- El nombre del archivo es una pista, no una prueba.\n\n\
Tipos de entidad:\n{types}\n\nPapeles (\"rol\"):\n{roles}\n\nIdentificadores (\"clase\"):\n{identifiers}\n\n\
Predicados:\n{predicates}\n\n\
Lo que ya se sabe del documento sin leerlo:\n- {known}\n\n\
Archivo: «{}»\n\nTexto del documento:\n\"\"\"\n{part}\n\"\"\"\n\n\
Respondé solo con el JSON pedido.",
        hints.name
    )
}

/// The shape of the answer, with the closed lists of the vocabulary.
pub fn schema(vocabulary: &Vocabulary) -> serde_json::Value {
    let ids = |terms: Vec<String>| serde_json::json!({ "type": "string", "enum": terms });
    let types = ids(vocabulary.entity_types.iter().map(|t| t.id.clone()).collect());
    let roles = ids(vocabulary.roles.iter().map(|t| t.id.clone()).collect());
    let identifiers = ids(vocabulary.identifiers.iter().map(|t| t.id.clone()).collect());
    let predicates = ids(vocabulary.predicates.iter().map(|p| p.id.clone()).collect());
    let text = serde_json::json!({ "type": "string" });
    let page = serde_json::json!({ "type": ["integer", "null"] });
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["menciones", "afirmaciones"],
        "properties": {
            "menciones": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["ref", "misma_que", "tipo", "escrito", "rol", "cita", "pagina", "identificadores"],
                    "properties": {
                        "ref": text,
                        "misma_que": { "type": ["string", "null"] },
                        "tipo": types,
                        "escrito": text,
                        "rol": roles,
                        "cita": text,
                        "pagina": page,
                        "identificadores": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "additionalProperties": false,
                                "required": ["clase", "valor", "cita"],
                                "properties": { "clase": identifiers, "valor": text, "cita": text }
                            }
                        }
                    }
                }
            },
            "afirmaciones": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["sujeto", "predicado", "objeto_ref", "objeto_valor", "cita", "pagina", "tiempo"],
                    "properties": {
                        "sujeto": text,
                        "predicado": predicates,
                        "objeto_ref": { "type": ["string", "null"] },
                        "objeto_valor": { "type": ["string", "null"] },
                        "cita": text,
                        "pagina": page,
                        "tiempo": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

#[derive(Debug, Clone, Deserialize)]
pub struct Answer {
    #[serde(default)]
    pub menciones: Vec<AnswerMention>,
    #[serde(default)]
    pub afirmaciones: Vec<AnswerClaim>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnswerMention {
    #[serde(rename = "ref")]
    pub reference: String,
    #[serde(default)]
    pub misma_que: Option<String>,
    pub tipo: String,
    pub escrito: String,
    pub rol: String,
    pub cita: String,
    #[serde(default)]
    pub pagina: Option<i64>,
    #[serde(default)]
    pub identificadores: Vec<AnswerIdentifier>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnswerIdentifier {
    pub clase: String,
    pub valor: String,
    pub cita: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnswerClaim {
    pub sujeto: String,
    pub predicado: String,
    #[serde(default)]
    pub objeto_ref: Option<String>,
    #[serde(default)]
    pub objeto_valor: Option<String>,
    pub cita: String,
    #[serde(default)]
    pub pagina: Option<i64>,
    #[serde(default)]
    pub tiempo: Option<String>,
}

/// A mention that passed the check.
#[derive(Debug, Clone, PartialEq)]
pub struct Mention {
    pub reference: String,
    pub same_as: Option<String>,
    pub kind: String,
    pub written: String,
    pub normalized: String,
    pub role: String,
    pub quote: String,
    pub page: Option<i64>,
    pub identifiers: Vec<Identifier>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub kind: String,
    pub value: String,
    pub normalized: String,
    pub quote: String,
    pub page: Option<i64>,
    pub by_shape: bool,
}

/// A claim that passed the check, between mentions of the document.
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    pub subject: String,
    pub predicate: String,
    pub object_mention: Option<String>,
    pub object_value: Option<String>,
    pub quote: String,
    pub page: Option<i64>,
    pub fact_time: Option<String>,
}

/// What is kept of the answers about one document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reading {
    pub mentions: Vec<Mention>,
    pub evidence: Vec<Evidence>,
    pub discarded: Vec<Discarded>,
}

fn discard(reading: &mut Reading, what: &str, reason: &str, said: &str, quote: &str) {
    reading.discarded.push(Discarded { what: what.into(), reason: reason.into(), said: said.trim().into(), quote: quote.trim().into() });
}

fn normalized(text: &str) -> String {
    quote::words(text).join(" ")
}

fn applies(types: &[String], kind: &str) -> bool {
    types.is_empty() || types.iter().any(|t| t == kind)
}

/// Checks the answers about the parts of one document against its text,
/// and puts them together: the mentions of every part, the same mention
/// (type, name and role) once, and each claim once.
pub fn verify(vocabulary: &Vocabulary, body: &str, shapes: &[String], answers: &[Answer]) -> Reading {
    let text = Located::new(body);
    let shapes: BTreeSet<String> = shapes.iter().map(|s| normalized(s)).collect();
    let mut reading = Reading::default();
    // The model's ref in each part, to the kept mention's ref.
    let mut refs: HashMap<(usize, String), String> = HashMap::new();
    let mut by_key: HashMap<(String, String, String), usize> = HashMap::new();
    for (part, answer) in answers.iter().enumerate() {
        for said in &answer.menciones {
            let Some(kind) = vocabulary.entity_type(&said.tipo).map(|t| t.id.clone()) else {
                discard(&mut reading, "mention", "tipo-desconocido", &said.escrito, &said.cita);
                continue;
            };
            let written = said.escrito.trim().to_string();
            let name = quote::words(&written);
            let Some(page) = text.locate(&said.cita, said.pagina) else {
                discard(&mut reading, "mention", "cita-no-encontrada", &written, &said.cita);
                continue;
            };
            if !quote::contains(&quote::words(&said.cita), &name) {
                discard(&mut reading, "mention", "nombre-fuera-de-la-cita", &written, &said.cita);
                continue;
            }
            let role = vocabulary
                .role(&said.rol)
                .filter(|r| applies(&r.types, &kind))
                .map(|r| r.id.clone())
                .unwrap_or_else(|| vocabulary.fallback_role().to_string());
            let mut identifiers = Vec::new();
            for identifier in &said.identificadores {
                let value = identifier.valor.trim().to_string();
                let value_words = quote::words(&value);
                let Some(id_page) = text.locate(&identifier.cita, said.pagina) else {
                    discard(&mut reading, "identifier", "cita-no-encontrada", &value, &identifier.cita);
                    continue;
                };
                if !quote::contains(&quote::words(&identifier.cita), &value_words) {
                    discard(&mut reading, "identifier", "valor-fuera-de-la-cita", &value, &identifier.cita);
                    continue;
                }
                let kind = vocabulary
                    .identifier(&identifier.clase)
                    .map(|t| t.id.clone())
                    .or_else(|| vocabulary.identifiers.last().map(|t| t.id.clone()))
                    .unwrap_or_default();
                let normalized_value = value_words.join(" ");
                if !identifiers.iter().any(|i: &Identifier| i.kind == kind && i.normalized == normalized_value) {
                    identifiers.push(Identifier {
                        kind,
                        by_shape: shapes.contains(&normalized_value),
                        value,
                        normalized: normalized_value,
                        quote: identifier.cita.trim().to_string(),
                        page: id_page,
                    });
                }
            }
            let key = (kind.clone(), name.join(" "), role.clone());
            if let Some(&kept) = by_key.get(&key) {
                // The same mention again (another part, or said twice).
                let mention = &mut reading.mentions[kept];
                for identifier in identifiers {
                    if !mention.identifiers.iter().any(|i| i.kind == identifier.kind && i.normalized == identifier.normalized) {
                        mention.identifiers.push(identifier);
                    }
                }
                refs.insert((part, said.reference.clone()), mention.reference.clone());
                continue;
            }
            let reference = format!("m{}", reading.mentions.len() + 1);
            refs.insert((part, said.reference.clone()), reference.clone());
            by_key.insert(key, reading.mentions.len());
            reading.mentions.push(Mention {
                reference,
                same_as: said.misma_que.clone().filter(|s| !s.trim().is_empty()).map(|s| format!("{part}\u{0}{s}")),
                kind,
                normalized: name.join(" "),
                written,
                role,
                quote: said.cita.trim().to_string(),
                page,
                identifiers,
            });
        }
    }
    // "same as" points to a mention of the same part, once all are known.
    for mention in &mut reading.mentions {
        mention.same_as = mention.same_as.take().and_then(|s| {
            let (part, said) = s.split_once('\u{0}')?;
            refs.get(&(part.parse().ok()?, said.to_string())).cloned()
        });
        if mention.same_as.as_deref() == Some(mention.reference.as_str()) {
            mention.same_as = None;
        }
    }
    let kinds: HashMap<String, String> = reading.mentions.iter().map(|m| (m.reference.clone(), m.kind.clone())).collect();
    for (part, answer) in answers.iter().enumerate() {
        for said in &answer.afirmaciones {
            let Some(predicate) = vocabulary.predicate(&said.predicado) else {
                discard(&mut reading, "claim", "predicado-desconocido", &said.predicado, &said.cita);
                continue;
            };
            let Some(subject) = refs.get(&(part, said.sujeto.clone())).cloned() else {
                discard(&mut reading, "claim", "sujeto-desconocido", &said.predicado, &said.cita);
                continue;
            };
            if !applies(&predicate.subject, &kinds[&subject]) {
                discard(&mut reading, "claim", "no-corresponde-al-tipo", &said.predicado, &said.cita);
                continue;
            }
            let Some(page) = text.locate(&said.cita, said.pagina) else {
                discard(&mut reading, "claim", "cita-no-encontrada", &said.predicado, &said.cita);
                continue;
            };
            let quote_words = quote::words(&said.cita);
            let (object_mention, object_value) = if predicate.takes_entity() {
                match said.objeto_ref.as_ref().and_then(|r| refs.get(&(part, r.clone()))) {
                    Some(object) if applies(&predicate.object_types, &kinds[object]) && object != &subject => (Some(object.clone()), None),
                    _ => {
                        discard(&mut reading, "claim", "objeto-desconocido", &said.predicado, &said.cita);
                        continue;
                    }
                }
            } else {
                let value = said.objeto_valor.clone().unwrap_or_default().trim().to_string();
                if value.is_empty() || !quote::contains(&quote_words, &quote::words(&value)) {
                    discard(&mut reading, "claim", "valor-fuera-de-la-cita", &format!("{}: {value}", said.predicado), &said.cita);
                    continue;
                }
                (None, Some(value))
            };
            // A time is kept only when it is in the quote too.
            let fact_time = said
                .tiempo
                .clone()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty() && quote::contains(&quote_words, &quote::words(t)));
            let evidence = Evidence {
                subject,
                predicate: predicate.id.clone(),
                object_mention,
                object_value,
                quote: said.cita.trim().to_string(),
                page,
                fact_time,
            };
            let same = |e: &Evidence| {
                e.subject == evidence.subject
                    && e.predicate == evidence.predicate
                    && e.object_mention == evidence.object_mention
                    && e.object_value.as_deref().map(normalized) == evidence.object_value.as_deref().map(normalized)
            };
            if !reading.evidence.iter().any(same) {
                reading.evidence.push(evidence);
            }
        }
    }
    // A part of an address read as a number that identifies someone (a
    // suite, a postal code, a floor): its value is inside a domicile the
    // same reading gives, or the word before it in its quote is a word of
    // an address. It would keep apart the same company in two documents.
    let addresses: Vec<Vec<String>> = reading
        .evidence
        .iter()
        .filter(|e| vocabulary.predicate(&e.predicate).is_some_and(|p| p.shape == "address"))
        .filter_map(|e| e.object_value.as_deref())
        .map(quote::words)
        .collect();
    let address_words = &vocabulary.identity.address_words;
    let mut dropped: Vec<(String, String)> = Vec::new();
    for mention in &mut reading.mentions {
        mention.identifiers.retain(|identifier| {
            let value = quote::words(&identifier.value);
            let quote_words = quote::words(&identifier.quote);
            let in_address = addresses.iter().any(|a| quote::contains(a, &value));
            let after_address_word = quote_words
                .windows(value.len() + 1)
                .any(|w| w[1..] == value[..] && address_words.iter().any(|a| a == &w[0]));
            if in_address || after_address_word {
                dropped.push((identifier.value.clone(), identifier.quote.clone()));
                false
            } else {
                true
            }
        });
    }
    for (value, quote) in dropped {
        discard(&mut reading, "identifier", "parte-de-un-domicilio", &value, &quote);
    }
    // A person's number that, everywhere the text has it, comes right
    // after the name of another person of the same reading and not after
    // hers (a form that lists a spouse under the client) is the other
    // person's: it would keep the same person apart.
    let body_words = quote::words(body);
    let people: Vec<(String, Vec<String>)> = reading
        .mentions
        .iter()
        .filter(|m| m.kind == PERSON_KIND)
        .map(|m| (m.reference.clone(), quote::words(&m.written)))
        .filter(|(_, w)| w.len() >= 2)
        .collect();
    let mut dropped: Vec<(String, String)> = Vec::new();
    for mention in reading.mentions.iter_mut().filter(|m| m.kind == PERSON_KIND) {
        let own = quote::words(&mention.written);
        let reference = mention.reference.clone();
        mention.identifiers.retain(|identifier| {
            let value = quote::words(&identifier.value);
            if value.is_empty() {
                return true;
            }
            let places: Vec<usize> = (0..body_words.len().saturating_sub(value.len() - 1))
                .filter(|&i| body_words[i..i + value.len()] == value[..])
                .collect();
            let anothers = |i: usize| {
                let before = &body_words[i.saturating_sub(OTHER_PERSON_WINDOW)..i];
                !quote::contains(before, &own)
                    && people.iter().any(|(r, name)| *r != reference && *name != own && quote::contains(before, name))
            };
            let other = !places.is_empty() && places.iter().all(|&i| anothers(i));
            if other {
                dropped.push((identifier.value.clone(), identifier.quote.clone()));
            }
            !other
        });
    }
    for (value, quote) in dropped {
        discard(&mut reading, "identifier", "de-otra-persona", &value, &quote);
    }
    reading
}

/// A document to read, with its questions.
struct Pending {
    content_id: i64,
    basis: String,
    text_hash: String,
    text_at: i64,
    body: String,
}

/// The questions about a document, one per part, and the identifiers the
/// base recognized in it by their shape.
fn questions(conn: &Connection, vocabulary: &Vocabulary, content_id: i64, body: &str) -> Result<(Vec<Question>, Vec<String>), String> {
    let hints = hints(conn, content_id, body)?;
    let parts = parts(body);
    let total = parts.len();
    let schema = schema(vocabulary);
    let questions = parts
        .iter()
        .enumerate()
        .map(|(index, part)| {
            let prompt = prompt(vocabulary, &hints, part, index, total);
            Question {
                key: classify::question_key(KNOWLEDGE_PROMPT, &prompt),
                version: KNOWLEDGE_PROMPT,
                task: TASK,
                content_id,
                prompt,
                schema: schema.clone(),
            }
        })
        .collect();
    Ok((questions, hints.shapes))
}

/// Writes the reading of a document, replacing the one before.
fn write_reading(
    conn: &mut Connection,
    pending: &Pending,
    questions: &[Question],
    engine: &str,
    reading: Result<&Reading, &str>,
) -> Result<(), String> {
    let vocabulary = vocabulary();
    let tx = conn.transaction().map_err(sql_err)?;
    tx.execute("DELETE FROM knowledge_mention WHERE content_id = ?1", [pending.content_id]).map_err(sql_err)?;
    tx.execute("DELETE FROM knowledge_evidence WHERE content_id = ?1", [pending.content_id]).map_err(sql_err)?;
    let keys: Vec<&str> = questions.iter().map(|q| q.key.as_str()).collect();
    let (state, mentions, evidence, discarded, error) = match reading {
        Ok(reading) => {
            let mut ids: HashMap<&str, i64> = HashMap::new();
            for mention in &reading.mentions {
                tx.execute(
                    "INSERT INTO knowledge_mention (content_id, ref, same_as, kind, written, normalized, role, quote, page)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        pending.content_id,
                        mention.reference,
                        mention.same_as,
                        mention.kind,
                        mention.written,
                        mention.normalized,
                        mention.role,
                        mention.quote,
                        mention.page
                    ],
                )
                .map_err(sql_err)?;
                let id = tx.last_insert_rowid();
                ids.insert(mention.reference.as_str(), id);
                for identifier in &mention.identifiers {
                    tx.execute(
                        "INSERT OR IGNORE INTO knowledge_identifier (mention_id, kind, value, normalized, quote, page, by_shape)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![id, identifier.kind, identifier.value, identifier.normalized, identifier.quote, identifier.page, identifier.by_shape],
                    )
                    .map_err(sql_err)?;
                }
            }
            for item in &reading.evidence {
                tx.execute(
                    "INSERT INTO knowledge_evidence
                        (content_id, subject_mention, predicate, object_mention, object_value, object_normalized, quote, page, fact_time)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                    params![
                        pending.content_id,
                        ids[item.subject.as_str()],
                        item.predicate,
                        item.object_mention.as_deref().map(|m| ids[m]),
                        item.object_value,
                        item.object_value.as_deref().map(normalized),
                        item.quote,
                        item.page,
                        item.fact_time
                    ],
                )
                .map_err(sql_err)?;
            }
            (
                "done",
                reading.mentions.len(),
                reading.evidence.len(),
                serde_json::to_string(&reading.discarded).map_err(|e| e.to_string())?,
                None,
            )
        }
        Err(error) => ("failed", 0, 0, "[]".to_string(), Some(error.to_string())),
    };
    tx.execute(
        "INSERT INTO knowledge_extraction
            (content_id, state, basis, prompt_version, vocabulary, text_hash, text_at, engine, questions, parts, mentions,
             evidence, discarded, error, extracted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
         ON CONFLICT (content_id) DO UPDATE SET
            state = excluded.state, basis = excluded.basis, prompt_version = excluded.prompt_version,
            vocabulary = excluded.vocabulary, text_hash = excluded.text_hash, text_at = excluded.text_at, engine = excluded.engine,
            questions = excluded.questions, parts = excluded.parts, mentions = excluded.mentions,
            evidence = excluded.evidence, discarded = excluded.discarded, error = excluded.error,
            extracted_at = excluded.extracted_at",
        params![
            pending.content_id,
            state,
            pending.basis,
            KNOWLEDGE_PROMPT,
            vocabulary.reading_digest,
            pending.text_hash,
            pending.text_at,
            engine,
            serde_json::to_string(&keys).map_err(|e| e.to_string())?,
            questions.len() as i64,
            mentions as i64,
            evidence as i64,
            discarded,
            error,
            now_ms()
        ],
    )
    .map_err(sql_err)?;
    // Its mentions changed: the resolution of identities looks at it again.
    mark_dirty(&tx, "resolve", &pending.content_id.to_string())?;
    tx.commit().map_err(sql_err)
}

impl DocBase {
    /// The documents still to read: never read, or read on another basis.
    /// Those whose type is still to be asked are left out and counted.
    fn knowledge_pending(&self) -> Result<(Vec<Pending>, usize), String> {
        let stale: Vec<Standing> = standings(&self.conn(), &Visible::All, None)?
            .into_iter()
            .filter(|s| !s.reading.as_ref().is_some_and(|r| r.current))
            .collect();
        if stale.is_empty() {
            return Ok((Vec::new(), 0));
        }
        let waiting = self.contents_waiting_for_type()?;
        let vocabulary = vocabulary();
        let conn = self.conn();
        let mut pending = Vec::new();
        let mut waiting_count = 0;
        for standing in stale {
            let body: String = conn
                .query_row("SELECT body FROM derivative WHERE content_id = ?1 AND kind = 'text'", [standing.content_id], |row| row.get(0))
                .map_err(sql_err)?;
            let hash = text_hash(&body);
            let basis = basis(vocabulary, &hash);
            if standing.reading.as_ref().is_some_and(|r| r.basis == basis) {
                // The same text written again: the reading stands.
                conn.execute(
                    "UPDATE knowledge_extraction SET text_at = ?1, prompt_version = ?2, vocabulary = ?3 WHERE content_id = ?4",
                    params![standing.text_at, KNOWLEDGE_PROMPT, vocabulary.reading_digest, standing.content_id],
                )
                .map_err(sql_err)?;
                continue;
            }
            if waiting.contains(&standing.content_id) {
                waiting_count += 1;
                continue;
            }
            pending.push(Pending { content_id: standing.content_id, basis, text_hash: hash, text_at: standing.text_at, body });
        }
        Ok((pending, waiting_count))
    }

    /// The questions the pending documents take (for the evaluation, which
    /// warms its cache with them before the run).
    #[cfg(test)]
    pub(crate) fn knowledge_questions(&self) -> Result<Vec<Question>, String> {
        let (pending, _) = self.knowledge_pending()?;
        let conn = self.conn();
        let mut out = Vec::new();
        for item in pending {
            out.extend(questions(&conn, vocabulary(), item.content_id, &item.body)?.0);
        }
        Ok(out)
    }

    /// What a run would do: how many documents are still to read, how
    /// many questions that takes (answers this engine already gave are not
    /// counted) and how many documents wait for their type.
    pub fn knowledge_plan(&self) -> Result<KnowledgePlan, String> {
        let engine = classify::ask_engine(&self.conn())?;
        let engine_id = engine.as_ref().map(crate::source_volume::asking::engine_id);
        let (pending, waiting) = self.knowledge_pending()?;
        let conn = self.conn();
        let mut count = 0;
        for item in &pending {
            for question in questions(&conn, vocabulary(), item.content_id, &item.body)?.0 {
                if cached_answer(&conn, &question.key, engine_id.as_deref())?.is_none() {
                    count += 1;
                }
            }
        }
        Ok(KnowledgePlan { documents: pending.len(), questions: count, waiting_for_type: waiting, engine })
    }

    /// Reads what is pending, asking the model at most `max_questions`
    /// questions, and writes what passes the check. One run asks at a time.
    pub fn knowledge_run(&self, asker: &dyn DocumentAsker, max_questions: usize) -> Result<KnowledgeReport, String> {
        let mut report = KnowledgeReport::default();
        let (pending, waiting) = self.knowledge_pending()?;
        report.waiting_for_type = waiting;
        let _one_at_a_time = ASKING.lock().unwrap_or_else(|p| p.into_inner());
        let vocabulary = vocabulary();
        let engine = asker.engine_id();
        let mut budget = max_questions;
        let mut stopped = false;
        for item in pending {
            if stopped {
                report.left += 1;
                continue;
            }
            let (item_questions, shapes) = questions(&self.conn(), vocabulary, item.content_id, &item.body)?;
            let mut answers: Vec<String> = Vec::new();
            for question in &item_questions {
                if let Some(answer) = cached_answer(&self.conn(), &question.key, Some(&engine))? {
                    answers.push(answer);
                    continue;
                }
                if budget == 0 || stopped {
                    break;
                }
                budget -= 1;
                report.asked += 1;
                match ask_and_keep(self, asker, question) {
                    Ok(answer) => answers.push(answer),
                    Err(error) => {
                        report.failed += 1;
                        stopped = stops_the_run(&error);
                        report.errors.push(format!("{error} ({})", item.content_id));
                        break;
                    }
                }
            }
            if answers.len() < item_questions.len() {
                report.left += 1;
                continue;
            }
            let parsed: Result<Vec<Answer>, String> = answers.iter().map(|a| parse_json::<Answer>(a)).collect();
            let mut conn = self.conn();
            match parsed {
                Ok(parsed) => {
                    let reading = verify(vocabulary, &item.body, &shapes, &parsed);
                    report.mentions += reading.mentions.len();
                    report.evidence += reading.evidence.len();
                    report.discarded += reading.discarded.len();
                    write_reading(&mut conn, &item, &item_questions, &engine, Ok(&reading))?;
                    report.extracted += 1;
                }
                Err(error) => {
                    write_reading(&mut conn, &item, &item_questions, &engine, Err(&error))?;
                    report.unusable += 1;
                    report.errors.push(format!("{error} ({})", item.content_id));
                }
            }
        }
        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_long_text_is_read_in_parts_of_whole_pages() {
        let page = |n: usize, words: usize| format!("## Page {n}\n\n{}\n", vec!["palabra"; words].join(" "));
        let body: String = (1..=5).map(|n| page(n, 1500)).collect();
        let read = parts(&body);
        assert_eq!(read.len(), 3, "two pages per part");
        assert!(read[0].starts_with("## Page 1") && read[0].contains("## Page 2") && !read[0].contains("## Page 3"));
        assert!(read[2].starts_with("## Page 5"));
        // A short text is one part; a single huge page is cut by lines.
        assert_eq!(parts("uno dos tres").len(), 1);
        let huge: String = format!("## Page 1\n{}", (0..9000).map(|i| format!("w{i}\n")).collect::<String>());
        let cut = parts(&huge);
        assert_eq!(cut.len(), 3);
        assert!(cut.iter().all(|p| p.split_whitespace().count() <= PART_WORDS + 4));
    }

    #[test]
    fn identifiers_are_recognized_by_their_shape() {
        let shapes = identifiers_by_shape("CUIT 30-11111111-1, DNI 22.222.222 y el expediente 1234567. Son 3 cosas.");
        assert!(shapes.contains(&"30-11111111-1".to_string()));
        assert!(shapes.contains(&"22.222.222".to_string()));
        assert!(shapes.contains(&"1234567".to_string()));
        assert!(!shapes.iter().any(|s| s == "3"));
    }

    #[test]
    fn the_question_has_the_closed_lists_and_the_hints() {
        let vocabulary = Vocabulary::builtin();
        let hints = Hints {
            name: "carta.pdf".into(),
            doc_type: Some("carta-documento".into()),
            stage: Some("enviada".into()),
            data: vec![Datum { field: "remitente".into(), value: "Ejemplo Uno".into(), source: "rule".into() }],
            shapes: vec!["30-11111111-1".into()],
        };
        let prompt = prompt(&vocabulary, &hints, "texto", 0, 1);
        for term in vocabulary.entity_types.iter().chain(&vocabulary.roles) {
            assert!(prompt.contains(&format!("- {}:", term.id)));
        }
        assert!(prompt.contains("tipo de documento: carta-documento, etapa: enviada"));
        assert!(prompt.contains("remitente: Ejemplo Uno"));
        assert!(prompt.contains("30-11111111-1"));
        assert!(prompt.contains("No agregues nada"));
        let schema = schema(&vocabulary);
        assert_eq!(schema["additionalProperties"], false);
        let roles = &schema["properties"]["menciones"]["items"]["properties"]["rol"]["enum"];
        assert_eq!(roles.as_array().unwrap().len(), vocabulary.roles.len());
    }
}
