//! Independent acceptance tests for `specs/0103-fixture-jobs` at the file level: format 41 (ADR 0052
//! §6, criteria 20 to 23), the three new goldens, the older goldens that must open unchanged, a
//! format 42 file refused, damage counts of the fixture jobs (criterion 22), and no damage for a job
//! child (ADR 0052 §5, criterion 12). Written from the specification and ADR 0052 before the
//! implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::items_after_statements,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::too_many_lines
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{CURRENT_FORMAT_VERSION, Document, pack, unpack};
use loro::{ExportMode, LoroDoc};

fn golden(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("golden {name}: {e}"))
}

fn state(d: &Document) -> String {
    format!("{:?}", d.manufacturings())
}

fn round_trip(d: &Document) -> Document {
    unpack(4343, &pack(d, "0.1.0").unwrap()).unwrap()
}

/// Rewrites the format of a saved file to `to`, in the manifest text and in the root map of the
/// snapshot, as the reader checks both (the helper of `acceptance_0102_tester`).
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

fn damaged_total(d: &Document) -> usize {
    d.manufacturings()
        .iter()
        .flat_map(|m| m.fixtures.iter())
        .map(|f| f.damaged)
        .sum()
}

// ---- criterion 20: format 41 and the job golden --------------------------------------------------

#[test]
fn ac20_the_current_format_is_41() {
    // Format 42 since `specs/0110-pen-modes-variable-stroke` (ADR 0053).
    // Format 44 since `specs/0111-laser-operation-tools` (ADR 0056 §8).
    assert_eq!(CURRENT_FORMAT_VERSION, 45);
}

#[test]
fn ac20_golden_v41_opens_with_its_fixtures_and_no_damage() {
    let d = unpack(1, &golden("fixture_jobs_v41.curvyo")).expect("fixture_jobs_v41 opens");
    let m = &d.manufacturings()[0];
    // A holder with its job, a grid with a Cut job, a grid with no job, an inactive holder with one.
    assert!(
        m.fixtures.len() >= 3,
        "the fixtures of the golden are read: {}",
        m.fixtures.len()
    );
    assert!(
        m.fixtures.iter().any(|f| f.kind.ends_with("holder")),
        "a holder is read"
    );
    assert!(
        m.fixtures.iter().any(|f| !f.active),
        "the inactive holder reads inactive"
    );
    // The dead target key is kept and is not damage (criterion 12); a job child is not a slot
    // that failed to read (ADR 0052 §5), so no fixture reports damage.
    assert_eq!(
        damaged_total(&d),
        0,
        "a golden of valid jobs, a dead key and no damage reads with no count"
    );
}

#[test]
fn ac20_golden_v41_round_trips_every_value() {
    let d = unpack(2, &golden("fixture_jobs_v41.curvyo")).unwrap();
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(
        state(&again),
        before,
        "the job order, names, Show, Output and ticks survive save and reopen"
    );
    assert_eq!(damaged_total(&again), 0);
}

#[test]
fn ac20_golden_v41_saves_as_format_41() {
    let d = unpack(3, &golden("fixture_jobs_v41.curvyo")).unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();
    let mut zin = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut seen = false;
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).unwrap();
        if f.name().unwrap().ends_with(".json") {
            let mut t = String::new();
            f.read_to_string(&mut t).unwrap();
            if t.contains("\"format_version\"") {
                // Saved at the current format, 42 since `specs/0110-pen-modes-variable-stroke`, 43
                // since `specs/0028-material-test-library`, 44 since `specs/0111`.
                assert!(
                    t.contains("\"format_version\": 45") || t.contains("\"format_version\":45"),
                    "the manifest says 45"
                );
                seen = true;
            }
        }
    }
    assert!(seen);
}

// ---- criterion 21: unknown kind kept and saved back unchanged ----------------------------------

#[test]
fn ac21_unknown_kind_golden_opens_and_is_saved_back_unchanged() {
    let d = unpack(4, &golden("fixture_jobs_unknown_kind_v41.curvyo")).expect("opens");
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(
        state(&again),
        before,
        "the unknown job keeps its values and is written back as read"
    );
    let again2 = round_trip(&again);
    assert_eq!(state(&again2), before, "and it stays after a second save");
}

// ---- criterion 22: damaged values are counted, never rewritten ---------------------------------

#[test]
fn ac22_damaged_golden_counts_its_four_damaged_values() {
    // A text `show`, a text `output`, a target with value false, a `.tapes` suffix on a holder.
    // ADR 0052 section 3: the document counts the values it sees at read time (the show, the
    // output and the false target). The `.tapes` key on a holder is judged by laser-core's
    // `job_parts()`, so the session total is 4 (`curvyo-editor-wasm/tests/fixture_jobs_damage.rs`).
    let d = unpack(5, &golden("fixture_jobs_damaged_v41.curvyo")).expect("opens");
    assert_eq!(
        damaged_total(&d),
        3,
        "the document counts the three values it reads; the tapes key is judged by laser-core"
    );
}

#[test]
fn ac22_damaged_golden_is_not_rewritten_on_read() {
    let d = unpack(6, &golden("fixture_jobs_damaged_v41.curvyo")).unwrap();
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(
        state(&again),
        before,
        "the reader never writes a default over a damaged value"
    );
    assert_eq!(
        damaged_total(&again),
        3,
        "the count survives save and reopen"
    );
}

// ---- criteria 20 to 22: format 42 refused, older goldens unchanged ------------------------------

#[test]
fn ac20_a_format_42_file_is_refused() {
    let bytes = pack(
        &unpack(7, &golden("fixture_jobs_v41.curvyo")).unwrap(),
        "0.1.0",
    )
    .unwrap();
    // Superseded by `0110` (format 42), `0028` (format 43) and `0111` (format 44): the next number
    // is refused.
    let future = with_format(&bytes, 46);
    assert!(
        unpack(8, &future).is_err(),
        "a file of a later format is refused, not read"
    );
}

#[test]
fn ac20_the_older_goldens_open_unchanged() {
    for name in [
        "fixtures_v40.curvyo",
        "fixtures_damaged_v40.curvyo",
        "fixtures_unknown_kind_v40.curvyo",
        "fixtures_grid_v39.curvyo",
        "operations_v39.curvyo",
        "laser_workbench_v36.curvyo",
        "laser_workbench_v35.curvyo",
        "machine_profile_v36.curvyo",
    ] {
        let d = unpack(9, &golden(name)).unwrap_or_else(|e| panic!("{name} opens: {e:?}"));
        let before = state(&d);
        let again = round_trip(&d);
        assert_eq!(state(&again), before, "{name} keeps every value");
    }
}

#[test]
fn ac20_a_format_40_fixture_file_reads_with_no_job_and_no_count() {
    let d = unpack(10, &golden("fixtures_v40.curvyo")).unwrap();
    assert_eq!(
        damaged_total(&d),
        0,
        "a format 40 file has no job node; nothing is damaged"
    );
}
