//! Tester acceptance cases of `specs/0058-bitmap-import` for placement, routing by content and
//! the notice texts (criteria 2, 4, 5, 9, 10, 12, 21, 22, 24, 26). Written from the specification;
//! they cover edges the spec examples and the branch's unit tests leave out.

#![allow(clippy::unwrap_used, missing_docs)]

use std::sync::Arc;

use curvyo_document_core::{DocumentSize, Length, Point};
use curvyo_ui_core::{
    FileKind, ImageTarget, STAGGER_MM, failure_notice, file_kind, fit_notice, image_display_name,
    natural_size_mm, place_image, plan_image_import, size_notice, truncated_notice,
};

const TOL: f64 = 1e-6;

fn page(width: f64, height: f64) -> DocumentSize {
    DocumentSize {
        width: Length::from_mm(width),
        height: Length::from_mm(height),
    }
}

/// The frame's corners (min x, min y, max x, max y) of a placement over the unit square.
fn frame(
    target: ImageTarget,
    pixels: (u32, u32),
    dpi: f64,
    doc: DocumentSize,
    index: usize,
) -> [f64; 4] {
    let placed = place_image(pixels, (dpi, dpi), doc, target, index).placement;
    let corners = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
        .map(|(x, y)| placed.apply_point(Point::new(x, y)));
    let xs = corners.map(|p| p.x);
    let ys = corners.map(|p| p.y);
    [
        xs.iter().copied().fold(f64::INFINITY, f64::min),
        ys.iter().copied().fold(f64::INFINITY, f64::min),
        xs.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        ys.iter().copied().fold(f64::NEG_INFINITY, f64::max),
    ]
}

fn assert_frame(got: [f64; 4], want: [f64; 4], what: &str) {
    for (g, w) in got.iter().zip(want) {
        assert!((g - w).abs() < TOL, "{what}: {got:?} vs {want:?}");
    }
}

// --- criteria 9, 10, 12: size and place ---------------------------------------------------------

#[test]
fn ac10_an_image_exactly_the_document_size_is_not_scaled() {
    // 794 x 1123 px at 96 dpi is 210.08 x 297.13 mm: just over A4, so it is scaled; the exact
    // A4 size in pixels at 25.4 dpi (210 x 297 px) is exactly A4 and is not.
    let a4 = page(210.0, 297.0);
    let exact = place_image(
        (210, 297),
        (25.4, 25.4),
        a4,
        ImageTarget::ViewCentre(Point::new(105.0, 148.5)),
        0,
    );
    assert!((exact.fit - 1.0).abs() < 1e-12, "fit {}", exact.fit);
    let over = place_image(
        (794, 1123),
        (96.0, 96.0),
        a4,
        ImageTarget::ViewCentre(Point::new(105.0, 148.5)),
        0,
    );
    assert!(over.fit < 1.0);
    let (w, h) = natural_size_mm((794, 1123), (96.0, 96.0));
    let fit = (210.0 / w).min(297.0 / h);
    assert!((over.fit - fit).abs() < 1e-12);
}

#[test]
fn ac10_a_picture_too_tall_only_is_fitted_by_its_height_and_never_enlarged() {
    // 100 x 2000 px at 96 dpi: 26.46 x 529.17 mm into A4 portrait: factor 297 / 529.17.
    let a4 = page(210.0, 297.0);
    let placed = place_image(
        (100, 2000),
        (96.0, 96.0),
        a4,
        ImageTarget::ViewCentre(Point::new(105.0, 148.5)),
        0,
    );
    let (_, h) = natural_size_mm((100, 2000), (96.0, 96.0));
    assert!((placed.fit - 297.0 / h).abs() < 1e-12);
    let got = frame(
        ImageTarget::ViewCentre(Point::new(105.0, 148.5)),
        (100, 2000),
        96.0,
        a4,
        0,
    );
    assert!((got[3] - got[1] - 297.0).abs() < TOL, "{got:?}");
    let tiny = place_image(
        (10, 10),
        (96.0, 96.0),
        a4,
        ImageTarget::ViewCentre(Point::new(0.0, 0.0)),
        0,
    );
    assert!((tiny.fit - 1.0).abs() < 1e-12, "never enlarged");
}

#[test]
fn ac12_a_view_centre_above_left_of_the_document_is_pulled_into_the_corner() {
    // 100 x 50 mm (377.95 x 188.98 px is not whole; use 25.4 dpi: 100 x 50 px).
    let doc = page(297.0, 210.0);
    let got = frame(
        ImageTarget::ViewCentre(Point::new(-400.0, -90.0)),
        (100, 50),
        25.4,
        doc,
        0,
    );
    assert_frame(
        got,
        [0.0, 0.0, 100.0, 50.0],
        "pulled to the top-left corner",
    );
}

#[test]
fn ac4_a_drop_outside_the_document_keeps_its_centre_at_the_drop_point() {
    let doc = page(297.0, 210.0);
    let got = frame(
        ImageTarget::Drop(Point::new(-40.0, 500.0)),
        (100, 50),
        25.4,
        doc,
        0,
    );
    assert_frame(got, [-90.0, 475.0, 10.0, 525.0], "no clamp for a drop");
}

#[test]
fn ac4_ac10_a_dropped_photo_larger_than_the_document_is_still_fitted() {
    // The clamp does not apply to a drop; the fit of criterion 10 still does.
    let a4 = page(210.0, 297.0);
    let placed = place_image(
        (4000, 3000),
        (96.0, 96.0),
        a4,
        ImageTarget::Drop(Point::new(500.0, 500.0)),
        0,
    );
    assert!((placed.fit - 210.0 / natural_size_mm((4000, 3000), (96.0, 96.0)).0).abs() < 1e-9);
    let got = frame(
        ImageTarget::Drop(Point::new(500.0, 500.0)),
        (4000, 3000),
        96.0,
        a4,
        0,
    );
    assert_frame(
        got,
        [395.0, 421.25, 605.0, 578.75],
        "centred on the drop point",
    );
}

#[test]
fn ac5_the_third_dropped_image_is_20_mm_right_and_down() {
    let doc = page(297.0, 210.0);
    let got = frame(
        ImageTarget::Drop(Point::new(50.0, 50.0)),
        (20, 10),
        25.4,
        doc,
        2,
    );
    assert!((STAGGER_MM - 10.0).abs() < 1e-12);
    assert_frame(got, [60.0, 65.0, 80.0, 75.0], "centre at (70, 70)");
}

// --- criterion 2: routing by content ------------------------------------------------------------

#[test]
fn ac2_the_kind_comes_from_the_first_bytes_not_the_name() {
    let png = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../curvyo-image-core/tests/fixtures/images/png_rgb_8.png"
    ))
    .unwrap();
    assert_eq!(file_kind(&png), FileKind::Image);
    let mut polyglot = png.clone();
    polyglot.extend_from_slice(b"PK\x03\x04 zip tail");
    assert_eq!(
        file_kind(&polyglot),
        FileKind::Image,
        "a PNG with a zip appended"
    );
    assert_eq!(
        file_kind(b"\xEF\xBB\xBF<?xml version=\"1.0\"?><svg/>"),
        FileKind::Svg
    );
    assert_eq!(file_kind(b"PK\x03\x04project"), FileKind::Project);
    assert_eq!(file_kind(b"BM\x36\x00\x00\x00"), FileKind::Other, "BMP");
    assert_eq!(file_kind(b"GIF89a\x01\x00"), FileKind::Other, "GIF");
    assert_eq!(file_kind(b"II*\x00"), FileKind::Other, "TIFF");
    assert_eq!(file_kind(b""), FileKind::Other, "empty");
    // A JPEG signature cut after two bytes is not a JPEG.
    assert_eq!(file_kind(&[0xFF, 0xD8]), FileKind::Other);
}

#[test]
fn ac2_ac26_a_text_file_named_png_is_refused_with_the_unsupported_reason() {
    let result = plan_image_import(
        Arc::from(&b"just text"[..]),
        "x.png",
        page(210.0, 297.0),
        ImageTarget::ViewCentre(Point::new(0.0, 0.0)),
        0,
    );
    let reason = result.err().unwrap();
    assert!(
        reason.starts_with("This file type isn't supported."),
        "{reason}"
    );
}

// --- criteria 10, 21, 22, 24, 26: texts --------------------------------------------------------

#[test]
fn ac10_the_fit_notice_rounds_the_percent_and_keeps_its_length() {
    assert_eq!(
        fit_notice(0.19843),
        "Scaled to 20 % to fit the document. Original size restores it."
    );
    assert_eq!(fit_notice(0.19843).chars().count(), 62);
    assert!(
        fit_notice(0.104).starts_with("Scaled to 10 % "),
        "{}",
        fit_notice(0.104)
    );
    assert!(fit_notice(0.005).chars().count() <= 70);
}

#[test]
fn ac22_display_names_drop_folders_controls_and_markup_and_cut_at_128_chars() {
    assert_eq!(
        image_display_name("C:\\Users\\me\\Pictures\\cat.png"),
        "cat.png"
    );
    assert_eq!(image_display_name("/home/me/a<b>\"c\".png"), "abc.png");
    assert_eq!(image_display_name("tab\there\u{7}.png"), "tabhere.png");
    assert_eq!(image_display_name("dir/"), "Image");
    assert_eq!(image_display_name("<>\"\u{1}"), "Image");
    let long: String = "é".repeat(200);
    let cut = image_display_name(&long);
    assert_eq!(cut.chars().count(), 128);
    assert!(cut.chars().all(|c| c == 'é'));
}

#[test]
fn ac26_the_failure_notice_cuts_the_name_to_24_characters_with_an_ellipsis() {
    let notice = failure_notice(
        "/tmp/photo-of-the-long-name-from-holiday.jpg",
        "The image is damaged and could not be read.",
    );
    let mut lines = notice.lines();
    let first = lines.next().unwrap();
    assert_eq!(first.chars().count(), 24, "{first}");
    // The rule: 24 characters with an ellipsis. The spec's example cuts at the end; the code
    // cuts in the middle and keeps the extension (the 0024 convention): reported, not failed.
    assert_eq!(first.matches('…').count(), 1, "{first}");
    assert!(first.starts_with("photo-of-the-lo"), "{first}");
    assert_eq!(
        lines.next(),
        Some("The image is damaged and could not be read.")
    );
    assert!(!notice.contains("/tmp"), "never a path");
}

#[test]
fn ac21_the_truncated_notice_cuts_a_long_name_and_stays_one_line() {
    assert_eq!(
        truncated_notice("photo.jpg"),
        "photo.jpg is incomplete; the part that could be read was imported."
    );
    let long = truncated_notice("a-very-long-camera-file-name-0001.jpg");
    let name = long.split(" is incomplete").next().unwrap();
    assert_eq!(name.chars().count(), 24, "{long}");
    assert_eq!(name.matches('…').count(), 1, "{long}");
    assert_eq!(long.lines().count(), 1);
}

#[test]
fn ac24_the_size_line_starts_at_exactly_10_mib() {
    assert_eq!(size_notice(10_485_759), None);
    assert_eq!(
        size_notice(10_485_760).as_deref(),
        Some("This image adds 10.0 MB to the project file.")
    );
    assert_eq!(
        size_notice(25_217_843).as_deref(),
        Some("This image adds 24.0 MB to the project file.")
    );
}
