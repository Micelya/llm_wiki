//! A signing certificate cut apart from what it certifies (D-607, D-608).
//!
//! When the cutter splits a file into parts (D-560), the certificate a
//! signing service adds at the end becomes a part of its own, and the
//! signatures it shows leave the document they certify. The catalog says
//! which relation holds between such parts: a part with
//! all of its `certifierMarks` certifies the parts of the same file it
//! names, by a long identifier found in both or by their first line;
//! when it names none, the part before it. The certified part gets the
//! certifier's `gives` mark, and pass 6 records the relation between the
//! two (`by_type.rs`). The rules are in `between-parts.json`.
//!
//! Nothing is inherited from the file: an annex of a signed filing is
//! not certified by anything, and stays unsigned (P-408).

use std::collections::BTreeSet;

use super::marks::Mark;
use super::text::padded;
use super::types::{Catalog, WithinFile};

/// A part of a file, in the order of the file.
pub struct Part<'a> {
    pub version_id: i64,
    /// Its text, with the `## Page N` headings of the file.
    pub text: &'a str,
    /// What the catalog read in it, with the seals of its file as context.
    pub marks: &'a [Mark],
}

/// What the certificates of its file change in a part.
#[derive(Debug, Default, PartialEq)]
pub struct Certification {
    /// The marks it gets from the parts that certify it.
    pub gets: Vec<Mark>,
    /// The kinds of mark it gives to the parts it certifies: they are
    /// theirs, not its own (the signatures a certificate shows are the
    /// signatures of what it certifies).
    pub gives: Vec<String>,
}

/// What the parts of its file that certify it, or that it certifies,
/// change in the part `version_id`.
pub fn certification(catalog: &Catalog, version_id: i64, parts: &[Part<'_>]) -> Certification {
    let mut out = Certification::default();
    out.gets = marks_for(catalog, version_id, parts);
    let Some(me) = parts.iter().position(|p| p.version_id == version_id) else {
        return out;
    };
    for rule in &catalog.within_file {
        if certified(rule, parts).iter().any(|(_, certifier)| *certifier == me) && !out.gives.contains(&rule.gives) {
            out.gives.push(rule.gives.clone());
        }
    }
    out
}

/// The marks the part `version_id` gets from the parts of its file that
/// certify it.
pub fn marks_for(catalog: &Catalog, version_id: i64, parts: &[Part<'_>]) -> Vec<Mark> {
    let mut out = Vec::new();
    let Some(me) = parts.iter().position(|p| p.version_id == version_id) else {
        return out;
    };
    for rule in &catalog.within_file {
        for (certified, certifier) in certified(rule, parts) {
            if certified != me {
                continue;
            }
            let Some(given) = parts[certifier].marks.iter().find(|m| m.kind == rule.gives) else { continue };
            let mark = Mark {
                kind: rule.gives.clone(),
                doc_type: given.doc_type.clone(),
                variant: given.variant.clone(),
                stage: None,
                signal: rule.id.clone(),
                page: None,
                value: Some(parts[certifier].version_id.to_string()),
            };
            if !out.contains(&mark) {
                out.push(mark);
            }
        }
    }
    out
}

/// The parts a certifier certifies, as (certified, certifier) indexes.
pub fn certified(rule: &WithinFile, parts: &[Part<'_>]) -> Vec<(usize, usize)> {
    let is_certifier = |part: &Part<'_>| rule.certifier_marks.iter().all(|kind| part.marks.iter().any(|m| &m.kind == kind));
    let mut out = Vec::new();
    for (c, certifier) in parts.iter().enumerate() {
        if !is_certifier(certifier) {
            continue;
        }
        let ids = identifiers(certifier.text);
        let said = padded(certifier.text);
        let named: Vec<usize> = parts
            .iter()
            .enumerate()
            .filter(|(i, part)| *i != c && !is_certifier(part))
            .filter(|(_, part)| {
                rule.named_by.iter().any(|by| match by.as_str() {
                    "identifier" => !ids.is_disjoint(&identifiers(part.text)),
                    "title" => title(part.text).is_some_and(|t| said.contains(&t)),
                    _ => false,
                })
            })
            .map(|(i, _)| i)
            .collect();
        if !named.is_empty() {
            out.extend(named.into_iter().map(|i| (i, c)));
        } else if rule.otherwise.as_deref() == Some("previous") {
            if let Some(previous) = (0..c).rev().find(|&i| !is_certifier(&parts[i])) {
                out.push((previous, c));
            }
        }
    }
    out
}

/// Long tokens of letters and digits: what names one thing only (the
/// identifier of a signing process, of a signature). Hyphens do not count,
/// so the same identifier written with and without them is one.
fn identifiers(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .map(|run| run.replace('-', "").to_ascii_uppercase())
        .filter(|id| {
            let digits = id.chars().filter(char::is_ascii_digit).count();
            id.len() >= 12 && digits >= 4 && id.len() - digits >= 2
        })
        .collect()
}

/// The first line of a part that can name it: words, no identifier.
fn title(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("## Page"))
        .find(|line| identifiers(line).is_empty() && line.split_whitespace().filter(|w| w.chars().any(char::is_alphabetic)).count() >= 2)
        .map(padded)
        .filter(|title| title.trim().len() >= 12)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(kind: &str) -> Mark {
        Mark {
            kind: kind.into(),
            doc_type: "tipo".into(),
            variant: None,
            stage: None,
            signal: format!("senal-{kind}"),
            page: None,
            value: None,
        }
    }

    fn signed_by(catalog: &Catalog, parts: &[Part<'_>]) -> Vec<bool> {
        parts.iter().map(|p| !marks_for(catalog, p.version_id, parts).is_empty()).collect()
    }

    #[test]
    fn a_certificate_that_names_nothing_certifies_the_part_before_it() {
        let catalog = Catalog::builtin();
        let certificate = [mark("signing-audit"), mark("party-signature")];
        let parts = [
            Part { version_id: 1, text: "## Page 1\n\nContrato de servicios.", marks: &[] },
            Part { version_id: 2, text: "## Page 2\n\nAnexo con su propio texto.", marks: &[] },
            Part { version_id: 3, text: "## Page 3\n\nResumen del proceso de firma.", marks: &certificate },
        ];
        assert_eq!(signed_by(&catalog, &parts), [false, true, false]);
        let given = marks_for(&catalog, 2, &parts);
        assert_eq!(given[0].kind, "party-signature");
        assert_eq!(given[0].signal, "firmado-segun-su-constancia");
        assert_eq!(given[0].value.as_deref(), Some("3"));
        // The certificate gives its signatures away; the others keep theirs.
        assert_eq!(certification(&catalog, 3, &parts).gives, ["party-signature"]);
        assert!(certification(&catalog, 1, &parts).gives.is_empty());
        // A summary that shows no signature certifies nothing.
        let unsigned = [mark("signing-audit")];
        let parts = [
            Part { version_id: 1, text: "## Page 1\n\nContrato de servicios.", marks: &[] },
            Part { version_id: 3, text: "## Page 2\n\nResumen del proceso.", marks: &unsigned },
        ];
        assert_eq!(signed_by(&catalog, &parts), [false, false]);
        // Nothing before it: nothing certified.
        let parts = [
            Part { version_id: 3, text: "## Page 1\n\nResumen del proceso de firma.", marks: &certificate },
            Part { version_id: 1, text: "## Page 2\n\nContrato de servicios.", marks: &[] },
        ];
        assert_eq!(signed_by(&catalog, &parts), [false, false]);
    }

    #[test]
    fn a_certificate_certifies_the_part_it_names() {
        let catalog = Catalog::builtin();
        let certificate = [mark("signing-audit"), mark("party-signature")];
        // By the identifier of the process, with or without hyphens.
        let parts = [
            Part { version_id: 1, text: "## Page 1\n\nIdentificador 0A1B2C3D-4E5F-6071\nContrato principal.", marks: &[] },
            Part { version_id: 2, text: "## Page 2\n\nNota sin relación.", marks: &[] },
            Part { version_id: 3, text: "## Page 3\n\nProceso 0A1B2C3D4E5F6071 completado.", marks: &certificate },
        ];
        assert_eq!(signed_by(&catalog, &parts), [true, false, false]);
        // By the first line of the part.
        let parts = [
            Part { version_id: 1, text: "## Page 1\n\nAcuerdo de confidencialidad\nTexto.", marks: &[] },
            Part { version_id: 2, text: "## Page 2\n\nPlanilla de horarios\nTexto.", marks: &[] },
            Part { version_id: 3, text: "## Page 3\n\nAsunto: Acuerdo de Confidencialidad.pdf", marks: &certificate },
        ];
        assert_eq!(signed_by(&catalog, &parts), [true, false, false]);
        // A date or a short number names nothing.
        assert!(identifiers("2024-01-31 · 12345678 · 20-12345678-9").is_empty());
    }

    #[test]
    fn the_annex_of_a_signed_filing_is_not_signed() {
        let catalog = Catalog::builtin();
        // The filing signed by a party, its annexes, and no certificate.
        let signed = [mark("party-signature")];
        let parts = [
            Part { version_id: 1, text: "## Page 1\n\nEscrito presentado.", marks: &signed },
            Part { version_id: 2, text: "## Page 2\n\nAnexo documental.", marks: &[] },
            Part { version_id: 3, text: "## Page 3\n\nOtro anexo.", marks: &[] },
        ];
        assert_eq!(signed_by(&catalog, &parts), [false, false, false]);
    }
}
