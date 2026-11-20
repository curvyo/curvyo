//! The plaintext formats of the device group room's two sealed streams (ADR 0062 decision 1):
//! the stream ids, the routing of a record's plaintext and the room's framed snapshot.
//!
//! The devices list keeps its bare updates as every pre-0116 build writes them; a library
//! update carries the ADR 0037 inner header with [`LIBRARY_STREAM_ID`], so a session routes a
//! record by decoding its header. Everything here is sealed before it leaves a client.

use crate::library_blob::{BLOB_HASH_BYTES, BLOB_STREAM_ID, MAX_BLOB_BYTES, encode_blob_entry};
use crate::workspace_stream::{
    SnapshotAssembler, StreamEntry, StreamError, decode_inner_record, encode_inner_record,
    encode_snapshot_stream,
};

/// The most bytes the devices list's or the library's entry of a group snapshot may have: 32 MiB
/// **(Proposal)**, far over any list the toggles' text limits allow.
const MAX_LIST_ENTRY_BYTES: u64 = 32 * 1024 * 1024;

/// The stream id of the devices list: the first entry of the group room's framed snapshot.
pub const DEVICES_STREAM_ID: [u8; 16] = *b"curvyo.devices-1";
/// The stream id of the library list (ADR 0062 decision 1): the header of every library update.
pub const LIBRARY_STREAM_ID: [u8; 16] = *b"curvyo.library-1";

/// Which of the group room's streams a record's plaintext belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupRecord<'a> {
    /// A bare update of the devices list, as every pre-0116 build writes it (unchanged).
    Devices(&'a [u8]),
    /// An update of the library list: the inner header (ADR 0037 decision 5) with
    /// [`LIBRARY_STREAM_ID`], then the Loro update.
    Library {
        /// The Loro peer that wrote the update.
        peer: u64,
        /// The library list's update bytes.
        payload: &'a [u8],
    },
}

/// A reader of the group room's framed snapshot that drops an entry over its limit: a blob entry
/// over [`MAX_BLOB_BYTES`] plus its hash, a list entry over 32 MiB.
#[must_use]
pub fn group_snapshot_assembler() -> SnapshotAssembler {
    SnapshotAssembler::with_entry_limit(|stream| {
        if *stream == BLOB_STREAM_ID {
            MAX_BLOB_BYTES + BLOB_HASH_BYTES as u64
        } else {
            MAX_LIST_ENTRY_BYTES
        }
    })
}

/// Routes the plaintext of one group room record: an inner record of the library stream apart
/// from the bare devices list update of an older build.
///
/// # Errors
/// [`StreamError::Malformed`] when the bytes claim the library stream's id without carrying a
/// whole inner header; the caller skips such a record with a notice, never rewriting it.
pub fn route_group_record(bytes: &[u8]) -> Result<GroupRecord<'_>, StreamError> {
    let is_library = bytes
        .first_chunk::<16>()
        .is_some_and(|stream| *stream == LIBRARY_STREAM_ID);
    if !is_library {
        return Ok(GroupRecord::Devices(bytes));
    }
    let (header, payload) = decode_inner_record(bytes)?;
    Ok(GroupRecord::Library {
        peer: header.peer,
        payload,
    })
}

/// The library list's update `payload` as a group room record (the inverse of
/// [`route_group_record`] for the library stream).
#[must_use]
pub fn encode_library_record(peer: u64, payload: &[u8]) -> Vec<u8> {
    encode_inner_record(LIBRARY_STREAM_ID, peer, payload)
}

/// The group room's snapshot stream: the devices list's snapshot `devices` first, then the
/// library list's `library` (ADR 0062 decision 1), framed as one snapshot stream of ADR 0037
/// decision 7 and cut by its `split_snapshot`.
#[must_use]
pub fn encode_group_snapshot(devices: &[u8], library: &[u8]) -> Vec<u8> {
    encode_group_snapshot_with_blobs(devices, library, &[])
}

/// As [`encode_group_snapshot`], followed by one entry per blob of a switched-on font (ADR 0062
/// decision 3: the relay prunes the records a snapshot covers, so the blobs live in it).
#[must_use]
pub fn encode_group_snapshot_with_blobs(
    devices: &[u8],
    library: &[u8],
    blobs: &[([u8; BLOB_HASH_BYTES], &[u8])],
) -> Vec<u8> {
    let mut entries = vec![
        StreamEntry {
            stream: DEVICES_STREAM_ID,
            bytes: devices.to_vec(),
        },
        StreamEntry {
            stream: LIBRARY_STREAM_ID,
            bytes: library.to_vec(),
        },
    ];
    entries.extend(
        blobs
            .iter()
            .map(|(hash, bytes)| encode_blob_entry(hash, bytes)),
    );
    encode_snapshot_stream(&entries)
}
