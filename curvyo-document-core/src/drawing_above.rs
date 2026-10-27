//! Writing drawing nodes directly above existing objects (`specs/0025-raster-trace` criteria 13,
//! 14 and 46): each node lands in its object's parent, right above it in the stacking order, all
//! in one commit. Existing node kinds and registers only; no format change.

use crate::document::{Document, OBJECTS_TREE};
use crate::drawing_insert::{DrawingNode, Writer};
use crate::group_tree::{groups_above, is_layer_node, node_id};
use crate::objects::ObjectEditError;
use crate::path_codec::node_exists;
use crate::path_model::{AnchorId, NodeId};
use crate::paths::tree_id_of;

/// The label of the Trace tool's commit; its history name is "Trace image".
pub const TRACE_IMAGE_COMMIT_LABEL: &str = "trace_image";

impl Document {
    /// Writes each `(object, node)` of `items` as a new node directly above `object`, in its
    /// parent (a layer, a group or the root), in **one commit** labelled `label`. `anchor_ids`
    /// are the paths' anchor ids, [`DrawingNode::anchor_count`] in item order. Clones and texts
    /// on path link within one item only. Returns the new nodes in item order.
    ///
    /// Everything is checked before the first write.
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] when an object does not exist;
    /// [`ObjectEditError::IsALayer`] when an object is a layer; the errors of
    /// [`Document::insert_drawing`] for the nodes (nothing is written).
    ///
    /// # Panics
    /// Does not panic in practice: every tree operation acts on nodes just confirmed live or
    /// just created.
    pub fn insert_drawing_above(
        &self,
        label: &str,
        items: &[(NodeId, DrawingNode)],
        anchor_ids: &[AnchorId],
    ) -> Result<Vec<NodeId>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut depth = 0;
        for (object, _) in items {
            let id = tree_id_of(*object);
            if !node_exists(&tree, id) {
                return Err(ObjectEditError::NoSuchObject);
            }
            if is_layer_node(&tree, id) {
                return Err(ObjectEditError::IsALayer);
            }
            depth = depth.max(groups_above(&tree, id));
        }
        let nodes: Vec<DrawingNode> = items.iter().map(|(_, node)| node.clone()).collect();
        crate::drawing_check::validate(self, &tree, &nodes, anchor_ids, depth, &[])?;
        Ok(self.batch(label, |document| {
            tree.enable_fractional_index(0);
            let mut ids = anchor_ids.iter().copied();
            let mut created = Vec::with_capacity(items.len());
            for (object, node) in items {
                let below = tree_id_of(*object);
                // invariant: checked live above.
                #[allow(clippy::unwrap_used)]
                let parent = tree.parent(below).unwrap();
                let mut writer = Writer {
                    document,
                    tree: tree.clone(),
                    ids: ids.by_ref(),
                    leaves: Vec::new(),
                    clones: Vec::new(),
                    links: Vec::new(),
                    assets: &[],
                };
                let new = writer.write_one(parent, node);
                writer.link_clones();
                // invariant: both nodes are live siblings and the fractional index is on.
                #[allow(clippy::unwrap_used)]
                tree.mov_after(new, below).unwrap();
                created.push(node_id(new));
            }
            created
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::drawing_insert::{DrawingAnchor, DrawingOutline, DrawingRegisters};
    use crate::path_model::AnchorKind;
    use crate::style_model::Style;
    use crate::units::{Point, Vec2};

    fn square(name: &str) -> DrawingNode {
        let anchors = [(0.0, 0.0), (5.0, 0.0), (5.0, 5.0)]
            .iter()
            .map(|&(x, y)| DrawingAnchor {
                point: Point::new(x, y),
                handle_in: Vec2::new(0.0, 0.0),
                handle_out: Vec2::new(0.0, 0.0),
                kind: AnchorKind::Corner,
            })
            .collect();
        DrawingNode::Group {
            registers: DrawingRegisters {
                name: Some(name.to_string()),
                ..DrawingRegisters::default()
            },
            layer: false,
            children: vec![DrawingNode::Path {
                registers: DrawingRegisters::default(),
                outlines: vec![DrawingOutline {
                    closed: true,
                    anchors,
                }],
                style: Style::default(),
            }],
        }
    }

    #[test]
    fn nodes_land_directly_above_their_objects_in_one_step() {
        let doc = Document::new(1);
        let rect = |x: f64| {
            doc.create_rect(crate::RectBounds {
                origin: Point::new(x, 0.0),
                width: crate::Length::from_mm(10.0),
                height: crate::Length::from_mm(10.0),
            })
        };
        let (a, b) = (rect(0.0), rect(20.0));
        let steps = doc.own_step_count();
        let ids: Vec<AnchorId> = (0..6).map(|i| AnchorId::new(9, i)).collect();
        let created = doc
            .insert_drawing_above(
                TRACE_IMAGE_COMMIT_LABEL,
                &[(a, square("Trace of a")), (b, square("Trace of b"))],
                &ids,
            )
            .unwrap();
        assert_eq!(doc.own_step_count(), steps + 1);
        assert_eq!(created.len(), 2);
        // Leaves bottom to top: each trace's path sits right above its object.
        let order = doc.object_ids();
        assert_eq!(order.len(), 4);
        assert_eq!((order[0], order[2]), (a, b));
        assert_eq!(doc.parent_group(order[1]), Some(created[0]));
        assert_eq!(doc.parent_group(order[3]), Some(created[1]));
    }

    #[test]
    fn a_missing_object_writes_nothing() {
        let doc = Document::new(1);
        let steps = doc.own_step_count();
        let ids: Vec<AnchorId> = (0..3).map(|i| AnchorId::new(9, i)).collect();
        let result = doc.insert_drawing_above(
            TRACE_IMAGE_COMMIT_LABEL,
            &[(NodeId::from_parts(77, 77), square("x"))],
            &ids,
        );
        assert_eq!(result, Err(ObjectEditError::NoSuchObject));
        assert_eq!(doc.own_step_count(), steps);
    }
}
