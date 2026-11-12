//! Thin host bridge of the material library (`specs/0028-material-test-library`): the record
//! texts in, the host write list out, the commands of the Materials section and the Save as
//! material form; strings and scalars only (ADR 0001 §5). A command's text result is its chip or
//! notice, empty when there is none.

use wasm_bindgen::prelude::*;

use crate::session::SaveAsMaterial;
use crate::wasm_api::WasmSession;

#[wasm_bindgen]
impl WasmSession {
    /// Reads the listed records, `[{"name": "<id>.json", "text": "..."}]`, and the text of
    /// `material-order.json` (empty when absent).
    pub fn load_materials(&mut self, json: &str, order: &str) {
        self.session
            .load_materials_json(json, (!order.is_empty()).then_some(order));
    }

    /// Reads the kept `test-grid-choice.json`.
    pub fn load_test_grid_choice(&mut self, text: &str) {
        self.session.load_test_grid_choice(text);
    }

    /// The host writes made since the last call, as the JSON list of `adrs.md` §6.
    pub fn take_material_writes(&mut self) -> String {
        self.session.take_material_writes_json()
    }

    /// The Materials section as JSON, `null` on another page.
    pub fn materials_view_json(&self) -> String {
        self.session.materials_view_json()
    }

    /// Save as material: the new or existing material's id, or (with `ok` false) the chip text,
    /// as `{"ok": true, "id"}` or `{"ok": false, "text"}`. `existing` empty means a new material;
    /// `random` holds 32 bytes of `crypto.getRandomValues`.
    pub fn save_as_material(
        &mut self,
        existing: &str,
        name: &str,
        thickness_mm: f64,
        passes: f64,
        random: &[u8],
    ) -> String {
        let request = SaveAsMaterial {
            existing: (!existing.is_empty()).then(|| existing.to_owned()),
            name: name.to_owned(),
            thickness_mm,
            passes,
        };
        let result = match self.session.save_as_material(&request, random) {
            Ok(id) => serde_json::json!({"ok": true, "id": id}),
            Err(text) => serde_json::json!({"ok": false, "text": text}),
        };
        result.to_string()
    }

    /// Types `value` into the `field` (`power`, `speed`, `passes`) of the `kind` set of material
    /// `id`. The chip or refusal text, empty when taken.
    pub fn edit_material_set(
        &mut self,
        id: &str,
        kind: &str,
        field: &str,
        value: f64,
        random: &[u8],
    ) -> String {
        self.session
            .edit_material_set(id, kind, field, value, random)
            .err()
            .unwrap_or_default()
    }

    /// Removes the `kind` set of material `id` (the armed set trash). The refusal, empty when
    /// removed.
    pub fn remove_material_set(&mut self, id: &str, kind: &str, random: &[u8]) -> String {
        self.session
            .remove_material_set(id, kind, random)
            .err()
            .unwrap_or_default()
    }

    /// Renames material `id` in place. The refusal, empty when renamed or reverted.
    pub fn rename_material(&mut self, id: &str, name: &str) -> String {
        self.session
            .rename_material(id, name)
            .err()
            .unwrap_or_default()
    }

    /// The Thickness field of material `id`. The chip, empty when taken.
    pub fn set_material_thickness(&mut self, id: &str, mm: f64) -> String {
        self.session
            .set_material_thickness(id, mm)
            .err()
            .unwrap_or_default()
    }

    /// Deletes material `id` (the armed second press). The refusal, empty when deleted.
    pub fn delete_material(&mut self, id: &str) -> String {
        self.session.delete_material(id).err().unwrap_or_default()
    }

    /// Moves the row `id` to `index` of this machine's rows (arrows, drag, Alt+arrows). Whether
    /// it moved.
    pub fn move_material(&mut self, id: &str, index: u32) -> bool {
        self.session.move_material(id, index as usize)
    }

    /// Selects the material row `id` (empty: none), its fields open.
    pub fn select_material(&mut self, id: &str) {
        self.session.select_material((!id.is_empty()).then_some(id));
    }

    /// The notice of the last material command, once.
    pub fn take_materials_notice(&mut self) -> Option<String> {
        self.session.take_materials_notice()
    }
}
