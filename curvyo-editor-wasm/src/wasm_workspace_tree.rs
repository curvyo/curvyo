//! The `wasm-bindgen` surface of a shared workspace's live tree (`specs/0095-workspace` criteria
//! 35 to 41): list edits, the disk plan, a new document's first records, the Deleted list,
//! Download again and the words of the tree. Pass-throughs to `session::workspace_tree` and
//! `workspace_tree_glue`.

use curvyo_ui_core::workspace_tree_text as texts;
use wasm_bindgen::prelude::*;

use crate::session::unhex;
use crate::wasm_api::WasmSession;
use crate::workspace_tree_glue::{deleted_json, disk_answer_json, disk_scan, list_edit, stamps};

fn seconds(now_ms: f64) -> i64 {
    // Date.now() is a whole number of milliseconds well inside i64.
    #[allow(clippy::cast_possible_truncation)]
    let seconds = (now_ms / 1000.0).floor() as i64;
    seconds
}

#[wasm_bindgen]
impl WasmSession {
    /// Records the maker's change (JSON, see `workspace_tree_glue::list_edit`) in the shared
    /// list at `now` (ms), new entries stamped with `seed` (16 random bytes): empty when done,
    /// else why not.
    pub fn workspace_list_edit(&mut self, edit: &str, now: f64, seed: &[u8]) -> String {
        let (Some(edit), Ok(seed)) = (list_edit(edit, seconds(now)), seed.try_into()) else {
            return "unreadable".to_owned();
        };
        let mut stamp = stamps(seed, seconds(now));
        match self.session.workspace_list_edit(edit, stamp()) {
            Ok(()) => String::new(),
            Err(error) => error.to_string(),
        }
    }

    /// The plan that brings the folder (the host's scan as JSON) to the list (JSON, see
    /// `workspace_tree_glue::disk_answer_json`); empty without a shared workspace.
    pub fn workspace_disk_plan(&mut self, scan: &str, now: f64, seed: &[u8]) -> String {
        let Some(workspace) = self.session.workspace_id() else {
            return String::new();
        };
        let (Some(scan), Ok(seed)) = (disk_scan(scan, workspace), seed.try_into()) else {
            return String::new();
        };
        self.session
            .workspace_disk_plan(&scan, stamps(seed, seconds(now)))
            .map(|answer| disk_answer_json(&answer))
            .unwrap_or_default()
    }

    /// A document joins the shared workspace as `document` (hex): its file bytes with
    /// `share.json` v2; its whole state goes out as the first records of its stream.
    ///
    /// # Errors
    /// When there is no shared workspace or the bytes do not open.
    pub fn workspace_adopt_document(
        &mut self,
        document: &str,
        bytes: &[u8],
    ) -> Result<Vec<u8>, JsError> {
        let id = unhex::<16>(document).ok_or_else(|| JsError::new("bad document id"))?;
        self.session
            .workspace_adopt_document(id, bytes)
            .map_err(|error| JsError::new(&error.to_string()))
    }

    /// Sends a workspace document's own changes `outgoing` that the background writer found
    /// in its `file` (criterion 14); returns the file marked synced.
    ///
    /// # Errors
    /// When there is no shared workspace, the file does not open or the push is refused.
    pub fn workspace_send_local(
        &mut self,
        document: &str,
        file: &[u8],
        outgoing: &[u8],
    ) -> Result<Vec<u8>, JsError> {
        let id = unhex::<16>(document).ok_or_else(|| JsError::new("bad document id"))?;
        self.session
            .workspace_send_local(id, file, outgoing)
            .map_err(|error| JsError::new(&error.to_string()))
    }

    /// The Deleted list's rows (JSON), times local `offset_minutes` east of UTC.
    #[must_use]
    pub fn workspace_deleted_json(&self, now: f64, offset_minutes: i32) -> String {
        let me = self.session.workspace_label().unwrap_or_default();
        #[allow(clippy::cast_sign_loss)]
        let now = seconds(now).max(0) as u64;
        deleted_json(&self.session.workspace_deleted(), me, now, offset_minutes)
    }

    /// The shared workspace's id as hex, empty without one.
    #[must_use]
    pub fn workspace_id_hex(&self) -> String {
        self.session
            .workspace_id()
            .map(|id| crate::session::hex(&id))
            .unwrap_or_default()
    }

    /// Download again: the room is read anew from its snapshot (criterion 41).
    pub fn workspace_download_again(&mut self) {
        self.session.workspace_download_again();
    }
}

/// The project block's status for an open document another member deleted (criterion 40).
#[wasm_bindgen]
#[must_use]
pub fn workspace_deleted_while_open_text(by: &str) -> String {
    texts::deleted_while_open_text(by)
}

/// "Restored Lid." (`ux-notes.md` section 5).
#[wasm_bindgen]
#[must_use]
pub fn workspace_restored_text(name: &str) -> String {
    texts::restored_text(name)
}

/// "Added Lid to the workspace." (criterion 41).
#[wasm_bindgen]
#[must_use]
pub fn workspace_added_text(name: &str) -> String {
    texts::added_text(name)
}

/// The fixed words of the live tree: `{notInWorkspace, missing, receiving, noDeleted}`.
#[wasm_bindgen]
#[must_use]
pub fn workspace_tree_words_json() -> String {
    serde_json::json!({
        "notInWorkspace": texts::NOT_IN_WORKSPACE_TEXT,
        "missing": texts::MISSING_TEXT,
        "receiving": texts::RECEIVING_TEXT,
        "noDeleted": texts::NO_DELETED_TEXT,
    })
    .to_string()
}
