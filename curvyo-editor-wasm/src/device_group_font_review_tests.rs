//! Session-level tests of what the reviews of the font sync asked for (`specs/0116-library-sync`
//! criteria 16 to 19, ADR 0062 decisions 3, 4 and 6): the receiving budget, a stuck font, a
//! replacing snapshot, a font switched on offline.

use curvyo_document_core::{AssetHash, CuttableChoice, LibraryList};
use curvyo_sync_core::{
    Millis, RecordKind, SessionEvent, encode_blob_parts, encode_library_record,
};

use super::DeviceGroupGlue;
use super::FontChange;
use super::font_tests::{desktop, font_bytes, input, laptop, notices};
use super::library_tests::{Net, ROOM, SERVER, identity, seeds};
use crate::session::Session;

/// Delivers an entry of a font that `other` peer switched on, as a record of the room.
fn announce(glue: &mut DeviceGroupGlue, seq: u64, hash: &AssetHash, name: &str, size: u64) {
    let other = LibraryList::from_snapshot(99, &glue.library().snapshot().unwrap()).unwrap();
    let before = other.version();
    other
        .font_set(
            hash,
            &curvyo_document_core::FontDetails {
                file_name: name.to_owned(),
                folder: None,
                size,
                tags: Vec::new(),
                favourite: false,
                cuttable: CuttableChoice::Auto,
                documents: Vec::new(),
            },
        )
        .unwrap();
    let entry = encode_library_record(99, &other.export_updates(&before).unwrap());
    glue.on_event(SessionEvent::Remote {
        seq,
        kind: RecordKind::Update,
        bytes: entry,
    });
}

fn parts_to(glue: &mut DeviceGroupGlue, seq: u64, parts: Vec<Vec<u8>>) {
    for part in parts {
        glue.on_event(SessionEvent::Remote {
            seq,
            kind: RecordKind::Asset,
            bytes: part,
        });
    }
}

/// Architect finding 1: a font whose bytes never arrive (nobody pushes them, the entry is a
/// stranger's) does not hold the snapshots back; a blob that is still arriving does, until it
/// completes or goes quiet for ten minutes.
#[test]
fn a_font_that_never_arrives_does_not_block_the_snapshot_a_half_arrived_one_does_until_stale() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(5 * 1024 * 1024, 12);
    let hash = AssetHash::of(&bytes);
    announce(
        &mut net.devices[a].glue,
        1,
        &hash,
        "never.ttf",
        bytes.len() as u64,
    );
    assert_eq!(net.devices[a].glue.font_wants().len(), 0);
    assert!(
        net.devices[a].glue.fonts_complete(),
        "nothing is arriving: the snapshot goes up without it"
    );

    let parts = encode_blob_parts(&hash.to_bytes(), &bytes);
    assert!(parts.len() > 1);
    parts_to(&mut net.devices[a].glue, 2, parts[..1].to_vec());
    assert!(!net.devices[a].glue.fonts_complete(), "parts are arriving");
    net.devices[a].glue.tick(Millis(net.now + 11 * 60_000));
    assert!(
        net.devices[a].glue.fonts_complete(),
        "ten quiet minutes: the half blob is given up"
    );
    parts_to(&mut net.devices[a].glue, 3, parts[1..].to_vec());
    assert_eq!(
        net.devices[a].glue.font_blob(&hash).len(),
        0,
        "its start is gone"
    );
}

/// Architect finding 2: the receiving side counts held and arriving bytes against the library
/// budget, refuses a blob that would pass it with a notice, refuses bytes that are not the
/// size the entry says, and makes room by dropping blobs no entry names.
#[test]
fn blobs_are_held_within_the_budget_and_the_size_the_entry_says() {
    let (mut net, a) = desktop();
    net.devices[a].glue.fonts.limit = 100_000;
    let first = font_bytes(60_000, 13);
    let second = font_bytes(60_000, 14);
    let (h1, h2) = (AssetHash::of(&first), AssetHash::of(&second));
    announce(&mut net.devices[a].glue, 1, &h1, "first.ttf", 60_000);
    announce(&mut net.devices[a].glue, 2, &h2, "second.ttf", 60_000);
    parts_to(
        &mut net.devices[a].glue,
        3,
        encode_blob_parts(&h1.to_bytes(), &first),
    );
    parts_to(
        &mut net.devices[a].glue,
        4,
        encode_blob_parts(&h2.to_bytes(), &second),
    );
    assert_eq!(net.devices[a].glue.font_blob(&h1).len(), 60_000);
    assert_eq!(
        net.devices[a].glue.font_blob(&h2).len(),
        0,
        "past the budget"
    );
    let told = notices(&mut net, a);
    assert!(
        told.iter().any(|n| n.starts_with("Not synced: second;")),
        "{told:?}"
    );

    let liar = font_bytes(50, 15);
    let hl = AssetHash::of(&liar);
    announce(&mut net.devices[a].glue, 5, &hl, "liar.ttf", 10);
    parts_to(
        &mut net.devices[a].glue,
        6,
        encode_blob_parts(&hl.to_bytes(), &liar),
    );
    assert_eq!(
        net.devices[a].glue.font_blob(&hl).len(),
        0,
        "size differs from the entry"
    );
    assert!(
        notices(&mut net, a)
            .iter()
            .any(|n| n == "A synced font could not be taken. It was skipped.")
    );

    // A blob no entry names is held only while there is room: a named one pushes it out.
    let (mut net, a) = desktop();
    net.devices[a].glue.fonts.limit = 100_000;
    let stray = font_bytes(60_000, 16);
    let hs = AssetHash::of(&stray);
    parts_to(
        &mut net.devices[a].glue,
        1,
        encode_blob_parts(&hs.to_bytes(), &stray),
    );
    assert_eq!(net.devices[a].glue.font_blob(&hs).len(), 60_000);
    let named = font_bytes(60_000, 17);
    let hn = AssetHash::of(&named);
    announce(&mut net.devices[a].glue, 2, &hn, "named.ttf", 60_000);
    parts_to(
        &mut net.devices[a].glue,
        3,
        encode_blob_parts(&hn.to_bytes(), &named),
    );
    assert_eq!(net.devices[a].glue.font_blob(&hn).len(), 60_000);
    assert_eq!(net.devices[a].glue.font_blob(&hs).len(), 0);
    assert_eq!(notices(&mut net, a), Vec::<String>::new());
}

/// Architect finding 3: a snapshot that replaces the list never reads as the deletion of a font
/// (a font switched on offline is missing from it): no removal, no "deleted" notice.
#[test]
fn a_replacing_snapshot_removes_no_font() {
    let (mut net, a) = desktop();
    let earlier = net.devices[a].glue.library().snapshot().unwrap();
    let hash = net.devices[a]
        .glue
        .font_set(input("mine.ttf"), font_bytes(30_000, 18))
        .unwrap();
    net.devices[a].page();
    net.devices[a].font_arrivals.clear();
    let _ = notices(&mut net, a);
    // The room's list has its own history from before the font and moved on without it.
    let room = LibraryList::from_snapshot(77, &earlier).unwrap();
    room.set(
        curvyo_document_core::LibraryKind::Machine,
        "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f",
        "{}",
    )
    .unwrap();
    room.set(
        curvyo_document_core::LibraryKind::Machine,
        "7f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f",
        "{}",
    )
    .unwrap();
    let state = room.snapshot_state().unwrap();
    net.devices[a]
        .glue
        .snapshot_stream(&curvyo_sync_core::StreamEntry {
            stream: curvyo_sync_core::LIBRARY_STREAM_ID,
            bytes: encode_library_record(77, &state),
        });
    net.devices[a].page();
    assert!(
        net.devices[a].glue.library().font_entry(&hash).is_none(),
        "the snapshot replaced the list"
    );
    assert!(
        net.devices[a]
            .font_arrivals
            .iter()
            .all(|arrival| arrival.change != FontChange::Removed),
        "{:?}",
        net.devices[a].font_arrivals
    );
    assert!(
        !notices(&mut net, a)
            .iter()
            .any(|n| n.contains("was deleted")),
        "no false deleted notice"
    );
    let _ = hash;
}

/// Architect finding 1: a font switched on while the group is unreachable sends its blob once
/// the group is online.
#[test]
fn a_font_switched_on_offline_sends_its_blob_when_online() {
    let mut net = Net::new();
    let a = net.add(
        DeviceGroupGlue::create(SERVER, ROOM, "Desktop", &seeds(1), 11, Millis(net.now)).unwrap(),
        Session::new(11),
    );
    let bytes = font_bytes(120_000, 19);
    let hash = net.devices[a]
        .glue
        .font_set(input("offline.ttf"), bytes.clone())
        .unwrap();
    net.run(500);
    net.devices[a].glue.add_device(identity(2), "Laptop");
    net.run(200);
    let b = laptop(&mut net);
    assert_eq!(net.devices[b].glue.font_blob(&hash), bytes.as_slice());
}

/// A font whose bytes this host holds but the page has not supplied yet holds the snapshot back
/// (after a restart the glue has no bytes of its own fonts), for ten minutes at most.
#[test]
fn a_snapshot_waits_for_bytes_the_host_holds_but_not_for_ever() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(80_000, 20);
    let hash = AssetHash::of(&bytes);
    announce(&mut net.devices[a].glue, 1, &hash, "own.ttf", 80_000);
    net.devices[a]
        .glue
        .font_host_hashes(std::iter::once(hash.clone()).collect());
    assert!(
        !net.devices[a].glue.fonts_complete(),
        "the page will supply it"
    );
    net.devices[a].glue.font_supply_blob(&hash, bytes);
    assert!(net.devices[a].glue.fonts_complete());

    let lost = font_bytes(80_000, 21);
    let hl = AssetHash::of(&lost);
    announce(&mut net.devices[a].glue, 2, &hl, "lost.ttf", 80_000);
    net.devices[a]
        .glue
        .font_host_hashes([hash, hl].into_iter().collect());
    assert!(!net.devices[a].glue.fonts_complete());
    net.devices[a].glue.tick(Millis(net.now + 11 * 60_000));
    assert!(
        !net.devices[a].glue.fonts.snapshot_deferred,
        "the page never read it: the deferred snapshot went up after the patience"
    );
}
