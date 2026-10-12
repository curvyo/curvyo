//! The input record, the pressure rules and the Pencil's stroke state machine of
//! `specs/0032-pen-tablet-input` (criteria 1, 3, 4, 5, 9, 12 and 14).

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use curvyo_document_core::{Angle, Length, Point};
use curvyo_ui_core::{
    PencilSettings, PencilTool, PointerKind, PointerSample, PressureCurve, StrokeShape,
    smooth_pressures, stroke_has_pressure,
};

const PRIMARY: u32 = 1;

fn sample(kind: PointerKind, x: f64, pressure: f64) -> PointerSample {
    PointerSample {
        position: Point::new(x, 10.0),
        kind,
        pressure,
        tilt_x: Angle::from_radians(0.0),
        tilt_y: Angle::from_radians(0.0),
        eraser: false,
        time_ms: x,
    }
}

fn pen(x: f64, pressure: f64) -> PointerSample {
    sample(PointerKind::Pen, x, pressure)
}

// Criterion 1: the record carries what the webview delivered.

#[test]
fn criterion_1_the_record_carries_every_value_as_delivered() {
    let record = PointerSample::from_values("pen", 1 | 32, [12.5, 7.25, 0.62, 12.0, -3.0, 1234.5]);
    assert_eq!(record.position, Point::new(12.5, 7.25));
    assert_eq!(record.kind, PointerKind::Pen);
    assert!((record.pressure - 0.62).abs() < 1e-12);
    assert!((record.tilt_x.as_radians() - 12.0_f64.to_radians()).abs() < 1e-12);
    assert!((record.tilt_y.as_radians() + 3.0_f64.to_radians()).abs() < 1e-12);
    assert!(record.eraser);
    assert!((record.time_ms - 1234.5).abs() < 1e-12);
    assert!(!PointerSample::from_values("pen", 1, [0.0; 6]).eraser);
    assert_eq!(
        PointerSample::from_values("mouse", 1, [0.0; 6]).kind,
        PointerKind::Mouse
    );
    assert_eq!(
        PointerSample::from_values("touch", 1, [0.0; 6]).kind,
        PointerKind::Touch
    );
    // An unknown or empty type (some webviews) is treated as a mouse.
    assert_eq!(
        PointerSample::from_values("", 1, [0.0; 6]).kind,
        PointerKind::Mouse
    );
}

// Criterion 3: when a stroke has pressure.

#[test]
fn criterion_3_a_pen_with_varying_pressure_has_pressure() {
    let samples: Vec<_> = (1..=9)
        .map(|i| pen(f64::from(i), f64::from(i) / 10.0))
        .collect();
    assert!(stroke_has_pressure(&samples));
}

#[test]
fn criterion_3_a_pen_at_constant_one_half_has_none() {
    let samples: Vec<_> = (0..10).map(|i| pen(f64::from(i), 0.5)).collect();
    assert!(!stroke_has_pressure(&samples));
}

#[test]
fn criterion_3_a_pen_at_constant_zero_on_contact_has_none() {
    let samples: Vec<_> = (0..10).map(|i| pen(f64::from(i), 0.0)).collect();
    assert!(!stroke_has_pressure(&samples));
}

#[test]
fn criterion_3_a_mouse_has_none() {
    let samples: Vec<_> = (0..10)
        .map(|i| sample(PointerKind::Mouse, f64::from(i), 0.3))
        .collect();
    assert!(!stroke_has_pressure(&samples));
}

// Criterion 4: touch is ignored.

#[test]
fn criterion_4_a_touch_starts_no_stroke() {
    let mut tool = PencilTool::default();
    assert!(!tool.press(sample(PointerKind::Touch, 0.0, 0.5), PRIMARY));
    tool.extend(&[sample(PointerKind::Touch, 5.0, 0.5)]);
    assert!(!tool.is_drawing());
    assert_eq!(tool.release(&[sample(PointerKind::Touch, 9.0, 0.5)]), None);
}

// Criterion 5: a pen in range but not touching starts nothing.

#[test]
fn criterion_5_a_hovering_pen_starts_nothing_and_a_touching_pen_starts_a_stroke() {
    let mut tool = PencilTool::default();
    assert!(!tool.press(pen(0.0, 0.0), 0));
    assert!(!tool.is_drawing());
    assert!(tool.press(pen(0.0, 0.3), PRIMARY));
    assert!(tool.is_drawing());
}

// Criterion 9: smoothing first, the curve second.

#[test]
fn criterion_9_pressure_is_the_mean_of_a_sample_and_the_two_before_it() {
    let smoothed = smooth_pressures(&[0.2, 0.2, 1.0, 0.2, 0.2]);
    let expected = [0.2, 0.2, 0.467, 0.467, 0.467];
    assert_eq!(smoothed.len(), expected.len());
    for (got, want) in smoothed.iter().zip(expected) {
        assert!((got - want).abs() <= 0.001, "{smoothed:?}");
    }
}

#[test]
fn criterion_9_the_curves_and_the_widths() {
    for (curve, mapped, width) in [
        (PressureCurve::Soft, 0.5, 1.1),
        (PressureCurve::Linear, 0.25, 0.65),
        (PressureCurve::Firm, 0.0625, 0.3125),
    ] {
        assert!((curve.apply(0.25) - mapped).abs() < 1e-12, "{curve:?}");
        let mut settings = PencilSettings::default();
        settings.curve = curve;
        assert!(
            (settings.width_for(0.25).as_mm() - width).abs() < 1e-12,
            "{curve:?}"
        );
    }
}

// Criterion 14: the defaults and the limits of the settings.

#[test]
fn criterion_14_defaults_and_limits() {
    let mut settings = PencilSettings::default();
    assert!((settings.minimum().as_mm() - 0.2).abs() < 1e-12);
    assert!((settings.maximum().as_mm() - 2.0).abs() < 1e-12);
    assert_eq!(settings.curve, PressureCurve::Linear);
    settings.set_minimum(Length::from_mm(0.01));
    assert!((settings.minimum().as_mm() - 0.05).abs() < 1e-12);
    settings.set_minimum(Length::from_mm(99.0));
    assert!((settings.minimum().as_mm() - 20.0).abs() < 1e-12);
    assert!(
        (settings.maximum().as_mm() - 20.0).abs() < 1e-12,
        "Maximum is raised to Minimum"
    );
    settings.set_maximum(Length::from_mm(5.0));
    assert!(
        (settings.maximum().as_mm() - 20.0).abs() < 1e-12,
        "a Maximum below Minimum is raised to it"
    );
    settings.set_minimum(Length::from_mm(0.2));
    settings.set_maximum(Length::from_mm(80.0));
    assert!((settings.maximum().as_mm() - 50.0).abs() < 1e-12);
    settings.set_maximum(Length::from_mm(0.0));
    assert!((settings.maximum().as_mm() - 0.2).abs() < 1e-12);
}

// Criterion 12: Escape and too-short strokes.

#[test]
fn criterion_12_escape_cancels_the_stroke() {
    let mut tool = PencilTool::default();
    assert!(tool.press(pen(0.0, 0.3), PRIMARY));
    tool.extend(&[pen(5.0, 0.4), pen(10.0, 0.5)]);
    assert!(tool.cancel());
    assert!(!tool.is_drawing());
    assert_eq!(tool.release(&[pen(12.0, 0.4)]), None);
    assert!(!tool.cancel(), "nothing left to cancel");
}

#[test]
fn criterion_12_a_single_sample_or_a_stroke_under_a_tenth_of_a_millimetre_creates_nothing() {
    let mut tool = PencilTool::default();
    assert!(tool.press(pen(0.0, 0.3), PRIMARY));
    assert_eq!(tool.release(&[]), None);

    assert!(tool.press(pen(0.0, 0.3), PRIMARY));
    tool.extend(&[pen(0.03, 0.4), pen(0.06, 0.4)]);
    assert_eq!(tool.release(&[pen(0.09, 0.4)]), None);
}

// Criteria 7 and 8 at this level: which kind of shape a stroke gives.

#[test]
fn a_stroke_without_pressure_gives_an_open_centre_line() {
    let mut tool = PencilTool::default();
    assert!(tool.press(sample(PointerKind::Mouse, 0.0, 0.5), PRIMARY));
    let middle: Vec<_> = (1..50)
        .map(|i| sample(PointerKind::Mouse, f64::from(i), 0.5))
        .collect();
    tool.extend(&middle);
    let stroke = tool
        .release(&[sample(PointerKind::Mouse, 50.0, 0.5)])
        .unwrap();
    assert!(!stroke.has_pressure());
    match stroke.shape(&PencilSettings::default()) {
        Some(StrokeShape::Centre(anchors)) => assert_eq!(anchors.len(), 2),
        other => panic!("expected an open centre line, got {other:?}"),
    }
}

#[test]
fn a_stroke_with_pressure_gives_a_closed_outline() {
    let mut tool = PencilTool::default();
    assert!(tool.press(pen(0.0, 0.1), PRIMARY));
    let middle: Vec<_> = (1..50).map(|i| pen(f64::from(i), 0.8)).collect();
    tool.extend(&middle);
    let stroke = tool.release(&[pen(50.0, 0.8)]).unwrap();
    assert!(stroke.has_pressure());
    match stroke.shape(&PencilSettings::default()) {
        Some(StrokeShape::Outlines(outlines)) => assert_eq!(outlines.len(), 1),
        other => panic!("expected an outline, got {other:?}"),
    }
}

#[test]
fn the_preview_switches_to_an_outline_when_the_first_usable_pressure_arrives() {
    let mut tool = PencilTool::default();
    let settings = PencilSettings::default();
    assert!(tool.press(pen(0.0, 0.5), PRIMARY));
    tool.extend(&[pen(5.0, 0.5), pen(10.0, 0.5)]);
    assert!(matches!(
        tool.preview(&settings),
        Some(StrokeShape::Centre(_))
    ));
    tool.extend(&[pen(15.0, 0.7)]);
    assert!(matches!(
        tool.preview(&settings),
        Some(StrokeShape::Outlines(_))
    ));
}

#[test]
fn a_touch_sample_inside_a_pen_stroke_is_ignored() {
    let mut tool = PencilTool::default();
    assert!(tool.press(pen(0.0, 0.3), PRIMARY));
    tool.extend(&[sample(PointerKind::Touch, 100.0, 0.5), pen(5.0, 0.3)]);
    let stroke = tool.release(&[pen(10.0, 0.3)]).unwrap();
    assert_eq!(stroke.points().len(), 3);
}
