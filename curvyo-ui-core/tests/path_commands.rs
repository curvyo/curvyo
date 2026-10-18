//! The pure part of Combine, Split and Break apart (`specs/0035-combine-and-break-apart` criteria
//! 1, 3 to 6, 8 to 11; `specs/0048-split-compound-path` criteria 1, 3 to 7, 11 to 13):
//! availability, plans and refusals.

#![allow(clippy::unwrap_used, clippy::float_cmp, clippy::cast_precision_loss)]

use curvyo_document_core::{
    AnchorId, Document, EllipseFrame, Length, NewAnchor, NodeId, ObjectSnapshot, Point, RectBounds,
    StyleEdit,
};
use curvyo_ui_core::{
    AnchorIdMinter, BooleanAvailability, BreakApartRefusal, CombineRefusal, ObjectSelection,
    SplitRefusal, path_availability, plan_break_apart, plan_combine, plan_split,
};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn rect(document: &Document, x: f64, y: f64, side: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: pt(x, y),
        width: Length::from_mm(side),
        height: Length::from_mm(side),
    })
}

fn disc(document: &Document, x: f64, y: f64, radius: f64) -> NodeId {
    document.create_ellipse(EllipseFrame {
        center: pt(x, y),
        rx: Length::from_mm(radius),
        ry: Length::from_mm(radius),
    })
}

/// A path through `points` with ids from `first`.
fn poly(document: &Document, first: u64, points: &[(f64, f64)], closed: bool) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .enumerate()
        .map(|(k, &(x, y))| NewAnchor::corner(AnchorId::new(1, first + k as u64), pt(x, y)))
        .collect();
    document.create_path(&anchors, closed)
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

/// A minter of its own peer, for tests that write several plans into one document.
fn minter_of(peer: u64) -> AnchorIdMinter {
    AnchorIdMinter::new(peer)
}

/// The signed shoelace area of a straight-edged outline.
fn area(anchors: &[NewAnchor]) -> f64 {
    let n = anchors.len();
    (0..n)
        .map(|i| {
            let (a, b) = (anchors[i].point, anchors[(i + 1) % n].point);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

/// Two outlines placed as a ring: a plate with a square hole, written through Combine.
fn ring(document: &Document) -> NodeId {
    let plate = rect(document, 0.0, 0.0, 40.0);
    let hole = rect(document, 10.0, 10.0, 20.0);
    let all = objects(document);
    let plan = plan_combine(&all, &select(&[plate, hole]), &mut minter_of(70)).unwrap();
    document
        .replace_with_path(&plan.operands, plan.base, &plan.outlines, "combine_paths")
        .unwrap()
}

#[test]
fn availability_follows_the_selection() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 10.0);
    let b = rect(&document, 20.0, 0.0, 10.0);
    let compound = ring(&document);
    let all = objects(&document);

    let none = path_availability(&all, &ObjectSelection::new());
    assert_eq!(none.combine, BooleanAvailability::NeedsTwo);
    assert!(!none.compound);
    assert!(!none.cut, "nothing selected");

    let two = path_availability(&all, &select(&[a, b]));
    assert_eq!(two.combine, BooleanAvailability::Ready);
    assert!(!two.compound, "no compound path selected");
    assert!(two.cut, "ordinary paths enable Cut");

    let one = path_availability(&all, &select(&[compound]));
    assert_eq!(one.combine, BooleanAvailability::NeedsTwo);
    assert!(one.compound, "one compound path is enough");
    assert!(one.cut);

    let mixed = path_availability(&all, &select(&[a, compound]));
    assert_eq!(mixed.combine, BooleanAvailability::Ready);
    assert!(mixed.compound);

    let open = poly(&document, 900, &[(0.0, 0.0), (5.0, 5.0), (9.0, 0.0)], false);
    let all = objects(&document);
    let with_open = path_availability(&all, &select(&[a, open]));
    assert_eq!(
        with_open.combine,
        BooleanAvailability::Ready,
        "an open path is an operand of Combine (0054 criterion 13)"
    );
}

/// Criteria 3, 4, 6: the winding follows the depth, whatever direction the shapes were drawn in,
/// and the result takes the bottom-most style and place.
#[test]
fn nested_shapes_become_holes_and_the_bottom_style_wins() {
    for reversed in [false, true] {
        let document = Document::new(1);
        let mut inner = vec![(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)];
        if reversed {
            inner.reverse();
        }
        let plate = rect(&document, 0.0, 0.0, 40.0);
        let hole = poly(&document, 10, &inner, true);
        let island = rect(&document, 15.0, 15.0, 10.0);
        document
            .edit_style(&[plate], &StyleEdit::StrokeWidth(Length::from_mm(3.0)))
            .unwrap();
        let all = objects(&document);
        let plan = plan_combine(&all, &select(&[island, hole, plate]), &mut minter()).unwrap();

        assert_eq!(plan.operands, vec![plate, hole, island], "stacking order");
        assert_eq!(plan.base, plate);
        assert_eq!(plan.holes, 1);
        assert!(plan.styles_differ);
        let signs: Vec<bool> = plan.outlines.iter().map(|(a, _)| area(a) > 0.0).collect();
        assert_eq!(signs, vec![true, false, true], "shape, hole, island");
    }
}

/// Criterion 5: nothing is flattened. Two circles keep their eight nodes, kinds and handles.
#[test]
fn circles_keep_their_nodes() {
    let document = Document::new(1);
    let a = disc(&document, 0.0, 0.0, 5.0);
    let b = disc(&document, 30.0, 0.0, 5.0);
    let all = objects(&document);
    let plan = plan_combine(&all, &select(&[a, b]), &mut minter()).unwrap();
    assert_eq!(plan.holes, 0);
    assert!(!plan.styles_differ);
    let nodes: usize = plan.outlines.iter().map(|(anchors, _)| anchors.len()).sum();
    assert_eq!(nodes, 8);
    let ObjectSnapshot::Primitive(first) = &all[0] else {
        panic!("a primitive");
    };
    let expected = curvyo_document_core::outline_of_rotated(&first.shape, first.rotation);
    for (anchor, wanted) in plan.outlines[0].0.iter().zip(&expected) {
        assert!(
            anchor.point == wanted.point
                || anchor.handle_in == wanted.handle_out
                || anchor.handle_out == wanted.handle_in
        );
        assert_eq!(anchor.kind, wanted.kind);
    }
}

/// Criterion 8: the same objects give the same plan however they were selected; anchor ids are
/// fresh and different.
#[test]
fn the_selection_order_does_not_matter_and_ids_are_fresh() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 10.0);
    let b = rect(&document, 30.0, 0.0, 10.0);
    let all = objects(&document);
    let one = plan_combine(&all, &select(&[a, b]), &mut minter()).unwrap();
    let two = plan_combine(&all, &select(&[b, a]), &mut minter()).unwrap();
    assert_eq!(one, two);
    let mut ids: Vec<AnchorId> = one
        .outlines
        .iter()
        .flat_map(|(anchors, _)| anchors.iter().map(|a| a.id))
        .collect();
    let total = ids.len();
    ids.sort_by_key(|id| format!("{id:?}"));
    ids.dedup();
    assert_eq!(ids.len(), total);
}

/// 0035 criteria 9, 10, 10a and the no-area part of 11 are withdrawn by `0054` criterion 8: open,
/// touching, crossing, self-crossing and flat operands combine and are counted; only a coordinate
/// out of range and fewer than two objects stay refused (criterion 6).
#[test]
fn only_range_and_count_refuse_and_the_rest_is_counted() {
    let document = Document::new(1);
    let a = rect(&document, 0.0, 0.0, 10.0);
    let overlapping = rect(&document, 5.0, 5.0, 10.0);
    let sharing = rect(&document, 10.0, 0.0, 10.0);
    let apart = rect(&document, 40.0, 0.0, 10.0);
    let near = rect(&document, 50.01, 0.0, 10.0);
    let open = poly(&document, 900, &[(0.0, 0.0), (5.0, 5.0), (9.0, 0.0)], false);
    let eight = poly(
        &document,
        920,
        &[(100.0, 0.0), (110.0, 10.0), (110.0, 0.0), (100.0, 10.0)],
        true,
    );
    let flat = poly(
        &document,
        940,
        &[(0.0, 100.0), (5.0, 100.0), (9.0, 100.0)],
        true,
    );
    let far = poly(
        &document,
        960,
        &[(1.0e8, 0.0), (1.0e8 + 1.0, 0.0), (1.0e8, 1.0)],
        true,
    );
    let all = objects(&document);
    let plan = |ids: &[NodeId]| plan_combine(&all, &select(ids), &mut minter());

    assert_eq!(plan(&[a]), Err(CombineRefusal::NeedsTwo));
    assert_eq!(
        plan(&[a, far]),
        Err(CombineRefusal::OutOfRange {
            offenders: vec![far],
            of: 2
        })
    );
    let with_open = plan(&[a, open, apart]).unwrap();
    assert_eq!((with_open.open, with_open.touching), (1, 0));
    assert!(with_open.outlines.iter().any(|(_, closed)| !closed));
    let with_flat = plan(&[a, flat]).unwrap();
    assert_eq!((with_flat.outlines.len(), with_flat.touching), (2, 0));
    assert_eq!(plan(&[a, overlapping, apart]).unwrap().touching, 2);
    assert_eq!(
        plan(&[a, sharing]).unwrap().touching,
        2,
        "a shared edge touches"
    );
    assert_eq!(plan(&[apart, near]).unwrap().touching, 0, "0.01 mm apart");
    let eight_plan = plan(&[a, eight, apart]).unwrap();
    assert_eq!((eight_plan.touching, eight_plan.holes), (1, 0));
    assert_eq!(plan(&[a, overlapping, eight]).unwrap().touching, 3);
}

/// 0048 criteria 3 and 5: Split keeps holes with their piece.
#[test]
fn split_keeps_holes_with_their_piece() {
    let document = Document::new(1);
    let outer = rect(&document, 0.0, 0.0, 40.0);
    let hole = rect(&document, 10.0, 10.0, 20.0);
    let island = rect(&document, 15.0, 15.0, 10.0);
    let all = objects(&document);
    let combined = plan_combine(&all, &select(&[outer, hole, island]), &mut minter_of(71)).unwrap();
    let compound = document
        .replace_with_path(
            &combined.operands,
            combined.base,
            &combined.outlines,
            "combine_paths",
        )
        .unwrap();
    let plain = rect(&document, 100.0, 0.0, 10.0);
    let all = objects(&document);

    let broken = plan_split(&all, &select(&[plain, compound]), &mut minter_of(72)).unwrap();
    assert_eq!(broken.removed, vec![compound]);
    assert_eq!(broken.pieces.len(), 2, "the ring and the island");
    assert!(broken.pieces.iter().all(|(id, _)| *id == compound));
    let pieces: Vec<_> = broken.pieces.iter().map(|(_, piece)| piece).collect();
    assert_eq!(pieces[0].len(), 2, "the ring has its hole");
    assert_eq!(pieces[1].len(), 1);
    assert!(broken.with_holes);
    assert_eq!(broken.kept, vec![plain]);
    assert_eq!(broken.left_alone, 0);
    // Nodes and windings are exactly as they were; only the ids are new.
    let ObjectSnapshot::Path(path) = all.iter().find(|o| o.id() == compound).unwrap() else {
        panic!("the compound path");
    };
    let original: Vec<_> = path.subpaths().collect();
    assert_eq!(pieces[0][0].0.len(), original[0].anchors.len());
    for (new, old) in pieces[1][0].0.iter().zip(original[2].anchors) {
        assert_eq!(
            (new.point, new.handle_in, new.handle_out),
            (old.point, old.handle_in, old.handle_out)
        );
        assert_ne!(new.id, old.id);
    }
}

#[test]
fn split_refusals() {
    let document = Document::new(1);
    let ring_id = ring(&document);
    let plain = rect(&document, 100.0, 0.0, 10.0);
    let all = objects(&document);
    assert_eq!(
        plan_split(&all, &select(&[plain]), &mut minter()),
        Err(SplitRefusal::NoCompound)
    );
    assert_eq!(
        plan_split(&all, &ObjectSelection::new(), &mut minter()),
        Err(SplitRefusal::NoCompound)
    );
    assert_eq!(
        plan_split(&all, &select(&[ring_id, plain]), &mut minter()),
        Err(SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: true
        })
    );
    assert_eq!(
        plan_break_apart(&all, &select(&[plain]), &mut minter()),
        Err(BreakApartRefusal::NoCompound)
    );
}

/// A ring left alone beside a compound that does split: counted, kept, selected after.
#[test]
fn one_region_compounds_are_left_as_they_are() {
    let document = Document::new(1);
    let ring_id = ring(&document);
    let a = rect(&document, 100.0, 0.0, 10.0);
    let b = rect(&document, 130.0, 0.0, 10.0);
    let all = objects(&document);
    let pair = plan_combine(&all, &select(&[a, b]), &mut minter_of(73)).unwrap();
    let two = document
        .replace_with_path(&pair.operands, pair.base, &pair.outlines, "combine_paths")
        .unwrap();
    let all = objects(&document);
    let plan = plan_split(&all, &select(&[ring_id, two]), &mut minter()).unwrap();
    assert_eq!(plan.removed, vec![two]);
    assert_eq!(plan.pieces.len(), 2);
    assert!(!plan.with_holes);
    assert_eq!(plan.kept, vec![ring_id]);
    assert_eq!(plan.left_alone, 1);
}

/// Criterion 4: concentric discs, drawn in either stacking order, combine into a ring that the
/// canvas paints (nonzero fill) at radius 15 mm and not at the centre, of the area
/// pi (400 - 100) within 1 mm squared; a third disc inside the hole is painted again.
#[test]
fn concentric_discs_paint_as_a_ring_with_an_island() {
    use curvyo_geometry_core::{Outline, contains_point_in_outlines, outline_area_mm2};

    for inner_first in [false, true] {
        let document = Document::new(1);
        let (outer, inner) = if inner_first {
            let inner = disc(&document, 50.0, 50.0, 10.0);
            (disc(&document, 50.0, 50.0, 20.0), inner)
        } else {
            let outer = disc(&document, 50.0, 50.0, 20.0);
            (outer, disc(&document, 50.0, 50.0, 10.0))
        };
        let island = disc(&document, 50.0, 50.0, 5.0);
        let all = objects(&document);
        let plan = plan_combine(&all, &select(&[outer, inner, island]), &mut minter()).unwrap();
        let triples: Vec<Vec<_>> = plan
            .outlines
            .iter()
            .map(|(anchors, _)| {
                anchors
                    .iter()
                    .map(|a| (a.point, a.handle_in, a.handle_out))
                    .collect()
            })
            .collect();
        let views: Vec<Outline<'_>> = triples.iter().map(|t| Outline::new(t, true)).collect();

        assert!(
            contains_point_in_outlines(&views, pt(65.0, 50.0)),
            "radius 15"
        );
        assert!(
            !contains_point_in_outlines(&views, pt(50.0, 50.0 - 7.5)),
            "radius 7.5, the hole"
        );
        assert!(
            contains_point_in_outlines(&views, pt(50.0, 50.0 - 2.0)),
            "the island"
        );
        let ring: f64 = views[..2].iter().map(outline_area_mm2).sum();
        let expected = std::f64::consts::PI * 300.0;
        assert!(
            (ring - expected).abs() < 1.0,
            "ring area {ring} against {expected}"
        );
    }
}

/// A plate with slot holes (rounded rectangles whose radius is half a side, so two nodes of a side
/// coincide) and a fully rounded square combine: zero-length sides are no self-touch.
#[test]
fn a_plate_with_slots_and_a_rounded_square_combine() {
    let document = Document::new(1);
    let plate = rect(&document, 0.0, 0.0, 100.0);
    let slot = document.create_rect(RectBounds {
        origin: pt(20.0, 20.0),
        width: Length::from_mm(30.0),
        height: Length::from_mm(10.0),
    });
    let pill = rect(&document, 20.0, 60.0, 20.0);
    document
        .set_corner_radius(&[slot], Length::from_mm(5.0))
        .unwrap();
    document
        .set_corner_radius(&[pill], Length::from_mm(10.0))
        .unwrap();
    let all = objects(&document);
    let plan = plan_combine(&all, &select(&[plate, slot, pill]), &mut minter()).unwrap();
    assert_eq!(plan.holes, 2);
    assert_eq!(plan.outlines.len(), 3);
    // A duplicated node is no self-crossing either.
    let doubled = poly(
        &document,
        500,
        &[
            (200.0, 0.0),
            (210.0, 0.0),
            (210.0, 0.0),
            (210.0, 10.0),
            (200.0, 10.0),
        ],
        true,
    );
    let all = objects(&document);
    assert!(plan_combine(&all, &select(&[plate, doubled]), &mut minter()).is_ok());
}

/// Overlapping outlines can make the depth count differ from the enclosure chain; whatever the
/// plan does, no outline may be lost: every outline is in a piece, or the compound is left alone.
#[test]
fn split_never_drops_an_outline() {
    let document = Document::new(1);
    let base = rect(&document, 0.0, 0.0, 1.0);
    let square = |first: u64, x: f64, size: f64| {
        let anchors: Vec<NewAnchor> = [(x, x), (x + size, x), (x + size, x + size), (x, x + size)]
            .iter()
            .enumerate()
            .map(|(k, &(px, py))| NewAnchor::corner(AnchorId::new(3, first + k as u64), pt(px, py)))
            .collect();
        (anchors, true)
    };
    // A holds B; C overlaps A; X lies inside A, B and C.
    let outlines = vec![
        square(10, 0.0, 100.0),
        square(20, 40.0, 30.0),
        square(30, 50.0, 100.0),
        square(40, 55.0, 5.0),
    ];
    let compound = document
        .replace_with_path(&[base], base, &outlines, "combine_paths")
        .unwrap();
    let all = objects(&document);
    match plan_split(&all, &select(&[compound]), &mut minter()) {
        Ok(plan) => {
            let kept: usize = plan.pieces.iter().map(|(_, piece)| piece.len()).sum();
            assert_eq!(kept, outlines.len(), "a split must keep every outline");
        }
        Err(refusal) => assert_eq!(
            refusal,
            SplitRefusal::OnePiece {
                compounds: 1,
                with_holes: true
            }
        ),
    }
}
