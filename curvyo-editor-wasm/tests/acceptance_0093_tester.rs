//! Independent acceptance tests for `specs/0093-laser-workbench-shell` through the public session:
//! the registry and Add manufacturing (criteria 1, 2), the output page and work area (3), what the
//! Laser workbench shows and hides (4), the default and live placement (5, 6), read-only placed
//! content (7), overhang (8), fixtures (9, 10), the extension switch (11b), save and reopen with
//! an unknown process (12) and delete with undo (13). Placed content is found in the Laser frame
//! by its red fill; the output page's top-left corner is the frame's origin (ADR 0032 §5).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

use std::io::{Cursor, Read};

use curvyo_document_core::{
    AnchorId, Color, Document, DocumentSize, Length, NewAnchor, NewManufacturing, NodeId, Point,
    RectBounds, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, Session, SvgExportResult, Tool, Workbench};
use curvyo_render_core::DrawList;
use curvyo_ui_core::{StyleField, SvgExportOptions};
use serde_json::Value;

const LASER: &str = "curvyo.laser";
const TOL: f64 = 1e-6;

// ---- helpers ------------------------------------------------------------------------------------

type Box4 = (f64, f64, f64, f64);

fn red_rect(d: &Document, a: (f64, f64), b: (f64, f64)) -> NodeId {
    let id = d.create_rect(RectBounds::from_corners(
        Point::new(a.0, a.1),
        Point::new(b.0, b.1),
    ));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[id], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    d.edit_style(&[id], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    id
}

/// A drawing whose second page sits away from the plane origin, holding a red 40 x 20 mm
/// rectangle 10 mm right and 20 mm below that page's top-left corner, and a red rectangle on the
/// first page that must never appear in a placement of the second. The second page is active.
fn two_pages() -> (Session, String, Point) {
    let d = Document::new(1);
    let second = d.add_page();
    let o = second.origin;
    assert!(
        o.x.abs() > 1.0 || o.y.abs() > 1.0,
        "the second page is off the origin"
    );
    red_rect(&d, (o.x + 10.0, o.y + 20.0), (o.x + 50.0, o.y + 40.0));
    red_rect(&d, (20.0, 20.0), (60.0, 60.0));
    let mut s = Session::open(9200, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert!(s.activate_page(&second.id));
    (s, second.id, o)
}

/// One page at the origin with a red 40 x 20 rectangle at (10, 20).
fn one_page() -> Session {
    let d = Document::new(1);
    red_rect(&d, (10.0, 20.0), (50.0, 40.0));
    Session::open(9201, &pack(&d, "0.1.0").unwrap()).unwrap()
}

fn is_red(v: &curvyo_render_core::Vertex) -> bool {
    v.color.r > 200 && v.color.g < 60 && v.color.b < 60 && v.color.a > 0
}

fn is_blue(v: &curvyo_render_core::Vertex) -> bool {
    v.color.b > 200 && v.color.r < 60 && v.color.g < 60 && v.color.a > 0
}

fn bbox_of(list: &DrawList, pick: fn(&curvyo_render_core::Vertex) -> bool) -> Option<Box4> {
    list.triangles
        .iter()
        .filter(|v| pick(v))
        .fold(None, |acc, v| {
            let (x, y) = (v.position.x, v.position.y);
            Some(match acc {
                None => (x, y, x, y),
                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            })
        })
}

fn red_box(s: &Session) -> Option<Box4> {
    bbox_of(&s.frame_draw_list(), is_red)
}

fn close(a: Box4, b: Box4, what: &str) {
    for (x, y) in [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)] {
        assert!((x - y).abs() < 0.05, "{what}: {a:?} vs {b:?}");
    }
}

fn workbenches(s: &Session) -> Value {
    serde_json::from_str(&s.workbenches_json()).unwrap()
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.manufacturing_view_json()).unwrap()
}

fn active(s: &Session) -> String {
    workbenches(s)["active"].as_str().unwrap().to_owned()
}

fn switch(s: &mut Session, name: &str) {
    let w = s.workbench_named(name).unwrap();
    s.set_workbench(w);
}

fn to_vector(s: &mut Session) {
    s.set_workbench(Workbench::Vector);
}

fn key(s: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    s.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

fn drag(s: &mut Session, a: Point, b: Point) {
    s.pointer_down(a, false);
    s.pointer_hover(b, false, false);
    s.pointer_up(b, false, false);
}

fn opened(s: &Session) -> Document {
    unpack(9299, &s.pack("0.1.0").unwrap()).unwrap()
}

fn drawing(s: &Session) -> Vec<curvyo_document_core::ObjectSnapshot> {
    opened(s).read_objects().objects.to_vec()
}

fn svg(s: &Session) -> String {
    match s.export_svg(SvgExportOptions::default()) {
        SvgExportResult::Written { text, .. } => text,
        SvgExportResult::NothingSelected => panic!("nothing selected"),
    }
}

/// The page rows without their screen rectangles (each workbench keeps its own view).
fn page_rows(s: &Session) -> Value {
    let mut rows: Value = serde_json::from_str(&s.pages_json()).unwrap();
    for row in rows.as_array_mut().unwrap() {
        let row = row.as_object_mut().unwrap();
        row.retain(|k, _| !k.starts_with("screen"));
    }
    rows
}

fn mm(v: f64) -> Length {
    Length::from_mm(v)
}

fn add_laser(s: &mut Session) -> NodeId {
    s.add_manufacturing(LASER).expect("Laser can be added")
}

fn fixture_ids(s: &Session) -> Vec<String> {
    view(s)["fixtures"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["id"].as_str().unwrap().to_owned())
        .collect()
}

// ---- criterion 1 --------------------------------------------------------------------------------

#[test]
fn ac1_a_new_project_offers_laser_only_through_add_manufacturing() {
    let s = Session::new(9202);
    let w = workbenches(&s);
    let ids: Vec<&str> = w["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        vec!["vector", "sketch"],
        "no Laser entry without a manufacturing"
    );
    let add = w["add"].as_array().unwrap();
    assert_eq!(add.len(), 1);
    assert_eq!(add[0]["label"], "Laser");
    assert_eq!(add[0]["process"], LASER);
    assert_eq!(w["active"], "vector");
    assert_eq!(s.manufacturing_view_json(), "null");
    assert!(
        opened(&s).manufacturings().is_empty(),
        "nothing added at start"
    );
}

#[test]
fn ac1_an_unknown_process_cannot_be_added() {
    let mut s = Session::new(9203);
    let steps = s.undo_len();
    assert!(s.add_manufacturing("acme.cnc").is_none());
    assert!(s.add_manufacturing("").is_none());
    assert_eq!(s.undo_len(), steps);
    assert_eq!(active(&s), "vector");
}

// ---- criterion 2 --------------------------------------------------------------------------------

#[test]
fn ac2_add_laser_makes_laser_1_shows_it_and_is_one_undo_step() {
    let mut s = one_page();
    let pages = s.pages_json();
    let steps = s.undo_len();
    let id = add_laser(&mut s);
    assert_eq!(s.undo_len(), steps + 1, "one undo step");
    assert_eq!(active(&s), id.full_id(), "the Laser workbench is shown");
    let v = view(&s);
    assert_eq!(v["name"], "Laser 1");
    assert_eq!(v["values"]["host.machine_name"], "Generic laser");
    assert_eq!(v["values"]["host.work_width"], 400.0);
    assert_eq!(v["values"]["host.work_height"], 300.0);
    let entry = workbenches(&s)["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id.full_id().as_str())
        .cloned()
        .unwrap();
    assert_eq!(entry["label"], "Laser 1");
    assert_eq!(entry["enabled"], true);

    s.undo();
    assert_eq!(
        active(&s),
        "vector",
        "undo returns to the workbench it came from"
    );
    assert!(opened(&s).manufacturings().is_empty());
    assert_eq!(s.pages_json(), pages, "the output page is gone again");
    assert_eq!(workbenches(&s)["entries"].as_array().unwrap().len(), 2);
    s.redo();
    assert_eq!(opened(&s).manufacturings().len(), 1);
}

#[test]
fn ac2_a_second_laser_is_laser_2() {
    let mut s = one_page();
    add_laser(&mut s);
    to_vector(&mut s);
    let b = add_laser(&mut s);
    assert_eq!(view(&s)["name"], "Laser 2");
    assert_eq!(active(&s), b.full_id());
}

// ---- criterion 3 --------------------------------------------------------------------------------

#[test]
fn ac3_work_area_edit_resizes_the_output_page_keeps_content_and_is_one_step() {
    let mut s = one_page();
    add_laser(&mut s);
    let placed = red_box(&s).unwrap();
    let steps = s.undo_len();
    assert!(s.set_work_area(mm(600.0), mm(400.0)));
    assert_eq!(s.undo_len(), steps + 1);
    let v = view(&s);
    assert_eq!(v["values"]["host.work_width"], 600.0);
    assert_eq!(v["values"]["host.work_height"], 400.0);
    let d = opened(&s);
    let m = &d.manufacturings()[0];
    assert_eq!(
        m.page.as_ref().unwrap().size,
        DocumentSize::from_mm(600.0, 400.0)
    );
    close(
        red_box(&s).unwrap(),
        placed,
        "content keeps its place from the top-left",
    );
    assert!(!s.set_work_area(mm(600.0), mm(400.0)), "no change, no step");
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(view(&s)["values"]["host.work_width"], 400.0);
}

#[test]
fn ac3_out_of_range_work_areas_are_clamped_and_nan_writes_nothing() {
    let mut s = one_page();
    add_laser(&mut s);
    assert!(s.set_work_area(mm(0.0), mm(1e9)));
    let v = view(&s);
    assert_eq!(v["values"]["host.work_width"], 1.0);
    assert_eq!(v["values"]["host.work_height"], 10_000.0);
    let steps = s.undo_len();
    let _ = s.set_work_area(mm(f64::NAN), mm(300.0));
    let _ = s.set_work_area(mm(f64::INFINITY), mm(f64::NEG_INFINITY));
    let v = view(&s);
    assert!(v["values"]["host.work_width"].as_f64().unwrap().is_finite());
    assert!(
        v["values"]["host.work_height"]
            .as_f64()
            .unwrap()
            .is_finite()
    );
    let d = opened(&s);
    let size = d.manufacturings()[0].work_area();
    assert!(size.width.as_mm().is_finite() && size.height.as_mm().is_finite());
    assert!(s.undo_len() <= steps + 2);
}

#[test]
fn ac3_work_area_does_nothing_in_vector() {
    let mut s = one_page();
    add_laser(&mut s);
    to_vector(&mut s);
    let steps = s.undo_len();
    assert!(!s.set_work_area(mm(600.0), mm(400.0)));
    assert_eq!(s.undo_len(), steps);
}

// ---- criterion 4 --------------------------------------------------------------------------------

#[test]
fn ac4_laser_shows_the_output_page_and_only_laser_file_history_tabs() {
    let (mut s, _, _) = two_pages();
    add_laser(&mut s);
    let tabs = s.panel_view().tabs;
    let labels: Vec<&str> = tabs.iter().map(|t| t.label).collect();
    assert_eq!(labels[0], "Laser");
    let names: Vec<&str> = tabs.iter().map(|t| t.name).collect();
    assert_eq!(names.len(), 3, "{names:?}");
    assert!(
        names.contains(&"file") && names.contains(&"history"),
        "{names:?}"
    );
    assert!(tabs[0].selected, "entering selects the Laser tab");
    for drawing_tab in ["document", "style", "layers", "fonts"] {
        assert!(!names.contains(&drawing_tab), "{drawing_tab} hidden");
        assert!(
            !s.press_panel_tab(drawing_tab) || s.panel_view().tabs[0].selected,
            "{drawing_tab} cannot take over"
        );
    }
    // Only the placed page: the first page's rectangle at (20..60) is not on the output page.
    let b = red_box(&s).unwrap();
    close(
        b,
        (10.0, 20.0, 50.0, 40.0),
        "only the placed page's content",
    );
}

#[test]
fn ac4_drawing_tool_keys_do_nothing_in_laser() {
    let mut s = one_page();
    add_laser(&mut s);
    let tool = s.tool();
    let before = drawing(&s);
    for k in ["r", "e", "p", "t", "b", "n", "v"] {
        let _ = key(&mut s, k, false);
    }
    assert_eq!(s.tool(), tool);
    assert_eq!(drawing(&s), before);
    assert!(active(&s) != "vector");
}

#[test]
fn ac4_vector_is_unchanged_by_a_manufacturing_page_list_canvas_and_export() {
    let mut s = one_page();
    let frame = s.frame_draw_list();
    let pages = s.pages_json();
    let export = svg(&s);
    add_laser(&mut s);
    let _ = s.add_fixture("engraved_grid");
    to_vector(&mut s);
    assert_eq!(active(&s), "vector");
    assert_eq!(
        s.pages_json(),
        pages,
        "output page not in the Vector page list"
    );
    let after = s.frame_draw_list();
    assert_eq!(
        after.triangles.len(),
        frame.triangles.len(),
        "nothing new on the Vector canvas"
    );
    assert_eq!(after.triangles, frame.triangles);
    assert_eq!(
        svg(&s),
        export,
        "no fixture or output page in the drawing's export"
    );
}

#[test]
fn ac4_each_workbench_keeps_its_own_view() {
    let mut s = one_page();
    s.resize_viewport(1200.0, 800.0);
    let vector_view = s.view();
    let id = add_laser(&mut s);
    let laser_view = s.view();
    to_vector(&mut s);
    assert_eq!(
        s.view(),
        vector_view,
        "entering Laser never changes the Vector view"
    );
    switch(&mut s, &id.full_id());
    assert_eq!(s.view(), laser_view);
}

// ---- criteria 5, 6 ------------------------------------------------------------------------------

#[test]
fn ac5_the_page_is_placed_at_the_top_left_scale_1_rotation_0() {
    let (mut s, second, _) = two_pages();
    let id = add_laser(&mut s);
    close(
        red_box(&s).unwrap(),
        (10.0, 20.0, 50.0, 40.0),
        "page-local position kept",
    );
    let d = opened(&s);
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.placements.len(), 1);
    assert_eq!(m.placements[0].page, second);
    assert_eq!(
        m.placements[0].matrix,
        curvyo_document_core::Affine::IDENTITY
    );
    assert_eq!(d.object_ids().len(), 2, "a reference, not a copy");
}

#[test]
fn ac6_drawing_edits_in_vector_show_in_laser_without_a_command() {
    let (mut s, _, o) = two_pages();
    let id = add_laser(&mut s);
    let laser = id.full_id();
    let n0 = s.frame_draw_list().triangles.len();

    // Move the rectangle 30 mm right and 15 mm down.
    to_vector(&mut s);
    s.set_tool(Tool::Select);
    drag(
        &mut s,
        Point::new(o.x + 30.0, o.y + 30.0),
        Point::new(o.x + 60.0, o.y + 45.0),
    );
    switch(&mut s, &laser);
    close(red_box(&s).unwrap(), (40.0, 35.0, 80.0, 55.0), "moved path");

    // Draw a circle and fill it red.
    to_vector(&mut s);
    s.set_tool(Tool::Ellipse);
    drag(
        &mut s,
        Point::new(o.x + 100.0, o.y + 100.0),
        Point::new(o.x + 140.0, o.y + 140.0),
    );
    s.set_fill_paint(true);
    s.set_style_text(StyleField::FillColor, "#FF0000").unwrap();
    switch(&mut s, &laser);
    let b = red_box(&s).unwrap();
    assert!(b.2 > 135.0 && b.3 > 135.0, "new circle placed: {b:?}");
    assert!(s.frame_draw_list().triangles.len() > n0);

    // Stroke colour of the circle to blue.
    to_vector(&mut s);
    s.set_stroke_paint(true);
    s.set_style_text(StyleField::StrokeColor, "#0000FF")
        .unwrap();
    switch(&mut s, &laser);
    assert!(
        bbox_of(&s.frame_draw_list(), is_blue).is_some(),
        "stroke colour shows"
    );

    // Undo it, in Vector.
    to_vector(&mut s);
    s.undo();
    switch(&mut s, &laser);
    assert!(
        bbox_of(&s.frame_draw_list(), is_blue).is_none(),
        "undo shows too"
    );
}

#[test]
fn ac6_an_undo_of_a_drawing_edit_pressed_in_laser_shows_at_once() {
    let (mut s, _, o) = two_pages();
    let id = add_laser(&mut s);
    to_vector(&mut s);
    s.set_tool(Tool::Select);
    drag(
        &mut s,
        Point::new(o.x + 30.0, o.y + 30.0),
        Point::new(o.x + 60.0, o.y + 45.0),
    );
    switch(&mut s, &id.full_id());
    close(red_box(&s).unwrap(), (40.0, 35.0, 80.0, 55.0), "moved");
    match key(&mut s, "z", true) {
        KeyOutcome::Consumed => {}
        KeyOutcome::UndoBusy => {
            s.undo();
        }
        other => panic!("Ctrl+Z in Laser: {other:?}"),
    }
    close(
        red_box(&s).unwrap(),
        (10.0, 20.0, 50.0, 40.0),
        "undone in the next frame",
    );
    assert_eq!(active(&s), id.full_id());
}

#[test]
fn ac6_an_object_moved_off_the_page_leaves_the_placement_and_one_moved_on_joins_it() {
    let (mut s, _, o) = two_pages();
    let id = add_laser(&mut s);
    to_vector(&mut s);
    s.set_tool(Tool::Select);
    // Far away from every page.
    drag(
        &mut s,
        Point::new(o.x + 30.0, o.y + 30.0),
        Point::new(o.x + 30.0, o.y + 5000.0),
    );
    switch(&mut s, &id.full_id());
    assert!(red_box(&s).is_none(), "no longer a member of the page");
}

// ---- criterion 7 --------------------------------------------------------------------------------

#[test]
fn ac7_select_move_delete_style_on_the_output_page_change_nothing_and_say_so() {
    let mut s = one_page();
    add_laser(&mut s);
    let before = drawing(&s);
    let steps = s.undo_len();
    assert!(s.read_only_notice().is_none());

    s.pointer_down(Point::new(30.0, 30.0), false);
    s.pointer_hover(Point::new(130.0, 130.0), false, false);
    s.pointer_up(Point::new(130.0, 130.0), false, false);
    let first = s
        .read_only_notice()
        .expect("a press on the content shows the notice");
    assert_eq!(s.selected_object_count(), 0);
    assert!(!s.double_click(Point::new(30.0, 30.0), false, false));

    for (k, ctrl) in [
        ("Delete", false),
        ("Backspace", false),
        ("a", true),
        ("ArrowLeft", false),
        ("ArrowDown", false),
        ("v", true),
        ("x", true),
        ("d", true),
        ("g", true),
    ] {
        let _ = key(&mut s, k, ctrl);
    }
    assert!(
        s.read_only_notice().unwrap().seq != first.seq,
        "keys show it again"
    );
    assert_eq!(s.selected_object_count(), 0);
    assert_eq!(drawing(&s), before, "nothing in the drawing changed");
    assert_eq!(s.undo_len(), steps);

    let text = view(&s)["read_only_notice"]["text"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(text.chars().count() <= 70, "{text}");
    assert!(text.contains("Vector"), "{text}");
}

/// A drawing selection made in Vector and kept for the way back (UX notes §3).
fn laser_with_carried_selection() -> (Session, Vec<curvyo_document_core::ObjectSnapshot>, usize) {
    let mut s = one_page();
    s.set_tool(Tool::Select);
    key(&mut s, "a", true);
    assert_eq!(s.selected_object_count(), 1);
    let id = add_laser(&mut s);
    assert_eq!(active(&s), id.full_id());
    let before = drawing(&s);
    let steps = s.undo_len();
    (s, before, steps)
}

#[test]
fn ac7_keys_on_a_carried_selection_change_nothing_in_laser() {
    let (mut s, before, steps) = laser_with_carried_selection();
    for (k, ctrl) in [
        ("Delete", false),
        ("Backspace", false),
        ("ArrowRight", false),
        ("x", true),
    ] {
        let _ = key(&mut s, k, ctrl);
    }
    assert_eq!(drawing(&s), before);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac7_the_node_context_menu_delete_in_laser_changes_nothing_in_the_drawing() {
    // Canvas.tsx keeps the Node context menu enabled whenever the tool is Node outside Sketch, so
    // in Laser its Delete calls `delete_selected` on the nodes picked in Vector.
    let d = Document::new(1);
    let pts = [(10.0, 20.0), (50.0, 20.0), (50.0, 40.0), (10.0, 40.0)];
    let anchors: Vec<NewAnchor> = pts
        .iter()
        .enumerate()
        .map(|(i, p)| NewAnchor::corner(AnchorId::new(9207, i as u64 + 1), Point::new(p.0, p.1)))
        .collect();
    let _ = d.create_path(&anchors, true);
    let mut s = Session::open(9207, &pack(&d, "0.1.0").unwrap()).unwrap();
    s.set_tool(Tool::Select);
    s.pointer_down(Point::new(10.0, 30.0), false);
    s.pointer_up(Point::new(10.0, 30.0), false, false);
    s.set_tool(Tool::Node);
    s.pointer_down(Point::new(10.0, 20.0), false);
    s.pointer_up(Point::new(10.0, 20.0), false, false);
    add_laser(&mut s);
    assert_eq!(s.tool(), Tool::Node, "the tool is carried into Laser");
    let before = drawing(&s);
    let steps = s.undo_len();
    s.delete_selected();
    assert_eq!(
        drawing(&s),
        before,
        "placed content cannot be edited in Laser"
    );
    assert_eq!(s.undo_len(), steps);
}

// ---- criterion 8 --------------------------------------------------------------------------------

#[test]
fn ac8_content_past_the_work_area_is_drawn_whole_and_named_in_the_notice() {
    let d = Document::new(1);
    red_rect(&d, (0.0, 0.0), (440.0, 312.0));
    let mut s = Session::open(9204, &pack(&d, "0.1.0").unwrap()).unwrap();
    let before = drawing(&s);
    add_laser(&mut s);
    close(
        red_box(&s).unwrap(),
        (0.0, 0.0, 440.0, 312.0),
        "not clipped",
    );
    assert_eq!(
        view(&s)["notices"]["overhang"],
        "Extends past the work area by 40 x 12 mm"
    );
    assert_eq!(drawing(&s), before, "nothing moved");
    assert!(s.set_work_area(mm(500.0), mm(400.0)));
    assert!(
        view(&s)["notices"].get("overhang").is_none(),
        "notice follows the work area"
    );
}

#[test]
fn ac8_within_the_work_area_no_notice() {
    let mut s = one_page();
    add_laser(&mut s);
    assert!(view(&s)["notices"].get("overhang").is_none());
}

// ---- criteria 9, 10 -----------------------------------------------------------------------------

#[test]
fn ac9_a_new_setup_has_no_fixture_and_the_grid_is_added_with_its_defaults() {
    let mut s = one_page();
    add_laser(&mut s);
    assert!(view(&s)["fixtures"].as_array().unwrap().is_empty());
    let steps = s.undo_len();
    assert!(s.add_fixture("engraved_grid").is_some());
    assert_eq!(s.undo_len(), steps + 1);
    let f = &view(&s)["fixtures"][0];
    assert_eq!(f["name"], "Engraved grid");
    assert_eq!(f["visible"], true);
    assert_eq!(f["values"]["spacing"], 10.0);
    assert_eq!(f["values"]["line_width"], 0.1);
    assert_eq!(f["values"]["origin_x"], 0.0);
    assert_eq!(f["values"]["origin_y"], 0.0);
    assert!(s.add_fixture("pocket").is_none(), "only the declared kind");
    s.add_fixture("engraved_grid").unwrap();
    assert_eq!(view(&s)["fixtures"][1]["name"], "Engraved grid 2");
}

#[test]
fn ac9_grid_lines_are_on_the_output_page_below_the_placed_drawing() {
    let mut s = one_page();
    add_laser(&mut s);
    let plain = s.frame_draw_list();
    let first_red = plain.triangles.iter().position(is_red).unwrap();
    s.add_fixture("engraved_grid").unwrap();
    let with = s.frame_draw_list();
    let extra = with.triangles.len() - plain.triangles.len();
    assert!(extra > 0, "the grid draws lines");
    assert_eq!(
        with.triangles.iter().position(is_red).unwrap(),
        first_red + extra,
        "grid vertices come before (below) the placed drawing"
    );
    let px = 1.0 / s.view().scale();
    for v in &with.triangles[first_red..first_red + extra] {
        assert!(
            v.position.x >= -2.0 * px - TOL
                && v.position.x <= 400.0 + 2.0 * px + TOL
                && v.position.y >= -2.0 * px - TOL
                && v.position.y <= 300.0 + 2.0 * px + TOL,
            "grid on the output page area: {:?}",
            v.position
        );
    }
}

#[test]
fn ac9_a_spacing_edit_redraws_and_is_one_step_and_clamped() {
    let mut s = one_page();
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    let id = s.fixture_named(&fixture_ids(&s)[0]).unwrap();
    let n10 = s.frame_draw_list().triangles.len();
    let steps = s.undo_len();
    assert!(s.set_fixture_length(id, "spacing", mm(50.0)));
    assert_eq!(s.undo_len(), steps + 1);
    assert!(s.frame_draw_list().triangles.len() < n10);
    assert!(s.set_fixture_length(id, "spacing", mm(0.0)));
    assert_eq!(
        view(&s)["fixtures"][0]["values"]["spacing"],
        0.5,
        "clamped to its minimum"
    );
    assert!(!s.set_fixture_length(id, "nonsense", mm(3.0)));
}

#[test]
fn ac10_fixtures_are_never_selectable_and_hide_is_view_state_of_one_step() {
    let mut s = one_page();
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    let id = s.fixture_named(&fixture_ids(&s)[0]).unwrap();
    let shown = s.frame_draw_list().triangles.len();

    // Press on a grid line away from the drawing, marquee over all, Select All.
    s.pointer_down(Point::new(200.0, 200.0), false);
    s.pointer_up(Point::new(200.0, 200.0), false, false);
    drag(&mut s, Point::new(-50.0, -50.0), Point::new(450.0, 350.0));
    let _ = key(&mut s, "a", true);
    assert_eq!(s.selected_object_count(), 0);

    let steps = s.undo_len();
    assert!(s.set_fixture_visible(id, false));
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(view(&s)["fixtures"][0]["visible"], false);
    let hidden = s.frame_draw_list().triangles.len();
    assert!(hidden < shown, "hidden fixture is not drawn");
    s.undo();
    assert_eq!(s.frame_draw_list().triangles.len(), shown);
}

#[test]
fn ac10_in_vector_select_all_marquee_and_click_reach_only_the_drawing() {
    let mut s = one_page();
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    to_vector(&mut s);
    s.set_tool(Tool::Select);
    // On page-local (100, 100), where a grid line lies on the output page.
    s.pointer_down(Point::new(100.0, 100.0), false);
    s.pointer_up(Point::new(100.0, 100.0), false, false);
    assert_eq!(s.selected_object_count(), 0);
    key(&mut s, "a", true);
    assert_eq!(s.selected_object_count(), 1, "only the drawing object");
    s.escape();
    drag(&mut s, Point::new(-50.0, -50.0), Point::new(700.0, 500.0));
    assert_eq!(s.selected_object_count(), 1);
    // Delete everything selectable: the manufacturing stays.
    s.delete_selected();
    let d = opened(&s);
    assert!(d.object_ids().is_empty());
    assert_eq!(d.manufacturings().len(), 1);
    assert_eq!(d.manufacturings()[0].fixtures.len(), 1);
}

#[test]
fn ac10_layers_show_no_manufacturing_fixture_or_placement() {
    let mut s = one_page();
    let layers = s.layers_view_json(0, 100);
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    to_vector(&mut s);
    assert_eq!(s.layers_view_json(0, 100), layers);
}

#[test]
fn ac10_export_from_the_laser_workbench_is_the_drawing_only() {
    let mut s = one_page();
    let export = svg(&s);
    add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    assert_eq!(svg(&s), export);
}

// ---- criterion 11b ------------------------------------------------------------------------------

#[test]
fn ac11b_switched_off_the_laser_entries_go_and_the_manufacturing_stays_disabled() {
    let mut s = one_page();
    let id = add_laser(&mut s);
    s.add_fixture("engraved_grid").unwrap();
    let ext = workbenches(&s)["extensions"].clone();
    let laser = ext
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == LASER)
        .cloned()
        .unwrap();
    assert_eq!(laser["name"], "Laser workbench");
    assert!(!laser["version"].as_str().unwrap().is_empty());
    assert_eq!(laser["enabled"], true);

    let steps = s.undo_len();
    let text = s.set_extension_enabled(LASER, false);
    assert!(text.contains(LASER));
    assert_eq!(s.undo_len(), steps, "not an undo step");
    assert_eq!(
        active(&s),
        "vector",
        "an active disabled workbench hands over to Vector"
    );
    let w = workbenches(&s);
    assert!(
        w["add"].as_array().unwrap().is_empty(),
        "Add manufacturing: Laser disappears"
    );
    assert!(w["manifests"].as_array().unwrap().is_empty());
    let entry = w["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id.full_id().as_str())
        .cloned()
        .expect("kept in the switcher");
    assert_eq!(entry["enabled"], false);
    assert_eq!(entry["label"], "Laser 1");
    assert_eq!(w["disabled"][0]["label"], "Laser 1");
    assert!(s.add_manufacturing(LASER).is_none());
    switch(&mut s, &id.full_id());
    assert_eq!(
        active(&s),
        "vector",
        "a disabled manufacturing does not open"
    );

    let saved = s.pack("0.1.0").unwrap();
    let back = unpack(5, &saved).unwrap();
    assert_eq!(back.manufacturings(), opened(&s).manufacturings());
    assert_eq!(
        back.manufacturings()[0].fixtures.len(),
        1,
        "data kept in the file"
    );

    s.set_extension_enabled(LASER, true);
    switch(&mut s, &id.full_id());
    assert_eq!(active(&s), id.full_id());
}

#[test]
fn ac11b_the_settings_file_switch_is_read_at_load() {
    let mut s = one_page();
    let id = add_laser(&mut s);
    s.load_extension_settings("disabled = [\"curvyo.laser\"]\n");
    assert_eq!(active(&s), "vector");
    assert!(workbenches(&s)["add"].as_array().unwrap().is_empty());
    s.load_extension_settings("");
    switch(&mut s, &id.full_id());
    assert_eq!(active(&s), id.full_id());
}

#[test]
fn ac11b_a_disabled_manufacturing_can_still_be_deleted_and_undone() {
    let mut s = one_page();
    let id = add_laser(&mut s);
    s.set_extension_enabled(LASER, false);
    assert!(s.delete_manufacturing(id));
    assert!(opened(&s).manufacturings().is_empty());
    s.undo();
    assert_eq!(opened(&s).manufacturings().len(), 1);
    assert_eq!(active(&s), "vector", "still disabled, so it does not open");
}

// ---- criterion 12 -------------------------------------------------------------------------------

#[test]
fn ac12_save_and_reopen_keep_everything_and_the_placement_still_follows() {
    let (mut s, second, o) = two_pages();
    let id = add_laser(&mut s);
    assert!(s.set_work_area(mm(600.0), mm(400.0)));
    s.add_fixture("engraved_grid").unwrap();
    s.add_fixture("engraved_grid").unwrap();
    let hidden = s.fixture_named(&fixture_ids(&s)[1]).unwrap();
    assert!(s.set_fixture_visible(hidden, false));
    let before = opened(&s).manufacturings();
    let laser_frame = s.frame_draw_list();

    let mut r = Session::open(9205, &s.pack("0.1.0").unwrap()).unwrap();
    let after = opened(&r).manufacturings();
    assert_eq!(after, before);
    assert_eq!(after[0].placements[0].page, second);
    assert!(!after[0].fixtures[1].visible);
    switch(&mut r, &id.full_id());
    assert_eq!(active(&r), id.full_id());
    assert_eq!(
        r.frame_draw_list().triangles.len(),
        laser_frame.triangles.len()
    );
    // Still live after the reopen.
    to_vector(&mut r);
    assert!(r.activate_page(&second));
    r.set_tool(Tool::Select);
    drag(
        &mut r,
        Point::new(o.x + 30.0, o.y + 30.0),
        Point::new(o.x + 40.0, o.y + 30.0),
    );
    switch(&mut r, &id.full_id());
    close(
        red_box(&r).unwrap(),
        (20.0, 20.0, 60.0, 40.0),
        "follows the drawing",
    );
}

fn stored_vv(bytes: &[u8]) -> loro::VersionVector {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut stored = Vec::new();
    archive
        .by_name("document.loro")
        .unwrap()
        .read_to_end(&mut stored)
        .unwrap();
    let loro = loro::LoroDoc::new();
    loro.import(&stored).unwrap();
    loro.oplog_vv()
}

#[test]
fn ac12_an_unknown_process_opens_disabled_and_saves_back_unchanged() {
    let d = Document::new(1);
    red_rect(&d, (10.0, 20.0), (50.0, 40.0));
    let id = d
        .add_manufacturing(NewManufacturing {
            process: "acme.cnc",
            label: "CNC",
            machine_name: "Router",
            work_area: DocumentSize::from_mm(800.0, 600.0),
            place: Some("first"),
        })
        .unwrap();
    let params = serde_json::from_value(serde_json::json!({"depth": {"mm": 3.0}})).unwrap();
    d.add_fixture(id, "acme.cnc/clamp", "Clamp", &params)
        .unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();

    let mut s = Session::open(9206, &bytes).unwrap();
    let w = workbenches(&s);
    let entry = w["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == id.full_id().as_str())
        .cloned()
        .unwrap();
    assert_eq!(entry["enabled"], false);
    assert_eq!(entry["label"], "CNC 1");
    switch(&mut s, &id.full_id());
    assert_eq!(active(&s), "vector", "an unknown process does not open");
    // Ordinary drawing work around it leaves it alone.
    s.set_tool(Tool::Select);
    key(&mut s, "a", true);
    assert_eq!(s.selected_object_count(), 1);
    s.escape();
    let saved = s.pack("0.1.0").unwrap();
    assert_eq!(stored_vv(&saved), stored_vv(&bytes), "saved back unchanged");
    assert_eq!(
        unpack(3, &saved).unwrap().manufacturings(),
        d.manufacturings()
    );
}

// ---- criterion 13 -------------------------------------------------------------------------------

#[test]
fn ac13_delete_goes_at_once_leaves_the_drawing_and_undo_restores_the_workbench() {
    let (mut s, _, _) = two_pages();
    let id = add_laser(&mut s);
    s.set_work_area(mm(600.0), mm(400.0));
    s.add_fixture("engraved_grid").unwrap();
    let full = opened(&s).manufacturings();
    let laser_frame = s.frame_draw_list();
    let pages = page_rows(&s);
    let objects = drawing(&s);
    let steps = s.undo_len();

    assert!(s.delete_manufacturing(id));
    assert_eq!(s.undo_len(), steps + 1, "one step, no confirm");
    assert_eq!(active(&s), "vector");
    assert!(opened(&s).manufacturings().is_empty());
    assert_eq!(drawing(&s), objects);
    assert_eq!(page_rows(&s), pages);
    assert_eq!(workbenches(&s)["entries"].as_array().unwrap().len(), 2);

    s.undo();
    assert_eq!(opened(&s).manufacturings(), full);
    assert_eq!(
        active(&s),
        id.full_id(),
        "undo restores the Laser workbench"
    );
    assert_eq!(s.frame_draw_list().triangles, laser_frame.triangles);
    assert_eq!(drawing(&s), objects);
    let object = opened(&s).object_ids()[0];
    assert!(
        !s.delete_manufacturing(object),
        "a drawing object is no manufacturing"
    );
    assert_eq!(drawing(&s), objects);
}

#[test]
fn ac13_deleting_the_placed_drawing_page_empties_the_placement_and_undo_brings_it_back() {
    let (mut s, second, _) = two_pages();
    let id = add_laser(&mut s);
    to_vector(&mut s);
    assert!(s.delete_page(&second));
    switch(&mut s, &id.full_id());
    assert!(
        red_box(&s).is_none(),
        "a placement of a deleted page draws nothing"
    );
    assert_eq!(
        opened(&s).manufacturings()[0].placements.len(),
        1,
        "and is kept"
    );
    to_vector(&mut s);
    s.undo();
    switch(&mut s, &id.full_id());
    assert!(red_box(&s).is_some());
}
