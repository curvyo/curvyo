//! The fixture job commands of ADR 0052 §4 (`specs/0103-fixture-jobs` criteria 2 to 5, 7, 9 to
//! 11, 23): one step per command with its own label, Output off on a new job, the ticks as keys
//! on the job, the slot sweep, and the jobs of Pick and Reset from profile.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Affine, Document, DocumentSize, FixtureAdd, FixtureJobError, FixtureJobSwitch, FixtureParams,
    FixturePartKey, JobRecord, JobTarget, NewFixture, NewOperation, NewOutputPage, NodeId,
    PageCorner, ProfileChange, StepRef, Vec2,
};

fn params(value: serde_json::Value) -> FixtureParams {
    serde_json::from_value(value).unwrap()
}

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(600.0, 400.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn holder(d: &Document, mfg: NodeId) -> NodeId {
    d.add_fixture(
        mfg,
        &NewFixture {
            add: FixtureAdd::Holder,
            kind: "curvyo.laser/holder",
            label: "Holder",
            params: &params(serde_json::json!({"width_mm": 100.0, "height_mm": 60.0})),
            matrix: Some(Affine::translation(Vec2::new(250.0, 170.0))),
        },
    )
    .unwrap()
}

fn slot(d: &Document, holder: NodeId) -> NodeId {
    let values = params(serde_json::json!({
        "shape": "rect", "cx_mm": 50.0, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 20.0, "d_mm": 20.0
    }));
    d.add_slot(holder, "Slot", &values).unwrap()
}

fn part(node: NodeId) -> FixturePartKey {
    FixturePartKey { node, suffix: None }
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

#[test]
fn a_new_job_has_show_on_output_off_no_tick_and_one_step() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board = holder(&d, mfg);
    let steps = d.own_step_count();
    let job = d.add_fixture_job(board, "engrave", "Engrave 1").unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), "add_fixture_job");
    let read = d.fixture_job(job).unwrap();
    assert!(read.show && !read.output && read.targets.is_empty());
    assert_eq!(read.name, "Engrave 1");
    let fixture = d.fixture(board).unwrap();
    assert!(fixture.slots.is_empty(), "a job is no slot");
    assert_eq!(fixture.damaged, 0, "a job is no damaged slot");
}

#[test]
fn each_command_is_one_step_with_its_label() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board = holder(&d, mfg);
    let a = d.add_fixture_job(board, "cut", "Cut 1").unwrap();
    let b = d.add_fixture_job(board, "cut", "Cut 2").unwrap();
    assert!(d.rename_fixture_job(a, "Outline").unwrap());
    assert_eq!(last_label(&d), "rename_fixture_job");
    assert!(!d.rename_fixture_job(a, "Outline").unwrap(), "unchanged");
    assert!(!d.rename_fixture_job(a, "   ").unwrap(), "empty reverts");
    for (which, on, label) in [
        (FixtureJobSwitch::Show, false, "set_fixture_job_show_off"),
        (FixtureJobSwitch::Show, true, "set_fixture_job_show_on"),
        (FixtureJobSwitch::Output, true, "set_fixture_job_output_on"),
        (
            FixtureJobSwitch::Output,
            false,
            "set_fixture_job_output_off",
        ),
    ] {
        assert!(d.set_fixture_job_switch(a, which, on).unwrap());
        assert_eq!(last_label(&d), label);
        assert!(!d.set_fixture_job_switch(a, which, on).unwrap());
    }
    assert!(d.move_fixture_job(b, 0).unwrap());
    assert_eq!(last_label(&d), "move_fixture_job");
    assert!(!d.move_fixture_job(b, 0).unwrap());
    let order: Vec<NodeId> = d
        .fixture(board)
        .unwrap()
        .jobs
        .iter()
        .map(|j| j.id)
        .collect();
    assert_eq!(order, [b, a]);
    assert!(d.set_fixture_job_targets(a, &[part(board)], true).unwrap());
    assert_eq!(last_label(&d), "assign_fixture_target");
    assert!(!d.set_fixture_job_targets(a, &[part(board)], true).unwrap());
    assert!(d.set_fixture_job_targets(a, &[part(board)], false).unwrap());
    assert_eq!(last_label(&d), "remove_fixture_target");
    d.delete_fixture_job(a).unwrap();
    assert_eq!(last_label(&d), "delete_fixture_job");
    assert_eq!(d.fixture(board).unwrap().jobs.len(), 1);
}

#[test]
fn job_commands_refuse_work_operations_slots_fixtures_and_foreign_parts() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board = holder(&d, mfg);
    let other = holder(&d, mfg);
    let pocket = slot(&d, board);
    let job = d.add_fixture_job(board, "cut", "Cut 1").unwrap();
    let op = d
        .add_operation(
            mfg,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &[],
        )
        .unwrap();
    for id in [op, pocket, board] {
        assert_eq!(
            d.rename_fixture_job(id, "x"),
            Err(FixtureJobError::NoSuchJob)
        );
    }
    assert_eq!(
        d.add_fixture_job(pocket, "cut", "Cut 1"),
        Err(FixtureJobError::NoSuchFixture)
    );
    assert_eq!(
        d.add_fixture_job(mfg, "cut", "Cut 1"),
        Err(FixtureJobError::NoSuchFixture)
    );
    assert_eq!(
        d.set_fixture_job_targets(job, &[part(other)], true),
        Err(FixtureJobError::NoSuchPart),
        "another fixture's board"
    );
    let odd = FixturePartKey {
        node: board,
        suffix: Some("wing".to_owned()),
    };
    assert_eq!(
        d.set_fixture_job_targets(job, &[odd], true),
        Err(FixtureJobError::NoSuchPart)
    );
    assert!(
        d.rename_fixture(job, "x").is_err(),
        "a fixture command refuses a job"
    );
    assert!(d.remove_slot(job).is_err(), "a slot command refuses a job");
}

#[test]
fn removing_a_slot_drops_its_ticks_in_the_same_step_and_undo_restores_them() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board = holder(&d, mfg);
    let first = slot(&d, board);
    let second = slot(&d, board);
    let job = d.add_fixture_job(board, "engrave", "Engrave 1").unwrap();
    let ticks = [part(board), part(first), part(second)];
    d.set_fixture_job_targets(job, &ticks, true).unwrap();
    let before = d.fixture_job(job).unwrap();
    let steps = d.own_step_count();
    d.remove_slot(first).unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    let after = d.fixture_job(job).unwrap();
    let nodes: Vec<NodeId> = after.targets.iter().map(|t| t.key.node).collect();
    assert_eq!(nodes, [board, second]);
    assert_eq!(
        after.targets[1].target.slot,
        Some(0),
        "the place follows the slots"
    );
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(d.fixture_job(job).unwrap(), before);
}

#[test]
fn removing_a_fixture_takes_its_jobs_and_undo_restores_them_exactly() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board = holder(&d, mfg);
    let job = d.add_fixture_job(board, "cut", "Cut 1").unwrap();
    d.set_fixture_job_targets(job, &[part(board)], true)
        .unwrap();
    let before = d.fixture(board).unwrap();
    d.delete_fixture(board).unwrap();
    assert!(d.fixture_job(job).is_none());
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(d.fixture(board).unwrap(), before);
}

#[test]
fn pick_copies_the_jobs_with_output_off_fresh_ids_and_ticks_by_slot_index() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board = holder(&d, mfg);
    slot(&d, board);
    slot(&d, board);
    let job = d.add_fixture_job(board, "engrave_outline", "E 1").unwrap();
    let second = d.fixture(board).unwrap().slots[1].id;
    d.set_fixture_job_targets(job, &[part(board), part(second)], true)
        .unwrap();
    d.set_fixture_job_switch(job, FixtureJobSwitch::Output, true)
        .unwrap();
    let mut record = d.fixture(board).unwrap().record();
    assert_eq!(
        record.jobs,
        [JobRecord {
            kind: "engrave_outline".to_owned(),
            name: "E 1".to_owned(),
            show: true,
            targets: vec![
                JobTarget {
                    slot: None,
                    suffix: None
                },
                JobTarget {
                    slot: Some(1),
                    suffix: None
                }
            ],
        }],
        "no Output in a record"
    );
    record.name = "Copied".to_owned();
    let copy = params(serde_json::json!({"id": "generic-laser", "name": "Generic laser"}));
    let steps = d.own_step_count();
    assert!(
        d.set_manufacturing_profile_and_fixtures(
            mfg,
            &copy,
            DocumentSize::from_mm(600.0, 400.0),
            PageCorner::default(),
            ProfileChange::Pick,
            std::slice::from_ref(&record),
            &curvyo_document_core::PageContents::new(),
        )
        .unwrap()
    );
    assert_eq!(d.own_step_count(), steps + 1, "the same one step");
    let fixture = d.manufacturing(mfg).unwrap().fixtures.remove(0);
    let copied = &fixture.jobs[0];
    assert_ne!(copied.id, job, "a fresh id");
    assert!(!copied.output, "a copy starts with Output off");
    assert_eq!(copied.targets[0].key.node, fixture.id);
    assert_eq!(copied.targets[1].key.node, fixture.slots[1].id);
    assert_eq!(fixture.record(), record);
}
