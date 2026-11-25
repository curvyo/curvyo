//! The typed values of the focus pulse fields set into their ranges, with the notice that says so
//! (`specs/0118` 35 a, ux-notes W60).

use crate::machine_profile::{Limited, PulseMs, PulsePermille, grouped};

/// Per mille as the field shows a percent: one decimal, none for a whole percent ("5", "1.3").
fn percent(permille: u16) -> String {
    if permille.is_multiple_of(10) {
        (permille / 10).to_string()
    } else {
        format!("{}.{}", permille / 10, permille % 10)
    }
}

/// A typed Pulse power in percent, rounded to 0.1 % and set into 0.1 to 5 %; `None` for a value
/// that is not a number.
#[must_use]
pub fn clamp_pulse_power(typed: f64) -> Option<Limited<PulsePermille>> {
    if !typed.is_finite() {
        return None;
    }
    let tenths = (typed * 10.0).round();
    let (min, max) = (f64::from(PulsePermille::MIN), f64::from(PulsePermille::MAX));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: clamped into 1 to 50 before the cast.
    let value = tenths.clamp(min, max) as u16;
    let notice = if (f64::from(value) - tenths).abs() > 0.0 {
        Some(format!("Pulse power limited to {} %.", percent(value)))
    } else if (tenths - typed * 10.0).abs() > 1e-9 {
        Some(format!("Pulse power rounded to {} %.", percent(value)))
    } else {
        None
    };
    Some(Limited {
        value: PulsePermille::new(u64::from(value))?,
        notice,
    })
}

/// A typed Pulse length in milliseconds, rounded and set into 50 to 2,000 ms; `None` for a value
/// that is not a number.
#[must_use]
pub fn clamp_pulse_length(typed: f64) -> Option<Limited<PulseMs>> {
    if !typed.is_finite() {
        return None;
    }
    let rounded = typed.round();
    let (min, max) = (f64::from(PulseMs::MIN), f64::from(PulseMs::MAX));
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    // invariant: clamped into 50 to 2,000 before the cast.
    let value = rounded.clamp(min, max) as u16;
    let shown = grouped(u64::from(value));
    let notice = if (f64::from(value) - rounded).abs() > 0.0 {
        Some(format!("Pulse length limited to {shown} ms."))
    } else if (rounded - typed).abs() > 0.0 {
        Some(format!("Pulse length rounded to {shown} ms."))
    } else {
        None
    };
    Some(Limited {
        value: PulseMs::new(u64::from(value))?,
        notice,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn power(typed: f64) -> (u16, Option<String>) {
        let l = clamp_pulse_power(typed).unwrap();
        (l.value.get(), l.notice)
    }

    fn length(typed: f64) -> (u16, Option<String>) {
        let l = clamp_pulse_length(typed).unwrap();
        (l.value.get(), l.notice)
    }

    #[test]
    fn the_power_is_kept_rounded_or_limited_with_the_notices_of_w60() {
        assert_eq!(power(1.0), (10, None));
        assert_eq!(power(2.5), (25, None));
        assert_eq!(power(9.0), (50, Some("Pulse power limited to 5 %.".into())));
        assert_eq!(
            power(0.0),
            (1, Some("Pulse power limited to 0.1 %.".into()))
        );
        assert_eq!(
            power(1.26),
            (13, Some("Pulse power rounded to 1.3 %.".into()))
        );
        assert_eq!(clamp_pulse_power(f64::NAN), None);
    }

    #[test]
    fn the_length_is_kept_rounded_or_limited_with_the_notices_of_w60() {
        assert_eq!(length(300.0), (300, None));
        assert_eq!(
            length(5_000.0),
            (2_000, Some("Pulse length limited to 2,000 ms.".into()))
        );
        assert_eq!(
            length(10.0),
            (50, Some("Pulse length limited to 50 ms.".into()))
        );
        assert_eq!(
            length(300.6),
            (301, Some("Pulse length rounded to 301 ms.".into()))
        );
        assert_eq!(clamp_pulse_length(f64::INFINITY), None);
        for notice in [
            "Pulse length limited to 2,000 ms.",
            "Pulse power rounded to 1.3 %.",
        ] {
            assert!(notice.len() <= 70);
        }
    }
}
