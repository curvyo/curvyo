//! The Offset tool's center handle and entry chip in the session (`specs/0038-path-offset`
//! criteria 13, 53, 56): where the handle is, what a press, a hover or a double-click on a handle
//! does, and whether the chip is open.

use curvyo_document_core::Point;
use curvyo_ui_core::{OffsetDirection, OffsetResultChoice};

use super::{DoubleClickHint, Session};

/// The radius of the center handle's hit area, in screen pixels (`ux-rework.md` 2.1).
pub(super) const CENTER_HIT_RADIUS_PX: f64 = 12.0;

impl Session {
    /// The center handle: the center of the operand box, while the handles are drawn and no
    /// corner drag runs (criterion 53).
    pub(super) fn offset_center(&self) -> Option<Point> {
        if !self.offset_handles_shown() || self.offset.drag.is_some() {
            return None;
        }
        let corners = self.offset_box_now()?.document_corners();
        Some(Point::new(
            corners.iter().map(|c| c.x).sum::<f64>() / 4.0,
            corners.iter().map(|c| c.y).sum::<f64>() / 4.0,
        ))
    }

    /// Whether `point` is in the center handle's 12 px hit area.
    pub(super) fn offset_center_at(&self, point: Point) -> bool {
        self.offset_center().is_some_and(|center| {
            center.vector_to(point).length() * self.view().scale() <= CENTER_HIT_RADIUS_PX
        })
    }

    /// Opens the Offset entry chip (a double-click on the center handle, or O or Enter): only
    /// in the Offset tool with a closed operand selected and no drag. Returns whether it is open.
    pub fn open_offset_chip(&mut self) -> bool {
        if self.offset_center().is_none() {
            return false;
        }
        self.offset.chip_open = true;
        true
    }

    /// Closes the Offset entry chip without writing: its values stay remembered (criterion 54).
    pub fn close_offset_chip(&mut self) {
        self.offset.chip_open = false;
    }

    /// Whether the Offset entry chip is open: it closes with the tool or when the selection has no
    /// operand left.
    #[must_use]
    pub fn offset_chip_open(&self) -> bool {
        self.offset.chip_open && self.offset_center().is_some()
    }

    /// The cursor in the Offset tool: `"pointer"` over a handle or during a corner drag
    /// (`ux-rework.md` 2.1), else `"default"`.
    pub(super) fn offset_cursor_hint(&self) -> &'static str {
        if self.offset.drag.is_some() || self.offset.hovered.is_some() || self.offset.center_hovered
        {
            "pointer"
        } else {
            "default"
        }
    }

    /// The hover hint of the Offset handle under the pointer: `"offset-center"`,
    /// `"offset-corner"` (Ctrl makes a new object, the remembered Mode being Change original) or
    /// `"offset-corner-change"` (the remembered Mode is New object, so Ctrl changes the original), `""` during a drag or off the handles (`ux-rework.md` 2.1, 3).
    pub(super) fn offset_handle_hint(&self) -> &'static str {
        if self.offset.drag.is_some() || self.offset_chip_open() {
            ""
        } else if self.offset.center_hovered {
            "offset-center"
        } else if self.offset.hovered.is_none() {
            ""
        } else if self.offset.values.result() == OffsetResultChoice::NewObject {
            "offset-corner-change"
        } else {
            "offset-corner"
        }
    }

    /// A double-click in the Offset tool: on the center handle it opens the chip (criterion 53)
    /// with the modifiers of the second press preselecting it, Shift = Direction Both, Ctrl = Mode
    /// New object (criterion 57). A double-click on a corner handle does nothing (criterion 13).
    pub(super) fn offset_double_click(
        &mut self,
        point: Point,
        shift: bool,
        ctrl: bool,
    ) -> DoubleClickHint {
        if self.offset_center_at(point) && self.open_offset_chip() {
            if shift {
                self.offset.values.set_direction(OffsetDirection::Both);
            }
            if ctrl {
                self.offset.values.set_result(OffsetResultChoice::NewObject);
            }
            DoubleClickHint::OffsetChip
        } else {
            DoubleClickHint::None
        }
    }
}
