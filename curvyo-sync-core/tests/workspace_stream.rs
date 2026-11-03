//! The plaintext formats of a workspace room (`specs/0095-workspace` criteria 28, 34 and 43, ADR
//! 0037 decisions 5, 7 and 10): the inner record header, the snapshot stream and its parts, the
//! `Moved` payload and when a snapshot is due.

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_sync_core::{
    Moved, PART_BYTES, PART_PADDING, SnapshotAssembler, StreamEntry, StreamError,
    decode_inner_record, decode_moved, encode_inner_record, encode_moved, encode_snapshot_stream,
    snapshot_due, split_snapshot,
};

const LIST: [u8; 16] = [0x11; 16];
const LID: [u8; 16] = [0xd1; 16];
const BASE: [u8; 16] = [0xd2; 16];

fn entries() -> Vec<StreamEntry> {
    vec![
        StreamEntry {
            stream: LIST,
            bytes: b"list snapshot".to_vec(),
        },
        StreamEntry {
            stream: LID,
            bytes: vec![0xaa; 300],
        },
        StreamEntry {
            stream: BASE,
            bytes: Vec::new(),
        },
    ]
}

/// The golden bytes of every format; regenerate with `CURVYO_UPDATE_GOLDEN=1`.
#[test]
fn golden_workspace_stream_v1() {
    let mut bytes = Vec::new();
    bytes.extend(encode_inner_record(LID, 0x0102_0304_0506_0708, b"update"));
    bytes.extend(encode_snapshot_stream(&entries()));
    bytes.extend(encode_moved(&Moved {
        workspace: LIST,
        document: LID,
    }));
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workspace_stream_v1.bin");
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &bytes).unwrap();
    }
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "workspace_stream_v1.bin differs"
    );
}

/// ADR 0037 decision 5: every update record of a workspace begins with its stream and peer.
#[test]
fn the_inner_header_names_the_stream_and_the_peer() {
    let record = encode_inner_record(LID, 42, b"loro update");
    assert_eq!(record.len(), 16 + 8 + 11);
    let (header, payload) = decode_inner_record(&record).unwrap();
    assert_eq!(header.stream, LID);
    assert_eq!(header.peer, 42);
    assert_eq!(payload, b"loro update");
    assert_eq!(
        decode_inner_record(&record[..23]),
        Err(StreamError::Malformed)
    );
}

/// ADR 0037 decision 7: the stream cut into parts of 4 MiB, the last padded to 64 KiB; parts
/// fed in order give each entry as soon as its bytes are complete (criterion 28).
#[test]
fn a_snapshot_is_cut_into_parts_and_read_back_entry_by_entry() {
    let mut big = entries();
    big.push(StreamEntry {
        stream: [0xd3; 16],
        bytes: vec![0x5a; 9 * 1024 * 1024],
    });
    let stream = encode_snapshot_stream(&big);
    let parts = split_snapshot(&stream, [0x77; 16]);
    assert_eq!(parts.len(), 3);
    for part in &parts[..2] {
        assert_eq!(part.len(), PART_BYTES);
    }
    assert_eq!(parts[2].len() % PART_PADDING, 0);

    let mut assembler = SnapshotAssembler::default();
    let first = assembler.push_part(&parts[0]).unwrap();
    assert_eq!(
        first,
        entries(),
        "the small entries are complete after part 0"
    );
    assert_eq!(assembler.push_part(&parts[1]).unwrap(), Vec::new());
    let last = assembler.push_part(&parts[2]).unwrap();
    assert_eq!(last, vec![big[3].clone()]);
    assert!(assembler.is_complete());
}

/// A part of another snapshot, out of order, or after the last is refused.
#[test]
fn parts_out_of_order_or_of_another_snapshot_are_refused() {
    let stream = encode_snapshot_stream(&[StreamEntry {
        stream: LID,
        bytes: vec![1; 5 * 1024 * 1024],
    }]);
    let parts = split_snapshot(&stream, [0x77; 16]);
    let other = split_snapshot(&stream, [0x78; 16]);
    let mut assembler = SnapshotAssembler::default();
    assert_eq!(assembler.push_part(&parts[1]), Err(StreamError::OutOfOrder));
    assembler.push_part(&parts[0]).unwrap();
    assert_eq!(assembler.push_part(&other[1]), Err(StreamError::OutOfOrder));
    assembler.push_part(&parts[1]).unwrap();
    assert!(assembler.is_complete());
    assert_eq!(assembler.push_part(&parts[1]), Err(StreamError::OutOfOrder));
}

/// Criterion 34: two workspaces of the same total size, one of 3 and one of 30 documents, give
/// parts of the same count and sizes.
#[test]
fn the_parts_do_not_show_the_number_of_documents() {
    let few: Vec<StreamEntry> = (0..3_u8)
        .map(|i| StreamEntry {
            stream: [i; 16],
            bytes: vec![i; 1_000_000],
        })
        .collect();
    let many: Vec<StreamEntry> = (0..30_u8)
        .map(|i| StreamEntry {
            stream: [i; 16],
            bytes: vec![i; 100_000 - 24 * 27 / 30],
        })
        .collect();
    let sizes = |entries: &[StreamEntry]| -> Vec<usize> {
        split_snapshot(&encode_snapshot_stream(entries), [0; 16])
            .iter()
            .map(Vec::len)
            .collect()
    };
    assert_eq!(sizes(&few), sizes(&many));
}

/// A stream that is cut short, or not version 1, is refused.
#[test]
fn a_damaged_stream_is_refused() {
    let stream = encode_snapshot_stream(&entries());
    let parts = split_snapshot(&stream[..stream.len() - 1], [1; 16]);
    let mut assembler = SnapshotAssembler::default();
    assert_eq!(assembler.push_part(&parts[0]), Err(StreamError::Malformed));
    let mut newer = stream.clone();
    newer[0] = 2;
    let mut assembler = SnapshotAssembler::default();
    assert_eq!(
        assembler.push_part(&split_snapshot(&newer, [1; 16])[0]),
        Err(StreamError::FormatTooNew)
    );
}

/// ADR 0037 decision 10: the `Moved` record of a taken-in document.
#[test]
fn a_moved_payload_round_trips_and_is_told_apart_from_updates() {
    let moved = Moved {
        workspace: LIST,
        document: LID,
    };
    assert_eq!(decode_moved(&encode_moved(&moved)), Some(moved));
    assert_eq!(decode_moved(b"\x00\x00\x00\x00loro"), None);
    assert_eq!(decode_moved(&encode_moved(&moved)[..20]), None);
}

/// ADR 0037 decision 7: a snapshot is due when the tail reaches max(4 MiB, half the last
/// snapshot) or max(500, last snapshot / 8 KiB) records.
#[test]
fn a_snapshot_is_due_by_the_size_ratio() {
    const MIB: u64 = 1024 * 1024;
    assert!(!snapshot_due(4 * MIB - 1, 499, 0));
    assert!(snapshot_due(4 * MIB, 0, 0));
    assert!(snapshot_due(0, 500, 0));
    assert!(!snapshot_due(9 * MIB, 0, 20 * MIB));
    assert!(snapshot_due(10 * MIB, 0, 20 * MIB));
    assert!(!snapshot_due(0, 2559, 20 * MIB));
    assert!(snapshot_due(0, 2560, 20 * MIB));
}
