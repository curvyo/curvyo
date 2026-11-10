//! Turns OCR lines into region outlines and the pixels the trace leaves out
//! (`specs/0091-trace-text-recognition` `adrs.md` decisions 2 and 3).
//!
//! Per accepted line: Otsu over the pixels inside its quad picks the line's own Threshold; the
//! class that touches the border of the quad's box more is the background, so white text on a
//! dark box is foreground; the quad is traced on its own with the current settings, the
//! foreground made dark; the foreground pixels inside the quad, grown by 1 px, are left out of
//! the main trace and filled from their surroundings. Integers and fixed orders as `0089` §8.

use std::ops::ControlFlow;

use crate::{Mm, PixelMap, Raster, Rgb8, Selection, TraceParams, auto_threshold, trace};
use kurbo::{BezPath, Point, Vec2};

use crate::inpaint::{inpaint, median};
use crate::ocr::OcrLine;

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
pub(crate) struct TextRegion {
    /// The OCR's string.
    pub text: String,
    /// The line's own trace, in document millimetres.
    pub outlines: Vec<BezPath>,
    /// The line's direction in document millimetres (the quad's top edge).
    pub direction: Vec2,
    /// The centre of the line's box (its quad) in document millimetres: the point a mask's
    /// region tests (`specs/0094` criterion 11).
    pub centre: Point,
    /// Pixels left out of the main trace for it.
    pub masked: usize,
    /// The OCR line passes the garbage filter ([`OcrLine::plausible`]).
    pub plausible: bool,
    /// The size of one picture pixel along the line, document millimetres.
    pub pixel: f64,
    /// Where the reader saw each character: document millimetres along the line's unit
    /// direction (empty when not known).
    pub positions: Vec<f64>,
    /// The per-channel lower median of the line's text pixels (`specs/0091` decision 6).
    pub colour: Rgb8,
    /// The same median of those pixels once filled from their surroundings: what lies under the
    /// text (slice 2, the background candidate).
    pub under: Rgb8,
}

/// What the text pass hands to the main trace and back to the session.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextPass {
    /// The picture with the left-out pixels filled from their surroundings.
    pub rgba: Vec<u8>,
    /// Per pixel: left out.
    pub left_out: Vec<bool>,
    /// The accepted lines with a trace, in line order.
    pub regions: Vec<TextRegion>,
    /// Lines the OCR found, accepted or not.
    pub found: usize,
}

/// The luma of the main trace's pixel `px`.
fn luma(px: &[u8]) -> u8 {
    crate::luma(px[0], px[1], px[2])
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

/// The text pass over `raster` (straight alpha) mapped by `pixel_map`, with the trace settings
/// `params` and the OCR `lines` (in this picture's pixels). Lines that are not accepted, or whose
/// own trace has no outline, are left to the main trace.
#[must_use]
pub(crate) fn text_pass(
    raster: Raster<'_>,
    pixel_map: PixelMap,
    params: &TraceParams,
    lines: &[OcrLine],
) -> TextPass {
    let rgba = raster.rgba;
    let (w, h) = (raster.width as usize, raster.height as usize);
    let mut pass = TextPass {
        rgba: rgba.to_vec(),
        left_out: vec![false; w * h],
        regions: Vec::new(),
        found: lines.len(),
    };
    if rgba.len() != w * h * 4 {
        return pass;
    }
    // Each region's text pixels, for what lies under them after the filling.
    let mut inks: Vec<Vec<usize>> = Vec::new();
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
        let mut inked: Vec<usize> = Vec::new();
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
                if foreground(px) {
                    text[y * w + x] = true;
                    inked.push(y * w + x);
                }
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
            specks: Mm::from_mm(0.0),
            ..*params
        };
        // The crop's pixels moved to their place in the picture, then mapped (`compose` applies
        // its right operand first).
        // invariant: the box lies inside the picture.
        #[allow(clippy::cast_precision_loss)]
        let map = pixel_map.compose(PixelMap::translation(Vec2::new(x0 as f64, y0 as f64)));
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
        let positions = reader_positions(&line.positions, &q, m, direction);
        let ink = median(rgba, &mut inked);
        inks.push(inked);
        pass.regions.push(TextRegion {
            text: line.text.clone(),
            outlines: result.outlines,
            direction,
            centre: quad_centre(&q, pixel_map),
            masked,
            plausible: line.plausible(),
            positions,
            colour: Rgb8::new(ink[0], ink[1], ink[2]),
            under: Rgb8::new(ink[0], ink[1], ink[2]),
            pixel: if edge_px > 0.0 {
                direction.hypot() / edge_px
            } else {
                0.0
            },
        });
    }
    inpaint(&mut pass.rgba, w, h, &pass.left_out);
    set_under(&mut pass.regions, &pass.rgba, &mut inks);
    pass
}

/// The centre of `quad` (picture pixels) in document millimetres.
fn quad_centre(quad: &[[f64; 2]; 4], pixel_map: PixelMap) -> Point {
    let mid = quad
        .iter()
        .fold([0.0, 0.0], |c, p| [c[0] + p[0] / 4.0, c[1] + p[1] / 4.0]);
    pixel_map.apply_point(Point::new(mid[0], mid[1]))
}

/// Sets what lies under each of `regions`: the median of its text pixels `inks` in the filled
/// picture `rgba`.
fn set_under(regions: &mut [TextRegion], rgba: &[u8], inks: &mut [Vec<usize>]) {
    for (region, inked) in regions.iter_mut().zip(inks) {
        let under = median(rgba, inked);
        region.under = Rgb8::new(under[0], under[1], under[2]);
    }
}

/// The reader's character `positions` (pixels along `quad`'s top edge) as document mm along the
/// unit `direction`, through the pixel map's coefficients `m`.
fn reader_positions(
    positions: &[f64],
    quad: &[[f64; 2]; 4],
    m: [f64; 6],
    direction: Vec2,
) -> Vec<f64> {
    let edge = Vec2::new(quad[1][0] - quad[0][0], quad[1][1] - quad[0][1]);
    let (along, unit) = (edge.hypot(), direction.hypot());
    if along <= 0.0 || unit <= 0.0 {
        return Vec::new();
    }
    positions
        .iter()
        .map(|p| {
            let (px, py) = (
                quad[0][0] + edge.x / along * p,
                quad[0][1] + edge.y / along * p,
            );
            let (dx, dy) = (m[0] * px + m[2] * py + m[4], m[1] * px + m[3] * py + m[5]);
            (dx * direction.x + dy * direction.y) / unit
        })
        .collect()
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

    fn raster(rgba: &[u8]) -> Raster<'_> {
        Raster {
            width: 40,
            height: 20,
            rgba,
        }
    }

    fn line() -> OcrLine {
        OcrLine {
            quad: [[6.0, 6.0], [34.0, 6.0], [34.0, 14.0], [6.0, 14.0]],
            text: "Il".to_owned(),
            confidence: vec![0.9, 0.9],
            positions: Vec::new(),
        }
    }

    /// Decisions 2 and 3: white text on a grey box is foreground; its pixels are left out and
    /// filled with the box's grey, so the box is whole; the line has its own outlines.
    #[test]
    fn light_text_on_a_box_is_left_out_and_the_box_stays_whole() {
        let rgba = picture();
        let pass = text_pass(
            raster(&rgba),
            PixelMap::scale(0.1, 0.1),
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
        // Decision 6: the line's colour is its text pixels', white here.
        assert_eq!(pass.regions[0].colour, Rgb8::new(255, 255, 255));
        // Slice 2: under the text lies the box's grey.
        assert_eq!(pass.regions[0].under, Rgb8::new(90, 90, 90));
    }

    /// Criterion 9: a line's own outlines lie on its letters in the document: the crop is moved
    /// to its place in the picture, then mapped by the picture's map (0.1 mm per pixel here).
    #[test]
    fn a_line_traces_where_its_letters_are() {
        let rgba = picture();
        let pass = text_pass(
            raster(&rgba),
            PixelMap::new([0.1, 0.0, 0.0, 0.1, 5.0, 7.0]),
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
            raster(&rgba),
            PixelMap::scale(0.1, 0.1),
            &TraceParams::with_threshold(128),
            &[weak],
        );
        assert!(pass.regions.is_empty() && pass.found == 1);
        assert_eq!(pass.rgba, rgba);
    }
}
