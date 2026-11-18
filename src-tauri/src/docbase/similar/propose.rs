//! The relation proposed for a pair, and the sentence that describes it.
//!
//! The general rule of the proposal (section 6); the numbers are starting
//! points, adjusted only with general criteria against the answer sheet
//! (R-25), always preferring to miss a relation over proposing a false
//! one (R-07). With A the document with less own text:
//!
//! | If… | Proposed |
//! |---|---|
//! | One has the verifiable signature of a party, the other not, and holds it whole | `signed-copy` |
//! | One is sealed by a signing service and all it adds are stamps of a name with a date and a time | `signed-copy` |
//! | One has the marks of a preview, the other not, and holds it almost whole, with no identifier or amount replaced | `preview` |
//! | They share almost everything (`NEAR_SAME`), in short changes, none of them data | `near-identical` |
//! | A is almost whole inside B (`CONTAINED`) and B has much more text (`CONTAINER_LONGER`) | `contained-in` |
//! | They share much (`SHARE_MUCH`) and an identifier or an amount is replaced by another, or a form field, or there is a repeated substitution, or what is replaced in the shared text is mostly data (a clause may be added) | `same-template` |
//! | They share much, what changes is mostly wording, and B also shares much of itself with A (`VERSION_BOTH_WAYS`) | `version` |
//! | The same, but B shares little of itself with A: `contained-in` if A is almost whole inside B (`CONTAINED`), otherwise `similar` | `contained-in` or `similar` |
//! | They share between `SHOW_MIN` and `SHARE_MUCH` | `similar`, with no relation suggested |
//! | Less than `SHOW_MIN` | nothing |
//!
//! "Share" is the part of A's own words (without the archive's routine
//! text) found, in order, in B. Text read from an image brings reading
//! errors: those pairs get `RECOGNIZED_TOLERANCE` more slack and low
//! certainty, except for `near-identical`, which never gets slack.
//!
//! Then the key data of the two (`with_key_data`, D-497): a `version` or a
//! `similar` whose parties, matter, identifiers, amounts or dates differ
//! for real (`key_data`) is not the same document. A `version` becomes
//! `same-template` (they share a template's worth of text), and `similar`
//! is no pair at all.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::changes::{Changes, Kind};
use super::key_data::Key;

pub const SHOW_MIN: f64 = 0.5;
pub const SHARE_MUCH: f64 = 0.8;
/// A version shares much in both directions: the longer one also has at
/// least this share of its own words in the shorter one.
pub const VERSION_BOTH_WAYS: f64 = 0.8;
pub const NEAR_SAME: f64 = 0.97;
/// Longest change, in words, that still counts as "loose words".
pub const NEAR_SAME_LONGEST: usize = 3;
pub const CONTAINED: f64 = 0.9;
pub const CONTAINER_LONGER: f64 = 1.5;
/// The signed copy holds the other whole: at most this share of the
/// other's words may be missing from it.
pub const SIGNED_COVER: f64 = 0.98;
pub const PREVIEW_COVER: f64 = 0.85;
/// Share of the replaced tokens (changes where both sides have
/// something) that data must exceed for `same-template`: "mostly data".
pub const TEMPLATE_DATA_SHARE: f64 = 0.5;
pub const RECOGNIZED_TOLERANCE: f64 = 0.05;

/// One side of a pair, as the rules see it.
#[derive(Debug, Clone)]
pub struct Side {
    pub content_id: i64,
    /// Words of its own: without the archive's routine text.
    pub own_words: usize,
    /// Of those, how many were found in the other document.
    pub matched: usize,
    pub recognized: bool,
    /// Kinds of the marks it carries.
    pub marks: Vec<String>,
}

impl Side {
    pub fn cover(&self) -> f64 {
        if self.own_words == 0 {
            0.0
        } else {
            self.matched as f64 / self.own_words as f64
        }
    }

    fn has(&self, mark: &str) -> bool {
        self.marks.iter().any(|m| m == mark)
    }
}

/// The two measures over shingles, without the routine text.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Measures {
    /// Shingles in common over all the shingles of the two.
    pub jaccard: f64,
    /// Shingles in common over those of the one with fewer.
    pub containment: f64,
    /// Word level: part of the shorter one found in the other.
    pub cover: f64,
    /// Word level: words in common over the words of the two.
    pub similarity: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proposal {
    pub relation: String,
    /// For a directed relation, the document that is the relation of the
    /// other (the signed copy, the preview, the one contained).
    pub subject: Option<i64>,
    /// "high", or "low" for text read from images and for `similar`.
    pub certainty: String,
    pub description: String,
    /// For relations that make both one document, which one the app
    /// suggests as the wiki's source; a person chooses.
    pub suggested_source: Option<i64>,
    /// Which rule decided, for the evidence.
    pub rule: String,
}

pub fn measures(a: &Side, b: &Side, shingles: (usize, usize, usize)) -> Measures {
    let (both, in_a, in_b) = shingles;
    let union = in_a + in_b - both;
    let words = a.own_words + b.own_words;
    Measures {
        jaccard: ratio(both, union),
        containment: ratio(both, in_a.min(in_b)),
        cover: a.cover().max(b.cover()),
        similarity: ratio(a.matched + b.matched, words),
    }
}

fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// The relation proposed for `a` and `b`, or none when they share too
/// little to show.
pub fn propose(a: &Side, b: &Side, m: &Measures, changes: &Changes) -> Option<Proposal> {
    let recognized = a.recognized || b.recognized;
    let slack = if recognized { RECOGNIZED_TOLERANCE } else { 0.0 };
    let certainty = if recognized { "low" } else { "high" };
    // The shorter one, by its own words.
    let (short, long) = if (a.own_words, a.content_id) <= (b.own_words, b.content_id) { (a, b) } else { (b, a) };
    let cover = short.cover();
    if cover < SHOW_MIN - slack {
        return None;
    }
    let make = |relation: &str, subject: Option<i64>, source: Option<i64>, rule: &str, certainty: &str| Proposal {
        relation: relation.to_string(),
        subject,
        certainty: certainty.to_string(),
        description: describe(relation, cover, changes),
        suggested_source: source,
        rule: rule.to_string(),
    };
    let swapped_ids = changes.swapped(Kind::Identifier);
    let removed_share = |side: &Side| ratio(changes_on(side, a, changes), side.own_words.max(1));

    for (signed, other) in [(a, b), (b, a)] {
        if signed.has("party-signature")
            && !other.has("party-signature")
            && other.cover() >= SIGNED_COVER - slack
            && removed_share(other) <= 1.0 - SIGNED_COVER + slack
            && swapped_ids == 0
        {
            return Some(make("signed-copy", Some(signed.content_id), Some(signed.content_id), "party-signature", certainty));
        }
    }
    // The stamp a signing service puts by a signature (a name with a date
    // and a time) in a file the service sealed is a party's signature
    // (P-485), when it is all that tells the two apart.
    for (signed, other, stamps) in [(a, b, changes.stamps_a), (b, a, changes.stamps_b)] {
        if signed.has("service-seal")
            && !other.has("party-signature")
            && stamps > 0
            && stamps == changes.hunks
            && other.cover() >= SIGNED_COVER - slack
            && swapped_ids == 0
        {
            return Some(make("signed-copy", Some(signed.content_id), Some(signed.content_id), "service-seal-stamp", certainty));
        }
    }
    for (preview, other) in [(a, b), (b, a)] {
        if preview.has("preview")
            && !other.has("preview")
            && other.cover() >= PREVIEW_COVER - slack
            && swapped_ids == 0
            && changes.swapped(Kind::Amount) == 0
        {
            return Some(make("preview", Some(preview.content_id), None, "preview-mark", certainty));
        }
    }
    if m.similarity >= NEAR_SAME && changes.data_items() == 0 && changes.longest_hunk <= NEAR_SAME_LONGEST {
        // Text extracted from the file is better than text read from pixels.
        let source = match (a.recognized, b.recognized) {
            (false, true) => a.content_id,
            (true, false) => b.content_id,
            _ => long.content_id,
        };
        return Some(make("near-identical", None, Some(source), "near-same", certainty));
    }
    if cover >= CONTAINED - slack && long.own_words as f64 >= CONTAINER_LONGER * short.own_words as f64 {
        return Some(make("contained-in", Some(short.content_id), None, "contained", certainty));
    }
    if cover >= SHARE_MUCH - slack {
        // Data only added on one side (a signature block filled in) is not
        // another party: a value has to be replaced by another.
        // "Two contracts identical but for the amount are two contracts"
        // (proposal, section 6): an identifier or an amount replaced by
        // another is enough.
        let replaced = changes.swaps.iter().any(|(kind, n)| kind.is_data() && *n > 0);
        let data = swapped_ids > 0
            || changes.swapped(Kind::Amount) > 0
            || changes.count(Kind::Field) > 0
            || (replaced && changes.replaced_data_share() > TEMPLATE_DATA_SHARE)
            || !changes.repeated.is_empty();
        if data {
            return Some(make("same-template", None, None, "shared-text-other-data", certainty));
        }
        // A version shares much both ways: a short text mostly inside a
        // long one that shares little of itself back is not the same
        // document reworded. It is one inside the other when the short
        // one is almost whole in it, and otherwise only similar.
        if long.cover() >= VERSION_BOTH_WAYS - slack {
            return Some(make("version", None, None, "shared-text-other-wording", certainty));
        }
        if cover >= CONTAINED - slack {
            return Some(make("contained-in", Some(short.content_id), None, "contained-one-way", certainty));
        }
        return Some(make("similar", None, None, "shared-one-way", "low"));
    }
    Some(make("similar", None, None, "some-shared-text", "low"))
}

/// What a proposal becomes with the key data of the two documents:
/// `differing` are the keys in which they differ for real
/// (`key_data::differing`). Two documents that share almost all their text
/// but name other parties, another matter, other identifiers, amounts or
/// dates are two documents (D-497): a `version` is taken down to
/// `same-template`, since they share as much text as a template's
/// documents do, and a `similar` is no pair (`None`): they share less
/// than a template and are not the same document either. Everything else
/// is left as it is: a signed copy, a preview, the practically same text
/// or one inside the other already need the text of one whole in the
/// other, and the same template already says they are two documents.
pub fn with_key_data(proposal: Proposal, differing: &[Key]) -> Option<Proposal> {
    if differing.is_empty() {
        return Some(proposal);
    }
    match proposal.relation.as_str() {
        "version" => {
            let words: Vec<String> = differing.iter().map(|key| key.words().to_string()).collect();
            Some(Proposal {
                relation: "same-template".to_string(),
                subject: None,
                suggested_source: None,
                description: format!("Mismo texto de base, pero son documentos distintos: cambian {}.", listed(&words)),
                rule: "different-key-data".to_string(),
                ..proposal
            })
        }
        "similar" => None,
        _ => Some(proposal),
    }
}

/// Words that `side` has and the other does not.
fn changes_on(side: &Side, a: &Side, changes: &Changes) -> usize {
    if side.content_id == a.content_id {
        changes.removed_words
    } else {
        changes.added_words
    }
}

/// "una fecha", "3 importes"…
fn counted(n: usize, kind: Kind) -> String {
    let (one, many) = match kind {
        Kind::Identifier => ("un identificador", "identificadores"),
        Kind::Amount => ("un importe", "importes"),
        Kind::Date => ("una fecha", "fechas"),
        Kind::Number => ("una cifra", "cifras"),
        Kind::Name => ("un nombre", "nombres"),
        Kind::Field => ("un campo del formulario", "campos del formulario"),
        Kind::Wording => ("la redacción en un tramo", "tramos"),
    };
    match (n, kind) {
        (1, _) => one.to_string(),
        (n, Kind::Wording) => format!("la redacción en {n} {many}"),
        (n, _) => format!("{n} {many}"),
    }
}

fn listed(parts: &[String]) -> String {
    match parts {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} y {last}", init.join(", ")),
    }
}

/// What changes, in words: "cambian 3 importes y una fecha".
fn what_changes(changes: &Changes) -> Option<String> {
    let order = [Kind::Identifier, Kind::Name, Kind::Amount, Kind::Date, Kind::Number, Kind::Field, Kind::Wording];
    let items: BTreeMap<Kind, usize> = changes.items.iter().filter(|(_, n)| **n > 0).map(|(k, n)| (*k, *n)).collect();
    let parts: Vec<String> = order.iter().filter_map(|kind| items.get(kind).map(|n| counted(*n, *kind))).collect();
    if parts.is_empty() {
        return None;
    }
    let total: usize = items.values().sum();
    let verb = if total == 1 { "cambia" } else { "cambian" };
    Some(format!("{verb} {}", listed(&parts)))
}

/// One sentence, in Spanish, on what the two have in common and what
/// changes. The relation's own name comes from the vocabulary; this only
/// describes the evidence.
pub fn describe(relation: &str, cover: f64, changes: &Changes) -> String {
    let percent = (cover * 100.0).floor() as i64;
    let changed = what_changes(changes);
    let substitution = if changes.repeated.is_empty() {
        String::new()
    } else {
        "; un mismo dato se reemplaza siempre por otro".to_string()
    };
    match relation {
        "near-identical" => {
            let words = changes.removed_words.max(changes.added_words);
            if words == 0 {
                "El mismo texto.".to_string()
            } else if words == 1 {
                "Prácticamente el mismo texto; cambia una palabra suelta.".to_string()
            } else {
                format!("Prácticamente el mismo texto; cambian {words} palabras sueltas.")
            }
        }
        "signed-copy" => "El mismo texto, y uno de los dos tiene la firma verificable de una parte.".to_string(),
        "preview" => "El mismo texto, y uno de los dos tiene las marcas de una vista previa.".to_string(),
        "contained-in" => format!("Uno está casi entero dentro del otro ({percent} % de su texto)."),
        "same-template" | "version" => match changed {
            Some(changed) => format!("Mismo texto de base; {changed}{substitution}."),
            None => "Mismo texto de base.".to_string(),
        },
        _ => match changed {
            Some(changed) => format!("Comparten el {percent} % del texto; {changed}{substitution}."),
            None => format!("Comparten el {percent} % del texto."),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(id: i64, own: usize, matched: usize, marks: &[&str]) -> Side {
        Side {
            content_id: id,
            own_words: own,
            matched,
            recognized: false,
            marks: marks.iter().map(|m| m.to_string()).collect(),
        }
    }

    fn changes(items: &[(Kind, usize)], data: usize, wording: usize) -> Changes {
        Changes {
            items: items.iter().cloned().collect(),
            data_tokens: data,
            wording_tokens: wording,
            hunks: items.iter().map(|(_, n)| n).sum(),
            swaps: items.iter().filter(|(kind, _)| kind.is_data()).cloned().collect(),
            replaced_data_tokens: data,
            replaced_wording_tokens: wording,
            longest_hunk: 2,
            ..Default::default()
        }
    }

    fn relation(a: &Side, b: &Side, c: &Changes) -> Option<String> {
        let m = measures(a, b, (0, 1, 1));
        propose(a, b, &m, c).map(|p| p.relation)
    }

    #[test]
    fn the_description_counts_what_changes() {
        let c = changes(&[(Kind::Amount, 3), (Kind::Date, 1)], 4, 0);
        assert_eq!(describe("same-template", 0.9, &c), "Mismo texto de base; cambian 3 importes y una fecha.");
        let c = changes(&[(Kind::Wording, 4)], 0, 12);
        assert_eq!(describe("version", 0.9, &c), "Mismo texto de base; cambian la redacción en 4 tramos.");
        let c = changes(&[(Kind::Name, 1)], 1, 0);
        assert_eq!(describe("similar", 0.62, &c), "Comparten el 62 % del texto; cambia un nombre.");
    }

    #[test]
    fn data_makes_a_template_and_wording_a_version() {
        let a = side(1, 100, 90, &[]);
        let b = side(2, 100, 90, &[]);
        assert_eq!(relation(&a, &b, &changes(&[(Kind::Amount, 5)], 10, 2)).as_deref(), Some("same-template"));
        assert_eq!(relation(&a, &b, &changes(&[(Kind::Wording, 3)], 1, 9)).as_deref(), Some("version"));
        let mut swapped = changes(&[(Kind::Wording, 3), (Kind::Identifier, 1)], 1, 9);
        swapped.swaps.insert(Kind::Identifier, 1);
        assert_eq!(relation(&a, &b, &swapped).as_deref(), Some("same-template"));
    }

    #[test]
    fn little_in_common_is_similar_or_nothing() {
        let c = changes(&[(Kind::Wording, 9)], 0, 40);
        assert_eq!(relation(&side(1, 100, 60, &[]), &side(2, 100, 60, &[]), &c).as_deref(), Some("similar"));
        assert_eq!(relation(&side(1, 100, 30, &[]), &side(2, 100, 30, &[]), &c), None);
    }

    #[test]
    fn a_signed_copy_needs_a_verifiable_signature_of_a_party() {
        let plain = side(1, 100, 100, &[]);
        let signed = side(2, 100, 100, &["party-signature"]);
        let sealed = side(2, 100, 100, &["service-seal"]);
        let c = changes(&[], 0, 0);
        let proposal = propose(&plain, &signed, &measures(&plain, &signed, (1, 1, 1)), &c).unwrap();
        assert_eq!(proposal.relation, "signed-copy");
        assert_eq!(proposal.subject, Some(2));
        assert_eq!(proposal.suggested_source, Some(2));
        // A service's seal alone proves nothing.
        assert_ne!(relation(&plain, &sealed, &c).as_deref(), Some("signed-copy"));
    }

    #[test]
    fn the_stamp_of_a_signing_service_in_a_sealed_file_is_a_partys_signature() {
        // P-485: all that tells them apart is one block the sealed one adds,
        // a name with a date and a time.
        let plain = side(1, 100, 100, &[]);
        let sealed = side(2, 120, 100, &["service-seal"]);
        let stamped = Changes { hunks: 1, stamps_b: 1, added_words: 20, longest_hunk: 20, largest_block: 20, ..Default::default() };
        let proposal = propose(&plain, &sealed, &measures(&plain, &sealed, (1, 1, 1)), &stamped).unwrap();
        assert_eq!((proposal.relation.as_str(), proposal.subject, proposal.rule.as_str()), ("signed-copy", Some(2), "service-seal-stamp"));
        assert_eq!(proposal.suggested_source, Some(2));
        // Without the seal of a service the stamp proves nothing.
        let unsealed = side(2, 120, 100, &[]);
        assert_eq!(relation(&plain, &unsealed, &stamped).as_deref(), Some("version"));
        // Another change besides the stamp, or the stamp on the other side:
        // not a signed copy.
        let more = Changes { hunks: 2, ..stamped.clone() };
        assert_ne!(relation(&plain, &sealed, &more).as_deref(), Some("signed-copy"));
        let elsewhere = Changes { stamps_a: 1, stamps_b: 0, ..stamped.clone() };
        assert_ne!(relation(&plain, &sealed, &elsewhere).as_deref(), Some("signed-copy"));
    }

    #[test]
    fn other_key_data_take_a_version_down_to_the_same_template_and_leave_no_similar_pair() {
        let a = side(1, 100, 90, &[]);
        let b = side(2, 100, 90, &[]);
        // A clause reworded: a version, until the key data say otherwise.
        let reworded = changes(&[(Kind::Wording, 3)], 1, 9);
        let version = propose(&a, &b, &measures(&a, &b, (0, 1, 1)), &reworded).unwrap();
        assert_eq!(version.relation, "version");
        assert_eq!(with_key_data(version.clone(), &[]), Some(version.clone()), "the same key data: still a version");
        let other = with_key_data(version, &[Key::Parties, Key::Amounts]).unwrap();
        assert_eq!(other.relation, "same-template");
        assert_eq!(other.rule, "different-key-data");
        assert_eq!(other.description, "Mismo texto de base, pero son documentos distintos: cambian las partes y los importes.");
        // Less in common and other parties: no pair at all.
        let little = changes(&[(Kind::Wording, 9)], 0, 40);
        let (c, d) = (side(1, 100, 60, &[]), side(2, 100, 60, &[]));
        let similar = propose(&c, &d, &measures(&c, &d, (0, 1, 1)), &little).unwrap();
        assert_eq!(similar.relation, "similar");
        assert_eq!(with_key_data(similar, &[Key::Parties]), None);
        // A signed copy is left alone.
        let plain = side(1, 100, 100, &[]);
        let signed = side(2, 100, 100, &["party-signature"]);
        let copy = propose(&plain, &signed, &measures(&plain, &signed, (1, 1, 1)), &changes(&[], 0, 0)).unwrap();
        assert_eq!(with_key_data(copy, &[Key::Dates]).unwrap().relation, "signed-copy");
    }

    #[test]
    fn short_inside_long_is_contained() {
        let annex = side(1, 100, 95, &[]);
        let contract = side(2, 400, 95, &[]);
        let c = changes(&[(Kind::Wording, 1)], 0, 300);
        let proposal = propose(&annex, &contract, &measures(&annex, &contract, (1, 1, 1)), &c).unwrap();
        assert_eq!(proposal.relation, "contained-in");
        assert_eq!(proposal.subject, Some(1));
    }

    #[test]
    fn a_version_shares_much_in_both_directions() {
        let reworded = changes(&[(Kind::Wording, 3)], 0, 9);
        // Two real versions: each has most of its text in the other.
        let (a, b) = (side(1, 200, 180, &[]), side(2, 220, 185, &[]));
        assert_eq!(relation(&a, &b, &reworded).as_deref(), Some("version"));
        // A short text mostly inside a long one that shares little back:
        // only similar, not a version.
        let (short, long) = (side(1, 35, 29, &[]), side(2, 341, 29, &[]));
        let proposal = propose(&short, &long, &measures(&short, &long, (0, 1, 1)), &reworded).unwrap();
        assert_eq!(proposal.relation, "similar");
        assert_eq!(proposal.certainty, "low");
        // The same, read from an image: the lower threshold does not make it a version.
        let read = Side { recognized: true, ..side(1, 35, 27, &[]) };
        assert_eq!(relation(&read, &long, &reworded).as_deref(), Some("similar"));
        // Almost whole inside, though not much shorter: one inside the other.
        let (inner, outer) = (side(1, 100, 92, &[]), side(2, 130, 92, &[]));
        let proposal = propose(&inner, &outer, &measures(&inner, &outer, (0, 1, 1)), &reworded).unwrap();
        assert_eq!(proposal.relation, "contained-in");
        assert_eq!(proposal.subject, Some(1));
    }
}
