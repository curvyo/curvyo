//! The text section of the wire: the accepted lines with their fills (`specs/0091-trace-text-recognition`
//! `adrs.md` decisions 1 to 3), written after a result's numbers and read back.

use kurbo::{BezPath, Vec2};

use crate::text_pass::TextPass;
use crate::wire::{count, read_outlines, write_outline};
use crate::{Mm, Rgb8};

/// The number that opens the text section (no count is negative).
pub(crate) const TEXT_MARK: f64 = -1.0;

/// The number that opens the lines' fills after the lines of a multicolour result (slice 2); a
/// black and white text section has none and is as in slice 1.
const FILL_MARK: f64 = -2.0;

/// The number that opens the excluded lines after the fills (their count and places); absent when
/// the maker excluded none of the lines' fills.
const EXCLUDED_MARK: f64 = -3.0;

/// One accepted line of a result.
#[derive(Debug, Clone, PartialEq)]
pub struct Text {
    /// The OCR's string.
    pub text: String,
    /// The line's direction in document millimetres.
    pub direction: Vec2,
    /// Pixels left out of the main trace for it.
    pub masked: usize,
    /// The OCR line passes the garbage filter (Frage 115 B / 117 B): it becomes text even
    /// without a sure letter.
    pub plausible: bool,
    /// The size of one picture pixel along the line, document millimetres (the fit's tolerance,
    /// criterion 26).
    pub pixel: Mm,
    /// Where the reader saw each character, along the line's unit direction (round 3).
    pub positions: Vec<Mm>,
    /// The text's fill in a multicolour result: the palette or background colour nearest the
    /// line's text pixels (slice 2, decision 6); `None` in black and white and in the text pass
    /// alone.
    pub fill: Option<Rgb8>,
    /// The maker excluded the line's fill: it is left out, as the shapes of that colour are
    /// (multicolour only).
    pub excluded: bool,
    /// The line's own outlines in document millimetres.
    pub outlines: Vec<BezPath>,
}

impl Text {
    /// Whether `other` is the same line, its fill aside (the text pass alone has none).
    #[must_use]
    pub fn same_line(&self, other: &Self) -> bool {
        Self {
            fill: None,
            excluded: false,
            ..self.clone()
        } == Self {
            fill: None,
            excluded: false,
            ..other.clone()
        }
    }
}

/// The text section of a result.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Texts {
    /// Lines the OCR found, accepted or not.
    pub found: usize,
    /// The accepted lines with a trace.
    pub lines: Vec<Text>,
}

/// The text section of `pass` after a result's numbers; with `fills` (one per line, multicolour:
/// the colour and whether it is excluded) [`FILL_MARK`] and the colours after the lines, then
/// [`EXCLUDED_MARK`] and the excluded lines when there are any.
pub(crate) fn write_texts(wire: &mut Vec<f64>, pass: &TextPass, fills: &[(Rgb8, bool)]) {
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    {
        wire.extend([TEXT_MARK, pass.found as f64, pass.regions.len() as f64]);
        for region in &pass.regions {
            let chars: Vec<char> = region.text.chars().collect();
            wire.push(chars.len() as f64);
            wire.extend(chars.iter().map(|&c| f64::from(u32::from(c))));
            wire.extend([
                region.direction.x,
                region.direction.y,
                region.masked as f64,
                f64::from(u8::from(region.plausible)),
                region.pixel,
                region.positions.len() as f64,
            ]);
            wire.extend(region.positions.iter().copied());
            wire.extend([region.outlines.len() as f64]);
            for outline in &region.outlines {
                write_outline(wire, outline);
            }
        }
        if !fills.is_empty() {
            wire.push(FILL_MARK);
            for (c, _) in fills {
                wire.extend([f64::from(c.r), f64::from(c.g), f64::from(c.b)]);
            }
            let excluded: Vec<f64> = (0..fills.len())
                .filter(|&k| fills[k].1)
                .map(|k| k as f64)
                .collect();
            if !excluded.is_empty() {
                wire.extend([EXCLUDED_MARK, excluded.len() as f64]);
                wire.extend(excluded);
            }
        }
    }
}

/// The text section at the start of `tail` (it starts with [`TEXT_MARK`]) and the numbers it
/// took; `None` unless it is well formed.
pub(crate) fn read_texts(tail: &[f64]) -> Option<(Texts, usize)> {
    let mut at = 1;
    let next = |at: &mut usize| -> Option<f64> {
        let value = *tail.get(*at)?;
        *at += 1;
        value.is_finite().then_some(value)
    };
    let found = count(next(&mut at)?)?;
    let n = count(next(&mut at)?)?;
    let mut lines = Vec::with_capacity(n.min(tail.len()));
    for _ in 0..n {
        let chars = count(next(&mut at)?)?;
        let mut text = String::new();
        for _ in 0..chars {
            let code = u32::try_from(count(next(&mut at)?)?).ok()?;
            text.push(char::from_u32(code)?);
        }
        let direction = Vec2::new(next(&mut at)?, next(&mut at)?);
        let masked = count(next(&mut at)?)?;
        let plausible = next(&mut at)? != 0.0;
        let pixel = Mm::from_mm(next(&mut at)?);
        let positions_count = count(next(&mut at)?)?;
        let mut positions = Vec::with_capacity(positions_count.min(tail.len()));
        for _ in 0..positions_count {
            positions.push(Mm::from_mm(next(&mut at)?));
        }
        let outlines = count(next(&mut at)?)?;
        let outlines = read_outlines(tail, &mut at, outlines)?;
        lines.push(Text {
            text,
            direction,
            masked,
            plausible,
            pixel,
            positions,
            fill: None,
            excluded: false,
            outlines,
        });
    }
    if tail.get(at) == Some(&FILL_MARK) {
        at += 1;
        let byte = |v: f64| u8::try_from(count(v)?).ok();
        for line in &mut lines {
            line.fill = Some(Rgb8::new(
                byte(next(&mut at)?)?,
                byte(next(&mut at)?)?,
                byte(next(&mut at)?)?,
            ));
        }
        if tail.get(at) == Some(&EXCLUDED_MARK) {
            at += 1;
            for _ in 0..count(next(&mut at)?)? {
                lines.get_mut(count(next(&mut at)?)?)?.excluded = true;
            }
        }
    }
    Some((Texts { found, lines }, at))
}
