//! The copy key of a parameter handle drag (`specs/0073-parameter-handle-clone`, Frage 387
//! answered: Shift): Shift held during the drag makes it write a copy, and Shift's state at the
//! release decides. A child module of `select_tool.rs`, so it reads `SelectTool`'s private drag state.

use curvyo_document_core::{Document, ObjectSnapshot};

use super::{SelectDrag, SelectTool};
use crate::object_selection::ObjectSelection;
use crate::param_edit::commit_param_copy;
use crate::transform_drag::TransformDrag;

impl SelectTool {
    /// The live state of Shift for the drag in flight (criterion 1): held, not toggled. Call it
    /// with every pointer and key event's Shift flag; a parameter drag is a copy drag while it is
    /// set. A Shift held from the press counts only after it was let go (it chose one corner
    /// there). Every other gesture ignores it here (its own Shift rules read the press or the
    /// event).
    pub fn set_copy_key(&mut self, shift: bool) {
        if let SelectDrag::Transforming(drag) = &mut self.drag {
            drag.note_copy_key(shift);
        }
    }

    /// Whether a parameter drag in flight would write a copy if released now (criterion 5):
    /// the Copy badge and the readout's " · copy" follow it. `false` before the press and for
    /// every other drag.
    #[must_use]
    pub fn param_copy_in_flight(&self) -> bool {
        matches!(&self.drag, SelectDrag::Transforming(drag) if drag.param_copy().is_some())
    }

    /// Writes the released copy drag's `result` to a copy of the original and selects the copy
    /// (criteria 4, 12). `false` when the drag is not a copy drag: the caller then writes the
    /// change. The caller has already checked the dead zone and that the result differs.
    pub(super) fn finish_param_copy(
        drag: &TransformDrag,
        document: &Document,
        selection: &mut ObjectSelection,
        result: &ObjectSnapshot,
    ) -> bool {
        let Some(param) = drag.param_copy() else {
            return false;
        };
        if let Some(copy) = commit_param_copy(document, param, result) {
            selection.set(&[copy]);
        }
        true
    }
}
