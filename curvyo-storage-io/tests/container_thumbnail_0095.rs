//! The thumbnail a save puts into a `.curvyo` container (`specs/0095-workspace` criterion 6): the
//! page's PNG becomes the `thumbnail.png` member, every other member stays byte for byte, and the
//! container still opens.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::io::{Cursor, Read};

use curvyo_document_core::{Document, pack, unpack};
use curvyo_storage_io::{
    DocumentProbe, MAX_THUMBNAIL_BYTES, ThumbnailError, probe_document, with_thumbnail,
};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nfirst";
const OTHER_PNG: &[u8] = b"\x89PNG\r\n\x1a\nsecond";

/// Every member of a zip archive, name and bytes, in archive order.
fn members(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut member = archive.by_index(index).unwrap();
            let mut out = Vec::new();
            member.read_to_end(&mut out).unwrap();
            (member.name().unwrap().into_owned(), out)
        })
        .collect()
}

fn document_bytes() -> Vec<u8> {
    pack(&Document::new(1), "0.1.0").unwrap()
}

/// The thumbnail is added beside the other members, which stay as they were; the container opens
/// and a tree row reads the thumbnail.
#[test]
fn a_thumbnail_is_added_and_every_other_member_is_kept() {
    let plain = document_bytes();
    let with = with_thumbnail(&plain, PNG).unwrap();

    let mut expected = members(&plain);
    expected.push(("thumbnail.png".to_owned(), PNG.to_vec()));
    assert_eq!(members(&with), expected);
    assert!(unpack(2, &with).is_ok());

    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("Lid.curvyo"), &with).unwrap();
    assert_eq!(
        probe_document(dir.path(), &["Lid.curvyo".to_owned()]).unwrap(),
        DocumentProbe::Readable {
            thumbnail: Some(PNG.to_vec())
        }
    );
}

/// A second save replaces the thumbnail: the container holds one, the newest.
#[test]
fn a_new_thumbnail_replaces_the_old_one() {
    let once = with_thumbnail(&document_bytes(), PNG).unwrap();
    let twice = with_thumbnail(&once, OTHER_PNG).unwrap();
    let thumbnails: Vec<_> = members(&twice)
        .into_iter()
        .filter(|(name, _)| name == "thumbnail.png")
        .map(|(_, bytes)| bytes)
        .collect();
    assert_eq!(thumbnails, vec![OTHER_PNG.to_vec()]);
    assert!(unpack(2, &twice).is_ok());
}

/// Bytes that are no PNG, a PNG larger than a row reads, and bytes that are no container are
/// refused; the caller then saves without a thumbnail.
#[test]
fn a_thumbnail_that_no_row_would_show_is_refused() {
    let plain = document_bytes();
    assert_eq!(
        with_thumbnail(&plain, b"GIF89a"),
        Err(ThumbnailError::NotPng)
    );
    let mut large = PNG.to_vec();
    large.resize(usize::try_from(MAX_THUMBNAIL_BYTES).unwrap() + 1, 0);
    assert_eq!(
        with_thumbnail(&plain, &large),
        Err(ThumbnailError::TooLarge)
    );
    assert_eq!(
        with_thumbnail(b"not a zip", PNG),
        Err(ThumbnailError::NotAContainer)
    );
}
