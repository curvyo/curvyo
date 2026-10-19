//! What each field of a click-to-create placement chip shows: its text, its placeholder and
//! whether its text is refused (`specs/0052-click-to-create-shapes/` criteria 12, 15, 20).

use curvyo_document_core::{DisplayUnit, Length, Shape};

use super::{PlacementField, ShapePlacement, positive, untouched_degrees};
use crate::conversion::is_circle;
use crate::display_unit_text::{decimal_text, format_status_length, parse_length};
use crate::shape_tool_common::CreatePreview;
use crate::transform_entry::{InvalidReason, parse_entry_number};

impl ShapePlacement {
    /// The text `field` shows: the maker's text once edited, otherwise the value of the live
    /// shape (criterion 12), or of the shape kept at the freeze. `live` is ignored once frozen.
    #[must_use]
    pub fn field_text(
        &self,
        field: PlacementField,
        live: Option<CreatePreview>,
        unit: DisplayUnit,
    ) -> String {
        if let Some(text) = self.index(field).and_then(|i| self.edited[i].as_ref()) {
            return text.clone();
        }
        let shape = self.kept_or(live).map(|p| p.shape);
        match field {
            PlacementField::Angle => decimal_text(untouched_degrees(shape.as_ref()), 1),
            _ => self
                .untouched_length(field, shape.as_ref())
                .map_or_else(String::new, |length| format_status_length(length, unit)),
        }
    }

    /// The muted placeholder of an untouched, empty ellipse size field: "rx × ry" in numbers
    /// while the live ellipse is not a circle (criterion 12).
    #[must_use]
    pub fn placeholder(
        &self,
        field: PlacementField,
        live: Option<CreatePreview>,
        unit: DisplayUnit,
    ) -> Option<String> {
        if field != PlacementField::Size
            || self
                .index(field)
                .and_then(|i| self.edited[i].as_ref())
                .is_some()
        {
            return None;
        }
        match self.kept_or(live)?.shape {
            Shape::Ellipse { frame, .. } if !is_circle(&frame) => Some(format!(
                "{} \u{d7} {}",
                format_status_length(frame.rx, unit),
                format_status_length(frame.ry, unit)
            )),
            _ => None,
        }
    }

    /// Why `field`'s edited text is refused, while it is (criteria 15, 23); `None` for an
    /// untouched or valid field.
    #[must_use]
    pub fn field_problem(&self, field: PlacementField, unit: DisplayUnit) -> Option<InvalidReason> {
        let text = self.index(field).and_then(|i| self.edited[i].as_ref())?;
        if field == PlacementField::Angle {
            return parse_entry_number(text, true)
                .is_none()
                .then_some(InvalidReason::NotANumber);
        }
        match parse_length(text, unit) {
            None => Some(InvalidReason::NotANumber),
            Some(length) if !positive(length) => Some(InvalidReason::NotPositive),
            Some(_) => None,
        }
    }

    /// The value an untouched length field shows, in the current r/⌀ mode; `None` for an
    /// ellipse that is not a circle (its field is empty).
    fn untouched_length(&self, field: PlacementField, shape: Option<&Shape>) -> Option<Length> {
        if let (PlacementField::Size, Some(Shape::Ellipse { frame, .. })) = (field, shape)
            && !is_circle(frame)
        {
            return None;
        }
        let length = self.kept_length(field, shape);
        Some(if field == PlacementField::Size && self.diameter {
            Length::from_mm(2.0 * length.as_mm())
        } else {
            length
        })
    }
}
