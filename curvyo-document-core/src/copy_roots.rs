//! The copy of a list of top-level sources with everything under them, shared by Duplicate and
//! the insert of a fragment (ADR 0084 §6): linked clones after every other node, shallow chains
//! first, so each original's copy exists before the copies of its clones (`specs/0049` decision
//! 8, ADR 0015 §3).

use loro::{LoroTree, TreeID, TreeParentId};

use crate::clone_copy::{Copying, Place};
use crate::clone_transform::CloneLinks;
use crate::objects::copy_subtree;
use crate::path_model::{AnchorId, NodeId};
use crate::units::Vec2;

/// One top-level source of a copy.
pub(crate) struct RootCopy<'a> {
    /// The source node in the source tree.
    pub(crate) source: TreeID,
    /// The copy's anchor ids, one per anchor under the source in paint order.
    pub(crate) anchor_ids: &'a [AnchorId],
    /// Where the copy goes.
    pub(crate) place: RootPlace,
}

/// Where the copy of one top-level source goes.
#[derive(Debug, Clone, Copy)]
pub(crate) enum RootPlace {
    /// Directly after its source (Duplicate, the same tree).
    AfterSource,
    /// The last child of this parent of the target tree.
    End(TreeParentId),
}

/// What a copy made: the copy of each root (in the order of the roots) and every copied node with
/// its source.
pub(crate) struct Copied {
    /// The copy of each root, `None` for none.
    pub(crate) roots: Vec<Option<TreeID>>,
    /// `(source, copy)` of every copied node.
    pub(crate) made: Vec<(TreeID, TreeID)>,
}

/// Copies `roots` from the tree `from` into the tree `into`, uncommitted. `links` are the clone
/// links of the source document; `version` the source version a copy inside one document
/// records for the object history (`None` for a copy from another document).
///
/// # Panics
/// Does not panic in practice: every source was resolved by the caller and every move acts on
/// nodes this call just created.
pub(crate) fn copy_roots(
    (from, into): (&LoroTree, &LoroTree),
    roots: &[RootCopy<'_>],
    (offset, version): (Vec2, Option<&loro::Frontiers>),
    links: &CloneLinks,
) -> Copied {
    // `mov_after` needs the fractional-index feature on for this tree.
    into.enable_fractional_index(0);
    let mut copying = Copying::default();
    let mut created: Vec<Option<TreeID>> = vec![None; roots.len()];
    let parent_of = |root: &RootCopy<'_>| match root.place {
        RootPlace::AfterSource => TreeParentId::Root,
        RootPlace::End(parent) => parent,
    };
    for (index, root) in roots.iter().enumerate() {
        if links.contains_key(&NodeId::from_parts(root.source.peer, root.source.counter)) {
            copying
                .waiting
                .push((root.source, Place::After(root.source)));
            continue;
        }
        let mut anchor_ids = root.anchor_ids.iter();
        let copy = copy_subtree(
            (from, into),
            root.source,
            parent_of(root),
            &mut anchor_ids,
            (offset, version),
            (links, &mut copying),
        );
        if matches!(root.place, RootPlace::AfterSource) {
            // invariant: the source was resolved by the caller and the copy was just created.
            #[allow(clippy::unwrap_used)]
            into.mov_after(copy, root.source).unwrap();
        }
        created[index] = Some(copy);
    }
    let mut waiting = std::mem::take(&mut copying.waiting);
    crate::clone_copy::creation_order(links, &mut waiting);
    let position: std::collections::HashMap<TreeID, usize> = roots
        .iter()
        .enumerate()
        .map(|(index, root)| (root.source, index))
        .collect();
    let mut nested = Vec::new();
    for (source, place) in waiting {
        let index = position.get(&source).copied();
        let parent = index.map_or(TreeParentId::Root, |i| parent_of(&roots[i]));
        let copy = copy_subtree(
            (from, into),
            source,
            parent,
            &mut [].iter(),
            (offset, version),
            (links, &mut copying),
        );
        match place {
            Place::After(after) => {
                if index.is_some_and(|i| matches!(roots[i].place, RootPlace::AfterSource)) {
                    // invariant: as above.
                    #[allow(clippy::unwrap_used)]
                    into.mov_after(copy, after).unwrap();
                }
                if let Some(index) = index {
                    created[index] = Some(copy);
                }
            }
            Place::Into(group, child) => nested.push((group, child, copy)),
        }
    }
    // Each group's waiting children go to their own index, lowest first, so every index is
    // counted among the children already placed.
    nested.sort_unstable_by_key(|&(group, child, _)| (group.peer, group.counter, child));
    for (group, child, node) in nested {
        // invariant: the group copy and the node were created by this call.
        #[allow(clippy::unwrap_used)]
        into.mov_to(node, TreeParentId::Node(group), child).unwrap();
    }
    Copied {
        roots: created,
        made: copying.made,
    }
}
