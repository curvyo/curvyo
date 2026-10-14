//! Tests of [`super::ShapePlacement`], with the examples of `specs/0052-click-to-create-shapes/`
//! criteria 12 to 26 as given.

use curvyo_document_core::{InnerRatio, Point, PointCount, Shape};

use super::*;
use crate::{EllipseTool, PolyStarMode, PolygonStarTool, RectangleTool};

const TOL: f64 = 1e-9;
const MM: DisplayUnit = DisplayUnit::Mm;
const A: Point = Point::new(100.0, 50.0);
const POINTER: Point = Point::new(130.0, 40.0);

fn settings() -> PolyStarSettings {
    PolygonStarTool::new().settings()
}

/// The live shape of `kind` from `a` to `b`, as the session computes it.
fn live(kind: PlacementKind, a: Point, b: Point, modifiers: Modifiers) -> Option<CreatePreview> {
    match kind {
        PlacementKind::Rect => RectangleTool::shape_for(a, b, modifiers),
        PlacementKind::Ellipse => EllipseTool::shape_for(a, b, modifiers),
        PlacementKind::PolyStar => PolygonStarTool::new().shape_for(a, b, modifiers),
    }
}

/// A placement of `kind` at `a` that follows the pointer to `b` with `modifiers`.
fn following(
    kind: PlacementKind,
    a: Point,
    b: Point,
    modifiers: Modifiers,
) -> (ShapePlacement, Option<CreatePreview>) {
    let mut placement = ShapePlacement::new(kind, a, Modifiers::NONE);
    placement.follow(b, modifiers);
    (placement, live(kind, a, b, modifiers))
}

/// Types `steps` (field, text) into the placement, freezing it at the first.
fn typed(
    mut placement: ShapePlacement,
    live: Option<CreatePreview>,
    steps: &[(PlacementField, &str)],
) -> ShapePlacement {
    for (field, text) in steps {
        placement.edit(*field, text, live);
    }
    placement
}

fn rect_of(placed: Placed) -> (f64, f64, f64, f64) {
    let Shape::Rect { bounds, .. } = placed.shape else {
        panic!("a rectangle");
    };
    (
        bounds.origin.x,
        bounds.origin.y,
        bounds.width.as_mm(),
        bounds.height.as_mm(),
    )
}

fn close4(got: (f64, f64, f64, f64), want: (f64, f64, f64, f64)) -> bool {
    (got.0 - want.0).abs() < TOL
        && (got.1 - want.1).abs() < TOL
        && (got.2 - want.2).abs() < TOL
        && (got.3 - want.3).abs() < TOL
}

fn ellipse_of(placed: Placed) -> (f64, f64, f64, f64) {
    let Shape::Ellipse { frame } = placed.shape else {
        panic!("an ellipse");
    };
    (
        frame.center.x,
        frame.center.y,
        frame.rx.as_mm(),
        frame.ry.as_mm(),
    )
}

fn star_frame_of(placed: Placed) -> curvyo_document_core::StarFrame {
    match placed.shape {
        Shape::Polygon { frame, .. } | Shape::Star { frame, .. } => frame,
        _ => panic!("a polygon or star"),
    }
}

use PlacementField::{Angle as AngleField, Height, Size, Width};

/// Criterion 12: the fields show the live shape's values.
#[test]
fn criterion_12_untouched_fields_show_the_live_values() {
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    assert_eq!(p.field_text(Width, l, MM), "30.0");
    assert_eq!(p.field_text(Height, l, MM), "10.0");
    assert_eq!(p.field_text(AngleField, l, MM), "0");
    // From the centre W and H are the full box.
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::new(true, false));
    assert_eq!(p.field_text(Width, l, MM), "60.0");
    // An ellipse that is not a circle: empty, with the rx × ry placeholder.
    let a = Point::new(50.0, 50.0);
    let (p, l) = following(
        PlacementKind::Ellipse,
        a,
        Point::new(80.0, 60.0),
        Modifiers::NONE,
    );
    assert_eq!(p.field_text(Size, l, MM), "");
    assert_eq!(
        p.placeholder(Size, l, MM).as_deref(),
        Some("15.0 \u{d7} 5.0")
    );
    let (p, l) = following(
        PlacementKind::Ellipse,
        a,
        Point::new(80.0, 60.0),
        Modifiers::new(false, true),
    );
    assert_eq!(p.field_text(Size, l, MM), "15.0");
    assert_eq!(p.placeholder(Size, l, MM), None);
    // The polygon: distance and angle, cm with two decimals.
    let (p, l) = following(
        PlacementKind::PolyStar,
        A,
        Point::new(100.0, 25.0),
        Modifiers::NONE,
    );
    assert_eq!(p.field_text(Size, l, DisplayUnit::Cm), "2.50");
    assert_eq!(p.field_text(AngleField, l, MM), "-90");
}

/// Criterion 14: the first edit freezes; the pointer and the modifiers change nothing.
#[test]
fn criterion_14_the_first_edit_freezes_the_placement() {
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    let mut p = typed(p, l, &[(Width, "40")]);
    assert!(p.is_frozen());
    p.follow(Point::new(0.0, 0.0), Modifiers::new(true, true));
    p.set_modifiers(Modifiers::new(true, true));
    let other = live(
        PlacementKind::Rect,
        A,
        Point::new(0.0, 0.0),
        Modifiers::NONE,
    );
    assert_eq!(p.field_text(Width, other, MM), "40");
    assert_eq!(p.field_text(Height, other, MM), "10.0");
    let placed = p.result(other, settings(), MM).expect("valid");
    assert!(close4(rect_of(placed), (100.0, 40.0, 40.0, 10.0)));
}

/// Criterion 16 (a), (b), (c).
#[test]
fn criterion_16_typed_rectangles() {
    let (start, at_freeze) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    let sized = typed(start.clone(), at_freeze, &[(Width, "40"), (Height, "20")]);
    let placed = sized.result(None, settings(), MM).expect("valid");
    assert!(close4(rect_of(placed), (100.0, 30.0, 40.0, 20.0)));
    assert!(placed.rotation.as_radians().abs() < TOL);

    let turned = typed(
        start,
        at_freeze,
        &[(Width, "40"), (Height, "20"), (AngleField, "90")],
    );
    let placed = turned.result(None, settings(), MM).expect("valid");
    assert!(close4(rect_of(placed), (90.0, 60.0, 40.0, 20.0)));
    assert!((placed.rotation.as_radians() - 90_f64.to_radians()).abs() < TOL);

    let (start, at_freeze) =
        following(PlacementKind::Rect, A, POINTER, Modifiers::new(true, false));
    let centred = typed(start, at_freeze, &[(Width, "40"), (Height, "20")]);
    assert!(close4(
        rect_of(centred.result(None, settings(), MM).expect("valid")),
        (80.0, 40.0, 40.0, 20.0)
    ));
}

/// Criterion 17: Ctrl at the freeze links W and H.
#[test]
fn criterion_17_ctrl_links_width_and_height() {
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::new(false, true));
    assert!(p.linked());
    let p = typed(p, l, &[(Width, "25")]);
    assert_eq!(p.field_text(Height, None, MM), "25");
    assert!(close4(
        rect_of(p.result(None, settings(), MM).expect("valid")),
        (100.0, 25.0, 25.0, 25.0)
    ));
    let (p, _) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    assert!(!p.linked());
}

/// Criterion 18: the circle examples, Space included.
#[test]
fn criterion_18_typed_circles() {
    let a = Point::new(50.0, 50.0);
    let b = Point::new(70.0, 80.0);
    let check =
        |m: Modifiers, space: bool, steps: &[(PlacementField, &str)], want: (f64, f64, f64)| {
            let (mut p, l) = following(PlacementKind::Ellipse, a, b, m);
            if space {
                p.toggle_diameter();
            }
            let p = typed(p, l, steps);
            let placed = p.result(None, settings(), MM).expect("valid");
            let (cx, cy, rx, ry) = ellipse_of(placed);
            assert!(
                (cx - want.0).abs() < TOL
                    && (cy - want.1).abs() < TOL
                    && (rx - want.2).abs() < TOL
                    && (ry - want.2).abs() < TOL,
                "{steps:?}: ({cx}, {cy}) r {rx} {ry}"
            );
            placed
        };
    check(Modifiers::NONE, false, &[(Size, "10")], (60.0, 60.0, 10.0));
    check(Modifiers::NONE, true, &[(Size, "10")], (55.0, 55.0, 5.0));
    check(
        Modifiers::new(true, false),
        false,
        &[(Size, "10")],
        (50.0, 50.0, 10.0),
    );
    let turned = check(
        Modifiers::NONE,
        false,
        &[(Size, "10"), (AngleField, "90")],
        (40.0, 60.0, 10.0),
    );
    assert!((turned.rotation.as_radians() - 90_f64.to_radians()).abs() < TOL);
}

/// Criterion 18: an untouched r keeps the live rx and ry.
#[test]
fn criterion_18_an_untouched_radius_keeps_the_live_ellipse() {
    let a = Point::new(50.0, 50.0);
    let (p, l) = following(
        PlacementKind::Ellipse,
        a,
        Point::new(80.0, 60.0),
        Modifiers::NONE,
    );
    let p = typed(p, l, &[(AngleField, "0")]);
    let (cx, cy, rx, ry) = ellipse_of(p.result(None, settings(), MM).expect("valid"));
    assert!(
        (cx - 65.0).abs() < TOL
            && (cy - 55.0).abs() < TOL
            && (rx - 15.0).abs() < TOL
            && (ry - 5.0).abs() < TOL
    );
}

/// Criterion 19 and the architect's M1 note: centre A, the first vertex at the angle, the
/// `rotation` register 0, the bar's mode and ratio.
#[test]
fn criterion_19_typed_polygons_and_stars_keep_rotation_zero() {
    let a = Point::new(100.0, 100.0);
    let b = Point::new(110.0, 100.0);
    let (p, l) = following(PlacementKind::PolyStar, a, b, Modifiers::NONE);
    let first = typed(p.clone(), l, &[(Size, "25")]);
    let placed = first.result(None, settings(), MM).expect("valid");
    let frame = star_frame_of(placed);
    assert!((frame.radius.as_mm() - 25.0).abs() < TOL && frame.angle.as_radians().abs() < TOL);
    assert_eq!(frame.center, a);
    assert!(placed.rotation.as_radians().abs() < TOL);

    let turned = typed(p.clone(), l, &[(Size, "25"), (AngleField, "-90")]);
    let placed = turned.result(None, settings(), MM).expect("valid");
    assert!((star_frame_of(placed).angle.as_radians() + std::f64::consts::FRAC_PI_2).abs() < TOL);
    assert!(
        placed.rotation.as_radians().abs() < TOL,
        "the angle goes into the frame"
    );

    let star = PolyStarSettings {
        mode: PolyStarMode::Star,
        point_count: PointCount::new(6).expect("valid"),
        ratio: InnerRatio::new(0.5).expect("valid"),
    };
    let Shape::Star {
        frame, inner_ratio, ..
    } = first.result(None, star, MM).expect("valid").shape
    else {
        panic!("a star");
    };
    assert!((frame.radius.as_mm() - 25.0).abs() < TOL && (inner_ratio.get() - 0.5).abs() < TOL);
}

/// Criterion 20: Space re-shows an untouched field and keeps an edited one's text.
#[test]
fn criterion_20_space_switches_radius_and_diameter() {
    let (mut p, l) = following(
        PlacementKind::Ellipse,
        A,
        Point::new(124.0, 74.0),
        Modifiers::new(false, true),
    );
    assert_eq!(p.field_text(Size, l, MM), "12.0");
    p.toggle_diameter();
    assert!(p.diameter() && !p.is_frozen());
    assert_eq!(p.field_text(Size, l, MM), "24.0");
    let mut p = typed(p, l, &[(Size, "20")]);
    p.toggle_diameter();
    p.toggle_diameter();
    assert_eq!(p.field_text(Size, None, MM), "20");
    let (.., r, _) = ellipse_of(p.result(None, settings(), MM).expect("valid"));
    assert!((r - 10.0).abs() < TOL);
    let (mut rect, _) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    rect.toggle_diameter();
    assert!(!rect.diameter());
}

/// Criterion 21: Enter with nothing edited places the live shape.
#[test]
fn criterion_21_an_untouched_enter_places_the_live_shape() {
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    let placed = p.result(l, settings(), MM).expect("valid");
    assert_eq!(placed.shape, l.expect("a live shape").shape);
    assert!(close4(rect_of(placed), (100.0, 40.0, 30.0, 10.0)));
}

/// Criterion 23: the messages, the first refused field, and the accepted spellings.
#[test]
fn criterion_23_refused_values() {
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    let refused = |steps: &[(PlacementField, &str)]| {
        typed(p.clone(), l, steps)
            .result(None, settings(), MM)
            .expect_err("refused")
    };
    let err = |field, reason| PlacementError { field, reason };
    assert_eq!(
        refused(&[(Width, "abc")]),
        err(Width, InvalidReason::NotANumber)
    );
    assert_eq!(
        refused(&[(Width, "")]),
        err(Width, InvalidReason::NotANumber)
    );
    assert_eq!(
        refused(&[(Height, "1,2,3"), (Width, "-5")]),
        err(Width, InvalidReason::NotPositive)
    );
    assert_eq!(
        refused(&[(Width, "20000000")]),
        err(Width, InvalidReason::TooLarge)
    );
    assert_eq!(
        refused(&[(AngleField, "x")]),
        err(AngleField, InvalidReason::NotANumber)
    );
    assert_eq!(
        refused(&[(Height, "0.0000001")]),
        err(Height, InvalidReason::NotPositive)
    );
    let (c, l) = following(PlacementKind::Ellipse, A, POINTER, Modifiers::NONE);
    for text in ["0", "-5"] {
        assert_eq!(
            typed(c.clone(), l, &[(Size, text)]).result(None, settings(), MM),
            Err(err(Size, InvalidReason::NotPositive))
        );
    }
    let ok = typed(
        p.clone(),
        l,
        &[
            (Width, "+40,5"),
            (Height, "\u{2212}0"),
            (Height, "20"),
            (AngleField, "720°"),
        ],
    );
    let placed = ok.result(None, settings(), MM).expect("valid");
    assert!((rect_of(placed).2 - 40.5).abs() < TOL);
    assert_eq!(p.field_problem(Width, MM), None);
    let bad = typed(p, l, &[(Width, "abc")]);
    assert_eq!(
        bad.field_problem(Width, MM),
        Some(InvalidReason::NotANumber)
    );
}

/// Criterion 15: a refused text keeps the last valid preview.
#[test]
fn criterion_15_the_preview_keeps_the_last_valid_shape() {
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    let mut p = typed(p, l, &[(Width, "40")]);
    p.refresh(None, settings(), MM);
    let valid = p.shown().expect("a preview");
    p.edit(Width, "40x", None);
    p.refresh(None, settings(), MM);
    assert_eq!(p.shown(), Some(valid));
}

/// Criterion 23: a refusal's message clears on the next edit.
#[test]
fn a_refusal_clears_on_the_next_edit() {
    let (p, l) = following(PlacementKind::Rect, A, POINTER, Modifiers::NONE);
    let mut p = typed(p, l, &[(Width, "abc")]);
    let error = p.result(None, settings(), MM).expect_err("refused");
    p.refuse(error);
    assert_eq!(p.error(), Some(error));
    p.edit(Width, "40", None);
    assert_eq!(p.error(), None);
}

/// Criterion 25: the bar's settings reach a frozen polygon/star placement.
#[test]
fn criterion_25_bar_settings_reach_a_frozen_placement() {
    let (p, l) = following(PlacementKind::PolyStar, A, POINTER, Modifiers::NONE);
    let p = typed(p, l, &[(Size, "25")]);
    let mut eleven = settings();
    eleven.point_count = PointCount::new(11).expect("valid");
    let Shape::Polygon { point_count, .. } = p.result(None, eleven, MM).expect("valid").shape
    else {
        panic!("a polygon");
    };
    assert_eq!(point_count.get(), 11);
}

/// Criterion 26 and 23: a keyboard placement grows right and down; an untouched size refuses.
#[test]
fn criterion_26_a_keyboard_placement() {
    let a = Point::new(148.5, 105.0);
    let p = ShapePlacement::new(PlacementKind::Rect, a, Modifiers::NONE);
    assert_eq!(
        p.result(None, settings(), MM),
        Err(PlacementError {
            field: Width,
            reason: InvalidReason::NotPositive
        })
    );
    let w_only = typed(p.clone(), None, &[(Width, "30")]);
    assert_eq!(
        w_only.result(None, settings(), MM),
        Err(PlacementError {
            field: Height,
            reason: InvalidReason::NotPositive
        })
    );
    let both = typed(p, None, &[(Width, "30"), (Height, "20")]);
    assert!(close4(
        rect_of(both.result(None, settings(), MM).expect("valid")),
        (148.5, 105.0, 30.0, 20.0)
    ));
    let star = ShapePlacement::new(PlacementKind::PolyStar, a, Modifiers::NONE);
    let star = typed(star, None, &[(Size, "10")]);
    assert!(
        star_frame_of(star.result(None, settings(), MM).expect("valid"))
            .angle
            .as_radians()
            .abs()
            < TOL
    );
}
