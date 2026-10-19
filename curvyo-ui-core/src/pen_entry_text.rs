//! The words of the Pen entry chip: field labels and names, the live-region sentences and the
//! canvas notices (`specs/0072-pen-angle-snap-and-typed-values/` criteria 13, 24 to 29, 34).

use curvyo_document_core::{DisplayUnit, Length};

use crate::display_unit_text::{decimal_text, format_status_length};
use crate::pen_entry::{PenEntryField, PenEntryNotice, PenEntryPlaced};
use crate::shape_placement::{sentence_number, unit_words};

/// The chip's group name (criterion 13).
pub const PEN_ENTRY_GROUP_NAME: &str = "Next node";

/// What the live region says when the chip opens (criterion 34).
pub const PEN_ENTRY_OPENED_SENTENCE: &str = "Next node.";

/// The notice of the one refusal (criterion 26).
pub const PEN_ENTRY_LENGTH_NOT_POSITIVE: &str = "Length must be more than 0.";

/// The visible label inside the field.
#[must_use]
pub const fn pen_entry_label(field: PenEntryField) -> &'static str {
    match field {
        PenEntryField::Length => "Length",
        PenEntryField::Angle => "Angle",
    }
}

/// The accessible name: "Length, millimetres" or "Angle, degrees" (criterion 13).
#[must_use]
pub fn pen_entry_name(field: PenEntryField, unit: DisplayUnit) -> String {
    match field {
        PenEntryField::Length => format!("Length, {}", unit_words(unit)),
        PenEntryField::Angle => "Angle, degrees".to_owned(),
    }
}

/// The suffix inside the field's right edge: the display unit or "°".
#[must_use]
pub const fn pen_entry_suffix(field: PenEntryField, unit: DisplayUnit) -> &'static str {
    match field {
        PenEntryField::Length => unit.symbol(),
        PenEntryField::Angle => "°",
    }
}

/// What the live region says after a node was added: "Node added, 40 millimetres at 0 degrees."
/// (criterion 34).
#[must_use]
pub fn pen_entry_added_sentence(placed: &PenEntryPlaced, unit: DisplayUnit) -> String {
    format!(
        "Node added, {} {} at {} degrees.",
        sentence_number(placed.length, unit),
        unit_words(unit),
        decimal_text(placed.angle.as_radians().to_degrees(), 1)
    )
}

/// The canvas notice of a value that was not used as typed (criteria 24, 25, 28).
#[must_use]
pub fn pen_entry_notice(notice: PenEntryNotice, unit: DisplayUnit) -> String {
    let length =
        |length: Length| format!("{} {}", format_status_length(length, unit), unit.symbol());
    match notice {
        PenEntryNotice::LengthNotANumber(used) => format!("Not a number. Used {}.", length(used)),
        PenEntryNotice::AngleNotANumber(used) => {
            format!(
                "Not a number. Used {}°.",
                decimal_text(used.as_radians().to_degrees(), 1)
            )
        }
        PenEntryNotice::NegativeLength => {
            "Negative length: drawn in the opposite direction.".to_owned()
        }
        PenEntryNotice::TooFar => "Too far away. Length limited to the document limit.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Angle, Point};

    use super::*;

    /// Criteria 13, 24, 29, 34: the texts as written, each notice at most 70 characters.
    #[test]
    fn the_texts_are_the_specs() {
        let unit = DisplayUnit::Mm;
        assert_eq!(
            pen_entry_name(PenEntryField::Length, unit),
            "Length, millimetres"
        );
        assert_eq!(
            pen_entry_suffix(PenEntryField::Length, DisplayUnit::Cm),
            "cm"
        );
        let placed = PenEntryPlaced {
            point: Point::new(140.0, 50.0),
            length: Length::from_mm(40.0),
            angle: Angle::from_radians(0.0),
            notice: None,
        };
        assert_eq!(
            pen_entry_added_sentence(&placed, unit),
            "Node added, 40 millimetres at 0 degrees."
        );
        let notices = [
            PenEntryNotice::LengthNotANumber(Length::from_mm(31.62)),
            PenEntryNotice::AngleNotANumber(Angle::from_radians(-18.43f64.to_radians())),
            PenEntryNotice::NegativeLength,
            PenEntryNotice::TooFar,
        ]
        .map(|notice| pen_entry_notice(notice, unit));
        assert_eq!(notices[0], "Not a number. Used 31.6 mm.");
        assert_eq!(notices[1], "Not a number. Used -18.4°.");
        assert!(notices.iter().all(|text| text.chars().count() <= 70));
    }
}
