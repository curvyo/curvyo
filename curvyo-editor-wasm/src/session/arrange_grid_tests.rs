//! Session-level tests of the Arrange in a grid group of
//! `specs/0061-align-and-distribute/`: reading order, cells, refusal,
//! history steps and undo.

use curvyo_ui_core::{AlignAxis, AlignBox, AlignOutcome};

use super::Session;
use super::align_test_support::{
    box_of, click, draw_rect, last_label, near, select_all, select_nothing, steps,
};

/// Criteria 20, 21: six rectangles of different sizes, 2 rows, 3 columns,
/// gap 5: reading order, centred in cells of the largest box, the block at
/// the selection box's top-left; one step "Arrange in a grid", undone as
/// one; a second press changes nothing.
#[test]
fn c20_six_objects_arrange_in_two_rows_of_three() {
    let mut session = Session::new(1);
    let ids = [
        draw_rect(&mut session, (100.0, 10.0), (110.0, 20.0)),
        draw_rect(&mut session, (10.0, 50.0), (30.0, 60.0)),
        draw_rect(&mut session, (60.0, 10.0), (80.0, 30.0)),
        draw_rect(&mut session, (100.0, 50.0), (106.0, 54.0)),
        draw_rect(&mut session, (10.0, 12.0), (18.0, 42.0)),
        draw_rect(&mut session, (50.0, 50.0), (62.0, 58.0)),
    ];
    select_all(&mut session);
    assert_eq!(session.grid_count(), 6);
    let key = session.grid_selection_key();
    let before: Vec<AlignBox> = ids.iter().map(|&id| box_of(&session, id)).collect();
    assert_eq!(session.arrange_grid(2, 3, 5.0), AlignOutcome::Aligned);
    assert_eq!(last_label(&session), "arrange_grid");
    // Selection box top-left (10, 10); cells 20 x 30.
    let expected = [
        (4, (20.0, 25.0)),
        (2, (45.0, 25.0)),
        (0, (70.0, 25.0)),
        (1, (20.0, 60.0)),
        (5, (45.0, 60.0)),
        (3, (70.0, 60.0)),
    ];
    for (index, (x, y)) in expected {
        let box_ = box_of(&session, ids[index]);
        near(box_.centre(AlignAxis::Horizontal), x, "x");
        near(box_.centre(AlignAxis::Vertical), y, "y");
    }
    assert_eq!(session.selection.ids().len(), 6, "the selection stays");
    assert_eq!(session.grid_selection_key(), key);
    let count = steps(&session);
    assert_eq!(session.arrange_grid(2, 3, 5.0), AlignOutcome::Unchanged);
    assert_eq!(steps(&session), count);
    assert!(session.undo().changed);
    for (index, &id) in ids.iter().enumerate() {
        assert_eq!(
            box_of(&session, id),
            before[index],
            "undo puts {index} back"
        );
    }
}

/// Criterion 19: too few cells write nothing; one object is no grid.
#[test]
fn c19_too_few_cells_or_one_object_write_nothing() {
    let mut session = Session::new(1);
    draw_rect(&mut session, (0.0, 0.0), (10.0, 10.0));
    draw_rect(&mut session, (20.0, 0.0), (30.0, 10.0));
    draw_rect(&mut session, (40.0, 0.0), (50.0, 10.0));
    select_all(&mut session);
    let count = steps(&session);
    assert_eq!(session.arrange_grid(1, 2, 0.0), AlignOutcome::TooFewCells);
    select_nothing(&mut session);
    click(&mut session, (5.0, 0.0), false);
    assert_eq!(session.grid_count(), 1);
    assert_eq!(
        session.arrange_grid(1, 1, 0.0),
        AlignOutcome::NothingSelected
    );
    assert_eq!(steps(&session), count);
}
