//! The Engrave test mark is a circle (`specs/0146` criteria 18, 20, 21): the cells of an Engrave
//! grid are the ellipse primitive with equal radii, a Cut grid keeps its squares, an old grid is
//! changed only by the commit that regenerates it. The golden is
//! `tests/fixtures/test_grid_items/engrave_3x3_10mm.json` (`CURVYO_WRITE_FIXTURES=1` rewrites it).

use curvyo_document_core::{ObjectSnapshot, Shape};
use serde_json::{Value, json};

use super::super::Session;
use super::tests::{laser_with_fonts, place_grid, undo_label};

/// The cell items of the selected grid as `{column, row, shape, ...}`, in row-major order, and the
/// number of label items.
fn items(s: &Session, group: curvyo_document_core::NodeId) -> Value {
    let grid = s.grid_object(group).unwrap();
    let mut cells: Vec<Value> = grid
        .cells
        .iter()
        .map(|item| {
            let column = item.tag["column"].as_u64().unwrap();
            let row = item.tag["row"].as_u64().unwrap();
            let ObjectSnapshot::Primitive(primitive) = s.document.object(item.id).unwrap() else {
                panic!("a cell is a primitive");
            };
            let round = |n: f64| (n * 1000.0).round() / 1000.0;
            let shape = match primitive.shape {
                Shape::Ellipse { frame, arc, .. } => {
                    assert!(arc.is_none());
                    json!({
                        "shape": "ellipse",
                        "cx": round(frame.center.x),
                        "cy": round(frame.center.y),
                        "rx": round(frame.rx.as_mm()),
                        "ry": round(frame.ry.as_mm()),
                    })
                }
                Shape::Rect { bounds, .. } => json!({
                    "shape": "rect",
                    "x": round(bounds.origin.x),
                    "y": round(bounds.origin.y),
                    "width": round(bounds.width.as_mm()),
                }),
                _ => panic!("a cell is a rect or an ellipse"),
            };
            let mut one = json!({"column": column, "row": row});
            one.as_object_mut()
                .unwrap()
                .extend(shape.as_object().unwrap().clone());
            one
        })
        .collect();
    cells.sort_by_key(|c| (c["row"].as_u64().unwrap(), c["column"].as_u64().unwrap()));
    json!({"cells": cells, "labels": grid.labels.len()})
}

fn engrave_grid(s: &mut Session) -> curvyo_document_core::NodeId {
    let group = place_grid(s);
    s.set_test_grid_kind("engrave").unwrap();
    s.set_test_grid_field("columns_count", "3").unwrap();
    s.set_test_grid_field("rows_count", "3").unwrap();
    s.set_test_grid_field("cell", "10").unwrap();
    group
}

fn pinned() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/test_grid_items/engrave_3x3_10mm.json")
}

/// Criteria 18, 20: nine circles of diameter 10 mm on the 12 mm pitch, the labels unchanged.
#[test]
fn an_engrave_grid_is_nine_circles_pinned_by_a_golden() {
    let mut s = laser_with_fonts();
    let group = engrave_grid(&mut s);
    let got = items(&s, group);
    if std::env::var_os("CURVYO_WRITE_FIXTURES").is_some() {
        std::fs::create_dir_all(pinned().parent().unwrap()).unwrap();
        std::fs::write(pinned(), serde_json::to_string_pretty(&got).unwrap() + "\n").unwrap();
    }
    let want: Value = serde_json::from_str(&std::fs::read_to_string(pinned()).unwrap()).unwrap();
    assert_eq!(got, want);
    let cells = got["cells"].as_array().unwrap();
    assert_eq!(cells.len(), 9);
    assert!(
        cells
            .iter()
            .all(|c| c["shape"] == "ellipse" && c["rx"] == 5.0 && c["ry"] == 5.0)
    );
    let (first, second) = (&cells[0], &cells[1]);
    let pitch = second["cx"].as_f64().unwrap() - first["cx"].as_f64().unwrap();
    assert!(
        (pitch - 12.0).abs() < 1e-6,
        "the pitch is Cell size plus the 2 mm gap: {pitch}"
    );
}

/// Criterion 18: Cut cells stay squares.
#[test]
fn a_cut_grid_keeps_its_squares() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let got = items(&s, group);
    assert!(
        got["cells"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["shape"] == "rect")
    );
}

/// Criterion 21: squares turn to circles only at the commit that regenerates the grid, in that
/// one step; undo brings the squares back.
#[test]
fn squares_become_circles_at_the_regenerating_commit_and_undo_restores_them() {
    let mut s = laser_with_fonts();
    let group = place_grid(&mut s);
    let before = items(&s, group);
    let steps = s.undo_len();
    s.set_test_grid_kind("engrave").unwrap();
    assert_eq!(s.undo_len(), steps + 1);
    assert_eq!(undo_label(&mut s), "Edit test grid");
    let after = items(&s, group);
    assert!(
        after["cells"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["shape"] == "ellipse")
    );
    let _ = s.undo();
    assert_eq!(items(&s, group), before);
}
