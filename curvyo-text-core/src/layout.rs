//! Line layout of a string in one face at one size with the text's layout values: glyph
//! positions, line metrics, the line box and the caret boundaries (`specs/0074-text-objects`
//! criteria 3, 4, 10, 25; `specs/0079-text-layout-controls` criteria 2, 3, 8 to 11, 15).
//!
//! Text space is millimetres, origin at the anchor on the first baseline, y down (ADR 0020 §1,
//! `text_placement` maps it to the document).

use std::ops::Range;

use kurbo::{BezPath, Point, Rect};

use crate::align::{Align, Extra, LineMeasure, justify, shift};
use crate::error::TextError;
use crate::face::Face;
use crate::glyph_map::glyph_to_text;
use crate::limits::MAX_SHAPE_CHARS;
use crate::shape::{ShapedGlyph, shape_line};
use crate::units::Mm;

/// The characters word spacing widens (`0079` criterion 9).
const SPACES: [char; 3] = [' ', '\u{a0}', '\u{3000}'];

/// The layout values of a text, in millimetres and radians (`0079` criterion 7). The default is
/// the layout of `0074`: left-aligned, no added spacing, the face's line spacing, upright glyphs.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LayoutOptions {
    /// How the lines sit on the anchor's x.
    pub align: Align,
    /// Extra width after each cluster except the last of a line.
    pub letter_spacing: Mm,
    /// Extra width added to each space character.
    pub word_spacing: Mm,
    /// The distance from one baseline to the next; `None` is the face's natural line spacing.
    pub line_spacing: Option<Mm>,
    /// The turn of every glyph about its cluster's centre, clockwise on screen.
    pub rotation_rad: f64,
}

/// One glyph placed in text space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    /// The glyph id in the face.
    pub glyph: u32,
    /// Where the glyph's origin sits, in text space (mm).
    pub origin: Point,
    /// The byte offset in the whole string of the first character this glyph draws.
    pub cluster: usize,
    /// The x of the middle of its cluster's advance: the glyph turns about it (criterion 11).
    pub centre_x: Mm,
}

/// One line of the layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LineLayout {
    /// The line's bytes in the whole string, without its line break.
    pub range: Range<usize>,
    /// The baseline's y in text space (the first line's is 0).
    pub baseline: Mm,
    /// Where the pen starts, after the alignment's shift.
    pub start: Mm,
    /// The sum of the advances with the added spacing, from `start`.
    pub width: Mm,
    /// The line extent: from the left edge of its first cluster to the right edge of its last
    /// cluster that is not a space (spec "Terms"), after the shift.
    pub extent: Range<Mm>,
    /// The glyphs in drawing order.
    pub glyphs: Vec<PlacedGlyph>,
    /// The caret positions: byte offsets in the whole string and their x, sorted by byte.
    /// Shaper cluster boundaries, so a ligature or a combining mark is never split
    /// (`specs/0074-text-objects/adrs.md` decision 3); between two clusters in the middle of the
    /// letter spacing (`0079` criterion 15).
    pub boundaries: Vec<(usize, Mm)>,
}

/// A laid-out text.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout {
    /// Millimetres per em: the size divided by the face's cap height.
    pub mm_per_em: f64,
    /// The size, the height of the capital H.
    pub size: Mm,
    /// The glyphs' turn (criterion 11), radians, clockwise on screen.
    pub rotation_rad: f64,
    /// The line's top above the baseline (positive).
    pub ascender: Mm,
    /// The line's bottom below the baseline (positive).
    pub descender: Mm,
    /// The distance between baselines as used.
    pub line_height: Mm,
    /// The face's own line spacing at this size (Line spacing Auto, `0079` criterion 10).
    pub natural_line_height: Mm,
    /// The lines, top first; an empty string has one empty line.
    pub lines: Vec<LineLayout>,
}

/// Lays out `text` in `face` so that the capital H is `size` high (spec "Terms", Size), with the
/// layout values of `options`. Lines break at `\n` only; each next baseline is one line spacing
/// lower (criterion 4).
///
/// # Errors
/// [`TextError::TextTooLong`] for more than [`MAX_SHAPE_CHARS`] characters.
pub fn layout(
    face: &Face,
    text: &str,
    size: Mm,
    options: &LayoutOptions,
) -> Result<TextLayout, TextError> {
    if text.chars().count() > MAX_SHAPE_CHARS {
        return Err(TextError::TextTooLong);
    }
    let metrics = face.metrics();
    let mm_per_em = size.get() / metrics.cap_height;
    let natural = metrics.line_height() * mm_per_em;
    let line_height = options.line_spacing.map_or(natural, Mm::get);
    let ligatures = options.letter_spacing.0 == 0.0;
    let mut shaped = Vec::new();
    let mut start = 0;
    for line in text.split('\n') {
        shaped.push(ShapedLine::new(face, line, start, ligatures));
        start += line.len() + 1;
    }
    let spacing = Spacing {
        mm_per_em,
        letter: options.letter_spacing.0,
        word: options.word_spacing.0,
    };
    let mut lines: Vec<LineLayout> = shaped
        .iter()
        .enumerate()
        .map(|(index, line)| line.place(line_height * f64_from(index), spacing, Extra::default()))
        .collect();
    if options.align == Align::Justify {
        let measures: Vec<LineMeasure> = shaped
            .iter()
            .zip(&lines)
            .map(|(line, placed)| line.measure(placed))
            .collect();
        for (index, extra) in justify(&measures).into_iter().enumerate() {
            if extra != Extra::default() {
                lines[index] = shaped[index].place(line_height * f64_from(index), spacing, extra);
            }
        }
    }
    for line in &mut lines {
        let dx = shift(options.align, line.extent.start, line.extent.end);
        line.shift(dx);
    }
    Ok(TextLayout {
        mm_per_em,
        size,
        rotation_rad: options.rotation_rad,
        ascender: Mm(metrics.ascender * mm_per_em),
        descender: Mm(-metrics.descender * mm_per_em),
        line_height: Mm(line_height),
        natural_line_height: Mm(natural),
        lines,
    })
}

fn f64_from(index: usize) -> f64 {
    f64::from(u32::try_from(index).unwrap_or(u32::MAX))
}

/// The added spacing of a placement, mm.
#[derive(Debug, Clone, Copy)]
struct Spacing {
    mm_per_em: f64,
    letter: f64,
    word: f64,
}

/// One cluster of a shaped line: its glyphs, its byte in the line and its advance in em.
#[derive(Debug, Clone)]
struct Cluster {
    glyphs: Range<usize>,
    byte: usize,
    advance: f64,
    space: bool,
}

/// A line shaped once, kept so Justify places it again without shaping twice.
#[derive(Debug, Clone)]
struct ShapedLine {
    range: Range<usize>,
    glyphs: Vec<ShapedGlyph>,
    clusters: Vec<Cluster>,
}

impl ShapedLine {
    fn new(face: &Face, line: &str, start: usize, ligatures: bool) -> Self {
        let glyphs = shape_line(face, line, ligatures);
        let mut clusters: Vec<Cluster> = Vec::new();
        for (index, glyph) in glyphs.iter().enumerate() {
            let byte = glyph.cluster.min(line.len());
            match clusters.last_mut() {
                // Left to right, so a cluster seen before is the last one pushed.
                Some(last) if last.byte == byte => {
                    last.glyphs.end = index + 1;
                    last.advance += glyph.advance;
                }
                _ => clusters.push(Cluster {
                    glyphs: index..index + 1,
                    byte,
                    advance: glyph.advance,
                    space: line[byte..].starts_with(SPACES),
                }),
            }
        }
        Self {
            range: start..start + line.len(),
            glyphs,
            clusters,
        }
    }

    /// The index after the last cluster that is not a space.
    fn extent_end(&self) -> usize {
        self.clusters
            .iter()
            .rposition(|cluster| !cluster.space)
            .map_or(0, |index| index + 1)
    }

    fn measure(&self, placed: &LineLayout) -> LineMeasure {
        let inside = self.extent_end();
        LineMeasure {
            width: placed.extent.end.0 - placed.extent.start.0,
            spaces: self.clusters[..inside].iter().filter(|c| c.space).count(),
            clusters: inside,
        }
    }

    /// The line placed from x = 0 on `baseline` with `spacing` and the justify `extra`.
    fn place(&self, baseline: f64, spacing: Spacing, extra: Extra) -> LineLayout {
        let inside = self.extent_end();
        let start = self.range.start;
        let mut x = 0.0;
        let mut extent_end = 0.0;
        let mut glyphs = Vec::with_capacity(self.glyphs.len());
        let mut boundaries = vec![(start, Mm(0.0))];
        for (index, cluster) in self.clusters.iter().enumerate() {
            if index > 0 {
                let gap = spacing.letter + if index < inside { extra.per_gap } else { 0.0 };
                boundaries.push((start + cluster.byte, Mm(x + gap / 2.0)));
                x += gap;
            }
            let advance = cluster.advance * spacing.mm_per_em;
            let centre_x = Mm(x + advance / 2.0);
            let mut pen = x;
            for shaped in &self.glyphs[cluster.glyphs.clone()] {
                glyphs.push(PlacedGlyph {
                    glyph: shaped.glyph,
                    origin: Point::new(
                        pen + shaped.x_offset * spacing.mm_per_em,
                        baseline - shaped.y_offset * spacing.mm_per_em,
                    ),
                    cluster: start + cluster.byte,
                    centre_x,
                });
                pen += shaped.advance * spacing.mm_per_em;
            }
            x += advance;
            if cluster.space {
                x += spacing.word + if index < inside { extra.per_space } else { 0.0 };
            }
            if index < inside {
                extent_end = x;
            }
        }
        boundaries.push((self.range.end, Mm(x)));
        // Stable: for a byte seen twice the first x is kept.
        boundaries.sort_by_key(|(byte, _)| *byte);
        boundaries.dedup_by_key(|(byte, _)| *byte);
        LineLayout {
            range: self.range.clone(),
            baseline: Mm(baseline),
            start: Mm(0.0),
            width: Mm(x),
            extent: Mm(0.0)..Mm(extent_end),
            glyphs,
            boundaries,
        }
    }
}

impl LineLayout {
    /// Moves the line by `dx` along its baseline: glyphs, extent and caret boundaries alike, so
    /// caret and drawing never disagree.
    fn shift(&mut self, dx: f64) {
        if dx == 0.0 {
            return;
        }
        self.start.0 += dx;
        self.extent = Mm(self.extent.start.0 + dx)..Mm(self.extent.end.0 + dx);
        for glyph in &mut self.glyphs {
            glyph.origin.x += dx;
            glyph.centre_x.0 += dx;
        }
        for (_, x) in &mut self.boundaries {
            x.0 += dx;
        }
    }
}

impl TextLayout {
    /// The box of the lines in text space: from the first line's ascender to the last line's
    /// descender, from the leftmost line start to the rightmost line end (criterion 25; with
    /// Centre and Right lines start at negative x).
    #[must_use]
    pub fn line_box(&self) -> Rect {
        let (left, right) =
            self.lines
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(left, right), line| {
                    (
                        left.min(line.start.0),
                        right.max(line.start.0 + line.width.0),
                    )
                });
        let (left, right) = if left.is_finite() {
            (left, right)
        } else {
            (0.0, 0.0)
        };
        let last = self.lines.last().map_or(0.0, |line| line.baseline.0);
        Rect::new(left, -self.ascender.0, right, last + self.descender.0)
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
            .flat_map(|line| {
                line.glyphs
                    .iter()
                    .map(move |placed| (line.baseline, placed))
            })
            .filter_map(|(baseline, placed)| {
                let outline = face.outline(placed.glyph);
                if outline.path.elements().is_empty() {
                    return None;
                }
                let to_text = glyph_to_text(
                    placed,
                    baseline.0,
                    self.mm_per_em,
                    self.size.0,
                    self.rotation_rad,
                );
                Some(to_text * &outline.path)
            })
            .collect()
    }
}
