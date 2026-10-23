//! Planning Flatten on the selected objects: every operand is trimmed to the part no operand
//! above covers, removed when nothing is left, and kept when nothing above overlaps it
//! (`specs/0037-fracture-and-flatten` criteria 10 to 14; not the curve flattening of the kernel).
//!
//! Pure, like [`crate::plan_fracture`]; both build on the shared operand stack.

use curvyo_document_core::ObjectSnapshot;
use curvyo_geometry_core::{Visible, visible_regions};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::boolean::BOOLEAN_TOLERANCE;
use crate::object_selection::ObjectSelection;
use crate::stack_plan::{MAX_STACK_PIECES, Stack, StackPlan, StackRefusal, piece_outlines};

/// The commit label of Flatten; the undo slice maps it to display text.
pub const FLATTEN_COMMIT_LABEL: &str = "flatten";

/// Plans Flatten over the selected objects.
///
/// # Errors
/// A [`StackRefusal`]; a group in the selection is the session's check.
pub fn plan_flatten(
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
    let visible = stack.run(|operands| visible_regions(operands, BOOLEAN_TOLERANCE))?;
    let mut plan = stack.plan();
    for (object, entry) in stack.operands.iter().zip(&visible) {
        match entry {
            Visible::Untouched => plan.kept.push(object.id()),
            Visible::Trimmed(outlines) => {
                plan.removed.push(object.id());
                plan.pieces
                    .push((object.id(), piece_outlines(outlines, minter)));
                plan.strokes_follow |= object.style().stroke.enabled;
            }
            Visible::Hidden => {
                plan.removed.push(object.id());
                plan.hidden += 1;
            }
        }
    }
    if plan.removed.is_empty() {
        return Err(StackRefusal::NoOverlap);
    }
    if plan.pieces.len() > MAX_STACK_PIECES {
        return Err(StackRefusal::TooManyPieces);
    }
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

    /// Criteria 10, 11 and 13: the middle square is hidden, the bottom one trimmed with its
    /// stroke following the trim, the top one kept.
    #[test]
    fn a_stack_is_trimmed_hidden_and_kept() {
        let document = Document::new(1);
        let bottom = rect(&document, 0.0, 0.0, 20.0);
        let middle = rect(&document, 12.0, 12.0, 5.0);
        let top = rect(&document, 10.0, 10.0, 20.0);
        let objects: Vec<ObjectSnapshot> = document
            .object_ids()
            .into_iter()
            .filter_map(|id| document.object(id))
            .collect();
        let mut selection = ObjectSelection::new();
        for id in [bottom, middle, top] {
            selection.add(id);
        }
        let plan = plan_flatten(&objects, &selection, &mut AnchorIdMinter::new(5)).unwrap();
        assert_eq!(plan.removed, vec![bottom, middle]);
        assert_eq!(plan.kept, vec![top]);
        assert_eq!(plan.hidden, 1);
        assert_eq!(plan.pieces.len(), 1);
        assert_eq!(plan.pieces[0].1[0].0.len(), 6, "the L");
        assert!(plan.strokes_follow, "a new rectangle has a stroke");
    }
}
