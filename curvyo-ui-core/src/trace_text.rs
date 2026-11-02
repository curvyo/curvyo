//! Turns OCR lines into region outlines and the pixels the trace leaves out
//! (`specs/0091-trace-text-recognition` `adrs.md` decisions 2 and 3).
//!
//! Per accepted line: Otsu over the pixels inside its quad picks the line's own Threshold; the
//! class that touches the border of the quad's box more is the background, so white text on a
//! dark box is foreground; the quad is traced on its own with the current settings, the
//! foreground made dark; the foreground pixels inside the quad, grown by 1 px, are left out of
//! the main trace and filled from their surroundings. Integers and fixed orders as `0089` §8.

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use curvyo_vectorize_core::{Raster, Selection, TraceParams, auto_threshold, trace};
use kurbo::{BezPath, Vec2};

use crate::trace_inpaint::inpaint;
use crate::trace_ocr::OcrLine;

/// The box of a quad grows by this many pixels on each side (decision 2).
const BOX_MARGIN: f64 = 2.0;
/// A quad grows by this share of its height on each side before it is traced and masked
/// (Proposal): the detector's box is tight around the letters' bodies, and umlaut dots, accents
/// and descenders reach past it.
const QUAD_GROWTH: f64 = 0.2;

/// `quad` grown by [`QUAD_GROWTH`] of its height along its own axes.
fn grown(quad: &[[f64; 2]; 4]) -> [[f64; 2]; 4] {
    let across = [quad[3][0] - quad[0][0], quad[3][1] - quad[0][1]];
    let height = across[0].hypot(across[1]);
    let along = [quad[1][0] - quad[0][0], quad[1][1] - quad[0][1]];
    let length = along[0].hypot(along[1]);
    if height <= 0.0 || length <= 0.0 {
        return *quad;
    }
    let g = QUAD_GROWTH * height;
    let (u, v) = (
        [along[0] / length * g, along[1] / length * g],
        [across[0] / height * g, across[1] / height * g],
    );
    let at = |p: [f64; 2], su: f64, sv: f64| {
        [p[0] + u[0] * su + v[0] * sv, p[1] + u[1] * su + v[1] * sv]
    };
    [
        at(quad[0], -1.0, -1.0),
        at(quad[1], 1.0, -1.0),
        at(quad[2], 1.0, 1.0),
        at(quad[3], -1.0, 1.0),
    ]
}

/// One accepted line after the text pass.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRegion {
    /// The OCR's string.
    pub text: String,
    /// The line's own trace, in document millimetres.
    pub outlines: Vec<BezPath>,
    /// The line's direction in document millimetres (the quad's top edge).
    pub direction: Vec2,
    /// Pixels left out of the main trace for it.
    pub masked: usize,
    /// The OCR line passes the garbage filter ([`OcrLine::plausible`]).
    pub plausible: bool,
    /// The size of one picture pixel along the line, document millimetres.
    pub pixel: f64,
}

/// What the text pass hands to the main trace and back to the session.
#[derive(Debug, Clone, PartialEq)]
pub struct TextPass {
    /// The picture with the left-out pixels filled from their surroundings.
    pub rgba: Vec<u8>,
    /// Per pixel: left out.
    pub left_out: Vec<bool>,
    /// The accepted lines with a trace, in line order.
    pub regions: Vec<TextRegion>,
    /// Lines the OCR found, accepted or not.
    pub found: usize,
}

/// The luma of the main trace (Rec. 709, as `vectorize-core`).
fn luma(px: &[u8]) -> u8 {
    let weighted = 2126 * u32::from(px[0]) + 7152 * u32::from(px[1]) + 722 * u32::from(px[2]);
    u8::try_from((weighted + 5000) / 10_000).unwrap_or(u8::MAX)
}

/// Whether the pixel centre `(x + 0.5, y + 0.5)` lies inside the convex `quad`.
fn inside(quad: &[[f64; 2]; 4], x: usize, y: usize) -> bool {
    // invariant: pixel indices far below 2^52.
    #[allow(clippy::cast_precision_loss)]
    let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
    // The side of every edge the centre lies on; on an edge counts as either side.
    let (mut left, mut right) = (false, false);
    for k in 0..4 {
        let (a, b) = (quad[k], quad[(k + 1) % 4]);
        let cross = (b[0] - a[0]) * (py - a[1]) - (b[1] - a[1]) * (px - a[0]);
        left |= cross > 0.0;
        right |= cross < 0.0;
    }
    !(left && right)
}

/// The pixel box of `quad` grown by [`BOX_MARGIN`], clamped: `(x0, y0, x1, y1)` exclusive end.
fn pixel_box(
    quad: &[[f64; 2]; 4],
    width: usize,
    height: usize,
) -> Option<(usize, usize, usize, usize)> {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in quad {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    // invariant: clamped into the picture before the casts.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let clamp = |v: f64, max: usize| v.clamp(0.0, max as f64) as usize;
    let b = (
        clamp((x0 - BOX_MARGIN).floor(), width),
        clamp((y0 - BOX_MARGIN).floor(), height),
        clamp((x1 + BOX_MARGIN).ceil(), width),
        clamp((y1 + BOX_MARGIN).ceil(), height),
    );
    (b.0 < b.2 && b.1 < b.3).then_some(b)
}

/// One line's Threshold and polarity: `(threshold, light_text)`.
fn threshold_and_polarity(
    rgba: &[u8],
    width: usize,
    quad: &[[f64; 2]; 4],
    (x0, y0, x1, y1): (usize, usize, usize, usize),
) -> (u8, bool) {
    let mut pixels: Vec<u8> = Vec::new();
    for y in y0..y1 {
        for x in x0..x1 {
            if inside(quad, x, y) {
                let i = (y * width + x) * 4;
                pixels.extend_from_slice(&rgba[i..i + 4]);
            }
        }
    }
    let threshold = auto_threshold(Raster {
        width: u32::try_from(pixels.len() / 4).unwrap_or(0),
        height: 1,
        rgba: &pixels,
    });
    // The class that touches the box's border more is the background.
    let (mut dark, mut light) = (0_usize, 0_usize);
    for y in y0..y1 {
        for x in x0..x1 {
            if y != y0 && y + 1 != y1 && x != x0 && x + 1 != x1 {
                continue;
            }
            let i = (y * width + x) * 4;
            if luma(&rgba[i..i + 4]) < threshold {
                dark += 1;
            } else {
                light += 1;
            }
        }
    }
    (threshold, dark > light)
}

/// The text pass over `rgba` (`width` x `height`, straight alpha) mapped by `pixel_map`, with the
/// trace settings `params` and the OCR `lines` (in this picture's pixels). Lines that are not
/// accepted, or whose own trace has no outline, are left to the main trace.
#[must_use]
pub fn text_pass(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: Affine,
    params: &TraceParams,
    lines: &[OcrLine],
) -> TextPass {
    let (w, h) = (width as usize, height as usize);
    let mut pass = TextPass {
        rgba: rgba.to_vec(),
        left_out: vec![false; w * h],
        regions: Vec::new(),
        found: lines.len(),
    };
    if rgba.len() != w * h * 4 {
        return pass;
    }
    for line in lines.iter().filter(|l| l.accepted()) {
        let quad = grown(&line.quad);
        let Some(bounds) = pixel_box(&quad, w, h) else {
            continue;
        };
        let (threshold, light_text) = threshold_and_polarity(rgba, w, &quad, bounds);
        let foreground = |px: &[u8]| (luma(px) < threshold) != light_text;
        let (x0, y0, x1, y1) = bounds;
        let (cw, ch) = (x1 - x0, y1 - y0);
        // The quad on its own, foreground dark and with its grey kept, white around it.
        let mut crop = vec![255_u8; cw * ch * 4];
        let mut text = vec![false; w * h];
        for y in y0..y1 {
            for x in x0..x1 {
                if !inside(&quad, x, y) {
                    continue;
                }
                let i = (y * w + x) * 4;
                let px = &rgba[i..i + 4];
                let level = if light_text { 255 - luma(px) } else { luma(px) };
                let c = ((y - y0) * cw + (x - x0)) * 4;
                crop[c..c + 3].fill(level);
                text[y * w + x] = foreground(px);
            }
        }
        let own = TraceParams {
            selection: Selection::Brightness,
            threshold: if light_text {
                255 - threshold + 1
            } else {
                threshold
            }
            .max(1),
            invert: false,
            ignore_background: false,
            // A letter's dot or a full stop is no speck.
            specks: curvyo_document_core::Length::from_mm(0.0),
            ..*params
        };
        // The crop's pixels moved to their place in the picture, then mapped (`compose` applies
        // its right operand first).
        // invariant: the box lies inside the picture.
        #[allow(clippy::cast_precision_loss)]
        let map = pixel_map.compose(Affine::translation(curvyo_document_core::Vec2::new(
            x0 as f64, y0 as f64,
        )));
        let Ok(result) = trace(
            Raster {
                width: u32::try_from(cw).unwrap_or(0),
                height: u32::try_from(ch).unwrap_or(0),
                rgba: &crop,
            },
            map,
            &own,
            &mut |_| ControlFlow::Continue(()),
        ) else {
            continue;
        };
        if result.outlines.is_empty() {
            continue;
        }
        let masked = grow_into(&text, &mut pass.left_out, w, h, bounds);
        let q = line.quad;
        let edge = Vec2::new(q[1][0] - q[0][0], q[1][1] - q[0][1]);
        let m = pixel_map.to_array();
        let direction = Vec2::new(m[0] * edge.x + m[2] * edge.y, m[1] * edge.x + m[3] * edge.y);
        let edge_px = edge.hypot();
        pass.regions.push(TextRegion {
            text: line.text.clone(),
            outlines: result.outlines,
            direction,
            masked,
            plausible: line.plausible(),
            pixel: if edge_px > 0.0 {
                direction.hypot() / edge_px
            } else {
                0.0
            },
        });
    }
    inpaint(&mut pass.rgba, w, h, &pass.left_out);
    pass
}

/// Sets in `left_out` every pixel of `text` and its 8 neighbours (within `bounds` grown by one);
/// returns how many pixels that line leaves out.
fn grow_into(
    text: &[bool],
    left_out: &mut [bool],
    width: usize,
    height: usize,
    (x0, y0, x1, y1): (usize, usize, usize, usize),
) -> usize {
    let mut count = 0;
    for y in y0.saturating_sub(1)..(y1 + 1).min(height) {
        for x in x0.saturating_sub(1)..(x1 + 1).min(width) {
            let near = (y.saturating_sub(1)..(y + 2).min(height)).any(|ny| {
                (x.saturating_sub(1)..(x + 2).min(width)).any(|nx| text[ny * width + nx])
            });
            if near {
                count += 1;
                left_out[y * width + x] = true;
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 40 x 20 picture: a grey box with a white bar (the "text") in it, the rest white.
    fn picture() -> Vec<u8> {
        let mut rgba = vec![255_u8; 40 * 20 * 4];
        for y in 2..18 {
            for x in 2..38 {
                let level = if (8..12).contains(&y) && (8..32).contains(&x) {
                    255
                } else {
                    90
                };
                let i = (y * 40 + x) * 4;
                rgba[i..i + 3].fill(level);
            }
        }
        rgba
    }

    fn line() -> OcrLine {
        OcrLine {
            quad: [[6.0, 6.0], [34.0, 6.0], [34.0, 14.0], [6.0, 14.0]],
            text: "Il".to_owned(),
            confidence: vec![0.9, 0.9],
        }
    }

    /// Decisions 2 and 3: white text on a grey box is foreground; its pixels are left out and
    /// filled with the box's grey, so the box is whole; the line has its own outlines.
    #[test]
    fn light_text_on_a_box_is_left_out_and_the_box_stays_whole() {
        let rgba = picture();
        let pass = text_pass(
            &rgba,
            40,
            20,
            Affine::scale(0.1, 0.1),
            &TraceParams::with_threshold(128),
            &[line()],
        );
        assert_eq!(pass.regions.len(), 1);
        assert_ne!(pass.regions[0].outlines.len(), 0);
        assert!((pass.regions[0].direction.x - 2.8).abs() < 1e-9);
        let at = |x: usize, y: usize| pass.rgba[(y * 40 + x) * 4];
        assert_eq!(at(20, 10), 90);
        assert!(pass.left_out[10 * 40 + 20] && !pass.left_out[3 * 40 + 3]);
        assert_eq!(pass.regions[0].masked, 26 * 6);
    }

    /// Criterion 9: a line's own outlines lie on its letters in the document: the crop is moved
    /// to its place in the picture, then mapped by the picture's map (0.1 mm per pixel here).
    #[test]
    fn a_line_traces_where_its_letters_are() {
        let rgba = picture();
        let pass = text_pass(
            &rgba,
            40,
            20,
            Affine::new(0.1, 0.0, 0.0, 0.1, 5.0, 7.0),
            &TraceParams::with_threshold(128),
            &[line()],
        );
        let b = pass.regions[0]
            .outlines
            .iter()
            .map(kurbo::Shape::bounding_box)
            .reduce(|a, b| a.union(b))
            .unwrap_or_default();
        // The white bar spans pixels 8..32 x 8..12: 5.8..8.2 mm x 7.8..8.2 mm.
        let near = |a: f64, b: f64| (a - b).abs() <= 0.15;
        assert!(
            near(b.x0, 5.8) && near(b.x1, 8.2) && near(b.y0, 7.8) && near(b.y1, 8.2),
            "{b:?}"
        );
    }

    #[test]
    fn a_line_that_is_not_accepted_changes_nothing() {
        let rgba = picture();
        let mut weak = line();
        weak.confidence = vec![0.1, 0.1];
        let pass = text_pass(
            &rgba,
            40,
            20,
            Affine::scale(0.1, 0.1),
            &TraceParams::with_threshold(128),
            &[weak],
        );
        assert!(pass.regions.is_empty() && pass.found == 1);
        assert_eq!(pass.rgba, rgba);
    }
}
