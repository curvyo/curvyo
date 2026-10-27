//! Whether a primitive draws the line of an Inkscape `d` (`specs/0024-svg-import-export`
//! criterion 111): the same subpaths, each open or closed alike, and every point of one within
//! a distance of the other, both ways, bounded exactly on flattened outlines.

use std::collections::HashMap;

use curvyo_document_core::{Angle, ArcKind, PrimitiveOutline, Shape, drawing_outline_of_rotated};
use kurbo::{BezPath, Line, ParamCurve, ParamCurveNearest, PathEl, Point, Shape as _, Vec2};

/// How far the outline of a primitive may be from the drawn `d`, millimetres (criterion 111).
pub(crate) const MATCH_MM: f64 = 1e-3;

/// How close the comparison path of an ellipse or arc is to the true curve, millimetres.
const ELLIPSE_ACCURACY_MM: f64 = 1e-6;

/// How far each outline's polyline may be from the outline, millimetres.
const FLAT_MM: f64 = 1e-5;

/// A piece shorter than this is judged by its ends, millimetres.
const SHORTEST_PIECE_MM: f64 = 1e-7;

/// How far a point of one polyline may be from the other: the polylines' own error and the
/// error of judging the shortest piece by its ends are taken off, so a match bounds the true
/// distance by [`MATCH_MM`].
const POLYLINE_MM: f64 = MATCH_MM - 2.0 * FLAT_MM - ELLIPSE_ACCURACY_MM - SHORTEST_PIECE_MM;

/// Whether `shape` turned by `rotation` draws `drawn` within [`MATCH_MM`]. An ellipse or an arc
/// is compared as the true curve (`specs/0024-svg-import-export`, lead 2026-10-27: the
/// primitive is exact, its drawing in cubics is a rendering detail); the rest as its outline.
pub(crate) fn draws(shape: &Shape, rotation: Angle, drawn: &BezPath) -> bool {
    let (ours, theirs) = (subpaths(&reference(shape, rotation)), subpaths(drawn));
    !ours.is_empty()
        && ours.len() == theirs.len()
        && ours.iter().zip(&theirs).all(|(a, b)| {
            a.closed == b.closed && covers(&a.lines, &b.lines) && covers(&b.lines, &a.lines)
        })
}

/// The outline `shape` stands for: the true ellipse or arc, else its drawing outline.
fn reference(shape: &Shape, rotation: Angle) -> BezPath {
    let Shape::Ellipse { frame, arc, .. } = *shape else {
        return outline_path(&drawing_outline_of_rotated(shape, rotation));
    };
    let center = Point::new(frame.center.x, frame.center.y);
    let radii = Vec2::new(frame.rx.as_mm(), frame.ry.as_mm());
    let Some(arc) = arc else {
        return kurbo::Ellipse::new(center, radii, rotation.as_radians())
            .to_path(ELLIPSE_ACCURACY_MM);
    };
    let curve = kurbo::Arc::new(
        center,
        radii,
        arc.start().as_radians(),
        arc.sweep().as_radians(),
        rotation.as_radians(),
    );
    let mut path = curve.to_path(ELLIPSE_ACCURACY_MM);
    match arc.kind {
        ArcKind::Arc => {}
        ArcKind::Chord => path.close_path(),
        ArcKind::Pie => {
            path.line_to(center);
            path.close_path();
        }
    }
    path
}

/// One subpath flattened to lines, and whether it is closed: by `Z`, or by ending within twice
/// [`MATCH_MM`] of where it starts (the missing closing line is then within [`MATCH_MM`] of its
/// ends, as Inkscape's arc of 2π − 0.0001 rad).
struct Polyline {
    lines: Vec<Line>,
    closed: bool,
}

/// The subpaths of `path` that draw something, flattened within [`FLAT_MM`].
fn subpaths(path: &BezPath) -> Vec<Polyline> {
    let mut out: Vec<Polyline> = Vec::new();
    let mut current = Polyline {
        lines: Vec::new(),
        closed: false,
    };
    let (mut start, mut at) = (Point::ORIGIN, Point::ORIGIN);
    let mut finish = |current: &mut Polyline, start: Point, at: Point| {
        let mut done = std::mem::replace(
            current,
            Polyline {
                lines: Vec::new(),
                closed: false,
            },
        );
        if !done.lines.is_empty() {
            done.closed |= at.distance(start) <= 2.0 * MATCH_MM;
            out.push(done);
        }
    };
    kurbo::flatten(path, FLAT_MM, |el| match el {
        PathEl::MoveTo(p) => {
            finish(&mut current, start, at);
            (start, at) = (p, p);
        }
        PathEl::LineTo(p) => {
            current.lines.push(Line::new(at, p));
            at = p;
        }
        PathEl::ClosePath => {
            if at != start {
                current.lines.push(Line::new(at, start));
            }
            current.closed = true;
            finish(&mut current, start, start);
            at = start;
        }
        // invariant: `flatten` gives only moves, lines and closes.
        PathEl::QuadTo(..) | PathEl::CurveTo(..) => {}
    });
    finish(&mut current, start, at);
    out
}

/// Every point of `of` lies within [`POLYLINE_MM`] of `near`: the distance to one line is
/// convex along a line, so a piece of `of` whose ends are both near one line of `near` is near
/// all along; any other piece is split (at the shared corner of the two lines its ends are
/// near, else halved) until it is, or a point is too far.
fn covers(of: &[Line], near: &[Line]) -> bool {
    let near = Near {
        lines: near,
        grid: std::cell::OnceCell::new(),
        hint: std::cell::Cell::new(0),
    };
    of.iter().all(|line| near.covers(*line))
}

/// Lines of `near` tried before the grid: the one the last piece was near and its neighbours.
const HINT_REACH: usize = 2;

/// The lines of a polyline, looked up first around the line the last piece was near (the two
/// outlines run alongside), else through a grid of square cells.
struct Near<'a> {
    lines: &'a [Line],
    grid: std::cell::OnceCell<Grid>,
    hint: std::cell::Cell<usize>,
}

/// What one lookup found for the ends of a piece.
#[derive(Default)]
struct Found {
    /// A line near both ends.
    both: Option<usize>,
    /// A line near the start and one near the end.
    start: Option<usize>,
    end: Option<usize>,
}

impl Near<'_> {
    fn is_near(&self, i: usize, p: Point) -> bool {
        self.lines[i].nearest(p, 0.0).distance_sq <= POLYLINE_MM * POLYLINE_MM
    }

    fn look(&self, piece: Line, found: &mut Found, candidates: impl Iterator<Item = usize>) {
        for i in candidates {
            let (a, b) = (self.is_near(i, piece.p0), self.is_near(i, piece.p1));
            if a && b {
                found.both = Some(i);
                return;
            }
            if a {
                found.start.get_or_insert(i);
            }
            if b {
                found.end.get_or_insert(i);
            }
        }
    }

    fn find(&self, piece: Line) -> Found {
        let mut found = Found::default();
        let n = self.lines.len();
        let hint = self.hint.get();
        let window = (0..=2 * HINT_REACH).map(|k| (hint + n * HINT_REACH + k - HINT_REACH) % n);
        self.look(piece, &mut found, window);
        if found.both.is_none() && (found.start.is_none() || found.end.is_none()) {
            let grid = self.grid.get_or_init(|| Grid::new(self.lines));
            self.look(piece, &mut found, grid.around(piece));
        }
        if let Some(i) = found.both.or(found.end) {
            self.hint.set(i);
        }
        found
    }

    /// Every point of `piece` is within [`POLYLINE_MM`] of a line.
    fn covers(&self, piece: Line) -> bool {
        let found = self.find(piece);
        if found.both.is_some() {
            return true;
        }
        let (Some(i), Some(j)) = (found.start, found.end) else {
            return false;
        };
        if piece.length() <= SHORTEST_PIECE_MM {
            return true;
        }
        let corner = [self.lines[i].p0, self.lines[i].p1]
            .into_iter()
            .find(|c| *c == self.lines[j].p0 || *c == self.lines[j].p1);
        let at = corner
            .map(|c| piece.nearest(c, 0.0).t)
            .filter(|t| (0.01..=0.99).contains(t))
            .unwrap_or(0.5);
        let mid = piece.eval(at);
        self.covers(Line::new(piece.p0, mid)) && self.covers(Line::new(mid, piece.p1))
    }
}

/// The indices of a polyline's lines, bucketed by square cells.
struct Grid {
    cell: f64,
    cells: HashMap<(i64, i64), Vec<usize>>,
}

impl Grid {
    fn new(lines: &[Line]) -> Self {
        #[allow(clippy::cast_precision_loss)]
        let mean = lines.iter().map(|l| l.length()).sum::<f64>() / lines.len().max(1) as f64;
        let cell = mean.max(MATCH_MM * 4.0);
        let mut cells: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
        for (i, line) in lines.iter().enumerate() {
            // Every point of the line is within half a cell of one of these samples.
            // invariant: the count is at most the line's length over a cell, plus one.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let steps = (line.length() / cell).ceil().max(1.0) as u32;
            for k in 0..=steps {
                let key = Self::key(line.eval(f64::from(k) / f64::from(steps)), cell);
                let bucket = cells.entry(key).or_default();
                if bucket.last() != Some(&i) {
                    bucket.push(i);
                }
            }
        }
        Self { cell, cells }
    }

    fn key(p: Point, cell: f64) -> (i64, i64) {
        // invariant: coordinates are finite; `as` saturates far outside the page.
        #[allow(clippy::cast_possible_truncation)]
        ((p.x / cell).floor() as i64, (p.y / cell).floor() as i64)
    }

    /// The lines that may come within [`POLYLINE_MM`] of the segment `piece`.
    fn around(&self, piece: Line) -> impl Iterator<Item = usize> + '_ {
        let room = POLYLINE_MM + self.cell;
        let lo = Self::key(
            Point::new(
                piece.p0.x.min(piece.p1.x) - room,
                piece.p0.y.min(piece.p1.y) - room,
            ),
            self.cell,
        );
        let hi = Self::key(
            Point::new(
                piece.p0.x.max(piece.p1.x) + room,
                piece.p0.y.max(piece.p1.y) + room,
            ),
            self.cell,
        );
        (lo.0..=hi.0)
            .flat_map(move |x| (lo.1..=hi.1).map(move |y| (x, y)))
            .filter_map(|key| self.cells.get(&key))
            .flatten()
            .copied()
    }
}

/// A primitive's outline as a path: a cubic between two nodes with handles, else a line.
fn outline_path(outline: &PrimitiveOutline) -> BezPath {
    let mut path = BezPath::new();
    let at = |p: curvyo_document_core::Point| Point::new(p.x, p.y);
    let Some(first) = outline.anchors.first() else {
        return path;
    };
    path.move_to(at(first.point));
    let count = outline.anchors.len();
    let segments = if outline.closed { count } else { count - 1 };
    for i in 0..segments {
        let (from, to) = (&outline.anchors[i], &outline.anchors[(i + 1) % count]);
        let straight = from.handle_out.x == 0.0
            && from.handle_out.y == 0.0
            && to.handle_in.x == 0.0
            && to.handle_in.y == 0.0;
        if straight {
            path.line_to(at(to.point));
        } else {
            let c1 = at(from.point) + kurbo::Vec2::new(from.handle_out.x, from.handle_out.y);
            let c2 = at(to.point) + kurbo::Vec2::new(to.handle_in.x, to.handle_in.y);
            path.curve_to(c1, c2, at(to.point));
        }
    }
    if outline.closed {
        path.close_path();
    }
    path
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use curvyo_document_core::{Curve, EllipseFrame, Length, Point as DocPoint};

    fn circle() -> Shape {
        Shape::Ellipse {
            frame: EllipseFrame {
                center: DocPoint::new(0.0, 0.0),
                rx: Length::from_mm(10.0),
                ry: Length::from_mm(10.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        }
    }

    #[test]
    fn ac111_within_and_beyond_the_distance() {
        let drawn = |r: f64| kurbo::Circle::new((0.0, 0.0), r).to_path(1e-7);
        let rotation = Angle::from_radians(0.0);
        assert!(draws(&circle(), rotation, &drawn(10.0005)));
        assert!(!draws(&circle(), rotation, &drawn(10.002)));
        let mut half = BezPath::new();
        half.move_to((10.0, 0.0));
        half.line_to((-10.0, 0.0));
        assert!(!draws(&circle(), rotation, &half), "both ways");
        assert!(!draws(&circle(), rotation, &BezPath::new()));
    }

    /// Appends the arc of the ellipse about (0, 0) with radii `r` from `start` over `sweep`,
    /// split within 1e-7 mm (as the import's fine `d`).
    fn arc_to(path: &mut BezPath, r: (f64, f64), start: f64, sweep: f64) {
        path.extend(kurbo::Arc::new((0.0, 0.0), r, start, sweep, 0.0).append_iter(1e-7));
    }

    fn hexagon() -> Shape {
        Shape::Polygon {
            frame: curvyo_document_core::StarFrame {
                center: DocPoint::new(0.0, 0.0),
                radius: Length::from_mm(10.0),
                angle: Angle::from_radians(0.0),
            },
            point_count: curvyo_document_core::PointCount::new(6).unwrap(),
            curve: Curve::POLYGON_DEFAULT,
        }
    }

    /// The hexagon's corners, the first at (10, 0).
    fn corners() -> Vec<Point> {
        (0..6)
            .map(|k| {
                let t = f64::from(k) * std::f64::consts::FRAC_PI_3;
                Point::new(10.0 * t.cos(), 10.0 * t.sin())
            })
            .collect()
    }

    #[test]
    fn ac111_a_gap_between_any_samples_is_found() {
        let rotation = Angle::from_radians(0.0);
        let c = corners();
        let mut whole = BezPath::new();
        whole.move_to(c[0]);
        for p in &c[1..] {
            whole.line_to(*p);
        }
        whole.close_path();
        assert!(draws(&hexagon(), rotation, &whole));
        // The last side split at 0.51 to 0.55: two open subpaths.
        let (a, b) = (c[5], c[0]);
        let mut gap = BezPath::new();
        gap.move_to(c[0]);
        for p in &c[1..] {
            gap.line_to(*p);
        }
        gap.line_to(a.lerp(b, 0.51));
        gap.move_to(a.lerp(b, 0.55));
        gap.line_to(b);
        assert!(!draws(&hexagon(), rotation, &gap), "two subpaths");
    }

    #[test]
    fn ac111_open_against_closed_is_no_match() {
        let rotation = Angle::from_radians(0.0);
        let c = corners();
        let mut open = BezPath::new();
        open.move_to(c[0]);
        for p in &c[1..] {
            open.line_to(*p);
        }
        assert!(!draws(&hexagon(), rotation, &open));
    }

    #[test]
    fn ac111_a_circle_with_a_small_gap_is_no_whole_ellipse() {
        let rotation = Angle::from_radians(0.0);
        let end = std::f64::consts::TAU - 0.0015;
        let mut gap = BezPath::new();
        gap.move_to((10.0, 0.0));
        arc_to(&mut gap, (10.0, 10.0), 0.0, end);
        assert!(!draws(&circle(), rotation, &gap));
        let mut whole = BezPath::new();
        whole.move_to((10.0, 0.0));
        arc_to(&mut whole, (10.0, 10.0), 0.0, std::f64::consts::PI);
        arc_to(
            &mut whole,
            (10.0, 10.0),
            std::f64::consts::PI,
            std::f64::consts::PI,
        );
        whole.close_path();
        assert!(draws(&circle(), rotation, &whole));
    }

    fn pie(rotation: f64) -> (Shape, Angle) {
        let arc = curvyo_document_core::EllipseArc::new(
            Angle::from_radians(0.0),
            Angle::from_radians(std::f64::consts::FRAC_PI_2),
            curvyo_document_core::ArcKind::Pie,
        );
        let mut shape = circle();
        if let Shape::Ellipse { arc: a, frame, .. } = &mut shape {
            *a = arc;
            frame.ry = Length::from_mm(6.0);
        }
        (shape, Angle::from_radians(rotation))
    }

    #[test]
    fn ac111_an_arc_is_compared_with_the_true_ellipse() {
        let (shape, rotation) = pie(0.5);
        let drawn = outline_path(&drawing_outline_of_rotated(&shape, rotation));
        assert!(draws(&shape, rotation, &drawn));
        // 200 times the size: the 22.5° drawing is 3e-6 of the radius off, the true arc is not.
        let (_, r0) = pie(0.0);
        let mut big = pie(0.0).0;
        if let Shape::Ellipse { frame, .. } = &mut big {
            frame.rx = Length::from_mm(2000.0);
            frame.ry = Length::from_mm(1200.0);
        }
        let quarter = kurbo::Arc::new(
            (0.0, 0.0),
            (2000.0, 1200.0),
            0.0,
            std::f64::consts::FRAC_PI_2,
            0.0,
        );
        let mut fine = BezPath::new();
        fine.move_to((2000.0, 0.0));
        fine.extend(quarter.append_iter(1e-7));
        fine.line_to((0.0, 0.0));
        fine.close_path();
        assert!(draws(&big, r0, &fine));
    }

    #[test]
    fn ac111_a_pie_that_retraces_an_edge_matches() {
        let (shape, rotation) = pie(0.0);
        let mut d = BezPath::new();
        d.move_to((10.0, 0.0));
        arc_to(&mut d, (10.0, 6.0), 0.0, std::f64::consts::FRAC_PI_2);
        d.line_to((0.0, 0.0));
        d.line_to((10.0, 0.0));
        d.line_to((0.0, 0.0));
        d.close_path();
        assert!(draws(&shape, rotation, &d));
    }
}
