//! Reads a PDF by zones (D-725): each page drawn at 2.000 pixels on its
//! long side, its zones by the model of this computer (`layout_local`)
//! and its lines by the reader of this computer (`recognition_local`),
//! or, on a page with a text layer of its own, by pdfium. Copy of
//! `leer.py` and `perfil.py` of the scripts of N, without the reader of
//! tables and the codes QR (the cut does not use them).
//!
//! Which pages are read (`perfil.capa_n`): the ones with few letters of
//! their own under a picture or many drawn strokes (a scan), and the ones
//! whose layer is an old recognition (under a picture that covers the
//! page, or of words that do not look like words). The old layer of these
//! is the second witness of their lines (D-671).

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;

use super::page::{band_counts, flags, round_to, structure, Bands, Evidence, Flags, Line, ModelZone, Zone};
use super::words::{junk, letter_runs};
use crate::docbase::cut::by_zones::py::{is_space, splitlines};
use crate::source_volume::{layout_local, recognition_local};

/// Pixels of the long side of a page drawn to be read.
pub const LONG_SIDE: f64 = 2000.0;
/// How the pages are read, in each reading.
pub const RECIPE: &str = "n-zonas-rust:pp_doc_layoutv2+rapidocr-onnx:ppocrv6-small";

/// What pdfium alone tells of a page (`perfil.py`).
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct Profile {
    pub chars: usize,
    pub own_chars: usize,
    pub single: f64,
    pub dudosas: f64,
    #[serde(skip)]
    pub palabras: usize,
    pub cover: f64,
    #[serde(skip)]
    pub images: usize,
    #[serde(skip)]
    pub paths: usize,
}

/// How a page is made, as its reading tells it: its layer, whether it was
/// read, its shape (`digital`, `scan`, `blank_or_footer`), its mark of a
/// signature, its marks and what pdfium told of it. Kept in the reading of
/// each page: it is the ficha of shape of the cut by zones.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Shape {
    pub capa: String,
    pub se_lee: bool,
    pub forma: &'static str,
    pub firma: &'static str,
    pub banderas: Flags,
    pub perfil: Profile,
}

/// The reading by zones of one page, as the reader of N writes it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PageOut {
    pub page: u32,
    pub tamano_px: [u32; 2],
    pub forma: Shape,
    pub zonas: Vec<Zone>,
    pub evidencia: Vec<Evidence>,
    pub texto_firme: String,
    pub texto_aproximado: String,
    pub texto_seguro: String,
    pub franjas: std::collections::BTreeMap<&'static str, usize>,
    pub receta: String,
}

fn visible(s: &str) -> usize {
    s.chars().filter(|c| !is_space(*c)).count()
}

fn norm_line(line: &str) -> String {
    static DIGITS: OnceLock<Regex> = OnceLock::new();
    let joined = line.split(is_space).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ").to_lowercase();
    DIGITS.get_or_init(|| Regex::new(r"\d+").unwrap()).replace_all(&joined, "#").into_owned()
}

/// The profile of each page from its text, the share of the page its
/// pictures cover and how many strokes it draws (`perfil_pdf`).
pub fn profiles(pages: &[(String, f64, usize, usize)]) -> Vec<Profile> {
    let n = pages.len();
    let mut repeated: HashSet<String> = HashSet::new();
    if n >= 3 {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for (text, ..) in pages {
            let lines: HashSet<String> = splitlines(text).into_iter().filter(|l| visible(l) >= 8).map(norm_line).collect();
            for l in lines {
                *counts.entry(l).or_default() += 1;
            }
        }
        repeated = counts.into_iter().filter(|(_, c)| *c as f64 / n as f64 >= 0.6).map(|(l, _)| l).collect();
    }
    pages
        .iter()
        .map(|(text, cover, images, paths)| {
            let own: Vec<&str> = splitlines(text).into_iter().filter(|l| visible(l) > 0 && !repeated.contains(&norm_line(l))).collect();
            let own_t = own.join("\n");
            let words = letter_runs(&own_t);
            let single = if words.is_empty() { 0.0 } else { words.iter().filter(|w| w.chars().count() == 1).count() as f64 / words.len() as f64 };
            let (dud, nw) = junk(&own_t);
            Profile {
                chars: visible(text),
                own_chars: visible(&own_t),
                single: round_to(single, 3),
                dudosas: round_to(dud, 3),
                palabras: nw,
                cover: round_to(*cover, 3),
                images: *images,
                paths: *paths,
            }
        })
        .collect()
}

/// The layer of a page and whether it is read (`capa_n`).
pub fn layer(p: &Profile) -> (&'static str, bool) {
    let without = if p.chars == 0 { "none" } else { "footer_only" };
    if p.own_chars < 80 {
        return (without, p.cover >= 0.25 || p.paths >= 200);
    }
    if p.cover >= 0.85 {
        return ("old_ocr", true);
    }
    if p.dudosas >= 0.35 && p.palabras >= 20 {
        return ("old_ocr", true);
    }
    ("digital", false)
}

/// One letter of pdfium in the pixels of the page drawn: left, top,
/// right, bottom and the letter.
pub type Char = (f64, f64, f64, f64, String);

/// The letters of a text layer placed in the page drawn
/// (`chars_por_posicion`): a space takes the height of the letter before
/// it, so words are not joined; line breaks are left out.
pub fn placed_chars(raw: &[(String, f64, f64, f64, f64)], height_px: f64, scale: f64) -> Vec<Char> {
    let mut out: Vec<Char> = Vec::new();
    for (ch, l, b, r, t) in raw {
        if ch.chars().all(is_space) {
            if ch == "\r" || ch == "\n" || out.is_empty() {
                continue;
            }
            let (_, y0, x1, y1, _) = out.last().cloned().unwrap();
            let x = x1.max(l * scale);
            out.push((x, y0, x + 0.01, y1, " ".into()));
            continue;
        }
        out.push((l * scale, height_px - t * scale, r * scale, height_px - b * scale, ch.clone()));
    }
    out
}

/// The letters gathered in lines by their base (`renglones_de_chars`).
pub fn char_lines(chars: &[Char]) -> Vec<Line> {
    let mut keys: Vec<i64> = Vec::new();
    let mut rows: HashMap<i64, Vec<Char>> = HashMap::new();
    for c in chars {
        let (_, y0, _, y1, _) = c;
        let y = y1.round_ties_even() as i64;
        let key = keys.iter().copied().find(|k| ((k - y) as f64).abs() <= 4f64.max(0.35 * (y1 - y0))).unwrap_or(y);
        if !rows.contains_key(&key) {
            keys.push(key);
        }
        rows.entry(key).or_default().push(c.clone());
    }
    keys.sort();
    let mut out = Vec::new();
    for k in keys {
        let mut cs = rows.remove(&k).unwrap_or_default();
        cs.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.total_cmp(&b.2)).then(a.1.total_cmp(&b.1)).then(a.3.total_cmp(&b.3)).then(a.4.cmp(&b.4)));
        let mut line = String::new();
        let mut last: Option<f64> = None;
        for (x0, _, x1, _, ch) in &cs {
            if last.is_some_and(|l| x0 - l > 15.0) {
                line.push_str("  ");
            }
            line.push_str(ch);
            last = Some(*x1);
        }
        let text = line.split(is_space).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
        if text.is_empty() {
            continue;
        }
        let bbox = [
            cs.iter().map(|c| c.0).fold(f64::INFINITY, f64::min) as i64,
            cs.iter().map(|c| c.1).fold(f64::INFINITY, f64::min) as i64,
            cs.iter().map(|c| c.2).fold(f64::NEG_INFINITY, f64::max) as i64,
            cs.iter().map(|c| c.3).fold(f64::NEG_INFINITY, f64::max) as i64,
        ];
        out.push(Line { bbox, text, score: None });
    }
    out
}

/// The reading of one page from what was drawn and read of it.
pub fn page_out(
    page: u32,
    size: (u32, u32),
    profile: &Profile,
    lines: &[Line],
    zones: &[ModelZone],
    bands: Bands,
    old_layer: Option<&str>,
) -> PageOut {
    let (capa, read) = layer(profile);
    let s = structure(lines, zones, bands, old_layer, (layout_local::MODEL_NAME, "rapidocr-onnx:ppocrv6-small"));
    let (banderas, firma, blank) = flags(lines, zones, bands.low, read, profile.own_chars);
    let forma = if blank {
        "blank_or_footer"
    } else if read {
        "scan"
    } else {
        "digital"
    };
    PageOut {
        page,
        tamano_px: [size.0, size.1],
        forma: Shape { capa: capa.to_string(), se_lee: read, forma, firma, banderas, perfil: profile.clone() },
        franjas: band_counts(&s.zones),
        zonas: s.zones,
        evidencia: s.evidence,
        texto_seguro: s.firm.clone(),
        texto_firme: s.firm,
        texto_aproximado: s.approximate,
        receta: format!("{RECIPE}|alto {}|bajo {}", bands.high, bands.low),
    }
}

/// What pdfium gives of one page to be read: its picture, its lines when
/// it has a layer of its own, and the old layer when it is one.
struct Drawn {
    picture: image::RgbImage,
    lines: Vec<Line>,
    old_layer: Option<String>,
}

fn open<'a>(pdfium: &'a pdfium_render::prelude::Pdfium, pdf: &Path) -> Result<pdfium_render::prelude::PdfDocument<'a>, String> {
    pdfium.load_pdf_from_file(pdf, None).map_err(|e| format!("Failed to open PDF '{}': {e}", pdf.display()))
}

/// The text, the cover of pictures, the pictures and the strokes of every
/// page.
fn pdf_pages(pdf: &Path) -> Result<Vec<(String, f64, usize, usize)>, String> {
    use pdfium_render::prelude::*;
    fn visit(object: &PdfPageObject<'_>, depth: usize, size: (f64, f64), grid: &mut [bool], counts: &mut (usize, usize)) {
        match object.object_type() {
            PdfPageObjectType::Image => {
                counts.0 += 1;
                let (w, h) = size;
                if let Ok(q) = object.bounds() {
                    let r = q.to_rect();
                    let (l, b, rr, t) = (r.left().value as f64, r.bottom().value as f64, r.right().value as f64, r.top().value as f64);
                    let cell = |v: f64| (v.max(0.0) as i64).clamp(0, 200) as usize;
                    let (x0, x1) = (cell(l.max(0.0) / w * 200.0), cell(rr.min(w) / w * 200.0 + 0.999));
                    let (y0, y1) = (cell(b.max(0.0) / h * 200.0), cell(t.min(h) / h * 200.0 + 0.999));
                    for y in y0..y1 {
                        for x in x0..x1 {
                            grid[y * 200 + x] = true;
                        }
                    }
                }
            }
            PdfPageObjectType::Path => counts.1 += 1,
            PdfPageObjectType::XObjectForm if depth < 3 => {
                if let Some(form) = object.as_x_object_form_object() {
                    for child in form.iter() {
                        visit(&child, depth + 1, size, grid, counts);
                    }
                }
            }
            _ => {}
        }
    }
    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = open(pdfium, pdf)?;
    let mut out = Vec::new();
    for page in doc.pages().iter() {
        let text = page.text().map(|t| t.all()).unwrap_or_default();
        let size = (page.width().value as f64, page.height().value as f64);
        let mut grid = vec![false; 200 * 200];
        let mut counts = (0, 0);
        if size.0 > 0.0 && size.1 > 0.0 {
            for object in page.objects().iter() {
                visit(&object, 0, size, &mut grid, &mut counts);
            }
        }
        let cover = grid.iter().filter(|g| **g).count() as f64 / grid.len() as f64;
        out.push((text, cover, counts.0, counts.1));
    }
    Ok(out)
}

/// Draws a page at `LONG_SIDE` pixels on its long side (as `pypdfium2`:
/// each side rounded up), without the data of its forms; with its own
/// lines when it is not to be read.
fn draw(pdf: &Path, page_no: u32, read: bool, old: bool) -> Result<Drawn, String> {
    use pdfium_render::prelude::*;
    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = open(pdfium, pdf)?;
    let page = doc.pages().get((page_no - 1) as i32).map_err(|e| format!("Page {page_no} not found: {e}"))?;
    let (w, h) = (page.width().value as f64, page.height().value as f64);
    let scale = LONG_SIDE / w.max(h).max(1e-6);
    let (pw, ph) = ((w * scale).ceil().max(1.0) as i32, (h * scale).ceil().max(1.0) as i32);
    // The page fills the picture, as `FPDF_RenderPageBitmap` draws it.
    let config = PdfRenderConfig::new()
        .set_fixed_size(pw, ph)
        .scale_page_width_by_factor((pw as f64 / w.max(1e-6)) as f32)
        .scale_page_height_by_factor((ph as f64 / h.max(1e-6)) as f32)
        .render_form_data(false)
        .render_annotations(true);
    let picture = page
        .render_with_config(&config)
        .map_err(|e| format!("Page {page_no} could not be rendered: {e}"))?
        .as_image()
        .map_err(|e| format!("Page {page_no} could not be converted to an image: {e}"))?
        .to_rgb8();
    let text = page.text().ok();
    let lines = match (&text, read) {
        (Some(t), false) => {
            let raw: Vec<(String, f64, f64, f64, f64)> = t
                .chars()
                .iter()
                .map(|c| {
                    let ch = c.unicode_char().filter(|c| *c != '\0').map(String::from).unwrap_or_default();
                    let r = c.loose_bounds().ok();
                    let v = |f: fn(&PdfRect) -> PdfPoints| r.as_ref().map_or(0.0, |r| f(r).value as f64);
                    (ch, v(PdfRect::left), v(PdfRect::bottom), v(PdfRect::right), v(PdfRect::top))
                })
                .collect();
            char_lines(&placed_chars(&raw, h * scale, scale))
        }
        _ => Vec::new(),
    };
    let old_layer = if old { text.map(|t| t.all()) } else { None };
    Ok(Drawn { picture, lines, old_layer })
}

/// Reads every page of a PDF by zones, with the thresholds `bands`. Each
/// page is drawn with pdfium held, and read with it free.
pub fn read_pdf(pdf: &Path, bands: Bands) -> Result<Vec<PageOut>, String> {
    let pages = pdf_pages(pdf)?;
    let profiles = profiles(&pages);
    let mut out = Vec::with_capacity(profiles.len());
    for (i, profile) in profiles.iter().enumerate() {
        let page_no = i as u32 + 1;
        let (capa, read) = layer(profile);
        let drawn = draw(pdf, page_no, read, capa == "old_ocr")?;
        let zones: Vec<ModelZone> = {
            let mut model = layout_local::model()?;
            layout_local::zones(&mut model, &drawn.picture)?
                .into_iter()
                .map(|z| ModelZone { bbox: z.bbox, class: z.class, score: z.score })
                .collect()
        };
        let lines = if read {
            let mut ocr = recognition_local::engine()?;
            recognition_local::read_picture(&mut ocr, &drawn.picture)?
                .into_iter()
                .map(|l| Line { bbox: l.bbox, text: l.text, score: Some(round_to(l.score as f64, 4)) })
                .collect()
        } else {
            drawn.lines
        };
        out.push(page_out(page_no, drawn.picture.dimensions(), profile, &lines, &zones, bands, drawn.old_layer.as_deref()));
    }
    Ok(out)
}
