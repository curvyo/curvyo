//! Tester cases for `specs/0055-compound-path-node-editing` after the revival on today's main:
//! node commands on compound paths in combination with what merged since 2026-10-19 (text on a
//! compound carrier `0080`, linked clones `0049`, guides `0046`). Written from the specification
//! and the neighbouring specs before the implementation was read.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{
    Affine, AnchorId, CarrierRef, Document, FontKind, FontRef, Length, NewAnchor, NodeId,
    ObjectSnapshot, OnPathValues, PathSnapshot, Point, TextLayoutValues,
};

fn id(n: u64) -> AnchorId {
    AnchorId::new(1, n)
}

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

type Spec = (Vec<NewAnchor>, bool);

fn outline(first: u64, points: &[(f64, f64)], closed: bool) -> Spec {
    (
        points
            .iter()
            .zip(first..)
            .map(|(&(x, y), n)| NewAnchor::corner(id(n), pt(x, y)))
            .collect(),
        closed,
    )
}

fn compound(document: &Document, seed: u64, outlines: &[Spec]) -> NodeId {
    let base = document.create_path(
        &[
            NewAnchor::corner(id(seed), pt(900.0, 900.0)),
            NewAnchor::corner(id(seed + 1), pt(901.0, 900.0)),
        ],
        false,
    );
    document
        .replace_with_path(&[base], base, outlines, "combine_paths")
        .unwrap()
}

fn ring(document: &Document, seed: u64, first: u64) -> NodeId {
    compound(
        document,
        seed,
        &[
            outline(
                first,
                &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0), (100.0, 0.0)],
                true,
            ),
            outline(
                first + 10,
                &[(40.0, 40.0), (60.0, 40.0), (60.0, 60.0), (40.0, 60.0)],
                true,
            ),
        ],
    )
}

/// The path of any object kind: a path itself, a clone as it resolves.
fn shown(document: &Document, node: NodeId) -> Option<PathSnapshot> {
    match document.read_objects().get(node)? {
        ObjectSnapshot::Path(p) => Some(p.clone()),
        ObjectSnapshot::Clone(c) => Some(c.resolved.clone()),
        _ => None,
    }
}

fn is_clone(document: &Document, node: NodeId) -> bool {
    matches!(
        document.read_objects().get(node),
        Some(ObjectSnapshot::Clone(_))
    )
}

fn geometry(p: &PathSnapshot) -> Vec<(bool, Vec<(AnchorId, Point)>)> {
    p.subpaths()
        .map(|o| {
            (
                o.closed,
                o.anchors.iter().map(|a| (a.id, a.point)).collect(),
            )
        })
        .collect()
}

fn undo_last(d: &Document) {
    let s = d.own_steps().last().unwrap().step;
    d.undo_step(s).unwrap();
}

// ------------------------------------------------------------------------- linked clones (0049)

/// `0049` criterion 10: every node command of this story on a compound original shows on its
/// clone in the next read, outline by outline, with the anchor ids kept.
#[test]
fn a_clone_of_a_compound_path_follows_every_node_command() {
    let d = Document::new(1);
    let source = ring(&d, 9000, 100);
    let open = compound(
        &d,
        9100,
        &[
            outline(300, &[(200.0, 0.0), (210.0, 0.0), (220.0, 0.0)], false),
            outline(310, &[(200.0, 20.0), (220.0, 20.0)], false),
        ],
    );
    let clones = d.linked_clone(&[source, open], &[]).unwrap();
    let (cs, co) = (clones[0], clones[1]);
    let same = |d: &Document| {
        assert_eq!(
            geometry(&shown(d, cs).unwrap()),
            geometry(&shown(d, source).unwrap()),
            "ring clone"
        );
        assert_eq!(
            geometry(&shown(d, co).unwrap()),
            geometry(&shown(d, open).unwrap()),
            "open clone"
        );
        assert!(is_clone(d, cs) && is_clone(d, co), "still clones");
    };
    same(&d);
    d.move_anchors(&[(source, id(110), pt(45.0, 47.0))])
        .unwrap();
    same(&d);
    assert_eq!(
        shown(&d, cs).unwrap().extra_subpaths[0].anchors[0].point,
        pt(45.0, 47.0)
    );
    d.split_at_anchor(open, id(301), id(399)).unwrap();
    same(&d);
    assert_eq!(shown(&d, co).unwrap().extra_subpaths.len(), 2);
    d.join_endpoints(open, id(311), open, id(399)).unwrap();
    same(&d);
    d.delete_anchors(&[(source, id(110)), (source, id(111))]);
    same(&d);
    assert_eq!(
        shown(&d, cs).unwrap().extra_subpaths[0].anchors.len(),
        2,
        "the hole keeps two nodes (criterion 4: removed only below 2)"
    );
    d.delete_anchors(&[(source, id(112))]);
    same(&d);
    assert_eq!(
        shown(&d, cs).unwrap().extra_subpaths.len(),
        0,
        "below 2 nodes the outline went"
    );
    d.split_at_anchor(source, id(101), id(398)).ok();
    same(&d);
}

/// `0049` criterion 31 with Join of two objects (`0055` criterion 5 c): the removed object's
/// clone is repaired in the same commit and keeps its outline; undo brings both back as they were.
#[test]
fn a_clone_of_the_object_a_join_removes_keeps_its_outline_and_undo_restores_the_link() {
    let d = Document::new(1);
    let plate = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(20.0, 50.0), (80.0, 50.0)], false),
        ],
    );
    let line = d.create_path(
        &[
            NewAnchor::corner(id(30), pt(80.0, 90.0)),
            NewAnchor::corner(id(31), pt(80.0, 60.0)),
        ],
        false,
    );
    let clone = d.linked_clone(&[line], &[]).unwrap()[0];
    let line_outline = geometry(&shown(&d, line).unwrap());
    d.join_endpoints(plate, id(21), line, id(31)).unwrap();
    assert!(d.path(line).is_none(), "the other object is gone");
    let kept = shown(&d, clone).expect("the clone survives the removal of its original");
    assert_eq!(
        geometry(&kept)[0]
            .1
            .iter()
            .map(|(_, p)| *p)
            .collect::<Vec<_>>(),
        line_outline[0]
            .1
            .iter()
            .map(|(_, p)| *p)
            .collect::<Vec<_>>(),
        "the clone draws the outline it drew before"
    );
    undo_last(&d);
    assert!(is_clone(&d, clone), "undo turns it back into a clone");
    assert_eq!(geometry(&shown(&d, clone).unwrap()), line_outline);
    assert_eq!(d.path(plate).unwrap().extra_subpaths.len(), 1);
}

/// A clone of the surviving object follows the outlines that moved in (`0049` criterion 10).
#[test]
fn a_clone_of_the_surviving_object_shows_the_outlines_that_moved_in() {
    let d = Document::new(1);
    let line = d.create_path(
        &[
            NewAnchor::corner(id(30), pt(80.0, 90.0)),
            NewAnchor::corner(id(31), pt(80.0, 60.0)),
        ],
        false,
    );
    let plate = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (0.0, 100.0), (100.0, 100.0)], true),
            outline(20, &[(80.0, 50.0), (20.0, 50.0)], false),
        ],
    );
    let clone = d.linked_clone(&[line], &[]).unwrap()[0];
    d.join_endpoints(line, id(31), plate, id(20)).unwrap();
    let shown_now = shown(&d, clone).unwrap();
    assert_eq!(
        shown_now.subpaths().count(),
        2,
        "joined outline and the triangle"
    );
    assert_eq!(geometry(&shown_now), geometry(&d.path(line).unwrap()));
}

// ----------------------------------------------------------------------------- guides (0046)

fn is_guide(d: &Document, node: NodeId) -> bool {
    d.guide_marking(&[node]).all_guides()
}

/// A compound guide stays a guide through every node command that keeps the object, also when
/// it drops to an ordinary path (same id), and undo restores the flag.
#[test]
fn a_compound_guide_stays_a_guide_through_node_commands() {
    let d = Document::new(1);
    let g = ring(&d, 9000, 100);
    d.set_guide(&[g], true).unwrap();
    assert!(is_guide(&d, g));
    d.move_anchors(&[(g, id(110), pt(41.0, 41.0))]).unwrap();
    assert!(is_guide(&d, g), "move");
    d.split_at_anchor(g, id(111), id(199)).unwrap();
    assert!(is_guide(&d, g), "split of a closed outline");
    d.delete_anchors(&[(g, id(110)), (g, id(112))]);
    assert!(is_guide(&d, g), "delete");
    d.delete_anchors(&[(g, id(100)), (g, id(101)), (g, id(102)), (g, id(103))]);
    let left = d.path(g).expect("one outline is left under the same id");
    assert!(!left.is_compound());
    assert!(
        is_guide(&d, g),
        "promotion to an ordinary path keeps the guide"
    );
    undo_last(&d);
    assert!(is_guide(&d, g) && d.path(g).unwrap().is_compound());
}

/// `0046` criterion 34 as the plan keeps it for Join: the survivor is a guide only when both
/// objects were guides.
#[test]
fn join_across_objects_makes_a_guide_only_when_both_were_guides() {
    let make = |first_guide: bool, second_guide: bool| {
        let d = Document::new(1);
        let a = compound(
            &d,
            9000,
            &[
                outline(10, &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)], false),
                outline(20, &[(0.0, 50.0), (10.0, 50.0)], false),
            ],
        );
        let b = d.create_path(
            &[
                NewAnchor::corner(id(30), pt(20.0, 10.0)),
                NewAnchor::corner(id(31), pt(30.0, 10.0)),
            ],
            false,
        );
        if first_guide {
            d.set_guide(&[a], true).unwrap();
        }
        if second_guide {
            d.set_guide(&[b], true).unwrap();
        }
        (d, a, b)
    };
    for (ga, gb, expected) in [
        (true, true, true),
        (true, false, false),
        (false, true, false),
        (false, false, false),
    ] {
        for a_first in [true, false] {
            let (d, a, b) = make(ga, gb);
            let survivor = if a_first {
                d.join_endpoints(a, id(12), b, id(30)).unwrap();
                a
            } else {
                d.join_endpoints(b, id(30), a, id(12)).unwrap();
                b
            };
            assert!(d.path(survivor).is_some());
            assert_eq!(
                is_guide(&d, survivor),
                expected,
                "a guide {ga}, b guide {gb}, a first {a_first}"
            );
            undo_last(&d);
            assert_eq!(is_guide(&d, a), ga, "undo restores a");
            assert_eq!(is_guide(&d, b), gb, "undo restores b");
        }
    }
}

// ------------------------------------------------------------------- text on a carrier (0080)

fn text_on(d: &Document, carrier: NodeId, outline: u32) -> NodeId {
    let font = FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    };
    let text = d
        .create_text(
            "On",
            &font,
            Length::from_mm(10.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    d.put_text_on_path(
        text,
        CarrierRef {
            node: carrier,
            outline,
        },
        &OnPathValues::at(Length::from_mm(0.0)),
        false,
    )
    .unwrap();
    text
}

/// (carrier node, carried outline, kept copy, first point of the curve the text sits on)
fn carried(d: &Document, text: NodeId) -> (NodeId, u32, bool, Point) {
    match d.read_objects().get(text) {
        Some(ObjectSnapshot::Text(t)) => {
            let on = t.on_path.as_ref().unwrap();
            (
                on.carrier.node,
                on.carrier.outline,
                on.kept,
                on.curve.as_ref().expect("a curve to sit on").anchors[0].point,
            )
        }
        other => panic!("not a text: {other:?}"),
    }
}

/// A text on the hole of a ring stays on the hole when the outer outline is deleted: the object
/// drops to an ordinary path and the hole becomes outline 0.
#[test]
fn a_text_on_the_hole_follows_it_when_the_outer_outline_is_deleted() {
    let d = Document::new(1);
    let g = ring(&d, 9000, 100);
    let text = text_on(&d, g, 1);
    let before = carried(&d, text);
    d.delete_anchors(&[(g, id(100)), (g, id(101)), (g, id(102)), (g, id(103))]);
    let after = carried(&d, text);
    assert_eq!(after.0, g);
    assert_eq!(after.1, 0, "the hole is the first outline now");
    assert!(!after.2, "still on its live carrier");
    assert_eq!(after.3, before.3, "it sits on the same curve");
    undo_last(&d);
    assert_eq!(carried(&d, text), before);
}

/// A text on an open outline that is split stays on the first part (the outline "that took its
/// place"); a text on a later outline moves its index by one; undo restores both.
#[test]
fn a_text_on_a_split_outline_stays_on_the_first_part_and_later_ones_renumber() {
    let d = Document::new(1);
    let g = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (50.0, 0.0), (90.0, 0.0)], false),
            outline(20, &[(0.0, 10.0), (50.0, 10.0), (90.0, 10.0)], false),
        ],
    );
    let on_first = text_on(&d, g, 0);
    let on_second = text_on(&d, g, 1);
    let (a0, b0) = (carried(&d, on_first), carried(&d, on_second));
    d.split_at_anchor(g, id(11), id(50)).unwrap();
    let (a1, b1) = (carried(&d, on_first), carried(&d, on_second));
    assert_eq!(a1.1, 0, "first part keeps the text");
    assert_eq!(a1.3, a0.3);
    assert_eq!(b1.1, 2, "the later outline moved from 1 to 2");
    assert_eq!(b1.3, b0.3);
    undo_last(&d);
    assert_eq!((carried(&d, on_first), carried(&d, on_second)), (a0, b0));
}

/// Opening a closed outline keeps its index; Pen extension and Close keep every carried index.
#[test]
fn opening_closing_and_extending_outlines_leave_carried_indexes_alone() {
    let d = Document::new(1);
    let g = ring(&d, 9000, 100);
    let t = text_on(&d, g, 1);
    let before = carried(&d, t);
    d.split_at_anchor(g, id(110), id(77)).unwrap();
    let opened = carried(&d, t);
    assert_eq!(opened.1, 1, "the opened hole is the same outline");
    assert!(!opened.2);
    undo_last(&d);
    assert_eq!(carried(&d, t), before);
}

/// Join across objects removes the other object: a text on its outline keeps that outline as a
/// kept copy (`0080` criterion 25) and does not jump; a text on the survivor stays where it was.
#[test]
fn join_across_objects_keeps_the_texts_on_both_carriers_where_they_were() {
    let d = Document::new(1);
    let survivor = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0)], false),
            outline(20, &[(0.0, 50.0), (10.0, 50.0)], false),
        ],
    );
    let other = d.create_path(
        &[
            NewAnchor::corner(id(30), pt(20.0, 10.0)),
            NewAnchor::corner(id(31), pt(30.0, 10.0)),
        ],
        false,
    );
    let t_survivor = text_on(&d, survivor, 1);
    let t_other = text_on(&d, other, 0);
    let (s0, o0) = (carried(&d, t_survivor), carried(&d, t_other));
    d.join_endpoints(survivor, id(12), other, id(30)).unwrap();
    let s1 = carried(&d, t_survivor);
    assert_eq!(
        (s1.0, s1.1, s1.3),
        (s0.0, s0.1, s0.3),
        "survivor text unmoved"
    );
    let o1 = carried(&d, t_other);
    assert_eq!(o1.3, o0.3, "the other text still sits on the same curve");
    undo_last(&d);
    assert_eq!(carried(&d, t_survivor), s0);
    assert_eq!(carried(&d, t_other), o0);
}

/// Join within one object when the text sits on the second of the two joined outlines goes to the
/// merged outline (`0055` plan note: "the outline that took its place").
#[test]
fn a_text_on_a_merged_outline_goes_to_the_merged_outline() {
    let d = Document::new(1);
    let g = compound(
        &d,
        9000,
        &[
            outline(10, &[(0.0, 0.0), (10.0, 0.0)], false),
            outline(20, &[(30.0, 0.0), (40.0, 0.0)], false),
            outline(30, &[(0.0, 50.0), (10.0, 50.0)], false),
        ],
    );
    let on_second = text_on(&d, g, 1);
    let on_third = text_on(&d, g, 2);
    let third0 = carried(&d, on_third);
    d.join_endpoints(g, id(11), g, id(20)).unwrap();
    assert_eq!(carried(&d, on_second).1, 0, "merged outline");
    assert_eq!(carried(&d, on_third).1, 1, "renumbered");
    assert_eq!(carried(&d, on_third).3, third0.3);
}
