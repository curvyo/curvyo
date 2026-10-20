//! Loading the outlines of another Curvyo document into this one as guides
//! (`specs/0046-document-shapes` criteria 20, 22 and 23, ADR 0019 §4, `adrs.md` decision 4).
//!
//! Pure: the caller reads the file and opens it with [`crate::unpack`]; this module only picks
//! the outlines of that document and writes copies of them here, with their coordinates unchanged.

use std::collections::HashSet;

use crate::document::{Document, OBJECTS_TREE};
use crate::node_read::NodeKind;
use crate::objects::ObjectEditError;
use crate::path_model::{AnchorId, NewAnchor, NodeId};
use crate::paths::tree_id_of;
use crate::primitive_model::ObjectSnapshot;

/// The label of a template load's commit; its history name is "Template".
pub const LOAD_TEMPLATE_COMMIT_LABEL: &str = "load_template";

/// One outline of a template source: a path or a shape (a clone already resolved to its path),
/// with the name it had there.
#[derive(Debug, Clone, PartialEq)]
pub struct TemplateNode {
    /// The geometry and stored style; never [`ObjectSnapshot::Clone`].
    pub object: ObjectSnapshot,
    /// The node's own name in the source, if it had one.
    pub name: Option<String>,
}

impl TemplateNode {
    /// How many anchor ids its copy needs: one per anchor of a path, none for a shape.
    #[must_use]
    pub fn anchor_count(&self) -> usize {
        match &self.object {
            ObjectSnapshot::Path(path) => {
                path.anchors.len()
                    + path
                        .extra_subpaths
                        .iter()
                        .map(|sub| sub.anchors.len())
                        .sum::<usize>()
            }
            ObjectSnapshot::Primitive(_) | ObjectSnapshot::Clone(_) | ObjectSnapshot::Text(_) => 0,
        }
    }
}

impl Document {
    /// The outlines this document gives as a template (criterion 20): its guides when it has
    /// any, otherwise every visible path, compound path and shape. A linked clone comes as the
    /// path it shows. In stacking order, bottom first.
    #[must_use]
    pub fn template_nodes(&self) -> Vec<TemplateNode> {
        let leaves: Vec<_> = self
            .read_nodes()
            .into_iter()
            .filter(|read| read.object().is_some())
            .collect();
        let has_guides = leaves.iter().any(|read| read.guide);
        leaves
            .into_iter()
            .filter(|read| if has_guides { read.guide } else { read.visible })
            .filter_map(|read| {
                let object = match read.kind {
                    NodeKind::Leaf(ObjectSnapshot::Clone(clone)) => {
                        ObjectSnapshot::Path(clone.resolved)
                    }
                    NodeKind::Leaf(object) => object,
                    NodeKind::Layer | NodeKind::Group => return None,
                };
                Some(TemplateNode {
                    object,
                    name: read.registers.name,
                })
            })
            .collect()
    }

    /// Adds `nodes` as guides at the top of the draw parent (the active layer, `0050`), in their
    /// order, with their coordinates, style and names, in one commit labelled
    /// [`LOAD_TEMPLATE_COMMIT_LABEL`] (criterion 23). `anchor_ids` are the copies' anchor ids,
    /// [`TemplateNode::anchor_count`] per node in order. Returns the new ids; with no node it
    /// writes nothing and returns none.
    ///
    /// # Errors
    /// [`ObjectEditError::AnchorIds`] if `anchor_ids` are not exactly as many as needed, not all
    /// different, or one already names an anchor of this document (nothing is written).
    ///
    /// # Panics
    /// Does not panic in practice: every write goes to a node this call just created.
    pub fn load_template(
        &self,
        nodes: &[TemplateNode],
        anchor_ids: &[AnchorId],
    ) -> Result<Vec<NodeId>, ObjectEditError> {
        let needed: usize = nodes.iter().map(TemplateNode::anchor_count).sum();
        let seen: HashSet<AnchorId> = anchor_ids.iter().copied().collect();
        if anchor_ids.len() != needed || seen.len() != needed {
            return Err(ObjectEditError::AnchorIds);
        }
        if nodes.is_empty() {
            return Ok(Vec::new());
        }
        let tree = self.loro().get_tree(OBJECTS_TREE);
        crate::replace::check_new_anchor_ids(&tree, &[], &seen)?;
        let mut ids = anchor_ids.iter().copied();
        let mut created = Vec::with_capacity(nodes.len());
        for node in nodes {
            let id = self.write_template_node(&node.object, &mut ids);
            // invariant: the node was just created.
            #[allow(clippy::unwrap_used)]
            let meta = tree.get_meta(tree_id_of(id)).unwrap();
            crate::guides::write_guide(&meta, true);
            if let Some(name) = &node.name {
                // invariant: an insert on a live map does not fail.
                #[allow(clippy::unwrap_used)]
                meta.insert(crate::node_registers::KEY_NAME, name.as_str())
                    .unwrap();
            }
            created.push(id);
        }
        self.commit_with_label(LOAD_TEMPLATE_COMMIT_LABEL);
        Ok(created)
    }

    /// Writes one copy without committing: a path renumbered from `ids`, a shape as it is.
    fn write_template_node(
        &self,
        object: &ObjectSnapshot,
        ids: &mut impl Iterator<Item = AnchorId>,
    ) -> NodeId {
        let path = match object {
            ObjectSnapshot::Primitive(primitive) => {
                let id = self.create_primitive_uncommitted(primitive.shape, primitive.rotation);
                let tree = self.loro().get_tree(OBJECTS_TREE);
                // invariant: the node was just created.
                #[allow(clippy::unwrap_used)]
                let meta = tree.get_meta(tree_id_of(id)).unwrap();
                let created = crate::style_codec::read_style(&meta);
                crate::style_codec::write_changes(&meta, &created, &primitive.style);
                return id;
            }
            ObjectSnapshot::Text(text) => return self.create_text_uncommitted(text),
            ObjectSnapshot::Path(path) => path,
            ObjectSnapshot::Clone(clone) => &clone.resolved,
        };
        let mut renumber = |anchors: &[NewAnchor]| -> Vec<NewAnchor> {
            anchors
                .iter()
                .map(|anchor| NewAnchor {
                    // invariant: `load_template` checked one id per anchor.
                    #[allow(clippy::unwrap_used)]
                    id: ids.next().unwrap(),
                    ..*anchor
                })
                .collect()
        };
        let first = renumber(&path.anchors);
        let rest: Vec<(Vec<NewAnchor>, bool)> = path
            .extra_subpaths
            .iter()
            .map(|sub| (renumber(&sub.anchors), sub.closed))
            .collect();
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let id = self.create_path_uncommitted(
            self.drawing_parent(&tree),
            &first,
            path.closed,
            &path.style,
            path.rotation,
        );
        if !rest.is_empty() {
            // invariant: the node was just created.
            #[allow(clippy::unwrap_used)]
            let meta = tree.get_meta(tree_id_of(id)).unwrap();
            crate::subpath_codec::write_extra_subpaths(&meta, &rest);
        }
        id
    }
}
