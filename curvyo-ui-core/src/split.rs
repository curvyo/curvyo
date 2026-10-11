//! Planning Split on the selected compound paths: which outlines form a part (a shape with the
//! holes directly inside it, joined with every part whose outline touches it) and what the pieces
//! are (`specs/0048-split-compound-path`, criteria 3 to 10).
//!
//! Pure: it reads snapshots and the selection and returns a plan or a refusal. The one write
//! (`Document::replace_with_pieces`) is the session's. Nodes are copied as they are, no flattening
//! and no reversal: the windings of a compound path are canonical already.

use curvyo_document_core::{NewAnchor, NodeId, ObjectSnapshot, PieceOutlines, SubpathRef};
use curvyo_geometry_core::{Nesting, Outline, OutlineTriple, outline_nesting, touching_outlines};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::operands_in_order;
use crate::combine::{fresh_ids, is_compound};
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
    /// Every selected compound path is one part: `compounds` of them, each already one piece with
    /// its holes.
    OnePiece {
        /// How many compound paths are selected.
        compounds: usize,
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

/// The parts of a compound path, each as indices into its outlines: the regions (an even-depth
/// outline, then the outlines directly inside it) of which an outline touches or crosses an
/// outline of another one, joined. Ordered by the first region of each part. `None` when the
/// regions do not hold every outline (see [`plan_split`]).
fn parts_of(outlines: &[SubpathRef<'_>]) -> Option<Vec<Vec<usize>>> {
    let triples = kernel_triples(outlines);
    let views = kernel_views(&triples, outlines);
    let nesting = outline_nesting(&views);
    let mut regions: Vec<Vec<usize>> = Vec::new();
    let mut region_of = vec![usize::MAX; outlines.len()];
    for (index, nest) in nesting.iter().enumerate() {
        if nest.depth % 2 == 0 {
            region_of[index] = regions.len();
            regions.push(vec![index]);
        }
    }
    for (index, nest) in nesting.iter().enumerate() {
        if nest.depth % 2 == 1 {
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
    if regions.iter().map(Vec::len).sum::<usize>() != outlines.len() {
        return None;
    }
    let mut sets = Sets::new(regions.len());
    if regions.len() > 1 {
        for (a, b) in touching_outlines(&views) {
            if a != b {
                sets.join(region_of[a], region_of[b]);
            }
        }
    }
    let mut parts: Vec<Vec<usize>> = Vec::new();
    let mut part_of_root = vec![usize::MAX; regions.len()];
    for (region, members) in regions.iter().enumerate() {
        let root = sets.find(region);
        if part_of_root[root] == usize::MAX {
            part_of_root[root] = parts.len();
            parts.push(Vec::new());
        }
        parts[part_of_root[root]].extend(members);
    }
    Some(parts)
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
    let mut removed = Vec::new();
    let mut pieces = Vec::new();
    let mut kept = Vec::new();
    let mut left_alone = 0;
    let mut with_holes = false;
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
        let parts = parts_of(&outlines).filter(|parts| parts.len() > 1);
        let Some(parts) = parts else {
            kept.push(object.id());
            left_alone += 1;
            continue;
        };
        with_holes |= parts.iter().any(|part| part.len() > 1);
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
        return Err(SplitRefusal::OnePiece { compounds });
    }
    Ok(SplitPlan {
        removed,
        pieces,
        with_holes,
        kept,
        left_alone,
    })
}

/// The anchors of an outline with fresh ids, as one outline of a piece.
pub(crate) fn fresh_outline(
    anchors: &[NewAnchor],
    closed: bool,
    minter: &mut AnchorIdMinter,
) -> (Vec<NewAnchor>, bool) {
    (fresh_ids(anchors.to_vec(), minter), closed)
}
