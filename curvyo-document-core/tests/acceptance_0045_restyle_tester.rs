//! Independent tester cases for the 2026-11-10 follow-up of
//! `0045-document-formats-library` (criteria 5 amended, 11 amended, 27 amended,
//! 39, 42 data side, open question 14), document-core side: formats of one size
//! are allowed in the user file, in add and edit and in an import; the first
//! match in list order is the current format; the shipped file keeps its
//! no-duplicate rule; the user file's `format` stays 1. Written from
//! `specification.md` before the implementation diff was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    missing_docs
)]

use curvyo_document_core::{
    DocumentSize, FormatLibrary, FormatReason, FormatSpec, GroupTarget, Length, Orientation,
    PresetList, PresetReason, PresetUnit,
};

const SHIPPED_TEXT: &str = include_str!("../data/document-presets.toml");
const VALID_SAME_SIZE: &str = include_str!("fixtures/formats/valid_same_size.toml");

fn lib_from(user: &str) -> FormatLibrary {
    FormatLibrary::load(PresetList::shipped().unwrap(), user)
        .unwrap_or_else(|e| panic!("refused: {e:?}\n{user}"))
}

fn size(w: f64, h: f64) -> DocumentSize {
    DocumentSize {
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    }
}

fn spec_in(name: &str, group: &str, short: f64, long: f64) -> FormatSpec {
    FormatSpec {
        name: name.to_string(),
        group: GroupTarget::Existing(group.to_string()),
        short,
        long,
        unit: PresetUnit::Mm,
        favourite: true,
    }
}

fn spec_new(name: &str, group: &str, short: f64, long: f64) -> FormatSpec {
    FormatSpec {
        name: name.to_string(),
        group: GroupTarget::New {
            name: group.to_string(),
            orientation: Orientation::Portrait,
        },
        short,
        long,
        unit: PresetUnit::Mm,
        favourite: true,
    }
}

fn current(lib: &FormatLibrary, w: f64, h: f64) -> Option<String> {
    lib.list()
        .matching(size(w, h))
        .map(|(_, preset)| preset.id.clone())
}

fn names_in(lib: &FormatLibrary, group: &str) -> Vec<String> {
    lib.list()
        .groups
        .iter()
        .find(|g| g.id == group)
        .map(|g| g.presets.iter().map(|p| p.name.clone()).collect())
        .unwrap_or_default()
}

const KEYRING_AND_COASTER: &str = r#"format = 1

[[group]]
id = "u-laser"
name = "Laser"
default_orientation = "portrait"

  [[group.preset]]
  id = "u-key-ring"
  name = "Key ring"
  short_side = 90
  long_side = 90
  unit = "mm"

  [[group.preset]]
  id = "u-coaster"
  name = "Coaster"
  short_side = 90
  long_side = 90
  unit = "mm"
"#;

// ---- criterion 5 (amended) / 39: the user file ------------------------------

#[test]
fn c39_a_user_file_with_two_formats_of_one_size_loads_and_round_trips() {
    let lib = lib_from(KEYRING_AND_COASTER);
    assert_eq!(names_in(&lib, "u-laser"), ["Key ring", "Coaster"]);
    let text = lib.user_text().unwrap();
    let again = lib_from(&text);
    assert_eq!(names_in(&again, "u-laser"), ["Key ring", "Coaster"]);
    assert_eq!(again.user_text().unwrap(), text, "canonical text is stable");
}

#[test]
fn c39_a_user_format_may_have_the_size_of_a_built_in_in_either_orientation() {
    // 297 x 210 mm is A4 landscape; 29.7 cm x 21 cm is A4 in cm.
    let user = r#"format = 1
[[group]]
id = "u-mine"
name = "Mine"
default_orientation = "landscape"
  [[group.preset]]
  id = "u-a4-cm"
  name = "A4 in cm"
  short_side = 21
  long_side = 29.7
  unit = "cm"
"#;
    let lib = lib_from(user);
    assert_eq!(names_in(&lib, "u-mine"), ["A4 in cm"]);
}

#[test]
fn c39_the_renamed_fixture_is_valid_and_keeps_both_formats_through_a_round_trip() {
    let lib = lib_from(VALID_SAME_SIZE);
    let paper = names_in(&lib, "paper");
    assert!(paper.contains(&"My A4".to_string()), "{paper:?}");
    assert!(paper.contains(&"Copy".to_string()), "{paper:?}");
    let again = lib_from(&lib.user_text().unwrap());
    assert_eq!(names_in(&again, "paper"), paper);
}

#[test]
#[ignore = "spec gap reported to the lead: criterion 39 says the name rules stay for a user file, criterion 5 never listed a unique name in the loader; since the same-size rule went, a user file with two identical formats loads"]
fn c5_name_rule_still_refuses_two_formats_of_one_name_in_one_group_in_the_file() {
    // The same-size rule went; the "unique name in its group" rule stays.
    let user = KEYRING_AND_COASTER.replace("name = \"Coaster\"", "name = \"Key ring\"");
    assert!(
        FormatLibrary::load(PresetList::shipped().unwrap(), &user).is_err(),
        "two formats named Key ring in one group must be refused"
    );
}

// ---- built-in file keeps its rule --------------------------------------------

#[test]
fn c39_the_built_in_file_still_refuses_two_formats_of_one_size() {
    let dup = SHIPPED_TEXT.replacen(
        "  [[group.preset]]\n  id = \"a1\"",
        "  [[group.preset]]\n  id = \"a0-copy\"\n  name = \"A0 copy\"\n  short_side = 841\n  long_side = 1189\n  unit = \"mm\"\n\n  [[group.preset]]\n  id = \"a1\"",
        1,
    );
    assert_ne!(dup, SHIPPED_TEXT, "the injection must hit the shipped text");
    let err = PresetList::parse(&dup).expect_err("built-in duplicate must be refused");
    assert!(
        matches!(err.reason, PresetReason::SameSizeAs(_)),
        "reason: {:?}",
        err.reason
    );
}

#[test]
fn c39_the_shipped_file_has_no_two_formats_of_one_size() {
    let list = PresetList::shipped().unwrap();
    let all: Vec<_> = list.presets().collect();
    for (i, (_, a)) in all.iter().enumerate() {
        for (_, b) in &all[i + 1..] {
            let same = (a.short_side.as_mm() - b.short_side.as_mm()).abs() <= 0.01
                && (a.long_side.as_mm() - b.long_side.as_mm()).abs() <= 0.01;
            assert!(!same, "{} and {} share a size", a.id, b.id);
        }
    }
}

// ---- add and edit ------------------------------------------------------------

#[test]
fn c39_add_accepts_the_size_of_a_built_in_and_of_another_user_format() {
    let mut lib = FormatLibrary::shipped().unwrap();
    let a = lib
        .add_format(&spec_new("Key ring", "Laser", 90.0, 90.0))
        .unwrap();
    let group = lib.list().find(&a).unwrap().0.id.clone();
    lib.add_format(&spec_in("Coaster", &group, 90.0, 90.0))
        .unwrap();
    lib.add_format(&spec_in("My A4", &group, 210.0, 297.0))
        .unwrap();
    lib.add_format(&spec_in("My A4 cm", &group, 210.000_5, 297.0))
        .unwrap();
    assert_eq!(
        names_in(&lib, &group),
        ["Key ring", "Coaster", "My A4", "My A4 cm"]
    );
    let again = lib_from(&lib.user_text().unwrap());
    assert_eq!(names_in(&again, &group), names_in(&lib, &group));
}

#[test]
fn c39_add_still_refuses_a_name_the_group_has_and_allows_it_in_another_group() {
    let mut lib = FormatLibrary::shipped().unwrap();
    let a = lib
        .add_format(&spec_new("Key ring", "Laser", 90.0, 90.0))
        .unwrap();
    let group = lib.list().find(&a).unwrap().0.id.clone();
    let err = lib
        .add_format(&spec_in("key ring", &group, 30.0, 50.0))
        .err()
        .or_else(|| {
            lib.add_format(&spec_in("Key ring", &group, 30.0, 50.0))
                .err()
        })
        .expect("a name the group has must be refused");
    assert!(
        matches!(err.reason, FormatReason::NameInGroup(_)),
        "{err:?}"
    );
    lib.add_format(&spec_new("Key ring", "Embroidery", 90.0, 90.0))
        .unwrap();
}

#[test]
fn c39_edit_accepts_the_size_of_another_format() {
    let mut lib = FormatLibrary::shipped().unwrap();
    let a = lib
        .add_format(&spec_new("Key ring", "Laser", 30.0, 50.0))
        .unwrap();
    let group = lib.list().find(&a).unwrap().0.id.clone();
    let b = lib
        .add_format(&spec_in("Coaster", &group, 90.0, 90.0))
        .unwrap();
    lib.edit_format(&a, &spec_in("Key ring", &group, 90.0, 90.0))
        .unwrap();
    lib.edit_format(&b, &spec_in("Coaster", &group, 148.0, 210.0))
        .unwrap(); // A5
    assert_eq!(lib.list().find(&a).unwrap().1.short_side.as_mm(), 90.0);
    assert_eq!(lib.list().find(&b).unwrap().1.long_side.as_mm(), 210.0);
}

#[test]
fn c39_check_format_has_no_size_refusal() {
    let mut lib = FormatLibrary::shipped().unwrap();
    let a = lib
        .add_format(&spec_new("Key ring", "Laser", 90.0, 90.0))
        .unwrap();
    let group = lib.list().find(&a).unwrap().0.id.clone();
    assert!(
        lib.check_format(&spec_in("Other", &group, 90.0, 90.0), None)
            .is_ok()
    );
    assert!(
        lib.check_format(&spec_in("A4 twin", "paper", 210.0, 297.0), None)
            .is_ok()
    );
}

// ---- criterion 11 / 39: the current format ----------------------------------

#[test]
fn c39_the_first_format_in_list_order_is_the_current_one() {
    let lib = lib_from(KEYRING_AND_COASTER);
    assert_eq!(current(&lib, 90.0, 90.0).as_deref(), Some("u-key-ring"));
}

#[test]
fn c39_a_built_in_comes_before_a_user_format_of_its_size() {
    let mut lib = lib_from(VALID_SAME_SIZE);
    assert_eq!(current(&lib, 210.0, 297.0).as_deref(), Some("a4"));
    assert_eq!(current(&lib, 297.0, 210.0).as_deref(), Some("a4"));
    // The user formats are in Paper too, so turning Paper off leaves none.
    lib.set_group_enabled("paper", false).unwrap();
    assert_eq!(current(&lib, 210.0, 297.0), None);
}

#[test]
fn c39_a_group_that_is_off_is_skipped_and_the_next_match_is_current() {
    let user = r#"format = 1
[[group]]
id = "u-a"
name = "A"
default_orientation = "portrait"
  [[group.preset]]
  id = "u-a-one"
  name = "One"
  short_side = 90
  long_side = 90
  unit = "mm"
[[group]]
id = "u-b"
name = "B"
default_orientation = "portrait"
  [[group.preset]]
  id = "u-b-one"
  name = "One"
  short_side = 90
  long_side = 90
  unit = "mm"
"#;
    let mut lib = lib_from(user);
    assert_eq!(current(&lib, 90.0, 90.0).as_deref(), Some("u-a-one"));
    lib.set_group_enabled("u-a", false).unwrap();
    assert_eq!(current(&lib, 90.0, 90.0).as_deref(), Some("u-b-one"));
    // Within the 0.01 mm tolerance it is the same size.
    assert_eq!(current(&lib, 90.009, 89.991).as_deref(), Some("u-b-one"));
    assert_eq!(current(&lib, 90.02, 90.0), None);
}

// ---- criterion 27 (amended): import ----------------------------------------

#[test]
fn c27_importing_the_same_file_twice_adds_nothing_the_second_time() {
    let mut lib = FormatLibrary::shipped().unwrap();
    let first = lib.import(KEYRING_AND_COASTER).unwrap();
    assert_eq!(first.added, 2, "both formats of one size are added");
    let text = lib.user_text().unwrap();
    let second = lib.import(KEYRING_AND_COASTER).unwrap();
    assert_eq!(second.added, 0);
    assert_eq!(second.skipped, 2);
    assert_eq!(lib.user_text().unwrap(), text, "nothing changed");
}

#[test]
fn c27_a_format_of_an_existing_size_but_another_name_is_added() {
    let mut lib = lib_from(KEYRING_AND_COASTER);
    let other = r#"format = 1
[[group]]
id = "u-laser"
name = "Laser"
default_orientation = "portrait"
  [[group.preset]]
  id = "u-slate"
  name = "Slate"
  short_side = 90
  long_side = 90
  unit = "mm"
"#;
    let rep = lib.import(other).unwrap();
    assert_eq!((rep.added, rep.skipped), (1, 0));
    assert_eq!(names_in(&lib, "u-laser"), ["Key ring", "Coaster", "Slate"]);
}

#[test]
fn c27_a_format_of_a_built_in_size_is_added() {
    let mut lib = FormatLibrary::shipped().unwrap();
    let rep = lib.import(VALID_SAME_SIZE).unwrap();
    assert_eq!((rep.added, rep.skipped), (2, 0));
}

#[test]
fn c27_same_name_and_size_in_another_group_is_not_skipped() {
    let mut lib = lib_from(KEYRING_AND_COASTER);
    let other = KEYRING_AND_COASTER
        .replace("u-laser", "u-shop")
        .replace("\"Laser\"", "\"Shop\"")
        .replace("u-key-ring", "u-key-ring-shop")
        .replace("u-coaster", "u-coaster-shop");
    let rep = lib.import(&other).unwrap();
    assert_eq!((rep.added, rep.skipped), (2, 0));
    assert_eq!(names_in(&lib, "u-shop"), ["Key ring", "Coaster"]);
}

#[test]
fn c27_same_group_and_name_with_another_size_leaves_a_valid_library() {
    // Not skipped (the size differs); whatever the merge does, the result must
    // stay a library that loads again (unique names within a group).
    let mut lib = lib_from(KEYRING_AND_COASTER);
    let other = KEYRING_AND_COASTER
        .replace("u-key-ring", "u-key-ring-big")
        .replace(
            "short_side = 90\n  long_side = 90\n  unit = \"mm\"\n\n  [[group.preset]]",
            "short_side = 95\n  long_side = 95\n  unit = \"mm\"\n\n  [[group.preset]]",
        );
    assert_ne!(other, KEYRING_AND_COASTER);
    let rep = lib.import(&other).unwrap();
    assert_eq!(rep.skipped, 1, "the Coaster 90 x 90 is skipped");
    assert_eq!(rep.added, 1, "the Key ring 95 x 95 is not skipped");
    lib_from(&lib.user_text().unwrap());
}

#[test]
#[ignore = "FAIL reported to the lead: import adds a format whose name its group already has (format_library_import.rs:109); remove this ignore with the fix"]
fn c39_c27_an_import_keeps_names_unique_within_a_group() {
    // Criterion 39: "The name rules stay (1 to 24 characters, unique within
    // its group)" for the form, a user file and an import.
    let mut lib = lib_from(KEYRING_AND_COASTER);
    let other = KEYRING_AND_COASTER
        .replace("u-key-ring", "u-key-ring-big")
        .replace("u-coaster", "u-coaster-big")
        .replace(
            "short_side = 90\n  long_side = 90",
            "short_side = 95\n  long_side = 95",
        );
    lib.import(&other).unwrap();
    let names: Vec<String> = names_in(&lib, "u-laser")
        .iter()
        .map(|n| n.to_lowercase())
        .collect();
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len(), "names in Laser: {names:?}");
}

// ---- criterion 42 (data) and open question 14 --------------------------------

#[test]
fn c42_a_group_made_as_portrait_reads_portrait_in_the_user_file() {
    let mut lib = FormatLibrary::shipped().unwrap();
    let id = lib
        .add_format(&spec_new("Key ring", "Laser", 30.0, 50.0))
        .unwrap();
    let group = lib.list().find(&id).unwrap().0;
    assert_eq!(group.default_orientation, Orientation::Portrait);
    let text = lib.user_text().unwrap();
    assert!(
        text.contains("default_orientation = \"portrait\""),
        "{text}"
    );
}

#[test]
fn q14_the_user_file_schema_stays_1_and_old_files_load() {
    let lib = lib_from(KEYRING_AND_COASTER);
    let text = lib.user_text().unwrap();
    assert!(text.trim_start().starts_with("format = 1"), "{text}");
    // A file in the old schema written before this change still loads.
    lib_from(include_str!("fixtures/formats/valid.toml"));
    lib_from(include_str!("fixtures/formats/tester_valid_full.toml"));
    // And a newer schema is still refused.
    let newer = KEYRING_AND_COASTER.replace("format = 1", "format = 2");
    let err = FormatLibrary::load(PresetList::shipped().unwrap(), &newer).unwrap_err();
    assert_eq!(err.reason, PresetReason::FormatNewer);
}
