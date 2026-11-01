//! Tester acceptance cases of `specs/0087-image-clipboard-and-drop` slice 1 at the session level
//! (criteria 1, 2, 5, 7, 8, 9, 11, 12, 13), written from the specification: the paste origin, the
//! size rules, one undo step, the placement under the pointer in a zoomed and panned view, content
//! over MIME label, refused data, the limits, and the copy of the stored pixels.

#![allow(
    clippy::unwrap_used,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    missing_docs
)]

use curvyo_document_core::{ImageSnapshot, ObjectSnapshot, Point, unpack};
use curvyo_editor_wasm::{DocumentSide, ImportOrigin, ImportOutcome, KeyInput, Session, Tool};
use curvyo_image_core::{WorkingImage, decode_working, encode_png};
use curvyo_ui_core::FlipAxis;

const TOL: f64 = 1e-6;

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../curvyo-image-core/tests/fixtures/images")
            .join(name),
    )
    .unwrap()
}

fn fresh() -> Session {
    let mut session = Session::new(1);
    session.resize_viewport(1600.0, 1000.0);
    session
}

/// A document of `w` x `h` mm.
fn sized(w: &str, h: &str) -> Session {
    let mut session = fresh();
    session.set_document_side(DocumentSide::Width, w);
    session.set_document_side(DocumentSide::Height, h);
    session
}

fn paste(
    session: &mut Session,
    at: Option<(f64, f64)>,
    bytes: &[u8],
    label: &str,
) -> ImportOutcome {
    session.import_begin_from(ImportOrigin::Paste, at);
    session.import_add(bytes, label);
    session.import_finish(false)
}

fn images(session: &Session) -> Vec<ImageSnapshot> {
    let document = unpack(99, &session.pack("t").unwrap()).unwrap();
    document
        .object_ids()
        .into_iter()
        .filter_map(|id| match document.object(id) {
            Some(ObjectSnapshot::Image(image)) => Some(image),
            _ => None,
        })
        .collect()
}

fn object_count(session: &Session) -> usize {
    unpack(99, &session.pack("t").unwrap())
        .unwrap()
        .object_ids()
        .len()
}

fn centre(image: &ImageSnapshot) -> Point {
    image.placement.apply_point(Point::new(0.5, 0.5))
}

fn size_mm(image: &ImageSnapshot) -> (f64, f64) {
    let o = image.placement.apply_point(Point::new(0.0, 0.0));
    let x = image.placement.apply_point(Point::new(1.0, 0.0));
    let y = image.placement.apply_point(Point::new(0.0, 1.0));
    (
        ((x.x - o.x).powi(2) + (x.y - o.y).powi(2)).sqrt(),
        ((y.x - o.x).powi(2) + (y.y - o.y).powi(2)).sqrt(),
    )
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn solid(width: u32, height: u32, mut pixel: impl FnMut(u32, u32) -> [u8; 4]) -> WorkingImage {
    let mut pixels = Vec::new();
    for y in 0..height {
        for x in 0..width {
            pixels.extend_from_slice(&pixel(x, y));
        }
    }
    WorkingImage {
        width,
        height,
        pixels,
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

/// A PNG of a flat colour, with a `pHYs` chunk of `dpi` when given.
fn flat_png(width: u32, height: u32, dpi: Option<f64>) -> Vec<u8> {
    let png = encode_png(&solid(width, height, |_, _| [10, 20, 30, 255])).unwrap();
    let Some(dpi) = dpi else { return png };
    let ppm = (dpi / 0.0254).round() as u32;
    let mut data = Vec::new();
    data.extend_from_slice(&ppm.to_be_bytes());
    data.extend_from_slice(&ppm.to_be_bytes());
    data.push(1);
    let mut chunk = Vec::new();
    chunk.extend_from_slice(&9u32.to_be_bytes());
    let mut body = b"pHYs".to_vec();
    body.extend_from_slice(&data);
    chunk.extend_from_slice(&body);
    chunk.extend_from_slice(&crc32(&body).to_be_bytes());
    let mut out = png[..33].to_vec(); // signature + IHDR
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&png[33..]);
    out
}

fn marker_png() -> Vec<u8> {
    encode_png(&solid(40, 20, |x, y| {
        if x < 5 && y < 5 {
            [255, 0, 0, 255]
        } else {
            [255, 255, 255, 255]
        }
    }))
    .unwrap()
}

// --- criterion 1 -------------------------------------------------------------------------------

#[test]
fn ac1_a_3000_by_2000_png_at_300_dpi_is_254_by_169_33_mm() {
    let mut s = sized("500", "500");
    let out = paste(&mut s, None, &flat_png(3000, 2000, Some(300.0)), "");
    assert_eq!(
        (out.imported, out.alert, out.notice.as_str()),
        (1, false, "")
    );
    let (w, h) = size_mm(&images(&s)[0]);
    assert!(
        (w - 254.0).abs() < 0.01 && (h - 169.33).abs() < 0.01,
        "{w} x {h}"
    );
}

#[test]
fn ac1_a_4000_by_3000_picture_without_density_in_a4_portrait_scales_to_20_percent() {
    let mut s = sized("210", "297");
    let out = paste(&mut s, None, &flat_png(4000, 3000, None), "");
    assert_eq!(out.imported, 1, "{}", out.notice);
    assert!(
        out.notice.contains("Scaled to 20 % to fit the document."),
        "{}",
        out.notice
    );
    assert!(!out.alert);
    let (w, h) = size_mm(&images(&s)[0]);
    assert!(w <= 210.0 + TOL && h <= 297.0 + TOL, "{w} x {h}");
    assert!((w - 210.0).abs() < 0.01, "width fills the page: {w}");
}

#[test]
fn ac1_one_paste_is_one_undo_step_and_redo_brings_it_back() {
    let mut s = fresh();
    let steps = s.undo_len();
    paste(&mut s, None, &marker_png(), "");
    assert_eq!(s.undo_len(), steps + 1, "one step per paste");
    assert_eq!(s.selected_object_count(), 1);
    assert!(s.undo().changed);
    assert_eq!(object_count(&s), 0);
    assert_eq!(s.undo_len(), steps);
    assert!(s.redo().changed);
    assert_eq!(images(&s).len(), 1);
}

#[test]
fn ac1_the_pasted_image_is_selected_and_the_select_tool_is_active() {
    let mut s = fresh();
    s.set_tool(Tool::Ellipse);
    paste(&mut s, None, &marker_png(), "");
    assert_eq!(s.tool(), Tool::Select);
    assert_eq!(s.selected_object_count(), 1);
}

#[test]
fn ac1_the_picture_is_identified_by_content_not_by_its_label() {
    let mut s = fresh();
    for (bytes, label) in [
        (fixture("png_rgb_8.png"), "image.jpg"),
        (fixture("jpeg_progressive.jpg"), "image.png"),
        (fixture("webp_lossy.webp"), "image.png"),
        (fixture("png_renamed.jpg"), ""),
    ] {
        let out = paste(&mut s, None, &bytes, label);
        assert_eq!(
            (out.imported, out.alert),
            (1, false),
            "{label}: {}",
            out.notice
        );
    }
    assert_eq!(images(&s).len(), 4);
}

#[test]
fn ac1_the_same_bytes_pasted_twice_embed_the_asset_once() {
    // A noisy PNG that does not compress: the container grows by the object only.
    let mut seed = 7u32;
    let noisy = encode_png(&solid(200, 200, |_, _| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let b = seed.to_be_bytes();
        [b[0], b[1], b[2], 255]
    }))
    .unwrap();
    let mut s = fresh();
    paste(&mut s, None, &noisy, "");
    let one = s.pack("t").unwrap().len();
    paste(&mut s, None, &noisy, "");
    let two = s.pack("t").unwrap().len();
    assert!(
        two - one < 1024,
        "second paste grew the project by {} bytes",
        two - one
    );
}

// --- criterion 2 -------------------------------------------------------------------------------

#[test]
fn ac2_a_paste_in_a_zoomed_and_panned_view_centres_under_the_pointer() {
    let mut s = fresh();
    s.wheel(0.0, -480.0, 300.0, 200.0, false, true);
    s.pan_to(-350.0, 120.0);
    let (sx, sy) = (777.25, 411.5);
    let under = s.screen_to_document(sx, sy);
    paste(&mut s, Some((sx, sy)), &flat_png(30, 20, None), "");
    let got = centre(&images(&s)[0]);
    assert!(
        (got.x - under.x).abs() < TOL && (got.y - under.y).abs() < TOL,
        "{got:?} {under:?}"
    );
}

#[test]
fn ac2_without_a_pointer_the_image_centre_is_the_view_centre_and_follows_the_view() {
    let mut s = fresh();
    s.import_begin_from(ImportOrigin::Picker, None);
    s.import_add(&flat_png(30, 20, None), "a.png");
    s.import_finish(false);
    let picker = centre(&images(&s)[0]);
    let mut p = fresh();
    paste(&mut p, None, &flat_png(30, 20, None), "");
    let pasted = centre(&images(&p)[0]);
    assert!((pasted.x - picker.x).abs() < TOL && (pasted.y - picker.y).abs() < TOL);
    // The view centre moves with the view: zoom in about a point off the centre.
    let mut q = fresh();
    q.wheel(0.0, -480.0, 300.0, 200.0, false, true);
    paste(&mut q, None, &flat_png(30, 20, None), "");
    let moved = centre(&images(&q)[0]);
    assert!(
        (moved.x - pasted.x).abs() > 1.0 || (moved.y - pasted.y).abs() > 1.0,
        "{moved:?} vs {pasted:?}"
    );
}

#[test]
fn ac2_a_pointer_inside_the_document_is_not_moved_by_any_clamp() {
    let mut s = sized("297", "210");
    let inside = s.screen_to_document(800.0, 500.0);
    assert!(
        inside.x > 20.0 && inside.x < 270.0 && inside.y > 20.0 && inside.y < 190.0,
        "{inside:?}"
    );
    paste(&mut s, Some((800.0, 500.0)), &flat_png(100, 50, None), "");
    let c = centre(&images(&s)[0]);
    assert!((c.x - inside.x).abs() < TOL && (c.y - inside.y).abs() < TOL);
}

#[test]
fn ac2_a_second_paste_is_pasted_image_again_without_a_counter() {
    let mut s = fresh();
    paste(&mut s, None, &marker_png(), "");
    paste(&mut s, None, &marker_png(), "");
    let names: Vec<_> = {
        let document = unpack(99, &s.pack("t").unwrap()).unwrap();
        document
            .object_ids()
            .into_iter()
            .filter_map(|id| document.node_registers(id).and_then(|r| r.name))
            .collect()
    };
    assert_eq!(
        names,
        vec!["Pasted image".to_owned(), "Pasted image".to_owned()]
    );
}

// --- criteria 5, 7, 8: nothing to paste, damaged, over the limits ------------------------------

#[test]
fn ac5_ac7_refusals_leave_document_and_history_unchanged() {
    let mut s = fresh();
    let steps = s.undo_len();
    let hostile = [
        ("png_truncated.png", true),
        ("png_bad_crc.png", true),
        ("png_zero_width.png", true),
        ("webp_truncated.webp", true),
        ("webp_chunk_overflow.webp", true),
        ("jpeg_endless_markers.jpg", true),
        ("empty.png", true),
        ("text_renamed.png", true),
        ("pdf_renamed.png", true),
        ("svg_renamed.png", true),
    ];
    for (name, _) in hostile {
        let out = paste(&mut s, None, &fixture(name), "");
        assert_eq!(
            (out.imported, out.alert),
            (0, true),
            "{name}: {}",
            out.notice
        );
        assert!(!out.notice.is_empty(), "{name}");
    }
    for bytes in [
        &b"GIF89a\x01\x00\x01\x00\x00\x00\x00;"[..],
        b"BM\x1e\x00\x00\x00\x00\x00\x00\x00\x1a\x00\x00\x00",
        b"",
    ] {
        let out = paste(&mut s, None, bytes, "");
        assert_eq!((out.imported, out.alert), (0, true), "{}", out.notice);
    }
    assert_eq!(object_count(&s), 0);
    assert_eq!(s.undo_len(), steps, "no history step");
}

#[test]
fn ac8_the_header_bombs_are_refused_with_the_size_notice_and_nothing_is_inserted() {
    let mut s = fresh();
    for name in [
        "png_bomb_60000x60000.png",
        "jpeg_dimensions_65535x65535.jpg",
    ] {
        let out = paste(&mut s, None, &fixture(name), "");
        assert_eq!((out.imported, out.alert), (0, true), "{name}");
        assert!(
            out.notice.contains("Image too large") && out.notice.contains("limit is 50 megapixels"),
            "{name}: {}",
            out.notice
        );
    }
    assert_eq!(object_count(&s), 0);
}

#[test]
fn ac7_a_truncated_jpeg_imports_its_readable_part_with_the_incomplete_notice() {
    let mut s = fresh();
    let out = paste(&mut s, None, &fixture("jpeg_truncated.jpg"), "");
    assert_eq!(out.imported, 1);
    assert!(out.notice.contains("is incomplete"), "{}", out.notice);
}

#[test]
fn ac1_an_exif_rotated_jpeg_is_pasted_upright() {
    let mut s = sized("500", "500");
    paste(&mut s, None, &fixture("jpeg_exif_orientation_6.jpg"), "");
    let (w, h) = size_mm(&images(&s)[0]);
    // 20 x 40 px upright at 96 dpi.
    assert!(
        (w - 20.0 * 25.4 / 96.0).abs() < 0.01 && (h - 40.0 * 25.4 / 96.0).abs() < 0.01,
        "{w} x {h}"
    );
}

// --- criterion 9 -------------------------------------------------------------------------------

#[test]
fn ac9_trace_stays_the_tool_and_a_later_non_trace_paste_selects_the_select_tool() {
    let mut s = fresh();
    s.set_tool(Tool::Trace);
    paste(&mut s, None, &marker_png(), "");
    assert_eq!(s.tool(), Tool::Trace);
    assert_eq!(s.trace_view().images.len(), 1);
    s.set_tool(Tool::Pen);
    paste(&mut s, None, &marker_png(), "");
    assert_eq!(s.tool(), Tool::Select);
}

// --- criteria 11, 12, 13: copy -----------------------------------------------------------------

#[test]
fn ac12_the_copy_ignores_flip_and_returns_the_stored_pixels_png_round_trips() {
    let mut s = fresh();
    paste(&mut s, None, &marker_png(), "");
    s.flip_selection(FlipAxis::Horizontal);
    s.flip_selection(FlipAxis::Vertical);
    let copy = s.image_copy().unwrap();
    assert_eq!((copy.width, copy.height), (40, 20));
    assert_eq!(&copy.rgba[..4], &[255, 0, 0, 255], "marker still top-left");
    let png =
        curvyo_editor_wasm::encode_rgba_png(copy.rgba.clone(), copy.width, copy.height).unwrap();
    assert_eq!(decode_working(&png).unwrap().pixels, copy.rgba);
}

#[test]
fn ac12_a_pasted_jpeg_copies_as_png_and_pastes_back_with_the_same_pixels() {
    let mut s = fresh();
    paste(&mut s, None, &fixture("jpeg_exif_orientation_6.jpg"), "");
    let copy = s.image_copy().unwrap();
    assert_eq!(
        (copy.width, copy.height),
        (20, 40),
        "upright after the EXIF rotation"
    );
    let png =
        curvyo_editor_wasm::encode_rgba_png(copy.rgba.clone(), copy.width, copy.height).unwrap();
    let mut t = fresh();
    let out = paste(&mut t, None, &png, "");
    assert_eq!(out.imported, 1);
    let again = t.image_copy().unwrap();
    assert_eq!(
        (again.width, again.height, again.rgba),
        (copy.width, copy.height, copy.rgba)
    );
}

#[test]
fn ac12_translucent_pixels_survive_the_copy_exactly() {
    let src = solid(4, 4, |x, y| {
        [(x * 60) as u8, (y * 60) as u8, 7, (x * 80 + y) as u8]
    });
    let mut s = fresh();
    paste(&mut s, None, &encode_png(&src).unwrap(), "");
    assert_eq!(s.image_copy().unwrap().rgba, src.pixels);
}

#[test]
fn ac13_a_selection_without_an_image_copies_nothing_and_changes_nothing() {
    let mut s = fresh();
    assert_eq!(s.image_copy(), None, "empty document");
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(10.0, 10.0), false);
    s.pointer_up(Point::new(60.0, 50.0), false, false);
    assert!(s.selected_object_count() >= 1);
    assert_eq!(s.image_copy(), None, "a rectangle is not an image");
}

#[test]
fn ac13_one_image_among_other_objects_copies_it_and_counts_one() {
    let mut s = fresh();
    paste(&mut s, None, &marker_png(), "");
    s.set_tool(Tool::Rectangle);
    s.pointer_down(Point::new(10.0, 10.0), false);
    s.pointer_up(Point::new(60.0, 50.0), false, false);
    s.set_tool(Tool::Select);
    s.key_down(KeyInput {
        key: "a",
        shift: false,
        ctrl: true,
        alt: false,
        repeat: false,
        dom_blocked: false,
        mac: false,
        time_ms: 0.0,
    });
    assert!(s.selected_object_count() >= 2);
    let copy = s.image_copy().unwrap();
    assert_eq!((copy.width, copy.height), (40, 20));
}

#[test]
fn ac11_a_copy_changes_neither_history_nor_selection() {
    let mut s = fresh();
    paste(&mut s, None, &marker_png(), "");
    let steps = s.undo_len();
    let _ = s.image_copy();
    assert_eq!(s.undo_len(), steps);
    assert_eq!(s.selected_object_count(), 1);
}
