//! Tester acceptance for `specs/0115-spiro-smoothing-and-width-handles` criteria 2 and 4 to 7 at
//! the session, and the amended `0110` criterion 8 (a Spiro saved by the `0110` build, format 42,
//! opens and draws). Written from the specification; the Pen walks are those named by criterion 2
//! (`session/spiro_cache/measure.rs`), measured here from the draw list only.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::assert_is_empty
)]

use curvyo_document_core::{
    AnchorId, AnchorKind, CURRENT_FORMAT_VERSION, CurveKind, Document, Length, NewAnchor, NodeId,
    PathSnapshot, Point, StyleEdit, pack, unpack,
};
use curvyo_editor_wasm::{DoubleClickHint, Session, Tool};
use curvyo_render_core::{DrawList, Vertex};
use curvyo_ui_core::{PenMode, PenTarget};

// ---------- helpers ----------

fn artwork(list: &DrawList) -> Vec<Vertex> {
    let end = list.layers().last().copied().unwrap_or(0);
    list.triangles[..end].to_vec()
}

fn overlay(list: &DrawList) -> Vec<Vertex> {
    let end = list.layers().last().copied().unwrap_or(0);
    list.triangles[end..].to_vec()
}

fn saved(session: &Session) -> Document {
    unpack(900, &session.pack("test").unwrap()).unwrap()
}

fn snap(session: &Session, id: NodeId) -> PathSnapshot {
    saved(session).path(id).unwrap()
}

fn click(session: &mut Session, p: Point) {
    session.pointer_down(p, false);
    session.pointer_up(p, false, false);
}

fn hover(session: &mut Session, p: Point) -> DrawList {
    session.pointer_hover(p, false, false);
    session.draw_list()
}

fn dist(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

/// Distance from `p` to the segment `q`-`r`.
fn seg_dist(p: Point, q: Point, r: Point) -> f64 {
    let (dx, dy) = (r.x - q.x, r.y - q.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - q.x) * dx + (p.y - q.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - q.x - t * dx).hypot(p.y - q.y - t * dy)
}

fn edges(tris: &[Vertex]) -> Vec<(Point, Point)> {
    tris.as_chunks::<3>()
        .0
        .iter()
        .flat_map(|t| {
            [
                (t[0].position, t[1].position),
                (t[1].position, t[2].position),
                (t[2].position, t[0].position),
            ]
        })
        .collect()
}

/// Points along every triangle edge at most `step` mm apart.
fn dense(tris: &[Vertex], step: f64) -> Vec<Point> {
    let mut out = Vec::new();
    for (p, q) in edges(tris) {
        let n = (dist(p, q) / step).ceil().max(1.0) as u32;
        for i in 0..=n {
            let f = f64::from(i) / f64::from(n);
            out.push(Point::new(p.x + (q.x - p.x) * f, p.y + (q.y - p.y) * f));
        }
    }
    out
}

/// One direction of the Hausdorff distance from points `a` to the edges `b`, exact per segment;
/// the edges are bucketed in a grid so a frame pair costs milliseconds, not seconds.
fn reach_edges(a: &[Point], b: &[(Point, Point)]) -> f64 {
    if b.is_empty() {
        return if a.is_empty() { 0.0 } else { f64::INFINITY };
    }
    let (mut lo, mut hi) = (
        Point::new(f64::MAX, f64::MAX),
        Point::new(f64::MIN, f64::MIN),
    );
    for &(q, r) in b {
        lo = Point::new(lo.x.min(q.x).min(r.x), lo.y.min(q.y).min(r.y));
        hi = Point::new(hi.x.max(q.x).max(r.x), hi.y.max(q.y).max(r.y));
    }
    let cell = ((hi.x - lo.x).max(hi.y - lo.y) / 256.0).max(1.0);
    let key = |x: f64, y: f64| {
        (
            ((x - lo.x) / cell).floor() as i64,
            ((y - lo.y) / cell).floor() as i64,
        )
    };
    let mut grid: std::collections::HashMap<(i64, i64), Vec<usize>> =
        std::collections::HashMap::new();
    for (i, &(q, r)) in b.iter().enumerate() {
        let (k0, k1) = (
            key(q.x.min(r.x), q.y.min(r.y)),
            key(q.x.max(r.x), q.y.max(r.y)),
        );
        for cx in k0.0..=k1.0 {
            for cy in k0.1..=k1.1 {
                grid.entry((cx, cy)).or_default().push(i);
            }
        }
    }
    let (kmax, kmin) = (key(hi.x, hi.y), (0i64, 0i64));
    a.iter()
        .map(|&p| {
            let c = key(p.x, p.y);
            let mut best = f64::INFINITY;
            let gap = (kmin.0 - c.0)
                .max(c.0 - kmax.0)
                .max(kmin.1 - c.1)
                .max(c.1 - kmax.1)
                .max(0);
            for ring in gap.. {
                for cx in c.0 - ring..=c.0 + ring {
                    for cy in c.1 - ring..=c.1 + ring {
                        if (cx - c.0).abs() != ring && (cy - c.1).abs() != ring {
                            continue;
                        }
                        if let Some(list) = grid.get(&(cx, cy)) {
                            for &i in list {
                                best = best.min(seg_dist(p, b[i].0, b[i].1));
                            }
                        }
                    }
                }
                // Every edge in a farther ring is at least `ring * cell` away.
                let covers_all = c.0 - ring <= kmin.0
                    && c.1 - ring <= kmin.1
                    && c.0 + ring >= kmax.0
                    && c.1 + ring >= kmax.1;
                if best <= ring as f64 * cell || covers_all {
                    break;
                }
            }
            best
        })
        .fold(0.0, f64::max)
}

/// The Hausdorff distance between the edge sets of two drawings (edges sampled every 0.25 mm on
/// one side, exact segments on the other), in mm.
fn exact_distance(a: &[Vertex], b: &[Vertex]) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    if a.is_empty() || b.is_empty() {
        return f64::INFINITY;
    }
    reach_edges(&dense(a, 0.25), &edges(b)).max(reach_edges(&dense(b, 0.25), &edges(a)))
}

/// The Hausdorff distance over triangle vertices only (the `0110` measure before the amendment).
fn vertex_distance(a: &[Vertex], b: &[Vertex]) -> f64 {
    let reach = |a: &[Vertex], b: &[Vertex]| {
        a.iter()
            .map(|p| {
                b.iter()
                    .map(|q| dist(p.position, q.position))
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0.0, f64::max)
    };
    reach(a, b).max(reach(b, a))
}

/// The `0110` stability tester's measure as amended for `0115`: vertices plus edge points 1 mm
/// apart, vertex to vertex.
fn amended_distance(a: &[Vertex], b: &[Vertex]) -> f64 {
    let pts = |t: &[Vertex]| dense(t, 1.0);
    let (a, b) = (pts(a), pts(b));
    let reach = |a: &[Point], b: &[Point]| {
        a.iter()
            .map(|&p| b.iter().map(|&q| dist(p, q)).fold(f64::INFINITY, f64::min))
            .fold(0.0, f64::max)
    };
    reach(&a, &b).max(reach(&b, &a))
}

// ---------- criterion 2: the Pen walks ----------

const STEP: f64 = 0.5;

fn walk(path: &[Point], step: f64) -> Vec<Point> {
    let mut out = vec![path[0]];
    for pair in path.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let n = (dist(a, b) / step).ceil().max(1.0) as u32;
        for i in 1..=n {
            let t = f64::from(i) / f64::from(n);
            out.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

fn circle(c: Point, r: f64, from_deg: f64, to_deg: f64) -> Vec<Point> {
    let steps = 360;
    (0..=steps)
        .map(|i| {
            let a = (from_deg + (to_deg - from_deg) * f64::from(i) / f64::from(steps)).to_radians();
            Point::new(c.x + r * a.cos(), c.y + r * a.sin())
        })
        .collect()
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

fn doubling_back() -> Vec<Point> {
    vec![
        Point::new(70.0, 20.0),
        Point::new(30.0, 12.0),
        Point::new(0.0, 30.0),
        Point::new(70.0, 20.0),
    ]
}

/// The walks of criterion 2: name, placed controls, pointer path.
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
        ("wave doubling back", WAVE.to_vec(), doubling_back()),
        ("wave raster", WAVE.to_vec(), raster()),
        (
            "hairpin circle r20",
            HAIRPIN.to_vec(),
            circle(HAIRPIN[2], 20.0, 0.0, 360.0),
        ),
        ("hairpin raster", HAIRPIN.to_vec(), raster()),
    ]
}

fn spiro_pen(placed: &[Point], peer: u64) -> Session {
    let mut s = Session::new(peer);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(PenMode::Spiro));
    for &p in placed {
        hover(&mut s, p);
        click(&mut s, p);
    }
    s
}

#[derive(Debug, Default)]
struct Figures {
    events: usize,
    stalls: usize,
    /// Worst move between two events by the exact edge measure, where, and the other measures
    /// of that same frame pair (vertex only, amended `0110` measure).
    worst: f64,
    worst_at: Option<Point>,
    worst_vertex: f64,
    worst_amended: f64,
    /// Events at which the curve moved more than [`TURNOVER_MM`]: where it turns over.
    turnovers: usize,
    /// The worst move by the exact edge measure among frame pairs whose bounding boxes moved at
    /// most 10 mm, and the other measures of that pair.
    measured: f64,
    measured_vertex: f64,
    measured_amended: f64,
    /// Events at which the curve moved more than 4 times the pointer's step plus 2 mm: the
    /// figure criterion 2 (amended 2026-11-12) allows at no more than 1 % of the events.
    over_spec: usize,
    /// Events at which the drawing is the control polygon (criterion 2: never).
    polygon: usize,
}

/// A move of the curve between two pointer events above this (mm) is a turnover (`specs/0115`
/// criterion 2 as amended 2026-11-12): the curve is a function of the controls, so where the
/// pointer crosses the line behind a neighbour it flips from one side to the other at once.
const TURNOVER_MM: f64 = 10.0;

impl Figures {
    fn stall_share(&self) -> f64 {
        self.stalls as f64 / self.events.max(1) as f64
    }
}

/// The largest shift of a side of the bounding box between two drawings, in mm: a lower bound
/// of their Hausdorff distance.
fn box_shift(a: &[Vertex], b: &[Vertex]) -> f64 {
    let bb = |v: &[Vertex]| {
        v.iter()
            .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, q| {
                [
                    b[0].min(q.position.x),
                    b[1].min(q.position.y),
                    b[2].max(q.position.x),
                    b[3].max(q.position.y),
                ]
            })
    };
    let (a, b) = (bb(a), bb(b));
    (0..4).map(|i| (a[i] - b[i]).abs()).fold(0.0, f64::max)
}

fn closing(s: &Session, placed: &[Point], p: Point) -> bool {
    matches!(s.pen_target(), Some(PenTarget::Close { .. }))
        || placed.iter().any(|&q| dist(q, p) < 1.0)
}

/// The live Spiro point walks along `path`. A frame is a stall when the drawing passes no closer
/// than 0.5 mm to the pointer: the point stayed behind.
fn pen_walk(placed: &[Point], path: &[Point], peer: u64) -> Figures {
    pen_walk_at(placed, path, STEP, peer)
}

/// The Lines preview of the same placed points and pointer: what the control polygon draws.
fn polygon_frame(placed: &[Point], pointer: Point) -> Vec<Vertex> {
    let mut l = Session::new(99);
    l.set_tool(Tool::Pen);
    assert!(l.set_pen_mode(PenMode::Lines));
    for &p in placed {
        hover(&mut l, p);
        click(&mut l, p);
    }
    hover(&mut l, pointer).triangles
}

/// Whether `frame` is the control polygon of `placed` and `pointer`: every drawn vertex lies
/// within 1 mm of the polygon and the drawing is the Lines preview's within 0.05 mm.
fn is_polygon(frame: &[Vertex], placed: &[Point], pointer: Point) -> bool {
    let mut corners = placed.to_vec();
    corners.push(pointer);
    let near = frame.iter().all(|v| {
        corners
            .windows(2)
            .any(|w| seg_dist(v.position, w[0], w[1]) <= 1.0)
    });
    near && exact_distance(frame, &polygon_frame(placed, pointer)) < 0.05
}

fn pen_walk_at(placed: &[Point], path: &[Point], step: f64, peer: u64) -> Figures {
    let mut s = spiro_pen(placed, peer);
    let mut f = Figures::default();
    let mut last: Option<Vec<Vertex>> = None;
    for p in walk(path, step) {
        let frame = hover(&mut s, p).triangles;
        if closing(&s, placed, p) {
            last = None;
            continue;
        }
        f.events += 1;
        assert!(!frame.is_empty(), "{p:?}: the curve is shown");
        let near = edges(&frame)
            .iter()
            .map(|&(q, r)| seg_dist(p, q, r))
            .fold(f64::INFINITY, f64::min);
        if near > 0.5 {
            f.stalls += 1;
        }
        if is_polygon(&frame, placed, p) {
            f.polygon += 1;
        }
        if let Some(before) = &last {
            // An unchanged frame did not move; a bounding box that moved by more than 10 mm is
            // a jump of at least that much (a lower bound of the Hausdorff distance).
            let bound = box_shift(before, &frame);
            let moved = if *before == frame {
                0.0
            } else if bound > 10.0 {
                bound
            } else {
                exact_distance(before, &frame)
            };
            if moved > TURNOVER_MM {
                f.turnovers += 1;
            }
            if moved > 4.0 * step + 2.0 {
                f.over_spec += 1;
            }
            if moved > f.worst {
                f.worst = moved;
                f.worst_at = Some(p);
                if bound <= 10.0 {
                    f.worst_vertex = vertex_distance(before, &frame);
                    f.worst_amended = amended_distance(before, &frame);
                }
            }
            if bound <= 10.0 && moved > f.measured {
                f.measured = moved;
                f.measured_vertex = vertex_distance(before, &frame);
                f.measured_amended = amended_distance(before, &frame);
            }
        }
        last = Some(frame);
    }
    f
}

/// Criterion 2 (amended 2026-11-12, customer: no stalls): circling the last node 15 mm away
/// twice, the live point is under the pointer at every event, and the curve turns over at most
/// once per round (where the pointer crosses the line behind the last node), by no more than the
/// pointer's distance from that node. Was: at most 12 % stalls and no move above 10 mm (the hold).
#[test]
fn c2_circling_the_last_node_never_stalls_and_turns_over_once_per_round() {
    let (name, placed, path) = walks().swap_remove(0);
    let f = pen_walk(&placed, &path, 70);
    println!("{name}: {f:?} stall {:.1} %", 100.0 * f.stall_share());
    assert!(f.events > 300, "{name}: the walk is measured: {f:?}");
    assert_eq!(f.stalls, 0, "{name}: {f:?}");
    assert_eq!(f.polygon, 0, "{name}: never the control polygon: {f:?}");
    assert!(f.turnovers <= 2, "{name}: two rounds: {f:?}");
    assert!(f.worst <= 15.0, "{name}: {f:?}");
}

/// Criterion 2 (amended): doubling back past the path, no stalls; the curve turns over at most
/// twice (each crossing of the line behind the last node), by no more than 15 mm. Was: at most
/// 20 % stalls and no move above 10 mm.
#[test]
fn c2_doubling_back_never_stalls_and_turns_over_at_most_twice() {
    let (name, placed, path) = walks().swap_remove(2);
    let f = pen_walk(&placed, &path, 71);
    println!("{name}: {f:?} stall {:.1} %", 100.0 * f.stall_share());
    assert!(f.events > 200, "{name}: the walk is measured: {f:?}");
    assert_eq!(f.stalls, 0, "{name}: {f:?}");
    assert_eq!(f.polygon, 0, "{name}: never the control polygon: {f:?}");
    assert!(f.turnovers <= 2, "{name}: {f:?}");
    assert!(f.worst <= 15.0, "{name}: {f:?}");
}

/// Criterion 2 (amended 2026-11-12) as written: on every walk at 0.5 mm steps the point never
/// stalls, the curve is never the control polygon, and it moves more than 4 x 0.5 + 2 = 4 mm
/// between two events at no more than 1 % of them. (The rewrite of 2026-11-13 counted only moves
/// above 10 mm, which is not the spec's figure; restored by the tester.)
#[test]
fn c2_every_walk_follows_the_pointer_and_rarely_turns_over() {
    let mut failures = Vec::new();
    for (n, (name, placed, path)) in walks().into_iter().enumerate() {
        let f = pen_walk(&placed, &path, 80 + n as u64);
        println!(
            "{name}: stall {:.1} % over 4 mm {:.2} % turnovers {:.2} % {f:?}",
            100.0 * f.stall_share(),
            100.0 * f.over_spec as f64 / f.events.max(1) as f64,
            100.0 * f.turnovers as f64 / f.events.max(1) as f64
        );
        if f.stalls > 0 || f.polygon > 0 || f.over_spec as f64 > 0.01 * f.events as f64 {
            failures.push(format!("{name}: {f:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Criterion 2 at 6.7 mm pointer steps: no stalls, never the polygon, and a move of more than
/// 4 x 6.7 + 2 = 28.8 mm at no more than 6 % of the events.
#[test]
fn c2_every_walk_at_coarse_steps_rarely_turns_over() {
    let mut failures = Vec::new();
    for (n, (name, placed, path)) in walks().into_iter().enumerate() {
        let f = pen_walk_at(&placed, &path, 6.7, 140 + n as u64);
        println!(
            "{name} @6.7: stall {:.1} % over 28.8 mm {:.2} % {f:?}",
            100.0 * f.stall_share(),
            100.0 * f.over_spec as f64 / f.events.max(1) as f64
        );
        if f.stalls > 0 || f.polygon > 0 || f.over_spec as f64 > 0.06 * f.events as f64 {
            failures.push(format!("{name}: {f:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Criterion 2: what is shown is what a reopen draws. At points along the walks the pointer
/// clicks; the finished path lies on the curve shown just before the click, and a reopen draws
/// it unchanged.
#[test]
fn c2_a_click_anywhere_on_a_walk_stores_and_reopens_what_was_shown() {
    let mut failures = Vec::new();
    for (n, (name, placed, path)) in walks().into_iter().take(4).enumerate() {
        let events = walk(&path, STEP);
        for k in (37..events.len()).step_by(97) {
            let mut s = spiro_pen(&placed, 100 + n as u64);
            for &p in &events[..k] {
                hover(&mut s, p);
            }
            let p = events[k];
            let shown = hover(&mut s, p).triangles;
            if closing(&s, &placed, p) {
                continue;
            }
            click(&mut s, p);
            s.finish_pen();
            let finished = artwork(&s.draw_list());
            if finished.is_empty() {
                failures.push(format!("{name} at {p:?}: nothing committed"));
                continue;
            }
            let off = reach_edges(&dense(&finished, 0.25), &edges(&shown));
            if off > 0.75 {
                failures.push(format!(
                    "{name} at {p:?}: the finished path is {off:.2} mm off the curve shown"
                ));
            }
            let r = Session::open(901, &s.pack("t").unwrap()).unwrap();
            let again = artwork(&r.draw_list());
            let d = exact_distance(&finished, &again);
            if d > 0.01 {
                failures.push(format!("{name} at {p:?}: a reopen draws {d:.3} mm off"));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Re-check of the amended `distance` of `acceptance_0110_stability_tester.rs`: on the worst
/// frame pairs of the walks, the amended measure stays within 0.6 mm of the exact edge distance
/// (it may only round, never hide a move), and the old vertex-only measure is printed alongside.
#[test]
fn amended_0110_distance_agrees_with_the_exact_edge_distance() {
    for (n, (name, placed, path)) in walks().into_iter().take(4).enumerate() {
        let f = pen_walk(&placed, &path, 120 + n as u64);
        // Since `specs/0115` drops the hold, a turnover moves the bounding box by more than
        // 10 mm, which the walk takes as its move without the exact measure; the re-check runs on
        // the worst pair the exact measure saw.
        println!(
            "{name}: worst exact {:.2} mm, same frames vertex-only {:.2} mm, amended {:.2} mm",
            f.measured, f.measured_vertex, f.measured_amended
        );
        assert!(
            (f.measured - f.measured_amended).abs() <= 0.6,
            "{name}: amended {:.2} vs exact {:.2}",
            f.measured_amended,
            f.measured
        );
    }
}

/// Criterion 2 for dragging: a Node drag of a Spiro control in open space follows the pointer
/// and stores the point where the pointer let go.
#[test]
fn c2_a_node_drag_in_open_space_follows_the_pointer() {
    let points = [
        Point::new(10.0, 20.0),
        Point::new(30.0, 10.0),
        Point::new(50.0, 20.0),
        Point::new(70.0, 10.0),
    ];
    let mut s = spiro_pen(&points, 130);
    s.finish_pen();
    s.set_tool(Tool::Select);
    click(&mut s, points[1]);
    assert_eq!(s.selected_object_count(), 1);
    s.set_tool(Tool::Node);
    let _ = s.draw_list();
    s.pointer_down(points[2], false);
    let path = walk(
        &[points[2], Point::new(52.0, 26.0), Point::new(46.0, 30.0)],
        STEP,
    );
    let mut stalls = 0;
    for &p in &path {
        let frame = artwork(&hover(&mut s, p));
        let near = edges(&frame)
            .iter()
            .map(|&(q, r)| seg_dist(p, q, r))
            .fold(f64::INFINITY, f64::min);
        if near > 0.5 {
            stalls += 1;
            println!("drag stall at {p:?}: curve {near:.2} mm away");
        }
    }
    let end = *path.last().unwrap();
    s.pointer_up(end, false, false);
    assert_eq!(stalls, 0, "the dragged point stays under the pointer");
    let doc = saved(&s);
    let p = doc.path(doc.object_ids()[0]).unwrap();
    assert!(
        dist(p.anchors[2].point, end) < 1e-6,
        "{:?}",
        p.anchors[2].point
    );
}

// ---------- criteria 4 to 7: width points ----------

/// A straight path from (10, 10) to (50, 10) with a 1 mm stroke and width points at the given
/// positions, each 2 mm wide.
fn line_doc(peer: u64, points: &[f64]) -> (Document, NodeId) {
    let doc = Document::new(peer);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(peer, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(peer, 2), Point::new(50.0, 10.0)),
        ],
        false,
    );
    doc.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
        .unwrap();
    for (i, &pos) in points.iter().enumerate() {
        doc.set_width_point(id, AnchorId::new(peer, 10 + i as u64), pos, 2.0)
            .unwrap();
    }
    (doc, id)
}

fn open_selected(doc: &Document, at: Point) -> Session {
    let mut s = Session::open(777, &pack(doc, "test").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Select);
    click(&mut s, at);
    assert_eq!(s.selected_object_count(), 1, "setup: selected");
    s
}

fn width_session(points: &[f64]) -> (Session, NodeId) {
    let (doc, id) = line_doc(60, points);
    let mut s = open_selected(&doc, Point::new(30.0, 12.0 - 2.0));
    assert!(s.begin_width_edit());
    let _ = s.draw_list();
    (s, id)
}

fn widths(s: &Session, id: NodeId) -> Vec<f64> {
    let mut w: Vec<_> = snap(s, id)
        .width_points
        .iter()
        .map(|p| (p.position, p.multiplier))
        .collect();
    w.sort_by(|a, b| a.0.total_cmp(&b.0));
    w.into_iter().map(|p| p.1).collect()
}

const MID: Point = Point::new(30.0, 10.0);

/// Criterion 4: Edit width is offered exactly when one stroked path is selected, in the Select
/// and the Node tool.
#[test]
fn c4_edit_width_is_offered_for_one_stroked_path_in_select_and_node() {
    let (doc, id) = line_doc(61, &[]);
    let other = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(61, 50), Point::new(10.0, 40.0)),
            NewAnchor::corner(AnchorId::new(61, 51), Point::new(50.0, 40.0)),
        ],
        false,
    );
    let bare = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(61, 60), Point::new(10.0, 70.0)),
            NewAnchor::corner(AnchorId::new(61, 61), Point::new(50.0, 70.0)),
        ],
        false,
    );
    doc.edit_style(&[bare], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    let _ = (id, other);
    let mut s = Session::open(778, &pack(&doc, "t").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Select);
    assert!(!s.width_edit_offered(), "nothing selected");
    click(&mut s, MID);
    assert_eq!(s.selected_object_count(), 1);
    assert!(s.width_edit_offered(), "one stroked path in Select");
    s.set_tool(Tool::Node);
    let _ = s.draw_list();
    assert!(s.width_edit_offered(), "one stroked path in Node");
    s.set_tool(Tool::Select);
    s.pointer_down(Point::new(30.0, 40.0), true);
    s.pointer_up(Point::new(30.0, 40.0), true, false);
    assert_eq!(s.selected_object_count(), 2);
    assert!(!s.width_edit_offered(), "two paths");
    let _ = s.group_selection();
    assert!(!s.width_edit_offered(), "a group");
}

#[test]
fn c4_edit_width_is_not_offered_for_a_path_without_stroke() {
    let doc = Document::new(62);
    let bare = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(62, 1), Point::new(10.0, 10.0)),
            NewAnchor::corner(AnchorId::new(62, 2), Point::new(50.0, 10.0)),
            NewAnchor::corner(AnchorId::new(62, 3), Point::new(50.0, 40.0)),
        ],
        true,
    );
    doc.edit_style(&[bare], &StyleEdit::StrokeEnabled(false))
        .unwrap();
    let mut s = Session::open(779, &pack(&doc, "t").unwrap()).unwrap();
    let _ = s.draw_list();
    s.set_tool(Tool::Select);
    click(&mut s, Point::new(30.0, 10.0));
    assert_eq!(
        s.selected_object_count(),
        1,
        "setup: the filled shape is selected"
    );
    assert!(!s.width_edit_offered(), "no stroke, no Edit width");
}

/// Document millimetres per screen pixel at the current zoom.
fn mm_per_px(s: &Session) -> f64 {
    let a = s.screen_to_document(100.0, 100.0);
    let b = s.screen_to_document(200.0, 100.0);
    dist(a, b) / 100.0
}

/// Criterion 5: the hit area of a width point is an 11 px radius, at two zoom levels.
#[test]
fn c5_a_width_point_is_hit_within_11_px() {
    let (mut s, _) = width_session(&[0.5]);
    for zoom_in in [false, true] {
        if zoom_in {
            for _ in 0..4 {
                s.wheel(0.0, -100.0, 300.0, 300.0, false, true);
            }
            let _ = s.draw_list();
        }
        let px = mm_per_px(&s);
        for (dx, dy) in [
            (1.0, 0.0),
            (0.0, 1.0),
            (0.0, -1.0),
            (
                -std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
            ),
        ] {
            let at = |r: f64| Point::new(MID.x + dx * r * px, MID.y + dy * r * px);
            assert!(
                s.width_point_under(at(10.0)),
                "zoom {zoom_in}: 10 px ({dx},{dy}) hits"
            );
            assert!(
                !s.width_point_under(at(12.5)),
                "zoom {zoom_in}: 12.5 px ({dx},{dy}) misses"
            );
        }
    }
}

/// Criterion 5: a width point is drawn 16 px corner to corner (11 before), with white in it and
/// the selected one in a colour that is not white.
#[test]
fn c5_a_width_point_is_drawn_16_px_with_white_and_the_selected_one_in_the_accent() {
    let (mut s, _) = width_session(&[0.5]);
    let px = mm_per_px(&s);
    let near = |list: &DrawList| -> Vec<Vertex> {
        overlay(list)
            .into_iter()
            .filter(|v| (v.position.x - MID.x).abs() <= 1.0 * px)
            .filter(|v| (v.position.y - MID.y).abs() <= 14.0 * px)
            .collect()
    };
    let list = s.draw_list();
    let vs = near(&list);
    let white = |v: &&Vertex| v.color.r > 245 && v.color.g > 245 && v.color.b > 245;
    // The innermost corner (the fill) is 8 px from the centre: 16 px corner to corner; ring and
    // casing (1.5 px each) lie outside it.
    let inner = vs
        .iter()
        .map(|v| (v.position.y - MID.y).abs() / px)
        .fold(f64::INFINITY, f64::min);
    let outer = vs
        .iter()
        .map(|v| (v.position.y - MID.y).abs() / px)
        .fold(0.0, f64::max);
    assert!(
        (inner - 8.0).abs() < 0.1,
        "fill corner at {inner:.2} px, 16 px corner to corner"
    );
    assert!(outer <= 11.1, "ring and casing end at {outer:.2} px");
    let dark = |v: &&Vertex| v.color.r < 90 && v.color.g < 90 && v.color.b < 90;
    assert!(vs.iter().any(|v| white(&v)), "white fill or casing");
    assert!(vs.iter().any(|v| dark(&v)), "a dark ring");
    // Select the point (a click on it), then its fill is the accent.
    click(&mut s, MID);
    let sel = near(&s.draw_list());
    let coloured = sel
        .iter()
        .filter(|v| (v.position.y - MID.y).abs() < 8.1 * px)
        .any(|v| !white(&v) && v.color.b > v.color.r + 100);
    assert!(
        coloured,
        "the selected diamond is filled with the accent colour"
    );
}

/// Criterion 6: a double-click on a width point opens the typed width without a step; the typed
/// value is one undo step.
#[test]
fn c6_double_click_opens_the_typed_width_and_enter_is_one_undo_step() {
    let (mut s, id) = width_session(&[0.25, 0.75]);
    let p = Point::new(40.0, 10.0);
    let before = snap(&s, id);
    let undo = s.undo_len();
    click(&mut s, p);
    click(&mut s, p);
    assert_eq!(
        s.double_click_hint(p, false, false),
        DoubleClickHint::WidthEntry
    );
    assert_eq!(snap(&s, id), before, "opening the field changes nothing");
    assert_eq!(s.undo_len(), undo, "no undo step");
    assert!(
        (s.width_point_mm().unwrap() - 2.0).abs() < 1e-9,
        "the field shows the clicked point's width in mm"
    );
    assert!(s.set_width_point_mm(3.25));
    assert_eq!(s.undo_len(), undo + 1, "Enter is one undo step");
    assert_eq!(widths(&s, id), vec![2.0, 3.25], "only the clicked point");
    let _ = s.undo();
    assert_eq!(snap(&s, id), before);
}

/// Criterion 6: Escape (or a click elsewhere) writes nothing; outside width editing a double
/// click on the path is not a width entry.
#[test]
fn c6_cancel_writes_nothing_and_outside_width_edit_there_is_no_width_entry() {
    let (mut s, id) = width_session(&[0.5]);
    let before = snap(&s, id);
    let undo = s.undo_len();
    click(&mut s, MID);
    click(&mut s, MID);
    assert_eq!(
        s.double_click_hint(MID, false, false),
        DoubleClickHint::WidthEntry
    );
    // The host cancels without calling set_width_point_mm.
    assert_eq!(snap(&s, id), before);
    assert_eq!(s.undo_len(), undo);
    s.end_width_edit();
    let hint = s.double_click_hint(MID, false, false);
    assert_ne!(hint, DoubleClickHint::WidthEntry, "not in width editing");
    assert!(
        !s.width_point_under(MID),
        "no width point is hit outside width editing"
    );
}

/// Criterion 7: 0.5 mm per notch, a burst is one undo step.
#[test]
fn c7_wheel_changes_half_a_millimetre_per_notch_and_a_burst_is_one_step() {
    let (mut s, id) = width_session(&[0.5]);
    let undo = s.undo_len();
    for _ in 0..3 {
        assert!(s.width_wheel(MID, 1.0, false));
    }
    let _ = s.draw_list();
    assert!(s.end_width_wheel());
    assert_eq!(s.undo_len(), undo + 1, "one burst, one undo step");
    assert!(
        (widths(&s, id)[0] - 3.5).abs() < 1e-9,
        "{:?}",
        widths(&s, id)
    );
    let _ = s.undo();
    assert!((widths(&s, id)[0] - 2.0).abs() < 1e-9);
    let _ = s.redo();
    assert!((widths(&s, id)[0] - 3.5).abs() < 1e-9);
}

#[test]
fn c7_shift_wheel_changes_a_tenth_and_the_width_never_goes_below_zero() {
    let (mut s, id) = width_session(&[0.5]);
    assert!(s.width_wheel(MID, 2.0, true));
    assert!(s.end_width_wheel());
    assert!(
        (widths(&s, id)[0] - 2.2).abs() < 1e-9,
        "{:?}",
        widths(&s, id)
    );
    assert!(s.width_wheel(MID, -1.0, false));
    assert!(s.end_width_wheel());
    assert!(
        (widths(&s, id)[0] - 1.7).abs() < 1e-9,
        "{:?}",
        widths(&s, id)
    );
    for _ in 0..10 {
        s.width_wheel(MID, -1.0, false);
    }
    s.end_width_wheel();
    let w = widths(&s, id);
    assert!(w.iter().all(|&m| m >= 0.0), "never below zero: {w:?}");
    assert!(w[0] < 1e-9, "down to zero: {w:?}");
    assert_eq!(s.take_width_edit_notice(), "", "no notice at zero");
}

#[test]
fn c7_the_wheel_away_from_a_width_point_is_not_taken() {
    let (mut s, id) = width_session(&[0.5]);
    let before = snap(&s, id);
    let away = Point::new(30.0, 30.0);
    assert!(!s.width_point_under(away));
    assert!(!s.width_wheel(away, 1.0, false), "the host zooms there");
    assert!(!s.end_width_wheel(), "no burst");
    assert_eq!(snap(&s, id), before);
    assert!(
        s.width_point_under(MID),
        "over the point the host gives the wheel to the session"
    );
}

#[test]
fn c7_moving_to_another_point_mid_burst_makes_two_steps() {
    let (mut s, id) = width_session(&[0.25, 0.75]);
    let (a, b) = (Point::new(20.0, 10.0), Point::new(40.0, 10.0));
    let undo = s.undo_len();
    s.width_wheel(a, 1.0, false);
    s.width_wheel(a, 1.0, false);
    s.width_wheel(b, -1.0, false);
    s.end_width_wheel();
    assert_eq!(widths(&s, id), vec![3.0, 1.5]);
    assert_eq!(s.undo_len(), undo + 2);
    let _ = s.undo();
    assert_eq!(widths(&s, id), vec![3.0, 2.0]);
    let _ = s.undo();
    assert_eq!(widths(&s, id), vec![2.0, 2.0]);
}

/// Criterion 7: Ctrl+Z in the middle of a burst ends the burst first: the document goes back to
/// before the burst, redo brings the burst back, and the steps before it survive.
#[test]
fn c7_undo_mid_burst_undoes_the_burst_and_keeps_the_earlier_history() {
    let (mut s, id) = width_session(&[0.5]);
    click(&mut s, MID);
    assert!(s.set_width_point_mm(2.5));
    let base = s.undo_len();
    s.width_wheel(MID, 1.0, false);
    s.width_wheel(MID, 1.0, false);
    let _ = s.draw_list();
    let _ = s.undo();
    let _ = s.draw_list();
    assert!(
        (widths(&s, id)[0] - 2.5).abs() < 1e-9,
        "back to before the burst: {:?}",
        widths(&s, id)
    );
    assert!(!s.end_width_wheel(), "the burst is over");
    assert!(
        (widths(&s, id)[0] - 2.5).abs() < 1e-9,
        "a late end writes nothing"
    );
    let _ = s.undo();
    assert!(
        (widths(&s, id)[0] - 2.0).abs() < 1e-9,
        "the step before survives"
    );
    let _ = s.redo();
    let _ = s.redo();
    assert!(
        (widths(&s, id)[0] - 3.5).abs() < 1e-9,
        "redo restores the burst: {:?}",
        widths(&s, id)
    );
    assert_eq!(s.undo_len(), base + 1);
}

#[test]
fn c7_a_tool_change_or_done_mid_burst_keeps_the_burst_as_one_step() {
    for done in [false, true] {
        let (mut s, id) = width_session(&[0.5]);
        let undo = s.undo_len();
        s.width_wheel(MID, 1.0, false);
        s.width_wheel(MID, 1.0, false);
        if done {
            s.end_width_edit();
        } else {
            s.set_tool(Tool::Rectangle);
        }
        let _ = s.draw_list();
        assert!(
            (widths(&s, id)[0] - 3.0).abs() < 1e-9,
            "done {done}: {:?}",
            widths(&s, id)
        );
        assert_eq!(s.undo_len(), undo + 1, "done {done}");
        let _ = s.undo();
        assert!((widths(&s, id)[0] - 2.0).abs() < 1e-9, "done {done}");
    }
}

// ---------- amended 0110 criterion 8: a Spiro saved by the 0110 build ----------

/// Spiro paths as the `0110` build writes them (format 42; `curvyo-document-core` is unchanged
/// since `0110`): an open wave, a closed ring, one with a corner and a 13-point zigzag.
fn old_spiro_doc() -> (Document, Vec<(NodeId, Vec<Point>)>) {
    let doc = Document::new(5);
    let mut n = 0u64;
    let mut anchors = |pts: &[Point], corner: Option<usize>| -> Vec<NewAnchor> {
        pts.iter()
            .enumerate()
            .map(|(i, &p)| {
                n += 1;
                let mut a = NewAnchor::corner(AnchorId::new(5, n), p);
                if Some(i) != corner {
                    a.kind = AnchorKind::Symmetric;
                }
                a
            })
            .collect()
    };
    let wave: Vec<_> = [(10.0, 20.0), (30.0, 10.0), (50.0, 20.0), (70.0, 10.0)]
        .map(|(x, y)| Point::new(x, y))
        .to_vec();
    let ring: Vec<_> = [(110.0, 10.0), (140.0, 15.0), (135.0, 45.0), (105.0, 40.0)]
        .map(|(x, y)| Point::new(x, y))
        .to_vec();
    let corner: Vec<_> = [
        (10.0, 60.0),
        (30.0, 50.0),
        (50.0, 70.0),
        (70.0, 55.0),
        (90.0, 65.0),
    ]
    .map(|(x, y)| Point::new(x, y))
    .to_vec();
    let zigzag: Vec<_> = (0..13)
        .map(|i| {
            Point::new(
                10.0 + 4.0 * f64::from(i),
                if i % 2 == 0 { 100.0 } else { 120.0 },
            )
        })
        .collect();
    let mut out = Vec::new();
    for (pts, closed, c) in [
        (&wave, false, None),
        (&ring, true, None),
        (&corner, false, Some(2)),
        (&zigzag, false, None),
    ] {
        let id = doc
            .create_editable_curve(&anchors(pts, c), closed, CurveKind::Spiro)
            .unwrap();
        doc.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(1.0)))
            .unwrap();
        out.push((id, pts.clone()));
    }
    (doc, out)
}

fn covers(tris: &[Vertex], p: Point, r: f64) -> bool {
    edges(tris).iter().any(|&(q, s)| seg_dist(p, q, s) <= r)
}

#[test]
fn old_0110_spiro_document_opens_and_draws_through_its_controls() {
    assert_eq!(
        CURRENT_FORMAT_VERSION, 46,
        "no format bump in 0115 (46 is 0112)"
    );
    let (doc, paths) = old_spiro_doc();
    let bytes = pack(&doc, "old").unwrap();
    let mut s = Session::open(6, &bytes).unwrap();
    let art = artwork(&s.draw_list());
    assert!(!art.is_empty());
    assert_eq!(s.take_unsupported_width_notice(), "");
    assert_eq!(s.take_history_notice(), None);
    let reopened = saved(&s);
    for (id, pts) in &paths {
        let p = reopened.path(*id).unwrap();
        assert_eq!(p.curve_kind, CurveKind::Spiro);
        assert_eq!(p.anchors.len(), pts.len(), "controls unchanged");
        for (a, &q) in p.anchors.iter().zip(pts) {
            assert_eq!(a.point, q, "control unchanged");
        }
        for &q in pts {
            assert!(covers(&art, q, 0.6), "the curve is drawn through {q:?}");
        }
    }
    // Not the polygon: the wave's curve leaves the straight segment between its first controls.
    let mid = Point::new(20.0, 15.0);
    let off = edges(&art)
        .iter()
        .filter(|&&(q, r)| q.x > 18.0 && q.x < 22.0 && r.x > 18.0 && r.x < 22.0 && q.y < 30.0)
        .map(|&(q, r)| seg_dist(mid, q, r))
        .fold(f64::INFINITY, f64::min);
    assert!(
        off > 0.6,
        "the wave is curved, {off:.2} mm off its chord midpoint"
    );
    let again = Session::open(7, &s.pack("t").unwrap()).unwrap();
    assert!(
        exact_distance(&art, &artwork(&again.draw_list())) < 0.01,
        "reopen draws the same"
    );
}

/// The same container as the `0110` build wrote it: `format_version` 42 in the manifest and the
/// JSON mirror (formats 43 and 44 changed nothing about paths).
fn as_format_42(bytes: &[u8]) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    let mut zin = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for i in 0..zin.len() {
        let mut f = zin.by_index(i).unwrap();
        let name = f.name().unwrap().to_string();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        if std::path::Path::new(&name)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("json"))
        {
            let text = String::from_utf8(data).unwrap();
            let v = CURRENT_FORMAT_VERSION;
            let patched = text
                .replace(&format!("\"format_version\":{v}"), "\"format_version\":42")
                .replace(
                    &format!("\"format_version\": {v}"),
                    "\"format_version\": 42",
                );
            assert_ne!(patched, text, "{name} carries the format version");
            data = patched.into_bytes();
        }
        out.start_file(name, opts).unwrap();
        out.write_all(&data).unwrap();
    }
    out.finish().unwrap().into_inner()
}

/// A Spiro file of format 42 (the `0110` build) opens without a notice, keeps its controls, and
/// draws exactly what the same document draws at the current format, also after a reopen.
#[test]
fn a_format_42_spiro_file_opens_and_draws_as_the_current_format() {
    let (doc, paths) = old_spiro_doc();
    let current = pack(&doc, "old").unwrap();
    let old = as_format_42(&current);
    let mut s = Session::open(8, &old).unwrap();
    let art = artwork(&s.draw_list());
    assert_eq!(s.take_unsupported_width_notice(), "");
    assert_eq!(s.take_history_notice(), None);
    let want = artwork(&Session::open(9, &current).unwrap().draw_list());
    assert!(!art.is_empty());
    assert!(art == want, "format 42 draws as the current format");
    let reopened = saved(&s);
    for (id, pts) in &paths {
        let p = reopened.path(*id).unwrap();
        assert_eq!(p.curve_kind, CurveKind::Spiro);
        let got: Vec<_> = p.anchors.iter().map(|a| a.point).collect();
        assert_eq!(&got, pts, "controls unchanged");
    }
    let again = Session::open(10, &s.pack("t").unwrap()).unwrap();
    assert!(
        artwork(&again.draw_list()) == art,
        "a reopen draws the same"
    );
}
