//! The taper of `specs/0079-text-layout-controls` slice 2 in the engine (criteria 17 to 19, 38):
//! each glyph becomes a trapezoid about the middle of its advance, heights unchanged, horizontal
//! and vertical lines stay lines, curves within 0.01 mm of the exact map, before the rotation.
#![allow(clippy::unwrap_used)]

mod common;

use common::{SANS, SERIF, face, store_with};
use curvyo_text_core::{LayoutOptions, Mm, TextLayout, layout};
use kurbo::{Affine, BezPath, ParamCurve, ParamCurveNearest, PathEl, PathSeg, Point, Shape};

const TOLERANCE_MM: f64 = 0.01;

fn tapered(taper: f64) -> LayoutOptions {
    LayoutOptions {
        taper,
        ..LayoutOptions::default()
    }
}

/// The exact map of criterion 17 for the first glyph of `laid` (y down, baseline at 0).
fn exact(laid: &TextLayout, taper: f64, p: Point) -> Point {
    let centre = laid.lines[0].glyphs[0].centre_x.0;
    let height = -p.y;
    let scale = (1.0 + taper * (2.0 * height / laid.size.0 - 1.0)).max(0.05);
    Point::new(centre + (p.x - centre) * scale, p.y)
}

/// The leftmost and rightmost outline point at height `y` (within 1e-6 mm).
fn span_at(path: &BezPath, y: f64) -> (f64, f64) {
    let xs: Vec<f64> = path
        .elements()
        .iter()
        .filter_map(|el| match el {
            PathEl::MoveTo(p) | PathEl::LineTo(p) | PathEl::CurveTo(_, _, p) => Some(*p),
            _ => None,
        })
        .filter(|p| (p.y - y).abs() < 1e-6)
        .map(|p| p.x)
        .collect();
    let min = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (min, max)
}

fn distance_to(path: &BezPath, p: Point) -> f64 {
    path.segments()
        .map(|seg| seg.nearest(p, 1e-9).distance_sq.sqrt())
        .fold(f64::INFINITY, f64::min)
}

/// Criterion 17: "H" at 20 mm, Taper 20 %: the outer stem edges are 0.8 and 1.2 times the
/// untapered width apart at the baseline and at the top; the top stays 20 mm high.
#[test]
fn ac17_h_is_a_trapezoid_of_the_same_height() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let plain = layout(face, "H", Mm(20.0), &LayoutOptions::default()).unwrap();
    let laid = layout(face, "H", Mm(20.0), &tapered(0.2)).unwrap();
    let before = &plain.glyph_paths(face)[0];
    let after = &laid.glyph_paths(face)[0];
    let top = before.bounding_box().y0;
    assert!((top + 20.0).abs() < TOLERANCE_MM, "the H is 20 mm: {top}");
    let width = |path: &BezPath, y: f64| {
        let (min, max) = span_at(path, y);
        max - min
    };
    let base = width(before, 0.0);
    assert!((width(after, 0.0) - 0.8 * base).abs() < TOLERANCE_MM);
    assert!((width(after, top) - 1.2 * width(before, top)).abs() < TOLERANCE_MM);
    assert!((after.bounding_box().y0 - top).abs() < 1e-9, "height kept");
    assert!(after.bounding_box().y1.abs() < 1e-9);
    // Wider at the top for a positive taper, about the middle of the advance.
    let centre = laid.lines[0].glyphs[0].centre_x.0;
    let (min, max) = span_at(after, top);
    assert!((f64::midpoint(min, max) - centre).abs() < 0.5);
    // Advances and line layout do not change (criterion 18).
    assert_eq!(laid.lines[0].width, plain.lines[0].width);
    assert_eq!(laid.lines[0].boundaries, plain.lines[0].boundaries);
}

/// Criterion 18: horizontal and vertical lines stay lines; the slanted lines of "A" and every
/// curve of "S" and "g" lie within 0.01 mm of the exact mapped outline, both ways.
#[test]
fn ac18_lines_stay_lines_and_curves_hold_the_tolerance() {
    let (serif_store, serif_key) = store_with(SERIF);
    let serif = face(&serif_store, &serif_key);
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let h = layout(face, "H", Mm(20.0), &tapered(0.3)).unwrap();
    assert!(
        h.glyph_paths(face)[0]
            .elements()
            .iter()
            .all(|el| !matches!(el, PathEl::CurveTo(..) | PathEl::QuadTo(..))),
        "an H of straight stems stays straight"
    );
    for (face, text, taper) in [
        (face, "A", 0.3),
        (face, "S", 0.5),
        (face, "g", 0.5),
        (serif, "S", -0.4),
        (serif, "g", -0.5),
    ] {
        let plain = layout(face, text, Mm(20.0), &LayoutOptions::default()).unwrap();
        let laid = layout(face, text, Mm(20.0), &tapered(taper)).unwrap();
        let before = &plain.glyph_paths(face)[0];
        let after = &laid.glyph_paths(face)[0];
        let mut exact_points = Vec::new();
        for seg in before.segments() {
            for i in 0..=32 {
                exact_points.push(exact(&laid, taper, seg.eval(f64::from(i) / 32.0)));
            }
        }
        for p in &exact_points {
            let d = distance_to(after, *p);
            assert!(
                d < TOLERANCE_MM,
                "{text} at {taper}: exact point {d} mm off"
            );
        }
        // The other way: a dense polyline of the exact outline is near every drawn point.
        for seg in after.segments() {
            for i in 0..=8 {
                let p = seg.eval(f64::from(i) / 8.0);
                let d = exact_points
                    .windows(2)
                    .map(|w| kurbo::Line::new(w[0], w[1]).nearest(p, 1e-9).distance_sq)
                    .fold(f64::INFINITY, f64::min)
                    .sqrt();
                assert!(
                    d < 2.0 * TOLERANCE_MM,
                    "{text} at {taper}: drawn point {d} mm off"
                );
            }
        }
        // Horizontal and vertical lines of the outline are lines in the result (a vertical
        // line leans, but stays a line).
        let lines = |path: &BezPath, only_straight: bool| {
            path.segments()
                .filter(|seg| match seg {
                    PathSeg::Line(l) => {
                        !only_straight
                            || (l.p0.x - l.p1.x).abs() < 1e-9
                            || (l.p0.y - l.p1.y).abs() < 1e-9
                    }
                    _ => false,
                })
                .count()
        };
        assert!(
            lines(after, false) >= lines(before, true),
            "{text}: H and V lines kept"
        );
    }
}

/// Criterion 19: the taper is applied in the glyph's frame first, then the rotation about the
/// cluster's centre.
#[test]
fn ac19_taper_before_rotation() {
    let (store, key) = store_with(SANS);
    let face = face(&store, &key);
    let angle = 30.0_f64.to_radians();
    let laid = layout(face, "H", Mm(20.0), &tapered(0.2)).unwrap();
    let both = layout(
        face,
        "H",
        Mm(20.0),
        &LayoutOptions {
            rotation_rad: angle,
            ..tapered(0.2)
        },
    )
    .unwrap();
    let glyph = laid.lines[0].glyphs[0];
    let pivot = Point::new(glyph.centre_x.0, glyph.centre_y.0);
    let expected = Affine::rotate_about(angle, pivot) * &laid.glyph_paths(face)[0];
    let got = &both.glyph_paths(face)[0];
    for (a, b) in expected.elements().iter().zip(got.elements()) {
        let (PathEl::LineTo(a) | PathEl::MoveTo(a), PathEl::LineTo(b) | PathEl::MoveTo(b)) = (a, b)
        else {
            continue;
        };
        assert!(a.distance(*b) < 1e-9);
    }
    assert_eq!(expected.elements().len(), got.elements().len());
}

/// Criterion 19: on a ring of baseline radius R, t = h / (2(R + h/2)) outside and
/// t = −h / (2(R − h/2)) inside make a vertical stem point at the ring's centre: the stem's
/// tapered x at height y is its x at half height times the ring's radius ratio.
#[test]
fn ac19_the_circle_values() {
    let h = 10.0;
    let r = 40.0;
    let scale = |t: f64, y: f64| 1.0 + t * (2.0 * y / h - 1.0);
    let outside = h / (2.0 * (r + h / 2.0));
    let inside = -h / (2.0 * (r - h / 2.0));
    for y in [0.0, 2.5, 7.5, 10.0] {
        assert!((scale(outside, y) - (r + y) / (r + h / 2.0)).abs() < 1e-12);
        assert!((scale(inside, y) - (r - y) / (r - h / 2.0)).abs() < 1e-12);
    }
}
