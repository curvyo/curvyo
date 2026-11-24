//! Which dot pictures the canvas draws for the raster items of the active laser page
//! (`specs/0123` criteria 20, 37 d, e; ADR 0069 §5): the Toolpath view's from the job's full
//! sweeps, the Operations view's from a preview grid of `k·I` (at most one dot per CSS pixel and
//! 300,000 dots per operation), each picture rebuilt only when its sweeps change.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{AssetHash, DocumentVersion, NodeId, Opacity, Point, ViewTransform};
use curvyo_laser_core::job::{
    BlockInput, BlockNumbers, BlockSource, LeafGeometry, MoveKind, NumbersContext, NumbersRead,
    PlanCache, PointUm, Sweep, block_numbers, read_output_settings,
};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::Interval;

use super::super::Session;
use super::job_cache::PageJob;
use super::raster_picture::{RasterPicture, picture_of};

/// The pale opacity of a raster item the play bar's head has not reached (`0029` UX P7: 35 %).
const PALE: f64 = 0.35;

/// What the preview pictures were built from: equal keys give equal pictures.
#[derive(Debug, Clone, PartialEq)]
struct PreviewKey {
    version: DocumentVersion,
    manufacturing: Option<NodeId>,
    library: u64,
    /// The view's scale, whose preview step each item's Interval sets.
    scale_bits: u64,
    /// The Strength held on a picture while its slider is dragged (`specs/0135` criterion 26).
    held: Option<(NodeId, u8)>,
}

/// A raster item: its leaf's node and stacking place (the leaves of one clone share the node).
pub(super) type Item = (NodeId, u32);

/// The Toolpath view's pictures of one job generation, each with the job move its item starts on.
type HeldToolpath = (u64, Arc<Vec<(RasterPicture, usize)>>);

/// The dot pictures of the session and the preview plans they come from.
#[derive(Debug, Default)]
pub(super) struct RasterCache {
    toolpath: RefCell<Option<HeldToolpath>>,
    preview: RefCell<Option<(PreviewKey, Arc<Vec<RasterPicture>>)>>,
    /// The preview grid's plans, sharing the job's grey pyramids (made on first use).
    preview_plans: RefCell<Option<PlanCache>>,
    /// The Before and after pane's After picture and its reduced plans (`pane_picture.rs`).
    pub(super) pane: RefCell<Option<RasterPicture>>,
    pub(super) pane_plans: RefCell<Option<PlanCache>>,
    /// The working images of the pictures both views hold, by name.
    names: RefCell<HashMap<AssetHash, Arc<curvyo_image_core::WorkingImage>>>,
    serial: Cell<u64>,
    /// How often a picture was rendered, for the tests.
    #[cfg(test)]
    pub(super) rendered: Cell<usize>,
}

impl RasterCache {
    /// The picture of `item` from `sweeps`: one of `old` drawn from the same sweeps, else a new
    /// one under a fresh name.
    pub(super) fn picture(
        &self,
        item: Item,
        sweeps: Vec<Arc<Sweep>>,
        old: &HashMap<Item, &RasterPicture>,
    ) -> Option<RasterPicture> {
        if let Some(kept) = old.get(&item).filter(|p| p.drawn_from(&sweeps)) {
            return Some((*kept).clone());
        }
        let serial = self.serial.get() + 1;
        self.serial.set(serial);
        #[cfg(test)]
        self.rendered.set(self.rendered.get() + 1);
        picture_of(
            item,
            sweeps,
            AssetHash::of(format!("curvyo raster picture {serial}").as_bytes()),
        )
    }

    /// The working image of the picture named `asset`, if one of the views holds it.
    fn image(&self, asset: &AssetHash) -> Option<Arc<curvyo_image_core::WorkingImage>> {
        self.names.borrow().get(asset).cloned()
    }

    /// Indexes the pictures both views hold by name.
    fn index(&self) {
        let toolpath = self.toolpath.borrow();
        let preview = self.preview.borrow();
        *self.names.borrow_mut() = toolpath
            .iter()
            .flat_map(|(_, pictures)| pictures.iter().map(|(p, _)| p))
            .chain(preview.iter().flat_map(|(_, pictures)| pictures.iter()))
            .map(|p| (p.asset.clone(), Arc::clone(&p.image)))
            .collect();
    }
}

/// The pictures of `pictures` by item.
pub(super) fn by_item<'a>(
    pictures: impl Iterator<Item = &'a RasterPicture>,
) -> HashMap<Item, &'a RasterPicture> {
    pictures.map(|p| ((p.node, p.stacking), p)).collect()
}

/// The sweeps of each raster item of `job`, once (passes repeat them), with the move it starts on,
/// in run order.
fn job_items(job: &PageJob) -> Vec<(Item, Vec<Arc<Sweep>>, usize)> {
    let mut items: Vec<(Item, Vec<Arc<Sweep>>, usize)> = Vec::new();
    let mut at: HashMap<(usize, Item), usize> = HashMap::new();
    let mut seen = std::collections::HashSet::new();
    for (index, m) in job.built.job.moves.iter().enumerate() {
        let MoveKind::Sweep {
            node,
            stacking,
            sweep,
            ..
        } = m.kind
        else {
            continue;
        };
        if !seen.insert(sweep) {
            continue;
        }
        let Some(sweep) = usize::try_from(sweep)
            .ok()
            .and_then(|i| job.built.job.sweeps.get(i))
        else {
            continue;
        };
        let slot = *at.entry((m.block, (node, stacking))).or_insert_with(|| {
            items.push(((node, stacking), Vec::new(), index));
            items.len() - 1
        });
        items[slot].1.push(Arc::clone(sweep));
    }
    items
}

/// The smallest power of two `k` for which `k` times `interval` is at least one CSS pixel at
/// `view` (ADR 0069 §5, with `k` rounded up to a power of two so a zoom rebuilds rarely).
fn preview_step(interval: Interval, view: ViewTransform) -> i64 {
    let px_mm = 1.0 / view.scale();
    let mut k = 1_i64;
    while interval.mm() * f64::from(u32::try_from(k).unwrap_or(u32::MAX)) < px_mm && k < 1 << 16 {
        k *= 2;
    }
    k
}

/// The most dots the Operations view's preview of one operation's raster items has: about 10 ms
/// of dithering, so an edit's frame stays within 50 ms at any zoom (ADR 0069 §5, `specs/0123`
/// criterion 37 d).
const PREVIEW_DOTS: f64 = 300_000.0;

/// The area in mm² of the box of each raster leaf of `block`, summed.
pub(super) fn raster_area_mm2(block: &BlockInput) -> f64 {
    block
        .leaves
        .iter()
        .map(|leaf| {
            let points: Vec<Point> = match &leaf.geometry {
                LeafGeometry::Image { frame, .. } => frame.to_vec(),
                LeafGeometry::Gradient { outlines, .. } => outlines
                    .iter()
                    .flat_map(|o| o.anchors.iter().map(|&(p, _, _)| p))
                    .collect(),
                LeafGeometry::Outlines(_) => Vec::new(),
            };
            let (lo, hi) = points.iter().fold(
                (
                    Point::new(f64::INFINITY, f64::INFINITY),
                    Point::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
                ),
                |(lo, hi), p| {
                    (
                        Point::new(lo.x.min(p.x), lo.y.min(p.y)),
                        Point::new(hi.x.max(p.x), hi.y.max(p.y)),
                    )
                },
            );
            let area = (hi.x - lo.x) * (hi.y - lo.y);
            if area.is_finite() { area.max(0.0) } else { 0.0 }
        })
        .sum()
}

/// The smallest power of two `k` from `k` up for which the raster items of `area_mm2` have at
/// most `most` dots at `k` times `interval` ([`PREVIEW_DOTS`] for the Operations view).
pub(super) fn capped_step(mut k: i64, interval: Interval, area_mm2: f64, most: f64) -> i64 {
    let dots = |k: i64| {
        let step = interval.mm() * f64::from(u32::try_from(k).unwrap_or(u32::MAX));
        area_mm2 / (step * step)
    };
    while dots(k) > most && k < 1 << 16 {
        k *= 2;
    }
    k
}

impl Session {
    /// The page corner of the active laser page in document mm.
    fn laser_page_origin(&self) -> Option<Point> {
        self.active_manufacturing()?.page.map(|page| page.origin)
    }

    /// The Toolpath view's dot pictures as image objects in document mm: an item the play
    /// position has not reached is pale (`0029` play bar rule).
    pub(in crate::session) fn toolpath_raster_objects(
        &self,
        job: &PageJob,
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        let fraction = self.play_fraction();
        let head = (fraction < 1.0).then(|| job.timeline.head_at(&job.built.job, fraction).done);
        let pictures = self.toolpath_pictures(job);
        let origin = self.laser_page_origin().unwrap_or(Point::new(0.0, 0.0));
        pictures
            .iter()
            .map(|(picture, first)| {
                let pale = head.is_some_and(|done| done <= *first);
                let opacity =
                    Opacity::new(if pale { PALE } else { 1.0 }).unwrap_or(Opacity::OPAQUE);
                curvyo_document_core::ObjectSnapshot::Image(picture.object(origin, opacity))
            })
            .collect()
    }

    /// The Toolpath view's pictures of `job`, from the cache while it is the same job; a rebuilt
    /// job keeps the pictures whose sweeps it reused.
    fn toolpath_pictures(&self, job: &PageJob) -> Arc<Vec<(RasterPicture, usize)>> {
        let cache = &self.output.raster;
        if let Some((generation, pictures)) = cache.toolpath.borrow().as_ref()
            && *generation == job.generation
        {
            return Arc::clone(pictures);
        }
        let old = cache.toolpath.borrow_mut().take();
        let kept = by_item(
            old.iter()
                .flat_map(|(_, pictures)| pictures.iter().map(|(p, _)| p)),
        );
        let pictures: Vec<(RasterPicture, usize)> = job_items(job)
            .into_iter()
            .filter_map(|(item, sweeps, first)| {
                cache.picture(item, sweeps, &kept).map(|p| (p, first))
            })
            .collect();
        let pictures = Arc::new(pictures);
        *cache.toolpath.borrow_mut() = Some((job.generation, Arc::clone(&pictures)));
        cache.index();
        pictures
    }

    /// The Operations view's dot pictures as image objects in document mm, with the items they
    /// stand for (`specs/0123` criterion 20: always the dots, never the hatch).
    pub(in crate::session) fn preview_raster_objects(
        &self,
        view: ViewTransform,
    ) -> Vec<curvyo_document_core::ObjectSnapshot> {
        let origin = self.laser_page_origin().unwrap_or(Point::new(0.0, 0.0));
        self.preview_pictures(view)
            .iter()
            .map(|p| curvyo_document_core::ObjectSnapshot::Image(p.object(origin, Opacity::OPAQUE)))
            .collect()
    }

    /// The Operations view's pictures at `view`'s preview step, rebuilt when the page, the
    /// library or the step changed; each item's picture kept while its preview sweeps are the
    /// same.
    pub(in crate::session) fn preview_pictures(
        &self,
        view: ViewTransform,
    ) -> Arc<Vec<RasterPicture>> {
        let cache = &self.output.raster;
        // A zoom rebuilds the key only: the plans keep their sweeps while an item's step stays,
        // and the pictures theirs.
        let key = PreviewKey {
            version: self.document.version(),
            manufacturing: self.active_manufacturing().map(|m| m.id),
            library: self.materials.library_generation(),
            scale_bits: view.scale().to_bits(),
            held: self.held_adjust_strength(),
        };
        if let Some((held, pictures)) = cache.preview.borrow().as_ref()
            && *held == key
        {
            return Arc::clone(pictures);
        }
        let old = cache.preview.borrow_mut().take();
        let kept = by_item(old.iter().flat_map(|(_, p)| p.iter()));
        let pictures: Vec<RasterPicture> = self
            .preview_items(view)
            .into_iter()
            .filter_map(|(item, sweeps)| cache.picture(item, sweeps, &kept))
            .collect();
        let pictures = Arc::new(pictures);
        *cache.preview.borrow_mut() = Some((key, Arc::clone(&pictures)));
        cache.index();
        pictures
    }

    /// The preview sweeps of every raster item of the active laser page's operations, by item.
    fn preview_items(&self, view: ViewTransform) -> Vec<(Item, Vec<Arc<Sweep>>)> {
        let Some(page) = self.page_input() else {
            return Vec::new();
        };
        let input = &page.input;
        let Some(manufacturing) = self.active_manufacturing() else {
            return Vec::new();
        };
        let fixture_material = read_output_settings(&manufacturing).fixture_material;
        let context = NumbersContext {
            materials: self.materials().reads(),
            profile_id: &input.machine.profile_id,
            fixture_material: fixture_material.as_deref(),
            max_feed: input.machine.values.max_feed,
        };
        let mut held = self.output.raster.preview_plans.borrow_mut();
        let plans = held.get_or_insert_with(|| self.output.job.sharing_pyramids());
        let mut items: Vec<(Item, Vec<Arc<Sweep>>)> = Vec::new();
        let mut at: HashMap<Item, usize> = HashMap::new();
        let mut ids = Vec::new();
        for block in &input.blocks {
            let Some(raster) = raster_block(block) else {
                continue;
            };
            let read = block_numbers(&raster, context);
            let Some(mut numbers) =
                preview_numbers(&read).filter(|n| n.kind == OperationKind::Image)
            else {
                continue;
            };
            let k = capped_step(
                preview_step(numbers.interval, view),
                numbers.interval,
                raster_area_mm2(&raster),
                PREVIEW_DOTS,
            );
            let hundredths = i64::from(numbers.interval.hundredths()) * k;
            numbers.interval =
                Interval::from_hundredths(hundredths.min(i64::from(Interval::MAX_HUNDREDTHS)))
                    .unwrap_or(numbers.interval);
            let plan = plans.plan(&raster, &numbers, PointUm::default());
            ids.push(raster.id);
            for planned in plan.runs.iter().flat_map(|run| &run.sweeps) {
                let item = (planned.node, planned.stacking);
                let slot = *at.entry(item).or_insert_with(|| {
                    items.push((item, Vec::new()));
                    items.len() - 1
                });
                items[slot].1.push(Arc::clone(&planned.sweep));
            }
        }
        retain_preview_plans(plans, input, &ids);
        items
    }

    /// The working image of a dot picture named `asset` that a view of this session drew.
    pub(in crate::session) fn raster_picture_image(
        &self,
        asset: &AssetHash,
    ) -> Option<Arc<curvyo_image_core::WorkingImage>> {
        self.output.raster.image(asset)
    }
}

/// Keeps the preview plans of the Image blocks `ids` and the images and leaves `input` names.
pub(super) fn retain_preview_plans(
    plans: &mut PlanCache,
    input: &curvyo_laser_core::job::JobInput,
    ids: &[NodeId],
) {
    let assets: Vec<&AssetHash> = input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .filter_map(|leaf| match &leaf.geometry {
            LeafGeometry::Image { asset, .. } => Some(asset),
            _ => None,
        })
        .collect();
    let leaves: Vec<NodeId> = input
        .blocks
        .iter()
        .flat_map(|b| &b.leaves)
        .map(|leaf| leaf.node)
        .collect();
    plans.retain(ids, &assets, &leaves);
}

/// The numbers the preview draws a block's dots with: the block's own (an operation always has
/// numbers since `specs/0150`), `None` for damage.
pub(super) fn preview_numbers(read: &NumbersRead) -> Option<BlockNumbers> {
    read.numbers().copied()
}

/// `block` with only its raster leaves (images with pixels and gradient-filled leaves) and no
/// outline work, `None` when it has none or is no operation.
pub(super) fn raster_block(block: &BlockInput) -> Option<BlockInput> {
    if !matches!(block.source, BlockSource::Operation { .. }) {
        return None;
    }
    let leaves: Vec<_> = block
        .leaves
        .iter()
        .filter(|leaf| {
            matches!(
                leaf.geometry,
                LeafGeometry::Image {
                    pixels: Some(_),
                    ..
                } | LeafGeometry::Gradient { .. }
            )
        })
        .cloned()
        .collect();
    (!leaves.is_empty()).then(|| BlockInput {
        leaves,
        cut_out: Vec::new(),
        ..block.clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_step_is_the_smallest_power_of_two_of_at_least_one_pixel() {
        let interval = Interval::DEFAULT;
        let at =
            |scale: f64| preview_step(interval, ViewTransform::new(scale, Point::new(0.0, 0.0)));
        assert_eq!(at(20.0), 1, "0.1 mm is 2 px");
        assert_eq!(at(10.0), 1, "0.1 mm is 1 px");
        assert_eq!(at(4.0), 4, "0.25 mm per px: 3 rounds up to 4");
        assert_eq!(at(1.0), 16);
    }

    #[test]
    fn the_preview_step_is_capped_at_300_000_dots() {
        let interval = Interval::DEFAULT;
        assert_eq!(
            capped_step(1, interval, 50.0 * 50.0, PREVIEW_DOTS),
            1,
            "250,000 dots"
        );
        assert_eq!(
            capped_step(1, interval, 200.0 * 200.0, PREVIEW_DOTS),
            4,
            "4 million dots: 250,000 at 4"
        );
        assert_eq!(
            capped_step(8, interval, 200.0 * 200.0, PREVIEW_DOTS),
            8,
            "never finer than the view's"
        );
        assert_eq!(capped_step(1, interval, 0.0, PREVIEW_DOTS), 1);
    }
}
