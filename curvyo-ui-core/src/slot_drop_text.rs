//! The words a library item put into a slot shows (`specs/0154-library-into-slots` criteria 1,
//! 5, 7, 8; UX notes sections 3, 5, 6).

use std::fmt::Write;

use curvyo_laser_core::SlotFit;

/// A slot or holder name is cut to this many characters with an ellipsis (UX notes section 6).
const NAME_CHARS: usize = 24;
/// Scales this close are one scale in a fill's words.
const SAME_SCALE: f64 = 1e-9;

/// The badge of a source with several pages over a slot (UX notes 3.4).
pub const ONE_PAGE_ONLY_TEXT: &str = "Only one page goes into a slot.";
/// The badge reason while the Fixtures snap switch is off (criterion 9).
pub const FIXTURE_SNAP_OFF_TEXT: &str = "Fixture snapping is off.";

/// `name` cut at 24 characters with an ellipsis.
#[must_use]
pub fn short_name(name: &str) -> String {
    if name.chars().count() <= NAME_CHARS {
        return name.to_owned();
    }
    let mut cut: String = name.chars().take(NAME_CHARS - 1).collect();
    cut.push('…');
    cut
}

/// A scale in words: whole percent, one decimal under 10 %, a no-break space before "%".
#[must_use]
pub fn scale_words(scale: f64) -> String {
    let percent = scale * 100.0;
    if percent < 9.95 {
        format!("{percent:.1}\u{a0}%")
    } else {
        format!("{percent:.0}\u{a0}%")
    }
}

/// What the unit becomes in the slot, for the badge: "50 %", "67 %, turned", "turned",
/// "real size", "real size, too large", or nothing for a plain centre.
#[must_use]
pub fn outcome_words(fit: &SlotFit, real_size: bool) -> Option<String> {
    if fit.matched {
        return fit.turned.then(|| "turned".to_owned());
    }
    if real_size {
        return Some(if fit.overflows {
            "real size, too large".to_owned()
        } else {
            "real size".to_owned()
        });
    }
    let scale = scale_words(fit.scale);
    Some(if fit.turned {
        format!("{scale}, turned")
    } else {
        scale
    })
}

/// The outcome cell of a slot row of the Into slot list (UX notes 8.2): as the badge, but "too
/// large" alone when the real size does not fit.
#[must_use]
pub fn row_outcome_words(fit: &SlotFit, real_size: bool) -> Option<String> {
    if real_size && !fit.matched && fit.overflows {
        return Some("too large".to_owned());
    }
    outcome_words(fit, real_size)
}

/// The Drop badge over a target slot: "Into Slot 2 · 50 %", with " of Holder 1" when two target
/// slots share the name.
#[must_use]
pub fn into_slot_badge(slot: &str, holder: Option<&str>, fit: &SlotFit, real_size: bool) -> String {
    let mut text = format!("Into {}", short_name(slot));
    if let Some(holder) = holder {
        let _ = write!(text, " of {}", short_name(holder));
    }
    if let Some(outcome) = outcome_words(fit, real_size) {
        let _ = write!(text, " · {outcome}");
    }
    text
}

/// The polite status of the drag over a target slot (UX notes 3.1): "Into Slot 2 of Holder 1,
/// scaled to 50 percent."
#[must_use]
pub fn into_slot_status(slot: &str, holder: &str, fit: &SlotFit, real_size: bool) -> String {
    let base = format!("Into {} of {}", short_name(slot), short_name(holder));
    // Spoken with the badge's rounding ("6.7 percent", UX review).
    let percent = || scale_words(fit.scale).replace("\u{a0}%", " percent");
    let tail = if fit.matched {
        if fit.turned {
            ", turned".to_owned()
        } else {
            String::new()
        }
    } else if real_size {
        if fit.overflows {
            ", real size, too large".to_owned()
        } else {
            ", real size".to_owned()
        }
    } else if fit.turned {
        format!(", scaled to {} and turned", percent())
    } else {
        format!(", scaled to {}", percent())
    };
    format!("{base}{tail}.")
}

/// The polite status of a Shift drag (UX notes 3.1): "Fill 12 of 15 slots of Holder 1."
#[must_use]
pub fn fill_status(empty: usize, of: usize, holder: &str) -> String {
    format!(
        "Fill {empty} of {of} {} of {}.",
        slots_word(of),
        short_name(holder)
    )
}

/// The notice after a drop that was not a plain centre (criterion 5), or none.
#[must_use]
pub fn fit_notice(slot: &str, fit: &SlotFit, real_size: bool) -> Option<String> {
    let slot = short_name(slot);
    if fit.matched {
        return fit.turned.then(|| format!("Turned to fit {slot}."));
    }
    if real_size {
        return fit
            .overflows
            .then(|| format!("Real size: larger than {slot}."));
    }
    let scale = scale_words(fit.scale);
    Some(if fit.turned {
        format!("Fitted into {slot} at {scale}, turned.")
    } else {
        format!("Fitted into {slot} at {scale}.")
    })
}

/// The refusal of a drop onto a filled slot (criterion 7).
#[must_use]
pub fn slot_filled_text(slot: &str) -> String {
    format!("{} is filled.", short_name(slot))
}

/// The badge and the notice of a fill with no empty slot (criterion 8).
#[must_use]
pub fn no_empty_slot_text(holder: &str) -> String {
    format!("No empty slot in {}.", short_name(holder))
}

/// The scales of a fill in words: "50 %", "48 to 100 %", or none when nothing is scaled.
fn scales_words(scales: &[f64]) -> Option<String> {
    let lo = scales.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = scales.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() || ((lo - 1.0).abs() < SAME_SCALE && (hi - 1.0).abs() < SAME_SCALE) {
        return None;
    }
    if (hi - lo).abs() < SAME_SCALE {
        return Some(scale_words(lo));
    }
    let low = scale_words(lo);
    let low = low.trim_end_matches("\u{a0}%");
    Some(format!("{low} to {}", scale_words(hi)))
}

/// The badge of a Shift drag: "Fill 12 of 15 slots", "· 50 %", "· 48 to 100 %", "· real size".
#[must_use]
pub fn fill_badge(empty: usize, of: usize, scales: &[f64], real_size: bool) -> String {
    let mut text = format!("Fill {empty} of {of} {}", slots_word(of));
    let tail = if real_size {
        Some("real size".to_owned())
    } else {
        scales_words(scales)
    };
    if let Some(tail) = tail {
        let _ = write!(text, " · {tail}");
    }
    text
}

/// The notice after a fill (criterion 8), or none for a fill without scale and without skips.
#[must_use]
pub fn fill_notice(
    filled: usize,
    skipped: usize,
    scales: &[f64],
    real_size: bool,
) -> Option<String> {
    let scale = if real_size {
        None
    } else {
        scales_words(scales)
    };
    if scale.is_none() && skipped == 0 {
        return None;
    }
    let mut text = format!("Filled {filled} {}", slots_word(filled));
    if let Some(scale) = scale {
        let _ = write!(text, " at {scale}");
    }
    if skipped > 0 {
        let verb = if skipped == 1 { "was" } else { "were" };
        let _ = write!(text, "; {skipped} {verb} already filled");
    }
    text.push('.');
    Some(text)
}

const fn slots_word(count: usize) -> &'static str {
    if count == 1 { "slot" } else { "slots" }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn fit(scale: f64, turned: bool, matched: bool, overflows: bool) -> SlotFit {
        SlotFit {
            scale,
            turned,
            matched,
            overflows,
        }
    }

    #[test]
    fn the_badge_says_the_slot_and_the_scale() {
        let half = fit(0.5, false, false, false);
        assert_eq!(
            into_slot_badge("Slot 2", None, &half, false),
            "Into Slot 2 · 50\u{a0}%"
        );
        assert_eq!(
            into_slot_badge(
                "Slot 2",
                Some("Holder 1"),
                &fit(0.667, true, false, false),
                false
            ),
            "Into Slot 2 of Holder 1 · 67\u{a0}%, turned"
        );
        assert_eq!(
            into_slot_badge("Slot 2", None, &fit(1.0, false, true, false), false),
            "Into Slot 2"
        );
        assert_eq!(
            into_slot_badge("Slot 2", None, &fit(1.0, true, true, false), false),
            "Into Slot 2 · turned"
        );
        assert_eq!(
            into_slot_badge("Slot 2", None, &fit(1.0, false, false, false), true),
            "Into Slot 2 · real size"
        );
        assert_eq!(
            into_slot_badge("Slot 2", None, &fit(1.0, false, false, true), true),
            "Into Slot 2 · real size, too large"
        );
    }

    #[test]
    fn scale_words_are_whole_percent_with_one_decimal_under_ten() {
        assert_eq!(scale_words(0.045), "4.5\u{a0}%");
        assert_eq!(scale_words(4.0), "400\u{a0}%");
        assert_eq!(scale_words(0.6667), "67\u{a0}%");
    }

    #[test]
    fn the_notices_of_criterion_5_and_8_stay_short() {
        assert_eq!(
            fit_notice("Slot 2", &fit(0.5, false, false, false), false).unwrap(),
            "Fitted into Slot 2 at 50\u{a0}%."
        );
        assert_eq!(
            fit_notice("Slot 2", &fit(1.0, true, true, false), false).unwrap(),
            "Turned to fit Slot 2."
        );
        assert_eq!(
            fit_notice("Slot 2", &fit(1.0, false, true, false), false),
            None
        );
        assert_eq!(
            fit_notice("Slot 2", &fit(1.0, false, false, false), true),
            None
        );
        assert_eq!(
            fit_notice("Slot 2", &fit(1.0, false, false, true), true).unwrap(),
            "Real size: larger than Slot 2."
        );
        assert_eq!(
            fill_notice(11, 4, &[0.5; 11], false).unwrap(),
            "Filled 11 slots at 50\u{a0}%; 4 were already filled."
        );
        assert_eq!(
            fill_notice(11, 4, &[1.0; 11], false).unwrap(),
            "Filled 11 slots; 4 were already filled."
        );
        assert_eq!(fill_notice(12, 0, &[1.0; 12], false), None);
        assert_eq!(
            fill_notice(2, 0, &[0.48, 1.0], false).unwrap(),
            "Filled 2 slots at 48 to 100\u{a0}%."
        );
        let long = "A slot with a very long name indeed, really";
        let text = fit_notice(long, &fit(0.674, true, false, false), false).unwrap();
        assert!(text.chars().count() <= 70, "{text}");
        assert_eq!(slot_filled_text("Slot 2"), "Slot 2 is filled.");
        assert_eq!(no_empty_slot_text("Holder 1"), "No empty slot in Holder 1.");
        assert_eq!(
            fill_badge(12, 15, &[0.5; 12], false),
            "Fill 12 of 15 slots · 50\u{a0}%"
        );
        assert_eq!(
            fill_badge(12, 15, &[], true),
            "Fill 12 of 15 slots · real size"
        );
    }

    #[test]
    fn the_spoken_status_says_the_slot_holder_and_scale_with_the_badges_rounding() {
        assert_eq!(
            into_slot_status("Slot 2", "Holder 1", &fit(0.5, false, false, false), false),
            "Into Slot 2 of Holder 1, scaled to 50 percent."
        );
        assert_eq!(
            into_slot_status(
                "Slot 2",
                "Holder 1",
                &fit(0.0667, false, false, false),
                false
            ),
            "Into Slot 2 of Holder 1, scaled to 6.7 percent."
        );
        assert_eq!(
            fill_status(12, 15, "Holder 1"),
            "Fill 12 of 15 slots of Holder 1."
        );
    }
}
