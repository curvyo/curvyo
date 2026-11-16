//! Tests of the fixture triangles.

use super::*;

fn view() -> ViewTransform {
    ViewTransform::identity()
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> [Point; 4] {
    [
        Point::new(x0, y0),
        Point::new(x1, y0),
        Point::new(x1, y1),
        Point::new(x0, y1),
    ]
}

#[test]
fn a_clamp_hatch_stays_inside_its_rectangle() {
    let lines = hatch(rect(10.0, 10.0, 50.0, 30.0), 3.0);
    assert_eq!(lines.len(), 14);
    for (a, b) in lines {
        for p in [a, b] {
            assert!((10.0 - 1e-9..=50.0 + 1e-9).contains(&p.x), "{p:?}");
            assert!((10.0 - 1e-9..=30.0 + 1e-9).contains(&p.y), "{p:?}");
        }
    }
}

#[test]
fn faded_minor_lines_draw_nothing_and_major_lines_stay() {
    let grid = GridLook {
        minor: vec![(Point::new(0.0, 0.0), Point::new(0.0, 10.0))],
        major: vec![(Point::new(5.0, 0.0), Point::new(5.0, 10.0))],
        minor_alpha: 0.0,
        tapes: Vec::new(),
    };
    let look = FixtureLook {
        grid: Some(grid),
        ..FixtureLook::default()
    };
    assert_eq!(build_fixture_look(&look, view(), 1.0).triangles.len(), 6);
}

#[test]
fn a_board_draws_its_outline_before_its_casing_and_fill() {
    let look = FixtureLook {
        boards: vec![BoardLook {
            corners: rect(0.0, 0.0, 100.0, 60.0),
            slots: Vec::new(),
            engraved: false,
        }],
        ..FixtureLook::default()
    };
    let list = build_fixture_look(&look, view(), 1.0);
    assert_eq!(list.triangles.first().unwrap().color, INK);
    assert_eq!(list.triangles.last().unwrap().color, BOARD);
}

#[test]
fn an_engraved_board_takes_the_canvas_ground() {
    let look = FixtureLook {
        boards: vec![BoardLook {
            corners: rect(0.0, 0.0, 100.0, 60.0),
            slots: Vec::new(),
            engraved: true,
        }],
        ..FixtureLook::default()
    };
    let list = build_fixture_look(&look, view(), 1.0);
    assert_eq!(list.triangles.first().unwrap().color, INK);
    assert_eq!(list.triangles.last().unwrap().color, theme::CANVAS_BG);
}

#[test]
fn a_screw_draws_two_ink_rings_with_no_fill_and_drops_a_tiny_thread() {
    let screw = ScrewLook {
        centre: Point::new(50.0, 50.0),
        head_mm: 10.0,
        thread_mm: 6.0,
    };
    let look = FixtureLook {
        screws: vec![screw],
        ..FixtureLook::default()
    };
    let list = build_fixture_look(&look, view(), 1.0);
    assert_eq!(list.triangles.first().unwrap().color, INK);
    assert!(
        list.triangles
            .iter()
            .all(|v| v.color == INK || v.color == theme::SELECTION_CASING)
    );
    let tiny = FixtureLook {
        screws: vec![ScrewLook {
            thread_mm: 2.0,
            ..screw
        }],
        ..FixtureLook::default()
    };
    let both = list.triangles.len();
    assert_eq!(
        build_fixture_look(&tiny, view(), 1.0).triangles.len(),
        both / 2
    );
}

#[test]
fn a_pin_draws_its_ring_before_its_opaque_ground_under_its_ink_marks() {
    let look = FixtureLook {
        pins: vec![PinLook {
            centre: Point::new(20.0, 20.0),
            diameter_mm: 6.0,
        }],
        marks: vec![(Point::new(18.0, 20.0), Point::new(22.0, 20.0))],
        ..FixtureLook::default()
    };
    let list = build_fixture_look(&look, view(), 1.0);
    assert_eq!(
        list.triangles.first().unwrap().color,
        INK,
        "the mark on top"
    );
    assert_eq!(
        list.triangles.last().unwrap().color,
        BOARD,
        "the ground last"
    );
}

#[test]
fn a_thin_stop_leg_is_widened_to_two_pixels() {
    let leg = widened(rect(0.0, 0.0, 60.0, 0.5), 2.0);
    assert!((leg[1].vector_to(leg[2]).length() - 2.0).abs() < 1e-9);
}
