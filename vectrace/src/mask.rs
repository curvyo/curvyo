//! The seam between the trace jobs and the area a masked trace keeps (`specs/0094` slice 1;
//! ADR 0048 §4): what a job asks of a mask, so the jobs need no boolean kernel.

use crate::{ColourTrace, TraceResult, node_count};
use kurbo::{BezPath, Point};

/// The area a masked trace keeps.
pub trait Mask {
    /// Whether the mask keeps `p` (mm); text lines are kept by their box centre.
    fn keeps(&self, p: Point) -> bool;

    /// `outlines` (closed, holes wound against their outer outline) clipped to the kept area.
    fn clip(&self, outlines: &[BezPath]) -> Clipped;
}

/// Outlines clipped by a [`Mask`].
#[derive(Debug, Clone, PartialEq)]
pub struct Clipped {
    /// The outlines that remain: those kept whole in their order, the clipped ones in the place
    /// of the first outline that crossed the edge.
    pub outlines: Vec<BezPath>,
    /// Whether anything was dropped or cut.
    pub changed: bool,
    /// How many crossing outlines the mask could not clip and kept whole.
    pub kept_whole: usize,
}

/// A traced result clipped in place (counts after the clip, `specs/0094` `adrs.md` decision 1);
/// the outlines kept whole after a kernel refusal.
pub(crate) fn clip_trace(mask: &dyn Mask, result: &mut TraceResult) -> usize {
    let clipped = mask.clip(&result.outlines);
    if clipped.changed {
        result.counts.paths = clipped.outlines.len();
        result.counts.nodes = clipped.outlines.iter().map(node_count).sum();
        result.covers_whole_image = false;
        result.outlines = clipped.outlines;
    }
    clipped.kept_whole
}

/// A multicolour result clipped in place, layer by layer; a layer left without outlines is
/// dropped. The outlines kept whole after a kernel refusal.
pub(crate) fn clip_colours(mask: &dyn Mask, result: &mut ColourTrace) -> usize {
    let mut changed = false;
    let mut kept_whole = 0;
    for layer in &mut result.layers {
        let clipped = mask.clip(&layer.outlines);
        kept_whole += clipped.kept_whole;
        if clipped.changed {
            layer.outlines = clipped.outlines;
            changed = true;
        }
    }
    if changed {
        result.layers.retain(|layer| !layer.outlines.is_empty());
        let all = || result.layers.iter().flat_map(|l| &l.outlines);
        result.counts.paths = all().count();
        result.counts.nodes = all().map(node_count).sum();
    }
    kept_whole
}

/// The number that opens the unclipped count after a result's outlines (`specs/0094`): present
/// only when a kernel refusal kept crossing outlines whole.
pub(crate) const UNCLIPPED_MARK: f64 = -5.0;

/// Appends the unclipped count to a result's numbers when there is one.
pub(crate) fn write_unclipped(wire: &mut Vec<f64>, count: usize) {
    if count > 0 {
        #[allow(clippy::cast_precision_loss)] // counts far below 2^52
        wire.extend([UNCLIPPED_MARK, count as f64]);
    }
}

/// The second implementation of [`Mask`] besides `TraceRegion`: a box mask for the job tests.
#[cfg(test)]
pub(crate) mod test_double {
    use super::{Clipped, Mask};
    use kurbo::{BezPath, Point};

    /// A mask that keeps what lies in a box (the second implementation of [`Mask`], besides
    /// `TraceRegion`): an outline whose control box lies in the box stays, one that meets its edge
    /// is kept whole and counted, the others go.
    pub(crate) struct BoxMask {
        pub(crate) min: Point,
        pub(crate) max: Point,
    }

    impl BoxMask {
        fn holds(&self, p: Point) -> bool {
            (self.min.x..=self.max.x).contains(&p.x) && (self.min.y..=self.max.y).contains(&p.y)
        }
    }

    impl Mask for BoxMask {
        fn keeps(&self, p: Point) -> bool {
            self.holds(p)
        }

        fn clip(&self, outlines: &[BezPath]) -> Clipped {
            let mut kept_whole = 0;
            let kept: Vec<BezPath> = outlines
                .iter()
                .filter(|path| {
                    let b = kurbo::Shape::bounding_box(*path);
                    let corners = [(b.x0, b.y0), (b.x1, b.y0), (b.x1, b.y1), (b.x0, b.y1)]
                        .map(|(x, y)| self.holds(Point::new(x, y)));
                    let all = corners.iter().all(|&c| c);
                    kept_whole += usize::from(!all && corners.iter().any(|&c| c));
                    corners.iter().any(|&c| c)
                })
                .cloned()
                .collect();
            Clipped {
                changed: kept.len() != outlines.len(),
                outlines: kept,
                kept_whole,
            }
        }
    }
}
