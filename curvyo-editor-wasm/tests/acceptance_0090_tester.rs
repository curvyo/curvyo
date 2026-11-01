//! Independent acceptance tests for `specs/0090-grid` through public session gestures: the
//! Shift+click toggle and Grid panel on the active page (criteria 2, 3), Add page copying the
//! grid (1), drawing only in the frame list (4), and grid snapping per tool (10 to 15).
#![allow(clippy::unwrap_used, clippy::float_cmp, clippy::assert_is_empty)]

use curvyo_document_core::{
    AnchorId, Angle, Direction, Document, DocumentSnapping, GridSnapPoint, Length, NewAnchor,
    OriginCorner, PageGrid, Point, RectBounds, StyleEdit, Vec2, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, Tool};
use curvyo_ui_core::{EntryOutcome, MoveEntryMode, ReferencePoint, object_outline_bounds};

const EXACT: f64 = 1e-8;

fn opened(s: &Session) -> Document {
    unpack(9090, &s.pack("acceptance").unwrap()).unwrap()
}

fn only_bounds(s: &Session) -> (Point, Point) {
    let d = opened(s);
    assert_eq!(d.object_ids().len(), 1);
    object_outline_bounds(&d.object(d.object_ids()[0]).unwrap())
}

fn close(a: (Point, Point), b: (Point, Point)) {
    for (x, y) in [
        (a.0.x, b.0.x),
        (a.0.y, b.0.y),
        (a.1.x, b.1.x),
        (a.1.y, b.1.y),
    ] {
        assert!((x - y).abs() < EXACT, "{a:?} != {b:?}");
    }
}

fn shifted(b: (Point, Point), d: Vec2) -> (Point, Point) {
    (b.0.translated(d), b.1.translated(d))
}

/// Snapping on with only the grid target.
fn grid_only(point: GridSnapPoint) -> DocumentSnapping {
    DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        grid: true,
        grid_point: point,
        ..DocumentSnapping::DEFAULT
    }
}

fn shown() -> PageGrid {
    PageGrid {
        shown: true,
        ..PageGrid::DEFAULT
    }
}

fn filled_rect(d: &Document, a: Point, b: Point) {
    let id = d.create_rect(RectBounds::from_corners(a, b));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
}

fn drag(s: &mut Session, start: Point, end: Point) {
    s.pointer_down(start, false);
    s.pointer_hover(end, false, false);
    s.pointer_up(end, false, false);
}

/// Presses, then holds Alt during the drag (Alt at press is a selection modifier).
fn alt_drag(s: &mut Session, start: Point, end: Point) {
    s.pointer_down(start, false);
    s.modifiers_changed(false, false, true);
    s.pointer_hover(end, false, false);
    s.pointer_up(end, false, false);
    s.modifiers_changed(false, false, false);
}

/// Selects the object under `grab` with a click, then drags it by `delta`.
fn move_by(s: &mut Session, grab: Point, delta: Vec2) {
    s.set_tool(Tool::Select);
    drag(s, grab, grab);
    assert_eq!(s.selected_object_count(), 1);
    drag(s, grab, grab.translated(delta));
}

fn session_of(d: &Document) -> Session {
    Session::open(9091, &pack(d, "acceptance").unwrap()).unwrap()
}

// ------------------------------------------------------------ criteria 2, 3

#[test]
fn ac2_shift_click_toggle_is_one_labelled_history_step_on_the_active_page() {
    let mut s = Session::new(9092);
    assert!(!s.active_page_grid().shown);
    let steps = s.undo_len();
    assert!(s.toggle_active_page_grid());
    assert!(s.active_page_grid().shown);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(
        s.history_target_name(Direction::Undo).as_deref(),
        Some("Change grid")
    );
    s.undo();
    assert!(!s.active_page_grid().shown);
    s.redo();
    assert!(s.active_page_grid().shown);
    assert!(!s.toggle_active_page_grid());
    assert_eq!(s.undo_len(), steps + 2);
    assert!(!opened(&s).pages()[0].grid.shown);
}

#[test]
fn ac2_entry_toggle_and_panel_act_on_the_active_page_only_and_name_it() {
    let mut s = Session::new(9093);
    let first = s.active_page().id;
    let second = s.add_page();
    assert_eq!(s.active_page().id, second);
    assert!(s.rename_page(&second, "Bed"));
    assert_eq!(s.active_page_grid().page_name, "Bed");
    assert_eq!(s.active_page_grid().page_id, second);
    s.toggle_active_page_grid();
    assert_eq!(s.set_active_page_grid_field("spacing", "10"), "committed");
    let d = opened(&s);
    assert!(d.page_grid(&second).shown);
    assert!((d.page_grid(&second).spacing.as_mm() - 10.0).abs() < 1e-9);
    assert!(!d.page_grid(&first).shown, "the other page is untouched");
    assert!((d.page_grid(&first).spacing.as_mm() - 5.0).abs() < 1e-9);
    assert!(s.activate_page(&first));
    assert_eq!(s.active_page_grid().page_id, first);
    assert!(!s.active_page_grid().shown);
}

#[test]
fn ac3_every_control_works_with_the_grid_hidden_and_each_change_is_one_step() {
    let mut s = Session::new(9094);
    assert!(!s.active_page_grid().shown);
    let mut steps = s.undo_len();
    let mut step = |s: &mut Session, ok: bool| {
        assert!(ok);
        steps += 1;
        assert_eq!(s.undo_len(), steps);
        assert!(!s.active_page_grid().shown, "controls never show the grid");
    };
    let ok = s.set_active_page_grid_mode("fixed");
    step(&mut s, ok);
    let ok = s.set_active_page_grid_scale("log");
    step(&mut s, ok);
    for (field, text) in [("spacing", "2.5"), ("subdivision", "10"), ("angle", "30")] {
        let ok = s.set_active_page_grid_field(field, text) == "committed";
        step(&mut s, ok);
    }
    let g = opened(&s).pages()[0].grid;
    assert!((g.spacing.as_mm() - 2.5).abs() < 1e-9);
    assert_eq!(g.subdivision, 10);
    assert!((g.angle.as_radians().to_degrees() - 30.0).abs() < 1e-9);
    assert_eq!(g.origin, OriginCorner::TopLeft);
    // Criterion 18: each origin corner is one step and works with the grid hidden.
    for corner in ["bottom_left", "bottom_right", "top_right", "top_left"] {
        let ok = s.set_active_page_origin(corner);
        step(&mut s, ok);
        assert_eq!(s.active_page_grid().origin, corner);
        assert_eq!(opened(&s).pages()[0].grid.origin.as_str(), corner);
    }
    // The panel's own switch.
    assert!(s.set_active_page_grid_shown(true));
    assert!(s.active_page_grid().shown);
}

#[test]
fn ac3_invalid_values_are_refused_without_a_step_and_the_angle_wraps() {
    let mut s = Session::new(9095);
    let steps = s.undo_len();
    for (field, text) in [
        ("spacing", "0"),
        ("spacing", "-1"),
        ("spacing", "abc"),
        ("spacing", "NaN"),
        ("spacing", "inf"),
        ("spacing", ""),
        ("subdivision", "1"),
        ("subdivision", "0"),
        ("subdivision", "-3"),
        ("subdivision", "2.5"),
        ("subdivision", "x"),
        ("angle", "NaN"),
        ("angle", "zz"),
        ("angle", ""),
    ] {
        let outcome = s.set_active_page_grid_field(field, text);
        assert!(
            outcome.starts_with("invalid:"),
            "{field} = {text:?} gave {outcome}"
        );
        assert_eq!(s.undo_len(), steps, "{field} = {text:?} made a step");
    }
    for corner in ["centre", "", "TopLeft", "top-left"] {
        assert!(!s.set_active_page_origin(corner), "{corner:?}");
        assert_eq!(s.undo_len(), steps, "{corner:?} made a step");
    }
    assert_eq!(opened(&s).pages()[0].grid, PageGrid::DEFAULT);
    // Subdivision 2 is the smallest valid value.
    assert_eq!(
        s.set_active_page_grid_field("subdivision", "2"),
        "committed"
    );
    // Permit over refuse: the angle is taken modulo 360.
    assert_eq!(s.set_active_page_grid_field("angle", "370"), "committed");
    let a = opened(&s).pages()[0].grid.angle.as_radians().to_degrees();
    assert!((a - 10.0).abs() < 1e-9, "{a}");
    assert_eq!(s.active_page_grid().angle, "10");
    assert_eq!(s.set_active_page_grid_field("angle", "-90"), "committed");
    let a = opened(&s).pages()[0].grid.angle.as_radians().to_degrees();
    assert!((a - 270.0).abs() < 1e-9, "{a}");
    assert_eq!(s.set_active_page_grid_field("angle", "360"), "committed");
    assert_eq!(opened(&s).pages()[0].grid.angle.as_radians(), 0.0);
}

// ------------------------------------------------------------------ criterion 1

#[test]
fn ac1_add_page_copies_the_active_grid_and_delete_undo_restores_it() {
    let mut s = Session::new(9096);
    s.toggle_active_page_grid();
    s.set_active_page_grid_field("spacing", "7");
    s.set_active_page_grid_field("angle", "15");
    let second = s.add_page();
    let g = s.active_page_grid();
    assert_eq!(g.page_id, second);
    assert!(g.shown);
    let d = opened(&s);
    let copy = d.page_grid(&second);
    assert!((copy.spacing.as_mm() - 7.0).abs() < 1e-9);
    assert!((copy.angle.as_radians().to_degrees() - 15.0).abs() < 1e-9);
    s.set_active_page_grid_field("spacing", "9");
    assert!(s.delete_page(&second));
    s.undo();
    let back = opened(&s).page_grid(&second);
    assert!((back.spacing.as_mm() - 9.0).abs() < 1e-9, "{back:?}");
}

// ------------------------------------------------------------------ criterion 4

#[test]
fn ac4_grid_is_only_in_the_frame_list_never_in_the_artwork_list() {
    let d = Document::new(9097);
    filled_rect(&d, Point::new(30.0, 30.0), Point::new(60.0, 60.0));
    let mut s = session_of(&d);
    s.resize_viewport(1200.0, 900.0);
    let artwork_hidden = s.draw_list().triangles.len();
    let frame_hidden = s.frame_draw_list().triangles.len();
    s.toggle_active_page_grid();
    assert_eq!(
        s.draw_list().triangles.len(),
        artwork_hidden,
        "the artwork list (export, print, machine) carries no grid"
    );
    assert!(
        s.frame_draw_list().triangles.len() > frame_hidden,
        "a shown grid is drawn in the frame"
    );
}

#[test]
fn ac4_a_click_on_a_grid_line_hits_nothing() {
    let mut s = Session::new(9098);
    s.toggle_active_page_grid();
    s.set_tool(Tool::Select);
    drag(&mut s, Point::new(20.0, 20.0), Point::new(20.0, 20.0));
    assert_eq!(s.selected_object_count(), 0);
    assert!(opened(&s).object_ids().is_empty());
}

// ---------------------------------------------------- criteria 10 to 15 (moves)

/// A 20 mm square at (10.6, 10.6), moved by (0.1, 0.2): its centre lands 0.7/0.8 mm from (20, 20).
fn square_doc(snapping: DocumentSnapping, grid: PageGrid) -> Document {
    let d = Document::new(9100);
    filled_rect(&d, Point::new(10.6, 10.6), Point::new(30.6, 30.6));
    d.set_snapping(snapping);
    let first = d.pages()[0].id.clone();
    d.set_page_grid(&first, grid).unwrap();
    d
}

const SQUARE: (Point, Point) = (Point::new(10.6, 10.6), Point::new(30.6, 30.6));
/// One grid step plus a small error, so the drag clears the click threshold.
const NUDGE: Vec2 = Vec2::new(5.1, 5.2);
const SNAPPED: (Point, Point) = (Point::new(15.0, 15.0), Point::new(35.0, 35.0));
const CREATED: (Point, Point) = (Point::new(10.0, 10.0), Point::new(30.0, 30.0));

#[test]
fn ac10_ac11_ac15_centre_snaps_exactly_to_a_grid_intersection_as_one_step() {
    let mut s = session_of(&square_doc(grid_only(GridSnapPoint::Centre), shown()));
    let steps = s.undo_len();
    move_by(&mut s, Point::new(20.6, 20.6), NUDGE);
    close(only_bounds(&s), SNAPPED);
    assert_eq!(s.undo_len(), steps + 1);
}

#[test]
fn ac10_default_snapping_settings_include_the_grid_target() {
    // A new document's grid target is on; with snapping on and a shown grid it pulls.
    let snapping = DocumentSnapping {
        enabled: true,
        objects: false,
        guides: false,
        nodes: false,
        page: false,
        ..DocumentSnapping::DEFAULT
    };
    let mut s = session_of(&square_doc(snapping, shown()));
    move_by(&mut s, Point::new(20.6, 20.6), NUDGE);
    close(only_bounds(&s), SNAPPED);
}

#[test]
fn ac10_ac12_hidden_grid_or_grid_switch_off_does_not_pull() {
    let mut s = session_of(&square_doc(
        grid_only(GridSnapPoint::Centre),
        PageGrid::DEFAULT,
    ));
    move_by(&mut s, Point::new(20.6, 20.6), NUDGE);
    close(only_bounds(&s), shifted(SQUARE, NUDGE));
    let off = DocumentSnapping {
        grid: false,
        ..grid_only(GridSnapPoint::Centre)
    };
    let mut s = session_of(&square_doc(off, shown()));
    move_by(&mut s, Point::new(20.6, 20.6), NUDGE);
    close(only_bounds(&s), shifted(SQUARE, NUDGE));
}

#[test]
fn ac12_alt_inverts_grid_snapping() {
    // Stored off + Alt: snaps.
    let stored_off = DocumentSnapping {
        enabled: false,
        ..grid_only(GridSnapPoint::Centre)
    };
    let mut s = session_of(&square_doc(stored_off, shown()));
    s.set_tool(Tool::Select);
    drag(&mut s, Point::new(20.6, 20.6), Point::new(20.6, 20.6));
    alt_drag(&mut s, Point::new(20.6, 20.6), Point::new(25.7, 25.8));
    close(only_bounds(&s), SNAPPED);
    // Stored on + Alt: does not snap.
    let mut s = session_of(&square_doc(grid_only(GridSnapPoint::Centre), shown()));
    s.set_tool(Tool::Select);
    drag(&mut s, Point::new(20.6, 20.6), Point::new(20.6, 20.6));
    alt_drag(&mut s, Point::new(20.6, 20.6), Point::new(25.7, 25.8));
    close(only_bounds(&s), shifted(SQUARE, NUDGE));
}

#[test]
fn ac11_box_choice_snaps_the_nearest_of_nine_reference_points() {
    // 12.7 mm square moved by (5.1, 5.2): its top-left (15.7, 15.8) is near (15, 15); its centre (22.05, 22.15) is
    // ~3 mm from every intersection and 2.05/2.15 mm from lines.
    let build = |point| {
        let d = Document::new(9101);
        filled_rect(&d, Point::new(10.6, 10.6), Point::new(23.3, 23.3));
        d.set_snapping(grid_only(point));
        d.set_page_grid(&d.pages()[0].id.clone(), shown()).unwrap();
        d
    };
    let raw = shifted((Point::new(10.6, 10.6), Point::new(23.3, 23.3)), NUDGE);
    let mut s = session_of(&build(GridSnapPoint::Box));
    move_by(&mut s, Point::new(17.0, 17.0), NUDGE);
    close(
        only_bounds(&s),
        (Point::new(15.0, 15.0), Point::new(27.7, 27.7)),
    );
    let mut s = session_of(&build(GridSnapPoint::Centre));
    move_by(&mut s, Point::new(17.0, 17.0), NUDGE);
    let b = only_bounds(&s);
    assert!(
        (b.0.x - raw.0.x).abs() < EXACT || (b.0.x - (raw.0.x - 2.05)).abs() < 1e-6,
        "centre choice must not snap the corner: {b:?}"
    );
    assert!(
        (b.0.x - 15.0).abs() > 0.1,
        "the box corner must not snap with Centre: {b:?}"
    );
}

#[test]
fn ac11_nodes_choice_snaps_a_moving_path_anchor() {
    // Box edges are all 2.5 mm off the 5 mm grid, so neither Centre nor Box can snap; the
    // interior notch anchor (20.3, 20.4) is near (20, 20).
    let build = |point| {
        let d = Document::new(9102);
        let pts = [
            Point::new(15.3, 12.5),
            Point::new(27.5, 17.5),
            Point::new(20.3, 20.4),
            Point::new(17.5, 32.5),
            Point::new(7.5, 22.5),
        ];
        let anchors: Vec<NewAnchor> = pts
            .iter()
            .enumerate()
            .map(|(i, p)| NewAnchor::corner(AnchorId::new(9102, i as u64 + 1), *p))
            .collect();
        let id = d.create_path(&anchors, true);
        d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
        d.set_snapping(grid_only(point));
        d.set_page_grid(&d.pages()[0].id.clone(), shown()).unwrap();
        d
    };
    let original = (Point::new(7.5, 12.5), Point::new(27.5, 32.5));
    let delta = Vec2::new(5.1, 5.1);
    let grab = Point::new(14.0, 24.0);
    let mut s = session_of(&build(GridSnapPoint::Nodes));
    move_by(&mut s, grab, delta);
    // Notch (25.4, 25.5) -> (25, 25): total shift (4.7, 4.6).
    close(only_bounds(&s), shifted(original, Vec2::new(4.7, 4.6)));
    for point in [GridSnapPoint::Centre, GridSnapPoint::Box] {
        let mut s = session_of(&build(point));
        move_by(&mut s, grab, delta);
        close(only_bounds(&s), shifted(original, delta));
    }
}

#[test]
fn ac11_nodes_choice_on_an_object_without_nodes_uses_the_box_centre() {
    // A rectangle shape is not a path; Proposal: it snaps by its box centre.
    let mut s = session_of(&square_doc(grid_only(GridSnapPoint::Nodes), shown()));
    move_by(&mut s, Point::new(20.6, 20.6), NUDGE);
    let b = only_bounds(&s);
    let centre_snapped = (b.0.x - 15.0).abs() < EXACT && (b.0.y - 15.0).abs() < EXACT;
    assert!(centre_snapped, "rect with Nodes choice: {b:?}");
}

#[test]
fn ac11_creation_snaps_the_placed_point_whatever_the_choice() {
    for point in [
        GridSnapPoint::Centre,
        GridSnapPoint::Box,
        GridSnapPoint::Nodes,
    ] {
        let d = Document::new(9103);
        d.set_snapping(grid_only(point));
        d.set_page_grid(&d.pages()[0].id.clone(), shown()).unwrap();
        let mut s = session_of(&d);
        s.set_tool(Tool::Rectangle);
        drag(&mut s, Point::new(10.6, 10.7), Point::new(30.4, 29.3));
        close(only_bounds(&s), CREATED);
    }
}

#[test]
fn ac11_resize_snaps_the_dragged_handle() {
    let mut s = session_of(&square_doc(grid_only(GridSnapPoint::Centre), shown()));
    s.set_tool(Tool::Select);
    drag(&mut s, Point::new(20.6, 20.6), Point::new(20.6, 20.6));
    assert_eq!(s.selected_object_count(), 1);
    drag(&mut s, Point::new(30.6, 30.6), Point::new(40.5, 39.4));
    let b = only_bounds(&s);
    assert!(
        (b.1.x - 40.0).abs() < EXACT && (b.1.y - 40.0).abs() < EXACT,
        "dragged corner must land on (40, 40): {b:?}"
    );
    assert!((b.0.x - 10.6).abs() < EXACT && (b.0.y - 10.6).abs() < EXACT);
}

#[test]
fn ac11_node_tool_snaps_the_dragged_anchor() {
    let d = Document::new(9104);
    let path = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9104, 1), Point::new(12.6, 12.6)),
            NewAnchor::corner(AnchorId::new(9104, 2), Point::new(42.6, 12.6)),
        ],
        false,
    );
    d.set_snapping(grid_only(GridSnapPoint::Centre));
    d.set_page_grid(&d.pages()[0].id.clone(), shown()).unwrap();
    let mut s = session_of(&d);
    s.set_tool(Tool::Node);
    drag(&mut s, Point::new(12.6, 12.6), Point::new(25.4, 24.7));
    let p = opened(&s).path(path).unwrap().anchors[0].point;
    assert!(
        (p.x - 25.0).abs() < EXACT && (p.y - 25.0).abs() < EXACT,
        "{p:?}"
    );
}

#[test]
fn ac12_pasteboard_has_no_grid_target() {
    let d = Document::new(9105);
    filled_rect(&d, Point::new(-30.6, -30.6), Point::new(-10.6, -10.6));
    d.set_snapping(grid_only(GridSnapPoint::Centre));
    d.set_page_grid(&d.pages()[0].id.clone(), shown()).unwrap();
    let mut s = session_of(&d);
    move_by(&mut s, Point::new(-20.6, -20.6), NUDGE);
    close(
        only_bounds(&s),
        shifted((Point::new(-30.6, -30.6), Point::new(-10.6, -10.6)), NUDGE),
    );
}

#[test]
fn ac12_overlapping_pages_the_page_on_top_decides() {
    let build = |top_is_second: bool, second_shown: bool| {
        let d = Document::new(9106);
        let first = d.pages()[0].id.clone();
        let second = d.add_page();
        d.move_page(
            &second.id,
            Vec2::new(100.0 - second.origin.x, -second.origin.y),
            &[],
        )
        .unwrap();
        d.set_page_grid(&first, shown()).unwrap();
        d.set_page_grid(
            &second.id,
            PageGrid {
                shown: second_shown,
                spacing: Length::from_mm(7.0),
                ..PageGrid::DEFAULT
            },
        )
        .unwrap();
        if !top_is_second {
            d.reorder_page(&first, 1).unwrap();
        }
        // Centre at (119.8, 19.8) after the nudge: first grid (120, 20), second grid (121, 21).
        filled_rect(&d, Point::new(104.7, 4.6), Point::new(124.7, 24.6));
        d.set_snapping(grid_only(GridSnapPoint::Centre));
        d
    };
    let raw = (Point::new(104.7, 4.6), Point::new(124.7, 24.6));
    let mut s = session_of(&build(true, true));
    move_by(&mut s, Point::new(114.7, 14.6), NUDGE);
    close(only_bounds(&s), shifted(raw, Vec2::new(6.3, 6.4)));
    let mut s = session_of(&build(false, true));
    move_by(&mut s, Point::new(114.7, 14.6), NUDGE);
    close(only_bounds(&s), shifted(raw, Vec2::new(5.3, 5.4)));
    let mut s = session_of(&build(true, false));
    move_by(&mut s, Point::new(114.7, 14.6), NUDGE);
    close(only_bounds(&s), shifted(raw, NUDGE));
}

#[test]
fn ac13_rotated_grid_snaps_the_centre_to_a_rotated_intersection() {
    let a = 30.0_f64.to_radians();
    let grid = PageGrid {
        angle: Angle::from_radians(a),
        origin: OriginCorner::BottomRight,
        ..shown()
    };
    // The page is moved so that its bottom-right corner, the grid origin, is at (100, 100);
    // the grid's u axis points into the page.
    let target = Point::new(100.0 - 5.0 * a.cos(), 100.0 - 5.0 * a.sin());
    let start_centre = Point::new(target.x + 0.3 - NUDGE.x, target.y + 0.2 - NUDGE.y);
    let d = Document::new(9107);
    filled_rect(
        &d,
        Point::new(start_centre.x - 10.0, start_centre.y - 10.0),
        Point::new(start_centre.x + 10.0, start_centre.y + 10.0),
    );
    d.set_snapping(grid_only(GridSnapPoint::Centre));
    let page = d.pages()[0].clone();
    d.set_page_grid(&page.id, grid).unwrap();
    d.move_page(
        &page.id,
        Vec2::new(
            100.0 - page.size.width.as_mm(),
            100.0 - page.size.height.as_mm(),
        ),
        &[],
    )
    .unwrap();
    let mut s = session_of(&d);
    move_by(&mut s, start_centre, NUDGE);
    let b = only_bounds(&s);
    let centre = Point::new(f64::midpoint(b.0.x, b.1.x), f64::midpoint(b.0.y, b.1.y));
    assert!(
        (centre.x - target.x).abs() < EXACT && (centre.y - target.y).abs() < EXACT,
        "{centre:?} expected {target:?}"
    );
}

#[test]
fn ac12_page_move_keeps_the_grid_on_the_page() {
    // The grid's origin is the page's top-left corner: after moving the page by (3.3, 1.1) the
    // intersection near the square's centre is (23.3, 21.1).
    let d = square_doc(grid_only(GridSnapPoint::Centre), shown());
    d.move_page(&d.pages()[0].id.clone(), Vec2::new(3.3, 1.1), &[])
        .unwrap();
    let mut s = session_of(&d);
    move_by(&mut s, Point::new(20.6, 20.6), Vec2::new(2.5, 0.6));
    let b = only_bounds(&s);
    let centre = Point::new(f64::midpoint(b.0.x, b.1.x), f64::midpoint(b.0.y, b.1.y));
    assert!(
        (centre.x - 23.3).abs() < EXACT && (centre.y - 21.1).abs() < EXACT,
        "{centre:?}"
    );
}

// ------------------------------------------------------------ criterion 19

/// Opens the typed move on the only object (selected by a click on it) and returns the
/// Absolute prefill of its top-left reference point.
fn open_move_on(s: &mut Session, at: Point) -> [String; 2] {
    s.set_tool(Tool::Select);
    s.pointer_down(at, false);
    s.pointer_up(at, false, false);
    let out = s.key_down(KeyInput {
        key: "m",
        ..KeyInput::default()
    });
    assert_eq!(out, KeyOutcome::EntryOpened);
    let index = ReferencePoint::ALL
        .iter()
        .position(|r| *r == ReferencePoint::TopLeft)
        .unwrap();
    s.set_move_reference(ReferencePoint::TopLeft);
    s.move_entry().unwrap().absolute_prefills[index].clone()
}

#[test]
fn ac19_typed_positions_measure_from_the_origin_corner_into_the_page() {
    // A 30 x 40 rectangle with its top-left at (10, 20) on the A4 page (210 x 297) at (0, 0).
    for (corner, prefill, landed) in [
        (
            OriginCorner::TopLeft,
            ["10.0", "20.0"],
            Point::new(100.0, 50.0),
        ),
        // Bottom left: 297 - 20 = 277 mm above the bottom edge; typed y 50 is 50 mm up.
        (
            OriginCorner::BottomLeft,
            ["10.0", "277.0"],
            Point::new(100.0, 247.0),
        ),
        // Top right: 210 - 10 = 200 mm left of the right edge; typed x 100 is 100 mm left.
        (
            OriginCorner::TopRight,
            ["200.0", "20.0"],
            Point::new(110.0, 50.0),
        ),
    ] {
        let d = Document::new(9120);
        filled_rect(&d, Point::new(10.0, 20.0), Point::new(40.0, 60.0));
        d.set_page_grid(
            &d.pages()[0].id.clone(),
            PageGrid {
                origin: corner,
                ..PageGrid::DEFAULT
            },
        )
        .unwrap();
        let mut s = session_of(&d);
        s.resize_viewport(1200.0, 800.0);
        assert_eq!(
            open_move_on(&mut s, Point::new(25.0, 40.0)),
            prefill,
            "{corner:?}"
        );
        let mode = MoveEntryMode {
            absolute: true,
            copy: false,
        };
        assert_eq!(
            s.commit_move_entry("100", "50", mode),
            EntryOutcome::Committed,
            "{corner:?}"
        );
        let (min, _) = only_bounds(&s);
        assert!(
            (min.x - landed.x).abs() < EXACT && (min.y - landed.y).abs() < EXACT,
            "{corner:?}: {min:?}"
        );
    }
}

#[test]
fn ac19_a_mark_click_measures_from_the_origin_corner_too() {
    let d = Document::new(9121);
    filled_rect(&d, Point::new(10.0, 20.0), Point::new(40.0, 60.0));
    d.set_page_grid(
        &d.pages()[0].id.clone(),
        PageGrid {
            origin: OriginCorner::BottomRight,
            ..PageGrid::DEFAULT
        },
    )
    .unwrap();
    let mut s = session_of(&d);
    s.resize_viewport(1200.0, 800.0);
    let _ = open_move_on(&mut s, Point::new(25.0, 40.0));
    s.set_move_chip_absolute(true);
    assert_eq!(
        s.commit_move_entry_at("100", "50", false, ReferencePoint::BottomRight),
        EntryOutcome::Committed
    );
    // The box's bottom-right corner lands 100 mm left of the right edge, 50 mm above the bottom.
    let (_, max) = only_bounds(&s);
    assert!(
        (max.x - 110.0).abs() < EXACT && (max.y - 247.0).abs() < EXACT,
        "{max:?}"
    );
}
