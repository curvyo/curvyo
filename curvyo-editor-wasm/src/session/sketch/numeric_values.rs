//! Numeric entry reads the current geometric proposal only when focus requests it.
use super::{Session, SketchTool};
use curvyo_sketch_core::{Entity, Expression, Quantity};
impl Session {
    /// Current numeric-entry values, in millimetres or displayed degrees.
    /// This is an on-demand read rather than pointer-rate Properties state.
    #[must_use]
    pub fn sketch_numeric_values(&self, action: &str) -> [String; 2] {
        self.sketch_numeric_pair(action)
            .map_or([String::new(), String::new()], |(a, b)| {
                [a.to_string(), b.map_or_else(String::new, |n| n.to_string())]
            })
    }
    fn sketch_numeric_pair(&self, action: &str) -> Option<(f64, Option<f64>)> {
        if self.sketch.tool == SketchTool::Constraints {
            let action = if action.is_empty() {
                self.sketch_dimension_action()?
            } else {
                action
            };
            let snapshot = self.sketch_snapshot_for_edit().ok()?;
            let ids = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
            let value = if action == "angle" {
                Quantity::angle(1.0)
            } else {
                Quantity::length(1.0)
            }
            .ok()?;
            let relation = curvyo_ui_core::sketch_relation(
                action,
                &snapshot.graph,
                &ids,
                Some(&Expression::Literal(value)),
            )
            .ok()?;
            let measured = curvyo_sketch_core::measure(&snapshot.solution, &relation).ok()?;
            return Some((
                if action == "angle" {
                    measured.value().to_degrees()
                } else {
                    measured.value()
                },
                None,
            ));
        }
        let hover = self.sketch.hover?;
        let a = self.sketch_live_picks().first().copied().unwrap_or(hover);
        match self.sketch.tool {
            SketchTool::Point => Some((hover.x.0, Some(hover.y.0))),
            SketchTool::Polyline => Some(self.sketch_chain_values(a, hover)),
            SketchTool::Circle | SketchTool::ArcCenter => Some((
                self.sketch.picks.get(1).map_or_else(
                    || self.sketch.arc_radius.map_or(a.distance(hover).0, |r| r.0),
                    |start| a.distance(*start).0,
                ),
                None,
            )),
            SketchTool::Rectangle => Some((hover.x.0 - a.x.0, Some(hover.y.0 - a.y.0))),
            // A slot spine gives its length and direction before the width (AC39).
            SketchTool::Slot if self.sketch_live_picks().len() < 2 => Some((
                a.distance(hover).0,
                Some((hover.y.0 - a.y.0).atan2(hover.x.0 - a.x.0).to_degrees()),
            )),
            SketchTool::Ellipse | SketchTool::Slot => {
                let b = self.sketch.picks.get(1).copied().unwrap_or(hover);
                let length = a.distance(b).0;
                if length <= 1e-9 {
                    return None;
                }
                let height = ((b.x.0 - a.x.0) * (hover.y.0 - a.y.0)
                    - (b.y.0 - a.y.0) * (hover.x.0 - a.x.0))
                    .abs()
                    / length;
                if self.sketch.tool == SketchTool::Slot {
                    Some((height * 2.0, None))
                } else {
                    Some((length, Some(height)))
                }
            }
            _ => None,
        }
    }
    fn sketch_chain_values(
        &self,
        a: curvyo_sketch_core::Position,
        hover: curvyo_sketch_core::Position,
    ) -> (f64, Option<f64>) {
        let mut end = hover;
        if let (Some(prefix), Some(operation)) = (
            &self.sketch.pending_prefix,
            self.sketch_construction_preview(),
        ) {
            if let Some(Entity::Arc { radius, .. }) = operation
                .candidate
                .entities
                .get(&format!("{prefix}:arc").into())
            {
                return (radius.0, None);
            }
            let key = format!(
                "{prefix}:{}",
                if self.sketch.chain.is_some() {
                    "end"
                } else {
                    "p1"
                }
            );
            end = operation.candidate.point(&key.into()).unwrap_or(hover);
        }
        (
            a.distance(end).0,
            Some((end.y.0 - a.y.0).atan2(end.x.0 - a.x.0).to_degrees()),
        )
    }
}
