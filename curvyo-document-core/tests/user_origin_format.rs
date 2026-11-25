//! Format 61 (`specs/0118-machine-control` criteria 29, 30, 33; `adrs.md` O1 to O3, O9): the page's
//! user origin `x.curvyo.laser.user_origin_um` and the Start spelling `user_origin`. The command
//! writes the point and Start in one step, the goldens read back and save unchanged, and a profile
//! pick of another record returns Start to Machine origin (Frage 345 C). The laser extension's
//! reading of each value is tested in `curvyo-laser-core` (`tests/user_origin_0118.rs`).
//! `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOutputPage, NodeId,
    OperationError, PageContents, PageCorner, ProfileChange, SET_START_USER_ORIGIN_COMMIT_LABEL,
    SET_USER_ORIGIN_COMMIT_LABEL, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::json;

const ORIGIN: &str = "x.curvyo.laser.user_origin_um";
const START: &str = "x.curvyo.laser.start";
const DIODE: &str = "6f1c2b0a-5d4e-4f3a-9b2c-1d0e8f7a6b5c";
const K40: &str = "7a2d3c1b-6e5f-4a4b-8c3d-2e1f9a8b7c6d";

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn copy(id: &str, name: &str) -> FixtureParams {
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

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: Some(&copy(DIODE, "Diode 10 W")),
        work_area: DocumentSize::from_mm(400.0, 300.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn registers(d: &Document, m: NodeId) -> serde_json::Map<String, serde_json::Value> {
    d.manufacturing(m).unwrap().process_values
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo(d: &Document) {
    let last = d.own_steps().last().unwrap().step;
    d.undo_step(last).unwrap();
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

fn format_of(bytes: &[u8]) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
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
fn c29_set_user_origin_writes_the_point_and_start_in_one_step_and_one_undo_takes_both_back() {
    let d = Document::new(1);
    let m = laser(&d);
    d.set_start(m, "head_position").unwrap();
    let steps = d.own_step_count();
    assert_eq!(d.set_user_origin(m, [123_456, 78_900]), Ok(true));
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert_eq!(last_label(&d), SET_USER_ORIGIN_COMMIT_LABEL);
    let values = registers(&d, m);
    assert_eq!(values[ORIGIN], json!([123_456, 78_900]));
    assert_eq!(values[START], json!("user_origin"));
    assert_eq!(
        d.set_user_origin(m, [123_456, 78_900]),
        Ok(false),
        "the same point with User origin picked writes no step"
    );
    undo(&d);
    let values = registers(&d, m);
    assert!(
        !values.contains_key(ORIGIN),
        "the first point goes with its undo"
    );
    assert_eq!(
        values[START],
        json!("head_position"),
        "and the pick returns"
    );
}

#[test]
fn c29_the_typed_pair_changes_only_the_point() {
    let d = Document::new(1);
    let m = laser(&d);
    d.set_user_origin(m, [123_456, 78_900]).unwrap();
    assert_eq!(d.set_user_origin(m, [1_000, 2_000]), Ok(true));
    assert_eq!(registers(&d, m)[ORIGIN], json!([1_000, 2_000]));
    undo(&d);
    let values = registers(&d, m);
    assert_eq!(values[ORIGIN], json!([123_456, 78_900]));
    assert_eq!(values[START], json!("user_origin"));
}

#[test]
fn c29_a_point_beyond_ten_metres_is_refused() {
    let d = Document::new(1);
    let m = laser(&d);
    let steps = d.own_step_count();
    for point in [[10_000_001, 0], [0, -10_000_001]] {
        assert_eq!(
            d.set_user_origin(m, point),
            Err(OperationError::BadUserOrigin)
        );
    }
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(d.set_user_origin(m, [10_000_000, -10_000_000]), Ok(true));
}

#[test]
fn c30_user_origin_is_picked_only_with_a_point_and_another_pick_keeps_it() {
    let d = Document::new(1);
    let m = laser(&d);
    assert_eq!(
        d.set_start(m, "user_origin"),
        Err(OperationError::NoUserOrigin)
    );
    d.set_user_origin(m, [5_000, 6_000]).unwrap();
    assert_eq!(d.set_start(m, "machine_origin"), Ok(true));
    assert_eq!(registers(&d, m)[ORIGIN], json!([5_000, 6_000]), "kept");
    assert_eq!(d.set_start(m, "user_origin"), Ok(true));
    assert_eq!(last_label(&d), SET_START_USER_ORIGIN_COMMIT_LABEL);
    assert_eq!(registers(&d, m)[START], json!("user_origin"));
}

fn pick(d: &Document, m: NodeId, profile: &FixtureParams, change: ProfileChange) -> bool {
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

#[test]
fn frage_345_another_profile_returns_start_to_machine_origin_in_the_same_step_and_keeps_the_point()
{
    let d = Document::new(1);
    let m = laser(&d);
    d.set_user_origin(m, [5_000, 6_000]).unwrap();
    let steps = d.own_step_count();
    assert!(pick(&d, m, &copy(K40, "K40"), ProfileChange::Pick));
    assert_eq!(d.own_step_count(), steps + 1, "one step with the pick");
    let values = registers(&d, m);
    assert_eq!(values[START], json!("machine_origin"));
    assert_eq!(values[ORIGIN], json!([5_000, 6_000]));
    undo(&d);
    assert_eq!(
        registers(&d, m)[START],
        json!("user_origin"),
        "undo restores both"
    );
}

#[test]
fn frage_345_an_edit_or_a_pick_of_the_same_record_keeps_user_origin() {
    let d = Document::new(1);
    let m = laser(&d);
    d.set_user_origin(m, [5_000, 6_000]).unwrap();
    assert!(pick(
        &d,
        m,
        &copy(DIODE, "Diode 10 W renamed"),
        ProfileChange::Pick
    ));
    assert_eq!(registers(&d, m)[START], json!("user_origin"));
    let mut edited = copy(DIODE, "Diode 10 W renamed");
    edited.insert("origin".to_owned(), json!("top_left"));
    for change in [
        ProfileChange::EditOrigin,
        ProfileChange::Reset,
        ProfileChange::SaveAs,
    ] {
        let mut next = edited.clone();
        next.insert("max_power".to_owned(), json!(990 + d.own_step_count()));
        assert!(pick(&d, m, &next, change));
        assert_eq!(registers(&d, m)[START], json!("user_origin"), "{change:?}");
    }
}

fn golden_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let first = laser(&d);
    d.set_user_origin(first, [123_456, 78_900]).unwrap();
    let second = laser(&d);
    d.set_user_origin(second, [1_000, 2_000]).unwrap();
    d.set_start(second, "machine_origin").unwrap();
    let _third = laser(&d);
    pack(&d, "0.1.0").unwrap()
}

#[test]
fn c29_the_golden_keeps_the_point_and_the_start() {
    let bytes = read_or_write("user_origin_v61.curvyo", golden_bytes);
    assert_eq!(format_of(&bytes), 61);
    let d = open_unchanged(&bytes);
    let pages = d.manufacturings();
    assert_eq!(pages.len(), 3);
    assert_eq!(pages[0].process_values[ORIGIN], json!([123_456, 78_900]));
    assert_eq!(pages[0].process_values[START], json!("user_origin"));
    assert_eq!(pages[1].process_values[ORIGIN], json!([1_000, 2_000]));
    assert_eq!(pages[1].process_values[START], json!("machine_origin"));
    assert!(!pages[2].process_values.contains_key(ORIGIN));
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

fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    for _ in 0..4 {
        let m = laser(&d);
        // A step between the pages, so no two history rows share a label.
        d.set_start(m, "head_position").unwrap();
    }
    edited(&pack(&d, "0.1.0").unwrap(), |loro| {
        let tree = loro.get_tree("paths");
        let pages: Vec<_> = tree
            .children(TreeParentId::Root)
            .unwrap()
            .into_iter()
            .filter(|&node| tree.get_meta(node).unwrap().get("mfg_profile").is_some())
            .collect();
        let values: [LoroValue; 3] = [
            vec![LoroValue::Double(1.5), LoroValue::I64(2)].into(),
            vec![LoroValue::I64(1), LoroValue::I64(2), LoroValue::I64(3)].into(),
            "12,3".into(),
        ];
        for (page, value) in pages.iter().zip(values) {
            let meta = tree.get_meta(*page).unwrap();
            meta.insert(ORIGIN, value).unwrap();
            meta.insert(START, "user_origin").unwrap();
        }
        let meta = tree.get_meta(pages[3]).unwrap();
        meta.insert(START, "user_origin").unwrap();
    })
}

#[test]
fn c29_damaged_points_are_kept_as_stored_and_saved_back_unchanged() {
    let bytes = read_or_write("user_origin_damaged_v61.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 61);
    let d = open_unchanged(&bytes);
    let pages = d.manufacturings();
    assert_eq!(pages[0].process_values[ORIGIN], json!([1.5, 2]));
    assert_eq!(pages[1].process_values[ORIGIN], json!([1, 2, 3]));
    assert_eq!(pages[2].process_values[ORIGIN], json!("12,3"));
    assert!(!pages[3].process_values.contains_key(ORIGIN));
    for page in &pages {
        assert_eq!(page.process_values[START], json!("user_origin"));
    }
}

#[test]
fn the_format_is_61() {
    // Format 61 since `specs/0118-machine-control` slice 2c (`adrs.md` O3).
    const { assert!(CURRENT_FORMAT_VERSION >= 61) };
}
