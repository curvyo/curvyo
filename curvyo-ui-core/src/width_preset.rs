//! The ready-made width profiles the Style panel offers (`specs/0110-pen-modes-variable-stroke`
//! criterion 5) as width points along the centre line.

/// A ready-made width profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidthPreset {
    /// The stroke's own width everywhere: no width points.
    Uniform,
    /// From nothing at the start to the full width at the end.
    TaperStart,
    /// From the full width at the start to nothing at the end.
    TaperEnd,
    /// From nothing at both ends to the full width in the middle.
    TaperBoth,
}

/// How far a stored position or multiplier may lie from a preset's and still be that preset.
const PRESET_MATCH_TOLERANCE: f64 = 1e-9;

impl WidthPreset {
    /// The preset of a host code (`uniform`, `taper-start`, `taper-end`, `taper-both`).
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "uniform" => Self::Uniform,
            "taper-start" => Self::TaperStart,
            "taper-end" => Self::TaperEnd,
            "taper-both" => Self::TaperBoth,
            _ => return None,
        })
    }

    /// The width points as (fraction of the centre line, multiplier of the stroke width).
    #[must_use]
    pub const fn points(self) -> &'static [(f64, f64)] {
        match self {
            Self::Uniform => &[],
            Self::TaperStart => &[(0.0, 0.0), (1.0, 1.0)],
            Self::TaperEnd => &[(0.0, 1.0), (1.0, 0.0)],
            Self::TaperBoth => &[(0.0, 0.0), (0.5, 1.0), (1.0, 0.0)],
        }
    }

    /// The host code of the preset, as [`WidthPreset::from_code`] reads it.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Uniform => "uniform",
            Self::TaperStart => "taper-start",
            Self::TaperEnd => "taper-end",
            Self::TaperBoth => "taper-both",
        }
    }

    /// The preset whose points are `points` (position, multiplier) in any order, within
    /// 1e-9; `None` for a profile drawn by hand.
    #[must_use]
    pub fn matching(points: &[(f64, f64)]) -> Option<Self> {
        let mut sorted = points.to_vec();
        sorted.sort_by(|a, b| a.0.total_cmp(&b.0));
        [
            Self::Uniform,
            Self::TaperStart,
            Self::TaperEnd,
            Self::TaperBoth,
        ]
        .into_iter()
        .find(|preset| {
            preset.points().len() == sorted.len()
                && preset.points().iter().zip(&sorted).all(|(a, b)| {
                    (a.0 - b.0).abs() <= PRESET_MATCH_TOLERANCE
                        && (a.1 - b.1).abs() <= PRESET_MATCH_TOLERANCE
                })
        })
    }

    /// Whether the preset needs two ends, which a closed path does not have.
    #[must_use]
    pub const fn needs_ends(self) -> bool {
        !matches!(self, Self::Uniform)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_is_matched_to_its_preset_in_any_order() {
        assert_eq!(WidthPreset::matching(&[]), Some(WidthPreset::Uniform));
        assert_eq!(
            WidthPreset::matching(&[(1.0, 0.0), (0.0, 0.0), (0.5, 1.0)]),
            Some(WidthPreset::TaperBoth)
        );
        assert_eq!(WidthPreset::matching(&[(0.0, 0.0), (1.0, 1.2)]), None);
        for preset in [WidthPreset::TaperStart, WidthPreset::TaperEnd] {
            assert_eq!(WidthPreset::from_code(preset.code()), Some(preset));
        }
    }

    #[test]
    fn every_code_reads_back_and_tapers_reach_zero_at_their_ends() {
        for (code, preset) in [
            ("uniform", WidthPreset::Uniform),
            ("taper-start", WidthPreset::TaperStart),
            ("taper-end", WidthPreset::TaperEnd),
            ("taper-both", WidthPreset::TaperBoth),
        ] {
            assert_eq!(WidthPreset::from_code(code), Some(preset));
        }
        assert_eq!(WidthPreset::from_code("wobble"), None);
        assert_eq!(WidthPreset::Uniform.points(), &[] as &[(f64, f64)]);
        assert_eq!(WidthPreset::TaperStart.points()[0], (0.0, 0.0));
        assert_eq!(WidthPreset::TaperEnd.points()[1], (1.0, 0.0));
        let both = WidthPreset::TaperBoth.points();
        assert_eq!((both[0].1, both[2].1), (0.0, 0.0));
    }
}
