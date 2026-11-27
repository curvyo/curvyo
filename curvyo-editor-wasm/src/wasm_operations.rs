//! Thin host bridge of the laser operations of the active laser page (`specs/0101-laser-operations`):
//! the commands of the Operations tab, the number keys and the notice. Ids are the JSON texts of
//! node ids, as the other bridges take them.

use curvyo_document_core::{NodeId, OperationSwitch};
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// The node id a host sends back (its JSON text), `None` for anything else.
pub(crate) fn node(text: &str) -> Option<NodeId> {
    serde_json::from_str(text).ok()
}

/// The node ids of a JSON array of id texts; anything unreadable is left out.
pub(crate) fn nodes(json: &str) -> Vec<NodeId> {
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
        self.session.untracked_mut().take_operations_status()
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
    /// `overscan`, `z_offset`; the Image settings `threshold`, `lowest_power`, `brightness`,
    /// `contrast`, `gamma`) or presses a toggle (`crosshatch`, `bidirectional`, `invert`;
    /// non-zero is on), `specs/0111` criteria 19 to 24, `specs/0123` criteria 11, 12. The chip,
    /// empty when taken or nothing changed.
    pub fn edit_operation_option(&mut self, id: &str, option: &str, value: f64) -> String {
        node(id)
            .map(|id| self.session.edit_operation_option(id, option, value))
            .and_then(Result::err)
            .unwrap_or_default()
    }

    /// Picks the Dither mode `mode` of Engrave operation `id` (`threshold`, `ordered`,
    /// `floyd_steinberg`, `jarvis`, `atkinson`, `stucki`, `grey`; `specs/0123` criteria 11, 12):
    /// one step "Edit settings". Whether it was written.
    pub fn set_operation_dither(&mut self, id: &str, mode: &str) -> bool {
        node(id).is_some_and(|id| self.session.set_operation_dither(id, mode))
    }

    /// Whether the Tabs toggle shows in the Select bar and whether it is on (`specs/0139` UX 4.2),
    /// as JSON `{ "shown": bool, "on": bool }`.
    pub fn tabs_tool_state(&self) -> String {
        format!(
            "{{\"shown\":{},\"on\":{}}}",
            self.session.tabs_tool_shown(),
            self.session.tabs_tool()
        )
    }

    /// Turns the Tabs tool on or off (no step). Whether it changed.
    pub fn set_tabs_tool(&mut self, on: bool) -> bool {
        self.session.set_tabs_tool(on)
    }

    /// The width chip of the current tab as JSON (`width_mm`, `own_width`, `at`), `""` without
    /// one (`specs/0139` criterion 6).
    pub fn current_tab_chip(&self) -> String {
        self.session
            .current_tab_chip()
            .and_then(|chip| serde_json::to_string(&chip).ok())
            .unwrap_or_default()
    }

    /// Types `mm` as the current tab's width, a value that is not finite returns it to the
    /// operation's width: one step "Tab width". The chip text, empty when taken.
    pub fn set_current_tab_width(&mut self, mm: f64) -> String {
        let width = mm.is_finite().then_some(mm);
        self.session
            .set_current_tab_width(width)
            .err()
            .unwrap_or_default()
    }

    /// The Tabs pill of operation `id` for the selected items: Own (`true`) or Operation's.
    /// Whether it wrote.
    pub fn set_selection_tabs_own(&mut self, id: &str, own: bool) -> bool {
        node(id).is_some_and(|id| self.session.set_selection_tabs_own(id, own))
    }

    /// The Path options pill of operation `id` for the selected items (`specs/0139` criterion
    /// 21): Own (`true`) or Operation's. Whether it wrote.
    pub fn set_selection_path_own(&mut self, id: &str, own: bool) -> bool {
        node(id).is_some_and(|id| self.session.set_selection_path_own(id, own))
    }

    /// Types `value` into the own path field `option` of the selected items in operation `id`
    /// (`lead_in`, `lead_out` as the pill's segment index, `lead_in_mm`, `lead_out_mm`,
    /// `overcut`): one step "Edit settings". The chip text, empty when taken.
    pub fn edit_selection_path_option(&mut self, id: &str, option: &str, value: f64) -> String {
        let Some(id) = node(id) else {
            return String::new();
        };
        self.session
            .edit_selection_path_option(id, option, value)
            .err()
            .unwrap_or_default()
    }

    /// The Automatic button of the item part of operation `id`: every selected closed outline
    /// returns to Automatic, one step "Automatic start". Whether it wrote.
    pub fn set_selection_start_automatic(&mut self, id: &str) -> bool {
        node(id).is_some_and(|id| self.session.set_selection_start_automatic(id))
    }

    /// The Direction pick of the item part of operation `id` (`automatic`, `as_drawn`,
    /// `reversed`): one step "Direction". Whether it wrote.
    pub fn set_selection_direction(&mut self, id: &str, direction: &str) -> bool {
        node(id).is_some_and(|id| self.session.set_selection_direction(id, direction))
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

    /// Whether the Operations tab and the output group read a stale job (`specs/0123` criteria 12,
    /// 37 d) or an adjusted picture's canvas look waits (`specs/0135` criterion 15): the host then
    /// calls `advance_job` once per animation frame.
    pub fn job_pending(&self) -> bool {
        self.session.ticks_pending()
    }

    /// One animation frame's share of the pending job's raster dots, else of a canvas look;
    /// whether the job or a look was finished by this call, so the host reads the views and
    /// draws again.
    pub fn advance_job(&self) -> bool {
        self.session.advance_tick()
    }

    /// The host's 300 ms mark of a pending job: the views now write its times as "…" (UX notes
    /// P35).
    pub fn show_stale_words(&self) {
        self.session.show_stale_words();
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

    /// The pick pick for the operations `ids` (`specs/0146` criterion 13): each follows the pick
    /// `pick` (`best`, `second_best`, `fastest`, `deepest`) of its linked material. Whether
    /// anything was written; the skip notice comes with [`WasmSession::operations_notice`].
    pub fn pick_operation_pick(&mut self, ids: &str, pick: &str) -> bool {
        let ids = nodes(ids);
        self.session.use_pick_on(&ids, pick)
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
