//! The alignment of a page's set mirror with a newer profile record (`specs/0113-fixture-sets-and-pins`
//! criteria 44, 47; ADR 0059 §9 amended, §10): the project golden `fixture_sets_align_v47.curvyo`
//! of `curvyo-document-core/tests/fixtures/` holds the mirror of `format5_fixture_sets.json` with
//! page choices; `format5_align_newer.json` is the same profile, newer. `CURVYO_WRITE_FIXTURES=1`
//! rewrites the project golden.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{
    AlignCommit, Document, FixtureJobSwitch, JobRole, ManufacturingSnapshot, NewOutputPage, NodeId,
    PageContents, PageCorner, ProfileChange, ProfileSets, pack, unpack,
};
use curvyo_laser_core::{
    MachineProfile, ProfileCopy, align_due, read_record, record_file_name, write_copy,
};

const ID: &str = "6f1c2a4e-9b3d-4c1a-8e2f-0a1b2c3d4e5f";

fn record(name: &str) -> MachineProfile {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/machine_profiles")
        .join(name);
    read_record(
        &record_file_name(ID),
        &std::fs::read_to_string(path).unwrap(),
    )
    .unwrap()
}

fn golden_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures/fixture_sets_align_v47.curvyo")
}

/// A laser page on the profile of `format5_fixture_sets.json`, picked, with page choices: Bed off
/// this page, Holder 1 off the bed and hidden, the pin's Diameters Output on.
fn golden_document() -> Document {
    let profile = record("format5_fixture_sets.json");
    let d = Document::new(1);
    let mfg = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: profile.work_area,
        })
        .unwrap()
        .kind
        .manufacturing()
        .unwrap();
    d.set_manufacturing_profile_and_sets(
        mfg,
        &write_copy(&ProfileCopy::of(&profile)),
        profile.work_area,
        PageCorner::default(),
        ProfileChange::Pick,
        ProfileSets {
            sets: &profile.fixture_sets,
            modified: profile.modified,
        },
        &PageContents::new(),
    )
    .unwrap();
    let read = d.manufacturing(mfg).unwrap();
    d.set_fixture_set_on_page(read.sets[0].id, false).unwrap();
    let holder = read.fixtures[1].id;
    d.set_fixture_active(holder, false).unwrap();
    d.set_fixture_visible(holder, false).unwrap();
    let pin = read
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("/pin"))
        .unwrap();
    let diameters = pin
        .jobs
        .iter()
        .find(|j| j.role == Some(JobRole::Diameters))
        .unwrap();
    d.set_fixture_job_switch(diameters.id, FixtureJobSwitch::Output, true)
        .unwrap();
    d
}

fn golden() -> Vec<u8> {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(golden_path(), pack(&golden_document(), "0.1.0").unwrap()).unwrap();
    }
    std::fs::read(golden_path()).unwrap()
}

fn page(d: &Document) -> ManufacturingSnapshot {
    d.manufacturings().remove(0)
}

fn node_of(read: &ManufacturingSnapshot, uid: &str) -> NodeId {
    read.fixtures
        .iter()
        .find(|f| f.uid.as_deref() == Some(uid))
        .map(|f| f.id)
        .unwrap()
}

#[test]
fn a_newer_profile_aligns_the_mirror_in_one_reserved_commit_and_keeps_the_page_choices() {
    let opened = unpack(2, &golden()).unwrap();
    let before = page(&opened);
    let older = record("format5_fixture_sets.json");
    assert_eq!(
        before.set_records(),
        older.fixture_sets,
        "the mirror of the record"
    );
    assert_eq!(before.sets_modified, older.modified);
    let newer = record("format5_align_newer.json");
    assert!(align_due(
        &before.set_records(),
        before.sets_modified,
        &newer
    ));
    let holder_uid = before.fixtures[1].uid.clone().unwrap();
    let holder = before.fixtures[1].id;
    let bed = before.sets[0].id;
    assert!(
        opened
            .align_fixture_sets(
                before.id,
                &newer.fixture_sets,
                newer.modified,
                AlignCommit::Reserved
            )
            .unwrap()
    );
    assert_eq!(opened.own_step_count(), 0, "no step on the stacks");
    let after = page(&opened);
    assert_eq!(after.set_records(), newer.fixture_sets);
    assert_eq!(after.sets_modified, newer.modified);
    assert_eq!(after.sets[0].id, bed, "a kept set keeps its node");
    assert!(!after.sets[0].on_page, "On this page stays off");
    assert_eq!(after.sets[1].name, "Jig");
    assert!(
        !after.sets[2].on_page,
        "the new set is off this page (Frage 218 A)"
    );
    let kept = after.fixtures.iter().find(|f| f.id == holder).unwrap();
    assert_eq!(node_of(&after, &holder_uid), holder);
    assert!(!kept.active && !kept.visible, "On the bed and the eye stay");
    assert_eq!(kept.matrix.unwrap().e, 60.0, "moved");
    let stop = after
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("/stop"))
        .unwrap();
    assert!(stop.active, "a new fixture is On the bed");
    let pin = after
        .fixtures
        .iter()
        .find(|f| f.kind.ends_with("/pin"))
        .unwrap();
    assert!(
        pin.jobs
            .iter()
            .any(|j| j.role == Some(JobRole::Diameters) && j.output),
        "Output stays"
    );
    assert!(!align_due(
        &after.set_records(),
        after.sets_modified,
        &newer
    ));
}

#[test]
fn a_profile_that_is_not_newer_writes_nothing() {
    let opened = unpack(2, &golden()).unwrap();
    let before = page(&opened);
    let mut newer = record("format5_align_newer.json");
    newer.modified = before.sets_modified;
    assert!(
        !align_due(&before.set_records(), before.sets_modified, &newer),
        "not newer: the project's sets are shown and nothing is written"
    );
    let same = record("format5_fixture_sets.json");
    let mut later = same.clone();
    later.modified += 1;
    assert!(
        !align_due(&before.set_records(), before.sets_modified, &later),
        "newer but equal in content"
    );
}
