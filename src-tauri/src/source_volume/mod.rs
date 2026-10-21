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
mod provider;
// The base does not recognize scans and images by itself yet: the wiki's
// ingest asked the engine and handed the text back to it. The engine stays
// for the base to use (P-242); until then most of it has no caller.
#[allow(dead_code)]
pub mod recognition;
#[allow(dead_code)]
pub mod recognition_progress;
#[allow(dead_code)]
pub mod recognized_store;
#[allow(dead_code)]
mod recognition_codex;

pub use provider::{LocalFolderProvider, SourceEntry, SourceProvider};
