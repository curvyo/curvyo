//! Each text clone filled from its source text (ADR 0022 §4): in the one read pass of every object,
//! and for a read of one node. A clone whose source is missing, not a text or itself a clone is an
//! orphan: not read.

use std::collections::HashMap;

use loro::{LoroMap, LoroTree, TreeID};

use crate::path_model::NodeId;
use crate::primitive_model::ObjectSnapshot;
use crate::text_clone_codec::TextCloneParts;
use crate::text_model::TextSnapshot;

/// What [`fill_clones`] found.
#[derive(Debug, Default)]
pub(crate) struct FilledClones {
    /// `(source, clone)` of every filled clone, in the order of the pending clones.
    pub(crate) links: Vec<(NodeId, NodeId)>,
    /// The clones without a readable source, in the order of the pending clones.
    pub(crate) orphans: Vec<NodeId>,
}

/// Puts each pending clone `(leaf index, parts)` into `leaves` (stacking order, `None` at the
/// pending indices) filled from its source among `leaves`; a clone without a source text stays
/// `None` and is an orphan.
pub(crate) fn fill_clones(
    leaves: &mut [Option<ObjectSnapshot>],
    pending: Vec<(usize, TextCloneParts)>,
) -> FilledClones {
    let mut filled = FilledClones::default();
    if pending.is_empty() {
        return filled;
    }
    let sources: HashMap<NodeId, usize> = leaves
        .iter()
        .enumerate()
        .filter_map(|(i, leaf)| match leaf {
            Some(ObjectSnapshot::Text(text)) if text.source.is_none() => Some((text.id, i)),
            _ => None,
        })
        .collect();
    for (at, parts) in pending {
        let shown = sources.get(&parts.source).and_then(|&i| match &leaves[i] {
            Some(ObjectSnapshot::Text(source)) => Some(parts.fill(source)),
            _ => None,
        });
        match shown {
            Some(text) => {
                filled.links.push((parts.source, parts.id));
                leaves[at] = Some(ObjectSnapshot::Text(Box::new(text)));
            }
            None => filled.orphans.push(parts.id),
        }
    }
    filled
}

/// The text node `id` (`meta`, a text) as shown: a straight text or source as stored, a clone
/// filled from its source; `None` for an unreadable node or an orphan clone. The carrier is not
/// resolved.
pub(crate) fn read_text_node(tree: &LoroTree, id: NodeId, meta: &LoroMap) -> Option<TextSnapshot> {
    if !crate::text_clone_codec::has_source_key(meta) {
        return crate::text_codec::read_text_snapshot(id, meta);
    }
    let parts = crate::text_clone_codec::read_parts(id, meta)?;
    let source = source_meta(tree, parts.source)?;
    let source = crate::text_codec::read_text_snapshot(parts.source, &source)?;
    Some(parts.fill(&source))
}

/// The meta map of the live source text `source`, or `None` when it is missing, not a text or a
/// clone itself (no chains).
pub(crate) fn source_meta(tree: &LoroTree, source: NodeId) -> Option<LoroMap> {
    let tree_id = TreeID::new(source.peer, source.counter);
    if !crate::path_codec::leaf_exists(tree, tree_id) {
        return None;
    }
    let meta = tree.get_meta(tree_id).ok()?;
    (crate::text_codec::is_text_meta(&meta) && !crate::text_clone_codec::has_source_key(&meta))
        .then_some(meta)
}
