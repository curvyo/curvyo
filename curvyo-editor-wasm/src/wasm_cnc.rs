//! Thin host bridge of CNC pages (`specs/0161-cnc-output` slice 1, ADR 0098): the Add page machine
//! list and the stock and tool commands of the Fixtures and Tools tabs.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

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
    /// The machine list of Add page with CNC.
    pub fn cnc_profiles_json(&self) -> String {
        self.session.cnc_profiles_json()
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
}
