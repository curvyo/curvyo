//! A text in edit mode (`specs/0074-text-objects` criteria 2, 3, 7 to 9, 31; ADR 0020 §5): the
//! string, caret, selection and IME composition the hidden input sink sends messages to, with
//! its own undo, written to the document once when edit mode ends.

use std::ops::Range;

use curvyo_document_core::{
    Affine, Document, FontRef, Length, MAX_TEXT_CHARS, NodeId, Style, TextEditError, TextSnapshot,
};
use curvyo_text_core::TextLayout;

/// A tab in typed or pasted text becomes this many spaces (criterion 3; `adrs.md` decision 4).
const TAB_SPACES: usize = 4;

/// What the session edits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EditTarget {
    /// A text that does not exist yet: it is created at the first character, when edit mode ends.
    New,
    /// An existing text.
    Existing(NodeId),
}

/// Where a caret move goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaretMove {
    /// One cluster left (Left).
    Left,
    /// One cluster right (Right).
    Right,
    /// The line above, at the nearest boundary to the caret's x (Up).
    Up,
    /// The line below (Down).
    Down,
    /// The start of the line (Home).
    LineStart,
    /// The end of the line (End).
    LineEnd,
    /// The start of the word before (Ctrl+Left).
    WordLeft,
    /// The end of the word after (Ctrl+Right).
    WordRight,
}

/// One message of the input sink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextInput {
    /// Typed or pasted characters replace the selection.
    Insert(String),
    /// Backspace: the selection, else the character before the caret.
    DeleteBackward,
    /// Delete: the selection, else the cluster after the caret.
    DeleteForward,
    /// A caret move; `extend` (Shift) keeps the other end of the selection.
    Move {
        /// Where to.
        to: CaretMove,
        /// Whether the selection is extended.
        extend: bool,
    },
    /// The selection from `anchor` to `caret`, byte offsets snapped by the caller (a pointer
    /// press, drag or double-click).
    Select {
        /// The fixed end.
        anchor: usize,
        /// The moving end, where the caret is.
        caret: usize,
    },
    /// Ctrl+A: every character of this text.
    SelectAll,
    /// The IME composition in flight, its caret as a byte offset in it; empty ends it.
    Preedit {
        /// The composed text so far.
        text: String,
        /// The IME's caret in it.
        caret: usize,
    },
    /// Ctrl+Z inside the session (criterion 9).
    Undo,
    /// Ctrl+Shift+Z inside the session.
    Redo,
}

/// What a message did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputOutcome {
    /// The string, caret, selection or composition changed: draw again.
    pub changed: bool,
    /// Characters were cut off at [`MAX_TEXT_CHARS`]: show the notice (criterion 31).
    pub limited: bool,
}

/// How edit mode ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finished {
    /// Nothing written: a new text with no character, or an existing one unchanged.
    Nothing,
    /// A new text was created ("Add text").
    Created(NodeId),
    /// The existing text's string was changed ("Edit text").
    Edited(NodeId),
    /// The existing text was emptied and removed ("Edit text", criterion 8).
    Removed(NodeId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct State {
    string: String,
    caret: usize,
    anchor: usize,
}

/// One edit session.
#[derive(Debug, Clone, PartialEq)]
pub struct TextEdit {
    target: EditTarget,
    original: String,
    state: State,
    preedit: Option<(String, usize)>,
    undo: Vec<State>,
    redo: Vec<State>,
    /// Whether the last change was a run of typing, so the next typed character joins its undo
    /// step.
    typing: bool,
    /// The text's face, size, placement and style while it is edited.
    pub font: FontRef,
    /// The text's size.
    pub size: Length,
    /// The text's placement.
    pub placement: Affine,
    /// The text's style.
    pub style: Style,
}

/// Normalises typed or pasted text: CR LF and CR to LF, a tab to [`TAB_SPACES`] spaces, other
/// control characters dropped.
fn normalised(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\t' => out.extend(std::iter::repeat_n(' ', TAB_SPACES)),
            '\n' => out.push('\n'),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

impl TextEdit {
    /// A session on a new text at `placement` (criterion 2) with the face, size and style the
    /// next text gets (criterion 14).
    #[must_use]
    pub fn new_text(font: FontRef, size: Length, placement: Affine, style: Style) -> Self {
        Self {
            target: EditTarget::New,
            original: String::new(),
            state: State {
                string: String::new(),
                caret: 0,
                anchor: 0,
            },
            preedit: None,
            undo: Vec::new(),
            redo: Vec::new(),
            typing: false,
            font,
            size,
            placement,
            style,
        }
    }

    /// A session on the existing `text` with the caret at byte `caret` (criterion 10).
    #[must_use]
    pub fn existing(text: &TextSnapshot, caret: usize) -> Self {
        let caret = floor_char(&text.string, caret);
        Self {
            target: EditTarget::Existing(text.id),
            original: text.string.clone(),
            state: State {
                string: text.string.clone(),
                caret,
                anchor: caret,
            },
            preedit: None,
            undo: Vec::new(),
            redo: Vec::new(),
            typing: false,
            font: text.font.clone(),
            size: text.size,
            placement: text.placement,
            style: text.style.clone(),
        }
    }

    /// What is edited.
    #[must_use]
    pub const fn target(&self) -> EditTarget {
        self.target
    }

    /// The string as typed so far, without the composition.
    #[must_use]
    pub fn string(&self) -> &str {
        &self.state.string
    }

    /// The caret (byte offset in [`TextEdit::string`]).
    #[must_use]
    pub const fn caret(&self) -> usize {
        self.state.caret
    }

    /// The selected bytes, empty when there is only a caret.
    #[must_use]
    pub fn selection(&self) -> Range<usize> {
        self.state.caret.min(self.state.anchor)..self.state.caret.max(self.state.anchor)
    }

    /// The selected text, for Copy and Cut.
    #[must_use]
    pub fn selected_text(&self) -> &str {
        &self.state.string[self.selection()]
    }

    /// The string as drawn: the composition inserted at the caret (criterion 3, IME preedit),
    /// with the composition's bytes and its caret in it.
    #[must_use]
    pub fn shown(&self) -> (String, Option<Range<usize>>, usize) {
        match &self.preedit {
            Some((text, caret)) => {
                let at = self.selection().start;
                let mut shown = self.state.string.clone();
                shown.replace_range(self.selection(), text);
                (shown, Some(at..at + text.len()), at + caret)
            }
            None => (self.state.string.clone(), None, self.state.caret),
        }
    }

    /// The text as drawn while it is edited: the session's string, face, size, placement and
    /// style under `id` (the node's, or a placeholder for a new text).
    #[must_use]
    pub fn snapshot(&self, id: NodeId) -> TextSnapshot {
        TextSnapshot {
            id,
            string: self.shown().0,
            font: self.font.clone(),
            size: self.size,
            placement: self.placement,
            style: self.style.clone(),
            outline: None,
        }
    }

    /// Applies one message. Caret moves by line need `layout`, the engine's layout of
    /// [`TextEdit::string`]; without it they do nothing.
    pub fn apply(&mut self, input: TextInput, layout: Option<&TextLayout>) -> InputOutcome {
        let before = (self.state.clone(), self.preedit.clone());
        let mut limited = false;
        let typing = matches!(&input, TextInput::Insert(text) if text.chars().count() == 1);
        match input {
            TextInput::Insert(text) => limited = self.insert(&normalised(&text), typing),
            TextInput::DeleteBackward => self.delete(true, layout),
            TextInput::DeleteForward => self.delete(false, layout),
            TextInput::Move { to, extend } => self.move_caret(to, extend, layout),
            TextInput::Select { anchor, caret } => {
                self.state.anchor = floor_char(&self.state.string, anchor);
                self.state.caret = floor_char(&self.state.string, caret);
            }
            TextInput::SelectAll => {
                self.state.anchor = 0;
                self.state.caret = self.state.string.len();
            }
            TextInput::Preedit { text, caret } => {
                // The caret is an offset in the IME's raw text: it is carried through the same
                // normalisation as the text (a tab becomes four spaces, a CR goes).
                self.preedit = (!text.is_empty()).then(|| {
                    let shown = normalised(&text);
                    let caret = normalised(&text[..floor_char(&text, caret)]).len();
                    let caret = floor_char(&shown, caret);
                    (shown, caret)
                });
            }
            TextInput::Undo => self.step(true),
            TextInput::Redo => self.step(false),
        }
        if !typing {
            self.typing = false;
        }
        InputOutcome {
            changed: (self.state.clone(), self.preedit.clone()) != before,
            limited,
        }
    }

    fn record(&mut self, typing: bool) {
        if !(typing && self.typing) {
            self.undo.push(self.state.clone());
        }
        self.redo.clear();
        self.typing = typing;
    }

    /// Replaces the selection by `text`, cut at the character limit; true when it was cut.
    fn insert(&mut self, text: &str, typing: bool) -> bool {
        let range = self.selection();
        let kept =
            self.state.string.chars().count() - self.state.string[range.clone()].chars().count();
        let room = MAX_TEXT_CHARS.saturating_sub(kept);
        let fitting: String = text.chars().take(room).collect();
        let limited = fitting.len() < text.len();
        if fitting.is_empty() && range.is_empty() {
            return limited;
        }
        self.record(typing);
        self.preedit = None;
        self.state.string.replace_range(range.clone(), &fitting);
        self.state.caret = range.start + fitting.len();
        self.state.anchor = self.state.caret;
        limited
    }

    fn delete(&mut self, backward: bool, layout: Option<&TextLayout>) {
        let mut range = self.selection();
        if range.is_empty() {
            let caret = self.state.caret;
            range = if backward {
                previous_char(&self.state.string, caret)..caret
            } else {
                let next = layout.map_or_else(
                    || next_char(&self.state.string, caret),
                    |l| l.next_boundary(caret),
                );
                caret..next.max(caret).min(self.state.string.len())
            };
        }
        if range.is_empty() {
            return;
        }
        self.record(false);
        self.state.string.replace_range(range.clone(), "");
        self.state.caret = range.start;
        self.state.anchor = range.start;
    }

    fn move_caret(&mut self, to: CaretMove, extend: bool, layout: Option<&TextLayout>) {
        let string = &self.state.string;
        let caret = self.state.caret;
        let selection = self.selection();
        let target = match (to, layout) {
            // A plain Left or Right with a selection collapses it to that end.
            (CaretMove::Left, _) if !extend && !selection.is_empty() => selection.start,
            (CaretMove::Right, _) if !extend && !selection.is_empty() => selection.end,
            (CaretMove::Left, Some(l)) => l.previous_boundary(caret),
            (CaretMove::Left, None) => previous_char(string, caret),
            (CaretMove::Right, Some(l)) => l.next_boundary(caret),
            (CaretMove::Right, None) => next_char(string, caret),
            (CaretMove::Up | CaretMove::Down, Some(l)) => {
                let line = l.line_of(caret);
                let wanted = if to == CaretMove::Up {
                    line.checked_sub(1)
                } else {
                    Some(line + 1).filter(|&next| next < l.lines.len())
                };
                wanted.map_or(caret, |wanted| {
                    l.boundary_on_line(wanted, l.caret_point(caret).x)
                })
            }
            (CaretMove::LineStart, _) => string[..caret].rfind('\n').map_or(0, |at| at + 1),
            (CaretMove::LineEnd, _) => string[caret..]
                .find('\n')
                .map_or(string.len(), |at| caret + at),
            (CaretMove::WordLeft, _) => word_left(string, caret),
            (CaretMove::WordRight, _) => word_right(string, caret),
            (CaretMove::Up | CaretMove::Down, None) => caret,
        };
        self.state.caret = floor_char(string, target);
        if !extend {
            self.state.anchor = self.state.caret;
        }
    }

    fn step(&mut self, back: bool) {
        let (from, to) = if back {
            (&mut self.undo, &mut self.redo)
        } else {
            (&mut self.redo, &mut self.undo)
        };
        if let Some(state) = from.pop() {
            to.push(std::mem::replace(&mut self.state, state));
            self.preedit = None;
        }
    }

    /// Ends edit mode and writes the session as its one step (criteria 7, 8): a new text is
    /// created when it has a character; an existing one is changed, or removed when emptied;
    /// nothing is written when nothing changed. The composition in flight is dropped.
    ///
    /// # Errors
    /// The document's refusal (a text deleted by a peer meanwhile, an invalid placement).
    pub fn finish(&self, document: &Document) -> Result<Finished, TextEditError> {
        let string = &self.state.string;
        match self.target {
            EditTarget::New if string.is_empty() => Ok(Finished::Nothing),
            EditTarget::New => document
                .create_text(string, &self.font, self.size, self.placement)
                .map(Finished::Created),
            EditTarget::Existing(_) if *string == self.original => Ok(Finished::Nothing),
            EditTarget::Existing(id) => {
                document.edit_text_string(id, string)?;
                Ok(if string.is_empty() {
                    Finished::Removed(id)
                } else {
                    Finished::Edited(id)
                })
            }
        }
    }
}

/// `at` moved back to a character boundary of `string`.
fn floor_char(string: &str, at: usize) -> usize {
    let mut at = at.min(string.len());
    while !string.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn previous_char(string: &str, at: usize) -> usize {
    string[..at]
        .char_indices()
        .next_back()
        .map_or(0, |(i, _)| i)
}

fn next_char(string: &str, at: usize) -> usize {
    string[at..]
        .chars()
        .next()
        .map_or(string.len(), |c| at + c.len_utf8())
}

fn word_left(string: &str, at: usize) -> usize {
    let before = &string[..at];
    let trimmed = before.trim_end_matches(|c: char| !c.is_alphanumeric());
    trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_alphanumeric())
        .map_or(0, |(i, c)| i + c.len_utf8())
}

fn word_right(string: &str, at: usize) -> usize {
    let after = &string[at..];
    let skipped = after.len()
        - after
            .trim_start_matches(|c: char| !c.is_alphanumeric())
            .len();
    let rest = &after[skipped..];
    let word = rest
        .char_indices()
        .find(|(_, c)| !c.is_alphanumeric())
        .map_or(rest.len(), |(i, _)| i);
    at + skipped + word
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> FontRef {
        FontRef {
            family: "Source Sans 3".to_owned(),
            style: "Regular".to_owned(),
            face_hash: "a".repeat(64),
            face_index: 0,
            kind: curvyo_document_core::FontKind::Outline,
        }
    }

    fn session() -> TextEdit {
        TextEdit::new_text(
            font(),
            Length::from_mm(10.0),
            Affine::IDENTITY,
            Style::default(),
        )
    }

    fn typed(edit: &mut TextEdit, text: &str) {
        for c in text.chars() {
            edit.apply(TextInput::Insert(c.to_string()), None);
        }
    }

    /// Criterion 3: typing, Enter, Backspace, Delete, moves with Shift, Ctrl+A, paste with tabs.
    #[test]
    fn ac03_typing_and_moving() {
        let mut edit = session();
        typed(&mut edit, "Hello");
        edit.apply(TextInput::Insert("\r\n".to_owned()), None);
        typed(&mut edit, "wörld");
        assert_eq!(edit.string(), "Hello\nwörld");
        edit.apply(TextInput::DeleteBackward, None);
        edit.apply(
            TextInput::Move {
                to: CaretMove::LineStart,
                extend: true,
            },
            None,
        );
        assert_eq!(edit.selected_text(), "wörl");
        edit.apply(TextInput::Insert("a\tb".to_owned()), None);
        assert_eq!(edit.string(), "Hello\na    b");
        edit.apply(
            TextInput::Move {
                to: CaretMove::WordLeft,
                extend: false,
            },
            None,
        );
        edit.apply(TextInput::DeleteForward, None);
        assert_eq!(edit.string(), "Hello\na    ");
        edit.apply(TextInput::SelectAll, None);
        assert_eq!(edit.selected_text(), "Hello\na    ");
    }

    /// Criterion 9: undo and redo inside the session; a run of typing is one step.
    #[test]
    fn ac09_session_undo() {
        let mut edit = session();
        typed(&mut edit, "abc");
        edit.apply(TextInput::DeleteBackward, None);
        typed(&mut edit, "x");
        edit.apply(TextInput::Undo, None);
        assert_eq!(edit.string(), "ab");
        edit.apply(TextInput::Undo, None);
        assert_eq!(edit.string(), "abc");
        edit.apply(TextInput::Undo, None);
        assert_eq!(edit.string(), "");
        edit.apply(TextInput::Redo, None);
        assert_eq!(edit.string(), "abc");
    }

    /// Criterion 31: typing and pasting stop at the limit and say so.
    #[test]
    fn ac31_the_limit() {
        let mut edit = session();
        let outcome = edit.apply(TextInput::Insert("y".repeat(MAX_TEXT_CHARS + 5)), None);
        assert!(outcome.limited);
        assert_eq!(edit.string().chars().count(), MAX_TEXT_CHARS);
        assert!(edit.apply(TextInput::Insert("z".to_owned()), None).limited);
        assert_eq!(MAX_TEXT_CHARS, 10_000);
    }

    /// Criteria 2, 7 and 8: nothing is written until the first character; the session is one step;
    /// an emptied text is removed in that one step.
    #[test]
    fn ac02_ac07_ac08_one_step_at_the_end() {
        let document = Document::new(1);
        assert_eq!(session().finish(&document), Ok(Finished::Nothing));
        assert_eq!(document.own_steps().len(), 0);
        let mut edit = session();
        typed(&mut edit, "Name");
        let Ok(Finished::Created(id)) = edit.finish(&document) else {
            panic!("created")
        };
        assert_eq!(document.own_steps().len(), 1);
        let text = document.text(id).unwrap();
        let mut again = TextEdit::existing(&text, 4);
        again.apply(TextInput::SelectAll, None);
        again.apply(TextInput::DeleteBackward, None);
        assert_eq!(again.finish(&document), Ok(Finished::Removed(id)));
        assert_eq!(document.own_steps().len(), 2);
        assert!(document.text(id).is_none());
    }

    /// Criterion 3: the IME composition is drawn at the caret and not part of the string.
    #[test]
    fn ac03_preedit() {
        let mut edit = session();
        typed(&mut edit, "ab");
        edit.apply(
            TextInput::Preedit {
                text: "ü".to_owned(),
                caret: 2,
            },
            None,
        );
        assert_eq!(edit.shown(), ("abü".to_owned(), Some(2..4), 4));
        // A tab before the IME caret moves it by the four spaces it becomes.
        edit.apply(
            TextInput::Preedit {
                text: "\tx".to_owned(),
                caret: 1,
            },
            None,
        );
        assert_eq!(edit.shown(), ("ab    x".to_owned(), Some(2..7), 6));
        assert_eq!(edit.string(), "ab");
        edit.apply(TextInput::Insert("ü".to_owned()), None);
        assert_eq!(edit.shown(), ("abü".to_owned(), None, 4));
    }
}
