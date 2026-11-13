//! The laser extension reads the format 43 goldens of `curvyo-document-core/tests/fixtures/`
//! (`specs/0028-material-test-library` criteria 26, 28; ADR 0054 §1, §2, §6): every operation's
//! settings state and every damage. Reading old grids is `specs/0112` Part F.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;

use curvyo_document_core::{Document, OperationSnapshot, unpack};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{
    Field, Role, SettingsDamage, SettingsRead, SettingsState, StoredReason, read_settings, resolve,
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

fn read(op: &OperationSnapshot) -> (OperationKind, SettingsRead) {
    let kind = KindRead::of(op.kind.as_deref()).known().unwrap();
    (kind, read_settings(op.settings.as_ref(), kind))
}

#[test]
fn the_settings_golden_resolves_stored_own_and_no_settings_without_the_library() {
    let d = golden("laser_settings_v43.curvyo");
    let ops = operations(&d);
    let state = |at: usize| {
        let (kind, settings) = read(&ops[at]);
        resolve(&settings, kind, &[], "generic-laser")
    };
    let SettingsState::Stored {
        name,
        reason,
        material,
        run,
        overridden,
    } = state(0)
    else {
        panic!("the material is not on this computer");
    };
    assert_eq!(
        (name.as_str(), reason),
        ("Birch 3 mm", StoredReason::Missing)
    );
    assert_eq!(material.main.power.to_string(), "30");
    assert_eq!(run.main.power.to_string(), "35", "the override runs");
    assert_eq!(
        run.outline.unwrap().speed.get(),
        9000,
        "the stored outline set"
    );
    assert_eq!(overridden, [(Role::Main, Field::Power)]);
    assert!(state(0).differs_text().is_some());
    let cut = state(1);
    assert_eq!(cut.run().unwrap().main.passes.get(), 2);
    assert_eq!(cut.differs_text(), None);
    assert!(matches!(state(2), SettingsState::Own(_)));
    assert_eq!(state(3), SettingsState::NoSettings);
}

#[test]
fn every_damaged_register_reads_as_damaged_settings_with_its_reason() {
    use SettingsDamage::{BadLink, BadOverride, BadSet, NotAMap, OutlineOnKind};
    let d = golden("laser_settings_damaged_v43.curvyo");
    let damages: Vec<SettingsDamage> = operations(&d)
        .iter()
        .map(|op| match read(op).1 {
            SettingsRead::Damaged(damage) => damage,
            other => panic!("{}: {other:?}", op.name),
        })
        .collect();
    assert_eq!(
        damages,
        [
            NotAMap,
            BadSet,
            BadSet,
            BadLink,
            BadLink,
            OutlineOnKind,
            BadOverride,
            BadOverride,
            BadOverride
        ]
    );
}
