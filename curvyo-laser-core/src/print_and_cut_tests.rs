//! Tests of the Print and cut reading (ADR 0081 Verification; `specs/0121-print-and-cut`
//! criteria 10 to 14, 18).

use std::collections::BTreeMap;

use super::*;
use crate::machine_profile::OriginCorner;
use crate::print_and_cut_words as text;

/// Marks 1 to 4, 10 mm inside a 210 x 297 mm sheet, clockwise from the top-left.
const MARKS: [(u8, Point); 4] = [
    (1, Point::new(10.0, 10.0)),
    (2, Point::new(200.0, 10.0)),
    (3, Point::new(200.0, 287.0)),
    (4, Point::new(10.0, 287.0)),
];

fn frame(corner: OriginCorner) -> Frame {
    Frame {
        corner,
        width: 400_000,
        height: 300_000,
    }
}

fn fitted() -> Affine {
    let (s, c) = 1.8_f64.to_radians().sin_cos();
    let k = 1.001;
    Affine::new(k * c, k * s, -k * s, k * c, 83.4, 4.2)
}

/// The measures of `marks` placed by `map` on `frame`, in machine µm.
fn measured(map: Affine, frame: Frame, numbers: &[u8]) -> BTreeMap<u8, [i64; 2]> {
    numbers
        .iter()
        .map(|&n| {
            let centre = MARKS[usize::from(n - 1)].1;
            let m = machine_mm(frame, map.apply_point(centre));
            (n, head_position_um(m.x, m.y))
        })
        .collect()
}

fn register(marks: BTreeMap<u8, [i64; 2]>, reset: Option<Affine>) -> PrintAndCutRegister {
    PrintAndCutRegister::Measures(PrintAndCutMeasures {
        marks,
        tolerance_um: 500,
        reset,
    })
}

fn read(register: &PrintAndCutRegister, frame: Frame, matrix: Affine) -> PrintAndCut {
    read_print_and_cut(&SheetInput {
        marks: &MARKS,
        register,
        frame,
        matrix,
        sheet: (210.0, 297.0),
        start: Start::MachineOrigin,
    })
}

fn summary(reading: &PrintAndCut) -> &FitSummary {
    match &reading.fit {
        FitState::Fit(fit) => fit,
        other => panic!("{other:?}"),
    }
}

const IDENTITY: Affine = Affine::new(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);

#[test]
fn no_register_and_one_measure_ask_for_two() {
    let f = frame(OriginCorner::BottomLeft);
    let none = read(&PrintAndCutRegister::Absent, f, IDENTITY);
    assert_eq!(none.fit, FitState::NotEnough);
    assert_eq!(none.rows.len(), 4);
    assert_eq!(text::header_summary(&none), "Not measured");
    let one = read(&register(measured(fitted(), f, &[1]), None), f, IDENTITY);
    assert_eq!(text::state_line(&one), Some(text::MEASURE_AT_LEAST_TWO));
    assert_eq!(text::header_summary(&one), "1 of 4 measured");
    assert_eq!(
        text::apply_button(&one),
        Some(("Apply", Some(text::MEASURE_AT_LEAST_TWO)))
    );
}

#[test]
fn an_unreadable_register_shows_empty_rows() {
    let f = frame(OriginCorner::TopLeft);
    let r = read(&PrintAndCutRegister::Unreadable, f, IDENTITY);
    assert!(r.unreadable);
    assert!(r.rows.iter().all(|row| row.measured.is_none()));
    assert_eq!(r.tolerance_um, 500);
}

#[test]
fn every_origin_corner_recovers_the_fit_without_a_mirror() {
    for corner in OriginCorner::ALL {
        let f = frame(corner);
        let r = read(
            &register(measured(fitted(), f, &[1, 2, 3, 4]), None),
            f,
            IDENTITY,
        );
        let fit = summary(&r);
        assert!(fit.largest_mm < 0.001, "{corner:?}: {fit:?}");
        assert!(fit.map.approx_eq(fitted(), 1e-3));
        assert!(r.warnings.is_empty(), "{corner:?}: {:?}", r.warnings);
        assert!((fit.scale_pct - 100.1).abs() < 1e-3);
        assert!(
            (fit.turned_deg + 1.8).abs() < 1e-4,
            "counter-clockwise on screen"
        );
        let origin = machine_mm(f, Point::new(83.4, 4.2));
        assert!((fit.moved_mm[0] - origin.x).abs() < 1e-3);
        assert_eq!(r.apply, Apply::Offered { again: false });
    }
}

#[test]
fn two_marks_say_no_error_check() {
    let f = frame(OriginCorner::BottomLeft);
    let r = read(&register(measured(fitted(), f, &[1, 3]), None), f, IDENTITY);
    assert_eq!(
        text::error_lines(&r),
        Some((text::NO_ERROR_CHECK.to_owned(), None))
    );
}

#[test]
fn a_mark_gone_from_the_sheet_and_a_number_twice_do_not_count() {
    let f = frame(OriginCorner::BottomLeft);
    let marks = [MARKS[0], MARKS[1], MARKS[2], (2, Point::new(100.0, 100.0))];
    let mut m = measured(fitted(), f, &[1, 2, 3]);
    m.insert(4, [1, 1]);
    let reg = register(m, None);
    let r = read_print_and_cut(&SheetInput {
        marks: &marks,
        register: &reg,
        frame: f,
        matrix: IDENTITY,
        sheet: (210.0, 297.0),
        start: Start::MachineOrigin,
    });
    let kinds: Vec<(u8, RowKind)> = r.rows.iter().map(|row| (row.number, row.kind)).collect();
    assert_eq!(
        kinds,
        [
            (1, RowKind::Mark),
            (2, RowKind::Twice),
            (3, RowKind::Mark),
            (4, RowKind::Gone)
        ]
    );
    assert_eq!(
        text::row_note(&r.rows[3]).as_deref(),
        Some("Mark 4 is gone from the sheet")
    );
    assert_eq!(summary(&r).pairs, 2);
}

#[test]
fn measures_within_a_millimetre_are_too_close() {
    let f = frame(OriginCorner::TopLeft);
    let r = read(
        &register(
            BTreeMap::from([(1, [5_000, 5_000]), (2, [5_400, 5_300])]),
            None,
        ),
        f,
        IDENTITY,
    );
    assert_eq!(r.fit, FitState::TooClose);
    assert_eq!(text::header_summary(&r), "Too close");
    assert_eq!(
        text::apply_button(&r),
        Some(("Apply", Some(text::TOO_CLOSE)))
    );
}

#[test]
fn a_residual_above_the_tolerance_names_the_worst_mark_and_still_applies() {
    let f = frame(OriginCorner::TopLeft);
    let mut m = measured(fitted(), f, &[1, 2, 3, 4]);
    m.get_mut(&3).unwrap()[0] += 1_500;
    let r = read(&register(m, None), f, IDENTITY);
    let fit = summary(&r);
    assert!(fit.over_tolerance);
    assert_eq!(fit.worst_mark, 3);
    let (line, second) = text::error_lines(&r).unwrap();
    assert!(
        line.starts_with("Largest error ") && line.ends_with("; check mark 3"),
        "{line}"
    );
    assert!(second.unwrap().ends_with("tolerance 0.50\u{a0}mm"));
    assert_eq!(r.apply, Apply::Offered { again: false });
}

#[test]
fn scale_mirror_same_place_and_head_start_warn_and_permit() {
    let f = frame(OriginCorner::TopLeft);
    let shrunk = Affine::new(0.942, 0.0, 0.0, 0.942, 30.0, 0.0);
    let r = read(
        &register(measured(shrunk, f, &[1, 2, 3]), None),
        f,
        IDENTITY,
    );
    assert_eq!(r.warnings.len(), 1);
    assert_eq!(
        text::warning_text(&r.warnings[0]),
        "Scale 94.20\u{a0}%: was the sheet printed at 100\u{a0}%?"
    );
    let flipped = Affine::new(1.0, 0.0, 0.0, -1.0, 20.0, 299.0);
    let r = read(
        &register(measured(flipped, f, &[1, 2, 3, 4]), None),
        f,
        IDENTITY,
    );
    assert!(r.warnings.contains(&Warning::Mirrored), "{:?}", r.warnings);
    let mut m = measured(fitted(), f, &[1, 2, 3, 4]);
    m.insert(4, m[&2]);
    let r = read(&register(m, None), f, IDENTITY);
    assert!(r.warnings.contains(&Warning::SamePlace(2, 4)));
    let reg = register(measured(fitted(), f, &[1, 2]), None);
    let r = read_print_and_cut(&SheetInput {
        marks: &MARKS,
        register: &reg,
        frame: f,
        matrix: IDENTITY,
        sheet: (210.0, 297.0),
        start: Start::HeadPosition,
    });
    assert_eq!(r.warnings, [Warning::HeadPosition]);
    assert_eq!(r.apply, Apply::Offered { again: false });
}

#[test]
fn collinear_marks_fit_and_do_not_warn() {
    let f = frame(OriginCorner::TopLeft);
    let marks = [
        (1, Point::new(10.0, 10.0)),
        (2, Point::new(100.0, 10.0)),
        (3, Point::new(200.0, 10.0)),
    ];
    let m: BTreeMap<u8, [i64; 2]> = marks
        .iter()
        .map(|&(n, c)| {
            let p = fitted().apply_point(c);
            (
                n,
                head_position_um(p.x, p.y + if n == 2 { 0.1 } else { 0.0 }),
            )
        })
        .collect();
    let reg = register(m, None);
    let r = read_print_and_cut(&SheetInput {
        marks: &marks,
        register: &reg,
        frame: f,
        matrix: IDENTITY,
        sheet: (210.0, 297.0),
        start: Start::MachineOrigin,
    });
    assert!(matches!(r.fit, FitState::Fit(_)));
    assert!(r.warnings.is_empty(), "{:?}", r.warnings);
}

#[test]
fn after_apply_the_sheet_is_on_the_fit_and_a_hand_move_offers_apply_again() {
    let f = frame(OriginCorner::BottomLeft);
    let reg = register(measured(fitted(), f, &[1, 2, 3]), Some(IDENTITY));
    let fit = summary(&read(&reg, f, IDENTITY)).map;
    let on = read(&reg, f, fit);
    assert_eq!(on.apply, Apply::OnFit);
    assert!(on.reset_offered);
    assert_eq!(on.on_fit, Some(true));
    let moved = Affine::translation(curvyo_document_core::Vec2::new(0.5, 0.0)).compose(fit);
    let off = read(&reg, f, moved);
    assert_eq!(off.apply, Apply::Offered { again: true });
    assert!(!off.reset_offered);
    assert_eq!(off.on_fit, Some(false));
    assert_eq!(text::apply_button(&off), Some(("Apply again", None)));
}

#[test]
fn a_fit_of_31_percent_is_not_usable() {
    let f = frame(OriginCorner::TopLeft);
    let tiny = Affine::new(0.31, 0.0, 0.0, 0.31, 10.0, 10.0);
    let r = read(&register(measured(tiny, f, &[1, 2, 3]), None), f, IDENTITY);
    assert_eq!(r.apply, Apply::NotUsable(31.0));
    assert_eq!(
        text::not_usable(&r).as_deref(),
        Some("The fit is not usable: scale 31.00\u{a0}%.")
    );
    assert_eq!(text::apply_button(&r), None);
}

#[test]
fn the_head_position_is_rounded_to_a_micrometre_and_numbers_never_read_minus_zero() {
    assert_eq!(head_position_um(12.345_4, -0.000_6), [12_345, -1]);
    assert_eq!(text::fixed(-0.001, 2), "0.00");
    assert_eq!(text::fixed(-1.256, 2), "-1.26");
}

/// UX review: a fit that is not usable shows no placement line, even after an Apply.
#[test]
fn a_fit_that_is_not_usable_shows_no_placement_line() {
    let f = frame(OriginCorner::TopLeft);
    let tiny = Affine::new(0.31, 0.0, 0.0, 0.31, 10.0, 10.0);
    let r = read(
        &register(measured(tiny, f, &[1, 2, 3]), Some(IDENTITY)),
        f,
        IDENTITY,
    );
    assert_eq!(r.apply, Apply::NotUsable(31.0));
    assert_eq!(r.on_fit, None);
    assert!(!r.reset_offered);
}

/// UX review: one number format, two decimals, a non-breaking space before the unit.
#[test]
fn every_number_has_two_decimals_and_a_non_breaking_space_before_its_unit() {
    assert_eq!(text::mm_text(36.0), "36.00\u{a0}mm");
    assert_eq!(text::percent_text(36.012), "36.01\u{a0}%");
    assert_eq!(text::pair_mm_text(83.4, -0.001), "83.40, 0.00\u{a0}mm");
    assert_eq!(text::degrees_text(1.8), "1.80°");
}
