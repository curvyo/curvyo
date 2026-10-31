//! The on-canvas mode label names what the active drawing tool draws next (AC34, AC35).
use super::{Session, SketchTool};
use curvyo_sketch_core::ContinuationMode;
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
            mode_name(self.sketch.tool, self.sketch.mode),
            self.sketch.hover,
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
        })
    }
}
