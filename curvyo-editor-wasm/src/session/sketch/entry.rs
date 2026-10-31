//! The on-canvas entry box: which fields the current stage takes and where the object is (AC33).
use super::{Session, SketchTool, first_segment::is_arc};
use curvyo_sketch_core::{ContinuationMode, Entity, Position};
use serde_json::{Value, json};

/// One stage of the entry box before it is placed on the canvas.
struct Stage {
    group: &'static str,
    labels: Vec<&'static str>,
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
        let (Some((x, y)), Some((px, py))) = (
            self.sketch_screen(stage.anchor),
            self.sketch.hover.and_then(|p| self.sketch_screen(p)),
        ) else {
            return Value::Null;
        };
        let action = self
            .sketch_dimension_action()
            .filter(|_| kind == "dimension");
        let values = self.sketch_numeric_values(action.unwrap_or(""));
        let fields = stage
            .labels
            .iter()
            .zip(values)
            .map(|(label, value)| {
                json!({
                    "label": label,
                    "name": format!("Sketch {label}"),
                    "suffix": if *label == "Angle" { "°" } else { "mm" },
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
        let label = match action {
            "length" => "Length",
            "radius" => "Radius",
            "diameter" => "Diameter",
            "distance" => "Distance",
            _ => "Angle",
        };
        Some(Stage {
            group: "Dimension",
            labels: vec![label],
            anchor,
            away: (1.0, 1.0),
        })
    }

    fn sketch_construction_stage(&self) -> Option<Stage> {
        let hover = self.sketch.hover?;
        let picks = &self.sketch.picks;
        let at = |group, labels: &[&'static str], anchor| {
            Some(Stage {
                group,
                labels: labels.to_vec(),
                anchor,
                away: (1.0, 1.0),
            })
        };
        match (self.sketch.tool, picks.as_slice()) {
            (SketchTool::Point, _) => at("Point", &["X", "Y"], hover),
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
                    labels: vec!["Radius"],
                    anchor: Position::mm(center.x.0 + radius, center.y.0),
                    away: (1.0, 0.0),
                })
            }
            (SketchTool::Rectangle, [_]) => at("Rectangle", &["Width", "Height"], hover),
            (SketchTool::Ellipse, [_] | [_, _]) => {
                at("Ellipse", &["First axis", "Second axis"], hover)
            }
            (SketchTool::Slot, [_, _]) => at("Slot", &["Width"], hover),
            (SketchTool::Fillet, _) => at("Fillet", &["Radius"], self.sketch_fillet_corner()?),
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
        let mut away = if length > 1e-9 {
            (-(ey - sy) / length, (ex - sx) / length)
        } else {
            (1.0, 1.0)
        };
        // The label sits on the far side of S from the segment; keep the box below/right of it.
        if away.1 < 0.0 {
            away = (-away.0, -away.1);
        }
        let labels = if arc {
            vec!["Radius"]
        } else if self.sketch.mode == ContinuationMode::FreeLine {
            vec!["Length", "Angle"]
        } else {
            vec!["Length"]
        };
        Some(Stage {
            group: if arc { "Arc" } else { "Line" },
            labels,
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
fn rounded(value: &str) -> String {
    value.parse::<f64>().map_or_else(
        |_| value.to_owned(),
        |number| {
            let text = format!("{number:.3}");
            let text = text.trim_end_matches('0').trim_end_matches('.');
            if text == "-0" {
                "0".into()
            } else {
                text.into()
            }
        },
    )
}
