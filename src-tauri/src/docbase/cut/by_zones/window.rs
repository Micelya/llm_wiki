//! The question about a window of pages (a «tramo»: up to `pages` pages
//! of a part, two windows in a row sharing `overlap`): the images, the
//! firm text by zones and the hints of each step; the model says on which
//! pages a document starts, of what type, and how sure it is. Its key is
//! the print of the whole text of the question (never of the images). A
//! page two windows see takes the least sure answer.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::page::{question_text, PageReading};
use super::rules::{Decision, Hint, Part, CUTS, DOUBTFUL, JOINS};
use crate::docbase::cut::catalog::catalog;
use crate::docbase::similar::classify::question_key;

/// The task of the answers about windows in `model_answer`.
pub const WINDOW_TASK: &str = "cut-window";

/// How sure an answer is, from the surest.
pub const SURE: [&str; 3] = ["alta", "media", "baja"];

fn rank(s: &str) -> Option<usize> {
    SURE.iter().position(|x| *x == s)
}

/// One question about a window of a part.
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub first: u32,
    pub last: u32,
    pub part: (u32, u32),
    /// 1-based, of `windows` in its part.
    pub number: u32,
    pub windows: u32,
    pub prompt: String,
    pub key: String,
}

/// The windows of the pages `first..=last`.
pub fn spans(first: u32, last: u32) -> Vec<(u32, u32)> {
    let z = &catalog().zones;
    let mut out = Vec::new();
    let mut start = first;
    loop {
        let end = (start + z.pages - 1).min(last);
        out.push((start, end));
        if end == last {
            return out;
        }
        start = end + 1 - z.overlap;
    }
}

/// How a rule is named in the hints.
fn gloss(rule: &str) -> String {
    let (base, doubtful) = match rule.strip_suffix("_dudoso") {
        Some(base) => (base, true),
        None => (rule, false),
    };
    let g = catalog().zones.hints.get(base).cloned().unwrap_or_else(|| base.replace('_', " "));
    if doubtful {
        g + " (con texto dudoso)"
    } else {
        g
    }
}

fn step_hint(d: &Decision) -> String {
    let a = d.after;
    let mut head = format!("- De la {a} a la {}: ", a + 1);
    if d.decision != DOUBTFUL {
        let verb = if d.decision == JOINS { "sigue el mismo documento" } else { "empieza otro documento" };
        head += &format!("resuelto por una regla segura ({}): {verb}.", gloss(&d.rule));
    } else {
        head += "dudoso.";
    }
    let hints: Vec<String> = d
        .hints
        .iter()
        .filter(|h: &&Hint| !h.is_final)
        .map(|h| {
            let gives = if h.da == CUTS { "empezaría otro" } else { "seguiría el mismo" };
            let datum: String = h.dato.chars().take(60).collect();
            let datum = if datum.is_empty() { String::new() } else { format!(" [{datum}]") };
            format!("{}{datum}: {gives}", gloss(&h.regla))
        })
        .collect();
    if !hints.is_empty() {
        head += &format!(" Pistas: {}.", hints.join("; "));
    } else if d.decision == DOUBTFUL {
        head += " Sin pistas.";
    }
    head
}

/// The general criteria of the question about two pages (`ask.prompt`),
/// and the types, one per line.
fn criteria_and_types() -> (String, String) {
    let ask = &catalog().ask;
    let mut lines = Vec::new();
    let mut inside = false;
    for line in ask.prompt.lines() {
        if line.starts_with("Criterios generales") {
            inside = true;
            continue;
        }
        if inside {
            if line.trim().is_empty() {
                break;
            }
            lines.push(line);
        }
    }
    let types: Vec<String> = ask.types.iter().map(|(k, v)| format!("- {k}: {v}")).collect();
    (lines.join("\n"), types.join("\n"))
}

/// Puts the values in the `{name}` places of a template, in one pass.
fn fill(template: &str, values: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(template.len() + values.iter().map(|(_, v)| v.len()).sum::<usize>());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}').and_then(|close| values.iter().find(|(k, _)| *k == &after[..close]).map(|(_, v)| (close, v))) {
            Some((close, v)) => {
                out.push_str(v);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The key of a question: its version, the size of its images and its
/// whole text.
pub fn key(prompt: &str) -> String {
    let z = &catalog().zones;
    question_key(&z.version, &format!("{}\n{prompt}", catalog().ask.image_long_side))
}

/// The windows of a file of `pages` pages, over the parts with some
/// doubtful step: their question and key. `readings` by page.
pub fn windows(pages: u32, parts: &[Part], decisions: &[Decision], readings: &BTreeMap<u32, &PageReading>) -> Vec<Window> {
    let z = &catalog().zones;
    let (criteria, types) = criteria_and_types();
    let steps: BTreeMap<u32, &Decision> = decisions.iter().map(|d| (d.after, d)).collect();
    let mut out = Vec::new();
    for part in parts.iter().filter(|p| !p.doubtful.is_empty()) {
        let all = spans(part.first, part.last);
        for (index, (a, b)) in all.iter().copied().enumerate() {
            let number = index as u32 + 1;
            let texts: Vec<String> = (a..=b)
                .map(|pg| {
                    let (t, forma) = question_text(readings.get(&pg).copied(), z.text_max);
                    format!("--- Página {pg} (imagen {}) · {forma} ---\n{t}", pg - a + 1)
                })
                .collect();
            let mut hints: Vec<String> = (a..b).filter_map(|j| steps.get(&j)).map(|d| step_hint(d)).collect();
            if hints.is_empty() {
                hints.push("(una sola página: no hay saltos)".into());
            }
            let mut edge = if number == 1 {
                format!("La página {a} es el principio de una parte: ahí empieza un documento.")
            } else {
                format!(
                    "Este es el tramo {number} de {} de una parte más larga. La página {a} sigue algo que no ves: para ella respondé por lo que muestra, pero no se usa para cortar.",
                    all.len()
                )
            };
            if b < part.last {
                edge += &format!(" La parte sigue después de la página {b}.");
            }
            let prompt = fill(
                &z.prompt,
                &[
                    ("k", (b - a + 1).to_string()),
                    ("desde", a.to_string()),
                    ("hasta", b.to_string()),
                    ("total", pages.to_string()),
                    ("borde", edge + "\n"),
                    ("criterios", criteria.clone()),
                    ("tipos", types.clone()),
                    ("texto", texts.join("\n\n")),
                    ("pistas", hints.join("\n")),
                ],
            );
            out.push(Window {
                first: a,
                last: b,
                part: (part.first, part.last),
                number,
                windows: all.len() as u32,
                key: key(&prompt),
                prompt,
            });
        }
    }
    out
}

/// The shape of the answer.
pub fn schema() -> Value {
    let mut types: Vec<String> = catalog().ask.types.keys().cloned().collect();
    types.push(super::super::ask::OTHER.to_string());
    json!({
        "type": "object", "additionalProperties": false, "required": ["paginas", "documentos"],
        "properties": {
            "paginas": {"type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["pagina", "empieza_documento", "seguridad"],
                "properties": {"pagina": {"type": "integer"}, "empieza_documento": {"type": "boolean"},
                               "seguridad": {"type": "string", "enum": SURE}}}},
            "documentos": {"type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["primera_pagina", "tipo", "seguridad"],
                "properties": {"primera_pagina": {"type": "integer"}, "tipo": {"type": "string", "enum": types},
                               "seguridad": {"type": "string", "enum": SURE}}}},
        },
    })
}

/// Whether an answer has the shape asked for (its pages at least).
pub fn valid(answer: &str) -> Result<Value, String> {
    let v: Value = serde_json::from_str(answer).map_err(|e| format!("The answer about a window is not JSON: {e}"))?;
    if !v["paginas"].is_array() {
        return Err("The answer about a window has no pages".into());
    }
    Ok(v)
}

/// What one answer says of each page of its window (`(starts, sure)`) and
/// of each document (`(first page, type, sure)`). A model that numbered
/// the images 1..k instead of the pages of the file is moved.
pub fn answer_pages(first: u32, last: u32, answer: &Value) -> (BTreeMap<u32, (bool, String)>, Vec<(i64, String, String)>) {
    let pages: Vec<&Value> = answer["paginas"].as_array().map(|a| a.iter().filter(|p| p["pagina"].is_i64()).collect()).unwrap_or_default();
    let k = (last - first + 1) as i64;
    let (first, last) = (first as i64, last as i64);
    let nums: Vec<i64> = pages.iter().map(|p| p["pagina"].as_i64().unwrap()).collect();
    let moved = first > 1
        && !nums.is_empty()
        && nums.iter().all(|n| (1..=k).contains(n))
        && !nums.iter().any(|n| (first..=last).contains(n));
    let shift = if moved { first - 1 } else { 0 };
    let mut out = BTreeMap::new();
    for p in pages {
        let n = p["pagina"].as_i64().unwrap() + shift;
        let sure = p["seguridad"].as_str().unwrap_or_default();
        if (first..=last).contains(&n) && rank(sure).is_some() {
            out.insert(n as u32, (p["empieza_documento"].as_bool().unwrap_or(false), sure.to_string()));
        }
    }
    let mut docs = Vec::new();
    for d in answer["documentos"].as_array().into_iter().flatten() {
        if let Some(fp) = d["primera_pagina"].as_i64() {
            let s = |k: &str| d[k].as_str().unwrap_or_default().to_string();
            docs.push((fp + shift, s("tipo"), s("seguridad")));
        }
    }
    (out, docs)
}

/// What the model says of a page: whether a document starts there and how
/// sure, merged over the windows that see it.
#[derive(Debug, Clone, PartialEq)]
pub struct PageAnswer {
    pub starts: bool,
    pub sure: String,
}

/// Several answers for a page (starts, sure, window number): the least
/// sure. Two different answers equally sure: the first window's, «baja».
pub fn merge(candidates: &[(bool, String, u32)]) -> Option<PageAnswer> {
    let worst = candidates.iter().filter_map(|(_, s, _)| rank(s)).max()?;
    let low: Vec<&(bool, String, u32)> = candidates.iter().filter(|(_, s, _)| rank(s) == Some(worst)).collect();
    let first = low[0];
    if low.iter().any(|(e, _, _)| *e != first.0) {
        let earliest = low.iter().min_by_key(|(_, _, w)| *w).unwrap();
        return Some(PageAnswer { starts: earliest.0, sure: "baja".into() });
    }
    Some(PageAnswer { starts: first.0, sure: first.1.clone() })
}

/// The answers of a file, merged: for each page, what the windows that
/// see it say (the first page of a window that is not the first of its
/// part is not used to cut), and the type of each document by its first
/// page (a window's own answer first, then the surest).
pub fn merged(windows: &[(&Window, Option<Value>)]) -> (BTreeMap<u32, PageAnswer>, BTreeMap<u32, (String, String)>) {
    let mut candidates: BTreeMap<(u32, u32), BTreeMap<u32, Vec<(bool, String, u32)>>> = BTreeMap::new();
    let mut types: BTreeMap<(u32, u32), BTreeMap<i64, Vec<(bool, String, String)>>> = BTreeMap::new();
    for (w, answer) in windows {
        let Some(answer) = answer else { continue };
        let (pages, docs) = answer_pages(w.first, w.last, answer);
        let by_page = candidates.entry(w.part).or_default();
        for (pg, (e, s)) in pages {
            if w.number > 1 && pg == w.first {
                continue;
            }
            by_page.entry(pg).or_default().push((e, s, w.number));
        }
        let by_first = types.entry(w.part).or_default();
        for (fp, t, s) in docs {
            by_first.entry(fp).or_default().push((w.number > 1 && fp == w.first as i64, t, s));
        }
    }
    let mut pages = BTreeMap::new();
    for by_page in candidates.into_values() {
        for (pg, c) in by_page {
            if let Some(m) = merge(&c) {
                pages.insert(pg, m);
            }
        }
    }
    let mut out_types = BTreeMap::new();
    for by_first in types.into_values() {
        for (fp, mut ts) in by_first {
            ts.sort_by_key(|(edge, _, s)| (*edge, rank(s).unwrap_or(3)));
            if fp >= 1 {
                out_types.insert(fp as u32, (ts[0].1.clone(), ts[0].2.clone()));
            }
        }
    }
    (pages, out_types)
}
