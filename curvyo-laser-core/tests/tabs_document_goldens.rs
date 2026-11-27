//! The laser extension reads the format 66 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0139-laser-cut-path-options` criteria 1, 5, 6, 11; adrs L3, L6): the tab options,
//! every own tabs record and every damage.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, member_key_text, unpack};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{OptionsDamage, read_options};
use curvyo_laser_core::tabs::{LeafTabs, RecordDamage, read_own_tabs};

fn golden(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(5, &std::fs::read(path).unwrap()).unwrap()
}

fn operations(d: &Document) -> Vec<OperationSnapshot> {
    d.operations(d.manufacturings()[0].id).unwrap()
}

fn kind(op: &OperationSnapshot) -> OperationKind {
    KindRead::of(op.kind.as_deref()).known().unwrap()
}

#[test]
fn the_golden_reads_the_tab_options_and_every_own_tabs_record() {
    let d = golden("laser_path_options_v66.curvyo");
    let ops = operations(&d);
    let cut = read_options(ops[0].options.as_ref(), kind(&ops[0]))
        .unwrap()
        .cut;
    assert_eq!(cut.tab_count.get(), 4);
    assert_eq!(cut.tab_width.hundredths(), 200);
    assert!(cut.tab_skip_inner);
    let records = d.operation_tabs(ops[0].id).unwrap();
    let item = &records[&member_key_text(&ops[0].members[0])];
    let own = read_own_tabs(item).unwrap();
    let tabs = own.get("0").unwrap();
    assert_eq!(tabs.len(), 2);
    assert_eq!(
        tabs[0]
            .width
            .map(curvyo_laser_core::settings::CutLength::hundredths),
        Some(150)
    );
    assert_eq!(tabs[1].width, None, "the operation's width");
    // The group's record names its member's outline: that leaf has no tabs, another leaf follows
    // the operation.
    let group = &records[&member_key_text(&ops[0].members[1])];
    let reference = group.as_object().unwrap().keys().next().unwrap().clone();
    let (below, _) = reference.rsplit_once(':').unwrap();
    let leaf = LeafTabs::of(Some(group), &[below.to_owned()], 1);
    assert_eq!(leaf.own(0), Some(&[][..]));
    assert_eq!(
        LeafTabs::of(Some(group), &["1:1".to_owned()], 1),
        LeafTabs::Operation
    );
    let cut_out = read_options(ops[1].options.as_ref(), kind(&ops[1]))
        .unwrap()
        .cut;
    assert_eq!(cut_out.tab_count.get(), 2);
}

#[test]
fn every_damaged_record_and_option_reads_as_damage() {
    let d = golden("laser_path_options_damaged_v66.curvyo");
    let ops = operations(&d);
    let expected = [
        RecordDamage::NotAMap,
        RecordDamage::BadFraction,
        RecordDamage::BadWidth,
        RecordDamage::UnknownField,
        RecordDamage::Overlap,
    ];
    for (op, damage) in ops.iter().zip(expected) {
        let records = d.operation_tabs(op.id).unwrap();
        let record = records.values().next().unwrap();
        assert_eq!(read_own_tabs(record), Err(damage), "{}", op.name);
        assert_eq!(LeafTabs::of(Some(record), &[], 1), LeafTabs::Damaged);
    }
    let by_name = |name: &str| ops.iter().find(|o| o.name == name).unwrap();
    let options = by_name("Damaged options");
    assert_eq!(
        read_options(options.options.as_ref(), kind(options)),
        Err(OptionsDamage::BadValue)
    );
    let scored = by_name("Damaged kind");
    assert_eq!(
        read_options(scored.options.as_ref(), kind(scored)),
        Err(OptionsDamage::KeyOnKind)
    );
    let planted = by_name("Damaged 1");
    let records = d.operation_tabs(planted.id).unwrap();
    assert_eq!(
        LeafTabs::of(records.values().next(), &[], 1),
        LeafTabs::Damaged
    );
}
