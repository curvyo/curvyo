//! The mask region of a masked trace as pixels of the picture (`specs/0094-masked-composite-trace`
//! `adrs.md` decision 3), for the Mask view and the region-only analysis.
//!
//! The region's polygons are mapped into the pixel grid by the inverse of the picture's map, and
//! each row's pixel centres are filled by the nonzero rule from the edges crossing it, so the cost
//! is rows times edges, not pixels times edges.

use curvyo_document_core::{Affine, Point};
use curvyo_vectorize_core::{
    AnalysisReport, Raster, Rgb8, analyse, auto_threshold, suggest_background,
};

use crate::trace_region::TraceRegion;

/// Per pixel of a `width` x `height` grid whose corners `pixel_map` takes to document
/// millimetres: whether the trace keeps it (its centre lies in the kept area). `None` when the
/// map has no inverse.
#[must_use]
pub(crate) fn region_pixels(
    region: &TraceRegion,
    pixel_map: Affine,
    width: u32,
    height: u32,
) -> Option<Vec<bool>> {
    let inverse = pixel_map.inverse()?;
    let (w, h) = (width as usize, height as usize);
    let polygons: Vec<Vec<Point>> = region
        .polygons()
        .iter()
        .map(|polygon| polygon.iter().map(|&p| inverse.apply_point(p)).collect())
        .collect();
    let mut inside = vec![false; w * h];
    let mut crossings: Vec<(f64, i32)> = Vec::new();
    for y in 0..h {
        #[allow(clippy::cast_precision_loss)] // rows far below 2^52
        let centre = y as f64 + 0.5;
        crossings.clear();
        for polygon in &polygons {
            let mut a = polygon[polygon.len() - 1];
            for &b in polygon {
                let up = a.y <= centre && centre < b.y;
                let down = b.y <= centre && centre < a.y;
                if up || down {
                    let x = a.x + (centre - a.y) * (b.x - a.x) / (b.y - a.y);
                    crossings.push((x, if up { 1 } else { -1 }));
                }
                a = b;
            }
        }
        crossings.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
        let row = &mut inside[y * w..(y + 1) * w];
        let mut winding = 0;
        for pair in crossings.windows(2) {
            winding += pair[0].1;
            if winding != 0 {
                let (from, to) = (first_centre(pair[0].0, w), first_centre(pair[1].0, w));
                row[from..to].fill(true);
            }
        }
    }
    if region.outside() {
        for kept in &mut inside {
            *kept = !*kept;
        }
    }
    Some(inside)
}

/// The first pixel whose centre lies at or right of `x`, within `0..=w`.
fn first_centre(x: f64, w: usize) -> usize {
    let first = (x - 0.5).ceil();
    if first <= 0.0 {
        0
    } else {
        // invariant: positive and compared with `w` before the cast.
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        if first >= w as f64 { w } else { first as usize }
    }
}

/// The Mask view of a masked trace (criterion 10): the pixels of `bits` (a `width` x `height`
/// mask, `channels` bytes per pixel, the picture's grid times its Upscale factor; the picture is
/// `picture_width` px wide and mapped by `pixel_map`) that the trace does not keep are made white
/// (0 in a one-byte mask, opaque white in an RGBA one).
pub fn trace_whiten_outside_region(
    bits: &mut [u8],
    channels: usize,
    width: u32,
    height: u32,
    picture_width: u32,
    pixel_map: [f64; 6],
    region: &TraceRegion,
) {
    let factor = f64::from(width) / f64::from(picture_width.max(1));
    let map = Affine::from_array(pixel_map).compose(Affine::scale(1.0 / factor, 1.0 / factor));
    let Some(kept) = region_pixels(region, map, width, height) else {
        return;
    };
    for (i, _) in kept.iter().enumerate().filter(|(_, k)| !**k) {
        if let Some(px) = bits.get_mut(i * channels..(i + 1) * channels) {
            if channels == 1 {
                px[0] = 0;
            } else {
                px.fill(255);
            }
        }
    }
}

/// What the region-only analysis of a picture proposes (criterion 9).
#[derive(Debug, Clone, PartialEq)]
pub struct TraceRegionDefaults {
    /// The automatic Threshold of the region's pixels.
    pub threshold: u8,
    /// The background Remove background's Auto suggests: the most frequent colour of the
    /// region's border pixels; `None` for a transparent one.
    pub background: Option<Rgb8>,
    /// The analysis of the region's box, the pixels outside it transparent; `None` when it fails.
    pub analysis: Option<AnalysisReport>,
}

/// The automatic Threshold, background and analysis of the pixels of `rgba` (`width` x `height`,
/// mapped by `pixel_map`) that `region` keeps (`specs/0094` criterion 9, Proposal in `plan.md`);
/// the whole picture's when the region keeps none of it or the map has no inverse.
#[must_use]
pub fn trace_region_defaults(
    rgba: &[u8],
    width: u32,
    height: u32,
    pixel_map: [f64; 6],
    region: &TraceRegion,
) -> TraceRegionDefaults {
    let whole = Raster {
        width,
        height,
        rgba,
    };
    let fallback = || TraceRegionDefaults {
        threshold: auto_threshold(whole),
        background: suggest_background(whole),
        analysis: analyse(whole).ok(),
    };
    let (w, h) = (width as usize, height as usize);
    if rgba.len() != w * h * 4 {
        return fallback();
    }
    let Some(kept) = region_pixels(region, Affine::from_array(pixel_map), width, height) else {
        return fallback();
    };
    let pick = |keep: &dyn Fn(usize) -> bool| -> Vec<u8> {
        (0..w * h)
            .filter(|&i| keep(i))
            .flat_map(|i| &rgba[i * 4..i * 4 + 4])
            .copied()
            .collect()
    };
    let inside = pick(&|i| kept[i]);
    if inside.is_empty() {
        return fallback();
    }
    let border = pick(&|i| kept[i] && on_region_border(&kept, w, h, i));
    let threshold = auto_threshold(one_row(&inside));
    let background = suggest_background(one_row(&border));
    TraceRegionDefaults {
        threshold,
        background,
        analysis: region_box_analysis(rgba, w, &kept),
    }
}

/// `rgba` as one row of pixels.
fn one_row(rgba: &[u8]) -> Raster<'_> {
    Raster {
        width: u32::try_from(rgba.len() / 4).unwrap_or(u32::MAX),
        height: 1,
        rgba,
    }
}

/// Whether kept pixel `at` of a `width` x `height` grid lies on the region's border: on the picture's edge
/// or next to a pixel the region does not keep.
fn on_region_border(kept: &[bool], width: usize, height: usize, at: usize) -> bool {
    let (column, row) = (at % width, at / width);
    column == 0
        || row == 0
        || column + 1 == width
        || row + 1 == height
        || !kept[at - 1]
        || !kept[at + 1]
        || !kept[at - width]
        || !kept[at + width]
}

/// The analysis of the box of the `kept` pixels of `rgba` (`w` px wide), the others transparent.
fn region_box_analysis(rgba: &[u8], w: usize, kept: &[bool]) -> Option<AnalysisReport> {
    let (mut x0, mut y0, mut x1, mut y1) = (usize::MAX, usize::MAX, 0, 0);
    for (i, _) in kept.iter().enumerate().filter(|(_, k)| **k) {
        let (x, y) = (i % w, i / w);
        (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + 1), y1.max(y + 1));
    }
    let (cw, ch) = (x1.checked_sub(x0)?, y1.checked_sub(y0)?);
    let mut crop = Vec::with_capacity(cw * ch * 4);
    for y in y0..y1 {
        for x in x0..x1 {
            let i = y * w + x;
            if kept[i] {
                crop.extend_from_slice(&rgba[i * 4..i * 4 + 4]);
            } else {
                crop.extend_from_slice(&[255, 255, 255, 0]);
            }
        }
    }
    analyse(Raster {
        width: u32::try_from(cw).ok()?,
        height: u32::try_from(ch).ok()?,
        rgba: &crop,
    })
    .ok()
}
