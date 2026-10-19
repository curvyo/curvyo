//! The words of a click-to-create placement: chip labels, names, hints, messages and the
//! live-region sentences (`specs/0052-click-to-create-shapes/` criteria 11, 23, 29 and 30).

use curvyo_document_core::{DisplayUnit, Length, Shape};

use super::{PlacementField, PlacementKind};
use crate::PolyStarMode;
use crate::conversion::is_circle;
use crate::display_unit_text::{decimal_text, format_status_length};
use crate::transform_entry::InvalidReason;

/// What the placement makes, as the chip's group name and the sentences say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlacementNoun {
    /// A rectangle.
    Rectangle,
    /// An ellipse or circle.
    Ellipse,
    /// A polygon.
    Polygon,
    /// A star.
    Star,
}

impl PlacementNoun {
    /// The noun for `kind`, with the polygon/star bar's mode.
    #[must_use]
    pub const fn of(kind: PlacementKind, mode: PolyStarMode) -> Self {
        match (kind, mode) {
            (PlacementKind::Rect, _) => Self::Rectangle,
            (PlacementKind::Ellipse, _) => Self::Ellipse,
            (PlacementKind::PolyStar, PolyStarMode::Polygon) => Self::Polygon,
            (PlacementKind::PolyStar, PolyStarMode::Star) => Self::Star,
        }
    }

    const fn word(self) -> &'static str {
        match self {
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Polygon => "polygon",
            Self::Star => "star",
        }
    }

    /// The chip's group name (criterion 11): "New rectangle", "New ellipse", ...
    #[must_use]
    pub fn group_name(self) -> String {
        format!("New {}", self.word())
    }

    /// The live region's sentence at the start (criterion 30).
    #[must_use]
    pub fn start_sentence(self) -> String {
        let how = match self {
            Self::Rectangle => "Type the width, Tab for height and angle.",
            Self::Ellipse => "Type the radius, Space for diameter, Tab for angle.",
            Self::Polygon | Self::Star => {
                "Type the outer radius, Space for diameter, Tab for angle."
            }
        };
        format!("New {}. {how} Enter places, Escape cancels.", self.word())
    }
}

/// The visible label of `field` (criterion 11), in diameter mode for the size field when
/// `diameter`.
#[must_use]
pub const fn field_label(field: PlacementField, diameter: bool) -> &'static str {
    match field {
        PlacementField::Width => "W",
        PlacementField::Height => "H",
        PlacementField::Size if diameter => "\u{2300}",
        PlacementField::Size => "r",
        PlacementField::Angle => "Angle",
    }
}

/// The accessible name of `field` of a `kind` chip (criteria 11 and 20).
#[must_use]
pub const fn field_name(
    kind: PlacementKind,
    field: PlacementField,
    diameter: bool,
) -> &'static str {
    match (field, kind, diameter) {
        (PlacementField::Width, ..) => "Width",
        (PlacementField::Height, ..) => "Height",
        (PlacementField::Angle, ..) => "Rotation",
        (PlacementField::Size, PlacementKind::PolyStar, false) => "Outer radius",
        (PlacementField::Size, PlacementKind::PolyStar, true) => "Outer diameter",
        (PlacementField::Size, _, false) => "Radius",
        (PlacementField::Size, _, true) => "Diameter",
    }
}

/// The suffix inside the field's right edge: the display unit, or "°".
#[must_use]
pub const fn field_suffix(field: PlacementField, unit: DisplayUnit) -> &'static str {
    match field {
        PlacementField::Angle => "°",
        _ => unit.symbol(),
    }
}

/// The message line of a refused field (criterion 23).
#[must_use]
pub const fn message(reason: InvalidReason) -> &'static str {
    match reason {
        InvalidReason::NotPositive => "Must be more than 0",
        InvalidReason::TooLarge => "Too large",
        _ => "Enter a number",
    }
}

/// The live region's sentence when it is cancelled (criterion 30).
pub const CANCELLED_SENTENCE: &str = "Cancelled.";

/// The live region's sentence when `shape` was placed (criterion 30): "Rectangle placed, 40 by
/// 20 millimetres.", "Circle placed, radius 10 millimetres." and so on, in `unit`.
#[must_use]
pub fn placed_sentence(shape: &Shape, unit: DisplayUnit) -> String {
    let n = |length: Length| number(length, unit);
    let words = unit_words(unit);
    match shape {
        Shape::Rect { bounds, .. } => format!(
            "Rectangle placed, {} by {} {words}.",
            n(bounds.width),
            n(bounds.height)
        ),
        Shape::Ellipse { frame, .. } if is_circle(frame) => {
            format!("Circle placed, radius {} {words}.", n(frame.rx))
        }
        Shape::Ellipse { frame, .. } => {
            format!(
                "Ellipse placed, {} by {} {words}.",
                n(frame.rx),
                n(frame.ry)
            )
        }
        Shape::Polygon { frame, .. } => {
            format!("Polygon placed, outer radius {} {words}.", n(frame.radius))
        }
        Shape::Star { frame, .. } => {
            format!("Star placed, outer radius {} {words}.", n(frame.radius))
        }
    }
}

/// A length for a sentence: the status bar's decimals without trailing zeros.
pub(crate) fn number(length: Length, unit: DisplayUnit) -> String {
    let fixed = format_status_length(length, unit);
    let decimals = fixed.split_once('.').map_or(0, |(_, d)| d.len());
    decimal_text(length.in_unit(unit), decimals)
}

pub(crate) const fn unit_words(unit: DisplayUnit) -> &'static str {
    match unit {
        DisplayUnit::Mm => "millimetres",
        DisplayUnit::Cm => "centimetres",
        DisplayUnit::In => "inches",
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{EllipseFrame, Point, RectBounds};

    use super::*;

    /// Criterion 30: the sentences, as written.
    #[test]
    fn the_live_region_sentences_are_the_specs() {
        assert_eq!(
            PlacementNoun::Rectangle.start_sentence(),
            "New rectangle. Type the width, Tab for height and angle. Enter places, Escape cancels."
        );
        assert_eq!(
            PlacementNoun::Ellipse.start_sentence(),
            "New ellipse. Type the radius, Space for diameter, Tab for angle. Enter places, \
             Escape cancels."
        );
        assert_eq!(
            PlacementNoun::Star.start_sentence(),
            "New star. Type the outer radius, Space for diameter, Tab for angle. Enter places, \
             Escape cancels."
        );
        let rect = Shape::Rect {
            bounds: RectBounds::from_corners(Point::new(0.0, 0.0), Point::new(40.0, 20.0)),
            corner_radii: curvyo_document_core::CornerRadii::uniform(Length::from_mm(0.0)),
        };
        assert_eq!(
            placed_sentence(&rect, DisplayUnit::Mm),
            "Rectangle placed, 40 by 20 millimetres."
        );
        let circle = Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(10.0),
                ry: Length::from_mm(10.0),
            },
            arc: None,
            curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
        };
        assert_eq!(
            placed_sentence(&circle, DisplayUnit::Mm),
            "Circle placed, radius 10 millimetres."
        );
        let ellipse = Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(0.0, 0.0),
                rx: Length::from_mm(15.0),
                ry: Length::from_mm(5.0),
            },
            arc: None,
            curve: curvyo_document_core::Curve::ELLIPSE_DEFAULT,
        };
        assert_eq!(
            placed_sentence(&ellipse, DisplayUnit::Cm),
            "Ellipse placed, 1.5 by 0.5 centimetres."
        );
    }

    /// Criteria 11, 20, 29: labels, names and hints.
    #[test]
    fn labels_names_and_hints_follow_the_kind_and_the_mode() {
        assert_eq!(field_label(PlacementField::Size, true), "\u{2300}");
        assert_eq!(
            field_name(PlacementKind::PolyStar, PlacementField::Size, false),
            "Outer radius"
        );
        assert_eq!(
            field_name(PlacementKind::Ellipse, PlacementField::Size, true),
            "Diameter"
        );
        assert_eq!(field_suffix(PlacementField::Width, DisplayUnit::Cm), "cm");
        assert_eq!(
            PlacementNoun::of(PlacementKind::PolyStar, PolyStarMode::Star).group_name(),
            "New star"
        );
    }
}
