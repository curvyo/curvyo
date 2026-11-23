//! Tester acceptance cases for `specs/0143-ai-tile-pool` in the model runtime, written from the
//! specification and ADR 0078 §2 before the implementation was read: the tiles of Upscale (x2 and
//! x4) and AI denoise, computed by N separate model instances (one per thread, as one per worker)
//! in any order, assemble pixel-identical to the one-worker run for N = 1, 3 and 8 and for edge
//! picture sizes (criteria 5, 13); `tile_count` names exactly the valid indices (25 for the 1 Mpx
//! reference picture); `assemble` refuses a missing, doubled, out-of-range or wrongly sized tile
//! and an empty set with `AiError::Invalid` (criterion 17).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    missing_docs
)]

mod support;

use curvyo_image_core::WorkingImage;
use curvyo_imageai_core::{
    AiError, Factor, Model, ModelFile, ModelKind, TileOutput, assemble, denoise, denoise_tile,
    tile_count, upscale, upscale_tile,
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

fn net(kind: ModelKind) -> Vec<u8> {
    match kind {
        ModelKind::Upscale => onnx::blur_x4_net(),
        _ => onnx::blur_net(),
    }
}

/// A fresh instance of the stand-in model (each worker loads its own copy).
fn instance(kind: ModelKind) -> Model {
    let bytes = net(kind);
    Model::load(&pin(kind, &bytes), &bytes).unwrap()
}

/// A noisy picture with a hard edge and a soft alpha band, different in each channel.
fn photo(width: u32, height: u32, seed: u32) -> WorkingImage {
    let mut s = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            let edge = if x * 3 > width + y { 200 } else { 40 };
            pixels.extend_from_slice(&[
                (edge + (s & 31)) as u8,
                ((x * 11 + y * 7) % 256) as u8,
                ((s >> 8) & 255) as u8,
                if y % 17 < 3 { 90 } else { 255 },
            ]);
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

/// A seeded permutation of `0..n`.
fn permutation(n: u32, seed: u64) -> Vec<u32> {
    let mut order: Vec<u32> = (0..n).collect();
    let mut s = seed ^ 0x9e37_79b9_7f4a_7c15;
    for i in (1..order.len()).rev() {
        s ^= s << 7;
        s ^= s >> 9;
        let j = (s % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    order
}

#[derive(Clone, Copy)]
enum Step {
    Denoise,
    Up(Factor),
}

impl Step {
    fn kind(self) -> ModelKind {
        match self {
            Self::Denoise => ModelKind::Denoise,
            Self::Up(_) => ModelKind::Upscale,
        }
    }
    fn times(self) -> u32 {
        match self {
            Self::Denoise => 1,
            Self::Up(f) => f.times(),
        }
    }
    fn whole(self, m: &Model, image: &WorkingImage) -> WorkingImage {
        match self {
            Self::Denoise => denoise(m, image, &mut |_, _| true).unwrap(),
            Self::Up(f) => upscale(m, image, f, &mut |_, _| true).unwrap(),
        }
    }
    fn tile(self, m: &Model, image: &WorkingImage, index: u32) -> Result<TileOutput, AiError> {
        match self {
            Self::Denoise => denoise_tile(m, image, index),
            Self::Up(f) => upscale_tile(m, image, f, index),
        }
    }
}

/// The pool as the coordinator runs it: `n` workers, each its own thread with its own model
/// instance and its own copy of the picture, take tile indices from `order` round-robin and
/// compute them in that order; the tiles reach the assembler in an order the threads decide,
/// then shuffled once more.
fn pool(step: Step, image: &WorkingImage, n: usize, seed: u64) -> WorkingImage {
    let count = tile_count(image.width, image.height);
    let order = permutation(count, seed);
    let mut shares: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (k, index) in order.into_iter().enumerate() {
        shares[k % n].push(index);
    }
    let mut tiles: Vec<TileOutput> = std::thread::scope(|scope| {
        let handles: Vec<_> = shares
            .into_iter()
            .map(|share| {
                let copy = image.clone();
                scope.spawn(move || {
                    let model = instance(step.kind());
                    share
                        .into_iter()
                        .map(|i| step.tile(&model, &copy, i).unwrap())
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles
            .into_iter()
            .rev()
            .flat_map(|h| h.join().unwrap())
            .collect()
    });
    let mix = permutation(tiles.len() as u32, seed.wrapping_add(99));
    let mut shuffled: Vec<Option<TileOutput>> = tiles.drain(..).map(Some).collect();
    let ordered: Vec<TileOutput> = mix
        .iter()
        .map(|&i| shuffled[i as usize].take().unwrap())
        .collect();
    assemble(
        image.width * step.times(),
        image.height * step.times(),
        ordered,
    )
    .unwrap()
}

/// Criteria 5 and 13: for N = 1, 3 and 8 separate instances and any finishing order the
/// assembled picture equals the one-worker run byte for byte, for AI denoise and Upscale x2/x4.
#[test]
fn c5_c13_any_pool_size_and_order_is_pixel_identical() {
    let image = photo(470, 300, 3);
    assert_eq!(tile_count(470, 300), 6);
    for step in [Step::Denoise, Step::Up(Factor::Four), Step::Up(Factor::Two)] {
        let one = step.whole(&instance(step.kind()), &image);
        assert_eq!(
            (one.width, one.height),
            (470 * step.times(), 300 * step.times())
        );
        for (n, seed) in [(1, 1), (3, 2), (8, 3), (3, 77)] {
            let got = pool(step, &image, n, seed);
            assert_eq!(got.width, one.width);
            assert_eq!(got.height, one.height);
            assert!(
                got.pixels == one.pixels,
                "N = {n}, seed {seed}: the pool's picture differs from one worker's"
            );
        }
    }
}

/// Criterion 5 on the edges of the grid: one pixel, exactly one core, one past a core, a thin
/// strip, and a picture whose last row and column of tiles are one pixel wide.
#[test]
fn c5_edge_sizes_assemble_to_the_whole_run() {
    for (w, h, tiles) in [
        (1, 1, 1),
        (224, 224, 1),
        (225, 224, 2),
        (3, 700, 4),
        (449, 225, 6),
    ] {
        let image = photo(w, h, w + h);
        assert_eq!(tile_count(w, h), tiles, "{w} x {h}");
        for step in [Step::Denoise, Step::Up(Factor::Two)] {
            let one = step.whole(&instance(step.kind()), &image);
            let got = pool(step, &image, 3, u64::from(w * 31 + h));
            assert!(got == one, "{w} x {h}: the pool differs from one worker");
        }
    }
}

/// The spec's reference picture has 25 tiles (spec "Words"), and `tile_count` names exactly the
/// valid indices: the last is computed, the next is refused before any model work.
#[test]
fn c5_c17_tile_count_names_exactly_the_valid_indices() {
    assert_eq!(tile_count(1000, 1000), 25);
    let image = photo(300, 230, 9);
    let n = tile_count(300, 230);
    assert_eq!(n, 4);
    let m = instance(ModelKind::Denoise);
    assert!(denoise_tile(&m, &image, n - 1).is_ok());
    for bad in [n, n + 1, u32::MAX] {
        assert!(
            matches!(denoise_tile(&m, &image, bad), Err(AiError::Invalid(_))),
            "index {bad}"
        );
    }
    let up = instance(ModelKind::Upscale);
    assert!(matches!(
        upscale_tile(&up, &image, Factor::Two, n),
        Err(AiError::Invalid(_))
    ));
}

/// The tiles of one run computed twice by two instances are equal (ADR 0078 §6, run to run).
#[test]
fn c5_a_tile_is_the_same_from_any_instance() {
    let image = photo(300, 230, 4);
    let (a, b) = (instance(ModelKind::Upscale), instance(ModelKind::Upscale));
    for i in 0..tile_count(300, 230) {
        let ta = upscale_tile(&a, &image, Factor::Four, i).unwrap();
        let tb = upscale_tile(&b, &image, Factor::Four, i).unwrap();
        assert_eq!(ta, tb, "tile {i}");
        assert_eq!(ta.index, i);
        assert_eq!(ta.rgba.len(), (ta.width * ta.height * 4) as usize);
    }
}

fn denoise_tiles(image: &WorkingImage) -> Vec<TileOutput> {
    let m = instance(ModelKind::Denoise);
    (0..tile_count(image.width, image.height))
        .map(|i| denoise_tile(&m, image, i).unwrap())
        .collect()
}

fn refused(w: u32, h: u32, tiles: Vec<TileOutput>, what: &str) {
    match assemble(w, h, tiles) {
        Err(AiError::Invalid(_)) => {}
        other => panic!("{what}: expected AiError::Invalid, got {other:?}"),
    }
}

/// Criterion 17: `assemble` refuses every wrong tile set with `AiError::Invalid`, never a picture.
#[test]
fn c17_assemble_refuses_every_wrong_tile_set() {
    let image = photo(449, 230, 5);
    let (w, h) = (449, 230);
    let all = denoise_tiles(&image);
    assert_eq!(all.len(), 6);
    assert!(assemble(w, h, all.clone()).is_ok());

    refused(w, h, Vec::new(), "no tile at all");
    for i in 0..all.len() {
        let mut missing = all.clone();
        missing.remove(i);
        refused(w, h, missing, &format!("tile {i} missing"));
    }
    let mut doubled = all.clone();
    doubled.push(all[4].clone());
    refused(w, h, doubled, "tile 4 twice (and all others present)");
    let mut out_of_range = all.clone();
    let mut beyond = all[5].clone();
    beyond.index = 6;
    out_of_range.push(beyond);
    refused(w, h, out_of_range, "index 6 of 6");
    // The two one-pixel-wide right-hand cores swapped in size with a full core.
    let mut swapped = all.clone();
    let (a, b) = (swapped[0].clone(), swapped[2].clone());
    swapped[0].width = b.width;
    swapped[0].height = b.height;
    swapped[0].rgba = b.rgba.clone();
    swapped[2].width = a.width;
    swapped[2].height = a.height;
    swapped[2].rgba = a.rgba;
    refused(w, h, swapped, "tiles 0 and 2 with each other's size");
    let mut taller = all.clone();
    taller[4].height += 1;
    let extra = taller[4].width as usize * 4;
    taller[4].rgba.extend(std::iter::repeat_n(0, extra));
    refused(w, h, taller, "a core one row too tall");
    let mut long = all.clone();
    long[1].rgba.push(0);
    refused(w, h, long, "one byte too many");
    refused(w + 1, h, all.clone(), "an output one pixel wider");
    refused(w, h - 1, all.clone(), "an output one pixel lower");
    refused(w * 2, h * 2, all, "a denoise set assembled as x2");
}

/// Criterion 17 for Upscale: an x4 tile set is not an x2 picture, and x2 tiles mixed with x4
/// tiles are refused.
#[test]
fn c17_assemble_refuses_tiles_of_another_factor() {
    let image = photo(300, 230, 6);
    let up = instance(ModelKind::Upscale);
    let n = tile_count(300, 230);
    let x4: Vec<_> = (0..n)
        .map(|i| upscale_tile(&up, &image, Factor::Four, i).unwrap())
        .collect();
    let x2: Vec<_> = (0..n)
        .map(|i| upscale_tile(&up, &image, Factor::Two, i).unwrap())
        .collect();
    assert!(assemble(1200, 920, x4.clone()).is_ok());
    assert!(assemble(600, 460, x2.clone()).is_ok());
    refused(600, 460, x4.clone(), "x4 tiles as an x2 picture");
    let mut mixed = x2;
    mixed[1] = x4[1].clone();
    refused(600, 460, mixed, "one x4 tile among x2 tiles");
}
