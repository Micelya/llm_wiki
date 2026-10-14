//! What changes between two aligned texts, and what kind of thing it is.
//!
//! The rule that orders everything (proposal, section 1): what matters
//! is less how much text two documents share than what changes between
//! them. Each changed token is told apart, by its form and without a
//! model, as:
//!
//! - an **identifier**: CUIT/CUIL (`NN-NNNNNNNN-N`), a DNI, a receipt or
//!   invoice number, a shipment number made of letters and digits;
//! - an **amount**, a **date** or another **number**;
//! - a **name**: a word with a capital letter that does not open a
//!   sentence (an approximation: there is no model here);
//! - **wording**: anything else.
//!
//! And across the whole text, the **repeated substitution**: where one
//! text says a name the other says always another, it is the same model
//! with another party.

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};

use super::align::Op;
use super::text::{Prepared, Raw};

/// Repeated substitutions are looked for among changes this short.
const SUBSTITUTION_MAX_WORDS: usize = 4;
/// Examples kept as evidence of a pair.
const MAX_EXAMPLES: usize = 12;
const EXAMPLE_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Identifier,
    Amount,
    Date,
    Number,
    Name,
    /// The value of a form field.
    Field,
    Wording,
}

impl Kind {
    pub fn is_data(self) -> bool {
        self != Kind::Wording
    }
}

const MONTHS: &[&str] = &[
    "enero", "febrero", "marzo", "abril", "mayo", "junio", "julio", "agosto", "septiembre", "setiembre",
    "octubre", "noviembre", "diciembre", "january", "february", "march", "april", "june", "july", "august",
    "september", "october", "november", "december",
];

const CURRENCIES: &[&str] = &["usd", "ars", "u$s", "us$", "eur", "pesos", "dolares", "dollars"];

/// The kind of thing a token of the original text is.
pub fn classify(raw: &Raw) -> Kind {
    let token = raw.text.as_str();
    if token.is_empty() {
        return Kind::Wording;
    }
    let lower = token.to_lowercase();
    if token.contains('$') || CURRENCIES.contains(&lower.as_str()) {
        return Kind::Amount;
    }
    let digits = token.chars().filter(char::is_ascii_digit).count();
    let letters = token.chars().filter(|c| c.is_alphabetic()).count();
    if digits == 0 {
        if MONTHS.contains(&lower.as_str()) {
            return Kind::Date;
        }
        if is_name(raw, letters) {
            return Kind::Name;
        }
        return Kind::Wording;
    }
    if letters > 0 {
        // Letters and digits together: a shipment or document number; a
        // short one is an ordinal or a unit ("1º", "30días").
        return if digits >= 4 && token.chars().count() >= 6 { Kind::Identifier } else { Kind::Number };
    }
    if token.ends_with('%') {
        return Kind::Number;
    }
    if is_date(token) {
        return Kind::Date;
    }
    if is_cuit(token) || is_dotted_dni(token) {
        return Kind::Identifier;
    }
    if is_amount(token) {
        return Kind::Amount;
    }
    if token.contains('-') && digits >= 8 {
        return Kind::Identifier;
    }
    if token.chars().all(|c| c.is_ascii_digit()) {
        if digits >= 7 {
            return Kind::Identifier;
        }
        if digits == 4 && (1900..=2100).contains(&token.parse::<u32>().unwrap_or(0)) {
            return Kind::Date;
        }
    }
    Kind::Number
}

fn is_name(raw: &Raw, letters: usize) -> bool {
    if raw.sentence_start || letters < 2 || letters != raw.text.chars().count() {
        return false;
    }
    let mut chars = raw.text.chars();
    let first_upper = chars.next().is_some_and(char::is_uppercase);
    let rest: Vec<char> = chars.collect();
    let capitalized = rest.iter().any(|c| c.is_lowercase());
    let all_caps = rest.iter().all(|c| c.is_uppercase());
    first_upper && (capitalized || (all_caps && letters >= 3))
}

fn digit_groups(token: &str, separators: &[char]) -> Vec<usize> {
    token
        .split(|c: char| separators.contains(&c))
        .map(|part| if part.chars().all(|c| c.is_ascii_digit()) { part.len() } else { usize::MAX })
        .collect()
}

fn is_date(token: &str) -> bool {
    let groups = digit_groups(token, &['/', '.', '-']);
    match groups.as_slice() {
        [d, m, y] => ((1..=2).contains(d) && (1..=2).contains(m) && (*y == 2 || *y == 4)) || (*d == 4 && (1..=2).contains(m) && (1..=2).contains(y)),
        _ => false,
    }
}

fn is_cuit(token: &str) -> bool {
    digit_groups(token, &['-']) == [2, 8, 1]
}

fn is_dotted_dni(token: &str) -> bool {
    matches!(digit_groups(token, &['.']).as_slice(), [first, 3, 3] if (1..=2).contains(first))
}

fn is_amount(token: &str) -> bool {
    // 1.234,56 · 1234,56 · 1,234.56 · 1.234 · 12.50
    let decimal = |part: &str| part.len() == 2 && part.chars().all(|c| c.is_ascii_digit());
    if let Some((whole, cents)) = token.rsplit_once(',') {
        if decimal(cents) {
            return whole.chars().all(|c| c.is_ascii_digit() || c == '.');
        }
    }
    if let Some((whole, cents)) = token.rsplit_once('.') {
        if decimal(cents) && whole.chars().all(|c| c.is_ascii_digit() || c == ',') {
            return true;
        }
    }
    let groups = digit_groups(token, &['.']);
    groups.len() >= 2 && groups[0] <= 3 && groups[1..].iter().all(|g| *g == 3)
}

/// One substitution seen more than once.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Substitution {
    pub from: String,
    pub to: String,
    pub times: usize,
}

/// One change, as shown to a person.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Example {
    pub kind: Kind,
    /// What the first document says (empty: it has nothing there).
    pub a: String,
    pub b: String,
}

/// What changes between two documents.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Changes {
    /// Words of the first document found, in order, in the second.
    pub matched_words: usize,
    /// Stretches that differ.
    pub hunks: usize,
    /// Words only in the first, and only in the second.
    pub removed_words: usize,
    pub added_words: usize,
    /// Changed tokens that are data, and that are wording (the larger
    /// side of each change).
    pub data_tokens: usize,
    pub wording_tokens: usize,
    /// The same, only in changes where both sides have something: what
    /// was replaced in the shared text, leaving out blocks only one has
    /// (a clause added, a signature block filled in).
    pub replaced_data_tokens: usize,
    pub replaced_wording_tokens: usize,
    /// How many things of each kind change.
    pub items: BTreeMap<Kind, usize>,
    /// Size, in words, of the longest change and of the largest block
    /// that only one of them has.
    pub longest_hunk: usize,
    pub largest_block: usize,
    pub repeated: Vec<Substitution>,
    /// Changes where both sides hold data: one value replaced by another
    /// (a CUIT for another CUIT), by the kinds involved.
    pub swaps: BTreeMap<Kind, usize>,
    pub examples: Vec<Example>,
}

impl Changes {
    #[cfg(test)]
    pub fn data_share(&self) -> f64 {
        let total = self.data_tokens + self.wording_tokens;
        if total == 0 {
            0.0
        } else {
            self.data_tokens as f64 / total as f64
        }
    }

    /// Share of data among what was replaced in the shared text.
    pub fn replaced_data_share(&self) -> f64 {
        let total = self.replaced_data_tokens + self.replaced_wording_tokens;
        if total == 0 {
            0.0
        } else {
            self.replaced_data_tokens as f64 / total as f64
        }
    }

    pub fn count(&self, kind: Kind) -> usize {
        self.items.get(&kind).copied().unwrap_or(0)
    }

    pub fn swapped(&self, kind: Kind) -> usize {
        self.swaps.get(&kind).copied().unwrap_or(0)
    }

    /// Kinds of data that change (identifiers, amounts, names…).
    pub fn data_items(&self) -> usize {
        self.items.iter().filter(|(kind, _)| kind.is_data()).map(|(_, n)| n).sum()
    }
}

/// The raw tokens a range of words came from, in order, without repeats.
fn raws_of(prepared: &Prepared, words: &std::ops::Range<usize>) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::new();
    for word in &prepared.words[words.clone()] {
        if out.last() != Some(&word.raw) {
            out.push(word.raw);
        }
    }
    out
}

fn joined(prepared: &Prepared, raws: &[usize]) -> String {
    let text = raws.iter().map(|&r| prepared.raws[r].text.as_str()).collect::<Vec<_>>().join(" ");
    if text.chars().count() > EXAMPLE_CHARS {
        format!("{}…", text.chars().take(EXAMPLE_CHARS).collect::<String>())
    } else {
        text
    }
}

fn key_of(prepared: &Prepared, words: &std::ops::Range<usize>) -> String {
    prepared.words[words.clone()].iter().map(|w| w.key.as_str()).collect::<Vec<_>>().join(" ")
}

/// Reads the alignment of `a` and `b` and says what changes.
pub fn summarize(a: &Prepared, b: &Prepared, ops: &[Op]) -> Changes {
    let mut changes = Changes::default();
    // Repeated substitutions first: their words count as data.
    let mut substitutions: HashMap<(String, String), usize> = HashMap::new();
    for op in ops {
        if let Op::Change { a: ra, b: rb } = op {
            if !ra.is_empty() && !rb.is_empty() && ra.len() <= SUBSTITUTION_MAX_WORDS && rb.len() <= SUBSTITUTION_MAX_WORDS {
                *substitutions.entry((key_of(a, ra), key_of(b, rb))).or_default() += 1;
            }
        }
    }

    for op in ops {
        match op {
            Op::Same { a: ra, .. } => changes.matched_words += ra.len(),
            Op::Change { a: ra, b: rb } => {
                changes.hunks += 1;
                changes.removed_words += ra.len();
                changes.added_words += rb.len();
                changes.longest_hunk = changes.longest_hunk.max(ra.len().max(rb.len()));
                if ra.is_empty() || rb.is_empty() {
                    changes.largest_block = changes.largest_block.max(ra.len().max(rb.len()));
                }
                let repeated = substitutions.get(&(key_of(a, ra), key_of(b, rb))).is_some_and(|times| *times > 1);

                let raws_a = raws_of(a, ra);
                let raws_b = raws_of(b, rb);
                let kinds_a: Vec<Kind> = raws_a.iter().map(|&r| classify(&a.raws[r])).collect();
                let kinds_b: Vec<Kind> = raws_b.iter().map(|&r| classify(&b.raws[r])).collect();
                let mut per_kind: BTreeMap<Kind, (usize, usize)> = BTreeMap::new();
                for kind in &kinds_a {
                    per_kind.entry(*kind).or_default().0 += 1;
                }
                for kind in &kinds_b {
                    per_kind.entry(*kind).or_default().1 += 1;
                }
                let mut data = 0;
                let mut wording = 0;
                // A value replaced by another: data on both sides, even of
                // two kinds ("$ 1.500" read as an amount, "1.500" as a figure).
                let data_a = kinds_a.iter().any(|kind| kind.is_data());
                let data_b = kinds_b.iter().any(|kind| kind.is_data());
                for (kind, (in_a, in_b)) in &per_kind {
                    if kind.is_data() && (*in_a > 0 || *in_b > 0) && data_a && data_b {
                        *changes.swaps.entry(*kind).or_default() += 1;
                    }
                    let n = (*in_a).max(*in_b);
                    if kind.is_data() || repeated {
                        data += n;
                    } else {
                        wording += n;
                    }
                }
                changes.data_tokens += data;
                changes.wording_tokens += wording;
                if !ra.is_empty() && !rb.is_empty() {
                    changes.replaced_data_tokens += data;
                    changes.replaced_wording_tokens += wording;
                }

                // A run of number tokens is one amount or one date ("$ 1.234,56").
                let mut counted: BTreeMap<Kind, usize> = BTreeMap::new();
                for kinds in [&kinds_a, &kinds_b] {
                    let mut here: BTreeMap<Kind, usize> = BTreeMap::new();
                    let mut previous: Option<Kind> = None;
                    for kind in kinds.iter() {
                        if previous != Some(*kind) {
                            *here.entry(*kind).or_default() += 1;
                        }
                        previous = Some(*kind);
                    }
                    for (kind, n) in here {
                        let slot = counted.entry(kind).or_default();
                        *slot = (*slot).max(n);
                    }
                }
                // Wording is counted by changes, not by tokens.
                if let Some(slot) = counted.get_mut(&Kind::Wording) {
                    *slot = 1;
                }
                if repeated && counted.keys().all(|kind| *kind == Kind::Wording) {
                    counted = BTreeMap::from([(Kind::Name, 1)]);
                }
                for (kind, n) in &counted {
                    *changes.items.entry(*kind).or_default() += n;
                }

                if changes.examples.len() < MAX_EXAMPLES {
                    let kind = counted
                        .keys()
                        .copied()
                        .find(|kind| kind.is_data())
                        .unwrap_or(Kind::Wording);
                    changes.examples.push(Example { kind, a: joined(a, &raws_a), b: joined(b, &raws_b) });
                }
            }
        }
    }

    let mut repeated: Vec<Substitution> = substitutions
        .into_iter()
        .filter(|(_, times)| *times > 1)
        .map(|((from, to), times)| Substitution { from, to, times })
        .collect();
    repeated.sort_by(|x, y| y.times.cmp(&x.times).then_with(|| x.from.cmp(&y.from)));
    repeated.truncate(MAX_EXAMPLES);
    changes.repeated = repeated;
    changes
}

/// Adds what differs between the filled-in fields of two forms. Every
/// difference there is data: the page is the template, the fields are
/// what was filled in.
pub fn add_fields(changes: &mut Changes, a: &[(String, String)], b: &[(String, String)]) {
    let index = |fields: &[(String, String)]| -> BTreeMap<String, Vec<String>> {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (name, value) in fields {
            map.entry(name.clone()).or_default().push(value.clone());
        }
        map
    };
    let (ia, ib) = (index(a), index(b));
    let names: std::collections::BTreeSet<&String> = ia.keys().chain(ib.keys()).collect();
    for name in names {
        let empty = Vec::new();
        let (va, vb) = (ia.get(name).unwrap_or(&empty), ib.get(name).unwrap_or(&empty));
        for position in 0..va.len().max(vb.len()) {
            let (x, y) = (va.get(position), vb.get(position));
            if x == y {
                continue;
            }
            let sample = x.or(y).cloned().unwrap_or_default();
            let kind = sample
                .split_whitespace()
                .map(|token| classify(&Raw { text: token.to_string(), sentence_start: false }))
                .find(|kind| kind.is_data())
                .unwrap_or(Kind::Field);
            changes.hunks += 1;
            changes.data_tokens += 1;
            *changes.items.entry(kind).or_default() += 1;
            if x.is_some() && y.is_some() {
                *changes.swaps.entry(kind).or_default() += 1;
                changes.replaced_data_tokens += 1;
            }
            if changes.examples.len() < MAX_EXAMPLES {
                changes.examples.push(Example {
                    kind,
                    a: x.cloned().unwrap_or_default(),
                    b: y.cloned().unwrap_or_default(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::similar::align::{align, intern};
    use crate::docbase::similar::text::pages;

    fn raw(text: &str) -> Raw {
        Raw { text: text.to_string(), sentence_start: false }
    }

    #[test]
    fn tokens_are_told_apart_by_their_form() {
        assert_eq!(classify(&raw("20-12345678-9")), Kind::Identifier);
        assert_eq!(classify(&raw("12.345.678")), Kind::Identifier);
        assert_eq!(classify(&raw("0001-00012345")), Kind::Identifier);
        assert_eq!(classify(&raw("XY123456789AR")), Kind::Identifier);
        assert_eq!(classify(&raw("00099B00012345")), Kind::Identifier);
        assert_eq!(classify(&raw("1.234,56")), Kind::Amount);
        assert_eq!(classify(&raw("$")), Kind::Amount);
        assert_eq!(classify(&raw("USD")), Kind::Amount);
        assert_eq!(classify(&raw("15/03/2026")), Kind::Date);
        assert_eq!(classify(&raw("2026-03-15")), Kind::Date);
        assert_eq!(classify(&raw("marzo")), Kind::Date);
        assert_eq!(classify(&raw("2026")), Kind::Date);
        assert_eq!(classify(&raw("30")), Kind::Number);
        assert_eq!(classify(&raw("Gómez")), Kind::Name);
        assert_eq!(classify(&raw("GÓMEZ")), Kind::Name);
        assert_eq!(classify(&Raw { text: "Por".into(), sentence_start: true }), Kind::Wording);
        assert_eq!(classify(&raw("plazo")), Kind::Wording);
    }

    fn compare(a: &str, b: &str) -> Changes {
        let pa = Prepared::new(&pages(a), &[], &[]);
        let pb = Prepared::new(&pages(b), &[], &[]);
        let (ia, ib) = intern(pa.words.iter().map(|w| w.key.as_str()), pb.words.iter().map(|w| w.key.as_str()));
        summarize(&pa, &pb, &align(&ia, &ib))
    }

    #[test]
    fn an_amount_and_a_date_are_data_and_wording_is_not() {
        let changes = compare(
            "Se abonará la suma de $ 1.500,00 el día 10/02/2026 por transferencia bancaria.",
            "Se abonará la suma de $ 2.750,00 el día 10/03/2026 mediante transferencia bancaria.",
        );
        assert_eq!(changes.count(Kind::Amount), 1);
        assert_eq!(changes.count(Kind::Date), 1);
        assert_eq!(changes.count(Kind::Wording), 1);
        assert!(changes.data_share() >= 0.5);
    }

    #[test]
    fn a_repeated_substitution_is_found_and_counts_as_data() {
        let changes = compare(
            "Entre la empresa y Ana Torres se acuerda. Ana Torres acepta. Firma Ana Torres.",
            "Entre la empresa y Bruno Vega se acuerda. Bruno Vega acepta. Firma Bruno Vega.",
        );
        assert_eq!(changes.repeated.len(), 1);
        assert_eq!(changes.repeated[0].times, 3);
        assert_eq!(changes.repeated[0].from, "ana torres");
        assert_eq!(changes.data_share(), 1.0);
    }

    #[test]
    fn form_fields_are_data() {
        let mut changes = Changes::default();
        add_fields(
            &mut changes,
            &[("Nombre".into(), "Ana Torres".into()), ("Fecha".into(), "01/02/2026".into())],
            &[("Nombre".into(), "Bruno Vega".into()), ("Fecha".into(), "01/02/2026".into())],
        );
        assert_eq!(changes.hunks, 1);
        assert_eq!(changes.count(Kind::Name), 1);
    }
}
