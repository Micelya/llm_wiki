//! The words of a text as the base compares documents: two files are the
//! same document when their normalized words match exactly, whatever the
//! format, the line breaks, the page headings or the recognition markers.
//! A fingerprint of those words identifies the document.
//!
//! It used to also keep a catalog of a wiki project's sources; that left
//! with the wiki (W6).

use sha2::{Digest, Sha256};

/// Below this many words a text says too little to call two files the
/// same document (an unread scan, a cover page, a photo).
const MIN_FINGERPRINT_WORDS: usize = 30;
/// Start of the marker `recognition.rs` writes before machine-read text.
pub(crate) const RECOGNIZED_MARKER_START: &str = "<!-- texto-reconocido";

/// Fingerprint of a source text, or `None` when the text is too short to
/// identify a document.
pub fn fingerprint(text: &str) -> Option<(String, usize)> {
    let words = normalized_words(text);
    if words.len() < MIN_FINGERPRINT_WORDS {
        return None;
    }
    let mut hasher = Sha256::new();
    hasher.update(words.join(" ").as_bytes());
    let id = hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Some((id, words.len()))
}

/// Words of `text` as compared between files: lowercase, without
/// accents, letters and digits only. Formatting that depends on the file
/// format — punctuation, line breaks, page headings, recognition
/// markers — does not take part.
pub(crate) fn normalized_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(RECOGNIZED_MARKER_START) || is_page_heading(trimmed) {
            continue;
        }
        let mut word = String::new();
        for c in trimmed.chars().flat_map(char::to_lowercase) {
            match unaccented(c) {
                Some(c) if c.is_ascii_alphanumeric() => word.push(c),
                _ => {
                    if !word.is_empty() {
                        words.push(std::mem::take(&mut word));
                    }
                }
            }
        }
        if !word.is_empty() {
            words.push(word);
        }
    }
    words
}

/// The `## Page N` headings the PDF extractor writes.
pub(crate) fn is_page_heading(line: &str) -> bool {
    line.strip_prefix("## Page ")
        .is_some_and(|rest| !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()))
}

/// `c` without its accent, for the accented letters of Western European
/// languages. Characters outside ASCII that are not listed are dropped
/// by the caller.
pub(crate) fn unaccented(c: char) -> Option<char> {
    Some(match c {
        'á' | 'à' | 'ä' | 'â' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i',
        'ó' | 'ò' | 'ö' | 'ô' | 'õ' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u',
        'ñ' => 'n',
        'ç' => 'c',
        c if c.is_ascii() => c,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGREEMENT: &str = "Acuerdo de colaboración entre las partes. La primera parte se obliga a \
        prestar los servicios descriptos en el anexo, y la segunda parte se obliga a pagar el precio \
        convenido dentro de los treinta días de recibida cada factura, en la cuenta que se indique.";

    #[test]
    fn the_same_text_in_another_format_has_the_same_fingerprint() {
        let as_pdf = format!("## Page 1\n{}\n", AGREEMENT.replace(". ", ".\n"));
        let as_docx = AGREEMENT.to_uppercase().replace("COLABORACIÓN", "colaboracion");
        assert_eq!(fingerprint(AGREEMENT), fingerprint(&as_pdf));
        assert_eq!(fingerprint(AGREEMENT), fingerprint(&as_docx));
    }

    #[test]
    fn a_different_figure_is_a_different_document() {
        let other = AGREEMENT.replace("treinta", "sesenta");
        assert_ne!(fingerprint(AGREEMENT), fingerprint(&other));
        let amount_a = format!("{AGREEMENT} Importe total: USD 4.114,80.");
        let amount_b = format!("{AGREEMENT} Importe total: USD 4.114,90.");
        assert_ne!(fingerprint(&amount_a), fingerprint(&amount_b));
    }

    #[test]
    fn short_texts_are_not_fingerprinted() {
        assert_eq!(fingerprint("Factura A 00121-00030080"), None);
        assert_eq!(fingerprint("Solo una portada"), None);
    }

    #[test]
    fn recognition_markers_do_not_change_the_fingerprint() {
        let recognized = format!("<!-- texto-reconocido motor=\"codex-cli/gpt\" -->\n{AGREEMENT}");
        assert_eq!(fingerprint(AGREEMENT), fingerprint(&recognized));
    }
}
