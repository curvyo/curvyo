//! The Layers panel's `wasm-bindgen` surface (`specs/0050-layers-and-objects-panel`): a further
//! `impl WasmSession` block whose every method passes straight through to `Session`, which holds
//! the rules and the words. Strings and scalars only (ADR 0001 §5); a node id travels as the JSON
//! the rows carry, and a reply as `{"kind", "text"}`.

use curvyo_ui_core::Reorder;
use wasm_bindgen::prelude::*;

use crate::session::{PanelReply, RowFlag, RowPress};
use crate::wasm_api::WasmSession;

fn reorder_from(name: &str) -> Option<Reorder> {
    Some(match name {
        "front" => Reorder::Front,
        "back" => Reorder::Back,
        "forward" => Reorder::Forward,
        "backward" => Reorder::Backward,
        "out" => Reorder::Out,
        "in" => Reorder::In,
        _ => return None,
    })
}

#[wasm_bindgen]
impl WasmSession {
    /// The Layers tab and the rows `first` to `first + count`, as JSON.
    pub fn layers_view(&mut self, first: u32, count: u32) -> String {
        self.session
            .layers_view_json(first as usize, count as usize)
    }

    /// The subject line of the Layers tab.
    #[must_use]
    pub fn layers_subject(&self) -> String {
        self.session.layers_subject()
    }

    /// The row to scroll into view after a canvas selection, once; -1 for none.
    pub fn take_layer_scroll(&mut self) -> i32 {
        self.session
            .take_layer_scroll()
            .and_then(|row| i32::try_from(row).ok())
            .unwrap_or(-1)
    }

    /// Shift+Ctrl+L: the Layers tab becomes active.
    pub fn panel_shortcut_layers(&mut self) {
        self.session.panel_shortcut_layers();
    }

    /// A press on a row: `press` is `"plain"`, `"toggle"` or `"range"`.
    pub fn press_layer_row(&mut self, id: &str, press: &str) -> String {
        match RowPress::from_name(press) {
            Some(press) => self.session.press_layer_row(id, press).to_json(),
            None => PanelReply::Quiet.to_json(),
        }
    }

    /// Select contents on a layer row.
    pub fn select_layer_contents(&mut self, id: &str) -> String {
        self.session.select_layer_contents(id).to_json()
    }

    /// Opens or closes a container row; returns whether it changed.
    pub fn set_layer_expanded(&mut self, id: &str, expanded: bool) -> bool {
        self.session.set_layer_expanded(id, expanded)
    }

    /// The row under the pointer or the focus (empty for none).
    pub fn set_layer_hover(&mut self, id: &str) {
        self.session.set_layer_hover(id);
    }

    /// The eye (`flag` `"visible"`) or the lock (`"locked"`) of a row.
    pub fn toggle_layer_row_flag(&mut self, id: &str, flag: &str) -> String {
        match RowFlag::from_name(flag) {
            Some(flag) => self.session.toggle_layer_row_flag(id, flag).to_json(),
            None => PanelReply::Quiet.to_json(),
        }
    }

    /// Commits a rename.
    pub fn rename_layer_row(&mut self, id: &str, name: &str) -> String {
        self.session.rename_layer_row(id, name).to_json()
    }

    /// The Opacity field is dragged (percent).
    pub fn preview_layer_opacity(&mut self, percent: f64) {
        self.session.preview_layer_opacity(percent);
    }

    /// The Opacity field's release: the previewed value is committed once.
    pub fn commit_layer_opacity_preview(&mut self) -> String {
        self.session.commit_layer_opacity_preview().to_json()
    }

    /// The panel hint while the Pen has an unfinished path, else empty.
    #[must_use]
    pub fn layers_pen_hint(&self) -> String {
        self.session.layers_pen_hint()
    }

    /// Escape in the Opacity field.
    pub fn cancel_layer_opacity(&mut self) {
        self.session.cancel_layer_opacity();
    }

    /// The Opacity field commits (percent).
    pub fn commit_layer_opacity(&mut self, percent: f64) -> String {
        self.session.commit_layer_opacity(percent).to_json()
    }

    /// New layer.
    pub fn new_layer_row(&mut self) -> String {
        self.session.new_layer_row().to_json()
    }

    /// Delete layer (the footer).
    pub fn delete_active_layer(&mut self) -> String {
        self.session.delete_active_layer().to_json()
    }

    /// Delete on a focused row.
    pub fn delete_layer_row(&mut self, id: &str) -> String {
        self.session.delete_layer_row(id).to_json()
    }

    /// Why a drop would be refused, or empty.
    #[must_use]
    pub fn check_layer_drop(&self, pressed: &str, target: &str, place: &str) -> String {
        self.session.check_layer_drop(pressed, target, place)
    }

    /// A drop: `place` is `"above"`, `"below"` or `"into"`.
    pub fn drop_layer_rows(&mut self, pressed: &str, target: &str, place: &str) -> String {
        self.session
            .drop_layer_rows(pressed, target, place)
            .to_json()
    }

    /// Alt+arrow keys on a row: `how` is `"front"`, `"back"`, `"forward"`, `"backward"`,
    /// `"out"` or `"in"`.
    pub fn reorder_layer_rows(&mut self, focused: &str, how: &str) -> String {
        match reorder_from(how) {
            Some(how) => self.session.reorder_layer_rows(focused, how).to_json(),
            None => PanelReply::Quiet.to_json(),
        }
    }

    /// The layer notice for the canvas notice slot (drawing into a hidden or locked layer, a
    /// refused stacking-order key), once; empty when there is none.
    pub fn take_layer_notice(&mut self) -> String {
        self.session.take_layer_notice()
    }
}
