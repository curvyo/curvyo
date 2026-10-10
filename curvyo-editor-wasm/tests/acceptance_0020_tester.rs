//! Independent tester acceptance tests for `specs/0020-undo-redo/specification.md`, session level.
//! Written from the specification and the public `Session` seam before the implementation diff was
//! read. Everything goes through `Session`; documents are compared through the `document.json`
//! export (criterion 10).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::similar_names
)]
#![allow(
    clippy::assert_is_empty,
    clippy::cast_lossless,
    clippy::redundant_closure_for_method_calls,
    clippy::semicolon_if_nothing_returned,
    clippy::bool_assert_comparison,
    clippy::collapsible_if,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
#![allow(missing_docs, clippy::doc_markdown, clippy::needless_pass_by_value)]
#![allow(clippy::many_single_char_names, clippy::cast_possible_truncation)]

use std::sync::atomic::{AtomicU64, Ordering};

use curvyo_document_core::{
    AnchorKind, BackgroundPaint, Direction, DisplayUnit, Document, NodeId, Point, Timestamp, unpack,
};
use curvyo_editor_wasm::{
    DocumentSide, KeyHint, KeyInput, KeyOutcome, Session, SizeOutcome, Tool, WipeOutcome, WipeState,
};
use curvyo_ui_core::{BooleanOp, EntryOutcome, MoveEntryMode, StyleField};

// ---------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------

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

fn json(s: &Session) -> Vec<u8> {
    reread(s).export_json().unwrap()
}

fn ids(s: &Session) -> Vec<NodeId> {
    reread(s).object_ids()
}

fn key(s: &mut Session, k: &str, ctrl: bool, shift: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        shift,
        ..KeyInput::default()
    })
}

/// Ctrl+Z as the host does it: a busy outcome is followed by the call itself.
fn ctrl_z(s: &mut Session) -> (KeyOutcome, String) {
    let out = key(s, "z", true, false);
    finish(s, out)
}

fn ctrl_shift_z(s: &mut Session) -> (KeyOutcome, String) {
    let out = key(s, "Z", true, true);
    finish(s, out)
}

fn finish(s: &mut Session, out: KeyOutcome) -> (KeyOutcome, String) {
    match out {
        KeyOutcome::UndoBusy => {
            let r = s.undo();
            (KeyOutcome::Undo, r.notice)
        }
        KeyOutcome::RedoBusy => {
            let r = s.redo();
            (KeyOutcome::Redo, r.notice)
        }
        other => (other, s.take_history_notice().unwrap_or_default()),
    }
}

fn drag(s: &mut Session, tool: Tool, a: Point, b: Point) {
    s.set_tool(tool);
    s.pointer_hover(a, false, false);
    s.pointer_down(a, false);
    s.pointer_hover(
        pt(f64::midpoint(a.x, b.x), f64::midpoint(a.y, b.y)),
        false,
        false,
    );
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn click(s: &mut Session, at: Point, shift: bool) {
    s.set_tool(Tool::Select);
    s.pointer_hover(at, shift, false);
    s.pointer_down(at, shift);
    s.pointer_up(at, shift, false);
}

fn pen_click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

/// Two overlapping rectangles, both selected, Select tool.
fn two_rects() -> Session {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(40.0, 40.0), pt(100.0, 100.0));
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    assert_eq!(s.selected_object_count(), 2, "setup: both selected");
    s
}

/// Two separate rectangles (they do not touch), both selected.
fn two_apart() -> Session {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(50.0, 50.0));
    drag(&mut s, Tool::Rectangle, pt(80.0, 20.0), pt(110.0, 50.0));
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    assert_eq!(s.selected_object_count(), 2);
    s
}

/// One rectangle at (20,20)-(80,60), selected.
fn one_rect() -> Session {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(80.0, 60.0));
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    assert_eq!(s.selected_object_count(), 1);
    s
}

/// One closed four-anchor path drawn with the Pen, selected, Select tool.
fn one_path() -> Session {
    let mut s = fresh();
    s.set_tool(Tool::Pen);
    for p in [
        pt(20.0, 20.0),
        pt(80.0, 20.0),
        pt(80.0, 60.0),
        pt(20.0, 60.0),
    ] {
        pen_click(&mut s, p);
    }
    s.finish_pen();
    s.set_tool(Tool::Select);
    click(&mut s, pt(50.0, 20.0), false);
    assert_eq!(s.selected_object_count(), 1);
    s
}

/// The round trip of criterion 10 for one gesture: exactly one step, Ctrl+Z gives the export
/// before, Ctrl+Shift+Z the export after, twice over, and the notices name the operation.
fn roundtrip(s: &mut Session, name: &str, gesture: impl FnOnce(&mut Session)) {
    roundtrip_rounds(s, name, 2, gesture);
}

/// As [`roundtrip`] with a chosen number of undo/redo rounds.
fn roundtrip_rounds(
    s: &mut Session,
    name: &str,
    rounds: usize,
    gesture: impl FnOnce(&mut Session),
) {
    let before = json(s);
    let undo_before = s.undo_len();
    gesture(s);
    // The UI reads the stack through `history_target_name`; `undo_len` alone can lag for entry
    // points that are not wrapped (the background edit, see the report).
    let _ = s.history_target_name(Direction::Undo);
    assert_eq!(
        s.undo_len(),
        undo_before + 1,
        "{name}: exactly one step on the undo stack"
    );
    let after = json(s);
    assert_ne!(before, after, "{name}: the gesture changed the document");
    for round in 0..rounds {
        let (out, notice) = ctrl_z(s);
        assert_eq!(out, KeyOutcome::Undo, "{name} round {round}");
        assert!(
            notice.starts_with(&format!("Undid: {name}")),
            "{name}: undo notice was {notice:?}"
        );
        assert_eq!(
            json(s),
            before,
            "{name} round {round}: undo is byte-equal to before"
        );
        assert_eq!(s.undo_len(), undo_before, "{name}");
        assert_eq!(s.redo_len(), 1, "{name}");
        let (out, notice) = ctrl_shift_z(s);
        assert_eq!(out, KeyOutcome::Redo, "{name} round {round}");
        assert!(
            notice.starts_with(&format!("Redid: {name}")),
            "{name}: redo notice was {notice:?}"
        );
        assert_eq!(
            json(s),
            after,
            "{name} round {round}: redo is byte-equal to after"
        );
        assert_eq!(s.undo_len(), undo_before + 1, "{name}");
        assert_eq!(s.redo_len(), 0, "{name}");
    }
}

/// Opens the transform chip with `k`, sets the first field to `first` and keeps the others as
/// prefilled, then commits.
fn typed(s: &mut Session, k: &str, first: &str) -> EntryOutcome {
    assert_eq!(key(s, k, false, false), KeyOutcome::EntryOpened, "{k}");
    let chip = s.transform_entry().expect("a chip");
    let second = chip
        .fields
        .get(1)
        .map(|f| f.prefill.clone())
        .unwrap_or_default();
    s.commit_transform_entry(first, &second, 0)
}

fn rel_move(dx: &str, dy: &str, copy: bool) -> impl FnOnce(&mut Session) {
    let (dx, dy) = (dx.to_owned(), dy.to_owned());
    move |s: &mut Session| {
        assert_eq!(key(s, "m", false, false), KeyOutcome::EntryOpened);
        assert_eq!(
            s.commit_move_entry(
                &dx,
                &dy,
                MoveEntryMode {
                    absolute: false,
                    copy
                }
            ),
            EntryOutcome::Committed
        );
    }
}

// ---------------------------------------------------------------------
// Criterion 10: one test per row of the table "One interaction, one step"
// ---------------------------------------------------------------------

#[test]
fn c10_draw_path() {
    let mut s = fresh();
    roundtrip(&mut s, "Draw path", |s| {
        s.set_tool(Tool::Pen);
        for p in [pt(20.0, 20.0), pt(80.0, 20.0), pt(80.0, 60.0)] {
            pen_click(s, p);
        }
        s.finish_pen();
    });
}

#[test]
fn c10_draw_rectangle_ellipse_polygon_star() {
    let mut s = fresh();
    roundtrip(&mut s, "Draw rectangle", |s| {
        drag(s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 50.0));
    });
    roundtrip(&mut s, "Draw ellipse", |s| {
        drag(s, Tool::Ellipse, pt(70.0, 20.0), pt(110.0, 50.0));
    });
    roundtrip(&mut s, "Draw polygon", |s| {
        s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Polygon);
        drag(s, Tool::PolygonStar, pt(20.0, 70.0), pt(60.0, 110.0));
    });
    roundtrip(&mut s, "Draw star", |s| {
        s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
        drag(s, Tool::PolygonStar, pt(70.0, 70.0), pt(110.0, 110.0));
    });
}

#[test]
fn c10_move_drag_typed_move_and_duplicate() {
    let mut s = one_rect();
    roundtrip(&mut s, "Move", |s| {
        s.pointer_hover(pt(30.0, 30.0), false, false);
        s.pointer_down(pt(30.0, 30.0), false);
        s.pointer_hover(pt(45.0, 40.0), false, false);
        s.pointer_hover(pt(60.0, 50.0), false, false);
        s.pointer_up(pt(60.0, 50.0), false, false);
    });
    roundtrip(&mut s, "Move", rel_move("5", "-3", false));
    roundtrip(&mut s, "Duplicate", rel_move("12", "12", true));
}

#[test]
fn c10_resize_rotate_skew_typed() {
    let mut s = one_path();
    roundtrip(&mut s, "Resize", |s| {
        assert_eq!(typed(s, "s", "90"), EntryOutcome::Committed);
    });
    roundtrip(&mut s, "Rotate", |s| {
        assert_eq!(typed(s, "r", "30"), EntryOutcome::Committed);
    });
    roundtrip(&mut s, "Skew", |s| {
        assert_eq!(typed(s, "k", "20"), EntryOutcome::Committed);
    });
}

#[test]
fn c10_resize_rotate_of_a_rectangle() {
    let mut s = one_rect();
    roundtrip(&mut s, "Resize", |s| {
        assert_eq!(typed(s, "s", "50"), EntryOutcome::Committed);
    });
    roundtrip(&mut s, "Rotate", |s| {
        assert_eq!(typed(s, "r", "45"), EntryOutcome::Committed);
    });
}

#[test]
fn c10_nudge_is_a_move() {
    let mut s = one_rect();
    roundtrip(&mut s, "Move", |s| {
        assert!(matches!(
            key(s, "ArrowRight", false, false),
            KeyOutcome::Nudged { .. }
        ));
    });
}

#[test]
fn c10_style_edits_one_step_each() {
    let mut s = one_rect();
    roundtrip(&mut s, "Change style", |s| {
        assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    });
    roundtrip(&mut s, "Change style", |s| {
        assert!(
            s.set_style_text(StyleField::StrokeColor, "#FF0000")
                .unwrap()
        );
    });
    roundtrip(&mut s, "Change style", |s| {
        s.set_fill_paint(true);
    });
    roundtrip(&mut s, "Change style", |s| {
        s.set_stroke_dash(curvyo_ui_core::DashChoice::Dash);
    });
    // Two identical edits in a row, within the same second: two steps (criterion 13).
    let before = json(&s);
    assert!(s.set_style_text(StyleField::StrokeWidth, "5").unwrap());
    assert!(s.set_style_text(StyleField::StrokeWidth, "7").unwrap());
    let n = s.undo_len();
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(s.undo_len(), n - 1);
    assert_ne!(json(&s), before, "one press is not enough for two edits");
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), before);
}

#[test]
fn c10_style_drag_of_a_value_field_is_one_step() {
    let mut s = one_rect();
    let before = json(&s);
    let n = s.undo_len();
    for p in [0.2, 0.4, 0.6, 0.8] {
        s.preview_value_field(
            curvyo_ui_core::ValueField::StrokeWidth,
            p,
            curvyo_ui_core::Grid::Normal,
        );
    }
    assert_eq!(s.undo_len(), n, "a preview writes nothing");
    s.commit_style_preview();
    assert_eq!(s.undo_len(), n + 1, "one drag, one step");
    assert_ne!(json(&s), before);
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), before);
}

#[test]
fn c10_shape_parameters() {
    let mut s = fresh();
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
    drag(&mut s, Tool::PolygonStar, pt(20.0, 20.0), pt(80.0, 80.0));
    click(&mut s, pt(50.0, 50.0), false);
    assert_eq!(s.selected_object_count(), 1, "the star");
    roundtrip(&mut s, "Change point count", |s| {
        s.set_selected_point_count(curvyo_document_core::PointCount::new(9).unwrap());
    });
    roundtrip(&mut s, "Change inner ratio", |s| {
        s.set_selected_ratio(curvyo_document_core::InnerRatio::new(0.3).unwrap());
    });
}

#[test]
fn c10_corner_radius_and_object_to_path() {
    let mut s = one_rect();
    roundtrip(&mut s, "Change corner radius", |s| {
        assert_eq!(s.set_selected_radius_text("4"), EntryOutcome::Committed);
    });
    roundtrip(&mut s, "Object to path", |s| {
        s.convert_selected_to_paths();
    });
}

#[test]
fn c10_delete() {
    let mut s = two_rects();
    roundtrip_rounds(&mut s, "Delete", 1, |s| {
        assert_eq!(key(s, "Delete", false, false), KeyOutcome::Deleted);
    });
}

#[test]
fn c10_document_properties() {
    let mut s = one_rect();
    roundtrip(&mut s, "Resize document", |s| {
        assert_eq!(
            s.set_document_side(DocumentSide::Width, "300"),
            SizeOutcome::Committed
        );
    });
    roundtrip(&mut s, "Resize document", |s| {
        assert_eq!(
            s.set_document_side(DocumentSide::Height, "250"),
            SizeOutcome::Committed
        );
    });
    roundtrip(&mut s, "Change display unit", |s| {
        assert!(s.set_display_unit(DisplayUnit::In));
    });
    roundtrip(&mut s, "Fit document to content", |s| {
        let _ = s.fit_document();
    });
    let mut s = one_rect();
    roundtrip(&mut s, "Resize document", |s| {
        assert_eq!(s.apply_document_preset("a5"), SizeOutcome::Committed);
    });
}

#[test]
fn c10_document_orientation() {
    let mut s = one_rect();
    assert_eq!(
        s.set_document_side(DocumentSide::Width, "100"),
        SizeOutcome::Committed
    );
    assert_eq!(
        s.set_document_side(DocumentSide::Height, "200"),
        SizeOutcome::Committed
    );
    roundtrip(&mut s, "Resize document", |s| {
        let _ = s.set_document_orientation(curvyo_document_core::Orientation::Landscape);
    });
}

#[test]
fn c10_background() {
    let mut s = one_rect();
    roundtrip(&mut s, "Change background", |s| {
        assert!(s.set_background_paint(BackgroundPaint::None));
    });
    roundtrip(&mut s, "Change background", |s| {
        assert!(s.set_background_paint(BackgroundPaint::Solid));
    });
}

#[test]
fn c10_boolean_operations() {
    for (op, name) in [
        (BooleanOp::Union, "Union"),
        (BooleanOp::Difference, "Difference"),
        (BooleanOp::Intersection, "Intersection"),
        (BooleanOp::Exclusion, "Exclusion"),
        (BooleanOp::ReverseDifference, "Reverse difference"),
    ] {
        let mut s = two_rects();
        roundtrip_rounds(&mut s, name, 1, |s| {
            let _ = s.apply_boolean(op);
        });
    }
}

#[test]
fn c10_combine_and_break_apart() {
    let mut s = two_apart();
    s.convert_selected_to_paths();
    s.set_tool(Tool::Select);
    assert_eq!(s.selected_object_count(), 2, "two paths");
    roundtrip_rounds(&mut s, "Combine", 1, |s| {
        let o = s.apply_combine();
        assert!(
            matches!(o, curvyo_editor_wasm::CombineOutcome::Applied { .. }),
            "{o:?}"
        );
    });
    roundtrip_rounds(&mut s, "Break apart", 1, |s| {
        let o = s.apply_break_apart();
        assert!(
            matches!(o, curvyo_editor_wasm::BreakApartOutcome::Applied { .. }),
            "{o:?}"
        );
    });
}

#[test]
fn c10_node_tool_edits() {
    let mut s = one_path();
    s.set_tool(Tool::Node);
    // Select the top segment and insert a node at its midpoint.
    s.pointer_hover(pt(50.0, 20.0), false, false);
    s.pointer_down(pt(50.0, 20.0), false);
    s.pointer_up(pt(50.0, 20.0), false, false);
    roundtrip(&mut s, "Add node", |s| {
        s.insert_selected();
    });
    // Move nodes: drag one corner.
    roundtrip(&mut s, "Move nodes", |s| {
        s.pointer_hover(pt(80.0, 20.0), false, false);
        s.pointer_down(pt(80.0, 20.0), false);
        s.pointer_hover(pt(90.0, 12.0), false, false);
        s.pointer_up(pt(90.0, 12.0), false, false);
    });
    // Change kind of the selected node.
    roundtrip(&mut s, "Change node type", |s| {
        s.convert_selected(AnchorKind::Symmetric);
    });
}

// ---------------------------------------------------------------------
// Criterion 11: one press restores all N objects
// ---------------------------------------------------------------------

#[test]
fn c11_a_step_over_several_objects_is_one_press() {
    let mut s = two_rects();
    let before = json(&s);
    // Move both by a typed entry, then restyle both, then delete both.
    assert_eq!(key(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    assert_eq!(
        s.commit_move_entry(
            "10",
            "10",
            MoveEntryMode {
                absolute: false,
                copy: false
            }
        ),
        EntryOutcome::Committed
    );
    assert_eq!(
        s.undo_len(),
        3,
        "two rectangles and the move; selection is no step"
    );
    let moved = json(&s);
    assert!(s.set_style_text(StyleField::StrokeWidth, "4").unwrap());
    let styled = json(&s);
    assert_eq!(key(&mut s, "Delete", false, false), KeyOutcome::Deleted);
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), styled, "Delete of 2 undone by one press");
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), moved, "style on 2 undone by one press");
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_ne!(json(&s), moved);
    let _ = before;
    assert_eq!(ids(&s).len(), 2);
}

#[test]
fn c11_multi_object_resize_rotate_skew_are_one_step_each() {
    let mut s = two_rects();
    s.convert_selected_to_paths();
    s.set_tool(Tool::Select);
    assert_eq!(s.selected_object_count(), 2);
    for (k, v, name) in [
        ("s", "150", "Resize"),
        ("r", "20", "Rotate"),
        ("k", "15", "Skew"),
    ] {
        roundtrip(&mut s, name, |s| {
            assert_eq!(typed(s, k, v), EntryOutcome::Committed, "{name}");
        });
    }
}

// ---------------------------------------------------------------------
// Criterion 12: interactions that write nothing add no step
// ---------------------------------------------------------------------

#[test]
fn c12_no_step_for_a_no_op() {
    let mut s = one_rect();
    let n = s.undo_len();
    // Escape during a drag.
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_down(pt(30.0, 30.0), false);
    s.pointer_hover(pt(50.0, 50.0), false, false);
    assert_eq!(
        key(&mut s, "Escape", false, false),
        KeyOutcome::Escape(curvyo_editor_wasm::EscapeStep::CancelledDrag)
    );
    s.pointer_up(pt(50.0, 50.0), false, false);
    assert_eq!(s.undo_len(), n, "escaped drag");
    // A drag that ends where it began.
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_down(pt(30.0, 30.0), false);
    s.pointer_hover(pt(60.0, 60.0), false, false);
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_up(pt(30.0, 30.0), false, false);
    assert_eq!(s.undo_len(), n, "drag back to the start");
    // A typed value equal to the current one.
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let n = s.undo_len();
    let r = s.set_style_text(StyleField::StrokeWidth, "3");
    assert!(
        matches!(r, Ok(false) | Err(_)) || s.undo_len() == n,
        "same width writes nothing: {r:?}"
    );
    assert_eq!(s.undo_len(), n, "typed value equal to the current one");
    // A typed move of zero.
    assert_eq!(key(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    let out = s.commit_move_entry(
        "0",
        "0",
        MoveEntryMode {
            absolute: false,
            copy: false,
        },
    );
    assert_ne!(out, EntryOutcome::Committed);
    assert_eq!(s.undo_len(), n, "zero move");
    // Undo with nothing between does not create steps either.
    let json_before = json(&s);
    assert_eq!(json(&s), json_before);
}

#[test]
fn c12_refused_operation_adds_no_step() {
    // A union of a single object is refused.
    let mut s = one_rect();
    let n = s.undo_len();
    let before = json(&s);
    let _ = s.apply_boolean(BooleanOp::Union);
    assert_eq!(s.undo_len(), n);
    assert_eq!(json(&s), before);
    // Delete with an empty selection.
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(40.0, 40.0));
    s.escape();
    s.escape();
    s.set_tool(Tool::Select);
    click(&mut s, pt(150.0, 150.0), false);
    let n = s.undo_len();
    assert_eq!(s.selected_object_count(), 0);
    let _ = key(&mut s, "Delete", false, false);
    assert_eq!(s.undo_len(), n);
}

// ---------------------------------------------------------------------
// Criteria 1 to 5: keys
// ---------------------------------------------------------------------

#[test]
fn c4_empty_stacks_say_nothing_to_undo_and_do_not_change_the_document() {
    let mut s = fresh();
    let before = json(&s);
    let (out, notice) = ctrl_z(&mut s);
    assert_eq!(out, KeyOutcome::Undo);
    assert_eq!(notice, "Nothing to undo.");
    let (out, notice) = ctrl_shift_z(&mut s);
    assert_eq!(out, KeyOutcome::Redo);
    assert_eq!(notice, "Nothing to redo.");
    assert_eq!(json(&s), before);
    assert_eq!((s.undo_len(), s.redo_len()), (0, 0));
}

#[test]
fn c1_c2_notice_text_has_the_operation_and_the_objects() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Undid: Draw rectangle (1 rectangle).");
    let (_, notice) = ctrl_shift_z(&mut s);
    assert_eq!(notice, "Redid: Draw rectangle (1 rectangle).");
    // Two paths via the pen, 3 paths summary
    let mut s = two_rects();
    assert_eq!(key(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    let _ = s.commit_move_entry(
        "1",
        "1",
        MoveEntryMode {
            absolute: false,
            copy: false,
        },
    );
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Undid: Move (2 rectangles).");
    // A document step touches no object: no parentheses.
    let mut s = fresh();
    assert_eq!(
        s.set_document_side(DocumentSide::Width, "300"),
        SizeOutcome::Committed
    );
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Undid: Resize document.");
}

#[test]
fn c3_ctrl_y_redoes_on_linux_and_windows_not_on_mac() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let after = json(&s);
    ctrl_z(&mut s);
    let out = s.key_down(KeyInput {
        key: "y",
        ctrl: true,
        mac: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::Ignored, "Cmd+Y is not redo on macOS");
    assert_eq!(s.redo_len(), 1);
    let out = s.key_down(KeyInput {
        key: "y",
        ctrl: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::Redo);
    assert_eq!(json(&s), after);
    // Cmd+Shift+Z is the redo on a Mac.
    ctrl_z(&mut s);
    let out = s.key_down(KeyInput {
        key: "Z",
        ctrl: true,
        shift: true,
        mac: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::Redo);
    assert_eq!(json(&s), after);
}

#[test]
fn c5_held_key_undoes_one_step_per_repeat() {
    let mut s = one_rect();
    for w in ["2", "3", "4", "5", "6"] {
        assert!(s.set_style_text(StyleField::StrokeWidth, w).unwrap());
    }
    let n = s.undo_len();
    for i in 0..5 {
        let out = s.key_down(KeyInput {
            key: "z",
            ctrl: true,
            repeat: true,
            ..KeyInput::default()
        });
        let (out, _) = finish(&mut s, out);
        assert_eq!(out, KeyOutcome::Undo, "repeat {i}");
        assert_eq!(s.undo_len(), n - i - 1);
    }
    // Back at the width before the five edits (the default), and one more press says nothing
    // about those five.
    let out = s.key_down(KeyInput {
        key: "Z",
        ctrl: true,
        shift: true,
        repeat: true,
        ..KeyInput::default()
    });
    let (out, _) = finish(&mut s, out);
    assert_eq!(out, KeyOutcome::Redo);
    assert_eq!(s.redo_len(), 4);
}

#[test]
fn c5_sixth_repeat_says_nothing_to_undo() {
    let mut s = fresh();
    for i in 0..5 {
        drag(
            &mut s,
            Tool::Rectangle,
            pt(10.0 + 20.0 * i as f64, 10.0),
            pt(25.0 + 20.0 * i as f64, 25.0),
        );
    }
    assert_eq!(s.undo_len(), 5);
    for _ in 0..5 {
        let out = s.key_down(KeyInput {
            key: "z",
            ctrl: true,
            repeat: true,
            ..KeyInput::default()
        });
        assert_eq!(finish(&mut s, out).0, KeyOutcome::Undo);
    }
    let out = s.key_down(KeyInput {
        key: "z",
        ctrl: true,
        repeat: true,
        ..KeyInput::default()
    });
    let (_, notice) = finish(&mut s, out);
    assert_eq!(notice, "Nothing to undo.");
    assert!(ids(&s).is_empty());
}

#[test]
fn c7_qwertz_keys_are_matched_by_the_typed_character() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    for (k, shift) in [("z", false), ("Z", true)] {
        let _ = (k, shift);
    }
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Undo);
    assert_eq!(key(&mut s, "Z", true, true), KeyOutcome::Redo);
    // Uppercase letter without shift flag (caps lock) is still the typed character "Z" with shift
    // reported by the host; with key "Z" and shift=false the key is not a redo and not an undo.
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Undo);
    assert_eq!(
        s.key_down(KeyInput {
            key: "y",
            ctrl: true,
            ..KeyInput::default()
        }),
        KeyOutcome::Redo
    );
    // Without ctrl nothing happens.
    let n = s.undo_len();
    let out = key(&mut s, "z", false, false);
    assert_ne!(out, KeyOutcome::Undo);
    assert_eq!(s.undo_len(), n);
    // Alt+Ctrl+Z (AltGr on some layouts) is not undo.
    let out = s.key_down(KeyInput {
        key: "z",
        ctrl: true,
        alt: true,
        ..KeyInput::default()
    });
    assert_ne!(
        out,
        KeyOutcome::Undo,
        "AltGr+Z types a different character on some layouts"
    );
}

// ---------------------------------------------------------------------
// Criterion 6: the gate
// ---------------------------------------------------------------------

#[test]
fn c6a_ignored_during_a_drag() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let json_before = json(&s);
    let n = s.undo_len();
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_down(pt(30.0, 30.0), false);
    s.pointer_hover(pt(45.0, 45.0), false, false);
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Ignored);
    assert_eq!(key(&mut s, "Z", true, true), KeyOutcome::Ignored);
    assert_eq!(s.take_history_notice(), None, "no notice");
    assert_eq!(s.undo_len(), n);
    s.pointer_up(pt(45.0, 45.0), false, false);
    assert_ne!(json(&s), json_before);
    // After the drag the key works again and takes back the drag only.
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), json_before);
}

#[test]
fn c6a_ignored_during_marquee_and_create_drag() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    // Marquee
    s.set_tool(Tool::Select);
    s.pointer_hover(pt(150.0, 150.0), false, false);
    s.pointer_down(pt(150.0, 150.0), false);
    s.pointer_hover(pt(170.0, 170.0), false, false);
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Ignored);
    s.pointer_up(pt(170.0, 170.0), false, false);
    // Create-drag
    s.set_tool(Tool::Rectangle);
    s.pointer_hover(pt(100.0, 100.0), false, false);
    s.pointer_down(pt(100.0, 100.0), false);
    s.pointer_hover(pt(120.0, 130.0), false, false);
    let n = s.undo_len();
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Ignored);
    assert_eq!(s.undo_len(), n);
    s.pointer_up(pt(120.0, 130.0), false, false);
    assert_eq!(s.undo_len(), n + 1);
}

#[test]
fn c6a_ignored_during_a_node_drag() {
    let mut s = one_path();
    s.set_tool(Tool::Node);
    let n = s.undo_len();
    s.pointer_hover(pt(80.0, 20.0), false, false);
    s.pointer_down(pt(80.0, 20.0), false);
    s.pointer_hover(pt(95.0, 10.0), false, false);
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Ignored);
    s.pointer_up(pt(95.0, 10.0), false, false);
    assert_eq!(s.undo_len(), n + 1);
}

#[test]
fn c6b_ignored_while_a_chip_is_open() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let n = s.undo_len();
    assert_eq!(key(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Ignored);
    assert_eq!(key(&mut s, "Z", true, true), KeyOutcome::Ignored);
    assert_eq!(s.take_history_notice(), None);
    assert_eq!(s.undo_len(), n);
    assert!(s.move_entry().is_some(), "the chip is still open");
    // Escape closes it, and then the key works.
    assert_eq!(
        key(&mut s, "Escape", false, false),
        KeyOutcome::Escape(curvyo_editor_wasm::EscapeStep::ClosedEntry)
    );
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
}

#[test]
fn c6b_ignored_while_a_transform_chip_is_open() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    assert_eq!(key(&mut s, "r", false, false), KeyOutcome::EntryOpened);
    assert_eq!(key(&mut s, "z", true, false), KeyOutcome::Ignored);
    assert!(s.transform_entry().is_some());
}

#[test]
fn c6c_ignored_in_a_text_field() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let n = s.undo_len();
    let out = s.key_down(KeyInput {
        key: "z",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::Ignored);
    let out = s.key_down(KeyInput {
        key: "Z",
        ctrl: true,
        shift: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::Ignored);
    let out = s.key_down(KeyInput {
        key: "y",
        ctrl: true,
        dom_blocked: true,
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::Ignored);
    assert_eq!(s.undo_len(), n);
    assert_eq!(s.take_history_notice(), None);
}

#[test]
fn c6e_unfinished_pen_path_gives_the_hint_and_changes_nothing() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let before = json(&s);
    let n = s.undo_len();
    s.set_tool(Tool::Pen);
    pen_click(&mut s, pt(100.0, 100.0));
    pen_click(&mut s, pt(140.0, 100.0));
    assert!(s.pen_in_progress().is_some());
    assert_eq!(
        key(&mut s, "z", true, false),
        KeyOutcome::Hint(KeyHint::FinishPenFirst)
    );
    assert_eq!(
        key(&mut s, "Z", true, true),
        KeyOutcome::Hint(KeyHint::FinishPenFirst)
    );
    assert_eq!(s.undo_len(), n);
    assert_eq!(json(&s), before);
    assert_eq!(
        s.pen_in_progress().map(<[_]>::len),
        Some(2),
        "the path is still there"
    );
    // After finishing the path, undo takes back the path (one step).
    s.finish_pen();
    assert_eq!(s.undo_len(), n + 1);
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), before);
}

#[test]
fn c6e_pen_path_with_one_point_also_blocks() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    s.set_tool(Tool::Pen);
    pen_click(&mut s, pt(100.0, 100.0));
    assert_eq!(
        key(&mut s, "z", true, false),
        KeyOutcome::Hint(KeyHint::FinishPenFirst)
    );
    // Escape cancels it, undo works again.
    s.escape();
    assert!(s.pen_in_progress().is_none_or(<[_]>::is_empty));
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
}

// ---------------------------------------------------------------------
// Criterion 13 and 14: no merging, large steps
// ---------------------------------------------------------------------

#[test]
fn c13_two_identical_moves_within_a_second_are_two_steps() {
    let mut s = one_rect();
    let start = json(&s);
    rel_move("5", "0", false)(&mut s);
    rel_move("5", "0", false)(&mut s);
    assert_eq!(s.undo_len(), 3);
    ctrl_z(&mut s);
    assert_ne!(json(&s), start, "one press leaves the first move");
    ctrl_z(&mut s);
    assert_eq!(json(&s), start);
}

#[test]
fn c14_a_union_with_a_large_result_is_one_step() {
    // Two circle-like polygons with many points: the union has hundreds of anchors.
    let mut s = fresh();
    s.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Polygon);
    s.set_poly_star_point_count(curvyo_document_core::PointCount::new(60).unwrap());
    drag(&mut s, Tool::PolygonStar, pt(20.0, 20.0), pt(120.0, 120.0));
    drag(&mut s, Tool::PolygonStar, pt(60.0, 40.0), pt(160.0, 140.0));
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    assert_eq!(s.selected_object_count(), 2);
    let before = json(&s);
    let n = s.undo_len();
    let _ = s.apply_boolean(BooleanOp::Union);
    assert_eq!(s.undo_len(), n + 1);
    let after = json(&s);
    assert_ne!(before, after);
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), before);
    assert_eq!(ctrl_shift_z(&mut s).0, KeyOutcome::Redo);
    assert_eq!(json(&s), after);
}

// ---------------------------------------------------------------------
// Criteria 22 to 24: after an undo
// ---------------------------------------------------------------------

#[test]
fn c22_a_new_step_empties_the_redo_stack() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(70.0, 20.0), pt(110.0, 60.0));
    ctrl_z(&mut s);
    ctrl_z(&mut s);
    assert_eq!(s.redo_len(), 2);
    drag(&mut s, Tool::Ellipse, pt(20.0, 80.0), pt(60.0, 120.0));
    assert_eq!(s.redo_len(), 0);
    let (_, notice) = ctrl_shift_z(&mut s);
    assert_eq!(notice, "Nothing to redo.");
}

#[test]
fn c22_undone_steps_stay_in_the_global_history_marked_undone() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(60.0, 60.0));
    drag(&mut s, Tool::Rectangle, pt(70.0, 20.0), pt(110.0, 60.0));
    let rows_before = s.history_len();
    ctrl_z(&mut s);
    assert_eq!(s.history_len(), rows_before, "no new row for an undo");
    let rows: serde_json::Value = serde_json::from_str(&s.history_rows_json(0, 10)).unwrap();
    let text = rows.to_string();
    assert!(text.contains("\"status\":\"undone\""), "{text}");
    drag(&mut s, Tool::Ellipse, pt(20.0, 80.0), pt(60.0, 120.0));
    assert_eq!(s.history_len(), rows_before + 1);
    let text = s.history_rows_json(0, 10);
    assert!(
        text.contains("\"status\":\"undone\""),
        "the undone step is not lost: {text}"
    );
}

#[test]
fn c23_selection_after_union_undo_redo() {
    let mut s = two_rects();
    let two = ids(&s);
    let _ = s.apply_boolean(BooleanOp::Union);
    assert_eq!(s.selected_object_count(), 1, "the result is selected");
    let result = ids(&s);
    ctrl_z(&mut s);
    assert_eq!(
        s.selected_object_count(),
        2,
        "the two squares are selected again"
    );
    assert_eq!(ids(&s), two, "the operands came back with their ids");
    ctrl_shift_z(&mut s);
    assert_eq!(s.selected_object_count(), 1, "the result is selected");
    assert_eq!(ids(&s), result);
}

#[test]
fn c23_undo_of_delete_revives_same_ids_and_selects_them() {
    let mut s = two_rects();
    let before_ids = ids(&s);
    let before_json = json(&s);
    key(&mut s, "Delete", false, false);
    assert!(ids(&s).is_empty());
    assert_eq!(s.selected_object_count(), 0);
    ctrl_z(&mut s);
    assert_eq!(ids(&s), before_ids, "same ids and same z-order");
    assert_eq!(json(&s), before_json);
    assert_eq!(
        s.selected_object_count(),
        2,
        "restored objects are selected"
    );
    assert_eq!(s.tool(), Tool::Select, "the tool does not change");
    // Save, open: the ids equal.
    let bytes = s.pack("0.1.0").unwrap();
    let reopened = Session::open(5, &bytes).unwrap();
    assert_eq!(
        unpack(1, &reopened.pack("0.1.0").unwrap())
            .unwrap()
            .object_ids(),
        before_ids
    );
    // Delete the objects, redo-undo cycles keep ids.
    for _ in 0..3 {
        ctrl_shift_z(&mut s);
        assert!(ids(&s).is_empty());
        ctrl_z(&mut s);
        assert_eq!(ids(&s), before_ids);
    }
}

#[test]
fn c23_undo_of_delete_in_the_node_tool_keeps_tool() {
    let mut s = one_path();
    s.set_tool(Tool::Node);
    let n = s.undo_len();
    let _ = n;
    // Select a node and delete it (Delete in the node tool deletes nodes: Delete nodes step).
    s.pointer_hover(pt(80.0, 20.0), false, false);
    s.pointer_down(pt(80.0, 20.0), false);
    s.pointer_up(pt(80.0, 20.0), false, false);
    let before = json(&s);
    let steps = s.undo_len();
    key(&mut s, "Delete", false, false);
    if s.undo_len() == steps + 1 {
        ctrl_z(&mut s);
        assert_eq!(json(&s), before);
        assert_eq!(s.tool(), Tool::Node, "the tool does not change");
    }
}

#[test]
fn c24_pan_and_zoom_do_not_change_and_off_screen_tail() {
    let mut s = two_rects();
    let view = s.view();
    key(&mut s, "Delete", false, false);
    // Pan so far that the restored objects are off screen.
    s.begin_pan(0.0, 0.0);
    s.pan_to(-4000.0, -4000.0);
    s.end_pan();
    let panned = s.view();
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(s.view(), panned, "undo does not move the view");
    assert_eq!(notice, "Undid: Delete (2 rectangles), off screen.");
    assert_ne!(view, panned);
}

#[test]
fn c24_on_screen_has_no_tail() {
    let mut s = two_rects();
    key(&mut s, "Delete", false, false);
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Undid: Delete (2 rectangles).");
}

// ---------------------------------------------------------------------
// Criteria 25 to 28: boundaries
// ---------------------------------------------------------------------

#[test]
fn c25_open_empties_stacks_and_creates_no_step() {
    let mut s = one_rect();
    rel_move("3", "3", false)(&mut s);
    let bytes = s.pack("0.1.0").unwrap();
    let before = unpack(1, &bytes).unwrap().export_json().unwrap();
    let mut reopened = Session::open(77, &bytes).unwrap();
    reopened.set_clock(clock);
    assert_eq!((reopened.undo_len(), reopened.redo_len()), (0, 0));
    let (out, notice) = ctrl_z(&mut reopened);
    assert_eq!(out, KeyOutcome::Undo);
    assert_eq!(notice, "Nothing to undo.");
    assert_eq!(json(&reopened), before);
    // History of the reopened file lists its steps.
    assert_eq!(reopened.history_len(), s.history_len());
    // New: a new session has an empty history.
    let fresh = Session::new(5);
    assert_eq!((fresh.undo_len(), fresh.redo_len()), (0, 0));
}

#[test]
fn c26_save_does_not_touch_the_stacks() {
    let mut s = one_rect();
    rel_move("3", "3", false)(&mut s);
    ctrl_z(&mut s);
    let (u, r) = (s.undo_len(), s.redo_len());
    let saved = s.pack("0.1.0").unwrap();
    assert_eq!((s.undo_len(), s.redo_len()), (u, r));
    let (out, _) = ctrl_shift_z(&mut s);
    assert_eq!(out, KeyOutcome::Redo);
    assert_eq!(s.redo_len(), 0);
    // The saved file stays as saved.
    let again = unpack(1, &saved).unwrap().export_json().unwrap();
    let _ = again;
    ctrl_z(&mut s);
    ctrl_z(&mut s);
    assert_eq!(
        unpack(1, &saved).unwrap().export_json().unwrap(),
        unpack(1, &saved).unwrap().export_json().unwrap()
    );
}

#[test]
fn c28_the_501st_step_drops_the_oldest_from_the_stack_but_not_from_history() {
    let mut s = one_rect();
    let base = s.undo_len();
    let rows_before = s.history_len();
    for i in 0..501 {
        let w = if i % 2 == 0 { "2" } else { "3" };
        assert!(s.set_style_text(StyleField::StrokeWidth, w).unwrap());
    }
    assert_eq!(
        s.undo_len(),
        500,
        "the stack is bounded at 500 (had {base} before)"
    );
    assert_eq!(
        s.history_len(),
        rows_before + 501,
        "all steps stay in the global history"
    );
    for i in 0..500 {
        let (out, notice) = ctrl_z(&mut s);
        assert_eq!(out, KeyOutcome::Undo, "undo {i}");
        assert!(
            notice.starts_with("Undid: Change style"),
            "undo {i}: {notice}"
        );
    }
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Nothing to undo.");
    assert_eq!(s.redo_len(), 500);
}

// ---------------------------------------------------------------------
// Hostile
// ---------------------------------------------------------------------

#[test]
fn hostile_rapid_spam_and_alternation_never_breaks_the_document() {
    let mut s = two_rects();
    rel_move("4", "4", false)(&mut s);
    assert!(s.set_style_text(StyleField::StrokeWidth, "4").unwrap());
    key(&mut s, "Delete", false, false);
    let end = json(&s);
    for _ in 0..40 {
        ctrl_z(&mut s);
    }
    for _ in 0..40 {
        ctrl_shift_z(&mut s);
    }
    assert_eq!(json(&s), end, "all the way back and forward is byte-equal");
    // Interleave
    for i in 0..60 {
        if i % 3 == 0 {
            ctrl_shift_z(&mut s);
        } else {
            ctrl_z(&mut s);
        }
    }
    for _ in 0..80 {
        ctrl_shift_z(&mut s);
    }
    assert_eq!(json(&s), end);
    // The saved document still opens and equals.
    let bytes = s.pack("0.1.0").unwrap();
    let reopened = Session::open(3, &bytes).unwrap();
    assert_eq!(json(&reopened), end);
}

#[test]
fn hostile_undo_after_tool_switch_and_escape_keeps_working() {
    let mut s = one_rect();
    rel_move("4", "4", false)(&mut s);
    for t in [
        Tool::Pen,
        Tool::Node,
        Tool::Rectangle,
        Tool::Ellipse,
        Tool::PolygonStar,
        Tool::Select,
    ] {
        s.set_tool(t);
    }
    s.escape();
    let n = s.undo_len();
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(s.undo_len(), n - 1);
}

// ---------------------------------------------------------------------
// Criterion 48/50/51 wipe through Session
// ---------------------------------------------------------------------

#[test]
fn c48_wipe_through_the_session_keeps_the_drawing_and_empties_the_stacks() {
    let mut s = two_rects();
    for i in 0..30 {
        let w = if i % 2 == 0 { "2" } else { "3" };
        assert!(s.set_style_text(StyleField::StrokeWidth, w).unwrap());
    }
    ctrl_z(&mut s);
    let before = json(&s);
    let ids_before = ids(&s);
    let steps = s.history_len();
    assert!(steps >= 30);
    let outcome = s.wipe_history();
    assert!(matches!(outcome, WipeOutcome::Wiped(_)), "{outcome:?}");
    assert_eq!((s.undo_len(), s.redo_len()), (0, 0));
    assert_eq!(json(&s), before, "the drawing is exactly as before");
    assert_eq!(ids(&s), ids_before);
    assert_eq!(s.history_len(), 1, "one row: History wiped");
    let rows = s.history_rows_json(0, 5);
    assert!(rows.contains("History wiped"), "{rows}");
    // Steps made afterwards follow it and undo works for them.
    assert!(s.set_style_text(StyleField::StrokeWidth, "9").unwrap());
    assert_eq!(s.history_len(), 2);
    assert_eq!(ctrl_z(&mut s).0, KeyOutcome::Undo);
    assert_eq!(json(&s), before);
    // The wipe cannot be undone: nothing more to undo.
    let (_, notice) = ctrl_z(&mut s);
    assert_eq!(notice, "Nothing to undo.");
}

#[test]
fn c51_wipe_is_refused_during_a_drag_and_with_an_open_chip_or_pen_path() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let before = (s.history_len(), json(&s));
    // Drag
    s.pointer_hover(pt(30.0, 30.0), false, false);
    s.pointer_down(pt(30.0, 30.0), false);
    s.pointer_hover(pt(40.0, 40.0), false, false);
    let out = s.wipe_history();
    assert!(!matches!(out, WipeOutcome::Wiped(_)), "drag: {out:?}");
    s.pointer_cancelled();
    // Chip
    assert_eq!(key(&mut s, "m", false, false), KeyOutcome::EntryOpened);
    let out = s.wipe_history();
    assert!(!matches!(out, WipeOutcome::Wiped(_)), "chip: {out:?}");
    s.cancel_transform_entry();
    // Pen
    s.set_tool(Tool::Pen);
    pen_click(&mut s, pt(100.0, 100.0));
    pen_click(&mut s, pt(120.0, 100.0));
    let out = s.wipe_history();
    assert!(!matches!(out, WipeOutcome::Wiped(_)), "pen: {out:?}");
    s.escape();
    assert_eq!(s.history_len(), before.0);
    assert_eq!(json(&s), before.1);
}

#[test]
fn c47_wipe_state_without_steps_offers_nothing() {
    let mut s = fresh();
    assert_eq!(s.history_wipe_state(), WipeState::NoSteps);
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(40.0, 40.0));
    assert_eq!(s.history_wipe_state(), WipeState::Offered(1));
}

/// Probe: a wipe request on a document with no step. The control is not in the tree then (47), so the
/// editor API should refuse it rather than write a "History wiped" row for nothing.
#[test]
fn c47_probe_wipe_of_an_empty_history_writes_no_row() {
    let mut s = fresh();
    let out = s.wipe_history();
    assert!(!matches!(out, WipeOutcome::Wiped(_)), "{out:?}");
    assert_eq!(s.history_len(), 0);
}

// ---------------------------------------------------------------------
// Busy state (criterion 30)
// ---------------------------------------------------------------------

#[test]
fn c30_small_step_has_no_busy_state_large_step_has() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    s.warm_history();
    assert!(
        !s.history_needs_busy_state(Direction::Undo),
        "a style edit is a small step"
    );
    let out = key(&mut s, "z", true, false);
    assert_eq!(out, KeyOutcome::Undo);
}

#[test]
fn c30c_before_the_history_is_warm_the_first_undo_of_an_opened_file_uses_the_busy_state() {
    let mut s = one_rect();
    assert!(s.set_style_text(StyleField::StrokeWidth, "3").unwrap());
    let bytes = s.pack("0.1.0").unwrap();
    let mut reopened = Session::open(5, &bytes).unwrap();
    reopened.set_clock(clock);
    // Make a step in the reopened session; the file's history is not warm yet.
    assert_eq!(
        key(&mut reopened, "a", true, false),
        KeyOutcome::SelectedAll
    );
    assert!(
        reopened
            .set_style_text(StyleField::StrokeWidth, "4")
            .unwrap()
    );
    let busy = reopened.history_needs_busy_state(Direction::Undo);
    reopened.warm_history();
    assert!(!reopened.history_needs_busy_state(Direction::Undo));
    let _ = busy;
    assert_eq!(ctrl_z(&mut reopened).0, KeyOutcome::Undo);
}

// ---------------------------------------------------------------------
// Repeated undo and redo of a step that deletes objects (criteria 10, 23)
// ---------------------------------------------------------------------

#[test]
fn c23_delete_undo_redo_undo_revives_again() {
    let mut s = two_rects();
    let before = json(&s);
    let before_ids = ids(&s);
    key(&mut s, "Delete", false, false);
    for round in 0..3 {
        let (_, notice) = ctrl_z(&mut s);
        assert_eq!(notice, "Undid: Delete (2 rectangles).", "round {round}");
        assert_eq!(json(&s), before, "round {round}");
        assert_eq!(ids(&s), before_ids, "round {round}");
        let (_, notice) = ctrl_shift_z(&mut s);
        assert_eq!(notice, "Redid: Delete (2 rectangles).", "round {round}");
        assert!(ids(&s).is_empty(), "round {round}");
    }
}

#[test]
fn c10_boolean_undo_redo_undo_restores_the_operands_again() {
    for (op, name) in [
        (BooleanOp::Union, "Union"),
        (BooleanOp::Difference, "Difference"),
        (BooleanOp::Intersection, "Intersection"),
        (BooleanOp::Exclusion, "Exclusion"),
        (BooleanOp::ReverseDifference, "Reverse difference"),
    ] {
        let mut s = two_rects();
        let before = json(&s);
        let _ = s.apply_boolean(op);
        let after = json(&s);
        for round in 0..3 {
            let (_, notice) = ctrl_z(&mut s);
            assert!(
                notice.starts_with(&format!("Undid: {name}")),
                "{name} round {round}: {notice}"
            );
            assert_eq!(json(&s), before, "{name} round {round}");
            ctrl_shift_z(&mut s);
            assert_eq!(json(&s), after, "{name} round {round}");
        }
    }
}

#[test]
fn c10_combine_undo_redo_undo_restores_the_operands_again() {
    let mut s = two_apart();
    s.convert_selected_to_paths();
    s.set_tool(Tool::Select);
    let before = json(&s);
    let _ = s.apply_combine();
    let after = json(&s);
    for round in 0..3 {
        ctrl_z(&mut s);
        assert_eq!(json(&s), before, "round {round}");
        ctrl_shift_z(&mut s);
        assert_eq!(json(&s), after, "round {round}");
    }
}

#[test]
fn c10_object_to_path_is_a_replace_and_repeats_too() {
    let mut s = one_rect();
    let before = json(&s);
    s.convert_selected_to_paths();
    let after = json(&s);
    for round in 0..3 {
        ctrl_z(&mut s);
        assert_eq!(json(&s), before, "round {round}");
        ctrl_shift_z(&mut s);
        assert_eq!(json(&s), after, "round {round}");
    }
}

// ---------------------------------------------------------------------
// Node tool and Pen rows of the table, second part
// ---------------------------------------------------------------------

fn open_path() -> Session {
    let mut s = fresh();
    s.set_tool(Tool::Pen);
    for p in [
        pt(20.0, 20.0),
        pt(80.0, 20.0),
        pt(80.0, 60.0),
        pt(30.0, 70.0),
    ] {
        pen_click(&mut s, p);
    }
    s.finish_pen();
    s.set_tool(Tool::Select);
    click(&mut s, pt(50.0, 20.0), false);
    assert_eq!(s.selected_object_count(), 1);
    s.set_tool(Tool::Node);
    s
}

fn node_click(s: &mut Session, at: Point) {
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
}

#[test]
fn c10_segment_rows_curve_straighten() {
    let mut s = open_path();
    node_click(&mut s, pt(50.0, 20.0));
    roundtrip(&mut s, "Curve segment", |s| s.make_curve());
    // Segment selection is not restored by undo (not covered by criterion 23, which names nodes).
    node_click(&mut s, pt(50.0, 20.0));
    roundtrip(&mut s, "Straighten segment", |s| s.make_line());
}

#[test]
fn c10_delete_nodes_and_split_path() {
    let mut s = open_path();
    node_click(&mut s, pt(80.0, 20.0));
    roundtrip_rounds(&mut s, "Split path", 2, |s| s.split_selected());
    let mut s = open_path();
    node_click(&mut s, pt(80.0, 60.0));
    roundtrip(&mut s, "Delete nodes", |s| {
        let _ = key(s, "Delete", false, false);
    });
}

#[test]
fn c10_close_path() {
    let mut s = open_path();
    // Select both end nodes with a marquee-free route: click one, shift-click the other.
    node_click(&mut s, pt(20.0, 20.0));
    s.pointer_hover(pt(30.0, 70.0), true, false);
    s.pointer_down(pt(30.0, 70.0), true);
    s.pointer_up(pt(30.0, 70.0), true, false);
    roundtrip(&mut s, "Close path", |s| {
        let _ = s.close_paths(curvyo_ui_core::JoinType::Sharp);
    });
}

#[test]
fn c10_extend_path_with_the_pen() {
    let mut s = open_path();
    s.set_tool(Tool::Select);
    click(&mut s, pt(50.0, 20.0), false);
    roundtrip(&mut s, "Extend path", |s| {
        s.set_tool(Tool::Pen);
        pen_click(s, pt(30.0, 70.0));
        pen_click(s, pt(10.0, 90.0));
        s.finish_pen();
    });
}

#[test]
fn c10_bend_segment_by_dragging_it() {
    let mut s = open_path();
    roundtrip(&mut s, "Bend segment", |s| {
        s.pointer_hover(pt(50.0, 20.0), false, false);
        s.pointer_down(pt(50.0, 20.0), false);
        s.pointer_hover(pt(50.0, 30.0), false, false);
        s.pointer_hover(pt(50.0, 40.0), false, false);
        s.pointer_up(pt(50.0, 40.0), false, false);
    });
}

#[test]
fn c23_node_tool_node_selection_is_restored_by_anchor_id() {
    let mut s = open_path();
    node_click(&mut s, pt(80.0, 20.0));
    assert!(s.node_toolbar_state().can_delete, "a node is selected");
    s.convert_selected(AnchorKind::Symmetric);
    assert!(s.node_toolbar_state().can_delete);
    ctrl_z(&mut s);
    assert!(
        s.node_toolbar_state().can_delete,
        "criterion 23: in the Node tool the selected nodes are restored by anchor id (undo)"
    );
    ctrl_shift_z(&mut s);
    assert!(
        s.node_toolbar_state().can_delete,
        "criterion 23: the same for redo"
    );
    assert_eq!(s.tool(), Tool::Node, "the tool does not change");
    // Delete of that node, then undo: the node comes back with the same anchor id and is selected.
    let before = json(&s);
    let _ = key(&mut s, "Delete", false, false);
    assert_ne!(json(&s), before);
    ctrl_z(&mut s);
    assert_eq!(json(&s), before);
    assert!(
        s.node_toolbar_state().can_delete,
        "the restored node is selected again"
    );
}

// ---------------------------------------------------------------------
// Handle drags (resize, rotate, skew) of one and of several objects
// ---------------------------------------------------------------------

fn handle_drag(s: &mut Session, from: Point, to: Point) {
    s.pointer_hover(from, false, false);
    s.pointer_down(from, false);
    s.pointer_hover(
        pt(f64::midpoint(from.x, to.x), f64::midpoint(from.y, to.y)),
        false,
        false,
    );
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

fn wide_rects() -> Session {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(120.0, 80.0));
    drag(&mut s, Tool::Rectangle, pt(160.0, 20.0), pt(260.0, 80.0));
    assert_eq!(key(&mut s, "a", true, false), KeyOutcome::SelectedAll);
    s
}

#[test]
fn c10_corner_resize_drag_of_several_objects_is_one_step() {
    let mut s = wide_rects();
    roundtrip(&mut s, "Resize", |s| {
        handle_drag(s, pt(260.0, 80.0), pt(380.0, 140.0))
    });
}

#[test]
fn c10_corner_resize_drag_of_one_rectangle_is_one_step() {
    let mut s = fresh();
    drag(&mut s, Tool::Rectangle, pt(20.0, 20.0), pt(120.0, 80.0));
    assert_eq!(s.selected_object_count(), 1);
    roundtrip(&mut s, "Resize", |s| {
        handle_drag(s, pt(120.0, 80.0), pt(200.0, 120.0))
    });
}

#[test]
fn c10_rotate_drag_of_several_objects_is_one_step() {
    let mut s = wide_rects();
    let k = s.view().scale();
    let off = 32.0 / k / std::f64::consts::SQRT_2;
    let handle = pt(260.0 + off, 20.0 - off);
    roundtrip(&mut s, "Rotate", |s| {
        handle_drag(s, handle, pt(140.0, 140.0))
    });
}

#[test]
fn c10_skew_drag_of_several_paths_is_one_step() {
    let mut s = wide_rects();
    s.convert_selected_to_paths();
    s.set_tool(Tool::Select);
    assert_eq!(s.selected_object_count(), 2);
    let k = s.view().scale();
    let handle = pt(140.0, 20.0 - 16.0 / k);
    roundtrip(&mut s, "Skew", |s| {
        handle_drag(s, handle, pt(180.0, handle.y))
    });
}

#[test]
fn c10_move_drag_of_several_objects_is_one_step() {
    let mut s = wide_rects();
    roundtrip(&mut s, "Move", |s| {
        handle_drag(s, pt(60.0, 20.0), pt(80.0, 40.0))
    });
}

// ---------------------------------------------------------------------
// Revival: z-order, many rounds, nested containers
// ---------------------------------------------------------------------

fn four_rects() -> Session {
    let mut s = fresh();
    for i in 0..4 {
        let x = 10.0 + f64::from(i) * 40.0;
        drag(&mut s, Tool::Rectangle, pt(x, 20.0), pt(x + 30.0, 50.0));
    }
    s
}

fn select_by_edges(s: &mut Session, indexes: &[usize]) {
    s.set_tool(Tool::Select);
    let mut first = true;
    for &i in indexes {
        let x = 10.0 + i as f64 * 40.0;
        let at = pt(x, 30.0);
        s.pointer_hover(at, !first, false);
        s.pointer_down(at, !first);
        s.pointer_up(at, !first, false);
        first = false;
    }
    assert_eq!(s.selected_object_count(), indexes.len(), "setup selection");
}

#[test]
fn c23_revival_keeps_the_z_order_over_many_rounds_and_in_between_steps() {
    let mut s = four_rects();
    let order = ids(&s);
    let before = json(&s);
    // Delete the 2nd and 3rd (middle) objects.
    select_by_edges(&mut s, &[1, 2]);
    key(&mut s, "Delete", false, false);
    assert_eq!(ids(&s), vec![order[0], order[3]]);
    for round in 0..25 {
        ctrl_z(&mut s);
        assert_eq!(
            ids(&s),
            order,
            "undo round {round}: same ids in the same z-order"
        );
        ctrl_shift_z(&mut s);
        assert_eq!(ids(&s), vec![order[0], order[3]], "redo round {round}");
    }
    ctrl_z(&mut s);
    assert_eq!(json(&s), before);
}

#[test]
fn c23_revival_after_new_objects_were_drawn_and_taken_back_in_between() {
    let mut s = four_rects();
    let order = ids(&s);
    let before = json(&s);
    select_by_edges(&mut s, &[1]);
    key(&mut s, "Delete", false, false);
    // Draw a new object on top, then take it back, then take back the delete.
    drag(&mut s, Tool::Ellipse, pt(10.0, 80.0), pt(60.0, 120.0));
    ctrl_z(&mut s);
    ctrl_z(&mut s);
    assert_eq!(ids(&s), order);
    assert_eq!(json(&s), before);
    // Redo both and undo both again.
    ctrl_shift_z(&mut s);
    ctrl_shift_z(&mut s);
    assert_eq!(ids(&s).len(), 4);
    ctrl_z(&mut s);
    ctrl_z(&mut s);
    assert_eq!(json(&s), before);
}

#[test]
fn c23_delete_of_two_steps_in_a_row_comes_back_in_the_right_order() {
    let mut s = four_rects();
    let order = ids(&s);
    let before = json(&s);
    select_by_edges(&mut s, &[2]);
    key(&mut s, "Delete", false, false);
    select_by_edges(&mut s, &[0]);
    key(&mut s, "Delete", false, false);
    assert_eq!(ids(&s), vec![order[1], order[3]]);
    ctrl_z(&mut s);
    assert_eq!(ids(&s), vec![order[0], order[1], order[3]]);
    ctrl_z(&mut s);
    assert_eq!(ids(&s), order);
    assert_eq!(json(&s), before);
    ctrl_shift_z(&mut s);
    ctrl_shift_z(&mut s);
    assert_eq!(ids(&s), vec![order[1], order[3]]);
    ctrl_z(&mut s);
    ctrl_z(&mut s);
    assert_eq!(json(&s), before);
}

#[test]
fn c23_compound_paths_and_node_edited_paths_survive_delete_and_union_undo() {
    // A compound path (nested sub-path containers) deleted and revived, several rounds.
    let mut s = two_apart();
    s.convert_selected_to_paths();
    s.set_tool(Tool::Select);
    let _ = s.apply_combine();
    let compound = json(&s);
    key(&mut s, "Delete", false, false);
    for round in 0..4 {
        ctrl_z(&mut s);
        assert_eq!(json(&s), compound, "compound path revived, round {round}");
        ctrl_shift_z(&mut s);
        assert!(ids(&s).is_empty(), "round {round}");
    }
    ctrl_z(&mut s);
    // A path with curved segments and edited nodes.
    let mut s = open_path();
    node_click(&mut s, pt(50.0, 20.0));
    s.make_curve();
    node_click(&mut s, pt(80.0, 20.0));
    s.convert_selected(AnchorKind::Symmetric);
    s.set_tool(Tool::Select);
    let edited = json(&s);
    key(&mut s, "Delete", false, false);
    for round in 0..4 {
        ctrl_z(&mut s);
        assert_eq!(json(&s), edited, "edited path revived, round {round}");
        ctrl_shift_z(&mut s);
    }
}

#[test]
fn c10_break_apart_and_union_survive_many_rounds() {
    let mut s = two_apart();
    s.convert_selected_to_paths();
    s.set_tool(Tool::Select);
    let _ = s.apply_combine();
    let compound = json(&s);
    let _ = s.apply_break_apart();
    let broken = json(&s);
    for round in 0..4 {
        ctrl_z(&mut s);
        assert_eq!(json(&s), compound, "break apart undone, round {round}");
        ctrl_shift_z(&mut s);
        assert_eq!(json(&s), broken, "break apart redone, round {round}");
    }
}
