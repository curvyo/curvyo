//! Independent acceptance tests for `specs/0118-machine-control` slice 2c, document part (criteria
//! 29, 30 and 33; `adrs.md` O1 to O3, O9; ux-notes W41, W44, W45), written from the specification
//! and the ADR before reading the implementation. They use only the public document API.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::missing_panics_doc)]

use std::io::{Cursor, Read};

use curvyo_document_core::{
    Document, DocumentSize, FixtureParams, NewOutputPage, NodeId, OperationError, PageContents,
    PageCorner, ProfileChange, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::json;

const ORIGIN: &str = "x.curvyo.laser.user_origin_um";
const START: &str = "x.curvyo.laser.start";
const DIODE: &str = "6f1c2b0a-5d4e-4f3a-9b2c-1d0e8f7a6b5c";
const K40: &str = "7a2d3c1b-6e5f-4a4b-8c3d-2e1f9a8b7c6d";

fn record(id: &str, name: &str) -> FixtureParams {
    serde_json::from_value(json!({
        "id": id,
        "name": name,
        "origin": "bottom_left",
        "max_feed_mm_per_min": 6000,
        "max_power": 1000,
        "laser_mode": true,
        "motorised_z": false,
    }))
    .unwrap()
}

/// A new laser page on a 400 x 300 mm machine of the diode record.
fn laser_page(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(&record(DIODE, "Diode 10 W")),
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn register(d: &Document, m: NodeId, key: &str) -> Option<serde_json::Value> {
    d.manufacturing(m).unwrap().process_values.get(key).cloned()
}

/// The page's Start; an absent register is the default, Machine origin.
fn start_of(d: &Document, m: NodeId) -> serde_json::Value {
    register(d, m, START).unwrap_or(json!("machine_origin"))
}

fn last_step_count(d: &Document) -> usize {
    d.own_step_count()
}

fn undo_last(d: &Document) {
    let last = d.own_steps().last().unwrap().step;
    d.undo_step(last).unwrap();
}

fn pick_profile(d: &Document, m: NodeId, profile: &FixtureParams, change: ProfileChange) -> bool {
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

// Criterion 29: Set origin here stores the point and picks User origin in one undo step.

#[test]
fn c29_set_origin_stores_the_point_and_picks_user_origin_in_one_undo_step() {
    let d = Document::new(1);
    let m = laser_page(&d);
    assert_eq!(Some(start_of(&d, m)), Some(json!("machine_origin")));
    let before = last_step_count(&d);
    assert_eq!(d.set_user_origin(m, [120_000, 85_500]), Ok(true));
    assert_eq!(last_step_count(&d), before + 1, "exactly one step");
    assert_eq!(register(&d, m, ORIGIN), Some(json!([120_000, 85_500])));
    assert_eq!(Some(start_of(&d, m)), Some(json!("user_origin")));
    undo_last(&d);
    assert_eq!(
        register(&d, m, ORIGIN),
        None,
        "the first point goes with its undo"
    );
    assert_eq!(
        Some(start_of(&d, m)),
        Some(json!("machine_origin")),
        "and the pick returns in the same undo"
    );
}

#[test]
fn c29_set_origin_at_the_stored_point_with_user_origin_picked_writes_no_step() {
    let d = Document::new(1);
    let m = laser_page(&d);
    d.set_user_origin(m, [120_000, 85_500]).unwrap();
    let before = last_step_count(&d);
    assert_eq!(d.set_user_origin(m, [120_000, 85_500]), Ok(false));
    assert_eq!(last_step_count(&d), before, "no step for the same point");
}

#[test]
fn c29_set_origin_moves_the_point_without_touching_other_pages() {
    let d = Document::new(1);
    let first = laser_page(&d);
    let second = laser_page(&d);
    d.set_user_origin(second, [1_000, 2_000]).unwrap();
    d.set_user_origin(first, [5_000, 6_000]).unwrap();
    assert_eq!(register(&d, second, ORIGIN), Some(json!([1_000, 2_000])));
    assert_eq!(register(&d, first, ORIGIN), Some(json!([5_000, 6_000])));
}

// Criterion 30: User origin is picked only with a stored point; another pick keeps the point.

#[test]
fn c30_user_origin_cannot_be_picked_before_a_point_is_stored() {
    let d = Document::new(1);
    let m = laser_page(&d);
    let before = last_step_count(&d);
    assert_eq!(
        d.set_start(m, "user_origin"),
        Err(OperationError::NoUserOrigin)
    );
    assert_eq!(last_step_count(&d), before, "a refused pick writes no step");
    assert_eq!(Some(start_of(&d, m)), Some(json!("machine_origin")));
}

#[test]
fn c30_another_pick_keeps_the_point_and_user_origin_can_be_picked_again() {
    let d = Document::new(1);
    let m = laser_page(&d);
    d.set_user_origin(m, [5_000, 6_000]).unwrap();
    for other in ["head_position", "home_then_origin", "machine_origin"] {
        d.set_start(m, other).unwrap();
        assert_eq!(
            register(&d, m, ORIGIN),
            Some(json!([5_000, 6_000])),
            "{other}"
        );
    }
    d.set_start(m, "user_origin").unwrap();
    assert_eq!(Some(start_of(&d, m)), Some(json!("user_origin")));
    assert_eq!(register(&d, m, ORIGIN), Some(json!([5_000, 6_000])));
}

// Criterion 30 with Frage 345 C: another machine profile returns the pick, keeps the point.

#[test]
fn c30_another_profile_returns_start_to_machine_origin_in_one_step_and_keeps_the_point() {
    let d = Document::new(1);
    let m = laser_page(&d);
    d.set_user_origin(m, [5_000, 6_000]).unwrap();
    let before = last_step_count(&d);
    assert!(pick_profile(
        &d,
        m,
        &record(K40, "K40"),
        ProfileChange::Pick
    ));
    assert_eq!(
        last_step_count(&d),
        before + 1,
        "one step for the profile pick"
    );
    assert_eq!(Some(start_of(&d, m)), Some(json!("machine_origin")));
    assert_eq!(register(&d, m, ORIGIN), Some(json!([5_000, 6_000])));
    undo_last(&d);
    assert_eq!(
        Some(start_of(&d, m)),
        Some(json!("user_origin")),
        "undo restores the pick"
    );
    assert_eq!(register(&d, m, ORIGIN), Some(json!([5_000, 6_000])));
}

#[test]
fn c30_editing_the_same_profile_keeps_the_pick_and_the_point() {
    let d = Document::new(1);
    let m = laser_page(&d);
    d.set_user_origin(m, [5_000, 6_000]).unwrap();
    let mut edited = record(DIODE, "Diode 10 W");
    edited.insert("origin".to_owned(), json!("top_left"));
    assert!(pick_profile(&d, m, &edited, ProfileChange::EditOrigin));
    assert_eq!(Some(start_of(&d, m)), Some(json!("user_origin")));
    assert_eq!(register(&d, m, ORIGIN), Some(json!([5_000, 6_000])));
    let mut renamed = record(DIODE, "Diode, renamed");
    renamed.insert("max_power".to_owned(), json!(900));
    assert!(pick_profile(&d, m, &renamed, ProfileChange::EditMaxPower));
    assert_eq!(Some(start_of(&d, m)), Some(json!("user_origin")));
    assert_eq!(register(&d, m, ORIGIN), Some(json!([5_000, 6_000])));
}

// Criterion 33: this slice stores exactly one point and writes nothing to the profile.

#[test]
fn c33_one_point_per_page_and_the_profile_record_is_not_written_by_set_origin() {
    let d = Document::new(1);
    let m = laser_page(&d);
    let profile_before = d.manufacturing(m).unwrap().process_values.clone();
    d.set_user_origin(m, [7_000, 8_000]).unwrap();
    d.set_user_origin(m, [9_000, 1_000]).unwrap();
    let values = d.manufacturing(m).unwrap().process_values;
    assert_eq!(
        values[ORIGIN],
        json!([9_000, 1_000]),
        "the typed pair replaces it"
    );
    for key in profile_before.keys() {
        if key != START && key != ORIGIN {
            assert_eq!(values[key], profile_before[key], "{key} unchanged");
        }
    }
}

// Format 61: the point round-trips, a damaged value is kept and saved back unchanged.

fn members(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
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
    let manifest: serde_json::Value =
        serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    manifest["format_version"].as_u64().unwrap()
}

fn nodes(bytes: &[u8]) -> Vec<std::collections::BTreeMap<String, LoroValue>> {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    tree.children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .map(|node| {
            let mut map = std::collections::BTreeMap::new();
            tree.get_meta(node).unwrap().for_each(|key, value| {
                map.insert(key.to_owned(), value.into_value().unwrap());
            });
            map
        })
        .collect()
}

#[test]
fn format_61_round_trip_keeps_the_point_and_the_pick() {
    let d = Document::new(1);
    let first = laser_page(&d);
    d.set_user_origin(first, [123_456, 78_900]).unwrap();
    let second = laser_page(&d);
    d.set_user_origin(second, [1_000, 2_000]).unwrap();
    d.set_start(second, "machine_origin").unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();
    assert_eq!(format_of(&bytes), 63);
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let pages = opened.manufacturings();
    assert_eq!(pages[0].process_values[ORIGIN], json!([123_456, 78_900]));
    assert_eq!(pages[0].process_values[START], json!("user_origin"));
    assert_eq!(pages[1].process_values[ORIGIN], json!([1_000, 2_000]));
    assert_eq!(pages[1].process_values[START], json!("machine_origin"));
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(nodes(&saved), nodes(&bytes), "saved back unchanged");
}

#[test]
fn format_61_a_damaged_point_is_kept_as_stored_and_saved_back_unchanged() {
    let d = Document::new(1);
    let m = laser_page(&d);
    d.set_user_origin(m, [1_000, 2_000]).unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();
    // Replace the stored point with a value of the wrong shape, as a damaged file has it.
    let loro = LoroDoc::new();
    loro.import(&members(&bytes)["document.loro"]).unwrap();
    let tree = loro.get_tree("paths");
    let page = tree
        .children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&node| tree.get_meta(node).unwrap().get(ORIGIN).is_some())
        .unwrap();
    tree.get_meta(page).unwrap().insert(ORIGIN, "12,3").unwrap();
    loro.commit();
    let mut files = members(&bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, content) in &files {
        use std::io::Write;
        writer
            .start_file(name.as_str(), zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(content).unwrap();
    }
    let damaged = writer.finish().unwrap().into_inner();
    let opened = unpack(2, &damaged).unwrap();
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(
        nodes(&saved),
        nodes(&damaged),
        "kept as stored, not rewritten"
    );
}

#[test]
fn format_61_is_the_current_format_and_a_picked_start_survives_open() {
    const { assert!(curvyo_document_core::CURRENT_FORMAT_VERSION >= 61) };
    let d = Document::new(1);
    let m = laser_page(&d);
    d.set_start(m, "head_position").unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();
    let opened = unpack(2, &bytes).unwrap();
    assert_eq!(
        opened.manufacturings()[0].process_values[START],
        json!("head_position")
    );
}
