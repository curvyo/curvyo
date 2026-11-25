//! `specs/0118` criterion 35 a (adrs P7, P8): the laser extension reads the focus pulse of the
//! format 63 goldens of `curvyo-document-core`: the stored values, the defaults of a copy
//! without the keys, and each damaged value at its default, counted once.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{ManufacturingSnapshot, unpack};
use curvyo_laser_core::{CopyRead, FocusPulse, PulseMs, PulsePermille, read_copy};

fn pages(name: &str) -> Vec<ManufacturingSnapshot> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(2, &std::fs::read(path).unwrap())
        .unwrap()
        .manufacturings()
}

fn read(page: &ManufacturingSnapshot) -> CopyRead {
    read_copy(page.profile.as_ref(), None)
}

#[test]
fn c35a_the_golden_reads_the_pulse_its_defaults_and_a_copy_without_the_keys() {
    let pages = pages("focus_pulse_v63.curvyo");
    let on = read(&pages[0]);
    assert_eq!(
        on.copy.values.focus_pulse,
        FocusPulse {
            on: true,
            power: PulsePermille::new(25).unwrap(),
            length: PulseMs::new(500).unwrap(),
        }
    );
    assert_eq!(on.damaged_values(), 0);
    for page in &pages[1..] {
        let read = read(page);
        assert_eq!(read.copy.values.focus_pulse, FocusPulse::default());
        assert_eq!(read.damaged_values(), 0);
    }
    assert_eq!(FocusPulse::default().power.get(), 10, "1 % (Frage 370 A)");
    assert_eq!(FocusPulse::default().length.get(), 300, "Frage 371 A");
    assert!(
        !FocusPulse::default().on,
        "off in a new profile (Frage 369 B)"
    );
}

#[test]
fn c35a_each_damaged_value_reads_its_default_and_counts_one() {
    let pages = pages("focus_pulse_damaged_v63.curvyo");
    assert_eq!(pages.len(), 4);
    for page in &pages {
        let read = read(page);
        assert_eq!(read.copy.values.focus_pulse, FocusPulse::default());
        assert_eq!(read.focus_pulse_damaged, 1);
        assert_eq!(read.damaged_values(), 1);
    }
}
