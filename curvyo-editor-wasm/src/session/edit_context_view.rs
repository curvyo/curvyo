//! What a move-only selection shows outside the canvas (`specs/0106-move-versus-edit` criterion
//! 11, UX notes §2, §3 and §5): the Style line, the status bar text, the announcer's suffix, the
//! bold home page labels and the Layers rows' "On Laser 1" line.

use std::collections::HashSet;

use curvyo_document_core::NodeId;
use curvyo_ui_core::{
    ActivePage, Home, Scope, home_row_line, in_context, object_bounds, object_kind_name,
    objects_count_text, out_of_context_line, out_of_context_status, out_of_context_suffix,
};

use super::{Session, Workbench};

impl Session {
    /// The Style tab's muted line for a move-only selection (criterion 11, UX notes §3), empty
    /// while the selection may be edited: the host then shows the tab as built.
    #[must_use]
    pub fn out_of_context_line(&self) -> String {
        let homes = self.selection_homes();
        if homes.scope == Scope::MoveOnly {
            out_of_context_line(&homes.names)
        } else {
            String::new()
        }
    }

    /// Whether the selection is move-only: the Select bar and the rail hide their edit buttons
    /// (criterion 11).
    #[must_use]
    pub fn selection_move_only(&self) -> bool {
        self.edit_scope() == Scope::MoveOnly
    }

    /// The status bar's text for a move-only selection: "Rectangle · Drawing 2" (UX notes §2);
    /// empty otherwise.
    #[must_use]
    pub fn out_of_context_status(&self) -> String {
        let homes = self.selection_homes();
        if homes.scope == Scope::Edit {
            return String::new();
        }
        out_of_context_status(&self.selection_subject(), &homes.names)
    }

    /// "Rectangle" for one selected object, "3 objects" for several.
    pub(super) fn selection_subject(&self) -> String {
        let ids = self.selection.ids();
        match ids {
            [only] => self
                .objects()
                .iter()
                .find(|o| o.id() == *only)
                .map_or_else(|| objects_count_text(1), |o| object_kind_name(o).to_owned()),
            _ => objects_count_text(ids.len()),
        }
    }

    /// What the selection announcer adds for a move-only selection: ", on Drawing 2".
    pub(super) fn out_of_context_suffix(&self) -> String {
        let homes = self.selection_homes();
        if homes.scope == Scope::Edit {
            return String::new();
        }
        out_of_context_suffix(&homes.names)
    }

    /// The pages a move-only selection's out-of-context objects belong to: their canvas labels
    /// are semibold (UX notes §2). A pasteboard object has no label.
    pub(super) fn home_page_ids(&self) -> HashSet<String> {
        let homes = self.selection_homes();
        homes
            .outside
            .iter()
            .flat_map(|home| match home {
                Home::Pasteboard => Vec::new(),
                Home::Drawing(pages) => pages.clone(),
                Home::Laser(page) | Home::Sketch(page) => vec![page.clone()],
            })
            .collect()
    }

    /// For each of `ids` (Layers rows), "On Laser 1" when its object lies out of context of the
    /// active page, else empty (UX notes §5). A group reads its first leaf.
    pub(super) fn home_row_lines(&self, ids: &[NodeId]) -> Vec<String> {
        let (pages, membership) = self.context_read();
        let active = self.active_page();
        let kind = match self.workbench() {
            Workbench::Vector => curvyo_ui_core::ContextKind::Drawing,
            Workbench::Sketch => curvyo_ui_core::ContextKind::Sketch,
            Workbench::Manufacturing(_) => curvyo_ui_core::ContextKind::Laser,
        };
        let active = ActivePage {
            id: &active.id,
            kind,
        };
        let read = self.document_read();
        ids.iter()
            .map(|&id| {
                let Some(leaf) = self.document.leaves_of(id).first().copied() else {
                    return String::new();
                };
                let Some(object) = read.leaves.iter().find(|o| o.id() == leaf) else {
                    return String::new();
                };
                let home = pages.home_of(&membership, leaf, object_bounds(object));
                let page = match &home {
                    _ if in_context(active, &home) => return String::new(),
                    Home::Pasteboard => return String::new(),
                    Home::Drawing(pages) => pages.first().cloned().unwrap_or_default(),
                    Home::Laser(page) | Home::Sketch(page) => page.clone(),
                };
                home_row_line(pages.name_of(&page).unwrap_or(&page))
            })
            .collect()
    }
}
