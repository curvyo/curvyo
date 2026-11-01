//! The on-canvas entry box: which fields the current stage takes and where the object is (AC33).
use super::{Session, SketchTool, first_segment::is_arc};
use curvyo_sketch_core::{ContinuationMode, Dimension, Entity, Position};
use serde_json::{Value, json};

/// The refusal of a size of 0 or less, as the Pen and shape entry chips say it.
pub(super) const NOT_POSITIVE: &str = "Must be more than 0 mm";
/// Below this on-screen length in pixels a segment gives no direction to keep the box away from.
const DIRECTION_TOLERANCE_PX: f64 = 1e-9;

/// One field of the entry box: what the typed value sizes or places.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EntryField {
    /// A point's X coordinate.
    X,
    /// A point's Y coordinate.
    Y,
    /// A segment's or slot's length, or a length dimension.
    Length,
    /// A direction against the sketch X axis, or an angle dimension.
    Angle,
    /// A circle's, arc's or fillet's radius, or a radius dimension.
    Radius,
    /// A diameter dimension.
    Diameter,
    /// A distance dimension.
    Distance,
    /// A rectangle's width or a slot's total width.
    Width,
    /// A rectangle's height.
    Height,
    /// An ellipse's semi-axis along the first direction.
    FirstAxis,
    /// An ellipse's other semi-axis.
    SecondAxis,
}

impl EntryField {
    /// The field's visible label.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::X => "X",
            Self::Y => "Y",
            Self::Length => "Length",
            Self::Angle => "Angle",
            Self::Radius => "Radius",
            Self::Diameter => "Diameter",
            Self::Distance => "Distance",
            Self::Width => "Width",
            Self::Height => "Height",
            Self::FirstAxis => "First axis",
            Self::SecondAxis => "Second axis",
        }
    }

    /// The unit the field's text is read in.
    pub(super) const fn dimension(self) -> Dimension {
        match self {
            Self::Angle => Dimension::Angle,
            _ => Dimension::Length,
        }
    }

    /// Whether the value is a size, which must be more than 0.
    const fn is_size(self) -> bool {
        !matches!(self, Self::X | Self::Y | Self::Angle)
    }
}

/// One stage of the entry box before it is placed on the canvas.
struct Stage {
    group: &'static str,
    fields: Vec<EntryField>,
    /// The object point the box sits next to, local millimetres.
    anchor: Position,
    /// The screen direction (y down) the box keeps away to.
    away: (f64, f64),
}

impl Session {
    /// The entry box for the pending construction or placed dimension, or `null`.
    /// Field values are the live proposal in millimetres or degrees, rounded for display.
    pub(super) fn sketch_entry(&self) -> Value {
        if let Some(edit) = self.sketch_edit_entry() {
            return edit;
        }
        let dimension = self.sketch_dimension_stage();
        let kind = if dimension.is_some() {
            "dimension"
        } else {
            "construction"
        };
        let Some(stage) = dimension.or_else(|| self.sketch_construction_stage()) else {
            return Value::Null;
        };
        let Some((x, y)) = self.sketch_screen(stage.anchor) else {
            return Value::Null;
        };
        let pointer = self.sketch.hover.and_then(|p| self.sketch_screen(p));
        // A placed dimension keeps its box while the pointer is off the canvas, on the box
        // itself; with no cursor to keep clear of, the anchor stands in for it.
        let Some((px, py)) = pointer.or((kind == "dimension").then_some((x, y))) else {
            return Value::Null;
        };
        let action = self
            .sketch_dimension_action()
            .filter(|_| kind == "dimension");
        let values = self.sketch_numeric_values(action.unwrap_or(""));
        // A slot's width has no live value before its spine is set: the field starts empty.
        let fields = stage
            .fields
            .iter()
            .zip(values.into_iter().chain(std::iter::repeat(String::new())))
            .map(|(field, value)| {
                let label = field.label();
                json!({
                    "label": label,
                    "name": format!("Sketch {label}"),
                    "suffix": if *field == EntryField::Angle { "°" } else { "mm" },
                    "value": rounded(&value),
                })
            })
            .collect::<Vec<_>>();
        json!({
            "kind": kind,
            "action": action,
            "group": stage.group,
            "fields": fields,
            "x": x,
            "y": y,
            "away_x": stage.away.0,
            "away_y": stage.away.1,
            "pointer_x": px,
            "pointer_y": py,
            "reference": kind == "dimension",
            "error": self.sketch_entry_error(),
        })
    }

    /// How many fields the construction box has now.
    pub(super) fn sketch_entry_field_count(&self) -> usize {
        self.sketch_entry_fields().len()
    }

    /// The construction box's fields now.
    pub(super) fn sketch_entry_fields(&self) -> Vec<EntryField> {
        self.sketch_construction_stage()
            .map_or_else(Vec::new, |stage| stage.fields)
    }

    /// The picks of the shape being drawn; during a press-drag the press is the first (AC39).
    pub(super) fn sketch_live_picks(&self) -> Vec<Position> {
        let mut picks = self.sketch.picks.clone();
        if picks.is_empty()
            && self.button_down
            && let Some(anchor) = self.sketch.press_anchor
        {
            picks.push(anchor);
        }
        picks
    }

    /// The host opened the entry box: a press-drag in progress continues as the follow state,
    /// so its release no longer commits and the typed values do (AC39).
    pub fn sketch_open_entry(&mut self) {
        if self.sketch.picks.is_empty()
            && self.button_down
            && let Some(anchor) = self.sketch.press_anchor.take()
        {
            self.sketch.picks.push(anchor);
            self.sketch.press = None;
            self.button_down = false;
        }
    }

    /// The entry box's refusal as the host shows it: `{field, message}` or `null`.
    pub(super) fn sketch_entry_error(&self) -> Value {
        self.sketch.entry_error.as_ref().map_or(
            Value::Null,
            |e| json!({"field": e.field, "message": e.message}),
        )
    }

    /// The first field of the construction box whose text is refused on its own, with the
    /// reason: not a value of the field's unit, or a size that is not more than 0.
    pub(super) fn sketch_field_refusal(&self, texts: [&str; 2]) -> Option<(usize, String)> {
        let stage = self.sketch_construction_stage()?;
        let graph = self.sketch_intent_for_edit().ok()?;
        let values = graph.parameters.evaluate().ok()?;
        stage
            .fields
            .iter()
            .zip(texts)
            .enumerate()
            .find_map(|(index, (field, text))| {
                let value = super::input::parse_value(text, field.dimension(), &graph.parameters)
                    .and_then(|e| {
                        e.evaluate(&values)
                            .map_err(|e| curvyo_document_core::SketchEditError(e.to_string()))
                    });
                match value {
                    Err(error) => Some((index, error.0)),
                    // Sizes are positive; coordinates and angles take any value.
                    Ok(q) if field.is_size() && q.value() <= 0.0 => {
                        Some((index, NOT_POSITIVE.to_owned()))
                    }
                    Ok(_) => None,
                }
            })
    }

    fn sketch_dimension_stage(&self) -> Option<Stage> {
        if self.sketch.tool != SketchTool::Constraints
            || self.sketch.constraint_tool.as_deref() != Some("dimension")
        {
            return None;
        }
        let anchor = self.sketch.dimension_position?;
        let action = self.sketch_dimension_action()?;
        let field = match action {
            "length" => EntryField::Length,
            "radius" => EntryField::Radius,
            "diameter" => EntryField::Diameter,
            "distance" => EntryField::Distance,
            _ => EntryField::Angle,
        };
        Some(Stage {
            group: "Dimension",
            fields: vec![field],
            anchor,
            away: (1.0, 1.0),
        })
    }

    fn sketch_construction_stage(&self) -> Option<Stage> {
        let hover = self.sketch.hover?;
        let picks = &self.sketch_live_picks();
        let at = |group, fields: &[EntryField], anchor| {
            Some(Stage {
                group,
                fields: fields.to_vec(),
                anchor,
                away: (1.0, 1.0),
            })
        };
        match (self.sketch.tool, picks.as_slice()) {
            (SketchTool::Point, _) => at("Point", &[EntryField::X, EntryField::Y], hover),
            (SketchTool::Polyline, [.., start]) => self.sketch_segment_stage(*start, hover),
            (SketchTool::Circle | SketchTool::ArcCenter, [center]) => {
                let radius = self
                    .sketch
                    .arc_radius
                    .map_or_else(|| center.distance(hover).0, |r| r.0);
                let group = if self.sketch.tool == SketchTool::Circle {
                    "Circle"
                } else {
                    "Arc"
                };
                Some(Stage {
                    group,
                    fields: vec![EntryField::Radius],
                    anchor: Position::mm(center.x.0 + radius, center.y.0),
                    away: (1.0, 0.0),
                })
            }
            (SketchTool::Rectangle, [_]) => {
                at("Rectangle", &[EntryField::Width, EntryField::Height], hover)
            }
            (SketchTool::Ellipse, [_] | [_, _]) => at(
                "Ellipse",
                &[EntryField::FirstAxis, EntryField::SecondAxis],
                hover,
            ),
            (SketchTool::Slot, [_]) => at(
                "Slot",
                &[EntryField::Length, EntryField::Angle, EntryField::Width],
                hover,
            ),
            (SketchTool::Slot, [_, _]) => at("Slot", &[EntryField::Width], hover),
            (SketchTool::Fillet, _) => at(
                "Fillet",
                &[EntryField::Radius],
                self.sketch_fillet_corner()?,
            ),
            _ => None,
        }
    }

    /// A polyline segment's box: at the midpoint of a line or the mid-arc point of an arc,
    /// on the side of the segment the pointer's start label does not use.
    fn sketch_segment_stage(&self, start: Position, hover: Position) -> Option<Stage> {
        let arc = is_arc(self.sketch.mode);
        let (anchor, end) = if arc {
            (self.sketch_preview_mid_arc().unwrap_or(hover), hover)
        } else {
            let end = self.sketch_preview_segment_end().unwrap_or(hover);
            (
                Position::mm(
                    f64::midpoint(start.x.0, end.x.0),
                    f64::midpoint(start.y.0, end.y.0),
                ),
                end,
            )
        };
        let (sx, sy) = self.sketch_screen(start)?;
        let (ex, ey) = self.sketch_screen(end)?;
        let length = (ex - sx).hypot(ey - sy);
        let mut away = if length > DIRECTION_TOLERANCE_PX {
            (-(ey - sy) / length, (ex - sx) / length)
        } else {
            (1.0, 1.0)
        };
        // The label sits on the far side of S from the segment; keep the box below/right of it.
        if away.1 < 0.0 {
            away = (-away.0, -away.1);
        }
        let fields = if arc {
            vec![EntryField::Radius]
        } else if self.sketch.mode == ContinuationMode::FreeLine {
            vec![EntryField::Length, EntryField::Angle]
        } else {
            vec![EntryField::Length]
        };
        Some(Stage {
            group: if arc { "Arc" } else { "Line" },
            fields,
            anchor,
            away,
        })
    }

    /// The previewed end of the pending straight segment.
    fn sketch_preview_segment_end(&self) -> Option<Position> {
        let prefix = self.sketch.pending_prefix.as_ref()?;
        let preview = self.sketch_construction_preview()?;
        ["end", "p1"].iter().find_map(|suffix| {
            preview
                .candidate
                .point(&format!("{prefix}:{suffix}").into())
                .ok()
        })
    }

    /// The middle of the previewed new arc.
    fn sketch_preview_mid_arc(&self) -> Option<Position> {
        let (_, base) = self.sketch.preview_base.as_ref()?;
        let preview = self.sketch_construction_preview()?;
        preview
            .candidate
            .entities
            .iter()
            .find_map(|(id, entity)| match entity {
                Entity::Arc {
                    center,
                    radius,
                    start_angle,
                    end_angle,
                    ..
                } if !base.entities.contains_key(id) => {
                    let c = preview.candidate.point(center).ok()?;
                    let mut sweep = end_angle.0 - start_angle.0;
                    if sweep < 0.0 {
                        sweep += std::f64::consts::TAU;
                    }
                    let mid = start_angle.0 + sweep / 2.0;
                    Some(Position::mm(
                        c.x.0 + radius.0 * mid.cos(),
                        c.y.0 + radius.0 * mid.sin(),
                    ))
                }
                _ => None,
            })
    }

    /// The point two selected segments share, where a fillet rounds them.
    fn sketch_fillet_corner(&self) -> Option<Position> {
        let snapshot = self.sketch_snapshot_for_edit().ok()?;
        let mut ends =
            self.sketch
                .entities
                .iter()
                .filter_map(|id| match snapshot.graph.entities.get(id)? {
                    Entity::Line { start, end } => Some([start.clone(), end.clone()]),
                    _ => None,
                });
        let (a, b) = (ends.next()?, ends.next()?);
        let shared = a.iter().find(|id| b.contains(id))?;
        snapshot.solution.point(shared).ok()
    }
}

/// A live value shortened to three decimals for the field; typed text is never rounded.
pub(super) fn rounded(value: &str) -> String {
    value
        .parse::<f64>()
        .map_or_else(|_| value.to_owned(), entry_number)
}

/// A number with at most three decimals, trailing zeros trimmed, as the entry box shows it.
pub(super) fn entry_number(number: f64) -> String {
    let text = format!("{number:.3}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".into()
    } else {
        text.into()
    }
}
