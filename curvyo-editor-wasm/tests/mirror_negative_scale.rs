//! `curvyo-editor-wasm` tests of `specs/0056-mirror-and-negative-scale/` through the
//! session: the flip commands and their History step (criteria 1 to 5, 29), the keys H
//! and V (criterion 4, Proposal), the signed readout with its mirror line (criterion 10),
//! the notice of a size set to 0.01 mm (criterion 15), the chip's note line, and a file
//! round trip (criterion 26).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    Direction, Document, ObjectSnapshot, Point, PrimitiveSnapshot, Shape, Timestamp, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{EntryOutcome, FlipAxis, FlipOutcome};

static NOW: AtomicU64 = AtomicU64::new(1_800_000_000);

fn clock() -> Timestamp {
    Timestamp::from_secs(NOW.fetch_add(7, Ordering::SeqCst))
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn fresh() -> Session {
    let mut s = Session::new(11);
    s.set_clock(clock);
    s.resize_viewport(1200.0, 800.0);
    s.set_tool(Tool::Select);
    s
}

fn reread(s: &Session) -> Document {
    unpack(99, &s.pack("0.1.0").unwrap()).unwrap()
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

fn drag_shape(s: &mut Session, tool: Tool, a: Point, b: Point) {
    s.set_tool(tool);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

/// One rectangle (20, 20) to (80, 60), selected, Select tool.
fn one_rect() -> Session {
    let mut s = fresh();
    drag_shape(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    s
}

fn rect_x(s: &Session) -> (f64, f64) {
    let d = reread(s);
    let Some(ObjectSnapshot::Primitive(PrimitiveSnapshot {
        shape: Shape::Rect { bounds, .. },
        ..
    })) = d.object(d.object_ids()[0])
    else {
        panic!("a rectangle");
    };
    (bounds.origin.x, bounds.origin.x + bounds.width.as_mm())
}

/// Criteria 2 and 29: a flip is one step named "Flip horizontally", undone exactly.
#[test]
fn a_flip_is_one_named_step() {
    let mut s = fresh();
    drag_shape(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(40.0, 40.0));
    drag_shape(&mut s, Tool::Rectangle, pt(60.0, 20.0), pt(100.0, 30.0));
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    let before = reread(&s).export_json().unwrap();
    let undo_before = s.undo_len();
    assert!(s.flip_enabled());
    assert_eq!(s.flip_selection(FlipAxis::Horizontal), FlipOutcome::Flipped);
    assert_eq!(s.undo_len(), undo_before + 1);
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Flip horizontally")
    );
    assert_eq!(s.selected_object_count(), 2, "the selection stays");
    assert_ne!(reread(&s).export_json().unwrap(), before);
    let _ = key(&mut s, "z", true);
    assert_eq!(reread(&s).export_json().unwrap(), before);
    assert_eq!(s.flip_selection(FlipAxis::Vertical), FlipOutcome::Flipped);
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Flip vertically")
    );
}

/// Criterion 4 (Proposal): H and V flip in the Select tool, gated like every letter;
/// with nothing selected the hint; in another tool nothing.
#[test]
fn the_keys_h_and_v() {
    let mut s = fresh();
    drag_shape(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    drag_shape(&mut s, Tool::Rectangle, pt(100.0, 20.0), pt(110.0, 30.0));
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    // About x = 65: the 60 mm rectangle goes to 50..110.
    assert_eq!(key(&mut s, "h", false).code(), "flip-horizontal");
    assert_eq!(rect_x(&s), (50.0, 110.0));
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Flip horizontally")
    );
    assert_eq!(key(&mut s, "V", false).code(), "flip-vertical");
    let steps = s.undo_len();
    assert_eq!(
        key(&mut s, "h", true),
        KeyOutcome::Ignored,
        "Ctrl+H is not a flip"
    );
    assert_eq!(s.undo_len(), steps);
    assert_eq!(key(&mut s, "s", false), KeyOutcome::EntryOpened);
    assert_eq!(key(&mut s, "h", false), KeyOutcome::Ignored, "an open chip");
    s.cancel_transform_entry();
    let _ = key(&mut s, "Escape", false);
    assert_eq!(s.selected_object_count(), 0);
    assert_eq!(key(&mut s, "h", false).code(), "hint-select-first");
    s.set_tool(Tool::Node);
    assert_eq!(key(&mut s, "v", false), KeyOutcome::Ignored);
}

/// Criterion 3 through the key: a circle looks the same flipped.
#[test]
fn a_circle_looks_the_same_flipped() {
    let mut s = fresh();
    s.set_tool(Tool::Ellipse);
    s.pointer_hover(pt(20.0, 20.0), false, false);
    s.pointer_down(pt(20.0, 20.0), false);
    s.pointer_hover(pt(40.0, 40.0), false, true);
    s.pointer_up(pt(40.0, 40.0), false, true);
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    let steps = s.undo_len();
    assert_eq!(key(&mut s, "h", false).code(), "flip-unchanged");
    assert_eq!(s.undo_len(), steps, "nothing written");
}

/// Criterion 10: the live readout shows the signed size and names the mirror.
#[test]
fn the_readout_is_signed_and_names_the_mirror() {
    let mut s = one_rect();
    s.pointer_hover(pt(80.0, 40.0), false, false);
    s.pointer_down(pt(80.0, 40.0), false);
    s.pointer_hover(pt(8.0, 40.0), false, false);
    assert_eq!(
        s.live_readout().unwrap().text,
        "\u{2212}12.0 × 40.0 mm\nMirrored left to right"
    );
    s.pointer_hover(pt(8.0, 0.0), false, false);
    s.pointer_up(pt(8.0, 0.0), false, false);
    assert_eq!(rect_x(&s), (8.0, 20.0));

    let mut s = fresh();
    drag_shape(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(40.0, 40.0));
    drag_shape(&mut s, Tool::Rectangle, pt(60.0, 20.0), pt(100.0, 40.0));
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    s.pointer_hover(pt(100.0, 40.0), false, false);
    s.pointer_down(pt(100.0, 40.0), false);
    s.pointer_hover(pt(10.0, 0.0), false, false);
    assert_eq!(
        s.live_readout().unwrap().text,
        "\u{2212}10.0 mm × \u{2212}20.0 mm\nTurned 180°"
    );
}

/// Criterion 15: a release on the fixed edge and a typed 0 set 0.01 mm and say so once.
#[test]
fn a_size_set_to_a_hundredth_says_so() {
    let mut s = one_rect();
    s.pointer_hover(pt(80.0, 40.0), false, false);
    s.pointer_down(pt(80.0, 40.0), false);
    s.pointer_hover(pt(30.0, 40.0), false, false);
    s.pointer_hover(pt(20.0, 40.0), false, false);
    s.pointer_up(pt(20.0, 40.0), false, false);
    let (x0, x1) = rect_x(&s);
    assert!((x0 - 20.0).abs() < 1e-9 && (x1 - 20.01).abs() < 1e-9);
    assert_eq!(
        s.take_transform_notice(),
        "Width set to 0.01 mm. A zero size would flatten the shape for good."
    );
    assert_eq!(s.take_transform_notice(), "");

    let mut s = one_rect();
    assert_eq!(key(&mut s, "s", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.transform_entry_mirror_line("-60", "40.0", 0),
        "Mirrored left to right"
    );
    assert_eq!(s.transform_entry_mirror_line("60.0", "40.0", 0), "");
    assert_eq!(
        s.commit_transform_entry("0", "0", 1),
        EntryOutcome::Committed
    );
    assert_eq!(
        s.take_transform_notice(),
        "Width and height set to 0.01 mm. Zero would flatten the shape."
    );
}

/// Criterion 26: flipped, turned over and typed-negative objects reopen as they were,
/// with no negative stored length.
#[test]
fn a_mirrored_project_reopens_unchanged() {
    let mut s = one_rect();
    drag_shape(&mut s, Tool::PolygonStar, pt(120.0, 40.0), pt(140.0, 50.0));
    assert_eq!(key(&mut s, "a", true), KeyOutcome::SelectedAll);
    assert_eq!(s.flip_selection(FlipAxis::Vertical), FlipOutcome::Flipped);
    assert_eq!(key(&mut s, "s", false), KeyOutcome::EntryOpened);
    let view = s.transform_entry().unwrap();
    let (width, height) = (
        view.fields[0].prefill.clone(),
        view.fields[1].prefill.clone(),
    );
    assert_eq!(
        s.commit_transform_entry(&format!("-{width}"), &height, 0),
        EntryOutcome::Committed
    );
    let before = reread(&s);
    let bytes = s.pack("0.1.0").unwrap();
    let reopened = Session::open(5, &bytes).unwrap();
    let after = reread(&reopened);
    assert_eq!(before.export_json().unwrap(), after.export_json().unwrap());
    for id in after.object_ids() {
        if let Some(ObjectSnapshot::Primitive(p)) = after.object(id) {
            match p.shape {
                Shape::Rect { bounds, .. } => {
                    assert!(bounds.width.as_mm() > 0.0 && bounds.height.as_mm() > 0.0);
                }
                Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => {
                    assert!(frame.radius.as_mm() > 0.0);
                }
                Shape::Ellipse { frame, .. } => {
                    assert!(frame.rx.as_mm() > 0.0 && frame.ry.as_mm() > 0.0);
                }
            }
        }
    }
}

/// UX notes "Announcement" (`review-ux.md` finding 3): a release or an Enter whose resize
/// mirrors is announced once; a resize with positive factors stays silent.
#[test]
fn a_mirroring_resize_is_announced() {
    let mut s = one_rect();
    s.pointer_hover(pt(80.0, 40.0), false, false);
    s.pointer_down(pt(80.0, 40.0), false);
    s.pointer_hover(pt(8.0, 40.0), false, false);
    s.pointer_up(pt(8.0, 40.0), false, false);
    assert_eq!(
        s.take_transform_announcement(),
        "Resized. Mirrored left to right."
    );
    assert_eq!(s.take_transform_announcement(), "");

    let mut s = one_rect();
    s.pointer_hover(pt(80.0, 40.0), false, false);
    s.pointer_down(pt(80.0, 40.0), false);
    s.pointer_hover(pt(90.0, 40.0), false, false);
    s.pointer_up(pt(90.0, 40.0), false, false);
    assert_eq!(s.take_transform_announcement(), "");

    let mut s = one_rect();
    assert_eq!(key(&mut s, "s", false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_transform_entry("-30", "-40", 1),
        EntryOutcome::Committed
    );
    assert_eq!(s.take_transform_announcement(), "Resized. Turned 180°.");
}
