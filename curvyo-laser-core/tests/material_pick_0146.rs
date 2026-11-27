//! The laser extension reads the format 65 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0146-material-test-picks` criteria 13 to 15, 17; ADR 0093 decisions 7 to 9): a link
//! follows the pick it stores, absent means Best, and `"best"`, an unknown word and a number are
//! Damaged settings (a bad link).

#![allow(clippy::unwrap_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, unpack};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{PickKind, SettingsDamage, SettingsRead, read_settings};

fn golden(name: &str) -> Document {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../curvyo-document-core/tests/fixtures")
        .join(name);
    unpack(5, &std::fs::read(path).unwrap()).unwrap()
}

fn reads(name: &str) -> Vec<(String, SettingsRead)> {
    let d = golden(name);
    let ops: Vec<OperationSnapshot> = d.operations(d.manufacturings()[0].id).unwrap();
    ops.into_iter()
        .map(|op| {
            let kind = KindRead::of(op.kind.as_deref()).known().unwrap();
            assert_eq!(kind, OperationKind::Cut);
            let read = read_settings(
                op.settings.as_ref(),
                kind,
                curvyo_laser_core::DefaultNumbers::BUILT_IN,
            );
            (op.name, read)
        })
        .collect()
}

#[test]
fn each_link_follows_the_pick_it_stores_and_a_missing_key_means_best() {
    let picks: Vec<(String, PickKind, bool)> = reads("material_pick_v66.curvyo")
        .into_iter()
        .map(|(name, read)| {
            let SettingsRead::Stored(stored) = read else {
                panic!("{name} reads");
            };
            (
                name,
                stored.material.as_ref().unwrap().pick,
                !stored.overrides.is_empty(),
            )
        })
        .collect();
    assert_eq!(
        picks,
        [
            ("Cut Best".to_owned(), PickKind::Best, false),
            ("Cut Second".to_owned(), PickKind::SecondBest, false),
            ("Cut Fastest".to_owned(), PickKind::Fastest, false),
            ("Cut Deepest".to_owned(), PickKind::Deepest, true),
        ]
    );
}

#[test]
fn a_bad_pick_is_damaged_settings() {
    for (name, read) in reads("material_pick_damaged_v66.curvyo") {
        assert_eq!(
            read,
            SettingsRead::Damaged(SettingsDamage::BadLink),
            "{name}"
        );
    }
}
