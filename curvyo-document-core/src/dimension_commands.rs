//! Editing dimension objects (ADR 0027 §5): leaving one, moving its line and text, turning it
//! into paths and a text, and pointing copied dimensions at copied geometry.

use std::collections::{HashMap, HashSet};

use loro::{LoroTree, TreeID, TreeParentId};

use crate::dimension_codec::{
    is_dimension_meta, read_dimension_snapshot, write_dimension, write_place, write_slots,
};
use crate::dimension_model::{
    CONVERT_DIMENSION_COMMIT_LABEL, CREATE_DIMENSION_COMMIT_LABEL, CurveSlot, DimensionKind,
    DimensionPlace, DimensionSlots, DimensionSnapshot, EDIT_DIMENSION_NODES_COMMIT_LABEL,
    MOVE_DIMENSION_COMMIT_LABEL, PointSlot,
};
use crate::document::{Document, OBJECTS_TREE};
use crate::group_tree::{children_of, node_id};
use crate::objects::ObjectEditError;
use crate::path_codec::leaf_exists;
use crate::path_model::{AnchorId, NewAnchor, NodeId};
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;
use crate::style_model::Style;
use crate::text_model::TextSnapshot;
use crate::units::Vec2;

/// One plain path of a converted dimension: its anchors (ids minted by the caller), whether it
/// is closed, and its style.
#[derive(Debug, Clone, PartialEq)]
pub struct ConvertedPath {
    /// The anchors.
    pub anchors: Vec<NewAnchor>,
    /// Whether the last anchor joins the first (an arrowhead).
    pub closed: bool,
    /// Its stroke and fill.
    pub style: Style,
}

/// What one dimension becomes when converted (ADR 0027 §5): the caller lays out the marks for
/// the view at that moment.
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionConversion {
    /// The dimension.
    pub id: NodeId,
    /// Its lines and arrowheads.
    pub paths: Vec<ConvertedPath>,
    /// Its value text, as an editable text object (its `id` is ignored); `None` only while no
    /// face is loaded.
    pub text: Option<TextSnapshot>,
}

impl Document {
    /// The objects a dimension's slots refer to, read once each.
    fn referents(&self, slots: &DimensionSlots) -> HashMap<NodeId, ObjectSnapshot> {
        slots
            .nodes()
            .into_iter()
            .filter_map(|node| Some((node, self.object(node)?)))
            .collect()
    }

    /// The dimension `id` resolved against the current geometry, or `None` when it is not a
    /// live dimension.
    #[must_use]
    pub fn dimension(&self, id: NodeId) -> Option<DimensionSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = tree_id_of(id);
        if !leaf_exists(&tree, tree_id) {
            return None;
        }
        let meta = tree.get_meta(tree_id).ok()?;
        if !is_dimension_meta(&meta) {
            return None;
        }
        let mut dimension = read_dimension_snapshot(id, &meta)?;
        let referents = self.referents(&dimension.slots);
        crate::dimension_read::resolve_into(&mut dimension, |node| referents.get(&node));
        Some(dimension)
    }

    /// The linked dimensions that a move or transform of the nodes `moved` leaves unchanged
    /// ([`crate::dimension_model::staying_dimensions`] over the stored dimensions).
    pub(crate) fn staying_dimensions(&self, moved: &HashSet<NodeId>) -> HashSet<NodeId> {
        crate::dimension_model::staying_dimensions(moved, |id| self.dimension(id))
    }

    /// Leaves a dimension of `kind` over `slots` at `place` on top of the active layer, in one
    /// commit "Dimension" (criterion 4), and returns its id. Its kept value is what it measures
    /// now.
    ///
    /// # Errors
    /// [`ObjectEditError::InvalidConversion`] when the slots do not fit the kind, the place is not
    /// finite, or the slots do not resolve against the current geometry (nothing is written).
    ///
    /// # Panics
    /// Does not panic in practice: every write goes to the node this call creates.
    pub fn add_dimension(
        &self,
        kind: DimensionKind,
        slots: DimensionSlots,
        place: DimensionPlace,
    ) -> Result<NodeId, ObjectEditError> {
        if !slots.fit(kind) || !place.is_valid() {
            return Err(ObjectEditError::InvalidConversion);
        }
        let referents = self.referents(&slots);
        let probe = DimensionSnapshot::new(
            NodeId::from_parts(0, 0),
            kind,
            slots,
            place,
            crate::dimension_model::Measured {
                points: Vec::new(),
                value: crate::dimension_model::DimensionValue::Length(
                    crate::units::Length::from_mm(0.0),
                ),
            },
        );
        let kept = crate::dimension_read::resolve(&probe, |node| referents.get(&node))
            .ok_or(ObjectEditError::InvalidConversion)?;
        let dimension = DimensionSnapshot::new(probe.id, kind, probe.slots, place, kept);
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // invariant: creating a node under the root or a live group cannot fail.
        #[allow(clippy::unwrap_used)]
        let node = tree.create(self.drawing_parent(&tree)).unwrap();
        // invariant: the meta map of a node just created can be read.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(node).unwrap();
        write_dimension(&meta, &dimension);
        self.forget_dimension_scan();
        self.commit_with_label(CREATE_DIMENSION_COMMIT_LABEL);
        Ok(node_id(node))
    }

    /// Moves the line and text of the dimension `id` to `place`, in one commit "Move dimension"
    /// (criterion 8). Writes nothing when the place is unchanged.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] when `id` is not a live dimension;
    /// [`ObjectEditError::NonFiniteGeometry`] for a place that is not finite.
    pub fn set_dimension_place(
        &self,
        id: NodeId,
        place: DimensionPlace,
    ) -> Result<(), ObjectEditError> {
        if !place.is_valid() {
            return Err(ObjectEditError::NonFiniteGeometry);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta = tree
            .get_meta(tree_id_of(id))
            .ok()
            .filter(|meta| leaf_exists(&tree, tree_id_of(id)) && is_dimension_meta(meta))
            .ok_or(ObjectEditError::NoSuchObject)?;
        let stored = read_dimension_snapshot(id, &meta).ok_or(ObjectEditError::NoSuchObject)?;
        if stored.place != place {
            write_place(&meta, place);
            self.commit_with_label(MOVE_DIMENSION_COMMIT_LABEL);
        }
        Ok(())
    }

    /// Moves kept point `index` of the unlinked dimension `id` to `to`, in one commit "Edit
    /// nodes"; its value never changes (criterion 13).
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] when `id` is not a live dimension, is linked, or has no
    /// point `index`; [`ObjectEditError::NonFiniteGeometry`] for a point that is not finite.
    pub fn move_dimension_point(
        &self,
        id: NodeId,
        index: usize,
        to: crate::units::Point,
    ) -> Result<(), ObjectEditError> {
        if !(to.x.is_finite() && to.y.is_finite()) {
            return Err(ObjectEditError::NonFiniteGeometry);
        }
        let dimension = self.dimension(id).ok_or(ObjectEditError::NoSuchObject)?;
        if dimension.linked || index >= dimension.kept.points.len() {
            return Err(ObjectEditError::NoSuchObject);
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta = tree
            .get_meta(tree_id_of(id))
            .map_err(|_| ObjectEditError::NoSuchObject)?;
        let mut kept = dimension.kept;
        kept.points[index] = to;
        crate::dimension_codec::write_kept(&meta, &kept);
        self.commit_with_label(EDIT_DIMENSION_NODES_COMMIT_LABEL);
        Ok(())
    }

    /// Replaces each dimension of `conversions`, in place, by a group of its paths and its text,
    /// in one commit "Convert to path" (criterion 10), and returns the groups in order.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] when an id is not a live dimension (nothing is written).
    ///
    /// # Panics
    /// Does not panic in practice: every tree operation acts on nodes confirmed live or just made.
    pub fn convert_dimensions(
        &self,
        conversions: &[DimensionConversion],
    ) -> Result<Vec<NodeId>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        for conversion in conversions {
            let tree_id = tree_id_of(conversion.id);
            let live = leaf_exists(&tree, tree_id)
                && tree
                    .get_meta(tree_id)
                    .is_ok_and(|meta| is_dimension_meta(&meta));
            if !live {
                return Err(ObjectEditError::NoSuchObject);
            }
        }
        Ok(self.batch(CONVERT_DIMENSION_COMMIT_LABEL, |document| {
            conversions
                .iter()
                .map(|conversion| document.convert_one(&tree, conversion))
                .collect()
        }))
    }

    fn convert_one(&self, tree: &LoroTree, conversion: &DimensionConversion) -> NodeId {
        let source = tree_id_of(conversion.id);
        // invariant: the caller confirmed the dimension live.
        #[allow(clippy::unwrap_used)]
        let parent = tree.parent(source).unwrap();
        let parent_key = match parent {
            TreeParentId::Node(group) => Some(group),
            _ => None,
        };
        let at = children_of(tree, parent_key)
            .iter()
            .position(|&child| child == source)
            .unwrap_or(0);
        tree.enable_fractional_index(0);
        // invariant: the parent is live and the index is at most the child count.
        #[allow(clippy::unwrap_used)]
        let group = tree.create_at(parent, at + 1).unwrap();
        // invariant: the node was just created.
        #[allow(clippy::unwrap_used)]
        tree.get_meta(group)
            .unwrap()
            .insert(crate::group_tree::KEY_GROUP, true)
            .unwrap();
        for path in &conversion.paths {
            self.create_path_uncommitted(
                TreeParentId::Node(group),
                &path.anchors,
                path.closed,
                &path.style,
                crate::units::Angle::from_radians(0.0),
            );
        }
        if let Some(text) = &conversion.text {
            let text = self.create_text_uncommitted(text);
            // invariant: the text and the group were just made; the group is not below the text.
            #[allow(clippy::unwrap_used)]
            tree.mov(tree_id_of(text), group).unwrap();
        }
        // A dimension is never followed by a clone or a text; the repair is the removal rule.
        self.repair_clones_of_removed(tree, &[source]);
        // invariant: the dimension is live.
        #[allow(clippy::unwrap_used)]
        tree.delete(source).unwrap();
        // The group took the dimension's place, so no group empties; kept for the delete rule.
        crate::groups::remove_emptied_groups(tree, &[parent_key]);
        node_id(group)
    }
}

/// The anchor ids of the object `id` in their stored order (all outlines), for a positional map
/// between an object and its copy; empty for an object without anchors.
fn anchor_order(tree: &LoroTree, id: NodeId) -> Vec<AnchorId> {
    crate::shapes::object_in(tree, id)
        .and_then(|object| match object {
            ObjectSnapshot::Path(path) => Some(path.all_anchors().map(|a| a.id).collect()),
            ObjectSnapshot::Clone(clone) => {
                Some(clone.resolved.all_anchors().map(|a| a.id).collect())
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// The free circle a curve dimension showed as `shown` (centre, rim point), moved by `offset`.
fn free_circle(shown: &crate::dimension_model::Measured, offset: Vec2) -> CurveSlot {
    let center = shown.points[0];
    CurveSlot::FreeCircle {
        center: center.translated(offset),
        radius: crate::units::Length::from_mm(center.vector_to(shown.points[1]).length()),
    }
}

/// Points every copied dimension at the copies of what it measured when those were copied too
/// (node ids through `made`, anchor ids by position), and turns a slot whose node was not copied
/// into a free point or a free circle where it measured, moved by `offset` (ADR 0027 §5, as
/// `relink_copied_texts`). The copies are uncommitted.
pub(crate) fn relink_copied_dimensions(
    document: &Document,
    tree: &LoroTree,
    made: &[(TreeID, TreeID)],
    offset: Vec2,
) {
    let copy_of: HashMap<NodeId, NodeId> = made
        .iter()
        .map(|(source, copy)| (node_id(*source), node_id(*copy)))
        .collect();
    for &(source, copy) in made {
        let Ok(meta) = tree.get_meta(copy) else {
            continue;
        };
        if !is_dimension_meta(&meta) {
            continue;
        }
        document.forget_dimension_scan();
        let Some(original) = document.dimension(node_id(source)) else {
            continue;
        };
        let mut slots = original.slots.clone();
        let remap_anchor = |node: NodeId, anchor: AnchorId| -> Option<(NodeId, AnchorId)> {
            let copied = *copy_of.get(&node)?;
            let at = anchor_order(tree, node).iter().position(|&a| a == anchor)?;
            Some((copied, *anchor_order(tree, copied).get(at)?))
        };
        match &mut slots {
            DimensionSlots::Points(points) => {
                for (i, slot) in points.iter_mut().enumerate() {
                    let moved = match *slot {
                        PointSlot::Anchor { node, anchor } => remap_anchor(node, anchor)
                            .map(|(node, anchor)| PointSlot::Anchor { node, anchor }),
                        PointSlot::SegmentMid { node, anchor } => remap_anchor(node, anchor)
                            .map(|(node, anchor)| PointSlot::SegmentMid { node, anchor }),
                        PointSlot::ShapePoint { node, feature } => copy_of
                            .get(&node)
                            .map(|&node| PointSlot::ShapePoint { node, feature }),
                        PointSlot::Free { point } => Some(PointSlot::Free {
                            point: point.translated(offset),
                        }),
                    };
                    *slot = moved.unwrap_or_else(|| PointSlot::Free {
                        point: original.shown.points[i].translated(offset),
                    });
                }
            }
            DimensionSlots::Curve(curve) => {
                let linked = match *curve {
                    CurveSlot::Circle { node } => {
                        copy_of.get(&node).map(|&node| CurveSlot::Circle { node })
                    }
                    CurveSlot::RoundCorner { node, corner } => copy_of
                        .get(&node)
                        .map(|&node| CurveSlot::RoundCorner { node, corner }),
                    CurveSlot::Arc { node, anchor } => remap_anchor(node, anchor)
                        .map(|(node, anchor)| CurveSlot::Arc { node, anchor }),
                    CurveSlot::FreeCircle { .. } => continue,
                };
                *curve = linked.unwrap_or_else(|| free_circle(&original.shown, offset));
            }
        }
        write_slots(&meta, &slots);
    }
}

#[cfg(test)]
#[path = "dimension_commands_tests.rs"]
mod tests;
