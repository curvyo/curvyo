use super::*;

const TOLERANCE_PX: f64 = 1e-6;

fn view(scale: f64) -> ViewTransform {
    ViewTransform::new(scale, Point::new(0.0, 0.0))
}
fn linear(from: Point, to: Point, label: Option<Point>) -> DimensionItem {
    DimensionItem {
        geometry: DimensionGeometry::Linear { from, to },
        label,
        text_chars: 5,
    }
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= TOLERANCE_PX
}

#[test]
fn a_linear_dimension_keeps_its_offset_and_gaps_in_screen_pixels_at_every_zoom() {
    for scale in [2.0, 4.0, 16.0] {
        let from = Point::new(0.0, 0.0);
        let to = Point::new(50.0, 0.0);
        let marks = &layout_dimensions(&[linear(from, to, None)], view(scale))[0];
        // Two extension lines from 4px off the geometry to 4px past the dimension line, above.
        let (start, end) = marks.lines[0];
        assert!(close(start.y * scale, -4.0), "{scale}");
        assert!(close(end.y * scale, -28.0), "{scale}");
        // The dimension line at 24px, arrows pointing at both extension lines, text in the middle.
        let (a, b) = marks.lines[2];
        assert!(close(a.y * scale, -24.0) && close(b.y * scale, -24.0));
        assert!(close(a.x, 0.0) && close(b.x, 50.0));
        assert_eq!(marks.arrows.len(), 2);
        assert!(close(marks.arrows[0].back.x, 1.0) && close(marks.arrows[1].back.x, -1.0));
        assert!(close(marks.text.x, 25.0) && close(marks.text.y * scale, -24.0));
    }
}

#[test]
fn a_vertical_dimension_stands_left_of_its_line() {
    let marks = &layout_dimensions(
        &[linear(Point::new(0.0, 0.0), Point::new(0.0, 25.0), None)],
        view(4.0),
    )[0];
    assert!(close(marks.text.x * 4.0, -24.0) && close(marks.text.y, 12.5));
}

#[test]
fn small_spans_flip_the_arrows_then_use_ticks() {
    let flipped = &layout_dimensions(
        &[linear(Point::new(0.0, 0.0), Point::new(40.0, 0.0), None)],
        view(1.0),
    )[0];
    assert!(close(flipped.arrows[0].back.x, -1.0) && close(flipped.arrows[1].back.x, 1.0));
    assert!(flipped.text.x > 40.0, "text beside the line end");
    let ticks = &layout_dimensions(
        &[linear(Point::new(0.0, 0.0), Point::new(20.0, 0.0), None)],
        view(1.0),
    )[0];
    assert_eq!(ticks.arrows, []);
    // Two extension lines, the dimension line and two ticks.
    assert_eq!(ticks.lines.len(), 5);
}

#[test]
fn a_stored_label_sets_offset_and_text_position() {
    let marks = &layout_dimensions(
        &[linear(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Some(Point::new(70.0, 30.0)),
        )],
        view(4.0),
    )[0];
    assert!(close(marks.text.x, 70.0) && close(marks.text.y, 30.0));
    // Below the line now, and the dimension line reaches the text.
    let (a, b) = marks.lines[2];
    assert!(close(a.y, 30.0) && close(a.x, 0.0) && close(b.x, 70.0));
}

#[test]
fn unlabelled_dimensions_on_one_line_stack_into_ranks() {
    let items = [
        linear(Point::new(0.0, 0.0), Point::new(50.0, 0.0), None),
        linear(Point::new(10.0, 0.0), Point::new(80.0, 0.0), None),
        linear(Point::new(0.0, 100.0), Point::new(50.0, 100.0), None),
    ];
    let marks = layout_dimensions(&items, view(2.0));
    assert_eq!(marks.iter().map(|m| m.rank).collect::<Vec<_>>(), [0, 1, 0]);
    assert!(close(marks[1].text.y * 2.0, -44.0));
}

#[test]
fn an_overlapping_text_slides_along_its_line() {
    // Two parallel lines 2mm apart: the default texts would sit on top of each other.
    let items = [
        linear(Point::new(0.0, 0.0), Point::new(300.0, 0.0), None),
        linear(Point::new(0.0, 2.0), Point::new(300.0, 2.0), None),
    ];
    let marks = layout_dimensions(&items, view(1.0));
    assert_eq!(marks[1].rank, 0);
    assert!((marks[1].text.x - 150.0).abs() >= 7.0 * 5.0 + 8.0 - TOLERANCE_PX);
    assert!((marks[1].text.x - 150.0).abs() <= 300.0 / 3.0 + TOLERANCE_PX);
    // Without room to slide, the text moves out a rank instead.
    let short = [
        linear(Point::new(0.0, 0.0), Point::new(100.0, 0.0), None),
        linear(Point::new(0.0, 2.0), Point::new(100.0, 2.0), None),
    ];
    assert_eq!(layout_dimensions(&short, view(1.0))[1].rank, 1);
}

#[test]
fn a_radius_leader_ends_outside_the_circle_with_one_arrow_on_the_rim() {
    let item = DimensionItem {
        geometry: DimensionGeometry::Radius {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(25.0),
            toward: Vec2::new(1.0, 0.0),
        },
        label: None,
        text_chars: 7,
    };
    let marks = &layout_dimensions(&[item], view(2.0))[0];
    assert_eq!(marks.arrows.len(), 1);
    assert!(close(marks.arrows[0].tip.x, 25.0) && close(marks.arrows[0].back.x, 1.0));
    assert!(close(marks.text.x, 25.0 + 14.0));
}

#[test]
fn a_diameter_runs_through_the_centre_with_two_arrows() {
    let item = DimensionItem {
        geometry: DimensionGeometry::Diameter {
            center: Point::new(0.0, 0.0),
            radius: Length::from_mm(25.0),
            toward: Vec2::new(0.0, -1.0),
        },
        label: None,
        text_chars: 7,
    };
    let marks = &layout_dimensions(&[item], view(1.0))[0];
    assert!(close(marks.arrows[0].tip.y, 25.0) && close(marks.arrows[1].tip.y, -25.0));
    assert!(close(marks.lines[0].0.y, 25.0) && close(marks.lines[0].1.y, -53.0));
}

#[test]
fn an_angle_arc_spans_the_smaller_angle_at_forty_pixels() {
    let item = DimensionItem {
        geometry: DimensionGeometry::Angle {
            vertex: Point::new(0.0, 0.0),
            first: Vec2::new(0.0, -1.0),
            second: Vec2::new(1.0, 0.0),
        },
        label: None,
        text_chars: 3,
    };
    let marks = &layout_dimensions(&[item], view(4.0))[0];
    for arrow in &marks.arrows {
        assert!(close(
            Point::new(0.0, 0.0).vector_to(arrow.tip).length() * 4.0,
            40.0
        ));
    }
    // The text sits on the bisector of the right angle (up and right).
    assert!(marks.text.x > 0.0 && marks.text.y < 0.0);
    assert!(close(marks.text.x, -marks.text.y));
}
