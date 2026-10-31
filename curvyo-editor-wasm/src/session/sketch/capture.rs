//! Magnetic endpoint capture pulls a pick onto an open endpoint with screen-pixel hysteresis.
use super::{Session, SketchTool};
use curvyo_document_core::Point;
use curvyo_sketch_core::{Entity, EntityId, Length, Position};
use std::collections::BTreeMap;

/// Screen pixels within which an open endpoint captures the pointer (UX notes revision 2, 4).
const CAPTURE_PX: f64 = 12.0;
/// Screen pixels beyond which a captured endpoint lets the pointer go again.
const RELEASE_PX: f64 = 16.0;
/// A pick this close to an open endpoint was captured on it.
pub(super) const CAPTURED: Length = Length(1e-9);

impl Session {
    /// Updates the captured endpoint for the pointer at `local`; returns the captured position.
    /// The nearest open endpoint wins; an exact tie goes to the older (lower) identity.
    pub(super) fn sketch_capture(&mut self, local: Position) -> Option<Position> {
        let candidates = self.sketch_capture_candidates();
        let scale = self.view().scale();
        let distance = |p: Position| p.distance(local).0 * scale;
        if let Some(id) = &self.sketch.capture
            && let Some(position) = candidates.get(id)
            && distance(*position) <= RELEASE_PX
        {
            return Some(*position);
        }
        let nearest = candidates
            .into_iter()
            .map(|(id, p)| (distance(p), id, p))
            .filter(|(d, ..)| *d <= CAPTURE_PX)
            .min_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        self.sketch.capture = nearest.as_ref().map(|(_, id, _)| id.clone());
        nearest.map(|(.., p)| p)
    }

    /// Open endpoints (one adjoining line or arc) and the chain start a pick may land on, for
    /// every pick of every drawing tool (AC36); never the chain's own last point.
    fn sketch_capture_candidates(&self) -> BTreeMap<EntityId, Position> {
        if matches!(
            self.sketch.tool,
            SketchTool::Select | SketchTool::Constraints | SketchTool::Fillet
        ) {
            return BTreeMap::new();
        }
        let mut candidates = self.sketch_open_endpoints();
        if let Some((_, last)) = &self.sketch.chain {
            candidates.remove(last);
        }
        candidates
    }

    /// Every open endpoint of the current sketch with its solved position.
    fn sketch_open_endpoints(&self) -> BTreeMap<EntityId, Position> {
        let Ok(snapshot) = self.sketch_snapshot_for_edit() else {
            return BTreeMap::new();
        };
        if snapshot.stale {
            return BTreeMap::new();
        }
        let mut adjoining = BTreeMap::<&EntityId, usize>::new();
        for entity in snapshot.graph.entities.values() {
            if let Entity::Line { start, end } | Entity::Arc { start, end, .. } = entity {
                *adjoining.entry(start).or_default() += 1;
                *adjoining.entry(end).or_default() += 1;
            }
        }
        adjoining
            .into_iter()
            .filter(|(_, count)| *count == 1)
            .filter_map(|(id, _)| {
                let position = snapshot.solution.point(id).ok()?;
                Some((id.clone(), position))
            })
            .collect()
    }

    /// The open endpoints `picks` were captured on: a pick standing exactly on one (capture
    /// sets it there) hands that identity to the point it creates (AC36).
    pub(super) fn sketch_joins(&self, picks: &[Position]) -> Vec<(Position, EntityId)> {
        if matches!(
            self.sketch.tool,
            SketchTool::Select | SketchTool::Constraints | SketchTool::Fillet
        ) {
            return Vec::new();
        }
        let endpoints = self.sketch_open_endpoints();
        picks
            .iter()
            .filter_map(|pick| {
                endpoints
                    .iter()
                    .find(|(_, p)| p.distance(*pick).0 <= CAPTURED.0)
                    .map(|(id, _)| (*pick, id.clone()))
            })
            .collect()
    }

    /// The capture ring at the captured endpoint (UX notes revision 2, section 4).
    pub(super) fn sketch_capture_draw_list(&self) -> curvyo_render_core::DrawList {
        let (Some(_), Some(hover), Some(placement)) = (
            &self.sketch.capture,
            self.sketch.hover,
            self.sketch_placement(),
        ) else {
            return curvyo_render_core::DrawList::default();
        };
        curvyo_render_core::build_sketch_capture_marker(
            placement.apply_point(Point::new(hover.x.0, hover.y.0)),
            self.view(),
        )
    }

    /// The "Endpoint" label over the capture ring.
    pub(super) fn sketch_capture_annotation(&self) -> Option<super::inspection::SketchAnnotation> {
        self.sketch.capture.as_ref()?;
        let (x, y) = self.sketch_screen(self.sketch.hover?)?;
        Some(super::inspection::SketchAnnotation {
            id: "pending-capture".into(),
            x,
            y: y - 20.0,
            text: "Endpoint".into(),
            dimension: false,
            selected: false,
            conflicting: false,
            preview: true,
            ..Default::default()
        })
    }

    /// A local sketch position in canvas CSS pixels.
    pub(super) fn sketch_screen(&self, local: Position) -> Option<(f64, f64)> {
        let placement = self.sketch_placement()?;
        Some(
            self.view()
                .document_to_screen(placement.apply_point(Point::new(local.x.0, local.y.0))),
        )
    }
}
