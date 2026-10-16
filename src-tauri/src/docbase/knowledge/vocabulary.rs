//! The vocabulary of the knowledge layer: the types of entity, the roles
//! a mention can have in a document, the identifiers and the predicates
//! of the claims.
//!
//! It is data, not code (R-24, R-26): `src-tauri/resources/catalog/
//! knowledge.json`, compiled in, and replaced at start by a file of the
//! same shape in the app's data folder when there is one, like the
//! catalog of document types. Each entry carries a description in words:
//! that is what the model reads. Changing what the reading uses (types,
//! roles, identifiers, predicates and their descriptions) changes its
//! `reading_digest`, and every document is read again; the words of the
//! resolution of identities (`identity`) and the marks `strong` and
//! `single` do not (D-193).

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
    /// What the resolution of identities does not count when it compares
    /// names, and the legal forms of organizations.
    #[serde(default)]
    pub identity: IdentityWords,
    /// Which entities have a page in the archive's wiki, and the words the
    /// pages are written with (`pages.rs`).
    #[serde(default)]
    pub pages: Pages,
    /// The words the shapes of values are checked with.
    #[serde(default)]
    pub values: ValueWords,
    /// How documents are grouped into matters (`matters.rs`).
    #[serde(default)]
    pub matters: MatterRules,
    /// What the layer brings to the attention view (`screens.rs`).
    #[serde(default)]
    pub attention: AttentionRules,
    /// Fingerprint of the vocabulary text.
    #[serde(skip)]
    pub digest: String,
    /// Fingerprint of what the reading of a document uses (types, roles,
    /// identifiers and predicates, with their descriptions): changing
    /// anything else (the words of the resolution of identities, which
    /// identifiers are strong) does not read the archive again.
    #[serde(skip)]
    pub reading_digest: String,
}

/// The words the resolution of identities works with (D-186).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityWords {
    #[serde(default)]
    pub about: String,
    /// Words that are not part of a name ("de", "the"…).
    #[serde(default)]
    pub stopwords: Vec<String>,
    /// Titles and forms of address before a person's name ("dr", "sr"…).
    #[serde(default)]
    pub titles: Vec<String>,
    /// Legal forms of organizations, each with the ways it is written
    /// (normalized words). The same name with another form is another
    /// organization.
    #[serde(default)]
    pub legal_forms: Vec<LegalForm>,
    /// Words that name someone without being a name ("el cliente", "mi",
    /// "la locataria"): a mention written only with them is not a name to
    /// compare, to show as an alias or to name an entity by.
    #[serde(default)]
    pub generic: Vec<String>,
    /// Words of an address ("calle", "piso", "cp"…): a way of writing with
    /// one of them and a number is where someone is, not who.
    #[serde(default)]
    pub address_words: Vec<String>,
}

/// How documents are grouped into matters (D-213).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatterRules {
    #[serde(default)]
    pub about: String,
    /// The type of entity a matter is.
    pub kind: String,
    /// Relations between two documents, confirmed by a person, that keep
    /// them in one matter (ids of the catalog of document types).
    #[serde(default)]
    pub relations: Vec<String>,
    /// Kinds of identifier of a matter whose same number in two documents
    /// puts them in one matter, and how long and with how many digits.
    #[serde(default)]
    pub identifier_kinds: Vec<String>,
    pub min_length: usize,
    pub min_digits: usize,
    /// Digits of an amount, at least, for the same amount to count.
    pub amount_digits: usize,
    /// A number or amount in more than this share of the documents is too
    /// common to say anything (R-16).
    pub max_share: f64,
    /// Roles of a matter's mention that say the document is about it.
    #[serde(default)]
    pub subject_roles: Vec<String>,
    /// Roles of the parties of a matter, and how many name it.
    #[serde(default)]
    pub party_roles: Vec<String>,
    pub parties_in_name: usize,
    /// Documents a matter needs for a page of its own.
    pub min_documents: usize,
    /// A document no evidence puts in a matter is asked about the matters
    /// with which it shares this many parties or more, up to this many
    /// matters in one question (`matters::propose`).
    #[serde(default = "two")]
    pub propose_with_shared: usize,
    #[serde(default = "three")]
    pub propose_candidates: usize,
}

fn two() -> usize {
    2
}

fn three() -> usize {
    3
}

impl Default for MatterRules {
    fn default() -> Self {
        Self {
            about: String::new(),
            kind: "asunto".into(),
            relations: Vec::new(),
            identifier_kinds: Vec::new(),
            min_length: 6,
            min_digits: 3,
            amount_digits: 4,
            max_share: 0.5,
            subject_roles: Vec::new(),
            party_roles: Vec::new(),
            parties_in_name: 2,
            min_documents: 2,
            propose_with_shared: 2,
            propose_candidates: 3,
        }
    }
}

/// What the layer brings to the attention view (`screens.rs`).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionRules {
    #[serde(default)]
    pub about: String,
    /// Reasons of a discarded piece that say its quote is not in the text
    /// (what a badly recognized scan does).
    #[serde(default)]
    pub noisy_reasons: Vec<String>,
    /// A reading with at least this many such discards, and more than this
    /// share of all the model said, is shown as a problem.
    pub noisy_min: usize,
    pub noisy_share: f64,
    /// Joins by identifier of different names are shown when the document
    /// that brought them was read less than this many days ago.
    pub recent_join_days: i64,
}

impl Default for AttentionRules {
    fn default() -> Self {
        Self { about: String::new(), noisy_reasons: Vec::new(), noisy_min: 4, noisy_share: 0.4, recent_join_days: 30 }
    }
}

/// The words the shapes of values are checked with (normalized).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ValueWords {
    #[serde(default)]
    pub about: String,
    /// Currency signs and names: with them, a number is an amount.
    #[serde(default)]
    pub currencies: Vec<String>,
    /// Words of an address that do not tell two addresses apart ("piso",
    /// "ciudad", "provincia").
    #[serde(default)]
    pub address_noise: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegalForm {
    pub id: String,
    pub variants: Vec<String>,
    /// Also written before the name ("Fundación …", "Verein …").
    #[serde(default)]
    pub before: bool,
}

/// Which entities have a page (decided over the whole archive, D-196),
/// where the pages go and the words they are written with.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pages {
    #[serde(default)]
    pub about: String,
    /// Folder of the pages inside the wiki project.
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub kinds: Vec<PageRule>,
    /// Headings and phrases of the pages, by key.
    #[serde(default)]
    pub words: std::collections::BTreeMap<String, String>,
}

/// When an entity of one type has a page: it appears in `min_documents`
/// documents or more, or has one of `roles` in any, or (with
/// `strong_identifier`) a document gives it a strong identifier; and, when
/// `roles_required` says some, one of its mentions has one of them.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageRule {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub min_documents: usize,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub strong_identifier: bool,
    /// Besides, at least one mention must have one of these roles (for a
    /// matter: a document that is about it, not one that only cites it).
    #[serde(default)]
    pub roles_required: Vec<String>,
    /// Entities of this type have no page of their own: their page is the
    /// one of the matter (group of documents) they are in (`matters.rs`).
    #[serde(default)]
    pub by_matter: bool,
}

impl Pages {
    /// The word kept under `key`, or the key itself when it is missing.
    pub fn word<'a>(&'a self, key: &'a str) -> &'a str {
        self.words.get(key).map(String::as_str).unwrap_or(key)
    }

    pub fn rule(&self, kind: &str) -> Option<&PageRule> {
        self.kinds.iter().find(|rule| rule.kind == kind)
    }

    pub fn folder(&self) -> &str {
        if self.folder.trim().is_empty() {
            "wiki/entities"
        } else {
            self.folder.trim().trim_end_matches('/')
        }
    }
}

/// A type of entity, a role or a kind of identifier.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Term {
    pub id: String,
    /// How the pages of the wiki name it.
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    /// The types of entity it applies to (empty: all).
    #[serde(default)]
    pub types: Vec<String>,
    /// An identifier that names one entity only (a DNI, a CUIT): the same
    /// one joins two mentions by itself, a different one keeps them apart.
    #[serde(default)]
    pub strong: bool,
}

/// What a claim can say of an entity.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Predicate {
    pub id: String,
    /// How the pages of the wiki name it.
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    /// Types of entity it is said of (empty: all).
    #[serde(default)]
    pub subject: Vec<String>,
    /// "entity" (another mention of the document) or "value".
    pub object: String,
    /// How the pages name the claim seen from its object ("representada
    /// por" for "representa a"); none: it is not shown from the object.
    #[serde(default)]
    pub inverse_label: String,
    #[serde(default)]
    pub object_types: Vec<String>,
    /// One value at a time (a domicile, a marital status): two different
    /// values for the same time are a conflict, at different times a history.
    #[serde(default)]
    pub single: bool,
    /// What a value must look like to count (`values.shapes`): an amount
    /// has digits, a text has more letters than digits and is not the name
    /// of whom it is said of, an address has a number and words. A value
    /// of another shape is kept in the reading but is not a claim.
    #[serde(default)]
    pub shape: String,
    /// Types of entity a value is said within (a position is held in a
    /// company): the entity of the document the quote names, or the one
    /// the subject is related to in the document. The same value within
    /// two entities is two claims; values within different entities are
    /// never in conflict.
    #[serde(default)]
    pub within: Vec<String>,
    /// Kinds of value the quote can say (a domicile "constituted" for a
    /// proceeding): values of different kinds are not in conflict.
    #[serde(default)]
    pub qualifiers: Vec<Qualifier>,
}

/// A kind of value, told by words of the quote (normalized, each a run of
/// words that must be in it).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Qualifier {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub words: Vec<String>,
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
        let read = serde_json::json!({
            "types": vocabulary.entity_types.iter().map(|t| (&t.id, &t.description, &t.types)).collect::<Vec<_>>(),
            "roles": vocabulary.roles.iter().map(|t| (&t.id, &t.description, &t.types)).collect::<Vec<_>>(),
            "identifiers": vocabulary.identifiers.iter().map(|t| (&t.id, &t.description, &t.types)).collect::<Vec<_>>(),
            "predicates": vocabulary
                .predicates
                .iter()
                .map(|p| (&p.id, &p.description, &p.subject, &p.object, &p.object_types))
                .collect::<Vec<_>>(),
        });
        let digest = Sha256::digest(read.to_string().as_bytes());
        vocabulary.reading_digest = format!("r-{}", digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
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

    /// Whether a normalized name is only generic words (or no word at all).
    pub fn is_generic(&self, normalized: &str) -> bool {
        let words = &self.identity;
        normalized
            .split_whitespace()
            .filter(|w| !words.stopwords.iter().any(|s| s == w) && !words.titles.iter().any(|t| t == w))
            .all(|w| words.generic.iter().any(|g| g == w))
    }

    /// Whether a way of writing can name a person or an organization (a
    /// matter is often named by its number, so any name does): not a mail
    /// address or a number, not a blank of a form ("[Nombre]"), not only
    /// generic words ("apoderado", "Uds."), not an address or a postal code
    /// read as a mention. Such a mention stays in the record, but it gets
    /// no page, no link and does not name anything.
    pub fn is_a_name(&self, kind: &str, text: &str) -> bool {
        let letters = text.chars().filter(|c| c.is_alphabetic()).count();
        let digits = text.chars().filter(|c| c.is_ascii_digit()).count();
        if text.contains('@') {
            return false;
        }
        if kind == "asunto" {
            return true;
        }
        if letters == 0 || letters < digits || text.contains(['[', ']']) {
            return false;
        }
        let words = super::quote::words(text);
        if words.is_empty() || self.is_generic(&words.join(" ")) {
            return false;
        }
        if digits > 0 && words.iter().any(|w| self.identity.address_words.iter().any(|a| a == w)) {
            return false;
        }
        // A street number or a postal code: a word of three digits or more,
        // or a code that mixes letters with four digits or more.
        !words.iter().any(|w| {
            let d = w.chars().filter(|c| c.is_ascii_digit()).count();
            d >= 3 && (d == w.chars().count() || d >= 4)
        })
    }

    /// Whether this kind of identifier names one entity only.
    pub fn is_strong(&self, identifier: &str) -> bool {
        self.identifier(identifier).is_some_and(|t| t.strong)
    }

    /// The role kept when the model gave one that is not in the list, or
    /// one that does not apply to the type: the last one, "other".
    /// How the pages name a type, a role, an identifier or a predicate:
    /// its label, or its id when it has none.
    pub fn type_label<'a>(&'a self, id: &'a str) -> &'a str {
        self.entity_type(id).map(|t| t.label.as_str()).filter(|l| !l.is_empty()).unwrap_or(id)
    }

    pub fn role_label<'a>(&'a self, id: &'a str) -> &'a str {
        self.role(id).map(|t| t.label.as_str()).filter(|l| !l.is_empty()).unwrap_or(id)
    }

    pub fn identifier_label<'a>(&'a self, id: &'a str) -> &'a str {
        self.identifier(id).map(|t| t.label.as_str()).filter(|l| !l.is_empty()).unwrap_or(id)
    }

    pub fn predicate_label<'a>(&'a self, id: &'a str) -> &'a str {
        self.predicate(id).map(|p| p.label.as_str()).filter(|l| !l.is_empty()).unwrap_or(id)
    }

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
        // Every type has a page rule, naming known roles, and every word
        // a page needs is there.
        for t in &types {
            let rule = vocabulary.pages.rule(t).unwrap_or_else(|| panic!("no page rule for {t}"));
            for role in rule.roles.iter().chain(&rule.roles_required) {
                assert!(vocabulary.role(role).is_some(), "the page rule of {t} names an unknown role {role}");
            }
        }
        for key in super::super::pages::WORDS {
            assert!(vocabulary.pages.words.contains_key(*key), "the pages have no word for {key}");
        }
        for term in vocabulary.entity_types.iter().chain(&vocabulary.roles).chain(&vocabulary.identifiers) {
            assert!(!term.label.is_empty(), "{} has no label", term.id);
        }
        assert!(vocabulary.predicates.iter().all(|p| !p.label.is_empty()));
        // The fallback role applies to every type.
        let fallback = vocabulary.role(vocabulary.fallback_role()).unwrap();
        assert!(fallback.types.is_empty() || types.iter().all(|t| fallback.types.iter().any(|f| f == t)));
    }

    #[test]
    fn what_is_not_a_name_gets_no_page() {
        let v = Vocabulary::builtin();
        for name in ["Persona Dos", "Ejemplo Uno S.A.", "Banco de Ejemplo", "Provincia de Ejemplo", "Ejemplo 3 S.R.L."] {
            assert!(v.is_a_name("persona", name) && v.is_a_name("organizacion", name), "{name}");
        }
        for not_a_name in [
            "apoderado",
            "Uds.",
            "mi representada",
            "[Nombre / Razón social]",
            "Calle Uno 100, Ciudad Ejemplo",
            "Av. Ejemplo 1234 piso 3",
            "Ejemplo 555 Ciudad 1000",
            "X1234ABC",
            "alguien@ejemplo.com",
            "(011) 4444-5555",
        ] {
            assert!(!v.is_a_name("organizacion", not_a_name), "{not_a_name}");
        }
        // A matter is often named by its number.
        assert!(v.is_a_name("asunto", "1234/2020") && v.is_a_name("asunto", "Calle Uno 100"));
    }

    #[test]
    fn another_vocabulary_has_another_digest_and_bad_ones_are_refused() {
        let one = Vocabulary::parse(r#"{"version": 1, "entityTypes": [{"id": "a"}], "roles": [{"id": "r"}], "predicates": [{"id": "p", "object": "value"}]}"#).unwrap();
        let two = Vocabulary::parse(r#"{"version": 1, "entityTypes": [{"id": "b"}], "roles": [{"id": "r"}], "predicates": [{"id": "p", "object": "value"}]}"#).unwrap();
        assert_ne!(one.digest, two.digest);
        assert!(Vocabulary::parse(r#"{"version": 1, "entityTypes": [], "roles": [], "predicates": []}"#).is_err());
        // The words of the resolution of identities do not change what a
        // reading uses.
        let three = Vocabulary::parse(r#"{"version": 1, "entityTypes": [{"id": "a"}], "roles": [{"id": "r"}], "predicates": [{"id": "p", "object": "value"}], "identity": {"stopwords": ["de"]}}"#).unwrap();
        assert_ne!(one.digest, three.digest);
        assert_eq!(one.reading_digest, three.reading_digest);
        assert_ne!(one.reading_digest, two.reading_digest);
        assert!(Vocabulary::parse(r#"{"version": 1, "entityTypes": [{"id": "a"}], "roles": [{"id": "r"}], "predicates": [{"id": "p", "object": "x"}]}"#).is_err());
    }
}
