//! `specs/0123-raster-engraving` criterion 37 (a) and (b) for the raster job of
//! `curvyo-laser-core` in thread CPU time (ADR 0068 §8, ADR 0069 §11): (a) a 200 x 200 mm image
//! at 0.1 mm (2,000 rows of 2,000 dots) builds its sweeps within 200 ms in Threshold and Ordered
//! and 350 ms in the four error diffusions, the grey pyramid of its 16 Mpx picture included;
//! (b) the GRBL writer turns a worst-case dither (about 2 million runs) into the file within
//! 250 ms. Budgets are asserted in release builds; a debug build prints the times and checks
//! the rest.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use std::sync::Arc;
use std::time::Duration;

use curvyo_document_core::{AssetHash, Document, DocumentSize, Length, NodeId, Point, RectBounds};
use curvyo_image_core::WorkingImage;
use curvyo_laser_core::ProfileValues;
use curvyo_laser_core::job::{
    BlockInput, BlockSource, Built, JobInput, LeafGeometry, LeafInput, MachineInput, PlanCache,
    Ring, Start, build_job, write,
};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{Dither, ImageSettings, OperationOptions, read_settings};
use serde_json::json;

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

/// A 4,000 x 4,000 picture (16 Mpx) of a smooth two-axis shade with a fine checker on top, so
/// every mode has work in every row.
fn picture() -> Arc<WorkingImage> {
    let side = 4_000_u32;
    let mut pixels = Vec::with_capacity(side as usize * side as usize * 4);
    for y in 0..side {
        for x in 0..side {
            let shade = (x + y) * 255 / (2 * side);
            let checker = if (x / 2 + y / 2) % 2 == 0 { 0 } else { 40 };
            let g = u8::try_from((shade + checker).min(255)).unwrap();
            pixels.extend_from_slice(&[g, g, g, 255]);
        }
    }
    Arc::new(WorkingImage {
        width: side,
        height: side,
        pixels,
        original_size: (side, side),
        truncated: false,
        animated: false,
    })
}

fn ids(n: usize) -> Vec<NodeId> {
    let d = Document::new(1);
    (0..n)
        .map(|i| {
            let x = f64::from(u32::try_from(i).unwrap());
            d.create_rect(RectBounds::from_corners(
                Point::new(x, 0.0),
                Point::new(x + 1.0, 1.0),
            ))
        })
        .collect()
}

fn page(pixels: &Arc<WorkingImage>, dither: Dither) -> JobInput {
    let ids = ids(2);
    let kind = OperationKind::Image;
    let settings = read_settings(
        Some(
            &json!({"main": {"power_pct": 50, "speed_mm_min": 6000, "passes": 1, "interval_mm": 0.1}}),
        ),
        kind,
    );
    let options = OperationOptions {
        image: ImageSettings {
            dither,
            ..ImageSettings::DEFAULT
        },
        ..OperationOptions::default()
    };
    let frame = [
        Point::new(50.0, 50.0),
        Point::new(250.0, 50.0),
        Point::new(250.0, 250.0),
        Point::new(50.0, 250.0),
    ];
    JobInput {
        name: "Laser 1".into(),
        machine: MachineInput {
            profile_id: "budget".into(),
            values: ProfileValues::default(),
            bed: DocumentSize::from_mm(400.0, 300.0),
            damaged: false,
        },
        start: Start::default(),
        start_damaged: false,
        fixture_material: None,
        blocks: vec![BlockInput {
            id: ids[0],
            ring: Ring::Operation(1),
            name: "Image 1".into(),
            kind: KindRead::Known(kind),
            source: BlockSource::Operation {
                settings,
                options: Ok(options),
                margin: Length::from_mm(0.0),
                grid: None,
            },
            leaves: vec![LeafInput {
                node: ids[1],
                stacking: 0,
                geometry: LeafGeometry::Image {
                    adjust: curvyo_laser_core::adjust::AdjustRegisters::default(),
                    frame,
                    asset: AssetHash::of(b"budget"),
                    pixels: Some(Arc::clone(pixels)),
                },
            }],
            cut_out: Vec::new(),
        }],
    }
}

fn check(name: &str, took: Duration, budget: Duration) {
    println!("{name}: {took:?} (budget {budget:?})");
    if TIMED {
        assert!(took <= budget, "{name}: {took:?} > {budget:?}");
    }
}

fn build(input: &JobInput) -> (Built, Duration) {
    cpu_time::cpu_time_of(|| build_job(input, &[], &mut PlanCache::default()))
}

#[test]
fn a_four_million_dot_image_builds_within_its_budget_in_every_mode() {
    let pixels = picture();
    for (dither, ms) in [
        (Dither::Threshold, 200),
        (Dither::Ordered, 200),
        (Dither::FloydSteinberg, 350),
        (Dither::Jarvis, 350),
        (Dither::Atkinson, 350),
        (Dither::Stucki, 350),
        // ADR 0074 §6: the two screens within the Threshold and Ordered budget.
        (Dither::Halftone, 200),
        (Dither::LineScreen, 200),
    ] {
        let (built, took) = build(&page(&pixels, dither));
        assert!(
            built.job.sweeps.len() >= 2_000,
            "{dither:?}: a sweep per row at least"
        );
        check(&format!("37 a {dither:?}"), took, Duration::from_millis(ms));
    }
}

#[test]
fn the_grbl_writer_turns_two_million_runs_into_the_file_within_250_ms() {
    let pixels = picture();
    let input = page(&pixels, Dither::FloydSteinberg);
    let built = build_job(&input, &[], &mut PlanCache::default());
    let runs: usize = built.job.sweeps.iter().map(|s| s.runs().count()).sum();
    assert!(runs >= 1_500_000, "a worst-case dither: {runs} runs");
    let (bytes, took) =
        cpu_time::cpu_time_of(|| write(input.machine.values.output_format, &built.job));
    assert!(bytes.len() > runs * 10);
    check("37 b", took, Duration::from_millis(250));
}
