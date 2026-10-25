//! Prints what the knowledge layer brings to the attention view, read from
//! a copy of a real base as it is (nothing is computed again, nothing is
//! asked): the readings that failed or lost many quotes, the identities in
//! doubt, the data in which the documents differ, the joins by identifier,
//! the matters the model proposed and the pages still without synthesis.
//!
//! Not part of the normal test run. Only `MICELYA_DOCBASE_COPY` (a copy of
//! the app's base, outside the repository: it holds real documents).
//!
//! ```text
//! MICELYA_DOCBASE_COPY=… cargo test --lib docbase::knowledge::check_attention -- --ignored --nocapture --test-threads=1
//! ```

use super::check::open_base;
use crate::docbase::Visible;

#[test]
#[ignore = "reads a copy of a real base"]
fn what_the_layer_brings_to_attention() {
    let Some((_scratch, base)) = open_base() else {
        println!("MICELYA_DOCBASE_COPY is not set: nothing to print");
        return;
    };
    let attention = base.knowledge_attention(&Visible::All).unwrap();
    let doubts = base.knowledge_doubts(&Visible::All).unwrap();
    println!("== status ==\n{}", serde_json::to_string_pretty(&attention.status).unwrap());
    println!(
        "== counts ==\nfailed readings: {}\nnoisy readings: {}\nconflicts: {}\njoins by identifier: {}\nproposed matters: {}\nmatters to confirm: {}\nmatters to join: {}\nsynthesis pending: {}\ndiscarded sentences: {}\nkept pages: {}\nleft with documents: {}\ndoubts: {}",
        attention.failed.len(),
        attention.noisy.len(),
        attention.conflicts.len(),
        attention.identifier_joins.len(),
        attention.proposed_matters.len(),
        attention.matters_to_confirm.len(),
        attention.matters_to_join.len(),
        attention.synthesis_pending,
        attention.discarded_sentences,
        attention.kept_pages.len(),
        attention.left_with_documents.len(),
        doubts.len(),
    );
    println!("== attention ==\n{}", serde_json::to_string_pretty(&attention).unwrap());
    println!("== doubts ==\n{}", serde_json::to_string_pretty(&doubts).unwrap());
}
