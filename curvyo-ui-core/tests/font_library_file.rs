//! The golden fixtures of the font library file (`specs/0075-font-library` criterion 12,
//! `adrs.md` decision 1): a valid file, each failing rule, and the round trip.
#![allow(clippy::unwrap_used, missing_docs)]

use std::fmt::Write as _;

use curvyo_ui_core::{FamilyFacts, FontLibraryFileError, read_font_library, write_font_library};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(format!(
        "{}/tests/fixtures/font_library/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

/// The valid file is read with its stars, tags, overrides and chosen cell, and written back to
/// the same text.
#[test]
fn ac12_the_valid_file_reads_and_writes_back_to_itself() {
    let text = fixture("valid.toml");
    let library = read_font_library(&text).unwrap();
    let none = FamilyFacts::default();
    assert_eq!(library.chosen(), "Wedding");
    assert!(library.is_favourite("lobster"));
    assert!(library.is_member("script", "lobster", &none));
    assert!(library.is_member("Wedding", "great vibes", &none));
    assert_eq!(library.own_tags(), ["Signs", "Wedding"]);
    assert_eq!(write_font_library(&library), text);
    assert_eq!(
        read_font_library(&write_font_library(&library)).unwrap(),
        library
    );
}

/// Every rule has a failing fixture; a newer format is its own error, read before any other.
#[test]
fn ac12_every_rule_has_a_failing_fixture() {
    assert_eq!(
        read_font_library(&fixture("newer.toml")),
        Err(FontLibraryFileError::Newer)
    );
    for name in [
        "syntax.toml",
        "no_format.toml",
        "format_zero.toml",
        "unknown_key.toml",
        "override_not_built_in.toml",
        "override_adds_and_removes.toml",
        "tag_name_too_long.toml",
        "tag_name_empty.toml",
        "tag_is_built_in.toml",
        "tag_is_favourites.toml",
        "tag_two_spellings.toml",
    ] {
        assert!(
            matches!(
                read_font_library(&fixture(name)),
                Err(FontLibraryFileError::Broken(_))
            ),
            "{name}"
        );
    }
}

/// The limits: 256 KiB, 100 own tags, 10,000 keys per list.
#[test]
fn ac12_the_limits() {
    let big = format!("format = 1\n# {}\n", "x".repeat(256 * 1024));
    assert!(read_font_library(&big).is_err());
    let mut tags = String::from("format = 1\n\n[tags]\n");
    for n in 0..101 {
        let _ = writeln!(tags, "\"f{n}\" = [\"T{n}\"]");
    }
    assert!(read_font_library(&tags).is_err());
    let keys: Vec<String> = (0..10_001).map(|n| format!("\"f{n}\"")).collect();
    let many = format!("format = 1\nfavourites = [{}]\n", keys.join(", "));
    assert!(read_font_library(&many).is_err());
}
