//! The `wasm-bindgen` surface of a take-in (`specs/0095-workspace` criteria 42 and 49): the
//! membership check of a document shared on its own, the `Moved` record its own room gets, and
//! the words of the confirm and the refusal. Pass-throughs to `session::workspace_take_in`.

use curvyo_ui_core::workspace_tree_text as texts;
use wasm_bindgen::prelude::*;

use crate::session::{TakeIn, unhex};
use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// What the document file `bytes` is to this session's shared workspace:
    /// `{"kind":"plain"}`, `{"kind":"allowed","people":2}` or `{"kind":"refused"}`.
    #[must_use]
    pub fn workspace_take_in_check(&self, bytes: &[u8]) -> String {
        match self.session.workspace_take_in_check(bytes) {
            TakeIn::Plain => serde_json::json!({ "kind": "plain" }),
            TakeIn::Allowed { people } => {
                serde_json::json!({ "kind": "allowed", "people": people })
            }
            TakeIn::Refused => serde_json::json!({ "kind": "refused" }),
        }
        .to_string()
    }

    /// Appends `Moved{workspace, document}` (hex) to this document's own room; whether it went
    /// into the outbox.
    pub fn push_moved(&mut self, workspace: &str, document: &str) -> bool {
        match (unhex::<16>(workspace), unhex::<16>(document)) {
            (Some(workspace), Some(document)) => self.session.push_moved(workspace, document),
            _ => false,
        }
    }

    /// Whether the relay stored the `Moved` record.
    #[must_use]
    pub fn moved_stored(&self) -> bool {
        self.session.moved_stored()
    }

    /// Whether this device was removed from its shared workspace (criteria 44, 45).
    #[must_use]
    pub fn workspace_removed(&self) -> bool {
        self.session.workspace_removed()
    }

    /// Whether the session is connected and caught up.
    #[must_use]
    pub fn sync_online(&self) -> bool {
        self.session.sync_online()
    }
}

/// The block on a removed device (criterion 44).
#[wasm_bindgen]
#[must_use]
pub fn workspace_removed_text() -> String {
    texts::WORKSPACE_REMOVED_TEXT.to_owned()
}

/// The confirm of Add to workspace for a document shared on its own (criterion 42).
#[wasm_bindgen]
#[must_use]
pub fn workspace_take_in_confirm_text(name: &str, people: usize, workspace: &str) -> String {
    texts::take_in_confirm_text(name, people, workspace)
}

/// The alert of a document that cannot be taken in (criterion 49).
#[wasm_bindgen]
#[must_use]
pub fn workspace_take_in_refused_text(name: &str, workspace: &str) -> String {
    texts::take_in_refused_text(name, workspace)
}

/// What the document file `bytes` is to a workspace about to be shared by the device whose
/// identity entropy is `identity` (its only member): `plain`, `allowed` (taken in, criterion 42)
/// or `refused` (left out, criterion 49), for the Share confirm.
#[wasm_bindgen]
#[must_use]
pub fn workspace_share_take_in(bytes: &[u8], identity: &[u8]) -> String {
    let Ok(entropy) = identity.try_into() else {
        return "refused".to_owned();
    };
    let me = curvyo_crypto_core::IdentityKeys::from_entropy(&entropy).identity();
    match crate::session::take_in_check(bytes, &[me]) {
        TakeIn::Plain => "plain",
        TakeIn::Allowed { .. } => "allowed",
        TakeIn::Refused => "refused",
    }
    .to_owned()
}

/// The alert of a take-in whose document's own room cannot be reached (nothing changed).
#[wasm_bindgen]
#[must_use]
pub fn workspace_take_in_unreachable_text(name: &str) -> String {
    texts::take_in_unreachable_text(name)
}
