//! Left-aligned line layout of a string in one face at one size: glyph positions, line metrics,
//! the line box and the caret boundaries (spec criteria 3, 4, 10, 25).
//!
//! Text space is millimetres, origin at the left end of the first baseline, y down (ADR 0020 §1,
//! `text_placement` maps it to the document).

use std::ops::Range;

use kurbo::{Affine, BezPath, Point, Rect};

use crate::error::TextError;
use crate::face::Face;
use crate::limits::MAX_SHAPE_CHARS;
use crate::shape::shape_line;
use crate::units::Mm;

/// One glyph placed in text space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    /// The glyph id in the face.
    pub glyph: u32,
    /// Where the glyph's origin sits, in text space (mm).
    pub origin: Point,
    /// The byte offset in the whole string of the first character this glyph draws.
    pub cluster: usize,
}

/// One line of the layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LineLayout {
    /// The line's bytes in the whole string, without its line break.
    pub range: Range<usize>,
    /// The baseline's y in text space (the first line's is 0).
    pub baseline: Mm,
    /// The sum of the advances.
    pub width: Mm,
    /// The glyphs in drawing order.
    pub glyphs: Vec<PlacedGlyph>,
    /// The caret positions: byte offsets in the whole string and their x, sorted by byte.
    /// Shaper cluster boundaries, so a ligature or a combining mark is never split
    /// (`specs/0074-text-objects/adrs.md` decision 3).
    pub boundaries: Vec<(usize, Mm)>,
}

/// A laid-out text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    /// Millimetres per em: the size divided by the face's cap height.
    pub mm_per_em: f64,
    /// The line's top above the baseline (positive).
    pub ascender: Mm,
    /// The line's bottom below the baseline (positive).
    pub descender: Mm,
    /// The distance between baselines.
    pub line_height: Mm,
    /// The lines, top first; an empty string has one empty line.
    pub lines: Vec<LineLayout>,
}

/// Lays out `text` in `face` so that the capital H is `size` high (spec "Terms", Size). Lines
/// break at `\n` only and are left-aligned at x = 0; each next baseline is one line height lower
/// (criterion 4).
///
/// # Errors
/// [`TextError::TextTooLong`] for more than [`MAX_SHAPE_CHARS`] characters.
pub fn layout(face: &Face, text: &str, size: Mm) -> Result<TextLayout, TextError> {
    if text.chars().count() > MAX_SHAPE_CHARS {
        return Err(TextError::TextTooLong);
    }
    let metrics = face.metrics();
    let mm_per_em = size.get() / metrics.cap_height;
    let line_height = metrics.line_height() * mm_per_em;
    let mut lines = Vec::new();
    let mut start = 0;
    for (index, line) in text.split('\n').enumerate() {
        let baseline = line_height * f64_from(index);
        lines.push(layout_line(face, line, start, baseline, mm_per_em));
        start += line.len() + 1;
    }
    Ok(TextLayout {
        mm_per_em,
        ascender: Mm(metrics.ascender * mm_per_em),
        descender: Mm(-metrics.descender * mm_per_em),
        line_height: Mm(line_height),
        lines,
    })
}

fn f64_from(index: usize) -> f64 {
    f64::from(u32::try_from(index).unwrap_or(u32::MAX))
}

fn layout_line(face: &Face, line: &str, start: usize, baseline: f64, mm_per_em: f64) -> LineLayout {
    let mut x = 0.0;
    let mut glyphs = Vec::new();
    let mut boundaries = vec![(start, Mm(0.0))];
    for shaped in shape_line(face, line) {
        let cluster = start + shaped.cluster.min(line.len());
        // Left to right, so a cluster seen before is the last one pushed.
        if boundaries.last().is_none_or(|(byte, _)| *byte != cluster) {
            boundaries.push((cluster, Mm(x)));
        }
        glyphs.push(PlacedGlyph {
            glyph: shaped.glyph,
            origin: Point::new(
                x + shaped.x_offset * mm_per_em,
                baseline - shaped.y_offset * mm_per_em,
            ),
            cluster,
        });
        x += shaped.advance * mm_per_em;
    }
    let end = start + line.len();
    boundaries.push((end, Mm(x)));
    // Stable: for a byte seen twice the first x is kept.
    boundaries.sort_by_key(|(byte, _)| *byte);
    boundaries.dedup_by_key(|(byte, _)| *byte);
    LineLayout {
        range: start..end,
        baseline: Mm(baseline),
        width: Mm(x),
        glyphs,
        boundaries,
    }
}

impl TextLayout {
    /// The box of the lines in text space: from the first line's ascender to the last line's
    /// descender, from x = 0 to the end of the longest line (criterion 25).
    #[must_use]
    pub fn line_box(&self) -> Rect {
        let width = self
            .lines
            .iter()
            .map(|line| line.width.0)
            .fold(0.0, f64::max);
        let last = self.lines.last().map_or(0.0, |line| line.baseline.0);
        Rect::new(0.0, -self.ascender.0, width, last + self.descender.0)
    }

    /// The line that holds caret position `byte` (the last line for a position past the end).
    #[must_use]
    pub fn line_of(&self, byte: usize) -> usize {
        self.lines
            .iter()
            .position(|line| byte <= line.range.end)
            .unwrap_or(self.lines.len().saturating_sub(1))
    }

    /// The caret's x and baseline for caret position `byte`, which is snapped to the nearest
    /// boundary at or before it.
    #[must_use]
    pub fn caret_point(&self, byte: usize) -> Point {
        let Some(line) = self.lines.get(self.line_of(byte)) else {
            return Point::ZERO;
        };
        let x = line
            .boundaries
            .iter()
            .rev()
            .find(|(at, _)| *at <= byte)
            .map_or(0.0, |(_, x)| x.0);
        Point::new(x, line.baseline.0)
    }

    /// The caret position nearest to `point` (text space): the line by y, then the nearest
    /// boundary by x (criterion 10).
    #[must_use]
    pub fn nearest_boundary(&self, point: Point) -> usize {
        let middle = (self.descender.0 - self.ascender.0) / 2.0;
        let index = self
            .lines
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                (a.baseline.0 + middle - point.y)
                    .abs()
                    .total_cmp(&(b.baseline.0 + middle - point.y).abs())
            })
            .map_or(0, |(index, _)| index);
        self.boundary_on_line(index, point.x)
    }

    /// The boundary of line `index` nearest to `x`.
    #[must_use]
    pub fn boundary_on_line(&self, index: usize, x: f64) -> usize {
        self.lines.get(index).map_or(0, |line| {
            line.boundaries
                .iter()
                .min_by(|a, b| (a.1.0 - x).abs().total_cmp(&(b.1.0 - x).abs()))
                .map_or(line.range.start, |(byte, _)| *byte)
        })
    }

    /// The next caret position after `byte`, or `byte` at the end.
    #[must_use]
    pub fn next_boundary(&self, byte: usize) -> usize {
        self.all_boundaries().find(|at| *at > byte).unwrap_or(byte)
    }

    /// The caret position before `byte`, or `byte` at the start.
    #[must_use]
    pub fn previous_boundary(&self, byte: usize) -> usize {
        self.all_boundaries()
            .filter(|at| *at < byte)
            .last()
            .unwrap_or(byte)
    }

    fn all_boundaries(&self) -> impl Iterator<Item = usize> + '_ {
        self.lines
            .iter()
            .flat_map(|line| line.boundaries.iter().map(|(byte, _)| *byte))
    }

    /// Every glyph's outline in text space (mm), one path per drawn glyph, in drawing order;
    /// glyphs without contours (spaces) give no path (criterion 22).
    #[must_use]
    pub fn glyph_paths(&self, face: &Face) -> Vec<BezPath> {
        self.lines
            .iter()
            .flat_map(|line| line.glyphs.iter())
            .filter_map(|placed| {
                let outline = face.outline(placed.glyph);
                if outline.path.elements().is_empty() {
                    return None;
                }
                let to_text =
                    Affine::translate(placed.origin.to_vec2()) * Affine::scale(self.mm_per_em);
                Some(to_text * &outline.path)
            })
            .collect()
    }
}
