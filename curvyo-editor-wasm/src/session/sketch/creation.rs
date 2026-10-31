//! Sketch construction picks stay ephemeral until a complete analytic operation exists.
use super::{Session, SketchTool};
use curvyo_document_core::Point;
use curvyo_sketch_core::{Construction, Length, Position};

impl Session {
    pub(in crate::session) fn sketch_pointer_down(&mut self, point: Point) {
        self.sync_sketch();
        if matches!(
            self.sketch.tool,
            SketchTool::Select | SketchTool::Constraints
        ) {
            if self.sketch_annotation_press(point, false) {
                return;
            }
            self.sketch_select_press(point);
            if self.sketch.tool == SketchTool::Constraints {
                self.sketch.drag = None;
                self.button_down = false;
                self.end_snapping_drag();
            }
            return;
        }
        self.button_down = true;
        if self.sketch.pending_prefix.is_none() {
            self.sketch.pending_prefix = Some(self.minter.mint().to_hex());
        }
        self.sketch.press = Some(point);
        self.sketch_pointer_hover(point);
        self.sketch.press_anchor = self.sketch.hover;
    }

    pub(in crate::session) fn sketch_pointer_hover(&mut self, point: Point) {
        self.sync_sketch();
        if matches!(
            self.sketch.tool,
            SketchTool::Select | SketchTool::Constraints
        ) {
            self.sketch_drag_hover(point);
            return;
        }
        self.sketch.hover = self
            .sketch_local(point)
            .map(|local| self.infer_sketch_axes(local));
        self.project_sketch_arc_hover();
        self.refresh_sketch_construction_preview();
    }

    pub(in crate::session) fn sketch_pointer_up(&mut self, point: Point) {
        if matches!(
            self.sketch.tool,
            SketchTool::Select | SketchTool::Constraints
        ) {
            self.sketch_drag_release(point);
            return;
        }
        let Some(press) = self.sketch.press.take() else {
            return;
        };
        if press.vector_to(point).length() * self.view().scale() <= 3.0
            && self.sketch_resume_at(point)
        {
            return;
        }
        let count = pick_count(self.sketch.tool);
        // Click-vs-drag is raw screen travel, independent of attraction.
        if self.sketch.picks.is_empty()
            && count == 2
            && press.vector_to(point).length() * self.view().scale() > 3.0
            && let Some(first) = self.sketch.press_anchor.take()
        {
            self.sketch.picks.push(first);
        }
        self.sketch_pointer_hover(point);
        let picked = if press.vector_to(point).length() * self.view().scale() <= 3.0 {
            self.sketch.press_anchor.take().or(self.sketch.hover)
        } else {
            self.sketch.hover
        };
        let Some(local) = picked else {
            return;
        };
        if self.sketch.picks.len() == 1
            && count >= 2
            && self.sketch.picks[0].distance(local).0 <= 1e-9
        {
            self.sketch.error = Some("Choose a distinct second point".into());
            return;
        }
        self.sketch.picks.push(local);
        if self.sketch.picks.len() < count {
            return;
        }
        let prefix = self
            .sketch
            .pending_prefix
            .clone()
            .unwrap_or_else(|| self.minter.mint().to_hex());
        let Some(mut command) = construction(
            self.sketch.tool,
            &self.sketch.picks,
            &prefix,
            self.sketch.chain.as_ref(),
        ) else {
            return;
        };
        if let (Some(previous), Some((_, start))) = (&self.sketch.chain_curve, &self.sketch.chain)
            && self.sketch.tool == SketchTool::Polyline
        {
            command = Construction::Continue {
                previous: previous.clone(),
                start: start.clone(),
                end: format!("{prefix}:end").into(),
                position: local,
                mode: self.sketch.mode,
            };
        }
        let closing = self.sketch_close_target(local);
        if let Some((first, last)) = &closing {
            command = self.sketch.chain_curve.as_ref().map_or_else(
                || Construction::Segment {
                    start: last.clone(),
                    end: first.clone(),
                    end_position: None,
                },
                |previous| Construction::Continue {
                    previous: previous.clone(),
                    start: last.clone(),
                    end: first.clone(),
                    position: local,
                    mode: self.sketch.mode,
                },
            );
        }
        let saved_picks = self.sketch.picks.clone();
        let chain = self.sketch.chain.clone();
        match self.sketch_construct_named(&prefix, &command) {
            Ok(_) => {
                self.sketch.cancel();
                if self.sketch.tool == SketchTool::Polyline && closing.is_none() {
                    self.continue_sketch_chain(&prefix, &command, chain.as_ref());
                }
            }
            Err(error) => {
                self.sketch.picks = saved_picks;
                self.sketch.picks.pop();
                self.sketch.error = Some(error.to_string());
            }
        }
    }

    pub(super) fn sketch_close_target(
        &self,
        local: Position,
    ) -> Option<(curvyo_sketch_core::EntityId, curvyo_sketch_core::EntityId)> {
        if self.sketch.tool != SketchTool::Polyline {
            return None;
        }
        let (first, last) = self.sketch.chain.as_ref()?;
        let snapshot = self.sketch_snapshot_for_edit().ok()?;
        let position = snapshot.solution.point(first).ok()?;
        (first != last && position.distance(local).0 * self.view().scale() <= 6.0)
            .then(|| (first.clone(), last.clone()))
    }

    pub(super) fn sketch_local(&self, point: Point) -> Option<Position> {
        let local = self.sketch_placement()?.inverse()?.apply_point(point);
        (local.x.is_finite() && local.y.is_finite()).then(|| Position::mm(local.x, local.y))
    }
}

pub(super) fn pick_count(tool: SketchTool) -> usize {
    match tool {
        SketchTool::Point => 1,
        SketchTool::Arc
        | SketchTool::ArcCenter
        | SketchTool::CircleThreePoints
        | SketchTool::Ellipse
        | SketchTool::EllipseThreePoints
        | SketchTool::Slot => 3,
        _ => 2,
    }
}

pub(super) fn construction(
    tool: SketchTool,
    picks: &[Position],
    prefix: &str,
    chain: Option<&(curvyo_sketch_core::EntityId, curvyo_sketch_core::EntityId)>,
) -> Option<Construction> {
    let a = *picks.first()?;
    if tool == SketchTool::Point {
        return Some(Construction::Point(a));
    }
    let b = *picks.get(1)?;
    Some(match tool {
        SketchTool::Polyline => match chain {
            Some((_, last)) => Construction::Segment {
                start: last.clone(),
                end: format!("{prefix}:end").into(),
                end_position: Some(b),
            },
            None => Construction::Polyline(vec![a, b]),
        },
        SketchTool::Rectangle => Construction::Rectangle(a, b),
        SketchTool::Circle => Construction::Circle {
            center: a,
            radius: a.distance(b),
        },
        SketchTool::ArcCenter => Construction::ArcCenter {
            center: a,
            start: b,
            end: *picks.get(2)?,
        },
        SketchTool::Arc => match chain {
            Some((_, start)) => Construction::ArcFrom {
                start: start.clone(),
                through: b,
                end: *picks.get(2)?,
            },
            None => Construction::Arc([a, b, *picks.get(2)?]),
        },
        SketchTool::CircleThreePoints => Construction::CircleThreePoints([a, b, *picks.get(2)?]),
        SketchTool::EllipseThreePoints => Construction::EllipseThreePoints([a, b, *picks.get(2)?]),
        SketchTool::Ellipse => {
            let c = *picks.get(2)?;
            let length = a.distance(b).0;
            let height = ((b.x.0 - a.x.0) * (c.y.0 - a.y.0) - (b.y.0 - a.y.0) * (c.x.0 - a.x.0))
                .abs()
                / length;
            Construction::Ellipse {
                center: a,
                axis_end: b,
                second_axis: Length(height),
            }
        }
        SketchTool::Slot => {
            let c = *picks.get(2)?;
            let length = a.distance(b).0;
            let half = ((b.x.0 - a.x.0) * (c.y.0 - a.y.0) - (b.y.0 - a.y.0) * (c.x.0 - a.x.0))
                .abs()
                / length;
            Construction::Slot {
                start: a,
                end: b,
                width: Length(2.0 * half),
            }
        }
        _ => return None,
    })
}
