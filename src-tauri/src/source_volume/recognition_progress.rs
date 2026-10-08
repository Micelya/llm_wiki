//! Progress of a text recognition run, as an app event.
//!
//! Recognizing a long scan takes minutes. `recognition.rs` reports each
//! page it is about to read through a plain callback and knows nothing
//! about the UI; this module turns those calls into the event the ingest
//! activity listens to.

use serde::Serialize;
use tauri::{AppHandle, Emitter};

pub const PROGRESS_EVENT: &str = "text-recognition://progress";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionProgress {
    /// The source as the app asked for it, so listeners can match it.
    pub path: String,
    /// 1-based position among the pages that need recognition.
    pub page: usize,
    pub total: usize,
}

/// Best effort: a listener that went away must not fail the recognition.
pub fn emit(app: &AppHandle, path: &str, page: usize, total: usize) {
    let _ = app.emit(
        PROGRESS_EVENT,
        RecognitionProgress {
            path: path.to_string(),
            page,
            total,
        },
    );
}
