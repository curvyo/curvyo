//! The font entries of the synced library list (`specs/0116-library-sync` criteria 16 to 19,
//! ADR 0062 decisions 2, 4 and 6): registers per field, the marker of a switched-off font, the
//! budgets and the format version.

#![allow(clippy::unwrap_used, missing_docs)]

use curvyo_document_core::{
    AssetHash, CuttableChoice, FontBudgetError, FontDetails, FontDocument, LibraryKind,
    LibraryList, LibraryListError, MAX_SYNCED_FONT_BYTES, MAX_SYNCED_FONTS_BYTES,
};
use loro::{LoroValue, ValueOrContainer};

fn hash(seed: u8) -> AssetHash {
    AssetHash::of(&[seed; 32])
}

fn details(name: &str, size: u64) -> FontDetails {
    FontDetails {
        file_name: name.to_owned(),
        folder: None,
        size,
        tags: vec!["ZQX-TAG-7731".to_owned()],
        favourite: true,
        cuttable: CuttableChoice::Yes,
        documents: Vec::new(),
    }
}

fn format_of(list: &LibraryList) -> i64 {
    let doc = loro::LoroDoc::new();
    doc.import(&list.snapshot().unwrap()).unwrap();
    match doc.get_map("library").get("format_version") {
        Some(ValueOrContainer::Value(LoroValue::I64(version))) => version,
        other => panic!("no format version: {other:?}"),
    }
}

/// Criteria 16 and 18: an entry carries file name, folder, size, tags, star, Cuttable and the
/// kept documents, and reads back as written.
#[test]
fn a_font_entry_round_trips_with_its_documents() {
    let list = LibraryList::new(1);
    let mut font = details("ZQX-FONT-7731.ttf", 12_345);
    font.folder = Some("Lobster".to_owned());
    font.documents = vec![FontDocument {
        hash: hash(9),
        size: 4_096,
        name: "OFL.txt".to_owned(),
    }];
    list.font_set(&hash(1), &font).unwrap();
    let entry = list.font_entry(&hash(1)).unwrap();
    assert!(entry.sync);
    assert_eq!(entry.details.as_ref(), Some(&font));
    assert_eq!(list.font_entries().len(), 1);
}

/// Setting what is already there writes nothing (the receiver's own Add-font write must not echo
/// a push).
#[test]
fn setting_the_same_details_twice_writes_nothing() {
    let list = LibraryList::new(1);
    list.font_set(&hash(1), &details("a.ttf", 10)).unwrap();
    let version = list.version();
    list.font_set(&hash(1), &details("a.ttf", 10)).unwrap();
    assert_eq!(list.version(), version);
}

/// Decision of the plan (P22): a library without fonts stays format 1 for builds that read only
/// that; the first font entry makes it 2.
#[test]
fn the_first_font_entry_makes_the_list_format_two() {
    let list = LibraryList::new(1);
    assert_eq!(format_of(&list), 1);
    list.set(
        LibraryKind::Machine,
        "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f",
        "{}",
    )
    .unwrap();
    assert_eq!(format_of(&list), 1);
    list.font_set(&hash(1), &details("a.ttf", 10)).unwrap();
    assert_eq!(format_of(&list), 2);
}

/// A list of a format newer than 2 is still refused (ADR 0040 decision 11).
#[test]
fn a_list_of_format_three_is_refused() {
    let doc = loro::LoroDoc::new();
    doc.get_map("library").insert("format_version", 3).unwrap();
    doc.commit();
    assert_eq!(
        LibraryList::from_snapshot(1, &doc.export(loro::ExportMode::Snapshot).unwrap()).err(),
        Some(LibraryListError::FormatTooNew)
    );
}

/// The registers of the `fonts` map a state-only snapshot holds, as `key=value` lines.
fn state_registers(list: &LibraryList) -> Vec<String> {
    let doc = loro::LoroDoc::new();
    doc.import(&list.snapshot_state().unwrap()).unwrap();
    let mut lines = Vec::new();
    doc.get_map("fonts").for_each(|key, value| {
        if let ValueOrContainer::Value(value) = value {
            lines.push(format!("{key}={value:?}"));
        }
    });
    lines.sort();
    lines
}

/// Criteria 4 and 27 (fonts): a switched-off font leaves only its marker; the state-only
/// snapshot, which the room's snapshot carries, holds no name, tag or size of it.
#[test]
fn a_switched_off_font_leaves_no_name_or_tag_in_the_state_snapshot() {
    let list = LibraryList::new(1);
    list.font_set(&hash(1), &details("ZQX-FONT-7731.ttf", 10))
        .unwrap();
    let on = state_registers(&list).join(" ");
    assert!(on.contains("ZQX-FONT-7731.ttf") && on.contains("ZQX-TAG-7731"));
    list.font_switch_off(&hash(1)).unwrap();
    let off = list.font_entry(&hash(1)).unwrap();
    assert!(!off.sync);
    assert_eq!(off.details, None);
    let state = state_registers(&list);
    assert_eq!(state.len(), 1, "{state:?}");
    assert!(state[0].ends_with(".sync=Bool(false)"), "{state:?}");
}

/// Deleting a font removes its entry altogether; an unknown hash does nothing.
#[test]
fn a_removed_font_has_no_entry() {
    let list = LibraryList::new(1);
    list.font_set(&hash(1), &details("a.ttf", 10)).unwrap();
    list.font_remove(&hash(2)).unwrap();
    list.font_remove(&hash(1)).unwrap();
    assert!(list.font_entry(&hash(1)).is_none());
    assert_eq!(list.font_entries().len(), 0);
}

/// A switch-off of a font that never synced writes nothing (criterion 4).
#[test]
fn switching_off_an_unknown_font_writes_nothing() {
    let list = LibraryList::new(1);
    let version = list.version();
    list.font_switch_off(&hash(1)).unwrap();
    assert_eq!(list.version(), version);
}

/// A name that is a path or empty is refused and nothing is written.
#[test]
fn a_name_that_is_not_a_plain_file_name_is_refused() {
    let list = LibraryList::new(1);
    for name in ["", "..", "a/b.ttf", "a\\b.ttf"] {
        assert_eq!(
            list.font_set(&hash(1), &details(name, 10)),
            Err(LibraryListError::BadId),
            "{name:?}"
        );
    }
    assert_eq!(list.font_entries().len(), 0);
}

/// Criterion 19: a file over 16 MiB is refused with its size and the limit; 16 MiB itself fits.
#[test]
fn a_file_over_the_per_file_limit_is_refused_with_the_numbers() {
    let list = LibraryList::new(1);
    assert_eq!(
        list.font_check_budget(&hash(1), &details("big.ttf", MAX_SYNCED_FONT_BYTES + 1)),
        Err(FontBudgetError::TooLarge {
            size: MAX_SYNCED_FONT_BYTES + 1,
            limit: MAX_SYNCED_FONT_BYTES
        })
    );
    assert_eq!(
        list.font_check_budget(&hash(1), &details("ok.ttf", MAX_SYNCED_FONT_BYTES)),
        Ok(())
    );
}

/// Criterion 19: the 33rd font of 16 MiB passes the 512 MiB library budget and is refused with
/// the total it would reach; a font that is on already is never refused for its own bytes; a
/// licence file two entries share counts once.
#[test]
fn the_library_budget_counts_each_hash_once() {
    let list = LibraryList::new(1);
    for seed in 0..32u8 {
        let font = details("f.ttf", MAX_SYNCED_FONT_BYTES);
        list.font_check_budget(&hash(seed), &font).unwrap();
        list.font_set(&hash(seed), &font).unwrap();
    }
    assert_eq!(list.font_total_bytes(), MAX_SYNCED_FONTS_BYTES);
    assert_eq!(
        list.font_check_budget(&hash(100), &details("g.ttf", 1)),
        Err(FontBudgetError::OverBudget {
            total: MAX_SYNCED_FONTS_BYTES + 1,
            limit: MAX_SYNCED_FONTS_BYTES
        })
    );
    assert_eq!(
        list.font_check_budget(&hash(3), &details("f.ttf", MAX_SYNCED_FONT_BYTES)),
        Ok(())
    );
    let licence = FontDocument {
        hash: hash(200),
        size: 100,
        name: "OFL.txt".to_owned(),
    };
    let other = LibraryList::new(2);
    for seed in 1..=2u8 {
        let mut font = details("f.ttf", 1_000);
        font.documents = vec![licence.clone()];
        other.font_set(&hash(seed), &font).unwrap();
    }
    assert_eq!(other.font_total_bytes(), 2_000 + 100);
}

/// Criterion 17: two devices edit different fields of one font offline; both edits survive.
/// Both edit the same field; the later writer wins on both.
#[test]
fn offline_edits_merge_per_field_and_the_newer_writer_wins_a_field() {
    let a = LibraryList::new(1);
    a.font_set(&hash(1), &details("a.ttf", 10)).unwrap();
    let b = LibraryList::from_snapshot(2, &a.snapshot().unwrap()).unwrap();

    let mut on_a = details("a.ttf", 10);
    on_a.favourite = false;
    a.font_set(&hash(1), &on_a).unwrap();
    let mut on_b = details("a.ttf", 10);
    on_b.cuttable = CuttableChoice::No;
    b.font_set(&hash(1), &on_b).unwrap();

    let to_b = a.export_updates(&b.version()).unwrap();
    let to_a = b.export_updates(&a.version()).unwrap();
    b.import_updates(&to_b).unwrap();
    a.import_updates(&to_a).unwrap();

    let merged_a = a.font_entry(&hash(1)).unwrap().details.unwrap();
    let merged_b = b.font_entry(&hash(1)).unwrap().details.unwrap();
    assert_eq!(merged_a, merged_b);
    assert!(!merged_a.favourite);
    assert_eq!(merged_a.cuttable, CuttableChoice::No);
}

/// Entries with a bad shape are skipped, the good ones read.
#[test]
fn an_entry_of_a_bad_shape_is_skipped() {
    let list = LibraryList::new(1);
    list.font_set(&hash(1), &details("a.ttf", 10)).unwrap();
    let doc = loro::LoroDoc::new();
    doc.import(&list.snapshot().unwrap()).unwrap();
    let map = doc.get_map("fonts");
    map.insert(&format!("{}.sync", hash(2).as_str()), "yes")
        .unwrap();
    map.insert("not-a-hash.sync", true).unwrap();
    doc.commit();
    let read =
        LibraryList::from_snapshot(5, &doc.export(loro::ExportMode::Snapshot).unwrap()).unwrap();
    assert_eq!(read.font_entries().len(), 1);
}

/// The raw bytes of a hash and back are the same hash.
#[test]
fn a_hash_round_trips_through_its_raw_bytes() {
    let hash = hash(7);
    assert_eq!(AssetHash::from_bytes(&hash.to_bytes()), hash);
}

/// A peer's entry over the per-file limits is not read at all (ADR 0062 decision 6, receiving
/// side): a font over 16 MiB, or a licence file over 1 MiB.
#[test]
fn entries_over_the_limits_are_not_read() {
    let list = LibraryList::new(1);
    list.font_set(&hash(1), &details("big.ttf", MAX_SYNCED_FONT_BYTES + 1))
        .unwrap();
    let mut heavy_doc = details("doc.ttf", 10);
    heavy_doc.documents = vec![FontDocument {
        hash: hash(9),
        size: 1024 * 1024 + 1,
        name: "OFL.txt".to_owned(),
    }];
    list.font_set(&hash(2), &heavy_doc).unwrap();
    list.font_set(&hash(4), &details("ok.ttf", MAX_SYNCED_FONT_BYTES))
        .unwrap();
    let hashes: Vec<_> = list.font_entries().into_iter().map(|e| e.hash).collect();
    assert_eq!(hashes, vec![hash(4)]);
    assert!(list.font_entry(&hash(1)).is_none());
    assert!(list.font_entry(&hash(4)).is_some());
}

/// More than 16 licence files in an entry (a peer's register; this build never writes it): the
/// entry is not read.
#[test]
fn an_entry_with_too_many_licence_files_is_not_read() {
    let list = LibraryList::new(1);
    list.font_set(&hash(1), &details("a.ttf", 10)).unwrap();
    let doc = loro::LoroDoc::new();
    doc.import(&list.snapshot().unwrap()).unwrap();
    let lines = |count: u8| {
        (0..count)
            .map(|n| format!("{} 10 L{n}.txt", hash(100 + n).as_str()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let key = format!("{}.documents", hash(1).as_str());
    for (count, read) in [(16_u8, true), (17, false)] {
        doc.get_map("fonts").insert(&key, lines(count)).unwrap();
        doc.commit();
        let edited =
            LibraryList::from_snapshot(2, &doc.export(loro::ExportMode::Snapshot).unwrap())
                .unwrap();
        assert_eq!(edited.font_entry(&hash(1)).is_some(), read, "{count} files");
    }
}

/// Reader golden: a version 2 list with one font entry on, one marker and a machine, written
/// by this build, reads back the same (ADR 0040 decision 11 keeps old goldens readable).
#[test]
fn a_version_two_list_golden_reads_back() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/library_list_v2_fonts.loro");
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        let list = LibraryList::new(7);
        list.set(
            LibraryKind::Machine,
            "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f",
            "{}",
        )
        .unwrap();
        let mut with_docs = details("ZQX-FONT-7731.ttf", 1_000);
        with_docs.folder = Some("Source Serif".to_owned());
        with_docs.documents = vec![FontDocument {
            hash: hash(5),
            size: 12,
            name: "OFL.txt".to_owned(),
        }];
        list.font_set(&hash(1), &with_docs).unwrap();
        list.font_set(&hash(2), &details("off.ttf", 10)).unwrap();
        list.font_switch_off(&hash(2)).unwrap();
        std::fs::write(&path, list.snapshot().unwrap()).unwrap();
    }
    let list = LibraryList::from_snapshot(1, &std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(format_of(&list), 2);
    let on = list.font_entry(&hash(1)).unwrap();
    assert!(on.sync);
    let details = on.details.unwrap();
    assert_eq!(details.file_name, "ZQX-FONT-7731.ttf");
    assert_eq!(details.folder.as_deref(), Some("Source Serif"));
    assert_eq!(details.size, 1_000);
    assert_eq!(details.tags, vec!["ZQX-TAG-7731".to_owned()]);
    assert!(details.favourite);
    assert_eq!(details.cuttable, CuttableChoice::Yes);
    assert_eq!(details.documents.len(), 1);
    let off = list.font_entry(&hash(2)).unwrap();
    assert!(!off.sync && off.details.is_none());
    assert_eq!(list.entries(LibraryKind::Machine).len(), 1);
}
