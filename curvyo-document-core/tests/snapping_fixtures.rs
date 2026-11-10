//! The Fixtures snap target switch of `specs/0102-laser-fixtures` criterion 24 (ADR 0049 §5): the
//! root register `snapping_fixtures`, absent reads as on, written only when switched, one step
//! labelled `set_document_snapping`, and nothing written on open.

#![allow(clippy::unwrap_used)]

use curvyo_document_core::{Document, DocumentSnapping, pack, unpack};
use loro::{LoroDoc, LoroValue};

fn root_value(document: &Document, key: &str) -> Option<LoroValue> {
    let loro = LoroDoc::new();
    loro.import(&document.export_loro_snapshot().unwrap())
        .unwrap();
    loro.get_map("root").get(key).map(|v| v.get_deep_value())
}

#[test]
fn a_file_without_the_register_reads_fixtures_on_and_open_writes_nothing() {
    let d = Document::new(1);
    assert!(d.snapping().fixtures);
    assert_eq!(
        DocumentSnapping::default(),
        DocumentSnapping {
            fixtures: true,
            ..DocumentSnapping::DEFAULT
        }
    );
    assert_eq!(root_value(&d, "snapping_fixtures"), None);
    let opened = unpack(2, &pack(&d, "f").unwrap()).unwrap();
    assert!(opened.snapping().fixtures);
    assert_eq!(root_value(&opened, "snapping_fixtures"), None);
}

#[test]
fn switching_fixtures_off_writes_one_register_in_one_step_and_undo_restores_it() {
    let d = Document::new(1);
    let steps = d.own_step_count();
    let off = DocumentSnapping {
        fixtures: false,
        ..d.snapping()
    };
    assert!(d.set_snapping(off));
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(
        root_value(&d, "snapping_fixtures"),
        Some(LoroValue::Bool(false))
    );
    assert_eq!(root_value(&d, "snapping_objects"), None, "nothing else");
    assert!(!d.snapping().fixtures);
    assert!(!d.set_snapping(off), "a repeat writes nothing");
    let reopened = unpack(2, &pack(&d, "f").unwrap()).unwrap();
    assert!(!reopened.snapping().fixtures);
}
