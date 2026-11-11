//! Memoises the Bézier geometry of Spiro paths, each solved from its earlier shape so it never
//! jumps to another solution.

use curvyo_document_core::{
    AnchorId, CurveKind, Document, NewAnchor, NodeId, PathSnapshot, Tolerance,
};
use curvyo_geometry_core::SpiroShape;
use std::collections::HashMap;

/// The refusals of Curve to Bézier on a Spiro: a dependent text or dimension, a shared
/// document, a changed or invalid path.
pub(super) const CONVERT_REFERENCED_NOTICE: &str =
    "Remove dependent text or dimensions before converting this curve.";
pub(super) const CONVERT_SHARED_NOTICE: &str =
    "Spiro curves cannot be edited in a shared document.";
pub(super) const CONVERT_FAILED_NOTICE: &str =
    "Spiro conversion failed because the path changed or its geometry is invalid.";

/// The refusal of a transformation, width edit or conversion of a Spiro curve without a solution:
/// one that came in from an older build, since what this build draws and stores always has one
/// (Proposal, customer 2026-11-11).
pub(super) const SPIRO_UNSOLVED_NOTICE: &str =
    "This Spiro curve comes from an older build; move one of its points with the Node tool first.";

/// One solve: the controls it was made for, their shape and anchors; no shape when the solver
/// found none (a curve that came in from outside).
#[derive(Debug, Clone)]
struct Entry {
    controls: Vec<NewAnchor>,
    closed: bool,
    tolerance: f64,
    shape: Option<SpiroShape>,
    output: Option<Vec<NewAnchor>>,
}

impl Entry {
    fn of_shape(controls: &[NewAnchor], closed: bool, tolerance: f64, shape: SpiroShape) -> Self {
        let output = curvyo_ui_core::spiro_shape_nodes(
            controls,
            closed,
            &shape,
            Tolerance::from_mm(tolerance),
        )
        .ok();
        Self {
            controls: controls.to_vec(),
            closed,
            tolerance,
            shape: output.is_some().then_some(shape),
            output,
        }
    }

    /// `controls` solved from `earlier`'s shape (or from nothing), staying near it; `None` when
    /// the curve has no solution there. Without `fresh` only the earlier shape is followed.
    fn follow(
        earlier: Option<&Self>,
        controls: &[NewAnchor],
        closed: bool,
        tolerance: f64,
        fresh: bool,
    ) -> Option<Self> {
        let near = near_of(earlier, controls, closed)?;
        let shape = curvyo_ui_core::solve_spiro_nodes_near(controls, closed, &near, fresh).ok()?;
        let entry = Self::of_shape(controls, closed, tolerance, shape);
        entry.output.is_some().then_some(entry)
    }

    /// As [`Entry::follow`] for a point that moves: with the same controls as `earlier`, only
    /// moved, the curve may move at most [`STEADY_FACTOR`] times as far as they did plus
    /// [`STEADY_SLACK_MM`]; farther is a jump (lead decision 2026-11-11, Proposal).
    fn follow_steadily(
        earlier: Option<&Self>,
        controls: &[NewAnchor],
        closed: bool,
        tolerance: f64,
        fresh: bool,
    ) -> Option<Self> {
        let entry = Self::follow(earlier, controls, closed, tolerance, fresh)?;
        let Some((earlier, before, after)) = earlier.and_then(|earlier| {
            Some((
                earlier,
                earlier.output.as_deref()?,
                entry.output.as_deref()?,
            ))
        }) else {
            return Some(entry);
        };
        if earlier.closed != closed
            || earlier.controls.len() != controls.len()
            || earlier
                .controls
                .iter()
                .zip(controls)
                .any(|(a, b)| a.id != b.id)
        {
            return Some(entry);
        }
        let moved = earlier
            .controls
            .iter()
            .zip(controls)
            .map(|(a, b)| a.point.vector_to(b.point).length())
            .fold(0.0, f64::max);
        let allowed = STEADY_FACTOR * moved + STEADY_SLACK_MM;
        // Anchors that pair up and stay within the allowance bound the move at once (the usual
        // case); otherwise the move is measured against the other curve's chords.
        let paired = before.len() == after.len()
            && before
                .iter()
                .zip(after)
                .all(|(a, b)| a.point.vector_to(b.point).length() <= allowed);
        (paired || (reach(before, after) <= allowed && reach(after, before) <= allowed))
            .then_some(entry)
    }

    /// `controls` solved from scratch, for a stored curve without an earlier shape.
    fn fresh(controls: &[NewAnchor], closed: bool, tolerance: f64) -> Self {
        match curvyo_ui_core::solve_spiro_nodes(controls, closed) {
            Ok(shape) => Self::of_shape(controls, closed, tolerance, shape),
            Err(_) => Self {
                controls: controls.to_vec(),
                closed,
                tolerance,
                shape: None,
                output: None,
            },
        }
    }

    /// The same shape drawn to `tolerance`.
    fn at_tolerance(&self, tolerance: f64) -> Self {
        match &self.shape {
            Some(shape) => Self::of_shape(&self.controls, self.closed, tolerance, shape.clone()),
            None => self.clone(),
        }
    }

    fn same_controls(&self, controls: &[NewAnchor], closed: bool) -> bool {
        self.controls == controls && self.closed == closed
    }

    fn serves(&self, controls: &[NewAnchor], closed: bool, tolerance: f64) -> bool {
        self.same_controls(controls, closed) && self.tolerance <= tolerance
    }
}

/// How many times as far as its controls moved a followed curve may move.
const STEADY_FACTOR: f64 = 4.0;
/// How far (mm) a followed curve may move beyond [`STEADY_FACTOR`] times its controls' move.
const STEADY_SLACK_MM: f64 = 2.0;

/// The farthest any anchor of `from` lies from the polyline through the anchors of `to`.
fn reach(from: &[NewAnchor], to: &[NewAnchor]) -> f64 {
    from.iter()
        .map(|anchor| {
            let p = anchor.point;
            to.windows(2)
                .map(|pair| distance_to_chord(p, pair[0].point, pair[1].point))
                .fold(
                    to.first()
                        .map_or(f64::INFINITY, |q| q.point.vector_to(p).length()),
                    f64::min,
                )
        })
        .fold(0.0, f64::max)
}

fn distance_to_chord(
    p: curvyo_document_core::Point,
    a: curvyo_document_core::Point,
    b: curvyo_document_core::Point,
) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = dx * dx + dy * dy;
    let t = if length > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (p.x - (a.x + t * dx)).hypot(p.y - (a.y + t * dy))
}

/// The earlier coefficients of each segment of `controls`, matched by the ids of its two end
/// controls; `None` for a segment `earlier` does not have. `None` when `controls` cannot form a
/// curve.
fn near_of(
    earlier: Option<&Entry>,
    controls: &[NewAnchor],
    closed: bool,
) -> Option<Vec<Option<[f64; 4]>>> {
    let segments = if closed {
        controls.len()
    } else {
        controls.len().checked_sub(1)?
    };
    if segments == 0 || (closed && controls.len() < 3) {
        return None;
    }
    let known: HashMap<(AnchorId, AnchorId), [f64; 4]> = earlier
        .and_then(|entry| entry.shape.as_ref().map(|shape| (entry, shape)))
        .map(|(entry, shape)| {
            let count = entry.controls.len();
            shape
                .segments()
                .iter()
                .enumerate()
                .map(|(i, k)| {
                    (
                        (entry.controls[i].id, entry.controls[(i + 1) % count].id),
                        *k,
                    )
                })
                .collect()
        })
        .unwrap_or_default();
    Some(
        (0..segments)
            .map(|i| {
                known
                    .get(&(controls[i].id, controls[(i + 1) % controls.len()].id))
                    .copied()
            })
            .collect(),
    )
}

/// The last solve of each Spiro path, of the Pen's path in progress and of each path in a Node
/// drag. A solve runs only when its controls change, starts from the earlier shape, and is taken
/// only when it stays near it; otherwise the earlier one is kept (customer 2026-11-11).
#[derive(Debug, Default)]
pub(super) struct SpiroCache {
    entries: HashMap<NodeId, Entry>,
    preview: Option<Entry>,
    /// The Pen's latest refused controls, so a frame does not solve them again.
    refused: Option<(Vec<NewAnchor>, bool)>,
    /// The Pen's last preview, kept after its path ended: the shape a just drawn path starts
    /// from.
    retired: Option<Entry>,
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

/// The anchors of `entry`; a curve without solution (one that came in from outside) as its
/// controls joined by straight lines, so it stays visible and editable (Proposal).
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
    /// The Pen's path in progress through `controls`: their curve, or, when they have no
    /// solution near the last one shown, the last one shown. Returns the curve and the controls
    /// it goes through; `None` before there is a curve.
    pub fn preview(
        &mut self,
        controls: &[NewAnchor],
        closed: bool,
    ) -> Option<(Vec<NewAnchor>, Vec<NewAnchor>)> {
        if controls.len() < if closed { 3 } else { 2 } {
            return None;
        }
        self.try_preview(controls, closed, true);
        let entry = self.preview.as_ref()?;
        Some((drawn(entry), entry.controls.clone()))
    }

    /// Takes `controls` as the Pen's path in progress when they solve near the last preview;
    /// returns whether they did. A refusal keeps the last preview. Without `fresh` only the last
    /// preview's shape is followed (a step of a held point's search).
    pub fn try_preview(&mut self, controls: &[NewAnchor], closed: bool, fresh: bool) -> bool {
        let tolerance = curvyo_ui_core::SPIRO_TOLERANCE.as_mm();
        if self
            .preview
            .as_ref()
            .is_some_and(|entry| entry.serves(controls, closed, tolerance))
        {
            return true;
        }
        if self
            .refused
            .as_ref()
            .is_some_and(|(refused, was_closed)| refused == controls && *was_closed == closed)
        {
            return false;
        }
        if let Some(entry) =
            Entry::follow_steadily(self.preview.as_ref(), controls, closed, tolerance, fresh)
        {
            self.preview = Some(entry);
            self.refused = None;
            true
        } else {
            self.refused = Some((controls.to_vec(), closed));
            false
        }
    }

    /// Takes `controls` as the Pen's path in progress for a click: solved near the last preview
    /// when they can be, else from nothing, since every click places a control (lead decision
    /// 2026-11-11, Proposal). Returns whether they have a curve at all.
    pub fn place_preview(&mut self, controls: &[NewAnchor], closed: bool) -> bool {
        self.refused = None;
        if self.try_preview(controls, closed, true) {
            return true;
        }
        let entry = Entry::fresh(controls, closed, curvyo_ui_core::SPIRO_TOLERANCE.as_mm());
        if entry.output.is_none() {
            return false;
        }
        self.preview = Some(entry);
        self.refused = None;
        true
    }

    /// Remembers `controls` as refused for the Pen, so a frame does not solve them again.
    pub fn refuse_preview(&mut self, controls: &[NewAnchor], closed: bool) {
        self.refused = Some((controls.to_vec(), closed));
    }

    /// The controls of the Pen's last accepted preview.
    pub fn preview_controls(&self) -> Option<&[NewAnchor]> {
        self.preview.as_ref().map(|entry| entry.controls.as_slice())
    }

    /// Takes `path`'s live controls in a Node drag when they solve near the last ones taken (at
    /// first, the stored curve's); returns whether they did. A refusal keeps the last ones.
    /// Without `fresh` only the last shape is followed (a step of a held drag's search).
    pub fn try_node(&mut self, document: &Document, path: &PathSnapshot, fresh: bool) -> bool {
        let tolerance = source_tolerance(path, curvyo_ui_core::SPIRO_TOLERANCE.as_mm());
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return false;
        }
        if self
            .node_previews
            .get(&path.id)
            .is_some_and(|entry| entry.serves(&path.anchors, path.closed, tolerance))
        {
            return true;
        }
        if !self.node_previews.contains_key(&path.id) {
            let Some(source) = document.path(path.id) else {
                return false;
            };
            let Some(start) = self.solved(document, &source, tolerance).cloned() else {
                return false;
            };
            self.node_previews.insert(path.id, start);
        }
        let earlier = self.node_previews.get(&path.id);
        match Entry::follow_steadily(earlier, &path.anchors, path.closed, tolerance, fresh) {
            Some(entry) => {
                self.node_previews.insert(path.id, entry);
                true
            }
            None => false,
        }
    }

    /// `path` as a Node drag shows it: the live controls the drag last took, solved, under its
    /// placement.
    pub fn node_preview(
        &mut self,
        document: &Document,
        path: &PathSnapshot,
    ) -> Option<PathSnapshot> {
        let source = document.path(path.id)?;
        if source.curve_kind != CurveKind::Spiro || source.is_compound() {
            return None;
        }
        self.try_node(document, path, true);
        let entry = self.node_previews.get(&path.id)?;
        Some(placed_result(path, &drawn(entry)))
    }

    /// The drag ended with its last taken controls written: they are the stored curves' shapes.
    pub fn commit_node_previews(&mut self) {
        self.entries.extend(self.node_previews.drain());
    }

    pub fn clear_node_previews(&mut self) {
        self.node_previews.clear();
    }

    pub fn clear_preview(&mut self) {
        if let Some(entry) = self.preview.take() {
            self.retired = Some(entry);
        }
        self.refused = None;
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

    /// The solve of `path`'s controls, made now when the memo does not serve it: from the
    /// path's earlier shape (an edit, an undo), or the Pen's preview it was drawn from, or from
    /// scratch; `None` for a path that is not a plain Spiro of the document.
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
                earlier => match earlier.or(self.preview.as_ref()).or(self.retired.as_ref()) {
                    // A path just drawn keeps the very shape the Pen showed.
                    Some(shown) if shown.same_controls(&path.anchors, path.closed) => {
                        shown.at_tolerance(tolerance)
                    }
                    earlier => Entry::follow(earlier, &path.anchors, path.closed, tolerance, true)
                        .unwrap_or_else(|| Entry::fresh(&path.anchors, path.closed, tolerance)),
                },
            };
            self.entries.insert(path.id, entry);
        }
        self.entries.get(&path.id)
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
        let (first, through) = cache.preview(&controls, false).unwrap();
        assert!(first.len() > controls.len(), "the curve, not the controls");
        assert_eq!(through, controls);
        assert_eq!(cache.preview(&controls, false).unwrap().0, first);
        assert!(cache.preview(&controls[..1], false).is_none());
    }

    /// Customer 2026-11-11: near a fold the curve moves far more than its point and past it the
    /// only solution is on another branch; the preview keeps the last curve and the controls it goes through, and takes the next near one again.
    #[test]
    fn a_preview_without_a_near_solution_keeps_the_last_curve() {
        let mut cache = SpiroCache::default();
        let mut controls = smooth(&[(0.0, 0.0), (20.0, 10.0), (40.0, 0.0), (60.0, 10.0)]);
        cache.preview(&controls, false).unwrap();
        for step in 1..=8 {
            controls[2].point = Point::new(40.0, -f64::from(step));
            assert!(cache.try_preview(&controls, false, true), "step {step}");
        }
        let (curve, _) = cache.preview(&controls, false).unwrap();
        let kept = controls.clone();
        controls[2].point = Point::new(40.0, -10.0);
        assert!(!cache.try_preview(&controls, false, true));
        let (held, through) = cache.preview(&controls, false).unwrap();
        assert_eq!(held, curve);
        assert_eq!(through, kept);
        controls[2].point = Point::new(40.0, -7.5);
        let (next, through) = cache.preview(&controls, false).unwrap();
        assert_ne!(next, curve);
        assert_eq!(through, controls);
    }

    /// A drawn path starts from the Pen's preview: its stored curve is the one the Pen showed,
    /// not another solution a fresh solve might find.
    #[test]
    fn a_drawn_path_keeps_the_shape_the_pen_showed() {
        let document = Document::new(1);
        let mut cache = SpiroCache::default();
        let mut controls = smooth(&[(0.0, 0.0), (20.0, 10.0), (40.0, 0.0), (60.0, 10.0)]);
        cache.preview(&controls, false).unwrap();
        for step in 1..=8 {
            controls[2].point = Point::new(40.0, -f64::from(step));
            assert!(cache.try_preview(&controls, false, true));
        }
        let (shown, _) = cache.preview(&controls, false).unwrap();
        let id = document
            .create_editable_curve(&controls, false, CurveKind::Spiro)
            .unwrap();
        cache.clear_preview();
        let path = document.path(id).unwrap();
        assert_eq!(cache.lookup(&document, &path).unwrap().anchors, shown);
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
