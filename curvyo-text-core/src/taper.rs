//! The taper of one glyph in its own frame (`specs/0079-text-layout-controls` criteria 17, 18;
//! `adrs.md` decisions 8, 9): every point keeps its height and moves towards or away from the
//! middle of the advance by a scale that grows linearly with the height.
//!
//! In the glyph frame (mm, y down, the glyph's baseline at y = 0) the point `(x, y)` goes to
//! `(x_c + (x − x_c)·s, y)` with `s = max(0.05, 1 + t(2h_y/h − 1))`, `h_y = −y` the height above
//! the baseline and `h` the size. Lines that are horizontal or vertical stay lines (the map is
//! affine along them), other lines become exact quadratics, curves are refitted with
//! `kurbo::fit_to_bezpath` within the asked accuracy.

use kurbo::{
    BezPath, CubicBez, CurveFitSample, ParamCurve, ParamCurveDeriv, ParamCurveFit, PathEl, Point,
    QuadBez, Vec2, common::solve_cubic, fit_to_bezpath,
};

/// The smallest scale of the map (criterion 17).
const MIN_SCALE: f64 = 0.05;
/// Two coordinates closer than this (mm) are equal: a line along them is horizontal or vertical.
const SAME_MM: f64 = 1e-9;

/// The taper of one glyph: its middle, the amount and the size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Taper {
    /// The x of the middle of the glyph's advance, in the glyph frame.
    pub centre_x: f64,
    /// The taper `t`, a fraction (0.2 is 20 %).
    pub amount: f64,
    /// The size `h` (mm), the height of the capital H.
    pub size: f64,
}

impl Taper {
    /// The scale at frame height `y` (y down) before the clamp: `1 + t(2(−y)/h − 1)`.
    fn raw_scale(&self, y: f64) -> f64 {
        1.0 + self.amount * (-2.0 * y / self.size - 1.0)
    }

    /// The scale at frame height `y`, never below [`MIN_SCALE`].
    fn scale(&self, y: f64) -> f64 {
        self.raw_scale(y).max(MIN_SCALE)
    }

    /// The scale's derivative by `y`: 0 where it is clamped.
    fn scale_slope(&self, y: f64) -> f64 {
        if self.raw_scale(y) < MIN_SCALE {
            0.0
        } else {
            -2.0 * self.amount / self.size
        }
    }

    /// The frame y where the scale reaches [`MIN_SCALE`]; `None` without a taper.
    fn clamp_y(&self) -> Option<f64> {
        (self.amount != 0.0 && self.size > 0.0)
            .then(|| -self.size / 2.0 * (1.0 + (MIN_SCALE - 1.0) / self.amount))
    }

    fn clamped(&self, y: f64) -> bool {
        self.raw_scale(y) < MIN_SCALE
    }

    /// The map of one point.
    pub fn point(&self, p: Point) -> Point {
        Point::new(self.centre_x + (p.x - self.centre_x) * self.scale(p.y), p.y)
    }

    /// The map's derivative along `d` at `p` (the chain rule).
    fn derivative(&self, p: Point, d: Vec2) -> Vec2 {
        Vec2::new(
            self.scale(p.y) * d.x + (p.x - self.centre_x) * self.scale_slope(p.y) * d.y,
            d.y,
        )
    }

    /// `path` (frame mm) tapered, each curve within `accuracy` mm of the exact map. Nothing
    /// changes for a taper of 0.
    pub fn path(&self, path: &BezPath, accuracy: f64) -> BezPath {
        if self.amount == 0.0 {
            return path.clone();
        }
        let mut out = BezPath::new();
        let mut current = Point::ZERO;
        let mut start = Point::ZERO;
        for element in path.elements() {
            match *element {
                PathEl::MoveTo(p) => {
                    out.move_to(self.point(p));
                    current = p;
                    start = p;
                }
                PathEl::LineTo(p) => {
                    self.line(&mut out, current, p);
                    current = p;
                }
                PathEl::QuadTo(c, p) => {
                    self.cubic(&mut out, QuadBez::new(current, c, p).raise(), accuracy);
                    current = p;
                }
                PathEl::CurveTo(a, b, p) => {
                    self.cubic(&mut out, CubicBez::new(current, a, b, p), accuracy);
                    current = p;
                }
                PathEl::ClosePath => {
                    // The closing line is mapped like any other: it may become a curve.
                    if current.distance(start) > SAME_MM {
                        self.line(&mut out, current, start);
                    }
                    out.close_path();
                    current = start;
                }
            }
        }
        out
    }

    /// The parameters in (0, 1) where `y(u)` of a curve with y polynomial `c0 + c1 u + c2 u² +
    /// c3 u³` crosses the clamp height, sorted.
    fn clamp_crossings(&self, c: [f64; 4]) -> Vec<f64> {
        let Some(at) = self.clamp_y() else {
            return Vec::new();
        };
        let mut roots: Vec<f64> = solve_cubic(c[0] - at, c[1], c[2], c[3])
            .into_iter()
            .filter(|u| *u > 1e-9 && *u < 1.0 - 1e-9)
            .collect();
        roots.sort_by(f64::total_cmp);
        roots
    }

    /// A line from `a` to `b`: split at the clamp, each piece a line where the map is affine on
    /// it (horizontal, vertical, or clamped), else its exact quadratic as a cubic.
    fn line(&self, out: &mut BezPath, a: Point, b: Point) {
        let mut cuts = self.clamp_crossings([a.y, b.y - a.y, 0.0, 0.0]);
        cuts.push(1.0);
        let mut from = 0.0;
        for to in cuts {
            let (p, q) = (a.lerp(b, from), a.lerp(b, to));
            let middle = p.midpoint(q);
            let affine = (p.x - q.x).abs() < SAME_MM
                || (p.y - q.y).abs() < SAME_MM
                || self.clamped(middle.y);
            if affine {
                out.line_to(self.point(q));
            } else {
                // x' = x_c + (x − x_c)·s(y) is a product of two linear functions of u: a
                // quadratic, whose middle control point follows from the start tangent.
                let start = self.point(p);
                let control = start + self.derivative(p, q - p) / 2.0;
                let quad = QuadBez::new(start, control, self.point(q)).raise();
                out.curve_to(quad.p1, quad.p2, quad.p3);
            }
            from = to;
        }
    }

    /// A cubic: split at the clamp, each piece mapped exactly where the map is affine on it,
    /// else refitted.
    fn cubic(&self, out: &mut BezPath, curve: CubicBez, accuracy: f64) {
        let (y0, y1, y2, y3) = (curve.p0.y, curve.p1.y, curve.p2.y, curve.p3.y);
        let polynomial = [
            y0,
            3.0 * (y1 - y0),
            3.0 * (y2 - 2.0 * y1 + y0),
            y3 - 3.0 * y2 + 3.0 * y1 - y0,
        ];
        let mut cuts = self.clamp_crossings(polynomial);
        cuts.push(1.0);
        let mut from = 0.0;
        for to in cuts {
            let piece = curve.subsegment(from..to);
            self.cubic_piece(out, piece, accuracy);
            from = to;
        }
    }

    fn cubic_piece(&self, out: &mut BezPath, piece: CubicBez, accuracy: f64) {
        let points = [piece.p0, piece.p1, piece.p2, piece.p3];
        let same_x = points.iter().all(|p| (p.x - piece.p0.x).abs() < SAME_MM);
        let same_y = points.iter().all(|p| (p.y - piece.p0.y).abs() < SAME_MM);
        let clamped = self.clamped(piece.eval(0.5).y);
        if same_x || same_y || clamped {
            // Affine on this piece: the mapped control points are the mapped curve. A clamped
            // piece is scaled by one constant.
            out.curve_to(
                self.point(piece.p1),
                self.point(piece.p2),
                self.point(piece.p3),
            );
            return;
        }
        let fitted = fit_to_bezpath(
            &TaperedCubic {
                taper: *self,
                curve: piece,
            },
            accuracy,
        );
        for element in fitted.elements() {
            match *element {
                PathEl::MoveTo(_) | PathEl::ClosePath => {}
                other => out.push(other),
            }
        }
    }
}

/// One cubic under the taper, as the curve fitter samples it.
struct TaperedCubic {
    taper: Taper,
    curve: CubicBez,
}

impl ParamCurveFit for TaperedCubic {
    fn sample_pt_tangent(&self, t: f64, sign: f64) -> CurveFitSample {
        let (p, tangent) = self.sample_pt_deriv(t);
        if tangent.hypot2() > 1e-18 {
            return CurveFitSample { p, tangent };
        }
        // A zero-length handle: the direction from a nearby point on the side asked for.
        let step = 1e-6_f64.copysign(if sign == 0.0 { 1.0 } else { sign });
        let near = (t + step).clamp(0.0, 1.0);
        let q = self.taper.point(self.curve.eval(near));
        let tangent = if near > t { q - p } else { p - q };
        CurveFitSample { p, tangent }
    }

    fn sample_pt_deriv(&self, t: f64) -> (Point, Vec2) {
        let p = self.curve.eval(t);
        let d = self.curve.deriv().eval(t).to_vec2();
        (self.taper.point(p), self.taper.derivative(p, d))
    }

    fn break_cusp(&self, _: std::ops::Range<f64>) -> Option<f64> {
        // The pieces are split at the clamp; inside a piece the map is smooth.
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const TAPER: Taper = Taper {
        centre_x: 5.0,
        amount: 0.2,
        size: 20.0,
    };

    #[test]
    fn the_scale_at_the_baseline_middle_and_top() {
        assert!((TAPER.point(Point::new(0.0, 0.0)).x - 1.0).abs() < 1e-12);
        assert!((TAPER.point(Point::new(0.0, -10.0)).x).abs() < 1e-12);
        assert!((TAPER.point(Point::new(0.0, -20.0)).x + 1.0).abs() < 1e-12);
        let clamp = TAPER.clamp_y().unwrap();
        assert!((TAPER.raw_scale(clamp) - MIN_SCALE).abs() < 1e-12);
    }

    #[test]
    fn a_slanted_line_becomes_its_exact_quadratic() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((4.0, -20.0));
        let out = TAPER.path(&path, 0.01);
        let PathEl::CurveTo(a, b, end) = out.elements()[1] else {
            panic!("a curve");
        };
        let curve = CubicBez::new(TAPER.point(Point::ZERO), a, b, end);
        for i in 0..=10 {
            let u = f64::from(i) / 10.0;
            let exact = TAPER.point(Point::new(4.0 * u, -20.0 * u));
            assert!(curve.eval(u).distance(exact) < 1e-9);
        }
    }

    #[test]
    fn a_closing_line_is_mapped_too() {
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((10.0, 0.0));
        path.line_to((4.0, -20.0));
        path.close_path();
        let out = TAPER.path(&path, 0.01);
        assert_eq!(
            out.elements()
                .iter()
                .filter(|el| matches!(el, PathEl::CurveTo(..)))
                .count(),
            2
        );
    }

    /// Criterion 17: below the clamp height the scale stays 0.05; a line across it is split
    /// there and both pieces end where the exact map puts them.
    #[test]
    fn the_scale_stops_at_five_percent() {
        let taper = Taper {
            amount: 0.5,
            ..TAPER
        };
        let clamp = taper.clamp_y().unwrap();
        assert!((clamp - 9.0).abs() < 1e-12, "0.45 h below the baseline");
        assert!((taper.scale(15.0) - MIN_SCALE).abs() < 1e-12);
        let mut path = BezPath::new();
        path.move_to((0.0, 0.0));
        path.line_to((2.0, 15.0));
        let out = taper.path(&path, 0.01);
        let ends: Vec<Point> = out
            .elements()
            .iter()
            .filter_map(PathEl::end_point)
            .collect();
        assert_eq!(ends.len(), 3, "start, the clamp, the end");
        assert!((ends[1].y - 9.0).abs() < 1e-9);
        assert!(ends[2].distance(taper.point(Point::new(2.0, 15.0))) < 1e-12);
        assert!(
            matches!(out.elements()[2], PathEl::LineTo(_)),
            "clamped: affine"
        );
    }
}
