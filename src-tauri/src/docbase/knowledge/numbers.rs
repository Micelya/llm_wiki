//! How the resolution of identities compares the numbers of the strong
//! identifiers, and how far two names are (E5 of 2 nov, audit 1C).
//!
//! The owner's rules, general for any identifier, with the words in the
//! vocabulary (`identity.numbers` in `knowledge.json`) and the caps of the
//! names in the base's configuration (`thresholds.rs`), never a name, a
//! number or a case in the code:
//!
//! - A number is taken to a **common form**: without labels, format or
//!   zeros in front; the letters the reader takes for a digit ("s" for 5,
//!   "i" for 1), when they stand alone next to a digit; a registration as
//!   its groups (volume and folio), so "T° 18, F° 751" and "Tomo 18, Folio
//!   751" are one.
//! - Two numbers are **the same**, **near** (one digit misread: changed, one
//!   more or one less; in a registration, one more in front of a group, the
//!   "°" read as a digit) or **other**. A check digit right on both sides
//!   and different numbers are two people, with no tolerance.
//! - A number that is near joins only with a name equal or very like
//!   (`name_distance` within the caps); the same number joins as before
//!   (D-174), and also with a name within the caps.
//! - A number whose quote does not say what it is (no label of its kind)
//!   may join, but never keeps two apart.

use std::collections::BTreeSet;

use super::names::{self, NameKey, ORGANIZATION, PERSON};
use super::quote;
use super::vocabulary::{NumberWords, Vocabulary};

/// The shape of a number: its digits, or its groups (volume and folio).
pub const DIGITS: &str = "cifras";
pub const GROUPS: &str = "tomo-y-folio";

/// How two numbers of one kind compare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Match {
    /// The same number, written otherwise.
    Same,
    /// One digit apart: perhaps one read wrong (the tolerance).
    Near,
    /// Another number.
    Other,
}

/// A text in lower case, without accents.
fn fold(text: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    text.to_lowercase().nfd().filter(|c| !unicode_normalization::char::is_combining_mark(*c)).collect()
}

fn rules(vocabulary: &Vocabulary) -> &NumberWords {
    &vocabulary.identity.numbers
}

/// The shape of a kind of identifier, if it has one.
pub fn shape<'a>(vocabulary: &'a Vocabulary, kind: &str) -> Option<&'a str> {
    rules(vocabulary).shapes.get(kind).map(String::as_str)
}

/// The text with the letters the reader takes for a digit read as that
/// digit, when they stand alone (no letter right before or after) and the
/// nearest character that is not a space is a digit: "760790s" is
/// "7607905"; "C.S.J.N." and "Tomo" stay as they are.
pub fn misread_as_digits(vocabulary: &Vocabulary, value: &str) -> String {
    let table = &rules(vocabulary).misread;
    let chars: Vec<char> = fold(value).chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        let Some(digit) = table.get(&c.to_string()).and_then(|d| d.chars().next()) else {
            out.push(c);
            continue;
        };
        let alone = (i == 0 || !chars[i - 1].is_alphabetic()) && chars.get(i + 1).is_none_or(|n| !n.is_alphabetic());
        let left = chars[..i].iter().rev().find(|x| !x.is_whitespace());
        let right = chars[i + 1..].iter().find(|x| !x.is_whitespace());
        let by_a_digit = left.is_some_and(char::is_ascii_digit) || right.is_some_and(char::is_ascii_digit);
        out.push(if alone && by_a_digit { digit } else { c });
    }
    out
}

fn digits_of(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect::<String>().trim_start_matches('0').to_string()
}

/// The groups of digits of a registration: the letters (the labels "T°",
/// "F°", "Tomo", "Folio", the initials of the body) and the dashes cut
/// them; spaces, dots and commas inside a number do not.
fn groups_of(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_alphabetic() || matches!(c, '-' | '/' | '–' | '—' | 'º' | '°' | 'ª') {
            out.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    out.push(current);
    out.iter().map(|g| digits_of(g)).filter(|g| !g.is_empty()).collect()
}

/// The common form of a number of this kind: its digits, or its groups
/// joined by "-"; a kind without a shape, as written (letters and digits).
/// `None` when nothing is left.
pub fn common_form(vocabulary: &Vocabulary, kind: &str, value: &str) -> Option<String> {
    let form = match shape(vocabulary, kind) {
        Some(DIGITS) => digits_of(&misread_as_digits(vocabulary, value)),
        Some(GROUPS) => groups_of(&misread_as_digits(vocabulary, value)).join("-"),
        _ => names::identifier_value(value),
    };
    (!form.is_empty()).then_some(form)
}

/// How many digits a common form has.
pub fn digit_count(form: &str) -> usize {
    form.chars().filter(char::is_ascii_digit).count()
}

/// Whether the check digit of a kind holds for these digits: `None` when
/// the kind has none or the number has not its length.
pub fn check_digit(vocabulary: &Vocabulary, kind: &str, digits: &str) -> Option<bool> {
    let check = rules(vocabulary).check_digits.get(kind)?;
    let values: Vec<u32> = digits.chars().filter_map(|c| c.to_digit(10)).collect();
    if check.modulo == 0 || values.len() != check.weights.len() + 1 {
        return None;
    }
    let sum: u32 = check.weights.iter().zip(&values).map(|(w, d)| w * d).sum();
    let computed = match check.modulo - sum % check.modulo {
        x if x == check.modulo => 0,
        10 => check.if_ten,
        x => x,
    };
    Some(computed == values[check.weights.len()])
}

/// One group of a registration against another: equal, one digit changed,
/// or one more or one less in front (the "°" read as a digit, "T°18" as
/// "T218"). One more at the end is not: "F° 23" and "F° 234" are two.
fn group_near(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let (x, y): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if x.len() == y.len() {
        return x.iter().zip(&y).filter(|(p, q)| p != q).count() == 1;
    }
    let (short, long) = if x.len() < y.len() { (&x, &y) } else { (&y, &x) };
    long.len() == short.len() + 1 && long[1..] == short[..]
}

/// How two common forms of one kind compare.
pub fn same_number(vocabulary: &Vocabulary, kind: &str, a: &str, b: &str) -> Match {
    if a == b {
        return Match::Same;
    }
    let r = rules(vocabulary);
    match shape(vocabulary, kind) {
        Some(DIGITS) => {
            let (va, vb) = (check_digit(vocabulary, kind, a), check_digit(vocabulary, kind, b));
            if va == Some(true) && vb == Some(true) {
                // Both read right, and different: two of them.
                return Match::Other;
            }
            let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
            let contained_from = r.check_digits.get(kind).map(|c| c.contained_from).unwrap_or(0).max(r.min_digits);
            if short.len() >= contained_from && long.contains(short) {
                // Written with more digits (a prefix, a letter read as one).
                return Match::Same;
            }
            if short.len() >= r.min_digits && names::distance(a, b, 1) <= 1 {
                return Match::Near;
            }
            Match::Other
        }
        Some(GROUPS) => {
            let (ga, gb): (Vec<&str>, Vec<&str>) = (a.split('-').collect(), b.split('-').collect());
            if ga.concat() == gb.concat() {
                return Match::Same;
            }
            if ga.len() >= 2 && gb.len() >= 2 {
                let (ta, tb) = (&ga[ga.len() - 2..], &gb[gb.len() - 2..]);
                let least = ta.concat().len().min(tb.concat().len());
                if least >= r.min_group_digits && ta.iter().zip(tb).all(|(x, y)| group_near(x, y)) {
                    return Match::Near;
                }
                return Match::Other;
            }
            let (ja, jb) = (ga.concat(), gb.concat());
            if ja.len().min(jb.len()) > r.min_group_digits && names::distance(&ja, &jb, 1) <= 1 {
                return Match::Near;
            }
            Match::Other
        }
        // A kind compared as written: the same or another, as before.
        _ if names::written_otherwise(a, b) => Match::Same,
        _ => Match::Other,
    }
}

/// The best way two sets of numbers of one kind meet: `None` when no pair
/// is the same or near (two different numbers of the kind).
pub fn sets_meet(vocabulary: &Vocabulary, kind: &str, a: &BTreeSet<String>, b: &BTreeSet<String>) -> Option<Match> {
    a.iter().flat_map(|x| b.iter().map(move |y| same_number(vocabulary, kind, x, y))).filter(|m| *m != Match::Other).min()
}

/// The number of `inner` kind a number of `outer` kind holds (the DNI in
/// the middle of a CUIT), in its common form.
pub fn held(vocabulary: &Vocabulary, outer: &str, inner: &str, form: &str) -> Option<String> {
    let rule = rules(vocabulary).holds.iter().find(|h| h.id == outer && h.holds == inner)?;
    if form.len() != rule.length || rule.from >= rule.to || rule.to > form.len() || !form.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let middle = digits_of(&form[rule.from..rule.to]);
    (middle.len() >= rules(vocabulary).min_digits).then_some(middle)
}

/// The labels of a kind, as words: the identifier's and the identity's.
fn labels(vocabulary: &Vocabulary, kind: &str) -> Vec<Vec<String>> {
    let own = vocabulary.identifier(kind).map(|t| t.labels.clone()).unwrap_or_default();
    let more = rules(vocabulary).labels.get(kind).cloned().unwrap_or_default();
    own.iter().chain(&more).map(|l| letter_words(l)).filter(|w| !w.is_empty()).collect()
}

/// The runs of letters of a text, folded ("T218-F2751" is "t", "f").
fn letter_words(text: &str) -> Vec<String> {
    let folded = fold(text);
    folded.split(|c: char| !c.is_alphabetic()).filter(|w| !w.is_empty()).map(str::to_string).collect()
}

/// Whether the quote of a number, or the number as written, says it is of
/// this kind: one of its labels.
pub fn labeled(vocabulary: &Vocabulary, kind: &str, quote_text: &str, value: &str) -> bool {
    let mut words = letter_words(quote_text);
    words.extend(letter_words(value));
    labels(vocabulary, kind).iter().any(|label| quote::contains(&words, label))
}

/// The strong kind a number given as another kind ("otro número") says it
/// is by the label it starts with, of two letters or more ("LE 1234 567"
/// is a DNI), when that kind applies to the entity.
pub fn kind_by_its_start(vocabulary: &Vocabulary, entity: &str, value: &str) -> Option<String> {
    let words = letter_words(value);
    let digits = value.chars().any(|c| c.is_ascii_digit());
    if words.is_empty() || !digits {
        return None;
    }
    let mut best: Option<(usize, String)> = None;
    for term in vocabulary.identifiers.iter().filter(|t| t.strong && (t.types.is_empty() || t.types.iter().any(|x| x == entity))) {
        for label in labels(vocabulary, &term.id) {
            let letters: usize = label.iter().map(|w| w.chars().count()).sum();
            if letters >= 2 && words.starts_with(&label) && best.as_ref().is_none_or(|(n, _)| letters > *n) {
                best = Some((letters, term.id.clone()));
            }
        }
    }
    best.map(|(_, kind)| kind)
}

/// The keys two numbers of one kind that may meet share (the number, each
/// way of taking one digit out, the windows of `min_digits` digits; for a
/// registration, its volume and folio each so): candidates are found
/// through them, never all against all.
pub fn keys(vocabulary: &Vocabulary, kind: &str, form: &str) -> Vec<String> {
    let r = rules(vocabulary);
    let less_one = |text: &str| -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut out = vec![text.to_string()];
        for skip in 0..chars.len() {
            out.push(chars.iter().enumerate().filter(|(i, _)| *i != skip).map(|(_, c)| c).collect());
        }
        out
    };
    let mut out: BTreeSet<String> = BTreeSet::new();
    match shape(vocabulary, kind) {
        Some(DIGITS) => {
            out.extend(less_one(form).into_iter().map(|k| format!("d:{k}")));
            let chars: Vec<char> = form.chars().collect();
            if r.min_digits > 0 && chars.len() >= r.min_digits {
                out.extend(chars.windows(r.min_digits).map(|w| format!("w:{}", w.iter().collect::<String>())));
            }
        }
        Some(GROUPS) => {
            let groups: Vec<&str> = form.split('-').collect();
            out.extend(less_one(&groups.concat()).into_iter().map(|k| format!("d:{k}")));
            if groups.len() >= 2 {
                let (volume, folio) = (groups[groups.len() - 2], groups[groups.len() - 1]);
                for v in less_one(volume) {
                    for f in less_one(folio) {
                        out.insert(format!("g:{v}-{f}"));
                    }
                }
            }
        }
        _ => {
            out.insert(format!("d:{form}"));
        }
    }
    out.into_iter().collect()
}

/// The caps of the names, from the base's configuration (E5 of 2 nov).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NameCaps {
    pub letters: usize,
    pub share: f64,
}

impl Default for NameCaps {
    fn default() -> Self {
        NameCaps { letters: crate::docbase::thresholds::DEFAULT_NAME_LETTERS, share: crate::docbase::thresholds::DEFAULT_NAME_SHARE }
    }
}

impl NameCaps {
    /// Whether two names this far apart are the same or very like.
    pub fn close(&self, far: Option<(usize, f64)>) -> bool {
        far.is_some_and(|(letters, share)| letters <= self.letters && share <= self.share + 1e-9)
    }
}

/// The words, and the words with two neighbors joined (a scan splits a
/// word or joins two: "Etelv ina", "DINOJA").
fn ways(words: &[String]) -> Vec<Vec<String>> {
    let mut out = vec![words.to_vec()];
    for i in 0..words.len().saturating_sub(1) {
        let mut joined = words[..i].to_vec();
        joined.push(format!("{}{}", words[i], words[i + 1]));
        joined.extend_from_slice(&words[i + 2..]);
        out.push(joined);
    }
    out
}

fn letters(words: &[String]) -> usize {
    words.iter().map(|w| w.chars().count()).sum()
}

fn edit(a: &str, b: &str) -> usize {
    names::distance(a, b, a.chars().count().max(b.chars().count()))
}

/// The distance between two people's names: each word of the shorter with
/// its best partner in the other, in any order; a word left over costs its
/// letters.
fn words_apart(a: &[String], b: &[String]) -> usize {
    let (a, b) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if a.is_empty() {
        return letters(b);
    }
    if b.len() > 7 {
        let sorted = |w: &[String]| {
            let mut w = w.to_vec();
            w.sort();
            w.join(" ")
        };
        return edit(&sorted(a), &sorted(b));
    }
    fn best(a: &[String], b: &[String], at: usize, used: &mut Vec<bool>, cost: usize, found: &mut usize) {
        if cost >= *found {
            return;
        }
        if at == a.len() {
            let left: usize = b.iter().zip(used.iter()).filter(|(_, u)| !**u).map(|(w, _)| w.chars().count()).sum();
            *found = (*found).min(cost + left);
            return;
        }
        for j in 0..b.len() {
            if !used[j] {
                used[j] = true;
                best(a, b, at + 1, used, cost + edit(&a[at], &b[j]), found);
                used[j] = false;
            }
        }
    }
    let mut found = usize::MAX;
    best(a, b, 0, &mut vec![false; b.len()], 0, &mut found);
    found
}

/// How far two names of one kind are: (letters, share of the longer name).
/// `None` when one has no name, or two organizations have two legal forms.
pub fn name_distance(kind: &str, a: &NameKey, b: &NameKey) -> Option<(usize, f64)> {
    if !a.has_name() || !b.has_name() {
        return None;
    }
    let far = match kind {
        PERSON => ways(&a.words).iter().flat_map(|x| ways(&b.words).into_iter().map(move |y| words_apart(x, &y))).min()?,
        ORGANIZATION => {
            if matches!((&a.form, &b.form), (Some(x), Some(y)) if x != y) {
                return None;
            }
            ways(&a.words).iter().flat_map(|x| ways(&b.words).into_iter().map(move |y| edit(&x.join(" "), &y.join(" ")))).min()?
        }
        _ => return None,
    };
    let longest = letters(&a.words).max(letters(&b.words)).max(1);
    Some((far, far as f64 / longest as f64))
}

/// The nearest two names of two lists are.
pub fn nearest(kind: &str, a: &[NameKey], b: &[NameKey]) -> Option<(usize, f64)> {
    a.iter()
        .flat_map(|x| b.iter().filter_map(move |y| name_distance(kind, x, y)))
        .min_by(|p, q| p.0.cmp(&q.0).then(p.1.total_cmp(&q.1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v() -> Vocabulary {
        Vocabulary::builtin()
    }

    fn form(kind: &str, value: &str) -> String {
        common_form(&v(), kind, value).unwrap()
    }

    fn meet(kind: &str, a: &str, b: &str) -> Match {
        same_number(&v(), kind, &form(kind, a), &form(kind, b))
    }

    #[test]
    fn a_registration_written_without_format_or_misread_is_the_same_or_near() {
        // Invented numbers, written the ways the scans of the folder write one.
        assert_eq!(form("matricula", "T° 12, F° 345"), "12-345");
        assert_eq!(form("matricula", "Tomo 12, Folio 345"), "12-345");
        assert_eq!(meet("matricula", "T° 12, F° 345", "Tomo 12, Folio 345"), Match::Same);
        assert_eq!(meet("matricula", "Tº12, Fº345", "Tº 12, Fº 345 CPACF"), Match::Same, "the initials of the body are a label");
        // The "°" read as a digit in front: near.
        assert_eq!(form("matricula", "T212-F2345"), "212-2345");
        assert_eq!(meet("matricula", "T° 12, F° 345", "T212-F2345"), Match::Near);
        // A letter at the end read for a digit, the digits split by spaces.
        assert_eq!(form("matricula", "T21 2 - F3 3 4 s"), "212-3345");
        assert_eq!(meet("matricula", "T° 12, F° 345", "T21 2 - F3 3 4 s"), Match::Near);
        // A digit changed.
        assert_eq!(meet("matricula", "T° 12, F° 345", "Tomo 12, Folio 348"), Match::Near);
        // The initials of a court and a letter for the volume: near.
        assert_eq!(meet("matricula", "T° 12, F° 345", "C.S.J.N. T M 2-P 345"), Match::Near);
        // One more digit at the end is another folio; another volume, another.
        assert_eq!(meet("matricula", "T° 1, F° 23 CPA", "T° 1, F° 234"), Match::Other);
        assert_eq!(meet("matricula", "T° 12, F° 345", "T° 47, F° 345"), Match::Other);
        // "Tomo" and "C.S.J.N." keep their letters.
        assert_eq!(misread_as_digits(&v(), "C.S.J.N. Tomo 1s"), "c.s.j.n. tomo 15");
    }

    #[test]
    fn a_document_number_takes_the_readers_confusions_and_its_labels_out() {
        assert_eq!(form("dni", "LE 1234 56s"), "1234565");
        assert_eq!(meet("dni", "LE 1234 565", "L.E. 1234 56s"), Match::Same);
        assert_eq!(form("dni", "D.N.I.N°0 12.345.678"), "12345678");
        assert_eq!(meet("dni", "DNI 12.345.678", "D.N.I.N012345678"), Match::Same, "the «°» read as 0 and the letter in front");
        assert_eq!(meet("dni", "12.345.678", "i2.345.678"), Match::Same, "an i for a 1");
        assert_eq!(meet("dni", "12.345.678", "12.345.679"), Match::Near);
        assert_eq!(meet("dni", "12.345.678", "12.399.999"), Match::Other);
        assert_eq!(meet("dni", "12345", "12346"), Match::Other, "too short to tolerate");
        assert_eq!(meet("dni", "12.345.678", "M 12.345.678"), Match::Same);
    }

    #[test]
    fn the_check_digit_of_a_tax_number_says_two_of_them_or_one_misread() {
        let v = v();
        // 20-12345678-6 is right: 2·5+0·4+1·3+2·2+3·7+4·6+5·5+6·4+7·3+8·2 = 148; 11 − 148 mod 11 = 6.
        assert_eq!(check_digit(&v, "cuit", "20123456786"), Some(true));
        assert_eq!(check_digit(&v, "cuit", "20123456787"), Some(false));
        assert_eq!(check_digit(&v, "dni", "12345678"), None);
        // Two right and different: two people.
        let right_other = (0..10).map(|d| format!("2012345679{d}")).find(|n| check_digit(&v, "cuit", n) == Some(true)).unwrap();
        assert_eq!(same_number(&v, "cuit", "20123456786", &right_other), Match::Other);
        // One right and one with a digit misread (its check fails): near.
        assert_eq!(same_number(&v, "cuit", "20123456786", "20123456186"), Match::Near);
        // Written without the prefix of its type: the same.
        assert_eq!(same_number(&v, "cuit", "20123456786", "123456786"), Match::Same);
        // A person's CUIT holds the DNI.
        assert_eq!(held(&v, "cuit", "dni", "20123456786").as_deref(), Some("12345678"));
        assert_eq!(held(&v, "cuit", "dni", "2012345678"), None);
    }

    #[test]
    fn a_number_is_labeled_by_its_quote_and_another_number_takes_its_kind_by_its_label() {
        let v = v();
        assert!(labeled(&v, "matricula", "abogado T° 12 F° 345", "T° 12 F° 345"));
        assert!(labeled(&v, "matricula", "Mat. 12345", "12345"));
        assert!(!labeled(&v, "matricula", "Comprobante de pago 0012345", "0012345"));
        assert!(labeled(&v, "dni", "Doc. Ident. LE 1234 565", "1234 565"));
        assert_eq!(kind_by_its_start(&v, PERSON, "LE 1234 565").as_deref(), Some("dni"));
        assert_eq!(kind_by_its_start(&v, PERSON, "L.C. 1.234.565").as_deref(), Some("dni"));
        assert_eq!(kind_by_its_start(&v, PERSON, "Tomo 12 Folio 345").as_deref(), Some("matricula"));
        assert_eq!(kind_by_its_start(&v, PERSON, "∠E 1234 565"), None, "a misread label says nothing");
        assert_eq!(kind_by_its_start(&v, ORGANIZATION, "LE 1234 565"), None, "a DNI is a person's");
    }

    #[test]
    fn numbers_that_may_meet_share_a_key() {
        let v = v();
        let share = |kind: &str, a: &str, b: &str| {
            let (ka, kb): (BTreeSet<String>, BTreeSet<String>) = (keys(&v, kind, &form(kind, a)).into_iter().collect(), keys(&v, kind, &form(kind, b)).into_iter().collect());
            !ka.is_disjoint(&kb)
        };
        assert!(share("dni", "12.345.678", "12.345.679"));
        assert!(share("dni", "12.345.678", "112.345.678"));
        assert!(share("cuit", "123456786", "20123456786"));
        assert!(share("matricula", "T° 12, F° 345", "T212-F2345"));
        assert!(share("matricula", "T° 12, F° 345", "Tomo 12, Folio 348"));
        assert!(!share("dni", "12.345.678", "40.999.111"));
    }

    fn k(kind: &str, name: &str) -> NameKey {
        names::key(&Vocabulary::builtin(), kind, name)
    }

    #[test]
    fn names_are_compared_by_their_letters_in_any_order_split_or_joined() {
        let d = |a: &str, b: &str| name_distance(PERSON, &k(PERSON, a), &k(PERSON, b)).map(|x| x.0);
        assert_eq!(d("Persona Uno Ejemplar", "EJEMPLAR, Persona Uno"), Some(0));
        assert_eq!(d("Persona Uno Ejemplar", "Persona Uno Ejemplor"), Some(1));
        assert_eq!(d("Personna Uno Ejemplar", "Persona Uno Ejemplor"), Some(2));
        assert_eq!(d("Persana Uma Ejemplor", "Persona Uno Ejemplar"), Some(4));
        // A word split by the scan, or two joined.
        assert_eq!(d("Ejemplar Persona Unodos", "EJEMPLAR Persona Uno.dos"), Some(0));
        // A word of more costs its letters.
        assert_eq!(d("Persona Ejemplar", "Persona Uno Ejemplar"), Some(3));
        let caps = NameCaps::default();
        let close = |a: &str, b: &str| caps.close(name_distance(PERSON, &k(PERSON, a), &k(PERSON, b)));
        assert!(close("Personna Uno Ejemplar", "Persona Uno Ejemplor"), "2 letters of 19");
        assert!(!close("Persana Una Ejemplor", "Persona Uno Ejemplar"), "3 letters");
        assert!(!close("Ana Pez", "Ina Paz"), "2 letters, but 33 % of a short name");
        // Two legal forms are two organizations.
        assert_eq!(name_distance(ORGANIZATION, &k(ORGANIZATION, "Ejemplo Uno S.A."), &k(ORGANIZATION, "Ejemplo Uno LLC")), None);
        assert_eq!(name_distance(ORGANIZATION, &k(ORGANIZATION, "Ejemplo Uno S.A."), &k(ORGANIZATION, "Ejempla Uno SA")).map(|x| x.0), Some(1));
    }
}
