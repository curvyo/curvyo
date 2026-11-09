//! Unlink and the removal repair of a clone whose root is a group, a text, an image or a
//! primitive (ADR 0045 §5): the clone is replaced, in its place, by a copy of its root's subtree
//! under its composed matrix, and the clones that followed it follow the copy.

use loro::TreeID;

use crate::affine::Affine;
use crate::clone_model::CloneSnapshot;
use crate::clones::CloneError;
use crate::document::{Document, OBJECTS_TREE};
use crate::objects::CopySource;
use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;
use crate::units::{Angle, Point, Vec2};

/// Below this a matrix entry counts as zero for "the map keeps a primitive one".
const RIGID_EPS: f64 = 1e-12;

impl Document {
    /// Whether the clone `clone` unlinks into a copy of its root rather than into a path: its
    /// root is a group, a text, an image or a primitive.
    pub(crate) fn unlinks_as_copy(&self, clone: &CloneSnapshot) -> bool {
        clone.is_picture()
            || matches!(
                self.object(clone.chain.root),
                Some(ObjectSnapshot::Primitive(_))
            )
    }

    /// Turns `clone` into an independent copy of its root under its composed matrix, keeping its
    /// id and place, uncommitted; every clone that followed it keeps following it (the copy draws
    /// what the clone drew). A group root's own hidden flag does not hide the copy (ADR 0045 §2).
    /// Returns the id (the clone's).
    ///
    /// # Errors
    /// [`CloneError::NoSuchObject`] when the root or the clone is gone; nothing is written.
    pub(crate) fn unlink_as_copy_uncommitted(
        &self,
        clone: &CloneSnapshot,
    ) -> Result<NodeId, CloneError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let root = clone.chain.root;
        let root_node = TreeID::new(root.peer, root.counter);
        let clone_node = TreeID::new(clone.id().peer, clone.id().counter);
        if !crate::path_codec::node_exists(&tree, root_node)
            || !crate::path_codec::node_exists(&tree, clone_node)
        {
            return Err(CloneError::NoSuchObject);
        }
        let anchors = crate::group_tree::anchors_under(&tree, root_node);
        let anchor_ids = self.repair_anchor_ids(anchors);
        let copy = self
            .uncommitted(|doc| {
                doc.duplicate_objects(
                    &[CopySource {
                        id: root,
                        anchor_ids,
                    }],
                    Vec2::ZERO,
                )
            })
            .map_err(|_| CloneError::NoSuchObject)?
            .first()
            .copied()
            .ok_or(CloneError::NoSuchObject)?;
        let leaves = if self.is_group(copy) {
            self.leaves_of(copy)
        } else {
            vec![copy]
        };
        let m = clone.chain.composed;
        let links = self.clone_links();
        let matrices: std::collections::HashMap<NodeId, Affine> =
            crate::clone_transform::clone_matrices_after(
                &links,
                &crate::HeldClones::new(),
                &leaves,
                m,
            )
            .matrices
            .into_iter()
            .collect();
        let results: Vec<ObjectSnapshot> = leaves
            .iter()
            .filter_map(|&leaf| {
                let object = self.object(leaf)?;
                Some(match &object {
                    ObjectSnapshot::Clone(inner) => {
                        let mut mapped = inner.mapped_by(m);
                        if let Some(&matrix) = matrices.get(&leaf) {
                            mapped.chain.matrix = matrix;
                        }
                        ObjectSnapshot::Clone(mapped)
                    }
                    ObjectSnapshot::Primitive(_) => self.mapped_primitive(&object, m),
                    _ => crate::clone_picture::map_leaf(&object, m),
                })
            })
            .collect();
        self.uncommitted(|doc| doc.transform_objects(&results, false))
            .map_err(|_| CloneError::NoSuchObject)?;
        // The clone's own node becomes the copy (ADR 0045 §5, as `0049` keeps the id): its keys
        // are replaced by the copy's, the copy's children move under it and the copy goes.
        // Clones that followed it keep following the same id.
        let copy_node = TreeID::new(copy.peer, copy.counter);
        let (Ok(copy_meta), Ok(clone_meta)) = (tree.get_meta(copy_node), tree.get_meta(clone_node))
        else {
            return Err(CloneError::NoSuchObject);
        };
        for key in clone_meta.keys() {
            if !crate::objects::LINEAGE_KEYS.contains(&&*key) {
                let _ = clone_meta.delete(&key);
            }
        }
        crate::objects::copy_map(&copy_meta, &clone_meta, crate::objects::LINEAGE_KEYS);
        if !crate::node_registers::read_registers(&clone_meta).visible {
            // invariant: a plain value under a known key on a live map.
            #[allow(clippy::unwrap_used)]
            clone_meta
                .insert(crate::node_registers::KEY_VISIBLE, true)
                .unwrap();
        }
        tree.enable_fractional_index(0);
        for (index, child) in tree
            .children(copy_node)
            .unwrap_or_default()
            .into_iter()
            .enumerate()
        {
            // invariant: both nodes are live; a child of the copy never holds the clone.
            #[allow(clippy::unwrap_used)]
            tree.mov_to(child, loro::TreeParentId::Node(clone_node), index)
                .unwrap();
        }
        let parents = crate::group_tree::parents_of(&tree, &[copy_node]);
        // Nothing follows the empty copy; the one removal rule still runs.
        self.repair_clones_of_removed(&tree, &[copy_node]);
        // invariant: the copy was created above.
        #[allow(clippy::unwrap_used)]
        tree.delete(copy_node).unwrap();
        crate::groups::remove_emptied_groups(&tree, &parents);
        let copy = clone.id();
        Ok(copy)
    }

    /// A copied primitive under `m`: still a primitive when `m` is a translation or a turn
    /// (ADR 0045 §5), else its outline as a path with fresh anchor ids (converted silently, as
    /// `0047` converts on skew).
    fn mapped_primitive(&self, object: &ObjectSnapshot, m: Affine) -> ObjectSnapshot {
        let ObjectSnapshot::Primitive(primitive) = object else {
            return object.clone();
        };
        let linear_rigid = (m.a - m.d).abs() < RIGID_EPS
            && (m.b + m.c).abs() < RIGID_EPS
            && (m.a.mul_add(m.d, -(m.b * m.c)) - 1.0).abs() < 1e-9;
        if linear_rigid {
            let angle = Angle::from_radians(m.b.atan2(m.a));
            let turned = object.rotated(Point::new(0.0, 0.0), angle);
            return turned.translated(Vec2::new(m.e, m.f));
        }
        let mut path = crate::clone_picture::primitive_path(primitive).mapped(m);
        let ids = self.repair_anchor_ids(path.all_anchors().count());
        for (anchor, id) in path.all_anchors_mut().zip(ids) {
            anchor.id = id;
        }
        ObjectSnapshot::Path(path)
    }
}
