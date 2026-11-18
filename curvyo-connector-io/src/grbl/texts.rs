//! The words of GRBL 1.1's error and alarm numbers and of the connector's own GRBL sentences
//! (`specs/0127-connector-grbl-serial/ux-notes.md` sections 4 to 6, 10). Plain ASCII.

/// GRBL 1.1's error meanings, `ERRORS[n - 1]`; 18 and 19 are not defined.
const ERRORS: [&str; 38] = [
    "Command letter missing",
    "Bad number",
    "Unknown $ command",
    "Negative value",
    "Homing is not enabled",
    "Step pulse under 3 microseconds",
    "Settings lost, defaults loaded",
    "$ command only when idle",
    "G-code locked by alarm or jog",
    "Soft limits need homing",
    "Line too long",
    "Step rate over 30 kHz",
    "Safety door open",
    "Text over 80 characters",
    "Jog beyond the travel",
    "Invalid jog command",
    "Laser mode needs PWM",
    "",
    "",
    "Unsupported command",
    "Two commands of one modal group",
    "No feed rate",
    "Whole number expected",
    "Two commands need axis words",
    "Word repeated in one line",
    "No axis words",
    "Line number out of range",
    "Value missing",
    "Work offset not supported",
    "G53 only with G0 or G1",
    "Axis words not used",
    "Arc without an axis in its plane",
    "Invalid motion target",
    "Arc radius does not fit",
    "Arc without an offset in its plane",
    "Value words not used",
    "Tool offset on the wrong axis",
    "Tool number too high",
];

/// GRBL 1.1's alarm meanings, `ALARMS[n - 1]`.
const ALARMS: [&str; 9] = [
    "Hard limit hit, position may be lost",
    "Target beyond the soft limits",
    "Reset while moving, position may be lost",
    "Probe already triggered",
    "Probe did not touch",
    "Reset during homing",
    "Door opened during homing",
    "Homing pull-off did not clear the switch",
    "Homing did not find the switch",
];

/// The alarm without a number after power-up or a reset with homing on.
pub const LOCKED_NOT_HOMED: &str = "Locked, not homed";
/// The state `Alarm` with neither a number nor GRBL's unlock message.
pub const LOCKED: &str = "Locked";
/// An `ok` or `error` with no line in flight (`specs/0127` G28).
pub const UNEXPECTED_ANSWER: &str = "Unexpected answer from the controller";

/// What a GRBL error number means, `"error <n>"` for one GRBL 1.1 does not define.
#[must_use]
pub fn error_meaning(code: u16) -> Option<&'static str> {
    usize::from(code)
        .checked_sub(1)
        .and_then(|i| ERRORS.get(i))
        .copied()
        .filter(|m| !m.is_empty())
}

/// `"<meaning> (error <n>)"`, or `"error <n>"` for an unknown number.
#[must_use]
pub fn error_text(code: u16) -> String {
    match error_meaning(code) {
        Some(meaning) => format!("{meaning} (error {code})"),
        None => format!("error {code}"),
    }
}

/// `"<meaning> (alarm <n>)"`, or `"Unknown alarm (alarm <n>)"`.
#[must_use]
pub fn alarm_text(code: u16) -> String {
    let meaning = usize::from(code)
        .checked_sub(1)
        .and_then(|i| ALARMS.get(i))
        .copied()
        .unwrap_or("Unknown alarm");
    format!("{meaning} (alarm {code})")
}

/// A count with a thousands comma: "12,345".
#[must_use]
pub fn count(n: u32) -> String {
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

/// `"Failed at line <n>: <cause>"`.
#[must_use]
pub fn failed_at(line: u32, cause: &str) -> String {
    format!("Failed at line {}: {cause}", count(line))
}

/// `"The controller was reset at line <n>"` (`specs/0127` G29).
#[must_use]
pub fn reset_at(line: u32) -> String {
    format!("The controller was reset at line {}", count(line))
}

/// `"No GRBL answer on <port>"`.
#[must_use]
pub fn no_answer(port: &str) -> String {
    format!("No GRBL answer on {port}")
}

/// GRBL's laser mode is off: the connector starts no job (Frage 240 B).
pub const LASER_MODE_OFF: &str = "Laser mode is off ($32=0)";

/// GRBL's laser mode could not be read (no `$32` in `$$`): the connector starts no job (G34).
pub const LASER_MODE_UNKNOWN: &str = "Laser mode unknown ($32 not read)";

/// "GRBL 0.9 is not supported".
pub const OLD_GRBL: &str = "GRBL 0.9 is not supported";

/// A length in millimetres as the terminal prints it: "400", "406.4".
#[must_use]
pub fn millimetres(value: f64) -> String {
    let tenths = (value * 10.0).round() / 10.0;
    if (tenths - tenths.round()).abs() < 1e-9 {
        format!("{tenths:.0}")
    } else {
        format!("{tenths:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_of_grbl_1_1_has_its_meaning_and_unknown_numbers_say_so() {
        assert_eq!(error_text(22), "No feed rate (error 22)");
        assert_eq!(error_text(1), "Command letter missing (error 1)");
        assert_eq!(error_text(38), "Tool number too high (error 38)");
        assert_eq!(error_text(9), "G-code locked by alarm or jog (error 9)");
        for unknown in [0, 18, 19, 39, 255] {
            assert_eq!(error_text(unknown), format!("error {unknown}"));
        }
        for n in (1..=38).filter(|n| *n != 18 && *n != 19) {
            let meaning = error_meaning(n).unwrap();
            assert!(meaning.is_ascii() && !meaning.contains("try"), "{n}");
            // 5-digit line numbers still fit the 70-character line of `0125`.
            assert!(failed_at(99_999, &error_text(n)).len() <= 69, "{n}");
        }
    }

    #[test]
    fn every_alarm_of_grbl_1_1_has_its_meaning() {
        assert_eq!(
            alarm_text(1),
            "Hard limit hit, position may be lost (alarm 1)"
        );
        assert_eq!(alarm_text(6), "Reset during homing (alarm 6)");
        assert_eq!(alarm_text(9), "Homing did not find the switch (alarm 9)");
        assert_eq!(alarm_text(10), "Unknown alarm (alarm 10)");
        assert_eq!(alarm_text(0), "Unknown alarm (alarm 0)");
        for n in 1..=9 {
            assert!(format!("Machine in alarm: {}", alarm_text(n)).len() <= 70);
        }
    }

    #[test]
    fn the_connector_sentences() {
        assert_eq!(
            failed_at(12, "Longer than 79 characters"),
            "Failed at line 12: Longer than 79 characters"
        );
        assert_eq!(reset_at(12_345), "The controller was reset at line 12,345");
        assert_eq!(no_answer("/dev/ttyUSB0"), "No GRBL answer on /dev/ttyUSB0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1_000), "1,000");
        assert_eq!(millimetres(400.0), "400");
        assert_eq!(millimetres(406.4), "406.4");
        assert_eq!(millimetres(304.80001), "304.8");
        assert!(LASER_MODE_UNKNOWN.len() <= 70 && LASER_MODE_UNKNOWN.is_ascii());
    }
}
