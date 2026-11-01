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
        // (broken around the text, see the gap test below).
        let (a, _) = marks.lines[2];
        let (_, b) = marks.lines[marks.lines.len() - 1];
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
    // Below the line now; along it the text stays between the extension lines, clear of the
    // arrowhead (half width 25.5px with its gap, 8px arrow; 0066 AC49 supersedes the text
    // beyond the extension line), and the line breaks around it.
    let x = 50.0 - (25.5 + 8.0) / 4.0;
    assert!(
        close(marks.text.x, x) && close(marks.text.y, 30.0),
        "{:?}",
        marks.text
    );
    let (a, b) = marks.lines[2];
    assert!(close(a.y, 30.0) && close(a.x, 0.0) && close(b.x, x - 25.5 / 4.0));
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
    let scale = 2.0;
    let marks = &layout_dimensions(&[item], view(scale))[0];
    assert_eq!(marks.arrows.len(), 1);
    assert!(close(marks.arrows[0].tip.x, 25.0) && close(marks.arrows[0].back.x, 1.0));
    // 6px clear of the rim plus half the 7-character text box (28.5px), at scale 2.
    assert!(close(marks.text.x, 25.0 + (6.0 + 28.5) / scale));
}

#[test]
fn a_radius_text_box_never_lies_on_the_circle() {
    // The slot cap of the review: R 3.44 mm, small on screen, leader up and right.
    for scale in [1.0, 4.0] {
        let radius = 3.44;
        let item = DimensionItem {
            geometry: DimensionGeometry::Radius {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(radius),
                toward: Vec2::new(1.0, -1.0),
            },
            label: None,
            text_chars: "R 3.44 mm".chars().count(),
        };
        let marks = &layout_dimensions(&[item], view(scale))[0];
        let (hw, hh) = (f64::midpoint(7.0 * 9.0, 8.0) / scale, 8.0 / scale);
        // The box corner nearest the centre stays outside the circle.
        let nearest = Point::new(marks.text.x - hw, marks.text.y + hh);
        assert!(
            nearest.x.hypot(nearest.y) > radius,
            "{scale}: {:?} inside R {radius}",
            marks.text
        );
    }
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
    // The text sits 6px plus its half height (8px) past the rim; the line stops 12px short of
    // its centre (half height plus the 4px gap).
    assert!(close(marks.text.y, -25.0 - 14.0));
    assert!(close(marks.lines[0].0.y, 25.0) && close(marks.lines[0].1.y, -39.0 + 12.0));
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

/// Whether a segment passes through the box of half sizes `w` x `h` around `c` (sampled).
fn crosses_box(segment: (Point, Point), centre: Point, half_w: f64, half_h: f64) -> bool {
    (0..=200).any(|step| {
        let t = f64::from(step) / 200.0;
        let x = segment.0.x + (segment.1.x - segment.0.x) * t;
        let y = segment.0.y + (segment.1.y - segment.0.y) * t;
        (x - centre.x).abs() < half_w - TOLERANCE_PX && (y - centre.y).abs() < half_h - TOLERANCE_PX
    })
}

#[test]
fn no_dimension_line_runs_under_its_text() {
    let scale = 4.0;
    let items = [
        linear(Point::new(0.0, 0.0), Point::new(50.0, 0.0), None),
        linear(
            Point::new(0.0, 0.0),
            Point::new(50.0, 0.0),
            Some(Point::new(70.0, 30.0)),
        ),
        DimensionItem {
            geometry: DimensionGeometry::Diameter {
                center: Point::new(0.0, 0.0),
                radius: Length::from_mm(25.0),
                toward: Vec2::new(0.0, -1.0),
            },
            label: Some(Point::new(0.0, 0.0)),
            text_chars: 7,
        },
    ];
    for (item, marks) in items.iter().zip(layout_dimensions(&items, view(scale))) {
        // The text box (7px per character plus 8px padding, 16px high) and a 2px gap around it.
        let width_px = if item.text_chars == 5 { 43.0 } else { 57.0 };
        let half_w = (width_px / 2.0 + 2.0) / scale;
        let half_h = (16.0 / 2.0 + 2.0) / scale;
        for line in &marks.lines {
            assert!(
                !crosses_box(*line, marks.text, half_w, half_h),
                "{item:?} {line:?}"
            );
        }
    }
}

#[test]
fn the_dimension_line_breaks_around_a_centred_text_and_still_reaches_both_arrows() {
    let scale = 4.0;
    let marks = &layout_dimensions(
        &[linear(Point::new(0.0, 0.0), Point::new(50.0, 0.0), None)],
        view(scale),
    )[0];
    let dimension: Vec<_> = marks.lines[2..].to_vec();
    assert_eq!(dimension.len(), 2, "{dimension:?}");
    // Left part from the first arrow to the gap, right part from the gap to the second arrow.
    // Half of the 43px text box (5 characters) plus the 4px gap.
    let half_gap = (21.5 + 4.0) / scale;
    assert!(close(dimension[0].0.x, 0.0) && close(dimension[0].1.x, 25.0 - half_gap));
    assert!(close(dimension[1].0.x, 25.0 + half_gap) && close(dimension[1].1.x, 50.0));
}

/// A horizontal distance (0066 AC45): the dimension line runs along X beyond the outer point,
/// both extension lines stand perpendicular to it, the arrows point along X.
#[test]
fn a_projected_dimension_runs_along_its_axis_with_perpendicular_extension_lines() {
    let scale = 4.0;
    let from = Point::new(0.0, 0.0);
    let to = Point::new(30.0, -20.0);
    let item = DimensionItem {
        geometry: DimensionGeometry::Projected {
            from,
            to,
            along: Vec2::new(1.0, 0.0),
        },
        label: None,
        text_chars: 7,
    };
    let marks = &layout_dimensions(&[item], view(scale))[0];
    // Extension lines: vertical, from 4px off each point to 4px past the line 24px above `to`.
    let level = -20.0 - 24.0 / scale;
    for (base, (start, end)) in [from, to].into_iter().zip(&marks.lines[..2]) {
        assert!(
            close(start.x, base.x) && close(end.x, base.x),
            "{start:?} {end:?}"
        );
        assert!(close(start.y, base.y - 4.0 / scale), "{start:?}");
        assert!(close(end.y, level - 4.0 / scale), "{end:?}");
    }
    // The dimension line (broken around the text) lies on that level, arrows along X.
    for (a, b) in &marks.lines[2..] {
        assert!(close(a.y, level) && close(b.y, level), "{a:?} {b:?}");
    }
    assert_eq!(marks.arrows.len(), 2);
    assert!(close(marks.arrows[0].tip.x, 0.0) && close(marks.arrows[1].tip.x, 30.0));
    assert!(close(marks.arrows[0].back.x.abs(), 1.0) && close(marks.arrows[0].back.y, 0.0));
    assert!(close(marks.text.x, 15.0) && close(marks.text.y, level));
}

/// A vertical distance stands left of the leftmost point; a dragged label sets its level.
#[test]
fn a_vertical_projected_dimension_stands_left_and_follows_its_label() {
    let item = |label| DimensionItem {
        geometry: DimensionGeometry::Projected {
            from: Point::new(10.0, 0.0),
            to: Point::new(0.0, -100.0),
            along: Vec2::new(0.0, -1.0),
        },
        label,
        text_chars: 7,
    };
    let marks = &layout_dimensions(&[item(None)], view(1.0))[0];
    assert!(
        close(marks.text.x, -24.0) && close(marks.text.y, -50.0),
        "{:?}",
        marks.text
    );
    // Dragged 5px from the end, the text keeps clear of the arrowhead (12px box, 8px arrow;
    // 0066 AC49).
    let dragged = &layout_dimensions(&[item(Some(Point::new(40.0, -5.0)))], view(1.0))[0];
    assert!(
        close(dragged.text.x, 40.0) && close(dragged.text.y, -20.0),
        "{:?}",
        dragged.text
    );
    for arrow in &dragged.arrows {
        assert!(close(arrow.tip.x, 40.0));
    }
}
