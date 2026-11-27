//! Thin host bridge of CNC pages (`specs/0161-cnc-output` slice 1): the page's view as JSON, the
//! play position, Send, and the commands of its machine, stock, tools and operations.

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

#[allow(clippy::cast_possible_truncation)] // a Unix time in milliseconds
fn millis(now_ms: f64) -> i64 {
    now_ms as i64
}

/// A CNC job for the connector, as the host's link store takes it.
#[wasm_bindgen]
pub struct CncSentJob {
    bytes: Vec<u8>,
    name: String,
    profile_id: String,
    address: String,
}

#[wasm_bindgen]
impl CncSentJob {
    /// The G-code's bytes.
    #[wasm_bindgen(getter)]
    pub fn bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }
    /// The page's name.
    #[wasm_bindgen(getter)]
    pub fn name(&self) -> String {
        self.name.clone()
    }
    /// The profile whose pairing is used.
    #[wasm_bindgen(getter)]
    pub fn profile_id(&self) -> String {
        self.profile_id.clone()
    }
    /// The connector's address.
    #[wasm_bindgen(getter)]
    pub fn address(&self) -> String {
        self.address.clone()
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The active CNC page's view, `null` on another page.
    pub fn cnc_view_json(&self) -> String {
        self.session.cnc_view_json()
    }

    /// The machine list of Add page with CNC.
    pub fn cnc_profiles_json(&self) -> String {
        self.session.cnc_profiles_json()
    }

    /// Pick profile `id` on the active CNC page; whether it changed.
    pub fn pick_cnc_profile(&mut self, id: &str) -> bool {
        self.session.pick_cnc_profile(id)
    }

    /// Save as profile; the new id, empty when nothing was saved.
    pub fn save_cnc_profile(&mut self, random: &[u8], now_ms: f64) -> String {
        self.session
            .save_cnc_profile(sixteen(random), millis(now_ms))
            .unwrap_or_default()
    }

    /// The connector address of the page's profile; the message, empty when it was taken.
    pub fn set_cnc_connector(&mut self, address: &str, now_ms: f64) -> String {
        message(self.session.set_cnc_connector(address, millis(now_ms)))
    }

    /// A machine field; the message, empty when it read.
    pub fn set_cnc_machine_value(&mut self, field: &str, text: &str) -> String {
        message(self.session.set_cnc_machine_value(field, text))
    }

    /// A stock field; the message, empty when it read.
    pub fn edit_cnc_stock(&mut self, field: &str, text: &str) -> String {
        message(self.session.edit_cnc_stock(field, text))
    }

    /// + Tool; the new tool's id, empty when nothing changed.
    pub fn add_cnc_tool(&mut self, random: &[u8]) -> String {
        self.session
            .add_cnc_tool(sixteen(random))
            .unwrap_or_default()
    }

    /// A tool field; the message, empty when it read.
    pub fn edit_cnc_tool(&mut self, id: &str, field: &str, text: &str) -> String {
        message(self.session.edit_cnc_tool(id, field, text))
    }

    /// Deletes tool `id`; whether it was deleted.
    pub fn remove_cnc_tool(&mut self, id: &str) -> bool {
        self.session.remove_cnc_tool(id)
    }

    /// A press on Profile outside or inside; the operation written to, empty when nothing
    /// changed.
    pub fn press_cnc_profile(&mut self, side: &str, shift: bool) -> String {
        self.session
            .press_cnc_profile(side, shift)
            .map(|id| id.full_id())
            .unwrap_or_default()
    }

    /// Takes the selection out of operation `id`; whether it changed.
    pub fn remove_selection_from_cnc_operation(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.remove_selection_from_cnc_operation(id))
    }

    /// Deletes CNC operation `id`; whether it was deleted.
    pub fn delete_cnc_operation(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.delete_cnc_operation(id))
    }

    /// Sets a field of Profile `id`; the message, empty when it read.
    pub fn edit_cnc_setting(&mut self, id: &str, field: &str, text: &str) -> String {
        match node(id) {
            Some(id) => message(self.session.edit_cnc_setting(id, field, text)),
            None => String::new(),
        }
    }

    /// Moves the play position to `seconds`; the readout as JSON.
    pub fn set_cnc_play(&mut self, seconds: f64) -> String {
        self.session.set_cnc_play(seconds)
    }

    /// Send to machine on a CNC page: the job for the connector, `undefined` without one.
    pub fn cnc_send_job(&mut self) -> Option<CncSentJob> {
        self.session.cnc_send_job().map(|job| CncSentJob {
            bytes: job.bytes,
            name: job.name,
            profile_id: job.profile_id,
            address: job.address,
        })
    }
}
