//! What a line looks like as words (copies of `comun.py` and
//! `estructura.py` of the scripts of N): the share of doubtful words, with
//! initials and acronyms counted as words (V1, D-680), the lexicon of
//! Spanish and the text two readers can agree on.

use std::collections::HashSet;
use std::sync::OnceLock;

use regex::Regex;
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

/// A general list of 50 thousand Spanish words by frequency (`es_50k`,
/// FrequencyWords, from the subtitles of OpenSubtitles; CC BY-SA 4.0), as
/// the scripts read it: the first word of each line.
const LEXICON: &str = include_str!("../../../resources/catalog/lexicon-es-50k.txt");
const VOWELS: &str = "aeiouáéíóúüy";

/// A letter as Python's `[^\W\d_]` sees it: a word character that is
/// not a decimal digit nor the underscore.
pub fn is_letter(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_alphabetic();
    }
    c.is_alphanumeric() && !is_decimal(c)
}

/// A decimal digit of any script (`\d`).
fn is_decimal(c: char) -> bool {
    if c.is_ascii() {
        return c.is_ascii_digit();
    }
    static DIGIT: OnceLock<Regex> = OnceLock::new();
    let mut buf = [0u8; 4];
    DIGIT.get_or_init(|| Regex::new(r"^\d$").unwrap()).is_match(c.encode_utf8(&mut buf))
}

/// The runs of letters of a text (`re.findall(r"[^\W\d_]+", text)`).
pub fn letter_runs(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        match (is_letter(c), start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                out.push(&text[s..i]);
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        out.push(&text[s..]);
    }
    out
}

fn len(w: &str) -> usize {
    w.chars().count()
}

fn has_vowel(w: &str) -> bool {
    w.to_lowercase().chars().any(|c| VOWELS.contains(c))
}

/// How much a text does not look like words (`basura`): the share of
/// words of 15 letters or more, of 3 or more without a vowel and of one
/// letter; and how many words. Without words, 1 and 0.
pub fn junk(text: &str) -> (f64, usize) {
    let toks = letter_runs(text);
    if toks.is_empty() {
        return (1.0, 0);
    }
    let bad = toks.iter().filter(|w| len(w) >= 15).count()
        + toks.iter().filter(|w| len(w) >= 3 && !has_vowel(w)).count()
        + toks.iter().filter(|w| len(w) == 1).count();
    (bad as f64 / toks.len() as f64, toks.len())
}

/// The initials with dots of a text (`S.A.`, `C.U.I.T.`), as `RE_INI` of
/// the scripts finds them: where each starts and ends.
fn initials(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let letter = |k: usize| chars.get(k).is_some_and(|(_, c)| is_letter(*c));
    let dot = |k: usize| chars.get(k).is_some_and(|(_, c)| *c == '.');
    let byte = |k: usize| chars.get(k).map_or(text.len(), |(b, _)| *b);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if i > 0 && letter(i - 1) {
            i += 1;
            continue;
        }
        // As many «letter and dot» as there are, then back off until a
        // letter, an optional dot and no letter after.
        let mut reps = 0;
        while letter(i + 2 * reps) && dot(i + 2 * reps + 1) {
            reps += 1;
        }
        let mut found = None;
        for r in (1..=reps).rev() {
            let p = i + 2 * r;
            if !letter(p) {
                continue;
            }
            if dot(p + 1) && !letter(p + 2) {
                found = Some(p + 2);
                break;
            }
            if !letter(p + 1) {
                found = Some(p + 1);
                break;
            }
        }
        match found {
            Some(end) => {
                out.push((byte(i), byte(end)));
                i = end;
            }
            None => i += 1,
        }
    }
    out
}

/// Like `junk`, but initials with dots are one good word, and so is an
/// acronym in capitals of 2 to 5 letters (`basura_v1`).
pub fn junk_v1(text: &str) -> (f64, usize) {
    let found = initials(text);
    let mut rest = String::with_capacity(text.len());
    let mut at = 0;
    for (s, e) in &found {
        rest.push_str(&text[at..*s]);
        rest.push(' ');
        at = *e;
    }
    rest.push_str(&text[at..]);
    let toks = letter_runs(&rest);
    let n = found.len() + toks.len();
    if n == 0 {
        return (1.0, 0);
    }
    let acronym = |w: &str| (2..=5).contains(&len(w)) && w.chars().any(char::is_uppercase) && !w.chars().any(char::is_lowercase);
    let bad = toks.iter().filter(|w| len(w) >= 15).count()
        + toks.iter().filter(|w| len(w) >= 3 && !has_vowel(w) && !acronym(w)).count()
        + toks.iter().filter(|w| len(w) == 1).count();
    (bad as f64 / n as f64, n)
}

/// Without accents (`sin_tildes`): decomposed, the combining marks out.
pub fn without_accents(s: &str) -> String {
    s.nfkd().filter(|c| canonical_combining_class(*c) == 0).collect()
}

/// Only the letters a to z and the digits, lower case and without accents
/// (`_n`): how two readings of the same text are compared.
pub fn bare(s: &str) -> String {
    without_accents(s).to_lowercase().chars().filter(|c| c.is_ascii_lowercase() || c.is_ascii_digit()).collect()
}

fn lexicon() -> &'static HashSet<String> {
    static WORDS: OnceLock<HashSet<String>> = OnceLock::new();
    WORDS.get_or_init(|| {
        LEXICON
            .lines()
            .filter_map(|l| l.split_whitespace().next())
            .map(|w| without_accents(w).to_lowercase())
            .collect()
    })
}

/// Without digits, with at least 2 words of 3 letters or more and all of
/// them in the lexicon (`todo_palabras`).
pub fn all_words(text: &str) -> bool {
    if text.chars().any(is_decimal) {
        return false;
    }
    let words: Vec<String> = letter_runs(text).into_iter().filter(|w| len(w) >= 3).map(|w| without_accents(w).to_lowercase()).collect();
    words.len() >= 2 && words.iter().all(|w| lexicon().contains(w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_look_like_words_and_initials_count_as_one() {
        assert_eq!(letter_runs("Hola, señor 2º_x"), vec!["Hola", "señor", "º", "x"]);
        assert_eq!(junk("xkcd qwrt a b"), (1.0, 4));
        assert_eq!(junk("123"), (1.0, 0));
        // S.A. and C.U.I.T. are one word each; CUIT, an acronym, is good.
        assert_eq!(initials("Empresa S.A. y C.U.I.T. 20"), vec![(8, 12), (15, 23)]);
        assert_eq!(initials("S.A.Bc"), vec![(0, 3)]);
        assert!(initials("ab.c.").is_empty());
        let (bad, n) = junk_v1("Empresa S.A. CUIT");
        assert_eq!((bad, n), (0.0, 3));
        let (bad, _) = junk("Empresa S.A. CUIT");
        assert!(bad > 0.0);
    }

    #[test]
    fn the_lexicon_and_the_bare_text() {
        assert!(all_words("Señores de la reunión"));
        assert!(!all_words("Señores de la reunión 2"));
        assert!(!all_words("xqzt wvbk"));
        assert!(!all_words("casa"));
        assert_eq!(bare("Calle Ñandú 123, piso 4º"), "callenandu123piso4o");
        assert_eq!(without_accents("acción"), "accion");
    }
}
