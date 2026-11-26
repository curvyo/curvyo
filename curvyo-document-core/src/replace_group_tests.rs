//! `replace_with_group` against `specs/0156-fused-lettering` criteria 11, 14 and 20.

use super::*;
use crate::{DocumentSize, Length, NewOperation, NewOutputPage, Point, RectBounds, Style};

fn rect(document: &Document, x: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, 0.0),
        width: Length::from_mm(2.0),
        height: Length::from_mm(2.0),
    })
}

fn square(id: u64, x: f64) -> PieceOutlines {
    let anchors = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .iter()
        .zip(0..)
        .map(|(&(dx, dy), k)| {
            NewAnchor::corner(AnchorId::new(9, id * 10 + k), Point::new(x + dx, dy))
        })
        .collect();
    vec![(anchors, true)]
}

fn name(document: &Document, id: NodeId) -> Option<String> {
    document.node_registers(id).and_then(|r| r.name)
}

/// Criteria 11 and 14: one step, the group at the top-most operand's place with Outline above
/// Bridges, names written, the Outline a member of the given operation, the operands gone.
#[test]
fn the_group_replaces_the_operands_in_one_step() {
    let document = Document::new(1);
    let laser = document
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap()
        .kind
        .manufacturing()
        .unwrap();
    let cut = document
        .add_operation(
            laser,
            &NewOperation {
                kind: "cut",
                name: "Cut 1",
                settings: None,
            },
            0,
            &[],
        )
        .unwrap();
    let below = rect(&document, -10.0);
    let a = rect(&document, 0.0);
    let b = rect(&document, 5.0);
    let above = rect(&document, 20.0);
    let steps = document.own_step_count();
    let outline = square(1, 0.0);
    let parts = vec![
        ("Bridge 1".to_owned(), square(2, 2.0)),
        ("Bridge 2".to_owned(), square(3, 4.0)),
    ];
    let ids = document
        .replace_with_group(
            &[a, b],
            b,
            &NewGroup {
                name: "Bridged lettering",
                style: &Style::default(),
                outline: ("Outline", &outline),
                operations: &[cut],
                parts_name: "Bridges",
                parts: &parts,
            },
            "fuse",
        )
        .unwrap();
    assert_eq!(document.own_step_count(), steps + 1);
    assert_eq!(document.own_steps().last().unwrap().label, "fuse");
    assert_eq!(document.children(None), vec![below, ids.group, above]);
    assert_eq!(
        document.children(Some(ids.group)),
        vec![ids.parts, ids.outline]
    );
    assert_eq!(document.children(Some(ids.parts)), ids.part_paths);
    assert_eq!(
        name(&document, ids.group).as_deref(),
        Some("Bridged lettering")
    );
    assert_eq!(name(&document, ids.outline).as_deref(), Some("Outline"));
    assert_eq!(name(&document, ids.parts).as_deref(), Some("Bridges"));
    assert_eq!(
        name(&document, ids.part_paths[1]).as_deref(),
        Some("Bridge 2")
    );
    assert!(document.object(a).is_none() && document.object(b).is_none());
    let op = document.operations(laser).unwrap().remove(0);
    assert_eq!(op.members, vec![vec![ids.outline]]);
}

#[test]
fn a_place_that_is_not_removed_is_refused_and_nothing_changes() {
    let document = Document::new(1);
    let a = rect(&document, 0.0);
    let other = rect(&document, 5.0);
    let steps = document.own_step_count();
    let outline = square(1, 0.0);
    let result = document.replace_with_group(
        &[a],
        other,
        &NewGroup {
            name: "Bridged lettering",
            style: &Style::default(),
            outline: ("Outline", &outline),
            operations: &[],
            parts_name: "Bridges",
            parts: &[],
        },
        "fuse",
    );
    assert_eq!(result, Err(ObjectEditError::BaseNotAnOperand));
    assert_eq!(document.own_step_count(), steps);
    assert!(document.object(a).is_some());
}
