use serde_json::{Map, Value, json};

use super::*;
use crate::{
    Affine, DocumentSize, FixtureAdd, FixtureJobSwitch, FixtureParams, NewFixture, NewOperation,
    NewOutputPage, SettingsStep, Vec2,
};

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn registers(d: &Document, m: NodeId) -> Map<String, Value> {
    d.manufacturing(m).unwrap().process_values
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo(d: &Document) {
    let last = d.own_steps().last().unwrap().step;
    d.undo_step(last).unwrap();
}

const BIRCH: &str = "0b3c7f1e-8f0e-4c4a-9a51-2f6c1c1e9d10";

#[test]
fn c2_the_fixture_material_pick_is_one_step_and_none_removes_it() {
    let d = Document::new(1);
    let m = laser(&d);
    let steps = d.own_step_count();
    assert_eq!(d.set_fixture_material(m, Some(BIRCH)), Ok(true));
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), SET_FIXTURE_MATERIAL_COMMIT_LABEL);
    assert_eq!(
        registers(&d, m).get("x.curvyo.laser.fixture_material"),
        Some(&json!(BIRCH))
    );
    assert_eq!(d.set_fixture_material(m, Some(BIRCH)), Ok(false), "no step");
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(d.set_fixture_material(m, None), Ok(true));
    assert!(!registers(&d, m).contains_key("x.curvyo.laser.fixture_material"));
    assert_eq!(d.set_fixture_material(m, None), Ok(false));
    undo(&d);
    assert_eq!(
        registers(&d, m).get("x.curvyo.laser.fixture_material"),
        Some(&json!(BIRCH)),
        "undo restores the pick"
    );
}

#[test]
fn c38_start_is_one_register_with_one_step_per_value() {
    // Superseded by 0029 rework (2026-11-17): one register `start` with three values replaces
    // Start from and Home first.
    let d = Document::new(1);
    let m = laser(&d);
    assert!(
        !registers(&d, m).contains_key("x.curvyo.laser.start"),
        "absent"
    );
    for (value, label) in [
        ("head_position", SET_START_HEAD_POSITION_COMMIT_LABEL),
        ("home_then_origin", SET_START_HOME_THEN_ORIGIN_COMMIT_LABEL),
        ("machine_origin", SET_START_MACHINE_ORIGIN_COMMIT_LABEL),
    ] {
        let steps = d.own_step_count();
        assert_eq!(d.set_start(m, value), Ok(true));
        assert_eq!(d.own_step_count(), steps + 1, "one step");
        assert_eq!(last_label(&d), label);
        assert_eq!(
            registers(&d, m).get("x.curvyo.laser.start"),
            Some(&json!(value))
        );
        assert_eq!(d.set_start(m, value), Ok(false), "no step when it holds it");
    }
    undo(&d);
    assert_eq!(
        registers(&d, m).get("x.curvyo.laser.start"),
        Some(&json!("home_then_origin")),
        "undo restores the pick"
    );
    let values = registers(&d, m);
    assert!(
        !values
            .keys()
            .any(|key| key.ends_with("start_from") || key.ends_with("home_first")),
        "the first build's registers are never written"
    );
}

#[test]
fn a_damaged_start_is_rewritten_by_any_pick_and_bad_input_is_refused() {
    let d = Document::new(1);
    let m = laser(&d);
    let node = crate::paths::tree_id_of(m);
    let meta = d.loro().get_tree(OBJECTS_TREE).get_meta(node).unwrap();
    meta.insert("x.curvyo.laser.start", "middle").unwrap();
    d.loro().commit();
    assert_eq!(
        d.set_start(m, "machine_origin"),
        Ok(true),
        "even the default"
    );
    assert_eq!(
        registers(&d, m).get("x.curvyo.laser.start"),
        Some(&json!("machine_origin"))
    );
    assert_eq!(d.set_start(m, "middle"), Err(OperationError::BadStart));
    let rect = d.create_rect(crate::RectBounds::from_corners(
        crate::Point::new(0.0, 0.0),
        crate::Point::new(1.0, 1.0),
    ));
    assert_eq!(
        d.set_start(rect, "head_position"),
        Err(OperationError::NoSuchManufacturing)
    );
}

fn holder_job(d: &Document, m: NodeId) -> NodeId {
    let params: FixtureParams =
        serde_json::from_value(json!({"width_mm": 100.0, "height_mm": 60.0})).unwrap();
    let holder = d
        .add_fixture(
            m,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: "curvyo.laser/holder",
                label: "Holder",
                params: &params,
                matrix: Some(Affine::translation(Vec2::new(50.0, 50.0))),
            },
        )
        .unwrap();
    let job = d.add_fixture_job(holder, "score", "Score 1").unwrap();
    d.set_fixture_job_switch(job, FixtureJobSwitch::Output, true)
        .unwrap();
    job
}

fn own(power: f64) -> Map<String, Value> {
    let Value::Object(map) =
        json!({"main": {"power_pct": power, "speed_mm_min": 300, "passes": 1}})
    else {
        panic!("a map")
    };
    map
}

#[test]
fn c28_the_export_step_switches_fixture_jobs_off_and_refreshes_in_one_undoable_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let job = holder_job(&d, m);
    let cut = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &[],
        )
        .unwrap();
    d.write_operation_settings(
        SettingsStep::EditSettings,
        &[SettingsWrite {
            operation: cut,
            value: Some(own(80.0)),
        }],
    )
    .unwrap();
    let steps = d.own_step_count();
    let refreshed = [SettingsWrite {
        operation: cut,
        value: Some(own(85.0)),
    }];
    assert!(d.export_job(&[job], &refreshed));
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert_eq!(last_label(&d), EXPORT_JOB_COMMIT_LABEL);
    assert!(!d.fixture_job(job).unwrap().output);
    let power = |d: &Document| {
        d.operations(m).unwrap()[0].settings.as_ref().unwrap()["main"]["power_pct"].clone()
    };
    assert_eq!(power(&d), json!(85.0));
    undo(&d);
    assert!(d.fixture_job(job).unwrap().output, "undo restores (a)");
    assert_eq!(power(&d), json!(80.0), "undo restores (b)");
}

#[test]
fn c28_an_export_with_nothing_to_change_writes_no_step_and_gone_ids_are_skipped() {
    let d = Document::new(1);
    let m = laser(&d);
    let job = holder_job(&d, m);
    d.set_fixture_job_switch(job, FixtureJobSwitch::Output, false)
        .unwrap();
    let gone = d
        .add_operation(
            m,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
            },
            0,
            &[],
        )
        .unwrap();
    d.delete_operation(gone).unwrap();
    let steps = d.own_step_count();
    let refreshed = [SettingsWrite {
        operation: gone,
        value: Some(own(85.0)),
    }];
    assert!(!d.export_job(&[job, m], &refreshed));
    assert_eq!(d.own_step_count(), steps);
}
