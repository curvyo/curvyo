//! Criterion 26 of `specs/0025-raster-trace`: a trace of a 16 Mpx picture needs at most 16 bytes
//! per pixel besides the picture, measured by a counting global allocator. Its own test binary
//! (the allocator is global); `#[ignore]`, run in release:
//!
//! ```text
//! cargo test --release -p curvyo-vectorize-core --test memory -- --ignored --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::type_complexity,
    clippy::needless_pass_by_value
)]

mod common;

use std::alloc::System;

use cap::Cap;
use common::{params, run};
use curvyo_document_core::Affine;

#[global_allocator]
static ALLOCATOR: Cap<System> = Cap::new(System, usize::MAX);

#[test]
#[ignore = "memory: run in release"]
fn a_16_mpx_trace_stays_within_16_bytes_per_pixel() {
    let side = 4096_u32;
    let mut noise = common::quality::Noise::new(9);
    // Line art over the whole picture: rings, a full-page frame and noise specks.
    let rings: Vec<(f64, f64, f64)> = (0..600)
        .map(|_| {
            (
                2048.0 + 2000.0 * noise.next(),
                2048.0 + 2000.0 * noise.next(),
                10.0 + 60.0 * noise.next().abs(),
            )
        })
        .collect();
    let mut levels = vec![255_u8; (side * side) as usize];
    for (i, level) in levels.iter_mut().enumerate() {
        let (x, y) = ((i % side as usize) as f64, (i / side as usize) as f64);
        if x < 8.0 || y < 8.0 || x > f64::from(side) - 9.0 || y > f64::from(side) - 9.0 {
            *level = 0;
        }
    }
    for &(cx, cy, r) in &rings {
        let (x0, x1) = (
            (cx - r - 3.0).max(0.0) as usize,
            ((cx + r + 3.0) as usize).min(side as usize - 1),
        );
        let (y0, y1) = (
            (cy - r - 3.0).max(0.0) as usize,
            ((cy + r + 3.0) as usize).min(side as usize - 1),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
                if (d - r).abs() <= 3.0 {
                    levels[y * side as usize + x] = 0;
                }
            }
        }
    }
    for _ in 0..50_000 {
        let x = ((noise.next() + 1.0) * 2047.5) as usize;
        let y = ((noise.next() + 1.0) * 2047.5) as usize;
        levels[y * side as usize + x] = 0;
    }
    let picture = common::Picture::grey(side, side, &levels);
    drop(levels);
    let before = ALLOCATOR.allocated();
    // `max_allocated` is the peak since the start; the picture's own build peaked lower than
    // the trace (checked by the print), so the difference is the trace's.
    let peak_before = ALLOCATOR.max_allocated();
    let result = run(
        &picture,
        Affine::scale(25.4 / 300.0, 25.4 / 300.0),
        &params(128, 80, 0.5),
    );
    let peak = ALLOCATOR.max_allocated();
    let pixels = f64::from(side) * f64::from(side);
    let per_pixel = (peak - before) as f64 / pixels;
    println!(
        "16 Mpx: {} paths, {} nodes; picture {} MB, peak before {} MB, trace peak {:.2} B/px",
        result.counts.paths,
        result.counts.nodes,
        before / 1_000_000,
        peak_before / 1_000_000,
        per_pixel
    );
    assert!(peak > peak_before, "the trace set the peak");
    assert!(per_pixel <= 16.0, "{per_pixel} bytes per pixel");
}

/// `specs/0089` slice M (adrs.md §8): the peak memory of a multicolour trace of the slowest
/// real photo (`customer-photo-fox-2` where the local images are present, else
/// `photo-still-life`), in bytes per pixel besides the picture; reported, not asserted (the
/// 16 B/px budget is single colour's, `docs/technical-debt.md`).
#[test]
#[ignore = "memory: run in release"]
fn multicolour_peak_of_the_slowest_photo_is_reported() {
    use std::ops::ControlFlow;

    use curvyo_vectorize_core::{
        ColourParams, DEFAULT_BACKGROUND_TOLERANCE, Selection, TraceParams, suggest_background,
        trace_colours,
    };

    let image = common::quality::real::customer_images()
        .into_iter()
        .find(|i| i.id == "customer-photo-fox-2")
        .or_else(|| {
            common::quality::real::real_images()
                .into_iter()
                .find(|i| i.id == "photo-still-life")
        })
        .unwrap();
    let picture = image.picture;
    let params = ColourParams::new(TraceParams {
        selection: Selection::RemoveBackground {
            background: suggest_background(picture.raster()),
            tolerance: DEFAULT_BACKGROUND_TOLERANCE,
        },
        ..TraceParams::with_threshold(128)
    });
    let before = ALLOCATOR.allocated();
    let peak_before = ALLOCATOR.max_allocated();
    let mut go = |_| ControlFlow::Continue(());
    let result = trace_colours(picture.raster(), common::at_96_dpi(), &params, &mut go).unwrap();
    let peak = ALLOCATOR.max_allocated();
    let pixels = f64::from(picture.width) * f64::from(picture.height);
    println!(
        "{} {} x {}: {} colours, {} paths; picture {} MB, peak before {} MB, trace peak {:.2} B/px",
        image.id,
        picture.width,
        picture.height,
        result.palette.len(),
        result.counts.paths,
        before / 1_000_000,
        peak_before / 1_000_000,
        (peak - before) as f64 / pixels
    );
    assert!(peak > peak_before, "the trace set the peak");
}
