//! What pdfium tells of each page for the cut by zones: its paper (in
//! points, rounded), how it is made (the shapes of `page-shapes.json`,
//! telling a picture and an old recognition layer apart from a scan) and
//! its main image (pixels, resolution, bits and color space). Only hints
//! come from here: the shape, the paper and the image never decide.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;
use serde::Serialize;

use super::py::{round, splitlines};
use super::py::is_space;

/// The largest image of a page.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct MainImage {
    pub px: (i64, i64),
    pub dpi: i64,
    pub bpp: i64,
    pub cs: i64,
    pub cover: f64,
}

/// One page as pdfium reads it.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct PageScan {
    pub w: i64,
    pub h: i64,
    /// `scan`, `scan_old_ocr`, `picture`, `blank`, `digital_prose`,
    /// `digital_form` or `flattened_form`; empty when pdfium could not read
    /// the file.
    pub shape: String,
    pub image: Option<MainImage>,
    pub chars: usize,
    pub own_chars: usize,
    pub own_lines: usize,
    pub images_cover: f64,
    pub drawn: usize,
    pub fields: usize,
    pub group_values: usize,
}

struct Shapes {
    repeated_share: f64,
    repeated_min_pages: usize,
    repeated_min_line_chars: usize,
    scan_cover: f64,
    scan_layer_min_chars: usize,
    picture_cover: f64,
    picture_max_own_chars: usize,
    drawn_min_objects: usize,
    blank_max_own_chars: usize,
    form_short_line_chars: usize,
    form_short_line_share: f64,
    form_min_lines: usize,
    form_min_fields: usize,
    form_flattened_min_values: usize,
    signing_services: Vec<String>,
}

fn shapes() -> &'static Shapes {
    static S: OnceLock<Shapes> = OnceLock::new();
    S.get_or_init(|| {
        let v: serde_json::Value =
            serde_json::from_str(crate::source_volume::page_shape::CATALOG).expect("page-shapes.json is valid");
        let f = |a: &str, b: &str| v[a][b].as_f64().expect("a number of page-shapes.json");
        let u = |a: &str, b: &str| f(a, b) as usize;
        Shapes {
            repeated_share: f("repeated", "share"),
            repeated_min_pages: u("repeated", "minPages"),
            repeated_min_line_chars: u("repeated", "minLineChars"),
            scan_cover: f("scan", "imageCover"),
            scan_layer_min_chars: u("scan", "layerMinChars"),
            picture_cover: f("picture", "imageCover"),
            picture_max_own_chars: u("picture", "maxOwnChars"),
            drawn_min_objects: u("drawn", "minObjects"),
            blank_max_own_chars: u("blank", "maxOwnChars"),
            form_short_line_chars: u("form", "shortLineChars"),
            form_short_line_share: f("form", "shortLineShare"),
            form_min_lines: u("form", "minLines"),
            form_min_fields: u("form", "minFields"),
            form_flattened_min_values: u("form", "flattenedMinValues"),
            signing_services: v["form"]["signingServices"]
                .as_array()
                .map(|a| a.iter().filter_map(|s| s.as_str().map(str::to_string)).collect())
                .unwrap_or_default(),
        }
    })
}

fn visible(s: &str) -> usize {
    s.chars().filter(|c| !is_space(*c)).count()
}

fn norm_line(line: &str) -> String {
    static DIGIT: OnceLock<Regex> = OnceLock::new();
    let joined = line.split(is_space).filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" ");
    DIGIT.get_or_init(|| Regex::new(r"\d").unwrap()).replace_all(&joined, "#").into_owned()
}

/// The lines (normalized) on most pages of a file.
fn repeated_lines(texts: &[String]) -> HashSet<String> {
    let s = shapes();
    let n = texts.len();
    if n < s.repeated_min_pages {
        return HashSet::new();
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for t in texts {
        let lines: HashSet<String> =
            splitlines(t).into_iter().map(norm_line).filter(|l| l.chars().count() >= s.repeated_min_line_chars).collect();
        for l in lines {
            *counts.entry(l).or_default() += 1;
        }
    }
    let need = (s.repeated_share * n as f64).ceil().max(s.repeated_min_pages as f64);
    counts.into_iter().filter(|(_, c)| *c as f64 >= need).map(|(l, _)| l).collect()
}

fn classify(p: &PageScan, short_lines: f64, signed: bool) -> &'static str {
    let s = shapes();
    if p.images_cover >= s.scan_cover {
        return if p.own_chars < s.scan_layer_min_chars { "scan" } else { "scan_old_ocr" };
    }
    let picture = p.images_cover >= s.picture_cover && p.own_chars < s.picture_max_own_chars;
    let drawn = p.own_chars < s.blank_max_own_chars && p.drawn >= s.drawn_min_objects;
    if picture {
        return "picture";
    }
    if drawn {
        return "scan";
    }
    if p.own_chars < s.blank_max_own_chars {
        return "blank";
    }
    let form = (p.own_lines >= s.form_min_lines && short_lines >= s.form_short_line_share) || p.fields >= s.form_min_fields;
    if form && (p.group_values >= s.form_flattened_min_values || signed) {
        return "flattened_form";
    }
    if form {
        "digital_form"
    } else {
        "digital_prose"
    }
}

/// A number rounded to three decimals, as the images are ordered.
fn round3(x: f64) -> f64 {
    format!("{x:.3}").parse().unwrap_or(x)
}

#[derive(Default)]
struct Objects {
    images: Vec<MainImage>,
    cover: f64,
    group_values: usize,
    drawn: usize,
}

fn visit(object: &pdfium_render::prelude::PdfPageObject<'_>, inside: bool, size: (f64, f64), out: &mut Objects) {
    use pdfium_render::prelude::*;
    match object.object_type() {
        PdfPageObjectType::Image => {
            let (w, h) = size;
            let (l, b, r, t) = object
                .bounds()
                .map(|q| {
                    let q = q.to_rect();
                    (q.left().value as f64, q.bottom().value as f64, q.right().value as f64, q.top().value as f64)
                })
                .unwrap_or((0.0, 0.0, 0.0, 0.0));
            let area = w * h;
            let a = if area > 0.0 { (r.min(w) - l.max(0.0)).max(0.0) * (t.min(h) - b.max(0.0)).max(0.0) / area } else { 0.0 };
            out.cover += a;
            let (px, bpp, cs) = match object.as_image_object() {
                Some(image) => match (image.width(), image.height()) {
                    (Ok(pw), Ok(ph)) => (
                        (pw as i64, ph as i64),
                        image.bits_per_pixel().map(|v| v as i64).unwrap_or(0),
                        image.color_space().map(|c| c as i64).unwrap_or(0),
                    ),
                    _ => ((0, 0), 0, 0),
                },
                None => ((0, 0), 0, 0),
            };
            let dpi = if r > l { round(px.0 as f64 / ((r - l) / 72.0)) } else { 0 };
            out.images.push(MainImage { px, dpi, bpp, cs, cover: round3(a) });
        }
        PdfPageObjectType::Text if inside => out.group_values += 1,
        PdfPageObjectType::Path => out.drawn += 1,
        PdfPageObjectType::XObjectForm => {
            if let Some(form) = object.as_x_object_form_object() {
                for child in form.iter() {
                    visit(&child, true, size, out);
                }
            }
        }
        _ => {}
    }
}

/// Every page of a PDF.
pub fn scan_pdf(pdf: &Path) -> Result<Vec<PageScan>, String> {
    use pdfium_render::prelude::*;

    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(pdf, None)
        .map_err(|e| format!("Failed to open PDF '{}': {e}", pdf.display()))?;
    let producer =
        doc.metadata().get(PdfDocumentMetadataTagType::Producer).map(|m| m.value().to_string()).unwrap_or_default().to_lowercase();
    let signed = shapes().signing_services.iter().any(|s| producer.contains(s.as_str()));
    let texts: Vec<String> = doc
        .pages()
        .iter()
        .map(|page| {
            page.text()
                .map(|t| t.chars().iter().filter_map(|c| c.unicode_char()).filter(|c| *c != '\0').collect())
                .unwrap_or_default()
        })
        .collect();
    let repeated = repeated_lines(&texts);
    let mut out = Vec::new();
    for (index, page) in doc.pages().iter().enumerate() {
        let (w, h) = (page.width().value as f64, page.height().value as f64);
        let text = &texts[index];
        let own: Vec<&str> = splitlines(text).into_iter().filter(|l| visible(l) > 0 && !repeated.contains(&norm_line(l))).collect();
        let mut objects = Objects::default();
        for object in page.objects().iter() {
            visit(&object, false, (w, h), &mut objects);
        }
        let fields = page.annotations().iter().filter(|a| a.annotation_type() == PdfPageAnnotationType::Widget).count();
        // The largest first; the first of them on a tie.
        let mut images = objects.images;
        images.sort_by(|a, b| b.cover.total_cmp(&a.cover));
        let mut p = PageScan {
            w: round(w),
            h: round(h),
            shape: String::new(),
            image: images.into_iter().next(),
            chars: visible(text),
            own_chars: own.iter().map(|l| visible(l)).sum(),
            own_lines: own.len(),
            images_cover: objects.cover.min(1.0),
            drawn: objects.drawn,
            fields,
            group_values: objects.group_values,
        };
        let short = shapes().form_short_line_chars;
        let short_lines =
            if own.is_empty() { 0.0 } else { own.iter().filter(|l| visible(l) <= short).count() as f64 / own.len() as f64 };
        p.shape = classify(&p, short_lines, signed).to_string();
        out.push(p);
    }
    Ok(out)
}
