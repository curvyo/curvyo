//! Width profile interpolation follows the approved curve and seam rules.
#![allow(clippy::unwrap_used)]

use curvyo_geometry_core::{WidthPoint, WidthProfile};

fn point(id: u128, position: f64, multiplier: f64) -> WidthPoint {
    WidthPoint::new(id, position, multiplier, false).unwrap()
}

#[test]
fn open_profile_extends_and_interpolates_without_overshoot() {
    let profile = WidthProfile::new(vec![point(1, 0.25, 0.0), point(2, 0.75, 2.0)], false).unwrap();
    for (at, expected) in [(0.0, 0.0), (0.375, 0.3125), (0.5, 1.0), (1.0, 2.0)] {
        assert!((profile.sample(at).unwrap() - expected).abs() < 1e-12);
    }
}

#[test]
fn closed_profile_interpolates_across_seam_and_resolves_duplicates_by_id() {
    let profile = WidthProfile::new(
        vec![
            point(2, 0.25, 2.0),
            point(1, 0.25, 1.0),
            point(3, 0.75, 0.0),
        ],
        true,
    )
    .unwrap();
    assert!((profile.sample(0.25).unwrap() - 2.0).abs() < 1e-12);
    assert!((profile.sample(0.0).unwrap() - 1.0).abs() < 1e-12);
    assert_eq!(profile.sample(0.0), profile.sample(1.0));
}

#[test]
fn empty_single_and_invalid_inputs() {
    assert_eq!(
        WidthProfile::new(vec![], false).unwrap().sample(0.5),
        Some(1.0)
    );
    assert_eq!(
        WidthProfile::new(vec![point(1, 0.4, 3.0)], false)
            .unwrap()
            .sample(0.9),
        Some(3.0)
    );
    assert!(WidthPoint::new(1, f64::NAN, 1.0, false).is_none());
    assert!(WidthPoint::new(1, 0.5, -1.0, false).is_none());
    assert!(WidthPoint::new(1, 1.0, 1.0, true).is_none());
    assert!(
        WidthProfile::new(vec![], false)
            .unwrap()
            .sample(f64::NAN)
            .is_none()
    );
    let large =
        WidthProfile::new(vec![point(1, 0.0, 0.0), point(2, 1.0, f64::MAX)], false).unwrap();
    assert!(large.sample(0.5).unwrap().is_finite());
}
