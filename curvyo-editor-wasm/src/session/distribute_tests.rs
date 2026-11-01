//! Session-level tests of the Distribute group of
//! `specs/0061-align-and-distribute/`: relative to the selection's ends or
//! the page, the typed gap, history steps and undo.

use curvyo_document_core::NodeId;
use curvyo_ui_core::{AlignAxis, AlignOutcome, DistributeMode, object_outline_bounds};

use super::Session;
use super::align_test_support::{
    box_of, click, draw_rect, last_label, left_x, near, second_page, select_all, select_nothing,
};

/// Criterion 16: one selected object is centred across the page, two sit in
/// thirds, nothing selected spreads the page's three with four equal gaps;
/// the steps are named "on page".
#[test]
fn c16_distribute_relative_to_the_page_with_none_one_or_two_selected() {
    let mut session = Session::new(1);
    let (_, page) = second_page(&mut session);
    let (x0, w) = (page.min.x, page.extent(AlignAxis::Horizontal));
    let a = draw_rect(&mut session, (260.0, 60.0), (280.0, 80.0));
    let b = draw_rect(&mut session, (300.0, 100.0), (330.0, 120.0));
    let c = draw_rect(&mut session, (350.0, 140.0), (360.0, 150.0));
    // One object: centred across the page, both modes.
    click(&mut session, (270.0, 60.0), false);
    assert_eq!(session.selection.ids(), &[a]);
    assert_eq!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    assert_eq!(last_label(&session), "distribute_h_page");
    near(
        box_of(&session, a).centre(AlignAxis::Horizontal),
        page.centre(AlignAxis::Horizontal),
        "one",
    );
    assert_eq!(
        session.distribute_selection(AlignAxis::Vertical, DistributeMode::Centres, None),
        AlignOutcome::Aligned
    );
    assert_eq!(last_label(&session), "distribute_v_page");
    near(
        box_of(&session, a).centre(AlignAxis::Vertical),
        page.centre(AlignAxis::Vertical),
        "one v",
    );
    // Two objects: thirds (equal gaps), and centres on the thirds.
    session.selection.clear();
    click(&mut session, (310.0, 100.0), false);
    click(&mut session, (355.0, 140.0), true);
    assert_eq!(session.selection.ids().len(), 2);
    assert_eq!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    let gap = (w - 40.0) / 3.0;
    near(box_of(&session, b).min.x, x0 + gap, "two: first");
    near(
        box_of(&session, c).min.x,
        x0 + gap + 30.0 + gap,
        "two: second",
    );
    assert_eq!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Centres, None),
        AlignOutcome::Aligned
    );
    near(
        box_of(&session, b).centre(AlignAxis::Horizontal),
        x0 + w / 3.0,
        "two: c1",
    );
    near(
        box_of(&session, c).centre(AlignAxis::Horizontal),
        x0 + 2.0 * w / 3.0,
        "two: c2",
    );
    // Nothing selected, three objects: four equal gaps.
    select_nothing(&mut session);
    assert_eq!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, None),
        AlignOutcome::Aligned
    );
    let gap = (w - 60.0) / 4.0;
    // The order is the positions before: b, then a (centred), then c.
    near(box_of(&session, b).min.x, x0 + gap, "page: b");
    near(box_of(&session, a).min.x, x0 + 2.0 * gap + 30.0, "page: a");
    near(box_of(&session, c).min.x, x0 + 3.0 * gap + 50.0, "page: c");
    assert_eq!(session.selection.ids(), &[] as &[NodeId]);
    // One undo puts all three back where the two-object step left them.
    assert!(session.undo().changed);
    near(
        box_of(&session, b).centre(AlignAxis::Horizontal),
        x0 + w / 3.0,
        "undo",
    );
}

/// Criterion 16: three or more selected keep the selection's ends.
#[test]
fn c16_three_selected_keep_the_ends() {
    let mut session = Session::new(1);
    draw_rect(&mut session, (0.0, 0.0), (10.0, 10.0));
    draw_rect(&mut session, (16.0, 0.0), (20.0, 10.0));
    draw_rect(&mut session, (40.0, 0.0), (50.0, 10.0));
    select_all(&mut session);
    assert_eq!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Centres, None),
        AlignOutcome::Aligned
    );
    assert_eq!(last_label(&session), "distribute_h");
    let object = session.objects()[1].clone();
    let (min, max) = object_outline_bounds(&object);
    assert_eq!(f64::midpoint(min.x, max.x), 25.0);
    assert_eq!(left_x(&session, 0), 0.0);
    assert_eq!(left_x(&session, 2), 40.0);
    assert!(session.undo().changed);
    assert_eq!(left_x(&session, 1), 16.0);
}

#[test]
fn a_typed_gap_distributes_three_objects_from_the_first() {
    let mut session = Session::new(1);
    draw_rect(&mut session, (0.0, 0.0), (30.0, 10.0));
    draw_rect(&mut session, (50.0, 0.0), (70.0, 10.0));
    draw_rect(&mut session, (90.0, 0.0), (100.0, 10.0));
    select_all(&mut session);
    assert_eq!(
        session.distribute_selection(AlignAxis::Horizontal, DistributeMode::Gaps, Some(5.0)),
        AlignOutcome::Aligned
    );
    // 0 + 30 + 5, then 35 + 20 + 5.
    assert_eq!(left_x(&session, 1), 35.0);
    assert_eq!(left_x(&session, 2), 60.0);
    assert!(session.undo().changed);
    assert_eq!(left_x(&session, 1), 50.0);
}
