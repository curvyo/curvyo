//! Format 59 (`specs/0150-operation-always-has-numbers` criteria 4, 15, 16, 20, 22, 28; `adrs.md`
//! decisions 1, 3, 4, 7): the golden `operation_defaults_v59.curvyo` reads back every copy key and
//! operation register as plain values and saves back unchanged; a new operation carries its
//! numbers in its add step; a profile step writes its fills in the same step; a file of format 58
//! opens unchanged. What the laser extension reads of the golden is in
//! `curvyo-laser-core/tests/operation_defaults_0150.rs`. `CURVYO_WRITE_FIXTURES=1` rewrites it.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOperation, NewOutputPage,
    NodeId, PageContents, PageCorner, ProfileChange, SettingsStep, SettingsWrite, pack, unpack,
};
use serde_json::{Map, Value, json};

const GOLDEN: &str = "operation_defaults_v59.curvyo";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("a map")
    };
    map
}

/// Page 1's copy: the pair 12.5 % and 1,500 mm/min.
fn copy_with_pair() -> FixtureParams {
    map(json!({
        "id": "generic-laser", "name": "Generic laser", "origin": "bottom_left",
        "max_feed_mm_per_min": 6000, "max_power": 1000, "laser_mode": true,
        "motorised_z": false, "air_assist": false, "output_format": "grbl",
        "default_power_pct": 12.5, "default_speed_mm_min": 1500,
    }))
}

/// Page 2's copy: made before this story, no pair, max feed 600.
fn copy_without_pair() -> FixtureParams {
    map(json!({
        "id": "generic-laser", "name": "Slow laser", "origin": "bottom_left",
        "max_feed_mm_per_min": 600, "max_power": 1000, "laser_mode": true,
        "motorised_z": false, "air_assist": false, "output_format": "grbl",
    }))
}

fn laser(d: &Document, profile: &FixtureParams) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(profile),
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

/// Adds an operation under a first name and renames it, so the golden's history has no two
/// adjacent steps with one label (the History list's coalescing oracle of `0020`).
fn add(d: &Document, mfg: NodeId, kind: &str, name: &str, settings: Option<Value>) -> NodeId {
    let settings = settings.map(map);
    let id = d
        .add_operation(
            mfg,
            &NewOperation {
                kind,
                name: "New",
                settings: settings.as_ref(),
            },
            usize::MAX,
            &[],
        )
        .unwrap();
    assert!(d.rename_operation(id, name).unwrap());
    id
}

/// The golden: page 1 with a Cut, an Engrave + Outline and an Image without `op_settings`, an
/// own Score, a linked Engrave, a damaged Cut (power 0) and an unknown kind `drill`; page 2 with
/// a Cut without `op_settings`.
fn golden_document() -> Document {
    let d = Document::new(1);
    let one = laser(&d, &copy_with_pair());
    add(&d, one, "cut", "Cut 1", None);
    add(&d, one, "engrave_outline", "Engrave + Outline 1", None);
    add(&d, one, "image", "Image 1", None);
    add(
        &d,
        one,
        "score",
        "Score 1",
        Some(json!({"main": {"power_pct": 20.0, "speed_mm_min": 3000, "passes": 1}})),
    );
    add(
        &d,
        one,
        "engrave",
        "Engrave 1",
        Some(json!({
            "main": {"power_pct": 30.0, "speed_mm_min": 6000, "passes": 1},
            "material": {"id": "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c", "name": "Birch 3 mm",
                         "modified": 1_762_862_400_000_u64},
        })),
    );
    add(
        &d,
        one,
        "cut",
        "Cut 2",
        Some(json!({"main": {"power_pct": 0.0, "speed_mm_min": 300, "passes": 1}})),
    );
    add(&d, one, "drill", "Drill 1", None);
    let two = laser(&d, &copy_without_pair());
    add(&d, two, "cut", "Cut 1", None);
    d
}

fn write_fixture_on_request() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(
            fixture_path(GOLDEN),
            pack(&golden_document(), "0.1.0").unwrap(),
        )
        .unwrap();
    }
}

fn members(bytes: &[u8]) -> BTreeMap<String, Vec<u8>> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|i| {
            let mut file = archive.by_index(i).unwrap();
            let mut content = Vec::new();
            file.read_to_end(&mut content).unwrap();
            (file.name().unwrap().to_string(), content)
        })
        .collect()
}

/// Each operation of each manufacturing as `(kind, name, op_settings)`.
fn registers(d: &Document) -> Vec<Vec<(String, String, Option<Value>)>> {
    d.manufacturings()
        .iter()
        .map(|m| {
            d.operations(m.id)
                .unwrap()
                .into_iter()
                .map(|op| (op.kind.unwrap_or_default(), op.name, op.settings))
                .collect()
        })
        .collect()
}

#[test]
fn the_golden_reads_back_every_register_and_saves_back_unchanged() {
    write_fixture_on_request();
    let bytes = std::fs::read(fixture_path(GOLDEN)).unwrap();
    let manifest: Value = serde_json::from_slice(&members(&bytes)["manifest.json"]).unwrap();
    assert_eq!(manifest["format_version"], 59);
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0, "opening writes no step");
    let m = d.manufacturings();
    assert_eq!(m[0].profile.as_ref(), Some(&copy_with_pair()));
    assert_eq!(m[1].profile.as_ref(), Some(&copy_without_pair()));
    let ops = registers(&d);
    let absent: Vec<&str> = ops[0]
        .iter()
        .filter(|(_, _, settings)| settings.is_none())
        .map(|(_, name, _)| name.as_str())
        .collect();
    assert_eq!(
        absent,
        ["Cut 1", "Engrave + Outline 1", "Image 1", "Drill 1"]
    );
    assert!(ops[1][0].2.is_none(), "page 2's Cut has no op_settings");
    assert_eq!(ops, registers(&golden_document()));
    let saved = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        members(&saved)["document.loro"],
        members(&bytes)["document.loro"],
        "saved back unchanged"
    );
}

#[test]
fn criteria_4_20_a_new_operation_holds_its_numbers_in_the_add_step() {
    let d = Document::new(1);
    let mfg = laser(&d, &copy_with_pair());
    let steps = d.own_step_count();
    let numbers = json!({"main": {"power_pct": 12.5, "speed_mm_min": 1500, "passes": 1}});
    let id = d
        .add_operation(
            mfg,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: Some(&map(numbers.clone())),
            },
            0,
            &[],
        )
        .unwrap();
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert_eq!(d.own_steps().last().unwrap().label, "add_operation");
    let op = d.operations(mfg).unwrap();
    assert_eq!((op[0].id, op[0].settings.clone()), (id, Some(numbers)));
}

#[test]
fn criteria_8_22_a_profile_step_writes_its_fills_in_the_same_step() {
    let d = Document::new(1);
    let mfg = laser(&d, &copy_with_pair());
    let cut = add(&d, mfg, "cut", "Cut 1", None);
    let steps = d.own_step_count();
    let mut lowered = copy_with_pair();
    lowered.insert("default_speed_mm_min".into(), json!(900));
    let fill = SettingsWrite {
        operation: cut,
        value: Some(map(
            json!({"main": {"power_pct": 12.5, "speed_mm_min": 1500, "passes": 1}}),
        )),
    };
    assert!(
        d.set_manufacturing_profile_filled(
            mfg,
            &lowered,
            DocumentSize::from_mm(400.0, 300.0),
            PageCorner::default(),
            ProfileChange::EditDefaultSpeed,
            None,
            std::slice::from_ref(&fill),
            &PageContents::new(),
        )
        .unwrap()
    );
    assert_eq!(d.own_step_count(), steps + 1, "copy and fill are one step");
    assert_eq!(d.own_steps().last().unwrap().label, "edit_default_speed");
    assert_eq!(d.manufacturing(mfg).unwrap().profile, Some(lowered));
    assert_eq!(
        d.operations(mfg).unwrap()[0].settings,
        fill.value.map(Value::Object)
    );
    // A grid row or a missing operation refuses the whole step.
    let missing = SettingsWrite {
        operation: mfg,
        value: None,
    };
    assert!(
        d.set_manufacturing_profile_filled(
            mfg,
            &copy_with_pair(),
            DocumentSize::from_mm(400.0, 300.0),
            PageCorner::default(),
            ProfileChange::EditDefaultSpeed,
            None,
            &[missing],
            &PageContents::new(),
        )
        .is_err()
    );
    assert_eq!(d.own_step_count(), steps + 1);
    // An own number typed later is an ordinary settings step.
    let typed = SettingsWrite {
        operation: cut,
        value: Some(map(
            json!({"main": {"power_pct": 20.0, "speed_mm_min": 1500, "passes": 1}}),
        )),
    };
    assert!(
        d.write_operation_settings(SettingsStep::EditSettings, &[typed])
            .unwrap()
    );
}

#[test]
fn criterion_28_this_build_writes_59_and_a_file_of_58_opens_unchanged() {
    assert_eq!(CURRENT_FORMAT_VERSION, 72);
    let bytes = std::fs::read(fixture_path("came_from_v58.curvyo")).unwrap();
    let d = unpack(2, &bytes).unwrap();
    assert_eq!(d.own_step_count(), 0);
    let saved = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        members(&saved)["document.loro"],
        members(&bytes)["document.loro"]
    );
}
