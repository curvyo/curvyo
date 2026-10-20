//! `Session`-level tests of `specs/0047-transform-polish/`: Part A (skew and stretch
//! convert primitives, silently), Part B has no Rust surface, Part C (the typed move's
//! reference point) is in the second half of this file. The arithmetic is tested in
//! `curvyo-ui-core`; this drives the public API of `Session` with pointer events and keys.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]
#![allow(clippy::float_cmp)]

use std::collections::BTreeSet;

use curvyo_document_core::{
    Angle, Document, EllipseFrame, InnerRatio, Length, ObjectSnapshot, Point, PointCount,
    RectBounds, StarFrame, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::EntryOutcome;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn doc_of(session: &Session) -> Document {
    unpack(99, &session.pack("0.1.0").unwrap()).unwrap()
}

fn objects(session: &Session) -> Vec<ObjectSnapshot> {
    let document = doc_of(session);
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn change_count(session: &Session) -> usize {
    let doc = loro::LoroDoc::new();
    doc.import(&doc_of(session).export_loro_snapshot().unwrap())
        .unwrap();
    doc.len_changes()
}

fn k(session: &Session) -> f64 {
    session.view().scale()
}

fn hold(session: &mut Session, at: Point, shift: bool, ctrl: bool) {
    session.modifiers_changed(shift, ctrl, false);
    session.pointer_hover(at, shift, ctrl);
}

fn click(session: &mut Session, at: Point, shift: bool) {
    hold(session, at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
    hold(session, at, false, false);
}

fn drag_with(session: &mut Session, from: Point, to: Point, shift: bool, ctrl: bool) {
    hold(session, from, shift, ctrl);
    session.pointer_down(from, shift);
    let middle = pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y));
    hold(session, middle, shift, ctrl);
    hold(session, to, shift, ctrl);
    session.pointer_up(to, shift, ctrl);
    hold(session, to, false, false);
}

fn key(session: &mut Session, key: &str, shift: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key,
        shift,
        ..KeyInput::default()
    })
}

fn rect_doc(x: f64, y: f64, w: f64, h: f64) -> Document {
    let document = Document::new(1);
    let _ = document.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    });
    document
}

#[allow(clippy::cast_possible_truncation)] // test coordinates are small whole numbers
fn whole(value: f64) -> i64 {
    value.round() as i64
}

fn corners(object: &ObjectSnapshot) -> Vec<(i64, i64)> {
    let ObjectSnapshot::Path(path) = object else {
        panic!("a path");
    };
    let mut corners: Vec<(i64, i64)> = path
        .anchors
        .iter()
        .map(|a| (whole(a.point.x), whole(a.point.y)))
        .collect();
    corners.sort_unstable();
    corners
}

fn collect_keys(value: &loro::LoroValue, out: &mut BTreeSet<String>) {
    match value {
        loro::LoroValue::Map(map) => {
            for (key, inner) in map.iter() {
                out.insert(key.clone());
                collect_keys(inner, out);
            }
        }
        loro::LoroValue::List(list) => {
            for inner in list.iter() {
                collect_keys(inner, out);
            }
        }
        _ => {}
    }
}

fn stored_keys(document: &Document) -> BTreeSet<String> {
    let doc = loro::LoroDoc::new();
    doc.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    let mut out = BTreeSet::new();
    collect_keys(&doc.get_deep_value(), &mut out);
    out
}

/// The skew handle of the top side of a box centred on `x`, 16 px above `top`.
fn top_skew(session: &Session, x: f64, top: f64) -> Point {
    pt(x, top - 16.0 / k(session))
}

// ---------------------------------------------------------------------
// Part A
// ---------------------------------------------------------------------

/// Criteria 2, 3, 4, 14: the worked example is one commit, saves and reopens as a path with
/// the same keys "Object to path" writes, and a replica sees the old or the final document.
#[test]
fn a_skewed_rectangle_is_one_commit_and_a_path_after_save_and_reopen() {
    let document = rect_doc(0.0, 0.0, 10.0, 10.0);
    let id = document.object_ids()[0];
    let before_bytes = pack(&document, "0.1.0").unwrap();
    let mut session = session_of(&document);
    click(&mut session, pt(5.0, 0.0), false);
    assert_eq!(session.selected_object_count(), 1);
    let commits = change_count(&session);
    let top = top_skew(&session, 5.0, 0.0);
    drag_with(&mut session, top, pt(top.x + 10.0, top.y), false, false);
    assert_eq!(change_count(&session), commits + 1, "one commit");
    let after = objects(&session);
    assert_eq!(after.len(), 1);
    assert_eq!(corners(&after[0]), [(0, 10), (10, 0), (10, 10), (20, 0)]);
    assert_eq!(after[0].id(), id, "same object");
    // Saved and reopened: a path with the geometry it had, no primitive left.
    let reopened = unpack(7, &session.pack("0.1.0").unwrap()).unwrap();
    assert!(reopened.primitive(id).is_none());
    assert_eq!(corners(&reopened.object(id).unwrap()), corners(&after[0]));
    // No new field: the keys are the ones "Object to path" writes.
    let mut reference = session_of(&rect_doc(0.0, 0.0, 10.0, 10.0));
    click(&mut reference, pt(5.0, 0.0), false);
    reference.convert_selected_to_paths();
    assert_eq!(
        stored_keys(&doc_of(&session)),
        stored_keys(&doc_of(&reference))
    );
    // A replica that imports the earlier snapshot sees the primitive, the later one the path.
    let old = unpack(8, &before_bytes).unwrap();
    assert!(old.primitive(id).is_some());
    assert!(reopened.path(id).is_some());
}

/// Criterion 11: no text for a conversion. K on a rectangle opens the chip, and the
/// refusal and notice APIs are gone: the hint outcome is a plain "entry opened".
#[test]
fn k_opens_the_skew_chip_of_any_shape_and_says_nothing_about_paths() {
    let document = Document::new(1);
    let _ = document.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(40.0),
        height: Length::from_mm(30.0),
    });
    let _ = document.create_ellipse(EllipseFrame {
        center: pt(80.0, 15.0),
        rx: Length::from_mm(15.0),
        ry: Length::from_mm(10.0),
    });
    let _ = document.create_star(
        StarFrame {
            center: pt(130.0, 15.0),
            radius: Length::from_mm(14.0),
            angle: Angle::from_radians(-std::f64::consts::FRAC_PI_2),
        },
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    );
    let mut session = session_of(&document);
    click(&mut session, pt(20.0, 0.0), false);
    assert_eq!(key(&mut session, "k", false), KeyOutcome::EntryOpened);
    session.cancel_transform_entry();
    // The whole selection: the typed angle skews and converts all three, in one commit.
    session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(session.selected_object_count(), 3);
    let commits = change_count(&session);
    assert_eq!(key(&mut session, "k", false), KeyOutcome::EntryOpened);
    assert_eq!(
        session.commit_transform_entry("30", "", 0),
        EntryOutcome::Committed
    );
    assert_eq!(change_count(&session), commits + 1, "one commit");
    assert!(
        objects(&session)
            .iter()
            .all(|o| matches!(o, ObjectSnapshot::Path(_))),
        "every shape became a path"
    );
    assert_eq!(session.selected_object_count(), 3, "all stay selected");
    // "Object to path" is not offered any more (nothing is left to convert).
    assert!(!session.select_bar_state().object_to_path);
}

// ---------------------------------------------------------------------
// Part C
// ---------------------------------------------------------------------

use curvyo_render_core::RgbaColor;
use curvyo_ui_core::{MoveEntryMode, ReferencePoint};

/// The 30 x 40 mm rectangle with its top-left corner at (10, 20) of criteria 37 to 40.
fn tall() -> Document {
    rect_doc(10.0, 20.0, 30.0, 40.0)
}

fn origin_of(session: &Session) -> Point {
    match &objects(session)[0] {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => match p.shape {
            curvyo_document_core::Shape::Rect { bounds, .. } => bounds.origin,
            _ => panic!("a rectangle"),
        },
        ObjectSnapshot::Path(_) => panic!("a primitive"),
    }
}

const RED_BOX: RgbaColor = RgbaColor::opaque(0xE5, 0x48, 0x4D).with_alpha(140);
const RED_MARK: RgbaColor = RgbaColor::opaque(0xE5, 0x48, 0x4D);

fn count_colour(session: &Session, colour: RgbaColor) -> usize {
    session
        .draw_list()
        .triangles
        .iter()
        .filter(|vertex| vertex.color == colour)
        .count()
}

/// The chip of the key M on the selected rectangle, in the given mode.
fn open_move_chip(document: &Document, absolute: bool) -> Session {
    let mut session = session_of(document);
    click(&mut session, pt(25.0, 20.0), false);
    assert_eq!(key(&mut session, "m", false), KeyOutcome::EntryOpened);
    session.set_move_chip_absolute(absolute);
    session
}

/// Criterion 37: the chip opens in Relative with the centre as its reference, and shows that
/// point's current position in Absolute: X "25.0", Y "40.0".
#[test]
fn the_chip_opens_with_the_centre_and_its_position() {
    let session = open_move_chip(&tall(), false);
    let view = session.move_entry().expect("the chip is open");
    assert_eq!(view.reference, ReferencePoint::Centre);
    assert_eq!(
        view.absolute_prefills[ReferencePoint::Centre.index()],
        ["25.0", "40.0"]
    );
    assert_eq!(
        view.absolute_prefills[ReferencePoint::TopLeft.index()],
        ["10.0", "20.0"]
    );
    assert_eq!(view.relative_prefill, ["0", "0"]);
    // The box bottom is the lower edge of the box (y = 60) under its centre.
    assert!((view.box_bottom.y - 60.0).abs() < 1e-9 && (view.box_bottom.x - 25.0).abs() < 1e-9);
}

/// Criterion 38, worked example: typed 100 and 50, plain Enter, the centre is at (100, 50) and
/// the top-left corner at (85, 30), in one commit.
#[test]
fn plain_enter_uses_the_centre() {
    let mut session = open_move_chip(&tall(), true);
    let commits = change_count(&session);
    let outcome = session.commit_move_entry(
        "100",
        "50",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    assert_eq!(outcome, EntryOutcome::Committed);
    assert_eq!(change_count(&session), commits + 1);
    let origin = origin_of(&session);
    assert_eq!((origin.x, origin.y), (85.0, 30.0));
}

/// Criterion 41: choosing a point applies nothing; Enter then uses it.
#[test]
fn a_chosen_point_is_used_by_enter_and_applies_nothing_by_itself() {
    let mut session = open_move_chip(&tall(), true);
    let before = session.pack("0.1.0").unwrap();
    session.set_move_reference(ReferencePoint::LeftMiddle);
    assert_eq!(session.pack("0.1.0").unwrap(), before);
    assert_eq!(
        session.move_entry().expect("the chip is open").reference,
        ReferencePoint::LeftMiddle
    );
    session.commit_move_entry(
        "100",
        "50",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    let origin = origin_of(&session);
    assert_eq!(
        (origin.x, origin.y),
        (100.0, 30.0),
        "the middle of the left edge is at (100, 50)"
    );
    // Opened again: the centre, whatever was chosen before; and Relative.
    click(&mut session, pt(100.0, 50.0), false);
    assert_eq!(key(&mut session, "m", false), KeyOutcome::EntryOpened);
    assert_eq!(
        session.move_entry().expect("the chip is open").reference,
        ReferencePoint::Centre
    );
}

/// Criterion 40, the examples: a click on each mark moves that point to the typed position.
#[test]
fn a_click_on_a_mark_moves_its_point_there() {
    for (reference, origin) in [
        (ReferencePoint::TopLeft, (100.0, 50.0)),
        (ReferencePoint::BottomRight, (70.0, 10.0)),
        (ReferencePoint::Centre, (85.0, 30.0)),
        (ReferencePoint::LeftMiddle, (100.0, 30.0)),
        (ReferencePoint::BottomCentre, (85.0, 10.0)),
    ] {
        let mut session = open_move_chip(&tall(), true);
        // The mark is where the point is; the pointer rests on it.
        let at = reference.point((pt(10.0, 20.0), pt(40.0, 60.0)));
        assert_eq!(
            session.move_mark_at(at, false),
            Some(reference),
            "{reference:?}"
        );
        let commits = change_count(&session);
        assert_eq!(
            session.commit_move_entry_at("100", "50", false, reference),
            EntryOutcome::Committed
        );
        assert_eq!(change_count(&session), commits + 1);
        let moved = origin_of(&session);
        assert_eq!((moved.x, moved.y), origin, "{reference:?}");
        assert!(session.move_entry().is_none(), "the chip closes");
    }
}

/// Criterion 40: typed X = 100 only, then the top-right mark: its x becomes 100 and its y stays:
/// the origin goes to (70, 20). Criterion 45: nothing typed writes nothing and closes.
#[test]
fn an_untouched_field_stays_and_nothing_typed_writes_nothing() {
    let mut session = open_move_chip(&tall(), true);
    // The fields showed the top-right prefill "20.0" for Y while the pointer was on its mark.
    assert_eq!(
        session.commit_move_entry_at("100", "20.0", false, ReferencePoint::TopRight),
        EntryOutcome::Committed
    );
    let origin = origin_of(&session);
    assert_eq!((origin.x, origin.y), (70.0, 20.0));
    let mut session = open_move_chip(&tall(), true);
    let before = session.pack("0.1.0").unwrap();
    assert_eq!(
        session.commit_move_entry_at("40.0", "20.0", false, ReferencePoint::TopRight),
        EntryOutcome::Unchanged
    );
    assert_eq!(session.pack("0.1.0").unwrap(), before);
    assert!(session.move_entry().is_none(), "the chip closes");
}

/// Criterion 45: an invalid field keeps the chip open and writes nothing.
#[test]
fn an_invalid_field_on_a_mark_click_keeps_the_chip_open() {
    let mut session = open_move_chip(&tall(), true);
    let before = session.pack("0.1.0").unwrap();
    assert!(matches!(
        session.commit_move_entry_at("abc", "50", false, ReferencePoint::TopLeft),
        EntryOutcome::Invalid { field: 0, .. }
    ));
    assert!(matches!(
        session.commit_move_entry_at("99999999", "50", false, ReferencePoint::TopLeft),
        EntryOutcome::Invalid { field: 0, .. }
    ));
    assert_eq!(session.pack("0.1.0").unwrap(), before);
    assert!(session.move_entry().is_some());
}

/// Criterion 46: a click on a mark with the Copy check makes the displaced copy, selects it
/// and leaves the original.
#[test]
fn a_click_on_a_mark_with_copy_makes_a_copy() {
    let mut session = open_move_chip(&tall(), true);
    assert_eq!(
        session.commit_move_entry_at("100", "50", true, ReferencePoint::Centre),
        EntryOutcome::Committed
    );
    assert_eq!(objects(&session).len(), 2);
    assert_eq!(session.selected_object_count(), 1, "the copy is selected");
}

/// Criteria 39 and 40: the marks exist only in Absolute mode, and the cursor and the hint
/// follow a mark under the pointer.
#[test]
fn the_marks_exist_in_absolute_only() {
    let mut session = open_move_chip(&tall(), false);
    let top_left = pt(10.0, 20.0);
    session.pointer_hover(top_left, false, false);
    assert_eq!(
        session.move_mark_at(top_left, false),
        None,
        "Relative: no marks"
    );
    assert_eq!(count_colour(&session, RED_MARK), 0);
    assert_ne!(session.cursor_hint(), "pointer", "no mark in Relative");
    session.set_move_chip_absolute(true);
    session.pointer_hover(top_left, false, false);
    assert_eq!(
        session.move_mark_at(top_left, false),
        Some(ReferencePoint::TopLeft)
    );
    assert_eq!(session.move_mark_hover(), Some(ReferencePoint::TopLeft));
    assert_eq!(session.cursor_hint(), "pointer");
    assert_eq!(session.handle_hint(), "reference-mark");
    assert!(count_colour(&session, RED_MARK) > 0, "the marks are drawn");
    // A pointer 20 px from every mark is on none.
    let off = pt(
        top_left.x + 20.0 / k(&session),
        top_left.y + 20.0 / k(&session),
    );
    assert_eq!(session.move_mark_at(off, false), None);
}

/// Criterion 34: the box is drawn while the chip is open, in both modes, and goes with it.
#[test]
fn the_box_is_drawn_while_the_chip_is_open() {
    let mut session = open_move_chip(&tall(), false);
    assert!(count_colour(&session, RED_BOX) > 0, "Relative");
    session.set_move_chip_absolute(true);
    assert!(count_colour(&session, RED_BOX) > 0, "Absolute");
    session.escape();
    assert_eq!(count_colour(&session, RED_BOX), 0, "closed");
    assert_eq!(count_colour(&session, RED_MARK), 0);
}

/// Criterion 33: the box follows a move drag, in step with the preview, and disappears at the
/// release and at Escape.
#[test]
fn the_box_follows_a_move_drag() {
    let mut session = session_of(&tall());
    click(&mut session, pt(25.0, 20.0), false);
    assert_eq!(count_colour(&session, RED_BOX), 0, "no box at rest");
    hold(&mut session, pt(20.0, 30.0), false, false);
    session.pointer_down(pt(20.0, 30.0), false);
    hold(&mut session, pt(40.0, 40.0), false, false);
    assert!(count_colour(&session, RED_BOX) > 0, "during the drag");
    session.escape();
    assert_eq!(count_colour(&session, RED_BOX), 0, "Escape");
    hold(&mut session, pt(20.0, 30.0), false, false);
    session.pointer_down(pt(20.0, 30.0), false);
    hold(&mut session, pt(40.0, 40.0), false, false);
    session.pointer_up(pt(40.0, 40.0), false, false);
    assert_eq!(count_colour(&session, RED_BOX), 0, "released");
}

/// Criterion 36 (UX re-check): in a move drag the box lies above the blue preview, whose white
/// casing would cover it where the two coincide, and below the selection box and the handles:
/// blue vertices come both before and after the box in the draw list.
#[test]
fn the_box_is_drawn_above_the_blue_preview_and_below_the_handles() {
    let mut session = session_of(&tall());
    click(&mut session, pt(25.0, 20.0), false);
    hold(&mut session, pt(20.0, 30.0), false, false);
    session.pointer_down(pt(20.0, 30.0), false);
    hold(&mut session, pt(40.0, 40.0), false, false);
    let list = session.draw_list();
    let index_of = |keep: &dyn Fn(usize, RgbaColor) -> bool| -> Vec<usize> {
        list.triangles
            .iter()
            .enumerate()
            .filter(|(i, vertex)| keep(*i, vertex.color))
            .map(|(i, _)| i)
            .collect()
    };
    let red = index_of(&|_, colour| colour == RED_BOX);
    let blue =
        index_of(&|_, colour| i32::from(colour.b) - i32::from(colour.r) > 60 && colour.a > 0);
    let (first_red, last_red) = (red[0], red[red.len() - 1]);
    assert!(
        blue.iter().any(|&i| i < first_red),
        "the blue preview is drawn before the box"
    );
    assert!(
        blue.iter().any(|&i| i > last_red),
        "the selection box and handles are drawn after the box"
    );
}

/// Criterion 50: a 2 mm square at 100 % zoom has the box but no mark; the Reference control
/// with "Top left" and Enter moves the top-left corner.
#[test]
fn a_tiny_box_has_no_marks_and_the_control_still_works() {
    let document = rect_doc(10.0, 10.0, 2.0, 2.0);
    let mut session = session_of(&document);
    click(&mut session, pt(11.0, 10.0), false);
    assert_eq!(key(&mut session, "m", false), KeyOutcome::EntryOpened);
    session.set_move_chip_absolute(true);
    assert!(count_colour(&session, RED_BOX) > 0, "the box is drawn");
    assert_eq!(count_colour(&session, RED_MARK), 0, "no mark is");
    assert_eq!(session.move_mark_at(pt(10.0, 10.0), false), None);
    session.set_move_reference(ReferencePoint::TopLeft);
    session.commit_move_entry(
        "30",
        "40",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    let origin = origin_of(&session);
    assert_eq!((origin.x, origin.y), (30.0, 40.0));
}

/// Criterion 48: a multi-selection uses the group box: typed 0 and 0 with the centre moves every
/// object by the same offset in one commit.
#[test]
fn a_selection_moves_by_one_offset_about_the_group_centre() {
    let document = Document::new(1);
    let _ = document.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(10.0),
    });
    let _ = document.create_rect(RectBounds {
        origin: pt(30.0, 0.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(10.0),
    });
    let mut session = session_of(&document);
    click(&mut session, pt(10.0, 0.0), false);
    click(&mut session, pt(40.0, 0.0), true);
    assert_eq!(key(&mut session, "m", false), KeyOutcome::EntryOpened);
    session.set_move_chip_absolute(true);
    let view = session.move_entry().expect("the chip is open");
    assert_eq!(
        view.absolute_prefills[ReferencePoint::Centre.index()],
        ["25.0", "5.0"]
    );
    let commits = change_count(&session);
    session.commit_move_entry(
        "0",
        "0",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    assert_eq!(change_count(&session), commits + 1, "one commit");
    let all = objects(&session);
    let first = match &all[0] {
        ObjectSnapshot::Text(_) => unreachable!("no text in this test"),
        ObjectSnapshot::Clone(_) => panic!("no clone here"),
        ObjectSnapshot::Primitive(p) => match p.shape {
            curvyo_document_core::Shape::Rect { bounds, .. } => bounds.origin,
            _ => panic!(),
        },
        ObjectSnapshot::Path(_) => panic!(),
    };
    assert_eq!((first.x, first.y), (-25.0, -5.0));
}

/// Criterion 40, multi-selection example: the rotated 20 x 10 rectangle and the circle of radius
/// 5 at (40, 5): typed 0 and 0 and Enter move every object by (-21.92, -5.00); the rectangle's
/// centre goes to (-11.92, 0.00) and the circle's to (18.08, 0.00).
#[test]
fn the_multi_selection_example_moves_every_object_by_one_offset() {
    let document = Document::new(1);
    let turned = document.create_rect(RectBounds {
        origin: pt(0.0, 0.0),
        width: Length::from_mm(20.0),
        height: Length::from_mm(10.0),
    });
    let _ = document.create_ellipse(EllipseFrame {
        center: pt(40.0, 5.0),
        rx: Length::from_mm(5.0),
        ry: Length::from_mm(5.0),
    });
    let object = document
        .object(turned)
        .unwrap()
        .rotated(pt(10.0, 5.0), Angle::from_radians(30.0_f64.to_radians()));
    document.rotate_object(&object).unwrap();
    let mut session = session_of(&document);
    // The rectangle's turned outline passes through (10, -4.3) at its top.
    click(&mut session, pt(10.0, 5.0), false);
    session.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(session.selected_object_count(), 2);
    assert_eq!(key(&mut session, "m", false), KeyOutcome::EntryOpened);
    let prefill =
        session.move_entry().unwrap().absolute_prefills[ReferencePoint::Centre.index()].clone();
    assert_eq!(
        prefill,
        ["21.9", "5.0"],
        "the centre (21.92, 5.00) of the group box"
    );
    session.set_move_chip_absolute(true);
    session.commit_move_entry(
        "0",
        "0",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    let all = objects(&session);
    let ObjectSnapshot::Primitive(rect) = &all[0] else {
        panic!("a rectangle")
    };
    let curvyo_document_core::Shape::Rect { bounds, .. } = rect.shape else {
        panic!("a rectangle")
    };
    let centre = (
        bounds.origin.x + bounds.width.as_mm() / 2.0,
        bounds.origin.y + bounds.height.as_mm() / 2.0,
    );
    assert!(
        (centre.0 + 11.920_254).abs() < 1e-3 && centre.1.abs() < 1e-3,
        "{centre:?}"
    );
    let ObjectSnapshot::Primitive(circle) = &all[1] else {
        panic!("a circle")
    };
    let curvyo_document_core::Shape::Ellipse { frame, .. } = circle.shape else {
        panic!("a circle")
    };
    assert!(
        (frame.center.x - 18.079_746).abs() < 1e-3 && frame.center.y.abs() < 1e-3,
        "{:?}",
        frame.center
    );
}
