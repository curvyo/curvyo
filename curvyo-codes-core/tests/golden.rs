//! Criterion 19: golden SVGs of the styled output (`tests/fixtures/*.svg`, one path with the
//! non-zero rule, compared number by number within 0.001 mm; `UPDATE_GOLDEN=1` rewrites them).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_cmp,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::len_zero,
    clippy::format_push_string,
    clippy::too_many_lines,
    clippy::similar_names,
    clippy::needless_pass_by_value
)]

mod support;

use curvyo_extension_core::{Edit, ParamValue};
use support::{assert_golden, choice, generate, generate_from, params, paths, text};

#[test]
fn qr_v1_square_merged() {
    assert_golden(
        "qr_v1_square_merged.svg",
        paths(&generate(&[("content", text("Hello"))])),
    );
}

#[test]
fn qr_v3_rounded_with_a_circular_clear_area_at_level_h() {
    let asked = params(&[
        ("content", text("https://example.com/qr")),
        ("module_shape", choice("rounded")),
        ("finder_outer", choice("rounded")),
        ("logo", ParamValue::Switch(true)),
    ]);
    let generated = generate_from(&asked, Some(Edit::Param("logo")));
    assert_eq!(generated.params["ec"], serde_json::json!({"choice": "h"}));
    assert_golden("qr_v3_rounded_logo_circle_h.svg", paths(&generated));
}

#[test]
fn qr_dot_separate() {
    assert_golden(
        "qr_dot_separate.svg",
        paths(&generate(&[
            ("content", text("Hello")),
            ("module_shape", choice("dot")),
            ("module_fill", ParamValue::Percent(85)),
        ])),
    );
}

#[test]
fn qr_inverted_merged() {
    assert_golden(
        "qr_inverted_merged.svg",
        paths(&generate(&[
            ("content", text("Hello")),
            ("inverted", ParamValue::Switch(true)),
        ])),
    );
}
