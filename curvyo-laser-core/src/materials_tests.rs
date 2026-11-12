use super::*;
use crate::machine_profile::FeedRate;
use crate::material::{
    HistoryEvent, MachineTie, Passes, PowerPercent, SetKind, SetSource, Settings, Thickness,
    next_modified,
};
use crate::material_edits::{NewMaterial, RevertRefused, Reverted, SaveTarget, TestResult};
use crate::material_file::write_material;
use crate::record_envelope::{new_record_id, record_file_name};

const MACHINE: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";
const FULL: &str = include_str!("../tests/fixtures/materials/full.json");
const FULL_ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";
const MAX: &str = include_str!("../tests/fixtures/materials/history_max.json");

fn id(seed: u8) -> String {
    new_record_id([seed; 16])
}

fn settings(power_tenths: u16, speed: u64) -> Settings {
    Settings {
        power: PowerPercent::from_tenths(power_tenths).unwrap(),
        speed: FeedRate::new(speed).unwrap(),
        passes: Passes::ONE,
    }
}

fn test_result(kind: SetKind) -> TestResult {
    let material = read_material(&record_file_name(FULL_ID), FULL).unwrap();
    let HistoryEvent::Tested { grid, cell, .. } = material.history[0].event else {
        panic!("a test first");
    };
    TestResult {
        kind,
        grid,
        cell,
        set: settings(600, 3000),
    }
}

/// A record named `name` for `machine` with only its Cut set, as the file text.
fn record(seed: u8, name: &str, machine: &str) -> (String, String) {
    let mut material = read_material(&record_file_name(FULL_ID), FULL).unwrap();
    material.id = id(seed);
    material.name = name.to_owned();
    material.machine.id = machine.to_owned();
    (record_file_name(&material.id), write_material(&material))
}

fn new_material(seed: u8, name: &str) -> SaveTarget {
    SaveTarget::New(NewMaterial {
        id: id(seed),
        name: name.to_owned(),
        thickness: Thickness::from_mm(3.0).unwrap(),
        machine: MachineTie {
            id: MACHINE.to_owned(),
            name: "K40".to_owned(),
        },
    })
}

fn written(write: &LibraryWrite) -> &str {
    match write {
        LibraryWrite::Write { text, .. } => text,
        LibraryWrite::Remove { .. } => panic!("a write"),
    }
}

#[test]
fn a_load_keeps_damaged_and_newer_records_with_one_notice_each() {
    let (newer_name, newer_text) = record(3, "Oak", MACHINE);
    let files = vec![
        record(2, "b Birch", MACHINE),
        record(1, "A Acrylic", MACHINE),
        (record_file_name(&id(4)), "{".to_owned()),
        (
            newer_name.clone(),
            newer_text.replace("\"format_version\": 1", "\"format_version\": 2"),
        ),
        ("notes.txt".to_owned(), "{".to_owned()),
        (
            format!("{}.sync-conflict", record_file_name(&id(5))),
            FULL.to_owned(),
        ),
    ];
    let library = Materials::load(&files, None);
    assert_eq!(library.reads().len(), 4);
    assert_eq!(
        library.load_notices(),
        [
            format!("{newer_name}: needs a newer Curvyo."),
            format!(
                "{}: The file is not a material record.",
                record_file_name(&id(4))
            ),
        ]
    );
    assert_eq!(
        library.name_of(&id(3)),
        Some("Oak"),
        "a newer record's name"
    );
    let names: Vec<Option<&str>> = library
        .listed(MACHINE)
        .iter()
        .map(|read| library.name_of(read.id()))
        .collect();
    assert_eq!(
        names,
        [None, Some("A Acrylic"), Some("b Birch"), Some("Oak")],
        "the unreadable record names no machine and sorts by an empty name"
    );
    assert!(
        library
            .listed("generic-laser")
            .iter()
            .all(|read| matches!(read, MaterialRead::Damaged { .. })),
        "only the record naming no machine shows on every page"
    );
}

#[test]
fn records_past_1000_are_never_read_or_written() {
    let (_, text) = record(0, "x", MACHINE);
    let files: Vec<(String, String)> = (0..=MAX_MATERIALS)
        .map(|n| {
            let mut bytes = [0u8; 16];
            bytes[..8].copy_from_slice(&(n as u64).to_be_bytes());
            let id = new_record_id(bytes);
            (record_file_name(&id), text.replace(&id_of(&text), &id))
        })
        .collect();
    let library = Materials::load(&files, None);
    assert_eq!(library.reads().len(), MAX_MATERIALS);
    let last = record_id(&files[MAX_MATERIALS].0).unwrap().to_owned();
    let mut library = library;
    let mut target = new_material(0, "New");
    if let SaveTarget::New(new) = &mut target {
        new.id = last.clone();
    }
    assert_eq!(
        library.save_test(target, test_result(SetKind::Cut), &id(9), 1),
        Err(LibraryRefusal::Taken)
    );
    assert_eq!(
        library.revert(&last, None, Some(&text), 1),
        Err(RevertRefused::Changed)
    );
}

fn id_of(text: &str) -> String {
    let start = text.find("\"id\": \"").unwrap() + 7;
    text[start..start + 36].to_owned()
}

#[test]
fn the_order_lists_known_ids_first_then_by_name_and_a_move_writes_it() {
    let files = vec![
        record(1, "Acrylic", MACHINE),
        record(2, "birch", MACHINE),
        record(3, "Cork", MACHINE),
        record(4, "Felt", "generic-laser"),
    ];
    let order = write_order(&[
        id(3),
        "11111111-2222-4333-8444-555555555555".to_owned(),
        id(4),
    ]);
    let mut library = Materials::load(&files, Some(&order));
    let ids = |library: &Materials, profile| -> Vec<String> {
        library
            .listed(profile)
            .iter()
            .map(|read| read.id().to_owned())
            .collect()
    };
    assert_eq!(ids(&library, MACHINE), [id(3), id(1), id(2)]);
    assert_eq!(ids(&library, "generic-laser"), [id(4)]);
    let text = library.move_row(MACHINE, &id(2), 0).unwrap();
    assert_eq!(
        read_order(&text),
        [id(2), id(3), id(1), id(4)],
        "unknown id dropped"
    );
    assert_eq!(ids(&library, MACHINE), [id(2), id(3), id(1)]);
    assert!(library.move_row("generic-laser", &id(2), 0).is_none());
    assert_eq!(
        library
            .move_row(MACHINE, &id(2), 99)
            .map(|t| read_order(&t)[2].clone()),
        Some(id(2))
    );
}

#[test]
fn save_as_material_makes_a_record_or_adds_and_replaces_a_set() {
    let mut library = Materials::default();
    let write = library
        .save_test(
            new_material(1, "Birch 3 mm"),
            test_result(SetKind::Cut),
            &id(10),
            5_000,
        )
        .unwrap();
    let material = library.find(&id(1)).unwrap().clone();
    assert_eq!(written(&write), library.text(&id(1)).unwrap());
    assert_eq!(
        read_material(&record_file_name(&id(1)), written(&write)).unwrap(),
        material
    );
    assert_eq!(material.modified, 5_000);
    assert_eq!(material.sets[&SetKind::Cut].source, SetSource::Test(id(10)));
    assert!(matches!(
        material.history[0].event,
        HistoryEvent::Tested { .. }
    ));
    let mut replaced = test_result(SetKind::Cut);
    replaced.set = settings(650, 2000);
    library
        .save_test(SaveTarget::Existing(id(1)), replaced, &id(11), 4_000)
        .unwrap();
    library
        .save_test(
            SaveTarget::Existing(id(1)),
            test_result(SetKind::Score),
            &id(12),
            4_000,
        )
        .unwrap();
    let material = library.find(&id(1)).unwrap();
    assert_eq!(
        material.modified, 5_002,
        "modified rises with a clock behind"
    );
    assert_eq!(material.set(SetKind::Cut).unwrap().power.tenths(), 650);
    assert_eq!(material.sets[&SetKind::Cut].source, SetSource::Test(id(11)));
    assert!(material.set(SetKind::Score).is_some());
    assert_eq!(material.history.len(), 3, "append-only");
    assert_eq!(
        library.save_test(
            new_material(1, "Again"),
            test_result(SetKind::Cut),
            &id(13),
            1
        ),
        Err(LibraryRefusal::Taken)
    );
    assert_eq!(
        library.save_test(new_material(2, "  "), test_result(SetKind::Cut), &id(13), 1),
        Err(LibraryRefusal::InvalidName)
    );
    assert_eq!(
        library.save_test(
            SaveTarget::Existing(id(7)),
            test_result(SetKind::Cut),
            &id(13),
            1
        ),
        Err(LibraryRefusal::Unknown)
    );
}

#[test]
fn set_edits_removals_renames_and_thickness_write_with_history() {
    let mut library = Materials::load(&[(record_file_name(FULL_ID), FULL.to_owned())], None);
    let before = library.find(FULL_ID).unwrap().clone();
    library
        .edit_set(FULL_ID, SetKind::Cut, settings(650, 300), id(20), 1)
        .unwrap();
    let material = library.find(FULL_ID).unwrap();
    assert_eq!(material.sets[&SetKind::Cut].source, SetSource::Edited);
    assert!(matches!(
        material.history.last().unwrap().event,
        HistoryEvent::Edited { .. }
    ));
    assert_eq!(material.modified, before.modified + 1);
    assert_eq!(
        library.edit_set(FULL_ID, SetKind::Cut, settings(650, 300), id(21), 1),
        Err(LibraryRefusal::Unchanged)
    );
    library
        .remove_set(FULL_ID, SetKind::Score, id(22), 2)
        .unwrap();
    assert_eq!(
        library.remove_set(FULL_ID, SetKind::Score, id(23), 2),
        Err(LibraryRefusal::NoSet {
            name: "Birch plywood".to_owned(),
            kind: "Score"
        })
    );
    assert_eq!(
        LibraryRefusal::NoSet {
            name: "Birch plywood".to_owned(),
            kind: "Score"
        }
        .to_string(),
        "Birch plywood has no Score set."
    );
    let material = library.find(FULL_ID).unwrap();
    assert!(material.set(SetKind::Score).is_none());
    assert!(matches!(
        material.history.last().unwrap().event,
        HistoryEvent::Removed
    ));
    let write = library.rename(FULL_ID, "Birch 3 mm".to_owned(), 3).unwrap();
    assert!(written(&write).contains("\"name\": \"Birch 3 mm\""));
    assert_eq!(
        library.rename(FULL_ID, "Birch 3 mm".to_owned(), 3),
        Err(LibraryRefusal::Unchanged)
    );
    assert_eq!(
        library.rename(FULL_ID, "x".repeat(61), 3),
        Err(LibraryRefusal::InvalidName)
    );
    library
        .set_thickness(FULL_ID, Thickness::from_mm(4.0).unwrap(), 4)
        .unwrap();
    assert_eq!(library.find(FULL_ID).unwrap().thickness.mm(), 4.0);
    assert_eq!(
        library.find(FULL_ID).unwrap().history.len(),
        before.history.len() + 2
    );
}

#[test]
fn an_edit_past_500_entries_is_refused_and_writes_nothing() {
    let name = record_file_name(FULL_ID);
    let mut library = Materials::load(&[(name, MAX.to_owned())], None);
    let before = library.clone();
    assert_eq!(
        library.edit_set(FULL_ID, SetKind::Cut, settings(650, 300), id(30), 1),
        Err(LibraryRefusal::FullHistory {
            name: "Birch plywood".to_owned()
        })
    );
    assert_eq!(library, before);
    assert!(
        library.rename(FULL_ID, "Birch".to_owned(), 1).is_ok(),
        "no entry added"
    );
}

#[test]
fn delete_removes_readable_and_damaged_records_but_never_a_newer_one() {
    let (newer_name, newer_text) = record(3, "Oak", MACHINE);
    let files = vec![
        record(1, "Birch", MACHINE),
        (record_file_name(&id(2)), "{".to_owned()),
        (
            newer_name,
            newer_text.replace("\"format_version\": 1", "\"format_version\": 9"),
        ),
    ];
    let mut library = Materials::load(&files, None);
    assert_eq!(
        library.delete(&id(1)),
        Ok(LibraryWrite::Remove { id: id(1) })
    );
    assert_eq!(
        library.delete(&id(2)),
        Ok(LibraryWrite::Remove { id: id(2) })
    );
    assert_eq!(library.delete(&id(3)), Err(LibraryRefusal::Newer));
    assert_eq!(library.delete(&id(1)), Err(LibraryRefusal::Unknown));
    assert_eq!(library.reads().len(), 1);
}

#[test]
fn undo_and_redo_rewrite_a_record_only_when_it_is_what_the_step_left() {
    let mut library = Materials::load(&[(record_file_name(FULL_ID), FULL.to_owned())], None);
    let before = library.text(FULL_ID).unwrap().to_owned();
    let write = library
        .edit_set(FULL_ID, SetKind::Cut, settings(650, 300), id(40), 10)
        .unwrap();
    let after = written(&write).to_owned();
    let undone = library
        .revert(FULL_ID, Some(&after), Some(&before), 20)
        .unwrap();
    let material = library.find(FULL_ID).unwrap().clone();
    assert_eq!(material.set(SetKind::Cut).unwrap().power.tenths(), 600);
    assert_eq!(
        material.history.len(),
        5,
        "the entry the step added is gone (Frage 190 A)"
    );
    let modified_after = read_material(&record_file_name(FULL_ID), &after)
        .unwrap()
        .modified;
    assert_eq!(material.modified, next_modified(modified_after, 20));
    let undone_text = undone.text.clone().unwrap();
    assert_eq!(
        undone.write,
        Some(LibraryWrite::Write {
            id: FULL_ID.to_owned(),
            text: undone_text.clone()
        })
    );
    let redone = library
        .revert(FULL_ID, Some(&undone_text), Some(&after), 30)
        .unwrap();
    let material = library.find(FULL_ID).unwrap();
    assert_eq!(material.set(SetKind::Cut).unwrap().power.tenths(), 650);
    assert_eq!(
        material.history.last().unwrap().id,
        id(40),
        "redo adds the same entry"
    );
    assert!(redone.text.is_some());
    assert_eq!(
        library.revert(FULL_ID, Some(&after), Some(&before), 40),
        Err(RevertRefused::Changed),
        "the record changed since"
    );
}

#[test]
fn undo_of_a_new_material_removes_it_and_of_a_delete_brings_it_back() {
    let mut library = Materials::default();
    let write = library
        .save_test(
            new_material(1, "Birch"),
            test_result(SetKind::Cut),
            &id(50),
            1,
        )
        .unwrap();
    let after = written(&write).to_owned();
    assert_eq!(
        library.revert(&id(1), Some(&after), None, 2),
        Ok(Reverted {
            write: Some(LibraryWrite::Remove { id: id(1) }),
            text: None
        })
    );
    assert!(library.find(&id(1)).is_none());
    let back = library.revert(&id(1), None, Some(&after), 3).unwrap();
    assert!(library.find(&id(1)).is_some(), "redo of the save");
    library.delete(&id(1)).unwrap();
    let restored = library
        .revert(&id(1), None, back.text.as_deref(), 4)
        .unwrap();
    assert!(restored.write.is_some());
    assert_eq!(library.find(&id(1)).unwrap().name, "Birch");
    let damaged = "{".to_owned();
    let mut library = Materials::load(&[(record_file_name(&id(2)), damaged.clone())], None);
    library.delete(&id(2)).unwrap();
    let back = library.revert(&id(2), None, Some(&damaged), 5).unwrap();
    assert_eq!(
        back.text.as_deref(),
        Some("{"),
        "a damaged text goes back as it was"
    );
    assert!(matches!(library.reads()[0], MaterialRead::Damaged { .. }));
}
