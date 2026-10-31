//! Tester acceptance cases of `specs/0058-bitmap-import` for the decoder boundary: formats by
//! content, colour rules, orientation, density, limits, hostile files and the truncated JPEG
//! (criteria 6 to 9, 16 to 21, 28). Written from the specification before the implementation was
//! read; the edges below are the ones the spec examples do not cover.
//!
//! Memory is the growth of the process's peak resident size during one decode (`VmHWM`, reset
//! through `/proc/self/clear_refs`); time is the calling thread's CPU time from `/proc/thread-self/schedstat` (`CLAUDE.md` §5), so a
//! busy machine does not fail a bound. Measured tests take one lock so a parallel `cargo test` does
//! not mix their peaks.

#![allow(clippy::unwrap_used, clippy::expect_used, missing_docs)]

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use curvyo_image_core::{
    DECODER_HEADROOM_BYTES, DEFAULT_DPI, ImageError, ImageFormat, MAX_FILE_BYTES,
    MAX_HEADER_PIXELS, MAX_WORKING_PIXELS, MAX_WORKING_SIDE, WorkingImage, decode_working, probe,
    working_size,
};
use image::ImageEncoder;

// --- measuring --------------------------------------------------------------------------------

static MEASURED: Mutex<()> = Mutex::new(());

/// A `kB` line of `/proc/self/status` in bytes.
fn status_bytes(key: &str) -> Option<usize> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with(key))?;
    let kb: usize = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb * 1024)
}

/// Resets the process's peak resident size to its current size (Linux `clear_refs` 5).
fn reset_peak() -> bool {
    std::fs::write("/proc/self/clear_refs", "5").is_ok()
}

/// The calling thread's CPU time (Linux `schedstat`; wall time elsewhere).
fn thread_cpu() -> Duration {
    std::fs::read_to_string("/proc/thread-self/schedstat")
        .ok()
        .and_then(|text| text.split_whitespace().next()?.parse::<u64>().ok())
        .map_or_else(
            || {
                static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
                START.get_or_init(std::time::Instant::now).elapsed()
            },
            Duration::from_nanos,
        )
}

struct Measured<T> {
    result: T,
    peak_bytes: usize,
    cpu: Duration,
}

/// Runs `work` alone, returning its result, the growth of the process's peak resident size over
/// its size at the start (0 where Linux `/proc` is not there) and the thread CPU time it took.
/// Under `cargo nextest` every test is its own process, so the peak is this work's alone.
fn measured<T>(work: impl FnOnce() -> T) -> Measured<T> {
    let _guard = MEASURED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let reset = reset_peak();
    let base = status_bytes("VmRSS:").unwrap_or(0);
    let started = thread_cpu();
    let result = work();
    let cpu = thread_cpu().saturating_sub(started);
    let peak_bytes = if reset {
        status_bytes("VmHWM:").unwrap_or(0).saturating_sub(base)
    } else {
        0
    };
    Measured {
        result,
        peak_bytes,
        cpu,
    }
}

/// The CPU bound of a criterion for this build: as written for an optimised build (the build
/// host's product), five times that for an unoptimised test build, whose decoder dependencies
/// run without optimisation and under the full nextest load (measured: the iCCP bomb takes 0.02 s
/// in release, 0.7 s alone and up to 2.3 s under load in debug).
fn budget(bound: Duration) -> Duration {
    if cfg!(debug_assertions) {
        bound * 5
    } else {
        bound
    }
}

const MIB: usize = 1024 * 1024;

// --- inputs -----------------------------------------------------------------------------------

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn spec_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join("images").join(name)).unwrap()
}

fn tester_fixture(name: &str) -> Vec<u8> {
    std::fs::read(fixtures().join("tester").join(name)).unwrap()
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
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

fn png_chunk(kind: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = u32::try_from(data.len()).unwrap().to_be_bytes().to_vec();
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let mut covered = kind.to_vec();
    covered.extend_from_slice(data);
    out.extend_from_slice(&crc32(&covered).to_be_bytes());
    out
}

fn encode_png(width: u32, height: u32, pixels: &[u8], colour: image::ExtendedColorType) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(pixels, width, height, colour)
        .unwrap();
    out
}

/// `png` with `chunk` inserted right before its first IDAT.
fn png_insert_before_idat(png: &[u8], chunk: &[u8]) -> Vec<u8> {
    let idat = png.windows(4).position(|w| w == b"IDAT").unwrap() - 4;
    let mut out = png[..idat].to_vec();
    out.extend_from_slice(chunk);
    out.extend_from_slice(&png[idat..]);
    out
}

/// A PNG that only has a header: IHDR of `width` x `height` RGB8, a tiny IDAT and IEND.
fn png_header_only(width: u32, height: u32) -> Vec<u8> {
    let mut ihdr = width.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    out.extend(png_chunk(*b"IHDR", &ihdr));
    out.extend(png_chunk(
        *b"IDAT",
        &[0x78, 0x9C, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01],
    ));
    out.extend(png_chunk(*b"IEND", &[]));
    out
}

fn encode_jpeg(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 95)
        .write_image(rgb, width, height, image::ExtendedColorType::Rgb8)
        .unwrap();
    out
}

fn encode_webp_lossless(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    image::codecs::webp::WebPEncoder::new_lossless(&mut out)
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)
        .unwrap();
    out
}

/// A big-endian TIFF block whose IFD0 holds only `Orientation` = `value`.
fn exif_tiff(value: u16) -> Vec<u8> {
    let mut out = b"MM\x00\x2a\x00\x00\x00\x08".to_vec();
    out.extend_from_slice(&1_u16.to_be_bytes());
    out.extend_from_slice(&0x0112_u16.to_be_bytes());
    out.extend_from_slice(&3_u16.to_be_bytes());
    out.extend_from_slice(&1_u32.to_be_bytes());
    out.extend_from_slice(&value.to_be_bytes());
    out.extend_from_slice(&[0, 0]);
    out.extend_from_slice(&0_u32.to_be_bytes());
    out
}

fn riff_chunk(kind: [u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = kind.to_vec();
    out.extend_from_slice(&u32::try_from(payload.len()).unwrap().to_le_bytes());
    out.extend_from_slice(payload);
    if payload.len() % 2 == 1 {
        out.push(0);
    }
    out
}

fn u24(value: u32) -> [u8; 3] {
    let b = value.to_le_bytes();
    [b[0], b[1], b[2]]
}

/// A lossless WebP of `rgba` in the extended form (VP8X) with an EXIF chunk naming `orientation`.
fn webp_with_orientation(width: u32, height: u32, rgba: &[u8], orientation: u16) -> Vec<u8> {
    let simple = encode_webp_lossless(width, height, rgba);
    let bitstream_at = simple.windows(4).position(|w| w == b"VP8L").unwrap();
    let bitstream = &simple[bitstream_at..];
    let mut header = vec![0x08 | 0x10, 0, 0, 0];
    header.extend_from_slice(&u24(width - 1));
    header.extend_from_slice(&u24(height - 1));
    let mut body = b"WEBP".to_vec();
    body.extend(riff_chunk(*b"VP8X", &header));
    body.extend_from_slice(bitstream);
    body.extend(riff_chunk(*b"EXIF", &exif_tiff(orientation)));
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
    out.extend(body);
    out
}

/// A 3 x 2 picture whose six pixels all differ, as RGBA.
fn asymmetric_rgba() -> (u32, u32, Vec<u8>) {
    let colours: [[u8; 4]; 6] = [
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
        [255, 255, 0, 255],
        [0, 255, 255, 255],
        [255, 0, 255, 255],
    ];
    (3, 2, colours.concat())
}

/// The stored pixel shown at upright (`col`, `row`) for EXIF orientation `value` of a
/// `width` x `height` picture.
fn stored_coordinate(value: u16, width: u32, height: u32, col: u32, row: u32) -> (u32, u32) {
    let (last_col, last_row) = (width - 1, height - 1);
    match value {
        1 => (col, row),
        2 => (last_col - col, row),
        3 => (last_col - col, last_row - row),
        4 => (col, last_row - row),
        5 => (row, col),
        6 => (row, last_row - col),
        7 => (last_col - row, last_row - col),
        8 => (last_col - row, col),
        _ => unreachable!(),
    }
}

fn check_upright(name: &str, working: &WorkingImage, o: u16, w: u32, h: u32, stored: &[u8]) {
    let (uw, uh) = if o >= 5 { (h, w) } else { (w, h) };
    assert_eq!(
        (working.width, working.height),
        (uw, uh),
        "{name} orientation {o}"
    );
    for y in 0..uh {
        for x in 0..uw {
            let (sx, sy) = stored_coordinate(o, w, h, x, y);
            let at = ((sy * w + sx) * 4) as usize;
            assert_eq!(
                working.pixel(x, y).unwrap(),
                [stored[at], stored[at + 1], stored[at + 2], stored[at + 3]],
                "{name} orientation {o} at ({x}, {y})"
            );
        }
    }
}

// --- criterion 6: formats by content ----------------------------------------------------------

#[test]
fn ac6_tiff_avif_heic_jxl_ico_and_a_riff_wave_are_unsupported_by_content() {
    let contents: [(&str, Vec<u8>); 7] = [
        (
            "tiff little endian",
            b"II*\x00\x08\x00\x00\x00rest of a tiff".to_vec(),
        ),
        (
            "tiff big endian",
            b"MM\x00*\x00\x00\x00\x08rest of a tiff".to_vec(),
        ),
        (
            "avif",
            b"\x00\x00\x00\x1cftypavif\x00\x00\x00\x00avifmif1miaf".to_vec(),
        ),
        (
            "heic",
            b"\x00\x00\x00\x18ftypheic\x00\x00\x00\x00mif1heic".to_vec(),
        ),
        (
            "jpeg xl",
            b"\xff\x0a\xfa\x7f\x01\x90\x08\x00\x00\x00".to_vec(),
        ),
        (
            "ico",
            b"\x00\x00\x01\x00\x01\x00\x10\x10\x00\x00\x01\x00\x20\x00".to_vec(),
        ),
        (
            "riff wave",
            b"RIFF\x24\x00\x00\x00WAVEfmt \x10\x00\x00\x00".to_vec(),
        ),
    ];
    for (name, bytes) in contents {
        assert_eq!(ImageFormat::detect(&bytes), None, "{name}");
        assert_eq!(probe(&bytes).err(), Some(ImageError::Unsupported), "{name}");
        assert_eq!(
            decode_working(&bytes).err(),
            Some(ImageError::Unsupported),
            "{name}"
        );
    }
}

#[test]
fn ac6_an_animated_webp_of_2000_frames_gives_its_first_frame_in_bounded_time() {
    let bytes = tester_fixture("webp_2000_frames.webp");
    let run = measured(|| decode_working(&bytes));
    let working = run.result.unwrap();
    assert!(working.animated);
    assert_eq!((working.width, working.height), (1, 1));
    let [r, g, b, a] = working.pixel(0, 0).unwrap();
    assert!(
        r >= 254 && g == 0 && b == 0 && a == 255,
        "{:?}",
        [r, g, b, a]
    );
    assert!(
        run.cpu < budget(Duration::from_secs(2)),
        "took {:?}",
        run.cpu
    );
    assert!(run.peak_bytes < 300 * MIB, "peak {} bytes", run.peak_bytes);
}

#[test]
#[ignore = "0058 FAIL (minor, upstream image-webp): a lossless opaque frame blended onto the canvas reads 254 instead of 255"]
fn ac6_ac7_the_first_frame_of_a_lossless_animation_keeps_its_exact_pixels() {
    let working = decode_working(&tester_fixture("webp_2000_frames.webp")).unwrap();
    assert_eq!(working.pixel(0, 0), Some([255, 0, 0, 255]));
}

// --- criterion 7: colour rules ----------------------------------------------------------------

#[test]
fn ac7_sixteen_bit_channels_round_by_v_plus_128_over_257_not_by_the_high_byte() {
    // 385 -> 1 and 386 -> 2 by (v + 128) / 257; the high byte would give 1 for both.
    let values: [u16; 6] = [0, 385, 386, 32_896, 65_407, 65_535];
    let expect = |v: u16| u8::try_from((u32::from(v) + 128) / 257).unwrap();
    let mut rgb16 = Vec::new();
    for v in values {
        for channel in [v, 65_535 - v, v] {
            rgb16.extend_from_slice(&channel.to_ne_bytes());
        }
    }
    let png = encode_png(6, 1, &rgb16, image::ExtendedColorType::Rgb16);
    let working = decode_working(&png).unwrap();
    for (x, v) in (0_u32..).zip(values) {
        assert_eq!(
            working.pixel(x, 0).unwrap(),
            [expect(v), expect(65_535 - v), expect(v), 255],
            "value {v}"
        );
    }
    let mut la16 = Vec::new();
    for v in values {
        la16.extend_from_slice(&v.to_ne_bytes());
        la16.extend_from_slice(&(65_535 - v).to_ne_bytes());
    }
    let png = encode_png(6, 1, &la16, image::ExtendedColorType::La16);
    let working = decode_working(&png).unwrap();
    for (x, v) in (0_u32..).zip(values) {
        let g = expect(v);
        assert_eq!(
            working.pixel(x, 0).unwrap(),
            [g, g, g, expect(65_535 - v)],
            "grey+alpha {v}"
        );
    }
}

#[test]
fn ac7_an_rgb_trns_colour_becomes_transparent_with_its_colour_kept_straight() {
    let rgb = [10, 20, 30, 200, 100, 50];
    let png = encode_png(2, 1, &rgb, image::ExtendedColorType::Rgb8);
    let png = png_insert_before_idat(&png, &png_chunk(*b"tRNS", &[0, 10, 0, 20, 0, 30]));
    let working = decode_working(&png).unwrap();
    assert_eq!(
        working.pixel(0, 0),
        Some([10, 20, 30, 0]),
        "straight, not 0,0,0,0"
    );
    assert_eq!(working.pixel(1, 0), Some([200, 100, 50, 255]));
}

#[test]
fn ac7_an_inverted_adobe_cmyk_jpeg_is_converted_by_the_plain_formula() {
    // C, M, Y, K quadrants (0,255,255,0), (255,0,255,0), (255,255,0,0), (0,0,0,128):
    // R = 255 - min(255, C + K) etc. gives red, green, blue and grey 127.
    let working = decode_working(&tester_fixture("jpeg_cmyk_inverted.jpg")).unwrap();
    let expected = [
        ((4, 4), [255, 0, 0]),
        ((12, 4), [0, 255, 0]),
        ((4, 12), [0, 0, 255]),
        ((12, 12), [127, 127, 127]),
    ];
    for ((x, y), rgb) in expected {
        let got = working.pixel(x, y).unwrap();
        for c in 0..3 {
            assert!(
                got[c].abs_diff(rgb[c]) <= 3,
                "({x}, {y}): got {got:?}, expected {rgb:?}"
            );
        }
        assert_eq!(got[3], 255);
    }
}

#[test]
fn ac7_ac20_ztxt_and_iccp_chunks_that_inflate_to_256_mib_are_read_past_in_bounded_memory() {
    for name in ["png_ztxt_bomb.png", "png_iccp_bomb.png"] {
        let bytes = tester_fixture(name);
        let run = measured(|| decode_working(&bytes));
        assert!(
            run.peak_bytes < 300 * MIB,
            "{name}: peak {} bytes",
            run.peak_bytes
        );
        assert!(
            run.cpu < budget(Duration::from_secs(2)),
            "{name}: took {:?}",
            run.cpu
        );
        let working = run
            .result
            .expect("metadata is ignored, the picture imports");
        assert_eq!(working.pixel(0, 0), Some([10, 20, 30, 255]), "{name}");
    }
}

// --- criterion 8: orientation -----------------------------------------------------------------

#[test]
fn ac8_a_png_exif_chunk_orients_all_eight_values_pixel_exactly() {
    let (w, h, rgba) = asymmetric_rgba();
    for o in 1..=8_u16 {
        let png = encode_png(w, h, &rgba, image::ExtendedColorType::Rgba8);
        let png = png_insert_before_idat(&png, &png_chunk(*b"eXIf", &exif_tiff(o)));
        let working = decode_working(&png).unwrap();
        check_upright("png", &working, o, w, h, &rgba);
        assert_eq!(probe(&png).unwrap().orientation, u8::try_from(o).unwrap());
    }
}

#[test]
fn ac8_a_webp_exif_chunk_orients_all_eight_values_pixel_exactly() {
    let (w, h, rgba) = asymmetric_rgba();
    for o in 1..=8_u16 {
        let webp = webp_with_orientation(w, h, &rgba, o);
        let working = decode_working(&webp).unwrap();
        check_upright("webp", &working, o, w, h, &rgba);
    }
}

#[test]
fn ac8_ac18_a_turned_picture_over_the_working_side_is_reduced_and_then_upright() {
    // Stored 10000 x 100 with orientation 6: f = 8192 / 10000, stored working 8192 x 81, shown
    // upright 81 x 8192; the original size is the upright 100 x 10000.
    let mut grey = vec![200_u8; 10_000 * 100];
    for y in 0..100 {
        grey[y * 10_000] = 0; // the stored left column is black
    }
    let png = encode_png(10_000, 100, &grey, image::ExtendedColorType::L8);
    let png = png_insert_before_idat(&png, &png_chunk(*b"eXIf", &exif_tiff(6)));
    let working = decode_working(&png).unwrap();
    assert_eq!((working.width, working.height), (81, 8192));
    assert_eq!(working.original_size, (100, 10_000));
    assert!(working.is_reduced());
    // Orientation 6 turns the stored left column into the top row.
    let top = working.pixel(40, 0).unwrap();
    let bottom = working.pixel(40, 8191).unwrap();
    assert!(
        top[0] < 200,
        "the black column is averaged into the top row: {top:?}"
    );
    assert_eq!(bottom, [200, 200, 200, 255]);
}

// --- criterion 9: density ---------------------------------------------------------------------

fn png_with_phys(x_ppm: u32, y_ppm: u32, unit: u8) -> Vec<u8> {
    let png = encode_png(4, 2, &[90; 8], image::ExtendedColorType::L8);
    let mut phys = x_ppm.to_be_bytes().to_vec();
    phys.extend_from_slice(&y_ppm.to_be_bytes());
    phys.push(unit);
    png_insert_before_idat(&png, &png_chunk(*b"pHYs", &phys))
}

fn upright_dpi(bytes: &[u8]) -> (f64, f64) {
    let density = probe(bytes).unwrap().upright_density();
    (density.x_dpi(), density.y_dpi())
}

#[test]
fn ac9_phys_values_at_the_10_and_2400_dpi_edges() {
    let per_metre = |dpi: f64| dpi / 0.0254;
    let cases = [
        (393, 96.0),                 // 9.98 dpi: implausible
        (394, 394.0 * 0.0254),       // 10.008 dpi: used
        (94_488, 94_488.0 * 0.0254), // 2399.995 dpi: used
        (94_489, 96.0),              // 2400.02 dpi: implausible
        (0, 96.0),                   // zero
    ];
    for (ppm, dpi) in cases {
        let (x, y) = upright_dpi(&png_with_phys(ppm, ppm, 1));
        assert!(
            (x - dpi).abs() < 1e-6 && (y - dpi).abs() < 1e-6,
            "{ppm} px/m gives {x} x {y}"
        );
    }
    assert!((per_metre(300.0) - 11_811.02).abs() < 0.01);
    // Unit "unknown" gives only an aspect ratio: 96 dpi.
    assert_eq!(upright_dpi(&png_with_phys(11_811, 11_811, 0)), (96.0, 96.0));
    assert!(
        probe(&png_with_phys(11_811, 11_811, 0))
            .unwrap()
            .density
            .is_none()
    );
}

#[test]
fn ac9_one_implausible_axis_falls_back_to_96_on_both_axes() {
    // 300 dpi horizontally, 5 dpi vertically: the rule names a value below 10, and "96 dpi on
    // both axes", so a half-valid pair is not mixed into 300 x 96.
    let (x, y) = upright_dpi(&png_with_phys(11_811, 197, 1));
    assert_eq!((x, y), (DEFAULT_DPI, DEFAULT_DPI));
}

fn jpeg_with_jfif(units: u8, x: u16, y: u16) -> Vec<u8> {
    let mut jpeg = encode_jpeg(8, 8, &[128; 8 * 8 * 3]);
    let at = jpeg
        .windows(5)
        .position(|w| w == b"JFIF\0")
        .expect("a JFIF APP0");
    jpeg[at + 7] = units;
    jpeg[at + 8..at + 10].copy_from_slice(&x.to_be_bytes());
    jpeg[at + 10..at + 12].copy_from_slice(&y.to_be_bytes());
    jpeg
}

#[test]
fn ac9_jfif_dots_per_cm_are_converted_and_aspect_only_units_are_ignored() {
    let (x, y) = upright_dpi(&jpeg_with_jfif(2, 118, 59));
    assert!(
        (x - 299.72).abs() < 1e-9 && (y - 149.86).abs() < 1e-9,
        "{x} x {y}"
    );
    assert_eq!(upright_dpi(&jpeg_with_jfif(0, 3, 1)), (96.0, 96.0));
    assert_eq!(upright_dpi(&jpeg_with_jfif(1, 0, 0)), (96.0, 96.0));
}

#[test]
fn ac8_ac9_non_square_density_follows_a_quarter_turn() {
    // 150 x 300 dpi stored, orientation 6: upright the picture's x axis is the stored y axis.
    let png = encode_png(4, 2, &[90; 8], image::ExtendedColorType::L8);
    let mut phys = 5_906_u32.to_be_bytes().to_vec();
    phys.extend_from_slice(&11_811_u32.to_be_bytes());
    phys.push(1);
    let png = png_insert_before_idat(&png, &png_chunk(*b"pHYs", &phys));
    let png = png_insert_before_idat(&png, &png_chunk(*b"eXIf", &exif_tiff(6)));
    let (x, y) = upright_dpi(&png);
    assert!((x - 11_811.0 * 0.0254).abs() < 1e-6, "{x}");
    assert!((y - 5_906.0 * 0.0254).abs() < 1e-6, "{y}");
    assert_eq!(probe(&png).unwrap().upright_size(), (2, 4));
}

// --- criteria 16 to 19: limits ----------------------------------------------------------------

#[test]
fn ac16_the_constants_have_the_values_of_decision_d5() {
    assert_eq!(MAX_FILE_BYTES, 104_857_600);
    assert_eq!(MAX_HEADER_PIXELS, 50_000_000);
    assert_eq!(MAX_WORKING_PIXELS, 16_000_000);
    assert_eq!(MAX_WORKING_SIDE, 8192);
    assert_eq!(DECODER_HEADROOM_BYTES, 64 * 1024 * 1024);
}

#[test]
fn ac17_exactly_50_megapixels_passes_the_header_check_and_one_column_more_does_not() {
    let at_limit = png_header_only(10_000, 5_000);
    let info = probe(&at_limit).unwrap();
    assert_eq!((info.stored_width, info.stored_height), (10_000, 5_000));
    assert_eq!(
        probe(&png_header_only(10_001, 5_000)).err(),
        Some(ImageError::TooManyPixels {
            width: 10_001,
            height: 5_000
        })
    );
}

#[test]
fn ac17_a_file_of_exactly_100_mib_is_read_and_one_byte_more_is_refused() {
    let png = encode_png(2, 2, &[50; 4], image::ExtendedColorType::L8);
    let mut exact = png.clone();
    exact.resize(usize::try_from(MAX_FILE_BYTES).unwrap(), 0);
    assert!(
        probe(&exact).is_ok(),
        "trailing bytes after IEND are never read"
    );
    exact.push(0);
    assert_eq!(
        probe(&exact).err(),
        Some(ImageError::FileTooLarge {
            bytes: MAX_FILE_BYTES + 1
        })
    );
    assert_eq!(
        decode_working(&exact).err(),
        Some(ImageError::FileTooLarge {
            bytes: MAX_FILE_BYTES + 1
        })
    );
}

fn vp8l_header(width: u32, height: u32) -> Vec<u8> {
    let bits = (width - 1) | ((height - 1) << 14);
    let mut payload = vec![0x2F];
    payload.extend_from_slice(&bits.to_le_bytes());
    payload.extend_from_slice(&[0; 16]);
    let mut body = b"WEBP".to_vec();
    body.extend(riff_chunk(*b"VP8L", &payload));
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&u32::try_from(body.len()).unwrap().to_le_bytes());
    out.extend(body);
    out
}

#[test]
fn ac17_ac20_headers_over_the_ceiling_are_refused_before_any_buffer_exists() {
    let mut vp8x_canvas = b"WEBP".to_vec();
    let mut vp8x = vec![0x10, 0, 0, 0];
    vp8x.extend_from_slice(&u24(0xFF_FFFF));
    vp8x.extend_from_slice(&u24(0xFF_FFFF));
    vp8x_canvas.extend(riff_chunk(*b"VP8X", &vp8x));
    vp8x_canvas.extend(riff_chunk(*b"ALPH", &[0; 8]));
    let mut riff = b"RIFF".to_vec();
    riff.extend_from_slice(&u32::try_from(vp8x_canvas.len()).unwrap().to_le_bytes());
    riff.extend(vp8x_canvas);
    let cases: [(&str, Vec<u8>); 3] = [
        ("webp vp8l 16384 x 16384", vp8l_header(16_384, 16_384)),
        (
            "png 2^31-1 square",
            png_header_only(0x7FFF_FFFF, 0x7FFF_FFFF),
        ),
        ("webp vp8x canvas 2^24 square", riff),
    ];
    for (name, bytes) in cases {
        let run = measured(|| decode_working(&bytes));
        assert!(
            matches!(
                run.result,
                Err(ImageError::TooManyPixels { .. } | ImageError::Damaged)
            ),
            "{name}: {:?}",
            run.result.map(|w| (w.width, w.height))
        );
        assert!(
            run.peak_bytes < 16 * MIB,
            "{name}: peak {} bytes",
            run.peak_bytes
        );
        assert!(
            run.cpu < budget(Duration::from_millis(500)),
            "{name}: took {:?}",
            run.cpu
        );
    }
}

fn spec_working_size(w: u32, h: u32) -> (u32, u32) {
    let (wf, hf) = (f64::from(w), f64::from(h));
    let f = 1.0_f64
        .min(8192.0 / wf.max(hf))
        .min((16_000_000.0 / (wf * hf)).sqrt());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    ((wf * f).floor() as u32, (hf * f).floor() as u32)
}

#[test]
fn ac18_the_working_size_follows_the_formula_at_its_edges() {
    for (w, h) in [
        (5000, 4000),
        (4000, 4000),
        (4001, 4000),
        (8192, 1953),
        (8193, 10),
        (9000, 1000),
        (7071, 7071),
        (1, 8192),
        (20_000, 2500),
    ] {
        assert_eq!(working_size(w, h), spec_working_size(w, h), "{w} x {h}");
    }
    assert_eq!(working_size(5000, 4000), (4472, 3577));
}

#[test]
fn ac18_a_long_thin_picture_never_gets_a_zero_side() {
    // The formula's floor gives 0 for 20,000 x 1 (f = 0.4096); a working image of zero pixels
    // cannot be drawn or traced, so at least one pixel must stay on each side.
    let (w, h) = working_size(20_000, 1);
    assert!(w >= 1 && h >= 1, "{w} x {h}");
    let png = encode_png(20_000, 1, &vec![77; 20_000], image::ExtendedColorType::L8);
    let working = decode_working(&png).unwrap();
    assert!(working.width >= 1 && working.height >= 1);
    assert_eq!(
        working.pixels.len(),
        (working.width * working.height * 4) as usize
    );
}

#[test]
fn ac18_the_box_filter_averages_areas_instead_of_picking_pixels() {
    // A 1-pixel checkerboard of 9000 x 2 becomes 8192 x 1: every output pixel covers equal
    // amounts of black and white, so it is mid-grey; nearest-neighbour would give 0 or 255.
    let grey: Vec<u8> = (0..2_u32)
        .flat_map(|y| (0..9000_u32).map(move |x| if (x + y) % 2 == 0 { 0 } else { 255 }))
        .collect();
    let png = encode_png(9000, 2, &grey, image::ExtendedColorType::L8);
    let working = decode_working(&png).unwrap();
    assert_eq!((working.width, working.height), (8192, 1));
    for x in 0..8192 {
        let [r, g, b, a] = working.pixel(x, 0).unwrap();
        assert!((126..=129).contains(&r), "x {x}: {r}");
        assert_eq!((r, a), (g, 255));
        assert_eq!(r, b);
    }
    assert_eq!(
        decode_working(&png).unwrap().pixels,
        working.pixels,
        "deterministic"
    );
}

#[test]
fn ac19_a_49_megapixel_zero_png_decodes_within_the_memory_cap() {
    // A valid 7000 x 7000 RGB8 file of 143 KB: decoder buffer 147 MB, working image 4000 x 4000
    // RGBA8 64 MB, cap = both + 64 MiB (criterion 19, spike correction 2).
    let bytes = tester_fixture("png_zeros_7000x7000.png");
    let run = measured(|| decode_working(&bytes));
    let working = run.result.unwrap();
    assert_eq!((working.width, working.height), (4000, 4000));
    assert_eq!(working.pixel(1234, 2345), Some([0, 0, 0, 255]));
    if std::path::Path::new("/proc/self/clear_refs").exists() {
        assert!(
            run.peak_bytes > 100 * MIB,
            "the measurement sees the decoder buffer: {} MiB",
            run.peak_bytes / MIB
        );
    }
    let cap = 7000 * 7000 * 3 + 4000 * 4000 * 4 + 64 * MIB;
    assert!(
        run.peak_bytes <= cap,
        "peak {} MiB over the cap {} MiB",
        run.peak_bytes / MIB,
        cap / MIB
    );
}

// --- criterion 20: hostile files --------------------------------------------------------------

const HOSTILE_SPEC_FIXTURES: [&str; 9] = [
    "empty.png",
    "png_truncated.png",
    "png_bad_crc.png",
    "webp_truncated.webp",
    "png_bomb_60000x60000.png",
    "png_zero_width.png",
    "jpeg_endless_markers.jpg",
    "jpeg_dimensions_65535x65535.jpg",
    "text_renamed.png",
];

#[test]
fn ac20_each_hostile_fixture_fails_within_2_s_of_cpu_and_300_mib() {
    for name in HOSTILE_SPEC_FIXTURES {
        let bytes = spec_fixture(name);
        let run = measured(|| (probe(&bytes).err(), decode_working(&bytes).err()));
        assert!(
            run.result.0.is_some() || run.result.1.is_some(),
            "{name} imported"
        );
        assert!(run.result.1.is_some(), "{name}: decode_working accepted it");
        assert!(
            run.cpu < budget(Duration::from_secs(2)),
            "{name}: took {:?}",
            run.cpu
        );
        assert!(
            run.peak_bytes < 300 * MIB,
            "{name}: peak {} bytes",
            run.peak_bytes
        );
    }
}

#[test]
fn ac20_a_png_with_an_invalid_palette_index_imports_with_the_index_black() {
    // Criterion 20 as amended 2026-10-27: an invalid palette index may import, reading black.
    let working = decode_working(&spec_fixture("png_bad_palette_index.png")).unwrap();
    assert_eq!(working.pixel(0, 0), Some([0, 0, 255, 255]));
    assert_eq!(working.pixel(1, 0), Some([0, 0, 0, 255]));
}

#[test]
fn ac20_a_webp_with_an_overflowing_chunk_size_is_not_imported() {
    assert_eq!(
        decode_working(&spec_fixture("webp_chunk_overflow.webp")).err(),
        Some(ImageError::Damaged)
    );
}

#[test]
fn ac20_the_bad_palette_index_and_the_chunk_overflow_are_at_least_memory_safe() {
    for name in ["png_bad_palette_index.png", "webp_chunk_overflow.webp"] {
        let bytes = spec_fixture(name);
        let run = measured(|| decode_working(&bytes).map(|w| (w.width, w.height)));
        assert!(
            run.cpu < budget(Duration::from_secs(2)),
            "{name}: took {:?}",
            run.cpu
        );
        assert!(
            run.peak_bytes < 16 * MIB,
            "{name}: peak {} bytes",
            run.peak_bytes
        );
    }
}

#[test]
fn ac20_a_riff_size_of_4_gib_and_a_chunk_past_the_end_never_allocate_their_claim() {
    let good = encode_webp_lossless(4, 4, &[9; 64]);
    let mut riff_huge = good.clone();
    riff_huge[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
    let mut chunk_huge = good;
    chunk_huge[16..20].copy_from_slice(&0xFFFF_FFF0_u32.to_le_bytes());
    for (name, bytes) in [("riff size", riff_huge), ("vp8l size", chunk_huge)] {
        let run = measured(|| decode_working(&bytes).map(|w| (w.width, w.height)));
        assert!(
            run.peak_bytes < 16 * MIB,
            "{name}: peak {} bytes",
            run.peak_bytes
        );
        assert!(
            run.cpu < budget(Duration::from_millis(500)),
            "{name}: took {:?}",
            run.cpu
        );
    }
}

#[test]
fn ac20_a_png_cut_anywhere_inside_its_image_data_fails_without_panic() {
    let rgba: Vec<u8> = (0..32 * 32 * 4)
        .map(|i| u8::try_from(i * 7 % 251).unwrap())
        .collect();
    let png = encode_png(32, 32, &rgba, image::ExtendedColorType::Rgba8);
    let iend = png.len() - 12;
    let idat = png.windows(4).position(|w| w == b"IDAT").unwrap() - 4;
    for cut in idat..iend {
        assert!(
            decode_working(&png[..cut]).is_err(),
            "cut at {cut} of {} imported",
            png.len()
        );
    }
}

#[test]
fn ac20_a_webp_cut_anywhere_fails_without_panic() {
    let rgba: Vec<u8> = (0..32 * 32 * 4)
        .map(|i| u8::try_from(i * 13 % 251).unwrap())
        .collect();
    let webp = encode_webp_lossless(32, 32, &rgba);
    for cut in 12..webp.len() - 1 {
        assert!(
            decode_working(&webp[..cut]).is_err(),
            "cut at {cut} of {} imported",
            webp.len()
        );
    }
}

#[test]
fn ac20_a_progressive_jpeg_with_5000_repeated_scans_ends_within_2_s_of_cpu() {
    let bytes = tester_fixture("jpeg_progressive_scan_bomb.jpg");
    let run = measured(|| decode_working(&bytes).map(|w| (w.width, w.height)));
    assert!(run.peak_bytes < 300 * MIB, "peak {} bytes", run.peak_bytes);
    assert!(
        run.cpu < budget(Duration::from_secs(2)),
        "took {:?} ({:?})",
        run.cpu,
        run.result
    );
}

// --- criterion 21: truncated JPEG -------------------------------------------------------------

fn gradient_jpeg() -> Vec<u8> {
    let rgb: Vec<u8> = (0..64_u32 * 64)
        .flat_map(|i| {
            [
                u8::try_from(i % 64 * 4).unwrap(),
                90,
                u8::try_from(i / 64 * 4).unwrap(),
            ]
        })
        .collect();
    encode_jpeg(64, 64, &rgb)
}

#[test]
fn ac21_a_jpeg_cut_before_its_first_scan_is_damaged_not_partial() {
    let jpeg = gradient_jpeg();
    let sos = jpeg.windows(2).position(|w| w == [0xFF, 0xDA]).unwrap();
    for cut in [sos, sos - 1, sos / 2, 4] {
        let result = decode_working(&jpeg[..cut]);
        assert!(
            matches!(result, Err(ImageError::Damaged)),
            "cut at {cut}: {:?}",
            result.map(|w| w.truncated)
        );
    }
}

#[test]
fn ac21_cuts_after_the_first_scan_are_truncated_and_a_complete_file_is_not() {
    let jpeg = gradient_jpeg();
    let sos = jpeg.windows(2).position(|w| w == [0xFF, 0xDA]).unwrap();
    for cut in [sos + 20, usize::midpoint(sos, jpeg.len()), jpeg.len() - 2] {
        let working = decode_working(&jpeg[..cut]).unwrap();
        assert!(working.truncated, "cut at {cut}");
        assert_eq!((working.width, working.height), (64, 64));
    }
    assert!(!decode_working(&jpeg).unwrap().truncated);
    let mut trailing = jpeg.clone();
    trailing.extend_from_slice(b"\x00\x00 trailing bytes after the end marker");
    assert!(!decode_working(&trailing).unwrap().truncated);
}

#[test]
fn ac21_an_exif_thumbnail_end_marker_does_not_hide_a_cut_main_image() {
    // Camera files carry a thumbnail JPEG (with its own SOS and EOI) inside APP1; the main
    // picture is cut. The thumbnail's end marker belongs to APP1, not to the file.
    let working = decode_working(&tester_fixture("jpeg_thumbnail_truncated.jpg")).unwrap();
    assert_eq!((working.width, working.height), (64, 64));
    assert!(
        working.truncated,
        "the cut main image must carry the notice"
    );
}

// --- criterion 28: the mip chain for the GPU upload -------------------------------------------

#[test]
fn ac28_the_mip_chain_halves_down_to_one_pixel_and_premultiplies() {
    let rgba: Vec<u8> = (0..5 * 3).flat_map(|_| [200, 100, 50, 128]).collect();
    let png = encode_png(5, 3, &rgba, image::ExtendedColorType::Rgba8);
    let working = decode_working(&png).unwrap();
    let mips = working.premultiplied_mips();
    let sizes: Vec<_> = mips.iter().map(|m| (m.width, m.height)).collect();
    assert_eq!(sizes, [(5, 3), (2, 1), (1, 1)]);
    for level in &mips {
        for px in level.pixels.chunks(4) {
            assert_eq!(px[3], 128);
            for (c, straight) in [(0, 200_u32), (1, 100), (2, 50)] {
                let exact = straight * 128 / 255;
                assert!(u32::from(px[c]).abs_diff(exact) <= 1, "{px:?}");
            }
        }
    }
}

// --- fuzz findings (criterion 19: a panic aborts the wasm module) -------------------------------

/// An 8-bit indexed 2 x 1 PNG whose PLTE chunk has `plte_len` bytes (CRCs valid).
fn png_with_plte_len(plte_len: usize) -> Vec<u8> {
    let mut ihdr = 2_u32.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&1_u32.to_be_bytes());
    ihdr.extend_from_slice(&[8, 3, 0, 0, 0]);
    let plte: Vec<u8> = (0..plte_len)
        .map(|i| u8::try_from(i * 40 % 256).unwrap())
        .collect();
    let idat = encode_png(3, 1, &[0, 0, 1], image::ExtendedColorType::L8);
    let at = idat.windows(4).position(|w| w == b"IDAT").unwrap();
    let len = u32::from_be_bytes(idat[at - 4..at].try_into().unwrap()) as usize;
    let zlib = &idat[at + 4..at + 4 + len];
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    out.extend(png_chunk(*b"IHDR", &ihdr));
    out.extend(png_chunk(*b"PLTE", &plte));
    out.extend(png_chunk(*b"IDAT", zlib));
    out.extend(png_chunk(*b"IEND", &[]));
    out
}

#[test]
fn ac19_ac20_a_plte_chunk_that_is_not_a_multiple_of_3_fails_without_panic() {
    // Found by the decode_png fuzz target (with CRC checks off); rebuilt here with valid CRCs.
    for len in [4, 5, 7, 8] {
        let bytes = png_with_plte_len(len);
        let result =
            std::panic::catch_unwind(|| decode_working(&bytes).map(|w| (w.width, w.height)));
        assert!(result.is_ok(), "PLTE of {len} bytes: the decoder panicked");
    }
}

/// Runs the decode of tester fixture `name` in a child of this test binary whose address space is
/// limited to 1.5 GB (as a wasm32 heap is bounded); `true` when the child ends normally.
fn decodes_within_address_limit(test: &str, name: &str) -> bool {
    if let Some(inner) = std::env::var_os("CURVYO_TESTER_DECODE") {
        let _ = decode_working(&tester_fixture(&inner.to_string_lossy()));
        return true;
    }
    let exe = std::env::current_exe().unwrap();
    // Only Linux enforces `ulimit -v` (macOS refuses it, Windows has no `sh`); elsewhere the
    // child runs unlimited and the test still checks that the decode ends normally.
    #[cfg(target_os = "linux")]
    let mut command = {
        let mut command = std::process::Command::new("sh");
        command
            .arg("-c")
            .arg("ulimit -v 1500000 && exec \"$0\" --exact \"$1\" --include-ignored --test-threads 1")
            .arg(&exe)
            .arg(test);
        command
    };
    #[cfg(not(target_os = "linux"))]
    let mut command = {
        let mut command = std::process::Command::new(&exe);
        command.args(["--exact", test, "--include-ignored", "--test-threads", "1"]);
        command
    };
    let status = command
        .env("CURVYO_TESTER_DECODE", name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .unwrap();
    status.success()
}

#[test]
fn ac19_ac20_a_webp_exif_chunk_claiming_4_gib_fails_without_allocating_it() {
    assert!(
        decodes_within_address_limit(
            "ac19_ac20_a_webp_exif_chunk_claiming_4_gib_fails_without_allocating_it",
            "fuzz_webp_exif_claims_4gib.webp"
        ),
        "the decoder aborted on a 108-byte file under a 1.5 GB address limit"
    );
}

#[test]
fn ac19_the_address_limit_harness_lets_a_normal_file_through() {
    assert!(decodes_within_address_limit(
        "ac19_the_address_limit_harness_lets_a_normal_file_through",
        "jpeg_cmyk_inverted.jpg"
    ));
}
