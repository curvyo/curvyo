//! The warning lines of the Connector row's Controller block (`specs/0127` G19, UX notes section
//! 2): where the machine profile and the controller's reported settings differ.

/// A count with a thousands comma: "1,000".
fn count(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The lines, laser mode first. Laser mode off on the controller is no warning: the connector
/// refuses jobs then (G34). `s_max` is the controller's rounded `$30`; `None` values are unknown.
#[must_use]
pub fn controller_warnings(
    profile_laser_mode: bool,
    profile_max_power: f64,
    laser_mode: Option<bool>,
    s_max: Option<u32>,
) -> Vec<String> {
    let mut lines = Vec::new();
    if laser_mode == Some(true) && !profile_laser_mode {
        lines.push("Laser mode differs: profile off, controller on".to_owned());
    }
    if let Some(s_max) = s_max
        && profile_max_power.is_finite()
        && profile_max_power >= 0.0
    {
        // invariant: finite and non-negative, so the rounded value fits after the clamp.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let profile = profile_max_power.round().min(u64::MAX as f64) as u64;
        if profile != u64::from(s_max) {
            lines.push(format!(
                "Max power differs: profile {}, controller {}",
                count(profile),
                count(u64::from(s_max))
            ));
        }
    }
    lines
}

#[cfg(test)]
#[allow(clippy::assert_is_empty)]
mod tests {
    use super::*;

    #[test]
    fn g19_lines_for_a_differing_profile() {
        assert!(controller_warnings(true, 1000.0, Some(true), Some(1000)).is_empty());
        assert!(controller_warnings(true, 999.6, Some(true), Some(1000)).is_empty());
        assert_eq!(
            controller_warnings(false, 1000.0, Some(true), Some(255)),
            [
                "Laser mode differs: profile off, controller on",
                "Max power differs: profile 1,000, controller 255"
            ]
        );
        // `$32=0` is a refusal (G34), not a warning; unknown values say nothing.
        assert!(controller_warnings(true, 1000.0, Some(false), None).is_empty());
        assert!(controller_warnings(false, 1000.0, None, None).is_empty());
    }
}
