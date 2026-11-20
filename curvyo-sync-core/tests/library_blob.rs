//! The font blobs of the device group room (`specs/0116-library-sync` criteria 16, 18 and 20,
//! ADR 0062 decision 3): parts of at most 4 MiB, the blob entry of the framed snapshot, and the
//! golden font entry and sealed blob part.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_crypto_core::{DocumentId, EnvelopeKind, EpochKey, NonceStream, Sealing, open, seal};
use curvyo_document_core::{AssetHash, CuttableChoice, FontDetails, LibraryList};
use curvyo_sync_core::{
    BLOB_STREAM_ID, BlobAssembler, MAX_BLOB_BYTES, PART_BYTES, SnapshotAssembler, StreamError,
    decode_blob_entry, decode_blob_part, encode_blob_parts, encode_group_snapshot_with_blobs,
    encode_library_record, split_snapshot,
};

const PEER: u64 = 0x0102_0304_0506_0708;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/library")
        .join(name)
}

fn golden(name: &str, bytes: &[u8]) {
    let path = fixture(name);
    if std::env::var_os("CURVYO_UPDATE_GOLDEN").is_some() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, bytes).unwrap();
    }
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "{name} differs");
}

/// A deterministic font stand-in of `len` bytes.
fn font_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|n| u8::try_from(n % 251).unwrap()).collect()
}

/// The golden bytes of a font entry record and a blob part v1 (ADR 0062 consequences);
/// regenerate with `CURVYO_UPDATE_GOLDEN=1`.
#[test]
fn golden_font_entry_record_and_sealed_blob_part() {
    let bytes = font_bytes(1_000);
    let hash = AssetHash::of(&bytes);
    let list = LibraryList::new(PEER);
    let before = list.version();
    list.font_set(
        &hash,
        &FontDetails {
            file_name: "ZQX-FONT-7731.ttf".to_owned(),
            folder: None,
            size: 1_000,
            tags: vec!["ZQX-TAG-7731".to_owned()],
            favourite: true,
            cuttable: CuttableChoice::Auto,
            documents: Vec::new(),
        },
    )
    .unwrap();
    let update = list.export_updates(&before).unwrap();
    golden(
        "library_record_font_entry.bin",
        &encode_library_record(PEER, &update),
    );

    let parts = encode_blob_parts(&hash.to_bytes(), &bytes);
    assert_eq!(parts.len(), 1);
    golden("font_blob_part_v1.bin", &parts[0]);
    let key = EpochKey::from_bytes([7; 32]);
    let sealing = Sealing {
        kind: EnvelopeKind::Asset,
        epoch: 0,
        key: &key,
        document_id: DocumentId([0x6e; 16]),
        sender_peer: PEER,
        signer: None,
    };
    let nonce = NonceStream::new([3; 32]).next_nonce().unwrap();
    let sealed = seal(&sealing, nonce, &parts[0]).unwrap();
    golden("font_blob_part_sealed_v1.bin", &sealed);
    let opened = open(&sealed, &key, DocumentId([0x6e; 16]), EnvelopeKind::Asset).unwrap();
    assert_eq!(opened.plaintext, parts[0]);
}

/// Criterion 16: a blob of 9 MiB is three parts of at most 4 MiB, in order, and assembles back
/// to the same bytes; a small blob is one part.
#[test]
fn a_blob_is_cut_into_parts_of_at_most_four_mib_and_assembles_back() {
    let bytes = font_bytes(9 * 1024 * 1024);
    let hash = AssetHash::of(&bytes).to_bytes();
    let parts = encode_blob_parts(&hash, &bytes);
    assert_eq!(parts.len(), 3);
    assert!(parts.iter().all(|part| part.len() <= PART_BYTES));
    let mut assembler = BlobAssembler::start(&parts[0]).unwrap();
    assert_eq!(assembler.hash(), &hash);
    assert!(!assembler.is_complete());
    assert_eq!(assembler.push(&parts[1]).unwrap(), None);
    assert_eq!(assembler.push(&parts[2]).unwrap().unwrap(), bytes);
    assert_eq!(encode_blob_parts(&[1; 32], &[]).len(), 1);
}

/// `start` takes part 0 into the assembly: a blob of one part is complete at once and
/// leaves the assembler with `take`; a part fed twice is out of order.
#[test]
fn start_takes_the_first_part_into_the_assembly() {
    let bytes = font_bytes(1_000);
    let hash = AssetHash::of(&bytes).to_bytes();
    let parts = encode_blob_parts(&hash, &bytes);
    let mut one = BlobAssembler::start(&parts[0]).unwrap();
    assert!(one.is_complete());
    assert_eq!(one.take().unwrap(), bytes);
    assert_eq!(one.push(&parts[0]).err(), Some(StreamError::OutOfOrder));

    let big = font_bytes(5 * 1024 * 1024);
    let parts = encode_blob_parts(&AssetHash::of(&big).to_bytes(), &big);
    let mut many = BlobAssembler::start(&parts[0]).unwrap();
    assert_eq!(many.take(), None);
    assert_eq!(many.total(), big.len() as u64);
    assert_eq!(many.push(&parts[0]).err(), Some(StreamError::OutOfOrder));
}

/// A blob's last part is padded to 64 KiB so the relay cannot tell a known font by its exact
/// size; the padding is cut off on reading and must be zero.
#[test]
fn the_last_part_is_padded_and_the_padding_is_cut_off() {
    for len in [0_usize, 1, 1_000, 65_536, 65_537] {
        let bytes = font_bytes(len);
        let parts = encode_blob_parts(&[5; 32], &bytes);
        let last = parts.last().unwrap();
        assert_eq!(last.len() % 65_536, 0, "{len} bytes");
        let read = decode_blob_part(last).unwrap();
        assert_eq!(read.bytes.len(), len.min(PART_BYTES - 48));
    }
    let a = encode_blob_parts(&[5; 32], &font_bytes(1_000));
    let b = encode_blob_parts(&[5; 32], &font_bytes(40_000));
    assert_eq!(a[0].len(), b[0].len(), "two sizes, one padded length");
    let mut dirty = a[0].clone();
    *dirty.last_mut().unwrap() = 1;
    assert_eq!(decode_blob_part(&dirty).err(), Some(StreamError::Malformed));
    let mut long = a[0].clone();
    long.resize(long.len() + 65_536, 0);
    assert_eq!(decode_blob_part(&long).err(), Some(StreamError::Malformed));
    // A part count that does not fit the announced length is refused.
    let mut lying = a[0].clone();
    lying[36..40].copy_from_slice(&3_u32.to_le_bytes());
    assert_eq!(decode_blob_part(&lying).err(), Some(StreamError::Malformed));
}

/// A part out of order, of another blob or short is refused; a blob announced over 16 MiB is
/// refused with its first part before any byte is held (criterion 19, receipt).
#[test]
fn a_hostile_blob_is_refused_before_it_is_held() {
    let bytes = font_bytes(5 * 1024 * 1024);
    let hash = AssetHash::of(&bytes).to_bytes();
    let parts = encode_blob_parts(&hash, &bytes);
    assert_eq!(
        BlobAssembler::start(&parts[1]).err(),
        Some(StreamError::OutOfOrder)
    );
    let mut assembler = BlobAssembler::start(&parts[0]).unwrap();
    assert_eq!(
        assembler.push(&parts[0]).err(),
        Some(StreamError::OutOfOrder)
    );
    let other = encode_blob_parts(&[9; 32], &font_bytes(5 * 1024 * 1024));
    assert_eq!(
        assembler.push(&other[1]).err(),
        Some(StreamError::OutOfOrder)
    );
    assert_eq!(
        decode_blob_part(&parts[0][..10]).err(),
        Some(StreamError::Malformed)
    );

    let mut huge = parts[0].clone();
    huge[40..48].copy_from_slice(&(MAX_BLOB_BYTES + 1).to_le_bytes());
    assert_eq!(
        BlobAssembler::start(&huge).err(),
        Some(StreamError::Malformed)
    );
}

/// Criteria 16 and 23: the framed room snapshot carries each blob as an entry beside the two
/// lists, so a later device gets the bytes though the relay pruned the records.
#[test]
fn the_group_snapshot_carries_the_blobs() {
    let font = font_bytes(5 * 1024 * 1024);
    let hash = AssetHash::of(&font).to_bytes();
    let stream = encode_group_snapshot_with_blobs(b"devices", b"library", &[(hash, &font)]);
    let mut assembler = SnapshotAssembler::default();
    let mut entries = Vec::new();
    for part in split_snapshot(&stream, [1; 16]) {
        entries.extend(assembler.push_part(&part).unwrap());
    }
    assert!(assembler.is_complete());
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[2].stream, BLOB_STREAM_ID);
    let (read_hash, read_bytes) = decode_blob_entry(&entries[2]).unwrap();
    assert_eq!(read_hash, hash);
    assert_eq!(read_bytes, font.as_slice());
    assert!(decode_blob_entry(&entries[0]).is_none());
}
