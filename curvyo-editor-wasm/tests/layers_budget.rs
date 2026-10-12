//! The time budgets of the Layers panel (`specs/0050-layers-and-objects-panel` criteria 48 to 52),
//! measured through the session as the host calls it. `#[ignore]`d: the budgets are for a release
//! build on the CI runner:
//!
//! ```text
//! cargo test --release -p curvyo-editor-wasm --test layers_budget -- --ignored --nocapture
//! ```
//!
//! The browser build may take up to twice as long (the spec's allowance); these numbers are native.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]

use std::time::{Duration, Instant};

use curvyo_document_core::{Document, Length, NodeId, Point, RectBounds, pack};
use curvyo_editor_wasm::{KeyInput, KeyOutcome, RowPress, Session, Tool};

const OBJECTS: usize = 10_000;
const LAYERS: usize = 20;

/// `OBJECTS` rectangles in `LAYERS` layers, opened as a session; the layer ids back to front.
fn session_with_layers(objects: usize, layers: usize) -> (Session, Vec<NodeId>) {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..layers).map(|_| document.new_layer(None)).collect();
    let per_layer = objects / layers;
    for (n, &layer) in ids.iter().enumerate() {
        document.set_draw_parent(Some(layer));
        document.batch("create_rect", |document| {
            for i in 0..per_layer {
                let x = (i % 100) as f64 * 20.0;
                let y = (n * per_layer / 100 + i / 100) as f64 * 20.0;
                let _ = document.create_rect(RectBounds {
                    origin: Point::new(x, y),
                    width: Length::from_mm(10.0),
                    height: Length::from_mm(10.0),
                });
            }
        });
    }
    let mut session = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1600.0, 900.0);
    session.set_tool(Tool::Select);
    (session, ids)
}

fn key(id: NodeId) -> String {
    serde_json::to_string(&id).unwrap()
}

fn timed<R>(what: &str, budget: Duration, run: impl FnOnce() -> R) -> R {
    let started = Instant::now();
    let result = run();
    let took = started.elapsed();
    println!("{what}: {took:?} (budget {budget:?})");
    assert!(took <= budget, "{what} took {took:?}, budget {budget:?}");
    result
}

fn row_count(view: &str) -> usize {
    let view: serde_json::Value = serde_json::from_str(view).unwrap();
    view["rows"].as_array().unwrap().len()
}

/// Criterion 48: the first rows within 100 ms, at most 60 rows cross, the end in one frame.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn the_first_rows_and_a_jump_to_the_end() {
    let (mut session, _) = session_with_layers(OBJECTS, LAYERS);
    let first = timed("first window", Duration::from_millis(100), || {
        session.layers_view_json(0, 60)
    });
    assert!(row_count(&first) <= 60);
    let total = usize::try_from(
        serde_json::from_str::<serde_json::Value>(&first).unwrap()["count"]
            .as_u64()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        total,
        OBJECTS + LAYERS,
        "the scrollbar is exact from the first frame"
    );
    let end = timed("jump to the end", Duration::from_millis(16), || {
        session.layers_view_json(total - 60, 60)
    });
    assert_eq!(row_count(&end), 60);
}

/// Criterion 49: Ctrl+A over 10,000 objects shows on the canvas and the rows within 100 ms.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn ctrl_a_over_ten_thousand_objects() {
    let (mut session, _) = session_with_layers(OBJECTS, LAYERS);
    let _ = session.layers_view_json(0, 60);
    timed("Ctrl+A, frame and rows", Duration::from_millis(100), || {
        let outcome = session.key_down(KeyInput {
            key: "a",
            ctrl: true,
            ..KeyInput::default()
        });
        assert_eq!(outcome, KeyOutcome::SelectedAll);
        let _ = session.draw_list();
        session.layers_view_json(0, 60)
    });
    assert_eq!(session.selected_object_count(), OBJECTS);
}

/// Criterion 50: hiding, showing, locking and fading a layer of 10,000 redraws within 150 ms.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn a_layer_of_ten_thousand_hides_shows_locks_and_fades() {
    let (mut session, layers) = session_with_layers(OBJECTS, 1);
    let layer = layers[0];
    let _ = session.draw_list();
    for (what, flag) in [
        ("hide", Some(curvyo_editor_wasm::RowFlag::Visible)),
        ("show", Some(curvyo_editor_wasm::RowFlag::Visible)),
        ("lock", Some(curvyo_editor_wasm::RowFlag::Locked)),
        ("unlock", Some(curvyo_editor_wasm::RowFlag::Locked)),
        ("opacity", None),
    ] {
        timed(what, Duration::from_millis(150), || {
            if let Some(flag) = flag {
                session.toggle_layer_row_flag(&key(layer), flag);
            } else {
                session.press_layer_row(&key(layer), RowPress::Plain);
                session.commit_layer_opacity(50.0);
            }
            let _ = session.draw_list();
        });
    }
}

/// The id text of the row at `index`.
fn row_id(session: &mut Session, index: usize) -> String {
    let view: serde_json::Value =
        serde_json::from_str(&session.layers_view_json(index, 1)).unwrap();
    view["rows"][0]["id"].to_string()
}

/// Criterion 51: one row within 100 ms, 1,000 within 1 s, 10,000 within 5 s.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn moves_of_one_a_thousand_and_ten_thousand_rows() {
    // Rows: the empty front layer (0), the back layer (1), its 10,000 children (2 on).
    let (mut session, layers) = session_with_layers(OBJECTS, 1);
    let front = layers[0];
    session.new_layer_row();
    let empty = session.layers_view_json(0, 1);
    let empty: serde_json::Value = serde_json::from_str(&empty).unwrap();
    let empty = empty["rows"][0]["id"].to_string();
    let (one, anchor) = (row_id(&mut session, 2), row_id(&mut session, 12));
    timed("move one row", Duration::from_millis(100), || {
        session.drop_layer_rows(&one, &anchor, "below");
    });
    let (from, to) = (row_id(&mut session, 100), row_id(&mut session, 1099));
    session.press_layer_row(&from, RowPress::Plain);
    session.press_layer_row(&to, RowPress::Range);
    assert_eq!(session.selected_object_count(), 1000);
    timed(
        "move 1,000 rows into another layer",
        Duration::from_secs(1),
        || {
            session.drop_layer_rows(&from, &empty, "into");
            let _ = session.draw_list();
        },
    );
    session.select_layer_contents(&key(front));
    let pressed = row_id(&mut session, 1 + 1000 + 1);
    let all = session.selected_object_count();
    assert_eq!(all, OBJECTS - 1000);
    timed(
        "move the rest into the other layer",
        Duration::from_secs(5),
        || {
            session.drop_layer_rows(&pressed, &empty, "into");
            let _ = session.draw_list();
        },
    );
    let shown: serde_json::Value = serde_json::from_str(&session.layers_view_json(0, 1)).unwrap();
    assert_eq!(shown["count"], OBJECTS + 2);
}

/// Criterion 52: a layer of 10,000 children expands and collapses within 100 ms each.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn a_layer_of_ten_thousand_expands_and_collapses() {
    let (mut session, layers) = session_with_layers(OBJECTS, 1);
    let _ = session.layers_view_json(0, 60);
    timed("collapse", Duration::from_millis(100), || {
        assert!(session.set_layer_expanded(&key(layers[0]), false));
        session.layers_view_json(0, 60)
    });
    timed("expand", Duration::from_millis(100), || {
        assert!(session.set_layer_expanded(&key(layers[0]), true));
        session.layers_view_json(0, 60)
    });
}

/// Criterion 15 within the frame budget: the hover box of a group row of 5,000 leaves in a
/// 10,000-object document costs one frame, every frame the pointer rests there.
#[test]
#[ignore = "a timing budget for release builds; run on purpose"]
fn hovering_a_group_row_of_five_thousand_leaves() {
    let document = Document::new(1);
    let ids: Vec<NodeId> = (0..OBJECTS)
        .map(|i| {
            document.create_rect(RectBounds {
                origin: Point::new((i % 100) as f64 * 20.0, (i / 100) as f64 * 20.0),
                width: Length::from_mm(10.0),
                height: Length::from_mm(10.0),
            })
        })
        .collect();
    let group = document.group_objects(&ids[..OBJECTS / 2]).unwrap();
    let mut session = Session::open(2, &pack(&document, "0.1.0").unwrap()).unwrap();
    session.resize_viewport(1600.0, 900.0);
    session.set_tool(Tool::Select);
    let _ = session.draw_list();
    session.set_layer_hover(&key(group));
    timed(
        "a frame with the group row hovered",
        Duration::from_millis(16),
        || {
            let _ = session.draw_list();
        },
    );
}
