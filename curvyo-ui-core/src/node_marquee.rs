//! Which anchors a node marquee selects (`specs/0053-node-marquee-selection/`).
//!
//! **Document order** (criterion 9) is the one order rule for every list of anchors that a
//! marquee or Ctrl+A adds to the node selection, and that Join then reads as "first selected"
//! and "second selected". [`editing_set_anchors`] defines it; [`anchors_in_box`] keeps it. The
//! caller passes the editing set in stacking order, bottom to top (`Session::paths`), so the
//! order is:
//!
//! 1. path by path, in the order of the `paths` slice;
//! 2. inside a path, outline by outline in [`PathSnapshot::all_anchors`] order: the first
//!    outline (`anchors`) from its first anchor to its last, then each further outline
//!    (`extra_subpaths`) in storage order, each from its first anchor to its last;
//! 3. every anchor exactly once, also on a closed outline (its first anchor is not repeated at
//!    the end).
//!
//! The order does not depend on where an anchor lies, on the box or on the drag direction.

use curvyo_document_core::{AnchorId, AnchorSnapshot, NodeId, PathSnapshot, Point, Tolerance};
use curvyo_geometry_core::MAX_COORDINATE_MM;

use crate::conversion::GEOMETRIC_TOLERANCE_MM;

/// How far outside the box an anchor may lie and still count as on its edge: the geometric
/// tolerance (`specs/0053-node-marquee-selection/` criteria 3, 16).
const BOX_TOLERANCE: Tolerance = Tolerance::from_mm(GEOMETRIC_TOLERANCE_MM);

/// A node marquee in flight (`adrs.md` decision 2): the press point, the pointer's last valid
/// document position, whether the drag has left the dead zone, and the anchors inside the box
/// at that position, computed once per pointer event (decision 6) and read by the draw, the
/// legend and the release.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeMarquee {
    down_at: Point,
    current: Point,
    armed: bool,
    inside: Vec<(NodeId, AnchorId)>,
}

impl NodeMarquee {
    /// A press on nothing at `down_at`: not armed yet, so a release now is a click.
    #[must_use]
    pub const fn new(down_at: Point) -> Self {
        Self {
            down_at,
            current: down_at,
            armed: false,
            inside: Vec::new(),
        }
    }

    /// The pointer is at `point`: arms the marquee once it is `threshold` or farther from the
    /// press (criterion 1; it stays armed), and while armed refreshes the anchors of `paths`
    /// inside the box. A point that is not finite is ignored, so the box keeps its last valid
    /// corner (criterion 16).
    pub fn pointer_moved(&mut self, paths: &[PathSnapshot], point: Point, threshold: Tolerance) {
        if !(point.x.is_finite() && point.y.is_finite()) {
            return;
        }
        self.current = point;
        if !self.armed && self.down_at.vector_to(point).length() >= threshold.as_mm() {
            self.armed = true;
        }
        if self.armed {
            self.inside = anchors_in_box(paths, self.down_at, self.current, BOX_TOLERANCE);
        }
    }

    /// Whether the drag has left the dead zone: a release now applies the box, not a click.
    #[must_use]
    pub const fn armed(&self) -> bool {
        self.armed
    }

    /// The press corner and the pointer corner, in document space.
    #[must_use]
    pub const fn corners(&self) -> (Point, Point) {
        (self.down_at, self.current)
    }

    /// The anchors inside the box at the last pointer position, in document order; empty while
    /// not armed.
    #[must_use]
    pub fn inside(&self) -> &[(NodeId, AnchorId)] {
        &self.inside
    }
}

/// "1 node", "5 nodes", "0 nodes".
fn nodes_text(count: usize) -> String {
    if count == 1 {
        "1 node".to_string()
    } else {
        format!("{count} nodes")
    }
}

/// The legend of a node marquee in flight (criterion 10): "5 nodes · Replace", "1 node · +Add",
/// "0 nodes · −Remove" (U+2212, the Select legend's convention).
#[must_use]
pub fn node_marquee_legend_text(count: usize, combine: crate::SelectionCombine) -> String {
    let combine = match combine {
        crate::SelectionCombine::Replace => "Replace",
        crate::SelectionCombine::Add => "+Add",
        crate::SelectionCombine::Remove => "\u{2212}Remove",
    };
    format!("{} \u{b7} {combine}", nodes_text(count))
}

/// The polite announcement after a node marquee or Ctrl+A (criterion 24): "5 nodes selected.",
/// "1 node selected.", "No nodes selected.".
#[must_use]
pub fn nodes_selected_announcement(count: usize) -> String {
    if count == 0 {
        "No nodes selected.".to_string()
    } else {
        format!("{} selected.", nodes_text(count))
    }
}

/// Every anchor of `paths`, each with its path, in document order (this module's doc comment).
/// The Node tool's Ctrl+A selects exactly these (criterion 22); the status-bar count counts
/// them (criterion 23).
pub fn editing_set_anchors(
    paths: &[PathSnapshot],
) -> impl Iterator<Item = (NodeId, &AnchorSnapshot)> {
    paths
        .iter()
        .flat_map(|path| path.all_anchors().map(move |anchor| (path.id, anchor)))
}

/// The anchors of `paths` whose anchor position lies inside the axis-aligned box with the
/// opposite corners `from` and `to`, or on its edge, within `tolerance` (criteria 3, 4, 16), in
/// document order (criterion 9).
///
/// Only anchor positions count: handles and segments are never part of the result, and a
/// handle inside the box does not select its anchor. A corner beyond ±[`MAX_COORDINATE_MM`] is
/// cut at that limit; a corner that is NaN selects nothing. A box of zero width or height
/// selects the anchors on that line within `tolerance`.
#[must_use]
pub fn anchors_in_box(
    paths: &[PathSnapshot],
    from: Point,
    to: Point,
    tolerance: Tolerance,
) -> Vec<(NodeId, AnchorId)> {
    let Some(bounds) = Bounds::of(from, to, tolerance) else {
        return Vec::new();
    };
    editing_set_anchors(paths)
        .filter(|(_, anchor)| bounds.contains(anchor.point))
        .map(|(path, anchor)| (path, anchor.id))
        .collect()
}

/// The box of [`anchors_in_box`], clamped and grown by the tolerance once, so the per-anchor
/// test is four comparisons.
struct Bounds {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Bounds {
    fn of(from: Point, to: Point, tolerance: Tolerance) -> Option<Self> {
        let clamp = |value: f64| value.clamp(-MAX_COORDINATE_MM, MAX_COORDINATE_MM);
        if [from.x, from.y, to.x, to.y].iter().any(|v| v.is_nan()) {
            return None;
        }
        let (fx, fy, tx, ty) = (clamp(from.x), clamp(from.y), clamp(to.x), clamp(to.y));
        let grow = tolerance.as_mm().max(0.0);
        Some(Self {
            min_x: fx.min(tx) - grow,
            min_y: fy.min(ty) - grow,
            max_x: fx.max(tx) + grow,
            max_y: fy.max(ty) + grow,
        })
    }

    fn contains(&self, point: Point) -> bool {
        (self.min_x..=self.max_x).contains(&point.x) && (self.min_y..=self.max_y).contains(&point.y)
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{AnchorKind, Document, NewAnchor, SubpathSnapshot, Vec2};

    use super::*;

    const TOL: Tolerance = Tolerance::from_mm(0.001);

    fn pt(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    fn path(document: &Document, tag: u64, points: &[(f64, f64)], closed: bool) -> PathSnapshot {
        let anchors: Vec<NewAnchor> = points
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| NewAnchor::corner(AnchorId::new(tag, i as u64), pt(x, y)))
            .collect();
        let id = document.create_path(&anchors, closed);
        document.path(id).unwrap()
    }

    fn ids(result: &[(NodeId, AnchorId)]) -> Vec<AnchorId> {
        result.iter().map(|&(_, a)| a).collect()
    }

    /// Criterion 3's example: the handle of (0, 0) reaches into the box, its anchor does not.
    /// Criteria 10 and 24: the legend and announcement texts.
    #[test]
    fn legend_and_announcement_texts() {
        use crate::SelectionCombine;
        assert_eq!(
            node_marquee_legend_text(5, SelectionCombine::Replace),
            "5 nodes \u{b7} Replace"
        );
        assert_eq!(
            node_marquee_legend_text(1, SelectionCombine::Add),
            "1 node \u{b7} +Add"
        );
        assert_eq!(
            node_marquee_legend_text(0, SelectionCombine::Remove),
            "0 nodes \u{b7} \u{2212}Remove"
        );
        assert_eq!(nodes_selected_announcement(5), "5 nodes selected.");
        assert_eq!(nodes_selected_announcement(1), "1 node selected.");
        assert_eq!(nodes_selected_announcement(0), "No nodes selected.");
    }

    /// Criteria 1 and 16: the marquee arms at the threshold, stays armed, keeps its last valid
    /// corner on a non-finite point, and its inside list follows the box.
    #[test]
    fn a_marquee_arms_at_the_threshold_and_tracks_the_box() {
        let document = Document::new(1);
        let paths = vec![path(
            &document,
            5,
            &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)],
            false,
        )];
        let threshold = Tolerance::from_mm(3.0);
        let mut marquee = NodeMarquee::new(pt(5.0, -1.0));
        marquee.pointer_moved(&paths, pt(6.0, 0.0), threshold);
        assert!(!marquee.armed());
        assert_eq!(marquee.inside(), &[]);
        marquee.pointer_moved(&paths, pt(12.0, 1.0), threshold);
        assert!(marquee.armed());
        assert_eq!(ids(marquee.inside()), vec![AnchorId::new(5, 1)]);
        marquee.pointer_moved(&paths, pt(f64::NAN, 1.0), threshold);
        assert_eq!(marquee.corners(), (pt(5.0, -1.0), pt(12.0, 1.0)));
        marquee.pointer_moved(&paths, pt(5.5, 0.0), threshold);
        assert!(marquee.armed(), "back inside the dead zone it stays armed");
        assert_eq!(marquee.inside(), &[]);
    }

    #[test]
    fn only_anchor_positions_inside_the_box_count() {
        let document = Document::new(1);
        let id = document.create_path(
            &[
                NewAnchor {
                    id: AnchorId::new(5, 0),
                    point: pt(0.0, 0.0),
                    handle_in: Vec2::ZERO,
                    handle_out: Vec2::new(6.0, 0.0),
                    kind: AnchorKind::Corner,
                },
                NewAnchor::corner(AnchorId::new(5, 1), pt(10.0, 0.0)),
                NewAnchor::corner(AnchorId::new(5, 2), pt(20.0, 0.0)),
                NewAnchor::corner(AnchorId::new(5, 3), pt(30.0, 0.0)),
            ],
            false,
        );
        let paths = vec![document.path(id).unwrap()];
        let result = anchors_in_box(&paths, pt(5.0, -5.0), pt(25.0, 5.0), TOL);
        assert_eq!(ids(&result), vec![AnchorId::new(5, 1), AnchorId::new(5, 2)]);
        assert!(result.iter().all(|&(p, _)| p == id));
    }

    /// Criterion 3: on the edge counts; the drag direction does not matter.
    #[test]
    fn the_edge_is_inside_and_the_direction_does_not_matter() {
        let document = Document::new(1);
        let paths = vec![path(
            &document,
            5,
            &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)],
            false,
        )];
        let forward = anchors_in_box(&paths, pt(10.0, 0.0), pt(20.0, 5.0), TOL);
        let backward = anchors_in_box(&paths, pt(20.0, 5.0), pt(10.0, 0.0), TOL);
        assert_eq!(
            ids(&forward),
            vec![AnchorId::new(5, 1), AnchorId::new(5, 2)]
        );
        assert_eq!(forward, backward);
    }

    /// Criterion 9's example: P1 (lower) ends at (10, 0) where P2 (upper) starts; the box takes
    /// P1's last node first, then P2's first node.
    #[test]
    fn the_result_is_in_document_order_across_paths() {
        let document = Document::new(1);
        let p1 = path(&document, 1, &[(0.0, 0.0), (10.0, 0.0)], false);
        let p2 = path(&document, 2, &[(10.0, 0.0), (20.0, 0.0)], false);
        let paths = vec![p1.clone(), p2.clone()];
        let result = anchors_in_box(&paths, pt(12.0, 2.0), pt(8.0, -2.0), TOL);
        assert_eq!(
            result,
            vec![(p1.id, AnchorId::new(1, 1)), (p2.id, AnchorId::new(2, 0))]
        );
    }

    /// Criterion 4: a closed path gives every anchor once.
    #[test]
    fn a_closed_path_gives_every_anchor_once() {
        let document = Document::new(1);
        let square = path(
            &document,
            3,
            &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
            true,
        );
        let result = anchors_in_box(&[square], pt(-1.0, -1.0), pt(11.0, 11.0), TOL);
        assert_eq!(result.len(), 4);
    }

    /// Criterion 16: a zero-height box selects the nodes on its line within the tolerance.
    #[test]
    fn a_zero_height_box_selects_the_nodes_on_its_line() {
        let document = Document::new(1);
        let paths = vec![path(
            &document,
            5,
            &[(0.0, 0.0), (10.0, 0.0005), (20.0, 1.0)],
            false,
        )];
        let result = anchors_in_box(&paths, pt(-5.0, 0.0), pt(25.0, 0.0), TOL);
        assert_eq!(ids(&result), vec![AnchorId::new(5, 0), AnchorId::new(5, 1)]);
    }

    /// Criterion 16: corners beyond ±1e7 mm select as if cut there; a NaN corner selects
    /// nothing and does not panic.
    #[test]
    fn hostile_corners_are_clamped_or_select_nothing() {
        let document = Document::new(1);
        let paths = vec![path(&document, 5, &[(0.0, 0.0), (9.0e6, 0.0)], false)];
        let huge = anchors_in_box(&paths, pt(-1.0e9, -1.0e9), pt(f64::INFINITY, 1.0e12), TOL);
        assert_eq!(huge.len(), 2);
        assert_eq!(
            anchors_in_box(&paths, pt(f64::NAN, 0.0), pt(10.0, 10.0), TOL),
            Vec::new()
        );
    }

    /// The order guarantee of the module doc comment: slice order, then `all_anchors` order,
    /// whatever the anchors' positions.
    #[test]
    fn editing_set_anchors_follows_slice_then_anchor_order() {
        let document = Document::new(1);
        let upper_first = path(&document, 1, &[(50.0, 0.0), (40.0, 0.0)], false);
        let lower = path(&document, 2, &[(5.0, 0.0), (0.0, 0.0)], false);
        let paths = vec![upper_first.clone(), lower.clone()];
        let order: Vec<(NodeId, AnchorId)> = editing_set_anchors(&paths)
            .map(|(p, a)| (p, a.id))
            .collect();
        assert_eq!(
            order,
            vec![
                (upper_first.id, AnchorId::new(1, 0)),
                (upper_first.id, AnchorId::new(1, 1)),
                (lower.id, AnchorId::new(2, 0)),
                (lower.id, AnchorId::new(2, 1)),
            ]
        );
    }

    /// The order guarantee for a path with several outlines (`specs/0055-compound-path-node-
    /// editing/adrs.md`): the first outline first to last, then each further outline in
    /// storage order, before the next path.
    #[test]
    fn editing_set_anchors_walks_every_outline_in_order() {
        let document = Document::new(1);
        let mut compound = path(&document, 1, &[(0.0, 0.0), (10.0, 0.0)], false);
        let second = path(&document, 2, &[(0.0, 5.0), (10.0, 5.0)], true);
        compound.extra_subpaths.push(SubpathSnapshot {
            closed: true,
            anchors: second.anchors.clone(),
        });
        let next = path(&document, 3, &[(0.0, 9.0), (1.0, 9.0)], false);
        let paths = vec![compound.clone(), next.clone()];
        let order: Vec<AnchorId> = editing_set_anchors(&paths).map(|(_, a)| a.id).collect();
        assert_eq!(
            order,
            vec![
                AnchorId::new(1, 0),
                AnchorId::new(1, 1),
                AnchorId::new(2, 0),
                AnchorId::new(2, 1),
                AnchorId::new(3, 0),
                AnchorId::new(3, 1),
            ]
        );
        let boxed = anchors_in_box(&paths, pt(5.0, 4.0), pt(11.0, 10.0), TOL);
        assert_eq!(
            boxed,
            vec![(compound.id, AnchorId::new(2, 1))],
            "an anchor of a further outline is found with its path's id"
        );
    }
}
