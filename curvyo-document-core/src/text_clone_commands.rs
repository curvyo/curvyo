//! The document commands of text clones (`specs/0080-text-on-path` criteria 37 to 41, ADR 0022
//! §4): put one text on several carriers, Unlink clone, the Style row, the unlink of the clones of
//! a removed source, and the node a content edit or a size change of a clone writes to (its
//! source).

use std::collections::{HashMap, HashSet};

use loro::{LoroMap, LoroTree, TreeID};

use crate::document::{Document, OBJECTS_TREE};
use crate::path_model::NodeId;
use crate::text_clone_codec::{has_source_key, read_source, unlink_meta, write_source};
use crate::text_model::TextSnapshot;
use crate::text_path_model::{CarrierRef, OnPathValues, PUT_ON_PATH_LABEL};
use crate::texts::TextEditError;

/// The commit label of "Put text on paths" (criterion 37).
pub const PUT_ON_PATHS_LABEL: &str = "put_text_on_paths";

fn tree_id(id: NodeId) -> TreeID {
    TreeID::new(id.peer, id.counter)
}

/// Whether live node `id` is an object a text can be put on: a leaf that is not a text.
pub(crate) fn is_carrier(tree: &LoroTree, id: NodeId) -> bool {
    crate::path_codec::leaf_exists(tree, tree_id(id))
        && tree
            .get_meta(tree_id(id))
            .is_ok_and(|meta| !crate::text_codec::is_text_meta(&meta))
}

fn node(id: TreeID) -> NodeId {
    NodeId::from_parts(id.peer, id.counter)
}

impl Document {
    /// Puts text `text` on the first of `carriers` and a new text clone of its source on each
    /// further one, with the values paired with each (criterion 37), in **one commit** labelled
    /// [`PUT_ON_PATHS_LABEL`] ("Put text on path" for one carrier). The clones follow the text's
    /// source (the text itself, or the source of a clone: no chains), take its straight
    /// placement and sit directly above it in stacking order. With `mark_guide` every carrier
    /// becomes a guide as in [`Document::put_text_on_path`]. Returns the new clones in carrier
    /// order.
    ///
    /// Only paths, primitives and clones are carriers: the text itself, another text or a group
    /// among `carriers` is skipped; with no carrier left nothing is written.
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] when `text` is not a live text or a carrier is not a live
    /// node; nothing is written.
    pub fn put_text_on_paths(
        &self,
        text: NodeId,
        carriers: &[(CarrierRef, OnPathValues)],
        mark_guide: bool,
    ) -> Result<Vec<NodeId>, TextEditError> {
        let meta = self.text_meta(text)?;
        let tree = self.loro().get_tree(OBJECTS_TREE);
        if carriers
            .iter()
            .any(|(carrier, _)| !crate::path_codec::node_exists(&tree, tree_id(carrier.node)))
        {
            return Err(TextEditError::NoSuchText);
        }
        let carriers: Vec<(CarrierRef, OnPathValues)> = carriers
            .iter()
            .copied()
            .filter(|(carrier, _)| carrier.node != text && is_carrier(&tree, carrier.node))
            .collect();
        let Some((&(first, first_values), rest)) = carriers.split_first() else {
            return Ok(Vec::new());
        };
        let source = read_source(&meta).unwrap_or(text);
        let placement =
            crate::text_codec::read_placement(&meta).ok_or(TextEditError::NoSuchText)?;
        crate::text_path_codec::write_carrier(&meta, first);
        crate::text_path_codec::write_values(&meta, &first_values);
        tree.enable_fractional_index(0);
        let mut previous = tree_id(text);
        let mut created = Vec::with_capacity(rest.len());
        for &(carrier, values) in rest {
            let clone = create_text_clone(&tree, previous, source, placement);
            if let Ok(clone_meta) = tree.get_meta(clone) {
                crate::text_path_codec::write_carrier(&clone_meta, carrier);
                crate::text_path_codec::write_values(&clone_meta, &values);
            }
            created.push(node(clone));
            previous = clone;
        }
        if mark_guide {
            for (carrier, _) in &carriers {
                crate::text_path_commands::mark_carrier_guide(&tree, tree_id(carrier.node));
            }
        }
        self.commit_with_label(if rest.is_empty() {
            PUT_ON_PATH_LABEL
        } else {
            PUT_ON_PATHS_LABEL
        });
        Ok(created)
    }

    /// The text clones of `ids` with the text each shows, for Unlink clone (criterion 41,
    /// [`Document::unlink_clones_and_texts`]); duplicates once. Writes nothing.
    pub(crate) fn plan_text_unlinks(
        &self,
        ids: &[NodeId],
    ) -> Result<Vec<(LoroMap, TextSnapshot)>, TextEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut planned = Vec::with_capacity(ids.len());
        let mut seen = HashSet::new();
        for &id in ids {
            if !seen.insert(id) {
                continue;
            }
            let meta = self.text_meta(id)?;
            let shown = crate::text_clone_read::read_text_node(&tree, id, &meta)
                .filter(|text| text.source.is_some())
                .ok_or(TextEditError::NoSuchText)?;
            planned.push((meta, shown));
        }
        Ok(planned)
    }

    /// The text clones of `ids` whose style state is not `own` yet, with the text each shows,
    /// for the Style row (criterion 40, [`Document::set_own_style_and_texts`]). Writes nothing.
    pub(crate) fn plan_text_clone_style(
        &self,
        ids: &[NodeId],
        own: bool,
    ) -> Result<Vec<(LoroMap, TextSnapshot)>, TextEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut planned: Vec<(LoroMap, TextSnapshot)> = Vec::new();
        for &id in ids {
            let meta = self.text_meta(id)?;
            let shown = crate::text_clone_read::read_text_node(&tree, id, &meta)
                .ok_or(TextEditError::NoSuchText)?;
            let Some(source) = shown.source else {
                return Err(TextEditError::NoSuchText);
            };
            if source.own_style != own && !planned.iter().any(|(_, t)| t.id == id) {
                planned.push((meta, shown));
            }
        }
        Ok(planned)
    }

    /// How many text clones became texts because their source was removed since the last call
    /// ("3 text clones became texts.", criterion 41); resets the count.
    #[must_use]
    pub fn take_text_clones_unlinked(&self) -> usize {
        self.text_clones_unlinked.replace(0)
    }

    /// Writes, without committing, every live text clone outside `removed` whose source is in it
    /// as an independent text (criterion 41). Call it before the nodes are deleted, in the commit
    /// that deletes them.
    pub(crate) fn unlink_text_clones_of_removed(&self, tree: &LoroTree, removed: &[TreeID]) {
        let mut gone: HashSet<NodeId> = HashSet::new();
        for &root in removed {
            gone.insert(node(root));
            for leaf in crate::group_tree::leaves_under(tree, Some(root)) {
                gone.insert(node(leaf));
            }
        }
        let mut planned = Vec::new();
        for leaf in crate::group_tree::leaves_under(tree, None) {
            let Ok(meta) = tree.get_meta(leaf) else {
                continue;
            };
            let id = node(leaf);
            if gone.contains(&id) || !read_source(&meta).is_some_and(|s| gone.contains(&s)) {
                continue;
            }
            if let Some(shown) = crate::text_clone_read::read_text_node(tree, id, &meta) {
                planned.push((meta, shown));
            }
        }
        for (meta, shown) in &planned {
            unlink_meta(meta, shown);
        }
        self.text_clones_unlinked
            .set(self.text_clones_unlinked.get() + planned.len());
    }

    /// The meta map a content edit of text `id` writes (criterion 38): its own for a source or an
    /// ordinary text, its source's for a text clone.
    pub(crate) fn text_content_meta(&self, id: NodeId) -> Result<LoroMap, TextEditError> {
        let meta = self.text_meta(id)?;
        match read_source(&meta) {
            None if !has_source_key(&meta) => Ok(meta),
            None => Err(TextEditError::NoSuchText),
            Some(source) => {
                crate::text_clone_read::source_meta(&self.loro().get_tree(OBJECTS_TREE), source)
                    .ok_or(TextEditError::NoSuchText)
            }
        }
    }
}

/// Creates a text clone of `source` with the straight `placement` directly above `after`.
/// Uncommitted.
fn create_text_clone(
    tree: &LoroTree,
    after: TreeID,
    source: NodeId,
    placement: crate::affine::Affine,
) -> TreeID {
    // invariant: creating a node under the root, reading its meta map and moving it next to a
    // node the caller checked live cannot fail.
    #[allow(clippy::unwrap_used)]
    let clone = tree.create(loro::TreeParentId::Root).unwrap();
    #[allow(clippy::unwrap_used)]
    tree.mov_after(clone, after).unwrap();
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(clone).unwrap();
    crate::shape_codec::write_shape_tag(&meta, crate::text_codec::SHAPE_TEXT);
    write_source(&meta, source);
    crate::text_codec::write_placement(&meta, placement);
    clone
}

/// Writes the Style row's state `own` on each planned text clone: a copy of the style it shows,
/// or none, so it shows its source's again. Opacity stays its own register. Uncommitted.
pub(crate) fn write_text_clone_style(planned: &[(LoroMap, TextSnapshot)], own: bool) {
    for (meta, shown) in planned {
        clear_style_keys(meta);
        if own {
            crate::style_codec::write_style(meta, &shown.style);
        }
        crate::clone_codec::write_flag(meta, crate::clone_codec::KEY_OWN_STYLE, own);
    }
}

/// Gives the text clone `meta` (showing `shown`) the colours of `style` for Spread colours: the
/// changed keys when it has its own style, else an own style equal to `style`. Returns whether
/// something was written. Uncommitted.
pub(crate) fn write_text_clone_colours(
    meta: &LoroMap,
    shown: &TextSnapshot,
    style: &crate::style_model::Style,
) -> bool {
    if shown.source.is_some_and(|source| source.own_style) {
        return crate::style_codec::write_changes(meta, &shown.style, style);
    }
    clear_style_keys(meta);
    crate::style_codec::write_style(meta, style);
    crate::clone_codec::write_flag(meta, crate::clone_codec::KEY_OWN_STYLE, true);
    true
}

/// Removes every style key of a text clone (its opacity register stays). Uncommitted.
fn clear_style_keys(meta: &LoroMap) {
    for key in crate::style_codec::ALL_STYLE_KEYS {
        if meta.get(key).is_some() {
            // invariant: deleting a key just confirmed present on an attached map cannot fail.
            #[allow(clippy::unwrap_used)]
            meta.delete(key).unwrap();
        }
    }
}

/// Writes a transformed text (`transform_objects`, `rotate_object`): placement on its own node,
/// size where it differs, on the source for a text clone (criterion 38). Uncommitted.
pub(crate) fn write_transformed(tree: &LoroTree, meta: &LoroMap, text: &TextSnapshot) {
    let Some(source) = read_source(meta) else {
        crate::text_codec::write_size_and_placement_if_changed(meta, text);
        return;
    };
    if crate::text_codec::read_placement(meta) != Some(text.placement) {
        crate::text_codec::write_placement(meta, text.placement);
    }
    if let Some(source) = crate::text_clone_read::source_meta(tree, source)
        && crate::text_codec::read_size(&source) != Some(text.size)
    {
        crate::text_codec::write_size(&source, text.size);
    }
}

/// Points every copied text clone whose source was copied too at that copy (Duplicate of a source
/// with its clones); a clone copied alone keeps its source. Uncommitted.
pub(crate) fn relink_copied_text_clones(tree: &LoroTree, made: &[(TreeID, TreeID)]) {
    let copy_of: HashMap<NodeId, NodeId> = made
        .iter()
        .map(|(source, copy)| (node(*source), node(*copy)))
        .collect();
    for &(_, copy) in made {
        if let Ok(meta) = tree.get_meta(copy)
            && let Some(source) = read_source(&meta)
            && let Some(&source_copy) = copy_of.get(&source)
        {
            write_source(&meta, source_copy);
        }
    }
}

/// Whether the node `meta` owns the style keys it stores: every node but a text clone without
/// own style (`0057` criterion 18, applied to text clones).
pub(crate) fn owns_style_keys(meta: &LoroMap) -> bool {
    !has_source_key(meta) || crate::clone_codec::read_overrides(meta).own_style
}
