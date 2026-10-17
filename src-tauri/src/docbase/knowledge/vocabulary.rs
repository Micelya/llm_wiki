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

/// The version of the program's check of a reading (`extract::verify`).
const READING_CHECK: &str = "lectura-4";

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
    /// Money as a relation between a debtor and a creditor (D-244).
    #[serde(default)]
    pub money: MoneyRules,
    /// The program's checks of a reading (D-245 to D-250).
    #[serde(default)]
    pub reading: ReadingRules,
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
    /// Articles, possessives and demonstratives: a way of writing that
    /// starts with one in lowercase and goes on with a common word ("la
    /// provincia", "su carta documento", "los tribunales ordinarios") is a
    /// phrase, not a proper name (D-259).
    #[serde(default)]
    pub determiners: Vec<String>,
    /// Words of an area or a mailbox of an organization ("legales",
    /// "finance", "cobranzas", "noreply"): a name without a legal form with
    /// one of them, or a mail address whose user is one, is a contact of
    /// an organization, never joined by its name (D-262).
    #[serde(default)]
    pub area_words: Vec<String>,
    /// Words of a group of companies (D-260): a name that starts with one
    /// ("Grupo Ejemplo") names a group; a quote with the start of one of
    /// `affiliateWords` ("afiliada", "subsidiaria") in a document that
    /// names two companies of the same name says they are of one group.
    #[serde(default)]
    pub group_words: Vec<String>,
    #[serde(default)]
    pub affiliate_words: Vec<String>,
    /// Relations that link a person to a company as one of its people (a
    /// director, an attorney, a partner): the same person so linked to two
    /// companies of the same name is a sign of a group.
    #[serde(default)]
    pub group_links: Vec<String>,
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
    /// The kinds of amount that make a matter a claim of one party to
    /// another, which names it (D-267).
    #[serde(default)]
    pub claim_kinds: Vec<String>,
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
            claim_kinds: Vec::new(),
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
    /// A part of a document with at least this many words that the model
    /// reads as naming nobody and saying nothing is taken as a failed
    /// reading, not as an empty document.
    #[serde(default = "default_empty_min_words")]
    pub empty_min_words: usize,
    /// How many times such a part is asked in one run before the reading
    /// is written as failed.
    #[serde(default = "default_empty_attempts")]
    pub empty_attempts: usize,
}

fn default_empty_min_words() -> usize {
    300
}

fn default_empty_attempts() -> usize {
    3
}

impl Default for AttentionRules {
    fn default() -> Self {
        Self {
            about: String::new(),
            noisy_reasons: Vec::new(),
            noisy_min: 4,
            noisy_share: 0.4,
            recent_join_days: 30,
            empty_min_words: default_empty_min_words(),
            empty_attempts: default_empty_attempts(),
        }
    }
}

/// Money as a relation (D-244): the kinds of amount, each with how a page
/// tells it from each side, and the words the direction of a quote is
/// checked with (normalized beginnings of words).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoneyRules {
    #[serde(default)]
    pub about: String,
    /// Beginnings of the verbs of whoever pays or owes ("adeud", "pagar").
    #[serde(default)]
    pub payer_cues: Vec<String>,
    /// Beginnings of the verbs of whoever is paid or claims ("reclam").
    #[serde(default)]
    pub payee_cues: Vec<String>,
    /// Words right before the creditor ("a favor de").
    #[serde(default)]
    pub creditor_markers: Vec<String>,
    /// Words there can be between a name and the verb.
    #[serde(default = "eight")]
    pub window: usize,
    #[serde(default)]
    pub kinds: Vec<MoneyKind>,
}

fn eight() -> usize {
    8
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoneyKind {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    /// How the debtor's page tells it ("Debe a"), and the creditor's.
    #[serde(default)]
    pub debtor: String,
    #[serde(default)]
    pub creditor: String,
}

/// The program's checks of a reading (D-245 to D-250).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingRules {
    #[serde(default)]
    pub about: String,
    /// Beginnings of words of a label that puts a datum under another
    /// person of a form ("cónyuge").
    #[serde(default)]
    pub other_person_labels: Vec<String>,
    /// Marks of the catalog of document types that say a page is the
    /// record of an electronic signing service.
    #[serde(default)]
    pub certificate_marks: Vec<String>,
    /// On such a page, the role a role becomes.
    #[serde(default)]
    pub certificate_roles: std::collections::BTreeMap<String, String>,
    /// Words there can be between the label of a form and its value.
    #[serde(default = "sixty")]
    pub label_window: usize,
}

fn sixty() -> usize {
    60
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
    /// The roles that make an entity one of "the most named" of the
    /// overview: a document where it has one of them counts; a mention in
    /// passing does not (D-240).
    #[serde(default)]
    pub most_named_roles: Vec<String>,
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
    /// An entity of this type with claims of its own (what the documents
    /// say of it) has a page too (D-266).
    #[serde(default)]
    pub with_claims: bool,
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
    /// For a kind of identifier: whether it identifies who it is given to
    /// at all. A number of an invoice, of a shipment or of a signing
    /// envelope does not: it is kept only on a mention of one of `types`
    /// (the matter of the document), never on a person or an organization.
    #[serde(default = "yes")]
    pub identifies: bool,
    /// For a kind of identifier: the labels a text writes it with
    /// (normalized words). The label right before a number says its kind,
    /// whatever kind the model gave (D-249).
    #[serde(default)]
    pub labels: Vec<String>,
}

fn yes() -> bool {
    true
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
    /// Words (or how they start, normalized) of which the quote of a claim
    /// needs one to say it ("adeud", "saldo" for a debt); none: any quote.
    #[serde(default)]
    pub cues: Vec<String>,
    /// Words (or how they start) that say the quote is about something
    /// else ("percepciones", "IVA" next to an amount: a tax, not a debt).
    #[serde(default)]
    pub not_cues: Vec<String>,
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
    /// A relation whose object must be named in the quote, not only the
    /// subject: what has legal consequences (represents, is a party, is
    /// the beneficial owner of) is kept only when the quote says of whom.
    #[serde(default)]
    pub object_in_quote: bool,
    /// A value whose time, when the quote does not say one, is the date of
    /// its document: two values at different dates are a history.
    #[serde(default)]
    pub time_from_document: bool,
}

/// A kind of value, told by words of the quote (normalized, each a run of
/// words that must be in it).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Qualifier {
    pub id: String,
    #[serde(default)]
    pub label: String,
    /// What the model reads to tell it (the "clase" of a claim).
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub words: Vec<String>,
}

impl Predicate {
    pub fn takes_entity(&self) -> bool {
        self.object == "entity"
    }

    /// An amount between a debtor and a creditor (D-244): read in its own
    /// list, with its kind and the amount besides the other party.
    pub fn is_money(&self) -> bool {
        self.shape == "money"
    }

    /// The kinds of value the model is asked to tell ("clase").
    pub fn classes(&self) -> impl Iterator<Item = &Qualifier> {
        self.qualifiers.iter().filter(|q| !q.description.is_empty())
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
            if predicate.is_money() && !predicate.takes_entity() {
                return Err(format!("Predicate '{}': an amount between two parties takes an entity as object", predicate.id));
            }
        }
        let digest = Sha256::digest(text.as_bytes());
        vocabulary.digest = format!("v{}-{}", vocabulary.version, digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
        let read = serde_json::json!({
            "types": vocabulary.entity_types.iter().map(|t| (&t.id, &t.description, &t.types)).collect::<Vec<_>>(),
            "roles": vocabulary.roles.iter().map(|t| (&t.id, &t.description, &t.types)).collect::<Vec<_>>(),
            "identifiers": vocabulary.identifiers.iter().map(|t| (&t.id, &t.description, &t.types, &t.labels, t.identifies)).collect::<Vec<_>>(),
            "predicates": vocabulary
                .predicates
                .iter()
                .map(|p| (&p.id, &p.description, &p.subject, &p.object, &p.object_types, &p.shape, &p.qualifiers, p.object_in_quote))
                .collect::<Vec<_>>(),
            // The kinds of amount are asked; the direction of a quote and
            // the other checks of a reading are computed from these words.
            "money": &vocabulary.money,
            "reading": &vocabulary.reading,
            // The check of a reading drops what is part of an address.
            "addressWords": &vocabulary.identity.address_words,
            // Raised when the program's check of a reading changes: every
            // document is checked again from the answers kept.
            "check": READING_CHECK,
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
        // A web address ("www.ejemplo.com.ar"): where to find someone.
        let lower = text.trim().to_lowercase();
        let web = lower.starts_with("www.")
            || lower.starts_with("http")
            || (!lower.contains(char::is_whitespace)
                && lower.split('.').count() >= 2
                && lower.rsplit('.').next().is_some_and(|tld| (2..=4).contains(&tld.len()) && tld.chars().all(|c| c.is_ascii_lowercase()))
                && lower.split('.').next().is_some_and(|first| first.chars().count() >= 3));
        if web {
            return false;
        }
        if kind == "asunto" {
            // A matter is often named by its number, so a number names it;
            // a file's name ("2020-01-01_Acuerdo.pdf") or only common words
            // ("el acuerdo", "this Agreement") do not (D-259).
            let file = lower.contains('_')
                || lower.rsplit_once('.').is_some_and(|(_, ext)| (2..=4).contains(&ext.len()) && ext.chars().all(|c| c.is_ascii_lowercase()));
            let words = super::quote::words(text);
            return !file && !(words.iter().any(|w| w.chars().any(char::is_alphabetic)) && self.is_generic(&words.join(" ")));
        }
        if letters == 0 || letters < digits || text.contains(['[', ']']) {
            return false;
        }
        let words = super::quote::words(text);
        if words.is_empty() || self.is_generic(&words.join(" ")) {
            return false;
        }
        if self.is_a_phrase(text) {
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

    /// Whether a way of writing is a phrase rather than a proper name: it
    /// starts with an article, a possessive or a demonstrative written in
    /// lowercase, and the next word is in lowercase too ("la provincia",
    /// "su carta documento"), D-259. "la República Argentina" is not one.
    pub fn is_a_phrase(&self, text: &str) -> bool {
        let mut tokens = text.split(|c: char| c.is_whitespace() || c == ',').filter(|t| !t.is_empty());
        let (Some(first), Some(second)) = (tokens.next(), tokens.next()) else { return false };
        let lower = |t: &str| t.chars().find(|c| c.is_alphabetic()).is_some_and(char::is_lowercase);
        let determiner = super::quote::words(first).join(" ");
        lower(first) && lower(second) && self.identity.determiners.iter().any(|d| d == &determiner)
    }

    /// Whether a way of writing can name a matter on its page or as an
    /// alias (D-267): a name (not a file, not only common words, not a
    /// phrase) without a number of four digits or more (a voucher, an
    /// envelope or a file number is an identifier of the matter, shown
    /// apart) and of eight words at most (a title or a sentence is not a
    /// name).
    pub fn is_a_matter_name(&self, text: &str) -> bool {
        let words = super::quote::words(text);
        // The number of a case after its label ("Expediente 1234/2020")
        // is how a case is named; a voucher's is not.
        let captioned = self.identifiers.iter().filter(|i| i.strong && (i.types.is_empty() || i.types.iter().any(|t| t == "asunto"))).any(|i| {
            i.labels.iter().any(|label| {
                let label = super::quote::words(label);
                !label.is_empty() && words.starts_with(&label)
            })
        });
        let long_number = !captioned && words.iter().any(|w| w.chars().filter(|c| c.is_ascii_digit()).count() >= 4);
        // A voucher named by its label ("Factura A", "Facturas enviadas",
        // "Carta documento"): the labels of what identifies no one.
        let voucher = self.identifiers.iter().filter(|i| !i.identifies).any(|i| {
            i.labels.iter().any(|label| {
                let label = super::quote::words(label);
                match label.as_slice() {
                    [one] => words.first().is_some_and(|w| w == one || (one.chars().count() >= 4 && w.starts_with(one.as_str()))),
                    many => !many.is_empty() && words.starts_with(many),
                }
            })
        });
        if voucher {
            return false;
        }
        self.is_a_name("asunto", text) && !self.is_a_phrase(text) && !long_number && !words.is_empty() && words.len() <= 8
    }

    /// Whether this kind of identifier names one entity only.
    pub fn is_strong(&self, identifier: &str) -> bool {
        self.identifier(identifier).is_some_and(|t| t.strong)
    }

    /// The predicate of an amount between two parties, when there is one.
    pub fn money_predicate(&self) -> Option<&Predicate> {
        self.predicates.iter().find(|p| p.is_money())
    }

    pub fn money_kind(&self, id: &str) -> Option<&MoneyKind> {
        self.money.kinds.iter().find(|k| k.id == id)
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
        for name in ["Persona Dos", "Ejemplo Uno S.A.", "Banco de Ejemplo", "Provincia de Ejemplo", "Ejemplo 3 S.R.L.", "S.A.", "J.P. Ejemplo", "la República Ejemplar"] {
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
            "www.ejemplo.com.ar",
            "ejemplo.com",
            "(011) 4444-5555",
            "PROVINCIA",
            "la provincia",
            "su carta documento",
            "presente contrato",
            "accionistas",
            "los tribunales ordinarios de Ejemplo",
        ] {
            assert!(!v.is_a_name("organizacion", not_a_name), "{not_a_name}");
        }
        // A matter is often named by its number.
        assert!(v.is_a_name("asunto", "1234/2020") && v.is_a_name("asunto", "Calle Uno 100"));
        // What names a matter on its page (D-267): not a voucher, a file,
        // a number, a phrase or a title; a case caption does.
        for name in ["Acuerdo de terminación", "Contrato de locación", "Expediente 1234/2020", "Partnership Agreement"] {
            assert!(v.is_a_matter_name(name), "{name}");
        }
        for not_a_name in [
            "Factura A",
            "Facturas enviadas",
            "Carta Documento",
            "BOLETA N° 0024928963",
            "2020-01-01_Ejemplo.pdf",
            "la renovación de la suscripción anual",
            "Acuerdo",
            "1234 - Firmar con un servicio: Ejemplo Uno LLC - Memorando de entendimiento entre partes",
        ] {
            assert!(!v.is_a_matter_name(not_a_name), "{not_a_name}");
        }
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
