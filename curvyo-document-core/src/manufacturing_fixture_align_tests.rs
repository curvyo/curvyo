//! Tests of the set mirror alignment (ADR 0059 §9 amended, §10; `specs/0113` criteria 44, 47).

#![allow(clippy::unwrap_used, clippy::float_cmp)]

use super::*;
use crate::{
    DocumentSize, FixtureJobSwitch, FixtureParams, JobRole, JobTarget, NewOutputPage, Vec2,
};

const HOLDER: &str = "curvyo.laser/holder";
const PIN: &str = "curvyo.laser/pin";

fn params(value: serde_json::Value) -> FixtureParams {
    serde_json::from_value(value).unwrap()
}

fn laser(d: &Document) -> NodeId {
    d.add_output_page(NewOutputPage {
        process: "curvyo.laser",
        label: "Laser",
        profile: None,
        work_area: DocumentSize::from_mm(600.0, 400.0),
    })
    .unwrap()
    .kind
    .manufacturing()
    .unwrap()
}

fn uid(n: u32) -> String {
    format!("00000000-0000-4000-8000-{n:012}")
}

fn holder(id: u32, name: &str, x: f64) -> FixtureRecord {
    FixtureRecord {
        id: uid(id),
        kind: HOLDER.to_owned(),
        name: name.to_owned(),
        locked: false,
        params: params(serde_json::json!({"width_mm": 100.0, "height_mm": 60.0})),
        matrix: Some(Affine::translation(Vec2::new(x, 10.0))),
        slots: vec![SlotRecord {
            name: "Slot 1".to_owned(),
            params: params(serde_json::json!({
                "shape": "circle", "cx_mm": 20.0, "cy_mm": 20.0, "w_mm": 0.0, "h_mm": 0.0,
                "d_mm": 10.0
            })),
        }],
        jobs: vec![JobRecord {
            id: uid(id + 500),
            kind: "engrave_outline".to_owned(),
            name: "Engrave 1".to_owned(),
            show: true,
            targets: vec![JobTarget {
                slot: Some(0),
                suffix: None,
            }],
            role: None,
        }],
    }
}

fn pin(id: u32) -> FixtureRecord {
    let jobs = crate::fixture_job_codec::role_job_records(PIN)
        .into_iter()
        .enumerate()
        .map(|(n, job)| JobRecord {
            id: uid(id + 500 + u32::try_from(n).unwrap()),
            ..job
        })
        .collect();
    FixtureRecord {
        id: uid(id),
        kind: PIN.to_owned(),
        name: "Pin 1".to_owned(),
        locked: true,
        params: params(serde_json::json!({"diameter_mm": 6.0})),
        matrix: Some(Affine::translation(Vec2::new(200.0, 50.0))),
        slots: Vec::new(),
        jobs,
    }
}

fn set(id: u32, name: &str, fixtures: Vec<FixtureRecord>) -> FixtureSetRecord {
    FixtureSetRecord {
        id: uid(id),
        name: name.to_owned(),
        locked: false,
        fixtures,
    }
}

fn record() -> Vec<FixtureSetRecord> {
    vec![
        set(1, "Bed", vec![holder(10, "Holder 1", 10.0), pin(11)]),
        set(2, "Coaster jig", vec![holder(20, "Holder 2", 300.0)]),
    ]
}

fn align(d: &Document, mfg: NodeId, sets: &[FixtureSetRecord], modified: i64) -> bool {
    d.align_fixture_sets(mfg, sets, modified, AlignCommit::Reserved)
        .unwrap()
}

#[test]
fn an_empty_mirror_takes_new_sets_off_this_page_and_their_fixtures_on_the_bed() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let steps = d.own_step_count();
    assert!(align(&d, mfg, &record(), 1_000));
    assert_eq!(d.own_step_count(), steps, "no step on the stacks");
    let read = d.manufacturing(mfg).unwrap();
    assert_eq!(read.sets_modified, 1_000);
    assert_eq!(read.sets.len(), 2);
    assert!(read.sets.iter().all(|set| !set.on_page), "Frage 218 A");
    assert!(read.sets.iter().all(|set| set.damaged == 0));
    assert!(read.fixtures.iter().all(|fixture| fixture.active));
    assert!(read.fixtures[1].locked);
    assert!(
        read.fixtures
            .iter()
            .flat_map(|f| &f.jobs)
            .all(|j| !j.output)
    );
    assert_eq!(read.set_records(), record(), "contents equal the record");
    let id = d.loro().oplog_frontiers().iter().next().unwrap();
    let message = d.loro().get_change(id).unwrap().message.unwrap();
    assert!(
        message.starts_with("align_fixture_sets;"),
        "the reserved label"
    );
    assert!(!align(&d, mfg, &record(), 1_000), "nothing to write");
}

#[test]
fn kept_nodes_keep_their_ids_and_the_page_choices_and_the_record_order_wins() {
    let d = Document::new(1);
    let mfg = laser(&d);
    align(&d, mfg, &record(), 1_000);
    let read = d.manufacturing(mfg).unwrap();
    let (bed, jig) = (read.sets[0].id, read.sets[1].id);
    let holder_node = read.fixtures[0].id;
    let job_node = read.fixtures[0].jobs[0].id;
    d.set_fixture_set_on_page(bed, true).unwrap();
    d.set_fixture_active(holder_node, false).unwrap();
    d.set_fixture_visible(holder_node, false).unwrap();
    d.set_fixture_job_switch(job_node, FixtureJobSwitch::Output, true)
        .unwrap();
    // Newer: the jig goes first and is renamed, the holder moves, the pin leaves, a set comes.
    let mut moved = holder(10, "Holder 1", 50.0);
    moved.slots.push(SlotRecord {
        name: "Slot 2".to_owned(),
        params: moved.slots[0].params.clone(),
    });
    moved.jobs[0].targets.push(JobTarget {
        slot: Some(1),
        suffix: None,
    });
    let newer = vec![
        set(2, "Jig", vec![holder(20, "Holder 2", 300.0)]),
        set(1, "Bed", vec![moved, holder(12, "Holder 3", 400.0)]),
        set(3, "Set 3", Vec::new()),
    ];
    assert!(align(&d, mfg, &newer, 2_000));
    let read = d.manufacturing(mfg).unwrap();
    assert_eq!(read.set_records(), newer);
    assert_eq!(read.sets[0].id, jig);
    assert_eq!(read.sets[1].id, bed, "the node id stays");
    assert!(read.sets[1].on_page, "On this page stays");
    assert!(!read.sets[2].on_page, "a new set is off this page");
    let kept = read.fixtures.iter().find(|f| f.id == holder_node).unwrap();
    assert!(!kept.active && !kept.visible, "On the bed and the eye stay");
    assert_eq!(kept.jobs[0].id, job_node);
    assert!(kept.jobs[0].output, "Output stays");
    assert_eq!(kept.jobs[0].targets.len(), 2, "ticks from the record");
    assert!(read.fixtures.iter().all(|f| f.kind != PIN), "the pin left");
    assert_eq!(read.sets_modified, 2_000);
}

#[test]
fn a_second_set_of_one_id_is_not_read_and_the_next_alignment_deletes_it() {
    let d = Document::new(1);
    let mfg = laser(&d);
    align(&d, mfg, &record(), 1_000);
    let twin = d.add_fixture_set(mfg, &uid(1)).unwrap();
    let read = d.manufacturing(mfg).unwrap();
    assert_eq!(read.sets.len(), 2, "the twin is kept and never read");
    assert!(read.sets.iter().all(|set| set.id != twin));
    assert!(align(&d, mfg, &record(), 1_000));
    assert_eq!(d.set_nodes(tree_id_of(mfg)).len(), 2, "the twin is deleted");
}

#[test]
fn a_set_without_its_profile_id_counts_one_damaged_value_and_fill_gives_every_node_one() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let set = d.add_fixture_set(mfg, "").unwrap();
    let new = crate::NewFixture {
        add: crate::FixtureAdd::Holder,
        kind: PIN,
        label: "Pin",
        params: &params(serde_json::json!({"diameter_mm": 6.0})),
        matrix: Some(Affine::translation(Vec2::new(10.0, 10.0))),
    };
    d.add_set_fixture(set, &new).unwrap();
    assert_eq!(d.manufacturing(mfg).unwrap().sets[0].damaged, 1);
    let mut n = 100;
    let mut next = || {
        n += 1;
        uid(n)
    };
    assert!(d.fill_fixture_set_uids(mfg, &mut next, "fill").unwrap());
    let read = d.manufacturing(mfg).unwrap();
    assert_eq!(read.sets[0].uid.as_deref(), Some(uid(101).as_str()));
    assert_eq!(read.sets[0].damaged, 0);
    assert_eq!(read.fixtures[0].uid.as_deref(), Some(uid(102).as_str()));
    let roles: Vec<_> = read.fixtures[0]
        .jobs
        .iter()
        .map(|j| (j.role, j.uid.is_some()))
        .collect();
    assert_eq!(
        roles,
        [
            (Some(JobRole::CentreMark), true),
            (Some(JobRole::Diameters), true)
        ]
    );
    assert!(!d.fill_fixture_set_uids(mfg, &mut next, "fill").unwrap());
}

#[test]
fn inside_a_batch_the_alignment_joins_the_batch_step() {
    let d = Document::new(1);
    let mfg = laser(&d);
    let other = laser(&d);
    let steps = d.own_step_count();
    d.batch("rename_fixture_set", |d| {
        d.align_fixture_sets(mfg, &record(), 5, AlignCommit::Step("rename_fixture_set"))
            .unwrap();
        d.align_fixture_sets(other, &record(), 5, AlignCommit::Step("rename_fixture_set"))
            .unwrap();
    });
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    assert_eq!(d.manufacturing(other).unwrap().sets.len(), 2);
    assert!(
        d.manufacturing(mfg)
            .unwrap()
            .job_fixtures()
            .next()
            .is_none()
    );
}
