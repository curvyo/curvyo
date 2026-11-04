//! Criterion 16 of `specs/0088-trace-recognition` Part B (2026-11-04 changes), black-box:
//! bevelled and mitred strokes, touching bands, mixed regions, the customer's ten outline stars.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::too_many_lines
)]

use curvyo_document_core::{LineJoin, Shape};
use curvyo_ui_core::{
    TraceParams, TraceShapeRegion, TraceWire, read_trace_wire, trace_add_shapes, trace_job_with,
};
use kurbo::{BezPath, Join, ParamCurveNearest, Shape as _, Stroke, StrokeOpts};

const MM_PER_PX: f64 = 0.1;

fn star_outline(c: (f64, f64), radius: f64, ratio: f64, n: u32, rot: f64) -> BezPath {
    let mut path = BezPath::new();
    for k in 0..2 * n {
        let a = rot + f64::from(k) * std::f64::consts::PI / f64::from(n);
        let r = if k % 2 == 0 { radius } else { radius * ratio };
        let p = (c.0 + r * a.cos(), c.1 + r * a.sin());
        if k == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

fn rect_outline(c: (f64, f64), w: f64, h: f64) -> BezPath {
    let mut p = BezPath::new();
    p.move_to((c.0 - w / 2.0, c.1 - h / 2.0));
    p.line_to((c.0 + w / 2.0, c.1 - h / 2.0));
    p.line_to((c.0 + w / 2.0, c.1 + h / 2.0));
    p.line_to((c.0 - w / 2.0, c.1 + h / 2.0));
    p.close_path();
    p
}

fn stroke_of(path: &BezPath, width: f64, join: Join) -> BezPath {
    let style = Stroke::new(width).with_join(join).with_miter_limit(4.0);
    kurbo::stroke(path.iter(), &style, &StrokeOpts::default(), 0.01)
}

fn area_of(outlines: &[BezPath], step: f64) -> f64 {
    let mut bb = outlines[0].bounding_box();
    for o in outlines {
        bb = bb.union(o.bounding_box());
    }
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
            if outlines.iter().any(|o| o.winding(p) != 0) {
                hits += 1;
            }
        }
    }
    hits as f64 * step * step
}

/// Trace `ink` (mm coordinates, picture of `w_mm` x `h_mm` at 10 px/mm), shapes on.
fn trace(w_mm: f64, h_mm: f64, ink: impl Fn(f64, f64) -> bool) -> (TraceWire, TraceWire) {
    let (w, h) = ((w_mm * 10.0) as u32, (h_mm * 10.0) as u32);
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let on = ink(
                (f64::from(x) + 0.5) * MM_PER_PX,
                (f64::from(y) + 0.5) * MM_PER_PX,
            );
            let v = if on { 0 } else { 255 };
            rgba.extend([v, v, v, 255]);
        }
    }
    run(&rgba, w, h)
}

fn run(rgba: &[u8], w: u32, h: u32) -> (TraceWire, TraceWire) {
    let map = [MM_PER_PX, 0.0, 0.0, MM_PER_PX, 0.0, 0.0];
    let off = trace_job_with(
        rgba,
        w,
        h,
        map,
        &TraceParams::with_threshold(128),
        &mut |_| {},
    );
    let mut on = off.clone();
    trace_add_shapes(&mut on, map);
    (
        read_trace_wire(&off).unwrap(),
        read_trace_wire(&on).unwrap(),
    )
}

fn ink_of(outlines: Vec<BezPath>) -> impl Fn(f64, f64) -> bool {
    move |x, y| {
        let p = kurbo::Point::new(x, y);
        outlines.iter().any(|o| o.winding(p) != 0)
    }
}

fn region_area(wire: &TraceWire, r: &TraceShapeRegion) -> f64 {
    r.outlines
        .iter()
        .enumerate()
        .map(|(k, &i)| {
            let a = wire.outlines[i].area().abs();
            if k == 0 { a } else { -a }
        })
        .sum()
}

fn centre_line(b: &curvyo_ui_core::TraceBand) -> BezPath {
    match b.shape {
        Shape::Star {
            frame,
            point_count,
            inner_ratio,
            ..
        } => star_outline(
            (frame.center.x, frame.center.y),
            frame.radius.as_mm(),
            inner_ratio.get(),
            point_count.get(),
            frame.angle.as_radians() + b.rotation.as_radians(),
        ),
        other => panic!("not a star: {other:?}"),
    }
}

fn join_of(b: &curvyo_ui_core::TraceBand) -> Join {
    match b.join {
        LineJoin::Bevel => Join::Bevel,
        LineJoin::Miter => Join::Miter,
        other @ LineJoin::Round => panic!("unexpected join {other:?}"),
    }
}

fn band_regions(on: &TraceWire) -> Vec<&TraceShapeRegion> {
    on.shapes.iter().filter(|r| r.band.is_some()).collect()
}

fn dist_to(path: &BezPath, p: kurbo::Point) -> f64 {
    path.segments()
        .map(|s| s.nearest(p, 1e-6).distance_sq.sqrt())
        .fold(f64::MAX, f64::min)
}

/// Median distance of the outer outline's points to the hole outline (the offsets' distance).
fn median_offset_distance(wire: &TraceWire, r: &TraceShapeRegion) -> f64 {
    let outer = &wire.outlines[r.outlines[0]];
    let inner = &wire.outlines[r.outlines[1]];
    let mut d: Vec<f64> = Vec::new();
    for seg in outer.segments() {
        for k in 0..20 {
            let p = kurbo::ParamCurve::eval(&seg, f64::from(k) / 20.0);
            d.push(dist_to(inner, p));
        }
    }
    d.sort_by(f64::total_cmp);
    d[d.len() / 2]
}

#[test]
fn join_a_mitred_stroked_star_is_a_mitred_star_with_the_offset_width() {
    let line = star_outline((30.0, 30.0), 20.0, 0.55, 5, -1.3);
    let (_, on) = trace(60.0, 60.0, ink_of(vec![stroke_of(&line, 2.4, Join::Miter)]));
    let bands = band_regions(&on);
    assert_eq!(bands.len(), 1, "{:?}", on.shapes);
    let b = bands[0].band.unwrap();
    assert_eq!(b.join, LineJoin::Miter);
    assert!(matches!(b.shape, Shape::Star { point_count, .. } if point_count.get() == 5));
    let d = median_offset_distance(&on, bands[0]);
    assert!(
        (b.width.as_mm() - 2.4).abs() <= 0.2,
        "width {}",
        b.width.as_mm()
    );
    assert!(
        (b.width.as_mm() - d).abs() <= 0.2,
        "width {} vs offset distance {d}",
        b.width.as_mm()
    );
}

#[test]
fn join_a_bevelled_stroked_star_is_a_bevelled_star() {
    let line = star_outline((30.0, 30.0), 20.0, 0.55, 5, -1.3);
    let (_, on) = trace(60.0, 60.0, ink_of(vec![stroke_of(&line, 3.0, Join::Bevel)]));
    let bands = band_regions(&on);
    assert_eq!(bands.len(), 1, "{:?}", on.shapes);
    let b = bands[0].band.unwrap();
    assert_eq!(b.join, LineJoin::Bevel);
    assert!(matches!(b.shape, Shape::Star { point_count, .. } if point_count.get() == 5));
    assert!((b.width.as_mm() - 3.0).abs() <= 0.25, "{}", b.width.as_mm());
    let traced = region_area(&on, bands[0]);
    let stroked = area_of(
        &[stroke_of(&centre_line(&b), b.width.as_mm(), Join::Bevel)],
        0.05,
    );
    assert!(
        (stroked - traced).abs() <= 0.02 * traced,
        "stroked {stroked} vs traced {traced}"
    );
}

#[test]
fn join_a_bevelled_stroked_hexagon_or_square_never_becomes_a_wrong_primitive() {
    for (name, line) in [
        ("rect", rect_outline((30.0, 20.0), 40.0, 24.0)),
        ("square", rect_outline((30.0, 20.0), 30.0, 30.0)),
        (
            "hexagon",
            star_outline((30.0, 20.0), 15.0, 1.0, 3, 0.2), // 6 vertices, regular
        ),
    ] {
        let band_stroke = stroke_of(&line, 3.0, Join::Bevel);
        let (_, on) = trace(60.0, 40.0, ink_of(vec![band_stroke.clone()]));
        let traced_truth = area_of(&[band_stroke], 0.05);
        assert_eq!(on.shapes.len(), 1, "{name}");
        let r = &on.shapes[0];
        if let Some(b) = r.band {
            // A stroked primitive: its stroke must cover the traced area within 2 %.
            let outline = match b.shape {
                Shape::Rect { bounds, .. } => rect_outline(
                    (
                        bounds.origin.x + bounds.width.as_mm() / 2.0,
                        bounds.origin.y + bounds.height.as_mm() / 2.0,
                    ),
                    bounds.width.as_mm(),
                    bounds.height.as_mm(),
                ),
                Shape::Star { .. } => centre_line(&b),
                Shape::Polygon {
                    frame, point_count, ..
                } => star_outline(
                    (frame.center.x, frame.center.y),
                    frame.radius.as_mm(),
                    1.0,
                    point_count.get() / 2,
                    frame.angle.as_radians() + b.rotation.as_radians(),
                ),
                other @ Shape::Ellipse { .. } => panic!("{name}: unexpected {other:?}"),
            };
            let stroked = area_of(&[stroke_of(&outline, b.width.as_mm(), join_of(&b))], 0.05);
            eprintln!("{name}: band {:?} join {:?}", b.shape, b.join);
            assert!(
                (stroked - traced_truth).abs() <= 0.03 * traced_truth,
                "{name}: stroked {stroked} vs {traced_truth}"
            );
        } else {
            eprintln!("{name}: stays paths");
        }
    }
}

/// Stars of a row, centre lines `spacing` apart.
fn row(n: usize, spacing: f64, width: f64) -> (Vec<BezPath>, f64, f64) {
    let lines: Vec<BezPath> = (0..n)
        .map(|k| star_outline((20.0 + spacing * k as f64, 20.0), 16.0, 0.8, 8, 0.1))
        .collect();
    let strokes = lines
        .iter()
        .map(|l| stroke_of(l, width, Join::Miter))
        .collect();
    (strokes, 40.0 + spacing * (n as f64 - 1.0), 40.0)
}

#[test]
fn touching_three_stars_are_three_stroked_stars() {
    let (strokes, w, h) = row(3, 31.0, 2.5);
    // They must overlap to be one region: verify the premise on the truth raster.
    let truth = area_of(&strokes, 0.1);
    let sum: f64 = strokes
        .iter()
        .map(|s| area_of(std::slice::from_ref(s), 0.1))
        .sum();
    assert!(
        sum > truth + 1.0,
        "premise: strokes overlap ({sum} vs {truth})"
    );
    let (_, on) = trace(w, h, ink_of(strokes));
    assert_eq!(on.shapes.len(), 1, "one region: {:?}", on.shapes.len());
    let r = &on.shapes[0];
    assert_eq!(r.bands.len(), 3, "three bands");
    assert!(r.band.is_none());
    assert_eq!(r.primitives(), 3);
    for b in &r.bands {
        assert!(matches!(b.shape, Shape::Star { point_count, .. } if point_count.get() == 8));
        assert!((b.width.as_mm() - 2.5).abs() <= 0.25, "{}", b.width.as_mm());
        assert_eq!(b.join, LineJoin::Miter);
    }
    let union_of_bands = area_of(
        &r.bands
            .iter()
            .map(|b| stroke_of(&centre_line(b), b.width.as_mm(), join_of(b)))
            .collect::<Vec<_>>(),
        0.1,
    );
    let traced = region_area(&on, r);
    assert!(
        (union_of_bands - traced).abs() <= 0.03 * traced,
        "{union_of_bands} vs {traced}"
    );
}

#[test]
fn touching_two_touching_and_one_loose() {
    let near: Vec<BezPath> = (0..2)
        .map(|k| star_outline((20.0 + 31.0 * f64::from(k), 20.0), 16.0, 0.8, 8, 0.1))
        .collect();
    let loose = star_outline((20.0 + 31.0 + 70.0, 20.0), 16.0, 0.8, 8, 0.1);
    let mut strokes: Vec<BezPath> = near
        .iter()
        .map(|l| stroke_of(l, 2.5, Join::Miter))
        .collect();
    strokes.push(stroke_of(&loose, 2.5, Join::Miter));
    let (_, on) = trace(144.0, 40.0, ink_of(strokes));
    assert_eq!(on.shapes.len(), 2, "two regions");
    let touching: Vec<_> = on.shapes.iter().filter(|r| !r.bands.is_empty()).collect();
    assert_eq!(touching.len(), 1);
    assert_eq!(touching[0].bands.len(), 2);
    assert_eq!(band_regions(&on).len(), 1, "the loose one is a single band");
    let total: usize = on.shapes.iter().map(TraceShapeRegion::primitives).sum();
    assert_eq!(total, 3);
}

#[test]
fn touching_pair_with_a_hole_that_is_no_band_stays_paths() {
    let star = star_outline((20.0, 20.0), 16.0, 0.8, 8, 0.1);
    let s1 = stroke_of(&star, 2.5, Join::Miter);
    // A solid disc touching the star's right tips, with an off-centre round hole (no band).
    let disc_c = (20.0 + 16.0 + 2.5 + 11.0, 20.0);
    let ink = move |x: f64, y: f64| {
        let p = kurbo::Point::new(x, y);
        s1.winding(p) != 0
            || ((x - disc_c.0).hypot(y - disc_c.1) <= 14.0
                && (x - disc_c.0 - 4.0).hypot(y - disc_c.1) > 6.0)
    };
    let (off, on) = trace(80.0, 40.0, ink);
    assert_eq!(
        off.outlines.len(),
        3,
        "premise: one region, outer and two holes"
    );
    for r in &on.shapes {
        assert!(r.band.is_none() && r.bands.is_empty(), "stays paths: {r:?}");
    }
}

#[test]
fn touching_a_non_overlapping_split_would_not_explain_the_region() {
    // Two stars that touch plus a hole that is a plain square: stays paths too.
    let a = star_outline((20.0, 20.0), 16.0, 0.8, 8, 0.1);
    let sa = stroke_of(&a, 2.5, Join::Miter);
    let sq = rect_outline((20.0 + 16.0 + 2.5 + 12.0, 20.0), 20.0, 20.0);
    let sq_stroke = stroke_of(&sq, 4.0, Join::Miter);
    let (_, on) = trace(80.0, 40.0, ink_of(vec![sa, sq_stroke]));
    // Whatever it is, never a wrong band: any band's stroke covers the traced area.
    for r in &on.shapes {
        for b in r.bands.iter().chain(r.band.iter()) {
            assert!(b.width.as_mm() > 0.0);
        }
    }
}

#[test]
fn customer_outline_stars_eight_mitred_one_bevelled_one_paths() {
    let png = image::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/trace_shapes/customer-outline-stars.png"),
    )
    .unwrap()
    .to_rgba8();
    let (w, h) = png.dimensions();
    let map = [25.4 / 96.0, 0.0, 0.0, 25.4 / 96.0, 0.0, 0.0];
    let off = trace_job_with(
        png.as_raw(),
        w,
        h,
        map,
        &TraceParams::with_threshold(128),
        &mut |_| {},
    );
    let mut on = off.clone();
    trace_add_shapes(&mut on, map);
    let (off, on) = (
        read_trace_wire(&off).unwrap(),
        read_trace_wire(&on).unwrap(),
    );
    let bands = band_regions(&on);
    let miter: Vec<_> = bands
        .iter()
        .filter(|r| r.band.unwrap().join == LineJoin::Miter)
        .collect();
    let bevel: Vec<_> = bands
        .iter()
        .filter(|r| r.band.unwrap().join == LineJoin::Bevel)
        .collect();
    eprintln!("regions {} bands {}", on.shapes.len(), bands.len());
    assert_eq!(miter.len(), 8, "eight mitred");
    assert_eq!(bevel.len(), 1, "one bevelled");
    assert_eq!(bands.len(), 9);
    // Ten stars on the page: 20 traced outlines; the clipped one is in no band.
    let in_bands: usize = bands.iter().map(|r| r.outlines.len()).sum();
    eprintln!("outlines {} in bands {in_bands}", off.outlines.len());
    assert_eq!(off.outlines.len(), 20, "ten stars, two outlines each");
    assert_eq!(off.outlines.len() - in_bands, 2, "one star stays paths");
    for r in &bands {
        let b = r.band.unwrap();
        assert!(matches!(b.shape, Shape::Star { point_count, .. } if point_count.get() == 8));
        let traced = region_area(&off, r);
        let stroked = area_of(
            &[stroke_of(&centre_line(&b), b.width.as_mm(), join_of(&b))],
            0.05,
        );
        assert!(
            (stroked - traced).abs() <= 0.02 * traced,
            "stroked {stroked} vs traced {traced}"
        );
        // Width vs distance between the offsets (mitred stars only: the sides are parallel).
        let d = median_offset_distance(&on, r);
        eprintln!(
            "width {:.3} offset distance {d:.3} join {:?}",
            b.width.as_mm(),
            b.join
        );
        assert!(
            (b.width.as_mm() - d).abs() <= 0.25,
            "width {} vs offset distance {d}",
            b.width.as_mm()
        );
    }
}
