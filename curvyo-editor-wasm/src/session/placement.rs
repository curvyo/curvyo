//! `Session`'s click-to-create placement (`specs/0052-click-to-create-shapes/`, `adrs.md`
//! decision 1): a click with a creation tool starts it, the pointer and the chip's fields shape
//! it, and a second press, Enter or a cancel ends it. A child module of `session`, so its
//! methods join the same type's `impl Session`.

use curvyo_document_core::{Length, Point};
use curvyo_ui_core::{
    CANCELLED_SENTENCE, CreatePreview, EllipseTool, Modifiers, Placed, PlacementField,
    PlacementKind, PlacementNoun, RectangleTool, ShapePlacement, placed_sentence,
};

use super::{Session, Tool};

/// A tap is a touch press and release closer than this, in screen pixels (criterion 31,
/// `0047` criterion 17).
const TOUCH_TAP_PX: f64 = 8.0;

/// The session's placement facts: the placement, the press it may start from, the pointer
/// kind, and the sentence the live region says next.
#[derive(Debug, Default)]
pub(super) struct PlacementState {
    placement: Option<ShapePlacement>,
    /// Where the creation tool's press in flight went down, to tell a click from a drag.
    press: Option<Point>,
    /// The frontend's report before each press: touch, not mouse or pen (`adrs.md` decision 1).
    touch: bool,
    announcement: Option<String>,
    /// Shift and Ctrl as tap toggles of the running placement (`0052` criteria 3 and 32); they
    /// are inverted against the keys the pointer events report (exclusive or).
    toggled: Modifiers,
    /// The modifier tap that has not yet been told apart from a chord.
    tap: Option<Tap>,
    /// Keys held at the click that started the placement: they became toggles, so the held key
    /// does not count again until it has been released.
    consumed: Modifiers,
}

/// Which modifier a pending tap toggled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tap {
    Shift,
    Ctrl,
}

impl Session {
    /// The running placement, if any.
    #[must_use]
    pub fn placement(&self) -> Option<&ShapePlacement> {
        self.placements.placement.as_ref()
    }

    /// The frontend says whether the next press is a touch (criterion 31).
    pub fn set_touch_input(&mut self, touch: bool) {
        self.placements.touch = touch;
    }

    /// What the live region says next (criterion 30), taken once.
    pub fn take_placement_announcement(&mut self) -> Option<String> {
        self.placements.announcement.take()
    }

    /// The press-to-release distance under which a release is a click: 3 px, 8 px on touch,
    /// at the current zoom (criteria 1 and 31).
    fn click_threshold(&self) -> Length {
        if self.placements.touch {
            Length::from_mm(TOUCH_TAP_PX / self.view().scale())
        } else {
            self.drag_threshold()
        }
    }

    /// A creation tool's press went down at `point`: remembered to decide at the release.
    pub(super) fn note_shape_press(&mut self, point: Point) {
        self.placements.press = Some(point);
    }

    /// The creation tool's release at `point`: under the click threshold from the press, the
    /// tool's drag is dropped and a placement starts at the press (criterion 1). Returns whether
    /// it did; otherwise the create-drag commits as before.
    pub(super) fn release_starts_placement(&mut self, point: Point, modifiers: Modifiers) -> bool {
        let Some(press) = self.placements.press.take() else {
            return false;
        };
        if press.vector_to(point).length() >= self.click_threshold().as_mm() {
            return false;
        }
        self.shape_escape();
        self.start_placement(press, modifiers);
        true
    }

    /// Enter on the canvas (criterion 26): a placement at the pointer, or at the viewport's
    /// centre when the pointer is outside it. Refused, with the notice, on a hidden or locked
    /// layer.
    pub(super) fn start_keyboard_placement(&mut self) -> bool {
        if self.refuse_drawing_into_layer() {
            return false;
        }
        let a = self.pointer_position.unwrap_or_else(|| {
            let (width, height) = self.viewport.canvas_size();
            self.viewport.screen_to_document(width / 2.0, height / 2.0)
        });
        self.start_placement(a, Modifiers::NONE);
        true
    }

    /// Enter's key outcome (criterion 26).
    pub(super) fn start_placement_key(&mut self) -> super::KeyOutcome {
        if self.start_keyboard_placement() {
            super::KeyOutcome::PlacementStarted
        } else {
            super::KeyOutcome::Ignored
        }
    }

    /// The modifiers a placement works with: the keys the pointer reports, inverted by the tap
    /// toggles (exclusive or).
    pub(super) fn placement_effective(&self, held: Modifiers) -> Modifiers {
        let PlacementState {
            toggled, consumed, ..
        } = self.placements;
        Modifiers::new(
            (held.shift && !consumed.shift) != toggled.shift,
            (held.ctrl && !consumed.ctrl) != toggled.ctrl,
        )
    }

    /// A pointer event reports `held`: a key consumed by the starting click that is up again
    /// counts as a held key from its next press on.
    fn placement_note_release(&mut self, held: Modifiers) {
        let consumed = &mut self.placements.consumed;
        consumed.shift &= held.shift;
        consumed.ctrl &= held.ctrl;
    }

    fn start_placement(&mut self, a: Point, modifiers: Modifiers) {
        let Some(kind) = placement_kind(self.tool) else {
            return;
        };
        // A key held at the click latches its toggle on (customer feedback 2026-10-16, round 3).
        self.placements.toggled = Modifiers::new(
            modifiers.shift && !matches!(kind, PlacementKind::PolyStar),
            modifiers.ctrl,
        );
        self.placements.consumed = modifiers;
        self.placements.tap = None;
        let modifiers = self.placement_effective(modifiers);
        let mut placement = ShapePlacement::new(kind, a, modifiers);
        if let Some(pointer) = self.pointer_position {
            placement.follow(pointer, modifiers);
        }
        let noun = PlacementNoun::of(kind, self.poly_star.mode());
        self.placements.placement = Some(placement);
        self.placements.announcement = Some(noun.start_sentence());
        self.refresh_placement();
    }

    /// The live shape of the running placement while it follows (criterion 2): the tool's
    /// `shape_for` from A to the pointer, `None` within the click threshold of A, with its pivot
    /// at A (criterion 28).
    pub(super) fn placement_live(&self) -> Option<CreatePreview> {
        let placement = self.placements.placement.as_ref()?;
        let (pointer, modifiers) = placement.following()?;
        let a = placement.a();
        if a.vector_to(pointer).length() < self.click_threshold().as_mm() {
            return None;
        }
        let live = match placement.kind() {
            PlacementKind::Rect => RectangleTool::shape_for(a, pointer, modifiers),
            PlacementKind::Ellipse => EllipseTool::shape_for(a, pointer, modifiers),
            PlacementKind::PolyStar => self.poly_star.shape_for(a, pointer, modifiers),
        }?;
        Some(CreatePreview {
            pivot: Some(a),
            ..live
        })
    }

    /// Brings the preview up to date after a change of the pointer, the modifiers, a field or
    /// the polygon/star bar (criteria 2, 3, 15, 25).
    pub(super) fn refresh_placement(&mut self) {
        let live = self.placement_live();
        let settings = self.poly_star.settings();
        let unit = self.display_unit();
        if let Some(placement) = &mut self.placements.placement {
            placement.refresh(live, settings, unit);
        }
    }

    /// The pointer moved to `point` with `modifiers` (criteria 2 and 3).
    pub(super) fn placement_follow(&mut self, point: Point, modifiers: Modifiers) {
        self.placement_note_release(modifiers);
        let modifiers = self.placement_effective(modifiers);
        if let Some(placement) = &mut self.placements.placement {
            placement.follow(point, modifiers);
        }
        self.refresh_placement();
    }

    /// The modifiers changed with the pointer still (criterion 3).
    pub(super) fn placement_modifiers(&mut self, modifiers: Modifiers) {
        let modifiers = self.placement_effective(modifiers);
        if let Some(placement) = &mut self.placements.placement {
            placement.set_modifiers(modifiers);
        }
        self.refresh_placement();
    }

    /// A primary press at `point` during a placement (criteria 4, 5, 22): a frozen placement
    /// places wherever the press is; an unfrozen one places the live shape for the press and
    /// its modifiers, unless the press is within the click threshold of A, which is ignored.
    pub(super) fn placement_press(&mut self, point: Point, modifiers: Modifiers) {
        self.placement_note_release(modifiers);
        let modifiers = self.placement_effective(modifiers);
        let threshold = self.click_threshold().as_mm();
        let Some(placement) = &mut self.placements.placement else {
            return;
        };
        if !placement.is_frozen() {
            if placement.a().vector_to(point).length() < threshold {
                return;
            }
            placement.follow(point, modifiers);
        }
        self.place_placement();
    }

    /// Whether Shift (from the centre) and Ctrl (square, circle, angle snap) do anything for
    /// the running placement's kind: Shift does nothing for a polygon or star.
    fn placement_uses(&self, tap: Tap) -> bool {
        let Some(placement) = self.placement() else {
            return false;
        };
        tap == Tap::Ctrl || !matches!(placement.kind(), PlacementKind::PolyStar)
    }

    /// Flips one tap toggle of the running, unfrozen placement; the preview follows in the same
    /// frame (`0052` criterion 32). Returns whether it flipped.
    pub fn placement_toggle_modifier(&mut self, shift: bool) -> bool {
        let tap = if shift { Tap::Shift } else { Tap::Ctrl };
        if self.placement().is_none_or(ShapePlacement::is_frozen) || !self.placement_uses(tap) {
            return false;
        }
        let toggled = &mut self.placements.toggled;
        match tap {
            Tap::Shift => toggled.shift = !toggled.shift,
            Tap::Ctrl => toggled.ctrl = !toggled.ctrl,
        }
        self.placement_modifiers(self.held);
        true
    }

    /// One key-down during a placement (`0052` criterion 32). A Shift or Ctrl (Cmd) press
    /// toggles at once, ignoring auto-repeat and a placement whose field the maker has edited
    /// (typing: the first edit freezes it); it is told apart from a
    /// chord by the next key-down: another key whose own event still reports that modifier down
    /// (Shift+letter) takes the toggle back, any other next key ends the tap. No key-up is
    /// needed. Returns whether the placement changed.
    pub fn placement_key_down(&mut self, key: &str, shift: bool, ctrl: bool, repeat: bool) -> bool {
        if self.placement().is_none() {
            self.placements.tap = None;
            return false;
        }
        if repeat {
            return false;
        }
        let pending = self.placements.tap.take();
        match key {
            "Shift" | "Control" | "Meta" => {
                let shift_key = key == "Shift";
                let changed = self.placement_toggle_modifier(shift_key);
                if changed {
                    self.placements.tap = Some(if shift_key { Tap::Shift } else { Tap::Ctrl });
                }
                changed
            }
            "Alt" => false,
            _ => match pending {
                Some(Tap::Shift) if shift => self.placement_toggle_modifier(true),
                Some(Tap::Ctrl) if ctrl => self.placement_toggle_modifier(false),
                _ => false,
            },
        }
    }

    /// Edits one field of the chip (criterion 14); `index` is its position in Tab order.
    pub fn placement_edit(&mut self, index: usize, text: &str) {
        let live = self.placement_live();
        let Some(placement) = &mut self.placements.placement else {
            return;
        };
        let Some(&field) = placement.kind().fields().get(index) else {
            return;
        };
        placement.edit(field, text, live);
        self.refresh_placement();
    }

    /// Space or the r/⌀ button (criterion 20).
    pub fn placement_toggle_diameter(&mut self) {
        if let Some(placement) = &mut self.placements.placement {
            placement.toggle_diameter();
        }
        self.refresh_placement();
    }

    /// Enter in the chip, or a second press: places the shape (criteria 6 and 21), or refuses
    /// with the first invalid field (criterion 23). Returns whether a shape was placed. A layer
    /// locked or hidden meanwhile ends the placement with the notice and writes nothing.
    pub fn place_placement(&mut self) -> bool {
        self.recording(|this| {
            let live = this.placement_live();
            let settings = this.poly_star.settings();
            let unit = this.display_unit();
            let Some(placement) = &mut this.placements.placement else {
                return false;
            };
            let Placed { shape, rotation } = match placement.result(live, settings, unit) {
                Ok(placed) => placed,
                Err(error) => {
                    placement.refuse(error);
                    return false;
                }
            };
            this.placements.placement = None;
            if this.refuse_drawing_into_layer() {
                this.placements.announcement = Some(CANCELLED_SENTENCE.to_owned());
                return false;
            }
            let id = this.document.create_primitive(shape, rotation);
            this.placements.announcement = Some(placed_sentence(&shape, unit));
            this.selection.select_single(id);
            this.set_tool(Tool::Select);
            true
        })
    }

    /// Ends the placement without writing (criterion 7). Returns whether one was running.
    pub fn cancel_placement(&mut self) -> bool {
        self.placements.press = None;
        self.placements.tap = None;
        if self.placements.placement.take().is_none() {
            return false;
        }
        self.placements.announcement = Some(CANCELLED_SENTENCE.to_owned());
        true
    }

    /// The text `field` of the running placement shows (criterion 12).
    #[must_use]
    pub fn placement_field_text(&self, field: PlacementField) -> Option<String> {
        let placement = self.placement()?;
        Some(placement.field_text(field, self.placement_live(), self.display_unit()))
    }
}

/// The placement kind of a creation tool.
const fn placement_kind(tool: Tool) -> Option<PlacementKind> {
    match tool {
        Tool::Rectangle => Some(PlacementKind::Rect),
        Tool::Ellipse => Some(PlacementKind::Ellipse),
        Tool::PolygonStar => Some(PlacementKind::PolyStar),
        Tool::Select | Tool::Pen | Tool::Node | Tool::Offset | Tool::Text | Tool::Trace => None,
    }
}

#[cfg(test)]
mod tests;
