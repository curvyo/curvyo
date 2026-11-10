//! Independent acceptance tests for `specs/0026-machine-profile` at the document model and file
//! format (ADR 0039, format 36): the atomic `mfg_profile` register (criteria 5, 6), one undo step
//! per write and none for an unchanged one, the copy never holding a connection (6, 15), the
//! golden round trip and the damaged goldens (12), the format-35 migration with no write on open,
//! and refusal of a newer file or of a register that is not a map of plain values.
//!
//! Updated for `specs/0098-page-kinds` (its "Changes to accepted specs" table): Add manufacturing
//! is Add page with Kind Laser (`0026` 10 → `0098` 1, 4), a pick keeps the content's distance to
//! the copy's origin corner (`0026` 5 b → `0098` 14), the format-36 goldens are frozen and open
//! through the page-kinds migration, and placements come from older files only.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::assigning_clones,
    clippy::assert_is_empty
)]

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use curvyo_document_core::{
    CURRENT_FORMAT_VERSION, Document, DocumentSize, FixtureParams, NewOutputPage, NodeId,
    OpenError, PageCorner, Point, ProfileChange, RectBounds, pack, unpack,
};
use loro::{LoroDoc, LoroValue, TreeParentId};
use serde_json::{Value, json};

const USER_ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn copy(id: &str, name: &str, feed: i64) -> FixtureParams {
    json!({
        "id": id,
        "name": name,
        "origin": "top_right",
        "max_feed_mm_per_min": feed,
        "max_power": 255,
        "laser_mode": false,
    })
    .as_object()
    .unwrap()
    .clone()
}

fn generic() -> FixtureParams {
    json!({
        "id": "generic-laser",
        "name": "Generic laser",
        "origin": "bottom_left",
        "max_feed_mm_per_min": 6000,
        "max_power": 1000,
        "laser_mode": true,
    })
    .as_object()
    .unwrap()
    .clone()
}

fn laser(profile: Option<&FixtureParams>) -> NewOutputPage<'_> {
    NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile,
        work_area: DocumentSize::from_mm(400.0, 300.0),
    }
}

/// Add page with Kind Laser; the manufacturing of the new page.
fn add(d: &Document, new: NewOutputPage<'_>) -> NodeId {
    d.add_output_page(new)
        .unwrap()
        .kind
        .manufacturing()
        .unwrap()
}

/// The bottom-left origin corner of the Generic laser copy.
const BOTTOM_LEFT: PageCorner = PageCorner {
    right: false,
    bottom: true,
};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn current(stem: &str) -> Vec<u8> {
    // The goldens of this slice's format 36, frozen since format 37 (`0098`).
    std::fs::read(fixture(&format!("{stem}_v36.curvyo")))
        .unwrap_or_else(|e| panic!("golden {stem}_v36.curvyo: {e}"))
}

fn member(bytes: &[u8], name: &str) -> Vec<u8> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = Vec::new();
    archive
        .by_name(name)
        .unwrap()
        .read_to_end(&mut out)
        .unwrap();
    out
}

fn manifest_version(bytes: &[u8]) -> u64 {
    let v: Value = serde_json::from_slice(&member(bytes, "manifest.json")).unwrap();
    v["format_version"].as_u64().unwrap()
}

fn stored_loro(bytes: &[u8]) -> LoroDoc {
    let loro = LoroDoc::new();
    loro.import(&member(bytes, "document.loro")).unwrap();
    loro
}

fn container(loro: &LoroDoc, version: u64) -> Vec<u8> {
    let manifest = json!({
        "format_version": version,
        "loro_snapshot_version": curvyo_document_core::CURRENT_LORO_SNAPSHOT_VERSION,
        "app_version": "0.1.0",
    });
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default();
    writer.start_file("manifest.json", options).unwrap();
    writer.write_all(manifest.to_string().as_bytes()).unwrap();
    writer.start_file("document.loro", options).unwrap();
    writer
        .write_all(&loro.export(loro::ExportMode::Snapshot).unwrap())
        .unwrap();
    writer.start_file("document.json", options).unwrap();
    writer.write_all(b"{}").unwrap();
    writer.finish().unwrap().into_inner()
}

fn manufacturing_meta(loro: &LoroDoc) -> loro::LoroMap {
    let tree = loro.get_tree("paths");
    let node = tree
        .children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .expect("a manufacturing node");
    tree.get_meta(node).unwrap()
}

fn meta_value(loro: &LoroDoc, key: &str) -> Option<LoroValue> {
    manufacturing_meta(loro)
        .get(key)
        .map(|v| v.get_deep_value())
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn undo_last(d: &Document) {
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
}

fn with_placed_rect() -> (Document, NodeId) {
    let d = Document::new(1);
    let _ = d.create_rect(RectBounds::from_corners(
        Point::new(10.0, 20.0),
        Point::new(50.0, 40.0),
    ));
    let first = d.pages()[0].id.clone();
    let g = generic();
    let id = add(&d, laser(Some(&g)));
    // A placement as an older file holds it (`0098` criterion 12).
    let loro = LoroDoc::new();
    loro.import(&d.export_loro_snapshot().unwrap()).unwrap();
    let tree = loro.get_tree("paths");
    let node = tree
        .children(TreeParentId::Root)
        .unwrap()
        .into_iter()
        .find(|&n| tree.get_meta(n).unwrap().get("manufacturing").is_some())
        .unwrap();
    let child = tree.create(TreeParentId::Node(node)).unwrap();
    let meta = tree.get_meta(child).unwrap();
    meta.insert("shape", "placement").unwrap();
    let identity: Vec<LoroValue> = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0]
        .into_iter()
        .map(LoroValue::Double)
        .collect();
    let link: std::collections::HashMap<String, LoroValue> = [
        ("page".to_owned(), LoroValue::from(first.as_str())),
        ("matrix".to_owned(), LoroValue::from(identity)),
    ]
    .into();
    meta.insert("placement_link", LoroValue::from(link))
        .unwrap();
    loro.commit();
    let d = unpack(1, &container(&loro, u64::from(CURRENT_FORMAT_VERSION))).unwrap();
    (d, id)
}

// ---- criterion 5: a pick is one step ---------------------------------------------------------

#[test]
fn ac5_a_pick_writes_copy_and_work_area_in_one_step_and_keeps_placed_content() {
    let (d, id) = with_placed_rect();
    let before = d.manufacturing(id).unwrap();
    let old_page = before.page.clone().unwrap();
    let placed_at = |m: &curvyo_document_core::ManufacturingSnapshot| {
        let page = m.page.as_ref().unwrap();
        let local = m.placements[0].matrix.apply_point(Point::new(0.0, 0.0));
        Point::new(page.origin.x + local.x, page.origin.y + local.y)
    };
    let global = placed_at(&before);
    let steps = d.own_step_count();
    let k40 = copy(USER_ID, "K40", 12_000);
    assert!(
        d.set_manufacturing_profile(
            id,
            &k40,
            DocumentSize::from_mm(600.0, 400.0),
            BOTTOM_LEFT,
            ProfileChange::Pick,
            &curvyo_document_core::PageContents::new(),
        )
        .unwrap()
    );
    assert_eq!(d.own_step_count(), steps + 1, "one undo step");
    assert_eq!(last_label(&d), "pick_profile");
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.profile.as_ref(), Some(&k40));
    assert_eq!(m.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(
        placed_at(&m),
        global,
        "content keeps its place in the plane"
    );
    let page = m.page.unwrap();
    assert_eq!(page.origin.x, old_page.origin.x);
    assert_eq!(
        page.origin.y + 400.0,
        old_page.origin.y + 300.0,
        "the bottom-left origin corner stays (`0098` criterion 14)"
    );
    undo_last(&d);
    let back = d.manufacturing(id).unwrap();
    assert_eq!(
        back.profile, before.profile,
        "undo restores the previous copy"
    );
    assert_eq!(back.work_area(), DocumentSize::from_mm(400.0, 300.0));
}

#[test]
fn ac5_picking_the_same_profile_writes_nothing_and_adds_no_step() {
    let (d, id) = with_placed_rect();
    let k40 = copy(USER_ID, "K40", 12_000);
    d.set_manufacturing_profile(
        id,
        &k40,
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner::default(),
        ProfileChange::Pick,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let steps = d.own_step_count();
    let json_before = d.export_json().unwrap();
    assert!(
        !d.set_manufacturing_profile(
            id,
            &k40,
            DocumentSize::from_mm(600.0, 400.0),
            PageCorner::default(),
            ProfileChange::Pick,
            &curvyo_document_core::PageContents::new(),
        )
        .unwrap()
    );
    assert_eq!(d.own_step_count(), steps);
    assert_eq!(d.export_json().unwrap(), json_before);
}

#[test]
fn ac5_each_profile_change_has_its_own_label() {
    let (d, id) = with_placed_rect();
    let area = DocumentSize::from_mm(400.0, 300.0);
    let cases = [
        (ProfileChange::Reset, "reset_profile"),
        (ProfileChange::SaveAs, "save_as_profile"),
        (ProfileChange::EditName, "edit_profile_name"),
        (ProfileChange::EditOrigin, "edit_origin_corner"),
        (ProfileChange::EditMaxFeed, "edit_max_feed"),
        (ProfileChange::EditMaxPower, "edit_max_power"),
        (ProfileChange::EditLaserMode, "edit_laser_mode"),
    ];
    for (n, (change, label)) in cases.into_iter().enumerate() {
        let c = copy(USER_ID, "K40", 1_000 + i64::try_from(n).unwrap());
        let steps = d.own_step_count();
        assert!(
            d.set_manufacturing_profile(
                id,
                &c,
                area,
                PageCorner::default(),
                change,
                &curvyo_document_core::PageContents::new()
            )
            .unwrap()
        );
        assert_eq!(d.own_step_count(), steps + 1);
        assert_eq!(last_label(&d), label);
    }
}

#[test]
fn ac5_an_invalid_work_area_or_unknown_id_writes_nothing() {
    let (d, id) = with_placed_rect();
    let steps = d.own_step_count();
    let before = d.manufacturing(id).unwrap();
    let c = copy(USER_ID, "K40", 7_000);
    for (w, h) in [
        (0.0, 300.0),
        (f64::NAN, 300.0),
        (400.0, f64::INFINITY),
        (-1.0, -1.0),
    ] {
        assert!(
            d.set_manufacturing_profile(
                id,
                &c,
                DocumentSize::from_mm(w, h),
                PageCorner::default(),
                ProfileChange::Pick,
                &curvyo_document_core::PageContents::new()
            )
            .is_err(),
            "{w} x {h}"
        );
    }
    let stray = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(1.0, 1.0),
    ));
    let steps_with_stray = d.own_step_count();
    assert!(
        d.set_manufacturing_profile(
            stray,
            &c,
            DocumentSize::from_mm(400.0, 300.0),
            PageCorner::default(),
            ProfileChange::Pick,
            &curvyo_document_core::PageContents::new(),
        )
        .is_err()
    );
    assert_eq!(d.own_step_count(), steps_with_stray);
    assert_eq!(steps + 1, steps_with_stray);
    assert_eq!(d.manufacturing(id).unwrap().profile, before.profile);
}

// ---- criterion 6 and 15: the atomic copy, no connection ----------------------------------------

#[test]
fn ac6_the_copy_is_one_register_with_whole_numbers_as_integers() {
    let (d, id) = with_placed_rect();
    let k40 = copy(USER_ID, "K40", 12_000);
    d.set_manufacturing_profile(
        id,
        &k40,
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner::default(),
        ProfileChange::Pick,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();
    let loro = stored_loro(&bytes);
    let stored = meta_value(&loro, "mfg_profile").expect("one mfg_profile register");
    let LoroValue::Map(map) = stored else {
        panic!("mfg_profile is a map: {stored:?}");
    };
    assert_eq!(
        map.get("id"),
        Some(&LoroValue::from(USER_ID)),
        "the id sits inside the copy"
    );
    assert!(
        matches!(map.get("max_feed_mm_per_min"), Some(LoroValue::I64(12_000))),
        "feed stored as I64: {map:?}"
    );
    assert!(matches!(map.get("max_power"), Some(LoroValue::I64(255))));
    assert!(
        meta_value(&loro, "mfg_profile_id").is_none(),
        "no second id register"
    );
    assert!(
        meta_value(&loro, "mfg_machine_name").is_none(),
        "format 36 writes no placeholder"
    );
    for k in map.keys() {
        assert!(
            !k.contains("width") && !k.contains("height"),
            "work area is the page: {k}"
        );
    }
}

#[test]
fn ac6_an_existing_manufacturing_does_not_change_when_nothing_writes_it() {
    let (d, id) = with_placed_rect();
    let k40 = copy(USER_ID, "K40", 12_000);
    d.set_manufacturing_profile(
        id,
        &k40,
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner::default(),
        ProfileChange::Pick,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let back = unpack(5, &pack(&d, "0.1.0").unwrap()).unwrap();
    assert_eq!(back.manufacturing(id).unwrap().profile.as_ref(), Some(&k40));
}

#[test]
fn ac15_no_connection_data_and_no_library_entry_in_the_saved_file() {
    let (d, id) = with_placed_rect();
    d.set_manufacturing_profile(
        id,
        &copy(USER_ID, "K40", 12_000),
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner::default(),
        ProfileChange::Pick,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let bytes = pack(&d, "0.1.0").unwrap();
    let json_member = String::from_utf8(member(&bytes, "document.json")).unwrap();
    for word in [
        "connector_address",
        "controller_port",
        "baud_rate",
        "connection",
        "machines/",
    ] {
        assert!(!json_member.contains(word), "{word} in document.json");
    }
    let loro_bytes = member(&bytes, "document.loro");
    for word in [
        b"connector_address".as_slice(),
        b"controller_port",
        b"baud_rate",
    ] {
        assert!(
            !loro_bytes.windows(word.len()).any(|w| w == word),
            "{} in document.loro",
            String::from_utf8_lossy(word)
        );
    }
}

// ---- criterion 12: golden round trip --------------------------------------------------------------

#[test]
fn ac12_the_machine_profile_golden_holds_id_edited_copy_and_600_by_400() {
    let bytes = current("machine_profile");
    assert_eq!(manifest_version(&bytes), 36);
    let d = unpack(7, &bytes).unwrap();
    let all = d.manufacturings();
    assert_eq!(all.len(), 1);
    let m = &all[0];
    assert_eq!(m.work_area(), DocumentSize::from_mm(600.0, 400.0));
    let profile = m.profile.as_ref().expect("the golden carries a copy");
    let id = profile["id"].as_str().unwrap();
    assert_ne!(id, "generic-laser", "a user profile id");
    assert_eq!(id.len(), 36);
    assert_eq!(&id[14..15], "4", "UUID v4: {id}");
    let generic = generic();
    let edited = [
        "name",
        "origin",
        "max_feed_mm_per_min",
        "max_power",
        "laser_mode",
    ]
    .iter()
    .any(|k| profile.get(*k) != generic.get(*k));
    assert!(
        edited,
        "the copy holds values other than the defaults: {profile:?}"
    );
    for key in profile.keys() {
        assert!(
            [
                "id",
                "name",
                "origin",
                "max_feed_mm_per_min",
                "max_power",
                "laser_mode"
            ]
            .contains(&key.as_str()),
            "unexpected key {key} in the golden copy"
        );
    }
    assert_eq!(m.legacy_machine_name, None);
    // Saved and reopened: identical.
    let again = pack(&d, "0.1.0").unwrap();
    let e = unpack(8, &again).unwrap();
    assert_eq!(e.manufacturings(), d.manufacturings());
    assert_eq!(
        meta_value(&stored_loro(&again), "mfg_profile"),
        meta_value(&stored_loro(&bytes), "mfg_profile")
    );
}

#[test]
fn ac12_an_edited_600_by_400_copy_round_trips_exactly() {
    let (d, id) = with_placed_rect();
    let c = copy(USER_ID, "K40 edited", 8_765);
    d.set_manufacturing_profile(
        id,
        &c,
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner::default(),
        ProfileChange::EditMaxFeed,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    let back = unpack(9, &pack(&d, "0.1.0").unwrap()).unwrap();
    let m = back.manufacturing(id).unwrap();
    assert_eq!(m.profile.as_ref(), Some(&c));
    assert_eq!(m.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(back.manufacturings(), d.manufacturings());
}

#[test]
fn ac12_the_damaged_golden_opens_and_saves_its_register_back_unchanged() {
    let bytes = current("machine_profile_damaged");
    let d = unpack(7, &bytes).expect("a damaged value inside the copy does not refuse the file");
    let m = &d.manufacturings()[0];
    let p = m.profile.as_ref().expect("the copy is read as it is");
    assert!(
        !p["max_feed_mm_per_min"].is_number()
            || !p["max_feed_mm_per_min"].as_f64().unwrap().is_finite(),
        "the feed is damaged: {p:?}"
    );
    assert!(p["max_power"].is_string(), "power is a string: {p:?}");
    assert_eq!(p["name"].as_str().unwrap().chars().count(), 61);
    let again = pack(&d, "0.1.0").unwrap();
    assert_eq!(
        meta_value(&stored_loro(&again), "mfg_profile"),
        meta_value(&stored_loro(&bytes), "mfg_profile"),
        "never rewritten on read"
    );
}

#[test]
fn ac12_the_not_a_map_golden_is_refused_as_damaged() {
    assert!(matches!(
        unpack(7, &current("mfg_profile_not_a_map")),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn ac12_an_mfg_profile_that_is_a_string_list_number_or_container_is_damage() {
    for bad in [
        LoroValue::from("K40"),
        LoroValue::I64(3),
        LoroValue::from(vec![LoroValue::from("id")]),
    ] {
        let loro = stored_loro(&current("machine_profile"));
        manufacturing_meta(&loro)
            .insert("mfg_profile", bad.clone())
            .unwrap();
        loro.commit();
        assert!(
            matches!(
                unpack(2, &container(&loro, u64::from(CURRENT_FORMAT_VERSION))),
                Err(OpenError::Damaged)
            ),
            "{bad:?}"
        );
    }
    // A live container instead of a plain map value is not a map of plain values.
    let loro = stored_loro(&current("machine_profile"));
    manufacturing_meta(&loro)
        .insert_container("mfg_profile", loro::LoroMap::new())
        .unwrap();
    loro.commit();
    assert!(matches!(
        unpack(2, &container(&loro, u64::from(CURRENT_FORMAT_VERSION))),
        Err(OpenError::Damaged)
    ));
}

#[test]
fn ac12_a_map_with_damaged_plain_values_opens() {
    let loro = stored_loro(&current("machine_profile"));
    let damaged: std::collections::HashMap<String, LoroValue> = [
        ("id".to_owned(), LoroValue::I64(5)),
        ("name".to_owned(), LoroValue::from("")),
        (
            "max_feed_mm_per_min".to_owned(),
            LoroValue::Double(f64::INFINITY),
        ),
        ("max_power".to_owned(), LoroValue::Null),
        ("origin".to_owned(), LoroValue::from("nowhere")),
        ("laser_mode".to_owned(), LoroValue::from("on")),
    ]
    .into();
    manufacturing_meta(&loro)
        .insert("mfg_profile", LoroValue::from(damaged))
        .unwrap();
    loro.commit();
    let d = unpack(2, &container(&loro, u64::from(CURRENT_FORMAT_VERSION)))
        .expect("plain damaged values read as defaults in the extension, the file opens");
    assert!(d.manufacturings()[0].profile.is_some());
}

#[test]
fn ac12_a_newer_format_is_refused() {
    let loro = stored_loro(&current("machine_profile"));
    let r = unpack(2, &container(&loro, u64::from(CURRENT_FORMAT_VERSION) + 1));
    assert!(
        matches!(r, Err(OpenError::FormatTooNew { .. })),
        "{:?}",
        r.err()
    );
}

// ---- criterion 12: format 35 migration ------------------------------------------------------------

#[test]
fn ac12_a_format_35_file_opens_with_its_placeholder_name_and_no_copy() {
    let bytes = std::fs::read(fixture("laser_workbench_v35.curvyo")).unwrap();
    assert_eq!(manifest_version(&bytes), 35);
    let d = unpack(7, &bytes).unwrap();
    let m = &d.manufacturings()[0];
    assert_eq!(m.profile, None, "no copy is invented on open");
    assert!(
        m.legacy_machine_name.is_some(),
        "the placeholder name is read"
    );
    assert_eq!(d.own_step_count(), 0, "opening writes nothing");
    // Saved without an edit, the old key stays and no copy appears.
    let again = pack(&d, "0.1.0").unwrap();
    let loro = stored_loro(&again);
    assert_eq!(
        meta_value(&loro, "mfg_machine_name"),
        meta_value(&stored_loro(&bytes), "mfg_machine_name")
    );
    assert!(meta_value(&loro, "mfg_profile").is_none());
}

#[test]
fn ac12_the_first_profile_write_on_a_format_35_file_stores_the_copy_and_drops_the_old_key() {
    let bytes = std::fs::read(fixture("laser_workbench_v35.curvyo")).unwrap();
    let d = unpack(7, &bytes).unwrap();
    let m = d.manufacturings()[0].clone();
    let steps = d.own_step_count();
    let mut g = generic();
    g.insert(
        "name".to_owned(),
        json!(m.legacy_machine_name.clone().unwrap()),
    );
    // Same values, same size: still a write, because the old key goes away (ADR 0039 §4).
    let changed = d
        .set_manufacturing_profile(
            m.id,
            &g,
            m.work_area(),
            PageCorner::default(),
            ProfileChange::EditName,
            &curvyo_document_core::PageContents::new(),
        )
        .unwrap();
    assert!(changed);
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    let after = d.manufacturing(m.id).unwrap();
    assert_eq!(after.legacy_machine_name, None);
    assert_eq!(after.profile.as_ref(), Some(&g));
    let loro = stored_loro(&pack(&d, "0.1.0").unwrap());
    assert!(meta_value(&loro, "mfg_machine_name").is_none());
    undo_last(&d);
    let undone = d.manufacturing(m.id).unwrap();
    assert_eq!(
        undone.legacy_machine_name, m.legacy_machine_name,
        "undo brings the old key back"
    );
    assert_eq!(undone.profile, None);
}

#[test]
fn ac12_the_unknown_process_v35_golden_still_opens_and_keeps_its_registers() {
    let bytes = std::fs::read(fixture("manufacturing_unknown_process_v35.curvyo")).unwrap();
    let d = unpack(7, &bytes).unwrap();
    let again = pack(&d, "0.1.0").unwrap();
    let a = format!(
        "{:?}",
        manufacturing_meta(&stored_loro(&bytes)).get_deep_value()
    );
    let b = format!(
        "{:?}",
        manufacturing_meta(&stored_loro(&again)).get_deep_value()
    );
    assert_eq!(a, b);
}

#[test]
fn ac10_add_laser_page_writes_the_given_copy_in_the_same_single_step() {
    let d = Document::new(1);
    let k40 = copy(USER_ID, "K40", 12_000);
    let steps = d.own_step_count();
    let id = add(
        &d,
        NewOutputPage {
            work_area: DocumentSize::from_mm(600.0, 400.0),
            ..laser(Some(&k40))
        },
    );
    assert_eq!(d.own_step_count(), steps + 1);
    let m = d.manufacturing(id).unwrap();
    assert_eq!(m.profile.as_ref(), Some(&k40));
    assert_eq!(m.work_area(), DocumentSize::from_mm(600.0, 400.0));
    assert_eq!(m.legacy_machine_name, None);
}

#[test]
fn adr0037_two_manufacturings_keep_independent_copies() {
    let d = Document::new(1);
    let g = generic();
    let a = add(&d, laser(Some(&g)));
    let b = add(&d, laser(Some(&g)));
    let k40 = copy(USER_ID, "K40", 12_000);
    d.set_manufacturing_profile(
        a,
        &k40,
        DocumentSize::from_mm(600.0, 400.0),
        PageCorner::default(),
        ProfileChange::Pick,
        &curvyo_document_core::PageContents::new(),
    )
    .unwrap();
    assert_eq!(
        d.manufacturing(b).unwrap().profile.as_ref(),
        Some(&generic())
    );
    assert_eq!(
        d.manufacturing(b).unwrap().work_area(),
        DocumentSize::from_mm(400.0, 300.0)
    );
}
