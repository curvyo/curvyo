//! Reading objects out of the Loro tree with their linked clones resolved: the whole document in
//! one pass ([`Document::read_objects`]) or one clone through its own chain.

use std::collections::HashSet;

use loro::{LoroMap, LoroTree, TreeID};

use crate::clone_codec::{is_clone_meta, read_clone_link};
use crate::clone_model::CloneSnapshot;
use crate::clone_picture::Structure;
use crate::clone_read::{ObjectRead, RawLeaf, RawOverrides, resolve, resolve_in};
use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::{leaf_exists, read_path_snapshot};
use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;
use crate::shape_codec::{read_primitive_snapshot, read_shape_tag};

impl Document {
    /// Every object in stacking order with every linked clone resolved, the direct clones of each
    /// object and the orphans, in one pass over the tree (`specs/0049-linked-clone/adrs.md`
    /// decision 3). The session reads this once per document version.
    #[must_use]
    pub fn read_objects(&self) -> ObjectRead {
        let (leaves, structure) = self.raw_leaves_with_structure();
        resolve_in(leaves, Some(&structure)).into_read()
    }

    /// Every leaf as stored, in stacking order, with the containers and registers a clone of a
    /// group needs (ADR 0045 §2). One pre-order pass; manufacturings are skipped.
    pub(crate) fn raw_leaves_with_structure(&self) -> (Vec<RawLeaf>, Structure) {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut leaves = Vec::new();
        let mut structure = Structure::default();
        let mut stack: Vec<(TreeID, Option<NodeId>)> = crate::group_tree::children_of(&tree, None)
            .into_iter()
            .rev()
            .map(|id| (id, None))
            .collect();
        while let Some((node, parent)) = stack.pop() {
            let id = NodeId::from_parts(node.peer, node.counter);
            let Ok(meta) = tree.get_meta(node) else {
                continue;
            };
            let registers = crate::node_registers::read_registers(&meta);
            if crate::group_tree::meta_is_group(&meta) {
                let layer = crate::group_tree::is_layer_node(&tree, node);
                structure.container(id, parent, &registers, layer);
                stack.extend(
                    crate::group_tree::children_of(&tree, Some(node))
                        .into_iter()
                        .rev()
                        .map(|child| (child, Some(id))),
                );
                continue;
            }
            let raw = match self.sketch(id) {
                Some(sketch) => RawLeaf::Sketch(Box::new(sketch)),
                None => raw_from_meta(id, &meta),
            };
            leaves.push(raw);
            structure.leaf(parent, &registers);
        }
        (leaves, structure)
    }

    /// The stored link of the clone `id`, or `None` when `id` is not a live clone leaf with a
    /// readable link.
    pub(crate) fn clone_link_of(&self, id: NodeId) -> Option<crate::clone_codec::CloneLink> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = TreeID::new(id.peer, id.counter);
        if !leaf_exists(&tree, tree_id) {
            return None;
        }
        let meta = tree.get_meta(tree_id).ok()?;
        if !is_clone_meta(&meta) {
            return None;
        }
        read_clone_link(&meta)
    }

    /// The stored link, override flags and (with `own_style`) own style of the clone `id`, or
    /// `None` when `id` is not a live clone leaf with a readable link: what `document.json`
    /// lists, whether or not the chain resolves.
    pub(crate) fn clone_stored(
        &self,
        id: NodeId,
    ) -> Option<(
        crate::clone_codec::CloneLink,
        crate::clone_codec::CloneOverrides,
        Option<crate::style_model::Style>,
    )> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let meta = tree.get_meta(TreeID::new(id.peer, id.counter)).ok()?;
        let link = self.clone_link_of(id)?;
        let overrides = crate::clone_codec::read_overrides(&meta);
        let style = overrides
            .own_style
            .then(|| crate::style_codec::read_style(&meta));
        Some((link, overrides, style))
    }
}

/// One leaf as stored, or `None` when it has no meta map.
pub(crate) fn raw_leaf(tree: &LoroTree, leaf: TreeID) -> Option<RawLeaf> {
    let meta = tree.get_meta(leaf).ok()?;
    let id = NodeId::from_parts(leaf.peer, leaf.counter);
    Some(raw_from_meta(id, &meta))
}

#[cfg(test)]
thread_local! {
    /// How many path outlines this thread has read through [`raw_leaf`] (review F2).
    static PATH_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The path outlines read so far on this thread (test builds only).
#[cfg(test)]
pub(crate) fn path_reads() -> usize {
    PATH_READS.with(std::cell::Cell::get)
}

fn raw_from_meta(id: NodeId, meta: &LoroMap) -> RawLeaf {
    if is_clone_meta(meta) {
        return read_clone_link(meta).map_or(RawLeaf::BrokenClone(id), |link| {
            let overrides = crate::clone_codec::read_overrides(meta);
            RawLeaf::Clone(
                id,
                link,
                RawOverrides {
                    own_placement: overrides.own_placement,
                    own_style: overrides
                        .own_style
                        .then(|| crate::style_codec::read_style(meta)),
                },
            )
        });
    }
    if crate::text_codec::is_text_meta(meta) && crate::text_clone_codec::has_source_key(meta) {
        return crate::text_clone_codec::read_parts(id, meta)
            .map_or(RawLeaf::Unreadable(id), |parts| {
                RawLeaf::TextClone(Box::new(parts))
            });
    }
    if crate::text_codec::is_text_meta(meta) {
        return crate::text_codec::read_text_snapshot(id, meta)
            .map_or(RawLeaf::Unreadable(id), |text| {
                RawLeaf::Text(Box::new(text))
            });
    }
    if crate::image_codec::is_image_meta(meta) {
        return crate::image_codec::read_image_snapshot(id, meta)
            .map_or(RawLeaf::Unreadable(id), RawLeaf::Image);
    }
    if crate::dimension_codec::is_dimension_meta(meta) {
        return crate::dimension_codec::read_dimension_snapshot(id, meta)
            .map_or(RawLeaf::Unreadable(id), |dimension| {
                RawLeaf::Dimension(Box::new(dimension))
            });
    }
    if read_shape_tag(meta).as_deref() == Some("sketch") {
        return crate::sketch_read::read(id, meta).map_or(RawLeaf::Unreadable(id), |sketch| {
            RawLeaf::Sketch(Box::new(sketch))
        });
    }
    if let Some(tag) = read_shape_tag(meta) {
        return RawLeaf::Primitive(read_primitive_snapshot(id, meta, &tag));
    }
    #[cfg(test)]
    PATH_READS.with(|count| count.set(count.get() + 1));
    read_path_snapshot(id, meta).map_or(RawLeaf::Unreadable(id), RawLeaf::Path)
}

/// The clone `id` resolved by collecting the leaves of its own chain and handing them to the one
/// resolver, or `None` when it is an orphan. O(depth). The caller has checked that `id` is a live
/// clone leaf.
pub(crate) fn read_one_clone(tree: &LoroTree, id: NodeId) -> Option<CloneSnapshot> {
    read_clone_with(tree, id, |leaf| raw_leaf(tree, leaf))
}

impl Document {
    /// The clone `id` resolved, through its own chain or, for a picture root, the whole read.
    pub(crate) fn read_clone(&self, id: NodeId) -> Option<CloneSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if chain_ends_at_picture(&tree, id) {
            return match self.read_objects().get(id)? {
                ObjectSnapshot::Clone(clone) => Some(clone.clone()),
                _ => None,
            };
        }
        read_one_clone(&tree, id)
    }

    pub(crate) fn read_cached_clone(&self, id: NodeId) -> Option<CloneSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if chain_ends_at_picture(&tree, id) {
            return self.read_clone(id);
        }
        read_clone_with(&tree, id, |leaf| {
            self.sketch(NodeId::from_parts(leaf.peer, leaf.counter))
                .map(|sketch| RawLeaf::Sketch(Box::new(sketch)))
                .or_else(|| raw_leaf(&tree, leaf))
        })
    }
}

fn read_clone_with(
    tree: &LoroTree,
    id: NodeId,
    read: impl Fn(TreeID) -> Option<RawLeaf>,
) -> Option<CloneSnapshot> {
    let mut seen = HashSet::new();
    let mut chain = Vec::new();
    let mut current = id;
    while seen.insert(current) {
        let tree_id = TreeID::new(current.peer, current.counter);
        if !leaf_exists(tree, tree_id) {
            break;
        }
        let Some(leaf) = read(tree_id) else {
            break;
        };
        let next = match &leaf {
            RawLeaf::Clone(_, link, _) => Some(link.origin),
            _ => None,
        };
        chain.push(leaf);
        match next {
            Some(origin) => current = origin,
            None => break,
        }
    }
    match resolve(chain).leaves.into_iter().next()?? {
        ObjectSnapshot::Clone(clone) => Some(clone),
        _ => None,
    }
}

/// Whether the chain of the clone `id` ends at something other than a path, sketch or primitive
/// (a group, text or image, ADR 0045 §1), read from the links alone: such a clone is read through
/// the whole document.
fn chain_ends_at_picture(tree: &LoroTree, id: NodeId) -> bool {
    let mut seen = HashSet::new();
    let mut current = id;
    while seen.insert(current) {
        let tree_id = TreeID::new(current.peer, current.counter);
        if !crate::path_codec::node_exists(tree, tree_id) {
            return false;
        }
        let Ok(meta) = tree.get_meta(tree_id) else {
            return false;
        };
        if crate::group_tree::meta_is_group(&meta) {
            return true;
        }
        if is_clone_meta(&meta) {
            match read_clone_link(&meta) {
                Some(link) => current = link.origin,
                None => return false,
            }
            continue;
        }
        return crate::text_codec::is_text_meta(&meta) || crate::image_codec::is_image_meta(&meta);
    }
    false
}
