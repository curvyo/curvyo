//! The shape properties the analysis reads from a content mask of the reduced copy.

use std::ops::ControlFlow;

use super::{real, real64};
use crate::progress::Poller;
use crate::regions::{Region, label};

/// A run of boundary edges along a row or a column at least this long (reduced px) counts as
/// straight.
const STRAIGHT_RUN: usize = 8;

/// Regions below this many reduced pixels are specks and not counted.
const SPECK: u64 = 4;

/// A row of at least this many letter-like components with equal heights and bottoms is a
/// text-like row.
const TEXT_ROW: usize = 4;

/// The shape properties of a mask.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Shapes {
    /// The mean stroke width, `2 · area / boundary length`, mask px.
    pub stroke: f64,
    /// The content regions and the holes in them (specks left out).
    pub components: usize,
    pub holes: usize,
    /// The median content region's area, mask px.
    pub component_area: f64,
    /// The share of the boundary on straight row or column runs.
    pub straight: f64,
    /// Rows of letter-like components.
    pub text_rows: usize,
}

/// The shape properties of `mask` (`width` x `height`).
pub(crate) fn shapes(mask: &[bool], width: usize, height: usize) -> Shapes {
    let area = mask.iter().filter(|&&m| m).count();
    if area == 0 || width == 0 || height == 0 {
        return Shapes::default();
    }
    let (edges, straight_edges) = boundary(mask, width, height);
    let mut go = |_| ControlFlow::Continue(());
    let Ok(labels) = label(mask, width, height, &mut Poller::new(&mut go)) else {
        return Shapes::default();
    };
    let content: Vec<&Region> = labels
        .regions
        .iter()
        .filter(|r| r.foreground && r.pixels >= SPECK)
        .collect();
    let holes = labels
        .regions
        .iter()
        .filter(|r| !r.foreground && !r.touches_border && r.pixels >= SPECK)
        .count();
    let mut areas: Vec<u64> = content.iter().map(|r| r.pixels).collect();
    areas.sort_unstable();
    Shapes {
        stroke: 2.0 * real(area) / real(edges.max(1)),
        components: content.len(),
        holes,
        component_area: areas.get(areas.len() / 2).map_or(0.0, |&a| real64(a)),
        straight: real(straight_edges) / real(edges.max(1)),
        text_rows: text_rows(&content, height),
    }
}

/// The boundary edges of `mask` (between content and the rest or the border) and how many of
/// them lie on straight runs of at least [`STRAIGHT_RUN`].
fn boundary(mask: &[bool], width: usize, height: usize) -> (usize, usize) {
    let at = |x: usize, y: usize| x < width && y < height && mask[y * width + x];
    // Edges between rows y − 1 and y, then between columns x − 1 and x.
    let rows = runs(height + 1, width, |y, x| {
        side(y > 0 && at(x, y - 1), at(x, y))
    });
    let columns = runs(width + 1, height, |x, y| {
        side(x > 0 && at(x - 1, y), at(x, y))
    });
    (rows.0 + columns.0, rows.1 + columns.1)
}

/// Which side of an edge holds the content: 0 for no edge, 1 before it, 2 after it.
fn side(before: bool, after: bool) -> u8 {
    match (before, after) {
        (true, false) => 1,
        (false, true) => 2,
        _ => 0,
    }
}

/// The edges of `lines` lines of `length` edges each (`edge(line, i)` their sides) and how
/// many lie on runs of one side of at least [`STRAIGHT_RUN`].
fn runs(lines: usize, length: usize, edge: impl Fn(usize, usize) -> u8) -> (usize, usize) {
    let (mut edges, mut straight) = (0, 0);
    for line in 0..lines {
        let mut run = (0, 0_u8);
        for i in 0..=length {
            let side = if i == length { 0 } else { edge(line, i) };
            edges += usize::from(side != 0);
            if side == run.1 && side != 0 {
                run.0 += 1;
            } else {
                if run.0 >= STRAIGHT_RUN {
                    straight += run.0;
                }
                run = (usize::from(side != 0), side);
            }
        }
    }
    (edges, straight)
}

/// Rows of at least [`TEXT_ROW`] letter-like components: 5 px to a sixth of the picture high,
/// at most 1.5 times as wide as high, filling 15 to 90 % of their box, with heights within a
/// third and bottoms within a fifth of each other and gaps under 1.2 heights; each component
/// counts in one row.
fn text_rows(content: &[&Region], height: usize) -> usize {
    let mut letters: Vec<(usize, usize, usize, usize)> = content
        .iter()
        .filter_map(|r| {
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            let fill = real64(r.pixels) / real(w * h);
            (h >= 5 && h * 6 <= height && w * 2 <= h * 3 && (0.15..=0.9).contains(&fill))
                .then_some((r.left, r.right, r.bottom, h))
        })
        .collect();
    letters.sort_unstable();
    let mut used = vec![false; letters.len()];
    let mut rows = 0;
    for i in 0..letters.len() {
        if used[i] {
            continue;
        }
        let mut row = vec![i];
        let mut last = letters[i];
        for j in i + 1..letters.len() {
            let c = letters[j];
            let h = last.3.max(c.3);
            if c.0 > last.1 + 2 * h {
                break;
            }
            let similar = last.3.abs_diff(c.3) * 3 <= h
                && last.2.abs_diff(c.2) * 5 <= h
                && c.0 <= last.1 + h + h / 5;
            if !used[j] && similar {
                row.push(j);
                last = c;
            }
        }
        if row.len() >= TEXT_ROW {
            rows += 1;
            for k in row {
                used[k] = true;
            }
        }
    }
    rows
}
