//! Planning Split on the selected compound paths: which outlines form a part (a shape with the
//! holes directly inside it, joined with every part whose outline touches it, and the open
//! outlines that touch it or start on it) and what the pieces are
//! (`specs/0048-split-compound-path`, criteria 3 to 10; `specs/0054-combine-open-paths`,
//! criteria 15 to 17).
//!
//! Pure: it reads snapshots and the selection and returns a plan or a refusal. The one write
//! (`Document::replace_with_pieces`) is the session's. Nodes are copied as they are, no flattening
//! and no reversal: the windings of a compound path are canonical already.

use curvyo_document_core::{NewAnchor, NodeId, ObjectSnapshot, PieceOutlines, SubpathRef};
use curvyo_geometry_core::{
    Nesting, Outline, OutlineTriple, contains_point_in_outlines, outline_nesting, touching_outlines,
};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::operands_in_order;
use crate::combine::{fresh_ids, is_compound, out_of_range};
use crate::object_selection::ObjectSelection;

/// The commit label of Split; the undo slice maps it to display text.
pub const SPLIT_COMMIT_LABEL: &str = "split";

/// Why Split was refused. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SplitRefusal {
    /// The selection holds no compound path (the button is dimmed then; reachable only through
    /// the session function).
    NoCompound,
    /// The selection holds a group; Split does not look through groups
    /// (`specs/0023-groups` criterion 28). `offenders` are the leaves of the groups, to outline.
    Groups {
        /// The leaves of the selected groups, to outline.
        offenders: Vec<NodeId>,
    },
    /// `offenders` of the `of` selected objects are compound paths with a coordinate that is not
    /// finite or beyond the kernel's range: the part search cannot measure them.
    OutOfRange {
        /// The compound paths out of range, to outline.
        offenders: Vec<NodeId>,
        /// How many objects are selected.
        of: usize,
    },
    /// Every selected compound path is one part: `compounds` of them, each already one piece.
    OnePiece {
        /// How many compound paths are selected.
        compounds: usize,
        /// Whether one of them has a hole (the sentence then names the holes, `0054`
        /// criterion 17).
        with_holes: bool,
    },
}

/// What [`plan_split`] decided, ready for `Document::replace_with_pieces`.
#[derive(Debug, Clone, PartialEq)]
pub struct SplitPlan {
    /// The compound paths to replace, bottom-most first.
    pub removed: Vec<NodeId>,
    /// The pieces, each with the compound path it takes its place and style from: the parts of
    /// one compound path are consecutive, in the order of their first outline, with fresh anchor
    /// ids.
    pub pieces: Vec<(NodeId, PieceOutlines)>,
    /// Whether a piece has a hole.
    pub with_holes: bool,
    /// The selected objects that stay as they are: ordinary objects and compound paths of one
    /// part.
    pub kept: Vec<NodeId>,
    /// How many of those are compound paths of one part.
    pub left_alone: usize,
}

/// The outlines of a compound path as the kernel reads them.
pub(crate) fn kernel_triples(outlines: &[SubpathRef<'_>]) -> Vec<Vec<OutlineTriple>> {
    outlines
        .iter()
        .map(|outline| {
            outline
                .anchors
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect()
        })
        .collect()
}

/// Closed views of `triples`, one per outline of `outlines`.
pub(crate) fn kernel_views<'a>(
    triples: &'a [Vec<OutlineTriple>],
    outlines: &[SubpathRef<'_>],
) -> Vec<Outline<'a>> {
    triples
        .iter()
        .zip(outlines)
        .map(|(points, outline)| Outline::new(points, outline.closed))
        .collect()
}

/// The nesting of the outlines of one compound path.
pub(crate) fn nesting_of(outlines: &[SubpathRef<'_>]) -> Vec<Nesting> {
    let triples = kernel_triples(outlines);
    outline_nesting(&kernel_views(&triples, outlines))
}

/// A union-find over region indices; the root of a set is its smallest member.
struct Sets(Vec<usize>);

impl Sets {
    fn new(count: usize) -> Self {
        Self((0..count).collect())
    }

    fn find(&mut self, mut item: usize) -> usize {
        while self.0[item] != item {
            self.0[item] = self.0[self.0[item]];
            item = self.0[item];
        }
        item
    }

    fn join(&mut self, a: usize, b: usize) {
        let (a, b) = (self.find(a), self.find(b));
        self.0[a.max(b)] = a.min(b);
    }
}

/// The regions of the closed outlines (an even-depth outline, then the holes directly inside it)
/// and, per outline, the index of its region (`usize::MAX` for an open outline). `None` when a
/// hole's parent has no region.
fn regions_of(
    outlines: &[SubpathRef<'_>],
    nesting: &[Nesting],
) -> Option<(Vec<Vec<usize>>, Vec<usize>)> {
    let mut regions: Vec<Vec<usize>> = Vec::new();
    let mut region_of = vec![usize::MAX; outlines.len()];
    for (index, nest) in nesting.iter().enumerate() {
        if outlines[index].closed && nest.depth % 2 == 0 {
            region_of[index] = regions.len();
            regions.push(vec![index]);
        }
    }
    for (index, nest) in nesting.iter().enumerate() {
        if outlines[index].closed && nest.depth % 2 == 1 {
            // A hole's parent is the innermost enclosing outline, normally an even one that was
            // given a region above; if not, the hole joins no region.
            if let Some(region) = nest
                .parent
                .map(|parent| region_of[parent])
                .filter(|&region| region != usize::MAX)
            {
                region_of[index] = region;
                regions[region].push(index);
            }
        }
    }
    let closed_count = outlines.iter().filter(|outline| outline.closed).count();
    (regions.iter().map(Vec::len).sum::<usize>() == closed_count).then_some((regions, region_of))
}

/// The parts of a compound path, each as indices into its outlines. The closed outlines form
/// regions (an even-depth outline, then the outlines directly inside it); regions of which an
/// outline touches or crosses an outline of another one are joined. Each open outline then joins
/// by the rule of `0054` criterion 15, first match wins: (a) the part of an outline it touches or
/// crosses (several parts touched become one), (b) the part whose closed outlines paint its first
/// node, (c) a part of its own; open outlines that touch each other end in one part. A part lists
/// its regions' outlines, then its open outlines in outline order; parts with a region come first,
/// in the order of their first region, then parts of open outlines only, in outline order. `None` when the regions do not hold
/// every closed outline (see [`plan_split`]). The flag says whether an outline is a hole (odd
/// depth).
fn parts_of(outlines: &[SubpathRef<'_>]) -> Option<(Vec<Vec<usize>>, bool)> {
    let triples = kernel_triples(outlines);
    let views = kernel_views(&triples, outlines);
    let nesting = outline_nesting(&views);
    let (regions, mut node_of) = regions_of(outlines, &nesting)?;
    let open: Vec<usize> = (0..outlines.len())
        .filter(|&index| !outlines[index].closed)
        .collect();
    // An open outline is a union-find node of its own, after the regions.
    for (k, &index) in open.iter().enumerate() {
        node_of[index] = regions.len() + k;
    }
    let mut sets = Sets::new(regions.len() + open.len());
    if regions.len() + open.len() > 1 {
        for (a, b) in touching_outlines(&views) {
            if a != b {
                sets.join(node_of[a], node_of[b]);
            }
        }
    }
    join_open_by_first_node(&mut sets, &regions, &open, &views, outlines);
    let mut parts: Vec<Vec<usize>> = Vec::new();
    let mut part_of_root = vec![usize::MAX; sets.0.len()];
    let members = regions
        .iter()
        .enumerate()
        .map(|(region, members)| (region, members.clone()))
        .chain(
            open.iter()
                .enumerate()
                .map(|(k, &index)| (regions.len() + k, vec![index])),
        );
    for (node, members) in members {
        let root = sets.find(node);
        if part_of_root[root] == usize::MAX {
            part_of_root[root] = parts.len();
            parts.push(Vec::new());
        }
        parts[part_of_root[root]].extend(members);
    }
    let has_holes = nesting
        .iter()
        .zip(outlines)
        .any(|(nest, outline)| outline.closed && nest.depth % 2 == 1);
    Some((parts, has_holes))
}

/// Rule (b) of `0054` criterion 15: an open outline whose set holds no region yet joins the part
/// whose closed outlines paint its first node (the first such part in region order).
fn join_open_by_first_node(
    sets: &mut Sets,
    regions: &[Vec<usize>],
    open: &[usize],
    views: &[Outline<'_>],
    outlines: &[SubpathRef<'_>],
) {
    if regions.is_empty() {
        return;
    }
    // The parts as they stand after the touch rule: the closed outlines of every set. Joining an
    // open outline to a part keeps the part's root (the smaller index), so this list stays valid.
    let mut part_outlines: Vec<(usize, Vec<Outline<'_>>)> = Vec::new();
    for (region, members) in regions.iter().enumerate() {
        let part = sets.find(region);
        let views_of = members.iter().map(|&member| views[member]);
        match part_outlines.iter_mut().find(|(root, _)| *root == part) {
            Some((_, list)) => list.extend(views_of),
            None => part_outlines.push((part, views_of.collect())),
        }
    }
    for (k, &index) in open.iter().enumerate() {
        let node = regions.len() + k;
        if sets.find(node) < regions.len() {
            continue;
        }
        let Some(first) = outlines[index].anchors.first() else {
            continue;
        };
        if let Some((part, _)) = part_outlines
            .iter()
            .find(|(_, list)| contains_point_in_outlines(list, first.point))
        {
            sets.join(*part, node);
        }
    }
}

/// Plans Split over the selected objects.
///
/// # Errors
/// A [`SplitRefusal`]; nothing is outlined.
pub fn plan_split(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    minter: &mut AnchorIdMinter,
) -> Result<SplitPlan, SplitRefusal> {
    let operands = operands_in_order(objects, selection);
    let compounds = operands.iter().filter(|object| is_compound(object)).count();
    if compounds == 0 {
        return Err(SplitRefusal::NoCompound);
    }
    let offenders: Vec<NodeId> = operands
        .iter()
        .filter(|object| is_compound(object) && !in_range(object))
        .map(|object| object.id())
        .collect();
    if !offenders.is_empty() {
        return Err(SplitRefusal::OutOfRange {
            offenders,
            of: operands.len(),
        });
    }
    let mut removed = Vec::new();
    let mut pieces = Vec::new();
    let mut kept = Vec::new();
    let mut left_alone = 0;
    let mut with_holes = false;
    let mut alone_with_holes = false;
    for object in &operands {
        let ObjectSnapshot::Path(path) = object else {
            kept.push(object.id());
            continue;
        };
        if !path.is_compound() {
            kept.push(object.id());
            continue;
        }
        let outlines: Vec<SubpathRef<'_>> = path.subpaths().collect();
        // A hole whose parent has no region (outlines that touch at a vertex can make the depth
        // count differ from the enclosure chain) would be lost by a split: leave the compound
        // path as it is rather than drop geometry.
        // (Such a path is reported as "one piece", which is not quite true; leaving it alone is
        // what matters.)
        let found = parts_of(&outlines);
        let alone = match &found {
            Some((parts, has_holes)) => (parts.len() <= 1).then_some(*has_holes),
            None => Some(true),
        };
        let Some((parts, has_holes)) = found.filter(|_| alone.is_none()) else {
            alone_with_holes |= alone.unwrap_or(false);
            kept.push(object.id());
            left_alone += 1;
            continue;
        };
        // A part of several touching shapes has several outlines and no hole.
        with_holes |= has_holes;
        removed.push(object.id());
        for part in parts {
            let piece = part
                .iter()
                .map(|&index| {
                    fresh_outline(outlines[index].anchors, outlines[index].closed, minter)
                })
                .collect();
            pieces.push((object.id(), piece));
        }
    }
    if removed.is_empty() {
        return Err(SplitRefusal::OnePiece {
            compounds,
            with_holes: alone_with_holes,
        });
    }
    Ok(SplitPlan {
        removed,
        pieces,
        with_holes,
        kept,
        left_alone,
    })
}

/// Whether every node and handle end of `object` is finite and within the kernel's range (the
/// rule of the Boolean operations, Combine and Cut).
pub(crate) fn in_range(object: &ObjectSnapshot) -> bool {
    match object {
        ObjectSnapshot::Path(path)
        | ObjectSnapshot::Clone(curvyo_document_core::CloneSnapshot { resolved: path, .. }) => path
            .subpaths()
            .all(|outline| !out_of_range(outline.anchors)),
        ObjectSnapshot::Primitive(_) => true,
    }
}

/// The anchors of an outline with fresh ids, as one outline of a piece.
pub(crate) fn fresh_outline(
    anchors: &[NewAnchor],
    closed: bool,
    minter: &mut AnchorIdMinter,
) -> (Vec<NewAnchor>, bool) {
    (fresh_ids(anchors.to_vec(), minter), closed)
}
