//! Thin host bridge of the machine library (`specs/0026-machine-profile`): the library texts in, the
//! host writes out, and the commands; strings and scalars only (ADR 0001 §5). A command's text
//! result is its notice, empty when there is none.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The first 16 bytes of `random`, zero-padded: the caller passes `crypto.getRandomValues`.
fn sixteen(random: &[u8]) -> [u8; 16] {
    let mut bytes = [0; 16];
    for (slot, byte) in bytes.iter_mut().zip(random) {
        *slot = *byte;
    }
    bytes
}

/// Unix milliseconds from the page's clock.
#[allow(clippy::cast_possible_truncation)] // a clock value far inside the i64 range
fn millis(now_ms: f64) -> i64 {
    now_ms as i64
}

#[wasm_bindgen]
impl WasmSession {
    /// Reads the listed records, `[{"name": "<id>.json", "text": "..."}]`.
    pub fn load_machine_profiles(&mut self, json: &str) {
        self.session.load_machine_profiles_json(json);
    }

    /// Reads the kept `machine-choice.json`.
    pub fn load_machine_choice(&mut self, text: &str) {
        self.session.load_machine_choice(text);
    }

    /// The host could not write the profile record `id`: a set edit that wrote it is undone
    /// (`0113` criterion 45). The notice, or `undefined`.
    pub fn machine_record_write_failed(&mut self, id: &str) -> Option<String> {
        self.session.machine_record_write_failed(id)
    }

    /// The host writes made since the last call, as a JSON array.
    pub fn take_machine_writes(&mut self) -> String {
        self.session.untracked_mut().take_machine_writes_json()
    }

    /// Save as profile.
    pub fn save_machine_profile(&mut self, random: &[u8], now_ms: f64) -> String {
        self.session
            .save_machine_profile(sixteen(random), millis(now_ms))
            .unwrap_or_default()
    }
}
