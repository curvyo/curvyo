//! The per-character values of a text in edit mode (`specs/0079-text-layout-controls` criteria 27,
//! 29, 30; ADR 0021 §2): one value per character kept in step with the string's edits, and the
//! characters the kerning acts on.

use std::ops::Range;

use curvyo_document_core::CharValues;

/// The character index of byte `byte` of `string`.
pub(crate) fn char_index(string: &str, byte: usize) -> usize {
    string[..byte.min(string.len())].chars().count()
}

/// `chars` after the bytes `range` of `string` (the string before the edit) were replaced by
/// `inserted` characters: the replaced characters' values go, the new ones have none
/// (criterion 30).
pub(crate) fn splice(
    chars: &mut Vec<CharValues>,
    string: &str,
    range: Range<usize>,
    inserted: usize,
) {
    let start = char_index(string, range.start).min(chars.len());
    let end = char_index(string, range.end).clamp(start, chars.len());
    chars.splice(
        start..end,
        std::iter::repeat_n(CharValues::DEFAULT, inserted),
    );
}

/// The characters kerning acts on (criterion 27): those of the selection `selection` (bytes),
/// else the one after the caret; never a line break, none at the end of a line.
pub(crate) fn subject(string: &str, selection: Range<usize>) -> Vec<usize> {
    let start = char_index(string, selection.start);
    if selection.is_empty() {
        return match string[selection.start.min(string.len())..].chars().next() {
            Some(c) if c != '\n' => vec![start],
            _ => Vec::new(),
        };
    }
    string[selection]
        .chars()
        .enumerate()
        .filter(|(_, c)| *c != '\n')
        .map(|(offset, _)| start + offset)
        .collect()
}

/// `values` moved by `along_mm` and `across_mm` at Size `size_mm`, held to their ranges.
pub(crate) fn kerned(
    values: CharValues,
    along_mm: f64,
    across_mm: f64,
    size_mm: f64,
) -> CharValues {
    CharValues {
        kern_x: values.kern_x + along_mm / size_mm,
        kern_y: values.kern_y + across_mm / size_mm,
        ..values
    }
    .held()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kern(x: f64) -> CharValues {
        CharValues {
            kern_x: x,
            kern_y: 0.0,
            rotation: 0.0,
        }
    }

    #[test]
    fn splice_drops_replaced_and_adds_defaults() {
        let mut chars = vec![kern(0.1), kern(0.2), kern(0.3)];
        splice(&mut chars, "aéc", 1..3, 2);
        assert_eq!(
            chars,
            [
                kern(0.1),
                CharValues::DEFAULT,
                CharValues::DEFAULT,
                kern(0.3)
            ]
        );
    }

    #[test]
    fn the_subject_is_the_character_after_the_caret_or_the_range() {
        assert_eq!(subject("AV\nB", 1..1), [1]);
        assert_eq!(subject("AV\nB", 2..2), Vec::<usize>::new(), "end of line");
        assert_eq!(subject("AV\nB", 4..4), Vec::<usize>::new(), "end of text");
        assert_eq!(subject("AV\nB", 0..4), [0, 1, 3]);
    }

    #[test]
    fn kerned_holds_to_the_range() {
        assert_eq!(kerned(kern(0.95), 1.0, 0.0, 10.0), kern(1.0));
        assert!((kerned(kern(0.0), -0.1, 0.0, 10.0).kern_x + 0.01).abs() < 1e-12);
    }
}
