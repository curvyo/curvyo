//! Reads a line's string and character confidences with the recogniser from its crop.
//!
//! The line is sampled along its quad at [`LINE_HEIGHT`] px, its width rounded up to a multiple
//! of [`WIDTH_BUCKET`], at least [`MIN_WIDTH`], and padded with the recogniser's zero (mid grey), then decoded greedily:
//! per time step the most likely class, repeats and blanks dropped; a character's confidence is
//! its highest probability over the steps it spans (ADR 0028 §3). A line whose read is unsure or
//! whose quad is steeper than 45° is read once more turned by 180°, and the surer read is kept
//! (`specs/0091` slice 3: upside-down and steep text, no second model).

use rten_tensor::prelude::*;
use rten_tensor::{NdTensor, Tensor};

use crate::ocr::detect::RotatedBox;
use crate::ocr::model::OcrModel;
use crate::ocr::resize::{Grid, Rgb, sample};
use crate::ocr::{OcrError, TextLine};

/// The recogniser's input height, px.
pub const LINE_HEIGHT: usize = 48;

/// Widths are rounded up to a multiple of this, so similar lines share a shape (Proposal).
const WIDTH_BUCKET: usize = 32;
/// The narrowest input: shorter lines are padded to it, which reads them markedly better
/// than a tight pad (spike and reference set, Proposal).
const MIN_WIDTH: usize = 1024;
/// The widest input; a longer line is squeezed into it (Proposal).
const MAX_WIDTH: usize = 3200;
/// A read whose mean character confidence is below this is unsure and the line is read turned
/// too (Proposal, `specs/0091` slice 3, calibrated in its `adrs.md`: upright lines of the posters
/// read at 0.93 and up, upside-down lines at 0.82 and below).
const SURE_READ: f32 = 0.9;
/// A line is sampled this share of its height further at both ends along the line, for the
/// turned read (lead 2026-11-03: the 180° poster's "BÄCKEREI" box ends at its "I").
const END_PAD: f32 = 0.3;
/// A line kept as read turned grows by this share of its height above its text (Proposal,
/// `specs/0091` slice 3): the detector boxes the marks above upright letters but not those below
/// upside-down ones (the reference poster's "BÄCKEREI": 107 px upright, 91 px turned), so the
/// text pass would leave the dots of "Ä" as shapes.
const MARKS_ROOM: f32 = 0.2;
/// A turned read wins only when the first read's doubt (1 minus its mean confidence) is at least
/// this many times its own (Proposal, `specs/0091` slice 3, calibrated: the upside-down lines of
/// the fixtures 12 times and more, the `0025` dot grid's "0000…" against its turned "0000…01"
/// 3.7 times).
const TURNED_DOUBT: f32 = 5.0;
/// A turned read must hold this many letters or digits to win (Proposal: short symbol lines of
/// the screenshot, "3" or "*", read about as well either way and stay as first read).
const MIN_TURNED_LETTERS: usize = 2;

/// One detected line sampled for the recogniser: its quad and its pixels at [`LINE_HEIGHT`] px,
/// before the padding. Lines are independent, so their crops can be read anywhere, in any order.
#[derive(Debug, Clone, PartialEq)]
pub struct LineCrop {
    /// The corners in source pixels: top left, top right, bottom right, bottom left.
    pub quad: [[f32; 2]; 4],
    /// The sampled width of the quad, px.
    pub width: usize,
    /// Columns sampled before and after the quad along the line, read only turned (lead
    /// 2026-11-03: a detector box can end at a letter, which the turned read then loses).
    pub pad: [usize; 2],
    /// `pad[0] + width + pad[1]` x [`LINE_HEIGHT`] RGB samples, row by row, three values each.
    pub rgb: Vec<f32>,
}

impl LineCrop {
    /// Sampled columns per row, the padding included.
    fn columns(&self) -> usize {
        self.pad[0] + self.width + self.pad[1]
    }

    /// The quad alone, without the padding: the samples the line is first read from.
    fn inner(&self) -> Self {
        let (columns, [before, _]) = (self.columns(), self.pad);
        Self {
            quad: self.quad,
            width: self.width,
            pad: [0, 0],
            rgb: (0..LINE_HEIGHT)
                .flat_map(|y| {
                    let start = (y * columns + before) * 3;
                    self.rgb[start..start + self.width * 3].iter().copied()
                })
                .collect(),
        }
    }

    /// The same line with its padding, turned by 180°: the quad grown by the padding starts at
    /// the opposite corner and the samples run backwards (the sampling grid is symmetric, so
    /// they are exactly the turned line's).
    fn turned(&self) -> Self {
        let q = self.quad;
        let step = [
            (q[1][0] - q[0][0]) / self.width as f32,
            (q[1][1] - q[0][1]) / self.width as f32,
        ];
        let [before, after] = self.pad.map(|p| p as f32);
        let back = |p: [f32; 2]| [p[0] - step[0] * before, p[1] - step[1] * before];
        let on = |p: [f32; 2]| [p[0] + step[0] * after, p[1] + step[1] * after];
        Self {
            quad: [on(q[2]), back(q[3]), back(q[0]), on(q[1])],
            width: self.columns(),
            pad: [0, 0],
            rgb: self
                .rgb
                .as_chunks::<3>()
                .0
                .iter()
                .rev()
                .flatten()
                .copied()
                .collect(),
        }
    }

    /// Whether the line's top edge is steeper than 45°, so its reading direction is not known.
    fn steep(&self) -> bool {
        let q = self.quad;
        (q[1][1] - q[0][1]).abs() > (q[1][0] - q[0][0]).abs()
    }
}

/// The mean character confidence of `line`, 0 when nothing was read.
fn mean_confidence(line: Option<&TextLine>) -> f32 {
    line.filter(|l| !l.confidence.is_empty()).map_or(0.0, |l| {
        l.confidence.iter().sum::<f32>() / l.confidence.len() as f32
    })
}

/// How far `from` may move along the unit `direction` and stay on `image`, px (0 when outside).
fn room(image: &Rgb, from: [f32; 2], direction: [f32; 2]) -> f32 {
    let size = [image.width as f32, image.height as f32];
    (0..2)
        .map(|k| {
            if direction[k] > 1e-6 {
                (size[k] - from[k]) / direction[k]
            } else if direction[k] < -1e-6 {
                -from[k] / direction[k]
            } else if (0.0..=size[k]).contains(&from[k]) {
                f32::MAX
            } else {
                0.0
            }
        })
        .fold(f32::MAX, f32::min)
        .max(0.0)
}

/// `quad` of `image` sampled at the recogniser's height, with [`END_PAD`] of its height sampled
/// before and after it along the line where the picture reaches; `None` for a degenerate quad.
pub(crate) fn crop(image: &Rgb, quad: RotatedBox) -> Option<LineCrop> {
    let (len_u, len_v) = (2.0 * quad.half_u, 2.0 * quad.half_v);
    if len_u < 1.0 || len_v < 1.0 {
        return None;
    }
    // invariant: positive, clamped below MAX_WIDTH.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let text_width = ((len_u * LINE_HEIGHT as f32 / len_v).ceil() as usize).clamp(8, MAX_WIDTH);
    let (u, v) = (quad.u, quad.v());
    let corners = quad.quad();
    let (step_u, step_v) = (len_u / text_width as f32, len_v / LINE_HEIGHT as f32);
    let grid = Grid {
        origin: corners[0],
        step_x: [u[0] * step_u, u[1] * step_u],
        step_y: [v[0] * step_v, v[1] * step_v],
        width: text_width,
        height: LINE_HEIGHT,
    };
    let inner = sample(image, &grid);
    let back = [-u[0], -u[1]];
    let reach = |a: f32, b: f32| (END_PAD * len_v).min(a).min(b);
    let lengths = [
        reach(room(image, corners[0], back), room(image, corners[3], back)),
        reach(room(image, corners[1], u), room(image, corners[2], u)),
    ];
    // invariant: non-negative lengths over a positive step, rounded down.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let mut pad = lengths.map(|l| (l / step_u).floor() as usize);
    let spare = MAX_WIDTH - text_width;
    pad[0] = pad[0].min(spare / 2);
    pad[1] = pad[1].min(spare - pad[0]);
    let side = |origin: [f32; 2], width: usize| {
        sample(
            image,
            &Grid {
                origin,
                width,
                ..grid
            },
        )
    };
    let shift = step_u * pad[0] as f32;
    let before = side(
        [corners[0][0] - u[0] * shift, corners[0][1] - u[1] * shift],
        pad[0],
    );
    let after = side(corners[1], pad[1]);
    let mut rgb = Vec::with_capacity((pad[0] + text_width + pad[1]) * LINE_HEIGHT * 3);
    for y in 0..LINE_HEIGHT {
        for row in [
            &before[y * pad[0]..(y + 1) * pad[0]],
            &inner[y * text_width..(y + 1) * text_width],
            &after[y * pad[1]..(y + 1) * pad[1]],
        ] {
            rgb.extend(row.iter().flatten());
        }
    }
    Some(LineCrop {
        quad: corners,
        width: text_width,
        pad,
        rgb,
    })
}

/// The string of `line`, or `None` when nothing was read. A line whose read is unsure or whose
/// quad is steep is read turned by 180° too; the turned read is kept, with its quad turned, when
/// it is sure, clearly surer (its doubt at most a fifth of the first read's) and holds at least
/// two letters or digits (`specs/0091` slice 3).
///
/// # Errors
/// [`OcrError::Invalid`] for a crop whose samples do not match its width, [`OcrError::BadModel`]
/// when inference fails.
pub fn read_crop(model: &OcrModel, line: &LineCrop) -> Result<Option<TextLine>, OcrError> {
    Ok(read_either_way(model, line)?.0)
}

/// [`read_crop`] and whether the line was read turned too.
fn read_either_way(
    model: &OcrModel,
    line: &LineCrop,
) -> Result<(Option<TextLine>, bool), OcrError> {
    if line.rgb.len() != line.columns() * LINE_HEIGHT * 3 || line.columns() > MAX_WIDTH {
        return Err(OcrError::Invalid(format!(
            "a crop {} px wide with {} samples",
            line.columns(),
            line.rgb.len()
        )));
    }
    let first = read_once(model, &line.inner())?;
    let first_mean = mean_confidence(first.as_ref());
    if first_mean >= SURE_READ && !line.steep() {
        return Ok((first, false));
    }
    let turned = read_once(model, &line.turned())?;
    let turned_mean = mean_confidence(turned.as_ref());
    let letters = turned.as_ref().map_or(0, |l| {
        l.text.chars().filter(|c| c.is_alphanumeric()).count()
    });
    let wins = turned_mean >= SURE_READ
        && letters >= MIN_TURNED_LETTERS
        && TURNED_DOUBT * (1.0 - turned_mean) <= 1.0 - first_mean;
    Ok((
        if wins {
            turned.map(with_marks_room)
        } else {
            first
        },
        true,
    ))
}

/// `line` with its quad grown by [`MARKS_ROOM`] of its height above the text.
fn with_marks_room(mut line: TextLine) -> TextLine {
    let q = line.quad;
    let up = [q[0][0] - q[3][0], q[0][1] - q[3][1]];
    for k in [0, 1] {
        line.quad[k] = [q[k][0] + MARKS_ROOM * up[0], q[k][1] + MARKS_ROOM * up[1]];
    }
    line
}

/// One read of `line` as it is sampled, its padding left out.
fn read_once(model: &OcrModel, line: &LineCrop) -> Result<Option<TextLine>, OcrError> {
    let text_width = line.width;
    if text_width == 0 || text_width > MAX_WIDTH || line.rgb.len() != text_width * LINE_HEIGHT * 3 {
        return Err(OcrError::Invalid(format!(
            "a crop {text_width} px wide with {} samples",
            line.rgb.len()
        )));
    }
    let width = (text_width.div_ceil(WIDTH_BUCKET) * WIDTH_BUCKET).max(MIN_WIDTH);
    let input = NdTensor::from_fn([1, 3, LINE_HEIGHT, width], |[_, c, y, x]| {
        if x < text_width {
            // Paddle's order: the blue channel first; [-1, 1].
            (line.rgb[(y * text_width + x) * 3 + 2 - c] / 255.0 - 0.5) / 0.5
        } else {
            0.0
        }
    });
    let output: Tensor<f32> = model
        .recogniser()
        .run_one(input.view().into(), None)
        .map_err(|e| OcrError::BadModel(e.to_string()))?
        .try_into()
        .map_err(|e: rten::TryFromValueError| OcrError::BadModel(e.to_string()))?;
    let shape = output.shape().to_vec();
    if shape.len() != 3 {
        return Err(OcrError::BadModel(format!("recogniser output {shape:?}")));
    }
    let (steps, classes) = (shape[1], shape[2]);
    let (text, confidence, at) = decode(model, &output.to_vec(), steps, classes);
    if text.is_empty() {
        return Ok(None);
    }
    // A step covers `width / steps` input pixels; the line's `text_width` of them span its quad.
    let length = (line.quad[1][0] - line.quad[0][0]).hypot(line.quad[1][1] - line.quad[0][1]);
    let per_step = width as f32 / steps.max(1) as f32;
    let positions = at
        .iter()
        .map(|&step| (step as f32 + 0.5) * per_step * length / text_width as f32)
        .collect();
    Ok(Some(TextLine {
        quad: line.quad,
        text,
        confidence,
        positions,
    }))
}

/// Greedy CTC over `probabilities` (`steps` x `classes`, class 0 the blank): the string without
/// leading and trailing spaces, and each character's confidence.
fn decode(
    model: &OcrModel,
    probabilities: &[f32],
    steps: usize,
    classes: usize,
) -> (String, Vec<f32>, Vec<usize>) {
    let mut chars: Vec<(char, f32, usize)> = Vec::new();
    let mut previous = 0;
    for step in 0..steps {
        let row = &probabilities[step * classes..(step + 1) * classes];
        let (best, p) = row
            .iter()
            .copied()
            .enumerate()
            .fold(
                (0, f32::MIN),
                |(bi, bp), (i, p)| {
                    if p > bp { (i, p) } else { (bi, bp) }
                },
            );
        if best != 0 && best == previous {
            if let Some(last) = chars.last_mut() {
                last.1 = last.1.max(p);
            }
        } else if best != 0
            && let Some(symbol) = model.symbol(best)
        {
            chars.extend(symbol.chars().map(|c| (c, p, step)));
        }
        previous = best;
    }
    let first = chars.iter().position(|(c, _, _)| *c != ' ');
    let last = chars.iter().rposition(|(c, _, _)| *c != ' ');
    match (first, last) {
        (Some(a), Some(b)) => {
            let kept = &chars[a..=b];
            (
                kept.iter().map(|k| k.0).collect(),
                kept.iter().map(|k| k.1).collect(),
                kept.iter().map(|k| k.2).collect(),
            )
        }
        _ => (String::new(), Vec::new(), Vec::new()),
    }
}

#[cfg(test)]
#[path = "../../tests/common/decode.rs"]
mod decode;

#[cfg(test)]
#[path = "../../tests/common/assets.rs"]
mod assets;

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn model() -> Option<OcrModel> {
        let dir = assets::models()?;
        let read = |name: &str| std::fs::read(dir.join(name)).unwrap();
        let alphabet = String::from_utf8(read("dict.txt")).unwrap();
        Some(OcrModel::load(&read("det.onnx"), &read("rec.onnx"), &alphabet).unwrap())
    }

    /// The crops of the fixture `file`.
    fn crops(model: &OcrModel, file: &str) -> Vec<LineCrop> {
        let bytes = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures")
                .join(file),
        )
        .unwrap();
        let image = decode::decode_working(&bytes).unwrap();
        crate::ocr::crop_lines(model, image.width, image.height, &image.pixels, &mut |_| {
            std::ops::ControlFlow::Continue(())
        })
        .unwrap()
    }

    /// A crop turned twice is the crop; turned once, its quad starts at the opposite corner.
    #[test]
    fn a_turned_crop_is_the_line_backwards() {
        let crop = LineCrop {
            quad: [[0.0, 0.0], [2.0, 0.0], [2.0, 1.0], [0.0, 1.0]],
            width: 2,
            pad: [0, 0],
            rgb: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        };
        let turned = crop.turned();
        assert_eq!(turned.rgb, vec![4.0, 5.0, 6.0, 1.0, 2.0, 3.0]);
        assert_eq!(turned.quad[0], [2.0, 1.0]);
        assert_eq!(turned.turned(), crop);
        assert!(!crop.steep() && !turned.steep());
    }

    /// Lead 2026-11-03: the crop is sampled 0.3 of the line's height further at both ends, as far
    /// as the picture reaches; the first read sees the quad's samples as before, the turned read
    /// the padded line with its quad grown by the padding.
    #[test]
    fn the_padding_is_read_only_turned() {
        let Some(model) = model() else { return };
        for crop in crops(&model, "poster.png") {
            let inner = crop.inner();
            assert_eq!(inner.rgb.len(), crop.width * LINE_HEIGHT * 3);
            let height = (crop.quad[3][1] - crop.quad[0][1]).abs();
            let step = (crop.quad[1][0] - crop.quad[0][0]) / crop.width as f32;
            for pad in crop.pad {
                assert!(
                    (pad as f32 * step - 0.3 * height).abs() <= step + 0.5,
                    "{crop:?}"
                );
            }
            let turned = crop.turned();
            assert_eq!(turned.width, crop.columns());
            let grown =
                (turned.quad[0][0] - turned.quad[1][0]) - (crop.quad[1][0] - crop.quad[0][0]);
            assert!((grown - step * (crop.pad[0] + crop.pad[1]) as f32).abs() < 1e-2);
        }
    }

    /// Criteria 32 and 34: on the clean and the noisy poster every line reads sure and level, so
    /// no line is read a second time; the upside-down poster reads every line twice.
    #[test]
    fn only_unsure_or_steep_lines_are_read_twice() {
        let Some(model) = model() else { return };
        for (file, twice) in [
            ("poster.png", 0),
            ("poster_noisy.jpg", 0),
            ("turned/poster_180.png", 8),
        ] {
            let crops = crops(&model, file);
            let read_twice = crops
                .iter()
                .filter(|c| read_either_way(&model, c).unwrap().1)
                .count();
            assert_eq!(read_twice, twice, "{file}");
        }
    }
}
