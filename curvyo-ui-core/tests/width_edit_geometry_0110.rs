//! Mouse width handles use distance normal to the centre line.
#![allow(clippy::unwrap_used)]
use curvyo_document_core::{Length, Point, Vec2};
use curvyo_geometry_core::PathMeasure;
use curvyo_ui_core::{width_handle, width_multiplier_at_handle, width_position_at};

fn line() -> PathMeasure {
    PathMeasure::new(
        &[
            (Point::new(0.0, 0.0), Vec2::ZERO, Vec2::ZERO),
            (Point::new(10.0, 0.0), Vec2::ZERO, Vec2::ZERO),
        ],
        false,
    )
}

#[test]
fn mouse_projects_to_length_and_sets_full_width_relative_to_base() {
    let path = line();
    let position = width_position_at(&path, Point::new(5.0, 2.0)).unwrap();
    assert!((position - 0.5).abs() < 1e-6);
    let point = width_handle(&path, position, Length::from_mm(2.0), 1.5).unwrap();
    assert!((point.x - 5.0).abs() < 1e-6 && (point.y - 1.5).abs() < 1e-6);
    let factor =
        width_multiplier_at_handle(&path, position, Length::from_mm(2.0), Point::new(5.0, -3.0))
            .unwrap();
    assert!((factor - 3.0).abs() < 1e-6);
}

#[test]
fn collapsed_path_or_zero_base_width_rejects_handle_input() {
    let path = PathMeasure::new(&[(Point::new(0.0, 0.0), Vec2::ZERO, Vec2::ZERO)], false);
    assert!(width_position_at(&path, Point::new(0.0, 0.0)).is_none());
    assert!(
        width_multiplier_at_handle(&line(), 0.5, Length::from_mm(0.0), Point::new(5.0, 1.0))
            .is_none()
    );
}
