//! The signals of each page the cutter decides with: how the page is made
//! (its ficha of shape, `page_shape`), its size and its main image as
//! pdfium reads them, and the patterns of its own text (`cut.json`).

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use crate::source_volume::page_shape::{normalized_line, repeated_lines, visible_chars, PageShape, PdfShapes};

use super::catalog::{catalog, fold};

/// How a page is made, as the cutter compares two pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Making {
    Scan,
    Digital,
    /// Blank or with only the footer.
    Blank,
}

/// The largest image of a page: its pixels, its resolution on the page
/// and its color space.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MainImage {
    pub pixels: (u32, u32),
    pub dpi: u32,
    pub color: String,
}

/// What pdfium tells of a page for the cutter: its size in points (as it
/// is shown) and its main image.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageScan {
    pub size: (f64, f64),
    pub image: Option<MainImage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PageSignals {
    /// 1-based.
    pub page: u32,
    pub size: (f64, f64),
    pub making: Making,
    pub image: Option<MainImage>,
    /// «Página x de y» of its own text: (x, y), with 0 < x <= y.
    pub page_of: Vec<(u32, u32)>,
    pub codes: BTreeSet<String>,
    /// Notarial sheet serials: (letters, number).
    pub serials: Vec<(String, u64)>,
    pub machine_zone: bool,
    /// Its own lines, folded, without the empty ones.
    pub lines: Vec<String>,
}

impl PageSignals {
    /// A page with nothing but how it is made, for the tests.
    #[cfg(test)]
    pub fn bare(page: u32, making: Making) -> Self {
        Self {
            page,
            size: (595.0, 842.0),
            making,
            image: None,
            page_of: Vec::new(),
            codes: BTreeSet::new(),
            serials: Vec::new(),
            machine_zone: false,
            lines: Vec::new(),
        }
    }
}

/// The size and the main image of every page of a PDF.
pub fn scan_pages(pdf: &Path) -> Result<Vec<PageScan>, String> {
    use pdfium_render::prelude::*;

    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(pdf, None)
        .map_err(|e| format!("Failed to open PDF '{}': {e}", pdf.display()))?;
    let mut pages = Vec::new();
    for page in doc.pages().iter() {
        let (width, height) = (page.width().value as f64, page.height().value as f64);
        // (share of the page, width in points) of an object.
        let place = |object: &PdfPageObject<'_>| -> (f64, f64) {
            if width <= 0.0 || height <= 0.0 {
                return (0.0, 0.0);
            }
            object
                .bounds()
                .map(|b| {
                    let b = b.to_rect();
                    let w = (b.right().value as f64).min(width) - (b.left().value as f64).max(0.0);
                    let h = (b.top().value as f64).min(height) - (b.bottom().value as f64).max(0.0);
                    ((w.max(0.0) * h.max(0.0)) / (width * height), (b.right().value - b.left().value) as f64)
                })
                .unwrap_or((0.0, 0.0))
        };
        let metadata = |image: &PdfPageImageObject<'_>, shown_width: f64| -> MainImage {
            let pixels = (image.width().unwrap_or(0).max(0) as u32, image.height().unwrap_or(0).max(0) as u32);
            let dpi = if shown_width > 0.0 { (pixels.0 as f64 / (shown_width / 72.0)).round() as u32 } else { 0 };
            let color = image.color_space().map(|c| format!("{c:?}")).unwrap_or_default();
            MainImage { pixels, dpi, color }
        };
        let mut best: Option<(f64, MainImage)> = None;
        for object in page.objects().iter() {
            let found = match object.object_type() {
                PdfPageObjectType::Image => object.as_image_object().map(|image| {
                    let (cover, shown) = place(&object);
                    (cover, metadata(image, shown))
                }),
                // A scan wrapped in a group: the group's place is the image's.
                PdfPageObjectType::XObjectForm => object.as_x_object_form_object().and_then(|group| {
                    let inner = group.iter().find(|o| o.object_type() == PdfPageObjectType::Image)?;
                    let image = inner.as_image_object()?;
                    let (cover, shown) = place(&object);
                    Some((cover, metadata(image, shown)))
                }),
                _ => None,
            };
            if let Some((cover, image)) = found {
                if best.as_ref().is_none_or(|(c, _)| cover > *c) {
                    best = Some((cover, image));
                }
            }
        }
        pages.push(PageScan { size: (width, height), image: best.map(|(_, image)| image) });
    }
    Ok(pages)
}

/// The signals of every page of a file, from its ficha, what pdfium read
/// and the text of each page (in order; a missing page has no text).
pub fn signals(shapes: &PdfShapes, scans: &[PageScan], texts: &[String]) -> Vec<PageSignals> {
    let repeated: HashSet<String> = repeated_lines(texts);
    let patterns = &catalog().patterns;
    shapes
        .pages
        .iter()
        .enumerate()
        .map(|(index, profile)| {
            let text = texts.get(index).map(String::as_str).unwrap_or("");
            let own: Vec<&str> =
                text.lines().filter(|l| visible_chars(l) > 0 && !repeated.contains(&normalized_line(l))).collect();
            let own_text = own.join("\n");
            let scan = scans.get(index).cloned().unwrap_or_default();
            let making = match profile.shape {
                PageShape::Scan => Making::Scan,
                PageShape::BlankOrFooter => Making::Blank,
                _ => Making::Digital,
            };
            let page_of = patterns
                .page_of
                .captures_iter(&own_text)
                .filter_map(|c| Some((c[1].parse::<u32>().ok()?, c[2].parse::<u32>().ok()?)))
                .filter(|(x, y)| *x > 0 && x <= y)
                .collect();
            let codes = patterns.document_code.captures_iter(&own_text).map(|c| c[1].to_string()).collect();
            let serials: BTreeSet<(String, u64)> = patterns
                .sheet_serial
                .captures_iter(&own_text)
                .filter_map(|c| Some((c[1].to_string(), c[2].parse::<u64>().ok()?)))
                .collect();
            PageSignals {
                page: profile.page,
                size: scan.size,
                making,
                image: scan.image,
                page_of,
                codes,
                serials: serials.into_iter().collect(),
                machine_zone: patterns.machine_zone.is_match(text),
                lines: own.iter().map(|l| fold(l)).filter(|l| !l.trim().is_empty()).collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_volume::page_shape::{PageProfile, TextLayer};

    fn profile(page: u32, shape: PageShape) -> PageProfile {
        PageProfile {
            page,
            chars: 0,
            own_chars: 0,
            own_lines: 0,
            short_lines: 0.0,
            single_letters: 0.0,
            largest_image: 0.0,
            images_cover: 0.0,
            group_values: 0,
            fields: 0,
            drawn: 0,
            shape,
            text_layer: TextLayer::Digital,
            read: false,
        }
    }

    #[test]
    fn the_patterns_of_the_own_text_are_read_and_the_repeated_lines_left_out() {
        let shapes = PdfShapes {
            producer: String::new(),
            pages: (1..=3).map(|p| profile(p, PageShape::DigitalProse)).collect(),
        };
        let footer = "Sistema inventado de prueba - hoja 1 de 3";
        let texts = vec![
            format!("Señor Juez:\nPágina 1 de 2\nABC-2026-00123456-XYZ-QQ\n{footer}"),
            format!("Foja AB 012345678 sigue\npág. 2/2\n{footer}"),
            format!("P<ZZZINVENTADO<<NOMBRE<<<<<<<<<<<<\n{footer}"),
        ];
        let s = signals(&shapes, &[], &texts);
        assert_eq!(s[0].page_of, vec![(1, 2)]);
        assert_eq!(s[0].codes.iter().cloned().collect::<Vec<_>>(), vec!["ABC-2026-00123456-XYZ-QQ".to_string()]);
        assert_eq!(s[0].lines[0], "senor juez:");
        assert_eq!(s[1].page_of, vec![(2, 2)]);
        assert_eq!(s[1].serials, vec![("AB".to_string(), 12345678)]);
        assert!(s[2].machine_zone);
        assert!(s.iter().all(|p| !p.lines.iter().any(|l| l.contains("sistema inventado"))));
    }
}
