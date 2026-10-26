//! The raw loops of a stroke (`specs/0085-stroke-to-path` `adrs.md` decision 2): an open chain or
//! a dash becomes one loop (right side forward with joins, end cap, left side backward with joins,
//! start cap), a closed undashed outline two loops (its right-side offset and that of its
//! reverse), a dot of a zero-length dash one loop of its cap's shape. Every loop is wound with the
//! area it adds on its left.

use kurbo::{Line, ParamCurve, PathSeg, Point, Vec2};

use crate::offset_pieces::{
    COINCIDENT_MM, RawStyle, arc, closed_loop, corner, end_tangent, offset_segment, right_of,
    start_tangent,
};

/// How an open end is finished, as `curvyo_document_core::LineCap` says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cap {
    Butt,
    Round,
    Square,
}

/// The two loops of a closed outline stroked by the half width `half`: the outer and the inner
/// side, whichever way it runs.
pub(crate) fn closed_loops(segments: &[PathSeg], half: f64, style: RawStyle) -> [Vec<PathSeg>; 2] {
    let reversed = reversed(segments);
    [
        closed_up(&closed_loop(segments, half, style)),
        closed_up(&closed_loop(&reversed, half, style)),
    ]
}

/// The loop of the open chain `segments` (at least one) stroked by `half` with `cap` at both ends.
pub(crate) fn open_loop(
    segments: &[PathSeg],
    half: f64,
    style: RawStyle,
    cap: Cap,
) -> Vec<PathSeg> {
    let reversed = reversed(segments);
    let mut pieces = side(segments, half, style);
    let right = side(&reversed, half, style);
    // invariant: callers pass at least one segment, so both ends exist.
    let (first, last) = (segments[0], segments[segments.len() - 1]);
    end_cap(
        &mut pieces,
        last.eval(1.0),
        end_tangent(&last),
        right.first(),
        half,
        cap,
    );
    pieces.extend(right);
    let (start, ahead) = (first.eval(0.0), -start_tangent(&first));
    let first_piece = pieces.first().copied();
    end_cap(&mut pieces, start, ahead, first_piece.as_ref(), half, cap);
    closed_up(&pieces)
}

/// The loop of a zero-length dash at `at` heading `ahead` (a unit vector): a circle for a Round
/// cap, a square along the direction for a Square cap, nothing for Butt.
pub(crate) fn dot_loop(at: Point, ahead: Vec2, half: f64, cap: Cap) -> Option<Vec<PathSeg>> {
    match cap {
        Cap::Butt => None,
        Cap::Round => Some(circle_loop(at, half)),
        Cap::Square => {
            let (u, n) = (ahead * half, Vec2::new(-ahead.y, ahead.x) * half);
            let corners = [at - u - n, at + u - n, at + u + n, at - u + n];
            Some(polygon_loop(&corners))
        }
    }
}

/// A full circle around `center`, wound positively.
pub(crate) fn circle_loop(center: Point, radius: f64) -> Vec<PathSeg> {
    let mut pieces = Vec::new();
    let (east, west) = (
        center + Vec2::new(radius, 0.0),
        center - Vec2::new(radius, 0.0),
    );
    arc(&mut pieces, center, east, west, Vec2::new(0.0, 1.0), radius);
    arc(
        &mut pieces,
        center,
        west,
        east,
        Vec2::new(0.0, -1.0),
        radius,
    );
    pieces
}

/// The closed polygon through `corners`, wound positively whatever their order.
pub(crate) fn polygon_loop(corners: &[Point]) -> Vec<PathSeg> {
    let n = corners.len();
    let twice: f64 = (0..n)
        .map(|i| corners[i].to_vec2().cross(corners[(i + 1) % n].to_vec2()))
        .sum();
    let ordered: Vec<Point> = if twice < 0.0 {
        corners.iter().rev().copied().collect()
    } else {
        corners.to_vec()
    };
    (0..n)
        .map(|i| PathSeg::Line(Line::new(ordered[i], ordered[(i + 1) % n])))
        .collect()
}

/// `segments` run backwards.
fn reversed(segments: &[PathSeg]) -> Vec<PathSeg> {
    segments.iter().rev().map(PathSeg::reverse).collect()
}

/// The offset of the chain `segments` by `half` to its right, with the joins between segments.
fn side(segments: &[PathSeg], half: f64, style: RawStyle) -> Vec<PathSeg> {
    let offsets: Vec<Vec<PathSeg>> = segments
        .iter()
        .map(|segment| offset_segment(segment, half, style))
        .collect();
    let mut pieces: Vec<PathSeg> = offsets[0].clone();
    for index in 1..segments.len() {
        corner(
            &mut pieces,
            &segments[index - 1],
            &segments[index],
            &offsets[index],
            half,
            style,
        );
        pieces.extend_from_slice(&offsets[index]);
    }
    pieces
}

/// The cap at the end `at` of a chain heading `ahead`, from the last piece of `pieces` (the side
/// arriving on the right) to the start of `next` (the side leaving on the left).
fn end_cap(
    pieces: &mut Vec<PathSeg>,
    at: Point,
    ahead: Vec2,
    next: Option<&PathSeg>,
    half: f64,
    cap: Cap,
) {
    let across = right_of(ahead) * half;
    let from = pieces.last().map_or(at + across, |piece| piece.eval(1.0));
    let to = next.map_or(at - across, |piece| piece.eval(0.0));
    match cap {
        Cap::Butt => pieces.push(PathSeg::Line(Line::new(from, to))),
        Cap::Square => {
            let reach = ahead * half;
            let (a, b) = (from + reach, to + reach);
            pieces.push(PathSeg::Line(Line::new(from, a)));
            pieces.push(PathSeg::Line(Line::new(a, b)));
            pieces.push(PathSeg::Line(Line::new(b, to)));
        }
        Cap::Round => arc(pieces, at, from, to, ahead, half),
    }
}

/// `pieces` with a straight line wherever one piece does not end where the next starts, the
/// wrap-around included, so the loop is closed.
fn closed_up(pieces: &[PathSeg]) -> Vec<PathSeg> {
    let n = pieces.len();
    let mut out = Vec::with_capacity(n + 2);
    for index in 0..n {
        let piece = pieces[index];
        out.push(piece);
        let (end, start) = (piece.eval(1.0), pieces[(index + 1) % n].eval(0.0));
        if (start - end).hypot() > COINCIDENT_MM {
            out.push(PathSeg::Line(Line::new(end, start)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offset::OffsetJoin;

    fn area(pieces: &[PathSeg]) -> f64 {
        crate::offset_finish::double_area_mm2(pieces) / 2.0
    }

    #[test]
    fn an_open_line_loop_is_wound_positively() {
        let segment = PathSeg::Line(Line::new(Point::new(0.0, 0.0), Point::new(10.0, 0.0)));
        let style = RawStyle {
            join: OffsetJoin::Miter,
            curve_tolerance_mm: 0.001,
            clip_long_miters: false,
        };
        for (cap, expected) in [
            (Cap::Butt, 20.0),
            (Cap::Square, 24.0),
            (Cap::Round, 20.0 + std::f64::consts::PI),
        ] {
            let pieces = open_loop(&[segment], 1.0, style, cap);
            assert!((area(&pieces) - expected).abs() < 0.01, "{cap:?}");
        }
    }

    #[test]
    fn dots_and_polygons_are_wound_positively() {
        let dot = dot_loop(Point::new(1.0, 1.0), Vec2::new(0.0, 1.0), 1.0, Cap::Square).unwrap();
        assert!((area(&dot) - 4.0).abs() < 1e-9);
        let circle = circle_loop(Point::new(0.0, 0.0), 2.0);
        assert!((area(&circle) - std::f64::consts::PI * 4.0).abs() < 0.01);
        assert!(dot_loop(Point::ORIGIN, Vec2::new(1.0, 0.0), 1.0, Cap::Butt).is_none());
    }
}
