//! Questions to an AI model about a document's text: what type of
//! document it is, in what stage, what data it carries.
//!
//! Like text recognition (`recognition.rs`), the engine is a detail
//! behind a contract: callers hand over a question and the JSON shape of
//! the answer, and never name an engine. Engines can be added or swapped
//! without touching them. The questions cost the provider's quota, so
//! nothing here decides when to ask: the document base does, and only on
//! an explicit action of the user or while the archive's wiki is active.

use serde::{Deserialize, Serialize};

use super::recognition_codex::CodexAsker;

/// Which engine answers, and with which model.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskConfig {
    pub engine: String,
    #[serde(default)]
    pub model: String,
}

pub trait DocumentAsker: Send + Sync {
    /// Stable name of the engine and model, saved with every answer.
    fn engine_id(&self) -> String;

    /// The answer to `prompt`, as JSON following the JSON Schema `schema`.
    /// An engine must fail rather than return an empty or partial answer,
    /// so a failure is never saved as an answer.
    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String>;
}

pub fn asker_for(config: &AskConfig) -> Result<Box<dyn DocumentAsker>, String> {
    match config.engine.as_str() {
        "codex-cli" => Ok(Box::new(CodexAsker::new(&config.model))),
        other => Err(format!("Unknown engine for questions about documents: '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_engines_are_accepted() {
        let codex = AskConfig { engine: "codex-cli".into(), model: "m".into() };
        assert_eq!(asker_for(&codex).unwrap().engine_id(), "codex-cli:m");
        assert!(asker_for(&AskConfig { engine: "nada".into(), model: String::new() }).is_err());
    }
}
