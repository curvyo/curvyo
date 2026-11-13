//! Memoises the Bézier geometry of every Spiro curve the session draws.

use curvyo_document_core::{CurveKind, Document, NewAnchor, NodeId, PathSnapshot};
use std::collections::HashMap;

mod entry;

use entry::Entry;

/// The refusals of Curve to Bézier on a Spiro: a dependent text or dimension, a shared
/// document, a changed or invalid path.
pub(super) const CONVERT_REFERENCED_NOTICE: &str =
    "Remove dependent text or dimensions before converting this curve.";
pub(super) const CONVERT_SHARED_NOTICE: &str =
    "Spiro curves cannot be edited in a shared document.";
pub(super) const CONVERT_FAILED_NOTICE: &str =
    "Spiro conversion failed because the path changed or its geometry is invalid.";

/// The refusal of a transformation, width edit or conversion of a Spiro path without a curve:
/// one with too few points, several subpaths or coordinates too large to draw at the tolerance,
/// which only a file from outside can hold (every other Spiro has a curve since `specs/0115`).
pub(super) const SPIRO_UNSOLVED_NOTICE: &str = "This Spiro path has no curve to transform.";

/// The last solve of each Spiro path, of the Pen's path in progress and of each path in a Node
/// drag. A solve runs only when the controls change, and the curve is a function of the controls
/// alone (`specs/0115` criterion 2): what a moving point shows is what is stored and what a
/// reopen draws.
#[derive(Debug, Default)]
pub(super) struct SpiroCache {
    entries: HashMap<NodeId, Entry>,
    preview: Option<Entry>,
    node_previews: HashMap<NodeId, Entry>,
}

fn source_tolerance(path: &PathSnapshot, tolerance: f64) -> f64 {
    let matrix = path
        .spiro_transform
        .as_deref()
        .copied()
        .unwrap_or(curvyo_document_core::Affine::IDENTITY);
    tolerance / curvyo_ui_core::stretch(matrix)
}

/// `path` drawn as `anchors` (its local curve) under its placement.
fn placed_result(path: &PathSnapshot, anchors: &[NewAnchor]) -> PathSnapshot {
    let mut result = path.clone();
    result.anchors = anchors.to_vec();
    result.curve_kind = CurveKind::Bezier;
    let placement = result
        .spiro_transform
        .take()
        .map_or(curvyo_document_core::Affine::IDENTITY, |matrix| *matrix);
    result.mapped(placement)
}

/// The anchors of `entry`; controls without a curve (an invalid tolerance) as their polygon, so
/// they stay visible and editable (Proposal).
fn drawn(entry: &Entry) -> Vec<NewAnchor> {
    entry.output.clone().unwrap_or_else(|| {
        entry
            .controls
            .iter()
            .map(|control| NewAnchor::corner(control.id, control.point))
            .collect()
    })
}

impl SpiroCache {
    /// The curve of the Pen's path in progress through `controls`; `None` before there is one.
    pub fn preview(&mut self, controls: &[NewAnchor], closed: bool) -> Option<Vec<NewAnchor>> {
        if controls.len() < if closed { 3 } else { 2 } {
            return None;
        }
        let tolerance = curvyo_ui_core::SPIRO_TOLERANCE.as_mm();
        if !self
            .preview
            .as_ref()
            .is_some_and(|entry| entry.serves(controls, closed, tolerance))
        {
            self.preview = Some(Entry::fresh(controls, closed, tolerance));
        }
        self.preview.as_ref().map(drawn)
    }

    /// `path` as a Node drag shows it: its live controls solved, under its placement.
    pub fn node_preview(
        &mut self,
        document: &Document,
        path: &PathSnapshot,
    ) -> Option<PathSnapshot> {
        let source = document.path(path.id)?;
        if source.curve_kind != CurveKind::Spiro || source.is_compound() {
            return None;
        }
        let tolerance = source_tolerance(path, curvyo_ui_core::SPIRO_TOLERANCE.as_mm());
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return None;
        }
        if !self
            .node_previews
            .get(&path.id)
            .is_some_and(|entry| entry.serves(&path.anchors, path.closed, tolerance))
        {
            let entry = Entry::fresh(&path.anchors, path.closed, tolerance);
            self.node_previews.insert(path.id, entry);
        }
        let entry = self.node_previews.get(&path.id)?;
        Some(placed_result(path, &drawn(entry)))
    }

    /// The drag ended with its live controls written: their solves serve the stored paths.
    pub fn commit_node_previews(&mut self) {
        self.entries.extend(self.node_previews.drain());
    }

    pub fn clear_node_previews(&mut self) {
        self.node_previews.clear();
    }

    pub fn clear_preview(&mut self) {
        self.preview = None;
    }

    /// `path` evaluated to `tolerance` in document mm; `None` when it is not a Spiro path or the
    /// solver failed.
    pub(super) fn ready(
        &mut self,
        document: &Document,
        path: &PathSnapshot,
        tolerance: f64,
    ) -> Option<PathSnapshot> {
        self.solved(document, path, tolerance).and_then(|entry| {
            entry
                .output
                .as_ref()
                .map(|anchors| placed_result(path, anchors))
        })
    }

    /// `path` as drawn: its curve, or its control polygon when the solver failed.
    pub fn lookup(&mut self, document: &Document, path: &PathSnapshot) -> Option<PathSnapshot> {
        self.lookup_with_tolerance(document, path, curvyo_ui_core::SPIRO_TOLERANCE.as_mm())
    }

    pub fn lookup_with_tolerance(
        &mut self,
        document: &Document,
        path: &PathSnapshot,
        tolerance: f64,
    ) -> Option<PathSnapshot> {
        let entry = self.solved(document, path, tolerance)?;
        Some(placed_result(path, &drawn(entry)))
    }

    /// The solve of `path`'s controls, made now when the memo does not serve it; `None` for a
    /// path that is not a plain Spiro of the document.
    fn solved(
        &mut self,
        document: &Document,
        path: &PathSnapshot,
        tolerance: f64,
    ) -> Option<&Entry> {
        let tolerance = source_tolerance(path, tolerance);
        if !tolerance.is_finite()
            || tolerance <= 0.0
            || path.curve_kind != CurveKind::Spiro
            || path.is_compound()
            || path.anchors.len() < if path.closed { 3 } else { 2 }
            || !document
                .path(path.id)
                .is_some_and(|current| current.curve_kind == CurveKind::Spiro)
        {
            return None;
        }
        let served = self
            .entries
            .get(&path.id)
            .is_some_and(|entry| entry.serves(&path.anchors, path.closed, tolerance));
        if !served {
            let entry = match self.entries.get(&path.id) {
                Some(earlier) if earlier.same_controls(&path.anchors, path.closed) => {
                    earlier.at_tolerance(tolerance)
                }
                _ => Entry::fresh(&path.anchors, path.closed, tolerance),
            };
            self.entries.insert(path.id, entry);
        }
        self.entries.get(&path.id)
    }
}

#[cfg(test)]
mod measure;

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{AnchorId, AnchorKind, Point};

    fn smooth(points: &[(f64, f64)]) -> Vec<NewAnchor> {
        points
            .iter()
            .zip(1..)
            .map(|(&(x, y), k)| {
                let mut anchor = NewAnchor::corner(AnchorId::new(1, k), Point::new(x, y));
                anchor.kind = AnchorKind::Symmetric;
                anchor
            })
            .collect()
    }

    #[test]
    fn a_preview_is_solved_at_once_and_kept_while_its_controls_stay() {
        let mut cache = SpiroCache::default();
        let controls = smooth(&[(0.0, 0.0), (10.0, 5.0), (20.0, 0.0)]);
        let first = cache.preview(&controls, false).unwrap();
        assert!(first.len() > controls.len(), "the curve, not the controls");
        assert_eq!(cache.preview(&controls, false).unwrap(), first);
        assert!(cache.preview(&controls[..1], false).is_none());
    }

    /// `specs/0115` criterion 2 (customer 2026-11-12): the preview is a function of its controls
    /// alone. Wherever the point circling its neighbour has been, it shows the curve of the
    /// controls it has now, the one a stored path of those controls draws.
    #[test]
    fn a_preview_is_the_curve_of_its_controls_wherever_the_point_came_from() {
        let document = Document::new(1);
        let mut cache = SpiroCache::default();
        let mut controls = smooth(&[(10.0, 20.0), (30.0, 10.0), (50.0, 20.0), (65.0, 20.0)]);
        for degrees in (0..=720).step_by(3) {
            let angle = f64::from(degrees).to_radians();
            controls[3].point = Point::new(50.0 + 15.0 * angle.cos(), 20.0 + 15.0 * angle.sin());
            let shown = cache.preview(&controls, false).unwrap();
            if degrees % 90 == 0 {
                let id = document
                    .create_editable_curve(&controls, false, CurveKind::Spiro)
                    .unwrap();
                let path = document.path(id).unwrap();
                let stored = SpiroCache::default().lookup(&document, &path).unwrap();
                assert_eq!(stored.anchors, shown, "{degrees} degrees");
            }
        }
    }

    #[test]
    fn a_path_is_drawn_as_its_curve_and_follows_a_control_edit() {
        let document = Document::new(1);
        let id = document
            .create_editable_curve(
                &smooth(&[(0.0, 0.0), (10.0, 5.0), (20.0, 0.0)]),
                false,
                CurveKind::Spiro,
            )
            .unwrap();
        let mut cache = SpiroCache::default();
        let path = document.path(id).unwrap();
        let drawn = cache.lookup(&document, &path).unwrap();
        assert_eq!(drawn.curve_kind, CurveKind::Bezier);
        assert!(cache.ready(&document, &path, 0.01).is_some());
        let mut moved = path.clone();
        moved.anchors[1].point = Point::new(10.0, 8.0);
        assert_ne!(
            cache.lookup(&document, &moved).unwrap().anchors,
            drawn.anchors
        );
    }

    /// Controls on one spot are that spot: a curve, drawn and ready for a transform.
    #[test]
    fn controls_on_one_spot_are_drawn_as_that_spot() {
        let document = Document::new(1);
        let id = document
            .create_editable_curve(
                &smooth(&[(5.0, 5.0), (5.0, 5.0), (5.0, 5.0)]),
                false,
                CurveKind::Spiro,
            )
            .unwrap();
        let mut cache = SpiroCache::default();
        let path = document.path(id).unwrap();
        let drawn = cache.ready(&document, &path, 0.01).unwrap();
        assert!(
            drawn
                .anchors
                .iter()
                .all(|a| a.point == Point::new(5.0, 5.0))
        );
    }

    /// A closed Spiro through three points on one line (no curvature-continuous curve exists)
    /// is drawn as the solver's last iterate and moves like any other path (it was refused with
    /// a notice before `specs/0115` dropped the convergence check).
    #[test]
    fn a_curve_whose_iteration_does_not_converge_can_be_clicked_and_moved() {
        let mut session = super::super::Session::new(7);
        let id = session
            .document
            .create_editable_curve(
                &smooth(&[(5.0, 5.0), (25.0, 5.0), (15.0, 5.0)]),
                true,
                CurveKind::Spiro,
            )
            .unwrap();
        let path = session.document.path(id).unwrap();
        let drawn = session
            .spiro_cache
            .borrow_mut()
            .ready(&session.document, &path, 0.01)
            .unwrap();
        assert!(
            drawn
                .anchors
                .iter()
                .all(|a| a.point.x.is_finite() && a.point.y.is_finite())
        );
        session.pointer_down(Point::new(10.0, 5.0), false);
        session.pointer_up(Point::new(10.0, 5.0), false, false);
        assert_eq!(session.selection.ids(), &[id]);
        session.pointer_down(Point::new(10.0, 5.0), false);
        session.pointer_hover(Point::new(20.0, 15.0), false, false);
        session.pointer_up(Point::new(20.0, 15.0), false, false);
        let moved = session.document.path(id).unwrap();
        assert_ne!(moved, path, "the drag moved it");
        assert_eq!(session.transform_notice, None);
    }
}
