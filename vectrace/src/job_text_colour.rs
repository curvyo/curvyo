//! The multicolour trace job with OCR lines (`specs/0091-trace-text-recognition` slice 2,
//! criteria 23 and 24; `adrs.md` decisions 3 and 6).
//!
//! The text pass runs as in black and white. The palette is the one of the picture as it is (the
//! palette Recognise text off finds), so a colour only the text has stays a palette colour; the
//! filled picture is then traced with exactly that palette, and each accepted line's fill is the
//! palette or background colour nearest its text pixels, through the maker's Merge into.

use std::ops::ControlFlow;

use crate::{
    ColourParams, ColourTrace, PaletteEntry, PixelMap, Progress, Raster, Rgb8, Selection,
    TraceError, colour_palette, suggest_background, trace_colours,
};

use crate::job_text::{keep_lines_in, write_texts};
use crate::mask::{Mask, clip_colours, write_unclipped};
use crate::ocr::accept::OcrLine;
use crate::text_pass::{TextPass, text_pass};
use crate::wire::{
    TraceStatus, colour_head, colour_wire, job_percent, status_only, write_colour_tail,
};

/// The share of a job's percent the palette of the unfilled picture takes (Proposal: it is the
/// labelling part of a trace).
const PALETTE_PERCENT: u32 = 25;

/// One multicolour trace with the OCR `lines` (in this picture's pixels): the text pass, the
/// palette of the picture as it is, the trace of the filled picture with that palette, then the
/// numbers of [`colour_wire`] with the text section (each line's fill) between the outlines and
/// the palette. With a `mask` (`specs/0094` criterion 11) only the lines whose box centre the
/// mask keeps stay as text, and each layer is clipped. Without lines the numbers are exactly
/// those of [`colour_wire`] (criteria 12 and 24).
pub(crate) fn text_colour_wire(
    raster: Raster<'_>,
    map: PixelMap,
    params: &ColourParams,
    lines: &[OcrLine],
    mask: Option<&dyn Mask>,
    on_percent: &mut dyn FnMut(u8),
) -> Vec<f64> {
    if lines.is_empty() {
        return colour_wire(raster, map, params, mask, on_percent);
    }
    let mut pass = text_pass(raster, map, &params.trace, lines);
    keep_lines_in(&mut pass, mask);
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
            rgba: &pass.rgba,
            ..raster
        };
        trace_with_palette(filled, map, params, palette.as_deref(), &mut |p| {
            report(PALETTE_PERCENT + u32::from(job_percent(p)) * (100 - PALETTE_PERCENT) / 100);
        })
    });
    match traced {
        Ok(mut result) => {
            let unclipped = mask.map_or(0, |mask| clip_colours(mask, &mut result));
            report(100);
            let fills = line_fills(&pass, &result.palette, raster, params);
            let mut wire = colour_head(&result);
            write_unclipped(&mut wire, unclipped);
            write_texts(&mut wire, &pass, &fills);
            write_colour_tail(&mut wire, &result);
            wire
        }
        Err(TraceError::TooDetailed) => status_only(TraceStatus::TooDetailed),
        Err(TraceError::Invalid(_) | TraceError::Cancelled) => status_only(TraceStatus::Invalid),
    }
}

/// The palette of `raster` as it is, for the trace of the filled picture: `None` when the text
/// pass changed no pixel or Pick colours sets the palette (the filled picture's palette is then
/// the same).
fn palette_of_picture(
    raster: Raster<'_>,
    map: PixelMap,
    params: &ColourParams,
    pass: &TextPass,
    progress: &mut dyn FnMut(Progress),
) -> Result<Option<Vec<PaletteEntry>>, TraceError> {
    if pass.regions.is_empty() || matches!(params.trace.selection, Selection::PickColours { .. }) {
        return Ok(None);
    }
    let palette = colour_palette(raster, map, params, &mut |p| {
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
    map: PixelMap,
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
    use crate::Mm;
    use crate::TraceParams;
    use crate::job::{Job, Mode, run};
    use crate::wire::read_wire;

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
            specks: Mm::from_mm(0.0),
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
        let wire = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &white_on_red(),
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&params()),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        let read = read_wire(&wire).unwrap();
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
        let wire = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &rgba,
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&params()),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        let read = read_wire(&wire).unwrap();
        assert_eq!(read.texts.lines[0].fill, Some(rgb(RED)));
        let off = read_wire(&run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &rgba,
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&params()),
                lines: &[],
                mask: None,
                shapes: false,
            },
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
        let wire = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &rgba,
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&params()),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        let read = read_wire(&wire).unwrap();
        assert_eq!(
            read.texts.lines[0].fill,
            Some(rgb(GREY)),
            "{:?}",
            read.palette
        );
        assert!(!read.texts.lines[0].excluded);
        // Brightness leaves the grey out of the palette: the nearest palette colour, not white.
        let brightness = ColourParams::new(TraceParams {
            specks: Mm::from_mm(0.0),
            ..TraceParams::with_threshold(128)
        });
        let wire = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &rgba,
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&brightness),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        let read = read_wire(&wire).unwrap();
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
        let wire = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &red_on_white(),
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&excluded),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        assert!(read_wire(&wire).unwrap().texts.lines[0].excluded);
        let kept = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &red_on_white(),
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&params()),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        assert!(!read_wire(&kept).unwrap().texts.lines[0].excluded);
    }

    /// Criteria 24/12 and 24/13: no lines, the plain colour job; the same input twice, the same
    /// numbers; the percent only rises and ends at 100.
    #[test]
    fn without_lines_it_is_the_colour_job_and_it_repeats() {
        let rgba = white_on_red();
        assert_eq!(
            run(
                &Job {
                    raster: Raster {
                        width: W,
                        height: H,
                        rgba: &rgba
                    },
                    map: PixelMap::new(MAP),
                    mode: Mode::Colours(&params()),
                    lines: &[],
                    mask: None,
                    shapes: false
                },
                &mut |_| {}
            ),
            run(
                &Job {
                    raster: Raster {
                        width: W,
                        height: H,
                        rgba: &rgba
                    },
                    map: PixelMap::new(MAP),
                    mode: Mode::Colours(&params()),
                    lines: &[],
                    mask: None,
                    shapes: false
                },
                &mut |_| {}
            )
        );
        let mut seen = Vec::new();
        let first = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &rgba,
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&params()),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |p| {
                seen.push(p);
            },
        );
        let second = run(
            &Job {
                raster: Raster {
                    width: W,
                    height: H,
                    rgba: &rgba,
                },
                map: PixelMap::new(MAP),
                mode: Mode::Colours(&params()),
                lines: &[line()],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        assert_eq!(first, second);
        assert!(seen.windows(2).all(|w| w[0] < w[1]), "{seen:?}");
        assert_eq!(seen.last(), Some(&100));
    }

    #[test]
    fn the_nearest_colour_takes_the_first_on_a_tie() {
        let (a, b) = (Rgb8::new(0, 0, 0), Rgb8::new(20, 0, 0));
        assert_eq!(nearest(Rgb8::new(10, 0, 0), &[a, b]), Some(a));
        assert_eq!(nearest(Rgb8::new(11, 0, 0), &[a, b]), Some(b));
        assert_eq!(nearest(a, &[]), None);
    }
}
