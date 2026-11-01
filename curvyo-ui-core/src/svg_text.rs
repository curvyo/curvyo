//! An imported text as Curvyo text objects (`specs/0024-svg-import-export` criteria 83 to 95,
//! Frage 48 default A): the characters split where one text object cannot hold them (another
//! face, size, paint, writing mode or text path; a kern out of range; 10,000 characters), each
//! piece laid out in its face and every character moved by per-character kerns to where SVG
//! draws it, the alignment kept where the piece is whole lines of one anchor.

use curvyo_document_core::{
    Affine, CharValues, DrawingText, KERN_X_FACTORS, KERN_Y_FACTORS, LETTER_SPACING_FACTORS,
    LINE_SPACING_FACTORS, Length, MAX_TEXT_CHARS, OnPathValues, PathSide, TextAlign, TextChars,
    TextGlyphOrientation, TextLayoutValues, TextSnapshot, TextWritingMode, WORD_SPACING_FACTORS,
};
use curvyo_svg_core::{LineHeight, SvgChar, SvgOrientation, SvgText, SvgWritingMode, TextAnchor};
use curvyo_text_core::TextLayout;
use kurbo::{Point, Rect, Vec2};

use crate::svg_font_match::FaceMemo;
use crate::svg_text_flow::flowed;

use crate::svg_text_layout::{
    ImportFaces, Resolved, Targets, advances, line_height_user, resolve, resolve_by_reference,
    spacing, targets,
};

/// The smallest kern written, mm: below it the file's own rounding.
const KERN_SNAP_MM: f64 = 5e-4;

/// One text object made from an imported text.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextPiece {
    pub(crate) text: DrawingText,
    /// The carrier's leaf number and the values on it, for a text on path.
    pub(crate) on_path: Option<(usize, OnPathValues)>,
    /// Its letters' box in the file's millimetres (`None` on a path).
    pub(crate) bounds: Option<Rect>,
}

/// How far the scale of a text's matrix may be from 1 and still count as 1 (rounding of a turn).
const UNIT_SCALE_TOLERANCE: f64 = 1e-12;

/// The text objects of `text`, in order; empty when no face can draw it or it skips (a matrix
/// without area, criterion 84).
pub(crate) fn convert_text(
    text: &SvgText,
    fonts: &ImportFaces<'_>,
    memo: &FaceMemo<'_>,
) -> Vec<TextPiece> {
    let s = text.matrix.determinant().abs().sqrt();
    // A turn or a flip has a factor of 1 within rounding: exactly 1, so a turned text keeps its
    // Size to the last digit (criterion 152, as `0074`'s own rule for the size register).
    let s = if (s - 1.0).abs() <= UNIT_SCALE_TOLERANCE {
        1.0
    } else {
        s
    };
    if !(s.is_finite() && s > 0.0) {
        return Vec::new();
    }
    let resolved: Vec<Option<Resolved<'_>>> = text
        .styles
        .iter()
        .map(|style| {
            text.face
                .as_deref()
                .and_then(|face| resolve_by_reference(style, (face, text.size), fonts, s))
                .or_else(|| resolve(style, fonts, memo, s))
        })
        .collect();
    // A text path whose target is not drawn is straight (criterion 95).
    let mut chars: Vec<SvgChar> = text
        .chars
        .iter()
        .map(|c| SvgChar {
            path: c.path.filter(|p| text.paths[*p].carrier.is_some()),
            ..*c
        })
        .collect();
    let mut advances = advances(text, &chars, &resolved, s);
    let extra_spacing = stretch_to_length(text, &chars, &mut advances);
    let mut anchor = None;
    let mut justify = false;
    if let Some((flow_chars, flow_anchor, flow_justify)) = flowed(text, &advances, &resolved) {
        chars = flow_chars;
        anchor = Some(flow_anchor);
        justify = flow_justify;
    }
    let targets = targets(text, &chars, &advances, &resolved, anchor);
    let context = Context {
        text,
        chars: &chars,
        resolved: &resolved,
        targets: &targets,
        s,
        anchor,
        justify,
        extra_spacing,
    };
    let mut pieces = Vec::new();
    for run in context.runs() {
        let mut queue = vec![run];
        while let Some(chars) = queue.pop() {
            match context.piece(&chars) {
                Outcome::Piece(piece) => pieces.push(*piece),
                Outcome::Split(points) => {
                    let mut ends: Vec<usize> = points;
                    ends.push(chars.len());
                    let mut start = 0;
                    let mut parts = Vec::with_capacity(ends.len());
                    for end in ends {
                        parts.push(chars[start..end].to_vec());
                        start = end;
                    }
                    queue.extend(parts.into_iter().rev());
                }
                Outcome::Skip => {}
            }
        }
    }
    pieces
}

/// Widens (or narrows) the gaps between the characters so that the text's extent is its
/// `textLength` (criterion 88: `lengthAdjust` spacing and spacingAndGlyphs alike, the glyphs keep
/// their shape); returns the added spacing per gap in user units, 0 without `textLength`.
fn stretch_to_length(text: &SvgText, chars: &[SvgChar], advances: &mut [f64]) -> f64 {
    let Some(length) = text.text_length else {
        return 0.0;
    };
    let real: Vec<usize> = (0..chars.len()).filter(|&i| chars[i].ch != '\n').collect();
    let Some((&last, gaps)) = real.split_last() else {
        return 0.0;
    };
    if gaps.is_empty() {
        return 0.0;
    }
    let extent = gaps
        .iter()
        .map(|&i| advances[i] + spacing(&text.styles[chars[i].style], chars[i].ch))
        .sum::<f64>()
        + advances[last];
    #[allow(clippy::cast_precision_loss)] // at most 1,000,000 characters
    let extra = (length - extent) / gaps.len() as f64;
    if !extra.is_finite() {
        return 0.0;
    }
    for &i in gaps {
        advances[i] += extra;
    }
    extra
}

/// What became of a run of characters.
enum Outcome {
    /// One text object.
    Piece(Box<TextPiece>),
    /// Kerns fall out of range at these characters (criterion 92 (b)), never the first: each
    /// starts a new piece.
    Split(Vec<usize>),
    /// The engine cannot lay it out.
    Skip,
}

/// What every piece of one text reads.
pub(crate) struct Context<'t, 'f> {
    pub(crate) text: &'t SvgText,
    pub(crate) chars: &'t [SvgChar],
    pub(crate) resolved: &'t [Option<Resolved<'f>>],
    pub(crate) targets: &'t Targets,
    pub(crate) s: f64,
    pub(crate) anchor: Option<TextAnchor>,
    pub(crate) justify: bool,
    /// The letter spacing `textLength` adds, user units (criterion 88).
    pub(crate) extra_spacing: f64,
}

/// The positions of a piece's characters in its natural layout.
struct Natural {
    /// Per piece character: the glyph origin and turning centre in line space, its line and that
    /// line's baseline.
    at: Vec<Option<(Point, Point, usize, f64)>>,
    /// The layout itself, kept for a piece that needs no kern and is set left.
    layout: TextLayout,
}

impl Context<'_, '_> {
    /// The runs of characters one text object may hold by criterion 92 (a) and (c).
    fn runs(&self) -> Vec<Vec<usize>> {
        let mut runs: Vec<Vec<usize>> = Vec::new();
        let mut last: Option<usize> = None;
        for (i, c) in self.chars.iter().enumerate() {
            if c.ch == '\n' || self.resolved[c.style].is_none() {
                continue;
            }
            let same = last.is_some_and(|l| self.same_object(l, i));
            match runs.last_mut() {
                Some(run) if same && run.len() < MAX_TEXT_CHARS => run.push(i),
                _ => runs.push(vec![i]),
            }
            last = Some(i);
        }
        runs
    }

    /// Whether characters `a` and `b` can share one text object (criterion 92 (a)).
    fn same_object(&self, a: usize, b: usize) -> bool {
        let (ca, cb) = (self.chars[a], self.chars[b]);
        let (Some(ra), Some(rb)) = (&self.resolved[ca.style], &self.resolved[cb.style]) else {
            return false;
        };
        ca.path == cb.path
            && ra.font == rb.font
            && ra.size == rb.size
            && ra.style == rb.style
            && ra.writing_mode == rb.writing_mode
            && ra.orientation == rb.orientation
    }

    /// The text object of the characters `chars`, aligned when the file allows it and it lands.
    fn piece(&self, chars: &[usize]) -> Outcome {
        let on_path = self.chars[chars[0]].path;
        if on_path.is_none()
            && !self.justify
            && let Some(at) = self.partial_line_split(chars)
        {
            return Outcome::Split(vec![at]);
        }
        let aligned = on_path.is_none() && !self.justify && self.whole_lines_of_one_anchor(chars);
        match self.build(chars, aligned) {
            Outcome::Skip if aligned => self.build(chars, false),
            outcome => outcome,
        }
    }

    /// The text values of the piece without kerns.
    fn snapshot(&self, chars: &[usize]) -> Option<(TextSnapshot, &Resolved<'_>)> {
        let first = self.chars[chars[0]];
        let r = self.resolved[first.style].as_ref()?;
        let size = r.size.as_mm();
        let root = &self.text.styles[self.text.root_style];
        let factor = |value: f64, range: &std::ops::RangeInclusive<f64>| {
            (value / size).clamp(*range.start(), *range.end())
        };
        let style = &self.text.styles[first.style];
        let line_spacing = match style.line_height {
            LineHeight::Normal => None,
            _ => Some(factor(
                line_height_user(style, r) * self.s,
                &LINE_SPACING_FACTORS,
            )),
        };
        let mut string = String::new();
        for (k, &i) in chars.iter().enumerate() {
            if k > 0 {
                for _ in 0..self.targets.breaks[i] {
                    string.push('\n');
                }
            }
            string.push(self.chars[i].ch);
        }
        let layout = TextLayoutValues {
            align: TextAlign::Left,
            letter_spacing: factor(
                (root.letter_spacing + self.extra_spacing) * self.s,
                &LETTER_SPACING_FACTORS,
            ),
            word_spacing: factor(root.word_spacing * self.s, &WORD_SPACING_FACTORS),
            line_spacing,
            writing_mode: match r.writing_mode {
                SvgWritingMode::Horizontal => TextWritingMode::Horizontal,
                SvgWritingMode::VerticalRl => TextWritingMode::VerticalRl,
                SvgWritingMode::VerticalLr => TextWritingMode::VerticalLr,
            },
            orientation: match r.orientation {
                SvgOrientation::Auto => TextGlyphOrientation::Auto,
                SvgOrientation::Upright => TextGlyphOrientation::Upright,
                SvgOrientation::Sideways => TextGlyphOrientation::Sideways,
            },
            ..TextLayoutValues::DEFAULT
        };
        let snapshot = TextSnapshot {
            id: curvyo_document_core::UNCREATED_TEXT_ID,
            string,
            font: r.font.clone(),
            size: r.size,
            layout,
            chars: TextChars::default(),
            placement: Affine::IDENTITY,
            style: r.style.clone(),
            outline: None,
            on_path: None,
            source: None,
        };
        Some((snapshot, r))
    }

    /// Lays `snapshot` out and finds each piece character's glyph.
    fn natural(snapshot: &TextSnapshot, r: &Resolved<'_>, count: usize) -> Option<Natural> {
        let layout = crate::text_layout::text_layout(snapshot, r.face)?;
        let index_of_byte = Self::piece_indices(&snapshot.string);
        let mut at = vec![None; count];
        for (line_index, line) in layout.lines.iter().enumerate() {
            for glyph in &line.glyphs {
                if let Some(&Some(k)) = index_of_byte.get(glyph.cluster)
                    && at[k].is_none()
                {
                    at[k] = Some((
                        glyph.origin,
                        Point::new(glyph.centre_x.0, glyph.centre_y.0),
                        line_index,
                        line.baseline.0,
                    ));
                }
            }
        }
        Some(Natural { at, layout })
    }

    /// The piece index of the character at each byte of `string` (`None` inside a character
    /// and at a line break).
    fn piece_indices(string: &str) -> Vec<Option<usize>> {
        let mut out = vec![None; string.len()];
        let mut k = 0;
        for (byte, c) in string.char_indices() {
            if c != '\n' {
                out[byte] = Some(k);
                k += 1;
            }
        }
        out
    }

    /// The piece of `chars`, aligned by the file's anchor when `aligned`; [`Outcome::Skip`] also
    /// when the aligned layout does not land where the file draws the lines.
    #[allow(clippy::too_many_lines)] // one pass from the natural layout to the placed piece
    fn build(&self, chars: &[usize], aligned: bool) -> Outcome {
        let first = chars[0];
        let path = self.chars[first].path;
        let anchor = self
            .anchor
            .unwrap_or(self.text.styles[self.chars[first].style].anchor);
        let align = match (aligned, anchor, self.justify) {
            (_, _, true) => TextAlign::Justify,
            (true, TextAnchor::Middle, _) => TextAlign::Centre,
            (true, TextAnchor::End, _) => TextAlign::Right,
            _ => TextAlign::Left,
        };
        let Some((mut snapshot, r)) = self.snapshot(chars) else {
            return Outcome::Skip;
        };
        // A vertical text on a path is laid out horizontally (criterion 95); its mode is kept.
        let stored_mode = snapshot.layout.writing_mode;
        if path.is_some() {
            snapshot.layout.writing_mode = TextWritingMode::Horizontal;
        }
        let size = r.size.as_mm();
        let Some(natural) = Self::natural(&snapshot, r, chars.len()) else {
            return Outcome::Skip;
        };
        let vertical = snapshot.layout.writing_mode != TextWritingMode::Horizontal;
        let frame = if vertical {
            kurbo::Affine::new([0.0, 1.0, -1.0, 0.0, 0.0, 0.0])
        } else {
            kurbo::Affine::IDENTITY
        };
        // The origin O of text space in user units: the first character's pen on its baseline
        // (aligned: its chunk's start before anchoring).
        let pen = self.targets.pen[first];
        let at = self.targets.at[first];
        let origin = match (aligned, vertical) {
            (true, false) => Point::new(self.targets.chunk_start[first], pen.y),
            (true, true) => Point::new(pen.x, self.targets.chunk_start[first]),
            (false, false) => Point::new(at.x, pen.y),
            (false, true) => Point::new(pen.x, at.y),
        };
        // Aligned: where SVG draws the character before its chunk is anchored (the anchoring
        // is the alignment's), with its baseline move.
        let target = |i: usize, aligned: bool| -> Point {
            let (pen, at) = (self.targets.pen[i], self.targets.at[i]);
            match (aligned, vertical) {
                (false, _) => at,
                (true, false) => Point::new(pen.x, at.y),
                (true, true) => Point::new(at.x, pen.y),
            }
        };
        // The left-aligned origin of a piece that starts at character `i`.
        let left_origin = |i: usize| {
            let (pen, at) = (self.targets.pen[i], self.targets.at[i]);
            if vertical {
                Point::new(pen.x, at.y)
            } else {
                Point::new(at.x, pen.y)
            }
        };
        let mut values = Vec::with_capacity(chars.len());
        let mut previous = (0.0, 0.0);
        let mut line = usize::MAX;
        // The first glyph's own offset from its pen applies in SVG too: positions name the pen.
        // A pen is the glyph origin along the line and the line's baseline across it (a glyph of
        // a column is centred on it by its own offset, as in SVG).
        let pen_of = |origin: Point, baseline: f64| Point::new(origin.x, baseline);
        let first_offset = natural.at[0].map_or(Vec2::ZERO, |(origin, _, _, baseline)| {
            (frame * pen_of(origin, baseline)).to_vec2()
        });
        // After a kern out of range the later pieces of that line are found from this same
        // layout, rebased on their first character (no layout per split).
        let mut splits: Vec<usize> = Vec::new();
        let (mut scan_origin, mut scan_offset, mut scan_line) = (origin, first_offset, None);
        for (k, &i) in chars.iter().enumerate() {
            let Some((origin_line, centre, line_index, baseline)) = natural.at[k] else {
                values.push((k, CharValues::DEFAULT));
                continue;
            };
            if line_index != line {
                if scan_line.is_some() {
                    break;
                }
                line = line_index;
                previous = (0.0, 0.0);
            }
            let scanning = scan_line.is_some();
            let wanted = (target(i, aligned && !scanning) - scan_origin) * self.s;
            let natural_text = frame * pen_of(origin_line, baseline) - scan_offset;
            let desired = frame.inverse() * (Point::ZERO + (wanted - natural_text.to_vec2()));
            let (mut along, mut across) = (desired.x, desired.y);
            let rotation = self.chars[i].rotate.unwrap_or(0.0);
            if rotation != 0.0 && !vertical {
                let (sin, cos) = rotation.to_radians().sin_cos();
                let v = origin_line - centre;
                along += v.x - (v.x * cos - v.y * sin);
                across += v.y - (v.x * sin + v.y * cos);
            }
            // A kern below half a micron is the rounding of the file's numbers: none.
            let snap = |mm: f64| if mm.abs() < KERN_SNAP_MM { 0.0 } else { mm };
            let (kern_along, kern_across) = (snap(along - previous.0), snap(across - previous.1));
            previous = (previous.0 + kern_along, previous.1 + kern_across);
            let kern_x = kern_along / size;
            let kern_y = -kern_across / size;
            let out_of_range =
                !KERN_X_FACTORS.contains(&kern_x) || !KERN_Y_FACTORS.contains(&kern_y);
            if out_of_range && k > 0 && !self.justify {
                splits.push(k);
                scan_line = Some(line_index);
                scan_origin = left_origin(i);
                scan_offset = (frame * pen_of(origin_line, baseline)).to_vec2();
                previous = (0.0, 0.0);
                continue;
            }
            if scanning {
                continue;
            }
            values.push((
                k,
                CharValues {
                    kern_x,
                    kern_y,
                    rotation,
                },
            ));
        }
        if !splits.is_empty() {
            return Outcome::Split(splits);
        }
        // The string holds the line breaks too: a kern is stored at the character's place in it.
        let mut string_index = Vec::with_capacity(chars.len());
        let mut at = 0;
        for (k, &i) in chars.iter().enumerate() {
            if k > 0 {
                at += self.targets.breaks[i];
            }
            string_index.push(at);
            at += 1;
        }
        let kerns = if self.justify {
            TextChars::default()
        } else {
            TextChars::new(values.into_iter().map(|(k, v)| (string_index[k], v)))
        };
        let unchanged = kerns.is_empty() && align == TextAlign::Left;
        snapshot.chars = kerns;
        snapshot.layout.align = align;
        let mut origin = origin;
        let final_layout = if unchanged {
            natural.layout
        } else {
            match crate::text_layout::text_layout(&snapshot, r.face) {
                Some(layout) => layout,
                None => return Outcome::Skip,
            }
        };
        if aligned && !self.lands(chars, &final_layout, &snapshot, origin, frame, &mut origin) {
            return Outcome::Skip;
        }
        snapshot.layout.writing_mode = stored_mode;
        let to_doc = kurbo::Affine::new(self.text.matrix.as_coeffs())
            * kurbo::Affine::translate(origin.to_vec2())
            * kurbo::Affine::scale(1.0 / self.s);
        snapshot.placement = Affine::from_array(to_doc.as_coeffs());
        if let Some(p) = path {
            return Outcome::Piece(Box::new(self.on_path_piece(
                snapshot,
                &final_layout,
                p,
                origin,
            )));
        }
        let b = crate::text_caret::letters_box(&final_layout);
        let corners = [
            Point::new(b.x0, b.y0),
            Point::new(b.x1, b.y0),
            Point::new(b.x1, b.y1),
            Point::new(b.x0, b.y1),
        ]
        .map(|p| to_doc * p);
        let bounds = corners
            .iter()
            .skip(1)
            .fold(Rect::from_points(corners[0], corners[0]), |r, p| {
                r.union_pt(*p)
            });
        Outcome::Piece(Box::new(TextPiece {
            text: drawing_text(snapshot),
            on_path: None,
            bounds: Some(bounds),
        }))
    }

    /// A piece on its carrier (criterion 93): Position is the arc length of its middle.
    fn on_path_piece(
        &self,
        mut snapshot: TextSnapshot,
        layout: &TextLayout,
        path: usize,
        origin: Point,
    ) -> TextPiece {
        let text_path = &self.text.paths[path];
        let middle = layout.lines.first().map_or(0.0, |line| {
            f64::midpoint(line.extent.start.0, line.extent.end.0)
        });
        let along = origin.x * self.s + middle;
        // `side="right"` runs along the reversed carrier: its offsets count from the end.
        let position = if text_path.side_right {
            text_path.length * self.s - along
        } else {
            along
        };
        let mut values = OnPathValues::at(Length::from_mm(position));
        if text_path.side_right {
            values.side = PathSide::Below;
        }
        // The straight placement kept for Release: at the text's own x, y.
        let start = self
            .chars
            .iter()
            .find(|c| c.ch != '\n')
            .map_or(Point::ZERO, |c| {
                Point::new(c.x.unwrap_or(0.0), c.y.unwrap_or(0.0))
            });
        let to_doc = kurbo::Affine::new(self.text.matrix.as_coeffs())
            * kurbo::Affine::translate(start.to_vec2())
            * kurbo::Affine::scale(1.0 / self.s);
        snapshot.placement = Affine::from_array(to_doc.as_coeffs());
        TextPiece {
            text: drawing_text(snapshot),
            on_path: text_path.carrier.map(|carrier| (carrier, values)),
            bounds: None,
        }
    }
}

fn drawing_text(snapshot: TextSnapshot) -> DrawingText {
    DrawingText {
        string: snapshot.string,
        font: snapshot.font,
        size: snapshot.size,
        layout: snapshot.layout,
        chars: snapshot.chars,
        placement: snapshot.placement,
        style: snapshot.style,
    }
}
