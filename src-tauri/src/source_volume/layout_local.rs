//! The model of zones of this computer (D-725): PP-DocLayoutV2 over the
//! same ONNX Runtime as the reader (`recognition_local`), as the package
//! `rapid_layout` runs it in the scripts of chain N. It gives the zones of
//! a page drawn as a picture (text, title, header, footer, number, table,
//! seal, image...), each with its box and how sure it is; the reading by
//! zones (`docbase/structure`) puts every line in its zone.
//!
//! The model is not in the program nor in the repository: it goes in the
//! `models/` folder of the reader (`scripts/fetch-ocr-models.mjs`). Without
//! it the reading by zones is not available, and says so.
//!
//! What is done to the picture and to the answer of the model is a copy of
//! `rapid_layout` (`model_handler/pp_doc_layout`): the picture is scaled to
//! 800 × 800 as OpenCV does it (bicubic), in the order of the colors the
//! scripts gave it (blue, green, red); the boxes over 0.3 go through the
//! suppression of the overlapping ones, the filter of the images as large
//! as the page, the order of the model and the filter of the boxes inside
//! others.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use ort::session::Session;
use ort::value::Tensor;

use super::recognition_local::{ocr_dir, open_runtime, threads};

/// The file of the model, in the `models/` folder of the reader.
pub const MODEL_FILE: &str = "pp_doc_layoutv2.onnx";
/// Its name in the reading.
pub const MODEL_NAME: &str = "pp_doc_layoutv2";
/// The classes of the model, in its order (its metadata, `character`).
pub const CLASSES: [&str; 25] = [
    "abstract",
    "algorithm",
    "aside_text",
    "chart",
    "content",
    "display_formula",
    "doc_title",
    "figure_title",
    "footer",
    "footer_image",
    "footnote",
    "formula_number",
    "header",
    "header_image",
    "image",
    "inline_formula",
    "number",
    "paragraph_title",
    "reference",
    "reference_content",
    "seal",
    "table",
    "text",
    "vertical_text",
    "vision_footnote",
];
/// The side of the square the model sees.
const SIDE: u32 = 800;
/// The least score of a box the scripts asked for (`conf_thresh`).
const MIN_SCORE: f32 = 0.3;

/// One zone of a page: its box in the pixels of the picture (left, top,
/// right, bottom), its class and how sure the model is (three decimals).
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub bbox: [i64; 4],
    pub class: String,
    pub score: f64,
}

/// Where the model is: beside the models of the reader.
pub fn model_in(dir: &Path) -> PathBuf {
    dir.join("models").join(MODEL_FILE)
}

/// Why the model of zones cannot run now, or `None`. The reader must be
/// there too: the reading by zones reads the lines with it.
pub fn unavailable() -> Option<String> {
    if let Some(Err(error)) = MODEL_LOADED.get() {
        return Some(error.clone());
    }
    let dir = match ocr_dir() {
        Ok(dir) => dir,
        Err(error) => return Some(error),
    };
    (!model_in(&dir).is_file()).then(|| missing(&dir))
}

fn missing(dir: &Path) -> String {
    format!(
        "The model of zones of this computer is not installed: {MODEL_FILE} is missing in '{}' (scripts/fetch-ocr-models.mjs)",
        dir.join("models").display()
    )
}

/// The model, started from the folder `dir` of the reader.
pub fn start_in(dir: &Path) -> Result<Session, String> {
    let model = model_in(dir);
    if !model.is_file() {
        return Err(missing(dir));
    }
    open_runtime(dir)?;
    let failed = |error: String| format!("The model of zones of this computer could not start: {error}");
    let mut builder = Session::builder()
        .map_err(|e| failed(e.to_string()))?
        .with_intra_threads(threads())
        .map_err(|e| failed(e.to_string()))?;
    builder.commit_from_file(&model).map_err(|e| failed(e.to_string()))
}

static MODEL_LOADED: OnceLock<Result<Mutex<Session>, String>> = OnceLock::new();

/// The model of the process, started the first time and kept.
pub fn model() -> Result<MutexGuard<'static, Session>, String> {
    let loaded = MODEL_LOADED.get_or_init(|| ocr_dir().and_then(|dir| start_in(&dir)).map(Mutex::new));
    match loaded {
        Ok(model) => Ok(model.lock().unwrap_or_else(|poisoned| poisoned.into_inner())),
        Err(error) => Err(error.clone()),
    }
}

/// The zones of a page drawn as `picture`.
pub fn zones(session: &mut Session, picture: &image::RgbImage) -> Result<Vec<Zone>, String> {
    let (w, h) = picture.dimensions();
    if w == 0 || h == 0 {
        return Ok(Vec::new());
    }
    let input = model_input(picture);
    let failed = |error: ort::Error| format!("The model of zones of this computer failed: {error}");
    let im_shape = Tensor::from_array(([1usize, 2], vec![SIDE as f32, SIDE as f32])).map_err(failed)?;
    let image = Tensor::from_array(([1usize, 3, SIDE as usize, SIDE as usize], input)).map_err(failed)?;
    let scale = Tensor::from_array(([1usize, 2], vec![SIDE as f32 / h as f32, SIDE as f32 / w as f32])).map_err(failed)?;
    let outputs = session.run(ort::inputs!["im_shape" => im_shape, "image" => image, "scale_factor" => scale]).map_err(failed)?;
    let (shape, rows) = outputs[0].try_extract_tensor::<f32>().map_err(failed)?;
    let (_, counts) = outputs[1].try_extract_tensor::<i32>().map_err(failed)?;
    let width = shape.get(1).copied().unwrap_or(0) as usize;
    if width < 6 {
        return Err(format!("The model of zones of this computer answered boxes of {width} values"));
    }
    let count = counts.first().copied().unwrap_or(0).max(0) as usize;
    let raw: Vec<Vec<f32>> = rows.chunks(width).take(count).map(|r| r.to_vec()).collect();
    Ok(postprocess(raw, (w, h)))
}

/// The picture as the model takes it: 800 × 800, scaled as OpenCV's
/// bicubic, blue, green and red planes, from 0 to 1.
fn model_input(picture: &image::RgbImage) -> Vec<f32> {
    let small = resize_bicubic(picture, SIDE, SIDE);
    let plane = (SIDE * SIDE) as usize;
    let mut out = vec![0f32; 3 * plane];
    for (i, p) in small.pixels().enumerate() {
        // The scripts gave the model the picture of pdfium as it comes:
        // blue, green, red.
        out[i] = p[2] as f32 / 255.0;
        out[plane + i] = p[1] as f32 / 255.0;
        out[2 * plane + i] = p[0] as f32 / 255.0;
    }
    out
}

/// OpenCV's `resize` with `INTER_CUBIC` over 8-bit pictures: four taps
/// with A = -0.75, the edge repeated, weights in fixed point (11 bits) and
/// the sum rounded at the end.
pub(crate) fn resize_bicubic(src: &image::RgbImage, dw: u32, dh: u32) -> image::RgbImage {
    const BITS: i64 = 11;
    let (sw, sh) = src.dimensions();
    let taps = |dst: u32, src_len: u32| -> Vec<([usize; 4], [i64; 4])> {
        let scale = src_len as f64 / dst as f64;
        (0..dst)
            .map(|d| {
                let f = (d as f64 + 0.5) * scale - 0.5;
                let s = f.floor();
                let x = f - s;
                const A: f64 = -0.75;
                let w0 = ((A * (x + 1.0) - 5.0 * A) * (x + 1.0) + 8.0 * A) * (x + 1.0) - 4.0 * A;
                let w1 = ((A + 2.0) * x - (A + 3.0)) * x * x + 1.0;
                let w2 = ((A + 2.0) * (1.0 - x) - (A + 3.0)) * (1.0 - x) * (1.0 - x) + 1.0;
                let w3 = 1.0 - w0 - w1 - w2;
                let at = |k: i64| (s as i64 + k).clamp(0, src_len as i64 - 1) as usize;
                let fixed = |w: f64| (w * (1 << BITS) as f64).round() as i64;
                ([at(-1), at(0), at(1), at(2)], [fixed(w0), fixed(w1), fixed(w2), fixed(w3)])
            })
            .collect()
    };
    let xs = taps(dw, sw);
    let ys = taps(dh, sh);
    let raw = src.as_raw();
    // The rows across, each kept once.
    let mut across: Vec<Option<Vec<i64>>> = vec![None; sh as usize];
    let row = |y: usize, across: &mut Vec<Option<Vec<i64>>>| {
        if across[y].is_none() {
            let base = y * sw as usize * 3;
            let mut out = vec![0i64; dw as usize * 3];
            for (dx, (ix, wx)) in xs.iter().enumerate() {
                for c in 0..3 {
                    out[dx * 3 + c] = (0..4).map(|k| raw[base + ix[k] * 3 + c] as i64 * wx[k]).sum();
                }
            }
            across[y] = Some(out);
        }
    };
    let mut out = image::RgbImage::new(dw, dh);
    let half = 1i64 << (2 * BITS - 1);
    for (dy, (iy, wy)) in ys.iter().enumerate() {
        for k in 0..4 {
            row(iy[k], &mut across);
        }
        for dx in 0..dw as usize {
            let mut px = [0u8; 3];
            for (c, slot) in px.iter_mut().enumerate() {
                let sum: i64 = (0..4).map(|k| across[iy[k]].as_ref().unwrap()[dx * 3 + c] * wy[k]).sum();
                *slot = ((sum + half) >> (2 * BITS)).clamp(0, 255) as u8;
            }
            out.put_pixel(dx as u32, dy as u32, image::Rgb(px));
        }
    }
    out
}

/// Python's `round`: a half to the even one.
fn round_even(x: f32) -> f32 {
    x.round_ties_even()
}

/// The intersection over the union of two boxes, counting the pixels of
/// both edges (as `rapid_layout`'s `iou`).
fn iou(a: &[f32], b: &[f32]) -> f64 {
    let (a, b): (Vec<f64>, Vec<f64>) = (a.iter().map(|v| *v as f64).collect(), b.iter().map(|v| *v as f64).collect());
    let inter = (a[2].min(b[2]) - a[0].max(b[0]) + 1.0).max(0.0) * (a[3].min(b[3]) - a[1].max(b[1]) + 1.0).max(0.0);
    let area_a = (a[2] - a[0] + 1.0) * (a[3] - a[1] + 1.0);
    let area_b = (b[2] - b[0] + 1.0) * (b[3] - b[1] + 1.0);
    inter / (area_a + area_b - inter)
}

/// What `rapid_layout` does with the boxes of the model (class, score,
/// box and two values of order), for a picture of `size` pixels.
pub(crate) fn postprocess(mut rows: Vec<Vec<f32>>, size: (u32, u32)) -> Vec<Zone> {
    let (w, h) = (size.0 as f32, size.1 as f32);
    for r in rows.iter_mut() {
        for v in &mut r[2..6] {
            *v = round_even(*v);
        }
    }
    rows.retain(|r| r[1] > MIN_SCORE && r[0] > -1.0);
    // The suppression of overlapping boxes, the surest first: 0.6 with the
    // same class, 0.98 with another.
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|a, b| rows[*b][1].total_cmp(&rows[*a][1]));
    let mut kept: Vec<usize> = Vec::new();
    while let Some((&current, rest)) = order.split_first() {
        kept.push(current);
        order = rest
            .iter()
            .copied()
            .filter(|&i| {
                let limit = if rows[i][0] == rows[current][0] { 0.6 } else { 0.98 };
                iou(&rows[current][2..6], &rows[i][2..6]) < limit
            })
            .collect();
    }
    let mut rows: Vec<Vec<f32>> = kept.into_iter().map(|i| rows[i].clone()).collect();
    // An image as large as the page is not a zone.
    if rows.len() > 1 {
        let limit = if w > h { 0.82 } else { 0.93 };
        let image = CLASSES.iter().position(|c| *c == "image").unwrap() as f32;
        let small: Vec<Vec<f32>> = rows
            .iter()
            .filter(|r| {
                if r[0] != image {
                    return true;
                }
                let (x0, y0, x1, y1) = (r[2].max(0.0), r[3].max(0.0), r[4].min(w), r[5].min(h));
                ((x1 - x0) * (y1 - y0)) as f64 <= limit * (w * h) as f64
            })
            .cloned()
            .collect();
        if !small.is_empty() {
            rows = small;
        }
    }
    // The order the model gives.
    if rows.first().is_some_and(|r| r.len() >= 8) {
        rows.sort_by(|a, b| a[6].total_cmp(&b[6]).then(b[7].total_cmp(&a[7])));
    } else if rows.first().is_some_and(|r| r.len() == 7) {
        rows.sort_by(|a, b| a[6].total_cmp(&b[6]));
    }
    struct Box {
        label: &'static str,
        score: f32,
        c: [i64; 4],
    }
    let boxes: Vec<Box> = rows
        .iter()
        .filter_map(|r| {
            let c = [r[2].max(0.0) as i64, r[3].max(0.0) as i64, r[4].min(w) as i64, r[5].min(h) as i64];
            let label = CLASSES.get(r[0] as usize).copied()?;
            (c[2] > c[0] && c[3] > c[1]).then_some(Box { label, score: r[1], c })
        })
        .filter(|b| b.label != "reference")
        .collect();
    // The boxes inside others: the smaller one goes, but an image and a
    // zone of another class both stay.
    let area = |c: &[i64; 4]| ((c[2] - c[0]) as f64 * (c[3] - c[1]) as f64).abs();
    let overlap = |a: &[i64; 4], b: &[i64; 4]| {
        let inter = ((a[2].min(b[2]) - a[0].max(b[0])).max(0) * (a[3].min(b[3]) - a[1].max(b[1])).max(0)) as f64;
        let small = area(a).min(area(b));
        if small == 0.0 {
            0.0
        } else {
            inter / small
        }
    };
    let mut dropped = vec![false; boxes.len()];
    for i in 0..boxes.len() {
        let c = boxes[i].c;
        if c[2] - c[0] < 6 || c[3] - c[1] < 6 {
            dropped[i] = true;
        }
        for j in i + 1..boxes.len() {
            if dropped[i] || dropped[j] {
                continue;
            }
            let ratio = overlap(&boxes[i].c, &boxes[j].c);
            let (li, lj) = (boxes[i].label, boxes[j].label);
            if li == "inline_formula" || lj == "inline_formula" {
                if ratio > 0.5 {
                    if li == "inline_formula" {
                        dropped[i] = true;
                    }
                    if lj == "inline_formula" {
                        dropped[j] = true;
                    }
                    continue;
                }
            }
            if ratio > 0.7 {
                if (li == "image" || lj == "image") && li != lj {
                    continue;
                }
                if area(&boxes[i].c) >= area(&boxes[j].c) {
                    dropped[j] = true;
                } else {
                    dropped[i] = true;
                }
            }
        }
    }
    boxes
        .into_iter()
        .zip(dropped)
        .filter(|(_, d)| !d)
        .map(|(b, _)| Zone { bbox: b.c, class: b.label.to_string(), score: (b.score as f64 * 1000.0).round_ties_even() / 1000.0 })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(class: &str, score: f32, b: [f32; 4], order: f32) -> Vec<f32> {
        let c = CLASSES.iter().position(|x| *x == class).unwrap() as f32;
        vec![c, score, b[0], b[1], b[2], b[3], order, order]
    }

    #[test]
    fn the_boxes_of_the_model_are_kept_as_rapid_layout_keeps_them() {
        let rows = vec![
            row("text", 0.9, [100.0, 300.0, 900.0, 600.0], 2.0),
            // The same zone again, less sure: suppressed.
            row("text", 0.8, [105.0, 305.0, 900.0, 600.0], 3.0),
            row("doc_title", 0.95, [100.0, 100.0, 900.0, 200.4], 1.0),
            // Below the least score.
            row("seal", 0.2, [600.0, 700.0, 800.0, 900.0], 4.0),
            // A box inside a larger one of another class: the smaller goes.
            row("number", 0.7, [120.0, 320.0, 300.0, 400.0], 5.0),
            // An image as large as the page is not a zone.
            row("image", 0.6, [0.0, 0.0, 1000.0, 1400.0], 6.0),
            // A reference is never a zone.
            row("reference", 0.9, [100.0, 1200.0, 900.0, 1300.0], 7.0),
        ];
        let zones = postprocess(rows, (1000, 1414));
        let got: Vec<(&str, [i64; 4])> = zones.iter().map(|z| (z.class.as_str(), z.bbox)).collect();
        assert_eq!(got, vec![("doc_title", [100, 100, 900, 200]), ("text", [100, 300, 900, 600])]);
        assert_eq!(zones[0].score, 0.95);
    }

    #[test]
    fn the_picture_is_scaled_as_opencv_scales_it() {
        // A flat picture stays flat, and a picture of the same size is the same.
        let flat = image::RgbImage::from_pixel(37, 23, image::Rgb([200, 100, 7]));
        let small = resize_bicubic(&flat, 8, 5);
        assert!(small.pixels().all(|p| p.0 == [200, 100, 7]));
        let mut steps = image::RgbImage::new(4, 4);
        for (x, _, p) in steps.enumerate_pixels_mut() {
            *p = image::Rgb([(x * 60) as u8, 0, 255]);
        }
        assert_eq!(resize_bicubic(&steps, 4, 4), steps);
        // The planes go blue, green, red.
        let input = model_input(&image::RgbImage::from_pixel(10, 10, image::Rgb([255, 0, 51])));
        let plane = (SIDE * SIDE) as usize;
        assert_eq!((input[0], input[plane], input[2 * plane]), (0.2, 0.0, 1.0));
    }

    #[test]
    fn without_its_model_it_says_it_is_not_installed() {
        let empty = std::env::temp_dir().join(format!("micelya-layout-empty-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&empty).unwrap();
        let error = start_in(&empty).err().unwrap();
        assert!(error.contains("not installed") && error.contains(MODEL_FILE), "{error}");
        assert_eq!(crate::docbase::failures::classify(&error).code, "engine-missing");
        let _ = std::fs::remove_dir_all(&empty);
    }
}
