//! `curvyo-ui-core`'s share of `specs/0073-parameter-handle-clone`: Shift held during a
//! parameter handle drag makes a copy, Shift's state at the release decides (Frage 387: Shift),
//! and the entry's Copy check does the same for a typed value. The copy write itself is
//! unit-tested in `param_copy_tests.rs`; this drives the real tool.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_document_core::{
    Angle, Document, InnerRatio, Length, NodeId, ObjectSnapshot, Point, PointCount, RectBounds,
    Shape, StarFrame, Tolerance,
};
use curvyo_ui_core::{
    AnchorIdMinter, Corner, EditHandle, EntryOutcome, InvalidReason, Modifiers, ObjectSelection,
    ParamHandle, PressTarget, ResizeDirection, SelectDoubleClickOutcome, SelectTool,
    TransformHandleTolerances, classify_press,
};

const SEGMENT_TOLERANCE: Tolerance = Tolerance::from_mm(1.0);
const SHIFT: Modifiers = Modifiers::new(true, false);

/// Two screen pixels per millimetre.
fn tolerances() -> TransformHandleTolerances {
    TransformHandleTolerances::at_scale(2.0)
}

struct Rig {
    document: Document,
    id: NodeId,
    selection: ObjectSelection,
    tool: SelectTool,
}

impl Rig {
    /// A five-point star of radius 30 mm and ratio 0.5 at (50, 50), selected.
    fn star() -> Self {
        let document = Document::new(1);
        let id = document.create_star(
            StarFrame {
                center: Point::new(50.0, 50.0),
                radius: Length::from_mm(30.0),
                angle: Angle::from_radians(0.0),
            },
            PointCount::new(5).unwrap(),
            InnerRatio::new(0.5).unwrap(),
        );
        Self::with(document, id)
    }

    /// A 100 x 60 mm rectangle, selected.
    fn rect() -> Self {
        let document = Document::new(1);
        let id = document.create_rect(RectBounds {
            origin: Point::new(10.0, 20.0),
            width: Length::from_mm(100.0),
            height: Length::from_mm(60.0),
        });
        Self::with(document, id)
    }

    fn with(document: Document, id: NodeId) -> Self {
        let mut selection = ObjectSelection::new();
        selection.select_single(id);
        Self {
            document,
            id,
            selection,
            tool: SelectTool::new(),
        }
    }

    fn objects(&self) -> Vec<ObjectSnapshot> {
        self.document.read_objects().objects.to_vec()
    }

    fn handle_at(&self, wanted: EditHandle) -> Point {
        SelectTool::transform_handles(&self.objects(), &self.selection, tolerances(), false)
            .into_iter()
            .find(|(handle, _)| *handle == wanted)
            .expect("the handle is drawn")
            .1
    }

    fn knob(&self) -> Point {
        self.handle_at(EditHandle::Param(ParamHandle::InnerRadius))
    }

    fn press(&mut self, at: Point, modifiers: Modifiers) {
        let objects = self.objects();
        self.tool.pointer_down(
            &objects,
            &mut self.selection,
            at,
            SEGMENT_TOLERANCE,
            tolerances(),
            modifiers,
        );
    }

    fn release(&mut self, at: Point, modifiers: Modifiers) {
        let objects = self.objects();
        self.tool.pointer_up(
            &self.document,
            &objects,
            &mut self.selection,
            at,
            modifiers,
            &mut AnchorIdMinter::new(99),
        );
    }

    fn live_copy(&self, at: Point, shift: bool) -> Option<bool> {
        self.tool
            .live_edit(&self.objects(), &self.selection, at, shift, false)
            .map(|live| live.copy)
    }

    fn ratio_of(&self, id: NodeId) -> f64 {
        match self.document.primitive(id).unwrap().shape {
            Shape::Star { inner_ratio, .. } => inner_ratio.get(),
            _ => panic!("a star"),
        }
    }

    fn copy_id(&self) -> NodeId {
        let ids = self.document.object_ids();
        assert_eq!(ids.len(), 2, "one copy");
        ids[1]
    }
}

/// Inward along the knob's direction: the knob of a star at angle 0 sits at half a step.
fn inward(from: Point, mm: f64) -> Point {
    let theta = std::f64::consts::TAU / 10.0;
    Point::new(from.x - mm * theta.cos(), from.y - mm * theta.sin())
}

/// Criteria 1, 3, 4, 5: Shift pressed after the press turns the drag into a copy drag in that
/// frame, released it turns back; Shift down at the release writes one copy, selected, the
/// original unchanged.
#[test]
fn shift_during_the_drag_previews_and_writes_a_copy() {
    let mut rig = Rig::star();
    let start = rig.document.object(rig.id).unwrap();
    let knob = rig.knob();
    rig.press(knob, Modifiers::NONE);
    let to = inward(knob, 5.0);
    assert_eq!(
        rig.live_copy(to, false),
        Some(false),
        "a plain drag before Shift"
    );
    assert!(!rig.tool.param_copy_in_flight());
    rig.tool.set_copy_key(true);
    assert_eq!(
        rig.live_copy(to, true),
        Some(true),
        "a copy in the frame of the keydown"
    );
    assert!(rig.tool.param_copy_in_flight());
    let preview = rig
        .tool
        .live_edit(&rig.objects(), &rig.selection, to, true, false)
        .unwrap();
    rig.tool.set_copy_key(false);
    assert_eq!(
        rig.live_copy(to, false),
        Some(false),
        "plain again in the frame of the keyup"
    );
    rig.tool.set_copy_key(true);
    rig.release(to, SHIFT);
    assert_eq!(
        rig.document.object(rig.id).unwrap(),
        start,
        "original unchanged"
    );
    let copy = rig.copy_id();
    assert_eq!(rig.selection.ids(), &[copy], "the copy is selected");
    let ObjectSnapshot::Primitive(mut written) = rig.document.object(copy).unwrap() else {
        panic!("a primitive");
    };
    written.id = rig.id;
    assert_eq!(
        vec![ObjectSnapshot::Primitive(written)],
        preview.objects,
        "the copy is what the preview showed"
    );
    let steps = rig.document.own_steps();
    assert_eq!(steps.last().unwrap().label, "copy_with_inner_ratio");
}

/// Criterion 1 (the release decides): Shift pressed mid-drag and released before the end writes
/// a plain change and no copy; the release's own Shift flag wins over a stale live state.
#[test]
fn the_release_flag_decides_and_a_released_shift_writes_a_change() {
    let mut rig = Rig::star();
    let knob = rig.knob();
    rig.press(knob, Modifiers::NONE);
    rig.tool.set_copy_key(true);
    rig.tool.set_copy_key(false);
    rig.release(inward(knob, 5.0), Modifiers::NONE);
    assert_eq!(rig.document.object_ids(), vec![rig.id], "no copy");
    assert!(rig.ratio_of(rig.id) < 0.5, "the original changed");

    // A lost keyup: the live state says copy, the release says no.
    let mut rig = Rig::star();
    let knob = rig.knob();
    rig.press(knob, Modifiers::NONE);
    rig.tool.set_copy_key(true);
    rig.release(inward(knob, 5.0), Modifiers::NONE);
    assert_eq!(rig.document.object_ids(), vec![rig.id], "no copy");

    // A lost keydown: Shift down at the release only.
    let mut rig = Rig::star();
    let knob = rig.knob();
    rig.press(knob, Modifiers::NONE);
    rig.release(inward(knob, 5.0), SHIFT);
    assert_eq!(rig.ratio_of(rig.id), 0.5, "the original stays");
    assert!(rig.ratio_of(rig.copy_id()) < 0.5);
}

/// Criterion 6: a release inside the 3 px dead zone and Escape write nothing, with Shift.
#[test]
fn the_dead_zone_and_escape_make_no_copy() {
    let mut rig = Rig::star();
    let steps = rig.document.own_step_count();
    let knob = rig.knob();
    rig.press(knob, Modifiers::NONE);
    rig.tool.set_copy_key(true);
    rig.release(inward(knob, 0.5), SHIFT);
    rig.press(knob, Modifiers::NONE);
    rig.tool.set_copy_key(true);
    rig.tool.escape();
    rig.release(inward(knob, 5.0), SHIFT);
    assert_eq!(rig.document.own_step_count(), steps);
    assert_eq!(rig.document.object_ids(), vec![rig.id]);
    assert_eq!(rig.selection.ids(), &[rig.id]);
}

/// Criterion 9: Shift at the press makes the one-corner drag of `0013` and no copy, held to the
/// release too; let go and pressed again during the drag it makes a copy of that drag.
#[test]
fn a_one_corner_radius_copy_keeps_the_other_corners() {
    let diagonal = Corner::Tr.inward_diagonal();
    let mut rig = Rig::rect();
    let knob = rig.handle_at(EditHandle::Param(ParamHandle::CornerRadius(Corner::Tr)));
    let to = Point::new(knob.x + diagonal.x * 8.0, knob.y + diagonal.y * 8.0);
    rig.press(knob, SHIFT);
    rig.tool.set_copy_key(true);
    assert!(
        !rig.tool.param_copy_in_flight(),
        "Shift held from the press"
    );
    rig.release(to, SHIFT);
    assert_eq!(rig.document.object_ids(), vec![rig.id], "no copy");

    let mut rig = Rig::rect();
    rig.press(knob, SHIFT);
    rig.tool.set_copy_key(false);
    rig.tool.set_copy_key(true);
    assert!(rig.tool.param_copy_in_flight(), "pressed again");
    rig.release(to, SHIFT);
    let radii = |id| match rig.document.primitive(id).unwrap().shape {
        Shape::Rect { corner_radii, .. } => corner_radii,
        _ => panic!("a rectangle"),
    };
    let copy = rig.copy_id();
    assert!(radii(copy).tr.as_mm() > 0.0);
    assert_eq!(radii(copy).tl, Length::from_mm(0.0), "one corner only");
    assert_eq!(
        radii(rig.id).tr,
        Length::from_mm(0.0),
        "the original keeps its radii"
    );
}

/// Criteria 2 and 14: Shift held at the press of a knob still starts the knob drag and does
/// not toggle the selection; a resize drag with Shift held keeps its own Shift and copies
/// nothing.
#[test]
fn shift_at_the_press_starts_the_knob_drag_and_leaves_other_handles_alone() {
    let mut rig = Rig::star();
    let objects = rig.objects();
    let knob = rig.knob();
    let target = classify_press(
        &objects,
        &objects,
        &rig.selection,
        knob,
        SEGMENT_TOLERANCE,
        tolerances(),
        SHIFT,
    );
    assert_eq!(
        target,
        PressTarget::Handle(EditHandle::Param(ParamHandle::InnerRadius))
    );
    rig.press(knob, SHIFT);
    assert!(
        !rig.tool.param_copy_in_flight(),
        "Shift at the press makes no copy"
    );
    assert_eq!(
        rig.tool.dragging_handle(),
        Some(EditHandle::Param(ParamHandle::InnerRadius))
    );
    assert_eq!(
        rig.selection.ids(),
        &[rig.id],
        "the selection is not toggled"
    );
    rig.tool.escape();

    let corner = rig.handle_at(EditHandle::Resize(ResizeDirection::Se));
    rig.press(corner, Modifiers::NONE);
    rig.tool.set_copy_key(true);
    assert!(!rig.tool.param_copy_in_flight());
    rig.release(Point::new(corner.x + 10.0, corner.y + 10.0), SHIFT);
    assert_eq!(
        rig.document.object_ids(),
        vec![rig.id],
        "a resize makes no copy"
    );
}

fn open_entry(rig: &mut Rig) {
    let at = rig.knob();
    rig.press(at, Modifiers::NONE);
    rig.release(at, Modifiers::NONE);
    let outcome = rig.tool.double_click(
        &rig.objects(),
        &rig.selection,
        at,
        SEGMENT_TOLERANCE,
        tolerances(),
        (false, false),
    );
    assert_eq!(outcome, SelectDoubleClickOutcome::EntryOpened);
}

/// Criterion 11: Enter with Copy set writes the typed value to a selected copy; an untouched
/// Enter makes a copy at the current value; a refused text and a stale object make none.
#[test]
fn the_entry_copy_check_writes_a_copy() {
    let mut rig = Rig::star();
    open_entry(&mut rig);
    let outcome = rig
        .tool
        .commit_param_entry_as_copy(&rig.document, &mut rig.selection, "0.3");
    assert_eq!(outcome, EntryOutcome::Committed);
    let copy = rig.copy_id();
    assert_eq!(rig.selection.ids(), &[copy]);
    assert!((rig.ratio_of(copy) - 0.3).abs() < 1e-9);
    assert_eq!(rig.ratio_of(rig.id), 0.5);
    assert!(!rig.tool.has_entry());

    // Untouched: a copy at the current value.
    let mut rig = Rig::star();
    open_entry(&mut rig);
    let prefill = rig.tool.param_entry().unwrap().fields()[0].prefill.clone();
    let outcome = rig
        .tool
        .commit_param_entry_as_copy(&rig.document, &mut rig.selection, &prefill);
    assert_eq!(outcome, EntryOutcome::Committed);
    assert_eq!(rig.ratio_of(rig.copy_id()), 0.5);

    // Invalid: the entry stays open, nothing is written.
    let mut rig = Rig::star();
    open_entry(&mut rig);
    let steps = rig.document.own_step_count();
    let outcome = rig
        .tool
        .commit_param_entry_as_copy(&rig.document, &mut rig.selection, "2");
    assert_eq!(
        outcome,
        EntryOutcome::Invalid {
            field: 0,
            reason: InvalidReason::RatioRange
        }
    );
    assert!(rig.tool.has_entry());
    assert_eq!(rig.document.own_step_count(), steps);

    // Stale: the object changed since the entry opened.
    let mut rig = Rig::star();
    open_entry(&mut rig);
    rig.document
        .set_inner_ratio(&[rig.id], InnerRatio::new(0.6).unwrap())
        .unwrap();
    let outcome = rig
        .tool
        .commit_param_entry_as_copy(&rig.document, &mut rig.selection, "0.3");
    assert_eq!(outcome, EntryOutcome::Unchanged);
    assert_eq!(rig.document.object_ids(), vec![rig.id]);
}
