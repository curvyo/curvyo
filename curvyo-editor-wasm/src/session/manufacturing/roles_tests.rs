//! The session side of `specs/0027-manufacturing-roles`: the Layers block view, the role press as
//! one undo step, the role looks on the output page and the Role view switch.

use curvyo_document_core::{MovePlan, MoveTarget, NodeId, Opacity, Point, RectBounds};
use curvyo_laser_core::Role;
use curvyo_render_core::{ROLE_CUT, ROLE_ENGRAVE, ROLE_IGNORE};
use serde_json::Value;

use super::super::{Session, Workbench};

fn layer(session: &Session, name: &str, x: f64) -> (NodeId, NodeId) {
    let id = session.document.new_layer(None);
    session.document.rename_node(id, name).unwrap();
    session.document.set_draw_parent(Some(id));
    let rect = session.document.create_rect(RectBounds::from_corners(
        Point::new(x, 20.0),
        Point::new(x + 10.0, 30.0),
    ));
    session.document.set_draw_parent(None);
    (id, rect)
}

fn session() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1200.0, 800.0);
    session.show_default_view();
    session
}

fn layers(session: &Session) -> Value {
    let view: Value = serde_json::from_str(&session.manufacturing_view_json()).unwrap();
    view["layers"].clone()
}

fn row<'a>(layers: &'a Value, name: &str) -> &'a Value {
    layers["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap()
}

fn key(id: NodeId) -> String {
    id.to_register_text()
}

fn has(session: &Session, colour: curvyo_render_core::RgbaColor) -> bool {
    session
        .frame_draw_list()
        .triangles
        .iter()
        .any(|v| v.color == colour)
}

fn drawing(session: &Session) -> String {
    format!(
        "{:?} {:?}",
        session.document.read_nodes(),
        session.document.pages()
    )
}

#[test]
fn ac1_one_row_per_layer_front_most_first_and_the_hint_without_layers() {
    let mut s = session();
    let _ = layer(&s, "Back", 10.0);
    let _ = layer(&s, "Front", 30.0);
    s.add_manufacturing("curvyo.laser").unwrap();
    let names: Vec<String> = layers(&s)["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(names, vec!["Front", "Back"]);
    let mut empty = session();
    empty.add_manufacturing("curvyo.laser").unwrap();
    assert_eq!(layers(&empty)["rows"], Value::Array(Vec::new()));
}

#[test]
fn ac3_every_layer_starts_with_no_role_and_add_manufacturing_writes_none() {
    let mut s = session();
    let _ = layer(&s, "A", 10.0);
    let m = s.add_manufacturing("curvyo.laser").unwrap();
    assert!(
        s.document
            .manufacturing(m)
            .unwrap()
            .process_values
            .is_empty()
    );
    assert_eq!(row(&layers(&s), "A")["role"], Value::Null);
    let _ = layer(&s, "Later", 50.0);
    assert_eq!(row(&layers(&s), "Later")["role"], Value::Null);
}

#[test]
fn ac4_a_role_is_one_step_in_one_manufacturing_and_the_drawing_never_changes() {
    let mut s = session();
    let (a, _) = layer(&s, "A", 10.0);
    let _ = layer(&s, "B", 30.0);
    let m1 = s.add_manufacturing("curvyo.laser").unwrap();
    let m2 = s.add_manufacturing("curvyo.laser").unwrap();
    s.set_workbench(Workbench::Manufacturing(m1));
    let before = drawing(&s);
    let steps = s.undo_len();
    assert!(s.set_layer_role(&key(a), Some(Role::Cut)));
    assert_eq!(s.undo_len(), steps + 1);
    let label = s.document.own_steps().last().unwrap().label.clone();
    assert_eq!(
        curvyo_ui_core::known_operation_name(&label),
        Some("Set role")
    );
    assert!(
        !s.set_layer_role(&key(a), Some(Role::Cut)),
        "no second step"
    );
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(row(&layers(&s), "A")["role"], "cut");
    assert_eq!(drawing(&s), before);
    s.set_workbench(Workbench::Manufacturing(m2));
    assert_eq!(row(&layers(&s), "A")["role"], Value::Null);
    s.set_workbench(Workbench::Manufacturing(m1));
    s.undo();
    assert_eq!(row(&layers(&s), "A")["role"], Value::Null);
    assert_eq!(drawing(&s), before);
    s.redo();
    assert_eq!(row(&layers(&s), "A")["role"], "cut");
    assert!(
        s.set_layer_role(&key(a), None),
        "pressing the pressed one clears"
    );
    assert_eq!(row(&layers(&s), "A")["role"], Value::Null);
    assert!(!s.set_layer_role("9:9", Some(Role::Cut)), "no such row");
}

#[test]
fn ac6_the_output_page_draws_the_role_looks_and_role_view_off_the_drawing() {
    let mut s = session();
    let (a, _) = layer(&s, "A", 10.0);
    let (b, _) = layer(&s, "B", 30.0);
    let _ = layer(&s, "C", 50.0);
    s.add_manufacturing("curvyo.laser").unwrap();
    assert!(has(&s, ROLE_IGNORE), "no role draws the muted look");
    assert!(!has(&s, ROLE_CUT));
    s.set_layer_role(&key(a), Some(Role::Cut));
    s.set_layer_role(&key(b), Some(Role::Engrave));
    assert!(has(&s, ROLE_CUT) && has(&s, ROLE_ENGRAVE) && has(&s, ROLE_IGNORE));
    let steps = s.undo_len();
    s.set_role_view(false);
    assert!(!s.role_view());
    assert!(!has(&s, ROLE_CUT) && !has(&s, ROLE_ENGRAVE) && !has(&s, ROLE_IGNORE));
    assert_eq!(layers(&s)["role_view"], false);
    assert_eq!(s.undo_len(), steps, "not an undo step");
    s.set_role_view(true);
    assert!(has(&s, ROLE_CUT));
}

#[test]
fn ac9_a_leaf_moved_to_another_layer_takes_its_role_at_once() {
    let mut s = session();
    let (a, rect) = layer(&s, "A", 10.0);
    let (b, _) = layer(&s, "B", 30.0);
    s.add_manufacturing("curvyo.laser").unwrap();
    s.set_layer_role(&key(a), Some(Role::Cut));
    s.set_layer_role(&key(b), Some(Role::Engrave));
    let cut_before = s
        .frame_draw_list()
        .triangles
        .iter()
        .filter(|v| v.color == ROLE_CUT)
        .count();
    s.document
        .move_nodes(&MovePlan {
            nodes: vec![rect],
            target: MoveTarget::FrontOf(Some(b)),
        })
        .unwrap();
    assert!(!has(&s, ROLE_CUT), "A is empty now; was {cut_before}");
    assert!(has(&s, ROLE_ENGRAVE));
}

#[test]
fn ac11_loose_objects_have_their_own_row_under_the_reserved_key() {
    let mut s = session();
    let _ = layer(&s, "A", 10.0);
    let _ = s.document.create_rect(RectBounds::from_corners(
        Point::new(70.0, 20.0),
        Point::new(80.0, 30.0),
    ));
    s.add_manufacturing("curvyo.laser").unwrap();
    let rows = layers(&s)["rows"].as_array().unwrap().clone();
    assert_eq!(rows.last().unwrap()["key"], "outside");
    assert_eq!(rows.last().unwrap()["name"], "Objects outside layers");
    assert!(s.set_layer_role("outside", Some(Role::Cut)));
    assert!(has(&s, ROLE_CUT));
}

#[test]
fn ac13_a_transparent_leaf_is_drawn_in_its_look_at_full_strength() {
    let mut s = session();
    let (a, rect) = layer(&s, "A", 10.0);
    s.document
        .set_node_opacity(&[rect], Opacity::new(0.0).unwrap())
        .unwrap();
    s.add_manufacturing("curvyo.laser").unwrap();
    s.set_layer_role(&key(a), Some(Role::Engrave));
    assert!(has(&s, ROLE_ENGRAVE), "the opaque role colour");
}

#[test]
fn ac14_a_hidden_layer_is_not_drawn_and_comes_back_with_its_role() {
    let mut s = session();
    let (a, _) = layer(&s, "A", 10.0);
    s.add_manufacturing("curvyo.laser").unwrap();
    s.set_layer_role(&key(a), Some(Role::Cut));
    s.document.set_node_visible(&[a], false).unwrap();
    assert!(!has(&s, ROLE_CUT));
    assert_eq!(row(&layers(&s), "A")["hidden"], true);
    assert_eq!(row(&layers(&s), "A")["role"], "cut");
    s.document.set_node_visible(&[a], true).unwrap();
    assert!(has(&s, ROLE_CUT));
}

#[test]
fn ac15_the_notice_counts_layers_without_role_that_hold_output_capable_objects() {
    let mut s = session();
    let (a, _) = layer(&s, "A", 10.0);
    let _ = layer(&s, "B", 30.0);
    let _empty = s.document.new_layer(None);
    let (hidden, _) = layer(&s, "Hidden", 50.0);
    s.document.set_node_visible(&[hidden], false).unwrap();
    s.add_manufacturing("curvyo.laser").unwrap();
    assert_eq!(
        layers(&s)["notices"],
        serde_json::json!(["2 layers have no role and are not output."])
    );
    s.set_layer_role(&key(a), Some(Role::Ignore));
    assert_eq!(
        layers(&s)["notices"],
        serde_json::json!(["1 layer has no role and is not output."])
    );
}

#[test]
fn ac16_ac17_a_deleted_layer_leaves_the_rows_and_undo_brings_it_back_with_its_role() {
    let mut s = session();
    let (a, _) = layer(&s, "A", 10.0);
    let m = s.add_manufacturing("curvyo.laser").unwrap();
    s.set_layer_role(&key(a), Some(Role::Cut));
    s.set_workbench(Workbench::Vector);
    s.recording(|this| this.document.delete_layer(a).unwrap());
    s.set_workbench(Workbench::Manufacturing(m));
    assert_eq!(layers(&s)["rows"], Value::Array(Vec::new()));
    assert_eq!(
        s.document.manufacturing(m).unwrap().process_values.len(),
        1,
        "the role stays in the file"
    );
    s.undo();
    assert_eq!(row(&layers(&s), "A")["role"], "cut");
}

#[test]
fn ac18_an_unknown_value_reads_as_no_role_and_a_press_replaces_it() {
    let mut s = session();
    let (a, _) = layer(&s, "A", 10.0);
    let m = s.add_manufacturing("curvyo.laser").unwrap();
    let stored = format!("x.curvyo.laser.role.{}", key(a));
    s.document
        .set_process_register(m, &stored, Some(&"score".into()), "set_role")
        .unwrap();
    let a_row = row(&layers(&s), "A").clone();
    assert_eq!(a_row["role"], Value::Null);
    assert_eq!(a_row["unknown"], true);
    assert!(has(&s, ROLE_IGNORE));
    assert_eq!(
        layers(&s)["notices"],
        serde_json::json!(["1 layer has no role and is not output."])
    );
    assert!(s.set_layer_role(&key(a), Some(Role::Engrave)));
    assert_eq!(row(&layers(&s), "A")["unknown"], false);
}
