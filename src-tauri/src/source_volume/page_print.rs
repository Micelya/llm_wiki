//! The prints of each page of a PDF (D-534, D-560), from the page drawn
//! `PRINT_HEIGHT_PX` pixels high in grays:
//!
//! - `exact_hash`: SHA-256 of its pixels, the same only for pages that
//!   draw exactly alike;
//! - `phash`: the perceptual hash (64 bits, 16 hex digits), near for pages
//!   that look alike: the same one the measurement in the cloud used
//!   (`docs/investigacion/huellas-y-embeddings/guiones/huellas.py`, the
//!   `phash` of `imagehash`: 32 × 32 with Lanczos, the DCT, its 8 × 8 of
//!   lowest frequencies against their median);
//! - `text_hash`: SHA-256 of its normalized text (lower case, without
//!   accents, only letters and digits separated by one space), only from
//!   `MIN_TEXT_CHARS` characters on.

use image::GrayImage;
use sha2::{Digest, Sha256};
use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

/// The height the page is drawn at for its prints.
pub const PRINT_HEIGHT_PX: i32 = 256;
/// Below this, the normalized text of a page is too little to tell it.
pub const MIN_TEXT_CHARS: usize = 40;
/// Up to this many different bits, two pages look alike.
#[cfg_attr(not(test), allow(dead_code))] // The measurement asks it today.
pub const ALIKE_BITS: u32 = 6;

const SIDE: usize = 32;
const LOW: usize = 8;

/// The prints of one page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PagePrint {
    /// From 1.
    pub page_no: u32,
    pub exact_hash: String,
    pub phash: String,
    pub text_hash: Option<String>,
}

impl PagePrint {
    /// The prints of a page from its drawing in grays and its text.
    pub fn of(page_no: u32, gray: &GrayImage, text: &str) -> Self {
        Self { page_no, exact_hash: exact_hash(gray), phash: format!("{:016x}", phash(gray)), text_hash: text_hash(text) }
    }
}

/// SHA-256 of the pixels.
pub fn exact_hash(gray: &GrayImage) -> String {
    format!("{:x}", Sha256::digest(gray.as_raw()))
}

/// Lower case, without accents, only the runs of letters and digits
/// (a to z, 0 to 9), separated by one space.
pub fn normalized_text(text: &str) -> String {
    let folded: String = text.to_lowercase().nfkd().filter(|c| canonical_combining_class(*c) == 0).collect();
    folded
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|run| !run.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// SHA-256 of the normalized text, when it has `MIN_TEXT_CHARS` or more.
pub fn text_hash(text: &str) -> Option<String> {
    let normalized = normalized_text(text);
    (normalized.chars().count() >= MIN_TEXT_CHARS).then(|| format!("{:x}", Sha256::digest(normalized.as_bytes())))
}

/// The perceptual hash: the first bit is the highest.
pub fn phash(gray: &GrayImage) -> u64 {
    let pixels: Vec<f64> = resize_as_pillow(gray, SIDE, SIDE).iter().map(|p| *p as f64).collect();
    // DCT-II along the columns, then along the rows, only the lowest
    // frequencies. Its scale does not change which side of the median
    // each one falls on.
    let cosines: Vec<f64> = (0..LOW * SIDE)
        .map(|at| {
            let (k, n) = (at / SIDE, at % SIDE);
            (std::f64::consts::PI * k as f64 * (2 * n + 1) as f64 / (2 * SIDE) as f64).cos()
        })
        .collect();
    let mut by_column = [[0f64; SIDE]; LOW];
    for (k, row) in by_column.iter_mut().enumerate() {
        for (x, cell) in row.iter_mut().enumerate() {
            *cell = (0..SIDE).map(|y| pixels[y * SIDE + x] * cosines[k * SIDE + y]).sum();
        }
    }
    let mut low = [0f64; LOW * LOW];
    for k in 0..LOW {
        for l in 0..LOW {
            low[k * LOW + l] = (0..SIDE).map(|x| by_column[k][x] * cosines[l * SIDE + x]).sum();
        }
    }
    // What is only the rounding of the sums is zero, as the DCT of scipy
    // gives it: a blank page has the first bit alone.
    let largest = low.iter().fold(0f64, |max, value| max.max(value.abs()));
    for value in low.iter_mut() {
        if value.abs() <= largest * 1e-9 {
            *value = 0.0;
        }
    }
    let mut sorted = low;
    sorted.sort_by(|a, b| a.total_cmp(b));
    let median = (sorted[LOW * LOW / 2 - 1] + sorted[LOW * LOW / 2]) / 2.0;
    low.iter().fold(0u64, |bits, value| (bits << 1) | u64::from(*value > median))
}

/// Fixed point of the weights, as Pillow keeps them for 8-bit images.
const PRECISION_BITS: u32 = 32 - 8 - 2;

/// Resizes with Lanczos as Pillow does (`Image.resize`, `LANCZOS`), so the
/// hash is the one `imagehash` gives: the weights of each output pixel,
/// in fixed point, the rows first and then the columns, each pass rounded
/// to 8 bits.
fn resize_as_pillow(gray: &GrayImage, width: usize, height: usize) -> Vec<u8> {
    let (in_w, in_h) = (gray.width() as usize, gray.height() as usize);
    let source = gray.as_raw();
    let (bounds_x, weights_x, size_x) = coefficients(in_w, width);
    let mut rows = vec![0u8; width * in_h];
    for y in 0..in_h {
        for x in 0..width {
            let (start, count) = bounds_x[x];
            let k = &weights_x[x * size_x..x * size_x + count];
            let sum = k.iter().enumerate().fold(1i64 << (PRECISION_BITS - 1), |sum, (i, w)| sum + source[y * in_w + start + i] as i64 * w);
            rows[y * width + x] = clip8(sum);
        }
    }
    let (bounds_y, weights_y, size_y) = coefficients(in_h, height);
    let mut out = vec![0u8; width * height];
    for y in 0..height {
        let (start, count) = bounds_y[y];
        let k = &weights_y[y * size_y..y * size_y + count];
        for x in 0..width {
            let sum = k.iter().enumerate().fold(1i64 << (PRECISION_BITS - 1), |sum, (i, w)| sum + rows[(start + i) * width + x] as i64 * w);
            out[y * width + x] = clip8(sum);
        }
    }
    out
}

fn clip8(sum: i64) -> u8 {
    if sum >= (1i64 << PRECISION_BITS << 8) {
        255
    } else if sum <= 0 {
        0
    } else {
        (sum >> PRECISION_BITS) as u8
    }
}

/// For each output pixel, the first input pixel and how many it takes,
/// and their weights (`size` per pixel), as Pillow's `precompute_coeffs`.
fn coefficients(in_size: usize, out_size: usize) -> (Vec<(usize, usize)>, Vec<i64>, usize) {
    fn sinc(x: f64) -> f64 {
        if x == 0.0 {
            1.0
        } else {
            let x = x * std::f64::consts::PI;
            x.sin() / x
        }
    }
    let lanczos = |x: f64| if (-3.0..3.0).contains(&x) { sinc(x) * sinc(x / 3.0) } else { 0.0 };
    let scale = in_size as f64 / out_size as f64;
    let filter_scale = scale.max(1.0);
    let support = 3.0 * filter_scale;
    let ss = 1.0 / filter_scale;
    let size = support.ceil() as usize * 2 + 1;
    let mut bounds = Vec::with_capacity(out_size);
    let mut weights = vec![0i64; out_size * size];
    for xx in 0..out_size {
        let center = (xx as f64 + 0.5) * scale;
        let min = ((center - support + 0.5) as i64).max(0) as usize;
        let max = ((center + support + 0.5) as i64).min(in_size as i64) as usize - min;
        let k: Vec<f64> = (0..max).map(|x| lanczos((x as f64 + min as f64 - center + 0.5) * ss)).collect();
        let total: f64 = k.iter().sum();
        for (x, w) in k.iter().enumerate() {
            let w = if total != 0.0 { w / total } else { *w };
            let fixed = w * (1i64 << PRECISION_BITS) as f64;
            weights[xx * size + x] = if w < 0.0 { (fixed - 0.5) as i64 } else { (fixed + 0.5) as i64 };
        }
        bounds.push((min, max));
    }
    (bounds, weights, size)
}

/// How many bits two perceptual hashes differ in; none when one is not a
/// hash.
#[cfg_attr(not(test), allow(dead_code))] // The measurement asks it today.
pub fn distance(a: &str, b: &str) -> Option<u32> {
    let a = u64::from_str_radix(a, 16).ok()?;
    let b = u64::from_str_radix(b, 16).ok()?;
    Some((a ^ b).count_ones())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An invented page: dark lines of text where `lines` says.
    fn page(lines: &[(u32, u32)]) -> GrayImage {
        let mut image = GrayImage::from_pixel(180, 256, image::Luma([255]));
        for (top, width) in lines {
            for y in *top..top + 6 {
                for x in 20..20 + width {
                    image.put_pixel(x, y, image::Luma([30]));
                }
            }
        }
        image
    }

    #[test]
    fn the_same_page_has_the_same_prints_and_another_one_differs() {
        let letter = page(&[(20, 140), (40, 120), (60, 140), (80, 60)]);
        let again = page(&[(20, 140), (40, 120), (60, 140), (80, 60)]);
        let other = page(&[(120, 40), (200, 140), (220, 140)]);
        let text = "Señora Directora: el grupo de lectura informa el horario de la reunión.";
        let (a, b, c) = (PagePrint::of(1, &letter, text), PagePrint::of(2, &again, text), PagePrint::of(1, &other, "Anexo"));
        assert_eq!((a.exact_hash.as_str(), a.phash.as_str()), (b.exact_hash.as_str(), b.phash.as_str()));
        assert_eq!(a.phash.len(), 16);
        assert_eq!(distance(&a.phash, &b.phash), Some(0));
        assert_ne!(a.exact_hash, c.exact_hash);
        assert!(distance(&a.phash, &c.phash).unwrap() > ALIKE_BITS);
        assert!(a.text_hash.is_some() && c.text_hash.is_none());
    }

    #[test]
    fn a_page_drawn_a_little_different_still_looks_alike() {
        let letter = page(&[(20, 140), (40, 120), (60, 140), (80, 60)]);
        let mut touched = letter.clone();
        touched.put_pixel(100, 150, image::Luma([0]));
        let (a, b) = (PagePrint::of(1, &letter, ""), PagePrint::of(1, &touched, ""));
        assert_ne!(a.exact_hash, b.exact_hash);
        assert!(distance(&a.phash, &b.phash).unwrap() <= ALIKE_BITS);
    }

    #[test]
    fn a_blank_page_has_the_first_bit_alone() {
        let blank = GrayImage::from_pixel(198, 256, image::Luma([255]));
        assert_eq!(PagePrint::of(1, &blank, "").phash, "8000000000000000");
    }

    #[test]
    fn the_text_is_normalized_as_in_the_cloud_measurement() {
        assert_eq!(normalized_text("  Señor JUEZ: art. 3º, inc. «b»—fs. 12/13  "), "senor juez art 3o inc b fs 12 13");
        assert_eq!(text_hash("corto"), None);
        assert_eq!(text_hash("Á B C"), None);
        let long = "El grupo de lectura informa el horario de la reunion semanal";
        assert_eq!(text_hash(long), text_hash(&long.to_uppercase()));
    }
}
