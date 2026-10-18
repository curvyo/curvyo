//! Independent session-level acceptance tests for `0048-split-compound-path` (Split and Break
//! apart), written from the specification before the implementation was read. The six inputs
//! E1 to E6 of the spec sheet run all three commands; the performance inputs of criterion 15
//! are `#[ignore]`d release tests.

#![allow(
    unused_must_use,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::type_complexity
)]

mod split_cut_common;

use std::collections::HashSet;
use std::time::Instant;

use curvyo_document_core::{
    AnchorId, AnchorKind, Color, Document, Length, NewAnchor, NodeId, PathSnapshot, StyleEdit,
};
use curvyo_editor_wasm::{
    BreakApartOutcome, CombineOutcome, CutOutcome, KeyInput, Session, SplitOutcome, Tool,
};
use curvyo_ui_core::{BooleanOp, BreakApartRefusal, CutRefusal, SplitRefusal};
use split_cut_common::*;

fn styled(d: &Document, id: NodeId, stroke: (u8, u8, u8), width: f64, fill: (u8, u8, u8)) {
    d.edit_style(
        &[id],
        &StyleEdit::StrokeColor(Color {
            r: stroke.0,
            g: stroke.1,
            b: stroke.2,
        }),
    );
    d.edit_style(&[id], &StyleEdit::StrokeWidth(Length::from_mm(width)));
    d.edit_style(&[id], &StyleEdit::FillEnabled(true));
    d.edit_style(
        &[id],
        &StyleEdit::FillColor(Color {
            r: fill.0,
            g: fill.1,
            b: fill.2,
        }),
    );
}

/// A ring: outline radius 20, hole radius 10 (reversed), as Difference of concentric circles
/// makes it.
fn ring(d: &Document, cx: f64, cy: f64) -> NodeId {
    compound(
        d,
        vec![circle(cx, cy, 20.0, false), circle(cx, cy, 10.0, true)],
    )
}

fn split_applied(o: &SplitOutcome) -> (usize, usize, bool, usize) {
    match o {
        SplitOutcome::Applied {
            compounds,
            pieces,
            with_holes,
            left_alone,
        } => (*compounds, *pieces, *with_holes, *left_alone),
        other => panic!("split not applied: {other:?}"),
    }
}

fn break_applied(o: &BreakApartOutcome) -> (usize, usize, bool) {
    match o {
        BreakApartOutcome::Applied {
            compounds,
            pieces,
            with_holes,
            ..
        } => (*compounds, *pieces, *with_holes),
        other => panic!("break apart not applied: {other:?}"),
    }
}

fn split_refusal(o: SplitOutcome) -> SplitRefusal {
    match o {
        SplitOutcome::Refused(r) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn break_refusal(o: BreakApartOutcome) -> BreakApartRefusal {
    match o {
        BreakApartOutcome::Refused(r) => r,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn is_closed_plain(p: &PathSnapshot) -> bool {
    p.closed && !p.is_compound()
}

fn point_set(a: &[NewAnchor]) -> Vec<(i64, i64)> {
    let mut s: Vec<(i64, i64)> = a
        .iter()
        .map(|n| {
            (
                (n.point.x * 1e6).round() as i64,
                (n.point.y * 1e6).round() as i64,
            )
        })
        .collect();
    s.sort_unstable();
    s
}

fn outline_point_sets(p: &PathSnapshot) -> Vec<Vec<(i64, i64)>> {
    p.subpaths().map(|sp| point_set(sp.anchors)).collect()
}

fn anchor_ids(p: &PathSnapshot) -> Vec<AnchorId> {
    p.subpaths()
        .flat_map(|sp| sp.anchors.iter().map(|a| a.id))
        .collect()
}

fn assert_nothing_changed(s: &Session, before: &[u8], n_labels: usize, selected: usize) {
    assert_eq!(json(s), before, "the document is unchanged");
    assert_eq!(label_count(s), n_labels, "no commit was made");
    assert_eq!(s.selected_object_count(), selected, "the same selection");
}

// ---------------------------------------------------------------------------------------------
// Criteria 1 and 2: entry and availability
// ---------------------------------------------------------------------------------------------

#[test]
fn c1_availability_is_one_flag_for_split_and_break_apart_and_follows_selection_and_tool() {
    let d = Document::new(1);
    let ringid = ring(&d, 100.0, 100.0);
    d.create_path(&square(300.0, 0.0, 20.0), true);
    let mut s = session_of(&d);
    let a = s.path_availability();
    assert!(!a.compound && !a.cut, "nothing selected: all dimmed");

    // The ordinary square alone.
    marquee(&mut s, pt(290.0, -10.0), pt(330.0, 30.0));
    assert_eq!(s.selected_object_count(), 1);
    let a = s.path_availability();
    assert!(
        !a.compound,
        "an ordinary path does not enable Split or Break apart"
    );
    assert!(a.cut, "an ordinary path enables Cut");

    // The ring alone, then both.
    marquee(&mut s, pt(70.0, 70.0), pt(130.0, 130.0));
    assert_eq!(selected_ids(&mut s), vec![ringid]);
    let a = s.path_availability();
    assert!(a.compound && a.cut);
    select_all(&mut s);
    assert!(s.path_availability().compound, "compound among others");

    // Any other tool: dimmed, even with the selection kept.
    for tool in [Tool::Pen, Tool::Node] {
        s.set_tool(tool);
        let a = s.path_availability();
        assert!(!a.compound && !a.cut, "{tool:?}: everything dimmed");
    }
    s.set_tool(Tool::Select);
    assert!(
        s.path_availability().compound,
        "back on Select the selection answers again"
    );
}

#[test]
fn c1_a_dimmed_press_changes_nothing_and_never_changes_the_tool() {
    let d = Document::new(1);
    ring(&d, 100.0, 100.0);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let n = label_count(&s);
    for tool in [Tool::Pen, Tool::Node] {
        s.set_tool(tool);
        let a = s.apply_split();
        assert!(
            matches!(a, SplitOutcome::Ignored | SplitOutcome::Refused(_)),
            "{tool:?}: {a:?}"
        );
        let b = s.apply_break_apart();
        assert!(
            matches!(
                b,
                BreakApartOutcome::Ignored | BreakApartOutcome::Refused(_)
            ),
            "{tool:?}: {b:?}"
        );
        let c = s.apply_cut();
        assert!(
            matches!(c, CutOutcome::Ignored | CutOutcome::Refused(_)),
            "{tool:?}: {c:?}"
        );
        assert_eq!(json(&s), before);
        assert_eq!(label_count(&s), n);
    }
}

#[test]
fn c2_commit_labels_have_plain_history_names() {
    assert_eq!(curvyo_ui_core::known_operation_name("split"), Some("Split"));
    assert_eq!(
        curvyo_ui_core::known_operation_name("break_apart"),
        Some("Break apart")
    );
    assert_eq!(
        curvyo_ui_core::known_operation_name("cut_at_crossings"),
        Some("Cut")
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 3: Split
// ---------------------------------------------------------------------------------------------

#[test]
fn c3_e2_two_separate_circles_become_two_closed_paths_in_place() {
    let d = Document::new(1);
    let below = d.create_path(&square(-200.0, -200.0, 10.0), true);
    let a = circle(0.0, 0.0, 10.0, false);
    let b = circle(50.0, 0.0, 10.0, false);
    let old_ids: HashSet<AnchorId> = a.iter().chain(b.iter()).map(|n| n.id).collect();
    let comp = compound(&d, vec![a.clone(), b.clone()]);
    let above = d.create_path(&square(300.0, 300.0, 10.0), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = labels(&reread(&s));
    let out = s.apply_split();
    assert_eq!(split_applied(&out), (1, 2, false, 0));
    assert_eq!(new_labels(&before, &s), vec!["split".to_string()]);

    let doc = reread(&s);
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 4);
    assert_eq!(ids[0], below, "the object below keeps its id and place");
    assert_eq!(ids[3], above, "the object above keeps its id and place");
    assert!(!ids.contains(&comp), "the compound path is gone");
    let p0 = doc.path(ids[1]).unwrap();
    let p1 = doc.path(ids[2]).unwrap();
    assert!(is_closed_plain(&p0) && is_closed_plain(&p1));
    // In the order of the first outline of each part, nodes exact.
    let want = |o: &[NewAnchor]| o.iter().map(node_sig).collect::<Vec<_>>();
    assert_eq!(outline_sigs(&p0)[0].1.len(), 4);
    for (got, exp) in [(&p0, &a), (&p1, &b)] {
        let sig = outline_sigs(got);
        assert_eq!(sig.len(), 1);
        assert_eq!(
            sig[0].1,
            want(exp),
            "every node, handle and kind exactly as before"
        );
    }
    // New ids for the objects and the anchors.
    for p in [&p0, &p1] {
        assert!(!old_ids.contains(&anchor_ids(p)[0]));
        for id in anchor_ids(p) {
            assert!(!old_ids.contains(&id), "anchor ids are new");
        }
    }
    let new_anchor_ids: HashSet<AnchorId> =
        anchor_ids(&p0).into_iter().chain(anchor_ids(&p1)).collect();
    assert_eq!(new_anchor_ids.len(), 8, "no anchor id is used twice");

    // Selected: all pieces and the untouched objects.
    let mut sel = selected_ids(&mut s);
    let mut want_sel = ids.clone();
    sel.sort_by_key(|i| format!("{i:?}"));
    want_sel.sort_by_key(|i| format!("{i:?}"));
    assert_eq!(sel, want_sel);
    assert_eq!(s.selected_object_count(), 4);
    assert!(s.path_availability().cut);
}

#[test]
fn c3_e5_a_ring_with_a_disc_in_its_hole_gives_the_ring_and_the_disc() {
    for island_first in [false, true] {
        let d = Document::new(1);
        let mut outlines = vec![
            circle(100.0, 100.0, 20.0, false),
            circle(100.0, 100.0, 10.0, true),
            circle(100.0, 100.0, 5.0, false),
        ];
        if island_first {
            outlines.rotate_left(2); // island, ring outline, hole
        }
        compound(&d, outlines);
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_split();
        assert_eq!(
            split_applied(&out),
            (1, 2, true, 0),
            "island_first {island_first}"
        );
        let ps = paths(&s);
        assert_eq!(ps.len(), 2);
        let (ringp, discp) = if island_first {
            // Pieces are ordered by the first outline of each part.
            assert!(is_closed_plain(&ps[0]) && ps[1].is_compound());
            (&ps[1], &ps[0])
        } else {
            assert!(ps[0].is_compound() && is_closed_plain(&ps[1]));
            (&ps[0], &ps[1])
        };
        assert_eq!(ringp.subpaths().count(), 2, "the ring keeps its hole");
        assert!(painted(ringp, 115.0, 100.0));
        assert!(!painted(ringp, 100.0, 100.0), "the hole is still a hole");
        assert!(!painted(ringp, 107.0, 100.0));
        assert!(painted(discp, 100.0, 100.0), "the disc of radius 5");
        assert!(!painted(discp, 107.0, 100.0));
    }
}

#[test]
fn c3_depth_is_geometric_and_does_not_depend_on_the_winding_directions() {
    // All outlines run the same way (a file of another program may do that).
    let d = Document::new(1);
    compound(
        &d,
        vec![
            circle(100.0, 100.0, 20.0, false),
            circle(100.0, 100.0, 10.0, false),
            circle(100.0, 100.0, 5.0, false),
            circle(300.0, 100.0, 20.0, true),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let (c, n, holes, left) = split_applied(&s.apply_split());
    assert_eq!(
        (c, n, holes, left),
        (1, 3, true, 0),
        "ring, disc, separate circle"
    );
    let ps = paths(&s);
    assert_eq!(ps[0].subpaths().count(), 2);
    assert_eq!(ps[1].subpaths().count(), 1);
    assert_eq!(ps[2].subpaths().count(), 1);
}

#[test]
fn c3_a_plate_with_many_holes_is_one_part_and_several_plates_split_apart() {
    let d = Document::new(1);
    let mut outlines = vec![square(0.0, 0.0, 200.0)];
    for i in 0..3 {
        for j in 0..3 {
            outlines.push(circle(
                40.0 + 60.0 * f64::from(i),
                40.0 + 60.0 * f64::from(j),
                10.0,
                true,
            ));
        }
    }
    // A second plate far away, with one hole.
    outlines.push(square(500.0, 0.0, 100.0));
    outlines.push(circle(550.0, 50.0, 10.0, true));
    compound(&d, outlines);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(split_applied(&s.apply_split()), (1, 2, true, 0));
    let ps = paths(&s);
    assert_eq!(ps[0].subpaths().count(), 10);
    assert_eq!(ps[1].subpaths().count(), 2);
}

// ---------------------------------------------------------------------------------------------
// Criterion 4: the touch rule
// ---------------------------------------------------------------------------------------------

#[test]
fn c4_overlapping_squares_in_one_compound_path_are_one_part_and_refused() {
    let d = Document::new(1);
    let c = compound(&d, vec![square(0.0, 0.0, 20.0), square(10.0, 10.0, 20.0)]);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let n = label_count(&s);
    assert_eq!(
        split_refusal(s.apply_split()),
        SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        }
    );
    assert_nothing_changed(&s, &before, n, 1);
    assert_eq!(object_ids(&s), vec![c], "same object, same id");
    // A second press gives the same result.
    assert_eq!(
        split_refusal(s.apply_split()),
        SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        }
    );
    assert_nothing_changed(&s, &before, n, 1);
}

#[test]
fn c4_the_golden_file_of_overlapping_squares_loads_and_is_one_part() {
    let bytes = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/compound_overlapping_squares.curvyo"
    ))
    .unwrap();
    let mut s = Session::open(2, &bytes).unwrap();
    s.resize_viewport(1600.0, 1000.0);
    s.set_tool(Tool::Select);
    select_all(&mut s);
    assert!(s.path_availability().compound);
    assert_eq!(
        split_refusal(s.apply_split()),
        SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: false
        }
    );
}

#[test]
fn c4_the_touch_threshold_is_about_a_micrometre() {
    // 0.01 mm apart: two parts (the spec's own example). 0.0005 mm: touching. 0.0035 mm: apart.
    for (gap, parts) in [
        (0.01, 2),
        (0.0005, 1),
        (0.0035, 2),
        (0.0, 1),
        (-1.0, 1),
        (5.0, 2),
    ] {
        let d = Document::new(1);
        compound(
            &d,
            vec![square(0.0, 0.0, 20.0), square(20.0 + gap, 0.0, 20.0)],
        );
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_split();
        match parts {
            1 => assert_eq!(
                split_refusal(out),
                SplitRefusal::OnePiece {
                    compounds: 1,
                    with_holes: false
                },
                "gap {gap}"
            ),
            _ => assert_eq!(split_applied(&out), (1, 2, false, 0), "gap {gap}"),
        }
    }
}

#[test]
fn c4_corner_touch_and_curved_tangents_join_parts_too() {
    // Squares meeting in one corner.
    let d = Document::new(1);
    compound(&d, vec![square(0.0, 0.0, 20.0), square(20.0, 20.0, 20.0)]);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert!(matches!(
        s.apply_split(),
        SplitOutcome::Refused(SplitRefusal::OnePiece { .. })
    ));
    // Circles tangent, crossing, and almost tangent.
    for (dist, joined) in [(20.0, true), (19.0, true), (20.0005, true), (20.05, false)] {
        let d = Document::new(1);
        compound(
            &d,
            vec![
                circle(0.0, 0.0, 10.0, false),
                circle(dist, 0.0, 10.0, false),
            ],
        );
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_split();
        assert_eq!(
            matches!(out, SplitOutcome::Refused(SplitRefusal::OnePiece { .. })),
            joined,
            "centre distance {dist}: {out:?}"
        );
    }
}

#[test]
fn c4_a_disc_that_touches_or_crosses_the_hole_of_its_ring_is_not_a_part_of_its_own() {
    // Touching the hole's edge from inside the hole: connected to the ring.
    for (cx, joined) in [(105.0, true), (103.0, false), (107.0, true), (101.0, false)] {
        let d = Document::new(1);
        compound(
            &d,
            vec![
                circle(100.0, 100.0, 20.0, false),
                circle(100.0, 100.0, 10.0, true),
                circle(cx, 100.0, 5.0, false),
            ],
        );
        let mut s = session_of(&d);
        select_all(&mut s);
        let out = s.apply_split();
        // Disc centre at +5 / radius 5 touches the hole radius 10 at (110, 100); at +7 it crosses.
        assert_eq!(
            matches!(out, SplitOutcome::Refused(SplitRefusal::OnePiece { .. })),
            joined,
            "disc centre {cx}: {out:?}"
        );
    }
}

#[test]
fn c4_touching_regions_chain_into_one_part_which_stays_one_object() {
    let d = Document::new(1);
    compound(
        &d,
        vec![
            square(0.0, 0.0, 20.0),
            square(20.0, 0.0, 20.0),
            square(40.0, 0.0, 20.0),
            square(500.0, 0.0, 20.0),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let out = s.apply_split();
    let ps = paths(&s);
    assert_eq!(ps.len(), 2);
    assert_eq!(
        ps[0].subpaths().count(),
        3,
        "the chain of three squares stays one object"
    );
    assert_eq!(ps[1].subpaths().count(), 1);
    assert_eq!(
        split_applied(&out),
        (1, 2, false, 0),
        "no piece has a hole, so no hole sentence"
    );
}

// ---------------------------------------------------------------------------------------------
// Criteria 5, 6, 7, 8: node data, style, stacking, one piece, one commit
// ---------------------------------------------------------------------------------------------

fn mixed_nodes(x: f64) -> Vec<NewAnchor> {
    vec![
        NewAnchor {
            id: aid(),
            point: pt(x, 0.0),
            handle_in: v(0.0, 0.0),
            handle_out: v(5.0, 7.0),
            kind: AnchorKind::Corner,
        },
        NewAnchor {
            id: aid(),
            point: pt(x + 30.0, 4.0),
            handle_in: v(-4.0, -3.0),
            handle_out: v(4.0, 3.0),
            kind: AnchorKind::Symmetric,
        },
        NewAnchor {
            id: aid(),
            point: pt(x + 40.0, 40.0),
            handle_in: v(3.0, -9.0),
            handle_out: v(-6.0, 18.0),
            kind: AnchorKind::Asymmetric,
        },
        NewAnchor {
            id: aid(),
            point: pt(x, 33.5),
            handle_in: v(1.25, 0.0),
            handle_out: v(0.0, 0.0),
            kind: AnchorKind::Corner,
        },
    ]
}

#[test]
fn c5_nodes_handles_kinds_and_style_survive_split_and_break_apart_exactly() {
    for command in ["split", "break_apart"] {
        let d = Document::new(1);
        let a = mixed_nodes(0.0);
        let b = mixed_nodes(200.0);
        let comp = compound(&d, vec![a.clone(), b.clone()]);
        styled(&d, comp, (200, 10, 10), 1.75, (10, 200, 30));
        d.edit_style(
            &[comp],
            &StyleEdit::FillOpacity(curvyo_document_core::Opacity::new(0.4).unwrap()),
        );
        let style = d.path(comp).unwrap().style;
        let mut s = session_of(&d);
        select_all(&mut s);
        if command == "split" {
            split_applied(&s.apply_split());
        } else {
            break_applied(&s.apply_break_apart());
        }
        let ps = paths(&s);
        assert_eq!(ps.len(), 2, "{command}");
        for (p, orig) in ps.iter().zip([&a, &b]) {
            assert!(is_closed_plain(p), "{command}");
            assert_eq!(
                outline_sigs(p)[0].1,
                orig.iter().map(node_sig).collect::<Vec<_>>(),
                "{command}: exact bits, no flattening, no reversal"
            );
            assert_eq!(p.style, style, "{command}: the complete style is a copy");
            assert!(p.style.fill.enabled);
        }
        let ids: HashSet<NodeId> = object_ids(&s).into_iter().collect();
        assert!(!ids.contains(&comp));
    }
}

#[test]
fn c6_pieces_sit_consecutively_at_the_place_of_their_compound_path() {
    let d = Document::new(1);
    let bottom = d.create_path(&square(-300.0, 0.0, 10.0), true);
    let c1 = compound(
        &d,
        vec![
            square(0.0, 0.0, 10.0),
            square(50.0, 0.0, 10.0),
            square(100.0, 0.0, 10.0),
        ],
    );
    let middle = d.create_path(&square(-300.0, 100.0, 10.0), true);
    let c2 = compound(
        &d,
        vec![square(0.0, 200.0, 10.0), square(50.0, 200.0, 10.0)],
    );
    let top = d.create_path(&square(-300.0, 300.0, 10.0), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(split_applied(&s.apply_split()), (2, 5, false, 0));
    let doc = reread(&s);
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 8);
    assert_eq!((ids[0], ids[4], ids[7]), (bottom, middle, top));
    assert!(!ids.contains(&c1) && !ids.contains(&c2));
    let x_of = |id: NodeId| doc.path(id).unwrap().anchors[0].point.x;
    assert_eq!(
        [
            x_of(ids[1]),
            x_of(ids[2]),
            x_of(ids[3]),
            x_of(ids[5]),
            x_of(ids[6])
        ],
        [0.0, 50.0, 100.0, 0.0, 50.0]
    );
    assert_eq!(
        s.selected_object_count(),
        8,
        "pieces and untouched objects are selected"
    );
    assert_eq!(selected_ids(&mut s).len(), 8);
}

#[test]
fn c6_only_the_selected_compound_path_is_split_and_only_its_pieces_end_up_selected() {
    let d = Document::new(1);
    let c1 = compound(&d, vec![square(0.0, 0.0, 10.0), square(50.0, 0.0, 10.0)]);
    let c2 = compound(
        &d,
        vec![square(0.0, 200.0, 10.0), square(50.0, 200.0, 10.0)],
    );
    let mut s = session_of(&d);
    marquee(&mut s, pt(-5.0, -5.0), pt(70.0, 20.0));
    assert_eq!(selected_ids(&mut s), vec![c1]);
    assert_eq!(split_applied(&s.apply_split()), (1, 2, false, 0));
    let ids = object_ids(&s);
    assert_eq!(ids.len(), 3);
    assert_eq!(ids[2], c2, "the unselected compound path is untouched");
    assert_eq!(s.selected_object_count(), 2);
    assert!(s.path_availability().cut);
}

#[test]
fn c7_one_piece_compound_paths_are_left_alone_when_others_are_split() {
    let d = Document::new(1);
    let r = ring(&d, 0.0, 0.0);
    let two = compound(
        &d,
        vec![
            circle(100.0, 0.0, 10.0, false),
            circle(150.0, 0.0, 10.0, false),
        ],
    );
    let before_ring = reread_path(&d, r);
    let mut s = session_of(&d);
    select_all(&mut s);
    let out = s.apply_split();
    assert_eq!(split_applied(&out), (1, 2, false, 1));
    let doc = reread(&s);
    let ids = doc.object_ids();
    assert_eq!(ids[0], r, "the ring keeps its place and id");
    assert!(!ids.contains(&two));
    assert_eq!(
        outline_sigs(&doc.path(r).unwrap()),
        outline_sigs(&before_ring),
        "not rewritten"
    );
    assert_eq!(anchor_ids(&doc.path(r).unwrap()), anchor_ids(&before_ring));
    assert_eq!(s.selected_object_count(), 3);
}

fn reread_path(d: &Document, id: NodeId) -> PathSnapshot {
    d.path(id).unwrap()
}

#[test]
fn c7_a_one_piece_compound_path_is_refused_with_its_count_and_nothing_changes() {
    for n in 1..=3usize {
        let d = Document::new(1);
        for i in 0..n {
            ring(&d, 100.0 * i as f64, 0.0);
        }
        let mut s = session_of(&d);
        select_all(&mut s);
        let before = json(&s);
        let labels_before = label_count(&s);
        assert_eq!(
            split_refusal(s.apply_split()),
            SplitRefusal::OnePiece {
                compounds: n,
                with_holes: true
            }
        );
        assert_nothing_changed(&s, &before, labels_before, n);
    }
}

#[test]
fn c8_one_commit_labelled_split_and_undo_redo_are_byte_equal() {
    let d = Document::new(1);
    d.create_path(&square(-300.0, 0.0, 10.0), true);
    compound(
        &d,
        vec![
            circle(0.0, 0.0, 10.0, false),
            circle(50.0, 0.0, 10.0, false),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let before_json = json(&s);
    let before = labels(&reread(&s));
    split_applied(&s.apply_split());
    assert_eq!(
        new_labels(&before, &s),
        vec!["split".to_string()],
        "exactly one commit"
    );
    let after_json = json(&s);
    assert_ne!(after_json, before_json);
    assert!(s.undo_len() >= 1);
    s.undo();
    assert_eq!(
        json(&s),
        before_json,
        "undo restores the document byte for byte"
    );
    s.redo();
    assert_eq!(
        json(&s),
        after_json,
        "redo restores the result byte for byte"
    );
    s.undo();
    assert_eq!(json(&s), before_json);
}

// ---------------------------------------------------------------------------------------------
// Criterion 9 and 13: refusals
// ---------------------------------------------------------------------------------------------

#[test]
fn c9_c13_no_compound_path_refuses_all_three_session_functions_without_a_change() {
    let d = Document::new(1);
    d.create_path(&square(0.0, 0.0, 20.0), true);
    d.create_path(&circle(100.0, 0.0, 10.0, false), true);
    let mut s = session_of(&d);
    for selected in [0usize, 2] {
        if selected == 2 {
            select_all(&mut s);
        }
        let before = json(&s);
        let n = label_count(&s);
        for _ in 0..2 {
            assert_eq!(split_refusal(s.apply_split()), SplitRefusal::NoCompound);
            assert_eq!(
                break_refusal(s.apply_break_apart()),
                BreakApartRefusal::NoCompound
            );
            assert_nothing_changed(&s, &before, n, selected);
        }
    }
}

#[test]
fn c9_c13_a_group_in_the_selection_is_refused_and_names_the_leaves() {
    let d = Document::new(1);
    let r = ring(&d, 0.0, 0.0);
    let a = d.create_path(&square(100.0, 0.0, 10.0), true);
    let b = d.create_path(&square(150.0, 0.0, 10.0), true);
    let inner = compound(
        &d,
        vec![
            circle(300.0, 0.0, 10.0, false),
            circle(350.0, 0.0, 10.0, false),
        ],
    );
    d.group_objects(&[a, b, inner]).unwrap();
    let mut s = session_of(&d);
    select_all(&mut s);
    assert!(s.path_availability().compound, "the ring is selected too");
    let leaves = s.selected_object_count();
    assert_eq!(leaves, 4, "the ring and the three leaves of the group");
    let before = json(&s);
    let n = label_count(&s);
    for _ in 0..2 {
        match split_refusal(s.apply_split()) {
            SplitRefusal::Groups { offenders } => {
                let set: HashSet<NodeId> = offenders.into_iter().collect();
                assert_eq!(set, HashSet::from([a, b, inner]), "the leaves of the group");
                assert!(!set.contains(&r));
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(
            break_refusal(s.apply_break_apart()),
            BreakApartRefusal::Groups { .. }
        ));
        assert_nothing_changed(&s, &before, n, leaves);
    }
    // The ring alone still works while the group sits next to it.
    marquee(&mut s, pt(-30.0, -30.0), pt(30.0, 30.0));
    assert_eq!(selected_ids(&mut s), vec![r]);
}

#[test]
fn c9_a_compound_path_inside_an_entered_group_is_split_inside_that_group() {
    let d = Document::new(1);
    let below = d.create_path(&square(0.0, 300.0, 10.0), true);
    let a = d.create_path(&square(0.0, 0.0, 20.0), true);
    let comp = compound(
        &d,
        vec![
            circle(100.0, 10.0, 10.0, false),
            circle(160.0, 10.0, 10.0, false),
        ],
    );
    let group = d.group_objects(&[a, comp]).unwrap();
    let mut s = session_of(&d);
    // Enter the group by double-clicking a leaf.
    assert!(s.double_click(pt(100.0, 20.0), false, false) || s.selected_object_count() > 0);
    select_all(&mut s);
    assert!(
        s.path_availability().compound,
        "the compound path is selectable inside"
    );
    let out = s.apply_split();
    assert_eq!(split_applied(&out).1, 2);
    let doc = reread(&s);
    assert!(doc.is_group(group), "the group survives");
    let kids = doc.children(Some(group));
    assert_eq!(
        kids.len(),
        3,
        "square and two pieces stay in the group: {kids:?}"
    );
    assert_eq!(kids[0], a);
    assert!(!kids.contains(&comp));
    assert_eq!(doc.object_ids().len(), 4);
    assert!(doc.object_ids().contains(&below));
}

// ---------------------------------------------------------------------------------------------
// Criterion 10: round trips
// ---------------------------------------------------------------------------------------------

#[test]
fn c10_combine_then_split_returns_every_shape_node_for_node_with_the_holes_kept() {
    // Shapes inside shapes: a plate with a hole and a disc in the hole (E5), plus two lone shapes.
    let d = Document::new(1);
    let plate = circle(100.0, 100.0, 20.0, false);
    let hole = circle(100.0, 100.0, 10.0, true);
    let island = circle(100.0, 100.0, 5.0, false);
    let sq1 = square(400.0, 0.0, 30.0);
    let sq2 = square(500.0, 0.0, 30.0);
    for o in [&plate, &hole, &island, &sq1, &sq2] {
        d.create_path(o, true);
    }
    let mut s = session_of(&d);
    select_all(&mut s);
    assert!(matches!(s.apply_combine(), CombineOutcome::Applied { .. }));
    assert_eq!(paths(&s).len(), 1);
    s.apply_split();
    let ps = paths(&s);
    assert_eq!(
        ps.len(),
        4,
        "plate with its hole, the island, and the two squares: {}",
        ps.len()
    );
    // The squares come back exactly (same direction, same start).
    let squares: Vec<&PathSnapshot> = ps
        .iter()
        .filter(|p| !p.is_compound() && p.anchors.len() == 4 && p.anchors[0].point.x > 300.0)
        .collect();
    assert_eq!(squares.len(), 2);
    for (p, orig) in squares.iter().zip([&sq1, &sq2]) {
        assert_eq!(point_set(&p.anchors), point_set(orig));
    }
    let ring_piece = ps
        .iter()
        .find(|p| p.is_compound())
        .expect("one piece keeps its hole");
    assert_eq!(ring_piece.subpaths().count(), 2);
    assert!(painted(ring_piece, 115.0, 100.0) && !painted(ring_piece, 105.0, 100.0));
    let sets = outline_point_sets(ring_piece);
    assert!(sets.contains(&point_set(&plate)) && sets.contains(&point_set(&hole)));
    let disc = ps
        .iter()
        .find(|p| !p.is_compound() && point_set(&p.anchors) == point_set(&island))
        .expect("the island is its own object");
    assert!(painted(disc, 100.0, 100.0));
}

#[test]
fn c10_split_then_combine_is_again_one_compound_path_with_the_same_outlines() {
    let d = Document::new(1);
    let comp = compound(
        &d,
        vec![
            circle(100.0, 100.0, 20.0, false),
            circle(100.0, 100.0, 10.0, true),
            circle(100.0, 100.0, 5.0, false),
            square(300.0, 0.0, 30.0),
        ],
    );
    let orig = d.path(comp).unwrap();
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(split_applied(&s.apply_split()), (1, 3, true, 0));
    select_all(&mut s);
    let out = s.apply_combine();
    assert!(
        matches!(out, CombineOutcome::Applied { .. }),
        "Combine takes the pieces back: {out:?}"
    );
    let ps = paths(&s);
    assert_eq!(ps.len(), 1);
    assert!(ps[0].is_compound());
    let mut want = outline_point_sets(&orig);
    let mut got = outline_point_sets(&ps[0]);
    want.sort();
    got.sort();
    assert_eq!(got, want, "the same outlines");
    assert!(painted(&ps[0], 115.0, 100.0));
    assert!(!painted(&ps[0], 107.0, 100.0));
    assert!(painted(&ps[0], 100.0, 100.0));
}

// ---------------------------------------------------------------------------------------------
// Criteria 11, 12, 13: Break apart
// ---------------------------------------------------------------------------------------------

#[test]
fn c11_e1_a_ring_breaks_into_a_disc_and_a_disc_on_top_of_it() {
    let d = Document::new(1);
    let below = d.create_path(&square(-300.0, 0.0, 10.0), true);
    let outer = circle(100.0, 100.0, 20.0, false);
    let inner = circle(100.0, 100.0, 10.0, true);
    let comp = compound(&d, vec![outer.clone(), inner.clone()]);
    styled(&d, comp, (1, 2, 3), 0.8, (200, 100, 50));
    let style = d.path(comp).unwrap().style;
    let above = d.create_path(&square(-300.0, 300.0, 10.0), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = labels(&reread(&s));
    let out = s.apply_break_apart();
    assert_eq!(break_applied(&out), (1, 2, true));
    assert_eq!(new_labels(&before, &s), vec!["break_apart".to_string()]);
    let doc = reread(&s);
    let ids = doc.object_ids();
    assert_eq!(ids.len(), 4);
    assert_eq!((ids[0], ids[3]), (below, above));
    let big = doc.path(ids[1]).unwrap();
    let small = doc.path(ids[2]).unwrap();
    assert!(is_closed_plain(&big) && is_closed_plain(&small));
    assert_eq!(
        outline_sigs(&big)[0].1,
        outer.iter().map(node_sig).collect::<Vec<_>>()
    );
    assert_eq!(
        outline_sigs(&small)[0].1,
        inner.iter().map(node_sig).collect::<Vec<_>>()
    );
    assert_eq!(big.style, style);
    assert_eq!(small.style, style);
    assert!(
        painted(&small, 100.0, 100.0),
        "the hole is a filled shape now"
    );
    assert!(
        painted(&big, 100.0, 100.0),
        "and covers the middle of the bigger disc"
    );
    assert_eq!(s.selected_object_count(), 4);
    assert_eq!(selected_ids(&mut s).len(), 4);
    // Undo and redo, byte-equal.
    let after = json(&s);
    s.undo();
    assert!(object_ids(&s).contains(&comp));
    s.redo();
    assert_eq!(json(&s), after);
}

#[test]
fn c11_e5_gives_three_closed_paths_and_e2_two_in_outline_order() {
    let d = Document::new(1);
    compound(
        &d,
        vec![
            circle(100.0, 100.0, 20.0, false),
            circle(100.0, 100.0, 10.0, true),
            circle(100.0, 100.0, 5.0, false),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(break_applied(&s.apply_break_apart()), (1, 3, true));
    let ps = paths(&s);
    let radii: Vec<f64> = ps
        .iter()
        .map(|p| p.anchors.iter().map(|a| a.point.x).fold(f64::MIN, f64::max) - 100.0)
        .collect();
    assert_eq!(
        radii,
        vec![20.0, 10.0, 5.0],
        "in the compound path's outline order"
    );
    assert!(ps.iter().all(is_closed_plain));

    // E2.
    let d = Document::new(1);
    compound(
        &d,
        vec![
            circle(0.0, 0.0, 10.0, false),
            circle(50.0, 0.0, 10.0, false),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(
        break_applied(&s.apply_break_apart()),
        (1, 2, false),
        "no hole: no hole sentence"
    );
}

#[test]
fn c12_break_apart_without_a_hole_reports_none_and_touching_outlines_are_just_outlines() {
    // Overlapping squares: Break apart has no touch rule; every outline is a piece.
    let d = Document::new(1);
    compound(
        &d,
        vec![
            square(0.0, 0.0, 20.0),
            square(10.0, 10.0, 20.0),
            square(500.0, 0.0, 5.0),
        ],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let (c, n, _) = break_applied(&s.apply_break_apart());
    assert_eq!((c, n), (1, 3));
}

#[test]
fn c12_break_apart_leaves_other_objects_selected_and_keeps_the_tool() {
    let d = Document::new(1);
    let keep = d.create_path(&square(-100.0, 0.0, 10.0), true);
    ring(&d, 100.0, 100.0);
    ring(&d, 300.0, 100.0);
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(break_applied(&s.apply_break_apart()), (2, 4, true));
    assert_eq!(s.selected_object_count(), 5);
    assert_eq!(object_ids(&s)[0], keep);
    // Break apart has no "one piece" refusal and a second press on the pieces is refused only
    // because there is no compound path left.
    assert_eq!(
        break_refusal(s.apply_break_apart()),
        BreakApartRefusal::NoCompound
    );
}

// ---------------------------------------------------------------------------------------------
// Criterion 14: the six inputs of the sheet
// ---------------------------------------------------------------------------------------------

fn e4_document() -> Document {
    let d = Document::new(1);
    d.create_path(&circle(100.0, 100.0, 10.0, false), true);
    d.create_path(&circle(110.0, 100.0, 10.0, false), true);
    d
}

#[test]
fn c14_e1_ring() {
    let build = || {
        let d = Document::new(1);
        let r = ring(&d, 0.0, 0.0);
        (session_of(&d), r)
    };
    let (mut s, r) = build();
    select_all(&mut s);
    assert!(
        s.path_availability().compound,
        "Split and Break apart enabled"
    );
    let before = json(&s);
    assert_eq!(
        split_refusal(s.apply_split()),
        SplitRefusal::OnePiece {
            compounds: 1,
            with_holes: true
        }
    );
    assert_eq!(json(&s), before);
    match s.apply_cut() {
        CutOutcome::Refused(CutRefusal::Compound { offenders }) => assert_eq!(offenders, vec![r]),
        other => panic!("{other:?}"),
    }
    assert_eq!(json(&s), before);
    assert_eq!(break_applied(&s.apply_break_apart()), (1, 2, true));
    assert_eq!(paths(&s).len(), 2);
}

#[test]
fn c14_e2_two_circles_combined_into_one_compound_path() {
    let build = || {
        let d = Document::new(1);
        let c = compound(
            &d,
            vec![
                circle(0.0, 0.0, 10.0, false),
                circle(50.0, 0.0, 10.0, false),
            ],
        );
        (session_of(&d), c)
    };
    let (mut s, c) = build();
    select_all(&mut s);
    match s.apply_cut() {
        CutOutcome::Refused(CutRefusal::Compound { offenders }) => assert_eq!(offenders, vec![c]),
        other => panic!("{other:?}"),
    }
    assert_eq!(split_applied(&s.apply_split()), (1, 2, false, 0));
    assert!(paths(&s).iter().all(is_closed_plain));
    let (mut s, _) = build();
    select_all(&mut s);
    assert_eq!(break_applied(&s.apply_break_apart()), (1, 2, false));
    assert!(paths(&s).iter().all(is_closed_plain));
}

#[test]
fn c14_e3_the_same_two_circles_as_two_ordinary_objects() {
    let d = Document::new(1);
    d.create_path(&circle(0.0, 0.0, 10.0, false), true);
    d.create_path(&circle(50.0, 0.0, 10.0, false), true);
    let mut s = session_of(&d);
    select_all(&mut s);
    let a = s.path_availability();
    assert!(!a.compound, "Split and Break apart dimmed");
    assert!(a.cut, "Cut enabled");
    let before = json(&s);
    assert_eq!(split_refusal(s.apply_split()), SplitRefusal::NoCompound);
    assert_eq!(
        break_refusal(s.apply_break_apart()),
        BreakApartRefusal::NoCompound
    );
    assert_eq!(s.apply_cut(), CutOutcome::Refused(CutRefusal::NoCrossing));
    assert_eq!(json(&s), before);
}

#[test]
fn c14_e4_two_overlapping_circles_cut_into_four_arcs_at_the_crossings() {
    let d = e4_document();
    let mut s = session_of(&d);
    select_all(&mut s);
    let a = s.path_availability();
    assert!(!a.compound && a.cut);
    assert_eq!(split_refusal(s.apply_split()), SplitRefusal::NoCompound);
    assert_eq!(
        break_refusal(s.apply_break_apart()),
        BreakApartRefusal::NoCompound
    );
    assert_eq!(
        s.apply_cut(),
        CutOutcome::Applied {
            paths: 2,
            pieces: 4
        }
    );
    let ps = paths(&s);
    assert_eq!(ps.len(), 4);
    let cross = [(105.0, 100.0 + 75f64.sqrt()), (105.0, 100.0 - 75f64.sqrt())];
    for p in &ps {
        assert!(!p.closed && !p.is_compound());
        assert!(!p.style.fill.enabled, "Fill None");
        let (e0, e1) = endpoints(p);
        let near = |q: Xy| cross.iter().position(|c| dist(*c, q) < 0.01);
        let (i0, i1) = (near(e0), near(e1));
        assert!(
            i0.is_some() && i1.is_some() && i0 != i1,
            "arc from one crossing to the other: {e0:?} {e1:?}"
        );
    }
    // Two arcs per circle.
    let on = |p: &PathSnapshot, cx: f64| {
        sample_piece(p)
            .iter()
            .all(|q| (dist(*q, (cx, 100.0)) - 10.0).abs() < 0.01)
    };
    assert_eq!(ps.iter().filter(|p| on(p, 100.0)).count(), 2);
    assert_eq!(ps.iter().filter(|p| on(p, 110.0)).count(), 2);
}

#[test]
fn c14_e5_a_ring_with_a_disc_in_its_hole() {
    let build = || {
        let d = Document::new(1);
        let c = compound(
            &d,
            vec![
                circle(0.0, 0.0, 20.0, false),
                circle(0.0, 0.0, 10.0, true),
                circle(0.0, 0.0, 5.0, false),
            ],
        );
        (session_of(&d), c)
    };
    let (mut s, c) = build();
    select_all(&mut s);
    assert_eq!(split_applied(&s.apply_split()), (1, 2, true, 0));
    let ps = paths(&s);
    assert!(ps[0].is_compound() && is_closed_plain(&ps[1]));
    let (mut s, _) = build();
    select_all(&mut s);
    assert_eq!(break_applied(&s.apply_break_apart()), (1, 3, true));
    let (mut s, _) = build();
    select_all(&mut s);
    match s.apply_cut() {
        CutOutcome::Refused(CutRefusal::Compound { offenders }) => assert_eq!(offenders, vec![c]),
        other => panic!("{other:?}"),
    }
}

#[test]
fn c14_e6_the_union_of_the_two_circles_is_one_outline_that_neither_splits_nor_cuts() {
    let d = e4_document();
    let mut s = session_of(&d);
    select_all(&mut s);
    s.apply_boolean(BooleanOp::Union);
    let ps = paths(&s);
    assert_eq!(ps.len(), 1);
    assert!(is_closed_plain(&ps[0]), "an ordinary closed path");
    select_all(&mut s);
    let a = s.path_availability();
    assert!(!a.compound, "Split and Break apart dimmed");
    assert!(a.cut, "Cut enabled");
    let before = json(&s);
    assert_eq!(split_refusal(s.apply_split()), SplitRefusal::NoCompound);
    assert_eq!(
        break_refusal(s.apply_break_apart()),
        BreakApartRefusal::NoCompound
    );
    assert_eq!(s.apply_cut(), CutOutcome::Refused(CutRefusal::NoCrossing));
    assert_eq!(json(&s), before);
}

// ---------------------------------------------------------------------------------------------
// Criterion 16: determinism
// ---------------------------------------------------------------------------------------------

fn bits(s: &Session) -> Vec<Vec<(bool, Vec<NodeSig>)>> {
    paths(s).iter().map(outline_sigs).collect()
}

#[test]
fn c16_the_result_does_not_depend_on_how_the_objects_were_selected() {
    let build = || {
        let d = Document::new(1);
        compound(
            &d,
            vec![
                circle(100.0, 100.0, 20.0, false),
                circle(100.0, 100.0, 10.0, true),
                circle(100.0, 100.0, 5.0, false),
                square(300.0, 100.0, 20.0),
            ],
        );
        d.create_path(&square(500.0, 100.0, 20.0), true);
        d
    };
    let d = build();
    // Pack once so ids are the same in all runs.
    let bytes = pack_of(&d);
    let open = |bytes: &[u8]| {
        let mut s = Session::open(2, bytes).unwrap();
        s.resize_viewport(1600.0, 1000.0);
        s.set_tool(Tool::Select);
        s
    };
    let mut a = open(&bytes);
    select_all(&mut a);
    let mut b = open(&bytes);
    // Shift-click the objects in reverse order.
    for p in [pt(520.0, 100.0), pt(120.0, 100.0)] {
        b.pointer_hover(p, true, false);
        b.pointer_down(p, true);
        b.pointer_up(p, true, false);
    }
    let mut c = open(&bytes);
    marquee(&mut c, pt(0.0, 0.0), pt(1000.0, 300.0));
    for s in [&mut a, &mut b, &mut c] {
        assert_eq!(
            s.selected_object_count(),
            2,
            "the compound path and the square"
        );
        s.apply_split();
    }
    assert_eq!(bits(&a), bits(&b));
    assert_eq!(bits(&a), bits(&c));
    // The same input twice gives the same document, byte for byte.
    let mut a2 = open(&bytes);
    select_all(&mut a2);
    a2.apply_split();
    assert_eq!(bits(&a), bits(&a2));
    assert_eq!(json(&a), json(&a2), "same input, same document bytes");
    // Break apart likewise.
    let mut x = open(&bytes);
    select_all(&mut x);
    x.apply_break_apart();
    let mut y = open(&bytes);
    select_all(&mut y);
    y.apply_break_apart();
    assert_eq!(json(&x), json(&y));
}

fn pack_of(d: &Document) -> Vec<u8> {
    curvyo_document_core::pack(d, "0.1.0").unwrap()
}

// ---------------------------------------------------------------------------------------------
// Hostile input
// ---------------------------------------------------------------------------------------------

#[test]
fn hostile_degenerate_outlines_do_not_panic_and_nothing_is_lost() {
    let degenerate: Vec<(&str, Vec<NewAnchor>)> = vec![
        ("one node", vec![corner(300.0, 0.0)]),
        ("two nodes", vec![corner(300.0, 0.0), corner(310.0, 0.0)]),
        (
            "zero area",
            vec![corner(300.0, 0.0), corner(310.0, 0.0), corner(320.0, 0.0)],
        ),
        (
            "all nodes equal",
            vec![corner(300.0, 0.0), corner(300.0, 0.0), corner(300.0, 0.0)],
        ),
        (
            "doubled nodes",
            vec![
                corner(300.0, 0.0),
                corner(300.0, 0.0),
                corner(310.0, 0.0),
                corner(310.0, 10.0),
            ],
        ),
    ];
    for (name, bad) in degenerate {
        let d = Document::new(1);
        let nodes = 4 + bad.len();
        compound(
            &d,
            vec![
                square(0.0, 0.0, 20.0),
                bad.clone(),
                square(100.0, 0.0, 20.0),
            ],
        );
        let _ = nodes;
        let mut s = session_of(&d);
        select_all(&mut s);
        let before = json(&s);
        let total_nodes = |s: &Session| -> usize {
            paths(s)
                .iter()
                .map(|p| p.subpaths().map(|sp| sp.anchors.len()).sum::<usize>())
                .sum()
        };
        let want = total_nodes(&s);
        let out = s.apply_split();
        match out {
            SplitOutcome::Applied { .. } => {
                assert_eq!(total_nodes(&s), want, "{name}: no node lost");
            }
            SplitOutcome::Refused(_) => assert_eq!(json(&s), before, "{name}"),
            SplitOutcome::Ignored => panic!("{name}: ignored"),
        }
        let mut s2 = session_of(&d);
        select_all(&mut s2);
        let out = s2.apply_break_apart();
        let (_, n, _) = break_applied(&out);
        // `0054` criterion 18: an outline of a single node is dropped (and counted); every other
        // outline becomes one piece with all its nodes.
        let dropped = usize::from(bad.len() < 2);
        assert_eq!(n, 3 - dropped, "{name}: one piece per outline");
        assert_eq!(
            total_nodes(&s2),
            want - dropped,
            "{name}: no node lost but a single one"
        );
    }
}

/// One compound path of two squares and a triangle with one bad x coordinate. Split or Break
/// apart must answer quickly, without panic and without a huge allocation: no arithmetic beyond
/// ordering is needed for Break apart, and Split may refuse but must not change anything then.
fn hostile_value(name: &str, x: f64, command: &str) {
    let d = Document::new(1);
    let bad = vec![corner(x, 0.0), corner(1.0, 1.0), corner(5.0, 10.0)];
    compound(
        &d,
        vec![square(0.0, 0.0, 20.0), bad, square(100.0, 0.0, 20.0)],
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    let before = json(&s);
    let t = Instant::now();
    if command == "split" {
        let out = s.apply_split();
        if let SplitOutcome::Refused(_) | SplitOutcome::Ignored = out {
            assert_eq!(json(&s), before, "{name}: a refusal changes nothing");
        }
    } else {
        let out = s.apply_break_apart();
        assert_eq!(break_applied(&out).1, 3, "{name}: one piece per outline");
        let has = paths(&s)
            .iter()
            .any(|p| p.anchors.iter().any(|a| a.point.x.to_bits() == x.to_bits()));
        assert!(has || x.is_nan(), "{name}: the node keeps its value");
    }
    assert!(t.elapsed().as_secs() < 5, "{name}: {command} took too long");
}

macro_rules! hostile {
    ($($test:ident: $x:expr, $cmd:literal;)*) => {$(
        #[test]
        fn $test() {
            hostile_value(stringify!($test), $x, $cmd);
        }
    )*};
}

hostile! {
    hostile_split_nan: f64::NAN, "split";
    hostile_split_infinity: f64::INFINITY, "split";
    hostile_split_1e300: 1.0e300, "split";
    hostile_split_minus_1e300: -1.0e300, "split";
    hostile_split_beyond_kernel_range: 2.0e7, "split";
    hostile_split_inside_kernel_range: 9.9e6, "split";
    hostile_split_1e5: 1.0e5, "split";
    hostile_break_apart_nan: f64::NAN, "break_apart";
    hostile_break_apart_infinity: f64::INFINITY, "break_apart";
    hostile_break_apart_1e300: 1.0e300, "break_apart";
    hostile_break_apart_beyond_kernel_range: 2.0e7, "break_apart";
}

#[test]
fn hostile_empty_selection_and_a_huge_part_count_in_a_modest_document() {
    let d = Document::new(1);
    ring(&d, 0.0, 0.0);
    let mut s = session_of(&d);
    let before = json(&s);
    assert_eq!(split_refusal(s.apply_split()), SplitRefusal::NoCompound);
    assert_eq!(
        break_refusal(s.apply_break_apart()),
        BreakApartRefusal::NoCompound
    );
    assert_eq!(json(&s), before);
    // Sanity at scale: 300 touching-free squares.
    let d = Document::new(1);
    compound(
        &d,
        (0..300)
            .map(|i| square(30.0 * f64::from(i), 0.0, 20.0))
            .collect(),
    );
    let mut s = session_of(&d);
    select_all(&mut s);
    assert_eq!(split_applied(&s.apply_split()), (1, 300, false, 0));
    assert_eq!(s.selected_object_count(), 300);
}

// ---------------------------------------------------------------------------------------------
// Criterion 15: performance inputs (release, `--ignored`)
// ---------------------------------------------------------------------------------------------

fn perf_input(which: char) -> Vec<Vec<NewAnchor>> {
    match which {
        'a' => {
            let mut o = vec![square(0.0, 0.0, 10_000.0)];
            for i in 0..1000 {
                let (cx, cy) = (
                    200.0 + 300.0 * f64::from(i % 32),
                    200.0 + 300.0 * f64::from(i / 32),
                );
                o.push(circle(cx, cy, 50.0, true));
            }
            o
        }
        'b' => (0..2000)
            .map(|i| square(30.0 * f64::from(i % 50), 30.0 * f64::from(i / 50), 20.0))
            .collect(),
        'c' => (0..500)
            .map(|i| {
                let side = 4000.0 - 8.0 * f64::from(i);
                let mut sq = square(4.0 * f64::from(i), 4.0 * f64::from(i), side);
                if i % 2 == 1 {
                    sq.reverse();
                }
                sq
            })
            .collect(),
        'd' => (0..5000)
            .map(|i| square(0.02 * f64::from(i % 100), 0.02 * f64::from(i / 100), 0.01))
            .collect(),
        _ => unreachable!(),
    }
}

fn perf_session(which: char) -> Session {
    let d = Document::new(1);
    compound(&d, perf_input(which));
    let mut s = session_of(&d);
    select_all(&mut s);
    s
}

#[test]
#[ignore = "performance: run with --release -- --ignored"]
fn c15_split_golden_counts_and_budgets() {
    for (which, expect) in [
        ('a', None),
        ('b', Some(2000usize)),
        ('c', Some(250)),
        ('d', Some(5000)),
    ] {
        let mut s = perf_session(which);
        let t = Instant::now();
        let out = s.apply_split();
        let ms = t.elapsed().as_millis();
        eprintln!("split ({which}): {ms} ms");
        assert!(ms < 2000, "split ({which}) took {ms} ms");
        match expect {
            None => {
                assert_eq!(
                    split_refusal(out),
                    SplitRefusal::OnePiece {
                        compounds: 1,
                        with_holes: true
                    }
                );
                assert!(ms < 500, "the part search of (a) alone took {ms} ms");
            }
            Some(n) => assert_eq!(split_applied(&out).1, n, "({which})"),
        }
        if which == 'c' {
            assert!(
                paths(&s)
                    .iter()
                    .all(|p| p.is_compound() && p.subpaths().count() == 2)
            );
        }
        if which == 'b' || which == 'd' {
            assert!(paths(&s).iter().all(is_closed_plain));
        }
    }
}

#[test]
#[ignore = "performance: run with --release -- --ignored"]
fn c15_break_apart_golden_counts_and_budgets() {
    for (which, n) in [('a', 1001usize), ('b', 2000), ('c', 500), ('d', 5000)] {
        let mut s = perf_session(which);
        let t = Instant::now();
        let out = s.apply_break_apart();
        let ms = t.elapsed().as_millis();
        eprintln!("break apart ({which}): {ms} ms");
        assert!(ms < 2000, "break apart ({which}) took {ms} ms");
        assert_eq!(break_applied(&out).1, n, "({which})");
        assert!(paths(&s).iter().all(is_closed_plain));
    }
}

#[test]
#[ignore = "performance: run with --release -- --ignored"]
fn c15_split_input_c_is_two_commands_and_stays_correct_after_undo() {
    let mut s = perf_session('c');
    let before = json(&s);
    s.apply_split();
    s.undo();
    assert_eq!(json(&s), before);
    let t = Instant::now();
    s.apply_break_apart();
    assert!(t.elapsed().as_millis() < 2000);
}

#[allow(dead_code)]
fn unused(_: KeyInput<'_>, _: CombineOutcome) {}
