//! What the session does with text clones (`specs/0080-text-on-path` criteria 38, 40 and 41): the
//! edit of one member shown on its source and every clone in the same frame, and the selected
//! clones that Unlink clone, the Style row and Spread colours act on.

use std::sync::Arc;

use curvyo_document_core::{NodeId, ObjectSnapshot, TextSnapshot};

use super::Session;

impl Session {
    /// Shows `edited` (the text in edit mode as drawn) on every other member of its family: the
    /// source and the clones that share its string, face, size and layout (criterion 38). Each
    /// keeps its own placement, carrier and style.
    pub(super) fn show_edit_on_members(
        &self,
        objects: &mut [ObjectSnapshot],
        edited: &TextSnapshot,
    ) {
        let root = edited.source.map_or(edited.id, |source| source.node);
        for object in objects.iter_mut() {
            let ObjectSnapshot::Text(text) = object else {
                continue;
            };
            let member = text.id == root || text.source.is_some_and(|source| source.node == root);
            if !member || text.id == edited.id {
                continue;
            }
            let mut shown = (**text).clone();
            shown.string.clone_from(&edited.string);
            shown.chars.clone_from(&edited.chars);
            shown.font.clone_from(&edited.font);
            shown.size = edited.size;
            shown.layout = edited.layout;
            shown.outline = self.fonts.lay_out(&shown).map(Arc::new);
            **text = shown;
        }
    }

    /// The selection, when it holds clones only (linked or text): what Spread colours acts on.
    pub(super) fn selected_clones_of_both_kinds(&self) -> Vec<NodeId> {
        let read = self.object_read();
        let ids = self.selection.ids();
        let all_clones = ids.iter().all(|id| match read.get(*id) {
            Some(ObjectSnapshot::Clone(_)) => true,
            Some(ObjectSnapshot::Text(text)) => text.source.is_some(),
            _ => false,
        });
        if all_clones { ids.to_vec() } else { Vec::new() }
    }

    /// The selected text clones, when the selection holds clones only (linked or text): what
    /// Unlink clone and the Style row act on besides the linked clones.
    pub(super) fn selected_text_clones(&self) -> Vec<NodeId> {
        let read = self.object_read();
        self.selected_clones_of_both_kinds()
            .into_iter()
            .filter(|id| matches!(read.get(*id), Some(ObjectSnapshot::Text(_))))
            .collect()
    }

    /// The sentence the removal of a source adds to its notice (criterion 41): "3 text clones
    /// became texts.", or `None`.
    pub(super) fn text_clones_unlinked_sentence(&self) -> Option<String> {
        match self.document.take_text_clones_unlinked() {
            0 => None,
            1 => Some("1 text clone became a text.".to_owned()),
            n => Some(format!("{n} text clones became texts.")),
        }
    }
}
