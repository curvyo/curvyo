//! The Node tool's `wasm-bindgen` surface (`specs/0002-path-node-editing`,
//! `specs/0006-path-merge-split-and-node-types`): node-kind conversion, line
//! and curve, join, split, insert and the contextual toolbar's enabled state.
//! A further `impl WasmSession` block (`wasm_api.rs` holds the struct);
//! strings and scalars only (ADR 0001 §5), every method a pass-through to
//! `Session`.

use curvyo_document_core::AnchorKind;
use curvyo_ui_core::{JoinBlock, NodeSplitOutcome, NodeToolbarState as SessionNodeToolbarState};

/// A count as JS reads it; a count past `u32::MAX` cannot occur in a real document.
fn saturating_u32(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}
use wasm_bindgen::prelude::*;

use crate::wasm_api::WasmSession;

/// `"corner" | "symmetric" | "asymmetric"`
/// (`specs/0006-path-merge-split-and-node-types/adrs.md`: "the wasm
/// binding string (`"smooth"` in `wasm_api.rs`...) is not persisted and
/// is renamed outright" — the old `"smooth"` spelling is gone here too,
/// not kept as an accepted alias the way the on-disk tag is).
fn kind_from_str(name: &str) -> Result<AnchorKind, JsValue> {
    match name {
        "corner" => Ok(AnchorKind::Corner),
        "symmetric" => Ok(AnchorKind::Symmetric),
        "asymmetric" => Ok(AnchorKind::Asymmetric),
        other => Err(JsValue::from_str(&format!("unknown anchor kind: {other}"))),
    }
}

/// `wasm-bindgen`'s JS-facing mirror of [`curvyo_ui_core::NodeToolbarState`]
/// — a plain `bool`-fields struct needs no getter methods, unlike a type
/// `wasm-bindgen` can't expose by value. See that type's own doc comment
/// for why independent `bool`s, not an enum.
#[wasm_bindgen]
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct NodeToolbarState {
    pub can_insert: bool,
    pub can_delete: bool,
    pub can_convert_to_corner: bool,
    pub can_convert_to_symmetric: bool,
    pub can_convert_to_asymmetric: bool,
    pub can_make_line: bool,
    pub can_make_curve: bool,
    pub can_join: bool,
    pub can_split: bool,
    pub clone_only: bool,
    /// Why Join is disabled (`specs/0053-node-marquee-selection/` criterion 19): 0 enabled,
    /// 1 not exactly two nodes, 2 not end nodes, 3 another reason (no reason line).
    pub join_block: u8,
    /// With `join_block` 2: how many of the selected nodes are not an end node.
    pub join_not_end: u32,
    /// Selected nodes and nodes in the editing set (criterion 23).
    pub selected_nodes: u32,
    pub total_nodes: u32,
    /// A segment, not nodes, is selected (criterion 23).
    pub segment_selected: bool,
}

impl From<SessionNodeToolbarState> for NodeToolbarState {
    fn from(state: SessionNodeToolbarState) -> Self {
        Self {
            can_insert: state.can_insert,
            can_delete: state.can_delete,
            can_convert_to_corner: state.can_convert_to_corner,
            can_convert_to_symmetric: state.can_convert_to_symmetric,
            can_convert_to_asymmetric: state.can_convert_to_asymmetric,
            can_make_line: state.can_make_line,
            can_make_curve: state.can_make_curve,
            can_join: state.can_join,
            can_split: state.can_split,
            clone_only: state.clone_only,
            join_block: match state.join_block {
                None => 0,
                Some(JoinBlock::Count(_)) => 1,
                Some(JoinBlock::NotEndNodes { .. }) => 2,
                Some(JoinBlock::Other) => 3,
            },
            join_not_end: match state.join_block {
                Some(JoinBlock::NotEndNodes { not_end, .. }) => saturating_u32(not_end),
                _ => 0,
            },
            selected_nodes: saturating_u32(state.selected_nodes),
            total_nodes: saturating_u32(state.total_nodes),
            segment_selected: state.segment_selected,
        }
    }
}

#[wasm_bindgen]
impl WasmSession {
    /// The node-kind conversion actions: `"corner"`, `"symmetric"` or
    /// `"asymmetric"` (`specs/0006-path-merge-split-and-node-types/
    /// specification.md` acceptance criteria 1, 2, 4, 5, 16).
    ///
    /// # Errors
    /// A `JsValue` if `kind` is none of those.
    pub fn convert_selected(&mut self, kind: &str) -> Result<(), JsValue> {
        self.session.convert_selected(kind_from_str(kind)?);
        Ok(())
    }

    /// Acceptance criterion 14's "make line".
    pub fn make_line(&mut self) {
        self.session.make_line();
    }

    /// Acceptance criterion 14's "make curve".
    pub fn make_curve(&mut self) {
        self.session.make_curve();
    }

    /// Acceptance criteria 8-11: the "Join" toolbar/context-menu button. Returns how many
    /// outlines moved into the surviving object besides the joined one (0 when none, or nothing
    /// was joined), for the Join notice (`specs/0055-compound-path-node-editing` criterion 5 c).
    pub fn join_selected(&mut self) -> u32 {
        saturating_u32(self.session.join_selected().unwrap_or(0))
    }

    /// Acceptance criteria 12-15: the "Split" toolbar/context-menu
    /// button. Returns `""` (nothing split), `"path"`, `"outline_in_two"` (an open outline of a
    /// compound path became two of it) or `"outline_opened"` (a closed outline of a compound path
    /// was opened), for the Split notice (`0055` criterion 6).
    pub fn split_selected(&mut self) -> String {
        match self.session.split_selected_outcome() {
            NodeSplitOutcome::None => "",
            NodeSplitOutcome::Path => "path",
            NodeSplitOutcome::OutlineInTwo => "outline_in_two",
            NodeSplitOutcome::OutlineOpened => "outline_opened",
        }
        .to_string()
    }

    /// The contextual toolbar's "Insert node" button: splits the
    /// currently selected segment at its midpoint.
    pub fn insert_selected(&mut self) {
        self.session.insert_selected();
    }

    /// Which contextual-toolbar buttons are currently enabled
    /// (`specification.md`'s UX notes: "Buttons disable (not hide) when
    /// nothing selected/applicable").
    #[must_use]
    pub fn node_toolbar_state(&self) -> NodeToolbarState {
        self.session.node_toolbar_state().into()
    }
}
