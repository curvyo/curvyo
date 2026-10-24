//! Shaping one line: an OpenType face with its own tables (`GSUB`, `GPOS`; `kern`, `liga` and
//! `calt` on, as the defaults of `HarfBuzz`; `0074` criterion 5; the optional ligatures off on
//! request, `0079` criterion 8), an SVG font one glyph per character with its `hkern` pairs
//! (`0076` criterion 14); a vertical line in upright and sideways runs (`0079` criterion 23).

use harfrust::{Buffer, Direction, Feature, ShapeOptions, ShaperFont, Tag};
use unicode_segmentation::UnicodeSegmentation;

use crate::face::{Face, Source};
use crate::svg_font::SvgFont;
use crate::writing_mode::GlyphOrientation;

/// One shaped glyph, in em units, in line space: x along the line, y across it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapedGlyph {
    /// The glyph id; 0 (`.notdef`) for a character the face lacks (criterion 6).
    pub glyph: u32,
    /// The glyph's cluster: the byte offset in the line of the first character it draws.
    pub cluster: usize,
    /// How far the pen moves along the line after this glyph.
    pub advance: f64,
    /// The glyph origin's offset from the pen along the line.
    pub x_offset: f64,
    /// The glyph origin's offset from the pen across the line, y up as the font states it (for
    /// an upright glyph of a column: to the right in text space).
    pub y_offset: f64,
    /// The glyph stands upright in a column: it does not turn with the line (criterion 23).
    pub upright: bool,
}

/// The optional ligature features a letter spacing turns off, as CSS does (`0079` criterion 8);
/// `kern` stays on.
const LIGATURE_FEATURES: [&[u8; 4]; 3] = [b"liga", b"clig", b"calt"];

/// Shapes `line` (no line break inside) left to right. Bidi and right-to-left layout are out of
/// scope (spec "Out of scope"), so the direction is fixed and clusters grow along the line.
/// Without `ligatures` the optional ligatures are not applied (an SVG font has none).
pub(crate) fn shape_line(face: &Face, line: &str, ligatures: bool) -> Vec<ShapedGlyph> {
    shape_run(face, line, ligatures, Direction::LeftToRight)
}

/// Shapes a vertical `line`: runs of grapheme clusters whose base stands upright under `orientation` are
/// shaped top to bottom (vertical advances from `vmtx`, else the line height; origins from
/// `VORG`; `vert` applied by the shaper), the others left to right and lowered by `sideways_drop`
/// (em) so the turned run is centred on the column. Clusters stay byte offsets into `line`.
pub(crate) fn shape_vertical_line(
    face: &Face,
    line: &str,
    ligatures: bool,
    orientation: GlyphOrientation,
    sideways_drop: f64,
) -> Vec<ShapedGlyph> {
    let mut glyphs = Vec::new();
    // Split per grapheme cluster: a mark, variation selector or joiner keeps its base's run.
    let upright_at = |cluster: &str| {
        cluster
            .chars()
            .next()
            .is_some_and(|c| orientation.is_upright(c))
    };
    let mut clusters = line.grapheme_indices(true).peekable();
    while let Some((start, first)) = clusters.next() {
        let upright = upright_at(first);
        let mut end = line.len();
        while let Some(&(at, cluster)) = clusters.peek() {
            if upright_at(cluster) != upright {
                end = at;
                break;
            }
            clusters.next();
        }
        let run = &line[start..end];
        let shaped = if upright {
            shape_upright(face, run, ligatures)
        } else {
            shape_line(face, run, ligatures)
        };
        glyphs.extend(shaped.into_iter().map(|mut glyph| {
            glyph.cluster += start;
            if !upright {
                glyph.y_offset -= sideways_drop;
            }
            glyph
        }));
    }
    glyphs
}

/// An upright run in line space: the shaper's top-to-bottom advance and offsets turned into
/// along-the-line and across-the-line values. An SVG font has no vertical metrics: each glyph
/// advances by the em height, its top at the pen, centred on the column (criterion 25).
fn shape_upright(face: &Face, run: &str, ligatures: bool) -> Vec<ShapedGlyph> {
    match face.source() {
        Source::OpenType { .. } => shape_run(face, run, ligatures, Direction::TopToBottom)
            .into_iter()
            .map(|glyph| ShapedGlyph {
                // Down the column is +x of line space; the font's y is up, so its negative.
                advance: -glyph.advance,
                x_offset: -glyph.y_offset,
                y_offset: glyph.x_offset,
                upright: true,
                ..glyph
            })
            .collect(),
        Source::Svg(font) => {
            let metrics = face.metrics();
            let top = metrics.ascender / (metrics.ascender - metrics.descender);
            shape_svg(font, run)
                .into_iter()
                .map(|glyph| {
                    let width = font
                        .glyphs
                        .get(usize::try_from(glyph.glyph).unwrap_or(0))
                        .map_or(0.0, |(advance, _)| *advance)
                        / font.units_per_em;
                    ShapedGlyph {
                        advance: 1.0,
                        x_offset: top,
                        y_offset: -width / 2.0,
                        upright: true,
                        ..glyph
                    }
                })
                .collect()
        }
    }
}

/// Shapes `run` with the face's own tables in `direction`.
fn shape_run(face: &Face, run: &str, ligatures: bool, direction: Direction) -> Vec<ShapedGlyph> {
    if run.is_empty() {
        return Vec::new();
    }
    let shaper = match face.source() {
        Source::OpenType { shaper, .. } => shaper,
        Source::Svg(font) => return shape_svg(font, run),
    };
    let shaper = ShaperFont::new(shaper);
    let mut buffer = Buffer::new();
    buffer.push_str(run);
    buffer.guess_segment_properties();
    buffer.set_direction(direction);
    let vertical = direction == Direction::TopToBottom;
    let off: Vec<Feature> = if ligatures {
        Vec::new()
    } else {
        LIGATURE_FEATURES
            .iter()
            .map(|tag| Feature::new(Tag::new(tag), 0, ..))
            .collect()
    };
    if harfrust::shape(&shaper, &mut buffer, ShapeOptions::new().features(&off)).is_err() {
        return Vec::new();
    }
    let scale = 1.0 / face.units_per_em();
    buffer
        .glyph_infos()
        .iter()
        .zip(buffer.glyph_positions())
        .map(|(info, position)| ShapedGlyph {
            glyph: info.glyph_id,
            cluster: usize::try_from(info.cluster).unwrap_or(usize::MAX),
            // Top to bottom: the y advance (font y up, so negative), turned by `shape_upright`.
            advance: f64::from(if vertical {
                position.y_advance
            } else {
                position.x_advance
            }) * scale,
            x_offset: f64::from(position.x_offset) * scale,
            y_offset: f64::from(position.y_offset) * scale,
            upright: false,
        })
        .collect()
}

/// One glyph per character: the `<glyph>` whose `unicode` is that character, else the missing
/// glyph (0); its advance less the kerning to the next glyph.
fn shape_svg(font: &SvgFont, line: &str) -> Vec<ShapedGlyph> {
    let scale = 1.0 / font.units_per_em;
    let glyphs: Vec<(usize, u32)> = line
        .char_indices()
        .map(|(at, c)| (at, font.glyph_of(c)))
        .collect();
    glyphs
        .iter()
        .enumerate()
        .map(|(i, &(cluster, glyph))| {
            let advance = font
                .glyphs
                .get(usize::try_from(glyph).unwrap_or(0))
                .map_or(0.0, |(advance, _)| *advance);
            let kern = glyphs
                .get(i + 1)
                .map_or(0.0, |&(_, next)| font.kerning(glyph, next));
            ShapedGlyph {
                glyph,
                cluster,
                advance: (advance - kern) * scale,
                x_offset: 0.0,
                y_offset: 0.0,
                upright: false,
            }
        })
        .collect()
}
