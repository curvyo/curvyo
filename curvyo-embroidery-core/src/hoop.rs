//! The W6 EU7 hoops as fixtures of an embroidery page (`specs/0160-embroidery` criteria 1, 3, 4,
//! `adrs.md` EM2, EM7, EM10): their sewing fields, centred on the page, and the needle points that
//! fall outside the active one.

use crate::stitch::{Stitch, StitchKind};

/// The margin of the page around the largest sewing field on every side (UX M5).
pub const PAGE_MARGIN_MM: f64 = 20.0;

/// A hoop of the EU7 (criterion 2, fact 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HoopSize {
    /// The 150 x 250 mm sewing field, the default.
    #[default]
    Field150x250,
    /// The 100 x 100 mm sewing field.
    Field100x100,
}

impl HoopSize {
    /// Every hoop, in pick order.
    pub const ALL: [Self; 2] = [Self::Field150x250, Self::Field100x100];

    /// The stored spelling inside a profile (`150x250`).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Field150x250 => "150x250",
            Self::Field100x100 => "100x100",
        }
    }

    /// The hoop a stored spelling names.
    #[must_use]
    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|hoop| hoop.key() == key)
    }

    /// The stored fixture kind (`curvyo.embroidery/hoop-150x250`).
    #[must_use]
    pub const fn fixture_kind(self) -> &'static str {
        match self {
            Self::Field150x250 => "curvyo.embroidery/hoop-150x250",
            Self::Field100x100 => "curvyo.embroidery/hoop-100x100",
        }
    }

    /// The hoop a stored fixture kind names, `None` for another kind.
    #[must_use]
    pub fn of_fixture_kind(kind: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|hoop| hoop.fixture_kind() == kind)
    }

    /// Whether a stored fixture kind is a hoop of this family, known or not.
    #[must_use]
    pub fn is_hoop_kind(kind: &str) -> bool {
        kind.starts_with("curvyo.embroidery/hoop-")
    }

    /// The segment's label ("150 × 250 mm").
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Field150x250 => "150 × 250 mm",
            Self::Field100x100 => "100 × 100 mm",
        }
    }

    /// The sewing field's width and height in whole mm (150 wide, 250 high, UX E3).
    #[must_use]
    pub const fn field_mm(self) -> (u32, u32) {
        match self {
            Self::Field150x250 => (150, 250),
            Self::Field100x100 => (100, 100),
        }
    }

    /// The sewing field's area in mm², to find the largest hoop.
    #[must_use]
    pub const fn area_mm2(self) -> u32 {
        let (w, h) = self.field_mm();
        w * h
    }

    /// Whether `stitch` is a needle point outside the field centred on the hoop centre (EM7).
    #[must_use]
    pub fn is_outside(self, stitch: &Stitch) -> bool {
        if stitch.kind != StitchKind::Normal {
            return false;
        }
        let (w, h) = self.field_mm();
        // invariant: the fields are at most 250 mm, 2 500 tenths
        let half = |mm: u32| i32::try_from(mm * 5).unwrap_or(i32::MAX);
        stitch.at.x.get().abs() > half(w) || stitch.at.y.get().abs() > half(h)
    }
}

/// The page size of `hoops`: the largest sewing field plus the margin on every side, in mm
/// (190 x 290 for the EU7, `adrs.md` EM10).
#[must_use]
pub fn page_mm(hoops: &[HoopSize]) -> (f64, f64) {
    let largest = hoops
        .iter()
        .copied()
        .max_by_key(|h| h.area_mm2())
        .unwrap_or_default();
    let (w, h) = largest.field_mm();
    (
        f64::from(w) + 2.0 * PAGE_MARGIN_MM,
        f64::from(h) + 2.0 * PAGE_MARGIN_MM,
    )
}

/// The indices of the stitches of `stitches` outside `hoop`'s field.
#[must_use]
pub fn outside_field(stitches: &[Stitch], hoop: HoopSize) -> Vec<usize> {
    stitches
        .iter()
        .enumerate()
        .filter(|(_, s)| hoop.is_outside(s))
        .map(|(i, _)| i)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stitch::StitchPoint;

    #[test]
    fn the_page_is_the_largest_field_plus_20_mm_and_the_field_is_centred() {
        assert_eq!(page_mm(&HoopSize::ALL), (190.0, 290.0));
        let at = |x, y| Stitch {
            at: StitchPoint::new(x, y),
            kind: StitchKind::Normal,
        };
        let hoop = HoopSize::Field100x100;
        assert!(!hoop.is_outside(&at(500, -500)), "on the edge");
        assert!(hoop.is_outside(&at(510, 0)), "1 mm past it");
        assert_eq!(outside_field(&[at(0, 0), at(0, 501)], hoop), [1]);
        assert_eq!(
            HoopSize::of_fixture_kind("curvyo.embroidery/hoop-100x100"),
            Some(hoop)
        );
        assert!(HoopSize::is_hoop_kind("curvyo.embroidery/hoop-9x9"));
    }
}
