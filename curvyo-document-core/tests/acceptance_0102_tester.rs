//! Independent acceptance tests for `specs/0102-laser-fixtures` at the document level: format 40
//! (criteria 31 to 34, ADR 0049 §2 to §8), the goldens of `tests/fixtures/`, the older goldens that
//! must open unchanged, a format 41 file refused, the fixture commands of the document, and the
//! round trip of a laser page with one fixture of each kind. Written from the specification and
//! ADR 0049 before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::items_after_statements,
    clippy::len_zero,
    clippy::bool_assert_comparison,
    clippy::too_many_lines
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureAdd, FixtureParams, NewFixture,
    NewOutputPage, NodeId, OpenError, Vec2, pack, unpack,
};
use loro::{ExportMode, LoroDoc};

fn golden(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("golden {name}: {e}"))
}

fn params(v: serde_json::Value) -> FixtureParams {
    serde_json::from_value(v).unwrap()
}

fn laser_page(d: &Document) -> NodeId {
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

/// The document's manufacturing as a comparable text: every fixture value, slot and matrix.
fn state(d: &Document) -> String {
    format!("{:?}", d.manufacturings())
}

fn round_trip(d: &Document) -> Document {
    unpack(4242, &pack(d, "0.1.0").unwrap()).unwrap()
}

/// Rewrites a saved file so its format is `to`: the manifest text and the root map of the
/// snapshot both carry the version, as the reader checks them.
fn with_format(bytes: &[u8], to: i64) -> Vec<u8> {
    let mut zin = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    for i in 0..zin.len() {
        let mut file = zin.by_index(i).unwrap();
        let name = file.name().unwrap().into_owned();
        let mut data = Vec::new();
        file.read_to_end(&mut data).unwrap();
        if name.ends_with(".loro") {
            let doc = LoroDoc::new();
            doc.import(&data).unwrap();
            doc.get_map("root").insert("format_version", to).unwrap();
            data = doc.export(ExportMode::Snapshot).unwrap();
        } else {
            let text = String::from_utf8(data).unwrap();
            let from = CURRENT_FORMAT_VERSION;
            let text = text
                .replace(
                    &format!("\"format_version\": {from}"),
                    &format!("\"format_version\": {to}"),
                )
                .replace(
                    &format!("\"format_version\":{from}"),
                    &format!("\"format_version\":{to}"),
                );
            data = text.into_bytes();
        }
        out.start_file(name, options).unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

// ---- criterion 31, format 40 ---------------------------------------------------------------------

#[test]
fn ac31_the_format_is_40_and_a_new_file_says_so() {
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    // Format 43 since `specs/0028-material-test-library` (ADR 0054 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 43);
    let d = Document::new(1);
    let bytes = pack(&d, "0.1.0").unwrap();
    let mut zin = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut found = false;
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).unwrap();
        let mut t = String::new();
        if f.name().unwrap().ends_with(".json") {
            f.read_to_string(&mut t).unwrap();
            if t.contains("\"format_version\"") {
                assert!(
                    t.contains("\"format_version\": 43") || t.contains("\"format_version\":43")
                );
                found = true;
            }
        }
    }
    assert!(found, "the manifest carries the format");
}

#[test]
fn ac31_golden_v40_reads_every_value_and_round_trips_unchanged() {
    let d = unpack(1, &golden("fixtures_v40.curvyo")).expect("fixtures_v40 opens");
    let m = &d.manufacturings()[0];
    assert_eq!(
        m.fixtures.len(),
        6,
        "one of each of the five kinds, the second holder included"
    );
    let kinds: Vec<&str> = m.fixtures.iter().map(|f| f.kind.as_str()).collect();
    assert!(kinds.contains(&"curvyo.laser/screw"));
    assert!(kinds.contains(&"curvyo.laser/engraved_grid"));
    assert!(kinds.contains(&"curvyo.laser/clamp"));
    assert!(kinds.contains(&"curvyo.laser/stop"));
    assert_eq!(
        kinds
            .iter()
            .filter(|k| **k == "curvyo.laser/holder")
            .count(),
        2
    );
    let grid = m
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("engraved_grid"))
        .unwrap();
    assert!(!grid.active, "the inactive grid reads inactive");
    let clamp = m
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("clamp"))
        .unwrap();
    assert!(
        !clamp.visible,
        "the hidden clamp reads hidden and stays active"
    );
    assert!(clamp.active);
    let holders: Vec<_> = m
        .fixtures
        .iter()
        .filter(|f| f.kind.ends_with("holder"))
        .collect();
    let slot_total: usize = holders.iter().map(|h| h.slots.len()).sum();
    assert_eq!(slot_total, 4, "two holders with two slots each");
    assert!(
        holders.iter().any(|h| h.slots.len() == 2
            && h.slots
                .iter()
                .any(|s| { format!("{s:?}").to_lowercase().contains("circle") })),
        "one circle slot"
    );
    let turned = holders
        .iter()
        .find(|h| h.matrix.is_some_and(|m| m.a.abs() < 0.99))
        .expect("the turned holder");
    let a = turned.matrix.unwrap();
    assert!(
        (a.a - 30f64.to_radians().cos()).abs() < 1e-6,
        "turned by 30 degrees"
    );

    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(
        state(&again),
        before,
        "every value survives save and reopen"
    );
}

#[test]
fn ac31_a_laser_page_with_one_of_each_kind_round_trips() {
    let d = Document::new(7);
    let mfg = laser_page(&d);
    let grid = d
        .add_fixture(
            mfg,
            &NewFixture {
                add: FixtureAdd::Grid,
                kind: "curvyo.laser/engraved_grid",
                label: "Grid 1",
                params: &params(serde_json::json!({
                    "spacing_mm": 10.0, "major_every": 5,
                    "tapes": ["bottom", "left", "top", "right"],
                    "origin_x_mm": 0.0, "origin_y_mm": 0.0
                })),
                matrix: None,
            },
        )
        .unwrap();
    let holder = d
        .add_fixture(
            mfg,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: "curvyo.laser/holder",
                label: "Holder 1",
                params: &params(serde_json::json!({ "width_mm": 100.0, "height_mm": 60.0 })),
                matrix: Some(Affine::translation(Vec2::new(20.0, 20.0))),
            },
        )
        .unwrap();
    d.add_slot(
        holder,
        "Slot 1",
        &params(serde_json::json!({
            "shape": "circle", "cx_mm": 30.0, "cy_mm": 30.0, "w_mm": 0.0, "h_mm": 0.0, "d_mm": 12.0
        })),
    )
    .unwrap();
    d.add_slot(
        holder,
        "Slot 2",
        &params(serde_json::json!({
            "shape": "rect", "cx_mm": 70.0, "cy_mm": 30.0, "w_mm": 20.0, "h_mm": 10.0, "d_mm": 0.0
        })),
    )
    .unwrap();
    d.add_fixture(
        mfg,
        &NewFixture {
            add: FixtureAdd::Clamp,
            kind: "curvyo.laser/clamp",
            label: "Clamp 1",
            params: &params(serde_json::json!({ "width_mm": 40.0, "height_mm": 20.0 })),
            matrix: Some(Affine::translation(Vec2::new(200.0, 100.0))),
        },
    )
    .unwrap();
    d.add_fixture(
        mfg,
        &NewFixture {
            add: FixtureAdd::Stop,
            kind: "curvyo.laser/stop",
            label: "Stop 1",
            params: &params(
                serde_json::json!({ "leg_a_mm": 60.0, "leg_b_mm": 60.0, "thickness_mm": 5.0 }),
            ),
            matrix: Some(Affine::translation(Vec2::new(300.0, 100.0))),
        },
    )
    .unwrap();
    d.set_fixture_active(grid, false).unwrap();
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(state(&again), before);
    assert_eq!(again.manufacturings()[0].fixtures.len(), 4);
    assert_eq!(again.manufacturings()[0].fixtures[1].slots.len(), 2);
}

// ---- criteria 32 to 34: the other goldens --------------------------------------------------------

#[test]
fn ac32_unknown_kind_is_kept_with_its_values_and_saved_back_unchanged() {
    let d = unpack(2, &golden("fixtures_unknown_kind_v40.curvyo")).expect("opens");
    let unknown: Vec<_> = d.manufacturings()[0]
        .fixtures
        .iter()
        .filter(|f| {
            !f.kind.starts_with("curvyo.laser/engraved")
                && !["holder", "clamp", "stop"]
                    .iter()
                    .any(|k| f.kind.ends_with(k))
        })
        .cloned()
        .collect();
    assert_eq!(unknown.len(), 1, "one unknown fixture is read");
    assert!(!unknown[0].kind.is_empty());
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(
        state(&again),
        before,
        "kept with all values, saved back unchanged"
    );
}

#[test]
fn ac33_damaged_values_read_as_defaults_and_are_counted() {
    let d = unpack(3, &golden("fixtures_damaged_v40.curvyo")).expect("opens");
    let m = &d.manufacturings()[0];
    let damaged: usize = m.fixtures.iter().map(|f| f.damaged).sum();
    assert!(
        damaged >= 3,
        "text, NaN, a slot without shape and a 4-entry matrix count: {damaged}"
    );
    let holder = m
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("holder") && f.damaged >= 1)
        .expect("the damaged holder reads with a count");
    assert!(
        holder.slots.len() < 3,
        "a damaged slot is skipped, not drawn"
    );
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(
        state(&again),
        before,
        "the reader never rewrites a value it read as a default"
    );
}

#[test]
fn ac33_orphan_slot_is_kept_and_not_read_as_a_slot_of_any_fixture() {
    let d = unpack(4, &golden("fixtures_slot_orphan_v40.curvyo")).expect("opens");
    let total: usize = d
        .manufacturings()
        .iter()
        .flat_map(|m| m.fixtures.iter())
        .map(|f| f.slots.len())
        .sum();
    assert_eq!(total, 0, "the orphan is skipped, not drawn");
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(state(&again), before);
}

#[test]
fn ac34_a_v39_grid_opens_active_with_its_four_tapes_and_unchanged_lines() {
    let d = unpack(5, &golden("fixtures_grid_v39.curvyo")).expect("v39 opens");
    let m = &d.manufacturings()[0];
    let grid = m
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("engraved_grid"))
        .expect("the grid");
    assert!(grid.active, "absent active reads as on");
    assert!(grid.visible);
    assert_eq!(grid.damaged, 0);
    // The four-tape default (criterion 15) is applied by the laser reader; checked in
    // curvyo-editor-wasm/tests/acceptance_0102_tester.rs.
    let again = round_trip(&d);
    assert_eq!(state(&again), state(&d));
}

#[test]
fn ac34_older_goldens_still_open() {
    for name in [
        "laser_workbench_v35.curvyo",
        "laser_workbench_v36.curvyo",
        "machine_profile_v36.curvyo",
        "manufacturing_unknown_process_v35.curvyo",
        "operations_v39.curvyo",
        "operations_clone_member_v39.curvyo",
        "operations_dangling_v39.curvyo",
        "operations_unknown_kind_v39.curvyo",
        "operations_unreadable_v39.curvyo",
    ] {
        let bytes = golden(name);
        let d = unpack(6, &bytes);
        assert!(d.is_ok(), "{name} must open unchanged: {:?}", d.err());
    }
}

#[test]
fn ac31_a_format_41_file_is_refused() {
    let d = Document::new(8);
    let bytes = pack(&d, "0.1.0").unwrap();
    // Superseded by `0103` (format 41), `0110` (format 42) and `0028` (format 43): the next number is
    // refused.
    let too_new = with_format(&bytes, 44);
    assert!(
        matches!(unpack(9, &too_new), Err(OpenError::FormatTooNew { .. })),
        "a format 44 file is refused by a format 43 build"
    );
    // The rewrite itself is sound: the same bytes at 43 open.
    assert!(unpack(9, &with_format(&bytes, 43)).is_ok());
}

// ---- criterion 2, 4, 19: document commands ------------------------------------------------------

#[test]
fn ac2_19_4_a_new_fixture_is_active_and_visible_and_its_slots_go_with_it() {
    let d = Document::new(9);
    let mfg = laser_page(&d);
    let holder = d
        .add_fixture(
            mfg,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: "curvyo.laser/holder",
                label: "Holder 1",
                params: &params(serde_json::json!({ "width_mm": 100.0, "height_mm": 60.0 })),
                matrix: Some(Affine::translation(Vec2::new(0.0, 0.0))),
            },
        )
        .unwrap();
    let f = &d.manufacturings()[0].fixtures[0];
    assert!(f.active, "a new fixture starts Active");
    assert!(f.visible);
    let slot = d.add_slot(holder, "Slot 1", &params(serde_json::json!({
        "shape": "rect", "cx_mm": 10.0, "cy_mm": 10.0, "w_mm": 20.0, "h_mm": 20.0, "d_mm": 0.0
    }))).unwrap();
    assert_eq!(d.manufacturings()[0].fixtures[0].slots.len(), 1);
    d.remove_slot(slot).unwrap();
    assert!(d.manufacturings()[0].fixtures[0].slots.is_empty());
    d.add_slot(
        holder,
        "Slot 2",
        &params(serde_json::json!({
            "shape": "circle", "cx_mm": 10.0, "cy_mm": 10.0, "w_mm": 0.0, "h_mm": 0.0, "d_mm": 12.0
        })),
    )
    .unwrap();
    d.delete_fixture(holder).unwrap();
    assert!(
        d.manufacturings()[0].fixtures.is_empty(),
        "the fixture and its slots are gone"
    );
}

#[test]
fn ac19_a_holder_refuses_a_51st_slot_at_the_document_level() {
    let d = Document::new(10);
    let mfg = laser_page(&d);
    let holder = d
        .add_fixture(
            mfg,
            &NewFixture {
                add: FixtureAdd::Holder,
                kind: "curvyo.laser/holder",
                label: "Holder 1",
                params: &params(serde_json::json!({ "width_mm": 100.0, "height_mm": 60.0 })),
                matrix: None,
            },
        )
        .unwrap();
    let values = params(serde_json::json!({
        "shape": "circle", "cx_mm": 10.0, "cy_mm": 10.0, "w_mm": 0.0, "h_mm": 0.0, "d_mm": 5.0
    }));
    for i in 0..50 {
        d.add_slot(holder, &format!("S{i}"), &values).unwrap();
    }
    assert!(
        d.add_slot(holder, "S50", &values).is_err(),
        "the 51st is refused"
    );
    assert_eq!(d.manufacturings()[0].fixtures[0].slots.len(), 50);
}

#[test]
fn ac31_the_format_40_file_holds_fixture_values_after_a_second_open_and_save() {
    let d = unpack(11, &golden("fixtures_v40.curvyo")).unwrap();
    let once = pack(&d, "0.1.0").unwrap();
    let twice = pack(&unpack(12, &once).unwrap(), "0.1.0").unwrap();
    assert_eq!(
        state(&unpack(13, &once).unwrap()),
        state(&unpack(14, &twice).unwrap())
    );
}
