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
            if self.sketch_annotation_press(point) {
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
        // Capture wins over the Ctrl lock, which wins over axis inference (AC27, AC28).
        self.sketch.lock = None;
        self.sketch.hover =
            self.sketch_local(point)
                .map(|local| match self.sketch_capture(local) {
                    Some(captured) => captured,
                    None => self
                        .sketch_lock(local)
                        .unwrap_or_else(|| self.infer_sketch_axes(local)),
                });
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
        // Click-vs-drag is raw screen travel, independent of attraction (AC38).
        let mut click = press.vector_to(point).length() * self.view().scale() < CLICK_TRAVEL_PX;
        if !click {
            // Press and release captured onto one point span no shape: the release is a click
            // and the shape follows from that point, never a silent no-op.
            self.sketch_pointer_hover(point);
            click = self
                .sketch
                .press_anchor
                .zip(self.sketch.hover)
                .is_some_and(|(a, b)| a.distance(b).0 <= super::capture::CAPTURED.0);
        }
        if click && self.sketch_resume_at() {
            return;
        }
        let count = pick_count(self.sketch.tool);
        // A drag from the first press sets the first two picks: press and release.
        if self.sketch.picks.is_empty()
            && count >= 2
            && !click
            && let Some(first) = self.sketch.press_anchor.take()
        {
            self.sketch.picks.push(first);
        }
        self.sketch_pointer_hover(point);
        let picked = if click {
            self.sketch.press_anchor.take().or(self.sketch.hover)
        } else {
            self.sketch.hover
        };
        let Some(local) = picked else {
            return;
        };
        if self.sketch.picks.len() == 1
            && count >= 2
            && self.sketch.picks[0].distance(local).0 <= super::capture::CAPTURED.0
        {
            self.sketch.error = Some("Choose a distinct second point".into());
            return;
        }
        self.sketch.picks.push(local);
        self.sketch.variant_hint = false;
        if self.sketch.tool == SketchTool::Slot && self.sketch.picks.len() == 2 {
            self.sketch.spine_lock = self.sketch.lock;
        }
        if self.sketch.picks.len() >= count {
            self.sketch_complete_picks(local);
        }
    }

    /// Builds the command the complete picks ask for: chained, closing or joining a captured
    /// endpoint, and commits it.
    fn sketch_complete_picks(&mut self, local: Position) {
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
        let closing = self.sketch_close_target();
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
        let joined = self.sketch_capture_join(&prefix, &self.sketch.picks);
        let closes = closing.is_some() || joined.is_some();
        let step = if let Some(joined) = joined {
            super::first_segment::PolylineStep::plain(joined)
        } else {
            let picks = &self.sketch.picks[..self.sketch.picks.len() - 1];
            self.sketch_polyline_step(&prefix, picks, command, local)
        };
        self.sketch_commit_pick(&prefix, step, closes);
    }

    /// Commits the complete picks as one edit; a failure keeps all but the last pick.
    fn sketch_commit_pick(
        &mut self,
        prefix: &str,
        step: super::first_segment::PolylineStep,
        closing: bool,
    ) {
        let step = self.with_lock_relation(prefix, step);
        if step.degenerate {
            self.sketch.picks.pop();
            self.sketch.error = Some(DEGENERATE.into());
            return;
        }
        let saved_picks = self.sketch.picks.clone();
        let chain = self.sketch.chain.clone();
        let joins = self.sketch_joins(&saved_picks);
        if self.sketch.tool == SketchTool::Point && !joins.is_empty() {
            // A point on an existing endpoint is that endpoint: nothing new (AC36).
            self.sketch.cancel();
            return;
        }
        let mut relations = step.relation.iter().cloned().collect::<Vec<_>>();
        relations.extend(self.sketch_typed_relations(prefix, &step.command, None));
        match self.sketch_construct_related(prefix, &step.command, &relations, &joins) {
            Ok(_) => {
                self.sketch.cancel();
                if self.sketch.tool == SketchTool::Polyline && !closing {
                    self.continue_sketch_chain(prefix, &step.command, chain.as_ref(), step.ends);
                    self.sketch_rejoin_chain_start(&joins, saved_picks.first().copied());
                }
            }
            Err(error) => {
                self.sketch.picks = saved_picks;
                self.sketch.picks.pop();
                self.sketch.error = Some(error.to_string());
            }
        }
    }

    /// A first segment drawn from a captured endpoint starts its chain at that endpoint, whose
    /// identity its first point took.
    pub(super) fn sketch_rejoin_chain_start(
        &mut self,
        joins: &[(Position, curvyo_sketch_core::EntityId)],
        first_pick: Option<Position>,
    ) {
        let (Some((first, _)), Some(pick)) = (&self.sketch.chain, first_pick) else {
            return;
        };
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return;
        };
        if snapshot.graph.entities.contains_key(first) {
            return;
        }
        if let Some((_, existing)) = joins
            .iter()
            .find(|(at, _)| at.distance(pick).0 <= super::capture::CAPTURED.0)
            && let Some((first, _)) = &mut self.sketch.chain
        {
            *first = existing.clone();
        }
    }

    /// The captured chain start or open endpoint a chained polyline segment ends on, with the
    /// chain's last point.
    pub(super) fn sketch_close_target(
        &self,
    ) -> Option<(curvyo_sketch_core::EntityId, curvyo_sketch_core::EntityId)> {
        if self.sketch.tool != SketchTool::Polyline {
            return None;
        }
        let target = self.sketch.capture.clone()?;
        let (_, last) = self.sketch.chain.as_ref()?;
        (target != *last).then(|| (target, last.clone()))
    }

    /// An unchained first segment or three-point arc whose end pick is captured: drawn from
    /// the captured endpoint back to the free start, so that endpoint keeps its identity.
    pub(super) fn sketch_capture_join(
        &self,
        prefix: &str,
        picks: &[Position],
    ) -> Option<Construction> {
        let target = self.sketch.capture.clone()?;
        if self.sketch.chain.is_some() {
            return None;
        }
        match (self.sketch.tool, picks) {
            (SketchTool::Polyline, [first, _]) => Some(Construction::Segment {
                start: target,
                end: format!("{prefix}:p0").into(),
                end_position: Some(*first),
            }),
            (SketchTool::Arc, [first, through, _]) => Some(Construction::ArcFrom {
                start: target,
                through: *through,
                end: *first,
            }),
            _ => None,
        }
    }

    pub(super) fn sketch_local(&self, point: Point) -> Option<Position> {
        let local = self.sketch_placement()?.inverse()?.apply_point(point);
        (local.x.is_finite() && local.y.is_finite()).then(|| Position::mm(local.x, local.y))
    }
}

/// Screen travel from press to release below which a release is a click: the shape follows
/// the pointer until the next click; from it on, the release is a pick (AC38, R3-Q3).
const CLICK_TRAVEL_PX: f64 = 4.0;

/// Why an arc whose end lies on its start direction is not committed.
pub(super) const DEGENERATE: &str = "The arc end lies on its start direction: move off the line";

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
