//! The words of the Print and cut group (`specs/0121-print-and-cut` UX notes §6, §10).

use crate::print_and_cut_reading::{Apply, FitState, MarkRow, PrintAndCut, RowKind, Warning};

/// The group's state line below 2 measures.
pub const MEASURE_AT_LEAST_TWO: &str = "Measure at least 2 marks";
/// The state line when the marks or measures lie together.
pub const TOO_CLOSE: &str = "Marks too close to fit";
/// The Error row of a 2-mark fit.
pub const NO_ERROR_CHECK: &str = "2 marks: no error check";
/// The group's line for an unreadable register.
pub const UNREADABLE: &str = "The stored measures could not be read.";
/// The refusal of a fit off the laser page.
pub const OFF_PAGE: &str = "The fit puts the sheet off the laser page.";
/// The placement line while the sheet is on the fit.
pub const ON_FIT: &str = "On the fit";
/// The placement line after a move by hand.
pub const DIFFERS: &str = "Placement differs from the fit";
/// Apply's reason while greyed on the fit.
pub const SHEET_ON_FIT: &str = "The sheet is on the fit";
/// Reset placement's reason while greyed.
pub const NOTHING_TO_RESET: &str = "Nothing to reset";
/// The Operations row of a marks object.
pub const NOT_CUT: &str = "Registration marks (not cut)";
/// The Sheet line of a second marks object.
pub const NOT_USED: &str = "Not used";
/// The Take head position cause while the machine moves.
pub const MACHINE_MOVING: &str = "Machine is moving";
/// The Take head position cause without a position.
pub const NO_POSITION: &str = "No position from the machine";

/// `value` with `decimals` decimals, never "-0.00".
#[must_use]
pub fn fixed(value: f64, decimals: usize) -> String {
    let text = format!("{value:.decimals$}");
    if text.starts_with('-') && text[1..].chars().all(|c| c == '0' || c == '.') {
        text[1..].to_owned()
    } else {
        text
    }
}

/// The non-breaking space before a unit, so "0.21 mm" never breaks between number and unit.
pub const NBSP: char = '\u{a0}';

/// A length in mm with two decimals: "0.21 mm" (non-breaking space).
#[must_use]
pub fn mm_text(mm: f64) -> String {
    format!("{}{NBSP}mm", fixed(mm, 2))
}

/// Two lengths in mm: "83.40, 41.20 mm" (the Moved row).
#[must_use]
pub fn pair_mm_text(x: f64, y: f64) -> String {
    format!("{}, {}{NBSP}mm", fixed(x, 2), fixed(y, 2))
}

/// A percentage with two decimals: "100.10 %" (non-breaking space).
#[must_use]
pub fn percent_text(pct: f64) -> String {
    format!("{}{NBSP}%", fixed(pct, 2))
}

/// An angle in degrees with two decimals: "1.80°".
#[must_use]
pub fn degrees_text(degrees: f64) -> String {
    format!("{}°", fixed(degrees, 2))
}

/// The line under a row, `None` for a plain mark row.
#[must_use]
pub fn row_note(row: &MarkRow) -> Option<String> {
    match row.kind {
        RowKind::Mark => None,
        RowKind::Gone => Some(format!("Mark {} is gone from the sheet", row.number)),
        RowKind::Twice => Some(format!(
            "Mark {} is on the sheet twice: not used",
            row.number
        )),
    }
}

/// The text of a warning line.
#[must_use]
pub fn warning_text(warning: &Warning) -> String {
    match warning {
        Warning::Scale(pct) => format!(
            "Scale {}: was the sheet printed at 100{NBSP}%?",
            percent_text(*pct)
        ),
        Warning::Mirrored => "The marks look mirrored: check the numbers".to_owned(),
        Warning::SamePlace(a, b) => format!("Marks {a} and {b} are measured at the same place"),
        Warning::HeadPosition => {
            "Start is Head position: the cut will not follow the sheet".to_owned()
        }
    }
}

/// The Error row: `(line 1, line 2)`; line 2 only in the warn state.
#[must_use]
pub fn error_lines(reading: &PrintAndCut) -> Option<(String, Option<String>)> {
    let FitState::Fit(fit) = &reading.fit else {
        return None;
    };
    if fit.pairs == 2 {
        return Some((NO_ERROR_CHECK.to_owned(), None));
    }
    #[allow(clippy::cast_precision_loss)]
    let tolerance = reading.tolerance_um as f64 / 1000.0;
    Some(if fit.over_tolerance {
        (
            format!(
                "Largest error {}; check mark {}",
                mm_text(fit.largest_mm),
                fit.worst_mark
            ),
            Some(format!(
                "RMS {}, tolerance {}",
                mm_text(fit.rms_mm),
                mm_text(tolerance)
            )),
        )
    } else {
        (
            format!(
                "Largest {}, RMS {}",
                mm_text(fit.largest_mm),
                mm_text(fit.rms_mm)
            ),
            None,
        )
    })
}

/// The header's right-hand summary.
#[must_use]
pub fn header_summary(reading: &PrintAndCut) -> String {
    match &reading.fit {
        FitState::TooClose => "Too close".to_owned(),
        FitState::Fit(fit) if fit.pairs > 2 => format!("Error {}", mm_text(fit.largest_mm)),
        _ => {
            let marks = reading.rows.iter().filter(|r| r.kind == RowKind::Mark);
            let total = marks.clone().count();
            let measured = marks.filter(|r| r.measured.is_some()).count();
            if measured == 0 {
                "Not measured".to_owned()
            } else {
                format!("{measured} of {total} measured")
            }
        }
    }
}

/// The state line when there is no fit.
#[must_use]
pub fn state_line(reading: &PrintAndCut) -> Option<&'static str> {
    match reading.fit {
        FitState::NotEnough => Some(MEASURE_AT_LEAST_TWO),
        FitState::TooClose => Some(TOO_CLOSE),
        FitState::Fit(_) => None,
    }
}

/// Apply's label and, while greyed, its reason; `None` while Apply is not offered at all.
#[must_use]
pub fn apply_button(reading: &PrintAndCut) -> Option<(&'static str, Option<&'static str>)> {
    match reading.apply {
        Apply::Unavailable => Some((
            "Apply",
            Some(match reading.fit {
                FitState::TooClose => TOO_CLOSE,
                _ => MEASURE_AT_LEAST_TWO,
            }),
        )),
        Apply::OnFit => Some(("Apply", Some(SHEET_ON_FIT))),
        Apply::Offered { again } => Some((if again { "Apply again" } else { "Apply" }, None)),
        Apply::NotUsable(_) => None,
    }
}

/// "The fit is not usable: scale 31.0 %." while Apply is not offered.
#[must_use]
pub fn not_usable(reading: &PrintAndCut) -> Option<String> {
    match reading.apply {
        Apply::NotUsable(pct) => Some(format!(
            "The fit is not usable: scale {}.",
            percent_text(pct)
        )),
        _ => None,
    }
}
