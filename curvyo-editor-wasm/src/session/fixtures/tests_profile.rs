//! Sets in the profile at session level (`specs/0113-fixture-sets-and-pins` criteria 3, 9, 10,
//! 32, 44 to 47; ADR 0059 §10, ADR 0060 §5a): an edit writes the record in its step, the other
//! pages follow, undo takes the record back, read only, Generic laser and the guarded alignment.
#![allow(clippy::assert_is_empty)]

use super::super::Session;
use super::tests::{laser, last_label, view};

/// The profile id the active page's copy points at.
fn profile_id(s: &Session) -> String {
    let machine: serde_json::Value =
        serde_json::from_str::<serde_json::Value>(&s.manufacturing_view_json()).unwrap()["machine"]
            .clone();
    machine["copy"]["id"].as_str().unwrap().to_owned()
}

/// The record writes the host has to do, `(id, text)`, in order.
fn writes(s: &mut Session) -> Vec<(String, String)> {
    let all: Vec<serde_json::Value> = serde_json::from_str(&s.take_machine_writes_json()).unwrap();
    all.into_iter()
        .filter(|w| w["kind"] == "write")
        .map(|w| {
            (
                w["id"].as_str().unwrap().to_owned(),
                w["text"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// The library as the host would list it after `writes`: one `{name, text}` entry.
fn listed(id: &str, text: &str) -> String {
    serde_json::json!([{ "name": format!("{id}.json"), "text": text }]).to_string()
}

/// A second laser page on the active page's profile, activated.
fn second_page(s: &mut Session) -> String {
    let profile = profile_id(s);
    s.add_output_page_with("curvyo.laser", Some(&profile))
        .unwrap()
}

#[test]
fn criterion_44_a_set_edit_writes_the_profile_in_its_step_and_the_other_page_follows() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let first = s.active_page().id;
    let second = second_page(&mut s);
    let _ = writes(&mut s);
    assert!(s.activate_page(&first));
    let steps = s.undo_len();
    assert!(s.rename_fixture_set(set, "Bench"));
    assert_eq!(s.undo_len(), steps + 1, "one step");
    assert_eq!(last_label(&s), "rename_fixture_set");
    let written = writes(&mut s);
    assert_eq!(written.len(), 1);
    assert!(written[0].1.contains("\"name\": \"Bench\""));
    assert!(s.activate_page(&second));
    assert_eq!(
        view(&s).sets[0].name,
        "Bench",
        "the other page shows it at once"
    );
    let modified = |s: &Session| -> Vec<i64> {
        s.document
            .manufacturings()
            .iter()
            .map(|m| m.sets_modified)
            .collect()
    };
    let times = modified(&s);
    assert!(
        times[0] > 0 && times[0] == times[1],
        "both pages carry the record time"
    );
}

#[test]
fn criterion_44_page_choices_stay_on_their_page_and_never_write_the_profile() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let holder = s.add_set_fixture(set, "holder").unwrap();
    let first = s.active_page().id;
    second_page(&mut s);
    let _ = writes(&mut s);
    assert!(s.activate_page(&first));
    assert!(s.set_fixture_set_on_page(set, false));
    assert!(s.set_fixture_active(holder, false));
    assert!(s.set_fixture_visible(holder, false));
    assert!(writes(&mut s).is_empty(), "choices are the page's");
    let pages = s.document.manufacturings();
    assert!(!pages[0].sets[0].on_page && pages[1].sets[0].on_page);
    assert!(pages[1].fixtures[0].active && pages[1].fixtures[0].visible);
}

#[test]
fn frage_218_a_set_added_on_one_page_is_off_on_the_other() {
    let mut s = laser();
    let first = s.active_page().id;
    let second = second_page(&mut s);
    assert!(s.activate_page(&first));
    s.add_fixture_set().unwrap();
    assert!(view(&s).sets[0].on_page, "on where it was added");
    assert!(s.activate_page(&second));
    let v = view(&s);
    assert!(!v.sets[0].on_page, "off on the other page");
    assert!(v.sets[0].line2.ends_with("not on this page"));
}

#[test]
fn criterion_45_undo_takes_the_profile_back_and_redo_again() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let id = profile_id(&s);
    let _ = writes(&mut s);
    assert!(s.rename_fixture_set(set, "Bench"));
    let _ = writes(&mut s);
    s.undo();
    let undone = writes(&mut s);
    assert_eq!(undone.len(), 1, "the record is written back");
    assert!(!undone[0].1.contains("Bench"));
    assert_eq!(
        s.machine_profiles().find(&id).unwrap().fixture_sets[0].name,
        "Set 1"
    );
    s.redo();
    let redone = writes(&mut s);
    assert!(redone[0].1.contains("Bench"));
    assert_eq!(view(&s).sets[0].name, "Bench");
}

#[test]
fn criterion_45_an_undo_after_the_profile_changed_elsewhere_leaves_it() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let id = profile_id(&s);
    assert!(s.rename_fixture_set(set, "Bench"));
    let text = writes(&mut s).pop().unwrap().1;
    // Another window renamed the profile's set since; this library reloads it.
    let elsewhere = text
        .replace("\"Bench\"", "\"Jig\"")
        .replace("\"modified\": ", "\"modified\": 9");
    s.load_machine_profiles_json(&listed(&id, &elsewhere));
    let result = s.undo();
    assert!(
        result
            .notice
            .ends_with("changed since, its sets were not undone."),
        "{}",
        result.notice
    );
    assert!(result.notice.chars().count() <= 70);
    assert_eq!(
        s.machine_profiles().find(&id).unwrap().fixture_sets[0].name,
        "Jig",
        "the profile is left"
    );
}

#[test]
fn criterion_45_a_refused_undo_aligns_the_page_to_the_profile_whatever_its_time() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let id = profile_id(&s);
    let first = s.active_page().id;
    let second = second_page(&mut s);
    assert!(s.activate_page(&first));
    assert!(s.rename_fixture_set(set, "Bench"));
    let text = writes(&mut s).pop().unwrap().1;
    // Another device renamed the set with a clock behind this one: the reload does not align.
    let modified = regex_free_modified(&text);
    let elsewhere = text
        .replace("\"Bench\"", "\"Jig\"")
        .replace(&format!("\"modified\": {modified}"), "\"modified\": 1");
    s.load_machine_profiles_json(&listed(&id, &elsewhere));
    assert_eq!(
        view(&s).sets[0].name,
        "Bench",
        "an older record is not newer"
    );
    let result = s.undo();
    assert!(result.notice.ends_with("its sets were not undone."));
    assert_eq!(view(&s).sets[0].name, "Jig", "the page shows the profile");
    assert!(s.activate_page(&second));
    assert_eq!(view(&s).sets[0].name, "Jig", "so does the other page");
}

/// The `modified` number of a record text.
fn regex_free_modified(text: &str) -> String {
    let start = text.find("\"modified\": ").unwrap() + "\"modified\": ".len();
    text[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect()
}

#[test]
fn criterion_45_a_failed_profile_write_undoes_the_edit() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let id = profile_id(&s);
    let _ = writes(&mut s);
    assert!(s.rename_fixture_set(set, "Bench"));
    let notice = s.machine_record_write_failed(&id).unwrap();
    assert!(notice.starts_with("Could not save ") && notice.ends_with(". The change was undone."));
    assert_eq!(view(&s).sets[0].name, "Set 1");
    assert_eq!(
        s.machine_profiles().find(&id).unwrap().fixture_sets[0].name,
        "Set 1"
    );
    assert_eq!(view(&s).sets.len(), 1, "only the failed edit is undone");
    assert_eq!(
        s.machine_record_write_failed("other-id"),
        None,
        "another record"
    );
}

#[test]
fn criterion_3_generic_laser_has_no_plus_set_and_shows_the_hint() {
    let mut s = Session::new(1);
    s.add_page_of_kind("curvyo.laser", None).unwrap();
    assert!(!s.fixture_sets_editable());
    assert_eq!(s.add_fixture_set(), None);
    let v = view(&s);
    assert!(!v.can_add_set);
    assert_eq!(
        v.profile_hint.as_deref(),
        Some(
            "Fixture sets are kept in a machine profile. Save this machine as a profile to add sets."
        )
    );
    assert_eq!(v.read_only, None);
    s.save_machine_profile([3; 16], 5);
    assert!(s.fixture_sets_editable());
    assert!(view(&s).can_add_set && view(&s).profile_hint.is_none());
}

#[test]
fn criterion_46_a_profile_not_on_this_computer_shows_its_sets_read_only() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let holder = s.add_set_fixture(set, "holder").unwrap();
    let bytes = s.pack("0.1.0").unwrap();
    let mut other = Session::open(9, &bytes).unwrap();
    let page = other
        .document
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    assert!(other.activate_page(&page));
    let v = view(&other);
    assert_eq!(v.sets.len(), 1, "the stored sets are shown");
    let line = v.read_only.unwrap();
    assert!(
        line.starts_with("From the project file. ") && line.ends_with(" is not on this computer.")
    );
    assert!(!v.can_add_set && !v.sets[0].can_add);
    assert!(!other.rename_fixture_set(set, "Bench"), "no content edit");
    assert!(!other.delete_fixture(holder));
    assert!(!other.set_fixture_set_locked(set, true));
    assert!(
        other.set_fixture_set_on_page(set, false),
        "On this page still works"
    );
    assert!(
        other.set_fixture_active(holder, false),
        "On the bed still works"
    );
    other.save_machine_profile([5; 16], 7);
    assert!(
        other.fixture_sets_editable(),
        "Save as profile makes them editable"
    );
    assert!(other.rename_fixture_set(set, "Bench"));
    let read = other.document.manufacturings().remove(0);
    assert!(!read.sets[0].on_page, "the page's choices stay attached");
}

#[test]
fn criterion_47_a_newer_profile_aligns_at_load_without_an_undo_step() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    s.add_set_fixture(set, "holder").unwrap();
    let id = profile_id(&s);
    let text = writes(&mut s).pop().unwrap().1;
    let bytes = s.pack("0.1.0").unwrap();
    // The other window: the same project and profile; its set is renamed there and one added.
    let mut there = Session::open(5, &bytes).unwrap();
    there.load_machine_profiles_json(&listed(&id, &text));
    let page = there
        .document
        .pages()
        .into_iter()
        .find(|p| p.kind.manufacturing().is_some())
        .unwrap()
        .id;
    assert!(there.activate_page(&page));
    assert!(there.rename_fixture_set(set, "Bench"));
    there.add_fixture_set().unwrap();
    let newer = writes(&mut there).pop().unwrap().1;
    // Back here: the reload aligns this project, no undo step, the project counts as changed.
    let steps = s.undo_len();
    s.mark_saved();
    s.load_machine_profiles_json(&listed(&id, &newer));
    assert_eq!(s.undo_len(), steps, "no undo step");
    assert!(s.is_dirty());
    let v = view(&s);
    assert_eq!(v.sets[0].name, "Bench");
    assert_eq!(v.sets.len(), 2);
    assert!(!v.sets[1].on_page, "a new set reads not on this page");
    // The same record again finds nothing to do.
    let version = s.document.version();
    s.load_machine_profiles_json(&listed(&id, &newer));
    assert_eq!(s.document.version(), version, "nothing written");
}

#[test]
fn criterion_9_pick_shows_the_profiles_sets_all_on_and_reset_leaves_them() {
    let mut s = laser();
    let set = s.add_fixture_set().unwrap();
    let id = profile_id(&s);
    second_page(&mut s);
    let other = s.document.manufacturings()[1].sets[0].id;
    assert!(s.set_fixture_set_on_page(other, false));
    assert!(s.pick_machine_profile("generic-laser"));
    assert!(view(&s).sets.is_empty(), "Generic laser leaves no set");
    assert!(s.pick_machine_profile(&id));
    assert!(view(&s).sets[0].on_page, "every set on after a pick");
    assert!(!s.reset_machine_profile(), "Reset leaves the sets alone");
    let _ = set;
}
