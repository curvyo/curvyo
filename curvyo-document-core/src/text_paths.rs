//! Object to path for text objects (`specs/0074-text-objects` criterion 22): each text becomes a
//! group, named after the text, of one compound path per drawn glyph, at the text's place.

use std::collections::HashSet;

use loro::TreeParentId;

use crate::document::{Document, OBJECTS_TREE};
use crate::objects::ObjectEditError;
use crate::path_codec::leaf_exists;
use crate::path_model::{AnchorId, NewAnchor, NodeId};

/// One text and the outlines of its glyphs in document mm: per glyph its outlines
/// `(anchors, closed)`, the first stored in the ordinary keys, the rest as extra subpaths.
#[derive(Debug, Clone, PartialEq)]
pub struct TextConversion {
    /// The text node.
    pub id: NodeId,
    /// The group's name: the text's own name, or its first 24 characters.
    pub name: String,
    /// One entry per drawn glyph; a glyph without outlines is left out by the caller.
    pub glyphs: Vec<Vec<(Vec<NewAnchor>, bool)>>,
}

fn tree_id_of(id: NodeId) -> loro::TreeID {
    loro::TreeID::new(id.peer, id.counter)
}

impl Document {
    /// Replaces each text of `texts` by its group of glyph paths, without committing: the caller's
    /// commit (`convert_to_paths`) takes the primitives of the same selection too. Every text is
    /// resolved and every anchor id checked before the first write. The paths take the text's
    /// style; a guide text gives guide paths. A text with no glyph is removed. Returns the new
    /// groups in the order of `texts` (`None` for a text without glyphs).
    ///
    /// # Errors
    /// [`ObjectEditError::NoSuchObject`] when an id is not a live text;
    /// [`ObjectEditError::AnchorIds`] when anchor ids repeat or belong to an object that stays;
    /// [`ObjectEditError::NoOutlines`] for a glyph with an outline without anchors.
    pub(crate) fn convert_texts_uncommitted(
        &self,
        texts: &[TextConversion],
    ) -> Result<Vec<Option<NodeId>>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let mut doomed = Vec::with_capacity(texts.len());
        for text in texts {
            let tree_id = tree_id_of(text.id);
            let is_text = leaf_exists(&tree, tree_id)
                && tree
                    .get_meta(tree_id)
                    .is_ok_and(|meta| crate::text_codec::is_text_meta(&meta));
            if !is_text {
                return Err(ObjectEditError::NoSuchObject);
            }
            doomed.push(tree_id);
        }
        if texts
            .iter()
            .flat_map(|text| &text.glyphs)
            .any(Vec::is_empty)
        {
            return Err(ObjectEditError::NoOutlines);
        }
        let anchors = texts
            .iter()
            .flat_map(|text| &text.glyphs)
            .flatten()
            .map(|(anchors, _)| anchors);
        let mut seen: HashSet<AnchorId> = HashSet::new();
        for outline in anchors {
            if outline.is_empty() {
                return Err(ObjectEditError::NoOutlines);
            }
            if !outline.iter().all(|anchor| seen.insert(anchor.id)) {
                return Err(ObjectEditError::AnchorIds);
            }
        }
        crate::replace::check_new_anchor_ids(&tree, &doomed, &seen)?;
        tree.enable_fractional_index(0);
        let mut groups = Vec::with_capacity(texts.len());
        for (text, &tree_id) in texts.iter().zip(&doomed) {
            groups.push(write_group(&tree, tree_id, text));
        }
        self.repair_clones_of_removed(&tree, &doomed);
        let parents = crate::group_tree::parents_of(&tree, &doomed);
        for tree_id in doomed {
            // invariant: every id was confirmed live above and nothing deleted it since.
            #[allow(clippy::unwrap_used)]
            tree.delete(tree_id).unwrap();
        }
        crate::groups::remove_emptied_groups(&tree, &parents);
        Ok(groups)
    }

    /// Object to path of a selection (criterion 22): the primitives of `primitives` become paths
    /// in place, the texts of `texts` groups of glyph paths, all in **one commit** labelled
    /// `convert_to_paths`. Nothing is written when either part refuses.
    ///
    /// # Errors
    /// As [`Document::convert_to_paths`] for the primitives (mapped to
    /// [`ObjectEditError::NoSuchObject`]) and `convert_texts_uncommitted` for the texts.
    pub fn convert_to_paths_with_texts(
        &self,
        primitives: &[(NodeId, Vec<NewAnchor>)],
        texts: &[TextConversion],
    ) -> Result<Vec<Option<NodeId>>, ObjectEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let all_texts = texts.iter().all(|text| {
            let tree_id = tree_id_of(text.id);
            leaf_exists(&tree, tree_id)
                && tree
                    .get_meta(tree_id)
                    .is_ok_and(|meta| crate::text_codec::is_text_meta(&meta))
        });
        if !all_texts {
            return Err(ObjectEditError::NoSuchObject);
        }
        self.batch("convert_to_paths", |document| {
            document
                .convert_to_paths_uncommitted(primitives)
                .map_err(|_| ObjectEditError::NoSuchObject)?;
            document.convert_texts_uncommitted(texts)
        })
    }
}

/// Writes the group of `text` right above the text node; `None` when it has no glyphs.
fn write_group(
    tree: &loro::LoroTree,
    text_node: loro::TreeID,
    text: &TextConversion,
) -> Option<NodeId> {
    if text.glyphs.is_empty() {
        return None;
    }
    // invariant: the text node was resolved live by the caller and the trees below are written
    // to nodes this function just created.
    #[allow(clippy::unwrap_used)]
    let text_meta = tree.get_meta(text_node).unwrap();
    let style = crate::style_codec::read_style(&text_meta);
    let guide = crate::guides::all_guides(tree, &[text_node]);
    #[allow(clippy::unwrap_used)]
    let group = tree.create(TreeParentId::Root).unwrap();
    #[allow(clippy::unwrap_used)]
    tree.mov_after(group, text_node).unwrap();
    #[allow(clippy::unwrap_used)]
    let meta = tree.get_meta(group).unwrap();
    #[allow(clippy::unwrap_used)]
    meta.insert(crate::group_tree::KEY_GROUP, true).unwrap();
    #[allow(clippy::unwrap_used)]
    meta.insert(crate::node_registers::KEY_NAME, text.name.as_str())
        .unwrap();
    let mut previous: Option<loro::TreeID> = None;
    for glyph in &text.glyphs {
        // The first path is the group's first child, every next one goes above the one before.
        let path = match previous {
            None => {
                #[allow(clippy::unwrap_used)]
                let first = tree.create(TreeParentId::Node(group)).unwrap();
                crate::replace::write_path_node(tree, first, &style, glyph);
                first
            }
            Some(after) => crate::replace::create_path_after(tree, after, &style, glyph),
        };
        if guide {
            #[allow(clippy::unwrap_used)]
            crate::guides::write_guide(&tree.get_meta(path).unwrap(), true);
        }
        previous = Some(path);
    }
    Some(NodeId::from_parts(group.peer, group.counter))
}
