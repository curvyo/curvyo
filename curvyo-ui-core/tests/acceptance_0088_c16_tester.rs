//! Criterion 16 of `specs/0088-trace-recognition` Part B, black-box: the customer's page, whose
//! 8-point outline star becomes one stroked star whose stroked area matches the traced band's.

#![allow(
    clippy::unwrap_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use curvyo_document_core::Shape;
use curvyo_ui_core::{TraceParams, read_trace_wire, trace_add_shapes, trace_job_in};
use kurbo::{BezPath, Join, Shape as _, Stroke, StrokeOpts};

const MM_PER_PX: f64 = 25.4 / 96.0;

/// The star polygon of a centre line: `n` tips of `radius` from `angle`, valleys at `ratio`.
fn star_outline(
    centre: (f64, f64),
    radius: f64,
    angle: f64,
    ratio: f64,
    n: u32,
    rotation: f64,
) -> BezPath {
    let mut path = BezPath::new();
    for k in 0..2 * n {
        let a = angle + rotation + f64::from(k) * std::f64::consts::PI / f64::from(n);
        let r = if k % 2 == 0 { radius } else { radius * ratio };
        let p = (centre.0 + r * a.cos(), centre.1 + r * a.sin());
        if k == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    path.close_path();
    path
}

/// Area of the union covered by `path` stroked mitred (limit 4), by a winding-number grid.
fn stroked_area(path: &BezPath, width: f64, step: f64) -> f64 {
    let style = Stroke::new(width)
        .with_join(Join::Miter)
        .with_miter_limit(4.0);
    let outline = kurbo::stroke(path.iter(), &style, &StrokeOpts::default(), 0.01);
    let bb = outline.bounding_box();
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
            if outline.winding(p) != 0 {
                hits += 1;
            }
        }
    }
    hits as f64 * step * step
}

#[test]
fn c16_the_customers_8_star_is_a_stroked_star_with_the_traced_area() {
    let png = image::open(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/trace_shapes/customer-page.png"),
    )
    .unwrap()
    .to_rgba8();
    let (w, h) = png.dimensions();
    let map = [MM_PER_PX, 0.0, 0.0, MM_PER_PX, 0.0, 0.0];
    let off = trace_job_in(
        png.as_raw(),
        w,
        h,
        map,
        &TraceParams::with_threshold(128),
        None,
        &mut |_| {},
    );
    let mut on = off.clone();
    trace_add_shapes(&mut on, map);
    let (off, on) = (
        read_trace_wire(&off).unwrap(),
        read_trace_wire(&on).unwrap(),
    );
    let bands: Vec<_> = on.shapes.iter().filter(|r| r.band.is_some()).collect();
    assert_eq!(bands.len(), 1, "exactly one stroked band on the page");
    let region = bands[0];
    let band = region.band.unwrap();
    let Shape::Star {
        frame,
        point_count,
        inner_ratio,
        ..
    } = band.shape
    else {
        panic!("not a star: {band:?}");
    };
    assert_eq!(point_count.get(), 8);
    assert_eq!(region.primitives(), 1, "one object for the band");
    assert!(band.width.as_mm() > 0.0);
    let traced: f64 = region
        .outlines
        .iter()
        .map(|&i| kurbo::Shape::area(&off.outlines[i]))
        .sum::<f64>()
        .abs();
    let centre_line = star_outline(
        (frame.center.x, frame.center.y),
        frame.radius.as_mm(),
        frame.angle.as_radians(),
        inner_ratio.get(),
        8,
        band.rotation.as_radians(),
    );
    let stroked = stroked_area(&centre_line, band.width.as_mm(), 0.05);
    eprintln!(
        "traced {traced:.1} mm2, stroked {stroked:.1} mm2, width {}",
        band.width.as_mm()
    );
    assert!(
        (stroked - traced).abs() <= 0.02 * traced,
        "stroked {stroked} vs traced {traced}"
    );
    // The other regions of the page are not bands.
    assert_eq!(
        on.shapes.iter().filter(|r| r.band.is_none()).count(),
        on.shapes.len() - 1
    );
}
