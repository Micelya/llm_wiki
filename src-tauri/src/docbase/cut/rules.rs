//! The rules of the cutter, without a model (D-556). For each step from
//! one page to the next they say `continue`, `new` or `doubtful`, and the
//! rule that decided. In order, the first that applies decides:
//!
//! 1. `blank`: the next page is blank or has only the footer: continue.
//! 2. `page_of`: «página x de y» on the next page with x > 1: continue;
//!    x = 1, or the page before was the last («y de y»): new.
//! 3. `doc_code`: both pages carry the same document code: continue;
//!    both carry codes and share none: new.
//! 4. `serial`: correlative notarial sheet serials (same letters, the
//!    number plus one): continue.
//! 5. `photo_pair`: two photographed pages (under `photoMaxDpi`) of the
//!    same page size and the same image pixels, the two sides of a card:
//!    continue.
//! 6. `making`: one page is a scan and the other is digital: new.
//! 7. `capture`: two scans of another resolution (more than
//!    `captureDpiShare`) or another color space: new.
//! 8. `paper`: two scans of the same resolution whose page differs in
//!    orientation or by more than `paperShare` in width or height: new.
//! 9. `closer` / `opener`: the page before ends with a closing formula, or
//!    the next one starts with an opening formula: new.
//! 10. `none`: doubtful (it counts as continue until the model answers).
//!
//! Then, over the whole file, `uniform` (the even file, `uniform` in the
//! catalog): when no step starts another document by a rule and every
//! page that is not blank is made the same way, on paper of the same
//! orientation and size, a step `none` the model did not answer goes on
//! (D-603). An answer of the model weighs more (D-615), and a step after
//! a page that ends with a signature stays doubtful (D-616).

use super::catalog::catalog;
use super::signals::{Making, PageSignals};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Continue,
    New,
    Doubtful,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Continue => "continue",
            Verdict::New => "new",
            Verdict::Doubtful => "doubtful",
        }
    }

    pub fn parse(s: &str) -> Verdict {
        match s {
            "continue" => Verdict::Continue,
            "new" => Verdict::New,
            _ => Verdict::Doubtful,
        }
    }
}

fn differs(a: f64, b: f64, share: f64) -> bool {
    (a - b).abs() > share * a.max(b)
}

fn has_any<'a>(text: &str, phrases: &'a [String]) -> Option<&'a String> {
    phrases.iter().find(|p| text.contains(p.as_str()))
}

fn serial_next(a: &[(String, u64)], b: &[(String, u64)]) -> bool {
    a.iter().any(|(la, na)| b.iter().any(|(lb, nb)| la == lb && *nb == na + 1))
}

/// The verdict on the step from `prev` to `next`, and the rule that gave it.
pub fn step(prev: &PageSignals, next: &PageSignals) -> (Verdict, &'static str) {
    let t = &catalog().thresholds;
    if next.making == Making::Blank {
        return (Verdict::Continue, "blank");
    }
    if let Some((x, _)) = next.page_of.first() {
        return if *x > 1 { (Verdict::Continue, "page_of") } else { (Verdict::New, "page_of") };
    }
    if let Some((x, y)) = prev.page_of.last() {
        if x == y && *y > 1 {
            return (Verdict::New, "page_of");
        }
    }
    if !prev.codes.is_empty() && !next.codes.is_empty() {
        return if prev.codes.intersection(&next.codes).next().is_some() {
            (Verdict::Continue, "doc_code")
        } else {
            (Verdict::New, "doc_code")
        };
    }
    if serial_next(&prev.serials, &next.serials) {
        return (Verdict::Continue, "serial");
    }
    let both_scans = match (&prev.image, &next.image) {
        (Some(a), Some(b)) if prev.making == Making::Scan && next.making == Making::Scan => Some((a, b)),
        _ => None,
    };
    if let Some((a, b)) = both_scans {
        if a.dpi < t.photo_max_dpi && b.dpi < t.photo_max_dpi && a.pixels == b.pixels && a.pixels != (0, 0) && prev.size == next.size {
            return (Verdict::Continue, "photo_pair");
        }
    }
    let scan_and_digital = |a: Making, b: Making| a == Making::Scan && b == Making::Digital;
    if scan_and_digital(prev.making, next.making) || scan_and_digital(next.making, prev.making) {
        return (Verdict::New, "making");
    }
    if let Some((a, b)) = both_scans {
        if differs(a.dpi as f64, b.dpi as f64, t.capture_dpi_share) || a.color != b.color {
            return (Verdict::New, "capture");
        }
        let (pw, ph) = prev.size;
        let (nw, nh) = next.size;
        if (pw > ph) != (nw > nh) || differs(pw, nw, t.paper_share) || differs(ph, nh, t.paper_share) {
            return (Verdict::New, "paper");
        }
    }
    let c = catalog();
    let tail = prev.lines[prev.lines.len().saturating_sub(t.tail_lines)..].join(" ");
    let head = next.lines.iter().take(t.head_lines).cloned().collect::<Vec<_>>().join(" ");
    if prev.making != Making::Blank && has_any(&tail, &c.closers).is_some() {
        return (Verdict::New, "closer");
    }
    if has_any(&head, &c.openers).is_some() {
        return (Verdict::New, "opener");
    }
    (Verdict::Doubtful, NONE)
}

/// The rule a step takes when the file is even.
pub const UNIFORM: &str = "uniform";

/// The rule of a step no rule decided.
pub const NONE: &str = "none";

fn same_paper((aw, ah): (f64, f64), (bw, bh): (f64, f64), share: f64) -> bool {
    (aw > ah) == (bw > bh) && !differs(aw, bw, share) && !differs(ah, bh, share)
}

/// Whether a file is even: the rule is on, none of the `verdicts` of its
/// steps (from one page to the next) starts another document, and its
/// pages that are not blank are all digital or all scans, on paper of the
/// orientation and, within `sizeShare`, the size of the first of them.
pub fn uniform(pages: &[PageSignals], verdicts: &[Verdict]) -> bool {
    let u = &catalog().uniform;
    if !u.on || verdicts.contains(&Verdict::New) {
        return false;
    }
    let mut kept = pages.iter().filter(|p| p.making != Making::Blank);
    let Some(first) = kept.next() else {
        return false;
    };
    kept.all(|p| p.making == first.making && same_paper(first.size, p.size, u.size_share))
}

/// Whether a page ends with a signature (`signatures` of the even file,
/// in its last `signatureLines` own lines): the step after it is not let
/// go on by the even file, as a signed page often closes a document.
pub fn signed_at_the_end(page: &PageSignals) -> bool {
    let u = &catalog().uniform;
    let tail = page.lines[page.lines.len().saturating_sub(u.signature_lines)..].join(" ");
    has_any(&tail, &u.signatures).is_some()
}

/// The type the vocabulary finds in the first page of a document.
pub fn type_of(page: &PageSignals) -> Option<String> {
    let c = catalog();
    if page.machine_zone {
        return Some(c.machine_zone_type.clone());
    }
    let text = page.lines.join("\n");
    c.types.iter().find(|(_, phrases)| has_any(&text, phrases).is_some()).map(|(id, _)| id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::docbase::cut::signals::MainImage;

    fn digital(page: u32, lines: &[&str]) -> PageSignals {
        PageSignals { lines: lines.iter().map(|l| l.to_string()).collect(), ..PageSignals::bare(page, Making::Digital) }
    }

    fn scan(page: u32, dpi: u32, pixels: (u32, u32), color: &str, size: (f64, f64)) -> PageSignals {
        PageSignals {
            size,
            image: Some(MainImage { pixels, dpi, color: color.into() }),
            ..PageSignals::bare(page, Making::Scan)
        }
    }

    const PLAIN: [&str; 2] = ["texto inventado que sigue de una pagina a la otra", "sin formulas de apertura ni de cierre"];

    #[test]
    fn a_blank_page_follows_the_one_before() {
        let next = PageSignals { page_of: vec![(1, 3)], ..PageSignals::bare(2, Making::Blank) };
        assert_eq!(step(&digital(1, &PLAIN), &next), (Verdict::Continue, "blank"));
    }

    #[test]
    fn page_x_of_y_continues_from_the_second_and_starts_at_the_first_or_after_the_last() {
        let with = |page, of: (u32, u32)| PageSignals { page_of: vec![of], ..digital(page, &PLAIN) };
        assert_eq!(step(&digital(1, &PLAIN), &with(2, (2, 3))), (Verdict::Continue, "page_of"));
        assert_eq!(step(&with(1, (3, 3)), &with(2, (1, 2))), (Verdict::New, "page_of"));
        assert_eq!(step(&with(1, (3, 3)), &digital(2, &PLAIN)), (Verdict::New, "page_of"));
        // «1 de 1» is a whole document of one page: nothing follows from it.
        assert_eq!(step(&with(1, (1, 1)), &digital(2, &PLAIN)).1, "none");
    }

    #[test]
    fn document_codes_continue_when_shared_and_cut_when_different() {
        let with = |page, code: &str| PageSignals { codes: [code.to_string()].into(), ..digital(page, &PLAIN) };
        assert_eq!(step(&with(1, "AAA-2026-0000001-X"), &with(2, "AAA-2026-0000001-X")), (Verdict::Continue, "doc_code"));
        assert_eq!(step(&with(1, "AAA-2026-0000001-X"), &with(2, "AAA-2026-0000002-X")), (Verdict::New, "doc_code"));
        assert_eq!(step(&with(1, "AAA-2026-0000001-X"), &digital(2, &PLAIN)).1, "none");
    }

    #[test]
    fn correlative_notarial_serials_continue() {
        let with = |page, letters: &str, n| PageSignals { serials: vec![(letters.to_string(), n)], ..digital(page, &PLAIN) };
        assert_eq!(step(&with(1, "Z", 10000000), &with(2, "Z", 10000001)), (Verdict::Continue, "serial"));
        assert_eq!(step(&with(1, "Z", 10000000), &with(2, "Y", 10000001)).1, "none");
    }

    #[test]
    fn two_photos_of_a_card_continue_and_a_scan_beside_a_digital_page_starts_another() {
        let card = (300.0, 200.0);
        assert_eq!(step(&scan(1, 72, (900, 600), "Rgb", card), &scan(2, 72, (900, 600), "Rgb", card)), (Verdict::Continue, "photo_pair"));
        assert_eq!(step(&scan(1, 200, (1700, 2300), "Gray", (612.0, 792.0)), &digital(2, &PLAIN)), (Verdict::New, "making"));
        assert_eq!(step(&digital(1, &PLAIN), &scan(2, 200, (1700, 2300), "Gray", (612.0, 792.0))), (Verdict::New, "making"));
    }

    #[test]
    fn another_capture_or_another_paper_starts_another() {
        let a4 = (595.0, 842.0);
        assert_eq!(step(&scan(1, 200, (1654, 2339), "Gray", a4), &scan(2, 300, (2480, 3508), "Gray", a4)), (Verdict::New, "capture"));
        assert_eq!(step(&scan(1, 200, (1654, 2339), "Gray", a4), &scan(2, 200, (1654, 2339), "Rgb", a4)), (Verdict::New, "capture"));
        assert_eq!(step(&scan(1, 200, (1654, 2339), "Gray", a4), &scan(2, 200, (1700, 2800), "Gray", (612.0, 1008.0))), (Verdict::New, "paper"));
        assert_eq!(step(&scan(1, 200, (1654, 2339), "Gray", a4), &scan(2, 200, (2339, 1654), "Gray", (842.0, 595.0))), (Verdict::New, "paper"));
        assert_eq!(step(&scan(1, 200, (1654, 2339), "Gray", a4), &scan(2, 205, (1654, 2339), "Gray", (596.0, 840.0))).1, "none");
    }

    #[test]
    fn closing_and_opening_formulas_start_another_and_nothing_else_is_doubtful() {
        let c = catalog();
        let closer = c.closers[0].clone();
        let opener = c.openers[0].clone();
        assert_eq!(step(&digital(1, &[PLAIN[0], &closer]), &digital(2, &PLAIN)), (Verdict::New, "closer"));
        assert_eq!(step(&digital(1, &PLAIN), &digital(2, &[&opener, PLAIN[1]])), (Verdict::New, "opener"));
        assert_eq!(step(&digital(1, &PLAIN), &digital(2, &PLAIN)), (Verdict::Doubtful, "none"));
    }

    #[test]
    fn a_file_is_even_with_no_cut_of_a_rule_and_pages_made_alike_on_the_same_paper() {
        use Verdict::{Continue, Doubtful, New};
        let a4 = (595.0, 842.0);
        let sized = |page, size| PageSignals { size, ..digital(page, &PLAIN) };
        let even = [sized(1, a4), sized(2, (596.0, 840.0)), sized(3, a4)];
        assert!(uniform(&even, &[Doubtful, Continue]));
        // A cut of a rule anywhere: not even.
        assert!(!uniform(&even, &[New, Doubtful]));
        // Digital pages of two sizes, or of another orientation.
        assert!(!uniform(&[sized(1, a4), sized(2, (612.0, 1008.0))], &[Doubtful]));
        assert!(!uniform(&[sized(1, a4), sized(2, (842.0, 595.0))], &[Doubtful]));
        // A scan beside digital pages, past a blank page that hides it from the steps.
        let blank = PageSignals { size: (612.0, 1008.0), ..PageSignals::bare(2, Making::Blank) };
        assert!(uniform(&[sized(1, a4), blank.clone(), sized(3, a4)], &[Continue, Doubtful]));
        let paper = scan(3, 200, (1654, 2339), "Gray", a4);
        assert!(!uniform(&[sized(1, a4), blank, paper.clone()], &[Continue, Doubtful]));
        // Scans alike are even too.
        assert!(uniform(&[scan(1, 200, (1654, 2339), "Gray", a4), paper], &[Doubtful]));
    }

    #[test]
    fn a_page_that_ends_with_a_signature_is_signed_at_the_end() {
        let sign = catalog().uniform.signatures[1].clone();
        assert!(signed_at_the_end(&digital(1, &[PLAIN[0], &sign, "nombre inventado", PLAIN[1]])));
        assert!(!signed_at_the_end(&digital(1, &PLAIN)));
        // Far from the end it is not the page's signature.
        let mut lines = vec![sign.as_str()];
        lines.extend([PLAIN[0]; 8]);
        assert!(!signed_at_the_end(&digital(1, &lines)));
    }

    #[test]
    fn the_type_is_the_first_of_the_vocabulary_found_and_a_machine_zone_is_an_identity_document() {
        let c = catalog();
        let (first, phrases) = &c.types[0];
        assert_eq!(type_of(&digital(1, &[&phrases[0]])), Some(first.clone()));
        assert_eq!(type_of(&digital(1, &PLAIN)), None);
        let id = PageSignals { machine_zone: true, ..digital(1, &PLAIN) };
        assert_eq!(type_of(&id), Some(c.machine_zone_type.clone()));
    }
}
