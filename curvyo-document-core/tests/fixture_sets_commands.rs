//! The set, job fixture, lock, role job and Apply commands of `specs/0113-fixture-sets-and-pins`
//! (criteria 3 to 8, 13, 14, 16, 17, 27, 31, 32, 35, 37; ADR 0059 §1 to §3, §5, §6, §8).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::cast_precision_loss,
    clippy::type_complexity,
    clippy::similar_names
)]

use curvyo_document_core::{
    Affine, Document, DocumentSize, FixtureAdd, FixtureJobError, FixtureJobSwitch, FixtureParams,
    FixturePartKey, JobRole, MAX_FIXTURE_SETS, MAX_JOB_FIXTURES, ManufacturingError, NewFixture,
    NewOutputPage, NodeId, StepRef, Vec2,
};

const HOLDER: &str = "curvyo.laser/holder";
const SCREW: &str = "curvyo.laser/screw";
const PIN: &str = "curvyo.laser/pin";

/// A fresh profile id per call (the session makes them with `new_record_id`).
fn next_uid() -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(1);
    format!(
        "00000000-0000-4000-8000-{:012x}",
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

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

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

fn new<'a>(kind: &'a str, label: &'a str, values: &'a FixtureParams, x: f64) -> NewFixture<'a> {
    NewFixture {
        add: FixtureAdd::Holder,
        kind,
        label,
        params: values,
        matrix: Some(Affine::translation(Vec2::new(x, 50.0))),
    }
}

fn holder_values() -> FixtureParams {
    params(serde_json::json!({"width_mm": 100.0, "height_mm": 60.0}))
}

fn pin_values() -> FixtureParams {
    params(serde_json::json!({"diameter_mm": 6.0, "columns": 2, "rows": 2}))
}

fn screw_values() -> FixtureParams {
    params(serde_json::json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 10.0}))
}

fn job_pin(d: &Document, mfg: NodeId, x: f64) -> NodeId {
    d.add_job_fixture(mfg, &new(PIN, "Pin", &pin_values(), x))
        .unwrap()
}

// ---- sets (criteria 3 to 6, 17) ----------------------------------------------------------------

#[test]
fn plus_set_adds_the_lowest_free_number_on_this_page_in_one_step() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let first = d.add_fixture_set(mfg, &next_uid()).unwrap();
    assert_eq!(last_label(&d), "add_fixture_set");
    let second = d.add_fixture_set(mfg, &next_uid()).unwrap();
    d.rename_fixture_set(first, "Bench").unwrap();
    let third = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let read = d.manufacturing(mfg).unwrap();
    let names: Vec<&str> = read.sets.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, ["Bench", "Set 2", "Set 1"], "Set 1 is free again");
    assert_eq!(
        read.sets.iter().map(|s| s.id).collect::<Vec<_>>(),
        [first, second, third]
    );
    assert!(read.sets.iter().all(|s| s.on_page && s.damaged == 0));
    assert!(read.fixtures.is_empty(), "a new set is empty");
}

#[test]
fn a_page_takes_sixteen_sets_from_plus_set() {
    let d = Document::new(1);
    let mfg = laser(&d);
    for _ in 0..MAX_FIXTURE_SETS {
        d.add_fixture_set(mfg, &next_uid()).unwrap();
    }
    assert!(matches!(
        d.add_fixture_set(mfg, &next_uid()),
        Err(ManufacturingError::TooManySets)
    ));
}

#[test]
fn a_set_is_renamed_up_to_forty_characters_and_names_need_not_be_unique() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let a = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let b = d.add_fixture_set(mfg, &next_uid()).unwrap();
    assert!(d.rename_fixture_set(a, &"x".repeat(60)).unwrap());
    assert_eq!(last_label(&d), "rename_fixture_set");
    assert!(d.rename_fixture_set(b, &"x".repeat(40)).unwrap());
    let read = d.manufacturing(mfg).unwrap();
    assert_eq!(read.sets[0].name, "x".repeat(40));
    assert_eq!(read.sets[0].name, read.sets[1].name);
    assert!(!d.rename_fixture_set(a, "   ").unwrap(), "nothing left");
    assert!(
        !d.rename_fixture_set(a, &"x".repeat(40)).unwrap(),
        "unchanged"
    );
}

#[test]
fn removing_a_set_takes_its_fixtures_and_jobs_and_undo_restores_them_exactly() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let screw = d
        .add_set_fixture(set, &new(SCREW, "Screw", &screw_values(), 100.0))
        .unwrap();
    let kept = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let job = job_pin(&d, mfg, 300.0);
    let before = d.manufacturing(mfg).unwrap();
    d.remove_fixture_set(set).unwrap();
    assert_eq!(last_label(&d), "remove_fixture_set");
    let after = d.manufacturing(mfg).unwrap();
    assert_eq!(after.sets.len(), 1);
    assert_eq!(after.sets[0].id, kept);
    assert_eq!(after.fixtures.len(), 1);
    assert_eq!(after.fixtures[0].id, job, "nothing else changes");
    assert!(d.fixture(screw).is_none());
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(d.manufacturing(mfg).unwrap(), before);
}

#[test]
fn a_set_moves_among_the_sets_with_its_fixtures() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let a = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let b = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let c = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let holder = d
        .add_set_fixture(a, &new(HOLDER, "Holder", &holder_values(), 100.0))
        .unwrap();
    assert!(!d.move_fixture_set(a, 0).unwrap(), "already first");
    assert!(d.move_fixture_set(a, 9).unwrap());
    assert_eq!(last_label(&d), "move_fixture_set");
    let read = d.manufacturing(mfg).unwrap();
    assert_eq!(
        read.sets.iter().map(|s| s.id).collect::<Vec<_>>(),
        [b, c, a]
    );
    assert_eq!(read.fixtures[0].id, holder);
    assert_eq!(read.fixtures[0].set, Some(a));
    assert!(d.move_fixture_set(a, 1).unwrap());
    let order: Vec<NodeId> = d
        .manufacturing(mfg)
        .unwrap()
        .sets
        .iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(order, [b, a, c]);
}

#[test]
fn on_this_page_switches_in_its_own_steps_and_keeps_the_fixtures_own_checks() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let holder = d
        .add_set_fixture(set, &new(HOLDER, "Holder", &holder_values(), 100.0))
        .unwrap();
    assert!(!d.set_fixture_set_on_page(set, true).unwrap(), "already on");
    assert!(d.set_fixture_set_on_page(set, false).unwrap());
    assert_eq!(last_label(&d), "set_fixture_set_off_page");
    let read = d.manufacturing(mfg).unwrap();
    assert!(!read.sets[0].on_page);
    assert_eq!(read.set_on_page(&read.fixtures[0]), Some(false));
    assert!(read.fixtures[0].active, "its own check stays on");
    d.set_fixture_active(holder, false).unwrap();
    assert!(d.set_fixture_set_on_page(set, true).unwrap());
    assert_eq!(last_label(&d), "set_fixture_set_on_page");
    assert!(!d.fixture(holder).unwrap().active);
}

#[test]
fn a_set_id_is_refused_by_fixture_commands_and_a_fixture_id_by_set_commands() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let pin = job_pin(&d, mfg, 100.0);
    assert!(matches!(
        d.rename_fixture(set, "Nope"),
        Err(ManufacturingError::NoSuchFixture)
    ));
    assert!(matches!(
        d.delete_fixture(set),
        Err(ManufacturingError::NoSuchFixture)
    ));
    assert!(matches!(
        d.set_fixture_active(set, false),
        Err(ManufacturingError::NoSuchFixture)
    ));
    assert!(matches!(
        d.add_fixture_job(set, "cut", "Cut"),
        Err(FixtureJobError::NoSuchFixture)
    ));
    assert!(d.fixture(set).is_none());
    assert!(matches!(
        d.remove_fixture_set(pin),
        Err(ManufacturingError::NoSuchSet)
    ));
    assert!(matches!(
        d.add_set_fixture(mfg, &new(HOLDER, "Holder", &holder_values(), 1.0)),
        Err(ManufacturingError::NoSuchSet)
    ));
}

// ---- fixtures in sets and job fixtures (criteria 7, 8, 13, 16, 17) ----------------------------

#[test]
fn every_fixture_command_reaches_a_fixture_in_a_set() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let holder = d
        .add_set_fixture(set, &new(HOLDER, "Holder", &holder_values(), 100.0))
        .unwrap();
    assert_eq!(last_label(&d), "add_fixture");
    let read = d.fixture(holder).unwrap();
    assert_eq!(read.name, "Holder 1");
    assert_eq!(read.set, Some(set));
    assert!(read.active && read.visible && !read.locked);
    assert!(d.rename_fixture(holder, "Board").unwrap());
    assert!(d.set_fixture_active(holder, false).unwrap());
    assert!(d.set_fixture_visible(holder, false).unwrap());
    let slot_values = params(serde_json::json!({
        "shape": "rect", "cx_mm": 50.0, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 20.0, "d_mm": 20.0
    }));
    let slot = d.add_slot(holder, "Slot", &slot_values).unwrap();
    let job = d.add_fixture_job(holder, "cut", "Cut 1").unwrap();
    let tick = FixturePartKey {
        node: slot,
        suffix: None,
    };
    assert!(d.set_fixture_job_targets(job, &[tick], true).unwrap());
    let read = d.fixture(holder).unwrap();
    assert_eq!(read.slots.len(), 1);
    assert_eq!(read.jobs[0].targets[0].target.slot, Some(0));
    assert!(d.is_in_manufacturing(job), "a job at depth 3 is skipped");
    d.delete_fixture(holder).unwrap();
    assert!(d.manufacturing(mfg).unwrap().fixtures.is_empty());
}

#[test]
fn a_job_fixture_takes_the_lowest_free_number_of_its_kind_among_all_the_pages_fixtures() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    d.add_set_fixture(set, &new(PIN, "Pin", &pin_values(), 10.0))
        .unwrap();
    let first = job_pin(&d, mfg, 100.0);
    assert_eq!(last_label(&d), "add_job_fixture");
    let second = job_pin(&d, mfg, 200.0);
    let holder = d
        .add_job_fixture(mfg, &new(HOLDER, "Holder", &holder_values(), 300.0))
        .unwrap();
    let read = d.manufacturing(mfg).unwrap();
    assert_eq!(read.fixtures[0].name, "Pin 1", "the set's pin");
    let jobs: Vec<(&str, NodeId)> = read
        .job_fixtures()
        .map(|f| (f.name.as_str(), f.id))
        .collect();
    // The set's "Pin 1" is taken too (UX review 2026-11-16, criterion 13 amended).
    assert_eq!(
        jobs,
        [("Pin 2", first), ("Pin 3", second), ("Holder 1", holder)]
    );
    let placed = d.fixture(second).unwrap();
    assert_eq!(placed.matrix.unwrap().to_array()[4..], [200.0, 50.0]);
    assert!(placed.active && placed.set.is_none());
}

#[test]
fn a_page_takes_sixty_four_job_fixtures_from_its_add_bar() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let values = holder_values();
    for n in 0..MAX_JOB_FIXTURES {
        d.add_job_fixture(mfg, &new(HOLDER, "Holder", &values, n as f64))
            .unwrap();
    }
    assert!(matches!(
        d.add_job_fixture(mfg, &new(HOLDER, "Holder", &values, 1.0)),
        Err(ManufacturingError::TooManyJobFixtures)
    ));
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    assert!(
        d.add_set_fixture(set, &new(HOLDER, "Holder", &values, 1.0))
            .is_ok(),
        "the set fixtures do not count"
    );
}

#[test]
fn everything_lists_sets_in_order_with_their_fixtures_then_job_fixtures() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let first_job = job_pin(&d, mfg, 1.0);
    let a = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let b = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let in_b = d
        .add_set_fixture(b, &new(HOLDER, "Holder", &holder_values(), 2.0))
        .unwrap();
    let in_a = d
        .add_set_fixture(a, &new(HOLDER, "Holder", &holder_values(), 3.0))
        .unwrap();
    let second_job = job_pin(&d, mfg, 4.0);
    let in_a_2 = d
        .add_set_fixture(a, &new(SCREW, "Screw", &screw_values(), 5.0))
        .unwrap();
    let order: Vec<NodeId> = d
        .manufacturing(mfg)
        .unwrap()
        .fixtures
        .iter()
        .map(|f| f.id)
        .collect();
    assert_eq!(order, [in_a, in_a_2, in_b, first_job, second_job]);
}

// ---- role jobs (criterion 27) -------------------------------------------------------------------

#[test]
fn a_pin_and_a_screw_are_created_with_their_two_role_jobs_output_off() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let pin = job_pin(&d, mfg, 100.0);
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let screw = d
        .add_set_fixture(set, &new(SCREW, "Screw", &screw_values(), 200.0))
        .unwrap();
    let holder = d
        .add_job_fixture(mfg, &new(HOLDER, "Holder", &holder_values(), 300.0))
        .unwrap();
    for (id, circle) in [(pin, vec!["circle"]), (screw, vec!["head", "thread"])] {
        let read = d.fixture(id).unwrap();
        let jobs: Vec<(Option<JobRole>, &str, Option<&str>, bool, bool)> = read
            .jobs
            .iter()
            .map(|j| (j.role, j.name.as_str(), j.kind.as_deref(), j.show, j.output))
            .collect();
        assert_eq!(
            jobs,
            [
                (
                    Some(JobRole::CentreMark),
                    "Centre mark",
                    Some("score"),
                    true,
                    false
                ),
                (
                    Some(JobRole::Diameters),
                    "Diameters",
                    Some("score"),
                    true,
                    false
                ),
            ]
        );
        let marks: Vec<Option<&str>> = read.jobs[0]
            .targets
            .iter()
            .map(|t| t.key.suffix.as_deref())
            .collect();
        assert_eq!(marks, [Some("marks")]);
        let mut parts: Vec<&str> = read.jobs[1]
            .targets
            .iter()
            .filter_map(|t| t.key.suffix.as_deref())
            .collect();
        parts.sort_unstable();
        assert_eq!(parts, circle);
        assert!(read.jobs[1].targets.iter().all(|t| t.key.node == id));
        assert_eq!(read.damaged, 0);
    }
    assert!(d.fixture(holder).unwrap().jobs.is_empty());
}

#[test]
fn a_role_job_is_not_deleted_renamed_or_re_ticked_and_a_pin_takes_no_further_job() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let pin = job_pin(&d, mfg, 100.0);
    let jobs = d.fixture(pin).unwrap().jobs;
    let marks = jobs[0].id;
    let steps = d.own_step_count();
    assert_eq!(
        d.delete_fixture_job(marks).unwrap_err(),
        FixtureJobError::RoleJob
    );
    assert_eq!(
        d.rename_fixture_job(marks, "Mine").unwrap_err(),
        FixtureJobError::RoleJob
    );
    let circle = FixturePartKey {
        node: pin,
        suffix: Some("circle".to_owned()),
    };
    assert_eq!(
        d.set_fixture_job_targets(marks, &[circle], true)
            .unwrap_err(),
        FixtureJobError::RoleJob
    );
    assert_eq!(
        d.add_fixture_job(pin, "cut", "Cut").unwrap_err(),
        FixtureJobError::FixedJobs
    );
    assert_eq!(d.own_step_count(), steps, "nothing written");
    assert!(
        d.set_fixture_job_switch(marks, FixtureJobSwitch::Output, true)
            .unwrap(),
        "the toggle is the role job's Output"
    );
    assert_eq!(last_label(&d), "set_fixture_job_output_on");
    d.delete_fixture(pin).unwrap();
    assert!(d.fixture_job(marks).is_none(), "deleted with its fixture");
}

// ---- lock (criteria 31, 32) ---------------------------------------------------------------------

#[test]
fn a_job_fixture_or_a_set_fixture_locks_and_a_set_locks_in_their_own_steps() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let pin = job_pin(&d, mfg, 100.0);
    assert!(
        !d.set_fixture_locked(pin, false).unwrap(),
        "already unlocked"
    );
    assert!(d.set_fixture_locked(pin, true).unwrap());
    assert_eq!(last_label(&d), "lock_fixture");
    assert!(d.fixture(pin).unwrap().locked);
    assert!(!d.set_fixture_locked(pin, true).unwrap());
    assert!(d.set_fixture_locked(pin, false).unwrap());
    assert_eq!(last_label(&d), "unlock_fixture");
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    let holder = d
        .add_set_fixture(set, &new(HOLDER, "Holder", &holder_values(), 1.0))
        .unwrap();
    // Amended 2026-11-16 (criterion 32): a set fixture has its own Lock, and a set has one.
    assert!(d.set_fixture_locked(holder, true).unwrap());
    assert!(d.fixture(holder).unwrap().locked);
    assert!(matches!(
        d.set_fixture_locked(set, true),
        Err(ManufacturingError::NoSuchFixture)
    ));
    assert!(d.set_fixture_set_locked(set, true).unwrap());
    assert_eq!(last_label(&d), "lock_fixture_set");
    assert!(d.manufacturing(mfg).unwrap().sets[0].locked);
    assert!(
        !d.set_fixture_set_locked(set, true).unwrap(),
        "already locked"
    );
    assert!(d.set_fixture_set_locked(set, false).unwrap());
    assert_eq!(last_label(&d), "unlock_fixture_set");
    assert!(matches!(
        d.set_fixture_set_locked(holder, true),
        Err(ManufacturingError::NoSuchSet)
    ));
}

// ---- Apply fixtures from page (criteria 35, 37) -------------------------------------------------

#[test]
fn apply_replaces_the_job_fixtures_with_copies_fresh_ids_output_off_and_ticks_remapped() {
    let d = Document::new(1);
    let source = laser(&d);
    let target = laser(&d);
    let set = d.add_fixture_set(source, &next_uid()).unwrap();
    d.add_set_fixture(set, &new(HOLDER, "Holder", &holder_values(), 9.0))
        .unwrap();
    let pin = job_pin(&d, source, 100.0);
    d.set_fixture_locked(pin, true).unwrap();
    d.set_fixture_visible(pin, false).unwrap();
    let marks = d.fixture(pin).unwrap().jobs[0].id;
    d.set_fixture_job_switch(marks, FixtureJobSwitch::Output, true)
        .unwrap();
    let holder = d
        .add_job_fixture(source, &new(HOLDER, "Holder", &holder_values(), 300.0))
        .unwrap();
    let slot_values = params(serde_json::json!({
        "shape": "circle", "cx_mm": 50.0, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 20.0, "d_mm": 12.0
    }));
    let slot = d.add_slot(holder, "Slot", &slot_values).unwrap();
    let job = d.add_fixture_job(holder, "cut", "Cut 1").unwrap();
    let tick = FixturePartKey {
        node: slot,
        suffix: None,
    };
    d.set_fixture_job_targets(job, &[tick], true).unwrap();
    let own = job_pin(&d, target, 5.0);
    let own_set = d.add_fixture_set(target, &next_uid()).unwrap();

    let steps = d.own_step_count();
    let copies = d.apply_page_fixtures(target, source).unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert_eq!(last_label(&d), "apply_page_fixtures");
    let apply = last_step(&d);
    assert_eq!(copies.len(), 2, "the set's holder is not copied");
    let read = d.manufacturing(target).unwrap();
    let order: Vec<NodeId> = read.job_fixtures().map(|f| f.id).collect();
    assert_eq!(order, copies, "the page's own job fixture is replaced");
    assert!(d.fixture(own).is_none(), "the replaced job fixture is gone");
    assert!(
        d.manufacturing(target)
            .unwrap()
            .sets
            .iter()
            .any(|set| set.id == own_set),
        "the page's sets stay"
    );
    let originals = d.manufacturing(source).unwrap();
    for (copy, original) in copies.iter().zip(originals.job_fixtures()) {
        let copied = d.fixture(*copy).unwrap();
        assert_ne!(copied.id, original.id);
        assert_eq!(copied.name, original.name, "no rename");
        assert_eq!(copied.matrix, original.matrix, "same place, not mirrored");
        assert_eq!(copied.locked, original.locked);
        assert_eq!(copied.visible, original.visible);
        assert_eq!(copied.record(), original.record());
        assert!(copied.jobs.iter().all(|j| !j.output), "Output off");
        for (job, from) in copied.jobs.iter().zip(&original.jobs) {
            assert_ne!(job.id, from.id);
            assert_eq!(job.role, from.role);
            assert_eq!(job.targets.len(), from.targets.len());
            for target in &job.targets {
                let own_part = target.key.node == copied.id
                    || copied.slots.iter().any(|s| s.id == target.key.node);
                assert!(own_part, "a tick names the copy's own part");
            }
        }
        assert_eq!(copied.damaged, 0);
    }
    let copied_holder = d.fixture(copies[1]).unwrap();
    d.rename_fixture(copies[1], "Changed").unwrap();
    assert_eq!(
        d.fixture(holder).unwrap().name,
        copied_holder.name,
        "independent"
    );
    assert_ne!(copied_holder.slots[0].id, slot);
    d.undo_step(last_step(&d)).unwrap();
    d.undo_step(apply).unwrap();
    let restored: Vec<NodeId> = d
        .manufacturing(target)
        .unwrap()
        .job_fixtures()
        .map(|f| f.id)
        .collect();
    assert_eq!(
        restored,
        [own],
        "one undo brings the replaced job fixture back"
    );
}

#[test]
fn applying_the_same_page_twice_gives_one_copy_and_an_empty_page_writes_nothing() {
    let d = Document::new(1);
    let source = laser(&d);
    let target = laser(&d);
    let steps = d.own_step_count();
    assert!(d.apply_page_fixtures(target, source).unwrap().is_empty());
    assert_eq!(d.own_step_count(), steps);
    let pin = job_pin(&d, source, 1.0);
    d.apply_page_fixtures(target, source).unwrap();
    d.apply_page_fixtures(target, source).unwrap();
    let names: Vec<String> = d
        .manufacturing(target)
        .unwrap()
        .job_fixtures()
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(names, ["Pin 1"], "the second Apply replaces the first");
    assert!(matches!(
        d.apply_page_fixtures(target, pin),
        Err(ManufacturingError::NotFound)
    ));
}

#[test]
fn apply_onto_a_full_page_replaces_its_sixty_four_job_fixtures() {
    let d = Document::new(1);
    let source = laser(&d);
    let target = laser(&d);
    let values = holder_values();
    for n in 0..3 {
        d.add_job_fixture(source, &new(HOLDER, "Holder", &values, f64::from(n)))
            .unwrap();
    }
    for n in 0..MAX_JOB_FIXTURES {
        d.add_job_fixture(target, &new(HOLDER, "Holder", &values, n as f64))
            .unwrap();
    }
    let steps = d.own_step_count();
    assert_eq!(d.apply_page_fixtures(target, source).unwrap().len(), 3);
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert_eq!(
        d.manufacturing(target).unwrap().job_fixtures().count(),
        3,
        "only the source's count counts against 64"
    );
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(
        d.manufacturing(target).unwrap().job_fixtures().count(),
        MAX_JOB_FIXTURES
    );
}

// ---- profile copy (criteria 9, 10, 14) ----------------------------------------------------------

#[test]
fn the_set_records_hold_names_and_fixtures_without_job_fixtures_or_on_this_page() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let set = d.add_fixture_set(mfg, &next_uid()).unwrap();
    d.rename_fixture_set(set, "Bench").unwrap();
    d.add_set_fixture(set, &new(PIN, "Pin", &pin_values(), 1.0))
        .unwrap();
    d.set_fixture_set_on_page(set, false).unwrap();
    job_pin(&d, mfg, 2.0);
    let records = d.manufacturing(mfg).unwrap().set_records();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "Bench");
    assert_eq!(records[0].fixtures.len(), 1);
    let roles: Vec<Option<JobRole>> = records[0].fixtures[0].jobs.iter().map(|j| j.role).collect();
    assert_eq!(roles, [Some(JobRole::CentreMark), Some(JobRole::Diameters)]);
}
