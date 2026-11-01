//! The commit side of `specs/0061-align-and-distribute/`: every unit keeps
//! its own translation even when a unit has no leaf in the objects read.

use curvyo_document_core::{Document, Length, NodeId, ObjectSnapshot, Point, RectBounds};
use curvyo_ui_core::{
    AlignBox, AlignCommand, AlignInput, AlignOutcome, ObjectSelection, SelectedUnit, Units,
    align_page, object_outline_bounds,
};

fn square(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(10.0),
        height: Length::from_mm(10.0),
    })
}

fn leaf(id: NodeId) -> SelectedUnit {
    SelectedUnit {
        id,
        is_group: false,
        leaves: vec![id],
    }
}

fn left_of(document: &Document, id: NodeId) -> Option<f64> {
    let object = document.object(id)?;
    Some(object_outline_bounds(&object).0.x)
}

/// Review change 2: a unit whose leaves are not in `objects` is skipped
/// without shifting the later units' translations onto the wrong object.
#[test]
fn a_unit_without_leaves_in_the_objects_does_not_shift_the_others() {
    let document = Document::new(1);
    let ghost = square(&document, 50.0);
    let a = square(&document, 0.0);
    let b = square(&document, 20.0);
    let objects: Vec<ObjectSnapshot> = [a, b]
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect();
    let units = Units::build(&document, None);
    let selection = ObjectSelection::new();
    let input = AlignInput {
        document: &document,
        objects: &objects,
        units: &units,
        selection: &selection,
    };
    let page = AlignBox::new(Point::new(100.0, 0.0), Point::new(300.0, 200.0));
    let outcome = align_page(
        &input,
        &[leaf(ghost), leaf(a), leaf(b)],
        page,
        AlignCommand::Left,
    );
    assert_eq!(outcome, AlignOutcome::Aligned);
    for (id, want) in [(a, 100.0), (b, 100.0), (ghost, 50.0)] {
        let left = left_of(&document, id).unwrap_or(f64::NAN);
        assert!((left - want).abs() < 1e-6, "{id:?}: {left}");
    }
}
