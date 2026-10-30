//! Independent snapping persistence and history acceptance tests.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, DocumentSnapping, pack, unpack};
#[test]
fn ac1_ac14_defaults_roundtrip_and_reads_do_not_create_operations() {
    let d = Document::new(601);
    let version = d.version();
    assert_eq!(d.snapping(), DocumentSnapping::DEFAULT);
    assert_eq!(d.version(), version);
    let stored = DocumentSnapping {
        x: true,
        y: true,
        enabled: true,
        objects: false,
        guides: true,
        nodes: false,
        page: true,
        capture_px: 0.25,
        release_px: 1234.5,
    };
    assert!(d.set_snapping(stored));
    let restored = unpack(602, &pack(&d, "acceptance").unwrap()).unwrap();
    let version = restored.version();
    assert_eq!(restored.snapping(), stored);
    assert_eq!(restored.snapping(), stored);
    assert_eq!(restored.version(), version);
    assert_eq!(Document::new(603).snapping(), DocumentSnapping::DEFAULT);
}
#[test]
fn ac10_settings_reject_nonfinite_and_negative_values_without_history_or_writes() {
    let d = Document::new(604);
    let version = d.version();
    let history = d.history_len();
    for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for release in [false, true] {
            let mut value = DocumentSnapping::DEFAULT;
            if release {
                value.release_px = invalid;
            } else {
                value.capture_px = invalid;
            }
            assert!(!d.set_snapping(value));
            assert_eq!(d.version(), version);
            assert_eq!(d.history_len(), history);
        }
    }
    assert!(d.set_snapping(DocumentSnapping {
        capture_px: 0.0,
        release_px: 0.0,
        ..DocumentSnapping::DEFAULT
    }));
    assert!(d.set_snapping(DocumentSnapping {
        capture_px: 1024.25,
        release_px: 0.5,
        ..DocumentSnapping::DEFAULT
    }));
    assert!((d.snapping().release_px - 1024.25).abs() < 1e-9);
}
#[test]
fn ac1_ac12_peer_settings_merge_independently_and_undo_preserves_peer_fields() {
    let a = Document::new(605);
    let b = unpack(606, &pack(&a, "acceptance").unwrap()).unwrap();
    a.set_snapping(DocumentSnapping {
        enabled: true,
        ..a.snapping()
    });
    let step = a.own_steps().last().unwrap().step;
    b.set_snapping(DocumentSnapping {
        nodes: false,
        ..b.snapping()
    });
    let av = a.version();
    let bv = b.version();
    let au = a.export_updates(&bv).unwrap();
    let bu = b.export_updates(&av).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
    assert_eq!(a.snapping(), b.snapping());
    assert!(a.snapping().enabled);
    assert!(!a.snapping().nodes);
    a.undo_step(step).unwrap();
    assert!(!a.snapping().enabled);
    assert!(!a.snapping().nodes);
}

#[test]
fn ac16_axis_peers_merge_and_local_undo_retains_other_axis() {
    let a = Document::new(630);
    let b = unpack(631, &pack(&a, "acceptance").unwrap()).unwrap();
    assert!(a.snapping().x && a.snapping().y);
    a.set_snapping(DocumentSnapping {
        x: false,
        ..a.snapping()
    });
    let step = a.own_steps().last().unwrap().step;
    b.set_snapping(DocumentSnapping {
        y: false,
        ..b.snapping()
    });
    let au = a.export_updates(&b.version()).unwrap();
    let bu = b.export_updates(&a.version()).unwrap();
    a.import_updates(&bu).unwrap();
    b.import_updates(&au).unwrap();
    assert_eq!(a.snapping(), b.snapping());
    assert!(!a.snapping().x && !a.snapping().y && !a.snapping().enabled);
    a.undo_step(step).unwrap();
    assert!(a.snapping().x && !a.snapping().y);
    let reopened = unpack(632, &pack(&a, "acceptance").unwrap()).unwrap();
    assert_eq!(reopened.snapping(), a.snapping());
}
