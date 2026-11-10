//! The one entry point of the Trace worker (ADR 0023 §5, ADR 0048 §5): a [`Job`] describes the
//! picture, the settings, the OCR lines, the mask and whether to recognise shapes; [`run`] gives
//! the result as the flat numbers the session reads back, [`text_regions`] the text section only.

mod text;
mod text_colour;

use crate::{ColourParams, PixelMap, Raster, TraceParams};

use self::text::{text_mono_wire, text_regions_wire};
use self::text_colour::text_colour_wire;
use crate::mask::Mask;
use crate::ocr::OcrLine;
use crate::shapes::add_shapes;

/// The settings of a job: one black result or one per colour.
#[derive(Debug, Clone, Copy)]
pub enum Mode<'a> {
    /// One result in black (`specs/0025-raster-trace`).
    Mono(&'a TraceParams),
    /// One layer per palette colour (`specs/0089` slice M).
    Colours(&'a ColourParams),
}

/// One trace for the worker.
#[derive(Clone, Copy)]
pub struct Job<'a> {
    /// The picture, straight alpha.
    pub raster: Raster<'a>,
    /// Maps the picture's pixels to millimetres.
    pub map: PixelMap,
    /// The settings.
    pub mode: Mode<'a>,
    /// The OCR lines in the picture's pixels (`specs/0091`); none for no text pass.
    pub lines: &'a [OcrLine],
    /// The area the result keeps (`specs/0094`); `None` keeps all of it.
    pub mask: Option<&'a dyn Mask>,
    /// Recognise shapes and add their section to the result (`specs/0088` Part B).
    pub shapes: bool,
}

/// The result of `job` as numbers, never panicking on bad input (it says `Invalid`).
/// `on_percent` is called each time the job reaches a higher whole percent (criterion 10); the
/// values only rise. With lines the text pass runs first; with no lines the numbers are exactly
/// those of the plain trace (`specs/0091` criteria 12 and 18).
#[must_use]
pub fn run(job: &Job<'_>, on_percent: &mut dyn FnMut(u8)) -> Vec<f64> {
    let mut wire = match job.mode {
        Mode::Mono(params) => {
            text_mono_wire(job.raster, job.map, params, job.lines, job.mask, on_percent)
        }
        Mode::Colours(params) => {
            text_colour_wire(job.raster, job.map, params, job.lines, job.mask, on_percent)
        }
    };
    if job.shapes {
        add_shapes(&mut wire, job.map.to_array());
    }
    wire
}

/// The text pass of `job` alone (`specs/0091` criterion 25: the lines are decided before the
/// main trace): the numbers of a result without outlines, then the text section of the accepted
/// lines the mask keeps, as [`run`] would write it.
#[must_use]
pub fn text_regions(job: &Job<'_>) -> Vec<f64> {
    let params = match job.mode {
        Mode::Mono(params) => params,
        Mode::Colours(params) => &params.trace,
    };
    text_regions_wire(job.raster, job.map, params, job.lines, job.mask)
}
