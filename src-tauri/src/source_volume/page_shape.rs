//! The shape of each page of a PDF (P-320): how the page is made, from
//! cheap signals read with pdfium and without a model, so the base knows
//! which pages carry their text as pixels and need recognition.
//!
//! The signals of a page (its «ficha»):
//! - its own text: the characters of the page without the lines repeated
//!   on most pages of the file (a footer or a seal a system stamps on
//!   every sheet), digits normalized;
//! - the largest image and how much of the page the images cover;
//! - the share of single letters among its words (an old recognition
//!   layer puts spaces between letters);
//! - the text inside groups of objects (values added on top of a form, as
//!   a signing service flattens it) and the fillable fields;
//! - the producer of the file.
//!
//! The thresholds are in the catalog (`page-shapes.json`, R-26), where a
//! person can read them. Handwriting is not told apart here (D-518).

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use super::page_print::{PagePrint, PRINT_HEIGHT_PX};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

const CATALOG: &str = include_str!("../../resources/catalog/page-shapes.json");
/// What wrote the fichas kept in the base: a ficha of an earlier version
/// is computed again.
pub const SHAPES_ENGINE: &str = "page-shape/1";

/// How a page is made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PageShape {
    DigitalProse,
    DigitalForm,
    FlattenedForm,
    Scan,
    BlankOrFooter,
}

/// The digital text the file carries for a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextLayer {
    /// The page's own text, written by the program that made it.
    Digital,
    None,
    /// Only the footer or seal a system stamps on every page.
    FooterOnly,
    /// The layer of an earlier recognition, under the image of the scan.
    OldOcr,
}

/// The ficha of one page: its signals and the shape they give.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageProfile {
    /// 1-based.
    pub page: u32,
    /// Non-whitespace characters of the page's text.
    pub chars: usize,
    /// The same, without the lines repeated on most pages.
    pub own_chars: usize,
    /// Lines of its own text, and the share of them that are short.
    pub own_lines: usize,
    pub short_lines: f64,
    /// Share of single letters among the words of its own text.
    pub single_letters: f64,
    /// Share of the page the largest image covers, and all of them.
    pub largest_image: f64,
    pub images_cover: f64,
    /// Texts inside groups of objects drawn on the page.
    pub group_values: usize,
    /// Fillable fields of the page.
    pub fields: usize,
    /// Drawing strokes of the page.
    pub drawn: usize,
    pub shape: PageShape,
    pub text_layer: TextLayer,
    /// Whether it goes to recognition: a scan without text of its own, or
    /// with a noisy old layer.
    pub read: bool,
}

/// The fichas of a PDF.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfShapes {
    /// Producer and creator of the file, as its metadata says.
    pub producer: String,
    pub pages: Vec<PageProfile>,
}

impl PdfShapes {
    /// The pages that go to recognition, in order.
    pub fn to_read(&self) -> Vec<u32> {
        self.pages.iter().filter(|p| p.read).map(|p| p.page).collect()
    }

    /// The pages whose text is an old recognition layer: what is read
    /// again replaces it.
    pub fn old_layers(&self) -> BTreeSet<u32> {
        self.pages.iter().filter(|p| p.text_layer == TextLayer::OldOcr).map(|p| p.page).collect()
    }

    /// The pages the test used before this ficha sent to recognition
    /// (fewer characters than the catalog's `earlier.minChars`).
    pub fn read_by_the_earlier_test(&self) -> BTreeSet<u32> {
        let min = rules().earlier.min_chars;
        self.pages.iter().filter(|p| p.chars < min).map(|p| p.page).collect()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Rules {
    repeated: RepeatedRules,
    scan: ScanRules,
    picture: PictureRules,
    drawn: DrawnRules,
    blank: BlankRules,
    form: FormRules,
    earlier: EarlierRules,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RepeatedRules {
    share: f64,
    min_pages: usize,
    min_line_chars: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanRules {
    image_cover: f64,
    layer_min_chars: usize,
    noisy_single_letters: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PictureRules {
    image_cover: f64,
    max_own_chars: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DrawnRules {
    min_objects: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlankRules {
    max_own_chars: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormRules {
    short_line_chars: usize,
    short_line_share: f64,
    min_lines: usize,
    min_fields: usize,
    flattened_min_values: usize,
    signing_services: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EarlierRules {
    min_chars: usize,
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| serde_json::from_str(CATALOG).expect("page-shapes.json is valid"))
}

/// A line as compared between pages: spaces collapsed, digits as `#`.
fn normalized_line(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

fn visible_chars(text: &str) -> usize {
    text.chars().filter(|c| !c.is_whitespace()).count()
}

/// The lines repeated on most pages of a file of these page texts.
fn repeated_lines(texts: &[String]) -> HashSet<String> {
    let r = &rules().repeated;
    let n = texts.len();
    if n < r.min_pages {
        return HashSet::new();
    }
    let mut counts: HashMap<String, usize> = HashMap::new();
    for text in texts {
        let lines: HashSet<String> =
            text.lines().map(normalized_line).filter(|l| l.chars().count() >= r.min_line_chars).collect();
        for line in lines {
            *counts.entry(line).or_default() += 1;
        }
    }
    let needed = (r.share * n as f64).ceil().max(r.min_pages as f64) as usize;
    counts.into_iter().filter(|(_, count)| *count >= needed).map(|(line, _)| line).collect()
}

/// What pdfium tells of one page, before its shape is decided.
#[derive(Debug, Clone, Default)]
pub(crate) struct RawPage {
    pub text: String,
    pub largest_image: f64,
    pub images_cover: f64,
    pub group_values: usize,
    pub fields: usize,
    pub drawn: usize,
}

/// The fichas of a file from what pdfium read of its pages.
pub(crate) fn shapes_of(producer: String, raw: Vec<RawPage>) -> PdfShapes {
    let texts: Vec<String> = raw.iter().map(|p| p.text.clone()).collect();
    let repeated = repeated_lines(&texts);
    let r = rules();
    let signed = r.form.signing_services.iter().any(|s| producer.contains(s.as_str()));
    let pages = raw
        .into_iter()
        .enumerate()
        .map(|(index, page)| {
            let own: Vec<&str> = page
                .text
                .lines()
                .filter(|line| visible_chars(line) > 0 && !repeated.contains(&normalized_line(line)))
                .collect();
            let own_chars: usize = own.iter().map(|l| visible_chars(l)).sum();
            let short = own.iter().filter(|l| visible_chars(l) <= r.form.short_line_chars).count();
            let words: Vec<usize> = own
                .iter()
                .flat_map(|l| l.split(|c: char| !c.is_alphabetic()))
                .filter(|w| !w.is_empty())
                .map(|w| w.chars().count())
                .collect();
            let single = if words.is_empty() { 0.0 } else { words.iter().filter(|w| **w == 1).count() as f64 / words.len() as f64 };
            let mut profile = PageProfile {
                page: index as u32 + 1,
                chars: visible_chars(&page.text),
                own_chars,
                own_lines: own.len(),
                short_lines: if own.is_empty() { 0.0 } else { short as f64 / own.len() as f64 },
                single_letters: single,
                largest_image: page.largest_image,
                images_cover: page.images_cover,
                group_values: page.group_values,
                fields: page.fields,
                drawn: page.drawn,
                shape: PageShape::DigitalProse,
                text_layer: TextLayer::Digital,
                read: false,
            };
            (profile.shape, profile.text_layer, profile.read) = classify(&profile, signed);
            profile
        })
        .collect();
    PdfShapes { producer, pages }
}

/// The shape, the text layer and whether the page is read, from its
/// signals (the thresholds of the catalog).
fn classify(p: &PageProfile, signed: bool) -> (PageShape, TextLayer, bool) {
    let r = rules();
    let no_own_layer = if p.chars == 0 { TextLayer::None } else { TextLayer::FooterOnly };
    if p.images_cover >= r.scan.image_cover {
        if p.own_chars < r.scan.layer_min_chars {
            return (PageShape::Scan, no_own_layer, true);
        }
        return (PageShape::Scan, TextLayer::OldOcr, p.single_letters >= r.scan.noisy_single_letters);
    }
    let picture = p.images_cover >= r.picture.image_cover && p.own_chars < r.picture.max_own_chars;
    let drawn = p.own_chars < r.blank.max_own_chars && p.drawn >= r.drawn.min_objects;
    if picture || drawn {
        return (PageShape::Scan, no_own_layer, true);
    }
    if p.own_chars < r.blank.max_own_chars {
        return (PageShape::BlankOrFooter, no_own_layer, false);
    }
    let form = (p.own_lines >= r.form.min_lines && p.short_lines >= r.form.short_line_share) || p.fields >= r.form.min_fields;
    let shape = if form && (p.group_values >= r.form.flattened_min_values || signed) {
        PageShape::FlattenedForm
    } else if form {
        PageShape::DigitalForm
    } else {
        PageShape::DigitalProse
    };
    (shape, TextLayer::Digital, false)
}

/// The fichas of every page of a PDF, read with pdfium.
pub fn profile_pdf(pdf: &Path) -> Result<PdfShapes, String> {
    profile(pdf, false).map(|(shapes, _)| shapes)
}

/// Same as `profile_pdf`, with the prints of each page (`page_print`),
/// opening the PDF once. A page that cannot be drawn has no prints.
pub fn profile_pdf_with_prints(pdf: &Path) -> Result<(PdfShapes, Vec<PagePrint>), String> {
    profile(pdf, true)
}

fn profile(pdf: &Path, with_prints: bool) -> Result<(PdfShapes, Vec<PagePrint>), String> {
    use pdfium_render::prelude::*;

    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(pdf, None)
        .map_err(|e| format!("Failed to open PDF '{}': {e}", pdf.display()))?;
    let tag = |t: PdfDocumentMetadataTagType| doc.metadata().get(t).map(|m| m.value().to_string()).unwrap_or_default();
    let producer = format!("{} {}", tag(PdfDocumentMetadataTagType::Producer), tag(PdfDocumentMetadataTagType::Creator))
        .trim()
        .to_lowercase();
    let mut raw = Vec::new();
    // In grays, without the data of a form drawn, at the size the measurement the
    // perceptual hash comes from drew it (pypdfium2: each side times the
    // scale, rounded up).
    let drawing = |width: f64, height: f64| {
        let scale = PRINT_HEIGHT_PX as f64 / height.max(1.0);
        PdfRenderConfig::new()
            .set_target_size((width * scale).ceil() as i32, (height * scale).ceil() as i32)
            .use_grayscale_rendering(true)
            .render_form_data(false)
    };
    let mut prints = Vec::new();
    for (index, page) in doc.pages().iter().enumerate() {
        let (width, height) = (page.width().value as f64, page.height().value as f64);
        let area = |object: &PdfPageObject<'_>| -> f64 {
            if width <= 0.0 || height <= 0.0 {
                return 0.0;
            }
            object
                .bounds()
                .map(|b| {
                    let b = b.to_rect();
                    let w = (b.right().value as f64).min(width) - (b.left().value as f64).max(0.0);
                    let h = (b.top().value as f64).min(height) - (b.bottom().value as f64).max(0.0);
                    (w.max(0.0) * h.max(0.0)) / (width * height)
                })
                .unwrap_or(0.0)
        };
        let mut out = RawPage {
            text: page.text().map(|t| t.all()).unwrap_or_default(),
            ..RawPage::default()
        };
        for object in page.objects().iter() {
            match object.object_type() {
                PdfPageObjectType::Image => {
                    let a = area(&object);
                    out.largest_image = out.largest_image.max(a);
                    out.images_cover += a;
                }
                PdfPageObjectType::Path => out.drawn += 1,
                PdfPageObjectType::XObjectForm => {
                    if let Some(group) = object.as_x_object_form_object() {
                        let kinds: Vec<PdfPageObjectType> = group.iter().map(|o| o.object_type()).collect();
                        out.group_values += kinds.iter().filter(|k| **k == PdfPageObjectType::Text).count();
                        // A scan wrapped in a group: the group's place is the image's.
                        if kinds.contains(&PdfPageObjectType::Image) {
                            let a = area(&object);
                            out.largest_image = out.largest_image.max(a);
                            out.images_cover += a;
                        }
                    }
                }
                _ => {}
            }
        }
        out.images_cover = out.images_cover.min(1.0);
        out.fields = page.annotations().iter().filter(|a| a.annotation_type() == PdfPageAnnotationType::Widget).count();
        if with_prints {
            match page.render_with_config(&drawing(width, height)).and_then(|bitmap| bitmap.as_image()) {
                Ok(image) => prints.push(PagePrint::of(index as u32 + 1, &image.to_luma8(), &out.text)),
                Err(e) => eprintln!("[page_shape] page {} of '{}' could not be drawn: {e}", index + 1, pdf.display()),
            }
        }
        raw.push(out);
    }
    Ok((shapes_of(producer, raw), prints))
}

/// Builds invented PDFs for the tests (R-24): each page with lines of
/// text, and optionally an image covering part of it, drawn after the
/// text (so it hides it, as an old recognition layer under a scan).
#[cfg(test)]
pub(crate) mod test_pdf {
    #[derive(Default, Clone)]
    pub(crate) struct Page {
        pub lines: Vec<String>,
        /// Share of the page (0 to 1) a gray image covers, from the top.
        pub image: f64,
    }

    pub(crate) fn page(lines: &[&str], image: f64) -> Page {
        Page { lines: lines.iter().map(|l| l.to_string()).collect(), image }
    }

    pub(crate) fn build(pages: &[Page]) -> Vec<u8> {
        let mut objects: Vec<Vec<u8>> = vec![
            b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
            Vec::new(),
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
            b"<< /Type /XObject /Subtype /Image /Width 2 /Height 2 /ColorSpace /DeviceGray /BitsPerComponent 8 /Length 4 >>\nstream\n\x80\x90\xa0\xb0\nendstream".to_vec(),
        ];
        let mut kids = Vec::new();
        for page in pages {
            let mut content = page
                .lines
                .iter()
                .enumerate()
                .map(|(index, line)| format!("BT /F1 10 Tf 40 {} Td ({line}) Tj ET", 760 - 14 * index as i64))
                .collect::<Vec<_>>()
                .join("\n");
            if page.image > 0.0 {
                let h = 792.0 * page.image;
                content.push_str(&format!("\nq 612 0 0 {h:.1} 0 {:.1} cm /Im1 Do Q", 792.0 - h));
            }
            let stream = format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len());
            objects.push(stream.into_bytes());
            let contents = objects.len();
            objects.push(
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents {contents} 0 R \
                     /Resources << /Font << /F1 3 0 R >> /XObject << /Im1 4 0 R >> >> >>"
                )
                .into_bytes(),
            );
            kids.push(format!("{} 0 R", objects.len()));
        }
        objects[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), kids.len()).into_bytes();
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
            pdf.extend_from_slice(object);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        let xref = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for offset in offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes(),
        );
        pdf
    }
}

#[cfg(test)]
mod tests {
    use super::test_pdf::{build, page};
    use super::*;
    use crate::docbase::testing::TempDir;

    const PROSE: [&str; 4] = [
        "Primera pagina con texto propio del documento de prueba, escrita para que",
        "la extraccion la lea entera y no la tome por un escaneo: tiene bastante",
        "mas de ochenta letras y treinta palabras distintas entre si, repartidas",
        "en cuatro renglones largos como los de un escrito comun de un estudio.",
    ];
    /// A footer an invented system stamps on every page, with a date that changes.
    fn footer(n: usize) -> String {
        format!("Sistema Inventado de Expedientes - firmado digitalmente el {n:02}/03/2026 - hoja {n} de 4 - codigo verificador")
    }

    fn shapes(pages: &[super::test_pdf::Page]) -> PdfShapes {
        let dir = TempDir::new();
        let path = dir.path().join("prueba.pdf");
        std::fs::write(&path, build(pages)).unwrap();
        profile_pdf(&path).unwrap()
    }

    fn of(p: &PageProfile) -> (PageShape, TextLayer, bool) {
        (p.shape, p.text_layer, p.read)
    }

    #[test]
    fn a_footer_repeated_on_every_page_does_not_make_a_scan_a_page_with_text() {
        let f: Vec<String> = (1..=4).map(footer).collect();
        let s = shapes(&[
            page(&[PROSE[0], PROSE[1], PROSE[2], PROSE[3], &f[0]], 0.0),
            page(&[&f[1]], 1.0),
            page(&[&f[2]], 0.0),
            page(&[PROSE[0], PROSE[1], PROSE[2], PROSE[3], &f[3]], 0.0),
        ]);
        // The footer has more than the 80 characters of the earlier test.
        assert!(s.pages[1].chars >= 80 && s.pages[1].own_chars == 0, "{:?}", s.pages[1]);
        assert_eq!(of(&s.pages[0]), (PageShape::DigitalProse, TextLayer::Digital, false));
        assert_eq!(of(&s.pages[1]), (PageShape::Scan, TextLayer::FooterOnly, true));
        assert_eq!(of(&s.pages[2]), (PageShape::BlankOrFooter, TextLayer::FooterOnly, false));
        assert_eq!(s.to_read(), vec![2]);
        assert_eq!(s.read_by_the_earlier_test(), BTreeSet::new());
    }

    #[test]
    fn visible_text_hidden_by_an_image_with_single_letters_is_an_old_layer_read_again() {
        let noisy = [
            "C O N T R A T O de l o c a c i o n e n t r e l a s p a r t e s q u e s u s c r i b e n",
            "e l p r e s e n t e e n l a C i u d a d I n v e n t a d a a l o s d i a s d e l m e s",
        ];
        let s = shapes(&[page(&noisy, 1.0), page(&PROSE, 1.0)]);
        assert!(s.pages[0].single_letters >= 0.3, "{:?}", s.pages[0]);
        assert_eq!(of(&s.pages[0]), (PageShape::Scan, TextLayer::OldOcr, true));
        // A good layer under the scan stays: it is not read again.
        assert_eq!(of(&s.pages[1]), (PageShape::Scan, TextLayer::OldOcr, false));
        assert_eq!(s.to_read(), vec![1]);
        assert_eq!(s.old_layers(), BTreeSet::from([1, 2]));
    }

    #[test]
    fn a_digital_page_a_form_and_a_blank_page_are_not_read() {
        let form = ["Nombre: Persona Inventada", "Documento: 99.999.999", "Domicilio: Calle Falsa 123", "Fecha: 1 de marzo", "Importe: 100,00"];
        let s = shapes(&[page(&PROSE, 0.0), page(&form, 0.0), page(&[], 0.0), page(&["Pagina sin mas"], 0.4)]);
        assert_eq!(of(&s.pages[0]), (PageShape::DigitalProse, TextLayer::Digital, false));
        assert_eq!(of(&s.pages[1]), (PageShape::DigitalForm, TextLayer::Digital, false));
        assert_eq!(of(&s.pages[2]), (PageShape::BlankOrFooter, TextLayer::None, false));
        // Little text and a large picture: its content is in the pixels.
        assert_eq!(of(&s.pages[3]), (PageShape::Scan, TextLayer::FooterOnly, true));
        assert_eq!(s.read_by_the_earlier_test(), BTreeSet::from([3, 4]));
    }

    /// Measures the fichas against the answer sheet of parts (R-25: it
    /// measures, it does not teach): the shape, the text layer and the
    /// blank pages of each page of the PDFs it names.
    ///
    /// ```text
    /// MICELYA_PARTS_ANSWERS=C:\hoja\partes.json MICELYA_PARTS_PDFS=C:\carpeta\de\los\pdf \
    ///     cargo test --lib source_volume::page_shape -- --ignored --nocapture
    /// ```
    fn name<T: Serialize>(v: &T) -> String {
        serde_json::to_value(v).unwrap().as_str().unwrap().to_string()
    }

    #[test]
    #[ignore = "reads an answer sheet and real PDFs outside the repository"]
    fn fichas_against_the_parts_sheet() {
        let (Ok(sheet), Ok(folder)) = (std::env::var("MICELYA_PARTS_ANSWERS"), std::env::var("MICELYA_PARTS_PDFS")) else {
            println!("MICELYA_PARTS_ANSWERS or MICELYA_PARTS_PDFS is not set: nothing to measure");
            return;
        };
        let sheet: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(sheet).unwrap()).unwrap();
        // The sheet's forms that are scans to the ficha: it does not tell
        // what is printed from what is handwritten (D-518).
        let shape_of_sheet = |form: &str| match form {
            "digital_prose" => "digital_prose",
            "digital_form" => "digital_form",
            "flattened_form" => "flattened_form",
            "blank_or_footer" => "blank_or_footer",
            _ => "scan",
        };
        let (mut pages, mut shape_ok, mut layer_ok, mut blank_ok, mut blanks) = (0, 0, 0, 0, 0);
        let (mut read, mut earlier) = (0, 0);
        let mut misses: HashMap<String, usize> = HashMap::new();
        for file in sheet["files"].as_array().unwrap() {
            let path = Path::new(&folder).join(file["name"].as_str().unwrap());
            let shapes = profile_pdf(&path).unwrap();
            read += shapes.to_read().len();
            earlier += shapes.read_by_the_earlier_test().len();
            for mark in file["page_marks"].as_array().unwrap() {
                let n = mark["page"].as_u64().unwrap() as usize;
                let got = &shapes.pages[n - 1];
                let want_shape = shape_of_sheet(mark["form"].as_str().unwrap());
                let want_layer = mark["text_layer"].as_str().unwrap();
                let (got_shape, got_layer) = (name(&got.shape), name(&got.text_layer));
                pages += 1;
                shape_ok += (got_shape == want_shape) as usize;
                layer_ok += (got_layer == want_layer) as usize;
                let want_blank = want_shape == "blank_or_footer";
                blanks += want_blank as usize;
                blank_ok += (want_blank == (got.shape == PageShape::BlankOrFooter)) as usize;
                if got_shape != want_shape {
                    *misses.entry(format!("forma: {want_shape} leída como {got_shape}")).or_default() += 1;
                }
                if got_layer != want_layer {
                    *misses.entry(format!("texto: {want_layer} leído como {got_layer}")).or_default() += 1;
                }
            }
        }
        println!("páginas: {pages}");
        println!("forma: {shape_ok} de {pages}");
        println!("texto digital del archivo: {layer_ok} de {pages}");
        println!("en blanco o solo el pie (sí o no): {blank_ok} de {pages}, {blanks} en la hoja");
        println!("páginas que se reconocen: {read} (con la prueba de antes: {earlier})");
        let mut misses: Vec<_> = misses.into_iter().collect();
        misses.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        for (miss, count) in misses {
            println!("  {count} · {miss}");
        }
    }
}
