//! Tester acceptance cases for `specs/0121-print-and-cut` slice 1, the reading of a placed
//! sheet's measures (criteria 10 to 14, 16, 18; ADR 0081 §5), written from the specification
//! before the implementation was read. Black box through `read_print_and_cut` and the texts.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    missing_docs
)]

use std::collections::BTreeMap;

use curvyo_document_core::{Affine, Point, PrintAndCutMeasures, PrintAndCutRegister};
use curvyo_laser_core::OriginCorner;
use curvyo_laser_core::job::Frame;
use curvyo_laser_core::job::Start;
use curvyo_laser_core::print_and_cut::text::{
    apply_button, error_lines, not_usable, row_note, state_line, warning_text,
};
use curvyo_laser_core::print_and_cut::{
    Apply, FitState, PrintAndCut, RowKind, SheetInput, Warning, head_position_um, machine_mm,
    read_print_and_cut,
};

const MARKS: [(u8, Point); 4] = [
    (1, Point::new(10.0, 10.0)),
    (2, Point::new(200.0, 10.0)),
    (3, Point::new(200.0, 287.0)),
    (4, Point::new(10.0, 287.0)),
];

fn frame(corner: OriginCorner) -> Frame {
    Frame {
        corner,
        width: 600_000,
        height: 400_000,
    }
}

fn similarity(deg: f64, s: f64, tx: f64, ty: f64) -> Affine {
    let (sin, cos) = deg.to_radians().sin_cos();
    Affine::new(s * cos, s * sin, -s * sin, s * cos, tx, ty)
}

/// Measures in µm of `marks` placed by `fit` (sheet page-local to laser page-local), with
/// optional noise in mm per number.
fn measure(
    corner: OriginCorner,
    fit: Affine,
    marks: &[(u8, Point)],
    noise: &[(u8, f64, f64)],
) -> BTreeMap<u8, [i64; 2]> {
    marks
        .iter()
        .map(|&(n, c)| {
            let m = machine_mm(frame(corner), fit.apply_point(c));
            let (dx, dy) = noise
                .iter()
                .find(|e| e.0 == n)
                .map_or((0.0, 0.0), |e| (e.1, e.2));
            (n, head_position_um(m.x + dx, m.y + dy))
        })
        .collect()
}

fn read_with(
    marks: &[(u8, Point)],
    register: &PrintAndCutRegister,
    corner: OriginCorner,
    matrix: Affine,
    start: Start,
) -> PrintAndCut {
    read_print_and_cut(&SheetInput {
        marks,
        register,
        frame: frame(corner),
        matrix,
        sheet: (210.0, 297.0),
        start,
    })
}

fn measures(marks: BTreeMap<u8, [i64; 2]>) -> PrintAndCutRegister {
    PrintAndCutRegister::Measures(PrintAndCutMeasures {
        marks,
        ..PrintAndCutMeasures::default()
    })
}

fn read(fit: Affine, noise: &[(u8, f64, f64)]) -> PrintAndCut {
    let reg = measures(measure(OriginCorner::BottomLeft, fit, &MARKS, noise));
    read_with(
        &MARKS,
        &reg,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    )
}

fn summary(r: &PrintAndCut) -> &curvyo_laser_core::print_and_cut::FitSummary {
    match &r.fit {
        FitState::Fit(f) => f,
        other => panic!("no fit: {other:?}"),
    }
}

fn texts(r: &PrintAndCut) -> Vec<String> {
    r.warnings.iter().map(warning_text).collect()
}

// ---- criterion 10: not enough, gone, unreadable ----------------------------------------------

#[test]
fn c10_fewer_than_two_filled_rows_say_measure_at_least_2_and_grey_apply() {
    let fit = similarity(2.0, 1.0, 100.0, 50.0);
    let mut one = measure(OriginCorner::BottomLeft, fit, &MARKS, &[]);
    one.retain(|&n, _| n == 1);
    let r = read_with(
        &MARKS,
        &measures(one),
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert_eq!(r.fit, FitState::NotEnough);
    assert_eq!(state_line(&r), Some("Measure at least 2 marks"));
    assert_eq!(r.apply, Apply::Unavailable);
    assert_eq!(
        apply_button(&r),
        Some(("Apply", Some("Measure at least 2 marks")))
    );
    let absent = read_with(
        &MARKS,
        &PrintAndCutRegister::Absent,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert_eq!(absent.fit, FitState::NotEnough);
    assert_eq!(absent.rows.len(), 4, "a row per mark, in number order");
    assert_eq!(
        absent.rows.iter().map(|r| r.number).collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
}

#[test]
fn c10_a_row_whose_mark_is_gone_says_so_and_does_not_count() {
    let fit = similarity(2.0, 1.0, 100.0, 50.0);
    let all = measure(OriginCorner::BottomLeft, fit, &MARKS, &[]);
    let without_3: Vec<(u8, Point)> = MARKS.iter().copied().filter(|m| m.0 != 3).collect();
    let mut two_and_gone = all.clone();
    two_and_gone.remove(&4);
    let r = read_with(
        &without_3,
        &measures(two_and_gone),
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    let row3 = r.rows.iter().find(|row| row.number == 3).expect("row 3");
    assert_eq!(row3.kind, RowKind::Gone);
    assert_eq!(
        row_note(row3).as_deref(),
        Some("Mark 3 is gone from the sheet")
    );
    assert_eq!(summary(&r).pairs, 2, "mark 3 does not count");
    assert_eq!(
        error_lines(&r).unwrap().0,
        "2 marks: no error check",
        "criterion 11"
    );
}

#[test]
fn c10_an_unreadable_register_shows_the_text_and_empty_rows() {
    let r = read_with(
        &MARKS,
        &PrintAndCutRegister::Unreadable,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert!(r.unreadable);
    assert!(r.rows.iter().all(|row| row.measured.is_none()));
    assert_eq!(r.fit, FitState::NotEnough);
    assert_eq!(
        curvyo_laser_core::print_and_cut::text::UNREADABLE,
        "The stored measures could not be read."
    );
}

// ---- criteria 11, 12: the fit summary --------------------------------------------------------

#[test]
fn c11_c12_exact_fit_on_every_origin_corner_is_the_placement_and_not_mirrored() {
    let fit = similarity(1.8, 1.001, 83.4, 41.2);
    for corner in [
        OriginCorner::TopLeft,
        OriginCorner::TopRight,
        OriginCorner::BottomLeft,
        OriginCorner::BottomRight,
    ] {
        let reg = measures(measure(corner, fit, &MARKS, &[]));
        let r = read_with(&MARKS, &reg, corner, Affine::IDENTITY, Start::MachineOrigin);
        let s = summary(&r);
        // µm rounding of the measures moves the fit by below a micrometre.
        assert!(s.map.approx_eq(fit, 1e-3), "{corner:?}: {:?}", s.map);
        assert_eq!(s.pairs, 4);
        // µm rounding of the measures: residuals stay at or below about a micrometre.
        assert!(s.largest_mm < 2e-3, "{corner:?}: {}", s.largest_mm);
        assert!((s.scale_pct - 100.1).abs() < 1e-3, "{}", s.scale_pct);
        assert!((s.turned_deg.abs() - 1.8).abs() < 1e-3, "{}", s.turned_deg);
        let origin = machine_mm(frame(corner), fit.apply_point(Point::new(0.0, 0.0)));
        assert!(
            (s.moved_mm[0] - origin.x).abs() < 1e-2 && (s.moved_mm[1] - origin.y).abs() < 1e-2,
            "{corner:?}: moved {:?} vs the sheet's top-left corner at {origin:?}",
            s.moved_mm
        );
        assert!(
            !r.warnings.contains(&Warning::Mirrored),
            "{corner:?}: an origin-corner flip is no mirror"
        );
        assert!(!s.over_tolerance);
        assert!(matches!(r.apply, Apply::Offered { again: false }));
    }
}

#[test]
fn c12_turned_is_counter_clockwise_on_screen_positive() {
    // A turn of +5 degrees in the Affine sense (Y down) is clockwise on screen: shown -5.
    let r = read(similarity(5.0, 1.0, 100.0, 50.0), &[]);
    assert!(
        (summary(&r).turned_deg + 5.0).abs() < 1e-3,
        "{}",
        summary(&r).turned_deg
    );
}

// ---- criterion 13: tolerance ---------------------------------------------------------------

#[test]
fn c13_above_the_tolerance_names_the_worst_mark_and_still_offers_apply() {
    let r = read(similarity(0.5, 1.0, 100.0, 50.0), &[(3, 3.0, 0.0)]);
    let s = summary(&r);
    assert!(s.largest_mm > 0.5, "{}", s.largest_mm);
    assert!(s.over_tolerance);
    assert_eq!(s.worst_mark, 3);
    let (line, _) = error_lines(&r).unwrap();
    assert!(line.starts_with("Largest error "), "{line}");
    assert!(line.ends_with("; check mark 3"), "{line}");
    assert!(
        matches!(r.apply, Apply::Offered { .. }),
        "permit over refuse"
    );
    let quiet = read(similarity(0.5, 1.0, 100.0, 50.0), &[(3, 0.2, 0.0)]);
    assert!(
        !summary(&quiet).over_tolerance,
        "{}",
        summary(&quiet).largest_mm
    );
}

#[test]
fn c13_the_tolerance_is_the_stored_one() {
    let fit = similarity(0.5, 1.0, 100.0, 50.0);
    let marks = measure(OriginCorner::BottomLeft, fit, &MARKS, &[(3, 0.6, 0.0)]);
    let tight = PrintAndCutRegister::Measures(PrintAndCutMeasures {
        marks: marks.clone(),
        tolerance_um: 50,
        reset: None,
    });
    let loose = PrintAndCutRegister::Measures(PrintAndCutMeasures {
        marks,
        tolerance_um: 5000,
        reset: None,
    });
    let at = |reg: &PrintAndCutRegister| {
        read_with(
            &MARKS,
            reg,
            OriginCorner::BottomLeft,
            Affine::IDENTITY,
            Start::MachineOrigin,
        )
    };
    assert!(summary(&at(&tight)).over_tolerance);
    assert!(!summary(&at(&loose)).over_tolerance);
    assert_eq!(at(&tight).tolerance_um, 50);
}

// ---- criterion 14: scale, mirrored, same place, head position --------------------------------

#[test]
fn c14_scale_outside_98_to_102_percent_warns_and_permits() {
    for (s, warns) in [
        (0.979, true),
        (0.981, false),
        (1.0, false),
        (1.019, false),
        (1.021, true),
    ] {
        let r = read(similarity(0.0, s, 100.0, 50.0), &[]);
        let has = r.warnings.iter().any(|w| matches!(w, Warning::Scale(_)));
        assert_eq!(has, warns, "scale {s}: {:?}", r.warnings);
        assert!(matches!(r.apply, Apply::Offered { .. }), "scale {s}");
    }
    let r = read(similarity(0.0, 0.942, 100.0, 50.0), &[]);
    assert!(
        texts(&r).contains(&"Scale 94.20\u{a0}%: was the sheet printed at 100\u{a0}%?".to_owned()),
        "{:?}",
        texts(&r)
    );
}

#[test]
fn c16_scale_outside_50_to_200_percent_is_not_usable() {
    // A typing error: the measures say 31 %.
    let r = read(similarity(0.0, 0.31, 100.0, 50.0), &[]);
    assert!(matches!(r.apply, Apply::NotUsable(p) if (p - 31.0).abs() < 0.01));
    assert_eq!(
        not_usable(&r).as_deref(),
        Some("The fit is not usable: scale 31.00\u{a0}%.")
    );
    assert_eq!(apply_button(&r), None, "Apply is not offered");
    let r = read(similarity(0.0, 2.05, 10.0, 10.0), &[]);
    assert!(matches!(r.apply, Apply::NotUsable(_)), "{:?}", r.apply);
    for s in [0.51, 1.99] {
        let r = read(similarity(0.0, s, 10.0, 10.0), &[]);
        assert!(
            matches!(r.apply, Apply::Offered { .. }),
            "{s}: {:?}",
            r.apply
        );
    }
}

#[test]
fn c14_marks_measured_in_reverse_order_look_mirrored() {
    let fit = similarity(3.0, 1.0, 100.0, 50.0);
    let real = measure(OriginCorner::BottomLeft, fit, &MARKS, &[]);
    let swapped: BTreeMap<u8, [i64; 2]> =
        [(1, real[&4]), (2, real[&3]), (3, real[&2]), (4, real[&1])].into();
    let r = read_with(
        &MARKS,
        &measures(swapped),
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert!(r.warnings.contains(&Warning::Mirrored), "{:?}", r.warnings);
    assert!(texts(&r).contains(&"The marks look mirrored: check the numbers".to_owned()));
    // The best similarity of a reflected rectangle shrinks it far below 50 %: criterion 16's
    // "not usable" applies on top of the warning.
    assert!(
        matches!(r.apply, Apply::Offered { .. } | Apply::NotUsable(_)),
        "{:?}",
        r.apply
    );
}

#[test]
fn c14_a_mirrored_sheet_with_a_usable_scale_warns_and_permits_apply() {
    // A long thin triangle printed mirrored about its baseline: the similarity keeps the
    // baseline (scale near 100 %), the reflection fits exactly.
    let marks: [(u8, Point); 3] = [
        (1, Point::new(10.0, 10.0)),
        (2, Point::new(200.0, 10.0)),
        (3, Point::new(105.0, 30.0)),
    ];
    let mirrored: [(u8, Point); 3] = [
        (1, Point::new(10.0, 10.0)),
        (2, Point::new(200.0, 10.0)),
        (3, Point::new(105.0, -10.0)),
    ];
    let fit = similarity(0.0, 1.0, 100.0, 50.0);
    let reg = measures(measure(OriginCorner::BottomLeft, fit, &mirrored, &[]));
    let r = read_with(
        &marks,
        &reg,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert!(r.warnings.contains(&Warning::Mirrored), "{:?}", r.warnings);
    assert!(matches!(r.apply, Apply::Offered { .. }), "{:?}", r.apply);
}

#[test]
fn c11_collinear_marks_fit_and_do_not_warn_mirrored() {
    let line: [(u8, Point); 3] = [
        (1, Point::new(10.0, 100.0)),
        (2, Point::new(100.0, 100.0)),
        (3, Point::new(200.0, 100.0)),
    ];
    let fit = similarity(-4.0, 1.0, 90.0, 30.0);
    let reg = measures(measure(
        OriginCorner::BottomLeft,
        fit,
        &line,
        &[(2, 0.0, 0.15)],
    ));
    let r = read_with(
        &line,
        &reg,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert!(matches!(r.fit, FitState::Fit(_)));
    assert!(!r.warnings.contains(&Warning::Mirrored), "{:?}", r.warnings);
}

#[test]
fn c14_two_measures_within_1_mm_warn_same_place_and_permit() {
    let fit = similarity(0.0, 1.0, 100.0, 50.0);
    let mut m = measure(OriginCorner::BottomLeft, fit, &MARKS, &[]);
    let p2 = m[&2];
    m.insert(4, [p2[0] + 500, p2[1] + 400]);
    let r = read_with(
        &MARKS,
        &measures(m),
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert!(
        r.warnings.contains(&Warning::SamePlace(2, 4)),
        "{:?}",
        r.warnings
    );
    assert!(texts(&r).contains(&"Marks 2 and 4 are measured at the same place".to_owned()));
    // Mark 4 measured 270 mm from where it is halves the fit: not usable on top (criterion 16).
    assert!(
        matches!(r.apply, Apply::Offered { .. } | Apply::NotUsable(_)),
        "{:?}",
        r.apply
    );
    // Two marks that really are close on the sheet: the warning, and Apply still works.
    let close: [(u8, Point); 4] = [MARKS[0], MARKS[1], MARKS[2], (4, Point::new(200.4, 10.4))];
    let reg = measures(measure(OriginCorner::BottomLeft, fit, &close, &[]));
    let r = read_with(
        &close,
        &reg,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert!(
        r.warnings.contains(&Warning::SamePlace(2, 4)),
        "{:?}",
        r.warnings
    );
    assert!(matches!(r.apply, Apply::Offered { .. }), "{:?}", r.apply);
}

#[test]
fn c15_all_measures_within_1_mm_are_too_close() {
    let m: BTreeMap<u8, [i64; 2]> = [
        (1, [50_000, 50_000]),
        (2, [50_400, 50_000]),
        (3, [50_000, 50_600]),
    ]
    .into();
    let r = read_with(
        &MARKS,
        &measures(m),
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::MachineOrigin,
    );
    assert_eq!(r.fit, FitState::TooClose);
    assert_eq!(state_line(&r), Some("Marks too close to fit"));
    assert_eq!(r.apply, Apply::Unavailable);
}

#[test]
fn c14_start_is_head_position_warns_and_permits() {
    let fit = similarity(0.0, 1.0, 100.0, 50.0);
    let reg = measures(measure(OriginCorner::BottomLeft, fit, &MARKS, &[]));
    let r = read_with(
        &MARKS,
        &reg,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::HeadPosition,
    );
    assert!(r.warnings.contains(&Warning::HeadPosition));
    assert!(
        texts(&r).contains(&"Start is Head position: the cut will not follow the sheet".to_owned())
    );
    assert!(matches!(r.apply, Apply::Offered { .. }));
    let r = read_with(
        &MARKS,
        &reg,
        OriginCorner::BottomLeft,
        Affine::IDENTITY,
        Start::HomeThenOrigin,
    );
    assert!(!r.warnings.contains(&Warning::HeadPosition));
}

// ---- criteria 16, 18: on the fit, Reset, Apply again ----------------------------------------

#[test]
fn c16_c18_on_fit_greys_apply_and_a_hand_move_offers_apply_again() {
    let fit = similarity(1.0, 1.0, 100.0, 50.0);
    let marks = measure(OriginCorner::BottomLeft, fit, &MARKS, &[]);
    let earlier = Affine::translation(curvyo_document_core::Vec2::new(20.0, 0.0));
    let reg = PrintAndCutRegister::Measures(PrintAndCutMeasures {
        marks,
        tolerance_um: 500,
        reset: Some(earlier),
    });
    let on = read_with(
        &MARKS,
        &reg,
        OriginCorner::BottomLeft,
        summary(&read(fit, &[])).map,
        Start::MachineOrigin,
    );
    assert_eq!(on.apply, Apply::OnFit);
    assert_eq!(on.on_fit, Some(true));
    assert!(on.reset_offered, "Reset offered while on the fit");
    assert_eq!(
        apply_button(&on),
        Some(("Apply", Some("The sheet is on the fit")))
    );
    let moved = Affine::translation(curvyo_document_core::Vec2::new(0.5, 0.0)).compose(fit);
    let off = read_with(
        &MARKS,
        &reg,
        OriginCorner::BottomLeft,
        moved,
        Start::MachineOrigin,
    );
    assert_eq!(off.on_fit, Some(false));
    assert!(!off.reset_offered, "Reset only while on the fit");
    assert_eq!(off.apply, Apply::Offered { again: true });
    assert_eq!(apply_button(&off), Some(("Apply again", None)));
    assert_eq!(
        curvyo_laser_core::print_and_cut::text::DIFFERS,
        "Placement differs from the fit"
    );
}

// ---- criterion 8: head position rounding ----------------------------------------------------

#[test]
fn c8_head_position_rounds_to_a_micrometre() {
    assert_eq!(head_position_um(12.345_4, 7.000_6), [12_345, 7_001]);
    assert_eq!(head_position_um(-0.000_4, -3.210_06), [0, -3_210]);
}
