//! Golden tests over every image fixture (`specs/0058-bitmap-import` "Fixtures", criteria 6 to 9,
//! 18 to 21): each file imports or fails exactly as `expected.txt` says, hostile files fail fast,
//! and two decodes are equal.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::many_single_char_names,
    clippy::case_sensitive_file_extension_comparisons,
    missing_docs
)]

mod common;

use std::path::PathBuf;
use std::time::Duration;

use common::thread_cpu;

use curvyo_image_core::{ImageError, decode_working, probe};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/images")
}

fn expectations() -> Vec<(String, String)> {
    let text = std::fs::read_to_string(fixture_dir().join("expected.txt")).unwrap();
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (name, rest) = line.split_once('|').unwrap();
            (name.trim().to_owned(), rest.trim().to_owned())
        })
        .collect()
}

fn error_word(error: ImageError) -> &'static str {
    match error {
        ImageError::Unsupported => "unsupported",
        ImageError::Damaged => "damaged",
        ImageError::TooManyPixels { .. } => "too-many-pixels",
        ImageError::FileTooLarge { .. } => "file-too-large",
        ImageError::OutOfMemory => "out-of-memory",
    }
}

fn parse_numbers<const N: usize>(text: &str, separator: char) -> [f64; N] {
    let values: Vec<f64> = text.split(separator).map(|v| v.parse().unwrap()).collect();
    values.try_into().unwrap()
}

fn check(name: &str, expected: &str) -> Result<(), String> {
    let bytes = std::fs::read(fixture_dir().join(name)).unwrap();
    let mut words = expected.split_whitespace();
    let started = thread_cpu();
    let decoded = decode_working(&bytes);
    let elapsed = thread_cpu().saturating_sub(started);
    if elapsed > Duration::from_secs(2) {
        return Err(format!("{name}: took {elapsed:?}"));
    }
    match words.next() {
        Some("err") => {
            let want = words.next().unwrap();
            match decoded {
                Err(error) if error_word(error) == want => Ok(()),
                Err(other) => Err(format!("{name}: expected {want}, got {other:?}")),
                Ok(image) => Err(format!(
                    "{name}: expected {want}, got a {}x{} image",
                    image.width, image.height
                )),
            }
        }
        Some("ok") => {
            let image = decoded.map_err(|e| format!("{name}: {e:?}"))?;
            let info = probe(&bytes).unwrap();
            let format = words.next().unwrap();
            if info.format.name() != format {
                return Err(format!("{name}: format {}", info.format.name()));
            }
            let [w, h] = parse_numbers::<2>(words.next().unwrap(), 'x');
            if (f64::from(image.width), f64::from(image.height)) != (w, h) {
                return Err(format!("{name}: size {}x{}", image.width, image.height));
            }
            let rest: Vec<&str> = words.collect();
            let tolerance = rest
                .iter()
                .position(|w| *w == "tol")
                .map_or(0, |i| rest[i + 1].parse::<i32>().unwrap());
            let mut i = 0;
            while i < rest.len() {
                match rest[i] {
                    "dpi" => {
                        let value = rest[i + 1];
                        match (value, info.density) {
                            ("-", None) => {}
                            ("-", Some(d)) => return Err(format!("{name}: density {d:?}")),
                            (_, None) => return Err(format!("{name}: no density")),
                            (_, Some(d)) => {
                                let [x, y] = parse_numbers::<2>(value, 'x');
                                if (d.x_dpi() - x).abs() > 1e-3 || (d.y_dpi() - y).abs() > 1e-3 {
                                    return Err(format!("{name}: density {d:?}"));
                                }
                            }
                        }
                        i += 2;
                    }
                    "orient" => {
                        if info.orientation.to_string() != rest[i + 1] {
                            return Err(format!("{name}: orientation {}", info.orientation));
                        }
                        i += 2;
                    }
                    "animated" => {
                        if !image.animated {
                            return Err(format!("{name}: not animated"));
                        }
                        i += 1;
                    }
                    "truncated" => {
                        if !image.truncated {
                            return Err(format!("{name}: not truncated"));
                        }
                        i += 1;
                    }
                    "px" => {
                        let (at, rgba) = rest[i + 1].split_once('=').unwrap();
                        let [x, y] = parse_numbers::<2>(at, ',');
                        let want = parse_numbers::<4>(rgba, ',');
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let got = image.pixel(x as u32, y as u32).unwrap();
                        #[allow(clippy::cast_possible_truncation)]
                        let off = got
                            .iter()
                            .zip(want)
                            .any(|(&g, w)| (i32::from(g) - w as i32).abs() > tolerance);
                        if off {
                            return Err(format!("{name}: pixel {at} is {got:?}, want {want:?}"));
                        }
                        i += 2;
                    }
                    "tol" => i += 2,
                    other => panic!("unknown word {other} in expected.txt"),
                }
            }
            if image.truncated && !rest.contains(&"truncated") {
                return Err(format!("{name}: truncated"));
            }
            Ok(())
        }
        other => panic!("{name}: bad expectation {other:?}"),
    }
}

#[test]
fn every_fixture_imports_or_fails_as_expected() {
    let failures: Vec<String> = expectations()
        .iter()
        .filter_map(|(name, expected)| check(name, expected).err())
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn every_fixture_file_has_an_expectation() {
    let named: Vec<String> = expectations().into_iter().map(|(name, _)| name).collect();
    for entry in std::fs::read_dir(fixture_dir()).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        if name.ends_with(".py") || name == "expected.txt" {
            continue;
        }
        assert!(named.contains(&name), "{name} has no line in expected.txt");
    }
}

#[test]
fn bmp_and_gif_are_refused_by_their_signature() {
    let mut bmp = b"BM".to_vec();
    bmp.extend_from_slice(&[0; 60]);
    assert_eq!(decode_working(&bmp), Err(ImageError::Unsupported));
    assert_eq!(
        decode_working(b"GIF89a\x01\x00\x01\x00\x00\x00\x00;"),
        Err(ImageError::Unsupported)
    );
}

#[test]
fn a_file_over_100_mib_is_refused_before_anything_is_read() {
    let mut big = vec![0_u8; 104_857_601];
    big[..8].copy_from_slice(b"\x89PNG\r\n\x1a\n");
    assert_eq!(
        probe(&big),
        Err(ImageError::FileTooLarge { bytes: 104_857_601 })
    );
}

#[test]
fn two_decodes_give_identical_pixels() {
    for name in ["jpeg_progressive.jpg", "webp_lossy.webp", "png_gray_16.png"] {
        let bytes = std::fs::read(fixture_dir().join(name)).unwrap();
        assert_eq!(decode_working(&bytes), decode_working(&bytes), "{name}");
    }
}
