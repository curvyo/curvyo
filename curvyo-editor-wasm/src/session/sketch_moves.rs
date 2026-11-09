//! Drawing objects selected and moved while a sketch page is active (`specs/0106-move-versus-edit`
//! criteria 3, 7, 8 and 16): the Sketch Select hands a press off sketch geometry to the Select
//! tool's box, and the keys of a move to the vector key table.

use curvyo_document_core::{NodeId, Point};
use curvyo_ui_core::{Modifiers, ObjectSelection, PressTarget, classify_press};

use super::key_table::{KeyAction, decide};
use super::keys::{KeyInput, KeyOutcome};
use super::{DoubleClickHint, Session, SketchTool, Tool};

impl Session {
    /// The selected drawing objects: the selection without the page's sketch.
    fn selected_drawing_objects(&self) -> Vec<NodeId> {
        self.selection
            .ids()
            .iter()
            .copied()
            .filter(|&id| !self.document.is_fixed_sketch(id))
            .collect()
    }

    /// Whether a press at `point` reaches the selected drawing objects' box, one of its handles
    /// or a drawing object, as the Select tool reads a press.
    fn press_on_drawing(&self, point: Point) -> bool {
        // The page's sketch is no box of the Select tool (ADR 0043 §2).
        let mut drawing = ObjectSelection::new();
        drawing.set(&self.selected_drawing_objects());
        let objects = self.objects();
        !matches!(
            classify_press(
                &objects,
                &self.pickable_objects(),
                &drawing,
                point,
                self.object_tolerance(),
                self.transform_handle_tolerances(),
                Modifiers {
                    alt: false,
                    ..self.held
                },
            ),
            PressTarget::Empty | PressTarget::Lasso
        )
    }

    /// A Sketch Select press: off sketch geometry, on a drawing object or the selected drawing
    /// objects' box, the Select tool takes it (criteria 3 and 7); sketch geometry wins, and the
    /// two kinds are never selected together. `true` when the Select tool took the press.
    pub(super) fn sketch_press_on_drawing(&mut self, point: Point, shift: bool) -> bool {
        if self.sketch.tool != SketchTool::Select || self.sketch_geometry_at(point) {
            self.drop_drawing_objects_for_sketch();
            return false;
        }
        if !self.press_on_drawing(point) {
            self.drop_drawing_objects_for_sketch();
            return false;
        }
        // The other kind is replaced, with or without Shift (Proposal).
        self.sketch.entities.clear();
        self.sketch.constraints.clear();
        let drawing = self.selected_drawing_objects();
        self.selection.set(&drawing);
        self.tool = Tool::Select;
        self.button_down = true;
        self.edit_context.sketch_drawing_drag = true;
        self.select_pointer_down(point, shift);
        true
    }

    /// Sketch geometry is about to be selected: the drawing objects leave the selection.
    fn drop_drawing_objects_for_sketch(&mut self) {
        if self.selected_drawing_objects().is_empty() {
            return;
        }
        let kept: Vec<NodeId> = self
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|&id| self.document.is_fixed_sketch(id))
            .collect();
        self.selection.set(&kept);
    }

    /// A pointer move of a drag that [`Session::sketch_press_on_drawing`] started. `false` when
    /// none runs.
    pub(super) fn sketch_drawing_hover(&mut self, point: Point, modifiers: Modifiers) -> bool {
        if !self.edit_context.sketch_drawing_drag {
            return false;
        }
        self.select_hover(point, modifiers);
        true
    }

    /// The release of that drag: the Select tool commits the move, scale, turn or skew.
    pub(super) fn sketch_drawing_release(&mut self, point: Point, shift: bool, ctrl: bool) -> bool {
        if !std::mem::take(&mut self.edit_context.sketch_drawing_drag) {
            return false;
        }
        self.select_pointer_up(point, shift, ctrl);
        true
    }

    /// A key on a sketch page while drawing objects are selected: arrows, Delete, Flip, the
    /// typed entries and Escape act on them as on a drawing page; Ctrl+A goes to the sketch and
    /// drops them. `None` for every other key, which the sketch handles.
    pub(super) fn sketch_drawing_key(&mut self, input: &KeyInput<'_>) -> Option<KeyOutcome> {
        if self.selected_drawing_objects().is_empty() || input.dom_blocked {
            return None;
        }
        if input.ctrl && input.key.eq_ignore_ascii_case("a") {
            self.drop_drawing_objects_for_sketch();
            return None;
        }
        self.tool = Tool::Select;
        match decide(input, self.key_state()) {
            KeyAction::Nudge(arrow) => Some(self.nudge_selection(curvyo_ui_core::NudgeEvent {
                arrow,
                shift: input.shift,
                repeat: input.repeat,
                time_ms: input.time_ms,
            })),
            KeyAction::Delete => {
                self.delete_selected();
                Some(KeyOutcome::Deleted)
            }
            KeyAction::Flip(axis) => Some(KeyOutcome::Flipped(axis, self.flip_selection(axis))),
            KeyAction::OpenEntry(key) => {
                let objects = self.objects();
                self.select
                    .open_entry_for_key(&objects, &self.selection, key)
                    .ok()?;
                self.select.mint_conversion_ids(&mut self.minter);
                Some(KeyOutcome::EntryOpened)
            }
            KeyAction::Escape => {
                if self.select.has_entry() {
                    self.select.cancel_entry();
                } else {
                    self.drop_drawing_objects_for_sketch();
                }
                Some(KeyOutcome::Escape(super::EscapeStep::ClearedState))
            }
            _ => None,
        }
    }

    /// A double-click on a drawing object over the active sketch page (criterion 16): its page
    /// becomes active; on sketch geometry the double-click is as built.
    pub(super) fn sketch_double_click_on_drawing(
        &mut self,
        point: Point,
    ) -> Option<DoubleClickHint> {
        if self.sketch_geometry_at(point) {
            return None;
        }
        self.double_click_switch(point)
    }

    /// The selection box, handles and moved preview of drawing objects selected on a sketch
    /// page, as the Select tool draws them on a drawing page; empty without such a selection.
    pub(super) fn sketch_page_drawing_overlay(&self) -> curvyo_render_core::DrawList {
        let mut list = curvyo_render_core::DrawList::default();
        if self.selected_drawing_objects().is_empty() || self.tool != Tool::Select {
            return list;
        }
        let view = self.view();
        let objects = self.objects();
        let live = self.select_live_edit_in(&objects);
        let swap = self.drag_swap_in(&objects, live.as_ref());
        list.extend(swap.ghost(&objects, view));
        let live_objects = Self::live_objects_in(&objects, live.as_ref());
        if !swap.is_empty() {
            let moved: Vec<_> = live_objects
                .iter()
                .filter(|o| self.selection.contains(o.id()))
                .cloned()
                .collect();
            list.extend(curvyo_render_core::build_artwork_with_guides(
                &moved,
                &self.document_read().guides,
                view,
            ));
        }
        list.extend(curvyo_render_core::build_select_draw_list(
            view,
            &self.select_decoration_input_in(&live_objects),
        ));
        list.extend(self.clone_mark_layer(view, &objects, false));
        if let Some(group) = self.group_decoration_input_in(&live_objects) {
            list.extend(curvyo_render_core::build_group_draw_list(view, &group));
        }
        list.extend(curvyo_render_core::build_transform_draw_list(
            view,
            &self.select_transform_decoration_input_in(&live_objects),
        ));
        list
    }
}
