//! The sweeps of the raster items, built a few rows at a time and kept per item between builds
//! (ADR 0069 §12; `specs/0123` criteria 12, 37 d).

use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{NodeId, PlacedGradient, Point};

use crate::job_input::PointUm;
use crate::job_raster_dither::Ditherer;
use crate::job_raster_grid::{DotGrid, estimated_dots};
use crate::job_raster_plan::{MAX_DOTS, RasterSettings, RasterSource, row_levels, screen_anchor};
use crate::job_raster_rows::{Sweep, row_sweeps};
use crate::job_raster_sample::{GreyPyramid, ImageSampler};
use crate::job_raster_tone::ToneTable;
use crate::operation_image_options::Dither;

/// Where a raster item's greys come from, owned so its work can be resumed later.
#[derive(Debug, Clone)]
pub(crate) enum HeldSource {
    /// An image through its grey pyramid, placed in `frame` for dots of `interval_mm`.
    Image {
        pyramid: Arc<GreyPyramid>,
        frame: [Point; 4],
        interval_mm: f64,
    },
    /// A gradient fill placed on the page.
    Gradient(PlacedGradient),
}

/// The rows of one raster item in work: the grid, the tone and the dither state, and the sweeps
/// of the rows done so far. Advancing it by any number of rows at a time gives the same sweeps.
#[derive(Debug)]
pub(crate) struct ItemWork {
    grid: DotGrid,
    tone: ToneTable,
    ditherer: Ditherer,
    grey: bool,
    bidirectional: bool,
    next: usize,
    written: usize,
    greys: Vec<u8>,
    levels: Vec<u8>,
    sweeps: Vec<Sweep>,
}

impl ItemWork {
    /// The work of the item bounded by `rings`, `None` over [`MAX_DOTS`] (it blocks) or with no
    /// dot inside: such an item has no sweep.
    pub(crate) fn new(rings: &[Vec<PointUm>], settings: RasterSettings) -> Option<Self> {
        let RasterSettings {
            image,
            interval_um,
            angle_deg,
            bidirectional,
        } = settings;
        let image = &image;
        if estimated_dots(rings, interval_um, angle_deg) > MAX_DOTS {
            return None;
        }
        let grid = DotGrid::new(rings, interval_um, angle_deg)?;
        let ditherer = Ditherer::new(image, 0, grid.columns).screened(
            image,
            interval_um,
            screen_anchor(rings),
        );
        Some(Self {
            grid,
            tone: ToneTable::new(image),
            ditherer,
            grey: image.dither == Dither::Grey,
            bidirectional,
            next: 0,
            written: 0,
            greys: Vec::new(),
            levels: Vec::new(),
            sweeps: Vec::new(),
        })
    }

    /// Works through rows with the greys of `source` until `dots` is spent (a row is never
    /// split) or the item is done; `dots` is lessened by the dots worked. Whether it is done.
    pub(crate) fn advance(&mut self, source: &RasterSource<'_>, dots: &mut u64) -> bool {
        while let Some(row) = self.grid.rows.get(self.next) {
            if *dots == 0 {
                return false;
            }
            self.levels.clear();
            let worked = row_levels(
                &self.grid,
                row,
                source,
                &self.tone,
                &mut self.ditherer,
                &mut self.greys,
                &mut self.levels,
            );
            let before = self.sweeps.len();
            let reverse = self.bidirectional && self.written % 2 == 1;
            row_sweeps(
                &self.grid,
                row,
                &self.levels,
                reverse,
                self.grey,
                &mut self.sweeps,
            );
            if self.sweeps.len() > before {
                self.written += 1;
            }
            self.next += 1;
            *dots = dots.saturating_sub(worked);
        }
        true
    }

    /// The sweeps of the rows done, in run order.
    pub(crate) fn into_sweeps(self) -> Vec<Sweep> {
        self.sweeps
    }
}

/// The finished sweeps of one item, shared with the plans that hold them.
type Sweeps = Arc<Vec<Arc<Sweep>>>;

/// What starting an item's work costs, in dots of the budget: its grid, tone table and
/// ditherer take about as long as 1,000 dots, so a page of many small items keeps its ticks.
const ITEM_DOTS: u64 = 1_000;

/// One item in work: its node, its key and its work (`None`: an item with no sweep).
type InWork = (NodeId, u64, Option<ItemWork>);

/// The finished sweeps of each raster item with the key they were built for, and the one item in
/// work. Lookups only: nothing iterates the map into output.
#[derive(Debug, Default)]
pub(crate) struct SweepCache {
    done: HashMap<NodeId, (u64, Sweeps)>,
    work: Option<InWork>,
}

impl SweepCache {
    /// The finished sweeps of item `node` built for `key`.
    pub(crate) fn get(&self, node: NodeId, key: u64) -> Option<Sweeps> {
        self.done
            .get(&node)
            .filter(|(held, _)| *held == key)
            .map(|(_, sweeps)| Arc::clone(sweeps))
    }

    /// Advances item `node` with key `key` (bounded by `rings`, greys from `source()`) by about
    /// `dots` dots, resuming its work when it is the item in work; whether it is finished.
    pub(crate) fn advance(
        &mut self,
        (node, key): (NodeId, u64),
        rings: &[Vec<PointUm>],
        settings: RasterSettings,
        source: impl FnOnce() -> HeldSource,
        dots: &mut u64,
    ) -> bool {
        if self.get(node, key).is_some() {
            return true;
        }
        let mut work = match self.work.take() {
            Some((held, held_key, work)) if held == node && held_key == key => work,
            _ => {
                *dots = dots.saturating_sub(ITEM_DOTS);
                ItemWork::new(rings, settings)
            }
        };
        let finished = work
            .as_mut()
            .is_none_or(|work| with_source(&source(), |s| work.advance(s, dots)));
        if finished {
            let sweeps = work
                .map(ItemWork::into_sweeps)
                .unwrap_or_default()
                .into_iter()
                .map(Arc::new)
                .collect();
            self.done.insert(node, (key, Arc::new(sweeps)));
        } else {
            self.work = Some((node, key, work));
        }
        finished
    }

    /// Drops the sweeps of the items not in `nodes`.
    pub(crate) fn retain(&mut self, nodes: &[NodeId]) {
        let nodes: std::collections::HashSet<&NodeId> = nodes.iter().collect();
        self.done.retain(|node, _| nodes.contains(node));
    }
}

/// Runs `f` with the source of `held`; an image frame that gives no sampler has no dots, so its
/// work counts as done.
fn with_source(held: &HeldSource, f: impl FnOnce(&RasterSource<'_>) -> bool) -> bool {
    match held {
        HeldSource::Image {
            pyramid,
            frame,
            interval_mm,
        } => ImageSampler::new(pyramid, frame, *interval_mm)
            .is_none_or(|sampler| f(&RasterSource::Image(sampler))),
        HeldSource::Gradient(gradient) => f(&RasterSource::Gradient(gradient)),
    }
}
