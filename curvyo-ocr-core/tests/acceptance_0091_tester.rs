//! Tester cases for `specs/0091-trace-text-recognition` slice 1 at the reader: the tester's own
//! posters (`tests/fixtures/tester/`, rendered by `make_posters.py` from the included faces and
//! one face Curvyo does not include) read with the shipped weights. Written from the
//! specification before the implementation was read; the reference set of the implementer is
//! covered by `reference.rs` and not repeated here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss
)]

use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use curvyo_ocr_core::{OcrError, OcrModel, OcrProgress, TextLine, read_lines};
use serde::Deserialize;

#[derive(Deserialize)]
struct Truth {
    images: Vec<Image>,
}

#[derive(Deserialize)]
struct Image {
    file: String,
    lines: Vec<String>,
    rotation_degrees: Option<f32>,
    foreign: Option<String>,
}

fn models() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../frontend/public/models/paddleocr-latin")
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/tester")
}

fn files() -> (Vec<u8>, Vec<u8>, String) {
    let read = |name: &str| std::fs::read(models().join(name)).unwrap();
    (
        read("det.onnx"),
        read("rec.onnx"),
        String::from_utf8(read("dict.txt")).unwrap(),
    )
}

fn model() -> OcrModel {
    let (det, rec, dict) = files();
    OcrModel::load(&det, &rec, &dict).unwrap()
}

fn truth() -> Truth {
    toml::from_str(&std::fs::read_to_string(fixtures().join("truth.toml")).unwrap()).unwrap()
}

fn raster(file: &str) -> curvyo_image_core::WorkingImage {
    curvyo_image_core::decode_working(&std::fs::read(fixtures().join(file)).unwrap()).unwrap()
}

fn go(_: OcrProgress) -> ControlFlow<()> {
    ControlFlow::Continue(())
}

fn read(model: &OcrModel, file: &str) -> Vec<TextLine> {
    let image = raster(file);
    read_lines(model, image.width, image.height, &image.pixels, &mut go).unwrap()
}

/// The direction of a quad's top edge in degrees, pixel space.
fn angle(line: &TextLine) -> f32 {
    let [a, b, _, _] = line.quad;
    (b[1] - a[1]).atan2(b[0] - a[0]).to_degrees()
}

/// Criteria 1 and 2 on the tester's poster: every line in an included face reads exactly, and the
/// line in the foreign face (criterion 4) is at least found.
#[test]
fn tester_poster_lines_read_exactly_with_umlauts_and_sharp_s() {
    let model = model();
    let image = truth().images.into_iter().next().unwrap();
    let lines = read(&model, &image.file);
    let got: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
    eprintln!("tester poster read: {got:?}");
    for want in &image.lines {
        if image.foreign.as_ref() == Some(want) {
            assert!(
                got.iter()
                    .any(|g| g.chars().count() == want.chars().count()),
                "the foreign line {want:?} is not found; read {got:?}"
            );
        } else {
            assert!(
                got.contains(&want.as_str()),
                "line {want:?} is not read exactly; read {got:?}"
            );
        }
    }
    for line in &lines {
        assert_eq!(
            line.text.chars().count(),
            line.confidence.len(),
            "one confidence per character in {:?}",
            line.text
        );
        assert!(line.confidence.iter().all(|c| (0.0..=1.0).contains(c)));
    }
}

/// Criterion 2: every umlaut and ß of the source is in the read text at its place.
#[test]
fn tester_poster_umlauts_and_sharp_s_per_character() {
    let model = model();
    let image = truth().images.into_iter().next().unwrap();
    let lines = read(&model, &image.file);
    let mut checked = 0;
    for want in &image.lines {
        let Some(line) = lines.iter().find(|l| {
            l.text.chars().count() == want.chars().count()
                && l.text
                    .chars()
                    .zip(want.chars())
                    .filter(|(a, b)| a != b)
                    .count()
                    <= 1
        }) else {
            panic!("no line of the length of {want:?} in {lines:?}");
        };
        for (i, (got, src)) in line.text.chars().zip(want.chars()).enumerate() {
            if "äöüßÄÖÜ".contains(src) {
                checked += 1;
                assert_eq!(got, src, "{want:?} position {}", i + 1);
            }
        }
    }
    assert!(checked >= 7, "the poster holds at least 7 umlauts and ß");
}

/// Criterion 8, generalised to 7 and 30 degrees: the line is read and its quad has the rotation
/// within 1 degree.
#[test]
fn rotated_lines_keep_their_rotation_within_one_degree() {
    let model = model();
    for image in truth()
        .images
        .into_iter()
        .filter(|i| matches!(i.rotation_degrees, Some(d) if d.abs() < 45.0))
    {
        let lines = read(&model, &image.file);
        assert_eq!(lines.len(), 1, "{}: {lines:?}", image.file);
        assert_eq!(lines[0].text, image.lines[0], "{}", image.file);
        let want = image.rotation_degrees.unwrap();
        let got = angle(&lines[0]);
        assert!(
            (got - want).abs() <= 1.0,
            "{}: rotation {got} wanted {want}",
            image.file
        );
    }
}

/// A line turned by 90 degrees (reading upwards). Not a criterion of the spec (criterion 8 names
/// 15 degrees; vertical text is out of scope); reported, not asserted beyond "no crash".
#[test]
fn a_line_turned_ninety_degrees_is_reported() {
    let model = model();
    let lines = read(&model, "rotated_90.png");
    eprintln!(
        "rotated_90: {:?}",
        lines
            .iter()
            .map(|l| (l.text.clone(), angle(l)))
            .collect::<Vec<_>>()
    );
}

/// Criterion 18 at the reader: a picture with no text gives no line.
#[test]
fn a_plain_shape_gives_no_line() {
    let model = model();
    assert_eq!(read(&model, "plain.png"), Vec::new());
}

/// Criterion 13 at the reader: the same picture twice gives the same lines, bit for bit.
#[test]
fn tester_poster_reads_the_same_twice() {
    let model = model();
    assert_eq!(read(&model, "poster.png"), read(&model, "poster.png"));
    let other = self::model();
    assert_eq!(
        read(&model, "rotated_30.png"),
        read(&other, "rotated_30.png")
    );
}

/// Criterion 19 at the reader: a stop asked for in the first poll, and one asked while the lines
/// are read, both end with `Cancelled`; the model reads normally afterwards.
#[test]
fn a_stop_during_the_read_is_cancelled_and_leaves_the_model_usable() {
    let model = model();
    let image = raster("poster.png");
    let mut stop_first = |_: OcrProgress| ControlFlow::Break(());
    assert_eq!(
        read_lines(
            &model,
            image.width,
            image.height,
            &image.pixels,
            &mut stop_first
        ),
        Err(OcrError::Cancelled)
    );
    let mut seen = Vec::new();
    let mut stop_in_lines = |p: OcrProgress| {
        seen.push(p);
        match p {
            OcrProgress::Reading { done: 2, .. } => ControlFlow::Break(()),
            _ => ControlFlow::Continue(()),
        }
    };
    assert_eq!(
        read_lines(
            &model,
            image.width,
            image.height,
            &image.pixels,
            &mut stop_in_lines
        ),
        Err(OcrError::Cancelled)
    );
    assert_eq!(seen[0], OcrProgress::Detecting);
    assert!(matches!(seen[1], OcrProgress::Reading { done: 0, total } if total >= 7));
    assert_eq!(read(&model, "rotated_7.png").len(), 1);
}

/// The progress the bar shows (ux-notes §3): detecting first, then lines counting up, never
/// backwards, ending at `total of total`.
#[test]
fn progress_counts_lines_up_and_ends_full() {
    let model = model();
    let image = raster("poster.png");
    let mut seen = Vec::new();
    let lines = read_lines(&model, image.width, image.height, &image.pixels, &mut |p| {
        seen.push(p);
        ControlFlow::Continue(())
    })
    .unwrap();
    assert_eq!(seen[0], OcrProgress::Detecting);
    let done: Vec<usize> = seen[1..]
        .iter()
        .map(|p| match p {
            OcrProgress::Reading { done, .. } => *done,
            OcrProgress::Detecting => panic!("detecting twice"),
        })
        .collect();
    assert!(done.windows(2).all(|w| w[0] < w[1]), "{done:?}");
    let OcrProgress::Reading { done, total } = *seen.last().unwrap() else {
        panic!()
    };
    assert_eq!(done, total);
    assert!(total >= lines.len());
}

/// Criterion 15: a weight file whose bytes differ from the record is refused, not parsed; a swapped
/// detector and recogniser are refused too.
#[test]
fn a_changed_or_swapped_weight_file_is_refused() {
    let (det, rec, dict) = files();
    let mut changed = det.clone();
    let last = changed.len() - 1;
    changed[last] ^= 1;
    assert_eq!(
        OcrModel::load(&changed, &rec, &dict).err(),
        Some(OcrError::WrongModel)
    );
    assert_eq!(
        OcrModel::load(&rec, &det, &dict).err(),
        Some(OcrError::WrongModel)
    );
    let mut alphabet = dict.clone();
    alphabet.push('x');
    assert_eq!(
        OcrModel::load(&det, &rec, &alphabet).err(),
        Some(OcrError::WrongModel)
    );
    assert_eq!(
        OcrModel::load(&[], &[], "").err(),
        Some(OcrError::WrongModel)
    );
}

/// Degenerate rasters: empty, size not matching the bytes, a single pixel, a 1 px high strip.
/// Errors are `Invalid`, never a panic; tiny rasters give no line.
#[test]
fn degenerate_rasters_are_invalid_or_empty_never_a_panic() {
    let model = model();
    assert!(matches!(
        read_lines(&model, 0, 0, &[], &mut go),
        Err(OcrError::Invalid(_))
    ));
    assert!(matches!(
        read_lines(&model, 10, 10, &[255; 4 * 99], &mut go),
        Err(OcrError::Invalid(_))
    ));
    assert!(matches!(
        read_lines(&model, 0, 10, &[], &mut go),
        Err(OcrError::Invalid(_))
    ));
    assert_eq!(
        read_lines(&model, 1, 1, &[0, 0, 0, 255], &mut go).unwrap(),
        Vec::new()
    );
    assert_eq!(
        read_lines(&model, 3000, 1, &vec![0; 4 * 3000], &mut go).unwrap(),
        Vec::new()
    );
    // Fully transparent black composites on white: no text.
    assert_eq!(
        read_lines(&model, 64, 64, &[0; 4 * 64 * 64], &mut go).unwrap(),
        Vec::new()
    );
}

/// Criterion 20 (Proposal, ≤ 5 s once per image, wasm): native release CPU time of one read of the
/// tester poster with one `rayon` thread (`RAYON_NUM_THREADS=1`, the wasm worker's case),
/// measured as process CPU time since `rten` runs on `rayon`'s threads, not the caller's. Run:
/// `RAYON_NUM_THREADS=1 cargo test --release -p curvyo-ocr-core --test acceptance_0091_tester
/// -- --ignored --nocapture`.
#[test]
#[ignore = "timing, release only"]
fn ocr_time_of_the_tester_poster() {
    let model = model();
    let image = raster("poster.png");
    let _ = read_lines(&model, image.width, image.height, &image.pixels, &mut go).unwrap();
    let cpu = || {
        let text = std::fs::read_to_string("/proc/self/stat").unwrap();
        let fields: Vec<&str> = text
            .rsplit(')')
            .next()
            .unwrap()
            .split_whitespace()
            .collect();
        let ticks: u64 = fields[11].parse::<u64>().unwrap() + fields[12].parse::<u64>().unwrap();
        ticks as f64 / 100.0
    };
    let mut best = f64::MAX;
    for _ in 0..3 {
        let start = cpu();
        let lines = read_lines(&model, image.width, image.height, &image.pixels, &mut go).unwrap();
        best = best.min(cpu() - start);
        assert!(lines.len() >= 7);
    }
    eprintln!("OCR tester poster, process CPU (best of 3): {best:.2} s");
    assert!(best <= 5.0, "OCR {best} s over the 5 s target");
}
