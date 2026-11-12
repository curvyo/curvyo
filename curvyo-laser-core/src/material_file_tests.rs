use super::*;
use crate::record_envelope::record_file_name;

const ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

/// The golden files, compiled in (this crate reads no file, ADR 0034 §1).
fn fixture(name: &str) -> String {
    macro_rules! files {
        ($($file:literal),*) => {
            match name {
                $($file => include_str!(concat!("../tests/fixtures/materials/", $file)),)*
                _ => panic!("no fixture {name}"),
            }
        };
    }
    files!(
        "full.json",
        "generic_laser.json",
        "no_sets.json",
        "history_max.json",
        "unknown_key.json",
        "power_0_5.json",
        "power_100_5.json",
        "speed_0.json",
        "passes_21.json",
        "thickness_0.json",
        "thickness_100_1.json",
        "name_61.json",
        "source_no_entry.json",
        "history_501.json",
        "id_mismatch.json",
        "newer_format.json"
    )
    .to_owned()
}

fn read(name: &str) -> Result<Material, MaterialFileError> {
    read_material(&record_file_name(ID), &fixture(name))
}

#[test]
fn the_full_record_reads_and_writes_back_byte_for_byte() {
    let text = fixture("full.json");
    let material = read("full.json").unwrap();
    assert_eq!(material.name, "Birch plywood");
    assert_eq!(material.thickness.mm(), 3.0);
    assert_eq!(material.machine.id, MACHINE);
    assert_eq!(material.modified, 1_762_948_800_000);
    let cut = material.set(SetKind::Cut).unwrap();
    assert_eq!(
        (cut.power.tenths(), cut.speed.get(), cut.passes.get()),
        (600, 300, 2)
    );
    assert_eq!(material.set(SetKind::Score).unwrap().power.tenths(), 125);
    assert_eq!(material.sets[&SetKind::Score].source, SetSource::Edited);
    assert_eq!(
        material.sets[&SetKind::Engrave].source,
        SetSource::Test("4d5e6f7a-8b9c-4dae-bf01-3b4c5d6e7f80".to_owned())
    );
    let events: Vec<&str> = material
        .history
        .iter()
        .map(|entry| match entry.event {
            HistoryEvent::Tested { .. } => "tested",
            HistoryEvent::Edited { .. } => "edited",
            HistoryEvent::Removed => "removed",
        })
        .collect();
    assert_eq!(events, ["tested", "edited", "removed", "edited", "tested"]);
    let HistoryEvent::Tested { grid, cell, .. } = &material.history[0].event else {
        panic!("a test first");
    };
    assert_eq!((grid.columns, grid.rows, grid.cell.as_mm()), (5, 5, 10.0));
    assert_eq!(*cell, CellIndex { column: 2, row: 1 });
    assert_eq!(write_material(&material), text);
}

#[test]
fn the_other_good_goldens_read_write_read_as_the_identity() {
    for case in ["generic_laser.json", "no_sets.json", "history_max.json"] {
        let text = fixture(case);
        let material = read(case).unwrap();
        assert_eq!(write_material(&material), text, "{case}");
        assert_eq!(
            read_material(&record_file_name(ID), &text).unwrap(),
            material
        );
    }
    assert_eq!(
        read("generic_laser.json").unwrap().machine.id,
        "generic-laser"
    );
    assert!(read("no_sets.json").unwrap().sets.is_empty());
    let max = read("history_max.json").unwrap();
    assert_eq!(max.history.len(), MAX_HISTORY_ENTRIES);
    assert!(fixture("history_max.json").len() <= MAX_MATERIAL_BYTES);
}

#[test]
fn every_damage_has_its_reason() {
    for (case, reason) in [
        ("unknown_key.json", "Unknown key colour."),
        ("power_0_5.json", "Power 0.5 is outside 1 to 100."),
        ("power_100_5.json", "Power 100.5 is outside 1 to 100."),
        ("speed_0.json", "Speed 0 is outside 1 to 100,000 mm/min."),
        ("passes_21.json", "Passes 21 are outside 1 to 20."),
        ("thickness_0.json", "Thickness 0 is outside 0.1 to 100 mm."),
        (
            "thickness_100_1.json",
            "Thickness 100.1 is outside 0.1 to 100 mm.",
        ),
        ("name_61.json", "The name has 61 characters, not 1 to 60."),
        (
            "source_no_entry.json",
            "The Engrave set names no test in the history.",
        ),
        (
            "history_501.json",
            "The history has 501 entries, more than 500.",
        ),
        ("id_mismatch.json", "The id differs from the file name."),
    ] {
        assert_eq!(
            read(case),
            Err(MaterialFileError::Damaged(reason.to_owned())),
            "{case}"
        );
        assert!(reason.chars().count() <= 70, "{reason}");
    }
    assert_eq!(read("newer_format.json"), Err(MaterialFileError::Newer));
}

#[test]
fn shape_damage_beyond_the_goldens() {
    let full = fixture("full.json");
    let name = record_file_name(ID);
    let big = format!("{full}{}", " ".repeat(MAX_MATERIAL_BYTES));
    assert_eq!(
        read_material(&name, &big),
        Err(MaterialFileError::Damaged(
            "The file is over 256 KiB.".to_owned()
        ))
    );
    for (from, to, why) in [
        (
            "\"kind\": \"laser\"",
            "\"kind\": \"plotter\"",
            "a plotter record",
        ),
        (
            "\"modified\": 1762948800000",
            "\"modified\": \"x\"",
            "a type error",
        ),
        ("\"format_version\": 1", "\"format_version\": 0", "format 0"),
        (
            "\"source\": \"edited\"",
            "\"source\": \"typed\"",
            "a bad source",
        ),
        (
            "\"test\": \"5d1e7a2b-0c3f-4e6d-8a9b-7c6d5e4f3a2b\"",
            "\"test\": \"2b3c4d5e-6f7a-4b8c-9d0e-1f2a3b4c5d6e\"",
            "a source naming a removed entry",
        ),
        (
            "\"event\": \"removed\"",
            "\"event\": \"lost\"",
            "an unknown event",
        ),
        ("\"column\": 2", "\"column\": 5", "a cell outside its grid"),
        ("\"columns\": 5", "\"columns\": 13", "a grid of 13 columns"),
        (
            "\"id\": \"6f1c2a4e",
            "\"id\": \"6F1C2A4E",
            "a bad machine id",
        ),
        (
            "\"name\": \"Birch plywood\"",
            "\"name\": \"  \"",
            "a blank name",
        ),
        (
            "\"power_pct\": 60.0",
            "\"power_pct\": 60.25",
            "two decimals",
        ),
    ] {
        let damaged = full.replacen(from, to, 1);
        assert_ne!(damaged, full, "{why}");
        assert!(
            matches!(
                read_material(&name, &damaged),
                Err(MaterialFileError::Damaged(_))
            ),
            "{why}"
        );
    }
    assert_eq!(
        read_material(&record_file_name(MACHINE), &full),
        Err(MaterialFileError::Damaged(
            "The id differs from the file name.".to_owned()
        ))
    );
}

#[test]
fn a_speed_above_any_machine_feed_is_not_damage_while_it_is_a_feed_rate() {
    let fast = fixture("full.json").replacen("\"speed_mm_min\": 300", "\"speed_mm_min\": 90000", 1);
    assert_eq!(
        read_material(&record_file_name(ID), &fast)
            .unwrap()
            .set(SetKind::Cut)
            .unwrap()
            .speed
            .get(),
        90_000
    );
}
