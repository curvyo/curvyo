//! Cuts every region out of the labels and walks its boundary chains (`specs/0089` adrs.md §5):
//! the outer boundary with the region's holes filled, each hole's boundary reversed.

use kurbo::Point;

use crate::contour::outer_loop;
use crate::outline::Chain;
use crate::progress::{Phase, Poller};
use crate::regions::Labels;
use crate::trace::{MAX_HOLES_PER_REGION, TraceError};

/// The crack chains of every foreground region in label order: its outer boundary (holes
/// filled) and each hole's boundary, reversed; with their corners for `corner_cos`.
pub(crate) fn chains(
    labels: &Labels,
    corner_cos: f64,
    unit: usize,
    poller: &mut Poller<'_>,
) -> Result<Vec<(Chain, Vec<Chain>)>, TraceError> {
    let owners = labels.hole_owners();
    let mut holes: Vec<Vec<u32>> = vec![Vec::new(); labels.regions.len()];
    for (hole, owner) in owners.iter().enumerate() {
        if let Some(owner) = owner {
            // invariant: region indices fit `u32` (the labelling numbers them so).
            holes[*owner as usize].push(u32::try_from(hole).unwrap_or(u32::MAX));
        }
    }
    if holes.iter().any(|h| h.len() >= MAX_HOLES_PER_REGION) {
        return Err(TraceError::TooDetailed);
    }
    let total = labels.regions.iter().filter(|r| r.foreground).count();
    let mut out = Vec::with_capacity(total);
    for (index, region) in labels.regions.iter().enumerate() {
        if !region.foreground {
            continue;
        }
        poller.check(Phase::Tracing, out.len(), total)?;
        let id = u32::try_from(index).unwrap_or(u32::MAX);
        let own = &holes[index];
        let outer = walk(&outer_crop(labels, id, own, &owners), false);
        let inner = own
            .iter()
            .map(|&hole| walk(&hole_crop(labels, hole), true))
            .collect();
        out.push((outer, inner));
    }
    poller.check(Phase::Tracing, total, total)?;
    Ok(out
        .into_iter()
        .map(|(o, h): (Vec<Point>, Vec<Vec<Point>>)| {
            (
                Chain::new(o, corner_cos, unit),
                h.into_iter()
                    .map(|v| Chain::new(v, corner_cos, unit))
                    .collect(),
            )
        })
        .collect())
}

/// The boundary chain of `crop` in picture pixels, reversed for a hole.
fn walk(crop: &Crop, hole: bool) -> Vec<Point> {
    let mut points: Vec<Point> = outer_loop(&crop.shape, crop.width, crop.height, !hole)
        .into_iter()
        .map(|(x, y)| {
            #[allow(clippy::cast_precision_loss)] // picture coordinates are at most 16384
            let offset = (crop.left as f64, crop.top as f64);
            Point::new(f64::from(x) + offset.0, f64::from(y) + offset.1)
        })
        .collect();
    if hole {
        points.reverse();
    }
    points
}

/// A crop of the labels: the pixels of one boundary in the box at `(left, top)`.
struct Crop {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
    shape: Vec<bool>,
}

/// The region `id` with its holes filled, cropped to their common box.
fn outer_crop(labels: &Labels, id: u32, holes: &[u32], owners: &[Option<u32>]) -> Crop {
    let region = &labels.regions[id as usize];
    let (mut left, mut top, mut right, mut bottom) =
        (region.left, region.top, region.right, region.bottom);
    for &hole in holes {
        let h = &labels.regions[hole as usize];
        left = left.min(h.left);
        top = top.min(h.top);
        right = right.max(h.right);
        bottom = bottom.max(h.bottom);
    }
    crop(labels, (left, top, right, bottom), |label| {
        label == id || owners[label as usize] == Some(id)
    })
}

/// The hole `id` cropped to its box.
fn hole_crop(labels: &Labels, id: u32) -> Crop {
    let h = &labels.regions[id as usize];
    crop(labels, (h.left, h.top, h.right, h.bottom), |label| {
        label == id
    })
}

/// The pixels in the box `(left, top, right, bottom)` whose label `inside` accepts, with every
/// pocket they enclose filled, so the walk of the outer boundary never enters a pocket that
/// touches the outside only at a corner (its own outline is a hole's).
fn crop(
    labels: &Labels,
    (left, top, right, bottom): (usize, usize, usize, usize),
    inside: impl Fn(u32) -> bool,
) -> Crop {
    let (width, height) = (right - left, bottom - top);
    let mut shape = Vec::with_capacity(width * height);
    for y in top..bottom {
        let row = &labels.ids[y * labels.width + left..y * labels.width + right];
        shape.extend(row.iter().map(|&label| inside(label)));
    }
    fill_pockets(&mut shape, width, height);
    Crop {
        left,
        top,
        width,
        height,
        shape,
    }
}

/// Sets every empty pixel of `shape` (`width` x `height`) that is not 4-connected to the edge.
fn fill_pockets(shape: &mut [bool], width: usize, height: usize) {
    let mut outside = vec![false; shape.len()];
    let mut stack: Vec<usize> = (0..shape.len())
        .filter(|&i| {
            let (x, y) = (i % width, i / width);
            !shape[i] && (x == 0 || y == 0 || x + 1 == width || y + 1 == height)
        })
        .collect();
    for &i in &stack {
        outside[i] = true;
    }
    while let Some(i) = stack.pop() {
        let (x, y) = (i % width, i / width);
        let neighbours = [
            (x > 0).then(|| i - 1),
            (x + 1 < width).then(|| i + 1),
            (y > 0).then(|| i - width),
            (y + 1 < height).then(|| i + width),
        ];
        for j in neighbours.into_iter().flatten() {
            if !shape[j] && !outside[j] {
                outside[j] = true;
                stack.push(j);
            }
        }
    }
    for (pixel, out) in shape.iter_mut().zip(outside) {
        *pixel |= !out;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pockets_are_filled_and_the_outside_stays() {
        let rows = ["#####.", "#...#.", "#.#.#.", "#...#.", "##.###"];
        let mut shape: Vec<bool> = rows
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        fill_pockets(&mut shape, 6, 5);
        let text: Vec<String> = shape
            .chunks(6)
            .map(|row| row.iter().map(|&s| if s { '#' } else { '.' }).collect())
            .collect();
        // The ring's inside reaches the edge through the gap in the bottom row: not a pocket.
        assert_eq!(text, ["#####.", "#...#.", "#.#.#.", "#...#.", "##.###"]);
        let closed = ["#####", "#...#", "#.#.#", "#...#", "#####"];
        let mut shape: Vec<bool> = closed
            .iter()
            .flat_map(|r| r.chars().map(|c| c == '#'))
            .collect();
        fill_pockets(&mut shape, 5, 5);
        assert!(shape.iter().all(|&s| s));
    }
}
