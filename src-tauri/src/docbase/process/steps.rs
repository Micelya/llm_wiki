//! The 18 steps of the process, in order (A1 of 2 nov): what each one is,
//! and how many of its elements are left.
//!
//! Every step is told the same way: an element is **pending** when the
//! seal it keeps (what its result was made with) is not the seal of today,
//! and it is not set aside; the step is complete when none is pending. An
//! element **set aside** waits for a known reason (a failure until its next
//! attempt, a folder not connected, the asking held) and holds back only
//! what is its own: it is shown, and does not hold the step.
//!
//! The counts reuse the queries the base already had (`rules_left`,
//! `parts_left`, `zones_left`, the signature and the families, the type,
//! the reading): nothing here does the work of a step.

use std::collections::BTreeSet;

use rusqlite::{params, Connection};
use serde::Serialize;

use super::super::{excluded_sql, sql_err};
use crate::source_volume::page_shape::SHAPES_ENGINE;

/// Whether a step goes over the files or over the documents (D-718).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Level {
    File,
    Document,
}

/// Whether a step works without AI, asks the model, or one or the other
/// as the configuration says (the reading of the scans: the engine of this
/// computer, or one in the cloud).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Class {
    Local,
    Asks,
    BySettings,
}

/// The steps, in the order the process looks at them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    Inventory = 1,
    FileHash,
    Text,
    Scans,
    Shapes,
    Zones,
    CutRules,
    CutQuestion,
    Parts,
    Signature,
    Similar,
    Type,
    TypeData,
    TemplateFields,
    Reading,
    Identity,
    Matters,
    Pages,
}

/// What is known of a step without doing it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct About {
    pub step: Step,
    pub number: u8,
    pub key: &'static str,
    /// Its name, as «En curso» says it.
    pub name: &'static str,
    pub level: Level,
    pub class: Class,
    /// What its elements are.
    pub elements: &'static str,
    /// The seal: what its result keeps of what it was made with.
    pub seal: &'static str,
    /// What makes it incomplete again.
    pub undone_by: &'static str,
    /// The stages of the record of failures (`failures.rs`) of its
    /// elements: a failure of these sets an element aside.
    pub failure_stages: &'static [&'static str],
    /// A step after the type: where the documents of a type of low
    /// confidence are set aside (decision 5 of 2 nov; `low_confidence`).
    pub after_the_type: bool,
}

pub const STEPS: [Step; 18] = [
    Step::Inventory,
    Step::FileHash,
    Step::Text,
    Step::Scans,
    Step::Shapes,
    Step::Zones,
    Step::CutRules,
    Step::CutQuestion,
    Step::Parts,
    Step::Signature,
    Step::Similar,
    Step::Type,
    Step::TypeData,
    Step::TemplateFields,
    Step::Reading,
    Step::Identity,
    Step::Matters,
    Step::Pages,
];

impl Step {
    pub fn number(self) -> u8 {
        self as u8
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub fn from_number(number: u8) -> Option<Step> {
        STEPS.get((number as usize).checked_sub(1)?).copied()
    }

    pub fn about(self) -> About {
        let (key, name, level, class, elements, seal, undone_by, failure_stages): (_, _, _, _, _, _, _, &'static [&'static str]) =
            match self {
                Step::Inventory => (
                    "inventory",
                    "Inventario de las carpetas",
                    Level::File,
                    Class::Local,
                    "las carpetas conectadas",
                    "la hora de su último recorrido, frente a la del último cambio que avisó el vigilante o pidió una persona",
                    "el vigilante, una carpeta nueva, «Volver a procesar», excluir o volver a incluir algo, el barrido de resguardo",
                    &["sweep"],
                ),
                Step::FileHash => (
                    "file-hash",
                    "Huella del archivo",
                    Level::File,
                    Class::Local,
                    "los lugares de tipo archivo, no excluidos",
                    "la huella de sus bytes (`hash_state`)",
                    "un archivo nuevo o cambiado",
                    &[],
                ),
                Step::Text => (
                    "text",
                    "Texto",
                    Level::File,
                    Class::Local,
                    "las versiones",
                    "su estado y el extractor que hizo el texto",
                    "una versión nueva, o un extractor nuevo",
                    &[],
                ),
                Step::Scans => (
                    "scans",
                    "Lectura de escaneos",
                    Level::File,
                    Class::BySettings,
                    "las versiones que esperan su reconocimiento",
                    "el texto reconocido de sus páginas",
                    "un texto nuevo con páginas sin texto, o una persona que pide leer de nuevo",
                    &["recognition"],
                ),
                Step::Shapes => (
                    "shapes",
                    "Ficha de las páginas",
                    Level::File,
                    Class::Local,
                    "los PDF leídos",
                    "el motor de fichas",
                    "un PDF nuevo, o un motor de fichas nuevo",
                    &["shapes"],
                ),
                Step::Zones => (
                    "zones",
                    "Lectura por zonas",
                    Level::File,
                    Class::Local,
                    "los PDF y las imágenes (las páginas que ya leyó el paso de los escaneos no se leen de nuevo)",
                    "su lectura por zonas (un cambio de umbrales no vuelve a leer, D-747)",
                    "un PDF o una imagen nuevos",
                    &["zones"],
                ),
                Step::CutRules => (
                    "cut-rules",
                    "Corte con reglas",
                    Level::File,
                    Class::Local,
                    "los PDF de dos páginas o más",
                    "la versión de las reglas y el sello del texto o de las zonas",
                    "un PDF nuevo, reglas nuevas, otro texto de sus páginas",
                    &["cut"],
                ),
                Step::CutQuestion => (
                    "cut-question",
                    "Pregunta del corte",
                    Level::File,
                    Class::Asks,
                    "los saltos dudosos y los tramos que la app pregunta sola, salvo los que una persona ya decidió",
                    "la respuesta guardada para la clave de sus páginas",
                    "un corte nuevo",
                    &["cut-question"],
                ),
                Step::Parts => (
                    "parts",
                    "Partes",
                    Level::File,
                    Class::Local,
                    "los archivos cortados",
                    "los saltos, las respuestas y las decisiones de una persona",
                    "una respuesta del corte, una persona que corta o une",
                    &["parts"],
                ),
                Step::Signature => (
                    "signature",
                    "Firma",
                    Level::Document,
                    Class::Local,
                    "los documentos",
                    "la hora de su texto y el motor de firmas",
                    "un documento nuevo, el texto de una parte que cambia",
                    &["similar"],
                ),
                Step::Similar => (
                    "similar",
                    "Parecidos y familias",
                    Level::Document,
                    Class::Local,
                    "los documentos firmados",
                    "la huella de sus datos clave al medirlos",
                    "una firma nueva; una respuesta que cambia partes, identificadores, montos o fechas; una decisión sobre un par",
                    &[],
                ),
                Step::Type => (
                    "type",
                    "Tipo",
                    Level::Document,
                    Class::Asks,
                    "las unidades: una familia, o un documento suelto con palabras suficientes",
                    "la base de su unidad (sus miembros, sus textos, sus marcas y el catálogo)",
                    "un cambio en su familia (no un miembro nuevo, que hereda el tipo, decisión 3), un texto nuevo, un catálogo nuevo",
                    &["types"],
                ),
                Step::TypeData => (
                    "type-data",
                    "Datos del tipo",
                    Level::Document,
                    Class::Asks,
                    "los documentos con tipo",
                    "la base de sus datos y la respuesta a su pregunta",
                    "un tipo nuevo",
                    &["data"],
                ),
                Step::TemplateFields => (
                    "template-fields",
                    "Nombres de los campos de las plantillas",
                    Level::Document,
                    Class::Asks,
                    "las plantillas",
                    "la respuesta a la pregunta de sus campos",
                    "una familia de la misma plantilla que cambia",
                    &["templates"],
                ),
                Step::Reading => (
                    "reading",
                    "Lectura de entidades",
                    Level::Document,
                    Class::Asks,
                    "los documentos",
                    "el texto, las instrucciones y el control con que se leyó",
                    "un documento nuevo, el pedido de una persona",
                    &["reading"],
                ),
                Step::Identity => (
                    "identity",
                    "Identidad",
                    Level::Document,
                    Class::Asks,
                    "la resolución",
                    "las lecturas y decisiones de su última resolución completa",
                    "lecturas nuevas, decisiones de una persona",
                    &["resolve"],
                ),
                Step::Matters => (
                    "matters",
                    "Asuntos",
                    Level::Document,
                    Class::Asks,
                    "los documentos sin asunto que comparten partes con uno",
                    "la respuesta guardada",
                    "identidades nuevas",
                    &["matters"],
                ),
                Step::Pages => (
                    "pages",
                    "Páginas",
                    Level::Document,
                    Class::Asks,
                    "las páginas marcadas para escribir y las síntesis que faltan",
                    "lo que muestra la página",
                    "cualquier cambio en lo que la página muestra",
                    &["pages"],
                ),
            };
        About {
            step: self,
            number: self.number(),
            key,
            name,
            level,
            class,
            elements,
            seal,
            undone_by,
            failure_stages,
            after_the_type: self > Step::Type,
        }
    }
}

/// What part of the base a count looks at: everything, one document, one
/// file or one folder. A document takes also the steps of the file it is
/// in: what a person asks for waits only for the steps before of its own
/// document (decision 2 of 2 nov).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Scope {
    All,
    /// A document, by its version.
    Document(i64),
    /// A file, by the hash of its bytes.
    File(String),
    /// A folder of an origin, with forward slashes; empty, the whole origin.
    Folder { origin: String, folder: String },
}

fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

impl Scope {
    /// SQL: the hashes of the files of the scope (`None`: all).
    fn files_sql(&self) -> Option<String> {
        match self {
            Scope::All => None,
            Scope::Document(version) => Some(format!(
                "SELECT sa.resource_hash FROM appearance sa WHERE sa.version_id = {version}
                 UNION SELECT sf.hash FROM file sf WHERE sf.version_id = {version}"
            )),
            Scope::File(hash) => Some(format!("SELECT {}", quoted(hash))),
            Scope::Folder { origin, .. } => {
                Some(format!("SELECT sl.file_hash FROM location sl WHERE sl.file_hash IS NOT NULL AND {}", self.location_where("sl", origin)))
            }
        }
    }

    /// SQL over a location `alias` of the folder `origin` of the scope.
    fn location_where(&self, alias: &str, origin: &str) -> String {
        let folder = match self {
            Scope::Folder { folder, .. } => folder.trim_matches('/'),
            _ => "",
        };
        let inside = if folder.is_empty() {
            "1".to_string()
        } else {
            format!("substr({alias}.rel_path, 1, length({f}) + 1) = {f} || '/'", f = quoted(folder))
        };
        format!("{alias}.origin_id = {} AND {inside}", quoted(origin))
    }

    /// SQL: the version in `column` is of the scope: the documents, and
    /// the versions of the whole files they are in.
    pub(crate) fn version_sql(&self, column: &str) -> String {
        match (self, self.files_sql()) {
            (Scope::All, _) | (_, None) => "1".to_string(),
            (Scope::Document(version), Some(files)) => format!(
                "({column} = {version} OR {column} IN (SELECT sv.version_id FROM file sv WHERE sv.hash IN ({files})))"
            ),
            (_, Some(files)) => format!(
                "({column} IN (SELECT sv.version_id FROM file sv WHERE sv.hash IN ({files}))
                  OR {column} IN (SELECT sw.version_id FROM appearance sw WHERE sw.resource_hash IN ({files})))"
            ),
        }
    }

    /// SQL: the file hash in `column` is of the scope.
    pub(crate) fn file_sql(&self, column: &str) -> String {
        match self.files_sql() {
            None => "1".to_string(),
            Some(files) => format!("{column} IN ({files})"),
        }
    }

    /// SQL over a location `l`.
    fn location_sql(&self) -> String {
        match self {
            Scope::All => "1".to_string(),
            Scope::Folder { origin, .. } => self.location_where("l", origin),
            _ => self.file_sql("l.file_hash"),
        }
    }

    /// SQL over an origin `o`.
    fn origin_sql(&self) -> String {
        match self {
            Scope::All => "1".to_string(),
            Scope::Folder { origin, .. } => format!("o.id = {}", quoted(origin)),
            _ => format!("o.id IN (SELECT so.origin_id FROM location so WHERE {})", self.file_sql("so.file_hash")),
        }
    }

    /// SQL over a failure `fz`: it is about something of the scope.
    fn failure_sql(&self) -> String {
        match self {
            Scope::All => "1".to_string(),
            Scope::Folder { origin, .. } if self.files_sql().is_some() => format!(
                "(({}) OR ({}) OR (fz.item_kind = 'origin' AND fz.item = {}))",
                self.file_sql("fz.file_hash"),
                self.version_sql("fz.version_id"),
                quoted(origin)
            ),
            _ => format!("(({}) OR ({}))", self.file_sql("fz.file_hash"), self.version_sql("fz.version_id")),
        }
    }

    /// The versions of the scope (`None`: all), for the steps counted in
    /// Rust.
    pub(crate) fn versions(&self, conn: &Connection) -> Result<Option<BTreeSet<i64>>, String> {
        if *self == Scope::All {
            return Ok(None);
        }
        let mut statement = conn.prepare(&format!("SELECT c.id FROM version c WHERE {}", self.version_sql("c.id"))).map_err(sql_err)?;
        let rows = statement.query_map([], |row| row.get::<_, i64>(0)).map_err(sql_err)?;
        Ok(Some(rows.collect::<Result<_, _>>().map_err(sql_err)?))
    }
}

/// How many elements of a step are left.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Count {
    /// Elements whose seal is not today's, not set aside.
    pub pending: usize,
    /// Elements that wait for a known reason; they do not hold the step.
    pub set_aside: usize,
    /// Of the set aside, those that wait for a person (a scan whose reading
    /// failed for good, a document that went back too often) and those of
    /// a type of low confidence; the rest wait for a failure to be tried
    /// again or for their folder («En curso» says each, A3 of 2 nov).
    pub person: usize,
    pub low: usize,
    /// Of the pending, those the step closes without asking (the marks
    /// or the inheritance of the type, the check of a reading kept).
    pub without_asking: usize,
    /// Whether the base can count the step by itself. The identities, the
    /// matters and the syntheses of the pages know what they have left only
    /// once a pass goes through them (D-452): when nothing they mark is
    /// pending, they say they do not know.
    pub known: bool,
}

impl Count {
    fn of(pending: usize, set_aside: usize) -> Count {
        Count { pending, set_aside, person: 0, low: 0, without_asking: 0, known: true }
    }
}

/// SQL: no failure is open, of any class, about the version `version` (an
/// SQL value) or about a file it is in: what failed is set aside, and does
/// not leave its step incomplete.
pub(crate) fn free_sql(version: &str) -> String {
    free_sql_except(version, "")
}

/// The same, except the failures of `stage` (the step of the scans looks
/// at its own with their next attempt, `recognize::scan_free_sql`).
pub(crate) fn free_sql_except(version: &str, stage: &str) -> String {
    format!(
        "NOT EXISTS (SELECT 1 FROM failure fb WHERE fb.state <> 'resolved' AND fb.stage <> '{stage}' AND (
             (fb.item_kind = 'version' AND fb.item = CAST({version} AS TEXT))
             OR fb.file_hash IN (SELECT fc.hash FROM file fc WHERE fc.version_id = {version})
             OR fb.file_hash IN (SELECT ab.resource_hash FROM appearance ab WHERE ab.version_id = {version})))"
    )
}

/// What holds back every question to the model now, if anything does: the
/// automatic reading off, no engine chosen, or the engine paused after a
/// failure of the whole engine (D-714). Its pending elements are set aside.
pub(crate) fn asking_held(conn: &Connection, reading_on: bool) -> Result<Option<&'static str>, String> {
    if !reading_on {
        return Ok(Some("la lectura automática está apagada"));
    }
    if super::super::services::ask_engine(conn)?.is_none() {
        return Ok(Some("falta elegir el motor de IA"));
    }
    let paused: i64 = conn
        .query_row(
            &format!(
                "SELECT count(*) FROM failure WHERE item_kind = 'engine' AND scope = 'engine' AND state <> 'resolved'
                   AND (next_at IS NULL OR next_at > {})",
                super::super::failures::NOW_SQL
            ),
            [],
            |row| row.get(0),
        )
        .map_err(sql_err)?;
    Ok((paused > 0).then_some("el motor está en pausa"))
}

/// The class a step has now: the reading of the scans is local work with
/// the reader of this computer, and a question with one in the cloud, as
/// the reader chosen in Ajustes says (E2 of 2 nov, D-764). Then it has its
/// failures, its round and its quota like any question of the process.
pub(crate) fn class_now(conn: &Connection, step: Step) -> Result<Class, String> {
    let about = step.about();
    if about.class != Class::BySettings {
        return Ok(about.class);
    }
    let chosen = super::super::services::recognition_choice(conn)?;
    Ok(match chosen {
        Some(config) if !super::super::services::is_local(&config.engine) => Class::Asks,
        _ => Class::Local,
    })
}

/// The documents of a type of low confidence (decision 5 of 2 nov, D-769):
/// their share of safe words is below the threshold of the type of low
/// confidence or below the one of «no legible», now (`safe_words.rs`, E3
/// of 2 nov). They serve only for queries: their readings exist, marked
/// (E6 of 2 nov, over E3, as the principal decided at 04:55).
/// - **Read like the others,** because the model is never skipped: the
///   type, the data of the type (`classify::pass5_refresh`) and the reading
///   of entities (`extract::knowledge_pending`). «En curso» shows them with
///   their mark and reason (`safe_words::reasons`).
/// - **Taken by nothing derived:** the identities (their mentions,
///   `resolve::load_rows`, and left out as an excluded document is,
///   `changes::left_out`), the matters (`matters::documents`), the pages
///   (their page of document, `pages::family::families`, and every claim,
///   which comes from the mentions), the inheritance of the type (a family
///   does not take their type, nor are they described in its question,
///   `classify::load_units`, D-766), the fields of the templates whose
///   principal is one (`naming::templates_naming_run`), the reading a copy
///   inherits (`extract::copy_originals`), and the vectors when they come
///   (C1 and C2): this is their filter.
/// - **Shown marked in the queries:** the search (`search.rs`) and the
///   sheet of the document say they are of low confidence.
pub(crate) fn low_confidence(conn: &Connection) -> Result<BTreeSet<i64>, String> {
    super::super::safe_words::low_confidence(conn)
}

/// SQL over an origin `o`: its inventory is incomplete. It was never swept
/// (and is not known to be out of reach), or something changed in it after
/// its last sweep: the watcher heard it, or a person asked to process it
/// again (`origin.changed_at`, A3 of 2 nov). A sweep that failed sets it
/// aside, with its failure, until the next sweep of backup.
pub(crate) const INVENTORY_LEFT: &str = "((o.last_scan_at IS NULL AND o.reachable IS NOT 0) OR o.changed_at >= o.last_scan_at)
    AND NOT EXISTS (SELECT 1 FROM failure fo WHERE fo.item_kind = 'origin' AND fo.item = o.id AND fo.stage = 'sweep' AND fo.state <> 'resolved')";

fn count(conn: &Connection, sql: &str) -> Result<usize, String> {
    conn.query_row(sql, [], |row| row.get::<_, i64>(0)).map(|n| n.max(0) as usize).map_err(sql_err)
}

/// Open failures of the stages of `step` about something of the scope.
fn failures_of(conn: &Connection, step: Step, scope: &Scope) -> Result<usize, String> {
    let stages = step.about().failure_stages;
    if stages.is_empty() {
        return Ok(0);
    }
    let list: Vec<String> = stages.iter().map(|s| quoted(s)).collect();
    count(
        conn,
        &format!(
            "SELECT count(*) FROM failure fz WHERE fz.state <> 'resolved' AND fz.item_kind NOT IN ('engine', 'thread')
               AND fz.stage IN ({}) AND {}",
            list.join(", "),
            scope.failure_sql()
        ),
    )
}

/// How many elements of `step` are left in `scope`, before the asking is
/// held (`asking_held`): `state` in `mod.rs` sets them aside then.
pub(crate) fn count_step(conn: &Connection, step: Step, scope: &Scope) -> Result<Count, String> {
    let free = free_sql("c.id");
    let versions = scope.version_sql("c.id");
    let excluded = excluded_sql!();
    let failed = || failures_of(conn, step, scope);
    Ok(match step {
        Step::Inventory => {
            let origins = scope.origin_sql();
            Count::of(
                count(conn, &format!("SELECT count(*) FROM origin o WHERE {INVENTORY_LEFT} AND {origins}"))?,
                count(conn, &format!("SELECT count(*) FROM origin o WHERE o.reachable = 0 AND NOT ({INVENTORY_LEFT}) AND {origins}"))?
                    + failed()?,
            )
        }
        Step::FileHash => {
            let left = |reachable: &str| {
                count(
                    conn,
                    &format!(
                        "SELECT count(*) FROM location l JOIN origin o ON o.id = l.origin_id AND {reachable}
                         WHERE l.hash_state = 'pending' AND l.kind = 'file' AND NOT {excluded} AND {}",
                        scope.location_sql()
                    ),
                )
            };
            // A folder not connected: its files wait, set aside.
            Count::of(left("o.reachable = 1")?, left("o.reachable IS NOT 1")?)
        }
        Step::Text => {
            let left = |held: &str| {
                count(
                    conn,
                    &format!(
                        "SELECT count(DISTINCT c.id) FROM version c JOIN file f ON f.version_id = c.id
                         JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done'
                         JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                         WHERE c.state = 'pending' AND NOT {excluded} AND {held} AND {versions}"
                    ),
                )
            };
            Count::of(left(&free)?, left(&format!("NOT {free}"))?)
        }
        Step::Scans => {
            // The scans waiting, and what a person asked to read (again, or
            // with the reader of «Leer con IA»).
            let left = |held: &str| {
                count(
                    conn,
                    &format!(
                        "SELECT count(*) FROM version c
                         WHERE (c.state = 'needs-recognition' OR EXISTS (SELECT 1 FROM recognition_request r WHERE r.version_id = c.id))
                           AND {} AND {held} AND {versions}",
                        super::super::recognize::reachable_sql()
                    ),
                )
            };
            // A scan whose recognition failed for good (`error`) waits for a
            // person; one whose failure holds, until its next attempt.
            let scan_free = super::super::recognize::scan_free_sql();
            Count {
                person: left("c.error IS NOT NULL")?,
                ..Count::of(left(&format!("c.error IS NULL AND {scan_free}"))?, left(&format!("(c.error IS NOT NULL OR NOT ({scan_free}))"))?)
            }
        }
        Step::Shapes => {
            let left = |held: &str| -> Result<usize, String> {
                conn.query_row(
                    &format!(
                        "SELECT count(DISTINCT c.id) FROM version c JOIN file f ON f.version_id = c.id
                         JOIN location l ON l.file_hash = f.hash AND l.hash_state = 'done' AND lower(l.ext) = 'pdf'
                         JOIN origin o ON o.id = l.origin_id AND o.reachable = 1
                         WHERE c.state IN ('processed', 'needs-recognition') AND NOT {excluded} AND {held} AND {versions}
                           AND NOT EXISTS (SELECT 1 FROM derivative d WHERE d.version_id = c.id AND d.kind = ?1 AND d.engine = ?2)"
                    ),
                    params![super::super::page_shapes::SHAPES_KIND, SHAPES_ENGINE],
                    |row| row.get::<_, i64>(0),
                )
                .map(|n| n.max(0) as usize)
                .map_err(sql_err)
            };
            Count::of(left(&free)?, left(&format!("NOT {free}"))?)
        }
        Step::Zones => {
            let thresholds = super::super::thresholds::thresholds(conn)?;
            let bands = super::super::structure::page::Bands { high: thresholds.high, low: thresholds.low };
            Count::of(super::super::structure::zones_left(conn, &format!("{free} AND {versions}"), bands)?, failed()?)
        }
        Step::CutRules => Count::of(super::super::cut::rules_left(conn, &format!("{free} AND {versions}"))?, failed()?),
        Step::CutQuestion => {
            // Every window of the cut by zones is asked by itself (D-720);
            // one whose failure holds is set aside.
            let window_held = super::super::cut::by_zones::window_held_sql();
            let holds = super::super::failures::holds_any_sql("cut-question", "s.resource_hash || '#' || s.after_page");
            let steps = |held: &str| {
                count(
                    conn,
                    &format!(
                        "SELECT count(*) FROM cut_step s WHERE {} AND {} AND {held} AND {}",
                        super::super::cut::WITHOUT_ANSWER,
                        super::super::cut::ASKED_BY_ITSELF,
                        scope.file_sql("s.resource_hash")
                    ),
                )
            };
            let windows = |held: &str| {
                count(
                    conn,
                    &format!(
                        "SELECT count(*) FROM cut_window w WHERE {} AND {held} AND {}",
                        super::super::cut::by_zones::WINDOW_WITHOUT_ANSWER,
                        scope.file_sql("w.resource_hash")
                    ),
                )
            };
            Count::of(
                steps(&format!("NOT {holds}"))? + windows(&format!("NOT {window_held}"))?,
                steps(&holds)? + windows(&window_held)?,
            )
        }
        Step::Parts => Count::of(super::super::cut::parts::parts_left(conn, &format!("{free} AND {versions}"))?, failed()?),
        Step::Signature => Count::of(super::super::similar::store::unsigned_left(conn, &format!("{free} AND {versions}"))?, failed()?),
        Step::Similar => {
            // Marked to measure again, and those whose key data changed
            // since they were measured and nobody marked yet: told from
            // the base, in the same moment an answer is kept.
            let marked = super::super::similar::store::dirty_left(conn, &format!("{free} AND {versions}"))?;
            let only = scope.versions(conn)?;
            // A document set aside for going back too often waits for a
            // person (D-774, point 2).
            let back = super::super::failures::held_versions(conn, super::super::failures::BACK_STAGE)?;
            let (aside, stale): (Vec<i64>, Vec<i64>) = super::super::similar::store::stale_by_key_data_now(conn)?
                .into_iter()
                .filter(|id| only.as_ref().is_none_or(|o| o.contains(id)))
                .partition(|id| back.contains(id));
            Count { person: aside.len(), ..Count::of(marked + stale.len(), aside.len()) }
        }
        Step::Type => {
            let only = scope.versions(conn)?;
            let left = super::super::similar::classify::types_left(conn, only.as_ref())?;
            // A unit whose question a failure holds is set aside (P-533).
            Count { without_asking: left.by_marks + left.by_inheritance + left.by_answer, ..Count::of(left.units, left.held) }
        }
        Step::TypeData => {
            // The documents of low confidence are read too (E6 of 2 nov).
            let only = scope.versions(conn)?;
            let (pending, held) = super::super::similar::classify::data_left(conn, only.as_ref())?;
            Count::of(pending, held)
        }
        Step::TemplateFields => {
            let only = scope.versions(conn)?;
            let low = low_confidence(conn)?;
            let held = super::super::failures::held_versions(conn, "templates")?;
            let (mut pending, mut set_aside, mut of_low) = (0, 0, 0);
            for (principal, waiting) in super::super::similar::naming::waiting_templates(conn)? {
                if waiting == 0 || only.as_ref().is_some_and(|o| !o.contains(&principal)) {
                    continue;
                }
                if low.contains(&principal) || held.contains(&principal) {
                    set_aside += 1;
                    of_low += usize::from(low.contains(&principal));
                } else {
                    pending += 1;
                }
            }
            Count { low: of_low, ..Count::of(pending, set_aside) }
        }
        Step::Reading => {
            let only = scope.versions(conn)?;
            // A document whose reading failed waits for its next attempt,
            // or for a person (P-533): set aside. One of low confidence is
            // read like the others (E6 of 2 nov).
            let held = super::super::failures::held_versions(conn, "reading")?;
            let (mut pending, mut checks, mut set_aside) = (0, 0, 0);
            let standings = super::super::knowledge::extract::standings(conn, &super::super::Visible::All, None)?;
            for standing in standings {
                if only.as_ref().is_some_and(|o| !o.contains(&standing.content_id)) {
                    continue;
                }
                let reading = standing.reading.as_ref();
                let to_check = reading.is_some_and(|r| r.current && r.to_check);
                if reading.is_some_and(|r| r.current) && !to_check {
                    continue;
                }
                // What a person asked to read again is never held.
                let asked_again = reading.is_some_and(|r| r.reread);
                if !to_check && !asked_again && held.contains(&standing.content_id) {
                    set_aside += 1;
                } else {
                    pending += 1;
                    checks += usize::from(to_check);
                }
            }
            Count { without_asking: checks, ..Count::of(pending, set_aside) }
        }
        Step::Identity => {
            let marked = count(conn, "SELECT count(*) FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')")?;
            Count { known: marked > 0, ..Count::of(marked, 0) }
        }
        Step::Matters => Count { known: false, ..Count::default() },
        Step::Pages => {
            let marked = count(conn, "SELECT count(*) FROM knowledge_dirty WHERE scope = 'project'")?;
            Count { known: marked > 0, ..Count::of(marked, 0) }
        }
    })
}

