//! Shaping one line with the face's own tables (`GSUB`, `GPOS`; `kern`, `liga` and `calt` on, as
//! the defaults of `HarfBuzz`; spec criterion 5).

use harfrust::{Buffer, Direction, ShapeOptions, ShaperFont};

use crate::face::Face;

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
    let shaper = ShaperFont::new(face.shaper());
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
