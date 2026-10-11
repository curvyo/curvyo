//! `curvyo-ui-core` tests of `specs/0047-transform-polish/` Part C: the reference box of a move
//! drag (criterion 33) and the open typed move's state (criteria 37, 39, 41, 49).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds, Tolerance};
use curvyo_ui_core::{
    AnchorIdMinter, EntryKey, Modifiers, ObjectSelection, ReferencePoint, SelectTool,
    TransformHandleTolerances,
};

const SCALE: f64 = 4.0;
const NONE: Modifiers = Modifiers::NONE;
const SHIFT: Modifiers = Modifiers::new(true, false);
const CTRL: Modifiers = Modifiers::new(false, true);

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

struct Rig {
    document: Document,
    selection: ObjectSelection,
    tool: SelectTool,
}

impl Rig {
    fn new() -> Self {
        Self {
            document: Document::new(1),
            selection: ObjectSelection::new(),
            tool: SelectTool::new(),
        }
    }

    fn rect(&self, x: f64, y: f64, w: f64, h: f64) -> NodeId {
        self.document.create_rect(RectBounds {
            origin: pt(x, y),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        })
    }

    fn objects(&self) -> Vec<curvyo_document_core::ObjectSnapshot> {
        self.document
            .object_ids()
            .into_iter()
            .filter_map(|id| self.document.object(id))
            .collect()
    }

    fn press(&mut self, at: Point, modifiers: Modifiers) {
        let objects = self.objects();
        self.tool.pointer_down(
            &objects,
            &mut self.selection,
            at,
            Tolerance::from_mm(2.0),
            TransformHandleTolerances::at_scale(SCALE),
            modifiers,
        );
    }

    fn red_box(&self, at: Point, modifiers: Modifiers) -> Option<(Point, Point)> {
        self.tool.live_move_box(at, modifiers)
    }
}

/// Criterion 33, worked example: a 30 x 40 mm rectangle with its top-left corner at (10, 20)
/// dragged by (+20, +10) shows the red box from (30, 30) to (60, 70).
#[test]
fn the_reference_box_follows_a_move_drag() {
    let mut rig = Rig::new();
    let id = rig.rect(10.0, 20.0, 30.0, 40.0);
    rig.selection.select_single(id);
    // A press inside the box, on no handle.
    rig.press(pt(20.0, 30.0), NONE);
    assert_eq!(
        rig.red_box(pt(20.5, 30.0), NONE),
        None,
        "inside the dead zone"
    );
    let (min, max) = rig
        .red_box(pt(40.0, 40.0), NONE)
        .expect("past the dead zone");
    assert_eq!((min.x, min.y, max.x, max.y), (30.0, 30.0, 60.0, 70.0));
    // The next frame moves it with the preview.
    let (min, max) = rig.red_box(pt(10.0, 30.0), NONE).unwrap();
    assert_eq!((min.x, min.y, max.x, max.y), (0.0, 20.0, 30.0, 60.0));
    // A drag back to its start shows no box.
    assert_eq!(rig.red_box(pt(20.0, 30.0), NONE), None);
}

/// Criterion 33: Shift locks the axis and the box with it; a Ctrl copy moves the same box.
#[test]
fn the_box_follows_the_axis_lock_and_the_copy() {
    let mut rig = Rig::new();
    let id = rig.rect(10.0, 20.0, 30.0, 40.0);
    rig.selection.select_single(id);
    rig.press(pt(20.0, 30.0), NONE);
    let (min, max) = rig.red_box(pt(50.0, 34.0), SHIFT).unwrap();
    assert_eq!(
        (min.x, min.y, max.x, max.y),
        (40.0, 20.0, 70.0, 60.0),
        "x only"
    );
    let (min, _) = rig.red_box(pt(50.0, 34.0), CTRL).unwrap();
    assert_eq!((min.x, min.y), (40.0, 24.0), "the copy travels with it");
}

/// Criterion 48: the box of a selection is the group box, the same rectangle.
#[test]
fn the_box_of_a_selection_is_the_group_box() {
    let mut rig = Rig::new();
    let a = rig.rect(0.0, 0.0, 10.0, 10.0);
    let b = rig.rect(30.0, 5.0, 10.0, 20.0);
    rig.selection.set(&[a, b]);
    rig.press(pt(5.0, 0.0), NONE);
    let (min, max) = rig.red_box(pt(15.0, 10.0), NONE).unwrap();
    assert_eq!((min.x, min.y, max.x, max.y), (10.0, 10.0, 50.0, 35.0));
}

/// Criteria 37 and 49: a chip opens in Relative with the centre as its reference, whatever
/// was chosen before.
#[test]
fn a_chip_opens_in_relative_with_the_centre_every_time() {
    let mut rig = Rig::new();
    let id = rig.rect(10.0, 20.0, 30.0, 40.0);
    rig.selection.select_single(id);
    let objects = rig.objects();
    rig.tool
        .open_entry_for_key(&objects, &rig.selection, EntryKey::Move)
        .unwrap();
    let entry = rig.tool.move_entry().unwrap();
    assert_eq!(entry.reference(), ReferencePoint::Centre);
    assert!(!entry.is_absolute());
    assert_eq!(entry.absolute_prefill(), ["25.0", "40.0"]);
    {
        let entry = rig.tool.move_entry_mut().unwrap();
        entry.set_absolute(true);
        entry.set_reference(ReferencePoint::BottomRight);
        entry.set_radio_hover(Some(ReferencePoint::TopLeft));
    }
    let entry = rig.tool.move_entry().unwrap();
    assert_eq!(entry.reference(), ReferencePoint::BottomRight);
    assert_eq!(entry.absolute_prefill(), ["40.0", "60.0"]);
    rig.tool.cancel_entry();
    rig.tool
        .open_entry_for_key(&objects, &rig.selection, EntryKey::Move)
        .unwrap();
    let entry = rig.tool.move_entry().unwrap();
    assert_eq!(
        entry.reference(),
        ReferencePoint::Centre,
        "never remembered"
    );
    assert!(!entry.is_absolute());
    assert_eq!(entry.radio_hover(), None);
}

/// Criterion 40: the click on a mark executes the move with that point, in one commit.
#[test]
fn a_click_on_a_mark_moves_that_point() {
    let mut rig = Rig::new();
    let id = rig.rect(10.0, 20.0, 30.0, 40.0);
    rig.selection.select_single(id);
    let objects = rig.objects();
    rig.tool
        .open_entry_for_key(&objects, &rig.selection, EntryKey::Move)
        .unwrap();
    let mode = curvyo_ui_core::MoveEntryMode {
        absolute: true,
        copy: false,
    };
    let outcome = rig.tool.commit_move_entry_at(
        &rig.document,
        &mut rig.selection,
        &mut AnchorIdMinter::new(99),
        ["100", "50"],
        (mode, ReferencePoint::BottomRight),
    );
    assert_eq!(outcome, curvyo_ui_core::EntryOutcome::Committed);
    let bounds = curvyo_ui_core::object_outline_bounds(&rig.document.object(id).unwrap());
    assert_eq!((bounds.0.x, bounds.0.y), (70.0, 10.0));
    assert!(rig.tool.move_entry().is_none(), "the chip closes");
}
