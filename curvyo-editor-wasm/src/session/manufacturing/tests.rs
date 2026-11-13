//! The session side of laser pages (`specs/0098-page-kinds`, with `0093` and `0026` as amended):
//! the kind list and chips, the workbench and tabs following the active page, the laser page in
//! the plane with the vector tools, placements, fixtures, the extension switch, delete and undo,
//! the page switch keys and the profile's work area.
#![allow(clippy::assert_is_empty)]

use curvyo_document_core::{Length, Point, RectBounds};
use serde_json::Value;

use super::super::{KeyInput, KeyOutcome, Session, Tool, Workbench};

/// The format-36 laser golden: a rectangle on page 1, a laser of 600 x 400 mm placing page 1 and
/// an engraved grid shown and one hidden.
const LASER_V36: &[u8] =
    include_bytes!("../../../../curvyo-document-core/tests/fixtures/laser_workbench_v36.curvyo");

fn session_with_drawing() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.show_default_view();
    let _ = session.document.create_rect(RectBounds::from_corners(
        Point::new(20.0, 20.0),
        Point::new(60.0, 50.0),
    ));
    session
}

fn registry(session: &Session) -> Value {
    serde_json::from_str(&session.workbenches_json()).unwrap()
}

fn view(session: &Session) -> Value {
    serde_json::from_str(&session.manufacturing_view_json()).unwrap()
}

fn rows(session: &Session) -> Vec<Value> {
    serde_json::from_str(&session.pages_json()).unwrap()
}

fn key(session: &mut Session, k: &str, ctrl: bool) -> KeyOutcome {
    session.key_down(KeyInput {
        key: k,
        ctrl,
        ..KeyInput::default()
    })
}

fn triangles(session: &Session) -> usize {
    session.frame_draw_list().triangles.len()
}

fn tab_names(session: &Session) -> Vec<&'static str> {
    session.panel_view().tabs.iter().map(|t| t.name).collect()
}

fn add_laser(session: &mut Session) -> String {
    session.add_page_of_kind("curvyo.laser", None).unwrap()
}

#[test]
fn ac1_the_kind_list_is_drawing_sketch_and_one_entry_per_enabled_extension() {
    let session = session_with_drawing();
    let json = registry(&session);
    let ids: Vec<&str> = json["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["drawing", "sketch", "curvyo.laser"]);
    assert_eq!(json["kinds"][2]["label"], "Laser");
    assert_eq!(json["kinds"][2]["icon"], "Zap");
    assert_eq!(json["active"], "vector");
    assert_eq!(json["page"]["word"], "Drawing");
    assert_eq!(session.manufacturing_view_json(), "null");
}

#[test]
fn ac4_add_laser_page_is_one_step_active_and_undo_removes_it() {
    let mut session = session_with_drawing();
    let steps = session.history_len();
    let page = add_laser(&mut session);
    assert_eq!(session.history_len(), steps + 1);
    assert_eq!(session.active_page().id, page);
    assert!(matches!(session.workbench(), Workbench::Manufacturing(_)));
    let row = rows(&session)
        .into_iter()
        .find(|r| r["id"] == page.as_str())
        .unwrap();
    assert_eq!(row["name"], "Laser 1");
    assert_eq!(row["kindWord"], "Laser");
    assert_eq!(row["kindIcon"], "Zap");
    assert_eq!(row["machine"], "Generic laser");
    assert_eq!(row["fixedSize"], true);
    assert_eq!(view(&session)["status"], "Generic laser · 400.0 × 300.0 mm");
    assert_eq!(view(&session)["name"], "Laser 1");
    session.undo();
    assert_eq!(session.workbench(), Workbench::Vector);
    assert_eq!(session.document.manufacturings(), Vec::new());
}

#[test]
fn ac8_the_tabs_follow_the_page_kind() {
    let mut session = session_with_drawing();
    let drawing = tab_names(&session);
    assert!(!drawing.contains(&"sketches") && !drawing.contains(&"parameters"));
    assert!(!drawing.contains(&"workbench"));
    add_laser(&mut session);
    let laser = tab_names(&session);
    assert_eq!(
        laser
            .iter()
            .filter(|n| **n != "operations" && **n != "materials")
            .copied()
            .collect::<Vec<_>>(),
        drawing,
        "a laser page has the drawing tabs and the Operations tab"
    );
    assert!(laser.contains(&"operations"), "{laser:?}");
    session.add_page_of_kind("sketch", None).unwrap();
    assert_eq!(session.workbench(), Workbench::Sketch);
    let sketch = tab_names(&session);
    assert!(sketch.contains(&"sketches") && sketch.contains(&"parameters"));
}

#[test]
fn ac9_a_missing_tab_hands_over_to_the_last_tab_of_that_kind_else_document() {
    let mut session = session_with_drawing();
    let first = session.active_page().id;
    assert!(session.press_panel_tab("layers"));
    let sketch = session.add_page_of_kind("sketch", None).unwrap();
    assert_eq!(session.panel_tabs.get().active().name(), "layers", "kept");
    assert!(session.press_panel_tab("sketches"));
    assert!(session.activate_page(&first));
    assert_eq!(session.panel_tabs.get().active().name(), "layers");
    assert!(session.activate_page(&sketch));
    assert_eq!(session.panel_tabs.get().active().name(), "layers");
    assert!(session.press_panel_tab("parameters"));
    session.kind_tabs.clear();
    assert!(session.activate_page(&first));
    assert_eq!(session.panel_tabs.get().active().name(), "document");
}

#[test]
fn ac11_the_vector_tools_draw_on_a_laser_page() {
    let mut session = session_with_drawing();
    let page = add_laser(&mut session);
    let frame = session.active_page();
    assert_eq!(frame.id, page);
    let before = session.document.object_ids().len();
    session.set_tool(Tool::Rectangle);
    let a = Point::new(frame.origin.x + 20.0, frame.origin.y + 20.0);
    let b = Point::new(frame.origin.x + 80.0, frame.origin.y + 60.0);
    session.pointer_down(a, false);
    session.pointer_hover(b, false, false);
    session.pointer_up(b, false, false);
    assert_eq!(session.document.object_ids().len(), before + 1);
    assert_eq!(session.selection.ids().len(), 1);
    assert_eq!(session.active_page().id, page);
}

#[test]
fn ac8_a_press_in_a_frame_of_another_kind_only_activates_it() {
    let mut session = session_with_drawing();
    let first = session.active_page();
    add_laser(&mut session);
    session.set_tool(Tool::Rectangle);
    let objects = session.document.object_ids().len();
    let inside = Point::new(first.origin.x + 100.0, first.origin.y + 100.0);
    session.pointer_down(inside, false);
    session.pointer_up(inside, false, false);
    assert_eq!(session.active_page().id, first.id);
    assert_eq!(session.workbench(), Workbench::Vector);
    assert_eq!(
        session.document.object_ids().len(),
        objects,
        "nothing drawn"
    );
}

#[test]
fn ac8_ctrl_page_down_and_up_walk_the_list_and_stop_at_the_ends() {
    let mut session = session_with_drawing();
    let first = session.active_page().id;
    let sketch = session.add_page_of_kind("sketch", None).unwrap();
    let laser = add_laser(&mut session);
    let steps = session.history_len();
    assert_eq!(key(&mut session, "PageDown", true), KeyOutcome::Consumed);
    assert_eq!(session.active_page().id, laser, "already at the end");
    assert_eq!(key(&mut session, "PageUp", true), KeyOutcome::Consumed);
    assert_eq!(session.active_page().id, sketch);
    assert_eq!(session.workbench(), Workbench::Sketch);
    assert_eq!(key(&mut session, "PageUp", true), KeyOutcome::Consumed);
    assert_eq!(session.active_page().id, first);
    assert!(!session.activate_adjacent_page(false));
    assert_eq!(
        session.history_len(),
        steps,
        "activation is no history step"
    );
}

#[test]
fn ac9_ac10_an_engraved_grid_is_drawn_on_the_page_and_its_eye_hides_it() {
    let mut session = session_with_drawing();
    add_laser(&mut session);
    let bare = triangles(&session);
    assert!(session.add_fixture("engraved_grid").is_some());
    let with_grid = triangles(&session);
    assert!(with_grid > bare);
    let fixture = view(&session)["fixtures"][0].clone();
    // "<Kind> N" since `0102` criterion 2.
    assert_eq!(fixture["name"], "Grid 1");
    let id = session
        .fixture_named(fixture["id"].as_str().unwrap())
        .unwrap();
    // `0102` criterion 15: the spacing is 1 to 100 mm and the row is the Fixtures block's.
    assert!(session.set_fixture_length(id, "spacing", Length::from_mm(0.1)));
    assert_eq!(view(&session)["fixtures"][0]["grid"]["spacing_mm"], 1.0);
    assert!(session.set_fixture_visible(id, false));
    assert_eq!(triangles(&session), bare);
    assert!(session.delete_fixture(id));
    assert_eq!(view(&session)["fixtures"], Value::Array(Vec::new()));
}

#[test]
fn ac12_a_placement_from_an_older_file_is_drawn_selected_and_deleted() {
    let mut session = Session::open(3, LASER_V36).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session.show_default_view();
    let made = session.document.manufacturings()[0].clone();
    let page = made.page.clone().unwrap();
    assert!(session.activate_page(&page.id));
    // The rectangle of page 1 is drawn on the laser page at its placed position.
    let at = Point::new(page.origin.x + 25.0, page.origin.y + 5.0);
    let placed = Some(made.placements[0].id);
    session.set_tool(Tool::Select);
    session.pointer_down(at, false);
    session.pointer_up(at, false, false);
    // Since `0100` a placed page is an object of the one selection.
    assert_eq!(session.selected_placement(), placed);
    assert_eq!(session.selection.ids(), &[made.placements[0].id]);
    let steps = session.history_len();
    session.delete_selected();
    assert_eq!(session.history_len(), steps + 1);
    assert!(session.document.manufacturings()[0].placements.is_empty());
    assert_eq!(session.document.pages().len(), 2, "only the reference went");
    session.undo();
    assert_eq!(session.document.manufacturings()[0].placements.len(), 1);
}

#[test]
fn ac18_a_switched_off_extension_keeps_the_page_listed_as_laser_off_with_vector_tools() {
    let mut session = session_with_drawing();
    let page = add_laser(&mut session);
    let text = session.set_extension_enabled("curvyo.laser", false);
    assert!(text.contains("curvyo.laser"));
    assert_eq!(session.active_page().id, page);
    assert_eq!(session.workbench(), Workbench::Vector);
    let row = rows(&session)
        .into_iter()
        .find(|r| r["id"] == page.as_str())
        .unwrap();
    assert_eq!(row["kindWord"], "Laser (off)");
    assert_eq!(row["kindOff"], true);
    let json = registry(&session);
    assert_eq!(
        json["kinds"].as_array().unwrap().len(),
        2,
        "Laser is absent"
    );
    assert_eq!(json["extensions"][0]["name"], "Laser workbench");
    assert_eq!(session.manufacturing_view_json(), "null");
    assert!(session.add_page_of_kind("curvyo.laser", None).is_none());
    session.load_extension_settings("");
    assert!(matches!(session.workbench(), Workbench::Manufacturing(_)));
}

#[test]
fn ac16_delete_page_removes_the_laser_page_and_undo_brings_its_tools_back() {
    let mut session = session_with_drawing();
    let page = add_laser(&mut session);
    session.add_fixture("engraved_grid").unwrap();
    let drawing = session.document.read_objects().objects;
    assert!(session.delete_page(&page));
    assert_eq!(session.workbench(), Workbench::Vector);
    assert_eq!(session.document.read_objects().objects, drawing);
    session.undo();
    assert!(session.activate_page(&page));
    assert!(matches!(session.workbench(), Workbench::Manufacturing(_)));
    assert_eq!(view(&session)["fixtures"].as_array().unwrap().len(), 1);
}

#[test]
fn ac13_the_document_size_of_a_laser_page_cannot_be_typed() {
    let mut session = session_with_drawing();
    add_laser(&mut session);
    let before = session.active_page().size;
    let outcome =
        session.resize_active_page(curvyo_document_core::DocumentSize::from_mm(10.0, 10.0));
    assert_eq!(outcome, super::super::SizeOutcome::Unchanged);
    assert_eq!(session.active_page().size, before);
}

#[test]
fn ac14_ac15_a_profile_work_area_edit_writes_the_library_and_reset_resizes_about_the_corner() {
    let mut session = session_with_drawing();
    let page = add_laser(&mut session);
    let user = session
        .duplicate_machine_profile("generic-laser", [7; 16], 1)
        .unwrap();
    let steps = session.history_len();
    assert_eq!(
        session.set_profile_work_area(&user, Length::from_mm(600.0), Length::from_mm(400.0), 2),
        Some(None)
    );
    assert_eq!(session.history_len(), steps, "a library write, no step");
    assert!(
        session
            .set_profile_work_area(
                "generic-laser",
                Length::from_mm(1.0),
                Length::from_mm(1.0),
                2
            )
            .is_none(),
        "Generic laser is fixed"
    );
    let before = session.active_page();
    assert!(session.pick_machine_profile(&user));
    let after = session.active_page();
    assert_eq!(after.id, page);
    assert_eq!(
        after.size,
        curvyo_document_core::DocumentSize::from_mm(600.0, 400.0)
    );
    assert_eq!(after.origin.x, before.origin.x);
    assert!(
        (after.origin.y + 400.0 - (before.origin.y + 300.0)).abs() < 1e-9,
        "the bottom-left origin corner stays"
    );
    session.set_profile_work_area(&user, Length::from_mm(700.0), Length::from_mm(500.0), 3);
    assert_eq!(view(&session)["machine"]["state"], "differs");
    assert!(session.reset_machine_profile());
    assert_eq!(
        session.active_page().size,
        curvyo_document_core::DocumentSize::from_mm(700.0, 500.0)
    );
}

#[test]
fn picking_generic_laser_removes_the_fixtures_and_undo_restores_them() {
    // `specs/0102-laser-fixtures` criterion 29, ADR 0050 §4 and §6: the pick replaces the
    // page's fixtures by the profile's set, empty for the built-in, in the pick's one step.
    let mut session = session_with_drawing();
    add_laser(&mut session);
    session.add_fixture("engraved_grid").unwrap();
    assert!(session.pick_machine_profile(curvyo_laser_core::GENERIC_LASER_ID));
    assert!(view(&session)["fixtures"].as_array().unwrap().is_empty());
    session.undo();
    assert_eq!(view(&session)["fixtures"].as_array().unwrap().len(), 1);
}

#[test]
fn placement_measures_variable_artwork_at_once_and_follows_its_profile() {
    use curvyo_document_core::{AnchorId, NewAnchor};
    let session = Session::open(3, LASER_V36).unwrap();
    let id = session.document.create_path(
        &[
            NewAnchor::corner(AnchorId::new(3, 101), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(3, 102), Point::new(30.0, 10.0)),
        ],
        false,
    );
    session
        .document
        .set_width_point(id, AnchorId::new(3, 103), 0.5, 2.0)
        .unwrap();
    let made = session.document.manufacturings()[0].clone();
    let wide = session.placed_bounds_of(&made).unwrap();
    assert!(session.take_unsupported_width_notice().is_empty());
    let _ = session.frame_draw_list();
    session
        .document
        .set_width_point(id, AnchorId::new(3, 103), 0.5, 1.0)
        .unwrap();
    let narrow = session.placed_bounds_of(&made).unwrap();
    assert!(narrow.1.y - narrow.0.y < wide.1.y - wide.0.y);
}

#[test]
fn placement_tessellation_uses_the_largest_nonuniform_scale() {
    use curvyo_document_core::Affine;
    let map = Affine::new(10.0, 0.0, 0.0, 0.1, 0.0, 0.0);
    assert!((super::frame::linear_scale(map) - 10.0).abs() < 1e-12);
    let rotated = Affine::new(0.0, 10.0, -0.1, 0.0, 0.0, 0.0);
    assert!((super::frame::linear_scale(rotated) - 10.0).abs() < 1e-12);
}

#[test]
fn enlarged_placement_requests_tolerance_in_source_space() {
    use curvyo_document_core::Affine;
    let tolerance = super::frame::placement_source_tolerance;
    assert!(tolerance(Affine::scale(2.0, 2.0)).unwrap() <= 0.005);
    assert!(tolerance(Affine::scale(10.0, 0.1)).unwrap() <= 0.001);
    assert!(tolerance(Affine::scale(0.5, 0.8)).unwrap() <= 0.01);
    assert!(tolerance(Affine::scale(f64::NAN, 1.0)).is_none());
}
