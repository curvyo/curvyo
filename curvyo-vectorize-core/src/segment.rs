//! Assigns every content pixel of a multicolour trace to one palette colour (`specs/0089`
//! adrs.md §10, criterion 27): the nearest colour, or the dominant one of a two-colour blend
//! that explains the pixel far better (anti-aliased seams between colours and toward the
//! background); excluded colours become background, merged ones their target; regions below
//! Ignore below join the colour neighbour with the longest shared border.
//!
//! Labels are `0` for background and `i + 1` for palette entry `i`. Regions of one label are
//! 4-connected plus their diagonal touches where the two other pixels of the 2 x 2 block differ
//! from it; in a 2-label checkerboard only the winner of [`wins`] connects (a colour over the
//! background, the later palette entry over the earlier), the same rule `edges.rs` walks.

use crate::palette::{MERGE_DISTANCE, distance};

/// A blend wins over the nearest colour when its residual is below this share of the nearest
/// colour's (squared distances: half the distance).
const BLEND_SHARE: f64 = 0.25;

/// How many of the nearest colours are tried as blends.
const BLEND_NEAREST: usize = 4;

/// At most this many passes of joining small regions.
const JOIN_PASSES: usize = 8;

/// Whether label `a` connects across a 2 x 2 checkerboard over label `b`.
pub(crate) fn wins(a: u32, b: u32) -> bool {
    a != 0 && (b == 0 || a > b)
}

/// The palette index for the colour `p` (bytes): the nearest of `palette` when it lies within
/// the merge distance, else the dominant colour of the blend of two of the [`BLEND_NEAREST`]
/// nearest colours (or of one with `background`) that explains `p` with a residual below
/// [`BLEND_SHARE`] of the nearest colour's, else the nearest (ties by palette order).
pub(crate) fn assign(p: [f64; 3], palette: &[[f64; 3]], background: Option<[f64; 3]>) -> usize {
    let mut nearest = [(f64::INFINITY, usize::MAX); BLEND_NEAREST];
    for (i, c) in palette.iter().enumerate() {
        let d = squared(p, *c);
        let mut at = BLEND_NEAREST;
        while at > 0 && d < nearest[at - 1].0 {
            at -= 1;
        }
        if at < BLEND_NEAREST {
            nearest.copy_within(at..BLEND_NEAREST - 1, at + 1);
            nearest[at] = (d, i);
        }
    }
    let (best_d, best) = nearest[0];
    if best_d < MERGE_DISTANCE * MERGE_DISTANCE {
        return best;
    }
    let mut label = best;
    let mut residual = best_d * BLEND_SHARE;
    let found = nearest.iter().take_while(|n| n.1 != usize::MAX).count();
    for a in 0..found {
        let i = nearest[a].1;
        for &(_, j) in &nearest[a + 1..found] {
            let (r, t) = blend(p, palette[i], palette[j]);
            if r < residual {
                residual = r;
                label = if t < 0.5 {
                    i
                } else if t > 0.5 {
                    j
                } else {
                    i.min(j)
                };
            }
        }
        if let Some(bg) = background {
            let (r, _) = blend(p, palette[i], bg);
            if r < residual {
                residual = r;
                label = i;
            }
        }
    }
    label
}

/// The squared distance of `p` from the segment `a`–`b` and the share of `b` at its foot.
fn blend(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> (f64, f64) {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ap = [p[0] - a[0], p[1] - a[1], p[2] - a[2]];
    let len = ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2];
    if len == 0.0 {
        return (squared(p, a), 0.0);
    }
    let t = ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / len).clamp(0.0, 1.0);
    let foot = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
    (squared(p, foot), t)
}

fn squared(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = distance(a, b);
    d * d
}

/// The region of every pixel of `labels` (`width` x `height`) under the connectivity of the
/// module comment, numbered by first scan position, and each region's label and size.
pub(crate) fn regions(labels: &[u32], width: usize, height: usize) -> (Vec<u32>, Vec<(u32, u64)>) {
    let mut parent: Vec<u32> = (0..u32::try_from(labels.len()).unwrap_or(u32::MAX)).collect();
    let find = |parent: &mut Vec<u32>, mut i: u32| {
        while parent[i as usize] != i {
            let up = parent[parent[i as usize] as usize];
            parent[i as usize] = up;
            i = up;
        }
        i
    };
    let join = |parent: &mut Vec<u32>, a: usize, b: usize| {
        // invariant: pixel indices fit `u32` (pictures are at most 64 Mpx).
        let (ra, rb) = (
            find(parent, u32::try_from(a).unwrap_or(0)),
            find(parent, u32::try_from(b).unwrap_or(0)),
        );
        let (keep, drop) = if ra <= rb { (ra, rb) } else { (rb, ra) };
        parent[drop as usize] = keep;
    };
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let l = labels[i];
            if x + 1 < width && labels[i + 1] == l {
                join(&mut parent, i, i + 1);
            }
            if y + 1 < height && labels[i + width] == l {
                join(&mut parent, i, i + width);
            }
            if x + 1 < width && y + 1 < height {
                let (tl, tr, bl, br) = (l, labels[i + 1], labels[i + width], labels[i + width + 1]);
                if tl == br && tr != tl && bl != tl && (tr != bl || wins(tl, tr)) && tl != 0 {
                    join(&mut parent, i, i + width + 1);
                }
                if tr == bl && tl != tr && br != tr && (tl != br || wins(tr, tl)) && tr != 0 {
                    join(&mut parent, i + 1, i + width);
                }
            }
        }
    }
    let mut ids = vec![u32::MAX; labels.len()];
    let mut out: Vec<(u32, u64)> = Vec::new();
    for i in 0..labels.len() {
        let root = find(&mut parent, u32::try_from(i).unwrap_or(0)) as usize;
        if ids[root] == u32::MAX {
            ids[root] = u32::try_from(out.len()).unwrap_or(u32::MAX);
            out.push((labels[i], 0));
        }
        let id = ids[root];
        ids[i] = id;
        out[id as usize].1 += 1;
    }
    (ids, out)
}

/// Joins every colour region of fewer than `min_pixels` pixels to the colour neighbour it
/// shares the longest border with (ties by palette order; the background only when it has no
/// colour neighbour, so the content stays the mask's), smallest first, each seeing the joins
/// before it, pass by pass until none is left or after [`JOIN_PASSES`]; returns how many
/// regions were joined.
pub(crate) fn join_small(
    labels: &mut [u32],
    width: usize,
    height: usize,
    min_pixels: f64,
) -> usize {
    let mut joined = 0;
    for _ in 0..JOIN_PASSES {
        let (ids, regions) = regions(labels, width, height);
        #[allow(clippy::cast_precision_loss)] // at most 64 Mpx
        let small: Vec<bool> = regions
            .iter()
            .map(|&(l, n)| l != 0 && (n as f64) < min_pixels)
            .collect();
        if !small.iter().any(|&s| s) {
            break;
        }
        let pairs = borders(&ids, &small, width, height);
        let mut label: Vec<u32> = regions.iter().map(|r| r.0).collect();
        let mut order: Vec<usize> = (0..regions.len()).filter(|&r| small[r]).collect();
        order.sort_by_key(|&r| (regions[r].1, r));
        let mut changed = 0;
        for r in order {
            let r32 = u32::try_from(r).unwrap_or(u32::MAX);
            let from = pairs.partition_point(|p| p.0 < r32);
            let to = pairs.partition_point(|p| p.0 <= r32);
            if let Some(l) = longest_border(&pairs[from..to], &label)
                && l != label[r]
            {
                label[r] = l;
                changed += 1;
            }
        }
        for (i, l) in labels.iter_mut().enumerate() {
            *l = label[ids[i] as usize];
        }
        joined += changed;
        if changed == 0 {
            break;
        }
    }
    joined
}

/// Per crack edge between two regions of `ids` (`width` x `height`), one (small region,
/// neighbour region) pair for each side that is `small`, sorted.
fn borders(ids: &[u32], small: &[bool], width: usize, height: usize) -> Vec<(u32, u32)> {
    let mut pairs: Vec<(u32, u32)> = Vec::new();
    for y in 0..height {
        for x in 0..width {
            let i = y * width + x;
            let r = ids[i];
            let mut look = |j: usize| {
                let s = ids[j];
                if s != r {
                    if small[r as usize] {
                        pairs.push((r, s));
                    }
                    if small[s as usize] {
                        pairs.push((s, r));
                    }
                }
            };
            if x + 1 < width {
                look(i + 1);
            }
            if y + 1 < height {
                look(i + width);
            }
        }
    }
    pairs.sort_unstable();
    pairs
}

/// The label of the neighbour one small region shares the most crack edges with (`pairs` its
/// border pairs, `label` every region's current label): a colour before the background, then
/// the longest border, then the lower label.
fn longest_border(pairs: &[(u32, u32)], label: &[u32]) -> Option<u32> {
    let mut tally: Vec<u32> = pairs.iter().map(|p| label[p.1 as usize]).collect();
    tally.sort_unstable();
    // (a colour, shared edges, lower label first)
    let mut best: Option<((bool, usize), u32)> = None;
    let mut k = 0;
    while k < tally.len() {
        let mut e = k;
        while e < tally.len() && tally[e] == tally[k] {
            e += 1;
        }
        let key = (tally[k] != 0, e - k);
        if best.is_none_or(|(b, _)| key > b) {
            best = Some((key, tally[k]));
        }
        k = e;
    }
    best.map(|(_, l)| l)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&str]) -> (Vec<u32>, usize, usize) {
        let labels = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c.to_digit(10).unwrap()))
            .collect();
        (labels, rows[0].len(), rows.len())
    }

    #[test]
    fn blends_go_to_their_dominant_colour_and_points_to_their_own() {
        let red = [255.0, 0.0, 0.0];
        let yellow = [255.0, 255.0, 0.0];
        let orange = [255.0, 128.0, 0.0];
        let blue = [0.0, 0.0, 255.0];
        let palette = [red, yellow, orange, blue];
        // Orange itself lies on the red-yellow segment: it stays orange.
        assert_eq!(assign(orange, &palette, None), 2);
        // A 60/40 red-blue blend goes to red, a 30/70 one to blue.
        assert_eq!(assign([153.0, 0.0, 102.0], &palette, None), 0);
        assert_eq!(assign([76.0, 0.0, 178.0], &palette, None), 3);
        // A pink blend of red and the white background stays red.
        assert_eq!(
            assign([255.0, 160.0, 160.0], &palette, Some([255.0, 255.0, 255.0])),
            0
        );
    }

    #[test]
    fn diagonal_touches_connect_by_the_rule() {
        // 1 and 2 in a checkerboard: the later entry (2) connects.
        let (l, w, h) = grid(&["12", "21"]);
        let (_, r) = regions(&l, w, h);
        assert_eq!(r, [(1, 1), (2, 2), (1, 1)]);
        // A colour over the background connects.
        let (l, w, h) = grid(&["10", "01"]);
        let (_, r) = regions(&l, w, h);
        assert_eq!(r.iter().filter(|x| x.0 == 1).count(), 1);
        // Three labels: the diagonal pair connects.
        let (l, w, h) = grid(&["12", "31"]);
        let (_, r) = regions(&l, w, h);
        assert_eq!(r.iter().filter(|x| x.0 == 1).count(), 1);
    }

    #[test]
    fn small_regions_join_the_longest_colour_border() {
        let (mut l, w, h) = grid(&["111222", "113222", "111222", "000000"]);
        // The 1-pixel region 3 borders 1 on three sides, 2 on one.
        let joined = join_small(&mut l, w, h, 2.0);
        assert_eq!(joined, 1);
        assert_eq!(l[w + 2], 1);
        // A region with only background around joins the background.
        let (mut l, w, h) = grid(&["000", "020", "000"]);
        assert_eq!(join_small(&mut l, w, h, 2.0), 1);
        assert!(l.iter().all(|&x| x == 0));
    }
}
