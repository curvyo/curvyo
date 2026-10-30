//! Stored snapping zone boundaries.
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
