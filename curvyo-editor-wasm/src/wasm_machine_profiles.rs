//! Thin host bridge of the Machine block (`specs/0026-machine-profile`): the library texts in, the
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

    /// The host writes made since the last call, as a JSON array.
    pub fn take_machine_writes(&mut self) -> String {
        self.session.take_machine_writes_json()
    }

    /// Picks the library profile `id` for the active manufacturing.
    pub fn pick_machine_profile(&mut self, id: &str) -> bool {
        self.session.pick_machine_profile(id)
    }

    /// The Name field.
    pub fn set_machine_name(&mut self, text: &str) -> String {
        self.session.set_machine_name(text).unwrap_or_default()
    }

    /// The origin corner by its stored name.
    pub fn set_machine_origin(&mut self, name: &str) -> bool {
        self.session.set_machine_origin(name)
    }

    /// The Max feed field.
    pub fn set_machine_max_feed(&mut self, value: f64) -> String {
        self.session.set_machine_max_feed(value).unwrap_or_default()
    }

    /// The Max power field.
    pub fn set_machine_max_power(&mut self, value: f64) -> String {
        self.session
            .set_machine_max_power(value)
            .unwrap_or_default()
    }

    /// The Laser mode switch.
    pub fn set_machine_laser_mode(&mut self, on: bool) {
        self.session.set_machine_laser_mode(on);
    }

    /// Reset from profile.
    pub fn reset_machine_profile(&mut self) -> bool {
        self.session.reset_machine_profile()
    }

    /// Save as profile.
    pub fn save_machine_profile(&mut self, random: &[u8], now_ms: f64) -> String {
        self.session
            .save_machine_profile(sixteen(random), millis(now_ms))
            .unwrap_or_default()
    }

    /// Update profile (the armed second press).
    pub fn update_machine_profile(&mut self, now_ms: f64) -> String {
        self.session
            .update_machine_profile(millis(now_ms))
            .unwrap_or_default()
    }

    /// Duplicate: the new profile's id, empty when refused.
    pub fn duplicate_machine_profile(&mut self, id: &str, random: &[u8], now_ms: f64) -> String {
        self.session
            .duplicate_machine_profile(id, sixteen(random), millis(now_ms))
            .unwrap_or_default()
    }

    /// Rename in the list.
    pub fn rename_machine_profile(&mut self, id: &str, name: &str, now_ms: f64) -> String {
        self.session
            .rename_machine_profile(id, name, millis(now_ms))
            .unwrap_or_default()
    }

    /// Delete (the armed second press).
    pub fn delete_machine_profile(&mut self, id: &str) -> bool {
        self.session.delete_machine_profile(id)
    }

    /// The connection fields of the chosen user profile.
    pub fn set_machine_connection(
        &mut self,
        address: &str,
        port: &str,
        baud: f64,
        now_ms: f64,
    ) -> String {
        self.session
            .set_machine_connection(address, port, baud, millis(now_ms))
            .unwrap_or_default()
    }
}
