//! The vocabulary of the knowledge layer: the types of entity, the roles
//! a mention can have in a document, the identifiers and the predicates
//! of the claims.
//!
//! It is data, not code (R-24, R-26): `src-tauri/resources/catalog/
//! knowledge.json`, compiled in, and replaced at start by a file of the
//! same shape in the app's data folder when there is one, like the
//! catalog of document types. Each entry carries a description in words:
//! that is what the model reads. Changing the vocabulary changes its
//! digest, and every document is read again.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const BUILTIN: &str = include_str!("../../../resources/catalog/knowledge.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vocabulary {
    pub version: u32,
    pub entity_types: Vec<Term>,
    pub roles: Vec<Term>,
    #[serde(default)]
    pub identifiers: Vec<Term>,
    pub predicates: Vec<Predicate>,
    /// Fingerprint of the vocabulary text.
    #[serde(skip)]
    pub digest: String,
}

/// A type of entity, a role or a kind of identifier.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub id: String,
    #[serde(default)]
    pub description: String,
    /// The types of entity it applies to (empty: all).
    #[serde(default)]
    pub types: Vec<String>,
}

/// What a claim can say of an entity.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Predicate {
    pub id: String,
    #[serde(default)]
    pub description: String,
    /// Types of entity it is said of (empty: all).
    #[serde(default)]
    pub subject: Vec<String>,
    /// "entity" (another mention of the document) or "value".
    pub object: String,
    #[serde(default)]
    pub object_types: Vec<String>,
}

impl Predicate {
    pub fn takes_entity(&self) -> bool {
        self.object == "entity"
    }
}

impl Vocabulary {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut vocabulary: Vocabulary =
            serde_json::from_str(text).map_err(|e| format!("The knowledge vocabulary is not valid: {e}"))?;
        if vocabulary.entity_types.is_empty() || vocabulary.roles.is_empty() || vocabulary.predicates.is_empty() {
            return Err("The knowledge vocabulary needs types, roles and predicates".to_string());
        }
        for predicate in &vocabulary.predicates {
            if predicate.object != "entity" && predicate.object != "value" {
                return Err(format!("Predicate '{}': object must be \"entity\" or \"value\"", predicate.id));
            }
        }
        let digest = Sha256::digest(text.as_bytes());
        vocabulary.digest = format!("v{}-{}", vocabulary.version, digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
        Ok(vocabulary)
    }

    pub fn builtin() -> Self {
        Self::parse(BUILTIN).expect("the built-in knowledge vocabulary is valid")
    }

    pub fn entity_type(&self, id: &str) -> Option<&Term> {
        self.entity_types.iter().find(|t| t.id == id)
    }

    pub fn role(&self, id: &str) -> Option<&Term> {
        self.roles.iter().find(|t| t.id == id)
    }

    pub fn identifier(&self, id: &str) -> Option<&Term> {
        self.identifiers.iter().find(|t| t.id == id)
    }

    pub fn predicate(&self, id: &str) -> Option<&Predicate> {
        self.predicates.iter().find(|p| p.id == id)
    }

    /// The role kept when the model gave one that is not in the list, or
    /// one that does not apply to the type: the last one, "other".
    pub fn fallback_role(&self) -> &str {
        self.roles.last().map(|r| r.id.as_str()).unwrap_or_default()
    }
}

static INSTALLED: OnceLock<Vocabulary> = OnceLock::new();

/// The vocabulary in use: the one installed at start, or the built-in.
pub fn vocabulary() -> &'static Vocabulary {
    INSTALLED.get_or_init(Vocabulary::builtin)
}

/// Uses the vocabulary in `path` instead of the built-in one, when the
/// file exists and is valid. Only the first call counts.
pub fn install_from(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    match Vocabulary::parse(&text) {
        Ok(vocabulary) => {
            let _ = INSTALLED.set(vocabulary);
        }
        Err(error) => eprintln!("[docbase] {}: {error}; using the built-in knowledge vocabulary", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_vocabulary_loads_and_is_consistent() {
        let vocabulary = Vocabulary::builtin();
        let types: Vec<&str> = vocabulary.entity_types.iter().map(|t| t.id.as_str()).collect();
        assert!(types.len() >= 3);
        for term in vocabulary.roles.iter().chain(&vocabulary.identifiers) {
            assert!(!term.description.trim().is_empty(), "{} has no description", term.id);
            for t in &term.types {
                assert!(types.contains(&t.as_str()), "{} names an unknown type {t}", term.id);
            }
        }
        for predicate in &vocabulary.predicates {
            assert!(!predicate.description.trim().is_empty());
            for t in predicate.subject.iter().chain(&predicate.object_types) {
                assert!(types.contains(&t.as_str()), "{} names an unknown type {t}", predicate.id);
            }
        }
        // The fallback role applies to every type.
        let fallback = vocabulary.role(vocabulary.fallback_role()).unwrap();
        assert!(fallback.types.is_empty() || types.iter().all(|t| fallback.types.iter().any(|f| f == t)));
    }

    #[test]
    fn another_vocabulary_has_another_digest_and_bad_ones_are_refused() {
        let one = Vocabulary::parse(r#"{"version": 1, "entityTypes": [{"id": "a"}], "roles": [{"id": "r"}], "predicates": [{"id": "p", "object": "value"}]}"#).unwrap();
        let two = Vocabulary::parse(r#"{"version": 1, "entityTypes": [{"id": "b"}], "roles": [{"id": "r"}], "predicates": [{"id": "p", "object": "value"}]}"#).unwrap();
        assert_ne!(one.digest, two.digest);
        assert!(Vocabulary::parse(r#"{"version": 1, "entityTypes": [], "roles": [], "predicates": []}"#).is_err());
        assert!(Vocabulary::parse(r#"{"version": 1, "entityTypes": [{"id": "a"}], "roles": [{"id": "r"}], "predicates": [{"id": "p", "object": "x"}]}"#).is_err());
    }
}
