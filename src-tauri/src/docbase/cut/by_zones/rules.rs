//! The rules of the cut by zones over one step (page a to page b). Each
//! one that fires gives `corta` (another document starts) or `une` (the
//! same goes on), with a datum. Only the finals of `cut.json` decide, and
//! only when all of them that fire say the same; the others are hints for
//! the question.

use std::collections::BTreeSet;
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::page::Features;
use super::py::{fold, head, strip, tail};
use crate::docbase::cut::catalog::catalog;

pub const CUTS: &str = "corta";
pub const JOINS: &str = "une";
pub const DOUBTFUL: &str = "dudoso";

/// A rule that fired on a step.
#[derive(Debug, Clone, PartialEq)]
pub struct Fired {
    pub rule: String,
    pub gives: &'static str,
    pub datum: String,
}

/// A hint of a step: a rule that fired and does not decide (`final` is
/// true only for the finals of a step they did not decide).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hint {
    pub regla: String,
    pub da: String,
    pub dato: String,
    #[serde(rename = "final")]
    pub is_final: bool,
}

/// What the rules say of a step: `corta`, `une` or `dudoso`, the rules
/// that decided it (joined by `+`; `ninguna` or `conflicto` when doubtful)
/// and the hints.
#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub after: u32,
    pub decision: &'static str,
    pub rule: String,
    pub hints: Vec<Hint>,
}

const SCANS: [&str; 3] = ["scan", "scan_old_ocr", "picture"];
const DIGITAL: [&str; 3] = ["digital_prose", "digital_form", "flattened_form"];

struct Rx {
    closed_end: Regex,
    open_end: Regex,
    first_word: Regex,
    words: Regex,
    hyphen: Regex,
}

fn rx() -> &'static Rx {
    static R: OnceLock<Rx> = OnceLock::new();
    R.get_or_init(|| Rx {
        closed_end: Regex::new(r#"[.;:!?)\]»"”’]\s*$"#).unwrap(),
        open_end: Regex::new(r"(?:[^\W\d_]|,|[^\W\d_]-)\s*$").unwrap(),
        first_word: Regex::new(r"^\W*?([^\W\d_]+)(\S?)").unwrap(),
        words: Regex::new(r"[^\W\d_]+").unwrap(),
        hyphen: Regex::new(r"[^\W\d_]-\s*$").unwrap(),
    })
}

fn words(text: &str) -> Vec<&str> {
    rx().words.find_iter(text).map(|m| m.as_str()).collect()
}

/// The line ends in the middle of a sentence and is long enough to be prose.
fn open_sentence(text: &str) -> bool {
    let t = strip(text);
    words(t).len() >= 3 && !rx().closed_end.is_match(t) && rx().open_end.is_match(t)
}

/// Python's `str.islower()`.
fn is_lower(word: &str) -> bool {
    word.chars().any(|c| c.is_lowercase()) && !word.chars().any(|c| c.is_uppercase())
}

fn lower_start(text: &str) -> bool {
    let Some(c) = rx().first_word.captures(strip(text)) else {
        return false;
    };
    let (word, after) = (&c[1], &c[2]);
    is_lower(word)
        && word.chars().count() >= 2
        && after != ")"
        && after != "."
        && !["fs", "fjs", "www", "http", "https", "mail"].contains(&word)
}

/// A line all in capitals, as a stamp or a heading is written («ES COPIA
/// FIEL DEL»): its last word does not tie it to the next page.
fn shouted(text: &str) -> bool {
    let letters: Vec<char> = text.chars().filter(|c| c.is_alphabetic()).collect();
    letters.len() >= 6 && !letters.iter().any(|c| c.is_lowercase())
}

fn differs(x: f64, y: f64, share: f64) -> bool {
    (x - y).abs() > share * x.max(y).max(1.0)
}

fn serial_next(a: &[String], b: &[String]) -> bool {
    let parse = |s: &str| {
        let letters = s.trim_end_matches(|c: char| c.is_ascii_digit());
        (letters.to_string(), s[letters.len()..].parse::<u64>().unwrap_or(0))
    };
    a.iter().map(|s| parse(s)).any(|(la, na)| b.iter().map(|s| parse(s)).any(|(lb, nb)| la == lb && nb == na + 1))
}

/// A list of pairs as Python writes it: `[(1, 3), (2, 3)]`.
fn pairs(v: &[(u32, u32)]) -> String {
    format!("[{}]", v.iter().map(|(a, b)| format!("({a}, {b})")).collect::<Vec<_>>().join(", "))
}

/// A list of numbers as Python writes it: `[3, 4]`.
fn numbers(v: &[u64]) -> String {
    format!("[{}]", v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", "))
}

fn kind(p: &Features) -> Option<&'static str> {
    if p.forma == "scan" && SCANS.contains(&p.scan.shape.as_str()) {
        Some("scan")
    } else if p.forma == "digital" && DIGITAL.contains(&p.scan.shape.as_str()) {
        Some("digital")
    } else {
        None
    }
}

/// Every rule that fires on the step a -> b, in order.
pub fn rules(a: &Features, b: &Features) -> Vec<Fired> {
    let mut out: Vec<Fired> = Vec::new();
    let mut fire = |rule: &str, gives: &'static str, datum: String| out.push(Fired { rule: rule.to_string(), gives, datum });

    // 1. Digital and scanned: both readings of the page must agree.
    let (ka, kb) = (kind(a), kind(b));
    let made_otherwise = matches!((ka, kb), (Some(x), Some(y)) if x != y);
    if made_otherwise {
        fire("cambio_digital_escaneo", CUTS, format!("{}->{}", ka.unwrap(), kb.unwrap()));
    } else if (a.forma == "scan" && b.forma == "digital") || (a.forma == "digital" && b.forma == "scan") {
        fire("cambio_digital_escaneo_dudoso", CUTS, format!("{}/{}->{}/{}", a.forma, a.scan.shape, b.forma, b.scan.shape));
    }

    // 2. The sentence goes on.
    if let (Some(la), Some(fb)) = (&a.last, &b.first) {
        let hyphen = rx().hyphen.is_match(&la.text);
        if lower_start(&fb.text) && (open_sentence(&la.text) || hyphen) {
            let rule = if hyphen { "palabra_cortada" } else { "frase_sigue" };
            let doubtful = !(la.safe && fb.safe);
            let name = format!("{rule}{}", if doubtful { "_dudoso" } else { "" });
            fire(&name, JOINS, format!("«{}» | «{}»", tail(&la.text, 30), head(&fb.text, 30)));
        }
    }

    // 2b. Page a ends on a comma or on a function word.
    if let Some(la) = &a.last {
        let t = strip(&la.text);
        let folded = fold(t);
        let w = words(&folded);
        if w.len() >= 3 && !rx().closed_end.is_match(t) {
            let tag = if la.safe { "" } else { "_dudoso" };
            let last = *w.last().unwrap();
            if t.ends_with(',') {
                fire(&format!("termina_en_coma{tag}"), JOINS, format!("«{}»", tail(t, 30)));
            } else if !shouted(t)
                && catalog().zones.nexos.iter().any(|n| n == last)
                && Regex::new(&format!(r"\b{}\s*$", regex::escape(last))).is_ok_and(|r| r.is_match(&folded))
            {
                fire(&format!("termina_en_nexo{tag}"), JOINS, format!("«{}»", tail(t, 30)));
            }
        }
    }

    // 3. Line numbers go on.
    if let (Some(na), Some(nb)) = (a.line_numbers, b.line_numbers) {
        if nb.0 == na.1 + 1 {
            fire("renglones_siguen", JOINS, format!("{}->{}", na.1, nb.0));
        } else if nb.0 == 1 {
            fire("renglones_reinician", CUTS, format!("{}->{}", na.1, nb.0));
        }
    }

    // 4. «Página x de y», «Hoja x», «fs. x», bare page numbers.
    let (pa, pb): (BTreeSet<(u32, u32)>, BTreeSet<(u32, u32)>) = (a.page_of.iter().copied().collect(), b.page_of.iter().copied().collect());
    let (va, vb): (Vec<(u32, u32)>, Vec<(u32, u32)>) = (pa.iter().copied().collect(), pb.iter().copied().collect());
    if !pa.is_empty() && !pb.is_empty() {
        let datum = format!("{}->{}", pairs(&va), pairs(&vb));
        let totals_a: BTreeSet<u32> = pa.iter().map(|(_, y)| *y).collect();
        if pa.iter().any(|(x, y)| pb.contains(&(x + 1, *y))) {
            fire("pagina_de_sigue", JOINS, datum);
        } else if pb.iter().any(|(x, _)| *x == 1) {
            fire("pagina_de_reinicia", CUTS, datum);
        } else if !pb.iter().any(|(_, y)| totals_a.contains(y)) {
            fire("pagina_de_otro_total", CUTS, datum);
        } else {
            fire("pagina_de_salta", CUTS, datum);
        }
    } else if !pa.is_empty() && pb.is_empty() && pa.iter().any(|(x, y)| x == y && *y > 1) {
        fire("pagina_de_ultima", CUTS, format!("{}->nada", pairs(&va)));
    } else if !pb.is_empty() && pa.is_empty() {
        if pb.iter().any(|(x, _)| *x == 1) {
            fire("pagina_de_primera", CUTS, format!("nada->{}", pairs(&vb)));
        } else if pb.iter().any(|(x, _)| *x > 1) {
            fire("pagina_de_sola", JOINS, format!("nada->{}", pairs(&vb)));
        }
    }
    if !a.sheets.is_empty() && !b.sheets.is_empty() {
        let datum = format!("{}->{}", numbers(&a.sheets), numbers(&b.sheets));
        if a.sheets.iter().any(|s| b.sheets.contains(&(s + 1))) {
            fire("hoja_sigue", JOINS, datum);
        } else {
            fire("hoja_salta", CUTS, datum);
        }
    }
    if !a.folios.is_empty() && !b.folios.is_empty() {
        let datum = format!("{}->{}", numbers(&a.folios), numbers(&b.folios));
        if a.folios.iter().any(|s| b.folios.contains(s) || b.folios.contains(&(s + 1))) {
            fire("fojas_siguen", JOINS, datum);
        } else {
            fire("fojas_saltan", CUTS, datum);
        }
    }
    if let (Some(x), Some(y)) = (a.number, b.number) {
        if y == x + 1 {
            fire("numero_sigue", JOINS, format!("{x}->{y}"));
        } else if y == 1 {
            fire("numero_reinicia", CUTS, format!("{x}->{y}"));
        }
    }

    // 5. A signature closes page a (and it does not end mid-sentence).
    let ends_open = a.last.as_ref().is_some_and(|l| open_sentence(&l.text));
    let cues = &a.sign_cues;
    if !ends_open && (cues.iter().any(|c| c == "firmado") || cues.len() >= 2) {
        fire("firma_cierra", CUTS, cues.join(","));
    }
    if !ends_open && (a.signature_mark == "handwritten" || a.signature_mark == "system") {
        fire("marca_de_firma", CUTS, a.signature_mark.clone());
    }

    // 6. A page that only announces an annex is a piece of its own
    // (D-724): another thing starts on it and after it. It is not a blank
    // page, however few letters it has.
    if let Some(said) = &b.announces {
        fire(crate::docbase::cut::parts::ANNOUNCED, CUTS, said.clone());
    }
    if let Some(said) = &a.announces {
        fire(crate::docbase::cut::parts::AFTER_ANNOUNCEMENT, CUTS, said.clone());
    }

    // 7. All page b has is the foot of page a again (a code the reader
    // did not take as a foot there): it is a page of only the foot, and it
    // goes on (P-585, 1).
    if b.only_lines && !b.own_lines.is_empty() && b.own_lines.iter().all(|l| a.foot.contains(l)) {
        fire("repite_el_pie", JOINS, format!("{} renglón(es)", b.own_lines.len()));
    }

    // The text rules of the cut before, on firm text.
    if b.forma == "blank_or_footer" && b.announces.is_none() {
        fire("pagina_blanca", JOINS, String::new());
    }
    if !a.doc_codes.is_empty() && !b.doc_codes.is_empty() {
        if a.doc_codes.iter().any(|c| b.doc_codes.contains(c)) {
            fire("codigo_igual", JOINS, String::new());
        } else {
            fire("codigo_distinto", CUTS, String::new());
        }
    }
    if serial_next(&a.serials, &b.serials) {
        fire("serie_notarial_sigue", JOINS, String::new());
    }
    let start = fold(&b.body.iter().take(4).cloned().collect::<Vec<_>>().join(" "));
    let end = fold(&a.body[a.body.len().saturating_sub(4)..].join(" "));
    if let Some(op) = catalog().openers.iter().find(|x| start.contains(x.as_str())) {
        fire("formula_apertura", CUTS, op.clone());
    }
    // A closing formula does not push to cut when page b is only the
    // signature of what it closes (P-585, 2).
    if let Some(cl) = catalog().closers.iter().find(|x| end.contains(x.as_str())) {
        if a.forma != "blank_or_footer" && !b.only_signature {
            fire("formula_cierre", CUTS, cl.clone());
        }
    }

    // How the pages look: hints only.
    let (sa, sb) = (&a.scan, &b.scan);
    if let (Some(ia), Some(ib)) = (&sa.image, &sb.image) {
        if SCANS.contains(&sa.shape.as_str()) && SCANS.contains(&sb.shape.as_str()) {
            if ia.dpi < 100 && ib.dpi < 100 && ia.px == ib.px && (sa.w, sa.h) == (sb.w, sb.h) {
                fire("par_de_fotos", JOINS, String::new());
            }
            if differs(ia.dpi as f64, ib.dpi as f64, 0.10) {
                fire("otra_resolucion", CUTS, format!("{}->{}", ia.dpi, ib.dpi));
            }
            if ia.cs != ib.cs || ia.bpp != ib.bpp {
                fire("otro_color", CUTS, format!("{}/{}->{}/{}", ia.cs, ia.bpp, ib.cs, ib.bpp));
            }
            let ((wa, ha), (wb, hb)) = ((sa.w as f64, sa.h as f64), (sb.w as f64, sb.h as f64));
            if (wa > ha) != (wb > hb) || differs(wa, wb, 0.04) || differs(ha, hb, 0.04) {
                fire("otro_papel", CUTS, format!("{}x{}->{}x{}", sa.w, sa.h, sb.w, sb.h));
            }
        }
    }
    if sa.shape != sb.shape && !made_otherwise && sa.shape != "blank" && sb.shape != "blank" {
        fire("otra_hechura", CUTS, format!("{}->{}", sa.shape, sb.shape));
    }
    out
}

/// What decides a step: the finals that fire, when they all say the same.
pub fn decide(after: u32, fired: Vec<Fired>) -> Decision {
    let finals = &catalog().zones.finals;
    let is_final = |f: &Fired| finals.iter().any(|x| *x == format!("{}|{}", f.rule, f.gives));
    let hint = |f: &Fired, is_final: bool| Hint { regla: f.rule.clone(), da: f.gives.to_string(), dato: f.datum.clone(), is_final };
    let deciding: Vec<&Fired> = fired.iter().filter(|f| is_final(f)).collect();
    let mut hints: Vec<Hint> = fired.iter().filter(|f| !is_final(f)).map(|f| hint(f, false)).collect();
    let gives: BTreeSet<&str> = deciding.iter().map(|f| f.gives).collect();
    if gives.len() == 1 {
        let decision = if gives.contains(CUTS) { CUTS } else { JOINS };
        let rule = deciding.iter().map(|f| f.rule.as_str()).collect::<Vec<_>>().join("+");
        return Decision { after, decision, rule, hints };
    }
    hints.extend(deciding.iter().map(|f| hint(f, true)));
    let rule = if deciding.is_empty() { "ninguna" } else { "conflicto" };
    Decision { after, decision: DOUBTFUL, rule: rule.into(), hints }
}

/// The parts of a file of `pages` pages: the runs between two cuts the
/// rules decided, each with its doubtful steps.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub first: u32,
    pub last: u32,
    pub doubtful: Vec<u32>,
}

pub fn parts(pages: u32, decisions: &[Decision]) -> Vec<Part> {
    let mut out = Vec::new();
    let (mut start, mut doubtful) = (1, Vec::new());
    for d in decisions {
        if d.decision == CUTS {
            out.push(Part { first: start, last: d.after, doubtful: std::mem::take(&mut doubtful) });
            start = d.after + 1;
        } else if d.decision == DOUBTFUL {
            doubtful.push(d.after);
        }
    }
    out.push(Part { first: start, last: pages, doubtful });
    out
}
