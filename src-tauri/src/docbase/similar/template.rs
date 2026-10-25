//! The template of a family of documents of the same template (P-247):
//! the text they all share, with the fields that change, and the value
//! of each document in each field.
//!
//! No model. Each member is aligned with the family's principal
//! (`align`); a token of the principal is part of the template when every
//! member has it, whole, in the same place. Between two such tokens, what
//! each document says is the value of a field when not all of them say
//! the same. Each field is told apart by its form (`changes::classify`):
//! an identifier, an amount, a date, a name, a number or wording, and
//! named in layers (D-488): what a person wrote; the name its text gives it
//! at no cost (its name in a form, a clear label before it); the model's
//! (`naming`); or its kind and order ("Fecha 2"). What a person decides
//! is kept apart, by the words around the field, and survives every
//! build. The filled-in fields of a form are fields of the template too.
//!
//! Two documents of the family where no field changes, or only a few
//! words of wording, are not the same template with other data: they are
//! marked as possibly the same document, for a person to review (P-218).
//!
//! The templates are kept in the base (V21) and built again by
//! themselves whenever their family changes: other documents, another
//! text, another engine (`fingerprint`). Also for the families nobody
//! confirmed: they are shown as a proposal.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Range;

use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use super::super::{now_ms, sql_err, DocBase, Visible};
use super::align::{self, Op};
use super::changes::{classify, Kind};
use super::store::{content_of_location, doc_ref, DocRef};
use super::text::{self, Gap, Prepared, Raw};

/// Version of the computation: changing it builds every template again.
pub const ENGINE: &str = "template-3";
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
    /// The words of the template around it (`anchors`): what was said
    /// about the field is found again by them when the template is built
    /// again.
    pub anchor: String,
    /// The name its text gives it, at no cost (`field_labels`): its name in
    /// the form, or the label right before it ("Fecha de emisión:").
    pub label: Option<String>,
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
            fields.push(Field { kind: span.kind, ordinal: 0, form_name: None, values, same_as: None, anchor: String::new(), label: None });
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
    let form_names: Vec<Option<String>> = fields.iter().map(|field| field.form_name.clone()).collect();
    for (field, anchor) in fields.iter_mut().zip(field_anchors(&segments, &form_names)) {
        field.anchor = anchor;
    }
    let originals: Vec<bool> = fields.iter().map(|field| field.same_as.is_none()).collect();
    for (field, label) in fields.iter_mut().zip(field_labels(&segments, &form_names, &originals)) {
        field.label = label;
    }

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

/// Words of the template on each side of a field that find it again.
const ANCHOR_WORDS: usize = 4;

/// The anchor of each field (`form_names` says which are of a form): the
/// words of the template before and after its first place, normalized,
/// another field said as "_" ("recibi de|la suma de"); for a field of a
/// form, its name there and its order among those of that name. Two
/// fields with the same words around them are told apart by their order
/// ("…#2"). The values are never part of it: it is the shared text.
fn field_anchors(segments: &[Segment], form_names: &[Option<String>]) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    let mut first_at: HashMap<usize, usize> = HashMap::new();
    for segment in segments {
        match segment {
            Segment::Text { text } => words.extend(text::normalized(text).split(' ').filter(|w| !w.is_empty()).map(str::to_string)),
            Segment::Field { field } => {
                first_at.entry(*field).or_insert(words.len());
                words.push("_".to_string());
            }
        }
    }
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut out = Vec::with_capacity(form_names.len());
    for (index, form_name) in form_names.iter().enumerate() {
        let base = match (first_at.get(&index), form_name) {
            (Some(&at), _) => {
                let before = words[at.saturating_sub(ANCHOR_WORDS)..at].join(" ");
                let after = words[at + 1..(at + 1 + ANCHOR_WORDS).min(words.len())].join(" ");
                format!("{before}|{after}")
            }
            (None, Some(name)) => format!("form:{}", name.trim().to_lowercase()),
            (None, None) => "|".to_string(),
        };
        let n = seen.entry(base.clone()).or_default();
        *n += 1;
        out.push(if *n == 1 { base } else { format!("{base}#{n}") });
    }
    out
}

/// The name the text gives each field at no cost, without the model
/// (D-488): a field of a form, its name there when it is a name and not a
/// technical one ("Text1"); a field of the text, the label right before it
/// on its line when it is clear: it ends in ":" ("Fecha de emisión:") or
/// in the sign of a number ("Factura N°"), it is short and it holds no
/// data. A field in the middle of a sentence ("…domiciliado en [campo]")
/// has none: the model names it. `originals` says which fields are the
/// first place of their field; the others take its name.
fn field_labels(segments: &[Segment], form_names: &[Option<String>], originals: &[bool]) -> Vec<Option<String>> {
    // The text of its line before each field's first place, from the start
    // of the line or from the field before it; for a field that starts its
    // line, the whole line before it ("Fecha de alta:\n[campo]").
    let mut before: HashMap<usize, String> = HashMap::new();
    let mut line = String::new();
    let mut previous = String::new();
    for segment in segments {
        match segment {
            Segment::Text { text } => {
                let mut parts = text.split('\n');
                if let Some(first) = parts.next() {
                    line.push_str(first);
                }
                for part in parts {
                    previous = std::mem::take(&mut line);
                    line.push_str(part);
                }
            }
            Segment::Field { field } => {
                let own = if line.trim().is_empty() { previous.clone() } else { line.clone() };
                before.entry(*field).or_insert(own);
                line.clear();
                previous.clear();
            }
        }
    }
    (0..form_names.len())
        .map(|index| {
            if !originals.get(index).copied().unwrap_or(true) {
                return None;
            }
            match (before.get(&index), &form_names[index]) {
                (Some(text), _) => label_before(text),
                (None, Some(name)) => form_label(name),
                (None, None) => None,
            }
        })
        .collect()
}

/// The signs a text writes before a number ("N°", "Nro."), lowercase.
const NUMBER_SIGNS: &[&str] = &["n°", "nº", "n.º", "n.°", "nro", "nro.", "núm.", "num.", "número", "numero", "no."];
/// Currency signs the label of an amount may end in ("Total: $"), lowercase.
const CURRENCY_SIGNS: &[&str] = &["$", "u$s", "us$", "usd", "ars", "€", "eur"];
/// Short words of a label all in capitals that are not initials.
const SHORT_WORDS: &[&str] = &["de", "del", "la", "el", "los", "las", "y", "e", "o", "u", "en", "a", "al", "por", "con", "sin", "su", "sus", "the", "of", "and", "to"];
/// Initials of four letters or more, which stay in capitals.
const INITIALS: &[&str] = &["CUIT", "CUIL", "CUIM", "CDI", "NUP", "IBAN", "SWIFT"];
/// The longest label taken from the text.
const LABEL_MAX_WORDS: usize = 5;
const LABEL_MAX_CHARS: usize = 40;

/// The label a line ends in right before a field, when it is clear.
fn label_before(line: &str) -> Option<String> {
    let mut words: Vec<&str> = line.split_whitespace().collect();
    // "Total a pagar: $ [campo]": the sign of the currency is not the label.
    while words.last().is_some_and(|last| CURRENCY_SIGNS.contains(&last.to_lowercase().as_str())) {
        words.pop();
    }
    let mut text = words.join(" ");
    let colon = text.ends_with(':');
    if colon {
        text.pop();
    }
    // Only what comes after the end of the sentence before it.
    if let Some(at) = [". ", "; ", ": ", "? ", "! "].iter().filter_map(|sep| text.rfind(sep).map(|at| at + sep.len())).max() {
        text = text[at..].to_string();
    }
    let text = text.trim_start_matches(|c: char| !c.is_alphanumeric()).trim();
    let mut words: Vec<&str> = text.split_whitespace().collect();
    let number = words.last().is_some_and(|last| NUMBER_SIGNS.contains(&last.to_lowercase().as_str()));
    if !colon && !number {
        return None;
    }
    if number && words.len() == 1 {
        return Some("Número".to_string());
    }
    if colon && number {
        // "DNI N°:" is the DNI.
        words.pop();
    }
    clear_name(&words.join(" "))
}

/// The name of a field of a form, when it is a name and not a technical
/// one: "Nombre_y_apellido" is «Nombre y apellido»; "Text1", "Check Box3"
/// or "form1[0].f1[0]" say nothing.
fn form_label(name: &str) -> Option<String> {
    let last = name.rsplit('.').find(|part| !part.trim().is_empty()).unwrap_or(name);
    let mut plain = String::new();
    let mut depth = 0usize;
    for c in last.chars() {
        match c {
            '[' => depth += 1,
            ']' => depth = depth.saturating_sub(1),
            '_' | '-' if depth == 0 => plain.push(' '),
            c if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let plain = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    let words = text::normalized(&plain);
    const TECHNICAL: &[&str] =
        &["text", "texto", "campo", "field", "fill", "check box", "checkbox", "casilla", "textbox", "untitled", "sin titulo", "boton", "button"];
    let stem = words.trim_end_matches(|c: char| c.is_ascii_digit() || c == ' ');
    if stem.chars().count() < 2 || TECHNICAL.contains(&stem) {
        return None;
    }
    clear_name(&plain)
}

/// A label as a name: short, with letters, without data (an identifier,
/// an amount or a date), and with the capital letters of a name: «Factura
/// B N°» for "FACTURA B N°"; «DNI» stays.
fn clear_name(label: &str) -> Option<String> {
    let label = label.trim().trim_end_matches(['.', ',', ';']).trim();
    let words: Vec<&str> = label.split_whitespace().collect();
    if words.is_empty() || words.len() > LABEL_MAX_WORDS || label.chars().count() > LABEL_MAX_CHARS || label.contains(',') {
        return None;
    }
    let lettered = words.iter().any(|word| word.chars().filter(|c| c.is_alphabetic()).count() >= 2);
    if !label.chars().next().is_some_and(char::is_alphabetic) || !lettered {
        return None;
    }
    let data = words.iter().any(|word| {
        let raw = Raw { text: word.to_string(), sentence_start: false, numbering: false };
        matches!(classify(&raw), Kind::Identifier | Kind::Amount | Kind::Date)
    });
    if data {
        return None;
    }
    // In a label all in capitals, the words go to lowercase but the short
    // ones that are initials ("DNI", "IVA") and those of `INITIALS`.
    let shouting = label.chars().filter(|c| c.is_alphabetic()).all(char::is_uppercase);
    let joined = words
        .iter()
        .map(|word| {
            let letters = word.chars().filter(|c| c.is_alphabetic()).count();
            let lower = word.to_lowercase();
            let initials = (letters <= 3 && !SHORT_WORDS.contains(&lower.as_str())) || INITIALS.contains(word);
            if shouting && !initials { lower } else { word.to_string() }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut chars = joined.chars();
    let first = chars.next()?;
    Some(first.to_uppercase().chain(chars).collect())
}

/// The words before and after a field in its anchor, without its order.
fn anchor_sides(anchor: &str) -> (&str, &str) {
    let core = match anchor.rsplit_once('#') {
        Some((core, n)) if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => core,
        _ => anchor,
    };
    core.split_once('|').unwrap_or((core, ""))
}

/// Pairs the anchors something was kept by (`kept`) with those of the
/// fields of a template built again (`fields`): the same anchor first;
/// then, among the rest, the only field with the same words before it,
/// and then after it, when only one kept anchor has them too. Returns the
/// kept anchors that found their field under another anchor, with it.
fn reattach(kept: &[String], fields: &[String]) -> Vec<(String, String)> {
    let mut free_kept: Vec<&String> = kept.iter().filter(|k| !fields.contains(k)).collect();
    let mut free_fields: Vec<&String> = fields.iter().filter(|f| !kept.contains(f)).collect();
    let mut moved = Vec::new();
    for side in [0, 1] {
        let part = |anchor: &str| -> String {
            let (before, after) = anchor_sides(anchor);
            if side == 0 { before } else { after }.to_string()
        };
        let mut index = 0;
        while index < free_kept.len() {
            let wanted = part(free_kept[index]);
            let rivals = free_kept.iter().filter(|k| part(k) == wanted).count();
            let found: Vec<usize> = (0..free_fields.len()).filter(|&f| part(free_fields[f]) == wanted).collect();
            if !wanted.is_empty() && rivals == 1 && found.len() == 1 {
                let field = free_fields.remove(found[0]);
                moved.push((free_kept.remove(index).clone(), field.clone()));
            } else {
                index += 1;
            }
        }
    }
    moved
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
                .map(|token| classify(&Raw { text: token.to_string(), sentence_start: false, numbering: false }))
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
            anchor: String::new(),
            label: None,
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
                            let raw = Raw { text: token.trim_matches(|c: char| !c.is_alphanumeric()).to_string(), sentence_start: false, numbering: false };
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
    /// The name it is shown with (`resolve_names`); `None`: by its kind and
    /// order, with the words of the vocabulary ("Fecha 2").
    pub name: Option<String>,
    /// Where that name comes from: `person`, `label` (the form or the text
    /// before it) or `model`.
    pub name_source: Option<String>,
    /// What it is (`whats`): a person, a date…; `None` when nothing says.
    pub what: Option<String>,
    /// Where that comes from: `person`, `model` or `kind` (the kind of
    /// data the comparison sees).
    pub what_source: Option<String>,
    /// A person marked it fixed: it is not asked when a document is
    /// generated, and `fixed_value` is written there.
    pub fixed: bool,
    pub fixed_value: Option<String>,
    /// What a person decided, and what the label and the model said,
    /// before the layers are put together.
    #[serde(skip)]
    said: Said,
}

#[cfg(test)]
impl TemplateField {
    /// A field of this kind with these values and nothing said of it.
    pub(super) fn for_tests(kind: Kind, values: Vec<String>) -> Self {
        TemplateField {
            kind,
            ordinal: 1,
            form_name: None,
            same_as: None,
            values,
            name: None,
            name_source: None,
            what: None,
            what_source: None,
            fixed: false,
            fixed_value: None,
            said: Said::default(),
        }
    }
}

/// What each layer says of a field's name and of what it is.
#[derive(Debug, Clone, Default)]
struct Said {
    person_name: Option<String>,
    person_what: Option<String>,
    label: Option<String>,
    model_name: Option<String>,
    model_what: Option<String>,
}

/// What a field can be (`TemplateField::what`), besides the types of
/// entity of the knowledge layer's vocabulary: the kinds of data a
/// document is generated with.
pub const DATA_WHATS: [&str; 5] = ["fecha", "importe", "numero", "identificador", "texto"];

/// Everything a field can be: the types of entity of the vocabulary in use
/// (a person, an organization, a matter), then the kinds of data.
pub fn whats() -> Vec<String> {
    crate::docbase::knowledge::vocabulary::vocabulary()
        .entity_types
        .iter()
        .map(|t| t.id.clone())
        .chain(DATA_WHATS.iter().map(|w| w.to_string()))
        .collect()
}

/// What a field of this kind of data is, when its kind says it.
fn what_of_kind(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Identifier => Some("identificador"),
        Kind::Amount => Some("importe"),
        Kind::Date => Some("fecha"),
        Kind::Number => Some("numero"),
        Kind::Wording => Some("texto"),
        // A name may be a person's, an organization's or a matter's.
        Kind::Name | Kind::Field => None,
    }
}

/// Puts the layers together (D-488): the name is what a person wrote; or
/// the name the text gives the field (`label`); or the model's; or none
/// (its kind and order, on the screen). What it is: a person's choice, or
/// the model's, or what its kind of data says. A field said again takes
/// all of it from its first place. Two fields named alike by the program
/// or the model are told apart by their order: "Fecha", "Fecha (2)".
fn resolve_names(fields: &mut [TemplateField]) {
    let mut used: HashMap<String, usize> = HashMap::new();
    for index in 0..fields.len() {
        if let Some(original) = fields[index].same_as.map(|o| o as usize).filter(|o| *o < index) {
            let (name, source, what, what_source) = {
                let first = &fields[original];
                (first.name.clone(), first.name_source.clone(), first.what.clone(), first.what_source.clone())
            };
            let (fixed, fixed_value) = (fields[original].fixed, fields[original].fixed_value.clone());
            let field = &mut fields[index];
            (field.name, field.name_source, field.what, field.what_source) = (name, source, what, what_source);
            (field.fixed, field.fixed_value) = (fixed, fixed_value);
            continue;
        }
        let field = &mut fields[index];
        let said = &field.said;
        let (name, source) = match (&said.person_name, &said.label, &said.model_name) {
            (Some(name), _, _) => (Some(name.clone()), Some("person")),
            (None, Some(label), _) => (Some(label.clone()), Some("label")),
            (None, None, Some(name)) => (Some(name.clone()), Some("model")),
            (None, None, None) => (None, None),
        };
        let name = name.map(|name| {
            let n = used.entry(name.to_lowercase()).or_default();
            *n += 1;
            if *n > 1 && source != Some("person") {
                format!("{name} ({n})")
            } else {
                name
            }
        });
        let (what, what_source) = match (&said.person_what, &said.model_what, what_of_kind(field.kind)) {
            (Some(what), _, _) => (Some(what.clone()), Some("person")),
            (None, Some(what), _) => (Some(what.clone()), Some("model")),
            (None, None, Some(what)) => (Some(what.to_string()), Some("kind")),
            (None, None, None) => (None, None),
        };
        field.name = name;
        field.name_source = source.map(str::to_string);
        field.what = what;
        field.what_source = what_source.map(str::to_string);
    }
}

/// What a person decides about a field of a template (D-488). Kept apart
/// from the template, by the words around the field, so it survives every
/// time the template is built again.
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct FieldChoice {
    /// Its name; empty or none: the automatic one.
    pub name: Option<String>,
    /// What it is, one of `whats`; none: the automatic one.
    pub what: Option<String>,
    /// It is written always the same and not asked when generating.
    pub fixed: bool,
    /// What is written when it is fixed (one of the values of the
    /// documents, or another; empty: nothing).
    pub fixed_value: Option<String>,
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
    /// The name a person gave it, kept apart from it (`template_naming`).
    pub name: Option<String>,
    /// What a field can be, for the choice of a person (`whats`).
    pub whats: Vec<String>,
    /// The documents generated with it, the latest first (D-488).
    pub generated: Vec<super::generate::GeneratedEntry>,
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
    /// The name a person gave it.
    pub name: Option<String>,
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
    /// Fields whose name (or what they are) waits for the model.
    pub waiting_fields: i64,
    /// Documents generated with it (D-488).
    pub generated: i64,
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
    let body = super::format::comparable(&body);
    Ok(Some(Text::new(content_id, &text::pages(&body), &skip, &drop, form)))
}

fn kind_id(kind: Kind) -> String {
    serde_json::to_value(kind).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default()
}

pub(super) fn kind_from(id: &str) -> Kind {
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

/// The pairs of documents of every template where no datum changes:
/// possibly the same document (D-440). Made with one template, they would
/// wait for no one; as possibly the same document, they wait for a person
/// like any copy or version (D-497). Decided or not.
pub(super) fn possibly_same_pairs(conn: &Connection) -> Result<HashSet<(i64, i64)>, String> {
    let mut statement = conn.prepare("SELECT same_documents FROM similar_template").map_err(sql_err)?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(sql_err)?;
    let mut out = HashSet::new();
    for row in rows {
        let pairs: Vec<(i64, i64)> = serde_json::from_str(&row.map_err(sql_err)?).unwrap_or_default();
        out.extend(pairs.into_iter().map(|(a, b)| (a.min(b), a.max(b))));
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
                name: template_name(&conn, principal)?,
                state: state_of(&documents),
                documents: documents.len() as i64,
                fields,
                wording_fields,
                format: format_of(&conn, principal, has_form)?,
                possibly_same: possibly_same_of(&conn, &same, &shown)?.len() as i64,
                waiting_fields: super::naming::waiting_fields(&conn, principal)? as i64,
                generated: super::generate::generated_count(&conn, naming_now(&conn, principal)?)?,
            });
        }
        out.sort_by(|a, b| b.documents.cmp(&a.documents).then(a.principal.content_id.cmp(&b.principal.content_id)));
        Ok(out)
    }

    /// Builds again the templates whose family changed, and forgets those
    /// of families that are gone or no longer share a template. Returns
    /// how many it built or forgot. A document generated from a template is
    /// never one of its documents (D-488): the template stays joined only to
    /// the documents it came from.
    pub fn similar_templates_refresh(&self) -> Result<usize, String> {
        let wanted: BTreeMap<i64, Vec<i64>> = {
            let conn = self.conn();
            let generated = super::generate::generated_contents(&conn)?;
            // The same template is not a family (D-497): the units are
            // grouped here with it.
            let (units, links) = super::store::grouped(&conn, super::store::UNIT_RELATIONS)?;
            let mut rows: Vec<(i64, i64)> = Vec::new();
            for unit in units {
                for member in unit.members.into_iter().filter(|m| *m != unit.principal) {
                    let relation = links.get(&unit.principal).and_then(|l| l.get(&member)).map(|(r, _)| r.as_str());
                    if relation == Some(TEMPLATE_RELATION) {
                        rows.push((unit.principal, member));
                    }
                }
            }
            rows.sort();
            let mut wanted: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
            for (principal, member) in rows {
                if generated.contains(&principal) || generated.contains(&member) {
                    continue;
                }
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
        let previous: Option<i64> = tx
            .query_row("SELECT naming FROM similar_template WHERE principal = ?1", [principal], |row| row.get(0))
            .optional()
            .map_err(sql_err)?
            .flatten();
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
        let naming = match built {
            Some(built) => Some(claim_naming(&tx, principal, previous, &built.documents)?),
            None => previous,
        };
        tx.execute(
            "INSERT INTO similar_template (principal, fingerprint, segments, same_documents, computed_at, naming)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![principal, print, segments, same, now_ms(), naming],
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
            if let Some(naming) = naming {
                let anchors: Vec<String> = built.fields.iter().filter(|f| f.same_as.is_none()).map(|f| f.anchor.clone()).collect();
                follow_fields(&tx, naming, &anchors)?;
            }
            for (position, field) in built.fields.iter().enumerate() {
                tx.execute(
                    "INSERT INTO similar_template_field (principal, position, kind, ordinal, form_name, same_as, anchor, label)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        principal,
                        position as i64,
                        kind_id(field.kind),
                        field.ordinal as i64,
                        field.form_name,
                        field.same_as.map(|i| i as i64),
                        field.anchor,
                        field.label
                    ],
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
        let mut fields = read_fields(&conn, principal, shown.len())?;
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
        resolve_names(&mut fields);
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
            name: template_name(&conn, principal)?,
            whats: whats(),
            generated: super::generate::generated_with(&conn, naming_now(&conn, principal)?)?,
        }))
    }

    /// Names a template, or gives it back its automatic name (`None` or
    /// empty). Kept apart from the template (`template_naming`): it
    /// survives every time the template is built again.
    pub fn name_template(&self, principal: i64, name: Option<&str>) -> Result<(), String> {
        let conn = self.conn();
        let naming = naming_of(&conn, principal)?;
        let name = name.map(str::trim).filter(|n| !n.is_empty());
        conn.execute(
            "UPDATE template_naming SET name = ?2, named_at = CASE WHEN ?2 IS NULL THEN NULL ELSE ?3 END WHERE id = ?1",
            params![naming, name, now_ms()],
        )
        .map_err(sql_err)?;
        Ok(())
    }

    /// Records what a person decides about the field at `position` of a
    /// template (the field it says again, when it is one): its name, what
    /// it is, whether it is fixed and with what value. All of it empty: the
    /// field goes back to what the program and the model say. Kept apart
    /// from the template, by the words around the field.
    pub fn choose_template_field(&self, principal: i64, position: i64, choice: &FieldChoice) -> Result<(), String> {
        let conn = self.conn();
        let naming = naming_of(&conn, principal)?;
        let found: Option<(String, Option<i64>)> = conn
            .query_row(
                "SELECT anchor, same_as FROM similar_template_field WHERE principal = ?1 AND position = ?2",
                params![principal, position],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(sql_err)?;
        let Some((mut anchor, same_as)) = found else {
            return Err("Ese campo ya no está en la plantilla".to_string());
        };
        if let Some(original) = same_as {
            anchor = conn
                .query_row(
                    "SELECT anchor FROM similar_template_field WHERE principal = ?1 AND position = ?2",
                    params![principal, original],
                    |row| row.get(0),
                )
                .map_err(sql_err)?;
        }
        let name = choice.name.as_deref().map(str::trim).filter(|n| !n.is_empty()).map(str::to_string);
        let what = choice.what.as_deref().map(str::trim).filter(|w| !w.is_empty()).map(str::to_string);
        if let Some(what) = &what {
            if !whats().contains(what) {
                return Err(format!("'{what}' no es algo que pueda ser un campo"));
            }
        }
        let fixed_value = match (choice.fixed, &choice.fixed_value) {
            (true, Some(value)) => Some(value.chars().take(VALUE_MAX_CHARS).collect::<String>()),
            (true, None) => return Err("Un campo fijo necesita el valor que se escribe siempre".to_string()),
            (false, _) => None,
        };
        if name.is_none() && what.is_none() && !choice.fixed {
            conn.execute("DELETE FROM template_field_choice WHERE naming = ?1 AND anchor = ?2", params![naming, anchor])
                .map_err(sql_err)?;
            return Ok(());
        }
        conn.execute(
            "INSERT OR REPLACE INTO template_field_choice (naming, anchor, name, what, fixed, fixed_value, decided_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![naming, anchor, name, what, choice.fixed, fixed_value, now_ms()],
        )
        .map_err(sql_err)?;
        Ok(())
    }
}

/// The fields of a template in order, each with what a person decided and
/// what the model said of it (by its anchor, in the template's naming), and
/// room for the value of each of `documents` documents.
fn read_fields(conn: &Connection, principal: i64, documents: usize) -> Result<Vec<TemplateField>, String> {
    let mut statement = conn
        .prepare(
            "SELECT f.kind, f.ordinal, f.form_name, f.same_as, f.label,
                    c.name, c.what, coalesce(c.fixed, 0), c.fixed_value, a.name, a.what
             FROM similar_template_field f
             JOIN similar_template t ON t.principal = f.principal
             LEFT JOIN template_field_choice c ON c.naming = t.naming AND c.anchor = f.anchor
             LEFT JOIN template_field_answer a ON a.naming = t.naming AND a.anchor = f.anchor
             WHERE f.principal = ?1 ORDER BY f.position",
        )
        .map_err(sql_err)?;
    let rows = statement
        .query_map([principal], |row| {
            Ok(TemplateField {
                kind: kind_from(&row.get::<_, String>(0)?),
                ordinal: row.get(1)?,
                form_name: row.get(2)?,
                same_as: row.get(3)?,
                values: vec![String::new(); documents],
                name: None,
                name_source: None,
                what: None,
                what_source: None,
                fixed: row.get(7)?,
                fixed_value: row.get(8)?,
                said: Said {
                    label: row.get(4)?,
                    person_name: row.get(5)?,
                    person_what: row.get(6)?,
                    model_name: row.get(9)?,
                    model_what: row.get(10)?,
                },
            })
        })
        .map_err(sql_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_err)?;
    Ok(rows)
}

/// A template as a document is generated from it (`generate`): its text,
/// its fields with what each of its documents says there and what was
/// decided of them, its lasting identity and when it was built.
pub(super) struct ForGeneration {
    pub segments: Vec<Segment>,
    /// With the values of every document of the template.
    pub fields: Vec<TemplateField>,
    pub naming: Option<i64>,
    pub computed_at: i64,
}

/// The template of this principal, to generate a document from it.
pub(super) fn for_generation(conn: &Connection, principal: i64) -> Result<Option<ForGeneration>, String> {
    let row: Option<(String, i64, Option<i64>)> = conn
        .query_row(
            "SELECT segments, computed_at, naming FROM similar_template t WHERE principal = ?1
               AND EXISTS (SELECT 1 FROM similar_template_document d WHERE d.principal = t.principal)",
            [principal],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .map_err(sql_err)?;
    let Some((segments, computed_at, naming)) = row else {
        return Ok(None);
    };
    let documents: Vec<i64> = {
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
    let mut fields = read_fields(conn, principal, documents.len())?;
    {
        let mut statement = conn
            .prepare("SELECT position, content_id, value FROM similar_template_value WHERE principal = ?1")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([principal], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (position, id, value) = row.map_err(sql_err)?;
            if let (Some(field), Some(column)) = (fields.get_mut(position as usize), documents.iter().position(|d| *d == id)) {
                field.values[column] = value;
            }
        }
    }
    resolve_names(&mut fields);
    Ok(Some(ForGeneration {
        segments: serde_json::from_str(&segments).unwrap_or_default(),
        fields,
        naming,
        computed_at,
    }))
}

/// The lasting identity the template of this principal has now, if any.
fn naming_now(conn: &Connection, principal: i64) -> Result<Option<i64>, String> {
    Ok(conn
        .query_row("SELECT naming FROM similar_template WHERE principal = ?1", [principal], |row| row.get(0))
        .optional()
        .map_err(sql_err)?
        .flatten())
}

/// The name a person gave the template of this principal.
fn template_name(conn: &Connection, principal: i64) -> Result<Option<String>, String> {
    Ok(conn
        .query_row(
            "SELECT n.name FROM similar_template t JOIN template_naming n ON n.id = t.naming WHERE t.principal = ?1",
            [principal],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?
        .flatten())
}

/// The lasting identity of the template of this principal, given to it now
/// when it has none (a template built before there were names).
fn naming_of(conn: &Connection, principal: i64) -> Result<i64, String> {
    let found: Option<Option<i64>> = conn
        .query_row(
            "SELECT naming FROM similar_template t WHERE principal = ?1
               AND EXISTS (SELECT 1 FROM similar_template_document d WHERE d.principal = t.principal)",
            [principal],
            |row| row.get(0),
        )
        .optional()
        .map_err(sql_err)?;
    match found {
        None => Err("Esa plantilla ya no está: su familia de documentos cambió".to_string()),
        Some(Some(naming)) => Ok(naming),
        Some(None) => {
            let documents: Vec<i64> = {
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
            let naming = claim_naming(conn, principal, None, &documents)?;
            conn.execute("UPDATE similar_template SET naming = ?2 WHERE principal = ?1", params![principal, naming])
                .map_err(sql_err)?;
            Ok(naming)
        }
    }
}

/// The lasting identity (`template_naming`) of the template of `principal`
/// being built with these documents: the one of its previous build; or
/// else the one no other template has that shares with it more than half
/// of the documents of the smaller of the two (the family grew, lost a
/// document or changed its principal), the one that shares the most first;
/// or else a new one. It remembers these documents.
fn claim_naming(conn: &Connection, principal: i64, previous: Option<i64>, documents: &[i64]) -> Result<i64, String> {
    let found = match previous {
        Some(naming) => Some(naming),
        None => {
            let mut statement = conn
                .prepare(
                    "SELECT id, documents FROM template_naming WHERE id NOT IN (
                        SELECT naming FROM similar_template WHERE naming IS NOT NULL AND principal != ?1)
                     ORDER BY built_at DESC, id DESC",
                )
                .map_err(sql_err)?;
            let rows = statement
                .query_map([principal], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
                .map_err(sql_err)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_err)?;
            let mut best: Option<(usize, i64)> = None;
            for (naming, kept) in rows {
                let kept: Vec<i64> = serde_json::from_str(&kept).unwrap_or_default();
                let shared = kept.iter().filter(|id| documents.contains(id)).count();
                let smaller = kept.len().min(documents.len());
                if shared * 2 > smaller && best.is_none_or(|(most, _)| shared > most) {
                    best = Some((shared, naming));
                }
            }
            best.map(|(_, naming)| naming)
        }
    };
    let now = now_ms();
    let kept = serde_json::to_string(documents).map_err(|e| e.to_string())?;
    match found {
        Some(naming) => {
            conn.execute("UPDATE template_naming SET documents = ?2, built_at = ?3 WHERE id = ?1", params![naming, kept, now])
                .map_err(sql_err)?;
            Ok(naming)
        }
        None => {
            conn.execute("INSERT INTO template_naming (documents, built_at) VALUES (?1, ?2)", params![kept, now]).map_err(sql_err)?;
            Ok(conn.last_insert_rowid())
        }
    }
}

/// What was decided and answered about the fields of a template follows
/// them when it is built again (`reattach`): a field whose words around it
/// changed keeps what a person decided and what the model said of it.
/// What finds no field stays, in case the field comes back.
fn follow_fields(conn: &Connection, naming: i64, anchors: &[String]) -> Result<(), String> {
    for table in ["template_field_choice", "template_field_answer"] {
        let kept: Vec<String> = {
            let mut statement = conn.prepare(&format!("SELECT anchor FROM {table} WHERE naming = ?1")).map_err(sql_err)?;
            let rows = statement
                .query_map([naming], |row| row.get(0))
                .map_err(sql_err)?
                .collect::<Result<Vec<String>, _>>()
                .map_err(sql_err)?;
            rows
        };
        for (old, new) in reattach(&kept, anchors) {
            conn.execute(&format!("UPDATE {table} SET anchor = ?3 WHERE naming = ?1 AND anchor = ?2"), params![naming, old, new])
                .map_err(sql_err)?;
        }
    }
    Ok(())
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
    fn each_field_is_anchored_by_the_shared_words_around_it_never_by_its_values() {
        let made = built(&[
            "Recibí de Ana Torres la suma de $ 1.500,00 el día 10/02/2026 en concepto de honorarios.",
            "Recibí de Bruno Vega la suma de $ 2.750,00 el día 12/03/2026 en concepto de honorarios.",
        ]);
        let anchors: Vec<&str> = made.fields.iter().map(|f| f.anchor.as_str()).collect();
        assert_eq!(anchors, ["recibi de|la suma de _", "_ la suma de|el dia _ en", "de _ el dia|en concepto de honorarios"]);
        // The same words around two fields: told apart by their order.
        let twice = field_anchors(
            &[Segment::Text { text: "Fecha: ".into() }, Segment::Field { field: 0 }, Segment::Text { text: " Fecha: ".into() }, Segment::Field { field: 1 }],
            &[None, None, Some("Nombre".into()), Some("Nombre".into())],
        );
        assert_eq!(twice, ["fecha|fecha _", "fecha _ fecha|", "form:nombre", "form:nombre#2"]);
        assert_eq!(anchor_sides("a b|c d#2"), ("a b", "c d"));
    }

    #[test]
    fn a_clear_label_before_a_field_names_it_and_a_field_in_a_sentence_has_none() {
        assert_eq!(label_before(". Fecha de emisión: ").as_deref(), Some("Fecha de emisión"));
        assert_eq!(label_before("según contrato vigente. Total a pagar: $ ").as_deref(), Some("Total a pagar"));
        assert_eq!(label_before("FACTURA B N° ").as_deref(), Some("Factura B N°"));
        assert_eq!(label_before("DNI N°: ").as_deref(), Some("DNI"));
        assert_eq!(label_before("N° ").as_deref(), Some("Número"));
        assert_eq!(label_before("IVA 21 %: $ ").as_deref(), Some("IVA 21 %"));
        assert_eq!(label_before("DOMICILIO REAL: ").as_deref(), Some("Domicilio real"));
        assert_eq!(label_before("FECHA DE PAGO: ").as_deref(), Some("Fecha de pago"));
        assert_eq!(label_before("CUIT DEL LOCADOR: ").as_deref(), Some("CUIT del locador"));
        // In the middle of a sentence, a long line, data, a list: no clear label.
        assert_eq!(label_before("Recibí de "), None);
        assert_eq!(label_before(" al "), None);
        assert_eq!(label_before("que por la presente se deja constancia de lo siguiente: "), None);
        assert_eq!(label_before("Vence el 10/02/2026: "), None);
        assert_eq!(label_before("Nombre y apellido, DNI: "), None);
        assert_eq!(label_before(""), None);
        // A form's own names, unless they are technical.
        assert_eq!(form_label("Nombre_y_apellido").as_deref(), Some("Nombre y apellido"));
        assert_eq!(form_label("form1[0].page1[0].Domicilio[0]").as_deref(), Some("Domicilio"));
        assert_eq!(form_label("Text1"), None);
        assert_eq!(form_label("Check Box3"), None);
        assert_eq!(form_label("CUIT").as_deref(), Some("CUIT"));
    }

    #[test]
    fn an_invoice_names_its_fields_by_its_labels_and_the_one_in_a_sentence_stays_for_the_model() {
        let invoice = |number: &str, issued: &str, from: &str, to: &str, total: &str| {
            format!("FACTURA B N° {number}. Fecha de emisión: {issued}. Período facturado: {from} al {to}. Total a pagar: $ {total}. Gracias.")
        };
        let made = built(&[
            &invoice("0003-00001234", "15/03/2026", "01/02/2026", "28/02/2026", "145.200,00"),
            &invoice("0003-00001290", "14/04/2026", "01/03/2026", "31/03/2026", "159.115,00"),
        ]);
        let labels: Vec<Option<&str>> = made.fields.iter().map(|f| f.label.as_deref()).collect();
        assert_eq!(labels, [Some("Factura B N°"), Some("Fecha de emisión"), Some("Período facturado"), None, Some("Total a pagar")]);
        // A field said again takes the name of its first place.
        let twice = built(&[
            "Recibo N° 0001-00001234. Talón para el cliente. Recibo N° 0001-00001234. Talón para el banco.",
            "Recibo N° 0001-00005678. Talón para el cliente. Recibo N° 0001-00005678. Talón para el banco.",
        ]);
        assert_eq!(twice.fields[0].label.as_deref(), Some("Recibo N°"));
        assert_eq!(twice.fields[1].label, None);
        // A label on the line before; not across a paragraph.
        let made = built(&["Fecha de alta:\n10/02/2026\n\nObservaciones:\n\nsin novedad", "Fecha de alta:\n12/03/2026\n\nObservaciones:\n\ncon novedad"]);
        let labels: Vec<Option<&str>> = made.fields.iter().map(|f| f.label.as_deref()).collect();
        assert_eq!(labels, [Some("Fecha de alta"), None]);
        // A form's field, by its name there.
        let page = "Ficha de cliente. Complete los datos de la persona y firme al pie.";
        let one = Text::new(1, &text::pages(page), &[], &[], vec![("Nombre".into(), "Ana Torres".into())]);
        let two = Text::new(2, &text::pages(page), &[], &[], vec![("Nombre".into(), "Bruno Vega".into())]);
        assert_eq!(build(&one, &[&two]).fields[0].label.as_deref(), Some("Nombre"));
    }

    #[test]
    fn what_was_kept_follows_its_field_when_the_words_on_one_side_still_say_which() {
        let kept: Vec<String> = ["a b|c d", "x|y", "p|q", "m|n"].map(String::from).to_vec();
        let fields: Vec<String> = ["a b|c e", "z|y", "p|q", "h|k"].map(String::from).to_vec();
        let moved = reattach(&kept, &fields);
        assert_eq!(moved, vec![("a b|c d".to_string(), "a b|c e".to_string()), ("x|y".to_string(), "z|y".to_string())]);
        // Two kept with the same words before, one field: nobody can tell which.
        let kept: Vec<String> = ["a|b", "a|c"].map(String::from).to_vec();
        assert!(reattach(&kept, &["a|d".to_string()]).is_empty());
    }

    fn shown(kind: Kind, said: Said) -> TemplateField {
        TemplateField {
            kind,
            ordinal: 1,
            form_name: None,
            same_as: None,
            values: Vec::new(),
            name: None,
            name_source: None,
            what: None,
            what_source: None,
            fixed: false,
            fixed_value: None,
            said,
        }
    }

    #[test]
    fn a_person_wins_then_the_text_then_the_model_and_alike_names_are_told_apart() {
        let model = |name: &str, what: &str| Said { model_name: Some(name.into()), model_what: Some(what.into()), ..Default::default() };
        let mut fields = vec![
            shown(Kind::Name, Said { person_name: Some("Quien paga".into()), label: Some("Recibí".into()), ..model("Pagador", "persona") }),
            shown(Kind::Date, Said { label: Some("Fecha".into()), ..model("Fecha de pago", "fecha") }),
            shown(Kind::Date, model("Fecha", "fecha")),
            shown(Kind::Name, Said { person_what: Some("organizacion".into()), ..model("Pagador", "persona") }),
            shown(Kind::Number, Said::default()),
            shown(Kind::Name, Said::default()),
        ];
        fields.push(TemplateField { same_as: Some(1), ..shown(Kind::Date, Said::default()) });
        resolve_names(&mut fields);
        let names: Vec<(Option<&str>, Option<&str>)> = fields.iter().map(|f| (f.name.as_deref(), f.name_source.as_deref())).collect();
        assert_eq!(
            names,
            [
                (Some("Quien paga"), Some("person")),
                (Some("Fecha"), Some("label")),
                (Some("Fecha (2)"), Some("model")),
                (Some("Pagador"), Some("model")),
                (None, None),
                (None, None),
                (Some("Fecha"), Some("label")),
            ]
        );
        let whats: Vec<(Option<&str>, Option<&str>)> = fields.iter().map(|f| (f.what.as_deref(), f.what_source.as_deref())).collect();
        assert_eq!(
            whats,
            [
                (Some("persona"), Some("model")),
                (Some("fecha"), Some("model")),
                (Some("fecha"), Some("model")),
                (Some("organizacion"), Some("person")),
                (Some("numero"), Some("kind")),
                (None, None),
                (Some("fecha"), Some("model")),
            ]
        );
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
        let mut labelled = 0;
        let mut with_template = 0;
        for group in &groups {
            let Some(fields) = group.template_fields else {
                println!("family of {} documents: no template (no member of the same template)", group.members.len());
                continue;
            };
            with_template += 1;
            total_fields += fields;
            let template = base.similar_template(&Visible::All, group.principal.location_id).unwrap().unwrap();
            labelled += template.fields.iter().filter(|f| f.same_as.is_none() && f.name_source.as_deref() == Some("label")).count();
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
        println!(
            "families with a template: {with_template} of {}; fields in all: {total_fields}; named by their text at no cost: {labelled}",
            groups.len()
        );
        if let Ok(path) = std::env::var("MICELYA_TEMPLATE_DUMP") {
            std::fs::write(path, dump).unwrap();
        }
    }
}
