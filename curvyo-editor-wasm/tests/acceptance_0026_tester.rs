//! Updated for `specs/0098-page-kinds` by its "Changes to accepted specs" table: a laser page is
//! added with Add page (Kind Laser, `0026` 10 → `0098` 1, 4) and reached by activating it, and the
//! work area is no field on the page (`0026` 1, 7 → `0098` 13, 15), so the work-area edits and the
//! oversize-page case are gone.
//!
//! Independent acceptance tests for `specs/0026-machine-profile` through the public session: the
//! Machine block view (criterion 1), the built-in and Duplicate (2), Save as profile with its undo
//! boundary and the list edits (3), library edits outside history and the host writes that keep
//! them across a restart (4), Pick (5), the copy in the file without connection (6), Differs,
//! Update and Reset (7), Not in your profiles (8), the connection fields (9), Add manufacturing
//! with the last pick (10), the origin marker (11), Differs on reopening and damaged or format-35
//! copies (12), damaged records (13), the budgets (14) and what leaves the session (15).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assigning_clones,
    clippy::assert_is_empty,
    clippy::many_single_char_names
)]

mod cpu_time;

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    Color, Document, DocumentSize, Point, RectBounds, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::Session;
use curvyo_laser_core::{
    BaudRate, FeedRate, MachineProfile, OriginCorner, PowerValue, new_profile_id, read_copy,
    read_record, record_file_name, write_record,
};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";
const ADDRESS: &str = "192.168.77.20:8080";
const PORT: &str = "/dev/ttyUSB7";

// ---- helpers ------------------------------------------------------------------------------------

fn id(n: u8) -> String {
    new_profile_id([n; 16])
}

fn k40(n: u8) -> MachineProfile {
    let mut p = MachineProfile::generic_laser();
    p.id = id(n);
    p.modified = 1_000;
    p.values.name = "K40".to_owned();
    p.values.origin = OriginCorner::TopRight;
    p.values.max_feed = FeedRate::new(12_000).unwrap();
    p.values.max_power = PowerValue::new(255).unwrap();
    p.values.laser_mode = false;
    p.work_area = DocumentSize::from_mm(600.0, 400.0);
    p.connection.connector_address = ADDRESS.to_owned();
    p.connection.controller_port = PORT.to_owned();
    p.connection.baud_rate = BaudRate::new(250_000).unwrap();
    p
}

/// A fake per-user data directory: file name to text.
type Disk = BTreeMap<String, String>;

fn disk_of(profiles: &[MachineProfile]) -> Disk {
    profiles
        .iter()
        .map(|p| (record_file_name(&p.id), write_record(p)))
        .collect()
}

fn library_json(disk: &Disk) -> String {
    Value::from(
        disk.iter()
            .map(|(name, text)| json!({"name": name, "text": text}))
            .collect::<Vec<_>>(),
    )
    .to_string()
}

fn drawing() -> Vec<u8> {
    let d = Document::new(1);
    let r = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 20.0),
        Point::new(50.0, 40.0),
    ));
    d.edit_style(&[r], &StyleEdit::FillEnabled(true)).unwrap();
    d.edit_style(&[r], &StyleEdit::FillColor(Color { r: 255, g: 0, b: 0 }))
        .unwrap();
    d.edit_style(&[r], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    pack(&d, "0.1.0").unwrap()
}

/// A session over a one-rectangle drawing with `disk` as the library.
fn session(disk: &Disk) -> Session {
    let mut s = Session::open(9300, &drawing()).unwrap();
    s.load_machine_profiles_json(&library_json(disk));
    s
}

fn with_laser(disk: &Disk) -> Session {
    let mut s = session(disk);
    s.add_page_of_kind(LASER, None).expect("Laser can be added");
    let _ = s.take_machine_writes_json();
    s
}

fn machine(s: &Session) -> Value {
    let v: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    v["machine"].clone()
}

fn copy(s: &Session) -> Value {
    machine(s)["copy"].clone()
}

fn writes(s: &mut Session) -> Vec<Value> {
    serde_json::from_str::<Value>(&s.take_machine_writes_json())
        .unwrap()
        .as_array()
        .unwrap()
        .clone()
}

/// Applies the host writes to `disk` as the desktop host does.
fn apply(disk: &mut Disk, ws: &[Value]) {
    for w in ws {
        match w["kind"].as_str().unwrap() {
            "write" => {
                disk.insert(
                    record_file_name(w["id"].as_str().unwrap()),
                    w["text"].as_str().unwrap().to_owned(),
                );
            }
            "remove" => {
                disk.remove(&record_file_name(w["id"].as_str().unwrap()));
            }
            "choice" => {}
            other => panic!("unknown write kind {other}"),
        }
    }
}

fn library_writes(ws: &[Value]) -> Vec<&Value> {
    ws.iter().filter(|w| w["kind"] != "choice").collect()
}

fn opened(s: &Session) -> Document {
    unpack(9399, &s.pack("0.1.0").unwrap()).unwrap()
}

fn member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn contains(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

fn work_area(s: &Session) -> (f64, f64) {
    let c = copy(s);
    (c["width"].as_f64().unwrap(), c["height"].as_f64().unwrap())
}

fn profile_ids(s: &Session) -> Vec<String> {
    machine(s)["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_owned())
        .collect()
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-document-core/tests/fixtures")
            .join(name),
    )
    .unwrap()
}

fn red_box(s: &Session) -> (f64, f64, f64, f64) {
    marker_like_box(s, |c| c.r > 200 && c.g < 60 && c.b < 60 && c.a > 0).unwrap()
}

fn marker_box(s: &Session) -> Option<(f64, f64, f64, f64)> {
    marker_like_box(s, |c| c.r == 0x3A && c.g == 0x3A && c.b == 0x3F && c.a > 0)
}

fn marker_like_box(
    s: &Session,
    pick: impl Fn(&curvyo_render_core::RgbaColor) -> bool,
) -> Option<(f64, f64, f64, f64)> {
    s.frame_draw_list()
        .triangles
        .iter()
        .filter(|v| pick(&v.color))
        .fold(None, |acc, v| {
            let (x, y) = (v.position.x, v.position.y);
            Some(match acc {
                None => (x, y, x, y),
                Some((a, b, c, d)) => (a.min(x), b.min(y), c.max(x), d.max(y)),
            })
        })
}

// ---- criterion 1 --------------------------------------------------------------------------------

#[test]
fn ac1_a_new_laser_manufacturing_shows_the_machine_block_with_the_defaults() {
    let s = with_laser(&Disk::new());
    let m = machine(&s);
    assert!(m.is_object(), "the view has a Machine block: {m}");
    let c = &m["copy"];
    assert_eq!(c["id"], "generic-laser");
    assert_eq!(c["name"], "Generic laser");
    assert_eq!(c["origin"], "bottom_left");
    assert_eq!(c["max_feed"], 6_000);
    assert_eq!(c["max_power"], 1_000);
    assert_eq!(c["laser_mode"], true);
    assert_eq!(work_area(&s), (400.0, 300.0));
    assert_eq!(m["builtin"], true);
    assert!(m["state"].is_null(), "the built-in copy is in sync");
    assert!(m["connection"].is_null(), "no connection for the built-in");
}

#[test]
fn ac1_typed_values_out_of_range_are_limited_with_a_notice_and_one_step_each() {
    let mut s = with_laser(&Disk::new());
    let steps = s.undo_len();
    let n = s.set_machine_max_feed(250_000.0).expect("a notice");
    assert_eq!(n, "Max feed limited to 100,000 mm/min.");
    assert_eq!(copy(&s)["max_feed"], 100_000);
    assert_eq!(s.undo_len(), steps + 1);
    let n = s.set_machine_max_power(12.6).expect("a notice");
    assert!(n.starts_with("Max power rounded to"), "{n}");
    assert_eq!(copy(&s)["max_power"], 13);
    assert_eq!(s.undo_len(), steps + 2);
    assert_eq!(s.set_machine_max_feed(5_000.0), None, "in range: no notice");
    assert_eq!(copy(&s)["max_feed"], 5_000);
    let long = "n".repeat(80);
    assert_eq!(
        s.set_machine_name(&long).as_deref(),
        Some("Name limited to 60 characters.")
    );
    assert_eq!(copy(&s)["name"].as_str().unwrap().chars().count(), 60);
}

#[test]
fn ac1_empty_or_non_finite_input_changes_nothing_and_adds_no_step() {
    let mut s = with_laser(&Disk::new());
    let before = copy(&s);
    let steps = s.undo_len();
    assert_eq!(s.set_machine_max_feed(f64::NAN), None);
    assert_eq!(s.set_machine_max_power(f64::INFINITY), None);
    assert_eq!(s.set_machine_name(""), None);
    assert_eq!(s.set_machine_name("   "), None);
    assert!(!s.set_machine_origin("center"));
    assert_eq!(copy(&s), before);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac1_the_same_value_again_adds_no_step() {
    let mut s = with_laser(&Disk::new());
    let steps = s.undo_len();
    let _ = s.set_machine_max_feed(6_000.0);
    s.set_machine_laser_mode(true);
    assert!(!s.set_machine_origin("bottom_left"));
    let _ = s.set_machine_name("Generic laser");
    assert_eq!(s.undo_len(), steps, "unchanged values write nothing");
}

// ---- criterion 2 --------------------------------------------------------------------------------

#[test]
fn ac2_generic_laser_is_listed_first_and_duplicate_makes_an_editable_copy_outside_history() {
    let mut s = with_laser(&Disk::new());
    let rows = machine(&s)["profiles"].clone();
    assert_eq!(rows.as_array().unwrap().len(), 1);
    assert_eq!(rows[0]["id"], "generic-laser");
    assert_eq!(rows[0]["builtin"], true);
    assert_eq!(rows[0]["chosen"], true);
    let steps = s.undo_len();
    let new = s
        .duplicate_machine_profile("generic-laser", [9; 16], 5)
        .expect("Duplicate gives an id");
    assert_eq!(s.undo_len(), steps, "a library change is no history step");
    let ws = writes(&mut s);
    let lib = library_writes(&ws);
    assert_eq!(lib.len(), 1);
    assert_eq!(lib[0]["kind"], "write");
    assert_eq!(lib[0]["id"], json!(new));
    let rec = read_record(&record_file_name(&new), lib[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(rec.values.name, "Generic laser copy");
    assert_eq!(copy(&s)["id"], "generic-laser", "Duplicate does not pick");
    // Second duplicate is numbered.
    let second = s
        .duplicate_machine_profile("generic-laser", [10; 16], 6)
        .unwrap();
    let rows = machine(&s)["profiles"].clone();
    let name_of = |i: &str| {
        rows.as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == json!(i))
            .unwrap()["name"]
            .clone()
    };
    assert_eq!(name_of(&second), "Generic laser copy 2");
}

#[test]
fn ac2_the_built_in_cannot_be_renamed_deleted_or_updated_through_the_session() {
    let mut s = with_laser(&Disk::new());
    let _ = s.rename_machine_profile("generic-laser", "Mine", 1);
    assert!(!s.delete_machine_profile("generic-laser"));
    let _ = s.set_machine_max_feed(9_000.0);
    assert_eq!(
        machine(&s)["state"],
        "differs",
        "an edited built-in copy differs"
    );
    assert_eq!(
        machine(&s)["can_update"],
        false,
        "no Update for the built-in"
    );
    let _ = s.update_machine_profile(2);
    let ws = writes(&mut s);
    assert!(
        library_writes(&ws).is_empty(),
        "nothing written for the built-in: {ws:?}"
    );
    assert_eq!(machine(&s)["profiles"][0]["name"], "Generic laser");
}

// ---- criterion 3 --------------------------------------------------------------------------------

#[test]
fn ac3_save_as_profile_adds_a_record_and_points_the_copy_at_it_in_one_step() {
    let mut s = with_laser(&Disk::new());
    let _ = s.set_machine_max_feed(7_777.0);
    let steps = s.undo_len();
    let _ = s.save_machine_profile([3; 16], 50);
    assert_eq!(s.undo_len(), steps + 1, "Save as profile is one step");
    let c = copy(&s);
    let new = c["id"].as_str().unwrap().to_owned();
    assert_ne!(new, "generic-laser");
    assert_eq!(
        c["name"], "Generic laser copy",
        "the name was taken by the built-in"
    );
    let ws = writes(&mut s);
    let lib = library_writes(&ws);
    assert_eq!(lib.len(), 1);
    let rec = read_record(&record_file_name(&new), lib[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(rec.values.max_feed.get(), 7_777);
    assert_eq!(rec.work_area, DocumentSize::from_mm(400.0, 300.0));
    assert_eq!(rec.values.name, "Generic laser copy");
    assert_eq!(
        rec.connection.baud_rate.get(),
        115_200,
        "connection empty, default baud"
    );
    assert_eq!(rec.connection.connector_address, "");
    let m = machine(&s);
    assert!(m["state"].is_null(), "in sync after saving: {m}");
    assert!(m["connection"].is_object(), "connection now editable");
}

#[test]
fn ac3_undo_after_save_as_restores_the_copy_but_the_profile_stays_in_the_library() {
    let mut s = with_laser(&Disk::new());
    let _ = s.set_machine_name("Diode");
    let before = copy(&s);
    let _ = s.save_machine_profile([4; 16], 50);
    let new = copy(&s)["id"].as_str().unwrap().to_owned();
    assert_eq!(copy(&s)["name"], "Diode", "a free name is kept");
    let _ = writes(&mut s);
    let _ = s.undo();
    assert_eq!(
        copy(&s),
        before,
        "the copy goes back to its previous id and values"
    );
    assert!(
        profile_ids(&s).contains(&new),
        "the library keeps the saved profile"
    );
    let ws = writes(&mut s);
    assert!(
        library_writes(&ws).is_empty(),
        "undo does not touch the library: {ws:?}"
    );
    let _ = s.redo();
    assert_eq!(copy(&s)["id"], json!(new));
}

#[test]
fn ac3_save_as_with_a_taken_name_saves_name_copy_numbered() {
    let mut disk = disk_of(&[k40(1)]);
    let mut second = k40(2);
    second.values.name = "K40 copy".to_owned();
    disk.extend(disk_of(&[second]));
    let mut s = with_laser(&disk);
    let _ = s.set_machine_name("K40");
    let _ = s.save_machine_profile([5; 16], 1);
    assert_eq!(copy(&s)["name"], "K40 copy 2");
}

#[test]
fn ac3_rename_and_delete_are_library_changes_and_delete_leaves_the_copy() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let _ = writes(&mut s);
    let steps = s.undo_len();
    let _ = s.rename_machine_profile(&id(1), "K40 garage", 2);
    assert_eq!(s.undo_len(), steps);
    assert_eq!(copy(&s)["name"], "K40", "the copy keeps its name");
    assert_eq!(machine(&s)["state"], "differs", "a renamed record differs");
    let ws = writes(&mut s);
    assert_eq!(library_writes(&ws).len(), 1);
    assert!(s.delete_machine_profile(&id(1)));
    assert_eq!(s.undo_len(), steps);
    let ws = writes(&mut s);
    assert_eq!(ws, vec![json!({"kind": "remove", "id": id(1)})]);
    let m = machine(&s);
    assert_eq!(m["state"], "missing", "Not in your profiles");
    assert_eq!(m["copy"]["name"], "K40");
    assert_eq!(m["copy"]["max_feed"], 12_000);
    assert!(m["connection"].is_null());
    assert_eq!(profile_ids(&s), vec!["generic-laser".to_owned()]);
}

// ---- criterion 4 --------------------------------------------------------------------------------

#[test]
fn ac4_library_changes_survive_a_restart_through_the_host_writes() {
    let mut disk = disk_of(&[k40(1)]);
    let mut s = with_laser(&disk);
    let dup = s.duplicate_machine_profile(&id(1), [20; 16], 1).unwrap();
    let _ = s.rename_machine_profile(&dup, "Second K40", 2);
    let _ = s.duplicate_machine_profile(&id(1), [21; 16], 3).unwrap();
    assert!(s.delete_machine_profile(&id(1)));
    let ws = writes(&mut s);
    apply(&mut disk, &ws);
    assert_eq!(disk.len(), 2, "one record per profile: {:?}", disk.keys());
    for (name, text) in &disk {
        let rec = read_record(name, text).unwrap();
        assert!(
            !text.contains("generic-laser\""),
            "the built-in is never written"
        );
        assert_eq!(text.matches("\"id\"").count(), 1, "one profile per record");
        let _ = rec;
    }
    let restarted = with_laser(&disk);
    let mut before = profile_ids(&s);
    let mut after = profile_ids(&restarted);
    before.sort();
    after.sort();
    assert_eq!(before, after);
    let rows = machine(&restarted)["profiles"].clone();
    assert!(
        rows.as_array()
            .unwrap()
            .iter()
            .any(|r| r["name"] == "Second K40")
    );
}

#[test]
fn ac4_library_writes_never_enter_the_document_or_its_history() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let doc_before = opened(&s).export_json().unwrap();
    let steps = s.undo_len();
    let _ = s.rename_machine_profile(&id(1), "Renamed", 1);
    let _ = s.duplicate_machine_profile(&id(1), [30; 16], 2);
    let _ = s.set_machine_connection("10.0.0.1", "COM3", 9_600.0, 3);
    let _ = s.set_machine_max_feed(1_234.0);
    let doc_mid = opened(&s).export_json().unwrap();
    assert_ne!(doc_before, doc_mid);
    let _ = s.undo();
    let _ = s.update_machine_profile(4);
    assert_eq!(
        s.undo_len(),
        steps,
        "only the field edit was a step, and it was undone"
    );
    assert_eq!(opened(&s).export_json().unwrap(), doc_before);
}

// ---- criterion 5 --------------------------------------------------------------------------------

#[test]
fn ac5_pick_takes_values_and_work_area_in_one_step_and_content_keeps_its_place() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    // `0101` criterion 18: the unassigned content is in the Not assigned look under "View:
    // Operations"; this test reads the drawing's own colour, which "View: Drawn" keeps.
    s.set_operations_view_mode(curvyo_editor_wasm::OperationsViewMode::Drawn);
    let red = red_box(&s);
    let page_1 = |s: &Session| {
        let p = opened(s)
            .pages()
            .into_iter()
            .find(|p| p.id == "first")
            .unwrap();
        (p.origin.x, p.origin.y)
    };
    let corner = page_1(&s);
    let steps = s.undo_len();
    assert!(s.pick_machine_profile(&id(1)));
    assert_eq!(s.undo_len(), steps + 1, "one undo step");
    let c = copy(&s);
    assert_eq!(c["id"], json!(id(1)));
    assert_eq!(c["name"], "K40");
    assert_eq!(c["origin"], "top_right");
    assert_eq!(c["max_feed"], 12_000);
    assert_eq!(c["max_power"], 255);
    assert_eq!(c["laser_mode"], false);
    assert_eq!(work_area(&s), (600.0, 400.0));
    // The K40's origin is top right: the laser page grows to the left, and Page 1 beside its left
    // edge moves with that edge, carrying its drawing (`specs/0109` criterion 32).
    let moved = page_1(&s);
    let (dx, dy) = (moved.0 - corner.0, moved.1 - corner.1);
    assert!(dx < 0.0 && dy.abs() < 1e-9, "Page 1 moved left: {dx}");
    let after = red_box(&s);
    for (a, b) in [
        (red.0 + dx, after.0),
        (red.1, after.1),
        (red.2 + dx, after.2),
        (red.3, after.3),
    ] {
        assert!(
            (a - b).abs() < 0.05,
            "content keeps its place on its page: {red:?} vs {after:?}"
        );
    }
    assert!(machine(&s)["state"].is_null());
    let _ = s.undo();
    assert_eq!(copy(&s)["id"], "generic-laser");
    assert_eq!(work_area(&s), (400.0, 300.0));
}

#[test]
fn ac5_picking_the_chosen_profile_adds_no_step_and_an_unknown_id_does_nothing() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let steps = s.undo_len();
    assert!(!s.pick_machine_profile(&id(1)));
    assert_eq!(s.undo_len(), steps);
    assert!(!s.pick_machine_profile(&id(99)));
    assert!(!s.pick_machine_profile(""));
    assert_eq!(s.undo_len(), steps);
    assert_eq!(copy(&s)["id"], json!(id(1)));
}

// ---- criterion 6 and 15 -------------------------------------------------------------------------

#[test]
fn ac6_the_file_holds_the_copy_and_id_and_never_the_connection() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let bytes = s.pack("0.1.0").unwrap();
    let loro = member(&bytes, "document.loro");
    let json_member = member(&bytes, "document.json");
    for hay in [&loro, &json_member] {
        assert!(!contains(hay, ADDRESS), "address leaked");
        assert!(!contains(hay, PORT), "port leaked");
        assert!(!contains(hay, "250000"), "baud leaked");
        assert!(!contains(hay, "connector_address"));
    }
    let d = unpack(1, &bytes).unwrap();
    let p = d.manufacturings()[0].profile.clone().unwrap();
    assert_eq!(p["id"], json!(id(1)));
    assert_eq!(p["name"], "K40");
}

#[test]
fn ac6_editing_or_deleting_the_library_profile_does_not_change_the_manufacturing() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let before = opened(&s).manufacturings();
    let _ = s.rename_machine_profile(&id(1), "Other", 1);
    assert!(s.delete_machine_profile(&id(1)));
    assert_eq!(opened(&s).manufacturings(), before);
}

#[test]
fn ac15_a_record_holds_no_project_content() {
    let mut s = with_laser(&Disk::new());
    let _ = s.set_machine_name("Project-free");
    let _ = s.save_machine_profile([7; 16], 1);
    let ws = writes(&mut s);
    let text = library_writes(&ws)[0]["text"].as_str().unwrap().to_owned();
    // Since record format 2 (`0102`, ADR 0050) the record holds the bed's fixture set, here
    // the empty one; no other project content.
    assert!(text.contains("\"fixtures\": []"), "{text}");
    let text = text.replace("\"fixtures\": []", "");
    for word in ["Laser 1", "placement", "page", "fixture", "manufacturing"] {
        assert!(!text.contains(word), "{word} in the record: {text}");
    }
}

// ---- criterion 7 --------------------------------------------------------------------------------

#[test]
fn ac7_an_edit_shows_differs_and_typing_the_value_back_clears_it() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let _ = writes(&mut s);
    let _ = s.set_machine_max_feed(11_000.0);
    let m = machine(&s);
    assert_eq!(m["state"], "differs");
    assert_eq!(m["can_update"], true);
    assert!(
        library_writes(&writes(&mut s)).is_empty(),
        "an edit never writes the library"
    );
    let _ = s.set_machine_max_feed(12_000.0);
    assert!(machine(&s)["state"].is_null());
    assert!(s.set_machine_origin("bottom_right"));
    assert_eq!(machine(&s)["state"], "differs");
}

#[test]
fn ac7_reset_takes_the_library_values_in_one_step_and_asks_nothing() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let _ = s.set_machine_max_power(77.0);
    let _ = s.rename_machine_profile(&id(1), "K40 renamed", 3);
    let _ = writes(&mut s);
    let steps = s.undo_len();
    assert!(s.reset_machine_profile());
    assert_eq!(s.undo_len(), steps + 1);
    let c = copy(&s);
    assert_eq!(c["max_power"], 255);
    assert_eq!(c["name"], "K40 renamed", "Reset takes the new name");
    assert_eq!(work_area(&s), (600.0, 400.0));
    assert!(machine(&s)["state"].is_null());
    assert!(library_writes(&writes(&mut s)).is_empty());
    let _ = s.undo();
    assert_eq!(copy(&s)["max_power"], 77);
}

#[test]
fn ac7_update_overwrites_the_record_keeps_its_connection_and_adds_no_step() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let _ = s.set_machine_max_feed(9_999.0);
    let _ = writes(&mut s);
    let steps = s.undo_len();
    let _ = s.update_machine_profile(77);
    assert_eq!(s.undo_len(), steps, "Update is no history step");
    let ws = writes(&mut s);
    let lib = library_writes(&ws);
    assert_eq!(lib.len(), 1);
    assert_eq!(lib[0]["id"], json!(id(1)));
    let rec = read_record(&record_file_name(&id(1)), lib[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(rec.values.max_feed.get(), 9_999);
    assert_eq!(rec.work_area, DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(
        rec.connection.connector_address, ADDRESS,
        "connection stays"
    );
    assert_eq!(rec.modified, 77);
    assert!(machine(&s)["state"].is_null());
    // Undo of the earlier field edit now makes the copy differ from the updated record.
    let _ = s.undo();
    assert_eq!(machine(&s)["state"], "differs");
}

#[test]
fn ac7_update_keeps_the_old_name_in_the_library_after_a_library_rename() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let _ = s.rename_machine_profile(&id(1), "New name", 1);
    let _ = writes(&mut s);
    let _ = s.update_machine_profile(2);
    let ws = writes(&mut s);
    let rec = read_record(
        &record_file_name(&id(1)),
        library_writes(&ws)[0]["text"].as_str().unwrap(),
    )
    .unwrap();
    assert_eq!(rec.values.name, "K40", "Update writes the copy's name back");
}

#[test]
fn ac7_update_and_reset_do_nothing_without_a_record() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    assert!(s.delete_machine_profile(&id(1)));
    let _ = writes(&mut s);
    let steps = s.undo_len();
    assert!(!s.reset_machine_profile());
    let _ = s.update_machine_profile(1);
    assert_eq!(s.undo_len(), steps);
    assert!(library_writes(&writes(&mut s)).is_empty());
}

// ---- criterion 8 --------------------------------------------------------------------------------

/// A project whose manufacturing points at `profile`, made on "another computer".
fn foreign_file(profile: &MachineProfile) -> Vec<u8> {
    let mut other = with_laser(&disk_of(std::slice::from_ref(profile)));
    assert!(other.pick_machine_profile(&profile.id));
    other.pack("0.1.0").unwrap()
}

#[test]
fn ac8_a_file_with_an_unknown_profile_opens_unchanged_as_not_in_your_profiles() {
    let bytes = foreign_file(&k40(1));
    let mut s = open_laser(&bytes, &Disk::new());
    let m = machine(&s);
    assert_eq!(m["state"], "missing");
    assert_eq!(m["copy"]["name"], "K40");
    assert_eq!(m["copy"]["max_feed"], 12_000);
    assert!(m["connection"].is_null(), "connection unavailable");
    assert_eq!(s.undo_len(), 0, "opening adds no step");
    assert!(writes(&mut s).is_empty(), "opening writes nothing");
    assert_eq!(
        opened(&s).manufacturings(),
        unpack(1, &bytes).unwrap().manufacturings(),
        "nothing in the file changes"
    );
    // Every field editable.
    let _ = s.set_machine_max_power(300.0);
    assert_eq!(copy(&s)["max_power"], 300);
    assert!(s.set_machine_origin("top_left"));
}

#[test]
fn ac8_save_as_profile_adds_the_copy_under_the_same_id() {
    let bytes = foreign_file(&k40(1));
    let mut s = open_laser(&bytes, &Disk::new());
    let _ = s.save_machine_profile([42; 16], 9);
    assert_eq!(copy(&s)["id"], json!(id(1)), "the same id");
    let ws = writes(&mut s);
    let lib = library_writes(&ws);
    assert_eq!(lib.len(), 1);
    assert_eq!(lib[0]["id"], json!(id(1)));
    let rec = read_record(&record_file_name(&id(1)), lib[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(rec.values.name, "K40");
    assert_eq!(
        rec.connection.connector_address, "",
        "the connection did not travel"
    );
    assert!(machine(&s)["state"].is_null(), "in sync, not Differs");
    assert!(machine(&s)["connection"].is_object());
}

#[test]
fn ac8_save_as_of_an_unknown_id_does_not_overwrite_a_skipped_newer_record() {
    // The library holds this id in a record of a newer format (skipped at load, ADR 0038 §3).
    let p = k40(1);
    let bytes = foreign_file(&p);
    let mut newer: Value = serde_json::from_str(&write_record(&p)).unwrap();
    // Format 3 is current since `0103` (ADR 0052 §7); 4 is newer.
    newer["format_version"] = json!(4);
    let mut disk = Disk::new();
    disk.insert(record_file_name(&p.id), newer.to_string());
    let mut s = open_laser(&bytes, &disk);
    assert_eq!(machine(&s)["state"], "missing");
    let _ = s.save_machine_profile([43; 16], 9);
    let ws = writes(&mut s);
    assert!(
        library_writes(&ws).iter().all(|w| w["id"] != json!(p.id)),
        "the newer record must never be overwritten: {ws:?}"
    );
}

// ---- criterion 9 --------------------------------------------------------------------------------

#[test]
fn ac9_connection_fields_edit_the_library_record_only_for_a_user_profile() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let _ = writes(&mut s);
    let c = &machine(&s)["connection"];
    assert_eq!(c["connector_address"], ADDRESS);
    assert_eq!(c["controller_port"], PORT);
    assert_eq!(c["baud_rate"], 250_000);
    let steps = s.undo_len();
    let doc = opened(&s).export_json().unwrap();
    let notice = s.set_machine_connection("10.0.0.9", "COM3", 5_000_000.0, 5);
    assert_eq!(notice.as_deref(), Some("Baud rate limited to 4,000,000."));
    assert_eq!(s.undo_len(), steps, "no history step");
    assert_eq!(
        opened(&s).export_json().unwrap(),
        doc,
        "the document is untouched"
    );
    let ws = writes(&mut s);
    let lib = library_writes(&ws);
    assert_eq!(lib.len(), 1);
    let rec = read_record(&record_file_name(&id(1)), lib[0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(rec.connection.controller_port, "COM3");
    assert_eq!(rec.connection.baud_rate.get(), 4_000_000);
    assert!(
        machine(&s)["state"].is_null(),
        "the connection is not part of Differs"
    );
    let long = "a".repeat(250);
    assert_eq!(
        s.set_machine_connection(&long, "COM3", 9_600.0, 6)
            .as_deref(),
        Some("Address limited to 200 characters.")
    );
}

#[test]
fn ac9_connection_is_unavailable_for_the_built_in_and_writes_nothing() {
    let mut s = with_laser(&Disk::new());
    assert!(machine(&s)["connection"].is_null());
    let _ = s.set_machine_connection("10.0.0.1", "COM1", 9_600.0, 1);
    assert!(library_writes(&writes(&mut s)).is_empty());
}

// ---- criterion 10 -------------------------------------------------------------------------------

#[test]
fn ac10_add_manufacturing_starts_with_the_last_pick_in_one_step() {
    let disk = disk_of(&[k40(1)]);
    let mut first = with_laser(&disk);
    assert!(first.pick_machine_profile(&id(1)));
    let ws = writes(&mut first);
    let choice = ws
        .iter()
        .find(|w| w["kind"] == "choice")
        .expect("a pick is remembered on this computer")["text"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut s = session(&disk);
    s.load_machine_choice(&choice);
    let steps = s.undo_len();
    s.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(s.undo_len(), steps + 1, "still one step");
    assert_eq!(copy(&s)["id"], json!(id(1)));
    assert_eq!(copy(&s)["name"], "K40");
    assert_eq!(work_area(&s), (600.0, 400.0));
    let _ = s.undo();
    assert!(opened(&s).manufacturings().is_empty());
}

#[test]
fn ac10_a_last_pick_that_no_longer_exists_falls_back_to_generic_laser() {
    let mut s = session(&Disk::new());
    s.load_machine_choice(&format!(
        "{{\"format_version\": 1, \"last_picked\": \"{}\"}}",
        id(77)
    ));
    s.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(copy(&s)["id"], "generic-laser");
    assert_eq!(work_area(&s), (400.0, 300.0));
    let mut s = session(&Disk::new());
    s.load_machine_choice("garbage");
    s.add_page_of_kind(LASER, None).unwrap();
    assert_eq!(copy(&s)["id"], "generic-laser");
}

#[test]
fn ac10_opening_a_file_or_undoing_a_pick_does_not_change_the_last_pick() {
    let bytes = foreign_file(&k40(1));
    let mut s = Session::open(9320, &bytes).unwrap();
    s.load_machine_profiles_json(&library_json(&disk_of(&[k40(1)])));
    let ws = writes(&mut s);
    assert!(
        ws.iter().all(|w| w["kind"] != "choice"),
        "a pick in the file is not mine: {ws:?}"
    );
}

// ---- criterion 11 -------------------------------------------------------------------------------

#[test]
fn ac11_the_origin_marker_sits_at_the_chosen_corner_and_moves_no_content() {
    let mut s = with_laser(&Disk::new());
    let (w, h) = work_area(&s);
    let red = red_box(&s);
    // The laser page lies in the plane (`0098` criterion 10): boxes relative to its corner.
    let o = s.active_page().origin;
    let local = |b: (f64, f64, f64, f64)| (b.0 - o.x, b.1 - o.y, b.2 - o.x, b.3 - o.y);
    let bl = local(marker_box(&s).expect("a marker is drawn at bottom-left"));
    assert!(bl.0 >= -1.0 && bl.2 < w / 2.0, "left half: {bl:?}");
    assert!(bl.1 > h / 2.0 && bl.3 <= h + 1.0, "bottom half: {bl:?}");
    let steps = s.undo_len();
    assert!(s.set_machine_origin("top_right"));
    assert_eq!(s.undo_len(), steps + 1, "a corner change is one step");
    let tr = local(marker_box(&s).expect("a marker at top-right"));
    assert!(tr.0 > w / 2.0 && tr.2 <= w + 1.0, "right half: {tr:?}");
    assert!(tr.1 >= -1.0 && tr.3 < h / 2.0, "top half: {tr:?}");
    assert_eq!(red_box(&s), red, "no content moves");
    assert_eq!(work_area(&s), (w, h));
}

#[test]
fn ac11_the_marker_is_not_in_the_saved_file_or_the_drawing() {
    let mut s = with_laser(&Disk::new());
    let objects = opened(&s).read_objects().objects.len();
    assert!(s.set_machine_origin("top_left"));
    assert_eq!(
        opened(&s).read_objects().objects.len(),
        objects,
        "no object added"
    );
}

// ---- criterion 12 -------------------------------------------------------------------------------

/// The golden of document-core and a library record equal to its copy.
fn golden_with_record() -> (Vec<u8>, MachineProfile) {
    let bytes = fixture("machine_profile_v36.curvyo");
    let d = unpack(1, &bytes).unwrap();
    let m = &d.manufacturings()[0];
    let read = read_copy(m.profile.as_ref(), None);
    let mut p = MachineProfile::generic_laser();
    p.id = read.copy.id.clone().unwrap();
    p.values = read.copy.values;
    p.work_area = m.work_area();
    (bytes, p)
}

fn open_laser(bytes: &[u8], disk: &Disk) -> Session {
    let mut s = Session::open(9330, bytes).unwrap();
    s.load_machine_profiles_json(&library_json(disk));
    let page = serde_json::from_str::<Value>(&s.pages_json())
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["manufacturing"].is_string())
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(s.activate_page(&page));
    s
}

#[test]
fn ac12_differs_is_derived_for_the_golden_against_a_fixture_record() {
    let (bytes, record) = golden_with_record();
    let s = open_laser(&bytes, &disk_of(std::slice::from_ref(&record)));
    assert!(machine(&s)["state"].is_null(), "equal record: no Differs");
    assert_eq!(work_area(&s), (600.0, 400.0));
    let mut changed = record.clone();
    changed.values.max_power = PowerValue::new(4_242).unwrap();
    let s = open_laser(&bytes, &disk_of(&[changed]));
    assert_eq!(machine(&s)["state"], "differs");
    let s = open_laser(&bytes, &Disk::new());
    assert_eq!(machine(&s)["state"], "missing");
}

#[test]
fn ac12_save_and_reopen_keeps_id_copy_and_work_area_and_the_state() {
    let mut s = with_laser(&disk_of(&[k40(1)]));
    assert!(s.pick_machine_profile(&id(1)));
    let _ = s.set_machine_max_feed(3_333.0);
    let before = copy(&s);
    let state = machine(&s)["state"].clone();
    let bytes = s.pack("0.1.0").unwrap();
    let back = open_laser(&bytes, &disk_of(&[k40(1)]));
    assert_eq!(copy(&back), before);
    assert_eq!(machine(&back)["state"], state);
}

#[test]
fn ac12_the_damaged_golden_shows_defaults_and_the_name_notice_and_writes_nothing() {
    let bytes = fixture("machine_profile_damaged_v36.curvyo");
    let mut s = open_laser(&bytes, &Disk::new());
    let m = machine(&s);
    assert_eq!(m["copy"]["name"], "Generic laser");
    assert_eq!(m["name_notice"], "Name was damaged, showing Generic laser");
    assert_eq!(m["copy"]["max_feed"], 6_000);
    assert_eq!(m["copy"]["max_power"], 1_000);
    assert_eq!(s.undo_len(), 0);
    assert!(writes(&mut s).is_empty());
    let before = unpack(1, &bytes).unwrap().manufacturings();
    assert_eq!(
        opened(&s).manufacturings(),
        before,
        "never rewritten on read"
    );
}

#[test]
fn ac12_a_format_35_file_opens_as_generic_laser_and_writes_nothing_until_an_edit() {
    let bytes = fixture("laser_workbench_v35.curvyo");
    let legacy = unpack(1, &bytes).unwrap().manufacturings()[0]
        .legacy_machine_name
        .clone()
        .unwrap();
    let mut s = open_laser(&bytes, &Disk::new());
    let m = machine(&s);
    assert_eq!(m["copy"]["id"], "generic-laser");
    assert_eq!(m["copy"]["name"], json!(legacy));
    assert_eq!(m["copy"]["max_feed"], 6_000);
    assert!(m["name_notice"].is_null());
    assert_eq!(s.undo_len(), 0);
    assert!(writes(&mut s).is_empty());
    let reopened = opened(&s).manufacturings();
    assert_eq!(reopened[0].profile, None, "no copy written on open");
    assert_eq!(
        reopened[0].legacy_machine_name.as_deref(),
        Some(legacy.as_str())
    );
    let _ = s.set_machine_max_feed(4_000.0);
    let edited = opened(&s).manufacturings();
    assert!(edited[0].profile.is_some());
    assert_eq!(
        edited[0].legacy_machine_name, None,
        "the old key goes with the first edit"
    );
}

// ---- criterion 13 -------------------------------------------------------------------------------

#[test]
fn ac13_damaged_records_are_skipped_counted_in_the_tab_and_never_written() {
    let mut disk = disk_of(&[k40(1)]);
    disk.insert(record_file_name(&id(2)), "{ nope".to_owned());
    disk.insert(record_file_name(&id(3)), "[]".to_owned());
    let mut newer: Value = serde_json::from_str(&write_record(&k40(4))).unwrap();
    newer["format_version"] = json!(9);
    disk.insert(record_file_name(&id(4)), newer.to_string());
    disk.insert("readme.txt".to_owned(), "hello".to_owned());
    let original = disk.clone();
    let mut s = with_laser(&disk);
    let m = machine(&s);
    let notices: Vec<String> = m["library_notices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap().to_owned())
        .collect();
    let joined = notices.join(" | ");
    assert!(joined.contains("2 profiles could not be read"), "{joined}");
    assert!(
        joined.contains("1 profile is from a newer version"),
        "{joined}"
    );
    assert!(notices.iter().all(|n| n.chars().count() <= 70));
    assert_eq!(profile_ids(&s).len(), 2, "built-in plus the good record");
    // The library stays editable, and nothing touches the skipped files.
    assert!(s.pick_machine_profile(&id(1)));
    let _ = s.rename_machine_profile(&id(1), "Good one", 1);
    let _ = s.duplicate_machine_profile(&id(1), [50; 16], 2);
    let _ = s.save_machine_profile([51; 16], 3);
    let ws = writes(&mut s);
    apply(&mut disk, &ws);
    for skipped in [
        record_file_name(&id(2)),
        record_file_name(&id(3)),
        record_file_name(&id(4)),
        "readme.txt".to_owned(),
    ] {
        assert_eq!(
            disk.get(&skipped),
            original.get(&skipped),
            "{skipped} untouched"
        );
    }
}

// ---- criterion 14 -------------------------------------------------------------------------------

#[test]
fn ac14_200_records_load_within_200_ms_and_the_list_within_100_ms_of_thread_cpu() {
    let profiles: Vec<MachineProfile> = (0..200_u16)
        .map(|n| {
            let mut seed = [0xc3_u8; 16];
            seed[14..].copy_from_slice(&n.to_be_bytes());
            let mut p = k40(0);
            p.id = new_profile_id(seed);
            p.values.name = format!("{} {n}", "Laser with a long name".repeat(2));
            p.connection.connector_address = "x".repeat(200);
            p.connection.controller_port = "y".repeat(200);
            p
        })
        .collect();
    let json = library_json(&disk_of(&profiles));
    let mut s = Session::open(9340, &drawing()).unwrap();
    let ((), load) = cpu_time::cpu_time_of(|| s.load_machine_profiles_json(&json));
    s.add_page_of_kind(LASER, None).unwrap();
    let (view, list) = cpu_time::cpu_time_of(|| s.manufacturing_view_json());
    let view: Value = serde_json::from_str(&view).unwrap();
    assert_eq!(view["machine"]["profiles"].as_array().unwrap().len(), 201);
    let (load, list) = (load.as_secs_f64(), list.as_secs_f64());
    eprintln!("0026 tester: 200 records load {load:.4} s, list {list:.4} s thread CPU");
    if !cfg!(debug_assertions) {
        assert!(load <= 0.2, "load {load:.4} s");
        assert!(list <= 0.1, "list {list:.4} s");
    }
}
