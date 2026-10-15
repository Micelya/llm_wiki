//! The catalog of document types and of relations between documents.
//!
//! It is data, not code (R-26): `src-tauri/resources/catalog/
//! document-types.json`, compiled in, and replaced at start by a file of
//! the same shape in the app's data folder when there is one. Adding a
//! type, a variant of one (another postal service, another signing
//! service) or a relation is adding an entry there.
//!
//! What a signal can look at, all without a model:
//! - phrases of the text, compared without case, accents or punctuation;
//! - the shape of an identifier (`A` a capital letter, `9` a digit);
//! - whether the PDF is a form (it has fields);
//! - the digital signatures embedded in the PDF, told apart into the
//!   seal a signing service puts on every copy it issues and the
//!   signature of a party.

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const BUILTIN: &str = include_str!("../../../resources/catalog/document-types.json");

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub version: u32,
    #[serde(default)]
    pub relations: Vec<RelationDef>,
    #[serde(default)]
    pub types: Vec<DocType>,
    /// Fingerprint of the catalog text: what was computed with another
    /// catalog is computed again.
    #[serde(skip)]
    pub digest: String,
}

/// One relation two documents can have. Its name for the user comes from
/// the vocabulary of the vertical, by `id`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationDef {
    pub id: String,
    /// Both files are one document: once joined, only one of them feeds
    /// the wiki.
    #[serde(default)]
    pub same_document: bool,
    /// One of the two is the relation of the other ("a is the signed copy
    /// of b"), not both of each other.
    #[serde(default)]
    pub directed: bool,
    /// Found by comparing texts; the others are inferred.
    #[serde(default)]
    pub measured: bool,
    /// Applied without a person (only the exact same text).
    #[serde(default)]
    pub automatic: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocType {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub variants: Vec<Variant>,
    #[serde(default)]
    pub signals: Vec<Signal>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    pub id: String,
    /// Who issues the form or runs the service. The only company names
    /// the catalog may hold (R-24).
    #[serde(default)]
    pub issuer: String,
    /// Words in the certificate of an embedded signature that make it the
    /// seal of this service rather than the signature of a party.
    #[serde(default)]
    pub certificate_names: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Signal {
    pub id: String,
    #[serde(default)]
    pub variant: Option<String>,
    #[serde(default)]
    pub stage: Option<String>,
    /// The mark it leaves on the document ("preview", "party-signature"…).
    #[serde(default)]
    pub mark: Option<String>,
    #[serde(default)]
    pub phrases: Vec<String>,
    /// How many of `phrases` must be there (1 when not said).
    #[serde(default)]
    pub min_phrases: Option<usize>,
    #[serde(default)]
    pub shapes: Vec<String>,
    /// "document" (the default) or "page": the phrases must be together
    /// on one page.
    #[serde(default)]
    pub scope: Option<String>,
    /// A page where this signal is found is not content of the document
    /// (the history page a signing service adds) and is left out of the
    /// comparison.
    #[serde(default)]
    pub exclude_page: bool,
    /// Lines holding one of the phrases are left out of the comparison
    /// (a stamp repeated in the margin of every page).
    #[serde(default)]
    pub drop_lines: bool,
    /// Only when the PDF is a form (true) or is not (false).
    #[serde(default)]
    pub form_fields: Option<bool>,
    /// "service" or "party": an embedded signature of that kind.
    #[serde(default)]
    pub pdf_signature: Option<String>,
    /// Other signals of the same type that must have been found too.
    #[serde(default)]
    pub requires: Vec<String>,
}

impl Signal {
    pub fn on_page(&self) -> bool {
        self.scope.as_deref() == Some("page")
    }

    pub fn needed_phrases(&self) -> usize {
        self.min_phrases.unwrap_or(1).max(1).min(self.phrases.len().max(1))
    }
}

impl Catalog {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut catalog: Catalog =
            serde_json::from_str(text).map_err(|e| format!("The document type catalog is not valid: {e}"))?;
        catalog.digest = format!("{:016x}", super::text::fnv1a(text.as_bytes()));
        Ok(catalog)
    }

    pub fn builtin() -> Self {
        Self::parse(BUILTIN).expect("the built-in document type catalog is valid")
    }

    pub fn relation(&self, id: &str) -> Option<&RelationDef> {
        self.relations.iter().find(|relation| relation.id == id)
    }

    /// Every signing service named by a variant, with the certificate
    /// words that identify its seal.
    pub fn signing_services(&self) -> impl Iterator<Item = &Variant> {
        self.types
            .iter()
            .flat_map(|t| t.variants.iter())
            .filter(|variant| !variant.certificate_names.is_empty())
    }

    /// Who issues each form or service named in the catalog.
    #[cfg(test)]
    pub fn issuers(&self) -> Vec<String> {
        self.types
            .iter()
            .flat_map(|t| t.variants.iter())
            .flat_map(|variant| {
                std::iter::once(variant.issuer.clone()).chain(variant.certificate_names.iter().cloned())
            })
            .filter(|issuer| !issuer.is_empty())
            .collect()
    }
}

static INSTALLED: OnceLock<Catalog> = OnceLock::new();

/// The catalog in use: the one installed at start, or the built-in one.
pub fn catalog() -> &'static Catalog {
    INSTALLED.get_or_init(Catalog::builtin)
}

/// Uses the catalog in `path` instead of the built-in one, when the file
/// exists and is valid. Only the first call counts.
pub fn install_from(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    match Catalog::parse(&text) {
        Ok(catalog) => {
            let _ = INSTALLED.set(catalog);
        }
        Err(error) => eprintln!("[docbase] {}: {error}; using the built-in catalog", path.display()),
    }
}

/// Whether `token` has the form `shape` describes: `A` a capital letter,
/// `9` a digit, anything else itself.
pub fn has_shape(token: &str, shape: &str) -> bool {
    let token: Vec<char> = token.chars().collect();
    let shape: Vec<char> = shape.chars().collect();
    token.len() == shape.len()
        && token.iter().zip(&shape).all(|(t, s)| match s {
            'A' => t.is_ascii_uppercase(),
            '9' => t.is_ascii_digit(),
            other => t == other,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_catalog_loads_and_names_every_measured_relation() {
        let catalog = Catalog::builtin();
        for id in [
            "near-identical",
            "signed-copy",
            "preview",
            "version",
            "contained-in",
            "same-template",
            "similar",
        ] {
            assert!(catalog.relation(id).is_some_and(|r| r.measured), "{id}");
        }
        assert!(catalog.relation("signed-copy").unwrap().same_document);
        assert!(!catalog.relation("same-template").unwrap().same_document);
        assert!(catalog.signing_services().count() >= 1);
        // Signal ids are unique within each type: `requires` names them.
        for doc_type in &catalog.types {
            let mut ids: Vec<_> = doc_type.signals.iter().map(|s| &s.id).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), doc_type.signals.len(), "{}", doc_type.id);
        }
    }

    #[test]
    fn a_new_relation_or_type_is_only_data() {
        let text = r#"{"version": 1,
            "relations": [{"id": "annex-of", "directed": true, "measured": false}],
            "types": [{"id": "recibo", "signals": [{"id": "x", "mark": "receipt", "phrases": ["recibí de"]}]}]}"#;
        let catalog = Catalog::parse(text).unwrap();
        assert!(catalog.relation("annex-of").unwrap().directed);
        assert_eq!(catalog.types[0].signals[0].mark.as_deref(), Some("receipt"));
        assert_ne!(catalog.digest, Catalog::builtin().digest);
    }

    #[test]
    fn shapes_describe_identifiers() {
        assert!(has_shape("XY123456789AR", "AA999999999AR"));
        assert!(!has_shape("XY12345678AR", "AA999999999AR"));
        assert!(!has_shape("xy123456789AR", "AA999999999AR"));
        assert!(has_shape("Q12345ZZ678", "A99999AA999"));
    }
}
