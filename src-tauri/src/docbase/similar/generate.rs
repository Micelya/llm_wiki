//! A document generated from a template (D-488): a simple Word, without a
//! letterhead, with the template's text and the values a person gave, in
//! the folder «Generados» of the work folder; and its record in the base:
//! the template it came from (its lasting identity, its version and the
//! name it had then), each value with where it came from, and the matter
//! it was made for.
//!
//! The Word keeps the paragraphs and the lines of the template, nothing
//! else of the original: no bold, indents, tables nor letterhead (the
//! template keeps only its text). A field left empty: a wording that
//! changes (an optional clause) or a field that some document of the
//! template leaves empty too is left out; any other is written as its
//! name between brackets, highlighted, for the person to fill it in Word.
//! The fields of a form, which have no place in the text, go at the end,
//! one per line, with their names. A fixed field is written with its
//! fixed value, whatever the screen sent.
//!
//! The base finds the file as any other of the work folder. The record
//! follows it (`generated_settle`): by its place, which keeps it when a
//! person edits the Word, or else by the bytes it was written with. Once
//! the base has its document, it goes into the matter it was generated
//! for, as a person's decision (D-386): generating it for that matter is
//! one. The template is never joined to the matter nor to its people.
//!
//! A generated document is known as such (`generated_contents`): the base
//! never takes it for another document with the same text, it is never the
//! principal of a family nor a document of the template it came from (the
//! template stays joined only to the documents it came from), and with
//! those documents it is never «possibly the same document». Its screen
//! says when and from which template it was generated, and for which
//! matter (`generated_line`); the template lists what was generated with it
//! (`generated_with`).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use docx_rs::{Docx, LineSpacing, Paragraph, Run, RunFonts};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::super::origins::path_inside;
use super::super::{now_ms, sql_err, DocBase, Visible};
use super::changes::Kind;
use super::template::{self, Segment, TemplateField};
use crate::source_volume::file_hash::file_sha256;
use crate::source_volume::{LocalFolderProvider, SourceProvider};

/// The letter of the Word, and its size in half points.
const FONT: &str = "Times New Roman";
const FONT_SIZE: usize = 24;
/// Space after a paragraph, in twentieths of a point; none between its lines.
const PARAGRAPH_AFTER: u32 = 200;
/// The longest name of a file, without its extension.
const FILE_STEM_MAX_CHARS: usize = 120;
/// What a value longer than this keeps (the record is not the document).
const RECORDED_MAX_CHARS: usize = 4000;

/// Where the generated documents go: a folder inside an origin of the base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Place {
    pub origin_id: String,
    /// The folder on disk.
    pub folder: PathBuf,
    /// The same folder inside its origin, with `/`.
    pub rel_folder: String,
}

impl Place {
    /// The place of `folder`, made if needed: the origin that holds it.
    pub fn inside(base: &DocBase, folder: &Path) -> Result<Place, String> {
        fs::create_dir_all(folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
        for origin in base.origins(&Visible::All)? {
            if let Some(rel_folder) = path_inside(&origin.root(), folder) {
                return Ok(Place { origin_id: origin.id, folder: folder.to_path_buf(), rel_folder });
            }
        }
        Err(format!("La carpeta {} no está dentro de ningún origen de la base", folder.display()))
    }
}

/// What a person asks to generate.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GenerateRequest {
    /// The template, by its principal.
    pub principal: i64,
    /// Its name as the screen shows it: the record keeps it.
    pub template_name: String,
    /// The value of each field, by the position of its first place.
    pub values: Vec<GivenValue>,
    /// The matter it is generated for.
    pub matter_id: Option<String>,
    /// The name of the file, without folder; empty: the template's and the date.
    pub file_name: Option<String>,
    /// The document whose data it started with.
    pub from_document: Option<i64>,
}

/// The value of a field, as the screen gives it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GivenValue {
    /// The position of the field (its first place) in the template.
    pub position: i64,
    /// Its name as the screen shows it.
    pub name: String,
    pub value: String,
    /// Where the value came from: `hand` (written or chosen by a person),
    /// `entity` (a person or organization of the base), `card` (what the
    /// base knows of it: its DNI, its address), `matter`, `document` (one
    /// of the template's documents), `date` or `fixed`.
    pub source: String,
    /// What it came from: `entity:<id>`, `matter:<id>`, `content:<id>`.
    pub reference: Option<String>,
}

/// What was generated.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    /// Its record.
    pub id: i64,
    pub path: String,
    pub file_name: String,
    /// Where the base has it already, and its document.
    pub location_id: Option<i64>,
    pub content_id: Option<i64>,
    /// It is in the matter it was generated for.
    pub in_matter: bool,
    /// The fields left empty that the Word shows by name, to be filled.
    pub missing: Vec<String>,
}

/// What a field of the template gets.
#[derive(Debug, Clone, PartialEq)]
enum Filled {
    Value(String),
    /// Nothing at all: an optional field left empty.
    Nothing,
    /// Left empty and needed: the Word shows its name.
    Missing(String),
}

/// A piece of a line of the Word.
#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    Text(String),
    Value(String),
    Missing(String),
}

impl Piece {
    fn text(&self) -> &str {
        match self {
            Piece::Text(text) | Piece::Value(text) | Piece::Missing(text) => text,
        }
    }

    fn text_mut(&mut self) -> &mut String {
        match self {
            Piece::Text(text) | Piece::Value(text) | Piece::Missing(text) => text,
        }
    }
}

/// The paragraphs of the Word, each made of its lines.
pub type Paragraphs = Vec<Vec<Vec<Piece>>>;

/// What goes in each field (`Filled`), and what the record keeps of each
/// one asked for: a field said again takes its first place's; a fixed one,
/// its fixed value.
fn fill_fields(fields: &[TemplateField], given: &[GivenValue]) -> (Vec<Filled>, Vec<GivenValue>) {
    let mut filled: Vec<Filled> = Vec::with_capacity(fields.len());
    let mut recorded = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        if let Some(original) = field.same_as.map(|o| o as usize).filter(|o| *o < index) {
            filled.push(filled[original].clone());
            continue;
        }
        let asked = given.iter().find(|g| g.position == index as i64);
        let name = asked
            .map(|g| g.name.trim().to_string())
            .filter(|n| !n.is_empty())
            .or_else(|| field.name.clone())
            .unwrap_or_else(|| format!("Campo {}", index + 1));
        let (value, source, reference) = if field.fixed {
            (field.fixed_value.clone().unwrap_or_default(), "fixed".to_string(), None)
        } else {
            match asked {
                Some(g) => (g.value.trim().to_string(), g.source.clone(), g.reference.clone()),
                None => (String::new(), "hand".to_string(), None),
            }
        };
        let optional = field.kind == Kind::Wording || field.values.iter().any(|v| v.trim().is_empty());
        filled.push(match (value.is_empty(), optional) {
            (false, _) => Filled::Value(value.clone()),
            (true, true) => Filled::Nothing,
            (true, false) => Filled::Missing(name.clone()),
        });
        recorded.push(GivenValue {
            position: index as i64,
            name,
            value: value.chars().take(RECORDED_MAX_CHARS).collect(),
            source: if source.is_empty() { "hand".to_string() } else { source },
            reference,
        });
    }
    (filled, recorded)
}

/// Writes the lines and paragraphs of the Word from pieces of text.
#[derive(Default)]
struct Lines {
    paragraphs: Paragraphs,
    paragraph: Vec<Vec<Piece>>,
    line: Vec<Piece>,
    /// An optional field left empty was just passed: a punctuation mark
    /// right after it closes up to the word before.
    after_nothing: bool,
}

impl Lines {
    /// Adds text, where one line break ends a line and two or more end a
    /// paragraph.
    fn push(&mut self, piece: fn(String) -> Piece, text: &str) {
        let mut rest = text;
        loop {
            match rest.find('\n') {
                None => {
                    self.append(piece, rest);
                    break;
                }
                Some(at) => {
                    self.append(piece, &rest[..at]);
                    let after = &rest[at..];
                    let breaks = after.trim_start_matches(['\n', '\r', ' ', '\t']);
                    let newlines = after[..after.len() - breaks.len()].matches('\n').count();
                    if newlines >= 2 {
                        self.end_paragraph();
                    } else {
                        self.end_line();
                    }
                    rest = breaks;
                }
            }
        }
    }

    /// Adds text of one line: never two spaces together, nor spaces at the
    /// start of a line.
    fn append(&mut self, piece: fn(String) -> Piece, text: &str) {
        let text: String = text.replace(['\r', '\t'], " ");
        let mut text = text.as_str();
        let ends_in_space = self.line.last().is_some_and(|p| p.text().ends_with(' '));
        if self.line.is_empty() || ends_in_space {
            text = text.trim_start();
        }
        if text.is_empty() {
            return;
        }
        if self.after_nothing && text.starts_with(['.', ',', ';', ':', ')']) {
            self.trim_end();
        }
        self.after_nothing = false;
        // Two spaces inside a text are one.
        let mut clean = String::with_capacity(text.len());
        for c in text.chars() {
            if !(c == ' ' && clean.ends_with(' ')) {
                clean.push(c);
            }
        }
        let next = piece(clean);
        match (self.line.last_mut(), &next) {
            (Some(Piece::Text(last)), Piece::Text(more)) => last.push_str(more),
            _ => self.line.push(next),
        }
    }

    fn nothing(&mut self) {
        self.after_nothing = true;
    }

    fn trim_end(&mut self) {
        while let Some(last) = self.line.last_mut() {
            let kept = last.text().trim_end().len();
            last.text_mut().truncate(kept);
            if last.text().is_empty() {
                self.line.pop();
            } else {
                break;
            }
        }
    }

    fn end_line(&mut self) {
        self.trim_end();
        self.after_nothing = false;
        if !self.line.is_empty() {
            self.paragraph.push(std::mem::take(&mut self.line));
        }
    }

    fn end_paragraph(&mut self) {
        self.end_line();
        if !self.paragraph.is_empty() {
            self.paragraphs.push(std::mem::take(&mut self.paragraph));
        }
    }

    fn finish(mut self) -> Paragraphs {
        self.end_paragraph();
        self.paragraphs
    }
}

/// The text of the document: the template's, with what each field gets,
/// and the fields of a form at the end.
fn paragraphs(segments: &[Segment], fields: &[TemplateField], filled: &[Filled]) -> Paragraphs {
    let mut lines = Lines::default();
    let mut in_text: HashSet<usize> = HashSet::new();
    for segment in segments {
        match segment {
            Segment::Text { text } => lines.push(Piece::Text, text),
            Segment::Field { field } => {
                in_text.insert(*field);
                match filled.get(*field) {
                    Some(Filled::Value(value)) => lines.push(Piece::Value, value),
                    Some(Filled::Missing(name)) => lines.append(Piece::Missing, &format!("[{name}]")),
                    Some(Filled::Nothing) | None => lines.nothing(),
                }
            }
        }
    }
    let form: Vec<usize> = (0..fields.len()).filter(|i| !in_text.contains(i) && fields[*i].same_as.is_none()).collect();
    if !form.is_empty() {
        lines.end_paragraph();
        for index in form {
            let name = match &filled[index] {
                Filled::Missing(name) => name.clone(),
                _ => fields[index].name.clone().or_else(|| fields[index].form_name.clone()).unwrap_or_default(),
            };
            lines.append(Piece::Text, &format!("{name}: "));
            match &filled[index] {
                Filled::Value(value) => lines.push(Piece::Value, value),
                Filled::Missing(name) => lines.append(Piece::Missing, &format!("[{name}]")),
                Filled::Nothing => {}
            }
            lines.end_line();
        }
    }
    lines.finish()
}

/// The Word with these paragraphs; `at` is the moment it was made (ISO).
pub fn word(paragraphs: &Paragraphs, at: &str) -> Result<Vec<u8>, String> {
    let fonts = || RunFonts::new().ascii(FONT).hi_ansi(FONT).cs(FONT).east_asia(FONT);
    let mut docx = Docx::new().default_fonts(fonts()).default_size(FONT_SIZE).created_at(at).updated_at(at);
    for paragraph in paragraphs {
        for (index, line) in paragraph.iter().enumerate() {
            let after = if index + 1 == paragraph.len() { PARAGRAPH_AFTER } else { 0 };
            let mut out = Paragraph::new().line_spacing(LineSpacing::new().before(0).after(after));
            for piece in line {
                let run = Run::new().add_text(piece.text());
                out = out.add_run(match piece {
                    Piece::Missing(_) => run.highlight("yellow"),
                    _ => run,
                });
            }
            docx = docx.add_paragraph(out);
        }
    }
    let mut bytes = Cursor::new(Vec::new());
    docx.build().pack(&mut bytes).map_err(|e| format!("No se pudo armar el Word: {e}"))?;
    Ok(bytes.into_inner())
}

/// A name of a file as Windows takes it: without the signs it does not
/// take, without «.docx» (it is added), not too long, never empty.
pub fn file_stem(wanted: &str) -> String {
    let wanted = wanted.trim();
    let wanted = wanted.strip_suffix(".docx").or_else(|| wanted.strip_suffix(".DOCX")).unwrap_or(wanted);
    let clean: String = wanted
        .chars()
        .map(|c| if c.is_control() || r#"<>:"/\|?*"#.contains(c) { ' ' } else { c })
        .collect();
    let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let short: String = clean.chars().take(FILE_STEM_MAX_CHARS).collect();
    let short = short.trim_end_matches(['.', ' ']).trim_start_matches('.').to_string();
    const RESERVED: &[&str] = &["CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "LPT1", "LPT2", "LPT3"];
    if short.is_empty() {
        "Documento generado".to_string()
    } else if RESERVED.contains(&short.to_uppercase().as_str()) {
        format!("{short} (documento)")
    } else {
        short
    }
}

/// Writes the Word in `folder` as `<stem>.docx`, never over another file: a
/// name already there gets « (2)», « (3)»…
fn write_new(folder: &Path, stem: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    fs::create_dir_all(folder).map_err(|e| format!("No se pudo crear {}: {e}", folder.display()))?;
    let mut n = 1;
    loop {
        let name = if n == 1 { format!("{stem}.docx") } else { format!("{stem} ({n}).docx") };
        let path = folder.join(name);
        match fs::OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                file.write_all(bytes).map_err(|e| format!("No se pudo guardar {}: {e}", path.display()))?;
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && n < 1000 => n += 1,
            Err(error) => return Err(format!("No se pudo guardar {}: {error}", path.display())),
        }
    }
}

/// The name of an active matter, or why a document cannot be generated for it.
fn matter_name(conn: &Connection, matter_id: &str) -> Result<String, String> {
    let found: Option<(String, String)> = conn
        .query_row("SELECT name, state FROM knowledge_matter WHERE id = ?1", [matter_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()
        .map_err(sql_err)?;
    match found {
        Some((name, state)) if state == "active" => Ok(name),
        _ => Err("Lo que se eligió para el documento ya no existe: se agrupó de otra manera. Elegilo de nuevo.".to_string()),
    }
}

/// The documents the base has of the generated ones, with the lasting
/// identity of the template each came from: at the place it was written,
/// with the bytes it was written with, or the one its record follows.
pub(crate) const GENERATED_CONTENTS_SQL: &str = "
    SELECT g.naming AS naming, f.version_id AS version_id FROM generated_document g
      JOIN location l ON l.origin_id = g.origin_id AND l.rel_path = g.rel_path
      JOIN file f ON f.hash = l.file_hash
    UNION SELECT g.naming, f.version_id FROM generated_document g JOIN file f ON f.hash = g.file_hash
    UNION SELECT naming, version_id FROM generated_document WHERE version_id IS NOT NULL";

/// The documents that were generated from a template.
pub(crate) fn generated_contents(conn: &Connection) -> Result<HashSet<i64>, String> {
    let mut statement = conn.prepare(&format!("SELECT version_id FROM ({GENERATED_CONTENTS_SQL})")).map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| row.get::<_, i64>(0))
        .map_err(sql_err)?
        .collect::<Result<HashSet<i64>, _>>()
        .map_err(sql_err)?;
    Ok(rows)
}

/// Whether this document was generated from a template.
pub(crate) fn is_generated(conn: &Connection, content_id: i64) -> Result<bool, String> {
    conn.query_row(&format!("SELECT count(*) FROM ({GENERATED_CONTENTS_SQL}) WHERE version_id = ?1"), [content_id], |row| {
        row.get::<_, i64>(0)
    })
    .map(|n| n > 0)
    .map_err(sql_err)
}

/// The line of a generated document's screen: when, from which template
/// (the name it had then) and for which matter it was generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedLine {
    pub id: i64,
    pub generated_at: i64,
    pub template_name: String,
    /// The location of the template's principal now, to open it; `None`
    /// when the template is no more.
    pub template_location: Option<i64>,
    pub matter_id: Option<String>,
    /// Its name now, or the one it had then.
    pub matter_name: Option<String>,
}

/// What was generated from a template, in its screen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedEntry {
    pub id: i64,
    pub generated_at: i64,
    pub file_name: String,
    pub matter_name: Option<String>,
    /// Where the base has it; `None` until it finds it, or when it is gone.
    pub location_id: Option<i64>,
}

/// The location the base shows for a document: its first one.
fn location_of(conn: &Connection, content_id: i64) -> Result<Option<i64>, String> {
    conn.query_row(
        "SELECT l.id FROM location l JOIN file f ON f.hash = l.file_hash WHERE f.version_id = ?1 ORDER BY l.id LIMIT 1",
        [content_id],
        |row| row.get(0),
    )
    .optional()
    .map_err(sql_err)
}

/// The line of the screen of a document, when it was generated from a
/// template (the latest time, if more than once).
pub(crate) fn generated_line(conn: &Connection, content_id: i64) -> Result<Option<GeneratedLine>, String> {
    let found: Option<(i64, i64, String, Option<i64>, Option<String>, Option<String>, Option<String>)> = conn
        .query_row(
            "SELECT g.id, g.generated_at, g.template_name, g.naming, g.matter_id, g.matter_name,
                    (SELECT m.name FROM knowledge_matter m WHERE m.id = g.matter_id AND m.state = 'active')
             FROM generated_document g
             WHERE g.version_id = ?1
                OR g.file_hash IN (SELECT hash FROM file WHERE version_id = ?1)
                OR EXISTS (SELECT 1 FROM location l JOIN file f ON f.hash = l.file_hash
                           WHERE l.origin_id = g.origin_id AND l.rel_path = g.rel_path AND f.version_id = ?1)
             ORDER BY g.generated_at DESC, g.id DESC LIMIT 1",
            [content_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((id, generated_at, template_name, naming, matter_id, matter_then, matter_now)) = found else {
        return Ok(None);
    };
    let template_location = match naming {
        Some(naming) => {
            let principal: Option<i64> = conn
                .query_row("SELECT principal FROM similar_template WHERE naming = ?1", [naming], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            match principal {
                Some(principal) => location_of(conn, principal)?,
                None => None,
            }
        }
        None => None,
    };
    Ok(Some(GeneratedLine { id, generated_at, template_name, template_location, matter_id, matter_name: matter_now.or(matter_then) }))
}

/// What was generated with the template of this lasting identity, the
/// latest first.
pub(crate) fn generated_with(conn: &Connection, naming: Option<i64>) -> Result<Vec<GeneratedEntry>, String> {
    let Some(naming) = naming else {
        return Ok(Vec::new());
    };
    let rows: Vec<(i64, i64, String, Option<String>, Option<String>, Option<i64>, String, String)> = {
        let mut statement = conn
            .prepare(
                "SELECT g.id, g.generated_at, g.rel_path, g.matter_name,
                        (SELECT m.name FROM knowledge_matter m WHERE m.id = g.matter_id AND m.state = 'active'),
                        g.version_id, g.origin_id, g.rel_path
                 FROM generated_document g WHERE g.naming = ?1 ORDER BY g.generated_at DESC, g.id DESC",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([naming], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        rows
    };
    let mut out = Vec::new();
    for (id, generated_at, rel_path, matter_then, matter_now, content_id, origin_id, place) in rows {
        let here: Option<i64> = conn
            .query_row("SELECT id FROM location WHERE origin_id = ?1 AND rel_path = ?2", params![origin_id, place], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        let location_id = match (here, content_id) {
            (Some(here), _) => Some(here),
            (None, Some(content_id)) => location_of(conn, content_id)?,
            (None, None) => None,
        };
        let file_name = rel_path.rsplit('/').next().unwrap_or(&rel_path).to_string();
        out.push(GeneratedEntry { id, generated_at, file_name, matter_name: matter_now.or(matter_then), location_id });
    }
    Ok(out)
}

/// How many documents were generated with the template of this lasting identity.
pub(crate) fn generated_count(conn: &Connection, naming: Option<i64>) -> Result<i64, String> {
    match naming {
        Some(naming) => conn
            .query_row("SELECT count(*) FROM generated_document WHERE naming = ?1", [naming], |row| row.get(0))
            .map_err(sql_err),
        None => Ok(0),
    }
}

/// Each generated document and its family: the documents of the template
/// it came from, and the others generated with that same template.
const GENERATED_FAMILIES_SQL: &str = "
    SELECT g.version_id AS generated, d.version_id AS source FROM generated g
      JOIN similar_template t ON t.naming = g.naming
      JOIN similar_template_document d ON d.principal = t.principal
    UNION SELECT g.version_id, h.version_id FROM generated g
      JOIN generated h ON h.naming = g.naming AND h.version_id != g.version_id";

/// The family of each generated document (the documents of the template
/// it came from and the others generated with it), by the generated one.
pub(crate) fn generated_families(conn: &Connection) -> Result<HashMap<i64, HashSet<i64>>, String> {
    let mut statement = conn
        .prepare(&format!("WITH generated AS ({GENERATED_CONTENTS_SQL}) {GENERATED_FAMILIES_SQL}"))
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
        .map_err(sql_err)?;
    let mut out: HashMap<i64, HashSet<i64>> = HashMap::new();
    for row in rows {
        let (generated, source) = row.map_err(sql_err)?;
        if generated != source {
            out.entry(generated).or_default().insert(source);
        }
    }
    Ok(out)
}

/// A generated document is made with its template, not similar to the
/// documents the template came from: it never forms a pair with them
/// (D-497), so it is never «possibly the same document» as one of them
/// nor waits for a decision. It shows as made with the template, in the
/// template and in its own screen (`generated_line`). The pairs left from
/// before (measured before its family had its template) go, unless a
/// person decided them.
pub(crate) fn generated_pairs_dropped(conn: &Connection) -> Result<usize, String> {
    let sql = format!(
        "WITH generated AS ({GENERATED_CONTENTS_SQL}), theirs AS ({GENERATED_FAMILIES_SQL})
         DELETE FROM similar_pair
         WHERE state = 'proposed'
           AND EXISTS (SELECT 1 FROM theirs WHERE (theirs.generated = similar_pair.a AND theirs.source = similar_pair.b)
                                                OR (theirs.generated = similar_pair.b AND theirs.source = similar_pair.a))"
    );
    conn.execute(&sql, []).map_err(sql_err)
}

impl DocBase {
    /// Generates a document from the template of `request.principal`: the
    /// Word in `place`, and its record. The base takes the file right away
    /// (its place and its bytes); its text is read by the base as for any
    /// file. A failure after the file is written leaves it for the next
    /// sweep: the file and its record stay.
    pub fn generate_document(&self, place: &Place, request: &GenerateRequest) -> Result<Generated, String> {
        let (built, filled, recorded, matter) = {
            let conn = self.conn();
            let built = template::for_generation(&conn, request.principal)?
                .ok_or("Esa plantilla ya no está: su familia de documentos cambió")?;
            let matter = match request.matter_id.as_deref().map(str::trim).filter(|id| !id.is_empty()) {
                Some(id) => Some((id.to_string(), matter_name(&conn, id)?)),
                None => None,
            };
            let (filled, recorded) = fill_fields(&built.fields, &request.values);
            (built, filled, recorded, matter)
        };
        let text = paragraphs(&built.segments, &built.fields, &filled);
        if text.is_empty() {
            return Err("La plantilla no tiene texto para el documento".to_string());
        }
        let missing: Vec<String> = filled
            .iter()
            .enumerate()
            .filter(|(index, _)| built.fields[*index].same_as.is_none())
            .filter_map(|(_, f)| match f {
                Filled::Missing(name) => Some(name.clone()),
                _ => None,
            })
            .collect();
        let now = now_ms();
        let at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let bytes = word(&text, &at)?;
        let template_name = match request.template_name.trim() {
            "" => "Documento".to_string(),
            name => name.to_string(),
        };
        let wanted = match request.file_name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
            Some(name) => name.to_string(),
            None => format!("{template_name} {}", chrono::Local::now().format("%Y-%m-%d")),
        };
        let path = write_new(&place.folder, &file_stem(&wanted), &bytes)?;
        let file_name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let hash = file_sha256(&path)?.ok_or("El Word salió demasiado grande")?;
        let rel_path = if place.rel_folder.is_empty() { file_name.clone() } else { format!("{}/{file_name}", place.rel_folder) };
        let id = {
            let conn = self.conn();
            conn.execute(
                "INSERT INTO generated_document (naming, principal, template_name, template_engine, template_built_at, matter_id,
                                                 matter_name, from_document, origin_id, rel_path, file_hash, field_values, generated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    built.naming,
                    request.principal,
                    template_name,
                    template::ENGINE,
                    built.computed_at,
                    matter.as_ref().map(|(id, _)| id.clone()),
                    matter.as_ref().map(|(_, name)| name.clone()),
                    request.from_document,
                    place.origin_id,
                    rel_path,
                    hash,
                    serde_json::to_string(&recorded).map_err(|e| e.to_string())?,
                    now
                ],
            )
            .map_err(sql_err)?;
            conn.last_insert_rowid()
        };
        // The base takes it now, so it shows up at once.
        if let Err(error) = self.take_generated(&place.origin_id, &rel_path) {
            eprintln!("[docbase] the generated document waits for the next sweep: {error}");
        }
        let (location_id, content_id, in_matter): (Option<i64>, Option<i64>, bool) = {
            let conn = self.conn();
            let location_id = conn
                .query_row("SELECT id FROM location WHERE origin_id = ?1 AND rel_path = ?2", params![place.origin_id, rel_path], |row| {
                    row.get(0)
                })
                .optional()
                .map_err(sql_err)?;
            let (content_id, in_matter) = conn
                .query_row(
                    "SELECT g.version_id, EXISTS (SELECT 1 FROM knowledge_matter_choice c
                                                  WHERE c.matter_id = g.matter_id AND c.version_id = g.version_id AND c.choice = 'in')
                     FROM generated_document g WHERE g.id = ?1",
                    [id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(sql_err)?;
            (location_id, content_id, in_matter)
        };
        Ok(Generated { id, path: path.to_string_lossy().to_string(), file_name, location_id, content_id, in_matter, missing })
    }

    /// Sweeps the origin of a file just written, reads its bytes and follows
    /// its record to its document.
    fn take_generated(&self, origin_id: &str, rel_path: &str) -> Result<(), String> {
        self.scan_origin(origin_id)?;
        let location: Option<i64> = self
            .conn()
            .query_row("SELECT id FROM location WHERE origin_id = ?1 AND rel_path = ?2", params![origin_id, rel_path], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        if let Some(location) = location {
            self.hash_location(location)?;
        }
        self.generated_settle().map(|_| ())
    }

    /// Follows each generated document to the document the base has of it:
    /// the one at the place it was written (a person may have edited the
    /// Word there), or else the one with the bytes it was written with; and
    /// puts that document in the matter it was generated for, as a person's
    /// choice, once: a person who takes it out of the matter has the last
    /// word, also over a later edit of the Word. Returns how many records
    /// changed.
    pub fn generated_settle(&self) -> Result<usize, String> {
        type Row = (i64, Option<i64>, Option<i64>, Option<String>, Option<i64>);
        let rows: Vec<Row> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(
                    "SELECT g.id, g.version_id, coalesce(
                        (SELECT f.version_id FROM location l JOIN file f ON f.hash = l.file_hash
                         WHERE l.origin_id = g.origin_id AND l.rel_path = g.rel_path),
                        (SELECT f.version_id FROM file f WHERE f.hash = g.file_hash)),
                        g.matter_id, g.matter_content
                     FROM generated_document g",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<Row>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let mut changed = 0;
        for (id, kept, found, matter, in_matter) in rows {
            let Some(content) = found else { continue };
            if Some(content) != kept {
                self.conn()
                    .execute("UPDATE generated_document SET version_id = ?2 WHERE id = ?1", params![id, content])
                    .map_err(sql_err)?;
                changed += 1;
            }
            let Some(matter) = matter else { continue };
            if in_matter == Some(content) || !self.may_put_in_matter(&matter, in_matter)? {
                continue;
            }
            // What cannot be put in its matter is not tried again on every
            // turn of the worker: it is said once.
            if let Err(error) = self.knowledge_choose_matter_document(&matter, &format!("content:{content}"), "in") {
                eprintln!("[docbase] the generated document {id} could not go into its matter: {error}");
            }
            self.conn()
                .execute("UPDATE generated_document SET matter_content = ?2 WHERE id = ?1", params![id, content])
                .map_err(sql_err)?;
            changed += 1;
        }
        Ok(changed)
    }

    /// Whether a generated document goes into its matter now: the matter is
    /// still active, and no person took out of it the document put there
    /// before (`previous`).
    fn may_put_in_matter(&self, matter_id: &str, previous: Option<i64>) -> Result<bool, String> {
        let conn = self.conn();
        let active: bool = conn
            .query_row("SELECT count(*) FROM knowledge_matter WHERE id = ?1 AND state = 'active'", [matter_id], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(sql_err)?
            > 0;
        let taken_out = match previous {
            Some(previous) => {
                conn.query_row(
                    "SELECT count(*) FROM knowledge_matter_choice WHERE matter_id = ?1 AND version_id = ?2 AND choice = 'out'",
                    params![matter_id, previous],
                    |row| row.get::<_, i64>(0),
                )
                .map_err(sql_err)?
                    > 0
            }
            None => false,
        };
        Ok(active && !taken_out)
    }

    /// Where the file of a generated document is now: at the place it was
    /// written, or wherever the base has its document.
    pub fn generated_path(&self, id: i64) -> Result<PathBuf, String> {
        let conn = self.conn();
        let found: Option<(Option<String>, String, Option<i64>)> = conn
            .query_row(
                "SELECT o.location, g.rel_path, g.version_id FROM generated_document g
                 LEFT JOIN origin o ON o.id = g.origin_id WHERE g.id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((root, rel_path, content_id)) = found else {
            return Err("No hay registro de ese documento generado".to_string());
        };
        if let Some(path) = root.and_then(|root| LocalFolderProvider::new(root).local_path(&rel_path)) {
            if path.is_file() {
                return Ok(path);
            }
        }
        if let Some(content_id) = content_id {
            let elsewhere: Option<(String, String)> = conn
                .query_row(
                    "SELECT o.location, l.rel_path FROM location l JOIN origin o ON o.id = l.origin_id
                     JOIN file f ON f.hash = l.file_hash WHERE f.version_id = ?1 ORDER BY l.id LIMIT 1",
                    [content_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(sql_err)?;
            if let Some(path) = elsewhere.and_then(|(root, rel)| LocalFolderProvider::new(root).local_path(&rel)) {
                return Ok(path);
            }
        }
        Err("El documento generado ya no está en su carpeta".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(kind: Kind, values: &[&str]) -> TemplateField {
        TemplateField::for_tests(kind, values.iter().map(|v| v.to_string()).collect())
    }

    fn given(position: i64, name: &str, value: &str) -> GivenValue {
        GivenValue { position, name: name.into(), value: value.into(), source: "hand".into(), reference: None }
    }

    fn plain(paragraphs: &Paragraphs) -> String {
        paragraphs
            .iter()
            .map(|p| p.iter().map(|line| line.iter().map(Piece::text).collect::<String>()).collect::<Vec<_>>().join("\n"))
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    #[test]
    fn the_text_takes_each_value_and_leaves_out_what_is_optional_and_empty() {
        let segments = vec![
            Segment::Text { text: "Recibí de ".into() },
            Segment::Field { field: 0 },
            Segment::Text { text: " la suma de $ ".into() },
            Segment::Field { field: 1 },
            Segment::Text { text: " ".into() },
            Segment::Field { field: 2 },
            Segment::Text { text: ".\n\nSon $ ".into() },
            Segment::Field { field: 3 },
            Segment::Text { text: ".\nFirma:\n\n\n".into() },
        ];
        let mut fields = vec![
            field(Kind::Name, &["Persona Uno", "Persona Dos"]),
            field(Kind::Amount, &["1.500,00", "2.750,00"]),
            field(Kind::Wording, &["", "en efectivo"]),
            field(Kind::Amount, &["1.500,00", "2.750,00"]),
        ];
        fields[3].same_as = Some(1);
        let (filled, recorded) = fill_fields(&fields, &[given(0, "Quien paga", "Persona Tres"), given(1, "Importe", " 980,50 ")]);
        assert_eq!(recorded.len(), 3, "a field said again is recorded once");
        assert_eq!(recorded[1].value, "980,50");
        let text = paragraphs(&segments, &fields, &filled);
        assert_eq!(plain(&text), "Recibí de Persona Tres la suma de $ 980,50.\n\nSon $ 980,50.\nFirma:");
        assert_eq!(text[0][0][1], Piece::Value("Persona Tres".into()));
        // A name every document has, left empty: the Word asks for it.
        let (filled, _) = fill_fields(&fields, &[given(1, "Importe", "980,50")]);
        let text = paragraphs(&segments, &fields, &filled);
        assert_eq!(plain(&text), "Recibí de [Campo 1] la suma de $ 980,50.\n\nSon $ 980,50.\nFirma:");
        assert!(text[0][0].contains(&Piece::Missing("[Campo 1]".into())));
    }

    #[test]
    fn a_fixed_field_is_written_with_its_value_and_a_form_field_goes_at_the_end() {
        let segments = vec![Segment::Text { text: "Ficha de cliente. Fecha: ".into() }, Segment::Field { field: 0 }, Segment::Text { text: ".".into() }];
        let mut fields = vec![field(Kind::Date, &["01/02/2026", "03/04/2026"]), field(Kind::Field, &["Persona Uno", "Persona Dos"])];
        fields[0].fixed = true;
        fields[0].fixed_value = Some("05/06/2026".into());
        fields[1].form_name = Some("Nombre".into());
        fields[1].name = Some("Nombre".into());
        let (filled, recorded) = fill_fields(&fields, &[given(0, "Fecha", "otra"), given(1, "Nombre", "Persona Tres")]);
        assert_eq!(recorded[0].source, "fixed");
        let text = paragraphs(&segments, &fields, &filled);
        assert_eq!(plain(&text), "Ficha de cliente. Fecha: 05/06/2026.\n\nNombre: Persona Tres");
    }

    #[test]
    fn a_file_name_is_one_windows_takes() {
        assert_eq!(file_stem("Recibo: Persona Uno / 2026-10-22.docx"), "Recibo Persona Uno 2026-10-22");
        assert_eq!(file_stem("  ...  "), "Documento generado");
        assert_eq!(file_stem("con"), "con (documento)");
        assert_eq!(file_stem(&"a".repeat(300)).chars().count(), FILE_STEM_MAX_CHARS);
    }
}
