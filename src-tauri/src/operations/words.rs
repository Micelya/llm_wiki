//! The words of the profession the contract is written with (D-156): the
//! chat says «caso» and «ficha» where a law firm does. They are the words
//! of the screens (`src/vocabulary/legal.ts`), copied here.
//!
//! They are kept apart from the knowledge vocabulary (`knowledge.json`) on
//! purpose: that file's fingerprint marks every resolution and every page
//! to be computed again when it changes, and a word of the chat changes
//! nothing of what the base computed. A file of the same shape in the data
//! folder (`docbase/operations-words.json`) replaces them.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Words {
    /// A matter: «caso».
    pub caso: String,
    pub casos: String,
    /// The record of a person or an organization: «ficha».
    pub ficha: String,
    pub fichas: String,
}

impl Default for Words {
    fn default() -> Self {
        Self { caso: "caso".into(), casos: "casos".into(), ficha: "ficha".into(), fichas: "fichas".into() }
    }
}

static INSTALLED: OnceLock<Words> = OnceLock::new();

/// The words in use: the ones installed at start, or the built-in ones.
pub fn words() -> &'static Words {
    INSTALLED.get_or_init(Words::default)
}

/// Uses the words of `path` instead of the built-in ones, when the file
/// exists and is valid. Only the first call counts.
pub fn install_from(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    match serde_json::from_str::<Words>(&text) {
        Ok(words) if [&words.caso, &words.casos, &words.ficha, &words.fichas].iter().all(|w| !w.trim().is_empty()) => {
            let _ = INSTALLED.set(words);
        }
        _ => eprintln!("[operations] {}: not valid words; using the built-in ones", path.display()),
    }
}

impl Words {
    /// A text of the contract with its words: `{caso}`, `{casos}`,
    /// `{ficha}`, `{fichas}`, and the same capitalized (`{Caso}`).
    pub fn fill(&self, text: &str) -> String {
        let mut out = text.to_string();
        for (key, word) in [("caso", &self.caso), ("casos", &self.casos), ("ficha", &self.ficha), ("fichas", &self.fichas)] {
            out = out.replace(&format!("{{{key}}}"), word);
            out = out.replace(&format!("{{{}}}", capitalized(key)), &capitalized(word));
        }
        out
    }
}

pub fn capitalized(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_words_fill_a_text_of_the_contract() {
        let words = Words::default();
        assert_eq!(words.fill("Pone documentos en un {caso}. {Casos} y {fichas}."), "Pone documentos en un caso. Casos y fichas.");
        let other = Words { caso: "asunto".into(), casos: "asuntos".into(), ficha: "ficha".into(), fichas: "fichas".into() };
        assert_eq!(other.fill("El {caso}"), "El asunto");
    }
}
