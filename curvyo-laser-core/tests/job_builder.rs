//! The job builder of `specs/0029-laser-job-preview-and-output` without a format: flattening
//! tolerance (17), order (20, 22), passes (21), the time estimate (10), the play slider (37), the
//! plan cache (36 d) and the gate's causes (2 to 7, 21, 33, 38).

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

mod job_fixtures;

use std::sync::Arc;

use curvyo_document_core::Point;
use curvyo_laser_core::OutputFormat;
use curvyo_laser_core::job::{
    Block, BlockNumbers, BlockSource, Cause, Frame, GateFacts, Job, Move, MoveKind, Names,
    PlanCache, PointUm, Ring, RunNumbers, Severity, Start, Targets, Timeline, build_job,
    check_text, estimate_text, gate, machine_line, numbers_text, plan_block,
};
use curvyo_laser_core::library::MaterialRead;
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{Passes, PowerPercent, Role, SetKind};
use curvyo_laser_core::{FeedRate, GENERIC_LASER_ID, OriginCorner, ProfileValues};
use job_fixtures::{
    BIRCH, Page, circle, leaf, line, material, options, own, own_engrave, plain, rect, settings,
};
use serde_json::json;

use OperationKind::{Cut, Engrave, Score};

fn burns(job: &Job) -> Vec<PointUm> {
    job.moves
        .iter()
        .filter(|m| matches!(m.kind, MoveKind::Burn { .. }))
        .map(|m| m.to)
        .collect()
}

fn gate_of(
    page: &Page,
    materials: &[MaterialRead],
    facts: &GateFacts,
) -> curvyo_laser_core::job::Gate {
    let built = build_job(&page.input, materials, &mut PlanCache::default());
    let timeline = Timeline::of(&built.job);
    gate(&page.input, &built, &timeline, facts)
}

/// The checks' sentences with the page's operation names.
fn texts(page: &Page, gate: &curvyo_laser_core::job::Gate) -> Vec<String> {
    let names = Names::of(page.input.blocks.iter().map(|b| (b.id, b.name.clone())));
    gate.issues
        .iter()
        .map(|issue| check_text(issue, &names).whole())
        .collect()
}

fn causes(page: &Page, materials: &[MaterialRead]) -> Vec<Cause> {
    gate_of(page, materials, &GateFacts::default())
        .issues
        .into_iter()
        .map(|i| i.cause)
        .collect()
}

/// The cubic of one circle quarter, evaluated at `t`.
fn bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Point::new(
        a * p0.x + b * p1.x + c * p2.x + d * p3.x,
        a * p0.y + b * p1.y + c * p2.y + d * p3.y,
    )
}

fn distance_to_polyline(p: Point, points: &[Point]) -> f64 {
    points
        .windows(2)
        .map(|w| {
            let (a, b) = (w[0], w[1]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            (a.x + t * dx - p.x).hypot(a.y + t * dy - p.y)
        })
        .fold(f64::INFINITY, f64::min)
}

#[test]
fn a_curve_is_written_within_a_hundredth_of_a_millimetre() {
    // Criterion 17: sample the exact cubics against the written polyline.
    let mut page = Page::new();
    let ring = circle(100.0, 100.0, 40.0);
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![ring.clone()])],
    );
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    let start = job
        .moves
        .iter()
        .find(|m| m.kind == MoveKind::Travel)
        .unwrap()
        .to;
    let written: Vec<Point> = std::iter::once(start)
        .chain(burns(&job))
        .map(PointUm::to_mm)
        .collect();
    let mut worst = 0.0_f64;
    for (i, from) in ring.anchors.iter().enumerate() {
        let to = ring.anchors[(i + 1) % ring.anchors.len()];
        let (p0, p3) = (from.0, to.0);
        let p1 = Point::new(p0.x + from.2.x, p0.y + from.2.y);
        let p2 = Point::new(p3.x + to.1.x, p3.y + to.1.y);
        for k in 0..=200 {
            let p = bezier(p0, p1, p2, p3, f64::from(k) / 200.0);
            worst = worst.max(distance_to_polyline(p, &written));
        }
    }
    assert!(worst <= 0.01, "deviation {worst} mm");
    assert!(burns(&job).iter().all(|p| p.x >= 0), "µm points");
}

#[test]
fn passes_repeat_the_whole_operation_in_the_same_order() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 2),
        plain(),
        vec![
            leaf(vec![rect(10.0, 10.0, 10.0, 10.0)]),
            leaf(vec![rect(40.0, 10.0, 10.0, 10.0)]),
        ],
    );
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    let all = burns(&job);
    assert_eq!(all.len(), 16);
    assert_eq!(
        all[..8],
        all[8..],
        "pass 1 of all paths, then pass 2 (Frage 223 A)"
    );
}

#[test]
fn the_estimate_is_burn_over_feed_plus_travel_over_the_maximum_feed() {
    // Criterion 10's test: 100 mm Cut at 600 mm/min, 2 passes, 100 mm travel at 6,000 mm/min.
    let numbers = RunNumbers {
        power: PowerPercent::from_tenths(500).unwrap(),
        speed: FeedRate::new(600).unwrap(),
        feed: FeedRate::new(600).unwrap(),
        passes: Passes::new(2).unwrap(),
    };
    let ids = job_fixtures::ids(2);
    let burn = |x| Move {
        to: PointUm::new(x, 0),
        kind: MoveKind::Burn {
            role: Role::Main,
            node: ids[1],
        },
        block: 0,
    };
    let job = Job {
        name: "Laser 1".into(),
        machine: ProfileValues::default(),
        frame: Frame {
            corner: OriginCorner::TopLeft,
            width: 400_000,
            height: 300_000,
        },
        start: Start::default(),
        home: PointUm::new(0, 0),
        blocks: vec![Block {
            id: ids[0],
            ring: Ring::Operation(1),
            name: "Cut 1".into(),
            kind: Cut,
            numbers: BlockNumbers {
                kind: Cut,
                main: numbers,
                outline: None,
                interval: curvyo_laser_core::settings::Interval::default(),
                options: plain(),
            },
            z: None,
        }],
        moves: vec![
            burn(100_000),
            Move {
                to: PointUm::new(0, 0),
                kind: MoveKind::Travel,
                block: 0,
            },
            burn(100_000),
        ],
        sweeps: Vec::new(),
    };
    let timeline = Timeline::of(&job);
    assert!((timeline.total_s() - 21.0).abs() < 1e-9);
    assert_eq!(estimate_text(timeline.total_s()), "About 21 s");
    assert!((timeline.burn_mm() - 200.0).abs() < 1e-9);
    assert!((timeline.travel_mm() - 100.0).abs() < 1e-9);
    // Criterion 37: the slider maps time; at half the total time the head has burnt the first
    // 100 mm and travelled 50 mm back (10.5 s: 10 s of burn, 0.5 s of travel).
    let head = timeline.head_at(&job, 0.5);
    assert!((head.elapsed_s - 10.5).abs() < 1e-9);
    assert!((head.burn_mm - 100.0).abs() < 0.01);
    assert!((head.travel_mm - 50.0).abs() < 0.01);
    assert!((head.at.x - 50.0).abs() < 0.01);
    let end = timeline.head_at(&job, 1.0);
    assert_eq!(end.done, job.moves.len());
    assert!((end.burn_mm - 200.0).abs() < 1e-9);
    let start = timeline.head_at(&job, 0.0);
    assert_eq!((start.burn_mm, start.travel_mm), (0.0, 0.0));
}

#[test]
fn the_slider_length_up_to_the_head_equals_the_moves_up_to_it() {
    let mut page = Page::new();
    page.operation(
        Engrave,
        "Engrave 1",
        own_engrave(Engrave, 30.0, 3000, 0.5),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 20.0, 10.0)])],
    );
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    let timeline = Timeline::of(&job);
    for k in 0..=20 {
        let head = timeline.head_at(&job, f64::from(k) / 20.0);
        let mut from = job.home;
        let (mut burn, mut travel) = (0.0, 0.0);
        for m in &job.moves[..head.done] {
            let length = from.distance_mm(m.to);
            if matches!(m.kind, MoveKind::Burn { .. }) {
                burn += length;
            } else {
                travel += length;
            }
            from = m.to;
        }
        let current = job
            .moves
            .get(head.done)
            .map_or(0.0, |m| from.distance_mm(m.to));
        let extra = head.burn_mm + head.travel_mm - burn - travel;
        assert!(
            extra > -1e-9 && extra <= current + 0.01,
            "{extra} of {current}"
        );
        assert!((head.elapsed_s - timeline.total_s() * f64::from(k) / 20.0).abs() < 0.5);
    }
}

#[test]
fn hatch_lines_are_an_interval_apart_and_alternate() {
    let mut page = Page::new().corner(OriginCorner::TopLeft);
    page.operation(
        Engrave,
        "Engrave 1",
        own_engrave(Engrave, 30.0, 3000, 0.5),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 20.0, 2.0)])],
    );
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    let spans: Vec<(PointUm, PointUm)> = job
        .moves
        .windows(2)
        .filter(|w| matches!(w[1].kind, MoveKind::Burn { .. }))
        .map(|w| (w[0].to, w[1].to))
        .collect();
    let ys: Vec<i64> = spans.iter().map(|s| s.0.y).collect();
    assert_eq!(ys, [10_250, 10_750, 11_250, 11_750]);
    assert!(
        spans[0].1.x > spans[0].0.x && spans[1].1.x < spans[1].0.x,
        "bidirectional"
    );
    // Off: every line the same way, a laser-off travel back.
    page.input.blocks[0].source = BlockSource::Operation {
        settings: own_engrave(Engrave, 30.0, 3000, 0.5),
        options: Ok(options(false, 0.0, false, 0, 0.0)),
        margin: curvyo_document_core::Length::from_mm(0.0),
        grid: None,
    };
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    let dirs: Vec<bool> = job
        .moves
        .windows(2)
        .filter(|w| matches!(w[1].kind, MoveKind::Burn { .. }))
        .map(|w| w[1].to.x > w[0].to.x)
        .collect();
    assert!(dirs.iter().all(|d| *d == dirs[0]));
}

#[test]
fn the_plan_cache_reuses_plans_while_geometry_and_entry_stay() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let mut cache = PlanCache::default();
    let first = build_job(&page.input, &[], &mut cache);
    page.input.blocks[0].source = BlockSource::Operation {
        settings: own(Cut, 40.0, 900, 3),
        options: Ok(plain()),
        margin: curvyo_document_core::Length::from_mm(0.0),
        grid: None,
    };
    let numbers_only = build_job(&page.input, &[], &mut cache);
    assert!(Arc::ptr_eq(
        first.plans[0].as_ref().unwrap(),
        numbers_only.plans[0].as_ref().unwrap()
    ));
    assert_ne!(first.job.moves, numbers_only.job.moves, "passes changed");
    page.input.blocks[0].leaves[0].geometry = leaf(vec![rect(11.0, 10.0, 10.0, 10.0)]);
    let moved = build_job(&page.input, &[], &mut cache);
    assert!(!Arc::ptr_eq(
        first.plans[0].as_ref().unwrap(),
        moved.plans[0].as_ref().unwrap()
    ));
    assert_eq!(cache.len(), 1);
}

#[test]
fn plan_block_is_entered_where_the_previous_block_left_the_head() {
    let mut page = Page::new();
    page.operation(
        Score,
        "Score 1",
        own(Score, 20.0, 3000, 1),
        plain(),
        vec![leaf(vec![rect(100.0, 100.0, 10.0, 10.0)])],
    );
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![
            leaf(vec![rect(10.0, 10.0, 10.0, 10.0)]),
            leaf(vec![rect(120.0, 100.0, 10.0, 10.0)]),
        ],
    );
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let score_exit = built.plans[0].as_ref().unwrap().exit;
    let numbers = *built.numbers[1].numbers().unwrap();
    assert_eq!(
        **built.plans[1].as_ref().unwrap(),
        plan_block(&page.input.blocks[1], &numbers, score_exit)
    );
    // The nearer square (beside the Score) runs first.
    let cut = built.plans[1].as_ref().unwrap();
    assert!(cut.runs[0].paths[0].points[0].x >= 120_000);
}

#[test]
fn the_gate_names_the_machine_and_every_run() {
    let mut page = Page::new();
    page.values().air_assist = true;
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    page.operation(
        Engrave,
        "Engrave 1",
        own_engrave(Engrave, 35.0, 6000, 0.25),
        plain(),
        vec![leaf(vec![rect(30.0, 10.0, 10.0, 10.0)])],
    );
    let g = gate_of(&page, &[], &GateFacts::default());
    // Superseded by 0029 rework (2026-11-17): one machine line "<profile> · <format>[ · air
    // assist]" (P30), no mode line; the ring is the row's digit, the source word is gone.
    assert_eq!(machine_line(&g.machine), "Diode 10 W · GRBL · air assist");
    assert_eq!(g.runs.len(), 2);
    assert_eq!(g.runs[0].ring, Ring::Operation(1));
    assert_eq!(g.runs[1].ring, Ring::Operation(2));
    assert_eq!(
        numbers_text(&g.runs[1].numbers[0]),
        "35 %, 6,000 mm/min, 1 ×, 0.25 mm"
    );
    assert!(g.runs.iter().all(|r| r.time_s.is_some_and(|t| t > 0.0)));
    assert!(g.export_allowed(), "{:?}", g.issues);
}

#[test]
fn a_live_material_with_a_typed_power_warns_and_shows_typed() {
    let mut page = Page::new();
    let read = settings(
        Cut,
        json!({"main": {"power_pct": 80.0, "speed_mm_min": 300, "passes": 1},
               "material": {"id": BIRCH, "name": "Birch 3 mm", "modified": 1},
               "overrides": {"main": {"power_pct": 95.0}}}),
    );
    page.operation(
        Cut,
        "Cut 1",
        read,
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let birch = material(BIRCH, "Birch 3 mm", &[(SetKind::Cut, 85.0, 250, 1)]);
    let g = gate_of(&page, &[birch], &GateFacts::default());
    // Superseded by 0029 rework (2026-11-17): the material and source word leave the run (they
    // show in the unfolded row only, P27); the typed number and the check stay.
    assert_eq!(
        numbers_text(&g.runs[0].numbers[0]),
        "95 % typed, 250 mm/min, 1 ×"
    );
    assert_eq!(
        texts(&page, &g)[0],
        "Cut 1 runs a typed power, not Birch 3 mm’s."
    );
    assert!(g.export_allowed());
    // The material is missing: stored numbers, a warn.
    let g = gate_of(&page, &[], &GateFacts::default());
    assert_eq!(
        texts(&page, &g)[0],
        "Cut 1: Birch 3 mm is not found, using stored numbers."
    );
}

#[test]
fn blocks_disable_export_and_come_first() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 2",
        curvyo_laser_core::settings::SettingsRead::None,
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    page.operation(
        Cut,
        "Cut 3",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![
            leaf(vec![rect(390.0, 10.0, 20.0, 10.0)]),
            leaf(vec![rect(-5.0, 10.0, 20.0, 10.0)]),
        ],
    );
    page.input.machine.profile_id = GENERIC_LASER_ID.to_owned();
    let facts = GateFacts {
        not_assigned: job_fixtures::ids(3),
        ..GateFacts::default()
    };
    let g = gate_of(&page, &[], &facts);
    assert_eq!(
        texts(&page, &g),
        [
            "Cut 2 has no power and speed.",
            "2 items lie outside the 400 × 300 mm bed.",
            "The machine is Generic laser; its limits may not match yours.",
            "3 items are not assigned.",
        ]
    );
    assert!(!g.export_allowed());
    assert_eq!(
        g.first_block().unwrap().cause,
        Cause::NoSettings {
            op: page.input.blocks[0].id
        }
    );
    assert_eq!(
        g.issues[0].targets,
        Targets::Operations(vec![page.input.blocks[0].id])
    );
    assert!(matches!(&g.issues[1].targets, Targets::Items(items) if items.len() == 2));
    // Superseded by 0029 rework (2026-11-17): the run carries no text; No settings has no time.
    assert!(g.runs[0].no_settings && g.runs[0].time_s.is_none());
}

#[test]
fn nothing_runs_and_a_damaged_machine_block() {
    let mut page = Page::new();
    assert_eq!(causes(&page, &[]), [Cause::NothingRuns]);
    page.input.machine.damaged = true;
    let g = gate_of(&page, &[], &GateFacts::default());
    assert_eq!(machine_line(&g.machine), "Machine profile damaged");
    assert_eq!(g.issues[0].cause, Cause::MachineDamaged);
    assert_eq!(g.issues[0].cause.severity(), Severity::Block);
}

#[test]
fn operations_without_items_run_nothing_whatever_their_settings() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        curvyo_laser_core::settings::SettingsRead::None,
        plain(),
        vec![],
    );
    page.operation(Cut, "Cut 2", own(Cut, 90.0, 300, 1), plain(), vec![]);
    assert_eq!(causes(&page, &[]), [Cause::NothingRuns]);
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    assert_eq!(built.job.moves.len(), 0);
}

#[test]
fn over_feed_and_z_without_an_axis_warn() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 9000, 1),
        options(true, 0.0, false, 0, -2.0),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let found = causes(&page, &[]);
    // Superseded by 0029 rework (2026-11-17): causes carry the operation's id, not its name.
    let op = page.input.blocks[0].id;
    assert_eq!(
        found,
        [
            Cause::OverFeed { op, feed: 6000 },
            Cause::ZWithoutAxis { op },
        ]
    );
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    assert!(
        built
            .job
            .moves
            .iter()
            .all(|m| !matches!(m.kind, MoveKind::Z(_))),
        "Z ignored"
    );
    let numbers = built.job.blocks[0].of(Role::Main);
    assert_eq!(numbers.feed.get(), 6000, "the written feed is the maximum");
}

#[test]
fn unknown_and_damaged_operations_run_nothing_and_warn_once() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    page.operation(
        Cut,
        "X",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(30.0, 10.0, 10.0, 10.0)])],
    );
    page.input.blocks[1].kind = KindRead::Unknown;
    page.operation(
        Cut,
        "Cut 2",
        curvyo_laser_core::settings::read_settings(Some(&json!(7)), Cut),
        plain(),
        vec![leaf(vec![rect(50.0, 10.0, 10.0, 10.0)])],
    );
    let g = gate_of(&page, &[], &GateFacts::default());
    assert_eq!(g.runs.len(), 1, "no run line for them (criterion 33)");
    assert_eq!(g.issues.len(), 1);
    assert_eq!(
        texts(&page, &g)[0],
        "2 operations have damaged settings and do not run."
    );
    assert!(g.export_allowed());
}

#[test]
fn fixture_jobs_need_a_material_with_a_set() {
    let mut page = Page::new();
    page.fixture(Score, vec![leaf(vec![rect(5.0, 5.0, 10.0, 10.0)])]);
    assert_eq!(causes(&page, &[])[0], Cause::FixtureNoMaterial);
    page.input.fixture_material = Some(BIRCH.to_owned());
    let cut_only = material(BIRCH, "Oak 4 mm", &[(SetKind::Cut, 90.0, 300, 1)]);
    assert_eq!(
        causes(&page, &[cut_only])[0].clone(),
        Cause::FixtureNoSet {
            material: "Oak 4 mm".into(),
            kind: SetKind::Score
        }
    );
    let birch = material(BIRCH, "Birch 3 mm", &[(SetKind::Score, 12.0, 4000, 1)]);
    let g = gate_of(&page, &[birch], &GateFacts::default());
    assert!(g.export_allowed(), "{:?}", g.issues);
    assert_eq!(g.runs[0].ring, Ring::Fixture);
    assert_eq!(
        numbers_text(&g.runs[0].numbers[0]),
        "12 %, 4,000 mm/min, 1 ×"
    );
}

#[test]
fn head_position_checks_the_bed_from_the_jobs_own_box_and_never_warns() {
    // Superseded by 0029 rework (2026-11-17): no Head position warn; the bed check measures from
    // the job's own box (criterion 38), so geometry off the page fits while the job fits the bed.
    let mut page = Page::new().start(Start::HeadPosition);
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(-50.0, 10.0, 10.0, 10.0)])],
    );
    assert_eq!(causes(&page, &[]), [] as [Cause; 0]);
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    assert_eq!(
        job.home,
        PointUm::new(-40_000, 20_000),
        "the first point is where the head stands"
    );
    assert_eq!(
        job.moves.last().unwrap().to,
        job.home,
        "and the end returns to it"
    );
    // A job wider than the 400 mm bed blocks, measured from its own box.
    page.operation(
        Cut,
        "Cut 2",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(360.0, 10.0, 10.0, 10.0)])],
    );
    let found = causes(&page, &[]);
    assert_eq!(
        found,
        [Cause::OutsideBed {
            width_um: 400_000,
            height_um: 300_000
        }]
    );
    let g = gate_of(&page, &[], &GateFacts::default());
    assert!(matches!(&g.issues[0].targets, Targets::Items(items) if items.len() == 1));
}

#[test]
fn head_position_places_the_work_area_on_the_matching_corner_of_the_box() {
    // Bottom-right origin: the bed hangs left and up from the box's bottom-right corner, so a
    // part 390 mm left of another fits and one 410 mm left does not.
    for (gap, outside) in [(380.0, false), (400.0, true)] {
        let mut page = Page::new()
            .corner(OriginCorner::BottomRight)
            .start(Start::HeadPosition);
        page.operation(
            Cut,
            "Cut 1",
            own(Cut, 90.0, 300, 1),
            plain(),
            vec![
                leaf(vec![rect(1000.0, 500.0, 10.0, 10.0)]),
                leaf(vec![rect(1000.0 - gap, 500.0, 10.0, 10.0)]),
            ],
        );
        let blocked = causes(&page, &[])
            .iter()
            .any(|c| matches!(c, Cause::OutsideBed { .. }));
        assert_eq!(blocked, outside, "gap {gap}");
    }
}

#[test]
fn home_then_origin_runs_the_machine_origin_moves() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let origin = build_job(&page.input, &[], &mut PlanCache::default()).job;
    page.input.start = Start::HomeThenOrigin;
    let home = build_job(&page.input, &[], &mut PlanCache::default()).job;
    assert_eq!(home.moves, origin.moves);
    assert_eq!(home.home, origin.home);
    assert_eq!(causes(&page, &[]), [] as [Cause; 0]);
}

#[test]
fn a_damaged_start_blocks_with_the_start_pick_and_builds_machine_origin() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    page.input.machine.damaged = true;
    page.input.start_damaged = true;
    let g = gate_of(&page, &[], &GateFacts::default());
    let found: Vec<_> = g.issues.iter().map(|i| i.cause.clone()).collect();
    assert_eq!(found, [Cause::MachineDamaged, Cause::StartDamaged]);
    assert_eq!(g.issues[1].targets, Targets::StartPick);
    assert_eq!(texts(&page, &g)[1], "Start is damaged. Pick one above.");
    assert!(!g.export_allowed());
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    assert!(job.burns(), "the preview still draws");
    assert_eq!(job.start, Start::MachineOrigin);
}

#[test]
fn the_ring_is_the_digit_the_session_passes() {
    let mut page = Page::new();
    page.fixture(Score, vec![leaf(vec![rect(5.0, 5.0, 10.0, 10.0)])]);
    page.input.fixture_material = Some(BIRCH.to_owned());
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(30.0, 10.0, 10.0, 10.0)])],
    );
    page.input.blocks[1].ring = Ring::Operation(4);
    let birch = material(BIRCH, "Birch 3 mm", &[(SetKind::Score, 12.0, 4000, 1)]);
    let built = build_job(
        &page.input,
        std::slice::from_ref(&birch),
        &mut PlanCache::default(),
    );
    let rings: Vec<_> = built.job.blocks.iter().map(|b| b.ring).collect();
    assert_eq!(rings, [Ring::Fixture, Ring::Operation(4)]);
    let g = gate_of(&page, &[birch], &GateFacts::default());
    let rings: Vec<_> = g.runs.iter().map(|r| r.ring).collect();
    assert_eq!(rings, [Ring::Fixture, Ring::Operation(4)]);
}

#[test]
fn the_summary_count_lines_are_checks() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let ids = job_fixtures::ids(6);
    let facts = GateFacts {
        not_assigned: vec![ids[0], ids[1]],
        hidden_running: vec![ids[2]],
        output_off: vec![ids[3]],
        output_off_items: vec![ids[4], ids[5]],
        not_pictures: Vec::new(),
    };
    let mut g = gate_of(&page, &[], &facts);
    let names = Names::of([(ids[3], "Score 2".to_owned())]);
    // Each count's button selects its items, the Output off line too (`0101` criterion 17, as
    // amended: "the counts and the select buttons are unchanged").
    let targets: Vec<_> = g.issues.iter().map(|i| i.targets.clone()).collect();
    assert_eq!(
        targets,
        [
            Targets::Items(vec![ids[0], ids[1]]),
            Targets::Items(vec![ids[2]]),
            Targets::Items(vec![ids[4], ids[5]]),
        ]
    );
    let lines: Vec<_> = g
        .issues
        .drain(..)
        .map(|i| check_text(&i, &names).whole())
        .collect();
    assert_eq!(
        lines,
        [
            "2 items are not assigned.",
            "1 item runs while hidden.",
            "Score 2 has Output off."
        ]
    );
}

#[test]
fn an_unknown_format_is_never_guessed() {
    // Only GRBL exists; every format is listed once and writes something for a job.
    assert_eq!(OutputFormat::ALL, [OutputFormat::Grbl]);
    assert_eq!(OutputFormat::parse("marlin"), None);
    assert_eq!(OutputFormat::parse("grbl"), Some(OutputFormat::Grbl));
}

#[test]
fn an_item_in_two_operations_runs_in_each() {
    let mut page = Page::new();
    let shape = vec![leaf(vec![line(
        Point::new(10.0, 10.0),
        Point::new(20.0, 10.0),
    )])];
    page.operation(
        Score,
        "Score 1",
        own(Score, 20.0, 3000, 1),
        plain(),
        shape.clone(),
    );
    page.operation(Cut, "Cut 1", own(Cut, 90.0, 300, 1), plain(), shape);
    let job = build_job(&page.input, &[], &mut PlanCache::default()).job;
    let blocks: Vec<usize> = job
        .moves
        .iter()
        .filter(|m| matches!(m.kind, MoveKind::Burn { .. }))
        .map(|m| m.block)
        .collect();
    assert_eq!(blocks, [0, 1]);
}
