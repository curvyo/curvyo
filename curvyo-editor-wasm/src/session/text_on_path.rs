//! The session state of text on path (`specs/0080-text-on-path` slice 1) with the reads of a text
//! on its carrier that the sibling `text_on_path_*` modules share. The math lives in
//! `curvyo-ui-core` (`text_on_path`, `text_path_edit`), the storage in `curvyo-document-core`.

use curvyo_document_core::{CarrierRef, NodeId, ObjectSnapshot, Point, TextSnapshot};
use curvyo_text_core::TextLayout;
use curvyo_ui_core::PathPlacement;

use super::Session;
use super::text::edit_id;
use super::text_on_path_drag::PathDrag;
use super::text_on_path_handle_entry::HandleEntry;
use super::text_on_path_pick::Pick;

/// The session state of text on path.
#[derive(Debug, Default)]
pub(super) struct TextPathState {
    pub(super) pick: Option<Pick>,
    /// The carrier under the pointer in pick mode and its outline index.
    pub(super) hover: Option<CarrierRef>,
    pub(super) drag: Option<PathDrag>,
    /// The entry box of the position handle.
    pub(super) entry: Option<HandleEntry>,
    /// The polite announcement after a put ("Put on path"), until the host takes it.
    pub(super) announcement: Option<&'static str>,
    /// The canvas notice of a put on several carriers ("Put text on 3 paths.", criterion 37),
    /// until the host takes it.
    pub(super) notice: Option<String>,
}

/// The layout and the placement of a text on its carrier.
pub(super) struct OnCarrier {
    pub(super) laid: TextLayout,
    pub(super) placement: PathPlacement,
}

impl Session {
    /// The face's horizontal layout of `text` and its placement on the carrier, or `None` for a
    /// straight text or while the face loads.
    pub(super) fn on_carrier(&self, text: &TextSnapshot) -> Option<OnCarrier> {
        let on_path = text.on_path.as_ref()?;
        on_path.curve.as_ref()?;
        let face = self.face_of(&text.font)?;
        let laid = curvyo_ui_core::text_layout(text, face)?;
        let placement = PathPlacement::new(&laid, on_path)?;
        Some(OnCarrier { laid, placement })
    }

    /// The text `id` as read (its carrier resolved), with what the session shows of it.
    pub(super) fn read_text(&self, id: NodeId) -> Option<TextSnapshot> {
        self.document_read()
            .leaves
            .iter()
            .find_map(|object| match object {
                ObjectSnapshot::Text(text) if text.id == id => Some((**text).clone()),
                _ => None,
            })
    }

    /// The one selected text on path (outside edit mode), or the edited one.
    pub(super) fn sole_text_on_path(&self) -> Option<TextSnapshot> {
        if let Some(edit) = self.text.edit.as_ref() {
            let id = edit_id(edit)?;
            return self.read_text(id).filter(|t| t.on_path.is_some());
        }
        let [id] = self.selection.ids() else {
            return None;
        };
        self.read_text(*id)
            .filter(|t| t.on_path.as_ref().is_some_and(|p| p.curve.is_some()))
    }

    /// The text caret position near `point` on a text laid out along its carrier, or `None` for
    /// a straight text.
    pub(super) fn caret_on_path(&self, text: &TextSnapshot, point: Point) -> Option<usize> {
        let on = self.on_carrier(text)?;
        Some(curvyo_ui_core::nearest_boundary_on_path(
            &on.laid,
            &on.placement,
            point,
        ))
    }

    /// The edit geometry of `snapshot` (the edited text as shown) along its carrier.
    pub(super) fn edit_geometry_on_path(
        &self,
        snapshot: &TextSnapshot,
        caret: usize,
        selection: &std::ops::Range<usize>,
        preedit: Option<&std::ops::Range<usize>>,
    ) -> Option<curvyo_ui_core::TextEditGeometry> {
        let on = self.on_carrier(snapshot)?;
        Some(curvyo_ui_core::edit_geometry_on_path(
            &on.laid,
            &on.placement,
            caret,
            selection,
            preedit,
        ))
    }
}
