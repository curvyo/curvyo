//! Independent acceptance tests for `specs/0118-machine-control` slice 2b (criteria 22 to 25 and
//! 28, laser-core part), written from the specification and `adrs.md` F1 before reading the
//! implementation. They read only the public text of the frame and of the job: the frame's
//! points are checked against the burn moves of the same job, so no golden is needed here.
#![allow(clippy::unwrap_used, clippy::missing_panics_doc, clippy::expect_used)]

mod job_fixtures;

use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::job::{FrameShape, PlanCache, Start, build_job, write, write_frame};
use curvyo_laser_core::operations::OperationKind::Cut;
use job_fixtures::{Page, leaf, own, plain, rect};

/// One G1 move read from GRBL text: its start, its end, and whether it burns (`S` above 0).
#[derive(Debug, Clone, Copy)]
struct Move {
    from: (f64, f64),
    to: (f64, f64),
    burns: bool,
}

/// Reads the G0/G1 moves of `text`. `G91` switches to relative moves. The spindle (`M3`/`M4` on,
/// `M5` off) and the power `S` are modal, as GRBL keeps them: a G1 without an `S` word keeps the
/// last one. A move burns when it is a G1 with the laser on and a power above 0.
fn moves(text: &str) -> Vec<Move> {
    let mut out = Vec::new();
    let mut cur = (0.0, 0.0);
    let mut relative = false;
    let mut on = false;
    let mut power = 0.0_f64;
    for line in text.lines() {
        if line.starts_with(';') {
            continue;
        }
        if line.starts_with("G91") {
            relative = true;
        }
        if line.starts_with("G90") {
            relative = false;
        }
        if line.starts_with("M5") {
            on = false;
        }
        if line.starts_with("M3") || line.starts_with("M4") {
            on = true;
        }
        let is_move = line.starts_with("G0 ") || line.starts_with("G1 ");
        let mut to = cur;
        for word in line.split_whitespace().skip(1) {
            let (key, value) = word.split_at(1);
            match key {
                "X" if is_move => {
                    let v: f64 = value.parse().unwrap();
                    to.0 = if relative { cur.0 + v } else { v };
                }
                "Y" if is_move => {
                    let v: f64 = value.parse().unwrap();
                    to.1 = if relative { cur.1 + v } else { v };
                }
                "S" => power = value.parse().unwrap(),
                _ => {}
            }
        }
        if !is_move {
            continue;
        }
        out.push(Move {
            from: cur,
            to,
            burns: line.starts_with("G1 ") && on && power > 0.0,
        });
        cur = to;
    }
    out
}

/// The box of every burn move, both ends of each.
fn burn_box(text: &str) -> ((f64, f64), (f64, f64)) {
    let mut lo = (f64::MAX, f64::MAX);
    let mut hi = (f64::MIN, f64::MIN);
    for m in moves(text).iter().filter(|m| m.burns) {
        for p in [m.from, m.to] {
            lo = (lo.0.min(p.0), lo.1.min(p.1));
            hi = (hi.0.max(p.0), hi.1.max(p.1));
        }
    }
    (lo, hi)
}

/// The points a frame traces, in order: the end of each move (the head starts the frame at its
/// first move, so the first point is the first move's end).
fn traced(frame: &str) -> Vec<(f64, f64)> {
    moves(frame).iter().map(|m| m.to).collect()
}

fn near(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
}

/// The job as the app writes it, and the frame of the same input.
fn texts(page: &Page, shape: FrameShape) -> (String, String) {
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let format = page.input.machine.values.output_format;
    let job = String::from_utf8(write(format, &built.job)).unwrap();
    let frame = String::from_utf8(write_frame(format, &built.job, shape).unwrap()).unwrap();
    (job, frame)
}

/// Two rectangles apart, so the box and the hull differ.
fn two_rects(corner: OriginCorner, start: Start) -> Page {
    let mut page = Page::new().corner(corner).start(start);
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![
            rect(20.0, 20.0, 30.0, 20.0),
            rect(100.0, 60.0, 40.0, 30.0),
        ])],
    );
    page
}

/// Criterion 25 and F1: `M5` first, only `G1 ... S0` moves with a feed, no `G0`, `M3`, `M4`, `$`,
/// no `S` above 0 anywhere.
fn laser_off(frame: &str) {
    let code: Vec<&str> = frame.lines().filter(|l| !l.starts_with(';')).collect();
    let m5 = code.iter().position(|l| *l == "M5").expect("an M5 line");
    let first = code
        .iter()
        .position(|l| l.starts_with("G1 "))
        .expect("a G1");
    assert!(m5 < first, "M5 comes before the first move:\n{frame}");
    for line in &code {
        assert!(!line.starts_with("G0"), "no G0 (it ignores F): {line}");
        assert!(!line.contains("M3") && !line.contains("M4"), "{line}");
        assert!(
            !line.contains('$'),
            "a frame never carries a $ word: {line}"
        );
        assert!(!line.contains("G10") && !line.contains("G92"), "{line}");
        if line.starts_with("G1 ") {
            assert!(line.ends_with(" S0"), "every move is S0: {line}");
            assert!(line.contains(" F"), "every move has a feed: {line}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 23: Rectangle is the bounding box of every burn move, for all four corners
// ---------------------------------------------------------------------------------------------

#[test]
fn rectangle_traces_the_box_of_the_burn_moves_for_every_origin_corner() {
    for corner in [
        OriginCorner::BottomLeft,
        OriginCorner::BottomRight,
        OriginCorner::TopLeft,
        OriginCorner::TopRight,
    ] {
        let page = two_rects(corner, Start::MachineOrigin);
        let (job, frame) = texts(&page, FrameShape::Rectangle);
        laser_off(&frame);
        let (lo, hi) = burn_box(&job);
        let pts = traced(&frame);
        let flo = (
            pts.iter().map(|p| p.0).fold(f64::MAX, f64::min),
            pts.iter().map(|p| p.1).fold(f64::MAX, f64::min),
        );
        let fhi = (
            pts.iter().map(|p| p.0).fold(f64::MIN, f64::max),
            pts.iter().map(|p| p.1).fold(f64::MIN, f64::max),
        );
        assert!(
            near(flo, lo) && near(fhi, hi),
            "{corner:?}: frame {flo:?}..{fhi:?} burn {lo:?}..{hi:?}\n{frame}"
        );
    }
}

#[test]
fn rectangle_is_four_corners_and_starts_at_the_box_corner_nearest_the_machine_origin() {
    let page = two_rects(OriginCorner::BottomLeft, Start::MachineOrigin);
    let (job, frame) = texts(&page, FrameShape::Rectangle);
    let (lo, hi) = burn_box(&job);
    let pts = traced(&frame);
    // The traced outline is the rectangle: every point is one of its four corners.
    for p in &pts {
        let on_x = near((p.0, 0.0), (lo.0, 0.0)) || near((p.0, 0.0), (hi.0, 0.0));
        let on_y = near((0.0, p.1), (0.0, lo.1)) || near((0.0, p.1), (0.0, hi.1));
        assert!(on_x && on_y, "{p:?} is not a corner of {lo:?}..{hi:?}");
    }
    let first = pts[0];
    let nearest = [(lo.0, lo.1), (hi.0, lo.1), (lo.0, hi.1), (hi.0, hi.1)]
        .into_iter()
        .min_by(|a, b| {
            (a.0 * a.0 + a.1 * a.1)
                .partial_cmp(&(b.0 * b.0 + b.1 * b.1))
                .unwrap()
        })
        .unwrap();
    assert!(
        near(first, nearest),
        "starts at {first:?}, the corner nearest the origin is {nearest:?}"
    );
}

#[test]
fn rectangle_ignores_travel_moves() {
    // The job travels from the origin to the first rectangle: the box must not reach back to it.
    let page = two_rects(OriginCorner::BottomLeft, Start::MachineOrigin);
    let (job, frame) = texts(&page, FrameShape::Rectangle);
    assert!(
        job.lines().any(|l| l.starts_with("G0 ")),
        "the job has travel moves"
    );
    let (lo, _) = burn_box(&job);
    let pts = traced(&frame);
    let min_x = pts.iter().map(|p| p.0).fold(f64::MAX, f64::min);
    assert!(
        (min_x - lo.0).abs() < 0.01,
        "the travel does not count: {min_x} vs {}",
        lo.0
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 23: Rubber band is the convex hull of the burn moves
// ---------------------------------------------------------------------------------------------

fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

#[test]
fn rubber_band_is_a_counter_clockwise_convex_polygon_holding_every_burn_point() {
    for corner in [
        OriginCorner::BottomLeft,
        OriginCorner::BottomRight,
        OriginCorner::TopLeft,
        OriginCorner::TopRight,
    ] {
        let page = two_rects(corner, Start::MachineOrigin);
        let (job, frame) = texts(&page, FrameShape::RubberBand);
        laser_off(&frame);
        let pts = traced(&frame);
        // The last point returns to the first; the polygon is the points without it.
        assert!(
            near(pts[0], *pts.last().unwrap()),
            "the outline closes on its first point"
        );
        let poly = &pts[..pts.len() - 1];
        let n = poly.len();
        assert!(n >= 3, "a hull has at least three vertices: {poly:?}");
        // Counter-clockwise and convex: every turn is a left turn.
        for i in 0..n {
            let c = cross(poly[i], poly[(i + 1) % n], poly[(i + 2) % n]);
            assert!(
                c > -1e-6,
                "{corner:?}: the turn at {i} is a right turn: {poly:?}"
            );
        }
        let area2: f64 = (0..n)
            .map(|i| poly[i].0 * poly[(i + 1) % n].1 - poly[(i + 1) % n].0 * poly[i].1)
            .sum();
        assert!(
            area2 > 0.0,
            "{corner:?}: counter-clockwise in machine coordinates"
        );
        // Every burn point lies inside or on the hull.
        for m in moves(&job).iter().filter(|m| m.burns) {
            for p in [m.from, m.to] {
                for i in 0..n {
                    let c = cross(poly[i], poly[(i + 1) % n], p);
                    assert!(c > -0.01, "{corner:?}: burn point {p:?} outside the hull");
                }
            }
        }
        // The first point is the hull vertex nearest the machine origin.
        let d = |p: (f64, f64)| p.0 * p.0 + p.1 * p.1;
        let nearest = poly
            .iter()
            .copied()
            .min_by(|a, b| d(*a).partial_cmp(&d(*b)).unwrap())
            .unwrap();
        assert!(
            near(pts[0], nearest),
            "{corner:?}: starts at {:?}, nearest {nearest:?}",
            pts[0]
        );
    }
}

#[test]
fn rubber_band_is_the_same_shape_as_the_rectangle_for_a_single_rectangle() {
    let mut page = Page::new();
    page.operation(
        Cut,
        "Cut 1",
        own(Cut, 90.0, 300, 1),
        plain(),
        vec![leaf(vec![rect(20.0, 20.0, 30.0, 20.0)])],
    );
    let (_, rect_frame) = texts(&page, FrameShape::Rectangle);
    let (_, band_frame) = texts(&page, FrameShape::RubberBand);
    let a = traced(&rect_frame);
    let b = traced(&band_frame);
    assert_eq!(
        a.len(),
        b.len(),
        "a rectangle is its own hull: {a:?} / {b:?}"
    );
    for p in &b {
        assert!(
            a.iter().any(|q| near(*p, *q)),
            "{p:?} is a corner of the rectangle frame"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Criterion 24: Machine origin and Home then origin trace the same absolute outline; Head
// position is relative and returns to its start
// ---------------------------------------------------------------------------------------------

#[test]
fn home_then_origin_traces_the_same_points_and_never_homes() {
    let a = two_rects(OriginCorner::BottomLeft, Start::MachineOrigin);
    let b = two_rects(OriginCorner::BottomLeft, Start::HomeThenOrigin);
    let (_, fa) = texts(&a, FrameShape::Rectangle);
    let (_, fb) = texts(&b, FrameShape::Rectangle);
    assert!(!fb.contains("$H"), "the frame never homes");
    assert_eq!(
        traced(&fa),
        traced(&fb),
        "the frame of Home then origin is the origin's frame"
    );
}

#[test]
fn head_position_frame_is_relative_and_comes_back_to_where_it_started() {
    let page = two_rects(OriginCorner::BottomLeft, Start::HeadPosition);
    let (job, frame) = texts(&page, FrameShape::Rectangle);
    laser_off(&frame);
    assert!(
        frame.lines().any(|l| l == "G91"),
        "relative moves for Head position:\n{frame}"
    );
    let pts = traced(&frame);
    assert!(
        near(*pts.last().unwrap(), (0.0, 0.0)),
        "back at the head start (relative origin): {pts:?}"
    );
    let (lo, hi) = burn_box(&job);
    let (flo, fhi) = (
        (
            pts.iter().map(|p| p.0).fold(f64::MAX, f64::min),
            pts.iter().map(|p| p.1).fold(f64::MAX, f64::min),
        ),
        (
            pts.iter().map(|p| p.0).fold(f64::MIN, f64::max),
            pts.iter().map(|p| p.1).fold(f64::MIN, f64::max),
        ),
    );
    assert!(
        ((fhi.0 - flo.0) - (hi.0 - lo.0)).abs() < 0.01
            && ((fhi.1 - flo.1) - (hi.1 - lo.1)).abs() < 0.01,
        "the box size matches the job's: frame {flo:?}..{fhi:?} job {lo:?}..{hi:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 25: the laser is off for every corner and shape; a job with no burn frames nothing
// ---------------------------------------------------------------------------------------------

#[test]
fn every_corner_and_shape_is_laser_off_and_has_no_m3_or_s_above_zero() {
    for corner in [
        OriginCorner::BottomLeft,
        OriginCorner::BottomRight,
        OriginCorner::TopLeft,
        OriginCorner::TopRight,
    ] {
        for shape in [FrameShape::Rectangle, FrameShape::RubberBand] {
            for start in [
                Start::MachineOrigin,
                Start::HomeThenOrigin,
                Start::HeadPosition,
            ] {
                let page = two_rects(corner, start);
                let (_, frame) = texts(&page, shape);
                laser_off(&frame);
            }
        }
    }
}

#[test]
fn a_job_without_a_burn_move_has_no_frame() {
    let page = Page::new();
    let built = build_job(&page.input, &[], &mut PlanCache::default());
    let format = page.input.machine.values.output_format;
    assert!(
        write_frame(format, &built.job, FrameShape::Rectangle).is_none(),
        "nothing to frame"
    );
    assert!(write_frame(format, &built.job, FrameShape::RubberBand).is_none());
}

#[test]
fn the_frame_text_is_the_same_on_every_export() {
    let page = two_rects(OriginCorner::TopRight, Start::MachineOrigin);
    let (_, a) = texts(&page, FrameShape::RubberBand);
    let (_, b) = texts(&page, FrameShape::RubberBand);
    assert_eq!(a, b);
    assert!(a.is_ascii() && !a.contains('\r'));
}
