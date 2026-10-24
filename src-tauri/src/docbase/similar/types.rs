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
    /// A person decides it: the two may be one same document (a copy, a
    /// version, one inside the other, its delivery receipt or its signing
    /// certificate). The rest (the same template with other data, only
    /// similar) are different documents: they are shown and wait for no
    /// one (D-497).
    #[serde(default)]
    pub asks_person: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocType {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub stages: Vec<String>,
    /// How each stage is told apart, in words: it goes to the model.
    #[serde(default)]
    pub stage_descriptions: std::collections::BTreeMap<String, String>,
    /// It says how a document was issued or made (a signing service, a
    /// fillable form), not what the document is: its marks never give the
    /// type, and its data and rules apply to whatever document carries
    /// them.
    #[serde(default)]
    pub carrier: bool,
    #[serde(default)]
    pub variants: Vec<Variant>,
    #[serde(default)]
    pub signals: Vec<Signal>,
    /// The data that matter in a document of this type (pass 5).
    #[serde(default)]
    pub data: Vec<DataRule>,
    /// How two documents of this type relate (pass 6).
    #[serde(default)]
    pub rules: Vec<RelationRule>,
}

/// One datum of a type, and the general pattern it is found with.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataRule {
    pub id: String,
    /// What it is, in words: it goes to the model when the rules do not
    /// find it.
    #[serde(default)]
    pub description: String,
    /// "identifier", "amount", "date", "number" or "text" (the rest of
    /// the line).
    #[serde(default)]
    pub kind: String,
    /// It is the value found within `window` tokens after one of these
    /// phrases (compared like the signals' phrases).
    #[serde(default)]
    pub after: Vec<String>,
    /// Or any token with one of these shapes.
    #[serde(default)]
    pub shapes: Vec<String>,
    /// Or the value of the document's marks of this kind.
    #[serde(default)]
    pub from_mark: Option<String>,
    /// Tokens looked at after the phrase (12 when not said).
    #[serde(default)]
    pub window: Option<usize>,
    /// Every value found, not only the first. After a phrase, the values
    /// follow one another while each is within `window` tokens of the
    /// last (a list of invoices).
    #[serde(default)]
    pub many: bool,
    /// Of the values found, the one kept: "first" (the default) or
    /// "largest" (a total is the largest amount next to the word).
    #[serde(default)]
    pub pick: Option<String>,
    /// When the rules do not find it, the model is asked.
    #[serde(default)]
    pub required: bool,
}

/// How the type corrects or completes the relation measured for two
/// documents (pass 6). What the general rule already resolved is left
/// alone.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelationRule {
    pub id: String,
    /// - `different-data`: two documents of this type whose `data` differ
    ///   are different documents, however much text they share: a
    ///   relation that makes them one document becomes `relation`.
    /// - `stage-pair`: one in `subjectStage` and the other in one of
    ///   `otherStages`, sharing their base text, are `relation`, the first
    ///   being the subject.
    /// - `shared-data`: a document of this type and one of `otherType`
    ///   with the same `data` are `relation`, this one being the subject.
    pub kind: String,
    #[serde(default)]
    pub data: Vec<String>,
    #[serde(default)]
    pub subject_stage: Option<String>,
    #[serde(default)]
    pub other_stages: Vec<String>,
    #[serde(default)]
    pub other_type: Option<String>,
    pub relation: String,
    /// How sure the relation is when the rule holds: `low` when not said
    /// (a datum two documents may share by chance), `high` for a datum
    /// that names one thing only (the identifier of a signing process).
    #[serde(default)]
    pub certainty: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    pub id: String,
    /// Who issues the form or runs the service. The only company names
    /// the catalog may hold (R-24).
    #[serde(default)]
    pub issuer: String,
    /// Words in the signer's certificate of an embedded signature that
    /// make it the seal of this service rather than the signature of a
    /// party. They count only when its chain verifies up to one of
    /// `root_fingerprints` (seal.rs): a name alone proves nothing.
    #[serde(default)]
    pub certificate_names: Vec<String>,
    /// SHA-256 fingerprints (hex) of the certificates the service's seal
    /// chains up to: its root, or the authority it trusts below the root.
    #[serde(default)]
    pub root_fingerprints: Vec<String>,
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

    pub fn doc_type(&self, id: &str) -> Option<&DocType> {
        self.types.iter().find(|doc_type| doc_type.id == id)
    }

    /// The types a document can be (not the carriers): the closed list
    /// the model chooses from (R-22).
    pub fn document_types(&self) -> impl Iterator<Item = &DocType> {
        self.types.iter().filter(|doc_type| !doc_type.carrier)
    }

    /// Every signing service named by a variant, with the certificates
    /// its seal chains up to.
    pub fn signing_services(&self) -> impl Iterator<Item = &Variant> {
        self.types
            .iter()
            .flat_map(|t| t.variants.iter())
            .filter(|variant| !variant.root_fingerprints.is_empty())
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
/// The file the catalog in use came from, when it is not the built-in one.
static INSTALLED_FROM: OnceLock<std::path::PathBuf> = OnceLock::new();

/// The catalog in use: the one installed at start, or the built-in one.
pub fn catalog() -> &'static Catalog {
    INSTALLED.get_or_init(Catalog::builtin)
}

/// The file the catalog in use came from; none for the built-in one.
pub fn installed_from() -> Option<&'static std::path::Path> {
    INSTALLED_FROM.get().map(|p| p.as_path())
}

/// Uses the catalog in `path` instead of the built-in one, when the file
/// exists and is valid. Only the first call counts.
pub fn install_from(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    match Catalog::parse(&text) {
        Ok(catalog) => {
            if INSTALLED.set(catalog).is_ok() {
                let _ = INSTALLED_FROM.set(path.to_path_buf());
            }
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
    fn data_and_rules_name_what_exists() {
        let catalog = Catalog::builtin();
        let kinds = ["identifier", "amount", "date", "number", "text"];
        let rule_kinds = ["different-data", "stage-pair", "shared-data"];
        assert!(catalog.document_types().count() >= 5);
        for doc_type in &catalog.types {
            for datum in &doc_type.data {
                assert!(kinds.contains(&datum.kind.as_str()), "{} {}", doc_type.id, datum.id);
                assert!(!datum.description.is_empty(), "{} {}", doc_type.id, datum.id);
                assert!(!datum.after.is_empty() || !datum.shapes.is_empty() || datum.from_mark.is_some(), "{} {}", doc_type.id, datum.id);
            }
            for rule in &doc_type.rules {
                assert!(rule_kinds.contains(&rule.kind.as_str()), "{} {}", doc_type.id, rule.id);
                assert!(catalog.relation(&rule.relation).is_some(), "{} {}", doc_type.id, rule.id);
                for field in &rule.data {
                    let other = rule.other_type.as_deref().and_then(|t| catalog.doc_type(t));
                    let known = |t: &DocType| t.data.iter().any(|d| &d.id == field);
                    assert!(known(doc_type) && other.is_none_or(known), "{} {} {field}", doc_type.id, rule.id);
                }
                for stage in rule.subject_stage.iter().chain(&rule.other_stages) {
                    assert!(doc_type.stages.contains(stage), "{} {} {stage}", doc_type.id, rule.id);
                }
            }
            for stage in doc_type.stage_descriptions.keys() {
                assert!(doc_type.stages.contains(stage), "{} {stage}", doc_type.id);
            }
            for signal in &doc_type.signals {
                if let Some(stage) = &signal.stage {
                    assert!(doc_type.stages.contains(stage), "{} {}", doc_type.id, signal.id);
                }
            }
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
