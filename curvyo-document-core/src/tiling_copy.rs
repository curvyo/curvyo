//! Tiling registers of nodes that leave their tiling form (ADR 0028 §6, §7): Release, Ungroup,
//! copy.

use std::collections::{HashMap, HashSet};

use loro::{LoroTree, TreeID};

use crate::group_tree::{children_of, group_parent, meta_is_group, node_id};
use crate::path_model::NodeId;
use crate::tiling_codec::read_sources;

/// Removes the tiling registers that `group` owns (ADR 0028 §7): its own, its instances' cell
/// keys and their members' keys. A tiling nested deeper keeps its recipe. Uncommitted.
pub(crate) fn strip_tiling(tree: &LoroTree, group: TreeID) {
    let Ok(meta) = tree.get_meta(group) else {
        return;
    };
    let is_tiling = crate::tiling_codec::has_recipe(&meta);
    let is_instance = meta.get(crate::tiling_codec::KEY_CELL).is_some();
    crate::tiling_codec::strip(&meta);
    for child in children_of(tree, Some(group)) {
        let Ok(child_meta) = tree.get_meta(child) else {
            continue;
        };
        if is_instance {
            crate::tiling_codec::remove(&child_meta, crate::tiling_codec::KEY_MEMBER);
        }
        if !is_tiling || child_meta.get(crate::tiling_codec::KEY_CELL).is_none() {
            continue;
        }
        crate::tiling_codec::remove(&child_meta, crate::tiling_codec::KEY_CELL);
        if meta_is_group(&child_meta) {
            for member in children_of(tree, Some(child)) {
                if let Ok(member_meta) = tree.get_meta(member) {
                    crate::tiling_codec::remove(&member_meta, crate::tiling_codec::KEY_MEMBER);
                }
            }
        }
    }
}

/// After a copy (ADR 0028 §6): a copied tiling's source list and member keys point at the
/// copies of the sources where they were copied too; a copied instance or member whose tiling
/// group was not copied loses its keys. Uncommitted.
pub(crate) fn relink_copied_tilings(tree: &LoroTree, made: &[(TreeID, TreeID)]) {
    let map: HashMap<NodeId, NodeId> = made
        .iter()
        .map(|&(s, c)| (node_id(s), node_id(c)))
        .collect();
    let copies: HashSet<TreeID> = made.iter().map(|&(_, c)| c).collect();
    let remap = |id: NodeId| map.get(&id).copied().unwrap_or(id);
    for &(_, copy) in made {
        let Ok(meta) = tree.get_meta(copy) else {
            continue;
        };
        if let Some(sources) = read_sources(&meta) {
            let moved: Vec<NodeId> = sources.into_iter().map(remap).collect();
            crate::tiling_codec::write_sources(&meta, &moved);
        }
        let parent_is_copied_tiling = |node: TreeID| {
            group_parent(tree, node).is_some_and(|p| {
                copies.contains(&p)
                    && tree
                        .get_meta(p)
                        .is_ok_and(|m| crate::tiling_codec::has_recipe(&m))
            })
        };
        if meta.get(crate::tiling_codec::KEY_CELL).is_some() && !parent_is_copied_tiling(copy) {
            crate::tiling_codec::remove(&meta, crate::tiling_codec::KEY_CELL);
        }
        if let Some(leaf) = crate::tiling_codec::read_member(&meta) {
            let instance_ok = group_parent(tree, copy).is_some_and(parent_is_copied_tiling);
            if instance_ok {
                crate::tiling_codec::write_member(&meta, remap(leaf));
            } else {
                crate::tiling_codec::remove(&meta, crate::tiling_codec::KEY_MEMBER);
            }
        }
    }
}
