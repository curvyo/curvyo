//! Tester acceptance cases for `specs/0112-test-grid-and-materials` in the laser model, written
//! from the specification and ADR 0058 before the implementation was read: the 5-character test
//! id and its uniqueness against taken ids (criterion 9), record format 2 round trip and the
//! 512 KiB refusal (30), the retired ids file (25), the photo header checks (27) and the derived
//! operations of a grid (12).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    missing_docs
)]

use std::collections::HashSet;
use std::path::PathBuf;

use curvyo_laser_core::grids::{TEST_ID_CHARS, TestId};
use curvyo_laser_core::library::{
    MAX_PHOTO_BYTES, MAX_RECORD_TEXT, PhotoKind, PhotoRefusal, RetiredIds, check_photo,
    read_record, read_retired, write_photo, write_record, write_retired,
};

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/materials")
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(fixtures().join(name)).unwrap()
}

const FULL_ID: &str = "0b6c3f1e-2a4d-4b8e-9c7f-1d2e3f4a5b6c";

fn file_name(id: &str) -> String {
    format!("{id}.json")
}

/// A PNG header of `width` x `height`.
fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend_from_slice(&width.to_be_bytes());
    bytes.extend_from_slice(&height.to_be_bytes());
    bytes.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]);
    bytes
}

/// A JPEG of SOI, an APP0 segment and an SOF marker `sof` of `width` x `height`, then EOI.
fn jpeg(sof: u8, width: u16, height: u16) -> Vec<u8> {
    let mut b = vec![0xFF, 0xD8];
    // APP0 JFIF, length 16.
    b.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x10]);
    b.extend_from_slice(b"JFIF\0");
    b.extend_from_slice(&[1, 1, 0, 0, 1, 0, 1, 0, 0]);
    // SOFn: length 17, precision 8, height, width, 3 components.
    b.extend_from_slice(&[0xFF, sof, 0x00, 0x11, 0x08]);
    b.extend_from_slice(&height.to_be_bytes());
    b.extend_from_slice(&width.to_be_bytes());
    b.extend_from_slice(&[3, 1, 0x22, 0, 2, 0x11, 1, 3, 0x11, 1]);
    b.extend_from_slice(&[0xFF, 0xD9]);
    b
}

// ---- criterion 9: test ids ---------------------------------------------------------------------

#[test]
fn c9_test_id_is_five_of_0_9_a_z() {
    assert_eq!(TEST_ID_CHARS, 5, "five characters");
    for good in ["00000", "ZZZZZ", "K7Q2M", "8PL4D", "A0B1C"] {
        assert_eq!(TestId::parse(good).unwrap().as_str(), good);
    }
    for bad in [
        "", "K7Q2", "K7Q2MX", "k7q2m", "K7Q2m", "K7Q-M", "K7 2M", "ÄBCDE", "K7Q2M ", " K7Q2",
    ] {
        assert!(TestId::parse(bad).is_none(), "{bad:?} must not parse");
    }
}

#[test]
fn c9_made_ids_parse_and_cover_the_alphabet() {
    let mut seen = HashSet::new();
    for seed in 0u32..400 {
        let bytes: Vec<u8> = (0..64u32)
            .map(|i| (seed.wrapping_mul(2_654_435_761).wrapping_add(i * 97) >> 7) as u8)
            .collect();
        let Some(id) = TestId::make(&bytes, &|_| false) else {
            continue;
        };
        assert_eq!(id.as_str().len(), 5);
        assert!(TestId::parse(id.as_str()).is_some(), "{}", id.as_str());
        for c in id.as_str().chars() {
            assert!(c.is_ascii_digit() || c.is_ascii_uppercase(), "{c}");
            seen.insert(c);
        }
    }
    assert_eq!(seen.len(), 36, "every symbol of 0-9A-Z appears");
}

#[test]
fn c9_make_never_returns_a_taken_id() {
    let bytes: Vec<u8> = (0..64).map(|i| (i * 37 + 11) as u8).collect();
    let first = TestId::make(&bytes, &|_| false).unwrap();
    let again = TestId::make(&bytes, &|id| id == &first);
    if let Some(other) = again {
        assert_ne!(other, first);
    }
    // Everything taken: no id at all rather than a duplicate.
    assert!(TestId::make(&bytes, &|_| true).is_none());
    // Too few bytes for one id: none.
    assert!(TestId::make(&[1, 2, 3], &|_| false).is_none());
}

#[test]
fn c9_retired_ids_count_as_taken() {
    let (retired, notice) = read_retired(Some(&golden("retired_ids.json")));
    assert!(notice.is_none());
    for id in ["00042", "4H8ZP", "Z9Z9Z"] {
        assert!(retired.contains(&TestId::parse(id).unwrap()), "{id}");
    }
    assert!(!retired.contains(&TestId::parse("K7Q2M").unwrap()));
}

// ---- criterion 25: the retired file ------------------------------------------------------------

#[test]
fn c25_retired_ids_round_trip_sorted_and_append_only() {
    let (mut retired, _) = read_retired(None);
    assert!(retired.is_empty());
    let ids = ["ZZZZ1", "00001", "K7Q2M"].map(|t| TestId::parse(t).unwrap());
    assert!(retired.retire(ids).unwrap());
    // Retiring an id twice changes nothing and never duplicates it.
    assert!(!retired.retire([TestId::parse("K7Q2M").unwrap()]).unwrap());
    assert_eq!(retired.len(), 3);
    let text = write_retired(&retired);
    let (back, notice) = read_retired(Some(&text));
    assert!(notice.is_none());
    let order: Vec<&str> = back.iter().map(TestId::as_str).collect();
    assert_eq!(order, ["00001", "K7Q2M", "ZZZZ1"]);
    assert_eq!(write_retired(&back), text, "write is canonical");
}

#[test]
fn c25_damaged_retired_file_reads_empty_with_a_notice() {
    for bad in ["{", "[]", "{\"format_version\": 1, \"retired\": [\"abc\"]}"] {
        let (r, notice): (RetiredIds, _) = read_retired(Some(bad));
        assert!(r.is_empty(), "{bad}");
        assert!(notice.is_some(), "{bad}");
    }
}

// ---- criterion 30: record format 2 -------------------------------------------------------------

#[test]
fn c30_format2_golden_round_trips_byte_for_byte() {
    let text = golden("format2_full.json");
    let m = read_record(&file_name(FULL_ID), &text).unwrap();
    assert_eq!(m.id, FULL_ID);
    assert!(m.lasers.len() >= 2, "one record for several lasers");
    let back = write_record(&m);
    assert_eq!(back, text);
    let again = read_record(&file_name(FULL_ID), &back).unwrap();
    assert_eq!(write_record(&again), back);
}

#[test]
fn c30_format2_maximum_reads_and_fits() {
    let text = golden("format2_max.json");
    assert!(text.len() <= MAX_RECORD_TEXT);
    let id: serde_json::Value = serde_json::from_str(&text).unwrap();
    let id = id["id"].as_str().unwrap().to_owned();
    let m = read_record(&file_name(&id), &text).unwrap();
    assert_eq!(m.tests.len(), 200);
    assert!(m.tests.iter().all(|t| t.note.chars().count() <= 500));
}

#[test]
fn c30_record_over_512_kib_is_refused() {
    let text = golden("format2_full.json");
    // Same record, padded with JSON whitespace past 512 KiB: valid JSON, too long a file.
    let trimmed = text.trim_end();
    let body = &trimmed[..trimmed.len() - 1];
    let padded = format!("{body}{}}}\n", " ".repeat(MAX_RECORD_TEXT));
    assert!(padded.len() > MAX_RECORD_TEXT);
    assert!(serde_json::from_str::<serde_json::Value>(&padded).is_ok());
    assert!(read_record(&file_name(FULL_ID), &padded).is_err());
}

#[test]
fn c30_newer_format_is_refused_not_rewritten() {
    let text = golden("newer_format.json");
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let id = v["id"].as_str().unwrap_or(FULL_ID).to_owned();
    assert!(read_record(&file_name(&id), &text).is_err());
}

#[test]
fn c26_note_of_501_characters_is_refused_by_the_reader() {
    let text = golden("note_501.json");
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let id = v["id"].as_str().unwrap().to_owned();
    assert!(read_record(&file_name(&id), &text).is_err());
}

#[test]
fn c9_duplicate_test_id_in_one_record_is_refused() {
    let text = golden("format2_full.json");
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    let tests = v["tests"].as_array_mut().unwrap();
    assert!(tests.len() >= 2);
    let first = tests[0]["id"].clone();
    tests[1]["id"] = first;
    let text = serde_json::to_string_pretty(&v).unwrap();
    assert!(read_record(&file_name(FULL_ID), &text).is_err());
}

#[test]
fn c19_set_holds_only_the_four_numbers() {
    let text = golden("format2_full.json");
    let mut v: serde_json::Value = serde_json::from_str(&text).unwrap();
    // An operation setting inside a set is not part of a set: the reader refuses it.
    v["lasers"][0]["sets"]["cut"]["crosshatch"] = serde_json::json!(true);
    let text = serde_json::to_string_pretty(&v).unwrap();
    assert!(read_record(&file_name(FULL_ID), &text).is_err());
}

// ---- criterion 27: photo checks ----------------------------------------------------------------

#[test]
fn c27_png_and_jpeg_headers_at_600_pass_601_fail() {
    let h = check_photo(&png(600, 400)).unwrap();
    assert_eq!((h.kind, h.width, h.height), (PhotoKind::Png, 600, 400));
    let h = check_photo(&png(10, 600)).unwrap();
    assert_eq!(h.height, 600);
    assert_eq!(check_photo(&png(601, 10)), Err(PhotoRefusal::TooWide));
    assert_eq!(check_photo(&png(10, 601)), Err(PhotoRefusal::TooWide));

    for sof in [0xC0, 0xC1, 0xC2] {
        let h = check_photo(&jpeg(sof, 600, 450)).unwrap();
        assert_eq!(
            (h.kind, h.width, h.height),
            (PhotoKind::Jpg, 600, 450),
            "{sof:x}"
        );
        assert_eq!(
            check_photo(&jpeg(sof, 450, 601)),
            Err(PhotoRefusal::TooWide)
        );
    }
}

#[test]
fn c27_goldens_and_non_images() {
    let dir = fixtures().join("photos");
    let ok = std::fs::read(dir.join("7k2qx_600.jpg")).unwrap();
    assert_eq!(check_photo(&ok).unwrap().kind, PhotoKind::Jpg);
    let wide = std::fs::read(dir.join("wide_601.png")).unwrap();
    assert_eq!(check_photo(&wide), Err(PhotoRefusal::TooWide));
    let not = std::fs::read(dir.join("not_an_image.jpg")).unwrap();
    assert_eq!(check_photo(&not), Err(PhotoRefusal::NotAnImage));
    assert_eq!(check_photo(&[]), Err(PhotoRefusal::NotAnImage));
    assert_eq!(
        check_photo(b"GIF89a\x10\0\x10\0"),
        Err(PhotoRefusal::NotAnImage)
    );
    // A PNG signature with no IHDR, and a JPEG SOI with no SOF: no readable size.
    assert_eq!(
        check_photo(b"\x89PNG\r\n\x1a\n"),
        Err(PhotoRefusal::NotAnImage)
    );
    assert_eq!(
        check_photo(&[0xFF, 0xD8, 0xFF, 0xD9]),
        Err(PhotoRefusal::NotAnImage)
    );
}

#[test]
fn c27_more_than_1_mib_is_refused() {
    let mut big = png(600, 600);
    big.resize(MAX_PHOTO_BYTES + 1, 0);
    assert_eq!(check_photo(&big), Err(PhotoRefusal::TooLarge));
    let mut at = png(600, 600);
    at.resize(MAX_PHOTO_BYTES, 0);
    assert!(check_photo(&at).is_ok());
}

#[test]
fn c27_photo_file_sits_next_to_the_record() {
    let test = TestId::parse("7K2QX").unwrap();
    let (kind, write) = write_photo(FULL_ID, test, png(100, 100)).unwrap();
    assert_eq!(kind, PhotoKind::Png);
    let text = format!("{write:?}");
    assert!(text.contains(&format!("{FULL_ID}.7K2QX.png")), "{text}");
    let (kind, write) = write_photo(FULL_ID, test, jpeg(0xC0, 100, 80)).unwrap();
    assert_eq!(kind, PhotoKind::Jpg);
    assert!(format!("{write:?}").contains(&format!("{FULL_ID}.7K2QX.jpg")));
}
