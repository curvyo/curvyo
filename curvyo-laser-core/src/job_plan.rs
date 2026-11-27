//! The ordered burn paths of one block, by kind (ADR 0064 §4, §5, ADR 0069 §2; criteria 13 to
//! 18, 22): what each leaf runs, flattened at 0.0095 mm and rounded to µm, hatched, rastered and
//! ordered.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use curvyo_document_core::{AssetHash, NodeId, Tolerance};
use curvyo_geometry_core::Outline;

use crate::job_fingerprint::plan_fingerprint;
pub use crate::job_flatten::FLATTEN_MM;
use crate::job_flatten::{dedupe, flat};
use crate::job_hatch::hatch;
use crate::job_input::{BlockInput, LeafGeometry, LeafOutline, PointUm};
use crate::job_numbers::BlockNumbers;
use crate::job_order::{Unit, UnitShape, order_all};
pub use crate::job_plan_cuts::{CutMark, CutStart};
use crate::job_plan_cuts::{Cuts, cut_out_units, leaf_damaged, unit_cut};
use crate::job_raster_items::{
    RasterCaches, RasterLeaf, frame_outline, prepare_raster, raster_leaf, raster_sweeps,
};
use crate::job_raster_rows::Sweep;
use crate::job_raster_work::SweepCache;
use crate::operation_cut_out::cut_out_contours;
use crate::operation_kind::OperationKind;
use crate::operation_kind_run::{LeafShape, Lines, kind_run};
use crate::operation_settings::Role;

/// One burn path of a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPath {
    /// Its points in run order; a closed path ends on its first point.
    pub points: Vec<PointUm>,
    /// Whether it is a hatch span (gets the run-out of Overscanning).
    pub hatch: bool,
    /// The leaf (or Cut-out item) it belongs to.
    pub node: NodeId,
}

/// One sweep of a raster item in a plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedSweep {
    /// The image or gradient-filled leaf it belongs to.
    pub node: NodeId,
    /// That leaf's stacking place in the block ([`crate::job::LeafInput::stacking`]): leaves of
    /// one clone share their node.
    pub stacking: u32,
    /// The sweep.
    pub sweep: Arc<Sweep>,
}

/// The paths one set of a block runs, once per pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRun {
    /// The set the paths run with.
    pub role: Role,
    /// The paths in run order.
    pub paths: Vec<PlannedPath>,
    /// The raster items' sweeps, after the paths: items in stacking order, bottom first
    /// (`specs/0123` criterion 14).
    pub sweeps: Vec<PlannedSweep>,
}

/// The plan of one block.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    /// The runs in order (Engrave + Outline: the hatch, then the outlines).
    pub runs: Vec<PlannedRun>,
    /// Where the head stands after the block.
    pub exit: PointUm,
    /// The Cut-out items that gave no closed contour (they run nothing; a warn).
    pub no_contour: Vec<NodeId>,
    /// The marks of every cut outline that takes the cut path step (`specs/0139` criteria 7,
    /// 14, 15, 19).
    pub marks: Vec<CutMark>,
    /// The leaf (or item) of each outline its tabs cover whole (criterion 10), once per outline.
    pub covered: Vec<NodeId>,
    /// The leaves (or items) whose own tabs are damaged: they run nothing (criterion 11).
    pub damaged: Vec<NodeId>,
    /// How many automatic tabs each outline that follows the operation gets (criterion 2).
    pub placed: Vec<usize>,
    /// Where the job enters each closed outline of a Cut or Cut-out block (criterion 19, C7).
    pub starts: Vec<CutStart>,
    /// The leaf (or item) of each lead or overcut that touches a cut (criterion 18).
    pub leads_touch: Vec<NodeId>,
}

/// The plan of `block` with `numbers`, entered from `entry`.
#[must_use]
pub fn plan_block(block: &BlockInput, numbers: &BlockNumbers, entry: PointUm) -> Plan {
    plan_with(block, numbers, entry, &mut RasterCaches::default())
}

/// The plan of `block` with the grey pyramids and sweeps of `caches`.
fn plan_with(
    block: &BlockInput,
    numbers: &BlockNumbers,
    entry: PointUm,
    caches: &mut RasterCaches,
) -> Plan {
    let kind = numbers.kind;
    let mut main = Vec::new();
    let mut outline = Vec::new();
    let mut raster = Vec::new();
    let mut no_contour = Vec::new();
    let mut cuts = Cuts::of(block, numbers);
    if kind == OperationKind::CutOut {
        cut_out_units(block, &mut main, &mut no_contour, &mut cuts);
    } else {
        for leaf in &block.leaves {
            leaf_units(
                leaf,
                numbers,
                &mut main,
                &mut outline,
                &mut raster,
                &mut cuts,
            );
        }
    }
    let bidirectional = numbers.options.bidirectional;
    let mut head = entry;
    let mut runs = Vec::new();
    let paths = ordered(main, &mut head, bidirectional, &mut cuts);
    let sweeps: Vec<PlannedSweep> = raster_sweeps(&mut raster, numbers, caches)
        .into_iter()
        .map(|(node, stacking, sweep)| PlannedSweep {
            node,
            stacking,
            sweep,
        })
        .collect();
    if let Some(last) = sweeps.last() {
        head = last.sweep.end;
    }
    if !paths.is_empty() || !sweeps.is_empty() {
        runs.push(PlannedRun {
            role: Role::Main,
            paths,
            sweeps,
        });
    }
    let paths = ordered(outline, &mut head, bidirectional, &mut cuts);
    if !paths.is_empty() {
        runs.push(PlannedRun {
            role: Role::Outline,
            paths,
            sweeps: Vec::new(),
        });
    }
    Plan {
        runs,
        exit: head,
        no_contour,
        marks: cuts.marks,
        covered: cuts.covered,
        damaged: cuts.damaged,
        placed: cuts.placed,
        starts: cuts.starts,
        leads_touch: cuts.leads_touch,
    }
}

/// The units of `list` ordered from `head` (ADR 0064 §5), `head` moved to where they end, their
/// tab gaps and covered outlines into `cuts`.
fn ordered(
    list: Units,
    head: &mut PointUm,
    bidirectional: bool,
    cuts: &mut Cuts,
) -> Vec<PlannedPath> {
    if list.is_empty() {
        return Vec::new();
    }
    let (nodes, units): (Vec<NodeId>, Vec<Unit>) = list.into_iter().unzip();
    let ordered = order_all(&units, *head, bidirectional);
    *head = ordered.exit;
    let mut ends = ordered.ends;
    cuts.marks
        .extend(ordered.gaps.into_iter().map(|(unit, gaps)| {
            let end = ends
                .iter_mut()
                .find(|(at, _)| *at == unit)
                .map(|(_, end)| std::mem::take(end))
                .unwrap_or_default();
            CutMark {
                node: nodes[unit],
                outline: units[unit].outline,
                gaps,
                lead_in: end.lead_in,
                overcut: end.overcut,
                lead_out: end.lead_out,
                start: end.start,
            }
        }));
    if cuts.settings.is_some() {
        cuts.starts
            .extend(ordered.starts.iter().map(|&(unit, at)| CutStart {
                node: nodes[unit],
                outline: units[unit].outline,
                at,
            }));
    }
    cuts.leads_touch
        .extend(ordered.touching.iter().map(|&unit| nodes[unit]));
    cuts.covered
        .extend(ordered.covered.iter().map(|&unit| nodes[unit]));
    cuts.placed.extend(ordered.placed);
    ordered
        .paths
        .into_iter()
        .map(|path| PlannedPath {
            points: path.points,
            hatch: path.hatch,
            node: nodes[path.unit],
        })
        .collect()
}

pub(crate) type Units = Vec<(NodeId, Unit)>;

/// The units of one leaf: in an Image block an image or a gradient-filled leaf into `raster`
/// and nothing else (`specs/0123` criteria 1 to 3, 48); in every other block its lines into
/// `main` (or `outline` for the outlines of Engrave + Outline) and its hatch into `main`, an
/// image as its frame rectangle (Frage 271 A).
fn leaf_units<'a>(
    leaf: &'a crate::job_input::LeafInput,
    numbers: &BlockNumbers,
    main: &mut Units,
    outline: &mut Units,
    raster: &mut Vec<RasterLeaf<'a>>,
    cuts: &mut Cuts,
) {
    let kind = numbers.kind;
    if kind == OperationKind::Image {
        raster_leaf(leaf, raster);
        return;
    }
    let (shape, outlines) = leaf_shape(&leaf.geometry);
    if cuts.settings.is_some() && leaf_damaged(&leaf.cut, &leaf.path, &outlines, cuts.settings) {
        cuts.damaged.push(leaf.node);
        return;
    }
    let run = kind_run(kind, shape);
    let to_outline = kind == OperationKind::EngraveOutline && run.area;
    let stacking = leaf.stacking;
    for (index, item) in outlines.iter().enumerate() {
        let wanted = match run.lines {
            Lines::None => false,
            Lines::All => true,
            Lines::Closed => item.closed,
        };
        if !wanted {
            continue;
        }
        let into = if to_outline {
            &mut *outline
        } else {
            &mut *main
        };
        push_line(into, leaf, index, item, cuts);
    }
    if run.area
        && let Some(unit) = hatch_unit(&outlines, numbers, stacking)
    {
        main.push((leaf.node, unit));
    }
}

/// Pushes outline `item` (number `index`) of `leaf` as a line unit, with its cut path step when
/// the block takes one.
fn push_line(
    into: &mut Units,
    leaf: &crate::job_input::LeafInput,
    index: usize,
    item: &LeafOutline,
    cuts: &Cuts,
) {
    if let Some(points) = flat(item) {
        let closed = item.closed && points.len() >= 3;
        let outline = index_u32(index);
        let cut = unit_cut(cuts, &leaf.cut, &leaf.path, outline, &points, closed);
        into.push((
            leaf.node,
            Unit {
                shape: UnitShape::Path { closed, points },
                stacking: leaf.stacking,
                outline,
                cut,
            },
        ));
    }
}

/// The outlines a leaf runs as in a Cut block, flattened and rounded to µm as the job writes them,
/// by outline index (`None` for one with fewer than two points) with whether each runs closed:
/// what its tab handles sit on (`specs/0139` adrs L12).
#[must_use]
pub fn cut_outlines(geometry: &LeafGeometry) -> Vec<Option<(Vec<PointUm>, bool)>> {
    let (_, outlines) = leaf_shape(geometry);
    outlines
        .iter()
        .map(|item| {
            let points = flat(item)?;
            let closed = item.closed && points.len() >= 3;
            Some((points, closed))
        })
        .collect()
}

/// The contours a Cut-out item with `leaves` runs with `margin`, in µm as the job writes them
/// (`None` for one that gives fewer than three points).
#[must_use]
pub fn cut_out_outlines(
    leaves: &[Vec<LeafOutline>],
    margin: curvyo_document_core::Length,
) -> Vec<Option<Vec<PointUm>>> {
    let outlines: Vec<Vec<Outline<'_>>> = leaves
        .iter()
        .map(|leaf| {
            leaf.iter()
                .map(|o| Outline::new(&o.anchors, o.closed))
                .collect()
        })
        .collect();
    let refs: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
    cut_out_contours(&refs, margin, Tolerance::from_mm(FLATTEN_MM))
        .unwrap_or_default()
        .iter()
        .map(|contour| {
            let points = dedupe(contour.iter().map(|&p| PointUm::of_mm(p)), true);
            (points.len() >= 3).then_some(points)
        })
        .collect()
}

/// The shape a leaf runs as outside the raster and its outlines (an image's frame as one closed
/// outline, which every kind but Image runs as a rectangle; a gradient-filled leaf as its
/// outlines, the paint never counts there, `0101` criterion 24).
fn leaf_shape(geometry: &LeafGeometry) -> (LeafShape, Vec<LeafOutline>) {
    let counts = |outlines: &[LeafOutline]| {
        let closed = outlines.iter().filter(|o| o.closed).count();
        (closed, outlines.len() - closed)
    };
    match geometry {
        LeafGeometry::Image { frame, .. } => (LeafShape::Image, vec![frame_outline(frame)]),
        LeafGeometry::Outlines(outlines) => {
            let (closed, open) = counts(outlines);
            (LeafShape::Outlines { closed, open }, outlines.clone())
        }
        LeafGeometry::Gradient { outlines, .. } => {
            let (closed, open) = counts(outlines);
            (LeafShape::Gradient { closed, open }, outlines.clone())
        }
    }
}

/// The hatch unit of a leaf's area (criteria 14, 15), the crosshatch after it; `None` when no
/// line falls inside.
fn hatch_unit(outlines: &[LeafOutline], numbers: &BlockNumbers, stacking: u32) -> Option<Unit> {
    let rings: Vec<Vec<PointUm>> = outlines.iter().filter_map(flat).collect();
    let interval = i64::from(numbers.interval.hundredths()) * 10;
    let angle = numbers.options.scan_angle.degrees();
    let mut sets = vec![hatch(&rings, interval, angle)];
    if numbers.options.crosshatch {
        sets.push(hatch(&rings, interval, angle + 90.0));
    }
    sets.retain(|lines| !lines.is_empty());
    (!sets.is_empty()).then_some(Unit {
        shape: UnitShape::Hatch(sets),
        stacking,
        outline: 0,
        cut: None,
    })
}

pub(crate) fn index_u32(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// The plans of the blocks of one page, reused while a block's fingerprint and entry point stay
/// the same (ADR 0064 §9). Lookups only: nothing iterates the map into output.
#[derive(Debug, Default)]
pub struct PlanCache {
    entries: HashMap<NodeId, (u64, PointUm, Arc<Plan>)>,
    raster: RasterCaches,
}

impl PlanCache {
    /// The plan of `block`, from the cache when its key matches.
    pub fn plan(
        &mut self,
        block: &BlockInput,
        numbers: &BlockNumbers,
        entry: PointUm,
    ) -> Arc<Plan> {
        let key = plan_fingerprint(block, numbers);
        if let Some((cached, at, plan)) = self.entries.get(&block.id)
            && *cached == key
            && *at == entry
        {
            return Arc::clone(plan);
        }
        let plan = Arc::new(plan_with(block, numbers, entry, &mut self.raster));
        self.entries
            .insert(block.id, (key, entry, Arc::clone(&plan)));
        plan
    }

    /// Advances the raster sweeps the plan of `block` with `numbers` needs by about `dots` dots,
    /// `dots` lessened by the dots worked; whether its plan is cached or all of them are finished,
    /// so [`PlanCache::plan`] builds no dot.
    pub fn prepare(&mut self, block: &BlockInput, numbers: &BlockNumbers, dots: &mut u64) -> bool {
        let key = plan_fingerprint(block, numbers);
        if self
            .entries
            .get(&block.id)
            .is_some_and(|(cached, _, _)| *cached == key)
        {
            return true;
        }
        prepare_raster(block, numbers, &mut self.raster, dots)
    }

    /// Drops the plans of blocks not in `ids`, the grey pyramids of assets not in `assets` and the
    /// sweeps of raster items not in `items`.
    pub fn retain(&mut self, ids: &[NodeId], assets: &[&AssetHash], items: &[NodeId]) {
        let ids: HashSet<&NodeId> = ids.iter().collect();
        self.entries.retain(|id, _| ids.contains(id));
        self.raster.pyramids.retain(assets);
        self.raster.sweeps.retain(items);
    }

    /// An empty cache that shares the grey pyramids of this one (one pyramid per asset for the
    /// job and the Operations view's preview, ADR 0069 §5).
    #[must_use]
    pub fn sharing_pyramids(&self) -> Self {
        Self {
            entries: HashMap::new(),
            raster: RasterCaches {
                pyramids: self.raster.pyramids.clone(),
                sweeps: SweepCache::default(),
            },
        }
    }

    /// The grey pyramids and adjusted pictures this cache shares (a clone shares its maps).
    #[must_use]
    pub fn pictures(&self) -> crate::image_adjust_cache::PyramidCache {
        self.raster.pyramids.clone()
    }

    /// How many grey pyramids are kept.
    #[must_use]
    pub fn pyramids(&self) -> usize {
        self.raster.pyramids.len()
    }

    /// How many plans are cached.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether no plan is cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
