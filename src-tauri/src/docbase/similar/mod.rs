//! Similar documents: documents that share text without being the same
//! text, what changes between them, and the relation the app proposes
//! for a person to decide.
//!
//! Passes 0 to 3 of the proposal (`docs/design/parecidos-propuesta.md`,
//! section 8.4), all computation, no model:
//!
//! - **0 · What is common** (`store`): in how many documents each
//!   sequence of five words appears; the most common is routine text and
//!   does not count.
//! - **1 · Safe pairs** (`minhash`, `align`, `changes`, `propose`):
//!   candidates by signature, exact measures without the routine text,
//!   the two texts side by side, what changes and what kind of thing it
//!   is, and the proposed relation with a sentence that describes it.
//! - **2 · Marks** (`marks`, `types`): what is verified without a model,
//!   as the catalog of document types says: a preview's watermark, a
//!   signing service's history page, the fields of a form, the digital
//!   signatures embedded in a PDF.
//! - **3 · Provisional families** (`store::families`): against a
//!   principal document, never by chain (R-14).
//!
//! Nothing here joins two documents: it proposes, and a person decides
//! (R-06). The rules it follows are `docs/design/reglas-de-procesamiento.md`.

pub mod align;
pub mod changes;
#[cfg(test)]
mod check;
pub mod marks;
pub mod minhash;
pub mod propose;
pub mod store;
#[cfg(test)]
mod tests;
pub mod text;
pub mod types;

pub use store::{relations, Decision, SimilarDocument, SimilarGroup, SimilarPending};
