//! Stand-ins for the two routes of `specs/0101-laser-operations` that `specs/0111` replaced, so the
//! older acceptance tests keep checking the rest of their criteria through the public session:
//! the kind button of the Operations tab (every press created an operation, an empty one with
//! nothing selected; `0111` criterion 8 removed it) and the "View: Operations / Drawn" switch
//! (`0111` criterion 15 made it the page label's toggle).

#![allow(dead_code, clippy::unwrap_used)]

use curvyo_document_core::NodeId;
use curvyo_editor_wasm::Session;

/// The old view switch's two states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationsViewMode {
    /// The toggle pressed: every item by its operation look.
    Operations,
    /// The toggle unpressed: the drawing's own style.
    Drawn,
}

/// Sets the active laser page's Operations view toggle to `mode`; whether it changed.
pub fn set_view(s: &mut Session, mode: OperationsViewMode) -> bool {
    let view: serde_json::Value = serde_json::from_str(&s.operations_view_json()).unwrap();
    let pressed = view["operations_view"].as_bool().unwrap_or(true);
    if pressed == (mode == OperationsViewMode::Operations) {
        return false;
    }
    let page = s.active_page().id;
    s.toggle_operations_view(&page)
}

/// The old kind press: with something selected a Shift press (create and assign); with nothing
/// selected an operation that ends up empty (a Shift press on one item of the page, which is then
/// removed from it again, and the selection cleared). Writes two steps in the empty case.
pub fn add_operation(s: &mut Session, kind: &str) -> Option<NodeId> {
    if s.selected_object_count() > 0 {
        return s.press_operation_kind(kind, true);
    }
    if !(s.select_operation_items("not_assigned") && s.selected_object_count() > 0) {
        s.select_operation_items("running");
    }
    assert!(
        s.selected_object_count() > 0,
        "an item on the page to make the operation with"
    );
    let id = s.press_operation_kind(kind, true)?;
    assert!(s.set_selection_in_operation(id, false));
    s.escape();
    assert_eq!(s.selected_object_count(), 0);
    Some(id)
}
