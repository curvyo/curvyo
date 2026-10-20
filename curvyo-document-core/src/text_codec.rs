//! The stored shape of a text object (ADR 0020 §1), read, written and validated here.
//!
//! ```text
//! meta map of a text:
//!   shape          : "text"
//!   text_string    : LoroText container    per-character merge (ADR 0009 §3), at most
//!                                          MAX_TEXT_CHARS Unicode scalar values
//!   text_font      : { "family": "...", "style": "...", "face_hash": "<64 hex>",
//!                      "face_index": 0 }
//!                    ONE LWW register holding a map *value*, written whole by a maker's pick
//!   text_size      : 20.0                  mm, the capital H, 0.1 ..= 2000
//!   text_placement : [a, b, c, d, e, f]    text space to document, one list value
//!   + the style keys of a path (`style_codec`), `name`, `visible`, `locked`, `opacity`
//! ```
//!
//! No measured layout is stored: no advances, line positions or outlines.

use std::collections::HashMap;

use loro::{LoroMap, LoroText, LoroValue, ValueOrContainer};

use crate::affine::Affine;
use crate::path_codec::as_f64;
use crate::path_model::NodeId;
use crate::text_model::{
    FontRef, MAX_STORED_TEXT_CHARS, MAX_TEXT_SIZE, MIN_TEXT_SIZE, TextSnapshot,
};
use crate::units::Length;

/// The `shape` tag of a text node.
pub(crate) const SHAPE_TEXT: &str = "text";
const KEY_STRING: &str = "text_string";
const KEY_FONT: &str = "text_font";
const KEY_SIZE: &str = "text_size";
const KEY_PLACEMENT: &str = "text_placement";
const KEY_FAMILY: &str = "family";
const KEY_STYLE: &str = "style";
const KEY_FACE_HASH: &str = "face_hash";
const KEY_FACE_INDEX: &str = "face_index";

/// Whether `meta` is a text node's.
pub(crate) fn is_text_meta(meta: &LoroMap) -> bool {
    matches!(
        meta.get(crate::shape_codec::KEY_SHAPE),
        Some(ValueOrContainer::Value(LoroValue::String(tag))) if *tag == SHAPE_TEXT
    )
}

/// Writes a new text node's registers (the caller has written the `shape` tag and the style).
pub(crate) fn write_text(meta: &LoroMap, text: &TextSnapshot) {
    // invariant: inserting a new container under a known key of an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    let string = meta.insert_container(KEY_STRING, LoroText::new()).unwrap();
    // invariant: inserting into a fresh attached text at position 0 cannot fail.
    #[allow(clippy::unwrap_used)]
    string.insert(0, &text.string).unwrap();
    write_font(meta, &text.font);
    write_size(meta, text.size);
    write_placement(meta, text.placement);
}

/// The text's string container, if it has one.
pub(crate) fn string_container(meta: &LoroMap) -> Option<LoroText> {
    match meta.get(KEY_STRING)? {
        ValueOrContainer::Container(container) => container.into_text().ok(),
        ValueOrContainer::Value(_) => None,
    }
}

/// Turns the string of `container` into `string` by its difference: the common start and end are
/// kept, the middle is deleted and inserted (positions in Unicode scalar values), so a peer's
/// concurrent edit elsewhere in the string merges per character (ADR 0009 §3). False when the
/// CRDT refuses an edit.
pub(crate) fn apply_string_diff(container: &LoroText, string: &str) -> bool {
    let old: Vec<char> = container.to_string().chars().collect();
    let new: Vec<char> = string.chars().collect();
    let start = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
    let end = old[start..]
        .iter()
        .rev()
        .zip(new[start..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let deleted = old.len() - start - end;
    let inserted: String = new[start..new.len() - end].iter().collect();
    (deleted == 0 || container.delete(start, deleted).is_ok())
        && (inserted.is_empty() || container.insert(start, &inserted).is_ok())
}

/// Writes the font register as one value.
pub(crate) fn write_font(meta: &LoroMap, font: &FontRef) {
    let value: HashMap<String, LoroValue> = HashMap::from([
        (KEY_FAMILY.to_owned(), LoroValue::from(font.family.as_str())),
        (KEY_STYLE.to_owned(), LoroValue::from(font.style.as_str())),
        (
            KEY_FACE_HASH.to_owned(),
            LoroValue::from(font.face_hash.as_str()),
        ),
        (
            KEY_FACE_INDEX.to_owned(),
            LoroValue::I64(i64::from(font.face_index)),
        ),
    ]);
    // invariant: inserting a plain value under a known key on an attached map cannot fail.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_FONT, LoroValue::from(value)).unwrap();
}

/// Writes the size register.
pub(crate) fn write_size(meta: &LoroMap, size: Length) {
    // invariant: see `write_font`.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_SIZE, size.as_mm()).unwrap();
}

/// Writes the placement register as one list value.
pub(crate) fn write_placement(meta: &LoroMap, placement: Affine) {
    // invariant: see `write_font`.
    #[allow(clippy::unwrap_used)]
    meta.insert(KEY_PLACEMENT, placement.to_array().to_vec())
        .unwrap();
}

pub(crate) fn read_font(meta: &LoroMap) -> Option<FontRef> {
    let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_FONT) else {
        return None;
    };
    let text = |key: &str| match map.get(key) {
        Some(LoroValue::String(s)) => Some(s.to_string()),
        _ => None,
    };
    let face_hash = text(KEY_FACE_HASH)?;
    let hex = crate::text_model::is_face_hash(&face_hash);
    let Some(LoroValue::I64(index)) = map.get(KEY_FACE_INDEX) else {
        return None;
    };
    Some(FontRef {
        family: text(KEY_FAMILY)?,
        style: text(KEY_STYLE)?,
        face_hash: hex.then_some(face_hash)?,
        face_index: u32::try_from(*index).ok()?,
    })
}

pub(crate) fn read_size(meta: &LoroMap) -> Option<Length> {
    let Some(ValueOrContainer::Value(value)) = meta.get(KEY_SIZE) else {
        return None;
    };
    let mm = as_f64(&value)?;
    (MIN_TEXT_SIZE.as_mm()..=MAX_TEXT_SIZE.as_mm())
        .contains(&mm)
        .then(|| Length::from_mm(mm))
}

pub(crate) fn read_placement(meta: &LoroMap) -> Option<Affine> {
    let Some(ValueOrContainer::Value(LoroValue::List(list))) = meta.get(KEY_PLACEMENT) else {
        return None;
    };
    if list.len() != 6 {
        return None;
    }
    let mut entries = [0.0; 6];
    for (entry, value) in entries.iter_mut().zip(list.iter()) {
        *entry = as_f64(value)?;
    }
    let placement = Affine::from_array(entries);
    (placement.is_within_limit()
        && placement.determinant().abs() > crate::affine::SINGULAR_DETERMINANT)
        .then_some(placement)
}

/// The text node `id` as stored, or `None` when a register is missing or damaged (merge residue
/// after open-file validation; the readers skip such a node).
pub(crate) fn read_text_snapshot(id: NodeId, meta: &LoroMap) -> Option<TextSnapshot> {
    Some(TextSnapshot {
        id,
        string: string_container(meta)?.to_string(),
        font: read_font(meta)?,
        size: read_size(meta)?,
        placement: read_placement(meta)?,
        style: crate::style_codec::read_style(meta),
        outline: None,
    })
}

/// Writes a transformed text's size and placement where they differ from the stored ones. The
/// caller commits.
pub(crate) fn write_size_and_placement_if_changed(meta: &LoroMap, text: &TextSnapshot) {
    if read_size(meta) != Some(text.size) {
        write_size(meta, text.size);
    }
    if read_placement(meta) != Some(text.placement) {
        write_placement(meta, text.placement);
    }
}

/// Shifts a text by `offset`: its placement's translation (the anchor). The caller commits.
pub(crate) fn translate_text_meta(meta: &LoroMap, offset: crate::units::Vec2) {
    if let Some(placement) = read_placement(meta) {
        write_placement(meta, Affine::translation(offset).compose(placement));
    }
}

/// Open-file validation of a text node: every register present and of its stored shape, the
/// string at most [`MAX_TEXT_CHARS`] long, the size in range, the placement finite, within the
/// matrix limit and invertible, and the style keys valid as on a path.
pub(crate) fn validate_text_node(meta: &LoroMap) -> bool {
    string_container(meta).is_some_and(|string| string.len_unicode() <= MAX_STORED_TEXT_CHARS)
        && read_font(meta).is_some()
        && read_size(meta).is_some()
        && read_placement(meta).is_some()
        && crate::style_validation::style_is_valid(meta)
}

#[cfg(test)]
mod tests {
    use loro::LoroDoc;

    use super::*;
    use crate::style_model::Style;

    fn sample() -> TextSnapshot {
        TextSnapshot {
            id: NodeId::from_parts(1, 0),
            string: "Hi\nthere".to_owned(),
            font: FontRef {
                family: "Source Serif 4".to_owned(),
                style: "Bold".to_owned(),
                face_hash: "ab".repeat(32),
                face_index: 2,
            },
            size: Length::from_mm(12.5),
            placement: Affine::new(0.0, 1.0, -1.0, 0.0, 3.0, 4.0),
            style: Style::default(),
            outline: None,
        }
    }

    fn written() -> (LoroDoc, LoroMap) {
        let doc = LoroDoc::new();
        let meta = doc.get_map("meta");
        crate::shape_codec::write_shape_tag(&meta, SHAPE_TEXT);
        write_text(&meta, &sample());
        (doc, meta)
    }

    #[test]
    fn a_written_text_reads_back_and_validates() {
        let (_doc, meta) = written();
        assert!(is_text_meta(&meta));
        assert!(validate_text_node(&meta));
        assert_eq!(
            read_text_snapshot(NodeId::from_parts(1, 0), &meta),
            Some(sample())
        );
    }

    #[test]
    fn every_damaged_register_is_refused() {
        let damage: [(&str, LoroValue); 8] = [
            (KEY_SIZE, LoroValue::Double(0.01)),
            (KEY_SIZE, LoroValue::Double(f64::NAN)),
            (
                KEY_PLACEMENT,
                LoroValue::from(vec![1.0, 0.0, 0.0, 1.0, 0.0]),
            ),
            (KEY_PLACEMENT, LoroValue::from(vec![0.0; 6])),
            (KEY_FONT, LoroValue::from("Source Sans 3")),
            (KEY_STRING, LoroValue::from("not a container")),
            (KEY_FONT, {
                let (_doc, meta) = written();
                let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_FONT) else {
                    unreachable!()
                };
                let mut map = (*map).clone();
                map.insert(KEY_FACE_HASH.to_owned(), LoroValue::from("xyz"));
                LoroValue::from(map)
            }),
            (KEY_FONT, {
                let (_doc, meta) = written();
                let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_FONT) else {
                    unreachable!()
                };
                let mut map = (*map).clone();
                map.insert(KEY_FACE_HASH.to_owned(), LoroValue::from("AB".repeat(32)));
                LoroValue::from(map)
            }),
        ];
        for (key, value) in damage {
            let (_doc, meta) = written();
            meta.insert(key, value.clone()).unwrap();
            assert!(!validate_text_node(&meta), "{key} = {value:?}");
        }
        let (_doc, meta) = written();
        let long = "x".repeat(MAX_STORED_TEXT_CHARS + 1);
        let string = string_container(&meta).unwrap();
        string.insert(0, &long).unwrap();
        assert!(!validate_text_node(&meta), "too long");
    }
}
