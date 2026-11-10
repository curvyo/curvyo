//! The automatic background colour of an image, read from its border.

use crate::params::Rgb8;
use crate::raster::Raster;
use crate::select::OPAQUE;

/// The background colour of Remove background's Auto (criterion 16, `adrs.md` §3): the most
/// frequent colour of the opaque border pixels, quantised to 5 bits per channel (ties by the
/// smaller value), as the per-channel median of the border pixels in that bin. `None` when at
/// most half the border is opaque (a transparent background) or the raster is invalid.
#[must_use]
pub fn suggest_background(raster: Raster<'_>) -> Option<Rgb8> {
    raster.checked_pixels()?;
    let (w, h) = (raster.width as usize, raster.height as usize);
    let at = |x: usize, y: usize| {
        let i = (y * w + x) * 4;
        [
            raster.rgba[i],
            raster.rgba[i + 1],
            raster.rgba[i + 2],
            raster.rgba[i + 3],
        ]
    };
    let mut border = Vec::with_capacity(2 * (w + h));
    for x in 0..w {
        border.push(at(x, 0));
        if h > 1 {
            border.push(at(x, h - 1));
        }
    }
    for y in 1..h.saturating_sub(1) {
        border.push(at(0, y));
        if w > 1 {
            border.push(at(w - 1, y));
        }
    }
    let border_len = border.len();
    let opaque: Vec<[u8; 4]> = border.into_iter().filter(|p| p[3] >= OPAQUE).collect();
    if opaque.len() * 2 <= border_len {
        return None;
    }
    let bin = |p: &[u8; 4]| {
        usize::from(p[0] >> 3) << 10 | usize::from(p[1] >> 3) << 5 | usize::from(p[2] >> 3)
    };
    let mut counts = vec![0_u32; 1 << 15];
    for p in &opaque {
        counts[bin(p)] += 1;
    }
    let mut best = 0;
    for (i, &count) in counts.iter().enumerate() {
        if count > counts[best] {
            best = i;
        }
    }
    let members: Vec<&[u8; 4]> = opaque.iter().filter(|p| bin(p) == best).collect();
    let median = |k: usize| {
        let mut values: Vec<u8> = members.iter().map(|p| p[k]).collect();
        values.sort_unstable();
        values[values.len() / 2]
    };
    Some(Rgb8::new(median(0), median(1), median(2)))
}

/// The colour a pipette click at pixel (`x`, `y`) takes (criterion 17, `ux-notes.md` §6): the
/// per-channel median of the opaque pixels of the 3 x 3 window around it, so a click on an
/// anti-aliased edge does not take a blend; `None` outside the picture or on a transparent
/// pixel.
#[must_use]
pub fn sample_colour(raster: Raster<'_>, x: u32, y: u32) -> Option<Rgb8> {
    raster.checked_pixels()?;
    if x >= raster.width || y >= raster.height {
        return None;
    }
    let (width, height) = (raster.width as usize, raster.height as usize);
    let (x, y) = (x as usize, y as usize);
    let pixel = |x: usize, y: usize| {
        let i = (y * width + x) * 4;
        [
            raster.rgba[i],
            raster.rgba[i + 1],
            raster.rgba[i + 2],
            raster.rgba[i + 3],
        ]
    };
    if pixel(x, y)[3] < OPAQUE {
        return None;
    }
    let mut window = Vec::with_capacity(9);
    for yy in y.saturating_sub(1)..(y + 2).min(height) {
        for xx in x.saturating_sub(1)..(x + 2).min(width) {
            let p = pixel(xx, yy);
            if p[3] >= OPAQUE {
                window.push(p);
            }
        }
    }
    let median = |k: usize| {
        let mut values: Vec<u8> = window.iter().map(|p| p[k]).collect();
        values.sort_unstable();
        values[values.len() / 2]
    };
    Some(Rgb8::new(median(0), median(1), median(2)))
}
