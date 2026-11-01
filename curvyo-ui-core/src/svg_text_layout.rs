//! Where SVG draws each character of an imported text (`specs/0024-svg-import-export` criteria
//! 83, 85, 86, 89, 90, 93, 96): the face and Size of each style, the advances shaped in that face,
//! the lines, flowed paragraphs broken at the box width, and the SVG 1.1 positions with text
//! chunks anchored and baselines moved. Everything here is in the text's user units.

use curvyo_document_core::{FontKind, FontRef, Length, MISSING_FACE_HASH, Style, clamp_text_size};
use curvyo_svg_core::{
    Baseline, BaselineShift, LineHeight, SvgChar, SvgOrientation, SvgText, SvgWritingMode,
    TextAnchor, TextStyle,
};
use curvyo_text_core::{
    Face, FaceKey, FaceStore, GlyphOrientation, LayoutOptions, Mm, WritingMode,
};
use kurbo::Point;

use crate::font_match::ListedFace;
use crate::svg_font_match::{FaceMemo, SvgFace};
use crate::svg_import_style::model_style;

/// The faces an import draws with (phase 2 of `adrs.md` note 3).
#[derive(Debug, Clone, Copy)]
pub struct ImportFaces<'a> {
    /// The listed faces.
    pub faces: &'a [ListedFace],
    /// The loaded faces.
    pub store: &'a FaceStore,
    /// The included sans Regular a missing face is drawn in, when loaded.
    pub substitute: Option<FaceKey>,
}

/// The most characters shaped in one run for the advances (far below the engine's limit; a
/// longer run is shaped in parts).
const ADVANCE_RUN_CHARS: usize = 10_000;

/// The characters word spacing widens (as the engine's).
pub(crate) const SPACES: [char; 3] = [' ', '\u{a0}', '\u{3000}'];

/// One style resolved to its face and sizes.
#[derive(Debug, Clone)]
pub(crate) struct Resolved<'a> {
    pub(crate) face: &'a Face,
    pub(crate) font: FontRef,
    /// Size, the capital H, clamped to the size range.
    pub(crate) size: Length,
    /// Fill and stroke.
    pub(crate) style: Style,
    pub(crate) writing_mode: SvgWritingMode,
    pub(crate) orientation: SvgOrientation,
}

/// The style `style` resolved under the text's scale `s` (√|det| of user units to mm), or `None`
/// when no face can draw it.
pub(crate) fn resolve<'a>(
    style: &TextStyle,
    fonts: &ImportFaces<'a>,
    memo: &FaceMemo<'_>,
    s: f64,
) -> Option<Resolved<'a>> {
    let (key, font) = match memo.face(style) {
        SvgFace::Listed(at) => {
            let listed = &fonts.faces[at];
            match listed.key {
                Some(key) => (
                    key,
                    FontRef {
                        family: listed.family.clone(),
                        style: listed.style.clone(),
                        face_hash: key.hash.to_hex(),
                        face_index: key.index,
                        kind: listed.kind,
                    },
                ),
                None => (fonts.substitute?, missing(&listed.family, &listed.style)),
            }
        }
        SvgFace::Missing { family, style } => (fonts.substitute?, missing(&family, &style)),
    };
    let face = fonts.store.get(&key)?;
    let size = clamp_text_size(Length::from_mm(
        style.font_size * s * face.metrics().cap_height,
    ));
    Some(Resolved {
        face,
        font,
        size,
        style: model_style(&style.paint),
        writing_mode: style.writing_mode,
        orientation: style.orientation,
    })
}

/// The longest style name a `curvyo:face` reference is used with (the reader's font-name cut).
const MAX_STYLE_NAME_CHARS: usize = 128;

/// The style `style` resolved by the face reference `face` (`curvyo:face`, criteria 132 and
/// 135) and the exact Size `size` (`curvyo:size`), when they agree with the standard values next
/// to them: the family is the first named family, weight and slant are those of the reference's
/// style name (a name the engine parses, at most [`MAX_STYLE_NAME_CHARS`] characters), and the
/// font size gives the Size within 1e-6 mm, inside the Size limits. `None` when they disagree;
/// the lookup by name then applies.
pub(crate) fn resolve_by_reference<'a>(
    style: &TextStyle,
    (face, size): (&str, Option<f64>),
    fonts: &ImportFaces<'a>,
    s: f64,
) -> Option<Resolved<'a>> {
    let mut parts = face.splitn(3, '|');
    let (family, style_name, hash) = (parts.next()?, parts.next()?, parts.next()?);
    let named = match style.families.first()? {
        curvyo_svg_core::FontFamily::Named(name) => name.as_str(),
        _ => return None,
    };
    if style_name.chars().count() > MAX_STYLE_NAME_CHARS {
        return None;
    }
    let wanted = curvyo_text_core::FaceStyle::parse(style_name)?;
    if named != family || wanted.weight != style.weight || wanted.is_slanted() != style.italic {
        return None;
    }
    let listed = fonts
        .faces
        .iter()
        .find(|listed| listed.key.is_some_and(|key| key.hash.to_hex() == hash));
    let (key, font) = match listed.and_then(|l| l.key.map(|key| (l, key))) {
        Some((listed, key)) if fonts.store.get(&key).is_some() => (
            key,
            FontRef {
                family: family.to_owned(),
                style: style_name.to_owned(),
                face_hash: hash.to_owned(),
                face_index: key.index,
                kind: listed.kind,
            },
        ),
        _ => {
            let mut font = missing(family, style_name);
            if curvyo_text_core::FaceHash::from_hex(hash).is_some() {
                hash.clone_into(&mut font.face_hash);
            }
            (fonts.substitute?, font)
        }
    };
    let face = fonts.store.get(&key)?;
    let standard = style.font_size * s * face.metrics().cap_height;
    let size = match size {
        Some(exact)
            if (exact - standard).abs() <= 1e-6
                && clamp_text_size(Length::from_mm(exact)) == Length::from_mm(exact) =>
        {
            Length::from_mm(exact)
        }
        Some(_) => return None,
        None => clamp_text_size(Length::from_mm(standard)),
    };
    Some(Resolved {
        face,
        font,
        size,
        style: model_style(&style.paint),
        writing_mode: style.writing_mode,
        orientation: style.orientation,
    })
}

fn missing(family: &str, style: &str) -> FontRef {
    FontRef {
        family: family.to_owned(),
        style: style.to_owned(),
        face_hash: MISSING_FACE_HASH.to_owned(),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

/// The engine's writing mode and orientation of a style.
pub(crate) fn engine_mode(
    mode: SvgWritingMode,
    orientation: SvgOrientation,
) -> (WritingMode, GlyphOrientation) {
    (
        match mode {
            SvgWritingMode::Horizontal => WritingMode::Horizontal,
            SvgWritingMode::VerticalRl => WritingMode::VerticalRl,
            SvgWritingMode::VerticalLr => WritingMode::VerticalLr,
        },
        match orientation {
            SvgOrientation::Auto => GlyphOrientation::Auto,
            SvgOrientation::Upright => GlyphOrientation::Upright,
            SvgOrientation::Sideways => GlyphOrientation::Sideways,
        },
    )
}

/// The advance of every character in user units, along its line, without added spacing: each
/// run of one style between line breaks is shaped in its face (a turned character alone); a
/// character inside a cluster has 0, the cluster's first character the whole advance.
pub(crate) fn advances(
    text: &SvgText,
    chars: &[SvgChar],
    resolved: &[Option<Resolved<'_>>],
    s: f64,
) -> Vec<f64> {
    let mut out = vec![0.0; chars.len()];
    let mut start = 0;
    while start < chars.len() {
        let c = chars[start];
        if c.ch == '\n' {
            start += 1;
            continue;
        }
        let mut end = start + 1;
        while end < chars.len()
            && end - start < ADVANCE_RUN_CHARS
            && chars[end].ch != '\n'
            && chars[end].style == c.style
            && chars[end].path == c.path
            && !turned(&chars[end])
            && !turned(&chars[end - 1])
        {
            end += 1;
        }
        if let Some(r) = &resolved[c.style] {
            let ls = text.styles[c.style].letter_spacing * s;
            run_advances(r, &chars[start..end], ls, s, &mut out[start..end]);
        }
        start = end;
    }
    out
}

/// Whether `c` has its own turn: SVG renderers (Inkscape) shape it alone, so it has no kerning
/// pair with its neighbours (criterion 87).
fn turned(c: &SvgChar) -> bool {
    c.rotate.is_some_and(|r| r != 0.0)
}

/// The advances of one run shaped in `r` with the letter spacing `ls` mm (which decides the
/// ligatures, as the engine does), in user units.
fn run_advances(r: &Resolved<'_>, chars: &[SvgChar], ls: f64, s: f64, out: &mut [f64]) {
    let string: String = chars.iter().map(|c| c.ch).collect();
    let (writing_mode, orientation) = engine_mode(r.writing_mode, r.orientation);
    let options = LayoutOptions {
        letter_spacing: Mm(ls),
        writing_mode,
        orientation,
        ..LayoutOptions::default()
    };
    let Ok(laid) = curvyo_text_core::layout(r.face, &string, Mm(r.size.as_mm()), &options) else {
        return;
    };
    let Some(line) = laid.lines.first() else {
        return;
    };
    let byte_to_char: std::collections::BTreeMap<usize, usize> = string
        .char_indices()
        .enumerate()
        .map(|(index, (byte, _))| (byte, index))
        .collect();
    // The pen of each cluster: its first glyph's origin.
    let mut pens: Vec<(usize, f64)> = Vec::new();
    for glyph in &line.glyphs {
        if pens.last().is_none_or(|(byte, _)| *byte != glyph.cluster) {
            pens.push((glyph.cluster, glyph.origin.x));
        }
    }
    pens.sort_by_key(|(byte, _)| *byte);
    let end = line.start.0 + line.width.0;
    for (k, (byte, pen)) in pens.iter().enumerate() {
        let next = pens.get(k + 1).map_or(end, |(_, next)| next - ls);
        if let Some(&at) = byte_to_char.get(byte)
            && let Some(slot) = out.get_mut(at)
        {
            *slot = (next - pen) / s;
        }
    }
}

/// The natural line height of a style in user units: `line-height`, else the face's own.
pub(crate) fn line_height_user(style: &TextStyle, r: &Resolved<'_>) -> f64 {
    match style.line_height {
        LineHeight::Factor(f) => f * style.font_size,
        LineHeight::Length(l) => l,
        LineHeight::Normal => {
            let vertical = style.writing_mode != SvgWritingMode::Horizontal;
            r.face.metrics().auto_line_height(vertical) * style.font_size
        }
    }
}

/// The SVG positions of a text's characters, user units; for a character on a text path, the
/// arc length (x) and the shift across the path (y).
#[derive(Debug, Clone, Default)]
pub(crate) struct Targets {
    /// Where the character is drawn.
    pub(crate) at: Vec<Point>,
    /// Where it would be without its chunk's anchoring and its baseline move.
    pub(crate) pen: Vec<Point>,
    /// Whether it starts a text chunk.
    pub(crate) chunk: Vec<bool>,
    /// The along position of its chunk's start, before anchoring.
    pub(crate) chunk_start: Vec<f64>,
    /// Whether it starts a line (not the first) and how many line breaks stand before it.
    pub(crate) breaks: Vec<usize>,
}

/// The x-height of `face` in em: the top of its `x`, else 0.5.
fn x_height(face: &Face) -> f64 {
    face.outline(face.glyph_for('x')).top().unwrap_or(0.5)
}

/// How far down (user units) `style` moves the baseline of its characters (criterion 90).
fn baseline_drop(style: &TextStyle, face: &Face) -> f64 {
    let m = face.metrics();
    let fs = style.font_size;
    let dominant = match style.baseline {
        Baseline::Alphabetic => 0.0,
        Baseline::Middle => x_height(face) / 2.0,
        Baseline::Central => f64::midpoint(m.ascender, m.descender),
        Baseline::Hanging => 0.8 * m.ascender,
        Baseline::TextTop => m.ascender,
        Baseline::TextBottom => m.descender,
        Baseline::Mathematical => m.ascender / 2.0,
    } * fs;
    let shift = match style.baseline_shift {
        BaselineShift::None => 0.0,
        BaselineShift::Up(l) => -l,
        BaselineShift::Super => -m.superscript_offset.unwrap_or(1.0 / 3.0) * fs,
        BaselineShift::Sub => m.subscript_offset.unwrap_or(1.0 / 5.0) * fs,
    };
    dominant + shift
}

/// How much further down its column (user units) an upright character of a vertical text is
/// drawn than the engine's cell puts it (criteria 86 and 91): Inkscape 1.4 draws it the
/// ascender's share of the face's line (ascender over ascender minus descender) less the H
/// height further down, measured on its own conversion of an upright column.
fn upright_lead(style: &TextStyle, r: &Resolved<'_>, c: char) -> f64 {
    let (_, orientation) = engine_mode(r.writing_mode, r.orientation);
    if !orientation.is_upright(c) {
        return 0.0;
    }
    let m = r.face.metrics();
    let box_height = m.ascender - m.descender;
    if box_height <= 0.0 {
        return 0.0;
    }
    (m.ascender / box_height - m.cap_height) * style.font_size
}

/// The SVG 1.1 positions of `chars` (criteria 85, 86, 89, 90, 93): absolute and relative
/// positions, the advances with letter and word spacing, each chunk anchored by its first
/// character's `text-anchor` (`anchor` overrides it for flowed text); a new line where a
/// character returns to an earlier baseline, after a line break, or at an absolute position
/// that goes back along the line to another baseline.
pub(crate) fn targets(
    text: &SvgText,
    chars: &[SvgChar],
    advances: &[f64],
    resolved: &[Option<Resolved<'_>>],
    anchor: Option<TextAnchor>,
) -> Targets {
    let vertical = text.styles[text.root_style].writing_mode != SvgWritingMode::Horizontal;
    let leftward = text.styles[text.root_style].writing_mode == SvgWritingMode::VerticalRl;
    // (along, across) of a point and back.
    let split = |p: Point| if vertical { (p.y, p.x) } else { (p.x, p.y) };
    let join = |along: f64, across: f64| {
        if vertical {
            Point::new(across, along)
        } else {
            Point::new(along, across)
        }
    };
    let n = chars.len();
    let mut out = Targets {
        at: vec![Point::ZERO; n],
        pen: vec![Point::ZERO; n],
        chunk: vec![false; n],
        chunk_start: vec![0.0; n],
        breaks: vec![0; n],
    };
    let mut ends = vec![0.0; n];
    let (mut along, mut across) = (0.0, 0.0);
    let mut path_pen = PathPen::default();
    let mut line: Option<(f64, f64)> = None; // (start along, baseline across)
    let mut pending = 0_usize;
    let mut first = true;
    for (i, c) in chars.iter().enumerate() {
        if c.ch == '\n' {
            pending += 1;
            continue;
        }
        let style = &text.styles[c.style];
        let (abs_along, abs_across) = if vertical { (c.y, c.x) } else { (c.x, c.y) };
        let (d_along, d_across) = if vertical { (c.dy, c.dx) } else { (c.dx, c.dy) };
        if let Some(p) = c.path {
            let drop = resolved[c.style]
                .as_ref()
                .map_or(0.0, |r| baseline_drop(style, r.face));
            ends[i] = path_pen.place(text, c, p, i, drop, advances[i], &mut out);
            first = false;
            continue;
        }
        path_pen.slot = None;
        let before = along;
        let mut breaks = pending;
        if pending > 0
            && abs_across.is_none()
            && let (Some((start, base)), Some(r)) = (line, resolved[c.style].as_ref())
        {
            let lh = line_height_user(&text.styles[text.root_style], r)
                * f64::from(u32::try_from(pending).unwrap_or(u32::MAX));
            across = if vertical && leftward {
                base - lh
            } else {
                base + lh
            };
            if abs_along.is_none() {
                along = start;
            }
        }
        if let Some(a) = abs_along {
            along = a;
        }
        if let Some(a) = abs_across {
            across = a;
        }
        if breaks == 0
            && !first
            && abs_across.is_some()
            && line.is_some_and(|(_, base)| (across - base).abs() > 1e-9)
            && along < before - 1e-9
        {
            breaks = 1;
        }
        out.chunk[i] = first || breaks > 0 || abs_along.is_some() || abs_across.is_some();
        along += d_along;
        across += d_across;
        if first || breaks > 0 {
            line = Some((along, across));
        }
        out.breaks[i] = if first { 0 } else { breaks };
        pending = 0;
        first = false;
        let (drop, lead) = match resolved[c.style].as_ref() {
            Some(r) if vertical => (0.0, upright_lead(style, r, c.ch)),
            Some(r) => (baseline_drop(style, r.face), 0.0),
            None => (0.0, 0.0),
        };
        out.pen[i] = join(along + lead, across);
        out.at[i] = join(along + lead, across + drop);
        along += advances[i] + spacing(style, c.ch);
        ends[i] = along;
    }
    anchor_chunks(text, chars, &mut out, &ends, anchor, split, join);
    out
}

/// The pen along a text path: arc length and shift across it, restarted at each new path.
#[derive(Debug, Default)]
struct PathPen {
    slot: Option<usize>,
    along: f64,
    across: f64,
}

impl PathPen {
    /// Places character `i` (`c`, on path `p`, its baseline moved down by `drop`) and returns
    /// the pen after it.
    #[allow(clippy::too_many_arguments)] // the character, its place and what it moves
    fn place(
        &mut self,
        text: &SvgText,
        c: &SvgChar,
        p: usize,
        i: usize,
        drop: f64,
        advance: f64,
        out: &mut Targets,
    ) -> f64 {
        if self.slot != Some(p) {
            self.slot = Some(p);
            self.along = text.paths[p].start_offset;
            self.across = 0.0;
            out.chunk[i] = true;
        }
        if let Some(a) = c.x {
            self.along = a;
            out.chunk[i] = true;
        }
        self.along += c.dx;
        self.across += c.dy;
        out.pen[i] = Point::new(self.along, self.across);
        out.at[i] = Point::new(self.along, self.across + drop);
        self.along += advance + spacing(&text.styles[c.style], c.ch);
        self.along
    }
}

/// Letter spacing plus, for a space, word spacing.
pub(crate) fn spacing(style: &TextStyle, c: char) -> f64 {
    style.letter_spacing
        + if SPACES.contains(&c) {
            style.word_spacing
        } else {
            0.0
        }
}

/// Moves every chunk along by its anchor: 0, half or all of its advance.
fn anchor_chunks(
    text: &SvgText,
    chars: &[SvgChar],
    out: &mut Targets,
    ends: &[f64],
    anchor: Option<TextAnchor>,
    split: impl Fn(Point) -> (f64, f64),
    join: impl Fn(f64, f64) -> Point,
) {
    let real: Vec<usize> = (0..chars.len()).filter(|&i| chars[i].ch != '\n').collect();
    let mut k = 0;
    while k < real.len() {
        let first = real[k];
        let mut last = k;
        while last + 1 < real.len() && !out.chunk[real[last + 1]] {
            last += 1;
        }
        let on_path = chars[first].path.is_some();
        let start = if on_path {
            out.pen[first].x
        } else {
            split(out.pen[first]).0
        };
        let width = ends[real[last]] - start;
        let shift = match anchor.unwrap_or(text.styles[chars[first].style].anchor) {
            TextAnchor::Start => 0.0,
            TextAnchor::Middle => -width / 2.0,
            TextAnchor::End => -width,
        };
        for &i in &real[k..=last] {
            out.chunk_start[i] = start;
            if on_path {
                out.at[i].x += shift;
            } else {
                let (a, b) = split(out.at[i]);
                out.at[i] = join(a + shift, b);
            }
        }
        k = last + 1;
    }
}
