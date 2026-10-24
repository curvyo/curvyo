//! What `Session` shows for the Select tool's pointer: the cursor, and the
//! hint chip of the handle under it (`specs/0005-object-transform`,
//! `specs/0008-object-transform-refinements`, `specs/0009-unified-object-editing`).
//! Split out of `session/select_view.rs`.

use curvyo_document_core::{Angle, ObjectSnapshot};
use curvyo_ui_core::{
    EditHandle, ParamHandle, PressTarget, classify_press, is_corner, oriented_bounds,
    resize_cursor_angle_degrees, skew_cursor_angle_degrees,
};

use super::{Session, Tool};

impl Session {
    /// The handle the cursor and hint describe: the one being dragged, else
    /// the one under the pointer.
    fn select_cursor_handle(&self, objects: &[ObjectSnapshot]) -> Option<EditHandle> {
        self.select.dragging_handle().or_else(|| {
            if self.selection.ids().len() >= 2 {
                return self.group_hovered_handle(objects).map(|(_, handle)| handle);
            }
            self.select_hovered_handle(objects)
                .map(|(_, _, handle)| handle)
        })
    }

    /// Which cursor the canvas should show (`specs/0005-object-transform/
    /// specification.md`'s UX notes, "Cursor feedback"; `object-transform-
    /// refinements`' "Cursors"; `unified-object-editing` criterion 5), as a
    /// plain string the host turns into CSS: `"default"` (the Select tool's
    /// normal cursor, also every other tool), `"rotate"` (the non-rotating
    /// circular arrow over — or while dragging — any of the eight rotate
    /// handles), `"resize:<degrees>"` or `"skew:<degrees>"` (a double or
    /// paired arrow rotated to that on-screen angle, clockwise from
    /// horizontal: the handle's own base angle plus the object's rotation),
    /// `"move"` (the centre handle, and anywhere a press would move the selected
    /// object), `"pointer"` (a parameter handle, hover and drag, whatever the
    /// modifiers), `"crosshair"` (a marquee is armed or running) or `"lasso"`
    /// (a lasso is armed or running, or Alt is held with no drag: the next
    /// press would arm one) or `"locked"` (a locked object is under the pointer and a press
    /// there selects nothing, `specs/0050-layers-and-objects-panel` criterion 20). A handle
    /// cursor never changes with Shift or Ctrl; the move cursor follows the press, which Shift
    /// changes.
    #[must_use]
    pub fn cursor_hint(&self) -> String {
        if self.colour_pick_target().is_some() {
            return "eyedropper".to_string();
        }
        // Pick mode and the position handle of a text on path (`0080` UX notes §2, §3.1).
        if let Some(hint) = self.text_path_cursor() {
            return hint.to_string();
        }
        if self.tool == Tool::Offset {
            return self.offset_cursor_hint().to_string();
        }
        if self.tool == Tool::Node
            && let Some(hint) = self.node_marquee_cursor()
        {
            return hint.to_string();
        }
        if self.tool != Tool::Select {
            return "default".to_string();
        }
        if let Some(hint) = self.gesture_cursor() {
            return hint.to_string();
        }
        // A mark of the open typed move is a button (`0047` criterion 43).
        if self.move_mark_hover().is_some() {
            return "pointer".to_string();
        }
        let objects = self.objects();
        // The cursor arrows follow the box the maker sees: its direction is the
        // shown angle (`orientation()`), which is the `rotation` register for
        // every kind but polygon and star.
        let rotation = match self.selection.ids() {
            [only] => objects
                .iter()
                .find(|o| o.id() == *only)
                .map_or(Angle::from_radians(0.0), |o| oriented_bounds(o).angle),
            _ => Angle::from_radians(0.0),
        };
        match self.select_cursor_handle(&objects) {
            Some(EditHandle::Rotate(_)) => "rotate".to_string(),
            Some(EditHandle::Resize(direction)) => {
                format!(
                    "resize:{:.1}",
                    resize_cursor_angle_degrees(direction, rotation)
                )
            }
            Some(EditHandle::Skew(side)) => {
                format!("skew:{:.1}", skew_cursor_angle_degrees(side, rotation))
            }
            Some(EditHandle::Move) => "move".to_string(),
            Some(EditHandle::Param(_)) => "pointer".to_string(),
            None => self.press_cursor(&objects).to_string(),
        }
    }

    /// The cursor where no handle is under the pointer: `"move"` where a plain
    /// press would move the selected object, the arrow everywhere else (where
    /// it would select an object by its outline or filled interior, start a
    /// marquee, or while a drag runs). It asks [`classify_press`], the one
    /// function the press itself acts on, so cursor and press cannot disagree
    /// (`0007` criterion 28).
    fn press_cursor(&self, objects: &[ObjectSnapshot]) -> &'static str {
        let Some(pointer) = self.pointer_position else {
            return "default";
        };
        if self.select.drag_in_flight() {
            return "default";
        }
        match classify_press(
            objects,
            &self.pickable_objects(),
            &self.selection,
            pointer,
            self.object_tolerance(),
            self.transform_handle_tolerances(),
            self.held,
        ) {
            PressTarget::InsideSelectedBox => "move",
            // The 8 px band around the outline of the sole selected object:
            // a press there selects it again and a drag moves it, as inside.
            PressTarget::Object(id) if self.selection.ids() == [id] => "move",
            // A locked object under the pointer that the press passes through (criterion 20).
            PressTarget::Empty if self.locked_at(pointer) => "locked",
            _ => "default",
        }
    }

    /// Which hint the handle under the pointer earns (criterion 54;
    /// `unified-object-editing` criterion 20), as a plain string for the
    /// host's hover chip: `""` (none), `"resize-edge"`, `"resize-corner"`,
    /// `"rotate-corner"`, `"rotate-side"`, `"skew"` (top and bottom: skew x),
    /// `"skew-y"` (left and right), `"move"`,
    /// `"param-radius"` (a rectangle's corner radius), `"param-inner"` (a
    /// star's inner radius) or `"reference-mark"` (a mark of the typed move, whose
    /// hint the move chip words). Empty while a drag runs or an entry other than the typed
    /// move is open.
    #[must_use]
    pub fn handle_hint(&self) -> String {
        if self.tool == Tool::Offset {
            return self.offset_handle_hint().to_string();
        }
        if self.tool != Tool::Select || self.colour_pick_target().is_some() {
            return String::new();
        }
        // The hint of a mark is the chip's (its second line depends on the typed text).
        if self.move_mark_hover().is_some() {
            return "reference-mark".to_string();
        }
        let objects = self.objects();
        if self.selection.ids().len() >= 2 {
            return if self.group_hovered_handle(&objects).is_some() {
                "group".to_string()
            } else {
                String::new()
            };
        }
        let Some((_, _, handle)) = self.select_hovered_handle(&objects) else {
            return String::new();
        };
        match handle {
            EditHandle::Resize(direction) if !is_corner(direction) => "resize-edge",
            EditHandle::Resize(_) => "resize-corner",
            EditHandle::Rotate(direction) if is_corner(direction) => "rotate-corner",
            EditHandle::Rotate(_) => "rotate-side",
            EditHandle::Skew(side) if side.skews_along_u() => "skew",
            EditHandle::Skew(_) => "skew-y",
            EditHandle::Move => "move",
            EditHandle::Param(ParamHandle::CornerRadius(_)) => "param-radius",
            EditHandle::Param(ParamHandle::InnerRadius) => "param-inner",
            EditHandle::Param(ParamHandle::ArcStart) => "param-arc-start",
            EditHandle::Param(ParamHandle::ArcEnd) => "param-arc-end",
            EditHandle::Param(ParamHandle::Curve) => "param-curve",
        }
        .to_string()
    }
}
