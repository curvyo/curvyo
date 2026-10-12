//! The values of the Offset entry (`specs/0038-path-offset` criterion 1): the distance with its
//! typed and dragged ranges, the join and the cap, remembered for the session.

pub use curvyo_geometry_core::{OffsetCap, OffsetJoin};

/// The largest typed distance, in millimetres, either way.
pub const OFFSET_TYPED_LIMIT_MM: f64 = 1000.0;

/// The largest dragged distance, in millimetres, either way.
pub const OFFSET_DRAG_LIMIT_MM: f64 = 50.0;

/// The distance on first use, in millimetres.
pub const OFFSET_DEFAULT_MM: f64 = 1.0;

/// What the Offset entry holds. The session keeps one for its whole life, so an entry opened
/// again shows the last values used (criterion 1); it is never stored in the document.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetSettings {
    distance_mm: f64,
    join: OffsetJoin,
    cap: OffsetCap,
}

impl Default for OffsetSettings {
    fn default() -> Self {
        Self {
            distance_mm: OFFSET_DEFAULT_MM,
            join: OffsetJoin::Round,
            cap: OffsetCap::Round,
        }
    }
}

impl OffsetSettings {
    /// The signed distance in millimetres: positive outward, negative inward.
    #[must_use]
    pub const fn distance_mm(&self) -> f64 {
        self.distance_mm
    }

    /// The corner join.
    #[must_use]
    pub const fn join(&self) -> OffsetJoin {
        self.join
    }

    /// The end cap of open paths.
    #[must_use]
    pub const fn cap(&self) -> OffsetCap {
        self.cap
    }

    /// Sets the distance if it is a finite number within the typed range; returns whether it was
    /// taken. A dragged value is clamped to the drag range by the field before it gets here.
    pub fn set_distance_mm(&mut self, distance_mm: f64) -> bool {
        if distance_mm.is_finite() && distance_mm.abs() <= OFFSET_TYPED_LIMIT_MM {
            self.distance_mm = distance_mm;
            true
        } else {
            false
        }
    }

    /// Sets the join.
    pub fn set_join(&mut self, join: OffsetJoin) {
        self.join = join;
    }

    /// Sets the cap.
    pub fn set_cap(&mut self, cap: OffsetCap) {
        self.cap = cap;
    }
}

/// The join named by its host code (`"round"`, `"miter"`, `"bevel"`).
#[must_use]
pub fn offset_join_from_code(code: &str) -> Option<OffsetJoin> {
    Some(match code {
        "round" => OffsetJoin::Round,
        "miter" => OffsetJoin::Miter,
        "bevel" => OffsetJoin::Bevel,
        _ => return None,
    })
}

/// The cap named by its host code (`"round"`, `"flat"`, `"square"`).
#[must_use]
pub fn offset_cap_from_code(code: &str) -> Option<OffsetCap> {
    Some(match code {
        "round" => OffsetCap::Round,
        "flat" => OffsetCap::Flat,
        "square" => OffsetCap::Square,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_use_is_one_millimetre_round() {
        let settings = OffsetSettings::default();
        assert!((settings.distance_mm() - 1.0).abs() < f64::EPSILON);
        assert_eq!(settings.join(), OffsetJoin::Round);
        assert_eq!(settings.cap(), OffsetCap::Round);
    }

    #[test]
    fn the_distance_accepts_the_typed_range_only() {
        let mut settings = OffsetSettings::default();
        assert!(settings.set_distance_mm(-1000.0));
        assert!(settings.set_distance_mm(1000.0));
        assert!(!settings.set_distance_mm(1000.5));
        assert!(!settings.set_distance_mm(f64::NAN));
        assert!(!settings.set_distance_mm(f64::NEG_INFINITY));
        assert!((settings.distance_mm() - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn codes_name_the_joins_and_caps() {
        assert_eq!(offset_join_from_code("miter"), Some(OffsetJoin::Miter));
        assert_eq!(offset_join_from_code("bevel"), Some(OffsetJoin::Bevel));
        assert_eq!(offset_join_from_code("round"), Some(OffsetJoin::Round));
        assert_eq!(offset_join_from_code("butt"), None);
        assert_eq!(offset_cap_from_code("flat"), Some(OffsetCap::Flat));
        assert_eq!(offset_cap_from_code("square"), Some(OffsetCap::Square));
        assert_eq!(offset_cap_from_code("round"), Some(OffsetCap::Round));
        assert_eq!(offset_cap_from_code("miter"), None);
    }
}
