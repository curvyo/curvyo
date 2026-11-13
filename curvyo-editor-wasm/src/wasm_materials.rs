//! Thin host bridge of the material library (`specs/0112-test-grid-and-materials` criteria 16
//! to 27): the record, retired and photo texts in, the host write list and photo bytes out, the
//! commands of the Materials tab; strings, scalars and byte slices only (ADR 0001 §5). A
//! command's text result is its chip or notice, empty when there is none.

use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The text of a refusal, empty when the command was taken.
fn text_of<T>(result: Result<T, String>) -> String {
    result.err().unwrap_or_default()
}

/// `None` for an empty host text.
fn some(text: &str) -> Option<&str> {
    (!text.is_empty()).then_some(text)
}

#[wasm_bindgen]
impl WasmSession {
    /// Reads the listed records, `[{"name": "<id>.json", "text": "..."}]`, and the text of
    /// `material-order.json` (empty when absent).
    pub fn load_materials(&mut self, json: &str, order: &str) {
        self.session.load_materials_json(json, some(order));
    }

    /// Reads `retired-test-ids.json` (empty when absent).
    pub fn load_retired_test_ids(&mut self, text: &str) {
        self.session.load_retired_test_ids(some(text));
    }

    /// Hands in the bytes of the photo file `name` the host read to show it.
    pub fn load_material_photo(&mut self, name: &str, bytes: &[u8]) {
        self.session.load_material_photo(name, bytes);
    }

    /// Reads the kept `test-grid-choice.json`.
    pub fn load_test_grid_choice(&mut self, text: &str) {
        self.session.load_test_grid_choice(text);
    }

    /// The host writes made since the last call, as JSON.
    pub fn take_material_writes(&mut self) -> String {
        self.session.take_material_writes_json()
    }

    /// The bytes of the photo write `handle`, once (empty when unknown).
    pub fn take_material_photo_bytes(&mut self, handle: u32) -> Vec<u8> {
        self.session
            .take_material_photo_bytes(handle)
            .unwrap_or_default()
    }

    /// The Materials tab as JSON, `null` on another page.
    pub fn materials_view_json(&self) -> String {
        self.session.materials_view_json()
    }

    /// Opens the New material form; `grid` is the node id text of the grid whose bar asked
    /// (empty: none).
    pub fn open_material_form(&mut self, grid: &str) {
        self.session
            .open_material_form(serde_json::from_str(grid).ok());
    }

    /// Cancel on the New material form.
    pub fn cancel_material_form(&mut self) {
        self.session.cancel_material_form();
    }

    /// Add on the New material form: `{"ok": true, "id"}` or `{"ok": false, "text"}`; `random`
    /// holds 16 bytes of `crypto.getRandomValues`.
    pub fn add_material(&mut self, name: &str, thickness_mm: f64, random: &[u8]) -> String {
        let result = match self.session.add_material(name, thickness_mm, random) {
            Ok(id) => serde_json::json!({"ok": true, "id": id}),
            Err(text) => serde_json::json!({"ok": false, "text": text}),
        };
        result.to_string()
    }

    /// Renames material `id` in place.
    pub fn rename_material(&mut self, id: &str, name: &str) -> String {
        text_of(self.session.rename_material(id, name))
    }

    /// The Thickness field of material `id`.
    pub fn set_material_thickness(&mut self, id: &str, mm: f64) -> String {
        text_of(self.session.set_material_thickness(id, mm))
    }

    /// Deletes material `id` (the armed second press).
    pub fn delete_material(&mut self, id: &str) -> String {
        text_of(self.session.delete_material(id))
    }

    /// Moves the row `id` to `index` of the rows. Whether it moved.
    pub fn move_material(&mut self, id: &str, index: u32) -> bool {
        self.session.move_material(id, index as usize)
    }

    /// Unfolds the material row `id` (empty: folds).
    pub fn unfold_material(&mut self, id: &str) {
        self.session.unfold_material(some(id));
    }

    /// Folds or unfolds the laser block `profile_id`.
    pub fn toggle_material_laser(&mut self, profile_id: &str) {
        self.session.toggle_material_laser(profile_id);
    }

    /// Unfolds or folds "+ Sets for another laser".
    pub fn open_material_laser_pick(&mut self, open: bool) {
        self.session.open_material_laser_pick(open);
    }

    /// A pick of "+ Sets for another laser". Whether it was offered.
    pub fn pick_material_laser(&mut self, id: &str, profile_id: &str) -> bool {
        self.session.pick_material_laser(id, profile_id)
    }

    /// "+ `<kind>` set" of the laser `profile_id` in material `id`; `random` holds 16 bytes.
    pub fn add_material_set(
        &mut self,
        id: &str,
        profile_id: &str,
        kind: &str,
        random: &[u8],
    ) -> String {
        text_of(self.session.add_material_set(id, profile_id, kind, random))
    }

    /// Types `value` into `field` of a set; `random` holds 16 bytes.
    pub fn edit_material_set(
        &mut self,
        id: &str,
        profile_id: &str,
        kind: &str,
        field: &str,
        value: f64,
        random: &[u8],
    ) -> String {
        text_of(
            self.session
                .edit_material_set(id, profile_id, kind, field, value, random),
        )
    }

    /// The set's armed trash; `random` holds 16 bytes.
    pub fn delete_material_set(
        &mut self,
        id: &str,
        profile_id: &str,
        kind: &str,
        random: &[u8],
    ) -> String {
        text_of(
            self.session
                .delete_material_set(id, profile_id, kind, random),
        )
    }

    /// Opens the test `test` of the unfolded material (empty: back).
    pub fn open_material_test(&mut self, test: &str) {
        self.session.open_material_test(some(test));
    }

    /// A tap on a cell of the judge table; `random` holds 16 bytes.
    pub fn judge_test(
        &mut self,
        material: &str,
        test: &str,
        column: u8,
        row: u8,
        random: &[u8],
    ) -> String {
        text_of(self.session.judge_test(material, test, column, row, random))
    }

    /// Leaving the Note field.
    pub fn set_test_note(&mut self, material: &str, test: &str, note: &str) -> String {
        text_of(self.session.set_test_note(material, test, note))
    }

    /// The encoded photo bytes from a file, the camera or Crop (`cropped`).
    pub fn set_test_photo(
        &mut self,
        material: &str,
        test: &str,
        bytes: &[u8],
        cropped: bool,
    ) -> String {
        text_of(self.session.set_test_photo(material, test, bytes, cropped))
    }

    /// The photo's trash.
    pub fn remove_test_photo(&mut self, material: &str, test: &str) -> String {
        text_of(self.session.remove_test_photo(material, test))
    }

    /// The test's armed trash.
    pub fn delete_test(&mut self, material: &str, test: &str) -> String {
        text_of(self.session.delete_test(material, test))
    }

    /// "Place on `<page>`" or "Select on page" of a test.
    pub fn place_material_test(&mut self, material: &str, test: &str) -> String {
        text_of(self.session.place_material_test(material, test))
    }

    /// The notice of the last material command, once.
    pub fn take_materials_notice(&mut self) -> Option<String> {
        self.session.take_materials_notice()
    }
}
