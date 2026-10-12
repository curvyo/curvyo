//! Split and Break apart on the inputs E1 to E6 of `specs/0048-split-compound-path` (criteria 1,
//! 3, 4, 5, 7, 11 and 13): which parts a compound path has, what each command makes of it.

#![allow(clippy::unwrap_used, clippy::float_cmp, clippy::cast_precision_loss)]

use curvyo_document_core::{
    AnchorId, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point,
};
use curvyo_ui_core::{
    AnchorIdMinter, BreakApartRefusal, ObjectSelection, SplitRefusal, path_availability,
    plan_break_apart, plan_combine, plan_split,
};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn disc(document: &Document, x: f64, y: f64, radius: f64) -> NodeId {
    document.create_ellipse(EllipseFrame {
        center: pt(x, y),
        rx: Length::from_mm(radius),
        ry: Length::from_mm(radius),
    })
}

fn objects(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn select(ids: &[NodeId]) -> ObjectSelection {
    let mut selection = ObjectSelection::new();
    for &id in ids {
        selection.add(id);
    }
    selection
}

fn minter() -> AnchorIdMinter {
    AnchorIdMinter::new(77)
}

/// Combines `ids` (which must not touch) into one compound path.
fn combine(document: &Document, ids: &[NodeId]) -> NodeId {
    let all = objects(document);
    let plan = plan_combine(&all, &select(ids), &mut AnchorIdMinter::new(60)).unwrap();
    document
        .replace_with_path(&plan.operands, plan.base, &plan.outlines, "combine_paths")
        .unwrap()
}

/// A compound path of the given closed squares `(x, y, side)`, written as another program could
/// write it (no check for touching).
fn squares(document: &Document, first: u64, squares: &[(f64, f64, f64)]) -> NodeId {
    let base = disc(document, 500.0, 500.0, 1.0);
    let outlines: Vec<(Vec<NewAnchor>, bool)> = squares
        .iter()
        .enumerate()
        .map(|(n, &(x, y, side))| {
            let corners = [(x, y), (x + side, y), (x + side, y + side), (x, y + side)];
            let anchors = corners
                .iter()
                .enumerate()
                .map(|(k, &(cx, cy))| {
                    NewAnchor::corner(
                        AnchorId::new(8, first + 10 * n as u64 + k as u64),
                        pt(cx, cy),
                    )
                })
                .collect();
            (anchors, true)
        })
        .collect();
    document
        .replace_with_path(&[base], base, &outlines, "combine_paths")
        .unwrap()
}

/// E1: a ring is one part. Split refuses, Break apart makes two closed paths.
#[test]
fn e1_a_ring() {
    let document = Document::new(1);
    let outer = disc(&document, 0.0, 0.0, 20.0);
    let inner = disc(&document, 0.0, 0.0, 10.0);
    let ring = combine(&document, &[outer, inner]);
    let all = objects(&document);

    assert_eq!(
        plan_split(&all, &select(&[ring]), &mut minter()),
        Err(SplitRefusal::OnePiece { compounds: 1 })
    );
    let broken = plan_break_apart(&all, &select(&[ring]), &mut minter()).unwrap();
    assert_eq!(broken.removed, vec![ring]);
    assert_eq!(broken.pieces.len(), 2);
    assert!(
        broken
            .pieces
            .iter()
            .all(|(_, piece)| piece.len() == 1 && piece[0].1)
    );
    assert!(broken.with_holes, "the hole becomes a filled shape");
}

/// E2: two circles in one compound path are two parts and two outlines.
#[test]
fn e2_two_separate_circles() {
    let document = Document::new(1);
    let a = disc(&document, 0.0, 0.0, 10.0);
    let b = disc(&document, 50.0, 0.0, 10.0);
    let both = combine(&document, &[a, b]);
    let all = objects(&document);

    let split = plan_split(&all, &select(&[both]), &mut minter()).unwrap();
    assert_eq!(split.pieces.len(), 2);
    assert!(!split.with_holes);
    let broken = plan_break_apart(&all, &select(&[both]), &mut minter()).unwrap();
    assert_eq!(broken.pieces.len(), 2);
    assert!(!broken.with_holes, "no hole was released");
}

/// E3 and E6: ordinary objects dim Split and Break apart and enable Cut.
#[test]
fn e3_ordinary_objects_enable_only_cut() {
    let document = Document::new(1);
    let a = disc(&document, 0.0, 0.0, 10.0);
    let b = disc(&document, 50.0, 0.0, 10.0);
    let all = objects(&document);
    let state = path_availability(&all, &select(&[a, b]));
    assert!(!state.compound);
    assert!(state.cut);
    assert_eq!(
        plan_split(&all, &select(&[a, b]), &mut minter()),
        Err(SplitRefusal::NoCompound)
    );
    assert_eq!(
        plan_break_apart(&all, &select(&[a, b]), &mut minter()),
        Err(BreakApartRefusal::NoCompound)
    );
}

/// E5: a ring with a disc in its hole: Split gives the ring and the disc, Break apart three.
#[test]
fn e5_a_ring_with_a_disc_in_its_hole() {
    let document = Document::new(1);
    let outer = disc(&document, 0.0, 0.0, 20.0);
    let inner = disc(&document, 0.0, 0.0, 10.0);
    let island = disc(&document, 0.0, 0.0, 5.0);
    let compound = combine(&document, &[outer, inner, island]);
    let all = objects(&document);

    let split = plan_split(&all, &select(&[compound]), &mut minter()).unwrap();
    let sizes: Vec<usize> = split.pieces.iter().map(|(_, piece)| piece.len()).collect();
    assert_eq!(sizes, vec![2, 1], "the ring, then the disc");
    assert!(split.with_holes);
    assert_eq!(split.left_alone, 0);
    let broken = plan_break_apart(&all, &select(&[compound]), &mut minter()).unwrap();
    assert_eq!(broken.pieces.len(), 3);
}

/// Criterion 5: every node and handle is as before, in the same order, with new ids.
#[test]
fn pieces_keep_their_nodes_and_get_new_ids() {
    let document = Document::new(1);
    let a = disc(&document, 0.0, 0.0, 10.0);
    let b = disc(&document, 50.0, 0.0, 10.0);
    let both = combine(&document, &[a, b]);
    let all = objects(&document);
    let ObjectSnapshot::Path(path) = all.iter().find(|o| o.id() == both).unwrap() else {
        panic!("a path");
    };
    let original: Vec<_> = path.subpaths().collect();
    for plan_pieces in [
        plan_split(&all, &select(&[both]), &mut minter())
            .unwrap()
            .pieces,
        plan_break_apart(&all, &select(&[both]), &mut minter())
            .unwrap()
            .pieces,
    ] {
        for ((_, piece), old) in plan_pieces.iter().zip(&original) {
            assert_eq!(piece[0].0.len(), old.anchors.len());
            for (new, old) in piece[0].0.iter().zip(old.anchors) {
                assert_eq!(
                    (new.point, new.handle_in, new.handle_out, new.kind),
                    (old.point, old.handle_in, old.handle_out, old.kind)
                );
                assert_ne!(new.id, old.id);
            }
        }
    }
}

/// Criterion 4: regions whose outlines touch or cross are one part; two squares 0.01 mm apart
/// are two.
#[test]
fn touching_regions_are_one_part_and_near_ones_are_two() {
    let document = Document::new(1);
    let overlapping = squares(&document, 100, &[(0.0, 0.0, 10.0), (8.0, -5.0, 10.0)]);
    let apart = squares(&document, 300, &[(0.0, 0.0, 10.0), (10.01, 0.0, 10.0)]);
    let all = objects(&document);

    assert_eq!(
        plan_split(&all, &select(&[overlapping]), &mut minter()),
        Err(SplitRefusal::OnePiece { compounds: 1 })
    );
    assert_eq!(
        plan_split(&all, &select(&[apart]), &mut minter())
            .unwrap()
            .pieces
            .len(),
        2
    );
    // Break apart does not care about touching.
    assert_eq!(
        plan_break_apart(&all, &select(&[overlapping]), &mut minter())
            .unwrap()
            .pieces
            .len(),
        2
    );
}

/// Criterion 3: parts come in the order of their first outline; a touching chain joins parts in
/// the middle without reordering the others.
#[test]
fn parts_are_ordered_by_their_first_outline() {
    let document = Document::new(1);
    let compound = squares(
        &document,
        100,
        &[
            (0.0, 0.0, 10.0),
            (50.0, 0.0, 10.0),
            (8.0, -5.0, 10.0),
            (100.0, 0.0, 10.0),
        ],
    );
    let all = objects(&document);
    let split = plan_split(&all, &select(&[compound]), &mut minter()).unwrap();
    let sizes: Vec<usize> = split.pieces.iter().map(|(_, piece)| piece.len()).collect();
    assert_eq!(
        sizes,
        vec![2, 1, 1],
        "squares 0 and 2 touch and are one part"
    );
    assert_eq!(split.pieces[0].1[0].0[0].point, pt(0.0, 0.0));
    assert_eq!(split.pieces[1].1[0].0[0].point, pt(50.0, 0.0));
    assert_eq!(split.pieces[2].1[0].0[0].point, pt(100.0, 0.0));
}
