//! `Session`'s glue for the arrange groups of the Select bar
//! (`specs/0061-align-and-distribute/`): the units and the reference of
//! every press. The plans and the commit are `curvyo_ui_core`'s; the key
//! object and the page cue are view state on the session.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use curvyo_document_core::{Length, NodeId, ObjectSnapshot, Point};
use curvyo_ui_core::{
    AlignAxis, AlignBox, AlignCommand, AlignInput, AlignOutcome, AlignReference, DistributeMode,
    Modifiers, PressTarget, SelectedUnit, arrange_selection_in_grid, distribute_page,
    distribute_selection, page_object_ids_with_artwork,
};

use super::group_view::merged_corners;
use super::{Session, Tool};

/// The arrange press whose result is checked before it is written.
#[derive(Debug, Clone, Copy)]
enum ArrangeAction {
    Align(AlignCommand),
    Distribute(AlignAxis, DistributeMode, Option<f64>),
    Grid(u32, u32, f64),
}

/// The align command of a wasm `code` (`"left"`, `"centre-h"`, `"right"`,
/// `"top"`, `"centre-v"`, `"bottom"`); `None` for any other code, which then
/// runs nothing.
#[must_use]
pub fn align_command_from_code(code: &str) -> Option<AlignCommand> {
    Some(match code {
        "left" => AlignCommand::Left,
        "centre-h" => AlignCommand::CentreH,
        "right" => AlignCommand::Right,
        "top" => AlignCommand::Top,
        "centre-v" => AlignCommand::CentreV,
        "bottom" => AlignCommand::Bottom,
        _ => return None,
    })
}

/// The axis and mode of a wasm distribute `code` (`"h-gaps"`,
/// `"h-centres"`, `"v-gaps"`, `"v-centres"`); `None` for any other code,
/// which then runs nothing.
#[must_use]
pub fn distribute_from_code(code: &str) -> Option<(AlignAxis, DistributeMode)> {
    Some(match code {
        "h-gaps" => (AlignAxis::Horizontal, DistributeMode::Gaps),
        "h-centres" => (AlignAxis::Horizontal, DistributeMode::Centres),
        "v-gaps" => (AlignAxis::Vertical, DistributeMode::Gaps),
        "v-centres" => (AlignAxis::Vertical, DistributeMode::Centres),
        _ => return None,
    })
}

impl Session {
    /// Whether the Align and Distribute groups are in the Select bar
    /// (criteria 1, 15, 16): in every selection state of the Select tool,
    /// the empty page included.
    #[must_use]
    pub fn align_group_shown(&self) -> bool {
        self.tool == Tool::Select
    }

    /// The number of selected units the Arrange in a grid group works on
    /// (criteria 18, 19): the group is shown from two.
    #[must_use]
    pub fn grid_count(&self) -> usize {
        if self.tool != Tool::Select {
            return 0;
        }
        self.units().selected(self.selection.ids()).len()
    }

    /// A number that changes when the selected units change: the grid fields
    /// keep their values until then (criterion 21).
    #[must_use]
    pub fn grid_selection_key(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        for unit in self.units().selected(self.selection.ids()) {
            unit.id.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// What an align press acts on now: the page's objects with nothing
    /// selected, the page with one selected unit, else the selection.
    #[must_use]
    pub fn align_reference(&self) -> AlignReference {
        curvyo_ui_core::align_reference(self.units().selected(self.selection.ids()).len())
    }

    /// What a distribute press acts on now: the page's objects with nothing
    /// selected, the page with one or two selected units, else the selection
    /// (criterion 16).
    #[must_use]
    pub fn distribute_reference(&self) -> AlignReference {
        curvyo_ui_core::distribute_reference(self.units().selected(self.selection.ids()).len())
    }

    /// The units of the active page's pickable objects, the set a page-wise
    /// align or distribute acts on when nothing is selected. Groups come in
    /// whole; hidden and locked objects are not pickable and never move.
    pub(super) fn page_units(&self) -> Vec<SelectedUnit> {
        let pickable = self.pickable_objects();
        let ids = page_object_ids_with_artwork(&pickable, &self.active_page_box(), &|path| {
            self.width_artwork(path)
        });
        self.units().selected(&ids)
    }

    /// The active page's rectangle, the reference of a page-wise align or
    /// distribute (criteria 14 to 17).
    pub(super) fn active_page_box(&self) -> AlignBox {
        let page = self.active_page();
        let (x, y) = (page.origin.x, page.origin.y);
        AlignBox::new(
            Point::new(x, y),
            Point::new(x + page.size.width.as_mm(), y + page.size.height.as_mm()),
        )
    }

    /// The units a page-wise press acts on for `reference`: the page's
    /// objects with nothing selected, else the selected units.
    fn press_units(&self, reference: AlignReference) -> Vec<SelectedUnit> {
        match reference {
            AlignReference::PageObjects => self.page_units(),
            AlignReference::Page | AlignReference::Selection => {
                self.units().selected(self.selection.ids())
            }
        }
    }

    /// Whether a bar command may run now: the Select tool with no drag in
    /// flight; an open chip closes first, as with a flip.
    fn ready_for_bar_command(&mut self) -> bool {
        if self.tool != Tool::Select || self.select.drag_in_flight() {
            return false;
        }
        self.select.cancel_entry();
        self.sync_select_units();
        let units = self.units().selected(self.selection.ids());
        let leaves: std::collections::HashSet<_> = units
            .iter()
            .flat_map(|unit| unit.leaves.iter().copied())
            .collect();
        if self
            .spiro_selection_objects(&self.objects())
            .iter()
            .any(|object| {
                (leaves.is_empty() || leaves.contains(&object.id()))
                    && object.drawn_path().is_some_and(|path| {
                        path.curve_kind == curvyo_document_core::CurveKind::Spiro
                    })
            })
        {
            self.transform_notice = Some(super::spiro_cache::SPIRO_UNSOLVED_NOTICE);
            return false;
        }
        true
    }

    fn arrange_reference(&self, action: ArrangeAction) -> AlignReference {
        match action {
            ArrangeAction::Align(_) => self.align_reference(),
            ArrangeAction::Distribute(..) => self.distribute_reference(),
            ArrangeAction::Grid(..) => AlignReference::Selection,
        }
    }

    fn arrange_candidates(&self, action: ArrangeAction) -> Option<Vec<ObjectSnapshot>> {
        let objects = self.spiro_selection_objects(&self.objects());
        let reference = self.arrange_reference(action);
        let selected = self.press_units(reference);
        let boxed = curvyo_ui_core::unit_boxes_with_artwork(&objects, &selected, &|path| {
            self.width_artwork(path)
        });
        if boxed.units.is_empty() {
            return Some(Vec::new());
        }
        let plan = match action {
            ArrangeAction::Align(command) => {
                let reference_box = if reference.on_page() {
                    self.active_page_box()
                } else {
                    self.key_object
                        .and_then(|id| {
                            boxed
                                .units
                                .iter()
                                .position(|unit| unit.id == id)
                                .map(|index| boxed.boxes[index])
                        })
                        .or_else(|| AlignBox::union(&boxed.boxes))?
                };
                curvyo_ui_core::align_translations(&boxed.boxes, &reference_box, command)
            }
            ArrangeAction::Distribute(axis, mode, gap) => {
                let gap = gap.map(Length::from_mm);
                if reference.on_page() {
                    curvyo_ui_core::distribute_on_frame_translations(
                        &boxed.boxes,
                        &self.active_page_box(),
                        axis,
                        mode,
                        gap,
                    )
                } else {
                    curvyo_ui_core::distribute_translations(&boxed.boxes, axis, mode, gap)
                }
            }
            ArrangeAction::Grid(rows, columns, gap) => {
                let Some(plan) =
                    curvyo_ui_core::grid_translations(&boxed.boxes, rows, columns, gap)
                else {
                    return Some(Vec::new());
                };
                plan
            }
        };
        let mut candidates = Vec::new();
        for (unit, offset) in boxed.units.iter().zip(plan) {
            if offset == curvyo_document_core::Vec2::ZERO {
                continue;
            }
            for id in &unit.leaves {
                if let Some(object) = objects.iter().find(|object| object.id() == *id) {
                    candidates.push(object.translated(offset));
                }
            }
        }
        Some(candidates)
    }

    /// Whether the arrangement may be written: every moved variable stroke keeps an outline.
    fn preflight_arrange(&mut self, action: ArrangeAction) -> bool {
        let Some(candidates) = self.arrange_candidates(action) else {
            return false;
        };
        if self.width_outlines_exist(&candidates) {
            return true;
        }
        self.transform_notice = Some(super::width_transform::NO_OUTLINE_NOTICE);
        false
    }

    /// An align press: every unit's edge or centre onto the reference, one
    /// step named after the command and, relative to the page, "to page"
    /// (criteria 12, 14, 15, 17). Nothing happens outside the Select tool or
    /// while a drag runs.
    pub fn align_selection(&mut self, command: AlignCommand) -> AlignOutcome {
        if !self.ready_for_bar_command() {
            return AlignOutcome::NothingSelected;
        }
        let reference = self.align_reference();
        let on_page = reference.on_page();
        let (key, page) = (self.key_object, self.active_page_box());
        let units = self.press_units(reference);
        if !self.preflight_arrange(ArrangeAction::Align(command)) {
            return AlignOutcome::NothingSelected;
        }
        self.recording_unguarded(|this| {
            let objects = this.objects();
            let geometry = this.spiro_selection_objects(&objects);
            let all = this.units();
            let artwork = |path: &curvyo_document_core::PathSnapshot| this.width_artwork(path);
            let input = AlignInput {
                document: &this.document,
                objects: &objects,
                geometry_objects: Some(&geometry),
                units: &all,
                selection: &this.selection,
                artwork: Some(&artwork),
            };
            if on_page {
                curvyo_ui_core::align_page(&input, &units, page, command)
            } else {
                curvyo_ui_core::align_selection(&input, key, command)
            }
        })
    }

    /// A distribute press (criteria 6 to 8, 16): one step named "Distribute
    /// horizontally" or "Distribute vertically", "on page" when the page is
    /// the frame. `gap_mm` is the typed gap of criterion 8; `None` keeps the
    /// ends.
    pub fn distribute_selection(
        &mut self,
        axis: AlignAxis,
        mode: DistributeMode,
        gap_mm: Option<f64>,
    ) -> AlignOutcome {
        if !self.ready_for_bar_command() {
            return AlignOutcome::NothingSelected;
        }
        let reference = self.distribute_reference();
        let on_page = reference.on_page();
        let gap = gap_mm.map(Length::from_mm);
        let page = self.active_page_box();
        let units = self.press_units(reference);
        if !self.preflight_arrange(ArrangeAction::Distribute(axis, mode, gap_mm)) {
            return AlignOutcome::NothingSelected;
        }
        self.recording_unguarded(|this| {
            let objects = this.objects();
            let geometry = this.spiro_selection_objects(&objects);
            let all = this.units();
            let artwork = |path: &curvyo_document_core::PathSnapshot| this.width_artwork(path);
            let input = AlignInput {
                document: &this.document,
                objects: &objects,
                geometry_objects: Some(&geometry),
                units: &all,
                selection: &this.selection,
                artwork: Some(&artwork),
            };
            if on_page {
                distribute_page(&input, &units, page, axis, mode, gap)
            } else {
                distribute_selection(&input, axis, mode, gap)
            }
        })
    }

    /// An Arrange in a grid press (criteria 20, 21): the selected units in
    /// reading order into `rows` x `columns` cells with `gap_mm` between
    /// them, one step "Arrange in a grid"; too few cells write nothing.
    pub fn arrange_grid(&mut self, rows: u32, columns: u32, gap_mm: f64) -> AlignOutcome {
        if !self.ready_for_bar_command() {
            return AlignOutcome::NothingSelected;
        }
        if !self.preflight_arrange(ArrangeAction::Grid(rows, columns, gap_mm)) {
            return AlignOutcome::NothingSelected;
        }
        self.recording_unguarded(|this| {
            let objects = this.objects();
            let geometry = this.spiro_selection_objects(&objects);
            let all = this.units();
            let artwork = |path: &curvyo_document_core::PathSnapshot| this.width_artwork(path);
            let input = AlignInput {
                document: &this.document,
                objects: &objects,
                geometry_objects: Some(&geometry),
                units: &all,
                selection: &this.selection,
                artwork: Some(&artwork),
            };
            arrange_selection_in_grid(&input, rows, columns, Length::from_mm(gap_mm))
        })
    }

    /// Shows or hides the page cue: the active page's frame in the key
    /// object's mark while a page-wise button is hovered or focused.
    pub fn set_page_cue(&mut self, on: bool) {
        self.page_cue = on;
    }

    /// The box the key mark is drawn around (criterion 5): the active page
    /// while the page cue is on (UX notes, "Canvas cue"), else the box around
    /// all leaves of the key object's unit — a group's own box.
    pub(super) fn key_mark_in(&self, objects: &[ObjectSnapshot]) -> Option<[Point; 4]> {
        if self.page_cue && self.tool == Tool::Select {
            let page = self.active_page_box();
            return Some([
                page.min,
                Point::new(page.max.x, page.min.y),
                page.max,
                Point::new(page.min.x, page.max.y),
            ]);
        }
        let unit = self.key_object?;
        let leaves = self.units().leaves_of_unit(unit);
        let corners: Vec<_> = objects
            .iter()
            .filter(|object| leaves.contains(&object.id()))
            .filter_map(|object| {
                self.cached_oriented_bounds(object)
                    .map(|bounds| bounds.document_corners())
            })
            .collect();
        merged_corners(&corners)
    }

    /// The key object's unit id, for the canvas mark (criterion 5).
    #[must_use]
    pub fn key_object(&self) -> Option<NodeId> {
        self.key_object
    }

    /// A Ctrl-press on an already selected object marks its unit as the key
    /// object, a second on the marked one clears it (criterion 5, the
    /// gesture a Proposal). The press still runs as a copy-move: only a
    /// click without movement commits nothing, so the mark costs nothing.
    pub(super) fn key_object_press(&mut self, point: Point, shift: bool) {
        let modifiers = Modifiers { shift, ..self.held };
        if !modifiers.ctrl {
            return;
        }
        let objects = self.objects();
        let pickable = self.pickable_objects();
        let tolerance = self.object_tolerance();
        let handle_tolerances = self.transform_handle_tolerances();
        if let PressTarget::Object(id) = curvyo_ui_core::classify_press_with_artwork(
            &objects,
            &pickable,
            &self.selection,
            point,
            tolerance,
            handle_tolerances,
            modifiers,
            &|path| self.width_artwork(path),
        ) {
            let units = self.units();
            let unit = units.unit_of(id);
            let already = self
                .selection
                .ids()
                .iter()
                .any(|&leaf| units.unit_of(leaf) == unit);
            if already {
                self.key_object = if self.key_object == Some(unit) {
                    None
                } else {
                    Some(unit)
                };
            }
        }
    }

    /// Drops a key object that is no longer selected, so the mark never
    /// outlives its selection.
    pub(super) fn drop_detached_key_object(&mut self) {
        if let Some(key) = self.key_object {
            let units = self.units();
            let selected = self
                .selection
                .ids()
                .iter()
                .any(|&leaf| units.unit_of(leaf) == key);
            if !selected {
                self.key_object = None;
            }
        }
    }
}
