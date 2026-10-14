//! Copies of sets that hold linked clones (`specs/0049-linked-clone` criteria 19, 47 and 48,
//! `adrs.md` decision 8): which copies are made after which, and the link each copied clone gets.

use std::collections::HashMap;

use loro::{LoroTree, TreeID};

use crate::affine::Affine;
use crate::clone_codec::{CloneLink, read_clone_link, write_clone_link};
use crate::clone_transform::CloneLinks;
use crate::path_model::NodeId;
use crate::units::Vec2;

/// Where one copied clone goes once it exists.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Place {
    /// Directly after this top-level source.
    After(TreeID),
    /// Into this new group at this child index.
    Into(TreeID, usize),
}

/// What a copy command has made so far: every copied node with its source, and the clones whose
/// copies wait until every non-clone copy exists (so each original's copy is created before the
/// copies of the clones that follow it, ADR 0015 §3).
#[derive(Debug, Default)]
pub(crate) struct Copying {
    /// `(source, copy)` of every copied node.
    pub(crate) made: Vec<(TreeID, TreeID)>,
    /// The clones still to copy, with where their copies go.
    pub(crate) waiting: Vec<(TreeID, Place)>,
}

/// The number of clones from `id` down to its root along `links` (0 for a non-clone), bounded by
/// the number of links so a ring ends.
pub(crate) fn depth_of(links: &CloneLinks, id: NodeId) -> usize {
    let mut depth = 0;
    let mut current = id;
    while let Some(&(origin, _)) = links.get(&current) {
        depth += 1;
        if depth > links.len() {
            break;
        }
        current = origin;
    }
    depth
}

/// The waiting clones in the order their copies are created: shallower chains first.
pub(crate) fn creation_order(links: &CloneLinks, waiting: &mut [(TreeID, Place)]) {
    waiting.sort_by_key(|(source, _)| {
        depth_of(links, NodeId::from_parts(source.peer, source.counter))
    });
}

/// Gives every copied clone its link: a clone whose original was copied too follows that copy
/// with `G·M·G⁻¹`, any other keeps its original with `G·M`, where G is the move by `offset`
/// (criteria 19, 47, 48). The copies are uncommitted.
pub(crate) fn relink_copied_clones(tree: &LoroTree, made: &[(TreeID, TreeID)], offset: Vec2) {
    let copy_of: HashMap<NodeId, NodeId> = made
        .iter()
        .map(|(source, copy)| {
            (
                NodeId::from_parts(source.peer, source.counter),
                NodeId::from_parts(copy.peer, copy.counter),
            )
        })
        .collect();
    let g = Affine::translation(offset);
    let g_inverse = Affine::translation(Vec2::new(-offset.x, -offset.y));
    for &(_, copy) in made {
        let Ok(meta) = tree.get_meta(copy) else {
            continue;
        };
        if !crate::clone_codec::is_clone_meta(&meta) {
            continue;
        }
        let Some(link) = read_clone_link(&meta) else {
            continue;
        };
        let relinked = match copy_of.get(&link.origin) {
            Some(&origin) => CloneLink {
                origin,
                matrix: g.compose(link.matrix).compose(g_inverse),
            },
            None => CloneLink {
                origin: link.origin,
                matrix: g.compose(link.matrix),
            },
        };
        if relinked != link {
            write_clone_link(&meta, relinked);
        }
    }
}
