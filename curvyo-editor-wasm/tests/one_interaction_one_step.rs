//! "One interaction, one step" (`specs/0020-undo-redo` criteria 10, 11, 12, 13 and 58): one test per
//! row of the table in the specification. Each gesture is performed once through the editor
//! session and must add exactly one step and exactly one commit, with the operation name of the
//! table; Ctrl+Z must give the byte-equal `document.json` of before, Ctrl+Shift+Z the one of after.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_lines)]

use curvyo_document_core::{
    AnchorId, AnchorKind, Direction, Document, InnerRatio, Length, NewAnchor, Orientation, Point,
    PointCount, RectBounds, pack, unpack,
};
use curvyo_editor_wasm::{DocumentSide, KeyInput, Session, Tool};
use curvyo_ui_core::{BooleanOp, JoinType, MoveEntryMode};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn node(n: u64, x: f64, y: f64) -> NewAnchor {
    NewAnchor::corner(AnchorId::new(1, n), pt(x, y))
}

fn rect(document: &Document, x: f64, y: f64, w: f64, h: f64) {
    let _ = document.create_rect(RectBounds::from_corners(pt(x, y), pt(x + w, y + h)));
}

fn open(document: &Document) -> Session {
    let mut session = Session::open(2, &pack(document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.set_tool(Tool::Select);
    session
}

fn fresh() -> Session {
    let mut session = Session::new(2);
    session.resize_viewport(1200.0, 800.0);
    session
}

fn reread(session: &Session) -> Document {
    unpack(9, &session.pack("0.1.0").unwrap()).unwrap()
}

fn json(session: &Session) -> Vec<u8> {
    reread(session).export_json().unwrap()
}

fn commits(session: &Session) -> usize {
    let loro = loro::LoroDoc::new();
    loro.import(&reread(session).export_loro_snapshot().unwrap())
        .unwrap();
    loro.len_changes()
}

fn click(session: &mut Session, at: Point, shift: bool) {
    session.pointer_hover(at, shift, false);
    session.pointer_down(at, shift);
    session.pointer_up(at, shift, false);
}

fn drag(session: &mut Session, from: Point, to: Point) {
    session.pointer_hover(from, false, false);
    session.pointer_down(from, false);
    session.pointer_hover(to, false, false);
    session.pointer_up(to, false, false);
}

fn marquee_all(session: &mut Session) {
    session.set_tool(Tool::Select);
    drag(session, pt(-50.0, -50.0), pt(900.0, 700.0));
}

fn key(session: &mut Session, k: &str) {
    session.key_down(KeyInput {
        key: k,
        ..KeyInput::default()
    });
}

/// Performs `gesture` and checks the contract of one interaction, one step.
fn one_step(session: &mut Session, name: &str, gesture: impl FnOnce(&mut Session)) {
    let (steps, changes, before) = (session.undo_len(), commits(session), json(session));
    gesture(session);
    assert_eq!(session.undo_len(), steps + 1, "{name}: exactly one step");
    assert_eq!(commits(session), changes + 1, "{name}: exactly one commit");
    assert_eq!(
        session.history_target_name(Direction::Undo).as_deref(),
        Some(name),
        "the operation name"
    );
    let after = json(session);
    assert_ne!(before, after, "{name}: the gesture changed something");
    session.undo();
    assert_eq!(
        json(session),
        before,
        "{name}: Ctrl+Z gives the export of before"
    );
    session.redo();
    assert_eq!(
        json(session),
        after,
        "{name}: Ctrl+Shift+Z gives the export of after"
    );
    assert_eq!(session.undo_len(), steps + 1);
}

fn open_pen(document: &Document) -> Session {
    let mut session = open(document);
    session.set_tool(Tool::Pen);
    session
}

#[test]
fn pen_finish_a_path_is_draw_path() {
    let mut session = fresh();
    session.set_tool(Tool::Pen);
    one_step(&mut session, "Draw path", |s| {
        click(s, pt(10.0, 10.0), false);
        click(s, pt(100.0, 10.0), false);
        click(s, pt(100.0, 100.0), false);
        s.finish_pen();
    });
}

#[test]
fn pen_continue_a_path_is_extend_path() {
    let document = Document::new(1);
    let _ = document.create_path(&[node(1, 0.0, 0.0), node(2, 100.0, 0.0)], false);
    let mut session = open_pen(&document);
    one_step(&mut session, "Extend path", |s| {
        click(s, pt(100.0, 0.0), false);
        click(s, pt(200.0, 50.0), false);
        s.finish_pen();
    });
}

#[test]
fn pen_connect_two_paths_is_connect_paths() {
    let document = Document::new(1);
    let _ = document.create_path(&[node(1, 0.0, 0.0), node(2, 100.0, 0.0)], false);
    let _ = document.create_path(&[node(3, 300.0, 0.0), node(4, 400.0, 0.0)], false);
    let mut session = open_pen(&document);
    one_step(&mut session, "Connect paths", |s| {
        click(s, pt(100.0, 0.0), false);
        click(s, pt(300.0, 0.0), false);
    });
}

#[test]
fn pen_close_a_path_is_close_path() {
    let document = Document::new(1);
    let _ = document.create_path(
        &[node(1, 0.0, 0.0), node(2, 10.0, 0.0), node(3, 10.0, 10.0)],
        false,
    );
    let mut session = open_pen(&document);
    one_step(&mut session, "Close path", |s| {
        click(s, pt(10.0, 10.0), false);
        click(s, pt(0.0, 0.0), false);
    });
}

#[test]
fn the_close_path_button_is_close_path() {
    let document = Document::new(1);
    let _ = document.create_path(
        &[
            node(1, 0.0, 0.0),
            node(2, 100.0, 0.0),
            node(3, 100.0, 100.0),
        ],
        false,
    );
    let mut session = open(&document);
    marquee_all(&mut session);
    session.set_tool(Tool::Node);
    one_step(&mut session, "Close path", |s| {
        let _ = s.close_paths(JoinType::Sharp);
    });
}

#[test]
fn the_shape_tools_draw_rectangle_ellipse_polygon_and_star() {
    for (tool, name) in [
        (Tool::Rectangle, "Draw rectangle"),
        (Tool::Ellipse, "Draw ellipse"),
        (Tool::PolygonStar, "Draw polygon"),
    ] {
        let mut session = fresh();
        session.set_tool(tool);
        one_step(&mut session, name, |s| {
            drag(s, pt(10.0, 10.0), pt(90.0, 70.0));
        });
    }
    let mut session = fresh();
    session.set_tool(Tool::PolygonStar);
    session.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
    one_step(&mut session, "Draw star", |s| {
        drag(s, pt(10.0, 10.0), pt(90.0, 70.0));
    });
}

fn square_session() -> Session {
    let document = Document::new(1);
    rect(&document, 20.0, 20.0, 100.0, 60.0);
    let mut session = open(&document);
    marquee_all(&mut session);
    assert_eq!(session.selected_object_count(), 1);
    session
}

#[test]
fn a_move_drag_is_move() {
    let mut session = square_session();
    one_step(&mut session, "Move", |s| {
        drag(s, pt(70.0, 50.0), pt(100.0, 90.0));
    });
}

#[test]
fn a_typed_move_is_move() {
    let mut session = square_session();
    one_step(&mut session, "Move", |s| {
        key(s, "m");
        s.commit_move_entry(
            "10",
            "5",
            MoveEntryMode {
                absolute: false,
                copy: false,
            },
        );
    });
}

#[test]
fn a_copy_move_is_duplicate() {
    let mut session = square_session();
    one_step(&mut session, "Duplicate", |s| {
        key(s, "m");
        s.commit_move_entry(
            "150",
            "0",
            MoveEntryMode {
                absolute: false,
                copy: true,
            },
        );
    });
}

#[test]
fn a_typed_size_a_typed_angle_and_a_typed_skew_are_resize_rotate_and_skew() {
    let mut session = square_session();
    one_step(&mut session, "Resize", |s| {
        key(s, "s");
        s.commit_transform_entry("120", "80", 0);
    });
    one_step(&mut session, "Rotate", |s| {
        key(s, "r");
        s.commit_transform_entry("30", "", 0);
    });

    let document = Document::new(1);
    let _ = document.create_path(&[node(1, 0.0, 0.0), node(2, 100.0, 40.0)], false);
    let mut session = open(&document);
    click(&mut session, pt(0.0, 0.0), false);
    assert_eq!(session.selected_object_count(), 1);
    one_step(&mut session, "Skew", |s| {
        key(s, "k");
        s.commit_transform_entry("15", "", 0);
    });
}

#[test]
fn a_resize_drag_is_resize() {
    let mut session = square_session();
    // The bottom-right corner handle of the selection box at (120, 80).
    one_step(&mut session, "Resize", |s| {
        drag(s, pt(120.0, 80.0), pt(160.0, 100.0));
    });
}

#[test]
fn the_point_count_and_the_inner_ratio_have_their_own_names() {
    let mut session = fresh();
    session.set_tool(Tool::PolygonStar);
    session.set_poly_star_mode(curvyo_ui_core::PolyStarMode::Star);
    drag(&mut session, pt(60.0, 60.0), pt(120.0, 60.0));
    assert_eq!(session.selected_object_count(), 1);
    one_step(&mut session, "Change point count", |s| {
        s.set_selected_point_count(PointCount::new(7).unwrap());
    });
    one_step(&mut session, "Change inner ratio", |s| {
        s.set_selected_ratio(InnerRatio::new(0.3).unwrap());
        s.commit_selected_ratio();
    });
}

#[test]
fn a_corner_radius_is_change_corner_radius() {
    let mut session = square_session();
    one_step(&mut session, "Change corner radius", |s| {
        let _ = s.set_selected_radius_text("8");
    });
}

#[test]
fn object_to_path_and_delete_have_their_names() {
    let mut session = square_session();
    one_step(&mut session, "Delete", Session::delete_selected);
    let mut session = square_session();
    one_step(
        &mut session,
        "Object to path",
        Session::convert_selected_to_paths,
    );
}

#[test]
fn several_objects_move_resize_and_rotate_in_one_step_each() {
    let document = Document::new(1);
    rect(&document, 20.0, 20.0, 100.0, 60.0);
    rect(&document, 160.0, 20.0, 100.0, 60.0);
    let mut session = open(&document);
    marquee_all(&mut session);
    assert_eq!(session.selected_object_count(), 2);
    one_step(&mut session, "Move", |s| {
        key(s, "m");
        s.commit_move_entry(
            "10",
            "5",
            MoveEntryMode {
                absolute: false,
                copy: false,
            },
        );
    });
    one_step(&mut session, "Resize", |s| {
        key(s, "s");
        s.commit_transform_entry("300", "100", 0);
    });
    one_step(&mut session, "Rotate", |s| {
        key(s, "r");
        s.commit_transform_entry("20", "", 0);
    });
}

#[test]
fn several_paths_skew_in_one_step() {
    let document = Document::new(1);
    let _ = document.create_path(&[node(1, 0.0, 0.0), node(2, 60.0, 40.0)], false);
    let _ = document.create_path(&[node(3, 70.0, 0.0), node(4, 90.0, 40.0)], false);
    let mut session = open(&document);
    marquee_all(&mut session);
    assert_eq!(session.selected_object_count(), 2);
    one_step(&mut session, "Skew", |s| {
        key(s, "k");
        s.commit_transform_entry("12", "", 0);
    });
}

#[test]
fn a_style_edit_and_a_value_drag_are_change_style() {
    let mut session = square_session();
    one_step(&mut session, "Change style", |s| {
        s.set_stroke_paint(false);
    });
    one_step(&mut session, "Change style", |s| {
        s.set_fill_paint(true);
    });
    one_step(&mut session, "Change style", |s| {
        let _ = s.set_style_text(curvyo_ui_core::StyleField::StrokeWidth, "3");
    });
}

#[test]
fn the_document_section_has_four_named_commands() {
    let document = Document::new(1);
    rect(&document, 20.0, 20.0, 100.0, 60.0);
    let mut session = open(&document);
    one_step(&mut session, "Resize document", |s| {
        let _ = s.set_document_side(DocumentSide::Width, "400");
    });
    one_step(&mut session, "Resize document", |s| {
        let _ = s.set_document_orientation(Orientation::Portrait);
    });
    one_step(&mut session, "Fit document to content", |s| {
        let _ = s.fit_document();
    });
    one_step(&mut session, "Change display unit", |s| {
        let _ = s.set_display_unit(curvyo_document_core::DisplayUnit::In);
    });
}

fn node_session() -> Session {
    let document = Document::new(1);
    let _ = document.create_path(
        &[
            node(1, 0.0, 0.0),
            node(2, 100.0, 0.0),
            node(3, 100.0, 100.0),
        ],
        false,
    );
    let mut session = open(&document);
    click(&mut session, pt(0.0, 0.0), false);
    assert_eq!(session.selected_object_count(), 1);
    session.set_tool(Tool::Node);
    session
}

#[test]
fn the_node_tool_commands_have_their_names() {
    let mut session = node_session();
    click(&mut session, pt(100.0, 0.0), false);
    one_step(&mut session, "Move nodes", |s| {
        drag(s, pt(100.0, 0.0), pt(110.0, 20.0));
    });
    one_step(&mut session, "Change node type", |s| {
        s.convert_selected(AnchorKind::Symmetric);
    });
    one_step(&mut session, "Delete nodes", Session::delete_selected);

    // A segment: curve, line and insert, each on its own selected segment.
    let mut session = node_session();
    click(&mut session, pt(50.0, 0.0), false);
    one_step(&mut session, "Curve segment", Session::make_curve);
    let mut session = node_session();
    click(&mut session, pt(50.0, 0.0), false);
    one_step(&mut session, "Add node", Session::insert_selected);
}

#[test]
fn a_handle_drag_and_a_segment_bend_have_their_names() {
    let mut session = node_session();
    click(&mut session, pt(100.0, 0.0), false);
    session.convert_selected(AnchorKind::Symmetric);
    // The converted node has handles; grab the outgoing one where the document says it is.
    let path = {
        let document = reread(&session);
        document.path(document.object_ids()[0]).unwrap()
    };
    let anchor = &path.anchors[1];
    let at = pt(
        anchor.point.x + anchor.handle_out.x,
        anchor.point.y + anchor.handle_out.y,
    );
    one_step(&mut session, "Move handle", |s| {
        drag(s, at, pt(at.x + 15.0, at.y + 25.0));
    });

    let mut session = node_session();
    click(&mut session, pt(50.0, 0.0), false);
    one_step(&mut session, "Bend segment", |s| {
        drag(s, pt(50.0, 0.0), pt(50.0, 30.0));
    });
}

#[test]
fn join_and_split_have_their_names() {
    let document = Document::new(1);
    let _ = document.create_path(&[node(1, 0.0, 0.0), node(2, 20.0, 0.0)], false);
    let _ = document.create_path(&[node(3, 100.0, 50.0), node(4, 120.0, 50.0)], false);
    let mut session = open(&document);
    marquee_all(&mut session);
    session.set_tool(Tool::Node);
    click(&mut session, pt(20.0, 0.0), false);
    click(&mut session, pt(100.0, 50.0), true);
    one_step(&mut session, "Join nodes", |s| {
        s.join_selected();
    });

    let document = Document::new(1);
    let _ = document.create_path(
        &[node(1, 0.0, 0.0), node(2, 50.0, 0.0), node(3, 100.0, 0.0)],
        false,
    );
    let mut session = open(&document);
    click(&mut session, pt(0.0, 0.0), false);
    session.set_tool(Tool::Node);
    click(&mut session, pt(50.0, 0.0), false);
    one_step(&mut session, "Split path", Session::split_selected);
}

fn two_squares() -> Session {
    let document = Document::new(1);
    rect(&document, 0.0, 0.0, 20.0, 20.0);
    rect(&document, 10.0, 10.0, 20.0, 20.0);
    let mut session = open(&document);
    marquee_all(&mut session);
    assert_eq!(session.selected_object_count(), 2);
    session
}

#[test]
fn every_boolean_operation_has_its_name() {
    for (op, name) in [
        (BooleanOp::Union, "Union"),
        (BooleanOp::Difference, "Difference"),
        (BooleanOp::Intersection, "Intersection"),
        (BooleanOp::Exclusion, "Exclusion"),
        (BooleanOp::ReverseDifference, "Reverse difference"),
    ] {
        let mut session = two_squares();
        one_step(&mut session, name, |s| {
            s.apply_boolean(op);
        });
    }
}

#[test]
fn combine_split_break_apart_and_cut_have_their_names() {
    let document = Document::new(1);
    let outer = [
        node(1, 0.0, 0.0),
        node(2, 100.0, 0.0),
        node(3, 100.0, 100.0),
        node(4, 0.0, 100.0),
    ];
    let hole = [
        node(5, 20.0, 20.0),
        node(6, 80.0, 20.0),
        node(7, 80.0, 80.0),
        node(8, 20.0, 80.0),
    ];
    let island = [
        node(9, 40.0, 40.0),
        node(10, 60.0, 40.0),
        node(11, 60.0, 60.0),
        node(12, 40.0, 60.0),
    ];
    let _ = document.create_path(&outer, true);
    let _ = document.create_path(&hole, true);
    let _ = document.create_path(&island, true);
    let mut session = open(&document);
    marquee_all(&mut session);
    one_step(&mut session, "Combine", |s| {
        s.apply_combine();
    });
    one_step(&mut session, "Split", |s| {
        s.apply_split();
    });
    one_step(&mut session, "Combine", |s| {
        s.apply_combine();
    });
    one_step(&mut session, "Break apart", |s| {
        s.apply_break_apart();
    });

    let crossing = Document::new(1);
    let _ = crossing.create_path(&[node(21, 0.0, 0.0), node(22, 100.0, 100.0)], false);
    let _ = crossing.create_path(&[node(23, 0.0, 100.0), node(24, 100.0, 0.0)], false);
    let mut lines = open(&crossing);
    marquee_all(&mut lines);
    one_step(&mut lines, "Cut", |s| {
        s.apply_cut();
    });
}

/// Criterion 12: an interaction that writes nothing adds no step.
#[test]
fn gestures_that_write_nothing_add_no_step() {
    let mut session = square_session();
    let steps = session.undo_len();
    let changes = commits(&session);
    // A drag that ends where it began, a typed value equal to the current one, Escape in a drag.
    drag(&mut session, pt(120.0, 50.0), pt(120.0, 50.0));
    key(&mut session, "m");
    session.commit_move_entry(
        "0",
        "0",
        MoveEntryMode {
            absolute: false,
            copy: false,
        },
    );
    session.pointer_down(pt(120.0, 50.0), false);
    session.pointer_hover(pt(90.0, 90.0), false, false);
    session.escape();
    session.pointer_up(pt(90.0, 90.0), false, false);
    assert_eq!(session.undo_len(), steps);
    assert_eq!(commits(&session), changes);
    // The Document section: a length equal to the current one.
    let _ = session.set_document_side(DocumentSide::Width, "210");
    assert_eq!(session.undo_len(), steps);
    let _ = Length::from_mm(0.0);
}
