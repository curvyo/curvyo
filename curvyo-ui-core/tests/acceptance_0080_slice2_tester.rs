//! Independent tester cases for `specs/0080-text-on-path/specification.md` slice 2 on
//! `curvyo-ui-core`: the clone block of a text clone without the Placement row, Select original
//! and Select clones (criterion 40), and the budget of criterion 44.
//!
//! Written from the specification and the public names before the implementation was read.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    missing_docs
)]

use std::path::PathBuf;
use std::time::{Duration, Instant};

use curvyo_document_core::{
    Affine, AnchorId, CarrierRef, Document, EllipseFrame, FontKind, FontRef, Length, NewAnchor,
    NodeId, ObjectSnapshot, OnPathValues, Point, Shape, TextChars, TextLayoutValues,
    text_path_previews,
};
use curvyo_text_core::{Face, FaceStore};
use curvyo_ui_core::{clone_block, lay_out, select_clones, select_originals};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn sans() -> std::sync::Arc<Face> {
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf"),
    )
    .unwrap();
    let mut store = FaceStore::new();
    let key = store.add(bytes, 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn font() -> FontRef {
    FontRef {
        family: "Source Sans 3".to_owned(),
        style: "Regular".to_owned(),
        face_hash: "c".repeat(64),
        face_index: 0,
        kind: FontKind::Outline,
    }
}

fn circle(d: &Document, i: usize) -> NodeId {
    let cx = 100.0 + 120.0 * f64::from(u32::try_from(i).unwrap());
    d.create_ellipse(EllipseFrame {
        center: pt(cx, 100.0),
        rx: Length::from_mm(50.0),
        ry: Length::from_mm(50.0),
    })
}

/// A source text of `chars` characters on `n` circles: (doc, source, clones, circles).
fn family(n: usize, chars: usize) -> (Document, NodeId, Vec<NodeId>, Vec<NodeId>) {
    let d = Document::new(1);
    let cs: Vec<NodeId> = (0..n).map(|i| circle(&d, i)).collect();
    let s: String = "abcdefghij".repeat(chars / 10 + 1)[..chars].to_owned();
    let t = d
        .create_text(
            &s,
            &font(),
            Length::from_mm(3.0),
            Affine::IDENTITY,
            &TextLayoutValues::DEFAULT,
        )
        .unwrap();
    let carriers: Vec<(CarrierRef, OnPathValues)> = cs
        .iter()
        .map(|&c| {
            (
                CarrierRef {
                    node: c,
                    outline: 0,
                },
                OnPathValues::at(Length::from_mm(150.0)),
            )
        })
        .collect();
    let clones = d.put_text_on_paths(t, &carriers, true).unwrap();
    (d, t, clones, cs)
}

/// Criterion 40: a text clone's clone block has no Placement row (a linked clone's has); Select
/// original gives the source, Select clones on the source gives its text clones.
#[test]
fn c40_clone_block_without_placement_row_and_select_original_clones() {
    let (d, t, clones, _) = family(3, 4);
    let path = d.create_path(
        &[
            NewAnchor::corner(AnchorId::new(9, 1), pt(0.0, 300.0)),
            NewAnchor::corner(AnchorId::new(9, 2), pt(50.0, 300.0)),
        ],
        false,
    );
    let linked = d.linked_clone(&[path], &[]).unwrap()[0];
    let read = d.read_objects();
    let block = clone_block(&read, &[clones[0]]).expect("a clone block for a text clone");
    assert!(!block.placement_row, "no Placement row for a text clone");
    assert_eq!(block.count, 1);
    let linked_block = clone_block(&read, &[linked]).unwrap();
    assert!(linked_block.placement_row, "a linked clone keeps it");
    assert_eq!(select_originals(&read, &[clones[1]]), vec![t]);
    let mut got = select_clones(&read, &[t]);
    got.sort_by_key(|id| format!("{id:?}"));
    let mut want = clones.clone();
    want.sort_by_key(|id| format!("{id:?}"));
    assert_eq!(got, want);
}

/// Criterion 44: one source of 100 characters with 20 clones on 20 circles: a typed character is
/// read and laid out on all 21 within 50 ms; a radius drag frame of one circle within 33 ms.
/// Release build: `cargo nextest run --release -p curvyo-ui-core --test
/// acceptance_0080_slice2_tester --run-ignored only`.
#[test]
#[ignore = "release-mode benchmark"]
fn c44_budget_typing_on_21_and_radius_drag() {
    let face = sans();
    let (d, t, clones, cs) = family(21, 100);
    assert_eq!(clones.len(), 20);
    let mut worst = Duration::ZERO;
    for i in 0..10 {
        let s = format!("{}{}", d.text(t).unwrap().string, i % 10);
        let t0 = Instant::now();
        d.edit_text(t, &s, &TextChars::default()).unwrap();
        let objects = d.read_objects().objects;
        let mut drawn = 0;
        for o in objects.iter() {
            if let ObjectSnapshot::Text(x) = o {
                let out = lay_out(x, &face, false).unwrap();
                assert_ne!(out.glyph_outlines.len(), 0);
                drawn += 1;
            }
        }
        worst = worst.max(t0.elapsed());
        assert_eq!(drawn, 21);
    }
    println!("typed character on 21 texts, worst {worst:?}");
    assert!(worst < Duration::from_millis(50), "typing {worst:?}");

    let objects = d.read_objects().objects;
    let mut prim = d.primitive(cs[7]).unwrap();
    let mut worst = Duration::ZERO;
    for i in 0..30 {
        if let Shape::Ellipse { frame, .. } = &mut prim.shape {
            frame.rx = Length::from_mm(40.0 + f64::from(i));
            frame.ry = frame.rx;
        }
        let t0 = Instant::now();
        let previews =
            text_path_previews(&objects, &[ObjectSnapshot::Primitive(prim.clone())], 100).unwrap();
        assert!(
            !previews.is_empty(),
            "the clone on the dragged circle follows"
        );
        for p in &previews {
            let _ = lay_out(p, &face, false);
        }
        worst = worst.max(t0.elapsed());
    }
    println!("radius drag worst frame {worst:?}");
    assert!(worst < Duration::from_millis(33), "radius drag {worst:?}");
}
