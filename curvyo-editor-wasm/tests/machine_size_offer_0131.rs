//! `specs/0131` criteria 1 to 4: the offer of the size the controller reports, derived on every
//! read, and Use machine size as one undo step with the library write of a user profile.

#![allow(clippy::unwrap_used, clippy::float_cmp)]

use curvyo_document_core::{Document, DocumentSize, Point, RectBounds, pack};
use curvyo_editor_wasm::Session;
use curvyo_laser_core::{MachineProfile, new_profile_id, record_file_name, write_record};
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

fn user_profile() -> MachineProfile {
    let mut profile = MachineProfile::generic_laser();
    profile.id = new_profile_id([7; 16]);
    "Garage laser".clone_into(&mut profile.values.name);
    profile
}

/// A laser page on the user profile (400 x 300) or on the built-in one.
fn laser(user: bool) -> Session {
    let mut s = session();
    let profile = user_profile();
    let text = serde_json::json!([{"name": record_file_name(&profile.id), "text": write_record(&profile)}]);
    s.load_machine_profiles_json(&text.to_string());
    s.add_page_of_kind(LASER, None).unwrap();
    if user {
        assert!(s.pick_machine_profile(&profile.id));
    }
    let _ = s.take_machine_writes_json();
    s
}

fn writes(s: &mut Session) -> Vec<Value> {
    serde_json::from_str(&s.take_machine_writes_json()).unwrap()
}

fn size(s: &Session) -> DocumentSize {
    s.active_page().size
}

#[test]
fn the_offer_is_derived_from_the_copy_on_every_read() {
    let s = laser(true);
    assert_eq!(s.machine_size_offer(400.0, 850.0), Some([400.0, 850.0]));
    assert_eq!(s.machine_size_offer(400.0, 300.0), None);
    assert_eq!(s.machine_size_offer(400.005, 300.0), None);
    assert_eq!(s.machine_size_offer(0.0, 850.0), None);
    assert_eq!(s.machine_size_offer(f64::NAN, 850.0), None);
}

#[test]
fn no_offer_outside_a_laser_page() {
    let s = session();
    assert_eq!(s.machine_size_offer(400.0, 850.0), None);
}

#[test]
fn use_machine_size_is_one_undo_step_and_writes_the_user_profile() {
    let mut s = laser(true);
    let steps = s.history_len();
    assert!(s.use_machine_size(400.0, 850.0, 9));
    assert_eq!(s.history_len(), steps + 1, "one step");
    assert_eq!(size(&s), DocumentSize::from_mm(400.0, 850.0));
    assert_eq!(s.machine_size_offer(400.0, 850.0), None, "the line goes");
    let w = writes(&mut s);
    assert_eq!(w.len(), 1, "one library write: {w:?}");
    assert_eq!(w[0]["kind"], "write");
    let view: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    assert_eq!(
        view["machine"]["state"],
        Value::Null,
        "copy equals the record"
    );
    assert!(
        !s.use_machine_size(400.0, 850.0, 10),
        "a second press does nothing"
    );
    assert_eq!(s.history_len(), steps + 1);
    s.undo();
    assert_eq!(size(&s), DocumentSize::from_mm(400.0, 300.0));
    assert_eq!(
        s.machine_size_offer(400.0, 850.0),
        Some([400.0, 850.0]),
        "offer is back"
    );
    let view: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    assert_eq!(
        view["machine"]["state"], "differs",
        "the library kept the new size"
    );
}

#[test]
fn the_built_in_profile_changes_the_copy_only() {
    let mut s = laser(false);
    assert!(s.use_machine_size(400.0, 850.0, 9));
    assert_eq!(size(&s), DocumentSize::from_mm(400.0, 850.0));
    assert!(writes(&mut s).is_empty(), "the built-in is not editable");
    let view: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    assert_eq!(view["machine"]["state"], "differs");
}

#[test]
fn the_page_grows_about_the_origin_corner() {
    let mut s = laser(true);
    let before = s.active_page();
    assert!(s.use_machine_size(400.0, 850.0, 9));
    let after = s.active_page();
    assert_eq!(after.origin.x, before.origin.x);
    assert!(
        (after.origin.y + 850.0 - (before.origin.y + 300.0)).abs() < 1e-9,
        "the bottom-left origin corner stays"
    );
}

#[test]
fn a_hand_edit_brings_the_offer_back() {
    let mut s = laser(true);
    assert!(s.use_machine_size(400.0, 850.0, 9));
    s.set_profile_work_area(
        &user_profile().id,
        curvyo_document_core::Length::from_mm(400.0),
        curvyo_document_core::Length::from_mm(500.0),
        10,
    );
    s.reset_machine_profile();
    assert_eq!(size(&s), DocumentSize::from_mm(400.0, 500.0));
    assert_eq!(
        s.machine_size_offer(400.0, 850.0),
        Some([400.0, 850.0]),
        "no refusal memory"
    );
}

#[test]
fn a_copy_not_in_the_library_gets_no_offer() {
    let mut s = laser(true);
    s.load_machine_profiles_json("[]");
    assert_eq!(s.machine_size_offer(400.0, 850.0), None);
    assert!(!s.use_machine_size(400.0, 850.0, 9));
}

#[test]
fn the_place_is_kept_for_a_usable_report_whether_or_not_it_differs() {
    let s = laser(true);
    assert!(s.machine_size_reported(400.0, 300.0));
    assert!(s.machine_size_reported(400.0, 850.0));
    assert!(!s.machine_size_reported(0.0, 850.0));
    assert!(!s.machine_size_reported(400.0, f64::NAN));
    assert!(
        !session().machine_size_reported(400.0, 300.0),
        "no laser page"
    );
}
