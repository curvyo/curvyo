//! The order of the paths inside one block (ADR 0064 §5; criterion 22, Frage 225 A): a path runs
//! before every closed path that contains it, then the nearest start to the head goes next, ties
//! by stacking order, outline index and node index.

use curvyo_geometry_core::{
    NestPath, OrderUnit, PathEntry, nearest_order, path_parents, path_run, path_starts,
};

use crate::job_cut_ends::CutEnds;
use crate::job_cut_path::{CutEntry, CutRun, TabGap, UnitCut};
use crate::job_hatch::{HatchLines, Span};
use crate::job_input::PointUm;

/// One thing to order: a flattened path, or the hatch of one area.
#[derive(Debug, Clone, PartialEq)]
pub enum UnitShape {
    /// A path in µm; a closed one does not repeat its first point.
    Path {
        /// The points.
        points: Vec<PointUm>,
        /// Whether it is closed.
        closed: bool,
    },
    /// The hatch of one area: one set of lines, or two with Crosshatch.
    Hatch(Vec<HatchLines>),
}

/// One unit with its tie-breakers.
#[derive(Debug, Clone, PartialEq)]
pub struct Unit {
    /// The shape.
    pub shape: UnitShape,
    /// The leaf's stacking order.
    pub stacking: u32,
    /// The outline's index in its leaf.
    pub outline: u32,
    /// The tabs of a cut outline (`specs/0139`); `None` runs today's code path (criterion 24).
    pub cut: Option<Box<UnitCut>>,
}

/// How the chosen unit is entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Entry {
    /// A path, closed from a node or open from its nearer end.
    Path(PathEntry),
    /// A hatch, its first set by line order and direction.
    Hatch { reversed: bool, flipped: bool },
    /// A cut outline with tabs, by its entry `n` (adrs L7).
    Cut(usize),
}

/// One ordered burn path: its points in run order and the unit it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderedPath {
    /// The points; a closed path ends on its first point.
    pub points: Vec<PointUm>,
    /// The unit's index in the input.
    pub unit: usize,
    /// Whether it is a hatch span.
    pub hatch: bool,
}

/// The ordered paths of a block with what its tabs leave (`specs/0139` criteria 7, 10).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Ordered {
    /// The burn paths in run order.
    pub paths: Vec<OrderedPath>,
    /// Where the head stands at the end.
    pub exit: PointUm,
    /// The gaps of each unit with tabs, by unit index.
    pub gaps: Vec<(usize, Vec<TabGap>)>,
    /// The units whose tabs cover them whole: they burn nothing.
    pub covered: Vec<usize>,
    /// How many automatic tabs each outline that follows the operation gets (criterion 2's
    /// placed line); an inner outline under Skip inner shapes is left out.
    pub placed: Vec<usize>,
    /// The leads, overcut and start of each entered cut outline, by unit index.
    pub ends: Vec<(usize, CutEnds)>,
    /// Where each closed path was entered (its first burnt point), by unit index (C7).
    pub starts: Vec<(usize, PointUm)>,
    /// One unit index per lead or overcut that comes within 0.01 mm of a cut (criterion 18).
    pub touching: Vec<usize>,
}

/// The ordered burn paths of `units` from `head`, and where the head stands at the end.
/// `bidirectional` alternates the hatch direction line by line.
#[cfg(test)]
#[must_use]
pub fn order(units: &[Unit], head: PointUm, bidirectional: bool) -> (Vec<OrderedPath>, PointUm) {
    let ordered = order_all(units, head, bidirectional);
    (ordered.paths, ordered.exit)
}

/// [`order`] with the gaps and covered units of the cut outlines with tabs.
pub(crate) fn order_all(units: &[Unit], head: PointUm, bidirectional: bool) -> Ordered {
    let parents = parents(units);
    let Prepared {
        runs,
        cut_entries,
        candidates,
        placed,
    } = prepare(units, &parents, bidirectional);
    let mut done = vec![false; units.len()];
    let mut gaps = Vec::new();
    let mut covered = Vec::new();
    for (index, run) in runs.iter().enumerate() {
        let Some(run) = run else {
            continue;
        };
        gaps.push((index, run.gaps()));
        if run.covered() {
            // A covered outline burns nothing and holds back no parent (criterion 10).
            covered.push(index);
            done[index] = true;
        }
    }
    let order: Vec<OrderUnit> = units
        .iter()
        .enumerate()
        .map(|(index, unit)| OrderUnit {
            starts: candidates[index].iter().map(|c| c.0).collect(),
            rank: (unit.stacking, unit.outline),
            parent: parents[index],
            done: done[index],
        })
        .collect();
    let mut out = Vec::new();
    let mut ends = Vec::new();
    let mut starts = Vec::new();
    let head = nearest_order(&order, head, |index, tie| {
        let entry = candidates[index][tie].1;
        if let (Entry::Cut(k), Some(run)) = (entry, &runs[index]) {
            let (pieces, run_ends) = run.run(cut_entries[index][k]);
            if run.measured.closed()
                && let Some(at) = run_ends.entered
            {
                starts.push((index, at));
            }
            ends.push((index, run_ends));
            let mut end = None;
            for points in pieces {
                end = points.last().copied();
                out.push(OrderedPath {
                    points,
                    unit: index,
                    hatch: false,
                });
            }
            end
        } else {
            if let (UnitShape::Path { points, .. }, Entry::Path(PathEntry::Closed(n))) =
                (&units[index].shape, entry)
            {
                starts.push((index, points[n]));
            }
            emit(&units[index].shape, index, entry, bidirectional, &mut out)
        }
    });
    let touching = crate::job_cut_checks::touching(&ends, &out);
    Ordered {
        paths: out,
        exit: head,
        gaps,
        covered,
        placed,
        ends,
        starts,
        touching,
    }
}

/// The nesting depth of each unit: how many closed paths of the block contain it.
fn depths(parents: &[Option<usize>]) -> Vec<usize> {
    parents
        .iter()
        .map(|&parent| {
            let mut depth = 0;
            let mut at = parent;
            while let Some(p) = at {
                depth += 1;
                at = parents[p];
                if depth > parents.len() {
                    break;
                }
            }
            depth
        })
        .collect()
}

/// The tab runs of the cut units and every unit's entries.
struct Prepared {
    runs: Vec<Option<CutRun>>,
    cut_entries: Vec<Vec<CutEntry>>,
    candidates: Vec<Vec<(PointUm, Entry)>>,
    placed: Vec<usize>,
}

/// The tab runs of `units` (with their parents for Skip inner shapes) and the entries of each
/// unit, a cut outline with tabs by its burnt vertices and piece starts (adrs L7).
fn prepare(units: &[Unit], parents: &[Option<usize>], bidirectional: bool) -> Prepared {
    let mut placed = Vec::new();
    let depth = depths(parents);
    let runs: Vec<Option<CutRun>> = units
        .iter()
        .zip(parents)
        .zip(depth)
        .map(|((unit, parent), depth)| {
            let cut = unit.cut.as_ref()?;
            let spans = cut.spans(parent.is_some());
            if cut.own.is_none()
                && cut.settings.tab_count.get() > 0
                && !(parent.is_some() && cut.settings.tab_skip_inner)
            {
                placed.push(spans.len());
            }
            let mut run = CutRun::new(cut.measured.clone(), spans, cut.path, cut.entry)?;
            run.depth = depth;
            Some(run)
        })
        .collect();
    let mut cut_entries: Vec<Vec<CutEntry>> = vec![Vec::new(); units.len()];
    let candidates = units
        .iter()
        .zip(&runs)
        .enumerate()
        .map(|(index, (unit, run))| match run {
            Some(run) => {
                let entries = run.entries();
                cut_entries[index] = entries.iter().map(|e| e.1).collect();
                entries
                    .iter()
                    .enumerate()
                    .map(|(k, e)| (e.0, Entry::Cut(k)))
                    .collect()
            }
            None => candidates(&unit.shape, bidirectional),
        })
        .collect();
    Prepared {
        runs,
        cut_entries,
        candidates,
        placed,
    }
}

/// The start points a unit may be entered at, in tie order.
fn candidates(shape: &UnitShape, bidirectional: bool) -> Vec<(PointUm, Entry)> {
    match shape {
        UnitShape::Path { points, closed } => path_starts(points, *closed)
            .into_iter()
            .map(|(p, entry)| (p, Entry::Path(entry)))
            .collect(),
        UnitShape::Hatch(sets) => match sets.first() {
            Some(lines) if !lines.is_empty() => {
                [(false, false), (false, true), (true, false), (true, true)]
                    .into_iter()
                    .filter_map(|(reversed, flipped)| {
                        hatch_spans(lines, reversed, flipped, bidirectional)
                            .first()
                            .map(|span| (span.0, Entry::Hatch { reversed, flipped }))
                    })
                    .collect()
            }
            _ => Vec::new(),
        },
    }
}

/// The spans of `lines` in run order: lines reversed in order with `reversed`, each line's
/// direction the base one, turned with `flipped` and on every odd line when `bidirectional`.
fn hatch_spans(
    lines: &HatchLines,
    reversed: bool,
    flipped: bool,
    bidirectional: bool,
) -> Vec<Span> {
    let order: Box<dyn Iterator<Item = &Vec<Span>>> = if reversed {
        Box::new(lines.iter().rev())
    } else {
        Box::new(lines.iter())
    };
    let mut out = Vec::new();
    for (k, line) in order.enumerate() {
        let back = flipped ^ (bidirectional && k % 2 == 1);
        if back {
            out.extend(line.iter().rev().map(|&(a, b)| (b, a)));
        } else {
            out.extend(line.iter().copied());
        }
    }
    out
}

/// Appends the paths of `shape` entered by `entry`; returns where the head ends.
fn emit(
    shape: &UnitShape,
    unit: usize,
    entry: Entry,
    bidirectional: bool,
    out: &mut Vec<OrderedPath>,
) -> Option<PointUm> {
    match (shape, entry) {
        (UnitShape::Path { points, .. }, Entry::Path(entry)) => {
            let path = path_run(points, entry);
            let end = path.last().copied();
            out.push(OrderedPath {
                points: path,
                unit,
                hatch: false,
            });
            end
        }
        (UnitShape::Hatch(sets), Entry::Hatch { reversed, flipped }) => {
            let mut head = None;
            for (index, lines) in sets.iter().enumerate() {
                let spans = if index == 0 {
                    hatch_spans(lines, reversed, flipped, bidirectional)
                } else {
                    nearest_variant(lines, head, bidirectional)
                };
                for (a, b) in spans {
                    out.push(OrderedPath {
                        points: vec![a, b],
                        unit,
                        hatch: true,
                    });
                    head = Some(b);
                }
            }
            head
        }
        _ => None,
    }
}

/// The spans of a later hatch set (Crosshatch) entered nearest to `head`, ties in variant order.
fn nearest_variant(lines: &HatchLines, head: Option<PointUm>, bidirectional: bool) -> Vec<Span> {
    let variants = [(false, false), (false, true), (true, false), (true, true)];
    variants
        .into_iter()
        .map(|(reversed, flipped)| hatch_spans(lines, reversed, flipped, bidirectional))
        .filter(|spans| !spans.is_empty())
        .min_by_key(|spans| head.map_or(0, |h| h.distance_squared(spans[0].0)))
        .unwrap_or_default()
}

/// The innermost closed path that contains each unit (`curvyo-geometry-core`'s `path_parents`);
/// hatches have none.
pub(crate) fn parents(units: &[Unit]) -> Vec<Option<usize>> {
    let nest: Vec<NestPath<'_>> = units
        .iter()
        .map(|unit| match &unit.shape {
            UnitShape::Path { points, closed } => Some((points.as_slice(), *closed)),
            UnitShape::Hatch(_) => None,
        })
        .collect();
    path_parents(&nest)
}

#[cfg(test)]
#[path = "job_order_tests.rs"]
mod tests;
