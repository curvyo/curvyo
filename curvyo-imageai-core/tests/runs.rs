//! The model runs end to end on the pinned runtime with tiny test nets (`specs/0135` criteria 47,
//! 50, 64; ADR 0075 §2, §6): a file is parsed only after its pin matches, the tiles cover the
//! picture once in a fixed order, Cancel stops between tiles, the working limit is checked before
//! any work, and the same input gives the same output.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

mod support;

use curvyo_image_core::WorkingImage;
use curvyo_imageai_core::{
    AiError, Factor, MASK_SIDE, MODELS, Model, ModelFile, ModelKind, denoise, remove_background,
    upscale, verify,
};
use sha2::{Digest, Sha256};
use support::onnx;

fn pin(kind: ModelKind, bytes: &[u8]) -> ModelFile {
    ModelFile {
        kind,
        name: "test-net",
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

fn load(kind: ModelKind, bytes: &[u8]) -> Model {
    Model::load(&pin(kind, bytes), bytes).unwrap()
}

fn picture(width: u32, height: u32) -> WorkingImage {
    WorkingImage {
        width,
        height,
        pixels: (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                [
                    u8::try_from((x * 7) % 256).unwrap(),
                    u8::try_from((y * 5) % 256).unwrap(),
                    u8::try_from((x + y) % 256).unwrap(),
                    255,
                ]
            })
            .collect(),
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

fn go(_: u32, _: u32) -> bool {
    true
}

/// Criterion 64, ADR 0075 §6: only the pinned bytes load; a placeholder never does.
#[test]
fn only_the_pinned_file_loads() {
    let net = onnx::identity_net();
    let file = pin(ModelKind::Denoise, &net);
    assert!(verify(&file, &net).is_ok());
    let mut other = net.clone();
    other[10] ^= 1;
    assert_eq!(verify(&file, &other), Err(AiError::WrongModel));
    assert_eq!(
        verify(&file, &net[..net.len() - 1]),
        Err(AiError::WrongModel)
    );
    assert!(matches!(
        Model::load(&file, &other),
        Err(AiError::WrongModel)
    ));
    for pinned in MODELS {
        assert!(!pinned.placeholder, "{}: pinned", pinned.name);
        assert_eq!(
            verify(&pinned, &net),
            Err(AiError::WrongModel),
            "another file is refused"
        );
        // A placeholder (test-only manifest entry) refuses even bytes of the right size.
        let placeholder = ModelFile {
            sha256: [0; 32],
            placeholder: true,
            ..pinned
        };
        let sized = vec![0_u8; usize::try_from(pinned.bytes).unwrap()];
        assert_eq!(verify(&placeholder, &sized), Err(AiError::WrongModel));
    }
    let not_onnx = b"not a model".to_vec();
    assert!(matches!(
        Model::load(&pin(ModelKind::Denoise, &not_onnx), &not_onnx),
        Err(AiError::BadModel(_))
    ));
}

/// Criterion 47: AI denoise returns the model output at the same size; across tiles (a picture
/// wider than one tile's core) the identity net gives the picture back exactly.
#[test]
fn denoise_runs_in_tiles_and_keeps_the_size() {
    let model = load(ModelKind::Denoise, &onnx::identity_net());
    let image = picture(300, 230);
    let mut calls = Vec::new();
    let out = denoise(&model, &image, &mut |done, total| {
        calls.push((done, total));
        true
    })
    .unwrap();
    assert_eq!(out, image, "the identity net returns the picture");
    assert_eq!(calls.first(), Some(&(0, 4)), "2 x 2 tiles");
    assert_eq!(calls.last(), Some(&(4, 4)));
    assert!(
        calls.windows(2).all(|w| w[0].0 < w[1].0),
        "progress only grows"
    );
    let again = denoise(&model, &image, &mut go).unwrap();
    assert_eq!(out, again, "run-to-run identical");
}

/// Criterion 47, ADR 0075 §2: Upscale x4 by the model, x2 its 2 x 2 box.
#[test]
fn upscale_by_four_and_two() {
    let model = load(ModelKind::Upscale, &onnx::nearest_x4_net());
    let image = picture(40, 30);
    let x4 = upscale(&model, &image, Factor::Four, &mut go).unwrap();
    assert_eq!((x4.width, x4.height), (160, 120));
    assert_eq!(x4.pixel(5, 9), image.pixel(1, 2));
    let x2 = upscale(&model, &image, Factor::Two, &mut go).unwrap();
    assert_eq!((x2.width, x2.height), (80, 60));
    assert_eq!(x2.pixel(3, 5), image.pixel(1, 2));
}

/// Criteria 47, 66: an Upscale over the working limit is refused before any tile runs.
#[test]
fn upscale_over_the_working_limit_is_refused_before_any_work() {
    let model = load(ModelKind::Upscale, &onnx::nearest_x4_net());
    let image = picture(2_400, 1_800);
    let mut polled = false;
    let refused = upscale(&model, &image, Factor::Four, &mut |_, _| {
        polled = true;
        true
    });
    assert_eq!(
        refused,
        Err(AiError::TooLarge {
            width: 9_600,
            height: 7_200
        })
    );
    assert!(!polled, "nothing ran");
}

/// Criterion 47: Cancel between tiles stops the run.
#[test]
fn cancel_stops_between_tiles() {
    let model = load(ModelKind::Denoise, &onnx::identity_net());
    let image = picture(500, 500);
    let mut tiles = 0;
    let out = denoise(&model, &image, &mut |done, _| {
        tiles = done;
        done < 2
    });
    assert_eq!(out, Err(AiError::Cancelled));
    assert_eq!(tiles, 2);
}

/// Criterion 47: Remove background gives the raw 320 x 320 mask, stretched to the whole range:
/// a dark square on white is dark in the mask.
#[test]
fn remove_background_gives_the_raw_mask() {
    let model = load(ModelKind::Background, &onnx::channel_mean_net());
    let mut image = picture(64, 64);
    for (i, p) in image.pixels.chunks_mut(4).enumerate() {
        let (x, y) = (i % 64, i / 64);
        let v = if (16..48).contains(&x) && (16..48).contains(&y) {
            0
        } else {
            255
        };
        p.copy_from_slice(&[v, v, v, 255]);
    }
    let mask = remove_background(&model, &image, &mut go).unwrap();
    assert_eq!((mask.width, mask.height), (MASK_SIDE, MASK_SIDE));
    assert_eq!(mask.at(160, 160), 0, "the dark square");
    assert_eq!(mask.at(5, 5), 255, "the white ground");
}

/// A model of one kind is refused for another step.
#[test]
fn a_model_of_another_kind_is_refused() {
    let model = load(ModelKind::Denoise, &onnx::identity_net());
    assert!(matches!(
        upscale(&model, &picture(8, 8), Factor::Two, &mut go),
        Err(AiError::BadModel(_))
    ));
}
