//! Tester acceptance cases for `specs/0135-image-optimisation` slice 3 in the document, written
//! from the specification (criteria 48, 59, 66) before the implementation was read: one stored
//! result is at most 64 MiB, all of a document together at most 512 MiB (both refused before
//! anything is stored), separate from the pictures' own limit; a stored result nothing refers to
//! is not carried into a saved file by a session that only opened it.
#![allow(clippy::unwrap_used, clippy::cast_possible_truncation, missing_docs)]

use curvyo_document_core::{
    AssetError, Document, MAX_DERIVED_ASSET_BYTES, MAX_DERIVED_PROJECT_BYTES, pack, unpack,
};

const MIB: u64 = 1024 * 1024;

const ONE_PIXEL_PNG: &str = "89504e470d0a1a0a0000000d49484452000000010000000108060000001f15c4890000000d49444154789c6360f8cf00000301010018dd8db00000000049454e44ae426082";

#[test]
fn c66_the_limits_are_64_mib_per_result_and_512_mib_per_document() {
    assert_eq!(MAX_DERIVED_ASSET_BYTES, 64 * MIB);
    assert_eq!(MAX_DERIVED_PROJECT_BYTES, 512 * MIB);
    let d = Document::new(1);
    let mut bytes = vec![0_u8; (64 * MIB) as usize + 1];
    assert!(matches!(
        d.insert_derived_asset(&bytes),
        Err(AssetError::ResultTooLarge { .. })
    ));
    assert_eq!(d.derived_asset_bytes(), 0, "nothing stored");
    bytes.pop();
    // Eight distinct results of exactly 64 MiB, referred to by no step: since the architect
    // review of 2026-11-23 only referenced results count (a result kept for undo does not), so
    // they neither fill the budget nor block another; the referenced 512 MiB limit is checked in
    // `image_adjust_ai_format.rs` (`only_referenced_results_count_against_512_mib`).
    for i in 0..8_u8 {
        bytes[0] = i;
        let hash = d.insert_derived_asset(&bytes).unwrap();
        assert!(d.has_derived_asset(&hash));
    }
    assert_eq!(d.derived_asset_bytes(), 0);
    bytes[0] = 3;
    assert!(d.insert_derived_asset(&bytes).is_ok());
    assert!(d.insert_derived_asset(&[9]).is_ok());
    assert_eq!(d.derived_asset_bytes(), 0);
}

#[test]
fn c59_c66_an_unreferenced_result_is_not_saved_by_a_session_that_did_not_write_it() {
    let d = Document::new(1);
    // A 1 x 1 PNG (the store does not decode it).
    let png: Vec<u8> = (0..ONE_PIXEL_PNG.len() / 2)
        .map(|i| u8::from_str_radix(&ONE_PIXEL_PNG[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    let hash = d.insert_derived_asset(&png).unwrap();
    assert!(d.referenced_derived_assets().is_empty());
    let first = pack(&d, "t").unwrap();
    // Whatever the writing session kept, a later session that opens and saves the file drops a
    // result no step and no undo state refers to.
    let reopened = unpack(2, &first).unwrap();
    let second = pack(&reopened, "t").unwrap();
    let again = unpack(3, &second).unwrap();
    assert!(!again.has_derived_asset(&hash), "dropped at save");
    assert_eq!(again.derived_asset_bytes(), 0);
    let name = format!("assets/derived/{}.png", hash.as_str());
    assert!(
        !second.windows(name.len()).any(|w| w == name.as_bytes()),
        "no member {name}"
    );
}
