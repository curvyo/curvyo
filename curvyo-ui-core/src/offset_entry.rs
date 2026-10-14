//! The remembered values of the Offset tool (`specs/0038-path-offset` criteria 6, 7, 19, 26, 54):
//! the unsigned distance and its range, Direction, Corners and Mode as the Offset entry chip shows
//! them, and which sides a drag or the chip's Enter writes and how, from them and the modifier keys.

pub use curvyo_geometry_core::OffsetJoin;

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
    /// Change original: the remembered Mode, switched by Ctrl). Both with Change original: the
    /// original takes the outset and one copy is made for the inset, whatever the drag direction
    /// (criterion 19, third round).
    #[must_use]
    pub const fn of(dragged: OffsetSide, both: bool, change: bool) -> Self {
        let own = if change {
            OffsetRole::Original
        } else {
            OffsetRole::Copy
        };
        if both {
            return Self {
                out: Some(own),
                inside: Some(OffsetRole::Copy),
            };
        }
        match dragged {
            OffsetSide::Out => Self {
                out: Some(own),
                inside: None,
            },
            OffsetSide::In => Self {
                out: None,
                inside: Some(own),
            },
        }
    }
}

/// The remembered values, which the Offset entry chip shows and sets. The session keeps one for
/// its whole life; it is never stored in the document and never part of undo (criterion 7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OffsetValues {
    distance_mm: f64,
    direction: OffsetDirection,
    join: OffsetJoin,
    result: OffsetResultChoice,
}

impl Default for OffsetValues {
    fn default() -> Self {
        Self {
            distance_mm: OFFSET_DEFAULT_MM,
            direction: OffsetDirection::Out,
            join: OffsetJoin::Round,
            result: OffsetResultChoice::ChangeOriginal,
        }
    }
}

impl OffsetValues {
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

    /// Sets New object or Change original.
    pub fn set_result(&mut self, result: OffsetResultChoice) {
        self.result = result;
    }

    /// Whether a drag changes the original: the remembered Mode, inverted while Ctrl is held.
    #[must_use]
    pub fn changes_original(&self, ctrl: bool) -> bool {
        (self.result == OffsetResultChoice::ChangeOriginal) != ctrl
    }

    /// The sides the standing preview shows and the chip's Enter writes (criteria 26, 52):
    /// Direction and Mode as remembered; Both with Change original gives the Out side to the
    /// original.
    #[must_use]
    pub fn entry_sides(&self) -> OffsetSides {
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

/// The corner style after `join` in the cycle of a double-click on a corner handle: Round,
/// Miter, Bevel, back to Round (criteria 13, 56).
#[must_use]
pub const fn next_offset_join(join: OffsetJoin) -> OffsetJoin {
    match join {
        OffsetJoin::Round => OffsetJoin::Miter,
        OffsetJoin::Miter => OffsetJoin::Bevel,
        OffsetJoin::Bevel => OffsetJoin::Round,
    }
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
    fn ac6_ac54_the_first_use_is_one_millimetre_out_round_change_original() {
        let values = OffsetValues::default();
        assert!((values.distance_mm() - 1.0).abs() < f64::EPSILON);
        assert_eq!(values.direction(), OffsetDirection::Out);
        assert_eq!(values.join(), OffsetJoin::Round);
        assert_eq!(
            values.result(),
            OffsetResultChoice::ChangeOriginal,
            "third round: Change original by default"
        );
    }

    #[test]
    fn ac6_the_distance_is_clamped_to_its_range() {
        let mut values = OffsetValues::default();
        assert!(values.set_distance_mm(2.5));
        assert!(!values.set_distance_mm(1500.0));
        assert!((values.distance_mm() - 1000.0).abs() < f64::EPSILON);
        assert!(!values.set_distance_mm(0.001));
        assert!((values.distance_mm() - 0.01).abs() < f64::EPSILON);
        assert!(!values.set_distance_mm(f64::NAN));
        assert!((values.distance_mm() - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn ac19_the_modifier_table() {
        use OffsetRole::{Copy, Original};
        let values = OffsetValues::default();
        let sides =
            |dragged, shift, ctrl| OffsetSides::of(dragged, shift, values.changes_original(ctrl));
        for dragged in [OffsetSide::Out, OffsetSide::In] {
            let own = |role| match dragged {
                OffsetSide::Out => OffsetSides {
                    out: Some(role),
                    inside: None,
                },
                OffsetSide::In => OffsetSides {
                    out: None,
                    inside: Some(role),
                },
            };
            assert_eq!(sides(dragged, false, false), own(Original), "none: change");
            assert_eq!(sides(dragged, false, true), own(Copy), "Ctrl: new object");
            assert_eq!(
                sides(dragged, true, false),
                OffsetSides {
                    out: Some(Original),
                    inside: Some(Copy)
                },
                "Shift: the original takes the outset whatever the drag direction"
            );
            assert_eq!(
                sides(dragged, true, true),
                OffsetSides {
                    out: Some(Copy),
                    inside: Some(Copy)
                },
                "Ctrl+Shift: two new objects"
            );
        }
        let mut new = OffsetValues::default();
        new.set_result(OffsetResultChoice::NewObject);
        assert!(!new.changes_original(false));
        assert!(
            new.changes_original(true),
            "Ctrl switches to the other Mode"
        );
    }

    #[test]
    fn ac26_the_chip_route_reads_direction_and_mode() {
        let mut values = OffsetValues::default();
        values.set_direction(OffsetDirection::Both);
        assert_eq!(
            values.entry_sides(),
            OffsetSides {
                out: Some(OffsetRole::Original),
                inside: Some(OffsetRole::Copy)
            }
        );
    }

    #[test]
    fn ac13_ac56_a_double_click_steps_round_miter_bevel() {
        let mut join = OffsetJoin::Round;
        let mut seen = Vec::new();
        for _ in 0..4 {
            join = next_offset_join(join);
            seen.push(join);
        }
        assert_eq!(
            seen,
            [
                OffsetJoin::Miter,
                OffsetJoin::Bevel,
                OffsetJoin::Round,
                OffsetJoin::Miter
            ]
        );
    }

    #[test]
    fn codes_name_the_values() {
        assert_eq!(offset_join_from_code("miter"), Some(OffsetJoin::Miter));
        assert_eq!(
            offset_direction_from_code("both"),
            Some(OffsetDirection::Both)
        );
        assert_eq!(
            offset_result_from_code("change"),
            Some(OffsetResultChoice::ChangeOriginal)
        );
        assert_eq!(offset_join_from_code("butt"), None);
        assert_eq!(offset_direction_from_code("up"), None);
        assert_eq!(offset_result_from_code("keep"), None);
    }
}
