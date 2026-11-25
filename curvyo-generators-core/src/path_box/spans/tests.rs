//! Tests of `spans.rs`.

use super::*;
use crate::path_box::fixtures::{bulging_square, circle, heart, shield};
use crate::shape_ops::{direction, start};

fn gap(a: Point, b: Point) -> f64 {
    libm::hypot(a.x - b.x, a.y - b.y)
}

/// Consecutive shapes meet end to start; they are tangent except where `corners` say.
fn assert_closed_and_tangent(shapes: &Shapes) {
    let n = shapes.shapes.len();
    for i in 0..n {
        let (a, b) = (&shapes.shapes[i], &shapes.shapes[(i + 1) % n]);
        assert!(gap(end(a), start(b)) < 1e-6, "shape {i} meets the next");
        let (da, db) = (direction(a, true), direction(b, false));
        let turn = libm::atan2(da.0 * db.1 - da.1 * db.0, da.0 * db.0 + da.1 * db.1);
        let at_corner = shapes.piece_of[i] != shapes.piece_of[(i + 1) % n];
        if !at_corner {
            assert!(
                turn.abs() < 1e-6,
                "shape {i} is tangent to the next: {turn}"
            );
        }
    }
}

#[test]
fn a_heart_is_arcs_with_two_corners_and_two_pieces() {
    let built = curve_shapes(&heart(), 25.0, (3.0, 8.0));
    assert_eq!(built.corners.len(), 2);
    assert!(built.shapes.iter().all(|s| matches!(s, Shape::Bend { .. })));
    let mut pieces = built.piece_of.clone();
    pieces.dedup();
    assert_eq!(pieces.len(), 2);
    assert_closed_and_tangent(&built);
    assert!(gap(built.corners[0].at, start(&built.shapes[0])) < 1e-9);
}

#[test]
fn a_shield_is_three_straights_and_arcs_in_two_pieces() {
    let built = curve_shapes(&shield(), 25.0, (3.0, 8.0));
    assert_eq!(built.corners.len(), 2);
    let straights = built
        .shapes
        .iter()
        .filter(|s| matches!(s, Shape::Straight { .. }))
        .count();
    assert_eq!(straights, 3);
    let mut pieces = built.piece_of.clone();
    pieces.dedup();
    assert_eq!(pieces.len(), 2);
    assert_closed_and_tangent(&built);
}

#[test]
fn a_circle_is_one_loop_of_arcs_as_long_as_its_perimeter() {
    let built = curve_shapes(&circle(30.0), 25.0, (3.0, 8.0));
    assert_eq!(built.corners, []);
    let total: f64 = built.shapes.iter().map(length).sum();
    assert!(
        (total - std::f64::consts::TAU * 30.0).abs() < 0.05,
        "{total}"
    );
    assert_closed_and_tangent(&built);
}

/// ADR 0083 §13.5, criterion 6: the neutral length of the shapes is the length of the stored
/// cubics plus half a wall thickness times the turn the shapes walk, within 0.1 mm.
#[test]
fn the_neutral_length_is_the_cubic_length_plus_half_a_thickness_times_the_turn() {
    use crate::footprint::Footprint;
    use crate::path_box::check::triples;
    use curvyo_geometry_core::PathMeasure;
    let wall = 3.0;
    for (name, path) in [
        ("circle", circle(30.0)),
        ("ellipse", crate::path_box::fixtures::ellipse(45.0, 30.0)),
        ("shield", shield()),
        ("heart", heart()),
    ] {
        let built = curve_shapes(&path, 25.0, (wall, 8.0));
        let footprint = Footprint::from_shapes(&built.shapes, wall, wall);
        let measure = PathMeasure::new(&triples(&path), true).length().as_mm();
        let corner_turn: f64 = built.corners.iter().map(|c| c.turn).sum();
        let expected = measure + wall / 2.0 * (std::f64::consts::TAU - corner_turn);
        assert!(
            (footprint.total() - expected).abs() < 0.1,
            "{name}: {} against {expected}",
            footprint.total()
        );
    }
}

#[test]
fn a_kink_between_two_arcs_is_a_fillet_tangent_to_both() {
    let built = curve_shapes(&bulging_square(35.0), 25.0, (3.0, 8.0));
    assert_eq!(built.corners, []);
    assert_eq!(built.fillets.len(), 4);
    assert_closed_and_tangent(&built);
    for &(index, radius) in &built.fillets {
        assert!(
            radius >= 6.0 - 1e-9,
            "at least twice the wall thickness: {radius}"
        );
        assert!(matches!(built.shapes[index], Shape::Bend { .. }));
    }
}
