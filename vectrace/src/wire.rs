//! One trace job of the Trace tool's worker and its result as a flat list of numbers
//! (`specs/0025-raster-trace` criteria 11, 15, 16 and 17; ADR 0023 §5): the worker runs
//! [`crate::job::run`], posts the numbers, and the session reads them back with [`read`].

use std::ops::ControlFlow;

use crate::{
    ColourParams as TraceColourParams, ColourTrace, Phase, PixelMap, Progress, Raster,
    Rgb8 as TraceRgb, TraceError, TraceParams, trace, trace_colours,
};
use kurbo::{BezPath, PathEl, Point};

use crate::mask::{Mask, clip_colours, clip_trace, write_unclipped};

mod read;
mod text;
pub use read::read;
pub(crate) use read::{count, read_outlines, read_trace_body};
pub(crate) use text::{TEXT_MARK, read_texts, write_texts};
pub use text::{Text, Texts};

/// How a trace job ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Outlines were made (possibly none).
    Traced,
    /// A region has 65 000 holes or more: "Too much detail: raise Ignore below." (criterion 16).
    TooDetailed,
    /// The picture or the map cannot be traced.
    Invalid,
}

impl Status {
    pub(crate) fn code(self) -> f64 {
        match self {
            Self::Traced => 0.0,
            Self::TooDetailed => 1.0,
            Self::Invalid => 2.0,
        }
    }

    /// The host's code: `"traced"`, `"too_detailed"` or `"invalid"`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Traced => "traced",
            Self::TooDetailed => "too_detailed",
            Self::Invalid => "invalid",
        }
    }
}

/// A trace result as the session keeps it.
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    /// How the job ended.
    pub status: Status,
    /// Outlines.
    pub paths: usize,
    /// Anchor points of all outlines.
    pub nodes: usize,
    /// Regions and holes removed as specks.
    pub specks_removed: usize,
    /// Pixels below the Threshold before specks.
    pub foreground_pixels: u64,
    /// One outline covers the whole picture.
    pub covers_whole_image: bool,
    /// The outlines in document millimetres (a multicolour result: layer by layer).
    pub outlines: Vec<BezPath>,
    /// The palette of a multicolour result (`specs/0089` slice M); empty for one colour.
    pub palette: Vec<Palette>,
    /// The layers of a multicolour result in order, largest area first; empty for one colour.
    pub layers: Vec<Layer>,
    /// Colour regions below Ignore below joined to a neighbour (multicolour).
    pub joined: usize,
    /// The text pass's lines (`specs/0091`); empty without OCR lines.
    pub texts: Texts,
    /// The recognised shapes (`specs/0088` Part B); empty with Recognise shapes off. Each
    /// recognised outline in `outlines` is already its primitive's outline.
    pub shapes: Vec<crate::shapes::ShapeRegion>,
    /// Outlines kept whole across the mask edge after a kernel refusal (`specs/0094`).
    pub unclipped: usize,
}

/// One palette colour of a multicolour result, as the strip shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Its colour.
    pub colour: TraceRgb,
    /// The maker picked it.
    pub fixed: bool,
    /// Traced pixels it took before exclusions and joins.
    pub pixels: u64,
}

/// One layer of a multicolour result: its colour and how many of the outlines are its (they
/// follow each other in layer order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layer {
    /// Its colour.
    pub colour: TraceRgb,
    /// Outlines of the layer.
    pub outlines: usize,
}

/// The numbers before the outlines: status, paths, nodes, specks, foreground pixels, whole
/// picture, outline count.
pub const HEADER: usize = 7;

const MOVE: f64 = 0.0;
const LINE: f64 = 1.0;
const CURVE: f64 = 3.0;
const CLOSE: f64 = 4.0;

/// The share of the passes over the mask in a job's percentage; the outlines take the rest.
const LABELLING_PERCENT: u32 = 30;

/// Where a job is, in whole percent (the progress pill, `specs/0025-raster-trace` criterion 10):
/// the passes over the mask fill the first 30 %, the outlines the rest. A pass restarts its rows,
/// so the caller keeps the highest value.
pub(crate) fn job_percent(progress: Progress) -> u8 {
    let share = |part: u32, done: u32, total: u32| {
        if total == 0 {
            part
        } else {
            let done = u64::from(done.min(total));
            u32::try_from(u64::from(part) * done / u64::from(total)).unwrap_or(part)
        }
    };
    let percent = match progress.phase {
        Phase::Labelling => share(LABELLING_PERCENT, progress.done, progress.total),
        Phase::Tracing => {
            LABELLING_PERCENT + share(100 - LABELLING_PERCENT, progress.done, progress.total)
        }
    };
    u8::try_from(percent.min(100)).unwrap_or(100)
}

/// One trace of `raster` (straight alpha) mapped by `map` with `params`, the outlines clipped to
/// `mask` before they are written (`specs/0094` slice 1); the result as numbers. `on_percent` is
/// called each time the job reaches a higher whole percent (criterion 10); the values only rise.
/// Never panics on bad input: it says `Invalid`.
pub(crate) fn mono_wire(
    raster: Raster<'_>,
    map: PixelMap,
    params: &TraceParams,
    mask: Option<&dyn Mask>,
    on_percent: &mut dyn FnMut(u8),
) -> Vec<f64> {
    let mut reported: Option<u8> = None;
    let mut go = |progress: Progress| {
        let percent = job_percent(progress);
        if reported.is_none_or(|last| percent > last) {
            reported = Some(percent);
            on_percent(percent);
        }
        ControlFlow::Continue(())
    };
    match trace(raster, map, params, &mut go) {
        Ok(mut result) => {
            let unclipped = mask.map_or(0, |mask| clip_trace(mask, &mut result));
            #[allow(clippy::cast_precision_loss)] // counts far below 2^52
            let mut wire = vec![
                Status::Traced.code(),
                result.counts.paths as f64,
                result.counts.nodes as f64,
                result.counts.specks_removed as f64,
                result.foreground_pixels as f64,
                f64::from(u8::from(result.covers_whole_image)),
                result.outlines.len() as f64,
            ];
            for outline in &result.outlines {
                write_outline(&mut wire, outline);
            }
            write_unclipped(&mut wire, unclipped);
            wire
        }
        Err(TraceError::TooDetailed) => status_only(Status::TooDetailed),
        Err(TraceError::Invalid(_) | TraceError::Cancelled) => status_only(Status::Invalid),
    }
}

/// One multicolour trace (`specs/0089` slice M): as [`mono_wire`], the outlines layer by layer,
/// each layer clipped to `mask` before it is written (`specs/0094` slice 1), then the palette and
/// the layers (each colour and its outline count) and the joined regions after them.
pub(crate) fn colour_wire(
    raster: Raster<'_>,
    map: PixelMap,
    params: &TraceColourParams,
    mask: Option<&dyn Mask>,
    on_percent: &mut dyn FnMut(u8),
) -> Vec<f64> {
    let mut reported: Option<u8> = None;
    let mut go = |progress: Progress| {
        let percent = job_percent(progress);
        if reported.is_none_or(|last| percent > last) {
            reported = Some(percent);
            on_percent(percent);
        }
        ControlFlow::Continue(())
    };
    match trace_colours(raster, map, params, &mut go) {
        Ok(mut result) => {
            let unclipped = mask.map_or(0, |mask| clip_colours(mask, &mut result));
            let mut wire = colour_head(&result);
            write_unclipped(&mut wire, unclipped);
            write_colour_tail(&mut wire, &result);
            wire
        }
        Err(TraceError::TooDetailed) => status_only(Status::TooDetailed),
        Err(TraceError::Invalid(_) | TraceError::Cancelled) => status_only(Status::Invalid),
    }
}

/// The numbers of a multicolour result before its palette: the header and the outlines layer by
/// layer.
pub(crate) fn colour_head(result: &ColourTrace) -> Vec<f64> {
    let outlines: usize = result.layers.iter().map(|l| l.outlines.len()).sum();
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    let mut wire = vec![
        Status::Traced.code(),
        result.counts.paths as f64,
        result.counts.nodes as f64,
        result.counts.specks_removed as f64,
        result.foreground_pixels as f64,
        0.0,
        outlines as f64,
    ];
    for outline in result.layers.iter().flat_map(|l| &l.outlines) {
        write_outline(&mut wire, outline);
    }
    wire
}

/// The palette, the layers (each colour and its outline count) and the joined regions of a
/// multicolour result after its outlines (and its text section).
pub(crate) fn write_colour_tail(wire: &mut Vec<f64>, result: &ColourTrace) {
    let rgb = |c: TraceRgb| [f64::from(c.r), f64::from(c.g), f64::from(c.b)];
    #[allow(clippy::cast_precision_loss)] // counts far below 2^52
    {
        wire.push(result.palette.len() as f64);
        for entry in &result.palette {
            wire.extend(rgb(entry.colour));
            wire.extend([f64::from(u8::from(entry.fixed)), entry.pixels as f64]);
        }
        wire.push(result.layers.len() as f64);
        for layer in &result.layers {
            wire.extend(rgb(layer.sample));
            wire.push(layer.outlines.len() as f64);
        }
        wire.push(result.diagnostics.joined_regions as f64);
    }
}

pub(crate) fn status_only(status: Status) -> Vec<f64> {
    let mut wire = vec![0.0; HEADER];
    wire[0] = status.code();
    wire
}

pub(crate) fn write_outline(wire: &mut Vec<f64>, outline: &BezPath) {
    let elements = outline.elements();
    #[allow(clippy::cast_precision_loss)]
    wire.push(elements.len() as f64);
    let mut last = Point::ZERO;
    for element in elements {
        match *element {
            PathEl::MoveTo(p) => {
                wire.extend([MOVE, p.x, p.y]);
                last = p;
            }
            PathEl::LineTo(p) => {
                wire.extend([LINE, p.x, p.y]);
                last = p;
            }
            // The tracer makes no quadratic segments; one would be raised to a cubic.
            PathEl::QuadTo(c, p) => {
                let (c1, c2) = (last.lerp(c, 2.0 / 3.0), p.lerp(c, 2.0 / 3.0));
                wire.extend([CURVE, c1.x, c1.y, c2.x, c2.y, p.x, p.y]);
                last = p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                wire.extend([CURVE, c1.x, c1.y, c2.x, c2.y, p.x, p.y]);
                last = p;
            }
            PathEl::ClosePath => wire.push(CLOSE),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mm;
    use crate::job::{Job, Mode, run};
    use crate::mask::test_double::BoxMask;

    fn square_picture() -> Vec<u8> {
        (0..16 * 16)
            .flat_map(|i| {
                let (x, y) = (i % 16, i / 16);
                let v = if (4..12).contains(&x) && (4..12).contains(&y) {
                    0
                } else {
                    255
                };
                [v, v, v, 255]
            })
            .collect()
    }

    /// One trace with the three bar values and every other setting at its first-use value.
    fn trace_job(
        rgba: &[u8],
        width: u32,
        height: u32,
        pixel_map: [f64; 6],
        threshold: u8,
        smooth: u8,
        specks_mm: f64,
    ) -> Vec<f64> {
        let params = TraceParams {
            threshold,
            smooth,
            specks: Mm::from_mm(specks_mm),
            ..TraceParams::with_threshold(threshold)
        };
        run(
            &Job {
                raster: Raster {
                    width,
                    height,
                    rgba,
                },
                map: PixelMap::new(pixel_map),
                mode: Mode::Mono(&params),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        )
    }

    const SQUARE_MAP: [f64; 6] = [0.5, 0.0, 0.0, 0.5, 1.0, 2.0];

    fn job_with(mask: Option<&dyn Mask>) -> Output {
        let params = TraceParams::with_threshold(128);
        let wire = run(
            &Job {
                raster: Raster {
                    width: 16,
                    height: 16,
                    rgba: &square_picture(),
                },
                map: PixelMap::new(SQUARE_MAP),
                mode: Mode::Mono(&params),
                lines: &[],
                mask,
                shapes: false,
            },
            &mut |_| {},
        );
        read(&wire).unwrap()
    }

    /// The square outline spans x 3..7, y 4..8 mm.
    #[test]
    fn a_mask_that_keeps_everything_leaves_the_job_as_it_is() {
        let all = BoxMask {
            min: Point::new(0.0, 0.0),
            max: Point::new(100.0, 100.0),
        };
        assert_eq!(job_with(Some(&all)), job_with(None));
    }

    #[test]
    fn a_mask_that_drops_the_outline_leaves_none_and_recounts() {
        let away = BoxMask {
            min: Point::new(50.0, 50.0),
            max: Point::new(60.0, 60.0),
        };
        let read = job_with(Some(&away));
        assert_eq!(read.status, Status::Traced);
        assert_eq!((read.paths, read.nodes, read.outlines.len()), (0, 0, 0));
        assert!(!read.covers_whole_image);
        assert_eq!(read.unclipped, 0);
    }

    /// An outline the mask could not cut stays whole and its count travels on the wire.
    #[test]
    fn the_masks_whole_kept_outlines_are_counted_on_the_wire() {
        let edge = BoxMask {
            min: Point::new(0.0, 0.0),
            max: Point::new(5.0, 100.0),
        };
        let read = job_with(Some(&edge));
        assert_eq!((read.paths, read.nodes), (1, 4));
        assert_eq!(read.unclipped, 1);
    }

    /// A multicolour layer left empty by the mask is dropped and the counts follow.
    #[test]
    fn a_mask_drops_the_layers_it_empties() {
        let picture: Vec<u8> = (0..40 * 20)
            .flat_map(|i| {
                let (x, y) = (i % 40, i / 40);
                if !(4..36).contains(&x) || !(4..16).contains(&y) {
                    [255, 255, 255, 255]
                } else if x < 20 {
                    [208, 34, 42, 255]
                } else {
                    [35, 78, 184, 255]
                }
            })
            .collect();
        let params = TraceColourParams::new(TraceParams {
            selection: crate::Selection::RemoveBackground {
                background: Some(TraceRgb::new(255, 255, 255)),
                tolerance: 40,
            },
            specks: Mm::from_mm(0.0),
            ..TraceParams::with_threshold(128)
        });
        let map = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let left = BoxMask {
            min: Point::new(0.0, 0.0),
            max: Point::new(19.0, 20.0),
        };
        let wire = run(
            &Job {
                raster: Raster {
                    width: 40,
                    height: 20,
                    rgba: &picture,
                },
                map: PixelMap::new(map),
                mode: Mode::Colours(&params),
                lines: &[],
                mask: Some(&left),
                shapes: false,
            },
            &mut |_| {},
        );
        let read = read(&wire).unwrap();
        assert_eq!(read.layers.len(), 1);
        assert_eq!(read.layers[0].colour, TraceRgb::new(208, 34, 42));
        assert_eq!(read.outlines.len(), read.paths);
        assert_eq!(read.layers[0].outlines, read.outlines.len());
    }

    #[test]
    fn a_job_reports_a_rising_percentage_that_ends_at_100() {
        let mut seen = Vec::new();
        let params = TraceParams::with_threshold(128);
        let wire = run(
            &Job {
                raster: Raster {
                    width: 16,
                    height: 16,
                    rgba: &square_picture(),
                },
                map: PixelMap::new([0.5, 0.0, 0.0, 0.5, 1.0, 2.0]),
                mode: Mode::Mono(&params),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |percent| seen.push(percent),
        );
        assert_eq!(read(&wire).unwrap().status, Status::Traced);
        assert_ne!(seen.len(), 0);
        assert!(seen.windows(2).all(|w| w[0] < w[1]), "{seen:?}");
        assert_eq!(seen.last(), Some(&100));
    }

    #[test]
    fn the_percentage_gives_the_outlines_the_larger_share() {
        let at = |phase, done, total| job_percent(Progress { phase, done, total });
        assert_eq!(at(Phase::Labelling, 0, 10), 0);
        assert_eq!(at(Phase::Labelling, 10, 10), 30);
        assert_eq!(at(Phase::Tracing, 0, 4), 30);
        assert_eq!(at(Phase::Tracing, 2, 4), 65);
        assert_eq!(at(Phase::Tracing, 4, 4), 100);
        assert_eq!(at(Phase::Tracing, 0, 0), 100);
    }

    #[test]
    fn a_job_round_trips_through_the_wire() {
        let wire = trace_job(
            &square_picture(),
            16,
            16,
            [0.5, 0.0, 0.0, 0.5, 1.0, 2.0],
            128,
            80,
            0.0,
        );
        let read = read(&wire).unwrap();
        assert_eq!(read.status, Status::Traced);
        assert_eq!((read.paths, read.nodes, read.specks_removed), (1, 4, 0));
        assert_eq!(read.foreground_pixels, 64);
        assert!(!read.covers_whole_image);
        let b = kurbo::Shape::bounding_box(&read.outlines[0]);
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (3.0, 4.0, 7.0, 8.0));
    }

    /// `specs/0089` slice M: the colour job's palette, layers and joined regions cross the
    /// wire after the outlines; a tail that does not hold the outlines is refused.
    #[test]
    fn a_colour_job_round_trips_its_palette_and_layers() {
        let picture: Vec<u8> = (0..40 * 20)
            .flat_map(|i| {
                let (x, y) = (i % 40, i / 40);
                if !(4..36).contains(&x) || !(4..16).contains(&y) {
                    [255, 255, 255, 255]
                } else if x < 20 {
                    [208, 34, 42, 255]
                } else {
                    [35, 78, 184, 255]
                }
            })
            .collect();
        let params = TraceColourParams::new(TraceParams {
            selection: crate::Selection::RemoveBackground {
                background: Some(TraceRgb::new(255, 255, 255)),
                tolerance: 40,
            },
            specks: Mm::from_mm(0.0),
            ..TraceParams::with_threshold(128)
        });
        let map = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let wire = run(
            &Job {
                raster: Raster {
                    width: 40,
                    height: 20,
                    rgba: &picture,
                },
                map: PixelMap::new(map),
                mode: Mode::Colours(&params),
                lines: &[],
                mask: None,
                shapes: false,
            },
            &mut |_| {},
        );
        let read = read(&wire).unwrap();
        assert_eq!(read.status, Status::Traced);
        assert_eq!(read.palette.len(), 2);
        assert_eq!(read.layers.len(), 2);
        assert_eq!(
            read.layers.iter().map(|l| l.outlines).sum::<usize>(),
            read.outlines.len()
        );
        assert!(
            read.layers
                .iter()
                .any(|l| l.colour == TraceRgb::new(208, 34, 42))
        );
        let mut bad = wire.clone();
        // One more outline claimed by the last layer than there are.
        let last = bad.len() - 2;
        bad[last] += 1.0;
        assert_eq!(super::read(&bad), None);
        // A single-colour wire has no tail.
        let single = trace_job(&picture, 40, 20, map, 200, 80, 0.0);
        assert_eq!(super::read(&single).unwrap().layers, []);
    }

    /// The worker's analysis (`specs/0089` slice F): a report for a valid picture, none for
    /// bytes that do not match the size.
    #[test]
    fn the_analysis_reads_valid_pictures_only() {
        use crate::{analyse, auto_threshold};
        let picture = square_picture();
        let raster = |rgba| Raster {
            width: 16,
            height: 16,
            rgba,
        };
        let report = analyse(raster(&picture)).unwrap();
        assert_eq!(report.proposal.threshold, auto_threshold(raster(&picture)));
        assert!(analyse(raster(&[0; 12])).is_err());
    }

    #[test]
    fn bad_input_and_bad_wires_are_refused() {
        let wire = trace_job(
            &[0; 8],
            16,
            16,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            128,
            80,
            0.0,
        );
        assert_eq!(read(&wire).unwrap().status, Status::Invalid);
        assert_eq!(read(&[0.0; 3]), None);
        let mut good = trace_job(
            &square_picture(),
            16,
            16,
            [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            128,
            80,
            0.0,
        );
        good.push(1.0);
        assert_eq!(read(&good), None);
        good.pop();
        good[HEADER + 1] = 9.0;
        assert_eq!(read(&good), None);
        good[HEADER + 1] = MOVE;
        good[HEADER + 2] = f64::NAN;
        assert_eq!(read(&good), None);
    }
}
