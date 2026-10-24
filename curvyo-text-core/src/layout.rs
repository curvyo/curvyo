//! Line layout of a string in one face at one size with the text's layout values: glyph
//! positions, line metrics, the line box and the caret boundaries (`specs/0074-text-objects`
//! criteria 3, 4, 10, 25; `specs/0079-text-layout-controls` criteria 2, 3, 8 to 11, 15, 22 to 25).
//!
//! Text space is millimetres, origin at the anchor on the first baseline, y down (ADR 0020 §1,
//! `text_placement` maps it to the document). Lines are laid out in line space: x along the line,
//! y across it; for horizontal text it is text space, for vertical text
//! [`WritingMode::frame`] turns it a quarter clockwise, so a line becomes a column whose centre
//! line is its baseline.

use std::ops::Range;

use kurbo::{Affine, BezPath, Point, Rect};

use crate::align::{Align, Extra, LineMeasure, justify, shift};
use crate::char_offsets::{ByteOffsets, CharOffset, Offset};
use crate::error::TextError;
use crate::face::Face;
use crate::glyph_map::glyph_to_text;
use crate::limits::MAX_SHAPE_CHARS;
use crate::shape::{ShapedGlyph, shape_line, shape_vertical_line};
use crate::units::Mm;
use crate::writing_mode::{GlyphOrientation, WritingMode};

/// The characters word spacing widens (`0079` criterion 9).
const SPACES: [char; 3] = [' ', '\u{a0}', '\u{3000}'];

/// The layout values of a text, in millimetres and radians (`0079` criterion 7). The default is
/// the layout of `0074`: left-aligned, no added spacing, the face's line spacing, upright glyphs.
#[derive(Debug, Clone, PartialEq, Default)]
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
    /// Horizontal lines or vertical columns.
    pub writing_mode: WritingMode,
    /// How the glyphs of a column stand; horizontal text ignores it.
    pub orientation: GlyphOrientation,
    /// Offsets of single characters (kerning, ADR 0021 §6), applied to the cluster a character
    /// starts; any order.
    pub char_offsets: Vec<CharOffset>,
}

/// One glyph placed in line space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedGlyph {
    /// The glyph id in the face.
    pub glyph: u32,
    /// Where the glyph's origin sits, in line space (mm).
    pub origin: Point,
    /// The byte offset in the whole string of the first character this glyph draws.
    pub cluster: usize,
    /// The x of the middle of its cluster's advance: the glyph turns about it (criterion 11).
    pub centre_x: Mm,
    /// The line-space y of the point the glyph turns about: half the size above its baseline,
    /// or for an upright glyph of a column the column's centre line.
    pub centre_y: Mm,
    /// The line-space y of its cluster's baseline, moved across by the offsets up to it: the
    /// letter's caret and box stand on it (ADR 0021 §6).
    pub baseline: Mm,
    /// The glyph stands upright in a column: it is drawn unturned at its mapped origin
    /// (`0079` criterion 23); every other glyph turns with the line.
    pub upright: bool,
    /// Its character's own turn (ADR 0021), added to the text's [`TextLayout::rotation_rad`].
    pub rotation_rad: f64,
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
    /// Horizontal lines or vertical columns; [`Self::frame`] maps line space to text space.
    pub writing_mode: WritingMode,
    /// The line's top above the baseline (positive); a column's half width.
    pub ascender: Mm,
    /// The line's bottom below the baseline (positive); a column's half width.
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
/// lower (criterion 4), or each next column one line spacing further left or right.
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
    let mode = options.writing_mode;
    let vertical = mode.is_vertical();
    let natural = metrics.auto_line_height(vertical) * mm_per_em;
    let line_height = options.line_spacing.map_or(natural, Mm::get);
    let ligatures = options.letter_spacing.0 == 0.0;
    // A sideways run's baseline lies so that its line (ascender to descender) is centred on the
    // column (criterion 23).
    let drop = f64::midpoint(metrics.ascender, metrics.descender);
    let mut shaped = Vec::new();
    let mut start = 0;
    for line in text.split('\n') {
        let glyphs = if vertical {
            shape_vertical_line(face, line, ligatures, options.orientation, drop)
        } else {
            shape_line(face, line, ligatures)
        };
        shaped.push(ShapedLine::new(glyphs, line, start));
        start += line.len() + 1;
    }
    let offsets = ByteOffsets::new(text, &options.char_offsets);
    let spacing = Spacing {
        mm_per_em,
        letter: options.letter_spacing.0,
        word: options.word_spacing.0,
        pivot: if vertical { drop * mm_per_em } else { 0.0 } - size.get() / 2.0,
        offsets: &offsets,
    };
    let baseline = |index: usize| mode.baseline(f64_from(index), line_height);
    let mut lines: Vec<LineLayout> = shaped
        .iter()
        .enumerate()
        .map(|(index, line)| line.place(baseline(index), spacing, Extra::default()))
        .collect();
    if options.align == Align::Justify {
        let measures: Vec<LineMeasure> = shaped
            .iter()
            .zip(&lines)
            .map(|(line, placed)| line.measure(placed))
            .collect();
        for (index, extra) in justify(&measures).into_iter().enumerate() {
            if extra != Extra::default() {
                lines[index] = shaped[index].place(baseline(index), spacing, extra);
            }
        }
    }
    for line in &mut lines {
        let dx = shift(options.align, line.extent.start, line.extent.end);
        line.shift(dx);
    }
    let (ascender, descender) = if vertical {
        let half = (metrics.ascender - metrics.descender) / 2.0;
        (half, half)
    } else {
        (metrics.ascender, -metrics.descender)
    };
    Ok(TextLayout {
        mm_per_em,
        size,
        rotation_rad: options.rotation_rad,
        writing_mode: mode,
        ascender: Mm(ascender * mm_per_em),
        descender: Mm(descender * mm_per_em),
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
struct Spacing<'a> {
    mm_per_em: f64,
    letter: f64,
    word: f64,
    /// The turning point's y from the baseline of a glyph that turns with the line.
    pivot: f64,
    /// The per-character offsets.
    offsets: &'a ByteOffsets,
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
    fn new(glyphs: Vec<ShapedGlyph>, line: &str, start: usize) -> Self {
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
        let mut extent_start = 0.0;
        let mut extent_end = 0.0;
        let mut glyphs = Vec::with_capacity(self.glyphs.len());
        let mut boundaries = vec![(start, Mm(0.0))];
        // The sum of the across offsets so far: each moves the rest of the line (SVG `dy`).
        let mut rise = 0.0;
        for (index, cluster) in self.clusters.iter().enumerate() {
            let Offset {
                along,
                across,
                rotation_rad,
            } = spacing.offsets.at(start + cluster.byte);
            if index > 0 {
                let gap = spacing.letter + along + if index < inside { extra.per_gap } else { 0.0 };
                boundaries.push((start + cluster.byte, Mm(x + gap / 2.0)));
                x += gap;
            } else {
                // A kern on the first cluster moves the line from the anchor; the caret before
                // it sits in the middle of the gap.
                boundaries[0].1 = Mm(along / 2.0);
                x += along;
                extent_start = x;
            }
            rise += across;
            let baseline = baseline - rise;
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
                    centre_y: Mm(if shaped.upright {
                        baseline
                    } else {
                        baseline + spacing.pivot
                    }),
                    baseline: Mm(baseline),
                    upright: shaped.upright,
                    rotation_rad,
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
            extent: Mm(extent_start)..Mm(extent_end.max(extent_start)),
            glyphs,
            boundaries,
        }
    }
}

impl TextLayout {
    /// The map from line space to text space.
    #[must_use]
    pub const fn frame(&self) -> Affine {
        self.writing_mode.frame()
    }

    /// The box of the lines in text space: from the first line's ascender to the last line's
    /// descender, from the leftmost line start to the rightmost line end (criterion 25; with
    /// Centre and Right lines start at negative x), turned with the frame for columns.
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
        let (top, bottom) = self
            .lines
            .iter()
            .fold((0.0_f64, 0.0_f64), |(top, bottom), line| {
                (top.min(line.baseline.0), bottom.max(line.baseline.0))
            });
        let lines = Rect::new(
            left,
            top - self.ascender.0,
            right,
            bottom + self.descender.0,
        );
        self.frame().transform_rect_bbox(lines)
    }

    /// The line that holds caret position `byte` (the last line for a position past the end).
    #[must_use]
    pub fn line_of(&self, byte: usize) -> usize {
        self.lines
            .iter()
            .position(|line| byte <= line.range.end)
            .unwrap_or(self.lines.len().saturating_sub(1))
    }

    /// The caret's x and baseline in line space for caret position `byte`, which is snapped to
    /// the nearest boundary at or before it.
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
    /// boundary by x, both in line space (criterion 10).
    #[must_use]
    pub fn nearest_boundary(&self, point: Point) -> usize {
        let point = self.frame().inverse() * point;
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

    /// The boundary of line `index` nearest to line-space `x`.
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
                let turn = self.rotation_rad + placed.rotation_rad;
                let to_text = glyph_to_text(placed, self.frame(), self.mm_per_em, turn);
                Some(to_text * &outline.path)
            })
            .collect()
    }
}
