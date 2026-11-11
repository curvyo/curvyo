//! Spiro reference case exercised in WebAssembly.
#![cfg(target_arch = "wasm32")]
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Point, Tolerance};
use curvyo_geometry_core::{SpiroControl, evaluate_spiro};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn open_corner_control_is_retained() {
    let points = [
        (0.0, 0.0, false),
        (1.0, 1.0, false),
        (2.0, 0.0, true),
        (3.0, -1.0, false),
        (4.0, 0.0, false),
    ];
    let controls = points.map(|(x, y, corner)| SpiroControl {
        point: Point::new(x, y),
        corner,
    });
    let spans = evaluate_spiro(&controls, false, Tolerance::from_mm(0.001)).unwrap();
    for control in controls {
        assert!(spans.iter().any(|span| {
            (span.start.x - control.point.x).hypot(span.start.y - control.point.y) < 0.001
                || (span.end.x - control.point.x).hypot(span.end.y - control.point.y) < 0.001
        }));
    }
}
