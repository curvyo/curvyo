//! `Session`'s Pen entry chip (`specs/0072-pen-angle-snap-and-typed-values/` Part B): Tab opens
//! it, the pointer feeds it until the first edit, and Enter, a frozen press or a cancel ends it.
//! A child module of `session`, so its methods join the same type's `impl Session`.

use curvyo_document_core::Point;
use curvyo_ui_core::{
    PEN_ENTRY_GROUP_NAME, PEN_ENTRY_LENGTH_NOT_POSITIVE, PEN_ENTRY_OPENED_SENTENCE, PenEntry,
    PenEntryField, PenEntryPlaced, PenTarget, PressAction, pen_entry_added_sentence,
    pen_entry_label, pen_entry_name, pen_entry_notice, pen_entry_suffix,
};

use super::{Session, Tool};

/// The open entry and what the host reads from it once.
#[derive(Debug, Default)]
pub(super) struct PenEntryState {
    entry: Option<PenEntry>,
    /// What the live region says next (criterion 34).
    announcement: Option<String>,
    /// What the canvas notice slot shows next (criteria 24 to 29).
    notice: Option<String>,
}

/// One field of the chip as the host draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PenEntryFieldView {
    /// The visible label: "Length" or "Angle".
    pub label: &'static str,
    /// The accessible name: "Length, millimetres" or "Angle, degrees".
    pub name: String,
    /// The display unit's symbol or "°".
    pub suffix: &'static str,
    /// The maker's text, or the live or kept value.
    pub text: String,
}

/// The chip as a whole (`adrs.md` decision 7).
#[derive(Debug, Clone, PartialEq)]
pub struct PenEntryChipView {
    /// "Next node".
    pub group_name: &'static str,
    /// A field was edited: the chip stands still (criterion 16).
    pub frozen: bool,
    /// Length and Angle, in Tab order.
    pub fields: Vec<PenEntryFieldView>,
    /// The last node L, document space: the chip keeps clear of it.
    pub last: Point,
    /// The pointer the chip is placed from, document space.
    pub pointer: Point,
    /// The typed point the rubber band ends on, document space; `None` with a zero length.
    pub end: Option<Point>,
}

impl Session {
    /// The open Pen entry, if any.
    #[must_use]
    pub fn pen_entry(&self) -> Option<&PenEntry> {
        self.pen_entries.entry.as_ref()
    }

    /// Tab on the canvas (criterion 12): opens the chip for the segment from the last node.
    /// Returns whether it opened.
    pub(super) fn open_pen_entry(&mut self) -> bool {
        let Some(last) = self.pen_last_node() else {
            return false;
        };
        if self.tool != Tool::Pen || self.button_down {
            return false;
        }
        self.pen_entries.entry = Some(PenEntry::new(last));
        self.pen_entries.announcement = Some(PEN_ENTRY_OPENED_SENTENCE.to_owned());
        true
    }

    /// The maker edited field `index` (0 Length, 1 Angle) to `text` (criterion 16).
    pub fn pen_entry_edit(&mut self, index: usize, text: &str) {
        let Some(&field) = PenEntryField::ALL.get(index) else {
            return;
        };
        let live = self.pen_entry_live_end();
        let pointer = self.pointer_position.unwrap_or(live);
        if let Some(entry) = self.pen_entries.entry.as_mut() {
            entry.edit(field, text, live, pointer);
        }
    }

    /// Enter in the chip, or a press with the chip frozen (criteria 19, 20, 24 to 28): adds one
    /// node exactly as a click at the point adds it and closes the chip. With a zero length the
    /// chip stays open with the notice and nothing is added. Returns whether a node was added.
    pub fn pen_entry_place(&mut self) -> bool {
        let Some(entry) = self.pen_entries.entry.as_ref() else {
            return false;
        };
        // Untouched over a join target, Enter does what the click there does: it joins
        // (criterion 19).
        if !entry.is_frozen()
            && let Some(PenTarget::Join(end)) = self.pen_target()
        {
            self.pen_entries.entry = None;
            let point = end.anchor.point;
            let threshold = self.drag_threshold();
            self.recording(|this| {
                this.pen.pointer_down_action(point, PressAction::Join(end));
                this.pen
                    .pointer_up(&mut this.minter, &this.document, point, threshold);
            });
            return true;
        }
        let unit = self.display_unit();
        let Ok(placed) = entry.resolve(self.pen_entry_live_end(), unit) else {
            self.pen_entries.notice = Some(PEN_ENTRY_LENGTH_NOT_POSITIVE.to_owned());
            return false;
        };
        self.pen_entries.entry = None;
        self.pen_add_node(&placed);
        self.pen_entries.announcement = Some(pen_entry_added_sentence(&placed, unit));
        self.pen_entries.notice = placed.notice.map(|notice| pen_entry_notice(notice, unit));
        true
    }

    /// A click at the typed point: a press and a release with no drag (`adrs.md` decision 5).
    fn pen_add_node(&mut self, placed: &PenEntryPlaced) {
        let threshold = self.drag_threshold();
        self.pen
            .pointer_down_action(placed.point, PressAction::Place);
        self.pen
            .pointer_up(&mut self.minter, &self.document, placed.point, threshold);
    }

    /// Escape in the chip, a tool switch, a press outside, a finish (criterion 22): closes the
    /// chip and adds nothing. Returns whether it was open.
    pub fn cancel_pen_entry(&mut self) -> bool {
        self.pen_entries.entry.take().is_some()
    }

    /// A Pen press with the chip open (criterion 20): frozen, the press adds the typed node and
    /// does nothing else (returns `true`); not frozen, the chip closes and the press goes on as
    /// an ordinary Pen press (returns `false`).
    pub(super) fn pen_entry_press(&mut self) -> bool {
        match self.pen_entry() {
            None => false,
            Some(entry) if entry.is_frozen() => {
                self.pen_entry_place();
                true
            }
            Some(_) => {
                self.cancel_pen_entry();
                false
            }
        }
    }

    /// Where the rubber band ends while the chip is open: the point the chip's values give
    /// (criterion 17); `None` with a zero length.
    pub(super) fn pen_entry_preview_end(&self) -> Option<Point> {
        let entry = self.pen_entry()?;
        entry
            .resolve(self.pen_entry_live_end(), self.display_unit())
            .ok()
            .map(|placed| placed.point)
    }

    /// The end of the live segment the chip follows: the rubber band's end without the chip, or
    /// L itself with the pointer off the canvas.
    fn pen_entry_live_end(&self) -> Point {
        self.pen_segment_end()
            .or(self.pointer_position)
            .or_else(|| self.pen_last_node())
            .unwrap_or(Point::new(0.0, 0.0))
    }

    /// The chip, or `None` when it is closed.
    #[must_use]
    pub fn pen_entry_chip(&self) -> Option<PenEntryChipView> {
        let entry = self.pen_entry()?;
        let unit = self.display_unit();
        let live = self.pen_entry_live_end();
        let fields = PenEntryField::ALL
            .iter()
            .map(|&field| PenEntryFieldView {
                label: pen_entry_label(field),
                name: pen_entry_name(field, unit),
                suffix: pen_entry_suffix(field, unit),
                text: entry.field_text(field, live, unit),
            })
            .collect();
        Some(PenEntryChipView {
            group_name: PEN_ENTRY_GROUP_NAME,
            frozen: entry.is_frozen(),
            fields,
            last: entry.last(),
            pointer: entry
                .frozen_pointer()
                .or(self.pointer_position)
                .unwrap_or(live),
            end: self.pen_entry_preview_end(),
        })
    }

    /// What the live region says next, taken once.
    pub fn take_pen_entry_announcement(&mut self) -> Option<String> {
        self.pen_entries.announcement.take()
    }

    /// What the canvas notice slot shows next, taken once.
    pub fn take_pen_entry_notice(&mut self) -> Option<String> {
        self.pen_entries.notice.take()
    }
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{NewAnchor, Point};

    use super::super::keys::{KeyInput, KeyOutcome};
    use super::super::{Session, Tool};

    fn key(session: &mut Session, key: &str) -> KeyOutcome {
        session.key_down(KeyInput {
            key,
            ..KeyInput::default()
        })
    }

    fn click(session: &mut Session, at: Point) {
        session.pointer_down(at, false);
        session.pointer_up(at, false, false);
    }

    /// A Pen with one node at L = (100, 50) and the pointer at (130, 40).
    fn pen_at_l() -> Session {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        click(&mut session, Point::new(100.0, 50.0));
        session.pointer_hover(Point::new(130.0, 40.0), false, false);
        session
    }

    fn points(session: &Session) -> Vec<(f64, f64)> {
        session
            .pen_in_progress()
            .unwrap_or_default()
            .iter()
            .map(|node: &NewAnchor| {
                (
                    (node.point.x * 100.0).round() / 100.0,
                    (node.point.y * 100.0).round() / 100.0,
                )
            })
            .collect()
    }

    fn type_node(session: &mut Session, length: &str, angle: &str) -> bool {
        assert_eq!(key(session, "Tab"), KeyOutcome::PenEntryOpened);
        session.pen_entry_edit(0, length);
        session.pen_entry_edit(1, angle);
        session.pen_entry_place()
    }

    /// Criterion 12: Tab opens the chip only with a last node; with no path it is not consumed.
    #[test]
    fn tab_opens_the_chip_only_with_a_last_node() {
        let mut idle = Session::new(1);
        idle.set_tool(Tool::Pen);
        assert_eq!(key(&mut idle, "Tab"), KeyOutcome::Ignored);
        let mut session = pen_at_l();
        assert_eq!(key(&mut session, "Tab"), KeyOutcome::PenEntryOpened);
        let chip = session.pen_entry_chip().expect("the chip");
        assert_eq!(chip.fields[0].text, "31.6");
        assert_eq!(chip.fields[1].text, "-18.4");
        assert_eq!(
            session.take_pen_entry_announcement().as_deref(),
            Some("Next node.")
        );
    }

    /// Criterion 14: with the button held Tab is consumed and opens nothing.
    #[test]
    fn tab_during_a_drag_is_consumed_and_opens_nothing() {
        let mut session = pen_at_l();
        session.pointer_down(Point::new(130.0, 40.0), false);
        assert_eq!(key(&mut session, "Tab"), KeyOutcome::Consumed);
        assert!(session.pen_entry().is_none());
    }

    /// Criteria 19, 34: the polyline of the spec; the Pen keeps drawing.
    #[test]
    fn enter_adds_a_corner_node_and_keeps_drawing() {
        let mut session = pen_at_l();
        assert!(type_node(&mut session, "40", "0"));
        assert_eq!(
            session.take_pen_entry_announcement().as_deref(),
            Some("Node added, 40 millimetres at 0 degrees.")
        );
        assert!(type_node(&mut session, "25", "90"));
        assert!(type_node(&mut session, "40", "180"));
        assert_eq!(
            points(&session),
            [(100.0, 50.0), (140.0, 50.0), (140.0, 75.0), (100.0, 75.0)]
        );
        assert!(session.pen_entry().is_none());
        assert!(session.pen_in_progress().unwrap()[1].handle_out.length() < 1e-9);
    }

    /// Criterion 19a (customer 2026-10-19): a right press finishes the path, places nothing.
    #[test]
    fn a_right_press_finishes_the_path_like_enter() {
        let mut session = pen_at_l();
        click(&mut session, Point::new(140.0, 50.0));
        assert_eq!(key(&mut session, "Tab"), KeyOutcome::PenEntryOpened);
        assert!(session.pen_secondary_press());
        assert!(session.pen_in_progress().is_none());
        assert!(session.pen_entry().is_none());
        let path = session.document.object_ids()[0];
        assert_eq!(session.document.path(path).unwrap().anchors.len(), 2);
    }

    /// Criterion 20: a frozen chip places at the press wherever it is; an unfrozen one closes and
    /// the press is an ordinary one.
    #[test]
    fn a_press_places_when_frozen_and_closes_otherwise() {
        let mut session = pen_at_l();
        key(&mut session, "Tab");
        session.pen_entry_edit(0, "40");
        session.pen_entry_edit(1, "0");
        click(&mut session, Point::new(0.0, 0.0));
        assert_eq!(points(&session), [(100.0, 50.0), (140.0, 50.0)]);
        key(&mut session, "Tab");
        click(&mut session, Point::new(10.0, 10.0));
        assert!(session.pen_entry().is_none());
        assert_eq!(points(&session).last(), Some(&(10.0, 10.0)));
    }

    /// Criteria 22, 32: Escape closes the chip alone and writes nothing; the next discards.
    #[test]
    fn escape_closes_the_chip_then_discards_the_path() {
        let mut session = pen_at_l();
        let version = session.document.version();
        key(&mut session, "Tab");
        session.pen_entry_edit(0, "40");
        key(&mut session, "Escape");
        assert!(session.pen_entry().is_none());
        assert_eq!(points(&session), [(100.0, 50.0)]);
        assert_eq!(session.document.version(), version);
        key(&mut session, "Escape");
        assert!(session.pen_in_progress().is_none());
    }

    /// Criterion 26: a zero length leaves the chip open with the notice.
    #[test]
    fn a_zero_length_keeps_the_chip_open_with_the_notice() {
        let mut session = pen_at_l();
        assert!(!type_node(&mut session, "0", "0"));
        assert!(session.pen_entry().is_some());
        assert_eq!(
            session.take_pen_entry_notice().as_deref(),
            Some("Length must be more than 0.")
        );
    }

    /// Criterion 23: an open chip gates the letters and stops a stray Enter finishing the path.
    #[test]
    fn the_open_chip_gates_letters_and_enter() {
        let mut session = pen_at_l();
        key(&mut session, "Tab");
        assert_eq!(key(&mut session, "n"), KeyOutcome::Ignored);
        assert_eq!(key(&mut session, "Enter"), KeyOutcome::Ignored);
        assert_eq!(session.tool(), Tool::Pen);
        assert!(session.pen_in_progress().is_some());
    }

    /// Criterion 19: untouched over another path's end, Enter joins as the click would.
    #[test]
    fn enter_untouched_over_a_join_target_joins() {
        let mut session = Session::new(1);
        session.set_tool(Tool::Pen);
        click(&mut session, Point::new(0.0, 100.0));
        click(&mut session, Point::new(40.0, 100.0));
        session.finish_pen();
        click(&mut session, Point::new(100.0, 50.0));
        session.pointer_hover(Point::new(40.0, 100.0), false, false);
        assert_eq!(key(&mut session, "Tab"), KeyOutcome::PenEntryOpened);
        assert!(session.pen_entry_place());
        assert!(session.pen_in_progress().is_none());
        assert_eq!(session.document.object_ids().len(), 1);
        let path = session.document.object_ids()[0];
        assert_eq!(session.document.path(path).unwrap().anchors.len(), 3);
    }

    /// Criterion 22: a tool switch closes the chip and adds nothing.
    #[test]
    fn a_tool_switch_closes_the_chip() {
        let mut session = pen_at_l();
        click(&mut session, Point::new(140.0, 50.0));
        key(&mut session, "Tab");
        session.pen_entry_edit(0, "40");
        session.set_tool(Tool::Select);
        assert!(session.pen_entry().is_none());
        let path = session.document.object_ids()[0];
        assert_eq!(session.document.path(path).unwrap().anchors.len(), 2);
    }
}
