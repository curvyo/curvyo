//! The budgets of `specs/0080-text-on-path` criteria 43 and 45: a frame of a position-handle drag
//! and of a carrier node drag for 200 characters on 1,000 cubic segments within 33 ms; a carrier
//! of 100,000 segments and letters beyond ±1e7 mm neither hang nor crash.
//!
//! The budgets are asserted in release builds only; a debug build runs the same cases and prints
//! the timings: `cargo test --release -p curvyo-ui-core --test text_on_path_budgets`. Measured on
//! the implementer VM (navi, release build).

#![allow(clippy::unwrap_used, clippy::cast_precision_loss, missing_docs)]

use std::time::{Duration, Instant};

use curvyo_document_core::{
    Affine, AnchorId, AnchorKind, CarrierRef, Document, FontKind, FontRef, Length, NewAnchor,
    NodeId, ObjectSnapshot, OnPathValues, Point, TextLayoutValues, TextSnapshot, Vec2,
    text_path_previews,
};
use curvyo_text_core::{Face, FaceStore};
use curvyo_ui_core::{dragged_values, lay_out, measure_of};

fn sans() -> std::sync::Arc<Face> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
    let mut store = FaceStore::new();
    let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
    store.get(&key).unwrap().clone()
}

/// A wavy open path of `segments` cubics, 2 mm apart.
fn wave(segments: usize) -> Vec<NewAnchor> {
    (0..=segments)
        .map(|i| NewAnchor {
            id: AnchorId::new(9, i as u64 + 1),
            point: Point::new(i as f64 * 2.0, if i % 2 == 0 { 0.0 } else { 3.0 }),
            handle_in: Vec2::new(-0.7, 0.0),
            handle_out: Vec2::new(0.7, 0.0),
            kind: AnchorKind::Symmetric,
        })
        .collect()
}

fn text_on(segments: usize, chars: usize) -> (Document, NodeId, NodeId) {
    let d = Document::new(1);
    let string: String = "Curvyo text on a path "
        .chars()
        .cycle()
        .take(chars)
        .collect();
    let t = d
        .create_text(
            &string,
            &FontRef {
                family: "Source Sans 3".to_owned(),
                style: "Regular".to_owned(),
                face_hash: "c".repeat(64),
                face_index: 0,
                kind: FontKind::Outline,
            },
            Length::from_mm(5.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let c = d.create_path(&wave(segments), false);
    d.put_text_on_path(
        t,
        CarrierRef {
            node: c,
            outline: 0,
        },
        &OnPathValues::at(Length::from_mm(10.0)),
        true,
    )
    .unwrap();
    (d, t, c)
}

fn text_of(objects: &[ObjectSnapshot], id: NodeId) -> TextSnapshot {
    objects
        .iter()
        .find_map(|o| match o {
            ObjectSnapshot::Text(t) if t.id == id => Some((**t).clone()),
            _ => None,
        })
        .unwrap()
}

fn report(name: &str, elapsed: Duration, budget: Duration) {
    println!("{name}: {elapsed:?}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < budget, "{name}: {elapsed:?} over {budget:?}");
    }
}

/// Criterion 43, position drag: per frame the nearest arc length and the layout on the carrier.
#[test]
fn c43_a_position_drag_frame_within_33_ms() {
    let face = sans();
    let (d, t, _) = text_on(1_000, 200);
    let read = d.read_objects();
    let text = text_of(&read.objects, t);
    let mut worst = Duration::ZERO;
    for step in 0..10 {
        let started = Instant::now();
        let curve = text.on_path.as_ref().unwrap().curve.as_ref().unwrap();
        let measure = measure_of(curve);
        let values = dragged_values(
            &measure,
            text.on_path.as_ref().unwrap().values,
            Point::new(100.0 + f64::from(step) * 30.0, -5.0),
            1.0,
            None,
        );
        let mut moved = text.clone();
        moved.on_path.as_mut().unwrap().values = values;
        let outline = lay_out(&moved, &face, false).unwrap();
        assert_ne!(outline.glyph_outlines.len(), 0);
        worst = worst.max(started.elapsed());
    }
    report("position drag frame", worst, Duration::from_millis(33));
}

/// Criterion 43, node drag of the carrier: per frame the preview of the texts on it and their
/// layout.
#[test]
fn c43_a_carrier_node_drag_frame_within_33_ms() {
    let face = sans();
    let (d, t, c) = text_on(1_000, 200);
    let read = d.read_objects();
    let ObjectSnapshot::Path(carrier) = read.get(c).unwrap().clone() else {
        panic!("a path")
    };
    let mut worst = Duration::ZERO;
    for step in 0..10 {
        let started = Instant::now();
        let mut dragged = carrier.clone();
        dragged.anchors[500].point = Point::new(1_000.0, f64::from(step) * 4.0);
        let previews =
            text_path_previews(&read.objects, &[ObjectSnapshot::Path(dragged)], 500).unwrap();
        assert_eq!(previews.len(), 1);
        assert_eq!(previews[0].id, t);
        let outline = lay_out(&previews[0], &face, false).unwrap();
        assert_ne!(outline.glyph_outlines.len(), 0);
        worst = worst.max(started.elapsed());
    }
    report("node drag frame", worst, Duration::from_millis(33));
}

/// Criterion 45: 100,000 segments do not hang; letters beyond ±1e7 mm are left out.
#[test]
fn c45_huge_carriers_and_far_letters() {
    let face = sans();
    let (d, t, _) = text_on(100_000, 50);
    let started = Instant::now();
    let read = d.read_objects();
    let text = text_of(&read.objects, t);
    let outline = lay_out(&text, &face, false).unwrap();
    assert_ne!(outline.glyph_outlines.len(), 0);
    report(
        "100,000 segments",
        started.elapsed(),
        Duration::from_secs(5),
    );
    let mut far = text.clone();
    far.on_path.as_mut().unwrap().values.position = Length::from_mm(1e9);
    let outline = lay_out(&far, &face, false).unwrap();
    assert!(
        outline.glyph_outlines.is_empty(),
        "every letter beyond the limit is left out"
    );
    assert!(
        outline
            .line_box
            .iter()
            .all(|p| p.x.is_finite() && p.y.is_finite())
    );
}
