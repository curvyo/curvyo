//! The Rounded value on an ellipse (`specs/0145-shape-spiro-handles` slice 2, criteria 14 to
//! 16 and 19, `adrs.md` decisions 4 and 9): `q_c` draws today's ellipse, the outline loops
//! past the centre, the arc wins, the value is stored raw and survives save and open, and Curve
//! and Rounded exclude each other in one commit.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::many_single_char_names
)]

use curvyo_document_core::{
    Curve, Document, EllipseArc, EllipseFrame, Length, ObjectSnapshot, Point, Q_C, Rounded, Shape,
    ShapeEditError, drawing_outline_of, ellipse_outline, outline_of, pack, shape_frame_bounds,
    unpack,
};

const GOLDEN: &str = "rounded_ellipse_v57.curvyo";
const VALUES: [f64; 6] = [-3.0, 0.39, Q_C, 1.0, 2.5, 12.4];

fn frame(x: f64) -> EllipseFrame {
    EllipseFrame {
        center: Point::new(x, 5.0),
        rx: Length::from_mm(30.0),
        ry: Length::from_mm(12.0),
    }
}

fn fixture_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(GOLDEN)
}

fn golden_document() -> Document {
    let d = Document::new(1);
    for (i, q) in VALUES.iter().enumerate() {
        let e = d.create_ellipse(frame(50.0 * f64::from(u32::try_from(i).unwrap())));
        d.set_rounded(&[(e, Rounded::new(*q).unwrap())]).unwrap();
    }
    d
}

fn rounded_values(d: &Document) -> Vec<f64> {
    d.read_objects()
        .objects
        .iter()
        .filter_map(|o| match o {
            ObjectSnapshot::Primitive(p) => match p.shape {
                Shape::Ellipse { rounded, .. } => Some(rounded.get()),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn ellipse_of(d: &Document, id: curvyo_document_core::NodeId) -> Shape {
    d.primitive(id).unwrap().shape
}

/// Criterion 15: at `q_c` the outline is today's ellipse, node by node (the handles are KAPPA).
#[test]
fn q_c_draws_exactly_todays_ellipse() {
    let d = Document::new(1);
    let e = d.create_ellipse(frame(0.0));
    let plain = ellipse_outline(frame(0.0));
    d.set_rounded(&[(e, Rounded::new(Q_C).unwrap())]).unwrap();
    let rounded = outline_of(&ellipse_of(&d, e)).anchors;
    assert_eq!(rounded.len(), plain.len());
    for (a, b) in rounded.iter().zip(&plain) {
        assert!(a.point.vector_to(b.point).length() < 1e-9);
        assert!((a.handle_out - b.handle_out).length() < 1e-9);
        assert!((a.handle_in - b.handle_in).length() < 1e-9);
    }
    assert!((Q_C - 0.390_524).abs() < 1e-6);
}

/// Criterion 15: a typed 0.39 stays within 0.05 % of `rx` and `ry` of the ellipse.
#[test]
fn a_typed_0_39_stays_within_a_twentieth_of_a_percent() {
    let d = Document::new(1);
    let e = d.create_ellipse(frame(0.0));
    d.set_rounded(&[(e, Rounded::new(0.39).unwrap())]).unwrap();
    let outline = drawing_outline_of(&ellipse_of(&d, e)).anchors;
    // Midpoint of the first cubic (E to S) at t = 1/2 against the ellipse at 45 degrees.
    let (a, b) = (outline[0], outline[1]);
    let p0 = a.point;
    let p1 = a.point.translated(a.handle_out);
    let p2 = b.point.translated(b.handle_in);
    let p3 = b.point;
    let mid = |f: fn(Point) -> f64| (f(p0) + 3.0 * f(p1) + 3.0 * f(p2) + f(p3)) / 8.0;
    let (x, y) = (mid(|p| p.x), mid(|p| p.y));
    let radial = ((x / 30.0).powi(2) + ((y - 5.0) / 12.0).powi(2)).sqrt();
    assert!((radial - 1.0).abs() < 5e-4, "radial {radial}");
}

/// Criterion 16: over-range the ellipse stays a live ellipse and the box grows to the exact
/// Bezier box; at `q_c` and in between it is exactly today's box.
#[test]
fn over_range_keeps_the_ellipse_and_grows_the_box() {
    let d = Document::new(1);
    let e = d.create_ellipse(frame(0.0));
    d.set_rounded(&[(e, Rounded::new(Q_C).unwrap())]).unwrap();
    let (min, max) = shape_frame_bounds(&ellipse_of(&d, e));
    assert!((min.x + 30.0).abs() < 1e-9 && (max.y - 17.0).abs() < 1e-9);
    d.set_rounded(&[(e, Rounded::new(2.5).unwrap())]).unwrap();
    let shape = ellipse_of(&d, e);
    assert!(matches!(shape, Shape::Ellipse { .. }));
    let (min, max) = shape_frame_bounds(&shape);
    assert!(min.x < -30.0 - 1.0 && max.x > 30.0 + 1.0);
    // 4096 sampled points of the outline lie inside the box.
    let outline = outline_of(&shape).anchors;
    for i in 0..outline.len() {
        let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
        for k in 0..=1024 {
            let t = f64::from(k) / 1024.0;
            let u = 1.0 - t;
            let pt = |f: fn(Point) -> f64| {
                u.powi(3) * f(a.point)
                    + 3.0 * u * u * t * f(a.point.translated(a.handle_out))
                    + 3.0 * u * t * t * f(b.point.translated(b.handle_in))
                    + t.powi(3) * f(b.point)
            };
            let (x, y) = (pt(|p| p.x), pt(|p| p.y));
            assert!(x >= min.x - 1e-6 && x <= max.x + 1e-6);
            assert!(y >= min.y - 1e-6 && y <= max.y + 1e-6);
        }
    }
}

/// Criterion 19: an ellipse with an arc ignores a stored value and refuses a new one; the
/// whole ellipse gives the value back.
#[test]
fn the_arc_wins_over_rounded() {
    let d = Document::new(1);
    let e = d.create_ellipse(frame(0.0));
    d.set_rounded(&[(e, Rounded::new(2.0).unwrap())]).unwrap();
    d.set_curves(&[(e, Curve::ELLIPSE_DEFAULT)]).unwrap();
    d.set_ellipse_arcs(&[(e, Some(arc()))]).unwrap();
    // The stored value is ignored while the arc is there: the outline is the arc's.
    let with_arc = ellipse_of(&d, e);
    let Shape::Ellipse { rounded, .. } = with_arc else {
        panic!("an ellipse")
    };
    assert_eq!(rounded.get(), 0.0, "set_curves cleared it");
    assert_eq!(
        d.set_rounded(&[(e, Rounded::new(1.0).unwrap())]),
        Err(ShapeEditError::ArcWithCurve)
    );
    assert!(d.set_rounded(&[(e, Rounded::ZERO)]).is_ok());
    assert_eq!(
        outline_of(&with_arc).closed,
        outline_of(&ellipse_of(&d, e)).closed
    );
}

fn arc() -> EllipseArc {
    EllipseArc::new(
        curvyo_document_core::Angle::from_radians(0.2),
        curvyo_document_core::Angle::from_radians(3.5),
        curvyo_document_core::ArcKind::Pie,
    )
    .unwrap()
}

/// Criterion 14: Rounded and Curve exclude each other in one commit; one undo restores both.
#[test]
fn rounded_and_curve_exclude_each_other_in_one_commit() {
    let d = Document::new(1);
    let e = d.create_ellipse(frame(0.0));
    d.set_curves(&[(e, Curve::new(0.5).unwrap())]).unwrap();
    d.set_rounded(&[(e, Rounded::new(1.5).unwrap())]).unwrap();
    let Shape::Ellipse { curve, rounded, .. } = ellipse_of(&d, e) else {
        panic!("an ellipse")
    };
    assert_eq!((curve.get(), rounded.get()), (0.0, 1.5));
    d.set_curves(&[(e, Curve::new(0.25).unwrap())]).unwrap();
    let Shape::Ellipse { curve, rounded, .. } = ellipse_of(&d, e) else {
        panic!("an ellipse")
    };
    assert_eq!((curve.get(), rounded), (0.25, Rounded::ZERO));
    let step = d.own_steps().last().unwrap().step;
    d.undo_step(step).unwrap();
    let Shape::Ellipse { curve, rounded, .. } = ellipse_of(&d, e) else {
        panic!("an ellipse")
    };
    assert_eq!((curve.get(), rounded.get()), (0.0, 1.5));
}

/// Criterion 16: save and open keep every value exactly (also 12.4, beyond the limit), the
/// golden file reads back raw, zero stores nothing, a build before 57 refuses the file.
#[test]
fn the_value_survives_save_open_and_the_golden_reads_back() {
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::write(fixture_path(), pack(&golden_document(), "0.1.0").unwrap()).unwrap();
    }
    let d = golden_document();
    let reopened = unpack(2, &pack(&d, "t").unwrap()).unwrap();
    assert_eq!(rounded_values(&reopened), VALUES);
    let bytes = std::fs::read(fixture_path()).unwrap();
    assert_eq!(rounded_values(&unpack(2, &bytes).unwrap()), VALUES);
    // Evaluation limits 12.4 to 10 without writing it back.
    let (min, max) = shape_frame_bounds(&ellipse_of(&reopened, last_ellipse(&reopened)));
    assert!(min.x.is_finite() && max.x.is_finite());
}

fn last_ellipse(d: &Document) -> curvyo_document_core::NodeId {
    d.read_objects()
        .objects
        .iter()
        .rev()
        .find_map(|o| match o {
            ObjectSnapshot::Primitive(p) if matches!(p.shape, Shape::Ellipse { .. }) => Some(p.id),
            _ => None,
        })
        .unwrap()
}
