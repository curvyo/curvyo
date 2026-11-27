//! Independent acceptance tests for `specs/0113-fixture-sets-and-pins` at the file level: format
//! 47 (ADR 0059), the goldens `fixture_sets_v47`, `fixture_sets_damaged_v47` and
//! `fixtures_pre_sets_v46` (criteria 38 to 40), a format 48 file refused, nothing written on read,
//! and the older fixture goldens still opening unchanged. Written from the specification before the
//! implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::case_sensitive_file_extension_comparisons,
    missing_docs
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{CURRENT_FORMAT_VERSION, Document, pack, unpack};
use loro::{ExportMode, LoroDoc};

fn golden(name: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .unwrap_or_else(|e| panic!("golden {name}: {e}"))
}

fn state(d: &Document) -> String {
    format!("{:?}", d.manufacturings())
}

fn round_trip(d: &Document) -> Document {
    unpack(4747, &pack(d, "0.1.0").unwrap()).unwrap()
}

fn damaged(d: &Document) -> usize {
    d.manufacturings()
        .iter()
        .map(|m| {
            m.sets.iter().map(|s| s.damaged).sum::<usize>()
                + m.fixtures
                    .iter()
                    .map(|f| f.damaged + f.jobs.iter().map(|j| j.damaged).sum::<usize>())
                    .sum::<usize>()
        })
        .sum()
}

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
            let from = CURRENT_FORMAT_VERSION;
            data = String::from_utf8(data)
                .unwrap()
                .replace(
                    &format!("\"format_version\": {from}"),
                    &format!("\"format_version\": {to}"),
                )
                .replace(
                    &format!("\"format_version\":{from}"),
                    &format!("\"format_version\":{to}"),
                )
                .into_bytes();
        }
        out.start_file(name, options).unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

#[test]
fn format_is_47() {
    // Format 49 since `specs/0124-gradient-fill` (ADR 0066 §8).
    // Format 50 since `specs/0123-raster-engraving` (ADR 0069 §10).
    // Format 51 since `specs/0135-image-optimisation` slice 1 (ADR 0072 §8); Superseded by 0135 slice 1 (2026-11-19).
    assert_eq!(CURRENT_FORMAT_VERSION, 71);
}

#[test]
fn a_format_48_file_is_refused() {
    let bytes = pack(&unpack(1, &golden("fixture_sets_v47.curvyo")).unwrap(), "t").unwrap();
    assert!(unpack(2, &with_format(&bytes, 72)).is_err());
}

#[test]
fn ac38_golden_v47_holds_two_named_sets_one_off_and_two_job_fixtures() {
    let d = unpack(3, &golden("fixture_sets_v47.curvyo")).unwrap();
    let m = d
        .manufacturings()
        .into_iter()
        .find(|m| !m.sets.is_empty())
        .expect("a page with sets");
    assert_eq!(m.sets.len(), 2);
    assert_eq!(
        m.sets.iter().filter(|s| s.on_page).count(),
        1,
        "one set off"
    );
    let jobs: Vec<_> = m.fixtures.iter().filter(|f| f.set.is_none()).collect();
    assert_eq!(jobs.len(), 2, "two job fixtures");
    assert!(jobs.iter().any(|f| f.locked), "a locked pin grid");
    // Superseded by `0113` rework (criteria 32, 38 amended 2026-11-16): the golden holds a locked
    // set and a locked set fixture.
    assert!(
        m.fixtures.iter().any(|f| f.set.is_some() && f.locked),
        "a locked set fixture"
    );
    assert!(m.sets.iter().any(|s| s.locked), "a locked set");
    assert!(
        m.fixtures.iter().any(|f| f.set.is_some() && !f.active),
        "own check off"
    );
    assert!(
        m.fixtures
            .iter()
            .any(|f| f.kind.ends_with("/screw") && f.params.get("mark_mm") == Some(&3.0.into())),
        "the screw's 3 mm Size"
    );
    assert!(
        m.fixtures
            .iter()
            .flat_map(|f| f.jobs.iter())
            .any(|j| j.role.is_some() && j.output),
        "a role job with Output on"
    );
    assert_eq!(damaged(&d), 0);
}

#[test]
fn ac38_golden_v47_round_trips_every_value_and_saves_as_47() {
    let d = unpack(4, &golden("fixture_sets_v47.curvyo")).unwrap();
    let before = state(&d);
    let again = round_trip(&d);
    assert_eq!(state(&again), before);
    assert_eq!(
        state(&round_trip(&again)),
        before,
        "stable on a second save"
    );
    let bytes = pack(&d, "t").unwrap();
    let mut zin = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut seen = false;
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).unwrap();
        if f.name().unwrap().ends_with(".json") {
            let mut t = String::new();
            f.read_to_string(&mut t).unwrap();
            if t.contains("\"format_version\"") {
                assert!(
                    t.contains("\"format_version\": 71") || t.contains("\"format_version\":71")
                );
                seen = true;
            }
        }
    }
    assert!(seen);
}

#[test]
fn ac39_a_format_46_file_reads_its_fixtures_as_unlocked_job_fixtures_without_a_set() {
    let d = unpack(5, &golden("fixtures_pre_sets_v46.curvyo")).unwrap();
    let version = d.version();
    let ms = d.manufacturings();
    let m = ms
        .iter()
        .find(|m| !m.fixtures.is_empty())
        .expect("fixtures");
    assert!(m.sets.is_empty(), "no set is made");
    assert!(m.fixtures.iter().all(|f| f.set.is_none() && !f.locked));
    let screw = m
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("/screw"))
        .expect("a screw");
    assert!(screw.jobs.iter().all(|j| j.role.is_none()), "no role jobs");
    assert_eq!(d.version(), version, "nothing is written on read");
    let before = state(&d);
    assert_eq!(state(&round_trip(&d)), before, "and it keeps every value");
}

#[test]
fn ac40_the_damaged_golden_counts_damage_and_is_not_rewritten() {
    let d = unpack(6, &golden("fixture_sets_damaged_v47.curvyo")).unwrap();
    let version = d.version();
    let n = damaged(&d);
    assert!(n > 0, "damage is counted");
    assert_eq!(d.version(), version, "nothing is written on read");
    let again = round_trip(&d);
    assert_eq!(
        state(&again),
        state(&d),
        "the reader writes no default over damage"
    );
    assert_eq!(damaged(&again), n);
}

#[test]
fn the_older_fixture_goldens_still_open_unchanged() {
    for name in [
        "fixtures_v40.curvyo",
        "fixtures_damaged_v40.curvyo",
        "fixtures_unknown_kind_v40.curvyo",
        "fixtures_grid_v39.curvyo",
        "fixture_jobs_v41.curvyo",
        "fixture_jobs_damaged_v41.curvyo",
        "fixture_jobs_unknown_kind_v41.curvyo",
    ] {
        let d = unpack(7, &golden(name)).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        let before = state(&d);
        assert_eq!(state(&round_trip(&d)), before, "{name}");
        assert!(
            d.manufacturings().iter().all(|m| m.sets.is_empty()),
            "{name}: no set is made"
        );
    }
}
