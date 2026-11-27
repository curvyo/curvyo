//! Thin host bridge of embroidery pages (`specs/0160-embroidery` slice 1): the page's view as
//! JSON, the simulation's position, and the commands of its machine, hoop and operations.

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

/// `Ok(_)` as empty text, an error as its message.
fn message(result: Result<bool, String>) -> String {
    result.err().unwrap_or_default()
}

#[wasm_bindgen]
impl WasmSession {
    /// The active embroidery page's view, `null` on another page.
    pub fn embroidery_view_json(&self) -> String {
        self.session.embroidery_view_json()
    }

    /// The machine list of Add page with Embroidery.
    pub fn embroidery_profiles_json(&self) -> String {
        self.session.embroidery_profiles_json()
    }

    /// Pick profile `id` on the active embroidery page; whether it changed.
    pub fn pick_embroidery_profile(&mut self, id: &str) -> bool {
        self.session.pick_embroidery_profile(id)
    }

    /// Save as profile; the new id, empty when nothing was saved.
    pub fn save_embroidery_profile(&mut self, random: &[u8], now_ms: f64) -> String {
        #[allow(clippy::cast_possible_truncation)] // a Unix time in milliseconds
        let now = now_ms as i64;
        self.session
            .save_embroidery_profile(sixteen(random), now)
            .unwrap_or_default()
    }

    /// The Speed field; the field message, empty when it read.
    pub fn set_embroidery_speed(&mut self, text: &str) -> String {
        message(self.session.set_embroidery_speed(text))
    }

    /// Set hoop `key` (`150x250`, `100x100`); whether it changed.
    pub fn set_embroidery_hoop(&mut self, key: &str) -> bool {
        self.session.set_embroidery_hoop(key)
    }

    /// A press on Running stitch, Tatami fill or Satin; the operation written to, empty when
    /// nothing changed.
    pub fn press_embroidery_kind(&mut self, kind: &str, shift: bool) -> String {
        self.session
            .press_embroidery_kind(kind, shift)
            .map(|id| id.full_id())
            .unwrap_or_default()
    }

    /// Takes the selection out of operation `id`; whether it changed.
    pub fn remove_selection_from_embroidery_operation(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.remove_selection_from_embroidery_operation(id))
    }

    /// Deletes embroidery operation `id`; whether it was deleted.
    pub fn delete_embroidery_operation(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.delete_embroidery_operation(id))
    }

    /// Sets a number of operation `id` by its stored key; the field message, empty when it read.
    pub fn edit_embroidery_setting(&mut self, id: &str, field: &str, text: &str) -> String {
        match node(id) {
            Some(id) => message(self.session.edit_embroidery_setting(id, field, text)),
            None => String::new(),
        }
    }

    /// Sets the thread of operation `id` to `hex`; whether it changed.
    pub fn set_embroidery_thread(&mut self, id: &str, hex: &str) -> bool {
        node(id).is_some_and(|id| self.session.set_embroidery_thread(id, hex))
    }

    /// Moves the simulation to `position` stitches; the readout as JSON.
    pub fn set_embroidery_play(&mut self, position: u32) -> String {
        self.session
            .set_embroidery_play(usize::try_from(position).unwrap_or(usize::MAX))
    }
}
