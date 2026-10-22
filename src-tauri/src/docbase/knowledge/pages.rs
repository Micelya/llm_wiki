//! Stage 3 of the knowledge layer: the pages of the archive that are a
//! projection of the record (D-175), kept in the base (D-402): each page is
//! a row of `knowledge_page` with its markdown body, named by its logical
//! path (`wiki/entities/…`, `wiki/sources/…`), which is what the links
//! between pages say. No file is read or written.
//!
//! - **The page of each entity** that has one (`wiki/entities/`): its
//!   header (type, name, aliases, id, every document that names it), what
//!   the documents say of it, the list of its documents grouped by matter
//!   with its role in each and the entities related to it. The notes of a
//!   person are apart (`knowledge_note`), never written over. Which
//!   entities have a page is
//!   decided over the whole archive, by the rules of the vocabulary
//!   (`pages.kinds`, D-196): never by one document.
//! - **What the documents say** is written by the model from the verified
//!   claims of the entity alone, numbered; every sentence names the
//!   claims that back it, and a computation throws away a sentence that
//!   names none, names one that does not exist, or carries a number that
//!   none of its claims carries (D-197). Claims in conflict are listed as
//!   such, all values in view. The answer is kept like any other
//!   (`model_answer`): the same claims ask nothing again.
//! - **The page of each document** (`pages/family.rs`, D-340), one per
//!   document or family of versions: what the base knows of it, its block
//!   of entities (who and what it names, with its role and a link to its
//!   page, between two marks), its versions, its data, the summary of the
//!   one reading and its amounts, each saying which version it comes
//!   from.
//! - **The overview** of the archive (`knowledge_overview`), from the
//!   record, in the words of the vocabulary.
//!
//! A page's file name is fixed the first time (a readable name plus the
//! entity's id) and kept when the entity is renamed: links never break
//! (D-195). Everything is computed again from the record each time the
//! record changes, and only what changed is written (R-13, R-15): the
//! order in which documents were read or resolved does not matter.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::quote;
use super::vocabulary::{vocabulary, Vocabulary};
use crate::docbase::similar::classify::{self as classify, ask_and_keep, cached_answer, parse_json, stops_the_run, Question, ASKING};
use crate::docbase::{now_ms, sql_err, DocBase, Visible};
use crate::source_volume::asking::DocumentAsker;

mod family;
mod import;
#[cfg(test)]
mod check_rebuild;
#[cfg(test)]
mod check_retire;

pub(crate) use family::document_title;

/// Each document of the base with its versions (D-340): the id of the
/// family (`doc-…`, what its page and notes are kept under) and its
/// documents, the one it is named after first.
pub(crate) fn document_families(base: &DocBase) -> Result<Vec<(String, Vec<i64>)>, String> {
    Ok(family::families(base, vocabulary())?.list.into_iter().map(|f| (f.id, f.members)).collect())
}

/// Version of the question that writes what the documents say.
pub const SYNTHESIS_PROMPT: &str = "sintesis-3";
/// Raised when a change in the program changes what a projection writes
/// (which names have a page, what a block lists): every page is written
/// again from the record, without asking more.
const PROJECTION_VERSION: &str = "proyeccion-12";
/// The task its answers are kept under in `model_answer`.
pub const TASK: &str = "synthesis";
/// The words of the vocabulary (`pages.words`) a page is written with.
#[cfg_attr(not(test), allow(dead_code))]
pub const WORDS: &[&str] = &[
    "says",
    "saysNothing",
    "saysPending",
    "conflicts",
    "saysMore",
    "saysMoreOne",
    "documents",
    "withoutMatter",
    "related",
    "inCommon",
    "inCommonOne",
    "months",
    "dateFormat",
    "notes",
    "notesMark",
    "aliases",
    "page",
    "documentWord",
    "documentsWord",
    "kept",
    "keptMerged",
    "sourceBlock",
    "sourceBlockMark",
    "overviewTitle",
    "overviewIntro",
    "overviewDocuments",
    "overviewFailed",
    "overviewEntities",
    "overviewMost",
    "overviewMatters",
    "within",
    "matterBetween",
    "matterUnnamed",
    "matterParties",
    "matterWhy",
    "matterFacts",
    "matterNamed",
    "matterSameDocument",
    "matterSameMatter",
    "matterSameNumber",
    "matterSameAmount",
    "matterByPerson",
    "matterOf",
    "matterProposed",
    "matterProposedNote",
    "proposed",
    "relations",
    "matterClaim",
    "matterTyped",
    "partiesAnd",
    "matterOfOne",
    "matterWhyMany",
    "groupKind",
    "groupMembers",
    "groupOf",
    "groupWhy-mismas-personas",
    "groupWhy-misma-casilla",
    "groupWhy-lo-dice-un-documento",
    "groupWhy-nombrado-como-grupo",
    "groupWhy-mismo-nombre",
    "certainDoubtful",
    "certainConfirmed",
    "certainCorrected",
    "certainLegend",
    "doubtWhy-conflict",
    "doubtWhy-identity",
    "doubtWhy-attribution",
    "doubtWhy-states",
    "doubtWhy-contradicted",
    "docSummary",
    "docKind",
    "docDate",
    "docPending",
    "docAmounts",
    "docVersions",
    "docVersionsNote",
    "docVersion",
    "docLocations",
    "docData",
    "docRelated",
    "docThis",
    "docProposed",
    "docJoined",
    "docTitleFrom",
    "docTitleTo",
    "docTitleBetween",
];
/// Claims of one entity described in its question; the rest are left
/// out of the synthesis (the list of documents is always complete).
pub const MAX_CLAIMS: usize = 120;
/// Quotes shown for one claim.
const QUOTES_PER_CLAIM: usize = 2;
const MAX_RELATED: usize = 12;
const MOST_NAMED: usize = 10;
/// Where the language of the wiki is kept (`docbase_setting`).
const WIKI_LANGUAGE_SETTING: &str = "wiki-language";
/// What the last complete projection was computed on.
const BASIS_SETTING: &str = "knowledge-pages-basis";
/// The logical path of the overview, what links to it say.
pub(crate) const OVERVIEW_PATH: &str = "wiki/overview.md";
/// The marks around what the app writes inside a page it does not own,
/// and before the notes of a person. They are found by these words alone.
pub const BLOCK_START: &str = "<!-- micelya:entidades:inicio";
pub const BLOCK_END: &str = "<!-- micelya:entidades:fin -->";
pub const NOTES_MARK: &str = "<!-- micelya:notas";
/// Marks a page written by the app from the record.
pub const GENERATED_BY: &str = "micelya-conocimiento";

/// One projection at a time.
static PROJECTING: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// What a projection did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectReport {
    /// Nothing to do: the resolution is not done, or nothing changed since
    /// the last projection.
    pub skipped: bool,
    pub reason: Option<String>,
    /// Entities with a page.
    pub pages: usize,
    pub written: usize,
    pub unchanged: usize,
    /// Pages removed (their entity has no page any more and they held no
    /// notes), and pages kept only for their notes.
    pub removed: usize,
    pub kept: usize,
    /// Pages moved to the path of their entity's new name, and texts of
    /// the base (pages, notes, saved queries) whose links were redirected
    /// (D-239).
    pub moved: usize,
    pub redirected: usize,
    /// Questions sent to the model, failed, and left for another run.
    pub asked: usize,
    pub failed: usize,
    pub left: usize,
    /// Sentences of the syntheses in the pages, and thrown away by the check.
    pub sentences: usize,
    pub discarded: usize,
    /// Pages of documents, and how many got their block written.
    pub source_pages: usize,
    pub blocks_written: usize,
    pub overview_written: bool,
    pub errors: Vec<String>,
}

impl ProjectReport {
    /// Whether a page of the base changed.
    pub fn wrote(&self) -> bool {
        self.written + self.removed + self.blocks_written + self.redirected > 0 || self.overview_written
    }
}

#[derive(Debug, Clone)]
struct Claim {
    id: i64,
    predicate: String,
    /// The entity the value is said within (a position in a company), and
    /// the kind of value its quote says.
    context: Option<String>,
    qualifier: Option<String>,
    object_entity: Option<String>,
    object_value: Option<String>,
    state: String,
    valid_from: Option<String>,
    /// (content, quote, page)
    evidence: Vec<(i64, String, Option<i64>)>,
    /// An amount between two parties (D-244): the amount as written and
    /// what else its reading says (kind, currency, concept).
    amount: Option<String>,
    detail: Option<serde_json::Value>,
    /// The same amount listed on the page of its creditor: `object_entity`
    /// is then the debtor.
    mirrored: bool,
    /// How sure the record is of it (D-320): "quoted", "confirmed",
    /// "doubtful" or "corrected", and why.
    certainty: String,
    reasons: Vec<String>,
}

impl Claim {
    fn doubtful(&self) -> bool {
        self.certainty == super::claims::DOUBTFUL
    }

    /// Doubtful for a reason the page does not already show apart: a
    /// conflict is listed with all its values.
    fn doubtful_beyond_conflict(&self) -> bool {
        self.doubtful() && self.reasons.iter().any(|r| r != "conflict" && r != "by-person")
    }
}

/// The level and reasons of a certainty as the record keeps it.
fn certainty_of(json: Option<&str>) -> (String, Vec<String>) {
    let value: serde_json::Value = json.and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default();
    let level = value.get("level").and_then(|l| l.as_str()).unwrap_or(super::claims::QUOTED).to_string();
    let reasons = value
        .get("reasons")
        .and_then(|r| r.as_array())
        .map(|list| list.iter().filter_map(|r| r.as_str().map(str::to_string)).collect())
        .unwrap_or_default();
    (level, reasons)
}

/// What a page says after a claim of how sure the record is of it: nothing
/// for a claim with its quote and nobody's word on it; "(sin confirmar:
/// …)" for a doubtful one, with why; who confirmed or corrected it.
fn certainty_mark(vocabulary: &Vocabulary, claim: &Claim) -> String {
    let words = &vocabulary.pages;
    match claim.certainty.as_str() {
        super::claims::DOUBTFUL => {
            let why: Vec<&str> = claim
                .reasons
                .iter()
                .filter_map(|r| words.words.get(&format!("doubtWhy-{r}")).map(String::as_str))
                .collect();
            if why.is_empty() {
                format!(" *({})*", words.word("certainDoubtful"))
            } else {
                format!(" *({}: {})*", words.word("certainDoubtful"), why.join("; "))
            }
        }
        super::claims::CONFIRMED => format!(" *({})*", words.word("certainConfirmed")),
        super::claims::CORRECTED => format!(" *({})*", words.word("certainCorrected")),
        _ => String::new(),
    }
}

#[derive(Debug, Clone, Default)]
struct Entity {
    id: String,
    kind: String,
    name: String,
    state: String,
    merged_into: Option<String>,
    /// Ways it is written, with how many mentions.
    written: BTreeMap<String, usize>,
    /// Its documents, with its roles in each.
    documents: BTreeMap<i64, BTreeSet<String>>,
    /// (kind, normalized) -> (value as shown, documents)
    identifiers: BTreeMap<(String, String), (String, BTreeSet<i64>)>,
    claims: Vec<Claim>,
    /// A group of companies (D-260): its companies, with the reasons.
    members: Vec<(String, Vec<String>)>,
    /// The groups a company is in.
    groups: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct Document {
    name: String,
    /// What the wiki calls it (`<origin>/<path inside it>`).
    identity: Option<String>,
    /// The page the wiki wrote from it.
    page: Option<String>,
    doc_type: Option<String>,
    /// Joined with another one as the same document, and that one chosen
    /// as the wiki's source.
    chosen: Option<i64>,
}

#[derive(Debug, Clone)]
struct PageRow {
    path: String,
    state: String,
    fingerprint: String,
    /// Whether its body is in the base (V20: none until projected again).
    has_body: bool,
}

/// The record, as the pages need it.
struct Record {
    entities: BTreeMap<String, Entity>,
    documents: BTreeMap<i64, Document>,
    pages: BTreeMap<String, PageRow>,
    /// The matters (groups of documents, `matters.rs`), and the matters of
    /// each document.
    matters: BTreeMap<String, Matter>,
    matter_of: BTreeMap<i64, BTreeSet<String>>,
    /// The matters the model proposed for documents no evidence puts in
    /// one, with its reason: a proposal for a person (R-03).
    proposed: BTreeMap<i64, Vec<(String, Option<String>)>>,
}

/// A matter: a group of documents, with the reasons each one is in it.
#[derive(Debug, Clone, Default)]
struct Matter {
    id: String,
    name: String,
    documents: BTreeMap<i64, Vec<super::matters::Reason>>,
}

/// A sentence of the synthesis, as the model answers it.
#[derive(Debug, Deserialize)]
struct Answer {
    #[serde(default)]
    oraciones: Vec<Sentence>,
}

#[derive(Debug, Deserialize)]
struct Sentence {
    texto: String,
    #[serde(default)]
    afirmaciones: Vec<i64>,
}

fn hash(parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update([0]);
    }
    hasher.finalize().iter().take(12).map(|b| format!("{b:02x}")).collect()
}

/// The readable part of a page's file name: the words of the name, in
/// lower case without accents, joined by hyphens.
pub fn slug(name: &str, fallback: &str) -> String {
    let mut out = String::new();
    for word in quote::words(name) {
        let clean: String = word.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        if clean.is_empty() {
            continue;
        }
        if !out.is_empty() && out.len() + clean.len() + 1 > 60 {
            break;
        }
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(&clean);
    }
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

/// The path of a new page: the folder, the readable name and the short
/// form of the entity's id, which no other entity shares.
pub fn page_path(folder: &str, id: &str, name: &str, kind: &str) -> String {
    let short: String = id.trim_start_matches("ent-").trim_start_matches("mat-").chars().take(8).collect();
    format!("{folder}/{}-{short}.md", slug(name, kind))
}

/// What a wiki link names a page by: its file name without `.md`.
pub(crate) fn stem(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

fn link(path: &str, text: &str) -> String {
    let text = text.replace(['[', ']', '|'], " ");
    format!("[[{}|{}]]", stem(path), text.trim())
}

fn yaml_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

fn today() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// The digits of a text, by runs: "$ 100.000" has "100" and "000".
pub(crate) fn digit_runs(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut run = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if c.is_ascii_digit() {
            run.push(c);
        } else if !run.is_empty() {
            out.insert(std::mem::take(&mut run));
        }
    }
    out
}

/// Takes out references written by the model inside a sentence ("[2]",
/// "[^2]": the page numbers its notes itself).
fn without_markers(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let after = &rest[start + 1..];
        match after.find(']') {
            Some(end)
                if !after[..end].trim_start_matches('^').is_empty()
                    && after[..end].trim_start_matches('^').chars().all(|c| c.is_ascii_digit() || c == ',' || c == ' ') =>
            {
                out.push_str(rest[..start].trim_end());
                rest = &after[end + 1..];
            }
            _ => {
                out.push_str(&rest[..start + 1]);
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

impl Entity {
    /// The most used way of writing it, after its name.
    fn aliases(&self) -> Vec<String> {
        let mut list: Vec<(&String, &usize)> = self.written.iter().collect();
        list.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let own = quote::words(&self.name).join(" ");
        let mut seen = BTreeSet::from([own]);
        let vocabulary = vocabulary();
        // A form that is another one with a note in parentheses ("Persona
        // Uno (Presidente)") says nothing more (D-289).
        let bare = |w: &str| -> String {
            let w = w.trim();
            match (w.ends_with(')'), w.rfind('(')) {
                (true, Some(open)) if open > 0 => quote::words(&w[..open]).join(" "),
                _ => quote::words(w).join(" "),
            }
        };
        let forms: BTreeSet<String> = list.iter().map(|(w, _)| quote::words(w).join(" ")).chain(seen.iter().cloned()).collect();
        list.into_iter()
            .filter(|(w, _)| vocabulary.is_a_name(&self.kind, w))
            .filter(|(w, _)| {
                let b = bare(w);
                b == quote::words(w).join(" ") || !forms.contains(&b)
            })
            .filter(|(w, _)| seen.insert(quote::words(w).join(" ")))
            .map(|(w, _)| w.clone())
            .collect()
    }

    fn has_strong_identifier(&self, vocabulary: &Vocabulary) -> bool {
        self.identifiers.keys().any(|(kind, _)| vocabulary.is_strong(kind))
    }
}

/// The same number given once with its kind and once as just "another
/// number" is one identifier: the one with its kind, with the documents
/// of both (D-289).
fn one_value_once(identifiers: &mut BTreeMap<(String, String), (String, BTreeSet<i64>)>, fallback: &str) {
    let loose: Vec<(String, String)> = identifiers.keys().filter(|(kind, _)| kind == fallback).cloned().collect();
    for key in loose {
        let kinded = identifiers.keys().find(|(kind, normalized)| kind != fallback && *normalized == key.1).cloned();
        if let Some(kinded) = kinded {
            if let Some((_, documents)) = identifiers.remove(&key) {
                identifiers.get_mut(&kinded).expect("found").1.extend(documents);
            }
        }
    }
}

/// Whether a way of writing is the name of a person or an organization: a
/// mail address, a telephone or a number read as a mention is not (it is
/// contact data or an identifier). A matter is often named by its number.
pub fn name_like(kind: &str, text: &str) -> bool {
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let digits = text.chars().filter(|c| c.is_ascii_digit()).count();
    !text.contains('@') && (kind == "asunto" || (letters > 0 && letters >= digits))
}

/// English months, as documents in English write them.
const ENGLISH_MONTHS: [&str; 12] =
    ["january", "february", "march", "april", "may", "june", "july", "august", "september", "october", "november", "december"];

/// A date a document wrote in English ("December 30, 2021", "30 Dec
/// 2021") written in the language of the pages, with the months and the
/// form of the vocabulary (`pages.words.months`, `dateFormat`). Quotes are
/// never passed through this: they stay as the document writes them
/// (D-239).
pub(crate) fn localized_dates(vocabulary: &Vocabulary, text: &str) -> String {
    let words = &vocabulary.pages;
    let months: Vec<&str> = words.word("months").split(',').map(str::trim).filter(|m| !m.is_empty()).collect();
    let format = words.word("dateFormat");
    if months.len() != 12 || !format.contains("{month}") {
        return text.to_string();
    }
    let month_of = |word: &str| -> Option<usize> {
        let w = word.trim_end_matches('.').to_lowercase();
        if w.len() < 3 {
            return None;
        }
        ENGLISH_MONTHS.iter().position(|m| *m == w || (w.len() == 3 && m.starts_with(&w)) || (w == "sept" && *m == "september"))
    };
    let tokens: Vec<&str> = text.split(' ').collect();
    let day = |t: &str| -> Option<u32> {
        // "1st", "1°", "1º" (D-283).
        let t = t.trim_end_matches(',').trim_end_matches(['°', 'º']).trim_end_matches("st").trim_end_matches("nd").trim_end_matches("rd").trim_end_matches("th");
        t.parse::<u32>().ok().filter(|d| (1..=31).contains(d))
    };
    let year = |t: &str| -> Option<(String, String)> {
        let digits: String = t.chars().take_while(|c| c.is_ascii_digit()).collect();
        (digits.len() == 4).then(|| (digits.clone(), t[digits.len()..].to_string()))
    };
    let render = |d: u32, m: usize, y: &str| format.replace("{day}", &d.to_string()).replace("{month}", months[m]).replace("{year}", y);
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    let own_month = |word: &str| months.iter().position(|m| quote::words(m) == quote::words(word));
    while i < tokens.len() {
        // "23 días del mes de Octubre de 2025", as deeds write it (D-283).
        if i + 7 < tokens.len() {
            let filler = quote::words(&tokens[i + 1..i + 5].join(" ")) == ["dias", "del", "mes", "de"] && quote::words(tokens[i + 6]) == ["de"];
            if let (true, Some(d), Some(m), Some((y, rest))) = (filler, day(tokens[i]), own_month(tokens[i + 5]), year(tokens[i + 7])) {
                out.push(format!("{}{rest}", render(d, m, &y)));
                i += 8;
                continue;
            }
        }
        // "December 30, 2021", also in the brackets of a form ("[JANUARY 1°, 2026]").
        if i + 2 < tokens.len() {
            let open: String = tokens[i].chars().take_while(|c| matches!(c, '[' | '(')).collect();
            if let (Some(m), Some(d), Some((y, rest))) = (month_of(&tokens[i][open.len()..]), day(tokens[i + 1]), year(tokens[i + 2])) {
                if tokens[i + 1].ends_with(',') || tokens[i + 1].chars().all(|c| c.is_ascii_alphanumeric()) {
                    out.push(format!("{open}{}{rest}", render(d, m, &y)));
                    i += 3;
                    continue;
                }
            }
            // "30 December 2021"
            if let (Some(d), Some(m), Some((y, rest))) = (day(tokens[i]), month_of(tokens[i + 1]), year(tokens[i + 2])) {
                if !tokens[i].ends_with(',') {
                    out.push(format!("{}{rest}", render(d, m, &y)));
                    i += 3;
                    continue;
                }
            }
        }
        out.push(tokens[i].to_string());
        i += 1;
    }
    out.join(" ")
}

/// A quote on one line, as a list item needs it.
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A date written with numbers and separators ("16.04.2026", "16/04/2026",
/// "16-04-2026", "2026-04-16", "18/octubre/2024"), in the words of the
/// pages; the day first unless it cannot be.
fn separated_date(vocabulary: &Vocabulary, token: &str) -> Option<String> {
    let words = &vocabulary.pages;
    let months: Vec<&str> = words.word("months").split(',').map(str::trim).filter(|m| !m.is_empty()).collect();
    let format = words.word("dateFormat");
    if months.len() != 12 || !format.contains("{month}") {
        return None;
    }
    let parts: Vec<&str> = token.split(['.', '/', '-']).map(str::trim).collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let digits = |p: &str| p.chars().all(|c| c.is_ascii_digit());
    let number = |p: &str| p.parse::<u32>().unwrap_or(0);
    // A month in words: the pages' or English, whole or its first three letters.
    let month_word = |p: &str| -> Option<u32> {
        let w = quote::words(p).join(" ");
        let own = months.iter().position(|m| quote::words(m).join(" ") == w);
        let english = ENGLISH_MONTHS.iter().position(|m| *m == w || (w.len() == 3 && m.starts_with(&w)));
        own.or(english).map(|i| i as u32 + 1)
    };
    let (day, month, year) = if parts[0].len() == 4 && digits(parts[0]) {
        (parts[2], parts[1], parts[0])
    } else {
        (parts[0], parts[1], parts[2])
    };
    if !digits(day) || !digits(year) || year.len() != 4 {
        return None;
    }
    let mut month_number = if digits(month) { number(month) } else { month_word(month)? };
    let mut day_number = number(day);
    if month_number > 12 && day_number <= 12 {
        std::mem::swap(&mut day_number, &mut month_number);
    }
    ((1..=31).contains(&day_number) && (1..=12).contains(&month_number))
        .then(|| format.replace("{day}", &day_number.to_string()).replace("{month}", months[month_number as usize - 1]).replace("{year}", year))
}

/// Whether a word is a time of day ("17:06", "17:06:26").
fn is_time(word: &str) -> bool {
    word.contains(':') && word.chars().all(|c| c.is_ascii_digit() || c == ':')
}

/// A date as the pages write it (`dateFormat`, "16 de abril de 2026"),
/// from the ways documents and readings write one: with numbers and
/// separators (and a time after it, which stays), in English, or in
/// capitals. Anything else stays as written.
pub(crate) fn spelled_date(vocabulary: &Vocabulary, text: &str) -> String {
    let text = one_line(text);
    let (head, tail) = match text.split_once(' ') {
        Some((head, tail)) => (head, Some(tail)),
        None => (text.as_str(), None),
    };
    if let Some(date) = separated_date(vocabulary, head) {
        match tail {
            None => return date,
            Some(time) if time.split(' ').all(is_time) => return format!("{date} {time}"),
            Some(_) => {}
        }
    }
    let local = localized_dates(vocabulary, &text);
    // "12 DE JUNIO DE 2026", as a form writes it.
    if local.chars().any(char::is_uppercase) && !local.chars().any(char::is_lowercase) {
        return local.to_lowercase();
    }
    local
}

/// The date a title says (P-182): the spelled date without its time, when
/// it starts with its day or its month; none for one written otherwise
/// ("de octubre de 2024", what a reading kept of a longer date).
pub(crate) fn title_date(vocabulary: &Vocabulary, text: &str) -> Option<String> {
    let spelled = spelled_date(vocabulary, text);
    let words: Vec<&str> = spelled.split(' ').filter(|w| !is_time(w)).collect();
    let first = quote::words(words.first()?).join(" ");
    let months: Vec<String> = vocabulary.pages.word("months").split(',').map(|m| quote::words(m).join(" ")).collect();
    let starts = first.chars().next().is_some_and(|c| c.is_ascii_digit()) || months.contains(&first);
    starts.then(|| words.join(" "))
}

/// The numbered notes of a page (D-441): the quotes its text rests on, each
/// with its document, are notes `[^n]` where the text says them, written
/// once at the end of the page (`[^n]: …`, the way markdown writes notes).
/// The article shows the number and, when it is touched, the note; the
/// same note is numbered once, in the order the text first names it.
#[derive(Debug, Default)]
pub(crate) struct Notes {
    list: Vec<String>,
}

impl Notes {
    /// The mark of a note where the text says it.
    pub(crate) fn mark(&mut self, text: &str) -> String {
        let text = one_line(text);
        if text.is_empty() {
            return String::new();
        }
        let number = match self.list.iter().position(|t| *t == text) {
            Some(index) => index + 1,
            None => {
                self.list.push(text);
                self.list.len()
            }
        };
        format!("[^{number}]")
    }

    /// The notes, for the end of the page; nothing when there is none.
    pub(crate) fn block(&self) -> String {
        if self.list.is_empty() {
            return String::new();
        }
        let mut out: String = self.list.iter().enumerate().map(|(i, text)| format!("[^{}]: {text}\n", i + 1)).collect();
        out.push('\n');
        out
    }
}

/// Whether an entity has a page, by the rule of its type (D-196): it
/// appears in enough documents, has a role of the list in one of them, or
/// (if the rule says so) a document gives it a strong identifier.
fn has_page(vocabulary: &Vocabulary, entity: &Entity) -> bool {
    if entity.state != "active" || entity.documents.is_empty() || !vocabulary.is_a_name(&entity.kind, &entity.name) {
        return false;
    }
    // A group of companies is the page that ties them (D-260).
    if !entity.members.is_empty() {
        return true;
    }
    let Some(rule) = vocabulary.pages.rule(&entity.kind) else { return false };
    if rule.by_matter {
        return false;
    }
    if !rule.roles_required.is_empty()
        && !entity.documents.values().flatten().any(|role| rule.roles_required.iter().any(|r| r == role))
    {
        return false;
    }
    if rule.min_documents > 0 && entity.documents.len() >= rule.min_documents {
        return true;
    }
    if entity.documents.values().flatten().any(|role| rule.roles.iter().any(|r| r == role)) {
        return true;
    }
    // What the documents say of it is reason enough (D-266), but not its
    // contact data alone: a list of telephones and addresses of offices in
    // a form names no one the archive is about (D-284), nor does the
    // address of each office of a list of jurisdictions (D-288).
    let substantive =
        |c: &&Claim| !c.mirrored && vocabulary.predicate(&c.predicate).is_none_or(|p| p.shape != "contact" && p.shape != "address");
    if rule.with_claims && entity.claims.iter().any(|c| substantive(&c)) {
        return true;
    }
    rule.strong_identifier && entity.has_strong_identifier(vocabulary)
}

fn load(conn: &Connection, identities: &HashMap<i64, String>) -> Result<Record, String> {
    let mut entities: BTreeMap<String, Entity> = BTreeMap::new();
    {
        let mut statement = conn.prepare("SELECT id, kind, name, state, merged_into FROM knowledge_entity").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok(Entity {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    name: row.get(2)?,
                    state: row.get(3)?,
                    merged_into: row.get(4)?,
                    ..Default::default()
                })
            })
            .map_err(sql_err)?;
        for row in rows {
            let entity = row.map_err(sql_err)?;
            entities.insert(entity.id.clone(), entity);
        }
    }
    // A document joined with another as the same document, with the other
    // one chosen as the wiki's source, is listed as the chosen one (D-207).
    let chosen = passed_over(conn)?;
    let fold = |content: i64| chosen.get(&content).copied().unwrap_or(content);
    // Only documents that still have a file somewhere count.
    let (visible, params) = super::visible_content("m.content_id", &Visible::All);
    {
        let mut statement = conn
            .prepare(&format!(
                "SELECT m.entity_id, m.content_id, m.role, m.written, m.defined FROM knowledge_mention m
                 WHERE m.entity_id IS NOT NULL AND {visible}"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, bool>(4)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (entity, content, role, written, defined) = row.map_err(sql_err)?;
            if let Some(entity) = entities.get_mut(&entity) {
                entity.documents.entry(fold(content)).or_default().insert(role);
                // A term a document defines ("la Empresa") is a local alias:
                // it is neither a name nor an alias of the entity (D-245).
                if !defined {
                    *entity.written.entry(written).or_default() += 1;
                }
            }
        }
    }
    {
        let mut statement = conn
            .prepare(&format!(
                "SELECT m.entity_id, i.kind, i.value, i.normalized, m.content_id FROM knowledge_identifier i
                 JOIN knowledge_mention m ON m.id = i.mention_id WHERE m.entity_id IS NOT NULL AND {visible}"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (entity, kind, value, normalized, content) = row.map_err(sql_err)?;
            // A mail address is a contact, not an identifier (D-289).
            if value.contains('@') {
                continue;
            }
            if let Some(entity) = entities.get_mut(&entity) {
                entity.identifiers.entry((kind, normalized)).or_insert_with(|| (value, BTreeSet::new())).1.insert(fold(content));
            }
        }
        for entity in entities.values_mut() {
            one_value_once(&mut entity.identifiers, vocabulary().identifiers.last().map(|t| t.id.as_str()).unwrap_or_default());
        }
    }
    {
        let mut evidence: HashMap<i64, Vec<(i64, String, Option<i64>)>> = HashMap::new();
        let (visible_e, params_e) = super::visible_content("e.content_id", &Visible::All);
        let mut statement = conn
            .prepare(&format!(
                "SELECT e.claim_id, e.content_id, e.quote, e.page FROM knowledge_evidence e
                 WHERE e.claim_id IS NOT NULL AND {visible_e} ORDER BY e.content_id, e.id"
            ))
            .map_err(sql_err)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params_e.iter()), |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<i64>>(3)?))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (claim, content, quote, page) = row.map_err(sql_err)?;
            evidence.entry(claim).or_default().push((content, quote, page));
        }
        let mut statement = conn
            .prepare(
                "SELECT id, subject, predicate, object_entity, object_value, state, valid_from, context_entity, qualifier, amount, detail,
                        certainty
                 FROM knowledge_claim
                 WHERE state != 'retired' ORDER BY subject, predicate, coalesce(valid_from, ''), coalesce(object_normalized, object_entity, ''), id",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    Claim {
                        id: row.get(0)?,
                        predicate: row.get(2)?,
                        object_entity: row.get(3)?,
                        object_value: row.get(4)?,
                        state: row.get(5)?,
                        valid_from: row.get(6)?,
                        context: row.get(7)?,
                        qualifier: row.get(8)?,
                        evidence: Vec::new(),
                        amount: row.get(9)?,
                        detail: row.get::<_, Option<String>>(10)?.and_then(|d| serde_json::from_str(&d).ok()),
                        mirrored: false,
                        certainty: String::new(),
                        reasons: Vec::new(),
                    },
                ))
            })
            .map_err(sql_err)?;
        let mut certainties: HashMap<i64, Option<String>> = HashMap::new();
        {
            let mut statement = conn.prepare("SELECT id, certainty FROM knowledge_claim WHERE state != 'retired'").map_err(sql_err)?;
            let found = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<String>>(1)?))).map_err(sql_err)?;
            for row in found {
                let (id, certainty) = row.map_err(sql_err)?;
                certainties.insert(id, certainty);
            }
        }
        for row in rows {
            let (subject, mut claim) = row.map_err(sql_err)?;
            (claim.certainty, claim.reasons) = certainty_of(certainties.get(&claim.id).cloned().flatten().as_deref());
            claim.evidence = evidence.remove(&claim.id).unwrap_or_default();
            if claim.evidence.is_empty() {
                continue;
            }
            // An amount between two parties is told on both pages, each
            // from its side ("Debe a", "Le debe").
            if claim.amount.is_some() {
                if let Some(creditor) = claim.object_entity.clone().filter(|c| entities.contains_key(c)) {
                    let mut mirror = claim.clone();
                    mirror.object_entity = Some(subject.clone());
                    mirror.mirrored = true;
                    entities.get_mut(&creditor).expect("listed").claims.push(mirror);
                }
            }
            if let Some(entity) = entities.get_mut(&subject) {
                entity.claims.push(claim);
            }
        }
    }
    // Groups of companies (D-260).
    {
        let mut statement = conn.prepare("SELECT group_id, member_id, reasons FROM knowledge_group ORDER BY group_id, member_id").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (group, member, reasons) = row.map_err(sql_err)?;
            let active = |id: &str| entities.get(id).is_some_and(|e| e.state == "active");
            if !active(&group) || !active(&member) {
                continue;
            }
            let reasons: Vec<String> = serde_json::from_str(&reasons).unwrap_or_default();
            entities.get_mut(&group).expect("active").members.push((member.clone(), reasons));
            entities.get_mut(&member).expect("active").groups.push(group);
        }
    }
    // A name the record took from a mail address or a number (contact data
    // read as a mention), or from a generic word, gives way to the most
    // used way of writing that is a name, when there is one.
    let words = vocabulary();
    for entity in entities.values_mut() {
        if !words.is_a_name(&entity.kind, &entity.name) {
            let mut written: Vec<(&String, &usize)> = entity.written.iter().filter(|(w, _)| words.is_a_name(&entity.kind, w)).collect();
            written.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            if let Some((name, _)) = written.first() {
                entity.name = one_line(name);
            }
        }
        // On one line, and a name in capitals as the documents write it
        // otherwise (D-239).
        entity.name = super::names::shown_name(&entity.name, entity.written.keys().map(String::as_str));
    }
    let mut matters: BTreeMap<String, Matter> = BTreeMap::new();
    let mut matter_of: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    {
        let mut statement = conn
            .prepare(
                "SELECT m.id, m.name, d.content_id, d.reasons FROM knowledge_matter m JOIN knowledge_matter_document d ON d.matter_id = m.id
                 WHERE m.state = 'active' AND d.state = 'firm' ORDER BY m.id, d.content_id",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?, row.get::<_, String>(3)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (id, name, content, reasons) = row.map_err(sql_err)?;
            let content = fold(content);
            let matter = matters.entry(id.clone()).or_insert_with(|| Matter { id: id.clone(), name: one_line(&name), ..Default::default() });
            let mut reasons = serde_json::from_str::<Vec<super::matters::Reason>>(&reasons).unwrap_or_default();
            // A shared reference as the documents write it, never its
            // normalized form (D-377, D-381).
            for reason in reasons.iter_mut().filter(|r| r.kind == "asunto" && !entities.contains_key(&r.value)) {
                if let Some(written) = super::references::written_form(conn, vocabulary(), &reason.value)? {
                    reason.value = written;
                }
            }
            matter.documents.entry(content).or_default().extend(reasons);
            matter_of.entry(content).or_default().insert(id);
        }
    }
    let mut documents: BTreeMap<i64, Document> = BTreeMap::new();
    let mut contents: BTreeSet<i64> = entities.values().flat_map(|e| e.documents.keys().copied()).collect();
    contents.extend(matters.values().flat_map(|m| m.documents.keys().copied()));
    contents.extend(entities.values().flat_map(|e| e.claims.iter().flat_map(|c| c.evidence.iter().map(|(content, ..)| *content))));
    for content in &contents {
        documents.insert(
            *content,
            Document {
                name: classify::name_of(conn, *content)?,
                identity: identities.get(content).cloned(),
                chosen: chosen.get(content).copied(),
                ..Default::default()
            },
        );
    }
    {
        let mut statement = conn.prepare("SELECT content_id, doc_type FROM document_type").map_err(sql_err)?;
        let rows = statement.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))).map_err(sql_err)?;
        for row in rows {
            let (content, doc_type) = row.map_err(sql_err)?;
            if let Some(document) = documents.get_mut(&content) {
                document.doc_type = Some(doc_type);
            }
        }
    }
    let mut pages = BTreeMap::new();
    {
        let mut statement =
            conn.prepare("SELECT entity_id, path, state, fingerprint, body IS NOT NULL FROM knowledge_page").map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    PageRow { path: row.get(1)?, state: row.get(2)?, fingerprint: row.get(3)?, has_body: row.get(4)? },
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (id, page) = row.map_err(sql_err)?;
            pages.insert(id, page);
        }
    }
    let proposed: BTreeMap<i64, Vec<(String, Option<String>)>> = super::matters::proposals(conn)?
        .into_iter()
        .map(|(content, list)| (fold(content), list))
        .filter(|(content, _)| !matter_of.contains_key(content))
        .collect();
    for content in proposed.keys() {
        if !documents.contains_key(content) {
            documents.insert(
                *content,
                Document {
                    name: classify::name_of(conn, *content)?,
                    identity: identities.get(content).cloned(),
                    chosen: chosen.get(content).copied(),
                    ..Default::default()
                },
            );
        }
    }
    Ok(Record { entities, documents, pages, matters, matter_of, proposed })
}

/// The language the wiki is written in, as the wiki said it; none when it
/// did not say.
pub(crate) fn wiki_language(conn: &Connection) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [WIKI_LANGUAGE_SETTING], |row| row.get::<_, String>(0))
        .optional()
        .map_err(sql_err)
        .map(|v| v.filter(|l| !l.trim().is_empty() && l != "auto"))
}

fn setting(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM docbase_setting WHERE key = ?1", [key], |row| row.get(0)).optional().map_err(sql_err)
}

fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute("INSERT OR REPLACE INTO docbase_setting (key, value) VALUES (?1, ?2)", params![key, value]).map_err(sql_err)?;
    Ok(())
}

/// The claims the synthesis is written from: the first `MAX_CLAIMS` in
/// their order. The rest are still listed on the page (D-236).
fn synthesized(claims: &[Claim]) -> &[Claim] {
    &claims[..claims.len().min(MAX_CLAIMS)]
}

/// The claims of an entity in the order they are numbered: all of them.
fn numbered(vocabulary: &Vocabulary, entity: &Entity) -> Vec<Claim> {
    let order = |predicate: &str| vocabulary.predicates.iter().position(|p| p.id == predicate).unwrap_or(usize::MAX);
    let mut claims = entity.claims.clone();
    claims.sort_by(|a, b| {
        order(&a.predicate)
            .cmp(&order(&b.predicate))
            .then_with(|| a.valid_from.cmp(&b.valid_from))
            .then_with(|| a.object_value.cmp(&b.object_value))
            .then_with(|| a.object_entity.cmp(&b.object_entity))
            .then_with(|| a.context.cmp(&b.context))
            // Two amounts between the same parties: by their amount and kind,
            // never by when their claims were written (R-13).
            .then_with(|| a.mirrored.cmp(&b.mirrored))
            .then_with(|| a.qualifier.cmp(&b.qualifier))
            .then_with(|| a.amount.cmp(&b.amount))
    });
    claims
}

/// How a claim is named on a page: the label of its predicate or, for an
/// amount between two parties, how it is told from this side, with the
/// other party ("Debe a Ejemplo Uno S.A.", "Le reclama Persona Dos").
fn claim_label(vocabulary: &Vocabulary, record: &Record, claim: &Claim) -> String {
    if claim.amount.is_none() {
        return vocabulary.predicate_label(&claim.predicate).to_string();
    }
    let kind = claim.qualifier.as_deref().and_then(|k| vocabulary.money_kind(k)).or_else(|| vocabulary.money.kinds.last());
    let phrase = kind.map(|k| if claim.mirrored { k.creditor.as_str() } else { k.debtor.as_str() }).unwrap_or_default();
    let other = claim.object_entity.as_ref().and_then(|id| record.entities.get(id)).map(|e| e.name.clone()).unwrap_or_default();
    format!("{phrase} {other}").trim().to_string()
}

/// What an amount between two parties says besides its parties: the
/// amount, its state (paid, due, conditioned…: P-136), its kind and its
/// concept.
fn amount_text(vocabulary: &Vocabulary, claim: &Claim) -> String {
    let amount = one_line(claim.amount.as_deref().unwrap_or_default());
    let mut notes: Vec<String> = Vec::new();
    let detail = claim.detail.as_ref();
    let states: Vec<String> = match detail.and_then(|d| d.get("estado")).and_then(|s| s.as_str()) {
        Some(one) => vec![one.to_string()],
        None => detail
            .and_then(|d| d.get("estados"))
            .and_then(|s| s.as_array())
            .map(|list| list.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
            .unwrap_or_default(),
    };
    let labels: Vec<String> = states
        .iter()
        .map(|s| vocabulary.money_state(s).map(|m| if m.label.is_empty() { m.id.clone() } else { m.label.clone() }).unwrap_or_else(|| s.clone()))
        .collect();
    if !labels.is_empty() {
        // Several states: each document says one (due in one, paid later).
        notes.push(labels.join(&format!(" {} ", vocabulary.pages.word("or"))));
    }
    if let Some(kind) = claim.qualifier.as_deref().and_then(|k| vocabulary.money_kind(k)) {
        if !kind.label.is_empty() {
            notes.push(kind.label.clone());
        }
    }
    if let Some(concept) = claim.detail.as_ref().and_then(|d| d.get("concepto")).and_then(|c| c.as_str()) {
        // A concept that only repeats the kind ("intereses; intereses") is
        // said once (D-283).
        let concept = one_line(concept);
        if !notes.iter().any(|n| quote::words(n) == quote::words(&concept)) {
            notes.push(concept);
        }
    }
    if notes.is_empty() {
        amount
    } else {
        format!("{amount} ({})", notes.join("; "))
    }
}

fn object_text(record: &Record, claim: &Claim) -> String {
    if claim.amount.is_some() {
        return amount_text(vocabulary(), claim);
    }
    let text = match (&claim.object_value, &claim.object_entity) {
        (Some(value), _) => one_line(value),
        (None, Some(id)) => record.entities.get(id).map(|e| e.name.clone()).unwrap_or_else(|| id.clone()),
        (None, None) => String::new(),
    };
    let vocabulary = vocabulary();
    let mut text = text;
    if let Some(qualifier) = claim.qualifier.as_deref() {
        let label = vocabulary
            .predicate(&claim.predicate)
            .and_then(|p| p.qualifiers.iter().find(|q| q.id == qualifier))
            .map(|q| if q.label.is_empty() { q.id.as_str() } else { q.label.as_str() })
            .unwrap_or(qualifier);
        text = format!("{text} ({label})");
    }
    match claim.context.as_ref().and_then(|c| record.entities.get(c)) {
        Some(within) => format!("{text} ({} {})", vocabulary.pages.word("within"), within.name),
        None => text,
    }
}

/// The question that writes what the documents say of an entity.
fn synthesis_question(vocabulary: &Vocabulary, record: &Record, entity: &Entity, claims: &[Claim], language: Option<&str>) -> Question {
    let kind = vocabulary.type_label(&entity.kind);
    let mut listed = Vec::new();
    for (index, claim) in claims.iter().enumerate() {
        let mut line = format!("[{}] {}: {}", index + 1, claim_label(vocabulary, record, claim), object_text(record, claim));
        if let Some(from) = &claim.valid_from {
            line.push_str(&format!(" (tiempo: {from})"));
        }
        if claim.state == "conflict" {
            line.push_str(" — EN CONFLICTO con otra afirmación del mismo dato");
        }
        if claim.doubtful_beyond_conflict() {
            let why: Vec<&str> = claim
                .reasons
                .iter()
                .filter_map(|r| match r.as_str() {
                    "identity" => Some("no es seguro de quién se habla"),
                    "attribution" => Some("la cita nombra a otra persona u organización"),
                    "states" => Some("los documentos dicen estados distintos"),
                    "contradicted" => Some("el mismo documento dice un estado civil que no corresponde"),
                    _ => None,
                })
                .collect();
            line.push_str(&format!(" — DUDOSA ({})", why.join("; ")));
        }
        let documents: BTreeSet<i64> =
            claim.evidence.iter().map(|(c, ..)| record.documents.get(c).and_then(|d| d.chosen).unwrap_or(*c)).collect();
        line.push_str(&format!(" — en {} documento(s)", documents.len()));
        for (content, quote, _) in claim.evidence.iter().take(QUOTES_PER_CLAIM) {
            let name = record.documents.get(content).map(|d| d.name.as_str()).unwrap_or_default();
            line.push_str(&format!("\n    Cita: «{}» ({name})", one_line(quote)));
        }
        listed.push(line);
    }
    let language = match language {
        Some(language) => format!("Escribí en este idioma: {language}."),
        None => "Escribí en el idioma en que están escritas las citas.".to_string(),
    };
    // Only a question with a doubtful claim says how to write it: the
    // others keep their question, and their answer (D-322).
    let doubtful = if claims.iter().any(Claim::doubtful_beyond_conflict) {
        "- Las afirmaciones marcadas DUDOSA no son hechos firmes: escribilas como dudosas («según un documento…, sin confirmar»), \
en oraciones aparte de los hechos firmes, y nunca como algo seguro.\n"
    } else {
        ""
    };
    let prompt = format!(
        "Sos asistente de un estudio jurídico. Redactá lo que dicen los documentos del archivo sobre {kind} «{name}», \
para su ficha en el wiki del estudio. Usá solo las afirmaciones numeradas de abajo: cada una ya fue comprobada contra el \
texto de su documento.\n\n\
Reglas:\n\
- No agregues nada que no esté en las afirmaciones: ni conocimiento general, ni suposiciones, ni conclusiones que combinen \
dos afirmaciones (si una persona representa a una sociedad y la sociedad firmó un contrato, no digas que la persona lo firmó).\n\
- Oraciones breves y concretas, como las escribiría un abogado: partes, roles, cargos, domicilios, importes, fechas, \
obligaciones y plazos. Agrupá lo que se relaciona; no repitas.\n\
- Cada oración lleva los números de las afirmaciones que la respaldan (al menos uno). No escribas los números dentro del texto.\n\
- Copiá importes, fechas, números y nombres tal como están en las afirmaciones.\n\
- Si una afirmación dice un tiempo, decilo. Si las afirmaciones de un mismo dato dicen tiempos distintos, contalo como una historia \
(primero un valor, después otro). Datos de clases distintas (un domicilio real y uno constituido, un domicilio laboral y uno de la \
sociedad) no se contradicen: decí cada uno con su clase.\n\
- Un importe dice quién debe, paga o reclama a quién, con el tipo de importe: decilo así («debe a…», «reclama a…», «pagó a…»), \
nunca como un monto suelto sin la otra parte. Si dice su estado (pagado, a pagar, pactado, propuesto, condicionado), decilo con \
él: no digas que se pagó lo que no dice «pagado», ni que se debe lo que dice «pactado» o «propuesto».\n\
- No digas que los documentos difieren, se contradicen o no coinciden, salvo para las afirmaciones marcadas EN CONFLICTO: esas no \
se resuelven; nombrá cada valor con su número.\n\
{doubtful}\
- {language}\n\n\
{kind}: {name}\n\
Afirmaciones:\n{claims}\n\n\
Respondé solo con el JSON pedido: {{\"oraciones\": [{{\"texto\": \"...\", \"afirmaciones\": [1, 2]}}]}}.",
        name = entity.name,
        claims = listed.join("\n"),
    );
    Question {
        key: classify::question_key(SYNTHESIS_PROMPT, &prompt),
        version: SYNTHESIS_PROMPT,
        task: TASK,
        content_id: 0,
        prompt,
        schema: schema(),
    }
}

pub fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["oraciones"],
        "properties": {
            "oraciones": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["texto", "afirmaciones"],
                    "properties": {
                        "texto": { "type": "string" },
                        "afirmaciones": { "type": "array", "items": { "type": "integer" } }
                    }
                }
            }
        }
    })
}

/// The sentences of an answer the check keeps, each with the claims
/// (indexes into `claims`) that back it; and how many it threw away.
fn checked(record: &Record, entity: &Entity, claims: &[Claim], answer: &str) -> (Vec<(String, Vec<usize>)>, usize) {
    let Ok(answer) = parse_json::<Answer>(answer) else { return (Vec::new(), 1) };
    let mut kept = Vec::new();
    let mut discarded = 0;
    for sentence in answer.oraciones {
        let text = without_markers(&sentence.texto);
        let mut refs: Vec<usize> = Vec::new();
        let mut bad = text.is_empty() || sentence.afirmaciones.is_empty();
        for n in &sentence.afirmaciones {
            match usize::try_from(*n).ok().filter(|n| *n >= 1 && *n <= claims.len()) {
                Some(n) if !refs.contains(&(n - 1)) => refs.push(n - 1),
                Some(_) => {}
                None => bad = true,
            }
        }
        if !bad {
            // Every number of the sentence is in one of its claims.
            let mut backed = digit_runs(&entity.name);
            for &r in &refs {
                let claim = &claims[r];
                backed.extend(digit_runs(&object_text(record, claim)));
                backed.extend(digit_runs(claim.valid_from.as_deref().unwrap_or_default()));
                for (content, quote, _) in &claim.evidence {
                    backed.extend(digit_runs(quote));
                    // A sentence may name the document a claim comes from
                    // ("según «Carta 26-06-26.pdf»", D-322).
                    if let Some(document) = record.documents.get(content) {
                        backed.extend(digit_runs(&document.name));
                    }
                }
            }
            bad = digit_runs(&text).iter().any(|run| !backed.contains(run));
        }
        if bad {
            discarded += 1;
        } else {
            refs.sort_unstable();
            kept.push((text, refs));
        }
    }
    (kept, discarded)
}

/// What the page of an entity says, but its notes; and what the
/// synthesis left out.
struct Rendered {
    text: String,
    synthesis: Option<String>,
    sentences: usize,
    discarded: usize,
}

fn document_link(record: &Record, content: i64) -> String {
    match record.documents.get(&content) {
        // A copy joined with the chosen one: its own name (the quote is in
        // its text), and the one the wiki reads.
        Some(Document { chosen: Some(chosen), name, .. }) if *chosen != content => {
            format!("{name} → {}", document_link(record, *chosen))
        }
        Some(Document { page: Some(page), name, .. }) => link(page, name),
        Some(document) => document.name.clone(),
        None => format!("#{content}"),
    }
}

/// The documents joined with another one as the same document, with the
/// other one chosen as the wiki's source: each with the one chosen in its
/// place, following a chain.
pub(crate) fn passed_over(conn: &Connection) -> Result<HashMap<i64, i64>, String> {
    let mut statement = conn
        .prepare("SELECT a, b, wiki_source FROM similar_pair WHERE state = 'joined' AND wiki_source IS NOT NULL")
        .map_err(sql_err)?;
    let rows: Vec<(i64, i64, i64)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    let mut next: HashMap<i64, i64> = HashMap::new();
    for (a, b, source) in rows {
        let other = if source == a { b } else { a };
        if other != source {
            next.insert(other, source);
        }
    }
    let mut out = HashMap::new();
    for &start in next.keys() {
        let mut at = start;
        for _ in 0..16 {
            match next.get(&at) {
                Some(&n) if n != start => at = n,
                _ => break,
            }
        }
        if at != start {
            out.insert(start, at);
        }
    }
    Ok(out)
}

/// The entities related to one: the ones with a page that share documents
/// or claims with it, the closest first.
fn related<'a>(record: &'a Record, paths: &BTreeMap<String, String>, entity: &Entity) -> Vec<(&'a Entity, usize)> {
    let mut score: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for other in record.entities.values() {
        if other.id == entity.id || !paths.contains_key(&other.id) {
            continue;
        }
        let shared = entity.documents.keys().filter(|c| other.documents.contains_key(c)).count();
        let claimed = entity.claims.iter().filter(|c| c.object_entity.as_deref() == Some(other.id.as_str())).count()
            + other.claims.iter().filter(|c| c.object_entity.as_deref() == Some(entity.id.as_str())).count();
        if shared + claimed > 0 {
            score.insert(other.id.as_str(), (claimed, shared));
        }
    }
    let mut list: Vec<(&Entity, (usize, usize))> =
        score.into_iter().filter_map(|(id, s)| record.entities.get(id).map(|e| (e, s))).collect();
    list.sort_by(|a, b| (b.1 .0 * 2 + b.1 .1).cmp(&(a.1 .0 * 2 + a.1 .1)).then_with(|| a.0.name.cmp(&b.0.name)));
    list.truncate(MAX_RELATED);
    list.into_iter().map(|(e, (_, shared))| (e, shared)).collect()
}

fn documents_word<'a>(vocabulary: &'a Vocabulary, n: usize) -> &'a str {
    vocabulary.pages.word(if n == 1 { "documentWord" } else { "documentsWord" })
}

fn render(vocabulary: &Vocabulary, record: &Record, paths: &BTreeMap<String, String>, entity: &Entity, answer: Option<(&str, &str)>, claims: &[Claim]) -> Rendered {
    let words = &vocabulary.pages;
    let mut out = String::new();
    let aliases = entity.aliases();
    let mut sources: Vec<String> = entity.documents.keys().filter_map(|c| record.documents.get(c)?.identity.clone()).collect();
    sources.sort();
    sources.dedup();
    let related = related(record, paths, entity);
    out.push_str("---\ntype: entity\n");
    out.push_str(&format!("title: {}\n", yaml_string(&entity.name)));
    out.push_str(&format!("entity_kind: {}\n", entity.kind));
    out.push_str(&format!("entity_id: {}\n", entity.id));
    out.push_str(&format!("aliases: [{}]\n", aliases.iter().map(|a| yaml_string(a)).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("sources: [{}]\n", sources.iter().map(|s| yaml_string(s)).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!(
        "related: [{}]\n",
        related.iter().filter_map(|(e, _)| paths.get(&e.id)).map(|p| stem(p)).collect::<Vec<_>>().join(", ")
    ));
    out.push_str(&format!("generated_by: {GENERATED_BY}\n"));
    out.push_str("---\n\n");
    out.push_str(&format!("# {}\n\n", entity.name));
    let kind_label = if entity.members.is_empty() { vocabulary.type_label(&entity.kind) } else { words.word("groupKind") };
    let mut facts = vec![format!("*{kind_label}*")];
    if !aliases.is_empty() {
        facts.push(format!("{}: {}", words.word("aliases"), aliases.join(" · ")));
    }
    for ((kind, _), (value, documents)) in &entity.identifiers {
        facts.push(format!("{} {value} ({} {})", vocabulary.identifier_label(kind), documents.len(), documents_word(vocabulary, documents.len())));
    }
    out.push_str(&facts.join(" · "));
    out.push_str("\n\n");
    // The companies of a group, and the group of a company (D-260).
    let entity_link = |id: &str| -> Option<String> {
        let other = record.entities.get(id)?;
        Some(match paths.get(id) {
            Some(path) => link(path, &other.name),
            None => other.name.clone(),
        })
    };
    if !entity.members.is_empty() {
        let list: Vec<String> = entity
            .members
            .iter()
            .filter_map(|(member, reasons)| {
                let why: Vec<String> = reasons
                    .iter()
                    .filter_map(|r| words.words.get(&format!("groupWhy-{r}")).cloned())
                    .collect();
                let shown = entity_link(member)?;
                Some(if why.is_empty() { shown } else { format!("{shown} ({})", why.join(", ")) })
            })
            .collect();
        out.push_str(&format!("{}: {}\n\n", words.word("groupMembers"), list.join(" · ")));
    }
    if !entity.groups.is_empty() {
        let list: Vec<String> = entity.groups.iter().filter_map(|g| entity_link(g)).collect();
        out.push_str(&format!("{}: {}\n\n", words.word("groupOf"), list.join(" · ")));
    }

    // What the documents say. Each sentence carries its notes: the claims
    // it rests on, with their quotes and documents (D-441). What the
    // synthesis did not use is in «Datos y correcciones», with its quote
    // and its actions: the article says how many there are.
    let mut synthesis = None;
    let mut sentences = 0;
    let mut discarded = 0;
    let mut notes = Notes::default();
    out.push_str(&format!("## {}\n\n", words.word("says")));
    let mut used: BTreeSet<usize> = BTreeSet::new();
    // The note of a claim: what it says, how sure the record is of it, and
    // each quote with its document.
    let note_of = |index: usize| -> String {
        let claim = &claims[index];
        let quotes: Vec<String> = claim
            .evidence
            .iter()
            .map(|(content, quote, page)| {
                let at = page.map(|p| format!(", {} {p}", words.word("page"))).unwrap_or_default();
                format!("«{}» ({}{at})", one_line(quote), document_link(record, *content))
            })
            .collect();
        let time = claim.valid_from.as_deref().map(|t| format!(" ({})", localized_dates(vocabulary, &one_line(t)))).unwrap_or_default();
        format!(
            "{}: {}{time}{} — {}",
            claim_label(vocabulary, record, claim),
            localized_dates(vocabulary, &one_line(&object_text(record, claim))),
            certainty_mark(vocabulary, claim),
            quotes.join("; ")
        )
    };
    if claims.is_empty() {
        out.push_str(words.word("saysNothing"));
        out.push_str("\n\n");
    } else {
        match answer {
            Some((key, text)) => {
                synthesis = Some(key.to_string());
                let (kept, thrown) = checked(record, entity, synthesized(claims), text);
                discarded = thrown;
                sentences = kept.len();
                let unsure = words.word("certainDoubtful");
                let lines: Vec<String> = kept
                    .iter()
                    .map(|(text, refs)| {
                        used.extend(refs.iter().copied());
                        let marks: String = refs.iter().map(|r| notes.mark(&note_of(*r))).collect();
                        // Never as a firm fact (D-322): a sentence that rests
                        // on a doubtful claim is marked, unless it says so.
                        let doubtful = refs.iter().any(|r| claims[*r].doubtful_beyond_conflict());
                        let said = text.to_lowercase().contains(&unsure.to_lowercase());
                        let mark = if doubtful && !said { format!(" *({unsure})*") } else { String::new() };
                        format!("{}{mark}{marks}", localized_dates(vocabulary, text))
                    })
                    .collect();
                if lines.is_empty() {
                    out.push_str(words.word("saysPending"));
                } else {
                    out.push_str(&lines.join(" "));
                }
                out.push_str("\n\n");
            }
            None => {
                out.push_str(words.word("saysPending"));
                out.push_str("\n\n");
            }
        }
        out.push_str(&format!("*{}*\n\n", words.word("certainLegend")));
        // Conflicts are always in view, all their values.
        let mut conflicts: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
        for (index, claim) in claims.iter().enumerate() {
            if claim.state == "conflict" {
                conflicts.entry(claim.predicate.as_str()).or_default().push(index);
            }
        }
        if !conflicts.is_empty() {
            out.push_str(&format!("**{}**\n\n", words.word("conflicts")));
            for (predicate, indexes) in &conflicts {
                let values: Vec<String> = indexes
                    .iter()
                    .map(|i| format!("«{}»{}", localized_dates(vocabulary, &one_line(&object_text(record, &claims[*i]))), notes.mark(&note_of(*i))))
                    .collect();
                used.extend(indexes.iter().copied());
                out.push_str(&format!("- {}: {}\n", vocabulary.predicate_label(predicate), values.join(" · ")));
            }
            out.push('\n');
        }
        // What the synthesis did not use, left out or could not be asked
        // about: how many, and where they are (none goes missing, D-236).
        let unused = (0..claims.len()).filter(|i| !used.contains(i)).count();
        if !used.is_empty() && unused > 0 {
            let more = if unused == 1 { words.word("saysMoreOne").to_string() } else { words.word("saysMore").replace("{n}", &unused.to_string()) };
            out.push_str(&format!("{more}\n\n"));
        }
    }

    // Its documents, by matter.
    out.push_str(&format!("## {} ({})\n\n", words.word("documents"), entity.documents.len()));
    let line = |content: i64, roles: &BTreeSet<String>| {
        let roles: Vec<&str> = roles.iter().map(|r| vocabulary.role_label(r)).collect();
        let kind = record
            .documents
            .get(&content)
            .and_then(|d| d.doc_type.as_deref())
            .map(|t| format!(" · {}", t.replace('-', " ")))
            .unwrap_or_default();
        format!("- {} — {}{kind}\n", document_link(record, content), roles.join(", "))
    };
    if entity.kind == "asunto" && !vocabulary.pages.rule("asunto").is_some_and(|r| r.by_matter) {
        for (content, roles) in &entity.documents {
            out.push_str(&line(*content, roles));
        }
        out.push('\n');
    } else {
        // By the matters (groups of documents) with a page.
        let mut by_matter: BTreeMap<String, (String, Vec<i64>)> = BTreeMap::new();
        let mut alone = Vec::new();
        for content in entity.documents.keys() {
            let matters: Vec<&Matter> = record
                .matter_of
                .get(content)
                .into_iter()
                .flatten()
                .filter_map(|id| record.matters.get(id))
                .filter(|m| paths.contains_key(&m.id))
                .collect();
            let proposed: Vec<&Matter> = record
                .proposed
                .get(content)
                .into_iter()
                .flatten()
                .filter_map(|(id, _)| record.matters.get(id))
                .filter(|m| paths.contains_key(&m.id))
                .collect();
            if matters.is_empty() && proposed.is_empty() {
                alone.push(*content);
            }
            for matter in matters.into_iter().chain(proposed) {
                by_matter
                    .entry(matter.id.clone())
                    .or_insert_with(|| (link(&paths[&matter.id], &matter.name), Vec::new()))
                    .1
                    .push(*content);
            }
        }
        let mut groups: Vec<(String, Vec<i64>)> = by_matter.into_values().collect();
        groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));
        let several = !groups.is_empty();
        for (title, contents) in groups {
            out.push_str(&format!("### {title}\n\n"));
            for content in contents {
                let mut text = line(content, &entity.documents[&content]);
                if !record.matter_of.contains_key(&content) {
                    text = format!("{} ({})\n", text.trim_end(), words.word("proposed"));
                }
                out.push_str(&text);
            }
            out.push('\n');
        }
        if !alone.is_empty() {
            if several {
                out.push_str(&format!("### {}\n\n", words.word("withoutMatter")));
            }
            for content in alone {
                out.push_str(&line(content, &entity.documents[&content]));
            }
            out.push('\n');
        }
    }

    // Its relations with other entities, both ways (D-214): what the
    // documents say, each with its quote. Nothing is derived (R-02).
    // A claim and its inverse said from the other side ("deudor de" here,
    // "acreedor de" there) are one line, with the quotes of both (D-239).
    let mut ties: Vec<((String, String), Vec<String>)> = Vec::new();
    let mut tie = |label: &str, other: &str, quotes: Vec<String>| {
        let key = (label.to_string(), other.to_string());
        match ties.iter_mut().find(|(k, _)| *k == key) {
            Some((_, list)) => {
                for quote in quotes {
                    if !list.contains(&quote) {
                        list.push(quote);
                    }
                }
            }
            None => ties.push((key, quotes)),
        }
    };
    let name_of = |id: &str| -> String {
        match (record.entities.get(id), paths.get(id)) {
            (Some(e), Some(path)) => link(path, &e.name),
            (Some(e), None) => e.name.clone(),
            _ => id.to_string(),
        }
    };
    let quotes_of = |claim: &Claim| -> Vec<String> {
        claim
            .evidence
            .iter()
            .take(QUOTES_PER_CLAIM)
            .map(|(content, quote, _)| format!("«{}» ({})", one_line(quote), document_link(record, *content)))
            .collect()
    };
    for claim in &entity.claims {
        // An amount between two parties is in the claims of both pages.
        if claim.amount.is_some() {
            continue;
        }
        if let Some(object) = &claim.object_entity {
            if record.entities.get(object).is_some_and(|o| o.kind != "asunto") {
                let label = format!("{}{}", vocabulary.predicate_label(&claim.predicate), certainty_mark(vocabulary, claim));
                tie(&label, object, quotes_of(claim));
            }
        }
    }
    for other in record.entities.values().filter(|o| o.id != entity.id && o.state == "active") {
        for claim in other.claims.iter().filter(|c| c.amount.is_none() && c.object_entity.as_deref() == Some(entity.id.as_str())) {
            let Some(inverse) = vocabulary.predicate(&claim.predicate).map(|p| p.inverse_label.as_str()).filter(|l| !l.is_empty()) else { continue };
            if other.kind == "asunto" {
                continue;
            }
            let label = format!("{inverse}{}", certainty_mark(vocabulary, claim));
            tie(&label, &other.id, quotes_of(claim));
        }
    }
    if !ties.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("relations")));
        for ((label, other), quotes) in &ties {
            out.push_str(&format!("- {label} {}{}\n", name_of(other), notes.mark(&quotes.join("; "))));
        }
        out.push('\n');
    }

    if !related.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("related")));
        for (other, shared) in &related {
            let together =
                if *shared > 0 { format!(" — {shared} {}", words.word(if *shared == 1 { "inCommonOne" } else { "inCommon" })) } else { String::new() };
            out.push_str(&format!("- {} · {}{together}\n", link(&paths[&other.id], &other.name), vocabulary.type_label(&other.kind)));
        }
        out.push('\n');
    }
    out.push_str(&notes.block());
    Rendered { text: out, synthesis, sentences, discarded }
}

/// The notes of a person in a page of the old wiki: what follows the
/// notes mark, or the section of notes when the mark was taken out (only
/// the import of the old wiki reads them now, `import.rs`).
fn notes_of(text: &str, heading: &str) -> String {
    let text = text.replace("\r\n", "\n");
    if let Some(at) = text.find(NOTES_MARK) {
        let after = &text[at..];
        let line_end = after.find('\n').map(|n| n + 1).unwrap_or(after.len());
        return after[line_end..].trim_matches('\n').to_string();
    }
    let wanted = format!("## {heading}");
    if let Some(at) = text.lines().position(|l| l.trim() == wanted) {
        return text.lines().skip(at + 1).collect::<Vec<_>>().join("\n").trim_matches('\n').to_string();
    }
    String::new()
}

/// A page as one text, the way the wiki's file had it: what the app wrote,
/// then the section of notes with its mark and the notes of a person (for
/// the search and the chat, which read a page whole).
pub(crate) fn with_notes(vocabulary: &Vocabulary, generated: &str, notes: &str) -> String {
    let words = &vocabulary.pages;
    let mut out = generated.trim_end().to_string();
    out.push_str(&format!("\n\n## {}\n\n{NOTES_MARK}: {} -->\n", words.word("notes"), words.word("notesMark")));
    if !notes.trim().is_empty() {
        out.push_str(notes.trim_matches('\n'));
        out.push('\n');
    }
    out
}

/// Puts the date the page was written under its header.
fn dated(text: &str) -> String {
    text.replacen(&format!("generated_by: {GENERATED_BY}\n"), &format!("generated_by: {GENERATED_BY}\nupdated: {}\n", today()), 1)
}

/// The notes of a person on an object (`knowledge_note`); empty when none.
pub(crate) fn notes_of_object(conn: &Connection, id: &str) -> Result<String, String> {
    conn.query_row("SELECT text FROM knowledge_note WHERE object_id = ?1", [id], |row| row.get::<_, String>(0))
        .optional()
        .map_err(sql_err)
        .map(Option::unwrap_or_default)
}

/// Adds `text` to the notes of `id`, unless they already say it.
pub(crate) fn add_notes(conn: &Connection, id: &str, text: &str) -> Result<(), String> {
    let text = text.trim_matches('\n');
    if text.trim().is_empty() {
        return Ok(());
    }
    let before = notes_of_object(conn, id)?;
    if before.contains(text.trim()) {
        return Ok(());
    }
    let next = if before.trim().is_empty() { text.to_string() } else { format!("{}\n\n{text}", before.trim_end()) };
    conn.execute(
        "INSERT INTO knowledge_note (object_id, text, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT (object_id) DO UPDATE SET text = excluded.text, updated_at = excluded.updated_at",
        params![id, next, now_ms()],
    )
    .map_err(sql_err)?;
    Ok(())
}

/// The notes of a page that goes into another one (a version of a
/// document that joined its family's page): they go with it (D-340).
fn move_notes(conn: &Connection, from: &str, to: &str) -> Result<(), String> {
    let notes = notes_of_object(conn, from)?;
    if notes.trim().is_empty() {
        return Ok(());
    }
    add_notes(conn, to, &notes)?;
    conn.execute("DELETE FROM knowledge_note WHERE object_id = ?1", [from]).map_err(sql_err)?;
    Ok(())
}

/// The body of a page in the base, when it has one.
fn stored_body(conn: &Connection, id: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT body FROM knowledge_page WHERE entity_id = ?1", [id], |row| row.get::<_, Option<String>>(0))
        .optional()
        .map_err(sql_err)
        .map(Option::flatten)
}

/// The block of entities a document's page ends with.
fn source_block(vocabulary: &Vocabulary, record: &Record, paths: &BTreeMap<String, String>, contents: &[i64]) -> Option<String> {
    let order = |kind: &str| vocabulary.entity_types.iter().position(|t| t.id == kind).unwrap_or(usize::MAX);
    let roles_in: Vec<(&Entity, BTreeSet<String>)> = record
        .entities
        .values()
        .filter(|e| e.state == "active")
        .filter_map(|e| {
            let roles: BTreeSet<String> = contents.iter().filter_map(|c| e.documents.get(c)).flatten().cloned().collect();
            (!roles.is_empty()).then_some((e, roles))
        })
        .collect();
    let mut named: Vec<(&Entity, &BTreeSet<String>)> = roles_in
        .iter()
        .map(|(e, roles)| (*e, roles))
        // Who has no page and was only named in passing is left out, and
        // so is what is not a name (an address, a blank of a form).
        .filter(|(e, roles)| paths.contains_key(&e.id) || roles.iter().any(|r| r != vocabulary.fallback_role()))
        .filter(|(e, _)| vocabulary.is_a_name(&e.kind, &e.name))
        .collect();
    let mut matters: Vec<&Matter> = contents
        .iter()
        .filter_map(|c| record.matter_of.get(c))
        .flatten()
        .filter_map(|id| record.matters.get(id))
        .filter(|m| paths.contains_key(&m.id))
        .collect();
    matters.sort_by(|a, b| a.id.cmp(&b.id));
    matters.dedup_by(|a, b| a.id == b.id);
    let proposed: Vec<&(String, Option<String>)> = contents.iter().filter_map(|c| record.proposed.get(c)).flatten().collect();
    if named.is_empty() && matters.is_empty() && proposed.is_empty() {
        return None;
    }
    named.sort_by(|a, b| order(&a.0.kind).cmp(&order(&b.0.kind)).then_with(|| a.0.name.cmp(&b.0.name)));
    let words = &vocabulary.pages;
    let mut out = format!("{BLOCK_START}: {} -->\n## {}\n\n", words.word("sourceBlockMark"), words.word("sourceBlock"));
    for matter in matters {
        out.push_str(&format!("- {}: {}\n", words.word("matterOf"), link(&paths[&matter.id], &matter.name)));
    }
    let mut listed: BTreeSet<&str> = BTreeSet::new();
    for (id, _) in proposed {
        if !listed.insert(id.as_str()) {
            continue;
        }
        if let (Some(matter), Some(path)) = (record.matters.get(id), paths.get(id)) {
            out.push_str(&format!("- {} ({}): {}\n", words.word("matterOf"), words.word("proposed"), link(path, &matter.name)));
        }
    }
    for (entity, roles) in named {
        let name = match paths.get(&entity.id) {
            Some(path) => link(path, &entity.name),
            None => entity.name.clone(),
        };
        let roles: Vec<&str> = roles.iter().map(|r| vocabulary.role_label(r)).collect();
        out.push_str(&format!("- {name} · {} — {}\n", vocabulary.type_label(&entity.kind), roles.join(", ")));
    }
    out.push_str(BLOCK_END);
    out.push('\n');
    Some(out)
}

/// The names of the documents of a page that can be linked inside its text
/// without doubt (`links::safe_forms`), to the pages of their entities.
fn link_forms(
    base: &DocBase,
    vocabulary: &Vocabulary,
    record: &Record,
    paths: &BTreeMap<String, String>,
    contents: &[i64],
) -> Result<Vec<super::links::LinkForm>, String> {
    let list = contents.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(",");
    let conn = base.conn();
    let mut statement = conn
        .prepare(&format!(
            "SELECT entity_id, kind, written FROM knowledge_mention WHERE content_id IN ({list}) AND entity_id IS NOT NULL ORDER BY id"
        ))
        .map_err(sql_err)?;
    let rows: Vec<(String, String, String)> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .map_err(sql_err)?
        .collect::<Result<_, _>>()
        .map_err(sql_err)?;
    let mentions: Vec<(String, String, Option<String>)> = rows
        .into_iter()
        .filter(|(_, kind, written)| vocabulary.is_a_name(kind, written))
        .map(|(entity, _, written)| {
            let active = record.entities.get(&entity).is_some_and(|e| e.state == "active");
            let page = paths.get(&entity).filter(|_| active).map(|p| stem(p));
            (entity, written, page)
        })
        .collect();
    Ok(super::links::safe_forms(&mentions, |name| vocabulary.is_generic(&quote::words(name).join(" "))))
}

/// The page with its block of entities in place of the one it had (or at
/// its end); with none, without any. The links the app added inside its
/// text go too (`links.rs`): what is left is what the wiki wrote.
#[cfg(test)]
pub(crate) fn with_block(text: &str, block: Option<&str>) -> String {
    place_block(&super::links::strip_app_links(text), block)
}

/// The page with its block of entities in place of the one it had (or at
/// its end), its text untouched.
fn place_block(text: &str, block: Option<&str>) -> String {
    let mut rest = text.replace("\r\n", "\n");
    // Every block there is goes, so a page merged twice keeps one.
    while let Some(start) = rest.find(BLOCK_START) {
        let end = rest[start..].find(BLOCK_END).map(|e| start + e + BLOCK_END.len()).unwrap_or(rest.len());
        let after = rest[end..].trim_start_matches('\n').to_string();
        rest = format!("{}{}{after}", rest[..start].trim_end_matches('\n'), if after.is_empty() { "" } else { "\n\n" });
    }
    let mut out = rest.trim_end().to_string();
    if let Some(block) = block {
        out.push_str("\n\n");
        out.push_str(block.trim_end());
    }
    out.push('\n');
    out
}

/// The overview of the archive, from the record: how many documents were
/// read (and how many readings failed), who has a page, the most named by
/// the documents where they have a role of substance, and the matters
/// (D-240).
fn overview(vocabulary: &Vocabulary, record: &Record, paths: &BTreeMap<String, String>, documents: usize, failed: usize) -> String {
    let words = &vocabulary.pages;
    let mut out = format!(
        "---\ntype: overview\ntitle: {}\ngenerated_by: {GENERATED_BY}\n---\n\n# {}\n\n{}\n\n",
        yaml_string(words.word("overviewTitle")),
        words.word("overviewTitle"),
        words.word("overviewIntro")
    );
    out.push_str(&format!("- {}: {documents}\n", words.word("overviewDocuments")));
    if failed > 0 {
        out.push_str(&format!("- {}: {failed}\n", words.word("overviewFailed")));
    }
    let mut by_kind: BTreeMap<usize, (&str, usize)> = BTreeMap::new();
    for entity in record.entities.values().filter(|e| paths.contains_key(&e.id)) {
        let order = vocabulary.entity_types.iter().position(|t| t.id == entity.kind).unwrap_or(usize::MAX);
        by_kind.entry(order).or_insert((vocabulary.type_label(&entity.kind), 0)).1 += 1;
    }
    let counts: Vec<String> = by_kind.values().map(|(label, n)| format!("{label}: {n}")).collect();
    out.push_str(&format!("- {}: {}\n\n", words.word("overviewEntities"), counts.join(" · ")));
    // By the documents where they have a role of substance; a mention in
    // passing does not count.
    let substantive = |entity: &Entity| -> usize {
        let roles = &vocabulary.pages.most_named_roles;
        entity.documents.values().filter(|r| roles.is_empty() || r.iter().any(|role| roles.contains(role))).count()
    };
    let mut most: Vec<(&Entity, usize)> = record
        .entities
        .values()
        .filter(|e| paths.contains_key(&e.id) && e.kind != "asunto")
        .map(|e| (e, substantive(e)))
        .filter(|(_, n)| *n > 0)
        .collect();
    most.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| b.0.documents.len().cmp(&a.0.documents.len())).then_with(|| a.0.name.cmp(&b.0.name)));
    if !most.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("overviewMost")));
        for (entity, n) in most.into_iter().take(MOST_NAMED) {
            out.push_str(&format!(
                "- {} · {} — {n} {}\n",
                link(&paths[&entity.id], &entity.name),
                vocabulary.type_label(&entity.kind),
                documents_word(vocabulary, n)
            ));
        }
        out.push('\n');
    }
    let mut matters: Vec<(&str, &str, usize)> = record
        .entities
        .values()
        .filter(|e| paths.contains_key(&e.id) && e.kind == "asunto")
        .map(|e| (e.id.as_str(), e.name.as_str(), e.documents.len()))
        .chain(record.matters.values().filter(|m| paths.contains_key(&m.id)).map(|m| (m.id.as_str(), m.name.as_str(), m.documents.len())))
        .collect();
    matters.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(b.1)));
    if !matters.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("overviewMatters")));
        for (id, name, n) in matters {
            out.push_str(&format!("- {} — {n} {}\n", link(&paths[id], name), documents_word(vocabulary, n)));
        }
        out.push('\n');
    }
    out
}

/// Keeps a page in the base when what the app writes of it changed. Its
/// notes are apart and stay with its object, also when it moves.
fn write_page(
    base: &DocBase,
    record: &Record,
    id: &str,
    path: &str,
    moved_from: Option<&str>,
    rendered: &Rendered,
    report: &mut ProjectReport,
) -> Result<(), String> {
    let fingerprint = hash(&[&rendered.text]);
    let known = record.pages.get(id);
    if moved_from.is_none() && known.is_some_and(|p| p.fingerprint == fingerprint && p.state == "projected" && p.has_body) {
        report.unchanged += 1;
        return Ok(());
    }
    if moved_from.is_some_and(|from| from != path) {
        report.moved += 1;
    }
    base.conn()
        .execute(
            "INSERT OR REPLACE INTO knowledge_page (entity_id, path, state, fingerprint, synthesis, sentences, discarded, written_at, body)
             VALUES (?1, ?2, 'projected', ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                id,
                path,
                fingerprint,
                rendered.synthesis,
                rendered.sentences as i64,
                rendered.discarded as i64,
                now_ms(),
                dated(&rendered.text)
            ],
        )
        .map_err(sql_err)?;
    report.written += 1;
    Ok(())
}

/// The page of a matter (a group of documents, D-213): its parties and the
/// people around it, what its documents say of them, its documents and
/// why each one is in it. Written by the app, without the model.
fn render_matter(vocabulary: &Vocabulary, record: &Record, paths: &BTreeMap<String, String>, matter: &Matter) -> Rendered {
    let words = &vocabulary.pages;
    let contents: BTreeSet<i64> = matter.documents.keys().copied().collect();
    // Who appears in its documents, with their roles and in how many.
    let mut people: Vec<(&Entity, BTreeSet<&str>, usize)> = record
        .entities
        .values()
        .filter(|e| e.state == "active")
        .filter_map(|e| {
            let docs: Vec<&BTreeSet<String>> = e.documents.iter().filter(|(c, _)| contents.contains(c)).map(|(_, r)| r).collect();
            if docs.is_empty() {
                return None;
            }
            let roles: BTreeSet<&str> = docs.iter().flat_map(|r| r.iter().map(String::as_str)).collect();
            Some((e, roles, docs.len()))
        })
        .collect();
    people.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.name.cmp(&b.0.name)));
    let named_as: Vec<&Entity> = people.iter().filter(|(e, _, _)| e.kind == vocabulary.matters.kind).map(|(e, _, _)| *e).collect();
    let parties: Vec<&(&Entity, BTreeSet<&str>, usize)> = people
        .iter()
        .filter(|(e, roles, _)| e.kind != vocabulary.matters.kind && roles.iter().any(|r| *r != vocabulary.fallback_role()))
        .collect();
    let mut sources: Vec<String> = contents.iter().filter_map(|c| record.documents.get(c)?.identity.clone()).collect();
    sources.sort();
    sources.dedup();
    let mut aliases: Vec<String> = named_as
        .iter()
        .map(|e| one_line(&e.name))
        .filter(|n| quote::words(n) != quote::words(&matter.name) && vocabulary.is_a_matter_name(n))
        .collect();
    // The same name in capitals and not is one (D-289).
    let mut named_once = BTreeSet::new();
    aliases.retain(|a| named_once.insert(quote::words(a).join(" ")));
    let mut out = String::from("---\ntype: entity\n");
    out.push_str(&format!("title: {}\n", yaml_string(&matter.name)));
    out.push_str(&format!("entity_kind: {}\n", vocabulary.matters.kind));
    out.push_str(&format!("entity_id: {}\n", matter.id));
    out.push_str(&format!("aliases: [{}]\n", aliases.iter().map(|a| yaml_string(a)).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!("sources: [{}]\n", sources.iter().map(|s| yaml_string(s)).collect::<Vec<_>>().join(", ")));
    out.push_str(&format!(
        "related: [{}]\n",
        parties.iter().filter_map(|(e, _, _)| paths.get(&e.id)).map(|p| stem(p)).collect::<Vec<_>>().join(", ")
    ));
    out.push_str(&format!("generated_by: {GENERATED_BY}\n---\n\n# {}\n\n", matter.name));
    let mut facts = vec![format!("*{}*", vocabulary.type_label(&vocabulary.matters.kind))];
    if !aliases.is_empty() {
        facts.push(format!("{}: {}", words.word("matterNamed"), aliases.join(" · ")));
    }
    // Only what names a matter by itself (a case number): the number of
    // an invoice, an envelope or a shipment is of one of its documents.
    for entity in &named_as {
        for ((kind, _), (value, _)) in entity.identifiers.iter().filter(|((kind, _), _)| vocabulary.is_strong(kind)) {
            facts.push(format!("{} {value}", vocabulary.identifier_label(kind)));
        }
    }
    facts.dedup();
    out.push_str(&facts.join(" · "));
    out.push_str("\n\n");

    // Its parties and the people around it.
    if !parties.is_empty() {
        out.push_str(&format!("## {}\n\n", words.word("matterParties")));
        for (entity, roles, n) in &parties {
            let name = paths.get(&entity.id).map(|p| link(p, &entity.name)).unwrap_or_else(|| entity.name.clone());
            let roles: Vec<&str> = roles.iter().filter(|r| **r != vocabulary.fallback_role()).map(|r| vocabulary.role_label(r)).collect();
            out.push_str(&format!(
                "- {name} · {} — {} ({n} {})\n",
                vocabulary.type_label(&entity.kind),
                roles.join(", "),
                documents_word(vocabulary, *n)
            ));
        }
        out.push('\n');
    }

    // What its documents say of whom appears in them, each with its note:
    // the quotes and their documents (D-441).
    let mut notes = Notes::default();
    let mut said: Vec<String> = Vec::new();
    for (entity, _, _) in &people {
        for claim in entity.claims.iter().filter(|c| !c.mirrored) {
            let quotes: Vec<String> = claim
                .evidence
                .iter()
                .filter(|(c, ..)| contents.contains(&record.documents.get(c).and_then(|d| d.chosen).unwrap_or(*c)))
                .take(QUOTES_PER_CLAIM)
                .map(|(content, quote, page)| {
                    let at = page.map(|p| format!(", {} {p}", words.word("page"))).unwrap_or_default();
                    format!("«{}» ({}{at})", one_line(quote), document_link(record, *content))
                })
                .collect();
            if quotes.is_empty() {
                continue;
            }
            let subject = paths.get(&entity.id).map(|p| link(p, &entity.name)).unwrap_or_else(|| entity.name.clone());
            let object = match claim.object_entity.as_deref() {
                Some(_) if claim.amount.is_some() => object_text(record, claim),
                Some(id) => paths.get(id).map(|p| link(p, &object_text(record, claim))).unwrap_or_else(|| object_text(record, claim)),
                None => localized_dates(vocabulary, &object_text(record, claim)),
            };
            let conflict = if claim.state == "conflict" { format!(" ({})", words.word("conflicts").to_lowercase()) } else { String::new() };
            let mark = certainty_mark(vocabulary, claim);
            let note = notes.mark(&quotes.join("; "));
            said.push(format!("- {subject} — {}: {object}{conflict}{mark}{note}\n", claim_label(vocabulary, record, claim)));
        }
    }
    if !said.is_empty() {
        // All of them: nothing in the record goes missing from the page
        // (D-236).
        out.push_str(&format!("## {}\n\n", words.word("matterFacts")));
        for line in said {
            out.push_str(&line);
        }
        out.push('\n');
    }

    // Its documents, and why each one is in it.
    out.push_str(&format!("## {} ({})\n\n", words.word("documents"), contents.len()));
    for content in &contents {
        let kind = record
            .documents
            .get(content)
            .and_then(|d| d.doc_type.as_deref())
            .map(|t| format!(" · {}", t.replace('-', " ")))
            .unwrap_or_default();
        out.push_str(&format!("- {}{kind}\n", document_link(record, *content)));
    }
    out.push('\n');
    let proposed: Vec<(i64, Option<&String>)> = record
        .proposed
        .iter()
        .filter_map(|(content, list)| list.iter().find(|(id, _)| *id == matter.id).map(|(_, reason)| (*content, reason.as_ref())))
        .collect();
    if !proposed.is_empty() {
        out.push_str(&format!("### {}\n\n*{}*\n\n", words.word("matterProposed"), words.word("matterProposedNote")));
        for (content, reason) in proposed {
            let reason = reason.map(|r| format!(" — {}", one_line(r))).unwrap_or_default();
            out.push_str(&format!("- {}{reason}\n", document_link(record, content)));
        }
        out.push('\n');
    }
    // Why its documents go together, in one line: how many documents each
    // kind of reason holds (D-268). Each document with its reasons is in
    // «Datos y correcciones»: the documents are listed once (D-441).
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for reasons in matter.documents.values() {
        let kinds: BTreeSet<&str> = reasons.iter().map(|r| r.kind.as_str()).collect();
        for kind in kinds {
            *counts.entry(kind).or_default() += 1;
        }
    }
    if !counts.is_empty() {
        let label = |kind: &str| -> &str {
            match kind {
                "persona" => words.word("matterByPerson"),
                "documento" => words.word("matterSameDocument"),
                "asunto" => words.word("matterSameMatter"),
                "numero" => words.word("matterSameNumber"),
                _ => words.word("matterSameAmount"),
            }
        };
        let summary: Vec<String> = counts.iter().map(|(kind, n)| format!("{} ({n})", label(kind))).collect();
        out.push_str(&format!(
            "*{}* — {}: {}\n\n",
            words.word("matterWhy"),
            words.word("matterWhyMany").replace("{n}", &contents.len().to_string()),
            summary.join(", ")
        ));
    }
    out.push_str(&notes.block());
    Rendered { text: out, synthesis: None, sentences: 0, discarded: 0 }
}

/// Where the links to a page that moved or went lead now: another page,
/// or nowhere (the link is left as `text`, or as its own label).
struct Redirect {
    to: Option<String>,
    text: String,
}

/// The path of a page that exists: the file of its entity's name now when
/// the name changed and that file is free (the page moves, D-239); its
/// own path otherwise.
fn renamed_path(
    folder: &str,
    id: &str,
    page: &PageRow,
    name: &str,
    kind: &str,
    taken: &mut BTreeSet<String>,
    moved: &mut BTreeMap<String, String>,
) -> String {
    let wanted = page_path(folder, id, name, kind);
    if page.state == "projected" && wanted.to_lowercase() != page.path.to_lowercase() && !taken.contains(&wanted.to_lowercase()) {
        taken.insert(wanted.to_lowercase());
        moved.insert(id.to_string(), page.path.clone());
        return wanted;
    }
    page.path.clone()
}

/// The title in the header of a page.
pub(crate) fn title_of(text: &str) -> Option<String> {
    let text = text.replace("\r\n", "\n");
    let header = text.strip_prefix("---\n")?.split("\n---").next()?;
    let line = header.lines().find(|l| l.starts_with("title:"))?;
    let value = line.trim_start_matches("title:").trim();
    let value = serde_json::from_str::<String>(value).unwrap_or_else(|_| value.trim_matches('"').trim_matches('\'').to_string());
    Some(value).filter(|v| !v.trim().is_empty())
}

/// The readable part of a page's file name, without the id at its end:
/// "persona-dos-1a2b3c4d.md" is "persona dos".
fn readable_stem(path: &str) -> String {
    let stem = stem(path);
    let readable = match stem.rsplit_once('-') {
        Some((head, tail)) if tail.len() >= 8 && tail.chars().all(|c| c.is_ascii_hexdigit()) => head.to_string(),
        _ => stem.clone(),
    };
    readable.replace('-', " ")
}

/// Rewrites, in every text of the base that links pages (the pages, the
/// notes of a person, the overview and the saved queries), the links to a
/// page that moved or went: `[[old|text]]` leads to the new page, or is
/// left as `text`. The `related:` lists follow too. Returns how many
/// texts changed.
fn redirect_links(conn: &Connection, redirects: &BTreeMap<String, Redirect>) -> Result<usize, String> {
    let mut changed = 0;
    for (table, key, column) in [
        ("knowledge_page", "entity_id", "body"),
        ("knowledge_note", "id", "text"),
        ("knowledge_overview", "id", "body"),
        ("knowledge_query", "id", "answer"),
    ] {
        let rows: Vec<(String, String)> = {
            let mut statement = conn
                .prepare(&format!("SELECT CAST({key} AS TEXT), {column} FROM {table} WHERE {column} IS NOT NULL"))
                .map_err(sql_err)?;
            let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(sql_err)?;
            rows.collect::<Result<_, _>>().map_err(sql_err)?
        };
        for (id, text) in rows {
            let next = with_redirects(&text, redirects);
            if next != text {
                conn.execute(&format!("UPDATE {table} SET {column} = ?2 WHERE CAST({key} AS TEXT) = ?1"), params![id, next])
                    .map_err(sql_err)?;
                changed += 1;
            }
        }
    }
    Ok(changed)
}

/// A text with its links redirected (see `redirect_links`).
fn with_redirects(text: &str, redirects: &BTreeMap<String, Redirect>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find("]]") else { break };
        let inner = &after[..end];
        let (target, label) = match inner.split_once('|') {
            Some((t, l)) => (t, Some(l)),
            None => (inner, None),
        };
        let (bare, anchor) = match target.split_once('#') {
            Some((b, a)) => (b, Some(a)),
            None => (target, None),
        };
        let key = bare.trim().rsplit('/').next().unwrap_or(bare).trim_end_matches(".md").to_lowercase();
        let embed = rest[..start].ends_with('!');
        out.push_str(&rest[..start]);
        match redirects.get(&key) {
            Some(redirect) if !embed => match &redirect.to {
                Some(to) => {
                    out.push_str("[[");
                    out.push_str(to);
                    if let Some(anchor) = anchor {
                        out.push('#');
                        out.push_str(anchor);
                    }
                    if let Some(label) = label {
                        out.push('|');
                        out.push_str(label);
                    }
                    out.push_str("]]");
                }
                None => out.push_str(label.map(str::trim).filter(|l| !l.is_empty()).unwrap_or(&redirect.text)),
            },
            _ => {
                out.push_str("[[");
                out.push_str(inner);
                out.push_str("]]");
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    // The bare names of the `related:` list of the header; a line no
    // redirect touches stays as it is.
    out.split('\n')
        .map(|line| {
            let (body, cr) = match line.strip_suffix('\r') {
                Some(body) => (body, "\r"),
                None => (line, ""),
            };
            let Some(list) = body.strip_prefix("related:") else { return line.to_string() };
            let Some(inner) = list.trim().strip_prefix('[').and_then(|l| l.strip_suffix(']')) else { return line.to_string() };
            let items: Vec<&str> = inner.split(',').map(str::trim).filter(|i| !i.is_empty()).collect();
            if !items.iter().any(|item| redirects.contains_key(&item.trim_matches('"').to_lowercase())) {
                return line.to_string();
            }
            let items: Vec<String> = items
                .into_iter()
                .filter_map(|item| match redirects.get(&item.trim_matches('"').to_lowercase()) {
                    Some(Redirect { to: Some(to), .. }) => Some(to.clone()),
                    Some(Redirect { to: None, .. }) => None,
                    None => Some(item.to_string()),
                })
                .collect();
            format!("related: [{}]{cr}", items.join(", "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The entity a merged one went into, following the chain.
fn survivor<'a>(record: &'a Record, id: &str) -> Option<&'a Entity> {
    let mut current = record.entities.get(id)?;
    for _ in 0..16 {
        match (&current.state[..], &current.merged_into) {
            ("merged", Some(next)) => current = record.entities.get(next)?,
            ("active", _) => return Some(current),
            _ => return None,
        }
    }
    None
}

/// What a projection is computed on besides the record: the documents and
/// what the pages of documents are written from, the vocabulary and the
/// language.
fn basis_of(base: &DocBase, vocabulary: &Vocabulary, language: Option<&str>, documents: &str) -> Result<String, String> {
    let conn = base.conn();
    // What the pages of documents are written from besides the record:
    // their summaries, types and data (D-340).
    let written_from: String = conn
        .query_row(
            "SELECT coalesce((SELECT group_concat(content_id || ':' || coalesce(topic, '') || ':' || length(points), '|') FROM knowledge_summary), '')
                 || '#' || coalesce((SELECT group_concat(content_id || ':' || doc_type || ':' || coalesce(stage, ''), '|') FROM document_type), '')
                 || '#' || coalesce((SELECT group_concat(content_id || ':' || basis || ':' || length(data), '|') FROM document_data), '')",
            [],
            |row| row.get(0),
        )
        .map_err(sql_err)?;
    Ok(hash(&[PROJECTION_VERSION, &vocabulary.digest, language.unwrap_or(""), documents, &written_from]))
}

/// How the projection reaches the model.
struct Asking<'a> {
    asker: Option<&'a dyn DocumentAsker>,
    engine: Option<String>,
    budget: usize,
    collect: Option<Vec<Question>>,
}

impl DocBase {
    /// The language the archive's wiki is written in; none (or "auto")
    /// lets the pages follow the language of the documents.
    pub fn set_wiki_language(&self, language: Option<&str>) -> Result<(), String> {
        let conn = self.conn();
        match language.map(str::trim).filter(|l| !l.is_empty()) {
            Some(language) => set_setting(&conn, WIKI_LANGUAGE_SETTING, language),
            None => conn.execute("DELETE FROM docbase_setting WHERE key = ?1", [WIKI_LANGUAGE_SETTING]).map(|_| ()).map_err(sql_err),
        }
    }

    /// The questions a projection would ask now (for the evaluation, which
    /// warms its cache with them first). Writes nothing.
    #[cfg(test)]
    pub(crate) fn synthesis_questions(&self, engine: &str) -> Result<Vec<Question>, String> {
        let mut report = ProjectReport::default();
        let mut asking = Asking { asker: None, engine: Some(engine.to_string()), budget: 0, collect: Some(Vec::new()) };
        project(self, &mut asking, &mut report, true)?;
        Ok(asking.collect.unwrap_or_default())
    }

    /// Projects the record into the pages of the base: the page of each
    /// entity and matter that has one, the page of each document and the
    /// overview; up to `max_questions` questions to `asker` for what the
    /// documents say (none without one: the pages are written all the
    /// same, with the synthesis pending). Waits for the resolution of
    /// identities to finish, and does nothing when the record did not
    /// change since the last complete projection. Needs no wiki (D-402).
    pub fn knowledge_project(&self, asker: Option<&dyn DocumentAsker>, max_questions: usize) -> Result<ProjectReport, String> {
        let _one = PROJECTING.lock().unwrap_or_else(|p| p.into_inner());
        let _asking = asker.map(|_| ASKING.lock().unwrap_or_else(|p| p.into_inner()));
        let engine = match asker {
            Some(asker) => Some(asker.engine_id()),
            None => classify::ask_engine(&self.conn())?.map(|c| crate::source_volume::asking::engine_id(&c)),
        };
        let mut report = ProjectReport::default();
        let mut asking = Asking { asker, engine, budget: max_questions, collect: None };
        project(self, &mut asking, &mut report, false)?;
        Ok(report)
    }
}

fn project(base: &DocBase, asking: &mut Asking, report: &mut ProjectReport, dry: bool) -> Result<(), String> {
    let vocabulary = vocabulary();
    let started = now_ms();
    let to_resolve: i64 = base
        .conn()
        .query_row("SELECT count(*) FROM knowledge_dirty WHERE scope IN ('resolve', 'retire')", [], |row| row.get(0))
        .map_err(sql_err)?;
    if to_resolve > 0 {
        report.skipped = true;
        report.reason = Some("resolving".into());
        return Ok(());
    }
    // The documents and their versions (D-340), with the name of each of
    // their files; a copy a person passed over is named by the one chosen.
    let mut families = family::families(base, vocabulary)?;
    let identities: HashMap<i64, String> = {
        let passed = passed_over(&base.conn())?;
        families
            .identities
            .iter()
            .filter(|(content, _)| !passed.contains_key(content))
            .filter_map(|(content, names)| names.first().map(|name| (*content, name.clone())))
            .collect()
    };
    let (mut record, language, basis_before, dirty) = {
        let conn = base.conn();
        let record = load(&conn, &identities)?;
        let dirty: i64 =
            conn.query_row("SELECT count(*) FROM knowledge_dirty WHERE scope = 'project'", [], |row| row.get(0)).map_err(sql_err)?;
        (record, wiki_language(&conn)?, setting(&conn, BASIS_SETTING)?, dirty)
    };
    // What the pages are computed on, besides the record: the documents,
    // the vocabulary and the language.
    let documents_digest = families.digest();
    let basis = basis_of(base, vocabulary, language.as_deref(), &documents_digest)?;
    if !dry && dirty == 0 && basis_before.as_deref() == Some(basis.as_str()) {
        report.skipped = true;
        report.reason = Some("unchanged".into());
        return Ok(());
    }
    // The readable title of each document's page (P-182).
    families.titles = family::titles(&base.conn(), vocabulary, &record, &families)?;

    // Which entities have a page, and where.
    let folder = vocabulary.pages.folder().to_string();
    let mut paths: BTreeMap<String, String> = BTreeMap::new();
    let mut taken: BTreeSet<String> = record.pages.values().map(|p| p.path.to_lowercase()).collect();
    // A page whose entity changed its name moves to the file of the new
    // name (the same id at its end); the links to the old one follow (D-239).
    let mut moved: BTreeMap<String, String> = BTreeMap::new();
    for entity in record.entities.values() {
        if !has_page(vocabulary, entity) {
            continue;
        }
        let path = match record.pages.get(&entity.id) {
            Some(page) => renamed_path(&folder, &entity.id, page, &entity.name, &entity.kind, &mut taken, &mut moved),
            None => {
                let mut path = page_path(&folder, &entity.id, &entity.name, &entity.kind);
                if taken.contains(&path.to_lowercase()) {
                    path = format!("{folder}/{}-{}.md", slug(&entity.name, &entity.kind), entity.id.trim_start_matches("ent-"));
                }
                taken.insert(path.to_lowercase());
                path
            }
        };
        paths.insert(entity.id.clone(), path);
    }
    // And the matters with documents enough.
    for matter in record.matters.values() {
        if matter.documents.len() < vocabulary.matters.min_documents {
            continue;
        }
        let path = match record.pages.get(&matter.id) {
            Some(page) => renamed_path(&folder, &matter.id, page, &matter.name, &vocabulary.matters.kind, &mut taken, &mut moved),
            None => {
                let mut path = page_path(&folder, &matter.id, &matter.name, &vocabulary.matters.kind);
                if taken.contains(&path.to_lowercase()) {
                    path = format!("{folder}/{}-{}.md", slug(&matter.name, &vocabulary.matters.kind), matter.id.trim_start_matches("mat-"));
                }
                taken.insert(path.to_lowercase());
                path
            }
        };
        paths.insert(matter.id.clone(), path);
    }
    report.pages = paths.len();
    // The page of each document or family of versions; the documents of
    // the record link to it.
    let document_paths = family::page_paths(&base.conn(), vocabulary, &record, &families)?;
    for family in &families.list {
        let path = &document_paths[&family.id];
        for content in &family.members {
            if let Some(document) = record.documents.get_mut(content) {
                document.page = Some(path.clone());
            }
        }
    }
    paths.extend(document_paths.clone());
    report.source_pages = families.list.len();

    // The page of each entity.
    for (id, path) in &paths {
        let Some(entity) = record.entities.get(id) else {
            let Some(matter) = record.matters.get(id) else { continue };
            let rendered = render_matter(vocabulary, &record, &paths, matter);
            if !dry {
                write_page(base, &record, id, path, moved.get(id).map(String::as_str), &rendered, report)?;
            }
            continue;
        };
        let claims = numbered(vocabulary, entity);
        let mut answer: Option<(String, String)> = None;
        if !claims.is_empty() {
            let question = synthesis_question(vocabulary, &record, entity, synthesized(&claims), language.as_deref());
            let cached = cached_answer(&base.conn(), &question.key, asking.engine.as_deref())?;
            match cached {
                Some(text) => answer = Some((question.key.clone(), text)),
                None => {
                    if let Some(collect) = &mut asking.collect {
                        collect.push(question);
                    } else if let (Some(asker), true) = (asking.asker, asking.budget > 0) {
                        asking.budget -= 1;
                        report.asked += 1;
                        match ask_and_keep(base, asker, &question) {
                            Ok(text) => answer = Some((question.key.clone(), text)),
                            Err(error) => {
                                report.failed += 1;
                                report.errors.push(format!("{}: {error}", entity.name));
                                if stops_the_run(&error) {
                                    asking.budget = 0;
                                }
                            }
                        }
                    } else {
                        report.left += 1;
                    }
                }
            }
        }
        let rendered = render(vocabulary, &record, &paths, entity, answer.as_ref().map(|(k, t)| (k.as_str(), t.as_str())), &claims);
        report.sentences += rendered.sentences;
        report.discarded += rendered.discarded;
        if dry {
            continue;
        }
        write_page(base, &record, id, path, moved.get(id).map(String::as_str), &rendered, report)?;
    }
    if dry {
        return Ok(());
    }

    // Where the links to a page that moved or went now lead: the page of
    // its new name, the page of the entity it went into, or nowhere (the
    // link is left as its text).
    let mut redirects: BTreeMap<String, Redirect> = BTreeMap::new();
    for (id, old) in &moved {
        if let Some(new) = paths.get(id) {
            let text = record
                .entities
                .get(id)
                .map(|e| e.name.clone())
                .or_else(|| record.matters.get(id).map(|m| m.name.clone()))
                .unwrap_or_else(|| readable_stem(new));
            redirects.insert(stem(old).to_lowercase(), Redirect { to: Some(stem(new)), text });
        }
    }

    // Pages whose entity has no page any more (it left the record, went
    // into another one or no longer meets the rule): gone, unless a person
    // wrote notes on it (R-09). Then a short page stays in its place,
    // under the entity's name, saying where it went. The links to a page
    // that goes lead to the entity it went into, or are left as text
    // (D-239, replaces in part D-206). The notes of a version of a
    // document that joined its family's page go to that page (D-340).
    for (id, page) in &record.pages {
        if paths.contains_key(id) {
            continue;
        }
        let body = stored_body(&base.conn(), id)?;
        // A page of a document that is now a version on another page:
        // its notes and its links go there.
        let into_family = id.strip_prefix("doc-").and_then(|n| n.parse::<i64>().ok()).and_then(|c| families.family_of(c));
        if let Some(into) = into_family {
            let target = &paths[&into.id];
            if *target != page.path {
                move_notes(&base.conn(), id, &into.id)?;
                let text = body.as_deref().and_then(title_of).unwrap_or_else(|| readable_stem(&page.path));
                redirects.insert(stem(&page.path).to_lowercase(), Redirect { to: Some(stem(target)), text });
                report.removed += 1;
            }
            base.conn().execute("DELETE FROM knowledge_page WHERE entity_id = ?1", [id]).map_err(sql_err)?;
            continue;
        }
        let into = survivor(&record, id).filter(|s| s.id != *id).and_then(|s| paths.get(&s.id).map(|p| (s, p)));
        // Its name; never its id.
        let name = record
            .entities
            .get(id)
            .map(|e| e.name.clone())
            .or_else(|| record.matters.get(id).map(|m| m.name.clone()))
            .or_else(|| body.as_deref().and_then(title_of))
            .unwrap_or_else(|| readable_stem(&page.path));
        if notes_of_object(&base.conn(), id)?.trim().is_empty() {
            base.conn().execute("DELETE FROM knowledge_page WHERE entity_id = ?1", [id]).map_err(sql_err)?;
            redirects.insert(stem(&page.path).to_lowercase(), Redirect { to: into.map(|(_, path)| stem(path)), text: name });
            report.removed += 1;
            continue;
        }
        let mut text = format!(
            "---\ntype: entity\ntitle: {}\nentity_id: {id}\nstate: kept\ngenerated_by: {GENERATED_BY}\n---\n\n# {name}\n\n",
            yaml_string(&name)
        );
        match into {
            Some((into, path)) => text.push_str(&format!("{} {}.\n", vocabulary.pages.word("keptMerged"), link(path, &into.name))),
            None => text.push_str(&format!("{}\n", vocabulary.pages.word("kept"))),
        }
        let fingerprint = hash(&[&text]);
        if page.state == "kept" && page.fingerprint == fingerprint && page.has_body {
            continue;
        }
        base.conn()
            .execute(
                "UPDATE knowledge_page SET state = 'kept', fingerprint = ?2, synthesis = NULL, written_at = ?3, body = ?4 WHERE entity_id = ?1",
                params![id, fingerprint, now_ms(), text],
            )
            .map_err(sql_err)?;
        report.kept += 1;
    }
    if !redirects.is_empty() {
        report.redirected += redirect_links(&base.conn(), &redirects)?;
    }

    // The page of each document (D-340): what the base knows of it and of
    // its versions, and the block of entities.
    for family in &families.list {
        let path = &document_paths[&family.id];
        let generated = family::render_document(&base.conn(), vocabulary, &record, &paths, &families, family)?;
        let block = source_block(vocabulary, &record, &paths, &family.members);
        let forms = link_forms(base, vocabulary, &record, &paths, &family.members)?;
        let linked = super::links::with_links(&generated, &forms);
        let placed = match &block {
            Some(block) => linked.replacen(family::BLOCK_PLACE, block.trim_end(), 1),
            None => linked.replacen(&format!("{}\n\n", family::BLOCK_PLACE), "", 1),
        };
        let fingerprint = hash(&[&placed]);
        let known = record.pages.get(&family.id);
        if known.is_some_and(|p| p.path == *path && p.fingerprint == fingerprint && p.state == "projected" && p.has_body) {
            continue;
        }
        base.conn()
            .execute(
                "INSERT OR REPLACE INTO knowledge_page (entity_id, path, state, fingerprint, synthesis, sentences, discarded, written_at, body)
                 VALUES (?1, ?2, 'projected', ?3, NULL, 0, 0, ?4, ?5)",
                params![family.id, path, fingerprint, now_ms(), placed],
            )
            .map_err(sql_err)?;
        report.blocks_written += 1;
    }

    // The overview.
    {
        // What was read, not only the documents that name someone (D-240).
        let (documents, failed) = {
            let standings = super::extract::standings(&base.conn(), &Visible::All, None)?;
            let state = |wanted: &str| standings.iter().filter(|s| s.reading.as_ref().is_some_and(|r| r.current && r.state == wanted)).count();
            (state("done"), state("failed"))
        };
        let text = overview(vocabulary, &record, &paths, documents, failed);
        let fingerprint = hash(&[&text]);
        let conn = base.conn();
        let before: Option<String> =
            conn.query_row("SELECT fingerprint FROM knowledge_overview WHERE id = 1", [], |row| row.get(0)).optional().map_err(sql_err)?;
        if before.as_deref() != Some(fingerprint.as_str()) {
            conn.execute(
                "INSERT OR REPLACE INTO knowledge_overview (id, path, body, fingerprint, written_at) VALUES (1, ?1, ?2, ?3, ?4)",
                params![OVERVIEW_PATH, dated(&text), fingerprint, now_ms()],
            )
            .map_err(sql_err)?;
            report.overview_written = true;
        }
    }

    if report.left == 0 && report.failed == 0 {
        base.conn().execute("DELETE FROM knowledge_dirty WHERE scope = 'project' AND since <= ?1", [started]).map_err(sql_err)?;
        // What the next projection compares with.
        let basis = basis_of(base, vocabulary, language.as_deref(), &documents_digest)?;
        set_setting(&base.conn(), BASIS_SETTING, &basis)?;
    }
    Ok(())
}

#[cfg(test)]
mod page_rule_tests {
    use super::*;

    fn with(kind: &str, name: &str, roles: &[(i64, &str)]) -> Entity {
        let mut entity = Entity { id: "ent-x".into(), kind: kind.into(), name: name.into(), state: "active".into(), ..Default::default() };
        for (content, role) in roles {
            entity.documents.entry(*content).or_default().insert(role.to_string());
        }
        entity
    }

    #[test]
    fn a_note_is_numbered_once_in_the_order_the_text_names_it() {
        let mut notes = Notes::default();
        assert_eq!(notes.mark("«uno» (a)"), "[^1]");
        assert_eq!(notes.mark("«dos»\n (b)"), "[^2]");
        assert_eq!(notes.mark("«uno»  (a)"), "[^1]");
        assert_eq!(notes.mark("  "), "");
        assert_eq!(notes.block(), "[^1]: «uno» (a)\n[^2]: «dos» (b)\n\n");
        assert_eq!(Notes::default().block(), "");
        // What the model writes as a reference goes: the page numbers its notes.
        assert_eq!(without_markers("Vive en Calle Uno 100. [2] Tiene [^3] un dato [1, 2]."), "Vive en Calle Uno 100. Tiene un dato.");
        assert_eq!(without_markers("Ver [nota] y [^a]."), "Ver [nota] y [^a].");
    }

    #[test]
    fn one_number_given_with_its_kind_and_as_another_number_is_shown_once() {
        let mut ids: BTreeMap<(String, String), (String, BTreeSet<i64>)> = BTreeMap::new();
        ids.insert(("dni".into(), "22222222".into()), ("22.222.222".into(), BTreeSet::from([1])));
        ids.insert(("otro".into(), "22222222".into()), ("22222222".into(), BTreeSet::from([2])));
        ids.insert(("otro".into(), "999".into()), ("999".into(), BTreeSet::from([3])));
        one_value_once(&mut ids, "otro");
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[&("dni".to_string(), "22222222".to_string())].1, BTreeSet::from([1, 2]));
    }

    #[test]
    fn a_page_needs_a_role_of_substance_or_something_said() {
        let v = Vocabulary::builtin();
        // Named in passing in many documents (a bank on invoices, a
        // jurisdiction on a form): no page (D-266).
        let passing = with("organizacion", "Banco de Ejemplo", &[(1, "otro"), (2, "otro"), (3, "otro")]);
        assert!(!has_page(&v, &passing));
        assert!(has_page(&v, &with("organizacion", "Ejemplo Uno S.A.", &[(1, "parte")])));
        // What the documents say of it is reason enough.
        let mut said = passing.clone();
        said.claims.push(Claim {
            id: 1,
            predicate: "domicilio".into(),
            context: None,
            qualifier: None,
            object_entity: None,
            object_value: Some("Calle Uno 100".into()),
            state: "supported".into(),
            valid_from: None,
            evidence: vec![(1, "Calle Uno 100".into(), None)],
            amount: None,
            detail: None,
            mirrored: false,
            certainty: "quoted".into(),
            reasons: Vec::new(),
        });
        // Its address alone is not: the offices of a list of
        // jurisdictions on a form (D-288).
        assert!(!has_page(&v, &said));
        said.claims[0].predicate = "inicio_de_vinculo".into();
        said.claims[0].object_value = Some("1 de marzo de 2020".into());
        assert!(has_page(&v, &said));
        // Its telephone and mail on a form are not (D-284).
        let mut contact = passing.clone();
        let mut phone = said.claims[0].clone();
        phone.predicate = "contacto".into();
        phone.object_value = Some("0800-000-0000".into());
        contact.claims.push(phone);
        assert!(!has_page(&v, &contact));
        // A group of companies always has its page.
        let mut group = with("organizacion", "Ejemplo", &[(1, "otro")]);
        group.members.push(("ent-y".into(), vec!["misma-casilla".into()]));
        assert!(has_page(&v, &group));
    }
}
