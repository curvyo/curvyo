//! The per-character offsets of a layout (`specs/0079-text-layout-controls` criteria 27, 27b, ADR
//! 0021 §6): what the caller asks by character index, looked up by the byte of a cluster.

use crate::units::Mm;

/// An offset of one character, in line space: `along` is extra advance before it, `across` a
/// shift across the line, positive up (to the right of a column); both move the rest of the line
/// too (as SVG `dx`, `dy`). `rotation_rad` turns that character alone, on top of the text's turn.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct CharOffset {
    /// The character's index in the laid-out string (Unicode scalar values).
    pub index: usize,
    /// Extra advance before the character.
    pub along: Mm,
    /// Shift of the character and the rest of its line across the line, positive up.
    pub across: Mm,
    /// The character's own turn about its cluster's centre, clockwise on screen.
    pub rotation_rad: f64,
}

/// The along, across and rotation offsets of one cluster.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct Offset {
    pub(crate) along: f64,
    pub(crate) across: f64,
    pub(crate) rotation_rad: f64,
}

/// The offsets keyed by the byte of their character in the string, sorted by byte.
#[derive(Debug, Clone, Default)]
pub(crate) struct ByteOffsets(Vec<(usize, Offset)>);

impl ByteOffsets {
    /// The offsets of `offsets` for `text`; an index past the string is dropped.
    pub(crate) fn new(text: &str, offsets: &[CharOffset]) -> Self {
        if offsets.is_empty() {
            return Self::default();
        }
        let mut sorted: Vec<&CharOffset> = offsets.iter().collect();
        sorted.sort_by_key(|offset| offset.index);
        let mut wanted = sorted.into_iter().peekable();
        let mut bytes = Vec::new();
        for (index, (byte, _)) in text.char_indices().enumerate() {
            while let Some(offset) = wanted.next_if(|offset| offset.index <= index) {
                if offset.index == index {
                    bytes.push((
                        byte,
                        Offset {
                            along: offset.along.0,
                            across: offset.across.0,
                            rotation_rad: offset.rotation_rad,
                        },
                    ));
                }
            }
            if wanted.peek().is_none() {
                break;
            }
        }
        Self(bytes)
    }

    /// The offsets of the character at `byte` (0 for none).
    pub(crate) fn at(&self, byte: usize) -> Offset {
        self.0
            .binary_search_by_key(&byte, |(at, _)| *at)
            .map_or(Offset::default(), |found| self.0[found].1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indices_become_bytes() {
        let offset = |index, along| CharOffset {
            index,
            along: Mm(along),
            across: Mm(0.0),
            rotation_rad: 0.5,
        };
        let bytes = ByteOffsets::new("aé b", &[offset(3, 3.0), offset(1, 1.0), offset(9, 9.0)]);
        assert_eq!(bytes.at(1).along, 1.0);
        assert_eq!(bytes.at(4).along, 3.0);
        assert_eq!(bytes.at(4).rotation_rad, 0.5);
        assert_eq!(bytes.at(0), Offset::default());
    }
}
