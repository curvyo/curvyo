//! Format 40 (`specs/0102-laser-fixtures` criteria 31 to 34, ADR 0049 §6 to §8): the goldens
//! `fixtures_v40.curvyo`, `fixtures_unknown_kind_v40.curvyo`, `fixtures_damaged_v40.curvyo`,
//! `fixtures_slot_orphan_v40.curvyo` and `fixtures_grid_v39.curvyo` read back every value and save
//! back unchanged; the `0093` golden of format 35 opens with its grid Active; a newer file is
//! refused. `CURVYO_WRITE_FIXTURES=1` rewrites the goldens.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty
)]

use std::collections::BTreeMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, Angle, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureAdd, FixtureParams,
    FixtureSnapshot, FixtureTurn, NewFixture, NewOutputPage, NodeId, OpenError, Point, Vec2, pack,
    unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
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

fn add(
    d: &Document,
    mfg: NodeId,
    add: FixtureAdd,
    kind: &str,
    values: serde_json::Value,
) -> NodeId {
    let word = kind.rsplit('/').next().unwrap();
    d.add_fixture(
        mfg,
        &NewFixture {
            add,
            kind,
            label: word,
            params: &params(values),
            matrix: (add != FixtureAdd::Grid).then(|| Affine::translation(Vec2::new(20.0, 20.0))),
        },
    )
    .unwrap()
}

fn slot(d: &Document, holder: NodeId, shape: &str, cx: f64) {
    let values = serde_json::json!({
        "shape": shape, "cx_mm": cx, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 15.0, "d_mm": 12.0
    });
    let values = params(values);
    let id = d.add_slot(holder, "Slot", &values).unwrap();
    // A name of its own, so no two adjacent steps share a label (the legacy-history oracle of
    // `0020`).
    d.set_slot(id, &format!("Pocket {cx}"), &values).unwrap();
}

fn board(d: &Document, mfg: NodeId) -> NodeId {
    let values = serde_json::json!({"width_mm": 100.0, "height_mm": 60.0});
    add(d, mfg, FixtureAdd::Holder, "curvyo.laser/holder", values)
}

/// One fixture of each of the five kinds, two holders with two slots each (one circle), an inactive grid, a
/// hidden clamp, and the second holder moved and turned by 30 degrees (criterion 31).
fn golden_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let grid = add(
        &d,
        mfg,
        FixtureAdd::Grid,
        "curvyo.laser/engraved_grid",
        serde_json::json!({
            "spacing_mm": 10.0, "major_every": 5, "tapes": ["bottom", "left"],
            "origin_x_mm": 0.0, "origin_y_mm": 0.0
        }),
    );
    d.set_fixture_active(grid, false).unwrap();
    let first = board(&d, mfg);
    slot(&d, first, "rect", 30.0);
    slot(&d, first, "circle", 70.0);
    let second = board(&d, mfg);
    slot(&d, second, "rect", 25.0);
    slot(&d, second, "rect", 75.0);
    let turned = Affine::translation(Vec2::new(300.0, 120.0)).compose(Affine::rotation_about(
        Point::new(0.0, 0.0),
        Angle::from_radians(30.0_f64.to_radians()),
    ));
    d.set_fixture_matrix(second, turned, FixtureTurn::Rotate)
        .unwrap();
    let clamp = add(
        &d,
        mfg,
        FixtureAdd::Clamp,
        "curvyo.laser/clamp",
        serde_json::json!({"width_mm": 40.0, "height_mm": 20.0}),
    );
    d.set_fixture_visible(clamp, false).unwrap();
    add(
        &d,
        mfg,
        FixtureAdd::Stop,
        "curvyo.laser/stop",
        serde_json::json!({"leg_a_mm": 60.0, "leg_b_mm": 60.0, "thickness_mm": 5.0}),
    );
    add(
        &d,
        mfg,
        FixtureAdd::Screw,
        "curvyo.laser/screw",
        serde_json::json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 10.0}),
    );
    d
}

fn unknown_kind_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    let table = add(
        &d,
        mfg,
        FixtureAdd::Holder,
        "curvyo.laser/turntable",
        serde_json::json!({"diameter_mm": 300.0, "steps": 8}),
    );
    slot(&d, table, "rect", 40.0);
    board(&d, mfg);
    d
}

/// A grid as `0093` wrote it: the schema's `{mm}` values, no `active`, no tapes.
fn grid_v39_document() -> Document {
    let d = Document::new(1);
    let mfg = laser(&d);
    add(
        &d,
        mfg,
        FixtureAdd::Grid,
        "curvyo.laser/engraved_grid",
        serde_json::json!({
            "spacing": {"mm": 10.0}, "line_width": {"mm": 0.1},
            "origin_x": {"mm": 0.0}, "origin_y": {"mm": 0.0}
        }),
    );
    d
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

fn loro_of(bytes: &[u8]) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&members(bytes)["document.loro"]).unwrap();
    loro
}

/// `bytes` with its document replaced by `loro`'s state after `edit`.
fn edited(bytes: &[u8], edit: impl Fn(&LoroDoc)) -> Vec<u8> {
    let loro = loro_of(bytes);
    edit(&loro);
    loro.commit();
    let mut files = members(bytes);
    files.insert(
        "document.loro".to_owned(),
        loro.export(loro::ExportMode::Snapshot).unwrap(),
    );
    rezip(&files)
}

fn with_format(bytes: &[u8], version: u32) -> Vec<u8> {
    let mut files = members(bytes);
    let mut manifest: serde_json::Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["format_version"] = serde_json::json!(version);
    files.insert(
        "manifest.json".to_owned(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    rezip(&files)
}

fn format_of(bytes: &[u8]) -> u64 {
    let json: serde_json::Value = serde_json::from_slice(&members(bytes)["manifest.json"]).unwrap();
    json["format_version"].as_u64().unwrap()
}

type Registers = BTreeMap<String, LoroValue>;

/// The registers of every node below each manufacturing, depth first, as stored.
fn raw_nodes(bytes: &[u8]) -> Vec<Registers> {
    fn walk(tree: &loro::LoroTree, node: loro::TreeID, out: &mut Vec<Registers>) {
        let mut registers = Registers::new();
        tree.get_meta(node).unwrap().for_each(|key, value| {
            registers.insert(key.to_owned(), value.into_value().unwrap());
        });
        out.push(registers);
        for child in tree.children(TreeParentId::Node(node)).unwrap_or_default() {
            walk(tree, child, out);
        }
    }
    let loro = loro_of(bytes);
    let tree = loro.get_tree("paths");
    let mut found = Vec::new();
    for top in tree.children(TreeParentId::Root).unwrap() {
        for child in tree.children(TreeParentId::Node(top)).unwrap_or_default() {
            walk(&tree, child, &mut found);
        }
    }
    found
}

/// The meta map of every node below the manufacturings whose register `key` is `value`.
fn nodes_with(loro: &LoroDoc, key: &str, value: &str) -> Vec<loro::LoroMap> {
    let tree = loro.get_tree("paths");
    let mut found = Vec::new();
    let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
    while let Some(node) = stack.pop() {
        let meta = tree.get_meta(node).unwrap();
        if meta.get(key).and_then(|v| v.into_value().ok()) == Some(LoroValue::from(value)) {
            found.push(meta);
        }
        stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
    }
    found
}

fn fixtures(d: &Document) -> Vec<FixtureSnapshot> {
    d.manufacturings()[0].fixtures.clone()
}

/// Opens `bytes` and checks that the open wrote nothing and a save keeps every register.
fn open_unchanged(bytes: &[u8]) -> Document {
    let opened = unpack(2, bytes).unwrap();
    assert_eq!(opened.own_step_count(), 0, "opening writes no step");
    let saved = pack(&opened, "0.1.0").unwrap();
    assert_eq!(raw_nodes(&saved), raw_nodes(bytes), "saved back unchanged");
    opened
}

#[test]
fn the_round_trip_golden_reads_back_every_value_state_slot_and_place() {
    let bytes = read_or_write("fixtures_v40.curvyo", || {
        pack(&golden_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 40);
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 41);
    let opened = open_unchanged(&bytes);
    let read = fixtures(&opened);
    assert_eq!(read, fixtures(&golden_document()));
    let kinds: Vec<&str> = read.iter().map(|f| f.kind.as_str()).collect();
    assert_eq!(
        kinds,
        [
            "curvyo.laser/engraved_grid",
            "curvyo.laser/holder",
            "curvyo.laser/holder",
            "curvyo.laser/clamp",
            "curvyo.laser/stop",
            "curvyo.laser/screw"
        ]
    );
    assert_eq!(read[5].params["thread_diameter_mm"], 6.0);
    assert_eq!(read[5].params["head_diameter_mm"], 10.0);
    assert!(read[5].slots.is_empty() && read[5].matrix.is_some());
    assert!(!read[0].active && read[0].visible, "the inactive grid");
    assert_eq!(read[0].matrix, None, "the grid has no matrix");
    assert!(read[3].active && !read[3].visible, "the hidden clamp");
    assert_eq!(read[1].slots.len(), 2);
    assert_eq!(read[1].slots[1].params["shape"], "circle");
    let turn = read[2].matrix.unwrap().to_array();
    assert!((turn[1] - 30.0_f64.to_radians().sin()).abs() < 1e-12);
    assert_eq!(turn[4..], [300.0, 120.0]);
    assert!(read.iter().all(|f| f.damaged == 0));
}

#[test]
fn an_unknown_kind_is_kept_with_its_values_and_slots_and_saved_back_unchanged() {
    let bytes = read_or_write("fixtures_unknown_kind_v40.curvyo", || {
        pack(&unknown_kind_document(), "0.1.0").unwrap()
    });
    assert_eq!(format_of(&bytes), 40);
    let read = fixtures(&open_unchanged(&bytes));
    assert_eq!(read[0].kind, "curvyo.laser/turntable");
    assert_eq!(read[0].params["steps"], 8);
    assert_eq!(read[0].slots.len(), 1);
    assert_eq!(read[1].kind, "curvyo.laser/holder");
}

/// The first holder's width made a text and its height a NaN, its matrix four numbers, one slot
/// without a shape and one whose values are not a map; a second holder's matrix scaled; a screw
/// with a text thread (criterion 33, ADR 0049 §8).
fn damaged_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    let holder = board(&d, mfg);
    slot(&d, holder, "rect", 30.0);
    slot(&d, holder, "rect", 70.0);
    slot(&d, holder, "circle", 50.0);
    board(&d, mfg);
    add(
        &d,
        mfg,
        FixtureAdd::Screw,
        "curvyo.laser/screw",
        serde_json::json!({"thread_diameter_mm": 6.0, "head_diameter_mm": 10.0}),
    );
    edited(&pack(&d, "0.1.0").unwrap(), |loro| {
        let named = |name: &str| nodes_with(loro, "name", name).remove(0);
        let scaled: Vec<LoroValue> = [2.0, 0.0, 0.0, 2.0, 20.0, 20.0]
            .into_iter()
            .map(LoroValue::Double)
            .collect();
        named("holder 2")
            .insert("fixture_matrix", LoroValue::from(scaled))
            .unwrap();
        let screw = std::collections::HashMap::from([
            ("thread_diameter_mm".to_owned(), LoroValue::from("M6")),
            ("head_diameter_mm".to_owned(), LoroValue::Double(10.0)),
        ]);
        named("screw 1")
            .insert("fixture_params", LoroValue::from(screw))
            .unwrap();
        let meta = &named("holder 1");
        let values = std::collections::HashMap::from([
            ("width_mm".to_owned(), LoroValue::from("wide")),
            ("height_mm".to_owned(), LoroValue::Double(f64::NAN)),
        ]);
        meta.insert("fixture_params", LoroValue::from(values))
            .unwrap();
        let four: Vec<LoroValue> = [1.0, 0.0, 0.0, 1.0]
            .into_iter()
            .map(LoroValue::Double)
            .collect();
        meta.insert("fixture_matrix", LoroValue::from(four))
            .unwrap();
        let slots = nodes_with(loro, "shape", "fixture_slot");
        let no_shape =
            std::collections::HashMap::from([("cx_mm".to_owned(), LoroValue::Double(5.0))]);
        slots[0]
            .insert("slot_params", LoroValue::from(no_shape))
            .unwrap();
        slots[1].insert("slot_params", "not a map").unwrap();
    })
}

#[test]
fn damaged_values_read_as_their_defaults_are_counted_and_nothing_is_rewritten() {
    let bytes = read_or_write("fixtures_damaged_v40.curvyo", damaged_bytes);
    assert_eq!(format_of(&bytes), 40);
    let read = fixtures(&open_unchanged(&bytes));
    let holder = &read[0];
    assert_eq!(
        holder.params["width_mm"], "wide",
        "a text is kept for the extension"
    );
    assert!(
        !holder.params.contains_key("height_mm"),
        "the NaN does not read"
    );
    assert_eq!(holder.matrix, None, "four numbers read as identity");
    assert_eq!(
        holder.slots.len(),
        2,
        "the slot whose values are not a map is skipped"
    );
    assert!(
        !holder
            .slots
            .iter()
            .any(|s| s.params.contains_key("shape") && s.params.len() == 1)
    );
    assert_eq!(
        holder.damaged, 3,
        "the NaN, the matrix and the skipped slot"
    );
    assert_eq!(read[1].matrix, None, "a scaled matrix reads as identity");
    assert_eq!(read[1].damaged, 1, "the scaled matrix");
    assert!(
        !read[2].params.contains_key("thread_diameter_mm"),
        "the screw's text thread reads as its default"
    );
    assert_eq!(read[2].damaged, 1, "the text thread is counted");
    assert_eq!(read[2].kind, "curvyo.laser/screw");
}

/// A `fixture_slot` node directly under the manufacturing, outside any holder.
fn orphan_bytes() -> Vec<u8> {
    let d = Document::new(1);
    let mfg = laser(&d);
    board(&d, mfg);
    edited(&pack(&d, "0.1.0").unwrap(), |loro| {
        let tree = loro.get_tree("paths");
        let manufacturing = tree
            .children(TreeParentId::Root)
            .unwrap()
            .into_iter()
            .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
            .unwrap();
        let orphan = tree.create(TreeParentId::Node(manufacturing)).unwrap();
        let meta = tree.get_meta(orphan).unwrap();
        meta.insert("shape", "fixture_slot").unwrap();
        meta.insert("name", "Lost slot").unwrap();
        let values =
            std::collections::HashMap::from([("shape".to_owned(), LoroValue::from("rect"))]);
        meta.insert("slot_params", LoroValue::from(values)).unwrap();
    })
}

#[test]
fn an_orphan_slot_is_kept_never_read_and_saved_back_unchanged() {
    let bytes = read_or_write("fixtures_slot_orphan_v40.curvyo", orphan_bytes);
    assert_eq!(format_of(&bytes), 40);
    assert_eq!(raw_nodes(&bytes).len(), 2, "the holder and the orphan");
    let read = fixtures(&open_unchanged(&bytes));
    assert_eq!(read.len(), 1);
    assert!(read[0].slots.is_empty());
    assert_eq!(read[0].damaged, 0);
}

#[test]
fn a_format_39_grid_opens_active_with_its_values_and_nothing_written() {
    let bytes = read_or_write("fixtures_grid_v39.curvyo", || {
        with_format(&pack(&grid_v39_document(), "0.1.0").unwrap(), 39)
    });
    assert_eq!(format_of(&bytes), 39);
    let opened = open_unchanged(&bytes);
    let grid = &fixtures(&opened)[0];
    assert!(grid.active, "absent active reads as on");
    assert_eq!(grid.params["spacing"], serde_json::json!({"mm": 10.0}));
    assert!(!grid.params.contains_key("tapes"));
    assert_eq!(grid.damaged, 0);
}

#[test]
fn the_0093_golden_of_format_35_opens_with_its_grids_active() {
    let bytes = std::fs::read(fixture_path("laser_workbench_v35.curvyo")).unwrap();
    assert_eq!(format_of(&bytes), 35);
    let opened = unpack(2, &bytes).unwrap();
    let read = fixtures(&opened);
    assert!(!read.is_empty());
    assert!(
        read.iter()
            .all(|f| f.active && f.slots.is_empty() && f.matrix.is_none())
    );
    assert!(read.iter().all(|f| f.damaged == 0));
}

#[test]
fn a_file_of_a_newer_format_is_refused() {
    let bytes = with_format(
        &read_or_write("fixtures_v40.curvyo", || {
            pack(&golden_document(), "0.1.0").unwrap()
        }),
        CURRENT_FORMAT_VERSION + 1,
    );
    assert!(matches!(
        unpack(2, &bytes),
        Err(OpenError::FormatTooNew { found, supported })
            if found == CURRENT_FORMAT_VERSION + 1 && supported == CURRENT_FORMAT_VERSION
    ));
}

#[test]
fn a_fixture_holding_anything_but_slots_is_refused_as_damaged() {
    let bytes = edited(&pack(&golden_document(), "0.1.0").unwrap(), |loro| {
        let tree = loro.get_tree("paths");
        let mut stack: Vec<loro::TreeID> = tree.children(TreeParentId::Root).unwrap();
        while let Some(node) = stack.pop() {
            let meta = tree.get_meta(node).unwrap();
            if meta.get("shape").and_then(|v| v.into_value().ok())
                == Some(LoroValue::from("fixture"))
            {
                let child = tree.create(TreeParentId::Node(node)).unwrap();
                tree.get_meta(child)
                    .unwrap()
                    .insert("shape", "rect")
                    .unwrap();
                return;
            }
            stack.extend(tree.children(TreeParentId::Node(node)).unwrap_or_default());
        }
    });
    assert!(matches!(unpack(2, &bytes), Err(OpenError::Damaged)));
}
