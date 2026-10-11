//! What `Session` shows and answers for the reference of the typed move
//! (`specs/0047-transform-polish/` Part C): the pale red true bounding box during a move drag and
//! while the move chip is open, the nine marks in the chip's Absolute mode, the press on a mark
//! and the hint and cursor over one. The reference is view state of the open entry
//! (`curvyo_ui_core::MoveEntry`), reset when the chip opens; the DOM chip holds the text, the mode
//! and the focus, and reports the mode, the chosen point and the radio under the pointer here.

use curvyo_document_core::{Point, ViewTransform};
use curvyo_render_core::{
    DrawList, ReferenceBoxInput, ReferenceMark, build_reference_box_draw_list,
    build_reference_marks_draw_list,
};
use curvyo_ui_core::{
    EntryOutcome, Modifiers, MoveEntry, MoveEntryMode, ReferencePoint, hit_mark, visible_marks,
};

use super::{Session, Tool};

impl Session {
    /// The open move chip's entry, if it is the one shown (`Session::move_entry` checks the
    /// same).
    fn reference_entry(&self) -> Option<&MoveEntry> {
        if self.tool != Tool::Select {
            return None;
        }
        let entry = self.select.move_entry()?;
        (self.selection.ids() == entry.ids()).then_some(entry)
    }

    /// The marks that exist right now: those of the open chip in Absolute mode that are drawn and
    /// clickable, nothing in Relative mode or while the box is under 32 px (criterion 39).
    fn visible_reference_marks(&self, entry: &MoveEntry) -> Vec<ReferencePoint> {
        if !entry.is_absolute() {
            return Vec::new();
        }
        visible_marks(entry.bounds(), self.view(), self.viewport.canvas_size())
    }

    /// The mark a press at the document point `at` is on, if any: only while the move chip is
    /// open in Absolute mode (criterion 40). `touch` for a touch or a pen.
    #[must_use]
    pub fn move_mark_at(&self, at: Point, touch: bool) -> Option<ReferencePoint> {
        let entry = self.reference_entry()?;
        let marks = self.visible_reference_marks(entry);
        if marks.is_empty() {
            return None;
        }
        let view = self.view();
        let pointer = view.document_to_screen(at);
        hit_mark(entry.bounds(), &marks, view, pointer, touch)
    }

    /// The mark under the pointer, for the hover state, the hint chip and the prefill of the
    /// fields (criteria 43 and 44).
    #[must_use]
    pub fn move_mark_hover(&self) -> Option<ReferencePoint> {
        self.move_mark_at(self.pointer_position?, false)
    }

    /// The chip reports its mode: Relative or Absolute (the marks exist in Absolute only).
    pub fn set_move_chip_absolute(&mut self, absolute: bool) {
        if let Some(entry) = self.select.move_entry_mut() {
            entry.set_absolute(absolute);
        }
    }

    /// The chip reports the reference point its Reference control chose.
    pub fn set_move_reference(&mut self, reference: ReferencePoint) {
        if let Some(entry) = self.select.move_entry_mut() {
            entry.set_reference(reference);
        }
    }

    /// The chip reports the radio with the pointer or the focus, `None` when there is none.
    pub fn set_move_radio_hover(&mut self, hover: Option<ReferencePoint>) {
        if let Some(entry) = self.select.move_entry_mut() {
            entry.set_radio_hover(hover);
        }
    }

    /// A click on a mark: the move executes with that mark's point as the reference, in Absolute
    /// mode, with the typed texts and the state of the Copy check (criterion 40). A committed or
    /// unchanged entry closes; an invalid one stays open.
    pub fn commit_move_entry_at(
        &mut self,
        first: &str,
        second: &str,
        copy: bool,
        reference: ReferencePoint,
    ) -> EntryOutcome {
        // Recorded as one step of the maker's, like the Enter of the same chip.
        self.recording(|this| -> EntryOutcome {
            if this.reference_entry().is_none() {
                this.select.cancel_entry();
                return EntryOutcome::Unchanged;
            }
            this.select.commit_move_entry_at(
                &this.document,
                &mut this.selection,
                &mut this.minter,
                [first, second],
                (
                    MoveEntryMode {
                        absolute: true,
                        copy,
                    },
                    reference,
                ),
            )
        })
    }

    /// The box and marks to draw this frame: the true bounding box of the move drag in flight
    /// (criterion 33) or of the selection while the move chip is open (criterion 34), and the
    /// marks of the chip in Absolute mode (criterion 39). `None` when there is neither.
    pub(super) fn reference_decoration(&self) -> Option<ReferenceBoxInput> {
        if self.tool != Tool::Select {
            return None;
        }
        let (bounds, marks) = if let Some(entry) = self.reference_entry() {
            let hovered = self.move_mark_hover().or(entry.radio_hover());
            let marks = self
                .visible_reference_marks(entry)
                .into_iter()
                .map(|mark| ReferenceMark {
                    at: mark.point(entry.bounds()),
                    cross: mark.is_centre(),
                    current: mark == entry.reference(),
                    hovered: Some(mark) == hovered,
                })
                .collect();
            (entry.bounds(), marks)
        } else {
            let pointer = self.pointer_position?;
            let bounds = self
                .select
                .live_move_box(pointer, Modifiers::new(self.held.shift, self.held.ctrl))?;
            (bounds, Vec::new())
        };
        Some(ReferenceBoxInput {
            bounds,
            marks,
            canvas_px: self.viewport.canvas_size(),
            device_pixel_ratio: self.device_pixel_ratio,
        })
    }
}

/// The pale red box of `input`, nothing for `None`.
pub(super) fn reference_box_list(
    view: ViewTransform,
    input: Option<&ReferenceBoxInput>,
) -> DrawList {
    input.map_or_else(DrawList::default, |input| {
        build_reference_box_draw_list(view, input)
    })
}

/// The marks of `input`, nothing for `None`.
pub(super) fn reference_marks_list(
    view: ViewTransform,
    input: Option<&ReferenceBoxInput>,
) -> DrawList {
    input.map_or_else(DrawList::default, |input| {
        build_reference_marks_draw_list(view, input)
    })
}
