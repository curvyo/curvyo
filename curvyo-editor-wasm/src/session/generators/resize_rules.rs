//! When a generated selection resizes and when it must not (`specs/0141` criterion 5, ADR 0080 §5,
//! ADR 0092 §4): the unavailable generator, the proportional or free handles and the refusal notices.

use curvyo_ui_core::instance;

use super::{LOADING_FONT_NOTICE, OFF_RESIZE_NOTICE, UNKNOWN_RESIZE_NOTICE};
use crate::session::Session;

impl Session {
    /// Whether the selection holds a generated object whose generator is unknown or switched off:
    /// it is kept as stored and cannot be resized or changed by number (ADR 0080 §5).
    pub(in crate::session) fn selection_has_unavailable_generated(&self) -> bool {
        self.selected_generated().iter().any(|id| {
            self.document.generated(*id).is_some_and(|snapshot| {
                instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind).is_none()
            })
        })
    }

    /// Whether the selection is generated objects and nothing else.
    pub(in crate::session) fn selection_all_generated(&self) -> bool {
        !self.selection.ids().is_empty()
            && self.selected_generated().len() == self.selection.ids().len()
    }

    /// Whether the selection is generated objects of kinds that resize freely (a 1D code) with
    /// frames along the same axes: edges and corners resize, no skew (ADR 0092 §4). A
    /// proportional kind in it, or frames turned against each other, keeps the corners only.
    pub fn selection_resizes_freely(&self) -> bool {
        if !self.selection_all_generated() {
            return false;
        }
        let mut first: Option<(f64, f64)> = None;
        for id in self.selected_generated() {
            let Some(snapshot) = self.document.generated(id) else {
                return false;
            };
            let free = instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind)
                .is_some_and(|found| found.kind.resize == curvyo_extension_core::Resize::Free);
            let axis = (snapshot.frame.a, snapshot.frame.b);
            let aligned = first.is_none_or(|(a, b)| {
                let cross = a * axis.1 - b * axis.0;
                let dot = a * axis.0 + b * axis.1;
                cross.abs() < 1e-6 || dot.abs() < 1e-6
            });
            if !free || !aligned {
                return false;
            }
            first = first.or(Some(axis));
        }
        true
    }

    /// Tells the Select tool whether the next resize keeps the proportions (generated objects only,
    /// unless they all resize freely).
    pub(in crate::session) fn sync_generated_resize_mode(&mut self) {
        let proportional = self.selection_all_generated() && !self.selection_resizes_freely();
        self.select.set_proportional_resize(proportional);
    }

    /// A press on a resize handle, or a size entry, of a selection with an unavailable generated
    /// object is refused with the status of UX notes §12.
    pub(in crate::session) fn refuse_unavailable_generated_resize(&mut self) -> bool {
        let resizing = matches!(
            self.select.dragging_handle(),
            Some(curvyo_ui_core::EditHandle::Resize(_))
        );
        let sizing = self.select.size_entry_open();
        let waiting = self.selected_code_loading();
        if !(resizing || sizing) || !(self.selection_has_unavailable_generated() || waiting) {
            return false;
        }
        if resizing {
            self.select.escape();
        }
        if sizing {
            self.select.cancel_entry();
        }
        if waiting && !self.selection_has_unavailable_generated() {
            self.transform_notice = Some(LOADING_FONT_NOTICE);
            return true;
        }
        // A kind the registry has, with its extension off, is told apart from one it has not.
        let all_on = curvyo_ui_core::ExtensionSettings::default();
        let unknown = self.selected_generated().iter().any(|id| {
            self.document.generated(*id).is_some_and(|snapshot| {
                instance::find_kind(&self.manufacturing.extensions, &snapshot.record.kind).is_none()
                    && instance::find_kind(&all_on, &snapshot.record.kind).is_none()
            })
        });
        self.transform_notice = Some(if unknown {
            UNKNOWN_RESIZE_NOTICE
        } else {
            OFF_RESIZE_NOTICE
        });
        true
    }
}
