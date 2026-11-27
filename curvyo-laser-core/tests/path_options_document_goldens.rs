//! The laser extension reads the format 68 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0139-laser-cut-path-options` criteria 13, 19 to 22; adrs L2, L3, L6): the path
//! options, every own path options and entry record and every damage.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, ItemRecord, OperationSnapshot, member_key_text, unpack};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{LeadKind, OptionsDamage, read_options, read_own_path};
use curvyo_laser_core::tabs::{
    Direction, LeafPath, OutlineEntry, PathFraction, RecordDamage, read_entries,
};

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
fn the_golden_reads_the_path_options_and_every_record() {
    let d = golden("laser_path_options_v68.curvyo");
    let ops = operations(&d);
    let options = read_options(ops[0].options.as_ref(), kind(&ops[0])).unwrap();
    assert_eq!(options.path.lead_in, LeadKind::Arc);
    assert!((options.path.lead_in_length.mm() - 2.0).abs() < 1e-9);
    assert_eq!(options.path.lead_out, LeadKind::Line);
    assert!((options.path.overcut.mm() - 1.5).abs() < 1e-9);
    let item = member_key_text(&ops[0].members[0]);
    let own = d.operation_records(ops[0].id, ItemRecord::PathOwn).unwrap();
    let own = read_own_path(&own[&item]).unwrap();
    assert_eq!(own.lead_in, LeadKind::Line);
    let entries = d.operation_records(ops[0].id, ItemRecord::Entry).unwrap();
    let start = read_entries(&entries[&item]).unwrap();
    assert_eq!(
        start.get("0"),
        Some(OutlineEntry::Start(PathFraction::new(250_000_000).unwrap()))
    );
    let line = entries
        .values()
        .map(|v| read_entries(v).unwrap())
        .find(|e| matches!(e.get("0"), Some(OutlineEntry::Direction(_))))
        .unwrap();
    assert_eq!(
        line.get("0"),
        Some(OutlineEntry::Direction(Direction::Reversed))
    );
    let cut_out = d.operation_records(ops[1].id, ItemRecord::Entry).unwrap();
    let path = LeafPath::of_contours(None, cut_out.values().next());
    assert!(path.entry(0).is_some() && !path.damaged);
}

#[test]
fn every_damaged_record_and_option_reads_as_damage() {
    let d = golden("laser_path_options_damaged_v68.curvyo");
    let ops = operations(&d);
    let expected = [
        (ItemRecord::PathOwn, RecordDamage::WrongType),
        (ItemRecord::PathOwn, RecordDamage::BadValue),
        (ItemRecord::PathOwn, RecordDamage::WrongType),
        (ItemRecord::Entry, RecordDamage::NotAMap),
        (ItemRecord::Entry, RecordDamage::BadFraction),
        (ItemRecord::Entry, RecordDamage::BadValue),
    ];
    for (op, (family, damage)) in ops.iter().zip(expected) {
        let records = d.operation_records(op.id, family).unwrap();
        let record = records.values().next().unwrap();
        let read = match family {
            ItemRecord::PathOwn => read_own_path(record).err(),
            _ => read_entries(record).err(),
        };
        assert_eq!(read, Some(damage), "{}", op.name);
        let path = match family {
            ItemRecord::PathOwn => LeafPath::of(Some(record), None, &[], 1),
            _ => LeafPath::of(None, Some(record), &[], 1),
        };
        assert!(path.damaged, "{}", op.name);
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
}
