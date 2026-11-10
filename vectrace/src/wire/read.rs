//! Reads the numbers of a result back into an [`Output`].

use kurbo::{BezPath, PathEl, Point};

use super::{
    CLOSE, CURVE, HEADER, LINE, Layer, MOVE, Output, Palette, Status, TEXT_MARK, Texts, read_texts,
};
use crate::Rgb8 as TraceRgb;
use crate::mask::UNCLIPPED_MARK;

/// A count read from the wire: a whole, non-negative, finite number.
#[must_use]
pub(crate) fn count(value: f64) -> Option<usize> {
    // invariant: checked whole and in range before the cast.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    (value.is_finite() && value >= 0.0 && value.fract() == 0.0 && value < 9.0e15)
        .then_some(value as usize)
}

/// Reads the numbers of [`crate::job::run`] back as they are; `None` when they are not a
/// well-formed result.
#[must_use]
pub fn read(wire: &[f64]) -> Option<Output> {
    read_trace_body(wire).map(|(read, _)| read)
}

/// As [`read`], with where the shape section starts (or would start: after the text
/// section).
pub(crate) fn read_trace_body(wire: &[f64]) -> Option<(Output, usize)> {
    let header = wire.get(..HEADER)?;
    let status = match count(header[0])? {
        0 => Status::Traced,
        1 => Status::TooDetailed,
        2 => Status::Invalid,
        _ => return None,
    };
    let mut at = HEADER;
    let outline_count = count(header[6])?;
    let outlines = read_outlines(wire, &mut at, outline_count)?;
    // `specs/0094`: the unclipped count, when a kernel refusal kept outlines whole.
    let unclipped = if wire.get(at) == Some(&UNCLIPPED_MARK) {
        at += 2;
        count(*wire.get(at - 1)?)?
    } else {
        0
    };
    // `specs/0091`: the text section, when the job had OCR lines.
    let texts = if wire.get(at) == Some(&TEXT_MARK) {
        let (texts, used) = read_texts(&wire[at..])?;
        at += used;
        texts
    } else {
        Texts::default()
    };
    // `specs/0088` Part B: the shape section, when Recognise shapes was on.
    let shapes_at = at;
    let shapes = if wire.get(at) == Some(&crate::shapes::SHAPE_MARK) {
        let (shapes, used) = crate::shapes::read_shapes(&wire[at..], outline_count)?;
        at += used;
        shapes
    } else {
        Vec::new()
    };
    let (palette, layers, joined) = if at == wire.len() {
        (Vec::new(), Vec::new(), 0)
    } else {
        read_colour_tail(&wire[at..], outline_count)?
    };
    let read = Output {
        status,
        paths: count(header[1])?,
        nodes: count(header[2])?,
        specks_removed: count(header[3])?,
        foreground_pixels: u64::try_from(count(header[4])?).ok()?,
        covers_whole_image: header[5] != 0.0,
        outlines,
        palette,
        layers,
        joined,
        texts,
        shapes,
        unclipped,
    };
    Some((read, shapes_at))
}

/// `n` outlines of the numbers of [`write_outline`] from `wire[*at..]`; `at` moves past them.
pub(crate) fn read_outlines(wire: &[f64], at: &mut usize, n: usize) -> Option<Vec<BezPath>> {
    let mut next = || -> Option<f64> {
        let value = *wire.get(*at)?;
        *at += 1;
        value.is_finite().then_some(value)
    };
    let point = |next: &mut dyn FnMut() -> Option<f64>| Some(Point::new(next()?, next()?));
    let mut outlines = Vec::with_capacity(n.min(wire.len()));
    for _ in 0..n {
        let elements = count(next()?)?;
        let mut path = BezPath::new();
        for _ in 0..elements {
            let tag = next()?;
            let element = if tag == MOVE {
                PathEl::MoveTo(point(&mut next)?)
            } else if tag == LINE {
                PathEl::LineTo(point(&mut next)?)
            } else if tag == CURVE {
                PathEl::CurveTo(point(&mut next)?, point(&mut next)?, point(&mut next)?)
            } else if tag == CLOSE {
                PathEl::ClosePath
            } else {
                return None;
            };
            path.push(element);
        }
        outlines.push(path);
    }
    Some(outlines)
}

/// The palette, layers and joined regions after a multicolour result's outlines; `None` unless
/// they are well formed and the layers hold exactly the `outlines`.
fn read_colour_tail(tail: &[f64], outlines: usize) -> Option<(Vec<Palette>, Vec<Layer>, usize)> {
    let mut at = 0;
    let mut next = || -> Option<f64> {
        let value = *tail.get(at)?;
        at += 1;
        value.is_finite().then_some(value)
    };
    let byte = |v: f64| u8::try_from(count(v)?).ok();
    let colour = |next: &mut dyn FnMut() -> Option<f64>| -> Option<TraceRgb> {
        Some(TraceRgb::new(
            byte(next()?)?,
            byte(next()?)?,
            byte(next()?)?,
        ))
    };
    let entries = count(next()?)?;
    let mut palette = Vec::with_capacity(entries.min(tail.len()));
    for _ in 0..entries {
        let c = colour(&mut next)?;
        let fixed = next()? != 0.0;
        let pixels = u64::try_from(count(next()?)?).ok()?;
        palette.push(Palette {
            colour: c,
            fixed,
            pixels,
        });
    }
    let count_layers = count(next()?)?;
    let mut layers = Vec::with_capacity(count_layers.min(tail.len()));
    for _ in 0..count_layers {
        let c = colour(&mut next)?;
        layers.push(Layer {
            colour: c,
            outlines: count(next()?)?,
        });
    }
    let joined = count(next()?)?;
    let held: usize = layers.iter().map(|l| l.outlines).sum();
    (at == tail.len() && held == outlines).then_some((palette, layers, joined))
}
