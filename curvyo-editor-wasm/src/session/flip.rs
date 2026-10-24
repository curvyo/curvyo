//! `Session`'s glue for the two flip buttons and the keys H and V
//! (`specs/0056-mirror-and-negative-scale/` criteria 1 to 5): when a flip may act, and the
//! flip itself. The rule, the no-op test and the commit are `curvyo_ui_core::flip_selection`.

use curvyo_ui_core::{FlipAxis, FlipOutcome, flip_selection};

use super::{Session, Tool};

impl Session {
    /// Whether the flip buttons act now (criterion 1): the Select tool with something
    /// selected. They are shown in every selection state and only disabled with nothing
    /// selected.
    #[must_use]
    pub fn flip_enabled(&self) -> bool {
        self.tool == Tool::Select && !self.selection.ids().is_empty()
    }

    /// Flips the selection by `axis` about the centre of its box, as one step named
    /// "Flip horizontally" or "Flip vertically" (criteria 2 and 29). An open chip closes
    /// first without writing (criterion 5). Nothing happens outside the Select tool or
    /// while a drag runs ([`FlipOutcome::NothingSelected`]).
    pub fn flip_selection(&mut self, axis: FlipAxis) -> FlipOutcome {
        if self.tool != Tool::Select || self.select.drag_in_flight() {
            return FlipOutcome::NothingSelected;
        }
        self.select.cancel_entry();
        // Texts on path alone toggle Mirror letters (`specs/0080-text-on-path` criterion 19).
        if self.selection_is_texts_on_path() && self.flip_text_paths() {
            return FlipOutcome::Flipped;
        }
        self.recording(|this| {
            let objects = this.objects();
            flip_selection(&this.document, &objects, &this.selection, axis)
        })
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::Point;
    use curvyo_ui_core::{FlipAxis, FlipOutcome};

    use super::super::{Session, Tool};

    /// Criteria 1 and 5: no flip with nothing selected or outside the Select tool; a
    /// flip closes an open chip and writes the flip only.
    #[test]
    fn the_flip_needs_a_selection_and_closes_a_chip() {
        let mut session = Session::new(1);
        assert!(!session.flip_enabled());
        assert_eq!(
            session.flip_selection(FlipAxis::Horizontal),
            FlipOutcome::NothingSelected
        );
        // Two rectangles at different heights: a vertical flip changes them.
        for (from, to) in [((0.0, 0.0), (30.0, 10.0)), ((40.0, 0.0), (50.0, 30.0))] {
            session.set_tool(Tool::Rectangle);
            session.pointer_down(Point::new(from.0, from.1), false);
            session.pointer_up(Point::new(to.0, to.1), false, false);
        }
        assert_eq!(session.tool(), Tool::Select);
        let ids = session.document.object_ids();
        session.selection.set(&ids);
        assert!(session.flip_enabled());
        session
            .select
            .open_entry_for_key(
                &session.objects(),
                &session.selection,
                curvyo_ui_core::EntryKey::Size,
            )
            .expect("opens");
        assert!(session.select.has_entry());
        assert_eq!(
            session.flip_selection(FlipAxis::Vertical),
            FlipOutcome::Flipped
        );
        assert!(!session.select.has_entry());
        session.set_tool(Tool::Node);
        assert!(!session.flip_enabled());
        assert_eq!(
            session.flip_selection(FlipAxis::Vertical),
            FlipOutcome::NothingSelected
        );
    }
}
