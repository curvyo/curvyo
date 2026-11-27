//! Closed outlines of the shapes a code is drawn from: rectangles with round corners, circles and
//! diamonds in module units, mapped to millimetres around the symbol's centre. Every outline runs
//! clockwise on the screen (y down), the winding of ink under the non-zero rule.

use curvyo_document_core::Point;
use curvyo_extension_core::{ClosedPath, Cubic};

/// The cubic constant of a quarter circle.
const K: f64 = 0.552_284_749_8;
/// Two points closer than this (module units) are the same point.
const SAME: f64 = 1e-9;

/// Module units to millimetres: the origin at the centre of the symbol.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frame {
    /// Half the symbol width in modules.
    pub(crate) half: f64,
    /// One module in millimetres.
    pub(crate) module: f64,
}

impl Frame {
    pub(crate) fn pt(self, x: f64, y: f64) -> Point {
        Point::new((x - self.half) * self.module, (y - self.half) * self.module)
    }
}

/// Builds one outline point by point.
struct Pen {
    frame: Frame,
    start: Point,
    at: (f64, f64),
    segments: Vec<Cubic>,
}

impl Pen {
    fn new(frame: Frame, x: f64, y: f64) -> Self {
        Self {
            frame,
            start: frame.pt(x, y),
            at: (x, y),
            segments: Vec::with_capacity(8),
        }
    }

    /// A line to `(x, y)`; none when already there.
    fn line(&mut self, x: f64, y: f64) {
        if (self.at.0 - x).abs() < SAME && (self.at.1 - y).abs() < SAME {
            return;
        }
        let from = self.frame.pt(self.at.0, self.at.1);
        let to = self.frame.pt(x, y);
        self.segments.push(Cubic {
            c1: from,
            c2: to,
            to,
        });
        self.at = (x, y);
    }

    fn curve(&mut self, c1: (f64, f64), c2: (f64, f64), to: (f64, f64)) {
        self.segments.push(Cubic {
            c1: self.frame.pt(c1.0, c1.1),
            c2: self.frame.pt(c2.0, c2.1),
            to: self.frame.pt(to.0, to.1),
        });
        self.at = to;
    }

    fn finish(self) -> ClosedPath {
        ClosedPath {
            start: self.start,
            segments: self.segments,
        }
    }
}

/// The rectangle `x0..x1` by `y0..y1` with corners of radius `r` (held to half the shorter side).
pub(crate) fn rounded_rect(
    frame: Frame,
    (x0, y0): (f64, f64),
    (x1, y1): (f64, f64),
    r: f64,
) -> ClosedPath {
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
    let k = K * r;
    let mut pen = Pen::new(frame, x0 + r, y0);
    pen.line(x1 - r, y0);
    if r > SAME {
        pen.curve((x1 - r + k, y0), (x1, y0 + r - k), (x1, y0 + r));
    }
    pen.line(x1, y1 - r);
    if r > SAME {
        pen.curve((x1, y1 - r + k), (x1 - r + k, y1), (x1 - r, y1));
    }
    pen.line(x0 + r, y1);
    if r > SAME {
        pen.curve((x0 + r - k, y1), (x0, y1 - r + k), (x0, y1 - r));
    }
    pen.line(x0, y0 + r);
    if r > SAME {
        pen.curve((x0, y0 + r - k), (x0 + r - k, y0), (x0 + r, y0));
    }
    pen.finish()
}

/// The circle of radius `r` around `(cx, cy)`, four cubics.
pub(crate) fn circle(frame: Frame, (cx, cy): (f64, f64), r: f64) -> ClosedPath {
    let k = K * r;
    let mut pen = Pen::new(frame, cx + r, cy);
    pen.curve((cx + r, cy + k), (cx + k, cy + r), (cx, cy + r));
    pen.curve((cx - k, cy + r), (cx - r, cy + k), (cx - r, cy));
    pen.curve((cx - r, cy - k), (cx - k, cy - r), (cx, cy - r));
    pen.curve((cx + k, cy - r), (cx + r, cy - k), (cx + r, cy));
    pen.finish()
}

/// The square turned by 45 degrees around `(cx, cy)` with half diagonal `h`.
pub(crate) fn diamond(frame: Frame, (cx, cy): (f64, f64), h: f64) -> ClosedPath {
    let mut pen = Pen::new(frame, cx, cy - h);
    pen.line(cx + h, cy);
    pen.line(cx, cy + h);
    pen.line(cx - h, cy);
    pen.line(cx, cy - h);
    pen.finish()
}

/// `path` run the other way round: the same shape, the opposite winding.
pub(crate) fn reversed(path: &ClosedPath) -> ClosedPath {
    let mut points = vec![path.start];
    points.extend(path.segments.iter().map(|cubic| cubic.to));
    let segments = path
        .segments
        .iter()
        .enumerate()
        .rev()
        .map(|(at, cubic)| Cubic {
            c1: cubic.c2,
            c2: cubic.c1,
            to: points[at],
        })
        .collect();
    ClosedPath {
        start: path.start,
        segments,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            half: 0.0,
            module: 1.0,
        }
    }

    /// The shoelace area of the polyline of the end points (exact for straight outlines).
    fn area(path: &ClosedPath) -> f64 {
        let mut points = vec![path.start];
        points.extend(path.segments.iter().map(|c| c.to));
        points
            .windows(2)
            .map(|w| w[0].x * w[1].y - w[1].x * w[0].y)
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn a_square_is_closed_clockwise_and_reversing_flips_the_winding() {
        let square = rounded_rect(frame(), (0.0, 0.0), (2.0, 2.0), 0.0);
        assert_eq!(square.segments.len(), 4);
        assert!(
            (area(&square) - 4.0).abs() < 1e-12,
            "positive is clockwise on screen"
        );
        let back = reversed(&square);
        assert!((area(&back) + 4.0).abs() < 1e-12);
        assert_eq!(back.segments.last().unwrap().to, back.start);
        assert_eq!(reversed(&back), square);
    }

    #[test]
    fn a_rounded_rectangle_at_half_the_side_is_a_circle() {
        let round = rounded_rect(frame(), (0.0, 0.0), (2.0, 2.0), 5.0);
        assert_eq!(round.segments.len(), 4, "the straight runs vanish");
        let last = round.segments.last().unwrap().to;
        assert!((last.x - round.start.x).abs() < 1e-12 && (last.y - round.start.y).abs() < 1e-12);
    }

    #[test]
    fn a_circle_and_a_diamond_close_on_their_start() {
        for path in [
            circle(frame(), (1.0, 1.0), 0.5),
            diamond(frame(), (1.0, 1.0), 0.5),
        ] {
            let end = path.segments.last().unwrap().to;
            assert!((end.x - path.start.x).abs() < 1e-12);
            assert!((end.y - path.start.y).abs() < 1e-12);
        }
    }
}
