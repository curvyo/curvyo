//! `specs/0094-masked-composite-trace` criterion 16 (Proposal, reported, not blocking): the
//! stencil clip of a 2000 x 2000 px trace against a mask of 500 nodes, in thread CPU time. Run
//! with `cargo test --release -p curvyo-editor-wasm --test acceptance_0094_cpu -- --ignored
//! --nocapture`; a debug build is not what the worker runs.

#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

mod cpu_time;

use curvyo_document_core::{Point, Vec2};
use curvyo_ui_core::{
    Mask, TraceJob, TraceMode, TraceParams, TracePixelMap, TraceRaster, TraceRegion,
    read_trace_wire, run_trace_job,
};

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
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: 2000,
                    height: 2000,
                    rgba: &picture(),
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&TraceParams::with_threshold(128)),
                lines: &[],
                mask: None,
                shapes: false,
            },
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
    let (clipped, clip) = cpu_time::cpu_time_of(|| Mask::clip(&region, &outlines));
    println!(
        "criterion 16: trace {:.3} s, clip {:.3} s ({} outlines, {} kept)",
        trace.as_secs_f64(),
        clip.as_secs_f64(),
        outlines.len(),
        clipped.outlines.len()
    );
    assert_ne!(clipped.outlines.len(), 0);
}

/// `rgba` (`side` px square) cut to `x0..x1` by `y0..y1` px, and `map` moved with the cut (the
/// session's crop of a part, `trace_parts.rs`).
fn crop(
    rgba: &[u8],
    side: u32,
    (x0, y0, x1, y1): (u32, u32, u32, u32),
    map: [f64; 6],
) -> (Vec<u8>, [f64; 6]) {
    let mut out = Vec::with_capacity(((x1 - x0) * (y1 - y0) * 4) as usize);
    for y in y0..y1 {
        let start = ((y * side + x0) * 4) as usize;
        out.extend_from_slice(&rgba[start..start + ((x1 - x0) * 4) as usize]);
    }
    let moved = [
        map[0],
        map[1],
        map[2],
        map[3],
        map[4] + map[0] * f64::from(x0) + map[2] * f64::from(y0),
        map[5] + map[1] * f64::from(x0) + map[3] * f64::from(y0),
    ];
    (out, moved)
}

#[test]
#[ignore = "criterion 31 harness: run in release and read the printed numbers"]
fn c31_four_quarter_parts_against_one_whole_trace() {
    use curvyo_ui_core::{TracePartArea, TracePartClip, join_trace_parts, trace_part_clips};
    let side = 2000_u32;
    let map = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];
    let rgba = picture();
    let params = TraceParams::with_threshold(128);
    let (_, whole) = cpu_time::cpu_time_of(|| {
        run_trace_job(
            &TraceJob {
                raster: TraceRaster {
                    width: side,
                    height: side,
                    rgba: &rgba,
                },
                map: TracePixelMap::new(map),
                mode: TraceMode::Mono(&params),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        )
    });
    // Four quarters of 100 x 100 mm, Inside; the margin of the session's crop: 16 px, or the
    // pixels of Ignore below when more.
    let specks = params.specks.as_mm();
    // invariant: a small whole number of pixels.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let margin = (specks * specks / 0.01).ceil().max(16.0) as u32;
    let quarters = [(0.0, 0.0), (100.0, 0.0), (0.0, 100.0), (100.0, 100.0)];
    let rect = |x: f64, y: f64| {
        [
            (x, y),
            (x + 100.0, y),
            (x + 100.0, y + 100.0),
            (x, y + 100.0),
        ]
        .map(|(px, py)| (Point::new(px, py), Vec2::ZERO, Vec2::ZERO))
        .to_vec()
    };
    let areas: Vec<TracePartArea> = quarters
        .iter()
        .map(|&(x, y)| {
            TracePartArea::Mask(
                TraceRegion::of_operands(&[vec![(rect(x, y), true)]], false).unwrap(),
            )
        })
        .collect();
    let quad = [
        Point::new(0.0, 0.0),
        Point::new(200.0, 0.0),
        Point::new(200.0, 200.0),
        Point::new(0.0, 200.0),
    ];
    let (joined, parts) = cpu_time::cpu_time_of(|| {
        let clips = trace_part_clips(&areas, &quad);
        let wires: Vec<_> = quarters
            .iter()
            .zip(&clips)
            .map(|(&(x, y), clip)| {
                // invariant: the quarters' pixel boxes are whole numbers.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let (px, py) = ((x * 10.0) as u32, (y * 10.0) as u32);
                let cut = (
                    px.saturating_sub(16),
                    py.saturating_sub(16),
                    (px + 1016).min(side),
                    (py + 1016).min(side),
                );
                let (pixels, moved) = crop(&rgba, side, cut, map);
                read_trace_wire(&run_trace_job(
                    &TraceJob {
                        raster: TraceRaster {
                            width: cut.2 - cut.0,
                            height: cut.3 - cut.1,
                            rgba: &pixels,
                        },
                        map: TracePixelMap::new(moved),
                        mode: TraceMode::Mono(&params),
                        lines: &[],
                        mask: clip.region().map(|r| r as &dyn Mask),
                        shapes: false,
                    },
                    &mut |_| {},
                ))
                .unwrap()
            })
            .collect();
        let pairs: Vec<(&curvyo_ui_core::TraceWire, &TracePartClip)> =
            wires.iter().zip(&clips).collect();
        join_trace_parts(&pairs, 0)
    });
    println!(
        "criterion 31: one whole trace {:.3} s, four quarter parts {:.3} s ({:.2} x), margin {margin} px, {} outlines",
        whole.as_secs_f64(),
        parts.as_secs_f64(),
        parts.as_secs_f64() / whole.as_secs_f64(),
        joined.outlines.len()
    );
    assert_ne!(joined.outlines.len(), 0);
}
