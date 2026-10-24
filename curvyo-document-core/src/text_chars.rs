//! The per-character values of a text and their read and write as Loro marks (ADR 0021).
//!
//! ```text
//! text_string marks, one per character, ExpandType::None (format 21):
//!   kern_x : f64, factor of text_size, [-1, 1]   absent: 0   extra advance before the character
//!   kern_y : f64, factor of text_size, [-2, 2]   absent: 0   shift of the character and the
//!                                                             rest of its line, positive up
//!   rot    : f64, degrees, (-180, 180]           absent: 0   the character's own turn, added to
//!                                                             the text's Character rotation
//! ```
//!
//! A value at its default is never written. A non-finite or non-number value reads as 0, an
//! out-of-range factor is clamped (merge residue, never refused).

use std::ops::RangeInclusive;

use loro::{ExpandType, LoroDoc, LoroText, LoroValue, StyleConfig, StyleConfigMap, TextDelta};
use serde::Serialize;

/// The mark key of the horizontal kern.
const KEY_KERN_X: &str = "kern_x";
/// The mark key of the vertical kern.
const KEY_KERN_Y: &str = "kern_y";
/// The mark key of the character's own rotation.
const KEY_ROTATION: &str = "rot";
/// Two factors closer than this are the same stored value.
const SAME_VALUE: f64 = 1e-12;

/// The factors of Size a horizontal kern may hold (`0079` criterion 26).
pub const KERN_X_FACTORS: RangeInclusive<f64> = -1.0..=1.0;
/// The factors of Size a vertical kern may hold (`0079` criterion 26).
pub const KERN_Y_FACTORS: RangeInclusive<f64> = -2.0..=2.0;

/// The values of one character, factors of the text's Size.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct CharValues {
    /// Extra advance before the character, in reading direction.
    pub kern_x: f64,
    /// Shift of the character and the rest of its line, positive up (to the right of a column).
    pub kern_y: f64,
    /// The character's own turn in degrees, clockwise on screen, (-180, 180], added to the
    /// text's Character rotation.
    pub rotation: f64,
}

impl CharValues {
    /// No kern, no rotation.
    pub const DEFAULT: Self = Self {
        kern_x: 0.0,
        kern_y: 0.0,
        rotation: 0.0,
    };

    /// Whether every value is at its default.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.kern_x.abs() <= SAME_VALUE
            && self.kern_y.abs() <= SAME_VALUE
            && self.rotation.abs() <= SAME_VALUE
    }

    /// The values held to their ranges, the rotation wrapped into (-180°, 180°]; a non-finite
    /// value is 0.
    #[must_use]
    pub fn held(self) -> Self {
        let held = |value: f64, range: &RangeInclusive<f64>| {
            if value.is_finite() {
                value.clamp(*range.start(), *range.end())
            } else {
                0.0
            }
        };
        Self {
            kern_x: held(self.kern_x, &KERN_X_FACTORS),
            kern_y: held(self.kern_y, &KERN_Y_FACTORS),
            rotation: wrapped_degrees(self.rotation),
        }
    }
}

/// `degrees` wrapped into (-180, 180]; a non-finite value is 0.
fn wrapped_degrees(degrees: f64) -> f64 {
    if !degrees.is_finite() {
        return 0.0;
    }
    let turn = degrees.rem_euclid(360.0);
    if turn > 180.0 { turn - 360.0 } else { turn }
}

/// The non-default values of a text's characters, sorted by character index (Unicode scalar
/// values of the string); empty for almost every text.
#[derive(Debug, Clone, PartialEq, Default, Serialize)]
pub struct TextChars(Vec<(usize, CharValues)>);

impl TextChars {
    /// The values of `entries`, held to their ranges; defaults are dropped, the last value of an
    /// index wins.
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (usize, CharValues)>) -> Self {
        let mut entries: Vec<(usize, CharValues)> = entries
            .into_iter()
            .map(|(index, values)| (index, values.held()))
            .collect();
        entries.reverse();
        entries.sort_by_key(|(index, _)| *index);
        entries.dedup_by_key(|(index, _)| *index);
        entries.retain(|(_, values)| !values.is_default());
        Self(entries)
    }

    /// One value per character (`dense[i]` for character `i`).
    #[must_use]
    pub fn from_dense(dense: &[CharValues]) -> Self {
        Self::new(dense.iter().copied().enumerate())
    }

    /// One value per character of a string of `len` characters; values past it are dropped.
    #[must_use]
    pub fn to_dense(&self, len: usize) -> Vec<CharValues> {
        let mut dense = vec![CharValues::DEFAULT; len];
        for (index, values) in &self.0 {
            if let Some(slot) = dense.get_mut(*index) {
                *slot = *values;
            }
        }
        dense
    }

    /// The entries, sorted by index.
    #[must_use]
    pub fn entries(&self) -> &[(usize, CharValues)] {
        &self.0
    }

    /// The values of character `index`.
    #[must_use]
    pub fn get(&self, index: usize) -> CharValues {
        self.0
            .binary_search_by_key(&index, |(at, _)| *at)
            .map_or(CharValues::DEFAULT, |found| self.0[found].1)
    }

    /// Whether no character has a value.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Configures the mark keys on `doc` before any local edit (ADR 0021 §2): `ExpandType::None`, so
/// a character typed next to a kerned one gets nothing.
pub(crate) fn configure(doc: &LoroDoc) {
    let mut styles = StyleConfigMap::new();
    for key in [KEY_KERN_X, KEY_KERN_Y, KEY_ROTATION] {
        styles.insert(
            key.into(),
            StyleConfig {
                expand: ExpandType::None,
            },
        );
    }
    doc.config_text_style(styles);
}

/// A stored number (a factor or degrees); anything else is 0.
fn factor(value: Option<&LoroValue>) -> f64 {
    match value {
        Some(LoroValue::Double(value)) => *value,
        Some(LoroValue::I64(value)) => {
            // Small integers only: anything larger is clamped by `held` anyway.
            f64::from(i32::try_from(*value).unwrap_or(i32::MAX))
        }
        _ => 0.0,
    }
}

/// The values of `text`'s characters as stored.
pub(crate) fn read_chars(text: &LoroText) -> TextChars {
    let mut entries = Vec::new();
    let mut index = 0;
    for delta in text.to_delta() {
        let TextDelta::Insert { insert, attributes } = delta else {
            continue;
        };
        let count = insert.chars().count();
        if let Some(attributes) = attributes {
            let values = CharValues {
                kern_x: factor(attributes.get(KEY_KERN_X)),
                kern_y: factor(attributes.get(KEY_KERN_Y)),
                rotation: factor(attributes.get(KEY_ROTATION)),
            };
            if !values.held().is_default() {
                entries.extend((index..index + count).map(|at| (at, values)));
            }
        }
        index += count;
    }
    TextChars::new(entries)
}

/// Writes the values of `chars` to `text` where they differ from the stored ones: one mark per
/// character, one unmark per run of cleared characters (ADR 0021 §2). Indices past the string are
/// ignored. Returns whether anything was written; false also when the CRDT refused an edit. The
/// caller commits.
pub(crate) fn write_chars(text: &LoroText, chars: &TextChars) -> bool {
    let len = text.len_unicode();
    let stored = read_chars(text).to_dense(len);
    let wanted = chars.to_dense(len);
    let mut wrote = false;
    for (key, of) in [
        (
            KEY_KERN_X,
            (|v: &CharValues| v.kern_x) as fn(&CharValues) -> f64,
        ),
        (KEY_KERN_Y, |v: &CharValues| v.kern_y),
        (KEY_ROTATION, |v: &CharValues| v.rotation),
    ] {
        let mut cleared: Option<usize> = None;
        for index in 0..=len {
            let change = (index < len)
                .then(|| (of(&stored[index]), of(&wanted[index])))
                .filter(|(old, new)| (old - new).abs() > SAME_VALUE);
            let clears = change.is_some_and(|(_, new)| new.abs() <= SAME_VALUE);
            if !clears && let Some(start) = cleared.take() {
                wrote |= text.unmark(start..index, key).is_ok();
            }
            match change {
                Some(_) if clears => {
                    cleared.get_or_insert(index);
                }
                Some((_, new)) => wrote |= text.mark(index..index + 1, key, new).is_ok(),
                None => {}
            }
        }
    }
    wrote
}

/// Fills the fresh `into` with the string and values of `from` (a copy of a text object).
pub(crate) fn copy_text(from: &LoroText, into: &LoroText) -> loro::LoroResult<()> {
    into.insert(0, &from.to_string())?;
    write_chars(into, &read_chars(from));
    Ok(())
}

/// `chars` of `old` carried to `new` by their difference (the common start and end kept, as
/// [`crate::text_codec::apply_string_diff`] edits): the characters of the middle of `new` have no
/// value.
pub(crate) fn carried(chars: &TextChars, old: &str, new: &str) -> TextChars {
    let old: Vec<char> = old.chars().collect();
    let new: Vec<char> = new.chars().collect();
    let start = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let end = old[start..]
        .iter()
        .rev()
        .zip(new[start..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    TextChars::new(chars.entries().iter().filter_map(|&(index, values)| {
        if index < start {
            Some((index, values))
        } else if index >= old.len() - end && index < old.len() {
            Some((index + new.len() - old.len(), values))
        } else {
            None
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kern(x: f64) -> CharValues {
        CharValues {
            kern_x: x,
            ..CharValues::DEFAULT
        }
    }

    fn turned(degrees: f64) -> CharValues {
        CharValues {
            rotation: degrees,
            ..CharValues::DEFAULT
        }
    }

    /// Criterion 27b: a character's own rotation is stored in degrees, wrapped into
    /// (-180°, 180°], beside its kerns; clearing it leaves the kerns.
    #[test]
    fn ac27b_rotation_marks_round_trip_and_wrap() {
        assert_eq!(turned(270.0).held(), turned(-90.0));
        assert_eq!(turned(-180.0).held(), turned(180.0));
        assert_eq!(turned(f64::INFINITY).held(), CharValues::DEFAULT);
        let (_doc, text) = doc_text("AB");
        let both = CharValues {
            kern_x: 0.1,
            rotation: 30.0,
            ..CharValues::DEFAULT
        };
        let chars = TextChars::new([(0, both), (1, turned(-45.0))]);
        assert!(write_chars(&text, &chars));
        assert_eq!(read_chars(&text), chars);
        assert!(write_chars(&text, &TextChars::new([(0, kern(0.1))])));
        assert_eq!(read_chars(&text).entries(), &[(0, kern(0.1))]);
    }

    fn doc_text(string: &str) -> (LoroDoc, LoroText) {
        let doc = LoroDoc::new();
        configure(&doc);
        let text = doc
            .get_map("meta")
            .insert_container("s", LoroText::new())
            .unwrap();
        text.insert(0, string).unwrap();
        (doc, text)
    }

    #[test]
    fn values_hold_to_their_ranges_and_defaults_drop() {
        let chars = TextChars::new([
            (3, kern(5.0)),
            (1, kern(0.0)),
            (
                2,
                CharValues {
                    kern_x: f64::NAN,
                    kern_y: -9.0,
                    rotation: 0.0,
                },
            ),
            (3, kern(0.25)),
        ]);
        assert_eq!(
            chars.entries(),
            &[
                (
                    2,
                    CharValues {
                        kern_x: 0.0,
                        kern_y: -2.0,
                        rotation: 0.0,
                    }
                ),
                (3, kern(0.25))
            ]
        );
        assert_eq!(chars.get(0), CharValues::DEFAULT);
        assert_eq!(TextChars::from_dense(&chars.to_dense(4)), chars);
    }

    #[test]
    fn marks_round_trip_one_per_character_and_clear() {
        let (_doc, text) = doc_text("HAVE");
        let chars = TextChars::new([(1, kern(-0.1)), (2, kern(-0.1)), (3, kern(0.2))]);
        assert!(write_chars(&text, &chars));
        assert_eq!(read_chars(&text), chars);
        assert!(!write_chars(&text, &chars), "unchanged writes nothing");
        assert!(write_chars(&text, &TextChars::default()));
        assert!(read_chars(&text).is_empty());
    }

    /// ADR 0021 §1: a damaged value reads as 0 or is clamped.
    #[test]
    fn damaged_marks_read_leniently() {
        let (_doc, text) = doc_text("AB");
        text.mark(0..1, KEY_KERN_X, "wide").unwrap();
        text.mark(1..2, KEY_KERN_Y, 7.0).unwrap();
        assert_eq!(
            read_chars(&text).entries(),
            &[(
                1,
                CharValues {
                    kern_x: 0.0,
                    kern_y: 2.0,
                    rotation: 0.0,
                }
            )]
        );
    }

    /// Criterion 30: typing before a kerned character moves its kern along; a character typed
    /// directly before or after it gets none; deleting it deletes the kern.
    #[test]
    fn ac30_a_kern_stays_with_its_character() {
        let (_doc, text) = doc_text("AV");
        write_chars(&text, &TextChars::new([(1, kern(-0.1))]));
        assert!(crate::text_codec::apply_string_diff(&text, "xAV"));
        assert_eq!(read_chars(&text).entries(), &[(2, kern(-0.1))]);
        assert!(crate::text_codec::apply_string_diff(&text, "xAyV"));
        assert!(crate::text_codec::apply_string_diff(&text, "xAyVz"));
        assert_eq!(read_chars(&text).entries(), &[(3, kern(-0.1))]);
        assert!(crate::text_codec::apply_string_diff(&text, "xAyz"));
        assert!(read_chars(&text).is_empty());
        assert!(crate::text_codec::apply_string_diff(&text, "xAyVz"));
        assert!(read_chars(&text).is_empty(), "typed again, no kern");
    }

    /// ADR 0021 §2: one mark per character, so a character inserted between two kerned ones
    /// gets nothing.
    #[test]
    fn a_character_between_two_kerned_ones_gets_nothing() {
        let (_doc, text) = doc_text("AB");
        write_chars(&text, &TextChars::new([(0, kern(0.1)), (1, kern(0.1))]));
        text.insert(1, "x").unwrap();
        assert_eq!(
            read_chars(&text).entries(),
            &[(0, kern(0.1)), (2, kern(0.1))]
        );
    }

    #[test]
    fn carried_follows_the_difference() {
        let chars = TextChars::new([(0, kern(0.1)), (2, kern(0.2)), (3, kern(0.3))]);
        let moved = carried(&chars, "ABCD", "AxyCD");
        assert_eq!(
            moved.entries(),
            &[(0, kern(0.1)), (3, kern(0.2)), (4, kern(0.3))]
        );
        assert_eq!(
            carried(&chars, "ABCD", "AD").entries(),
            &[(0, kern(0.1)), (1, kern(0.3))]
        );
    }
}
