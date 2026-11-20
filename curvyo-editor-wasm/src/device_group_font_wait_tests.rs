//! Tests of the waits and the part handling of the font blobs (`specs/0116-library-sync` slice 2,
//! ADR 0062 decisions 3 and 6, architect re-review 5 and 6): the total wait per hash is capped,
//! and parts of a blob that is held or interleaved raise no false notice.

use curvyo_document_core::{AssetHash, CuttableChoice, FontDetails, LibraryList};
use curvyo_sync_core::{
    Millis, RecordKind, SessionEvent, encode_blob_parts, encode_library_record,
};

use super::DeviceGroupGlue;
use super::font_tests::{desktop, font_bytes, notices};

fn announce(glue: &mut DeviceGroupGlue, seq: u64, hash: &AssetHash, name: &str, size: u64) {
    let other = LibraryList::from_snapshot(99, &glue.library().snapshot().unwrap()).unwrap();
    let before = other.version();
    other
        .font_set(
            hash,
            &FontDetails {
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
    glue.on_event(SessionEvent::Remote {
        seq,
        kind: RecordKind::Update,
        bytes: encode_library_record(99, &other.export_updates(&before).unwrap()),
    });
}

fn part(glue: &mut DeviceGroupGlue, seq: u64, bytes: &[u8]) {
    glue.on_event(SessionEvent::Remote {
        seq,
        kind: RecordKind::Asset,
        bytes: bytes.to_vec(),
    });
}

/// A member that restarts the same blob every nine minutes does not hold the snapshots back for
/// ever: the wait for a hash is capped.
#[test]
fn a_restarting_sender_cannot_hold_the_snapshot_back_for_ever() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(6 * 1024 * 1024, 40);
    let hash = AssetHash::of(&bytes);
    announce(
        &mut net.devices[a].glue,
        1,
        &hash,
        "slow.ttf",
        bytes.len() as u64,
    );
    let parts = encode_blob_parts(&hash.to_bytes(), &bytes);
    part(&mut net.devices[a].glue, 2, &parts[0]);
    assert!(!net.devices[a].glue.fonts_complete(), "parts are arriving");
    for round in 1..=4 {
        net.devices[a]
            .glue
            .tick(Millis(net.now + round * 9 * 60_000));
        part(&mut net.devices[a].glue, 2 + round, &parts[0]);
    }
    assert!(
        net.devices[a].glue.fonts_complete(),
        "36 minutes after the first part the wait is over"
    );
}

/// A hash the page was asked for waits once, not again for every snapshot.
#[test]
fn the_wait_for_the_pages_supply_is_not_renewed_by_every_snapshot() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(80_000, 41);
    let hash = AssetHash::of(&bytes);
    announce(&mut net.devices[a].glue, 1, &hash, "own.ttf", 80_000);
    net.devices[a]
        .glue
        .font_host_hashes(std::iter::once(hash).collect());
    assert!(!net.devices[a].glue.fonts_complete());
    net.devices[a].glue.tick(Millis(net.now + 11 * 60_000));
    assert!(net.devices[a].glue.fonts_complete(), "patience is over");
    assert!(
        net.devices[a].glue.fonts_complete(),
        "the next snapshot does not wait ten minutes again"
    );
}

/// Two devices pushing the same blob interleaved: the duplicate parts are ignored, no assembly is
/// dropped and no notice says a font was skipped; parts of a blob already held start nothing.
#[test]
fn interleaved_and_late_parts_of_one_blob_raise_no_false_notice() {
    let (mut net, a) = desktop();
    let bytes = font_bytes(10 * 1024 * 1024, 42);
    let hash = AssetHash::of(&bytes);
    announce(
        &mut net.devices[a].glue,
        1,
        &hash,
        "twice.ttf",
        bytes.len() as u64,
    );
    let parts = encode_blob_parts(&hash.to_bytes(), &bytes);
    assert!(parts.len() >= 3);
    for index in 0..parts.len() {
        part(&mut net.devices[a].glue, 2, &parts[index]);
        if index < parts.len() - 1 {
            part(&mut net.devices[a].glue, 3, &parts[index]);
        }
    }
    assert_eq!(net.devices[a].glue.font_blob(&hash), bytes.as_slice());
    assert_eq!(notices(&mut net, a), Vec::<String>::new());
    part(&mut net.devices[a].glue, 4, &parts[0]);
    assert!(net.devices[a].glue.fonts.assembling.is_empty());
    assert_eq!(notices(&mut net, a), Vec::<String>::new());
}
