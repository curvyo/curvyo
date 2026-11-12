//! Thin host bridge of the laser operations of the active laser page (`specs/0101-laser-operations`):
//! the commands of the Operations tab, the number keys and the notice. Ids are the JSON texts of
//! node ids, as the other bridges take them.

use curvyo_document_core::{NodeId, OperationSwitch};
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The node id a host sends back (its JSON text), `None` for anything else.
fn node(text: &str) -> Option<NodeId> {
    serde_json::from_str(text).ok()
}

/// The node ids of a JSON array of id texts; anything unreadable is left out.
fn nodes(json: &str) -> Vec<NodeId> {
    serde_json::from_str::<Vec<String>>(json)
        .unwrap_or_default()
        .iter()
        .filter_map(|text| node(text))
        .collect()
}

#[wasm_bindgen]
impl WasmSession {
    /// A press on the Select bar's kind button `kind` (`cut`, `score`, `engrave`,
    /// `engrave_outline`, `cut_out`), `shift` held or not (`specs/0111` criteria 3 to 5): the
    /// operation written to as JSON text, `None` when nothing changed.
    pub fn press_operation_kind(&mut self, kind: &str, shift: bool) -> Option<String> {
        let id = self.session.press_operation_kind(kind, shift)?;
        serde_json::to_string(&id).ok()
    }

    /// The polite status of the last kind press ("Cut 3 added."), once; empty when none.
    pub fn take_operations_status(&mut self) -> String {
        self.session.take_operations_status()
    }

    /// A press on operation row `id` (`specs/0111` criterion 17): it unfolds, or folds when it
    /// was unfolded. Whether the row is of the active page.
    pub fn unfold_operation_row(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.unfold_operation_row(id))
    }

    /// The Operations view toggle in the label of laser page `page` (`specs/0111` criterion 15).
    /// Whether it is a laser page. Session state, no step.
    pub fn toggle_operations_view(&mut self, page: &str) -> bool {
        self.session.toggle_operations_view(page)
    }

    /// Types `value` into the option field `option` of operation `id` (`interval`, `scan_angle`,
    /// `overscan`, `z_offset`) or presses a toggle (`crosshatch`, `bidirectional`; non-zero is
    /// on), `specs/0111` criteria 19 to 24. The chip, empty when taken or nothing changed.
    pub fn edit_operation_option(&mut self, id: &str, option: &str, value: f64) -> String {
        node(id)
            .map(|id| self.session.edit_operation_option(id, option, value))
            .and_then(Result::err)
            .unwrap_or_default()
    }

    /// The Motorised Z axis switch of the active laser page's profile copy (`specs/0111`
    /// criterion 22). Whether it changed.
    pub fn set_motorised_z(&mut self, on: bool) -> bool {
        self.session.set_machine_motorised_z(on)
    }

    /// Renames operation `id` (criterion 3). Whether it changed.
    pub fn rename_operation(&mut self, id: &str, name: &str) -> bool {
        node(id).is_some_and(|id| self.session.rename_operation(id, name))
    }

    /// Deletes operation `id` with its memberships (criterion 4). Whether it was deleted.
    pub fn delete_operation(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.delete_operation(id))
    }

    /// Sets the Cut-out margin of operation `id` in mm (criterion 23). Whether it changed.
    pub fn set_operation_margin(&mut self, id: &str, margin_mm: f64) -> bool {
        node(id).is_some_and(|id| self.session.set_operation_margin(id, margin_mm))
    }

    /// Sets the Show switch of operation `id` (criterion 15). Whether it changed.
    pub fn set_operation_show(&mut self, id: &str, on: bool) -> bool {
        node(id).is_some_and(|id| {
            self.session
                .set_operation_switch(id, OperationSwitch::Show, on)
        })
    }

    /// Sets the Output switch of operation `id` (criterion 16). Whether it changed.
    pub fn set_operation_output(&mut self, id: &str, on: bool) -> bool {
        node(id).is_some_and(|id| {
            self.session
                .set_operation_switch(id, OperationSwitch::Output, on)
        })
    }

    /// Switches Cuts last on or off, the cutting operations moving below the marking ones when on
    /// (criterion 6). Whether it changed.
    pub fn set_cuts_last(&mut self, on: bool) -> bool {
        self.session.set_cuts_last(on)
    }

    /// Moves operation `id` to position `to` of the run order shown (criteria 5, 6). Whether it
    /// moved; a move across the section line is refused.
    pub fn move_operation(&mut self, id: &str, to: u32) -> bool {
        node(id).is_some_and(|id| self.session.move_operation(id, to as usize))
    }

    /// Ticks (`on` true) or unticks the selection's items in operation `id` (criterion 8).
    pub fn set_selection_in_operation(&mut self, id: &str, on: bool) -> bool {
        node(id).is_some_and(|id| self.session.set_selection_in_operation(id, on))
    }

    /// The number keys 1 to 9: toggles the selection in operation `number` of the list
    /// (criterion 8). Whether it changed.
    pub fn toggle_selection_by_number(&mut self, number: u32) -> bool {
        self.session.toggle_selection_by_number(number as usize)
    }

    /// Selects the items of the summary count `which` (`running`, `not_assigned`,
    /// `runs_while_hidden`, `output_off`), criterion 17. Whether it selected.
    pub fn select_operation_items(&mut self, which: &str) -> bool {
        self.session.select_operation_items(which)
    }

    /// The Operations tab as JSON (`specs/0101` criteria 1, 5, 17): rows in run order, ticks over the
    /// selection and the summary; `null` on another page.
    pub fn operations_view_json(&self) -> String {
        self.session.operations_view_json()
    }

    /// The notice of the last operation command (criterion 10), `None` when there is none.
    pub fn operations_notice(&self) -> Option<String> {
        self.session.operations_notice()
    }

    /// The Material pick for the operations `ids` (a JSON array of id texts, `specs/0028`
    /// criteria 25, 27): `material` empty means "Own numbers". Whether anything was written; the
    /// skip notice comes with [`WasmSession::operations_notice`].
    pub fn pick_operation_material(&mut self, ids: &str, material: &str) -> bool {
        let ids = nodes(ids);
        if material.is_empty() {
            self.session.use_own_numbers_on(&ids)
        } else {
            self.session.use_material_on(&ids, material)
        }
    }

    /// Types `value` into the `field` (`power`, `speed`, `passes`) of the `role` set (`main`,
    /// `outline`) of operation `id` (criterion 24). The chip, empty when taken.
    pub fn edit_operation_setting(
        &mut self,
        id: &str,
        role: &str,
        field: &str,
        value: f64,
    ) -> String {
        node(id)
            .map(|id| self.session.edit_operation_setting(id, role, field, value))
            .and_then(Result::err)
            .unwrap_or_default()
    }

    /// Reset to material of operation `id` (criterion 24). Whether it changed.
    pub fn reset_operation_to_material(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.reset_operation_to_material(id))
    }
}
