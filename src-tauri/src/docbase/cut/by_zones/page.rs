//! The reading by zones of a page (D-668) and what the rules of the cut
//! take from it: the first and last line of its own content, the marks of
//! «página x de y», sheets, folios and page numbers, the line numbers of
//! the margin, codes, serials and the cues of a signature. Only the firm
//! text founds a decision; a doubtful line only gives hints.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::py::{fold, splitlines, strip};
use super::scan::PageScan;

/// One line read in a zone: its text, its box in pixels and whether it is
/// firm (`seguro`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Line {
    #[serde(default)]
    pub texto: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recuadro: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seguro: Option<bool>,
}

/// A zone of the page: its class (text, title, header, footer, number,
/// table, seal, image...), its box, its firm text and its lines.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub clase: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recuadro: Option<Vec<f64>>,
    #[serde(default)]
    pub texto: String,
    #[serde(default)]
    pub renglones: Vec<Line>,
}

/// How the page is made, as the reading tells it: `forma` (`digital`,
/// `scan`, `blank_or_footer`), its layer and the mark of a signature
/// (`handwritten`, `system`, `none`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    #[serde(default)]
    pub forma: String,
    #[serde(default)]
    pub capa: String,
    #[serde(default)]
    pub firma: String,
}

/// A doubtful line of the page, by the zone it is in.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Doubt {
    #[serde(default)]
    pub zona: String,
}

/// The reading by zones of one page, as the reader of N writes it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PageReading {
    pub page: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tamano_px: Option<Vec<f64>>,
    #[serde(default)]
    pub forma: Shape,
    #[serde(default)]
    pub zonas: Vec<Zone>,
    #[serde(default)]
    pub evidencia: Vec<Doubt>,
    #[serde(default)]
    pub texto_seguro: String,
}

/// Classes of zones that are the furniture of the page: left out to find
/// its first and last line.
const EDGE: [&str; 7] = ["header", "header_image", "footer", "footer_image", "number", "aside_text", "vertical_text"];
/// Classes of zones of running prose.
const PROSE: [&str; 9] =
    ["text", "abstract", "content", "reference_content", "sin_zona", "paragraph_title", "doc_title", "algorithm", "figure_title"];

/// A line of the page's own content, with its text and whether it is firm.
#[derive(Debug, Clone, PartialEq)]
pub struct EndLine {
    pub text: String,
    pub safe: bool,
}

/// What the rules see of a page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Features {
    pub page: u32,
    /// `forma` of the reading, its mark of a signature, and the shape and
    /// paper pdfium gives (`scan`).
    pub forma: String,
    pub signature_mark: String,
    pub scan: PageScan,
    pub first: Option<EndLine>,
    pub last: Option<EndLine>,
    /// The firm lines of prose, from the top.
    pub body: Vec<String>,
    pub page_of: Vec<(u32, u32)>,
    pub sheets: Vec<u64>,
    pub folios: Vec<u64>,
    pub number: Option<u64>,
    pub line_numbers: Option<(u64, u64)>,
    pub doc_codes: Vec<String>,
    pub serials: Vec<String>,
    pub sign_cues: Vec<String>,
}

struct Patterns {
    page_of: [Regex; 3],
    bare_of: Regex,
    sheet: Regex,
    sheet_after: Regex,
    folio: Regex,
    bare_num: Regex,
    doc_code: Regex,
    serial: Regex,
    line_number: Regex,
    sign: Vec<(&'static str, Regex)>,
}

fn patterns() -> &'static Patterns {
    static P: OnceLock<Patterns> = OnceLock::new();
    P.get_or_init(|| {
        let r = |p: &str| Regex::new(p).expect("the patterns of the cut by zones are valid");
        Patterns {
            page_of: [
                r(r"(?i)\bp(?:a|á)g(?:ina|\.)?\s*(?:n[°º.]?\s*)?(\d{1,3})\s*(?:de|/)\s*(\d{1,3})\b"),
                r(r"(?i)\bpage\s*(\d{1,3})\s*(?:of|/)\s*(\d{1,3})\b"),
                r(r"(?i)\bhoja\s*(?:n[°º.]?\s*)?(\d{1,3})\s*(?:de|/)\s*(\d{1,3})\b"),
            ],
            bare_of: r(r"(?i)^\s*(\d{1,3})\s*(?:/|de|of)\s*(\d{1,3})\s*$"),
            sheet: r(r"(?i)\bhoja\s*(?:n[°º.]?\s*)?(\d{1,4})\b"),
            // What may not follow a sheet: «de 3», «/3» (that is a page of).
            sheet_after: r(r"(?i)^\s*(?:de|/)\s*\d"),
            folio: r(r"(?i)^\s*(?:fs|fjs|fojas?|folios?)\.?\s*(?:n[°º.]?\s*)?(\d{1,4})\s*(vta\.?|vuelta)?\s*$"),
            bare_num: r(r"^[\s\-–—.]*(\d{1,4})[\s\-–—.]*$"),
            doc_code: r(r"\b([A-Z]{2,4}-\d{4}-\d{6,}-[A-Z0-9-]*[A-Z])"),
            serial: r(r"\b([A-Z]{1,2})\s?(\d{8,9})\b"),
            line_number: r(r"^\d{1,3}$"),
            sign: vec![
                ("aclaracion", r(r"\baclaracion\b")),
                ("firma", r(r"\bfirma\b|\bfirmas\b")),
                ("firmado", r(r"firmad[oa] (?:digital|electronica)mente|firma digital|docusigned by|signed by")),
                (
                    "matricula",
                    r(r"\bt[°º]\s*\d|\btomo\s*\d+\s*(?:f[°º]|folio)|\bmat(?:ricula)?\.?\s*(?:n[°º.]?\s*)?\d|c\.?p\.?a\.?c\.?f|\bcuit\b|\bd\.?n\.?i\.?\b"),
                ),
                ("pp", r(r"\bp/p\b|\bp\.p\.\s")),
                (
                    "profesion",
                    r(r"\b(?:abogad[oa]|escriban[oa]|contador[a]? public[oa]|letrad[oa]|apoderad[oa]|juez|jueza|secretari[oa]|prosecretari[oa]|presidente|gerente)\b"),
                ),
            ],
        }
    })
}

/// Python's `int()` of a run of digits (any script).
fn int(digits: &str) -> u64 {
    digits.chars().filter_map(|c| c.to_digit(10)).fold(0u64, |n, d| n.saturating_mul(10).saturating_add(d as u64))
}

/// A line of prose: its box (y0, y1, x0, x1), text, whether it is firm.
struct ProseLine {
    y0: f64,
    y1: f64,
    x0: f64,
    x1: f64,
    text: String,
    safe: bool,
}

fn boxed(line: &Line, zone: &Zone) -> [f64; 4] {
    let b = line.recuadro.as_ref().or(zone.recuadro.as_ref());
    match b {
        Some(b) if b.len() >= 4 => [b[0], b[1], b[2], b[3]],
        _ => [0.0; 4],
    }
}

/// The features of a page, from its reading and what pdfium gives.
pub fn features(e: &PageReading, scan: &PageScan) -> Features {
    let p = patterns();
    let (w, h) = match e.tamano_px.as_deref() {
        Some([w, h, ..]) => (*w, *h),
        _ => (2000.0, 2000.0),
    };
    let mut lines: Vec<ProseLine> = Vec::new();
    let mut blockers: Vec<(f64, f64)> = Vec::new();
    for z in &e.zonas {
        let cls = z.clase.as_str();
        if EDGE.contains(&cls) {
            continue;
        }
        if !PROSE.contains(&cls) {
            let r = match z.recuadro.as_deref() {
                Some(r) if r.len() >= 4 => [r[0], r[1], r[2], r[3]],
                _ => [0.0; 4],
            };
            blockers.push((r[1], r[3]));
            continue;
        }
        for r in &z.renglones {
            let b = boxed(r, z);
            let t = strip(&r.texto);
            if !t.is_empty() {
                lines.push(ProseLine { y0: b[1], y1: b[3], x0: b[0], x1: b[2], text: t.to_string(), safe: r.seguro.unwrap_or(false) });
            }
        }
    }
    lines.sort_by(|a, b| a.y0.total_cmp(&b.y0).then(a.x0.total_cmp(&b.x0)));
    let mut edge_lines: Vec<(String, f64)> = Vec::new();
    let mut all_safe: Vec<String> = Vec::new();
    for z in &e.zonas {
        for r in &z.renglones {
            if !r.seguro.unwrap_or(false) {
                continue;
            }
            let t = strip(&r.texto).to_string();
            let b = boxed(r, z);
            all_safe.push(t.clone());
            if EDGE.contains(&z.clase.as_str()) {
                edge_lines.push((t, (b[1] + b[3]) / 2.0 / h));
            }
        }
    }
    let safe_text = if e.texto_seguro.is_empty() { all_safe.join("\n") } else { e.texto_seguro.clone() };
    let mut f = Features {
        page: e.page,
        forma: e.forma.forma.clone(),
        signature_mark: e.forma.firma.clone(),
        scan: scan.clone(),
        body: lines.iter().filter(|l| l.safe).map(|l| l.text.clone()).collect(),
        ..Features::default()
    };
    if let Some(first) = lines.first() {
        // The last line: the lowest bottom, then the rightmost end; the
        // first of them on a tie.
        let mut last = first;
        for l in &lines {
            if (l.y1, l.x1) > (last.y1, last.x1) {
                last = l;
            }
        }
        if !blockers.iter().any(|(b0, _)| *b0 < first.y0) {
            f.first = Some(EndLine { text: first.text.clone(), safe: first.safe });
        }
        if !blockers.iter().any(|(_, b1)| *b1 > last.y1) {
            f.last = Some(EndLine { text: last.text.clone(), safe: last.safe });
        }
    }
    let mut of: BTreeSet<(u32, u32)> = BTreeSet::new();
    for line in splitlines(&safe_text) {
        for rx in &p.page_of {
            for c in rx.captures_iter(line) {
                let (a, b) = (int(&c[1]), int(&c[2]));
                if 0 < a && a <= b {
                    of.insert((a as u32, b as u32));
                }
            }
        }
    }
    for (t, _) in &edge_lines {
        if let Some(c) = p.bare_of.captures(t) {
            let (a, b) = (int(&c[1]), int(&c[2]));
            if 0 < a && a <= b {
                of.insert((a as u32, b as u32));
            }
        }
    }
    f.page_of = of.into_iter().collect();
    let sheets: BTreeSet<u64> = p
        .sheet
        .captures_iter(&safe_text)
        .filter(|c| !p.sheet_after.is_match(&safe_text[c.get(0).unwrap().end()..]))
        .map(|c| int(&c[1]))
        .collect();
    f.sheets = sheets.into_iter().collect();
    let folios: BTreeSet<u64> = all_safe.iter().filter_map(|t| p.folio.captures(t)).map(|c| int(&c[1])).collect();
    f.folios = folios.into_iter().collect();
    let numbers: BTreeSet<u64> = edge_lines
        .iter()
        .filter(|(_, y)| *y < 0.12 || *y > 0.88)
        .filter_map(|(t, _)| p.bare_num.captures(t))
        .map(|c| int(&c[1]))
        .collect();
    f.number = if numbers.len() == 1 { numbers.into_iter().next() } else { None };
    f.line_numbers = line_numbers(e, w);
    let codes: BTreeSet<String> = p.doc_code.captures_iter(&safe_text).map(|c| c[1].to_string()).collect();
    f.doc_codes = codes.into_iter().collect();
    let serials: BTreeSet<String> = p.serial.captures_iter(&safe_text).map(|c| format!("{}{}", &c[1], &c[2])).collect();
    f.serials = serials.into_iter().collect();
    f.sign_cues = sign_cues(&lines);
    f
}

/// (first, last) of a column of line numbers in the margin that runs one
/// by one, or nothing.
fn line_numbers(e: &PageReading, w: f64) -> Option<(u64, u64)> {
    let p = patterns();
    let mut marks: Vec<(f64, u64)> = Vec::new();
    for z in &e.zonas {
        for r in &z.renglones {
            let t = strip(&r.texto);
            let b = boxed(r, z);
            let x = (b[0] + b[2]) / 2.0;
            if r.seguro.unwrap_or(false) && p.line_number.is_match(t) && (x < 0.15 * w || x > 0.85 * w) {
                marks.push(((b[1] + b[3]) / 2.0, int(t)));
            }
        }
    }
    if marks.len() < 5 {
        return None;
    }
    marks.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let values: Vec<i64> = marks.iter().map(|(_, v)| *v as i64).collect();
    let steps: Vec<i64> = values.windows(2).map(|w| w[1] - w[0]).collect();
    let ones = steps.iter().filter(|s| **s == 1).count() as f64;
    if ones < 0.8 * steps.len() as f64 || steps.iter().any(|s| *s <= 0) {
        return None;
    }
    Some((values[0] as u64, *values.last().unwrap() as u64))
}

/// The cues of a signature in the last six firm lines of the page's own
/// content, in the order of their names.
fn sign_cues(lines: &[ProseLine]) -> Vec<String> {
    let mut by_bottom: Vec<&ProseLine> = lines.iter().collect();
    by_bottom.sort_by(|a, b| a.y1.total_cmp(&b.y1));
    let safe: Vec<&str> = by_bottom.iter().filter(|l| l.safe).map(|l| l.text.as_str()).collect();
    let tail = &safe[safe.len().saturating_sub(6)..];
    let text = fold(&tail.join(" \n "));
    patterns().sign.iter().filter(|(_, rx)| rx.is_match(&text)).map(|(k, _)| k.to_string()).collect()
}

/// The text of a page as the question shows it: each zone in order with
/// its firm text, and a warning where it has doubtful lines; and its
/// `forma`. Cut at `max` letters.
pub fn question_text(e: Option<&PageReading>, max: usize) -> (String, String) {
    let Some(e) = e else {
        return ("(sin lectura)".into(), "?".into());
    };
    let mut doubts: BTreeMap<&str, usize> = BTreeMap::new();
    for d in &e.evidencia {
        *doubts.entry(d.zona.as_str()).or_default() += 1;
    }
    let mut out: Vec<String> = Vec::new();
    let mut mute = 0;
    for z in &e.zonas {
        let t = strip(&z.texto);
        let n = doubts.get(z.id.as_str()).copied().unwrap_or(0).max(z.renglones.iter().filter(|r| !r.seguro.unwrap_or(true)).count());
        if !t.is_empty() {
            out.push(format!("[{}] {t}", z.clase));
        }
        if n > 0 {
            out.push(format!("[{}] [dudoso: {n} renglón(es) sin transcribir; miralo en la imagen]", z.clase));
        } else if t.is_empty() {
            mute += 1;
        }
    }
    if mute > 0 {
        out.push(format!("({mute} zona(s) sin texto propio, por ejemplo imágenes o sellos: miralas en la imagen)"));
    }
    let mut text = out.join("\n");
    if text.is_empty() {
        text = "(sin texto)".into();
    }
    if text.chars().count() > max {
        text = text.chars().take(max).collect::<String>() + "\n(… el texto sigue; recortado)";
    }
    (text, e.forma.forma.clone())
}
