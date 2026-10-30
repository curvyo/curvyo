//! `Session`'s glue for the align and distribute groups of the Select bar
//! (`specs/0061-align-and-distribute/`): when the two groups show, the
//! commands themselves, and the key object's mark (criterion 5). The plan
//! and the commit are `curvyo_ui_core::align_selection` and
//! `distribute_selection`; the key object is view state on the session.

use curvyo_document_core::{NodeId, Point};
use curvyo_ui_core::{
    AlignAxis, AlignBox, AlignCommand, AlignInput, AlignOutcome, DistributeMode, Modifiers,
    PressTarget, classify_press, distribute_selection,
};

use super::{Session, Tool};

/// The commit label of `command` (`specs/0061-align-and-distribute/`
/// criterion 12), named for the maker in `history_names.rs`.
const fn align_label(command: AlignCommand) -> &'static str {
    match command {
        AlignCommand::Left => "align_left",
        AlignCommand::CentreH => "align_centre_h",
        AlignCommand::Right => "align_right",
        AlignCommand::Top => "align_top",
        AlignCommand::CentreV => "align_centre_v",
        AlignCommand::Bottom => "align_bottom",
    }
}

/// The commit label of a distribute on `axis` (criterion 12: per axis, not
/// per mode).
const fn distribute_label(axis: AlignAxis) -> &'static str {
    match axis {
        AlignAxis::Horizontal => "distribute_h",
        AlignAxis::Vertical => "distribute_v",
    }
}

impl Session {
    /// Whether the Align group is in the Select bar (criterion 1): the Select
    /// tool with something selected.
    #[must_use]
    pub fn align_group_shown(&self) -> bool {
        self.tool == Tool::Select && !self.selection.ids().is_empty()
    }

    /// Whether the Distribute group is in the Select bar (criterion 1): the
    /// Select tool with two or more selected objects. A group counts once.
    #[must_use]
    pub fn distribute_group_shown(&self) -> bool {
        self.tool == Tool::Select && self.units().selected(self.selection.ids()).len() >= 2
    }

    /// The active page's rectangle, the reference of a one-object align
    /// (criterion 4).
    fn active_page_box(&self) -> AlignBox {
        let page = self.active_page();
        let (x, y) = (page.origin.x, page.origin.y);
        AlignBox::new(
            Point::new(x, y),
            Point::new(x + page.size.width.as_mm(), y + page.size.height.as_mm()),
        )
    }

    /// An align press: every selected unit's edge or centre onto the
    /// reference, one step named after the command (criterion 12). An open
    /// chip closes first, as with a flip. Nothing happens outside the Select
    /// tool or while a drag runs.
    pub fn align_selection(&mut self, command: AlignCommand) -> AlignOutcome {
        if self.tool != Tool::Select || self.select.drag_in_flight() {
            return AlignOutcome::NothingSelected;
        }
        self.select.cancel_entry();
        self.sync_select_units();
        let label = align_label(command);
        let (key, page) = (self.key_object, self.active_page_box());
        self.recording(|this| {
            let objects = this.objects();
            let units = this.units();
            let input = AlignInput {
                document: &this.document,
                objects: &objects,
                units: &units,
                selection: &this.selection,
            };
            curvyo_ui_core::align_selection(&input, key, page, command, label)
        })
    }

    /// A distribute press (criteria 6 to 8): one step named "Distribute
    /// horizontally" or "Distribute vertically". `gap_mm` is the typed gap of
    /// criterion 8; `None` keeps the ends.
    pub fn distribute_selection(
        &mut self,
        axis: AlignAxis,
        mode: DistributeMode,
        gap_mm: Option<f64>,
    ) -> AlignOutcome {
        if self.tool != Tool::Select || self.select.drag_in_flight() {
            return AlignOutcome::NothingSelected;
        }
        self.select.cancel_entry();
        self.sync_select_units();
        let label = distribute_label(axis);
        self.recording(|this| {
            let objects = this.objects();
            let units = this.units();
            let input = AlignInput {
                document: &this.document,
                objects: &objects,
                units: &units,
                selection: &this.selection,
            };
            distribute_selection(&input, axis, mode, gap_mm, label)
        })
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
        if let PressTarget::Object(id) = classify_press(
            &objects,
            &pickable,
            &self.selection,
            point,
            tolerance,
            handle_tolerances,
            modifiers,
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

#[cfg(test)]
mod tests {
    use curvyo_document_core::{ObjectSnapshot, Point};
    use curvyo_ui_core::{AlignCommand, AlignOutcome, Modifiers, object_outline_bounds};

    use super::super::{Session, Tool};

    /// The left edge of the `at`-th object of the context.
    fn left_x(session: &Session, at: usize) -> f64 {
        let object = session.objects()[at].clone();
        object_outline_bounds(&object).0.x
    }

    /// One rectangle by its drag.
    fn draw_rect(session: &mut Session, from: (f64, f64), to: (f64, f64)) {
        session.set_tool(Tool::Rectangle);
        session.pointer_down(Point::new(from.0, from.1), false);
        session.pointer_up(Point::new(to.0, to.1), false, false);
    }

    /// Select every object of the context.
    fn select_all(session: &mut Session) {
        session.set_tool(Tool::Select);
        let ids: Vec<_> = session.objects().iter().map(ObjectSnapshot::id).collect();
        session.selection.clear();
        for id in ids {
            session.selection.add(id);
        }
    }

    /// Two rectangles, both selected, Select tool active.
    fn two_rects(session: &mut Session) {
        draw_rect(session, (0.0, 0.0), (30.0, 10.0));
        draw_rect(session, (50.0, 0.0), (70.0, 10.0));
        select_all(session);
    }

    #[test]
    fn the_groups_follow_the_selection_and_tool() {
        let mut session = Session::new(1);
        assert!(!session.align_group_shown());
        assert!(!session.distribute_group_shown());
        two_rects(&mut session);
        assert!(session.align_group_shown());
        assert!(session.distribute_group_shown());
        session.set_tool(Tool::Rectangle);
        assert!(!session.align_group_shown());
        assert!(!session.distribute_group_shown());
    }

    #[test]
    fn an_align_is_one_step_and_moves_onto_the_reference() {
        let mut session = Session::new(1);
        two_rects(&mut session);
        assert_eq!(
            session.align_selection(AlignCommand::Left),
            AlignOutcome::Aligned
        );
        assert_eq!(left_x(&session, 1), 0.0);
        // One step: one undo puts both rectangles back.
        assert!(session.undo().changed);
        assert_eq!(left_x(&session, 1), 50.0);
    }

    #[test]
    fn an_alignment_that_changes_nothing_writes_nothing() {
        let mut session = Session::new(1);
        draw_rect(&mut session, (0.0, 0.0), (30.0, 10.0));
        draw_rect(&mut session, (0.0, 20.0), (30.0, 30.0));
        select_all(&mut session);
        assert_eq!(
            session.align_selection(AlignCommand::Left),
            AlignOutcome::Unchanged
        );
        // Nothing was written: the undo takes back the last Draw rectangle.
        assert!(session.undo().changed);
        assert_eq!(session.objects().len(), 1);
    }

    #[test]
    fn one_object_aligns_onto_the_active_page() {
        let mut session = Session::new(1);
        draw_rect(&mut session, (40.0, 30.0), (50.0, 44.0));
        select_all(&mut session);
        assert_eq!(
            session.align_selection(AlignCommand::CentreH),
            AlignOutcome::Aligned
        );
        let object = session.objects()[0].clone();
        let (min, max) = object_outline_bounds(&object);
        assert_eq!(f64::midpoint(min.x, max.x), 105.0);
    }

    #[test]
    fn a_ctrl_click_marks_the_key_and_the_key_is_the_reference() {
        let mut session = Session::new(1);
        two_rects(&mut session);
        // A Ctrl-press on the second marks it as the key (criterion 5).
        session.held = Modifiers::new(false, true);
        session.pointer_down(Point::new(50.0, 5.0), false);
        session.pointer_up(Point::new(50.0, 5.0), false, true);
        session.held = Modifiers::NONE;
        eprintln!("key: {:?}", session.key_object().is_some());
        eprintln!(
            "n={} lefts={:?}",
            session.objects().len(),
            (0..session.objects().len())
                .map(|i| left_x(&session, i))
                .collect::<Vec<_>>()
        );
        assert!(session.key_object().is_some());
        // Align left: the key keeps its place, the other lands on it.
        assert_eq!(
            session.align_selection(AlignCommand::Left),
            AlignOutcome::Aligned
        );
        assert_eq!(left_x(&session, 0), 50.0);
        assert_eq!(left_x(&session, 1), 50.0);
    }

    #[test]
    fn escape_clears_the_key_before_the_selection() {
        let mut session = Session::new(1);
        two_rects(&mut session);
        session.held = Modifiers::new(false, true);
        session.pointer_down(Point::new(50.0, 5.0), false);
        session.pointer_up(Point::new(50.0, 5.0), false, true);
        session.held = Modifiers::NONE;
        eprintln!("key: {:?}", session.key_object().is_some());
        eprintln!(
            "n={} lefts={:?}",
            session.objects().len(),
            (0..session.objects().len())
                .map(|i| left_x(&session, i))
                .collect::<Vec<_>>()
        );
        assert!(session.key_object().is_some());
        session.escape();
        assert!(session.key_object().is_none());
        assert!(!session.selection.is_empty());
        session.escape();
        assert!(session.selection.is_empty());
    }

    #[test]
    fn a_distribute_spaces_the_centres_and_is_one_step() {
        let mut session = Session::new(1);
        draw_rect(&mut session, (0.0, 0.0), (10.0, 10.0));
        draw_rect(&mut session, (16.0, 0.0), (20.0, 10.0));
        draw_rect(&mut session, (40.0, 0.0), (50.0, 10.0));
        select_all(&mut session);
        assert_eq!(
            session.distribute_selection(
                curvyo_ui_core::AlignAxis::Horizontal,
                curvyo_ui_core::DistributeMode::Centres,
                None,
            ),
            AlignOutcome::Aligned
        );
        // The middle rect's centre: 5 + (45 - 5) / 2 = 25 - half of its own
        // extent moved it from 16 to 20.
        let object = session.objects()[1].clone();
        let (min, max) = object_outline_bounds(&object);
        assert_eq!(f64::midpoint(min.x, max.x), 25.0);
        // One undo puts all three back.
        assert!(session.undo().changed);
        let object = session.objects()[1].clone();
        let (min, _) = object_outline_bounds(&object);
        assert_eq!(min.x, 16.0);
    }

    #[test]
    fn a_distribute_with_fewer_than_three_objects_writes_nothing() {
        let mut session = Session::new(1);
        two_rects(&mut session);
        // Without a typed gap the ends define the spacing: two change nothing.
        assert_eq!(
            session.distribute_selection(
                curvyo_ui_core::AlignAxis::Horizontal,
                curvyo_ui_core::DistributeMode::Gaps,
                None,
            ),
            AlignOutcome::Unchanged
        );
    }

    #[test]
    fn a_typed_gap_distributes_two_objects_from_the_first() {
        let mut session = Session::new(1);
        two_rects(&mut session);
        assert_eq!(
            session.distribute_selection(
                curvyo_ui_core::AlignAxis::Horizontal,
                curvyo_ui_core::DistributeMode::Gaps,
                Some(5.0),
            ),
            AlignOutcome::Aligned
        );
        // The second rectangle's left edge: 0 + 30 (the first's extent) + 5.
        assert_eq!(left_x(&session, 1), 35.0);
        // One undo puts the second rectangle back.
        assert!(session.undo().changed);
        assert_eq!(left_x(&session, 1), 50.0);
    }
    #[test]
    fn a_clone_moves_through_its_own_matrix_and_the_original_stays() {
        let mut session = Session::new(1);
        draw_rect(&mut session, (0.0, 0.0), (25.0, 40.0));
        draw_rect(&mut session, (50.0, 0.0), (60.0, 40.0));
        // The clone of the second rectangle, at its original's place.
        session.selection.clear();
        session.selection.add(session.objects()[1].id());
        session.linked_clone();
        select_all(&mut session);
        // Ctrl-press on the first rectangle's outline (away from the group
        // box's own handle hit zones) marks it as the key (criterion 5).
        session.held = Modifiers::new(false, true);
        session.pointer_down(Point::new(15.0, 0.0), false);
        session.pointer_up(Point::new(15.0, 0.0), false, true);
        session.held = Modifiers::NONE;
        eprintln!("key: {:?}", session.key_object().is_some());
        eprintln!(
            "n={} lefts={:?}",
            session.objects().len(),
            (0..session.objects().len())
                .map(|i| left_x(&session, i))
                .collect::<Vec<_>>()
        );
        assert!(session.key_object().is_some());
        // Align right: the key keeps its place, the second rectangle and its
        // clone land on it, the clone through its own placement matrix
        // (criterion 11).
        let outcome = session.align_selection(AlignCommand::Right);
        assert_eq!(outcome, AlignOutcome::Aligned);
        // The key keeps its place; the rectangle and its clone land with
        // their right edges on the key's right edge (25).
        assert_eq!(left_x(&session, 0), 0.0);
        assert_eq!(left_x(&session, 1), 15.0);
        assert_eq!(left_x(&session, 2), 15.0);
    }
}
