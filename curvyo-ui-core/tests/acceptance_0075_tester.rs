//! Tester acceptance tests for `specs/0075-font-library`, the tag criteria reworded on
//! 2026-10-20 (4, 6, 7, 8, 10, 11). Written from `specification.md` and the public names
//! before the implementation was read.
#![allow(clippy::unwrap_used, clippy::assert_is_empty, missing_docs)]

use std::collections::BTreeSet;

use curvyo_text_core::DeclaredClass;
use curvyo_ui_core::{
    ALL_ID, BuiltInTag, CheckResult, FAVOURITES_ID, FamilyFacts, FontLibrary, MemberSource,
    read_font_library, write_font_library,
};

fn nothing() -> FamilyFacts {
    FamilyFacts::default()
}

fn one_piece() -> FamilyFacts {
    FamilyFacts {
        check: Some(CheckResult::Pieces(1)),
        ..FamilyFacts::default()
    }
}

fn declared_script() -> FamilyFacts {
    FamilyFacts {
        declared: Some(DeclaredClass::Script),
        ..FamilyFacts::default()
    }
}

fn tag_names(lib: &FontLibrary, key: &str, facts: &FamilyFacts) -> Vec<String> {
    lib.tags_of(key, facts)
        .into_iter()
        .map(|(entry, _)| entry.name)
        .collect()
}

fn strip(lib: &FontLibrary) -> Vec<String> {
    lib.all_tags().into_iter().map(|t| t.name).collect()
}

fn round_trip(lib: &FontLibrary) -> FontLibrary {
    read_font_library(&write_font_library(lib)).unwrap()
}

/// AC 1, 6: the built-in tags are always in the strip, first, in the fixed order; own tags
/// follow A to Z ignoring case.
#[test]
fn ac1_ac6_strip_order_builtins_first_then_own_a_to_z() {
    let mut lib = FontLibrary::default();
    // `specs/0076-single-line-fonts` criterion 4 puts Single line after Cuttable.
    assert_eq!(
        strip(&lib),
        [
            "Cuttable",
            "Single line",
            "Script",
            "Sans",
            "Serif",
            "Initials"
        ]
    );
    lib.add_tag("lobster", "wedding", &nothing()).unwrap();
    lib.add_tag("lobster", "Arabic", &nothing()).unwrap();
    lib.add_tag("lobster", "Zebra", &nothing()).unwrap();
    assert_eq!(
        strip(&lib),
        [
            "Cuttable",
            "Single line",
            "Script",
            "Sans",
            "Serif",
            "Initials",
            "Arabic",
            "wedding",
            "Zebra"
        ]
    );
}

/// AC 10: Add tag with a new name creates the tag, it shows on the family's tag line and
/// in the strip; the chip order is the strip order.
#[test]
fn ac10_add_new_name_creates_tag_and_chip_order_is_strip_order() {
    let mut lib = FontLibrary::default();
    assert!(lib.add_tag("lobster", "Wedding", &one_piece()).unwrap());
    assert!(lib.add_tag("lobster", "Initials", &one_piece()).unwrap());
    assert_eq!(
        tag_names(&lib, "lobster", &one_piece()),
        ["Cuttable", "Initials", "Wedding"]
    );
    assert!(lib.is_member("Wedding", "lobster", &one_piece()));
    // A second add of the same tag changes nothing.
    assert!(!lib.add_tag("lobster", "Wedding", &one_piece()).unwrap());
}

/// AC 7: a name equal to an existing tag ignoring case adds that tag in its spelling.
#[test]
fn ac7_add_field_reuses_existing_tag_ignoring_case() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    lib.add_tag("pacifico", "WEDDING", &nothing()).unwrap();
    assert_eq!(lib.own_tags(), ["Wedding"]);
    assert_eq!(tag_names(&lib, "pacifico", &nothing()), ["Wedding"]);
    // A built-in name in another case is the built-in tag (an override), not an own tag.
    lib.add_tag("pacifico", "script", &nothing()).unwrap();
    assert_eq!(lib.own_tags(), ["Wedding"]);
    assert!(
        lib.membership(BuiltInTag::Script, "pacifico", &nothing())
            .member
    );
}

/// AC 7: 1 to 24 characters, not All or Favourites; nothing changes on a refusal.
#[test]
fn ac7_add_field_refuses_bad_names_and_changes_nothing() {
    let mut lib = FontLibrary::default();
    let before = lib.clone();
    assert!(lib.add_tag("lobster", "", &nothing()).is_err());
    assert!(lib.add_tag("lobster", "   ", &nothing()).is_err());
    assert!(lib.add_tag("lobster", &"x".repeat(25), &nothing()).is_err());
    assert!(lib.add_tag("lobster", "All", &nothing()).is_err());
    assert!(lib.add_tag("lobster", "favourites", &nothing()).is_err());
    assert_eq!(lib, before);
    // 24 characters, also non-ASCII, are fine.
    assert!(lib.add_tag("lobster", &"ä".repeat(24), &nothing()).unwrap());
}

/// AC 10: completion offers existing tags the family does not carry, by prefix ignoring case.
#[test]
fn ac10_completion_offers_existing_tags_the_family_lacks() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    lib.add_tag("lobster", "Wood", &nothing()).unwrap();
    lib.add_tag("pacifico", "Wood", &nothing()).unwrap();
    assert_eq!(lib.completions("pacifico", "w", &nothing()), ["Wedding"]);
    assert_eq!(
        lib.completions("roboto", "W", &nothing()),
        ["Wedding", "Wood"]
    );
    // Built-in tags complete too, but not one the family already carries automatically.
    assert_eq!(lib.completions("roboto", "sc", &nothing()), ["Script"]);
    assert!(
        lib.completions("vibes", "sc", &declared_script())
            .is_empty()
    );
}

/// AC 10, 17: x on an automatic tag is a "removed" override; the tag stays hidden, also
/// after a restart; adding it back clears the override and the automatic source returns.
#[test]
fn ac10_removed_automatic_tag_stays_hidden_and_add_back_clears_override() {
    let mut lib = FontLibrary::default();
    let facts = one_piece();
    assert!(lib.membership(BuiltInTag::Cuttable, "vibes", &facts).member);
    assert!(lib.remove_tag("vibes", BuiltInTag::Cuttable.id(), &facts));
    assert!(!tag_names(&lib, "vibes", &facts).contains(&"Cuttable".to_owned()));
    assert!(!lib.is_member(BuiltInTag::Cuttable.id(), "vibes", &facts));

    // Hidden after a restart, and when the check is run again with the same result.
    let restarted = round_trip(&lib);
    assert!(
        !restarted
            .membership(BuiltInTag::Cuttable, "vibes", &facts)
            .member
    );

    // Adding it back clears the override: the source is the check again, not "by you".
    lib.add_tag("vibes", "Cuttable", &facts).unwrap();
    let m = lib.membership(BuiltInTag::Cuttable, "vibes", &facts);
    assert!(m.member);
    assert_eq!(m.source, MemberSource::Detected(1));
    assert_eq!(
        write_font_library(&lib),
        write_font_library(&FontLibrary::default())
    );
}

/// AC 10: a built-in tag the maker adds is "Added by you"; the declared one is "Declared".
#[test]
fn ac10_chip_sources() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Serif", &declared_script()).unwrap();
    let tags = lib.tags_of("lobster", &declared_script());
    let source = |name: &str| tags.iter().find(|(e, _)| e.name == name).unwrap().1;
    assert_eq!(
        source("Script"),
        MemberSource::Declared(DeclaredClass::Script)
    );
    assert_eq!(source("Serif"), MemberSource::ByYou);
}

/// AC 6, 10: removing the last use of an own tag deletes it, and a chosen cell of that tag
/// switches to All at once (AC 6, customer bug report 2026-10-20), also after a restart (AC 4).
#[test]
fn ac6_last_use_removed_deletes_own_tag_and_chosen_cell_falls_back_to_all() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    assert!(lib.choose("Wedding"));
    assert!(lib.remove_tag("lobster", "Wedding", &nothing()));
    assert!(lib.own_tags().is_empty());
    assert_eq!(lib.chosen(), ALL_ID);
    assert_eq!(round_trip(&lib).chosen(), ALL_ID);
    assert!(lib.choose(FAVOURITES_ID));
    assert_eq!(lib.chosen(), FAVOURITES_ID);
}

/// AC 4: the chosen cell survives a restart: a built-in, Favourites and an own tag in use.
#[test]
fn ac4_chosen_cell_survives_restart() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    for id in [FAVOURITES_ID, BuiltInTag::Cuttable.id(), "Wedding"] {
        assert!(lib.choose(id));
        assert_eq!(round_trip(&lib).chosen(), id);
    }
}

/// AC 7: rename keeps families, refuses a name taken ignoring case, allows a case change,
/// and built-in tags cannot be renamed.
#[test]
fn ac7_rename() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    lib.add_tag("pacifico", "Wedding", &nothing()).unwrap();
    lib.add_tag("pacifico", "Signs", &nothing()).unwrap();

    let before = lib.clone();
    assert!(lib.rename_tag("Wedding", "SIGNS").is_err());
    assert!(lib.rename_tag("Wedding", "script").is_err());
    assert!(lib.rename_tag("Wedding", "Favourites").is_err());
    assert!(lib.rename_tag("Wedding", &"y".repeat(25)).is_err());
    assert!(lib.rename_tag(BuiltInTag::Script.id(), "Hand").is_err());
    assert_eq!(lib, before);

    lib.rename_tag("Wedding", "wedding").unwrap();
    lib.rename_tag("wedding", "Marriage").unwrap();
    assert_eq!(lib.own_tags(), ["Marriage", "Signs"]);
    assert_eq!(tag_names(&lib, "lobster", &nothing()), ["Marriage"]);
    assert_eq!(
        tag_names(&lib, "pacifico", &nothing()),
        ["Marriage", "Signs"]
    );
}

/// AC 8, 11: Delete takes the tag off every family, also families not listed here; it
/// never touches built-in tags or favourites.
#[test]
fn ac8_delete_removes_from_every_family_builtins_cannot_be_deleted() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    lib.add_tag("not-on-this-computer", "Wedding", &nothing())
        .unwrap();
    lib.add_tag("lobster", "Initials", &nothing()).unwrap();
    lib.toggle_favourite("lobster");

    assert!(lib.delete_tag("Wedding"));
    assert!(lib.own_tags().is_empty());
    assert!(tag_names(&lib, "not-on-this-computer", &nothing()).is_empty());
    assert!(!lib.delete_tag(BuiltInTag::Initials.id()));
    assert_eq!(tag_names(&lib, "lobster", &nothing()), ["Initials"]);
    assert!(lib.is_favourite("lobster"));
}

/// AC 11: tags and the star of a family that is not listed are kept in the file and come
/// back with it; an own tag on an unlisted family alone keeps the tag alive.
#[test]
fn ac11_unlisted_family_keeps_tags_and_star_through_the_file() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    lib.add_tag("lobster", "Initials", &nothing()).unwrap();
    lib.remove_tag("lobster", BuiltInTag::Cuttable.id(), &one_piece());
    lib.toggle_favourite("lobster");
    let back = round_trip(&lib);
    assert_eq!(back, lib);
    assert_eq!(back.own_tags(), ["Wedding"]);
    assert_eq!(
        tag_names(&back, "lobster", &one_piece()),
        ["Initials", "Wedding"]
    );
    assert!(back.is_favourite("lobster"));
}

/// AC 2, 6: own tags are strip cells; a family is in the cell when it carries the tag.
#[test]
fn ac2_cell_membership_for_own_and_builtin_tags() {
    let mut lib = FontLibrary::default();
    lib.add_tag("lobster", "Wedding", &nothing()).unwrap();
    assert!(lib.is_member(ALL_ID, "anything", &nothing()));
    assert!(lib.is_member("Wedding", "lobster", &nothing()));
    assert!(!lib.is_member("Wedding", "pacifico", &nothing()));
    assert!(lib.is_member(BuiltInTag::Script.id(), "vibes", &declared_script()));
    let shipped = FamilyFacts {
        shipped: Some(BTreeSet::from([BuiltInTag::Serif])),
        ..FamilyFacts::default()
    };
    assert!(lib.is_member(BuiltInTag::Serif.id(), "source serif 4", &shipped));
}
