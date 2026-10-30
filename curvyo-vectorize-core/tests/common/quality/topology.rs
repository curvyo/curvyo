//! Topology of a trace against its truth on the 4x grid (`specs/0089-trace-quality-and-colour`
//! criterion 3): components or holes lost or extra, connections split or merged.

/// The regions of the cells equal to `value`, 4-connected or with `diagonal` 8-connected: a label
/// per cell (`u32::MAX` for the other value) and, per region, its size and whether it touches the
/// border.
pub fn label(
    mask: &[bool],
    w: usize,
    h: usize,
    value: bool,
    diagonal: bool,
) -> (Vec<u32>, Vec<(usize, bool)>) {
    let mut labels = vec![u32::MAX; mask.len()];
    let mut regions = Vec::new();
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if mask[start] != value || labels[start] != u32::MAX {
            continue;
        }
        let id = regions.len() as u32;
        labels[start] = id;
        stack.push(start);
        let (mut size, mut border) = (0_usize, false);
        while let Some(i) = stack.pop() {
            size += 1;
            let (x, y) = (i % w, i / w);
            border |= x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            let mut visit = |j: usize| {
                if mask[j] == value && labels[j] == u32::MAX {
                    labels[j] = id;
                    stack.push(j);
                }
            };
            if x > 0 {
                visit(i - 1);
            }
            if x + 1 < w {
                visit(i + 1);
            }
            if y > 0 {
                visit(i - w);
            }
            if y + 1 < h {
                visit(i + w);
            }
            if diagonal {
                if x > 0 && y > 0 {
                    visit(i - w - 1);
                }
                if x + 1 < w && y > 0 {
                    visit(i - w + 1);
                }
                if x > 0 && y + 1 < h {
                    visit(i + w - 1);
                }
                if x + 1 < w && y + 1 < h {
                    visit(i + w + 1);
                }
            }
        }
        regions.push((size, border));
    }
    (labels, regions)
}

/// The topology counts of a trace against the truth (both 4x rasters of the same size).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Topology {
    pub truth_components: usize,
    pub truth_holes: usize,
    /// True components the trace does not touch at all.
    pub components_lost: usize,
    /// Traced components touching no true component (noise, specks).
    pub components_extra: usize,
    /// True components split into several traced ones (a lost connection).
    pub connections_lost: usize,
    /// Traced components joining several true ones (a false connection).
    pub connections_extra: usize,
    /// True holes no traced hole overlaps.
    pub holes_lost: usize,
    /// Traced holes overlapping no true hole.
    pub holes_extra: usize,
}

/// Overlaps below this many cells (1 px² at 4x) do not count as a connection.
const MIN_OVERLAP: usize = 16;

/// The pairs of labels of `a` and `b` overlapping by at least `min` cells.
fn overlaps(a: &[u32], b: &[u32], min: usize) -> Vec<(u32, u32)> {
    let mut pairs: Vec<(u32, u32)> = a
        .iter()
        .zip(b)
        .filter(|(x, y)| **x != u32::MAX && **y != u32::MAX)
        .map(|(x, y)| (*x, *y))
        .collect();
    pairs.sort_unstable();
    let mut out = Vec::new();
    let mut i = 0;
    while i < pairs.len() {
        let mut j = i;
        while j < pairs.len() && pairs[j] == pairs[i] {
            j += 1;
        }
        if j - i >= min {
            out.push(pairs[i]);
        }
        i = j;
    }
    out
}

/// Counts per label of how many partners it has in `pairs` (by side).
fn degrees(pairs: &[(u32, u32)], n: usize, first: bool) -> Vec<usize> {
    let mut d = vec![0; n];
    for &(a, b) in pairs {
        d[if first { a } else { b } as usize] += 1;
    }
    d
}

/// The topology of `trace` (4x raster) against `truth` (4x raster), `w` x `h` cells.
pub fn compare(truth: &[bool], trace: &[bool], w: usize, h: usize) -> Topology {
    // Foreground 8-connected, background 4-connected (the dual pair): the 4x samples of a sharp
    // tip touch only diagonally and still belong to their shape.
    let (t_fg, t_regions) = label(truth, w, h, true, true);
    let (r_fg, r_regions) = label(trace, w, h, true, true);
    let (t_bg, t_bg_regions) = label(truth, w, h, false, false);
    let (r_bg, r_bg_regions) = label(trace, w, h, false, false);
    let touching = overlaps(&t_fg, &r_fg, 1);
    let t_touch = degrees(&touching, t_regions.len(), true);
    let r_touch = degrees(&touching, r_regions.len(), false);
    let fg = overlaps(&t_fg, &r_fg, MIN_OVERLAP);
    let t_deg = degrees(&fg, t_regions.len(), true);
    let r_deg = degrees(&fg, r_regions.len(), false);
    let holes = |regions: &[(usize, bool)]| -> Vec<bool> { regions.iter().map(|r| !r.1).collect() };
    let (t_hole, r_hole) = (holes(&t_bg_regions), holes(&r_bg_regions));
    let bg: Vec<(u32, u32)> = overlaps(&t_bg, &r_bg, 1)
        .into_iter()
        .filter(|&(a, b)| t_hole[a as usize] && r_hole[b as usize])
        .collect();
    let th_deg = degrees(&bg, t_bg_regions.len(), true);
    let rh_deg = degrees(&bg, r_bg_regions.len(), false);
    Topology {
        truth_components: t_regions.len(),
        truth_holes: t_hole.iter().filter(|h| **h).count(),
        components_lost: t_touch.iter().filter(|d| **d == 0).count(),
        components_extra: r_touch.iter().filter(|d| **d == 0).count(),
        connections_lost: t_deg.iter().filter(|d| **d > 1).count(),
        connections_extra: r_deg.iter().filter(|d| **d > 1).count(),
        holes_lost: (0..t_hole.len())
            .filter(|&i| t_hole[i] && th_deg[i] == 0)
            .count(),
        holes_extra: (0..r_hole.len())
            .filter(|&i| r_hole[i] && rh_deg[i] == 0)
            .count(),
    }
}

/// `mask` (1x) after the tracer's Specks step: foreground regions (8-connected, as the tracer
/// labels them since `specs/0089` Frage 96 A) under `min_pixels` become background, then holes
/// (4-connected) under it are filled.
pub fn without_specks(mask: &[bool], w: usize, h: usize, min_pixels: f64) -> Vec<bool> {
    let mut mask = mask.to_vec();
    for (value, holes_only) in [(true, false), (false, true)] {
        let (labels, regions) = label(&mask, w, h, value, value);
        for (i, cell) in mask.iter_mut().enumerate() {
            let l = labels[i];
            if l == u32::MAX {
                continue;
            }
            let (size, border) = regions[l as usize];
            if (size as f64) < min_pixels && !(holes_only && border) {
                *cell = !value;
            }
        }
    }
    mask
}
