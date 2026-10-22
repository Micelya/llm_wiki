//! What the base asks the model (P-278), for Ajustes → «Lo que se le
//! pregunta al modelo»: each question with its version, what it is for and
//! when it is asked, the catalogs it is written with (the one of the data
//! folder or the one that comes with the app) and the engine of the
//! registry of AI services that answers it.
//!
//! - **How it is built.** Each question is built by the same function the
//!   reading uses (`extract::prompt`, `classify::type_prompt`…), over an
//!   invented document (`example`) and with the catalogs in use. What
//!   changes in each case (the text of the document, the candidates) is
//!   marked where the function puts it: the function gets each such value
//!   between two marks (`marked`), and the marks become segments. Built
//!   with the values as they are (`plain`), the question is the same text
//!   without the marks: the tests check it.
//! - **The last one asked.** The base keeps every question it sent with its
//!   whole text (`model_answer`): the last one of the current version, on
//!   request, and how many were kept of each version. That text carries the
//!   text of the firm's documents. The recognition of scans and the chat
//!   keep none.
//!
//! Nothing here asks the model or writes in the base.

use std::collections::HashMap;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use super::knowledge::{extract, matters, pages, resolve, vocabulary};
use super::services::{self, EngineChoice, Service};
use super::similar::classify::{DATA_PROMPT, TYPE_PROMPT};
use super::similar::types::{self, Catalog, DocType};
use super::{sql_err, DocBase};

/// The marks around a value that changes in each case: start, the kind of
/// value (one word, so that no count of words changes), separator, the
/// value, end. Characters of private use: no text has them.
const START: char = '\u{E000}';
const SEP: char = '\u{E001}';
const END: char = '\u{E002}';

/// Writes a value of an example, given its kind: as it is (`plain`) or
/// between the marks (`marked`).
pub(crate) type Part<'a> = &'a dyn Fn(&str, &str) -> String;

#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn plain(_kind: &str, value: &str) -> String {
    value.to_string()
}

pub(crate) fn marked(kind: &str, value: &str) -> String {
    format!("{START}{kind}{SEP}{value}{END}")
}

/// The kinds of the values that change in each case.
pub(crate) const FILE: &str = "archivo";
pub(crate) const TEXT: &str = "texto";
pub(crate) const KNOWN: &str = "sabido";
pub(crate) const GROUP: &str = "grupo";
pub(crate) const PRINCIPAL: &str = "principal";
pub(crate) const CANDIDATE: &str = "candidato";
pub(crate) const ENTITY: &str = "entidad";
pub(crate) const CLAIM: &str = "afirmacion";
pub(crate) const QUOTE: &str = "cita";
pub(crate) const LANGUAGE: &str = "idioma";
pub(crate) const DOCUMENT: &str = "documento";
pub(crate) const MATTER: &str = "asunto";
const OVERVIEW: &str = "panorama";
const WORKSPACE: &str = "carpeta";
const REQUEST: &str = "pedido";
const FOUND: &str = "busqueda";

/// A kind of value in words, for the screen.
fn kind_label(kind: &str) -> &str {
    match kind {
        FILE => "el nombre del archivo",
        TEXT => "el texto del documento",
        KNOWN => "lo que la base ya sabe del documento",
        GROUP => "otro documento del grupo",
        PRINCIPAL => "lo que muestran los documentos de la entidad principal",
        CANDIDATE => "lo que muestran los documentos de un candidato",
        ENTITY => "un nombre de la base",
        CLAIM => "una afirmación comprobada",
        QUOTE => "una cita de un documento",
        LANGUAGE => "el idioma elegido para el archivo",
        DOCUMENT => "lo que la base sabe del documento",
        MATTER => "un asunto con sus documentos y sus partes",
        OVERVIEW => "el panorama del archivo",
        WORKSPACE => "la carpeta de trabajo del chat",
        REQUEST => "lo que escribió la persona",
        FOUND => "lo que encontró la búsqueda",
        other => other,
    }
}

/// An invented document and what the base would know of it. Every name,
/// number and word of it is made up; the questions are built over it.
pub(crate) mod example {
    use super::{Catalog, DocType};

    /// A letter of a company to a person.
    pub const FILE: &str = "Ejemplo - carta documento.pdf";
    pub const TEXT: &str = "## Page 1\n\nCARTA DOCUMENTO\nBuenos Aires, 10 de marzo de 2025.\n\
Remitente: Ejemplo Uno S.A., CUIT 30-11111111-1, con domicilio en Calle Inventada 456.\n\
Destinatario: Persona Dos, DNI 11.111.111, con domicilio en Calle Uno 100.\n\
Intimo a usted para que en el plazo de cinco días abone la suma de $ 100.000 que adeuda por la factura 0001-00000001, \
bajo apercibimiento de iniciar las acciones que correspondan.\n\
Persona Tres, por Ejemplo Uno S.A.";
    /// A copy of it that changes in one word: the group of the question
    /// about the type.
    pub const COPY_FILE: &str = "Ejemplo - carta documento (copia).pdf";
    /// The receipt of the same debt, signed with an initial.
    pub const OTHER_FILE: &str = "Ejemplo - recibo.pdf";
    pub const OTHER_TEXT: &str = "## Page 1\n\nRECIBO\n\
Recibí de P. Dos, con domicilio en Calle Uno 100, la suma de $ 100.000 por la factura 0001-00000001 de Ejemplo Uno S.A.\n\
Buenos Aires, 20 de marzo de 2025.";
    pub const COMPANY: &str = "Ejemplo Uno S.A.";
    pub const PERSON: &str = "Persona Dos";
    pub const PERSON_SHORT: &str = "P. Dos";
    pub const PERSON_ID: &str = "11.111.111";
    pub const ADDRESS: &str = "Calle Uno 100";
    pub const PERSON_QUOTE: &str = "Destinatario: Persona Dos, DNI 11.111.111, con domicilio en Calle Uno 100.";
    pub const COMPANY_QUOTE: &str = "Remitente: Ejemplo Uno S.A., CUIT 30-11111111-1";
    pub const DEBT_QUOTE: &str = "abone la suma de $ 100.000 que adeuda por la factura 0001-00000001";
    pub const PERSON_SHORT_QUOTE: &str = "Recibí de P. Dos, con domicilio en Calle Uno 100";
    pub const COMPANY_OTHER_QUOTE: &str = "por la factura 0001-00000001 de Ejemplo Uno S.A.";
    /// The matter the two documents are about.
    pub const MATTER: &str = "Reclamo por la factura 0001-00000001";
    /// For the chat: the folder of the archive, its overview, what a person
    /// wrote and what the search found.
    pub const PROJECT: &str = "C:/Ejemplo/Micelya";
    pub const OVERVIEW: &str = "# Panorama del archivo\n\nEl archivo tiene 2 documentos de 1 asunto, que nombran a 1 persona y a 1 organización.";
    pub const REQUEST: &str = "¿Qué le reclama Ejemplo Uno S.A. a Persona Dos?";
    pub const FOUND: &str = "base.search encontró 1 documento del archivo.";
    pub const FOUND_PATH: &str = "wiki/sources/ejemplo-carta-documento.md";

    /// The type of the catalog in use the invented document is said to be:
    /// the first one with data of its own that says what a document is.
    pub fn document_type(catalog: &Catalog) -> Option<&DocType> {
        catalog
            .document_types()
            .find(|t| !t.data.is_empty())
            .or_else(|| catalog.document_types().next())
    }
}

/// The term of a catalog an example uses: `wanted` when the catalog has
/// it, otherwise its first term of that kind.
pub(crate) fn term(ids: &[&str], wanted: &str) -> String {
    if ids.contains(&wanted) {
        wanted.to_string()
    } else {
        ids.first().copied().unwrap_or(wanted).to_string()
    }
}

/// A catalog a question is written with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CatalogFile {
    /// The vocabulary of the knowledge layer.
    Knowledge,
    /// The types of document.
    Types,
}

impl CatalogFile {
    fn file(self) -> &'static str {
        match self {
            CatalogFile::Knowledge => "knowledge.json",
            CatalogFile::Types => "document-types.json",
        }
    }

    fn name(self) -> &'static str {
        match self {
            CatalogFile::Knowledge => "Vocabulario de personas, organizaciones y asuntos",
            CatalogFile::Types => "Tipos de documento",
        }
    }

    /// Where the one in use came from, when it is the file of the data folder.
    fn installed_from(self) -> Option<&'static Path> {
        match self {
            CatalogFile::Knowledge => vocabulary::installed_from(),
            CatalogFile::Types => types::installed_from(),
        }
    }
}

/// One question, as the program knows it.
struct Definition {
    id: &'static str,
    /// The task its questions are kept under in `model_answer`; none when
    /// they are not kept.
    task: Option<&'static str>,
    version: Option<&'static str>,
    service: Service,
    catalogs: &'static [CatalogFile],
    name: &'static str,
    purpose: &'static str,
    when: &'static str,
    note: Option<&'static str>,
}

const DEFINITIONS: &[Definition] = &[
    Definition {
        id: "knowledge",
        task: Some(extract::TASK),
        version: Some(extract::KNOWLEDGE_PROMPT),
        service: Service::Ask,
        catalogs: &[CatalogFile::Knowledge, CatalogFile::Types],
        name: "Leer un documento",
        purpose: "Registra las personas, organizaciones y asuntos que nombra el documento, con qué papel aparece cada uno, \
lo que dice de ellos y un resumen. El programa comprueba cada cita contra el texto y descarta lo que no está.",
        when: "Una vez por documento con texto (una pregunta por cada parte de hasta 4000 palabras), después de saber su tipo. \
Otra vez solo si cambia su texto o una persona pide releerlo.",
        note: None,
    },
    Definition {
        id: "type",
        // As `classify::type_question` keeps them.
        task: Some("type"),
        version: Some(TYPE_PROMPT),
        service: Service::Ask,
        catalogs: &[CatalogFile::Types],
        name: "Qué tipo de documento es",
        purpose: "Elige el tipo y la etapa del documento de la lista de tipos, o propone uno nuevo, que queda a confirmar por una persona.",
        when: "Una vez por grupo de documentos parecidos (sobre el principal) o por documento suelto, cuando las marcas que el \
programa verificó en el texto no alcanzan para decidirlo.",
        note: Some("Los tipos que una persona confirmó o rechazó entran en la lista."),
    },
    Definition {
        id: "data",
        // As `classify::data_question` keeps them.
        task: Some("data"),
        version: Some(DATA_PROMPT),
        service: Service::Ask,
        catalogs: &[CatalogFile::Types],
        name: "Los datos de un documento según su tipo",
        purpose: "Copia del texto los datos obligatorios de su tipo que las reglas del programa no encontraron solas.",
        when: "Después de saber el tipo, y solo si las reglas no encontraron un dato obligatorio de ese tipo.",
        note: Some("En el ejemplo, el tipo es el primero del catálogo con un dato obligatorio que las reglas no encuentran en el \
documento inventado."),
    },
    Definition {
        id: "identity",
        task: Some(resolve::TASK),
        version: Some(resolve::IDENTITY_PROMPT),
        service: Service::Ask,
        catalogs: &[CatalogFile::Knowledge],
        name: "Quién es quién",
        purpose: "Decide si nombres parecidos de distintos documentos (una inicial, una abreviatura, un error de tipeo) son la \
misma persona u organización, distintas o dudosas.",
        when: "Al juntar las menciones de todo el archivo, solo para los nombres que pueden ser la misma entidad y que no \
resolvieron ni una persona, ni un identificador, ni el mismo nombre completo. Hasta 8 candidatos por pregunta.",
        note: None,
    },
    Definition {
        id: "synthesis",
        task: Some(pages::TASK),
        version: Some(pages::SYNTHESIS_PROMPT),
        service: Service::Ask,
        catalogs: &[CatalogFile::Knowledge],
        name: "Lo que dicen los documentos, en una ficha",
        purpose: "Redacta el texto de la ficha de una persona u organización con sus afirmaciones comprobadas. El programa \
descarta toda oración que no se apoya en ellas.",
        when: "Una vez por ficha, y otra vez cuando cambian sus afirmaciones.",
        note: None,
    },
    Definition {
        id: "matter",
        task: Some(matters::TASK),
        version: Some(matters::MATTER_PROMPT),
        service: Service::Ask,
        catalogs: &[CatalogFile::Knowledge],
        name: "De qué asunto es un documento",
        purpose: "Propone a qué asunto pertenece un documento que comparte personas con un asunto sin que nada lo ponga en él. \
Queda como propuesta para una persona.",
        when: "Para cada documento suelto que comparte partes con un asunto, después de juntar las menciones.",
        note: None,
    },
    Definition {
        id: "recognition",
        task: None,
        version: None,
        service: Service::Recognize,
        catalogs: &[],
        name: "Reconocer el texto de un escaneo",
        purpose: "Transcribe el texto de una página escaneada o fotografiada.",
        when: "Una vez por página de un documento que no tiene texto propio, con la imagen de la página adjunta.",
        note: Some("La pregunta va con la imagen de la página. No tiene versión, y no se guarda: se guarda solo el texto que devuelve."),
    },
    Definition {
        id: "chat",
        task: None,
        version: None,
        service: Service::Ask,
        catalogs: &[],
        name: "Contestar en el chat",
        purpose: "Contesta lo que una persona escribe en el chat, con lo que la base encuentra.",
        when: "En cada mensaje del chat.",
        note: Some("Es la pregunta del chat con un proveedor por internet (una clave de API); antes puede hacer otra corta, en \
inglés, para decidir qué buscar. Con Codex o Claude Code, que corren en esta computadora, la pantalla del chat arma una \
pregunta más corta, que no se muestra acá. No tiene versión, y no se guarda."),
    },
];

/// A piece of a question: as it is in every case, or a value that changes
/// in each case (`part` says which, in words).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub text: String,
    pub part: Option<String>,
}

/// One text sent to the model, in order: the question, or the text of
/// system and the message of the chat.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sent {
    pub title: String,
    pub segments: Vec<Segment>,
}

/// A catalog a question is written with, and which one is in use.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogUse {
    pub file: String,
    pub name: String,
    /// The one of the data folder (true) or the one that comes with the app.
    pub from_folder: bool,
    /// Where the data folder's file is (or would be).
    pub folder_path: Option<String>,
    /// There is a file in the data folder that is not in use: it is not
    /// valid, or it was put there after the app opened.
    pub unused_file: bool,
}

/// How many questions of a version the base keeps.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionCount {
    pub version: String,
    pub count: i64,
}

/// A question, for the screen.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelQuestion {
    pub id: String,
    pub name: String,
    pub purpose: String,
    pub when: String,
    pub note: Option<String>,
    /// None: the question has no version.
    pub version: Option<String>,
    pub catalogs: Vec<CatalogUse>,
    /// The service of the registry that answers it (`ask`, `recognize`)
    /// and the engine it has.
    pub service: String,
    pub engine: Option<EngineChoice>,
    /// Why it is not asked now, when it is not (no engine, or one the base
    /// does not run).
    pub engine_note: Option<String>,
    /// How it is built over the invented document: what is sent, in order.
    pub example: Vec<Sent>,
    /// Why there is no example, when there is none.
    pub no_example: Option<String>,
    /// The shape of the answer it asks for, as JSON, when it asks for one.
    pub answer_shape: Option<String>,
    /// The base keeps each one asked (`model_answer`).
    pub kept: bool,
    /// How many the base keeps of each version, the current one first.
    pub kept_by_version: Vec<VersionCount>,
}

/// The last question of its current version that the base keeps.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskedQuestion {
    pub version: String,
    pub engine: String,
    pub asked_at: i64,
    /// Its whole text, with the text of the documents it was about.
    pub prompt: String,
}

/// The texts of a question built over the invented document, and the
/// shape of its answer; none when the catalogs give nothing to ask.
type Built = Option<(Vec<(String, String)>, Option<serde_json::Value>)>;

fn one(prompt: String, schema: serde_json::Value) -> Built {
    Some((vec![("Pregunta".to_string(), prompt)], Some(schema)))
}

/// The question `id` built over the invented document, each value that
/// changes in each case written by `part`. `conn`, when there is a base:
/// what people decided about the types and the language of the archive.
fn built(id: &str, conn: Option<&Connection>, part: Part) -> Result<Built, String> {
    Ok(match id {
        "knowledge" => {
            let (prompt, schema) = extract::example_question(part);
            one(prompt, schema)
        }
        "type" => {
            let (prompt, schema) = super::similar::classify::example_type_question(conn, part)?;
            one(prompt, schema)
        }
        "data" => super::similar::classify::example_data_question(part).and_then(|(prompt, schema)| one(prompt, schema)),
        "identity" => {
            let (prompt, schema) = resolve::example_question(part);
            one(prompt, schema)
        }
        "synthesis" => {
            let language = match conn {
                Some(conn) => pages::wiki_language(conn)?,
                None => None,
            };
            let (prompt, schema) = pages::example_question(language.as_deref(), part);
            one(prompt, schema)
        }
        "matter" => {
            let (prompt, schema) = matters::example_question(part);
            one(prompt, schema)
        }
        "recognition" => Some((vec![("Pregunta".to_string(), crate::source_volume::RECOGNITION_PROMPT.to_string())], None)),
        "chat" => {
            let built = chat_example(part);
            Some((vec![("Texto de sistema".to_string(), built.system), ("Mensaje".to_string(), built.user)], None))
        }
        other => return Err(format!("Unknown question '{other}'")),
    })
}

/// What the chat sends with a provider of the internet, over the invented
/// archive: `build_agent_context`, as the agent calls it.
fn chat_example(part: Part) -> crate::agent::context::BuiltAgentContext {
    use crate::agent::context::{build_agent_context, AgentContextInput, ProjectContext};
    use crate::agent::types::{AgentMode, AgentReference, AgentSkillMode, AgentToolOptions};
    let project = ProjectContext {
        overview: Some(part(OVERVIEW, example::OVERVIEW)),
        schema: None,
        agent_workspace: part(WORKSPACE, &crate::agent::workspace::agent_workspace_display(example::PROJECT)),
    };
    let router = crate::agent::router::route_query(example::REQUEST, AgentMode::Standard, &AgentToolOptions::default());
    // What base.search finds of the invented letter (`base_tools.rs`).
    let found = AgentReference {
        title: part(FOUND, example::FILE),
        path: part(FOUND, example::FOUND_PATH),
        kind: "source".to_string(),
        snippet: Some(part(FOUND, example::DEBT_QUOTE)),
        score: Some(1.0),
        knowledge_context: None,
    };
    build_agent_context(AgentContextInput {
        query: &part(REQUEST, example::REQUEST),
        project: &project,
        router: &router,
        history: &[],
        skills: &[],
        skill_mode: AgentSkillMode::Auto,
        references: &[found],
        retrieval_summary: &part(FOUND, example::FOUND),
        explicit_files: &[],
        documents: &[],
    })
}

/// A text built with `marked`, cut into what is the same in every case and
/// what changes.
fn segments(text: &str) -> Vec<Segment> {
    let mut out: Vec<Segment> = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(START) {
        if start > 0 {
            out.push(Segment { text: rest[..start].to_string(), part: None });
        }
        let after = &rest[start + START.len_utf8()..];
        let Some(sep) = after.find(SEP) else { break };
        let kind = &after[..sep];
        let value_and_rest = &after[sep + SEP.len_utf8()..];
        let end = value_and_rest.find(END).unwrap_or(value_and_rest.len());
        out.push(Segment { text: value_and_rest[..end].to_string(), part: Some(kind_label(kind).to_string()) });
        rest = value_and_rest.get(end + END.len_utf8()..).unwrap_or_default();
    }
    if !rest.is_empty() {
        out.push(Segment { text: rest.to_string(), part: None });
    }
    out
}

/// The version of a question name, as a number: what it is ordered by.
fn version_number(version: &str) -> i64 {
    version.rsplit('-').next().and_then(|n| n.parse().ok()).unwrap_or(0)
}

/// How many questions the base keeps of each task and version.
fn kept_counts(conn: &Connection) -> Result<HashMap<String, Vec<VersionCount>>, String> {
    let mut statement = conn
        .prepare("SELECT task, prompt_version, count(*) FROM model_answer GROUP BY task, prompt_version")
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))
        .map_err(sql_err)?;
    let mut out: HashMap<String, Vec<VersionCount>> = HashMap::new();
    for row in rows {
        let (task, version, count) = row.map_err(sql_err)?;
        out.entry(task).or_default().push(VersionCount { version, count });
    }
    Ok(out)
}

fn catalog_use(file: CatalogFile, folder: Option<&Path>) -> CatalogUse {
    let path = folder.map(|folder| folder.join(file.file()));
    let installed = file.installed_from();
    CatalogUse {
        file: file.file().to_string(),
        name: file.name().to_string(),
        from_folder: installed.is_some(),
        folder_path: installed.or(path.as_deref()).map(|p| p.to_string_lossy().to_string()),
        unused_file: installed.is_none() && path.is_some_and(|p| p.is_file()),
    }
}

/// Why a question is not asked now, from the engine of its service.
fn engine_note(definition: &Definition, engine: Option<&EngineChoice>) -> Option<String> {
    let service = match definition.service {
        Service::Ask => "«Preguntar al modelo»",
        Service::Recognize => "«Reconocer escaneos»",
        Service::Research => "«Investigar en internet»",
    };
    match engine {
        None if definition.id == "chat" => Some(format!("Sin motor en {service}, el chat no contesta.")),
        None => Some(format!("Sin motor en {service}, esta pregunta no se hace.")),
        // The chat runs in the app, with any engine.
        Some(_) if definition.id == "chat" => None,
        Some(choice) if !services::runs_in_base(definition.service, &choice.engine) => {
            Some("La base no pregunta con este motor: hasta elegir Codex, esta pregunta no se hace.".to_string())
        }
        Some(_) => None,
    }
}

/// Every question the base and the chat ask the model, for the screen.
/// `base`, when there is one: the engines of the registry, what people
/// decided about the types, the language of the archive and how many
/// questions it keeps. `folder`: the data folder (`…/docbase`), where a
/// catalog can be replaced.
pub fn model_questions(base: Option<&DocBase>, folder: Option<&Path>) -> Result<Vec<ModelQuestion>, String> {
    let conn = base.map(DocBase::conn);
    let conn = conn.as_deref();
    let (registry, counts) = match conn {
        Some(conn) => (Some(services::ai_services(conn)?.0), kept_counts(conn)?),
        None => (None, HashMap::new()),
    };
    let mut out = Vec::new();
    for definition in DEFINITIONS {
        let engine = registry.as_ref().and_then(|r| r.service(definition.service).engine.clone());
        let (example, answer_shape, no_example) = match built(definition.id, conn, &marked)? {
            Some((texts, schema)) => (
                texts.into_iter().map(|(title, text)| Sent { title, segments: segments(&text) }).collect(),
                schema.map(|s| serde_json::to_string_pretty(&s).unwrap_or_default()),
                None,
            ),
            None => (Vec::new(), None, Some("Ningún tipo del catálogo en uso tiene datos obligatorios: esta pregunta no se hace.".to_string())),
        };
        let mut kept_by_version = definition.task.and_then(|task| counts.get(task)).cloned().unwrap_or_default();
        kept_by_version.sort_by_key(|v| (Some(v.version.as_str()) != definition.version, -version_number(&v.version)));
        out.push(ModelQuestion {
            id: definition.id.to_string(),
            name: definition.name.to_string(),
            purpose: definition.purpose.to_string(),
            when: definition.when.to_string(),
            note: definition.note.map(str::to_string),
            version: definition.version.map(str::to_string),
            catalogs: definition.catalogs.iter().map(|c| catalog_use(*c, folder)).collect(),
            service: definition.service.id().to_string(),
            engine_note: registry.as_ref().and_then(|_| engine_note(definition, engine.as_ref())),
            engine,
            example,
            no_example,
            answer_shape,
            kept: definition.task.is_some(),
            kept_by_version,
        });
    }
    Ok(out)
}

impl DocBase {
    /// The last question `id` of its current version the base keeps, with
    /// its whole text; none when it keeps none of that version, or does not
    /// keep that question.
    pub fn last_model_question(&self, id: &str) -> Result<Option<AskedQuestion>, String> {
        let definition = DEFINITIONS.iter().find(|d| d.id == id).ok_or_else(|| format!("Unknown question '{id}'"))?;
        let (Some(task), Some(version)) = (definition.task, definition.version) else {
            return Ok(None);
        };
        self.conn()
            .query_row(
                "SELECT prompt_version, engine, asked_at, prompt FROM model_answer
                 WHERE task = ?1 AND prompt_version = ?2 ORDER BY asked_at DESC LIMIT 1",
                params![task, version],
                |row| Ok(AskedQuestion { version: row.get(0)?, engine: row.get(1)?, asked_at: row.get(2)?, prompt: row.get(3)? }),
            )
            .optional()
            .map_err(sql_err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn without_marks(text: &str) -> String {
        segments(text).into_iter().map(|s| s.text).collect()
    }

    fn base_with_a_secret() -> DocBase {
        let base = DocBase::in_memory().unwrap();
        // A question the base kept, with the text of a document: it never
        // reaches the examples, only the last question asked.
        base.conn()
            .execute(
                "INSERT INTO model_answer (question, task, prompt_version, engine, content_id, prompt, answer, asked_at)
                 VALUES ('k1', 'knowledge', ?1, 'codex-cli', 1, 'Texto del documento: Secreto Inventado', '{}', 5),
                        ('k0', 'knowledge', 'conocimiento-1', 'codex-cli', 1, 'Una pregunta vieja', '{}', 9),
                        ('k2', 'knowledge', ?1, 'codex-cli', 2, 'Texto del documento: Otro Secreto', '{}', 7)",
                [extract::KNOWLEDGE_PROMPT],
            )
            .unwrap();
        base
    }

    #[test]
    fn each_question_is_built_by_the_functions_of_the_reading_and_only_marks_what_changes() {
        for with_base in [false, true] {
            let base = with_base.then(base_with_a_secret);
            let conn = base.as_ref().map(DocBase::conn);
            for definition in DEFINITIONS {
                let as_is = built(definition.id, conn.as_deref(), &plain).unwrap();
                let with_marks = built(definition.id, conn.as_deref(), &marked).unwrap();
                let (Some((as_is, schema)), Some((with_marks, _))) = (as_is, with_marks) else {
                    panic!("{} has no example", definition.id);
                };
                assert_eq!(as_is.len(), with_marks.len(), "{}", definition.id);
                for ((title, text), (_, marked_text)) in as_is.iter().zip(&with_marks) {
                    assert!(!text.contains(START) && !text.contains(SEP) && !text.contains(END), "{} {title}", definition.id);
                    // The marks take nothing away and add nothing.
                    assert_eq!(&without_marks(marked_text), text, "{} {title}", definition.id);
                }
                // Each question that carries a document marks it.
                let shown: Vec<Segment> = with_marks.iter().flat_map(|(_, text)| segments(text)).collect();
                let parts: Vec<&str> = shown.iter().filter_map(|s| s.part.as_deref()).collect();
                match definition.id {
                    "knowledge" | "type" | "data" => {
                        assert!(parts.contains(&kind_label(TEXT)), "{}: {parts:?}", definition.id);
                        assert!(parts.contains(&kind_label(FILE)), "{}: {parts:?}", definition.id);
                        assert!(schema.is_some(), "{}", definition.id);
                    }
                    "identity" => assert!(parts.contains(&kind_label(PRINCIPAL)) && parts.contains(&kind_label(CANDIDATE))),
                    "synthesis" => assert!(parts.contains(&kind_label(CLAIM)) && parts.contains(&kind_label(QUOTE))),
                    "matter" => assert!(parts.contains(&kind_label(DOCUMENT)) && parts.contains(&kind_label(MATTER))),
                    "chat" => assert!(parts.contains(&kind_label(REQUEST))),
                    "recognition" => assert!(parts.is_empty()),
                    other => panic!("an example without a check: {other}"),
                }
            }
        }
    }

    #[test]
    fn each_question_comes_with_its_version_and_without_anything_of_the_base() {
        let base = base_with_a_secret();
        let questions = model_questions(Some(&base), None).unwrap();
        let ids: Vec<&str> = questions.iter().map(|q| q.id.as_str()).collect();
        assert_eq!(ids, ["knowledge", "type", "data", "identity", "synthesis", "matter", "recognition", "chat"]);
        let version = |id: &str| questions.iter().find(|q| q.id == id).unwrap().version.clone();
        assert_eq!(version("knowledge").as_deref(), Some(extract::KNOWLEDGE_PROMPT));
        assert_eq!(version("type").as_deref(), Some(TYPE_PROMPT));
        assert_eq!(version("data").as_deref(), Some(DATA_PROMPT));
        assert_eq!(version("identity").as_deref(), Some(resolve::IDENTITY_PROMPT));
        assert_eq!(version("synthesis").as_deref(), Some(pages::SYNTHESIS_PROMPT));
        assert_eq!(version("matter").as_deref(), Some(matters::MATTER_PROMPT));
        assert_eq!(version("recognition"), None);
        assert_eq!(version("chat"), None);
        for question in &questions {
            assert!(!question.example.is_empty(), "{}", question.id);
            let words: String = question.example.iter().flat_map(|s| s.segments.iter()).map(|s| s.text.as_str()).collect();
            // Nothing of what the base keeps reaches an example.
            assert!(!words.contains("Secreto"), "{}", question.id);
            // The invented document is what is read.
            if ["knowledge", "type", "data"].contains(&question.id.as_str()) {
                assert!(words.contains(example::COMPANY) && words.contains(example::FILE), "{}", question.id);
            }
            assert_eq!(question.kept, question.version.is_some(), "{}", question.id);
        }
        // Of the reading: how many of each version, the current one first.
        let reading = questions.iter().find(|q| q.id == "knowledge").unwrap();
        assert_eq!(
            reading.kept_by_version,
            vec![
                VersionCount { version: extract::KNOWLEDGE_PROMPT.into(), count: 2 },
                VersionCount { version: "conocimiento-1".into(), count: 1 },
            ]
        );
        // Without an engine in the registry, the base says it does not ask.
        assert!(reading.engine.is_none());
        assert!(reading.engine_note.as_deref().unwrap().contains("no se hace"));
        // The catalogs that come with the app are in use.
        assert!(reading.catalogs.iter().all(|c| !c.from_folder && !c.unused_file));
        assert_eq!(reading.catalogs.len(), 2);
    }

    #[test]
    fn the_last_question_asked_is_the_latest_of_the_current_version() {
        let base = base_with_a_secret();
        let last = base.last_model_question("knowledge").unwrap().unwrap();
        assert_eq!(last.version, extract::KNOWLEDGE_PROMPT);
        assert_eq!(last.asked_at, 7);
        assert_eq!(last.prompt, "Texto del documento: Otro Secreto");
        assert_eq!(base.last_model_question("identity").unwrap(), None);
        assert_eq!(base.last_model_question("chat").unwrap(), None);
        assert!(base.last_model_question("otra").is_err());
    }

    #[test]
    fn a_catalog_file_in_the_data_folder_that_is_not_in_use_is_told() {
        let dir = super::super::testing::TempDir::new();
        dir.write("knowledge.json", b"{}");
        let shown = catalog_use(CatalogFile::Knowledge, Some(dir.path()));
        assert!(!shown.from_folder);
        assert!(shown.unused_file);
        assert!(shown.folder_path.unwrap().ends_with("knowledge.json"));
        assert!(!catalog_use(CatalogFile::Types, Some(dir.path())).unused_file);
    }

    #[test]
    fn marks_become_segments() {
        let text = format!("A {} B", marked(TEXT, "uno dos"));
        assert_eq!(
            segments(&text),
            vec![
                Segment { text: "A ".into(), part: None },
                Segment { text: "uno dos".into(), part: Some(kind_label(TEXT).into()) },
                Segment { text: " B".into(), part: None },
            ]
        );
    }
}

