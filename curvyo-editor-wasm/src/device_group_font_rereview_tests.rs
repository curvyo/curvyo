//! Tests of what the architect's re-review of the font sync asked for (`specs/0116-library-sync`
//! slice 2, ADR 0062 decision 4): a deletion learnt through a merged snapshot.

use curvyo_document_core::LibraryList;
use curvyo_sync_core::{LIBRARY_STREAM_ID, StreamEntry, encode_library_record};

use super::FontChange;
use super::font_tests::{desktop, font_bytes, input, notices};

/// A deletion learnt through a snapshot that merges (the room's list shares this list's
/// history) still removes the local copy; only a snapshot that replaces the list does not.
#[test]
fn a_deletion_learnt_through_a_merged_snapshot_still_removes_the_font() {
    let (mut net, a) = desktop();
    let hash = net.devices[a]
        .glue
        .font_set(input("gone.ttf"), font_bytes(30_000, 30))
        .unwrap();
    net.devices[a].page();
    net.devices[a].font_arrivals.clear();
    let _ = notices(&mut net, a);
    let room =
        LibraryList::from_snapshot(77, &net.devices[a].glue.library().snapshot().unwrap()).unwrap();
    room.font_remove(&hash).unwrap();
    net.devices[a].glue.snapshot_stream(&StreamEntry {
        stream: LIBRARY_STREAM_ID,
        bytes: encode_library_record(77, &room.snapshot().unwrap()),
    });
    net.devices[a].page();
    assert!(net.devices[a].glue.library().font_entry(&hash).is_none());
    assert!(
        net.devices[a]
            .font_arrivals
            .iter()
            .any(|arrival| arrival.hash == hash && arrival.change == FontChange::Removed),
        "{:?}",
        net.devices[a].font_arrivals
    );
    assert!(
        notices(&mut net, a)
            .iter()
            .any(|n| n.contains("was deleted"))
    );
}
