//! What of a document's text takes part in the comparison, in the two
//! forms the comparison needs.
//!
//! - **Words**, normalized exactly as the text fingerprint does
//!   (`source_volume::catalog`): lowercase, without accents, letters and
//!   digits only. The signature and the alignment work on them, so a
//!   change of format is never a difference.
//! - **The original tokens** each word came from (whitespace-separated,
//!   punctuation trimmed at the ends), so a difference can be told apart
//!   as an amount, a date, an identifier, a name or wording.
//!
//! Left out: the `## Page N` headings, the `### Form fields` list (the
//! fields are compared on their own), recognition markers, and whatever
//! the catalog says is not content (a signing service's history page, a
//! stamp in the margin).

use crate::commands::extract_images::FORM_FIELDS_HEADING;
use crate::source_volume::catalog::{is_page_heading, unaccented, RECOGNIZED_MARKER_START};

/// Words per shingle.
pub const SHINGLE_WORDS: usize = 5;

/// FNV-1a, 64 bits: stable across runs and machines, which a stored
/// signature needs.
pub fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// The lines of each page that are text of the document. A text with no
/// page headings (a Word file) is a single page.
pub fn pages(text: &str) -> Vec<Vec<&str>> {
    let mut pages: Vec<Vec<&str>> = vec![Vec::new()];
    let mut in_fields = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if is_page_heading(trimmed) {
            if pages.last().is_some_and(|page| !page.is_empty()) {
                pages.push(Vec::new());
            }
            in_fields = false;
            continue;
        }
        if trimmed == FORM_FIELDS_HEADING {
            in_fields = true;
            continue;
        }
        if in_fields || trimmed.starts_with(RECOGNIZED_MARKER_START) {
            continue;
        }
        pages.last_mut().expect("there is always a page").push(line);
    }
    pages
}

pub fn is_recognized(text: &str) -> bool {
    text.contains(RECOGNIZED_MARKER_START)
}

/// The normalized words of a piece of text, joined by single spaces.
pub fn normalized(text: &str) -> String {
    let mut out = String::new();
    each_word(text, |word, _| {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(word);
    });
    out
}

/// `text` normalized and padded with spaces, so that looking for
/// `" phrase "` inside it finds whole words only.
pub fn padded(text: &str) -> String {
    format!(" {} ", normalized(text))
}

/// Calls `found` with each normalized word of `text` and the index of the
/// whitespace-separated token it came from.
fn each_word(text: &str, mut found: impl FnMut(&str, usize)) {
    let mut word = String::new();
    for (index, raw) in text.split_whitespace().enumerate() {
        for c in raw.chars().flat_map(char::to_lowercase) {
            match unaccented(c) {
                Some(c) if c.is_ascii_alphanumeric() => word.push(c),
                _ => {
                    if !word.is_empty() {
                        found(&word, index);
                        word.clear();
                    }
                }
            }
        }
        if !word.is_empty() {
            found(&word, index);
            word.clear();
        }
    }
}

/// A normalized word and the original token it came from.
#[derive(Debug, Clone)]
pub struct Word {
    pub key: String,
    pub raw: usize,
}

/// A whitespace-separated token of the original text.
#[derive(Debug, Clone)]
pub struct Raw {
    /// The token with the punctuation around it removed ("$" and "%" stay).
    pub text: String,
    /// It opens a sentence or a paragraph: a capital letter there says
    /// nothing about a name.
    pub sentence_start: bool,
}

/// The text of a document as compared.
#[derive(Debug, Clone, Default)]
pub struct Prepared {
    pub words: Vec<Word>,
    pub raws: Vec<Raw>,
}

impl Prepared {
    /// Builds it from the pages of a text, without the pages in `skip`
    /// and without the lines that hold one of `drop` (normalized, padded
    /// phrases).
    pub fn new(pages: &[Vec<&str>], skip: &[usize], drop: &[String]) -> Self {
        let mut prepared = Prepared::default();
        let mut next_starts = true;
        for (index, page) in pages.iter().enumerate() {
            if skip.contains(&index) {
                continue;
            }
            for line in page {
                if line.trim().is_empty() {
                    next_starts = true;
                    continue;
                }
                if !drop.is_empty() {
                    let line_text = padded(line);
                    if drop.iter().any(|phrase| line_text.contains(phrase.as_str())) {
                        continue;
                    }
                }
                for raw in line.split_whitespace() {
                    let raw_index = prepared.raws.len();
                    prepared.raws.push(Raw {
                        text: trim_token(raw).to_string(),
                        sentence_start: next_starts,
                    });
                    each_word(raw, |word, _| {
                        prepared.words.push(Word {
                            key: word.to_string(),
                            raw: raw_index,
                        })
                    });
                    next_starts = ends_sentence(raw);
                }
            }
            // A new page usually starts a new block of text.
            next_starts = true;
        }
        prepared
    }

    /// The distinct shingles (sequences of `SHINGLE_WORDS` words), hashed
    /// and sorted.
    pub fn shingles(&self) -> Vec<u64> {
        shingles_of(&self.words)
    }

    pub fn word_count(&self) -> usize {
        self.words.len()
    }
}

pub fn shingles_of(words: &[Word]) -> Vec<u64> {
    if words.len() < SHINGLE_WORDS {
        return Vec::new();
    }
    let mut out: Vec<u64> = words
        .windows(SHINGLE_WORDS)
        .map(|window| shingle_hash(window.iter().map(|w| w.key.as_str())))
        .collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// The hash of one shingle; for word `i`, the shingles that cover it are
/// the windows starting at `i - 4 ..= i`.
pub fn shingle_hash<'a>(words: impl Iterator<Item = &'a str>) -> u64 {
    let mut joined = String::new();
    for word in words {
        if !joined.is_empty() {
            joined.push(' ');
        }
        joined.push_str(word);
    }
    fnv1a(joined.as_bytes())
}

fn trim_token(raw: &str) -> &str {
    raw.trim_matches(|c: char| !(c.is_alphanumeric() || c == '$' || c == '%'))
}

fn ends_sentence(raw: &str) -> bool {
    let end = raw.trim_end_matches(|c: char| matches!(c, '"' | '\'' | ')' | '»' | '”' | '’'));
    end.ends_with('.') || end.ends_with('!') || end.ends_with('?')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_volume::catalog::normalized_words;

    const TEXT: &str = "## Page 1\n\nLa Señora PÉREZ, D.N.I. 12.345.678, declara:\n\n## Page 2\n\nque recibió $ 1.234,56 (pesos).\n\n### Form fields\n\n- Nombre: Ana\n";

    #[test]
    fn words_are_the_same_as_the_fingerprints() {
        let prepared = Prepared::new(&pages(TEXT), &[], &[]);
        let keys: Vec<String> = prepared.words.iter().map(|w| w.key.clone()).collect();
        // The fingerprint keeps the form fields; the comparison does not.
        let fingerprint_words = normalized_words(TEXT);
        assert_eq!(keys, fingerprint_words[..keys.len()].to_vec());
        assert!(!keys.contains(&"ana".to_string()));
        assert_eq!(keys[0], "la");
        assert_eq!(keys[2], "perez");
    }

    #[test]
    fn tokens_keep_their_original_form() {
        let prepared = Prepared::new(&pages(TEXT), &[], &[]);
        let raws: Vec<&str> = prepared.raws.iter().map(|r| r.text.as_str()).collect();
        assert!(raws.contains(&"12.345.678"));
        assert!(raws.contains(&"1.234,56"));
        assert!(raws.contains(&"$"));
        assert!(prepared.raws[0].sentence_start);
        assert!(!prepared.raws[1].sentence_start);
        // "declara:" does not end a sentence, the page does.
        let que = prepared.raws.iter().position(|r| r.text == "que").unwrap();
        assert!(prepared.raws[que].sentence_start);
    }

    #[test]
    fn excluded_pages_and_dropped_lines_are_not_compared() {
        let text = "## Page 1\n\nEnvelope Id: 1234-ABCD\nTexto del contrato\n\n## Page 2\n\nHistorial de firma";
        let prepared = Prepared::new(&pages(text), &[1], &[padded("Envelope Id")]);
        let keys: Vec<&str> = prepared.words.iter().map(|w| w.key.as_str()).collect();
        assert_eq!(keys, ["texto", "del", "contrato"]);
    }

    #[test]
    fn shingles_are_sets_of_five_words() {
        let prepared = Prepared::new(&pages("uno dos tres cuatro cinco seis uno dos tres cuatro cinco"), &[], &[]);
        // 7 windows, one of them repeated.
        assert_eq!(prepared.shingles().len(), 6);
        assert!(Prepared::new(&pages("uno dos"), &[], &[]).shingles().is_empty());
    }
}
