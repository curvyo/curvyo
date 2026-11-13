use serde_json::{Map, Value, json};

use super::*;
use crate::operation_codec::KEY_OP_GRID_PART;
use crate::test_grid_codec::encode_grid;
use crate::test_grid_commands::{NewGridCell, NewGridLabel};
use crate::{
    Affine, CopySource, DocumentSize, DrawingText, FontKind, FontRef, Length, MISSING_FACE_HASH,
    NewOperation, NewOutputPage, Point, RectBounds, SettingsStep, SettingsWrite, Shape, Style,
    TextChars, TextLayoutValues, Vec2, pack, unpack,
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

fn cell(column: u8, row: u8) -> NewGridCell {
    let (x, y) = (12.0 * f64::from(column), 12.0 * f64::from(row));
    NewGridCell {
        shape: Shape::Rect {
            bounds: RectBounds::from_corners(Point::new(x, y), Point::new(x + 10.0, y + 10.0)),
            corner_radii: crate::CornerRadii::uniform(Length::from_mm(0.0)),
        },
        style: Style::default(),
        tag: map(json!({"column": column, "row": row})),
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
            placement: Affine::translation(Vec2::new(0.0, -3.0)),
            style: Style::default(),
        },
        tag: tag.to_owned(),
    }
}

/// A grid of `columns` x 2 cells with its id label and one value label.
fn items(id: &str, columns: u8) -> GridItems {
    GridItems {
        cells: (0..2)
            .flat_map(|row| (0..columns).map(move |column| cell(column, row)))
            .collect(),
        labels: vec![label(id, "id"), label("20", "column_value")],
    }
}

fn recipe(id: &str, columns: u8) -> Map<String, Value> {
    map(json!({
        "id": id, "kind": "cut",
        "columns": {"param": "power", "min": 20.0, "max": 100.0, "count": columns},
        "rows": {"param": "speed", "min": 600, "max": 6000, "count": 2},
        "fixed": {"passes": 1}, "cell_mm": 10.0, "crosshatch": false,
    }))
}

fn numbers(power: f64) -> Map<String, Value> {
    map(json!({"main": {"power_pct": power, "speed_mm_min": 600, "passes": 1}}))
}

/// The resolved rows of [`items`]: the labels, then a cell row each, as owned values.
struct Rows {
    names: Vec<String>,
    settings: Vec<Map<String, Value>>,
    parts: Vec<DerivedPart>,
}

fn rows(id: &str, columns: u8) -> Rows {
    let mut out = Rows {
        names: vec![format!("{id} labels")],
        settings: vec![numbers(20.0)],
        parts: vec![DerivedPart::Labels],
    };
    for row in 0..2 {
        for column in 0..columns {
            out.names.push(format!("{column} · {row} · {id}"));
            out.settings.push(numbers(20.0 + f64::from(column)));
            out.parts.push(DerivedPart::Cell { column, row });
        }
    }
    out
}

fn derived(rows: &Rows, at: usize) -> Vec<DerivedRow<'_>> {
    (0..rows.names.len())
        .map(|n| DerivedRow {
            part: rows.parts[n],
            kind: if n == 0 { "engrave" } else { "cut" },
            name: &rows.names[n],
            settings: &rows.settings[n],
            options: None,
            at,
        })
        .collect()
}

fn place(d: &Document, m: NodeId, id: &str, columns: u8, at: usize) -> NodeId {
    let (recipe, items, rows) = (recipe(id, columns), items(id, columns), rows(id, columns));
    let derived = derived(&rows, at);
    d.place_test_grid(&NewGridObject {
        name: "Test grid",
        write: TestGridWrite {
            manufacturing: m,
            recipe: &recipe,
            items: &items,
            rows: &derived,
        },
        lasered: None,
        step: GridStep::Add,
    })
    .unwrap()
}

fn regenerate(
    d: &Document,
    m: NodeId,
    grid: NodeId,
    id: &str,
    columns: u8,
) -> Result<(), TestGridError> {
    let (recipe, items, rows) = (recipe(id, columns), items(id, columns), rows(id, columns));
    let derived = derived(&rows, 99);
    d.regenerate_test_grid(
        grid,
        &TestGridWrite {
            manufacturing: m,
            recipe: &recipe,
            items: &items,
            rows: &derived,
        },
        GridStep::Edit,
    )
}

fn ops(d: &Document, m: NodeId) -> Vec<crate::OperationSnapshot> {
    d.operations(m).unwrap()
}

fn undo(d: &Document) {
    let last = d.own_steps().last().unwrap().step;
    d.undo_step(last).unwrap();
}

fn last_label(d: &Document) -> String {
    d.own_steps().last().unwrap().label.clone()
}

fn add_cut(d: &Document, m: NodeId, index: usize) -> NodeId {
    d.add_operation(
        m,
        &NewOperation {
            kind: "cut",
            name: "Cut 1",
        },
        index,
        &[],
    )
    .unwrap()
}

#[test]
fn placing_writes_the_group_its_items_and_one_derived_row_per_part_in_one_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let cut = add_cut(&d, m, 0);
    let steps = d.own_step_count();
    let grid = place(&d, m, "K7Q2M", 2, 0);
    assert_eq!(d.own_step_count(), steps + 1);
    assert_eq!(last_label(&d), "add_test_grid");
    let snapshot = d.test_grid(grid).unwrap();
    assert_eq!(snapshot.recipe, Value::Object(recipe("K7Q2M", 2)));
    assert_eq!(
        (
            snapshot.lasered,
            snapshot.cells.len(),
            snapshot.labels.len()
        ),
        (None, 4, 2)
    );
    assert!(snapshot.rows.is_empty(), "no folded row");
    let list = ops(&d, m);
    assert_eq!(list.len(), 6);
    assert_eq!(
        list[5].id, cut,
        "the rows went before the operation at index 0"
    );
    assert_eq!(
        snapshot.derived,
        list[..5].iter().map(|op| op.id).collect::<Vec<_>>()
    );
    assert_eq!(list[0].name, "K7Q2M labels");
    assert_eq!(list[0].kind.as_deref(), Some("engrave"));
    assert_eq!(list[0].members.len(), 2, "one key per label item");
    assert_eq!(
        list[0].grid_part,
        Some(json!({"grid": encode_grid(tree_id_of(grid)), "part": "labels"}))
    );
    assert_eq!(list[2].name, "1 · 0 · K7Q2M");
    assert_eq!(list[2].settings, Some(Value::Object(numbers(21.0))));
    assert_eq!(list[2].members.len(), 1);
    let (tag, _) = d.grid_item_tags(list[2].members[0][0]);
    assert_eq!(tag, Some(json!({"column": 1, "row": 0})));
    assert!(list.iter().all(|op| op.show && op.output && !op.grid));
    assert_eq!(d.test_grid_of_derived_row(list[3].id), Some(grid));
    assert_eq!(d.test_grid_of_derived_row(cut), None);
    let back = unpack(2, &pack(&d, "grid").unwrap()).unwrap();
    assert_eq!(back.test_grid(grid), Some(snapshot));
}

#[test]
fn a_new_grid_without_operations_appends_its_rows() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    assert_eq!(ops(&d, m).len(), 5);
    assert_eq!(d.test_grid(grid).unwrap().derived.len(), 5);
}

#[test]
fn regeneration_overwrites_hand_edits_in_place_and_one_undo_restores_them() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    let row = ops(&d, m)[1].id;
    // A hand edit: settings, Show, name.
    let write = [SettingsWrite {
        operation: row,
        value: Some(numbers(77.0)),
    }];
    assert!(
        d.write_operation_settings(SettingsStep::EditSettings, &write)
            .unwrap()
    );
    d.rename_operation(row, "Mine").unwrap();
    d.set_operation_switch(row, crate::OperationSwitch::Show, false)
        .unwrap();
    let edited = ops(&d, m);
    let steps = d.own_step_count();
    regenerate(&d, m, grid, "K7Q2M", 2).unwrap();
    assert_eq!(d.own_step_count(), steps + 1);
    let after = ops(&d, m);
    assert_eq!(
        after.iter().map(|op| op.id).collect::<Vec<_>>(),
        edited.iter().map(|op| op.id).collect::<Vec<_>>(),
        "node ids and places kept"
    );
    assert_eq!(after[1].name, "0 · 0 · K7Q2M");
    assert!(after[1].show);
    assert_eq!(after[1].settings, Some(Value::Object(numbers(20.0))));
    assert_ne!(
        after[1].members, edited[1].members,
        "the items were laid out anew"
    );
    undo(&d);
    assert_eq!(
        ops(&d, m),
        edited,
        "one undo restores the hand-edited rows exactly"
    );
}

#[test]
fn regeneration_adds_missing_parts_after_the_last_row_and_deletes_surplus_ones() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    let cut = add_cut(&d, m, 99);
    regenerate(&d, m, grid, "K7Q2M", 3).unwrap();
    let list = ops(&d, m);
    assert_eq!(list.len(), 8);
    assert_eq!(
        list[7].id, cut,
        "new rows after the grid's last row, not at the end"
    );
    assert_eq!(d.test_grid(grid).unwrap().cells.len(), 6);
    assert_eq!(
        list.iter().filter(|op| op.name.starts_with("2 · ")).count(),
        2,
        "the third column's rows"
    );
    regenerate(&d, m, grid, "K7Q2M", 2).unwrap();
    assert_eq!(ops(&d, m).len(), 6, "the third column's rows went");
    assert_eq!(d.test_grid(grid).unwrap().cells.len(), 4);
}

#[test]
fn deleting_cutting_or_ungrouping_a_grid_removes_its_rows_in_the_same_step() {
    for how in ["delete", "ungroup"] {
        let d = Document::new(1);
        let m = laser(&d);
        let grid = place(&d, m, "K7Q2M", 2, 99);
        let cut = add_cut(&d, m, 99);
        let steps = d.own_step_count();
        match how {
            "delete" => d.delete_objects(&[grid]).unwrap(),
            _ => {
                d.ungroup(&[grid]).unwrap();
            }
        }
        assert_eq!(d.own_step_count(), steps + 1, "{how}");
        let list = ops(&d, m);
        assert_eq!(
            list.iter().map(|op| op.id).collect::<Vec<_>>(),
            [cut],
            "{how}"
        );
        undo(&d);
        assert_eq!(ops(&d, m).len(), 6, "{how}: undo brings the rows back");
        assert_eq!(d.test_grid(grid).unwrap().derived.len(), 5, "{how}");
    }
}

#[test]
fn a_move_off_the_page_removes_the_rows_in_the_move_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    let steps = d.own_step_count();
    let gone = d.batch("translate_objects", |d| {
        let grid_items = d.test_grid(grid).unwrap();
        let leaves: Vec<NodeId> = grid_items
            .cells
            .iter()
            .chain(&grid_items.labels)
            .map(|i| i.id)
            .collect();
        d.translate_objects(&leaves, Vec2::new(900.0, 0.0)).unwrap();
        d.remove_test_grid_rows(grid)
    });
    assert_eq!((gone, d.own_step_count()), (5, steps + 1));
    assert_eq!(ops(&d, m), []);
    assert!(d.test_grid(grid).is_some(), "the grid stays, without rows");
    undo(&d);
    assert_eq!(ops(&d, m).len(), 5);
}

#[test]
fn a_copy_gets_a_new_id_its_own_rows_and_no_lasered_register_in_the_copy_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    d.mark_test_grid_lasered_for_tests(grid, 1_762_862_400_000)
        .unwrap();
    assert_eq!(d.test_grid(grid).unwrap().lasered, Some(1_762_862_400_000));
    let source = [CopySource {
        id: grid,
        anchor_ids: Vec::new(),
    }];
    let (recipe, items, rows) = (recipe("8PL4D", 2), items("8PL4D", 2), rows("8PL4D", 2));
    let derived = derived(&rows, 99);
    let write = TestGridWrite {
        manufacturing: m,
        recipe: &recipe,
        items: &items,
        rows: &derived,
    };
    let steps = d.own_step_count();
    let copy = d.batch("duplicate_objects", |d| {
        let copy = d.duplicate_objects(&source, Vec2::new(0.0, 40.0)).unwrap()[0];
        let plain = d.test_grid(copy).unwrap();
        assert_eq!(
            (plain.lasered, plain.derived.len()),
            (None, 0),
            "copied bare"
        );
        assert_eq!(
            regenerate(d, m, copy, "8PL4D", 2),
            Err(TestGridError::IdChanged),
            "an edit keeps the id"
        );
        d.copy_test_grid(copy, &write).unwrap();
        copy
    });
    assert_eq!(d.own_step_count(), steps + 1, "one step");
    let snapshot = d.test_grid(copy).unwrap();
    assert_eq!((snapshot.lasered, snapshot.derived.len()), (None, 5));
    assert_eq!(snapshot.recipe["id"], "8PL4D");
    assert_eq!(ops(&d, m).len(), 10, "the original keeps its own rows");
    assert_eq!(d.test_grid(grid).unwrap().derived.len(), 5);
    assert_eq!(d.copy_test_grid(copy, &write), Err(TestGridError::IdKept));
}

#[test]
fn a_lasered_grid_changes_only_with_a_new_id_and_a_plain_grid_keeps_its_id() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    assert_eq!(
        regenerate(&d, m, grid, "8PL4D", 2),
        Err(TestGridError::IdChanged)
    );
    regenerate(&d, m, grid, "K7Q2M", 3).unwrap();
    d.mark_test_grid_lasered_for_tests(grid, 5).unwrap();
    regenerate(&d, m, grid, "K7Q2M", 3).unwrap();
    assert_eq!(
        d.test_grid(grid).unwrap().lasered,
        Some(5),
        "an unchanged recipe keeps it"
    );
    assert_eq!(
        regenerate(&d, m, grid, "K7Q2M", 2),
        Err(TestGridError::IdKept)
    );
    regenerate(&d, m, grid, "8PL4D", 2).unwrap();
    let snapshot = d.test_grid(grid).unwrap();
    assert_eq!(
        (snapshot.lasered, snapshot.recipe["id"].as_str()),
        (None, Some("8PL4D"))
    );
}

#[test]
fn a_hand_edit_of_a_lasered_grids_row_renews_the_id_in_the_same_step() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    assert_eq!(
        d.renew_test_id(grid, "8PL4D"),
        Ok(false),
        "not Lasered: the id stays"
    );
    d.mark_test_grid_lasered_for_tests(grid, 5).unwrap();
    assert_eq!(d.renew_test_id(grid, "K7Q2M"), Err(TestGridError::IdKept));
    let row = ops(&d, m)[2].id;
    assert_eq!(d.test_grid_of_derived_row(row), Some(grid));
    let steps = d.own_step_count();
    d.batch(crate::EDIT_OPERATION_SETTINGS_COMMIT_LABEL, |d| {
        let write = [SettingsWrite {
            operation: row,
            value: Some(numbers(77.0)),
        }];
        d.write_operation_settings(SettingsStep::EditSettings, &write)
            .unwrap();
        assert_eq!(d.renew_test_id(grid, "8PL4D"), Ok(true));
    });
    assert_eq!(d.own_step_count(), steps + 1);
    let snapshot = d.test_grid(grid).unwrap();
    assert_eq!(
        (snapshot.lasered, snapshot.recipe["id"].as_str()),
        (None, Some("8PL4D"))
    );
    let id_label = snapshot
        .labels
        .iter()
        .find(|l| l.tag == json!("id"))
        .unwrap();
    assert_eq!(d.text(id_label.id).unwrap().string, "8PL4D");
    let list = ops(&d, m);
    assert_eq!(list[0].name, "8PL4D labels");
    assert_eq!(list[2].name, "1 · 0 · 8PL4D");
    assert_eq!(
        list[2].settings,
        Some(Value::Object(numbers(77.0))),
        "the hand edit stays"
    );
}

#[test]
fn place_from_materials_writes_the_lasered_register_and_its_label() {
    let d = Document::new(1);
    let m = laser(&d);
    let (recipe, items, rows) = (recipe("K7Q2M", 2), items("K7Q2M", 2), rows("K7Q2M", 2));
    let derived = derived(&rows, 99);
    let grid = d
        .place_test_grid(&NewGridObject {
            name: "Test grid",
            write: TestGridWrite {
                manufacturing: m,
                recipe: &recipe,
                items: &items,
                rows: &derived,
            },
            lasered: Some(42),
            step: GridStep::Place,
        })
        .unwrap();
    assert_eq!(last_label(&d), "place_test_grid");
    assert_eq!(d.test_grid(grid).unwrap().lasered, Some(42));
    let no_id = map(json!({"kind": "cut"}));
    let refused = d.place_test_grid(&NewGridObject {
        name: "Test grid",
        write: TestGridWrite {
            manufacturing: m,
            recipe: &no_id,
            items: &items,
            rows: &derived,
        },
        lasered: None,
        step: GridStep::Add,
    });
    assert_eq!(refused, Err(TestGridError::NoId));
}

#[test]
fn an_unreadable_part_or_lasered_register_is_ordinary_and_never_swept() {
    let d = Document::new(1);
    let m = laser(&d);
    let grid = place(&d, m, "K7Q2M", 2, 99);
    let tree = d.loro().get_tree(OBJECTS_TREE);
    let first = tree_id_of(ops(&d, m)[0].id);
    let meta = tree.get_meta(first).unwrap();
    meta.insert(KEY_OP_GRID_PART, "labels").unwrap();
    tree.get_meta(tree_id_of(grid))
        .unwrap()
        .insert(KEY_TEST_GRID_LASERED, 1.5)
        .unwrap();
    d.commit_with_label("edit_test_grid");
    let snapshot = d.test_grid(grid).unwrap();
    assert_eq!((snapshot.lasered, snapshot.derived.len()), (None, 4));
    assert_eq!(ops(&d, m)[0].grid_part, Some(json!("labels")));
    d.delete_objects(&[grid]).unwrap();
    let left = ops(&d, m);
    assert_eq!(left.len(), 1, "the ordinary row stays when its grid goes");
    assert_eq!(left[0].name, "K7Q2M labels");
}
