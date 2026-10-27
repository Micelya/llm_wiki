//! The question to the model in a doubtful step: the two pages as images,
//! and whether the second starts another document (D-527, D-556).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::catalog::catalog;
use crate::docbase::similar::classify::question_key;

pub const CUT_TASK: &str = "cut";
pub const OTHER: &str = "other";

/// What the model says of a step.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CutAnswer {
    pub second_starts_new_document: bool,
    pub first_page_type: String,
    pub second_page_type: String,
    pub confidence: String,
}

impl CutAnswer {
    pub fn parse(answer: &str) -> Result<Self, String> {
        serde_json::from_str(answer).map_err(|e| format!("The answer about a cut is not of the shape asked for: {e}"))
    }

    /// A type the model gave, when it is one of the list.
    pub fn known(t: &str) -> Option<String> {
        (t != OTHER && catalog().ask.types.contains_key(t)).then(|| t.to_string())
    }
}

/// The text of the question: the same for every pair of pages.
pub fn prompt() -> String {
    let ask = &catalog().ask;
    let types: Vec<String> = ask.types.iter().map(|(id, gloss)| format!("- {id}: {gloss}")).collect();
    ask.prompt.replace("{types}", &types.join("\n"))
}

/// The key of the question about two pages: the version of the
/// instruction, the exact prints of the two pages and the text. The same
/// pages in another file are the same question.
pub fn key(first_exact: &str, second_exact: &str) -> String {
    let ask = &catalog().ask;
    question_key(&ask.version, &format!("{first_exact}\n{second_exact}\n{}", prompt()))
}

pub fn schema() -> serde_json::Value {
    let mut types: Vec<String> = catalog().ask.types.keys().cloned().collect();
    types.push(OTHER.to_string());
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["second_starts_new_document", "first_page_type", "second_page_type", "confidence"],
        "properties": {
            "second_starts_new_document": {"type": "boolean"},
            "first_page_type": {"type": "string", "enum": types},
            "second_page_type": {"type": "string", "enum": types},
            "confidence": {"type": "string", "enum": ["high", "medium", "low"]},
        },
    })
}

/// Draws pages of a PDF in color, the long side at the catalog's size, as
/// JPEG files in `dir` (absolute paths: the engine runs elsewhere).
pub fn render_pages(pdf: &Path, pages: &[u32], dir: &Path) -> Result<Vec<PathBuf>, String> {
    use pdfium_render::prelude::*;

    let long = catalog().ask.image_long_side as i32;
    let _guard = crate::commands::fs::lock_pdfium();
    let pdfium = crate::commands::fs::pdfium()?;
    let doc = pdfium
        .load_pdf_from_file(pdf, None)
        .map_err(|e| format!("Failed to open PDF '{}': {e}", pdf.display()))?;
    let dir = std::path::absolute(dir).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for page in pages {
        let p = doc.pages().get((*page - 1) as i32).map_err(|e| format!("Page {page} not found: {e}"))?;
        let config = PdfRenderConfig::new().set_target_width(long).set_maximum_width(long).set_maximum_height(long);
        let image = p
            .render_with_config(&config)
            .map_err(|e| format!("Page {page} could not be rendered: {e}"))?
            .as_image()
            .map_err(|e| format!("Page {page} could not be converted to an image: {e}"))?;
        let path = dir.join(format!("p{page:04}.jpg"));
        image
            .to_rgb8()
            .save_with_format(&path, image::ImageFormat::Jpeg)
            .map_err(|e| format!("Page {page} could not be saved: {e}"))?;
        out.push(path);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_changes_with_the_pages_and_not_with_the_file() {
        assert_eq!(key("a", "b"), key("a", "b"));
        assert_ne!(key("a", "b"), key("b", "a"));
        assert!(key("a", "b").starts_with(&catalog().ask.version));
        assert!(!prompt().contains("{types}"));
        let answer = r#"{"second_starts_new_document": true, "first_page_type": "other", "second_page_type": "recibo", "confidence": "high"}"#;
        let parsed = CutAnswer::parse(answer).unwrap();
        assert_eq!(CutAnswer::known(&parsed.second_page_type), Some("recibo".to_string()));
        assert_eq!(CutAnswer::known(&parsed.first_page_type), None);
        assert!(CutAnswer::parse("{}").is_err());
        assert_eq!(schema()["properties"]["second_page_type"]["enum"].as_array().unwrap().len(), catalog().ask.types.len() + 1);
    }
}
