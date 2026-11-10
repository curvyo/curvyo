//! Stars and regular polygons of every count, inner ratio and turn through the tracer and the
//! shape pass (`specs/0088-trace-recognition` Part B criteria 3, 4 and 7; the customer's 8-point
//! star of 2026-11-03 that stayed a path), at screen resolution: radius 150 px.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names
)]

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use curvyo_ui_core::{
    TraceJob, TraceJoin, TraceMode, TracePixelMap, TracePrimitive, TraceRaster, run_trace_job,
};
use curvyo_ui_core::{TraceParams, read_trace_wire, trace_add_shapes};

/// 10 px/mm.
const MAP: [f64; 6] = [0.1, 0.0, 0.0, 0.1, 0.0, 0.0];
const SIDE: u32 = 340;
const RADIUS_PX: f64 = 150.0;

/// A star (or with `inner` None a polygon) of `n` corners about the picture's centre, its first
/// outer corner at `turn`, rendered with 4 x 4 sub-samples.
fn figure(n: u32, inner: Option<f64>, turn: f64) -> Vec<u8> {
    figure_at(n, inner, turn, RADIUS_PX, SIDE)
}

/// As [`figure`] with radius `radius` px in a picture of `side` px.
fn figure_at(n: u32, inner: Option<f64>, turn: f64, radius: f64, side: u32) -> Vec<u8> {
    let m = if inner.is_some() { 2 * n } else { n };
    let c = f64::from(side) / 2.0;
    let corners: Vec<(f64, f64)> = (0..m)
        .map(|j| {
            let r = if j % 2 == 1 {
                inner.unwrap_or(1.0)
            } else {
                1.0
            } * radius;
            let t = turn + TAU * f64::from(j) / f64::from(m);
            (c + r * t.cos(), c + r * t.sin())
        })
        .collect();
    let inside = |x: f64, y: f64| {
        let mut odd = false;
        for j in 0..corners.len() {
            let (a, b) = (corners[j], corners[(j + 1) % corners.len()]);
            if (a.1 > y) != (b.1 > y) && x < a.0 + (y - a.1) * (b.0 - a.0) / (b.1 - a.1) {
                odd = !odd;
            }
        }
        odd
    };
    let mut out = Vec::new();
    for py in 0..side {
        for px in 0..side {
            let mut dark = 0;
            for s in 0..16 {
                let x = f64::from(px) + (f64::from(s % 4) + 0.5) / 4.0;
                let y = f64::from(py) + (f64::from(s / 4) + 0.5) / 4.0;
                dark += u32::from(inside(x, y));
            }
            let g = u8::try_from(255 - dark * 255 / 16).unwrap();
            out.extend([g, g, g, 255]);
        }
    }
    out
}

/// What the shape pass made of the figure: its kind and count, and the outer radius in px.
fn recognised(rgba: &[u8]) -> Option<(&'static str, u32, f64, Option<f64>, f64)> {
    recognised_at(rgba, SIDE)
}

fn recognised_at(rgba: &[u8], side: u32) -> Option<(&'static str, u32, f64, Option<f64>, f64)> {
    let mut wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: side,
                height: side,
                rgba,
            },
            map: TracePixelMap::new(MAP),
            mode: TraceMode::Mono(&TraceParams::with_threshold(128)),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    trace_add_shapes(&mut wire, MAP);
    let read = read_trace_wire(&wire).unwrap();
    let region = read.shapes.first()?;
    Some(match *region.shapes[0].as_ref()? {
        (
            TracePrimitive::Star {
                radius,
                angle,
                points,
                inner_ratio,
                ..
            },
            _,
        ) => (
            "star",
            points,
            radius.as_mm() * 10.0,
            Some(inner_ratio),
            angle.as_radians(),
        ),
        (
            TracePrimitive::Polygon {
                radius,
                angle,
                points,
                ..
            },
            _,
        ) => (
            "polygon",
            points,
            radius.as_mm() * 10.0,
            None,
            angle.as_radians(),
        ),
        (TracePrimitive::Ellipse { rx, .. }, _) => ("ellipse", 0, rx.as_mm() * 10.0, None, 0.0),
        (TracePrimitive::Rect { .. }, _) => ("rect", 4, 0.0, None, 0.0),
    })
}

/// The turn `a` minus `b` in degrees, folded into the period of `n` corners.
fn turn_off(a: f64, b: f64, n: u32) -> f64 {
    let period = TAU / f64::from(n);
    let d = (a - b).rem_euclid(period);
    d.min(period - d).to_degrees()
}

const TURNS: [f64; 4] = [-FRAC_PI_2, 0.0, PI / 8.0, 0.3];

/// Every star from 3 to 16 points, inner ratio 0.2 to 0.8, a point up, on the x axis, at 22.5°
/// and at 0.3 rad is a star of its count, radii within 2 px (needles of a tip under 5°, ratio 0.2
/// and 11 points or more, within 1.5 % of the radius: their traced tips are thinned over tens of
/// pixels) and turn within 0.5°. Where the
/// valleys lie on the polygon's sides (ratio cos(π/n) within the tolerance) the polygon of n
/// (or the square) is the simplest fit and counts too.
#[test]
fn every_star_of_3_to_7_points_is_a_star_of_its_count() {
    stars(3..=7);
}

#[test]
fn every_star_of_8_to_12_points_is_a_star_of_its_count() {
    stars(8..=12);
}

#[test]
fn every_star_of_13_to_16_points_is_a_star_of_its_count() {
    stars(13..=16);
}

fn stars(counts: std::ops::RangeInclusive<u32>) {
    let mut failures = Vec::new();
    let mut tried = 0;
    for n in counts {
        for k in 2..=8 {
            let ratio = f64::from(k) / 10.0;
            for turn in TURNS {
                tried += 1;
                let got = recognised(&figure(n, Some(ratio), turn));
                let flat = ((ratio - (PI / f64::from(n)).cos()) * RADIUS_PX).abs() < 2.0;
                let ok = match got {
                    Some(("star", c, r, Some(q), a)) => {
                        let off = if k == 2 && n >= 11 {
                            0.015 * RADIUS_PX
                        } else {
                            2.0
                        };
                        c == n
                            && (r - RADIUS_PX).abs() <= off
                            && (q * r - ratio * RADIUS_PX).abs() <= off
                            && turn_off(a, turn, n) <= 0.5
                    }
                    Some(("polygon", c, ..)) => flat && c == n,
                    Some(("rect", ..)) => flat && n == 4,
                    _ => false,
                };
                if !ok {
                    failures.push(format!(
                        "n {n} ratio {ratio} turn {:.1}°: {got:?}",
                        turn.to_degrees()
                    ));
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {tried} failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Every regular polygon from 3 to 16 sides at the same turns is a polygon of its count (radius
/// within 2 px, turn within 0.5°); a square is a rectangle; one that is a circle within the
/// tolerance (its sides bulge less than 2 px from the mean circle) is the circle.
#[test]
fn every_regular_polygon_is_a_polygon_of_its_count() {
    let mut failures = Vec::new();
    for n in 3..=16_u32 {
        for turn in TURNS {
            let got = recognised(&figure(n, None, turn));
            let round = RADIUS_PX * (1.0 - (PI / f64::from(n)).cos()) / 2.0 < 2.5;
            let ok = match got {
                Some(("polygon", c, r, None, a)) => {
                    c == n && (r - RADIUS_PX).abs() <= 2.0 && turn_off(a, turn, n) <= 0.5
                }
                Some(("rect", ..)) => n == 4,
                Some(("ellipse", ..)) => round,
                _ => false,
            };
            if !ok {
                failures.push(format!("n {n} turn {:.1}°: {got:?}", turn.to_degrees()));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of 56 failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// The customer's screenshot of 2026-11-03 (`tests/fixtures/trace_shapes/customer-stars.png`,
/// cropped from the canvas, the selection frame painted out): the 15-point and the 8-point star
/// are both stars of their count. The ellipse and the rectangle above them are cut by the crop
/// (the toolbar covered their tops) and stay paths.
#[test]
fn the_customers_stars_are_stars() {
    let png = image::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/trace_shapes/customer-stars.png"),
    )
    .unwrap()
    .to_rgba8();
    let (w, h) = png.dimensions();
    let map = [
        0.264_583_333_333_333_34,
        0.0,
        0.0,
        0.264_583_333_333_333_34,
        0.0,
        0.0,
    ];
    let mut wire = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: w,
                height: h,
                rgba: png.as_raw(),
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&TraceParams::with_threshold(128)),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    trace_add_shapes(&mut wire, map);
    let read = read_trace_wire(&wire).unwrap();
    let mut stars: Vec<u32> = read
        .shapes
        .iter()
        .filter_map(|r| match *r.shapes[0].as_ref()? {
            (TracePrimitive::Star { points, .. }, _) => Some(points),
            _ => None,
        })
        .collect();
    stars.sort_unstable();
    eprintln!(
        "{} outlines; shapes {:?}",
        read.outlines.len(),
        read.shapes.iter().map(|r| &r.shapes).collect::<Vec<_>>()
    );
    assert_eq!(stars, [8, 15]);
}

/// The customer's page of 2026-11-03 (`tests/fixtures/trace_shapes/customer-page.png`, 794 x 1123
/// px at 96 dpi): an ellipse, a rounded rectangle, a filled 15-point star and an 8-point star
/// drawn as a thin outline. With Recognise shapes on, every region fills what it fills with the
/// switch off (within 2 %, nonzero as the document fills it), so the outline star keeps its hole.
#[test]
fn the_customers_page_keeps_every_hole() {
    let png = image::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/trace_shapes/customer-page.png"),
    )
    .unwrap()
    .to_rgba8();
    let (w, h) = png.dimensions();
    let map = [
        0.264_583_333_333_333_34,
        0.0,
        0.0,
        0.264_583_333_333_333_34,
        0.0,
        0.0,
    ];
    let off = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: w,
                height: h,
                rgba: png.as_raw(),
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&TraceParams::with_threshold(128)),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    let mut on = off.clone();
    trace_add_shapes(&mut on, map);
    let (off, on) = (
        read_trace_wire(&off).unwrap(),
        read_trace_wire(&on).unwrap(),
    );
    let signed = |p: &kurbo::BezPath| kurbo::Shape::area(p);
    let mut kinds = Vec::new();
    for region in &on.shapes {
        let before: f64 = region
            .outlines
            .iter()
            .map(|&i| signed(&off.outlines[i]))
            .sum();
        let after: f64 = region
            .outlines
            .iter()
            .map(|&i| signed(&on.outlines[i]))
            .sum();
        assert!(
            (after - before).abs() <= 0.02 * before.abs(),
            "region {:?}: fills {after:.1} mm² instead of {before:.1}",
            region.outlines
        );
        kinds.push((
            match region.shapes[0].as_ref().unwrap().0 {
                TracePrimitive::Star { points, .. } => format!("star{points}"),
                TracePrimitive::Polygon { points, .. } => format!("polygon{points}"),
                TracePrimitive::Ellipse { .. } => "ellipse".to_owned(),
                TracePrimitive::Rect { .. } => "rect".to_owned(),
            },
            region.outlines.len(),
            region.primitives(),
        ));
    }
    kinds.sort();
    eprintln!("{kinds:?}");
    let outline_star = kinds
        .iter()
        .find(|k| k.0 == "star8")
        .expect("the 8-point star is a star");
    assert_eq!(outline_star.1, 2, "the outline star keeps its hole");
    // Criterion 16 (Frage 119 A): the outline star is one star with a stroke, n = 8, covering
    // what the traced band covers within 2 %.
    let region = on.shapes.iter().find(|r| r.outlines.len() == 2).unwrap();
    let band = region.band.expect("a stroked star");
    let TracePrimitive::Star { points, .. } = band.primitive else {
        panic!("{band:?}");
    };
    assert_eq!(points, 8);
    let traced: f64 = region
        .outlines
        .iter()
        .map(|&i| kurbo::Shape::area(&off.outlines[i]))
        .sum::<f64>()
        .abs();
    let stroked = stroked_star_area(&band.primitive, band.width.as_mm());
    eprintln!(
        "outline star: width {:.3} mm, stroked {stroked:.1} mm², traced {traced:.1} mm²",
        band.width.as_mm()
    );
    assert!(
        (stroked - traced).abs() <= 0.02 * traced,
        "{stroked} vs {traced}"
    );
}

/// The area a mitred stroke of `width` along a star covers, from its two offset stars (each side
/// moved by half the width along its normal), by the area of a star from its radii.
fn stroked_star_area(shape: &TracePrimitive, width: f64) -> f64 {
    let TracePrimitive::Star {
        radius,
        points,
        inner_ratio,
        ..
    } = *shape
    else {
        return 0.0;
    };
    let n = f64::from(points);
    let (r, q) = (radius.as_mm(), inner_ratio);
    let h = PI / n;
    let (tip, valley) = ((r, 0.0), (r * q * h.cos(), r * q * h.sin()));
    let (ex, ey) = (valley.0 - tip.0, valley.1 - tip.1);
    let len = ex.hypot(ey);
    let normal = (ey / len, -ex / len);
    let p = normal.0 * tip.0;
    let area = |p: f64| {
        let t = p / normal.0;
        let v = p / (normal.0 * h.cos() + normal.1 * h.sin());
        n * t * v * h.sin()
    };
    area(p + width / 2.0) - area(p - width / 2.0)
}

/// The customer's page of 2026-11-04 (`tests/fixtures/trace_shapes/customer-outline-stars.png`,
/// 794 x 1123 px at 96 dpi): ten 8-point outline stars of several sizes, turns and stroke widths,
/// drawn with bevelled joins (the tips are cut flat). Criterion 16: each is one stroked star with
/// n = 8 covering what the traced band covers within 2 % (a stroke of its centre line with its
/// join; decision 22). The thinner ones are within the tolerance of a mitred outline and stay
/// mitred; the two thick ones (2 mm, bottom) have tips cut flat or clipped, which the tip zone
/// leaves to the join (customer 2026-11-04): each is the join nearest to its tips.
#[test]
fn the_customers_ten_outline_stars_are_stroked_stars() {
    let png = image::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/trace_shapes/customer-outline-stars.png"),
    )
    .unwrap()
    .to_rgba8();
    let (w, h) = png.dimensions();
    let map = [
        0.264_583_333_333_333_34,
        0.0,
        0.0,
        0.264_583_333_333_333_34,
        0.0,
        0.0,
    ];
    let off = run_trace_job(
        &TraceJob {
            raster: TraceRaster {
                width: w,
                height: h,
                rgba: png.as_raw(),
            },
            map: TracePixelMap::new(map),
            mode: TraceMode::Mono(&TraceParams::with_threshold(128)),
            lines: &[],
            mask: None,
            shapes: false,
        },
        &mut |_| {},
    );
    let mut on = off.clone();
    trace_add_shapes(&mut on, map);
    let (off, on) = (
        read_trace_wire(&off).unwrap(),
        read_trace_wire(&on).unwrap(),
    );
    assert_eq!(off.outlines.len(), 20, "ten stars of two outlines each");
    let mut failures = Vec::new();
    let mut bevelled = 0;
    for region in &on.shapes {
        let traced: f64 = region
            .outlines
            .iter()
            .map(|&i| kurbo::Shape::area(&off.outlines[i]))
            .sum::<f64>()
            .abs();
        let Some(band) = region.band else {
            failures.push(format!("{:?}: no band", region.outlines));
            continue;
        };
        let TracePrimitive::Star { points, .. } = band.primitive else {
            panic!("{band:?}");
        };
        let stroked = stroke_cover(&band.primitive, band.width.as_mm(), band.join);
        let off_by = 100.0 * (stroked - traced) / traced;
        eprintln!(
            "{:?}: width {:.3} mm, {:?}, stroked {stroked:.1} mm², traced {traced:.1} mm² ({off_by:+.1} %)",
            region.outlines,
            band.width.as_mm(),
            band.join
        );
        bevelled += usize::from(band.join == TraceJoin::Bevel);
        if points != 8 || off_by.abs() > 2.0 {
            failures.push(format!(
                "{:?}: n {}, {off_by:+.1} %",
                region.outlines, points
            ));
        }
    }
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(on.shapes.len(), 10, "one stroked star each, no path left");
    assert!(bevelled >= 1, "the bottom-right star is bevelled");
}

/// The area kurbo's stroke of `width` with `join` along the star `shape` covers (an oracle apart
/// from the shape pass), by a winding-number grid of 0.1 mm.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn stroke_cover(shape: &TracePrimitive, width: f64, join: TraceJoin) -> f64 {
    let TracePrimitive::Star {
        centre,
        radius,
        angle,
        points,
        inner_ratio,
    } = *shape
    else {
        return 0.0;
    };
    let m = 2 * points;
    let mut path = kurbo::BezPath::new();
    for j in 0..m {
        let r = radius.as_mm() * if j % 2 == 1 { inner_ratio } else { 1.0 };
        let a = angle.as_radians() + TAU * f64::from(j) / f64::from(m);
        let p = (centre.x + r * a.cos(), centre.y + r * a.sin());
        if j == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    let style = kurbo::Stroke::new(width)
        .with_join(match join {
            TraceJoin::Bevel => kurbo::Join::Bevel,
            TraceJoin::Round => kurbo::Join::Round,
            TraceJoin::Miter => kurbo::Join::Miter,
        })
        .with_miter_limit(4.0);
    let outline = kurbo::stroke(path.iter(), &style, &kurbo::StrokeOpts::default(), 0.01);
    let bb = kurbo::Shape::bounding_box(&outline);
    let step = 0.1;
    let (nx, ny) = (
        ((bb.x1 - bb.x0) / step).ceil() as u32,
        ((bb.y1 - bb.y0) / step).ceil() as u32,
    );
    let mut hits = 0u64;
    for j in 0..ny {
        for i in 0..nx {
            let p = kurbo::Point::new(
                bb.x0 + (f64::from(i) + 0.5) * step,
                bb.y0 + (f64::from(j) + 0.5) * step,
            );
            if kurbo::Shape::winding(&outline, p) != 0 {
                hits += 1;
            }
        }
    }
    hits as f64 * step * step
}
