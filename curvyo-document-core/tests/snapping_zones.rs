//! Stored snapping zone boundaries.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Document, DocumentSnapping};

#[test]
fn zones_accept_zero_fractional_and_large_finite_values() {
    let d = Document::new(1);
    for capture in [0.0, 0.25, 1000.0] {
        assert!(d.set_snapping(DocumentSnapping {
            capture_px: capture,
            release_px: 0.0,
            ..DocumentSnapping::DEFAULT
        }));
        assert_eq!(d.snapping().capture_px, capture);
        assert_eq!(d.snapping().release_px, capture);
    }
    for invalid in [-0.1, f64::INFINITY, f64::NAN] {
        assert!(!d.set_snapping(DocumentSnapping {
            capture_px: invalid,
            ..DocumentSnapping::DEFAULT
        }));
    }
}

#[test]
fn axis_switches_default_on_and_round_trip_independently() {
    let d = Document::new(16);
    assert!(d.snapping().x && d.snapping().y);
    assert!(d.set_snapping(DocumentSnapping {
        x: false,
        ..d.snapping()
    }));
    assert!(!d.snapping().x && d.snapping().y);
    let reopened =
        curvyo_document_core::unpack(17, &curvyo_document_core::pack(&d, "test").unwrap()).unwrap();
    assert_eq!(reopened.snapping(), d.snapping());
}
