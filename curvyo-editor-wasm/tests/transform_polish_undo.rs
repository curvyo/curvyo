//! Undo and redo of the gestures of `specs/0047-transform-polish/` (`specs/0020-undo-redo`
//! criteria 10 and 41): a skew or a stretch that converts shapes to paths, and a typed move with a
//! reference point other than the centre, are each exactly one history step with the gesture's
//! name, and undo and redo restore the document byte for byte.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use curvyo_document_core::{
    Angle, Direction, Document, InnerRatio, Length, ObjectSnapshot, Point, PointCount, RectBounds,
    StarFrame, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{MoveEntryMode, ReferencePoint};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn session_of(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.set_tool(Tool::Select);
    session.resize_viewport(1200.0, 800.0);
    // An opened file reads its history once; the first undo is then as fast as later ones.
    session.warm_history();
    session
}

fn json(session: &Session) -> Vec<u8> {
    unpack(99, &session.pack("0.1.0").unwrap())
        .unwrap()
        .export_json()
        .unwrap()
}

fn kinds(session: &Session) -> Vec<bool> {
    let document = unpack(99, &session.pack("0.1.0").unwrap()).unwrap();
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .map(|object| matches!(object, ObjectSnapshot::Path(_)))
        .collect()
}

fn hold(session: &mut Session, at: Point) {
    session.modifiers_changed(false, false, false);
    session.pointer_hover(at, false, false);
}

fn drag(session: &mut Session, from: Point, to: Point) {
    hold(session, from);
    session.pointer_down(from, false);
    hold(
        session,
        pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
    );
    hold(session, to);
    session.pointer_up(to, false, false);
    hold(session, to);
}

fn key(session: &mut Session, key: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key,
        shift,
        ctrl,
        ..KeyInput::default()
    })
}

fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) {
    let _ = document.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    });
}

/// Asserts that the latest step is `name`, that undo restores `before` and redo `after`, each
/// byte for byte, and that the stacks move by exactly one step.
fn assert_one_step(session: &mut Session, steps_before: usize, name: &str, before: &[u8]) {
    let after = json(session);
    assert_ne!(after, before, "the gesture changed the document");
    assert_eq!(session.undo_len(), steps_before + 1, "exactly one step");
    assert_eq!(
        session.history_target_name(Direction::Undo).as_deref(),
        Some(name)
    );
    assert_eq!(key(session, "z", true, false), KeyOutcome::Undo);
    let _ = session.take_history_notice();
    assert_eq!(json(session), before, "undo is byte-equal");
    assert_eq!(session.undo_len(), steps_before);
    assert_eq!(session.redo_len(), 1);
    assert_eq!(key(session, "z", true, true), KeyOutcome::Redo);
    let _ = session.take_history_notice();
    assert_eq!(json(session), after, "redo is byte-equal");
    assert_eq!(session.undo_len(), steps_before + 1);
}

/// A skew of a rectangle converts it to a path in one step named "Skew".
#[test]
fn a_converting_skew_of_one_shape_is_one_skew_step() {
    let document = Document::new(1);
    rect(&document, 0.0, 0.0, 10.0, 10.0);
    let mut session = session_of(&document);
    drag(&mut session, pt(-30.0, -30.0), pt(60.0, 60.0));
    assert_eq!(session.selected_object_count(), 1);
    let before = json(&session);
    let steps = session.undo_len();
    let handle = pt(5.0, -16.0 / session.view().scale());
    drag(&mut session, handle, pt(handle.x + 10.0, handle.y));
    assert_eq!(kinds(&session), [true], "the rectangle became a path");
    assert_one_step(&mut session, steps, "Skew", &before);
    assert_eq!(kinds(&session), [true]);
}

/// A skew of a mixed selection converts every primitive in one step named "Skew".
#[test]
fn a_converting_skew_of_a_selection_is_one_skew_step() {
    let document = Document::new(1);
    rect(&document, 0.0, 0.0, 40.0, 40.0);
    let _ = document.create_star(
        StarFrame {
            center: pt(70.0, 20.0),
            radius: Length::from_mm(15.0),
            angle: Angle::from_radians(-std::f64::consts::FRAC_PI_2),
        },
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    );
    let mut session = session_of(&document);
    drag(&mut session, pt(-30.0, -30.0), pt(200.0, 120.0));
    assert_eq!(session.selected_object_count(), 2);
    let before = json(&session);
    let steps = session.undo_len();
    // The top skew handle of the group box: 16 px above the middle of its top side.
    let max_x = 70.0 + 15.0 * 18.0_f64.to_radians().cos();
    let handle = pt(max_x / 2.0, -16.0 / session.view().scale());
    drag(&mut session, handle, pt(handle.x + 10.0, handle.y));
    assert_eq!(kinds(&session), [true, true]);
    assert_one_step(&mut session, steps, "Skew", &before);
}

/// A stretch of a selection that converts a star is one step named "Resize".
#[test]
fn a_converting_stretch_is_one_resize_step() {
    let document = Document::new(1);
    rect(&document, 0.0, 0.0, 40.0, 40.0);
    let _ = document.create_star(
        StarFrame {
            center: pt(70.0, 20.0),
            radius: Length::from_mm(15.0),
            angle: Angle::from_radians(-std::f64::consts::FRAC_PI_2),
        },
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    );
    let mut session = session_of(&document);
    drag(&mut session, pt(-30.0, -30.0), pt(200.0, 120.0));
    assert_eq!(session.selected_object_count(), 2);
    let before = json(&session);
    let steps = session.undo_len();
    let right = pt(70.0 + 15.0 * 18.0_f64.to_radians().cos(), 20.0);
    drag(&mut session, right, pt(right.x + 20.0, right.y));
    assert_eq!(kinds(&session), [false, true], "the star became a path");
    assert_one_step(&mut session, steps, "Resize", &before);
}

/// A lone polygon or star stretched by a free corner is one step named "Resize".
#[test]
fn a_converting_stretch_of_a_lone_star_is_one_resize_step() {
    let document = Document::new(1);
    let _ = document.create_star(
        StarFrame {
            center: pt(50.0, 50.0),
            radius: Length::from_mm(20.0),
            angle: Angle::from_radians(-std::f64::consts::FRAC_PI_2),
        },
        PointCount::new(5).unwrap(),
        InnerRatio::new(0.5).unwrap(),
    );
    let mut session = session_of(&document);
    drag(&mut session, pt(-30.0, -30.0), pt(120.0, 120.0));
    assert_eq!(session.selected_object_count(), 1);
    let before = json(&session);
    let steps = session.undo_len();
    // The east edge handle of the star's box.
    let east = pt(
        50.0 + 20.0 * 18.0_f64.to_radians().cos(),
        50.0 + (20.0 * (54.0_f64.to_radians().sin()) - 20.0) / 2.0,
    );
    drag(&mut session, east, pt(east.x + 20.0, east.y));
    assert_eq!(kinds(&session), [true], "the star became a path");
    assert_one_step(&mut session, steps, "Resize", &before);
}

/// A typed move to a position measured from the top-left corner is one step named "Move".
#[test]
fn a_typed_move_with_a_reference_is_one_move_step() {
    let document = Document::new(1);
    rect(&document, 10.0, 20.0, 30.0, 40.0);
    let mut session = session_of(&document);
    drag(&mut session, pt(-30.0, -30.0), pt(100.0, 100.0));
    assert_eq!(
        key(&mut session, "m", false, false),
        KeyOutcome::EntryOpened
    );
    session.set_move_chip_absolute(true);
    session.set_move_reference(ReferencePoint::TopLeft);
    let before = json(&session);
    let steps = session.undo_len();
    session.commit_move_entry(
        "100",
        "50",
        MoveEntryMode {
            absolute: true,
            copy: false,
        },
    );
    assert_one_step(&mut session, steps, "Move", &before);
}

/// A click on a mark executes the same single step.
#[test]
fn a_click_on_a_mark_is_one_move_step() {
    let document = Document::new(1);
    rect(&document, 10.0, 20.0, 30.0, 40.0);
    let mut session = session_of(&document);
    drag(&mut session, pt(-30.0, -30.0), pt(100.0, 100.0));
    assert_eq!(
        key(&mut session, "m", false, false),
        KeyOutcome::EntryOpened
    );
    session.set_move_chip_absolute(true);
    let before = json(&session);
    let steps = session.undo_len();
    let outcome = session.commit_move_entry_at("100", "50", false, ReferencePoint::BottomRight);
    assert_eq!(outcome, curvyo_ui_core::EntryOutcome::Committed);
    assert_one_step(&mut session, steps, "Move", &before);
}
