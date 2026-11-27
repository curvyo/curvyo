//! Criteria 23 to 30 of `specs/0144-qr-and-barcodes` (slice 4, ADR 0091 "Per-part paint" P3, P8, P9):
//! the Background and the Finder colour as parts with their own paint, goldens of every part in
//! draw order with its fill, and the decoder reading the ink parts on the Background's ground.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_lines
)]

mod support;

use curvyo_codes_core::{EcLevel, encode, encode_data_matrix};
use curvyo_document_core::{Color, PartPaint};
use curvyo_extension_core::{Edit, Generated, ParamValue};
use rxing::BarcodeFormat;
use support::glyphs::test_table;
use support::{
    assert_golden_parts, decode, generate_kind, ink_paths, params_of, raster, raster_window, text,
};

const PER_MODULE: usize = 10;
const RED: Color = Color { r: 200, g: 0, b: 0 };
const CREAM: Color = Color {
    r: 250,
    g: 240,
    b: 200,
};

fn on(key: &'static str) -> (&'static str, ParamValue) {
    (key, ParamValue::Switch(true))
}

fn colour(key: &'static str, colour: Color) -> (&'static str, ParamValue) {
    (key, ParamValue::Colour(colour))
}

fn qr(overrides: &[(&str, ParamValue)]) -> Generated {
    generate_kind("qr", &params_of("qr", overrides), None, None)
}

fn keys(generated: &Generated) -> Vec<&str> {
    generated.parts.iter().map(|p| p.key.as_ref()).collect()
}

#[test]
fn with_both_off_the_output_is_the_one_unpainted_part_of_slices_one_to_three() {
    for kind in [
        "qr",
        "data-matrix",
        "ean-13",
        "code-128",
        "code-39",
        "ean-8",
        "upc-a",
    ] {
        let table = test_table();
        let generated = generate_kind(kind, &params_of(kind, &[]), None, Some(&table));
        assert_eq!(keys(&generated), ["code"], "{kind}");
        assert_eq!(generated.parts[0].paint, None, "{kind}");
    }
}

#[test]
fn background_and_finder_colour_are_parts_in_draw_order_with_their_paints() {
    let generated = qr(&[
        on("background"),
        colour("background_colour", CREAM),
        on("finder_colour_on"),
        colour("finder_colour", RED),
    ]);
    assert_eq!(keys(&generated), ["background", "code", "finder"]);
    assert_eq!(
        generated.parts[0].paint,
        Some(PartPaint {
            fill: CREAM,
            job: false
        })
    );
    assert_eq!(generated.parts[1].paint, None);
    assert_eq!(
        generated.parts[2].paint,
        Some(PartPaint {
            fill: RED,
            job: true
        })
    );
    // The finders leave the code part; together they are what one part was.
    let plain = qr(&[]);
    let joined = generated.parts[1].paths.len() + generated.parts[2].paths.len();
    assert_eq!(joined, plain.parts[0].paths.len());
    assert_eq!(
        generated.parts[2].paths.len(),
        9,
        "three finders of three outlines"
    );
    // The Background is one rectangle of the symbol and its quiet zone (4 modules each side).
    let ground = &generated.parts[0].paths;
    assert_eq!(ground.len(), 1);
    assert_eq!(ground[0].segments.len(), 4);
}

#[test]
fn an_inverted_code_ignores_the_finder_colour_and_keeps_the_background_behind_its_ground() {
    let generated = qr(&[
        on("inverted"),
        on("background"),
        on("finder_colour_on"),
        colour("finder_colour", RED),
    ]);
    assert_eq!(keys(&generated), ["background", "code"]);
    assert!(generated.parts[1].paint.is_none());
}

#[test]
fn the_finder_colour_rows_hide_while_inverted_and_the_colour_rows_follow_their_switches() {
    let schema = support::schema();
    let hold = |key: &str, stored: &[(&str, ParamValue)]| {
        let params = params_of("qr", stored);
        schema
            .iter()
            .find(|p| p.key == key)
            .unwrap()
            .shown_when
            .is_none_or(|when| when.holds(&schema, &params))
    };
    assert!(!hold("finder_colour", &[]));
    assert!(hold("finder_colour", &[on("finder_colour_on")]));
    assert!(!hold(
        "finder_colour",
        &[on("finder_colour_on"), on("inverted")]
    ));
    assert!(!hold("finder_colour_on", &[on("inverted")]));
    assert!(!hold("background_colour", &[]));
    assert!(hold("background_colour", &[on("background")]));
    // Data Matrix and the 1D kinds have a Background and no Finder colour (criterion 30).
    for kind in ["data-matrix", "ean-13", "code-128"] {
        let keys: Vec<&str> = support::schema_of(kind).iter().map(|p| p.key).collect();
        assert!(
            keys.contains(&"background") && keys.contains(&"background_colour"),
            "{kind}"
        );
        assert!(!keys.contains(&"finder_colour_on"), "{kind}");
    }
}

#[test]
fn a_colour_row_stores_as_rgb_and_a_bad_one_reads_as_its_default() {
    let generated = qr(&[on("background"), colour("background_colour", CREAM)]);
    assert_eq!(
        generated.params["background_colour"],
        serde_json::json!({"rgb": [250, 240, 200]})
    );
    let mut params = params_of("qr", &[on("background")]);
    params.insert(
        "background_colour".into(),
        serde_json::json!({"rgb": [1, 2]}),
    );
    let read = generate_kind("qr", &params, None, None);
    assert_eq!(
        read.parts[0].paint.unwrap().fill,
        Color::WHITE,
        "a damaged colour reads as white"
    );
}

#[test]
fn the_ghost_under_the_pointer_draws_neither_part() {
    let params = params_of("qr", &[on("background"), on("finder_colour_on")]);
    let ghost = generate_kind("qr", &params, Some(Edit::Draft), None);
    assert_eq!(keys(&ghost), ["code"]);
}

// ---- goldens (P8): one path per part in draw order with its fill --------------------------------

#[test]
fn golden_qr_background_and_finder() {
    let generated = qr(&[
        ("content", text("Hello")),
        on("background"),
        colour("background_colour", CREAM),
        on("finder_colour_on"),
        colour("finder_colour", RED),
    ]);
    assert_golden_parts("qr_background_finder.svg", &generated);
}

#[test]
fn golden_ean13_background() {
    let table = test_table();
    let params = params_of(
        "ean-13",
        &[("content", text("4006381333931")), on("background")],
    );
    let generated = generate_kind("ean-13", &params, None, Some(&table));
    assert_golden_parts("ean13_background.svg", &generated);
}

#[test]
fn golden_data_matrix_10_background() {
    let params = params_of(
        "data-matrix",
        &[("content", text("123456")), on("background")],
    );
    let generated = generate_kind("data-matrix", &params, None, None);
    assert_golden_parts("datamatrix_10_background.svg", &generated);
}

// ---- the decoder (P9) ------------------------------------------------------------------------------

fn content_for(version: usize, level: EcLevel) -> String {
    (1..3000)
        .map(|n| {
            (0..n)
                .map(|i| char::from(b'a' + u8::try_from((i * 7 + i / 5) % 26).unwrap()))
                .collect::<String>()
        })
        .find(|text| encode(text, level).is_ok_and(|grid| grid.version() == version))
        .expect("a text for the version")
}

#[test]
fn a_qr_code_with_both_colours_reads_back_with_the_ink_parts_on_the_ground() {
    for version in [1, 3, 6] {
        let content = content_for(version, EcLevel::M);
        for extra in [
            vec![
                on("background"),
                on("finder_colour_on"),
                colour("finder_colour", RED),
            ],
            vec![on("background")],
            vec![
                on("background"),
                on("finder_colour_on"),
                colour("finder_colour", RED),
                ("module_shape", ParamValue::Choice("dot".into())),
            ],
        ] {
            let mut all = vec![("content", text(&content))];
            all.extend(extra);
            let generated = qr(&all);
            let image = raster(
                &ink_paths(&generated),
                1.0,
                17 + 4 * version,
                4,
                PER_MODULE,
                false,
            );
            assert_eq!(
                decode(image).as_deref(),
                Some(content.as_str()),
                "version {version}"
            );
        }
    }
}

#[test]
fn an_inverted_qr_code_with_a_background_reads_back_engraved() {
    let content = content_for(4, EcLevel::M);
    let generated = qr(&[
        ("content", text(&content)),
        on("inverted"),
        on("background"),
    ]);
    let image = raster(&ink_paths(&generated), 1.0, 17 + 16, 4, PER_MODULE, true);
    assert_eq!(decode(image).as_deref(), Some(content.as_str()));
}

#[test]
fn data_matrix_and_ean_13_with_a_background_read_back() {
    let params = params_of(
        "data-matrix",
        &[("content", text("123456")), on("background")],
    );
    let generated = generate_kind("data-matrix", &params, None, None);
    let width = encode_data_matrix("123456").unwrap().width();
    let (w, h, pixels) = raster(&ink_paths(&generated), 1.0, width, 3, PER_MODULE, false);
    let read = rxing::helpers::detect_in_luma(
        pixels,
        w as u32,
        h as u32,
        Some(BarcodeFormat::DATA_MATRIX),
    )
    .unwrap();
    assert_eq!(read.getText(), "123456");

    let table = test_table();
    let params = params_of(
        "ean-13",
        &[("content", text("4006381333931")), on("background")],
    );
    let generated = generate_kind("ean-13", &params, None, Some(&table));
    let ink = ink_paths(&generated);
    let module = 0.33;
    let (high, low) = ink
        .iter()
        .flat_map(|p| p.segments.iter().map(|s| s.to.y))
        .fold((f64::MAX, f64::MIN), |(h, l), y| (h.min(y), l.max(y)));
    let scale = PER_MODULE as f64 / module;
    let width = ((11.0 + 95.0 + 7.0 + 4.0) * PER_MODULE as f64) as usize;
    let height = ((low - high) * scale + 4.0 * PER_MODULE as f64).ceil() as usize;
    let shift = (
        (95.0 / 2.0 + 11.0 + 2.0) * PER_MODULE as f64,
        -high * scale + 2.0 * PER_MODULE as f64,
    );
    let (w, h, pixels) = raster_window(&ink, scale, shift, (width, height), false);
    let read =
        rxing::helpers::detect_in_luma(pixels, w as u32, h as u32, Some(BarcodeFormat::EAN_13))
            .unwrap();
    assert_eq!(read.getText(), "4006381333931");
}

#[test]
fn the_background_makes_the_box_the_symbol_plus_the_quiet_zone() {
    // With the Background on the outline's width is Width, the symbol and both quiet zones.
    let with = qr(&[on("background")]);
    let all: Vec<f64> = with
        .parts
        .iter()
        .flat_map(|p| p.paths.iter())
        .flat_map(|p| p.segments.iter().map(|s| s.to.x))
        .collect();
    let right = all.iter().copied().fold(f64::MIN, f64::max);
    let left = all.iter().copied().fold(f64::MAX, f64::min);
    let span = right - left;
    let asked = ParamValue::from_stored(&with.params["width"])
        .unwrap()
        .length()
        .unwrap()
        .as_mm();
    assert!((span - asked).abs() < 1e-6, "{span} against {asked}");
}
