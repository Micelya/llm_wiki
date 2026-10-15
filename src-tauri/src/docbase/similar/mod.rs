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
//! Passes 4 to 6, with the model only where it adds something:
//!
//! - **4 · Type and stage** (`classify`): one question per family, about
//!   its principal, and one per document alone; none when the marks say
//!   it.
//! - **5 · Data by type** (`data`, `classify`): the rules of the type,
//!   and the model only for what they did not find.
//! - **6 · Relations by type** (`by_type`): the rules of the type confirm
//!   or correct what pass 1 measured.
//!
//! Nothing here joins two documents: it proposes, and a person decides
//! (R-06). The rules it follows are `docs/design/reglas-de-procesamiento.md`.

pub mod align;
pub mod by_type;
pub mod changes;
#[cfg(test)]
mod check;
#[cfg(test)]
mod check_types;
pub mod classify;
pub mod data;
pub mod marks;
pub mod minhash;
pub mod propose;
pub mod store;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_types;
pub mod text;
pub mod types;

pub use store::{relations, Decision, SimilarDocument, SimilarGroup, SimilarPending};
