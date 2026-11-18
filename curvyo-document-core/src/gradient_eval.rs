//! The colour of a gradient fill at a point (ADR 0066 §4).
//!
//! The fractions are placed in the leaf's box and frame, then one integer rule maps the gradient
//! coordinate to a premultiplied colour; `over_white_grey` gives the grey a laser page shows.
//!
//! Geometry uses only `+ - * /` and `sqrt` in `f64`, so native and wasm agree bit for bit.

use crate::affine::Affine;
use crate::gradient_model::{GradientFill, GradientGeometry, GradientStop, GradientStops};
use crate::units::Point;

/// The gradient coordinate is quantised to steps of `1 / T_ONE` before the stop lookup.
const T_ONE: u64 = 65_536;

/// A colour with its channels already multiplied by its alpha (`r <= a` and so on), 8 bits each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PremultipliedRgba8 {
    /// Red times alpha.
    pub r: u8,
    /// Green times alpha.
    pub g: u8,
    /// Blue times alpha.
    pub b: u8,
    /// Alpha, 255 opaque.
    pub a: u8,
}

/// The grey of `colour` composited over white, 0 black to 255 white: the 0.2126 / 0.7152 /
/// 0.0722 weights in 16-bit fixed point on the gamma-encoded values (ADR 0066 §4,
/// `specs/0124-gradient-fill` criterion 10). One function for the canvas and the laser.
#[must_use]
pub fn over_white_grey(colour: PremultipliedRgba8) -> u8 {
    let white = 255 - u32::from(colour.a);
    let (r, g, b) = (
        u32::from(colour.r) + white,
        u32::from(colour.g) + white,
        u32::from(colour.b) + white,
    );
    let grey = (13_933 * r + 46_871 * g + 4_732 * b + 32_768) >> 16;
    // invariant: the weights sum to 65,536 and each channel is at most 255, so `grey <= 255`.
    #[allow(clippy::cast_possible_truncation)]
    let grey = grey as u8;
    grey
}

/// A stop's channels times its alpha, on the scale `0..=255 * 255`; alpha itself is scaled the
/// same way so one rounding rule serves all four.
fn premultiplied_wide(stop: &GradientStop) -> [u64; 4] {
    let a = u64::from(stop.alpha);
    [
        u64::from(stop.colour.r) * a,
        u64::from(stop.colour.g) * a,
        u64::from(stop.colour.b) * a,
        a * 255,
    ]
}

/// A wide channel back to 8 bits, rounding half up.
fn narrow(wide: u64) -> u8 {
    // invariant: `wide <= 65,025`, so the rounded quotient is at most 255.
    #[allow(clippy::cast_possible_truncation)]
    let byte = ((2 * wide + 255) / 510) as u8;
    byte
}

/// `t` on the grid of `1 / T_ONE`, clamped to `[0, 1]` (pad), rounding half up; not a number
/// reads as 0.
fn quantise(t: f64) -> u64 {
    if t.is_nan() || t <= 0.0 {
        0
    } else if t >= 1.0 {
        T_ONE
    } else {
        // invariant: `0 < t < 1`, so the product lies in `(0, T_ONE)` and the cast is exact
        // after the floor.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let q = (t * 65_536.0 + 0.5).floor() as u64;
        q
    }
}

impl GradientStops {
    /// The premultiplied colour at gradient coordinate `t` (clamped to `[0, 1]`): the two
    /// neighbouring stops blended linearly per channel in integers; on a hard edge the later
    /// stop wins, as in SVG.
    #[must_use]
    pub fn colour_at(&self, t: f64) -> PremultipliedRgba8 {
        let tq = quantise(t);
        let stops = self.as_slice();
        let offset_q = |stop: &GradientStop| quantise(stop.offset.get());
        // The first stop above `tq`; the stop before it is the last at or below `tq`.
        let above = stops.partition_point(|stop| offset_q(stop) <= tq);
        let wide = if above == stops.len() {
            // invariant: a stop list is never empty.
            premultiplied_wide(&stops[stops.len() - 1])
        } else {
            // invariant: the first stop sits at 0 and `tq >= 0`, so `above >= 1`.
            let (lo, hi) = (&stops[above - 1], &stops[above]);
            let (lo_q, hi_q) = (offset_q(lo), offset_q(hi));
            let (d, n) = (hi_q - lo_q, tq - lo_q);
            let (pl, ph) = (premultiplied_wide(lo), premultiplied_wide(hi));
            std::array::from_fn(|i| (pl[i] * (d - n) + ph[i] * n + d / 2) / d)
        };
        PremultipliedRgba8 {
            r: narrow(wide[0]),
            g: narrow(wide[1]),
            b: narrow(wide[2]),
            a: narrow(wide[3]),
        }
    }
}

/// A leaf's box in its own frame (ADR 0066 §4, the frame of ADR 0029 §4): `min` and `max` in
/// frame millimetres and the map from the frame into the document (the `rotation` register, the
/// text box, a clone's matrix).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientBox {
    /// The top-left corner in the frame.
    pub min: Point,
    /// The bottom-right corner in the frame.
    pub max: Point,
    /// From the frame into document millimetres.
    pub to_document: Affine,
}

impl GradientBox {
    /// An unrotated box in document millimetres.
    #[must_use]
    pub const fn axis_aligned(min: Point, max: Point) -> Self {
        Self {
            min,
            max,
            to_document: Affine::IDENTITY,
        }
    }

    /// The width and height in millimetres.
    #[must_use]
    pub fn size(&self) -> (f64, f64) {
        (self.max.x - self.min.x, self.max.y - self.min.y)
    }

    /// The frame point at box fractions `(x, y)`.
    #[must_use]
    pub fn frame_point(&self, x: f64, y: f64) -> Point {
        let (w, h) = self.size();
        Point::new(self.min.x + x * w, self.min.y + y * h)
    }
}

/// A gradient placed in the document: the affine map from document millimetres into gradient
/// space and the stops. Linear: `t = u`; radial: `t = sqrt(u² + v²)`, the ellipse folded into the
/// map.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedGradient {
    to_gradient: Affine,
    radial: bool,
    stops: GradientStops,
}

/// The map that sends every point to `u = 1`: a gradient with no length shows its last colour,
/// as SVG paints a zero-length gradient.
const TO_LAST_STOP: Affine = Affine::new(0.0, 0.0, 0.0, 0.0, 1.0, 0.0);

/// The frame-space map of a linear gradient from `s` to `e`: `u` along, `v` across.
fn linear_map(start: Point, end: Point) -> Option<Affine> {
    let (dx, dy) = (end.x - start.x, end.y - start.y);
    let len2 = dx * dx + dy * dy;
    if !len2.is_finite() || len2 <= 0.0 {
        return None;
    }
    let (u_x, u_y) = (dx / len2, dy / len2);
    let (v_x, v_y) = (-dy / len2, dx / len2);
    let u_0 = -(u_x * start.x + u_y * start.y);
    let v_0 = -(v_x * start.x + v_y * start.y);
    Some(Affine::new(u_x, v_x, u_y, v_y, u_0, v_0))
}

/// The two axes of a radial gradient in frame millimetres: from the centre to the radius point,
/// and that turned a quarter and scaled by the ratio.
pub(crate) fn radial_axes(centre: Point, radius: Point, ratio: f64) -> (Point, Point) {
    let (ax, ay) = (radius.x - centre.x, radius.y - centre.y);
    (Point::new(ax, ay), Point::new(-ay * ratio, ax * ratio))
}

impl PlacedGradient {
    /// Places `fill` in `frame`. A gradient of no length, or a frame that cannot be inverted,
    /// shows its last stop everywhere.
    #[must_use]
    pub fn new(fill: &GradientFill, frame: &GradientBox) -> Self {
        let point = |p: crate::gradient_model::BoxPoint| frame.frame_point(p.x.get(), p.y.get());
        let (in_frame, radial) = match fill.geometry {
            GradientGeometry::Linear { start, end } => {
                (linear_map(point(start), point(end)), false)
            }
            GradientGeometry::Radial {
                centre,
                radius,
                ratio,
            } => {
                let c = point(centre);
                let (a, b) = radial_axes(c, point(radius), ratio.get());
                let axes = Affine::new(a.x, a.y, b.x, b.y, c.x, c.y);
                (axes.inverse(), true)
            }
        };
        let to_gradient = in_frame
            .zip(frame.to_document.inverse())
            .map_or(TO_LAST_STOP, |(g, from_document)| g.compose(from_document));
        Self {
            to_gradient,
            radial,
            stops: fill.stops.clone(),
        }
    }

    /// The map from document millimetres into gradient space.
    #[must_use]
    pub const fn to_gradient(&self) -> Affine {
        self.to_gradient
    }

    /// Whether `t` is the length of `(u, v)` rather than `u`.
    #[must_use]
    pub const fn is_radial(&self) -> bool {
        self.radial
    }

    /// The stops.
    #[must_use]
    pub const fn stops(&self) -> &GradientStops {
        &self.stops
    }

    /// The gradient coordinate at document point `p`, before clamping.
    #[must_use]
    pub fn t_at(&self, p: Point) -> f64 {
        let q = self.to_gradient.apply_point(p);
        if self.radial {
            (q.x * q.x + q.y * q.y).sqrt()
        } else {
            q.x
        }
    }

    /// The premultiplied colour at document point `p`.
    #[must_use]
    pub fn colour_at(&self, p: Point) -> PremultipliedRgba8 {
        self.stops.colour_at(self.t_at(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gradient_model::{AxisRatio, BoxPoint, StopOffset};
    use crate::path_model::Color;

    fn stop(r: u8, g: u8, b: u8, alpha: u8, offset: f64) -> GradientStop {
        GradientStop {
            colour: Color { r, g, b },
            alpha,
            offset: StopOffset::new(offset).unwrap(),
        }
    }

    fn fill(geometry: GradientGeometry, stops: Vec<GradientStop>) -> GradientFill {
        GradientFill {
            geometry,
            stops: GradientStops::new(stops).unwrap(),
        }
    }

    fn box_100() -> GradientBox {
        GradientBox::axis_aligned(Point::new(0.0, 0.0), Point::new(100.0, 40.0))
    }

    fn within_one(a: PremultipliedRgba8, b: PremultipliedRgba8) -> bool {
        [(a.r, b.r), (a.g, b.g), (a.b, b.b), (a.a, b.a)]
            .iter()
            .all(|(x, y)| x.abs_diff(*y) <= 1)
    }

    /// Criterion 5, first test: black to white over a box 100 wide reads 50 % grey at x = 50.
    #[test]
    fn black_to_white_reads_half_grey_in_the_middle() {
        let f = fill(
            GradientGeometry::DEFAULT_LINEAR,
            vec![stop(0, 0, 0, 255, 0.0), stop(255, 255, 255, 255, 1.0)],
        );
        let placed = PlacedGradient::new(&f, &box_100());
        let c = placed.colour_at(Point::new(50.0, 7.0));
        let half = PremultipliedRgba8 {
            r: 128,
            g: 128,
            b: 128,
            a: 255,
        };
        assert!(within_one(c, half), "{c:?}");
        let ends = (
            placed.colour_at(Point::new(-5.0, 0.0)),
            placed.colour_at(Point::new(130.0, 0.0)),
        );
        assert_eq!((ends.0.r, ends.1.r), (0, 255), "pad beyond the ends");
    }

    /// Criterion 5, second test: black, red, white at 0, 50, 100 % read pure red at x = 50.
    #[test]
    fn three_stops_read_the_middle_stop_exactly() {
        let f = fill(
            GradientGeometry::DEFAULT_LINEAR,
            vec![
                stop(0, 0, 0, 255, 0.0),
                stop(255, 0, 0, 255, 0.5),
                stop(255, 255, 255, 255, 1.0),
            ],
        );
        let c = PlacedGradient::new(&f, &box_100()).colour_at(Point::new(50.0, 20.0));
        assert_eq!(
            c,
            PremultipliedRgba8 {
                r: 255,
                g: 0,
                b: 0,
                a: 255
            }
        );
    }

    /// Criterion 5, third test: a radial reads one colour on every point of one circle.
    #[test]
    fn a_radial_reads_one_colour_on_a_circle() {
        let f = fill(
            GradientGeometry::DEFAULT_RADIAL,
            vec![stop(10, 200, 30, 255, 0.0), stop(250, 0, 90, 100, 1.0)],
        );
        let frame = box_100();
        let placed = PlacedGradient::new(&f, &frame);
        let centre = Point::new(50.0, 20.0);
        let first = placed.colour_at(Point::new(80.0, 20.0));
        for (x, y) in [
            (0.0, 30.0),
            (-30.0, 0.0),
            (0.0, -30.0),
            (21.213_203_4, 21.213_203_4),
        ] {
            let c = placed.colour_at(Point::new(centre.x + x, centre.y + y));
            assert!(within_one(c, first), "{x} {y}: {c:?} against {first:?}");
        }
    }

    #[test]
    fn a_hard_edge_takes_the_later_stop() {
        let stops = GradientStops::new(vec![
            stop(0, 0, 0, 255, 0.0),
            stop(0, 0, 0, 255, 0.5),
            stop(255, 255, 255, 255, 0.5),
            stop(255, 255, 255, 255, 1.0),
        ])
        .unwrap();
        assert_eq!(stops.colour_at(0.5).r, 255);
        assert_eq!(stops.colour_at(0.499).r, 0);
    }

    /// Premultiplied blending: a fade to transparent white has no grey fringe.
    #[test]
    fn a_fade_to_transparent_is_premultiplied() {
        let stops =
            GradientStops::new(vec![stop(255, 0, 0, 255, 0.0), stop(0, 0, 0, 0, 1.0)]).unwrap();
        let c = stops.colour_at(0.5);
        assert_eq!((c.r, c.g, c.b, c.a), (128, 0, 0, 128));
    }

    #[test]
    fn the_grey_is_the_luminance_over_white() {
        let opaque = |r, g, b| PremultipliedRgba8 { r, g, b, a: 255 };
        assert_eq!(over_white_grey(opaque(0, 0, 0)), 0);
        assert_eq!(over_white_grey(opaque(255, 255, 255)), 255);
        assert_eq!(over_white_grey(opaque(255, 0, 0)), 54); // 0.2126 * 255
        assert_eq!(over_white_grey(opaque(0, 255, 0)), 182); // 0.7152 * 255
        assert_eq!(over_white_grey(opaque(0, 0, 255)), 18); // 0.0722 * 255
        let clear = PremultipliedRgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
        assert_eq!(
            over_white_grey(clear),
            255,
            "transparent is the white below"
        );
    }

    /// Interpolating grey stops equals the grey of the interpolated colour (ADR 0067 §3) within
    /// the rounding of two steps.
    #[test]
    fn grey_stops_interpolate_to_the_grey_of_the_colour() {
        let (a, b) = (stop(200, 30, 90, 180, 0.0), stop(20, 240, 10, 60, 1.0));
        let colour = GradientStops::new(vec![a, b]).unwrap();
        let grey_stop = |s: GradientStop| {
            let alone = GradientStops::new(vec![
                GradientStop {
                    offset: StopOffset::START,
                    ..s
                },
                GradientStop {
                    offset: StopOffset::END,
                    ..s
                },
            ])
            .unwrap();
            let g = over_white_grey(alone.colour_at(0.0));
            GradientStop {
                colour: Color { r: g, g, b: g },
                alpha: 255,
                ..s
            }
        };
        let greys = GradientStops::new(vec![grey_stop(a), grey_stop(b)]).unwrap();
        for i in 0..=20 {
            let t = f64::from(i) / 20.0;
            let direct = over_white_grey(colour.colour_at(t));
            let via = greys.colour_at(t).r;
            assert!(direct.abs_diff(via) <= 1, "{t}: {direct} {via}");
        }
    }

    #[test]
    fn the_frame_turns_the_gradient_with_the_shape() {
        let f = fill(
            GradientGeometry::DEFAULT_LINEAR,
            vec![stop(0, 0, 0, 255, 0.0), stop(255, 255, 255, 255, 1.0)],
        );
        // A quarter turn about the origin: frame x runs down the page.
        let frame = GradientBox {
            to_document: Affine::new(0.0, 1.0, -1.0, 0.0, 0.0, 0.0),
            ..box_100()
        };
        let placed = PlacedGradient::new(&f, &frame);
        assert!((placed.t_at(Point::new(-20.0, 0.0)) - 0.0).abs() < 1e-12);
        assert!((placed.t_at(Point::new(-20.0, 100.0)) - 1.0).abs() < 1e-12);
        assert!((placed.t_at(Point::new(-7.0, 50.0)) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn an_ellipse_reads_its_second_radius() {
        let f = fill(
            GradientGeometry::Radial {
                centre: BoxPoint::new(0.5, 0.5).unwrap(),
                radius: BoxPoint::new(1.0, 0.5).unwrap(),
                ratio: AxisRatio::new(0.5).unwrap(),
            },
            vec![stop(0, 0, 0, 255, 0.0), stop(255, 255, 255, 255, 1.0)],
        );
        let placed = PlacedGradient::new(&f, &box_100());
        // First radius 50 mm along x, second 25 mm along y.
        assert!((placed.t_at(Point::new(100.0, 20.0)) - 1.0).abs() < 1e-12);
        assert!((placed.t_at(Point::new(50.0, 45.0)) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_gradient_of_no_length_shows_its_last_colour() {
        let p = BoxPoint::new(0.3, 0.3).unwrap();
        let f = fill(
            GradientGeometry::Linear { start: p, end: p },
            vec![stop(0, 0, 0, 255, 0.0), stop(9, 9, 9, 255, 1.0)],
        );
        let placed = PlacedGradient::new(&f, &box_100());
        assert_eq!(placed.colour_at(Point::new(1.0, 2.0)).r, 9);
    }
}
