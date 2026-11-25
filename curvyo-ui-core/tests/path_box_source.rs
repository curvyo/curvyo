//! The path box from a selected outline (`specs/0149-path-box` criteria 1 and 8, ADR 0083 §2):
//! the copy, the untouched source, the placement beside it, one undo step and the refusals.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use curvyo_document_core::{Document, NewAnchor, NodeId, Point, Vec2};
use curvyo_extension_core::ParamValue;
use curvyo_ui_core::instance_source::{SourceRefusal, outline_of, place_from_source, preview};
use curvyo_ui_core::{AnchorIdMinter, ExtensionSettings};

const KIND: &str = "curvyo.generators/path-box";

fn corners(document: &Document, points: &[(f64, f64)], closed: bool) -> NodeId {
    let mut minter = AnchorIdMinter::new(1);
    let anchors: Vec<NewAnchor> = points
        .iter()
        .map(|&(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)))
        .collect();
    document.create_path(&anchors, closed)
}

fn coffin(document: &Document, dx: f64, dy: f64) -> NodeId {
    let points: Vec<(f64, f64)> = [
        (-50.0, -10.0),
        (-25.0, -30.0),
        (50.0, -20.0),
        (50.0, 20.0),
        (-25.0, 30.0),
        (-50.0, 10.0),
    ]
    .iter()
    .map(|&(x, y)| (x + dx, y + dy))
    .collect();
    corners(document, &points, true)
}

#[test]
fn a_coffin_becomes_a_box_right_of_the_source_with_the_source_untouched() {
    let document = Document::new(1);
    let source = coffin(&document, 200.0, 100.0);
    let before = document.object(source).unwrap();
    let settings = ExtensionSettings::default();
    let steps = document.own_step_count();
    let id = place_from_source(&document, &settings, KIND, source).unwrap();
    assert_eq!(document.own_step_count(), steps + 1, "one undo step");
    assert_eq!(
        document.object(source).unwrap(),
        before,
        "the source is untouched"
    );
    let snapshot = document.generated(id).unwrap();
    assert_eq!(snapshot.record.kind, KIND);
    let stored = ParamValue::from_stored(&snapshot.record.params["outline"]).unwrap();
    let outline = stored.path().unwrap();
    assert!(
        (outline.start.x + 50.0).abs() < 1e-9,
        "a copy round the origin"
    );
    // The layout starts Gap (5 mm) right of the source (right edge x = 250), tops level (y = 70).
    let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
    let outlines = snapshot.outlines();
    for subpath in outlines.iter().flatten() {
        for anchor in &subpath.anchors {
            min_x = min_x.min(anchor.point.x);
            min_y = min_y.min(anchor.point.y);
        }
    }
    assert!((min_x - 255.0).abs() < 1e-6, "{min_x}");
    assert!((min_y - 70.0).abs() < 1e-6, "{min_y}");
}

#[test]
fn the_box_lands_directly_above_the_source() {
    let document = Document::new(1);
    let below = coffin(&document, 0.0, 0.0);
    let source = coffin(&document, 300.0, 0.0);
    let above = coffin(&document, 600.0, 0.0);
    let id = place_from_source(&document, &ExtensionSettings::default(), KIND, source).unwrap();
    let order: Vec<NodeId> = document.object_ids();
    let at = |n: NodeId| order.iter().position(|o| *o == n).unwrap();
    assert!(
        at(below) < at(source) && at(source) < at(id) && at(id) < at(above),
        "{order:?}"
    );
}

#[test]
fn refusals_say_why_and_change_nothing() {
    let document = Document::new(1);
    let settings = ExtensionSettings::default();
    let open = corners(&document, &[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0)], false);
    let bow = corners(
        &document,
        &[(0.0, 0.0), (60.0, 60.0), (60.0, 0.0), (0.0, 60.0)],
        true,
    );
    let tiny = corners(
        &document,
        &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
        true,
    );
    let mut anchors = Vec::new();
    let mut minter = AnchorIdMinter::new(2);
    for (x, y, out) in [
        (0.0, 0.0, 15.0),
        (60.0, 0.0, 0.0),
        (60.0, 60.0, 0.0),
        (0.0, 60.0, 0.0),
    ] {
        let mut a = NewAnchor::corner(minter.mint(), Point::new(x, y));
        a.handle_out = Vec2::new(out, -out);
        anchors.push(a);
    }
    let curved = document.create_path(&anchors, true);
    let steps = document.own_step_count();
    let refusal = |id: NodeId| place_from_source(&document, &settings, KIND, id).unwrap_err();
    assert_eq!(refusal(open), SourceRefusal::NotOneOutline);
    assert_eq!(refusal(bow), SourceRefusal::Crosses);
    assert_eq!(refusal(tiny), SourceRefusal::TooSmall);
    assert_eq!(
        refusal(bow).to_string(),
        "The outline crosses itself. Nothing was changed."
    );
    assert_eq!(
        refusal(tiny).to_string(),
        "The outline is too small for a box with 3 mm walls. Nothing was changed."
    );
    assert_eq!(document.own_step_count(), steps, "nothing was written");
    // Slice 2 (ADR 0083 section 13.14): a curved outline is no longer refused; the box is built.
    assert!(place_from_source(&document, &settings, KIND, curved).is_ok());
}

#[test]
fn the_pick_mode_previews_marks_without_writing() {
    let document = Document::new(1);
    let source = coffin(&document, 0.0, 0.0);
    let steps = document.own_step_count();
    let generated = preview(&document, &ExtensionSettings::default(), KIND, source).unwrap();
    assert_ne!(generated.marks.on_footprint, []);
    assert_eq!(document.own_step_count(), steps);
    assert!(outline_of(&document.object(source).unwrap()).is_some());
}

/// ADR 0083 §10: a rotated source (a diamond) is copied round its box centre, the layout starts
/// Gap right of the box and the tops are level.
#[test]
fn a_rotated_source_is_centred_and_the_layout_starts_gap_right_of_its_box() {
    let document = Document::new(1);
    let source = corners(
        &document,
        &[
            (300.0, 160.0),
            (340.0, 200.0),
            (300.0, 240.0),
            (260.0, 200.0),
        ],
        true,
    );
    let id = place_from_source(&document, &ExtensionSettings::default(), KIND, source).unwrap();
    let snapshot = document.generated(id).unwrap();
    let stored = ParamValue::from_stored(&snapshot.record.params["outline"]).unwrap();
    let outline = stored.path().unwrap();
    assert!(
        outline.start.x.abs() < 1e-9 && (outline.start.y + 40.0).abs() < 1e-9,
        "the diamond's top, round the origin: {:?}",
        outline.start
    );
    let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
    for subpath in snapshot.outlines().iter().flatten() {
        for anchor in &subpath.anchors {
            min_x = min_x.min(anchor.point.x);
            min_y = min_y.min(anchor.point.y);
        }
    }
    assert!(
        (min_x - 345.0).abs() < 1e-6,
        "box right edge 340 + Gap 5: {min_x}"
    );
    assert!((min_y - 160.0).abs() < 1e-6, "tops level: {min_y}");
}

#[test]
fn replace_outline_swaps_the_stored_copy_and_leaves_the_source_alone() {
    use curvyo_ui_core::instance_source::replace_outline;
    let document = Document::new(1);
    let first = coffin(&document, 0.0, 0.0);
    let second = corners(
        &document,
        &[(500.0, 0.0), (580.0, 0.0), (580.0, 90.0), (500.0, 90.0)],
        true,
    );
    let settings = ExtensionSettings::default();
    let id = place_from_source(&document, &settings, KIND, first).unwrap();
    let before = document.object(second).unwrap();
    replace_outline(&document, &settings, id, second).unwrap();
    assert_eq!(document.object(second).unwrap(), before);
    let stored =
        ParamValue::from_stored(&document.generated(id).unwrap().record.params["outline"]).unwrap();
    let outline = stored.path().unwrap();
    assert!((outline.start.x + 40.0).abs() < 1e-9 && (outline.start.y + 45.0).abs() < 1e-9);
    let open = corners(&document, &[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0)], false);
    assert_eq!(
        replace_outline(&document, &settings, id, open),
        Err(SourceRefusal::NotOneOutline)
    );
}

/// An imported outline may close with a node on its start: that zero-length segment is dropped,
/// so it is no corner (ADR 0083 section 13.1).
#[test]
fn a_node_repeated_on_the_start_adds_no_segment_to_the_outline() {
    let document = Document::new(1);
    let mut minter = AnchorIdMinter::new(5);
    let mut anchors: Vec<NewAnchor> = [(0.0, 0.0), (60.0, 0.0), (60.0, 60.0), (0.0, 60.0)]
        .iter()
        .map(|&(x, y)| NewAnchor::corner(minter.mint(), Point::new(x, y)))
        .collect();
    anchors.push(NewAnchor::corner(minter.mint(), Point::new(0.0, 0.0)));
    let id = document.create_path(&anchors, true);
    let path = outline_of(&document.object(id).unwrap()).unwrap();
    assert_eq!(path.segments.len(), 4);
}

/// The Corner angle row says how many corners it makes: "6 corners", and "no corners" for none.
#[test]
fn the_corner_angle_row_says_no_corners_for_zero() {
    let document = Document::new(1);
    let source = coffin(&document, 0.0, 0.0);
    let settings = ExtensionSettings::default();
    let target = place_from_source(&document, &settings, KIND, source).unwrap();
    let state = |document: &Document| {
        let snapshot = document.generated(target).unwrap();
        curvyo_ui_core::instance::view(&settings, &snapshot)
            .rows
            .into_iter()
            .find(|row| row.param.key == "corner_angle")
            .and_then(|row| row.state)
    };
    assert_eq!(state(&document).as_deref(), Some("6 corners"));
    curvyo_ui_core::instance::change_value(
        &document,
        &settings,
        target,
        "corner_angle",
        &ParamValue::Angle(curvyo_extension_core::Degrees(90.0)),
    )
    .unwrap();
    assert_eq!(state(&document).as_deref(), Some("no corners"));
}
