//! `Session`'s glue for the typed move
//! (`specs/0010-edit-interaction-polish/specification.md`, criteria 15 to 25, 56;
//! `specs/0047-transform-polish/` Part C): the move chip's view for the DOM and its commit. All
//! rules (parsing, relative and absolute reading, the tight bounds, the reference point, the
//! coordinate limit) live in `curvyo_ui_core::MoveEntry`; the DOM holds the text, the mode switch
//! and the focus, and passes the mode with the commit.

use curvyo_document_core::Point;
use curvyo_ui_core::{EntryOutcome, MoveEntry, MoveEntryMode, ReferencePoint};

use super::{Session, Tool};

/// An open move chip as the host sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct MoveEntryView {
    /// The box centre, in document space: the centre handle is there, or
    /// would be; the chip opens 16 px right of and below it.
    pub center: Point,
    /// What an untouched X and Y field show in Relative mode.
    pub relative_prefill: [String; 2],
    /// What an untouched X and Y field show in Absolute mode for each reference point, in the
    /// order of [`ReferencePoint::ALL`]: its current position (criterion 37).
    pub absolute_prefills: [[String; 2]; 9],
    /// The chosen reference point: the centre until the maker chooses another.
    pub reference: ReferencePoint,
    /// The lower edge of the true bounding box, in document space under the box centre: the chip
    /// is placed 16 px below it (UX notes, "Chip").
    pub box_bottom: Point,
    /// Whether the Copy check opens on: Ctrl was held at the second press of
    /// the double-click (criterion 23).
    pub copy_preset: bool,
}

impl Session {
    /// The open move entry, if the Select tool is active and its objects are
    /// still the selection.
    fn open_move_entry(&self) -> Option<&MoveEntry> {
        if self.tool != Tool::Select {
            return None;
        }
        let entry = self.select.move_entry()?;
        (self.selection.ids() == entry.ids()).then_some(entry)
    }

    /// The move chip to show, or `None`.
    #[must_use]
    pub fn move_entry(&self) -> Option<MoveEntryView> {
        let entry = self.open_move_entry()?;
        let box_ = entry.start_box();
        let relative = |index: usize| entry.fields()[index].prefill.clone();
        let (min, max) = entry.bounds();
        Some(MoveEntryView {
            center: box_.to_document(box_.local_center()),
            relative_prefill: [relative(0), relative(1)],
            absolute_prefills: ReferencePoint::ALL
                .map(|reference| entry.absolute_prefill_for(reference)),
            reference: entry.reference(),
            box_bottom: Point::new(f64::midpoint(min.x, max.x), max.y),
            copy_preset: entry.copy_preset(),
        })
    }

    /// Enter in the move chip: `first` and `second` are the X and Y texts,
    /// `absolute` the chip's mode and `copy` its Copy check. A committed or
    /// unchanged entry closes; an invalid one stays open. After a typed copy
    /// the selection is the copy.
    pub fn commit_move_entry(
        &mut self,
        first: &str,
        second: &str,
        mode: MoveEntryMode,
    ) -> EntryOutcome {
        if self.open_move_entry().is_none() {
            self.select.cancel_entry();
            return EntryOutcome::Unchanged;
        }
        self.select.commit_move_entry(
            &self.document,
            &mut self.selection,
            &mut self.minter,
            [first, second],
            mode,
        )
    }
}
