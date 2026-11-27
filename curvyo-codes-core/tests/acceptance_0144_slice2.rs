//! The numbers of the slice 2 criteria 3, 11, 14, 21 and 22 at the generator (ADR 0092): defaults,
//! widths, quiet zones, normalisation, refusals, text, Bar width adjust, Inverted and the two
//! resize edits. The host rules (switch, handles, sentences) are tested in `curvyo-ui-core`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::assert_is_empty,
    clippy::cast_precision_loss,
    clippy::too_many_lines
)]

mod support;

use curvyo_codes_core::{CODES, Symbology};
use curvyo_extension_core::{
    ClosedPath, Edit, Extension, Generated, ParamValue, ScaleFactor, StoredParams,
};
use support::glyphs::test_table;
use support::{generate_kind, params_of, text};

fn make(
    symbology: Symbology,
    overrides: &[(&str, ParamValue)],
    edit: Option<Edit<'_>>,
) -> Generated {
    let params = params_of(symbology.id(), overrides);
    generate_kind(symbology.id(), &params, edit, Some(&test_table()))
}

fn mm(generated: &Generated, key: &str) -> f64 {
    ParamValue::from_stored(&generated.params[key])
        .unwrap()
        .length()
        .unwrap()
        .as_mm()
}

fn stored_text(generated: &Generated, key: &str) -> String {
    ParamValue::from_stored(&generated.params[key])
        .unwrap()
        .text()
        .unwrap()
        .to_owned()
}

fn bounds(paths: &[ClosedPath]) -> (f64, f64, f64, f64) {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for path in paths {
        for (x, y) in std::iter::once((path.start.x, path.start.y))
            .chain(path.segments.iter().map(|s| (s.to.x, s.to.y)))
        {
            b = (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y));
        }
    }
    b
}

fn warning_ids(generated: &Generated) -> Vec<&'static str> {
    generated.warnings.iter().map(|w| w.id).collect()
}

#[test]
fn the_kinds_are_one_family_and_only_the_1d_kinds_resize_freely() {
    let manifest = CODES.manifest();
    let kinds: Vec<_> = manifest.generators().collect();
    let ids: Vec<&str> = kinds.iter().map(|k| k.id).collect();
    assert_eq!(
        ids,
        [
            "qr",
            "data-matrix",
            "code-128",
            "code-39",
            "ean-13",
            "ean-8",
            "upc-a"
        ]
    );
    assert!(kinds.iter().all(|k| k.family.is_some() && k.scanned));
    // The 2D codes (QR, Data Matrix, slice 3) resize proportionally.
    assert!(
        kinds[..2]
            .iter()
            .all(|k| matches!(k.resize, curvyo_extension_core::Resize::Proportional))
    );
    assert!(
        kinds[2..]
            .iter()
            .all(|k| matches!(k.resize, curvyo_extension_core::Resize::Free))
    );
}

#[test]
fn the_defaults_give_the_widths_of_the_ux_table() {
    for (symbology, width) in [
        (Symbology::Code128, 54.45),
        (Symbology::Code39, 69.63),
        (Symbology::Ean13, 37.29),
        (Symbology::Ean8, 26.73),
        (Symbology::UpcA, 37.29),
    ] {
        let generated = make(symbology, &[], None);
        assert!(
            (mm(&generated, "width") - width).abs() < 0.001,
            "{symbology:?}: {}",
            mm(&generated, "width")
        );
        assert!((mm(&generated, "module_width") - 0.33).abs() < 1e-9);
        assert!((mm(&generated, "bar_height") - 15.0).abs() < 1e-9);
        let on = ParamValue::from_stored(&generated.params["text"])
            .unwrap()
            .switch()
            .unwrap();
        assert_eq!(on, symbology.text_default(), "{symbology:?}");
    }
}

#[test]
fn width_writes_the_module_width_by_the_modules_and_the_quiet_zones() {
    let mut params = params_of("ean-13", &[]);
    params.insert(
        "width".into(),
        ParamValue::Length(curvyo_document_core::Length::from_mm(113.0)).to_stored(),
    );
    let generated = generate_kind(
        "ean-13",
        &params,
        Some(Edit::Param("width")),
        Some(&test_table()),
    );
    // 95 + 11 + 7 = 113 modules.
    assert!((mm(&generated, "module_width") - 1.0).abs() < 1e-9);
    assert!((mm(&generated, "width") - 113.0).abs() < 1e-9);
}

#[test]
fn check_digits_are_appended_silently_and_a_wrong_one_is_corrected_with_a_notice() {
    let short = make(
        Symbology::Ean13,
        &[("content", text("400638133393"))],
        Some(Edit::Param("content")),
    );
    assert_eq!(stored_text(&short, "content"), "4006381333931");
    assert!(warning_ids(&short).is_empty());
    let wrong = make(
        Symbology::Ean13,
        &[("content", text("4006381333930"))],
        Some(Edit::Param("content")),
    );
    assert_eq!(stored_text(&wrong, "content"), "4006381333931");
    assert_eq!(warning_ids(&wrong), ["check_digit_corrected"]);
    assert!(wrong.warnings[0].value.is_none(), "no digit in the notice");
    // The same for EAN-8 and UPC-A; a plain regeneration says nothing.
    let upc = make(
        Symbology::UpcA,
        &[("content", text("036000291453"))],
        Some(Edit::Param("content")),
    );
    assert_eq!(stored_text(&upc, "content"), "036000291452");
    assert_eq!(warning_ids(&upc), ["check_digit_corrected"]);
    let again = make(Symbology::UpcA, &[("content", text("036000291453"))], None);
    assert!(warning_ids(&again).is_empty());
    assert_eq!(
        stored_text(&again, "content"),
        "036000291453",
        "a plain regeneration lets the stored truth win"
    );
}

#[test]
fn content_is_normalised_at_commit_without_a_notice() {
    let ean = make(
        Symbology::Ean13,
        &[("content", text("4 006381-333931"))],
        Some(Edit::Param("content")),
    );
    assert_eq!(stored_text(&ean, "content"), "4006381333931");
    assert!(ean.warnings.is_empty());
    let code39 = make(
        Symbology::Code39,
        &[("content", text("curvyo"))],
        Some(Edit::Param("content")),
    );
    assert_eq!(stored_text(&code39, "content"), "CURVYO");
    let lines = make(
        Symbology::Code128,
        &[("content", text("ab\ncd"))],
        Some(Edit::Param("content")),
    );
    assert_eq!(stored_text(&lines, "content"), "ab cd");
}

#[test]
fn invalid_content_keeps_no_parts_and_names_its_row_and_reason() {
    let cases: [(Symbology, &str, &str); 6] = [
        (Symbology::Ean13, "", "wrong_length"),
        (Symbology::Ean13, "12345", "wrong_length"),
        (Symbology::Ean13, "20012345678x", "digits_only"),
        (Symbology::Code128, "", "content_empty"),
        (Symbology::Code39, "Hello*", "not_encodable"),
        (Symbology::Code128, "caf\u{e9}", "not_encodable"),
    ];
    for (symbology, content, id) in cases {
        let generated = make(
            symbology,
            &[("content", text(content))],
            Some(Edit::Param("content")),
        );
        assert!(generated.parts.is_empty(), "{symbology:?} {content:?}");
        assert_eq!(warning_ids(&generated), [id]);
        assert_eq!(generated.warnings[0].param, Some("content"));
        assert_eq!(stored_text(&generated, "content"), content, "as typed");
    }
    let long = make(
        Symbology::Code128,
        &[("content", text(&"A".repeat(92)))],
        None,
    );
    assert!(long.parts.is_empty());
    assert_eq!(long.warnings[0].id, "too_many_characters");
    assert_eq!(long.warnings[0].value, Some(ParamValue::Count(92)));
    assert_eq!(long.warnings[0].limit, Some(ParamValue::Count(80)));
    let exactly = make(
        Symbology::Code39,
        &[("content", text(&"A".repeat(80)))],
        None,
    );
    assert!(!exactly.parts.is_empty());
    let eight = make(Symbology::Ean8, &[("content", text("123"))], None);
    assert_eq!(eight.warnings[0].value, Some(ParamValue::Count(7)));
    assert_eq!(eight.warnings[0].limit, Some(ParamValue::Count(8)));
    let code39 = make(Symbology::Code39, &[("content", text("a$"))], None);
    assert!(!code39.parts.is_empty());
    let bad = make(Symbology::Code39, &[("content", text("a_"))], None);
    assert_eq!(
        bad.warnings[0].subject,
        Some("A to Z, 0 to 9, space and - . $ / + %")
    );
}

#[test]
fn the_text_is_outlines_in_the_one_part_and_the_box_holds_bars_and_text() {
    let on = make(Symbology::Ean13, &[], None);
    let off = make(
        Symbology::Ean13,
        &[("text", ParamValue::Switch(false))],
        None,
    );
    assert_eq!(on.parts.len(), 1);
    assert_eq!(on.parts[0].key, "code");
    // 59 data and guard bars plus 13 digits of two outlines each (box and hole).
    let bars = off.parts[0].paths.len();
    assert_eq!(bars, 30, "EAN-13 has 30 bars");
    assert_eq!(on.parts[0].paths.len(), bars + 13 * 2);
    let (x0, y0, x1, y1) = bounds(&on.parts[0].paths);
    let module = 0.33;
    // The bars' centre is the origin; the box reaches the leading digit on the left.
    assert!((y0 + 7.5).abs() < 1e-9, "top of the bars: {y0}");
    assert!(
        x0 < -47.5 * module,
        "the leading digit stands in the quiet zone: {x0}"
    );
    assert!(x1 > 47.5 * module - 1e-9);
    // Digits' top 1 module below the bars, baseline at 3 mm * 0.7 below that.
    assert!(
        (y1 - (7.5 + module + 0.7 * 3.0)).abs() < 1e-6,
        "baseline: {y1}"
    );
    // With the text off the guard bars still reach 5 modules down.
    let (_, _, _, low) = bounds(&off.parts[0].paths);
    assert!((low - (7.5 + 5.0 * module)).abs() < 1e-9);
}

#[test]
fn code_128_and_code_39_print_the_content_centred_two_modules_under_the_bars() {
    let generated = make(
        Symbology::Code128,
        &[("text", ParamValue::Switch(true)), ("content", text("AB"))],
        None,
    );
    let (x0, _, x1, y1) = bounds(&generated.parts[0].paths);
    let module = 0.33;
    // The text top lies 2 modules below the bars (7.5), its baseline 0.7 * 3 mm further.
    assert!((y1 - (7.5 + 2.0 * module + 2.1)).abs() < 1e-6);
    let bars = bounds(&generated.parts[0].paths[..generated.parts[0].paths.len() - 4]);
    assert!(x0 <= bars.0 + 1e-9 && x1 >= bars.2 - 1e-9);
}

#[test]
fn the_digit_size_is_limited_by_the_module_width_for_ean_and_upc() {
    let generated = make(
        Symbology::Ean13,
        &[(
            "text_size",
            ParamValue::Length(curvyo_document_core::Length::from_mm(6.0)),
        )],
        None,
    );
    // 7 modules of 0.33 mm over an advance of 0.6 per mm of size.
    let most = 7.0 * 0.33 / 0.6;
    assert!((mm(&generated, "text_size") - most).abs() < 1e-9);
    assert_eq!(generated.clamps.len(), 1);
    assert_eq!(generated.clamps[0].key, "text_size");
    // Code 128 text is not limited that way.
    let line = make(
        Symbology::Code128,
        &[
            ("text", ParamValue::Switch(true)),
            (
                "text_size",
                ParamValue::Length(curvyo_document_core::Length::from_mm(6.0)),
            ),
        ],
        None,
    );
    assert!(line.clamps.is_empty());
}

#[test]
fn bar_width_adjust_widens_the_bars_and_keeps_the_length() {
    let plain = make(Symbology::Code128, &[], None);
    let wide = make(
        Symbology::Code128,
        &[("bar_adjust", ParamValue::Percent(20))],
        None,
    );
    let (a, b) = (&plain.parts[0].paths, &wide.parts[0].paths);
    assert_eq!(a.len(), b.len());
    let width = |p: &ClosedPath| {
        let (x0, _, x1, _) = bounds(std::slice::from_ref(p));
        x1 - x0
    };
    for (plain, wide) in a.iter().zip(b) {
        assert!((width(wide) - width(plain) - 0.2 * 0.33).abs() < 1e-9);
    }
    let centre = |p: &ClosedPath| {
        let (x0, _, x1, _) = bounds(std::slice::from_ref(p));
        f64::midpoint(x0, x1)
    };
    assert!(
        (centre(&a[3]) - centre(&b[3])).abs() < 1e-9,
        "centred on each bar"
    );
    let signed = make(
        Symbology::Code128,
        &[("bar_adjust", ParamValue::Percent(-90))],
        None,
    );
    assert_eq!(signed.params["bar_adjust"], serde_json::json!({"pct": -30}));
    assert_eq!(signed.clamps[0].key, "bar_adjust");
}

#[test]
fn an_inverted_code_is_one_ground_with_the_bars_and_text_knocked_out() {
    let generated = make(
        Symbology::Ean13,
        &[("inverted", ParamValue::Switch(true))],
        None,
    );
    let plain = make(Symbology::Ean13, &[], None);
    assert_eq!(
        generated.parts[0].paths.len(),
        plain.parts[0].paths.len() + 1
    );
    let (x0, _, x1, _) = bounds(&generated.parts[0].paths[..1]);
    let module = 0.33;
    assert!(
        (x0 - (-47.5 - 11.0) * module).abs() < 1e-9,
        "left quiet zone: {x0}"
    );
    assert!(
        (x1 - (47.5 + 7.0) * module).abs() < 1e-9,
        "right quiet zone: {x1}"
    );
}

fn flat(generated: &Generated) -> Vec<Vec<f64>> {
    generated.parts[0]
        .paths
        .iter()
        .map(ClosedPath::flat)
        .collect()
}

#[test]
fn a_scale_writes_all_three_lengths_and_a_stretch_the_two_factors() {
    let base = make(Symbology::Ean13, &[], None);
    let scaled = make(Symbology::Ean13, &[], Some(Edit::Scale(ScaleFactor(2.0))));
    assert!((mm(&scaled, "module_width") - 0.66).abs() < 1e-9);
    assert!((mm(&scaled, "bar_height") - 30.0).abs() < 1e-9);
    assert!((mm(&scaled, "text_size") - 6.0).abs() < 1e-9);
    // A pure x stretch keeps the bar height and scales the module width and the text size.
    let wide = make(
        Symbology::Ean13,
        &[],
        Some(Edit::Stretch {
            x: ScaleFactor(1.5),
            y: ScaleFactor(1.0),
        }),
    );
    assert!((mm(&wide, "module_width") - 0.495).abs() < 1e-9);
    assert!((mm(&wide, "bar_height") - 15.0).abs() < 1e-9);
    assert!((mm(&wide, "text_size") - 4.5).abs() < 1e-9);
    // A pure y stretch writes the bar height from the box: box 15 + band, doubled, less the band.
    let (_, y0, _, y1) = bounds(&base.parts[0].paths);
    let band = (y1 - y0) - 15.0;
    let tall = make(
        Symbology::Ean13,
        &[],
        Some(Edit::Stretch {
            x: ScaleFactor(1.0),
            y: ScaleFactor(2.0),
        }),
    );
    assert!((mm(&tall, "bar_height") - ((y1 - y0) * 2.0 - band)).abs() < 1e-9);
    assert!((mm(&tall, "module_width") - 0.33).abs() < 1e-9);
    // The generator is a pure function: the same request, the same outlines.
    assert_eq!(flat(&base), flat(&make(Symbology::Ean13, &[], None)));
}

#[test]
fn a_resize_past_a_bound_is_clamped_in_the_house_form() {
    let tiny = make(
        Symbology::Code128,
        &[],
        Some(Edit::Scale(ScaleFactor(0.01))),
    );
    assert!((mm(&tiny, "module_width") - 0.1).abs() < 1e-9);
    assert!(tiny.clamps.iter().any(|c| c.key == "module_width"));
}

#[test]
fn the_ghost_draws_only_the_bars_that_name_the_symbology() {
    let ghost = make(Symbology::Ean13, &[], Some(Edit::Draft));
    assert_eq!(ghost.parts[0].paths.len(), 6, "three guard pairs");
    let ghost = make(Symbology::Code128, &[], Some(Edit::Draft));
    assert!(ghost.parts[0].paths.len() < 20);
    let _ = StoredParams::new();
}

#[test]
fn a_missing_glyph_leaves_its_cell_empty_and_warns() {
    let mut table = test_table();
    table.glyphs.remove(&'7');
    let params = params_of("ean-13", &[]);
    let generated = generate_kind("ean-13", &params, None, Some(&table));
    assert!(warning_ids(&generated).contains(&"glyph_missing"));
    assert!(!generated.parts.is_empty());
    // No face at all: the bars are drawn, the text is not, and the warning says so.
    let bare = generate_kind("ean-13", &params, None, None);
    assert_eq!(bare.parts[0].paths.len(), 30);
    assert!(warning_ids(&bare).contains(&"glyph_missing"));
}
