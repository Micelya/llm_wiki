//! The reading by zones of one page (copy of `estructura.py` of the
//! scripts of N, D-668): every line in the smallest zone that holds its
//! center, the lines outside every zone in zones of their own, and each
//! line with its band (firm, approximate or discarded, D-668) and why.
//!
//! - Firm: a digital line, or a line read with the high threshold or more
//!   that looks like words (initials and acronyms count as words, D-680).
//! - Below the high threshold a line is rescued as firm when the old layer
//!   of the page says the same (two readers agree, D-671) or, from the low
//!   threshold up, when all its words are in the lexicon.
//! - What is read in a zone of image or seal is at most approximate.
//! - The broken logo is not joined here (stage 3b): each word stays the
//!   line the reader gave.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;

use super::words::{all_words, bare, junk, junk_v1, letter_runs};
use crate::docbase::cut::by_zones::py::{is_space, strip};

/// The zones whose text is at most approximate: what is read there is
/// evidence, not text.
pub const AT_MOST_APPROXIMATE: [&str; 7] = ["seal", "image", "figure", "Figure", "header_image", "footer_image", "chart"];
/// A zone of the model counts from this score up.
pub const ZONE_MIN: f64 = 0.5;

/// One line as a reader gave it: its box in pixels (left, top, right,
/// bottom), its text and its confidence (none: pdfium's text, digital).
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub bbox: [i64; 4],
    pub text: String,
    pub score: Option<f64>,
}

/// One zone the model found.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelZone {
    pub bbox: [i64; 4],
    pub class: String,
    pub score: f64,
}

/// The thresholds a page is read with (D-722): the high one makes a line
/// firm; from the low one up a line is approximate, and may be rescued by
/// the lexicon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bands {
    pub high: f64,
    pub low: f64,
}

/// A line of the reading, with its band.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub texto: String,
    pub confianza: Option<f64>,
    pub recuadro: [i64; 4],
    pub seguro: bool,
    pub franja: &'static str,
    pub firme_posible: bool,
    pub motivo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v1: Option<&'static str>,
}

/// A zone of the reading.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Zone {
    pub id: String,
    pub clase: String,
    pub recuadro: [i64; 4],
    pub puntaje_zona: Option<f64>,
    pub motor_zonas: Option<String>,
    pub motor_texto: Option<String>,
    pub texto: String,
    pub texto_aproximado: String,
    pub confianza: Option<f64>,
    pub confianza_media: Option<f64>,
    pub segura: bool,
    pub renglones: Vec<Row>,
}

/// A line that is not firm, by its zone.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Evidence {
    pub zona: String,
    pub texto: String,
    pub confianza: Option<f64>,
    pub recuadro: [i64; 4],
    pub por_que: String,
    pub franja: &'static str,
}

/// The reading of a page: its zones, its doubtful lines, its firm and its
/// approximate text.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Structure {
    pub zones: Vec<Zone>,
    pub evidence: Vec<Evidence>,
    pub firm: String,
    pub approximate: String,
}

/// Python's `round(x, n)`, near enough for the confidences.
pub fn round_to(x: f64, decimals: i32) -> f64 {
    let f = 10f64.powi(decimals);
    (x * f).round_ties_even() / f
}

/// Whether a line may be firm, why, and what V1 changed (`clasificar`):
/// `rescata` when the old rule saw no words and V1 does, `hunde` the
/// other way round. Does not depend on the low threshold but for the
/// lexicon. `old_layer` is the old layer of the page already bare (`bare`).
pub fn classify(line: &Line, class: &str, bands: Bands, old_layer: Option<&str>) -> (bool, String, Option<&'static str>) {
    let Some(c) = line.score else {
        return (true, "digital".into(), None);
    };
    if AT_MOST_APPROXIMATE.contains(&class) {
        return (false, format!("zona de {class}"), None);
    }
    if c < bands.high {
        let t = bare(&line.text);
        if let Some(old) = old_layer.filter(|o| !o.is_empty()) {
            if t.chars().count() >= 8 && old.contains(&t) {
                return (true, "capa_vieja".into(), None);
            }
        }
        if c >= bands.low && all_words(&line.text) {
            return (true, "lexico".into(), None);
        }
        return (false, "confianza baja".into(), None);
    }
    let (d0, n0) = junk(&line.text);
    let (d1, n1) = junk_v1(&line.text);
    let (bad0, bad1) = (n0 >= 3 && d0 >= 0.5, n1 >= 3 && d1 >= 0.5);
    let v1 = if bad0 && !bad1 {
        Some("rescata")
    } else if bad1 && !bad0 {
        Some("hunde")
    } else {
        None
    };
    if bad1 {
        return (false, "no parece palabras".into(), v1);
    }
    (true, "confianza".into(), v1)
}

/// The band of a line (`franja_de`).
pub fn band(firm_possible: bool, confidence: Option<f64>, low: f64) -> &'static str {
    if firm_possible {
        "firme"
    } else if confidence.is_some_and(|c| c >= low) {
        "aproximada"
    } else {
        "descartada"
    }
}

fn inside(b: &[i64; 4], z: &[i64; 4]) -> bool {
    let (cx, cy) = ((b[0] + b[2]) as f64 / 2.0, (b[1] + b[3]) as f64 / 2.0);
    z[0] as f64 <= cx && cx <= z[2] as f64 && z[1] as f64 <= cy && cy <= z[3] as f64
}

fn area(z: &[i64; 4]) -> i64 {
    ((z[2] - z[0]) * (z[3] - z[1])).max(1)
}

/// Top to bottom and left to right, by bands 40 pixels high (stable).
fn reading_key(b: &[i64; 4]) -> (i64, i64) {
    ((b[1] as f64 / 40.0).round_ties_even() as i64, b[0])
}

fn hull<'a>(boxes: impl Iterator<Item = &'a [i64; 4]> + Clone) -> [i64; 4] {
    [
        boxes.clone().map(|b| b[0]).min().unwrap_or(0),
        boxes.clone().map(|b| b[1]).min().unwrap_or(0),
        boxes.clone().map(|b| b[2]).max().unwrap_or(0),
        boxes.map(|b| b[3]).max().unwrap_or(0),
    ]
}

struct Building {
    bbox: [i64; 4],
    class: String,
    score: Option<f64>,
    lines: Vec<Line>,
}

/// The reading of a page from its lines and its zones (`pagina`), with
/// the engine that gave the zones and the one that gave the lines.
pub fn structure(lines: &[Line], zones: &[ModelZone], bands: Bands, old_layer: Option<&str>, engines: (&str, &str)) -> Structure {
    let old = old_layer.map(bare).filter(|o| !o.is_empty());
    let mut zs: Vec<Building> = zones
        .iter()
        .filter(|z| z.score >= ZONE_MIN)
        .map(|z| Building { bbox: z.bbox, class: z.class.clone(), score: Some(z.score), lines: Vec::new() })
        .collect();
    zs.sort_by_key(|z| reading_key(&z.bbox));
    let mut loose: Vec<Line> = Vec::new();
    for l in lines {
        let best = zs.iter().enumerate().filter(|(_, z)| inside(&l.bbox, &z.bbox)).min_by_key(|(i, z)| (area(&z.bbox), *i)).map(|(i, _)| i);
        match best {
            Some(i) => zs[i].lines.push(l.clone()),
            None => loose.push(l.clone()),
        }
    }
    if !loose.is_empty() {
        // The lines outside every zone: a zone of their own for each block
        // of them, split where more than 60 pixels separate them.
        loose.sort_by_key(|l| reading_key(&l.bbox));
        let mut block: Vec<Line> = Vec::new();
        let close = |block: &mut Vec<Line>, zs: &mut Vec<Building>| {
            let bbox = hull(block.iter().map(|l| &l.bbox));
            zs.push(Building { bbox, class: "sin_zona".into(), score: None, lines: std::mem::take(block) });
        };
        for l in loose {
            if !block.is_empty() && l.bbox[1] - block.iter().map(|b| b.bbox[3]).max().unwrap_or(0) > 60 {
                close(&mut block, &mut zs);
            }
            block.push(l);
        }
        close(&mut block, &mut zs);
        zs.sort_by_key(|z| reading_key(&z.bbox));
    }
    let mut out = Structure::default();
    let (mut firm_text, mut approx_text) = (Vec::new(), Vec::new());
    for (i, mut z) in zs.into_iter().enumerate() {
        let id = format!("z{}", i + 1);
        z.lines.sort_by_key(|l| reading_key(&l.bbox));
        let (mut rows, mut firm, mut approx) = (Vec::new(), Vec::new(), Vec::new());
        for l in &z.lines {
            let (possible, why, v1) = classify(l, &z.class, bands, old.as_deref());
            let fr = band(possible, l.score, bands.low);
            if fr == "firme" {
                firm.push(l.text.clone());
            } else if fr == "aproximada" {
                approx.push(l.text.clone());
            }
            if fr != "firme" {
                out.evidence.push(Evidence {
                    zona: id.clone(),
                    texto: l.text.clone(),
                    confianza: l.score,
                    recuadro: l.bbox,
                    por_que: why.clone(),
                    franja: fr,
                });
            }
            rows.push(Row {
                texto: l.text.clone(),
                confianza: l.score,
                recuadro: l.bbox,
                seguro: fr == "firme",
                franja: fr,
                firme_posible: possible,
                motivo: why,
                v1,
            });
        }
        let confs: Vec<f64> = z.lines.iter().filter_map(|l| l.score).collect();
        let text_engine = if !confs.is_empty() {
            Some(engines.1.to_string())
        } else if !z.lines.is_empty() {
            Some("pdfium".to_string())
        } else {
            None
        };
        out.zones.push(Zone {
            id,
            recuadro: z.bbox,
            puntaje_zona: z.score,
            motor_zonas: (z.class != "sin_zona").then(|| engines.0.to_string()),
            motor_texto: text_engine,
            texto: firm.join("\n"),
            texto_aproximado: approx.join("\n"),
            confianza: (!confs.is_empty()).then(|| round_to(confs.iter().copied().fold(f64::INFINITY, f64::min), 4)),
            confianza_media: (!confs.is_empty()).then(|| round_to(confs.iter().sum::<f64>() / confs.len() as f64, 4)),
            segura: !firm.is_empty() && firm.len() == rows.len(),
            renglones: rows,
            clase: z.class,
        });
        if !firm.is_empty() {
            firm_text.push(firm.join("\n"));
        }
        if !approx.is_empty() {
            approx_text.push(approx.join("\n"));
        }
    }
    out.firm = firm_text.join("\n");
    out.approximate = approx_text.join("\n");
    out
}

/// The general marks of a page (`banderas`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Flags {
    pub lectura_baja_confianza: bool,
    pub seal: bool,
    pub table: bool,
    pub photo: bool,
    pub qr: bool,
    pub form_fields: bool,
}

/// The marks of a page, its mark of a signature (`system`, `handwritten`
/// or `none`) and whether it is blank or only a footer. `read`: its lines
/// come from the reader (else from pdfium); `own_chars`: the letters of
/// its own of pdfium, for a page not read.
pub fn flags(lines: &[Line], zones: &[ModelZone], low: f64, read: bool, own_chars: usize) -> (Flags, &'static str, bool) {
    struct Rx {
        system: Regex,
        sign: Regex,
        field: Regex,
    }
    static RX: OnceLock<Rx> = OnceLock::new();
    let rx = RX.get_or_init(|| Rx {
        system: Regex::new(r"(?i)firmad[oa] digitalmente|firma digital|docusign|signed by|firma electr[oó]nica|certificado de firma|firmante")
            .unwrap(),
        sign: Regex::new(r"(?i)\bfirma\b|aclaraci[oó]n|\bp/|\bfdo\b|firmado").unwrap(),
        field: Regex::new(r"^[\p{L}\p{Nl}\p{No}][\w .°º/()-]{1,40}:\s*\S*").unwrap(),
    });
    let classes: Vec<String> = zones.iter().filter(|z| z.score >= ZONE_MIN).map(|z| z.class.to_lowercase()).collect();
    let has = |c: &str| classes.iter().any(|x| x == c);
    let all = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join("\n");
    let read_lines: Vec<&Line> = lines.iter().filter(|l| l.score.is_some()).collect();
    let low_lines = read_lines
        .iter()
        .filter(|l| {
            letter_runs(&l.text).iter().map(|w| w.chars().count()).sum::<usize>() >= 3
                && (l.score.unwrap_or(1.0) < low || junk(&l.text).0 >= 0.5)
        })
        .count();
    let low_reading = !read_lines.is_empty() && low_lines as f64 >= (2f64).max(0.1 * read_lines.len() as f64);
    let seal = has("seal");
    let signature = if rx.system.is_match(&all) {
        "system"
    } else if read && (rx.sign.is_match(&all) || seal) {
        "handwritten"
    } else {
        "none"
    };
    let fields = lines.iter().filter(|l| rx.field.is_match(strip(&l.text))).count() >= 4;
    let visible = all.chars().filter(|c| !is_space(*c)).count();
    let blank = if read { visible < 30 } else { own_chars < 20 };
    let flags = Flags {
        lectura_baja_confianza: low_reading,
        seal,
        table: has("table"),
        photo: has("image") || has("figure"),
        qr: false,
        form_fields: fields,
    };
    (flags, signature, blank)
}

/// How many lines of each band a page has.
pub fn band_counts(zones: &[Zone]) -> BTreeMap<&'static str, usize> {
    let mut out: BTreeMap<&'static str, usize> = [("firme", 0), ("aproximada", 0), ("descartada", 0)].into_iter().collect();
    for r in zones.iter().flat_map(|z| &z.renglones) {
        *out.entry(r.franja).or_default() += 1;
    }
    out
}
