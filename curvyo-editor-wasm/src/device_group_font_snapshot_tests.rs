//! Tests of the snapshot path's budget (`specs/0116-library-sync` slice 2, ADR 0062 decision 6,
//! architect re-review 3): an entry over its limit or the library budget drops the assembly.

use curvyo_document_core::AssetHash;
use curvyo_sync_core::{
    BLOB_STREAM_ID, RecordKind, SessionEvent, encode_snapshot_stream, split_snapshot,
};

use super::font_tests::{desktop, font_bytes, notices};

/// The snapshot path has a budget: an entry over the blob limit drops the assembly, so an endless
/// snapshot never grows in memory; a later complete snapshot is taken as usual.
#[test]
fn a_snapshot_entry_over_the_blob_limit_drops_the_assembly() {
    let (mut net, a) = desktop();
    // An entry that claims 2^40 bytes of the blob stream, then more parts than any blob.
    let mut stream = vec![1u8];
    stream.extend_from_slice(&1u32.to_le_bytes());
    stream.extend_from_slice(&BLOB_STREAM_ID);
    stream.extend_from_slice(&(1u64 << 40).to_le_bytes());
    stream.extend(std::iter::repeat_n(7u8, 5 * 1024 * 1024));
    stream.extend(std::iter::repeat_n(7u8, 4 * 1024 * 1024));
    let parts = split_snapshot(&stream, [3; 16]);
    assert!(parts.len() >= 3);
    for part in &parts {
        net.devices[a].glue.on_event(SessionEvent::Remote {
            seq: 9,
            kind: RecordKind::Snapshot,
            bytes: part.clone(),
        });
    }
    assert!(
        net.devices[a].glue.snapshot_assembly_bytes() <= 1024,
        "the endless entry was dropped, not kept: {}",
        net.devices[a].glue.snapshot_assembly_bytes()
    );
    assert!(
        notices(&mut net, a)
            .iter()
            .any(|n| n == "A synced font could not be taken. It was skipped.")
    );
}

/// The snapshot path counts the entry being assembled against the library budget: a blob that
/// cannot fit is dropped after its first part, and the rest of the snapshot is ignored.
#[test]
fn a_snapshot_blob_past_the_budget_drops_the_assembly() {
    let (mut net, a) = desktop();
    net.devices[a].glue.fonts.limit = 100_000;
    let bytes = font_bytes(6 * 1024 * 1024, 31);
    let hash = AssetHash::of(&bytes);
    let entry = curvyo_sync_core::encode_blob_entry(&hash.to_bytes(), &bytes);
    let parts = split_snapshot(&encode_snapshot_stream(&[entry]), [4; 16]);
    assert!(parts.len() >= 2);
    for part in &parts {
        net.devices[a].glue.on_event(SessionEvent::Remote {
            seq: 10,
            kind: RecordKind::Snapshot,
            bytes: part.clone(),
        });
    }
    assert_eq!(net.devices[a].glue.snapshot_assembly_bytes(), 0);
    assert_eq!(net.devices[a].glue.font_blob(&hash).len(), 0);
    assert!(
        notices(&mut net, a)
            .iter()
            .any(|n| n == "A synced font could not be taken. It was skipped.")
    );
}
