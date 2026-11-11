//! Smooth, nonnegative width multipliers along a path's normalised length (ADR 0053).

/// A width control point, identified independently of its position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WidthPoint {
    id: u128,
    position: f64,
    multiplier: f64,
}

impl WidthPoint {
    /// Validates a point for an open or closed path.
    #[must_use]
    pub fn new(id: u128, position: f64, multiplier: f64, closed: bool) -> Option<Self> {
        (position.is_finite()
            && position >= 0.0
            && if closed {
                position < 1.0
            } else {
                position <= 1.0
            }
            && multiplier.is_finite()
            && multiplier >= 0.0)
            .then_some(Self {
                id,
                position: if position == 0.0 { 0.0 } else { position },
                multiplier,
            })
    }
}

/// A profile ordered by position, with deterministic coincident-point resolution.
#[derive(Debug, Clone)]
pub struct WidthProfile {
    points: Vec<WidthPoint>,
    closed: bool,
}

impl WidthProfile {
    /// Validates and sorts points; coincident points retain the greatest identifier's value.
    #[must_use]
    pub fn new(mut points: Vec<WidthPoint>, closed: bool) -> Option<Self> {
        if closed && points.iter().any(|point| point.position >= 1.0) {
            return None;
        }
        points.sort_by(|a, b| a.position.total_cmp(&b.position).then(a.id.cmp(&b.id)));
        points.dedup_by(|a, b| {
            if a.position.total_cmp(&b.position).is_eq() {
                *b = *a;
                true
            } else {
                false
            }
        });
        Some(Self { points, closed })
    }

    /// The constant multiplier if the profile does not vary.
    #[must_use]
    pub fn constant_multiplier(&self) -> Option<f64> {
        let Some(first) = self.points.first() else {
            return Some(1.0);
        };
        self.points
            .iter()
            .all(|point| point.multiplier.total_cmp(&first.multiplier).is_eq())
            .then_some(first.multiplier)
    }

    /// Whether the profile wraps at a closed path's seam.
    #[must_use]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    /// The largest multiplier anywhere on the path (smoothstep never overshoots its knots).
    #[must_use]
    pub fn max_multiplier(&self) -> f64 {
        self.points
            .iter()
            .map(|point| point.multiplier)
            .fold(if self.points.is_empty() { 1.0 } else { 0.0 }, f64::max)
    }

    /// The positions of the points, in order.
    pub(crate) fn positions(&self) -> impl Iterator<Item = f64> + '_ {
        self.points.iter().map(|point| point.position)
    }

    /// Samples the multiplier at a normalised length, wrapping closed paths at the seam.
    #[must_use]
    pub fn sample(&self, position: f64) -> Option<f64> {
        if !position.is_finite() || !(0.0..=1.0).contains(&position) {
            return None;
        }
        let (Some(first), Some(last)) = (self.points.first(), self.points.last()) else {
            return Some(1.0);
        };
        if self.points.len() == 1 {
            return Some(first.multiplier);
        }
        let position = if self.closed && position >= 1.0 {
            0.0
        } else {
            position
        };
        if !self.closed && position <= first.position {
            return Some(first.multiplier);
        }
        if !self.closed && position >= last.position {
            return Some(last.multiplier);
        }
        for pair in self.points.windows(2) {
            if position >= pair[0].position && position <= pair[1].position {
                return Some(interpolate(pair[0], pair[1], position));
            }
        }
        let mut next = *first;
        next.position += 1.0;
        let position = if position < first.position {
            position + 1.0
        } else {
            position
        };
        Some(interpolate(*last, next, position))
    }
}

/// Smoothstep between two points: continuous, zero slope at both, no overshoot.
fn interpolate(a: WidthPoint, b: WidthPoint, position: f64) -> f64 {
    let t = (position - a.position) / (b.position - a.position);
    let weight = (t * t * (3.0 - 2.0 * t)).clamp(0.0, 1.0);
    a.multiplier * (1.0 - weight) + b.multiplier * weight
}
