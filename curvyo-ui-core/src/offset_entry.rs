//! The values of the Offset bar (`specs/0038-path-offset` criteria 4, 6, 7, 19, 26): the unsigned
//! distance and its range, Direction, Corners, Ends and Result, and which sides a drag or Apply
//! writes and how, from them and the modifier keys.

pub use curvyo_geometry_core::{OffsetCap, OffsetJoin};

/// The smallest distance the Distance field takes, in millimetres.
pub const OFFSET_MIN_MM: f64 = 0.01;

/// The largest distance, typed or dragged, in millimetres.
pub const OFFSET_MAX_MM: f64 = 1000.0;

/// The distance on first use, in millimetres.
pub const OFFSET_DEFAULT_MM: f64 = 1.0;

/// Which way the outline moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetDirection {
    /// Grow (Outset).
    Out,
    /// Shrink (Inset).
    In,
    /// Both sides.
    Both,
}

/// Whether a result is a new object or the changed original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetResultChoice {
    /// A new object; the original stays.
    NewObject,
    /// The original is changed.
    ChangeOriginal,
}

/// One side of an offset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetSide {
    /// The outset.
    Out,
    /// The inset.
    In,
}

/// What becomes of one side's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetRole {
    /// A new object.
    Copy,
    /// The original takes it.
    Original,
}

/// The sides an offset writes and what becomes of each (criterion 19's table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetSides {
    /// The outset's role, if the outset is written.
    pub out: Option<OffsetRole>,
    /// The inset's role, if the inset is written.
    pub inside: Option<OffsetRole>,
}

impl OffsetSides {
    /// The sides for a drag towards `dragged`, with `both` (Shift) and `change` (the effective
    /// Change original: the bar's Result inverted by Ctrl). Shift+Ctrl: the original takes the
    /// dragged side and a copy is made for the other.
    #[must_use]
    pub const fn of(dragged: OffsetSide, both: bool, change: bool) -> Self {
        let own = if change {
            OffsetRole::Original
        } else {
            OffsetRole::Copy
        };
        let other = if both { Some(OffsetRole::Copy) } else { None };
        match dragged {
            OffsetSide::Out => Self {
                out: Some(own),
                inside: other,
            },
            OffsetSide::In => Self {
                out: other,
                inside: Some(own),
            },
        }
    }
}

/// What the bar holds. The session keeps one for its whole life; it is never stored in the
/// document and never part of undo (criterion 7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetBar {
    distance_mm: f64,
    direction: OffsetDirection,
    join: OffsetJoin,
    cap: OffsetCap,
    result: OffsetResultChoice,
}

impl Default for OffsetBar {
    fn default() -> Self {
        Self {
            distance_mm: OFFSET_DEFAULT_MM,
            direction: OffsetDirection::Out,
            join: OffsetJoin::Round,
            cap: OffsetCap::Round,
            result: OffsetResultChoice::NewObject,
        }
    }
}

impl OffsetBar {
    /// The unsigned distance in millimetres.
    #[must_use]
    pub const fn distance_mm(&self) -> f64 {
        self.distance_mm
    }

    /// The direction.
    #[must_use]
    pub const fn direction(&self) -> OffsetDirection {
        self.direction
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

    /// New object or Change original.
    #[must_use]
    pub const fn result(&self) -> OffsetResultChoice {
        self.result
    }

    /// Sets the distance, clamped to 0.01..1000 mm (criterion 6). Returns whether the value was
    /// within the range (the field is marked invalid otherwise); a value that is not a number
    /// changes nothing and returns `false`.
    pub fn set_distance_mm(&mut self, distance_mm: f64) -> bool {
        if !distance_mm.is_finite() {
            return false;
        }
        let clamped = distance_mm.clamp(OFFSET_MIN_MM, OFFSET_MAX_MM);
        self.distance_mm = clamped;
        (clamped - distance_mm).abs() <= f64::EPSILON * OFFSET_MAX_MM
    }

    /// Sets the direction.
    pub fn set_direction(&mut self, direction: OffsetDirection) {
        self.direction = direction;
    }

    /// Sets the join.
    pub fn set_join(&mut self, join: OffsetJoin) {
        self.join = join;
    }

    /// Sets the cap.
    pub fn set_cap(&mut self, cap: OffsetCap) {
        self.cap = cap;
    }

    /// Sets New object or Change original.
    pub fn set_result(&mut self, result: OffsetResultChoice) {
        self.result = result;
    }

    /// Whether a drag changes the original: the bar's Result, inverted while Ctrl is held.
    #[must_use]
    pub fn changes_original(&self, ctrl: bool) -> bool {
        (self.result == OffsetResultChoice::ChangeOriginal) != ctrl
    }

    /// The sides of the Apply route (criterion 26): Direction and Result from the bar; Both with
    /// Change original gives the Out side to the original.
    #[must_use]
    pub fn apply_sides(&self) -> OffsetSides {
        let change = self.result == OffsetResultChoice::ChangeOriginal;
        match self.direction {
            OffsetDirection::Out => OffsetSides::of(OffsetSide::Out, false, change),
            OffsetDirection::In => OffsetSides::of(OffsetSide::In, false, change),
            OffsetDirection::Both => OffsetSides::of(OffsetSide::Out, true, change),
        }
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

/// The direction named by its host code (`"out"`, `"in"`, `"both"`).
#[must_use]
pub fn offset_direction_from_code(code: &str) -> Option<OffsetDirection> {
    Some(match code {
        "out" => OffsetDirection::Out,
        "in" => OffsetDirection::In,
        "both" => OffsetDirection::Both,
        _ => return None,
    })
}

/// The result choice named by its host code (`"new"`, `"change"`).
#[must_use]
pub fn offset_result_from_code(code: &str) -> Option<OffsetResultChoice> {
    Some(match code {
        "new" => OffsetResultChoice::NewObject,
        "change" => OffsetResultChoice::ChangeOriginal,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac4_ac6_the_first_use_is_one_millimetre_out_round_new_object() {
        let bar = OffsetBar::default();
        assert!((bar.distance_mm() - 1.0).abs() < f64::EPSILON);
        assert_eq!(bar.direction(), OffsetDirection::Out);
        assert_eq!(bar.join(), OffsetJoin::Round);
        assert_eq!(bar.cap(), OffsetCap::Round);
        assert_eq!(bar.result(), OffsetResultChoice::NewObject);
    }

    #[test]
    fn ac6_the_distance_is_clamped_to_its_range() {
        let mut bar = OffsetBar::default();
        assert!(bar.set_distance_mm(2.5));
        assert!(!bar.set_distance_mm(1500.0));
        assert!((bar.distance_mm() - 1000.0).abs() < f64::EPSILON);
        assert!(!bar.set_distance_mm(0.001));
        assert!((bar.distance_mm() - 0.01).abs() < f64::EPSILON);
        assert!(!bar.set_distance_mm(f64::NAN));
        assert!((bar.distance_mm() - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn ac19_the_modifier_table() {
        use OffsetRole::{Copy, Original};
        let bar = OffsetBar::default();
        let sides =
            |dragged, shift, ctrl| OffsetSides::of(dragged, shift, bar.changes_original(ctrl));
        assert_eq!(
            sides(OffsetSide::Out, false, false),
            OffsetSides {
                out: Some(Copy),
                inside: None
            }
        );
        assert_eq!(
            sides(OffsetSide::In, true, false),
            OffsetSides {
                out: Some(Copy),
                inside: Some(Copy)
            }
        );
        assert_eq!(
            sides(OffsetSide::In, false, true),
            OffsetSides {
                out: None,
                inside: Some(Original)
            }
        );
        assert_eq!(
            sides(OffsetSide::Out, true, true),
            OffsetSides {
                out: Some(Original),
                inside: Some(Copy)
            },
            "Shift+Ctrl: the original takes the dragged side"
        );
        let mut change = OffsetBar::default();
        change.set_result(OffsetResultChoice::ChangeOriginal);
        assert!(change.changes_original(false));
        assert!(
            !change.changes_original(true),
            "Ctrl inverts the bar's choice"
        );
    }

    #[test]
    fn ac26_the_apply_route_reads_direction_and_result() {
        let mut bar = OffsetBar::default();
        bar.set_direction(OffsetDirection::Both);
        bar.set_result(OffsetResultChoice::ChangeOriginal);
        assert_eq!(
            bar.apply_sides(),
            OffsetSides {
                out: Some(OffsetRole::Original),
                inside: Some(OffsetRole::Copy)
            }
        );
    }

    #[test]
    fn codes_name_the_values() {
        assert_eq!(offset_join_from_code("miter"), Some(OffsetJoin::Miter));
        assert_eq!(offset_cap_from_code("square"), Some(OffsetCap::Square));
        assert_eq!(
            offset_direction_from_code("both"),
            Some(OffsetDirection::Both)
        );
        assert_eq!(
            offset_result_from_code("change"),
            Some(OffsetResultChoice::ChangeOriginal)
        );
        assert_eq!(offset_join_from_code("butt"), None);
        assert_eq!(offset_cap_from_code("miter"), None);
        assert_eq!(offset_direction_from_code("up"), None);
        assert_eq!(offset_result_from_code("keep"), None);
    }
}
