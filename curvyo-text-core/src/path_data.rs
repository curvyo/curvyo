//! Parses SVG path data into a cubic path (`specs/0076-single-line-fonts` criterion 16).
//!
//! Every command of SVG 1.1 (absolute and relative M, L, H, V, C, S, Q, T, A, Z), implicit
//! repetition and packed arc flags. Quadratics are elevated to cubics exactly, arcs become cubics
//! within the caller's tolerance, and a sub-path is closed only by `Z`. A syntax error ends the
//! path where it occurs (SVG's error rule); a number that is not finite or beyond
//! [`MAX_SVG_COORDINATE`], or more than `max_points` points, refuse the whole path.

use kurbo::{Arc, BezPath, PathEl, Point, SvgArc, Vec2};

use crate::limits::MAX_SVG_COORDINATE;

/// Why a path was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathDataError {
    /// A number is not finite or larger than [`MAX_SVG_COORDINATE`].
    BadNumber,
    /// The path has more points than allowed.
    TooManyPoints,
}

/// The reader's position in the data.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    fn skip_separators(&mut self) {
        while let Some(b) = self.bytes.get(self.at) {
            if b.is_ascii_whitespace() || *b == b',' {
                self.at += 1;
            } else {
                break;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    /// Whether a number follows (after separators).
    fn number_follows(&mut self) -> bool {
        self.skip_separators();
        matches!(self.peek(), Some(b'0'..=b'9' | b'-' | b'+' | b'.'))
    }

    /// The next number; `Ok(None)` on a syntax error.
    fn number(&mut self) -> Result<Option<f64>, PathDataError> {
        self.skip_separators();
        let start = self.at;
        let digits = |cursor: &mut Self| {
            let from = cursor.at;
            while matches!(cursor.peek(), Some(b'0'..=b'9')) {
                cursor.at += 1;
            }
            cursor.at > from
        };
        if matches!(self.peek(), Some(b'-' | b'+')) {
            self.at += 1;
        }
        let whole = digits(self);
        let mut fraction = false;
        if self.peek() == Some(b'.') {
            self.at += 1;
            fraction = digits(self);
        }
        if !whole && !fraction {
            self.at = start;
            return Ok(None);
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            let mark = self.at;
            self.at += 1;
            if matches!(self.peek(), Some(b'-' | b'+')) {
                self.at += 1;
            }
            if !digits(self) {
                self.at = mark;
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.at]).unwrap_or("x");
        let Ok(value) = text.parse::<f64>() else {
            return Ok(None);
        };
        if !value.is_finite() || value.abs() > MAX_SVG_COORDINATE {
            return Err(PathDataError::BadNumber);
        }
        Ok(Some(value))
    }

    /// An arc flag: one `0` or `1`, which may be packed against the next number.
    fn flag(&mut self) -> Option<bool> {
        self.skip_separators();
        let flag = match self.peek()? {
            b'0' => false,
            b'1' => true,
            _ => return None,
        };
        self.at += 1;
        Some(flag)
    }
}

/// Builds the path, counting points.
struct Builder {
    path: BezPath,
    points: usize,
    max_points: usize,
    current: Point,
    start: Point,
    /// The second control point of the last cubic, for `S`.
    last_cubic: Option<Point>,
    /// The control point of the last quadratic, for `T`.
    last_quad: Option<Point>,
    /// Whether a drawing command needs a `MoveTo` first (start, or after `Z`).
    needs_move: bool,
}

impl Builder {
    fn take(&mut self, points: usize) -> Result<(), PathDataError> {
        self.points += points;
        if self.points > self.max_points {
            return Err(PathDataError::TooManyPoints);
        }
        Ok(())
    }

    fn move_to(&mut self, p: Point) -> Result<(), PathDataError> {
        self.take(1)?;
        self.path.move_to(p);
        self.current = p;
        self.start = p;
        self.needs_move = false;
        Ok(())
    }

    fn ensure_moved(&mut self) -> Result<(), PathDataError> {
        if self.needs_move {
            self.move_to(self.current)?;
        }
        Ok(())
    }

    fn line_to(&mut self, p: Point) -> Result<(), PathDataError> {
        self.ensure_moved()?;
        self.take(1)?;
        self.path.line_to(p);
        self.current = p;
        Ok(())
    }

    fn cubic_to(&mut self, c1: Point, c2: Point, p: Point) -> Result<(), PathDataError> {
        self.ensure_moved()?;
        self.take(3)?;
        self.path.curve_to(c1, c2, p);
        self.current = p;
        self.last_cubic = Some(c2);
        Ok(())
    }

    fn quad_to(&mut self, control: Point, p: Point) -> Result<(), PathDataError> {
        let from = self.current;
        // Exact degree elevation: the cubic with these handles is the same curve.
        let c1 = from + (control - from) * (2.0 / 3.0);
        let c2 = p + (control - p) * (2.0 / 3.0);
        self.cubic_to(c1, c2, p)?;
        self.last_cubic = None;
        self.last_quad = Some(control);
        Ok(())
    }

    fn arc_to(&mut self, arc: &SvgArc, tolerance: f64) -> Result<(), PathDataError> {
        match Arc::from_svg_arc(arc) {
            None => self.line_to(arc.to),
            Some(arc_shape) => {
                for element in arc_shape.append_iter(tolerance) {
                    if let PathEl::CurveTo(c1, c2, p) = element {
                        self.cubic_to(c1, c2, p)?;
                    }
                }
                // The last cubic ends on the arc's end within rounding; the next command starts
                // from the stated end.
                self.current = arc.to;
                self.last_cubic = None;
                Ok(())
            }
        }
    }

    fn close(&mut self) {
        if !self.needs_move {
            self.path.close_path();
        }
        self.current = self.start;
        self.needs_move = true;
    }
}

/// Parses `data` (font units, as written) into cubics. Arcs are split within `tolerance` (font
/// units).
///
/// # Errors
/// [`PathDataError`] for a number that is not finite or too large, or more than `max_points`
/// points.
pub(crate) fn parse_path_data(
    data: &str,
    tolerance: f64,
    max_points: usize,
) -> Result<BezPath, PathDataError> {
    let mut cursor = Cursor {
        bytes: data.as_bytes(),
        at: 0,
    };
    let mut b = Builder {
        path: BezPath::new(),
        points: 0,
        max_points,
        current: Point::ZERO,
        start: Point::ZERO,
        last_cubic: None,
        last_quad: None,
        needs_move: true,
    };
    let mut command: Option<u8> = None;
    loop {
        cursor.skip_separators();
        let Some(next) = cursor.peek() else {
            break;
        };
        if next.is_ascii_alphabetic() {
            cursor.at += 1;
            command = Some(next);
            if matches!(next, b'Z' | b'z') {
                b.close();
                b.last_cubic = None;
                b.last_quad = None;
                continue;
            }
        } else if command.is_none_or(|c| matches!(c, b'Z' | b'z')) {
            break;
        }
        let Some(letter) = command else { break };
        if !step(letter, &mut cursor, &mut b, tolerance)? {
            break;
        }
        // An implicit repetition of M is a L (m: l).
        command = match letter {
            b'M' => Some(b'L'),
            b'm' => Some(b'l'),
            other => Some(other),
        };
        if !cursor.number_follows() {
            // The next token must be a command letter.
            let more = cursor.peek().is_some_and(|c| c.is_ascii_alphabetic());
            if !more {
                break;
            }
        }
    }
    Ok(b.path)
}

/// Reads `n` numbers; `None` on a syntax error.
fn numbers<const N: usize>(cursor: &mut Cursor<'_>) -> Result<Option<[f64; N]>, PathDataError> {
    let mut out = [0.0; N];
    for slot in &mut out {
        let Some(value) = cursor.number()? else {
            return Ok(None);
        };
        *slot = value;
    }
    Ok(Some(out))
}

/// One command's arguments, drawn; false on a syntax error (the path ends here).
fn step(
    letter: u8,
    cursor: &mut Cursor<'_>,
    b: &mut Builder,
    tolerance: f64,
) -> Result<bool, PathDataError> {
    let relative = letter.is_ascii_lowercase();
    let origin = if relative {
        b.current.to_vec2()
    } else {
        Vec2::ZERO
    };
    let at = |x: f64, y: f64| Point::new(x, y) + origin;
    let (cubic, quad) = (b.last_cubic.take(), b.last_quad.take());
    match letter.to_ascii_uppercase() {
        b'M' => {
            let Some([x, y]) = numbers(cursor)? else {
                return Ok(false);
            };
            b.move_to(at(x, y))?;
        }
        b'L' => {
            let Some([x, y]) = numbers(cursor)? else {
                return Ok(false);
            };
            b.line_to(at(x, y))?;
        }
        b'H' => {
            let Some([x]) = numbers(cursor)? else {
                return Ok(false);
            };
            let y = b.current.y;
            b.line_to(Point::new(x + origin.x, y))?;
        }
        b'V' => {
            let Some([y]) = numbers(cursor)? else {
                return Ok(false);
            };
            let x = b.current.x;
            b.line_to(Point::new(x, y + origin.y))?;
        }
        b'C' => {
            let Some([x1, y1, x2, y2, x, y]) = numbers(cursor)? else {
                return Ok(false);
            };
            b.cubic_to(at(x1, y1), at(x2, y2), at(x, y))?;
        }
        b'S' => {
            let Some([x2, y2, x, y]) = numbers(cursor)? else {
                return Ok(false);
            };
            let c1 = cubic.map_or(b.current, |c| b.current + (b.current - c));
            b.cubic_to(c1, at(x2, y2), at(x, y))?;
        }
        b'Q' => {
            let Some([x1, y1, x, y]) = numbers(cursor)? else {
                return Ok(false);
            };
            b.quad_to(at(x1, y1), at(x, y))?;
        }
        b'T' => {
            let Some([x, y]) = numbers(cursor)? else {
                return Ok(false);
            };
            let control = quad.map_or(b.current, |c| b.current + (b.current - c));
            b.quad_to(control, at(x, y))?;
        }
        b'A' => {
            let Some([rx, ry, rotation]) = numbers(cursor)? else {
                return Ok(false);
            };
            let (Some(large_arc), Some(sweep)) = (cursor.flag(), cursor.flag()) else {
                return Ok(false);
            };
            let Some([x, y]) = numbers(cursor)? else {
                return Ok(false);
            };
            b.ensure_moved()?;
            let arc = SvgArc {
                from: b.current,
                to: at(x, y),
                radii: Vec2::new(rx, ry),
                x_rotation: rotation.to_radians(),
                large_arc,
                sweep,
            };
            b.arc_to(&arc, tolerance)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use kurbo::{ParamCurve, ParamCurveNearest, PathSeg, Shape};

    fn parse(d: &str) -> BezPath {
        parse_path_data(d, 1e-7, 65_535).unwrap()
    }

    fn closes(path: &BezPath) -> usize {
        path.elements()
            .iter()
            .filter(|e| matches!(e, PathEl::ClosePath))
            .count()
    }

    /// Criterion 16: every command, absolute and relative, implicit repetition, no auto-close,
    /// quadratics elevated exactly, arcs within the tolerance.
    #[test]
    fn ac16_every_command_and_no_auto_close() {
        let lines = parse("M0 0 10 0 L10 10 H0 V0 m5 5 l1 0 h1 v1 z");
        assert_eq!(
            closes(&lines),
            1,
            "only the Z closes; the first ends where it began"
        );
        let end_points: Vec<Point> = lines
            .elements()
            .iter()
            .filter_map(PathEl::end_point)
            .collect();
        assert_eq!(
            end_points,
            [(0., 0.), (10., 0.), (10., 10.), (0., 10.), (0., 0.)]
                .iter()
                .chain(&[(5., 5.), (6., 5.), (7., 5.), (7., 6.)])
                .map(|&(x, y)| Point::new(x, y))
                .collect::<Vec<_>>()
        );
        // S reflects the last cubic's handle; T the last quadratic's.
        let smooth = parse("M0 0 C0 10 10 10 10 0 s10 -10 10 0 Q25 10 30 0 t10 0");
        let segs: Vec<PathSeg> = smooth.segments().collect();
        let PathSeg::Cubic(second) = segs[1] else {
            panic!()
        };
        assert_eq!(second.p1, Point::new(10.0, -10.0));
        let PathSeg::Cubic(quad) = segs[2] else {
            panic!()
        };
        // Exact elevation of Q (20,0) (25,10) (30,0): the midpoint is (25, 5).
        assert!((quad.eval(0.5) - Point::new(25.0, 5.0)).hypot() < 1e-12);
        let PathSeg::Cubic(reflected) = segs[3] else {
            panic!()
        };
        assert!((reflected.eval(0.5) - Point::new(35.0, -5.0)).hypot() < 1e-12);
        // A half circle of radius 100, relative, packed flags: every point within the tolerance.
        let arc = parse("M0 0a100 100 0 01200 0");
        for seg in arc.segments() {
            for i in 0..=20 {
                let p = seg.eval(f64::from(i) / 20.0);
                let r = (p - Point::new(100.0, 0.0)).hypot();
                assert!((r - 100.0).abs() < 1e-6, "{r}");
            }
        }
        assert!((arc.bounding_box().y0 + 100.0).abs() < 1e-6);
        let nearest = arc.segments().next().unwrap().nearest(Point::ZERO, 1e-9);
        assert!(nearest.distance_sq < 1e-18);
    }

    /// Criterion 23: a number that is not finite refuses the path; too many points too; a
    /// syntax error ends it.
    #[test]
    fn ac23_hostile_numbers_and_points() {
        assert_eq!(
            parse_path_data("M0 0 L1e999 0", 1e-7, 100),
            Err(PathDataError::BadNumber)
        );
        assert_eq!(
            parse_path_data("M0 0 L1e7 0", 1e-7, 100),
            Err(PathDataError::BadNumber)
        );
        assert_eq!(
            parse_path_data("M0 0 1 1 2 2 3 3", 1e-7, 3),
            Err(PathDataError::TooManyPoints)
        );
        let cut = parse("M0 0 L5 5 L # 7 7");
        assert_eq!(cut.elements().len(), 2);
        assert_eq!(parse("1 2 3").elements(), []);
    }
}
