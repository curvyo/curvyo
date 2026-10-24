//! The document commands of text objects (`specs/0074-text-objects`): create, edit the string,
//! change the font, change the size, change the layout values and the per-character values
//! (`specs/0079-text-layout-controls`). Each is one commit.

use loro::{LoroMap, TreeID};

use crate::affine::Affine;
use crate::document::{Document, OBJECTS_TREE};
use crate::path_codec::leaf_exists;
use crate::path_model::NodeId;
use crate::style_model::Style;
use crate::text_chars::{self, TextChars};
use crate::text_codec::{self, SHAPE_TEXT};
use crate::text_model::{
    FontKind, FontRef, MAX_TEXT_CHARS, MAX_TEXT_SIZE, MIN_TEXT_SIZE, TextLayoutValues, TextSnapshot,
};
use crate::units::Length;

/// Why a text command wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TextEditError {
    /// No live text object has this id.
    #[error("no such text")]
    NoSuchText,
    /// A new text has an empty string (criterion 8: an empty text is never created).
    #[error("a text needs at least one character")]
    Empty,
    /// The string has more than [`MAX_TEXT_CHARS`] characters (criterion 31).
    #[error("a text holds at most {MAX_TEXT_CHARS} characters")]
    TooLong,
    /// The size is outside [`MIN_TEXT_SIZE`] ..= [`MAX_TEXT_SIZE`] (criterion 12; the caller
    /// clamps and posts the notice).
    #[error("a text size is between 0.1 mm and 2000 mm")]
    SizeOutOfRange,
    /// The placement is not finite, beyond the matrix limit or not invertible.
    #[error("the placement must be a finite, invertible map")]
    InvalidPlacement,
    /// A font hash is not 64 hex digits.
    #[error("a font reference needs a 64-digit hex hash")]
    InvalidFont,
}

/// The step a layout change is (`specs/0079-text-layout-controls/adrs.md` decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextLayoutLabel {
    /// "Change text alignment".
    Alignment,
    /// "Change letter spacing".
    LetterSpacing,
    /// "Change word spacing".
    WordSpacing,
    /// "Change line spacing".
    LineSpacing,
    /// "Change character rotation".
    CharacterRotation,
    /// "Change writing mode".
    WritingMode,
    /// "Change glyph orientation".
    GlyphOrientation,
}

impl TextLayoutLabel {
    /// The commit label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Alignment => "change_text_alignment",
            Self::LetterSpacing => "change_letter_spacing",
            Self::WordSpacing => "change_word_spacing",
            Self::LineSpacing => "change_line_spacing",
            Self::CharacterRotation => "change_character_rotation",
            Self::WritingMode => "change_writing_mode",
            Self::GlyphOrientation => "change_glyph_orientation",
        }
    }
}

/// The step a change of per-character values is (`0079` criteria 27 to 29, 27b).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextCharsLabel {
    /// "Change kerning".
    Kerning,
    /// "Clear kerning" (the reset outside edit mode, criterion 28).
    ClearKerning,
    /// "Change per-character rotation" (criterion 27b).
    CharRotation,
    /// "Clear per-character rotation" (its reset outside edit mode, criterion 28).
    ClearCharRotation,
}

impl TextCharsLabel {
    /// The commit label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Kerning => "change_kerning",
            Self::ClearKerning => "clear_kerning",
            Self::CharRotation => "change_char_rotation",
            Self::ClearCharRotation => "clear_char_rotation",
        }
    }
}

fn check_string(string: &str) -> Result<(), TextEditError> {
    if string.chars().count() > MAX_TEXT_CHARS {
        return Err(TextEditError::TooLong);
    }
    Ok(())
}

fn check_size(size: Length) -> Result<(), TextEditError> {
    if (MIN_TEXT_SIZE.as_mm()..=MAX_TEXT_SIZE.as_mm()).contains(&size.as_mm()) {
        Ok(())
    } else {
        Err(TextEditError::SizeOutOfRange)
    }
}

fn check_font(font: &FontRef) -> Result<(), TextEditError> {
    if crate::text_model::is_face_hash(&font.face_hash) {
        Ok(())
    } else {
        Err(TextEditError::InvalidFont)
    }
}

/// Switches a text's stroke on for a single-line face (`specs/0076-single-line-fonts` criterion
/// 9), in the caller's commit.
fn apply_single_line_stroke(meta: &LoroMap) {
    let style = crate::style_codec::read_style(meta);
    if let Some(stroked) = crate::text_model::single_line_style(&style) {
        crate::style_codec::write_changes(meta, &style, &stroked);
    }
}

impl Document {
    /// Creates a text object in the drawing parent with the creation style and the layout values
    /// `layout` (defaults are not stored, `0079` decision 1), in **one commit** labelled
    /// `add_text` (criteria 2, 7, 14). A single-line face switches a stroke that is off on (`0076`
    /// criterion 9).
    ///
    /// # Errors
    /// [`TextEditError::Empty`], [`TextEditError::TooLong`], [`TextEditError::SizeOutOfRange`],
    /// [`TextEditError::InvalidPlacement`] or [`TextEditError::InvalidFont`]; nothing is written.
    pub fn create_text(
        &self,
        string: &str,
        font: &FontRef,
        size: Length,
        placement: Affine,
        layout: &TextLayoutValues,
    ) -> Result<NodeId, TextEditError> {
        self.create_text_with_chars(string, font, size, placement, layout, &TextChars::default())
    }

    /// [`Document::create_text`] with the per-character values `chars` (ADR 0021), in the same
    /// one commit: a new text kerned in edit mode before it existed.
    ///
    /// # Errors
    /// As [`Document::create_text`].
    pub fn create_text_with_chars(
        &self,
        string: &str,
        font: &FontRef,
        size: Length,
        placement: Affine,
        layout: &TextLayoutValues,
        chars: &TextChars,
    ) -> Result<NodeId, TextEditError> {
        if string.is_empty() {
            return Err(TextEditError::Empty);
        }
        check_string(string)?;
        check_size(size)?;
        check_font(font)?;
        if !placement.is_within_limit()
            || placement.determinant().abs() <= crate::affine::SINGULAR_DETERMINANT
        {
            return Err(TextEditError::InvalidPlacement);
        }
        let id = self.create_text_uncommitted(&TextSnapshot {
            id: NodeId::from_parts(0, 0),
            string: string.to_owned(),
            font: font.clone(),
            size,
            layout: layout.held(),
            chars: chars.clone(),
            placement,
            style: Style::default(),
            outline: None,
        });
        if font.kind == FontKind::SingleLine
            && let Ok(meta) = self.text_meta(id)
        {
            apply_single_line_stroke(&meta);
        }
        self.commit_with_label("add_text");
        Ok(id)
    }

    /// Writes a new text node with `text`'s registers and style (its `id` is ignored) in the
    /// drawing parent, without committing. The caller has checked the values.
    pub(crate) fn create_text_uncommitted(&self, text: &TextSnapshot) -> NodeId {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        // invariant: creating a node under the root or a live group cannot fail.
        #[allow(clippy::unwrap_used)]
        let tree_id = tree.create(self.drawing_parent(&tree)).unwrap();
        // invariant: the meta map of a node just created can be read.
        #[allow(clippy::unwrap_used)]
        let meta = tree.get_meta(tree_id).unwrap();
        crate::shape_codec::write_shape_tag(&meta, SHAPE_TEXT);
        crate::style_codec::write_creation_style(&meta);
        let created = crate::style_codec::read_style(&meta);
        crate::style_codec::write_changes(&meta, &created, &text.style);
        text_codec::write_text(&meta, text);
        NodeId::from_parts(tree_id.peer, tree_id.counter)
    }

    /// The text object `id`, or `None` when it is not a live, readable text.
    #[must_use]
    pub fn text(&self, id: NodeId) -> Option<TextSnapshot> {
        let meta = self.text_meta(id).ok()?;
        text_codec::read_text_snapshot(id, &meta)
    }

    /// Replaces the string of text `id` by `string` as the difference of the two (common start
    /// and end kept, the middle deleted and inserted), so a peer's concurrent edit elsewhere in the
    /// string merges per character (ADR 0009 §3), then writes the per-character values `chars`
    /// (of `string`) where they differ from the stored ones (ADR 0021 §2: a typed character never
    /// inherits a kern). One commit labelled `edit_text`; nothing when neither changed (criterion
    /// 7). An empty `string` removes the text in that same commit, so Undo brings it back with its
    /// string (criterion 8). A string longer than [`MAX_TEXT_CHARS`] is refused unless it is no
    /// longer than the stored one (a merge may have made it longer; deleting from it is allowed).
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] or [`TextEditError::TooLong`].
    pub fn edit_text(
        &self,
        id: NodeId,
        string: &str,
        chars: &TextChars,
    ) -> Result<(), TextEditError> {
        let meta = self.text_meta(id)?;
        let container = text_codec::string_container(&meta).ok_or(TextEditError::NoSuchText)?;
        let old = container.to_string();
        if string.chars().count() > old.chars().count() {
            check_string(string)?;
        }
        if string.is_empty() {
            let tree = self.loro().get_tree(OBJECTS_TREE);
            let tree_id = TreeID::new(id.peer, id.counter);
            let parents = crate::group_tree::parents_of(&tree, &[tree_id]);
            self.repair_clones_of_removed(&tree, &[tree_id]);
            tree.delete(tree_id)
                .map_err(|_| TextEditError::NoSuchText)?;
            crate::groups::remove_emptied_groups(&tree, &parents);
            self.commit_with_label("edit_text");
            return Ok(());
        }
        let edited = old != string;
        if edited && !text_codec::apply_string_diff(&container, string) {
            return Err(TextEditError::NoSuchText);
        }
        if text_chars::write_chars(&container, chars) || edited {
            self.commit_with_label("edit_text");
        }
        Ok(())
    }

    /// Sets the per-character values of each text to the ones paired with it (`0079` criteria 27,
    /// 28; ADR 0021 §6), in **one commit** labelled by `label`; only the values that differ are
    /// written. Nothing is committed when nothing changed.
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] when an id is not a live text; nothing is written.
    pub fn set_text_chars(
        &self,
        texts: &[(NodeId, TextChars)],
        label: TextCharsLabel,
    ) -> Result<(), TextEditError> {
        let ids: Vec<NodeId> = texts.iter().map(|(id, _)| *id).collect();
        let metas = self.text_metas(&ids)?;
        let containers: Vec<_> = metas
            .iter()
            .map(|meta| text_codec::string_container(meta).ok_or(TextEditError::NoSuchText))
            .collect::<Result<_, _>>()?;
        let mut wrote = false;
        for (container, (_, chars)) in containers.iter().zip(texts) {
            wrote |= text_chars::write_chars(container, chars);
        }
        if wrote {
            self.commit_with_label(label.label());
        }
        Ok(())
    }

    /// Sets the font of each text to the font paired with it (criterion 13; a face per text for
    /// the style buttons, `specs/0078-font-style-buttons/adrs.md` decision 3), in **one commit**
    /// labelled `change_font`; a text that already has its font is not rewritten. A single-line
    /// face switches a stroke that is off on in the same commit (`0076` criterion 9); switching
    /// back to an outline face changes nothing else (criterion 10, Frage 23 SL-BACK default A).
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] when an id is not a live text, or
    /// [`TextEditError::InvalidFont`]; nothing is written.
    pub fn set_text_fonts(&self, fonts: &[(NodeId, FontRef)]) -> Result<(), TextEditError> {
        for (_, font) in fonts {
            check_font(font)?;
        }
        let ids: Vec<NodeId> = fonts.iter().map(|(id, _)| *id).collect();
        let metas = self.text_metas(&ids)?;
        for (meta, (_, font)) in metas.iter().zip(fonts) {
            if text_codec::read_font(meta).as_ref() != Some(font) {
                text_codec::write_font(meta, font);
                if font.kind == FontKind::SingleLine {
                    apply_single_line_stroke(meta);
                }
            }
        }
        self.commit_with_label("change_font");
        Ok(())
    }

    /// Sets the size of every text in `ids` (criterion 13), in **one commit** labelled
    /// `change_text_size`; an equal size is not rewritten.
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] or [`TextEditError::SizeOutOfRange`]; nothing is written.
    pub fn set_text_size(&self, ids: &[NodeId], size: Length) -> Result<(), TextEditError> {
        check_size(size)?;
        let metas = self.text_metas(ids)?;
        for meta in &metas {
            if text_codec::read_size(meta) != Some(size) {
                text_codec::write_size(meta, size);
            }
        }
        self.commit_with_label("change_text_size");
        Ok(())
    }

    /// Sets the layout values and placement of each text to the ones paired with it (`0079`
    /// criteria 4, 12 to 14; `adrs.md` decision 3), in **one commit** labelled by `label`. Only
    /// the keys that differ are written, a default deletes its key, and the placement is written
    /// only when it differs (the anchor shift of an alignment change). Nothing is committed when
    /// nothing changed.
    ///
    /// # Errors
    /// [`TextEditError::NoSuchText`] when an id is not a live text, or
    /// [`TextEditError::InvalidPlacement`]; nothing is written.
    pub fn set_text_layout(
        &self,
        texts: &[(NodeId, TextLayoutValues, Affine)],
        label: TextLayoutLabel,
    ) -> Result<(), TextEditError> {
        for (_, _, placement) in texts {
            if !placement.is_within_limit()
                || placement.determinant().abs() <= crate::affine::SINGULAR_DETERMINANT
            {
                return Err(TextEditError::InvalidPlacement);
            }
        }
        let ids: Vec<NodeId> = texts.iter().map(|(id, _, _)| *id).collect();
        let metas = self.text_metas(&ids)?;
        let mut wrote = false;
        for (meta, (_, layout, placement)) in metas.iter().zip(texts) {
            wrote |= text_codec::write_layout(meta, layout);
            if text_codec::read_placement(meta) != Some(*placement) {
                text_codec::write_placement(meta, *placement);
                wrote = true;
            }
        }
        if wrote {
            self.commit_with_label(label.label());
        }
        Ok(())
    }

    fn text_metas(&self, ids: &[NodeId]) -> Result<Vec<LoroMap>, TextEditError> {
        ids.iter().map(|id| self.text_meta(*id)).collect()
    }

    fn text_meta(&self, id: NodeId) -> Result<LoroMap, TextEditError> {
        let tree = self.loro().get_tree(OBJECTS_TREE);
        let tree_id = TreeID::new(id.peer, id.counter);
        if !leaf_exists(&tree, tree_id) {
            return Err(TextEditError::NoSuchText);
        }
        let meta = tree
            .get_meta(tree_id)
            .map_err(|_| TextEditError::NoSuchText)?;
        if text_codec::is_text_meta(&meta) {
            Ok(meta)
        } else {
            Err(TextEditError::NoSuchText)
        }
    }
}
