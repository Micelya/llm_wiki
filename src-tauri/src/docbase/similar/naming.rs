//! The names of the fields of a template that its text does not give
//! (D-488): one question to the model per template, asked by itself, like
//! the other questions of the reading: only while the automatic reading is
//! on and an engine was chosen, with its version and its answer kept in
//! `model_answer`, and shown in «En curso» while it runs. No button
//! confirms it.
//!
//! The question carries the template's shared text with its fields marked,
//! and a few values of the documents in each field asked about. It asks
//! about the fields that have no name yet (nor from a person nor from the
//! text: those in the middle of a sentence) and, for a field that is a
//! name, what it is (a person, an organization, a matter), which only the
//! model can tell. What the model says is kept apart (`template_field_answer`,
//! by the words around each field), so it survives the template being
//! built again, and it never goes over what a person wrote: the layers are
//! put together when the template is shown (`template::resolve_names`).
//! A name that repeats a value of the documents is not kept: the template
//! keeps no data of a matter nor of a person.

use std::collections::{BTreeSet, HashMap};

use rusqlite::{params, Connection, OptionalExtension};

use super::super::{now_ms, sql_err, DocBase};
use super::changes::Kind;
use super::classify::{ask_and_keep, cached_answer, question_key, stops_the_run, Question, ASKING};
use super::template::{whats, Segment};
use super::text;
use crate::docbase::questions::Part;
use crate::source_volume::asking::DocumentAsker;

/// Version of the question. Changing it asks again about every field the
/// model named.
pub const FIELDS_PROMPT: &str = "campos-1";
/// What its questions are kept as in `model_answer`.
pub const FIELDS_TASK: &str = "template-fields";
/// Values of the documents shown for each field asked about.
const VALUES_SHOWN: usize = 3;
/// Longest value shown.
const VALUE_CHARS: usize = 120;
/// A longer text is sent as the passages around the fields asked about.
const TEXT_MAX_CHARS: usize = 12_000;
/// Characters of the text on each side of a field, in such a passage.
const PASSAGE_CHARS: usize = 400;
/// Longest name kept.
const NAME_MAX_CHARS: usize = 60;

/// A template as the question sees it.
pub(crate) struct Draft {
    pub segments: Vec<Segment>,
    pub fields: Vec<DraftField>,
}

/// A field as the question sees it.
pub(crate) struct DraftField {
    pub kind: Kind,
    pub form_name: Option<String>,
    pub same_as: Option<usize>,
    pub anchor: String,
    /// The name it has: a person's, its text's or the model's.
    pub name: Option<String>,
    /// The question asks about it.
    pub asked: bool,
    /// What each document says there, the principal first.
    pub values: Vec<String>,
}

/// Whether the question asks about a field: the first place of its field,
/// that the model was not asked about with this version, and that has no
/// name (from a person or from its text), or that is a name nobody said
/// what it is of.
fn asks_about(
    kind: Kind,
    same_as: Option<i64>,
    label: Option<&str>,
    person_name: Option<&str>,
    person_what: Option<&str>,
    answered: Option<&str>,
) -> bool {
    let current = answered.is_some_and(|question| question.starts_with(&format!("{FIELDS_PROMPT}:")));
    if same_as.is_some() || current {
        return false;
    }
    let unnamed = person_name.is_none() && label.is_none();
    let unknown = matches!(kind, Kind::Name | Kind::Field) && person_what.is_none();
    unnamed || unknown
}

/// The template of this principal as the question sees it, and the naming
/// it is kept under; none when it has no documents or no naming yet.
fn load_draft(conn: &Connection, principal: i64) -> Result<Option<(i64, Draft)>, String> {
    let row: Option<(Option<i64>, String)> = conn
        .query_row(
            "SELECT t.naming, t.segments FROM similar_template t WHERE t.principal = ?1
               AND EXISTS (SELECT 1 FROM similar_template_document d WHERE d.principal = t.principal)",
            [principal],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((Some(naming), segments)) = row else {
        return Ok(None);
    };
    let segments: Vec<Segment> = serde_json::from_str(&segments).unwrap_or_default();
    type Row = (i64, String, Option<String>, Option<i64>, String, Option<String>, Option<String>, Option<String>, Option<String>, Option<String>);
    let rows: Vec<Row> = {
        let mut statement = conn
            .prepare(
                "SELECT f.position, f.kind, f.form_name, f.same_as, f.anchor, f.label, c.name, c.what, a.name, a.question
                 FROM similar_template_field f
                 LEFT JOIN template_field_choice c ON c.naming = ?2 AND c.anchor = f.anchor
                 LEFT JOIN template_field_answer a ON a.naming = ?2 AND a.anchor = f.anchor
                 WHERE f.principal = ?1 ORDER BY f.position",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map(params![principal, naming], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                ))
            })
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        rows
    };
    let mut values: HashMap<i64, Vec<String>> = HashMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT v.position, v.value FROM similar_template_value v
                 JOIN similar_template_document d ON d.principal = v.principal AND d.version_id = v.version_id
                 WHERE v.principal = ?1 ORDER BY v.position, d.position",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([principal], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (position, value) = row.map_err(sql_err)?;
            values.entry(position).or_default().push(value);
        }
    }
    let fields = rows
        .into_iter()
        .map(|(position, kind, form_name, same_as, anchor, label, person_name, person_what, model_name, answered)| {
            let kind = super::template::kind_from(&kind);
            let asked = asks_about(kind, same_as, label.as_deref(), person_name.as_deref(), person_what.as_deref(), answered.as_deref());
            DraftField {
                kind,
                form_name,
                same_as: same_as.map(|s| s as usize),
                anchor,
                name: person_name.or(label).or(model_name),
                asked,
                values: values.remove(&position).unwrap_or_default(),
            }
        })
        .collect();
    Ok(Some((naming, Draft { segments, fields })))
}

/// A kind of data in words, for the question.
fn kind_words(kind: Kind) -> &'static str {
    match kind {
        Kind::Identifier => "un identificador",
        Kind::Amount => "un importe",
        Kind::Date => "una fecha",
        Kind::Number => "un número",
        Kind::Name => "un nombre",
        Kind::Field => "un campo de formulario",
        Kind::Wording => "un texto que cambia",
    }
}

/// What each kind of data a field can be is, for the question.
fn data_what(id: &str) -> &'static str {
    match id {
        "fecha" => "una fecha.",
        "importe" => "una suma de dinero.",
        "numero" => "una cantidad, o un número que no identifica a nadie ni a nada.",
        "identificador" => "un número o un código que identifica algo o a alguien: un documento, un comprobante, una cuenta, un DNI.",
        "texto" => "una parte de la redacción que cambia de un documento a otro: una frase, una cláusula.",
        _ => "",
    }
}

/// The question about a template's fields, and the fields it asks about
/// (their positions); none when it asks about none. Each value that changes
/// in each case is written by `part`: the template's text and names, and the
/// values of the documents.
pub(crate) fn question_text(draft: &Draft, part: Part) -> Option<(String, Vec<usize>)> {
    use crate::docbase::questions::{TEMPLATE, VALUES};
    let asked: Vec<usize> = (0..draft.fields.len()).filter(|&i| draft.fields[i].asked).collect();
    if asked.is_empty() {
        return None;
    }
    let number = |index: usize| draft.fields.get(index).and_then(|f| f.same_as).unwrap_or(index) + 1;
    // The text, with each field marked where it is.
    let mut rendered = String::new();
    let mut places: Vec<(usize, usize)> = Vec::new();
    for segment in &draft.segments {
        match segment {
            Segment::Text { text } => rendered.push_str(text),
            Segment::Field { field } => {
                let n = number(*field);
                let first = draft.fields.get(n - 1);
                places.push((n - 1, rendered.chars().count()));
                match first.and_then(|f| f.name.as_deref()) {
                    Some(name) => rendered.push_str(&format!("[campo {n}: «{name}»]")),
                    None => rendered.push_str(&format!("[campo {n}]")),
                }
            }
        }
    }
    let (text, cut) = passages(&rendered, &places, &asked);
    let what_list = {
        let vocabulary = crate::docbase::knowledge::vocabulary::vocabulary();
        let mut lines: Vec<String> = vocabulary
            .entity_types
            .iter()
            .map(|t| format!("  - {}: su nombre. {}", t.id, t.description.trim()))
            .collect();
        for id in super::template::DATA_WHATS {
            lines.push(format!("  - {id}: {}", data_what(id)));
        }
        lines.join("\n")
    };
    let mut list = String::new();
    for &index in &asked {
        let field = &draft.fields[index];
        let mut line = format!("- campo {}: la app lo ve como {}", index + 1, kind_words(field.kind));
        if let Some(form) = &field.form_name {
            line.push_str(&format!(", del formulario, donde se llama «{}»", part(TEMPLATE, form)));
        }
        if let Some(name) = &field.name {
            line.push_str(&format!("; ya se llama «{}»: decí solo qué es", part(TEMPLATE, name)));
        }
        let mut shown: Vec<String> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for value in field.values.iter().filter(|v| !v.trim().is_empty()) {
            if shown.len() == VALUES_SHOWN {
                break;
            }
            if seen.insert(text::normalized(value)) {
                let cut: String = value.chars().take(VALUE_CHARS).collect();
                let cut = if value.chars().count() > VALUE_CHARS { format!("{cut}…") } else { cut };
                shown.push(format!("«{}»", part(VALUES, &cut)));
            }
        }
        if shown.is_empty() {
            line.push_str("; en los documentos está vacío");
        } else {
            line.push_str(&format!("; en los documentos dice: {}", shown.join("; ")));
            if field.values.iter().any(|v| v.trim().is_empty()) {
                line.push_str("; en alguno está vacío");
            }
        }
        line.push_str(".\n");
        list.push_str(&line);
    }
    let prompt = format!(
        "Esta es una plantilla: el texto que comparten varios documentos de un archivo profesional, con lo que cambia de un \
         documento a otro marcado como [campo N]. Un campo que el texto dice en varios lugares lleva el mismo número; uno que ya \
         tiene nombre lo lleva al lado.\n\n\
         Para cada campo de la lista de abajo, decí:\n\
         - \"nombre\": cómo se llama, en pocas palabras, como lo nombraría un profesional que completa el documento (por ejemplo, \
         «Domicilio del inmueble» o «Fecha de firma»). Nunca uses un valor de los documentos: ni nombres de personas o de \
         organizaciones, ni números, ni fechas. null si el texto no permite saberlo.\n\
         - \"que_es\": qué va en el campo, uno de estos:\n{what_list}\n  null si no se puede saber.\n\n\
         Texto de la plantilla{}:\n\"\"\"\n{}\n\"\"\"\n\n\
         Campos:\n{list}\n\
         Respondé solo con el JSON pedido, con un elemento por cada campo de la lista.",
        if cut { " (pasajes: el texto alrededor de cada campo de la lista)" } else { "" },
        part(TEMPLATE, &text)
    );
    Some((prompt, asked))
}

/// The text of the template as the question carries it: whole, or, when it
/// is long, the passages around the places of the fields asked about.
fn passages(rendered: &str, places: &[(usize, usize)], asked: &[usize]) -> (String, bool) {
    let chars: Vec<char> = rendered.chars().collect();
    if chars.len() <= TEXT_MAX_CHARS {
        return (rendered.trim().to_string(), false);
    }
    let mut ranges: Vec<(usize, usize)> = places
        .iter()
        .filter(|(field, _)| asked.contains(field))
        .map(|(_, at)| (at.saturating_sub(PASSAGE_CHARS), (at + PASSAGE_CHARS).min(chars.len())))
        .collect();
    ranges.sort();
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (start, end) in ranges {
        match merged.last_mut() {
            Some(last) if start <= last.1 => last.1 = last.1.max(end),
            _ => merged.push((start, end)),
        }
    }
    let parts: Vec<String> = merged.iter().map(|(start, end)| chars[*start..*end].iter().collect::<String>().trim().to_string()).collect();
    (parts.join("\n[…]\n"), true)
}

fn fields_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["campos"],
        "properties": {
            "campos": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["campo", "nombre", "que_es"],
                    "properties": {
                        "campo": { "type": "integer" },
                        "nombre": { "type": ["string", "null"] },
                        "que_es": { "type": ["string", "null"] }
                    }
                }
            }
        }
    })
}

/// A name the model gave, as it is kept: short, without quotes, and never
/// one that repeats a value of the documents in that field.
fn clean_name(name: &str, values: &[String]) -> Option<String> {
    let name = name.trim().trim_matches(|c: char| matches!(c, '«' | '»' | '"' | '\'' | '.')).trim();
    if name.is_empty() || name.chars().count() > NAME_MAX_CHARS || !name.chars().any(char::is_alphabetic) {
        return None;
    }
    let plain = text::padded(name);
    let repeats = values.iter().map(|v| text::normalized(v)).filter(|v| v.chars().count() >= 3).any(|v| plain.contains(&format!(" {v} ")));
    if repeats {
        return None;
    }
    let mut chars = name.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

/// Keeps what an answer says of each field asked about; a field it says
/// nothing of is kept as answered with nothing, so it is not asked again
/// with this version. Returns how many fields got a name.
fn apply(conn: &Connection, naming: i64, draft: &Draft, asked: &[usize], answer: &str, question: &str) -> Result<usize, String> {
    let value: serde_json::Value = super::classify::parse_json(answer).unwrap_or(serde_json::Value::Null);
    let said: HashMap<i64, &serde_json::Value> = value["campos"]
        .as_array()
        .map(|items| items.iter().filter_map(|item| item["campo"].as_i64().map(|n| (n, item))).collect())
        .unwrap_or_default();
    let allowed = whats();
    let now = now_ms();
    let mut named = 0;
    for &index in asked {
        let field = &draft.fields[index];
        let item = said.get(&(index as i64 + 1));
        let name = item.and_then(|item| item["nombre"].as_str()).and_then(|n| clean_name(n, &field.values));
        let what = item
            .and_then(|item| item["que_es"].as_str())
            .map(|w| super::text::normalized(w).replace(' ', "-"))
            .filter(|w| allowed.contains(w));
        named += usize::from(name.is_some());
        conn.execute(
            "INSERT OR REPLACE INTO template_field_answer (naming, anchor, name, what, question, answered_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![naming, field.anchor, name, what, question, now],
        )
        .map_err(sql_err)?;
    }
    Ok(named)
}

/// A question still to ask about a template.
struct Pending {
    naming: i64,
    draft: Draft,
    asked: Vec<usize>,
    question: Question,
}

fn pending_of(conn: &Connection, principal: i64) -> Result<Option<Pending>, String> {
    let Some((naming, draft)) = load_draft(conn, principal)? else {
        return Ok(None);
    };
    let Some((prompt, asked)) = question_text(&draft, &crate::docbase::questions::plain) else {
        return Ok(None);
    };
    let question = Question {
        key: question_key(FIELDS_PROMPT, &prompt),
        version: FIELDS_PROMPT,
        task: FIELDS_TASK,
        content_id: principal,
        prompt,
        schema: fields_schema(),
    };
    Ok(Some(Pending { naming, draft, asked, question }))
}

fn principals(conn: &Connection) -> Result<Vec<i64>, String> {
    let mut statement = conn
        .prepare(
            "SELECT t.principal FROM similar_template t
             WHERE EXISTS (SELECT 1 FROM similar_template_document d WHERE d.principal = t.principal)
             ORDER BY t.principal",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([], |row| row.get(0))
        .map_err(sql_err)?
        .collect::<Result<Vec<i64>, _>>()
        .map_err(sql_err)?;
    Ok(rows)
}

/// The documents the template of this principal is built from.
fn template_documents(conn: &Connection, principal: i64) -> Result<Vec<i64>, String> {
    let mut statement = conn.prepare("SELECT version_id FROM similar_template_document WHERE principal = ?1").map_err(sql_err)?;
    let rows = statement.query_map([principal], |row| row.get(0)).map_err(sql_err)?;
    rows.collect::<Result<_, _>>().map_err(sql_err)
}

/// How many fields of the template of this principal wait for the model.
pub(crate) fn waiting_fields(conn: &Connection, principal: i64) -> Result<usize, String> {
    Ok(load_draft(conn, principal)?.map(|(_, draft)| draft.fields.iter().filter(|f| f.asked).count()).unwrap_or(0))
}

/// Each template whose fields wait for the model, with how many (what «En
/// curso» shows). Reads only.
pub(crate) fn waiting_templates(conn: &Connection) -> Result<Vec<(i64, usize)>, String> {
    let mut out = Vec::new();
    for principal in principals(conn)? {
        out.push((principal, waiting_fields(conn, principal)?));
    }
    Ok(out)
}

/// What a run of the question did.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NamingReport {
    /// Questions sent to the model.
    pub asked: usize,
    /// Fields that got a name, from the model now or from an answer kept.
    pub named: usize,
    pub failed: usize,
    /// Templates left for another run.
    pub left: usize,
    pub errors: Vec<String>,
}

impl DocBase {
    /// Asks the model about the fields of at most `max_questions`
    /// templates, and keeps what it says. An answer this engine already
    /// gave to the same question costs nothing and is used first.
    pub fn templates_naming_run(&self, asker: &dyn DocumentAsker, max_questions: usize) -> Result<NamingReport, String> {
        let _one_at_a_time = ASKING.lock().unwrap_or_else(|p| p.into_inner());
        let engine = asker.engine_id();
        let mut report = NamingReport::default();
        let mut budget = max_questions;
        let mut stopped = false;
        let all = principals(&self.conn())?;
        // A template with a whole file that is to be cut again is named
        // once the cut makes it again (D-666).
        let to_cut = if all.is_empty() { BTreeSet::new() } else { self.waiting_for_cut()? };
        for principal in all {
            if !to_cut.is_empty() && template_documents(&self.conn(), principal)?.iter().any(|id| to_cut.contains(id)) {
                continue;
            }
            let Some(pending) = pending_of(&self.conn(), principal)? else { continue };
            let kept = cached_answer(&self.conn(), &pending.question.key, Some(&engine))?;
            let answer = match kept {
                Some(answer) => answer,
                None if budget == 0 || stopped => {
                    report.left += 1;
                    continue;
                }
                None => {
                    budget -= 1;
                    report.asked += 1;
                    match ask_and_keep(self, asker, &pending.question) {
                        Ok(answer) => answer,
                        Err(error) => {
                            report.failed += 1;
                            stopped = stops_the_run(&error);
                            report.errors.push(error);
                            continue;
                        }
                    }
                }
            };
            // The template may have been built again while the model answered.
            let conn = self.conn();
            let still: Option<Option<i64>> = conn
                .query_row("SELECT naming FROM similar_template WHERE principal = ?1", [principal], |row| row.get(0))
                .optional()
                .map_err(sql_err)?;
            if still.flatten() != Some(pending.naming) {
                continue;
            }
            report.named += apply(&conn, pending.naming, &pending.draft, &pending.asked, &answer, &pending.question.key)?;
        }
        Ok(report)
    }
}

/// The question about the fields of the invented template of Ajustes
/// (`questions.rs`), built by `question_text` as the base builds it: the
/// template of two invented receipts of the same model, built by the
/// template's own functions. `part` writes each value that changes in each
/// case.
pub(crate) fn example_question(part: Part) -> (String, serde_json::Value) {
    use super::template::{build, Text};
    use crate::docbase::questions::example;
    let one = Text::new(1, &text::pages(example::RECEIPT), &[], &[], Vec::new());
    let two = Text::new(2, &text::pages(example::OTHER_RECEIPT), &[], &[], Vec::new());
    let built = build(&one, &[&two]);
    let fields = built
        .fields
        .iter()
        .map(|field| DraftField {
            kind: field.kind,
            form_name: field.form_name.clone(),
            same_as: field.same_as,
            anchor: field.anchor.clone(),
            name: field.label.clone(),
            asked: asks_about(field.kind, field.same_as.map(|s| s as i64), field.label.as_deref(), None, None, None),
            values: field.values.clone(),
        })
        .collect();
    let draft = Draft { segments: built.segments, fields };
    let (prompt, _) = question_text(&draft, part).expect("the invented receipts have fields to name");
    (prompt, fields_schema())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::questions::plain;

    fn draft() -> Draft {
        let field = |kind: Kind, name: Option<&str>, asked: bool, values: &[&str]| DraftField {
            kind,
            form_name: None,
            same_as: None,
            anchor: format!("{kind:?}"),
            name: name.map(str::to_string),
            asked,
            values: values.iter().map(|v| v.to_string()).collect(),
        };
        Draft {
            segments: vec![
                Segment::Text { text: "Recibí de ".into() },
                Segment::Field { field: 0 },
                Segment::Text { text: " la suma de $ ".into() },
                Segment::Field { field: 1 },
                Segment::Text { text: ".\nFecha: ".into() },
                Segment::Field { field: 2 },
            ],
            fields: vec![
                field(Kind::Name, None, true, &["Persona Uno", "Persona Dos", "Persona Uno", ""]),
                field(Kind::Amount, None, true, &["100,00", "200,00"]),
                field(Kind::Date, Some("Fecha"), false, &["01/02/2026", "02/03/2026"]),
            ],
        }
    }

    #[test]
    fn the_question_marks_the_fields_in_the_text_and_asks_only_about_those_without_a_name() {
        let (prompt, asked) = question_text(&draft(), &plain).unwrap();
        assert_eq!(asked, [0, 1]);
        assert!(prompt.contains("Recibí de [campo 1] la suma de $ [campo 2].\nFecha: [campo 3: «Fecha»]"), "{prompt}");
        assert!(prompt.contains("- campo 1: la app lo ve como un nombre; en los documentos dice: «Persona Uno»; «Persona Dos»; en alguno está vacío."));
        assert!(prompt.contains("- campo 2: la app lo ve como un importe; en los documentos dice: «100,00»; «200,00»."));
        assert!(!prompt.contains("- campo 3"), "a field with a name is not asked about");
        // What a field can be: the types of entity of the vocabulary, then the kinds of data.
        assert!(prompt.contains("  - persona: su nombre."), "{prompt}");
        assert!(prompt.contains("  - fecha: una fecha."));
        // Nothing to ask: no question.
        let mut done = draft();
        done.fields.iter_mut().for_each(|f| f.asked = false);
        assert!(question_text(&done, &plain).is_none());
    }

    #[test]
    fn a_long_text_goes_as_the_passages_around_the_fields_asked_about() {
        let mut long = draft();
        let filler = "palabra ".repeat(3_000);
        long.segments.insert(0, Segment::Text { text: filler.clone() });
        long.segments.push(Segment::Text { text: filler });
        let (prompt, _) = question_text(&long, &plain).unwrap();
        assert!(prompt.contains("(pasajes: el texto alrededor de cada campo de la lista)"));
        assert!(prompt.contains("[campo 1]") && prompt.contains("[campo 2]"));
        assert!(prompt.len() < 6_000, "{}", prompt.len());
    }

    #[test]
    fn only_the_fields_without_a_name_or_a_name_nobody_said_what_it_is_of_are_asked_about() {
        assert!(asks_about(Kind::Wording, None, None, None, None, None));
        assert!(!asks_about(Kind::Wording, None, Some("Cláusula"), None, None, None));
        assert!(!asks_about(Kind::Date, None, None, Some("Vence"), None, None));
        // A name with a label: what it is of.
        assert!(asks_about(Kind::Name, None, Some("Locatario"), None, None, None));
        assert!(!asks_about(Kind::Name, None, Some("Locatario"), None, Some("persona"), None));
        // Said again, or answered with this version: no; with another: yes.
        assert!(!asks_about(Kind::Wording, Some(0), None, None, None, None));
        assert!(!asks_about(Kind::Wording, None, None, None, None, Some("campos-1:abc")));
        assert!(asks_about(Kind::Wording, None, None, None, None, Some("campos-0:abc")));
    }

    #[test]
    fn a_name_that_repeats_a_value_is_not_kept() {
        let values = vec!["Persona Uno".to_string(), "Calle Falsa 123".to_string()];
        assert_eq!(clean_name(" «domicilio del inmueble». ", &values).as_deref(), Some("Domicilio del inmueble"));
        assert_eq!(clean_name("Persona Uno", &values), None);
        assert_eq!(clean_name("Domicilio en Calle Falsa 123", &values), None);
        assert_eq!(clean_name("123", &values), None);
        assert_eq!(clean_name(&"x".repeat(80), &values), None);
    }

    #[test]
    fn the_example_of_ajustes_is_built_from_invented_receipts() {
        let (prompt, schema) = example_question(&plain);
        assert!(prompt.contains("[campo 1]"), "{prompt}");
        assert!(prompt.contains("Campos:\n- campo 1"));
        assert_eq!(schema["additionalProperties"], false);
    }
}
