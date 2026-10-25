//! The reading of source files that the document base and the chat share:
//! the text of a file (`provider`), its hash, the recognition of scans
//! and images, the engine that answers questions about documents, and
//! the text helpers the base compares documents with (`catalog`).
//!
//! It used to also mount the base's folders inside a wiki project; that
//! left with the wiki (W6).

pub mod asking;
pub mod catalog;
pub mod file_hash;
// The shape of each page of a PDF, which decides what is recognized (P-320).
pub mod page_shape;
mod provider;
// The engine that recognizes the text of scans and images, which the base
// runs by itself (`docbase::recognize`, P-242); parts of it are kept for
// engines still to come and have no caller yet.
#[allow(dead_code)]
pub mod recognition;
pub mod recognized_store;
#[allow(dead_code)]
mod recognition_codex;
// Text recognition through Micelya Core (Textract), page by page (D-483).
pub(crate) mod recognition_core;

pub use provider::{LocalFolderProvider, SourceEntry, SourceProvider};
/// What the recognition of scans asks about each image.
pub(crate) use recognition_codex::PROMPT as RECOGNITION_PROMPT;
