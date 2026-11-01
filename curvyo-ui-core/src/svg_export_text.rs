//! A text object and an image as elements of the SVG export (`specs/0024-svg-import-export`
//! criteria 132 to 137): a live `<text>` laid out by the same engine as the canvas, one line per
//! `tspan` (or one `textPath` on a carrier) with the per-character kerns and turns as `dx`,
//! `dy` and `rotate`; an image with its stored bytes.

use curvyo_document_core::{
    Document, FontRef, ImageSnapshot, MISSING_FACE_HASH, PathSide, TextAlign, TextChars,
    TextGlyphOrientation, TextSnapshot, TextWritingMode,
};
use curvyo_image_core::ImageFormat;
use curvyo_svg_core::{ExportImage, ExportLine, ExportText, ExportTextPath, TextAnchor};
use curvyo_text_core::{Face, FaceHash, FaceKey, FaceStyle, TextLayout};
use kurbo::{Affine, Vec2};

use crate::svg_export_leaf::export_style;
use crate::svg_export_pens::svg_pens;
use crate::svg_text_layout::ImportFaces;
use crate::text_layout::text_layout;

/// Kerns and turns smaller than this are not written (the file's own rounding).
const SHIFT_SNAP_MM: f64 = 5e-7;

/// The CSS keywords of the OS/2 width classes 1 to 9 (criterion 132).
const STRETCH: [&str; 9] = [
    "ultra-condensed",
    "extra-condensed",
    "condensed",
    "semi-condensed",
    "normal",
    "semi-expanded",
    "expanded",
    "extra-expanded",
    "ultra-expanded",
];

/// The face the canvas draws `font` with: the loaded face, else the substitute.
pub(crate) fn face_for<'a>(font: &FontRef, fonts: &ImportFaces<'a>) -> Option<&'a Face> {
    let own = (font.face_hash != MISSING_FACE_HASH)
        .then(|| FaceHash::from_hex(&font.face_hash))
        .flatten()
        .map(|hash| FaceKey {
            hash,
            index: font.face_index,
        })
        .and_then(|key| fonts.store.get(&key));
    own.or_else(|| fonts.substitute.and_then(|key| fonts.store.get(&key)))
        .map(AsRef::as_ref)
}

/// Where a text on path sits: its carrier's key and the length of the carrier's outline.
#[derive(Debug, Clone, Copy)]
pub(crate) struct OnCarrier {
    pub(crate) key: usize,
    pub(crate) length: f64,
}

/// The live text of `text` (criteria 132 to 135), its frame mapped by `t`.
pub(crate) fn live_text(
    text: &TextSnapshot,
    fonts: &ImportFaces<'_>,
    t: Affine,
    carrier: Option<OnCarrier>,
) -> ExportText {
    let face = face_for(&text.font, fonts);
    let size = text.size.as_mm();
    let cap = face.map_or(0.7, |f| f.metrics().cap_height);
    let font_size = size / cap;
    let style = FaceStyle::parse(&text.font.style).unwrap_or(FaceStyle::REGULAR);
    let values = &text.layout;
    let on_path = carrier.zip(text.on_path.as_ref());
    let anchor = if on_path.is_some() {
        TextAnchor::Middle
    } else {
        match values.align {
            TextAlign::Left | TextAlign::Justify => TextAnchor::Start,
            TextAlign::Centre => TextAnchor::Middle,
            TextAlign::Right => TextAnchor::End,
        }
    };
    let laid = face.and_then(|face| text_layout(text, face).map(|l| (face, l)));
    let letter_spacing = values.letter_spacing * size;
    let mut lines = laid.as_ref().map_or_else(
        || fallback_lines(text, font_size),
        |(face, layout)| {
            let pen = SvgPen {
                letter_spacing,
                upright_lead: upright_lead(face, font_size),
            };
            laid_lines(text, face, layout, pen)
        },
    );
    if let Some((carrier, on)) = on_path {
        lines.truncate(1);
        let position = on.values.position.as_mm();
        let below = on.values.side == PathSide::Below;
        for line in &mut lines {
            line.on_path = Some(ExportTextPath {
                carrier: carrier.key,
                start_offset: if below {
                    carrier.length - position
                } else {
                    position
                },
                side_right: below,
            });
            let distance = on.values.distance.as_mm();
            if distance != 0.0 && !line.text.is_empty() {
                line.dy.resize(line.text.chars().count(), 0.0);
                line.dy[0] -= distance;
            }
        }
    }
    let mut transform = if on_path.is_some() {
        Affine::IDENTITY
    } else {
        t * Affine::new(text.placement.to_array())
    };
    let coeffs = transform.as_coeffs();
    #[allow(clippy::float_cmp)] // only an exact translation is written as positions
    if coeffs[..4] == [1.0, 0.0, 0.0, 1.0] {
        // An unturned text writes its place as `x` and `y` (criterion 132's example).
        for line in &mut lines {
            line.x += coeffs[4];
            line.y += coeffs[5];
        }
        transform = Affine::IDENTITY;
    }
    ExportText {
        transform,
        family: text.font.family.clone(),
        weight: style.weight,
        italic: style.is_slanted(),
        stretch: STRETCH
            .get(usize::from(style.width.clamp(1, 9)) - 1)
            .copied()
            .filter(|k| *k != "normal"),
        font_size,
        anchor,
        letter_spacing,
        word_spacing: values.word_spacing * size,
        line_height: values
            .line_spacing
            .map(|f| written_line_height(f, size, font_size)),
        writing_mode: match values.writing_mode {
            TextWritingMode::Horizontal => None,
            TextWritingMode::VerticalRl => Some("vertical-rl"),
            TextWritingMode::VerticalLr => Some("vertical-lr"),
        },
        orientation: match values.orientation {
            TextGlyphOrientation::Auto => None,
            TextGlyphOrientation::Upright => Some("upright"),
            TextGlyphOrientation::Sideways => Some("sideways"),
        },
        style: export_style(&text.style, false).0,
        size,
        face: format!(
            "{}|{}|{}",
            text.font.family, text.font.style, text.font.face_hash
        ),
        lines,
    }
}

/// How much further down the import draws an upright glyph of a column than the engine's cell
/// puts it, in millimetres for the font size `font_size` (`svg_text_layout::upright_lead`).
fn upright_lead(face: &Face, font_size: f64) -> f64 {
    let m = face.metrics();
    let box_height = m.ascender - m.descender;
    if box_height <= 0.0 {
        return 0.0;
    }
    (m.ascender / box_height - m.cap_height) * font_size
}

/// Lines when no face can lay the text out: one per stored line, a line height apart.
fn fallback_lines(text: &TextSnapshot, font_size: f64) -> Vec<ExportLine> {
    text.string
        .split('\n')
        .enumerate()
        .map(|(k, line)| {
            #[allow(clippy::cast_precision_loss)] // line counts are small
            let y = k as f64 * font_size * 1.2;
            plain_line(0.0, y, line)
        })
        .collect()
}

fn plain_line(x: f64, y: f64, text: &str) -> ExportLine {
    ExportLine {
        x,
        y,
        text: text.to_owned(),
        dx: Vec::new(),
        dy: Vec::new(),
        rotate: Vec::new(),
        on_path: None,
    }
}

/// What SVG adds to a line that the canvas does not: the letter spacing after its last
/// character, and Inkscape's lead of an upright glyph in a column (criteria 86, 133 (a)).
#[derive(Debug, Clone, Copy)]
struct SvgPen {
    /// `letter-spacing` in millimetres.
    letter_spacing: f64,
    /// The import's upright lead in millimetres (`svg_text_layout::upright_lead`).
    upright_lead: f64,
}

/// The share of a line's advance its alignment puts before the anchor (criterion 133 (b)).
const fn anchor_share(align: TextAlign) -> f64 {
    match align {
        TextAlign::Left | TextAlign::Justify => 0.0,
        TextAlign::Centre => 0.5,
        TextAlign::Right => 1.0,
    }
}

/// The anchor of `line` along its baseline: where SVG, anchoring the line's chunk by its whole
/// advance (the trailing letter spacing included, unlike the canvas's extent) and adding the
/// upright lead, starts the line where the canvas does (criterion 133 (a)). `reference` is the
/// line without kerns (or `line` itself), `advance` SVG's advance of it without kerns, `out`
/// holds the line's `dx` already.
fn line_anchor(
    align: TextAlign,
    line: &curvyo_text_core::LineLayout,
    reference: &curvyo_text_core::LineLayout,
    advance: f64,
    out: &ExportLine,
    pen: SvgPen,
) -> f64 {
    let first = reference
        .glyphs
        .iter()
        .find(|g| g.cluster == reference.range.start);
    let Some(first) = first else {
        return line.start.0 + line.width.0 * anchor_share(align);
    };
    let lead = if first.upright { pen.upright_lead } else { 0.0 };
    let kerns: f64 = out.dx.iter().skip(1).sum();
    line.start.0 - lead + (advance + kerns) * anchor_share(align)
}

/// The lines of a laid-out text with its kerns, justified spacing and turns as `dx`, `dy` and
/// `rotate` against the same text laid out without them (criterion 133 (b), (e)).
fn laid_lines(
    text: &TextSnapshot,
    face: &Face,
    layout: &TextLayout,
    pen: SvgPen,
) -> Vec<ExportLine> {
    let align = text.layout.align;
    let vertical =
        text.layout.writing_mode != TextWritingMode::Horizontal && text.on_path.is_none();
    let needs_shifts = !text.chars.is_empty()
        || align == TextAlign::Justify
        || text.layout.rotation.as_radians() != 0.0;
    let natural = needs_shifts
        .then(|| {
            // The kerns are taken out, the turns kept: a turn may move the letters after it, and
            // the import measures its kerns against a layout with the turns (criterion 133 (e)).
            let mut plain = text.clone();
            plain.chars = TextChars::new(text.chars.entries().iter().map(|(at, values)| {
                (
                    *at,
                    curvyo_document_core::CharValues {
                        kern_x: 0.0,
                        kern_y: 0.0,
                        ..*values
                    },
                )
            }));
            if align == TextAlign::Justify {
                plain.layout.align = TextAlign::Left;
            }
            plain.layout.rotation = curvyo_document_core::Angle::from_radians(0.0);
            text_layout(&plain, face)
        })
        .flatten();
    layout
        .lines
        .iter()
        .enumerate()
        .map(|(k, line)| {
            let string = text.string.get(line.range.clone()).unwrap_or_default();
            let mut out = plain_line(0.0, line.baseline.0, string);
            let natural = natural.as_ref().and_then(|n| n.lines.get(k));
            let svg = natural.and_then(|_| svg_pens(text, face, line, layout.rotation_rad));
            if let Some(natural) = natural {
                let pens = svg.as_ref().map(|s| s.pens.as_slice());
                shifts(
                    line,
                    natural,
                    pens,
                    (layout.rotation_rad, !vertical),
                    &mut out,
                );
            }
            let reference = natural.unwrap_or(line);
            let advance = svg.map_or(reference.width.0 + pen.letter_spacing, |s| s.advance);
            let along = line_anchor(align, line, reference, advance, &out, pen);
            out.x = along;
            if vertical {
                // Line space runs down the column: text space is line space turned a quarter
                // (the import's frame), so along is y and across is minus x.
                out.x = -line.baseline.0;
                out.y = along;
                let (dx, dy) = (std::mem::take(&mut out.dx), std::mem::take(&mut out.dy));
                out.dx = dy.iter().map(|v| -v).collect();
                out.dy = dx;
            }
            out
        })
        .collect()
}

/// Fills `dx`, `dy` and `rotate` of `out` so that each character lands where `line` draws it
/// when SVG sets it from `natural`'s position, or from `svg_pens` where the import measures a
/// turned character apart (with `pivots`, pivots corrected: SVG turns a glyph about its origin,
/// the canvas about its centre; the import corrects no pivot in a column, criterion 87).
fn shifts(
    line: &curvyo_text_core::LineLayout,
    natural: &curvyo_text_core::LineLayout,
    svg_pens: Option<&[f64]>,
    (text_turn, pivots): (f64, bool),
    out: &mut ExportLine,
) {
    let glyph_at = |l: &curvyo_text_core::LineLayout, byte: usize| {
        l.glyphs.iter().find(|g| g.cluster == byte).copied()
    };
    let first_offset =
        glyph_at(natural, line.range.start).map_or(0.0, |g| g.origin.x - natural.start.0);
    let mut previous = Vec2::ZERO;
    let (mut dx, mut dy, mut rotate) = (Vec::new(), Vec::new(), Vec::new());
    for (index, (offset, _)) in out.text.char_indices().enumerate() {
        let byte = line.range.start + offset;
        let (Some(actual), Some(plain)) = (glyph_at(line, byte), glyph_at(natural, byte)) else {
            dx.push(0.0);
            dy.push(0.0);
            rotate.push(0.0);
            continue;
        };
        let turn = text_turn + actual.rotation_rad;
        let origin = actual.origin;
        let centre = kurbo::Point::new(actual.centre_x.0, actual.centre_y.0);
        let pivot = if pivots {
            (Affine::rotate_about(turn, centre) * origin) - origin
        } else {
            Vec2::ZERO
        };
        // SVG's pens run from the first glyph's origin, which keeps its own offset in the line.
        let set_at = svg_pens
            .and_then(|pens| pens.get(index).copied())
            .map_or(plain.origin.x - natural.start.0, |pen| pen + first_offset);
        let shift = Vec2::new(
            (origin.x - line.start.0) - set_at,
            origin.y - plain.origin.y,
        ) + pivot;
        dx.push(shift.x - previous.x);
        dy.push(shift.y - previous.y);
        rotate.push(stable_degrees(turn.to_degrees()));
        previous = shift;
    }
    let any = |v: &[f64]| v.iter().any(|x| x.abs() > SHIFT_SNAP_MM);
    if any(&dx) {
        out.dx = dx;
    }
    if any(&dy) {
        out.dy = dy;
    }
    if any(&rotate) {
        out.rotate = rotate;
    }
}

/// The most passes a written ratio or angle takes to settle (two in practice).
const SETTLE_PASSES: usize = 8;

/// The degrees next to `degrees` that come back unchanged through the radians the canvas turns
/// a character by, so the second export writes the same `rotate` (criteria 57, 152).
#[allow(clippy::float_cmp)] // a fixed point of the conversion is an exact question
fn stable_degrees(degrees: f64) -> f64 {
    let mut value = degrees;
    for _ in 0..SETTLE_PASSES {
        let back = value.to_radians().to_degrees();
        if back == value {
            break;
        }
        value = back;
    }
    value
}

/// `line-height` for the Line spacing factor `factor` (of the Size `size`): a factor of the
/// font size as the file states it (6 decimals), chosen so the import's factor gives it back
/// unchanged (criteria 57, 133 (c), 152).
#[allow(clippy::float_cmp)] // a fixed point of the conversion is an exact question
fn written_line_height(factor: f64, size: f64, font_size: f64) -> f64 {
    let stated: f64 = format!("{font_size:.6}").parse().unwrap_or(font_size);
    let mut value = factor * size / stated;
    for _ in 0..SETTLE_PASSES {
        let back = (value * stated / size) * size / stated;
        if back == value {
            break;
        }
        value = back;
    }
    value
}

/// The image `image` with its stored bytes, its map mapped by `t` (criterion 137); `None` when
/// its asset is missing or of no known format.
pub(crate) fn export_image(
    image: &ImageSnapshot,
    document: &Document,
    t: Affine,
) -> Option<ExportImage> {
    let bytes = document.asset(&image.asset)?;
    let mime = match ImageFormat::detect(&bytes)? {
        ImageFormat::Png => "image/png",
        ImageFormat::Jpeg => "image/jpeg",
        ImageFormat::WebP => "image/webp",
    };
    Some(ExportImage {
        mime,
        bytes,
        placement: t * Affine::new(image.placement.to_array()),
    })
}
