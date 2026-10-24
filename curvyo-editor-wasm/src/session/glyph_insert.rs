//! Inserts a character from the glyph table (`specs/0082-glyph-table` criteria 12 to 14,
//! `adrs.md` decision 4) and names the face the table shows (the subject face, spec "Terms").

use curvyo_document_core::{FontRef, Point};
use curvyo_text_core::glyph_cell;
use curvyo_ui_core::{FaceMatch, TextInput};

use super::{Session, Tool};

/// What an insert did, for the host's focus and notice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GlyphInsertOutcome {
    /// A text is in edit mode after it: the host hands the keyboard to the sink.
    pub editing: bool,
    /// The character limit cut it (the notice of `0074` criterion 31).
    pub limited: bool,
}

impl Session {
    /// The face the glyph table shows: the edited text's, else the one selected text's, else
    /// the next text's (spec "Terms", Frage 39 A).
    #[must_use]
    pub fn glyph_subject(&self) -> Option<FontRef> {
        if let Some(edit) = self.text.edit.as_ref() {
            return Some(edit.font.clone());
        }
        if let [id] = self.selection.ids()
            && let Some(text) = self.document.text(*id)
        {
            return Some(text.font);
        }
        self.next_text_font()
    }

    /// The glyph table's subject face as JSON `{family, style, missing, index}` (`index`: the
    /// listed face it resolves to, `null` when missing), or `null` before any font is listed.
    #[must_use]
    pub fn glyph_subject_json(&self) -> String {
        let Some(font) = self.glyph_subject() else {
            return "null".to_owned();
        };
        let index = match curvyo_ui_core::match_face(&font, self.fonts.faces()) {
            FaceMatch::Face(key) => self.fonts.faces().iter().position(|f| f.key == Some(key)),
            FaceMatch::NeedsLoad(index) => Some(index),
            FaceMatch::Missing => None,
        };
        serde_json::json!({
            "family": font.family,
            "style": font.style,
            "missing": index.is_none(),
            "index": index,
        })
        .to_string()
    }

    /// A click on a cell (or Enter, Space, Insert): `ch` (one character) at the caret of the
    /// edited text, as if typed (criterion 12); with one text selected, that text in edit mode
    /// with the caret at its end, then the same (criterion 13); otherwise a new text with `ch`
    /// in the next text's face and size, centred on the visible drawing, in edit mode (criterion
    /// 14, one step "Add text" when edit mode ends).
    pub fn insert_from_glyph_table(&mut self, ch: &str) -> GlyphInsertOutcome {
        let mut chars = ch.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            return GlyphInsertOutcome::default();
        };
        if c.is_control() {
            return GlyphInsertOutcome::default();
        }
        if self.text.edit.is_none() {
            if let [id] = self.selection.ids()
                && self.document.text(*id).is_some()
            {
                let id = *id;
                self.edit_text_at(id, None);
            } else {
                let point = self.glyph_origin(c);
                self.tool = Tool::Text;
                self.start_new_text(point);
            }
        }
        let limited = self.text_input(TextInput::Insert(ch.to_owned())).limited;
        GlyphInsertOutcome {
            editing: self.text_editing(),
            limited,
        }
    }

    /// Where a new text's baseline starts so that `c` in the next text's face and size is
    /// centred on the visible part of the drawing: half its advance left, half its cap height
    /// down (y points down).
    fn glyph_origin(&self, c: char) -> Point {
        let centre = self.viewport_centre();
        let size = self.next_text_size().as_mm();
        let advance = self
            .next_text_font()
            .and_then(|font| match self.fonts.resolve(&font) {
                FaceMatch::Face(key) => Some(key),
                FaceMatch::Missing => self.fonts.fallback(font.kind).map(|(key, _)| key),
                FaceMatch::NeedsLoad(_) => None,
            })
            .and_then(|key| self.fonts.face(&key))
            .and_then(|face| glyph_cell(face, c))
            .map_or(0.0, |cell| cell.advance);
        Point::new(centre.x - advance * size / 2.0, centre.y + size / 2.0)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use curvyo_document_core::Point;
    use curvyo_ui_core::TextInput;

    use super::super::Tool;
    use super::super::text_tests::{labels, load_wanted, session, texts, with_text};

    /// Criterion 12: in edit mode the character goes in at the caret, replacing the selection,
    /// as typed; the face is unchanged; nothing is written until edit mode ends.
    #[test]
    fn ac12_inserts_at_the_caret_replacing_the_selection() {
        let mut session = with_text("Name");
        let id = texts(&session)[0].id;
        session.edit_text_at(id, None);
        session.text_input(TextInput::SelectAll);
        let font = session.glyph_subject().unwrap();
        let outcome = session.insert_from_glyph_table("♥");
        assert!(outcome.editing);
        assert!(!outcome.limited);
        assert_eq!(session.text_sink_state().unwrap().0, "♥");
        assert_eq!(session.glyph_subject().unwrap(), font);
        assert_eq!(
            labels(&session),
            ["add_text"],
            "nothing written while editing"
        );
        session.escape();
        assert_eq!(labels(&session), ["add_text", "edit_text"]);
        assert_eq!(texts(&session)[0].string, "♥");
    }

    /// Criterion 13: one selected text enters edit mode with the caret at its end.
    #[test]
    fn ac13_one_selected_text_is_edited_at_its_end() {
        let mut session = with_text("Name\nLine");
        session.set_tool(Tool::Select);
        assert!(!session.text_editing());
        assert!(session.insert_from_glyph_table("ä").editing);
        assert_eq!(session.tool(), Tool::Text);
        assert_eq!(
            session.text_sink_state().unwrap(),
            ("Name\nLineä".to_owned(), 10, 10)
        );
        session.insert_from_glyph_table("ü");
        session.escape();
        assert_eq!(texts(&session)[0].string, "Name\nLineäü");
        assert_eq!(labels(&session), ["add_text", "edit_text"]);
    }

    /// Criterion 14: otherwise a new text in the next text's face and size, centred on the
    /// visible drawing, in edit mode; one step "Add text".
    #[test]
    fn ac14_a_new_text_is_centred_on_the_visible_drawing() {
        let mut session = session();
        session.resize_viewport(800.0, 600.0);
        session.cover_right(200.0);
        let centre = session.viewport_centre();
        let outcome = session.insert_from_glyph_table("H");
        assert!(outcome.editing);
        assert_eq!(session.tool(), Tool::Text);
        session.escape();
        load_wanted(&mut session);
        assert_eq!(labels(&session), ["add_text"]);
        let text = &texts(&session)[0];
        assert_eq!(text.string, "H");
        assert_eq!(text.size.as_mm(), 10.0);
        // Baseline half the cap height (the size) below the visible centre, the H's advance
        // centred on it.
        assert!((text.placement.f - (centre.y + 5.0)).abs() < 1e-9);
        let half = centre.x - text.placement.e;
        assert!(half > 2.0 && half < 5.0, "{half}");
        assert!(
            centre.x < session.view().screen_to_document(400.0, 0.0).x - 1.0,
            "the centre of the part left of the pane, not of the whole canvas"
        );
    }

    /// Criterion 14: several texts selected is "otherwise" too; the next-text face is the subject.
    #[test]
    fn ac14_several_texts_selected_make_a_new_text() {
        let mut session = with_text("One");
        session.set_tool(Tool::Text);
        session.pointer_down(Point::new(10.0, 60.0), false);
        session.pointer_up(Point::new(10.0, 60.0), false, false);
        session.text_input(TextInput::Insert("Two".into()));
        session.escape();
        session.set_tool(Tool::Select);
        let ids: Vec<_> = texts(&session).iter().map(|t| t.id).collect();
        session.selection.set(&ids);
        assert_eq!(session.glyph_subject(), session.next_text_font());
        session.insert_from_glyph_table("A");
        session.escape();
        assert_eq!(texts(&session).len(), 3);
    }

    /// Only one character that is not a control character is inserted.
    #[test]
    fn anything_but_one_character_is_ignored() {
        let mut session = session();
        assert!(!session.insert_from_glyph_table("ab").editing);
        assert!(!session.insert_from_glyph_table("").editing);
        assert!(!session.insert_from_glyph_table("\u{7}").editing);
        assert!(!session.text_editing());
    }

    /// The subject face's JSON names the listed face it resolves to.
    #[test]
    fn the_subject_names_its_listed_face() {
        let session = with_text("Name");
        let subject: serde_json::Value =
            serde_json::from_str(&session.glyph_subject_json()).unwrap();
        assert_eq!(subject["family"], "Source Sans 3");
        assert_eq!(subject["missing"], false);
        assert!(subject["index"].as_u64().is_some());
    }
}
