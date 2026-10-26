//! The red outline of the objects a refused rail command names (`specs/0016-boolean-operations`
//! criterion 15, `0035-combine-and-break-apart` criteria 9 to 11): one mark for any command, kept
//! only while the selection and the tool it was refused for stay.

use curvyo_document_core::{NodeId, ObjectSnapshot};
use curvyo_ui_core::ObjectSelection;

use super::{Session, Tool};

/// At most this many offenders are outlined; the count in the sentence is exact.
const MAX_OUTLINED: usize = 200;

/// The objects a refusal outlines, and the selection it was refused for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RefusalMarks {
    offenders: Vec<NodeId>,
    selection: Vec<NodeId>,
}

impl RefusalMarks {
    /// The mark for a refusal of `selection` that names `offenders`.
    pub(super) fn new(offenders: Vec<NodeId>, selection: &ObjectSelection) -> Self {
        Self {
            offenders,
            selection: selection.ids().to_vec(),
        }
    }
}

impl Session {
    /// Whether a selected object is an image (`specs/0058-bitmap-import` criteria 37 and 38).
    pub(super) fn selection_holds_image(&self) -> bool {
        let ids = self.selection.ids();
        self.objects()
            .iter()
            .any(|object| matches!(object, ObjectSnapshot::Image(_)) && ids.contains(&object.id()))
    }

    /// Refuses an outline command for a selection that holds an image (`specs/0058-bitmap-import`
    /// criterion 37): the images are outlined red and `true` says "refused, nothing changed".
    pub fn refuse_images(&mut self) -> bool {
        let objects = self.objects();
        let images: Vec<NodeId> = objects
            .iter()
            .filter(|object| {
                matches!(object, ObjectSnapshot::Image(_))
                    && self.selection.ids().contains(&object.id())
            })
            .map(ObjectSnapshot::id)
            .collect();
        if images.is_empty() {
            return false;
        }
        self.command_refusal = Some(RefusalMarks::new(images, &self.selection));
        true
    }

    /// Removes the red outline of a refusal of any rail command, Boolean or Path (the host's
    /// timer ran out, or the next action).
    pub fn clear_boolean_refusal(&mut self) {
        self.command_refusal = None;
    }

    /// The offending objects of the refusal that is showing, or none once the selection or the
    /// tool has changed since.
    pub(super) fn refusal_objects(&self, objects: &[ObjectSnapshot]) -> Vec<ObjectSnapshot> {
        let Some(marks) = &self.command_refusal else {
            return Vec::new();
        };
        if !matches!(self.tool, Tool::Select | Tool::Offset)
            || self.selection.ids() != marks.selection.as_slice()
        {
            return Vec::new();
        }
        objects
            .iter()
            .filter(|object| marks.offenders.contains(&object.id()))
            .take(MAX_OUTLINED)
            .cloned()
            .collect()
    }
}
