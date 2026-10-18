//! Planning Break apart on the selected compound paths: every outline becomes a path of its own,
//! open or closed as it was, holes included (`specs/0048-split-compound-path`, criteria 11 to 13;
//! `specs/0054-combine-open-paths` criterion 18).
//!
//! Pure: it reads snapshots and the selection and returns a plan or a refusal. The one write
//! (`Document::replace_with_pieces`) is the session's. Nodes are copied as they are, not reversed:
//! a released hole keeps its negative winding, which paints the same under the nonzero rule.

use curvyo_document_core::{NodeId, ObjectSnapshot, PieceOutlines, SubpathRef};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::operands_in_order;
use crate::combine::is_compound;
use crate::object_selection::ObjectSelection;
use crate::split::{fresh_outline, in_range, nesting_of};

/// The commit label of Break apart; the undo slice maps it to display text.
pub const BREAK_APART_COMMIT_LABEL: &str = "break_apart";

/// Why Break apart was refused. Nothing was changed in either case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreakApartRefusal {
    /// The selection holds no compound path (the button is dimmed then; reachable only through
    /// the session function).
    NoCompound,
    /// The selection holds a group; Break apart does not look through groups
    /// (`specs/0023-groups` criterion 28). `offenders` are the leaves of the groups, to outline.
    Groups {
        /// The leaves of the selected groups, to outline.
        offenders: Vec<NodeId>,
    },
}

/// What [`plan_break_apart`] decided, ready for `Document::replace_with_pieces`.
#[derive(Debug, Clone, PartialEq)]
pub struct BreakApartPlan {
    /// The compound paths to replace, bottom-most first.
    pub removed: Vec<NodeId>,
    /// One path per outline of two or more nodes, open or closed as the outline was, each with
    /// the compound path it takes its place and style from, in the compound path's outline order,
    /// with fresh anchor ids.
    pub pieces: Vec<(NodeId, PieceOutlines)>,
    /// Whether an outline was a hole (odd depth), which is now a filled shape.
    pub with_holes: bool,
    /// The selected objects that were no compound path and stay as they are.
    pub kept: Vec<NodeId>,
    /// How many outlines with a single node were left out (`0054` criterion 18).
    pub single_nodes_removed: usize,
}

/// Plans Break apart over the selected objects.
///
/// # Errors
/// A [`BreakApartRefusal`]; nothing is outlined.
pub fn plan_break_apart(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    minter: &mut AnchorIdMinter,
) -> Result<BreakApartPlan, BreakApartRefusal> {
    let operands = operands_in_order(objects, selection);
    if !operands.iter().any(|object| is_compound(object)) {
        return Err(BreakApartRefusal::NoCompound);
    }
    let mut removed = Vec::new();
    let mut pieces = Vec::new();
    let mut kept = Vec::new();
    let mut with_holes = false;
    let mut single_nodes_removed = 0;
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
        // The hole flag needs the nesting, which cannot measure a coordinate out of range; such a
        // compound path is still broken apart (no arithmetic), only the flag is not set for it.
        // Open outlines have no depth; only the closed ones are measured.
        let closed: Vec<SubpathRef<'_>> = outlines.iter().copied().filter(|o| o.closed).collect();
        if in_range(object) {
            with_holes |= nesting_of(&closed).iter().any(|nest| nest.depth % 2 == 1);
        }
        removed.push(object.id());
        for outline in &outlines {
            if outline.anchors.len() < 2 {
                single_nodes_removed += 1;
                continue;
            }
            let piece = vec![fresh_outline(outline.anchors, outline.closed, minter)];
            pieces.push((object.id(), piece));
        }
    }
    Ok(BreakApartPlan {
        removed,
        pieces,
        with_holes,
        kept,
        single_nodes_removed,
    })
}
