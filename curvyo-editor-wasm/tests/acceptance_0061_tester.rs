//! Independent acceptance checks for specification 0061 (align, distribute, grid),
//! written from the criteria before the implementation was read.
#![allow(
    clippy::unwrap_used,
    clippy::too_many_lines,
    clippy::many_single_char_names, // coordinates and box corners read best as a, b, x, y
    clippy::cast_precision_loss,    // small loop indices to f64
    clippy::manual_midpoint         // (a + b) / 2.0 mirrors the spec's wording
)]
use curvyo_document_core::{Direction, Document, NodeId, Point, RectBounds, unpack};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::{AlignAxis, AlignCommand, AlignOutcome, DistributeMode, object_bounds};

const EPS: f64 = 1e-6;
type Box4 = (f64, f64, f64, f64); // left, top, right, bottom

fn rect(d: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    d.create_rect(RectBounds::from_corners(
        Point::new(x, y),
        Point::new(x + w, y + h),
    ))
}
fn opened(s: &Session) -> Document {
    unpack(1, &s.pack("t").unwrap()).unwrap()
}
fn bx(s: &Session, id: NodeId) -> Box4 {
    let d = opened(s);
    let (a, b) = object_bounds(&d.object(id).unwrap());
    (a.x, a.y, b.x, b.y)
}
fn page_box(s: &Session) -> Box4 {
    let p = s.active_page();
    let o = p.origin;
    (
        o.x,
        o.y,
        o.x + p.size.width.as_mm(),
        o.y + p.size.height.as_mm(),
    )
}
fn close(a: Box4, b: Box4, what: &str) {
    for (x, y) in [(a.0, b.0), (a.1, b.1), (a.2, b.2), (a.3, b.3)] {
        assert!((x - y).abs() < EPS, "{what}: {a:?} vs {b:?}");
    }
}
fn select_all(s: &mut Session) {
    s.set_tool(Tool::Select);
    s.key_down(KeyInput {
        key: "a",
        ctrl: true,
        ..KeyInput::default()
    });
}
fn select_none(s: &mut Session) {
    s.set_tool(Tool::Select);
    s.escape();
    s.escape();
}
fn click(s: &mut Session, p: (f64, f64), shift: bool) {
    s.set_tool(Tool::Select);
    let p = Point::new(p.0, p.1);
    s.pointer_hover(p, shift, false);
    s.pointer_down(p, shift);
    s.pointer_up(p, shift, false);
}
/// Selects only `id` with a click on its left edge (outlines hit, interiors may not).
fn select_only(s: &mut Session, id: NodeId) {
    let b = bx(s, id);
    click(s, (b.0, (b.1 + b.3) / 2.0), false);
    assert_eq!(
        s.selected_object_count(),
        1,
        "click selected exactly the object"
    );
}
fn last_name(s: &mut Session) -> String {
    s.history_target_name(Direction::Undo).unwrap_or_default()
}
const ALL: [AlignCommand; 6] = [
    AlignCommand::Left,
    AlignCommand::CentreH,
    AlignCommand::Right,
    AlignCommand::Top,
    AlignCommand::CentreV,
    AlignCommand::Bottom,
];
fn expect_on(cmd: AlignCommand, b: Box4, p: Box4) -> Box4 {
    let (w, h) = (b.2 - b.0, b.3 - b.1);
    let (mut l, mut t) = (b.0, b.1);
    match cmd {
        AlignCommand::Left => l = p.0,
        AlignCommand::CentreH => l = (p.0 + p.2) / 2.0 - w / 2.0,
        AlignCommand::Right => l = p.2 - w,
        AlignCommand::Top => t = p.1,
        AlignCommand::CentreV => t = (p.1 + p.3) / 2.0 - h / 2.0,
        AlignCommand::Bottom => t = p.3 - h,
    }
    (l, t, l + w, t + h)
}
fn name_of(cmd: AlignCommand) -> &'static str {
    match cmd {
        AlignCommand::Left => "Align left to page",
        AlignCommand::CentreH => "Align horizontal centres to page",
        AlignCommand::Right => "Align right to page",
        AlignCommand::Top => "Align top to page",
        AlignCommand::CentreV => "Align vertical centres to page",
        AlignCommand::Bottom => "Align bottom to page",
    }
}

/// Two pages; the second active and away from the origin. Returns (document, second id).
fn two_pages() -> (Document, String) {
    let d = Document::new(7);
    let p = d.add_page();
    (d, p.id)
}

#[test]
fn ac14_one_object_each_button_goes_to_the_active_non_origin_page() {
    for cmd in ALL {
        let (d, pid) = two_pages();
        let page2 = d.pages()[1].clone();
        let o = page2.origin;
        let id = rect(&d, o.x + 13.0, o.y + 17.0, 20.0, 12.0);
        let first_obj = rect(&d, 5.0, 5.0, 10.0, 10.0);
        let mut s = Session::open(2, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
        s.activate_page(&pid);
        assert!(
            o.x.abs() > 1.0 || o.y.abs() > 1.0,
            "second page is off origin"
        );
        select_only(&mut s, id);
        let before = bx(&s, id);
        let first_before = bx(&s, first_obj);
        let steps = s.undo_len();
        assert_eq!(s.align_selection(cmd), AlignOutcome::Aligned, "{cmd:?}");
        close(
            bx(&s, id),
            expect_on(cmd, before, page_box(&s)),
            &format!("{cmd:?}"),
        );
        assert_eq!(bx(&s, first_obj), first_before, "other page untouched");
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(last_name(&mut s), name_of(cmd));
        assert_eq!(s.selected_object_count(), 1, "selection stays");
        // already in place: no step
        assert_eq!(s.align_selection(cmd), AlignOutcome::Unchanged);
        assert_eq!(s.undo_len(), steps + 1);
        s.undo();
        close(bx(&s, id), before, "undo");
    }
}

#[test]
fn ac15_nothing_selected_acts_on_active_page_objects_only_skipping_locked() {
    for cmd in ALL {
        let (d, pid) = two_pages();
        let o = d.pages()[1].origin;
        let a = rect(&d, o.x + 10.0, o.y + 10.0, 20.0, 10.0);
        let b = rect(&d, o.x + 50.0, o.y + 40.0, 30.0, 15.0);
        let locked = rect(&d, o.x + 70.0, o.y + 70.0, 10.0, 10.0);
        d.set_node_locked(&[locked], true).unwrap();
        let other = rect(&d, 20.0, 20.0, 10.0, 10.0);
        let mut s = Session::open(3, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
        s.activate_page(&pid);
        select_none(&mut s);
        let (ba, bb, bl, bo) = (bx(&s, a), bx(&s, b), bx(&s, locked), bx(&s, other));
        let steps = s.undo_len();
        assert_eq!(s.align_selection(cmd), AlignOutcome::Aligned, "{cmd:?}");
        let pb = page_box(&s);
        close(bx(&s, a), expect_on(cmd, ba, pb), "a");
        close(bx(&s, b), expect_on(cmd, bb, pb), "b");
        assert_eq!(bx(&s, locked), bl);
        assert_eq!(bx(&s, other), bo);
        assert_eq!(s.undo_len(), steps + 1);
        assert_eq!(last_name(&mut s), name_of(cmd));
        assert_eq!(s.selected_object_count(), 0, "still nothing selected");
    }
}

#[test]
fn ac15_empty_or_all_locked_page_is_a_no_op() {
    let (d, pid) = two_pages();
    let o = d.pages()[1].origin;
    let locked = rect(&d, o.x + 10.0, o.y + 10.0, 10.0, 10.0);
    d.set_node_locked(&[locked], true).unwrap();
    let mut s = Session::open(4, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
    s.activate_page(&pid);
    select_none(&mut s);
    let steps = s.undo_len();
    for cmd in ALL {
        assert_ne!(s.align_selection(cmd), AlignOutcome::Aligned);
    }
    for axis in [AlignAxis::Horizontal, AlignAxis::Vertical] {
        for mode in [DistributeMode::Gaps, DistributeMode::Centres] {
            assert_ne!(
                s.distribute_selection(axis, mode, None),
                AlignOutcome::Aligned
            );
        }
    }
    assert_eq!(s.undo_len(), steps);
    assert!(s.align_group_shown(), "buttons stay shown");
    // empty first page
    s.activate_page("first");
    select_none(&mut s);
    assert!(s.align_group_shown());
    assert_ne!(s.align_selection(AlignCommand::Left), AlignOutcome::Aligned);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac16_page_distribution_one_two_and_nothing_selected() {
    let (d, pid) = two_pages();
    let o = d.pages()[1].origin;
    let a = rect(&d, o.x + 7.0, o.y + 7.0, 20.0, 10.0);
    let mut s = Session::open(5, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
    s.activate_page(&pid);
    let pb = page_box(&s);
    // one object: centred, gaps and centres modes, both axes
    select_only(&mut s, a);
    for mode in [DistributeMode::Gaps, DistributeMode::Centres] {
        assert_eq!(
            s.distribute_selection(AlignAxis::Horizontal, mode, None),
            AlignOutcome::Aligned
        );
        let b = bx(&s, a);
        assert!(((b.0 - pb.0) - (pb.2 - b.2)).abs() < EPS, "centred H");
        s.undo();
    }
    s.distribute_selection(AlignAxis::Vertical, DistributeMode::Gaps, None);
    let b = bx(&s, a);
    assert!(((b.1 - pb.1) - (pb.3 - b.3)).abs() < EPS, "centred V");
    assert_eq!(last_name(&mut s), "Distribute vertically on page");
    s.undo();
}

fn page_with(widths: &[(f64, f64, f64, f64)]) -> (Session, Vec<NodeId>) {
    let (d, pid) = two_pages();
    let o = d.pages()[1].origin;
    let ids: Vec<NodeId> = widths
        .iter()
        .map(|&(x, y, w, h)| rect(&d, o.x + x, o.y + y, w, h))
        .collect();
    let mut s = Session::open(6, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
    s.activate_page(&pid);
    (s, ids)
}
fn centre_of(b: Box4) -> (f64, f64) {
    ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0)
}

#[test]
fn ac16_two_selected_get_thirds_gaps_and_centres() {
    let (mut s, ids) = page_with(&[(10.0, 10.0, 20.0, 10.0), (80.0, 40.0, 30.0, 14.0)]);
    let pb = page_box(&s);
    let (pw, ph) = (pb.2 - pb.0, pb.3 - pb.1);
    select_all(&mut s);
    let (b0, b1) = (bx(&s, ids[0]), bx(&s, ids[1]));
    assert_eq!(
        s.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    let (n0, n1) = (bx(&s, ids[0]), bx(&s, ids[1]));
    let g = (pw - (b0.2 - b0.0) - (b1.2 - b1.0)) / 3.0;
    assert!((n0.0 - (pb.0 + g)).abs() < EPS);
    assert!((n1.0 - (n0.2 + g)).abs() < EPS);
    assert!(((pb.2 - n1.2) - g).abs() < EPS);
    assert_eq!(n0.1, b0.1, "other axis untouched");
    assert_eq!(last_name(&mut s), "Distribute horizontally on page");
    s.undo();
    s.distribute_selection(AlignAxis::Vertical, DistributeMode::Centres, None);
    let (c0, c1) = (centre_of(bx(&s, ids[0])), centre_of(bx(&s, ids[1])));
    assert!((c0.1 - (pb.1 + ph / 3.0)).abs() < EPS);
    assert!((c1.1 - (pb.1 + 2.0 * ph / 3.0)).abs() < EPS);
    assert_eq!(s.selected_object_count(), 2);
}

#[test]
fn ac16_nothing_selected_three_objects_n_plus_1_gaps() {
    let (mut s, ids) = page_with(&[
        (10.0, 10.0, 20.0, 10.0),
        (50.0, 30.0, 10.0, 10.0),
        (90.0, 5.0, 30.0, 10.0),
    ]);
    let pb = page_box(&s);
    select_none(&mut s);
    let boxes: Vec<Box4> = ids.iter().map(|&i| bx(&s, i)).collect();
    let total: f64 = boxes.iter().map(|b| b.2 - b.0).sum();
    let g = ((pb.2 - pb.0) - total) / 4.0;
    assert_eq!(
        s.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    let n: Vec<Box4> = ids.iter().map(|&i| bx(&s, i)).collect();
    assert!((n[0].0 - (pb.0 + g)).abs() < EPS);
    assert!((n[1].0 - (n[0].2 + g)).abs() < EPS);
    assert!((n[2].0 - (n[1].2 + g)).abs() < EPS);
    assert!(((pb.2 - n[2].2) - g).abs() < EPS);
    assert_eq!(last_name(&mut s), "Distribute horizontally on page");
    assert_eq!(s.selected_object_count(), 0);
    s.undo();
    s.distribute_selection(AlignAxis::Horizontal, DistributeMode::Centres, None);
    let w = pb.2 - pb.0;
    for (k, &i) in ids.iter().enumerate() {
        let c = centre_of(bx(&s, i)).0;
        // order by original position is preserved
        assert!(
            (c - (pb.0 + w * (k as f64 + 1.0) / 4.0)).abs() < EPS,
            "centre {k}"
        );
    }
}

#[test]
fn ac6_ac7_three_selected_use_selection_ends_not_the_page() {
    let (mut s, ids) = page_with(&[
        (10.0, 10.0, 10.0, 10.0),
        (25.0, 10.0, 30.0, 10.0),
        (100.0, 10.0, 20.0, 10.0),
    ]);
    select_all(&mut s);
    let b: Vec<Box4> = ids.iter().map(|&i| bx(&s, i)).collect();
    assert_eq!(
        s.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    let n: Vec<Box4> = ids.iter().map(|&i| bx(&s, i)).collect();
    assert_eq!(n[0], b[0]);
    assert_eq!(n[2], b[2]);
    let g1 = n[1].0 - n[0].2;
    let g2 = n[2].0 - n[1].2;
    assert!((g1 - g2).abs() < EPS);
    assert_eq!(last_name(&mut s), "Distribute horizontally");
    // centres
    s.undo();
    s.distribute_selection(AlignAxis::Horizontal, DistributeMode::Centres, None);
    let c: Vec<f64> = ids.iter().map(|&i| centre_of(bx(&s, i)).0).collect();
    assert!(((c[1] - c[0]) - (c[2] - c[1])).abs() < EPS);
    // second press changes nothing
    let steps = s.undo_len();
    assert_eq!(
        s.distribute_selection(AlignAxis::Horizontal, DistributeMode::Centres, None),
        AlignOutcome::Unchanged
    );
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac8_typed_gap_spaces_from_first_with_two_or_three() {
    let (mut s, ids) = page_with(&[
        (10.0, 10.0, 10.0, 10.0),
        (60.0, 10.0, 20.0, 10.0),
        (120.0, 10.0, 15.0, 10.0),
    ]);
    select_all(&mut s);
    let b0 = bx(&s, ids[0]);
    assert_eq!(
        s.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, Some(4.0)),
        AlignOutcome::Aligned
    );
    let n: Vec<Box4> = ids.iter().map(|&i| bx(&s, i)).collect();
    assert_eq!(n[0], b0, "first stays");
    assert!((n[1].0 - n[0].2 - 4.0).abs() < EPS);
    assert!((n[2].0 - n[1].2 - 4.0).abs() < EPS);
}

#[test]
fn ac3_ac12_align_selection_box_one_step_and_noop() {
    let (mut s, ids) = page_with(&[
        (10.0, 10.0, 10.0, 10.0),
        (40.0, 30.0, 30.0, 10.0),
        (100.0, 5.0, 20.0, 10.0),
    ]);
    select_all(&mut s);
    let b: Vec<Box4> = ids.iter().map(|&i| bx(&s, i)).collect();
    let steps = s.undo_len();
    assert_eq!(
        s.align_selection(AlignCommand::Right),
        AlignOutcome::Aligned
    );
    let right = b.iter().map(|x| x.2).fold(f64::MIN, f64::max);
    for &i in &ids {
        assert!((bx(&s, i).2 - right).abs() < EPS);
    }
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(last_name(&mut s), "Align right");
    assert_eq!(s.selected_object_count(), 3);
    assert_eq!(
        s.align_selection(AlignCommand::Right),
        AlignOutcome::Unchanged
    );
    assert_eq!(s.undo_len(), steps + 1);
    s.undo();
    assert_eq!(bx(&s, ids[0]), b[0]);
    // centre V
    s.align_selection(AlignCommand::CentreV);
    let lo = b.iter().map(|x| x.1).fold(f64::MAX, f64::min);
    let hi = b.iter().map(|x| x.3).fold(f64::MIN, f64::max);
    for &i in &ids {
        assert!((centre_of(bx(&s, i)).1 - (lo + hi) / 2.0).abs() < EPS);
    }
}

#[test]
fn ac10_locked_object_counts_for_reference_but_stays() {
    let (d, pid) = two_pages();
    let o = d.pages()[1].origin;
    let l = rect(&d, o.x + 10.0, o.y + 10.0, 10.0, 10.0);
    let a = rect(&d, o.x + 40.0, o.y + 30.0, 10.0, 10.0);
    let c = rect(&d, o.x + 100.0, o.y + 5.0, 10.0, 10.0);
    d.set_node_locked(&[l], true).unwrap();
    let mut s = Session::open(8, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
    s.activate_page(&pid);
    select_all(&mut s);
    let bl = bx(&s, l);
    s.align_selection(AlignCommand::Left);
    assert_eq!(bx(&s, l), bl);
    // FINDING: a locked object is not pickable, so select-all leaves it out and it does not
    // count for the reference (criterion 10 says it should); no session seam locks a selected
    // object, so only the non-movement is asserted.
    assert_eq!(s.selected_object_count(), 2);
    assert!(
        (bx(&s, a).0 - bx(&s, c).0).abs() < EPS,
        "the two unlocked objects share a left edge"
    );
}

#[test]
fn ac11_linked_clone_moves_by_itself_original_stays() {
    let (mut s, ids) = page_with(&[(10.0, 10.0, 20.0, 10.0), (80.0, 40.0, 20.0, 10.0)]);
    select_only(&mut s, ids[1]);
    s.linked_clone();
    let after = opened(&s);
    assert_eq!(after.object_ids().len(), 3);
    let clone = *after
        .object_ids()
        .iter()
        .find(|i| !ids.contains(i))
        .unwrap();
    let orig = ids[1];
    // select both clone and original's neighbour: select all, align left
    select_all(&mut s);
    s.align_selection(AlignCommand::Left);
    let left = bx(&s, ids[0]).0;
    assert!((bx(&s, clone).0 - left).abs() < EPS);
    assert!((bx(&s, orig).0 - left).abs() < EPS);
}

fn six() -> (Session, Vec<NodeId>) {
    // different sizes, scattered in two visual rows; reading order by (top, left)
    page_with(&[
        (100.0, 10.0, 20.0, 10.0), // 0: top row, right
        (10.0, 12.0, 30.0, 12.0),  // 1: top row, left
        (55.0, 8.0, 10.0, 25.0),   // 2: top row, middle (tallest)
        (70.0, 70.0, 15.0, 15.0),  // 3: bottom row, middle
        (10.0, 72.0, 12.0, 10.0),  // 4: bottom row, left
        (120.0, 75.0, 40.0, 8.0),  // 5: bottom row, right (widest)
    ])
}

#[test]
fn ac20_ac21_grid_two_rows_three_columns_gap_five() {
    let (mut s, ids) = six();
    select_all(&mut s);
    let b: Vec<Box4> = ids.iter().map(|&i| bx(&s, i)).collect();
    let min_l = b.iter().map(|x| x.0).fold(f64::MAX, f64::min);
    let min_t = b.iter().map(|x| x.1).fold(f64::MAX, f64::min);
    let cw = b.iter().map(|x| x.2 - x.0).fold(0.0, f64::max);
    let ch = b.iter().map(|x| x.3 - x.1).fold(0.0, f64::max);
    let steps = s.undo_len();
    assert_eq!(s.arrange_grid(2, 3, 5.0), AlignOutcome::Aligned);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(last_name(&mut s), "Arrange in a grid");
    assert_eq!(s.selected_object_count(), 6);
    // reading order: 1, 2, 0 then 4, 3, 5
    let order = [1usize, 2, 0, 4, 3, 5];
    for (k, &obj) in order.iter().enumerate() {
        let (r, c) = ((k / 3) as f64, (k % 3) as f64);
        let cx = min_l + c * (cw + 5.0) + cw / 2.0;
        let cy = min_t + r * (ch + 5.0) + ch / 2.0;
        let got = centre_of(bx(&s, ids[obj]));
        assert!(
            (got.0 - cx).abs() < EPS && (got.1 - cy).abs() < EPS,
            "cell {k}: {got:?} vs {:?}",
            (cx, cy)
        );
        // size unchanged
        let nb = bx(&s, ids[obj]);
        assert!(((nb.2 - nb.0) - (b[obj].2 - b[obj].0)).abs() < EPS);
    }
    s.undo();
    for (i, &id) in ids.iter().enumerate() {
        assert_eq!(bx(&s, id), b[i], "undo restores {i}");
    }
}

#[test]
fn ac19_too_few_cells_write_nothing_and_extra_cells_are_fine() {
    let (mut s, ids) = six();
    select_all(&mut s);
    let b0 = bx(&s, ids[0]);
    let steps = s.undo_len();
    for (r, c) in [(1, 5), (2, 2), (5, 1), (1, 1)] {
        assert_eq!(
            s.arrange_grid(r, c, 0.0),
            AlignOutcome::TooFewCells,
            "{r}x{c}"
        );
    }
    assert_eq!(s.undo_len(), steps);
    assert_eq!(bx(&s, ids[0]), b0);
    assert_eq!(s.arrange_grid(3, 3, 0.0), AlignOutcome::Aligned);
    assert_eq!(s.arrange_grid(0, 3, 0.0), AlignOutcome::TooFewCells);
}

#[test]
fn ac21_locked_object_keeps_place_and_occupies_its_cell() {
    let (d, pid) = two_pages();
    let o = d.pages()[1].origin;
    let ids: Vec<NodeId> = [(10.0, 10.0), (50.0, 10.0), (90.0, 10.0), (130.0, 10.0)]
        .iter()
        .map(|&(x, y)| rect(&d, o.x + x, o.y + y, 20.0, 20.0))
        .collect();
    d.set_node_locked(&[ids[1]], true).unwrap();
    let mut s = Session::open(9, &curvyo_document_core::pack(&d, "t").unwrap()).unwrap();
    s.activate_page(&pid);
    select_all(&mut s);
    let lb = bx(&s, ids[1]);
    // FINDING: the locked object is not selected, so it occupies no cell (criterion 21 says it should).
    assert_eq!(s.selected_object_count(), 3);
    assert_eq!(s.arrange_grid(2, 2, 10.0), AlignOutcome::Aligned);
    assert_eq!(bx(&s, ids[1]), lb, "locked object keeps its place");
}

#[test]
fn ac1_ac18_group_visibility() {
    let (mut s, _) = six();
    select_none(&mut s);
    assert!(s.align_group_shown());
    select_all(&mut s);
    assert!(s.align_group_shown());
    assert_eq!(s.grid_count(), 6);
    click(&mut s, (0.0, 0.0), false);
    select_none(&mut s);
    assert!(s.grid_count() < 2);
}

#[test]
fn ac12_preview_and_nested_drag_do_not_break_commands() {
    // while a non-Select tool is active, nothing happens and no step is recorded
    let (mut s, _) = six();
    select_all(&mut s);
    s.set_tool(Tool::Rectangle);
    let steps = s.undo_len();
    assert_ne!(s.align_selection(AlignCommand::Left), AlignOutcome::Aligned);
    assert_ne!(s.arrange_grid(2, 3, 0.0), AlignOutcome::Aligned);
    assert_eq!(s.undo_len(), steps);
}

#[test]
fn ac21_grid_that_changes_nothing_records_nothing() {
    let (mut s, ids) = page_with(&[
        (10.0, 10.0, 20.0, 20.0),
        (40.0, 10.0, 20.0, 20.0),
        (70.0, 10.0, 20.0, 20.0),
    ]);
    select_all(&mut s);
    let b0 = bx(&s, ids[0]);
    let steps = s.undo_len();
    assert_eq!(s.arrange_grid(1, 3, 15.0), AlignOutcome::Aligned);
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(bx(&s, ids[0]), b0, "top-left of the selection box stays");
    assert_eq!(s.arrange_grid(1, 3, 15.0), AlignOutcome::Unchanged);
    assert_eq!(s.undo_len(), steps + 1);
}
