//! The Machine block in the session (`specs/0026-machine-profile` criteria 1 to 13): the view, a
//! pick, field edits, Reset, Save as profile, Update, the library edits and their host writes,
//! "Differs from profile" derived against a fixture record, "Not in your profiles", the damaged
//! name, the last pick for Add manufacturing and the origin marker.

#![allow(clippy::unwrap_used, clippy::float_cmp)]

mod page_kind_support;
use curvyo_document_core::{
    Document, DocumentSize, NewOutputPage, Point, ProfileChange, RectBounds, pack, unpack,
};
use curvyo_editor_wasm::Session;
use curvyo_laser_core::{
    MachineProfile, OriginCorner, new_profile_id, record_file_name, write_record,
};
use page_kind_support::PageKinds as _;
use serde_json::Value;

const LASER: &str = "curvyo.laser";

fn session() -> Session {
    let document = Document::new(1);
    let _ = document.create_rect(RectBounds::from_corners(
        Point::new(20.0, 20.0),
        Point::new(60.0, 50.0),
    ));
    let mut session = Session::open(2, &pack(&document, "test").unwrap()).unwrap();
    session.resize_viewport(1200.0, 800.0);
    session
}

fn opened(session: &Session) -> Document {
    unpack(3, &session.pack("test").unwrap()).unwrap()
}

fn k40() -> MachineProfile {
    let mut profile = MachineProfile::generic_laser();
    profile.id = new_profile_id([7; 16]);
    "K40 in the garage".clone_into(&mut profile.values.name);
    profile.values.origin = OriginCorner::TopRight;
    profile.work_area = DocumentSize::from_mm(600.0, 400.0);
    "/dev/ttyUSB0".clone_into(&mut profile.connection.controller_port);
    profile
}

fn library(profiles: &[MachineProfile]) -> String {
    let entries: Vec<Value> = profiles
        .iter()
        .map(|p| serde_json::json!({"name": record_file_name(&p.id), "text": write_record(p)}))
        .collect();
    Value::from(entries).to_string()
}

fn machine(session: &Session) -> Value {
    let view: Value = serde_json::from_str(&session.manufacturing_view_json()).unwrap();
    view["machine"].clone()
}

fn writes(session: &mut Session) -> Vec<Value> {
    serde_json::from_str::<Vec<Value>>(&session.take_machine_writes_json()).unwrap()
}

fn with_k40() -> Session {
    let mut s = session();
    s.load_machine_profiles_json(&library(&[k40()]));
    s.add_page_of_kind(LASER, None).unwrap();
    s
}

#[test]
fn ac1_a_new_laser_shows_generic_laser_with_its_defaults() {
    let mut s = session();
    s.add_page_of_kind(LASER, None).unwrap();
    let m = machine(&s);
    assert_eq!(m["copy"]["name"], "Generic laser");
    assert_eq!(m["copy"]["width"], 400.0);
    assert_eq!(m["copy"]["height"], 300.0);
    assert_eq!(m["copy"]["origin"], "bottom_left");
    assert_eq!(m["copy"]["max_feed"], 6000);
    assert_eq!(m["copy"]["max_power"], 1000);
    assert_eq!(m["copy"]["laser_mode"], true);
    assert_eq!(m["state"], Value::Null);
    assert_eq!(m["builtin"], true);
    assert_eq!(
        m["connection"],
        Value::Null,
        "the built-in has no connection"
    );
    assert_eq!(m["profiles"].as_array().unwrap().len(), 1);
}

#[test]
fn ac1_edits_are_limited_with_a_notice_and_one_step_each() {
    let mut s = session();
    s.add_page_of_kind(LASER, None).unwrap();
    let steps = s.history_len();
    assert_eq!(
        s.set_machine_max_feed(250_000.0).as_deref(),
        Some("Max feed limited to 100,000 mm/min.")
    );
    assert_eq!(s.history_len(), steps + 1);
    assert_eq!(
        s.set_machine_max_power(12.4).as_deref(),
        Some("Max power rounded to 12.")
    );
    assert_eq!(s.set_machine_name("  K40  "), None);
    assert!(s.set_machine_origin("top_left"));
    s.set_machine_laser_mode(false);
    assert_eq!(s.history_len(), steps + 5);
    let m = machine(&s);
    assert_eq!(m["copy"]["max_feed"], 100_000);
    assert_eq!(m["copy"]["max_power"], 12);
    assert_eq!(m["copy"]["name"], "K40");
    assert_eq!(m["state"], "differs", "an edited Generic laser differs");
    assert_eq!(m["can_update"], false, "the built-in cannot be updated");
    s.undo();
    s.undo();
    assert_eq!(machine(&s)["copy"]["laser_mode"], true);
    assert!(
        writes(&mut s).is_empty(),
        "copy edits never write the library"
    );
}

#[test]
fn ac5_a_pick_takes_values_and_work_area_in_one_step_and_is_remembered() {
    let mut s = with_k40();
    let steps = s.history_len();
    assert!(s.pick_machine_profile(&k40().id));
    assert_eq!(s.history_len(), steps + 1);
    let m = machine(&s);
    assert_eq!(m["copy"]["name"], "K40 in the garage");
    assert_eq!(m["copy"]["width"], 600.0);
    assert_eq!(m["state"], Value::Null);
    assert_eq!(m["connection"]["controller_port"], "/dev/ttyUSB0");
    assert!(
        !s.pick_machine_profile(&k40().id),
        "the same pick adds no step"
    );
    assert_eq!(s.history_len(), steps + 1);
    let w = writes(&mut s);
    assert_eq!(w[0]["kind"], "choice");
    assert!(w[0]["text"].as_str().unwrap().contains(&k40().id));
    s.undo();
    assert_eq!(machine(&s)["copy"]["name"], "Generic laser");
    assert_eq!(machine(&s)["copy"]["width"], 400.0);
}

#[test]
fn ac6_the_copy_in_the_document_carries_no_connection() {
    let mut s = with_k40();
    s.pick_machine_profile(&k40().id);
    let json = String::from_utf8(opened(&s).export_json().unwrap()).unwrap();
    assert!(json.contains("K40 in the garage"));
    assert!(
        !json.contains("ttyUSB0"),
        "the connection never enters the project"
    );
}

#[test]
fn ac7_differs_update_and_reset() {
    let mut s = with_k40();
    s.pick_machine_profile(&k40().id);
    s.set_machine_max_feed(9_000.0);
    let m = machine(&s);
    assert_eq!(m["state"], "differs");
    assert_eq!(m["can_update"], true);
    let steps = s.history_len();
    assert!(s.reset_machine_profile());
    assert_eq!(s.history_len(), steps + 1, "Reset is one undo step");
    assert_eq!(machine(&s)["copy"]["max_feed"], 6000);
    assert_eq!(machine(&s)["state"], Value::Null);
    s.undo();
    assert_eq!(machine(&s)["state"], "differs");
    let _ = writes(&mut s);
    let steps = s.history_len();
    assert_eq!(s.update_machine_profile(5), None);
    assert_eq!(s.history_len(), steps, "Update is no history step");
    let w = writes(&mut s);
    assert_eq!(w[0]["kind"], "write");
    assert!(
        w[0]["text"]
            .as_str()
            .unwrap()
            .contains("\"max_feed_mm_per_min\": 9000")
    );
    assert!(
        w[0]["text"].as_str().unwrap().contains("ttyUSB0"),
        "connection kept"
    );
    assert_eq!(machine(&s)["state"], Value::Null);
    // Typing the original value back clears the state.
    s.set_machine_max_feed(1_000.0);
    assert_eq!(machine(&s)["state"], "differs");
    s.set_machine_max_feed(9_000.0);
    assert_eq!(machine(&s)["state"], Value::Null);
}

#[test]
fn ac7_a_renamed_library_profile_makes_its_copies_differ() {
    let mut s = with_k40();
    s.pick_machine_profile(&k40().id);
    assert!(s.rename_machine_profile(&k40().id, "Garage", 3).is_none());
    assert_eq!(machine(&s)["state"], "differs");
    s.reset_machine_profile();
    assert_eq!(machine(&s)["copy"]["name"], "Garage");
}

#[test]
fn ac3_save_as_profile_adds_a_record_points_at_it_and_undo_keeps_the_record() {
    let mut s = session();
    s.add_page_of_kind(LASER, None).unwrap();
    let steps = s.history_len();
    assert_eq!(s.save_machine_profile([1; 16], 10), None);
    assert_eq!(s.history_len(), steps + 1);
    let m = machine(&s);
    assert_eq!(
        m["copy"]["name"], "Generic laser copy",
        "the built-in's name is taken"
    );
    assert_eq!(m["state"], Value::Null);
    assert_eq!(m["builtin"], false);
    assert_eq!(m["connection"]["baud_rate"], 115_200);
    let w = writes(&mut s);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0]["id"], new_profile_id([1; 16]));
    s.undo();
    let m = machine(&s);
    assert_eq!(m["copy"]["id"], "generic-laser");
    assert_eq!(
        m["profiles"].as_array().unwrap().len(),
        2,
        "the record stays"
    );
    s.redo();
    s.save_machine_profile([2; 16], 11);
    assert_eq!(machine(&s)["copy"]["name"], "Generic laser copy copy");
}

#[test]
fn ac2_ac3_library_rows_duplicate_rename_and_delete() {
    let mut s = with_k40();
    let copy = s
        .duplicate_machine_profile("generic-laser", [3; 16], 1)
        .unwrap();
    let names: Vec<String> = machine(&s)["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        names,
        vec!["Generic laser", "Generic laser copy", "K40 in the garage"]
    );
    assert_eq!(
        s.rename_machine_profile(&copy, &"z".repeat(70), 2)
            .as_deref(),
        Some("Name limited to 60 characters.")
    );
    assert!(
        s.rename_machine_profile("generic-laser", "Mine", 2)
            .is_none()
    );
    s.pick_machine_profile(&k40().id);
    let steps = s.history_len();
    assert!(s.delete_machine_profile(&k40().id));
    assert!(!s.delete_machine_profile("generic-laser"));
    assert_eq!(s.history_len(), steps, "library edits are no step");
    let m = machine(&s);
    assert_eq!(m["state"], "missing");
    assert_eq!(m["copy"]["name"], "K40 in the garage", "the copy stays");
    assert_eq!(m["connection"], Value::Null);
    let kinds: Vec<String> = writes(&mut s)
        .iter()
        .map(|w| w["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(kinds, vec!["write", "write", "choice", "remove"]);
}

#[test]
fn ac8_a_foreign_profile_opens_unchanged_and_save_as_keeps_its_id() {
    let document = Document::new(4);
    let foreign = k40();
    let copy = curvyo_laser_core::write_copy(&curvyo_laser_core::ProfileCopy::of(&foreign));
    let id = document
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: Some(&copy),
            work_area: DocumentSize::from_mm(600.0, 400.0),
        })
        .unwrap()
        .kind
        .manufacturing()
        .unwrap();
    let bytes = pack(&document, "test").unwrap();
    let mut s = Session::open(5, &bytes).unwrap();
    let steps = s.undo_len();
    s.to_output_page(id);
    let m = machine(&s);
    assert_eq!(m["state"], "missing");
    assert_eq!(m["copy"]["name"], "K40 in the garage");
    assert_eq!(s.undo_len(), steps, "opening changes nothing");
    assert_eq!(opened(&s).manufacturings(), document.manufacturings());
    s.save_machine_profile([9; 16], 1);
    let m = machine(&s);
    assert_eq!(m["copy"]["id"], foreign.id, "saved under the same id");
    assert_eq!(m["state"], Value::Null);
}

#[test]
fn ac10_add_manufacturing_starts_with_the_last_pick_else_generic() {
    let mut s = with_k40();
    s.pick_machine_profile(&k40().id);
    let text = writes(&mut s)[0]["text"].as_str().unwrap().to_owned();
    let mut next = session();
    next.load_machine_profiles_json(&library(&[k40()]));
    next.load_machine_choice(&text);
    let steps = next.history_len();
    next.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(next.history_len(), steps + 1);
    assert_eq!(machine(&next)["copy"]["name"], "K40 in the garage");
    assert_eq!(machine(&next)["copy"]["width"], 600.0);
    let mut gone = session();
    gone.load_machine_choice(&text);
    gone.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(machine(&gone)["copy"]["name"], "Generic laser");
}

#[test]
fn ac9_connection_edits_write_the_record_only_for_a_user_profile() {
    let mut s = with_k40();
    assert_eq!(
        s.set_machine_connection("10.0.0.2", "COM3", 9_600.0, 1),
        None
    );
    assert!(
        writes(&mut s).iter().all(|w| w["kind"] != "write"),
        "built-in: nothing"
    );
    s.pick_machine_profile(&k40().id);
    let steps = s.history_len();
    assert_eq!(
        s.set_machine_connection("10.0.0.2", "COM3", 5e6, 2)
            .as_deref(),
        Some("Baud rate limited to 4,000,000.")
    );
    assert_eq!(s.history_len(), steps);
    assert_eq!(machine(&s)["connection"]["controller_port"], "COM3");
    assert_eq!(machine(&s)["connection"]["baud_rate"], 4_000_000);
}

#[test]
fn ac12_differs_is_derived_from_a_fixture_record() {
    let record = k40();
    let document = Document::new(4);
    let mut edited = curvyo_laser_core::ProfileCopy::of(&record);
    edited.values.max_power = curvyo_laser_core::PowerValue::new(500).unwrap();
    let id = document
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: Some(&curvyo_laser_core::write_copy(&edited)),
            work_area: DocumentSize::from_mm(600.0, 400.0),
        })
        .unwrap()
        .kind
        .manufacturing()
        .unwrap();
    let bytes = pack(&document, "test").unwrap();
    let mut s = Session::open(5, &bytes).unwrap();
    s.load_machine_profiles_json(&library(std::slice::from_ref(&record)));
    s.to_output_page(id);
    assert_eq!(machine(&s)["state"], "differs");
    document
        .set_manufacturing_profile(
            id,
            &curvyo_laser_core::write_copy(&curvyo_laser_core::ProfileCopy::of(&record)),
            DocumentSize::from_mm(600.0, 400.0),
            curvyo_document_core::PageCorner::default(),
            ProfileChange::Reset,
        )
        .unwrap();
    let mut same = Session::open(6, &pack(&document, "test").unwrap()).unwrap();
    same.load_machine_profiles_json(&library(&[record]));
    same.to_output_page(id);
    assert_eq!(machine(&same)["state"], Value::Null);
}

#[test]
fn ac12_a_damaged_name_reads_generic_laser_with_its_notice() {
    let document = Document::new(4);
    let damaged: curvyo_document_core::FixtureParams =
        serde_json::from_value(serde_json::json!({"id": "generic-laser", "name": ""})).unwrap();
    let id = document
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: Some(&damaged),
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap()
        .kind
        .manufacturing()
        .unwrap();
    let mut s = Session::open(5, &pack(&document, "test").unwrap()).unwrap();
    s.to_output_page(id);
    let m = machine(&s);
    assert_eq!(m["copy"]["name"], "Generic laser");
    assert_eq!(m["name_notice"], "Name was damaged, showing Generic laser");
}

#[test]
fn ac13_damaged_records_are_counted_in_one_notice() {
    let mut s = session();
    let entries = serde_json::json!([
        {"name": record_file_name(&k40().id), "text": "{"},
        {"name": "readme.txt", "text": "{"},
    ]);
    s.load_machine_profiles_json(&entries.to_string());
    s.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(
        machine(&s)["library_notices"][0],
        "1 profile could not be read. It was left as it is."
    );
}

#[test]
fn ac11_the_origin_marker_moves_with_the_corner_and_changes_no_content() {
    let mut s = session();
    s.add_page_of_kind(LASER, None).unwrap();
    let origin = s.active_page().origin.x;
    let marker_x = |s: &Session| {
        // The marker is in the marker ink, page-local x.
        s.frame_draw_list()
            .triangles
            .iter()
            .filter(|v| v.color.r == 0x3A && v.color.g == 0x3A && v.color.b == 0x3F)
            .map(|v| v.position.x - origin)
            .fold(f64::NAN, f64::max)
    };
    let left = marker_x(&s);
    let objects = opened(&s).read_objects().objects;
    s.set_machine_origin("bottom_right");
    let right = marker_x(&s);
    assert!(left < 50.0 && right > 350.0, "{left} -> {right}");
    assert_eq!(opened(&s).read_objects().objects, objects);
}
