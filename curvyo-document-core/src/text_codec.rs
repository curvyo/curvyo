//! The stored shape of a text object (ADR 0020 §1), read, written and validated here.
//!
//! ```text
//! meta map of a text:
//!   shape          : "text"
//!   text_string    : LoroText container    per-character merge (ADR 0009 §3), at most
//!                                          MAX_TEXT_CHARS Unicode scalar values
//!   text_font      : { "family": "...", "style": "...", "face_hash": "<64 hex>",
//!                      "face_index": 0, "kind": "single_line" }
//!                    `kind` is optional (format 18, `specs/0076-single-line-fonts`): absent or
//!                    any other value is an outline face
//!                    ONE LWW register holding a map *value*, written whole by a maker's pick
//!   text_size      : 20.0                  mm, the capital H, 0.1 ..= 2000
//!   text_placement : [a, b, c, d, e, f]    text space to document, one list value
//!   text_align        : "centre" | "right" | "justify"   absent: left
//!   text_letter_space : f64, factor of text_size          absent: 0
//!   text_word_space   : f64, factor of text_size          absent: 0
//!   text_line_space   : f64, factor of text_size, >= 0    absent: Auto
//!   text_rotation     : f64, degrees in (-180, 180]       absent: 0
//!   text_writing_mode      : "vertical_rl" | "vertical_lr"   absent: horizontal
//!   text_glyph_orientation : "upright" | "sideways"          absent: auto
//!                    the layout keys are optional (format 19, the last two format 20,
//!                    `specs/0079-text-layout-controls/adrs.md` decisions 1 and 13): a value at
//!                    its default is deleted, never written; an unknown code reads as the
//!                    default, a non-finite number as the default, a factor out of range is
//!                    clamped
//!   text_string marks kern_x, kern_y: per character (format 21, ADR 0021, `crate::text_chars`)
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
    FontKind, FontRef, MAX_STORED_TEXT_CHARS, MAX_TEXT_SIZE, MIN_TEXT_SIZE, TextAlign,
    TextGlyphOrientation, TextLayoutValues, TextSnapshot, TextWritingMode,
};
use crate::units::{Angle, Length};

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
const KEY_KIND: &str = "kind";
const KIND_SINGLE_LINE: &str = "single_line";
const KEY_ALIGN: &str = "text_align";
const KEY_LETTER_SPACE: &str = "text_letter_space";
const KEY_WORD_SPACE: &str = "text_word_space";
const KEY_LINE_SPACE: &str = "text_line_space";
const KEY_ROTATION: &str = "text_rotation";
const KEY_WRITING_MODE: &str = "text_writing_mode";
const KEY_ORIENTATION: &str = "text_glyph_orientation";
/// Two factors or rotations (radians) closer than this are the same stored value.
const SAME_VALUE: f64 = 1e-12;

fn differs(a: f64, b: f64) -> bool {
    (a - b).abs() > SAME_VALUE
}

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
    crate::text_chars::write_chars(&string, &text.chars);
    write_font(meta, &text.font);
    write_size(meta, text.size);
    write_placement(meta, text.placement);
    write_layout(meta, &text.layout);
}

fn align_code(align: TextAlign) -> Option<&'static str> {
    match align {
        TextAlign::Left => None,
        TextAlign::Centre => Some("centre"),
        TextAlign::Right => Some("right"),
        TextAlign::Justify => Some("justify"),
    }
}

const fn writing_mode_code(mode: TextWritingMode) -> Option<&'static str> {
    match mode {
        TextWritingMode::Horizontal => None,
        TextWritingMode::VerticalRl => Some("vertical_rl"),
        TextWritingMode::VerticalLr => Some("vertical_lr"),
    }
}

const fn orientation_code(orientation: TextGlyphOrientation) -> Option<&'static str> {
    match orientation {
        TextGlyphOrientation::Auto => None,
        TextGlyphOrientation::Upright => Some("upright"),
        TextGlyphOrientation::Sideways => Some("sideways"),
    }
}

/// Writes the layout keys that differ from the stored ones: a value at its default deletes its
/// key (`0079` decision 1). Returns whether anything was written. The caller commits.
pub(crate) fn write_layout(meta: &LoroMap, layout: &TextLayoutValues) -> bool {
    let stored = read_layout(meta);
    let layout = layout.held();
    let mut wrote = false;
    let mut put = |key: &str, value: Option<LoroValue>| {
        let present = meta.get(key).is_some();
        // invariant: inserting or deleting a plain value under a known key on an attached map
        // cannot fail.
        #[allow(clippy::unwrap_used)]
        match value {
            Some(value) => meta.insert(key, value).unwrap(),
            None if present => meta.delete(key).unwrap(),
            None => return,
        }
        wrote = true;
    };
    if stored.align != layout.align {
        put(KEY_ALIGN, align_code(layout.align).map(LoroValue::from));
    }
    let factor = |value: f64| (value.abs() > SAME_VALUE).then_some(LoroValue::Double(value));
    if differs(stored.letter_spacing, layout.letter_spacing) {
        put(KEY_LETTER_SPACE, factor(layout.letter_spacing));
    }
    if differs(stored.word_spacing, layout.word_spacing) {
        put(KEY_WORD_SPACE, factor(layout.word_spacing));
    }
    let line_changed = match (stored.line_spacing, layout.line_spacing) {
        (Some(a), Some(b)) => differs(a, b),
        (a, b) => a.is_some() != b.is_some(),
    };
    if line_changed {
        put(KEY_LINE_SPACE, layout.line_spacing.map(LoroValue::Double));
    }
    if stored.writing_mode != layout.writing_mode {
        put(
            KEY_WRITING_MODE,
            writing_mode_code(layout.writing_mode).map(LoroValue::from),
        );
    }
    if stored.orientation != layout.orientation {
        put(
            KEY_ORIENTATION,
            orientation_code(layout.orientation).map(LoroValue::from),
        );
    }
    // Degrees on disk, radians in the model: equal within float noise is unchanged.
    if differs(stored.rotation.as_radians(), layout.rotation.as_radians()) {
        put(
            KEY_ROTATION,
            factor(layout.rotation.as_radians().to_degrees()),
        );
    }
    wrote
}

/// The layout values as stored, every absent or damaged key at its default.
pub(crate) fn read_layout(meta: &LoroMap) -> TextLayoutValues {
    let number = |key: &str| match meta.get(key) {
        Some(ValueOrContainer::Value(value)) => as_f64(&value).filter(|v| v.is_finite()),
        _ => None,
    };
    let code = |key: &str| match meta.get(key) {
        Some(ValueOrContainer::Value(LoroValue::String(code))) => Some(code.to_string()),
        _ => None,
    };
    let align = match code(KEY_ALIGN).as_deref() {
        Some("centre") => TextAlign::Centre,
        Some("right") => TextAlign::Right,
        Some("justify") => TextAlign::Justify,
        _ => TextAlign::Left,
    };
    let writing_mode = match code(KEY_WRITING_MODE).as_deref() {
        Some("vertical_rl") => TextWritingMode::VerticalRl,
        Some("vertical_lr") => TextWritingMode::VerticalLr,
        _ => TextWritingMode::Horizontal,
    };
    let orientation = match code(KEY_ORIENTATION).as_deref() {
        Some("upright") => TextGlyphOrientation::Upright,
        Some("sideways") => TextGlyphOrientation::Sideways,
        _ => TextGlyphOrientation::Auto,
    };
    TextLayoutValues {
        align,
        letter_spacing: number(KEY_LETTER_SPACE).unwrap_or(0.0),
        word_spacing: number(KEY_WORD_SPACE).unwrap_or(0.0),
        line_spacing: number(KEY_LINE_SPACE),
        rotation: Angle::from_radians(number(KEY_ROTATION).unwrap_or(0.0).to_radians()),
        writing_mode,
        orientation,
    }
    .held()
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
    let mut value: HashMap<String, LoroValue> = HashMap::from([
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
    if font.kind == FontKind::SingleLine {
        value.insert(KEY_KIND.to_owned(), LoroValue::from(KIND_SINGLE_LINE));
    }
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
        kind: if text(KEY_KIND).as_deref() == Some(KIND_SINGLE_LINE) {
            FontKind::SingleLine
        } else {
            FontKind::Outline
        },
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
    let string = string_container(meta)?;
    Some(TextSnapshot {
        id,
        string: string.to_string(),
        font: read_font(meta)?,
        size: read_size(meta)?,
        layout: read_layout(meta),
        chars: crate::text_chars::read_chars(&string),
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
                kind: FontKind::SingleLine,
            },
            size: Length::from_mm(12.5),
            layout: TextLayoutValues::DEFAULT,
            chars: crate::text_chars::TextChars::default(),
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

    /// `specs/0076-single-line-fonts` criterion 21: an unknown kind reads as outline.
    #[test]
    fn an_unknown_kind_reads_as_outline() {
        let (_doc, meta) = written();
        let Some(ValueOrContainer::Value(LoroValue::Map(map))) = meta.get(KEY_FONT) else {
            unreachable!()
        };
        let mut map = (*map).clone();
        map.insert(KEY_KIND.to_owned(), LoroValue::from("engraved"));
        meta.insert(KEY_FONT, LoroValue::from(map)).unwrap();
        assert_eq!(read_font(&meta).unwrap().kind, FontKind::Outline);
        assert!(validate_text_node(&meta));
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

    fn layout_keys(meta: &LoroMap) -> Vec<String> {
        meta.keys()
            .map(|key| key.to_string())
            .filter(|key| {
                [
                    KEY_ALIGN,
                    KEY_LETTER_SPACE,
                    KEY_WORD_SPACE,
                    KEY_LINE_SPACE,
                    KEY_ROTATION,
                    KEY_WRITING_MODE,
                    KEY_ORIENTATION,
                ]
                .contains(&key.as_str())
            })
            .collect()
    }

    /// `specs/0079-text-layout-controls` decision 1: the five keys round-trip, a default deletes
    /// its key, so a text back at its defaults is stored as before format 19.
    #[test]
    fn the_layout_keys_round_trip_and_defaults_are_not_stored() {
        let (_doc, meta) = written();
        assert!(
            layout_keys(&meta).is_empty(),
            "a default text writes no key"
        );
        let values = TextLayoutValues {
            align: TextAlign::Justify,
            letter_spacing: 0.15,
            word_spacing: -0.2,
            line_spacing: Some(1.5),
            rotation: Angle::from_radians(12.5_f64.to_radians()),
            writing_mode: TextWritingMode::VerticalRl,
            orientation: TextGlyphOrientation::Upright,
        };
        assert!(write_layout(&meta, &values));
        assert_eq!(layout_keys(&meta).len(), 7);
        assert_eq!(read_layout(&meta).writing_mode, TextWritingMode::VerticalRl);
        assert_eq!(
            read_layout(&meta).orientation,
            TextGlyphOrientation::Upright
        );
        let read = read_layout(&meta);
        assert_eq!(read.align, values.align);
        assert!((read.rotation.as_radians() - values.rotation.as_radians()).abs() < 1e-12);
        assert!(!write_layout(&meta, &read), "unchanged writes nothing");
        assert!(write_layout(&meta, &TextLayoutValues::DEFAULT));
        assert_eq!(layout_keys(&meta), Vec::<String>::new());
        assert_eq!(read_layout(&meta), TextLayoutValues::DEFAULT);
    }

    /// Decision 1, read rules: an unknown align is left, a non-finite number its default, a factor
    /// out of range is clamped, a rotation wraps into (-180, 180]; the text still validates.
    #[test]
    fn damaged_layout_keys_read_leniently() {
        let (_doc, meta) = written();
        meta.insert(KEY_ALIGN, LoroValue::from("diagonal")).unwrap();
        meta.insert(KEY_LETTER_SPACE, LoroValue::Double(f64::NAN))
            .unwrap();
        meta.insert(KEY_WORD_SPACE, LoroValue::Double(99.0))
            .unwrap();
        meta.insert(KEY_LINE_SPACE, LoroValue::Double(-3.0))
            .unwrap();
        meta.insert(KEY_ROTATION, LoroValue::Double(270.0)).unwrap();
        meta.insert(KEY_WRITING_MODE, LoroValue::from("diagonal"))
            .unwrap();
        meta.insert(KEY_ORIENTATION, LoroValue::Double(1.0))
            .unwrap();
        let read = read_layout(&meta);
        assert_eq!(read.align, TextAlign::Left);
        assert_eq!(read.writing_mode, TextWritingMode::Horizontal);
        assert_eq!(read.orientation, TextGlyphOrientation::Auto);
        assert!(read.letter_spacing.abs() < 1e-12);
        assert!((read.word_spacing - 5.0).abs() < 1e-12);
        assert_eq!(read.line_spacing, Some(0.0));
        assert!((read.rotation.as_radians().to_degrees() + 90.0).abs() < 1e-9);
        assert!(validate_text_node(&meta));
    }
}
