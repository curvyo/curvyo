//! Snapping adapters for node anchor and handle gestures.
use super::Session;
use curvyo_document_core::{Point, Vec2};
use curvyo_ui_core::{SnapExclusions, SnapProbe};
use std::collections::HashSet;
impl Session {
    pub(super) fn begin_node_snapping(&mut self, point: Point) {
        let paths = self.node_paths();
        let moved = self.node.snapping_points(&paths, point);
        if moved.is_empty() {
            return;
        }
        let anchors: HashSet<_> = moved.iter().map(|&(p, a, _)| (p, a)).collect();
        let boxes: HashSet<_> = moved.iter().map(|&(p, _, _)| p).collect();
        let read = self.document_read();
        self.snapping.update_stored(self.document.snapping());
        self.snapping.begin_drag(
            &read,
            &self.document.pages(),
            1.0 / self.view().scale(),
            SnapExclusions {
                boxes: Some(&boxes),
                nodes: None,
                anchor: None,
                anchors: Some(&anchors),
            },
        );
        self.snapping
            .set_nodes_only(self.node.snapping_handles_only());
        self.snapping.version = Some(self.document.version());
    }
    pub(super) fn resolve_node_snapping(&mut self, point: Point) -> Vec2 {
        if !self.node.snapping_moved(point) {
            self.resolve_snapping(&SnapProbe::default());
            return Vec2::ZERO;
        }
        let points: Vec<_> = self
            .node
            .snapping_points(&self.node_paths(), point)
            .into_iter()
            .map(|(_, _, p)| p)
            .collect();
        if points.is_empty() {
            return Vec2::ZERO;
        }
        // The pressed anchor snaps to the grid (`specs/0090-grid` criterion 11); a handle drag
        // has only its own point.
        let dragged: Vec<_> = if self.node.snapping_handles_only() {
            points.clone()
        } else {
            self.node
                .snapping_pressed_point(point)
                .into_iter()
                .collect()
        };
        self.resolve_snapping_grid(&SnapProbe::points(&points), &dragged, None)
            .offset
    }
    pub(super) fn snapped_node_pointer(&self, point: Point) -> Point {
        point.translated(self.snap_result().map_or(Vec2::ZERO, |r| r.offset))
    }
}
