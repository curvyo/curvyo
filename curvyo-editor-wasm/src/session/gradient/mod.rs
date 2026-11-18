//! `Session`'s side of the gradient fill (`specs/0124-gradient-fill` criteria 3, 6 to 8, 10, 18):
//! one state field ([`GradientState`]) and the edited gradients the submodules read: the
//! Style tab's Gradient block (`view`, `commands`), the canvas handles (`canvas`), their entry
//! boxes (`entry`) and the grey of a laser page (`laser`). Every rule lives in `curvyo_ui_core`
//! (`gradient_layout`, `GradientDragStart`, `gradient_panel`) or `curvyo_document_core`.

mod canvas;
mod commands;
mod entry;
mod laser;
#[cfg(test)]
mod tests;
mod view;

use std::cell::RefCell;
use std::collections::HashSet;
use std::sync::Arc;

use curvyo_document_core::{
    DocumentVersion, GradientBox, GradientFill, NodeId, ObjectSnapshot, StyleEdit,
};
use curvyo_ui_core::{GradientDragStart, GradientHandle, takes_gradient};

use super::Session;

/// A handle drag in flight on the shape `id`.
#[derive(Debug, Clone)]
struct HandleDrag {
    id: NodeId,
    start: GradientDragStart,
    /// The shape's box at the press.
    frame: GradientBox,
    /// The pointer, where the readout sits.
    pointer: curvyo_document_core::Point,
    /// The pointer left the 3 px dead zone.
    moved: bool,
}

/// An entry box open at a handle of the shape `id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OpenEntry {
    id: NodeId,
    handle: GradientHandle,
}

/// The gradient state of a session: never saved.
#[derive(Debug, Default)]
pub(super) struct GradientState {
    /// The shape whose stops the active stop indexes; a new one resets it.
    subject: Option<NodeId>,
    /// The active stop of the block (UX notes §4, P2).
    active: usize,
    /// The handle selected on the canvas (Delete removes an inner one).
    selected: Option<GradientHandle>,
    /// The handle under the pointer.
    hovered: Option<GradientHandle>,
    drag: Option<HandleDrag>,
    entry: Option<OpenEntry>,
    /// A release whose drag Escape cancelled: it does nothing.
    swallow_release: bool,
    /// The canvas notice of the gradient features the last SVG Open or Import lost, until the
    /// host takes it (criterion 27).
    import_notice: Option<String>,
    /// The leaves whose home is a laser page, for the document version they were read at.
    laser: RefCell<Option<(DocumentVersion, Arc<HashSet<NodeId>>)>>,
}

impl GradientState {
    /// Closes the entry box and ends the stop selection (a tool change).
    pub(super) fn forget_canvas(&mut self) {
        self.entry = None;
        self.selected = None;
    }
}

/// One edited shape with a painted gradient: the shape as shown, its box and its gradient.
#[derive(Debug, Clone)]
struct Shaded {
    object: ObjectSnapshot,
    frame: GradientBox,
    fill: GradientFill,
}

impl Shaded {
    fn of(object: &ObjectSnapshot) -> Option<Self> {
        Some(Self {
            fill: object.style().fill.painted_gradient()?.clone(),
            frame: object.gradient_box()?,
            object: object.clone(),
        })
    }
}

impl Session {
    /// The edited objects that can take a gradient, as shown (a panel drag previewed).
    fn gradient_objects(&self) -> Vec<ObjectSnapshot> {
        let ids: HashSet<NodeId> = self.style_scope().ids.into_iter().collect();
        let mut shown: Vec<ObjectSnapshot> = self
            .objects()
            .iter()
            .filter(|object| ids.contains(&object.id()) && takes_gradient(object))
            .cloned()
            .collect();
        if self.style.is_active() {
            self.style.apply_to(&mut shown);
        }
        shown
    }

    /// The edited shapes that paint a gradient, as shown.
    fn shaded_objects(&self) -> Vec<Shaded> {
        self.gradient_objects()
            .iter()
            .filter_map(Shaded::of)
            .collect()
    }

    /// The ids a gradient `edit` goes to: the edited objects that can take one.
    fn gradient_ids(&self, edit: &StyleEdit) -> Vec<NodeId> {
        let objects = self.objects();
        self.style_scope()
            .targets(&objects, edit)
            .into_iter()
            .filter(|id| objects.iter().any(|o| o.id() == *id && takes_gradient(o)))
            .collect()
    }

    /// Writes `edit` to the edited objects that can take a gradient, one commit; whether the
    /// document changed.
    pub(super) fn apply_gradient_edit(&mut self, edit: &StyleEdit) -> bool {
        self.recording(|this| -> bool {
            this.flush_style_preview();
            let ids = this.gradient_ids(edit);
            let before = this.document.version();
            !ids.is_empty()
                && this.document.edit_style(&ids, edit).is_ok()
                && this.document.version() != before
        })
    }

    /// The number of stops of the panel's gradient (the first edited one), 0 without one.
    fn panel_stop_count(&self) -> usize {
        self.shaded_objects()
            .first()
            .map_or(0, |shaded| shaded.fill.stops.len())
    }

    /// The active stop, kept inside the panel's list; a new subject starts at the first.
    fn active_stop(&self, subject: Option<NodeId>, count: usize) -> usize {
        if subject != self.gradient.subject {
            return 0;
        }
        self.gradient.active.min(count.saturating_sub(1))
    }

    /// Keeps the notice lines of the gradient features `import` lost (`spreadMethod` repeats,
    /// focal points, mesh gradients), one sentence per line; nothing when none was lost.
    pub(super) fn note_gradient_losses(&mut self, import: &curvyo_ui_core::SvgImport) {
        let lines = curvyo_ui_core::gradient_loss_lines(import.gradient_losses);
        self.gradient.import_notice = (!lines.is_empty()).then(|| lines.join("\n"));
    }

    /// The import notice of criterion 27 once (lines joined by a line feed); empty otherwise.
    pub fn take_gradient_import_notice(&mut self) -> String {
        self.gradient.import_notice.take().unwrap_or_default()
    }

    /// Makes `index` the active stop of the panel's gradient.
    pub fn set_gradient_active_stop(&mut self, index: usize) {
        let first = self.shaded_objects().first().map(|s| s.object.id());
        self.gradient.subject = first;
        self.gradient.active = index.min(self.panel_stop_count().saturating_sub(1));
    }
}
