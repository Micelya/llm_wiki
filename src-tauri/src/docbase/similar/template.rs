//! The template of a family of documents of the same template (P-247):
//! the text they all share, with the fields that change, and the value
//! of each document in each field.
//!
//! No model. Each member is aligned with the family's principal
//! (`align`); a token of the principal is part of the template when every
//! member has it, whole, in the same place. Between two such tokens, what
//! each document says is the value of a field when not all of them say
//! the same. Each field is told apart by its form (`changes::classify`):
//! an identifier, an amount, a date, a name, a number or wording; at this
//! stage it is named by its kind and its order ("Fecha 2"). The filled-in
//! fields of a form are fields of the template too.
//!
//! Two documents of the family where no field changes, or only a few
//! words of wording, are not the same template with other data: they are
//! marked as possibly the same document, for a person to review (P-218).
//!
//! The templates are kept in the base (V21) and built again by
//! themselves whenever their family changes: other documents, another
//! text, another engine (`fingerprint`). Also for the families nobody
//! confirmed: they are shown as a proposal.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::super::{now_ms, sql_err, DocBase, Visible};
use super::align::{self, Op};
use super::changes::{classify, Kind};
use super::store::{content_of_location, doc_ref, DocRef};
use super::text::{self, Gap, Prepared, Raw};

/// Version of the computation: changing it builds every template again.
pub const ENGINE: &str = "template-1";
/// The relation of a family's member that puts it in the template.
pub const TEMPLATE_RELATION: &str = "same-template";
/// A value longer than this is cut (the whole text stays in the document).
const VALUE_MAX_CHARS: usize = 600;
/// Two documents whose fields differ in at most this many words, none of
/// them data, are possibly the same document.
const SAME_MAX_WORDS: usize = 3;

/// A document's text as the template reads it.
pub struct Text {
    pub id: i64,
    pub prepared: Prepared,
    /// Each token of `prepared.raws` as written, and what separates it
    /// from the one before.
    pub tokens: Vec<(String, Gap)>,
    /// The filled-in fields of a form: name and value.
    pub form: Vec<(String, String)>,
}

impl Text {
    pub fn new(id: i64, pages: &[Vec<&str>], skip: &[usize], drop: &[String], form: Vec<(String, String)>) -> Self {
        let prepared = Prepared::new(pages, skip, drop);
        let mut tokens = Vec::with_capacity(prepared.raws.len());
        text::walk(pages, skip, drop, |raw, gap| tokens.push((raw.to_string(), gap)));
        Text { id, prepared, tokens, form }
    }

    /// The text of a document without pages nor form, for tests.
    #[cfg(test)]
    pub fn plain(id: i64, body: &str) -> Self {
        Text::new(id, &text::pages(body), &[], &[], Vec::new())
    }
}

/// A piece of the template's text: what all the documents say, or a field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Segment {
    Text { text: String },
    Field { field: usize },
}

/// A field of the template.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub kind: Kind,
    /// Its order among the fields of its kind, from 1: "Fecha 2".
    pub ordinal: usize,
    /// For a field of a form, its name in the form.
    pub form_name: Option<String>,
    /// The value of each document, in the order of `Built::documents`;
    /// empty when the document has nothing there.
    pub values: Vec<String>,
    /// The same field said again (every document says there what it says
    /// in that one): the index of its first place, whose name it takes.
    pub same_as: Option<usize>,
}

/// A template as built.
#[derive(Debug, Clone, PartialEq)]
pub struct Built {
    /// The documents, the principal first.
    pub documents: Vec<i64>,
    pub segments: Vec<Segment>,
    /// The fields of the text in their order, then those of the form.
    pub fields: Vec<Field>,
    /// Pairs of documents (content ids, the smaller first) where nothing
    /// changes but a few words: possibly the same document.
    pub same: Vec<(i64, i64)>,
}

/// The word range of each token (empty for a token without letters or
/// digits, at the place it would be).
fn token_words(prepared: &Prepared) -> Vec<Range<usize>> {
    let mut out = Vec::with_capacity(prepared.raws.len());
    let mut word = 0;
    for raw in 0..prepared.raws.len() {
        let start = word;
        while word < prepared.words.len() && prepared.words[word].raw == raw {
            word += 1;
        }
        out.push(start..word);
    }
    out
}

/// One document while the template is built.
struct Side<'a> {
    text: &'a Text,
    ranges: Vec<Range<usize>>,
}

impl Side<'_> {
    fn keys(&self, tokens: &Range<usize>) -> Vec<&str> {
        if tokens.is_empty() {
            return Vec::new();
        }
        let words = self.ranges[tokens.start].start..self.ranges[tokens.end - 1].end;
        self.text.prepared.words[words].iter().map(|w| w.key.as_str()).collect()
    }

    fn originals(&self, tokens: &Range<usize>) -> Vec<&str> {
        self.text.tokens[tokens.clone()].iter().map(|(t, _)| t.as_str()).collect()
    }

    /// The kind of thing a value is: its most frequent kind of data when
    /// data is at least half of it, wording otherwise.
    fn kind(&self, tokens: &Range<usize>) -> Option<Kind> {
        let raws: Vec<&Raw> = self.text.prepared.raws[tokens.clone()].iter().filter(|r| !r.text.is_empty()).collect();
        if raws.is_empty() {
            return None;
        }
        let mut counts: BTreeMap<Kind, usize> = BTreeMap::new();
        for raw in &raws {
            *counts.entry(classify(raw)).or_default() += 1;
        }
        let data: usize = counts.iter().filter(|(k, _)| k.is_data()).map(|(_, n)| n).sum();
        if data * 2 < raws.len() {
            return Some(Kind::Wording);
        }
        most_frequent(&counts, true)
    }
}

/// The order in which kinds win a tie: the more specific first.
fn rank(kind: Kind) -> usize {
    match kind {
        Kind::Identifier => 0,
        Kind::Amount => 1,
        Kind::Date => 2,
        Kind::Name => 3,
        Kind::Number => 4,
        Kind::Field => 5,
        Kind::Wording => 6,
    }
}

fn most_frequent(counts: &BTreeMap<Kind, usize>, data_only: bool) -> Option<Kind> {
    counts
        .iter()
        .filter(|(kind, n)| **n > 0 && (!data_only || kind.is_data()))
        .max_by(|(ka, na), (kb, nb)| na.cmp(nb).then(rank(**kb).cmp(&rank(**ka))))
        .map(|(kind, _)| *kind)
}

/// A field of the text while it is built: the slots it covers (the
/// stretches between two tokens of the template), its kind.
struct Span {
    slots: Range<usize>,
    kind: Kind,
}

/// Builds the template of `principal` and the documents of its family
/// that share it.
pub fn build(principal: &Text, members: &[&Text]) -> Built {
    let sides: Vec<Side> = std::iter::once(principal)
        .chain(members.iter().copied())
        .map(|text| Side { text, ranges: token_words(&text.prepared) })
        .collect();
    let p = &sides[0];

    // Each member's word for each word of the principal.
    let maps: Vec<Vec<Option<usize>>> = sides[1..]
        .iter()
        .map(|side| {
            let (a, b) = align::intern(
                p.text.prepared.words.iter().map(|w| w.key.as_str()),
                side.text.prepared.words.iter().map(|w| w.key.as_str()),
            );
            let mut map = vec![None; a.len()];
            for op in align::align(&a, &b) {
                if let Op::Same { a, b } = op {
                    for (i, j) in a.zip(b) {
                        map[i] = Some(j);
                    }
                }
            }
            map
        })
        .collect();

    // The tokens of the template: every member has them whole, in order.
    // `anchors[k]` holds the token of each document, the principal first.
    let mut anchors: Vec<Vec<usize>> = Vec::new();
    for (token, words) in p.ranges.iter().enumerate() {
        if words.is_empty() {
            continue;
        }
        let mut at = vec![token];
        for (side, map) in sides[1..].iter().zip(&maps) {
            let mapped: Option<Vec<usize>> = words.clone().map(|w| map[w]).collect();
            let Some(mapped) = mapped else { break };
            let consecutive = mapped.windows(2).all(|pair| pair[1] == pair[0] + 1);
            let theirs = side.text.prepared.words[mapped[0]].raw;
            let whole = side.ranges[theirs] == (mapped[0]..mapped[mapped.len() - 1] + 1);
            let after_previous = anchors.last().is_none_or(|previous| previous[at.len()] < theirs);
            if !consecutive || !whole || !after_previous {
                break;
            }
            at.push(theirs);
        }
        if at.len() == sides.len() {
            anchors.push(at);
        }
    }

    // The stretch of each document between two tokens of the template.
    let slots = anchors.len() + 1;
    let slot = |s: usize, d: usize| -> Range<usize> {
        let start = if s == 0 { 0 } else { anchors[s - 1][d] + 1 };
        let end = if s == anchors.len() { sides[d].text.tokens.len() } else { anchors[s][d] };
        start..end
    };
    let span_tokens = |slots: &Range<usize>, d: usize| -> Range<usize> { slot(slots.start, d).start..slot(slots.end - 1, d).end };
    let kind_of = |slots: &Range<usize>| -> Kind {
        let mut counts: BTreeMap<Kind, usize> = BTreeMap::new();
        for (d, side) in sides.iter().enumerate() {
            if let Some(kind) = side.kind(&span_tokens(slots, d)) {
                *counts.entry(kind).or_default() += 1;
            }
        }
        most_frequent(&counts, false).unwrap_or(Kind::Wording)
    };

    let mut spans: Vec<Span> = Vec::new();
    for s in 0..slots {
        let first = p.keys(&slot(s, 0));
        if (1..sides.len()).all(|d| sides[d].keys(&slot(s, d)) == first) {
            continue;
        }
        let kind = kind_of(&(s..s + 1));
        // A name with a short word in it ("de") is one field, not two.
        if let Some(previous) = spans.last_mut() {
            let between = s - 1;
            if previous.slots.end == s
                && p.ranges[anchors[between][0]].len() == 1
                && previous.kind == Kind::Name
                && kind == Kind::Name
            {
                previous.slots.end = s + 1;
                continue;
            }
        }
        spans.push(Span { slots: s..s + 1, kind });
    }

    // The principal's text, with each field in the place of its value.
    let mut fields: Vec<Field> = Vec::new();
    let mut out = Writer::default();
    let mut next_span = 0;
    let mut s = 0;
    while s < slots {
        if next_span < spans.len() && spans[next_span].slots.start == s {
            let span = &spans[next_span];
            let values: Vec<Vec<&str>> = (0..sides.len()).map(|d| sides[d].originals(&span_tokens(&span.slots, d))).collect();
            let (before, after, values) = shared_edges(values);
            let mine = span_tokens(&span.slots, 0);
            let gap = if mine.is_empty() { Gap::Space } else { p.text.tokens[mine.start].1 };
            out.field(fields.len(), gap, &before, &after);
            fields.push(Field { kind: span.kind, ordinal: 0, form_name: None, values, same_as: None });
            s = span.slots.end;
            next_span += 1;
        } else {
            for token in slot(s, 0) {
                let (written, gap) = &p.text.tokens[token];
                out.token(written, *gap);
            }
            s += 1;
        }
        // The token of the template that closes the slot.
        if s <= anchors.len() {
            let (written, gap) = &p.text.tokens[anchors[s - 1][0]];
            out.token(written, *gap);
        }
    }
    let segments = out.finish();

    fields.extend(form_fields(&sides));
    name_fields(&mut fields);

    let documents: Vec<i64> = sides.iter().map(|side| side.text.id).collect();
    let same = possibly_same(&documents, &fields);
    Built { documents, segments, fields, same }
}

/// Names the fields by their kind and order ("Fecha 2"). A field of data
/// where every document says what it says in an earlier one is that same
/// field said again (the number of a receipt on each of its stubs): it
/// takes its name.
fn name_fields(fields: &mut [Field]) {
    let mut first: HashMap<Vec<String>, usize> = HashMap::new();
    for (index, field) in fields.iter_mut().enumerate() {
        if field.kind == Kind::Wording || field.form_name.is_some() {
            continue;
        }
        let key: Vec<String> = field.values.iter().map(|value| text::normalized(value)).collect();
        if key.iter().any(String::is_empty) {
            continue;
        }
        match first.get(&key) {
            Some(&original) => field.same_as = Some(original),
            None => {
                first.insert(key, index);
            }
        }
    }
    let mut seen: HashMap<Kind, usize> = HashMap::new();
    for index in 0..fields.len() {
        match fields[index].same_as {
            Some(original) => {
                fields[index].kind = fields[original].kind;
                fields[index].ordinal = fields[original].ordinal;
            }
            None => {
                let n = seen.entry(fields[index].kind).or_default();
                *n += 1;
                fields[index].ordinal = *n;
            }
        }
    }
}

/// Writes the segments of the template's text.
#[derive(Default)]
struct Writer {
    segments: Vec<Segment>,
    buffer: String,
    /// Something was written already: what comes next is separated.
    started: bool,
}

impl Writer {
    fn separate(&mut self, gap: Gap) {
        if self.started {
            self.buffer.push_str(match gap {
                Gap::Space => " ",
                Gap::Line => "\n",
                Gap::Paragraph => "\n\n",
            });
        }
        self.started = true;
    }

    fn token(&mut self, written: &str, gap: Gap) {
        self.separate(gap);
        self.buffer.push_str(written);
    }

    fn field(&mut self, field: usize, gap: Gap, before: &str, after: &str) {
        self.separate(gap);
        self.buffer.push_str(before);
        if !self.buffer.is_empty() {
            self.segments.push(Segment::Text { text: std::mem::take(&mut self.buffer) });
        }
        self.segments.push(Segment::Field { field });
        self.buffer.push_str(after);
    }

    fn finish(mut self) -> Vec<Segment> {
        if !self.buffer.is_empty() {
            self.segments.push(Segment::Text { text: self.buffer });
        }
        self.segments
    }
}

/// What every value has at its edges goes to the template: tokens with
/// no letters or digits at the start ("$"), and the punctuation at the
/// end ("."). Returns that start, that end and the values without them,
/// joined and cut.
fn shared_edges(values: Vec<Vec<&str>>) -> (String, String, Vec<String>) {
    let all_have = values.iter().all(|value| !value.is_empty());
    let mut lead = 0;
    if all_have {
        let shortest = values.iter().map(Vec::len).min().unwrap_or(0);
        while lead + 1 < shortest
            && !values[0][lead].chars().any(char::is_alphanumeric)
            && values.iter().all(|value| value[lead] == values[0][lead])
        {
            lead += 1;
        }
    }
    let mut joined: Vec<String> = values.iter().map(|value| value[lead.min(value.len())..].join(" ")).collect();
    let mut tail = String::new();
    if all_have {
        loop {
            let Some(last) = joined[0].chars().last() else { break };
            if last.is_alphanumeric() || !joined.iter().all(|value| value.chars().count() > 1 && value.ends_with(last)) {
                break;
            }
            tail.insert(0, last);
            for value in &mut joined {
                value.pop();
            }
        }
    }
    let before = if lead > 0 { format!("{} ", values[0][..lead].join(" ")) } else { String::new() };
    for value in &mut joined {
        if value.chars().count() > VALUE_MAX_CHARS {
            *value = format!("{}…", value.chars().take(VALUE_MAX_CHARS).collect::<String>());
        }
    }
    (before, tail, joined)
}

/// The fields of a form whose values are not the same in all the
/// documents, in the order the principal (then the others) has them.
fn form_fields(sides: &[Side]) -> Vec<Field> {
    let mut keys: Vec<(String, usize)> = Vec::new();
    let mut by_document: Vec<HashMap<(String, usize), String>> = Vec::new();
    for side in sides {
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut values = HashMap::new();
        for (name, value) in &side.text.form {
            let n = seen.entry(name.clone()).or_default();
            let key = (name.clone(), *n);
            *n += 1;
            if !keys.contains(&key) {
                keys.push(key.clone());
            }
            values.insert(key, value.trim().to_string());
        }
        by_document.push(values);
    }
    let mut out = Vec::new();
    for key in keys {
        let values: Vec<String> = by_document.iter().map(|values| values.get(&key).cloned().unwrap_or_default()).collect();
        if values.iter().all(|value| text::normalized(value) == text::normalized(&values[0])) {
            continue;
        }
        let mut counts: BTreeMap<Kind, usize> = BTreeMap::new();
        for value in &values {
            let kind = value
                .split_whitespace()
                .map(|token| classify(&Raw { text: token.to_string(), sentence_start: false }))
                .find(|kind| kind.is_data())
                .unwrap_or(Kind::Field);
            if !value.is_empty() {
                *counts.entry(kind).or_default() += 1;
            }
        }
        out.push(Field {
            kind: most_frequent(&counts, false).unwrap_or(Kind::Field),
            ordinal: 0,
            form_name: Some(key.0),
            values,
            same_as: None,
        });
    }
    out
}

/// The pairs of documents where no field changes, or only a few words of
/// wording: possibly the same document.
fn possibly_same(documents: &[i64], fields: &[Field]) -> Vec<(i64, i64)> {
    let mut out = Vec::new();
    for x in 0..documents.len() {
        for y in x + 1..documents.len() {
            let mut words = 0;
            let mut data = false;
            for field in fields.iter().filter(|field| field.same_as.is_none()) {
                let (a, b) = (text::normalized(&field.values[x]), text::normalized(&field.values[y]));
                if a == b {
                    continue;
                }
                let count = |s: &str| s.split(' ').filter(|w| !w.is_empty()).count();
                words += count(&a).max(count(&b));
                // A field of a form that changes (a box ticked) is data too;
                // a capital letter in what changes counts as a name.
                data |= field.kind.is_data()
                    || [&field.values[x], &field.values[y]].iter().any(|value| {
                        value.split_whitespace().any(|token| {
                            let raw = Raw { text: token.trim_matches(|c: char| !c.is_alphanumeric()).to_string(), sentence_start: false };
                            classify(&raw).is_data()
                        })
                    });
            }
            if !data && words <= SAME_MAX_WORDS {
                let (a, b) = (documents[x].min(documents[y]), documents[x].max(documents[y]));
                out.push((a, b));
            }
        }
    }
    out
}

// ── In the base ─────────────────────────────────────────────────────────

/// A document of a template, as the screens show it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDocument {
    pub doc: DocRef,
    /// The state of its pair with the principal ("proposed", "joined");
    /// `None` for the principal.
    pub state: Option<String>,
}

/// A field of a template, with the value of each document shown.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateField {
    pub kind: Kind,
    pub ordinal: i64,
    pub form_name: Option<String>,
    /// The same field said again: the index of its first place.
    pub same_as: Option<i64>,
    /// In the order of `Template::documents`.
    pub values: Vec<String>,
}

/// The template of a family, for the screens.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Template {
    pub principal: i64,
    /// The document asked about (its content).
    pub asked: i64,
    /// The documents, the principal first.
    pub documents: Vec<TemplateDocument>,
    /// "confirmed" when a person confirmed every document of it as the
    /// same template; "proposed" otherwise.
    pub state: String,
    pub segments: Vec<Segment>,
    /// Indexed by the `field` of the segments.
    pub fields: Vec<TemplateField>,
    /// Pairs of documents where nothing changes but a few words, still
    /// undecided: possibly the same document.
    pub possibly_same: Vec<[i64; 2]>,
    pub computed_at: i64,
    /// What it is called until a person names it: the type of its principal.
    #[serde(flatten)]
    pub kind: TemplateKind,
    /// Where its format comes from (`format_of`).
    pub format: String,
}

/// The type of document of a template's principal, which names the
/// template until a person names it, and its order among the templates
/// whose principal has the same type ("Recibo (2)"), when there is more
/// than one.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateKind {
    pub doc_type: Option<String>,
    /// The name the model proposed for a type that is not in the catalog
    /// (or the one a person confirmed).
    pub proposed_type_name: Option<String>,
    pub type_rank: Option<i64>,
}

/// A template in the list of the section «Plantillas».
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub principal: DocRef,
    #[serde(flatten)]
    pub kind: TemplateKind,
    /// "confirmed" or "proposed", as `Template::state`.
    pub state: String,
    /// The documents of it the caller sees, the principal included.
    pub documents: i64,
    /// Its fields, each once however often the text says it; of them, the
    /// wording that changes.
    pub fields: i64,
    pub wording_fields: i64,
    pub format: String,
    /// Pairs of its documents possibly the same document, undecided.
    pub possibly_same: i64,
}

/// What a template is built from: its documents, their texts and the
/// engine. A family whose fingerprint did not change keeps its template.
fn fingerprint(conn: &Connection, documents: &[i64]) -> Result<String, String> {
    let mut parts = vec![format!("{ENGINE}+{}", super::store::engine_id())];
    for id in documents {
        let signed: Option<(i64, String)> = conn
            .query_row("SELECT text_at, engine FROM similar_signature WHERE content_id = ?1", [id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .optional()
            .map_err(sql_err)?;
        let fields_at: Option<i64> = conn
            .query_row("SELECT created_at FROM derivative WHERE content_id = ?1 AND kind = 'fields'", [id], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        let (text_at, engine) = signed.unwrap_or_default();
        parts.push(format!("{id}:{text_at}:{engine}:{}", fields_at.unwrap_or(0)));
    }
    Ok(format!("{:016x}", text::fnv1a(parts.join("|").as_bytes())))
}

fn load_text(conn: &Connection, content_id: i64) -> Result<Option<Text>, String> {
    let row: Option<(String, String, String, Option<String>)> = conn
        .query_row(
            "SELECT d.body, m.skip_pages, m.drop_phrases, fd.body
             FROM derivative d
             JOIN document_marks m ON m.content_id = d.content_id
             LEFT JOIN derivative fd ON fd.content_id = d.content_id AND fd.kind = 'fields'
             WHERE d.content_id = ?1 AND d.kind = 'text'",
            [content_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((body, skip, drop, form)) = row else {
        return Ok(None);
    };
    let skip: Vec<usize> = serde_json::from_str(&skip).unwrap_or_default();
    let drop: Vec<String> = serde_json::from_str(&drop).unwrap_or_default();
    let form: Vec<(String, String)> = form
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
    Ok(Some(Text::new(content_id, &text::pages(&body), &skip, &drop, form)))
}

fn kind_id(kind: Kind) -> String {
    serde_json::to_value(kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

fn kind_from(id: &str) -> Kind {
    serde_json::from_value(serde_json::Value::String(id.to_string())).unwrap_or(Kind::Wording)
}

/// Of these pairs, those still undecided: no person joined nor discarded
/// the two documents.
pub(super) fn undecided(conn: &Connection, pairs: &[(i64, i64)]) -> Result<Vec<(i64, i64)>, String> {
    let mut out = Vec::new();
    for &(a, b) in pairs {
        let state: Option<String> = conn
            .query_row("SELECT state FROM similar_pair WHERE a = ?1 AND b = ?2", params![a, b], |row| row.get(0))
            .optional()
            .map_err(sql_err)?;
        if state.is_none_or(|state| state == "proposed") {
            out.push((a, b));
        }
    }
    Ok(out)
}

/// The fields (each once, however often the text says it) and the pairs
/// possibly the same document of the template of this family, if it has
/// one.
pub(super) fn summary(conn: &Connection, principal: i64) -> Result<Option<(i64, i64)>, String> {
    let row: Option<(i64, String)> = conn
        .query_row(
            "SELECT (SELECT count(*) FROM similar_template_field f WHERE f.principal = t.principal AND f.same_as IS NULL), t.same_documents
             FROM similar_template t WHERE t.principal = ?1
               AND EXISTS (SELECT 1 FROM similar_template_document d WHERE d.principal = t.principal)",
            [principal],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((fields, same)) = row else {
        return Ok(None);
    };
    let same: Vec<(i64, i64)> = serde_json::from_str(&same).unwrap_or_default();
    Ok(Some((fields, undecided(conn, &same)?.len() as i64)))
}

/// The type of each template's principal, and its order among the
/// templates whose principal has the same type when there is more than one
/// (by principal, so it does not change while they stay).
fn template_kinds(conn: &Connection) -> Result<HashMap<i64, TemplateKind>, String> {
    let rows: Vec<(i64, Option<String>, Option<String>)> = {
        let mut statement = conn
            .prepare(
                "SELECT t.principal, dt.doc_type, dt.proposed_name FROM similar_template t
                 LEFT JOIN document_type dt ON dt.content_id = t.principal
                 WHERE EXISTS (SELECT 1 FROM similar_template_document d WHERE d.principal = t.principal)
                 ORDER BY t.principal",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .map_err(sql_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(sql_err)?;
        rows
    };
    let mut of_type: HashMap<String, i64> = HashMap::new();
    for (_, doc_type, _) in &rows {
        if let Some(doc_type) = doc_type {
            *of_type.entry(doc_type.clone()).or_default() += 1;
        }
    }
    let mut seen: HashMap<String, i64> = HashMap::new();
    let mut out = HashMap::new();
    for (principal, doc_type, proposed_type_name) in rows {
        let type_rank = doc_type.as_ref().filter(|t| of_type.get(*t).copied().unwrap_or(0) > 1).map(|t| {
            let n = seen.entry(t.clone()).or_default();
            *n += 1;
            *n
        });
        out.insert(principal, TemplateKind { doc_type, proposed_type_name, type_rank });
    }
    Ok(out)
}

/// Extensions of a file written in a word processor.
const WORD_EXTS: &[&str] = &["doc", "docx", "odt", "rtf"];
/// Extensions of a picture: its text is only pixels.
const IMAGE_EXTS: &[&str] = &["jpg", "jpeg", "png", "tif", "tiff", "bmp", "gif", "webp", "heic"];

/// Where the format of a template comes from, as its principal is: a Word
/// file (`word`), a scan or a picture whose text a recognition read
/// (`scan`), a PDF whose fields a person fills in (`pdf-form`), a PDF with
/// its own text (`pdf-text`) or a file of plain text (`text`).
fn format_of(conn: &Connection, principal: i64, has_form: bool) -> Result<String, String> {
    let ext: String = conn
        .query_row(
            "SELECT lower(l.ext) FROM location l JOIN file f ON f.hash = l.file_hash WHERE f.content_id = ?1
             ORDER BY l.id LIMIT 1",
            [principal],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?
        .unwrap_or_default();
    let engine: Option<String> = conn
        .query_row("SELECT engine FROM derivative WHERE content_id = ?1 AND kind = 'text'", [principal], |row| row.get(0))
        .optional()
        .map_err(sql_err)?;
    let recognized = engine.is_some_and(|engine| engine != crate::docbase::text::EXTRACTION_ENGINE);
    let format = if WORD_EXTS.contains(&ext.as_str()) {
        "word"
    } else if recognized || IMAGE_EXTS.contains(&ext.as_str()) {
        "scan"
    } else if ext == "pdf" && has_form {
        "pdf-form"
    } else if ext == "pdf" {
        "pdf-text"
    } else {
        "text"
    };
    Ok(format.to_string())
}

/// The documents of a template the caller sees, the principal first, each
/// with the state of its pair with the principal.
fn template_documents(conn: &Connection, visible: &Visible, principal: i64) -> Result<Vec<TemplateDocument>, String> {
    let ids: Vec<i64> = {
        let mut statement = conn
            .prepare("SELECT content_id FROM similar_template_document WHERE principal = ?1 ORDER BY position")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([principal], |row| row.get(0))
            .map_err(sql_err)?
            .collect::<Result<Vec<i64>, _>>()
            .map_err(sql_err)?;
        rows
    };
    let mut documents = Vec::new();
    for id in &ids {
        let Some(doc) = doc_ref(conn, visible, *id)? else { continue };
        let state: Option<String> = if *id == principal {
            None
        } else {
            conn.query_row(
                "SELECT state FROM similar_pair WHERE a = ?1 AND b = ?2",
                params![principal.min(*id), principal.max(*id)],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_err)?
        };
        documents.push(TemplateDocument { doc, state });
    }
    Ok(documents)
}

/// "confirmed" when a person confirmed every document as of the same
/// template as the principal; "proposed" otherwise.
fn state_of(documents: &[TemplateDocument]) -> String {
    let confirmed = documents.len() > 1 && documents.iter().skip(1).all(|d| d.state.as_deref() == Some("joined"));
    if confirmed { "confirmed" } else { "proposed" }.to_string()
}

/// The pairs of these documents possibly the same document, undecided.
fn possibly_same_of(conn: &Connection, same: &str, shown: &[i64]) -> Result<Vec<[i64; 2]>, String> {
    let pairs: Vec<(i64, i64)> = serde_json::from_str(same).unwrap_or_default();
    Ok(undecided(conn, &pairs)?
        .into_iter()
        .filter(|(a, b)| shown.contains(a) && shown.contains(b))
        .map(|(a, b)| [a, b])
        .collect())
}

impl DocBase {
    /// Every template whose principal the caller sees, for the section
    /// «Plantillas»: those of more documents first. Reads only.
    pub fn templates(&self, visible: &Visible) -> Result<Vec<TemplateSummary>, String> {
        let conn = self.conn();
        let mut kinds = template_kinds(&conn)?;
        let rows: Vec<(i64, String)> = {
            let mut statement = conn
                .prepare(
                    "SELECT t.principal, t.same_documents FROM similar_template t
                     WHERE EXISTS (SELECT 1 FROM similar_template_document d WHERE d.principal = t.principal)
                     ORDER BY t.principal",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        let mut out = Vec::new();
        for (principal, same) in rows {
            let Some(head) = doc_ref(&conn, visible, principal)? else { continue };
            let documents = template_documents(&conn, visible, principal)?;
            let shown: Vec<i64> = documents.iter().map(|d| d.doc.content_id).collect();
            let (fields, wording_fields, has_form): (i64, i64, bool) = conn
                .query_row(
                    "SELECT count(*), coalesce(sum(kind = 'wording'), 0), coalesce(max(form_name IS NOT NULL), 0)
                     FROM similar_template_field WHERE principal = ?1 AND same_as IS NULL",
                    [principal],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .map_err(sql_err)?;
            out.push(TemplateSummary {
                principal: head,
                kind: kinds.remove(&principal).unwrap_or_default(),
                state: state_of(&documents),
                documents: documents.len() as i64,
                fields,
                wording_fields,
                format: format_of(&conn, principal, has_form)?,
                possibly_same: possibly_same_of(&conn, &same, &shown)?.len() as i64,
            });
        }
        out.sort_by(|a, b| b.documents.cmp(&a.documents).then(a.principal.content_id.cmp(&b.principal.content_id)));
        Ok(out)
    }

    /// Builds again the templates whose family changed, and forgets those
    /// of families that are gone or no longer share a template. Returns
    /// how many it built or forgot.
    pub fn similar_templates_refresh(&self) -> Result<usize, String> {
        let wanted: BTreeMap<i64, Vec<i64>> = {
            let conn = self.conn();
            let mut statement = conn
                .prepare(
                    "SELECT principal, content_id FROM similar_group_member
                     WHERE relation = ?1 AND content_id != principal ORDER BY principal, content_id",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([TEMPLATE_RELATION], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))
                .map_err(sql_err)?;
            let mut wanted: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
            for row in rows {
                let (principal, member) = row.map_err(sql_err)?;
                wanted.entry(principal).or_default().push(member);
            }
            wanted
        };
        let mut done = 0;
        let mut to_build: Vec<(i64, Vec<i64>, String)> = Vec::new();
        {
            let conn = self.conn();
            let stored: HashMap<i64, String> = {
                let mut statement = conn.prepare("SELECT principal, fingerprint FROM similar_template").map_err(sql_err)?;
                let rows = statement
                    .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
                    .map_err(sql_err)?
                    .collect::<Result<HashMap<_, _>, _>>()
                    .map_err(sql_err)?;
                rows
            };
            for principal in stored.keys() {
                if !wanted.contains_key(principal) {
                    conn.execute("DELETE FROM similar_template WHERE principal = ?1", [principal]).map_err(sql_err)?;
                    done += 1;
                }
            }
            for (principal, members) in &wanted {
                let documents: Vec<i64> = std::iter::once(*principal).chain(members.iter().copied()).collect();
                let print = fingerprint(&conn, &documents)?;
                if stored.get(principal) != Some(&print) {
                    to_build.push((*principal, members.clone(), print));
                }
            }
        }
        for (principal, members, print) in to_build {
            let texts: Vec<Text> = {
                let conn = self.conn();
                let mut texts = Vec::new();
                for id in std::iter::once(principal).chain(members.iter().copied()) {
                    if let Some(text) = load_text(&conn, id)? {
                        texts.push(text);
                    }
                }
                texts
            };
            let built = match texts.split_first() {
                Some((head, rest)) if head.id == principal && !rest.is_empty() => {
                    let rest: Vec<&Text> = rest.iter().collect();
                    Some(build(head, &rest))
                }
                _ => None,
            };
            self.save_template(principal, &print, built.as_ref())?;
            done += 1;
        }
        Ok(done)
    }

    fn save_template(&self, principal: i64, print: &str, built: Option<&Built>) -> Result<(), String> {
        let mut conn = self.conn();
        let tx = conn.transaction().map_err(sql_err)?;
        tx.execute("DELETE FROM similar_template WHERE principal = ?1", [principal]).map_err(sql_err)?;
        // A family whose texts cannot be read keeps its fingerprint, with
        // no documents, so it is not tried again until it changes.
        let (segments, same) = match built {
            Some(built) => (
                serde_json::to_string(&built.segments).map_err(|e| e.to_string())?,
                serde_json::to_string(&built.same).map_err(|e| e.to_string())?,
            ),
            None => ("[]".to_string(), "[]".to_string()),
        };
        tx.execute(
            "INSERT INTO similar_template (principal, fingerprint, segments, same_documents, computed_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![principal, print, segments, same, now_ms()],
        )
        .map_err(sql_err)?;
        if let Some(built) = built {
            for (position, id) in built.documents.iter().enumerate() {
                tx.execute(
                    "INSERT INTO similar_template_document (principal, content_id, position) VALUES (?1, ?2, ?3)",
                    params![principal, id, position as i64],
                )
                .map_err(sql_err)?;
            }
            for (position, field) in built.fields.iter().enumerate() {
                tx.execute(
                    "INSERT INTO similar_template_field (principal, position, kind, ordinal, form_name, same_as)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![principal, position as i64, kind_id(field.kind), field.ordinal as i64, field.form_name, field.same_as.map(|i| i as i64)],
                )
                .map_err(sql_err)?;
                for (id, value) in built.documents.iter().zip(&field.values) {
                    tx.execute(
                        "INSERT INTO similar_template_value (principal, position, content_id, value) VALUES (?1, ?2, ?3, ?4)",
                        params![principal, position as i64, id, value],
                    )
                    .map_err(sql_err)?;
                }
            }
        }
        tx.commit().map_err(sql_err)
    }

    /// The template of the family the document at this location shares,
    /// with the documents the caller sees.
    pub fn similar_template(&self, visible: &Visible, location_id: i64) -> Result<Option<Template>, String> {
        let conn = self.conn();
        let Some(content_id) = content_of_location(&conn, location_id)? else {
            return Ok(None);
        };
        let row: Option<(i64, String, String, i64)> = conn
            .query_row(
                "SELECT t.principal, t.segments, t.same_documents, t.computed_at FROM similar_template t
                 JOIN similar_template_document d ON d.principal = t.principal
                 WHERE d.content_id = ?1",
                [content_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((principal, segments, same, computed_at)) = row else {
            return Ok(None);
        };
        // The template is the principal's text: without it, nothing is shown.
        if doc_ref(&conn, visible, principal)?.is_none() {
            return Ok(None);
        }
        let documents = template_documents(&conn, visible, principal)?;
        let shown: Vec<i64> = documents.iter().map(|d| d.doc.content_id).collect();
        let mut fields: Vec<TemplateField> = {
            let mut statement = conn
                .prepare("SELECT kind, ordinal, form_name, same_as FROM similar_template_field WHERE principal = ?1 ORDER BY position")
                .map_err(sql_err)?;
            let rows = statement
                .query_map([principal], |row| {
                    Ok(TemplateField {
                        kind: kind_from(&row.get::<_, String>(0)?),
                        ordinal: row.get(1)?,
                        form_name: row.get(2)?,
                        same_as: row.get(3)?,
                        values: vec![String::new(); shown.len()],
                    })
                })
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            rows
        };
        {
            let marks = vec!["?"; shown.len().max(1)].join(", ");
            let mut statement = conn
                .prepare(&format!(
                    "SELECT position, content_id, value FROM similar_template_value WHERE principal = ? AND content_id IN ({marks})"
                ))
                .map_err(sql_err)?;
            let bound: Vec<i64> = std::iter::once(principal).chain(shown.iter().copied()).collect();
            let rows = statement
                .query_map(params_from_iter(bound.iter()), |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(sql_err)?;
            for row in rows {
                let (position, id, value) = row.map_err(sql_err)?;
                if let (Some(field), Some(column)) = (fields.get_mut(position as usize), shown.iter().position(|s| *s == id)) {
                    field.values[column] = value;
                }
            }
        }
        let possibly_same = possibly_same_of(&conn, &same, &shown)?;
        let has_form = fields.iter().any(|f| f.form_name.is_some());
        Ok(Some(Template {
            principal,
            asked: content_id,
            state: state_of(&documents),
            documents,
            segments: serde_json::from_str(&segments).unwrap_or_default(),
            fields,
            possibly_same,
            computed_at,
            kind: template_kinds(&conn)?.remove(&principal).unwrap_or_default(),
            format: format_of(&conn, principal, has_form)?,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(bodies: &[&str]) -> Vec<Text> {
        bodies.iter().enumerate().map(|(i, body)| Text::plain(i as i64 + 1, body)).collect()
    }

    fn built(bodies: &[&str]) -> Built {
        let all = texts(bodies);
        let rest: Vec<&Text> = all[1..].iter().collect();
        build(&all[0], &rest)
    }

    fn rendered(built: &Built) -> String {
        built
            .segments
            .iter()
            .map(|segment| match segment {
                Segment::Text { text } => text.clone(),
                Segment::Field { field } => {
                    let f = &built.fields[*field];
                    format!("[{}{}]", kind_id(f.kind), f.ordinal)
                }
            })
            .collect()
    }

    #[test]
    fn what_all_share_is_the_template_and_what_changes_a_field() {
        let made = built(&[
            "Recibí de Ana Torres la suma de $ 1.500,00 el día 10/02/2026 en concepto de honorarios.",
            "Recibí de Bruno Vega la suma de $ 2.750,00 el día 12/03/2026 en concepto de honorarios.",
            "Recibí de Carla Ruiz la suma de $ 980,50 el día 01/04/2026 en concepto de honorarios.",
        ]);
        assert_eq!(
            rendered(&made),
            "Recibí de [name1] la suma de $ [amount1] el día [date1] en concepto de honorarios."
        );
        assert_eq!(made.fields[0].values, ["Ana Torres", "Bruno Vega", "Carla Ruiz"]);
        assert_eq!(made.fields[1].values, ["1.500,00", "2.750,00", "980,50"]);
        assert_eq!(made.fields[2].values, ["10/02/2026", "12/03/2026", "01/04/2026"]);
        assert!(made.same.is_empty());
    }

    #[test]
    fn a_name_with_a_short_word_in_it_is_one_field() {
        let made = built(&[
            "Comparece Elvira de los Olmos, mayor de edad, y declara bajo juramento lo que sigue en este acto.",
            "Comparece Juana de la Cruz, mayor de edad, y declara bajo juramento lo que sigue en este acto.",
        ]);
        assert_eq!(made.fields.len(), 1, "{}", rendered(&made));
        assert_eq!(made.fields[0].kind, Kind::Name);
        assert_eq!(made.fields[0].values, ["Elvira de los Olmos", "Juana de la Cruz"]);
    }

    #[test]
    fn text_only_one_has_is_a_field_and_the_others_have_nothing_there() {
        let made = built(&[
            "Primera: el plazo es de un año. Segunda: el precio se paga por mes adelantado.",
            "Primera: el plazo es de un año. Tercera: se pacta una garantía adicional. Segunda: el precio se paga por mes adelantado.",
        ]);
        assert_eq!(made.fields.len(), 1, "{}", rendered(&made));
        assert_eq!(made.fields[0].kind, Kind::Wording);
        assert_eq!(made.fields[0].values[0], "");
        assert!(made.fields[0].values[1].starts_with("Tercera"), "{:?}", made.fields[0].values);
        assert!(rendered(&made).starts_with("Primera: el plazo es de un año. [wording1]"), "{}", rendered(&made));
    }

    #[test]
    fn two_documents_where_nothing_changes_are_possibly_the_same() {
        let a = "Intimo a usted para que en el plazo de 48 horas regularice el pago de lo adeudado.";
        let b = "Intimo a usted para que en el plazo de 48 horas regularice el pago de todo lo adeudado.";
        let c = "Intimo a usted para que en el plazo de 72 horas regularice el pago de lo adeudado.";
        let made = built(&[a, b, c]);
        // The first two differ in a word of wording; the third in data.
        assert_eq!(made.same, vec![(1, 2)]);
        // Exactly the same text, too.
        let made = built(&[a, a]);
        assert!(made.fields.is_empty());
        assert_eq!(made.same, vec![(1, 2)]);
        assert_eq!(rendered(&made), a);
    }

    #[test]
    fn the_fields_of_a_form_are_fields_of_the_template() {
        let page = "Ficha de cliente. Complete los datos de la persona y firme al pie.";
        let one = Text::new(1, &text::pages(page), &[], &[], vec![("Nombre".into(), "Ana Torres".into()), ("Fecha".into(), "01/02/2026".into())]);
        let two = Text::new(2, &text::pages(page), &[], &[], vec![("Nombre".into(), "Bruno Vega".into()), ("Fecha".into(), "01/02/2026".into())]);
        let made = build(&one, &[&two]);
        assert_eq!(made.fields.len(), 1);
        assert_eq!(made.fields[0].form_name.as_deref(), Some("Nombre"));
        assert_eq!(made.fields[0].values, ["Ana Torres", "Bruno Vega"]);
        assert!(made.same.is_empty(), "a name changes");
        assert_eq!(rendered(&made), page);
    }

    #[test]
    fn the_same_value_said_again_is_the_same_field() {
        let made = built(&[
            "Recibo N° 0001-00001234. Talón para el cliente. Recibo N° 0001-00001234. Talón para el banco.",
            "Recibo N° 0001-00005678. Talón para el cliente. Recibo N° 0001-00005678. Talón para el banco.",
        ]);
        assert_eq!(
            rendered(&made),
            "Recibo N° [identifier1]. Talón para el cliente. Recibo N° [identifier1]. Talón para el banco."
        );
        assert_eq!(made.fields[1].same_as, Some(0));
    }

    #[test]
    fn an_inserted_token_and_the_lines_of_the_text_are_kept() {
        let made = built(&["Señor Juez:\n\nSolicito copia del expediente.", "Señor Juez:\n\nSolicito copia certificada del expediente."]);
        assert_eq!(rendered(&made), "Señor Juez:\n\nSolicito copia [wording1] del expediente.");
        assert_eq!(made.fields[0].values, ["", "certificada"]);
    }

    /// Builds the templates of a copy of a real base (`MICELYA_DOCBASE_COPY`)
    /// and counts them: families, documents and fields of each, possibly
    /// the same, and the time. Only numbers are printed (R-24); with
    /// `MICELYA_TEMPLATE_DUMP` the templates are written to that file,
    /// outside the repository, to look at them by hand.
    #[test]
    #[ignore = "reads a copy of a real base"]
    fn templates_of_a_copy_of_the_base() {
        if std::env::var("MICELYA_DOCBASE_COPY").is_err() {
            println!("MICELYA_DOCBASE_COPY is not set: nothing to measure");
            return;
        }
        let Some((_scratch, base)) = crate::docbase::knowledge::check::open_base() else { return };
        let started = std::time::Instant::now();
        let built = base.similar_templates_refresh().unwrap();
        let first = started.elapsed();
        let started = std::time::Instant::now();
        let again = base.similar_templates_refresh().unwrap();
        let second = started.elapsed();
        println!("templates built: {built} in {} ms; refresh with nothing changed: {again} in {} ms", first.as_millis(), second.as_millis());
        let groups = base.similar_groups(&Visible::All).unwrap();
        let mut dump = String::new();
        let mut total_fields = 0;
        let mut with_template = 0;
        for group in &groups {
            let Some(fields) = group.template_fields else {
                println!("family of {} documents: no template (no member of the same template)", group.members.len());
                continue;
            };
            with_template += 1;
            total_fields += fields;
            let template = base.similar_template(&Visible::All, group.principal.location_id).unwrap().unwrap();
            let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
            for field in &template.fields {
                *kinds.entry(format!("{}{}", kind_id(field.kind), if field.form_name.is_some() { "(form)" } else { "" })).or_default() += 1;
            }
            let text_chars: usize = template
                .segments
                .iter()
                .map(|s| match s {
                    Segment::Text { text } => text.chars().count(),
                    Segment::Field { .. } => 0,
                })
                .sum();
            println!(
                "family of {} documents: template of {} documents, {} fields in {} places {:?}, {} characters of shared text, {} possibly the same, {}",
                group.members.len(),
                template.documents.len(),
                template.fields.iter().filter(|f| f.same_as.is_none()).count(),
                template.fields.len(),
                kinds,
                text_chars,
                template.possibly_same.len(),
                template.state
            );
            dump.push_str(&format!("

===== principal {} ({} documents, {} fields, possibly same {:?})
", template.principal, template.documents.len(), template.fields.len(), template.possibly_same));
            for segment in &template.segments {
                match segment {
                    Segment::Text { text } => dump.push_str(text),
                    Segment::Field { field } => {
                        let f = &template.fields[*field];
                        dump.push_str(&format!("[{}{}]", kind_id(f.kind), f.ordinal));
                    }
                }
            }
            dump.push_str("
-- values
");
            for field in template.fields.iter().filter(|f| f.same_as.is_none()) {
                dump.push_str(&format!("{}{} {:?}: {:?}
", kind_id(field.kind), field.ordinal, field.form_name, field.values));
            }
        }
        println!("families with a template: {with_template} of {}; fields in all: {total_fields}", groups.len());
        if let Ok(path) = std::env::var("MICELYA_TEMPLATE_DUMP") {
            std::fs::write(path, dump).unwrap();
        }
    }
}
