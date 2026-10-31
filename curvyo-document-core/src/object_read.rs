//! Reading objects out of the Loro tree with their linked clones resolved: the whole document in
//! one pass ([`Document::read_objects`]) or one clone through its own chain.

use std::collections::HashSet;

use loro::{LoroMap, LoroTree, TreeID};

use crate::clone_codec::{is_clone_meta, read_clone_link};
use crate::clone_model::CloneSnapshot;
use crate::clone_read::{ObjectRead, RawLeaf, RawOverrides, resolve};
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
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let leaves = crate::group_tree::leaves_under(&tree, None)
            .into_iter()
            .filter_map(|leaf| {
                let id = NodeId::from_parts(leaf.peer, leaf.counter);
                if let Some(sketch) = self.sketch(id) {
                    Some(RawLeaf::Sketch(Box::new(sketch)))
                } else {
                    raw_leaf(&tree, leaf)
                }
            })
            .collect();
        resolve(leaves).into_read()
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
    RawLeaf::Path(read_path_snapshot(id, meta))
}

/// The clone `id` resolved by collecting the leaves of its own chain and handing them to the one
/// resolver, or `None` when it is an orphan. O(depth). The caller has checked that `id` is a live
/// clone leaf.
pub(crate) fn read_one_clone(tree: &LoroTree, id: NodeId) -> Option<CloneSnapshot> {
    read_clone_with(tree, id, |leaf| raw_leaf(tree, leaf))
}

impl Document {
    pub(crate) fn read_cached_clone(&self, id: NodeId) -> Option<CloneSnapshot> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
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
