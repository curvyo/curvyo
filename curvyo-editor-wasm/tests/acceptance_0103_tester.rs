//! Independent acceptance tests for `specs/0103-fixture-jobs` through the public session: the
//! Fixture jobs section and its add (criteria 1, 2), order and rows (3, 4), the Parts list and its
//! ticks (8, 9, 11), the run rule and the summary (13, 15, 17, 19), the look of a part (18), the
//! Unknown kind and the damage notice (21, 22), the profile record format 3 (23, 24) and the view
//! contract of `operations_view_json`. Written from the specification and ADR 0052 before the
//! implementation was read. Texts are searched in the view JSON, so a key name in the view is not
//! what these tests assert; the words of the spec are.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::needless_pass_by_value,
    clippy::similar_names,
    missing_docs
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Document, DocumentSize, FixtureJobSwitch, NewOutputPage, NodeId, pack, unpack,
};
use curvyo_editor_wasm::{Session, Tool};
use serde_json::{Value, json};

const LASER: &str = "curvyo.laser";

// ---- helpers ------------------------------------------------------------------------------------

fn one_page() -> Session {
    let d = Document::new(1);
    let made = d
        .add_output_page(NewOutputPage {
            process: LASER,
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(600.0, 400.0),
        })
        .unwrap();
    let mut s = Session::open(9301, &pack(&d, "0103-tester").unwrap()).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    s.load_machine_profiles_json("[]");
    s.activate_page(&made.id);
    s.wipe_history();
    s
}

fn view(s: &Session) -> Value {
    serde_json::from_str(&s.operations_view_json()).unwrap()
}

/// Every string anywhere in the operations view (the texts the maker reads).
fn texts(s: &Session) -> Vec<String> {
    fn walk(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::String(t) => out.push(t.clone()),
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            Value::Object(o) => o.values().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(&view(s), &mut out);
    out
}

/// The Fixture jobs section's view (`fixture_jobs_view_json`): count, groups, parts, running.
fn fj(s: &Session) -> Value {
    serde_json::from_str(&s.fixture_jobs_view_json()).unwrap()
}

/// How many jobs run, by the section's own count (the summary sentence is the frontend's text).
fn running(s: &Session) -> u64 {
    fj(s)["running"].as_u64().unwrap()
}

/// Every fixture job of the section, in group order (the `fixture_jobs.groups[].jobs[]` contract of
/// ADR 0103 `adrs.md` §6).
fn jobs(s: &Session) -> Vec<Value> {
    fj(s)["groups"]
        .as_array()
        .map(|groups| {
            groups
                .iter()
                .flat_map(|g| g["jobs"].as_array().cloned().unwrap_or_default())
                .collect()
        })
        .unwrap_or_default()
}

fn job_names(s: &Session) -> Vec<String> {
    jobs(s)
        .iter()
        .map(|j| j["name"].as_str().unwrap().to_owned())
        .collect()
}

fn job(s: &Session, name: &str) -> Value {
    jobs(s)
        .into_iter()
        .find(|j| j["name"] == name)
        .unwrap_or_else(|| panic!("no fixture job named {name}: {}", view(s)))
}

fn job_id(s: &Session, name: &str) -> NodeId {
    let id = job(s, name)["id"].as_str().unwrap().to_owned();
    s.fixture_job_named(&id).expect("the job's node")
}

/// The rows of the Parts list of the open job (empty when no list is open).
fn parts(s: &Session) -> Vec<Value> {
    fj(s)["parts"]["rows"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn part_names(s: &Session) -> Vec<String> {
    parts(s)
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_owned())
        .collect()
}

fn part_row(s: &Session, name: &str) -> Value {
    parts(s)
        .into_iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no part row {name}: {:?}", parts(s)))
}

/// The node of the part row `name` (its `select` id is the part's node).
fn part_node(s: &Session, name: &str) -> NodeId {
    let id = part_row(s, name)["select"].as_str().unwrap().to_owned();
    s.fixture_part_named(&id).expect("the part's node")
}

fn token_of(s: &Session, name: &str) -> String {
    part_row(s, name)["token"].as_str().unwrap().to_owned()
}

fn ticked_count(s: &Session) -> usize {
    parts(s).iter().filter(|r| r["ticked"] == true).count()
}

fn holder(s: &mut Session, slots: usize) -> NodeId {
    let h = s.add_fixture("holder").expect("a holder");
    for _ in 0..slots {
        s.add_slot(h).expect("a slot");
    }
    h
}

fn golden_session(name: &str, peer: u64) -> Session {
    let bytes = golden(name);
    let d = unpack(peer, &bytes).unwrap();
    let mut s = Session::open(peer, &bytes).unwrap();
    s.load_machine_profiles_json("[]");
    s.resize_viewport(1600.0, 1000.0);
    for page in d.pages() {
        if s.activate_page(&page.id) && view(&s) != Value::Null {
            return s;
        }
    }
    panic!("{name} has no laser page with an operations view");
}

fn golden(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("golden {name}: {e}"))
}

fn profile_record(json: &str) -> Value {
    fn find(v: &Value) -> Option<String> {
        match v {
            Value::String(s) if s.contains("format_version") => Some(s.clone()),
            Value::Array(a) => a.iter().find_map(find),
            Value::Object(o) => o.values().find_map(find),
            _ => None,
        }
    }
    let v: Value = serde_json::from_str(json).unwrap_or(Value::Null);
    serde_json::from_str(&find(&v).expect("a record was written")).unwrap()
}

fn view_profiles(s: &Session) -> Vec<Value> {
    let m: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    m["machine"]["profiles"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

fn view_notices(s: &Session) -> usize {
    let m: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    m["machine"]["library_notices"]
        .as_array()
        .map_or(0, Vec::len)
}

fn fixture_file(rel: &str) -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-laser-core/tests/fixtures/machine_profiles")
            .join(rel),
    )
    .unwrap()
}

// ---- criterion 1: the section -------------------------------------------------------------------

#[test]
fn ac1_the_section_is_empty_and_counts_zero_on_a_new_laser_page() {
    let s = one_page();
    assert_eq!(fj(&s)["count"], 0, "Fixture jobs (0)");
    assert_eq!(jobs(&s).len(), 0);
}

#[test]
fn ac1_the_section_counts_its_jobs() {
    let mut s = one_page();
    let g = s.add_fixture("grid").expect("a grid");
    s.select_fixture(g);
    s.add_fixture_job("cut").expect("a cut job");
    assert_eq!(fj(&s)["count"], 1);
}

// ---- criterion 2: the add buttons and their names ----------------------------------------------

#[test]
fn ac2_a_new_job_is_named_shown_with_output_off_selected_with_its_parts_open() {
    let mut s = one_page();
    let h = holder(&mut s, 2);
    assert!(s.select_fixture(h));
    let before = s.undo_len();
    let id = s.add_fixture_job("engrave").expect("an engrave job");
    assert_eq!(s.undo_len(), before + 1, "one undo step: Add fixture job");
    let j = job(&s, "Engrave 1");
    assert_eq!(j["show"], true, "Show on");
    assert_eq!(j["output"], false, "Output off, whatever the kind");
    assert_eq!(j["parts_ticked"], 0, "a new job has no tick");
    assert_eq!(j["parts_total"], 3, "board and two slots");
    assert_eq!(
        job_id(&s, "Engrave 1"),
        id,
        "the new job is the one returned"
    );
    assert_eq!(fj(&s)["parts"]["job"], j["id"], "its Parts list is open");
}

#[test]
fn ac2_names_take_the_lowest_free_number_of_their_kind_on_the_page() {
    let mut s = one_page();
    let a = holder(&mut s, 0);
    let b = holder(&mut s, 0);
    s.select_fixture(a);
    s.add_fixture_job("engrave").unwrap();
    s.select_fixture(b);
    s.add_fixture_job("engrave").unwrap();
    s.add_fixture_job("cut").unwrap();
    let names = job_names(&s);
    assert!(
        names.contains(&"Engrave 1".to_owned()) && names.contains(&"Engrave 2".to_owned()),
        "{names:?}"
    );
    assert!(names.contains(&"Cut 1".to_owned()), "{names:?}");
}

#[test]
fn ac2_no_job_is_added_without_a_selected_fixture() {
    let mut s = one_page();
    assert!(s.add_fixture_job("cut").is_none(), "no fixture selected");
    assert_eq!(jobs(&s).len(), 0);
    assert_eq!(fj(&s)["count"], 0);
}

// ---- criterion 3 and 4: order, rows, rename, delete, switches, margin ---------------------------

#[test]
fn ac3_jobs_of_one_fixture_move_inside_their_group_and_undo_restores() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    s.add_fixture_job("cut").unwrap();
    let second = s.add_fixture_job("score").unwrap();
    assert_eq!(job_names(&s), ["Cut 1", "Score 1"]);
    let before = s.undo_len();
    assert!(
        s.move_fixture_job(second, 0),
        "moved to the top of its group"
    );
    assert_eq!(s.undo_len(), before + 1, "one undo step: Move fixture job");
    assert_eq!(job_names(&s), ["Score 1", "Cut 1"]);
    s.undo();
    assert_eq!(
        job_names(&s),
        ["Cut 1", "Score 1"],
        "undo restores the order"
    );
}

#[test]
fn ac4_a_job_row_carries_kind_name_switches_and_arrow_position() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    s.add_fixture_job("engrave").unwrap();
    let j = job(&s, "Engrave 1");
    assert_eq!(j["kind"]["text"], "engrave");
    assert!(
        j["position"].is_number() && j["of"].is_number(),
        "the arrows know their place"
    );
    assert!(
        j["show"].is_boolean() && j["output"].is_boolean(),
        "Show and Output switches"
    );
}

#[test]
fn ac4_rename_is_one_undo_step() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    let before = s.undo_len();
    assert!(s.rename_fixture_job(id, "Coasters"));
    assert_eq!(s.undo_len(), before + 1, "one step: Rename fixture job");
    assert_eq!(job_names(&s), ["Coasters"]);
}

#[test]
fn ac4_delete_removes_the_job_and_one_undo_restores_it() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("cut").unwrap();
    assert!(s.delete_fixture_job(id));
    assert!(jobs(&s).is_empty(), "the job is gone");
    s.undo();
    assert_eq!(job_names(&s), ["Cut 1"], "one undo restores the job");
}

#[test]
fn ac4_show_and_output_are_one_undo_step_each() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    let before = s.undo_len();
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert_eq!(s.undo_len(), before + 1, "Output on: one step");
    assert_eq!(job(&s, "Engrave 1")["output"], true);
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Show, false));
    assert_eq!(s.undo_len(), before + 2, "Show off: one step");
    assert_eq!(job(&s, "Engrave 1")["show"], false);
}

#[test]
fn ac4_a_cut_out_job_has_no_margin_field_and_no_margin_text() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    s.add_fixture_job("cut_out").unwrap();
    let j = job(&s, "Cut-out 1");
    assert!(
        j.get("margin").is_none() && j.get("margin_mm").is_none(),
        "no margin on a job"
    );
    assert!(
        !texts(&s).iter().any(|t| t.contains("Margin")),
        "no margin text in the view"
    );
}

// ---- criterion 8, 9 and 11: the Parts list and the ticks ----------------------------------------

#[test]
fn ac8_the_parts_of_a_holder_are_the_board_and_one_row_per_slot_in_creation_order() {
    let mut s = one_page();
    let h = holder(&mut s, 2);
    s.select_fixture(h);
    s.add_fixture_job("engrave").unwrap();
    let names = part_names(&s);
    assert_eq!(names.len(), 3, "board and two slots: {names:?}");
    assert_eq!(names[0], "Board", "the board comes first");
    assert_eq!(
        part_row(&s, "Board")["ticked"],
        false,
        "nothing is ticked by default"
    );
}

#[test]
fn ac8_a_grid_lists_grid_lines_and_tapes_and_a_screw_head_and_thread() {
    let mut s = one_page();
    let g = s.add_fixture("grid").expect("a grid");
    s.select_fixture(g);
    s.add_fixture_job("cut").unwrap();
    assert_eq!(part_names(&s), ["Grid lines", "Tapes"]);
    let sc = s.add_fixture("screw").expect("a screw");
    s.select_fixture(sc);
    s.add_fixture_job("engrave").unwrap();
    assert_eq!(part_names(&s), ["Head", "Thread"]);
}

#[test]
fn ac9_a_tick_is_one_undo_step_and_untick_removes_it() {
    let mut s = one_page();
    let h = holder(&mut s, 1);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    let token = token_of(&s, "Board");
    let before = s.undo_len();
    assert!(s.set_fixture_job_part(id, &token, true));
    assert_eq!(s.undo_len(), before + 1, "one step: Assign to fixture job");
    assert_eq!(ticked_count(&s), 1);
    assert_eq!(job(&s, "Engrave 1")["parts_ticked"], 1);
    assert!(s.set_fixture_job_part(id, &token, false));
    assert_eq!(
        s.undo_len(),
        before + 2,
        "one step: Remove from fixture job"
    );
    assert_eq!(ticked_count(&s), 0);
}

#[test]
fn ac9_select_on_a_part_keeps_the_job_open_and_is_no_undo_step() {
    let mut s = one_page();
    let h = holder(&mut s, 1);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    let before = s.undo_len();
    let token = token_of(&s, "Board");
    assert!(s.select_fixture_job_part(id, &token));
    assert_eq!(s.undo_len(), before, "Select is not an undo step");
    assert_eq!(
        fj(&s)["parts"]["job"],
        job(&s, "Engrave 1")["id"],
        "the job stays open"
    );
}

#[test]
fn ac9_a_deleted_slot_drops_its_tick_in_the_same_step_and_undo_restores_it() {
    let mut s = one_page();
    let h = holder(&mut s, 2);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    let token = token_of(&s, "Slot 1");
    assert!(s.set_fixture_job_part(id, &token, true));
    assert_eq!(ticked_count(&s), 1);
    let slot0 = part_node(&s, "Slot 1");
    assert!(s.remove_slot(slot0));
    assert_eq!(
        part_names(&s).len(),
        2,
        "the deleted slot is no part any more"
    );
    assert_eq!(job(&s, "Engrave 1")["parts_ticked"], 0, "its tick is gone");
    s.undo();
    assert_eq!(
        ticked_count(&s),
        1,
        "one undo restores the slot and its tick"
    );
}

#[test]
fn ac9_a_renamed_slot_keeps_its_tick() {
    let mut s = one_page();
    let h = holder(&mut s, 1);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, &token_of(&s, "Slot 1"), true));
    let slot0 = part_node(&s, "Slot 1");
    assert!(s.rename_slot(slot0, "Coaster A"));
    assert_eq!(ticked_count(&s), 1, "a renamed slot keeps its tick");
}

#[test]
fn ac11_a_removed_fixture_takes_its_jobs_and_one_undo_restores_them_with_their_ticks() {
    let mut s = one_page();
    let h = holder(&mut s, 1);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.delete_fixture(h));
    assert!(jobs(&s).is_empty(), "the jobs are removed with the fixture");
    s.undo();
    assert_eq!(
        job_names(&s),
        ["Engrave 1"],
        "one undo restores the fixture and its job"
    );
    assert_eq!(job(&s, "Engrave 1")["parts_ticked"], 1, "and its tick");
}

// ---- criterion 13, 15, 17 and 19: the run rule and the summary ---------------------------------

#[test]
fn ac13_a_job_runs_with_output_on_and_one_ticked_part_and_the_summary_counts_jobs_not_parts() {
    let mut s = one_page();
    let h = holder(&mut s, 2);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(running(&s) == 0, "nothing runs yet");
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert!(running(&s) == 0, "Output on but no ticked part: no run");
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.set_fixture_job_part(id, "slot:0", true));
    assert!(s.set_fixture_job_part(id, "slot:1", true));
    assert!(
        running(&s) == 1,
        "one job runs, however many parts it ticks"
    );
    assert!(running(&s) > 0);
}

#[test]
fn ac13_show_does_not_decide_a_run() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Show, false));
    assert!(running(&s) == 1, "Show is view only");
}

#[test]
fn ac15_a_job_of_an_inactive_fixture_does_not_run_and_says_not_on_the_bed() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert_eq!(running(&s), 1);
    assert!(s.set_fixture_active(h, false));
    assert!(running(&s) == 0, "an inactive fixture runs nothing");
    assert_eq!(job(&s, "Engrave 1")["fixture_active"], false);
    assert_eq!(
        part_row(&s, "Board")["state"],
        "not_on_bed",
        "the part says not on the bed"
    );
    assert_eq!(
        job(&s, "Engrave 1")["output"],
        true,
        "its Output stays as it was"
    );
}

#[test]
fn ac17_two_running_jobs_read_as_two_fixture_jobs() {
    let mut s = one_page();
    for _ in 0..2 {
        let h = holder(&mut s, 0);
        s.select_fixture(h);
        let id = s.add_fixture_job("engrave").unwrap();
        assert!(s.set_fixture_job_part(id, "board", true));
        assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    }
    assert_eq!(running(&s), 2);
}

#[test]
fn ac19_a_cut_out_job_with_a_ticked_part_and_output_on_runs() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("cut_out").unwrap();
    assert_eq!(job(&s, "Cut-out 1")["kind"]["text"], "cut_out");
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert_eq!(running(&s), 1);
}

// ---- criterion 18: the look of a part (the view's state field) ---------------------------------

#[test]
fn c18_a_ticked_part_of_an_output_off_job_is_not_assigned_and_a_running_one_takes_its_kind() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    assert_eq!(
        part_row(&s, "Board")["state"],
        "output_off",
        "Not assigned look"
    );
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert_eq!(part_row(&s, "Board")["state"], "running");
    assert_eq!(part_row(&s, "Board")["kind"], "engrave");
}

#[test]
fn c18_with_show_off_the_parts_list_is_not_drawn_as_running() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Show, false));
    assert_ne!(
        part_row(&s, "Board")["state"],
        "running",
        "a part of a job with Show off is not drawn as running"
    );
}

// ---- criteria 21 and 22: Unknown kind and damaged values ---------------------------------------

#[test]
fn ac21_an_unknown_kind_job_is_listed_as_unknown_with_no_switch_no_arrows_no_parts() {
    let mut s = golden_session("fixture_jobs_unknown_kind_v41.curvyo", 9302);
    let unknown = jobs(&s)
        .into_iter()
        .find(|j| j["kind"].is_null())
        .expect("the unknown job is listed");
    assert!(unknown["kind"].is_null(), "listed as Unknown");
    assert_eq!(unknown["runs"], false, "an Unknown job runs nothing");
    let id = s
        .fixture_job_named(unknown["id"].as_str().unwrap())
        .expect("the unknown job's node");
    assert!(
        !s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true),
        "no Output switch"
    );
    assert!(
        !s.set_fixture_job_switch(id, FixtureJobSwitch::Show, false),
        "no Show switch"
    );
    assert!(
        s.rename_fixture_job(id, "Renamed"),
        "an Unknown job can be renamed"
    );
}

#[test]
fn ac21_an_unknown_kind_job_is_counted_in_no_summary_line() {
    let s = golden_session("fixture_jobs_unknown_kind_v41.curvyo", 9306);
    assert!(
        !texts(&s)
            .iter()
            .any(|t| t.contains("fixture job runs") || t.contains("fixture jobs run")),
        "counted in no summary line"
    );
}

#[test]
fn ac22_a_text_show_reads_on_and_a_text_output_reads_off() {
    let s = golden_session("fixture_jobs_damaged_v41.curvyo", 9304);
    let j = jobs(&s).into_iter().next().expect("the damaged job");
    assert_eq!(j["show"], true, "a text show reads on");
    assert_eq!(j["output"], false, "a text output reads off, never on");
}

#[test]
fn ac20_the_v41_golden_shows_its_jobs_with_their_switches_and_ticks() {
    let s = golden_session("fixture_jobs_v41.curvyo", 9305);
    assert!(
        job_names(&s).contains(&"Engrave + Outline 1".to_owned()),
        "{:?}",
        job_names(&s)
    );
    let eo = job(&s, "Engrave + Outline 1");
    assert_eq!(eo["output"], true);
    assert_eq!(eo["parts_ticked"], 3, "board and two slots ticked");
}

// ---- criteria 23 and 24: the profile record format 3 -------------------------------------------

#[test]
fn ac23_save_as_profile_writes_format_3_with_jobs_tokens_and_no_output_key() {
    let mut s = one_page();
    let h = holder(&mut s, 2);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave_outline").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.set_fixture_job_part(id, "slot:1", true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    let _ = s.save_machine_profile([7; 16], 50);
    let record = profile_record(&s.take_machine_writes_json());
    // Record format 4 since `0111` (ADR 0056 §7).
    assert_eq!(record["format_version"], 4, "any write is format 4");
    let job = &record["fixtures"][0]["jobs"][0];
    assert_eq!(job["kind"], "engrave_outline");
    assert_eq!(job["show"], true);
    assert_eq!(
        job["targets"],
        json!(["board", "slot:1"]),
        "tokens, not node ids"
    );
    assert!(
        job.get("output").is_none(),
        "the profile never holds Output"
    );
}

#[test]
fn ac23_reset_from_profile_copies_the_jobs_with_output_off_and_their_ticks() {
    let mut s = one_page();
    let h = holder(&mut s, 2);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "slot:1", true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    let _ = s.save_machine_profile([8; 16], 50);
    let profile = view_profiles(&s)
        .into_iter()
        .find(|p| p["builtin"] == false)
        .expect("the saved profile")["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(s.delete_fixture(h), "the holder is gone from the page");
    assert_eq!(jobs(&s).len(), 0);
    let before = s.undo_len();
    assert!(
        s.reset_machine_profile(),
        "the copy is taken from the profile"
    );
    assert_eq!(s.undo_len(), before + 1, "one undo step, as today");
    assert_eq!(
        job_names(&s),
        ["Engrave 1"],
        "the job came with the fixture"
    );
    assert_eq!(
        job(&s, "Engrave 1")["output"],
        false,
        "a copy starts with Output off"
    );
    assert_eq!(
        job(&s, "Engrave 1")["parts_ticked"],
        1,
        "its tick is mapped to the slot by index"
    );
    let _ = profile;
}

#[test]
fn ac24_a_format_2_record_with_a_jobs_key_damages_the_record() {
    let mut s = one_page();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": fixture_file("format2_jobs_key.json") }]).to_string(),
    );
    assert_eq!(view_profiles(&s).len(), 1, "only Generic laser is listed");
    assert_eq!(view_notices(&s), 1, "one notice");
}

#[test]
fn ac24_a_format_2_record_without_jobs_reads_and_is_listed() {
    let mut s = one_page();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": fixture_file("format2_fixtures.json") }]).to_string(),
    );
    assert_eq!(view_profiles(&s).len(), 2, "the format 2 set is listed");
    assert_eq!(view_notices(&s), 0, "no notice");
}

#[test]
fn ac24_a_format_3_record_with_an_output_key_is_damaged() {
    let mut s = one_page();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": fixture_file("format3_output_key.json") }]).to_string(),
    );
    assert_eq!(view_profiles(&s).len(), 1, "skipped");
    assert_eq!(view_notices(&s), 1, "an output key damages the record");
}

#[test]
fn ac24_a_format_3_record_with_a_bad_token_is_skipped_with_its_notice() {
    let mut s = one_page();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": fixture_file("format3_damaged_token.json") }]).to_string(),
    );
    assert_eq!(view_profiles(&s).len(), 1, "only Generic laser is listed");
    assert_eq!(view_notices(&s), 1, "one notice");
}

#[test]
fn ac24_a_format_3_fixture_jobs_record_reads_its_jobs_with_no_notice() {
    let mut s = one_page();
    let name = curvyo_laser_core::record_file_name("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f");
    s.load_machine_profiles_json(
        &json!([{ "name": name, "text": fixture_file("format3_fixture_jobs.json") }]).to_string(),
    );
    assert_eq!(view_profiles(&s).len(), 2, "the record is listed");
    assert_eq!(view_notices(&s), 0, "no notice for a valid record");
}

// ---- criteria 2, 4, 9: the undo step names of the fixed labels (ADR 0052 §8) -------------------

fn undo_name(s: &mut Session) -> Option<String> {
    s.history_target_name(curvyo_document_core::Direction::Undo)
}

#[test]
fn ac2_4_9_each_job_change_is_one_step_with_its_fixed_name_and_no_job_name() {
    let mut s = one_page();
    let h = holder(&mut s, 1);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert_eq!(undo_name(&mut s).as_deref(), Some("Add fixture job"));
    assert!(s.rename_fixture_job(id, "Coasters"));
    assert_eq!(undo_name(&mut s).as_deref(), Some("Rename fixture job"));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert_eq!(undo_name(&mut s).as_deref(), Some("Output on"));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, false));
    assert_eq!(undo_name(&mut s).as_deref(), Some("Output off"));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Show, false));
    assert_eq!(undo_name(&mut s).as_deref(), Some("Show off"));
    let token = token_of(&s, "Board");
    assert!(s.set_fixture_job_part(id, &token, true));
    assert_eq!(undo_name(&mut s).as_deref(), Some("Assign to fixture job"));
    assert!(s.set_fixture_job_part(id, &token, false));
    assert_eq!(
        undo_name(&mut s).as_deref(),
        Some("Remove from fixture job")
    );
    assert!(s.delete_fixture_job(id));
    assert_eq!(undo_name(&mut s).as_deref(), Some("Delete fixture job"));
}

#[test]
fn ac4_3_a_move_is_named_move_fixture_job() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    s.add_fixture_job("cut").unwrap();
    let second = s.add_fixture_job("score").unwrap();
    assert!(s.move_fixture_job(second, 0));
    assert_eq!(undo_name(&mut s).as_deref(), Some("Move fixture job"));
}

// ---- criterion 15: switching the fixture on again restores the run -----------------------------

#[test]
fn ac15_switching_the_fixture_back_on_restores_the_run() {
    let mut s = one_page();
    let h = holder(&mut s, 0);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert!(s.set_fixture_active(h, false));
    assert_eq!(running(&s), 0);
    assert!(s.set_fixture_active(h, true));
    assert_eq!(
        running(&s),
        1,
        "the run is restored, Output and ticks unchanged"
    );
    assert_eq!(job(&s, "Engrave 1")["output"], true);
}

// ---- criterion 12 and 20: the golden's section (dead key not a target) -------------------------

#[test]
fn ac12_20_the_golden_section_counts_three_jobs_and_the_dead_key_is_no_target() {
    let s = golden_session("fixture_jobs_v41.curvyo", 9307);
    assert_eq!(
        fj(&s)["count"],
        3,
        "a holder job, a grid job, an inactive holder's job"
    );
    for j in jobs(&s) {
        let ticked = j["parts_ticked"].as_u64().unwrap();
        let total = j["parts_total"].as_u64().unwrap();
        assert!(ticked <= total, "{j}: a dead key is no target");
    }
    assert_eq!(
        running(&s),
        1,
        "only the active holder's job with Output on runs"
    );
}

// ---- criterion 21 and 23: Differs from profile, Output not compared ----------------------------

fn machine_state(s: &Session) -> String {
    let m: Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    m["machine"]["state"].as_str().unwrap_or("").to_owned()
}

#[test]
fn ac21_an_unknown_kind_job_gives_no_differs_false_positive_after_a_save() {
    let mut s = golden_session("fixture_jobs_unknown_kind_v41.curvyo", 9308);
    let _ = s.save_machine_profile([9; 16], 50);
    assert_ne!(
        machine_state(&s),
        "differs",
        "an Unknown job does not make the set differ"
    );
}

#[test]
fn ac23_switching_output_does_not_make_the_set_differ() {
    let mut s = one_page();
    let h = holder(&mut s, 1);
    s.select_fixture(h);
    let id = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(id, "board", true));
    let _ = s.save_machine_profile([5; 16], 50);
    assert_ne!(machine_state(&s), "differs", "saved as it is");
    assert!(s.set_fixture_job_switch(id, FixtureJobSwitch::Output, true));
    assert_ne!(machine_state(&s), "differs", "Output is not compared");
    assert!(s.set_fixture_job_part(id, "slot:0", true));
    assert_eq!(machine_state(&s), "differs", "a changed tick does differ");
}

// ---- criterion 22: the damage notice counts the job's damaged values ---------------------------

#[test]
fn ac22_the_damage_notice_counts_all_four_damaged_values_of_the_job() {
    let s = golden_session("fixture_jobs_damaged_v41.curvyo", 9309);
    let notices = s.fixtures_view().expect("the fixtures view").damage_notices;
    assert_eq!(notices.len(), 1, "one notice per fixture: {notices:?}");
    assert!(
        notices[0].starts_with("4 values damaged on"),
        "a text show, a text output, a false target and a .tapes on a holder: {notices:?}"
    );
}
