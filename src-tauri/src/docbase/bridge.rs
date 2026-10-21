//! The base shared with the parts of the app that have no handle to its
//! state: the chat's agent (D-413).
//!
//! It used to also connect the base with the wiki of the general project
//! (its sources mounted in the wiki, the text the base already had); that
//! left with the wiki (W6).

use std::sync::{Arc, OnceLock};

use super::DocBase;

/// The base of the private space, for the parts of the app that have no
/// handle to the app state.
static SHARED: OnceLock<Arc<DocBase>> = OnceLock::new();

pub fn share(base: Arc<DocBase>) {
    let _ = SHARED.set(base);
}

/// The base of the private space, for the chat's agent (D-413).
pub fn shared_base() -> Option<Arc<DocBase>> {
    SHARED.get().cloned()
}
