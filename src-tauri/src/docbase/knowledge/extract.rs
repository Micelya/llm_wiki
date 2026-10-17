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
use super::{names, read_checks, values};
use super::vocabulary::{vocabulary, Vocabulary};
use super::{mark_dirty, visible_content, Discarded};
use crate::docbase::similar::changes::{classify as kind_of_token, Kind};
use crate::docbase::similar::classify::{
    self as classify, cached_answer, parse_json, stops_the_run, Question, ASKING, MIN_TYPE_WORDS,
};
use crate::docbase::similar::data::Datum;
use crate::docbase::similar::text::{pages, Prepared};
use crate::docbase::{now_ms, sql_err, DocBase, Visible};
use crate::source_volume::asking::DocumentAsker;
use crate::source_volume::catalog::{is_page_heading, RECOGNIZED_MARKER_START};

/// Version of the question. Changing the prompt changes it, and every
/// document is read again.
pub const KNOWLEDGE_PROMPT: &str = "conocimiento-2";
/// The task the answers are kept under in `model_answer`.
pub const TASK: &str = "knowledge";
/// Words of text in one question; a longer document is read in parts of
/// whole pages.
pub const PART_WORDS: usize = 4000;

/// The type of entity of a person (`knowledge.json`), and how many words
/// before a number are looked at to tell whose it is.
const PERSON_KIND: &str = "persona";
const ORGANIZATION_KIND: &str = "organizacion";
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
    let predicates = list(vocabulary.predicates.iter().filter(|p| !p.is_money()).map(|p| {
        let object = if p.takes_entity() {
            format!("el objeto es otra mención del documento{}", if p.object_types.is_empty() { String::new() } else { format!(" ({})", p.object_types.join(", ")) })
        } else {
            "el objeto es un valor copiado del texto".to_string()
        };
        let classes: Vec<String> = p.classes().map(|q| format!("{} ({})", q.id, q.description.trim())).collect();
        let classes = if classes.is_empty() { String::new() } else { format!(" Clases: {}.", classes.join("; ")) };
        (p.id.clone(), format!("{} Sujeto: {}; {object}.{classes}", p.description.trim(), if p.subject.is_empty() { "cualquiera".to_string() } else { p.subject.join(", ") }))
    }));
    let money = vocabulary.money_predicate().map(|p| p.description.trim().to_string()).unwrap_or_default();
    let kinds = list(vocabulary.money.kinds.iter().map(|k| (k.id.clone(), k.description.clone())));
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
- Los términos definidos (\"en adelante «la Empresa»\", \"(«la Locataria»)\", \"hereinafter the «Company»\") no son nombres: \
van en \"definiciones\", con la \"ref\" de la mención con nombre propio que definen y una cita que tiene el término y ese nombre. \
Si además registrás el término como mención, en \"misma_que\" poné la \"ref\" de lo que define. Un rótulo de formulario \
(\"PROVINCIA:\", \"Nombre:\") no es una mención: lo es el valor que lo sigue.\n\
- Los identificadores van en la mención a la que el texto los asocia, con el valor tal como está escrito y una cita que lo \
contiene. Su \"clase\" es la que dice el rótulo que el texto le pone (un número rotulado EIN es EIN aunque otro texto lo llame CUIT). \
Un número de factura, de carta documento o envío postal, de sobre de firma o un código postal no identifica a una persona ni a \
una organización: si el documento trata de un asunto, van en la mención del asunto; si no, no van.\n\
- Las afirmaciones dicen algo del sujeto (la \"ref\" de una mención) con un predicado de la lista. Si el predicado lleva otra \
mención como objeto, va en \"objeto_ref\" y \"objeto_valor\" es null; si lleva un valor, va en \"objeto_valor\" tal como está \
escrito, dentro de la cita, y \"objeto_ref\" es null. \"tiempo\" es la fecha o el período del hecho si el documento lo dice, \
tal como está escrito; si no, null. \"clase\" es la clase del dato cuando el predicado tiene clases y el texto la dice; si no, \"ninguna\".\n\
- Una relación con consecuencias jurídicas (representa a, socio de, beneficiario final de, cónyuge de, parte en) se registra solo \
si la cita nombra a los dos: quién y de quién.\n\
- El dinero no va en \"afirmaciones\" sino en \"importes\": {money} \"deudor\" y \"acreedor\" son \"ref\" de menciones de personas \
u organizaciones de este documento, ya resueltos los términos definidos (si el texto dice «la Empresa adeuda a la Locadora», el \
deudor es la mención que el documento definió como «la Empresa»). Si el texto no dice una de las dos partes, el importe no va. \
\"importe\" copiado del texto, dentro de la cita; \"moneda\" y \"concepto\" como los dice la cita, o null; \"fecha\" del pago, \
de la deuda o del reclamo si la cita la dice, o null. Un impuesto, una percepción o un subtotal no son un importe entre partes.\n\
- En un formulario cada valor va con su campo y con la persona de su rótulo: lo que está bajo \"cónyuge\" es del cónyuge, no de \
quien completa el formulario. Si el rótulo y el valor están separados en el texto, \"cita\" tiene el valor y \"rotulo\" el rótulo \
copiado del texto; si no, \"rotulo\" es null.\n\
- Las páginas de la constancia de un servicio de firma electrónica (resumen del sobre, eventos de firma) no dicen quién es parte \
del acto: allí, quien aparece es \"firmante\" u \"otro\".\n\
- \"fecha_documento\" es la fecha del documento (de su firma, emisión o envío) tal como está escrita, o null.\n\
- \"pagina\" es el número de la página (los encabezados \"## Page N\" del texto) donde está la cita, o null si el texto no tiene páginas.\n\
- La sección \"### Form fields\", si está, trae los campos completados de un formulario: son parte del texto.\n\
- El nombre del archivo es una pista, no una prueba.\n\n\
Tipos de entidad:\n{types}\n\nPapeles (\"rol\"):\n{roles}\n\nIdentificadores (\"clase\"):\n{identifiers}\n\n\
Predicados:\n{predicates}\n\nTipos de importe (\"tipo\"):\n{kinds}\n\n\
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
    let predicates = ids(vocabulary.predicates.iter().filter(|p| !p.is_money()).map(|p| p.id.clone()).collect());
    let mut classes: Vec<String> = vec![NO_CLASS.to_string()];
    for class in vocabulary.predicates.iter().flat_map(|p| p.classes()) {
        if !classes.contains(&class.id) {
            classes.push(class.id.clone());
        }
    }
    let classes = ids(classes);
    let kinds = ids(vocabulary.money.kinds.iter().map(|k| k.id.clone()).collect());
    let text = serde_json::json!({ "type": "string" });
    let maybe = serde_json::json!({ "type": ["string", "null"] });
    let page = serde_json::json!({ "type": ["integer", "null"] });
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["fecha_documento", "menciones", "definiciones", "afirmaciones", "importes"],
        "properties": {
            "fecha_documento": maybe,
            "definiciones": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["termino", "ref", "cita"],
                    "properties": { "termino": text, "ref": text, "cita": text }
                }
            },
            "importes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["deudor", "acreedor", "importe", "moneda", "tipo", "concepto", "fecha", "cita", "pagina"],
                    "properties": {
                        "deudor": maybe,
                        "acreedor": maybe,
                        "importe": text,
                        "moneda": maybe,
                        "tipo": kinds,
                        "concepto": maybe,
                        "fecha": maybe,
                        "cita": text,
                        "pagina": page
                    }
                }
            },
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
                                "required": ["clase", "valor", "cita", "rotulo"],
                                "properties": { "clase": identifiers, "valor": text, "cita": text, "rotulo": maybe }
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
                    "required": ["sujeto", "predicado", "objeto_ref", "objeto_valor", "clase", "cita", "rotulo", "pagina", "tiempo"],
                    "properties": {
                        "sujeto": text,
                        "predicado": predicates,
                        "objeto_ref": { "type": ["string", "null"] },
                        "objeto_valor": { "type": ["string", "null"] },
                        "clase": classes,
                        "cita": text,
                        "rotulo": maybe,
                        "pagina": page,
                        "tiempo": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

/// The class of a claim when the text says none.
pub const NO_CLASS: &str = "ninguna";

#[derive(Debug, Clone, Deserialize)]
pub struct Answer {
    #[serde(default)]
    pub fecha_documento: Option<String>,
    #[serde(default)]
    pub menciones: Vec<AnswerMention>,
    #[serde(default)]
    pub definiciones: Vec<AnswerDefinition>,
    #[serde(default)]
    pub afirmaciones: Vec<AnswerClaim>,
    #[serde(default)]
    pub importes: Vec<AnswerAmount>,
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
    #[serde(default)]
    pub rotulo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnswerDefinition {
    pub termino: String,
    #[serde(rename = "ref")]
    pub reference: String,
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
    #[serde(default)]
    pub clase: Option<String>,
    pub cita: String,
    #[serde(default)]
    pub rotulo: Option<String>,
    #[serde(default)]
    pub pagina: Option<i64>,
    #[serde(default)]
    pub tiempo: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnswerAmount {
    #[serde(default)]
    pub deudor: Option<String>,
    #[serde(default)]
    pub acreedor: Option<String>,
    pub importe: String,
    #[serde(default)]
    pub moneda: Option<String>,
    #[serde(default)]
    pub tipo: String,
    #[serde(default)]
    pub concepto: Option<String>,
    #[serde(default)]
    pub fecha: Option<String>,
    pub cita: String,
    #[serde(default)]
    pub pagina: Option<i64>,
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
    /// A term the document defines ("la Empresa"): a local alias of the
    /// entity it defines (D-245).
    pub defined: bool,
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
    /// The amount of an amount between two parties, as written (D-244).
    pub amount: Option<String>,
    /// What else the reading says of it, as JSON: the kind, currency and
    /// concept of an amount; the class of a value ("real", "sede").
    pub detail: Option<serde_json::Value>,
}

/// What is kept of the answers about one document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reading {
    pub mentions: Vec<Mention>,
    pub evidence: Vec<Evidence>,
    pub discarded: Vec<Discarded>,
    /// The date of the document as its text writes it (D-247).
    pub doc_date: Option<String>,
}

fn discard(reading: &mut Reading, what: &str, reason: &str, said: &str, quote: &str) {
    reading.discarded.push(Discarded { what: what.into(), reason: reason.into(), said: said.trim().into(), quote: quote.trim().into() });
}

/// What the program corrected of a reading (kept, not thrown away): the
/// discards list keeps it with `what` = "correction" so that it is counted.
fn corrected(reading: &mut Reading, reason: &str, said: &str, quote: &str) {
    discard(reading, CORRECTION, reason, said, quote);
}

/// `what` of a correction in the list of discards.
pub const CORRECTION: &str = "correction";

fn normalized(text: &str) -> String {
    quote::words(text).join(" ")
}

fn applies(types: &[String], kind: &str) -> bool {
    types.is_empty() || types.iter().any(|t| t == kind)
}

/// Where a quote of a label and its value is when the text has them apart
/// (a form or an invoice laid out in columns: "CUIT:" in one place and the
/// number in another, D-248): the value as one run of the text, and what
/// the quote has before it (the label) and after it, each as one run, at
/// most `window` words away on its side. `None` when it is not so.
fn locate_apart(text: &Located, quote_text: &str, value: &[String], window: usize) -> Option<Option<i64>> {
    let quote_words = quote::words(quote_text);
    let at = read_checks::positions(&quote_words, value).into_iter().next()?;
    let (before, after) = (&quote_words[..at], &quote_words[at + value.len()..]);
    if before.is_empty() && after.is_empty() {
        return None;
    }
    let befores = text.starts(before);
    let afters = text.starts(after);
    text.starts(value)
        .into_iter()
        .find(|&s| {
            let end = s + value.len();
            (before.is_empty() || befores.iter().any(|&l| l + before.len() <= s && s - (l + before.len()) <= window))
                && (after.is_empty() || afters.iter().any(|&r| r >= end && r - end <= window))
        })
        .map(|s| text.page_at(s))
}

/// A quote found, or found with its value apart.
fn locate_value(text: &Located, quote_text: &str, value: &[String], page: Option<i64>, window: usize) -> Option<Option<i64>> {
    text.locate(quote_text, page).or_else(|| locate_apart(text, quote_text, value, window))
}

/// Whether the label of a form the model gave apart is the value's: in its
/// quote, or in the text at most `window` words before it. A label that is
/// not is ignored (the value stands on its own quote).
fn label_verified(text: &Located, quote_text: &str, value: &[String], label: Option<&str>, window: usize) -> bool {
    let Some(label) = label.map(str::trim).filter(|l| !l.is_empty()) else {
        return false;
    };
    let label_words = quote::words(label);
    let quote_words = quote::words(quote_text);
    if label_words.is_empty() {
        return false;
    }
    if quote::contains(&quote_words, &label_words) {
        return true;
    }
    let mut starts = text.starts(&quote_words);
    if starts.is_empty() {
        starts = text.starts(value);
    }
    text.starts(&label_words)
        .iter()
        .any(|&l| starts.iter().any(|&s| s >= l + label_words.len() && s - (l + label_words.len()) <= window))
}

/// Checks the answers about the parts of one document against its text,
/// and puts them together: the mentions of every part, the same mention
/// (type, name and role) once, and each claim once.
pub fn verify(vocabulary: &Vocabulary, body: &str, shapes: &[String], answers: &[Answer]) -> Reading {
    let text = Located::new(body);
    let shapes: BTreeSet<String> = shapes.iter().map(|s| names::identifier_value(s)).collect();
    let certificate = read_checks::certificate_pages(vocabulary, body);
    let window = vocabulary.reading.label_window;
    let mut reading = Reading::default();
    // The model's ref in each part, to the kept mention's ref.
    let mut refs: HashMap<(usize, String), String> = HashMap::new();
    let mut by_key: HashMap<(String, String, String), usize> = HashMap::new();
    for (part, answer) in answers.iter().enumerate() {
        if reading.doc_date.is_none() {
            reading.doc_date = answer
                .fecha_documento
                .as_deref()
                .map(str::trim)
                .filter(|d| !d.is_empty() && text.locate(d, None).is_some())
                .map(str::to_string);
        }
        for said in &answer.menciones {
            let Some(kind) = vocabulary.entity_type(&said.tipo).map(|t| t.id.clone()) else {
                discard(&mut reading, "mention", "tipo-desconocido", &said.escrito, &said.cita);
                continue;
            };
            let written = said.escrito.trim().to_string();
            let name = quote::words(&written);
            let Some(page) = locate_value(&text, &said.cita, &name, said.pagina, window) else {
                discard(&mut reading, "mention", "cita-no-encontrada", &written, &said.cita);
                continue;
            };
            if !read_checks::name_in_quote(&quote::words(&said.cita), &name) {
                discard(&mut reading, "mention", "nombre-fuera-de-la-cita", &written, &said.cita);
                continue;
            }
            // A label of a form ("PROVINCIA:") is not someone (D-245).
            if kind != vocabulary.matters.kind && read_checks::is_form_label(body, &written) {
                discard(&mut reading, "mention", "rotulo-de-formulario", &written, &said.cita);
                continue;
            }
            let mut role = vocabulary
                .role(&said.rol)
                .filter(|r| applies(&r.types, &kind))
                .map(|r| r.id.clone())
                .unwrap_or_else(|| vocabulary.fallback_role().to_string());
            // The record of a signing service does not say the roles of the
            // act (D-251).
            if certificate.contains(&page) {
                if let Some(other) = vocabulary.reading.certificate_roles.get(&role) {
                    let other = vocabulary
                        .role(other)
                        .filter(|r| applies(&r.types, &kind))
                        .map(|r| r.id.clone())
                        .unwrap_or_else(|| vocabulary.fallback_role().to_string());
                    if other != role {
                        corrected(&mut reading, "papel-de-la-constancia-de-firma", &format!("{written}: {role} → {other}"), &said.cita);
                        role = other;
                    }
                }
            }
            let mut identifiers = Vec::new();
            for identifier in &said.identificadores {
                let value = identifier.valor.trim().to_string();
                let value_words = quote::words(&value);
                let Some(id_page) = locate_value(&text, &identifier.cita, &value_words, said.pagina, window) else {
                    discard(&mut reading, "identifier", "cita-no-encontrada", &value, &identifier.cita);
                    continue;
                };
                let quote_words = quote::words(&identifier.cita);
                if !quote::contains(&quote_words, &value_words) {
                    discard(&mut reading, "identifier", "valor-fuera-de-la-cita", &value, &identifier.cita);
                    continue;
                }
                let said_kind = vocabulary
                    .identifier(&identifier.clase)
                    .map(|t| t.id.clone())
                    .or_else(|| vocabulary.identifiers.last().map(|t| t.id.clone()))
                    .unwrap_or_default();
                // The label the text puts right before the number says its
                // kind (D-249); the label of a form apart, too.
                let labelled = read_checks::kind_by_label(vocabulary, &quote_words, &value_words).or_else(|| {
                    identifier.rotulo.as_deref().filter(|r| label_verified(&text, &identifier.cita, &value_words, Some(r), window)).and_then(|r| {
                        let mut words = quote::words(r);
                        words.extend(value_words.iter().cloned());
                        read_checks::kind_by_label(vocabulary, &words, &value_words)
                    })
                });
                let id_kind = match labelled {
                    Some(label) if label != said_kind => {
                        corrected(&mut reading, "clase-por-su-rotulo", &format!("{value}: {said_kind} → {label}"), &identifier.cita);
                        label
                    }
                    _ => said_kind,
                };
                // A number of an invoice, a shipment or an envelope does not
                // identify a person or an organization.
                let term = vocabulary.identifier(&id_kind);
                if term.is_some_and(|t| !t.identifies && (t.types.is_empty() || !t.types.iter().any(|x| x == &kind))) {
                    discard(&mut reading, "identifier", "no-identifica", &value, &identifier.cita);
                    continue;
                }
                let normalized_value = names::identifier_value(&value);
                if normalized_value.is_empty() {
                    continue;
                }
                if !identifiers.iter().any(|i: &Identifier| i.kind == id_kind && i.normalized == normalized_value) {
                    identifiers.push(Identifier {
                        kind: id_kind,
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
                defined: false,
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
    define_terms(vocabulary, body, answers, &text, &refs, &mut reading);
    let kinds: HashMap<String, String> = reading.mentions.iter().map(|m| (m.reference.clone(), m.kind.clone())).collect();
    // The names of each mention within the document: its own and those of
    // the mentions that are the same entity (the defined term and what it
    // defines, the short name and the full one).
    let group_names = group_names(&reading.mentions);
    for (part, answer) in answers.iter().enumerate() {
        for said in &answer.afirmaciones {
            let Some(predicate) = vocabulary.predicate(&said.predicado).filter(|p| !p.is_money()) else {
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
            let value_words = if predicate.takes_entity() { Vec::new() } else { quote::words(said.objeto_valor.as_deref().unwrap_or_default()) };
            let Some(page) = locate_value(&text, &said.cita, &value_words, said.pagina, window) else {
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
            // What has legal consequences names its object in the quote
            // (D-250).
            if predicate.object_in_quote {
                let object = object_mention.as_deref().expect("an entity");
                let mut named = group_names.get(object).is_some_and(|names| names.iter().any(|n| read_checks::name_in_quote(&quote_words, n)));
                // A signature block or "X, representada por Y": the
                // organization named right before the quote (P-122, D-269).
                let rules = &vocabulary.reading;
                let acts_for = quote_words.iter().any(|w| {
                    rules.representation_cues.iter().any(|cue| if cue.chars().count() <= 3 { w == cue } else { w.starts_with(cue.as_str()) })
                });
                if !named && acts_for && rules.object_window > 0 && rules.object_before.iter().any(|p| *p == predicate.id) && kinds[object] == super::names::ORGANIZATION {
                    if let Some(own) = group_names.get(object) {
                        let organizations: Vec<&Vec<Vec<String>>> =
                            group_names.iter().filter(|(r, _)| kinds.get(r.as_str()).is_some_and(|k| k == super::names::ORGANIZATION)).map(|(_, n)| n).collect();
                        named = read_checks::nearest_before(body, &said.cita, &organizations, own, rules.object_window);
                    }
                }
                if !named {
                    discard(&mut reading, "claim", "objeto-fuera-de-la-cita", &said.predicado, &said.cita);
                    continue;
                }
            }
            // A time is kept only when it is in the quote too.
            let fact_time = said
                .tiempo
                .clone()
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty() && quote::contains(&quote_words, &quote::words(t)));
            let class = said
                .clase
                .as_deref()
                .filter(|c| *c != NO_CLASS && predicate.classes().any(|q| q.id == *c))
                .map(str::to_string);
            let mut detail = serde_json::Map::new();
            if let Some(class) = &class {
                detail.insert("clase".into(), class.clone().into());
            }
            if let Some(label) = said.rotulo.as_deref().map(str::trim).filter(|l| label_verified(&text, &said.cita, &value_words, Some(l), window)) {
                detail.insert("rotulo".into(), label.into());
            }
            let evidence = Evidence {
                subject,
                predicate: predicate.id.clone(),
                object_mention,
                object_value,
                quote: said.cita.trim().to_string(),
                page,
                fact_time,
                amount: None,
                detail: (!detail.is_empty()).then_some(serde_json::Value::Object(detail)),
            };
            push_evidence(&mut reading, evidence);
        }
        for said in &answer.importes {
            amount_evidence(vocabulary, &text, &refs, &kinds, &group_names, part, said, &mut reading);
        }
    }
    other_person(vocabulary, body, &mut reading);
    signature_block(vocabulary, &group_names, &mut reading);
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
    // person's: it goes to her, when she is only one (D-248).
    let body_words = quote::words(body);
    let people: Vec<(String, Vec<String>)> = reading
        .mentions
        .iter()
        .filter(|m| m.kind == PERSON_KIND)
        .map(|m| (m.reference.clone(), quote::words(&m.written)))
        .filter(|(_, w)| w.len() >= 2)
        .collect();
    let mut moved: Vec<(String, Identifier)> = Vec::new();
    let mut dropped: Vec<(String, String)> = Vec::new();
    for mention in reading.mentions.iter_mut().filter(|m| m.kind == PERSON_KIND) {
        let own = quote::words(&mention.written);
        let reference = mention.reference.clone();
        mention.identifiers.retain(|identifier| {
            let value = quote::words(&identifier.value);
            if value.is_empty() {
                return true;
            }
            let places = read_checks::positions(&body_words, &value);
            let others = |i: usize| -> BTreeSet<String> {
                let before = &body_words[i.saturating_sub(OTHER_PERSON_WINDOW)..i];
                if quote::contains(before, &own) {
                    return BTreeSet::new();
                }
                people.iter().filter(|(r, name)| *r != reference && *name != own && quote::contains(before, name)).map(|(r, _)| r.clone()).collect()
            };
            let all: Vec<BTreeSet<String>> = places.iter().map(|&i| others(i)).collect();
            let other = !places.is_empty() && all.iter().all(|o| !o.is_empty());
            if other {
                let first = &all[0];
                if first.len() == 1 && all.iter().all(|o| o == first) {
                    moved.push((first.iter().next().cloned().expect("one"), identifier.clone()));
                } else {
                    dropped.push((identifier.value.clone(), identifier.quote.clone()));
                }
            }
            !other
        });
    }
    for (to, identifier) in moved {
        corrected(&mut reading, "de-otra-persona", &identifier.value, &identifier.quote);
        if let Some(mention) = reading.mentions.iter_mut().find(|m| m.reference == to) {
            if !mention.identifiers.iter().any(|i| i.kind == identifier.kind && i.normalized == identifier.normalized) {
                mention.identifiers.push(identifier);
            }
        }
    }
    for (value, quote) in dropped {
        discard(&mut reading, "identifier", "de-otra-persona", &value, &quote);
    }
    reading
}

fn push_evidence(reading: &mut Reading, evidence: Evidence) {
    let same = |e: &Evidence| {
        e.subject == evidence.subject
            && e.predicate == evidence.predicate
            && e.object_mention == evidence.object_mention
            && e.object_value.as_deref().map(normalized) == evidence.object_value.as_deref().map(normalized)
            && e.amount.as_deref().map(normalized) == evidence.amount.as_deref().map(normalized)
    };
    if !reading.evidence.iter().any(same) {
        reading.evidence.push(evidence);
    }
}

/// The names (normalized words) of each mention within its document: its
/// own and those of the mentions that are the same entity in it.
fn group_names(mentions: &[Mention]) -> HashMap<String, Vec<Vec<String>>> {
    let index: HashMap<&str, usize> = mentions.iter().enumerate().map(|(i, m)| (m.reference.as_str(), i)).collect();
    let mut root: Vec<usize> = (0..mentions.len()).collect();
    fn find(root: &mut [usize], x: usize) -> usize {
        let mut r = x;
        while root[r] != r {
            r = root[r];
        }
        root[x] = r;
        r
    }
    for (i, mention) in mentions.iter().enumerate() {
        if let Some(&j) = mention.same_as.as_deref().and_then(|s| index.get(s)) {
            if mentions[j].kind == mention.kind {
                let (a, b) = (find(&mut root, i), find(&mut root, j));
                root[a.max(b)] = a.min(b);
            }
        }
    }
    // The same name in the document is the same mention's.
    for i in 0..mentions.len() {
        for j in 0..i {
            if mentions[i].kind == mentions[j].kind && mentions[i].normalized == mentions[j].normalized {
                let (a, b) = (find(&mut root, i), find(&mut root, j));
                root[a.max(b)] = a.min(b);
            }
        }
    }
    let mut by_root: HashMap<usize, Vec<Vec<String>>> = HashMap::new();
    for i in 0..mentions.len() {
        let r = find(&mut root, i);
        let words: Vec<String> = mentions[i].normalized.split_whitespace().map(str::to_string).collect();
        if !words.is_empty() {
            let list = by_root.entry(r).or_default();
            if !list.contains(&words) {
                list.push(words);
            }
        }
    }
    (0..mentions.len())
        .map(|i| {
            let r = find(&mut root, i);
            (mentions[i].reference.clone(), by_root.get(&r).cloned().unwrap_or_default())
        })
        .collect()
}

/// The terms the document defines are local aliases (D-245): what the
/// model gave in "definiciones" (with a quote that has the term and a name
/// of what it defines) and what has the shape of a definition in the text.
/// A mention written as such a term is marked as defined and, when it is
/// not already the same as another mention, made the same as what the
/// term defines: the model's, or the nearest name before the definition.
fn define_terms(vocabulary: &Vocabulary, body: &str, answers: &[Answer], text: &Located, refs: &HashMap<(usize, String), String>, reading: &mut Reading) {
    let mut targets: HashMap<String, String> = HashMap::new();
    let names: HashMap<String, (String, Vec<String>)> =
        reading.mentions.iter().map(|m| (m.reference.clone(), (m.kind.clone(), quote::words(&m.written)))).collect();
    for (part, answer) in answers.iter().enumerate() {
        for said in &answer.definiciones {
            let term = read_checks::without_article(&normalized(&said.termino));
            let quote_words = quote::words(&said.cita);
            let Some(target) = refs.get(&(part, said.reference.clone())).cloned() else {
                discard(reading, "definition", "sujeto-desconocido", &said.termino, &said.cita);
                continue;
            };
            let named = names.get(&target).is_some_and(|(_, n)| read_checks::name_in_quote(&quote_words, n));
            let term_words: Vec<String> = term.split_whitespace().map(str::to_string).collect();
            if term.is_empty() || text.locate(&said.cita, None).is_none() || !quote::contains(&quote_words, &term_words) {
                discard(reading, "definition", "cita-no-encontrada", &said.termino, &said.cita);
                continue;
            }
            if !named {
                // A term of something that is not a mention ("los Servicios").
                discard(reading, "definition", "definicion-sin-nombre", &said.termino, &said.cita);
                continue;
            }
            targets.entry(term).or_insert(target);
        }
    }
    let body_words = quote::words(body);
    let mut shaped: HashMap<String, Option<String>> = HashMap::new();
    for (term, at) in read_checks::defined_terms(body) {
        // The nearest name of a mention (not itself a term) that ends
        // before the definition, in the forty words before it.
        let from = at.saturating_sub(40);
        let before = &body_words[from..at.min(body_words.len())];
        let nearest = reading
            .mentions
            .iter()
            .filter(|m| read_checks::without_article(&m.normalized) != term && !m.normalized.is_empty())
            .filter_map(|m| {
                let words: Vec<String> = m.normalized.split_whitespace().map(str::to_string).collect();
                read_checks::positions(before, &words).last().map(|p| (p + words.len(), m.reference.clone()))
            })
            .max_by_key(|(end, _)| *end)
            .map(|(_, r)| r);
        shaped.entry(term).and_modify(|t| {
            if *t != nearest {
                *t = None;
            }
        }).or_insert(nearest);
    }
    let kinds: HashMap<String, String> = reading.mentions.iter().map(|m| (m.reference.clone(), m.kind.clone())).collect();
    for mention in &mut reading.mentions {
        if mention.kind == vocabulary.matters.kind {
            continue;
        }
        let term = read_checks::without_article(&mention.normalized);
        let target = match (targets.get(&term), shaped.get(&term)) {
            (Some(target), _) => Some(target.clone()),
            (None, Some(target)) => target.clone(),
            (None, None) => continue,
        };
        mention.defined = true;
        let target = target.filter(|t| *t != mention.reference && kinds.get(t) == Some(&mention.kind));
        if mention.same_as.is_none() {
            mention.same_as = target;
        }
    }
    // What a term defines is not itself a defined term.
    let defined: BTreeSet<String> = reading.mentions.iter().filter(|m| m.defined).map(|m| m.reference.clone()).collect();
    for mention in &mut reading.mentions {
        if mention.same_as.as_ref().is_some_and(|s| defined.contains(s)) && mention.defined {
            mention.same_as = None;
        }
    }
}

/// An amount between two parties (D-244): both parties are mentions of
/// the document (persons or organizations, not the same one), the amount
/// is in the quote and not next to a word of a tax, and the direction the
/// quote says wins over the model's (counted as a correction).
#[allow(clippy::too_many_arguments)]
fn amount_evidence(
    vocabulary: &Vocabulary,
    text: &Located,
    refs: &HashMap<(usize, String), String>,
    kinds: &HashMap<String, String>,
    group_names: &HashMap<String, Vec<Vec<String>>>,
    part: usize,
    said: &AnswerAmount,
    reading: &mut Reading,
) {
    let Some(predicate) = vocabulary.money_predicate() else { return };
    let what = format!("{}: {}", said.tipo, said.importe.trim());
    let party = |r: &Option<String>| r.as_ref().and_then(|r| refs.get(&(part, r.clone()))).cloned();
    let (Some(mut debtor), Some(mut creditor)) = (party(&said.deudor), party(&said.acreedor)) else {
        discard(reading, "amount", "sin-contraparte", &what, &said.cita);
        return;
    };
    let fits = |r: &String| kinds.get(r).is_some_and(|k| applies(&predicate.subject, k) && applies(&predicate.object_types, k));
    if !fits(&debtor) || !fits(&creditor) {
        discard(reading, "amount", "no-corresponde-al-tipo", &what, &said.cita);
        return;
    }
    let same_group = group_names.get(&debtor) == group_names.get(&creditor) && debtor != creditor && group_names.get(&debtor).is_some_and(|g| !g.is_empty());
    if debtor == creditor || same_group {
        discard(reading, "amount", "mismas-partes", &what, &said.cita);
        return;
    }
    let Some(page) = locate_value(text, &said.cita, &quote::words(said.importe.trim()), said.pagina, vocabulary.reading.label_window) else {
        discard(reading, "amount", "cita-no-encontrada", &what, &said.cita);
        return;
    };
    let quote_words = quote::words(&said.cita);
    let amount = said.importe.trim().to_string();
    let amount_words = quote::words(&amount);
    if amount_words.is_empty() || !quote::contains(&quote_words, &amount_words) {
        discard(reading, "amount", "valor-fuera-de-la-cita", &what, &said.cita);
        return;
    }
    if !values::fits(vocabulary, "amount", &amount, "") {
        discard(reading, "amount", "no-es-un-importe", &what, &said.cita);
        return;
    }
    if read_checks::amount_about_else(&predicate.not_cues, &quote_words, &amount_words) {
        discard(reading, "amount", "es-un-impuesto", &what, &said.cita);
        return;
    }
    let names_of = |r: &String| group_names.get(r).cloned().unwrap_or_default();
    if read_checks::money_direction(vocabulary, &quote_words, &names_of(&debtor), &names_of(&creditor)) == Some(false) {
        corrected(reading, "direccion-invertida", &what, &said.cita);
        std::mem::swap(&mut debtor, &mut creditor);
    }
    let kind = vocabulary.money_kind(&said.tipo).or_else(|| vocabulary.money.kinds.last()).map(|k| k.id.clone()).unwrap_or_default();
    let in_quote = |v: &Option<String>| -> Option<String> {
        v.as_deref().map(str::trim).filter(|v| !v.is_empty() && quote::contains(&quote_words, &quote::words(v))).map(str::to_string)
    };
    let mut detail = serde_json::Map::new();
    detail.insert("tipo".into(), kind.into());
    let currency = in_quote(&said.moneda).or_else(|| {
        let words = quote::words(&amount);
        let sign = amount.contains('$') || amount.contains('€');
        vocabulary.values.currencies.iter().find(|c| words.contains(c)).cloned().or_else(|| sign.then(|| if amount.contains('€') { "€".to_string() } else { "$".to_string() }))
    });
    if let Some(currency) = currency {
        detail.insert("moneda".into(), currency.into());
    }
    if let Some(concept) = in_quote(&said.concepto) {
        detail.insert("concepto".into(), concept.into());
    }
    push_evidence(
        reading,
        Evidence {
            subject: debtor,
            predicate: predicate.id.clone(),
            object_mention: Some(creditor),
            object_value: None,
            quote: said.cita.trim().to_string(),
            page,
            fact_time: in_quote(&said.fecha),
            amount: Some(amount),
            detail: Some(serde_json::Value::Object(detail)),
        },
    );
}

/// A value the quote puts under a label of another person of a form
/// ("cónyuge") is that person's, when the reading names one such person
/// whose own quote has that label; otherwise it is nobody's (D-248).
fn other_person(vocabulary: &Vocabulary, _body: &str, reading: &mut Reading) {
    let labels = &vocabulary.reading.other_person_labels;
    if labels.is_empty() {
        return;
    }
    let has_label = |text: &str| quote::words(text).iter().any(|w| labels.iter().any(|l| w.starts_with(l.as_str())));
    let labelled: Vec<String> =
        reading.mentions.iter().filter(|m| m.kind == PERSON_KIND && has_label(&m.quote)).map(|m| m.reference.clone()).collect();
    let person: BTreeSet<String> = reading.mentions.iter().filter(|m| m.kind == PERSON_KIND).map(|m| m.reference.clone()).collect();
    let mut kept = Vec::new();
    let evidence = std::mem::take(&mut reading.evidence);
    for mut item in evidence {
        let under = has_label(&item.quote) || item.detail.as_ref().and_then(|d| d.get("rotulo")).and_then(|r| r.as_str()).is_some_and(has_label);
        // A relation between the two (the spouse of) says it of both.
        let between = item.object_mention.as_ref().is_some_and(|o| labelled.contains(o));
        if !under || !person.contains(&item.subject) || labelled.contains(&item.subject) || between {
            kept.push(item);
            continue;
        }
        match labelled.as_slice() {
            [other] => {
                corrected(reading, "de-otra-persona", &item.predicate, &item.quote);
                item.subject = other.clone();
                kept.push(item);
            }
            _ => discard(reading, "claim", "de-otra-persona", &item.predicate, &item.quote),
        }
    }
    reading.evidence = kept;
    // The same for identifiers.
    let mut moved: Vec<(String, Identifier)> = Vec::new();
    let mut dropped: Vec<(String, String)> = Vec::new();
    for mention in reading.mentions.iter_mut().filter(|m| m.kind == PERSON_KIND && !labelled.contains(&m.reference)) {
        mention.identifiers.retain(|identifier| {
            if !has_label(&identifier.quote) {
                return true;
            }
            match labelled.as_slice() {
                [other] => moved.push((other.clone(), identifier.clone())),
                _ => dropped.push((identifier.value.clone(), identifier.quote.clone())),
            }
            false
        });
    }
    for (to, identifier) in moved {
        corrected(reading, "de-otra-persona", &identifier.value, &identifier.quote);
        if let Some(mention) = reading.mentions.iter_mut().find(|m| m.reference == to) {
            if !mention.identifiers.iter().any(|i| i.kind == identifier.kind && i.normalized == identifier.normalized) {
                mention.identifiers.push(identifier);
            }
        }
    }
    for (value, quote) in dropped {
        discard(reading, "identifier", "de-otra-persona", &value, &quote);
    }
}

/// A domicile the model gave a person whose quote names, between the
/// person and the address, an organization of the document ("X, por
/// Ejemplo S.A., con domicilio en…"): the address of a signature block is
/// the organization's (D-247).
fn signature_block(vocabulary: &Vocabulary, group_names: &HashMap<String, Vec<Vec<String>>>, reading: &mut Reading) {
    let organization = ORGANIZATION_KIND.to_string();
    let organizations: Vec<String> =
        reading.mentions.iter().filter(|m| m.kind == organization).map(|m| m.reference.clone()).collect();
    let kinds: HashMap<String, String> = reading.mentions.iter().map(|m| (m.reference.clone(), m.kind.clone())).collect();
    let mut changes: Vec<(usize, String)> = Vec::new();
    for (index, item) in reading.evidence.iter().enumerate() {
        let address = vocabulary.predicate(&item.predicate).is_some_and(|p| p.shape == "address");
        if !address || kinds.get(&item.subject).map(String::as_str) != Some(PERSON_KIND) {
            continue;
        }
        let Some(value) = item.object_value.as_deref() else { continue };
        let words = quote::words(&item.quote);
        let Some(&at) = read_checks::positions(&words, &quote::words(value)).first() else { continue };
        let ends = |r: &String| -> Vec<(usize, usize)> {
            group_names
                .get(r)
                .into_iter()
                .flatten()
                .flat_map(|n| read_checks::positions(&words, n).into_iter().map(move |p| (p, p + n.len())))
                .filter(|(_, end)| *end <= at)
                .collect()
        };
        let Some(person_end) = ends(&item.subject).iter().map(|(_, e)| *e).max() else { continue };
        let between: Vec<&String> =
            organizations.iter().filter(|o| ends(o).iter().any(|(start, _)| *start >= person_end)).collect();
        if let [only] = between.as_slice() {
            changes.push((index, (*only).clone()));
        }
    }
    for (index, to) in changes {
        let quote = reading.evidence[index].quote.clone();
        corrected(reading, "domicilio-de-la-sociedad", &reading.evidence[index].object_value.clone().unwrap_or_default(), &quote);
        reading.evidence[index].subject = to;
    }
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
    let doc_date = reading.ok().and_then(|r| r.doc_date.clone());
    let (state, mentions, evidence, discarded, error) = match reading {
        Ok(reading) => {
            let mut ids: HashMap<&str, i64> = HashMap::new();
            for mention in &reading.mentions {
                tx.execute(
                    "INSERT INTO knowledge_mention (content_id, ref, same_as, kind, written, normalized, role, quote, page, defined)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                    params![
                        pending.content_id,
                        mention.reference,
                        mention.same_as,
                        mention.kind,
                        mention.written,
                        mention.normalized,
                        mention.role,
                        mention.quote,
                        mention.page,
                        mention.defined
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
                        (content_id, subject_mention, predicate, object_mention, object_value, object_normalized, quote, page, fact_time,
                         amount, detail)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        pending.content_id,
                        ids[item.subject.as_str()],
                        item.predicate,
                        item.object_mention.as_deref().map(|m| ids[m]),
                        item.object_value,
                        item.object_value.as_deref().map(normalized).or_else(|| item.amount.as_deref().map(normalized)),
                        item.quote,
                        item.page,
                        item.fact_time,
                        item.amount,
                        item.detail.as_ref().map(|d| d.to_string())
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
             evidence, discarded, error, extracted_at, doc_date)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
         ON CONFLICT (content_id) DO UPDATE SET
            state = excluded.state, basis = excluded.basis, prompt_version = excluded.prompt_version,
            vocabulary = excluded.vocabulary, text_hash = excluded.text_hash, text_at = excluded.text_at, engine = excluded.engine,
            questions = excluded.questions, parts = excluded.parts, mentions = excluded.mentions,
            evidence = excluded.evidence, discarded = excluded.discarded, error = excluded.error,
            extracted_at = excluded.extracted_at, doc_date = excluded.doc_date",
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
            now_ms(),
            doc_date
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
        let min_words = vocabulary().attention.empty_min_words;
        for item in &pending {
            let part_words: Vec<usize> = parts(&item.body).iter().map(|p| p.split_whitespace().count()).collect();
            for (index, question) in questions(&conn, vocabulary(), item.content_id, &item.body)?.0.into_iter().enumerate() {
                let long = part_words.get(index).copied().unwrap_or(0) >= min_words;
                let kept = cached_answer(&conn, &question.key, engine_id.as_deref())?.filter(|a| !(long && is_empty_answer(a)));
                if kept.is_none() {
                    count += 1;
                }
            }
        }
        Ok(KnowledgePlan { documents: pending.len(), questions: count, waiting_for_type: waiting, engine })
    }

    /// Forgets the readings that failed (over what `visible` lets see), so
    /// that the next run reads those documents again. What the model
    /// answered well for some of their parts is kept and not asked again.
    pub fn knowledge_read_again_failed(&self, visible: &Visible) -> Result<usize, String> {
        let failed: Vec<i64> = standings(&self.conn(), visible, None)?
            .into_iter()
            .filter(|s| s.reading.as_ref().is_some_and(|r| r.state == "failed"))
            .map(|s| s.content_id)
            .collect();
        let conn = self.conn();
        for content_id in &failed {
            conn.execute("DELETE FROM knowledge_extraction WHERE content_id = ?1 AND state = 'failed'", [content_id])
                .map_err(sql_err)?;
        }
        Ok(failed.len())
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
            let part_words: Vec<usize> = parts(&item.body).iter().map(|p| p.split_whitespace().count()).collect();
            let rules = &vocabulary.attention;
            let mut answers: Vec<String> = Vec::new();
            let mut empty_parts = 0;
            for (index, question) in item_questions.iter().enumerate() {
                // An empty answer about a long part is not a reading: the
                // kept one does not count, and it is asked again (D-235).
                let long = part_words.get(index).copied().unwrap_or(0) >= rules.empty_min_words;
                let usable = |answer: &str| !(long && is_empty_answer(answer));
                if let Some(answer) = cached_answer(&self.conn(), &question.key, Some(&engine))?.filter(|a| usable(a)) {
                    answers.push(answer);
                    continue;
                }
                let mut got: Option<String> = None;
                let mut attempts = 0;
                while attempts < rules.empty_attempts.max(1) && budget > 0 && !stopped {
                    attempts += 1;
                    budget -= 1;
                    report.asked += 1;
                    match ask_fresh(self, asker, question, &engine, &usable) {
                        Ok(Some(answer)) => {
                            got = Some(answer);
                            break;
                        }
                        Ok(None) => {}
                        Err(error) => {
                            report.failed += 1;
                            stopped = stops_the_run(&error);
                            report.errors.push(format!("{error} ({})", item.content_id));
                            attempts = usize::MAX;
                        }
                    }
                }
                match got {
                    Some(answer) => answers.push(answer),
                    // Asked as many times as allowed and still empty.
                    None if attempts != usize::MAX && attempts >= rules.empty_attempts.max(1) => {
                        empty_parts += 1;
                        answers.push(String::new());
                    }
                    None => break,
                }
            }
            if answers.len() < item_questions.len() {
                report.left += 1;
                continue;
            }
            if empty_parts > 0 {
                let error = empty_reading_error(empty_parts, item_questions.len(), rules.empty_attempts.max(1));
                let mut conn = self.conn();
                write_reading(&mut conn, &item, &item_questions, &engine, Err(&error))?;
                report.unusable += 1;
                report.errors.push(format!("{error} ({})", item.content_id));
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

/// Whether an answer names nobody and says nothing. An answer that does
/// not have the shape asked for is not empty: it is unusable.
fn is_empty_answer(answer: &str) -> bool {
    parse_json::<Answer>(answer).is_ok_and(|a| a.menciones.is_empty() && a.afirmaciones.is_empty())
}

/// Asks a question again, without looking at the answer kept for it, and
/// keeps the new answer only when it is usable. `None`: the model answered,
/// but with an answer that does not count.
fn ask_fresh(
    base: &DocBase,
    asker: &dyn DocumentAsker,
    question: &Question,
    engine: &str,
    usable: &dyn Fn(&str) -> bool,
) -> Result<Option<String>, String> {
    if let Some(answer) = cached_answer(&base.conn(), &question.key, Some(engine))?.filter(|a| usable(a)) {
        return Ok(Some(answer));
    }
    let answer = asker.ask(&question.prompt, &question.schema)?;
    parse_json::<serde_json::Value>(&answer)?;
    if !usable(&answer) {
        return Ok(None);
    }
    classify::keep_answer(&base.conn(), question, engine, &answer)?;
    Ok(Some(answer))
}

/// The reason of a reading written as failed because the model read a long
/// text as naming nobody (shown in the attention view).
fn empty_reading_error(empty: usize, parts: usize, attempts: usize) -> String {
    let which = if parts == 1 {
        "El modelo leyó el texto sin encontrar a nadie".to_string()
    } else if empty == 1 {
        format!("El modelo leyó una de las {parts} partes del texto sin encontrar a nadie")
    } else {
        format!("El modelo leyó {empty} de las {parts} partes del texto sin encontrar a nadie")
    };
    let times = if attempts == 1 { "1 vez".to_string() } else { format!("{attempts} veces") };
    format!("{which}, aunque es largo; se preguntó {times}. {EMPTY_READING_MARK}")
}

/// Ends the reason of a reading that came back empty (for the attention
/// view and the evaluation).
pub const EMPTY_READING_MARK: &str = "(lectura vacía)";

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
