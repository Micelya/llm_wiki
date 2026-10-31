//! The check that anchors the layer to its sources (D-171): a quote the
//! model gave is kept only if it is in the text of the document.
//!
//! Both are compared as the base compares texts everywhere: lowercase,
//! without accents, letters and digits only (`normalized_words`), so a
//! line break, a hyphen or a different quotation mark is not a
//! difference, and a word changed is. The quote must appear as one
//! continuous run of words. The text locates by page only (its `## Page
//! N` headings): that is the page a quote is on.
//!
//! A quote that is not in the text is looked for once more without the
//! legal form that closes a name, on either side (P-480): "Ejemplo S.A.,
//! con domicilio" is the text "Ejemplo, con domicilio", and the other way
//! around; "Ejemplo SA" is "Ejemplo S.A.". Two different forms are two
//! organizations, and are not. A quote that is only a web address is not
//! looked for so.

use std::collections::HashMap;

use super::vocabulary::is_web;
use crate::commands::extract_images::FORM_FIELDS_HEADING;
use crate::source_volume::catalog::{is_page_heading, normalized_words, RECOGNIZED_MARKER_START};

/// The words of a document, each with the page it is on.
#[derive(Debug, Clone, Default)]
pub struct Located {
    words: Vec<String>,
    pages: Vec<Option<i64>>,
    first: HashMap<String, Vec<usize>>,
    /// The text with its legal forms told apart (`with_forms`).
    forms: Option<Box<FormText>>,
}

/// A word of a text, or a legal form (its place in the list of forms).
#[derive(Debug, Clone, PartialEq)]
enum Token {
    Word(String),
    Form(usize),
}

/// The text as words and legal forms, each with the position of its first
/// word in the text, and the ways of writing each form (the longest first).
#[derive(Debug, Clone, Default)]
struct FormText {
    forms: Vec<(Vec<String>, usize)>,
    tokens: Vec<(Token, usize)>,
    first: HashMap<String, Vec<usize>>,
}

/// The words as tokens: each way of writing a legal form, the longest
/// first, is one token of its form.
fn tokenize(words: &[String], forms: &[(Vec<String>, usize)]) -> Vec<(Token, usize)> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < words.len() {
        match forms.iter().find(|(variant, _)| words[at..].starts_with(variant)) {
            Some((variant, form)) => {
                out.push((Token::Form(*form), at));
                at += variant.len();
            }
            None => {
                out.push((Token::Word(words[at].clone()), at));
                at += 1;
            }
        }
    }
    out
}

/// Whether `quote` is the text from token `start` on, but for a legal form
/// that one of them has and the other not. Two different forms in the
/// same place are not the same.
fn same_but_forms(quote: &[Token], text: &[(Token, usize)], start: usize) -> bool {
    let (mut q, mut t) = (0, start);
    while q < quote.len() {
        let Some((token, _)) = text.get(t) else {
            // The text ends: only forms may be left of the quote.
            return quote[q..].iter().all(|x| matches!(x, Token::Form(_)));
        };
        match (&quote[q], token) {
            (a, b) if a == b => {
                q += 1;
                t += 1;
            }
            (Token::Form(_), Token::Form(_)) => return false,
            (_, Token::Form(_)) => t += 1,
            (Token::Form(_), _) => q += 1,
            _ => return false,
        }
    }
    true
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

    /// The same text, where a quote that is not in it is looked for once
    /// more without the legal forms that close a name (P-480): `forms` are
    /// each way of writing one (its normalized words) with the form it is.
    pub fn with_forms(mut self, forms: &[(String, Vec<String>)]) -> Self {
        let mut ids: Vec<&str> = Vec::new();
        let mut variants: Vec<(Vec<String>, usize)> = Vec::new();
        for (id, words) in forms.iter().filter(|(_, words)| !words.is_empty()) {
            let index = match ids.iter().position(|i| *i == id.as_str()) {
                Some(index) => index,
                None => {
                    ids.push(id);
                    ids.len() - 1
                }
            };
            variants.push((words.clone(), index));
        }
        if variants.is_empty() {
            return self;
        }
        variants.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
        let tokens = tokenize(&self.words, &variants);
        let mut first: HashMap<String, Vec<usize>> = HashMap::new();
        for (index, (token, _)) in tokens.iter().enumerate() {
            if let Token::Word(word) = token {
                first.entry(word.clone()).or_default().push(index);
            }
        }
        self.forms = Some(Box::new(FormText { forms: variants, tokens, first }));
        self
    }

    /// The pages where `quote` starts when only a legal form tells it from
    /// the text (`with_forms`); empty otherwise.
    fn find_but_forms(&self, quote_text: &str, quote: &[String]) -> Vec<Option<i64>> {
        let Some(text) = self.forms.as_deref() else {
            return Vec::new();
        };
        // A web address is where to find someone, not a name.
        if is_web(&quote_text.trim().to_lowercase()) {
            return Vec::new();
        }
        let tokens: Vec<Token> = tokenize(quote, &text.forms).into_iter().map(|(token, _)| token).collect();
        // A quote that starts with the end of a name ("S.A., con
        // domicilio") starts at its first word.
        let Some(head) = tokens.iter().position(|t| matches!(t, Token::Word(_))) else {
            return Vec::new();
        };
        let Token::Word(word) = &tokens[head] else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for &start in text.first.get(word).map(Vec::as_slice).unwrap_or_default() {
            if same_but_forms(&tokens[head..], &text.tokens, start) {
                let page = self.pages[text.tokens[start].1];
                if !found.contains(&page) {
                    found.push(page);
                }
            }
        }
        found
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

    /// The words of the text, in order (what `starts` counts positions in).
    pub fn words(&self) -> &[String] {
        &self.words
    }

    /// The page of the word at `position`.
    pub fn page_at(&self, position: usize) -> Option<i64> {
        self.pages.get(position).copied().flatten()
    }

    /// The places (word positions) where `quote` starts.
    pub fn starts(&self, quote: &[String]) -> Vec<usize> {
        let Some(head) = quote.first() else {
            return Vec::new();
        };
        self.first
            .get(head)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .copied()
            .filter(|&start| self.words.get(start..start + quote.len()).is_some_and(|w| w == quote))
            .collect()
    }

    /// Where a quote is: `None` when it is not in the text, otherwise the
    /// page (the one the model said, when the quote is there too).
    pub fn locate(&self, quote: &str, said_page: Option<i64>) -> Option<Option<i64>> {
        let quote_words = words(quote);
        let mut found = self.find(&quote_words);
        if found.is_empty() {
            found = self.find_but_forms(quote, &quote_words);
        }
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

    fn forms() -> Vec<(String, Vec<String>)> {
        crate::docbase::knowledge::vocabulary::Vocabulary::builtin().closing_legal_forms()
    }

    const COMPANY: &str = "## Page 1\n\nEntre Ejemplo Dos S.A., con domicilio en Calle Falsa 123, y Ejemplo Tres, con domicilio en Calle Uno 1.\n\n## Page 2\n\nVisite www.ejemplo.com.ar para más datos.\n";

    #[test]
    fn a_quote_with_or_without_the_closing_legal_form_is_found() {
        let text = Located::new(COMPANY).with_forms(&forms());
        // The text has the form and the quote not.
        assert_eq!(text.locate("Ejemplo Dos, con domicilio en Calle Falsa 123", None), Some(Some(1)));
        // The other way around.
        assert_eq!(text.locate("Ejemplo Tres S.R.L., con domicilio en Calle Uno 1", Some(1)), Some(Some(1)));
        assert_eq!(text.locate("y Ejemplo Tres SA", None), Some(Some(1)));
        // The same form written otherwise.
        assert_eq!(text.locate("Entre Ejemplo Dos SA, con domicilio", None), Some(Some(1)));
        assert_eq!(text.locate("Ejemplo Dos Sociedad Anónima", None), Some(Some(1)));
        // A quote that starts with the end of the name.
        assert_eq!(text.locate("S.A., con domicilio en Calle Falsa 123", None), Some(Some(1)));
    }

    #[test]
    fn another_form_another_word_or_a_web_address_is_still_not_found() {
        let text = Located::new(COMPANY).with_forms(&forms());
        // Two forms are two organizations.
        assert_eq!(text.locate("Ejemplo Dos S.R.L., con domicilio", None), None);
        // A word changed is not a form.
        assert_eq!(text.locate("Ejemplo Dos S.A., con domicilio en Calle Falsa 124", None), None);
        assert_eq!(text.locate("Ejemplo Cuatro S.A.", None), None);
        // A quote that is only a web address is thrown away, even when only
        // a form would tell it from the text.
        assert_eq!(text.locate("www.ejemplo.sa.com.ar", None), None);
        assert_eq!(text.locate("www.ejemplo.com.ar", None), Some(Some(2)));
        // Without the forms, the text is compared word by word, as before.
        let plain = Located::new(COMPANY);
        assert_eq!(plain.locate("Ejemplo Dos, con domicilio en Calle Falsa 123", None), None);
    }

    #[test]
    fn words_inside_words_are_compared_as_runs() {
        let quote = words("Ejemplo Uno, DNI 11.111.111");
        assert!(contains(&quote, &words("ejemplo uno")));
        assert!(!contains(&quote, &words("uno ejemplo")));
        assert!(!contains(&quote, &[]));
    }
}
