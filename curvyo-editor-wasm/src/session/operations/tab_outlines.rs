//! What the page's job says of the tabs (`specs/0139` criteria 2, 4, 5; adrs L12): the outlines
//! of the selected items with the tabs each shows, so a handle sits where the file cuts, and the
//! placed line of each operation.

use std::collections::HashMap;

use curvyo_document_core::{NodeId, Point};
use curvyo_laser_core::job::{BlockSource, PointUm, cut_out_outlines, cut_outlines};
use curvyo_laser_core::operations::OperationKind;
use curvyo_laser_core::settings::{CutLength, CutSettings, PathOptions, cuts_through};
use curvyo_laser_core::tabs::{
    LeafPath, LeafTabs, Measured, OutlineEntry, TabSpan, automatic, outline_ref,
};

use super::super::Session;

/// Handles show only on at most this many selected outlines (C5).
const MAX_OUTLINES: usize = 50;

/// One outline of a selected item in the operation whose tabs it shows.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::session) struct TabOutline {
    /// The operation.
    pub op: NodeId,
    /// The membership key text of the item's target.
    pub key: String,
    /// The outline reference in the item's record.
    pub reference: String,
    /// The outline in page µm.
    pub measured: Measured,
    /// Its tabs in centre order.
    pub spans: Vec<TabSpan>,
    /// The own width of each tab, `None` for the operation's.
    pub widths: Vec<Option<CutLength>>,
    /// Whether the item has own tabs in the operation.
    pub own: bool,
    /// The operation's cut settings.
    pub settings: CutSettings,
    /// The operation's path options.
    pub path: PathOptions,
    /// The page origin in document mm (page µm + origin = document).
    pub origin: Point,
    /// The outline's set start (closed) or fixed direction (open); `None` for Automatic.
    pub entry: Option<OutlineEntry>,
    /// Where the job enters a closed outline, as an arc (the Automatic marker, C7).
    pub entered: Option<f64>,
}

impl TabOutline {
    pub(super) fn to_document(&self, arc: f64) -> Point {
        let (x, y) = self.measured.point_at(arc);
        Point::new(x / 1000.0 + self.origin.x, y / 1000.0 + self.origin.y)
    }

    pub(super) fn to_page_um(&self, point: Point) -> (f64, f64) {
        (
            (point.x - self.origin.x) * 1000.0,
            (point.y - self.origin.y) * 1000.0,
        )
    }
}

impl Session {
    /// The placed line of every Cut and Cut-out of the page's job (`specs/0139` criterion 2).
    pub(super) fn placed_lines(&self) -> HashMap<NodeId, String> {
        let Some((job, _)) = self.settled_job() else {
            return HashMap::new();
        };
        job.input
            .blocks
            .iter()
            .zip(&job.built.plans)
            .filter_map(|(block, plan)| {
                let curvyo_laser_core::job::BlockSource::Operation {
                    options: Ok(options),
                    ..
                } = &block.source
                else {
                    return None;
                };
                let text = curvyo_laser_core::tabs::placed_text(
                    options.cut.tab_count,
                    &plan.as_ref()?.placed,
                )?;
                Some((block.id, text))
            })
            .collect()
    }

    /// The outlines of the selected items with their tabs, in the operation `only` or each in its
    /// first Cut or Cut-out of the run order (C2), at most [`MAX_OUTLINES`].
    pub(in crate::session) fn tab_outlines(&self, only: Option<NodeId>) -> Vec<TabOutline> {
        let selected = self.selection.ids();
        // Only Cut and Cut-out blocks have cut keys: no job is built for a page without them.
        if selected.is_empty()
            || !self
                .active_operations()
                .is_some_and(|active| active.any_cuts_through())
        {
            return Vec::new();
        }
        // The job the views read (stale while a big raster edit is worked, `0029` 37 d).
        let Some((job, _)) = self.settled_job() else {
            return Vec::new();
        };
        let Some(origin) = self
            .active_manufacturing()
            .and_then(|m| m.page.map(|page| page.origin))
        else {
            return Vec::new();
        };
        let is_selected = |node: NodeId| {
            selected.contains(&node)
                || self
                    .document
                    .node_chain(node)
                    .iter()
                    .any(|id| selected.contains(id))
        };
        let mut out = Vec::new();
        let mut taken: Vec<NodeId> = Vec::new();
        for key in &job.cut_keys {
            if only.is_some_and(|op| op != key.op)
                || taken.contains(&key.node)
                || !is_selected(key.node)
            {
                continue;
            }
            let Some(found) = key_outlines(&job, key, origin) else {
                continue;
            };
            if only.is_none() {
                taken.push(key.node);
            }
            out.extend(found);
            if out.len() >= MAX_OUTLINES {
                out.truncate(MAX_OUTLINES);
                return out;
            }
        }
        out
    }
}

/// The flattened outlines of one cut leaf (or Cut-out item) of a job, by outline index.
type Rings = Vec<Option<(Vec<PointUm>, bool)>>;

/// The outlines of the leaf (or Cut-out item) `key` names in `job` with their tabs; `None` when
/// its block is no Cut or Cut-out, or its record is damaged (it runs nothing).
fn key_outlines(
    job: &super::super::output::PageJob,
    key: &super::super::output::CutKey,
    origin: Point,
) -> Option<Vec<TabOutline>> {
    let (at, block) = job
        .input
        .blocks
        .iter()
        .enumerate()
        .find(|(_, b)| b.id == key.op)?;
    let BlockSource::Operation {
        options: Ok(options),
        margin,
        ..
    } = &block.source
    else {
        return None;
    };
    let kind = block.kind.known().filter(|&k| cuts_through(k))?;
    let (rings, tabs, path): (Rings, &LeafTabs, &LeafPath) = if kind == OperationKind::CutOut {
        let item = block.cut_out.get(key.at)?;
        let margin = curvyo_document_core::Length::from_mm(margin.as_mm());
        let rings = cut_out_outlines(&item.leaves, margin)
            .into_iter()
            .map(|r| r.map(|p| (p, true)))
            .collect();
        (rings, &item.cut, &item.path)
    } else {
        let leaf = block.leaves.get(key.at)?;
        (cut_outlines(&leaf.geometry), &leaf.cut, &leaf.path)
    };
    if matches!(tabs, LeafTabs::Damaged) || path.damaged {
        return None;
    }
    let plan = job.built.plans.get(at).and_then(Option::as_ref);
    let settings = options.cut;
    let mut out = Vec::new();
    for (index, ring) in rings.into_iter().enumerate() {
        let Some((points, closed)) = ring else {
            continue;
        };
        let outline = u32::try_from(index).unwrap_or(u32::MAX);
        let measured = Measured::new(points, closed);
        let (spans, widths) = if let Some(own) = tabs.own(outline) {
            let spans = curvyo_laser_core::tabs::own_spans(&measured, own, settings.tab_width);
            (spans, own.iter().map(|t| t.width).collect())
        } else {
            // Skip inner shapes: the job's marks say whether this outline got any.
            let placed = plan.is_some_and(|plan| {
                plan.marks
                    .iter()
                    .any(|m| m.node == key.node && m.outline == outline && !m.gaps.is_empty())
            });
            let spans = if placed || !settings.tab_skip_inner {
                automatic(
                    measured.length(),
                    settings.tab_count,
                    settings.tab_width,
                    measured.closed(),
                )
            } else {
                Vec::new()
            };
            let widths = vec![None; spans.len()];
            (spans, widths)
        };
        let reference = if kind == OperationKind::CutOut {
            index.to_string()
        } else {
            outline_ref(&key.below, index)
        };
        let entry = path
            .entry(outline)
            .filter(|e| matches!(e, OutlineEntry::Start(_)) == measured.closed());
        let entered = plan
            .and_then(|plan| {
                plan.starts
                    .iter()
                    .find(|s| s.node == key.node && s.outline == outline)
            })
            .filter(|_| measured.closed())
            .map(|s| {
                let at = s.at;
                measured.nearest((um(at.x), um(at.y))).0
            });
        out.push(TabOutline {
            entry,
            entered,
            op: key.op,
            key: key.key.clone(),
            reference,
            measured,
            spans,
            widths,
            own: !matches!(tabs, LeafTabs::Operation),
            settings,
            path: options.path,
            origin,
        });
    }
    Some(out)
}

fn um(value: i64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    // exact: page coordinates in µm are far below 2^53
    let v = value as f64;
    v
}
