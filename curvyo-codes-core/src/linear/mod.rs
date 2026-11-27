//! The 1D symbologies (ADR 0092 §1): the run list of a code, the pure encoders of Code 128, Code 39,
//! EAN-13, EAN-8 and UPC-A, and the drawing and parameters built on it.

mod code128;
mod code39;
mod ean;
pub(crate) mod generate;
pub(crate) mod schema;
mod style;

pub use generate::Symbology;
pub use style::{Drawn, LinearStyle, TextLayout, draw};

/// The most characters Code 128 and Code 39 take (spec 0144 criterion 22).
pub const MAX_CHARACTERS: usize = 80;

/// Why a content cannot be encoded as a 1D code. The content itself is never in an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LinearError {
    /// Nothing to encode.
    #[error("nothing to encode")]
    Empty,
    /// EAN or UPC content of the wrong length (digits, check digit excluded or included).
    #[error("the content has the wrong length for this symbology")]
    WrongLength {
        /// The shorter accepted length.
        short: u32,
        /// The longer accepted length (with the check digit).
        long: u32,
    },
    /// EAN or UPC content with a character that is not a digit.
    #[error("the content has a character that is not a digit")]
    DigitsOnly,
    /// A character the symbology cannot encode.
    #[error("the content has a character this symbology cannot encode")]
    NotEncodable,
    /// More characters than the symbology takes here.
    #[error("the content is too long")]
    TooLong {
        /// The characters in the content.
        characters: usize,
        /// The most it takes.
        limit: usize,
    },
}

/// One bar: where it starts and how wide it is, in modules from the first bar's left edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    /// The left edge.
    pub x: f64,
    /// The width.
    pub width: f64,
    /// Whether it reaches into the text band (an EAN or UPC guard bar).
    pub guard: bool,
}

/// One character of the human-readable text: where its cell is centred.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The character.
    pub ch: char,
    /// The centre of its cell in modules from the first bar's left edge.
    pub x: f64,
    /// Whether the cell lies in the left or right quiet zone (the leading and trailing digit).
    pub outside: bool,
}

/// The text a 1D code prints and how its characters are placed.
#[derive(Debug, Clone, PartialEq)]
pub enum Printed {
    /// One digit per cell, centred in the cell (EAN and UPC).
    Cells(Vec<Cell>),
    /// A line centred under the bars (Code 128 and Code 39).
    Line(String),
}

/// An encoded 1D code: the bars in modules and the text to print.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearCode {
    /// The bars, left to right.
    pub bars: Vec<Bar>,
    /// The width of the symbol in modules, quiet zones excluded.
    pub width: f64,
    /// The quiet zones in modules, left and right, when the standard fixes them.
    pub fixed_quiet: Option<(f64, f64)>,
    /// What the human-readable text is.
    pub printed: Printed,
    /// The content as encoded (digits with the check digit, upper case for Code 39).
    pub normalised: String,
    /// The check digit was corrected or appended: `Some(true)` for a wrong one that was replaced.
    pub corrected: Option<bool>,
}

impl LinearCode {
    /// The run list: widths in modules, first a bar, bars and spaces alternating.
    #[must_use]
    pub fn runs(&self) -> Vec<f64> {
        let mut runs = Vec::with_capacity(self.bars.len() * 2);
        let mut at = 0.0;
        for bar in &self.bars {
            if bar.x > at + 1e-9 && !runs.is_empty() {
                runs.push(bar.x - at);
            }
            runs.push(bar.width);
            at = bar.x + bar.width;
        }
        runs
    }
}

/// Builds the bars of a pattern of alternating run widths, first a bar.
pub(super) fn bars_of(runs: &[f64], guards: impl Fn(f64) -> bool) -> (Vec<Bar>, f64) {
    let mut bars = Vec::with_capacity(runs.len() / 2 + 1);
    let mut x = 0.0;
    for (index, width) in runs.iter().enumerate() {
        if index % 2 == 0 {
            bars.push(Bar {
                x,
                width: *width,
                guard: guards(x),
            });
        }
        x += width;
    }
    (bars, x)
}

/// Encodes `content` as `symbology`; `wide_narrow` is Code 39's ratio and ignored by the others.
/// The content is normalised first (EAN and UPC drop spaces and hyphens, Code 39 upper-cases,
/// line breaks become spaces) and a missing check digit is appended, a wrong one replaced.
///
/// # Errors
/// [`LinearError`] when the content cannot be encoded.
pub fn encode(
    symbology: Symbology,
    content: &str,
    wide_narrow: f64,
) -> Result<LinearCode, LinearError> {
    match symbology {
        Symbology::Code128 => code128::encode(content),
        Symbology::Code39 => code39::encode(content, wide_narrow),
        Symbology::Ean13 | Symbology::Ean8 | Symbology::UpcA => ean::encode(symbology, content),
    }
}

/// Line breaks become spaces; used by the two text symbologies.
pub(super) fn flatten_lines(content: &str) -> String {
    content
        .chars()
        .map(|c| if matches!(c, '\n' | '\r') { ' ' } else { c })
        .collect()
}
