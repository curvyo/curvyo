//! Send to machine: the button's state in the group view, the job handed out, and the commit
//! after the connector accepted it (`specs/0125-send-to-machine` S1 to S6, S10 to S12, S16).

use curvyo_document_core::NodeId;
use curvyo_laser_core::{MachineProfile, new_profile_id, record_file_name, write_record};

use super::{cut, laser_with_square, undo_label, view};
use crate::session::Session;
use crate::session::test_grid::tests::{place_grid, recipe_of};
use crate::session::text_tests;

const ADDRESS: &str = "192.168.1.20:7447";

fn connector_profile() -> MachineProfile {
    let mut profile = MachineProfile::generic_laser();
    profile.id = new_profile_id([3; 16]);
    profile.modified = 1_000;
    profile.values.name = "K40".to_owned();
    profile.connection.connector_address = ADDRESS.to_owned();
    profile
}

fn load_connector_profile(s: &mut Session) -> String {
    let profile = connector_profile();
    s.load_machine_profiles(&[(record_file_name(&profile.id), write_record(&profile))]);
    profile.id
}

/// A laser page with a square and a Cut, on a profile that names a connector, paired.
fn paired_page() -> (Session, NodeId) {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let id = load_connector_profile(&mut s);
    assert!(s.pick_machine_profile(&id));
    s.set_send_link("paired");
    (s, square)
}

fn send_of(s: &Session) -> serde_json::Value {
    view(s)["send"].clone()
}

#[test]
fn s1_there_is_a_send_button_only_for_a_profile_with_a_connector() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    assert!(send_of(&s).is_null(), "Generic laser has no connector");
    assert!(s.send_job().is_none());
    let id = load_connector_profile(&mut s);
    assert!(s.pick_machine_profile(&id));
    let send = send_of(&s);
    assert_eq!(send["address"], ADDRESS);
    assert_eq!(send["profile_id"], id);
    assert_eq!(send["cause"], "not_paired", "a new page is not paired yet");
}

#[test]
fn s2_to_s5_the_cause_follows_the_documented_order() {
    let (mut s, _) = paired_page();
    let send = send_of(&s);
    assert_eq!(send["enabled"], true);
    assert!(send["cause"].is_null());
    for (link, cause) in [
        ("browser", "browser"),
        ("not_paired", "not_paired"),
        ("no_longer_paired", "no_longer_paired"),
        ("key_changed", "key_changed"),
    ] {
        s.set_send_link(link);
        let send = send_of(&s);
        assert_eq!(send["enabled"], false, "{link}");
        assert_eq!(send["cause"], cause);
        assert!(s.send_job().is_none(), "{link}: a press sends nothing");
    }
    assert_eq!(view(&s)["export"]["enabled"], true, "Export is unaffected");
}

#[test]
fn s2_a_blocked_page_disables_both_buttons_with_the_same_text() {
    let (mut s, _) = laser_with_square();
    let id = load_connector_profile(&mut s);
    assert!(s.pick_machine_profile(&id));
    s.set_send_link("paired");
    let v = view(&s);
    assert_eq!(v["export"]["enabled"], false, "nothing runs");
    assert_eq!(v["send"]["enabled"], false);
    assert_eq!(v["send"]["cause"], "blocked");
    assert_eq!(v["send"]["text"], v["export"]["tooltip"]);
    assert!(s.send_job().is_none());
}

#[test]
fn s6_the_bytes_are_the_file_export_writes_and_the_name_is_the_pages() {
    let (mut s, _) = paired_page();
    let exported = s.export_job("/home/m/Coasters.curvyo").unwrap();
    let steps = s.undo_len();
    let sent = s.send_job().unwrap();
    assert_eq!(s.undo_len(), steps, "handing out writes nothing");
    assert_eq!(sent.bytes, exported.bytes);
    assert!(exported.name.contains(&sent.name), "{}", exported.name);
    assert_eq!(sent.address, ADDRESS);
    assert_eq!(sent.profile_id, send_of(&s)["profile_id"]);
}

#[test]
fn s8_a_send_in_progress_disables_send_and_a_second_press_sends_nothing() {
    let (mut s, _) = paired_page();
    let first = s.send_job().unwrap();
    let button = send_of(&s);
    assert_eq!(button["enabled"], false);
    assert_eq!(button["cause"], "sending");
    assert!(s.send_job().is_none());
    assert!(s.export_job("").is_some(), "Export stays enabled");
    s.send_job_failed(first.token);
    assert_eq!(send_of(&s)["enabled"], true);
}

#[test]
fn s11_a_failed_send_commits_nothing_and_a_used_token_commits_nothing() {
    let (mut s, _) = paired_page();
    let steps = s.undo_len();
    let sent = s.send_job().unwrap();
    s.send_job_failed(sent.token);
    assert!(s.send_job_done(sent.token, 1).is_none());
    assert_eq!(s.undo_len(), steps);
    let sent = s.send_job().unwrap();
    assert!(
        s.send_job_done(sent.token + 7, 1).is_none(),
        "unknown token"
    );
    assert!(
        s.send_job_done(sent.token, 1).is_none(),
        "the pending send was kept for its own token"
    );
}

fn fixture_job_page() -> (Session, NodeId) {
    use curvyo_document_core::FixtureJobSwitch;
    let (mut s, square) = paired_page();
    let holder = crate::session::fixtures::add_fixture(&mut s, "holder").unwrap();
    assert!(s.select_fixture(holder));
    let job = s.add_fixture_job("score").unwrap();
    assert!(s.set_fixture_job_part(job, "board", true));
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    let birch = s.add_material("Birch 3 mm", 3.0, &[1; 16]).unwrap();
    let laser = s.active_profile_id().unwrap();
    s.add_material_set(&birch, &laser, "score", &[2; 16])
        .unwrap();
    assert!(s.set_fixture_material(&birch));
    let _ = square;
    (s, job)
}

#[test]
fn s10_the_commit_is_one_step_switches_fixture_jobs_off_and_undo_restores_them() {
    let (mut s, job) = fixture_job_page();
    let sent = s.send_job().unwrap();
    let steps = s.undo_len();
    let (notice, ms) = s.send_job_done(sent.token, 5).unwrap();
    assert_eq!(notice, "Output turned off on 1 fixture job.");
    assert_eq!(ms, 3_000);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Send job to machine");
    assert!(!s.document.fixture_job(job).unwrap().output);
    let _ = s.undo();
    assert!(
        s.document.fixture_job(job).unwrap().output,
        "undo restores (a)"
    );
}

#[test]
fn s12_an_unchanged_second_send_adds_no_step_and_says_nothing() {
    let (mut s, _) = fixture_job_page();
    let first = s.send_job().unwrap();
    assert!(s.send_job_done(first.token, 5).is_some());
    let steps = s.undo_len();
    let second = s.send_job().unwrap();
    assert!(
        s.send_job_done(second.token, 6).is_none(),
        "no notice for the send itself"
    );
    assert_eq!(s.undo_len(), steps, "nothing changed, no step");
}

#[test]
fn s10_a_send_marks_its_test_grid_lasered_and_undo_keeps_it() {
    let mut s = text_tests::session();
    let profile = connector_profile();
    s.load_machine_profiles(&[(record_file_name(&profile.id), write_record(&profile))]);
    s.remember_machine_pick(&profile.id);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    let _ = s.grid_label_font();
    text_tests::load_wanted(&mut s);
    let birch = s.add_material("Birch 3 mm", 3.0, &[5; 16]).unwrap();
    let group = place_grid(&mut s);
    assert_eq!(s.pick_test_grid_material(Some(&birch)), Ok(true));
    s.set_send_link("paired");
    assert_eq!(send_of(&s)["enabled"], true, "{:#}", view(&s)["checks"]);
    let id = recipe_of(&s, group).id;
    let sent = s.send_job().unwrap();
    assert_eq!(
        s.grid_object(group).unwrap().lasered,
        None,
        "not before the connector says so"
    );
    s.send_job_done(sent.token, 1_762_948_800_000);
    assert_eq!(
        s.grid_object(group).unwrap().lasered,
        Some(1_762_948_800_000)
    );
    let _ = id;
    if undo_label(&mut s) == "Send job to machine" {
        let _ = s.undo();
    }
    assert!(
        s.grid_object(group).unwrap().lasered.is_some(),
        "Lasered stays"
    );
    let again = s.send_job().unwrap();
    let steps = s.undo_len();
    s.send_job_done(again.token, 1_762_948_900_000);
    assert_eq!(
        s.undo_len(),
        steps,
        "a second send of an unchanged grid adds no step"
    );
}

#[test]
fn ss9_the_button_starts_only_on_a_free_machine_and_only_while_enabled() {
    let (mut s, _) = paired_page();
    assert_eq!(send_of(&s)["starts"], false, "no status read yet");
    assert!(!s.send_will_start());
    s.set_send_machine("Idle", "");
    assert_eq!(send_of(&s)["starts"], true);
    assert!(s.send_will_start());
    s.set_send_machine("Done", "");
    assert!(s.send_will_start());
    for (state, word) in [
        ("Ready", ""),
        ("Running", ""),
        ("Homing", ""),
        ("Idle", "Door"),
    ] {
        s.set_send_machine(state, word);
        assert_eq!(send_of(&s)["starts"], false, "{state} {word}");
        assert!(!s.send_will_start(), "{state} {word}");
    }
    s.set_send_machine("Idle", "");
    let sent = s.send_job().unwrap();
    assert_eq!(
        send_of(&s)["starts"],
        false,
        "a send in progress disables the button"
    );
    s.send_job_failed(sent.token);
    s.set_send_link("not_paired");
    assert_eq!(
        send_of(&s)["starts"],
        false,
        "a disabled button never starts"
    );
}

#[test]
fn ss13_the_job_number_changes_with_the_job_and_not_with_the_status() {
    let (mut s, _) = paired_page();
    let before = send_of(&s)["job"].clone();
    s.set_send_machine("Idle", "");
    assert_eq!(send_of(&s)["job"], before, "a status is not a job change");
    assert!(s.set_start("current_position") || s.set_start("machine_origin"));
    assert_ne!(
        send_of(&s)["job"],
        before,
        "the Start pick changes the job (P15)"
    );
}
