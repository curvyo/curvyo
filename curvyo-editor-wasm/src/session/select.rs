//! `Session`'s Select-tool glue (`specs/0004-canvas-navigation-and-
//! selection/specification.md`, acceptance criteria 14-23; `specs/0005-
//! object-transform`; `specs/0008-object-transform-refinements`): dispatching
//! pointer, modifier and double-click events to [`curvyo_ui_core::
//! SelectTool`], its live move/transform preview, the stroke switch and the
//! double-click handoff. What the Select tool *shows* (decoration input,
//! cursor, hint, readout) is in `session/select_view.rs`, the typed entry
//! in `session/transform_entry.rs`. A child module of `session`, so it
//! shares `Session`'s privacy boundary and its methods join the same type's
//! `impl Session`.

use std::borrow::Cow;

use curvyo_document_core::Point;
use curvyo_ui_core::{
    CornerLinking, CornerRadiusScaling, EditHandle, Modifiers, ParamHandle,
    SelectDoubleClickOutcome, SnapProbe, StrokeScaling, TransformHandleTolerances,
};

use super::{Session, Tool};

/// What a double-click asks the host to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoubleClickHint {
    /// Nothing.
    None,
    /// The edit hint of a primitive (`unified-object-editing` criterion 32).
    EditHint,
    /// The center handle of the Offset tool opened the Offset entry chip: the host focuses its
    /// Distance field (`specs/0038-path-offset` criterion 53).
    OffsetChip,
    /// The sentence that a linked clone has no nodes (`0049-linked-clone` criterion 26).
    Clone,
    /// A double-click on a traced image in the Trace tool: the host creates the paths
    /// (`specs/0025-raster-trace` criterion 12).
    CreateTrace,
}

impl DoubleClickHint {
    /// The code the host keys its text or action on: `""`, `"edit_hint"`, `"offset_chip"`,
    /// `"clone"` or `"create_trace"`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::None => "",
            Self::EditHint => "edit_hint",
            Self::OffsetChip => "offset_chip",
            Self::Clone => "clone",
            Self::CreateTrace => "create_trace",
        }
    }
}

impl Session {
    /// The "Scale stroke width" switch (`specs/0005-object-transform/
    /// specification.md` AC 26-31): whether a Select-tool resize scales the
    /// stroke width. Off in every new session (`Session::new`/`open` build a
    /// fresh `SelectTool`); never written to the document.
    #[must_use]
    pub fn scale_stroke_width(&self) -> bool {
        self.select.stroke_scaling() == StrokeScaling::Proportional
    }

    /// Sets the switch for the *next* resize drag (AC 28: a drag in flight
    /// keeps the value it started with). Clicking the switch while a numeric
    /// entry is open closes the entry without writing (criterion 31).
    pub fn set_scale_stroke_width(&mut self, on: bool) {
        self.select.cancel_entry();
        self.select.set_stroke_scaling(if on {
            StrokeScaling::Proportional
        } else {
            StrokeScaling::Keep
        });
    }

    /// The "Scale corner radius" switch (`specs/0009-unified-object-editing/`,
    /// criterion 23): whether a Select-tool resize scales a rectangle's
    /// corner radius with it. Off in every new session; never written to the
    /// document.
    #[must_use]
    pub fn scale_corner_radius(&self) -> bool {
        self.select.corner_radius_scaling() == CornerRadiusScaling::Proportional
    }

    /// Sets the switch for the *next* resize drag or typed size (a drag in
    /// flight and an open entry keep the value they started with). Clicking
    /// the switch while a numeric entry is open closes the entry without
    /// writing (`object-transform-refinements` criterion 31).
    pub fn set_scale_corner_radius(&mut self, on: bool) {
        self.select.cancel_entry();
        self.select.set_corner_radius_scaling(if on {
            CornerRadiusScaling::Proportional
        } else {
            CornerRadiusScaling::Keep
        });
    }

    /// The "Link corners" switch (`specs/0013-rectangle-corner-radii/` criterion 2):
    /// whether a corner radius handle sets all four radii (the default) or
    /// only its own corner. On in every new session (`Session::new`/`open`
    /// build a fresh `SelectTool`, so New and Open reset it); never written
    /// to the document.
    #[must_use]
    pub fn link_corners(&self) -> bool {
        self.select.corner_linking() == CornerLinking::Linked
    }

    /// Sets the switch for the *next* corner radius drag or entry. Changes no
    /// radius and writes nothing; an open numeric entry closes without writing
    /// (criterion 8).
    pub fn set_link_corners(&mut self, on: bool) {
        self.select.set_corner_linking(if on {
            CornerLinking::Linked
        } else {
            CornerLinking::Unlinked
        });
    }

    /// Removes the "max" notice of a limited typed radius (the host calls this
    /// 1.5 s after it appeared).
    pub fn clear_limit_notice(&mut self) {
        self.limit_notice = None;
    }

    /// The Shift, Ctrl and Alt modifiers changed with no pointer movement
    /// (`adrs.md`, "Shift state"): the host calls this from window-level key
    /// events, before every pointer event with that event's modifiers, and
    /// with `(false, false, false)` when the window or canvas loses focus.
    /// The cached state is all the next frame, cursor and hint queries need —
    /// side rotate handles appear and vanish in the frame the key changes
    /// (criterion 6), the pivot marker and live preview follow (criterion 14),
    /// and so do the marquee's mode and the lasso cursor
    /// (`advanced-selection` criteria 11, 14).
    ///
    /// In the Select tool the hover is refreshed at the pointer's last position
    /// with the new modifiers, so the hover box (an Alt press arms a lasso: no
    /// object lights) and the object a press would select follow the key by
    /// themselves, whatever the host re-sends.
    pub fn modifiers_changed(&mut self, shift: bool, ctrl: bool, alt: bool) {
        let next = Modifiers::new(shift, ctrl).with_alt(alt);
        if self.workbench() == super::Workbench::Sketch && self.held == next {
            return;
        }
        self.held = next;
        if self.workbench() == super::Workbench::Sketch {
            if let Some(point) = self.pointer_position {
                self.sketch_pointer_hover(point);
            }
            return;
        }
        // An unfrozen placement follows the keys with the pointer still (`0052` criterion 3).
        if self.placement().is_some() {
            self.placement_modifiers(Modifiers::new(shift, ctrl));
        }
        if let Some(point) = self.pointer_position {
            match self.tool {
                Tool::Select => self.select_hover(point, self.held),
                Tool::Pen => self.pen_snap_hover(point),
                Tool::Rectangle | Tool::Ellipse | Tool::PolygonStar
                    if self.placement().is_none() =>
                {
                    self.shape_pointer_move(point, self.held);
                }
                _ => {}
            }
        }
    }

    /// The Select tool's handle tolerances at the current zoom
    /// (`docs/design-system.md`, "Transform handle layout"), converted from
    /// screen pixels to document millimetres.
    pub(super) fn transform_handle_tolerances(&self) -> TransformHandleTolerances {
        let mut tolerances = TransformHandleTolerances::at_scale(self.view().scale());
        // A move-only selection draws no parameter handle, so none can be pressed
        // (`specs/0106` criterion 10, UX notes §2).
        if self.edit_scope() == curvyo_ui_core::Scope::MoveOnly {
            tolerances.param_min_side_mm = f64::INFINITY;
        }
        // A whole test grid keeps its box and centre move only (`specs/0028` ux-notes 6.5).
        tolerances.move_only = self.whole_test_grid_selected();
        tolerances
    }

    /// Acceptance criteria 1, 4-18: dispatches a press to the Select
    /// tool — first against the current single-object selection's own
    /// transform handles, then (unchanged) against every object's body.
    /// An open numeric entry closes first; this press is processed as usual
    /// (criterion 20).
    pub(super) fn select_pointer_down(&mut self, point: Point, shift: bool) {
        self.limit_notice = None;
        // An object press ends a fixture selection: the two never mix (`specs/0102` crit. 8).
        self.clear_fixture_selection();
        self.sync_select_units();
        let raw_objects = self.objects();
        let objects = if raw_objects.iter().any(|object| {
            object
                .drawn_path()
                .is_some_and(|path| path.curve_kind == curvyo_document_core::CurveKind::Spiro)
        }) {
            Cow::Owned(self.spiro_pick_objects(&raw_objects))
        } else {
            Cow::Borrowed(&raw_objects[..])
        };
        let tolerance = self.object_tolerance();
        let handle_tolerances = self.transform_handle_tolerances();
        // Ctrl and Alt are the cached state (the host sends them with the
        // press); Alt decides at this press whether the drag is a lasso.
        let modifiers = Modifiers { shift, ..self.held };
        // A Ctrl-press on an already selected object marks or clears the key
        // object before the press runs (`specs/0061-align-and-distribute/`
        // criterion 5): the click itself commits nothing, so the mark is the
        // only effect a Ctrl-click has.
        self.key_object_press(point, shift);
        self.select.pointer_down_with_artwork(
            &objects,
            &mut self.selection,
            point,
            tolerance,
            handle_tolerances,
            modifiers,
            &super::width_outlines::memo_lookup(&self.width_outlines),
        );
        if !self.select.restore_drag_sources(&raw_objects) {
            self.select.escape();
            self.transform_notice = Some("The selected object changed; try again.");
            return;
        }
        // A resize that may turn shapes into paths carries the session's own
        // anchor ids from the first preview (`0019` decision 8).
        self.select.mint_conversion_ids(&mut self.minter);
        let ids = self.transform_selection_ids();
        self.select_drag_sources = raw_objects
            .iter()
            .filter(|object| ids.contains(&object.id()))
            .filter_map(|object| self.document.object(object.id()))
            .collect();
        // If a move drag began, start snapping.
        if self.select.drag_in_flight() {
            self.begin_snapping_drag_for_selection();
            // A move may drop into a fitting slot of the active laser page (`0102` crit. 25).
            if self.select.move_in_flight() {
                self.begin_fixture_fit(false);
            }
        }
        // A move may end as a cloning drop on a laser page (`specs/0100`).
        self.clone_drop_begin();
    }

    fn cancel_stale_select_sources(&mut self) -> bool {
        if self
            .select_drag_sources
            .iter()
            .any(|source| self.document.object(source.id()).as_ref() != Some(source))
        {
            self.select_drag_sources.clear();
            self.select.escape();
            self.end_snapping_drag();
            self.transform_notice =
                Some("The selection changed; the transformation was cancelled.");
            return true;
        }
        false
    }

    /// What a Select release checks before it writes: the dragged sources and Spiro paths are
    /// unchanged and solvable, and every transformed variable stroke has an outline. `None` when
    /// the release was cancelled.
    fn select_release_checks(
        &mut self,
        objects: &[curvyo_document_core::ObjectSnapshot],
    ) -> Option<()> {
        if self.cancel_stale_select_sources() {
            return None;
        }
        if self.select.stale_spiro_drag(&self.document) {
            self.select.escape();
            self.end_snapping_drag();
            self.transform_notice =
                Some("The Spiro path changed during the drag; the transformation was cancelled.");
            return None;
        }
        // A Spiro without solution may be clicked, but a transform of it is refused.
        if self.selection_contains_spiro()
            && let Some(live) = self.select_live_edit_in(objects)
            && self
                .spiro_selection_objects(&live.objects)
                .iter()
                .any(|object| {
                    object.drawn_path().is_some_and(|path| {
                        path.curve_kind == curvyo_document_core::CurveKind::Spiro
                    }) && !objects.contains(object)
                })
        {
            self.select.escape();
            self.end_snapping_drag();
            self.transform_notice = Some(super::spiro_cache::SPIRO_UNSOLVED_NOTICE);
            return None;
        }
        if self.selection_has_width_profile() && self.select_drag_deforms() {
            self.select.escape();
            self.select_drag_sources.clear();
            self.end_snapping_drag();
            self.transform_notice = Some(super::width_transform::UNEVEN_WIDTH_NOTICE);
            return None;
        }
        if let Some(live) = self.select_live_edit_in(objects) {
            let candidates = self.spiro_selection_objects(&live.objects);
            let selected_ids = self.transform_selection_ids();
            let changed: Vec<_> = candidates
                .into_iter()
                .filter(|object| selected_ids.contains(&object.id()))
                .collect();
            if !self.width_outlines_exist(&changed) {
                self.select.escape();
                self.select_drag_sources.clear();
                self.end_snapping_drag();
                self.transform_notice = Some(super::width_transform::NO_OUTLINE_NOTICE);
                return None;
            }
        }
        Some(())
    }

    /// Acceptance criteria 3, 15-18, 20, 41: commits whatever move/resize/
    /// rotate/skew drag [`Session::select_pointer_down`] began. `shift`/
    /// `ctrl` are the modifiers' state at release.
    pub(super) fn select_pointer_up(&mut self, point: Point, shift: bool, ctrl: bool) {
        self.pointer_position = Some(point);
        self.held.shift = shift;
        self.held.ctrl = ctrl;
        self.sync_select_units();
        let objects = self.objects();
        if self.select_release_checks(&objects).is_none() {
            return;
        }
        // A resize released through zero is announced with its mirror (`0056` UX notes).
        let mirror = self
            .select
            .live_resize_factors(point, shift, ctrl)
            .and_then(|(sx, sy)| curvyo_ui_core::mirror_line(sx, sy));
        let dragged = self.select.dragging_handle();
        let snap_offset = self
            .snap_offset_for_move()
            .unwrap_or(curvyo_document_core::Vec2::ZERO);
        // Over a laser page the release decides by its own flags (`specs/0100` criteria 1 to 4);
        // a click that never left the dead zone is no drop (a Shift-click toggles,
        // `specs/0106` criterion 3).
        if self.select.live_move(point, shift, ctrl).is_some()
            && self.clone_drop_release(point, snap_offset)
        {
            self.end_snapping_drag();
            return;
        }
        // A move that fits a slot lands in it, turned when needed (`specs/0102` criterion 25).
        if self.release_fixture_fit() {
            self.clone_drop_cancel();
            self.end_snapping_drag();
            return;
        }
        self.end_fixture_fit();
        let target = self.clone_drop_target();
        // Ctrl with Alt over a laser page is a copy (criterion 4).
        let alt = self.held.alt && self.clone_drop_mode() != super::DropMode::Copy;
        self.clone_drop_cancel();
        let recovery = self.select.pointer_up_snapped_with_artwork(
            &self.document,
            &objects,
            &mut self.selection,
            point,
            Modifiers::new(shift, ctrl).with_alt(alt),
            &mut self.minter,
            snap_offset,
            &super::width_outlines::memo_lookup(&self.width_outlines),
        );
        self.select_drag_sources.clear();
        self.end_snapping_drag();
        if let Some(page) = target {
            self.activate_page(&page);
        }
        // A release that set an axis to 0.01 mm says so (`0056` criterion 15).
        if let Some(notice) = curvyo_ui_core::zero_recovery_notice(recovery) {
            self.transform_notice = Some(notice);
        }
        if self.has_unrecorded_steps() {
            self.transform_announcement = mirror
                .map(resized_announcement)
                .or_else(|| dragged.and_then(|handle| self.arc_or_curve_announcement(handle)));
        }
    }

    /// The announcement of the selected primitive's arc or curve after an edit through
    /// `handle` (`specs/0021-ellipse-arcs-and-shaping/` criterion 65); `None` for any other
    /// handle.
    pub(super) fn arc_or_curve_announcement(&self, handle: EditHandle) -> Option<String> {
        let EditHandle::Param(param) = handle else {
            return None;
        };
        let id = *self.selection.ids().first()?;
        let shape = self.document.primitive(id)?.shape;
        match param {
            ParamHandle::ArcStart | ParamHandle::ArcEnd => curvyo_ui_core::arc_announcement(&shape),
            ParamHandle::Curve => curvyo_ui_core::curve_announcement(&shape),
            ParamHandle::CornerRadius(_) | ParamHandle::InnerRadius => None,
        }
    }

    /// The polite announcement of the last resize that mirrored, by a release or an Enter
    /// ("Resized. Mirrored left to right."), once; an empty string when there is none
    /// (`specs/0056-mirror-and-negative-scale/` UX notes, `review-ux.md` finding 3).
    pub fn take_transform_announcement(&mut self) -> String {
        self.transform_announcement.take().unwrap_or_default()
    }

    /// The notice of the last release or typed size that set an axis to 0.01 mm instead
    /// of 0 (`specs/0056-mirror-and-negative-scale/` criterion 15), once: an empty string
    /// when there is none. The host shows it in the canvas notice slot for 3 s.
    pub fn take_transform_notice(&mut self) -> String {
        self.transform_notice.take().unwrap_or_default().to_string()
    }

    /// Updates the Select tool's own hover state (UX notes: "Hover, tool
    /// not yet clicked into selection: same box at `--accent-hover`") and
    /// lets a drag in flight note the pointer (the dead zone, once left,
    /// stays left).
    pub(super) fn select_hover(&mut self, point: Point, modifiers: Modifiers) {
        self.sync_select_units();
        self.select
            .pointer_moved(point, modifiers, &mut self.selection);
        self.snap_offset_for_move();
        if self.select.drag_in_flight() {
            self.clone_drop_update(point);
        }
        let raw_objects = self.objects();
        let objects = if raw_objects.iter().any(|object| {
            object
                .drawn_path()
                .is_some_and(|path| path.curve_kind == curvyo_document_core::CurveKind::Spiro)
        }) {
            Cow::Owned(self.spiro_pick_objects(&raw_objects))
        } else {
            Cow::Borrowed(&raw_objects[..])
        };
        // Nothing lights up while a drag runs; hover returns after release.
        // Otherwise hover lights the object a press at this point would
        // select, and nothing where a press would grab a handle, move the
        // selection or start a marquee (`0007` criterion 28).
        // (An Alt press arms a lasso, `PressTarget::Lasso`, so nothing lights.)
        self.hovered_object = if self.select.drag_in_flight() {
            None
        } else {
            match curvyo_ui_core::classify_press_with_artwork(
                &objects,
                &self.pickable_objects(),
                &self.selection,
                point,
                self.object_tolerance(),
                self.transform_handle_tolerances(),
                modifiers,
                &|path| self.width_artwork(path),
            ) {
                curvyo_ui_core::PressTarget::Object(id) => Some(id),
                _ => None,
            }
        };
    }

    /// `unified-object-editing` criteria 31 to 34, and 3, 18, 25-32, 49 of the
    /// refinements: a double-click while the Select tool is active, at the
    /// second press's position with its modifiers. On a handle with a typed
    /// entry (rotate, resize, skew, parameter) and on the drawn centre handle
    /// (the typed move, `edit-interaction-polish` criterion 15) it opens the
    /// entry; on empty canvas nothing happens; on a path (outline, or inside
    /// its selected box away from the centre handle) the Node tool is
    /// activated with the path selected, "ready for node editing with no
    /// nodes selected" (`adrs.md`); on a primitive nothing changes and the
    /// host is asked to show the edit hint (the returned `true`). No
    /// double-click switches to a primitive's own tool: there is none. On a compound path
    /// the tool does not change either, and the host is told to show the sentence of criterion
    /// 38 of `0016-boolean-operations`.
    pub(super) fn select_double_click(
        &mut self,
        point: Point,
        shift: bool,
        ctrl: bool,
    ) -> DoubleClickHint {
        self.sync_select_units();
        let objects = self.objects();
        let tolerance = self.object_tolerance();
        let handle_tolerances = self.transform_handle_tolerances();
        let outcome = self.select.double_click(
            &objects,
            &self.selection,
            point,
            tolerance,
            handle_tolerances,
            (shift, ctrl),
        );
        self.select.mint_conversion_ids(&mut self.minter);
        match outcome {
            SelectDoubleClickOutcome::Hit(_) => {
                self.node.cancel_drag();
                self.node.clear_selection();
                self.tool = Tool::Node;
                DoubleClickHint::None
            }
            SelectDoubleClickOutcome::EditHint => DoubleClickHint::EditHint,
            // A clone of a page's sketch opens its sketch page (`specs/0099` criterion 13).
            SelectDoubleClickOutcome::Clone => {
                let hit = curvyo_ui_core::hit_test_objects(&objects, point, tolerance);
                if self.open_sketch_page_of(&hit) {
                    DoubleClickHint::None
                } else {
                    DoubleClickHint::Clone
                }
            }
            // A text opens for editing in the Text tool (`specs/0074-text-objects` criterion 10).
            SelectDoubleClickOutcome::Text(id) => {
                self.edit_text_at(id, Some(point));
                DoubleClickHint::None
            }
            // A group opens: everything outside it recedes (`0023` criterion 15).
            SelectDoubleClickOutcome::EnterGroup { group, leaf } => {
                self.enter_group(group, Some(leaf));
                DoubleClickHint::None
            }
            // Empty canvas inside a group leaves it by one level (criterion 18).
            SelectDoubleClickOutcome::Miss => {
                self.leave_group(false);
                DoubleClickHint::None
            }
            SelectDoubleClickOutcome::EntryOpened | SelectDoubleClickOutcome::NoNodes => {
                DoubleClickHint::None
            }
        }
    }

    /// The snap offset for the current move drag, if any.
    pub(super) fn snap_offset_for_move(&mut self) -> Option<curvyo_document_core::Vec2> {
        let pointer = self.pointer_position?;
        let modifiers = Modifiers::new(self.held.shift, self.held.ctrl);
        let Some((min, max)) =
            self.select
                .live_move_box_snapped(pointer, modifiers, curvyo_document_core::Vec2::ZERO)
        else {
            let (point, direction) =
                self.select
                    .resize_snap_point(pointer, self.held.shift, self.held.ctrl)?;
            return Some(
                self.resolve_snapping_direction(&SnapProbe::point(point), direction)
                    .offset,
            );
        };
        let probe = SnapProbe::box_edges(min, max);
        let live = self
            .select
            .live_move(pointer, self.held.shift, self.held.ctrl);
        let direction = live
            .as_ref()
            .and_then(|live| live.axis)
            .map(|axis| match axis {
                curvyo_ui_core::Axis::X => curvyo_document_core::Vec2::new(1.0, 0.0),
                curvyo_ui_core::Axis::Y => curvyo_document_core::Vec2::new(0.0, 1.0),
            });
        let offset = live.map_or(curvyo_document_core::Vec2::ZERO, |live| live.offset);
        let grid_points = self.move_grid_points(min, max, offset);
        let result = self.resolve_snapping_grid(&probe, &grid_points, direction);
        // A fit to a slot wins over every other snap (`specs/0102` criterion 25).
        if let Some(fit) = self.fixture_fit(min, max) {
            return Some(fit);
        }
        if result.is_empty() {
            None
        } else {
            Some(result.offset)
        }
    }
}

impl Session {
    /// The points of a moved selection that snap to a grid (`specs/0090-grid` criterion 11):
    /// the box centre, the nine reference points of the box, or the moving nodes (`offset` is the
    /// live move since the press).
    fn move_grid_points(
        &self,
        min: curvyo_document_core::Point,
        max: curvyo_document_core::Point,
        offset: curvyo_document_core::Vec2,
    ) -> Vec<curvyo_document_core::Point> {
        use curvyo_document_core::{GridSnapPoint, Point};
        let xs = [min.x, f64::midpoint(min.x, max.x), max.x];
        let ys = [min.y, f64::midpoint(min.y, max.y), max.y];
        match self.document.snapping().grid_point {
            GridSnapPoint::Centre => vec![Point::new(xs[1], ys[1])],
            GridSnapPoint::Box => xs
                .iter()
                .flat_map(|&x| ys.iter().map(move |&y| Point::new(x, y)))
                .collect(),
            GridSnapPoint::Nodes => self
                .snapping
                .grid_nodes()
                .iter()
                .map(|p| p.translated(offset))
                .collect(),
        }
    }
}

/// "Resized. " and the mirror `line` of a resize that turned an axis over.
pub(super) fn resized_announcement(line: &str) -> String {
    format!("Resized. {line}.")
}
