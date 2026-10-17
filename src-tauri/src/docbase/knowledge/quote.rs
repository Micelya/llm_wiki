//! The check that anchors the layer to its sources (D-171): a quote the
//! model gave is kept only if it is in the text of the document.
//!
//! Both are compared as the base compares texts everywhere: lowercase,
//! without accents, letters and digits only (`normalized_words`), so a
//! line break, a hyphen or a different quotation mark is not a
//! difference, and a word changed is. The quote must appear as one
//! continuous run of words. The text locates by page only (its `## Page
//! N` headings): that is the page a quote is on.

use std::collections::HashMap;

use crate::commands::extract_images::FORM_FIELDS_HEADING;
use crate::source_volume::catalog::{is_page_heading, normalized_words, RECOGNIZED_MARKER_START};

/// The words of a document, each with the page it is on.
#[derive(Debug, Clone, Default)]
pub struct Located {
    words: Vec<String>,
    pages: Vec<Option<i64>>,
    first: HashMap<String, Vec<usize>>,
}

/// The normalized words of a piece of text.
pub fn words(text: &str) -> Vec<String> {
    normalized_words(text)
}

/// Whether `needle` appears in `haystack` as one continuous run.
pub fn contains(haystack: &[String], needle: &[String]) -> bool {
    !needle.is_empty() && needle.len() <= haystack.len() && haystack.windows(needle.len()).any(|w| w == needle)
}

impl Located {
    pub fn new(body: &str) -> Self {
        let mut located = Located::default();
        let mut page: Option<i64> = None;
        for line in body.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with(RECOGNIZED_MARKER_START) {
                continue;
            }
            if is_page_heading(trimmed) {
                page = trimmed["## Page ".len()..].parse().ok();
                continue;
            }
            if trimmed == FORM_FIELDS_HEADING {
                // The fields of a form are not on a page.
                page = None;
                continue;
            }
            for word in normalized_words(trimmed) {
                located.first.entry(word.clone()).or_default().push(located.words.len());
                located.words.push(word);
                located.pages.push(page);
            }
        }
        located
    }

    /// The pages where `quote` starts, in order; empty when it is not in
    /// the text (or says nothing once normalized).
    pub fn find(&self, quote: &[String]) -> Vec<Option<i64>> {
        let Some(head) = quote.first() else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for &start in self.first.get(head).map(Vec::as_slice).unwrap_or_default() {
            if self.words.get(start..start + quote.len()).is_some_and(|w| w == quote) {
                let page = self.pages[start];
                if !found.contains(&page) {
                    found.push(page);
                }
            }
        }
        found
    }

    /// Where a quote is: `None` when it is not in the text, otherwise the
    /// page (the one the model said, when the quote is there too).
    pub fn locate(&self, quote: &str, said_page: Option<i64>) -> Option<Option<i64>> {
        let found = self.find(&words(quote));
        if found.is_empty() {
            return None;
        }
        if found.contains(&said_page) {
            return Some(said_page);
        }
        Some(found[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "## Page 1\n\nEn la ciudad, a los dos días, comparece el Sr. Ejemplo Uno,\nDNI 11.111.111, quien dice:\n\n## Page 2\n\nQue constituye domicilio en Calle Falsa 123.\n\n### Form fields\n\nNombre: Persona Dos\n";

    #[test]
    fn a_quote_is_found_whatever_its_format_and_on_its_page() {
        let text = Located::new(TEXT);
        assert_eq!(text.locate("comparece el SR. EJEMPLO uno, DNI 11111111", None), None, "11111111 is not 11 111 111");
        assert_eq!(text.locate("comparece el SR. EJEMPLO uno, DNI 11.111.111", None), Some(Some(1)));
        // Across a line break, with other punctuation.
        assert_eq!(text.locate("«Ejemplo Uno» — DNI 11.111.111", Some(1)), Some(Some(1)));
        assert_eq!(text.locate("constituye domicilio en calle falsa 123", Some(1)), Some(Some(2)));
        // A form field is on no page.
        assert_eq!(text.locate("Nombre: Persona Dos", Some(2)), Some(None));
    }

    #[test]
    fn a_changed_word_or_an_empty_quote_is_not_found() {
        let text = Located::new(TEXT);
        assert_eq!(text.locate("constituye domicilio en calle falsa 124", None), None);
        assert_eq!(text.locate("comparece el señor Ejemplo Uno", None), None);
        assert_eq!(text.locate(" — ", None), None);
    }

    #[test]
    fn the_page_the_model_said_wins_when_the_quote_is_on_several() {
        let text = Located::new("## Page 1\nla misma frase\n## Page 2\nla misma frase\n");
        assert_eq!(text.locate("la misma frase", Some(2)), Some(Some(2)));
        assert_eq!(text.locate("la misma frase", Some(7)), Some(Some(1)));
        assert_eq!(text.locate("la misma frase", None), Some(Some(1)));
    }

    #[test]
    fn words_inside_words_are_compared_as_runs() {
        let quote = words("Ejemplo Uno, DNI 11.111.111");
        assert!(contains(&quote, &words("ejemplo uno")));
        assert!(!contains(&quote, &words("uno ejemplo")));
        assert!(!contains(&quote, &[]));
    }
}
