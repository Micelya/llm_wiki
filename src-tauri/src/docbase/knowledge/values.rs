//! What a value said of an entity must look like to count, and when two
//! ways of writing a value are the same value (D-209).
//!
//! The model sometimes puts in a value what is not one: a document number
//! as a position, a time limit as an amount claimed, an identity
//! number as a domicile. The quote is real (it was verified), the value is
//! not: such a piece of evidence stays in the reading but is not a claim.
//! Each predicate says its shape in the vocabulary (`shape`); the words
//! the shapes are checked with are data too (`values`).
//!
//! The same domicile is often written two ways ("Calle 100" and "Calle
//! 100, piso 3, Ciudad"); the same amount with or without its currency.
//! Those are one claim with all their evidence, shown with the fullest
//! writing: they are not a conflict.

use super::quote;
use super::vocabulary::Vocabulary;

/// The runs of digits of a text, in order.
pub fn digit_runs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        if c.is_ascii_digit() {
            current.push(c);
        } else if !current.is_empty() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

/// The words of a text that have letters.
fn lettered(text: &str) -> Vec<String> {
    quote::words(text).into_iter().filter(|w| w.chars().any(char::is_alphabetic)).collect()
}

/// Whether `value`, said of the entity named `subject` (normalized), has
/// the shape its predicate asks for. A predicate without a shape takes
/// anything that is not empty.
pub fn fits(vocabulary: &Vocabulary, shape: &str, value: &str, subject: &str) -> bool {
    // A line to fill in ("By: ______", "Firma: ........") is not a value
    // of anything (D-288).
    if is_a_blank(value) {
        return false;
    }
    let digits: usize = value.chars().filter(|c| c.is_ascii_digit()).count();
    let letters: usize = value.chars().filter(|c| c.is_alphabetic()).count();
    match shape {
        "amount" => {
            if digits == 0 {
                return false;
            }
            // A number with separators ("5.129,80") is an amount; a bare
            // number needs a currency ("USD 500", "$ 100").
            let words = quote::words(value);
            let currency = value.contains('$')
                || value.contains('€')
                || words.iter().any(|w| vocabulary.values.currencies.iter().any(|c| c == w));
            let separated = value.chars().zip(value.chars().skip(1)).zip(value.chars().skip(2)).any(|((a, b), c)| {
                a.is_ascii_digit() && (b == '.' || b == ',') && c.is_ascii_digit()
            });
            // Without a currency, a separated number with few words
            // around it ("71.008,64"), not a phrase that has one.
            let other_words = lettered(value).iter().filter(|w| !vocabulary.values.currencies.iter().any(|c| c == *w)).count();
            currency || (separated && other_words <= 2)
        }
        "text" => {
            if letters == 0 || digits > letters || value.contains('@') {
                return false;
            }
            // The name of whom it is said of is not a position.
            let own: Vec<String> = subject.split_whitespace().filter(|w| w.chars().count() > 2).map(str::to_string).collect();
            let words = quote::words(value);
            own.is_empty() || !own.iter().all(|w| words.contains(w))
        }
        // A street and a number: a postal code alone ("C1429BNO") or a
        // number with no word is not where someone is (D-247).
        // A timetable ("días hábiles de 9 hs a 18 hs") is when, not where.
        "address" => {
            let words = quote::words(value);
            digits > 0
                && letters >= 3
                && !value.contains('@')
                && words.iter().any(|w| w.chars().count() >= 3 && w.chars().all(char::is_alphabetic))
                && !words.iter().any(|w| vocabulary.values.schedule.iter().any(|s| s == w) || is_a_time_zone(w))
        }
        // A mail address, or a telephone: not a name. A telephone has seven
        // digits or more; a postal code ("CP 326000") or a number written
        // with dots every three digits (a DNI) is not one (D-287).
        "contact" => {
            let trimmed = value.trim();
            let dotted = trimmed.split('.').count() > 1
                && trimmed.split('.').enumerate().all(|(i, p)| {
                    p.chars().all(|c| c.is_ascii_digit()) && if i == 0 { (1..=3).contains(&p.len()) } else { p.len() == 3 }
                });
            // A date or a time ("11/08/2024 20:59:06") is not a telephone.
            value.contains('@') || (digits >= 7 && !dotted && !is_a_date_or_time(trimmed))
        }
        _ => !value.trim().is_empty(),
    }
}

/// A line to fill in: a run of three or more underscores or dots, with at
/// most a label before it.
fn is_a_blank(value: &str) -> bool {
    let mut run = 0;
    let mut longest = 0;
    let mut previous = ' ';
    for c in value.chars() {
        if (c == '_' || c == '.' || c == '…') && (run == 0 || c == previous) {
            run += 1;
        } else if c == '_' || c == '.' || c == '…' {
            run = 1;
        } else {
            run = 0;
        }
        previous = c;
        longest = longest.max(run);
    }
    longest >= 3 && quote::words(value).len() <= 2
}

/// A time zone written as a word with its offset ("gmt-3" reads "gmt").
fn is_a_time_zone(word: &str) -> bool {
    ["gmt", "utc"].iter().any(|z| word.starts_with(z) && word[z.len()..].chars().all(|c| c.is_ascii_digit()))
}

/// Whether a value is written as a date ("11/08/2024", "2024-08-11") or a
/// time ("20:59", "20:59:06"): digits split by slashes, dashes or colons
/// in the shape of one, and nothing else but spaces and those.
fn is_a_date_or_time(value: &str) -> bool {
    let date = |t: &str| {
        let parts: Vec<&str> = t.split(|c| c == '/' || c == '-').collect();
        parts.len() == 3
            && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            && (parts[0].len() == 4 || parts[2].len() == 4 || parts[2].len() == 2)
            && parts.iter().filter(|p| p.len() <= 2).count() >= 2
    };
    let time = |t: &str| {
        let parts: Vec<&str> = t.split(':').collect();
        (2..=3).contains(&parts.len()) && parts.iter().all(|p| (1..=2).contains(&p.len()) && p.chars().all(|c| c.is_ascii_digit()))
    };
    let tokens: Vec<&str> = value.split_whitespace().collect();
    !tokens.is_empty() && tokens.iter().any(|t| date(t) || time(t)) && tokens.iter().all(|t| date(t) || time(t))
}

/// Whether two values of a predicate of this shape are the same value
/// written two ways.
pub fn same(vocabulary: &Vocabulary, shape: &str, a: &str, b: &str) -> bool {
    let (da, db) = (digit_runs(a), digit_runs(b));
    match shape {
        "amount" => !da.is_empty() && da == db,
        "address" => {
            if da.is_empty() || db.is_empty() {
                return false;
            }
            let (short, long) = if da.len() <= db.len() { (&da, &db) } else { (&db, &da) };
            if !short.iter().all(|d| long.contains(d)) {
                return false;
            }
            if short.len() >= 2 {
                return true;
            }
            let noise = |w: &String| vocabulary.values.address_noise.iter().any(|n| n == w) || w.chars().count() < 3;
            let wa: Vec<String> = lettered(a).into_iter().filter(|w| !noise(w)).collect();
            let wb: Vec<String> = lettered(b).into_iter().filter(|w| !noise(w)).collect();
            wa.iter().any(|w| wb.contains(w))
        }
        _ => false,
    }
}

/// Of several ways of writing the same value, the one shown: the fullest
/// (most words), then the first in order.
pub fn fullest<'a>(values: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    values.max_by(|a, b| quote::words(a).len().cmp(&quote::words(b).len()).then_with(|| b.cmp(a)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_counts_only_with_its_shape() {
        let v = Vocabulary::builtin();
        for amount in ["USD 5.129,80", "$ 100.000", "71.008,64", "500 dólares", "CINCO MIL DÓLARES (USD 5.000)"] {
            assert!(fits(&v, "amount", amount, "x"), "{amount}");
        }
        for not in ["72 horas corridas", "lo que surja de la liquidación", "2024", "30-11111111-1", "1,000+ usuarios activos garantizados por día"] {
            assert!(!fits(&v, "amount", not, "x"), "{not}");
        }
        assert!(fits(&v, "text", "Presidente", "persona dos"));
        for not in ["22.222.222", "30-11111111-1", "Atención: Persona Dos", "alguien@ejemplo.com"] {
            assert!(!fits(&v, "text", not, "persona dos"), "{not}");
        }
        assert!(fits(&v, "address", "Calle Uno 100, piso 3", "x"));
        assert!(fits(&v, "contact", "alguien@ejemplo.com", "x") && fits(&v, "contact", "+54 11 4444-5555", "x"));
        assert!(!fits(&v, "contact", "Atención: Persona Dos", "persona dos"));
        assert!(fits(&v, "contact", "0800-000-0000", "x") && fits(&v, "contact", "4444555", "x"));
        for not in ["123456", "22.222.222"] {
            assert!(!fits(&v, "contact", not, "x"), "{not}");
        }
        for not in ["22.222.222", "Islas del Norte", "C1429ABC", "CP 1000"] {
            assert!(!fits(&v, "address", not, "x"), "{not}");
        }
    }

    #[test]
    fn a_blank_a_timetable_and_a_date_are_not_values_of_their_kind() {
        let v = Vocabulary::builtin();
        // D-288: a line to fill in is no position; a timetable is no
        // address; a date with its time is no telephone.
        for blank in ["By: ________________________________", "________", "Firma: ..........."] {
            assert!(!fits(&v, "text", blank, "persona dos"), "{blank}");
        }
        assert!(fits(&v, "text", "Socio gerente", "persona dos"));
        for timetable in ["Días hábiles de 9 hs a 18 hs. (GMT-3)", "Lunes a viernes de 9 a 18 horas"] {
            assert!(!fits(&v, "address", timetable, "x"), "{timetable}");
        }
        assert!(fits(&v, "address", "Calle Uno 100, piso 3", "x"));
        assert!(fits(&v, "address", "Ruta 9 km 1500", "x"));
        for date in ["11/08/2024 20:59:06", "2024-08-11", "20:59:06"] {
            assert!(!fits(&v, "contact", date, "x"), "{date}");
        }
        for telephone in ["+54 11 4444-5555", "011 4444-5555", "4444-5555 int. 22"] {
            assert!(fits(&v, "contact", telephone, "x"), "{telephone}");
        }
    }

    #[test]
    fn the_same_value_written_two_ways() {
        let v = Vocabulary::builtin();
        assert!(same(&v, "address", "Calle Uno 100", "Calle Uno 100, piso 3, Ciudad Autónoma de Buenos Aires"));
        assert!(same(&v, "address", "Av. del Ejemplo 500, CABA", "Ejemplo AV 500 Ciudad Autonoma"));
        assert!(same(&v, "address", "10 Main St Suite 20, Town 300", "10 Main Street, suite 20, Other Town, 300"));
        assert!(!same(&v, "address", "Calle Uno 100, piso 3", "Calle Uno 100, piso 5"));
        assert!(!same(&v, "address", "Calle Uno 100", "Calle Dos 100"));
        assert!(same(&v, "amount", "USD 5.129,80", "5.129,80"));
        assert!(!same(&v, "amount", "USD 5.129,80", "USD 5.129,81"));
        assert_eq!(fullest(["Calle Uno 100", "Calle Uno 100, piso 3"].into_iter()), Some("Calle Uno 100, piso 3"));
    }
}
