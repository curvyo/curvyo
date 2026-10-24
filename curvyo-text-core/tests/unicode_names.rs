//! The shipped Unicode character names and blocks (`specs/0082-glyph-table` criteria 6, 16 and
//! 17; `adrs.md` decision 2).
#![allow(clippy::unwrap_used)]

use curvyo_text_core::{TextError, UNICODE_VERSION, UnicodeNames, block_of};

fn names_file() -> Vec<u8> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    std::fs::read(dir.join(format!(
        "../frontend/public/unicode/names-{UNICODE_VERSION}.bin"
    )))
    .unwrap()
}

fn shipped() -> UnicodeNames {
    UnicodeNames::parse(&names_file()).unwrap()
}

fn name(names: &UnicodeNames, c: char) -> Option<String> {
    names.name_of(c)
}

#[test]
fn the_shipped_file_names_stored_and_computed_characters() {
    let names = shipped();
    assert_eq!(names.version(), (17, 0, 0));
    assert_eq!(
        name(&names, '\u{2665}').as_deref(),
        Some("BLACK HEART SUIT")
    );
    assert_eq!(
        name(&names, '\u{2764}').as_deref(),
        Some("HEAVY BLACK HEART")
    );
    assert_eq!(
        name(&names, 'ä').as_deref(),
        Some("LATIN SMALL LETTER A WITH DIAERESIS")
    );
    assert_eq!(name(&names, '-').as_deref(), Some("HYPHEN-MINUS"));
    assert_eq!(
        name(&names, '\u{AC00}').as_deref(),
        Some("HANGUL SYLLABLE GA")
    );
    assert_eq!(
        name(&names, '\u{D7A3}').as_deref(),
        Some("HANGUL SYLLABLE HIH")
    );
    assert_eq!(
        name(&names, '\u{AC01}').as_deref(),
        Some("HANGUL SYLLABLE GAG")
    );
    assert_eq!(
        name(&names, '\u{4E00}').as_deref(),
        Some("CJK UNIFIED IDEOGRAPH-4E00")
    );
    assert_eq!(
        name(&names, '\u{2F800}').as_deref(),
        Some("CJK COMPATIBILITY IDEOGRAPH-2F800")
    );
    assert_eq!(
        name(&names, '\u{17000}').as_deref(),
        Some("TANGUT IDEOGRAPH-17000")
    );
    assert_eq!(name(&names, '\u{E000}'), None, "private use has no name");
    assert_eq!(
        name(&names, '\u{7}'),
        None,
        "control characters have no name"
    );
    assert_eq!(name(&names, '\u{1F600}').as_deref(), Some("GRINNING FACE"));
}

#[test]
fn abbreviations_label_the_empty_characters() {
    let names = shipped();
    assert_eq!(names.abbreviation_of(' '), Some("SP"));
    assert_eq!(names.abbreviation_of('\u{A0}'), Some("NBSP"));
    assert_eq!(names.abbreviation_of('\u{200D}'), Some("ZWJ"));
    assert_eq!(names.abbreviation_of('\u{FEFF}'), Some("ZWNBSP"));
    assert_eq!(names.abbreviation_of('A'), None);
}

#[test]
fn the_names_file_stays_within_its_budget() {
    // Criterion 17: at most 1 MiB loaded for the names.
    assert!(names_file().len() <= 1024 * 1024, "{}", names_file().len());
}

#[test]
fn a_damaged_file_is_an_error_not_a_panic() {
    let bytes = names_file();
    for cut in [0, 3, 4, 8, 100, bytes.len() / 2, bytes.len() - 1] {
        assert_eq!(
            UnicodeNames::parse(&bytes[..cut]).unwrap_err(),
            TextError::UnreadableNames,
            "cut at {cut}"
        );
    }
    let mut wrong = bytes.clone();
    wrong[0] = b'X';
    assert!(UnicodeNames::parse(&wrong).is_err());
    let mut longer = bytes;
    longer.push(0);
    assert!(UnicodeNames::parse(&longer).is_err(), "trailing bytes");
    // A word index past the dictionary, an endless varint.
    assert!(UnicodeNames::parse(b"CVYN\x01\x11\x00\x00\x01\x01A\x00\x01\x41\x01\x05\x00").is_err());
    assert!(UnicodeNames::parse(b"CVYN\x01\x11\x00\x00\xff\xff\xff\xff\xff\xff\xff").is_err());
}

#[test]
fn blocks_follow_unicode_17() {
    assert_eq!(block_of('A'), Some("Basic Latin"));
    assert_eq!(block_of('ä'), Some("Latin-1 Supplement"));
    assert_eq!(block_of('\u{2192}'), Some("Arrows"));
    assert_eq!(block_of('\u{2764}'), Some("Dingbats"));
    assert_eq!(block_of('\u{E000}'), Some("Private Use Area"));
    assert_eq!(block_of('\u{2FFFF}'), None, "no block");
    assert_eq!(
        block_of('\u{10FFFF}'),
        Some("Supplementary Private Use Area-B")
    );
}
