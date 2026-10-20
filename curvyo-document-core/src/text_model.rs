//! The read model of a text object (ADR 0020 §1): its string, font reference, size and placement,
//! and the transform rule that puts a uniform scale into the size.

use serde::Serialize;

use crate::affine::Affine;
use crate::path_model::{NodeId, PathSnapshot};
use crate::style_model::Style;
use crate::units::{Angle, Length, Point};

/// The most Unicode scalar values a text holds (`specs/0074-text-objects` criterion 31).
pub const MAX_TEXT_CHARS: usize = 10_000;

/// The most Unicode scalar values a stored text may hold when a file opens: a hostile-input
/// bound, not the typing limit. A merge of two valid edits may pass [`MAX_TEXT_CHARS`] and must
/// still open (ADR 0009 §3, ADR 0004 §9); `curvyo-text-core` lays out up to the same count.
pub const MAX_STORED_TEXT_CHARS: usize = 100_000;

/// The smallest text size, the height of the capital H (criterion 12).
pub const MIN_TEXT_SIZE: Length = Length::from_mm(0.1);

/// The largest text size (criterion 12).
pub const MAX_TEXT_SIZE: Length = Length::from_mm(2000.0);

/// How many characters of the string name a text whose `name` is empty (criterion 27).
pub const TEXT_NAME_CHARS: usize = 24;

/// The id a new text in edit mode is drawn and hit under before it exists: no node has it
/// (counter `i32::MIN` is never minted).
pub const UNCREATED_TEXT_ID: NodeId = NodeId::from_parts(0, i32::MIN);

/// The relative tolerance within which a linear map counts as a uniform scale (ADR 0020 §1).
const UNIFORM_TOLERANCE: f64 = 1e-9;

/// The face a text uses: written only by a maker's pick (ADR 0020 §1, `text_font`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FontRef {
    /// The family name, for example "Source Sans 3".
    pub family: String,
    /// The face's style name, for example "Bold".
    pub style: String,
    /// The SHA-256 of the font file's bytes, 64 lower-case hex digits.
    pub face_hash: String,
    /// The face's index in a collection file, else 0.
    pub face_index: u32,
}

/// One text object as read from the document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TextSnapshot {
    /// The node.
    pub id: NodeId,
    /// The string; lines are separated by `\n`.
    pub string: String,
    /// The face.
    pub font: FontRef,
    /// The height of the capital H (spec "Terms", Size).
    pub size: Length,
    /// The map from text space (mm, origin at the left end of the first baseline, y down) to
    /// the document. Its translation is the anchor.
    pub placement: Affine,
    /// Fill and stroke.
    pub style: Style,
    /// What the editor session laid out through the text engine; `None` as read from the
    /// document and until the face is laid out. Derived, never stored (ADR 0020 §1: no measured
    /// layout in the file), and mapped with the text by [`TextSnapshot::mapped_by`].
    #[serde(skip)]
    pub outline: Option<std::sync::Arc<TextOutline>>,
}

/// A text as laid out, in document mm.
#[derive(Debug, Clone, PartialEq)]
pub struct TextOutline {
    /// Every glyph's contours as one compound path, with the text's id and style.
    pub glyphs: PathSnapshot,
    /// The corners of the line box (criterion 25): top-left, top-right, bottom-right,
    /// bottom-left in text space, mapped to the document.
    pub line_box: [Point; 4],
    /// How many outlines of `glyphs` each drawn glyph has, in drawing order: Object to path
    /// writes one compound path per glyph (criterion 22).
    pub glyph_outlines: Vec<usize>,
    /// Whether the glyphs are those of the included sans face standing in for a missing face
    /// (ADR 0020 §1, matching step c): drawn and hit, never cut or converted (criterion 29).
    pub substitute: bool,
}

impl TextOutline {
    /// This outline under the document map `map`.
    #[must_use]
    pub fn mapped(&self, map: Affine) -> Self {
        Self {
            glyphs: self.glyphs.mapped(map),
            line_box: self.line_box.map(|corner| map.apply_point(corner)),
            glyph_outlines: self.glyph_outlines.clone(),
            substitute: self.substitute,
        }
    }
}

/// Whether `hash` is 64 lower-case hex digits, as a face hash is stored (ADR 0020 §1).
#[must_use]
pub fn is_face_hash(hash: &str) -> bool {
    hash.len() == 64 && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// `size` held to [`MIN_TEXT_SIZE`] ..= [`MAX_TEXT_SIZE`]; a non-finite size reads as the smallest.
#[must_use]
pub fn clamp_text_size(size: Length) -> Length {
    let mm = size.as_mm();
    if mm.is_nan() {
        return MIN_TEXT_SIZE;
    }
    Length::from_mm(mm.clamp(MIN_TEXT_SIZE.as_mm(), MAX_TEXT_SIZE.as_mm()))
}

/// The factor `s` when the linear part of `map` is `s` times an orthogonal matrix (a rotation,
/// possibly with a flip), within [`UNIFORM_TOLERANCE`] relative; else `None`.
fn uniform_scale(map: Affine) -> Option<f64> {
    let s_squared = map.determinant().abs();
    if !(s_squared.is_finite() && s_squared > 0.0) {
        return None;
    }
    let tolerance = UNIFORM_TOLERANCE * s_squared;
    let columns_equal = (map.a.mul_add(map.a, map.b * map.b) - s_squared).abs() <= tolerance
        && (map.c.mul_add(map.c, map.d * map.d) - s_squared).abs() <= tolerance;
    let orthogonal = map.a.mul_add(map.c, map.b * map.d).abs() <= tolerance;
    // A rotation or a flip has a factor of 1 within rounding: exactly 1, so the size register is
    // never rewritten by a turn (architect review m2, must-fix 3).
    (columns_equal && orthogonal).then(|| {
        let scale = s_squared.sqrt();
        if (scale - 1.0).abs() <= UNIFORM_TOLERANCE {
            1.0
        } else {
            scale
        }
    })
}

impl TextSnapshot {
    /// This text after the document map `map` (spec criterion 26, ADR 0020 §1). A uniform scale
    /// `s` multiplies the size (held to the size range) and the placement takes the rest of
    /// `map`, so the drawn letters land where `map` puts them; any other map is composed into the
    /// placement and the size is kept.
    #[must_use]
    pub fn mapped_by(&self, map: Affine) -> Self {
        let mut text = self.clone();
        match uniform_scale(map) {
            Some(scale) => {
                let size = clamp_text_size(Length::from_mm(self.size.as_mm() * scale));
                let applied = size.as_mm() / self.size.as_mm();
                text.size = size;
                text.placement = map
                    .compose(self.placement)
                    .compose(Affine::scale(1.0 / applied, 1.0 / applied));
            }
            None => text.placement = map.compose(self.placement),
        }
        text.outline = self
            .outline
            .as_ref()
            .map(|outline| std::sync::Arc::new(outline.mapped(map)));
        text
    }

    /// The angle of the text's baseline in the document: the direction of the placement's x-axis.
    #[must_use]
    pub fn rotation(&self) -> Angle {
        Angle::from_radians(self.placement.b.atan2(self.placement.a))
    }

    /// The name a text without a stored name shows: its first [`TEXT_NAME_CHARS`] characters,
    /// line breaks as spaces, with an ellipsis when the string is longer (criterion 27).
    #[must_use]
    pub fn default_name(&self) -> String {
        let mut name: String = self
            .string
            .chars()
            .take(TEXT_NAME_CHARS)
            .map(|c| if c == '\n' { ' ' } else { c })
            .collect();
        if self.string.chars().nth(TEXT_NAME_CHARS).is_some() {
            name.push('…');
        }
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Vec2;

    fn text(size: f64, placement: Affine) -> TextSnapshot {
        TextSnapshot {
            id: NodeId::from_parts(1, 1),
            string: "Hello\nworld, this is a long line".to_owned(),
            font: FontRef {
                family: "Source Sans 3".to_owned(),
                style: "Regular".to_owned(),
                face_hash: "0".repeat(64),
                face_index: 0,
            },
            size: Length::from_mm(size),
            placement,
            style: Style::default(),
            outline: None,
        }
    }

    fn draws_alike(before: &TextSnapshot, after: &TextSnapshot, map: Affine) {
        // A point of the text at its old size, mapped, must equal the same glyph point at the new
        // size under the new placement.
        let ratio = after.size.as_mm() / before.size.as_mm();
        for p in [
            Point::new(0.0, 0.0),
            Point::new(10.0, -7.0),
            Point::new(-3.0, 4.0),
        ] {
            let expected = map.apply_point(before.placement.apply_point(p));
            let scaled = Point::new(p.x * ratio, p.y * ratio);
            let got = after.placement.apply_point(scaled);
            assert!((expected.x - got.x).abs() < 1e-9 && (expected.y - got.y).abs() < 1e-9);
        }
    }

    /// Criterion 26: a uniform scale goes into the size, a rotation and a stretch into the
    /// placement; the drawn letters land where the map puts them either way.
    #[test]
    fn ac26_uniform_scale_into_size_else_placement() {
        let start = text(10.0, Affine::translation(Vec2::new(5.0, 7.0)));
        let uniform = Affine::rotation_about(
            Point::new(1.0, 2.0),
            Angle::from_radians(30.0_f64.to_radians()),
        )
        .compose(Affine::scale(2.0, 2.0));
        let scaled = start.mapped_by(uniform);
        assert!((scaled.size.as_mm() - 20.0).abs() < 1e-9);
        assert!((scaled.rotation().as_radians() - 30.0_f64.to_radians()).abs() < 1e-9);
        draws_alike(&start, &scaled, uniform);

        let stretch = Affine::scale(2.0, 1.0);
        let stretched = start.mapped_by(stretch);
        assert_eq!(stretched.size, start.size);
        draws_alike(&start, &stretched, stretch);

        let flip = Affine::scale(-1.0, 1.0);
        assert_eq!(start.mapped_by(flip).size, start.size);

        let huge = Affine::scale(1000.0, 1000.0);
        let clamped = start.mapped_by(huge);
        assert_eq!(clamped.size, MAX_TEXT_SIZE);
        draws_alike(&start, &clamped, huge);
    }

    /// Criterion 27: the first 24 characters, line breaks as spaces, an ellipsis after.
    #[test]
    fn ac27_default_name() {
        assert_eq!(
            text(10.0, Affine::IDENTITY).default_name(),
            "Hello world, this is a l…"
        );
        let mut short = text(10.0, Affine::IDENTITY);
        short.string = "Hi".to_owned();
        assert_eq!(short.default_name(), "Hi");
    }
}
