//! Session-level tests of the font sync (`specs/0116-library-sync` criteria 16 to 19, ADR 0062
//! decisions 2 to 6): two devices' groups in one in-memory relay, the page's pump included.

use std::collections::BTreeSet;

use curvyo_document_core::{AssetHash, CuttableChoice, LibraryList};
use curvyo_sync_core::{
    Millis, RecordKind, SessionEvent, SnapshotAssembler, decode_blob_entry, encode_blob_parts,
    encode_library_record, split_snapshot,
};

use super::library_tests::{Net, ROOM, SERVER, identity, seeds};
use super::{DeviceGroupGlue, FontChange, FontInput};
use crate::session::Session;

/// A deterministic font stand-in of `len` bytes.
pub(super) fn font_bytes(len: usize, seed: u8) -> Vec<u8> {
    (0..len)
        .map(|n| u8::try_from(n % 251).unwrap().wrapping_add(seed))
        .collect()
}

pub(super) fn input(name: &str) -> FontInput {
    FontInput {
        file_name: name.to_owned(),
        folder: None,
        tags: vec!["own:ZQX-TAG-7731".to_owned()],
        favourite: true,
        cuttable: CuttableChoice::Auto,
        documents: Vec::new(),
    }
}

/// A desktop A that is online with a paired-in laptop identity; B is not yet added.
pub(super) fn desktop() -> (Net, usize) {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    net.run(500);
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    (net, a)
}

pub(super) fn laptop(net: &mut Net) -> usize {
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.run(2_000);
    b
}

pub(super) fn notices(net: &mut Net, at: usize) -> Vec<String> {
    let mut out = Vec::new();
    while let Some(notice) = net.devices[at].glue.take_notice() {
        out.push(notice);
    }
    out
}

/// Criteria 16 and 18: A switches a font of two parts on, with a licence file; B joins and
/// receives the file bytes with the same SHA-256, the metadata and the licence file, once, with
/// the notice, and its toggle reads on.
#[test]
fn a_font_arrives_with_its_bytes_metadata_and_licence_file() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(5 * 1024 * 1024, 0);
    let licence = b"SIL Open Font License".to_vec();
    let mut font = input("ZQX-FONT-7731.ttf");
    font.folder = Some("ZQX-FONT-7731".to_owned());
    font.documents = vec![("OFL.txt".to_owned(), licence.clone())];
    let hash = net.devices[a].glue.font_set(font, bytes.clone()).unwrap();
    assert_eq!(hash, AssetHash::of(&bytes));
    net.run(1_000);
    let b = laptop(&mut net);

    let arrivals = &net.devices[b].font_arrivals;
    let adds: Vec<_> = arrivals
        .iter()
        .filter(|a| matches!(a.change, FontChange::Add(_)))
        .collect();
    assert_eq!(adds.len(), 1, "one Add, not one per part: {arrivals:?}");
    let FontChange::Add(details) = &adds[0].change else {
        unreachable!()
    };
    assert_eq!(adds[0].hash, hash);
    assert_eq!(details.file_name, "ZQX-FONT-7731.ttf");
    assert_eq!(details.folder.as_deref(), Some("ZQX-FONT-7731"));
    assert!(details.favourite);
    assert_eq!(details.tags, ["own:ZQX-TAG-7731"]);
    let blob = net.devices[b].glue.font_blob(&hash);
    assert_eq!(
        AssetHash::of(blob),
        hash,
        "the same SHA-256 on both devices"
    );
    assert_eq!(
        net.devices[b].glue.font_blob(&details.documents[0].hash),
        licence.as_slice()
    );
    assert!(
        notices(&mut net, b)
            .iter()
            .any(|n| n == "Font ZQX-FONT-7731 arrived from your devices.")
    );
    let view: serde_json::Value =
        serde_json::from_str(&net.devices[b].glue.library_view_json()).unwrap();
    assert_eq!(view["fonts"][0]["id"], hash.as_str());
    assert_eq!(view["fonts"][0]["sync"], true);
}

/// Criterion 16 (collection): a file is one entry and one blob, however many faces it has; and a
/// hash the host holds already is never asked for again (ADR 0062 decision 3): the arrival is
/// the metadata only.
#[test]
fn a_font_the_host_holds_arrives_as_metadata_without_the_bytes() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(100_000, 1);
    let hash = AssetHash::of(&bytes);
    net.devices[a]
        .glue
        .font_set(input("held.ttf"), bytes)
        .unwrap();
    net.run(500);
    let b = net.add(
        DeviceGroupGlue::join(SERVER, ROOM, "Laptop", &seeds(2), 12, Millis(net.now)).unwrap(),
        Session::new(12),
    );
    net.devices[b].host = std::iter::once(hash.clone()).collect::<BTreeSet<_>>();
    net.run(2_000);
    let kinds: Vec<_> = net.devices[b]
        .font_arrivals
        .iter()
        .map(|a| matches!(a.change, FontChange::Meta(_)))
        .collect();
    assert_eq!(kinds, [true], "{:?}", net.devices[b].font_arrivals);
}

/// A device that holds nothing of a font and receives its blob before its entry, and one that
/// receives the entry before the blob, both end with exactly one Add (the order of the room's
/// records does not matter).
#[test]
fn the_blob_and_the_entry_may_arrive_in_either_order() {
    for blob_first in [true, false] {
        let (mut net, a) = desktop();
        let b = laptop(&mut net);
        assert!(net.devices[b].glue.is_online());
        let bytes = font_bytes(300_000, 2);
        let hash = AssetHash::of(&bytes);
        let other =
            LibraryList::from_snapshot(99, &net.devices[a].glue.library().snapshot().unwrap())
                .unwrap();
        let before = other.version();
        other
            .font_set(
                &hash,
                &curvyo_document_core::FontDetails {
                    file_name: "order.ttf".to_owned(),
                    folder: None,
                    size: bytes.len() as u64,
                    tags: Vec::new(),
                    favourite: false,
                    cuttable: CuttableChoice::Auto,
                    documents: Vec::new(),
                },
            )
            .unwrap();
        let entry = encode_library_record(99, &other.export_updates(&before).unwrap());
        let parts = encode_blob_parts(&hash.to_bytes(), &bytes);
        let glue = &mut net.devices[b].glue;
        let mut records = vec![(RecordKind::Update, entry)];
        records.extend(parts.into_iter().map(|p| (RecordKind::Asset, p)));
        if blob_first {
            records.rotate_left(1);
        }
        for (seq, (kind, bytes)) in (1..).zip(records) {
            glue.on_event(SessionEvent::Remote { seq, kind, bytes });
        }
        net.devices[b].page();
        let adds = net.devices[b]
            .font_arrivals
            .iter()
            .filter(|a| matches!(a.change, FontChange::Add(_)))
            .count();
        assert_eq!(adds, 1, "blob_first = {blob_first}");
    }
}

/// Criterion 19: a font over 16 MiB is refused with the reason and the numbers; no entry, no
/// blob, nothing sent; a font of exactly 16 MiB goes.
#[test]
fn a_font_over_the_per_file_limit_writes_nothing() {
    let (mut net, a) = desktop();
    let version = net.devices[a].glue.library().version();
    let refusal = net.devices[a]
        .glue
        .font_set(input("Giant.ttf"), font_bytes(16 * 1024 * 1024 + 1, 3))
        .unwrap_err();
    assert_eq!(refusal, "Not synced: Giant is 16.1 MiB, over 16 MiB.");
    assert_eq!(net.devices[a].glue.library().version(), version);
    assert_eq!(net.devices[a].glue.library().font_entries().len(), 0);
    assert_eq!(net.devices[a].glue.font_wants().len(), 0);
    assert!(
        net.devices[a]
            .glue
            .font_set(input("Fits.ttf"), font_bytes(16 * 1024 * 1024, 4))
            .is_ok()
    );
}

/// ADR 0062 decision 3 and the snapshot rule of plan P23: the room's snapshot carries the blobs
/// of switched-on fonts only, so a switched-off font's bytes leave the server with the next
/// snapshot (criteria 4 and 27).
#[test]
fn the_snapshot_carries_the_bytes_of_on_fonts_only() {
    let (mut net, a) = desktop();
    let kept = font_bytes(50_000, 5);
    let dropped = font_bytes(60_000, 6);
    let kept_hash = net.devices[a]
        .glue
        .font_set(input("kept.ttf"), kept)
        .unwrap();
    let dropped_hash = net.devices[a]
        .glue
        .font_set(input("dropped.ttf"), dropped)
        .unwrap();
    let blob_hashes = |glue: &DeviceGroupGlue| -> Vec<AssetHash> {
        let stream = glue.group_snapshot_stream().unwrap();
        let mut assembler = SnapshotAssembler::default();
        let mut entries = Vec::new();
        for part in split_snapshot(&stream, [7; 16]) {
            entries.extend(assembler.push_part(&part).unwrap());
        }
        entries
            .iter()
            .filter_map(decode_blob_entry)
            .map(|(hash, _)| AssetHash::from_bytes(&hash))
            .collect()
    };
    assert_eq!(blob_hashes(&net.devices[a].glue).len(), 2);
    net.devices[a].glue.font_switch_off(&dropped_hash);
    assert_eq!(blob_hashes(&net.devices[a].glue), [kept_hash]);
    assert_eq!(net.devices[a].glue.font_blob(&dropped_hash).len(), 0);
}

/// A device that learns of a switched-on font but does not hold its bytes uploads no snapshot
/// (the relay would prune the blob records): `font_wants` names what the page has to read,
/// a wrong blob is ignored and the right one completes it.
#[test]
fn no_snapshot_goes_up_while_an_on_font_lacks_its_bytes() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(80_000, 7);
    let hash = AssetHash::of(&bytes);
    let other =
        LibraryList::from_snapshot(99, &net.devices[a].glue.library().snapshot().unwrap()).unwrap();
    let before = other.version();
    other
        .font_set(
            &hash,
            &curvyo_document_core::FontDetails {
                file_name: "late.ttf".to_owned(),
                folder: None,
                size: bytes.len() as u64,
                tags: Vec::new(),
                favourite: false,
                cuttable: CuttableChoice::Auto,
                documents: Vec::new(),
            },
        )
        .unwrap();
    let entry = encode_library_record(99, &other.export_updates(&before).unwrap());
    net.devices[a].glue.on_event(SessionEvent::Remote {
        seq: 1,
        kind: RecordKind::Update,
        bytes: entry,
    });
    net.devices[a]
        .glue
        .font_host_hashes(std::iter::once(hash.clone()).collect::<BTreeSet<_>>());
    assert_eq!(
        net.devices[a].glue.font_wants(),
        std::slice::from_ref(&hash)
    );
    net.devices[a].glue.font_supply_blob(&hash, vec![1, 2, 3]);
    assert_eq!(
        net.devices[a].glue.font_wants(),
        std::slice::from_ref(&hash),
        "a wrong blob is ignored"
    );
    net.devices[a].glue.font_supply_blob(&hash, bytes);
    assert_eq!(net.devices[a].glue.font_wants().len(), 0);
}

/// Criterion 17: a star set on A reaches B as a metadata change, silently; two offline-style
/// concurrent edits of one field converge to one value on both and the device whose edit lost
/// says which field.
#[test]
fn metadata_converges_and_a_lost_edit_names_the_field() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(40_000, 8);
    let hash = net.devices[a]
        .glue
        .font_set(input("ZQX-FONT-7731.ttf"), bytes)
        .unwrap();
    net.run(500);
    let b = laptop(&mut net);
    net.devices[b].font_arrivals.clear();
    let _ = notices(&mut net, b);

    net.devices[a].glue.font_meta(
        &hash,
        vec!["own:ZQX-TAG-7731".to_owned()],
        false,
        CuttableChoice::Auto,
    );
    net.run(500);
    let seen: Vec<_> = net.devices[b]
        .font_arrivals
        .iter()
        .filter_map(|a| match &a.change {
            FontChange::Meta(d) => Some(d.favourite),
            _ => None,
        })
        .collect();
    assert_eq!(
        seen,
        [false],
        "the star change arrives as a metadata change"
    );
    assert!(notices(&mut net, b).is_empty(), "a plain change is silent");

    // Both change the tag register at once, before either hears of the other.
    net.devices[a].glue.font_meta(
        &hash,
        vec!["own:Wedding".to_owned()],
        false,
        CuttableChoice::Auto,
    );
    net.devices[b].glue.font_meta(
        &hash,
        vec!["own:Signs".to_owned()],
        false,
        CuttableChoice::Auto,
    );
    net.run(1_000);
    let tags_of = |net: &Net, at: usize| {
        net.devices[at]
            .glue
            .library()
            .font_entry(&hash)
            .unwrap()
            .details
            .unwrap()
            .tags
    };
    assert_eq!(tags_of(&net, a), tags_of(&net, b));
    let told: Vec<_> = [a, b]
        .into_iter()
        .flat_map(|at| notices(&mut net, at))
        .filter(|n| n == "ZQX-FONT-7731: newer tags from your devices kept.")
        .collect();
    assert_eq!(told.len(), 1, "exactly the device whose edit lost says so");
}

/// ADR 0062 decision 4 and criterion 11: a font deleted on A reaches B as a removal with the
/// deleted notice; a switch-off reaches it as the "copy stays here" notice and no removal.
#[test]
fn a_deleted_font_is_removed_elsewhere_and_a_switch_off_keeps_the_copy() {
    let (mut net, a) = desktop();
    let first = net.devices[a]
        .glue
        .font_set(input("gone.ttf"), font_bytes(30_000, 9))
        .unwrap();
    let second = net.devices[a]
        .glue
        .font_set(input("kept.ttf"), font_bytes(30_000, 10))
        .unwrap();
    net.run(500);
    let b = laptop(&mut net);
    let _ = notices(&mut net, b);
    net.devices[b].font_arrivals.clear();

    net.devices[a].glue.font_switch_off(&second);
    net.devices[a].glue.font_remove(&first);
    net.run(1_500);
    let removed: Vec<_> = net.devices[b]
        .font_arrivals
        .iter()
        .filter(|a| a.change == FontChange::Removed)
        .map(|a| a.hash.clone())
        .collect();
    assert_eq!(removed, [first]);
    let told = notices(&mut net, b);
    assert!(
        told.iter()
            .any(|n| n == "gone was deleted on another device."),
        "{told:?}"
    );
    assert!(
        told.iter().any(|n| n == "Sync off, this copy stays here."),
        "{told:?}"
    );
}

/// A blob that does not hash to what its parts claim is dropped with a notice and never becomes
/// an arrival.
#[test]
fn a_blob_that_does_not_match_its_hash_is_dropped() {
    let (mut net, a) = desktop();
    let b = laptop(&mut net);
    let _ = (a, notices(&mut net, b));
    let bytes = font_bytes(10_000, 11);
    let wrong = [0x42; 32];
    for part in encode_blob_parts(&wrong, &bytes) {
        net.devices[b].glue.on_event(SessionEvent::Remote {
            seq: 1,
            kind: RecordKind::Asset,
            bytes: part,
        });
    }
    assert!(
        notices(&mut net, b)
            .iter()
            .any(|n| n == "A synced font could not be taken. It was skipped.")
    );
    assert_eq!(
        net.devices[b]
            .glue
            .font_blob(&AssetHash::from_bytes(&wrong))
            .len(),
        0
    );
}
