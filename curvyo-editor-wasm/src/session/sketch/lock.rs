//! Ctrl locks a free drawing direction to the shared angle stops from the sketch X axis.
use super::{Session, SketchTool, inspection::SketchAnnotation};
use curvyo_document_core::{Angle, Point};
use curvyo_sketch_core::{
    Constraint, ConstraintId, Construction, ContinuationMode, EntityId, Position, Relation,
};
use std::f64::consts::FRAC_PI_2;

/// How far the dashed lock guide runs past the anchor and the pointer, in screen pixels.
const GUIDE_OVERRUN_PX: f64 = 24.0;

impl Session {
    /// The pointer position with Ctrl's direction lock applied, when it applies (AC28).
    /// Records the locked angle; a capture or a mode-fixed direction disengages it.
    pub(super) fn sketch_lock(&mut self, local: Position) -> Option<Position> {
        self.sketch.lock = None;
        if !self.held.ctrl || self.sketch.capture.is_some() {
            return None;
        }
        let anchor = self.sketch_lock_anchor()?;
        let (dx, dy) = (local.x.0 - anchor.x.0, local.y.0 - anchor.y.0);
        let length = dx.hypot(dy);
        if length <= 1e-9 {
            return None;
        }
        let angle = curvyo_ui_core::snap_angle(Angle::from_radians(dy.atan2(dx))).as_radians();
        self.sketch.lock = Some(angle);
        Some(Position::mm(
            anchor.x.0 + length * angle.cos(),
            anchor.y.0 + length * angle.sin(),
        ))
    }

    /// The start of a pick whose direction is free: a free-line segment or a slot centreline.
    fn sketch_lock_anchor(&self) -> Option<Position> {
        let free = match self.sketch.tool {
            SketchTool::Polyline => self.sketch.mode == ContinuationMode::FreeLine,
            SketchTool::Slot => self.sketch.picks.len() == 1,
            _ => false,
        };
        if !free {
            return None;
        }
        self.sketch.picks.last().copied().or_else(|| {
            self.button_down
                .then_some(self.sketch.press_anchor)
                .flatten()
        })
    }

    /// The locked direction the pending commit persists, if any.
    fn sketch_committed_lock(&self) -> Option<f64> {
        if self.sketch.tool == SketchTool::Slot && self.sketch.picks.len() >= 2 {
            return self.sketch.spine_lock;
        }
        self.sketch.lock
    }

    /// Horizontal (0, 180 degrees) or Vertical (90, 270) for a lock on an axis stop;
    /// every other stop only positions the point (customer decision Q2).
    fn sketch_lock_relation(
        &self,
        prefix: &str,
        command: &Construction,
    ) -> Option<(ConstraintId, Constraint)> {
        let vertical = axis_stop(self.sketch_committed_lock()?)?;
        let line = match command {
            Construction::Polyline(_) => "line0",
            Construction::Segment { .. } | Construction::Continue { .. } => "line",
            Construction::Slot { .. } => "top",
            _ => return None,
        };
        let line = EntityId::from(format!("{prefix}:{line}"));
        let relation = if vertical {
            Relation::Vertical(line)
        } else {
            Relation::Horizontal(line)
        };
        Some((
            format!("{prefix}:lock").into(),
            Constraint::driving(relation),
        ))
    }

    /// Adds the lock's Horizontal or Vertical to a step that carries no mode relation.
    pub(super) fn with_lock_relation(
        &self,
        prefix: &str,
        mut step: super::first_segment::PolylineStep,
    ) -> super::first_segment::PolylineStep {
        if step.relation.is_none() {
            step.relation = self.sketch_lock_relation(prefix, &step.command);
        }
        step
    }

    /// The pending relation at the segment midpoint and the locked length and angle at the
    /// pointer, both shown before the commit.
    pub(super) fn sketch_lock_annotations(&self) -> Vec<SketchAnnotation> {
        let mut annotations = Vec::new();
        let Some(hover) = self.sketch.hover else {
            return annotations;
        };
        let segment = if self.sketch.tool == SketchTool::Slot && self.sketch.picks.len() >= 2 {
            Some((self.sketch.picks[0], self.sketch.picks[1]))
        } else {
            self.sketch_lock_anchor().map(|a| (a, hover))
        };
        let (Some(angle), Some((a, b))) = (self.sketch_committed_lock(), segment) else {
            return annotations;
        };
        if let Some(vertical) = axis_stop(angle)
            && let Some((x, y)) = self.sketch_screen(Position::mm(
                f64::midpoint(a.x.0, b.x.0),
                f64::midpoint(a.y.0, b.y.0),
            ))
        {
            annotations.push(preview(
                "pending-lock-relation",
                x,
                y - 14.0,
                if vertical { "Vertical" } else { "Horizontal" },
            ));
        }
        if self.sketch.lock.is_some()
            && let Some((x, y)) = self.sketch_screen(hover)
        {
            let degrees = angle.to_degrees().rem_euclid(360.0);
            let text = format!("{:.1} mm, {}°", a.distance(b).0, trimmed(degrees));
            annotations.push(preview("pending-lock-readout", x + 40.0, y + 22.0, &text));
        }
        annotations
    }

    /// The dashed guide through the anchor along the locked direction (UX notes revision 2, 5).
    pub(super) fn sketch_lock_draw_list(&self) -> curvyo_render_core::DrawList {
        let (Some(angle), Some(anchor), Some(hover), Some(placement)) = (
            self.sketch.lock,
            self.sketch_lock_anchor(),
            self.sketch.hover,
            self.sketch_placement(),
        ) else {
            return curvyo_render_core::DrawList::default();
        };
        let overrun = GUIDE_OVERRUN_PX / self.view().scale();
        let (c, s) = (angle.cos(), angle.sin());
        let point = |p: Position, by: f64| {
            placement.apply_point(Point::new(p.x.0 + by * c, p.y.0 + by * s))
        };
        curvyo_render_core::build_sketch_lock_guide(
            point(anchor, -overrun),
            point(hover, overrun),
            self.view(),
        )
    }
}

/// `Some(true)` for a vertical axis stop, `Some(false)` for a horizontal one.
fn axis_stop(angle: f64) -> Option<bool> {
    let quarters = angle / FRAC_PI_2;
    let nearest = quarters.round();
    // Snapped stops are exact multiples of a quarter turn up to float rounding.
    ((quarters - nearest).abs() <= 1e-9).then(|| nearest.rem_euclid(2.0) > 0.5)
}

fn trimmed(degrees: f64) -> String {
    let text = format!("{degrees:.1}");
    text.strip_suffix(".0").map_or(text.clone(), str::to_owned)
}

fn preview(id: &str, x: f64, y: f64, text: &str) -> SketchAnnotation {
    SketchAnnotation {
        id: id.into(),
        x,
        y,
        text: text.into(),
        dimension: false,
        selected: false,
        conflicting: false,
        preview: true,
        ..Default::default()
    }
}
