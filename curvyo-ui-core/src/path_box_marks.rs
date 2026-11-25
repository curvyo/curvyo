//! The overlay of a path box as strokes (ADR 0083 §11, UX notes `0149` §3): a cross at every
//! corner, a striped band on every bent stretch, a tick at the seam and the piece numbers on
//! discs, each a stripe and disc in page mm.

use curvyo_document_core::Point;
use curvyo_extension_core::{Mark, Marks};

/// A piece shorter than this many pixels on screen leaves its badge out (spec criterion 12).
const MIN_BADGE_PX: f64 = 24.0;
/// The height of a digit, the distance of a badge from its stretch and the cross arm, in pixels.
const DIGIT_PX: f64 = 9.0;
/// The stripes: a band this wide, a stripe every this many pixels.
const BAND_PX: f64 = 6.0;
const STRIPE_EVERY_PX: f64 = 4.0;
/// The badge disc and the sharp corner's ring, in pixels across.
const DISC_PX: f64 = 16.0;
const RING_PX: f64 = 12.0;
const BADGE_OFFSET_PX: f64 = 12.0;
const ARM_PX: f64 = 4.0;
/// The seam tick, in pixels long.
const TICK_PX: f64 = 10.0;

/// The look of a piece of the overlay; the shell turns it into a colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Corner crosses and badge discs.
    Accent,
    /// The stripes of a bent span.
    Stripe,
    /// A very sharp corner or a bend too tight for the wood.
    Warning,
    /// A badge digit, on its disc.
    OnDisc,
}

/// The overlay in page mm: strokes, filled discs and rings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overlay {
    /// Strokes between two points.
    pub lines: Vec<(Point, Point, Tone)>,
    /// Filled discs: centre, diameter.
    pub discs: Vec<(Point, f64, Tone)>,
    /// Rings: centre, diameter.
    pub rings: Vec<(Point, f64, Tone)>,
}

/// The strokes of each digit in a unit box (x right, y down).
const DIGITS: [&[&[(f64, f64)]]; 10] = [
    &[&[(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0), (0.0, 0.0)]],
    &[&[(0.2, 0.2), (0.55, 0.0), (0.55, 1.0)]],
    &[&[
        (0.0, 0.25),
        (0.25, 0.0),
        (0.75, 0.0),
        (1.0, 0.25),
        (1.0, 0.45),
        (0.0, 1.0),
        (1.0, 1.0),
    ]],
    &[&[
        (0.0, 0.0),
        (1.0, 0.0),
        (0.4, 0.42),
        (0.75, 0.42),
        (1.0, 0.7),
        (0.75, 1.0),
        (0.0, 1.0),
    ]],
    &[&[(0.75, 1.0), (0.75, 0.0), (0.0, 0.7), (1.0, 0.7)]],
    &[&[
        (1.0, 0.0),
        (0.0, 0.0),
        (0.0, 0.45),
        (0.8, 0.45),
        (1.0, 0.7),
        (0.8, 1.0),
        (0.0, 1.0),
    ]],
    &[&[
        (1.0, 0.0),
        (0.2, 0.0),
        (0.0, 0.3),
        (0.0, 1.0),
        (1.0, 1.0),
        (1.0, 0.5),
        (0.0, 0.5),
    ]],
    &[&[(0.0, 0.0), (1.0, 0.0), (0.4, 1.0)]],
    &[&[
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
        (0.0, 0.0),
        (0.0, 0.5),
        (1.0, 0.5),
    ]],
    &[&[
        (1.0, 0.5),
        (0.0, 0.5),
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (0.2, 1.0),
    ]],
];

fn add(a: Point, b: Point, k: f64) -> Point {
    Point::new(a.x + b.x * k, a.y + b.y * k)
}

/// A point of the cubic `p0..p3` at `t` and the unit direction of travel there.
fn cubic_at(p: [Point; 4], t: f64) -> (Point, (f64, f64)) {
    let u = 1.0 - t;
    let at = |f: fn(Point) -> f64| {
        u * u * u * f(p[0])
            + 3.0 * u * u * t * f(p[1])
            + 3.0 * u * t * t * f(p[2])
            + t * t * t * f(p[3])
    };
    let d = |f: fn(Point) -> f64| {
        3.0 * u * u * (f(p[1]) - f(p[0]))
            + 6.0 * u * t * (f(p[2]) - f(p[1]))
            + 3.0 * t * t * (f(p[3]) - f(p[2]))
    };
    let (dx, dy) = (d(|q| q.x), d(|q| q.y));
    let length = dx.hypot(dy).max(1e-12);
    (
        Point::new(at(|q| q.x), at(|q| q.y)),
        (dx / length, dy / length),
    )
}

/// The stripes of a bent stretch: 45 degrees to the line, one every 4 px, 6 px inside it.
fn stripes(
    start: Point,
    segments: &[curvyo_extension_core::Cubic],
    pixel: f64,
    tone: Tone,
    map: &dyn Fn(Point) -> Point,
    out: &mut Overlay,
) {
    let mut from = start;
    for cubic in segments {
        let p = [from, cubic.c1, cubic.c2, cubic.to];
        let hull: f64 = p
            .windows(2)
            .map(|w| (w[1].x - w[0].x).hypot(w[1].y - w[0].y))
            .sum();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let steps = ((hull / (STRIPE_EVERY_PX * pixel)).ceil() as usize).max(1);
        for k in 0..=steps {
            #[allow(clippy::cast_precision_loss)]
            let (at, tangent) = cubic_at(p, k as f64 / steps as f64);
            // Inside the clockwise walk is its right-hand side.
            let inward = (-tangent.1, tangent.0);
            let reach = BAND_PX * pixel;
            let end = Point::new(
                at.x + (inward.0 + tangent.0) * reach,
                at.y + (inward.1 + tangent.1) * reach,
            );
            out.lines.push((map(at), map(end), tone));
        }
        from = cubic.to;
    }
}

/// A badge: a blue disc with `number` on it in white.
fn badge(number: u32, at: Point, pixel: f64, out: &mut Overlay) {
    out.discs.push((at, DISC_PX * pixel, Tone::Accent));
    let height = DIGIT_PX * pixel;
    let width = height * 0.6;
    let count = number.to_string().len();
    #[allow(clippy::cast_precision_loss)]
    let total = width + (count - 1) as f64 * width * 1.4;
    for (place, digit) in number.to_string().bytes().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let left = at.x - total / 2.0 + place as f64 * width * 1.4;
        for stroke in DIGITS[usize::from(digit - b'0')] {
            let points: Vec<Point> = stroke
                .iter()
                .map(|&(x, y)| Point::new(left + x * width, at.y - height / 2.0 + y * height))
                .collect();
            for pair in points.windows(2) {
                out.lines.push((pair[0], pair[1], Tone::OnDisc));
            }
        }
    }
}

/// Whether a badge at `at` would overlap one already placed.
fn collides(placed: &[Point], at: Point, pixel: f64) -> bool {
    placed
        .iter()
        .any(|p| (p.x - at.x).hypot(p.y - at.y) < (DISC_PX + 2.0) * pixel)
}

fn corner(
    at: Point,
    bisector: curvyo_document_core::Vec2,
    sharp: bool,
    pixel: f64,
    map: &dyn Fn(Point) -> Point,
    out: &mut Overlay,
) {
    let tone = if sharp { Tone::Warning } else { Tone::Accent };
    let arm = ARM_PX * pixel;
    let r = std::f64::consts::FRAC_1_SQRT_2;
    for sign in [1.0, -1.0] {
        let d = (
            bisector.x * r - sign * bisector.y * r,
            sign * bisector.x * r + bisector.y * r,
        );
        out.lines.push((
            map(add(at, Point::new(d.0, d.1), -arm)),
            map(add(at, Point::new(d.0, d.1), arm)),
            tone,
        ));
    }
    if sharp {
        out.rings.push((map(at), RING_PX * pixel, Tone::Warning));
    }
}

/// A tick across the footprint line at the seam, 10 px long, centred on it.
fn seam(
    at: Point,
    inward: curvyo_document_core::Vec2,
    pixel: f64,
    map: &dyn Fn(Point) -> Point,
    out: &mut Overlay,
) {
    let half = TICK_PX * pixel / 2.0;
    let toward = Point::new(inward.x, inward.y);
    out.lines.push((
        map(add(at, toward, -half)),
        map(add(at, toward, half)),
        Tone::Accent,
    ));
}

/// The badges on the wall pieces of the layout.
fn part_badges(marks: &[Mark], map: &dyn Fn(Point) -> Point, pixel: f64, out: &mut Overlay) {
    let mut placed: Vec<Point> = Vec::new();
    for mark in marks {
        if let Mark::Piece { number, at, .. } = mark {
            let place = map(*at);
            if !collides(&placed, place, pixel) {
                placed.push(place);
                badge(*number, place, pixel, out);
            }
        }
    }
}

/// The overlay of `marks`. `on_base` maps the marks on the footprint to the page, `on_part` the
/// marks on the wall pieces; `pixel` is the size of a screen pixel in page mm; `badges` is off
/// for the outline the pointer hovers.
#[must_use]
pub fn overlay(
    marks: &Marks,
    on_base: &dyn Fn(Point) -> Point,
    on_part: &dyn Fn(Point) -> Point,
    pixel: f64,
    badges: bool,
) -> Overlay {
    let mut out = Overlay::default();
    let mut placed: Vec<Point> = Vec::new();
    for mark in &marks.on_footprint {
        match mark {
            Mark::Corner {
                at,
                bisector,
                sharp,
            } => corner(*at, *bisector, *sharp, pixel, on_base, &mut out),
            Mark::Bend {
                start,
                segments,
                tight,
            } => {
                let tone = if *tight { Tone::Warning } else { Tone::Stripe };
                stripes(*start, segments, pixel, tone, on_base, &mut out);
            }
            Mark::Seam { at, inward } => seam(*at, *inward, pixel, on_base, &mut out),
            Mark::Piece {
                number,
                at,
                inward,
                length,
            } if badges => {
                let place = on_base(add(
                    *at,
                    Point::new(inward.x, inward.y),
                    BADGE_OFFSET_PX * pixel,
                ));
                if length.as_mm() >= MIN_BADGE_PX * pixel && !collides(&placed, place, pixel) {
                    placed.push(place);
                    badge(*number, place, pixel, &mut out);
                }
            }
            Mark::Piece { .. } => {}
        }
    }
    if badges {
        part_badges(&marks.on_parts, on_part, pixel, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Length, Vec2};

    fn corner_mark(sharp: bool) -> Mark {
        Mark::Corner {
            at: Point::new(0.0, 0.0),
            bisector: Vec2::new(0.0, 1.0),
            sharp,
        }
    }

    #[test]
    fn a_cross_has_extent_along_the_bisector_and_a_sharp_one_a_warning_ring() {
        let marks = Marks {
            footprint: Vec2::ZERO,
            on_footprint: vec![corner_mark(false), corner_mark(true)],
            on_parts: Vec::new(),
        };
        let id = |p: Point| p;
        let o = overlay(&marks, &id, &id, 0.5, true);
        let ordinary: Vec<_> = o.lines.iter().filter(|l| l.2 == Tone::Accent).collect();
        assert_eq!(ordinary.len(), 2);
        for (a, b, _) in &ordinary {
            assert!(
                (a.x - b.x).hypot(a.y - b.y) > 1.0,
                "an arm of 8 px at 0.5 mm per px is 4 mm"
            );
        }
        assert_eq!(o.lines.iter().filter(|l| l.2 == Tone::Warning).count(), 2);
        assert_eq!(o.rings.len(), 1);
        assert!((o.rings[0].1 - 6.0).abs() < 1e-9, "12 px");
    }

    #[test]
    fn badges_are_16_px_discs_skipped_when_short_or_overlapping_and_absent_on_hover() {
        let piece = |number, x, length| Mark::Piece {
            number,
            at: Point::new(x, 0.0),
            inward: Vec2::new(0.0, 1.0),
            length: Length::from_mm(length),
        };
        let marks = Marks {
            footprint: Vec2::ZERO,
            on_footprint: vec![
                piece(1, 0.0, 40.0),
                piece(2, 1.0, 40.0),
                piece(3, 50.0, 2.0),
            ],
            on_parts: Vec::new(),
        };
        let id = |p: Point| p;
        let o = overlay(&marks, &id, &id, 0.5, true);
        assert_eq!(o.discs.len(), 1, "2 overlaps 1, 3 is shorter than 24 px");
        assert!((o.discs[0].1 - 8.0).abs() < 1e-9, "16 px at 0.5 mm per px");
        assert_eq!(overlay(&marks, &id, &id, 0.5, false).discs, []);
    }

    #[test]
    fn a_bent_span_gets_stripes_a_tight_one_in_the_warning_tone() {
        let cubic = curvyo_extension_core::Cubic {
            c1: Point::new(3.0, 0.0),
            c2: Point::new(7.0, 0.0),
            to: Point::new(10.0, 0.0),
        };
        let bend = |tight| Mark::Bend {
            start: Point::new(0.0, 0.0),
            segments: vec![cubic],
            tight,
        };
        let marks = Marks {
            footprint: Vec2::ZERO,
            on_footprint: vec![bend(false), bend(true)],
            on_parts: Vec::new(),
        };
        let id = |p: Point| p;
        let o = overlay(&marks, &id, &id, 1.0, true);
        assert!(o.lines.iter().filter(|l| l.2 == Tone::Stripe).count() >= 3);
        assert!(o.lines.iter().any(|l| l.2 == Tone::Warning));
    }
}
