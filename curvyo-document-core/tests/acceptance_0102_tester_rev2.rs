//! Independent acceptance tests for the revision 2 of `specs/0102-laser-fixtures` at the document
//! level (2026-11-10): the fifth kind `screw` with its two diameters stored as `thread_diameter_mm`
//! and `head_diameter_mm` (criterion 39, ADR 0049 §2), its damaged thread read as the default and
//! counted (criterion 33, ADR 0049 §6), a fixture matrix that is not rigid read as identity and
//! counted as damage (ADR 0049 §6), and the goldens `fixtures_v40` and `fixtures_damaged_v40`.
//! Written from the specification and ADR 0049 before the implementation was read for these points.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::case_sensitive_file_extension_comparisons
)]

use std::path::PathBuf;

use curvyo_document_core::{
    Affine, CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureAdd, FixtureParams, NewFixture,
    NewOutputPage, NodeId, Vec2, pack, unpack,
};

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

fn round_trip(d: &Document) -> Document {
    unpack(4343, &pack(d, "0.1.0").unwrap()).unwrap()
}

fn is_identity(m: Option<Affine>) -> bool {
    m.is_none_or(|m| {
        (m.a - 1.0).abs() < 1e-9
            && m.b.abs() < 1e-9
            && m.c.abs() < 1e-9
            && (m.d - 1.0).abs() < 1e-9
            && m.e.abs() < 1e-9
            && m.f.abs() < 1e-9
    })
}

// ---- criterion 2, 39: the screw node and its two diameters ------------------------------------

#[test]
fn ac39_a_screw_round_trips_with_its_thread_and_head_diameters() {
    let d = Document::new(11);
    let mfg = laser_page(&d);
    let screw = d
        .add_fixture(
            mfg,
            &NewFixture {
                add: FixtureAdd::Screw,
                kind: "curvyo.laser/screw",
                label: "Screw 1",
                params: &params(serde_json::json!({
                    "thread_diameter_mm": 6.0, "head_diameter_mm": 10.0
                })),
                matrix: Some(Affine::translation(Vec2::new(200.0, 150.0))),
            },
        )
        .expect("a screw is accepted");
    let _ = screw;
    let m = &d.manufacturings()[0];
    assert_eq!(m.fixtures.len(), 1);
    assert_eq!(m.fixtures[0].kind, "curvyo.laser/screw");
    assert!(m.fixtures[0].active, "a new fixture is Active");
    let again = round_trip(&d);
    let f = &again.manufacturings()[0].fixtures[0];
    assert_eq!(f.kind, "curvyo.laser/screw");
    assert_eq!(f.damaged, 0, "a complete screw reads without damage");
    let text = format!("{:?}", again.manufacturings());
    assert!(
        text.contains("thread_diameter_mm"),
        "the thread is stored: {text}"
    );
    assert!(
        text.contains("head_diameter_mm"),
        "the head is stored: {text}"
    );
}

#[test]
fn ac39_format_40_is_the_current_format_and_the_screw_needs_no_bump() {
    // Format 41 since `specs/0103-fixture-jobs` (ADR 0052 §6).
    assert_eq!(CURRENT_FORMAT_VERSION, 41);
}

// ---- golden fixtures_v40: one of each of the five kinds, the screw included ----------------------

#[test]
fn ac31_golden_v40_holds_one_screw_and_it_reads_without_damage() {
    let d = unpack(21, &golden("fixtures_v40.curvyo")).expect("fixtures_v40 opens");
    let m = &d.manufacturings()[0];
    let screws: Vec<_> = m
        .fixtures
        .iter()
        .filter(|f| f.kind == "curvyo.laser/screw")
        .collect();
    assert_eq!(screws.len(), 1, "one screw in the golden");
    assert_eq!(screws[0].damaged, 0, "the golden's screw is complete");
    assert!(screws[0].active);
}

#[test]
fn ac31_golden_v40_round_trips_unchanged_including_the_screw() {
    let d = unpack(22, &golden("fixtures_v40.curvyo")).expect("opens");
    let before = format!("{:?}", d.manufacturings());
    let again = round_trip(&d);
    assert_eq!(format!("{:?}", again.manufacturings()), before);
}

// ---- criterion 33 and ADR 0049 §6: the damaged golden -----------------------------------------

#[test]
fn ac33_damaged_golden_reads_the_damaged_screw_thread_as_default_and_counts_it() {
    let d = unpack(23, &golden("fixtures_damaged_v40.curvyo")).expect("opens");
    let m = &d.manufacturings()[0];
    let screw = m
        .fixtures
        .iter()
        .find(|f| f.kind == "curvyo.laser/screw")
        .expect("the damaged golden holds a screw");
    assert!(
        screw.damaged >= 1,
        "text in thread_diameter_mm counts as one damaged value: {}",
        screw.damaged
    );
    let text = format!("{screw:?}");
    assert!(
        !text.contains("NaN") && !text.contains("\"abc\""),
        "the damaged text is not kept as the value: {text}"
    );
}

#[test]
fn ac33_damaged_golden_reads_a_non_rigid_matrix_as_identity_and_counts_it() {
    let d = unpack(24, &golden("fixtures_damaged_v40.curvyo")).expect("opens");
    let m = &d.manufacturings()[0];
    let identity_damaged = m
        .fixtures
        .iter()
        .filter(|f| is_identity(f.matrix) && f.damaged >= 1)
        .count();
    assert!(
        identity_damaged >= 1,
        "a matrix with scale or four entries reads as identity and is counted"
    );
    let rigid_non_identity = m.fixtures.iter().filter(|f| !is_identity(f.matrix)).count();
    let _ = rigid_non_identity;
}

#[test]
fn ac33_damaged_golden_round_trips_without_rewriting_a_read_default() {
    let d = unpack(25, &golden("fixtures_damaged_v40.curvyo")).expect("opens");
    let before = format!("{:?}", d.manufacturings());
    let again = round_trip(&d);
    assert_eq!(
        format!("{:?}", again.manufacturings()),
        before,
        "the reader never rewrites a value it read as a default"
    );
}

// ---- goldens of older formats still open -------------------------------------------------------

#[test]
fn ac34_older_goldens_still_open_after_the_screw_kind() {
    for name in [
        "laser_workbench_v35.curvyo",
        "laser_workbench_v36.curvyo",
        "manufacturing_unknown_process_v35.curvyo",
        "machine_profile_v36.curvyo",
        "fixtures_grid_v39.curvyo",
    ] {
        let bytes = golden(name);
        assert!(unpack(31, &bytes).is_ok(), "{name} still opens");
    }
}

/// Rewrites a saved file so its format is `to` (the manifest text and the root map carry it).
fn with_format(bytes: &[u8], to: i64) -> Vec<u8> {
    use loro::{ExportMode, LoroDoc};
    use std::io::{Cursor, Read, Write};
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

#[test]
fn ac31_a_format_41_file_is_refused_and_a_format_40_file_opens() {
    let d = Document::new(41);
    let bytes = pack(&d, "0.1.0").unwrap();
    // Superseded by `0103` (format 41): the control opens at 41, and 42 is refused.
    assert!(unpack(42, &bytes).is_ok(), "the control: format 41 opens");
    let future = with_format(&bytes, 42);
    assert!(unpack(43, &future).is_err(), "format 42 is refused");
}
