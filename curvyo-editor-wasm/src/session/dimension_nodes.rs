//! The Node tool on an unlinked dimension (`specs/0070-measure-and-dimension` criterion 13): its
//! kept points show as nodes and move with a drag, one step "Edit nodes"; its value stays.

use curvyo_document_core::{ObjectSnapshot, Point};
use curvyo_render_core::DrawList;

use super::{Session, Tool};

/// A kept point is hit within this many screen pixels.
const POINT_HIT_PX: f64 = 8.0;

impl Session {
    /// The kept points of the visible unlinked dimensions: (dimension, index, point).
    fn unlinked_points(&self) -> Vec<(curvyo_document_core::NodeId, usize, Point)> {
        self.document_read()
            .visible
            .iter()
            .filter_map(|o| match o {
                ObjectSnapshot::Dimension(d) if !d.linked => Some(d),
                _ => None,
            })
            .flat_map(|d| {
                d.kept
                    .points
                    .iter()
                    .enumerate()
                    .map(move |(i, p)| (d.id, i, *p))
            })
            .collect()
    }

    /// A Node-tool press on a kept point of an unlinked dimension starts its drag.
    pub(super) fn kept_point_pointer_down(&mut self, point: Point) -> bool {
        if self.tool != Tool::Node {
            return false;
        }
        let tolerance = POINT_HIT_PX / self.view().scale();
        let hit = self
            .unlinked_points()
            .into_iter()
            .map(|(id, i, p)| (p.vector_to(point).length(), id, i))
            .filter(|(d, ..)| *d <= tolerance)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        let Some((_, id, index)) = hit else {
            return false;
        };
        self.kept_point_drag = Some((id, index, point));
        true
    }

    /// The drag follows the pointer.
    pub(super) fn kept_point_pointer_move(&mut self, point: Point) -> bool {
        let Some((id, index, _)) = self.kept_point_drag else {
            return false;
        };
        self.kept_point_drag = Some((id, index, point));
        true
    }

    /// The release writes the moved point as one step "Edit nodes".
    pub(super) fn kept_point_pointer_up(&mut self, point: Point) -> bool {
        let Some((id, index, _)) = self.kept_point_drag.take() else {
            return false;
        };
        let _ = self.recording(|this| this.document.move_dimension_point(id, index, point));
        true
    }

    /// Escape during the drag: nothing is written.
    pub(super) fn cancel_kept_point_drag(&mut self) -> bool {
        self.kept_point_drag.take().is_some()
    }

    /// The kept points drawn as nodes in the Node tool.
    pub(super) fn kept_point_list(&self) -> DrawList {
        if self.tool != Tool::Node {
            return DrawList::default();
        }
        let points: Vec<Point> = self
            .unlinked_points()
            .into_iter()
            .map(|(id, i, p)| match self.kept_point_drag {
                Some((drag, at, to)) if drag == id && at == i => to,
                _ => p,
            })
            .collect();
        curvyo_render_core::build_measure_dots(&points, self.view())
    }
}
