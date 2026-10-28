//! One boundary through visioncortex (ADR 0023 §3 steps 4 and 5): a cropped bit image of a
//! region (or a hole) in, its smoothed closed outline as a `kurbo::BezPath` in pixel coordinates
//! out, near-straight cubics made lines.

use kurbo::{BezPath, Point};
use visioncortex::{BinaryImage, PathI32, PathSimplifyMode, PointF64, Spline};

use crate::params::CurveSettings;

/// visioncortex's outset ratio of its spline mode (`Cluster::image_to_splines`).
const OUTSET_RATIO: f64 = 8.0;

/// A cubic whose control points lie within this of its chord, in pixels, is a line.
const STRAIGHT_PX: f64 = 0.01;

/// A crop of the mask: the pixels of one boundary in the box at `(left, top)`.
pub(crate) struct Crop {
    pub left: usize,
    pub top: usize,
    pub image: BinaryImage,
}

/// The outline of the shape in `crop`, clockwise for an outer boundary, counter-clockwise for a
/// hole, in pixel-corner coordinates of the whole picture; `None` when it has no extent.
pub(crate) fn trace_crop(crop: &Crop, outer: bool, settings: CurveSettings) -> Option<BezPath> {
    let spline = Spline::from_image(
        &crop.image,
        outer,
        settings.corner_threshold,
        OUTSET_RATIO,
        settings.segment_length,
        settings.max_iterations,
        settings.splice_threshold,
    );
    #[allow(clippy::cast_precision_loss)] // picture coordinates are at most 8192
    let offset = Point::new(crop.left as f64, crop.top as f64);
    if spline.is_empty() {
        // Too small to smooth (a pixel or a short line): its pixel border as a polygon.
        // A single pixel simplifies to nothing; its unsimplified border is the square.
        return [PathSimplifyMode::Polygon, PathSimplifyMode::None]
            .into_iter()
            .find_map(|mode| {
                polygon_path(&PathI32::image_to_path(&crop.image, outer, mode), offset)
            });
    }
    Some(spline_path(&spline, offset))
}

/// `spline` (a start point, then control-point triples) as a closed path moved by `offset`.
fn spline_path(spline: &Spline, offset: Point) -> BezPath {
    let at = |p: &PointF64| Point::new(p.x + offset.x, p.y + offset.y);
    let mut path = BezPath::new();
    let mut points = spline.points.iter();
    let Some(start) = points.next() else {
        return path;
    };
    let mut current = at(start);
    path.move_to(current);
    let rest: Vec<Point> = points.map(at).collect();
    for &[c1, c2, end] in rest.as_chunks::<3>().0 {
        if near_chord(current, end, c1) && near_chord(current, end, c2) {
            path.line_to(end);
        } else {
            path.curve_to(c1, c2, end);
        }
        current = end;
    }
    path.close_path();
    path
}

/// Whether `p` lies within [`STRAIGHT_PX`] of the segment `a`–`b`.
fn near_chord(a: Point, b: Point, p: Point) -> bool {
    let chord = b - a;
    let length_sq = chord.hypot2();
    let distance = if length_sq == 0.0 {
        (p - a).hypot()
    } else {
        let t = ((p - a).dot(chord) / length_sq).clamp(0.0, 1.0);
        (p - (a + chord * t)).hypot()
    };
    distance <= STRAIGHT_PX
}

/// A pixel-border polygon as a closed path moved by `offset`; `None` below three corners.
fn polygon_path(polygon: &PathI32, offset: Point) -> Option<BezPath> {
    let mut corners: Vec<Point> = polygon
        .path
        .iter()
        .map(|p| Point::new(f64::from(p.x) + offset.x, f64::from(p.y) + offset.y))
        .collect();
    if corners.len() > 1 && corners.first() == corners.last() {
        corners.pop();
    }
    // Points on the straight line between their neighbours are no corners.
    let mut index = 0;
    while corners.len() >= 3 && index < corners.len() {
        let n = corners.len();
        let (a, b, c) = (
            corners[(index + n - 1) % n],
            corners[index],
            corners[(index + 1) % n],
        );
        if (b - a).cross(c - b).abs() < 1e-12 {
            corners.remove(index);
        } else {
            index += 1;
        }
    }
    if corners.len() < 3 {
        return None;
    }
    let mut path = BezPath::new();
    path.move_to(corners[0]);
    for &corner in &corners[1..] {
        path.line_to(corner);
    }
    path.close_path();
    Some(path)
}

/// The anchors of a closed outline: one per segment, the closing one not counted twice.
pub(crate) fn node_count(path: &BezPath) -> usize {
    use kurbo::PathEl;
    let mut start = None;
    let mut last = None;
    let mut segments = 0;
    for element in path.elements() {
        match *element {
            PathEl::MoveTo(p) => {
                start = Some(p);
                last = Some(p);
            }
            PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) => {
                segments += 1;
                last = Some(p);
            }
            PathEl::ClosePath => {}
        }
    }
    // The last segment ends on the start: its end is the first anchor again.
    if segments > 0 && start == last {
        segments
    } else {
        segments + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params::curve_settings;

    fn crop(rows: &[&str]) -> Crop {
        let mut image = BinaryImage::new_w_h(rows[0].len(), rows.len());
        for (y, row) in rows.iter().enumerate() {
            for (x, c) in row.chars().enumerate() {
                image.set_pixel(x, y, c == '#');
            }
        }
        Crop {
            left: 10,
            top: 20,
            image,
        }
    }

    #[test]
    fn a_single_pixel_is_its_square() {
        let path = trace_crop(&crop(&["#"]), true, curve_settings(80, 70)).unwrap();
        let bounds = kurbo::Shape::bounding_box(&path);
        assert!(
            (bounds.x0 - 10.0).abs() < 0.5 && (bounds.y0 - 20.0).abs() < 0.5,
            "{bounds:?}"
        );
        assert!(node_count(&path) >= 3);
    }

    #[test]
    fn a_thin_line_traces() {
        let path = trace_crop(&crop(&["##########"]), true, curve_settings(80, 70)).unwrap();
        let area = kurbo::Shape::area(&path).abs();
        assert!(area > 5.0 && area < 15.0, "{area}");
    }

    #[test]
    fn a_square_stays_four_lines() {
        let rows = vec!["##########"; 10];
        let path = trace_crop(&crop(&rows), true, curve_settings(80, 70)).unwrap();
        assert_eq!(node_count(&path), 4, "{path:?}");
        let area = kurbo::Shape::area(&path).abs();
        assert!((area - 100.0).abs() < 1e-9, "{area}");
    }
}
