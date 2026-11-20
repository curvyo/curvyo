//! Local contrast as contrast-limited adaptive histogram equalisation (CLAHE, ADR 0073 §4) in
//! integer histograms and tables.

use crate::adjust_blur::SubPx;
use crate::adjust_plane::GreyPlane;

/// The smallest tile side in pixels.
const MIN_TILE: u32 = 8;

/// `plane` with local contrast in place: tiles of side `2 · radius` (at least 8 px) from the top
/// left, a 256-bin histogram each clipped at `mean · (1 + amount / 25)` with the excess spread
/// evenly in exact fractions, each tile's equalisation table, and per pixel the
/// bilinear blend of the four nearest tile tables in 1/256. Amount 0 changes nothing.
#[cfg(test)]
pub(crate) fn local_contrast(plane: &mut GreyPlane, radius: SubPx, amount_pct: u16) {
    let mut run = ClaheRun::new(plane, radius, amount_pct);
    let mut all = u64::MAX;
    run.work(plane, &mut all);
}

/// The work units of one blended pixel: four table reads and a bilinear sum.
const BLEND_WORK: u64 = 3;

/// Local contrast worked a band at a time (`specs/0135` criterion 13): the tile tables one row
/// of tiles at a time, then the pixels one row at a time; the bytes of [`local_contrast`].
#[derive(Debug, Clone)]
pub(crate) struct ClaheRun {
    amount: u16,
    side: u32,
    /// Tiles across and down.
    tiles: (u32, u32),
    tables: Vec<[u8; 256]>,
    /// The tile row whose tables are made next.
    tile_row: u32,
    columns: Vec<(usize, usize, u32)>,
    rows: Vec<(usize, usize, u32)>,
    row: usize,
}

impl ClaheRun {
    /// The local contrast of `plane`, nothing worked yet.
    pub(crate) fn new(plane: &GreyPlane, radius: SubPx, amount_pct: u16) -> Self {
        let side = ((2 * radius.0 + SubPx::PER_PX / 2) / SubPx::PER_PX).max(MIN_TILE);
        let (w, h) = (plane.width, plane.height);
        let (nx, ny) = (w.div_ceil(side), h.div_ceil(side));
        let idle = amount_pct == 0;
        Self {
            amount: amount_pct,
            side,
            tiles: (nx, ny),
            tables: Vec::with_capacity(if idle { 0 } else { nx as usize * ny as usize }),
            tile_row: if idle { ny } else { 0 },
            columns: if idle {
                Vec::new()
            } else {
                blend_axis(w, side, nx)
            },
            rows: if idle {
                Vec::new()
            } else {
                blend_axis(h, side, ny)
            },
            row: if idle { h as usize } else { 0 },
        }
    }

    /// Works `plane` (the plane the run was made from) until `work` is spent, at least one row
    /// of tiles or of pixels; whether the step is finished.
    pub(crate) fn work(&mut self, plane: &mut GreyPlane, work: &mut u64) -> bool {
        let (nx, ny) = self.tiles;
        let h = plane.height as usize;
        while self.tile_row < ny {
            tile_row_tables(
                plane,
                self.side,
                (nx, self.tile_row),
                self.amount,
                &mut self.tables,
            );
            self.tile_row += 1;
            let rows = u64::from(
                self.side
                    .min(plane.height - (self.tile_row - 1) * self.side),
            );
            *work = work.saturating_sub(rows * u64::from(plane.width));
            if *work == 0 && (self.tile_row < ny || self.row < h) {
                return false;
            }
        }
        let w = plane.width as usize;
        let nx = nx as usize;
        while self.row < h {
            let (t0, t1, fy) = self.rows[self.row];
            let gy = 256 - fy;
            let line = &mut plane.grey[self.row * w..(self.row + 1) * w];
            for (x, g) in line.iter_mut().enumerate() {
                let (s0, s1, fx) = self.columns[x];
                let gx = 256 - fx;
                let v = usize::from(*g);
                let m = |t: usize, s: usize| u32::from(self.tables[t * nx + s][v]);
                let sum = m(t0, s0) * gx * gy
                    + m(t0, s1) * fx * gy
                    + m(t1, s0) * gx * fy
                    + m(t1, s1) * fx * fy;
                // invariant: the weights sum to 65,536 and each table entry is a byte.
                #[allow(clippy::cast_possible_truncation)]
                let out = ((sum + 32_768) >> 16) as u8;
                *g = out;
            }
            self.row += 1;
            *work = work.saturating_sub(BLEND_WORK * w as u64);
            if *work == 0 && self.row < h {
                return false;
            }
        }
        true
    }
}

/// The equalisation tables of the tiles of tile row `ty`, appended to `tables`.
fn tile_row_tables(
    plane: &GreyPlane,
    side: u32,
    (nx, ty): (u32, u32),
    amount: u16,
    tables: &mut Vec<[u8; 256]>,
) {
    let w = plane.width as usize;
    let y0 = (ty * side) as usize;
    let y1 = ((ty + 1) * side).min(plane.height) as usize;
    for tx in 0..nx {
        let x0 = (tx * side) as usize;
        let x1 = ((tx + 1) * side).min(plane.width) as usize;
        let mut histogram = [0_u32; 256];
        for row in plane.grey[y0 * w..y1 * w].chunks_exact(w) {
            for &g in &row[x0..x1] {
                histogram[usize::from(g)] += 1;
            }
        }
        tables.push(equalised(&histogram, amount));
    }
}

/// The table of a clipped histogram: `round(255 · cdf(g) / total)`, where the bins are clipped at
/// `mean · (1 + amount / 25)` and the excess is spread over all 256 bins in exact fractions (in
/// 1/65,536 counts), so a flat tile maps its grey to itself whatever its size.
fn equalised(histogram: &[u32; 256], amount: u16) -> [u8; 256] {
    let total: u64 = histogram.iter().map(|&b| u64::from(b)).sum();
    // The clip in 1/256 counts: `total / 256 · (25 + amount) / 25 · 256`.
    let clip = total * (25 + u64::from(amount)) / 25;
    let excess: u64 = histogram
        .iter()
        .map(|&b| (256 * u64::from(b)).saturating_sub(clip))
        .sum();
    let mut table = [0_u8; 256];
    let mut kept = 0_u64;
    let whole = 65_536 * total.max(1);
    for ((slot, &bin), g) in table.iter_mut().zip(histogram.iter()).zip(1_u64..) {
        kept += (256 * u64::from(bin)).min(clip);
        // In 1/65,536 counts: the clipped bins up to `g` and `g / 256` of the excess.
        let cdf = 256 * kept + excess * g;
        *slot = u8::try_from((255 * cdf + whole / 2) / whole).unwrap_or(u8::MAX);
    }
    table
}

/// Per pixel along one axis of `len` pixels: the two tiles whose centres bracket it and the
/// weight of the second in 1/256 (0 before the first centre and after the last).
fn blend_axis(len: u32, side: u32, tiles: u32) -> Vec<(usize, usize, u32)> {
    // Centres in half pixels: 2 · t · side + the tile's own length.
    let centre = |t: u32| i64::from(2 * t * side + side.min(len - t * side));
    (0..len)
        .map(|p| {
            let at = 2 * i64::from(p) + 1;
            let mut t0 = 0;
            while t0 + 1 < tiles && centre(t0 + 1) <= at {
                t0 += 1;
            }
            let t1 = (t0 + 1).min(tiles - 1);
            let (c0, c1) = (centre(t0), centre(t1));
            let f = if c1 > c0 && at > c0 {
                ((at - c0) * 256 / (c1 - c0)).clamp(0, 256)
            } else {
                0
            };
            (t0 as usize, t1 as usize, u32::try_from(f).unwrap_or(0))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flat_plane_stays_nearly_flat_and_amount_zero_changes_nothing() {
        let mut flat = GreyPlane {
            width: 40,
            height: 40,
            grey: vec![120; 1_600],
        };
        let before = flat.clone();
        local_contrast(&mut flat, SubPx(160), 0);
        assert_eq!(flat, before);
        local_contrast(&mut flat, SubPx(160), 50);
        assert!(flat.grey.iter().all(|&g| g.abs_diff(flat.grey[0]) == 0));
    }

    #[test]
    fn low_contrast_detail_is_spread() {
        let mut p = GreyPlane {
            width: 64,
            height: 64,
            grey: (0..4_096_u32)
                .map(|i| u8::try_from(120 + (i % 64) / 8).unwrap())
                .collect(),
        };
        local_contrast(&mut p, SubPx(16 * 16), 100);
        let row = &p.grey[32 * 64..33 * 64];
        let (lo, hi) = (row.iter().min().unwrap(), row.iter().max().unwrap());
        assert!(hi - lo > 14, "twice the span of 7: {lo} {hi}");
    }
}
