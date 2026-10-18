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

/// Which engine answers, with which model and how much reasoning.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AskConfig {
    pub engine: String,
    #[serde(default)]
    pub model: String,
    /// How much the model reasons before answering (`low`, `medium` or
    /// `high`); empty leaves the model's own default. Part of the engine's
    /// name: answers given with another level are other answers.
    #[serde(default)]
    pub reasoning: String,
}

/// The reasoning levels an engine may be asked for. Higher ones exist in
/// some models but are not offered (D-274): the process is improved by
/// adjusting the questions, not by spending more on each answer.
pub const REASONING_LEVELS: [&str; 3] = ["low", "medium", "high"];

/// The reasoning level of a configuration, checked: empty when none was
/// chosen, an error for one that is not offered.
pub fn reasoning_level(reasoning: &str) -> Result<String, String> {
    let level = reasoning.trim().to_lowercase();
    if level.is_empty() || REASONING_LEVELS.contains(&level.as_str()) {
        Ok(level)
    } else {
        Err(format!("Unknown reasoning level for questions about documents: '{reasoning}'"))
    }
}

/// The name of a model with its reasoning level, as engines save it:
/// `model@level`, or just `model` when no level was chosen.
pub fn model_with_reasoning(model: &str, reasoning: &str) -> String {
    let model = model.trim();
    let level = reasoning.trim().to_lowercase();
    if level.is_empty() {
        model.to_string()
    } else {
        format!("{model}@{level}")
    }
}

pub trait DocumentAsker: Send + Sync {
    /// Stable name of the engine and model, saved with every answer.
    fn engine_id(&self) -> String;

    /// The answer to `prompt`, as JSON following the JSON Schema `schema`.
    /// An engine must fail rather than return an empty or partial answer,
    /// so a failure is never saved as an answer.
    fn ask(&self, prompt: &str, schema: &serde_json::Value) -> Result<String, String>;
}

/// The name an engine and model are saved with (what `engine_id` of its
/// asker returns), without starting the engine.
pub fn engine_id(config: &AskConfig) -> String {
    let model = model_with_reasoning(&config.model, &config.reasoning);
    if model.is_empty() {
        config.engine.clone()
    } else {
        format!("{}:{model}", config.engine)
    }
}

pub fn asker_for(config: &AskConfig) -> Result<Box<dyn DocumentAsker>, String> {
    let reasoning = reasoning_level(&config.reasoning)?;
    match config.engine.as_str() {
        "codex-cli" => Ok(Box::new(CodexAsker::new(&config.model).with_reasoning(&reasoning))),
        other => Err(format!("Unknown engine for questions about documents: '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_engines_are_accepted() {
        let codex = AskConfig { engine: "codex-cli".into(), model: "m".into(), ..Default::default() };
        assert_eq!(asker_for(&codex).unwrap().engine_id(), "codex-cli:m");
        assert!(asker_for(&AskConfig { engine: "nada".into(), ..Default::default() }).is_err());
        assert_eq!(engine_id(&codex), asker_for(&codex).unwrap().engine_id());
        let bare = AskConfig { engine: "codex-cli".into(), model: " ".into(), ..Default::default() };
        assert_eq!(engine_id(&bare), asker_for(&bare).unwrap().engine_id());
    }

    #[test]
    fn the_reasoning_level_is_part_of_the_engine_and_only_offered_levels_are_accepted() {
        let medium = AskConfig { engine: "codex-cli".into(), model: "m".into(), reasoning: "medium".into() };
        let high = AskConfig { reasoning: " High ".into(), ..medium.clone() };
        assert_eq!(engine_id(&medium), "codex-cli:m@medium");
        assert_eq!(engine_id(&high), "codex-cli:m@high");
        assert_eq!(asker_for(&high).unwrap().engine_id(), engine_id(&high));
        assert_eq!(asker_for(&medium).unwrap().engine_id(), engine_id(&medium));
        assert_ne!(engine_id(&medium), engine_id(&AskConfig { reasoning: String::new(), ..medium.clone() }));
        for level in ["xhigh", "max", "ultra", "mucho"] {
            assert!(asker_for(&AskConfig { reasoning: level.into(), ..medium.clone() }).is_err(), "{level}");
        }
        // A configuration saved before the level existed reads as none.
        let old: AskConfig = serde_json::from_str(r#"{"engine":"codex-cli","model":"m"}"#).unwrap();
        assert_eq!(old.reasoning, "");
        assert_eq!(engine_id(&old), "codex-cli:m");
        let saved: AskConfig = serde_json::from_str(&serde_json::to_string(&high).unwrap()).unwrap();
        assert_eq!(saved, high);
    }
}
