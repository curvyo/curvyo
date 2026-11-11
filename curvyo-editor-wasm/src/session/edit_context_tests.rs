//! Move versus edit in the session (`specs/0106-move-versus-edit` criteria 2 to 5, 9, 10 and 13
//! to 16): presses keep the page, move commands work on out-of-context objects, edit commands
//! write nothing for them, and a double-click or Enter makes their page active.
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{
    EllipseFrame, FIRST_PAGE_ID, NodeId, ObjectSnapshot, Page, Point, RectBounds, Shape,
};
use curvyo_ui_core::{FlipAxis, Scope};

use crate::session::keys::KeyInput;
use crate::session::{Session, Tool};

/// Drawing pages A ("first") and B, each with a 20 mm square, A active, Select.
fn two_pages(peer: u64) -> (Session, Page, NodeId, NodeId) {
    let mut s = Session::new(peer);
    let b = s.add_page();
    s.activate_page(FIRST_PAGE_ID);
    let b = s.document.pages().into_iter().find(|p| p.id == b).unwrap();
    let on_a = square(&s, Point::new(10.0, 10.0));
    let on_b = square(&s, Point::new(b.origin.x + 10.0, b.origin.y + 10.0));
    s.set_tool(Tool::Select);
    (s, b, on_a, on_b)
}

fn square(s: &Session, at: Point) -> NodeId {
    s.document.create_rect(RectBounds::from_corners(
        at,
        Point::new(at.x + 20.0, at.y + 20.0),
    ))
}

/// A point on the left edge of the square at `at`: a square has no fill, its outline is hit.
fn centre(at: Point) -> Point {
    Point::new(at.x, at.y + 10.0)
}

fn click(s: &mut Session, at: Point, shift: bool) {
    s.pointer_hover(at, shift, false);
    s.pointer_down(at, shift);
    s.pointer_up(at, shift, false);
}

fn double_click(s: &mut Session, at: Point) {
    click(s, at, false);
    s.double_click(at, false, false);
}

fn key(s: &mut Session, k: &str, ctrl: bool) {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    });
}

fn origin_of(s: &Session, id: NodeId) -> Point {
    let (min, _) = curvyo_ui_core::object_bounds(s.document.object(id).as_ref().unwrap());
    min
}

#[test]
fn criterion_2_a_press_on_another_pages_object_selects_it_and_keeps_the_page() {
    let (mut s, b, _, on_b) = two_pages(106_201);
    let steps = s.undo_len();
    click(
        &mut s,
        centre(Point::new(b.origin.x + 10.0, b.origin.y + 10.0)),
        false,
    );
    assert_eq!(s.selection.ids(), [on_b]);
    assert_eq!(s.active_page().id, FIRST_PAGE_ID);
    assert_eq!(s.undo_len(), steps, "no history step");
    assert_eq!(s.edit_scope(), Scope::MoveOnly);
    // An empty press inside B's frame still activates B (`0098` criterion 8).
    click(
        &mut s,
        Point::new(b.origin.x + 80.0, b.origin.y + 80.0),
        false,
    );
    assert_eq!(s.active_page().id, b.id);
}

#[test]
fn criterion_3_select_all_is_the_active_pages_context_shift_and_marquee_are_not() {
    let (mut s, b, on_a, on_b) = two_pages(106_202);
    let paste = square(&s, Point::new(10.0, -200.0));
    key(&mut s, "a", true);
    let all: std::collections::HashSet<NodeId> = s.selection.ids().iter().copied().collect();
    assert_eq!(all, [on_a, paste].into_iter().collect());
    // Shift+press adds an object of another context.
    click(
        &mut s,
        centre(Point::new(b.origin.x + 10.0, b.origin.y + 10.0)),
        true,
    );
    assert!(s.selection.contains(on_b) && s.selection.contains(on_a));
    assert_eq!(s.edit_scope(), Scope::MoveOnly);
}

#[test]
fn criterion_4_a_mixed_selection_is_move_only_and_no_part_is_edited() {
    let (mut s, _, on_a, on_b) = two_pages(106_203);
    s.selection.set(&[on_a, on_b]);
    let version = s.document.version();
    let steps = s.undo_len();
    let filled = s.document.object(on_a).unwrap().style().fill.enabled;
    s.set_fill_paint(!filled);
    assert_eq!(s.document.version(), version, "nothing changes");
    assert_eq!(s.undo_len(), steps);
    s.selection.set(&[on_a]);
    s.set_fill_paint(!filled);
    assert_ne!(
        s.document.version(),
        version,
        "with only A's square the fill changes"
    );
}

#[test]
fn criterion_5_move_commands_work_out_of_context_and_keep_a_primitive() {
    let mut s = Session::new(106_205);
    let b = s.add_page();
    let b = s.document.pages().into_iter().find(|p| p.id == b).unwrap();
    let ellipse = s.document.create_ellipse(EllipseFrame::from_corners(
        Point::new(b.origin.x + 10.0, b.origin.y + 10.0),
        Point::new(b.origin.x + 40.0, b.origin.y + 30.0),
    ));
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let laser = s.active_page().id;
    s.set_tool(Tool::Select);
    s.selection.set(&[ellipse]);
    assert_eq!(s.edit_scope(), Scope::MoveOnly);
    let before = origin_of(&s, ellipse);
    let steps = s.undo_len();
    s.key_down(KeyInput {
        key: "ArrowRight",
        shift: true,
        ..KeyInput::default()
    });
    let moved = origin_of(&s, ellipse);
    assert!(moved.x > before.x + 1.0, "moved right");
    assert!(s.undo_len() > steps);
    key(&mut s, "r", false);
    s.commit_transform_entry("30", "", 0);
    assert!(matches!(
        s.document.object(ellipse),
        Some(ObjectSnapshot::Primitive(p)) if matches!(p.shape, Shape::Ellipse { .. })
    ));
    s.flip_selection(FlipAxis::Horizontal);
    s.delete_selected();
    assert!(s.document.object(ellipse).is_none());
    assert_eq!(
        s.active_page().id,
        laser,
        "a move does not change the active page"
    );
    s.undo();
    assert!(s.document.object(ellipse).is_some());
}

#[test]
fn criterion_9_edit_commands_write_nothing_for_an_out_of_context_selection() {
    let (mut s, _, _, on_b) = two_pages(106_209);
    s.selection.set(&[on_b]);
    let version = s.document.version();
    let steps = s.undo_len();
    s.set_fill_paint(false);
    s.group_selection();
    s.ungroup_selection();
    s.unlink_clones();
    s.convert_selected_to_paths();
    s.set_selected_radius_text("3");
    s.toggle_guide();
    key(&mut s, "Home", false);
    key(&mut s, "End", false);
    s.rename_layer_row(&serde_json::to_string(&on_b).unwrap(), "Renamed");
    assert_eq!(s.document.version(), version, "byte-identical");
    assert_eq!(s.undo_len(), steps, "history unchanged");
    // A chord that reaches a hidden command says where the object lies, once.
    assert_eq!(s.take_context_notice(), "Not editable here: on Page 2.");
    assert_eq!(s.take_context_notice(), "");
    let second = square(&s, Point::new(s.document.pages()[1].origin.x + 60.0, 10.0));
    s.selection.set(&[on_b, second]);
    let version = s.document.version();
    key(&mut s, "g", true);
    assert_eq!(s.take_context_notice(), "Not editable here: on Page 2.");
    assert_eq!(s.document.version(), version, "Group wrote nothing");
}

#[test]
fn criterion_10_the_node_tool_shows_nothing_on_a_foreign_path_and_its_press_selects_it() {
    let (mut s, b, _, on_b) = two_pages(106_210);
    s.set_tool(Tool::Node);
    assert!(s.node_paths().iter().all(|p| p.id != on_b));
    // The square's outline on its left edge.
    let edge = Point::new(b.origin.x + 10.0, b.origin.y + 20.0);
    click(&mut s, edge, false);
    assert_eq!(s.selection.ids(), [on_b]);
    assert_eq!(s.tool(), Tool::Node, "no tool is switched away");
    assert!(
        s.node_paths().is_empty(),
        "no nodes on a move-only selection"
    );
}

#[test]
fn criterion_13_a_double_click_activates_the_page_and_the_second_enters_the_group() {
    let (mut s, b, _, on_b) = two_pages(106_213);
    let second = square(&s, Point::new(b.origin.x + 40.0, b.origin.y + 10.0));
    let group = s.document.group_objects(&[on_b, second]).unwrap();
    // On the first square's bottom edge, clear of the group box's handles.
    let at = Point::new(b.origin.x + 22.0, b.origin.y + 30.0);
    s.sync_history();
    let steps = s.undo_len();
    double_click(&mut s, at);
    assert_eq!(s.active_page().id, b.id);
    assert!(s.selection.contains(on_b) && s.selection.contains(second));
    assert_eq!(s.context_group(), None, "not entered");
    assert_eq!(s.undo_len(), steps, "the activation is no step");
    double_click(&mut s, at);
    assert_eq!(s.context_group(), Some(group));
}

#[test]
fn criterion_13_enter_does_what_the_double_click_does() {
    let (mut s, b, _, on_b) = two_pages(106_214);
    click(
        &mut s,
        centre(Point::new(b.origin.x + 10.0, b.origin.y + 10.0)),
        false,
    );
    key(&mut s, "Enter", false);
    assert_eq!(s.active_page().id, b.id);
    assert_eq!(s.selection.ids(), [on_b]);
}

#[test]
fn criterion_14_a_pasteboard_double_click_on_a_laser_page_opens_the_last_drawing_page() {
    let (mut s, b, _, _) = two_pages(1_062_141);
    s.activate_page(&b.id);
    let paste = square(&s, Point::new(10.0, -200.0));
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.set_tool(Tool::Select);
    double_click(&mut s, centre(Point::new(10.0, -200.0)));
    assert_eq!(s.active_page().id, b.id, "the last active drawing page");
    assert_eq!(s.selection.ids(), [paste]);
    assert_eq!(s.edit_scope(), Scope::Edit, "editable there");
}

#[test]
fn criterion_14_without_a_drawing_page_nothing_is_activated_and_a_notice_says_so() {
    let mut s = Session::new(1_062_142);
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.delete_page(FIRST_PAGE_ID);
    s.activate_page(&laser);
    let paste = square(&s, Point::new(10.0, -900.0));
    s.set_tool(Tool::Select);
    double_click(&mut s, centre(Point::new(10.0, -900.0)));
    assert_eq!(s.active_page().id, laser);
    assert_eq!(s.selection.ids(), [paste]);
    assert_eq!(
        s.take_context_notice(),
        "There is no drawing page to edit this on."
    );
}

#[test]
fn criterion_15_an_own_item_of_a_laser_page_activates_it_and_a_placed_page_its_source() {
    let mut s = Session::new(106_215);
    let source = square(&s, Point::new(10.0, 10.0));
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    let l = s.active_page();
    // Clear of the placed page's frame, which covers the drawing page's size at the origin.
    let corner = Point::new(
        l.origin.x + l.size.width.as_mm() - 40.0,
        l.origin.y + l.size.height.as_mm() - 40.0,
    );
    let item = square(&s, corner);
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let placed = s.selected_placement().unwrap();
    s.set_tool(Tool::Select);
    s.activate_page(FIRST_PAGE_ID);
    double_click(&mut s, centre(corner));
    assert_eq!(s.active_page().id, laser);
    assert_eq!(s.selection.ids(), [item]);
    // The placed page's copy of the source square, double-clicked on the laser page.
    let map = s
        .document
        .placed_page(placed)
        .unwrap()
        .global_map()
        .unwrap();
    let at = map.apply_point(Point::new(10.0, 20.0));
    double_click(&mut s, at);
    assert_eq!(s.active_page().id, FIRST_PAGE_ID);
    assert_eq!(s.selection.ids(), [source]);
}

#[test]
fn the_parameter_handles_are_gone_for_a_move_only_selection() {
    let (mut s, _, _, on_b) = two_pages(106_216);
    s.selection.set(&[on_b]);
    let tolerances = s.transform_handle_tolerances();
    assert!(tolerances.param_min_side_mm.is_infinite());
}

// ---- a sketch page active: drawing objects move only (criteria 3, 7, 8, 16) -----------------

/// A sketch page active with a linked clone of its sketch on the pasteboard, 500 mm above.
fn sketch_with_pasteboard_clone(peer: u64) -> (Session, String, NodeId, NodeId, Point) {
    let (mut s, sketch_page, sketch) = crate::session::sketch_use::tests::setup(peer);
    let clone = s
        .document
        .place_linked_clone(sketch, curvyo_document_core::Vec2::new(0.0, -500.0))
        .unwrap();
    s.activate_page(&sketch_page);
    let centre = crate::session::sketch_use::tests::centre_of(&s, clone);
    (s, sketch_page, sketch, clone, centre)
}

#[test]
fn criterion_7_the_pasteboard_clone_of_a_sketch_moves_and_turns_on_its_sketch_page() {
    let (mut s, sketch_page, sketch, clone, centre) = sketch_with_pasteboard_clone(106_207);
    let solved = format!("{:?}", s.document.sketch(sketch).unwrap().solution);
    // Its left edge.
    click(&mut s, Point::new(centre.x - 20.0, centre.y), false);
    assert_eq!(s.selection.ids(), [clone]);
    assert_eq!(
        s.active_page().id,
        sketch_page,
        "the sketch page stays active"
    );
    assert_eq!(s.edit_scope(), Scope::MoveOnly);
    let steps = s.undo_len();
    key(&mut s, "ArrowRight", false);
    assert_eq!(s.undo_len(), steps + 1, "one step");
    let moved = crate::session::sketch_use::tests::centre_of(&s, clone);
    assert!((moved.x - centre.x - 1.0).abs() < 1e-6, "{moved:?}");
    key(&mut s, "r", false);
    s.commit_transform_entry("90", "", 0);
    let ObjectSnapshot::Clone(turned) = s.document.object(clone).unwrap() else {
        panic!("a clone");
    };
    assert!(turned.chain.matrix.a.abs() < 1e-9, "turned by 90 degrees");
    assert_eq!(
        format!("{:?}", s.document.sketch(sketch).unwrap().solution),
        solved,
        "the sketch is never written"
    );
    assert_ne!(
        s.sketch_page_drawing_overlay().triangles,
        [],
        "box and handles"
    );
}

#[test]
fn criterion_3_sketch_geometry_wins_and_the_two_kinds_are_never_selected_together() {
    let (mut s, sketch_page, _, clone, centre) = sketch_with_pasteboard_clone(1_062_031);
    click(&mut s, Point::new(centre.x - 20.0, centre.y), false);
    assert_eq!(s.selection.ids(), [clone]);
    // The sketch's own left edge, on its page.
    let page = crate::session::sketch_use::tests::page(&s, &sketch_page);
    let sketch_edge = Point::new(
        page.origin.x + page.size.width.as_mm() / 2.0 - 20.0,
        page.origin.y + page.size.height.as_mm() / 2.0,
    );
    click(&mut s, sketch_edge, true);
    assert!(
        !s.selection.contains(clone),
        "pressing sketch geometry replaces the selection"
    );
    assert!(!s.sketch.entities.is_empty());
}

#[test]
fn criterion_16_a_double_click_on_a_drawing_object_over_the_sketch_page_opens_its_page() {
    let (mut s, sketch_page, _, _, _) = sketch_with_pasteboard_clone(1_062_161);
    let page = crate::session::sketch_use::tests::page(&s, &sketch_page);
    // A square over the sketch page's frame, clear of the sketch: a pasteboard object.
    let over = square(&s, Point::new(page.origin.x + 5.0, page.origin.y + 5.0));
    let edge = centre(Point::new(page.origin.x + 5.0, page.origin.y + 5.0));
    click(&mut s, edge, false);
    assert_eq!(s.selection.ids(), [over]);
    assert_eq!(s.active_page().id, sketch_page);
    s.double_click(edge, false, false);
    assert_eq!(
        s.active_page().id,
        FIRST_PAGE_ID,
        "the last active drawing page"
    );
    assert_eq!(s.selection.ids(), [over]);
    assert_eq!(s.edit_scope(), Scope::Edit);
}

// ---- the laser page's own Layers tree (criteria 17 to 19) ----------------------------------

fn tree_rows(s: &mut Session) -> Vec<serde_json::Value> {
    let view: serde_json::Value = serde_json::from_str(&s.layers_view_json(0, 60)).unwrap();
    view["rows"].as_array().unwrap().clone()
}

#[test]
fn criteria_17_to_19_a_laser_page_lists_its_own_tree() {
    let mut s = Session::new(106_217);
    let source = square(&s, Point::new(10.0, 10.0));
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let l = s.active_page();
    let item = square(
        &s,
        Point::new(
            l.origin.x + l.size.width.as_mm() - 40.0,
            l.origin.y + l.size.height.as_mm() - 40.0,
        ),
    );
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    let placed = s.selected_placement().unwrap();
    s.selection.clear();
    let rows = tree_rows(&mut s);
    let kinds: Vec<&str> = rows.iter().map(|r| r["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["page-root", "rectangle", "placed-page", "fixtures"]);
    assert_eq!(rows[0]["name"], l.name);
    assert_eq!(rows[1]["id"], serde_json::to_value(item).unwrap());
    assert_eq!(rows[2]["name"], "Placed page of Page 1");
    assert!(kinds.iter().all(|k| *k != "layer"), "no document layer");
    // Expanded: the source page's members, live and read-only, through the placed page.
    let id = serde_json::to_string(&placed).unwrap();
    assert!(s.set_layer_expanded(&id, true));
    let rows = tree_rows(&mut s);
    assert_eq!(rows[3]["id"], serde_json::to_value(source).unwrap());
    assert_eq!(rows[3]["readOnly"], true);
    assert_eq!(rows[3]["via"], serde_json::to_value(placed).unwrap());
    assert_eq!(rows[3]["level"], 3);
    // Pressing the inner row selects the placed page.
    s.press_layer_row(&id, crate::session::layers_panel::RowPress::Plain);
    assert_eq!(s.selection.ids(), [placed]);
    // Delete removes the reference only.
    s.delete_selected();
    assert!(s.document.placed_page(placed).is_none());
    assert!(s.document.object(source).is_some());
    // A drawing page keeps the document's layer tree.
    s.activate_page(FIRST_PAGE_ID);
    let rows = tree_rows(&mut s);
    assert!(rows.iter().all(|r| r["kind"] != "page-root"));
}

// ---- what a move-only selection shows outside the canvas (UX notes §2, §3, §5) ----------------

#[test]
fn the_announcer_the_home_label_and_the_layers_rows_name_the_home() {
    let (mut s, b, _, on_b) = two_pages(106_220);
    s.selection.set(&[on_b]);
    assert!(
        s.selection_announcement().ends_with(", on Page 2"),
        "{}",
        s.selection_announcement()
    );
    let pages: Vec<serde_json::Value> = serde_json::from_str(&s.pages_json()).unwrap();
    let home = |id: &str| pages.iter().find(|p| p["id"] == id).unwrap()["home"].clone();
    assert_eq!(home(&b.id), true);
    assert_eq!(home(FIRST_PAGE_ID), false);
    // The Layers row of B's square says where it lies; the Opacity field is hidden.
    let view: serde_json::Value = serde_json::from_str(&s.layers_view_json(0, 60)).unwrap();
    let row = view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == serde_json::to_value(on_b).unwrap())
        .unwrap()
        .clone();
    assert_eq!(row["homeLine"], "On Page 2");
    assert_eq!(view["opacity"]["field"], false);
    // In context nothing of this shows.
    s.selection.clear();
    assert!(!s.selection_announcement().contains(", on "));
}

// ---- handle drags out of context (criteria 5 and 7) ------------------------------------------

/// Drags the first rotate handle of the selection a quarter turn about the box centre.
fn rotate_by_handle(s: &mut Session) {
    let objects = s.objects();
    let handles = curvyo_ui_core::SelectTool::transform_handles(
        &objects,
        &s.selection,
        s.transform_handle_tolerances(),
        false,
    );
    let (_, at) = handles
        .iter()
        .find(|(h, _)| matches!(h, curvyo_ui_core::EditHandle::Rotate(_)))
        .copied()
        .unwrap();
    let id = s.selection.ids()[0];
    let (min, max) = curvyo_ui_core::object_bounds(s.document.object(id).as_ref().unwrap());
    let c = Point::new(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y));
    // A quarter turn of the handle about the centre.
    let to = Point::new(c.x - (at.y - c.y), c.y + (at.x - c.x));
    s.pointer_hover(at, false, false);
    s.pointer_down(at, false);
    let mid = Point::new(f64::midpoint(at.x, to.x), f64::midpoint(at.y, to.y));
    s.pointer_hover(mid, false, false);
    s.pointer_hover(to, false, false);
    s.pointer_up(to, false, false);
}

#[test]
fn criterion_5_an_out_of_context_ellipse_turned_by_its_handle_stays_a_primitive() {
    let mut s = Session::new(106_251);
    let b = s.add_page();
    let b = s.document.pages().into_iter().find(|p| p.id == b).unwrap();
    let ellipse = s.document.create_ellipse(EllipseFrame::from_corners(
        Point::new(b.origin.x + 40.0, b.origin.y + 40.0),
        Point::new(b.origin.x + 120.0, b.origin.y + 80.0),
    ));
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    s.set_tool(Tool::Select);
    s.selection.set(&[ellipse]);
    s.sync_history();
    let steps = s.undo_len();
    let before = format!("{:?}", s.document.object(ellipse));
    rotate_by_handle(&mut s);
    assert_ne!(
        format!("{:?}", s.document.object(ellipse)),
        before,
        "turned"
    );
    assert!(matches!(
        s.document.object(ellipse),
        Some(ObjectSnapshot::Primitive(p)) if matches!(p.shape, Shape::Ellipse { .. })
    ));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(s.active_page().id, laser, "the laser page stays active");
}

#[test]
fn criterion_7_the_pasteboard_sketch_clone_turns_by_its_handle_on_its_sketch_page() {
    let (mut s, sketch_page, sketch, clone, centre) = sketch_with_pasteboard_clone(106_271);
    let solved = format!("{:?}", s.document.sketch(sketch).unwrap().solution);
    click(&mut s, Point::new(centre.x - 20.0, centre.y), false);
    assert_eq!(s.selection.ids(), [clone]);
    rotate_by_handle(&mut s);
    let ObjectSnapshot::Clone(turned) = s.document.object(clone).unwrap() else {
        panic!("a clone");
    };
    assert!(
        turned.chain.matrix.a.abs() < 0.2,
        "turned about a quarter: {:?}",
        turned.chain.matrix
    );
    assert_eq!(s.active_page().id, sketch_page);
    assert_eq!(
        format!("{:?}", s.document.sketch(sketch).unwrap().solution),
        solved,
        "the sketch is never written"
    );
}

// ---- UX review 2026-11-09 -----------------------------------------------------------------------

#[test]
fn criterion_10_the_node_tool_press_shows_the_box_and_its_double_click_opens_the_page() {
    let (mut s, b, _, on_b) = two_pages(106_261);
    s.set_tool(Tool::Node);
    let edge = Point::new(b.origin.x + 10.0, b.origin.y + 20.0);
    click(&mut s, edge, false);
    assert_eq!(s.selection.ids(), [on_b]);
    assert_eq!(s.tool(), Tool::Node);
    let objects = s.objects();
    assert!(
        !s.select_transform_decoration_input_in(&objects)
            .handles
            .is_empty(),
        "the box has the handles of 0019 in the Node tool"
    );
    // A drag in the Node tool writes nothing (the tester's ac9); moving is the Select tool's.
    let before = origin_of(&s, on_b);
    s.pointer_down(edge, false);
    s.pointer_hover(Point::new(edge.x + 10.0, edge.y), false, false);
    s.pointer_up(Point::new(edge.x + 10.0, edge.y), false, false);
    assert_eq!(origin_of(&s, on_b), before);
    assert_eq!(s.active_page().id, FIRST_PAGE_ID);
    click(&mut s, edge, false);
    s.double_click(edge, false, false);
    assert_eq!(s.active_page().id, b.id);
    assert_eq!(s.selection.ids(), [on_b]);
}

#[test]
fn a_laser_page_with_one_placed_page_counts_one_object_and_its_style_has_no_sections() {
    let mut s = Session::new(106_262);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(s.place_page_on_active(FIRST_PAGE_ID));
    assert!(s.selected_placement().is_some());
    assert_eq!(s.layers_subject(), "1 object");
    assert!(
        s.style_panel_state().is_none(),
        "no Stroke or Fill for a placed page"
    );
}

#[test]
fn a_drawing_page_row_of_a_laser_item_says_on_laser_1_and_linked_clone_is_silent() {
    let mut s = Session::new(106_263);
    let laser = s.add_page_of_kind("curvyo.laser", None).unwrap();
    let l = s.active_page();
    let item = square(&s, Point::new(l.origin.x + 10.0, l.origin.y + 10.0));
    s.activate_page(FIRST_PAGE_ID);
    let view: serde_json::Value = serde_json::from_str(&s.layers_view_json(0, 60)).unwrap();
    let row = view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == serde_json::to_value(item).unwrap())
        .unwrap()
        .clone();
    assert_eq!(
        row["homeLine"],
        format!(
            "On {}",
            s.document
                .pages()
                .iter()
                .find(|p| p.id == laser)
                .unwrap()
                .name
        )
    );
    // No notice after a success (customer rule 2026-10-31).
    s.set_tool(Tool::Select);
    let a = square(&s, Point::new(10.0, 10.0));
    s.selection.set(&[a]);
    let made = s.linked_clone();
    assert_eq!(made.notice, "");
    assert_eq!(s.selection.ids().len(), 1);
    assert_ne!(s.selection.ids(), [a]);
}
