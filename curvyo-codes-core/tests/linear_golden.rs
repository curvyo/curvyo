//! Criterion 3 and 19 for the 1D codes: the module counts and the bar patterns follow the
//! standards' tables (written here as literals, not taken from the encoder), and the SVG of each
//! symbology with the test glyph table is a golden fixture within 0.001 mm.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod support;

use curvyo_codes_core::Symbology;
use curvyo_codes_core::linear::encode;
use support::glyphs::test_table;
use support::{assert_golden, generate_kind, params_of, paths};

/// The bit string (1 = bar) a run list draws.
fn bits(runs: &[f64]) -> String {
    let mut out = String::new();
    for (index, run) in runs.iter().enumerate() {
        out.push_str(&(if index % 2 == 0 { "1" } else { "0" }).repeat(*run as usize));
    }
    out
}

#[test]
fn ean_13_4006381333931_is_the_pattern_of_the_standard() {
    let code = encode(Symbology::Ean13, "4006381333931", 3.0).unwrap();
    // 4 = LGLLGG: 0 L, 0 G, 6 L, 3 L, 8 G, 1 G; the right group is R: 3 3 3 9 3 1.
    let expected = [
        "101", "0001101", "0100111", "0101111", "0111101", "0001001", "0110011", "01010",
        "1000010", "1000010", "1000010", "1110100", "1000010", "1100110", "101",
    ]
    .concat();
    assert_eq!(bits(&code.runs()), expected);
    assert_eq!(expected.len(), 95);
    assert_eq!(code.fixed_quiet, Some((11.0, 7.0)));
}

#[test]
fn ean_8_and_upc_a_follow_the_standard() {
    let eight = encode(Symbology::Ean8, "96385074", 3.0).unwrap();
    let expected = [
        "101", "0001011", "0101111", "0111101", "0110111", "01010", "1001110", "1110010",
        "1000100", "1011100", "101",
    ]
    .concat();
    assert_eq!(bits(&eight.runs()), expected);
    assert_eq!(expected.len(), 67);
    assert_eq!(eight.fixed_quiet, Some((7.0, 7.0)));
    // 036000291452 is the EAN-13 0036000291452: parity 0 = LLLLLL.
    let upc = encode(Symbology::UpcA, "036000291452", 3.0).unwrap();
    let expected = [
        "101", "0001101", "0111101", "0101111", "0001101", "0001101", "0001101", "01010",
        "1101100", "1110100", "1100110", "1011100", "1001110", "1101100", "101",
    ]
    .concat();
    assert_eq!(bits(&upc.runs()), expected);
    assert_eq!(upc.fixed_quiet, Some((9.0, 9.0)));
}

#[test]
fn code_128_and_code_39_follow_the_standard() {
    // "A" in set B: Start B 211214, A (33) 111323, check (104 + 33) % 103 = 34 is 131123, stop 2331112.
    let a = encode(Symbology::Code128, "A", 3.0).unwrap();
    assert_eq!(
        a.runs(),
        [
            2.0, 1.0, 1.0, 2.0, 1.0, 4.0, 1.0, 1.0, 1.0, 3.0, 2.0, 3.0, 1.0, 3.0, 1.0, 1.0, 2.0,
            3.0, 2.0, 3.0, 3.0, 1.0, 1.0, 1.0, 2.0
        ]
    );
    assert_eq!(a.width, 11.0 * 3.0 + 13.0);
    let curvyo = encode(Symbology::Code128, "Curvyo", 3.0).unwrap();
    assert_eq!(curvyo.width, 101.0);
    // Code 39 "A" between the stars at 3 : 1: * is nwnnwnwnn, A is wnnnnwnnw, a gap of one.
    let code = encode(Symbology::Code39, "A", 3.0).unwrap();
    assert_eq!(
        code.runs(),
        [
            1.0, 3.0, 1.0, 1.0, 3.0, 1.0, 3.0, 1.0, 1.0, 1.0, // *
            3.0, 1.0, 1.0, 1.0, 1.0, 3.0, 1.0, 1.0, 3.0, 1.0, // gap, A
            1.0, 3.0, 1.0, 1.0, 3.0, 1.0, 3.0, 1.0, 1.0, // gap, *
        ]
    );
    assert_eq!(
        encode(Symbology::Code39, "CURVYO", 3.0).unwrap().width,
        127.0
    );
}

fn golden(symbology: Symbology, content: &str, name: &str) {
    let params = params_of(symbology.id(), &[("content", support::text(content))]);
    let table = test_table();
    let generated = generate_kind(symbology.id(), &params, None, Some(&table));
    assert_golden(name, paths(&generated));
}

#[test]
fn the_svg_fixtures_of_the_five_symbologies() {
    golden(Symbology::Ean13, "4006381333931", "ean13.svg");
    golden(Symbology::Ean8, "96385074", "ean8.svg");
    golden(Symbology::UpcA, "036000291452", "upca.svg");
    golden(Symbology::Code128, "Curvyo", "code128.svg");
    golden(Symbology::Code39, "CURVYO", "code39.svg");
}
