//! The geometry of one drawn element (`specs/0024-svg-import-export` criteria 21 to 30): path
//! data, the basic shapes and Inkscape's star data, with every coordinate checked against the
//! limits after the transforms.

use kurbo::{Affine, BezPath, PathEl, Point, Rect};
use roxmltree::Node;

use crate::drawing::{LeafKind, StarData};
use crate::limits::{MAX_ATTRIBUTE_BYTES, MAX_COORDINATE_MM};
use crate::page::{INKSCAPE_NS, SODIPODI_NS};
use crate::path_data::{ArcSplit, PathDataError, PathLimits, parse_path_data};
use crate::units::{Axis, Units};

/// Why an element gave no geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShapeError {
    /// The element is skipped (no or bad geometry).
    Skip,
    /// The point budget is used up: the reading stops.
    Budget,
}

/// An attribute's value unless it is longer than [`MAX_ATTRIBUTE_BYTES`].
pub(crate) fn attr<'a>(node: Node<'a, '_>, name: &str) -> Option<&'a str> {
    node.attribute(name)
        .filter(|value| value.len() <= MAX_ATTRIBUTE_BYTES)
}

/// How many points `path` holds (one per move or line, three per cubic).
pub(crate) fn point_count(path: &BezPath) -> usize {
    path.elements()
        .iter()
        .map(|e| match e {
            PathEl::CurveTo(..) => 3,
            PathEl::QuadTo(..) => 2,
            PathEl::ClosePath => 0,
            _ => 1,
        })
        .sum()
}

/// Whether every point of `path` is finite and within [`MAX_COORDINATE_MM`].
pub(crate) fn within_limits(path: &BezPath) -> bool {
    let ok = |p: &Point| {
        p.x.is_finite()
            && p.y.is_finite()
            && p.x.abs() <= MAX_COORDINATE_MM
            && p.y.abs() <= MAX_COORDINATE_MM
    };
    path.elements().iter().all(|e| match e {
        PathEl::MoveTo(p) | PathEl::LineTo(p) => ok(p),
        PathEl::QuadTo(a, b) => ok(a) && ok(b),
        PathEl::CurveTo(a, b, c) => ok(a) && ok(b) && ok(c),
        PathEl::ClosePath => true,
    })
}

/// Whether `path` draws anything: some subpath has a segment.
fn has_segment(path: &BezPath) -> bool {
    path.elements().iter().any(|e| {
        matches!(
            e,
            PathEl::LineTo(_) | PathEl::CurveTo(..) | PathEl::QuadTo(..)
        )
    })
}

/// A path in user units mapped by `matrix` and checked; `points` is the budget left and is
/// charged.
fn mapped(path: &BezPath, matrix: Affine, points: &mut usize) -> Result<BezPath, ShapeError> {
    let count = point_count(path);
    if count > *points {
        return Err(ShapeError::Budget);
    }
    *points -= count;
    let path = matrix * path.clone();
    if !has_segment(&path) || !within_limits(&path) {
        return Err(ShapeError::Skip);
    }
    Ok(path)
}

/// The geometry of a drawn element `node` under `matrix` (user units to millimetres).
pub(crate) fn leaf_kind(
    node: Node<'_, '_>,
    matrix: Affine,
    units: &Units,
    points: &mut usize,
) -> Result<LeafKind, ShapeError> {
    let length = |name: &str, axis: Axis| attr(node, name).and_then(|t| units.parse(t, axis));
    let node_types = || {
        node.attribute((SODIPODI_NS, "nodetypes"))
            .map(str::to_owned)
    };
    let path = match node.tag_name().name() {
        "path" => path_data(node, *points)?,
        "rect" => return rect_leaf(&length, matrix, points),
        "circle" | "ellipse" => {
            let circle = node.tag_name().name() == "circle";
            return ellipse_leaf(&length, circle, matrix, points);
        }
        "line" => {
            let mut path = BezPath::new();
            path.move_to((
                length("x1", Axis::X).unwrap_or(0.0),
                length("y1", Axis::Y).unwrap_or(0.0),
            ));
            path.line_to((
                length("x2", Axis::X).unwrap_or(0.0),
                length("y2", Axis::Y).unwrap_or(0.0),
            ));
            path
        }
        "polyline" | "polygon" => {
            let numbers = number_list(attr(node, "points").unwrap_or_default());
            let mut path = BezPath::new();
            for (i, [x, y]) in numbers.as_chunks::<2>().0.iter().enumerate() {
                if i == 0 {
                    path.move_to((*x, *y));
                } else {
                    path.line_to((*x, *y));
                }
            }
            if node.tag_name().name() == "polygon" && has_segment(&path) {
                path.close_path();
            }
            path
        }
        _ => return Err(ShapeError::Skip),
    };
    let path = mapped(&path, matrix, points)?;
    if let Some(star) = star_data(node) {
        return Ok(LeafKind::Star {
            star,
            path,
            node_types: node_types(),
            matrix,
        });
    }
    Ok(LeafKind::Path {
        path,
        node_types: node_types(),
    })
}

/// The `d` of a `<path>` in user units, within the point budget.
fn path_data(node: Node<'_, '_>, points: usize) -> Result<BezPath, ShapeError> {
    let d = attr(node, "d").ok_or(ShapeError::Skip)?;
    let limits = PathLimits {
        max_coordinate: f64::MAX,
        max_points: points,
        arcs: ArcSplit::EighthTurns,
    };
    match parse_path_data(d, limits) {
        Ok(path) => Ok(path),
        Err(PathDataError::TooManyPoints) => Err(ShapeError::Budget),
        Err(PathDataError::BadNumber) => Err(ShapeError::Skip),
    }
}

/// A `<rect>`: a missing radius takes the other's value, both are clamped to half the sides; no
/// or a non-positive width or height skips it (criterion 26).
fn rect_leaf(
    length: &dyn Fn(&str, Axis) -> Option<f64>,
    matrix: Affine,
    points: &mut usize,
) -> Result<LeafKind, ShapeError> {
    let left = length("x", Axis::X).unwrap_or(0.0);
    let top = length("y", Axis::Y).unwrap_or(0.0);
    let width = length("width", Axis::X)
        .filter(|w| *w > 0.0)
        .ok_or(ShapeError::Skip)?;
    let height = length("height", Axis::Y)
        .filter(|h| *h > 0.0)
        .ok_or(ShapeError::Skip)?;
    let radius_x = length("rx", Axis::X).filter(|r| *r >= 0.0);
    let radius_y = length("ry", Axis::Y).filter(|r| *r >= 0.0);
    let (rx, ry) = match (radius_x, radius_y) {
        (Some(rx), Some(ry)) => (rx, ry),
        (Some(r), None) | (None, Some(r)) => (r, r),
        (None, None) => (0.0, 0.0),
    };
    let rect = Rect::new(left, top, left + width, top + height);
    check_corners(rect, matrix)?;
    charge(points, 4)?;
    Ok(LeafKind::Rect {
        rect,
        rx: rx.min(width / 2.0),
        ry: ry.min(height / 2.0),
        matrix,
    })
}

/// A `<circle>` or `<ellipse>`; a radius of 0 or below skips it (criterion 27).
fn ellipse_leaf(
    length: &dyn Fn(&str, Axis) -> Option<f64>,
    circle: bool,
    matrix: Affine,
    points: &mut usize,
) -> Result<LeafKind, ShapeError> {
    let center = Point::new(
        length("cx", Axis::X).unwrap_or(0.0),
        length("cy", Axis::Y).unwrap_or(0.0),
    );
    let (radius_x, radius_y) = if circle {
        let r = length("r", Axis::Diagonal);
        (r, r)
    } else {
        let (rx, ry) = (length("rx", Axis::X), length("ry", Axis::Y));
        (rx.or(ry), ry.or(rx))
    };
    let rx = radius_x.filter(|r| *r > 0.0).ok_or(ShapeError::Skip)?;
    let ry = radius_y.filter(|r| *r > 0.0).ok_or(ShapeError::Skip)?;
    let frame = Rect::new(center.x - rx, center.y - ry, center.x + rx, center.y + ry);
    check_corners(frame, matrix)?;
    charge(points, 4)?;
    Ok(LeafKind::Ellipse {
        center,
        rx,
        ry,
        matrix,
    })
}

fn charge(points: &mut usize, count: usize) -> Result<(), ShapeError> {
    if count > *points {
        return Err(ShapeError::Budget);
    }
    *points -= count;
    Ok(())
}

/// Skips a shape whose frame leaves the limits after `matrix`.
fn check_corners(rect: Rect, matrix: Affine) -> Result<(), ShapeError> {
    if within_limits(&(matrix * kurbo::Shape::to_path(&rect, 0.0))) {
        Ok(())
    } else {
        Err(ShapeError::Skip)
    }
}

/// The numbers of a `points` list up to the first that cannot be read; an odd count keeps all
/// (the caller pairs them and drops the last).
fn number_list(text: &str) -> Vec<f64> {
    svgtypes::NumberListParser::from(text)
        .map_while(Result::ok)
        .filter(|n| n.is_finite())
        .collect()
}

/// Inkscape's star data of `node`, when complete and not rounded or randomized
/// (criterion 28); the caller still compares it with the drawn path.
fn star_data(node: Node<'_, '_>) -> Option<StarData> {
    if node.attribute((SODIPODI_NS, "type"))? != "star" {
        return None;
    }
    let number = |ns: &str, name: &str| {
        node.attribute((ns, name))
            .and_then(|t| t.trim().parse::<f64>().ok())
            .filter(|n| n.is_finite())
    };
    let zero_or_absent = |name: &str| {
        node.attribute((INKSCAPE_NS, name))
            .is_none_or(|_| number(INKSCAPE_NS, name) == Some(0.0))
    };
    if !zero_or_absent("rounded") || !zero_or_absent("randomized") {
        return None;
    }
    let sides = number(SODIPODI_NS, "sides")?;
    if sides.fract() != 0.0 || !(3.0..=1024.0).contains(&sides) {
        return None;
    }
    // invariant: a whole number from 3 to 1024.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let sides = sides as u32;
    Some(StarData {
        sides,
        center: Point::new(number(SODIPODI_NS, "cx")?, number(SODIPODI_NS, "cy")?),
        r1: number(SODIPODI_NS, "r1")?,
        r2: number(SODIPODI_NS, "r2")?,
        arg1: number(SODIPODI_NS, "arg1")?,
        flat: node.attribute((INKSCAPE_NS, "flatsided")).map(str::trim) == Some("true"),
    })
}
