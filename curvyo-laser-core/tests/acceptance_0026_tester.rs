//! Independent acceptance tests for `specs/0026-machine-profile` at the laser extension: the
//! profile model and its ranges (criteria 1, 9), the built-in (2), naming of saved and duplicated
//! profiles (3), one record per profile (4), the copy written to the project (6, 15), "Differs"
//! (7), a copy whose id the library lacks (8), the last pick (10), damaged copies and the
//! format-35 reading (12), and the per-record load with its skips (13, ADR 0036 §3).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assigning_clones,
    clippy::assert_is_empty
)]

use curvyo_document_core::{DocumentSize, Length};
use curvyo_laser_core::{
    BaudRate, DAMAGED_NAME_NOTICE, FeedRate, GENERIC_LASER_ID, GENERIC_LASER_NAME, LibraryWrite,
    MAX_RECORD_BYTES, MachineProfile, MachineProfiles, OriginCorner, PowerValue, ProfileCopy,
    RecordError, clamp_address, clamp_baud, clamp_feed, clamp_name, clamp_port, clamp_power,
    clamp_work_side, differs, is_profile_id, new_profile_id, read_choice, read_copy, read_record,
    record_file_name, record_id, save_as_name, write_choice, write_copy, write_record,
};
use serde_json::{Value, json};

const NOTICE_MAX: usize = 70;

fn id(n: u8) -> String {
    new_profile_id([n; 16])
}

/// A user profile `n` named `name` with a connection, for library tests.
fn user(n: u8, name: &str) -> MachineProfile {
    let mut p = MachineProfile::generic_laser();
    p.id = id(n);
    p.modified = 1_762_171_200_000;
    p.values.name = name.to_owned();
    p.values.origin = OriginCorner::TopRight;
    p.values.max_feed = FeedRate::new(12_000).unwrap();
    p.values.max_power = PowerValue::new(255).unwrap();
    p.values.laser_mode = false;
    p.work_area = DocumentSize::from_mm(600.0, 400.0);
    p.connection.connector_address = "192.168.1.20:8080".to_owned();
    p.connection.controller_port = "/dev/ttyUSB0".to_owned();
    p.connection.baud_rate = BaudRate::new(250_000).unwrap();
    p
}

fn entry(p: &MachineProfile) -> (String, String) {
    (record_file_name(&p.id), write_record(p))
}

/// The record text of `p` as JSON, edited by `edit`, for damage cases.
fn record_with(
    p: &MachineProfile,
    edit: impl FnOnce(&mut serde_json::Map<String, Value>),
) -> String {
    let mut v: Value = serde_json::from_str(&write_record(p)).unwrap();
    edit(v.as_object_mut().unwrap());
    serde_json::to_string_pretty(&v).unwrap()
}

fn written_id(w: &LibraryWrite) -> &str {
    match w {
        LibraryWrite::Write { id, .. } | LibraryWrite::Remove { id } => id,
    }
}

fn written_text(w: &LibraryWrite) -> &str {
    match w {
        LibraryWrite::Write { text, .. } => text,
        LibraryWrite::Remove { .. } => panic!("expected a write, got a removal"),
    }
}

fn names(lib: &MachineProfiles) -> Vec<String> {
    lib.listed().iter().map(|p| p.values.name.clone()).collect()
}

// ---- criterion 1: defaults and ranges ----------------------------------------------------------

#[test]
fn ac1_new_profile_defaults_are_the_criterion_values() {
    let g = MachineProfile::generic_laser();
    assert_eq!(g.work_area, DocumentSize::from_mm(400.0, 300.0));
    assert_eq!(g.values.origin, OriginCorner::BottomLeft);
    assert_eq!(g.values.max_feed.get(), 6_000);
    assert_eq!(g.values.max_power.get(), 1_000);
    assert!(g.values.laser_mode);
    assert_eq!(FeedRate::DEFAULT.get(), 6_000);
    assert_eq!(PowerValue::DEFAULT.get(), 1_000);
    assert_eq!(BaudRate::DEFAULT.get(), 115_200);
}

#[test]
fn ac1_integer_ranges_are_inclusive_at_both_ends() {
    assert!(FeedRate::new(0).is_none());
    assert_eq!(FeedRate::new(1).unwrap().get(), 1);
    assert_eq!(FeedRate::new(100_000).unwrap().get(), 100_000);
    assert!(FeedRate::new(100_001).is_none());
    assert!(PowerValue::new(0).is_none());
    assert_eq!(PowerValue::new(65_535).unwrap().get(), 65_535);
    assert!(PowerValue::new(65_536).is_none());
    assert!(BaudRate::new(0).is_none());
    assert_eq!(BaudRate::new(4_000_000).unwrap().get(), 4_000_000);
    assert!(BaudRate::new(4_000_001).is_none());
    assert!(FeedRate::new(u64::MAX).is_none(), "no wrap on huge input");
}

#[test]
fn ac1_a_value_in_range_is_set_without_notice() {
    let f = clamp_feed(1_250.0).unwrap();
    assert_eq!(f.value.get(), 1_250);
    assert_eq!(f.notice, None);
    let p = clamp_power(1.0).unwrap();
    assert_eq!(p.value.get(), 1);
    assert_eq!(p.notice, None);
    let b = clamp_baud(4_000_000.0).unwrap();
    assert_eq!(b.value.get(), 4_000_000);
    assert_eq!(b.notice, None);
}

#[test]
fn ac1_out_of_range_is_set_to_the_nearest_limit_with_a_short_notice() {
    let feed = |t: f64| {
        let l = clamp_feed(t).unwrap();
        (u64::from(l.value.get()), l.notice)
    };
    let power = |t: f64| {
        let l = clamp_power(t).unwrap();
        (u64::from(l.value.get()), l.notice)
    };
    let baud = |t: f64| {
        let l = clamp_baud(t).unwrap();
        (u64::from(l.value.get()), l.notice)
    };
    let cases = [
        (
            feed(100_001.0),
            100_000_u64,
            "Max feed limited to 100,000 mm/min.",
        ),
        (feed(0.0), 1, "Max feed limited to 1 mm/min."),
        (feed(-50.0), 1, "Max feed limited to 1 mm/min."),
        (feed(1e300), 100_000, "Max feed limited to 100,000 mm/min."),
        (power(70_000.0), 65_535, "Max power limited to 65,535."),
        (power(0.0), 1, "Max power limited to 1."),
        (
            baud(5_000_000.0),
            4_000_000,
            "Baud rate limited to 4,000,000.",
        ),
        (baud(0.0), 1, "Baud rate limited to 1."),
    ];
    for ((value, notice), want, text) in cases {
        assert_eq!(value, want);
        let notice = notice.expect("a limited value has a notice");
        assert_eq!(notice, text);
        assert!(notice.chars().count() <= NOTICE_MAX);
    }
}

#[test]
fn ac1_a_fraction_is_rounded_to_the_nearest_whole_number_with_a_notice() {
    let f = clamp_feed(1_250.4).unwrap();
    assert_eq!(f.value.get(), 1_250);
    assert_eq!(f.notice.as_deref(), Some("Max feed rounded to 1,250."));
    let f = clamp_feed(1_250.6).unwrap();
    assert_eq!(f.value.get(), 1_251);
    let p = clamp_power(999.5).unwrap();
    assert_eq!(p.value.get(), 1_000, "half rounds away from zero");
    assert!(p.notice.unwrap().starts_with("Max power rounded to"));
    let b = clamp_baud(115_200.2).unwrap();
    assert_eq!(b.value.get(), 115_200);
    assert_eq!(b.notice.as_deref(), Some("Baud rate rounded to 115,200."));
    // A fraction that rounds into the range below 1 is a limit, not a value of 0.
    let f = clamp_feed(0.4).unwrap();
    assert_eq!(f.value.get(), 1);
    assert!(f.notice.is_some());
}

#[test]
fn ac1_a_non_finite_typed_number_is_not_set() {
    assert!(clamp_feed(f64::NAN).is_none());
    assert!(clamp_power(f64::INFINITY).is_none());
    assert!(clamp_baud(f64::NEG_INFINITY).is_none());
    assert!(clamp_work_side("Width", Length::from_mm(f64::NAN)).is_none());
}

#[test]
fn ac1_work_area_sides_are_limited_to_1_to_10000_mm() {
    let w = clamp_work_side("Width", Length::from_mm(20_000.0)).unwrap();
    assert_eq!(w.value.as_mm(), 10_000.0);
    assert_eq!(w.notice.as_deref(), Some("Width limited to 10,000 mm."));
    let h = clamp_work_side("Height", Length::from_mm(0.2)).unwrap();
    assert_eq!(h.value.as_mm(), 1.0);
    assert_eq!(h.notice.as_deref(), Some("Height limited to 1 mm."));
    let ok = clamp_work_side("Width", Length::from_mm(600.5)).unwrap();
    assert_eq!(ok.value.as_mm(), 600.5, "mm sides are not rounded");
    assert_eq!(ok.notice, None);
}

#[test]
fn ac1_name_is_1_to_60_characters_counted_as_characters_not_bytes() {
    assert!(clamp_name("").is_none(), "empty reverts");
    assert!(clamp_name("   ").is_none(), "blank reverts");
    let ok = clamp_name("K40 in the garage").unwrap();
    assert_eq!(ok.value, "K40 in the garage");
    assert_eq!(ok.notice, None);
    let long = "x".repeat(61);
    let cut = clamp_name(&long).unwrap();
    assert_eq!(cut.value.chars().count(), 60);
    assert_eq!(
        cut.notice.as_deref(),
        Some("Name limited to 60 characters.")
    );
    let sixty_umlauts = "ä".repeat(60);
    let n = clamp_name(&sixty_umlauts).unwrap();
    assert_eq!(n.value, sixty_umlauts, "60 two-byte characters fit");
    assert_eq!(n.notice, None);
    let emoji = "\u{1F525}".repeat(65);
    let n = clamp_name(&emoji).unwrap();
    assert_eq!(n.value.chars().count(), 60);
}

// ---- criterion 2: the built-in ----------------------------------------------------------------

#[test]
fn ac2_generic_laser_exists_and_is_the_only_built_in() {
    let lib = MachineProfiles::load(&[]);
    let listed = lib.listed();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, GENERIC_LASER_ID);
    assert_eq!(listed[0].values.name, GENERIC_LASER_NAME);
    assert_eq!(GENERIC_LASER_NAME, "Generic laser");
    assert!(listed[0].is_builtin());
    assert_eq!(*listed[0], MachineProfile::generic_laser());
    assert!(!lib.is_user(GENERIC_LASER_ID));
    assert!(
        !is_profile_id(GENERIC_LASER_ID),
        "the built-in id is no UUID"
    );
}

#[test]
fn ac2_generic_laser_cannot_be_renamed_updated_deleted_or_connected() {
    let mut lib = MachineProfiles::load(&[]);
    assert_eq!(lib.rename(GENERIC_LASER_ID, "Mine".to_owned(), 1), None);
    let mut values = MachineProfile::generic_laser().values;
    values.max_feed = FeedRate::new(9_000).unwrap();
    assert_eq!(
        lib.update(
            GENERIC_LASER_ID,
            values,
            DocumentSize::from_mm(500.0, 500.0),
            1
        ),
        None
    );
    assert_eq!(lib.delete(GENERIC_LASER_ID), None);
    let mut c = MachineProfile::generic_laser().connection;
    c.controller_port = "COM3".to_owned();
    assert_eq!(lib.set_connection(GENERIC_LASER_ID, c, 1), None);
    assert_eq!(lib.add(MachineProfile::generic_laser()), None);
    assert_eq!(
        *lib.find(GENERIC_LASER_ID).unwrap(),
        MachineProfile::generic_laser()
    );
}

#[test]
fn ac2_duplicate_of_generic_laser_is_an_editable_user_profile_named_copy() {
    let mut lib = MachineProfiles::load(&[]);
    let w = lib.duplicate(GENERIC_LASER_ID, id(1), 5).unwrap();
    assert_eq!(written_id(&w), id(1));
    let p = lib.find(&id(1)).unwrap().clone();
    assert_eq!(p.values.name, "Generic laser copy");
    assert!(lib.is_user(&id(1)));
    assert_eq!(p.work_area, DocumentSize::from_mm(400.0, 300.0));
    assert_eq!(p.values.max_feed.get(), 6_000);
    assert_eq!(
        read_record(&record_file_name(&id(1)), written_text(&w)).unwrap(),
        p
    );
    // Numbered when taken.
    lib.duplicate(GENERIC_LASER_ID, id(2), 6).unwrap();
    assert_eq!(
        lib.find(&id(2)).unwrap().values.name,
        "Generic laser copy 2"
    );
    lib.duplicate(GENERIC_LASER_ID, id(3), 7).unwrap();
    assert_eq!(
        lib.find(&id(3)).unwrap().values.name,
        "Generic laser copy 3"
    );
    // Editable.
    assert!(lib.rename(&id(1), "Shop laser".to_owned(), 8).is_some());
    assert!(lib.delete(&id(2)).is_some());
}

// ---- criterion 3: naming --------------------------------------------------------------------------

#[test]
fn ac3_save_as_name_keeps_a_free_name_and_numbers_a_taken_one() {
    let taken = ["K40", "K40 copy", "K40 copy 2", "Generic laser"];
    let is_taken = |n: &str| taken.contains(&n);
    assert_eq!(save_as_name("Diode", is_taken), "Diode");
    assert_eq!(save_as_name("K40", is_taken), "K40 copy 3");
    assert_eq!(
        save_as_name("Generic laser", is_taken),
        "Generic laser copy"
    );
    assert_eq!(save_as_name("K40 copy", is_taken), "K40 copy copy");
}

#[test]
fn ac3_a_copy_name_is_cut_to_60_characters_keeping_the_suffix() {
    let base = "a".repeat(60);
    let first = save_as_name(&base, |n| n == base);
    assert!(first.ends_with(" copy"), "{first}");
    assert_eq!(first.chars().count(), 60);
    assert_eq!(first, format!("{} copy", "a".repeat(55)));
    let second = save_as_name(&base, |n| n == base || n == first);
    assert_eq!(second, format!("{} copy 2", "a".repeat(53)));
    assert_eq!(second.chars().count(), 60);
    // Multi-byte base, cut by characters.
    let umlaut = "ü".repeat(60);
    let n = save_as_name(&umlaut, |n| n == umlaut);
    assert_eq!(n.chars().count(), 60);
    assert!(n.ends_with(" copy"));
    // Many copies: the number grows and the whole stays within 60.
    let mut taken = vec![base.clone()];
    for _ in 0..120 {
        let next = save_as_name(&base, |n| taken.iter().any(|t| t == n));
        assert!(next.chars().count() <= 60, "{next}");
        assert!(!taken.contains(&next));
        taken.push(next);
    }
    assert!(taken.last().unwrap().ends_with(" copy 120"));
}

#[test]
fn ac3_name_taken_includes_the_built_in_and_user_profiles() {
    let lib = MachineProfiles::load(&[entry(&user(1, "K40"))]);
    assert!(lib.name_taken("Generic laser"));
    assert!(lib.name_taken("K40"));
    assert!(!lib.name_taken("k40 copy"));
}

#[test]
fn ac3_duplicate_of_a_user_profile_keeps_values_and_connection_under_a_new_id() {
    let k40 = user(1, "K40");
    let mut lib = MachineProfiles::load(&[entry(&k40)]);
    lib.duplicate(&k40.id, id(2), 99).unwrap();
    let d = lib.find(&id(2)).unwrap();
    assert_eq!(d.values.name, "K40 copy");
    assert_eq!(d.values.origin, k40.values.origin);
    assert_eq!(d.values.max_feed, k40.values.max_feed);
    assert_eq!(d.work_area, k40.work_area);
    assert_eq!(d.modified, 99);
    assert_eq!(lib.duplicate(&id(77), id(3), 1), None, "unknown profile");
}

#[test]
fn ac3_hand_renaming_may_duplicate_another_name() {
    let mut lib = MachineProfiles::load(&[entry(&user(1, "K40")), entry(&user(2, "Diode"))]);
    let w = lib.rename(&id(2), "K40".to_owned(), 3).unwrap();
    assert_eq!(written_id(&w), id(2));
    assert_eq!(lib.find(&id(2)).unwrap().values.name, "K40");
    assert!(lib.rename(&id(2), "Generic laser".to_owned(), 4).is_some());
    assert_eq!(
        lib.rename(&id(2), "Generic laser".to_owned(), 5),
        None,
        "unchanged"
    );
}

#[test]
fn ac3_delete_returns_a_removal_and_the_profile_is_gone() {
    let mut lib = MachineProfiles::load(&[entry(&user(1, "K40"))]);
    assert_eq!(lib.delete(&id(1)), Some(LibraryWrite::Remove { id: id(1) }));
    assert!(lib.find(&id(1)).is_none());
    assert_eq!(lib.delete(&id(1)), None);
    assert_eq!(names(&lib), vec!["Generic laser"]);
}

#[test]
fn ac3_list_order_is_generic_first_then_by_name_ignoring_case() {
    let lib = MachineProfiles::load(&[
        entry(&user(1, "zeta")),
        entry(&user(2, "Alpha")),
        entry(&user(3, "beta")),
        entry(&user(4, "Aardvark laser")),
    ]);
    assert_eq!(
        names(&lib),
        vec!["Generic laser", "Aardvark laser", "Alpha", "beta", "zeta"]
    );
}

// ---- criterion 4 and ADR 0036 §2: one record per profile -------------------------------------

#[test]
fn ac4_each_edit_writes_only_its_own_record() {
    let a = user(1, "A");
    let b = user(2, "B");
    let mut lib = MachineProfiles::load(&[entry(&a), entry(&b)]);
    let w = lib.rename(&a.id, "A2".to_owned(), 10).unwrap();
    let text = written_text(&w);
    assert_eq!(written_id(&w), a.id);
    assert!(!text.contains(&b.id), "a record holds one profile");
    let back = read_record(&record_file_name(&a.id), text).unwrap();
    assert_eq!(back.values.name, "A2");
    assert_eq!(back.modified, 10, "modified is the caller's time");
}

#[test]
fn ac4_record_file_name_is_the_id_dot_json() {
    let i = id(9);
    assert_eq!(record_file_name(&i), format!("{i}.json"));
    assert_eq!(record_id(&record_file_name(&i)), Some(i.as_str()));
}

#[test]
fn adr0036_record_round_trips_byte_for_byte_and_follows_the_canonical_shape() {
    let p = user(1, "K40 in the garage");
    let text = write_record(&p);
    assert!(text.ends_with("}\n"), "trailing newline");
    assert!(!text.ends_with("\n\n"));
    assert!(
        text.starts_with("{\n  \"format_version\": 1,\n  \"id\": "),
        "{text}"
    );
    let order = [
        "\"format_version\"",
        "\"id\"",
        "\"kind\"",
        "\"modified\"",
        "\"name\"",
        "\"work_area_mm\"",
        "\"origin\"",
        "\"max_feed_mm_per_min\"",
        "\"max_power\"",
        "\"laser_mode\"",
        "\"connection\"",
    ];
    let mut at = 0;
    for key in order {
        let pos = text[at..]
            .find(key)
            .unwrap_or_else(|| panic!("{key} after {at}: {text}"));
        at += pos;
    }
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["kind"], "laser");
    assert_eq!(v["origin"], "top_right");
    assert_eq!(v["max_feed_mm_per_min"], 12_000);
    assert!(v["max_feed_mm_per_min"].is_u64(), "feed is an integer");
    assert_eq!(v["work_area_mm"]["width"].as_f64(), Some(600.0));
    assert_eq!(v["connection"]["baud_rate"], 250_000);
    let back = read_record(&record_file_name(&p.id), &text).unwrap();
    assert_eq!(back, p);
    assert_eq!(
        write_record(&back),
        text,
        "read, write, read is the identity"
    );
}

#[test]
fn adr0036_the_adr_example_record_reads() {
    let text = r#"{
  "format_version": 1,
  "id": "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f",
  "kind": "laser",
  "modified": 1762171200000,
  "name": "K40 in the garage",
  "work_area_mm": { "width": 400.0, "height": 300.0 },
  "origin": "bottom_left",
  "max_feed_mm_per_min": 6000,
  "max_power": 1000,
  "laser_mode": true,
  "connection": { "connector_address": "", "controller_port": "", "baud_rate": 115200 }
}
"#;
    let p = read_record("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f.json", text).unwrap();
    assert_eq!(p.values.name, "K40 in the garage");
    assert_eq!(p.modified, 1_762_171_200_000);
    assert_eq!(p.work_area, DocumentSize::from_mm(400.0, 300.0));
    assert_eq!(p.connection.baud_rate.get(), 115_200);
    // An integer width is a number too.
    let int = text.replace("400.0", "400");
    assert!(read_record("6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f.json", &int).is_ok());
}

#[test]
fn adr0036_new_ids_are_lowercase_uuid_v4() {
    for seed in [[0_u8; 16], [0xff; 16], [0x5a; 16]] {
        let i = new_profile_id(seed);
        assert_eq!(i.len(), 36);
        assert!(is_profile_id(&i), "{i}");
        assert_eq!(i, i.to_lowercase());
        let b = i.as_bytes();
        assert_eq!(b[14], b'4', "version 4: {i}");
        assert!(matches!(b[19], b'8' | b'9' | b'a' | b'b'), "variant: {i}");
        for dash in [8, 13, 18, 23] {
            assert_eq!(b[dash], b'-');
        }
    }
    assert_ne!(id(1), id(2));
    assert!(
        !is_profile_id("6F1C2A4E-9B3D-4C1A-8E2F-0A1B2C3D4E5F"),
        "uppercase is no id"
    );
    assert!(
        !is_profile_id("6f1c2a4e-9b3d-1c1a-8e2f-0a1b2c3d4e5f"),
        "not v4"
    );
    assert!(!is_profile_id(""));
    assert!(!is_profile_id("../../etc/passwd"));
}

// ---- criterion 6 and 15: the copy in the project file ----------------------------------------

#[test]
fn ac6_the_copy_holds_id_and_copied_values_and_no_connection_or_work_area() {
    let p = user(1, "K40");
    let map = write_copy(&ProfileCopy::of(&p));
    let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "id",
            "laser_mode",
            "max_feed_mm_per_min",
            "max_power",
            "name",
            "origin"
        ]
    );
    assert_eq!(map["id"], json!(p.id));
    assert_eq!(map["name"], "K40");
    assert_eq!(map["origin"], "top_right");
    assert_eq!(map["max_feed_mm_per_min"], 12_000);
    assert!(map["max_feed_mm_per_min"].is_i64() || map["max_feed_mm_per_min"].is_u64());
    assert_eq!(map["max_power"], 255);
    assert_eq!(map["laser_mode"], false);
    let text = Value::Object(map.clone()).to_string();
    for secret in [
        "192.168",
        "ttyUSB",
        "250000",
        "connection",
        "baud",
        "width",
        "height",
    ] {
        assert!(
            !text.contains(secret),
            "{secret} leaked into the copy: {text}"
        );
    }
    let back = read_copy(Some(&map), None);
    assert!(!back.name_damaged);
    assert_eq!(back.copy, ProfileCopy::of(&p));
}

#[test]
fn ac6_copy_round_trips_through_write_and_read_for_every_corner() {
    for corner in OriginCorner::ALL {
        let mut p = user(3, "Corner");
        p.values.origin = corner;
        let map = write_copy(&ProfileCopy::of(&p));
        assert_eq!(read_copy(Some(&map), None).copy.values.origin, corner);
        assert_eq!(OriginCorner::from_name(corner.name()), Some(corner));
    }
    let names: Vec<&str> = OriginCorner::ALL.iter().map(|c| c.name()).collect();
    assert_eq!(
        names,
        vec!["top_left", "top_right", "bottom_left", "bottom_right"]
    );
}

// ---- criterion 7: Differs ----------------------------------------------------------------------

#[test]
fn ac7_a_copy_equal_to_its_record_does_not_differ_and_each_field_makes_it_differ() {
    let p = user(1, "K40");
    let page = p.work_area;
    let copy = ProfileCopy::of(&p);
    assert!(!differs(&copy, page, &p));
    let mut c = copy.clone();
    c.values.name = "K41".to_owned();
    assert!(differs(&c, page, &p), "the name counts");
    let mut c = copy.clone();
    c.values.origin = OriginCorner::BottomLeft;
    assert!(differs(&c, page, &p));
    let mut c = copy.clone();
    c.values.max_feed = FeedRate::new(12_001).unwrap();
    assert!(differs(&c, page, &p));
    let mut c = copy.clone();
    c.values.max_power = PowerValue::new(256).unwrap();
    assert!(differs(&c, page, &p));
    let mut c = copy.clone();
    c.values.laser_mode = true;
    assert!(differs(&c, page, &p));
}

#[test]
fn ac7_page_size_differs_only_beyond_a_hundredth_of_a_millimetre() {
    let p = user(1, "K40");
    let copy = ProfileCopy::of(&p);
    assert!(!differs(&copy, DocumentSize::from_mm(600.005, 400.0), &p));
    assert!(!differs(&copy, DocumentSize::from_mm(600.0, 399.995), &p));
    assert!(differs(&copy, DocumentSize::from_mm(600.02, 400.0), &p));
    assert!(differs(&copy, DocumentSize::from_mm(600.0, 400.5), &p));
    assert!(
        differs(&copy, DocumentSize::from_mm(400.0, 600.0), &p),
        "rotated is different"
    );
}

#[test]
fn ac7_the_connection_is_not_part_of_the_comparison() {
    let p = user(1, "K40");
    let copy = ProfileCopy::of(&p);
    let mut q = p.clone();
    q.connection.controller_port = "COM9".to_owned();
    q.modified += 1000;
    assert!(!differs(&copy, p.work_area, &q));
}

#[test]
fn ac7_update_overwrites_values_and_work_area_and_keeps_id_and_connection() {
    let p = user(1, "K40");
    let mut lib = MachineProfiles::load(&[entry(&p)]);
    let mut v = p.values.clone();
    v.max_feed = FeedRate::new(3_000).unwrap();
    v.name = "K40 tuned".to_owned();
    let w = lib
        .update(&p.id, v.clone(), DocumentSize::from_mm(500.0, 250.0), 42)
        .unwrap();
    assert_eq!(written_id(&w), p.id);
    let after = lib.find(&p.id).unwrap();
    assert_eq!(after.values, v);
    assert_eq!(after.work_area, DocumentSize::from_mm(500.0, 250.0));
    assert_eq!(after.connection, p.connection);
    assert_eq!(after.modified, 42);
    assert_eq!(
        read_record(&record_file_name(&p.id), written_text(&w)).unwrap(),
        *after
    );
    assert_eq!(
        lib.update(&id(50), v, DocumentSize::from_mm(1.0, 1.0), 1),
        None
    );
}

// ---- criterion 8: a copy the library does not have ---------------------------------------------

#[test]
fn ac8_a_foreign_id_reads_and_is_kept_and_save_as_adds_it_under_the_same_id() {
    let foreign = user(200, "Their laser");
    let map = write_copy(&ProfileCopy::of(&foreign));
    let read = read_copy(Some(&map), None);
    assert_eq!(read.copy.id.as_deref(), Some(foreign.id.as_str()));
    let mut lib = MachineProfiles::load(&[]);
    assert!(lib.find(&foreign.id).is_none());
    let mut saved = foreign.clone();
    saved.connection = MachineProfile::generic_laser().connection;
    let w = lib.add(saved.clone()).unwrap();
    assert_eq!(written_id(&w), foreign.id);
    assert!(lib.is_user(&foreign.id));
    assert!(!differs(
        &read.copy,
        foreign.work_area,
        lib.find(&foreign.id).unwrap()
    ));
}

#[test]
fn ac8_a_missing_or_non_string_id_reads_as_no_record() {
    let mut map = write_copy(&ProfileCopy::of(&user(1, "K40")));
    map.remove("id");
    assert_eq!(read_copy(Some(&map), None).copy.id, None);
    map.insert("id".to_owned(), json!(17));
    assert_eq!(read_copy(Some(&map), None).copy.id, None);
    map.insert("id".to_owned(), json!(["x"]));
    assert_eq!(read_copy(Some(&map), None).copy.id, None);
}

// ---- criterion 9: connection ----------------------------------------------------------------------

#[test]
fn ac9_connection_defaults_and_text_limits() {
    let c = MachineProfile::generic_laser().connection;
    assert_eq!(c.connector_address, "");
    assert_eq!(c.controller_port, "");
    assert_eq!(c.baud_rate.get(), 115_200);
    let long = "1".repeat(250);
    let a = clamp_address(&long);
    assert_eq!(a.value.chars().count(), 200);
    assert_eq!(
        a.notice.as_deref(),
        Some("Address limited to 200 characters.")
    );
    let p = clamp_port(&long);
    assert_eq!(p.value.chars().count(), 200);
    assert_eq!(p.notice.as_deref(), Some("Port limited to 200 characters."));
    assert_eq!(clamp_port("/dev/ttyUSB0").notice, None);
    assert_eq!(clamp_address("").value, "", "empty is allowed");
}

#[test]
fn ac9_set_connection_writes_the_record_and_nothing_else_changes() {
    let p = user(1, "K40");
    let mut lib = MachineProfiles::load(&[entry(&p)]);
    let mut c = p.connection.clone();
    c.controller_port = "COM3".to_owned();
    let w = lib.set_connection(&p.id, c.clone(), 77).unwrap();
    let back = read_record(&record_file_name(&p.id), written_text(&w)).unwrap();
    assert_eq!(back.connection, c);
    assert_eq!(back.values, p.values);
    assert_eq!(
        lib.set_connection(&p.id, c, 78),
        None,
        "unchanged writes nothing"
    );
}

#[test]
fn ac9_a_record_holds_no_secret_field() {
    let text = write_record(&user(1, "K40"));
    let lower = text.to_lowercase();
    for word in ["password", "secret", "token", "key\"", "pairing"] {
        assert!(!lower.contains(word), "{word} in {text}");
    }
}

// ---- criterion 10: last pick ----------------------------------------------------------------------

#[test]
fn ac10_last_pick_round_trips_and_a_damaged_or_newer_file_reads_as_none() {
    let i = id(4);
    let text = write_choice(&i);
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["format_version"], 1);
    assert_eq!(v["last_picked"], json!(i));
    assert_eq!(read_choice(&text), Some(i.clone()));
    assert_eq!(
        read_choice(&write_choice(GENERIC_LASER_ID)).as_deref(),
        Some(GENERIC_LASER_ID)
    );
    assert_eq!(read_choice(""), None);
    assert_eq!(read_choice("not json"), None);
    assert_eq!(
        read_choice(r#"{"format_version": 2, "last_picked": "x"}"#),
        None
    );
    assert_eq!(
        read_choice(r#"{"format_version": 1, "last_picked": 5}"#),
        None
    );
}

// ---- criterion 12: damaged copies and the format-35 reading ------------------------------------

#[test]
fn ac12_each_damaged_field_reads_as_its_own_default_and_the_others_stay() {
    let p = user(1, "K40");
    let good = write_copy(&ProfileCopy::of(&p));
    let cases: Vec<(&str, Value)> = vec![
        ("max_feed_mm_per_min", Value::Null),
        ("max_feed_mm_per_min", json!("fast")),
        ("max_feed_mm_per_min", json!(1.5)),
        ("max_feed_mm_per_min", json!(0)),
        ("max_feed_mm_per_min", json!(100_001)),
        ("max_feed_mm_per_min", json!(-6000)),
        ("max_power", json!("1000")),
        ("max_power", json!(65_536)),
        ("max_power", json!(true)),
        ("origin", json!("middle")),
        ("origin", json!(3)),
        ("laser_mode", json!("yes")),
        ("laser_mode", json!(1)),
    ];
    for (key, bad) in cases {
        let mut map = good.clone();
        map.insert(key.to_owned(), bad.clone());
        let before = map.clone();
        let r = read_copy(Some(&map), None);
        assert_eq!(map, before, "read never rewrites");
        let v = &r.copy.values;
        match key {
            "max_feed_mm_per_min" => {
                assert_eq!(v.max_feed, FeedRate::DEFAULT, "{bad}");
                assert_eq!(v.max_power, p.values.max_power);
            }
            "max_power" => {
                assert_eq!(v.max_power, PowerValue::DEFAULT, "{bad}");
                assert_eq!(v.max_feed, p.values.max_feed);
            }
            "origin" => assert_eq!(v.origin, OriginCorner::BottomLeft, "{bad}"),
            "laser_mode" => assert!(v.laser_mode, "{bad}"),
            _ => unreachable!(),
        }
        assert_eq!(v.name, "K40");
        assert!(!r.name_damaged);
        assert_eq!(r.copy.id.as_deref(), Some(p.id.as_str()));
    }
}

#[test]
fn ac12_an_integral_float_is_accepted_as_a_whole_number() {
    let mut map = write_copy(&ProfileCopy::of(&user(1, "K40")));
    map.insert("max_feed_mm_per_min".to_owned(), json!(7000.0));
    map.insert("max_power".to_owned(), json!(500.0));
    let r = read_copy(Some(&map), None);
    assert_eq!(r.copy.values.max_feed.get(), 7_000);
    assert_eq!(r.copy.values.max_power.get(), 500);
}

#[test]
fn ac12_a_damaged_name_reads_generic_laser_with_the_notice() {
    let good = write_copy(&ProfileCopy::of(&user(1, "K40")));
    for bad in [
        json!(""),
        json!("   "),
        json!("x".repeat(61)),
        json!(42),
        Value::Null,
    ] {
        let mut map = good.clone();
        map.insert("name".to_owned(), bad.clone());
        let r = read_copy(Some(&map), None);
        assert_eq!(r.copy.values.name, "Generic laser", "{bad}");
        assert!(r.name_damaged, "{bad}");
    }
    let mut map = good.clone();
    map.remove("name");
    assert!(
        read_copy(Some(&map), None).name_damaged,
        "a missing name is damage too"
    );
    assert_eq!(
        DAMAGED_NAME_NOTICE,
        "Name was damaged, showing Generic laser"
    );
    assert!(DAMAGED_NAME_NOTICE.chars().count() <= NOTICE_MAX);
    let mut map = good;
    map.insert("name".to_owned(), json!("x".repeat(60)));
    assert!(!read_copy(Some(&map), None).name_damaged, "60 is valid");
}

#[test]
fn ac12_an_empty_map_reads_as_all_defaults() {
    let map = serde_json::Map::new();
    let r = read_copy(Some(&map), None);
    assert_eq!(r.copy.id, None);
    assert_eq!(r.copy.values, MachineProfile::generic_laser().values);
}

#[test]
fn ac12_unknown_keys_in_the_copy_are_ignored_not_damage() {
    let mut map = write_copy(&ProfileCopy::of(&user(1, "K40")));
    map.insert("future_field".to_owned(), json!({"a": 1}));
    let r = read_copy(Some(&map), None);
    assert_eq!(r.copy.values.name, "K40");
    assert!(!r.name_damaged);
}

#[test]
fn ac12_a_format_35_manufacturing_reads_as_generic_laser_with_its_name() {
    let r = read_copy(None, Some("Generic laser"));
    assert_eq!(r.copy.id.as_deref(), Some(GENERIC_LASER_ID));
    assert_eq!(r.copy.values, MachineProfile::generic_laser().values);
    assert!(!r.name_damaged);
    let r = read_copy(None, Some("My old K40"));
    assert_eq!(r.copy.id.as_deref(), Some(GENERIC_LASER_ID));
    assert_eq!(r.copy.values.name, "My old K40");
    assert_eq!(r.copy.values.max_feed, FeedRate::DEFAULT);
    let r = read_copy(None, None);
    assert_eq!(r.copy.values.name, "Generic laser");
    let r = read_copy(None, Some(&"y".repeat(61)));
    assert_eq!(
        r.copy.values.name, "Generic laser",
        "an invalid legacy name is not used"
    );
}

// ---- criterion 13: the per-record load --------------------------------------------------------

#[test]
fn ac13_every_good_record_loads_and_each_bad_one_is_skipped_and_counted() {
    let good_a = user(1, "Good A");
    let good_b = user(2, "Good B");
    let base = user(10, "Bad");
    let bad = |n: u8, edit: &dyn Fn(&mut serde_json::Map<String, Value>)| {
        let mut p = base.clone();
        p.id = id(n);
        (record_file_name(&p.id), record_with(&p, |m| edit(m)))
    };
    let mismatch = {
        let p = user(30, "Mismatch");
        (record_file_name(&id(31)), write_record(&p))
    };
    let entries = vec![
        entry(&good_a),
        (record_file_name(&id(11)), "{ not json".to_owned()),
        (record_file_name(&id(12)), String::new()),
        bad(13, &|m| {
            m.insert("max_power".into(), json!("lots"));
        }),
        bad(14, &|m| {
            m.insert("max_feed_mm_per_min".into(), json!(200_000));
        }),
        bad(15, &|m| {
            m.insert("kind".into(), json!("cnc"));
        }),
        bad(16, &|m| {
            m.insert("extra".into(), json!(1));
        }),
        bad(17, &|m| {
            m.insert("name".into(), json!(""));
        }),
        bad(18, &|m| {
            m.insert(
                "work_area_mm".into(),
                json!({"width": 0.5, "height": 300.0}),
            );
        }),
        bad(19, &|m| {
            m.insert(
                "connection".into(),
                json!({"connector_address": "", "controller_port": "", "baud_rate": 0}),
            );
        }),
        bad(20, &|m| {
            m.insert("origin".into(), json!("center"));
        }),
        bad(21, &|m| {
            m.remove("laser_mode");
        }),
        bad(22, &|m| {
            m.insert("max_feed_mm_per_min".into(), json!(6000.5));
        }),
        mismatch,
        entry(&good_b),
    ];
    let lib = MachineProfiles::load(&entries);
    assert!(lib.find(&good_a.id).is_some());
    assert!(lib.find(&good_b.id).is_some());
    for n in 11..=22 {
        assert!(lib.find(&id(n)).is_none(), "record {n} must be skipped");
    }
    assert!(lib.find(&id(30)).is_none() && lib.find(&id(31)).is_none());
    assert_eq!(lib.listed().len(), 3);
    let notices = lib.load_notices();
    assert_eq!(notices.len(), 1, "{notices:?}");
    assert!(
        notices[0].starts_with("13 profiles could not be read"),
        "{notices:?}"
    );
    assert!(notices[0].chars().count() <= NOTICE_MAX);
}

#[test]
fn ac13_one_damaged_record_uses_the_singular_notice() {
    let lib = MachineProfiles::load(&[(record_file_name(&id(1)), "]".to_owned())]);
    assert_eq!(
        lib.load_notices(),
        vec!["1 profile could not be read. It was left as it is.".to_owned()]
    );
    assert!(MachineProfiles::load(&[]).load_notices().is_empty());
}

#[test]
fn ac13_a_newer_record_is_skipped_with_its_own_count() {
    let p = user(1, "From the future");
    let newer = record_with(&p, |m| {
        m.insert("format_version".into(), json!(2));
        m.insert("new_thing".into(), json!(true));
    });
    assert_eq!(
        read_record(&record_file_name(&p.id), &newer),
        Err(RecordError::Newer)
    );
    let lib = MachineProfiles::load(&[(record_file_name(&p.id), newer)]);
    assert!(lib.find(&p.id).is_none());
    let notices = lib.load_notices();
    assert_eq!(notices.len(), 1);
    assert!(
        notices[0].starts_with("1 profile is from a newer version"),
        "{notices:?}"
    );
    assert!(notices[0].chars().count() <= NOTICE_MAX);
}

#[test]
fn ac13_format_version_zero_or_missing_is_damage_not_newer() {
    let p = user(1, "Old");
    for edit in [
        Box::new(|m: &mut serde_json::Map<String, Value>| {
            m.insert("format_version".into(), json!(0));
        }) as Box<dyn Fn(&mut serde_json::Map<String, Value>)>,
        Box::new(|m: &mut serde_json::Map<String, Value>| {
            m.remove("format_version");
        }),
        Box::new(|m: &mut serde_json::Map<String, Value>| {
            m.insert("format_version".into(), json!("1"));
        }),
    ] {
        let text = record_with(&p, |m| edit(m));
        assert_eq!(
            read_record(&record_file_name(&p.id), &text),
            Err(RecordError::Damaged)
        );
    }
}

#[test]
fn ac13_the_size_limit_is_16_kib_inclusive() {
    let p = user(1, "Padded");
    let text = write_record(&p);
    assert_eq!(MAX_RECORD_BYTES, 16 * 1024);
    let at_limit = format!("{text}{}", " ".repeat(MAX_RECORD_BYTES - text.len()));
    assert_eq!(at_limit.len(), MAX_RECORD_BYTES);
    assert!(
        read_record(&record_file_name(&p.id), &at_limit).is_ok(),
        "exactly 16 KiB reads"
    );
    let over = format!("{at_limit} ");
    assert_eq!(
        read_record(&record_file_name(&p.id), &over),
        Err(RecordError::Damaged)
    );
    let huge = "x".repeat(10 * 1024 * 1024);
    assert_eq!(
        read_record(&record_file_name(&p.id), &huge),
        Err(RecordError::Damaged)
    );
}

#[test]
fn ac13_files_not_named_uuid_json_are_ignored_and_not_counted() {
    let p = user(0xab, "Real");
    assert_ne!(p.id, p.id.to_uppercase(), "the id has letters");
    let text = write_record(&p);
    let upper = format!("{}.json", p.id.to_uppercase());
    let entries = vec![
        ("notes.json".to_owned(), text.clone()),
        (format!("{}.json.bak", p.id), text.clone()),
        (format!("{} (conflict).json", p.id), text.clone()),
        (upper, text.clone()),
        (p.id.clone(), text.clone()),
        ("generic-laser.json".to_owned(), text.clone()),
        (".json".to_owned(), "{".to_owned()),
    ];
    let lib = MachineProfiles::load(&entries);
    assert_eq!(lib.listed().len(), 1, "only the built-in");
    assert!(
        lib.load_notices().is_empty(),
        "ignored files are not skipped records"
    );
    assert_eq!(record_id("notes.json"), None);
    assert_eq!(record_id(&format!("{}.JSON", p.id)), None);
}

#[test]
fn ac13_the_rest_of_the_library_stays_editable_after_a_skip() {
    let good = user(1, "Good");
    let mut lib =
        MachineProfiles::load(&[entry(&good), (record_file_name(&id(2)), "nope".to_owned())]);
    assert!(lib.rename(&good.id, "Still good".to_owned(), 1).is_some());
    assert!(lib.duplicate(&good.id, id(3), 2).is_some());
    assert!(lib.add(user(4, "Added")).is_some());
}

#[test]
#[ignore = "FINDING 0026 AC13: MachineProfiles::add writes over the id of a skipped newer-format record (ADR 0004 §9)"]
fn ac13_a_skipped_newer_record_is_never_overwritten_by_save_as_profile() {
    // ADR 0004 §9 / ADR 0036 §3: a newer record is never written. A project made on the newer
    // computer carries that id, reads "Not in your profiles" here, and Save as profile adds the
    // copy "under the same id" (criterion 8) -- which must not overwrite the newer file.
    let p = user(1, "From the future");
    let newer = record_with(&p, |m| {
        m.insert("format_version".into(), json!(2));
    });
    let mut lib = MachineProfiles::load(&[(record_file_name(&p.id), newer)]);
    let write = lib.add(p.clone());
    assert!(
        write.as_ref().is_none_or(|w| written_id(w) != p.id),
        "the newer record {} must stay untouched, got {write:?}",
        p.id
    );
}

#[test]
#[ignore = "FINDING 0026 AC13: MachineProfiles::add writes over the id of a skipped damaged record"]
fn ac13_a_skipped_damaged_record_is_never_overwritten_by_save_as_profile() {
    let p = user(1, "Damaged on disk");
    let mut lib = MachineProfiles::load(&[(record_file_name(&p.id), "{ broken".to_owned())]);
    let write = lib.add(p.clone());
    assert!(
        write.as_ref().is_none_or(|w| written_id(w) != p.id),
        "the skipped file {} must stay untouched, got {write:?}",
        p.id
    );
}

#[test]
fn ac13_at_most_1000_records_are_read_in_file_name_order() {
    let mut entries: Vec<(String, String)> = (0..1_005_u32)
        .map(|n| {
            let mut seed = [7_u8; 16];
            seed[12..].copy_from_slice(&n.to_be_bytes());
            let mut p = user(0, &format!("P{n}"));
            p.id = new_profile_id(seed);
            entry(&p)
        })
        .collect();
    entries.reverse();
    let lib = MachineProfiles::load(&entries);
    assert_eq!(lib.listed().len(), 1 + 1_000);
    let mut sorted: Vec<&String> = entries.iter().map(|(n, _)| n).collect();
    sorted.sort();
    let last_loaded = record_id(sorted[999]).unwrap();
    let first_dropped = record_id(sorted[1000]).unwrap();
    assert!(lib.find(last_loaded).is_some());
    assert!(lib.find(first_dropped).is_none());
}

#[test]
fn ac13_a_record_named_after_another_id_is_damage() {
    let p = user(1, "A");
    assert_eq!(
        read_record(&record_file_name(&id(2)), &write_record(&p)),
        Err(RecordError::Damaged)
    );
}

// ---- criterion 15 ---------------------------------------------------------------------------------

#[test]
fn ac15_a_record_holds_no_project_content() {
    let v: Value = serde_json::from_str(&write_record(&user(1, "K40"))).unwrap();
    let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "connection",
            "format_version",
            "id",
            "kind",
            "laser_mode",
            "max_feed_mm_per_min",
            "max_power",
            "modified",
            "name",
            "origin",
            "work_area_mm"
        ]
    );
}
