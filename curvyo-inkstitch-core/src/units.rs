// SPDX-License-Identifier: GPL-3.0-or-later
// Part of the Ink/Stitch port (https://github.com/inkstitch/inkstitch); Copyright (c) 2010 Authors
// (see git history). Not derived from one upstream file; see `NOTICE.md`.

//! The port's units (`CLAUDE.md` §5): lengths in millimetres and angles in radians.

/// A length in millimetres; never negative, never NaN.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Mm(f64);

impl Mm {
    /// `mm` when it is finite and not negative.
    #[must_use]
    pub fn new(mm: f64) -> Option<Self> {
        (mm.is_finite() && mm >= 0.0).then_some(Self(mm))
    }

    /// A constant of the port; `mm` must be finite and not negative.
    pub(crate) const fn literal(mm: f64) -> Self {
        Self(mm)
    }

    /// `mm` clamped into the unit's domain: NaN and negatives become zero, infinity the largest
    /// finite value.
    #[must_use]
    pub fn saturating(mm: f64) -> Self {
        if mm.is_nan() || mm <= 0.0 {
            Self(0.0)
        } else {
            Self(mm.min(f64::MAX))
        }
    }

    /// The length in mm.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// An angle in radians, counter-clockwise from the X axis as Ink/Stitch counts it; finite.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Angle(f64);

impl Angle {
    /// `degrees` as an angle, `None` when it is not finite.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Option<Self> {
        degrees.is_finite().then(|| Self(degrees.to_radians()))
    }

    /// The angle in radians.
    #[must_use]
    pub const fn radians(self) -> f64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_length_is_finite_and_not_negative() {
        assert_eq!(Mm::new(2.5).map(Mm::get), Some(2.5));
        assert!(Mm::new(-0.1).is_none());
        assert!(Mm::new(f64::NAN).is_none());
        assert_eq!(Mm::saturating(-3.0).get(), 0.0);
    }
}
