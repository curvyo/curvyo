//! Recognise shapes on traced pictures (`specs/0088-trace-recognition` Part B criteria 1 to 5, 7
//! to 11 and 13): pictures rendered at 10 px/mm (so the 2 px floor is 0.2 mm), traced with the
//! real tracer, the shape section added and read back.

#![allow(
    clippy::unwrap_used,
    clippy::many_single_char_names,
    clippy::similar_names
)]

use curvyo_document_core::{Angle, Shape};
use curvyo_ui_core::{
    TraceColourParams, TraceMethod, TraceParams, TraceRgb, TraceShapeRegion, TraceWire,
    read_trace_wire, trace_add_shapes, trace_colour_job_with, trace_job_with,
};

/// 10 px/mm.
const MAP: [f64; 6] = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];

/// A grey picture of `w` x `h` px whose pixel is dark where `inside` holds for its 4 x 4
/// sub-samples (document mm under [`MAP`]), anti-aliased as a scan is.
fn picture(w: u32, h: u32, inside: impl Fn(f64, f64) -> bool) -> Vec<u8> {
    picture_rgb(w, h, |x, y| inside(x, y).then_some([0, 0, 0]))
}

/// A colour picture on white: each sub-sample takes the colour `paint` gives it.
fn picture_rgb(w: u32, h: u32, paint: impl Fn(f64, f64) -> Option<[u8; 3]>) -> Vec<u8> {
    let mut out = Vec::with_capacity((w * h * 4) as usize);
    for py in 0..h {
        for px in 0..w {
            let mut sum = [0u32; 3];
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = (f64::from(px) + (f64::from(sx) + 0.5) / 4.0) * 0.1;
                    let y = (f64::from(py) + (f64::from(sy) + 0.5) / 4.0) * 0.1;
                    let c = paint(x, y).unwrap_or([255, 255, 255]);
                    for k in 0..3 {
                        sum[k] += u32::from(c[k]);
                    }
                }
            }
            for s in sum {
                out.push(u8::try_from(s / 16).unwrap());
            }
            out.push(255);
        }
    }
    out
}

fn traced(rgba: &[u8], w: u32, h: u32, shapes: bool) -> (Vec<f64>, TraceWire) {
    let mut wire = trace_job_with(
        rgba,
        w,
        h,
        MAP,
        &TraceParams::with_threshold(128),
        &mut |_| {},
    );
    if shapes {
        trace_add_shapes(&mut wire, MAP);
    }
    let read = read_trace_wire(&wire).unwrap();
    (wire, read)
}

/// The one region of a result and its outer primitive.
fn only(read: &TraceWire) -> (&TraceShapeRegion, &(Shape, Angle)) {
    assert_eq!(read.shapes.len(), 1, "{:?}", read.shapes);
    let region = &read.shapes[0];
    (region, region.shapes[0].as_ref().unwrap())
}

fn near(a: f64, b: f64, within: f64) -> bool {
    (a - b).abs() <= within
}

/// The angle `a` minus `b`, folded into a period of `period` radians, in degrees.
fn degrees_off(a: f64, b: f64, period: f64) -> f64 {
    let d = (a - b).rem_euclid(period);
    d.min(period - d).to_degrees()
}

fn rounded_box(x: f64, y: f64, cx: f64, cy: f64, w: f64, h: f64, r: f64) -> bool {
    let qx = (x - cx).abs() - (w / 2.0 - r);
    let qy = (y - cy).abs() - (h / 2.0 - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) <= r
}

/// Criteria 1 and 7: a 200 x 100 mm rectangle with 10 mm corners is one rounded rectangle.
#[test]
fn c1_a_rounded_rectangle_is_a_rectangle_with_its_corners() {
    let rgba = picture(2200, 1200, |x, y| {
        rounded_box(x, y, 110.0, 60.0, 200.0, 100.0, 10.0)
    });
    let (_, read) = traced(&rgba, 2200, 1200, true);
    let (region, (shape, turn)) = only(&read);
    assert_eq!(region.outlines.len(), 1);
    let Shape::Rect {
        bounds,
        corner_radii,
    } = shape
    else {
        panic!("{shape:?}");
    };
    eprintln!("{bounds:?} {corner_radii:?} {turn:?}");
    assert!(near(bounds.width.as_mm(), 200.0, 0.2), "{bounds:?}");
    assert!(near(bounds.height.as_mm(), 100.0, 0.2), "{bounds:?}");
    assert!(near(corner_radii.tl.as_mm(), 10.0, 0.2), "{corner_radii:?}");
    assert!(near(bounds.origin.x, 10.0, 0.2) && near(bounds.origin.y, 10.0, 0.2));
    assert!(turn.as_radians().abs().to_degrees() < 0.5);
}

/// Criterion 7: a square-cornered rectangle is not a rounded one.
#[test]
fn c7_a_square_cornered_rectangle_has_no_corner_radius() {
    let rgba = picture(800, 600, |x, y| {
        (10.0..70.0).contains(&x) && (10.0..50.0).contains(&y)
    });
    let (_, read) = traced(&rgba, 800, 600, true);
    let (_, (shape, _)) = only(&read);
    let Shape::Rect {
        bounds,
        corner_radii,
    } = shape
    else {
        panic!("{shape:?}");
    };
    assert_eq!(corner_radii.tl.as_mm(), 0.0);
    assert!(near(bounds.width.as_mm(), 60.0, 0.2) && near(bounds.height.as_mm(), 40.0, 0.2));
}

/// Criteria 2 and 7: a 50 mm circle is a circle (not an ellipse).
#[test]
fn c2_a_circle_is_a_circle() {
    let rgba = picture(600, 600, |x, y| (x - 30.0).hypot(y - 30.0) <= 25.0);
    let (_, read) = traced(&rgba, 600, 600, true);
    let (_, (shape, _)) = only(&read);
    let Shape::Ellipse { frame, .. } = shape else {
        panic!("{shape:?}");
    };
    eprintln!("{frame:?}");
    assert_eq!(frame.rx, frame.ry, "a circle");
    assert!(near(frame.rx.as_mm() * 2.0, 50.0, 0.2));
    assert!(near(frame.center.x, 30.0, 0.2) && near(frame.center.y, 30.0, 0.2));
}

/// Criterion 2: a 60 x 40 mm ellipse turned by 25 degrees keeps its radii and turn.
#[test]
fn c2_a_turned_ellipse_has_its_radii_and_turn() {
    let (s, c) = 25_f64.to_radians().sin_cos();
    let rgba = picture(800, 800, |x, y| {
        let (dx, dy) = (x - 40.0, y - 40.0);
        let (u, v) = (dx * c + dy * s, -dx * s + dy * c);
        (u / 30.0).powi(2) + (v / 20.0).powi(2) <= 1.0
    });
    let (_, read) = traced(&rgba, 800, 800, true);
    let (_, (shape, turn)) = only(&read);
    let Shape::Ellipse { frame, .. } = shape else {
        panic!("{shape:?}");
    };
    eprintln!("{frame:?} {}", turn.as_radians().to_degrees());
    assert!(near(frame.rx.as_mm(), 30.0, 0.2) && near(frame.ry.as_mm(), 20.0, 0.2));
    assert!(degrees_off(turn.as_radians(), 25_f64.to_radians(), std::f64::consts::PI) < 0.5);
}

fn regular(x: f64, y: f64, n: u32, outer: f64, inner: Option<f64>, turn: f64) -> bool {
    // Even-odd over the figure's corners about (40, 40).
    let m = if inner.is_some() { 2 * n } else { n };
    let corner = |j: u32| {
        let r = if j % 2 == 1 {
            inner.unwrap_or(outer)
        } else {
            outer
        };
        let t = turn + std::f64::consts::TAU * f64::from(j) / f64::from(m);
        (40.0 + r * t.cos(), 40.0 + r * t.sin())
    };
    let mut inside = false;
    for j in 0..m {
        let (a, b) = (corner(j), corner((j + 1) % m));
        if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1) {
            inside = !inside;
        }
    }
    inside
}

/// Criterion 4: a regular hexagon is a polygon of 6 sides.
#[test]
fn c4_a_hexagon_is_a_polygon_of_six() {
    let rgba = picture(800, 800, |x, y| regular(x, y, 6, 30.0, None, 0.3));
    let (_, read) = traced(&rgba, 800, 800, true);
    let (_, (shape, _)) = only(&read);
    let Shape::Polygon {
        frame, point_count, ..
    } = shape
    else {
        panic!("{shape:?}");
    };
    eprintln!("{frame:?}");
    assert_eq!(point_count.get(), 6);
    assert!(near(frame.radius.as_mm(), 30.0, 0.2));
}

/// Criteria 3 and 7: a five-point star, outer 40 mm, ratio 0.5, is a star with its radii and
/// turn (a polygon would not fit).
#[test]
fn c3_a_five_point_star_has_its_radii_and_turn() {
    let turn = -0.4;
    let rgba = picture(800, 800, |x, y| regular(x, y, 5, 40.0, Some(20.0), turn));
    let (_, read) = traced(&rgba, 800, 800, true);
    let (_, (shape, _)) = only(&read);
    let Shape::Star {
        frame,
        point_count,
        inner_ratio,
        ..
    } = shape
    else {
        panic!("{shape:?}");
    };
    eprintln!("{frame:?} {}", inner_ratio.get());
    assert_eq!(point_count.get(), 5);
    assert!(near(frame.radius.as_mm(), 40.0, 0.2));
    assert!(near(frame.radius.as_mm() * inner_ratio.get(), 20.0, 0.2));
    let period = std::f64::consts::TAU / 5.0;
    assert!(degrees_off(frame.angle.as_radians(), turn, period) < 0.5);
}

/// Criterion 5: a hand-drawn wobbly circle (radius 25 mm varying by about ±3 %, not regularly)
/// stays a path, unchanged.
#[test]
fn c5_a_wobbly_circle_stays_a_path() {
    let rgba = picture(600, 600, |x, y| {
        let (dx, dy) = (x - 30.0, y - 30.0);
        let t = dy.atan2(dx);
        let wobble = 0.018 * (3.0 * t + 0.4).sin() + 0.012 * (5.0 * t + 1.9).sin();
        dx.hypot(dy) <= 25.0 * (1.0 + wobble)
    });
    let (on_wire, on) = traced(&rgba, 600, 600, true);
    let (off_wire, off) = traced(&rgba, 600, 600, false);
    assert_eq!(on.shapes, []);
    assert_eq!(on.outlines, off.outlines);
    assert_eq!(on_wire[..off_wire.len()], off_wire[..]);
}

/// Criterion 8: a ring is one region of two circles, a rectangle frame one of two rectangles.
#[test]
fn c8_holes_are_kept() {
    let ring = picture(700, 700, |x, y| {
        let r = (x - 35.0).hypot(y - 35.0);
        (15.0..=30.0).contains(&r)
    });
    let (_, read) = traced(&ring, 700, 700, true);
    let (region, _) = only(&read);
    assert_eq!(region.outlines.len(), 2);
    let radii: Vec<f64> = region
        .shapes
        .iter()
        .map(|s| match s {
            Some((Shape::Ellipse { frame, .. }, _)) if frame.rx == frame.ry => frame.rx.as_mm(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert!(
        near(radii[0], 30.0, 0.2) && near(radii[1], 15.0, 0.2),
        "{radii:?}"
    );
    // Criterion 16 (Frage 119 A): the ring is a band, a circle at its centre line with a stroke.
    let band = region.band.unwrap();
    let Shape::Ellipse { frame, .. } = band.shape else {
        panic!("{band:?}");
    };
    assert!(
        frame.rx == frame.ry && near(frame.rx.as_mm(), 22.5, 0.2),
        "{band:?}"
    );
    assert!(near(band.width.as_mm(), 15.0, 0.2), "{band:?}");
    let frame = picture(1400, 1000, |x, y| {
        let outer = (10.0..130.0).contains(&x) && (10.0..90.0).contains(&y);
        let hole = (30.0..110.0).contains(&x) && (30.0..70.0).contains(&y);
        outer && !hole
    });
    let (_, read) = traced(&frame, 1400, 1000, true);
    let (region, _) = only(&read);
    let sizes: Vec<(f64, f64)> = region
        .shapes
        .iter()
        .map(|s| match s {
            Some((Shape::Rect { bounds, .. }, _)) => (bounds.width.as_mm(), bounds.height.as_mm()),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(sizes.len(), 2);
    assert!(
        near(sizes[0].0, 120.0, 0.2) && near(sizes[1].1, 40.0, 0.2),
        "{sizes:?}"
    );
    // Criterion 16: a square-cornered rectangle at the centre line, a 20 mm stroke.
    let band = region.band.unwrap();
    let Shape::Rect {
        bounds,
        corner_radii,
    } = band.shape
    else {
        panic!("{band:?}");
    };
    assert!(near(bounds.width.as_mm(), 100.0, 0.2) && near(bounds.height.as_mm(), 60.0, 0.2));
    assert_eq!(corner_radii.tl.as_mm(), 0.0);
    assert!(near(band.width.as_mm(), 20.0, 0.2), "{band:?}");
}

/// Criterion 16: a hexagon outline (outer radius 30 mm, sides 3 mm apart) is a hexagon at the
/// centre line with a 3 mm stroke; bands of varying width (a ring whose hole is off centre, a
/// frame with borders of 20 and 10 mm) stay two clean outlines.
#[test]
fn c16_bands_of_constant_width_are_stroked_primitives() {
    let apothem = 30.0 * (std::f64::consts::PI / 6.0).cos();
    let inner = (apothem - 3.0) / (std::f64::consts::PI / 6.0).cos();
    let hexagons = picture(800, 800, |x, y| {
        regular(x, y, 6, 30.0, None, 0.2) && !regular(x, y, 6, inner, None, 0.2)
    });
    let (_, read) = traced(&hexagons, 800, 800, true);
    let (region, _) = only(&read);
    let band = region.band.unwrap();
    let Shape::Polygon {
        frame, point_count, ..
    } = band.shape
    else {
        panic!("{band:?}");
    };
    assert_eq!(point_count.get(), 6);
    let mid = (apothem - 1.5) / (std::f64::consts::PI / 6.0).cos();
    assert!(near(frame.radius.as_mm(), mid, 0.2), "{band:?}");
    assert!(near(band.width.as_mm(), 3.0, 0.2), "{band:?}");
    assert_eq!(region.primitives(), 1);
    let off_centre = picture(700, 700, |x, y| {
        (x - 35.0).hypot(y - 35.0) <= 30.0 && (x - 39.0).hypot(y - 35.0) > 15.0
    });
    let uneven = picture(1400, 1000, |x, y| {
        let outer = (10.0..130.0).contains(&x) && (10.0..90.0).contains(&y);
        let hole = (30.0..110.0).contains(&x) && (20.0..80.0).contains(&y);
        outer && !hole
    });
    for (name, rgba, w, h) in [
        ("off centre", off_centre, 700, 700),
        ("uneven", uneven, 1400, 1000),
    ] {
        let (_, read) = traced(&rgba, w, h, true);
        let (region, _) = only(&read);
        assert_eq!(region.band, None, "{name}");
        assert_eq!(region.primitives(), 2, "{name}: two clean outlines");
    }
}

/// Criterion 9: the `0025` ring (radii 20 and 60 px in 200 x 200 px) and dot grid (3 px dots
/// every 8 px in 800 x 800 px), whatever the pixel size.
#[test]
fn c9_the_0025_ring_and_dot_grid() {
    let grey = |w: u32, h: u32, dark: &dyn Fn(f64, f64) -> bool| -> Vec<u8> {
        (0..w * h)
            .flat_map(|i| {
                let (x, y) = (f64::from(i % w) + 0.5, f64::from(i / w) + 0.5);
                let g = if dark(x, y) { 0 } else { 255 };
                [g, g, g, 255]
            })
            .collect()
    };
    let ring = grey(200, 200, &|x, y| {
        let r = (x - 100.0).hypot(y - 100.0);
        r < 60.0 && r > 20.0
    });
    let dots = grey(800, 800, &|x, y| {
        let (dx, dy) = ((x - 0.5) % 8.0 - 3.5, (y - 0.5) % 8.0 - 3.5);
        dx * dx + dy * dy < 9.0
    });
    for map in [[0.264_583_333, 0.0, 0.0, 0.264_583_333, 0.0, 0.0], MAP] {
        let shape_wire = |rgba: &[u8], side: u32| {
            let mut wire = trace_job_with(
                rgba,
                side,
                side,
                map,
                &TraceParams::with_threshold(128),
                &mut |_| {},
            );
            trace_add_shapes(&mut wire, map);
            read_trace_wire(&wire).unwrap()
        };
        let read = shape_wire(&ring, 200);
        let (region, _) = only(&read);
        // Criterion 16: the ring is one stroked circle.
        assert_eq!(region.primitives(), 1, "{:?}", region.shapes);
        assert!(region.band.is_some());
        let read = shape_wire(&dots, 800);
        assert_eq!(read.outlines.len(), 10_000);
        assert_eq!(read.shapes.len(), 10_000);
        assert!(read.shapes.iter().all(|r| {
            r.outlines.len() == 1
                && matches!(&r.shapes[0], Some((Shape::Ellipse { frame, .. }, _)) if frame.rx == frame.ry)
        }));
    }
}

/// Criterion 10: in Multicolour each primitive stays in its colour's layer.
#[test]
fn c10_multicolour_keeps_each_colour() {
    let (red, blue) = ([208, 34, 42], [35, 78, 184]);
    let rgba = picture_rgb(1000, 600, |x, y| {
        if (x - 25.0).hypot(y - 30.0) <= 15.0 {
            Some(red)
        } else if (50.0..90.0).contains(&x) && (10.0..50.0).contains(&y) {
            Some(blue)
        } else {
            None
        }
    });
    let params = TraceColourParams::new(TraceParams {
        selection: TraceMethod::RemoveBackground {
            background: Some(TraceRgb::new(255, 255, 255)),
            tolerance: 40,
        },
        ..TraceParams::with_threshold(128)
    });
    let mut wire = trace_colour_job_with(&rgba, 1000, 600, MAP, &params, &mut |_| {});
    trace_add_shapes(&mut wire, MAP);
    let read = read_trace_wire(&wire).unwrap();
    assert_eq!(read.layers.len(), 2);
    let layer_of = |i: usize| {
        let mut at = 0;
        read.layers
            .iter()
            .find(|l| {
                at += l.outlines;
                i < at
            })
            .unwrap()
            .colour
    };
    let kinds: Vec<(TraceRgb, &'static str)> = read
        .shapes
        .iter()
        .map(|r| {
            let kind = match &r.shapes[0] {
                Some((Shape::Ellipse { .. }, _)) => "circle",
                Some((Shape::Rect { .. }, _)) => "rect",
                other => panic!("{other:?}"),
            };
            (layer_of(r.outlines[0]), kind)
        })
        .collect();
    assert!(
        kinds.contains(&(TraceRgb::new(208, 34, 42), "circle")),
        "{kinds:?}"
    );
    assert!(
        kinds.contains(&(TraceRgb::new(35, 78, 184), "rect")),
        "{kinds:?}"
    );
    assert_eq!(kinds.len(), 2);
}

/// Criteria 11 and 13: with the section off the wire is the trace's alone; with it on, the
/// numbers before the section are the same and two runs give the same bits.
#[test]
fn c11_c13_off_is_today_and_runs_repeat() {
    let rgba = picture(800, 800, |x, y| regular(x, y, 5, 30.0, Some(14.0), 0.7));
    let plain = trace_job_with(
        &rgba,
        800,
        800,
        MAP,
        &TraceParams::with_threshold(128),
        &mut |_| {},
    );
    let (off, _) = traced(&rgba, 800, 800, false);
    assert_eq!(off, plain);
    let (on, _) = traced(&rgba, 800, 800, true);
    let (again, _) = traced(&rgba, 800, 800, true);
    assert_eq!(on, again, "bit for bit");
    assert_eq!(on[..plain.len()], plain[..]);
}
