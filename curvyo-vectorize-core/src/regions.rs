//! The regions of the foreground mask (ADR 0023 §3 steps 2 and 3): one scanline union-find
//! labelling with `u32` labels, 4-connected for foreground and background alike.

use crate::TraceError;
use crate::progress::Poller;

/// One 4-connected region of equal mask value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Region {
    /// Foreground (`true`) or background.
    pub foreground: bool,
    /// Its pixel count.
    pub pixels: u64,
    /// The index of its first pixel in scan order.
    pub first: usize,
    /// Its box: the leftmost column, top row, and one past the rightmost column and bottom row.
    pub left: usize,
    pub top: usize,
    pub right: usize,
    pub bottom: usize,
    /// Whether it touches the picture's border.
    pub touches_border: bool,
}

/// The labels of a mask: the region of every pixel, numbered by first scan position.
pub(crate) struct Labels {
    pub width: usize,
    /// The region index of every pixel.
    pub ids: Vec<u32>,
    pub regions: Vec<Region>,
}

/// A root not numbered yet.
const UNSET: u32 = u32::MAX;

/// The root of `label`, halving the path on the way.
fn find(parent: &mut [u32], mut label: u32) -> u32 {
    while parent[label as usize] != label {
        let up = parent[parent[label as usize] as usize];
        parent[label as usize] = up;
        label = up;
    }
    label
}

/// Joins the sets of `a` and `b`; the smaller root wins, so the result is order independent.
fn union(parent: &mut [u32], a: u32, b: u32) -> u32 {
    let (ra, rb) = (find(parent, a), find(parent, b));
    let (keep, drop) = if ra <= rb { (ra, rb) } else { (rb, ra) };
    parent[drop as usize] = keep;
    keep
}

/// Labels both values of `mask` (`width` x `height`) 4-connected, polling every 64 rows of each
/// of the two passes.
pub(crate) fn label(
    mask: &[bool],
    width: usize,
    height: usize,
    poller: &mut Poller<'_>,
) -> Result<Labels, TraceError> {
    let mut ids = vec![0_u32; mask.len()];
    let mut parent: Vec<u32> = Vec::new();
    for y in 0..height {
        poller.row(y, height * 2)?;
        let row = y * width;
        for x in 0..width {
            let i = row + x;
            let value = mask[i];
            let up = (y > 0 && mask[i - width] == value).then(|| ids[i - width]);
            let left = (x > 0 && mask[i - 1] == value).then(|| ids[i - 1]);
            ids[i] = match (up, left) {
                (Some(u), Some(l)) if u == l => u,
                (Some(u), Some(l)) => union(&mut parent, u, l),
                (Some(only), None) | (None, Some(only)) => only,
                (None, None) => {
                    let fresh = u32::try_from(parent.len()).map_err(|_| TraceError::TooDetailed)?;
                    parent.push(fresh);
                    fresh
                }
            };
        }
    }
    // Second pass: roots to compact numbers in order of first appearance, with the statistics.
    let mut compact = vec![UNSET; parent.len()];
    let mut regions: Vec<Region> = Vec::new();
    for y in 0..height {
        poller.row(height + y, height * 2)?;
        let row = y * width;
        for x in 0..width {
            let i = row + x;
            let root = find(&mut parent, ids[i]) as usize;
            if compact[root] == UNSET {
                compact[root] =
                    u32::try_from(regions.len()).map_err(|_| TraceError::TooDetailed)?;
                regions.push(Region {
                    foreground: mask[i],
                    pixels: 0,
                    first: i,
                    left: x,
                    top: y,
                    right: x + 1,
                    bottom: y + 1,
                    touches_border: false,
                });
            }
            let id = compact[root];
            ids[i] = id;
            let region = &mut regions[id as usize];
            region.pixels += 1;
            region.left = region.left.min(x);
            region.right = region.right.max(x + 1);
            region.bottom = y + 1;
            if x == 0 || y == 0 || x + 1 == width || y + 1 == height {
                region.touches_border = true;
            }
        }
    }
    Ok(Labels {
        width,
        ids,
        regions,
    })
}

impl Labels {
    /// Sets every pixel of the regions `pick` accepts to `value` in `mask`; returns how many
    /// regions that was.
    pub(crate) fn flip(
        &self,
        mask: &mut [bool],
        pick: impl Fn(&Region) -> bool,
        value: bool,
    ) -> u64 {
        let chosen: Vec<bool> = self.regions.iter().map(&pick).collect();
        for (pixel, &id) in mask.iter_mut().zip(&self.ids) {
            if chosen[id as usize] {
                *pixel = value;
            }
        }
        chosen.iter().filter(|&&c| c).count() as u64
    }

    /// The foreground region each background region is a hole of, or `None` for the background
    /// that reaches the border (and for foreground regions): the region above the hole's first
    /// pixel (its first pixel's upper neighbour is never background of the same hole).
    pub(crate) fn hole_owners(&self) -> Vec<Option<u32>> {
        self.regions
            .iter()
            .map(|region| {
                if region.foreground || region.touches_border {
                    return None;
                }
                let above = self.ids[region.first - self.width];
                self.regions[above as usize].foreground.then_some(above)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::ops::ControlFlow;

    use super::*;

    fn mask(rows: &[&str]) -> (Vec<bool>, usize, usize) {
        let width = rows[0].len();
        let bits = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        (bits, width, rows.len())
    }

    fn labels(rows: &[&str]) -> Labels {
        let (bits, w, h) = mask(rows);
        let mut go = |_| ControlFlow::Continue(());
        label(&bits, w, h, &mut Poller::new(&mut go)).unwrap()
    }

    #[test]
    fn regions_are_four_connected_and_numbered_in_scan_order() {
        let l = labels(&["#..#", ".#..", "....", "##.."]);
        let fg: Vec<_> = l.regions.iter().filter(|r| r.foreground).collect();
        // Diagonal neighbours are separate regions.
        assert_eq!(fg.len(), 4);
        assert_eq!(l.regions[0].first, 0);
        assert!(l.regions.windows(2).all(|w| w[0].first < w[1].first));
        let bottom = fg[3];
        assert_eq!(
            (bottom.left, bottom.top, bottom.right, bottom.bottom),
            (0, 3, 2, 4)
        );
        assert_eq!(bottom.pixels, 2);
    }

    #[test]
    fn a_u_shape_is_one_region() {
        let l = labels(&["#.#", "#.#", "###"]);
        assert_eq!(l.regions.iter().filter(|r| r.foreground).count(), 1);
        assert_eq!(l.regions[0].pixels, 7);
    }

    #[test]
    fn a_hole_belongs_to_the_region_above_it() {
        let l = labels(&[".....", ".###.", ".#.#.", ".###.", "....."]);
        let owners = l.hole_owners();
        let hole = l
            .regions
            .iter()
            .position(|r| !r.foreground && !r.touches_border)
            .unwrap();
        let ring = l.regions.iter().position(|r| r.foreground).unwrap();
        assert_eq!(owners[hole], Some(u32::try_from(ring).unwrap()));
        assert_eq!(owners.iter().flatten().count(), 1);
    }

    #[test]
    fn a_stop_between_rows_cancels() {
        let (bits, w, h) = mask(&["#"; 200]);
        let mut calls = 0;
        let mut stop = |_| {
            calls += 1;
            if calls > 1 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        };
        let result = label(&bits, w, h, &mut Poller::new(&mut stop));
        assert!(matches!(result, Err(TraceError::Cancelled)));
    }
}
