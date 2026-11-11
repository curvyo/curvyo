//! The fixture commands of ADR 0049 (`specs/0102-laser-fixtures` criteria 2 to 6, 14, 15, 18 to
//! 20, 29): one step per command with its own label, names and limits, slots as child nodes, and
//! the fixture part of Pick and Reset from profile (ADR 0050 §4, §6).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty
)]

use curvyo_document_core::{
    Affine, Angle, Document, DocumentSize, FixtureAdd, FixtureParams, FixtureRecord, FixtureResize,
    FixtureTurn, ManufacturingError, NewFixture, NewOutputPage, NodeId, PageCorner, Point,
    ProfileChange, SlotRecord, StepRef, Vec2,
};

const HOLDER: &str = "curvyo.laser/holder";
const GRID: &str = "curvyo.laser/engraved_grid";
const SCREW: &str = "curvyo.laser/screw";

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

fn board() -> FixtureParams {
    params(serde_json::json!({"width_mm": 100.0, "height_mm": 60.0}))
}

fn slot(shape: &str) -> FixtureParams {
    params(serde_json::json!({
        "shape": shape, "cx_mm": 50.0, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 20.0, "d_mm": 20.0
    }))
}

fn holder(d: &Document, mfg: NodeId) -> NodeId {
    d.add_fixture(
        mfg,
        &NewFixture {
            add: FixtureAdd::Holder,
            kind: HOLDER,
            label: "Holder",
            params: &board(),
            matrix: Some(Affine::translation(Vec2::new(250.0, 170.0))),
        },
    )
    .unwrap()
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn last_step(d: &Document) -> StepRef {
    d.own_steps().last().unwrap().step
}

#[test]
fn each_add_button_has_its_label_and_names_count_per_kind() {
    let d = Document::new(1);
    let mfg = laser(&d);
    for (add, kind, label, step) in [
        (FixtureAdd::Grid, GRID, "Grid", "add_grid"),
        (FixtureAdd::Holder, HOLDER, "Holder", "add_holder"),
        (
            FixtureAdd::Clamp,
            "curvyo.laser/clamp",
            "Clamp",
            "add_clamp",
        ),
        (FixtureAdd::Stop, "curvyo.laser/stop", "Stop", "add_stop"),
        (FixtureAdd::Screw, SCREW, "Screw", "add_screw"),
        (FixtureAdd::Holder, HOLDER, "Holder", "add_holder"),
    ] {
        d.add_fixture(
            mfg,
            &NewFixture {
                add,
                kind,
                label,
                params: &FixtureParams::new(),
                matrix: None,
            },
        )
        .unwrap();
        assert_eq!(last_label(&d), step);
    }
    let fixtures = d.manufacturing(mfg).unwrap().fixtures;
    let names: Vec<&str> = fixtures.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "Grid 1", "Holder 1", "Clamp 1", "Stop 1", "Screw 1", "Holder 2"
        ]
    );
    assert!(fixtures.iter().all(|f| f.active && f.visible));
    assert!(fixtures.iter().all(|f| f.damaged == 0));
}

#[test]
fn rename_cuts_to_forty_characters_and_an_empty_name_changes_nothing() {
    let d = Document::new(1);
    let id = holder(&d, laser(&d));
    assert!(d.rename_fixture(id, &"x".repeat(60)).unwrap());
    assert_eq!(last_label(&d), "rename_fixture");
    assert_eq!(d.fixture(id).unwrap().name.chars().count(), 40);
    let steps = d.own_step_count();
    assert!(!d.rename_fixture(id, "   ").unwrap());
    assert!(!d.rename_fixture(id, &"x".repeat(41)).unwrap(), "same name");
    assert_eq!(d.own_step_count(), steps);
}

#[test]
fn active_and_eye_are_two_registers_with_their_own_steps() {
    let d = Document::new(1);
    let id = holder(&d, laser(&d));
    assert!(
        !d.set_fixture_active(id, true).unwrap(),
        "new fixtures are on"
    );
    assert!(d.set_fixture_active(id, false).unwrap());
    assert_eq!(last_label(&d), "set_fixture_inactive");
    assert!(d.set_fixture_active(id, true).unwrap());
    assert_eq!(last_label(&d), "set_fixture_active");
    assert!(d.set_fixture_visible(id, false).unwrap());
    assert_eq!(last_label(&d), "set_fixture_visible");
    let read = d.fixture(id).unwrap();
    assert!(read.active && !read.visible);
}

#[test]
fn a_move_and_a_turn_write_the_matrix_alone() {
    let d = Document::new(1);
    let id = holder(&d, laser(&d));
    let moved = Affine::translation(Vec2::new(10.0, 20.0));
    assert!(d.set_fixture_matrix(id, moved, FixtureTurn::Move).unwrap());
    assert_eq!(last_label(&d), "move_fixture");
    assert!(!d.set_fixture_matrix(id, moved, FixtureTurn::Move).unwrap());
    let turned = moved.compose(Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(30.0_f64.to_radians()),
    ));
    assert!(
        d.set_fixture_matrix(id, turned, FixtureTurn::Rotate)
            .unwrap()
    );
    assert_eq!(last_label(&d), "rotate_fixture");
    let read = d.fixture(id).unwrap();
    assert_eq!(read.matrix, Some(turned));
    assert_eq!(read.params, board(), "the size is untouched");
    let far = Affine::translation(Vec2::new(2e7, 0.0));
    assert!(matches!(
        d.set_fixture_matrix(id, far, FixtureTurn::Move),
        Err(ManufacturingError::OutOfRange)
    ));
}

#[test]
fn params_are_written_whole_and_unchanged_values_write_nothing() {
    let d = Document::new(1);
    let id = holder(&d, laser(&d));
    let wide = params(serde_json::json!({"width_mm": 200.0, "height_mm": 60.0}));
    assert!(d.set_fixture_params(id, &wide).unwrap());
    assert_eq!(last_label(&d), "set_fixture_params");
    assert!(!d.set_fixture_params(id, &wide).unwrap());
    assert_eq!(d.fixture(id).unwrap().params, wide);
}

#[test]
fn slots_are_added_numbered_edited_and_removed_one_step_each() {
    let d = Document::new(1);
    let id = holder(&d, laser(&d));
    let first = d.add_slot(id, "Slot", &slot("rect")).unwrap();
    assert_eq!(last_label(&d), "add_slot");
    let second = d.add_slot(id, "Slot", &slot("circle")).unwrap();
    assert!(d.set_slot(second, "Coaster", &slot("circle")).unwrap());
    assert_eq!(last_label(&d), "set_slot");
    assert!(!d.set_slot(second, "Coaster", &slot("circle")).unwrap());
    let names: Vec<String> = d
        .fixture(id)
        .unwrap()
        .slots
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, ["Slot 1", "Coaster"]);
    d.remove_slot(first).unwrap();
    assert_eq!(last_label(&d), "remove_slot");
    assert_eq!(d.fixture(id).unwrap().slots.len(), 1);
    assert!(matches!(
        d.remove_slot(id),
        Err(ManufacturingError::NoSuchSlot)
    ));
    assert!(matches!(
        d.add_slot(second, "Slot", &slot("rect")),
        Err(ManufacturingError::NoSuchFixture)
    ));
}

#[test]
fn a_holder_takes_fifty_slots_from_add_slot() {
    let d = Document::new(1);
    let id = holder(&d, laser(&d));
    for _ in 0..50 {
        d.add_slot(id, "Slot", &slot("rect")).unwrap();
    }
    assert!(matches!(
        d.add_slot(id, "Slot", &slot("rect")),
        Err(ManufacturingError::TooManySlots)
    ));
    assert_eq!(d.fixture(id).unwrap().slots[49].name, "Slot 50");
}

#[test]
fn remove_fixture_takes_its_slots_and_undo_restores_them_exactly() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let id = holder(&d, mfg);
    d.add_slot(id, "Slot", &slot("rect")).unwrap();
    d.set_fixture_active(id, false).unwrap();
    let before = d.fixture(id).unwrap();
    d.delete_fixture(id).unwrap();
    assert_eq!(last_label(&d), "delete_fixture");
    assert!(d.manufacturing(mfg).unwrap().fixtures.is_empty());
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(d.fixture(id).unwrap(), before);
}

fn set(name: &str) -> Vec<FixtureRecord> {
    vec![FixtureRecord {
        kind: HOLDER.to_owned(),
        name: name.to_owned(),
        active: true,
        params: board(),
        matrix: Some(Affine::translation(Vec2::new(40.0, 25.0))),
        slots: vec![SlotRecord {
            name: "Slot 1".to_owned(),
            params: slot("circle"),
        }],
        jobs: Vec::new(),
    }]
}

fn pick(d: &Document, mfg: NodeId, records: &[FixtureRecord]) -> bool {
    let copy = params(serde_json::json!({"id": "generic-laser", "name": "Generic laser"}));
    d.set_manufacturing_profile_and_fixtures(
        mfg,
        &copy,
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner::default(),
        ProfileChange::Pick,
        records,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap()
}

#[test]
fn pick_replaces_the_fixtures_in_its_one_step_with_fresh_ids() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let old = holder(&d, mfg);
    let steps = d.own_step_count();
    assert!(pick(&d, mfg, &set("Board")));
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), "pick_profile");
    let fixtures = d.manufacturing(mfg).unwrap().fixtures;
    assert_eq!(fixtures.len(), 1);
    assert_ne!(fixtures[0].id, old, "a fresh id");
    assert_eq!(fixtures[0].record(), set("Board")[0]);
    assert!(!pick(&d, mfg, &set("Board")), "the same set writes nothing");
    d.undo_step(last_step(&d)).unwrap();
    let restored = d.manufacturing(mfg).unwrap().fixtures;
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].id, old, "undo restores the old node");
}

#[test]
fn an_empty_set_removes_the_fixtures_and_undo_brings_them_back() {
    let d = Document::new(1);
    let mfg = laser(&d);
    holder(&d, mfg);
    assert!(pick(&d, mfg, &[]));
    assert!(d.manufacturing(mfg).unwrap().fixtures.is_empty());
    d.undo_step(last_step(&d)).unwrap();
    assert_eq!(d.manufacturing(mfg).unwrap().fixtures.len(), 1);
}

#[test]
fn a_screw_thread_above_its_head_is_set_to_the_head_by_every_writer() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let screw = d
        .add_fixture(
            mfg,
            &NewFixture {
                add: FixtureAdd::Screw,
                kind: SCREW,
                label: "Screw",
                params: &params(
                    serde_json::json!({"thread_diameter_mm": 12.0, "head_diameter_mm": 10.0}),
                ),
                matrix: Some(Affine::translation(Vec2::new(300.0, 200.0))),
            },
        )
        .unwrap();
    let read = d.fixture(screw).unwrap();
    assert_eq!(read.params["thread_diameter_mm"], 10.0);
    let wide = params(serde_json::json!({"thread_diameter_mm": 20.0, "head_diameter_mm": 16.0}));
    let resize = FixtureResize {
        params: &wide,
        matrix: None,
        slots: &[],
    };
    assert!(d.resize_fixture(screw, &resize).unwrap());
    assert_eq!(last_label(&d), "resize_fixture");
    let read = d.fixture(screw).unwrap();
    assert_eq!(read.params["thread_diameter_mm"], 16.0);
    assert_eq!(read.params["head_diameter_mm"], 16.0);
    assert_eq!(
        read.matrix,
        Some(Affine::translation(Vec2::new(300.0, 200.0)))
    );
}

#[test]
fn a_resize_writes_the_sizes_the_matrix_and_the_slot_centres_in_one_step() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let board_id = holder(&d, mfg);
    let slot_id = d.add_slot(board_id, "Slot", &slot("rect")).unwrap();
    let steps = d.own_steps().len();
    let wider = params(serde_json::json!({"width_mm": 120.0, "height_mm": 60.0}));
    let moved_slot = params(serde_json::json!({
        "shape": "rect", "cx_mm": 70.0, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 20.0, "d_mm": 20.0
    }));
    let matrix = Affine::translation(Vec2::new(230.0, 170.0));
    let resize = FixtureResize {
        params: &wider,
        matrix: Some(matrix),
        slots: &[(slot_id, moved_slot.clone())],
    };
    assert!(d.resize_fixture(board_id, &resize).unwrap());
    assert_eq!(d.own_steps().len(), steps + 1, "one step");
    assert_eq!(last_label(&d), "resize_fixture");
    let read = d.fixture(board_id).unwrap();
    assert_eq!(read.params, wider);
    assert_eq!(read.matrix, Some(matrix));
    assert_eq!(read.slots[0].params, moved_slot);
    assert!(!d.resize_fixture(board_id, &resize).unwrap(), "unchanged");
    d.undo_step(last_step(&d)).unwrap();
    let read = d.fixture(board_id).unwrap();
    assert_eq!(read.params, board());
    assert_eq!(read.slots[0].params, slot("rect"));
    let other = holder(&d, mfg);
    let foreign = FixtureResize {
        params: &wider,
        matrix: None,
        slots: &[(slot_id, moved_slot)],
    };
    assert!(
        matches!(
            d.resize_fixture(other, &foreign),
            Err(ManufacturingError::NoSuchSlot)
        ),
        "a slot of another holder"
    );
}
