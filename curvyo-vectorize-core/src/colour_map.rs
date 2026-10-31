//! Builds the label map of a multicolour trace (`specs/0089` adrs.md §10, criteria 26, 27,
//! 29) from the cleaned mask of the single-colour path: the palette, every content pixel's
//! colour, the maker's exclusions and merges, small colour regions joined.

use curvyo_document_core::Affine;

use crate::background::suggest_background;
use crate::colour_trace::ColourParams;
use crate::palette::{Entry, FLAT_BYTES, Lines, MERGE_DISTANCE, bytes, distance, palette};
use crate::params::{Rgb8, Selection};
use crate::progress::{Phase, Poller};
use crate::raster::Raster;
use crate::regions::Labels;
use crate::segment::{assign, join_small};
use crate::trace::{TraceError, masked};

/// A pixel takes part in the palette's vote only with at least this alpha.
const FLAT_ALPHA: u8 = 250;

/// The label map of a multicolour trace and what it was made from.
pub(crate) struct ColourMap {
    /// Per traced pixel: `0` background, `i + 1` palette entry `i`.
    pub labels: Vec<u32>,
    pub width: usize,
    pub height: usize,
    /// Traced pixels to millimetres.
    pub map: Affine,
    /// Traced pixels per source pixel (2 after Upscale).
    pub unit: usize,
    pub palette: Vec<Entry>,
    /// Per palette entry: the traced pixels it was assigned before exclusions, merges and joins.
    pub assigned: Vec<u64>,
    pub joined: usize,
    pub specks_removed: u64,
    pub foreground_pixels: u64,
}

/// The source picture and where its pixels lie in the traced mask.
struct Source<'a> {
    raster: Raster<'a>,
    width: usize,
    height: usize,
    /// Traced pixels per source pixel.
    unit: usize,
    /// The traced mask's width.
    traced_width: usize,
}

impl Source<'_> {
    /// The RGBA bytes of source pixel (`x`, `y`).
    fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.width + x) * 4;
        let p = &self.raster.rgba[i..i + 4];
        [p[0], p[1], p[2], p[3]]
    }

    /// The traced pixel at the top left of source pixel (`x`, `y`).
    fn traced(&self, x: usize, y: usize) -> usize {
        (y * self.unit) * self.traced_width + x * self.unit
    }
}

/// The label map [`crate::trace_colours`] walks for `params`.
pub(crate) fn colour_map(
    raster: Raster<'_>,
    pixel_map: Affine,
    params: &ColourParams,
    poller: &mut Poller<'_>,
) -> Result<ColourMap, TraceError> {
    let masked = masked(raster, pixel_map, &params.trace, poller)?;
    let width = masked.labels.width;
    let height = masked.labels.ids.len().checked_div(width).unwrap_or(0);
    let source = Source {
        raster,
        width: raster.width as usize,
        height: raster.height as usize,
        unit: masked.unit,
        traced_width: width,
    };
    let (entries, background) = entries(&source, params, &masked.labels, poller)?;
    poller.check(Phase::Tracing, 0, 1)?;
    let (mut out, assigned) = labelled(&source, &masked.labels, &entries, background, poller)?;
    let targets = targets(&entries, &params.excluded, &params.merged);
    for label in &mut out {
        if *label != 0 {
            *label = targets[*label as usize - 1];
        }
    }
    let min_pixels = masked.min_area_mm2 / masked.map.determinant().abs();
    let joined = if min_pixels > 0.0 {
        join_small(&mut out, width, height, min_pixels)
    } else {
        0
    };
    Ok(ColourMap {
        labels: out,
        width,
        height,
        map: masked.map,
        unit: masked.unit,
        palette: entries,
        assigned,
        joined,
        specks_removed: masked.specks_removed,
        foreground_pixels: masked.foreground_pixels,
    })
}

/// Every traced pixel's label (`0` outside the content of `mask`, else its palette index + 1,
/// each source pixel assigned once) and the traced pixels per palette entry.
fn labelled(
    source: &Source<'_>,
    mask: &Labels,
    entries: &[Entry],
    background: Option<Rgb8>,
    poller: &mut Poller<'_>,
) -> Result<(Vec<u32>, Vec<u64>), TraceError> {
    let width = mask.width;
    let height = mask.ids.len().checked_div(width).unwrap_or(0);
    let colours: Vec<[f64; 3]> = entries.iter().map(|e| bytes(e.colour)).collect();
    let background = background.map(bytes);
    let mut out = vec![0_u32; width * height];
    let mut assigned = vec![0_u64; entries.len()];
    if colours.is_empty() {
        return Ok((out, assigned));
    }
    let (sw, sh) = (source.width, source.height);
    // Each source pixel's palette index, found once (`u8::MAX`: not yet).
    let mut cache = vec![u8::MAX; sw * sh];
    for y in 0..height {
        poller.row(y, height)?;
        for x in 0..width {
            let i = y * width + x;
            if !mask.regions[mask.ids[i] as usize].foreground {
                continue;
            }
            let (px, py) = ((x / source.unit).min(sw - 1), (y / source.unit).min(sh - 1));
            let slot = &mut cache[py * sw + px];
            if *slot == u8::MAX {
                let p = source.pixel(px, py);
                let colour = [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])];
                // invariant: a palette holds at most 16 colours.
                *slot = u8::try_from(assign(colour, &colours, background)).unwrap_or(0);
            }
            assigned[usize::from(*slot)] += 1;
            out[i] = u32::from(*slot) + 1;
        }
    }
    Ok((out, assigned))
}

/// The palette of a trace and the background its blends run toward: the picked colours of Pick
/// colours, else the palette of the flat content pixels (and the ridges of content without
/// any) with the fixed colours.
fn entries(
    source: &Source<'_>,
    params: &ColourParams,
    mask: &Labels,
    poller: &mut Poller<'_>,
) -> Result<(Vec<Entry>, Option<Rgb8>), TraceError> {
    Ok(match params.trace.selection {
        Selection::PickColours {
            swatches,
            background,
            ..
        } => {
            let picked: Vec<Entry> = swatches
                .as_slice()
                .iter()
                .map(|&colour| Entry {
                    colour,
                    fixed: true,
                })
                .collect();
            (picked, background)
        }
        selection => {
            let background = match selection {
                Selection::RemoveBackground { background, .. } => background,
                _ => suggest_background(source.raster),
            };
            let (flat, lines) = samples(source, mask, background.map(bytes), poller)?;
            let count = params.colours.map(usize::from);
            (palette(&flat, &lines, count, &params.fixed), background)
        }
    })
}

/// The flat content pixels of `source`, and the ridge pixels (no content neighbour farther
/// from `toward`) of the content regions of `mask` that hold no flat pixel.
fn samples(
    source: &Source<'_>,
    mask: &Labels,
    toward: Option<[f64; 3]>,
    poller: &mut Poller<'_>,
) -> Result<(Vec<[u8; 3]>, Lines), TraceError> {
    let (sw, sh) = (source.width, source.height);
    let region = |x: usize, y: usize| mask.ids[source.traced(x, y)] as usize;
    let content = |x: usize, y: usize| mask.regions[region(x, y)].foreground;
    let mut flat_in = vec![false; mask.regions.len()];
    let mut out = Vec::new();
    for y in 0..sh {
        poller.row(y, sh)?;
        for x in 0..sw {
            if content(x, y) && flat(source, (x, y)) {
                let p = source.pixel(x, y);
                out.push([p[0], p[1], p[2]]);
                flat_in[region(x, y)] = true;
            }
        }
    }
    let mut lines = Lines {
        samples: Vec::new(),
        toward,
    };
    if flat_in
        .iter()
        .zip(&mask.regions)
        .all(|(&f, r)| f || !r.foreground)
    {
        return Ok((out, lines));
    }
    let strength = |x: usize, y: usize| {
        let p = source.pixel(x, y);
        let away = toward.map_or(255.0, |bg| {
            distance(bg, [f64::from(p[0]), f64::from(p[1]), f64::from(p[2])])
        });
        away * f64::from(p[3])
    };
    for y in 0..sh {
        poller.row(y, sh)?;
        for x in 0..sw {
            if !content(x, y) || flat_in[region(x, y)] {
                continue;
            }
            let own = strength(x, y);
            let ridge = (y.saturating_sub(1)..(y + 2).min(sh)).all(|row| {
                (x.saturating_sub(1)..(x + 2).min(sw))
                    .all(|column| !content(column, row) || strength(column, row) <= own)
            });
            if ridge {
                let p = source.pixel(x, y);
                lines.samples.push([p[0], p[1], p[2]]);
            }
        }
    }
    Ok((out, lines))
}

/// Whether the source pixel (`x`, `y`) is flat: opaque, and every opaque neighbour of its 3 x 3
/// window within [`FLAT_BYTES`] per channel.
fn flat(source: &Source<'_>, (x, y): (usize, usize)) -> bool {
    let centre = source.pixel(x, y);
    if centre[3] < FLAT_ALPHA {
        return false;
    }
    for row in y.saturating_sub(1)..(y + 2).min(source.height) {
        for column in x.saturating_sub(1)..(x + 2).min(source.width) {
            let other = source.pixel(column, row);
            if other[3] < FLAT_ALPHA {
                continue;
            }
            if (0..3).any(|k| centre[k].abs_diff(other[k]) > FLAT_BYTES) {
                return false;
            }
        }
    }
    true
}

/// The label every palette entry ends as: `0` when one of `excluded` is within the merge
/// distance of it, the target's label when it is merged into another entry (`merged` as from,
/// into), else its own; chains of merges are followed, a cycle keeps its colours.
fn targets(entries: &[Entry], excluded: &[Rgb8], merged: &[(Rgb8, Rgb8)]) -> Vec<u32> {
    let matches =
        |entry: &Entry, colour: Rgb8| distance(bytes(entry.colour), bytes(colour)) < MERGE_DISTANCE;
    let find = |colour: Rgb8| entries.iter().position(|e| matches(e, colour));
    let mut into: Vec<Option<usize>> = vec![None; entries.len()];
    for &(from, to) in merged {
        if let (Some(f), Some(t)) = (find(from), find(to))
            && f != t
        {
            into[f] = Some(t);
        }
    }
    (0..entries.len())
        .map(|i| {
            let mut at = i;
            for _ in 0..entries.len() {
                match into[at] {
                    Some(t) if t != i => at = t,
                    _ => break,
                }
            }
            if excluded.iter().any(|&c| matches(&entries[at], c)) {
                0
            } else {
                // invariant: at most 16 entries.
                u32::try_from(at + 1).unwrap_or(0)
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclusions_and_merges_find_their_entries() {
        let e = |r, g, b| Entry {
            colour: Rgb8::new(r, g, b),
            fixed: false,
        };
        let entries = [e(200, 0, 0), e(0, 0, 200), e(0, 200, 0)];
        // Green merged into blue, red excluded (given 2 bytes off).
        let t = targets(
            &entries,
            &[Rgb8::new(202, 0, 0)],
            &[(Rgb8::new(0, 200, 0), Rgb8::new(0, 0, 200))],
        );
        assert_eq!(t, [0, 2, 2]);
        // Merged into an excluded colour: background.
        let t = targets(
            &entries,
            &[Rgb8::new(0, 0, 200)],
            &[(Rgb8::new(0, 200, 0), Rgb8::new(0, 0, 200))],
        );
        assert_eq!(t, [1, 0, 0]);
    }
}
