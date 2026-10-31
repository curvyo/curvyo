//! The multicolour trace (`specs/0089` slice M, adrs.md §10): an RGBA8 raster and trace
//! settings in, one layer of closed outlines per palette colour out, neighbouring colours
//! sharing the same fitted boundary.

use std::ops::ControlFlow;

use curvyo_document_core::Affine;
use kurbo::BezPath;

use crate::colour_map::{ColourMap, colour_map};
use crate::edges::{Loop, edges};
use crate::fit::Seg;
use crate::outline::{node_count, to_path};
use crate::params::{Rgb8, TraceParams, geometry};
use crate::progress::{Poller, Progress};
use crate::raster::Raster;
use crate::shared_fit::{loop_segs, shared_fit};
use crate::trace::{Counts, Diagnostics, TraceError, check, to_kurbo};

/// The values of a multicolour trace: the single-colour values (the selection method decides
/// what is content, the geometry how it is fitted) and the palette's.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourParams {
    /// The selection, geometry and Ignore below of the trace.
    pub trace: TraceParams,
    /// How many colours (2 to 16, the fixed ones included); `None` finds the count. Pick colours
    /// ignores it: its palette is exactly the picked colours (Frage 90 A).
    pub colours: Option<u8>,
    /// Colours the maker picked into the palette; kept exactly (Brightness and Remove
    /// background; Pick colours takes its swatches).
    pub fixed: Vec<Rgb8>,
    /// Palette colours that become background (Exclude), matched within the merge distance.
    pub excluded: Vec<Rgb8>,
    /// Palette colours traced as another one (Merge into: from, into).
    pub merged: Vec<(Rgb8, Rgb8)>,
}

impl ColourParams {
    /// A multicolour trace of `trace` with the automatic palette and no edits.
    #[must_use]
    pub fn new(trace: TraceParams) -> Self {
        Self {
            trace,
            colours: None,
            fixed: Vec::new(),
            excluded: Vec::new(),
            merged: Vec::new(),
        }
    }
}

/// One colour of the palette as the strip shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteEntry {
    /// Its colour.
    pub colour: Rgb8,
    /// The maker picked it (or it is a picked colour of Pick colours).
    pub fixed: bool,
    /// Traced pixels it took before exclusions, merges and joins.
    pub pixels: u64,
}

/// The outlines of one colour: one compound path in the document.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourLayer {
    /// Its colour (a merged colour's layer is its target's).
    pub sample: Rgb8,
    /// Its colour is a fixed one.
    pub fixed: bool,
    /// Traced pixels it covers.
    pub pixels: u64,
    /// Closed outlines in document millimetres, holes wound against their outer outline.
    pub outlines: Vec<BezPath>,
}

/// What a multicolour trace made.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourTrace {
    /// The palette, fixed colours first, then by the pixels they took.
    pub palette: Vec<PaletteEntry>,
    /// One layer per traced colour, largest area first, ties by colour value.
    pub layers: Vec<ColourLayer>,
    /// The counts of all layers.
    pub counts: Counts,
    /// Content pixels of the picture (after Denoise, before Upscale and specks).
    pub foreground_pixels: u64,
    /// What the fit had to do and how many regions were joined.
    pub diagnostics: Diagnostics,
}

/// The label map of a multicolour trace as a picture for the Mask view (`specs/0089` UX notes
/// §4, the segmented palette): every content pixel in its palette colour, the rest transparent.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourMask {
    /// Width in mask pixels (twice the picture's after Upscale).
    pub width: u32,
    /// Height in mask pixels.
    pub height: u32,
    /// Straight-alpha RGBA8, row by row.
    pub rgba: Vec<u8>,
    /// Mask pixel corners to document millimetres.
    pub pixel_map: Affine,
}

/// Traces `raster` in colours: the content of the selection method (as [`crate::trace`]) is
/// split into the palette's colours, and every colour becomes one layer of closed outlines;
/// two neighbouring colours share the same fitted boundary and a triple junction is one node.
///
/// # Errors
/// As [`crate::trace`]: [`TraceError::Cancelled`] when `poll` breaks; [`TraceError::Invalid`]
/// for bytes that do not match the size or a singular map.
pub fn trace_colours(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &ColourParams,
    poll: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<ColourTrace, TraceError> {
    check(raster, pixel_map)?;
    let mut poller = Poller::new(poll);
    let cm = colour_map(raster, pixel_map, params, &mut poller)?;
    let (edges, loops) = edges(&cm.labels, cm.width, cm.height);
    let t = &params.trace;
    let geometry = geometry(t.smooth_clamped(), t.tolerance_px(), t.corners);
    let (edge_segs, fallbacks) = shared_fit(&edges, &loops, &geometry, cm.unit, &mut poller)?;
    let layers = layers(&cm, &loops, &edge_segs);
    let counts = Counts {
        paths: layers.iter().map(|l| l.outlines.len()).sum(),
        nodes: layers
            .iter()
            .flat_map(|l| &l.outlines)
            .map(node_count)
            .sum(),
        specks_removed: usize::try_from(cm.specks_removed).unwrap_or(usize::MAX),
    };
    Ok(ColourTrace {
        palette: palette_of(&cm),
        layers,
        counts,
        foreground_pixels: cm.foreground_pixels,
        diagnostics: Diagnostics {
            fallbacks,
            joined_regions: cm.joined,
        },
    })
}

/// One layer per palette colour of `cm` that has a loop, its outlines from the fitted
/// `edge_segs` in millimetres, largest area first, ties by colour value.
fn layers(cm: &ColourMap, loops: &[Loop], edge_segs: &[Vec<Seg>]) -> Vec<ColourLayer> {
    let map = to_kurbo(cm.map);
    let mut pixels = vec![0_u64; cm.palette.len() + 1];
    for &l in &cm.labels {
        pixels[l as usize] += 1;
    }
    let mut layers: Vec<ColourLayer> = Vec::new();
    for (index, entry) in cm.palette.iter().enumerate() {
        let label = u32::try_from(index + 1).unwrap_or(u32::MAX);
        let outlines: Vec<BezPath> = loops
            .iter()
            .filter(|l| l.colour == label)
            .map(|l| {
                let mut path = to_path(&loop_segs(edge_segs, l));
                path.apply_affine(map);
                path
            })
            .collect();
        if outlines.is_empty() {
            continue;
        }
        layers.push(ColourLayer {
            sample: entry.colour,
            fixed: entry.fixed,
            pixels: pixels[index + 1],
            outlines,
        });
    }
    layers.sort_by(|a, b| {
        b.pixels
            .cmp(&a.pixels)
            .then((a.sample.r, a.sample.g, a.sample.b).cmp(&(b.sample.r, b.sample.g, b.sample.b)))
    });
    layers
}

/// The palette entries of `cm` with the pixels each took.
fn palette_of(cm: &ColourMap) -> Vec<PaletteEntry> {
    cm.palette
        .iter()
        .zip(&cm.assigned)
        .map(|(e, &pixels)| PaletteEntry {
            colour: e.colour,
            fixed: e.fixed,
            pixels,
        })
        .collect()
}

/// The segmented palette [`trace_colours`] would walk for the same arguments, made by the same
/// code, with the palette it found.
///
/// # Errors
/// As [`trace_colours`].
pub fn colour_mask(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &ColourParams,
    poll: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<(ColourMask, Vec<PaletteEntry>), TraceError> {
    check(raster, pixel_map)?;
    let mut poller = Poller::new(poll);
    let cm = colour_map(raster, pixel_map, params, &mut poller)?;
    let rgba = cm
        .labels
        .iter()
        .flat_map(|&l| match l.checked_sub(1) {
            Some(i) => {
                let c = cm.palette[i as usize].colour;
                [c.r, c.g, c.b, 255]
            }
            None => [0, 0, 0, 0],
        })
        .collect();
    Ok((
        ColourMask {
            width: u32::try_from(cm.width).unwrap_or(u32::MAX),
            height: u32::try_from(cm.height).unwrap_or(u32::MAX),
            rgba,
            pixel_map: cm.map,
        },
        palette_of(&cm),
    ))
}
