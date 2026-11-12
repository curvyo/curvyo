//! Independent acceptance tests for `specs/0101-laser-operations` on the pure rules of
//! `curvyo-laser-core` (ADR 0046 §7): the run sequence and Cuts last (criteria 5 to 7), the
//! naming and the colours (2, 3), each kind's geometry (19 to 22, 24), the nearest rule and the
//! hidden-leaf rule (12, 13), the summary (17) and Cut-out (23).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    missing_docs
)]

use curvyo_document_core::{
    Document, DocumentSize, Length, NodeId, OperationSnapshot, Point, RectBounds, Tolerance, Vec2,
};
use curvyo_geometry_core::Outline;
use curvyo_geometry_core::OutlineTriple;
use curvyo_laser_core::operations::{
    ChainScope, Hidden, KindRead, Leaf, LeafShape, Lines, OperationKind, OperationList, Section,
    cut_out_contours, kind_run, leaf_status, next_operation_name, run_sequence, summarize,
};

fn ids(n: usize) -> Vec<NodeId> {
    let d = Document::new(1);
    (0..n)
        .map(|i| {
            d.create_rect(RectBounds::from_corners(
                Point::new(i as f64 * 30.0, 0.0),
                Point::new(i as f64 * 30.0 + 20.0, 10.0),
            ))
        })
        .collect()
}

fn op(
    id: NodeId,
    name: &str,
    kind: Option<&str>,
    output: bool,
    members: Vec<Vec<NodeId>>,
) -> OperationSnapshot {
    OperationSnapshot {
        id,
        name: name.to_owned(),
        kind: kind.map(str::to_owned),
        show: true,
        output,
        margin: Length::from_mm(0.0),
        members,
        settings: None,
        options: None,
        grid: false,
    }
}

fn known(kind: OperationKind) -> KindRead {
    KindRead::Known(kind)
}

// ---- criteria 5 to 7: the run sequence ---------------------------------------------------------

#[test]
fn c5_cuts_last_on_runs_marking_in_list_order_then_cutting_in_list_order() {
    // List: Cut, Engrave, Score, CutOut, EngraveOutline.
    let kinds = [
        known(OperationKind::Cut),
        known(OperationKind::Engrave),
        known(OperationKind::Score),
        known(OperationKind::CutOut),
        known(OperationKind::EngraveOutline),
    ];
    assert_eq!(run_sequence(&kinds, true), vec![1, 2, 4, 0, 3]);
}

#[test]
fn c6_cuts_last_off_keeps_the_list_order() {
    let kinds = [
        known(OperationKind::Cut),
        known(OperationKind::Engrave),
        known(OperationKind::Score),
    ];
    assert_eq!(run_sequence(&kinds, false), vec![0, 1, 2]);
}

#[test]
fn c5_run_sequence_is_a_permutation_with_unknown_operations_in_it() {
    let kinds = [
        known(OperationKind::Cut),
        KindRead::Unknown,
        known(OperationKind::Engrave),
    ];
    let mut seq = run_sequence(&kinds, true);
    seq.sort_unstable();
    assert_eq!(
        seq,
        vec![0, 1, 2],
        "every operation appears once; an unknown one runs nothing"
    );
}

#[test]
fn c5_a_cut_is_refused_above_the_section_line_and_a_score_is_not() {
    assert_eq!(OperationKind::Cut.section(), Section::Cutting);
    assert_eq!(OperationKind::CutOut.section(), Section::Cutting);
    assert_eq!(OperationKind::Score.section(), Section::Marking);
    assert_eq!(OperationKind::Engrave.section(), Section::Marking);
    assert_eq!(OperationKind::EngraveOutline.section(), Section::Marking);
}

// ---- criteria 2 and 3: names and colours -------------------------------------------------------

#[test]
fn c2_the_name_is_the_lowest_unused_number_of_its_kind() {
    assert_eq!(next_operation_name(OperationKind::Cut, []), "Cut 1");
    assert_eq!(
        next_operation_name(OperationKind::Cut, ["Cut 1", "Cut 3"]),
        "Cut 2"
    );
    assert_eq!(
        next_operation_name(OperationKind::Cut, ["Cut 1", "Cut 2"]),
        "Cut 3"
    );
    assert_eq!(
        next_operation_name(OperationKind::Cut, ["Score 1", "Score 2"]),
        "Cut 1",
        "other kinds do not take the number"
    );
    assert_eq!(
        next_operation_name(OperationKind::Cut, ["Cut 1", "Cut x", "Cut 0"]),
        "Cut 2",
        "a non-number is not a used number"
    );
}

// Customer 2026-11-10: no operation colour (tester change: the colour test of c2_there_are_eight_colours_and_they_are_op_1_to_op_8 was removed).

// Customer 2026-11-10: no operation colour (tester change: the colour test of c2_the_next_colour_follows_list_order_and_wraps_after_eight was removed).

// ---- criteria 19 to 22: each kind's geometry ---------------------------------------------------

#[test]
fn c19_cut_runs_every_outline_as_drawn_and_never_fill() {
    let run = kind_run(
        OperationKind::Cut,
        LeafShape::Outlines { closed: 2, open: 1 },
    );
    assert_eq!(run.lines, Lines::All);
    assert!(!run.area, "Cut never fills");
    assert!(!run.raster);
}

#[test]
fn c20_score_runs_every_outline_closed_or_open_as_a_line() {
    let run = kind_run(
        OperationKind::Score,
        LeafShape::Outlines { closed: 1, open: 1 },
    );
    assert_eq!(run.lines, Lines::All);
    assert!(!run.area);
    let image = kind_run(OperationKind::Score, LeafShape::Image);
    assert!(image.is_empty(), "an image runs nothing under Score");
}

#[test]
fn c21_engrave_with_a_closed_outline_is_one_area_and_open_only_is_a_line() {
    let mixed = kind_run(
        OperationKind::Engrave,
        LeafShape::Outlines { closed: 1, open: 1 },
    );
    assert!(
        mixed.area,
        "a compound with one closed outline is one filled area"
    );
    assert_eq!(mixed.lines, Lines::None);

    let open_only = kind_run(
        OperationKind::Engrave,
        LeafShape::Outlines { closed: 0, open: 2 },
    );
    assert!(!open_only.area, "all open outlines run as a line");
    assert_eq!(open_only.lines, Lines::All);
}

#[test]
fn c21_engrave_image_is_raster_and_counts() {
    let run = kind_run(OperationKind::Engrave, LeafShape::Image);
    assert!(run.raster);
}

#[test]
fn c22_engrave_outline_is_the_fill_plus_each_closed_outline_traced_once() {
    let run = kind_run(
        OperationKind::EngraveOutline,
        LeafShape::Outlines { closed: 2, open: 0 },
    );
    assert!(run.area);
    assert_eq!(run.lines, Lines::Closed);
    let image = kind_run(OperationKind::EngraveOutline, LeafShape::Image);
    assert!(image.raster);
    assert_eq!(
        image.lines,
        Lines::Frame,
        "an image is raster engraved and its frame traced"
    );
}

#[test]
fn c19_cut_of_an_image_runs_its_frame_only() {
    let run = kind_run(OperationKind::Cut, LeafShape::Image);
    assert_eq!(run.lines, Lines::Frame);
    assert!(!run.raster, "the pixels never run under Cut");
    assert!(!run.area);
}

#[test]
fn c23_cut_out_is_one_contour_per_item_and_nothing_for_an_open_empty_leaf() {
    let run = kind_run(
        OperationKind::CutOut,
        LeafShape::Outlines { closed: 1, open: 0 },
    );
    assert!(run.contour);
    assert!(
        kind_run(
            OperationKind::CutOut,
            LeafShape::Outlines { closed: 0, open: 0 }
        )
        .is_empty()
    );
}

// ---- criterion 23: Cut-out contour -------------------------------------------------------------

fn square(x: f64, y: f64, w: f64, h: f64) -> Vec<OutlineTriple> {
    [(x, y), (x + w, y), (x + w, y + h), (x, y + h)]
        .iter()
        .map(|&(px, py)| (Point::new(px, py), Vec2::ZERO, Vec2::ZERO))
        .collect()
}

fn bbox(contour: &[Point]) -> (f64, f64, f64, f64) {
    let xs: Vec<f64> = contour.iter().map(|p| p.x).collect();
    let ys: Vec<f64> = contour.iter().map(|p| p.y).collect();
    let min = |v: &[f64]| v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = |v: &[f64]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    (min(&xs), min(&ys), max(&xs), max(&ys))
}

#[test]
fn c23_two_clones_and_a_bar_give_one_outer_contour_with_no_line_inside() {
    // Two 20 x 20 squares and a 40 x 4 bar joining them at mid height: one union, one contour.
    let a = square(0.0, 0.0, 20.0, 20.0);
    let b = square(40.0, 0.0, 20.0, 20.0);
    let bar = square(18.0, 8.0, 24.0, 4.0);
    let (oa, ob, ob2) = (
        Outline::new(&a, true),
        Outline::new(&b, true),
        Outline::new(&bar, true),
    );
    let tol = Tolerance::from_mm(0.01);
    let leaves: [&[Outline<'_>]; 3] = [&[oa], &[ob], &[ob2]];
    let contours = cut_out_contours(&leaves, Length::from_mm(0.0), tol).unwrap();
    assert_eq!(contours.len(), 1, "one closed line for the item");
    assert_eq!(bbox(&contours[0]), (0.0, 0.0, 60.0, 20.0));
}

#[test]
fn c23_a_hole_does_not_run_and_the_outer_boundary_does() {
    let outer = square(0.0, 0.0, 30.0, 30.0);
    let hole = square(10.0, 10.0, 10.0, 10.0);
    let (o, h) = (Outline::new(&outer, true), Outline::new(&hole, true));
    let tol = Tolerance::from_mm(0.01);
    let contours = cut_out_contours(&[&[o, h]], Length::from_mm(0.0), tol).unwrap();
    assert_eq!(contours.len(), 1, "holes and inner detail do not run");
    assert_eq!(bbox(&contours[0]), (0.0, 0.0, 30.0, 30.0));
}

#[test]
fn c23_a_positive_margin_grows_the_contour_and_zero_keeps_it() {
    let s = square(0.0, 0.0, 20.0, 20.0);
    let o = Outline::new(&s, true);
    let tol = Tolerance::from_mm(0.01);
    let plain = cut_out_contours(&[&[o]], Length::from_mm(0.0), tol).unwrap();
    assert_eq!(bbox(&plain[0]), (0.0, 0.0, 20.0, 20.0));
    let grown = cut_out_contours(&[&[o]], Length::from_mm(2.0), tol).unwrap();
    let (x0, y0, x1, y1) = bbox(&grown[0]);
    assert!(
        (x0 + 2.0).abs() < 0.05 && (y0 + 2.0).abs() < 0.05,
        "grown by 2 mm: {x0} {y0}"
    );
    assert!(
        (x1 - 22.0).abs() < 0.05 && (y1 - 22.0).abs() < 0.05,
        "grown by 2 mm: {x1} {y1}"
    );
}

// ---- criteria 12 and 13: the nearest rule and hidden leaves ------------------------------------

#[test]
fn c12_a_leaf_takes_the_nearest_assignment_and_not_its_groups() {
    let n = ids(3); // n[0] leaf, n[1] group, n[2] unused
    let list = OperationList::new(
        &[
            op(n[0], "Engrave 1", Some("engrave"), true, vec![vec![n[0]]]),
            op(n[1], "Cut 1", Some("cut"), true, vec![vec![n[1]]]),
        ],
        true,
    );
    let chain = vec![vec![n[0]], vec![n[1]]];
    let got = list.nearest(&chain, ChainScope::Whole).unwrap();
    assert_eq!(got.target, vec![n[0]]);
    assert_eq!(got.operations.len(), 1, "the leaf runs Engrave only");
    assert_eq!(
        list.rules()[got.operations[0]].kind,
        known(OperationKind::Engrave)
    );
}

#[test]
fn c12_a_leaf_with_no_assignment_at_any_level_is_not_assigned() {
    let n = ids(2);
    let list = OperationList::new(
        &[op(n[0], "Cut 1", Some("cut"), true, vec![vec![n[0]]])],
        true,
    );
    let chain = vec![vec![n[1]]];
    assert!(list.nearest(&chain, ChainScope::Whole).is_none());
}

#[test]
fn c12_an_unknown_operation_stops_the_rule_and_runs_nothing() {
    let n = ids(2);
    let list = OperationList::new(
        &[
            op(n[0], "Odd 1", Some("sawdust"), true, vec![vec![n[0]]]),
            op(n[1], "Cut 1", Some("cut"), true, vec![vec![n[1]]]),
        ],
        true,
    );
    // The leaf is in an unknown operation; its group is in a Cut. It must not fall back to Cut.
    let chain = vec![vec![n[0]], vec![n[1]]];
    let got = list.nearest(&chain, ChainScope::Whole).unwrap();
    assert_eq!(
        list.running(&got).count(),
        0,
        "unknown runs nothing and does not fall back"
    );
}

#[test]
fn c12_output_off_operations_run_nothing_in_running() {
    let n = ids(1);
    let list = OperationList::new(
        &[op(n[0], "Cut 1", Some("cut"), false, vec![vec![n[0]]])],
        true,
    );
    let got = list.nearest(&[vec![n[0]]], ChainScope::Whole).unwrap();
    assert_eq!(list.running(&got).count(), 0, "Output off runs nothing");
}

#[test]
fn c13_a_source_leaf_hidden_inside_a_placed_page_uses_only_its_own_key() {
    let n = ids(2); // n[0] the source leaf, n[1] the placement
    let list = OperationList::new(
        &[op(n[1], "Cut 1", Some("cut"), true, vec![vec![n[1]]])],
        true,
    );
    let chain = vec![vec![n[0]], vec![n[1]]];
    assert!(
        list.nearest(&chain, ChainScope::OwnOnly).is_none(),
        "the placement's assignment does not reach a hidden source leaf"
    );
    let own = OperationList::new(
        &[op(n[0], "Cut 1", Some("cut"), true, vec![vec![n[0]]])],
        true,
    );
    assert!(
        own.nearest(&chain, ChainScope::OwnOnly).is_some(),
        "its own assignment runs it"
    );
}

// ---- criterion 17: the summary -----------------------------------------------------------------

#[test]
fn c17_summary_counts_are_disjoint_and_hidden_and_output_off_are_counted_once_each() {
    let n = ids(6);
    let list = OperationList::new(
        &[
            op(
                n[0],
                "Cut 1",
                Some("cut"),
                true,
                vec![vec![n[0]], vec![n[1]], vec![n[2]]],
            ),
            op(n[3], "Score 1", Some("score"), false, vec![vec![n[3]]]),
        ],
        true,
    );
    let shape = LeafShape::Outlines { closed: 1, open: 0 };
    let leaves = [
        Leaf {
            chain: &[vec![n[0]]],
            hidden: Hidden::No,
            shape,
        }, // running (Cut)
        Leaf {
            chain: &[vec![n[1]]],
            hidden: Hidden::InSource,
            shape,
        }, // hidden, own key in Cut: runs while hidden
        Leaf {
            chain: &[vec![n[4]]],
            hidden: Hidden::No,
            shape,
        }, // not assigned
        Leaf {
            chain: &[vec![n[3]]],
            hidden: Hidden::No,
            shape,
        }, // Score, Output off
    ];
    let s = summarize(&list, leaves.iter().copied());
    assert_eq!(s.operations, 1, "only Cut runs; Score is Output off");
    assert_eq!(
        s.running, 2,
        "the visible Cut item and the hidden item with its own key"
    );
    assert_eq!(
        s.runs_while_hidden, 1,
        "criterion 13: a hidden source leaf with its own key is counted"
    );
    assert_eq!(s.not_assigned, 1);
    assert_eq!(
        s.output_off, 1,
        "the Output-off item is in no running or not-assigned count"
    );
    assert!(s.running + s.not_assigned <= leaves.len(), "disjoint");
}

#[test]
fn c13_runs_while_hidden_is_inside_running_and_a_group_does_not_reach_a_hidden_source_leaf() {
    let n = ids(2);
    let list = OperationList::new(
        &[op(n[0], "Cut 1", Some("cut"), true, vec![vec![n[0]]])],
        true,
    );
    let shape = LeafShape::Outlines { closed: 1, open: 0 };
    let own = [Leaf {
        chain: &[vec![n[0]]],
        hidden: Hidden::InSource,
        shape,
    }];
    let s = summarize(&list, own.iter().copied());
    assert_eq!(s.running, 1);
    assert_eq!(s.runs_while_hidden, 1, "counted once, inside running");
    // A hidden source leaf whose only assignment is on a group above it does not run.
    let group_only = [Leaf {
        chain: &[vec![n[1]], vec![n[0]]],
        hidden: Hidden::InSource,
        shape,
    }];
    let s2 = summarize(&list, group_only.iter().copied());
    assert_eq!(
        s2.running, 0,
        "OwnOnly scope: the group's assignment does not reach it"
    );
    assert!(matches!(
        leaf_status(&list, &group_only[0]),
        curvyo_laser_core::operations::LeafStatus::Absent
    ));
}

/// Spec gap, not asserted here: criterion 17 defines "items run while hidden" only through
/// criterion 13 (a hidden placed-page source leaf). The implementation also counts a leaf hidden
/// by its eye on the output page (`Hidden::OnOutputPage`) as runs-while-hidden. Question for the
/// product owner, reported in the verdict.

#[test]
fn c26_an_unknown_operation_is_counted_in_no_summary_line() {
    let n = ids(1);
    let list = OperationList::new(
        &[op(n[0], "Odd 1", Some("sawdust"), true, vec![vec![n[0]]])],
        true,
    );
    let shape = LeafShape::Outlines { closed: 1, open: 0 };
    let leaves = [Leaf {
        chain: &[vec![n[0]]],
        hidden: Hidden::No,
        shape,
    }];
    let s = summarize(&list, leaves.iter().copied());
    assert_eq!(s.running, 0);
    assert_eq!(s.not_assigned, 0);
    assert_eq!(s.output_off, 0);
    assert_eq!(s.operations, 0);
}

#[test]
fn c27_an_assignment_with_no_live_target_is_counted_nowhere() {
    // An assignment for a node that no leaf chain names: no leaf matches it.
    let n = ids(2);
    let list = OperationList::new(
        &[op(n[0], "Cut 1", Some("cut"), true, vec![vec![n[0]]])],
        true,
    );
    let shape = LeafShape::Outlines { closed: 1, open: 0 };
    let leaves = [Leaf {
        chain: &[vec![n[1]]],
        hidden: Hidden::No,
        shape,
    }];
    let s = summarize(&list, leaves.iter().copied());
    assert_eq!(s.running, 0);
    assert_eq!(
        s.not_assigned, 1,
        "the orphan key does not make the leaf assigned"
    );
    assert_eq!(s.operations, 0);
}

#[test]
fn c12_operations_of_a_target_come_back_in_run_order() {
    let n = ids(1);
    let list = OperationList::new(
        &[
            op(n[0], "Cut 1", Some("cut"), true, vec![vec![n[0]]]),
            op(n[0], "Engrave 1", Some("engrave"), true, vec![vec![n[0]]]),
        ],
        true,
    );
    let got = list.operations_of(&[n[0]]).to_vec();
    assert_eq!(got.len(), 2);
    assert_eq!(
        list.sequence()[0],
        got[0],
        "the sequence puts Engrave (marking) first"
    );
    let _ = ChainScope::Whole;
    let _ = KindRead::Unknown;
    let _: Option<Section> = None;
    let _ = DocumentSize::from_mm(1.0, 1.0);
    let _ = OperationKind::ALL;
}
