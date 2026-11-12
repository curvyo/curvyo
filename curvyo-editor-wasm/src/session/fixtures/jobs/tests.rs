//! The fixture jobs at session level (`specs/0103-fixture-jobs` criteria 2 to 5, 8, 9, 12, 13,
//! 15, 17, 18, 21 to 23; UX notes J6, J15).
#![allow(clippy::unwrap_used)]

use curvyo_document_core::{FixtureJobSwitch, FixturePartKey, NodeId};
use curvyo_render_core::{OP_CUT, OP_ENGRAVE_HATCH, OP_LANE_LINE};

use super::super::super::Session;
use super::super::tests::{laser, last_label};

fn jobs(s: &Session) -> super::FixtureJobsView {
    s.fixture_jobs_view().expect("a laser page")
}

/// A selected holder with two slots; the holder and the slot ids.
fn holder_with_slots(s: &mut Session) -> (NodeId, [NodeId; 2]) {
    let holder = s.add_fixture("holder").unwrap();
    let a = s.add_slot(holder).unwrap();
    let b = s.add_slot(holder).unwrap();
    assert!(s.select_fixture(holder));
    (holder, [a, b])
}

/// The page's job of the document, read raw (no rule of this module).
fn stored(s: &Session, id: NodeId) -> curvyo_document_core::FixtureJobSnapshot {
    s.document.fixture_job(id).unwrap()
}

#[test]
fn criterion_2_a_kind_button_adds_a_job_to_the_one_selected_fixture() {
    let mut s = laser();
    assert!(
        jobs(&s).add_to.is_none(),
        "no fixture: the buttons are hidden"
    );
    assert!(s.add_fixture_job("cut").is_none());
    let (holder, _) = holder_with_slots(&mut s);
    assert_eq!(jobs(&s).add_to.unwrap().name, "Holder 1");
    let first = s.add_fixture_job("cut").unwrap();
    assert_eq!(last_label(&s), "add_fixture_job");
    let second = s.add_fixture_job("cut").unwrap();
    let engrave = s.add_fixture_job("engrave_outline").unwrap();
    assert!(s.add_fixture_job("laser_burn").is_none(), "no such kind");
    let names: Vec<String> = [first, second, engrave]
        .iter()
        .map(|&id| stored(&s, id).name)
        .collect();
    assert_eq!(names, ["Cut 1", "Cut 2", "Engrave + Outline 1"]);
    let job = stored(&s, first);
    assert!(
        job.show && !job.output && job.targets.is_empty(),
        "criterion 5"
    );
    assert_eq!(
        s.selected_fixture_job(),
        Some(engrave),
        "the new job is selected"
    );
    let view = jobs(&s);
    assert_eq!(view.count, 3);
    assert_eq!(view.groups.len(), 1);
    assert_eq!(view.groups[0].fixture, holder.full_id());
    let parts = view.parts.unwrap();
    let names: Vec<&str> = parts.rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Board", "Slot 1", "Slot 2"]);
    assert!(
        parts
            .rows
            .iter()
            .all(|r| !r.ticked && r.state == "unticked")
    );
    assert!(s.delete_fixture_job(first));
    assert!(s.select_fixture(holder));
    let again = s.add_fixture_job("cut").unwrap();
    assert_eq!(stored(&s, again).name, "Cut 1", "the lowest free number");
}

#[test]
fn criteria_3_and_4_rename_switch_move_and_delete_are_one_step_each() {
    let mut s = laser();
    holder_with_slots(&mut s);
    let a = s.add_fixture_job("cut").unwrap();
    let b = s.add_fixture_job("score").unwrap();
    assert!(s.rename_fixture_job(a, "Board outline"));
    assert_eq!(last_label(&s), "rename_fixture_job");
    assert!(s.set_fixture_job_switch(a, FixtureJobSwitch::Output, true));
    assert_eq!(last_label(&s), "set_fixture_job_output_on");
    assert!(
        !s.set_fixture_job_switch(a, FixtureJobSwitch::Output, true),
        "no change"
    );
    assert!(s.set_fixture_job_switch(a, FixtureJobSwitch::Show, false));
    assert_eq!(last_label(&s), "set_fixture_job_show_off");
    assert!(s.move_fixture_job(b, 0));
    assert_eq!(last_label(&s), "move_fixture_job");
    let rows = &jobs(&s).groups[0].jobs;
    assert_eq!(rows[0].id, b.full_id());
    assert_eq!((rows[1].position, rows[1].of), (1, 2));
    assert!(s.delete_fixture_job(a));
    assert_eq!(last_label(&s), "delete_fixture_job");
    assert_eq!(jobs(&s).count, 1);
    s.undo();
    assert_eq!(stored(&s, a).name, "Board outline", "undo revives it");
}

#[test]
fn criterion_9_ticks_write_one_key_each_and_select_keeps_the_job() {
    let mut s = laser();
    let (holder, [slot, _]) = holder_with_slots(&mut s);
    let job = s.add_fixture_job("engrave").unwrap();
    assert!(s.set_fixture_job_part(job, "slot:0", true));
    assert_eq!(last_label(&s), "assign_fixture_target");
    assert!(s.set_fixture_job_part(job, "board", true));
    assert!(
        !s.set_fixture_job_part(job, "grid.lines", true),
        "no part of a holder"
    );
    assert!(!s.set_fixture_job_part(job, "slot:2", true), "no such slot");
    let keys: Vec<FixturePartKey> = stored(&s, job).targets.into_iter().map(|t| t.key).collect();
    assert_eq!(
        keys,
        [
            FixturePartKey {
                node: holder,
                suffix: None
            },
            FixturePartKey {
                node: slot,
                suffix: None
            }
        ]
    );
    let parts = jobs(&s).parts.unwrap();
    assert_eq!(parts.rows[1].select, slot.full_id());
    assert_eq!(parts.rows[1].state, "output_off");
    assert!(s.set_fixture_job_part(job, "board", false));
    assert_eq!(last_label(&s), "remove_fixture_target");
    assert!(s.select_fixture_job_part(job, "slot:0"));
    assert_eq!(s.selected_slot(), Some(slot));
    assert_eq!(s.selected_fixture_job(), Some(job), "Select keeps the job");
}

#[test]
fn criteria_13_15_and_17_a_job_runs_only_on_an_active_fixture_with_output_and_a_tick() {
    let mut s = laser();
    let (holder, _) = holder_with_slots(&mut s);
    let job = s.add_fixture_job("cut").unwrap();
    assert_eq!(s.running_fixture_jobs(), 0, "Output off");
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    assert_eq!(s.running_fixture_jobs(), 0, "nothing ticked");
    assert!(s.set_fixture_job_part(job, "slot:1", true));
    assert!(s.set_fixture_job_part(job, "slot:0", true));
    assert_eq!(s.running_fixture_jobs(), 1, "jobs, not parts");
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Show, false));
    assert_eq!(s.running_fixture_jobs(), 1, "Show does not count");
    let view = jobs(&s);
    assert_eq!(view.running_fixtures, [holder.full_id()]);
    assert!(view.groups[0].jobs[0].runs);
    assert!(s.set_fixture_active(holder, false));
    let view = jobs(&s);
    assert_eq!(view.running, 0, "a job of an inactive fixture does not run");
    assert!(!view.groups[0].jobs[0].fixture_active);
    assert!(stored(&s, job).output, "its Output stays as it is");
}

#[test]
fn criterion_21_an_unknown_job_can_only_be_renamed_or_deleted() {
    let mut s = laser();
    let (holder, _) = holder_with_slots(&mut s);
    let unknown = s
        .document
        .add_fixture_job(holder, "laser_burn", "Burn 1")
        .unwrap();
    assert!(!s.select_fixture_job(unknown));
    assert!(!s.set_fixture_job_switch(unknown, FixtureJobSwitch::Output, true));
    assert!(!s.set_fixture_job_switch(unknown, FixtureJobSwitch::Show, false));
    assert!(!s.move_fixture_job(unknown, 0));
    assert!(!s.set_fixture_job_part(unknown, "board", true));
    let row = &jobs(&s).groups[0].jobs[0];
    assert!(row.kind.is_none() && !row.runs);
    assert_eq!(jobs(&s).count, 1, "counted in the header");
    assert!(s.rename_fixture_job(unknown, "Old burn"));
    assert!(s.delete_fixture_job(unknown));
}

#[test]
fn criterion_22_a_tick_of_no_part_of_the_kind_counts_as_damage() {
    let mut s = laser();
    let (holder, _) = holder_with_slots(&mut s);
    let job = s.add_fixture_job("cut").unwrap();
    let tapes = FixturePartKey {
        node: holder,
        suffix: Some("tapes".to_owned()),
    };
    assert!(
        s.document
            .set_fixture_job_targets(job, &[tapes], true)
            .unwrap()
    );
    let notices = s.fixtures_view().unwrap().damage_notices;
    assert_eq!(notices, ["1 value damaged on Holder 1."]);
    assert_eq!(jobs(&s).groups[0].jobs[0].parts_ticked, 0, "not a target");
}

#[test]
fn criterion_23_an_unknown_kind_job_does_not_make_the_profile_differ() {
    let mut s = laser();
    let (holder, _) = holder_with_slots(&mut s);
    let job = s.add_fixture_job("cut").unwrap();
    assert!(s.set_fixture_job_part(job, "slot:0", true));
    s.document
        .add_fixture_job(holder, "laser_burn", "Burn 1")
        .unwrap();
    let _ = s.save_machine_profile([7; 16], 1);
    let machine = |s: &Session| -> serde_json::Value {
        serde_json::from_str::<serde_json::Value>(&s.manufacturing_view_json()).unwrap()["machine"]
            .clone()
    };
    assert!(
        machine(&s)["state"].is_null(),
        "the write left the Unknown job out"
    );
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    assert!(machine(&s)["state"].is_null(), "Output is not compared");
    assert!(s.set_fixture_job_part(job, "slot:1", true));
    assert_eq!(machine(&s)["state"], "differs", "the ticks are");
}

#[test]
fn ux_j15_a_fixture_row_with_jobs_lists_them_in_the_layers_tree() {
    let mut s = laser();
    let (holder, _) = holder_with_slots(&mut s);
    let job = s.add_fixture_job("score").unwrap();
    let view: serde_json::Value = serde_json::from_str(&s.layers_view_json(0, 60)).unwrap();
    let row = view["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["fixture"]["id"] == holder.full_id())
        .unwrap()
        .clone();
    assert_eq!(
        row["fixture"]["jobs"],
        serde_json::json!([{"id": job.full_id(), "name": "Score 1", "kind": "score",
            "output": false}])
    );
}

#[test]
fn a_job_command_touches_no_drawing_node() {
    let mut s = laser();
    holder_with_slots(&mut s);
    let _ = s
        .document
        .create_rect(curvyo_document_core::RectBounds::from_corners(
            curvyo_document_core::Point::new(10.0, 10.0),
            curvyo_document_core::Point::new(20.0, 20.0),
        ));
    let job = s.add_fixture_job("cut").unwrap();
    let objects = |s: &Session| -> Vec<_> {
        s.document
            .read_nodes()
            .into_iter()
            .filter_map(|n| n.object().cloned())
            .collect()
    };
    let before = objects(&s);
    assert!(s.set_fixture_job_part(job, "board", true));
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    assert_eq!(objects(&s), before);
}

/// The colours of the parts' looks drawn on the active page.
fn look_colours(s: &Session) -> Vec<curvyo_render_core::RgbaColor> {
    let fixtures = s.active_fixtures().unwrap();
    s.fixture_job_triangles(&fixtures)
        .iter()
        .map(|v| v.color)
        .collect()
}

#[test]
fn criterion_18_a_running_part_takes_its_kind_look_and_output_off_the_not_assigned_look() {
    let mut s = laser();
    let (holder, _) = holder_with_slots(&mut s);
    let job = s.add_fixture_job("engrave").unwrap();
    assert!(look_colours(&s).is_empty(), "nothing ticked: fixture ink");
    assert!(s.set_fixture_job_part(job, "board", true));
    let off = look_colours(&s);
    assert!(
        off.contains(&OP_LANE_LINE) && !off.contains(&OP_ENGRAVE_HATCH),
        "Output off"
    );
    let fixtures = s.active_fixtures().unwrap();
    assert_eq!(s.engraved_boards(&fixtures), Vec::<NodeId>::new());
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Output, true));
    assert!(look_colours(&s).contains(&OP_ENGRAVE_HATCH));
    let fixtures = s.active_fixtures().unwrap();
    assert_eq!(
        s.engraved_boards(&fixtures),
        [holder],
        "J6: the canvas ground"
    );
    let board = s.fixture_look(&fixtures).boards[0].clone();
    assert!(board.engraved);
    let cut = s.add_fixture_job("cut").unwrap();
    assert!(s.set_fixture_job_switch(cut, FixtureJobSwitch::Output, true));
    assert!(s.set_fixture_job_part(cut, "slot:0", true));
    assert!(look_colours(&s).contains(&OP_CUT));
    let page = s.active_page().id;
    assert!(s.toggle_operations_view(&page));
    assert!(
        look_colours(&s).is_empty(),
        "View: Drawn keeps the fixture ink"
    );
    assert!(s.toggle_operations_view(&page));
    assert!(s.set_fixture_job_switch(job, FixtureJobSwitch::Show, false));
    assert!(s.set_fixture_job_switch(cut, FixtureJobSwitch::Show, false));
    assert!(look_colours(&s).is_empty(), "Show off: fixture ink");
}
