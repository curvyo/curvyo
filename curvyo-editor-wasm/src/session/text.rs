//! `Session`'s glue for text edit mode (`specs/0074-text-objects` criteria 2 to 10): entering and
//! leaving it, the input sink's messages, and what the canvas draws for the edited text. The
//! rules live in `curvyo-ui-core` (`TextEdit`, `edit_geometry`), the engine in `curvyo-text-core`;
//! the Text bar and Fonts tab are `text_bar.rs`, Object to path `text_path.rs`.

use std::borrow::Cow;

use curvyo_document_core::{
    Affine, FontRef, Length, NodeId, ObjectSnapshot, Point, TextSnapshot, Vec2, ViewTransform,
};
use curvyo_render_core::{DrawList, TextEditOverlay, build_text_edit_overlay};
use curvyo_text_core::{Mm, TextLayout};
use curvyo_ui_core::{
    FaceMatch, Finished, InputOutcome, TextEdit, TextInput, edit_geometry, line_box_contains,
};

use super::{Session, Tool};

/// The size of the very first text (criterion 14).
const FIRST_TEXT_SIZE_MM: f64 = 10.0;

/// The id a new text is drawn under until it exists.
pub(super) const NEW_TEXT_ID: NodeId = curvyo_document_core::UNCREATED_TEXT_ID;

/// The Text tool's session state.
#[derive(Debug, Default)]
pub(super) struct TextState {
    /// Edit mode.
    pub(super) edit: Option<TextEdit>,
    /// The face for the next new text, once set (criteria 13, 14).
    pub(super) next_font: Option<FontRef>,
    /// The size for the next new text, once set.
    pub(super) next_size: Option<Length>,
    /// A press inside the edited text selects while the button is held: its fixed end.
    pub(super) drag_anchor: Option<usize>,
    /// A pick or Style press whose faces are loading: applied when they arrive.
    pub(super) pending_pick: Option<super::text_style::PendingPick>,
    /// The Size field's live value while an arrow key is held (UX notes §3), drawn, not
    /// written.
    pub(super) size_preview: Option<Length>,
    /// The notice of a typed or pasted text cut at the limit, until the host takes it.
    pub(super) limited: bool,
    /// The caret's blink phase is off (the host blinks it, UX notes §2).
    pub(super) caret_hidden: bool,
    /// Edit mode ended and its text could not be written, until the host takes the notice.
    pub(super) write_failed: bool,
}

impl Session {
    /// The caret's blink phase, set by the host: on while typing and for 500 ms after, then
    /// 530 ms on and off; always on with reduced motion; off without window focus.
    pub fn set_text_caret_on(&mut self, on: bool) {
        self.text.caret_hidden = !on;
    }

    pub(super) const fn text_caret_on(&self) -> bool {
        !self.text.caret_hidden
    }

    /// Whether a text is in edit mode.
    #[must_use]
    pub fn text_editing(&self) -> bool {
        self.text.edit.is_some()
    }

    /// The face of the next new text: the last one set, else the included sans Regular.
    pub(super) fn next_text_font(&self) -> Option<FontRef> {
        self.text.next_font.clone().or_else(|| {
            self.fonts
                .fallback(curvyo_document_core::FontKind::Outline)
                .map(|(_, font)| font)
        })
    }

    pub(super) fn next_text_size(&self) -> Length {
        self.text
            .next_size
            .unwrap_or(Length::from_mm(FIRST_TEXT_SIZE_MM))
    }

    /// The engine's layout of `string` in `font` at `size` (the substitute for a missing face);
    /// `None` while the face loads.
    pub(super) fn layout_of(
        &self,
        font: &FontRef,
        size: Length,
        string: &str,
    ) -> Option<TextLayout> {
        let key = match self.fonts.resolve(font) {
            FaceMatch::Face(key) => key,
            FaceMatch::Missing => self.fonts.fallback(font.kind)?.0,
            FaceMatch::NeedsLoad(_) => return None,
        };
        curvyo_text_core::layout(self.fonts.face(&key)?, string, Mm(size.as_mm())).ok()
    }

    /// The topmost pickable text whose line box holds `point` (criterion 25).
    pub(super) fn text_at(&self, point: Point) -> Option<TextSnapshot> {
        let tolerance = self.object_tolerance().as_mm();
        self.pickable_objects()
            .iter()
            .rev()
            .find_map(|object| match object {
                ObjectSnapshot::Text(text) if line_box_contains(text, point, tolerance) => {
                    Some((**text).clone())
                }
                _ => None,
            })
    }

    /// The caret position of `text` nearest to the document point `point`.
    fn caret_near(&self, text: &TextSnapshot, point: Point) -> usize {
        let Some(inverse) = text.placement.inverse() else {
            return text.string.len();
        };
        let local = inverse.apply_point(point);
        self.layout_of(&text.font, text.size, &text.string)
            .map_or(text.string.len(), |layout| {
                layout.nearest_boundary(kurbo_point(local))
            })
    }

    /// Starts edit mode on the text `id` in the Text tool: the caret nearest to `point`, or at the
    /// end of the string without one (criterion 10, the Enter entry).
    pub(super) fn edit_text_at(&mut self, id: NodeId, point: Option<Point>) {
        self.finish_text_edit_inner();
        let Some(text) = self.document.text(id) else {
            return;
        };
        let caret = point.map_or(text.string.len(), |p| self.caret_near(&text, p));
        self.tool = Tool::Text;
        self.selection.select_single(id);
        self.text.edit = Some(TextEdit::existing(&text, caret));
    }

    /// Starts edit mode on a new text whose first baseline starts at `point` (criterion 2).
    fn start_new_text(&mut self, point: Point) {
        let Some(font) = self.next_text_font() else {
            return;
        };
        self.selection.clear();
        self.text.edit = Some(TextEdit::new_text(
            font,
            self.next_text_size(),
            Affine::translation(Vec2::new(point.x, point.y)),
            curvyo_document_core::Style::default(),
        ));
    }

    /// Ends edit mode and writes its one step (criteria 7, 8). The written text is selected.
    pub(super) fn finish_text_edit_inner(&mut self) -> bool {
        let Some(edit) = self.text.edit.take() else {
            return false;
        };
        self.text.drag_anchor = None;
        match edit.finish(&self.document) {
            Ok(Finished::Created(id) | Finished::Edited(id)) => self.selection.select_single(id),
            Ok(Finished::Nothing) => {
                if let curvyo_ui_core::EditTarget::Existing(id) = edit.target() {
                    self.selection.select_single(id);
                }
            }
            Ok(Finished::Removed(_)) => self.selection.clear(),
            // The node went meanwhile (a peer removed it): what was typed is kept as a new text,
            // and a write that still fails is said, never dropped silently.
            Err(_) => {
                let kept = (!edit.string().is_empty())
                    .then(|| {
                        self.document
                            .create_text(edit.string(), &edit.font, edit.size, edit.placement)
                            .ok()
                    })
                    .flatten();
                if let Some(id) = kept {
                    self.selection.select_single(id);
                } else {
                    self.selection.clear();
                    self.text.write_failed = !edit.string().is_empty();
                }
            }
        }
        true
    }

    /// Takes the notice that edit mode ended without its text being written.
    pub fn take_text_write_failed(&mut self) -> bool {
        std::mem::take(&mut self.text.write_failed)
    }

    /// Ends edit mode (Escape, a click outside, a tool switch): one step "Add text" or "Edit
    /// text", or none. Returns whether edit mode was on.
    pub fn finish_text_edit(&mut self) -> bool {
        self.recording(Self::finish_text_edit_inner)
    }

    /// Writes the session as its step and edit mode goes on (a Save in edit mode, UX notes §2).
    pub fn checkpoint_text_edit(&mut self) {
        self.recording(|this| {
            let Some(caret) = this.text.edit.as_ref().map(TextEdit::caret) else {
                return;
            };
            this.finish_text_edit_inner();
            if let Some(&id) = this.selection.ids().first()
                && let Some(text) = this.document.text(id)
            {
                this.text.edit = Some(TextEdit::existing(&text, caret));
            }
        });
    }

    /// A press with the Text tool: inside the edited text it places the caret; elsewhere it ends
    /// edit mode and edits the text under the pointer, or starts a new one there (criteria 2, 7,
    /// 10). A drag acts as a click at the press point.
    pub(super) fn text_pointer_down(&mut self, point: Point, shift: bool) {
        if let Some(caret) = self.caret_in_edited(point) {
            let input = if shift {
                let anchor = self
                    .text
                    .edit
                    .as_ref()
                    .map_or(caret, |e| e.selection().start);
                TextInput::Select { anchor, caret }
            } else {
                TextInput::Select {
                    anchor: caret,
                    caret,
                }
            };
            self.text_input(input);
            self.text.drag_anchor = self
                .text
                .edit
                .as_ref()
                .map(|e| if shift { e.selection().start } else { caret });
            return;
        }
        self.finish_text_edit_inner();
        if let Some(text) = self.text_at(point) {
            self.edit_text_at(text.id, Some(point));
        } else {
            self.start_new_text(point);
        }
    }

    /// The caret position in the edited text nearest to `point` when `point` is inside its line
    /// box.
    fn caret_in_edited(&self, point: Point) -> Option<usize> {
        let edit = self.text.edit.as_ref()?;
        let snapshot = self.laid_out_edit()?;
        let tolerance = self.object_tolerance().as_mm();
        if !line_box_contains(&snapshot, point, tolerance) {
            return None;
        }
        let local = edit.placement.inverse()?.apply_point(point);
        self.layout_of(&edit.font, edit.size, edit.string())
            .map(|layout| layout.nearest_boundary(kurbo_point(local)))
    }

    /// The pointer moved with the Text tool: a held press in the edited text extends the
    /// selection; otherwise the text under the pointer shows its hover box.
    pub(super) fn text_pointer_move(&mut self, point: Point) {
        if self.button_down
            && let Some(anchor) = self.text.drag_anchor
            && let Some(edit) = self.text.edit.as_ref()
        {
            let local = edit.placement.inverse().map(|i| i.apply_point(point));
            let caret = local.and_then(|local| {
                self.layout_of(&edit.font, edit.size, edit.string())
                    .map(|l| l.nearest_boundary(kurbo_point(local)))
            });
            if let Some(caret) = caret {
                self.text_input(TextInput::Select { anchor, caret });
            }
            return;
        }
        let editing = self.text.edit.as_ref().and_then(edit_id);
        self.hovered_object = self
            .text_at(point)
            .map(|text| text.id)
            .filter(|id| Some(*id) != editing);
    }

    /// The release ends a selecting drag.
    pub(super) fn text_pointer_up(&mut self) {
        self.text.drag_anchor = None;
    }

    /// A double-click in the edited text selects the word under it (UX notes §2).
    pub(super) fn text_double_click(&mut self, point: Point) {
        let Some(caret) = self.caret_in_edited(point) else {
            return;
        };
        let Some(string) = self.text.edit.as_ref().map(|e| e.string().to_owned()) else {
            return;
        };
        let is_word = |c: char| c.is_alphanumeric();
        let start = string[..caret]
            .char_indices()
            .rev()
            .find(|(_, c)| !is_word(*c))
            .map_or(0, |(i, c)| i + c.len_utf8());
        let end = string[caret..]
            .char_indices()
            .find(|(_, c)| !is_word(*c))
            .map_or(string.len(), |(i, _)| caret + i);
        self.text_input(TextInput::Select {
            anchor: start,
            caret: end,
        });
    }

    /// Enter on the canvas (UX notes §2, keyboard entry): one selected text opens for editing
    /// with the caret at its end; in the Text tool with nothing selected a new text starts at the
    /// pointer, or the viewport centre when the pointer is not over the canvas. Returns whether
    /// it acted.
    pub(super) fn text_enter(&mut self) -> bool {
        if let [id] = self.selection.ids()
            && self.document.text(*id).is_some()
        {
            let id = *id;
            self.edit_text_at(id, None);
            return true;
        }
        if self.tool == Tool::Text && self.selection.is_empty() {
            let point = self
                .pointer_position
                .unwrap_or_else(|| self.viewport_centre());
            self.start_new_text(point);
            return self.text.edit.is_some();
        }
        false
    }

    /// One message of the input sink (criteria 3, 9, 31). Nothing is written to the document.
    pub fn text_input(&mut self, input: TextInput) -> InputOutcome {
        let layout = self
            .text
            .edit
            .as_ref()
            .and_then(|edit| self.layout_of(&edit.font, edit.size, edit.string()));
        let Some(edit) = self.text.edit.as_mut() else {
            return InputOutcome::default();
        };
        let outcome = edit.apply(input, layout.as_ref());
        if outcome.limited {
            self.text.limited = true;
        }
        outcome
    }

    /// Takes the notice "A text holds at most 10000 characters." (criterion 31).
    pub fn take_text_limit_notice(&mut self) -> bool {
        std::mem::take(&mut self.text.limited)
    }

    /// The selected characters, for Copy.
    #[must_use]
    pub fn text_selected(&self) -> String {
        self.text
            .edit
            .as_ref()
            .map(|edit| edit.selected_text().to_owned())
            .unwrap_or_default()
    }

    /// The edited string and its selection as Unicode scalar positions, which the hidden
    /// textarea mirrors for screen readers and IMEs (UX notes §2).
    #[must_use]
    pub fn text_sink_state(&self) -> Option<(String, usize, usize)> {
        let edit = self.text.edit.as_ref()?;
        let string = edit.string();
        let chars = |byte: usize| string[..byte].chars().count();
        let selection = edit.selection();
        Some((
            string.to_owned(),
            chars(selection.start),
            chars(selection.end),
        ))
    }

    /// The edited text as drawn now: the session's string in its face, laid out.
    pub(super) fn laid_out_edit(&self) -> Option<TextSnapshot> {
        let edit = self.text.edit.as_ref()?;
        let mut snapshot = edit.snapshot(edit_id(edit).unwrap_or(NEW_TEXT_ID));
        if let Some(size) = self.text.size_preview {
            snapshot.size = size;
        }
        snapshot.outline = self.fonts.lay_out(&snapshot).map(std::sync::Arc::new);
        Some(snapshot)
    }

    /// `objects` with the edited text drawn as the session has it (criterion 3: each character
    /// in the next frame) and the Size field's live value applied.
    pub(super) fn with_text_edit<'a>(
        &self,
        objects: Cow<'a, [ObjectSnapshot]>,
    ) -> Cow<'a, [ObjectSnapshot]> {
        let edited = self.laid_out_edit();
        let preview = self.text.size_preview;
        if edited.is_none() && preview.is_none() {
            return objects;
        }
        let mut objects = objects.into_owned();
        if let Some(size) = preview {
            for object in &mut objects {
                if let ObjectSnapshot::Text(text) = object
                    && self.selection.ids().contains(&text.id)
                {
                    let mut resized = (**text).clone();
                    resized.size = size;
                    resized.outline = self.fonts.lay_out(&resized).map(std::sync::Arc::new);
                    **text = resized;
                }
            }
        }
        if let Some(edited) = edited {
            match objects.iter_mut().find(|o| o.id() == edited.id) {
                Some(slot) => *slot = ObjectSnapshot::Text(Box::new(edited)),
                None => objects.push(ObjectSnapshot::Text(Box::new(edited))),
            }
        }
        Cow::Owned(objects)
    }

    /// The overlay of the text in edit mode: line box, selection, composition underline and the
    /// caret (`caret_on`: the blink phase, held by the host).
    pub(super) fn text_overlay_list(&self, view: ViewTransform, caret_on: bool) -> DrawList {
        let Some(edit) = self.text.edit.as_ref() else {
            return DrawList::default();
        };
        let (shown, preedit, caret) = edit.shown();
        let Some(layout) = self.layout_of(&edit.font, edit.size, &shown) else {
            return DrawList::default();
        };
        let selection = if preedit.is_some() {
            caret..caret
        } else {
            edit.selection()
        };
        let geometry = edit_geometry(&layout, edit.placement, caret, &selection, preedit.as_ref());
        let id = edit_id(edit).unwrap_or(NEW_TEXT_ID);
        build_text_edit_overlay(
            view,
            &TextEditOverlay {
                id: Some(id),
                line_box: (!shown.is_empty()).then_some(geometry.line_box),
                caret: geometry.caret.filter(|_| caret_on),
                selection: geometry.selection,
                preedit: geometry.preedit,
                device_pixel_ratio: self.device_pixel_ratio,
            },
        )
    }

    /// The caret's screen point, where the hidden textarea sits so the IME's candidate window
    /// opens next to it (ADR 0020 §5).
    #[must_use]
    pub fn text_caret_screen_point(&self) -> Option<(f64, f64)> {
        let edit = self.text.edit.as_ref()?;
        let (shown, preedit, caret) = edit.shown();
        let layout = self.layout_of(&edit.font, edit.size, &shown)?;
        let geometry = edit_geometry(
            &layout,
            edit.placement,
            caret,
            &(caret..caret),
            preedit.as_ref(),
        );
        let [_, bottom] = geometry.caret?;
        Some(self.view().document_to_screen(bottom))
    }

    /// The point the viewport is centred on, in the document.
    pub(super) fn viewport_centre(&self) -> Point {
        let (width, height) = self.viewport.canvas_size();
        self.view().screen_to_document(width / 2.0, height / 2.0)
    }
}

/// The node of an edited existing text.
pub(super) fn edit_id(edit: &TextEdit) -> Option<NodeId> {
    match edit.target() {
        curvyo_ui_core::EditTarget::Existing(id) => Some(id),
        curvyo_ui_core::EditTarget::New => None,
    }
}

fn kurbo_point(point: Point) -> kurbo::Point {
    kurbo::Point::new(point.x, point.y)
}
