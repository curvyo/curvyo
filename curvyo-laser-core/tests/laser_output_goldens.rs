//! The format 48 goldens of `curvyo-document-core/tests/fixtures/` as the laser extension reads
//! them (`specs/0029-laser-job-preview-and-output` criteria 2, 25, 32, 38, 39; ADR 0065 §1 to
//! §3, §6): the page registers and the copy's Air assist and output format, each damaged value
//! read as its default or damaged state and counted once.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_document_core::{ManufacturingSnapshot, unpack};
use curvyo_laser_core::OutputFormat;
use curvyo_laser_core::job::{Start, read_output_settings};
use curvyo_laser_core::read_copy;

fn pages(name: &str) -> Vec<ManufacturingSnapshot> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(2, &std::fs::read(path).unwrap())
        .unwrap()
        .manufacturings()
}

#[test]
fn c32_the_golden_reads_the_pick_the_start_air_assist_and_grbl() {
    let pages = pages("laser_output_v48.curvyo");
    let settings = read_output_settings(&pages[0]);
    assert_eq!(
        settings.fixture_material.as_deref(),
        Some("0b3c7f1e-8f0e-4c4a-9a51-2f6c1c1e9d10")
    );
    // Superseded by 0029 rework (2026-11-17): one register `start` (ADR 0065 §6).
    assert_eq!(settings.start, Start::HomeThenOrigin);
    assert!(!settings.start_damaged);
    assert_eq!(settings.damaged, 0);
    let copy = read_copy(pages[0].profile.as_ref(), None);
    assert!(copy.copy.values.air_assist);
    assert_eq!(copy.copy.values.output_format, OutputFormat::Grbl);
    assert_eq!(copy.damaged_values(), 0);
    assert_eq!(read_output_settings(&pages[1]).start, Start::HeadPosition);
    let plain = read_output_settings(&pages[2]);
    assert_eq!(plain.fixture_material, None);
    assert_eq!(plain.start, Start::default());
    let copy = read_copy(pages[2].profile.as_ref(), None);
    assert!(!copy.copy.values.air_assist, "absent reads off");
    assert_eq!(
        copy.copy.values.output_format,
        OutputFormat::Grbl,
        "absent reads GRBL"
    );
    assert_eq!(copy.damaged_values(), 0);
}

#[test]
fn c32_each_damaged_value_reads_its_default_or_damaged_state_and_counts_once() {
    let pages = pages("laser_output_damaged_v48.curvyo");
    let settings = read_output_settings(&pages[0]);
    assert_eq!(settings.fixture_material, None, "a number is no pick");
    // Superseded by 0029 rework (2026-11-17): `start` "middle" reads Machine origin, damaged.
    assert_eq!(settings.start, Start::MachineOrigin);
    assert!(settings.start_damaged);
    assert_eq!(settings.damaged, 2);
    let second = read_output_settings(&pages[1]);
    assert!(second.start_damaged, "a boolean is no Start");
    assert_eq!(second.start, Start::MachineOrigin);
    let copy = read_copy(pages[0].profile.as_ref(), None);
    assert!(!copy.copy.values.air_assist);
    assert!(copy.air_assist_damaged);
    assert!(copy.output_format_damaged, "an unknown format blocks (4 c)");
    assert_eq!(copy.damaged_values(), 2);
}
