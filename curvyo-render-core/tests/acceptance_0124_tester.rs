//! Independent tester tests for `specs/0124-gradient-fill` in the draw list (criteria 9, 10,
//! 24 e): the tester runs the fragment rule of ADR 0067 §2 on the CPU (barycentric `u, v`, `t`,
//! pad, a search of the stop table, premultiplied blend) at points inside every gradient
//! triangle and compares it with its own reference of the shape's gradient; holes stay empty;
//! the stroke comes after the fill; laser grey stops equal the grey of the colours; 256 stops.
//! Written from the specification and ADR 0067; only public names were looked up.

#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]
#![allow(clippy::too_many_lines, missing_docs, clippy::doc_markdown)]

use curvyo_document_core::{
    AnchorId, AxisRatio, BoxPoint, Color, Document, FillPaint, GradientGeometry, GradientStop,
    GradientStops, Length, NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds, StopOffset,
    StyleEdit, ViewTransform,
};
use curvyo_render_core::{DrawList, StopTexel, build_artwork, build_artwork_greyed};

fn stop(r: u8, g: u8, b: u8, a: u8, o: f64) -> GradientStop {
    GradientStop {
        colour: Color { r, g, b },
        alpha: a,
        offset: StopOffset::new(o).unwrap(),
    }
}

fn shade(d: &Document, id: NodeId, geometry: GradientGeometry, stops: &[GradientStop]) {
    for e in [
        StyleEdit::FillEnabled(true),
        StyleEdit::FillPaint(FillPaint::Gradient),
        StyleEdit::GradientGeometry(geometry),
        StyleEdit::GradientStops(GradientStops::new(stops.to_vec()).unwrap()),
    ] {
        d.edit_style(&[id], &e).unwrap();
    }
}

/// The tester's own `t` for an axis-aligned box `min..max`.
fn ref_t(g: &GradientGeometry, min: Point, max: Point, p: Point) -> f64 {
    let at = |b: BoxPoint| {
        (
            min.x + b.x.get() * (max.x - min.x),
            min.y + b.y.get() * (max.y - min.y),
        )
    };
    match *g {
        GradientGeometry::Linear { start, end } => {
            let (s, e) = (at(start), at(end));
            let (dx, dy) = (e.0 - s.0, e.1 - s.1);
            ((p.x - s.0) * dx + (p.y - s.1) * dy) / (dx * dx + dy * dy)
        }
        GradientGeometry::Radial {
            centre,
            radius,
            ratio,
        } => {
            let (c, r) = (at(centre), at(radius));
            let (ax, ay) = (r.0 - c.0, r.1 - c.1);
            let l2 = ax * ax + ay * ay;
            let (qx, qy) = (p.x - c.0, p.y - c.1);
            let u = (qx * ax + qy * ay) / l2;
            let v = (-qx * ay + qy * ax) / l2 / ratio.get();
            (u * u + v * v).sqrt()
        }
    }
}

/// Reference premultiplied colour (0..1) at `t`.
fn ref_colour(stops: &[GradientStop], t: f64) -> [f64; 4] {
    let t = t.clamp(0.0, 1.0);
    let pm = |s: &GradientStop| {
        let a = f64::from(s.alpha) / 255.0;
        [
            f64::from(s.colour.r) / 255.0 * a,
            f64::from(s.colour.g) / 255.0 * a,
            f64::from(s.colour.b) / 255.0 * a,
            a,
        ]
    };
    let mut i = 0;
    for (k, s) in stops.iter().enumerate() {
        if s.offset.get() <= t {
            i = k;
        }
    }
    if i + 1 >= stops.len() {
        return pm(&stops[i]);
    }
    let (a, b) = (&stops[i], &stops[i + 1]);
    let span = b.offset.get() - a.offset.get();
    let f = if span <= 0.0 {
        1.0
    } else {
        (t - a.offset.get()) / span
    };
    let (pa, pb) = (pm(a), pm(b));
    [0, 1, 2, 3].map(|c| pa[c] + (pb[c] - pa[c]) * f)
}

/// The fragment rule run on the CPU from the table (the tester's reading of ADR 0067 §2).
fn fragment(table: &[StopTexel], first: usize, count: usize, t: f64) -> [f64; 4] {
    let s = &table[first..first + count];
    let t = t.clamp(0.0, 1.0) as f32;
    let mut i = 0;
    for (k, x) in s.iter().enumerate() {
        if x.offset <= t {
            i = k;
        }
    }
    if i + 1 >= s.len() {
        return s[i].colour.map(f64::from);
    }
    let (a, b) = (&s[i], &s[i + 1]);
    let span = b.offset - a.offset;
    let f = if span <= 0.0 {
        1.0
    } else {
        (t - a.offset) / span
    };
    [0, 1, 2, 3].map(|c| f64::from(a.colour[c] + (b.colour[c] - a.colour[c]) * f))
}

fn objects(d: &Document) -> Vec<ObjectSnapshot> {
    d.read_objects().objects.iter().cloned().collect()
}

fn view() -> ViewTransform {
    ViewTransform::new(3.0, Point::new(0.0, 0.0))
}

/// Checks every gradient triangle at its centroid and two inner points.
fn check_list(
    list: &DrawList,
    geometry: &GradientGeometry,
    stops: &[GradientStop],
    min: Point,
    max: Point,
) -> usize {
    let attrs = list.gradient_attributes();
    let table = list.gradient_stops();
    let mut checked = 0;
    for span in list.gradients() {
        let r = span.vertices();
        assert_eq!(r.len() % 3, 0, "whole triangles");
        for tri in r.clone().step_by(3) {
            for w in [[1.0 / 3.0; 3], [0.7, 0.2, 0.1], [0.1, 0.15, 0.75]] {
                let mut p = (0.0, 0.0);
                let (mut u, mut v) = (0.0, 0.0);
                for k in 0..3 {
                    let vert = &list.triangles[tri + k];
                    p.0 += w[k] * vert.position.x;
                    p.1 += w[k] * vert.position.y;
                    u += w[k] * f64::from(attrs[tri + k][0]);
                    v += w[k] * f64::from(attrs[tri + k][1]);
                }
                let a = attrs[tri];
                let radial = a[3] < 0.0;
                assert_eq!(radial, matches!(geometry, GradientGeometry::Radial { .. }));
                let t = if radial { (u * u + v * v).sqrt() } else { u };
                let p = Point::new(p.0, p.1);
                let want_t = ref_t(geometry, min, max, p);
                assert!(
                    (t - want_t).abs() < 1e-4,
                    "t at {p:?}: drawn {t} reference {want_t}"
                );
                if stops.iter().any(|s| (s.offset.get() - want_t).abs() < 1e-3) {
                    continue;
                }
                let got = fragment(table, a[2] as usize, a[3].abs() as usize, t);
                let want = ref_colour(stops, want_t);
                for c in 0..4 {
                    assert!(
                        (got[c] - want[c]).abs() < 2.0 / 255.0,
                        "colour at {p:?}: {got:?} vs {want:?}"
                    );
                }
                checked += 1;
            }
        }
    }
    checked
}

fn stops6() -> Vec<GradientStop> {
    vec![
        stop(0, 0, 0, 255, 0.0),
        stop(255, 0, 0, 255, 0.25),
        stop(255, 255, 0, 64, 0.25),
        stop(0, 255, 255, 255, 0.6),
        stop(0, 0, 255, 0, 0.8),
        stop(255, 255, 255, 255, 1.0),
    ]
}

#[test]
fn c9_the_drawn_shade_matches_the_reference_inside_every_triangle() {
    let geometries = [
        GradientGeometry::Linear {
            start: BoxPoint::new(0.1, 0.9).unwrap(),
            end: BoxPoint::new(0.95, 0.05).unwrap(),
        },
        GradientGeometry::DEFAULT_RADIAL,
        GradientGeometry::Radial {
            centre: BoxPoint::new(0.3, 0.6).unwrap(),
            radius: BoxPoint::new(0.9, 0.3).unwrap(),
            ratio: AxisRatio::new(0.35).unwrap(),
        },
    ];
    for g in geometries {
        let d = Document::new(1);
        let id = d.create_rect(RectBounds {
            origin: Point::new(10.0, 20.0),
            width: Length::from_mm(120.0),
            height: Length::from_mm(45.0),
        });
        shade(&d, id, g, &stops6());
        let list = build_artwork(&objects(&d), view());
        assert_eq!(list.gradients().len(), 1);
        let n = check_list(
            &list,
            &g,
            &stops6(),
            Point::new(10.0, 20.0),
            Point::new(130.0, 65.0),
        );
        assert!(n > 2, "{n}");
    }
}

#[test]
fn c9_holes_stay_empty_and_the_stroke_is_drawn_after_the_fill() {
    let d = Document::new(1);
    // A compound path: outer square with a square hole.
    let a = |k: u64, x: f64, y: f64| NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
    let outer = vec![
        a(1, 0.0, 0.0),
        a(2, 60.0, 0.0),
        a(3, 60.0, 40.0),
        a(4, 0.0, 40.0),
    ];
    let inner = vec![
        a(5, 20.0, 10.0),
        a(6, 20.0, 30.0),
        a(7, 40.0, 30.0),
        a(8, 40.0, 10.0),
    ];
    let seed = d.create_path(&[a(91, 500.0, 500.0), a(92, 501.0, 500.0)], false);
    let id = d
        .replace_with_path(
            &[seed],
            seed,
            &[(outer, true), (inner, true)],
            "boolean_union",
        )
        .unwrap();
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, &stops6());
    let list = build_artwork(&objects(&d), view());
    assert_eq!(list.gradients().len(), 1);
    let span = &list.gradients()[0];
    for tri in span.vertices().step_by(3) {
        let c = (0..3).fold((0.0, 0.0), |acc, k| {
            let p = list.triangles[tri + k].position;
            (acc.0 + p.x / 3.0, acc.1 + p.y / 3.0)
        });
        assert!(
            !(c.0 > 20.0 && c.0 < 40.0 && c.1 > 10.0 && c.1 < 30.0),
            "a gradient triangle in the hole at {c:?}"
        );
    }
    let fill_end = span.vertices().end;
    assert!(
        list.triangles.len() > fill_end,
        "stroke vertices follow the fill"
    );
    let attrs = list.gradient_attributes();
    assert!(
        attrs[fill_end..].iter().all(|a| a[3] == 0.0),
        "the stroke has no gradient"
    );
    check_list(
        &list,
        &GradientGeometry::DEFAULT_LINEAR,
        &stops6(),
        Point::new(0.0, 0.0),
        Point::new(60.0, 40.0),
    );
}

#[test]
fn c10_grey_stops_are_the_grey_of_the_colours_over_white() {
    let d = Document::new(1);
    let id = d.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(50.0),
        height: Length::from_mm(20.0),
    });
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, &stops6());
    let list = build_artwork_greyed(&objects(&d), &Default::default(), &|_| true, view());
    let span = &list.gradients()[0];
    let r = span.stops();
    let table = &list.gradient_stops()[r.start as usize..r.end as usize];
    assert_eq!(table.len(), 6);
    for (t, s) in table.iter().zip(stops6()) {
        let a = f64::from(s.alpha) / 255.0;
        let over = |c: u8| f64::from(c) * a + 255.0 * (1.0 - a);
        let want =
            (0.2126 * over(s.colour.r) + 0.7152 * over(s.colour.g) + 0.0722 * over(s.colour.b))
                / 255.0;
        assert!(
            (f64::from(t.colour[0]) - want).abs() < 1.5 / 255.0,
            "{t:?} vs {want}"
        );
        assert_eq!(t.colour[0], t.colour[1]);
        assert_eq!(t.colour[1], t.colour[2]);
        assert_eq!(t.colour[3], 1.0, "opaque");
    }
    // Grey of the interpolation equals interpolation of the grey (linearity), at a mid point.
    let colour = build_artwork(&objects(&d), view());
    let ct = colour.gradient_stops();
    let mid = fragment(ct, 0, 6, 0.42);
    let over_white = mid[3];
    let g = 0.2126 * (mid[0] + 1.0 - over_white)
        + 0.7152 * (mid[1] + 1.0 - over_white)
        + 0.0722 * (mid[2] + 1.0 - over_white);
    let grey_mid = fragment(table, 0, 6, 0.42);
    assert!((grey_mid[0] - g).abs() < 2.0 / 255.0, "{grey_mid:?} vs {g}");
}

#[test]
fn c24e_256_stops_draw_without_error_and_match() {
    let stops: Vec<GradientStop> = (0..256)
        .map(|k| {
            let o = if k == 255 { 1.0 } else { f64::from(k) / 255.0 };
            stop(
                (k * 7 % 256) as u8,
                (k * 13 % 256) as u8,
                (k * 29 % 256) as u8,
                255,
                o,
            )
        })
        .collect();
    let d = Document::new(1);
    let id = d.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: Length::from_mm(300.0),
        height: Length::from_mm(20.0),
    });
    shade(&d, id, GradientGeometry::DEFAULT_LINEAR, &stops);
    let list = build_artwork(&objects(&d), view());
    assert_eq!(list.gradient_stops().len(), 256);
    check_list(
        &list,
        &GradientGeometry::DEFAULT_LINEAR,
        &stops,
        Point::new(0.0, 0.0),
        Point::new(300.0, 20.0),
    );
}

#[test]
fn zero_size_and_huge_shapes_draw_without_panic() {
    for (x, w, h) in [(0.0, 0.0, 0.0), (5.0, 10.0, 0.0), (1.0e9, 1.0e6, 3.0)] {
        let d = Document::new(1);
        let id = d.create_rect(RectBounds {
            origin: Point::new(x, x),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        });
        shade(&d, id, GradientGeometry::DEFAULT_RADIAL, &stops6());
        let list = build_artwork(&objects(&d), view());
        for a in list.gradient_attributes() {
            assert!(a.iter().all(|x| x.is_finite()), "finite attributes: {a:?}");
        }
    }
}
