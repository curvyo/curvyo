//! The laser extension reads the format 44 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0111-laser-operation-tools` criteria 9, 14, 19 to 22, 25; ADR 0056 §1, §2, §5 to §9):
//! every operation's options and interval, every damage, the profile copies' motorised Z axis, the
//! group's ticks, and the clone-member keys of format 39 that run nothing.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, unpack};
use curvyo_laser_core::operations::{
    ChainScope, GroupTick, KindRead, OperationKind, OperationList, group_assign_keys, group_tick,
    last_of_kind,
};
use curvyo_laser_core::read_copy;
use curvyo_laser_core::settings::{
    OptionsDamage, SettingsDamage, SettingsRead, read_options, read_settings, resolve,
};

fn golden(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(5, &std::fs::read(path).unwrap()).unwrap()
}

fn operations(d: &Document, at: usize) -> Vec<OperationSnapshot> {
    d.operations(d.manufacturings()[at].id).unwrap()
}

fn kind(op: &OperationSnapshot) -> OperationKind {
    KindRead::of(op.kind.as_deref()).known().unwrap()
}

#[test]
fn the_golden_reads_every_option_interval_and_motorised_z_axis() {
    let d = golden("operation_tools_v44.curvyo");
    let ops = operations(&d, 0);
    let engrave = read_options(ops[0].options.as_ref(), kind(&ops[0])).unwrap();
    assert!(engrave.crosshatch && !engrave.bidirectional);
    assert_eq!(engrave.scan_angle.degrees(), 15.0);
    assert_eq!(engrave.overscan.percent(), 10);
    assert_eq!(engrave.z_offset.mm(), -1.5);
    let interval = |at: usize| {
        let read = read_settings(ops[at].settings.as_ref(), kind(&ops[at]));
        resolve(&read, kind(&ops[at]), &[], "generic-laser")
            .run()
            .unwrap()
            .interval
            .unwrap()
            .mm()
    };
    assert_eq!(interval(0), 0.08);
    assert_eq!(
        interval(1),
        0.12,
        "a linked operation keeps its own interval"
    );
    let defaults = read_options(ops[1].options.as_ref(), kind(&ops[1])).unwrap();
    assert!(defaults.bidirectional && !defaults.crosshatch);
    let cut = read_options(ops[2].options.as_ref(), kind(&ops[2])).unwrap();
    let copies: Vec<bool> = d
        .manufacturings()
        .iter()
        .map(|m| read_copy(m.profile.as_ref(), None).copy.values.motorised_z)
        .collect();
    assert_eq!(copies, [false, true]);
    assert_eq!(cut.running_z_offset(copies[0]), None, "kept, runs nothing");
    assert_eq!(cut.z_offset.mm(), 1.5);
    let second = operations(&d, 1);
    let z = read_options(second[0].options.as_ref(), kind(&second[0])).unwrap();
    assert_eq!(z.running_z_offset(copies[1]).unwrap().mm(), 2.0);
    assert_eq!(last_of_kind(&ops, OperationKind::Engrave), Some(0));
    assert_eq!(last_of_kind(&ops, OperationKind::Score), None);
}

#[test]
fn the_golden_group_runs_whole_in_the_cut_and_b_also_in_the_engrave() {
    let d = golden("operation_tools_v44.curvyo");
    let ops = operations(&d, 0);
    let b = ops[0].members[0][0];
    let group = ops[2]
        .members
        .iter()
        .map(|path| path[0])
        .find(|&id| id != b)
        .unwrap();
    let list = OperationList::new(&ops, false);
    // A has no key of its own, so its chain decides at the group.
    let chains = [vec![vec![group]], vec![vec![b], vec![group]]];
    assert_eq!(group_tick(&chains, &list, 2), GroupTick::Checked);
    assert_eq!(group_tick(&chains, &list, 0), GroupTick::Mixed);
    assert_eq!(group_tick(&chains, &list, 1), GroupTick::Unchecked);
    assert_eq!(group_assign_keys(group, &[b], &list, 2), []);
    assert_eq!(group_assign_keys(group, &[b], &list, 0), [group]);
}

#[test]
fn every_damaged_option_and_interval_reads_as_damaged_settings() {
    let d = golden("operation_options_damaged_v44.curvyo");
    let ops = operations(&d, 0);
    let options: Vec<Option<OptionsDamage>> = ops
        .iter()
        .map(|op| read_options(op.options.as_ref(), kind(op)).err())
        .collect();
    let settings: Vec<Option<SettingsDamage>> = ops
        .iter()
        .map(|op| match read_settings(op.settings.as_ref(), kind(op)) {
            SettingsRead::Damaged(damage) => Some(damage),
            _ => None,
        })
        .collect();
    assert_eq!(
        options,
        [
            Some(OptionsDamage::NotAMap),
            Some(OptionsDamage::UnknownKey),
            Some(OptionsDamage::BadValue),
            Some(OptionsDamage::BadValue),
            Some(OptionsDamage::HatchOnKind),
            None,
            None,
        ]
    );
    assert_eq!(
        settings,
        [
            None,
            None,
            None,
            None,
            None,
            Some(SettingsDamage::BadInterval),
            Some(SettingsDamage::BadInterval),
        ]
    );
    assert_eq!(
        last_of_kind(&ops, OperationKind::Engrave),
        None,
        "a damaged operation is never reused by a kind press"
    );
}

#[test]
fn clone_member_keys_of_format_39_run_nothing_and_the_member_runs_with_the_clone() {
    let d = golden("operations_clone_member_v39.curvyo");
    let ops = operations(&d, 0);
    let path = ops[0].members[0].clone();
    assert_eq!(path.len(), 2, "the legacy key is read");
    let list = OperationList::new(&ops, false);
    assert_eq!(list.operations_of(&path), &[] as &[usize]);
    let chain = [path.clone(), vec![path[0]]];
    assert_eq!(
        list.nearest(&chain, ChainScope::Whole),
        None,
        "the clone itself holds no key: Not assigned"
    );
}
