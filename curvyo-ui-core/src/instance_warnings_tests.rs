//! The sentences of the generators' warnings (`instance_warnings.rs`).

use super::*;
use curvyo_document_core::Length;
use curvyo_extension_core::{Degrees, ParamValue};

fn warning(id: &'static str, pieces: &[u32]) -> Warning {
    Warning {
        id,
        pieces: pieces.to_vec(),
        value: None,
        limit: None,
        param: None,
        subject: None,
    }
}

#[test]
fn one_and_several_sharp_corners_read_as_in_the_ux_notes() {
    let mut a = warning("sharp_corner", &[3, 4]);
    a.value = Some(ParamValue::Angle(Degrees(22.0)));
    assert_eq!(
        sentences(std::slice::from_ref(&a)),
        ["The corner between pieces 3 and 4 is very sharp: 22°."]
    );
    let b = warning("sharp_corner", &[6, 1]);
    assert_eq!(
        sentences(&[a, b]),
        ["2 corners are very sharp: between pieces 3 and 4, and 6 and 1."]
    );
}

#[test]
fn tight_bends_name_the_radius_and_the_limit() {
    let mut a = warning("tight_bend", &[2]);
    a.value = Some(ParamValue::Length(Length::from_mm(2.5)));
    a.limit = Some(ParamValue::Length(Length::from_mm(6.0)));
    assert_eq!(
        sentences(std::slice::from_ref(&a)),
        ["Piece 2 bends to 2.5 mm; below 6 mm the wall may crack."]
    );
    let mut b = warning("tight_bend", &[5]);
    b.limit = a.limit.clone();
    assert_eq!(
        sentences(&[a, b]),
        ["Pieces 2 and 5 bend below 6 mm; the wall may crack there."]
    );
}

#[test]
fn tabs_that_keep_their_window_say_how_many_and_why() {
    let mut one = warning("tab_window", &[]);
    one.value = Some(ParamValue::Count(1));
    assert_eq!(
        sentences(std::slice::from_ref(&one)),
        ["1 tab does not fit on one arc, so it stands in a flat window."]
    );
    let mut three = warning("tab_window", &[]);
    three.value = Some(ParamValue::Count(3));
    assert_eq!(
        sentences(&[three]),
        ["3 tabs do not fit on one arc, so they stand in flat windows."]
    );
}

#[test]
fn an_unknown_joint_pattern_says_even_is_used() {
    assert_eq!(
        sentences(&[warning("unknown_joint_pattern", &[])]),
        ["The Joint pattern is not known here; Even is used."]
    );
}

#[allow(clippy::fn_params_excessive_bools)]
fn with_context(one_piece: bool, lid: bool, seat: bool, hollow: bool) -> Context {
    Context {
        one_piece,
        lid,
        seat,
        hollow,
        wall: curvyo_document_core::Length::from_mm(3.0),
        label: "code",
    }
}

#[test]
fn a_rounded_cover_names_what_was_built_and_the_wall_thickness() {
    let mut w = warning("cover_rounded", &[]);
    w.value = Some(ParamValue::Length(Length::from_mm(3.0)));
    let said = |c| sentences_in(std::slice::from_ref(&w), c)[0].clone();
    assert_eq!(
        said(with_context(false, true, true, true)),
        "The notch is too narrow for the cover at 3 mm walls: the cover and lid are rounded there, without a finger hollow."
    );
    for c in [
        with_context(false, false, true, true),
        with_context(false, true, false, true),
    ] {
        assert_eq!(
            said(c),
            "The notch is too narrow for the cover at 3 mm walls: the cover is rounded there."
        );
    }
    assert_eq!(
        said(with_context(false, true, true, false)),
        "The notch is too narrow for the cover at 3 mm walls: the cover and lid are rounded there."
    );
}

fn tight_warning(piece: u32, radius: f64, limit: f64) -> Warning {
    let mut w = warning("tight_bend", &[piece]);
    w.value = Some(ParamValue::Length(Length::from_mm(radius)));
    w.limit = Some(ParamValue::Length(Length::from_mm(limit)));
    w
}

#[test]
fn one_piece_says_the_wall() {
    let one = with_context(true, true, true, true);
    assert_eq!(
        sentences_in(&[tight_warning(1, 2.5, 6.0)], one),
        ["The wall bends to 2.5 mm; below 6 mm it may crack."]
    );
    let mut sharp = warning("sharp_corner", &[1, 1]);
    sharp.value = Some(ParamValue::Angle(Degrees(22.0)));
    assert_eq!(
        sentences_in(&[sharp], one),
        ["The corner of the wall is very sharp: 22°."]
    );
}

#[test]
fn a_concave_stretch_has_its_own_sentence_and_limit() {
    let all = [
        tight_warning(2, 2.5, 6.0),
        tight_warning(5, 4.0, 6.0),
        tight_warning(3, 2.0, 3.0),
    ];
    assert_eq!(
        sentences_in(&all, Context::default()),
        [
            "Pieces 2 and 5 bend below 6 mm; the wall may crack there. Piece 3 bends inward to 2 mm; below 3 mm the wall may crack."
        ]
    );
    let two = [tight_warning(3, 2.0, 3.0), tight_warning(4, 1.0, 3.0)];
    assert_eq!(
        sentences_in(&two, Context::default()),
        ["Pieces 3 and 4 bend inward below 3 mm; the wall may crack there."]
    );
}

#[test]
fn a_bend_too_short_for_two_slits_has_its_own_sentence() {
    let mut w = warning("bend_too_short", &[2]);
    w.value = Some(ParamValue::Length(Length::from_mm(4.1)));
    assert_eq!(
        sentences(std::slice::from_ref(&w)),
        ["Piece 2 has a bend too short for two slits (4.1 mm); it may crack there."]
    );
    let mut v = warning("bend_too_short", &[5]);
    v.value = Some(ParamValue::Length(Length::from_mm(3.0)));
    assert_eq!(
        sentences(&[w, v]),
        ["Pieces 2 and 5 have a bend too short for two slits; they may crack there."]
    );
    let mut one = warning("bend_too_short", &[1]);
    one.value = Some(ParamValue::Length(Length::from_mm(2.0)));
    let single = Context {
        one_piece: true,
        ..Context::default()
    };
    assert_eq!(
        sentences_in(&[one], single),
        ["The wall has a bend too short for two slits (2 mm); it may crack there."]
    );
}
