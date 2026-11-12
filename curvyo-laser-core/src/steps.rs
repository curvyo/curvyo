//! Whole-step arithmetic for stored numbers: exact reads, typed rounding and the shared tolerance.

/// How far a stored number may lie from a whole number of its steps and still be exact (the
/// rounding noise of a decimal written as a float).
pub(crate) const STEP_TOLERANCE: f64 = 1e-6;

/// `value` as a whole number of steps of `1 / per_unit`: `None` when it is not finite or not
/// within [`STEP_TOLERANCE`] of a whole step (a stored value is never rounded, ADR 0054 §1).
pub(crate) fn exact_steps(value: f64, per_unit: f64) -> Option<i64> {
    let steps = value * per_unit;
    let whole = steps.round();
    if !steps.is_finite() || (steps - whole).abs() > STEP_TOLERANCE {
        return None;
    }
    whole_steps(whole)
}

/// `value` rounded to whole steps of `1 / per_unit` (a typed value), `None` when not finite.
pub(crate) fn rounded_steps(value: f64, per_unit: f64) -> Option<i64> {
    let steps = (value * per_unit).round();
    if !steps.is_finite() {
        return None;
    }
    whole_steps(steps)
}

/// A typed `value` rounded to whole steps of `1 / per_unit`, `None` when it is not finite or lies
/// outside `min..=max` steps before rounding: a value out of range is refused, never rounded into
/// it (`specs/0111` criterion 24).
pub(crate) fn typed_steps(value: f64, per_unit: f64, min: f64, max: f64) -> Option<i64> {
    let steps = value * per_unit;
    if !(min - STEP_TOLERANCE..=max + STEP_TOLERANCE).contains(&steps) {
        return None;
    }
    rounded_steps(value, per_unit)
}

fn whole_steps(whole: f64) -> Option<i64> {
    // a whole number well inside i64; every range here is a few thousand steps
    (whole.abs() < 1.0e12).then(|| {
        #[allow(clippy::cast_possible_truncation)] // checked above: whole and small
        let steps = whole as i64;
        steps
    })
}
