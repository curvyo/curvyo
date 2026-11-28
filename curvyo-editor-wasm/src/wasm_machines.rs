//! Thin host bridge of the Machines tab and the shared output-page commands (ADR 0098 decisions 3
//! to 5): the tab's view, a machine's field, duplicate, rename and delete, the offer to update
//! the pages, the Machine group's pick, reset and save, and a family row's field.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;
use crate::wasm_operations::node;

/// The first 16 bytes of `random`, zero-padded: the caller passes `crypto.getRandomValues`.
fn sixteen(random: &[u8]) -> [u8; 16] {
    let mut bytes = [0_u8; 16];
    for (slot, byte) in bytes.iter_mut().zip(random) {
        *slot = *byte;
    }
    bytes
}

#[allow(clippy::cast_possible_truncation)] // a Unix time in milliseconds
fn millis(now_ms: f64) -> i64 {
    now_ms as i64
}

#[wasm_bindgen]
impl WasmSession {
    /// The Machines tab as JSON, `null` off an output page.
    pub fn machines_view_json(&self) -> String {
        self.session.machines_view_json()
    }

    /// Sets a machine's field from typed text: `{"notice"}` or `{"error"}` as JSON, `{}` when
    /// nothing changed.
    pub fn set_machine_value(&mut self, id: &str, field: &str, text: &str, now_ms: f64) -> String {
        match self
            .session
            .set_machine_value(id, field, text, millis(now_ms))
        {
            Ok(Some(notice)) => serde_json::json!({ "notice": notice }).to_string(),
            Ok(None) => "{}".to_owned(),
            Err(error) => serde_json::json!({ "error": error }).to_string(),
        }
    }

    /// Duplicates machine `id` ("+ Machine" duplicates the built-in): the new id, empty when
    /// refused.
    pub fn duplicate_machine(&mut self, id: &str, random: &[u8], now_ms: f64) -> String {
        self.session
            .duplicate_machine(id, sixteen(random), millis(now_ms))
            .unwrap_or_default()
    }

    /// Renames machine `id`. Whether it changed.
    pub fn rename_machine(&mut self, id: &str, name: &str, now_ms: f64) -> bool {
        self.session.rename_machine(id, name, millis(now_ms))
    }

    /// Deletes machine `id`; pages keep their copy. Whether it was deleted.
    pub fn delete_machine(&mut self, id: &str) -> bool {
        self.session.delete_machine(id)
    }

    /// Update pages of the offer after an edit: one step. Whether a page changed.
    pub fn follow_machine(&mut self) -> bool {
        self.session.follow_machine()
    }

    /// Keep pages: the offer closes.
    pub fn keep_machine_pages(&mut self) {
        self.session.keep_machine_pages();
    }

    /// Pick profile on the active output page of any family. Whether it changed.
    pub fn pick_output_profile(&mut self, id: &str) -> bool {
        self.session.pick_output_profile(id)
    }

    /// Reset from profile on the active output page. Whether it changed.
    pub fn reset_output_profile(&mut self) -> bool {
        self.session.reset_output_profile()
    }

    /// Save as machine on the active output page: the new id, empty when refused.
    pub fn save_output_profile(&mut self, random: &[u8], now_ms: f64) -> String {
        self.session
            .save_output_profile(sixteen(random), millis(now_ms))
            .unwrap_or_default()
    }

    /// Types `text` into the field `field` of family operation `id`'s unfolded row: the field
    /// message, empty when it took.
    pub fn edit_operation_field(&mut self, id: &str, field: &str, text: &str) -> String {
        node(id)
            .map(|id| self.session.edit_operation_field(id, field, text))
            .and_then(Result::err)
            .unwrap_or_default()
    }
}
