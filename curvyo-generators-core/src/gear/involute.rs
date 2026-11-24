//! The toothed outline of a spur gear: involute flanks fitted by cubics, tip and root arcs.

use curvyo_document_core::Point;
use curvyo_extension_core::{ClosedPath, Cubic};

use crate::outline::{Builder, polar};

/// The deviation a fitted flank may have from the exact involute (ADR 0080, feature decision 5).
pub(crate) const FIT_TOLERANCE_MM: f64 = 0.004;

/// The involute function `tan x - x`.
fn inv(angle: f64) -> f64 {
    libm::tan(angle) - angle
}

/// The shape numbers of one gear.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Teeth {
    pub(crate) module: f64,
    pub(crate) count: u32,
    pub(crate) pressure: f64,
}

/// The circles and angles the outline is built from.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Profile {
    #[cfg(test)]
    pub(crate) pitch_radius: f64,
    pub(crate) base_radius: f64,
    pub(crate) root_radius: f64,
    /// The radius the flank ends at (the tip, pulled in when the tooth would come to a point).
    pub(crate) tip_radius: f64,
    /// The radius the flank starts at (the root or base circle, raised when teeth would overlap).
    pub(crate) start_radius: f64,
    /// Half the tooth angle at the pitch circle plus the involute of the pressure angle.
    offset: f64,
    count: f64,
    teeth: u32,
}

impl Profile {
    pub(crate) fn new(teeth: Teeth) -> Self {
        let count = f64::from(teeth.count);
        let pitch_radius = teeth.module * count / 2.0;
        let base_radius = pitch_radius * libm::cos(teeth.pressure);
        let root_radius = pitch_radius - 1.25 * teeth.module;
        let addendum = pitch_radius + teeth.module;
        let offset = std::f64::consts::PI / (2.0 * count) + inv(teeth.pressure);
        let mut profile = Self {
            #[cfg(test)]
            pitch_radius,
            base_radius,
            root_radius,
            tip_radius: addendum,
            start_radius: base_radius.max(root_radius),
            offset,
            count,
            teeth: teeth.count,
        };
        // A pointed tooth keeps a land of 0.05 module.
        let min_tip = 0.025 * teeth.module / addendum;
        if profile.half_angle(addendum) < min_tip {
            profile.tip_radius = profile.solve(pitch_radius, addendum, |angle| angle - min_tip);
        }
        // Teeth that would overlap at the start keep a root gap of 5 % of the pitch angle.
        let pitch_angle = std::f64::consts::TAU / count;
        let gap = |angle: f64| pitch_angle - 2.0 * angle - 0.05 * pitch_angle;
        if gap(profile.half_angle(profile.start_radius)) < 0.0 {
            profile.start_radius = profile.solve(profile.start_radius, profile.tip_radius, gap);
        }
        profile
    }

    /// The angle from the tooth's centre line to the flank at `radius` (at least the base radius).
    pub(crate) fn half_angle(&self, radius: f64) -> f64 {
        let ratio = (self.base_radius / radius).clamp(-1.0, 1.0);
        self.offset - inv(libm::acos(ratio))
    }

    /// The radius in `low..=high` where `f(half_angle)` crosses zero, by bisection; `f` is
    /// decreasing or increasing across the interval (the flank angle falls with the radius).
    fn solve(&self, low: f64, high: f64, f: impl Fn(f64) -> f64) -> f64 {
        let (mut lo, mut hi) = (low, high);
        let lo_positive = f(self.half_angle(lo.max(self.base_radius))) > 0.0;
        for _ in 0..60 {
            let mid = f64::midpoint(lo, hi);
            let positive = f(self.half_angle(mid.max(self.base_radius))) > 0.0;
            if positive == lo_positive {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        f64::midpoint(lo, hi)
    }

    /// The involute roll length `t` where the flank reaches `radius`.
    fn roll(&self, radius: f64) -> f64 {
        let ratio = (radius / self.base_radius).max(1.0);
        (ratio * ratio - 1.0).sqrt()
    }

    /// The point of the right flank at roll length `t`, turned by `turn` about the centre.
    fn flank_point(&self, t: f64, turn: f64, mirrored: bool) -> Point {
        let r = self.base_radius;
        let (x, y) = (
            r * (libm::cos(t) + t * libm::sin(t)),
            r * (libm::sin(t) - t * libm::cos(t)),
        );
        let y = if mirrored { -y } else { y };
        rotate(Point::new(x, y), turn)
    }

    fn flank_tangent(&self, t: f64, turn: f64, mirrored: bool) -> Point {
        let r = self.base_radius * t;
        let y = r * libm::sin(t);
        rotate(
            Point::new(r * libm::cos(t), if mirrored { -y } else { y }),
            turn,
        )
    }

    /// Cubic Hermite pieces of the flank from `t0` to `t1`, as many as the fit needs.
    fn flank_cubics(&self, t0: f64, t1: f64, turn: f64, mirrored: bool) -> Vec<Cubic> {
        for pieces in 1..=24_u32 {
            let cubics = self.hermite(t0, t1, pieces, turn, mirrored);
            if self.max_error(&cubics, t0, t1, turn, mirrored) <= FIT_TOLERANCE_MM {
                return cubics;
            }
        }
        self.hermite(t0, t1, 24, turn, mirrored)
    }

    fn hermite(&self, t0: f64, t1: f64, pieces: u32, turn: f64, mirrored: bool) -> Vec<Cubic> {
        let step = (t1 - t0) / f64::from(pieces);
        (0..pieces)
            .map(|index| {
                let a = t0 + step * f64::from(index);
                let b = if index + 1 == pieces { t1 } else { a + step };
                let (pa, pb) = (
                    self.flank_point(a, turn, mirrored),
                    self.flank_point(b, turn, mirrored),
                );
                let (ta, tb) = (
                    self.flank_tangent(a, turn, mirrored),
                    self.flank_tangent(b, turn, mirrored),
                );
                let h = (b - a) / 3.0;
                Cubic {
                    c1: Point::new(pa.x + ta.x * h, pa.y + ta.y * h),
                    c2: Point::new(pb.x - tb.x * h, pb.y - tb.y * h),
                    to: pb,
                }
            })
            .collect()
    }

    /// The largest distance between a cubic's sample points and the exact flank at the same roll
    /// length (an upper bound of the distance to the curve).
    fn max_error(&self, cubics: &[Cubic], t0: f64, t1: f64, turn: f64, mirrored: bool) -> f64 {
        let pieces = f64::from(u32::try_from(cubics.len()).unwrap_or(u32::MAX));
        let step = (t1 - t0) / pieces;
        let mut worst: f64 = 0.0;
        for (index, cubic) in cubics.iter().enumerate() {
            let a = t0 + step * f64::from(u32::try_from(index).unwrap_or(0));
            let from = self.flank_point(a, turn, mirrored);
            for u in [0.25, 0.5, 0.75] {
                let q = bezier(from, cubic, u);
                let exact = self.flank_point(a + step * u, turn, mirrored);
                worst = worst.max(libm::hypot(q.x - exact.x, q.y - exact.y));
            }
        }
        worst
    }

    /// The closed toothed outline, counter-clockwise, centred on the origin.
    pub(crate) fn outline(&self) -> ClosedPath {
        let two_pi = std::f64::consts::TAU;
        let origin = Point::new(0.0, 0.0);
        let pitch_angle = two_pi / self.count;
        let t0 = self.roll(self.start_radius);
        let t1 = self.roll(self.tip_radius);
        let flank = self.half_angle(self.start_radius);
        let tip = self.half_angle(self.tip_radius);
        let radial = self.start_radius > self.root_radius + 1e-9;
        let mut builder = Builder::new(polar(origin, self.root_radius, -flank));
        for index in 0..self.teeth {
            let centre = pitch_angle * f64::from(index);
            let right_turn = centre - self.offset;
            let left_turn = centre + self.offset;
            if radial {
                builder.line_to(polar(origin, self.start_radius, centre - flank));
            }
            for cubic in self.flank_cubics(t0, t1, right_turn, false) {
                builder.cubic_to(cubic.c1, cubic.c2, cubic.to);
            }
            builder.arc(origin, self.tip_radius, centre - tip, centre + tip);
            // The left flank is the mirror image, walked from the tip down.
            let up = self.flank_cubics(t0, t1, left_turn, true);
            let mut points = vec![self.flank_point(t0, left_turn, true)];
            points.extend(up.iter().map(|cubic| cubic.to));
            for (position, cubic) in up.iter().enumerate().rev() {
                builder.cubic_to(cubic.c2, cubic.c1, points[position]);
            }
            if radial {
                builder.line_to(polar(origin, self.root_radius, centre + flank));
            }
            builder.arc(
                origin,
                self.root_radius,
                centre + flank,
                centre + pitch_angle - flank,
            );
        }
        builder.snap_end();
        builder.finish()
    }

    /// The point of the exact right flank of the tooth centred on angle 0 at `radius`, for tests.
    #[cfg(test)]
    pub(crate) fn exact_right_flank(&self, radius: f64) -> Point {
        self.flank_point(self.roll(radius), -self.offset, false)
    }
}

fn rotate(point: Point, angle: f64) -> Point {
    let (sin, cos) = (libm::sin(angle), libm::cos(angle));
    Point::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

/// The point at `u` of the cubic that starts at `from`.
#[allow(clippy::many_single_char_names)] // the four Bernstein weights
pub(crate) fn bezier(from: Point, cubic: &Cubic, u: f64) -> Point {
    let v = 1.0 - u;
    let (a, b, c, d) = (v * v * v, 3.0 * v * v * u, 3.0 * v * u * u, u * u * u);
    Point::new(
        a * from.x + b * cubic.c1.x + c * cubic.c2.x + d * cubic.to.x,
        a * from.y + b * cubic.c1.y + c * cubic.c2.y + d * cubic.to.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The distance from `p` to the exact flank: a dense scan over the roll length, then a
    /// golden-section refinement around the best sample.
    fn distance_to_flank(profile: &Profile, p: Point, t0: f64, t1: f64) -> f64 {
        let dist = |t: f64| {
            let q = profile.flank_point(t, -profile.offset, false);
            (q.x - p.x).hypot(q.y - p.y)
        };
        let n = 2000;
        let mut best = (f64::MAX, t0);
        for i in 0..=n {
            let t = t0 + (t1 - t0) * f64::from(i) / f64::from(n);
            let d = dist(t);
            if d < best.0 {
                best = (d, t);
            }
        }
        let step = (t1 - t0) / f64::from(n);
        let (mut lo, mut hi) = ((best.1 - step).max(t0), (best.1 + step).min(t1));
        for _ in 0..60 {
            let (a, b) = (lo + (hi - lo) * 0.382, lo + (hi - lo) * 0.618);
            if dist(a) < dist(b) {
                hi = b;
            } else {
                lo = a;
            }
        }
        dist(f64::midpoint(lo, hi))
    }

    #[test]
    fn the_fitted_flank_stays_within_001_mm_of_the_exact_involute() {
        for (module, count, pressure) in [
            (2.0, 24, 20.0_f64),
            (1.0, 6, 20.0),
            (0.5, 200, 20.0),
            (10.0, 12, 14.5),
            (25.0, 200, 30.0),
            (3.0, 60, 10.0),
        ] {
            let profile = Profile::new(Teeth {
                module,
                count,
                pressure: pressure.to_radians(),
            });
            let (t0, t1) = (
                profile.roll(profile.start_radius),
                profile.roll(profile.tip_radius),
            );
            let turn = -profile.offset;
            let cubics = profile.flank_cubics(t0, t1, turn, false);
            let mut from = profile.flank_point(t0, turn, false);
            let mut worst: f64 = 0.0;
            for cubic in &cubics {
                for k in 0..=40 {
                    let q = bezier(from, cubic, f64::from(k) / 40.0);
                    worst = worst.max(distance_to_flank(&profile, q, t0 - 1e-9, t1 + 1e-9));
                }
                from = cubic.to;
            }
            assert!(
                worst < 0.01,
                "module {module} teeth {count} angle {pressure}: {worst} mm"
            );
        }
    }

    #[test]
    fn the_flank_passes_through_the_pitch_point_at_half_the_tooth_angle() {
        let profile = Profile::new(Teeth {
            module: 2.0,
            count: 24,
            pressure: 20.0_f64.to_radians(),
        });
        let p = profile.exact_right_flank(profile.pitch_radius);
        let angle = libm::atan2(p.y, p.x);
        assert!(
            (angle + std::f64::consts::PI / 48.0).abs() < 1e-9,
            "{angle}"
        );
        assert!((p.x.hypot(p.y) - 24.0).abs() < 1e-9);
    }
}
