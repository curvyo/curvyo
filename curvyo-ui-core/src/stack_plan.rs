//! The planning ground shared by both stack commands (`specs/0037-fracture-and-flatten`
//! criteria 1 to 4): the operand stack, its refusals, its plan.
//!
//! Pure: it reads snapshots and the selection; the sentences of a refusal are the host's.

use curvyo_document_core::{NodeId, ObjectSnapshot, PieceOutlines, Point};
use curvyo_geometry_core::{Outline, OutlineTriple, StackError};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::{
    BooleanRefusal, corner_anchors, open_counts, operands_in_order, outlines_of, refusal_of,
};
use crate::object_selection::ObjectSelection;

/// The most objects Fracture or Flatten may make in one step (criterion 4).
pub const MAX_STACK_PIECES: usize = 5_000;

/// Why Fracture or Flatten was refused. Nothing was changed in any of these cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackRefusal {
    /// The refusals the Boolean operations share: fewer than two objects, no area, out of range,
    /// groups (criterion 2).
    Operands(BooleanRefusal),
    /// No two operands overlap in area (criterion 3).
    NoOverlap,
    /// The result would have more than [`MAX_STACK_PIECES`] new objects (criterion 4).
    TooManyPieces,
}

/// What a Fracture or Flatten plan decided, ready for `Document::replace_with_pieces`.
#[derive(Debug, Clone, PartialEq)]
pub struct StackPlan {
    /// The operands that are replaced or removed, bottom-most first.
    pub removed: Vec<NodeId>,
    /// The new objects, each with the operand whose place and style it takes; the pieces of one
    /// operand are consecutive, ordered by their first node (x, then y), with fresh anchor ids.
    pub pieces: Vec<(NodeId, PieceOutlines)>,
    /// The operands left exactly as they are, bottom-most first.
    pub kept: Vec<NodeId>,
    /// How many objects were selected.
    pub operands: usize,
    /// Flatten: how many operands were covered completely and removed.
    pub hidden: usize,
    /// Flatten: whether a trimmed operand has a stroke, which now follows the trim (criterion 13).
    pub strokes_follow: bool,
    /// How many open outlines were closed with a straight line (`0054` criterion 21).
    pub closed_open: usize,
    /// How many open outlines enclosed no area and were removed (`0054` criterion 22).
    pub removed_open: usize,
}

/// The selected objects in stacking order with their outlines, as both commands give them to the
/// kernel.
pub(crate) struct Stack<'a> {
    pub(crate) operands: Vec<&'a ObjectSnapshot>,
    owned: Vec<Vec<(Vec<OutlineTriple>, bool)>>,
}

impl<'a> Stack<'a> {
    /// The stack of the selected objects; refused with fewer than two (criterion 1).
    pub(crate) fn of(
        objects: &'a [ObjectSnapshot],
        selection: &ObjectSelection,
    ) -> Result<Self, StackRefusal> {
        let operands = operands_in_order(objects, selection);
        if operands.len() < 2 {
            return Err(StackRefusal::Operands(BooleanRefusal::NeedsTwo));
        }
        let owned = operands.iter().map(|object| outlines_of(object)).collect();
        Ok(Self { operands, owned })
    }

    /// Runs `kernel` on the operands' outlines, bottom first, and maps a kernel refusal.
    pub(crate) fn run<T>(
        &self,
        kernel: impl FnOnce(&[&[Outline<'_>]]) -> Result<T, StackError>,
    ) -> Result<T, StackRefusal> {
        let borrowed: Vec<Vec<Outline<'_>>> = self
            .owned
            .iter()
            .map(|outlines| {
                outlines
                    .iter()
                    .map(|(anchors, closed)| Outline::new(anchors, *closed))
                    .collect()
            })
            .collect();
        let slices: Vec<&[Outline<'_>]> = borrowed.iter().map(Vec::as_slice).collect();
        kernel(&slices).map_err(|error| match error {
            StackError::Operands(error) => {
                StackRefusal::Operands(refusal_of(error, &self.operands, self.operands.len()))
            }
            StackError::TooManyPieces => StackRefusal::TooManyPieces,
        })
    }

    /// An empty plan for this stack, with the open-outline counts.
    pub(crate) fn plan(&self) -> StackPlan {
        let (closed_open, removed_open) = open_counts(&self.owned);
        StackPlan {
            removed: Vec::new(),
            pieces: Vec::new(),
            kept: Vec::new(),
            operands: self.operands.len(),
            hidden: 0,
            strokes_follow: false,
            closed_open,
            removed_open,
        }
    }
}

/// The outlines of a piece as corner anchors with fresh ids, all closed.
pub(crate) fn piece_outlines(
    outlines: &[Vec<Point>],
    minter: &mut AnchorIdMinter,
) -> PieceOutlines {
    outlines
        .iter()
        .map(|points| (corner_anchors(points, minter), true))
        .collect()
}
