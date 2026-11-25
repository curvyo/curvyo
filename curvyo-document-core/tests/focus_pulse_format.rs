//! Format 63 (`specs/0118-machine-control` criterion 35 a; `adrs.md` P7, P8): the focus pulse's
//! three keys in the profile copy `mfg_profile`. Each edit is one step with its own label and
//! undo takes it back; the goldens read back and save unchanged, damaged values included. The
//! laser extension's reading of each value is tested in `curvyo-laser-core`
//! (`tests/focus_pulse_0118.rs`). `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, EDIT_PULSE_LENGTH_COMMIT_LABEL,
    EDIT_PULSE_POWER_COMMIT_LABEL, FOCUS_PULSE_OFF_COMMIT_LABEL, FOCUS_PULSE_ON_COMMIT_LABEL,
    FixtureParams, NewOutputPage, NodeId, PageContents, PageCorner, ProfileChange, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Value, json};

const DIODE: &str = "6f1c2b0a-5d4e-4f3a-9b2c-1d0e8f7a6b5c";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn copy(extra: &Value) -> FixtureParams {
    let mut map: FixtureParams = serde_json::from_value(json!({
        "id": DIODE,
        "name": "Diode 10 W",
        "origin": "bottom_left",
        "max_feed_mm_per_min": 6000,
        "max_power": 1000,
        "laser_mode": true,
        "motorised_z": false,
        "air_assist": false,
        "output_format": "grbl",
    }))
    .unwrap();
    for (key, value) in extra.as_object().unwrap() {
        map.insert(key.clone(), value.clone());
    }
    map
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

fn edit(d: &Document, m: NodeId, profile: &FixtureParams, change: ProfileChange) -> bool {
    d.set_manufacturing_profile(
        m,
        profile,
        DocumentSize::from_mm(400.0, 300.0),
        PageCorner {
            right: false,
            bottom: true,
        },
        change,
        &PageContents::new(),
    )
    .unwrap()
}

fn stored(d: &Document, m: NodeId) -> FixtureParams {
    d.manufacturing(m).unwrap().profile.unwrap()
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
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

fn format_of(bytes: &[u8]) -> u64 {
    let json: Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(name), build()).unwrap();
    }
    std::fs::read(fixture_path(name)).unwrap()
}

type Registers = BTreeMap<String, LoroValue>;

fn raw_nodes(bytes: &[u8]) -> Vec<Registers> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    tree.children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .map(|node| {
            let mut registers = Registers::new();
            tree.get_meta(node).unwrap().for_each(|key, value| {
                registers.insert(key.to_owned(), value.into_value().unwrap());
            });
            registers
        })
        .collect()
}

fn open_unchanged(bytes: &[u8]) -> Document {
    let opened = unpack(2, bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_nodes(&saved), raw_nodes(bytes), "saved back unchanged");
    opened
}

#[test]
fn c35a_each_field_edit_is_one_step_with_its_label_and_undo_takes_it_back() {
    let d = Document::new(1);
    let defaults = json!({
        "focus_pulse": false,
        "focus_pulse_power_permille": 10,
        "focus_pulse_length_ms": 300,
    });
    let m = laser(&d, &copy(&defaults));
    let table = [
        (
            json!({"focus_pulse": true}),
            ProfileChange::FocusPulse { on: true },
            FOCUS_PULSE_ON_COMMIT_LABEL,
        ),
        (
            json!({"focus_pulse_power_permille": 25}),
            ProfileChange::EditPulsePower,
            EDIT_PULSE_POWER_COMMIT_LABEL,
        ),
        (
            json!({"focus_pulse_length_ms": 500}),
            ProfileChange::EditPulseLength,
            EDIT_PULSE_LENGTH_COMMIT_LABEL,
        ),
        (
            json!({"focus_pulse": false}),
            ProfileChange::FocusPulse { on: false },
            FOCUS_PULSE_OFF_COMMIT_LABEL,
        ),
    ];
    for (change, kind, label) in table {
        let before = stored(&d, m);
        let steps = d.own_step_count();
        let mut next = before.clone();
        for (key, value) in change.as_object().unwrap() {
            next.insert(key.clone(), value.clone());
        }
        assert!(edit(&d, m, &next, kind));
        assert_eq!(d.own_step_count(), steps + 1, "{label}");
        assert_eq!(last_label(&d), label);
        assert_eq!(stored(&d, m), next);
        let last = d.own_steps().last().unwrap().step;
        d.undo_step(last).unwrap();
        assert_eq!(stored(&d, m), before, "{label} undone");
        assert!(edit(&d, m, &next, kind), "{label} again");
    }
}

fn golden_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let on = json!({
        "focus_pulse": true,
        "focus_pulse_power_permille": 25,
        "focus_pulse_length_ms": 500,
    });
    let defaults = json!({
        "focus_pulse": false,
        "focus_pulse_power_permille": 10,
        "focus_pulse_length_ms": 300,
    });
    for extra in [on, defaults, json!({})] {
        let m = laser(&d, &copy(&extra));
        // A step between the pages, so no two history rows share a label.
        d.set_start(m, "head_position").unwrap();
    }
    pack(&d, "0.1.0").unwrap()
}

#[test]
fn c35a_the_golden_keeps_the_three_keys_and_a_page_without_them() {
    let bytes = read_or_write("focus_pulse_v63.curvyo", golden_bytes);
    assert_eq!(format_of(&bytes), 63);
    let d = open_unchanged(&bytes);
    let pages = d.manufacturings();
    assert_eq!(pages.len(), 3);
    let first = pages[0].profile.as_ref().unwrap();
    assert_eq!(first["focus_pulse"], json!(true));
    assert_eq!(first["focus_pulse_power_permille"], json!(25));
    assert_eq!(first["focus_pulse_length_ms"], json!(500));
    let second = pages[1].profile.as_ref().unwrap();
    assert_eq!(second["focus_pulse"], json!(false));
    let third = pages[2].profile.as_ref().unwrap();
    assert!(!third.contains_key("focus_pulse"));
}

fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    for extra in [
        json!({"focus_pulse": "on"}),
        json!({"focus_pulse_power_permille": 0}),
        json!({"focus_pulse_length_ms": 2001}),
        json!({"focus_pulse_power_permille": 1.5}),
    ] {
        let m = laser(&d, &copy(&extra));
        // A step between the pages, as the other damaged goldens have.
        d.set_start(m, "head_position").unwrap();
    }
    pack(&d, "0.1.0").unwrap()
}

#[test]
fn c35a_damaged_values_are_kept_as_stored_and_saved_back_unchanged() {
    let bytes = read_or_write("focus_pulse_damaged_v63.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 63);
    let d = open_unchanged(&bytes);
    let pages = d.manufacturings();
    let profile = |i: usize| pages[i].profile.clone().unwrap();
    assert_eq!(profile(0)["focus_pulse"], json!("on"));
    assert_eq!(profile(1)["focus_pulse_power_permille"], json!(0));
    assert_eq!(profile(2)["focus_pulse_length_ms"], json!(2001));
    assert_eq!(profile(3)["focus_pulse_power_permille"], json!(1.5));
}

#[test]
fn the_format_is_63() {
    // Format 63 since `specs/0118-machine-control` slice 2d (`adrs.md` P8).
    assert_eq!(CURRENT_FORMAT_VERSION, 63);
}
