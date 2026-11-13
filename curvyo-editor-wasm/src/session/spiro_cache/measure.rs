//! Measures how a Spiro point follows the pointer (`specs/0115` criterion 2): the share of
//! pointer events at which the shown point is not under the pointer, the largest move of the
//! curve between two events, the events at which the curve moves much more than the pointer (not
//! a smooth function of it), curves drawn as their control polygon, curves a reopen would draw
//! differently, and the CPU time of one event. `print_spiro_follow_figures` prints them for every
//! walk (`--ignored --nocapture`, release).
#![allow(clippy::unwrap_used, clippy::cast_precision_loss)]

use super::super::{Session, Tool};
use curvyo_document_core::{NewAnchor, Point};
use curvyo_ui_core::PenMode;
use std::time::Duration;

/// The figures of one walk of the pointer.
#[derive(Debug, Default, Clone)]
struct Figures {
    events: usize,
    stalls: usize,
    max_move: f64,
    /// Events at which the curve moved more than [`NOT_SMOOTH_FACTOR`] times the pointer's step
    /// plus [`NOT_SMOOTH_SLACK_MM`].
    not_smooth: usize,
    /// Events whose curve is the control polygon (every span straight).
    polygons: usize,
    not_canonical: usize,
    times: Vec<Duration>,
}

/// A curve that moves more than this many times the pointer's step...
const NOT_SMOOTH_FACTOR: f64 = 4.0;
/// ...plus this much (mm) between two events is not a smooth function of the pointer.
const NOT_SMOOTH_SLACK_MM: f64 = 2.0;

impl Figures {
    fn share(&self, count: usize) -> f64 {
        count as f64 / self.events.max(1) as f64
    }

    /// The largest and the 95th percentile CPU time of one event.
    fn time(&self) -> (Duration, Duration) {
        let mut times = self.times.clone();
        times.sort();
        let max = times.last().copied().unwrap_or_default();
        let p95 = times
            .get(times.len().saturating_mul(95) / 100)
            .copied()
            .unwrap_or(max);
        (max, p95)
    }
}

/// The CPU time `work` costs this thread.
fn cpu_time_of(work: impl FnOnce()) -> Duration {
    #[cfg(unix)]
    {
        let now = || {
            let t = rustix::time::clock_gettime(rustix::time::ClockId::ThreadCPUTime);
            Duration::new(
                u64::try_from(t.tv_sec).unwrap(),
                u32::try_from(t.tv_nsec).unwrap(),
            )
        };
        let start = now();
        work();
        now().saturating_sub(start)
    }
    #[cfg(not(unix))]
    {
        // Wall time: the figures are only printed here, nothing asserts on them.
        let start = std::time::Instant::now();
        work();
        start.elapsed()
    }
}

/// Points along a curve's cubic spans, 16 per span.
fn samples(anchors: &[NewAnchor]) -> Vec<Point> {
    let mut out = Vec::new();
    for pair in anchors.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let p = [
            a.point,
            Point::new(a.point.x + a.handle_out.x, a.point.y + a.handle_out.y),
            Point::new(b.point.x + b.handle_in.x, b.point.y + b.handle_in.y),
            b.point,
        ];
        for i in 0..16 {
            let t = f64::from(i) / 16.0;
            let rest = 1.0 - t;
            let weights = [
                rest * rest * rest,
                3.0 * rest * rest * t,
                3.0 * rest * t * t,
                t * t * t,
            ];
            out.push(Point::new(
                weights.iter().zip(&p).map(|(w, q)| w * q.x).sum(),
                weights.iter().zip(&p).map(|(w, q)| w * q.y).sum(),
            ));
        }
    }
    if let Some(last) = anchors.last() {
        out.push(last.point);
    }
    out
}

fn reach(a: &[Point], b: &[Point]) -> f64 {
    a.iter()
        .map(|p| {
            b.iter()
                .map(|q| (p.x - q.x).hypot(p.y - q.y))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max)
}

fn hausdorff(a: &[Point], b: &[Point]) -> f64 {
    reach(a, b).max(reach(b, a))
}

/// The pointer positions of a walk along `path` in steps of at most `step` mm.
fn walk(path: &[Point], step: f64) -> Vec<Point> {
    let mut out = vec![path[0]];
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = ((b.x - a.x).hypot(b.y - a.y) / step).ceil().max(1.0) as u32;
        for i in 1..=n {
            let t = f64::from(i) / f64::from(n);
            out.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

fn circle(c: Point, r: f64, from_deg: f64, to_deg: f64) -> Vec<Point> {
    (0..=72)
        .map(|i| {
            let a = (from_deg + (to_deg - from_deg) * f64::from(i) / 72.0).to_radians();
            Point::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
}

fn spiro_pen(placed: &[Point]) -> Session {
    let mut s = Session::new(70);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(PenMode::Spiro));
    for &p in placed {
        s.pointer_hover(p, false, false);
        let _ = s.draw_list();
        s.pointer_down(p, false);
        s.pointer_up(p, false, false);
    }
    s
}

/// The Pen's live Spiro point walks along `path` in steps of `step` mm over `placed` controls;
/// an event is the hover and the frame drawn after it.
fn pen_walk(placed: &[Point], path: &[Point], step: f64) -> Figures {
    let mut s = spiro_pen(placed);
    let mut figures = Figures::default();
    let mut last: Option<(Vec<Point>, Point)> = None;
    for p in walk(path, step) {
        let spent = cpu_time_of(|| {
            s.pointer_hover(p, false, false);
            let _ = s.draw_list();
        });
        let closing = matches!(
            s.pen_target(),
            Some(curvyo_ui_core::PenTarget::Close { .. })
        ) || placed.iter().any(|q| (q.x - p.x).hypot(q.y - p.y) < 1.0);
        let cache = s.spiro_cache.borrow();
        let Some(entry) = cache.preview.as_ref() else {
            continue;
        };
        if closing {
            // The closed preview over the first node is meant to differ (as the tester's walks).
            last = None;
            continue;
        }
        figures.events += 1;
        figures.times.push(spent);
        let live = (entry.controls.len() == placed.len() + 1)
            .then(|| entry.controls.last().unwrap().point);
        if live.is_none_or(|q| (q.x - p.x).hypot(q.y - p.y) > 0.5) {
            figures.stalls += 1;
        }
        let Some(output) = entry.output.as_deref() else {
            figures.polygons += 1;
            last = None;
            continue;
        };
        if output.windows(2).all(|pair| {
            let (a, b) = (pair[0], pair[1]);
            let chord = a.point.vector_to(b.point);
            let off = |v: curvyo_document_core::Vec2| (v.x * chord.y - v.y * chord.x).abs();
            off(a.handle_out) <= 1e-9 * chord.length() && off(b.handle_in) <= 1e-9 * chord.length()
        }) {
            figures.polygons += 1;
        }
        let curve = samples(output);
        if let Ok(fresh) = curvyo_ui_core::evaluate_spiro_nodes(
            &entry.controls,
            entry.closed,
            curvyo_ui_core::SPIRO_TOLERANCE,
        ) && hausdorff(&samples(&fresh), &curve) > 0.01
        {
            figures.not_canonical += 1;
        }
        if let Some((before, at)) = &last {
            let moved = hausdorff(before, &curve);
            figures.max_move = figures.max_move.max(moved);
            let pointer = (at.x - p.x).hypot(at.y - p.y);
            if moved > NOT_SMOOTH_FACTOR * pointer + NOT_SMOOTH_SLACK_MM {
                figures.not_smooth += 1;
            }
        }
        last = Some((curve, p));
    }
    figures
}

const WAVE: [Point; 3] = [
    Point::new(10.0, 20.0),
    Point::new(30.0, 10.0),
    Point::new(50.0, 20.0),
];

const HAIRPIN: [Point; 3] = [
    Point::new(10.0, 10.0),
    Point::new(50.0, 10.0),
    Point::new(20.0, 10.5),
];

fn raster() -> Vec<Point> {
    let mut raster = Vec::new();
    for row in 0..9 {
        let y = -20.0 + 7.5 * f64::from(row);
        let (a, b) = if row % 2 == 0 {
            (0.0, 90.0)
        } else {
            (90.0, 0.0)
        };
        raster.push(Point::new(a, y));
        raster.push(Point::new(b, y));
    }
    raster
}

/// The walks the figures are taken over: (name, placed controls, pointer path).
fn walks() -> Vec<(&'static str, Vec<Point>, Vec<Point>)> {
    vec![
        (
            "wave circle r15",
            WAVE.to_vec(),
            circle(WAVE[2], 15.0, 0.0, 720.0),
        ),
        (
            "wave circle r40",
            WAVE.to_vec(),
            circle(WAVE[1], 40.0, 0.0, 360.0),
        ),
        (
            "wave doubling back",
            WAVE.to_vec(),
            vec![
                Point::new(70.0, 20.0),
                Point::new(30.0, 12.0),
                Point::new(0.0, 30.0),
                Point::new(70.0, 20.0),
            ],
        ),
        (
            "wave circle r30",
            WAVE.to_vec(),
            circle(WAVE[2], 30.0, 0.0, 720.0),
        ),
        ("wave raster", WAVE.to_vec(), raster()),
        (
            "hairpin circle r20",
            HAIRPIN.to_vec(),
            circle(HAIRPIN[2], 20.0, 0.0, 360.0),
        ),
        ("hairpin raster", HAIRPIN.to_vec(), raster()),
    ]
}

/// A wave of `n` placed controls, the pointer circling its last one 12 mm away.
fn wave_of(n: u32) -> (Vec<Point>, Vec<Point>) {
    let placed: Vec<_> = (0..n)
        .map(|i| {
            let x = 5.0 * f64::from(i);
            Point::new(10.0 + x, 100.0 + 15.0 * (x / 15.0).sin())
        })
        .collect();
    let last = *placed.last().unwrap();
    (placed, circle(last, 12.0, 0.0, 360.0))
}

/// Prints the figures of every walk at a fine and a coarse pointer step, and the CPU time of an
/// event for waves of 3, 8 and 20 controls.
#[test]
#[ignore = "measurement, run by hand with --release --nocapture"]
fn print_spiro_follow_figures() {
    for (name, placed, path) in walks() {
        for step in [0.5, 6.7] {
            let f = pen_walk(&placed, &path, step);
            let (max, p95) = f.time();
            println!(
                "{name:20} step {step:3.1}: events {:5} stalls {:5.1}% max move {:6.1} mm not smooth {:5.1}% polygons {:3} not canonical {:3} time max {:5.2} p95 {:5.2} ms",
                f.events,
                100.0 * f.share(f.stalls),
                f.max_move,
                100.0 * f.share(f.not_smooth),
                f.polygons,
                f.not_canonical,
                max.as_secs_f64() * 1e3,
                p95.as_secs_f64() * 1e3,
            );
        }
    }
    for n in [3, 8, 20] {
        let (placed, path) = wave_of(n);
        let f = pen_walk(&placed, &path, 0.5);
        let (max, p95) = f.time();
        println!(
            "wave of {n:2} controls, circle r12: time max {:5.2} p95 {:5.2} ms",
            max.as_secs_f64() * 1e3,
            p95.as_secs_f64() * 1e3,
        );
    }
}

/// `specs/0115` criterion 2 (customer 2026-11-12): on every walk at 0.5 mm steps the Pen's live
/// Spiro point is under the pointer at every event, its curve is never the control polygon,
/// moves much more than the pointer at no more than 1 % of the events (where it turns over
/// behind a neighbour), and what is shown is what a reopen draws. Before (the hold of the first
/// `0115` build): 4.6 % to 69.8 % of the events stalled.
#[test]
fn the_live_spiro_point_follows_the_pointer_and_its_curve_rarely_jumps() {
    for (name, placed, path) in walks() {
        let f = pen_walk(&placed, &path, 0.5);
        assert!(f.events > 200, "{name}: {f:?}");
        assert_eq!(f.stalls, 0, "{name}: {f:?}");
        assert_eq!(f.polygons, 0, "{name}: {f:?}");
        assert!(f.share(f.not_smooth) <= 0.01, "{name}: {f:?}");
        assert_eq!(
            f.not_canonical, 0,
            "{name}: what is shown is what a reopen draws"
        );
    }
}

/// The same at 6.7 mm steps (a quick hand, UX review 2026-11-12): no stalls, and the curve moves
/// much more than the pointer at no more than 6 % of the events.
#[test]
fn a_live_spiro_point_follows_a_quick_pointer() {
    for (name, placed, path) in walks() {
        let f = pen_walk(&placed, &path, 6.7);
        assert_eq!(f.stalls, 0, "{name}: {f:?}");
        assert!(f.share(f.not_smooth) <= 0.06, "{name}: {f:?}");
        assert_eq!(f.not_canonical, 0, "{name}: {f:?}");
    }
}
