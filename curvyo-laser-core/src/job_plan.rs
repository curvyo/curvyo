//! The ordered burn paths of one block, by kind (ADR 0064 §4, §5; criteria 13 to 18, 22): what
//! each leaf runs, flattened at 0.0095 mm and rounded to µm, hatched and ordered.

use std::collections::HashMap;
use std::sync::Arc;

use curvyo_document_core::{NodeId, Tolerance};
use curvyo_geometry_core::{Outline, flatten_outline};

use crate::job_fingerprint::plan_fingerprint;
use crate::job_hatch::hatch;
use crate::job_input::{BlockInput, BlockSource, LeafGeometry, LeafOutline, PointUm};
use crate::job_numbers::BlockNumbers;
use crate::job_order::{Unit, UnitShape, order};
use crate::operation_cut_out::cut_out_contours;
use crate::operation_kind::OperationKind;
use crate::operation_kind_run::{LeafShape, Lines, kind_run};
use crate::operation_settings::Role;

/// The flattening tolerance: with the 0.0005 mm of µm rounding the written path stays within
/// 0.01 mm of the curve (criterion 17).
pub const FLATTEN_MM: f64 = 0.0095;

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

/// The paths one set of a block runs, once per pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRun {
    /// The set the paths run with.
    pub role: Role,
    /// The paths in run order.
    pub paths: Vec<PlannedPath>,
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
}

/// The plan of `block` with `numbers`, entered from `entry`.
#[must_use]
pub fn plan_block(block: &BlockInput, numbers: &BlockNumbers, entry: PointUm) -> Plan {
    let kind = numbers.kind;
    let mut main = Vec::new();
    let mut outline = Vec::new();
    let mut no_contour = Vec::new();
    if kind == OperationKind::CutOut {
        cut_out_units(block, &mut main, &mut no_contour);
    } else {
        for leaf in &block.leaves {
            leaf_units(leaf, numbers, &mut main, &mut outline);
        }
    }
    let bidirectional = numbers.options.bidirectional;
    let mut head = entry;
    let mut runs = Vec::new();
    for (role, list) in [(Role::Main, main), (Role::Outline, outline)] {
        if list.is_empty() {
            continue;
        }
        let (nodes, units): (Vec<NodeId>, Vec<Unit>) = list.into_iter().unzip();
        let (ordered, exit) = order(&units, head, bidirectional);
        head = exit;
        runs.push(PlannedRun {
            role,
            paths: ordered
                .into_iter()
                .map(|path| PlannedPath {
                    points: path.points,
                    hatch: path.hatch,
                    node: nodes[path.unit],
                })
                .collect(),
        });
    }
    Plan {
        runs,
        exit: head,
        no_contour,
    }
}

type Units = Vec<(NodeId, Unit)>;

/// The units of one leaf: its lines into `main` (or `outline` for the outlines of Engrave +
/// Outline) and its hatch into `main`. Engrave images run nothing (criterion 18).
fn leaf_units(
    leaf: &crate::job_input::LeafInput,
    numbers: &BlockNumbers,
    main: &mut Units,
    outline: &mut Units,
) {
    let kind = numbers.kind;
    let Some((shape, outlines)) = leaf_shape(&leaf.geometry, kind) else {
        return;
    };
    let run = kind_run(kind, shape);
    let to_outline = kind == OperationKind::EngraveOutline && run.area;
    let stacking = leaf.stacking;
    for (index, item) in outlines.iter().enumerate() {
        let wanted = match run.lines {
            Lines::None => false,
            Lines::All | Lines::Frame => true,
            Lines::Closed => item.closed,
        };
        if !wanted {
            continue;
        }
        if let Some(points) = flat(item) {
            let into = if to_outline {
                &mut *outline
            } else {
                &mut *main
            };
            into.push((
                leaf.node,
                Unit {
                    shape: UnitShape::Path {
                        closed: item.closed && points.len() >= 3,
                        points,
                    },
                    stacking,
                    outline: index_u32(index),
                },
            ));
        }
    }
    if run.area
        && let Some(unit) = hatch_unit(&outlines, numbers, stacking)
    {
        main.push((leaf.node, unit));
    }
}

/// The shape a leaf runs as and its outlines (an image's frame as one closed outline); `None`
/// for an image in an Engrave kind, which writes nothing (criterion 18).
fn leaf_shape(
    geometry: &LeafGeometry,
    kind: OperationKind,
) -> Option<(LeafShape, Vec<LeafOutline>)> {
    match geometry {
        LeafGeometry::Image { .. }
            if matches!(kind, OperationKind::Engrave | OperationKind::EngraveOutline) =>
        {
            None
        }
        LeafGeometry::Image { frame } => Some((
            LeafShape::Image,
            vec![LeafOutline {
                anchors: frame
                    .iter()
                    .map(|&p| {
                        (
                            p,
                            curvyo_document_core::Vec2::ZERO,
                            curvyo_document_core::Vec2::ZERO,
                        )
                    })
                    .collect(),
                closed: true,
            }],
        )),
        LeafGeometry::Outlines(outlines) => {
            let closed = outlines.iter().filter(|o| o.closed).count();
            Some((
                LeafShape::Outlines {
                    closed,
                    open: outlines.len() - closed,
                },
                outlines.clone(),
            ))
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
    })
}

/// The contour units of the Cut-out items; an item with no contour is listed in `no_contour`.
fn cut_out_units(block: &BlockInput, main: &mut Units, no_contour: &mut Vec<NodeId>) {
    let margin = match &block.source {
        BlockSource::Operation { margin, .. } => *margin,
        BlockSource::Fixture { .. } => curvyo_document_core::Length::from_mm(0.0),
    };
    for item in &block.cut_out {
        let outlines: Vec<Vec<Outline<'_>>> = item
            .leaves
            .iter()
            .map(|leaf| {
                leaf.iter()
                    .map(|o| Outline::new(&o.anchors, o.closed))
                    .collect()
            })
            .collect();
        let refs: Vec<&[Outline<'_>]> = outlines.iter().map(Vec::as_slice).collect();
        let contours =
            cut_out_contours(&refs, margin, Tolerance::from_mm(FLATTEN_MM)).unwrap_or_default();
        let mut any = false;
        for (index, contour) in contours.iter().enumerate() {
            let points = dedupe(contour.iter().map(|&p| PointUm::of_mm(p)), true);
            if points.len() < 3 {
                continue;
            }
            any = true;
            main.push((
                item.item,
                Unit {
                    shape: UnitShape::Path {
                        points,
                        closed: true,
                    },
                    stacking: item.stacking,
                    outline: index_u32(index),
                },
            ));
        }
        if !any {
            no_contour.push(item.item);
        }
    }
}

/// The outline flattened and rounded to µm, `None` with fewer than two distinct points.
fn flat(outline: &LeafOutline) -> Option<Vec<PointUm>> {
    let points = flatten_outline(
        &outline.anchors,
        outline.closed,
        Tolerance::from_mm(FLATTEN_MM),
    );
    let points = dedupe(points.into_iter().map(PointUm::of_mm), outline.closed);
    (points.len() >= 2).then_some(points)
}

/// The points with consecutive repeats removed (and a closing repeat of the first when closed).
fn dedupe(points: impl Iterator<Item = PointUm>, closed: bool) -> Vec<PointUm> {
    let mut out: Vec<PointUm> = Vec::new();
    for point in points {
        if out.last() != Some(&point) {
            out.push(point);
        }
    }
    if closed && out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

fn index_u32(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// The plans of the blocks of one page, reused while a block's fingerprint and entry point stay
/// the same (ADR 0064 §9). Lookups only: nothing iterates the map into output.
#[derive(Debug, Default)]
pub struct PlanCache {
    entries: HashMap<NodeId, (u64, PointUm, Arc<Plan>)>,
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
        let plan = Arc::new(plan_block(block, numbers, entry));
        self.entries
            .insert(block.id, (key, entry, Arc::clone(&plan)));
        plan
    }

    /// Drops the plans of blocks not in `ids`.
    pub fn retain(&mut self, ids: &[NodeId]) {
        self.entries.retain(|id, _| ids.contains(id));
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
