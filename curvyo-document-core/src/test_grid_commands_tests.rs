use serde_json::{Map, Value, json};

use super::*;
use crate::{
    Affine, CopySource, DocumentSize, FontKind, FontRef, Length, MISSING_FACE_HASH, NewOutputPage,
    OperationError, Point, RectBounds, TextChars, TextLayoutValues, Vec2, pack, unpack,
};

fn laser(d: &Document) -> NodeId {
    let page = d
        .add_output_page(NewOutputPage {
            process: "curvyo.laser",
            label: "Laser",
            profile: None,
            work_area: DocumentSize::from_mm(400.0, 300.0),
        })
        .unwrap();
    page.kind.manufacturing().unwrap()
}

fn map(value: Value) -> Map<String, Value> {
    let Value::Object(map) = value else {
        panic!("a map")
    };
    map
}

fn cell(column: u32, x: f64) -> NewGridCell {
    NewGridCell {
        shape: Shape::Rect {
            bounds: RectBounds::from_corners(Point::new(x, 0.0), Point::new(x + 10.0, 10.0)),
            corner_radii: crate::CornerRadii::uniform(Length::from_mm(0.0)),
        },
        style: Style::default(),
        tag: map(json!({"column": column, "row": 0, "power_pct": 20.0, "speed_mm_min": 600})),
    }
}

fn label(text: &str, tag: &str) -> NewGridLabel {
    NewGridLabel {
        text: DrawingText {
            string: text.to_owned(),
            font: FontRef {
                family: "Sans".into(),
                style: "Regular".into(),
                face_hash: MISSING_FACE_HASH.to_owned(),
                face_index: 0,
                kind: FontKind::Outline,
            },
            size: Length::from_mm(3.0),
            layout: TextLayoutValues::DEFAULT,
            chars: TextChars::new([]),
            placement: Affine::translation(Vec2::new(0.0, 3.0)),
            style: Style::default(),
        },
        tag: tag.to_owned(),
    }
}

fn items(columns: u32, labels: bool) -> GridItems {
    GridItems {
        cells: (0..columns).map(|c| cell(c, 13.0 * f64::from(c))).collect(),
        labels: if labels {
            vec![label("% power / mm/min", "unit"), label("20", "power")]
        } else {
            Vec::new()
        },
    }
}

fn recipe() -> Map<String, Value> {
    map(json!({"kind": "cut", "columns": 2, "rows": 2}))
}

fn add(d: &Document, m: NodeId) -> MadeTestGrid {
    d.add_test_grid(&NewTestGrid {
        manufacturing: m,
        name: "Test grid 1",
        kind: "cut",
        recipe: &recipe(),
        items: items(2, true),
        row_index: 9,
    })
    .unwrap()
}

#[test]
fn add_test_grid_makes_the_tagged_group_and_its_row_in_one_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let steps = d.own_step_count();
    let made = add(&d, m);
    assert_eq!(d.own_step_count(), steps + 1);
    let grid = d.test_grid(made.group).unwrap();
    assert_eq!(grid.recipe, Value::Object(recipe()));
    assert_eq!(grid.worked, None);
    assert_eq!(grid.cells.len(), 2);
    assert_eq!(grid.labels.len(), 2);
    assert_eq!(grid.labels[0].tag, json!("unit"));
    assert_eq!(grid.rows, [made.row]);
    let row = d.operations(m).unwrap().remove(0);
    assert!(row.grid);
    assert_eq!(
        (row.name.as_str(), row.kind.as_deref()),
        ("Test grid 1", Some("cut"))
    );
    assert_eq!(row.members, [vec![made.group]]);
    assert_eq!(row.settings, None, "a grid row has no settings");
    assert_eq!(d.test_grid_of_row(made.row), Some(made.group));
    let (cell_tag, label_tag) = d.grid_item_tags(grid.cells[0].id);
    assert_eq!(
        (cell_tag.unwrap()["column"].clone(), label_tag),
        (json!(0), None)
    );
    let back = unpack(2, &pack(&d, "grid").unwrap()).unwrap();
    assert_eq!(back.test_grid(made.group), Some(grid));
}

#[test]
fn a_rebuild_replaces_the_tagged_items_keeps_untagged_ones_and_sets_the_mark() {
    let d = Document::new(1);
    let m = laser(&d);
    let made = add(&d, m);
    let mark = map(json!({"column": 1, "row": 0}));
    assert!(d.set_test_grid_mark(made.group, Some(&mark)).unwrap());
    assert!(
        !d.set_test_grid_mark(made.group, Some(&mark)).unwrap(),
        "already so"
    );
    assert_eq!(
        d.test_grid(made.group).unwrap().worked,
        Some(Value::Object(mark.clone()))
    );
    let wider = map(json!({"kind": "cut", "columns": 3, "rows": 2}));
    let steps = d.own_step_count();
    d.edit_test_grid(&TestGridEdit {
        group: made.group,
        recipe: &wider,
        worked: None,
        items: Some(&items(3, false)),
        row: None,
    })
    .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    let grid = d.test_grid(made.group).unwrap();
    assert_eq!(grid.recipe, Value::Object(wider));
    assert_eq!(
        (grid.cells.len(), grid.labels.len()),
        (3, 0),
        "Labels off: no text"
    );
    assert_eq!(grid.worked, None, "the mark cleared");
    assert_eq!(d.operations(m).unwrap()[0].members, [vec![made.group]]);
    assert!(d.set_test_grid_mark(made.group, None).is_ok());
    undo(&d);
    assert_eq!(
        d.test_grid(made.group).unwrap().cells.len(),
        2,
        "one undo step"
    );
}

#[test]
fn deleting_the_top_group_deletes_its_row_and_deleting_the_row_keeps_the_items() {
    let d = Document::new(1);
    let m = laser(&d);
    let made = add(&d, m);
    d.delete_operation(made.row).unwrap();
    assert_eq!(d.operations(m).unwrap(), []);
    assert_eq!(
        d.test_grid(made.group).unwrap().cells.len(),
        2,
        "items stay"
    );
    undo(&d);
    assert_eq!(d.operations(m).unwrap().len(), 1);
    d.delete_objects(&[made.group]).unwrap();
    assert!(
        d.operations(m).unwrap().is_empty(),
        "the row goes with its group"
    );
    undo(&d);
    assert_eq!(
        d.operations(m).unwrap()[0].id,
        made.row,
        "and comes back with it"
    );
    assert_eq!(
        d.rename_operation(made.row, "Birch test")
            .map(|_| last_label(&d)),
        Ok("rename_test_grid".to_owned())
    );
    assert_eq!(
        d.set_operation_members(made.row, &[vec![made.group]], false),
        Err(OperationError::GridRow)
    );
}

fn undo(d: &Document) {
    let last = d.own_steps().last().unwrap().step;
    d.undo_step(last).unwrap();
}

fn last_label(d: &Document) -> String {
    let frontiers = d.loro().oplog_frontiers();
    let id = frontiers.iter().next().unwrap();
    let message = d.loro().get_change(id).unwrap().message.unwrap();
    message.split(';').next().unwrap().to_owned()
}

#[test]
fn a_copy_of_a_grid_gets_its_own_row_and_no_mark() {
    let d = Document::new(1);
    let m = laser(&d);
    let made = add(&d, m);
    let mark = map(json!({"column": 0, "row": 0}));
    d.set_test_grid_mark(made.group, Some(&mark)).unwrap();
    let copies = d
        .duplicate_objects(
            &[CopySource {
                id: made.group,
                anchor_ids: Vec::new(),
            }],
            Vec2::new(0.0, 80.0),
        )
        .unwrap();
    let copy = copies[0];
    let grid = d.test_grid(copy).unwrap();
    assert_eq!(grid.worked, None, "the copy has no Worked mark");
    assert_eq!(grid.cells.len(), 2);
    let rows = d.operations(m).unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].members,
        [vec![made.group]],
        "the original's row is unchanged"
    );
    assert_eq!(rows[1].members, [vec![copy]]);
    assert!(rows[1].grid);
    assert_eq!(d.test_grid_of_row(rows[1].id), Some(copy));
    assert_eq!(
        d.test_grid(made.group).unwrap().worked,
        Some(Value::Object(mark)),
        "the original keeps its mark"
    );
}

#[test]
fn a_grid_command_on_no_grid_is_refused() {
    let d = Document::new(1);
    let m = laser(&d);
    let rect = d.create_rect(RectBounds::from_corners(
        Point::new(0.0, 0.0),
        Point::new(5.0, 5.0),
    ));
    assert_eq!(d.test_grid(rect), None);
    assert_eq!(
        d.set_test_grid_mark(rect, None),
        Err(TestGridError::NoSuchGrid)
    );
    let mut bad = items(2, true);
    bad.labels[0].text.string.clear();
    assert_eq!(
        d.add_test_grid(&NewTestGrid {
            manufacturing: m,
            name: "Test grid 1",
            kind: "cut",
            recipe: &recipe(),
            items: bad,
            row_index: 0,
        }),
        Err(TestGridError::BadLabel)
    );
    assert!(d.operations(m).unwrap().is_empty(), "nothing written");
}

#[test]
fn an_edit_of_a_hand_edited_grid_writes_the_recipe_and_keeps_the_items() {
    let d = Document::new(1);
    let m = laser(&d);
    let made = add(&d, m);
    let before = d.test_grid(made.group).unwrap();
    let steps = d.own_step_count();
    let next = map(json!({"kind": "cut", "columns": 3, "rows": 2}));
    d.edit_test_grid(&TestGridEdit {
        group: made.group,
        recipe: &next,
        worked: None,
        items: None,
        row: None,
    })
    .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), EDIT_TEST_GRID_COMMIT_LABEL);
    let after = d.test_grid(made.group).unwrap();
    assert_eq!(after.recipe, Value::Object(next));
    assert_eq!(after.cells, before.cells, "the items stay until Rebuild");
}

#[test]
fn a_kind_edit_rebuilds_retypes_the_row_and_moves_it_in_one_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let made = add(&d, m);
    let other = d
        .add_operation(
            m,
            &crate::NewOperation {
                kind: "engrave",
                name: "Engrave 1",
            },
            0,
            &[],
        )
        .unwrap();
    assert_eq!(d.operations(m).unwrap()[1].id, made.row);
    let next = map(json!({"kind": "score", "columns": 3, "rows": 2}));
    let steps = d.own_step_count();
    d.edit_test_grid(&TestGridEdit {
        group: made.group,
        recipe: &next,
        worked: None,
        items: Some(&items(3, false)),
        row: Some(RowMove {
            kind: "score",
            index: 0,
        }),
    })
    .unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    let rows = d.operations(m).unwrap();
    assert_eq!((rows[0].id, rows[1].id), (made.row, other));
    assert_eq!(rows[0].kind.as_deref(), Some("score"));
    let grid = d.test_grid(made.group).unwrap();
    assert_eq!((grid.cells.len(), grid.labels.len()), (3, 0));
}
