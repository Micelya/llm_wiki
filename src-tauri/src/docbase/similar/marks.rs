//! Pass 2: what can be verified about a document without a model.
//!
//! The catalog says what to look for (`types.rs`); this reads the
//! document: phrases of its text, the shape of its identifiers, whether
//! it is a form, and the digital signatures embedded in the PDF.
//!
//! An embedded signature is read with pdfium and its certificate names
//! are taken from the signature's own bytes (PKCS #7). A signing service
//! seals every copy it issues, so a signature whose certificate names a
//! service of the catalog is that service's seal, not the signature of a
//! party (catalog: "PDF emitido por un servicio de firma electrónica").

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::text::{padded, Prepared};
use super::types::{has_shape, Catalog, Signal};

/// Something verified about a document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mark {
    /// What it means for the comparison: "preview", "party-signature",
    /// "signing-audit"… (the signal's `mark`), or "signal".
    pub kind: String,
    pub doc_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage: Option<String>,
    pub signal: String,
    /// Page where it was found (from 1), for signals read page by page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<usize>,
    /// What was found, when it is a value (a shipment number, the names
    /// in a certificate).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// A digital signature embedded in a PDF.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfSignature {
    /// Common names and organizations in its certificates.
    pub names: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
}

/// What is known of a document before reading the catalog.
pub struct Facts<'a> {
    pub pages: &'a [Vec<&'a str>],
    pub has_fields: bool,
    pub signatures: &'a [PdfSignature],
}

/// What the catalog found in a document, and what it says to leave out
/// of the comparison.
#[derive(Debug, Default)]
pub struct Reading {
    pub marks: Vec<Mark>,
    /// Pages (from 0) that are not content of the document.
    pub skip_pages: Vec<usize>,
    /// Lines with these phrases (normalized, padded) are not compared.
    pub drop_phrases: Vec<String>,
}

/// Reads a document with the catalog.
pub fn read(catalog: &Catalog, facts: &Facts<'_>) -> Reading {
    let mut reading = Reading::default();
    let page_texts: Vec<String> = facts.pages.iter().map(|page| padded(&page.join("\n"))).collect();
    let whole = page_texts.join(" ");
    let tokens = Prepared::new(facts.pages, &[], &[]).raws;
    let service_names: Vec<(String, String)> = catalog
        .signing_services()
        .flat_map(|variant| variant.certificate_names.iter().map(move |name| (variant.id.clone(), name.to_lowercase())))
        .collect();
    let service_of = |signature: &PdfSignature| -> Option<String> {
        signature.names.iter().find_map(|name| {
            let name = name.to_lowercase();
            service_names
                .iter()
                .find(|(_, word)| name.contains(word.as_str()))
                .map(|(variant, _)| variant.clone())
        })
    };

    for doc_type in &catalog.types {
        let mut fired: Vec<&str> = Vec::new();
        // Signals that require others go after them.
        let mut ordered: Vec<&Signal> = doc_type.signals.iter().filter(|s| s.requires.is_empty()).collect();
        ordered.extend(doc_type.signals.iter().filter(|s| !s.requires.is_empty()));
        for signal in ordered {
            if !signal.requires.iter().all(|id| fired.contains(&id.as_str())) {
                continue;
            }
            let mut found: Vec<(Option<usize>, Option<String>, Option<String>)> = Vec::new(); // page, value, variant
            let mut conditions = 0;
            let mut holds = true;

            if let Some(wanted) = signal.form_fields {
                conditions += 1;
                holds &= facts.has_fields == wanted;
            }
            if let Some(kind) = &signal.pdf_signature {
                conditions += 1;
                let matching: Vec<(&PdfSignature, Option<String>)> = facts
                    .signatures
                    .iter()
                    .map(|signature| (signature, service_of(signature)))
                    .filter(|(_, service)| match kind.as_str() {
                        "service" => service.is_some(),
                        "party" => service.is_none(),
                        _ => false,
                    })
                    .collect();
                holds &= !matching.is_empty();
                for (signature, service) in matching {
                    found.push((None, Some(signature.names.join("; ")), service));
                }
            }
            if !signal.shapes.is_empty() {
                conditions += 1;
                let values: Vec<String> = tokens
                    .iter()
                    .filter(|token| signal.shapes.iter().any(|shape| has_shape(&token.text, shape)))
                    .map(|token| token.text.clone())
                    .collect();
                holds &= !values.is_empty();
                let mut seen = Vec::new();
                for value in values {
                    if !seen.contains(&value) {
                        seen.push(value.clone());
                        found.push((None, Some(value), None));
                    }
                }
            }
            if !signal.phrases.is_empty() {
                conditions += 1;
                let phrases: Vec<String> = signal.phrases.iter().map(|p| padded(p)).collect();
                let needed = signal.needed_phrases();
                let count = |text: &str| phrases.iter().filter(|phrase| text.contains(phrase.as_str())).count();
                if signal.on_page() {
                    let pages: Vec<usize> =
                        (0..page_texts.len()).filter(|&index| count(&page_texts[index]) >= needed).collect();
                    holds &= !pages.is_empty();
                    for page in pages {
                        found.push((Some(page), None, None));
                    }
                } else {
                    holds &= count(&whole) >= needed;
                }
            }
            if conditions == 0 || !holds {
                continue;
            }
            fired.push(signal.id.as_str());
            if found.is_empty() {
                found.push((None, None, None));
            }
            for (page, value, variant) in found {
                if signal.exclude_page {
                    if let Some(page) = page {
                        if !reading.skip_pages.contains(&page) {
                            reading.skip_pages.push(page);
                        }
                    }
                }
                reading.marks.push(Mark {
                    kind: signal.mark.clone().unwrap_or_else(|| "signal".to_string()),
                    doc_type: doc_type.id.clone(),
                    variant: variant.or_else(|| signal.variant.clone()),
                    stage: signal.stage.clone(),
                    signal: signal.id.clone(),
                    page: page.map(|p| p + 1),
                    value,
                });
            }
            if signal.drop_lines {
                for phrase in &signal.phrases {
                    let phrase = padded(phrase);
                    if !reading.drop_phrases.contains(&phrase) {
                        reading.drop_phrases.push(phrase);
                    }
                }
            }
        }
    }
    // Never leave a document without pages to compare.
    if reading.skip_pages.len() >= facts.pages.len() {
        reading.skip_pages.clear();
    }
    reading.skip_pages.sort_unstable();
    reading
}

/// The digital signatures embedded in a PDF; none for other files.
pub fn pdf_signatures(path: &Path) -> Result<Vec<PdfSignature>, String> {
    let is_pdf = path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pdf"));
    if !is_pdf {
        return Ok(Vec::new());
    }
    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(path, None)
        .map_err(|e| format!("Failed to open PDF '{}': {e}", path.display()))?;
    let mut out = Vec::new();
    for signature in doc.signatures().iter() {
        out.push(PdfSignature {
            names: certificate_names(&signature.bytes()),
            reason: signature.reason().filter(|r| !r.trim().is_empty()),
            date: signature.signing_date().filter(|d| !d.trim().is_empty()),
        });
    }
    Ok(out)
}

/// Common names (2.5.4.3) and organizations (2.5.4.10) found in the DER
/// bytes of a PKCS #7 signature, in order, without repeats.
pub fn certificate_names(der: &[u8]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let mut index = 0;
    while index + 7 < der.len() {
        let is_name_oid = der[index] == 0x06
            && der[index + 1] == 0x03
            && der[index + 2] == 0x55
            && der[index + 3] == 0x04
            && (der[index + 4] == 0x03 || der[index + 4] == 0x0a);
        if !is_name_oid {
            index += 1;
            continue;
        }
        let tag = der[index + 5];
        let (length, start) = match der[index + 6] {
            short if short < 0x80 => (short as usize, index + 7),
            0x81 if index + 7 < der.len() => (der[index + 7] as usize, index + 8),
            _ => {
                index += 1;
                continue;
            }
        };
        let Some(bytes) = der.get(start..start + length) else {
            break;
        };
        let value = match tag {
            0x0c | 0x13 | 0x16 | 0x14 => String::from_utf8_lossy(bytes).to_string(),
            0x1e => {
                let units: Vec<u16> = bytes.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                String::from_utf16_lossy(&units)
            }
            _ => {
                index += 1;
                continue;
            }
        };
        let value = value.trim().to_string();
        if !value.is_empty() && !names.contains(&value) {
            names.push(value);
        }
        index = start + length;
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::similar::text::pages;

    fn read_text(text: &str, has_fields: bool, signatures: &[PdfSignature]) -> Reading {
        let pages = pages(text);
        read(&Catalog::builtin(), &Facts { pages: &pages, has_fields, signatures })
    }

    fn kinds(reading: &Reading) -> Vec<&str> {
        reading.marks.iter().map(|m| m.kind.as_str()).collect()
    }

    #[test]
    fn the_preview_watermark_is_a_mark() {
        let reading = read_text("CARTA DOCUMENTO\nVista previa - sin valor legal\nIntimo a usted…", false, &[]);
        assert!(kinds(&reading).contains(&"preview"));
        let mark = reading.marks.iter().find(|m| m.kind == "preview").unwrap();
        assert_eq!(mark.doc_type, "carta-documento");
        assert_eq!(mark.stage.as_deref(), Some("vista-previa"));
        assert!(read_text("Una carta cualquiera.", false, &[]).marks.is_empty());
    }

    #[test]
    fn a_shipment_number_is_recognised_by_its_shape() {
        let reading = read_text("Envío XY123456789AR del correo", false, &[]);
        let mark = reading.marks.iter().find(|m| m.kind == "shipment-number").unwrap();
        assert_eq!(mark.value.as_deref(), Some("XY123456789AR"));
        assert_eq!(mark.variant.as_deref(), Some("correo-argentino"));
    }

    #[test]
    fn a_signing_history_page_is_marked_and_left_out() {
        let text = "## Page 1\n\nContrato de locación entre las partes.\n\n## Page 2\n\nFinal audit report\nTransaction ID abc\nHistorial";
        let reading = read_text(text, false, &[]);
        assert!(kinds(&reading).contains(&"signing-audit"));
        assert_eq!(reading.skip_pages, vec![1]);
    }

    #[test]
    fn a_margin_stamp_is_dropped_from_the_comparison() {
        let reading = read_text("Docusign Envelope ID: 0000-AAAA\nTexto", false, &[]);
        assert_eq!(reading.drop_phrases, vec![" envelope id ".to_string()]);
    }

    #[test]
    fn embedded_signatures_tell_the_service_seal_from_a_party() {
        let seal = PdfSignature { names: vec!["DocuSign, Inc.".into(), "Some CA".into()], ..Default::default() };
        let party = PdfSignature { names: vec!["Una Persona".into(), "Some CA".into()], ..Default::default() };
        assert_eq!(kinds(&read_text("texto", false, &[seal.clone()])), ["service-seal"]);
        let both = read_text("texto", false, &[seal, party]);
        assert!(kinds(&both).contains(&"service-seal"));
        assert!(kinds(&both).contains(&"party-signature"));
    }

    #[test]
    fn a_form_is_marked_when_it_has_fields() {
        assert_eq!(kinds(&read_text("Ficha", true, &[])), ["form"]);
        assert!(read_text("Ficha", false, &[]).marks.is_empty());
    }

    #[test]
    fn certificate_names_are_read_from_der() {
        let mut der = vec![0x30, 0x10];
        der.extend([0x06, 0x03, 0x55, 0x04, 0x03, 0x0c, 0x05]);
        der.extend(b"Firma");
        der.extend([0x06, 0x03, 0x55, 0x04, 0x0a, 0x13, 0x03]);
        der.extend(b"Org");
        der.extend([0x00, 0x00]);
        assert_eq!(certificate_names(&der), ["Firma", "Org"]);
    }
}
