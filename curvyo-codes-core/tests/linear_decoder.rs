//! Criterion 4 for the 1D codes: the outlines at 10 pixels per module are read back by the
//! independent decoder (`rxing`, dev-only). The decoder reads the whole bar width adjust range, so
//! it is no proof of that setting (ADR 0092 §1); the demo check is a phone scan.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod support;

use curvyo_codes_core::Symbology;
use curvyo_codes_core::linear::encode;
use curvyo_extension_core::{Generated, ParamValue};
use rxing::BarcodeFormat;
use support::glyphs::test_table;
use support::{generate_kind, params_of, raster_window, text};

const PER_MODULE: f64 = 10.0;

fn format(symbology: Symbology) -> BarcodeFormat {
    match symbology {
        Symbology::Code128 => BarcodeFormat::CODE_128,
        Symbology::Code39 => BarcodeFormat::CODE_39,
        Symbology::Ean13 => BarcodeFormat::EAN_13,
        Symbology::Ean8 => BarcodeFormat::EAN_8,
        Symbology::UpcA => BarcodeFormat::UPC_A,
    }
}

/// The text the decoder reads from the code `generated` of `symbology`, drawn at `module` mm.
fn read(
    symbology: Symbology,
    generated: &Generated,
    module: f64,
    light_ink: bool,
) -> Option<String> {
    let ratio = generated.params.get("wide_narrow").map_or(3.0, |v| {
        ParamValue::from_stored(v).and_then(|v| v.ratio()).unwrap()
    });
    let content = ParamValue::from_stored(&generated.params["content"]).unwrap();
    let code = encode(symbology, content.text().unwrap(), ratio).ok()?;
    let (left, right) = code.fixed_quiet.unwrap_or_else(|| {
        let q = ParamValue::from_stored(&generated.params["bar_quiet_zone"])
            .unwrap()
            .count()
            .unwrap();
        (f64::from(q), f64::from(q))
    });
    let paths = &generated.parts[0].paths;
    let low = paths
        .iter()
        .flat_map(|p| p.segments.iter().map(|s| s.to.y))
        .fold(f64::MIN, f64::max);
    let high = paths
        .iter()
        .flat_map(|p| p.segments.iter().map(|s| s.to.y))
        .fold(f64::MAX, f64::min);
    let scale = PER_MODULE / module;
    let width = ((left + code.width + right + 4.0) * PER_MODULE).ceil() as usize;
    let height = ((low - high) * scale + 4.0 * PER_MODULE).ceil() as usize;
    let shift = (
        (code.width / 2.0 + left + 2.0) * PER_MODULE,
        -high * scale + 2.0 * PER_MODULE,
    );
    let (w, h, pixels) = raster_window(paths, scale, shift, (width, height), light_ink);
    rxing::helpers::detect_in_luma(pixels, w as u32, h as u32, Some(format(symbology)))
        .ok()
        .map(|result| result.getText().to_owned())
}

fn code_of(symbology: Symbology, content: &str, extra: &[(&str, ParamValue)]) -> Generated {
    let mut overrides = vec![("content", text(content))];
    overrides.extend(extra.iter().cloned());
    let params = params_of(symbology.id(), &overrides);
    let table = test_table();
    let generated = generate_kind(symbology.id(), &params, None, Some(&table));
    assert!(!generated.parts.is_empty(), "{symbology:?} {content}");
    generated
}

fn module_of(generated: &Generated) -> f64 {
    ParamValue::from_stored(&generated.params["module_width"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm()
}

fn round_trip(symbology: Symbology, content: &str, expected: &str) {
    let generated = code_of(symbology, content, &[]);
    let module = module_of(&generated);
    assert_eq!(
        read(symbology, &generated, module, false).as_deref(),
        Some(expected),
        "{symbology:?} {content}"
    );
}

#[test]
fn the_default_contents_and_the_standards_examples_read_back() {
    round_trip(Symbology::Code128, "Curvyo", "Curvyo");
    round_trip(Symbology::Code128, "EXAMPLE-01", "EXAMPLE-01");
    round_trip(Symbology::Code39, "CURVYO", "CURVYO");
    round_trip(Symbology::Code39, "EXAMPLE-01", "EXAMPLE-01");
    round_trip(Symbology::Ean13, "4006381333931", "4006381333931");
    round_trip(Symbology::Ean13, "2001234567893", "2001234567893");
    round_trip(Symbology::Ean8, "96385074", "96385074");
    round_trip(Symbology::Ean8, "20123451", "20123451");
    round_trip(Symbology::UpcA, "036000291452", "036000291452");
    round_trip(Symbology::UpcA, "212345678909", "212345678909");
}

#[test]
fn a_missing_check_digit_is_appended_and_read_back() {
    round_trip(Symbology::Ean13, "400638133393", "4006381333931");
    round_trip(Symbology::Ean8, "9638507", "96385074");
    round_trip(Symbology::UpcA, "03600029145", "036000291452");
}

#[test]
fn every_symbol_value_of_code_128_reads_back() {
    // Set B: all 95 printable characters, in chunks that fit the 80 characters.
    let all: String = (' '..='~').collect();
    for chunk in all.chars().collect::<Vec<_>>().chunks(40) {
        let text: String = chunk.iter().collect();
        // A leading or trailing space survives the symbology but a decoder may trim; guard them.
        let padded = format!("A{text}Z");
        round_trip(Symbology::Code128, &padded, &padded);
    }
    // Set C: every pair 00 to 99, 40 pairs at a time.
    let pairs: Vec<String> = (0..100).map(|n| format!("{n:02}")).collect();
    for chunk in pairs.chunks(40) {
        let digits: String = chunk.concat();
        round_trip(Symbology::Code128, &digits, &digits);
    }
    // Switching both ways in the middle of a text.
    round_trip(Symbology::Code128, "AB12345678cd", "AB12345678cd");
    round_trip(Symbology::Code128, "12345", "12345");
}

#[test]
fn every_character_of_code_39_reads_back() {
    let all = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%";
    for chunk in all.chars().collect::<Vec<_>>().chunks(11) {
        let text: String = chunk.iter().collect();
        let padded = format!("A{text}Z");
        round_trip(Symbology::Code39, &padded, &padded);
    }
}

#[test]
fn code_39_reads_at_every_ratio_the_schema_allows() {
    for ratio in [2.0, 2.5, 3.0] {
        let generated = code_of(
            Symbology::Code39,
            "CURVYO",
            &[("wide_narrow", ParamValue::Ratio(ratio))],
        );
        let module = module_of(&generated);
        assert_eq!(
            read(Symbology::Code39, &generated, module, false).as_deref(),
            Some("CURVYO"),
            "ratio {ratio}"
        );
    }
}

#[test]
fn the_bar_width_adjust_range_still_reads_but_proves_nothing_about_a_scanner() {
    for adjust in [-30, -10, 10, 30] {
        for symbology in [Symbology::Code128, Symbology::Ean13] {
            let content = if symbology == Symbology::Ean13 {
                "2001234567893"
            } else {
                "EXAMPLE-01"
            };
            let generated = code_of(
                symbology,
                content,
                &[("bar_adjust", ParamValue::Percent(adjust))],
            );
            let module = module_of(&generated);
            assert_eq!(
                read(symbology, &generated, module, false).as_deref(),
                Some(content),
                "{symbology:?} adjust {adjust}"
            );
        }
    }
}

#[test]
fn an_inverted_code_reads_back_as_light_bars_on_a_dark_ground() {
    let generated = code_of(
        Symbology::Code128,
        "EXAMPLE-01",
        &[("inverted", ParamValue::Switch(true))],
    );
    let module = module_of(&generated);
    // Engraved on a dark material: the ink is light; the bars are the dark gaps.
    assert_eq!(
        read(Symbology::Code128, &generated, module, true).as_deref(),
        Some("EXAMPLE-01")
    );
}
