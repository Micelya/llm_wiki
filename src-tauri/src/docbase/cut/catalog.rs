//! The vocabulary of the cutter (`resources/catalog/cut.json`, R-26): its
//! text patterns, thresholds, formulas, types and the question to the
//! model. Nothing of it names a case (R-24).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

const CATALOG: &str = include_str!("../../../resources/catalog/cut.json");

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Raw {
    version: u32,
    patterns: RawPatterns,
    thresholds: Thresholds,
    openers: Phrases,
    closers: Phrases,
    types: RawTypes,
    ask: AskCatalog,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawPatterns {
    page_of: String,
    document_code: String,
    sheet_serial: String,
    machine_zone: String,
}

#[derive(Debug, Deserialize)]
struct Phrases {
    phrases: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawTypes {
    machine_zone: String,
    rules: Vec<(String, Vec<String>)>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thresholds {
    pub photo_max_dpi: u32,
    pub capture_dpi_share: f64,
    pub paper_share: f64,
    pub tail_lines: usize,
    pub head_lines: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskCatalog {
    pub version: String,
    pub image_long_side: u32,
    pub prompt: String,
    pub types: BTreeMap<String, String>,
}

pub struct Patterns {
    pub page_of: Regex,
    pub document_code: Regex,
    pub sheet_serial: Regex,
    pub machine_zone: Regex,
}

pub struct CutCatalog {
    pub version: u32,
    pub patterns: Patterns,
    pub thresholds: Thresholds,
    /// Folded (no accents, lower case), as the text is compared.
    pub openers: Vec<String>,
    pub closers: Vec<String>,
    pub machine_zone_type: String,
    pub types: Vec<(String, Vec<String>)>,
    pub ask: AskCatalog,
}

pub fn catalog() -> &'static CutCatalog {
    static CUT: OnceLock<CutCatalog> = OnceLock::new();
    CUT.get_or_init(|| {
        let raw: Raw = serde_json::from_str(CATALOG).expect("the catalog cut.json is valid");
        let regex = |p: &str| Regex::new(p).expect("the patterns of cut.json are valid");
        let folded = |v: Vec<String>| v.iter().map(|p| fold(p)).collect();
        CutCatalog {
            version: raw.version,
            patterns: Patterns {
                page_of: regex(&raw.patterns.page_of),
                document_code: regex(&raw.patterns.document_code),
                sheet_serial: regex(&raw.patterns.sheet_serial),
                machine_zone: regex(&raw.patterns.machine_zone),
            },
            thresholds: raw.thresholds,
            openers: folded(raw.openers.phrases),
            closers: folded(raw.closers.phrases),
            machine_zone_type: raw.types.machine_zone,
            types: raw.types.rules.into_iter().map(|(id, phrases)| (id, folded(phrases))).collect(),
            ask: raw.ask,
        }
    })
}

/// A text as the formulas are compared: lower case, without accents.
pub fn fold(text: &str) -> String {
    text.to_lowercase()
        .nfd()
        .filter(|c| !unicode_normalization::char::is_combining_mark(*c))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalog_reads_and_its_types_are_the_ones_the_question_offers() {
        let c = catalog();
        assert!(c.version >= 1);
        assert_eq!(fold("Señor JUEZ: Será"), "senor juez: sera");
        assert!(c.ask.prompt.contains("{types}"));
        for (id, _) in &c.types {
            assert!(c.ask.types.contains_key(id), "{id}");
        }
        assert!(c.ask.types.contains_key(&c.machine_zone_type));
    }
}
