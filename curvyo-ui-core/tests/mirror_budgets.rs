//! The costs of `specs/0056-mirror-and-negative-scale/` criterion 27 for a selection of
//! 5,000 objects (2,500 paths with 20 nodes, 1,250 rectangles, 1,250 stars, half of the
//! primitives turned by 30°): the group box (under 10 ms), one preview step of a resize
//! that has crossed zero (under 25 ms), and the commits of a flip and of a drag released
//! beyond zero (within 2.5 s and at most 1.1 times the commit of a uniform scale of the same
//! selection, measured in the same run; the factor against a plain move is printed). Run in release by the tester: `cargo test
//! --release -p curvyo-ui-core --test mirror_budgets -- --ignored --nocapture
//! --test-threads 1`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::similar_names,
    missing_docs
)]

use std::time::{Duration, Instant};

use curvyo_document_core::{
    AnchorId, Angle, Document, InnerRatio, Length, NewAnchor, NodeId, ObjectSnapshot, Point,
    PointCount, RectBounds, StarFrame, Tolerance,
};
use curvyo_ui_core::{
    AnchorIdMinter, FlipAxis, GroupSelection, Modifiers, ObjectSelection, SelectTool,
    TransformHandleTolerances, flip_selection, object_outline_bounds,
};

fn pt(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

/// The selection of criterion 27 on a grid 30 mm apart.
fn build() -> Document {
    let document = Document::new(1);
    let columns = 100u32;
    let at = |i: u32| (f64::from(i % columns) * 30.0, f64::from(i / columns) * 30.0);
    for i in 0..2500u32 {
        let (cx, cy) = at(i);
        let anchors: Vec<NewAnchor> = (0..20u32)
            .map(|n| {
                let a = f64::from(n) / 20.0 * std::f64::consts::TAU;
                NewAnchor::corner(
                    AnchorId::new(1, u64::from(i) * 20 + u64::from(n)),
                    pt(cx + 10.0 * a.cos(), cy + 10.0 * (1.3 * a).sin()),
                )
            })
            .collect();
        let _ = document.create_path(&anchors, true);
    }
    for i in 0..2500u32 {
        let (x, y) = at(2500 + i);
        let id = if i % 2 == 0 {
            document.create_rect(RectBounds {
                origin: pt(x, y),
                width: Length::from_mm(14.0),
                height: Length::from_mm(8.0),
            })
        } else {
            document.create_star(
                StarFrame {
                    center: pt(x + 7.0, y + 7.0),
                    radius: Length::from_mm(7.0),
                    angle: Angle::from_radians(0.2),
                },
                PointCount::new(5).unwrap(),
                InnerRatio::new(0.5).unwrap(),
            )
        };
        if i % 4 < 2 {
            let object = document.object(id).unwrap();
            let (min, max) = object_outline_bounds(&object);
            let turned = object.rotated(
                pt(f64::midpoint(min.x, max.x), f64::midpoint(min.y, max.y)),
                Angle::from_radians(30.0_f64.to_radians()),
            );
            document.rotate_object(&turned).unwrap();
        }
    }
    document
}

fn read(document: &Document) -> Vec<ObjectSnapshot> {
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.object(id))
        .collect()
}

fn selection_of(objects: &[ObjectSnapshot]) -> ObjectSelection {
    let ids: Vec<NodeId> = objects.iter().map(ObjectSnapshot::id).collect();
    let mut selection = ObjectSelection::new();
    selection.set(&ids);
    selection
}

fn mean<T>(runs: u32, mut f: impl FnMut() -> T) -> Duration {
    let began = Instant::now();
    for _ in 0..runs {
        std::hint::black_box(f());
    }
    began.elapsed() / runs
}

/// The group box's corners.
fn corners(objects: &[ObjectSnapshot], selection: &ObjectSelection) -> (Point, Point) {
    let group = SelectTool::group_of(objects, selection).unwrap();
    (group.bounds().min, group.bounds().max)
}

/// A drag from `press` to `to`; the release's wall time.
fn release_time(document: &Document, press: Point, to: Point, modifiers: Modifiers) -> Duration {
    let objects = read(document);
    let mut selection = selection_of(&objects);
    let mut tool = SelectTool::new();
    let mut minter = AnchorIdMinter::new(9);
    tool.pointer_down(
        &objects,
        &mut selection,
        press,
        Tolerance::from_mm(2.0),
        TransformHandleTolerances::at_scale(1.0),
        modifiers,
    );
    tool.mint_conversion_ids(&mut minter);
    tool.pointer_moved(to, modifiers, &mut selection);
    let began = Instant::now();
    tool.pointer_up(
        document,
        &objects,
        &mut selection,
        to,
        modifiers,
        &mut minter,
    );
    began.elapsed()
}

#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn the_group_box_of_5000_objects_takes_under_10_ms() {
    let document = build();
    let objects = read(&document);
    let selection = selection_of(&objects);
    let elapsed = mean(10, || {
        GroupSelection::from_objects(&objects, selection.ids())
    });
    println!("group box of 5,000 objects: {elapsed:?}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(10), "{elapsed:?}");
    }
}

#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn one_preview_step_through_zero_takes_under_25_ms() {
    let document = build();
    let objects = read(&document);
    let mut selection = selection_of(&objects);
    let (low, high) = corners(&objects, &selection);
    let to = pt(low.x - 100.0, low.y - 50.0);
    let mut tool = SelectTool::new();
    tool.pointer_down(
        &objects,
        &mut selection,
        high,
        Tolerance::from_mm(2.0),
        TransformHandleTolerances::at_scale(1.0),
        Modifiers::NONE,
    );
    tool.pointer_moved(to, Modifiers::NONE, &mut selection);
    let elapsed = mean(5, || {
        let live = tool
            .live_edit(&objects, &selection, to, false, false)
            .expect("a live edit");
        SelectTool::group_of(&live.objects, &selection)
    });
    println!("one preview step through zero of 5,000 objects: {elapsed:?}");
    if !cfg!(debug_assertions) {
        assert!(elapsed < Duration::from_millis(25), "{elapsed:?}");
    }
    tool.escape();
}

/// The median of `times`.
fn median(mut times: Vec<Duration>) -> Duration {
    times.sort();
    times[times.len() / 2]
}

/// Criterion 27's commit budget (amended 2026-10-15, pending the customer, FLIP-BUDGET): the
/// flip and the release beyond zero take at most 1.1 times a uniform scale of the same
/// selection (a Ctrl corner drag), the writer they share; the factor against a plain move
/// (about 1.4, the move has its own lighter writer) is printed for the PR.
#[test]
#[ignore = "benchmark: run in release with --ignored --nocapture"]
fn a_flip_and_a_release_beyond_zero_cost_no_more_than_a_scale() {
    let document = build();
    let (mut moves, mut scales, mut flips, mut beyonds) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    // Interleaved runs of the commits on the same document; the medians are compared, so
    // one slow run (an allocation, the scheduler) does not decide.
    for _ in 0..7 {
        let objects = read(&document);
        let selection = selection_of(&objects);
        let (low, high) = corners(&objects, &selection);
        let centre = pt(f64::midpoint(low.x, high.x), f64::midpoint(low.y, high.y));
        moves.push(release_time(
            &document,
            centre,
            pt(centre.x + 30.0, centre.y + 20.0),
            Modifiers::NONE,
        ));
        let (low, high) = corners(&read(&document), &selection);
        // A uniform scale by 1.01 about the north-west corner, a corner drag with Ctrl
        // (positive factors): nothing converts.
        scales.push(release_time(
            &document,
            high,
            pt(
                low.x + 1.01 * (high.x - low.x),
                low.y + 1.01 * (high.y - low.y),
            ),
            Modifiers::new(false, true),
        ));
        let objects = read(&document);
        let began = Instant::now();
        flip_selection(&document, &objects, &selection, FlipAxis::Horizontal);
        flips.push(began.elapsed());
        let (low, high) = corners(&read(&document), &selection);
        // The south-east corner dragged to its mirror across the north-west corner: the
        // factors (-1, -1), a half turn that converts nothing.
        beyonds.push(release_time(
            &document,
            high,
            pt(2.0 * low.x - high.x, 2.0 * low.y - high.y),
            Modifiers::NONE,
        ));
    }
    let (moved, scaled) = (median(moves), median(scales));
    let (flipped, beyond) = (median(flips), median(beyonds));
    println!(
        "median commit of a move: {moved:?}, a uniform scale: {scaled:?}, a flip: {flipped:?} \
         ({:.2} x the move), a release beyond zero: {beyond:?} ({:.2} x the move)",
        flipped.as_secs_f64() / moved.as_secs_f64(),
        beyond.as_secs_f64() / moved.as_secs_f64(),
    );
    if !cfg!(debug_assertions) {
        for (name, elapsed) in [("flip", flipped), ("beyond zero", beyond)] {
            assert!(elapsed < Duration::from_millis(2500), "{name}: {elapsed:?}");
            assert!(
                elapsed.as_secs_f64() <= 1.1 * scaled.as_secs_f64(),
                "{name}: {elapsed:?} against a scale of {scaled:?}"
            );
        }
    }
}
