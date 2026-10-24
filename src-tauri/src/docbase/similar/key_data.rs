//! The key data of a document, as the base already knows them, and
//! whether two documents differ in them (D-497).
//!
//! In legal documents two texts often share almost everything and are
//! still two documents: a contract for two million with one client and
//! the same contract for twenty with another are not "similar", and much
//! less versions of one. So before pass 1 proposes that two documents are
//! a version of one another, or only similar, it compares their key data,
//! with what the base already has of each one (no model, nothing read
//! again):
//!
//! - **parties** (`Key::Parties`): the persons and organizations it names
//!   with a party role (`matters.partyRoles` of the knowledge vocabulary),
//!   by the entity the base resolved each one to;
//! - **matter** (`Key::Matter`): the matters it names (the vocabulary's
//!   matter kind) and their numbers (a case file number);
//! - **identifiers** (`Key::Identifiers`): the identifiers of its persons
//!   and organizations (a national id, a tax id) and those of its type's
//!   data (an invoice number, a shipment number);
//! - **amounts** (`Key::Amounts`): the amounts of its money claims and of
//!   its type's data;
//! - **dates** (`Key::Dates`): the dates of its type's data (the date of
//!   issue, the due date).
//!
//! **The criterion.** Two documents differ "for real" in one of those
//! when each one has a value the other has not, neither among its own key
//! data nor anywhere in its text. A value only one of them has (a
//! signature block filled in, one more signatory) is not a difference: a
//! value has to be replaced by another, as with the changes of the
//! alignment (D-118). Looking in the other's text too keeps a difference
//! in the reading (the model took a party in one document and not in the
//! other) from counting as a difference in the documents. A key with no
//! value in one of the two says nothing: nothing is compared.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rusqlite::{types::Value as SqlValue, Connection};
use serde::Serialize;

use super::super::sql_err;
use super::data::comparable;
use super::text::{normalized, Prepared};
use super::types;
use crate::docbase::knowledge::vocabulary::{vocabulary, Vocabulary};

/// A number with fewer digits says too little to be looked for by its
/// digits alone (an "20" is in any text): it is looked for by its words.
const MIN_DIGITS: usize = 4;

/// The kinds of key data, in the order they are told.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Key {
    Parties,
    Matter,
    Identifiers,
    Amounts,
    Dates,
}

impl Key {
    /// In a sentence: "cambian las partes y los importes".
    pub fn words(self) -> &'static str {
        match self {
            Key::Parties => "las partes",
            Key::Matter => "el asunto",
            Key::Identifiers => "los identificadores",
            Key::Amounts => "los importes",
            Key::Dates => "las fechas",
        }
    }
}

/// One key datum of a document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Datum {
    key: Key,
    /// What makes two of them the same datum: the entity, the digits, the
    /// value letters and digits only.
    identity: String,
    /// The value as words, to look for in the other text (empty: none).
    words: String,
    /// Its digits, to look for among the other text's numbers.
    digits: String,
}

/// The key data of one document.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct KeyData {
    data: BTreeSet<Datum>,
    /// Every person and organization it names, with any role: a party of
    /// the other named here, as whatever, is not missing from it.
    named: BTreeSet<String>,
}

impl KeyData {
    /// A fingerprint of what the comparison looks at: when it changes, the
    /// pairs of the document are measured again.
    pub fn print(&self) -> String {
        let mut joined = String::new();
        for datum in &self.data {
            joined.push_str(&format!("{:?}\u{1}{}\u{1}{}\u{1}{}\u{2}", datum.key, datum.identity, datum.words, datum.digits));
        }
        for name in &self.named {
            joined.push_str(name);
            joined.push('\u{3}');
        }
        format!("{:016x}", super::text::fnv1a(joined.as_bytes()))
    }

    fn push(&mut self, key: Key, identity: String, value: &str) {
        if identity.len() <= 2 {
            return;
        }
        self.data.insert(Datum { key, identity, words: normalized(value), digits: digits_of(value) });
    }

    fn of(&self, key: Key) -> impl Iterator<Item = &Datum> {
        self.data.iter().filter(move |d| d.key == key)
    }
}

/// What of a text the comparison looks values up in.
#[derive(Debug, Clone, Default)]
pub struct TextIndex {
    /// Its normalized words, padded with spaces: whole words only.
    padded: String,
    /// The digits of each of its tokens ("12.345.678" → "12345678").
    numbers: HashSet<String>,
}

impl TextIndex {
    pub fn new(prepared: &Prepared) -> Self {
        let mut padded = String::from(" ");
        for word in &prepared.words {
            padded.push_str(&word.key);
            padded.push(' ');
        }
        let numbers = prepared.raws.iter().map(|raw| digits_of(&raw.text)).filter(|d| !d.is_empty()).collect();
        TextIndex { padded, numbers }
    }

    fn says(&self, datum: &Datum) -> bool {
        (!datum.words.is_empty() && self.padded.contains(&format!(" {} ", datum.words)))
            || (datum.digits.len() >= MIN_DIGITS && self.numbers.contains(&datum.digits))
    }
}

fn digits_of(value: &str) -> String {
    value.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// The identity of a number: its digits when it has enough, otherwise its
/// letters and digits.
fn number_identity(value: &str) -> String {
    let digits = digits_of(value);
    if digits.len() >= MIN_DIGITS {
        format!("d:{digits}")
    } else {
        format!("c:{}", comparable(value))
    }
}

/// Whether `datum` of one document is also in the other: among its key
/// data, among whom it names, or in its text.
fn present(datum: &Datum, other: (&KeyData, &TextIndex)) -> bool {
    let (data, text) = other;
    data.of(datum.key).any(|d| d.identity == datum.identity)
        || (datum.key == Key::Parties && data.named.contains(&datum.identity))
        || text.says(datum)
}

/// The keys in which `a` and `b` differ for real: each one has a value the
/// other has not (see the module's criterion). Empty: nothing says they
/// are different documents.
pub fn differing(a: (&KeyData, &TextIndex), b: (&KeyData, &TextIndex)) -> Vec<Key> {
    let keys = [Key::Parties, Key::Matter, Key::Identifiers, Key::Amounts, Key::Dates];
    keys.into_iter()
        .filter(|key| {
            let (mut in_a, mut in_b) = (a.0.of(*key).peekable(), b.0.of(*key).peekable());
            if in_a.peek().is_none() || in_b.peek().is_none() {
                return false;
            }
            in_a.any(|datum| !present(datum, b)) && in_b.any(|datum| !present(datum, a))
        })
        .collect()
}

/// The key data of one document.
pub fn load(conn: &Connection, content_id: i64) -> Result<KeyData, String> {
    Ok(load_all(conn, Some(content_id))?.remove(&content_id).unwrap_or_default())
}

/// The key data of every document (or of one), from what the base has.
pub fn load_all(conn: &Connection, only: Option<i64>) -> Result<HashMap<i64, KeyData>, String> {
    load_with(conn, vocabulary(), only)
}

fn load_with(conn: &Connection, words: &Vocabulary, only: Option<i64>) -> Result<HashMap<i64, KeyData>, String> {
    let one = only.map(SqlValue::Integer).unwrap_or(SqlValue::Null);
    let mut out: HashMap<i64, KeyData> = HashMap::new();
    let merged: HashMap<String, String> = {
        let mut statement = conn
            .prepare("SELECT id, merged_into FROM knowledge_entity WHERE merged_into IS NOT NULL")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(sql_err)?
            .collect::<Result<HashMap<String, String>, _>>()
            .map_err(sql_err)?;
        rows
    };
    // An entity merged into another is that other one.
    let entity = |id: &str| -> String {
        let mut current = id.to_string();
        for _ in 0..16 {
            match merged.get(&current) {
                Some(next) if next != &current => current = next.clone(),
                _ => break,
            }
        }
        current
    };
    let matter_kind = words.matters.kind.as_str();
    let party_roles = &words.matters.party_roles;

    // Whom and what it names.
    {
        let mut statement = conn
            .prepare(
                "SELECT content_id, kind, role, entity_id, written, normalized FROM knowledge_mention
                 WHERE ?1 IS NULL OR content_id = ?1",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([&one], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            })
            .map_err(sql_err)?;
        for row in rows {
            let (content, kind, role, entity_id, written, name) = row.map_err(sql_err)?;
            let data = out.entry(content).or_default();
            if kind == matter_kind {
                data.push(Key::Matter, format!("n:{}", normalized(&name)), &written);
                continue;
            }
            let identity = match entity_id {
                Some(id) => format!("e:{}", entity(&id)),
                None => format!("n:{}", normalized(&name)),
            };
            data.named.insert(identity.clone());
            data.named.insert(format!("n:{}", normalized(&name)));
            if party_roles.iter().any(|r| r == &role) {
                data.push(Key::Parties, identity, &written);
            }
        }
    }
    // Their identifiers: of a matter (its number), or of a person or an
    // organization.
    {
        let mut statement = conn
            .prepare(
                "SELECT m.content_id, m.kind, i.value FROM knowledge_identifier i JOIN knowledge_mention m ON m.id = i.mention_id
                 WHERE ?1 IS NULL OR m.content_id = ?1",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([&one], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, kind, value) = row.map_err(sql_err)?;
            let key = if kind == matter_kind { Key::Matter } else { Key::Identifiers };
            out.entry(content).or_default().push(key, number_identity(&value), &value);
        }
    }
    // The amounts of what it says of money.
    {
        let mut statement = conn
            .prepare(
                "SELECT content_id, predicate, coalesce(amount, object_value) FROM knowledge_evidence
                 WHERE coalesce(amount, object_value) IS NOT NULL AND (?1 IS NULL OR content_id = ?1)",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map([&one], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, predicate, value) = row.map_err(sql_err)?;
            if words.predicate(&predicate).is_some_and(|p| p.shape == "amount" || p.is_money()) && !digits_of(&value).is_empty() {
                out.entry(content).or_default().push(Key::Amounts, format!("d:{}", digits_of(&value)), &value);
            }
        }
    }
    // The data of its type, by the kind of each one.
    {
        let catalog = types::catalog();
        let mut statement = conn
            .prepare("SELECT content_id, doc_type, data FROM document_data WHERE ?1 IS NULL OR content_id = ?1")
            .map_err(sql_err)?;
        let rows = statement
            .query_map([&one], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
            .map_err(sql_err)?;
        for row in rows {
            let (content, doc_type, data) = row.map_err(sql_err)?;
            let Some(definition) = catalog.doc_type(&doc_type) else { continue };
            let kinds: BTreeMap<&str, &str> = definition.data.iter().map(|d| (d.id.as_str(), d.kind.as_str())).collect();
            let items: Vec<serde_json::Value> = serde_json::from_str(&data).unwrap_or_default();
            for item in items {
                let (Some(field), Some(value)) = (item["field"].as_str(), item["value"].as_str()) else { continue };
                let entry = out.entry(content).or_default();
                match kinds.get(field).copied() {
                    Some("identifier") => entry.push(Key::Identifiers, number_identity(value), value),
                    Some("amount") if !digits_of(value).is_empty() => entry.push(Key::Amounts, format!("d:{}", digits_of(value)), value),
                    Some("date") => entry.push(Key::Dates, format!("c:{}", comparable(value)), value),
                    _ => {}
                }
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::similar::text::pages;

    fn datum(key: Key, identity: &str, value: &str) -> Datum {
        Datum { key, identity: identity.into(), words: normalized(value), digits: digits_of(value) }
    }

    fn doc(text: &str, data: &[(Key, &str, &str)]) -> (KeyData, TextIndex) {
        let mut key = KeyData::default();
        for (k, identity, value) in data {
            key.data.insert(datum(*k, identity, value));
            if *k == Key::Parties {
                key.named.insert(identity.to_string());
            }
        }
        (key, TextIndex::new(&Prepared::new(&pages(text), &[], &[])))
    }

    fn differ(a: &(KeyData, TextIndex), b: &(KeyData, TextIndex)) -> Vec<Key> {
        differing((&a.0, &a.1), (&b.0, &b.1))
    }

    #[test]
    fn two_contracts_of_one_template_with_other_parties_and_amounts_are_different_documents() {
        let alfa = doc(
            "Contrato de locación entre Alfa Servicios SA y el locatario por USD 2.000.000 por año.",
            &[(Key::Parties, "e:alfa", "Alfa Servicios SA"), (Key::Amounts, "d:2000000", "USD 2.000.000")],
        );
        let beta = doc(
            "Contrato de locación entre Beta Insumos SRL y el locatario por USD 20 por año.",
            &[(Key::Parties, "e:beta", "Beta Insumos SRL"), (Key::Amounts, "d:20", "USD 20")],
        );
        assert_eq!(differ(&alfa, &beta), vec![Key::Parties, Key::Amounts]);
    }

    #[test]
    fn a_value_only_one_has_or_one_the_other_says_in_its_text_is_no_difference() {
        // The signed one adds a signatory: nothing is replaced.
        let draft = doc("Acuerdo entre Alfa Servicios SA y Gamma Obras SA.", &[(Key::Parties, "e:alfa", "Alfa Servicios SA")]);
        let signed = doc(
            "Acuerdo entre Alfa Servicios SA y Gamma Obras SA. Firma: Delfina Ocampo.",
            &[(Key::Parties, "e:alfa", "Alfa Servicios SA"), (Key::Parties, "e:delfina", "Delfina Ocampo")],
        );
        assert!(differ(&draft, &signed).is_empty());
        // The reading took another party in each, but both texts name both.
        let one = doc("Acuerdo entre Alfa Servicios SA y Gamma Obras SA.", &[(Key::Parties, "e:alfa", "Alfa Servicios SA")]);
        let other = doc("Acuerdo entre Alfa Servicios SA y Gamma Obras SA.", &[(Key::Parties, "e:gamma", "Gamma Obras SA")]);
        assert!(differ(&one, &other).is_empty());
        // A date written another way is found by its digits.
        let a = doc("Buenos Aires, 15/03/2026.", &[(Key::Dates, "c:15032026", "15/03/2026")]);
        let b = doc("Buenos Aires, 15/03/2026. Recibido.", &[(Key::Dates, "c:15 03 2026", "15 03 2026")]);
        assert!(differ(&a, &b).is_empty());
    }

    #[test]
    fn what_one_of_the_two_does_not_have_is_not_compared() {
        let with = doc("Expediente 12345/2026, Alfa c/ Beta.", &[(Key::Matter, "d:123452026", "12345/2026")]);
        let without = doc("Escrito sin número de expediente.", &[]);
        assert!(differ(&with, &without).is_empty());
    }

    #[test]
    fn the_print_changes_with_what_is_compared() {
        let (a, _) = doc("x", &[(Key::Amounts, "d:1500", "1.500")]);
        let (b, _) = doc("x", &[(Key::Amounts, "d:1600", "1.600")]);
        assert_ne!(a.print(), b.print());
        assert_eq!(a.print(), a.clone().print());
    }
}
