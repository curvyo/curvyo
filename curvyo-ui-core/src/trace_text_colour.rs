//! The multicolour trace job with OCR lines (`specs/0091-trace-text-recognition` slice 2,
//! criteria 23 and 24; `adrs.md` decisions 3 and 6).
//!
//! The text pass runs as in black and white. The palette is the one of the picture as it is (the
//! palette Recognise text off finds), so a colour only the text has stays a palette colour; the
//! filled picture is then traced with exactly that palette, and each accepted line's fill is the
//! palette or background colour nearest its text pixels, through the maker's Merge into.

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use curvyo_vectorize_core::{
    ColourMask, ColourParams, ColourTrace, PaletteEntry, Progress, Raster, Rgb8, Selection,
    TraceError, colour_mask, suggest_background, trace_colours,
};

use crate::trace_ocr::OcrLine;
use crate::trace_text::{TextPass, text_pass};
use crate::trace_text_wire::{LEFT_OUT_LEVEL, write_texts};
use crate::trace_transfer::{
    TraceStatus, colour_head, job_percent, status_only, trace_colour_job_with,
    trace_colour_mask_job, write_colour_tail,
};

/// The share of a job's percent the palette of the unfilled picture takes (Proposal: it is the
/// labelling part of a trace).
const PALETTE_PERCENT: u32 = 25;

/// One multicolour trace with the OCR `lines` (in this picture's pixels): the text pass, the
/// palette of the picture as it is, the trace of the filled picture with that palette, then the
/// numbers of [`trace_colour_job_with`] with the text section (each line's fill) between the
/// outlines and the palette. Without lines the numbers are exactly those of
/// [`trace_colour_job_with`] (criteria 12 and 24).
#[must_use]
pub fn trace_text_colour_job_with(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    params: &ColourParams,
    lines: &[OcrLine],
    on_percent: &mut dyn FnMut(u8),
) -> Vec<f64> {
    if lines.is_empty() {
        return trace_colour_job_with(rgba, width, height, pixel_map, params, on_percent);
    }
    let map = Affine::from_array(pixel_map);
    let pass = text_pass(rgba, width, height, map, &params.trace, lines);
    let raster = Raster {
        width,
        height,
        rgba,
    };
    let mut reported: Option<u8> = None;
    let mut report = |percent: u32| {
        let percent = u8::try_from(percent.min(100)).unwrap_or(100);
        if reported.is_none_or(|last| percent > last) {
            reported = Some(percent);
            on_percent(percent);
        }
    };
    let traced = palette_of_picture(raster, map, params, &pass, &mut |p| {
        report(u32::from(job_percent(p)) * PALETTE_PERCENT / 100);
    })
    .and_then(|palette| {
        let filled = Raster {
            width,
            height,
            rgba: &pass.rgba,
        };
        trace_with_palette(filled, map, params, palette.as_deref(), &mut |p| {
            report(PALETTE_PERCENT + u32::from(job_percent(p)) * (100 - PALETTE_PERCENT) / 100);
        })
    });
    match traced {
        Ok(result) => {
            report(100);
            let fills = line_fills(&pass, &result.palette, raster, params);
            let mut wire = colour_head(&result);
            write_texts(&mut wire, &pass, &fills);
            write_colour_tail(&mut wire, &result);
            wire
        }
        Err(TraceError::TooDetailed) => status_only(TraceStatus::TooDetailed),
        Err(TraceError::Invalid(_) | TraceError::Cancelled) => status_only(TraceStatus::Invalid),
    }
}

/// The segmented palette the trace of the same arguments walks (`specs/0089` UX notes §4), the
/// pixels the text pass leaves out in [`LEFT_OUT_LEVEL`] grey (UX notes §2); without lines
/// exactly [`trace_colour_mask_job`].
#[must_use]
pub fn trace_text_colour_mask_job(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    params: &ColourParams,
    lines: &[OcrLine],
) -> Option<ColourMask> {
    if lines.is_empty() {
        return trace_colour_mask_job(rgba, width, height, pixel_map, params);
    }
    let map = Affine::from_array(pixel_map);
    let pass = text_pass(rgba, width, height, map, &params.trace, lines);
    let raster = Raster {
        width,
        height,
        rgba,
    };
    let palette = palette_of_picture(raster, map, params, &pass, &mut |_| {}).ok()?;
    let own = with_palette(params, palette.as_deref());
    let filled = Raster {
        width,
        height,
        rgba: &pass.rgba,
    };
    let (mut mask, _) = colour_mask(filled, map, &own, &mut |_| ControlFlow::Continue(())).ok()?;
    let (w, mw) = (width as usize, mask.width as usize);
    let factor = (mw / w.max(1)).max(1);
    let grey = [LEFT_OUT_LEVEL, LEFT_OUT_LEVEL, LEFT_OUT_LEVEL, 255];
    for (i, _) in pass.left_out.iter().enumerate().filter(|(_, out)| **out) {
        let (x, y) = (i % w, i / w);
        for dy in 0..factor {
            for dx in 0..factor {
                let at = ((y * factor + dy) * mw + x * factor + dx) * 4;
                if let Some(px) = mask.rgba.get_mut(at..at + 4) {
                    px.copy_from_slice(&grey);
                }
            }
        }
    }
    Some(mask)
}

/// The palette of `raster` as it is, for the trace of the filled picture: `None` when the text
/// pass changed no pixel or Pick colours sets the palette (the filled picture's palette is then
/// the same).
fn palette_of_picture(
    raster: Raster<'_>,
    map: Affine,
    params: &ColourParams,
    pass: &TextPass,
    progress: &mut dyn FnMut(Progress),
) -> Result<Option<Vec<PaletteEntry>>, TraceError> {
    if pass.regions.is_empty() || matches!(params.trace.selection, Selection::PickColours { .. }) {
        return Ok(None);
    }
    let (_, palette) = colour_mask(raster, map, params, &mut |p| {
        progress(p);
        ControlFlow::Continue(())
    })?;
    Ok(Some(palette))
}

/// `params` with `palette` as its fixed colours and their count, so the trace keeps exactly them.
fn with_palette(params: &ColourParams, palette: Option<&[PaletteEntry]>) -> ColourParams {
    match palette {
        Some(palette) if !palette.is_empty() => ColourParams {
            fixed: palette.iter().map(|e| e.colour).collect(),
            // invariant: a palette holds at most 16 colours.
            colours: Some(u8::try_from(palette.len()).unwrap_or(u8::MAX)),
            ..params.clone()
        },
        _ => params.clone(),
    }
}

/// The trace of the filled picture with `palette` (or its own palette for `None`); the palette
/// reported is `palette` with its marks and pixels, as Recognise text off reports it.
fn trace_with_palette(
    filled: Raster<'_>,
    map: Affine,
    params: &ColourParams,
    palette: Option<&[PaletteEntry]>,
    progress: &mut dyn FnMut(Progress),
) -> Result<ColourTrace, TraceError> {
    let own = with_palette(params, palette);
    let mut result = trace_colours(filled, map, &own, &mut |p| {
        progress(p);
        ControlFlow::Continue(())
    })?;
    if let Some(palette) = palette {
        let same = result.palette.len() == palette.len()
            && result
                .palette
                .iter()
                .zip(palette)
                .all(|(a, b)| a.colour == b.colour);
        if same {
            result.palette = palette.to_vec();
            for layer in &mut result.layers {
                layer.fixed = palette.iter().any(|e| e.fixed && e.colour == layer.sample);
            }
        }
    }
    Ok(result)
}

/// The background colour of a multicolour trace of `raster`: Remove background's or Pick
/// colours' colour, else the suggested one (as the trace's palette uses it).
fn background_of(raster: Raster<'_>, params: &ColourParams) -> Option<Rgb8> {
    match params.trace.selection {
        Selection::RemoveBackground { background, .. }
        | Selection::PickColours { background, .. } => background,
        Selection::Brightness => suggest_background(raster),
    }
}

/// Excluded colours match within this distance, as the palette's own Exclude does (0.03 of the
/// sRGB unit cube, `vectorize-core` `MERGE_DISTANCE`).
const EXCLUDE_DISTANCE_SQUARED: i32 = 58; // 7.65 squared, rounded down

/// The fill of every accepted line of `pass` (decision 6) and whether the maker excluded it: the
/// colour of `palette` nearest the line's colour, then through the maker's Merge into. The
/// background colour of `raster` is a candidate only when the text lies on a shape (what is under
/// it is nearer a palette colour than the background, white text on a box); text on the paper
/// takes a palette colour, so pale text never becomes the paper's colour (Proposal, lead
/// 2026-11-02). A fill the maker excluded leaves the line out, as its shapes are.
fn line_fills(
    pass: &TextPass,
    palette: &[PaletteEntry],
    raster: Raster<'_>,
    params: &ColourParams,
) -> Vec<(Rgb8, bool)> {
    let colours: Vec<Rgb8> = palette.iter().map(|e| e.colour).collect();
    let background = background_of(raster, params);
    let with_background: Vec<Rgb8> = colours.iter().copied().chain(background).collect();
    pass.regions
        .iter()
        .map(|region| {
            let on_shape =
                background.is_none_or(|b| nearest(region.under, &with_background) != Some(b));
            let candidates = if on_shape { &with_background } else { &colours };
            let fill = nearest(region.colour, candidates).unwrap_or(region.colour);
            let fill = params
                .merged
                .iter()
                .find(|(from, _)| *from == fill)
                .map_or(fill, |&(_, into)| into);
            let excluded = params
                .excluded
                .iter()
                .any(|e| distance_squared(*e, fill) <= EXCLUDE_DISTANCE_SQUARED);
            (fill, excluded)
        })
        .collect()
}

/// The squared sRGB distance of `a` and `b`, in bytes.
fn distance_squared(a: Rgb8, b: Rgb8) -> i32 {
    let d = |x: u8, y: u8| (i32::from(x) - i32::from(y)).pow(2);
    d(a.r, b.r) + d(a.g, b.g) + d(a.b, b.b)
}

/// The colour of `candidates` nearest `colour` (squared sRGB distance, the first on a tie).
fn nearest(colour: Rgb8, candidates: &[Rgb8]) -> Option<Rgb8> {
    let mut best: Option<(i32, Rgb8)> = None;
    for c in candidates {
        let at = distance_squared(*c, colour);
        if best.is_none_or(|(b, _)| at < b) {
            best = Some((at, *c));
        }
    }
    best.map(|(_, c)| c)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace_transfer::read_trace_wire;
    use curvyo_document_core::Length;
    use curvyo_vectorize_core::TraceParams;

    const RED: [u8; 3] = [208, 34, 42];
    const BLUE: [u8; 3] = [35, 78, 184];
    const W: u32 = 60;
    const H: u32 = 30;
    const MAP: [f64; 6] = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

    /// A 60 x 30 white picture: `paint` gives each pixel's colour or `None` for white.
    fn picture(paint: impl Fn(usize, usize) -> Option<[u8; 3]>) -> Vec<u8> {
        (0..(W * H) as usize)
            .flat_map(|i| {
                let [r, g, b] = paint(i % W as usize, i / W as usize).unwrap_or([255; 3]);
                [r, g, b, 255]
            })
            .collect()
    }

    /// The bar the "text" is: 40 x 6 px.
    fn in_bar(x: usize, y: usize) -> bool {
        (10..50).contains(&x) && (12..18).contains(&y)
    }

    /// White text on a red box.
    fn white_on_red() -> Vec<u8> {
        picture(|x, y| {
            let in_box = (2..58).contains(&x) && (2..28).contains(&y);
            (in_box && !in_bar(x, y)).then_some(RED)
        })
    }

    /// Red text on white with a blue box to its right.
    fn red_on_white() -> Vec<u8> {
        picture(|x, y| {
            if in_bar(x, y) {
                Some(RED)
            } else if (54..59).contains(&x) && (4..26).contains(&y) {
                Some(BLUE)
            } else {
                None
            }
        })
    }

    fn line() -> OcrLine {
        OcrLine {
            quad: [[8.0, 10.0], [52.0, 10.0], [52.0, 20.0], [8.0, 20.0]],
            text: "Il".to_owned(),
            confidence: vec![0.9, 0.9],
            positions: Vec::new(),
        }
    }

    fn params() -> ColourParams {
        ColourParams::new(TraceParams {
            selection: Selection::RemoveBackground {
                background: Some(Rgb8::new(255, 255, 255)),
                tolerance: 40,
            },
            specks: Length::from_mm(0.0),
            ..TraceParams::with_threshold(128)
        })
    }

    fn rgb(c: [u8; 3]) -> Rgb8 {
        Rgb8::new(c[0], c[1], c[2])
    }

    /// Criteria 23 and 24/6: white text on a red box is white text, and the box is one whole
    /// outline with no letter hole.
    #[test]
    fn white_text_on_a_box_is_white_above_the_whole_box() {
        let wire = trace_text_colour_job_with(
            &white_on_red(),
            W,
            H,
            MAP,
            &params(),
            &[line()],
            &mut |_| {},
        );
        let read = read_trace_wire(&wire).unwrap();
        assert_eq!(read.texts.lines.len(), 1);
        assert_eq!(read.texts.lines[0].fill, Some(Rgb8::new(255, 255, 255)));
        assert_eq!(read.layers.len(), 1);
        assert_eq!(read.layers[0].colour, rgb(RED));
        assert_eq!(read.outlines.len(), 1, "the box whole");
        let b = kurbo::Shape::bounding_box(&read.outlines[0]);
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (2.0, 2.0, 58.0, 28.0));
        // One subpath: no hole where the letters were.
        let moves = read.outlines[0]
            .elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 1);
    }

    /// Criterion 23: a colour only the text has stays in the palette and fills the text; the
    /// palette is the one Recognise text off finds.
    #[test]
    fn red_text_on_white_keeps_its_red() {
        let rgba = red_on_white();
        let wire = trace_text_colour_job_with(&rgba, W, H, MAP, &params(), &[line()], &mut |_| {});
        let read = read_trace_wire(&wire).unwrap();
        assert_eq!(read.texts.lines[0].fill, Some(rgb(RED)));
        let off = read_trace_wire(&trace_colour_job_with(
            &rgba,
            W,
            H,
            MAP,
            &params(),
            &mut |_| {},
        ))
        .unwrap();
        assert_eq!(read.palette, off.palette);
        // The red is the text's: no red path is left.
        assert!(
            read.layers.iter().all(|l| l.colour != rgb(RED)),
            "{:?}",
            read.layers
        );
    }

    /// Lead 2026-11-02: pale grey text on the paper takes its palette grey, never the paper's
    /// white; white text on a box stays white (above).
    #[test]
    fn pale_text_on_paper_takes_a_palette_colour_not_the_paper() {
        const GREY: [u8; 3] = [190, 190, 190];
        let rgba = picture(|x, y| {
            if in_bar(x, y) {
                Some(GREY)
            } else if (54..59).contains(&x) && (4..26).contains(&y) {
                Some(BLUE)
            } else {
                None
            }
        });
        // Remove background keeps the grey as a palette colour.
        let wire = trace_text_colour_job_with(&rgba, W, H, MAP, &params(), &[line()], &mut |_| {});
        let read = read_trace_wire(&wire).unwrap();
        assert_eq!(
            read.texts.lines[0].fill,
            Some(rgb(GREY)),
            "{:?}",
            read.palette
        );
        assert!(!read.texts.lines[0].excluded);
        // Brightness leaves the grey out of the palette: the nearest palette colour, not white.
        let brightness = ColourParams::new(TraceParams {
            specks: Length::from_mm(0.0),
            ..TraceParams::with_threshold(128)
        });
        let wire =
            trace_text_colour_job_with(&rgba, W, H, MAP, &brightness, &[line()], &mut |_| {});
        let read = read_trace_wire(&wire).unwrap();
        assert_eq!(
            read.texts.lines[0].fill,
            Some(rgb(BLUE)),
            "{:?}",
            read.palette
        );
    }

    /// Lead 2026-11-02: a text whose colour the maker excluded is left out, as its shapes are.
    #[test]
    fn a_text_in_an_excluded_colour_is_left_out() {
        let mut excluded = params();
        excluded.excluded = vec![rgb(RED)];
        let wire = trace_text_colour_job_with(
            &red_on_white(),
            W,
            H,
            MAP,
            &excluded,
            &[line()],
            &mut |_| {},
        );
        assert!(read_trace_wire(&wire).unwrap().texts.lines[0].excluded);
        let kept = trace_text_colour_job_with(
            &red_on_white(),
            W,
            H,
            MAP,
            &params(),
            &[line()],
            &mut |_| {},
        );
        assert!(!read_trace_wire(&kept).unwrap().texts.lines[0].excluded);
    }

    /// Merge into applies to a text as to its paths.
    #[test]
    fn a_merged_colour_fills_the_text_with_its_target() {
        let mut merged = params();
        merged.merged = vec![(rgb(RED), rgb(BLUE))];
        let wire =
            trace_text_colour_job_with(&red_on_white(), W, H, MAP, &merged, &[line()], &mut |_| {});
        assert_eq!(
            read_trace_wire(&wire).unwrap().texts.lines[0].fill,
            Some(rgb(BLUE))
        );
    }

    /// Criteria 24/12 and 24/13: no lines, the plain colour job; the same input twice, the same
    /// numbers; the percent only rises and ends at 100.
    #[test]
    fn without_lines_it_is_the_colour_job_and_it_repeats() {
        let rgba = white_on_red();
        assert_eq!(
            trace_text_colour_job_with(&rgba, W, H, MAP, &params(), &[], &mut |_| {}),
            trace_colour_job_with(&rgba, W, H, MAP, &params(), &mut |_| {})
        );
        let mut seen = Vec::new();
        let first = trace_text_colour_job_with(&rgba, W, H, MAP, &params(), &[line()], &mut |p| {
            seen.push(p);
        });
        let second =
            trace_text_colour_job_with(&rgba, W, H, MAP, &params(), &[line()], &mut |_| {});
        assert_eq!(first, second);
        assert!(seen.windows(2).all(|w| w[0] < w[1]), "{seen:?}");
        assert_eq!(seen.last(), Some(&100));
    }

    /// UX notes §2 in Multicolour: the left-out pixels grey, the box around them red.
    #[test]
    fn the_colour_mask_shows_left_out_pixels_grey() {
        let mask =
            trace_text_colour_mask_job(&white_on_red(), W, H, MAP, &params(), &[line()]).unwrap();
        let factor = (mask.width / W) as usize;
        let at = |x: usize, y: usize| {
            let i = ((y * factor) * mask.width as usize + x * factor) * 4;
            [
                mask.rgba[i],
                mask.rgba[i + 1],
                mask.rgba[i + 2],
                mask.rgba[i + 3],
            ]
        };
        assert_eq!(
            at(20, 14),
            [LEFT_OUT_LEVEL, LEFT_OUT_LEVEL, LEFT_OUT_LEVEL, 255]
        );
        assert_eq!(at(4, 4), [RED[0], RED[1], RED[2], 255]);
    }

    #[test]
    fn the_nearest_colour_takes_the_first_on_a_tie() {
        let (a, b) = (Rgb8::new(0, 0, 0), Rgb8::new(20, 0, 0));
        assert_eq!(nearest(Rgb8::new(10, 0, 0), &[a, b]), Some(a));
        assert_eq!(nearest(Rgb8::new(11, 0, 0), &[a, b]), Some(b));
        assert_eq!(nearest(a, &[]), None);
    }
}
