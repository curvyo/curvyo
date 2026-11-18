//! `specs/0124-gradient-fill` criterion 15 through the gesture results: a resize, a skew, a group
//! stretch and a flip carry the gradient so the picture inside the shape does not change.

use curvyo_document_core::{
    AnchorId, Angle, AxisRatio, BoxPoint, Document, FillPaint, GradientGeometry, NewAnchor, NodeId,
    ObjectSnapshot, PlacedGradient, Point, RectBounds, StyleEdit, Vec2,
};

use crate::ResizeDirection;
use crate::flip::{FlipAxis, FlipOutcome, flip_selection};
use crate::group_transform::{GroupMap, map_all};
use crate::object_selection::ObjectSelection;
use crate::oriented_box::oriented_bounds;
use crate::transform_drag::{ResizeOptions, ScaleModes, resize_by_local_delta, skew_by_angle};
use crate::transform_handle_layout::Side;
use crate::transform_math::ExtentSigns;

fn slanted() -> GradientGeometry {
    GradientGeometry::Linear {
        start: BoxPoint::new(0.1, 0.2).unwrap(),
        end: BoxPoint::new(0.9, 0.7).unwrap(),
    }
}

fn elliptical() -> GradientGeometry {
    GradientGeometry::Radial {
        centre: BoxPoint::new(0.4, 0.6).unwrap(),
        radius: BoxPoint::new(0.9, 0.3).unwrap(),
        ratio: AxisRatio::new(0.6).unwrap(),
    }
}

fn with_gradient(document: &Document, id: NodeId, geometry: GradientGeometry) {
    document
        .edit_style(&[id], &StyleEdit::FillPaint(FillPaint::Gradient))
        .unwrap();
    document
        .edit_style(&[id], &StyleEdit::GradientGeometry(geometry))
        .unwrap();
}

/// A path turned by 0.4 rad (its `rotation` register too), with `geometry`.
fn turned_path(document: &Document, geometry: GradientGeometry) -> ObjectSnapshot {
    let anchors: Vec<NewAnchor> = [(0.0, 0.0), (80.0, 0.0), (80.0, 30.0), (10.0, 45.0)]
        .iter()
        .zip(0..)
        .map(|((x, y), k)| NewAnchor::corner(AnchorId::new(1, k), Point::new(*x, *y)))
        .collect();
    let id = document.create_path(&anchors, true);
    let turned = document
        .object(id)
        .unwrap()
        .rotated(Point::new(20.0, 10.0), Angle::from_radians(0.4));
    document.rotate_object(&turned).unwrap();
    with_gradient(document, id, geometry);
    document.object(id).unwrap()
}

fn placed(object: &ObjectSnapshot) -> PlacedGradient {
    PlacedGradient::new(
        object.style().fill.gradient.as_ref().expect("a gradient"),
        &object.gradient_box().expect("a box"),
    )
}

/// The anchors of `object` and the midpoints of each pair, by anchor id: an affine map takes
/// these of `before` to these of `after`.
fn probes(object: &ObjectSnapshot) -> Vec<(AnchorId, AnchorId, Point)> {
    let ObjectSnapshot::Path(path) = object else {
        panic!("a path")
    };
    let anchors: Vec<_> = path.all_anchors().collect();
    let mut out = Vec::new();
    for a in &anchors {
        for b in &anchors {
            out.push((
                a.id,
                b.id,
                Point::new(
                    f64::midpoint(a.point.x, b.point.x),
                    f64::midpoint(a.point.y, b.point.y),
                ),
            ));
        }
    }
    out
}

fn assert_same_picture(before: &ObjectSnapshot, after: &ObjectSnapshot) {
    let (pb, pa) = (placed(before), placed(after));
    let after_probes = probes(after);
    for (a, b, p) in probes(before) {
        let (_, _, q) = after_probes
            .iter()
            .find(|(x, y, _)| *x == a && *y == b)
            .unwrap();
        let (tb, ta) = (pb.t_at(p), pa.t_at(*q));
        assert!((tb - ta).abs() < 1e-9, "{p:?} -> {q:?}: {tb} {ta}");
    }
}

#[test]
fn a_resize_of_a_turned_path_carries_the_picture() {
    let document = Document::new(1);
    for geometry in [slanted(), elliptical()] {
        let start = turned_path(&document, geometry);
        let options = ResizeOptions {
            shift: false,
            ctrl: false,
            modes: ScaleModes::default(),
            converted: None,
            prior: ExtentSigns::default(),
        };
        let result = resize_by_local_delta(
            &start,
            &oriented_bounds(&start),
            ResizeDirection::Se,
            Vec2::new(40.0, -12.0),
            options,
        );
        assert_ne!(result.style().fill.gradient, None);
        assert_same_picture(&start, &result);
    }
}

#[test]
fn a_skew_of_a_turned_path_carries_the_picture() {
    let document = Document::new(1);
    for geometry in [slanted(), elliptical()] {
        let start = turned_path(&document, geometry);
        let result = skew_by_angle(
            &start,
            &oriented_bounds(&start),
            (Side::Top, false),
            Angle::from_radians(0.5),
            None,
        );
        assert_ne!(result.style().fill.gradient, start.style().fill.gradient);
        assert_same_picture(&start, &result);
    }
}

#[test]
fn a_group_stretch_and_shear_along_the_page_carry_the_picture() {
    let document = Document::new(1);
    let start = turned_path(&document, elliptical());
    for map in [
        GroupMap::Scale {
            pivot: Point::new(5.0, 5.0),
            sx: 1.7,
            sy: -0.6,
        },
        GroupMap::Shear {
            fixed: Point::new(0.0, 20.0),
            ku: 0.4,
            kv: 0.0,
        },
    ] {
        let results = map_all(
            std::slice::from_ref(&start),
            &[None],
            &map,
            ScaleModes::default(),
        );
        assert_same_picture(&start, &results[0]);
    }
}

#[test]
fn a_flip_of_a_rectangle_mirrors_its_gradient_in_one_step() {
    let document = Document::new(1);
    let id = document.create_rect(RectBounds {
        origin: Point::new(0.0, 0.0),
        width: curvyo_document_core::Length::from_mm(60.0),
        height: curvyo_document_core::Length::from_mm(20.0),
    });
    with_gradient(&document, id, GradientGeometry::DEFAULT_LINEAR);
    let mut selection = ObjectSelection::new();
    selection.select_single(id);
    let steps = document.own_step_count();
    let objects = vec![document.object(id).unwrap()];
    assert_eq!(
        flip_selection(&document, &objects, &selection, FlipAxis::Horizontal),
        FlipOutcome::Flipped,
        "a symmetric rectangle with a gradient does not look the same flipped"
    );
    assert_eq!(document.own_step_count(), steps + 1);
    let geometry = document
        .object(id)
        .unwrap()
        .style()
        .fill
        .gradient
        .clone()
        .unwrap()
        .geometry;
    assert_eq!(
        geometry,
        GradientGeometry::Linear {
            start: BoxPoint::new(1.0, 0.5).unwrap(),
            end: BoxPoint::new(0.0, 0.5).unwrap(),
        }
    );
}
