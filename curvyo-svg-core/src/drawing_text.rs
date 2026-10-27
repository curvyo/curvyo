//! What the reader gives back for a `<text>`, a flowed text and an `<image>`
//! (`specs/0024-svg-import-export` criteria 78 to 105): font-free character lists with their
//! resolved style and positions, and decoded picture bytes with their viewport. Lengths are in
//! the text's or the image's own user units; `matrix` maps them to millimetres.

use kurbo::Affine;

use crate::drawing::LeafStyle;

/// One `<text>` (or `<flowRoot>`) read without fonts.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgText {
    /// The text's user units to millimetres: ancestors, the viewBox map and its own transform.
    pub matrix: Affine,
    /// The characters in file order after white-space handling (criterion 79); a `'\n'` entry is
    /// a line break (an Inkscape line, a newline under `white-space: pre`, a flowed paragraph).
    pub chars: Vec<SvgChar>,
    /// The resolved styles; every character names one.
    pub styles: Vec<TextStyle>,
    /// The style of the `<text>` element itself (its spacing is the text's, criterion 88).
    pub root_style: usize,
    /// The `<textPath>` pieces the characters may sit on.
    pub paths: Vec<SvgTextPath>,
    /// The box of a flowed text without positioned lines (criterion 96).
    pub flow: Option<FlowBox>,
    /// `textLength` of the `<text>` element in user units (criterion 88).
    pub text_length: Option<f64>,
}

/// One character with the values the file gives it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgChar {
    /// The character; `'\n'` is a line break and takes no position values.
    pub ch: char,
    /// An absolute x, user units.
    pub x: Option<f64>,
    /// An absolute y, user units.
    pub y: Option<f64>,
    /// A shift along x before the character, user units.
    pub dx: f64,
    /// A shift along y before the character, user units (down is positive).
    pub dy: f64,
    /// Its own turn in degrees, clockwise on the screen.
    pub rotate: Option<f64>,
    /// Its style in [`SvgText::styles`].
    pub style: usize,
    /// The `<textPath>` in [`SvgText::paths`] it sits on.
    pub path: Option<usize>,
}

/// A font-family entry.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontFamily {
    /// A named family, at most 128 characters.
    Named(String),
    /// `sans-serif`, `system-ui`, `monospace` or `fantasy`.
    SansSerif,
    /// `serif`.
    Serif,
    /// `cursive`.
    Cursive,
}

/// `text-anchor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAnchor {
    /// `start`.
    #[default]
    Start,
    /// `middle`.
    Middle,
    /// `end`.
    End,
}

/// `writing-mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SvgWritingMode {
    /// Horizontal lines.
    #[default]
    Horizontal,
    /// Columns, the next to the left.
    VerticalRl,
    /// Columns, the next to the right.
    VerticalLr,
}

/// `text-orientation` and `glyph-orientation-vertical`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SvgOrientation {
    /// `mixed`, `auto`.
    #[default]
    Auto,
    /// `upright`, `0`.
    Upright,
    /// `sideways`, `90`.
    Sideways,
}

/// `line-height`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LineHeight {
    /// `normal`, or none given: Auto.
    #[default]
    Normal,
    /// A number or percentage: times the font size of the element it applies to.
    Factor(f64),
    /// A length in user units.
    Length(f64),
}

/// `dominant-baseline` and `alignment-baseline` (criterion 90).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Baseline {
    /// `auto`, `alphabetic` and the rest: no move.
    #[default]
    Alphabetic,
    /// `middle`.
    Middle,
    /// `central`.
    Central,
    /// `hanging`.
    Hanging,
    /// `text-top`, `text-before-edge`.
    TextTop,
    /// `text-bottom`, `text-after-edge`, `ideographic`.
    TextBottom,
    /// `mathematical`.
    Mathematical,
}

/// `baseline-shift`.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum BaselineShift {
    /// None.
    #[default]
    None,
    /// Up by this length, user units.
    Up(f64),
    /// `super`.
    Super,
    /// `sub`.
    Sub,
}

/// `text-align` of a flowed text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlowAlign {
    /// `start`, `left`.
    #[default]
    Start,
    /// `center`.
    Centre,
    /// `end`, `right`.
    End,
    /// `justify`.
    Justify,
}

/// The resolved text style of a run of characters.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// The `font-family` list in order; never empty (no value is `sans-serif`).
    pub families: Vec<FontFamily>,
    /// The requested weight, 1 to 1000.
    pub weight: u16,
    /// `italic` or `oblique`.
    pub italic: bool,
    /// The requested width class, 1 to 9 (5 is normal).
    pub stretch: u16,
    /// The font size in user units, above 0.
    pub font_size: f64,
    /// `letter-spacing` in user units.
    pub letter_spacing: f64,
    /// `word-spacing` in user units.
    pub word_spacing: f64,
    /// `text-anchor`.
    pub anchor: TextAnchor,
    /// `writing-mode`.
    pub writing_mode: SvgWritingMode,
    /// The glyph orientation of vertical text.
    pub orientation: SvgOrientation,
    /// `line-height`.
    pub line_height: LineHeight,
    /// The baseline the characters sit on.
    pub baseline: Baseline,
    /// `baseline-shift`.
    pub baseline_shift: BaselineShift,
    /// `text-align` (flowed text only).
    pub flow_align: FlowAlign,
    /// Fill and stroke, the stroke width in millimetres.
    pub paint: LeafStyle,
}

/// A `<textPath>`.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgTextPath {
    /// The leaf (document order of all leaves) the text sits on; `None` when its target is not a
    /// drawn shape (criterion 95: a straight text).
    pub carrier: Option<usize>,
    /// `startOffset` in the text's user units along the carrier.
    pub start_offset: f64,
    /// `side="right"`.
    pub side_right: bool,
    /// The length of the carrier's first outline in the text's user units (0 for a straight
    /// text), from whose end `side="right"` measures.
    pub length: f64,
}

/// The frame of a flowed text, user units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowBox {
    /// The left edge.
    pub left: f64,
    /// The top edge; the first baseline lies the face's ascender below it.
    pub top: f64,
    /// The width lines are broken at; 0 or below gives one line per paragraph.
    pub width: f64,
    /// The first baseline when the file states it (`inline-size` at the text's `y`).
    pub first_baseline: Option<f64>,
    /// Whether the alignment comes from `text-anchor` (`inline-size`) instead of `text-align`.
    pub by_anchor: bool,
}

/// One embedded `<image>`.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgImage {
    /// The decoded bytes of the `data:` URI, not yet identified.
    pub bytes: Vec<u8>,
    /// `x`, `y` in user units.
    pub origin: (f64, f64),
    /// `width`, `height` in user units; `None` for `auto` or missing (the picture's pixels).
    pub size: (Option<f64>, Option<f64>),
    /// `preserveAspectRatio`.
    pub aspect: ImageAspect,
    /// User units to millimetres.
    pub matrix: Affine,
}

/// `preserveAspectRatio` of an image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageAspect {
    /// The alignment as fractions of the free space (0, 0.5, 1); `None` for `none`.
    pub align: Option<(f64, f64)>,
    /// `slice` instead of `meet`.
    pub slice: bool,
}
