//! Shaping one line: an OpenType face with its own tables (`GSUB`, `GPOS`; `kern`, `liga` and
//! `calt` on, as the defaults of `HarfBuzz`; `0074` criterion 5), an SVG font one glyph per
//! character with its `hkern` pairs (`0076` criterion 14).

use harfrust::{Buffer, Direction, ShapeOptions, ShaperFont};

use crate::face::{Face, Source};
use crate::svg_font::SvgFont;

/// One shaped glyph, in em units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapedGlyph {
    /// The glyph id; 0 (`.notdef`) for a character the face lacks (criterion 6).
    pub glyph: u32,
    /// The glyph's cluster: the byte offset in the line of the first character it draws.
    pub cluster: usize,
    /// How far the pen moves after this glyph.
    pub advance: f64,
    /// The glyph's offset from the pen, x right.
    pub x_offset: f64,
    /// The glyph's offset from the pen, y up as the font states it.
    pub y_offset: f64,
}

/// Shapes `line` (no line break inside) left to right. Bidi and right-to-left layout are out of
/// scope (spec "Out of scope"), so the direction is fixed and clusters grow along the line.
pub(crate) fn shape_line(face: &Face, line: &str) -> Vec<ShapedGlyph> {
    if line.is_empty() {
        return Vec::new();
    }
    let shaper = match face.source() {
        Source::OpenType { shaper, .. } => shaper,
        Source::Svg(font) => return shape_svg(font, line),
    };
    let shaper = ShaperFont::new(shaper);
    let mut buffer = Buffer::new();
    buffer.push_str(line);
    buffer.guess_segment_properties();
    buffer.set_direction(Direction::LeftToRight);
    if harfrust::shape(&shaper, &mut buffer, ShapeOptions::new()).is_err() {
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
            advance: f64::from(position.x_advance) * scale,
            x_offset: f64::from(position.x_offset) * scale,
            y_offset: f64::from(position.y_offset) * scale,
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
            }
        })
        .collect()
}
