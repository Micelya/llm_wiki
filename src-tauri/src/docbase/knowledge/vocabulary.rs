//! The vocabulary of the knowledge layer: the types of entity, the roles
//! a mention can have in a document, the identifiers and the predicates
//! of the claims.
//!
//! It is data, not code (R-24, R-26): `src-tauri/resources/catalog/
//! knowledge.json`, compiled in, and replaced at start by a file of the
//! same shape in the app's data folder when there is one, like the
//! catalog of document types. Each entry carries a description in words:
//! that is what the model reads. Changing what the question says (types,
//! roles, identifiers, predicates and their descriptions) changes its
//! `asking_digest`: the readings made before are kept, marked as of an
//! earlier version, and read again only when a person asks (D-432).
//! Changing what the program checks of a reading changes its
//! `reading_digest`: every reading is checked again from the answers kept,
//! without asking the model. The words of the resolution of identities
//! (`identity`) and the marks `strong` and `single` change neither (D-193).

use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The version of the program's check of a reading (`extract::verify`).
const READING_CHECK: &str = "lectura-15";

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
    /// Fingerprint of what the reading of a document is checked with (the
    /// words of the question and of the program's checks): when it
    /// changes, every reading is checked again from the answers kept,
    /// without asking the model. The words of the resolution of identities
    /// and which identifiers are strong do not change it.
    #[serde(skip)]
    pub reading_digest: String,
    /// Fingerprint of what the question to the model says of the
    /// vocabulary (the question of an empty document): when it changes,
    /// the readings made before are of an earlier version (D-432).
    #[serde(skip)]
    pub asking_digest: String,
}

/// Whether a way of writing, in lowercase, is a web address
/// ("www.ejemplo.com.ar"): where to find someone, not who.
fn is_web(lower: &str) -> bool {
    lower.starts_with("www.")
        || lower.starts_with("http")
        || (!lower.contains(char::is_whitespace)
            && lower.split('.').count() >= 2
            && lower.rsplit('.').next().is_some_and(|tld| (2..=4).contains(&tld.len()) && tld.chars().all(|c| c.is_ascii_lowercase()))
            && lower.split('.').next().is_some_and(|first| first.chars().count() >= 3))
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
    /// Offices someone holds ("juez", "secretaria", "actuario"): before a
    /// name they are like a title ("la secretaria Ana"); written alone
    /// they name an office, not a person.
    #[serde(default)]
    pub offices: Vec<String>,
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
    /// How a person is named by what most of its documents write (D-516).
    #[serde(default)]
    pub person_name: PersonNameRule,
}

/// A person is named by what most of its documents write (D-516): with
/// `from_documents` documents or more, a word of its ways of writing that
/// fewer than `min_documents` of them write, or fewer than `min_share` of
/// them, is not added to its name (a second surname, a married name, a
/// misread word seen once). It stays among the ways it is written.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PersonNameRule {
    pub about: String,
    pub from_documents: usize,
    pub min_documents: usize,
    pub min_share: f64,
}

impl Default for PersonNameRule {
    fn default() -> Self {
        Self { about: String::new(), from_documents: 3, min_documents: 2, min_share: 0.2 }
    }
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
    /// Relations that make two documents one for counting matters, besides
    /// the versions of `pages.versionRelations`: the record of the signing
    /// of an act, its translation, its delivery receipt (D-381).
    #[serde(default)]
    pub same_document: Vec<String>,
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
            same_document: Vec::new(),
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
    /// What stands of an amount (P-136): paid, due, agreed, offered,
    /// conditioned, an example... The reading asks it with the kind.
    #[serde(default)]
    pub states: Vec<MoneyState>,
    /// The state a paid amount takes when its date is after the date of
    /// its document: a payment that falls due later is not made yet.
    #[serde(default)]
    pub due_state: String,
}

/// A state of an amount, with how pages tell it.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoneyState {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: String,
    /// The amount was paid (or received): a date after the document's
    /// cannot be one of these.
    #[serde(default)]
    pub paid: bool,
    /// The document does not say it of the parties (an example, a
    /// simulation): it is not a relation between them and is not kept.
    #[serde(default)]
    pub not_a_fact: bool,
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
    /// The predicates a claim read on such a page may have (D-288).
    #[serde(default)]
    pub certificate_claims: Vec<String>,
    /// Words there can be between the label of a form and its value.
    #[serde(default = "sixty")]
    pub label_window: usize,
    /// Predicates whose object, when the quote does not name it, may be
    /// the organization the text names last before the quote, within
    /// `objectWindow` words (a signature block, "X S.A., representada por
    /// Y"): P-122, D-269.
    #[serde(default)]
    pub object_before: Vec<String>,
    #[serde(default)]
    pub object_window: usize,
    /// Beginnings of words of a quote that says how someone acts for a
    /// company ("apoderado", "Title", "representada por"): without one, the
    /// organization before the quote is not taken as its object.
    #[serde(default)]
    pub representation_cues: Vec<String>,
    /// Words with which whoever signs a document speaks of herself ("mi",
    /// "el suscripto", "the undersigned"), normalized: a quote with one of
    /// them names the signer (D-434).
    #[serde(default)]
    pub first_person: Vec<String>,
    /// The roles of a mention that make it whoever signs or writes the
    /// document, for `first_person`.
    #[serde(default)]
    pub signer_roles: Vec<String>,
    /// How a quote names someone by the role they have in the document
    /// ("el causante", "mis mandantes") instead of by their name (D-456).
    #[serde(default)]
    pub role_references: Vec<RoleReference>,
    /// How a text says who is the child of a filiation ("X, hijo de A y
    /// de B"), D-457.
    #[serde(default)]
    pub filiation: Filiation,
    /// Marks of a reading that could not read a name ("[ilegible]", "?"):
    /// a name with one of them names no one.
    #[serde(default)]
    pub illegible_marks: Vec<String>,
    /// First words of a rule or a standard ("ley", "decreto", "norma"): a
    /// name of an organization that starts with one is a rule, not who.
    #[serde(default)]
    pub rule_words: Vec<String>,
}

/// Words with which a document names someone by the role they have in it
/// (D-456): "el causante" is the person whose succession the document is
/// about, "mis mandantes" are the people the signer represents. They name
/// the entities on one side of a relation of the same document; a quote
/// with them names those entities as if it wrote their names.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleReference {
    /// By language, the words (normalized; one or several in a row).
    #[serde(default)]
    pub words: std::collections::BTreeMap<String, Vec<String>>,
    /// The relation of the same document whose side they name.
    pub predicate: String,
    /// "subject" or "object": the side of the relation they name.
    #[serde(default)]
    pub side: String,
    /// Only the relations whose subject signs the document ("mis
    /// mandantes": those the signer represents).
    #[serde(default)]
    pub by_signer: bool,
    /// When the document has none of those relations, the mentions with one
    /// of these roles in it ("parte": whom a lawyer's writ is for), not the
    /// signer's own; with `by_signer`, only in a document someone signs.
    #[serde(default)]
    pub mention_roles: Vec<String>,
    /// They name one only ("el causante"): when the document has several on
    /// that side, they name none.
    #[serde(default)]
    pub single: bool,
}

impl RoleReference {
    /// Whether a quote (normalized words) has its words.
    pub fn in_quote(&self, quote_words: &[String]) -> bool {
        self.words.values().flatten().any(|w| super::quote::contains(quote_words, &super::quote::words(w)))
    }
}

/// The words of a filiation (D-457): in "X, hijo de A y de B" the child is
/// the person named right before the words that say it, in the quote or
/// before it in the text.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Filiation {
    /// The relation from the parent to the child ("padre o madre de").
    #[serde(default)]
    pub predicate: String,
    /// By language, the words that start a filiation ("hijo de").
    #[serde(default)]
    pub cues: std::collections::BTreeMap<String, Vec<String>>,
    /// By language, the words that go on with the list of parents ("y de").
    #[serde(default)]
    pub joins: std::collections::BTreeMap<String, Vec<String>>,
    /// By language, the words right before a name that make it someone
    /// else than the child ("casado con": the spouse listed before the
    /// parents of a certificate).
    #[serde(default)]
    pub others: std::collections::BTreeMap<String, Vec<String>>,
    /// How many words before the filiation the child can be named.
    #[serde(default)]
    pub window: usize,
}

impl Filiation {
    pub fn cue_words(&self) -> Vec<Vec<String>> {
        self.cues.values().flatten().map(|c| super::quote::words(c)).filter(|w| !w.is_empty()).collect()
    }

    pub fn join_words(&self) -> Vec<Vec<String>> {
        self.joins.values().flatten().map(|c| super::quote::words(c)).filter(|w| !w.is_empty()).collect()
    }

    pub fn other_words(&self) -> Vec<Vec<String>> {
        self.others.values().flatten().map(|c| super::quote::words(c)).filter(|w| !w.is_empty()).collect()
    }
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
    /// Words of a timetable ("9 hs a 18 hs", "GMT-3"): a value with one of
    /// them is when, not where (D-288).
    #[serde(default)]
    pub schedule: Vec<String>,
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
    /// Folder of the pages of documents inside the wiki project.
    #[serde(default)]
    pub documents_folder: String,
    /// The relations between two documents that make them versions of one
    /// same document (a draft and its signed copy, a preview): they share
    /// one page (D-340). Documents of one template are pages apart.
    #[serde(default)]
    pub version_relations: Vec<String>,
    /// The relations whose subject is the version a page of versions is
    /// named after (the signed copy, not the draft).
    #[serde(default)]
    pub principal_relations: Vec<String>,
    /// Headings and phrases of the pages, by key.
    #[serde(default)]
    pub words: std::collections::BTreeMap<String, String>,
    /// Who the title of a document's page names, by their roles in it
    /// (P-182).
    #[serde(default)]
    pub title_roles: TitleRoles,
}

/// The roles of who sends a document, of whom it goes to, and of who it is
/// between when it says neither (a contract).
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TitleRoles {
    #[serde(default)]
    pub from: Vec<String>,
    #[serde(default)]
    pub to: Vec<String>,
    #[serde(default)]
    pub between: Vec<String>,
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

    pub fn documents_folder(&self) -> &str {
        if self.documents_folder.trim().is_empty() {
            "wiki/sources"
        } else {
            self.documents_folder.trim().trim_end_matches('/')
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
    /// For a kind of identifier whose first digits say what it is given
    /// to (a CUIT of a person and one of a company start differently): by
    /// type of entity, the prefixes of its numbers. A number with the
    /// prefix of another type is not this mention's (D-277).
    #[serde(default)]
    pub prefixes: std::collections::BTreeMap<String, Vec<String>>,
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
    /// One of several words is a run of whole words.
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
    /// The only values it takes (a marital status), each with the words
    /// that say it (the feminine and the masculine alike): a value with
    /// none of them is not a claim of it, and two values with words of the
    /// same one are one claim (D-516).
    #[serde(default)]
    pub closed_values: Vec<ClosedValue>,
    /// A relation whose object must be named in the quote, not only the
    /// subject: what has legal consequences (represents, is a party, is
    /// the beneficial owner of) is kept only when the quote says of whom.
    #[serde(default)]
    pub object_in_quote: bool,
    /// A relation whose subject must be named in the quote too (a spouse, a
    /// kinship): a quote that names others and not the subject was given to
    /// the wrong person (D-433).
    #[serde(default)]
    pub subject_in_quote: bool,
    /// A relation that is doubtful when every quote of it names other
    /// people or organizations and not its subject (the object aside; the
    /// first person counts as whoever signs the document), D-434.
    #[serde(default)]
    pub attribution: bool,
    /// A relation that is the other side of another one ("hijo de" is the
    /// other side of "padre o madre de"): it is kept as that one, with its
    /// subject and object swapped, so that it is stored once and shown from
    /// both sides.
    #[serde(default)]
    pub inverse_of: String,
    /// Beginnings of words of which one comes before the name of the
    /// object in the quote ("apoderado de Y", "en representación de Y"),
    /// or, `trail_cues`, after it ("Y, representada por"): a quote that
    /// names the object only away from them (a professional registration
    /// before "en representación de") does not say the relation (D-435).
    #[serde(default)]
    pub lead_cues: Vec<String>,
    #[serde(default)]
    pub trail_cues: Vec<String>,
    /// Words (normalized; one or several in a row) of a name that cannot
    /// be the subject, or the object, of the relation: a court does not
    /// represent anyone nor is a party in the case it hears (D-435).
    #[serde(default)]
    pub subject_not: Vec<String>,
    #[serde(default)]
    pub object_not: Vec<String>,
    /// Whole words that, between the names of the subject and the object
    /// and without a word of `cues` there, say the relation is of others:
    /// the children named after the union of their parents are not their
    /// spouses (D-433).
    #[serde(default)]
    pub not_between: Vec<String>,
    /// What the same document must not say of either side for the relation
    /// to be firm: a person said to be single and the spouse of someone is
    /// doubtful (D-434).
    #[serde(default)]
    pub contradicted_by: Option<Contradiction>,
    /// A relation with the matter of the document (being the person whose
    /// succession it is): when the reading gives no matter as its object
    /// (none, or the person herself), the object is the matter of the
    /// document whose name holds the subject's name, when it is one (D-456).
    #[serde(default)]
    pub object_from_document: bool,
    /// A value whose time, when the quote does not say one, is the date of
    /// its document: two values at different dates are a history.
    #[serde(default)]
    pub time_from_document: bool,
}

/// Values of another predicate, said of the subject or the object in the
/// same document, that do not fit a relation (D-434): by language, the
/// beginnings of their words.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Contradiction {
    pub predicate: String,
    #[serde(default)]
    pub words: std::collections::BTreeMap<String, Vec<String>>,
}

impl Contradiction {
    /// Whether a value (normalized words) is one of them, in any language.
    pub fn says(&self, value: &str) -> bool {
        let words: Vec<&str> = value.split_whitespace().collect();
        self.words.values().flatten().any(|cue| {
            let cue: Vec<&str> = cue.split_whitespace().collect();
            match cue.as_slice() {
                [one] => words.iter().any(|w| w.starts_with(one)),
                many => !many.is_empty() && words.windows(many.len()).any(|w| w == many),
            }
        })
    }
}

/// Whether a name (normalized words) has one of `list` (each one word or
/// several in a row, normalized) as its words.
pub fn name_has(name: &[String], list: &[String]) -> bool {
    list.iter().any(|entry| {
        let entry: Vec<&str> = entry.split_whitespace().collect();
        !entry.is_empty() && name.windows(entry.len()).any(|w| w.iter().map(String::as_str).eq(entry.iter().copied()))
    })
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

/// One of the values of a closed list (`closedValues`): its canonical id
/// and the words that say it (normalized; one or several in a row).
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClosedValue {
    pub id: String,
    #[serde(default)]
    pub words: Vec<String>,
}

impl Predicate {
    /// The value of the closed list a value says, with the word of it the
    /// value writes (the first of the list when it writes several of one):
    /// the first value whose words the value has; none when it has the words
    /// of none, or of two different values ("soltero y casado").
    pub fn closed_value(&self, value: &str) -> Option<(&ClosedValue, String)> {
        let words = super::quote::words(value);
        let mut found: Vec<(&ClosedValue, String)> = Vec::new();
        for closed in &self.closed_values {
            if let Some(word) = closed.words.iter().find(|w| name_has(&words, std::slice::from_ref(*w))) {
                found.push((closed, word.clone()));
            }
        }
        (found.len() == 1).then(|| found.remove(0))
    }

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
        for predicate in &vocabulary.predicates {
            if !predicate.inverse_of.is_empty() {
                let fits = vocabulary.predicates.iter().any(|p| p.id == predicate.inverse_of && p.takes_entity() && p.inverse_of.is_empty());
                if !fits || !predicate.takes_entity() {
                    return Err(format!("Predicate '{}': inverseOf must name a relation that is not itself the other side of one", predicate.id));
                }
            }
            if let Some(contradiction) = &predicate.contradicted_by {
                if !vocabulary.predicates.iter().any(|p| p.id == contradiction.predicate) {
                    return Err(format!("Predicate '{}': contradictedBy names an unknown predicate", predicate.id));
                }
            }
        }
        let digest = Sha256::digest(text.as_bytes());
        vocabulary.digest = format!("v{}-{}", vocabulary.version, digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
        let read = serde_json::json!({
            "types": vocabulary.entity_types.iter().map(|t| (&t.id, &t.description, &t.types)).collect::<Vec<_>>(),
            "roles": vocabulary.roles.iter().map(|t| (&t.id, &t.description, &t.types)).collect::<Vec<_>>(),
            "identifiers": vocabulary.identifiers.iter().map(|t| (&t.id, &t.description, &t.types, &t.labels, t.identifies)).collect::<Vec<_>>(),
            // Every word of a predicate the check uses (the quote that must
            // name both sides, the cues, the names that cannot be a side).
            "predicates": vocabulary
                .predicates
                .iter()
                .map(|p| {
                    serde_json::json!([
                        &p.id, &p.description, &p.subject, &p.object, &p.object_types, &p.shape, &p.qualifiers, p.object_in_quote,
                        p.subject_in_quote, &p.inverse_of, &p.lead_cues, &p.trail_cues, &p.subject_not, &p.object_not, &p.not_between, &p.cues,
                        p.object_from_document
                    ])
                })
                .collect::<Vec<_>>(),
            // The kinds of amount are asked; the direction of a quote and
            // the other checks of a reading are computed from these words.
            "money": &vocabulary.money,
            "reading": &vocabulary.reading,
            // The check of a reading drops what is part of an address.
            "addressWords": &vocabulary.identity.address_words,
            // It drops a person named only by an office.
            "offices": &vocabulary.identity.offices,
            // The check of a reading moves a number with another type's prefix.
            "identifierPrefixes": vocabulary.identifiers.iter().map(|t| (&t.id, &t.prefixes)).collect::<Vec<_>>(),
            // Raised when the program's check of a reading changes: every
            // document is checked again from the answers kept.
            "check": READING_CHECK,
        });
        let digest = Sha256::digest(read.to_string().as_bytes());
        vocabulary.reading_digest = format!("r-{}", digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
        // What the question says of the vocabulary: the question of an
        // empty document, which has every word of it the model reads.
        let asked = super::extract::prompt(&vocabulary, &super::extract::Hints::default(), "", 0, 1);
        let digest = Sha256::digest(asked.as_bytes());
        vocabulary.asking_digest = format!("q-{}", digest.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>());
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
            .filter(|w| !words.stopwords.iter().any(|s| s == w) && !words.titles.iter().any(|t| t == w) && !words.offices.iter().any(|o| o == w))
            .all(|w| words.generic.iter().any(|g| g == w))
    }

    /// Why a way of writing of a person or an organization names no one,
    /// whatever the model read: a mark of what a reading could not
    /// read ("Ana [ilegible]", "J?"); three letters or more apart, as a
    /// scan spaces them ("J u a n"); a mail or a web address; only an
    /// office or a title ("la actuaria", "Sr."), for a person; a rule or a
    /// standard ("Ley 123", "Norma ABC"), for an organization. The check of
    /// a reading drops such a mention (but a mail address, which still
    /// says whose contact it is); the screens and the pages show none.
    pub fn not_named(&self, kind: &str, text: &str) -> Option<&'static str> {
        if self.is_reference(kind) {
            return None;
        }
        let lower = text.trim().to_lowercase();
        if self.reading.illegible_marks.iter().any(|m| !m.trim().is_empty() && lower.contains(m.trim())) {
            return Some("nombre-ilegible");
        }
        if self.spaced_letters(text) {
            return Some("letras-sueltas");
        }
        if lower.contains('@') || is_web(&lower) {
            return Some("correo-o-web");
        }
        let words = super::quote::words(text);
        let identity = &self.identity;
        let named: Vec<&String> =
            words.iter().filter(|w| !identity.stopwords.contains(w) && !identity.determiners.contains(w)).collect();
        let office = |w: &String| identity.offices.contains(w) || identity.titles.contains(w);
        if kind == super::names::PERSON && !named.is_empty() && named.iter().all(|w| office(w)) {
            return Some("cargo-sin-nombre");
        }
        // A rule has no legal form: "Código Abierto S.A." is a company.
        if kind == super::names::ORGANIZATION
            && named.first().is_some_and(|w| self.reading.rule_words.contains(w))
            && super::names::key(self, kind, text).form.is_none()
        {
            return Some("norma-no-organizacion");
        }
        None
    }

    /// Whether a way of writing has three letters or more in a row, each a
    /// word apart, as a scan spaces the letters of a word ("C a r l o s"):
    /// with one in lowercase, or four or more. Initials ("J. R. R.") have a
    /// point; capitals of a legal form ("S A U") or of initials ("J R R")
    /// are three at most and none in lowercase.
    fn spaced_letters(&self, text: &str) -> bool {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        let single = |t: &str| t.chars().count() == 1 && t.chars().all(char::is_alphabetic);
        let forms: Vec<String> = self.identity.legal_forms.iter().flat_map(|f| f.variants.iter().cloned()).collect();
        let mut start = 0;
        while start < tokens.len() {
            if !single(tokens[start]) {
                start += 1;
                continue;
            }
            let mut end = start;
            while end < tokens.len() && single(tokens[end]) {
                end += 1;
            }
            let run = &tokens[start..end];
            let lowercase = run.iter().any(|t| t.chars().all(char::is_lowercase));
            let form = forms.iter().any(|f| *f == run.join(" ").to_lowercase());
            if run.len() >= 3 && (lowercase || run.len() >= 4) && !form {
                return true;
            }
            start = end;
        }
        false
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
        let lower = text.trim().to_lowercase();
        if is_web(&lower) {
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
        if letters == 0 || letters < digits || text.contains(['[', ']']) || self.not_named(kind, text).is_some() {
            return false;
        }
        let words = super::quote::words(text);
        if words.is_empty() || self.is_generic(&words.join(" ")) {
            return false;
        }
        // A field of a form written with abbreviations ("N° A.R. (Troquel
        // T&T)"): initials and a number sign, and every word of two
        // letters or more generic, is a label, not a name (D-288).
        let abbreviated = text.contains(['°', 'º']) || text.as_bytes().windows(2).any(|w| w[0].is_ascii_alphabetic() && w[1] == b'.');
        let long: Vec<String> = words.iter().filter(|w| w.chars().count() >= 2).cloned().collect();
        if abbreviated && !long.is_empty() && self.is_generic(&long.join(" ")) {
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

    /// Whether the mentions of this type are references of their document
    /// (the number of a record, a file, "the contract of X"), not entities:
    /// the reading still finds them, but they are never resolved into an
    /// entity; they are data of the document, searchable, and evidence to
    /// group documents (D-377). The type is the one of `matters.kind`.
    pub fn is_reference(&self, kind: &str) -> bool {
        self.matters.kind == kind
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

    pub fn money_state(&self, id: &str) -> Option<&MoneyState> {
        self.money.states.iter().find(|s| s.id == id)
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
/// The file the vocabulary in use came from, when it is not the built-in.
static INSTALLED_FROM: OnceLock<std::path::PathBuf> = OnceLock::new();

/// The vocabulary in use: the one installed at start, or the built-in.
pub fn vocabulary() -> &'static Vocabulary {
    INSTALLED.get_or_init(Vocabulary::builtin)
}

/// The file the vocabulary in use came from; none for the built-in one.
pub fn installed_from() -> Option<&'static std::path::Path> {
    INSTALLED_FROM.get().map(|p| p.as_path())
}

/// Uses the vocabulary in `path` instead of the built-in one, when the
/// file exists and is valid. Only the first call counts.
pub fn install_from(path: &std::path::Path) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    match Vocabulary::parse(&text) {
        Ok(vocabulary) => {
            if INSTALLED.set(vocabulary).is_ok() {
                let _ = INSTALLED_FROM.set(path.to_path_buf());
            }
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
        let titles = &vocabulary.pages.title_roles;
        assert!(!titles.from.is_empty() && !titles.to.is_empty() && !titles.between.is_empty());
        for role in titles.from.iter().chain(&titles.to).chain(&titles.between) {
            assert!(vocabulary.role(role).is_some(), "the titles of documents name an unknown role {role}");
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
            "N° A.R. (Troquel T&T)",
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
