//! Format 48 (`specs/0029-laser-job-preview-and-output` criterion 32, ADR 0065 §1, §3, §4, §6):
//! the goldens `laser_output_v48.curvyo` and `laser_output_damaged_v48.curvyo` read back every
//! register and copy value and save back unchanged; the version 47 goldens still open. The laser
//! extension's reading of each value is tested in `curvyo-laser-core`
//! (`tests/laser_output_goldens.rs`). `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOutputPage, NodeId, pack,
    unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::json;

const BIRCH: &str = "0b3c7f1e-8f0e-4c4a-9a51-2f6c1c1e9d10";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn params(value: serde_json::Value) -> FixtureParams {
    serde_json::from_value(value).unwrap()
}

fn copy(extra: serde_json::Value) -> FixtureParams {
    let mut map = params(json!({
        "id": "6f1c2b0a-5d4e-4f3a-9b2c-1d0e8f7a6b5c",
        "name": "Diode 10 W",
        "origin": "bottom_left",
        "max_feed_mm_per_min": 6000,
        "max_power": 1000,
        "laser_mode": true,
        "motorised_z": false,
    }));
    map.extend(params(extra));
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

fn read_or_write(name: &str, build: impl Fn() -> Vec<u8>) -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(name), build()).unwrap();
    }
    std::fs::read(fixture_path(name)).unwrap()
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

fn rezip(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in files {
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// `bytes` with its document replaced by its state after `edit`.
fn edited(bytes: &[u8], edit: impl Fn(&LoroDoc)) -> Vec<u8> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    edit(&loro);
    loro.commit();
    let mut files = members(bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

type Registers = BTreeMap<String, LoroValue>;

/// The registers of every top node, as stored.
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

/// Opens `bytes` and checks that the open wrote nothing and a save keeps every register.
fn open_unchanged(bytes: &[u8]) -> Document {
    let opened = unpack(2, bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_nodes(&saved), raw_nodes(bytes), "saved back unchanged");
    opened
}

// Superseded by 0029 rework (2026-11-17): the goldens are rewritten for the one register
// `start` (ADR 0065 §6); format 48 is unreleased, so its number stays.
fn golden_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let first = laser(
        &d,
        &copy(json!({"air_assist": true, "output_format": "grbl"})),
    );
    d.set_fixture_material(first, Some(BIRCH)).unwrap();
    d.set_start(first, "home_then_origin").unwrap();
    let second = laser(&d, &copy(json!({"output_format": "grbl"})));
    d.set_start(second, "head_position").unwrap();
    let _third = laser(&d, &copy(json!({})));
    pack(&d, "0.1.0").unwrap()
}

#[test]
fn the_format_is_48() {
    assert_eq!(CURRENT_FORMAT_VERSION, 48);
}

fn has_register(values: &serde_json::Map<String, serde_json::Value>, field: &str) -> bool {
    values.keys().any(|key| key.ends_with(field))
}

#[test]
fn the_golden_keeps_the_pick_the_start_and_the_copy_values() {
    let bytes = read_or_write("laser_output_v48.curvyo", golden_bytes);
    assert_eq!(format_of(&bytes), 48);
    let d = open_unchanged(&bytes);
    let pages = d.manufacturings();
    assert_eq!(pages.len(), 3);
    let values = &pages[0].process_values;
    assert_eq!(values["x.curvyo.laser.fixture_material"], json!(BIRCH));
    assert_eq!(values["x.curvyo.laser.start"], json!("home_then_origin"));
    assert!(!has_register(values, "start_from") && !has_register(values, "home_first"));
    let profile = pages[0].profile.as_ref().unwrap();
    assert_eq!(profile["air_assist"], json!(true));
    assert_eq!(profile["output_format"], json!("grbl"));
    assert_eq!(
        pages[1].process_values["x.curvyo.laser.start"],
        json!("head_position")
    );
    let plain = &pages[2];
    assert!(
        !has_register(&plain.process_values, "fixture_material")
            && !has_register(&plain.process_values, "start"),
        "a page with no registers"
    );
    let profile = plain.profile.as_ref().unwrap();
    assert!(!profile.contains_key("air_assist") && !profile.contains_key("output_format"));
}

fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let first = laser(
        &d,
        &copy(json!({"air_assist": "on", "output_format": "marlin"})),
    );
    // A step between the two pages, so no two history rows share a label; the edit below
    // replaces the value it writes.
    d.set_start(first, "head_position").unwrap();
    laser(&d, &copy(json!({})));
    let bytes = pack(&d, "0.1.0").unwrap();
    edited(&bytes, |loro| {
        let tree = loro.get_tree("paths");
        let pages: Vec<_> = tree
            .children(TreeParentId::Root)
            .unwrap()
            .into_iter()
            .filter(|&node| tree.get_meta(node).unwrap().get("mfg_profile").is_some())
            .collect();
        let meta = tree.get_meta(pages[0]).unwrap();
        meta.insert("x.curvyo.laser.fixture_material", 7).unwrap();
        meta.insert("x.curvyo.laser.start", "middle").unwrap();
        let meta = tree.get_meta(pages[1]).unwrap();
        meta.insert("x.curvyo.laser.start", true).unwrap();
    })
}

#[test]
fn damaged_values_are_kept_as_stored_and_saved_back_unchanged() {
    let bytes = read_or_write("laser_output_damaged_v48.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 48);
    let d = open_unchanged(&bytes);
    let pages = d.manufacturings();
    let page = &pages[0];
    assert_eq!(
        page.process_values["x.curvyo.laser.fixture_material"],
        json!(7)
    );
    assert_eq!(page.process_values["x.curvyo.laser.start"], json!("middle"));
    assert_eq!(pages[1].process_values["x.curvyo.laser.start"], json!(true));
    let profile = page.profile.as_ref().unwrap();
    assert_eq!(profile["air_assist"], json!("on"));
    assert_eq!(profile["output_format"], json!("marlin"));
}

#[test]
fn a_file_of_the_first_build_keeps_its_two_registers_untouched() {
    // The registers of PR #371's build (`start_from`, `home_first`) are unknown process registers
    // now: they open, are not read (the laser extension reads Machine origin) and save back.
    let d = Document::new(1);
    laser(&d, &copy(json!({})));
    let bytes = edited(&pack(&d, "0.1.0").unwrap(), |loro| {
        let tree = loro.get_tree("paths");
        let page = tree
            .children(TreeParentId::Root)
            .unwrap()
            .into_iter()
            .find(|&node| tree.get_meta(node).unwrap().get("mfg_profile").is_some())
            .unwrap();
        let meta = tree.get_meta(page).unwrap();
        meta.insert("x.curvyo.laser.start_from", "head_position")
            .unwrap();
        meta.insert("x.curvyo.laser.home_first", true).unwrap();
    });
    let opened = open_unchanged(&bytes);
    let values = &opened.manufacturings()[0].process_values;
    assert_eq!(values["x.curvyo.laser.start_from"], json!("head_position"));
    assert_eq!(values["x.curvyo.laser.home_first"], json!(true));
    assert!(!has_register(values, ".start"));
}

#[test]
fn the_format_47_goldens_open_unchanged() {
    for name in [
        "fixture_sets_v47.curvyo",
        "fixture_sets_damaged_v47.curvyo",
        "fixture_sets_align_v47.curvyo",
    ] {
        let bytes = std::fs::read(fixture_path(name)).unwrap();
        assert_eq!(format_of(&bytes), 47, "{name}");
        let opened = unpack(2, &bytes).unwrap();
        assert_eq!(opened.own_step_count(), 0, "{name}: opening writes no step");
    }
}
