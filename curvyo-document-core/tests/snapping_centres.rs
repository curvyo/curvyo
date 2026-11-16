//! The Centres snap target switch of `specs/0113-fixture-sets-and-pins` criterion 30 (amended
//! 2026-11-16): the root register `snapping_centres`, absent reads as on, written only when
//! switched, one step labelled `set_document_snapping`, nothing written on open, and a damaged
//! value refused on open like every snapping register.

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
fn a_file_without_the_register_reads_centres_on_and_open_writes_nothing() {
    let d = Document::new(1);
    assert!(d.snapping().centres);
    assert_eq!(DocumentSnapping::default(), DocumentSnapping::DEFAULT);
    assert_eq!(root_value(&d, "snapping_centres"), None);
    let opened = unpack(2, &pack(&d, "c").unwrap()).unwrap();
    assert!(opened.snapping().centres);
    assert_eq!(root_value(&opened, "snapping_centres"), None);
}

#[test]
fn switching_centres_off_writes_one_register_in_one_step_and_survives_a_reopen() {
    let d = Document::new(1);
    let steps = d.own_step_count();
    let off = DocumentSnapping {
        centres: false,
        ..d.snapping()
    };
    assert!(d.set_snapping(off));
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(
        root_value(&d, "snapping_centres"),
        Some(LoroValue::Bool(false))
    );
    assert_eq!(root_value(&d, "snapping_objects"), None, "nothing else");
    assert!(!d.snapping().centres);
    assert!(d.snapping().objects, "Objects is a separate target");
    assert!(!d.set_snapping(off), "a repeat writes nothing");
    let reopened = unpack(2, &pack(&d, "c").unwrap()).unwrap();
    assert!(!reopened.snapping().centres);
}
