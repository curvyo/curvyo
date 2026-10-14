//! When Linked clone is offered and what it does to a selection (`specs/0049-linked-clone`
//! criteria 1 to 4 and 52): the sources, the conversions of the shapes among them, and the
//! refusal of clones that are already eight levels deep.

use curvyo_document_core::{Document, MAX_CLONE_DEPTH, NewAnchor, NodeId, ObjectSnapshot};

use crate::anchor_id_minter::AnchorIdMinter;
use crate::conversion::build_primitive_conversions;
use crate::object_selection::{ObjectSelection, objects_with_ids};
use crate::units::Units;

/// At most this many refused clones are outlined (criterion 52).
pub const MAX_REFUSAL_OUTLINES: usize = 200;

/// Whether the Linked clone button is in the Select bar: the selection holds at least one
/// object and no group (criterion 4).
#[must_use]
pub fn linked_clone_shown(selection: &ObjectSelection, units: &Units) -> bool {
    !selection.ids().is_empty()
        && !units
            .selected(selection.ids())
            .iter()
            .any(|unit| unit.is_group)
}

/// What Linked clone on the selection does.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkedClonePlan {
    /// Make a clone of each source, converting these shapes first.
    Ready {
        /// The selected objects, in selection order.
        sources: Vec<NodeId>,
        /// The "Object to path" outline of each shape among them, with minted anchor ids.
        conversions: Vec<(NodeId, Vec<NewAnchor>)>,
    },
    /// Refused: these selected clones are already [`MAX_CLONE_DEPTH`] deep (the first
    /// [`MAX_REFUSAL_OUTLINES`], for the refusal outline). Nothing is created or converted.
    TooDeep(Vec<NodeId>),
}

/// The plan of Linked clone for `selection`, `None` when the button is not offered.
#[must_use]
pub fn plan_linked_clone(
    document: &Document,
    objects: &[ObjectSnapshot],
    selection: &ObjectSelection,
    units: &Units,
    minter: &mut AnchorIdMinter,
) -> Option<LinkedClonePlan> {
    if !linked_clone_shown(selection, units) {
        return None;
    }
    let held = objects_with_ids(objects, selection.ids());
    let too_deep: Vec<NodeId> = held
        .iter()
        .filter_map(|object| match object {
            ObjectSnapshot::Clone(clone) if clone.chain.depth >= MAX_CLONE_DEPTH => {
                Some(clone.id())
            }
            _ => None,
        })
        .take(MAX_REFUSAL_OUTLINES)
        .collect();
    if !too_deep.is_empty() {
        return Some(LinkedClonePlan::TooDeep(too_deep));
    }
    let sources: Vec<NodeId> = held.iter().map(|object| object.id()).collect();
    let shapes: Vec<NodeId> = held
        .iter()
        .filter(|object| matches!(object, ObjectSnapshot::Primitive(_)))
        .map(|object| object.id())
        .collect();
    let conversions = build_primitive_conversions(document, minter, &shapes);
    Some(LinkedClonePlan::Ready {
        sources,
        conversions,
    })
}

#[cfg(test)]
mod tests {
    use curvyo_document_core::{Length, Point, RectBounds};

    use super::*;

    #[test]
    fn a_rectangle_comes_with_its_conversion_and_a_deep_clone_is_refused() {
        let d = Document::new(1);
        let r = d.create_rect(RectBounds {
            origin: Point::new(0.0, 0.0),
            width: Length::from_mm(10.0),
            height: Length::from_mm(5.0),
        });
        let objects = d.read_objects().objects;
        let mut selection = ObjectSelection::default();
        selection.select_single(r);
        let mut minter = AnchorIdMinter::new(1);
        let Some(LinkedClonePlan::Ready {
            sources,
            conversions,
        }) = plan_linked_clone(&d, &objects, &selection, &Units::default(), &mut minter)
        else {
            panic!("offered")
        };
        assert_eq!(sources, vec![r]);
        assert_eq!(conversions.len(), 1);
        let first = d.linked_clone(&sources, &conversions).unwrap()[0];
        let mut deepest = first;
        for _ in 1..MAX_CLONE_DEPTH {
            deepest = d.linked_clone(&[deepest], &[]).unwrap()[0];
        }
        let objects = d.read_objects().objects;
        selection.select_single(deepest);
        assert_eq!(
            plan_linked_clone(&d, &objects, &selection, &Units::default(), &mut minter),
            Some(LinkedClonePlan::TooDeep(vec![deepest]))
        );
    }

    #[test]
    fn nothing_selected_offers_nothing() {
        assert!(!linked_clone_shown(
            &ObjectSelection::default(),
            &Units::default()
        ));
    }
}
