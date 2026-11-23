//! Tester acceptance cases for `specs/0135-image-optimisation` slice 3 in the model runtime,
//! written from the specification (criteria 47, 50, 63, 64, 66) before the implementation was
//! read: only the pinned bytes load (a flipped byte, a truncated or partial file, an extra byte
//! and every placeholder are refused), the one URL per model on the one host with no query, the
//! permissive licences and the size targets, Upscale over the working limit refused before the
//! first tile, Cancel at any tile, runs that are identical run to run, and the shapes of the raw
//! outputs (a 320 x 320 mask, 2x and 4x pictures, a same-size denoise).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    missing_docs
)]

mod support;

use curvyo_image_core::WorkingImage;
use curvyo_imageai_core::{
    AiError, Factor, MASK_SIDE, MODEL_HOST, MODELS, Model, ModelFile, ModelKind, denoise, model,
    remove_background, upscale, upscaled_size, verify,
};
use sha2::{Digest, Sha256};
use support::onnx;

fn pin(kind: ModelKind, bytes: &[u8]) -> ModelFile {
    ModelFile {
        kind,
        name: "tester-net",
        version: "t",
        sha256: Sha256::digest(bytes).into(),
        bytes: bytes.len() as u64,
        placeholder: false,
        licence: "MIT",
        copyright: "",
        upstream: "",
        training_data: "",
    }
}

fn picture(width: u32, height: u32) -> WorkingImage {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    let (cx, cy, r) = (
        i64::from(width / 2),
        i64::from(height / 2),
        i64::from(width.min(height) / 3),
    );
    for y in 0..height {
        for x in 0..width {
            let dark = (i64::from(x) - cx).pow(2) + (i64::from(y) - cy).pow(2) < r * r;
            let g = if dark {
                30
            } else {
                (200 + (x * 7 + y * 3) % 50) as u8
            };
            pixels.extend_from_slice(&[g, g.saturating_sub(5), g, 255]);
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

/// Criterion 64: a file that is not exactly the pinned one is never parsed or run.
#[test]
fn c64_only_the_exact_pinned_bytes_load() {
    let net = onnx::identity_net();
    let file = pin(ModelKind::Denoise, &net);
    assert!(verify(&file, &net).is_ok());
    assert!(Model::load(&file, &net).is_ok());
    let mut flipped = net.clone();
    let mid = flipped.len() / 2;
    flipped[mid] ^= 0x01;
    assert_eq!(verify(&file, &flipped), Err(AiError::WrongModel));
    assert_eq!(
        Model::load(&file, &flipped).unwrap_err(),
        AiError::WrongModel
    );
    // A partial file (an interrupted download) is never run.
    let partial = &net[..net.len() - 1];
    assert_eq!(
        Model::load(&file, partial).unwrap_err(),
        AiError::WrongModel
    );
    assert_eq!(Model::load(&file, &[]).unwrap_err(), AiError::WrongModel);
    let mut longer = net.clone();
    longer.push(0);
    assert_eq!(
        Model::load(&file, &longer).unwrap_err(),
        AiError::WrongModel
    );
}

/// The placeholder gate (plan "Model files"): a placeholder pin never loads, whatever the bytes,
/// even bytes of exactly the pinned size whose hash would be all zeros if it could.
#[test]
fn c64_a_placeholder_pin_never_loads_any_bytes() {
    for file in &MODELS {
        let same_size = vec![0_u8; usize::try_from(file.bytes).unwrap().min(8_000_000)];
        if same_size.len() as u64 == file.bytes {
            assert_eq!(
                verify(file, &same_size),
                Err(AiError::WrongModel),
                "{}",
                file.name
            );
        }
        assert_eq!(verify(file, &[]), Err(AiError::WrongModel));
        let mut forged = *file;
        forged.placeholder = true;
        let net = onnx::identity_net();
        forged.sha256 = Sha256::digest(&net).into();
        forged.bytes = net.len() as u64;
        assert_eq!(
            verify(&forged, &net),
            Err(AiError::WrongModel),
            "placeholder = true refuses even matching bytes"
        );
    }
}

/// Criteria 50 and 63: three models, permissive licences, the size targets, one fixed URL on the
/// one host, no query, no fragment, no user part.
#[test]
fn c50_c63_the_pinned_list_and_its_one_url_per_model() {
    assert_eq!(MODEL_HOST, "https://models.curvyo.org");
    let kinds: Vec<ModelKind> = MODELS.iter().map(|m| m.kind).collect();
    assert_eq!(kinds.len(), 3);
    for kind in [
        ModelKind::Background,
        ModelKind::Upscale,
        ModelKind::Denoise,
    ] {
        assert_eq!(model(kind).kind, kind);
        assert_eq!(kinds.iter().filter(|k| **k == kind).count(), 1);
    }
    for m in &MODELS {
        assert!(
            [
                "Apache-2.0",
                "MIT",
                "BSD-3-Clause",
                "BSD-2-Clause",
                "CC0-1.0"
            ]
            .contains(&m.licence),
            "{}: {}",
            m.name,
            m.licence
        );
        let limit = match m.kind {
            ModelKind::Background => 50_000_000,
            ModelKind::Upscale | ModelKind::Denoise => 100_000_000,
        };
        assert!(m.bytes < limit, "{} is {} bytes", m.name, m.bytes);
        let url = m.url();
        assert_eq!(
            url,
            format!(
                "https://models.curvyo.org/{}/{}/{}.onnx",
                m.name,
                m.version,
                m.sha256_hex()
            )
        );
        assert!(!url.contains('?') && !url.contains('#') && !url.contains('@'));
        assert_eq!(m.sha256_hex().len(), 64);
        assert!(m.licence_url().starts_with("https://models.curvyo.org/"));
        assert!(
            !m.training_data.is_empty(),
            "Frage 281: training data named"
        );
    }
}

/// Criteria 47 and 66: an Upscale over the working limit is refused before the first tile runs.
#[test]
fn c47_upscale_over_the_working_limit_is_refused_before_any_tile() {
    let net = onnx::nearest_x4_net();
    let m = Model::load(&pin(ModelKind::Upscale, &net), &net).unwrap();
    // 4096 x 4096 x2 = 8192 x 8192 = 67 Mpx: over the 16 Mpx limit.
    assert!(matches!(
        upscaled_size(4096, 4096, Factor::Two),
        Err(AiError::TooLarge { .. })
    ));
    assert!(matches!(
        upscaled_size(2001, 2000, Factor::Two),
        Err(AiError::TooLarge { .. })
    ));
    assert_eq!(upscaled_size(2000, 2000, Factor::Two), Ok((4000, 4000)));
    assert_eq!(upscaled_size(1000, 1000, Factor::Four), Ok((4000, 4000)));
    assert!(matches!(
        upscaled_size(1001, 1000, Factor::Four),
        Err(AiError::TooLarge { .. })
    ));
    // A long thin picture passes 8192 px on a side.
    assert!(matches!(
        upscaled_size(2100, 10, Factor::Four),
        Err(AiError::TooLarge { .. })
    ));
    let big = picture(1100, 1000);
    let mut polled = 0;
    let err = upscale(&m, &big, Factor::Four, &mut |_, _| {
        polled += 1;
        true
    })
    .unwrap_err();
    assert!(
        matches!(
            err,
            AiError::TooLarge {
                width: 4400,
                height: 4000
            }
        ),
        "{err:?}"
    );
    assert_eq!(polled, 0, "refused before any work");
}

/// Criterion 47: Cancel stops a Run at any tile; nothing is returned.
#[test]
fn c47_cancel_at_the_first_and_at_a_later_tile() {
    let net = onnx::identity_net();
    let m = Model::load(&pin(ModelKind::Denoise, &net), &net).unwrap();
    let pic = picture(600, 300);
    assert_eq!(
        denoise(&m, &pic, &mut |_, _| false).unwrap_err(),
        AiError::Cancelled
    );
    let mut calls = 0;
    let err = denoise(&m, &pic, &mut |done, total| {
        calls += 1;
        assert!(done < total, "{done} of {total}");
        done < 1
    })
    .unwrap_err();
    assert_eq!(err, AiError::Cancelled);
    assert!(calls >= 2);
    let bg = onnx::channel_mean_net();
    let m = Model::load(&pin(ModelKind::Background, &bg), &bg).unwrap();
    assert_eq!(
        remove_background(&m, &pic, &mut |_, _| false).unwrap_err(),
        AiError::Cancelled
    );
}

/// Criterion 48: the raw outputs and their shapes; a second run gives the same bytes.
#[test]
fn c48_raw_outputs_have_their_shapes_and_repeat_exactly() {
    let pic = picture(300, 170);
    let id = onnx::identity_net();
    let m = Model::load(&pin(ModelKind::Denoise, &id), &id).unwrap();
    let a = denoise(&m, &pic, &mut |_, _| true).unwrap();
    let b = denoise(&m, &pic, &mut |_, _| true).unwrap();
    assert_eq!((a.width, a.height), (300, 170));
    assert_eq!(a.pixels, b.pixels);
    // The identity model returns the picture (opaque, over white) after the tiled run.
    assert_eq!(a.pixels.len(), pic.pixels.len());
    let differ = a
        .pixels
        .iter()
        .zip(&pic.pixels)
        .filter(|(x, y)| x.abs_diff(**y) > 1)
        .count();
    assert_eq!(
        differ, 0,
        "the tiles stitch the identity back without seams"
    );

    let x4 = onnx::nearest_x4_net();
    let m = Model::load(&pin(ModelKind::Upscale, &x4), &x4).unwrap();
    let four = upscale(&m, &pic, Factor::Four, &mut |_, _| true).unwrap();
    assert_eq!((four.width, four.height), (1200, 680));
    let two = upscale(&m, &pic, Factor::Two, &mut |_, _| true).unwrap();
    assert_eq!((two.width, two.height), (600, 340));
    assert_eq!(
        two.pixels,
        upscale(&m, &pic, Factor::Two, &mut |_, _| true)
            .unwrap()
            .pixels
    );

    let bg = onnx::channel_mean_net();
    let m = Model::load(&pin(ModelKind::Background, &bg), &bg).unwrap();
    let mask = remove_background(&m, &pic, &mut |_, _| true).unwrap();
    assert_eq!((mask.width, mask.height), (MASK_SIDE, MASK_SIDE));
    assert_eq!(mask.grey.len(), (MASK_SIDE * MASK_SIDE) as usize);
    assert_eq!(
        mask.grey,
        remove_background(&m, &pic, &mut |_, _| true).unwrap().grey
    );
}

/// A model of another kind is refused (no mask from the upscaler).
#[test]
fn c47_each_run_takes_only_its_own_kind_of_model() {
    let pic = picture(64, 64);
    let id = onnx::identity_net();
    let denoiser = Model::load(&pin(ModelKind::Denoise, &id), &id).unwrap();
    assert!(matches!(
        upscale(&denoiser, &pic, Factor::Two, &mut |_, _| true),
        Err(AiError::BadModel(_))
    ));
    assert!(matches!(
        remove_background(&denoiser, &pic, &mut |_, _| true),
        Err(AiError::BadModel(_))
    ));
    let empty = picture(0, 0);
    assert!(matches!(
        denoise(&denoiser, &empty, &mut |_, _| true),
        Err(AiError::Invalid(_))
    ));
}
