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
        // A free first segment takes its mode direction from the sketch X axis.
        if self.sketch.tool != SketchTool::Polyline {
            return false;
        }
        self.sketch.mode = *mode;
        self.refresh_sketch_construction_preview();
        true
    }
    /// Alt: the next Polyline mode, the other Arc, Circle or Ellipse variant, or the other
    /// circle dimension kind; one step per press (AC25, AC34, AC35).
    pub(super) fn cycle_sketch_mode(&mut self) -> bool {
        if let Some(partner) = super::mode_label::variant_partner(self.sketch.tool) {
            if self.sketch.picks.is_empty() {
                self.sketch.tool = partner;
                self.sketch.arc_radius = None;
                self.refresh_sketch_construction_preview();
            } else {
                self.sketch.variant_hint = true;
            }
            return true;
        }
        if self.sketch.tool == SketchTool::Constraints
            && let Some(kind @ ("radius" | "diameter")) = self.sketch_dimension_action()
        {
            self.sketch.circle_dimension_radius = kind == "diameter";
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
    /// Holds the chain at the end of the segment just committed under `prefix`.
    /// `first_segment` names the ends of a mode-directed first segment.
    pub(super) fn continue_sketch_chain(
        &mut self,
        prefix: &str,
        command: &curvyo_sketch_core::Construction,
        chain: Option<&(EntityId, EntityId)>,
        first_segment: Option<(EntityId, EntityId, EntityId)>,
    ) {
        let (first, last, curve) = first_segment.unwrap_or_else(|| {
            let first = chain
                .as_ref()
                .map_or_else(|| format!("{prefix}:p0").into(), |(first, _)| first.clone());
            let last: EntityId =
                format!("{prefix}:{}", if chain.is_some() { "end" } else { "p1" }).into();
            let curve = match command {
                curvyo_sketch_core::Construction::Continue { mode, .. }
                    if super::first_segment::is_arc(*mode) =>
                {
                    "arc"
                }
                _ if chain.is_some() => "line",
                _ => "line0",
            };
            (first, last, format!("{prefix}:{curve}").into())
        });
        self.sketch.chain_curve = Some(curve);
        if let Ok(snapshot) = self.sketch_snapshot_for_edit()
            && let Ok(position) = snapshot.solution.point(&last)
        {
            self.sketch.picks = vec![position];
        }
        self.sketch.chain = Some((first, last));
        self.sketch.pending_prefix = Some(self.minter.mint().to_hex());
    }

    /// A first pick on a captured open endpoint resumes the chain from it (AC27).
    pub(super) fn sketch_resume_at(&mut self) -> bool {
        if !matches!(self.sketch.tool, SketchTool::Polyline | SketchTool::Arc)
            || !self.sketch.picks.is_empty()
            || self.sketch.chain.is_some()
        {
            return false;
        }
        let Some(id) = self.sketch.capture.clone() else {
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
