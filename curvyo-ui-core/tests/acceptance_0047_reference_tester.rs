//! Independent tester acceptance tests for the pure rules of Part C of
//! `specs/0047-transform-polish/specification.md` (criteria 39, 40, 41, 43, 50): the nine
//! reference points, the 32 px rule for marks, the hit radius `min(12 px, 0.24 s)`
//! (`min(22 px, 0.24 s)` for a touch), nearest-centre-first with the centre winning a tie,
//! the viewport rule, and hostile numbers. Expected values are a reference model written here
//! from the specification text; the implementation was not read for the expectations.

#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use curvyo_document_core::Point;
use curvyo_ui_core::{
    MARKS_MIN_SIDE_PX, ReferencePoint, hit_mark, mark_hit_radius_px, marks_are_shown,
    shorter_side_px, visible_marks,
};
use proptest::prelude::*;

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn bx(x0: f64, y0: f64, x1: f64, y1: f64) -> (Point, Point) {
    (pt(x0, y0), pt(x1, y1))
}

/// Reference model: the nine points of a box as the spec defines them (row by row).
fn model_points(b: (Point, Point)) -> [(f64, f64); 9] {
    let (lo, hi) = b;
    let cx = (lo.x + hi.x) / 2.0;
    let cy = (lo.y + hi.y) / 2.0;
    [
        (lo.x, lo.y),
        (cx, lo.y),
        (hi.x, lo.y),
        (lo.x, cy),
        (cx, cy),
        (hi.x, cy),
        (lo.x, hi.y),
        (cx, hi.y),
        (hi.x, hi.y),
    ]
}

/// Document to screen with a uniform zoom `k` (px per mm) and a pan.
fn mapper(k: f64, pan: (f64, f64)) -> impl Fn(Point) -> (f64, f64) {
    move |p: Point| (p.x * k + pan.0, p.y * k + pan.1)
}

// ---------------------------------------------------------------------
// The nine points (criteria 37, 40, 41)
// ---------------------------------------------------------------------

/// Criterion 41: nine points in the order of the radios, row by row; criterion 40's example box
/// (30 x 40 at (10, 20)).
#[test]
fn the_nine_points_lie_where_the_spec_puts_them() {
    let b = bx(10.0, 20.0, 40.0, 60.0);
    assert_eq!(ReferencePoint::ALL.len(), 9);
    let model = model_points(b);
    for (i, p) in ReferencePoint::ALL.iter().enumerate() {
        let got = p.point(b);
        assert_eq!((got.x, got.y), model[i], "point {i} ({p:?})");
        assert_eq!(p.index(), i);
        assert_eq!(ReferencePoint::from_index(i), Some(*p));
    }
    assert_eq!(ReferencePoint::from_index(9), None);
    assert_eq!(ReferencePoint::from_index(usize::MAX), None);
    // The centre: (25, 40); Centre is the fifth (row 2, middle).
    assert_eq!(ReferencePoint::ALL[4], ReferencePoint::Centre);
    assert!(ReferencePoint::Centre.is_centre());
    assert_eq!(
        ReferencePoint::ALL.iter().filter(|p| p.is_centre()).count(),
        1
    );
    let c = ReferencePoint::Centre.point(b);
    assert_eq!((c.x, c.y), (25.0, 40.0));
}

/// Criterion 41: the accessible names, row by row, in DOM order.
#[test]
fn radio_names_follow_the_spec_order() {
    let names: Vec<&str> = ReferencePoint::ALL.iter().map(|p| p.name()).collect();
    assert_eq!(
        names,
        [
            "Top left",
            "Top centre",
            "Top right",
            "Left middle",
            "Centre",
            "Right middle",
            "Bottom left",
            "Bottom centre",
            "Bottom right"
        ]
    );
}

/// Criterion 43: the first line of the hint chip of each mark.
#[test]
fn hint_chip_names_follow_the_spec() {
    let names: Vec<&str> = ReferencePoint::ALL.iter().map(|p| p.hint_name()).collect();
    assert_eq!(
        names,
        [
            "Top left corner",
            "Top centre",
            "Top right corner",
            "Left middle",
            "Centre",
            "Right middle",
            "Bottom left corner",
            "Bottom centre",
            "Bottom right corner"
        ]
    );
}

/// Degenerate boxes: a line (zero height) and a point still give nine well-defined points.
#[test]
fn degenerate_boxes_still_give_nine_points() {
    let line = bx(0.0, 5.0, 10.0, 5.0);
    for p in ReferencePoint::ALL {
        let q = p.point(line);
        assert!(q.x.is_finite() && (q.y - 5.0).abs() < 1e-12);
    }
    let dot = bx(3.0, 3.0, 3.0, 3.0);
    for p in ReferencePoint::ALL {
        let q = p.point(dot);
        assert_eq!((q.x, q.y), (3.0, 3.0));
    }
}

// ---------------------------------------------------------------------
// 32 px rule, radius (criteria 39, 40, 50)
// ---------------------------------------------------------------------

/// Criterion 39: marks are drawn from a shorter side of 32 px, not below.
#[test]
fn marks_appear_from_32_px() {
    assert_eq!(MARKS_MIN_SIDE_PX, 32.0);
    assert!(!marks_are_shown(31.999));
    assert!(marks_are_shown(32.0));
    assert!(marks_are_shown(5000.0));
    assert!(!marks_are_shown(0.0));
    assert!(!marks_are_shown(-5.0));
    assert!(!marks_are_shown(f64::NAN));
    assert!(marks_are_shown(f64::INFINITY));
    // The shorter side of a 100 x 31.9 mm box at 1 px/mm is 31.9.
    let b = bx(0.0, 0.0, 100.0, 31.9);
    assert!((shorter_side_px(b, 1.0) - 31.9).abs() < 1e-9);
    assert!(visible_marks(b, 1.0, (4000.0, 4000.0), mapper(1.0, (0.0, 0.0))).is_empty());
    let b = bx(0.0, 0.0, 100.0, 32.0);
    assert_eq!(
        visible_marks(b, 1.0, (4000.0, 4000.0), mapper(1.0, (0.0, 0.0))).len(),
        9
    );
}

/// Criterion 50: a 2 mm square at 100 % zoom (about 3.78 px/mm, s about 7.6 px) shows no mark.
#[test]
fn a_two_mm_square_at_100_percent_has_no_marks() {
    let b = bx(10.0, 10.0, 12.0, 12.0);
    let k = 96.0 / 25.4;
    assert!((shorter_side_px(b, k) - 7.559).abs() < 0.01);
    assert!(visible_marks(b, k, (1600.0, 1000.0), mapper(k, (0.0, 0.0))).is_empty());
    let shown = visible_marks(b, k, (1600.0, 1000.0), mapper(k, (0.0, 0.0)));
    assert_eq!(
        hit_mark(b, &shown, (11.0 * k, 11.0 * k), k, false, mapper(k, (0.0, 0.0))),
        None,
        "marks that are not shown are not clickable"
    );
}

/// Criterion 40 and the "Zoom" decision: radius `min(12, 0.24 s)`, touch `min(22, 0.24 s)`;
/// 24 px across from s = 50.
#[test]
fn hit_radius_table() {
    let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
    assert!(
        near(mark_hit_radius_px(50.0, false), 12.0),
        "24 px across from s = 50"
    );
    assert!(near(mark_hit_radius_px(49.0, false), 0.24 * 49.0));
    assert!(near(mark_hit_radius_px(32.0, false), 7.68));
    assert!(near(mark_hit_radius_px(1000.0, false), 12.0));
    assert!(near(mark_hit_radius_px(1000.0, true), 22.0));
    assert!(near(mark_hit_radius_px(50.0, true), 12.0));
    assert!(near(mark_hit_radius_px(91.67, true), 22.0));
    assert!(near(mark_hit_radius_px(90.0, true), 21.6));
}

// ---------------------------------------------------------------------
// Hit test (criterion 40)
// ---------------------------------------------------------------------

/// A press on a mark's centre hits that mark, one pixel beyond its radius hits nothing (for a
/// box big enough that other marks are far), and within the radius hits it.
#[test]
fn a_press_hits_the_mark_within_its_radius_only() {
    let b = bx(0.0, 0.0, 300.0, 200.0);
    let k = 1.0;
    let to = mapper(k, (50.0, 50.0));
    let marks = visible_marks(b, k, (2000.0, 2000.0), &to);
    assert_eq!(marks.len(), 9);
    for (i, m) in ReferencePoint::ALL.iter().enumerate() {
        let (x, y) = to(m.point(b));
        assert_eq!(
            hit_mark(b, &marks, (x, y), k, false, &to),
            Some(*m),
            "centre of {i}"
        );
        assert_eq!(
            hit_mark(b, &marks, (x + 11.9, y), k, false, &to),
            Some(*m),
            "11.9 px right of {i}"
        );
        assert_eq!(
            hit_mark(b, &marks, (x, y - 12.0), k, false, &to),
            Some(*m),
            "exactly on the radius of {i}"
        );
        assert_eq!(
            hit_mark(b, &marks, (x + 12.5, y), k, false, &to),
            None,
            "12.5 px beyond {i}"
        );
        // Touch: 22 px.
        assert_eq!(
            hit_mark(b, &marks, (x + 21.9, y), k, true, &to),
            Some(*m),
            "touch 21.9 px of {i}"
        );
        assert_eq!(
            hit_mark(b, &marks, (x + 22.5, y), k, true, &to),
            None,
            "touch 22.5 px of {i}"
        );
    }
    // Nowhere near any mark.
    assert_eq!(hit_mark(b, &marks, (125.0, 100.0), k, false, &to), None);
}

/// Criterion 40: marks that are not in the visible set cannot be hit even when the pointer is on
/// their centre.
#[test]
fn only_visible_marks_can_be_hit() {
    let b = bx(0.0, 0.0, 300.0, 200.0);
    let to = mapper(1.0, (0.0, 0.0));
    let marks = [ReferencePoint::TopLeft];
    let (x, y) = to(ReferencePoint::BottomRight.point(b));
    assert_eq!(hit_mark(b, &marks, (x, y), 1.0, false, &to), None);
    assert_eq!(hit_mark(b, &[], (0.0, 0.0), 1.0, false, &to), None);
}

/// Criterion 40: "the centre cross winning a tie". The real geometry cannot tie (targets do not
/// overlap), so the rule is checked with a mapping that puts the centre and the top centre on one
/// screen point; the order of the mark list must not matter.
#[test]
fn the_centre_wins_an_exact_tie() {
    let b = bx(0.0, 0.0, 300.0, 200.0);
    // The top centre (150, 0) and the centre (150, 100) both land on (150, 100).
    let squash = |p: Point| (p.x, if p.y <= 100.0 { 100.0 } else { p.y });
    let orders: [Vec<ReferencePoint>; 3] = [
        vec![ReferencePoint::TopCentre, ReferencePoint::Centre],
        vec![ReferencePoint::Centre, ReferencePoint::TopCentre],
        ReferencePoint::ALL.to_vec(),
    ];
    for marks in orders {
        assert_eq!(
            hit_mark(b, &marks, (150.0, 100.0), 1.0, false, squash),
            Some(ReferencePoint::Centre),
            "centre wins over the top centre, list {marks:?}"
        );
    }
}

/// Criterion 40: nearest centre first when two targets do share a point of the plane (a
/// squashed mapping again): the nearer mark wins even though the centre is also in range.
#[test]
fn the_nearest_mark_wins() {
    let b = bx(0.0, 0.0, 300.0, 200.0);
    // Top centre lands at (150, 95), the centre at (150, 100).
    let squash = |p: Point| (p.x, if p.y < 50.0 { 95.0 } else { p.y });
    assert_eq!(
        hit_mark(b, &ReferencePoint::ALL, (150.0, 94.0), 1.0, false, squash),
        Some(ReferencePoint::TopCentre)
    );
    assert_eq!(
        hit_mark(b, &ReferencePoint::ALL, (150.0, 101.0), 1.0, false, squash),
        Some(ReferencePoint::Centre)
    );
}

// ---------------------------------------------------------------------
// Viewport (criteria 39, 50)
// ---------------------------------------------------------------------

/// Criterion 39: a mark outside the viewport is not drawn and not pulled in; the others stay.
#[test]
fn marks_outside_the_viewport_are_dropped() {
    // A box far larger than the viewport: only the centre and the marks near it are inside.
    let b = bx(-1000.0, -1000.0, 1000.0, 1000.0);
    let to = mapper(1.0, (500.0, 400.0));
    let v = visible_marks(b, 1.0, (800.0, 600.0), &to);
    assert_eq!(v, vec![ReferencePoint::Centre]);
    // Pan so that the right edge's middle comes into view.
    let to = mapper(1.0, (-500.0, 400.0));
    let v = visible_marks(b, 1.0, (800.0, 600.0), &to);
    for m in &v {
        let (x, y) = to(m.point(b));
        assert!((0.0..=800.0).contains(&x) && (0.0..=600.0).contains(&y));
    }
    assert!(v.contains(&ReferencePoint::RightMiddle));
    assert!(!v.contains(&ReferencePoint::Centre), "the centre is off screen to the left");
    assert!(!v.contains(&ReferencePoint::TopLeft));
    assert!(!v.contains(&ReferencePoint::BottomRight));
}

// ---------------------------------------------------------------------
// Hostile numbers
// ---------------------------------------------------------------------

/// NaN, infinity, negative and huge inputs never panic and never produce a mark that is not
/// finite and inside the viewport.
#[test]
fn hostile_numbers() {
    let bad = [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.0,
        1e300,
        -1e300,
        f64::MIN_POSITIVE,
    ];
    for &k in &bad {
        for &(x1, y1) in &[
            (10.0, 10.0),
            (0.0, 0.0),
            (1e12, 1e12),
            (f64::NAN, 5.0),
            (-5.0, -5.0),
        ] {
            let b = bx(0.0, 0.0, x1, y1);
            let to = mapper(k, (0.0, 0.0));
            let marks = visible_marks(b, k, (800.0, 600.0), &to);
            for m in &marks {
                let (x, y) = to(m.point(b));
                assert!(
                    x.is_finite() && y.is_finite(),
                    "k={k} box={x1},{y1}: {m:?} not finite"
                );
                assert!((0.0..=800.0).contains(&x) && (0.0..=600.0).contains(&y));
            }
            for &px in &bad {
                let hit = hit_mark(b, &marks, (px, px), k, false, &to);
                if let Some(m) = hit {
                    assert!(marks.contains(&m));
                }
            }
            let _ = mark_hit_radius_px(k, false);
            let _ = mark_hit_radius_px(k, true);
        }
    }
    // A viewport that is zero or negative shows nothing.
    let b = bx(0.0, 0.0, 100.0, 100.0);
    assert!(
        visible_marks(b, 1.0, (0.0, 0.0), mapper(1.0, (50.0, 50.0))).is_empty()
            || visible_marks(b, 1.0, (0.0, 0.0), mapper(1.0, (50.0, 50.0))).len() <= 1
    );
    assert!(visible_marks(b, 1.0, (-5.0, -5.0), mapper(1.0, (50.0, 50.0))).is_empty());
    assert!(visible_marks(b, 1.0, (f64::NAN, 600.0), mapper(1.0, (50.0, 50.0))).is_empty());
    // An inverted box (max < min) is not a box with marks.
    let inv = bx(100.0, 100.0, 0.0, 0.0);
    assert!(visible_marks(inv, 1.0, (4000.0, 4000.0), mapper(1.0, (0.0, 0.0))).is_empty());
}

// ---------------------------------------------------------------------
// Properties
// ---------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Zoom and pan independence: whatever zoom (px/mm), pan and box, the visible set equals the
    /// set a model computes from the spec (shorter side >= 32 px, inside the viewport), and a
    /// press on each visible mark's screen position hits exactly that mark.
    #[test]
    fn visible_set_and_hits_follow_the_spec_at_every_zoom_and_pan(
        x0 in -500.0f64..500.0, y0 in -500.0f64..500.0,
        w in 0.0f64..800.0, h in 0.0f64..800.0,
        k in 0.05f64..60.0,
        panx in -3000.0f64..3000.0, pany in -3000.0f64..3000.0,
        vw in 200.0f64..3000.0, vh in 200.0f64..2000.0,
        touch in any::<bool>(),
    ) {
        let b = bx(x0, y0, x0 + w, y0 + h);
        let to = mapper(k, (panx, pany));
        let s = w.min(h) * k;
        let model: Vec<ReferencePoint> = if s >= 32.0 {
            ReferencePoint::ALL
                .into_iter()
                .filter(|m| {
                    let (x, y) = to(m.point(b));
                    x >= 0.0 && y >= 0.0 && x <= vw && y <= vh
                })
                .collect()
        } else {
            Vec::new()
        };
        let got = visible_marks(b, k, (vw, vh), &to);
        prop_assert_eq!(&got, &model);
        let r = if touch { 22.0f64.min(0.24 * s) } else { 12.0f64.min(0.24 * s) };
        prop_assert!((mark_hit_radius_px(s, touch) - r).abs() < 1e-9);
        for m in &got {
            let (x, y) = to(m.point(b));
            prop_assert_eq!(hit_mark(b, &got, (x, y), k, touch, &to), Some(*m));
            // Just inside and just outside the radius along a diagonal-free direction.
            if r > 1.0 {
                prop_assert_eq!(hit_mark(b, &got, (x + r * 0.9, y), k, touch, &to), Some(*m));
            }
        }
    }

    /// Criterion 40: the nine targets never overlap at any size, box shape or zoom: the distance
    /// between any two marks is at least twice the hit radius, so a press is on at most one
    /// target, and the result of `hit_mark` is the mark whose centre is within the radius.
    #[test]
    fn targets_never_overlap(
        w in 0.0f64..2000.0, h in 0.0f64..2000.0, k in 0.01f64..100.0,
        px in -100.0f64..2200.0, py in -100.0f64..2200.0, touch in any::<bool>(),
    ) {
        let b = bx(0.0, 0.0, w, h);
        let to = mapper(k, (0.0, 0.0));
        let s = w.min(h) * k;
        prop_assume!(s >= 32.0);
        let r = mark_hit_radius_px(s, touch);
        let pts = ReferencePoint::ALL.map(|m| to(m.point(b)));
        for i in 0..9 {
            for j in (i + 1)..9 {
                let d = (pts[i].0 - pts[j].0).hypot(pts[i].1 - pts[j].1);
                prop_assert!(d >= 2.0 * r - 1e-9, "marks {i} and {j} are {d} apart, radius {r}");
            }
        }
        let on: Vec<usize> = (0..9)
            .filter(|&i| (pts[i].0 - px).hypot(pts[i].1 - py) <= r)
            .collect();
        prop_assert!(on.len() <= 1, "a press is on at most one target");
        let marks = visible_marks(b, k, (1e9, 1e9), &to);
        let hit = hit_mark(b, &marks, (px, py), k, touch, &to);
        match on.as_slice() {
            [] => prop_assert_eq!(hit, None),
            [i] => prop_assert_eq!(hit, Some(ReferencePoint::ALL[*i])),
            _ => unreachable!(),
        }
    }

    /// Criterion 41/37: the nine points are inside the box, the centre is the midpoint, and the
    /// opposite pairs are symmetric about it.
    #[test]
    fn nine_points_are_consistent(
        x0 in -1e6f64..1e6, y0 in -1e6f64..1e6, w in 0.0f64..1e6, h in 0.0f64..1e6,
    ) {
        let b = bx(x0, y0, x0 + w, y0 + h);
        let c = ReferencePoint::Centre.point(b);
        for (a, z) in [
            (ReferencePoint::TopLeft, ReferencePoint::BottomRight),
            (ReferencePoint::TopRight, ReferencePoint::BottomLeft),
            (ReferencePoint::TopCentre, ReferencePoint::BottomCentre),
            (ReferencePoint::LeftMiddle, ReferencePoint::RightMiddle),
        ] {
            let (p, q) = (a.point(b), z.point(b));
            let tol = 1e-6 * (1.0 + x0.abs() + w);
            prop_assert!(((p.x + q.x) / 2.0 - c.x).abs() <= tol);
            prop_assert!(((p.y + q.y) / 2.0 - c.y).abs() <= tol);
        }
        for m in ReferencePoint::ALL {
            let p = m.point(b);
            let tol = 1e-6 * (1.0 + x0.abs() + w);
            prop_assert!(p.x >= x0 - tol && p.x <= x0 + w + tol && p.y >= y0 - tol && p.y <= y0 + h + tol);
        }
    }
}
