//! Spanish support for the query tokenizer in `search.rs`.
//!
//! The tokenizer only knew Chinese and English noise words, so a Spanish
//! question kept words such as "por", "qué" or "de". Tokens are matched as
//! substrings, so those words hit almost every page and pushed the pages
//! that actually answer the question out of the top results.
//!
//! `search.rs` asks this module two things: whether a character separates
//! words, and whether a (lowercased) token is noise.

/// Punctuation that opens or closes a Spanish sentence or quotation.
pub fn is_separator(c: char) -> bool {
    matches!(c, '¿' | '¡' | '«' | '»')
}

/// Articles, prepositions, pronouns, question words and auxiliary verbs.
/// Question words are listed with and without accent because users type
/// both. Single letters ("y", "a", "o") are already dropped by length.
pub fn is_stop_word(token: &str) -> bool {
    matches!(
        token,
        "el" | "la" | "los" | "las" | "lo" | "un" | "una" | "unos" | "unas"
            | "de" | "del" | "al" | "en" | "por" | "para" | "con" | "sin"
            | "sobre" | "entre" | "desde" | "hasta"
            | "que" | "qué" | "cual" | "cuál" | "cuales" | "cuáles"
            | "quien" | "quién" | "quienes" | "quiénes"
            | "como" | "cómo" | "cuando" | "cuándo" | "donde" | "dónde"
            | "cuanto" | "cuánto" | "cuanta" | "cuánta"
            | "cuantos" | "cuántos" | "cuantas" | "cuántas"
            | "es" | "son" | "fue" | "fueron" | "era" | "eran" | "ser"
            | "está" | "están" | "hay" | "ha" | "han"
            | "este" | "esta" | "estos" | "estas" | "ese" | "esa" | "esos" | "esas"
            | "se" | "su" | "sus" | "le" | "les" | "me" | "mi" | "nos"
            | "ya" | "no" | "si" | "sí" | "más" | "pero" | "porque" | "también" | "muy"
    )
}

#[cfg(test)]
mod tests {
    use crate::commands::search::tokenize_query;

    #[test]
    fn spanish_question_keeps_only_the_meaningful_words() {
        let tokens = tokenize_query("¿Qué facturas reclama BGH y por qué monto?");
        assert_eq!(tokens, vec!["bgh", "facturas", "monto", "reclama"]);
    }

    #[test]
    fn accented_and_unaccented_question_words_are_both_dropped() {
        assert_eq!(tokenize_query("cuando vence el contrato"), vec!["contrato", "vence"]);
        assert_eq!(tokenize_query("¿Cuándo vence el contrato?"), vec!["contrato", "vence"]);
    }

    #[test]
    fn opening_marks_and_guillemets_do_not_stick_to_words() {
        assert_eq!(tokenize_query("¡«Extrimian»!"), vec!["extrimian"]);
    }

    #[test]
    fn words_that_merely_contain_a_stop_word_are_kept() {
        let tokens = tokenize_query("persona delegada");
        assert_eq!(tokens, vec!["delegada", "persona"]);
    }

    #[test]
    fn english_and_chinese_queries_are_unchanged() {
        assert_eq!(tokenize_query("what is the invoice total"), vec!["invoice", "total"]);
        assert!(tokenize_query("默会知识").contains(&"默会".to_string()));
    }
}
