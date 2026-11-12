use super::*;

fn recipe() -> GridRecipe {
    GridRecipe::defaults(FeedRate::new(6000).unwrap())
}

fn lanes() -> LabelLanes {
    LabelLanes {
        speed_width: Length::from_mm(8.0),
        unit_width: Length::from_mm(30.0),
    }
}

#[test]
fn a_five_by_five_grid_with_labels_lays_out_cells_and_lanes() {
    let grid = layout(&recipe(), lanes());
    assert_eq!(grid.cells.len(), 25);
    let first = grid.cells[0];
    assert_eq!(first.cell, CellIndex { column: 0, row: 0 });
    assert_eq!(
        (first.min.x, first.min.y),
        (9.0, 8.0),
        "lanes: 8 + 1 mm, 2 x 3 + 2 mm"
    );
    let last = grid.cells[24];
    assert_eq!(
        (last.min.x, last.min.y),
        (9.0 + 4.0 * 13.0, 8.0 + 4.0 * 13.0)
    );
    assert_eq!(grid.width.as_mm(), 9.0 + 5.0 * 13.0 - 3.0);
    assert_eq!(grid.height.as_mm(), 8.0 + 5.0 * 13.0 - 3.0);
    let tags: Vec<LabelTag> = grid.labels.iter().map(|l| l.tag).collect();
    assert_eq!(tags.len(), 11, "unit, five powers, five speeds");
    assert_eq!(grid.labels[0].text, UNIT_TEXT);
    assert_eq!(grid.labels[1].text, "20");
    assert_eq!(grid.labels[6].text, "600");
    assert_eq!(speed_texts(&recipe())[4], "6000");
}

#[test]
fn without_labels_the_cells_start_at_the_corner_and_no_text_is_made() {
    let mut no_labels = recipe();
    no_labels.labels.on = false;
    let grid = layout(&no_labels, lanes());
    assert_eq!(grid.labels, Vec::<LabelPlace>::new());
    assert_eq!((grid.cells[0].min.x, grid.cells[0].min.y), (0.0, 0.0));
    assert_eq!(grid.width.as_mm(), 62.0);
}

#[test]
fn a_wide_unit_line_widens_the_grid() {
    let mut small = recipe();
    small.range.columns = 2;
    small.range.cell = Length::from_mm(3.0);
    let grid = layout(&small, lanes());
    assert_eq!(grid.width.as_mm(), 30.0);
}

#[test]
fn the_bed_fit_names_the_side_that_does_not_fit() {
    let bed = DocumentSize::from_mm(200.0, 100.0);
    assert!(fit_bed(Length::from_mm(200.0), Length::from_mm(100.0), bed).is_ok());
    assert_eq!(
        fit_bed(Length::from_mm(260.0), Length::from_mm(50.0), bed).unwrap_err(),
        "Grid is 260 mm wide, the bed is 200 mm."
    );
    assert_eq!(
        fit_bed(Length::from_mm(50.0), Length::from_mm(100.04), bed).unwrap_err(),
        "Grid is 100.1 mm high, the bed is 100 mm."
    );
    assert_eq!(
        no_free_spot_text(Length::from_mm(75.0), Length::from_mm(62.0)),
        "No free spot on the bed for a 75 × 62 mm grid."
    );
}

#[test]
fn the_free_spot_is_the_corner_else_the_first_left_to_right_top_to_bottom() {
    let bed = DocumentSize::from_mm(200.0, 100.0);
    let (w, h) = (Length::from_mm(50.0), Length::from_mm(40.0));
    assert_eq!(free_spot(w, h, bed, &[]), Some(Point::new(0.0, 0.0)));
    let left = (Point::new(0.0, 0.0), Point::new(60.0, 30.0));
    assert_eq!(free_spot(w, h, bed, &[left]), Some(Point::new(60.0, 0.0)));
    let wide = (Point::new(0.0, 0.0), Point::new(180.0, 30.0));
    assert_eq!(free_spot(w, h, bed, &[wide]), Some(Point::new(0.0, 30.0)));
    let full = (Point::new(0.0, 0.0), Point::new(200.0, 70.0));
    assert_eq!(free_spot(w, h, bed, &[full]), None);
}

#[test]
fn a_rebuild_keeps_the_mark_on_the_same_numbers_else_clears_it() {
    let old = recipe();
    let mark = CellIndex { column: 2, row: 1 }; // 60 %, 1950 mm/min
    let mut more = old;
    more.range.columns = 9; // 20, 30, ..., 100: 60 % is column 4
    assert_eq!(
        kept_mark(&old, mark, &more),
        Some(CellIndex { column: 4, row: 1 })
    );
    let mut other = old;
    other.range.power_from = PowerPercent::from_tenths(250).unwrap();
    assert_eq!(kept_mark(&old, mark, &other), None);
}

fn built(grid: &TestGridLayout, shift: (f64, f64)) -> (Vec<BuiltCell>, Vec<BuiltLabel>) {
    let cells = grid
        .cells
        .iter()
        .map(|c| BuiltCell {
            cell: c.cell,
            power: c.power,
            speed: c.speed,
            min: Point::new(c.min.x + shift.0, c.min.y + shift.1),
            max: Point::new(
                c.min.x + shift.0 + c.side.as_mm(),
                c.min.y + shift.1 + c.side.as_mm(),
            ),
            square: true,
            filled: false,
        })
        .collect();
    let labels = grid
        .labels
        .iter()
        .map(|l| BuiltLabel {
            tag: l.tag,
            text: l.text.clone(),
            origin: Some(Point::new(l.origin.x + shift.0, l.origin.y + shift.1)),
        })
        .collect();
    (cells, labels)
}

#[test]
fn a_moved_grid_is_not_hand_edited_but_a_moved_cell_or_retyped_label_is() {
    let grid = layout(&recipe(), lanes());
    let (cells, labels) = built(&grid, (40.0, 25.0));
    assert!(!hand_edited(&grid, SetKind::Cut, &cells, &labels));
    assert!(
        hand_edited(&grid, SetKind::Engrave, &cells, &labels),
        "restyled"
    );
    let mut moved = cells.clone();
    moved[3].min.x += 0.5;
    moved[3].max.x += 0.5;
    assert!(hand_edited(&grid, SetKind::Cut, &moved, &labels));
    let mut nudged = cells.clone();
    nudged[3].min.x += GRID_TOLERANCE_MM / 2.0;
    assert!(
        !hand_edited(&grid, SetKind::Cut, &nudged, &labels),
        "within tolerance"
    );
    assert!(
        hand_edited(&grid, SetKind::Cut, &cells[1..], &labels),
        "a cell deleted"
    );
    let mut retyped = labels.clone();
    retyped[2].text = "41".to_owned();
    assert!(hand_edited(&grid, SetKind::Cut, &cells, &retyped));
    assert!(
        hand_edited(&grid, SetKind::Cut, &cells, &labels[1..]),
        "a label deleted"
    );
    let mut first_gone = cells.clone();
    first_gone.remove(0);
    assert!(hand_edited(&grid, SetKind::Cut, &first_gone, &labels));
}
