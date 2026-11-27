//! Slice 3 of `specs/0144-qr-and-barcodes` (criteria 3, 4, 12, 19): the Data Matrix sizes, the module
//! styles, the always solid function cells, the independent decoder (`rxing`, dev-only) and the golden.

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

use curvyo_codes_core::{EncodeError, encode_data_matrix};
use curvyo_extension_core::{Edit, Generated, ParamValue};
use rxing::BarcodeFormat;
use support::{assert_golden, choice, covered_area, generate_kind, params_of, paths, raster, text};

const PER_MODULE: usize = 10;

fn code(overrides: &[(&str, ParamValue)]) -> Generated {
    generate_kind(
        "data-matrix",
        &params_of("data-matrix", overrides),
        None,
        None,
    )
}

fn read(overrides: &[(&str, ParamValue)], content: &str, light_ink: bool) -> Option<String> {
    let mut all = vec![("content", text(content))];
    all.extend(overrides.iter().cloned());
    let generated = code(&all);
    assert!(!generated.parts.is_empty(), "{overrides:?} was refused");
    let width = encode_data_matrix(content).unwrap().width();
    let (w, h, pixels) = raster(paths(&generated), 1.0, width, 3, PER_MODULE, light_ink);
    rxing::helpers::detect_in_luma(pixels, w as u32, h as u32, Some(BarcodeFormat::DATA_MATRIX))
        .ok()
        .map(|result| result.getText().to_owned())
}

/// A text that needs exactly the symbol of `width` modules.
fn content_for(width: usize) -> String {
    (1..400)
        .map(|n| {
            (0..n)
                .map(|i| char::from(b'a' + u8::try_from((i * 7 + i / 5) % 26).unwrap()))
                .collect::<String>()
        })
        .find(|text| encode_data_matrix(text).is_ok_and(|grid| grid.width() == width))
        .expect("a text for the size")
}

#[test]
fn the_smallest_square_that_fits() {
    // Criterion 3: ECC 200 "123456" is 10 x 10.
    assert_eq!(encode_data_matrix("123456").unwrap().width(), 10);
    let widths: Vec<usize> = [1, 4, 10, 30, 80, 300, 1000, 1555]
        .iter()
        .map(|n| encode_data_matrix(&"a".repeat(*n)).unwrap().width())
        .collect();
    assert!(widths.windows(2).all(|w| w[0] <= w[1]), "{widths:?}");
    assert_eq!(widths[0], 10, "one letter: 10 x 10 ({widths:?})");
    assert_eq!(
        encode_data_matrix(&"a".repeat(1556)),
        Err(EncodeError::TooLong {
            bytes: 1556,
            limit: 1555
        })
    );
    assert_eq!(encode_data_matrix(""), Err(EncodeError::Empty));
}

#[test]
fn sizes_ten_to_thirty_two_read_back_in_every_module_style() {
    let styles: [(&str, Vec<(&str, ParamValue)>); 4] = [
        ("square merged", vec![]),
        ("rounded merged", vec![("module_shape", choice("rounded"))]),
        ("square separate", vec![("output", choice("separate"))]),
        (
            "dot",
            vec![
                ("module_shape", choice("dot")),
                ("module_fill", ParamValue::Percent(90)),
            ],
        ),
    ];
    for width in [10, 12, 14, 16, 18, 20, 22, 24, 26, 32] {
        let content = content_for(width);
        for (name, style) in &styles {
            assert_eq!(
                read(style, &content, false).as_deref(),
                Some(content.as_str()),
                "{width} x {width}, {name}"
            );
        }
    }
}

#[test]
fn the_default_content_and_an_inverted_code_read_back() {
    let default = "https://example.com";
    assert_eq!(read(&[], default, false).as_deref(), Some(default));
    assert_eq!(
        read(&[("inverted", ParamValue::Switch(true))], "123456", true).as_deref(),
        Some("123456"),
        "inverted, engraved light on dark"
    );
}

#[test]
fn width_is_the_symbol_plus_the_quiet_zone_and_the_box_is_the_symbol() {
    // Criterion 12: Quiet zone defaults to 1; Width = (modules + 2 x quiet) x Module size.
    let generated = code(&[("content", text("123456"))]);
    let value = |key: &str| {
        ParamValue::from_stored(&generated.params[key])
            .unwrap()
            .length()
            .unwrap()
            .as_mm()
    };
    assert_eq!(
        ParamValue::from_stored(&generated.params["quiet_zone"])
            .unwrap()
            .count(),
        Some(1)
    );
    assert!((value("width") - 12.0).abs() < 1e-9 && (value("module_size") - 1.0).abs() < 1e-9);
    let (mut low, mut high) = (f64::MAX, f64::MIN);
    for path in paths(&generated) {
        for s in &path.segments {
            low = low.min(s.to.x);
            high = high.max(s.to.x);
        }
    }
    assert!(
        (low + 5.0).abs() < 1e-9 && (high - 5.0).abs() < 1e-9,
        "{low} {high}"
    );
}

#[test]
fn the_l_finder_and_clock_stay_solid_whatever_the_module_style() {
    let grid = encode_data_matrix("123456").unwrap();
    let dark_function = (0..100)
        .filter(|at| grid.is_function(at % 10, at / 10) && grid.is_dark(at % 10, at / 10))
        .count();
    assert_eq!(
        dark_function,
        10 + 9 + 4 + 4,
        "L finder 19, clock 8 with the corner shared"
    );
    let dots = code(&[
        ("content", text("123456")),
        ("module_shape", choice("dot")),
        ("module_fill", ParamValue::Percent(60)),
    ]);
    // The dark cells of the function pattern are one square each (mm^2 = 1), whatever the data modules are.
    let all = covered_area(paths(&dots), 1.0, 400);
    let data = (0..100)
        .filter(|at| !grid.is_function(at % 10, at / 10) && grid.is_dark(at % 10, at / 10))
        .count() as f64;
    let dot = std::f64::consts::PI * 0.3 * 0.3;
    assert!(
        (all - (dark_function as f64 + data * dot)).abs() < 0.2,
        "{all}"
    );
}

#[test]
fn merged_square_covers_the_dark_modules_exactly() {
    let grid = encode_data_matrix("https://example.com").unwrap();
    let w = grid.width();
    let dark = (0..w * w).filter(|at| grid.is_dark(at % w, at / w)).count() as f64;
    let merged = code(&[]);
    assert!((covered_area(paths(&merged), 1.0, 600) - dark).abs() < 0.2);
}

#[test]
fn a_refused_content_names_the_limit_and_draws_nothing() {
    let refused = code(&[("content", text(&"a".repeat(1700)))]);
    assert_eq!(refused.parts.len(), 0);
    let warning = &refused.warnings[0];
    assert_eq!(warning.id, "content_too_long");
    assert_eq!(warning.limit, Some(ParamValue::Count(1555)));
    let empty = code(&[("content", text(""))]);
    assert_eq!(empty.parts.len(), 0);
    assert_eq!(empty.warnings[0].id, "content_empty");
}

#[test]
fn the_draft_is_the_square_and_the_l_and_no_data() {
    let params = params_of("data-matrix", &[]);
    let ghost = generate_kind("data-matrix", &params, Some(Edit::Draft), None);
    assert!(ghost.parts[0].paths.len() < 6);
}

#[test]
fn golden_data_matrix_10() {
    assert_golden(
        "datamatrix_10.svg",
        paths(&code(&[("content", text("123456"))])),
    );
}
