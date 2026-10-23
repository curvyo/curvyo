//! Planning Fracture on the selected objects (`specs/0037-fracture-and-flatten` criteria 6 to 9).
//!
//! Pure: it reads snapshots and the selection and returns a plan or a refusal. The one write
//! (`Document::replace_with_pieces`) is the session's; refusals carry codes, counts and the ids
//! of the offending objects, the sentences are the host's.

use curvyo_document_core::ObjectSnapshot;
use curvyo_geometry_core::fracture_cells;

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::BOOLEAN_TOLERANCE;
use crate::object_selection::ObjectSelection;
use crate::stack_plan::{MAX_STACK_PIECES, Stack, StackPlan, StackRefusal, piece_outlines};

/// The commit label of Fracture; the undo slice maps it to display text.
pub const FRACTURE_COMMIT_LABEL: &str = "fracture";

/// Plans Fracture over the selected objects: every operand that overlaps another is replaced by
/// the cells it owns, one object per connected piece; the others stay.
///
/// # Errors
/// A [`StackRefusal`]; a group in the selection is the session's check.
pub fn plan_fracture(
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    minter: &mut AnchorIdMinter,
) -> Result<StackPlan, StackRefusal> {
    // Texts take the one conversion of `specs/0074-text-objects` criteria 22 and 24.
    if let Some(refusal) = crate::text_convert::area_refusal(objects, selection.ids()) {
        return Err(StackRefusal::Operands(
            crate::boolean::BooleanRefusal::Texts(refusal),
        ));
    }
    let converted = crate::text_convert::texts_as_paths(objects);
    let stack = Stack::of(&converted, selection)?;
    let fracture =
        stack.run(|operands| fracture_cells(operands, BOOLEAN_TOLERANCE, MAX_STACK_PIECES))?;
    if fracture.pieces.is_empty() {
        return Err(StackRefusal::NoOverlap);
    }
    let mut plan = stack.plan();
    for (index, object) in stack.operands.iter().enumerate() {
        if fracture.untouched.contains(&index) {
            plan.kept.push(object.id());
        } else {
            plan.removed.push(object.id());
        }
    }
    plan.pieces = fracture
        .pieces
        .iter()
        .map(|(owner, outlines)| {
            (
                stack.operands[*owner].id(),
                piece_outlines(outlines, minter),
            )
        })
        .collect();
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds};

    fn rect(document: &Document, x: f64, y: f64, side: f64) -> NodeId {
        document.create_rect(RectBounds {
            origin: Point::new(x, y),
            width: Length::from_mm(side),
            height: Length::from_mm(side),
        })
    }

    fn selection_of(ids: &[NodeId]) -> ObjectSelection {
        let mut selection = ObjectSelection::new();
        for &id in ids {
            selection.add(id);
        }
        selection
    }

    fn objects(document: &Document) -> Vec<ObjectSnapshot> {
        document
            .object_ids()
            .into_iter()
            .filter_map(|id| document.object(id))
            .collect()
    }

    /// Criteria 6, 8 and 16: pieces take their owner by stacking order whatever the selection
    /// order; an operand apart stays.
    #[test]
    fn pieces_belong_to_the_topmost_cover_and_an_operand_apart_stays() {
        let document = Document::new(1);
        let low = rect(&document, 0.0, 0.0, 20.0);
        let high = rect(&document, 10.0, 10.0, 20.0);
        let apart = rect(&document, 100.0, 0.0, 5.0);
        let all = objects(&document);
        let plan = plan_fracture(
            &all,
            &selection_of(&[apart, high, low]),
            &mut AnchorIdMinter::new(5),
        )
        .unwrap();
        let again = plan_fracture(
            &all,
            &selection_of(&[low, high, apart]),
            &mut AnchorIdMinter::new(5),
        )
        .unwrap();
        assert_eq!(plan, again);
        assert_eq!(plan.removed, vec![low, high]);
        assert_eq!(plan.kept, vec![apart]);
        let owners: Vec<NodeId> = plan.pieces.iter().map(|piece| piece.0).collect();
        assert_eq!(owners, vec![low, high, high]);
    }

    /// Criterion 3: shapes that only touch are refused.
    #[test]
    fn touching_shapes_do_not_overlap() {
        let document = Document::new(1);
        let a = rect(&document, 0.0, 0.0, 10.0);
        let b = rect(&document, 10.0, 0.0, 10.0);
        assert_eq!(
            plan_fracture(
                &objects(&document),
                &selection_of(&[a, b]),
                &mut AnchorIdMinter::new(5)
            ),
            Err(StackRefusal::NoOverlap)
        );
    }
}
