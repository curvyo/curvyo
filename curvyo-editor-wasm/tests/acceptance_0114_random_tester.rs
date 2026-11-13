//! Tester acceptance for `specs/0114-spiro-smoothing-and-width-handles` criteria 2 and 3 on
//! random Pen input: open and closed Spiros of 2 to 20 points, free, coincident, collinear or far
//! from the origin, are drawn finite at every pointer event without a notice and within the
//! per-event budget; the curve is a function of its controls (the same clicks with a different
//! pointer history store the same path and draw the same curve), and a reopen, an undo with its
//! redo, draw it unchanged. Written from the spec and the Session API.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::many_single_char_names
)]

mod cpu_time;

use std::time::Duration;

use curvyo_document_core::{CurveKind, Document, Point, unpack};
use curvyo_editor_wasm::{Session, Tool};
use curvyo_render_core::{DrawList, Vertex};
use curvyo_ui_core::PenMode;

/// A whole 16 ms frame (debug: ten times).
const FRAME: Duration = Duration::from_millis(if cfg!(debug_assertions) { 160 } else { 16 });

/// Criterion 3: one rebuild of the live Spiro per pointer event in 2 ms (release); unoptimised
/// code runs about ten times slower.
const BUDGET: Duration = Duration::from_millis(if cfg!(debug_assertions) { 20 } else { 2 });

fn artwork(list: &DrawList) -> Vec<Vertex> {
    let end = list.layers().last().copied().unwrap_or(0);
    list.triangles[..end].to_vec()
}

fn saved(session: &Session) -> Document {
    unpack(900, &session.pack("test").unwrap()).unwrap()
}

fn click(session: &mut Session, p: Point) {
    session.pointer_down(p, false);
    session.pointer_up(p, false, false);
}

fn assert_no_notice(s: &mut Session, at: &str) {
    assert_eq!(s.take_unsupported_width_notice(), "", "{at}: width notice");
    assert_eq!(s.take_width_edit_notice(), "", "{at}: width edit notice");
    assert_eq!(s.take_transform_notice(), "", "{at}: transform notice");
    assert_eq!(s.take_pen_entry_notice(), None, "{at}: pen entry notice");
    assert_eq!(s.take_history_notice(), None, "{at}: history notice");
}

fn assert_finite(frame: &[Vertex], at: &str) {
    assert!(
        frame
            .iter()
            .all(|v| v.position.x.is_finite() && v.position.y.is_finite()),
        "{at}: a non-finite vertex"
    );
}

#[derive(Clone)]
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f64) as usize).min(n - 1)
    }
}

/// One random control set: its kind for the messages and its points.
fn controls(rng: &mut Lcg) -> (String, Vec<Point>) {
    let n = 2 + rng.below(19);
    match rng.below(4) {
        0 => (
            "free".to_string(),
            (0..n)
                .map(|_| Point::new(100.0 * rng.next(), 100.0 * rng.next()))
                .collect(),
        ),
        1 => {
            // Neighbours 0 to 0.3 mm apart: on top of each other for the pointer.
            let mut v = Vec::new();
            while v.len() < n {
                let p = Point::new(100.0 * rng.next(), 100.0 * rng.next());
                for _ in 0..=rng.below(3) {
                    v.push(Point::new(p.x + 0.3 * rng.next(), p.y));
                }
            }
            v.truncate(n);
            ("coincident".to_string(), v)
        }
        2 => {
            let a = std::f64::consts::TAU * rng.next();
            (
                "collinear".to_string(),
                (0..n)
                    .map(|_| {
                        let t = 100.0 * rng.next();
                        Point::new(50.0 + t * a.cos(), 50.0 + t * a.sin())
                    })
                    .collect(),
            )
        }
        _ => {
            let off = [1e4, 1e5, 1e6][rng.below(3)];
            let scale = [0.01, 1.0, 1000.0][rng.below(3)];
            (
                format!("far x{scale}"),
                (0..n)
                    .map(|_| {
                        Point::new(
                            off + scale * 100.0 * rng.next(),
                            off - scale * 100.0 * rng.next(),
                        )
                    })
                    .collect(),
            )
        }
    }
}

/// Draws `points` with the Spiro Pen, hovering `hovers` positions on the way to each click (0:
/// straight to it); closes on the first point when `closed`. Returns the session and the worst
/// CPU time of one hover and frame.
fn draw(
    mode: PenMode,
    points: &[Point],
    closed: bool,
    hovers: usize,
    rng: &mut Lcg,
    at: &str,
) -> (Session, Vec<Duration>) {
    let mut s = Session::new(7);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(mode));
    let mut times = Vec::new();
    let mut prev = points[0];
    let mut targets = points.to_vec();
    if closed {
        targets.push(points[0]);
    }
    for (i, &p) in targets.iter().enumerate() {
        for k in 0..=hovers {
            // Wander round the way to the next point, as a hand does.
            let t = (k + 1) as f64 / (hovers + 1) as f64;
            let r = if k == hovers { 0.0 } else { 20.0 * rng.next() };
            let a = std::f64::consts::TAU * rng.next();
            let q = Point::new(
                prev.x + (p.x - prev.x) * t + r * a.cos(),
                prev.y + (p.y - prev.y) * t + r * a.sin(),
            );
            let (frame, spent) = cpu_time::cpu_time_of(|| {
                s.pointer_hover(q, false, false);
                s.draw_list()
            });
            times.push(spent);
            let where_ = format!("{at} point {i} hover {k} at {q:?}");
            assert_no_notice(&mut s, &where_);
            assert_finite(&frame.triangles, &where_);
            if i > 0 {
                assert!(!frame.triangles.is_empty(), "{where_}: nothing shown");
            }
        }
        click(&mut s, p);
        assert_no_notice(&mut s, &format!("{at} click {i}"));
        prev = p;
    }
    s.finish_pen();
    assert_no_notice(&mut s, &format!("{at} finish"));
    (s, times)
}

/// The artwork of the same controls drawn with the Lines Pen.
fn lines_artwork(points: &[Point], closed: bool) -> Vec<Vertex> {
    let mut s = Session::new(8);
    s.set_tool(Tool::Pen);
    assert!(s.set_pen_mode(PenMode::Lines));
    for &p in points {
        s.pointer_hover(p, false, false);
        click(&mut s, p);
    }
    if closed {
        s.pointer_hover(points[0], false, false);
        click(&mut s, points[0]);
    }
    s.finish_pen();
    artwork(&s.draw_list())
}

/// Criteria 2 and 3 on random input: finite, no notice, no event over a frame, a function of the
/// controls, unchanged by a reopen, by an undo and redo and by the undo of a Node drag.
#[test]
fn random_spiros_are_finite_within_budget_and_a_function_of_their_controls() {
    let mut rng = Lcg(0x0114_0002);
    let cases = if cfg!(debug_assertions) { 60 } else { 300 };
    let mut stats = std::collections::BTreeMap::new();
    let mut worst_case = std::collections::BTreeMap::new();
    let mut failures = Vec::new();
    let mut stored = 0;
    let mut drags = 0;
    for case in 0..cases {
        let (kind, points) = controls(&mut rng);
        let closed = points.len() >= 3 && rng.next() < 0.4;
        let at = format!("case {case} {kind} n {} closed {closed}", points.len());
        let (s, times) = draw(PenMode::Spiro, &points, closed, 6, &mut rng, &at);
        if points.len() >= 3 {
            let e = stats
                .entry(kind.clone())
                .or_insert((0, 0, 0, Duration::ZERO));
            e.0 += times.len();
            e.1 += times.iter().filter(|&&t| t > BUDGET).count();
            e.2 += times.iter().filter(|&&t| t > FRAME).count();
            let w = times.iter().copied().max().unwrap_or_default();
            if w > e.3 {
                e.3 = w;
                worst_case.insert(kind.clone(), at.clone());
            }
        }
        let doc = saved(&s);
        let Some(&id) = doc.object_ids().first() else {
            // Every point on top of the first: nothing to commit is fine, nothing must break.
            continue;
        };
        stored += 1;
        let path = doc.path(id).unwrap();
        assert_eq!(path.curve_kind, CurveKind::Spiro, "{at}");
        let shown = artwork(&s.draw_list());
        let anchors: Vec<_> = path.anchors.iter().map(|a| a.point).collect();
        // A path the Lines Pen does not draw either (a few hundredths of a millimetre) is not a
        // Spiro question.
        assert!(
            !shown.is_empty() || lines_artwork(&anchors, closed).is_empty(),
            "{at}: the stored path is drawn: {anchors:?}"
        );
        assert_finite(&shown, &at);

        // The same clicks without the wandering: the same controls, the same curve.
        let (direct, _) = draw(PenMode::Spiro, &points, closed, 0, &mut Lcg(case), &at);
        let direct_doc = saved(&direct);
        let direct_path = direct_doc.path(direct_doc.object_ids()[0]).unwrap();
        if direct_path.anchors != path.anchors || direct_path.closed != path.closed {
            failures.push(format!(
                "{at}: another pointer history stores other controls"
            ));
        } else if artwork(&direct.draw_list()) != shown {
            failures.push(format!(
                "{at}: the same controls draw another curve after another pointer history"
            ));
        }

        // A reopen draws it unchanged.
        let reopened = Session::open(901, &s.pack("t").unwrap()).unwrap();
        if artwork(&reopened.draw_list()) != shown {
            failures.push(format!("{at}: a reopen draws another curve"));
        }

        // Undo and redo draw it unchanged.
        let mut s = s;
        if s.undo_len() > 0 {
            s.undo();
            let _ = s.draw_list();
            s.redo();
            assert_no_notice(&mut s, &format!("{at} undo/redo"));
            if artwork(&s.draw_list()) != shown {
                failures.push(format!("{at}: undo and redo draw another curve"));
            }
        }

        // A Node drag of a control, then its undo, draws the curve from before the drag.
        if kind == "free" && anchors.len() >= 3 {
            let mut d = Session::open(903, &s.pack("t").unwrap()).unwrap();
            let before = artwork(&d.draw_list());
            d.set_tool(Tool::Select);
            click(&mut d, anchors[1]);
            if d.selected_object_count() == 1 {
                d.set_tool(Tool::Node);
                let _ = d.draw_list();
                let from = anchors[anchors.len() / 2];
                d.pointer_down(from, false);
                for k in 1..=20 {
                    let q = Point::new(
                        from.x + 2.0 * f64::from(k) * rng.next(),
                        from.y - 1.5 * f64::from(k),
                    );
                    d.pointer_hover(q, false, false);
                    let frame = d.draw_list();
                    assert_no_notice(&mut d, &format!("{at} drag {k}"));
                    assert_finite(&frame.triangles, &format!("{at} drag {k}"));
                }
                d.pointer_up(Point::new(from.x, from.y - 30.0), false, false);
                d.undo();
                drags += 1;
                if artwork(&d.draw_list()) != before {
                    failures.push(format!("{at}: undo of a Node drag draws another curve"));
                }
            }
        }
    }
    println!("{cases} cases, {stored} stored, {drags} Node drags undone");
    for (kind, (events, over, over_frame, worst)) in &stats {
        println!(
            "{kind}: {events} events, {over} over {BUDGET:?}, {over_frame} over {FRAME:?}, worst {worst:?} ({})",
            worst_case[kind]
        );
    }
    assert!(
        stored * 2 > cases,
        "most cases store a path: {stored} of {cases}"
    );
    assert!(drags > 0, "some Node drags were made");
    // Criterion 3 beyond `spiro_cpu_budget.rs`: in a drawing of laser-bed size every pointer
    // event, its frame included, takes at most 2 ms; in a drawing 100 m wide, `far x1000`, none
    // takes a whole frame (its stroke at screen tolerance is some 30 000 triangles, most of them
    // off the screen, which alone take up to about 1.7 ms).
    for (kind, (_, over, over_frame, worst)) in &stats {
        if *over_frame > 0 {
            failures.push(format!(
                "{kind}: {over_frame} events over a frame, worst {worst:?}"
            ));
        }
        if kind != "far x1000" && *over > 0 {
            failures.push(format!(
                "{kind}: {over} events over {BUDGET:?}, worst {worst:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
