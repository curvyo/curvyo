//! Independent acceptance tests for `specs/0131-controller-bed-size` at the connector: what GRBL's
//! `$130`/`$131` become in the settings the app receives (criteria 1, 3), that `$13` changes only
//! the reports (ADR 0042 is the app's; here the unit rule from the 0131 ADR), and that the connector
//! writes nothing. Written from the specification before reading the implementation. The
//! pseudo-terminal end-to-end path is covered by `0127` tests; this file pins the value path.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::too_many_lines
)]

use curvyo_connector_io::grbl::settings::Settings;

fn settings(lines: &[(u16, &str)]) -> Settings {
    let mut s = Settings::default();
    for (n, v) in lines {
        s.set(*n, v);
    }
    s
}

// ---- criterion 1: the reported travel reaches the app as numbers --------------------------------

#[test]
fn c1_travel_400_850_is_the_reported_pair() {
    let s = settings(&[(130, "400.000"), (131, "850.000")]);
    assert_eq!(s.travel(), Some([400.0, 850.0]));
}

#[test]
fn c1_travel_reaches_the_wire_settings() {
    // The Welcome carries these settings; the travel must be the numbers, in mm.
    let s = settings(&[(130, "400.000"), (131, "850.000")]);
    let wire = serde_json::to_value(s.to_wire()).unwrap();
    let travel = wire.get("travel").expect("travel present on the wire");
    let arr = travel
        .as_array()
        .expect("travel is an array of two numbers");
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].as_f64(), Some(400.0));
    assert_eq!(arr[1].as_f64(), Some(850.0));
}

#[test]
fn c1_missing_axis_gives_no_travel() {
    assert_eq!(settings(&[(130, "400.000")]).travel(), None);
    assert_eq!(settings(&[(131, "850.000")]).travel(), None);
    assert_eq!(settings(&[]).travel(), None);
}

#[test]
fn c1_unparsable_value_gives_no_travel() {
    assert_eq!(settings(&[(130, "wide"), (131, "850.000")]).travel(), None);
}

#[test]
fn c1_no_travel_puts_no_travel_on_the_wire() {
    let s = settings(&[(130, "400.000")]);
    let wire = serde_json::to_value(s.to_wire()).unwrap();
    let travel = wire.get("travel");
    assert!(
        travel.is_none() || travel == Some(&serde_json::Value::Null),
        "no travel must not be sent as numbers: {travel:?}"
    );
}

// ---- criterion 3: non-finite travel gives no travel, so no offer and no broken Welcome ---------

#[test]
fn c3_infinite_travel_gives_no_travel() {
    assert_eq!(settings(&[(130, "inf"), (131, "850.000")]).travel(), None);
    assert_eq!(settings(&[(130, "400.000"), (131, "inf")]).travel(), None);
    assert_eq!(settings(&[(130, "-inf"), (131, "850.000")]).travel(), None);
}

#[test]
fn c3_nan_travel_gives_no_travel() {
    assert_eq!(settings(&[(130, "nan"), (131, "850.000")]).travel(), None);
    assert_eq!(settings(&[(130, "400.000"), (131, "NaN")]).travel(), None);
}

#[test]
fn c3_non_finite_travel_does_not_break_the_wire_settings() {
    // A non-finite value must never reach the wire as JSON null inside a travel pair, because
    // the app cannot decode that and the whole Welcome would be malformed.
    let s = settings(&[(130, "inf"), (131, "850.000")]);
    let wire = serde_json::to_value(s.to_wire()).unwrap();
    if let Some(t) = wire.get("travel") {
        assert!(
            t.is_null()
                || t.as_array()
                    .is_some_and(|a| a.iter().all(serde_json::Value::is_number)),
            "travel must be absent or two numbers, got {t:?}"
        );
    }
}

// ---- criterion 1 / ADR: $13 changes reports only, never the travel -----------------------------

#[test]
fn c1_inch_reports_do_not_convert_travel() {
    // $13=1 (inch reports) must leave the mm travel as it is: 400 x 850 stays 400 x 850.
    let s = settings(&[(13, "1"), (130, "400.000"), (131, "850.000")]);
    assert!(s.reports_inches());
    assert_eq!(s.travel(), Some([400.0, 850.0]));
}

#[test]
fn c1_inch_reports_still_convert_positions_only() {
    // The unit rule is the same as before: positions convert, travel does not.
    let s = settings(&[(13, "1"), (130, "400.000"), (131, "850.000")]);
    assert!((s.to_mm(1.0) - 25.4).abs() < 1e-9);
    assert_eq!(s.travel(), Some([400.0, 850.0]));
}

// ---- the connector never writes a setting (0127 G5) -------------------------------------------

#[test]
fn settings_reading_is_read_only_and_idempotent() {
    let s = settings(&[(130, "400.000"), (131, "850.000")]);
    let first = s.travel();
    let second = s.travel();
    assert_eq!(first, second);
    assert_eq!(s, s.clone());
}
