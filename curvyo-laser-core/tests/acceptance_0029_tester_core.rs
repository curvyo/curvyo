//! Tester acceptance cases for `specs/0029-laser-job-preview-and-output` on the public job
//! builder with inputs the tester builds: the view's lengths equal the file's (criterion 9) on
//! random pages, determinism with a cold and a warm plan cache (24, 31), curve flattening within
//! 0.01 mm of the exact Bézier (17), the time formula (10), passes, the feed clamp (21), the
//! S value (25), inner first (22), Head position relative moves (38) and empty pages (7), all read
//! back with the tester's own GRBL reader.
#![allow(
    clippy::many_single_char_names,
    clippy::field_reassign_with_default,
    clippy::assigning_clones,
    clippy::semicolon_if_nothing_returned,
    clippy::cast_lossless,
    clippy::neg_multiply,
    clippy::bool_assert_comparison,
    clippy::assert_is_empty,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines,
    missing_docs
)]

mod tester_grbl_0029;

use curvyo_document_core::{DocumentSize, Length, NodeId, Point, Vec2};
use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::OutputFormat;
use curvyo_laser_core::ProfileValues;
use curvyo_laser_core::job::{
    BlockInput, BlockSource, JobInput, LeafGeometry, LeafInput, LeafOutline, MachineInput,
    PlanCache, Ring, Start, Timeline, build_job, write,
};
use curvyo_laser_core::operations::{KindRead, OperationKind};
use curvyo_laser_core::settings::{OperationOptions, read_settings};
use serde_json::json;
use tester_grbl_0029::{Kind, read};

fn node(n: u64) -> NodeId {
    static IDS: std::sync::OnceLock<Vec<NodeId>> = std::sync::OnceLock::new();
    IDS.get_or_init(|| {
        let d = curvyo_document_core::Document::new(1);
        (0..1100)
            .map(|i| {
                let x = f64::from(i);
                d.create_rect(curvyo_document_core::RectBounds::from_corners(
                    Point::new(x, 0.0),
                    Point::new(x + 1.0, 1.0),
                ))
            })
            .collect()
    })[n as usize]
}

fn corner(x: f64, y: f64) -> (Point, Vec2, Vec2) {
    (Point::new(x, y), Vec2::ZERO, Vec2::ZERO)
}

fn rect(x: f64, y: f64, w: f64, h: f64) -> LeafOutline {
    LeafOutline {
        anchors: vec![
            corner(x, y),
            corner(x + w, y),
            corner(x + w, y + h),
            corner(x, y + h),
        ],
        closed: true,
    }
}

const K: f64 = 0.552_284_749_830_793_4;

/// A circle as four cubic Béziers (handles relative to their anchors).
fn circle(cx: f64, cy: f64, r: f64) -> LeafOutline {
    let h = K * r;
    LeafOutline {
        anchors: vec![
            (
                Point::new(cx + r, cy),
                Vec2::new(0.0, -h),
                Vec2::new(0.0, h),
            ),
            (
                Point::new(cx, cy + r),
                Vec2::new(h, 0.0),
                Vec2::new(-h, 0.0),
            ),
            (
                Point::new(cx - r, cy),
                Vec2::new(0.0, h),
                Vec2::new(0.0, -h),
            ),
            (
                Point::new(cx, cy - r),
                Vec2::new(-h, 0.0),
                Vec2::new(h, 0.0),
            ),
        ],
        closed: true,
    }
}

fn line(x0: f64, y0: f64, x1: f64, y1: f64) -> LeafOutline {
    LeafOutline {
        anchors: vec![corner(x0, y0), corner(x1, y1)],
        closed: false,
    }
}

fn leaf(n: u64, outlines: Vec<LeafOutline>) -> LeafInput {
    LeafInput {
        node: node(n),
        stacking: n as u32,
        geometry: LeafGeometry::Outlines(outlines),
    }
}

fn own(kind: OperationKind, power: f64, speed: u64, passes: u64) -> BlockSource {
    let v = json!({"main": {"power_pct": power, "speed_mm_min": speed, "passes": passes}});
    let settings = read_settings(Some(&v), kind, curvyo_laser_core::DefaultNumbers::BUILT_IN);
    assert!(
        matches!(
            settings,
            curvyo_laser_core::settings::SettingsRead::Stored(_)
        ),
        "{settings:?}"
    );
    BlockSource::Operation {
        settings,
        options: Ok(OperationOptions::default()),
        margin: Length::from_mm(0.0),
        grid: None,
    }
}

fn block(
    id: u64,
    name: &str,
    kind: OperationKind,
    source: BlockSource,
    leaves: Vec<LeafInput>,
) -> BlockInput {
    BlockInput {
        id: node(id),
        // Superseded by 0029 rework (2026-11-17): every block carries its ring (the row digit).
        ring: Ring::Operation(u32::try_from(id).unwrap()),
        name: name.to_owned(),
        kind: KindRead::Known(kind),
        source,
        leaves,
        cut_out: Vec::new(),
    }
}

fn machine(max_feed: u64, max_power: u64) -> MachineInput {
    let mut values = ProfileValues::default();
    values.name = "Tester laser".to_owned();
    values.origin = OriginCorner::BottomLeft;
    values.max_feed = curvyo_laser_core::FeedRate::new(max_feed).unwrap();
    values.max_power = curvyo_laser_core::PowerValue::new(max_power).unwrap();
    values.laser_mode = true;
    MachineInput {
        profile_id: "6c1d2e3f-0000-4000-8000-000000000001".to_owned(),
        values,
        bed: DocumentSize::from_mm(400.0, 300.0),
        damaged: false,
    }
}

fn input(blocks: Vec<BlockInput>) -> JobInput {
    JobInput {
        name: "Laser 1".to_owned(),
        machine: machine(6000, 1000),
        // Superseded by 0029 rework (2026-11-17): Start is one value.
        start: Start::MachineOrigin,
        start_damaged: false,
        user_origin: None,
        fixture_material: None,
        blocks,
    }
}

fn file(i: &JobInput) -> Vec<u8> {
    let built = build_job(i, &[], &mut PlanCache::default());
    write(OutputFormat::Grbl, &built.job)
}

/// A small deterministic generator (xorshift), so the random pages are the same every run.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1_u64 << 53) as f64
    }
}

fn random_page(seed: u64, n: usize) -> JobInput {
    let mut r = Rng(seed);
    let mut cut = Vec::new();
    let mut score = Vec::new();
    for k in 0..n {
        let x = 10.0 + r.next() * 350.0;
        let y = 10.0 + r.next() * 250.0;
        let o = match k % 3 {
            0 => rect(x, y, 1.0 + r.next() * 30.0, 1.0 + r.next() * 30.0),
            1 => circle(x + 15.0, y + 15.0, 0.5 + r.next() * 14.0),
            _ => line(x, y, x + r.next() * 30.0, y + r.next() * 30.0),
        };
        let l = leaf(10 + k as u64, vec![o]);
        if k % 4 == 0 {
            score.push(l)
        } else {
            cut.push(l)
        }
    }
    input(vec![
        block(
            2,
            "Score 1",
            OperationKind::Score,
            own(OperationKind::Score, 20.0, 3000, 1),
            score,
        ),
        block(
            3,
            "Cut 1",
            OperationKind::Cut,
            own(OperationKind::Cut, 90.0, 300, 2),
            cut,
        ),
    ])
}

// ---- criterion 9: the view's lengths are the file's -----------------------------------------------

#[test]
fn c9_view_lengths_equal_file_lengths_on_random_pages() {
    for seed in [1_u64, 7, 99, 12_345, 777_777] {
        let i = random_page(seed, 60);
        let built = build_job(&i, &[], &mut PlanCache::default());
        let t = Timeline::of(&built.job);
        let p =
            read(&write(OutputFormat::Grbl, &built.job)).unwrap_or_else(|e| panic!("{seed}: {e}"));
        assert!(
            (t.burn_mm() - p.burn_mm()).abs() < 0.01,
            "seed {seed}: burn {} vs {}",
            t.burn_mm(),
            p.burn_mm()
        );
        let file_travel = p.travel_mm() + p.off_mm();
        assert!(
            (t.travel_mm() - file_travel).abs() < 0.01
                || (t.travel_mm() - p.travel_mm()).abs() < 0.01,
            "seed {seed}: travel {} vs {} (+{} off)",
            t.travel_mm(),
            p.travel_mm(),
            p.off_mm()
        );
        // The head at the end of the timeline is where the file ends.
        let head = t.head_at(&built.job, 1.0);
        assert!((head.burn_mm - p.burn_mm()).abs() < 0.01);
    }
}

#[test]
fn c37_head_at_half_time_lengths_equal_the_file_up_to_that_move() {
    let i = random_page(42, 30);
    let built = build_job(&i, &[], &mut PlanCache::default());
    let t = Timeline::of(&built.job);
    let p = read(&write(OutputFormat::Grbl, &built.job)).unwrap();
    let head = t.head_at(&built.job, 0.5);
    assert!(
        (head.elapsed_s - t.total_s() / 2.0).abs() < 0.5,
        "{} of {}",
        head.elapsed_s,
        t.total_s()
    );
    // Sum the file's moves by time up to half the total.
    let max_feed = 6000.0;
    let mut time = 0.0;
    let (mut burn, mut travel) = (0.0, 0.0);
    let half = t.total_s() / 2.0;
    for s in &p.segs {
        let feed = match s.kind {
            Kind::Travel => max_feed,
            _ => f64::from(s.f),
        };
        let dt = s.length_mm() / feed * 60.0;
        if time + dt > half {
            let part = (half - time) / dt * s.length_mm();
            if s.kind == Kind::Burn {
                burn += part
            } else {
                travel += part
            }
            break;
        }
        time += dt;
        if s.kind == Kind::Burn {
            burn += s.length_mm()
        } else {
            travel += s.length_mm()
        }
    }
    assert!(
        (head.burn_mm - burn).abs() < 0.01,
        "burn {} vs {}",
        head.burn_mm,
        burn
    );
    assert!(
        (head.travel_mm - travel).abs() < 0.01,
        "travel {} vs {}",
        head.travel_mm,
        travel
    );
}

// ---- criteria 24, 31: determinism -----------------------------------------------------------------

#[test]
fn c24_same_input_same_bytes_with_cold_and_warm_cache() {
    let i = random_page(5, 80);
    let a = file(&i);
    let mut cache = PlanCache::default();
    let b = write(OutputFormat::Grbl, &build_job(&i, &[], &mut cache).job);
    let c = write(OutputFormat::Grbl, &build_job(&i, &[], &mut cache).job);
    assert_eq!(a, b);
    assert_eq!(b, c, "a warm cache writes the same bytes");
    read(&a).unwrap();
}

#[test]
fn c22_order_does_not_depend_on_the_input_order_of_equal_leaves() {
    // Stacking breaks ties, not the vector order: the same leaves in reverse give the same file.
    let mut i = random_page(11, 40);
    let a = file(&i);
    for b in &mut i.blocks {
        b.leaves.reverse();
    }
    assert_eq!(
        a,
        file(&i),
        "the order is by geometry and stacking, not by input order"
    );
}

// ---- criterion 17: flattening ---------------------------------------------------------------------

fn bezier(p0: Point, p1: Point, p2: Point, p3: Point, t: f64) -> (f64, f64) {
    let u = 1.0 - t;
    let x = u * u * u * p0.x + 3.0 * u * u * t * p1.x + 3.0 * u * t * t * p2.x + t * t * t * p3.x;
    let y = u * u * u * p0.y + 3.0 * u * u * t * p1.y + 3.0 * u * t * t * p2.y + t * t * t * p3.y;
    (x, y)
}

fn dist_to_seg(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0)
    };
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

#[test]
fn c17_every_point_of_the_exact_curve_lies_within_0_01_mm_of_the_written_segments() {
    for r in [0.3, 2.0, 25.0, 140.0] {
        let (cx, cy) = (200.0, 150.0);
        let c = circle(cx, cy, r);
        let i = input(vec![block(
            3,
            "Cut 1",
            OperationKind::Cut,
            own(OperationKind::Cut, 50.0, 600, 1),
            vec![leaf(10, vec![c.clone()])],
        )]);
        let p = read(&file(&i)).unwrap();
        // Back to page mm (bottom-left: page y = 300 - machine y).
        let segs: Vec<((f64, f64), (f64, f64))> = p
            .segs
            .iter()
            .filter(|s| s.kind == Kind::Burn)
            .map(|s| {
                let f = |q: (i64, i64)| (q.0 as f64 / 1000.0, 300.0 - q.1 as f64 / 1000.0);
                (f(s.from), f(s.to))
            })
            .collect();
        assert!(!segs.is_empty());
        let n = c.anchors.len();
        let mut worst: f64 = 0.0;
        for k in 0..n {
            let (a0, _, out) = c.anchors[k];
            let (a1, inn, _) = c.anchors[(k + 1) % n];
            let p1 = Point::new(a0.x + out.x, a0.y + out.y);
            let p2 = Point::new(a1.x + inn.x, a1.y + inn.y);
            for j in 0..=400 {
                let q = bezier(a0, p1, p2, a1, f64::from(j) / 400.0);
                let d = segs
                    .iter()
                    .map(|(a, b)| dist_to_seg(q, *a, *b))
                    .fold(f64::INFINITY, f64::min);
                worst = worst.max(d);
            }
        }
        // And every written vertex lies near the curve (the segments do not wander off).
        for (a, _) in &segs {
            let d = (a.0 - cx).hypot(a.1 - cy) - r;
            assert!(
                d.abs() <= 0.01 + r * 0.000_3,
                "r {r}: vertex off the curve by {d}"
            );
        }
        assert!(worst <= 0.01, "r {r}: deviation {worst} mm");
    }
}

// ---- criterion 10: the time estimate --------------------------------------------------------------

#[test]
fn c10_estimate_is_burn_over_feed_times_passes_plus_travel_over_max_feed() {
    // A 100 mm line at 600 mm/min, 2 passes.
    let i = input(vec![block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 2),
        vec![leaf(10, vec![line(50.0, 150.0, 150.0, 150.0)])],
    )]);
    let built = build_job(&i, &[], &mut PlanCache::default());
    let t = Timeline::of(&built.job);
    let p = read(&write(OutputFormat::Grbl, &built.job)).unwrap();
    assert!(
        (p.burn_mm() - 200.0).abs() < 0.01,
        "two passes of 100 mm: {}",
        p.burn_mm()
    );
    let expect = 200.0 / 600.0 * 60.0 + p.travel_mm() / 6000.0 * 60.0;
    assert!(
        (t.total_s() - expect).abs() < 0.01,
        "{} vs {expect}",
        t.total_s()
    );
}

// ---- criterion 21: feed clamp and passes ----------------------------------------------------------

#[test]
fn c21_speed_above_max_feed_writes_the_max_feed() {
    let i = input(vec![block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 20_000, 3),
        vec![leaf(10, vec![rect(10.0, 10.0, 10.0, 10.0)])],
    )]);
    let p = read(&file(&i)).unwrap();
    assert!(
        p.segs
            .iter()
            .filter(|s| s.kind == Kind::Burn)
            .all(|s| s.f == 6000),
        "F clamped to 6000"
    );
    assert!((p.burn_mm() - 120.0).abs() < 0.01, "3 passes of 40 mm");
}

#[test]
fn c25_s_max_255_and_power_rounding() {
    let mut i = input(vec![block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        vec![leaf(10, vec![line(10.0, 10.0, 20.0, 10.0)])],
    )]);
    i.machine = machine(6000, 255);
    let p = read(&file(&i)).unwrap();
    // 50 % of 255 = 127.5 -> 128
    assert_eq!(p.segs.iter().find(|s| s.kind == Kind::Burn).unwrap().s, 128);
}

// ---- criterion 22: nesting to any depth -----------------------------------------------------------

#[test]
fn c22_nested_squares_run_innermost_first_whatever_the_stacking() {
    let outlines = [
        rect(10.0, 10.0, 200.0, 200.0),
        rect(20.0, 20.0, 180.0, 180.0),
        rect(40.0, 40.0, 100.0, 100.0),
        rect(60.0, 60.0, 10.0, 10.0),
    ];
    let leaves: Vec<LeafInput> = outlines
        .iter()
        .enumerate()
        .map(|(k, o)| leaf(10 + k as u64, vec![o.clone()]))
        .collect();
    let i = input(vec![block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        leaves,
    )]);
    let p = read(&file(&i)).unwrap();
    let runs = p.runs();
    assert_eq!(runs.len(), 4);
    let w = |r: &tester_grbl_0029::Run| {
        r.points.iter().map(|q| q.0).max().unwrap() - r.points.iter().map(|q| q.0).min().unwrap()
    };
    let widths: Vec<i64> = runs.iter().map(w).collect();
    assert_eq!(
        widths,
        [10_000, 100_000, 180_000, 200_000],
        "innermost first"
    );
}

#[test]
fn c22_nearest_next_among_equal_depth() {
    // Three separate squares on a line; from the bottom-left origin the nearest runs first.
    let leaves = vec![
        leaf(10, vec![rect(300.0, 280.0, 5.0, 5.0)]),
        leaf(11, vec![rect(10.0, 280.0, 5.0, 5.0)]),
        leaf(12, vec![rect(150.0, 280.0, 5.0, 5.0)]),
    ];
    let i = input(vec![block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        leaves,
    )]);
    let p = read(&file(&i)).unwrap();
    let xs: Vec<i64> = p
        .runs()
        .iter()
        .map(|r| r.points.iter().map(|q| q.0).min().unwrap())
        .collect();
    assert_eq!(xs, [10_000, 150_000, 300_000]);
}

// ---- criterion 38: Head position ------------------------------------------------------------------

#[test]
fn c38_head_position_relative_moves_sum_to_zero_and_match_the_absolute_shape() {
    let mut i = random_page(3, 20);
    let abs = read(&file(&i)).unwrap();
    i.start = Start::HeadPosition;
    let rel = read(&file(&i)).unwrap();
    assert!(rel.relative);
    assert_eq!(
        rel.end,
        (0, 0),
        "the end returns to the start exactly (no drift)"
    );
    assert!((rel.burn_mm() - abs.burn_mm()).abs() < 0.001, "same burns");
    // Same burn shapes, translated.
    let ra = abs.runs();
    let rr = rel.runs();
    assert_eq!(ra.len(), rr.len());
    let d = (
        ra[0].points[0].0 - rr[0].points[0].0,
        ra[0].points[0].1 - rr[0].points[0].1,
    );
    for (a, b) in ra.iter().zip(&rr) {
        for (p, q) in a.points.iter().zip(&b.points) {
            assert_eq!((p.0 - q.0, p.1 - q.1), d, "translated by one vector");
        }
    }
    // Superseded by 0029 rework (2026-11-17): Home, then machine origin is one value; identical
    // to abs after the $H.
    i.start = Start::HomeThenOrigin;
    let home = read(&file(&i)).unwrap();
    assert!(home.homes && !home.relative);
    assert_eq!(&home.commands[1..], &abs.commands[..]);
}

// ---- criterion 7: nothing runs --------------------------------------------------------------------

#[test]
fn c7_an_empty_job_burns_nothing() {
    let i = input(vec![block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        vec![],
    )]);
    let built = build_job(&i, &[], &mut PlanCache::default());
    assert!(!built.job.burns());
    let t = Timeline::of(&built.job);
    assert_eq!(t.burn_mm(), 0.0);
}

// ---- criterion 33: unknown kinds run nothing ------------------------------------------------------

#[test]
fn c33_unknown_kind_and_damaged_settings_run_nothing() {
    let mut unknown = block(
        4,
        "Weird 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        vec![leaf(20, vec![rect(50.0, 50.0, 5.0, 5.0)])],
    );
    unknown.kind = KindRead::Unknown;
    let damaged = BlockInput {
        source: BlockSource::Operation {
            settings: read_settings(
                Some(&json!("not a map")),
                OperationKind::Cut,
                curvyo_laser_core::DefaultNumbers::BUILT_IN,
            ),
            options: Ok(OperationOptions::default()),
            margin: Length::from_mm(0.0),
            grid: None,
        },
        ..block(
            5,
            "Cut 2",
            OperationKind::Cut,
            own(OperationKind::Cut, 50.0, 600, 1),
            vec![leaf(21, vec![rect(80.0, 50.0, 5.0, 5.0)])],
        )
    };
    let good = block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        vec![leaf(10, vec![rect(10.0, 10.0, 10.0, 10.0)])],
    );
    let i = input(vec![unknown, damaged, good]);
    let p = read(&file(&i)).unwrap();
    assert!(
        (p.burn_mm() - 40.0).abs() < 0.01,
        "only Cut 1 burns: {}",
        p.burn_mm()
    );
    let names: Vec<String> = p.header_runs().into_iter().map(|r| r.1).collect();
    assert_eq!(names, ["Cut 1"], "neither is listed as a run");
}

// ---- edge: very long and non-ASCII names ----------------------------------------------------------

#[test]
fn c24_long_and_non_ascii_names_stay_ascii_lines() {
    let long = "Ä".repeat(300);
    let mut i = input(vec![block(
        3,
        &format!("Schnitt {long}"),
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        vec![leaf(10, vec![rect(10.0, 10.0, 10.0, 10.0)])],
    )]);
    i.name = format!("Läser 🙂 {long}");
    i.machine.values.name = "Maschine ü".to_owned();
    let bytes = file(&i);
    let p = read(&bytes).unwrap(); // ASCII and LF enforced by the reader
    for h in String::from_utf8(bytes)
        .unwrap()
        .lines()
        .filter(|l| l.starts_with(';'))
    {
        assert!(h.len() <= 80, "comment line of {} characters", h.len());
    }
    assert_eq!(p.header_runs().len(), 1);
}

// ---- edge: geometry outside the bed is still written as it stands --------------------------------

#[test]
fn c4b_the_writer_never_moves_geometry_into_the_bed() {
    let i = input(vec![block(
        3,
        "Cut 1",
        OperationKind::Cut,
        own(OperationKind::Cut, 50.0, 600, 1),
        vec![leaf(10, vec![line(390.0, 10.0, 450.0, 10.0)])],
    )]);
    let p = read(&file(&i)).unwrap();
    let b = p.burn_box().unwrap();
    assert_eq!(
        b.2, 450_000,
        "written as it stands (the gate blocks the export, not the writer)"
    );
}

// ---- edge: 1,000 items ----------------------------------------------------------------------------

#[test]
fn c36_a_thousand_items_read_back_and_lengths_agree() {
    let i = random_page(2024, 1000);
    let built = build_job(&i, &[], &mut PlanCache::default());
    let t = Timeline::of(&built.job);
    let p = read(&write(OutputFormat::Grbl, &built.job)).unwrap();
    assert!((t.burn_mm() - p.burn_mm()).abs() < 0.01);
    assert_eq!(p.runs().len() >= 1000, true);
}

// ---- rework 2026-11-17: a damaged Start (criterion 4 e) --------------------------------------------

#[test]
fn rework_c4e_a_damaged_start_blocks_with_the_start_pick_and_builds_as_machine_origin() {
    use curvyo_laser_core::job::{Cause, GateFacts, Names, Severity, Targets, check_text, gate};
    let mut i = random_page(5, 6);
    let clean = file(&i);
    let built = build_job(&i, &[], &mut PlanCache::default());
    let g = gate(&i, &built, &Timeline::of(&built.job), &GateFacts::default());
    assert!(!g.has(&Cause::StartDamaged));
    i.start_damaged = true;
    // The stored value is read as Machine origin; the file it would write is not guessed.
    assert_eq!(i.start, Start::MachineOrigin);
    assert_eq!(file(&i), clean, "the job itself is Machine origin's");
    let built = build_job(&i, &[], &mut PlanCache::default());
    let g = gate(&i, &built, &Timeline::of(&built.job), &GateFacts::default());
    assert!(g.has(&Cause::StartDamaged));
    assert!(!g.export_allowed(), "a block disables Export");
    let issue = g
        .issues
        .iter()
        .find(|x| x.cause == Cause::StartDamaged)
        .unwrap();
    assert_eq!(issue.cause.severity(), Severity::Block);
    assert_eq!(issue.targets, Targets::StartPick);
    let text = check_text(issue, &Names::of(std::iter::empty()));
    assert_eq!(text.whole(), "Start is damaged. Pick one above.");
    assert_eq!(text.button, "Start");
    // Blocks come before warns, and 4 e after the other blocks of criterion 4.
    let sev: Vec<Severity> = g.issues.iter().map(|x| x.cause.severity()).collect();
    let first_warn = sev
        .iter()
        .position(|s| *s == Severity::Warn)
        .unwrap_or(sev.len());
    assert!(sev[first_warn..].iter().all(|s| *s == Severity::Warn));
}

#[test]
fn rework_c38_every_start_has_a_file_and_only_home_then_origin_homes() {
    let mut i = random_page(9, 8);
    for start in Start::ALL {
        i.start = start;
        let text = String::from_utf8(file(&i)).unwrap();
        let cmds: Vec<&str> = text
            .lines()
            .filter(|l| !l.starts_with(';') && !l.is_empty())
            .collect();
        assert_eq!(
            cmds[0] == "$H",
            start == Start::HomeThenOrigin,
            "{start:?}: {}",
            cmds[0]
        );
        assert_eq!(
            text.matches("$H").count(),
            usize::from(start == Start::HomeThenOrigin)
        );
        let mode = if start == Start::HeadPosition {
            "G91"
        } else {
            "G90"
        };
        assert!(cmds.contains(&mode), "{start:?} writes {mode}");
        assert!(
            !cmds
                .iter()
                .any(|c| c.starts_with("G10") || c.starts_with("G92") || c.starts_with("G54")),
            "{start:?}: no work offset"
        );
        let p = read(text.as_bytes()).unwrap();
        assert_eq!(p.relative, start == Start::HeadPosition);
        assert_eq!(p.homes, start == Start::HomeThenOrigin);
    }
}
