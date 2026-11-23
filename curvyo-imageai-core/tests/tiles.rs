//! The tiles of a tiled model run on their own (`specs/0143` criteria 5, 13, 17; ADR 0078 §2): a
//! run tile by tile in any order through `assemble` is pixel-identical to `upscale` and `denoise`,
//! and to the output before the tile functions existed (pinned by hash); `assemble` refuses a
//! missing, doubled or wrongly sized tile.

#![allow(clippy::unwrap_used, clippy::missing_panics_doc, missing_docs)]

mod support;

use curvyo_image_core::WorkingImage;
use curvyo_imageai_core::{
    AiError, Factor, Model, ModelFile, ModelKind, TileOutput, assemble, denoise, denoise_tile,
    tile_count, upscale, upscale_tile,
};
use sha2::{Digest, Sha256};
use support::onnx;

fn load(kind: ModelKind, bytes: &[u8]) -> Model {
    let file = ModelFile {
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
    };
    Model::load(&file, bytes).unwrap()
}

/// A picture with detail in every channel and a soft alpha edge, `width` x `height`.
fn picture(width: u32, height: u32) -> WorkingImage {
    WorkingImage {
        width,
        height,
        pixels: (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .flat_map(|(x, y)| {
                [
                    u8::try_from((x * 7 + y * 3) % 256).unwrap(),
                    u8::try_from((y * 5 + x * x / 9) % 256).unwrap(),
                    u8::try_from((x ^ y) % 256).unwrap(),
                    if x < 3 { 128 } else { 255 },
                ]
            })
            .collect(),
        original_size: (width, height),
        truncated: false,
        animated: false,
    }
}

fn hash(image: &WorkingImage) -> String {
    use std::fmt::Write as _;
    Sha256::digest(&image.pixels)
        .iter()
        .fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

fn go(_: u32, _: u32) -> bool {
    true
}

/// Criteria 5, 13: the output of the single loop is the one of before this story (hashes taken
/// on the code before the tile functions, 2026-11-21).
#[test]
fn the_outputs_are_the_ones_of_before() {
    let image = picture(500, 470);
    let blur = load(ModelKind::Denoise, &onnx::blur_net());
    let up = load(ModelKind::Upscale, &onnx::blur_x4_net());
    let small = picture(230, 250);
    let got = [
        hash(&denoise(&blur, &image, &mut go).unwrap()),
        hash(&upscale(&up, &small, Factor::Four, &mut go).unwrap()),
        hash(&upscale(&up, &small, Factor::Two, &mut go).unwrap()),
    ];
    assert_eq!(
        got,
        [
            "63daafcb7587554d40de5570ffa824920060de25d05bf3f165c0130784e29d10",
            "3ea50269c92acbff423d462011b96593d19c7e90e58ee9401a9f59be61e0ab5c",
            "737ac9a9e7360bcbb6a10fb85e8879f0dbcc0fa1a3200477e6d6fb06eddc1c1f",
        ]
    );
}

/// A fixed shuffle of `0..n` (a different order than the row-major one).
fn shuffled(n: u32) -> Vec<u32> {
    let mut order: Vec<u32> = (0..n).collect();
    let mut s = 7_u64;
    for i in (1..order.len()).rev() {
        s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        let j = usize::try_from(s >> 33).unwrap() % (i + 1);
        order.swap(i, j);
    }
    order
}

/// Criterion 5, ADR 0078 §6: tiles computed one by one in shuffled order and assembled equal
/// the whole run, for AI denoise and for Upscale x4 and x2.
#[test]
fn shuffled_tiles_assemble_to_the_whole_run() {
    let image = picture(500, 470);
    assert_eq!(tile_count(500, 470), 9);
    assert_eq!(tile_count(1000, 1000), 25);
    let blur = load(ModelKind::Denoise, &onnx::blur_net());
    let order = shuffled(tile_count(500, 470));
    assert_ne!(order, (0..9).collect::<Vec<_>>());
    let tiles: Vec<TileOutput> = order
        .iter()
        .map(|&i| denoise_tile(&blur, &image, i).unwrap())
        .collect();
    let assembled = assemble(500, 470, tiles).unwrap();
    assert_eq!(assembled, denoise(&blur, &image, &mut go).unwrap());

    let up = load(ModelKind::Upscale, &onnx::blur_x4_net());
    let small = picture(230, 250);
    for (factor, times) in [(Factor::Four, 4), (Factor::Two, 2)] {
        let n = tile_count(230, 250);
        let tiles: Vec<TileOutput> = shuffled(n)
            .iter()
            .map(|&i| upscale_tile(&up, &small, factor, i).unwrap())
            .collect();
        let assembled = assemble(230 * times, 250 * times, tiles).unwrap();
        assert_eq!(assembled, upscale(&up, &small, factor, &mut go).unwrap());
    }
}

/// ADR 0078 §2: x2 halved inside each tile equals the assembled x4 picture halved by 2 x 2
/// boxes (the cores are multiples of 896 output px, so no box crosses a core).
#[test]
fn upscale_by_two_equals_the_x4_picture_halved() {
    let up = load(ModelKind::Upscale, &onnx::blur_x4_net());
    let small = picture(461, 230);
    let x4 = upscale(&up, &small, Factor::Four, &mut go).unwrap();
    let x2 = upscale(&up, &small, Factor::Two, &mut go).unwrap();
    assert_eq!((x2.width, x2.height), (922, 460));
    for y in 0..x2.height {
        for x in 0..x2.width {
            let at = |dx: u32, dy: u32, c: usize| {
                u32::from(x4.pixels[(((2 * y + dy) * x4.width + 2 * x + dx) * 4) as usize + c])
            };
            for c in 0..3 {
                let mean = (at(0, 0, c) + at(1, 0, c) + at(0, 1, c) + at(1, 1, c) + 2) / 4;
                assert_eq!(
                    u32::from(x2.pixels[((y * x2.width + x) * 4) as usize + c]),
                    mean,
                    "({x}, {y}) channel {c}"
                );
            }
        }
    }
}

/// Criterion 17: `assemble` refuses a missing, doubled or wrongly sized tile, a tile of an index
/// out of range, and a size the tiles do not make up.
#[test]
fn assemble_refuses_a_wrong_tile_set() {
    let blur = load(ModelKind::Denoise, &onnx::blur_net());
    let image = picture(300, 230);
    let all = || -> Vec<TileOutput> {
        (0..tile_count(300, 230))
            .map(|i| denoise_tile(&blur, &image, i).unwrap())
            .collect()
    };
    assert!(assemble(300, 230, all()).is_ok());
    let invalid = |tiles: Vec<TileOutput>, w: u32, h: u32| {
        matches!(assemble(w, h, tiles), Err(AiError::Invalid(_)))
    };
    let mut missing = all();
    missing.remove(2);
    assert!(invalid(missing, 300, 230), "a missing tile");
    let mut first_missing = all();
    first_missing.remove(0);
    assert!(invalid(first_missing, 300, 230), "tile 0 missing");
    let mut doubled = all();
    let copy = doubled[1].clone();
    doubled[2] = copy;
    assert!(invalid(doubled, 300, 230), "a doubled tile");
    let mut extra = all();
    let mut beyond = extra[3].clone();
    beyond.index = 4;
    extra.push(beyond);
    assert!(invalid(extra, 300, 230), "an index out of range");
    let mut sized = all();
    sized[3].width -= 1;
    let keep = sized[3].rgba.len() - sized[3].height as usize * 4;
    sized[3].rgba.truncate(keep);
    assert!(invalid(sized, 300, 230), "a wrongly sized tile");
    let mut short = all();
    short[1].rgba.pop();
    assert!(
        invalid(short, 300, 230),
        "pixels that do not match the size"
    );
    assert!(invalid(all(), 301, 230), "another picture size");
    assert!(invalid(Vec::new(), 300, 230), "no tiles");
    assert!(
        matches!(denoise_tile(&blur, &image, 4), Err(AiError::Invalid(_))),
        "a tile index past the last"
    );
}
