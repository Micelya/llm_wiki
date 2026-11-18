//! «Calidad de la lectura»: the list and the counts by threshold, and what
//! moving the thresholds would change. Every text is invented (R-24); the
//! shares are set by the test on the kept derivative, as the step of the
//! signature would keep them for a scan.

use super::*;
use crate::docbase::knowledge::tests::base_with;
use crate::docbase::thresholds::Thresholds;

const TEXT: &str = "Nota de prueba con palabras inventadas para un documento que no existe en ningún archivo real.";

/// A base of four invented documents, and the version of each by its name.
fn base() -> (crate::docbase::testing::TempDir, DocBase, Vec<(String, i64)>) {
    let (dir, base) = base_with(&[("a.txt", TEXT), ("b.txt", &format!("{TEXT} b")), ("c.txt", &format!("{TEXT} c")), ("d.txt", &format!("{TEXT} d"))]);
    let found = base.reading_quality().unwrap();
    assert_eq!((found.documents, found.measured), (4, 4), "every text document has its share: {found:?}");
    assert!(found.below_one.is_empty(), "a text without scanned pages has every word firm");
    let versions: Vec<(String, i64)> = documents(&base.conn())
        .unwrap()
        .into_iter()
        .map(|(d, ..)| (d.name, d.version_id))
        .collect();
    (dir, base, versions)
}

fn version(versions: &[(String, i64)], name: &str) -> i64 {
    versions.iter().find(|(n, _)| n == name).map(|(_, v)| *v).unwrap()
}

/// Keeps `firm` firm words of 10 for a version, as a reading by zones would give them.
fn share(base: &DocBase, version: i64, firm: usize) {
    let body = serde_json::json!({ "firm": firm, "approximate": 10 - firm - 1, "discarded": 1 }).to_string();
    let n = base
        .conn()
        .execute("UPDATE derivative SET body = ?2 WHERE version_id = ?1 AND kind = ?3", params![version, body, safe_words::KIND])
        .unwrap();
    assert_eq!(n, 1);
}

#[test]
fn the_list_says_each_document_with_its_share_and_the_counts_below_each_value() {
    let (_dir, base, versions) = base();
    share(&base, version(&versions, "a.txt"), 2);
    share(&base, version(&versions, "b.txt"), 4);
    share(&base, version(&versions, "c.txt"), 6);

    let found = base.reading_quality().unwrap();
    assert_eq!((found.quality, found.low_confidence_type), (0.5, 0.3), "the thresholds of now, provisional");
    let listed: Vec<(&str, f64, Option<&str>, usize, usize, usize)> = found
        .below_one
        .iter()
        .map(|d| (d.name.as_str(), d.share, d.verdict.as_deref(), d.firm, d.approximate, d.discarded))
        .collect();
    assert_eq!(
        listed,
        [
            ("a.txt", 0.2, Some("no-legible"), 2, 7, 1),
            ("b.txt", 0.4, Some("no-legible"), 4, 5, 1),
            ("c.txt", 0.6, None, 6, 3, 1)
        ],
        "the worst first; d.txt reads whole and is not listed"
    );
    let first = &found.below_one[0];
    assert!(first.location_id.is_some(), "it can be opened");
    assert!(first.path.ends_with("/a.txt"), "{}", first.path);
    assert_eq!((first.first_page, first.last_page), (None, None), "a file of one document has no pages of a part");

    // Below each tenth, and below each threshold of now (0,3 and 0,5 are tenths already).
    let steps: Vec<(f64, usize, usize)> = found.steps.iter().map(|s| (s.value, s.documents, s.pages)).collect();
    assert_eq!(steps.len(), 10);
    assert_eq!(&steps[..7], &[(0.1, 0, 0), (0.2, 0, 0), (0.3, 1, 1), (0.4, 1, 1), (0.5, 2, 2), (0.6, 2, 2), (0.7, 3, 3)]);
    assert_eq!(steps[9], (1.0, 3, 3));

    // With other thresholds, the verdicts follow them; a value between the tenths is listed too.
    base.set_thresholds(&Thresholds { quality: 0.3, low_confidence_type: 0.45, ..Thresholds::default() }).unwrap();
    let found = base.reading_quality().unwrap();
    let verdicts: Vec<Option<&str>> = found.below_one.iter().map(|d| d.verdict.as_deref()).collect();
    assert_eq!(verdicts, [Some("no-legible"), Some("low-confidence"), None]);
    assert!(found.steps.iter().any(|s| s.value == 0.45 && s.documents == 2));

    // A document without its share yet, and one whose reading gave no word, go apart.
    base.conn()
        .execute("DELETE FROM derivative WHERE version_id = ?1 AND kind = ?2", params![version(&versions, "d.txt"), safe_words::KIND])
        .unwrap();
    base.conn()
        .execute(
            "UPDATE derivative SET body = '{\"firm\":0,\"approximate\":0,\"discarded\":0}' WHERE version_id = ?1 AND kind = ?2",
            params![version(&versions, "c.txt"), safe_words::KIND],
        )
        .unwrap();
    let found = base.reading_quality().unwrap();
    assert_eq!(found.measured, 2);
    assert_eq!(found.unmeasured.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["d.txt"]);
    assert_eq!(found.without_text.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["c.txt"]);
}

#[test]
fn moving_the_thresholds_says_which_documents_change_side_and_how_many_pages_are_written_again() {
    let (_dir, base, versions) = base();
    share(&base, version(&versions, "a.txt"), 2);
    share(&base, version(&versions, "b.txt"), 4);
    share(&base, version(&versions, "c.txt"), 6);
    // Two people named in c.txt, as the reading of entities keeps them.
    for (reference, written) in [("p1", "Persona Inventada Uno"), ("p2", "Persona Inventada Dos")] {
        base.conn()
            .execute(
                "INSERT INTO knowledge_mention (version_id, ref, kind, written, normalized, role, quote) VALUES (?1, ?2, 'persona', ?3, lower(?3), 'parte', ?3)",
                params![version(&versions, "c.txt"), reference, written],
            )
            .unwrap();
    }

    // The same thresholds: nothing changes.
    assert_eq!(base.confidence_effect(0.5, 0.3).unwrap(), ConfidenceEffect::default());
    // «No legible» down to 0,3: b.txt (40 %) stops being of low confidence.
    let down = base.confidence_effect(0.3, 0.3).unwrap();
    assert_eq!((down.to_low, down.from_low, down.from_low_pages, down.pages_written_again), (0, 1, 1, 0));
    // The type of low confidence up to 0,7: c.txt (60 %) becomes one, and the two people it names are written again.
    let up = base.confidence_effect(0.5, 0.7).unwrap();
    assert_eq!((up.to_low, up.to_low_pages, up.from_low, up.pages_written_again), (1, 1, 0, 2));
    // The higher of the two decides: «no legible» at 0,7 is the same.
    assert_eq!(base.confidence_effect(0.7, 0.3).unwrap(), up);
}
