//! The workspace's `wasm-bindgen` surface (`specs/0094-workspace` criteria 1, 4 to 8, 13 to 19):
//! the open workspace's rows and plans, the bytes of a copy, and the section's fixed words.
//! Every method is a pass-through to the plain-Rust glue or to `curvyo-ui-core`; strings and
//! scalars only (ADR 0001 §5).

use curvyo_ui_core::workspace_text::{
    OpenRefusal, follow_failed_text, open_refusal_text, operation_failed_text, unreadable_text,
};
use wasm_bindgen::prelude::*;

use crate::{APP_VERSION, WorkspaceGlue, workspace_copy_bytes};

/// The open workspace (one per page; it outlives the document sessions it opens).
#[wasm_bindgen]
#[derive(Debug, Default)]
pub struct WasmWorkspace {
    glue: WorkspaceGlue,
}

#[wasm_bindgen]
impl WasmWorkspace {
    /// An empty workspace named `name`.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            glue: WorkspaceGlue::new(name),
        }
    }

    /// Takes the host's scan (JSON); whether it changed.
    pub fn set_scan(&mut self, json: &str) -> bool {
        self.glue.set_scan(json)
    }

    /// The header line and the shown rows (JSON), times local `offset_minutes` east of UTC.
    #[must_use]
    pub fn rows_json(&self, now: f64, offset_minutes: i32) -> String {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        self.glue.rows_json(now.max(0.0) as u64, offset_minutes)
    }

    /// Takes a shared workspace's row states (JSON `{shared, outside, missing, receiving}`);
    /// whether they changed.
    pub fn set_shared_states(&mut self, json: &str) -> bool {
        self.glue.set_shared_states(json)
    }

    /// Expands or collapses a folder (path as JSON); whether it is one.
    pub fn set_expanded(&mut self, path: &str, expanded: bool) -> bool {
        self.glue.set_expanded(path, expanded)
    }

    /// Whether the last scan holds `path` (JSON).
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.glue.contains(path)
    }

    /// Keeps expanded folders open across a move from `from` to `to` (JSON paths).
    pub fn moved(&mut self, from: &str, to: &str) {
        self.glue.moved(from, to);
    }

    /// Plans a request (JSON): the operation and its words, or the refusal (JSON).
    #[must_use]
    pub fn plan(&self, request: &str) -> String {
        self.glue.plan(request)
    }
}

/// The bytes of a copy of a document without its share record (Duplicate, criterion 19).
///
/// # Errors
/// The open error's sentence when `bytes` is not a document this build reads.
#[wasm_bindgen]
pub fn workspace_copy(bytes: &[u8]) -> Result<Vec<u8>, JsError> {
    workspace_copy_bytes(bytes, APP_VERSION).map_err(|error| JsError::new(&error))
}

/// The refusal line of Open workspace for the host's code (criteria 4, 23).
#[wasm_bindgen]
#[must_use]
pub fn workspace_open_refusal(code: &str) -> String {
    open_refusal_text(OpenRefusal::from_code(code)).to_owned()
}

/// The row text of a file that cannot be read (criterion 8).
#[wasm_bindgen]
#[must_use]
pub fn workspace_unreadable(code: &str) -> String {
    unreadable_text(code).to_owned()
}

/// The alert of a file operation the host could not run, for its code.
#[wasm_bindgen]
#[must_use]
pub fn workspace_failure(code: &str) -> String {
    operation_failed_text(code).to_owned()
}

/// The alert when the folder could not follow a change of the shared list.
#[wasm_bindgen]
#[must_use]
pub fn workspace_follow_failed() -> String {
    follow_failed_text().to_owned()
}
