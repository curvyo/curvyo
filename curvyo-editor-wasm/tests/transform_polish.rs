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
