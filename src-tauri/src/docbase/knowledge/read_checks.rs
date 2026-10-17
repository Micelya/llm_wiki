//! The program's checks of a reading beyond its quotes (D-244 to D-252).
//!
//! The quote check (`quote.rs`) proves the text is there; these say what
//! the text says with it, by general signals, never by a case (R-24):
//!
//! - **Direction of money.** Who pays and who is paid, by the verb between
//!   the two names of the quote ("X pagará a Y", "Y reclama a X") and the
//!   words right before a creditor ("a favor de").
//! - **Kind of a number by its label.** The label right before a number
//!   ("CUIT", "EIN", "CD", "Envelope Id", "Factura") says what it is.
//! - **Labels of a form.** A word that the text always writes followed by
//!   a colon ("PROVINCIA:") is a label, not a name.
//! - **Defined terms.** "(en adelante «la Empresa»)", "(«la Empresa»)":
//!   the term is a local alias of the entity named right before it.
//! - **Pages of a signing service.** The record a signing service adds to
//!   a document (the marks of the catalog of document types) does not say
//!   the roles of the act.
//!
//! The words of each check are data (`knowledge.json`, `money` and
//! `reading`; the catalog of document types for the signing pages).

use std::collections::BTreeSet;

use super::quote;
use super::vocabulary::Vocabulary;
use crate::source_volume::catalog::{is_page_heading, RECOGNIZED_MARKER_START};

/// Where `needle` starts in `haystack`, every time.
pub fn positions(haystack: &[String], needle: &[String]) -> Vec<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return Vec::new();
    }
    (0..=haystack.len() - needle.len()).filter(|&i| haystack[i..i + needle.len()] == needle[..]).collect()
}

/// Whether a quote holds a name: as one run of words, or (a name of two
/// words or more) with all its words, the way a form writes a name in two
/// fields ("Apellidos: Dos · Nombres: Persona"). D-248.
pub fn name_in_quote(quote_words: &[String], name: &[String]) -> bool {
    quote::contains(quote_words, name) || (name.len() >= 2 && name.iter().all(|w| quote_words.contains(w)))
}

fn lowercase_unaccented(text: &str) -> Vec<char> {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|c| crate::source_volume::catalog::unaccented(c).unwrap_or(c))
        .collect()
}

/// Whether a way of writing of one word is a label of a form: wherever
/// the text writes it, a colon follows ("PROVINCIA:", "Domicilio:").
pub fn is_form_label(body: &str, written: &str) -> bool {
    let words = quote::words(written);
    if words.len() != 1 {
        return false;
    }
    let text = lowercase_unaccented(body);
    let wanted = lowercase_unaccented(written.trim().trim_end_matches(':').trim());
    if wanted.is_empty() {
        return false;
    }
    let mut found = 0;
    let mut i = 0;
    while i + wanted.len() <= text.len() {
        if text[i..i + wanted.len()] == wanted[..] {
            let before_ok = i == 0 || !text[i - 1].is_alphanumeric();
            let after = i + wanted.len();
            let after_ok = after >= text.len() || !text[after].is_alphanumeric();
            if before_ok && after_ok {
                found += 1;
                let next = text[after..].iter().find(|c| !(c.is_whitespace() && **c != '\n'));
                if next != Some(&':') {
                    return false;
                }
            }
            i = after;
        } else {
            i += 1;
        }
    }
    found > 0
}

const OPENING: [(char, &[char]); 4] = [('«', &['»']), ('“', &['”', '"']), ('"', &['"', '”']), ('‘', &['’'])];
const ARTICLES: [&str; 8] = ["el", "la", "los", "las", "lo", "the", "un", "una"];
const DEFINING: [&str; 6] = ["en adelante", "hereinafter", "denominad", "llamad", "referred to as", "designad"];

/// The terms a text defines by their shape ("en adelante «la Empresa»",
/// "(«la Empresa»)", "hereinafter the “Company”"), normalized and without
/// their article, each with the number of words of the text before it.
pub fn defined_terms(body: &str) -> Vec<(String, usize)> {
    let chars: Vec<char> = body.chars().collect();
    let mut out: Vec<(String, usize)> = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let Some((_, closing)) = OPENING.iter().find(|(open, _)| *open == chars[i]) else {
            i += 1;
            continue;
        };
        let Some(length) = chars[i + 1..].iter().take(50).position(|c| closing.contains(c) || *c == '\n') else {
            i += 1;
            continue;
        };
        let end = i + 1 + length;
        if chars[end] == '\n' {
            i += 1;
            continue;
        }
        let term: String = chars[i + 1..end].iter().collect();
        let start = i.saturating_sub(40);
        let before: String = lowercase_unaccented(&chars[start..i].iter().collect::<String>()).into_iter().collect();
        // What comes after the last sentence, parenthesis or quote closed.
        let open_part = match before.char_indices().rev().find(|(_, c)| ['.', ')', '»', '”', '"', '\n'].contains(c)) {
            Some((at, c)) => &before[at + c.len_utf8()..],
            None => &before[..],
        };
        let mut tail = open_part.trim_end().to_string();
        for article in ARTICLES {
            if let Some(cut) = tail.strip_suffix(article) {
                if cut.is_empty() || cut.ends_with(|c: char| !c.is_alphanumeric()) {
                    tail = cut.trim_end().to_string();
                    break;
                }
            }
        }
        let defining = tail.ends_with('(') || DEFINING.iter().any(|d| tail.contains(d));
        let mut words = quote::words(&term);
        while words.first().is_some_and(|w| ARTICLES.contains(&w.as_str())) {
            words.remove(0);
        }
        let letters = term.chars().filter(|c| c.is_alphabetic()).count();
        if defining && !words.is_empty() && words.len() <= 5 && letters >= 2 {
            let at = quote::words(&chars[..i].iter().collect::<String>()).len();
            out.push((words.join(" "), at));
        }
        i = end + 1;
    }
    out
}

/// A name without its leading articles ("la empresa" → "empresa").
pub fn without_article(normalized: &str) -> String {
    let mut words: Vec<&str> = normalized.split_whitespace().collect();
    while words.first().is_some_and(|w| ARTICLES.contains(w)) {
        words.remove(0);
    }
    words.join(" ")
}

/// The pages of a text that are the record of a signing service: those
/// that hold the phrases of a signal of the catalog of document types with
/// one of the marks of `reading.certificateMarks`. A text without pages is
/// one page (`None`).
pub fn certificate_pages(vocabulary: &Vocabulary, body: &str) -> BTreeSet<Option<i64>> {
    let marks = &vocabulary.reading.certificate_marks;
    let signals: Vec<(Vec<Vec<String>>, usize)> = crate::docbase::similar::types::catalog()
        .types
        .iter()
        .flat_map(|t| t.signals.iter())
        .filter(|s| s.mark.as_ref().is_some_and(|m| marks.contains(m)) && !s.phrases.is_empty())
        .map(|s| (s.phrases.iter().map(|p| quote::words(p)).collect(), s.min_phrases.unwrap_or(1).max(1)))
        .collect();
    let mut out = BTreeSet::new();
    if signals.is_empty() {
        return out;
    }
    let mut pages: Vec<(Option<i64>, Vec<String>)> = vec![(None, Vec::new())];
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(RECOGNIZED_MARKER_START) {
            continue;
        }
        if is_page_heading(trimmed) {
            pages.push((trimmed["## Page ".len()..].parse().ok(), Vec::new()));
            continue;
        }
        pages.last_mut().expect("one").1.extend(quote::words(trimmed));
    }
    for (page, words) in pages {
        let holds = signals.iter().any(|(phrases, min)| phrases.iter().filter(|p| quote::contains(&words, p)).count() >= *min);
        if holds {
            out.insert(page);
        }
    }
    out
}

/// Words that may stand between a label and its number ("Nro.", "de").
const FILLERS: [&str; 14] = ["nro", "n", "no", "num", "numero", "number", "de", "del", "id", "tipo", "y", "doc", "the", "o"];

/// The kind of identifier the label right before a number says, in its
/// quote; none when there is no label. Walking back from the number over
/// labels and filler words, a strong kind wins over a weak one ("CUIT del
/// cliente: 30-…" is a CUIT).
pub fn kind_by_label(vocabulary: &Vocabulary, quote_words: &[String], value: &[String]) -> Option<String> {
    let labels: Vec<(&super::vocabulary::Term, Vec<String>)> = vocabulary
        .identifiers
        .iter()
        .flat_map(|t| t.labels.iter().map(move |l| (t, quote::words(l))))
        .filter(|(_, l)| !l.is_empty())
        .collect();
    let mut best: Option<(bool, usize, String)> = None;
    for start in positions(quote_words, value) {
        let mut at = start;
        let mut found: Vec<&super::vocabulary::Term> = Vec::new();
        loop {
            // The longest label that ends right before `at`.
            let label = labels
                .iter()
                .filter(|(_, l)| l.len() <= at && quote_words[at - l.len()..at] == l[..])
                .max_by_key(|(_, l)| l.len());
            if let Some((term, l)) = label {
                found.push(term);
                at -= l.len();
                continue;
            }
            if at > 0 && FILLERS.contains(&quote_words[at - 1].as_str()) && start - at < 4 {
                at -= 1;
                continue;
            }
            break;
        }
        let Some(first) = found.first() else { continue };
        let chosen = found.iter().find(|t| t.strong).unwrap_or(first);
        let candidate = (chosen.strong, start, chosen.id.clone());
        if best.as_ref().is_none_or(|b| (candidate.0, std::cmp::Reverse(candidate.1)) > (b.0, std::cmp::Reverse(b.1))) {
            best = Some(candidate);
        }
    }
    best.map(|(_, _, kind)| kind)
}

fn starts_cue(words: &[String], at: usize, cue: &[String]) -> bool {
    if cue.is_empty() || at + cue.len() > words.len() {
        return false;
    }
    let last = cue.len() - 1;
    words[at..at + last] == cue[..last] && words[at + last].starts_with(cue[last].as_str())
}

const PARTICIPLE_ENDS: [&str; 8] = ["ado", "ada", "ados", "adas", "ido", "ida", "idos", "idas"];
const PASSIVE_AFTER: [&str; 2] = ["por", "by"];

/// What the quote of an amount says of its direction, given the names of
/// the debtor and of the creditor the model gave (normalized words of each
/// of their ways of writing in the document): `Some(true)` it agrees,
/// `Some(false)` it says the other way round, `None` it does not tell (the
/// names are not there, the verb is passive, two verbs say different
/// things).
pub fn money_direction(vocabulary: &Vocabulary, quote_words: &[String], debtor: &[Vec<String>], creditor: &[Vec<String>]) -> Option<bool> {
    let rules = &vocabulary.money;
    // (start, end, is the debtor) of every name, without the overlapping
    // ones ("Uno" inside "Uno S.A." when both name the two parties).
    let mut names: Vec<(usize, usize, bool)> = Vec::new();
    for (list, is_debtor) in [(debtor, true), (creditor, false)] {
        for name in list.iter().filter(|n| !n.is_empty()) {
            for start in positions(quote_words, name) {
                names.push((start, start + name.len(), is_debtor));
            }
        }
    }
    let overlapping = |a: &(usize, usize, bool), b: &(usize, usize, bool)| a.2 != b.2 && a.0 < b.1 && b.0 < a.1;
    let clean: Vec<(usize, usize, bool)> = names.iter().filter(|a| !names.iter().any(|b| overlapping(a, b))).copied().collect();
    if !clean.iter().any(|n| n.2) && !clean.iter().any(|n| !n.2) {
        return None;
    }
    let (mut agree, mut against) = (0, 0);
    // "a favor de <creditor>".
    for marker in rules.creditor_markers.iter().map(|m| quote::words(m)) {
        for at in positions(quote_words, &marker) {
            let after = at + marker.len();
            if let Some(name) = clean.iter().filter(|n| n.0 >= after && n.0 - after <= 2).min_by_key(|n| n.0) {
                if name.2 {
                    against += 1;
                } else {
                    agree += 1;
                }
            }
        }
    }
    let payer: Vec<Vec<String>> = rules.payer_cues.iter().map(|c| quote::words(c)).collect();
    let payee: Vec<Vec<String>> = rules.payee_cues.iter().map(|c| quote::words(c)).collect();
    for i in 0..quote_words.len() {
        let pays = payer.iter().any(|c| starts_cue(quote_words, i, c));
        let paid = payee.iter().any(|c| starts_cue(quote_words, i, c));
        if pays == paid {
            continue;
        }
        let word = &quote_words[i];
        let passive = PARTICIPLE_ENDS.iter().any(|e| word.ends_with(e) && word.len() > e.len() + 2)
            || quote_words.get(i + 1).is_some_and(|w| PASSIVE_AFTER.contains(&w.as_str()));
        if passive || clean.iter().any(|n| n.0 <= i && i < n.1) {
            continue;
        }
        let before = clean.iter().filter(|n| n.1 <= i && i - n.1 <= rules.window).max_by_key(|n| n.1);
        let after = clean.iter().filter(|n| n.0 > i && n.0 - i <= rules.window).min_by_key(|n| n.0);
        let Some(before) = before else { continue };
        let told = match after {
            Some(after) if after.2 != before.2 => Some(before.2),
            // Only the one before, right before the verb ("X adeuda").
            None if i - before.1 <= 2 => Some(before.2),
            _ => None,
        };
        // `told`: whether the one who comes first is the debtor.
        if let Some(first_is_debtor) = told {
            // The one before a verb of paying pays; before a verb of being
            // paid, is paid.
            if first_is_debtor == pays {
                agree += 1;
            } else {
                against += 1;
            }
        }
    }
    match (agree, against) {
        (0, 0) => None,
        (_, 0) => Some(true),
        (0, _) => Some(false),
        _ => None,
    }
}

/// Whether the words right before an amount in its quote say it is about
/// something else (`notCues`: "IVA", "percepción" next to it): a tax, not
/// what one party owes another.
pub fn amount_about_else(not_cues: &[String], quote_words: &[String], amount: &[String]) -> bool {
    positions(quote_words, amount).into_iter().any(|start| {
        quote_words[start.saturating_sub(4)..start].iter().any(|w| not_cues.iter().any(|c| w.starts_with(c.as_str())))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(text: &str) -> Vec<String> {
        quote::words(text)
    }

    #[test]
    fn the_verb_between_the_names_says_who_pays() {
        let v = Vocabulary::builtin();
        let (uno, dos) = (vec![w("Ejemplo Uno S.A.")], vec![w("Persona Dos")]);
        // The model said the debtor is Ejemplo Uno, the creditor Persona Dos.
        assert_eq!(money_direction(&v, &w("Ejemplo Uno S.A. pagará a Persona Dos la suma de $ 100"), &uno, &dos), Some(true));
        assert_eq!(money_direction(&v, &w("Persona Dos le adeuda a Ejemplo Uno S.A. la suma de $ 100"), &uno, &dos), Some(false));
        assert_eq!(money_direction(&v, &w("Persona Dos reclama a Ejemplo Uno S.A. el pago de $ 100"), &uno, &dos), Some(true));
        assert_eq!(money_direction(&v, &w("Ejemplo Uno S.A. intima a Persona Dos a pagar $ 100"), &uno, &dos), Some(false));
        assert_eq!(money_direction(&v, &w("un cheque a favor de Persona Dos por $ 100"), &uno, &dos), Some(true));
        assert_eq!(money_direction(&v, &w("un cheque a favor de Ejemplo Uno S.A. por $ 100"), &uno, &dos), Some(false));
        // Passive, or no names: it does not tell.
        assert_eq!(money_direction(&v, &w("la suma adeudada por Persona Dos a Ejemplo Uno S.A."), &uno, &dos), None);
        assert_eq!(money_direction(&v, &w("Ud. adeuda la suma de $ 100"), &uno, &dos), None);
    }

    #[test]
    fn the_label_before_a_number_says_its_kind() {
        let v = Vocabulary::builtin();
        assert_eq!(kind_by_label(&v, &w("EIN: 12-3456789"), &w("12-3456789")).as_deref(), Some("ein"));
        assert_eq!(kind_by_label(&v, &w("CD Nro. 123456789 AR"), &w("123456789")).as_deref(), Some("envio"));
        assert_eq!(kind_by_label(&v, &w("Envelope Id: ABCD1234EF"), &w("ABCD1234EF")).as_deref(), Some("sobre"));
        assert_eq!(kind_by_label(&v, &w("Factura N° 0001-00001234"), &w("0001-00001234")).as_deref(), Some("factura"));
        assert_eq!(kind_by_label(&v, &w("CUIT del cliente: 30-11111111-1"), &w("30-11111111-1")).as_deref(), Some("cuit"));
        assert_eq!(kind_by_label(&v, &w("con el número 30-11111111-1"), &w("30-11111111-1")), None);
    }

    #[test]
    fn a_word_always_followed_by_a_colon_is_a_label() {
        let body = "PROVINCIA: Ejemplo\nLOCALIDAD: Otra\nNombre: Persona Dos";
        assert!(is_form_label(body, "PROVINCIA"));
        assert!(!is_form_label(body, "Ejemplo"));
        assert!(!is_form_label("La Provincia firmó. PROVINCIA: Ejemplo", "Provincia"));
        assert!(!is_form_label(body, "Persona Dos"));
    }

    #[test]
    fn defined_terms_are_found_by_their_shape() {
        let body = "Ejemplo Uno S.A. (en adelante, la «Empresa») y Persona Dos («la Trabajadora») acuerdan. \
                    Otro Ejemplo LLC (hereinafter the “Company”). Dijo «hola» al pasar.";
        let terms: Vec<String> = defined_terms(body).into_iter().map(|(t, _)| t).collect();
        assert_eq!(terms, vec!["empresa", "trabajadora", "company"]);
    }

    #[test]
    fn a_name_of_a_form_in_two_fields_is_in_its_quote() {
        assert!(name_in_quote(&w("Apellidos: DOS - Nombres: PERSONA"), &w("Persona Dos")));
        assert!(!name_in_quote(&w("Apellidos: DOS"), &w("Persona Dos")));
        // A name of one word is only in its quote as itself.
        assert!(!name_in_quote(&w("Unos y otros"), &w("Uno")));
    }

    #[test]
    fn a_tax_next_to_an_amount_is_not_a_debt() {
        let v = Vocabulary::builtin();
        let not = &v.money_predicate().unwrap().not_cues;
        assert!(amount_about_else(not, &w("Percepción IVA $ 1.000,00 Total $ 11.000,00"), &w("$ 1.000,00")));
        assert!(!amount_about_else(not, &w("Percepción IVA $ 1.000,00 Total $ 11.000,00"), &w("$ 11.000,00")));
    }
}
