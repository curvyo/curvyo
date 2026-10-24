//! A text in edit mode (`specs/0074-text-objects` criteria 2, 3, 7 to 9, 31; ADR 0020 §5): the
//! string, caret, selection and IME composition the hidden input sink sends messages to, with
//! its own undo, written to the document once when edit mode ends.

use std::ops::Range;

use curvyo_document_core::{
    Affine, CharValues, Document, FontRef, Length, MAX_TEXT_CHARS, NodeId, Style, TextChars,
    TextEditError, TextLayoutValues, TextSnapshot,
};

use crate::text_edit_chars::{kerned, splice, subject};
use curvyo_text_core::{TextLayout, WritingMode};

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

/// The move an arrow key means in `mode` (`specs/0079-text-layout-controls` criterion 24): in a
/// column Up and Down move along it, Left and Right to the column before or after in reading
/// order (right to left: Left is the next column).
const fn along_the_lines(mode: WritingMode, key: CaretMove) -> CaretMove {
    match (mode, key) {
        (WritingMode::VerticalRl | WritingMode::VerticalLr, CaretMove::Up) => CaretMove::Left,
        (WritingMode::VerticalRl | WritingMode::VerticalLr, CaretMove::Down) => CaretMove::Right,
        (WritingMode::VerticalRl, CaretMove::Left)
        | (WritingMode::VerticalLr, CaretMove::Right) => CaretMove::Down,
        (WritingMode::VerticalRl, CaretMove::Right)
        | (WritingMode::VerticalLr, CaretMove::Left) => CaretMove::Up,
        _ => key,
    }
}

/// One message of the input sink.
#[derive(Debug, Clone, PartialEq)]
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
    /// Alt+arrows (`specs/0079-text-layout-controls` criterion 29): the kern of the character
    /// after the caret, or of each character of the selection, changes by these millimetres.
    Kern {
        /// Along the line, positive in reading direction.
        along_mm: f64,
        /// Across the line, positive up.
        across_mm: f64,
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

#[derive(Debug, Clone, PartialEq)]
struct State {
    string: String,
    /// One value per character of `string` (ADR 0021 §2).
    chars: Vec<CharValues>,
    caret: usize,
    anchor: usize,
}

/// What the last change was, so the next one of the same run joins its undo step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Run {
    /// A change of its own.
    None,
    /// Typing single characters.
    Typing,
    /// Alt+arrow kerning.
    Kerning,
}

/// One edit session.
#[derive(Debug, Clone, PartialEq)]
pub struct TextEdit {
    target: EditTarget,
    original: (String, TextChars),
    state: State,
    preedit: Option<(String, usize)>,
    undo: Vec<State>,
    redo: Vec<State>,
    /// The run the last change belongs to.
    run: Run,
    /// The text's face, size, placement and style while it is edited.
    pub font: FontRef,
    /// The text's size.
    pub size: Length,
    /// The text's alignment, spacing and character rotation (`specs/0079-text-layout-controls`):
    /// a new text starts at the defaults with the next text's alignment (criterion 33).
    pub layout: TextLayoutValues,
    /// The text's placement.
    pub placement: Affine,
    /// The text's style.
    pub style: Style,
    /// The carrier the text is on, with its resolved outline (`specs/0080-text-on-path`
    /// criterion 27): typing lays the letters out along it.
    pub on_path: Option<curvyo_document_core::TextOnPath>,
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
            original: (String::new(), TextChars::default()),
            state: State {
                string: String::new(),
                chars: Vec::new(),
                caret: 0,
                anchor: 0,
            },
            preedit: None,
            undo: Vec::new(),
            redo: Vec::new(),
            run: Run::None,
            font,
            size,
            layout: TextLayoutValues::DEFAULT,
            placement,
            style,
            on_path: None,
        }
    }

    /// A session on the existing `text` with the caret at byte `caret` (criterion 10).
    #[must_use]
    pub fn existing(text: &TextSnapshot, caret: usize) -> Self {
        let caret = floor_char(&text.string, caret);
        Self {
            target: EditTarget::Existing(text.id),
            original: (text.string.clone(), text.chars.clone()),
            state: State {
                string: text.string.clone(),
                chars: text.chars.to_dense(text.string.chars().count()),
                caret,
                anchor: caret,
            },
            preedit: None,
            undo: Vec::new(),
            redo: Vec::new(),
            run: Run::None,
            font: text.font.clone(),
            size: text.size,
            layout: text.layout,
            placement: text.placement,
            style: text.style.clone(),
            on_path: text.on_path.clone(),
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

    /// The values of the characters of [`TextEdit::string`].
    #[must_use]
    pub fn chars(&self) -> TextChars {
        TextChars::from_dense(&self.state.chars)
    }

    /// The values of the characters of the string as drawn ([`TextEdit::shown`]): the
    /// composition's characters have none.
    #[must_use]
    pub fn shown_chars(&self) -> TextChars {
        match &self.preedit {
            Some((text, _)) => {
                let mut chars = self.state.chars.clone();
                splice(
                    &mut chars,
                    &self.state.string,
                    self.selection(),
                    text.chars().count(),
                );
                TextChars::from_dense(&chars)
            }
            None => self.chars(),
        }
    }

    /// The characters kerning acts on and their values (criterion 27): the selection's, else
    /// the one after the caret; none at the end of a line.
    #[must_use]
    pub fn kern_subject(&self) -> Vec<(usize, CharValues)> {
        subject(&self.state.string, self.selection())
            .into_iter()
            .filter_map(|index| Some((index, *self.state.chars.get(index)?)))
            .collect()
    }

    /// Sets the values of single characters (a kerning field's change in edit mode, its own
    /// document step): not a step of the session's undo.
    pub fn set_chars(&mut self, values: &[(usize, CharValues)]) {
        for (index, value) in values {
            if let Some(slot) = self.state.chars.get_mut(*index) {
                *slot = value.held();
            }
        }
    }

    /// Marks the session's string and values as written to its node (a kerning field's step in
    /// edit mode): [`Self::finish`] writes what differs from them, so a later session undo that
    /// takes back earlier typing still ends with the node as shown.
    pub fn mark_written(&mut self) {
        self.original = (self.state.string.clone(), self.chars());
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
            layout: self.layout,
            chars: self.shown_chars(),
            placement: self.placement,
            style: self.style.clone(),
            outline: None,
            on_path: self.on_path.clone(),
        }
    }

    /// Applies one message. Caret moves by line need `layout`, the engine's layout of
    /// [`TextEdit::string`]; without it they do nothing.
    pub fn apply(&mut self, input: TextInput, layout: Option<&TextLayout>) -> InputOutcome {
        let before = (self.state.clone(), self.preedit.clone());
        let mut limited = false;
        let typing = matches!(&input, TextInput::Insert(text) if text.chars().count() == 1);
        let kerning = matches!(input, TextInput::Kern { .. });
        match input {
            TextInput::Insert(text) => limited = self.insert(&normalised(&text), typing),
            TextInput::Kern {
                along_mm,
                across_mm,
            } => self.kern(along_mm, across_mm),
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
        if !typing && !kerning {
            self.run = Run::None;
        }
        InputOutcome {
            changed: (self.state.clone(), self.preedit.clone()) != before,
            limited,
        }
    }

    fn record(&mut self, run: Run) {
        if run == Run::None || run != self.run {
            self.undo.push(self.state.clone());
        }
        self.redo.clear();
        self.run = run;
    }

    /// Alt+arrows (criterion 29): each subject character's kern moves, held to its range; a run
    /// of presses is one session undo step.
    fn kern(&mut self, along_mm: f64, across_mm: f64) {
        let subject = self.kern_subject();
        let size = self.size.as_mm();
        let changed: Vec<(usize, CharValues)> = subject
            .into_iter()
            .map(|(index, values)| (index, kerned(values, along_mm, across_mm, size)))
            .filter(|(index, values)| self.state.chars.get(*index) != Some(values))
            .collect();
        if changed.is_empty() {
            return;
        }
        self.record(Run::Kerning);
        self.set_chars(&changed);
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
        self.record(if typing { Run::Typing } else { Run::None });
        self.preedit = None;
        splice(
            &mut self.state.chars,
            &self.state.string,
            range.clone(),
            fitting.chars().count(),
        );
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
        self.record(Run::None);
        splice(&mut self.state.chars, &self.state.string, range.clone(), 0);
        self.state.string.replace_range(range.clone(), "");
        self.state.caret = range.start;
        self.state.anchor = range.start;
    }

    fn move_caret(&mut self, to: CaretMove, extend: bool, layout: Option<&TextLayout>) {
        let to = layout.map_or(to, |l| along_the_lines(l.writing_mode, to));
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
        let chars = self.chars();
        match self.target {
            EditTarget::New if string.is_empty() => Ok(Finished::Nothing),
            EditTarget::New => document
                .create_text_with_chars(
                    string,
                    &self.font,
                    self.size,
                    self.placement,
                    &self.layout,
                    &chars,
                )
                .map(Finished::Created),
            EditTarget::Existing(_) if (string, &chars) == (&self.original.0, &self.original.1) => {
                Ok(Finished::Nothing)
            }
            EditTarget::Existing(id) => {
                document.edit_text(id, string, &chars)?;
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

    fn kern_at(edit: &TextEdit, index: usize) -> CharValues {
        edit.chars().get(index)
    }

    /// Criteria 29 and 30: Alt+arrows kern the character after the caret (or each of the
    /// selection), a run of them is one session undo step; typed characters next to a kerned one
    /// get none, deleting it deletes its kern; the end of a line has no subject.
    #[test]
    fn ac29_ac30_kerning_in_the_session() {
        let mut edit = session();
        typed(&mut edit, "AV");
        edit.apply(
            TextInput::Move {
                to: CaretMove::Left,
                extend: false,
            },
            None,
        );
        for _ in 0..3 {
            edit.apply(
                TextInput::Kern {
                    along_mm: -0.1,
                    across_mm: 0.0,
                },
                None,
            );
        }
        assert!((kern_at(&edit, 1).kern_x + 0.03).abs() < 1e-12);
        typed(&mut edit, "x");
        assert_eq!(
            kern_at(&edit, 1),
            CharValues::DEFAULT,
            "typed before it: none"
        );
        assert!(
            (kern_at(&edit, 2).kern_x + 0.03).abs() < 1e-12,
            "moved along"
        );
        edit.apply(TextInput::Undo, None);
        edit.apply(TextInput::Undo, None);
        assert!(edit.chars().is_empty(), "the run of kerns is one step");
        edit.apply(TextInput::Redo, None);
        edit.apply(
            TextInput::Move {
                to: CaretMove::LineEnd,
                extend: false,
            },
            None,
        );
        assert_eq!(edit.kern_subject(), []);
        edit.apply(
            TextInput::Kern {
                along_mm: 1.0,
                across_mm: 0.0,
            },
            None,
        );
        edit.apply(TextInput::DeleteBackward, None);
        assert!(
            edit.chars().is_empty(),
            "deleting the character deletes its kern"
        );
        edit.apply(TextInput::SelectAll, None);
        edit.apply(
            TextInput::Kern {
                along_mm: 0.0,
                across_mm: 1.0,
            },
            None,
        );
        assert_eq!(edit.chars().entries().len(), 1);
        let document = Document::new(1);
        let Ok(Finished::Created(id)) = edit.finish(&document) else {
            panic!("created")
        };
        assert!((document.text(id).unwrap().chars.get(0).kern_y - 0.1).abs() < 1e-12);
    }
}
