//! Tests of the footprint spans, seams and tabs.

use super::*;

fn mm(value: f64) -> Length {
    Length::from_mm(value)
}

fn footprint(w: f64, d: f64, r: f64, t: f64) -> Footprint {
    Footprint::rounded_rect(mm(w), mm(d), mm(r), mm(t))
}

#[test]
fn the_strip_is_as_long_as_the_perimeter_on_the_neutral_line() {
    // An independent sum, not the spans' own lengths (ADR 0082 §9).
    for (w, d, r, t) in [
        (120.0, 80.0, 15.0, 3.0),
        (100.0, 40.0, 20.0, 3.0),
        (40.0, 100.0, 20.0, 3.0),
        (60.0, 60.0, 30.0, 3.0),
        (120.0, 80.0, 6.0, 3.0),
    ] {
        let expected =
            2.0 * (w - 2.0 * r) + 2.0 * (d - 2.0 * r) + std::f64::consts::PI * (2.0 * r + t);
        let total = footprint(w, d, r, t).total();
        assert!(
            (total - expected).abs() < 1e-9,
            "{w}x{d} r{r}: {total} vs {expected}"
        );
    }
}

#[test]
fn the_seam_is_the_middle_of_the_back_straight() {
    let fp = footprint(120.0, 80.0, 15.0, 3.0);
    assert_eq!(
        fp.spans.len(),
        9,
        "two seam halves, four bends, three straights"
    );
    let Shape::Straight { from, .. } = fp.spans[0].shape else {
        panic!("the strip starts on a straight");
    };
    assert!(
        from.x.abs() < 1e-9 && (from.y + 40.0).abs() < 1e-9,
        "{from:?}"
    );
    let tall = footprint(40.0, 100.0, 20.0, 3.0);
    let Shape::Straight { from, .. } = tall.spans[0].shape else {
        panic!("a right straight");
    };
    assert!(
        (from.x - 20.0).abs() < 1e-9 && from.y.abs() < 1e-9,
        "the longer side: {from:?}"
    );
}

#[test]
fn a_stadium_has_two_straights_and_two_half_circles() {
    let fp = footprint(100.0, 40.0, 20.0, 3.0);
    let bends: Vec<f64> = fp
        .spans
        .iter()
        .filter_map(|span| match span.shape {
            Shape::Bend { sweep, .. } => Some(sweep),
            Shape::Straight { .. } => None,
        })
        .collect();
    assert_eq!(bends.len(), 2);
    for sweep in bends {
        assert!((sweep - std::f64::consts::PI).abs() < 1e-9, "{sweep}");
    }
}

#[test]
fn a_circle_is_one_closed_bend_from_the_back_point() {
    let fp = footprint(60.0, 60.0, 29.995, 3.0);
    assert_eq!(fp.spans.len(), 1, "no straight, no zero-length piece");
    let Shape::Bend {
        centre,
        radius,
        from,
        sweep,
    } = fp.spans[0].shape
    else {
        panic!("a bend");
    };
    assert!(centre.x.abs() < 1e-9 && centre.y.abs() < 1e-9);
    assert!((radius - 30.0).abs() < 1e-9, "snapped to half the side");
    assert!((from + std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    assert!((sweep - std::f64::consts::TAU).abs() < 1e-9);
}

#[test]
fn tabs_sit_in_equal_sub_spans_and_the_seam_falls_between_two_tabs() {
    let fp = footprint(120.0, 80.0, 15.0, 3.0);
    let tabs = fp.straight_tabs(3.0, &crate::joint_layout::AUTO);
    // Top 90 mm: 3 rounds up to the even 4; right 50: 2; bottom 90: 3; left 50: 2.
    assert_eq!(tabs.len(), 4 + 2 + 3 + 2);
    let total = fp.total();
    let seam_gap = tabs
        .iter()
        .map(|tab| tab.s.min(total - tab.s))
        .fold(f64::MAX, f64::min);
    assert!(
        (seam_gap - 11.25).abs() < 1e-9,
        "half a pitch from the seam: {seam_gap}"
    );
    for tab in &tabs {
        assert!(tab.s > 0.0 && tab.s < total);
    }
}

#[test]
fn a_short_straight_has_no_tab() {
    // 40 x 28 with r 10: straights of 20 and 8 mm; wall 3 -> 8 < 4 * 3 has none.
    let fp = footprint(40.0, 28.0, 10.0, 3.0);
    let tabs = fp.straight_tabs(3.0, &crate::joint_layout::AUTO);
    assert!(
        tabs.iter()
            .all(|tab| (tab.centre.y.abs() - 14.0).abs() < 1e-9)
    );
}
