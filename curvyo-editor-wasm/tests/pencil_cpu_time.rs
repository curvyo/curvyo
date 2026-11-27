//! CPU budgets of the Pencil (`specs/0158-pen-display` slice 1 criteria 2 and 10, ADR 0095
//! decision 8), thread CPU time: with criterion 2's stroke (2000 samples on a 150 mm S-curve with a
//! hand's tremor and rising pressure), one preview frame at 2000 samples ≤ 8 ms and the release
//! (smoothing, fit, profile, commit) ≤ 200 ms in a release build
//! (`cargo test --release -p curvyo-editor-wasm --test pencil_cpu_time`). Debug builds get twelve
//! times the budget, as the other budget tests do.

#![allow(clippy::unwrap_used, missing_docs)]

mod cpu_time;

use std::time::Duration;

use cpu_time::cpu_time_of;
use curvyo_document_core::{Point, unpack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_ui_core::PEN_SAMPLE_STRIDE;

const fn budget(native_ms: u64) -> Duration {
    Duration::from_millis(if cfg!(debug_assertions) {
        12 * native_ms
    } else {
        native_ms
    })
}

/// Criterion 2's stroke as flat records for `session`: contact samples, then one hover.
fn s_curve(session: &Session) -> Vec<f64> {
    let origin = session.screen_to_document(0.0, 0.0);
    let unit = session.screen_to_document(1.0, 0.0);
    let px = 1.0 / (unit.x - origin.x);
    let raw = |t: f64| {
        Point::new(
            20.0 + t * 100.0,
            80.0 + 20.0 * (std::f64::consts::TAU * t).sin(),
        )
    };
    let length: f64 = (0..1000)
        .map(|k| {
            let (a, b) = (raw(f64::from(k) / 1000.0), raw(f64::from(k + 1) / 1000.0));
            (b.x - a.x).hypot(b.y - a.y)
        })
        .sum();
    let scale = 150.0 / length;
    let mut flat = Vec::new();
    for i in 0..2000 {
        let t = f64::from(i) / 1999.0;
        let p = raw(t);
        let tremor = (f64::from(i) * 12.9898).sin() * 0.05;
        let (x, y) = (
            20.0 + (p.x - 20.0) * scale,
            80.0 + (p.y - 80.0) * scale + tremor,
        );
        flat.extend([
            (x - origin.x) * px,
            (y - origin.y) * px,
            0.1 + 0.8 * t,
            f64::NAN,
            f64::NAN,
            0.0,
            1.0,
            0.0,
            f64::from(i) * 2.0,
        ]);
    }
    let last = flat[flat.len() - PEN_SAMPLE_STRIDE..].to_vec();
    flat.extend([
        last[0] + 1.0,
        last[1],
        0.0,
        f64::NAN,
        f64::NAN,
        0.0,
        0.0,
        0.0,
        4000.0,
    ]);
    flat
}

#[test]
fn criteria_2_10_a_preview_frame_and_the_release_of_2000_samples() {
    let mut session = Session::new(1);
    let _ = session.draw_list();
    session.set_tool(Tool::Pencil);
    let flat = s_curve(&session);
    let contact = flat.len() - PEN_SAMPLE_STRIDE;
    session.pen_samples(&flat[..contact]);
    let (triangles, frame) = cpu_time_of(|| session.draw_list().triangle_count());
    assert!(triangles > 0);
    let ((), release) = cpu_time_of(|| session.pen_samples(&flat[contact..]));
    let document = unpack(2, &session.pack("test").unwrap()).unwrap();
    let path = document.path(document.object_ids()[0]).unwrap();
    eprintln!(
        "preview frame {frame:?}, release {release:?}, {} nodes, {} width points",
        path.anchors.len(),
        path.width_points.len()
    );
    assert!(frame <= budget(8), "preview frame {frame:?}");
    assert!(release <= budget(200), "release {release:?}");
    // At most 1 node per 3 mm (at most 50) and no node per sample (criterion 2).
    assert!(path.anchors.len() <= 50, "{} nodes", path.anchors.len());
}
