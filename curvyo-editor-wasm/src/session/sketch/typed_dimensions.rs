//! Sizes typed into the entry box become driving dimensions of the shape they create (AC39).
use super::{Session, SketchTool, entry::EntryField};
use curvyo_document_core::SketchEditError;
use curvyo_sketch_core::{
    Constraint, ConstraintId, Construction, Dimension, EllipseAxis, EntityId, Expression, Position,
    Relation,
};

impl Session {
    /// Completes the entry box: `texts` per field and which of them the maker `typed`. Untyped
    /// fields keep the pointer's value and stay free; each typed size becomes a driving
    /// dimension (its typed expression) of the new shape in the same history step (AC39).
    /// # Errors
    /// As [`Session::sketch_complete_numeric`].
    pub fn sketch_complete_fields(
        &mut self,
        texts: &[&str],
        typed: &[bool],
    ) -> Result<bool, SketchEditError> {
        let fields = self.sketch_entry_fields();
        let graph = self.sketch_intent_for_edit()?;
        let mut dimensions = Vec::new();
        for ((field, text), typed) in fields.iter().zip(texts).zip(typed) {
            if !typed {
                continue;
            }
            // A refused text is reported by the value path below, at its field.
            if let Ok(expression) =
                super::input::parse_value(text, field.dimension(), &graph.parameters)
            {
                dimensions.push((*field, expression));
            }
        }
        // A typed centre-arc radius or slot spine waits for the next pick; earlier typed
        // sizes of the same shape stay.
        self.sketch.typed_dimensions.extend(dimensions);
        let first = texts.first().copied().unwrap_or("");
        let second = texts.get(1).copied().unwrap_or("");
        let result = if self.sketch.tool == SketchTool::Slot && fields.len() == 3 {
            self.sketch_slot_spine(first, second, texts.get(2).copied().unwrap_or(""))
        } else {
            self.sketch_complete_numeric(first, second)
        };
        // A commit consumed the sizes; a refusal drops them with the box's values.
        if result.is_err() {
            self.sketch.typed_dimensions.clear();
        }
        result
    }

    /// A slot's spine from its typed or live length and angle: with a width the slot is
    /// complete, without one the width pick follows (AC39).
    fn sketch_slot_spine(
        &mut self,
        length: &str,
        angle: &str,
        width: &str,
    ) -> Result<bool, SketchEditError> {
        let start = self
            .sketch_live_picks()
            .first()
            .copied()
            .ok_or_else(|| SketchEditError("Pick the construction anchor first".into()))?;
        let graph = self.sketch_intent_for_edit()?;
        let values = graph
            .parameters
            .evaluate()
            .map_err(|e| SketchEditError(e.to_string()))?;
        let value = |text, unit| {
            super::input::parse_value(text, unit, &graph.parameters)?
                .evaluate(&values)
                .map(curvyo_sketch_core::Quantity::value)
                .map_err(|e| SketchEditError(e.to_string()))
        };
        let refuse = |this: &mut Self, field: usize, error: SketchEditError| {
            this.sketch.entry_error = Some(super::state::EntryError {
                field: Some(field),
                message: error.0.clone(),
            });
            error
        };
        let length = match value(length, Dimension::Length) {
            Ok(length) if length > 0.0 => length,
            Ok(_) => {
                let error = SketchEditError(super::entry::NOT_POSITIVE.into());
                return Err(refuse(self, 0, error));
            }
            Err(error) => return Err(refuse(self, 0, error)),
        };
        let angle = value(angle, Dimension::Angle).map_err(|error| refuse(self, 1, error))?;
        let end = Position::mm(
            start.x.0 + length * angle.cos(),
            start.y.0 + length * angle.sin(),
        );
        self.sketch.picks = vec![start, end];
        self.sketch.press = None;
        self.button_down = false;
        self.sketch.entry_error = None;
        if width.trim().is_empty() {
            self.refresh_sketch_construction_preview();
            return Ok(true);
        }
        self.sketch_complete_numeric(width, "")
    }

    /// The driving dimensions the typed sizes give the shape `command` creates under `prefix`;
    /// `curve` is the segment a polyline step creates.
    pub(super) fn sketch_typed_relations(
        &self,
        prefix: &str,
        command: &Construction,
        curve: Option<&EntityId>,
    ) -> Vec<(ConstraintId, Constraint)> {
        let id = |name: &str| EntityId::from(format!("{prefix}:{name}"));
        let polyline = self.sketch.tool == SketchTool::Polyline;
        self.sketch
            .typed_dimensions
            .iter()
            .filter_map(|(field, value)| {
                let value = value.clone();
                let (key, relation) = match command {
                    _ if polyline => segment(*field, value, curve?.clone())?,
                    Construction::Rectangle(..) => rectangle(*field, value, id)?,
                    Construction::Circle { .. } | Construction::ArcCenter { .. } => {
                        round(*field, value, id("curve"))?
                    }
                    Construction::Slot { .. } => slot(*field, value, id)?,
                    Construction::Ellipse {
                        center,
                        axis_end,
                        second_axis,
                    } => {
                        let first_is_major = center.distance(*axis_end).0 >= second_axis.0;
                        ellipse(*field, value, id("curve"), first_is_major)?
                    }
                    _ => return None,
                };
                Some((
                    format!("{prefix}:{key}").into(),
                    Constraint::driving(relation),
                ))
            })
            .collect()
    }
}

/// A typed size as the record key it is stored under and its relation.
type Typed = (&'static str, Relation);

/// A polyline segment: its length, an arc's radius, a line's direction.
fn segment(field: EntryField, value: Expression, curve: EntityId) -> Option<Typed> {
    Some(match field {
        EntryField::Length => ("typed-length", Relation::Length { curve, value }),
        EntryField::Radius => ("typed-radius", Relation::Radius { curve, value }),
        EntryField::Angle => ("typed-angle", Relation::AxisAngle { line: curve, value }),
        _ => return None,
    })
}

/// A rectangle's width (first side) and height (second side).
fn rectangle(field: EntryField, value: Expression, id: impl Fn(&str) -> EntityId) -> Option<Typed> {
    Some(match field {
        EntryField::Width => (
            "typed-width",
            Relation::Length {
                curve: id("line0"),
                value,
            },
        ),
        EntryField::Height => (
            "typed-height",
            Relation::Length {
                curve: id("line1"),
                value,
            },
        ),
        _ => return None,
    })
}

/// A circle's or centre arc's radius.
fn round(field: EntryField, value: Expression, curve: EntityId) -> Option<Typed> {
    (field == EntryField::Radius).then_some(("typed-radius", Relation::Radius { curve, value }))
}

/// A slot always carries its centre spacing and cap size; typed values replace them (AC15). Its
/// direction is its top side's, which runs from the start cap to the end cap.
fn slot(field: EntryField, value: Expression, id: impl Fn(&str) -> EntityId) -> Option<Typed> {
    Some(match field {
        EntryField::Length => (
            "spacing",
            Relation::Distance {
                a: id("c0"),
                b: id("c1"),
                value,
            },
        ),
        EntryField::Width => (
            "radius",
            Relation::Diameter {
                curve: id("left"),
                value,
            },
        ),
        EntryField::Angle => (
            "typed-angle",
            Relation::AxisAngle {
                line: id("top"),
                value,
            },
        ),
        _ => return None,
    })
}

/// An ellipse's semi-axes: the longer typed or live axis is the major one.
fn ellipse(
    field: EntryField,
    value: Expression,
    curve: EntityId,
    first_is_major: bool,
) -> Option<Typed> {
    let axis = match (field, first_is_major) {
        (EntryField::FirstAxis, true) | (EntryField::SecondAxis, false) => EllipseAxis::Major,
        (EntryField::FirstAxis, false) | (EntryField::SecondAxis, true) => EllipseAxis::Minor,
        _ => return None,
    };
    let key = match field {
        EntryField::FirstAxis => "typed-first-axis",
        _ => "typed-second-axis",
    };
    Some((key, Relation::EllipseRadius { curve, axis, value }))
}
