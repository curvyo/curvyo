//! Criterion 16 of `specs/0083-font-info-and-zip` and ADR 0020 §2: damaged or hostile archives
//! never panic or hang. Every prefix and random one-byte changes of each small fixture, and random
//! entry tables, are opened and every font and document entry read.
#![allow(clippy::unwrap_used)]
#![cfg(not(target_arch = "wasm32"))]

use std::io::Write;

use curvyo_text_core::{EntryClass, FontArchive};
use proptest::prelude::*;

/// The fixtures small enough to mutate (the bombs and the 10,001 entries are read whole in
/// `font_archive.rs`).
const SMALL: [&str; 13] = [
    "plain.zip",
    "nested-folders.zip",
    "macosx.zip",
    "same-ofl.zip",
    "dotdot.zip",
    "absolute.zip",
    "symlink.zip",
    "nested-bomb.zip",
    "overlap-bomb.zip",
    "wrong-crc.zip",
    "encrypted.zip",
    "non-utf8-name.zip",
    "no-font.zip",
];

fn fixture(name: &str) -> Vec<u8> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fonts/zip");
    std::fs::read(dir.join(name)).unwrap()
}

/// Opens `bytes` and reads every font and document entry; returns the entries read.
fn exercise(bytes: Vec<u8>) -> usize {
    let Ok(mut archive) = FontArchive::open(bytes) else {
        return 0;
    };
    let kept: Vec<usize> = archive
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, e)| matches!(e.class, EntryClass::Font(_) | EntryClass::Document))
        .map(|(i, _)| i)
        .collect();
    for &i in &kept {
        if let Ok(bytes) = archive.read(i) {
            assert!(bytes.len() <= curvyo_text_core::limits::MAX_FACE_BYTES);
        }
        assert!(!archive.entries()[i].file_name.contains(['/', '\\']));
    }
    kept.len()
}

/// Prefixes of each small fixture (cut archives, a cut end record): every prefix that ends in
/// the last 4 KiB, where the central directory and the end record lie, and every 61st before.
#[test]
fn ac16_prefixes() {
    for name in SMALL {
        let bytes = fixture(name);
        let tail = bytes.len().saturating_sub(4096);
        for cut in (0..tail).step_by(61).chain(tail..bytes.len()) {
            exercise(bytes[..cut].to_vec());
        }
    }
}

/// An archive of `entries` (name, stored or deflated, bytes).
fn archive_of(entries: &[(String, bool, Vec<u8>)]) -> Vec<u8> {
    let mut out = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, stored, data) in entries {
        let method = if *stored {
            zip::CompressionMethod::Stored
        } else {
            zip::CompressionMethod::Deflated
        };
        let options = zip::write::SimpleFileOptions::default().compression_method(method);
        if out.start_file(name.as_str(), options).is_ok() {
            out.write_all(data).unwrap();
        }
    }
    out.finish().unwrap().into_inner()
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]

    /// One byte of a fixture changed: no panic, names stay base names.
    #[test]
    fn ac16_one_byte_changes(which in 0..SMALL.len(), at in any::<prop::sample::Index>(), value in any::<u8>()) {
        let mut bytes = fixture(SMALL[which]);
        let i = at.index(bytes.len());
        bytes[i] = value;
        exercise(bytes);
    }

    /// Random entry tables with hostile names and contents, then one byte changed.
    #[test]
    fn ac16_random_entry_tables(
        entries in prop::collection::vec(
            ("[a-zA-Z./\\\\:_ ]{0,12}(\\.ttf|\\.txt|\\.svg|\\.zip|/)?", any::<bool>(), prop::collection::vec(any::<u8>(), 0..64)),
            0..12,
        ),
        at in any::<prop::sample::Index>(),
        value in any::<u8>(),
    ) {
        let mut bytes = archive_of(&entries);
        exercise(bytes.clone());
        let i = at.index(bytes.len());
        bytes[i] = value;
        exercise(bytes);
    }
}
