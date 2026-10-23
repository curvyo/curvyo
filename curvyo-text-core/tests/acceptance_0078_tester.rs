//! Independent tester cases for `specs/0078-font-style-buttons/specification.md` on
//! `curvyo-text-core`: the weight bands of the Terms, weight/width/slant read from bytes
//! (criterion 28, OpenType and SVG fonts) and the four extra Source Sans 3 faces (criterion 27).
//!
//! Written from the specification, `adrs.md` and the public names before the implementation was
//! read.

#![allow(clippy::unwrap_used, clippy::assert_is_empty, missing_docs)]

use std::path::PathBuf;

use curvyo_text_core::{FaceStyle, NORMAL_WIDTH, WeightBand, listable_faces};

fn font(file: &str) -> Vec<u8> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../frontend/public/fonts")
            .join(file),
    )
    .unwrap()
}

/// Terms: the band edges of the weight steps.
#[test]
fn terms_band_edges() {
    for (w, band) in [
        (149, WeightBand::Thin),
        (150, WeightBand::ExtraLight),
        (249, WeightBand::ExtraLight),
        (250, WeightBand::Light),
        (349, WeightBand::Light),
        (350, WeightBand::Regular),
        (449, WeightBand::Regular),
        (450, WeightBand::Medium),
        (549, WeightBand::Medium),
        (550, WeightBand::Semibold),
        (649, WeightBand::Semibold),
        (650, WeightBand::Bold),
        (749, WeightBand::Bold),
        (750, WeightBand::ExtraBold),
        (849, WeightBand::ExtraBold),
        (850, WeightBand::Black),
        (1000, WeightBand::Black),
        (0, WeightBand::Thin),
        (u16::MAX, WeightBand::Black),
    ] {
        assert_eq!(WeightBand::of(w), band, "{w}");
    }
}

/// Criteria 27 and 28: the shipped Source Sans 3 faces read with the right band and slant from
/// their bytes; no Extra Bold ships.
#[test]
fn ac27_ac28_source_sans_faces_from_bytes() {
    for (file, band, italic) in [
        ("SourceSans3-ExtraLight.otf", WeightBand::ExtraLight, false),
        ("SourceSans3-ExtraLightIt.otf", WeightBand::ExtraLight, true),
        ("SourceSans3-Light.otf", WeightBand::Light, false),
        ("SourceSans3-LightIt.otf", WeightBand::Light, true),
        ("SourceSans3-Regular.otf", WeightBand::Regular, false),
        ("SourceSans3-It.otf", WeightBand::Regular, true),
        ("SourceSans3-Bold.otf", WeightBand::Bold, false),
        ("SourceSans3-BoldIt.otf", WeightBand::Bold, true),
    ] {
        let s = FaceStyle::from_bytes(&font(&format!("source-sans-3/{file}")), 0).unwrap();
        assert_eq!(
            (s.band(), s.is_slanted(), s.width),
            (band, italic, NORMAL_WIDTH),
            "{file}"
        );
    }
}

/// Criterion 27: `index.json` and `font.toml` list the eight faces, all present, all OFL.
#[test]
fn ac27_index_lists_eight_source_sans_faces() {
    let index: serde_json::Value =
        serde_json::from_str(&String::from_utf8(font("index.json")).unwrap()).unwrap();
    let mut styles: Vec<String> = index["faces"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["family"] == "Source Sans 3")
        .map(|f| {
            assert_eq!(f["licence"], "OFL-1.1");
            let file = f["file"].as_str().unwrap();
            let listed = listable_faces(&font(file));
            assert_eq!(listed.len(), 1, "{file}");
            assert_eq!(listed[0].style, f["style"].as_str().unwrap(), "{file}");
            f["style"].as_str().unwrap().to_owned()
        })
        .collect();
    styles.sort();
    assert_eq!(
        styles,
        [
            "Bold",
            "Bold Italic",
            "Extra Light",
            "Extra Light Italic",
            "Italic",
            "Light",
            "Light Italic",
            "Regular"
        ]
    );
    let toml = String::from_utf8(font("source-sans-3/font.toml")).unwrap();
    assert!(toml.contains("3.052R"));
    assert_eq!(toml.matches("[[faces]]").count(), 8);
}

fn svg_font(attrs: &str) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\"?><svg xmlns=\"http://www.w3.org/2000/svg\"><defs>\
         <font id=\"f\"><font-face font-family=\"T\" {attrs}/>\
         <glyph unicode=\"I\" d=\"M 0 0 V 700\"/></font></defs></svg>"
    )
    .into_bytes()
}

/// Terms and criterion 28: an SVG font face is Bold for `bold`, `bolder` or 600 and over,
/// else Regular; italic or oblique `font-style` is an italic face.
#[test]
fn ac28_svg_font_weight_and_slant() {
    for (attrs, band, slanted) in [
        ("", WeightBand::Regular, false),
        ("font-weight=\"bold\"", WeightBand::Bold, false),
        ("font-weight=\"bolder\"", WeightBand::Bold, false),
        ("font-weight=\"600\"", WeightBand::Bold, false),
        ("font-weight=\"599\"", WeightBand::Regular, false),
        ("font-weight=\"900\"", WeightBand::Bold, false),
        ("font-weight=\"100\"", WeightBand::Regular, false),
        ("font-weight=\"lighter\"", WeightBand::Regular, false),
        ("font-style=\"italic\"", WeightBand::Regular, true),
        ("font-style=\"oblique\"", WeightBand::Regular, true),
        (
            "font-weight=\"700\" font-style=\"italic\"",
            WeightBand::Bold,
            true,
        ),
    ] {
        let faces = listable_faces(&svg_font(attrs));
        assert_eq!(faces.len(), 1, "{attrs}");
        let s = FaceStyle::parse(&faces[0].style)
            .unwrap_or_else(|| panic!("{attrs}: {}", faces[0].style));
        assert_eq!(
            (s.band(), s.is_slanted()),
            (band, slanted),
            "{attrs}: {}",
            faces[0].style
        );
    }
}

/// Edge: hostile and empty bytes give no style, never a panic.
#[test]
fn from_bytes_rejects_garbage() {
    assert_eq!(FaceStyle::from_bytes(&[], 0), None);
    assert_eq!(FaceStyle::from_bytes(b"OTTO\0\0\0\0", 0), None);
    let regular = font("source-sans-3/SourceSans3-Regular.otf");
    assert_eq!(
        FaceStyle::from_bytes(&regular, 5),
        None,
        "index past a single face"
    );
    // A truncated file whose header parses may still read as Regular; the listing is the gate.
    assert!(listable_faces(&regular[..200]).is_empty());
}
