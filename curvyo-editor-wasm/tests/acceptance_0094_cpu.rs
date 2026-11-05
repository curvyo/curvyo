//! `specs/0094-masked-composite-trace` criterion 16 (Proposal, reported, not blocking): the
//! stencil clip of a 2000 x 2000 px trace against a mask of 500 nodes, in thread CPU time. Run
//! with `cargo test --release -p curvyo-editor-wasm --test acceptance_0094_cpu -- --ignored
//! --nocapture`; a debug build is not what the worker runs.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use curvyo_document_core::{Point, Vec2};
use curvyo_ui_core::{TraceParams, TraceRegion, read_trace_wire, trace_job_with};

/// A 2000 x 2000 px picture of 400 discs on a grid, 20 x 20 of them, placed 200 mm wide.
fn picture() -> Vec<u8> {
    let side = 2000_u32;
    (0..side * side)
        .flat_map(|i| {
            let (x, y) = (f64::from(i % side) + 0.5, f64::from(i / side) + 0.5);
            let (cx, cy) = (
                (x / 100.0).floor() * 100.0 + 50.0,
                (y / 100.0).floor() * 100.0 + 50.0,
            );
            let v = if (x - cx).hypot(y - cy) <= 35.0 {
                0
            } else {
                255
            };
            [v, v, v, 255]
        })
        .collect()
}

#[test]
#[ignore = "criterion 16 harness: run in release and read the printed numbers"]
fn c16_the_stencil_clip_of_a_2000_px_trace_with_a_500_node_mask() {
    let map = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];
    let (wire, trace) = cpu_time::cpu_time_of(|| {
        trace_job_with(
            &picture(),
            2000,
            2000,
            map,
            &TraceParams::with_threshold(128),
            &mut |_| {},
        )
    });
    let outlines = read_trace_wire(&wire).unwrap().outlines;
    // A 500-node polygon (a circle of 70 mm about the middle) crossing many discs.
    let circle: Vec<_> = (0..500)
        .map(|k| {
            let a = f64::from(k) * std::f64::consts::TAU / 500.0;
            (
                Point::new(100.0 + 70.0 * a.cos(), 100.0 + 70.0 * a.sin()),
                Vec2::ZERO,
                Vec2::ZERO,
            )
        })
        .collect();
    let region = TraceRegion::of_operands(&[vec![(circle, true)]], false).unwrap();
    let (clipped, clip) = cpu_time::cpu_time_of(|| region.clip(&outlines));
    println!(
        "criterion 16: trace {:.3} s, clip {:.3} s ({} outlines, {} kept)",
        trace.as_secs_f64(),
        clip.as_secs_f64(),
        outlines.len(),
        clipped.0.len()
    );
    assert_ne!(clipped.0.len(), 0);
}
