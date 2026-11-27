//! Criterion 4 and 8 of `specs/0144-qr-and-barcodes`: the styled output, rasterised at 10 pixels
//! per module, is read back by an independent decoder (`rqrr`). This test is the definition of
//! "scannable" for the module styles, the finders, Inverted and the logo clear area.

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

use curvyo_codes_core::{EcLevel, encode};
use curvyo_extension_core::{Edit, ParamValue};
use support::{choice, decode, generate, generate_from, params, paths, raster, text};

const PER_MODULE: usize = 10;

/// The shortest text of mixed bytes that needs exactly `version` at `level`.
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

fn round_trip(overrides: &[(&str, ParamValue)], content: &str, version: usize, light_ink: bool) {
    let generated = generate(overrides);
    assert!(!generated.parts.is_empty(), "{overrides:?} was refused");
    let width = 17 + 4 * version;
    let image = raster(paths(&generated), 1.0, width, 4, PER_MODULE, light_ink);
    assert_eq!(
        decode(image).as_deref(),
        Some(content),
        "version {version}, {overrides:?}"
    );
}

#[test]
fn every_version_to_ten_at_every_level_reads_back_in_every_module_style() {
    let styles: [(&str, Vec<(&str, ParamValue)>); 5] = [
        ("square merged", vec![]),
        ("rounded merged", vec![("module_shape", choice("rounded"))]),
        ("square separate", vec![("output", choice("separate"))]),
        ("dot", vec![("module_shape", choice("dot"))]),
        (
            "rounded 90 %",
            vec![
                ("module_shape", choice("rounded")),
                ("module_fill", ParamValue::Percent(90)),
            ],
        ),
    ];
    for version in 1..=10 {
        for level in EcLevel::ALL {
            let content = content_for(version, level);
            for (name, style) in &styles {
                let mut overrides = vec![("content", text(&content)), ("ec", choice(level.id()))];
                overrides.extend(style.iter().cloned());
                let generated = generate(&overrides);
                assert!(!generated.parts.is_empty(), "{name} {version} {level:?}");
                let image = raster(
                    paths(&generated),
                    1.0,
                    17 + 4 * version,
                    4,
                    PER_MODULE,
                    false,
                );
                assert_eq!(
                    decode(image).as_deref(),
                    Some(content.as_str()),
                    "{name}: version {version} at {level:?}"
                );
            }
        }
    }
}

#[test]
fn the_diamond_module_reads_back() {
    for version in [1, 3, 6] {
        let content = content_for(version, EcLevel::M);
        round_trip(
            &[
                ("content", text(&content)),
                ("module_shape", choice("diamond")),
            ],
            &content,
            version,
            false,
        );
    }
}

#[test]
fn the_square_and_rounded_finder_styles_read_back() {
    let content = content_for(3, EcLevel::M);
    for outer in ["square", "rounded"] {
        for inner in ["square", "rounded", "circle"] {
            round_trip(
                &[
                    ("content", text(&content)),
                    ("finder_outer", choice(outer)),
                    ("finder_inner", choice(inner)),
                ],
                &content,
                3,
                false,
            );
        }
    }
}

#[test]
fn an_inverted_code_reads_back_engraved_as_light_ink() {
    for version in [1, 4, 7] {
        let content = content_for(version, EcLevel::M);
        for shape in ["square", "rounded", "dot"] {
            round_trip(
                &[
                    ("content", text(&content)),
                    ("inverted", ParamValue::Switch(true)),
                    ("module_shape", choice(shape)),
                ],
                &content,
                version,
                true,
            );
        }
    }
}

#[test]
fn the_largest_clear_area_at_each_level_reads_back_at_versions_3_5_and_10() {
    for version in [3, 5, 10] {
        for level in EcLevel::ALL {
            let content = content_for(version, level);
            for shape in ["square", "rounded", "circle"] {
                let asked = params(&[
                    ("content", text(&content)),
                    ("ec", choice(level.id())),
                    ("logo", ParamValue::Switch(true)),
                    ("logo_shape", choice(shape)),
                    ("logo_size", ParamValue::Percent(35)),
                    ("logo_margin", ParamValue::Count(2)),
                ]);
                let generated = generate_from(&asked, Some(Edit::Param("logo_size")));
                assert_eq!(
                    generated.params["ec"]["choice"],
                    level.id(),
                    "Edit::Param(logo_size) leaves the level alone"
                );
                let image = raster(
                    paths(&generated),
                    1.0,
                    17 + 4 * version,
                    4,
                    PER_MODULE,
                    false,
                );
                assert_eq!(
                    decode(image).as_deref(),
                    Some(content.as_str()),
                    "version {version} at {level:?}, {shape}, size {:?}",
                    generated.params["logo_size"]
                );
            }
        }
    }
}

#[test]
fn the_default_clear_area_reads_back_from_version_1_to_10() {
    for version in 1..=10 {
        for level in EcLevel::ALL {
            let content = content_for(version, level);
            let asked = params(&[
                ("content", text(&content)),
                ("ec", choice(level.id())),
                ("logo", ParamValue::Switch(true)),
            ]);
            let generated = generate_from(&asked, Some(Edit::Param("logo_size")));
            let image = raster(
                paths(&generated),
                1.0,
                17 + 4 * version,
                4,
                PER_MODULE,
                false,
            );
            assert_eq!(
                decode(image).as_deref(),
                Some(content.as_str()),
                "version {version} at {level:?} with the default clear area"
            );
        }
    }
}
