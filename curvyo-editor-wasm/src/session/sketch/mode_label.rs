//! The on-canvas mode label names what the active drawing tool draws next (AC34, AC35).
use super::{Session, SketchTool};
use curvyo_sketch_core::{ContinuationMode, Entity, EntityId, Position};
use serde_json::{Value, json};

/// Shown when Alt is pressed after a variant pick (AC35: picks are never discarded silently).
pub(super) const VARIANT_HINT: &str = "Finish the shape or press Escape to change the method";

/// The Alt partner of a construction variant.
pub(super) const fn variant_partner(tool: SketchTool) -> Option<SketchTool> {
    Some(match tool {
        SketchTool::Arc => SketchTool::ArcCenter,
        SketchTool::ArcCenter => SketchTool::Arc,
        SketchTool::Circle => SketchTool::CircleThreePoints,
        SketchTool::CircleThreePoints => SketchTool::Circle,
        SketchTool::Ellipse => SketchTool::EllipseThreePoints,
        SketchTool::EllipseThreePoints => SketchTool::Ellipse,
        _ => return None,
    })
}

/// The domain's name for a tool's current mode; `None` for tools with a single mode.
pub(super) const fn mode_name(tool: SketchTool, mode: ContinuationMode) -> Option<&'static str> {
    Some(match tool {
        SketchTool::Polyline => match mode {
            ContinuationMode::FreeLine => "Line",
            ContinuationMode::PerpendicularLine => "Perpendicular line",
            ContinuationMode::TangentLine => "Tangent line",
            ContinuationMode::TangentArc => "Tangent arc",
            ContinuationMode::PerpendicularArcLeft => "Arc left",
            ContinuationMode::PerpendicularArcRight => "Arc right",
        },
        SketchTool::Arc => "Arc, 3 points",
        SketchTool::ArcCenter => "Arc, centre",
        SketchTool::Circle => "Circle, centre",
        SketchTool::CircleThreePoints => "Circle, 3 points",
        SketchTool::Ellipse => "Ellipse, centre",
        SketchTool::EllipseThreePoints => "Ellipse, 3 points",
        _ => return None,
    })
}

impl Session {
    /// Text and anchor of the mode label: at the start S of the current segment (the last
    /// pick, before the first pick the pointer), on the side away from the rubber band.
    /// `away_x`/`away_y` give the quadrant the host places the label in.
    pub(super) fn sketch_mode_label(&self) -> Value {
        let (Some(text), Some(hover)) = (
            mode_name(self.sketch.tool, self.sketch.mode)
                .or_else(|| self.sketch_dimension_label())
                .or_else(|| self.sketch_constraint_label()),
            self.sketch.hover.or_else(|| self.sketch_hint_anchor()),
        ) else {
            return Value::Null;
        };
        let start = self.sketch.picks.last().copied().unwrap_or(hover);
        let (Some((sx, sy)), Some((hx, hy))) =
            (self.sketch_screen(start), self.sketch_screen(hover))
        else {
            return Value::Null;
        };
        let into = hx > sx && hy > sy;
        let away = if into { -1.0 } else { 1.0 };
        json!({
            "text": text,
            "x": sx,
            "y": sy,
            "away_x": away,
            "away_y": away,
            "hint": self.sketch.variant_hint.then_some(VARIANT_HINT),
            "error": self.sketch_canvas_refusal(),
        })
    }

    /// The dimension tool's label: the kind it makes, which Alt switches for a circle or arc
    /// (AC34), or "Dimension" while the picks fit no kind yet.
    fn sketch_dimension_label(&self) -> Option<&'static str> {
        if self.sketch.tool != SketchTool::Constraints
            || self.sketch.constraint_tool.as_deref() != Some("dimension")
        {
            return None;
        }
        Some(match self.sketch_dimension_action() {
            Some("length") => "Length",
            Some("distance") => "Distance",
            Some("radius") => "Radius",
            Some("diameter") => "Diameter",
            Some(_) => "Angle",
            None => "Dimension",
        })
    }

    /// An armed constraint tool's name while it says what it needs (AC40).
    fn sketch_constraint_label(&self) -> Option<&'static str> {
        self.sketch.constraint_hint.as_ref()?;
        self.sketch
            .constraint_tool
            .as_deref()
            .map(super::rail::label)
    }

    /// Where a constraint tool's hint stands while the pointer is off the canvas (on the rail
    /// button just pressed): at the first selected geometry, else at the sketch origin (AC40).
    fn sketch_hint_anchor(&self) -> Option<Position> {
        self.sketch.constraint_hint.as_ref()?;
        let solution = self.sketch_snapshot_for_edit().ok()?.solution;
        let at = |id: &EntityId| match solution.entities.get(id)? {
            Entity::Point { position, .. } => Some(*position),
            Entity::Line { start, end } => {
                let (a, b) = (solution.point(start).ok()?, solution.point(end).ok()?);
                Some(Position::mm(
                    f64::midpoint(a.x.0, b.x.0),
                    f64::midpoint(a.y.0, b.y.0),
                ))
            }
            Entity::Circle { center, .. }
            | Entity::Arc { center, .. }
            | Entity::Ellipse { center, .. } => solution.point(center).ok(),
        };
        self.sketch
            .entities
            .iter()
            .find_map(at)
            .or_else(|| solution.point(&solution.origin).ok())
    }

    /// A refusal of the last canvas pick, shown in the label in red rather than only in
    /// Sketches (UX notes revision 2 review, changes 2 and 7).
    pub(super) fn sketch_canvas_refusal(&self) -> Option<&str> {
        if self.sketch.tool == SketchTool::Constraints {
            if let Some(hint) = &self.sketch.constraint_hint {
                return Some(hint);
            }
            let picks = self.sketch.entities.len();
            return (self.sketch.constraint_tool.as_deref() == Some("dimension")
                && picks >= 2
                && self.sketch_dimension_action().is_none())
            .then_some(super::relation_tools::DIMENSION_NEEDS);
        }
        mode_name(self.sketch.tool, self.sketch.mode)?;
        // A refused entry speaks in its own box, not twice.
        if self.sketch.entry_error.is_some() {
            return None;
        }
        self.sketch.error.as_deref()
    }
}
