//! Memoises the Bézier geometry of Spiro paths by their exact controls, solved synchronously.

use curvyo_document_core::{CurveKind, Document, NewAnchor, NodeId, PathSnapshot, Tolerance};
use std::collections::HashMap;

/// The notice when a Spiro curve has no solution and its control polygon is drawn instead.
pub(super) const SPIRO_FAILED_NOTICE: &str =
    "This Spiro curve cannot be solved; its points are joined by straight lines.";

/// The refusals of Curve to Bézier on a Spiro: a dependent text or dimension, a shared
/// document, a changed or invalid path.
pub(super) const CONVERT_REFERENCED_NOTICE: &str =
    "Remove dependent text or dimensions before converting this curve.";
pub(super) const CONVERT_SHARED_NOTICE: &str =
    "Spiro curves cannot be edited in a shared document.";
pub(super) const CONVERT_FAILED_NOTICE: &str =
    "Spiro conversion failed because the path changed or its geometry is invalid.";

/// The refusal of a transformation of a Spiro curve that has no solution.
pub(super) const SPIRO_UNSOLVED_NOTICE: &str =
    "This Spiro curve cannot be solved; move its points with the Node tool first.";

/// One solve: the controls it was made for and its anchors, `None` when the solver failed.
#[derive(Debug)]
struct Entry {
    controls: Vec<NewAnchor>,
    closed: bool,
    tolerance: f64,
    output: Option<Vec<NewAnchor>>,
}

impl Entry {
    fn solve(controls: &[NewAnchor], closed: bool, tolerance: f64) -> Self {
        Self {
            controls: controls.to_vec(),
            closed,
            tolerance,
            output: curvyo_ui_core::evaluate_spiro_nodes(
                controls,
                closed,
                Tolerance::from_mm(tolerance),
            )
            .ok(),
        }
    }

    fn serves(&self, controls: &[NewAnchor], closed: bool, tolerance: f64) -> bool {
        self.controls == controls && self.closed == closed && self.tolerance <= tolerance
    }
}

/// The last solve of each Spiro path, of the Pen's path in progress and of each path in a Node
/// drag; a solve runs only when its controls change.
#[derive(Debug, Default)]
pub(super) struct SpiroCache {
    entries: HashMap<NodeId, Entry>,
    preview: Option<Entry>,
    node_previews: HashMap<NodeId, Entry>,
    /// A solve failed since the last [`SpiroCache::take_new_failure`].
    new_failure: bool,
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

/// The anchors of `entry`, or its controls joined by straight lines when the solve failed, so a
/// curve that cannot be solved stays visible and editable.
fn drawn(entry: &Entry) -> Vec<NewAnchor> {
    entry.output.clone().unwrap_or_else(|| {
        entry
            .controls
            .iter()
            .map(|control| NewAnchor::corner(control.id, control.point))
            .collect()
    })
}

/// Whether `controls` of an open curve have no solution at the Pen's tolerance.
pub(super) fn unsolvable(controls: &[NewAnchor]) -> bool {
    Entry::solve(controls, false, curvyo_ui_core::SPIRO_TOLERANCE.as_mm())
        .output
        .is_none()
}

impl SpiroCache {
    /// Solves `controls` and notes a failure for the canvas notice.
    fn solve(&mut self, controls: &[NewAnchor], closed: bool, tolerance: f64) -> Entry {
        let entry = Entry::solve(controls, closed, tolerance);
        self.new_failure |= entry.output.is_none();
        entry
    }

    /// Whether a solve failed since the last call: the canvas says so once per failing edit.
    pub fn take_new_failure(&mut self) -> bool {
        std::mem::take(&mut self.new_failure)
    }

    /// Whether the Pen's path in progress is drawn as straight lines: its last solve failed.
    pub fn preview_failed(&self) -> bool {
        self.preview
            .as_ref()
            .is_some_and(|entry| entry.output.is_none())
    }

    /// The Pen's path in progress through `controls`, solved now when they changed.
    pub fn preview(&mut self, controls: &[NewAnchor], closed: bool) -> Option<Vec<NewAnchor>> {
        if controls.len() < if closed { 3 } else { 2 } {
            self.clear_preview();
            return None;
        }
        let tolerance = curvyo_ui_core::SPIRO_TOLERANCE.as_mm();
        if !self
            .preview
            .as_ref()
            .is_some_and(|entry| entry.serves(controls, closed, tolerance))
        {
            self.preview = Some(self.solve(controls, closed, tolerance));
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
            let entry = self.solve(&path.anchors, path.closed, tolerance);
            self.node_previews.insert(path.id, entry);
        }
        let entry = self.node_previews.get(&path.id)?;
        Some(placed_result(path, &drawn(entry)))
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
        if !self
            .entries
            .get(&path.id)
            .is_some_and(|entry| entry.serves(&path.anchors, path.closed, tolerance))
        {
            let entry = self.solve(&path.anchors, path.closed, tolerance);
            self.entries.insert(path.id, entry);
        }
        let entry = self.entries.get(&path.id)?;
        Some(entry)
    }
}

impl super::Session {
    /// Whether one selected path can be offered for Spiro-to-Bézier conversion.
    #[must_use]
    pub fn can_convert_selected_spiro_to_bezier(&self) -> bool {
        let [id] = self.selection.ids() else {
            return false;
        };
        self.document
            .path(*id)
            .is_some_and(|path| path.curve_kind == CurveKind::Spiro)
            && self.selected_group_leaves().is_empty()
    }

    /// Converts one selected Spiro path into the Bézier path it draws.
    pub fn convert_selected_spiro_to_bezier(&mut self) -> bool {
        self.recording(|this| {
            let [id] = this.selection.ids() else {
                return false;
            };
            let Some(source) = this.document.path(*id) else {
                return false;
            };
            if source.curve_kind != CurveKind::Spiro || !this.selected_group_leaves().is_empty() {
                return false;
            }
            let Some(mut evaluated) =
                this.spiro_cache
                    .borrow_mut()
                    .ready(&this.document, &source, 0.01)
            else {
                this.transform_notice = Some(SPIRO_FAILED_NOTICE);
                return false;
            };
            for anchor in &mut evaluated.anchors {
                anchor.id = this.minter.mint();
            }
            match this
                .document
                .convert_spiro_to_bezier(&source, &evaluated.anchors)
            {
                Ok(()) => true,
                Err(error) => {
                    this.transform_notice = Some(match error {
                        curvyo_document_core::CurveKindError::Referenced => {
                            CONVERT_REFERENCED_NOTICE
                        }
                        curvyo_document_core::CurveKindError::Shared => CONVERT_SHARED_NOTICE,
                        _ => CONVERT_FAILED_NOTICE,
                    });
                    false
                }
            }
        })
    }
}

impl super::Session {
    /// `objects` with each Spiro path and each clone of one replaced by the Bézier path it
    /// draws, for hit tests, bounds and transforms; a Spiro without solution stays as it is.
    pub(super) fn spiro_selection_objects(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        use curvyo_document_core::ObjectSnapshot;
        objects
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) if path.curve_kind == CurveKind::Spiro => {
                    let mut cache = self.spiro_cache.borrow_mut();
                    let result = cache.ready(&self.document, path, 0.01);
                    result.map_or_else(|| object.clone(), ObjectSnapshot::Path)
                }
                ObjectSnapshot::Clone(clone)
                    if clone.chain.root_outline.curve_kind == CurveKind::Spiro =>
                {
                    let matrix = clone.chain.composed;
                    let stretch = matrix
                        .a
                        .hypot(matrix.b)
                        .hypot(matrix.c.hypot(matrix.d))
                        .max(1.0);
                    if let Some(root) = self.spiro_cache.borrow_mut().ready(
                        &self.document,
                        &clone.chain.root_outline,
                        0.01 / stretch,
                    ) {
                        let mut clone = clone.clone();
                        let mut mapped = root.mapped(matrix);
                        mapped.id = clone.resolved.id;
                        mapped.style = clone.resolved.style.clone();
                        mapped.rotation = clone.resolved.rotation;
                        clone.resolved = Box::new(mapped);
                        clone.chain.root_outline = std::sync::Arc::new(root);
                        ObjectSnapshot::Clone(clone)
                    } else {
                        object.clone()
                    }
                }
                _ => object.clone(),
            })
            .collect()
    }
    /// `objects` as a press or hover picks them: as [`Session::spiro_selection_objects`], and a
    /// Spiro without solution as its control polygon, so it can be clicked and selected. A
    /// transform of it is still refused on release.
    pub(super) fn spiro_pick_objects(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        use curvyo_document_core::ObjectSnapshot;
        self.spiro_selection_objects(objects)
            .into_iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) if path.curve_kind == CurveKind::Spiro => self
                    .spiro_cache
                    .borrow_mut()
                    .lookup(&self.document, &path)
                    .map_or(ObjectSnapshot::Path(path), ObjectSnapshot::Path),
                other => other,
            })
            .collect()
    }

    pub(super) fn spiro_placement_objects(
        &self,
        objects: &[curvyo_document_core::ObjectSnapshot],
        tolerance: f64,
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        use curvyo_document_core::ObjectSnapshot;
        objects
            .iter()
            .map(|object| match object {
                ObjectSnapshot::Path(path) if path.curve_kind == CurveKind::Spiro => self
                    .spiro_cache
                    .borrow_mut()
                    .lookup_with_tolerance(&self.document, path, tolerance)
                    .map_or_else(|| object.clone(), ObjectSnapshot::Path),
                ObjectSnapshot::Clone(clone)
                    if clone.chain.root_outline.curve_kind == CurveKind::Spiro =>
                {
                    let matrix = clone.chain.composed;
                    let stretch = matrix
                        .a
                        .hypot(matrix.b)
                        .hypot(matrix.c.hypot(matrix.d))
                        .max(1.0);
                    if let Some(root) = self.spiro_cache.borrow_mut().lookup_with_tolerance(
                        &self.document,
                        &clone.chain.root_outline,
                        tolerance / stretch,
                    ) {
                        let mut clone = clone.clone();
                        let mut mapped = root.mapped(matrix);
                        mapped.id = clone.resolved.id;
                        mapped.style = clone.resolved.style.clone();
                        mapped.rotation = clone.resolved.rotation;
                        *clone.resolved = mapped;
                        clone.chain.root_outline = std::sync::Arc::new(root);
                        ObjectSnapshot::Clone(clone)
                    } else {
                        object.clone()
                    }
                }
                _ => object.clone(),
            })
            .collect()
    }

    pub(super) fn selection_contains_spiro(&self) -> bool {
        self.transform_selection_ids().iter().any(|id| {
            self.document
                .object(*id)
                .and_then(|object| object.drawn_path().map(|path| path.curve_kind))
                .is_some_and(|kind| kind == CurveKind::Spiro)
        })
    }
}

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

    #[test]
    fn a_curve_without_solution_is_drawn_as_its_control_polygon() {
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
        let drawn = cache.lookup(&document, &path).unwrap();
        assert!(cache.ready(&document, &path, 0.01).is_none());
        let points: Vec<_> = drawn.anchors.iter().map(|a| a.point).collect();
        let controls: Vec<_> = path.anchors.iter().map(|a| a.point).collect();
        assert_eq!(points, controls);
    }

    #[test]
    fn a_curve_without_solution_can_be_clicked_but_not_moved() {
        let mut session = super::super::Session::new(7);
        // Closed through three points on one line: the solver finds no curve.
        let id = session
            .document
            .create_editable_curve(
                &smooth(&[(5.0, 5.0), (25.0, 5.0), (15.0, 5.0)]),
                true,
                CurveKind::Spiro,
            )
            .unwrap();
        let path = session.document.path(id).unwrap();
        assert!(
            session
                .spiro_cache
                .borrow_mut()
                .ready(&session.document, &path, 0.01)
                .is_none()
        );
        let before = session.document.path(id).unwrap();
        session.pointer_down(Point::new(10.0, 5.0), false);
        session.pointer_up(Point::new(10.0, 5.0), false, false);
        assert_eq!(session.selection.ids(), &[id]);
        session.pointer_down(Point::new(10.0, 5.0), false);
        session.pointer_hover(Point::new(20.0, 15.0), false, false);
        session.pointer_up(Point::new(20.0, 15.0), false, false);
        assert_eq!(session.document.path(id).unwrap(), before);
        assert_eq!(session.transform_notice, Some(SPIRO_UNSOLVED_NOTICE));
    }
}
