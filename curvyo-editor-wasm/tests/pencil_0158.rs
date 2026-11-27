//! The Pressure Pencil at session level (`specs/0158-pen-display` slice 1): every behaviour is fed
//! as batches of flat `PenSample` records, so no tablet is needed (ADR 0095 decision 1).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::assert_is_empty, missing_docs)]

use curvyo_document_core::{
    AnchorId, Direction, Document, NewAnchor, NodeId, PathSnapshot, Point, pack, unpack,
};
use curvyo_editor_wasm::{KeyInput, Session, Tool};
use curvyo_ui_core::PressureCurve::{Firm, Linear, Soft};
use curvyo_ui_core::{PEN_SAMPLE_STRIDE, WidthPreset};

const TIP: f64 = 0.0;
const ERASER: f64 = 1.0;
const HOVER: f64 = 0.0;
const CONTACT: f64 = 1.0;

/// One flat record at document point `at`, converted to canvas CSS px for `session`.
#[allow(clippy::too_many_arguments)]
fn record(
    session: &Session,
    at: Point,
    pressure: f64,
    end: f64,
    phase: f64,
    time: f64,
) -> Vec<f64> {
    let origin = session.screen_to_document(0.0, 0.0);
    let unit = session.screen_to_document(1.0, 0.0);
    let px_per_mm = 1.0 / (unit.x - origin.x);
    vec![
        (at.x - origin.x) * px_per_mm,
        (at.y - origin.y) * px_per_mm,
        pressure,
        f64::NAN,
        f64::NAN,
        end,
        phase,
        0.0,
        time,
    ]
}

/// A stroke of `n` contact samples along x from `from`, `length` mm long at `ms` per sample,
/// with `pressure(t)`, then a hover sample.
fn stroke(
    session: &Session,
    from: Point,
    length: f64,
    n: usize,
    ms: f64,
    pressure: impl Fn(f64) -> f64,
) -> Vec<f64> {
    let mut flat = Vec::new();
    for i in 0..n {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f64 / (n - 1) as f64;
        let at = Point::new(
            from.x + t * length,
            from.y + 3.0 * (t * std::f64::consts::PI).sin(),
        );
        #[allow(clippy::cast_precision_loss)]
        flat.extend(record(
            session,
            at,
            pressure(t),
            TIP,
            CONTACT,
            i as f64 * ms,
        ));
    }
    #[allow(clippy::cast_precision_loss)]
    let end = Point::new(from.x + length + 1.0, from.y);
    #[allow(clippy::cast_precision_loss)]
    flat.extend(record(session, end, 0.0, TIP, HOVER, n as f64 * ms));
    flat
}

fn pencil() -> Session {
    let mut session = Session::new(1);
    let _ = session.draw_list();
    session.set_tool(Tool::Pencil);
    session
}

fn paths(session: &Session) -> Vec<PathSnapshot> {
    let document = unpack(2, &session.pack("test").unwrap()).unwrap();
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| document.path(id))
        .collect()
}

fn undo_name(session: &mut Session) -> Option<String> {
    session.history_target_name(Direction::Undo)
}

/// The width at normalised position `at` of a path's profile (smoothstep between its points).
fn width_at(path: &PathSnapshot, at: f64) -> f64 {
    let points: Vec<curvyo_geometry_core::WidthPoint> = path
        .width_points
        .iter()
        .enumerate()
        .filter_map(|(i, w)| {
            curvyo_geometry_core::WidthPoint::new(i as u128, w.position, w.multiplier, false)
        })
        .collect();
    let profile = curvyo_geometry_core::WidthProfile::new(points, false).unwrap();
    profile.sample(at).unwrap() * path.style.stroke.width.as_mm()
}

#[test]
fn criterion_1_a_pressure_stroke_is_one_selected_path_and_one_undo_step() {
    let mut session = pencil();
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 400, 4.0, |t| {
        0.1 + 0.8 * t
    });
    session.pen_samples(&flat);
    let made = paths(&session);
    assert_eq!(made.len(), 1);
    let path = &made[0];
    assert!(!path.closed);
    assert!(path.anchors.len() >= 2);
    assert!(!path.width_points.is_empty());
    assert!(width_at(path, 0.0) < width_at(path, 1.0));
    assert_eq!(session.selected_object_count(), 1);
    assert_eq!(session.tool(), Tool::Pencil);
    assert_eq!(undo_name(&mut session).as_deref(), Some("Pencil stroke"));
    session.undo();
    assert!(
        paths(&session).is_empty(),
        "one undo removes the whole stroke"
    );
}

#[test]
fn criteria_3_4_widths_come_from_min_max_and_the_curve() {
    let widths = |curve: curvyo_ui_core::PressureCurve| {
        let mut session = pencil();
        session.set_pencil_curve(curve);
        let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 300, 4.0, |t| {
            0.1 + 0.8 * t
        });
        session.pen_samples(&flat);
        let path = paths(&session).remove(0);
        // Max is the path's stroke width (criterion 13).
        assert!((path.style.stroke.width.as_mm() - 2.0).abs() < 1e-9);
        width_at(&path, 0.5)
    };
    let (soft, linear, firm) = (widths(Soft), widths(Linear), widths(Firm));
    assert!(soft > linear && linear > firm, "{soft} {linear} {firm}");
    // Linear at mid pressure 0.5: 0.2 + 1.8 x 0.5 = 1.1 mm, within the 5 % profile tolerance.
    assert!((linear - 1.1).abs() <= 0.09 + 0.02, "{linear}");
    let mut session = pencil();
    session.set_pencil_min(curvyo_document_core::Length::from_mm(1.0));
    session.set_pencil_max(curvyo_document_core::Length::from_mm(0.5));
    let settings = session.pencil_settings();
    assert!(
        (settings.widths.max().as_mm() - 1.0).abs() < 1e-12,
        "Max never below Min"
    );
}

#[test]
fn criterion_5_save_and_reopen_keep_the_path_and_its_profile() {
    let mut session = pencil();
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 400, 4.0, |t| {
        0.1 + 0.8 * t
    });
    session.pen_samples(&flat);
    let before = paths(&session).remove(0);
    let reopened = Session::open(5, &session.pack("test").unwrap()).unwrap();
    let after = paths(&reopened).remove(0);
    assert_eq!(before.anchors, after.anchors);
    assert_eq!(before.width_points, after.width_points);
    assert_eq!(before.style, after.style);
    assert!(
        before.width_points.len() <= 20,
        "{} points",
        before.width_points.len()
    );
}

#[test]
fn criterion_6_the_profile_buttons_work_on_a_pencil_stroke() {
    let mut session = pencil();
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 300, 4.0, |t| {
        0.1 + 0.8 * t
    });
    session.pen_samples(&flat);
    session.set_tool(Tool::Node);
    session.apply_width_preset(WidthPreset::TaperBoth).unwrap();
    assert_eq!(
        session.selected_width_preset(),
        Some(WidthPreset::TaperBoth)
    );
    session.apply_width_preset(WidthPreset::Uniform).unwrap();
    assert!(paths(&session)[0].width_points.is_empty());
    session.undo();
    assert_eq!(
        session.selected_width_preset(),
        Some(WidthPreset::TaperBoth)
    );
}

#[test]
fn criterion_7_a_mouse_or_a_pen_without_pressure_draws_a_uniform_line() {
    for pressure in [f64::NAN, 0.5, 0.0] {
        let mut session = pencil();
        let flat = stroke(&session, Point::new(20.0, 50.0), 60.0, 200, 4.0, |_| {
            pressure
        });
        session.pen_samples(&flat);
        let path = paths(&session).remove(0);
        assert!(path.width_points.is_empty(), "pressure {pressure}");
        assert!(
            (path.style.stroke.width.as_mm() - 0.25).abs() < 1e-9,
            "the style's width"
        );
        assert_eq!(session.pencil_last_had_pressure(), Some(false));
    }
    assert!(
        !Session::new(3).pencil_settings().simulate,
        "off by default"
    );
}

#[test]
fn criterion_8_simulated_pressure_follows_the_speed() {
    // 60 mm in 200 samples: 4 ms per sample is 75 mm/s, 2 ms is 150 mm/s.
    let width = |ms: f64| {
        let mut session = pencil();
        session.set_pencil_simulate(true);
        let flat = stroke(&session, Point::new(20.0, 50.0), 60.0, 200, ms, |_| {
            f64::NAN
        });
        session.pen_samples(&flat);
        let path = paths(&session).remove(0);
        assert!(!path.width_points.is_empty());
        (
            width_at(&path, 0.2),
            width_at(&path, 0.5),
            width_at(&path, 0.8),
        )
    };
    let slow = width(4.0);
    let fast = width(2.0);
    assert!(
        (slow.0 - slow.2).abs() <= 0.09 + 0.02,
        "constant speed, constant width: {slow:?}"
    );
    assert!(slow.1 > fast.1, "half as fast is wider: {slow:?} {fast:?}");
    // One stroke, fast then slow: the slow part is wider.
    let mut session = pencil();
    session.set_pencil_simulate(true);
    let mut flat = Vec::new();
    let mut time = 0.0;
    for i in 0..300 {
        let x = 20.0 + f64::from(i) * 0.3;
        time += if i < 150 { 1.5 } else { 6.0 };
        flat.extend(record(
            &session,
            Point::new(x, 50.0),
            f64::NAN,
            TIP,
            CONTACT,
            time,
        ));
    }
    flat.extend(record(
        &session,
        Point::new(120.0, 50.0),
        f64::NAN,
        TIP,
        HOVER,
        time + 5.0,
    ));
    session.pen_samples(&flat);
    let path = paths(&session).remove(0);
    assert!(width_at(&path, 0.8) > width_at(&path, 0.2));
    // With pen pressure the option is ignored.
    let pressured = |simulate: bool| {
        let mut session = pencil();
        session.set_pencil_simulate(simulate);
        let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 300, 2.0, |t| {
            0.1 + 0.8 * t
        });
        session.pen_samples(&flat);
        paths(&session).remove(0).width_points
    };
    let (on, off) = (pressured(true), pressured(false));
    let values = |w: &[curvyo_document_core::WidthPoint]| -> Vec<(f64, f64)> {
        w.iter().map(|p| (p.position, p.multiplier)).collect()
    };
    assert_eq!(values(&on), values(&off));
}

#[test]
fn criterion_9_smoothing_applies_from_the_next_stroke() {
    let nodes = |smoothing: f64| {
        let mut session = pencil();
        assert!(session.set_pencil_smoothing(curvyo_document_core::Length::from_mm(smoothing)));
        let mut flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 400, 4.0, |_| {
            f64::NAN
        });
        // A tremor across the stroke.
        for (i, record) in flat.chunks_mut(PEN_SAMPLE_STRIDE).enumerate() {
            record[1] += if i % 2 == 0 { 0.3 } else { -0.3 };
        }
        session.pen_samples(&flat);
        paths(&session).remove(0).anchors.len()
    };
    assert!(nodes(0.0) >= nodes(1.0));
    // A change during a stroke does not touch it.
    let mut session = pencil();
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 400, 4.0, |_| {
        f64::NAN
    });
    let half = flat.len() / 2 / PEN_SAMPLE_STRIDE * PEN_SAMPLE_STRIDE;
    session.pen_samples(&flat[..half]);
    session.set_pencil_smoothing(curvyo_document_core::Length::from_mm(0.0));
    session.pen_samples(&flat[half..]);
    let mut reference = pencil();
    reference.pen_samples(&flat);
    assert_eq!(
        paths(&session)[0].anchors.len(),
        paths(&reference)[0].anchors.len()
    );
    assert!(!session.set_pencil_smoothing(curvyo_document_core::Length::from_mm(-1.0)));
}

#[test]
fn criterion_10_the_preview_draws_while_a_stroke_runs() {
    let mut session = pencil();
    let empty = session.draw_list().triangle_count();
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 200, 4.0, |t| {
        0.1 + 0.8 * t
    });
    let contact = flat.len() - PEN_SAMPLE_STRIDE;
    session.pen_samples(&flat[..contact]);
    let outline = session.draw_list().triangle_count();
    assert!(outline > empty, "the hollow outline shows");
    session.pen_samples(&flat[contact..]);
    assert_eq!(paths(&session).len(), 1);
    let mut mouse = pencil();
    let flat = stroke(&mouse, Point::new(20.0, 50.0), 100.0, 200, 4.0, |_| {
        f64::NAN
    });
    mouse.pen_samples(&flat[..contact]);
    let line = mouse.draw_list().triangle_count();
    assert!(
        line > empty && line < outline,
        "the centre line shows: {line} vs {outline}"
    );
}

#[test]
fn criterion_11_hover_batches_tilt_and_barrel() {
    let mut session = pencil();
    let hover: Vec<f64> = (0..20)
        .flat_map(|i| {
            record(
                &session,
                Point::new(f64::from(i), 10.0),
                0.0,
                TIP,
                HOVER,
                f64::from(i),
            )
        })
        .collect();
    session.pen_samples(&hover);
    assert!(paths(&session).is_empty());
    assert_eq!(undo_name(&mut session), None);
    // One stroke over many batches equals one batch.
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 300, 4.0, |t| {
        0.1 + 0.8 * t
    });
    let mut split = pencil();
    for chunk in flat.chunks(PEN_SAMPLE_STRIDE * 13) {
        split.pen_samples(chunk);
    }
    session.pen_samples(&flat);
    let (a, b) = (paths(&session).remove(0), paths(&split).remove(0));
    assert_eq!(a.anchors.len(), b.anchors.len());
    assert_eq!(a.width_points.len(), b.width_points.len());
    // Tilt and barrel bits are carried and change nothing.
    let mut tilted = flat.clone();
    for record in tilted.chunks_mut(PEN_SAMPLE_STRIDE) {
        record[3] = 0.4;
        record[4] = -0.2;
        record[7] = 3.0;
    }
    let mut other = pencil();
    other.pen_samples(&tilted);
    let c = paths(&other).remove(0);
    assert_eq!(
        a.anchors.iter().map(|x| x.point).collect::<Vec<_>>(),
        c.anchors.iter().map(|x| x.point).collect::<Vec<_>>()
    );
    // Samples outside the Pencil are ignored.
    let mut select = Session::new(4);
    select.pen_samples(&flat);
    assert!(paths(&select).is_empty());
}

/// A session with one open path along y = 50 from x = 20 to 80 and one along y = 90.
fn with_lines() -> (Session, NodeId, NodeId) {
    let doc = Document::new(70);
    let line = |y: f64, tag: u64| {
        doc.create_path(
            &[
                NewAnchor::corner(AnchorId::new(70, tag), Point::new(20.0, y)),
                NewAnchor::corner(AnchorId::new(70, tag + 1), Point::new(80.0, y)),
            ],
            false,
        )
    };
    let (a, b) = (line(50.0, 1), line(90.0, 3));
    let mut session = Session::open(71, &pack(&doc, "test").unwrap()).unwrap();
    let _ = session.draw_list();
    session.set_tool(Tool::Pencil);
    (session, a, b)
}

fn eraser_across(session: &Session, x: f64, y0: f64, y1: f64) -> Vec<f64> {
    let mut flat = Vec::new();
    for i in 0..=20 {
        let y = y0 + (y1 - y0) * f64::from(i) / 20.0;
        flat.extend(record(
            session,
            Point::new(x, y),
            0.5,
            ERASER,
            CONTACT,
            f64::from(i),
        ));
    }
    flat.extend(record(session, Point::new(x, y1), 0.0, ERASER, HOVER, 30.0));
    flat
}

#[test]
fn criterion_12_the_eraser_end_deletes_what_it_crosses_in_one_step() {
    let (mut session, a, b) = with_lines();
    let flat = eraser_across(&session, 50.0, 40.0, 60.0);
    session.pen_samples(&flat);
    let left: Vec<NodeId> = paths(&session).iter().map(|p| p.id).collect();
    assert_eq!(
        left,
        vec![b],
        "only the crossed line goes, no stroke is drawn: {a:?}"
    );
    assert_eq!(undo_name(&mut session).as_deref(), Some("Erase"));
    assert_eq!(session.tool(), Tool::Pencil);
    session.undo();
    assert_eq!(paths(&session).len(), 2);
    // An eraser that touches nothing writes nothing.
    let (mut session, _, _) = with_lines();
    session.pen_samples(&eraser_across(&session, 50.0, 60.0, 80.0));
    assert_eq!(paths(&session).len(), 2);
    assert_eq!(undo_name(&mut session), None);
}

#[test]
fn criterion_12_a_locked_object_is_not_erased_and_the_hover_shows_the_target() {
    let doc = Document::new(80);
    let id = doc.create_path(
        &[
            NewAnchor::corner(AnchorId::new(80, 1), Point::new(20.0, 50.0)),
            NewAnchor::corner(AnchorId::new(80, 2), Point::new(80.0, 50.0)),
        ],
        false,
    );
    doc.set_node_locked(&[id], true).unwrap();
    let mut session = Session::open(81, &pack(&doc, "test").unwrap()).unwrap();
    let _ = session.draw_list();
    session.set_tool(Tool::Pencil);
    session.pen_samples(&eraser_across(&session, 50.0, 40.0, 60.0));
    assert_eq!(paths(&session).len(), 1, "a locked object stays");

    let (mut session, _, _) = with_lines();
    let far = record(&session, Point::new(50.0, 70.0), 0.0, ERASER, HOVER, 0.0);
    session.pen_samples(&far);
    let away = session.draw_list().triangle_count();
    let near = record(&session, Point::new(50.0, 50.5), 0.0, ERASER, HOVER, 1.0);
    session.pen_samples(&near);
    let over = session.draw_list().triangle_count();
    assert!(
        over > away,
        "the hover box marks what the eraser would delete"
    );
    assert_eq!(session.cursor_hint(), "pencil-eraser");
    let tip = record(&session, Point::new(50.0, 50.5), 0.0, TIP, HOVER, 2.0);
    session.pen_samples(&tip);
    assert_eq!(session.draw_list().triangle_count(), away);
    assert_eq!(session.cursor_hint(), "crosshair");
}

#[test]
fn criteria_13_14_the_stroke_is_an_ordinary_unfilled_path() {
    let mut session = pencil();
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 300, 4.0, |t| {
        0.1 + 0.8 * t
    });
    session.pen_samples(&flat);
    let path = paths(&session).remove(0);
    assert!(!path.style.fill.enabled);
    assert!(path.style.stroke.enabled);
    assert_eq!(path.curve_kind, curvyo_document_core::CurveKind::Bezier);
    assert!(path.extra_subpaths.is_empty());
    // Stroke to path makes the closed outline of it (`0085`).
    session.set_tool(Tool::Select);
    assert!(session.stroke_to_path_available());
    let _ = session.apply_stroke_to_path();
    let outlines = paths(&session);
    assert!(
        !outlines.is_empty()
            && outlines
                .iter()
                .all(|p| p.closed && p.width_points.is_empty())
    );
}

#[test]
fn criterion_15_escape_a_tap_and_a_locked_layer_write_nothing() {
    let mut session = pencil();
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 100, 4.0, |t| {
        0.1 + 0.8 * t
    });
    session.pen_samples(&flat[..PEN_SAMPLE_STRIDE * 50]);
    session.key_down(KeyInput {
        key: "Escape",
        ..Default::default()
    });
    assert_eq!(
        session.tool(),
        Tool::Pencil,
        "the first Escape cancels the stroke"
    );
    session.pen_samples(&flat[PEN_SAMPLE_STRIDE * 50..]);
    assert!(paths(&session).is_empty());
    assert_eq!(undo_name(&mut session), None);
    // A tap: one contact sample, then a hover.
    let mut flat = record(&session, Point::new(30.0, 30.0), 0.4, TIP, CONTACT, 0.0);
    flat.extend(record(
        &session,
        Point::new(30.05, 30.0),
        0.4,
        TIP,
        CONTACT,
        4.0,
    ));
    flat.extend(record(
        &session,
        Point::new(30.05, 30.0),
        0.0,
        TIP,
        HOVER,
        8.0,
    ));
    session.pen_samples(&flat);
    assert!(paths(&session).is_empty());
    // Two strokes in one batch: two paths, two undo steps.
    let mut both = stroke(&session, Point::new(20.0, 50.0), 50.0, 100, 4.0, |t| {
        0.2 + 0.6 * t
    });
    both.extend(stroke(
        &session,
        Point::new(20.0, 80.0),
        50.0,
        100,
        4.0,
        |t| 0.2 + 0.6 * t,
    ));
    session.pen_samples(&both);
    assert_eq!(paths(&session).len(), 2);
    session.undo();
    assert_eq!(paths(&session).len(), 1);
}

#[test]
fn criterion_15_a_hidden_or_locked_layer_starts_nothing_with_the_notice() {
    let doc = Document::new(90);
    let layer = doc.new_layer(None);
    doc.set_node_locked(&[layer], true).unwrap();
    let mut session = Session::open(91, &pack(&doc, "test").unwrap()).unwrap();
    let _ = session.draw_list();
    session.set_tool(Tool::Pencil);
    let flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 100, 4.0, |t| {
        0.1 + 0.8 * t
    });
    session.pen_samples(&flat);
    assert!(paths(&session).is_empty());
    assert!(session.take_layer_notice().contains("locked"));
}

#[test]
fn criterion_17_nothing_of_the_device_reaches_the_document() {
    // The same positions and pressures with other times, tilt and barrel bits give the same bytes
    // of path: only the fitted centre line and profile are kept.
    let draw = |time_scale: f64, tilt: f64| {
        let mut session = pencil();
        let mut flat = stroke(&session, Point::new(20.0, 50.0), 100.0, 300, 4.0, |t| {
            0.1 + 0.8 * t
        });
        for record in flat.chunks_mut(PEN_SAMPLE_STRIDE) {
            record[3] = tilt;
            record[4] = tilt;
            record[7] = 1.0;
            record[8] *= time_scale;
        }
        session.pen_samples(&flat);
        let path = paths(&session).remove(0);
        (
            path.anchors
                .iter()
                .map(|a| (a.point, a.handle_in, a.handle_out))
                .collect::<Vec<_>>(),
            path.width_points
                .iter()
                .map(|w| (w.position, w.multiplier))
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(draw(1.0, f64::NAN), draw(3.0, 0.7));
}

#[test]
fn key_p_picks_the_pencil() {
    let mut session = Session::new(6);
    session.key_down(KeyInput {
        key: "p",
        ..Default::default()
    });
    assert_eq!(session.tool(), Tool::Pencil);
}
