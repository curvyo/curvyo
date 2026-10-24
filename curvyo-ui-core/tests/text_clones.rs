//! Text clones in the panel, the marks and the style scope (`specs/0080-text-on-path` criterion
//! 40, UX notes §9), and the budget of criterion 44: one source of 100 characters with 20 clones
//! on 20 circles.
//!
//! The budgets are asserted in release builds only: `cargo test --release -p curvyo-ui-core --test
//! text_clones`.

#![allow(clippy::unwrap_used, missing_docs)]

use std::collections::HashSet;
use std::time::{Duration, Instant};

use curvyo_document_core::{
    Affine, Angle, CarrierRef, Curve, Document, EllipseFrame, FontKind, FontRef, Length, NodeId,
    ObjectSnapshot, OnPathValues, Point, Shape, TextChars, TextLayoutValues, ViewTransform,
    text_path_previews,
};
use curvyo_text_core::{Face, FaceStore};
use curvyo_ui_core::{
    MarkFrame, MarkRole, OverrideState, clone_block, clone_marks, followers_of, lay_out,
    object_name, owns_style, select_clones, select_originals,
};

fn sans() -> std::sync::Arc<Face> {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../frontend/public/fonts/source-sans-3/SourceSans3-Regular.otf");
    let mut store = FaceStore::new();
    let key = store.add(std::fs::read(path).unwrap(), 0).unwrap();
    store.get(&key).unwrap().clone()
}

fn circle(d: &Document, x: f64) -> NodeId {
    d.create_primitive(
        Shape::Ellipse {
            frame: EllipseFrame {
                center: Point::new(x, 100.0),
                rx: Length::from_mm(40.0),
                ry: Length::from_mm(40.0),
            },
            arc: None,
            curve: Curve::ELLIPSE_DEFAULT,
        },
        Angle::from_radians(0.0),
    )
}

/// A source of `string` on `count` circles: the source and its clones.
fn on_circles(string: &str, count: u32) -> (Document, NodeId, Vec<NodeId>) {
    let d = Document::new(1);
    let t = d
        .create_text(
            string,
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
    let carriers: Vec<(CarrierRef, OnPathValues)> = (0..count)
        .map(|i| {
            (
                CarrierRef {
                    node: circle(&d, f64::from(i) * 100.0),
                    outline: 0,
                },
                OnPathValues::at(Length::from_mm(10.0)),
            )
        })
        .collect();
    let clones = d.put_text_on_paths(t, &carriers, true).unwrap();
    (d, t, clones)
}

/// Criterion 40: the clone block of a text clone, its name, Select original and Select clones.
#[test]
fn c40_a_text_clone_is_a_clone_in_the_panel() {
    let (d, t, clones) = on_circles("Hello", 3);
    let read = d.read_objects();
    assert_eq!(
        object_name(&read, clones[0]),
        format!("Clone of Text {}", t.short_id())
    );
    let block = clone_block(&read, &clones).unwrap();
    assert_eq!(block.count, 2);
    assert!(!block.placement_row, "a text clone has no Placement row");
    assert!(block.texts);
    assert_eq!(block.style, OverrideState::Original);
    assert_eq!(select_originals(&read, &clones[..1]), vec![t]);
    assert_eq!(select_clones(&read, &[t]), clones);
    assert_eq!(followers_of(&read, &[t]), (2, 1));
    assert!(!owns_style(read.get(clones[0]).unwrap()));
    d.set_own_style_and_texts(&[], &clones[..1], true).unwrap();
    let read = d.read_objects();
    assert!(owns_style(read.get(clones[0]).unwrap()));
    assert_eq!(
        clone_block(&read, &clones).unwrap().style,
        OverrideState::Mixed
    );
    assert!(clone_block(&read, &[t]).is_none(), "the source is no clone");
}

/// UX notes §9: a selected text clone shows the marks on its source and its sibling clones.
#[test]
fn a_selected_text_clone_marks_its_source_and_siblings() {
    let face = sans();
    let (d, t, clones) = on_circles("Hello", 3);
    let objects: Vec<ObjectSnapshot> = d
        .read_objects()
        .objects
        .iter()
        .cloned()
        .map(|mut object| {
            if let ObjectSnapshot::Text(text) = &mut object {
                text.outline = lay_out(text, &face, false).map(std::sync::Arc::new);
            }
            object
        })
        .collect();
    let hidden = HashSet::new();
    let marks = clone_marks(
        &objects,
        &clones[..1],
        &MarkFrame {
            view: ViewTransform::new(2.0, Point::new(-50.0, 0.0)),
            viewport: (1200.0, 800.0),
            obstacles: &[],
            suppressed: false,
            hidden: &hidden,
        },
    );
    let role = |id: NodeId| marks.traces.iter().find(|m| m.id == id).map(|m| m.role);
    assert_eq!(role(clones[0]), Some(MarkRole::Clone));
    assert_eq!(role(t), Some(MarkRole::Original));
    assert_eq!(role(clones[1]), Some(MarkRole::FaintClone));
}

fn report(name: &str, elapsed: Duration, budget: Duration) {
    println!("{name}: {elapsed:?}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < budget, "{name}: {elapsed:?} over {budget:?}");
    }
}

/// Criterion 44: a typed character reaches the source and its 20 clones within 50 ms (the read
/// and the layout of all 21); a radius drag of one circle keeps its frames within 33 ms.
#[test]
fn c44_twenty_clones_follow_a_typed_character_and_a_radius_drag() {
    let face = sans();
    let string: String = "Curvyo ring text ".chars().cycle().take(100).collect();
    let (d, t, clones) = on_circles(&string, 21);
    assert_eq!(clones.len(), 20);
    let mut worst = Duration::ZERO;
    for step in 0..5 {
        let typed = format!("{string}{step}");
        let started = Instant::now();
        d.edit_text(t, &typed, &TextChars::default()).unwrap();
        let read = d.read_objects();
        let mut laid = 0;
        for object in read.objects.iter() {
            if let ObjectSnapshot::Text(text) = object {
                assert_eq!(text.string, typed);
                assert!(lay_out(text, &face, false).is_some());
                laid += 1;
            }
        }
        assert_eq!(laid, 21);
        worst = worst.max(started.elapsed());
    }
    report(
        "typed character on 21 texts",
        worst,
        Duration::from_millis(50),
    );
    let read = d.read_objects();
    let carrier = read
        .objects
        .iter()
        .find(|o| matches!(o, ObjectSnapshot::Primitive(_)))
        .unwrap()
        .clone();
    let mut worst = Duration::ZERO;
    for step in 0..10 {
        let started = Instant::now();
        let mut dragged = carrier.clone();
        if let ObjectSnapshot::Primitive(primitive) = &mut dragged
            && let Shape::Ellipse { frame, .. } = &mut primitive.shape
        {
            frame.rx = Length::from_mm(40.0 + f64::from(step));
            frame.ry = frame.rx;
        }
        let previews = text_path_previews(&read.objects, &[dragged], 500).unwrap();
        assert_eq!(previews.len(), 1);
        assert!(lay_out(&previews[0], &face, false).is_some());
        worst = worst.max(started.elapsed());
    }
    report("radius drag frame", worst, Duration::from_millis(33));
}
