//! The exact raw offset of an outline as closed loops of lines and cubics: every segment's parallel
//! curve, and between them the join where a gap opens or the corner point where the curves overlap
//! (`specs/0038-path-offset` criterion 34; ADR 0003 §4 as amended on 2026-10-12).
//!
//! A raw loop may cross itself; `offset_rebuild` lets `i_overlay` decide which parts survive. A
//! loop is wound so that the region it adds lies on its left (positive area in the kernel's
//! convention), and the offset goes to the right of each segment's direction.
//!
//! Lines and arcs are built with `+ - * /` and `sqrt` only. A cubic's parallel curve comes from
//! `kurbo::offset::offset_cubic` (ADR 0003 §2 names `kurbo` for single-curve offsetting).

use kurbo::{BezPath, CubicBez, Line, ParamCurve, PathEl, PathSeg, Point, Vec2};

use crate::offset::OffsetJoin;

/// Distances below this, in millimetres, are one point: segment lengths, gaps between pieces.
const COINCIDENT_MM: f64 = 1e-9;

/// The sine of the turn below which two tangents count as the same direction (no join).
const TANGENT_SINE: f64 = 1e-9;

/// The miter limit of 4, as `1 + cos(turn)`: a miter tip lies `|d| / cos(turn / 2)` from the
/// corner, which is at most `4 |d|` while `1 + cos(turn) ≥ 2 / 16`.
const MITER_LIMIT_ONE_PLUS_COS: f64 = 0.125;

/// Whether a miter at a corner whose edge directions turn by an angle with cosine `cos_turn`
/// stays within the miter limit of 4 (SVG's): its tip lies at most `4 |d|` from the corner.
/// The one rule for the kernel's joins and for primitives that stay primitives.
#[must_use]
pub fn miter_within_limit(cos_turn: f64) -> bool {
    1.0 + cos_turn >= MITER_LIMIT_ONE_PLUS_COS
}

/// Above this radius, in millimetres, an arc is cut into pieces of at most 45° instead of 90°,
/// so that a cubic stays within 0.01 mm of its circle (error 2.7e-4 x r at 90°).
const QUARTER_ARC_RADIUS_LIMIT_MM: f64 = 37.0;

/// The settings that shape the raw offset.
#[derive(Debug, Clone, Copy)]
pub(crate) struct RawStyle {
    /// The join where a gap opens.
    pub join: OffsetJoin,
    /// The accuracy handed to `kurbo`'s cubic offset, in millimetres.
    pub curve_tolerance_mm: f64,
}

/// The segments of an outline through `anchors` (point, handle in, handle out; handles relative),
/// without segments of zero length. A cubic whose control points lie on one line is a line.
pub(crate) fn segments_of(anchors: &[(Point, Vec2, Vec2)], closed: bool) -> Vec<PathSeg> {
    let count = if closed {
        anchors.len()
    } else {
        anchors.len().saturating_sub(1)
    };
    let mut segments = Vec::with_capacity(count);
    for index in 0..count {
        let (p0, _, out) = anchors[index];
        let (p3, hin, _) = anchors[(index + 1) % anchors.len()];
        let (p1, p2) = (p0 + out, p3 + hin);
        let chord = p3 - p0;
        let span = chord.hypot().max((p1 - p0).hypot()).max((p2 - p3).hypot());
        if span <= COINCIDENT_MM {
            continue;
        }
        let flat = (p1 - p0).cross(chord).abs() <= TANGENT_SINE * span * span
            && (p2 - p0).cross(chord).abs() <= TANGENT_SINE * span * span;
        if flat && chord.hypot() > COINCIDENT_MM {
            segments.push(PathSeg::Line(Line::new(p0, p3)));
        } else {
            segments.push(PathSeg::Cubic(CubicBez::new(p0, p1, p2, p3)));
        }
    }
    segments
}

/// The unit direction in which `segment` starts.
pub(crate) fn start_tangent(segment: &PathSeg) -> Vec2 {
    let (p0, rest) = match *segment {
        PathSeg::Line(line) => (line.p0, [line.p1, line.p1, line.p1]),
        PathSeg::Cubic(cubic) => (cubic.p0, [cubic.p1, cubic.p2, cubic.p3]),
        PathSeg::Quad(quad) => (quad.p0, [quad.p1, quad.p2, quad.p2]),
    };
    first_direction(p0, &rest)
}

/// The unit direction in which `segment` ends.
pub(crate) fn end_tangent(segment: &PathSeg) -> Vec2 {
    let (p3, rest) = match *segment {
        PathSeg::Line(line) => (line.p1, [line.p0, line.p0, line.p0]),
        PathSeg::Cubic(cubic) => (cubic.p3, [cubic.p2, cubic.p1, cubic.p0]),
        PathSeg::Quad(quad) => (quad.p2, [quad.p1, quad.p0, quad.p0]),
    };
    -first_direction(p3, &rest)
}

/// The unit vector from `from` to the first of `towards` that is not `from` itself.
fn first_direction(from: Point, towards: &[Point; 3]) -> Vec2 {
    for &point in towards {
        let v = point - from;
        let length = v.hypot();
        if length > COINCIDENT_MM {
            return v / length;
        }
    }
    Vec2::ZERO
}

/// The unit normal to the right of the unit direction `u`.
fn right_of(u: Vec2) -> Vec2 {
    Vec2::new(u.y, -u.x)
}

/// The raw offset loop of the closed outline `segments` by `d` (positive to the right).
pub(crate) fn closed_loop(segments: &[PathSeg], d: f64, style: RawStyle) -> Vec<PathSeg> {
    let mut pieces = Vec::new();
    let offsets: Vec<Vec<PathSeg>> = segments
        .iter()
        .map(|segment| offset_segment(segment, d, style))
        .collect();
    for (index, offset) in offsets.iter().enumerate() {
        let next = (index + 1) % segments.len();
        pieces.extend_from_slice(offset);
        corner(
            &mut pieces,
            &segments[index],
            &segments[next],
            &offsets[next],
            d,
            style,
        );
    }
    pieces
}

/// The parallel curve of one segment at `d` to its right.
fn offset_segment(segment: &PathSeg, d: f64, style: RawStyle) -> Vec<PathSeg> {
    match *segment {
        PathSeg::Line(line) => {
            let shift = right_of(start_tangent(segment)) * d;
            vec![PathSeg::Line(Line::new(line.p0 + shift, line.p1 + shift))]
        }
        PathSeg::Cubic(cubic) => {
            let mut path = BezPath::new();
            // `kurbo` offsets to the left of the direction of travel for a positive distance.
            kurbo::offset::offset_cubic(cubic, -d, style.curve_tolerance_mm, &mut path);
            let mut pieces = Vec::new();
            let mut current = None;
            for element in path.elements() {
                match *element {
                    PathEl::MoveTo(p) => current = Some(p),
                    PathEl::LineTo(p) => {
                        if let Some(from) = current {
                            pieces.push(PathSeg::Line(Line::new(from, p)));
                        }
                        current = Some(p);
                    }
                    PathEl::QuadTo(p1, p2) => {
                        if let Some(from) = current {
                            pieces.push(PathSeg::Cubic(kurbo::QuadBez::new(from, p1, p2).raise()));
                        }
                        current = Some(p2);
                    }
                    PathEl::CurveTo(p1, p2, p3) => {
                        if let Some(from) = current {
                            pieces.push(PathSeg::Cubic(CubicBez::new(from, p1, p2, p3)));
                        }
                        current = Some(p3);
                    }
                    PathEl::ClosePath => {}
                }
            }
            pieces
        }
        PathSeg::Quad(quad) => offset_segment(&PathSeg::Cubic(quad.raise()), d, style),
    }
}

/// The pieces between the offset of `before` and the offset `after_offset` of `after`, which meet
/// at their shared anchor.
fn corner(
    pieces: &mut Vec<PathSeg>,
    before: &PathSeg,
    after: &PathSeg,
    after_offset: &[PathSeg],
    d: f64,
    style: RawStyle,
) {
    let (Some(last), Some(first)) = (pieces.last(), after_offset.first()) else {
        return;
    };
    let (end, start) = (last.eval(1.0), first.eval(0.0));
    let at = before.eval(1.0);
    let (u1, u2) = (end_tangent(before), start_tangent(after));
    let (cross, dot) = (u1.cross(u2), u1.dot(u2));
    if cross.abs() <= TANGENT_SINE && dot > 0.0 {
        if (start - end).hypot() > COINCIDENT_MM {
            pieces.push(PathSeg::Line(Line::new(end, start)));
        }
        return;
    }
    let gap = if cross.abs() <= TANGENT_SINE {
        d > 0.0
    } else {
        cross * d > 0.0
    };
    if !gap {
        pieces.push(PathSeg::Line(Line::new(end, at)));
        pieces.push(PathSeg::Line(Line::new(at, start)));
        return;
    }
    let radius = d.abs();
    match style.join {
        OffsetJoin::Bevel => pieces.push(PathSeg::Line(Line::new(end, start))),
        OffsetJoin::Round => arc(pieces, at, end, start, u1, radius),
        OffsetJoin::Miter => {
            if miter_within_limit(dot) {
                let (n1, n2) = (right_of(u1), right_of(u2));
                let tip = at + (n1 + n2) * (d / (1.0 + dot));
                pieces.push(PathSeg::Line(Line::new(end, tip)));
                pieces.push(PathSeg::Line(Line::new(tip, start)));
            } else {
                // SVG 2 `miter-clip`: both edges extended to 4 |d| from the corner, where
                // `|d|² + s² = (4 |d|)²` gives `s = sqrt(15) |d|` along each edge.
                let reach = 15.0_f64.sqrt() * radius;
                let (a, b) = (end + u1 * reach, start - u2 * reach);
                pieces.push(PathSeg::Line(Line::new(end, a)));
                pieces.push(PathSeg::Line(Line::new(a, b)));
                pieces.push(PathSeg::Line(Line::new(b, start)));
            }
        }
    }
}

/// A circular arc around `center` from `from` to `to` (both at `radius` from it), as cubics of at
/// most 90° (45° above 37 mm). The arc turns the short way; for a half turn it passes through
/// `center + radius x ahead`.
fn arc(pieces: &mut Vec<PathSeg>, center: Point, from: Point, to: Point, ahead: Vec2, radius: f64) {
    if radius <= COINCIDENT_MM {
        return;
    }
    let (a, b) = ((from - center) / radius, (to - center) / radius);
    let largest_cos = if radius > QUARTER_ARC_RADIUS_LIMIT_MM {
        std::f64::consts::FRAC_1_SQRT_2
    } else {
        0.0
    };
    let sum = a + b;
    if sum.hypot() <= TANGENT_SINE {
        // A half turn: split at the point ahead first.
        let mid = ahead.normalize();
        arc_part(pieces, center, a, mid, radius, largest_cos);
        arc_part(pieces, center, mid, b, radius, largest_cos);
    } else {
        arc_part(pieces, center, a, b, radius, largest_cos);
    }
}

/// The arc between the unit directions `a` and `b` (less than a half turn apart), halved until
/// each part's cosine is at least `largest_cos`.
fn arc_part(
    pieces: &mut Vec<PathSeg>,
    center: Point,
    a: Vec2,
    b: Vec2,
    radius: f64,
    largest_cos: f64,
) {
    let cos = a.dot(b).clamp(-1.0, 1.0);
    if cos < largest_cos - 1e-12 {
        let mid = (a + b).normalize();
        arc_part(pieces, center, a, mid, radius, largest_cos);
        arc_part(pieces, center, mid, b, radius, largest_cos);
        return;
    }
    // The handle length of a cubic arc over the angle α is `4/3 tan(α/4) r`, with
    // `tan(α/4) = sin(α/2) / (1 + cos(α/2))` and the half angles from `cos α` by `sqrt`.
    let half_cos = f64::midpoint(1.0, cos).sqrt();
    let half_sin = ((1.0 - cos) / 2.0).max(0.0).sqrt();
    let handle = 4.0 / 3.0 * half_sin / (1.0 + half_cos) * radius;
    let turn = if a.cross(b) >= 0.0 { 1.0 } else { -1.0 };
    let perpendicular = |v: Vec2| Vec2::new(-v.y, v.x) * turn;
    let (p0, p3) = (center + a * radius, center + b * radius);
    pieces.push(PathSeg::Cubic(CubicBez::new(
        p0,
        p0 + perpendicular(a) * handle,
        p3 - perpendicular(b) * handle,
        p3,
    )));
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLE: RawStyle = RawStyle {
        join: OffsetJoin::Round,
        curve_tolerance_mm: 0.001,
    };

    fn square() -> Vec<PathSeg> {
        // Positive area in the kernel's convention: (0,0) → (20,0) → (20,20) → (0,20).
        let p = [
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(20.0, 20.0),
            Point::new(0.0, 20.0),
        ];
        (0..4)
            .map(|i| PathSeg::Line(Line::new(p[i], p[(i + 1) % 4])))
            .collect()
    }

    #[test]
    fn a_square_grows_by_parallel_lines_and_quarter_arcs() {
        let raw = closed_loop(&square(), 2.0, STYLE);
        assert_eq!(raw.len(), 8);
        // The first edge runs along y = 0 and moves to its right, which is y = -2.
        assert!((raw[0].eval(0.5).y + 2.0).abs() < 1e-12);
        let arc = raw[1];
        let middle = arc.eval(0.5);
        let r = (middle - Point::new(20.0, 0.0)).hypot();
        assert!((r - 2.0).abs() < 2.0 * 2.8e-4, "{r}");
        for pair in raw.windows(2) {
            assert!((pair[0].eval(1.0) - pair[1].eval(0.0)).hypot() < 1e-9);
        }
    }

    #[test]
    fn a_square_shrinks_through_its_corners() {
        let raw = closed_loop(&square(), -2.0, STYLE);
        // Four edges, each corner joined through the corner point (two lines).
        assert_eq!(raw.len(), 12);
    }

    #[test]
    fn miter_tips_and_clips() {
        let mut style = STYLE;
        style.join = OffsetJoin::Miter;
        let raw = closed_loop(&square(), 5.0, style);
        let tip = raw[1].eval(1.0);
        assert!((tip - Point::new(25.0, -5.0)).hypot() < 1e-12);
    }

    #[test]
    fn a_cubic_offset_goes_to_the_right() {
        let cubic = CubicBez::new(
            Point::new(0.0, 0.0),
            Point::new(3.0, 1.0),
            Point::new(7.0, 1.0),
            Point::new(10.0, 0.0),
        );
        let pieces = offset_segment(&PathSeg::Cubic(cubic), 1.0, STYLE);
        let start = pieces[0].eval(0.0);
        let expected = Point::new(0.0, 0.0) + right_of(start_tangent(&PathSeg::Cubic(cubic)));
        assert!((start - expected).hypot() < 1e-6, "{start:?} {expected:?}");
    }
}
