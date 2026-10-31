//! Drawing mode changes preserve pending chain picks without committing geometry.
use super::{Session, SketchTool};
use curvyo_sketch_core::{ContinuationMode, Entity, EntityId};

pub(super) const MODES: [(ContinuationMode, &str); 6] = [
    (ContinuationMode::FreeLine, "free-line"),
    (ContinuationMode::PerpendicularLine, "perpendicular-line"),
    (ContinuationMode::TangentLine, "tangent-line"),
    (ContinuationMode::TangentArc, "tangent-arc"),
    (
        ContinuationMode::PerpendicularArcLeft,
        "perpendicular-arc-left",
    ),
    (
        ContinuationMode::PerpendicularArcRight,
        "perpendicular-arc-right",
    ),
];
impl Session {
    /// Chooses an eligible connected drawing mode without accepting a pending segment.
    pub fn set_sketch_mode(&mut self, name: &str) -> bool {
        let Some((mode, _)) = MODES.iter().find(|(_, key)| *key == name) else {
            return false;
        };
        if self.sketch.tool != SketchTool::Polyline
            || (*mode != ContinuationMode::FreeLine && self.sketch.chain_curve.is_none())
        {
            return false;
        }
        self.sketch.mode = *mode;
        self.refresh_sketch_construction_preview();
        true
    }
    pub(super) fn cycle_sketch_mode(&mut self) -> bool {
        if matches!(self.sketch.tool, SketchTool::Arc | SketchTool::ArcCenter) {
            self.set_sketch_tool(if self.sketch.tool == SketchTool::Arc {
                SketchTool::ArcCenter
            } else {
                SketchTool::Arc
            });
            return true;
        }
        if self.sketch.tool != SketchTool::Polyline {
            return false;
        }
        let current = MODES
            .iter()
            .position(|(mode, _)| *mode == self.sketch.mode)
            .unwrap_or(0);
        for offset in 1..=MODES.len() {
            if self.set_sketch_mode(MODES[(current + offset) % MODES.len()].1) {
                return true;
            }
        }
        false
    }
    pub(super) fn continue_sketch_chain(
        &mut self,
        prefix: &str,
        command: &curvyo_sketch_core::Construction,
        chain: Option<&(EntityId, EntityId)>,
    ) {
        let first = chain
            .as_ref()
            .map_or_else(|| format!("{prefix}:p0").into(), |(first, _)| first.clone());
        let last: EntityId =
            format!("{prefix}:{}", if chain.is_some() { "end" } else { "p1" }).into();
        let curve = match command {
            curvyo_sketch_core::Construction::Continue {
                mode:
                    ContinuationMode::TangentArc
                    | ContinuationMode::PerpendicularArcLeft
                    | ContinuationMode::PerpendicularArcRight,
                ..
            } => "arc",
            _ if chain.is_some() => "line",
            _ => "line0",
        };
        self.sketch.chain_curve = Some(format!("{prefix}:{curve}").into());
        if let Ok(snapshot) = self.sketch_snapshot_for_edit()
            && let Ok(position) = snapshot.solution.point(&last)
        {
            self.sketch.picks = vec![position];
        }
        self.sketch.chain = Some((first, last));
        self.sketch.pending_prefix = Some(self.minter.mint().to_hex());
    }

    pub(super) fn sketch_resume_at(&mut self, point: curvyo_document_core::Point) -> bool {
        if !matches!(self.sketch.tool, SketchTool::Polyline | SketchTool::Arc)
            || !self.sketch.picks.is_empty()
            || self.sketch.chain.is_some()
        {
            return false;
        }
        let (Ok(snapshot), Some(local)) =
            (self.sketch_snapshot_for_edit(), self.sketch_local(point))
        else {
            return false;
        };
        let mut endpoints = std::collections::BTreeMap::<EntityId, usize>::new();
        for entity in snapshot.graph.entities.values() {
            if let Entity::Line { start, end } | Entity::Arc { start, end, .. } = entity {
                *endpoints.entry(start.clone()).or_default() += 1;
                *endpoints.entry(end.clone()).or_default() += 1;
            }
        }
        let nearest = endpoints
            .into_iter()
            .filter(|(_, count)| *count == 1)
            .filter_map(|(id, _)| {
                snapshot
                    .solution
                    .point(&id)
                    .ok()
                    .map(|p| (p.distance(local).0, id))
            })
            .filter(|(distance, _)| *distance * self.view().scale() <= 6.0)
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        let Some((_, id)) = nearest else {
            return false;
        };
        self.sketch.entities = [id].into();
        self.resume_sketch_endpoint();
        self.sketch.chain.is_some()
    }

    pub(super) fn sketch_can_resume(&self) -> bool {
        let mut selected = self.sketch.entities.iter();
        let Some(point) = selected.next() else {
            return false;
        };
        if selected.next().is_some() {
            return false;
        }
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return false;
        };
        !snapshot.stale && snapshot.solution.point(point).is_ok() && snapshot.graph.entities.values().filter(|e| matches!(e,Entity::Line{start,end}|Entity::Arc{start,end,..} if start==point || end==point)).count()==1
    }

    pub(super) fn resume_sketch_endpoint(&mut self) {
        let selected = self.sketch.entities.iter().cloned().collect::<Vec<_>>();
        let [point] = selected.as_slice() else {
            return;
        };
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return;
        };
        let Ok(position) = snapshot.solution.point(point) else {
            return;
        };
        let curves = snapshot
            .graph
            .entities
            .iter()
            .filter_map(|(id, entity)| match entity {
                Entity::Line { start, end } | Entity::Arc { start, end, .. }
                    if start == point || end == point =>
                {
                    Some(id.clone())
                }
                _ => None,
            })
            .collect::<Vec<EntityId>>();
        let [curve] = curves.as_slice() else {
            self.sketch.error =
                Some("Choose an open endpoint with one adjoining line or arc".into());
            return;
        };
        self.sketch.chain = Some((point.clone(), point.clone()));
        self.sketch.chain_curve = Some(curve.clone());
        self.sketch.picks.push(position);
        self.sketch.pending_prefix = Some(self.minter.mint().to_hex());
    }
}
