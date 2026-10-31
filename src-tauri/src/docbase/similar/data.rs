//! Pass 5: the data that matter in a document, by the rules of its type.
//!
//! The catalog says, for each type, which data interest and with what
//! general pattern they are found (`types::DataRule`): the value of a
//! kind (an identifier, an amount, a date, a number, the rest of a line)
//! a few tokens after a phrase, a token of a given shape, or the value of
//! a mark already verified. No model here: the model is asked only for
//! the required data the rules did not find (`classify.rs`).

use serde::{Deserialize, Serialize};

use super::changes::{classify, Kind};
use super::marks::Mark;
use super::text::{normalized, pages, Raw};
use super::types::{has_shape, DataRule};

/// Tokens looked at after a phrase when the rule does not say.
pub const WINDOW: usize = 12;

/// One value of one datum of a document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Datum {
    pub field: String,
    pub value: String,
    /// "rule", "mark" or "model".
    pub source: String,
}

/// A token of the text, where it is, and its normalized words.
struct Token {
    raw: Raw,
    line: usize,
    words: Vec<String>,
}

fn tokens(body: &str) -> Vec<Token> {
    let mut out = Vec::new();
    let mut line_index = 0;
    for page in pages(body) {
        for line in page {
            line_index += 1;
            let mut first = true;
            for piece in line.split_whitespace() {
                let text = piece.trim_matches(|c: char| !(c.is_alphanumeric() || c == '$' || c == '%')).to_string();
                let words: Vec<String> = normalized(piece).split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect();
                out.push(Token { raw: Raw { text, sentence_start: first, numbering: false }, line: line_index, words });
                first = false;
            }
        }
        line_index += 1;
    }
    out
}

/// Positions (token index of the last word) where `phrase` ends.
fn phrase_ends(tokens: &[Token], phrase: &str) -> Vec<usize> {
    let wanted: Vec<String> = normalized(phrase).split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    let flat: Vec<(&str, usize)> =
        tokens.iter().enumerate().flat_map(|(i, t)| t.words.iter().map(move |w| (w.as_str(), i))).collect();
    let mut out = Vec::new();
    if flat.len() < wanted.len() {
        return out;
    }
    for start in 0..=flat.len() - wanted.len() {
        if flat[start..start + wanted.len()].iter().zip(&wanted).all(|((word, _), want)| word == want) {
            out.push(flat[start + wanted.len() - 1].1);
        }
    }
    out
}

const CURRENCY_SIGNS: &[&str] = &["$", "u$s", "us$", "usd", "ars", "eur", "pesos", "dolares", "dollars"];

fn is_plain_number(text: &str) -> bool {
    text.chars().any(|c| c.is_ascii_digit()) && text.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',')
}

/// The value of `kind` starting at token `at`, if that token starts one.
fn value_at(tokens: &[Token], at: usize, kind: &str) -> Option<String> {
    let token = &tokens[at];
    let text = token.raw.text.as_str();
    if text.is_empty() {
        return None;
    }
    let found = classify(&token.raw);
    let digits = text.chars().filter(char::is_ascii_digit).count();
    match kind {
        "identifier" => (found == Kind::Identifier).then(|| text.to_string()),
        "number" => (digits > 0 && matches!(found, Kind::Number | Kind::Identifier | Kind::Amount)).then(|| text.to_string()),
        "amount" => {
            let lower = text.to_lowercase();
            if CURRENCY_SIGNS.contains(&lower.as_str()) {
                let next = tokens.get(at + 1)?;
                return is_plain_number(&next.raw.text).then(|| format!("{} {}", text, next.raw.text));
            }
            (found == Kind::Amount && digits > 0).then(|| text.to_string())
        }
        "date" => {
            if found != Kind::Date {
                return None;
            }
            if digits > 0 && text.len() > 4 {
                return Some(text.to_string());
            }
            if digits > 0 {
                // A year alone says too little.
                return None;
            }
            // "15 de marzo de 2026": a month with its day before it and
            // its year after it.
            let day = (1..=2)
                .filter_map(|back| at.checked_sub(back).map(|i| &tokens[i]))
                .find(|t| t.raw.text.len() <= 2 && t.raw.text.chars().all(|c| c.is_ascii_digit()) && !t.raw.text.is_empty())?;
            let year = (1..=2)
                .filter_map(|ahead| tokens.get(at + ahead))
                .find(|t| t.raw.text.len() == 4 && t.raw.text.chars().all(|c| c.is_ascii_digit()))?;
            Some(format!("{} de {} de {}", day.raw.text, text.to_lowercase(), year.raw.text))
        }
        _ => None,
    }
}

/// The rest of the line after token `at`, or the next line when nothing
/// is left on it.
fn line_after(tokens: &[Token], at: usize, window: usize) -> Option<String> {
    let line = tokens[at].line;
    let mut rest: Vec<&str> = tokens[at + 1..]
        .iter()
        .take_while(|t| t.line == line)
        .map(|t| t.raw.text.as_str())
        .filter(|t| !t.is_empty())
        .take(window)
        .collect();
    if rest.is_empty() {
        let next_line = tokens.get(at + 1)?.line;
        rest = tokens[at + 1..]
            .iter()
            .take_while(|t| t.line == next_line)
            .map(|t| t.raw.text.as_str())
            .filter(|t| !t.is_empty())
            .take(window)
            .collect();
    }
    let value = rest.join(" ");
    (!value.trim().is_empty()).then_some(value)
}

fn push(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|v| comparable(v) == comparable(&value)) {
        values.push(value);
    }
}

/// The values of one datum found by its rule.
pub fn find(rule: &DataRule, body: &str, marks: &[Mark]) -> Vec<(String, &'static str)> {
    let tokens = tokens(body);
    let window = rule.window.unwrap_or(WINDOW).max(1);
    let mut values: Vec<String> = Vec::new();
    let mut from_marks: Vec<String> = Vec::new();
    if let Some(kind) = &rule.from_mark {
        for mark in marks.iter().filter(|m| &m.kind == kind) {
            if let Some(value) = &mark.value {
                push(&mut from_marks, value.clone());
            }
        }
        // The document's own value is the first in reading order, the one
        // of its form or its heading; one cited in its body comes later and
        // is a reference to another document (P-485).
        let at = |value: &String| tokens.iter().position(|t| comparable(&t.raw.text) == comparable(value)).unwrap_or(usize::MAX);
        from_marks.sort_by_key(at);
    }
    for token in &tokens {
        if rule.shapes.iter().any(|shape| has_shape(&token.raw.text, shape)) {
            push(&mut values, token.raw.text.clone());
        }
    }
    for phrase in &rule.after {
        for end in phrase_ends(&tokens, phrase) {
            if rule.kind == "text" {
                if let Some(value) = line_after(&tokens, end, window) {
                    push(&mut values, value);
                }
                continue;
            }
            let mut at = end + 1;
            let mut limit = (end + 1 + window).min(tokens.len());
            while at < limit {
                if let Some(value) = value_at(&tokens, at, &rule.kind) {
                    push(&mut values, value);
                    if !rule.many {
                        break;
                    }
                    limit = (at + 1 + window).min(tokens.len());
                }
                at += 1;
            }
        }
    }
    let mut out: Vec<(String, &'static str)> = from_marks.into_iter().map(|v| (v, "mark")).collect();
    for value in values {
        if !out.iter().any(|(v, _)| comparable(v) == comparable(&value)) {
            out.push((value, "rule"));
        }
    }
    if rule.pick.as_deref() == Some("largest") && !rule.many {
        if let Some(largest) = out.iter().cloned().max_by(|x, y| amount_value(&x.0).total_cmp(&amount_value(&y.0))) {
            return vec![largest];
        }
    }
    if !rule.many {
        out.truncate(1);
    }
    out
}

/// The number an amount says ("$ 1.234,56" → 1234.56; "1,234.56" too).
pub fn amount_value(text: &str) -> f64 {
    let digits: String = text.chars().filter(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    let decimal = match (digits.rfind(','), digits.rfind('.')) {
        (Some(comma), Some(dot)) => Some(comma.max(dot)),
        (Some(comma), None) => (digits.len() - comma - 1 <= 2).then_some(comma),
        (None, Some(dot)) => (digits.len() - dot - 1 <= 2).then_some(dot),
        (None, None) => None,
    };
    let mut number = String::new();
    for (index, c) in digits.chars().enumerate() {
        if c.is_ascii_digit() {
            number.push(c);
        } else if Some(index) == decimal {
            number.push('.');
        }
    }
    number.parse().unwrap_or(0.0)
}

/// Everything the rules find, and the required data they did not.
pub fn extract(rules: &[&DataRule], body: &str, marks: &[Mark]) -> (Vec<Datum>, Vec<String>) {
    let mut data = Vec::new();
    let mut missing = Vec::new();
    for rule in rules {
        let found = find(rule, body, marks);
        if found.is_empty() && rule.required && !missing.contains(&rule.id) {
            missing.push(rule.id.clone());
        }
        for (value, source) in found {
            data.push(Datum { field: rule.id.clone(), value, source: source.to_string() });
        }
    }
    (data, missing)
}

/// A value as compared between documents: letters and digits only, in
/// capitals ("0003-00001234" and "0003 00001234" are the same number).
pub fn comparable(value: &str) -> String {
    normalized(value).replace(' ', "").to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(kind: &str, after: &[&str]) -> DataRule {
        DataRule {
            id: "x".into(),
            description: "x".into(),
            kind: kind.into(),
            after: after.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    fn values(rule: &DataRule, body: &str) -> Vec<String> {
        find(rule, body, &[]).into_iter().map(|(v, _)| v).collect()
    }

    const INVOICE: &str = "FACTURA B\nN° 0003-00001234\nFecha de emisión: 15/03/2026\nPeríodo facturado: 01/02/2026 al 28/02/2026\nTotal a pagar: $ 145.200,00\nVencimiento 25/03/2026";

    #[test]
    fn a_value_is_found_after_its_phrase() {
        assert_eq!(values(&rule("identifier", &["N°"]), INVOICE), ["0003-00001234"]);
        assert_eq!(values(&rule("date", &["fecha de emision"]), INVOICE), ["15/03/2026"]);
        assert_eq!(values(&rule("amount", &["total"]), INVOICE), ["$ 145.200,00"]);
        assert_eq!(values(&rule("text", &["periodo facturado"]), INVOICE), ["01/02/2026 al 28/02/2026"]);
        assert!(values(&rule("amount", &["descuento"]), INVOICE).is_empty());
    }

    #[test]
    fn the_value_of_a_mark_is_the_documents_own_the_first_in_reading_order() {
        // P-485: the marks come by the catalog's order of signals; the
        // number of the form comes first in the text, the one the body
        // cites later.
        let mark = |value: &str, signal: &str| Mark {
            kind: "shipment-number".into(),
            doc_type: "carta-documento".into(),
            variant: None,
            stage: None,
            signal: signal.into(),
            page: None,
            value: Some(value.into()),
        };
        let body = "CARTA DOCUMENTO\nN° Q12345ZZ678\nHemos recibido su carta identificada como XY123456789AR, la cual rechazamos.";
        let marks = [mark("XY123456789AR", "uno"), mark("Q12345ZZ678", "otro")];
        let rule = DataRule { id: "numero-de-envio".into(), kind: "identifier".into(), from_mark: Some("shipment-number".into()), ..Default::default() };
        let found: Vec<String> = find(&rule, body, &marks).into_iter().map(|(v, _)| v).collect();
        assert_eq!(found, ["Q12345ZZ678"]);
        // Every value, in reading order, when the datum has many.
        let many = DataRule { many: true, ..rule };
        let found: Vec<String> = find(&many, body, &marks).into_iter().map(|(v, _)| v).collect();
        assert_eq!(found, ["Q12345ZZ678", "XY123456789AR"]);
    }

    #[test]
    fn a_date_in_words_is_put_together() {
        let body = "Buenos Aires, 15 de marzo de 2026.\nSeñor gerente";
        assert_eq!(values(&rule("date", &["buenos aires"]), body), ["15 de marzo de 2026"]);
    }

    #[test]
    fn many_values_or_only_the_first() {
        let body = "Rechazo la factura 0003-00001111 y la factura 0003-00002222.";
        let mut many = rule("identifier", &["factura"]);
        assert_eq!(values(&many, body), ["0003-00001111"]);
        many.many = true;
        assert_eq!(values(&many, body), ["0003-00001111", "0003-00002222"]);
    }

    #[test]
    fn shapes_and_marks_give_values_too() {
        let mut by_shape = rule("identifier", &[]);
        by_shape.shapes = vec!["AA999999999AR".into()];
        assert_eq!(values(&by_shape, "Envío XY123456789AR recibido"), ["XY123456789AR"]);

        let mut by_mark = rule("identifier", &[]);
        by_mark.from_mark = Some("shipment-number".into());
        let mark = Mark {
            kind: "shipment-number".into(),
            doc_type: "t".into(),
            variant: None,
            stage: None,
            signal: "s".into(),
            page: None,
            value: Some("XY123456789AR".into()),
        };
        assert_eq!(find(&by_mark, "", &[mark]), [("XY123456789AR".to_string(), "mark")]);
    }

    #[test]
    fn a_list_after_one_phrase_is_read_whole() {
        let body = "Rechazo las facturas A0003-00001111 con fecha 16 de abril, A0003-00002222 con fecha 16 de abril                     y A0003-00003333 con fecha 17 de abril, recibidas por correo.";
        let mut many = rule("identifier", &["facturas"]);
        many.many = true;
        assert_eq!(values(&many, body), ["A0003-00001111", "A0003-00002222", "A0003-00003333"]);
    }

    #[test]
    fn a_total_is_the_largest_amount_next_to_the_word() {
        let body = "Total s/imp.
 3.453,64
 IVA 21 %: 725,26
 TOTAL
Percepciones
 4.178,90";
        let mut total = rule("amount", &["total"]);
        total.window = Some(3);
        assert_eq!(values(&total, body), ["3.453,64"]);
        total.pick = Some("largest".into());
        assert_eq!(values(&total, body), ["4.178,90"]);
        assert_eq!(amount_value("$ 1.234,56"), 1234.56);
        assert_eq!(amount_value("USD 1,234.56"), 1234.56);
        assert_eq!(amount_value("$6800.00"), 6800.0);
        assert_eq!(amount_value("USD 5.506"), 5506.0);
    }

    #[test]
    fn required_data_not_found_are_missing() {
        let mut total = rule("amount", &["total"]);
        total.id = "total".into();
        total.required = true;
        let mut client = rule("text", &["cliente"]);
        client.id = "cliente".into();
        client.required = true;
        let (data, missing) = extract(&[&total, &client], INVOICE, &[]);
        assert_eq!(data.len(), 1);
        assert_eq!(missing, ["cliente"]);
    }

    #[test]
    fn values_are_compared_by_letters_and_digits() {
        assert_eq!(comparable("0003-00001234"), comparable("0003 00001234"));
        assert_ne!(comparable("0003-00001234"), comparable("0003-00001235"));
    }
}
