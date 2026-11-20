//! The input of a Levels row (`specs/0135` criteria 5, 16): the histogram of the picture after
//! the switched-on rows above it, at a reduced mip, kept by content.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{AssetHash, NodeId};
use curvyo_image_core::{Adjusted, GreyPlane, Histogram, WorkingImage, run_steps};
use curvyo_laser_core::adjust::{AdjustRegisters, ManualStep, pixel_steps, stretch_points};
use curvyo_ui_core::stretch_levels;

/// The most pixels the histogram is read from (Proposal): enough for 256 bins, a quarter of the
/// canvas look's level, so a field edit above a Levels row stays inside a frame.
const HISTOGRAM_PIXELS: u64 = 1 << 18;

/// The histograms kept at most before the memo is cleared (a field drag adds one per value).
const KEPT: usize = 64;

/// What a Levels input is made from: the picture, the rows that run above it and its frame (the
/// bits of its four page corners).
type InputKey = (AssetHash, Vec<ManualStep>, [(u64, u64); 4]);

/// The Levels inputs of the session, never saved.
#[derive(Debug, Default)]
pub(in crate::session) struct LevelsInputs {
    histograms: RefCell<HashMap<InputKey, Arc<Histogram>>>,
}

/// `plane` as an opaque grey working image.
fn grey_image(plane: &GreyPlane) -> WorkingImage {
    WorkingImage {
        width: plane.width,
        height: plane.height,
        pixels: plane.grey.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        original_size: (plane.width, plane.height),
        truncated: false,
        animated: false,
    }
}

/// The 256 bars of `histogram` drawn, 0 to 255 of the tallest bin.
pub(super) fn bars(histogram: &Histogram) -> Vec<u8> {
    let tallest = u64::from(histogram.bins.iter().copied().max().unwrap_or(0).max(1));
    histogram
        .bins
        .iter()
        .map(|&bin| {
            // invariant: `bin <= tallest`, so the share is at most 255.
            u8::try_from((u64::from(bin) * 255 + tallest / 2) / tallest).unwrap_or(u8::MAX)
        })
        .collect()
}

impl crate::session::Session {
    /// The histogram of the input of row `index` of picture `target`: the picture (alpha over
    /// white, grey) after the switched-on rows above it that read, whether or not a preset runs
    /// instead of them; `None` when its bytes cannot be read.
    pub(super) fn levels_input(&self, target: NodeId, index: usize) -> Option<Arc<Histogram>> {
        let leaf = self.picture_leaf(target)?;
        let registers = AdjustRegisters::read(&leaf.adjustments);
        let above: Vec<ManualStep> = registers
            .steps
            .get(..index)?
            .iter()
            .filter(|s| s.enabled)
            .filter_map(|s| s.step)
            .collect();
        let corners = leaf.corners();
        let frame = corners.map(|p| (p.x.to_bits(), p.y.to_bits()));
        let key = (leaf.asset.clone(), above, frame);
        let memo = &self.image_adjust.levels.histograms;
        if let Some(kept) = memo.borrow().get(&key) {
            return Some(Arc::clone(kept));
        }
        let pixels = self.job_pixels(&leaf.asset)?;
        let plane = GreyPlane::of_rgba(&pixels);
        let mip = plane.mip_at_most(HISTOGRAM_PIXELS);
        let steps = pixel_steps(&key.1, &corners, mip.width, mip.height);
        let histogram = Arc::new(match run_steps(&grey_image(&mip), &steps) {
            Adjusted::Unchanged => Histogram::of(&mip),
            Adjusted::Grey(out) => Histogram::of(&out),
            Adjusted::Rgba(out) => Histogram::of(&GreyPlane::of_rgba(&out)),
        });
        let mut memo = memo.borrow_mut();
        if memo.len() >= KEPT {
            memo.clear();
        }
        memo.insert(key, Arc::clone(&histogram));
        Some(histogram)
    }

    /// The Stretch button of Levels row `index` (criterion 5): Black and White at the 0.5 % and
    /// 99.5 % percentiles of its input, one step; `false` when nothing changed.
    pub fn stretch_adjust_levels(&mut self, index: usize) -> bool {
        let Some(target) = self.adjust_target() else {
            return false;
        };
        let Some(histogram) = self.levels_input(target, index) else {
            return false;
        };
        self.shown_steps()
            .and_then(|steps| stretch_levels(&steps, index, stretch_points(&histogram)))
            .is_some_and(|next| self.write_steps(next))
    }
}
