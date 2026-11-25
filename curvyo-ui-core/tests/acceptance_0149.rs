//! Acceptance tests of `specs/0149-path-box/specification.md`, slice 1, the ui-core part of
//! criterion 1 (create from one selected closed outline, the flat layout right of the source, the
//! refusals with "Nothing was changed.", the source untouched, one step) and of criterion 8 (Replace
//! outline as one step). Black-box: the public instance-source functions and the document.
//!
//! Written before the implementation was read. Not covered here (named in the report): the pick
//! mode and the Select bar button (pointer and UI state, no public seam found), the "nothing
//! selected" refusal (the entry point takes one source id), the hover marks, the history label
//! "Add path box" (no public label read was found in this file's seams).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::float_cmp)]

use curvyo_document_core::{
    AnchorKind, Document, Length, NewAnchor, NodeId, Point, RectBounds, Vec2,
};
use curvyo_extension_core::{ClosedPath, ParamValue};
use curvyo_ui_core::instance_source::{self, SourceRefusal};
use curvyo_ui_core::{AnchorIdMinter, ExtensionSettings};

const PATH_BOX: &str = "curvyo.generators/path-box";

/// A closed outline of straight edges through `points`, as a drawn path.
fn drawn_polygon(
    document: &Document,
    minter: &mut AnchorIdMinter,
    points: &[(f64, f64)],
) -> NodeId {
    let anchors: Vec<NewAnchor> = points
        .iter()
        .map(|&(x, y)| NewAnchor {
            id: minter.mint(),
            kind: AnchorKind::Corner,
            point: Point::new(x, y),
            handle_in: Vec2::new(0.0, 0.0),
            handle_out: Vec2::new(0.0, 0.0),
        })
        .collect();
    document.create_path(&anchors, true)
}

/// A closed outline with one curved edge (its first anchor has an outgoing handle).
fn drawn_curved(document: &Document, minter: &mut AnchorIdMinter) -> NodeId {
    let corners = [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0), (0.0, 40.0)];
    let anchors: Vec<NewAnchor> = corners
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| NewAnchor {
            id: minter.mint(),
            kind: AnchorKind::Corner,
            point: Point::new(x, y),
            handle_in: Vec2::new(0.0, 0.0),
            handle_out: if i == 0 {
                Vec2::new(20.0, -15.0)
            } else {
                Vec2::new(0.0, 0.0)
            },
        })
        .collect();
    document.create_path(&anchors, true)
}

/// An open polyline (not a closed outline).
fn drawn_open(document: &Document, minter: &mut AnchorIdMinter) -> NodeId {
    let anchors: Vec<NewAnchor> = [(0.0, 0.0), (60.0, 0.0), (60.0, 40.0)]
        .iter()
        .map(|&(x, y)| NewAnchor {
            id: minter.mint(),
            kind: AnchorKind::Corner,
            point: Point::new(x, y),
            handle_in: Vec2::new(0.0, 0.0),
            handle_out: Vec2::new(0.0, 0.0),
        })
        .collect();
    document.create_path(&anchors, false)
}

fn drawn_rect(document: &Document, x: f64, y: f64, w: f64, h: f64) -> NodeId {
    document.create_rect(RectBounds {
        origin: Point::new(x, y),
        width: Length::from_mm(w),
        height: Length::from_mm(h),
    })
}

/// The page-mm bounds of an object: (min, max).
fn bounds_of(document: &Document, id: NodeId) -> (Point, Point) {
    let object = document.object(id).expect("the object exists");
    curvyo_ui_core::object_outline_bounds(&object)
}

fn stored_outline(document: &Document, id: NodeId) -> ClosedPath {
    let record = document.generated(id).expect("a generated object").record;
    match ParamValue::from_stored(&record.params["outline"]).expect("the outline is stored") {
        ParamValue::Path(path) => path,
        other => panic!("the outline is a path, got {other:?}"),
    }
}

fn vertices(path: &ClosedPath) -> Vec<(f64, f64)> {
    let mut points = vec![(path.start.x, path.start.y)];
    points.extend(path.segments.iter().map(|c| (c.to.x, c.to.y)));
    points
}

fn width_and_height(points: &[(f64, f64)]) -> (f64, f64) {
    let xs = points.iter().map(|p| p.0);
    let ys = points.iter().map(|p| p.1);
    let (x0, x1) = (
        xs.clone().fold(f64::MAX, f64::min),
        xs.fold(f64::MIN, f64::max),
    );
    let (y0, y1) = (
        ys.clone().fold(f64::MAX, f64::min),
        ys.fold(f64::MIN, f64::max),
    );
    (x1 - x0, y1 - y0)
}

fn assert_refused(result: Result<impl std::fmt::Debug, SourceRefusal>, expected: &str) {
    match result {
        Ok(value) => panic!("expected a refusal, got {value:?}"),
        Err(refusal) => assert_eq!(refusal.to_string(), expected),
    }
}

// ---------- criterion 1: create, placement, source, one step ----------

#[test]
fn criterion_1_box_from_path_builds_a_path_box_from_one_closed_outline_in_one_step() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let mut minter = AnchorIdMinter::new(1);
    let source = drawn_polygon(
        &document,
        &mut minter,
        &[
            (0.0, 30.0),
            (20.0, 0.0),
            (80.0, 0.0),
            (100.0, 30.0),
            (80.0, 60.0),
            (20.0, 60.0),
        ],
    );
    let steps_before = document.own_step_count();
    let made = instance_source::place_from_source(&document, &settings, PATH_BOX, source)
        .expect("one closed coffin outline builds a path box");
    assert_eq!(document.own_step_count(), steps_before + 1, "one undo step");
    let record = document
        .generated(made)
        .expect("the box is a generated object")
        .record;
    assert_eq!(record.kind, PATH_BOX, "the kind is the path box");
}

#[test]
fn criterion_1_the_box_lies_right_of_the_source_by_gap_with_top_edges_aligned() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let source = drawn_rect(&document, 10.0, 20.0, 60.0, 40.0);
    let made = instance_source::place_from_source(&document, &settings, PATH_BOX, source).unwrap();
    let gap = match ParamValue::from_stored(&document.generated(made).unwrap().record.params["gap"])
        .expect("the gap is stored")
    {
        ParamValue::Length(length) => length.as_mm(),
        other => panic!("the gap is a length, got {other:?}"),
    };
    let (source_min, source_max) = bounds_of(&document, source);
    let (box_min, _) = bounds_of(&document, made);
    assert!(
        (box_min.x - (source_max.x + gap)).abs() < 0.01,
        "the layout starts Gap right of the source: {} vs {}",
        box_min.x,
        source_max.x + gap
    );
    assert!(
        (box_min.y - source_min.y).abs() < 0.01,
        "the top edges are aligned"
    );
}

#[test]
fn criterion_1_the_source_stays_untouched_and_on_the_same_layer() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let source = drawn_rect(&document, 10.0, 20.0, 60.0, 40.0);
    let outline_before = instance_source::outline_of(&document.object(source).unwrap());
    let layer_before = document.parent_node(source);
    let made = instance_source::place_from_source(&document, &settings, PATH_BOX, source).unwrap();
    assert_eq!(
        instance_source::outline_of(&document.object(source).unwrap()),
        outline_before,
        "the source outline is unchanged"
    );
    assert_eq!(
        document.parent_node(made),
        layer_before,
        "the box is in the source's layer"
    );
    assert_eq!(
        document.parent_node(source),
        layer_before,
        "the source did not move layer"
    );
}

// ---------- criterion 1: refusals build nothing ----------

#[test]
fn criterion_1_an_open_path_is_refused_with_nothing_changed() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let mut minter = AnchorIdMinter::new(1);
    let open = drawn_open(&document, &mut minter);
    let steps = document.own_step_count();
    assert_refused(
        instance_source::place_from_source(&document, &settings, PATH_BOX, open),
        "Select one closed outline. Nothing was changed.",
    );
    assert_eq!(document.own_step_count(), steps, "nothing was written");
}

#[test]
fn criterion_1_a_self_crossing_outline_is_refused_with_nothing_changed() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let mut minter = AnchorIdMinter::new(1);
    let bow = drawn_polygon(
        &document,
        &mut minter,
        &[(0.0, 0.0), (60.0, 40.0), (60.0, 0.0), (0.0, 40.0)],
    );
    let steps = document.own_step_count();
    assert_refused(
        instance_source::place_from_source(&document, &settings, PATH_BOX, bow),
        "The outline crosses itself. Nothing was changed.",
    );
    assert_eq!(document.own_step_count(), steps, "nothing was written");
}

#[test]
fn criterion_1_an_outline_too_small_for_three_mm_walls_is_refused_with_nothing_changed() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let small = drawn_rect(&document, 0.0, 0.0, 10.0, 10.0);
    let steps = document.own_step_count();
    assert_refused(
        instance_source::place_from_source(&document, &settings, PATH_BOX, small),
        "The outline is too small for a box with 3 mm walls. Nothing was changed.",
    );
    assert_eq!(document.own_step_count(), steps, "nothing was written");
}

#[test]
fn criterion_1_a_curved_outline_is_refused_in_slice_1_with_nothing_changed() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let mut minter = AnchorIdMinter::new(1);
    let curved = drawn_curved(&document, &mut minter);
    let steps = document.own_step_count();
    assert_refused(
        instance_source::place_from_source(&document, &settings, PATH_BOX, curved),
        "This outline has curves. Box from path builds straight outlines only so far. Nothing was changed.",
    );
    assert_eq!(document.own_step_count(), steps, "nothing was written");
}

// ---------- criterion 8: Replace outline ----------

#[test]
fn criterion_8_replace_outline_takes_a_copy_of_the_new_outline_in_one_step_and_keeps_the_source() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let first = drawn_rect(&document, 0.0, 0.0, 60.0, 40.0);
    let target = instance_source::place_from_source(&document, &settings, PATH_BOX, first).unwrap();
    let second = drawn_rect(&document, 200.0, 0.0, 120.0, 80.0);
    let steps = document.own_step_count();
    instance_source::replace_outline(&document, &settings, target, second)
        .expect("a closed outline replaces the stored one");
    assert_eq!(document.own_step_count(), steps + 1, "one step");
    let (w, h) = width_and_height(&vertices(&stored_outline(&document, target)));
    assert!(
        (w - 120.0).abs() < 1e-6 && (h - 80.0).abs() < 1e-6,
        "the stored copy is the new outline"
    );
    let (sw, sh) = width_and_height(&vertices(
        &instance_source::outline_of(&document.object(second).unwrap()).unwrap(),
    ));
    assert!(
        (sw - 120.0).abs() < 1e-6 && (sh - 80.0).abs() < 1e-6,
        "the source is untouched"
    );
}

#[test]
fn criterion_8_replace_outline_with_a_curved_source_is_refused_and_the_box_keeps_its_outline() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let first = drawn_rect(&document, 0.0, 0.0, 60.0, 40.0);
    let target = instance_source::place_from_source(&document, &settings, PATH_BOX, first).unwrap();
    let before = stored_outline(&document, target);
    let mut minter = AnchorIdMinter::new(7);
    let curved = drawn_curved(&document, &mut minter);
    let steps = document.own_step_count();
    let refused = instance_source::replace_outline(&document, &settings, target, curved);
    assert!(refused.is_err(), "a curved outline is refused");
    assert_eq!(document.own_step_count(), steps, "nothing was written");
    assert_eq!(
        stored_outline(&document, target),
        before,
        "the stored outline is kept"
    );
}
