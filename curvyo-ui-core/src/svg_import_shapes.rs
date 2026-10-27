//! The geometry of one imported leaf (`specs/0024-svg-import-export` criteria 24 to 29 and 33):
//! path data into outlines with node kinds, and a rectangle, ellipse, Inkscape star or arc kept as a
//! primitive where its transform allows (the rules of `0056` criteria 18 to 22), else its exact
//! outline as a path.

use curvyo_document_core::{
    AnchorKind, Angle, CornerRadii, Curve, DrawingAnchor, DrawingOutline, EllipseFrame, InnerRatio,
    Length, Point, PointCount, RectBounds, Shape, StarFrame, Vec2, outline_of,
};
use curvyo_svg_core::{LeafKind, StarData, rounded_rect};

use crate::svg_import_arcs::arc_primitive;
use crate::svg_import_match::draws;
use kurbo::{Affine, BezPath, PathEl};

/// The relative tolerance of "collinear" and "equal length" for node kinds (criterion 25).
const KIND_TOLERANCE: f64 = 1e-6;

/// How far a matrix may be from orthogonal or uniform and still keep a primitive.
const MATRIX_TOLERANCE: f64 = 1e-9;

/// How far `arg2` may be from `arg1 + π/N` for a star that is not twisted (criterion 117).
const TWIST_TOLERANCE_RAD: f64 = 1e-6;

/// The geometry of a leaf in document millimetres.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LeafGeometry {
    /// A path or compound path.
    Path(Vec<DrawingOutline>),
    /// A primitive with its rotation.
    Primitive(Shape, Angle),
}

/// The geometry of `kind` after `placement` (the translation of Import, else the identity).
/// `None` for a clone, which has none of its own.
pub(crate) fn leaf_geometry(kind: &LeafKind, placement: Affine) -> Option<LeafGeometry> {
    Some(match kind {
        LeafKind::Path { path, node_types } => {
            LeafGeometry::Path(outlines(&(placement * path.clone()), node_types.as_deref()))
        }
        LeafKind::Rect {
            rect,
            rx,
            ry,
            matrix,
        } => rect_geometry(*rect, *rx, *ry, placement * *matrix),
        LeafKind::Ellipse {
            center,
            rx,
            ry,
            matrix,
        } => ellipse_geometry(*center, *rx, *ry, placement * *matrix),
        LeafKind::Star {
            star,
            path,
            node_types,
            matrix,
        } => {
            let drawn = placement * path.clone();
            star_geometry(star, &drawn, placement * *matrix)
                .unwrap_or_else(|| LeafGeometry::Path(outlines(&drawn, node_types.as_deref())))
        }
        LeafKind::Arc {
            arc,
            path,
            fine,
            node_types,
            matrix,
        } => arc_primitive(arc, placement * *matrix)
            .filter(|(shape, rotation)| {
                fine.as_ref()
                    .is_some_and(|fine| draws(shape, *rotation, &(placement * fine.clone())))
            })
            .map_or_else(
                || LeafGeometry::Path(outlines(&(placement * path.clone()), node_types.as_deref())),
                |(shape, rotation)| LeafGeometry::Primitive(shape, rotation),
            ),
        LeafKind::Clone { .. } => return None,
    })
}

/// How a matrix acts on a primitive.
pub(crate) enum Fit {
    /// A rotation, uniform scale, mirror and translation: the scale and the rotation angle.
    Similar(f64, f64),
    /// Axes kept parallel to the page: the scale along the primitive's x and y, and whether the
    /// axes swap (a quarter turn).
    Aligned(f64, f64, bool),
    /// A rotated stretch or a skew: only a path.
    Other,
}

#[allow(clippy::many_single_char_names)] // the SVG names of the matrix entries
pub(crate) fn fit(m: Affine) -> Fit {
    let [a, b, c, d, _, _] = m.as_coeffs();
    let (sx, sy) = (a.hypot(b), c.hypot(d));
    let scale = sx.max(sy);
    let orthogonal = a.mul_add(c, b * d).abs() <= MATRIX_TOLERANCE * scale * scale;
    if orthogonal && (sx - sy).abs() <= MATRIX_TOLERANCE * scale {
        return Fit::Similar(sx, b.atan2(a));
    }
    if b.abs() <= MATRIX_TOLERANCE * scale && c.abs() <= MATRIX_TOLERANCE * scale {
        return Fit::Aligned(a.abs(), d.abs(), false);
    }
    if a.abs() <= MATRIX_TOLERANCE * scale && d.abs() <= MATRIX_TOLERANCE * scale {
        return Fit::Aligned(b.abs(), c.abs(), true);
    }
    Fit::Other
}

/// A rectangle: kept under a similarity, or an axis-aligned stretch whose corners stay circular;
/// otherwise its exact outline (criteria 26 and 33).
fn rect_geometry(rect: kurbo::Rect, rx: f64, ry: f64, m: Affine) -> LeafGeometry {
    let center = m * rect.center();
    let primitive = |w: f64, h: f64, r: f64, turn: f64| {
        let bounds = RectBounds {
            origin: Point::new(center.x - w / 2.0, center.y - h / 2.0),
            width: Length::from_mm(w),
            height: Length::from_mm(h),
        };
        LeafGeometry::Primitive(
            Shape::Rect {
                bounds,
                corner_radii: CornerRadii::uniform(Length::from_mm(r)),
            },
            Angle::from_radians(turn),
        )
    };
    let circular = |a: f64, b: f64| (a - b).abs() <= MATRIX_TOLERANCE * a.max(b).max(1.0);
    match fit(m) {
        Fit::Similar(s, turn) if circular(rx, ry) => {
            primitive(rect.width() * s, rect.height() * s, rx * s, turn)
        }
        Fit::Aligned(sx, sy, swap) if circular(rx * sx, ry * sy) => {
            let (w, h) = (rect.width() * sx, rect.height() * sy);
            let (w, h) = if swap { (h, w) } else { (w, h) };
            primitive(w, h, rx * sx, 0.0)
        }
        _ => LeafGeometry::Path(outlines(&(m * rounded_rect(rect, rx, ry)), None)),
    }
}

/// An ellipse: kept under a similarity or an axis-aligned stretch, otherwise its exact outline.
fn ellipse_geometry(center: kurbo::Point, rx: f64, ry: f64, m: Affine) -> LeafGeometry {
    let c = m * center;
    let primitive = |rx: f64, ry: f64, turn: f64| {
        LeafGeometry::Primitive(
            Shape::Ellipse {
                frame: EllipseFrame {
                    center: Point::new(c.x, c.y),
                    rx: Length::from_mm(rx),
                    ry: Length::from_mm(ry),
                },
                arc: None,
                curve: Curve::ELLIPSE_DEFAULT,
            },
            Angle::from_radians(turn),
        )
    };
    match fit(m) {
        Fit::Similar(s, turn) => primitive(rx * s, ry * s, turn),
        Fit::Aligned(sx, sy, false) => primitive(rx * sx, ry * sy, 0.0),
        Fit::Aligned(sx, sy, true) => primitive(ry * sy, rx * sx, 0.0),
        Fit::Other => {
            let local = Shape::Ellipse {
                frame: EllipseFrame {
                    center: Point::new(center.x, center.y),
                    rx: Length::from_mm(rx),
                    ry: Length::from_mm(ry),
                },
                arc: None,
                curve: Curve::ELLIPSE_DEFAULT,
            };
            LeafGeometry::Path(vec![mapped_outline(&local, m)])
        }
    }
}

/// An Inkscape star or polygon as the primitive, when the matrix keeps it and its outline is
/// the drawn path within 0.001 mm (criteria 111, 116 to 118 and 33); `None` makes it a path.
fn star_geometry(star: &StarData, drawn: &BezPath, m: Affine) -> Option<LeafGeometry> {
    let Fit::Similar(scale, _) = fit(m) else {
        return None;
    };
    if star.r1 <= 0.0 {
        return None;
    }
    let count = PointCount::new(star.sides).ok()?;
    let vertex = star.center + kurbo::Vec2::from_angle(star.arg1) * star.r1;
    let (c, v) = (m * star.center, m * vertex);
    let frame = StarFrame {
        center: Point::new(c.x, c.y),
        radius: Length::from_mm(star.r1 * scale),
        angle: Angle::from_radians((v.y - c.y).atan2(v.x - c.x)),
    };
    let shape = if star.flat {
        Shape::Polygon {
            frame,
            point_count: count,
            curve: Curve::POLYGON_DEFAULT,
        }
    } else {
        let half_step = std::f64::consts::PI / f64::from(star.sides);
        let twist = (star.arg2? - star.arg1 - half_step + std::f64::consts::PI)
            .rem_euclid(std::f64::consts::TAU)
            - std::f64::consts::PI;
        if twist.abs() > TWIST_TOLERANCE_RAD {
            return None;
        }
        Shape::Star {
            frame,
            point_count: count,
            inner_ratio: InnerRatio::new(star.r2 / star.r1).ok()?,
            curve: Curve::POLYGON_DEFAULT,
        }
    };
    let rotation = Angle::from_radians(0.0);
    draws(&shape, rotation, drawn).then_some(LeafGeometry::Primitive(shape, rotation))
}

/// The outline of a primitive in its local units mapped by `m` (an affine map takes cubics to
/// cubics exactly).
fn mapped_outline(local: &Shape, m: Affine) -> DrawingOutline {
    let outline = outline_of(local);
    let point = |p: Point| {
        let q = m * kurbo::Point::new(p.x, p.y);
        Point::new(q.x, q.y)
    };
    let vector = |v: Vec2| {
        let q = m * kurbo::Point::new(v.x, v.y) - m * kurbo::Point::ZERO;
        Vec2::new(q.x, q.y)
    };
    let anchors = outline
        .anchors
        .iter()
        .map(|a| DrawingAnchor {
            point: point(a.point),
            handle_in: vector(a.handle_in),
            handle_out: vector(a.handle_out),
            kind: a.kind,
        })
        .collect();
    DrawingOutline {
        closed: outline.closed,
        anchors,
    }
}

/// The outlines of `path` (millimetres) as written: one per subpath with two or more nodes, `Z`
/// closing; nodes never merged (criterion 24). Node kinds from `node_types` when it has one
/// letter per node, else from the handles (criterion 25).
pub(crate) fn outlines(path: &BezPath, node_types: Option<&str>) -> Vec<DrawingOutline> {
    let mut out: Vec<DrawingOutline> = Vec::new();
    let mut current: Vec<DrawingAnchor> = Vec::new();
    let finish = |anchors: &mut Vec<DrawingAnchor>, closed: bool, out: &mut Vec<DrawingOutline>| {
        if anchors.len() >= 2 {
            out.push(DrawingOutline {
                closed,
                anchors: std::mem::take(anchors),
            });
        } else {
            anchors.clear();
        }
    };
    let corner = |p: kurbo::Point| DrawingAnchor {
        point: Point::new(p.x, p.y),
        handle_in: Vec2::ZERO,
        handle_out: Vec2::ZERO,
        kind: AnchorKind::Corner,
    };
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                finish(&mut current, false, &mut out);
                current.push(corner(p));
            }
            PathEl::LineTo(p) => current.push(corner(p)),
            PathEl::QuadTo(c, p) => {
                let from = current
                    .last()
                    .map_or(p, |a| kurbo::Point::new(a.point.x, a.point.y));
                let c1 = from + (c - from) * (2.0 / 3.0);
                let c2 = p + (c - p) * (2.0 / 3.0);
                push_curve(&mut current, c1, c2, p);
            }
            PathEl::CurveTo(c1, c2, p) => push_curve(&mut current, c1, c2, p),
            // The reader writes the move that starts a subpath after `Z` explicitly.
            PathEl::ClosePath => finish(&mut current, true, &mut out),
        }
    }
    finish(&mut current, false, &mut out);
    set_kinds(&mut out, node_types);
    out
}

fn push_curve(
    current: &mut Vec<DrawingAnchor>,
    c1: kurbo::Point,
    c2: kurbo::Point,
    p: kurbo::Point,
) {
    if let Some(last) = current.last_mut() {
        last.handle_out = Vec2::new(c1.x - last.point.x, c1.y - last.point.y);
    }
    current.push(DrawingAnchor {
        point: Point::new(p.x, p.y),
        handle_in: Vec2::new(c2.x - p.x, c2.y - p.y),
        handle_out: Vec2::ZERO,
        kind: AnchorKind::Corner,
    });
}

/// Criterion 25: Inkscape's letters when there is one per node (or one per node plus one per
/// closed outline, Inkscape's closing letter), else the handles decide.
fn set_kinds(outlines: &mut [DrawingOutline], node_types: Option<&str>) {
    let nodes: usize = outlines.iter().map(|o| o.anchors.len()).sum();
    let closed = outlines.iter().filter(|o| o.closed).count();
    let letters: Vec<char> = node_types.unwrap_or_default().trim().chars().collect();
    let known = letters.iter().all(|c| matches!(c, 'c' | 'z' | 's' | 'a'));
    let with_close = letters.len() == nodes + closed && closed > 0;
    if known && (letters.len() == nodes || with_close) {
        let mut at = 0;
        for outline in outlines.iter_mut() {
            for anchor in &mut outline.anchors {
                anchor.kind = match letters[at] {
                    'z' => AnchorKind::Symmetric,
                    's' | 'a' => AnchorKind::Asymmetric,
                    _ => AnchorKind::Corner,
                };
                at += 1;
            }
            if with_close && outline.closed {
                at += 1;
            }
        }
        return;
    }
    for anchor in outlines.iter_mut().flat_map(|o| o.anchors.iter_mut()) {
        anchor.kind = kind_from_handles(anchor.handle_in, anchor.handle_out);
    }
}

/// Collinear and equal handles are Symmetric, collinear and unequal Asymmetric, else Corner.
fn kind_from_handles(handle_in: Vec2, handle_out: Vec2) -> AnchorKind {
    let (a, b) = (handle_in.length(), handle_out.length());
    if a == 0.0 || b == 0.0 {
        return AnchorKind::Corner;
    }
    let cross = handle_in
        .x
        .mul_add(handle_out.y, -(handle_in.y * handle_out.x));
    let dot = handle_in
        .x
        .mul_add(handle_out.x, handle_in.y * handle_out.y);
    let collinear = cross.abs() <= KIND_TOLERANCE * a * b && dot < 0.0;
    if !collinear {
        AnchorKind::Corner
    } else if (a - b).abs() <= KIND_TOLERANCE * a.max(b) {
        AnchorKind::Symmetric
    } else {
        AnchorKind::Asymmetric
    }
}
