//! The session side of `specs/0093-laser-workbench-shell`: registry, switching, the Laser frame,
//! read-only content, notices, fixtures, extension switch, delete and undo.

use curvyo_document_core::{EllipseFrame, Length, Point, RectBounds};
use serde_json::Value;

use super::super::{KeyInput, KeyOutcome, Session, Tool, Workbench};

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

#[test]
fn ac1_without_a_manufacturing_the_switcher_offers_laser_only_through_add() {
    let session = session_with_drawing();
    let json = registry(&session);
    let ids: Vec<&str> = json["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["vector", "sketch"]);
    assert_eq!(json["add"][0]["label"], "Laser");
    assert_eq!(json["active"], "vector");
    assert_eq!(session.manufacturing_view_json(), "null");
}

#[test]
fn ac2_add_manufacturing_opens_laser_1_in_one_step_and_undo_removes_it() {
    let mut session = session_with_drawing();
    let steps = session.history_len();
    let id = session.add_manufacturing("curvyo.laser").unwrap();
    assert_eq!(session.history_len(), steps + 1);
    assert_eq!(session.workbench(), Workbench::Manufacturing(id));
    let json = registry(&session);
    assert_eq!(json["entries"][2]["label"], "Laser 1");
    assert_eq!(json["entries"][2]["icon"], "Zap");
    assert_eq!(view(&session)["status"], "Generic laser · 400.0 × 300.0 mm");
    session.undo();
    assert_eq!(session.workbench(), Workbench::Vector);
    assert_eq!(session.document.manufacturings(), Vec::new());
}

#[test]
fn ac3_the_work_area_edit_resizes_the_output_page_and_is_clamped() {
    let mut session = session_with_drawing();
    session.add_manufacturing("curvyo.laser").unwrap();
    assert!(session.set_work_area(Length::from_mm(600.0), Length::from_mm(400.0)));
    assert_eq!(view(&session)["values"]["host.work_width"], 600.0);
    assert!(session.set_work_area(Length::from_mm(20_000.0), Length::from_mm(0.0)));
    assert_eq!(view(&session)["values"]["host.work_width"], 10_000.0);
    assert_eq!(view(&session)["values"]["host.work_height"], 1.0);
}

#[test]
fn ac4_the_laser_panel_shows_its_tab_file_and_history_and_vector_is_unchanged() {
    let mut session = session_with_drawing();
    let vector_tabs = session.panel_view().tabs.len();
    session.add_manufacturing("curvyo.laser").unwrap();
    let names: Vec<&str> = session.panel_view().tabs.iter().map(|t| t.name).collect();
    assert_eq!(names, vec!["workbench", "file", "history"]);
    assert_eq!(session.panel_view().tabs[0].label, "Laser");
    session.set_workbench(Workbench::Vector);
    assert_eq!(session.panel_view().tabs.len(), vector_tabs);
    assert_eq!(
        session.document.pages().len(),
        1,
        "the output page is not a drawing page"
    );
}

#[test]
fn ac6_a_drawing_edit_shows_on_the_output_page_without_a_command() {
    let mut session = session_with_drawing();
    let id = session.add_manufacturing("curvyo.laser").unwrap();
    let before = triangles(&session);
    session.set_workbench(Workbench::Vector);
    let _ = session.document.create_ellipse(EllipseFrame {
        center: Point::new(100.0, 100.0),
        rx: Length::from_mm(10.0),
        ry: Length::from_mm(10.0),
    });
    session.set_workbench(Workbench::Manufacturing(id));
    assert!(triangles(&session) > before, "the new circle is drawn");
}

#[test]
fn ac7_placed_content_is_read_only_and_a_press_or_edit_key_shows_the_notice() {
    let mut session = session_with_drawing();
    session.add_manufacturing("curvyo.laser").unwrap();
    let version = session.document.version();
    session.pointer_down(Point::new(30.0, 30.0), false);
    session.pointer_up(Point::new(30.0, 30.0), false, false);
    assert!(session.selection.is_empty());
    let notice = session.read_only_notice().unwrap();
    assert!(notice.at.is_some());
    assert_eq!(key(&mut session, "Delete", false), KeyOutcome::Consumed);
    assert_eq!(key(&mut session, "a", true), KeyOutcome::Consumed);
    assert!(session.read_only_notice().unwrap().seq > notice.seq);
    assert_eq!(key(&mut session, "r", false), KeyOutcome::Ignored);
    assert_eq!(session.tool, Tool::Select);
    assert_eq!(session.document.version(), version);
    assert_eq!(
        view(&session)["read_only_notice"]["text"],
        super::READ_ONLY_NOTICE
    );
    assert!(super::READ_ONLY_NOTICE.chars().count() <= 70);
}

#[test]
fn ac8_content_past_the_work_area_is_drawn_and_named_in_a_notice() {
    let mut session = session_with_drawing();
    let _ = session.document.create_rect(RectBounds::from_corners(
        Point::new(150.0, 150.0),
        Point::new(209.0, 296.0),
    ));
    session.add_manufacturing("curvyo.laser").unwrap();
    assert_eq!(
        view(&session)["notices"]["overhang"],
        Value::Null,
        "inside 400 x 300"
    );
    assert!(session.set_work_area(Length::from_mm(200.0), Length::from_mm(290.0)));
    let text = view(&session)["notices"]["overhang"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(text.starts_with("Extends past the work area by "), "{text}");
}

#[test]
fn ac9_ac10_an_engraved_grid_is_drawn_below_and_its_eye_hides_it() {
    let mut session = session_with_drawing();
    session.add_manufacturing("curvyo.laser").unwrap();
    let bare = triangles(&session);
    assert!(session.add_fixture("engraved_grid").is_some());
    let with_grid = triangles(&session);
    assert!(with_grid > bare);
    let fixture = view(&session)["fixtures"][0].clone();
    assert_eq!(fixture["name"], "Engraved grid");
    assert_eq!(fixture["values"]["spacing"], 10.0);
    let id = session
        .fixture_named(fixture["id"].as_str().unwrap())
        .unwrap();
    assert!(session.set_fixture_length(id, "spacing", Length::from_mm(0.1)));
    assert_eq!(
        view(&session)["fixtures"][0]["values"]["spacing"],
        0.5,
        "clamped"
    );
    assert!(session.set_fixture_visible(id, false));
    assert_eq!(triangles(&session), bare);
    assert!(session.delete_fixture(id));
    assert_eq!(view(&session)["fixtures"], Value::Array(Vec::new()));
}

#[test]
fn ac11_a_switched_off_extension_keeps_its_manufacturings_disabled() {
    let mut session = session_with_drawing();
    let id = session.add_manufacturing("curvyo.laser").unwrap();
    let text = session.set_extension_enabled("curvyo.laser", false);
    assert!(text.contains("curvyo.laser"));
    assert_eq!(session.workbench(), Workbench::Vector);
    let json = registry(&session);
    assert_eq!(json["add"], Value::Array(Vec::new()));
    assert_eq!(json["entries"][2]["enabled"], false);
    assert_eq!(json["extensions"][0]["name"], "Laser workbench");
    assert_eq!(json["disabled"][0]["label"], "Laser 1");
    session.set_workbench(Workbench::Manufacturing(id));
    assert_eq!(
        session.workbench(),
        Workbench::Vector,
        "a disabled one does not open"
    );
    assert!(session.add_manufacturing("curvyo.laser").is_none());
    session.load_extension_settings("");
    session.set_workbench(Workbench::Manufacturing(id));
    assert_eq!(session.workbench(), Workbench::Manufacturing(id));
}

#[test]
fn ac13_delete_goes_to_vector_and_undo_brings_the_laser_workbench_back() {
    let mut session = session_with_drawing();
    let id = session.add_manufacturing("curvyo.laser").unwrap();
    session.add_fixture("engraved_grid").unwrap();
    let drawing = session.document.read_objects().objects;
    assert!(session.delete_manufacturing(id));
    assert_eq!(session.workbench(), Workbench::Vector);
    assert_eq!(session.document.read_objects().objects, drawing);
    session.undo();
    assert_eq!(session.workbench(), Workbench::Manufacturing(id));
    assert_eq!(
        session.document.manufacturing(id).unwrap().fixtures.len(),
        1
    );
}

#[test]
fn leaving_restores_the_drawing_tab_or_document_when_it_cannot_be_pressed() {
    let mut session = session_with_drawing();
    assert!(session.press_panel_tab("layers"));
    let id = session.add_manufacturing("curvyo.laser").unwrap();
    session.set_workbench(Workbench::Vector);
    assert_eq!(session.panel_tabs.get().active().name(), "layers");
    session.set_workbench(Workbench::Manufacturing(id));
    assert_eq!(session.panel_tabs.get().active().name(), "workbench");
    session.manufacturing.drawing_tab = Some(curvyo_ui_core::PanelTab::Style);
    session.set_workbench(Workbench::Vector);
    assert_eq!(session.panel_tabs.get().active().name(), "document");
}

#[test]
fn each_workbench_keeps_its_own_view() {
    let mut session = session_with_drawing();
    let vector = session.view();
    let id = session.add_manufacturing("curvyo.laser").unwrap();
    let laser = session.view();
    assert_ne!(vector, laser, "the output page is fitted");
    session.set_workbench(Workbench::Vector);
    assert_eq!(session.view(), vector);
    session.set_workbench(Workbench::Manufacturing(id));
    assert_eq!(session.view(), laser);
}
