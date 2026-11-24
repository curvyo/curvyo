//! `specs/0029-laser-job-preview-and-output` criterion 36 (a) to (d) for the job builder of
//! `curvyo-laser-core` in thread CPU time (`adrs.md` "Dependencies": the budgets live here, where
//! the thread clock already is): (a) 1,000 Cut items of 20 nodes plan in 100 ms; (b) a 200 x 200
//! mm Engrave at 0.1 mm (2,000 lines, bidirectional) in 150 ms; (c) the same with Crosshatch in
//! 300 ms; (d) the job, timeline and gate after a numbers edit on 50 operations and 1,000 items,
//! plans cached, in 16 ms. Budgets are asserted in release builds; a debug build prints the
//! times and checks the rest.

#![allow(clippy::unwrap_used)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{Document, DocumentSize, Length, NodeId, Point, RectBounds, Vec2};
use curvyo_laser_core::ProfileValues;
use curvyo_laser_core::job::{
    BlockInput, BlockSource, GateFacts, JobInput, LeafGeometry, LeafInput, LeafOutline,
    MachineInput, PlanCache, Ring, Start, Timeline, build_job, gate,
};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{
    OperationOptions, Overscan, ScanAngle, SettingsRead, ZOffset, read_settings,
};
use serde_json::json;

/// Whether the times are asserted: release builds only.
const TIMED: bool = !cfg!(debug_assertions);

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

/// A closed 20-gon of radius 3.5 mm around `(cx, cy)`.
fn polygon(cx: f64, cy: f64) -> LeafOutline {
    LeafOutline {
        anchors: (0..20)
            .map(|k| {
                let a = f64::from(k) * std::f64::consts::TAU / 20.0;
                (
                    Point::new(cx + 3.5 * a.cos(), cy + 3.5 * a.sin()),
                    Vec2::ZERO,
                    Vec2::ZERO,
                )
            })
            .collect(),
        closed: true,
    }
}

fn square(x: f64, y: f64, side: f64) -> LeafOutline {
    let c = |x, y| (Point::new(x, y), Vec2::ZERO, Vec2::ZERO);
    LeafOutline {
        anchors: vec![
            c(x, y),
            c(x + side, y),
            c(x + side, y + side),
            c(x, y + side),
        ],
        closed: true,
    }
}

fn settings(kind: OperationKind, power: f64) -> SettingsRead {
    let main = if kind == OperationKind::Engrave {
        json!({"power_pct": power, "speed_mm_min": 3000, "passes": 1, "interval_mm": 0.1})
    } else {
        json!({"power_pct": power, "speed_mm_min": 3000, "passes": 1})
    };
    read_settings(
        Some(&json!({ "main": main })),
        kind,
        curvyo_laser_core::DefaultNumbers::BUILT_IN,
    )
}

fn page(blocks: Vec<BlockInput>) -> JobInput {
    JobInput {
        name: "Laser 1".to_owned(),
        machine: MachineInput {
            profile_id: "6f1c2b0a-5d4e-4f3a-9b2c-1d0e8f7a6b5c".to_owned(),
            values: ProfileValues::default(),
            bed: DocumentSize::from_mm(400.0, 300.0),
            damaged: false,
        },
        start: Start::default(),
        start_damaged: false,
        fixture_material: None,
        blocks,
    }
}

fn block(
    id: NodeId,
    kind: OperationKind,
    power: f64,
    options: OperationOptions,
    leaves: Vec<LeafInput>,
) -> BlockInput {
    BlockInput {
        id,
        ring: Ring::Operation(1),
        name: "Op".to_owned(),
        kind: KindRead::Known(kind),
        source: BlockSource::Operation {
            settings: settings(kind, power),
            options: Ok(options),
            margin: Length::from_mm(0.0),
            grid: None,
        },
        leaves,
        cut_out: Vec::new(),
    }
}

/// 1,000 20-gons in a 40 x 25 grid, spread over `operations` Cut operations.
fn cut_page(operations: usize, power: f64) -> JobInput {
    let ids = ids(1_000 + operations);
    let mut blocks: Vec<BlockInput> = (0..operations)
        .map(|k| {
            block(
                ids[1_000 + k],
                OperationKind::Cut,
                power,
                OperationOptions::default(),
                Vec::new(),
            )
        })
        .collect();
    for i in 0..1_000_usize {
        let (col, row) = (
            f64::from(u32::try_from(i % 40).unwrap()),
            f64::from(u32::try_from(i / 40).unwrap()),
        );
        blocks[i % operations].leaves.push(LeafInput {
            node: ids[i],
            stacking: u32::try_from(i).unwrap(),
            geometry: LeafGeometry::Outlines(vec![polygon(5.0 + col * 9.5, 5.0 + row * 11.5)]),
        });
    }
    page(blocks)
}

fn engrave_page(crosshatch: bool) -> JobInput {
    let ids = ids(2);
    let options = OperationOptions {
        crosshatch,
        bidirectional: true,
        scan_angle: ScanAngle::default(),
        overscan: Overscan::DEFAULT,
        z_offset: ZOffset::default(),
        image: curvyo_laser_core::settings::ImageSettings::DEFAULT,
    };
    page(vec![block(
        ids[0],
        OperationKind::Engrave,
        30.0,
        options,
        vec![LeafInput {
            node: ids[1],
            stacking: 0,
            geometry: LeafGeometry::Outlines(vec![square(50.0, 50.0, 200.0)]),
        }],
    )])
}

fn check(name: &str, took: Duration, budget: Duration) {
    println!("{name}: {took:?} (budget {budget:?})");
    if TIMED {
        assert!(took <= budget, "{name}: {took:?} > {budget:?}");
    }
}

#[test]
fn a_thousand_cut_items_build_within_100_ms() {
    let input = cut_page(1, 90.0);
    let (built, took) = cpu_time::cpu_time_of(|| build_job(&input, &[], &mut PlanCache::default()));
    assert_eq!(built.job.blocks.len(), 1);
    assert!(built.job.moves.len() > 20_000);
    check("36 (a)", took, Duration::from_millis(100));
}

#[test]
fn an_engrave_of_two_thousand_lines_builds_within_150_ms() {
    let input = engrave_page(false);
    let (built, took) = cpu_time::cpu_time_of(|| build_job(&input, &[], &mut PlanCache::default()));
    let lines = built.plans[0].as_ref().unwrap().runs[0].paths.len();
    assert_eq!(lines, 2_000);
    check("36 (b)", took, Duration::from_millis(150));
}

#[test]
fn the_same_engrave_with_crosshatch_builds_within_300_ms() {
    let input = engrave_page(true);
    let (built, took) = cpu_time::cpu_time_of(|| build_job(&input, &[], &mut PlanCache::default()));
    let lines = built.plans[0].as_ref().unwrap().runs[0].paths.len();
    assert_eq!(lines, 4_000);
    check("36 (c)", took, Duration::from_millis(300));
}

#[test]
fn the_gate_after_a_numbers_edit_recomputes_within_16_ms() {
    let mut cache = PlanCache::default();
    let input = cut_page(50, 90.0);
    let _ = build_job(&input, &[], &mut cache);
    let edited = cut_page(50, 70.0);
    let (g, took) = cpu_time::cpu_time_of(|| {
        let built = build_job(&edited, &[], &mut cache);
        let timeline = Timeline::of(&built.job);
        gate(&edited, &built, &timeline, &GateFacts::default())
    });
    assert_eq!(g.runs.len(), 50);
    assert!(curvyo_laser_core::job::numbers_text(&g.runs[0].numbers[0]).starts_with("70 %"));
    check("36 (d)", took, Duration::from_millis(16));
}
