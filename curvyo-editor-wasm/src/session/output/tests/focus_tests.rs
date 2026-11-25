//! The Focus point of `specs/0118-machine-control` slice 2d in the session (criteria 34, 35 a,
//! 36; adrs P1, P7, P12; ux-notes W55, W59, W60): the press mapped per origin corner and Start
//! pick, the page's causes, the target ring as the mapping's inverse, the pulse fields with their
//! notices and steps, and the ranges equal to the connector's.

use curvyo_laser_core::job::{PointUm, mm_of_um, um_of_mm};

use super::super::focus::FocusTarget;
use super::{cut, laser_with_square, undo_label};
use crate::session::Session;

fn page_point(s: &Session, x: f64, y: f64) -> (f64, f64) {
    let origin = s.active_manufacturing().unwrap().page.unwrap().origin;
    (origin.x + x, origin.y + y)
}

fn target(s: &Session, x: f64, y: f64) -> FocusTarget {
    let (dx, dy) = page_point(s, x, y);
    s.focus_target(dx, dy).unwrap()
}

fn copy(s: &Session) -> serde_json::Value {
    let view: serde_json::Value = serde_json::from_str(&s.manufacturing_view_json()).unwrap();
    view["machine"]["copy"].clone()
}

#[test]
fn c34_a_press_maps_per_origin_corner_for_machine_origin_and_home_then_machine_origin() {
    // The page is the Generic laser's 400 x 300 mm bed; the press is at page (10, 20).
    let table = [
        ("bottom_left", (10.0, 280.0)),
        ("top_left", (10.0, 20.0)),
        ("top_right", (390.0, 20.0)),
        ("bottom_right", (390.0, 280.0)),
    ];
    for start in ["machine_origin", "home_then_origin"] {
        for (corner, (x, y)) in table {
            let (mut s, _) = laser_with_square();
            assert!(s.set_machine_origin(corner) || corner == "bottom_left");
            if start != "machine_origin" {
                assert!(s.set_start(start));
            }
            assert_eq!(
                target(&s, 10.0, 20.0),
                FocusTarget::Point { x, y },
                "{start} {corner}"
            );
        }
    }
}

#[test]
fn c34_with_user_origin_the_press_lands_where_that_drawing_point_burns() {
    for corner in ["bottom_left", "top_left", "top_right", "bottom_right"] {
        let (mut s, square) = laser_with_square();
        cut(&mut s, &[square]);
        s.set_machine_origin(corner);
        assert!(s.set_user_origin(100.0, 50.0));
        let FocusTarget::Point { x, y } = target(&s, 25.0, 15.0) else {
            panic!("{corner}: a point");
        };
        let job = s.laser_job().unwrap();
        let traced = s.frame_of(&job).0;
        let expected = traced.machine_point(PointUm::new(um_of_mm(25.0), um_of_mm(15.0)));
        assert_eq!(
            (x, y),
            (mm_of_um(expected.x), mm_of_um(expected.y)),
            "{corner}"
        );
        assert_ne!(
            traced.shift,
            PointUm::new(0, 0),
            "{corner}: the job is shifted"
        );
    }
}

#[test]
fn w55_head_position_and_user_origin_with_nothing_to_place_are_the_pages_causes() {
    let (mut s, _) = laser_with_square();
    assert!(s.set_start("head_position"));
    assert_eq!(
        target(&s, 1.0, 1.0),
        FocusTarget::Cause {
            cause: "Start is Head position"
        }
    );
    let (mut s, _) = laser_with_square();
    assert!(s.set_user_origin(100.0, 50.0));
    assert_eq!(
        target(&s, 1.0, 1.0),
        FocusTarget::Cause {
            cause: "Nothing to place"
        }
    );
    let (s, _) = laser_with_square();
    assert!(
        matches!(target(&s, 1.0, 1.0), FocusTarget::Point { .. }),
        "a page without a job is no cause with Machine origin"
    );
}

#[test]
fn w59_the_target_ring_is_the_inverse_of_the_press_and_hides_the_play_head() {
    let (mut s, square) = laser_with_square();
    cut(&mut s, &[square]);
    let view = curvyo_document_core::ViewTransform::identity();
    let FocusTarget::Point { x, y } = target(&s, 12.0, 34.0) else {
        panic!("a point");
    };
    assert!(s.frame_head_overlay(view).is_none());
    assert!(s.set_focus_ring(x, y));
    let ring = s.output.focus_ring.unwrap().0;
    let (px, py) = page_point(&s, 12.0, 34.0);
    assert!((ring.x - px).abs() < 1e-9 && (ring.y - py).abs() < 1e-9);
    assert!(s.frame_head_overlay(view).is_some());
    assert!(!s.toggle_play(0.0), "Play pauses while the ring shows");
    assert!(s.clear_focus_ring());
    assert!(!s.clear_focus_ring());
    assert!(s.frame_head_overlay(view).is_none());
}

#[test]
fn c35a_the_pulse_fields_are_steps_with_the_notices_of_w60_and_the_defaults() {
    let (mut s, _) = laser_with_square();
    let c = copy(&s);
    assert_eq!(
        c["focus_pulse"], false,
        "off in a new profile (Frage 369 B)"
    );
    assert_eq!(c["pulse_power"], 1.0);
    assert_eq!(c["pulse_length"], 300);
    assert_eq!(s.focus_pulse(), None);
    assert!(s.set_machine_focus_pulse(true));
    assert_eq!(undo_label(&mut s), "Edit focus pulse");
    assert!(!s.set_machine_focus_pulse(true), "unchanged writes no step");
    assert_eq!(
        s.set_machine_pulse_power(9.0).as_deref(),
        Some("Pulse power limited to 5 %.")
    );
    assert_eq!(undo_label(&mut s), "Edit pulse power");
    assert_eq!(s.set_machine_pulse_power(2.5), None);
    assert_eq!(copy(&s)["pulse_power"], 2.5);
    assert_eq!(
        s.set_machine_pulse_length(300.6).as_deref(),
        Some("Pulse length rounded to 301 ms.")
    );
    assert_eq!(undo_label(&mut s), "Edit pulse length");
    let pulse = s.focus_pulse().unwrap();
    assert_eq!((pulse.power_permille, pulse.length_ms), (25, 301));
    s.set_machine_laser_mode(false);
    assert_eq!(s.focus_pulse(), None, "Laser mode off sends no pulse");
}

#[test]
fn adrs_p7_the_profile_ranges_equal_the_connectors_caps() {
    use curvyo_connector_core::{PulseLength, PulsePower};
    use curvyo_laser_core::{PulseMs, PulsePermille};
    assert_eq!(u64::from(PulsePower::MIN), u64::from(PulsePermille::MIN));
    assert_eq!(u64::from(PulsePower::MAX), u64::from(PulsePermille::MAX));
    assert_eq!(u64::from(PulseLength::MIN), u64::from(PulseMs::MIN));
    assert_eq!(u64::from(PulseLength::MAX), u64::from(PulseMs::MAX));
}
