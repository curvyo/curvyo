//! Writes the fitted texts of a Trace into the document, with the record of their guessed letters
//! (`specs/0091-trace-text-recognition` criteria 4, 5, 10 and 27).

use curvyo_document_core::NodeId;

use super::Session;
use super::trace_text::FittedImage;

/// One text Create paths wrote: its node, string, guessed positions, size bits (for the guessed
/// record) and whether it was set in the nearest face.
pub(crate) type WrittenText = (NodeId, String, Vec<usize>, u64, bool);

/// Writes the texts of `fitted` into `document`, each directly above its picture's entry of
/// `above` (the "Trace of" group, or the picture when it has no paths), black, no stroke
/// (decision 6, criterion 10). `None` when a write failed (the caller's step is then reverted).
pub(super) fn write_texts(
    document: &curvyo_document_core::Document,
    fitted: &[FittedImage],
    above: &[(NodeId, NodeId)],
) -> Option<Vec<WrittenText>> {
    use curvyo_document_core::{
        Affine, Length, MAX_TEXT_SIZE, MIN_TEXT_SIZE, MoveTarget, TextChars, TextLayoutValues,
    };
    let range = MIN_TEXT_SIZE.as_mm()..=MAX_TEXT_SIZE.as_mm();
    let style = curvyo_ui_core::trace_style();
    let mut out = Vec::new();
    for image in fitted {
        let Some(&(_, anchor)) = above.iter().find(|(id, _)| *id == image.image) else {
            continue;
        };
        for text in &image.texts {
            let run = &text.run;
            let size = run.size.get();
            if !range.contains(&size) {
                continue;
            }
            let p = run.placement.as_coeffs();
            let id = document
                .create_text_with_chars(
                    &run.text,
                    &text.font,
                    Length::from_mm(size),
                    Affine::new(p[0], p[1], p[2], p[3], p[4], p[5]),
                    &TextLayoutValues {
                        letter_spacing: run.letter_spacing.get() / size,
                        word_spacing: run.word_spacing.get() / size,
                        ..TextLayoutValues::DEFAULT
                    },
                    &TextChars::default(),
                )
                .ok()?;
            let mut own = style.clone();
            // Criterion 27: a single-line text is drawn with the traced strokes' width, unfilled.
            if let Some(width) = text.stroke {
                own.fill.enabled = false;
                own.stroke.enabled = true;
                own.stroke.color = curvyo_document_core::Color::BLACK;
                own.stroke.width = Length::from_mm(width.max(0.01));
            }
            for edit in super::recognise::style_edits(&own) {
                document.edit_style(&[id], &edit).ok()?;
            }
            document
                .move_nodes(&curvyo_document_core::MovePlan {
                    nodes: vec![id],
                    target: MoveTarget::Above(anchor),
                })
                .ok()?;
            out.push((
                id,
                run.text.clone(),
                run.guessed.clone(),
                Length::from_mm(size).as_mm().to_bits(),
                text.nearest,
            ));
        }
    }
    Some(out)
}

/// One text of the last Create paths as the host words its warning (criterion 5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedText {
    /// The string.
    pub string: String,
    /// The guessed letters: character and its 1-based position.
    pub guessed: Vec<(char, usize)>,
    /// Set in the nearest face, every letter guessed (114 B).
    pub nearest: bool,
}

impl Session {
    /// Keeps the guessed letters of the texts Create paths just wrote in the session record the
    /// Glyph table and the Text tool highlight (criterion 4, `0088` decision 6), and the report of
    /// [`Session::trace_created_texts`].
    pub(super) fn record_trace_texts(&mut self, texts: &[WrittenText]) {
        let record: Vec<(NodeId, Vec<usize>, String, u64)> = texts
            .iter()
            .filter(|t| !t.2.is_empty())
            .map(|(id, string, guessed, size, _)| (*id, guessed.clone(), string.clone(), *size))
            .collect();
        if !record.is_empty() {
            self.recognise.guessed = super::recognise::GuessedRecord {
                texts: record,
                selection: self.selection.ids().to_vec(),
            };
        }
        self.trace_text.created = texts
            .iter()
            .map(|(_, string, guessed, _, nearest)| {
                let chars: Vec<char> = string.chars().collect();
                TracedText {
                    string: string.clone(),
                    guessed: guessed
                        .iter()
                        .filter_map(|&p| chars.get(p).map(|&c| (c, p + 1)))
                        .collect(),
                    nearest: *nearest,
                }
            })
            .collect();
        self.drop_trace_fit();
    }

    /// The warning about the guessed letters of the last Create paths (criterion 5, UX notes §5),
    /// `None` when every letter is sure.
    #[must_use]
    pub fn trace_created_warning(&self) -> Option<String> {
        let texts: Vec<curvyo_ui_core::WarnedText<'_>> = self
            .trace_text
            .created
            .iter()
            .map(|t| curvyo_ui_core::WarnedText {
                string: &t.string,
                guessed: &t.guessed,
                nearest: t.nearest,
            })
            .collect();
        curvyo_ui_core::traced_text_warning(&texts)
    }

    /// The texts the last Create paths wrote (criterion 5: the host warns about guessed letters).
    #[must_use]
    pub fn trace_created_texts(&self) -> &[TracedText] {
        &self.trace_text.created
    }
}
