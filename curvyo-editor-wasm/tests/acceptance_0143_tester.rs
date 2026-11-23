//! Tester acceptance cases for `specs/0143-ai-tile-pool` in the AI worker's calls, written from
//! the specification and ADR 0078 §2, §3: each worker is its own instance (here its own thread,
//! so its own loaded model, input and kept tiles); tiles run on N workers in any order and handed
//! to worker 0 assemble to exactly the picture of the one-worker `run_model` (criteria 5, 13);
//! worker 0's assembly refuses a missing or doubled tile and forgets its tiles after each
//! assembly (criterion 17); a worker without the model or the input refuses a tile instead of
//! guessing.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_possible_truncation,
    missing_docs
)]

#[path = "../../curvyo-imageai-core/tests/support/onnx.rs"]
mod onnx;

use curvyo_editor_wasm::ai_glue;
use curvyo_imageai_core::{ModelFile, ModelKind, TileOutput, tile_count};
use sha2::{Digest, Sha256};

fn pin(kind: ModelKind, net: &[u8]) -> ModelFile {
    ModelFile {
        kind,
        name: "tester-net",
        version: "t",
        sha256: Sha256::digest(net).into(),
        bytes: net.len() as u64,
        placeholder: false,
        licence: "MIT",
        copyright: "",
        upstream: "",
        training_data: "",
    }
}

/// This thread's worker loads the Upscale and AI denoise stand-ins.
fn load_nets() {
    for (kind, net) in [
        (ModelKind::Upscale, onnx::blur_x4_net()),
        (ModelKind::Denoise, onnx::blur_net()),
    ] {
        assert_eq!(ai_glue::load_pinned(&pin(kind, &net), &net), "");
    }
}

fn rgba(w: u32, h: u32) -> Vec<u8> {
    (0..w * h)
        .flat_map(|i| {
            let (x, y) = (i % w, i / w);
            [
                ((x * 13 + y * 5) % 251) as u8,
                ((x ^ (y * 3)) % 256) as u8,
                ((x * y) % 241) as u8,
                if x % 23 == 0 { 64 } else { 255 },
            ]
        })
        .collect()
}

/// The pool: `n` worker threads, each loads the model, keeps the input and computes its share
/// of the tiles (dealt from a reversed then interleaved order); worker 0 (this thread) receives
/// every tile in the order the workers return them and assembles.
fn pool(key: &str, factor: u8, (w, h): (u32, u32), n: usize) -> Vec<u8> {
    let count = tile_count(w, h);
    let mut shares = vec![Vec::new(); n];
    for (k, index) in (0..count).rev().enumerate() {
        shares[(k * 7 + 3) % n].push(index);
    }
    let input = rgba(w, h);
    let key = key.to_owned();
    let tiles: Vec<TileOutput> = std::thread::scope(|scope| {
        let handles: Vec<_> = shares
            .into_iter()
            .map(|share| {
                let copy = input.clone();
                let key = key.clone();
                scope.spawn(move || {
                    load_nets();
                    ai_glue::set_tile_input(copy, (w, h));
                    share
                        .into_iter()
                        .map(|i| ai_glue::run_tile(&key, factor, i).unwrap())
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
    // Worker 0 holds only the tiles it was handed (it never loaded a model on this thread here).
    ai_glue::set_tile_input(input, (w, h));
    for tile in tiles {
        ai_glue::add_tile(tile);
    }
    let f = if key == "upscale" {
        u32::from(factor)
    } else {
        1
    };
    ai_glue::assemble_tiles(w * f, h * f).unwrap()
}

/// The one-worker path of today on its own thread (its own instance).
fn one(key: &str, factor: u8, (w, h): (u32, u32)) -> Vec<u8> {
    let key = key.to_owned();
    std::thread::spawn(move || {
        load_nets();
        let mut calls = 0;
        let out = ai_glue::run_model(&key, rgba(w, h), (w, h), factor, &mut |_, _| {
            calls += 1;
            true
        })
        .unwrap();
        assert!(calls >= 1);
        out.pixels
    })
    .join()
    .unwrap()
}

/// Criteria 5 and 13: N = 1, 3 and 8 separate workers give the one-worker picture byte for byte,
/// for AI denoise and Upscale x2 and x4.
#[test]
fn c5_c13_tiles_on_separate_workers_equal_the_one_worker_run() {
    let size = (470, 230);
    for (key, factor) in [("denoise", 2), ("upscale", 2), ("upscale", 4)] {
        let expected = one(key, factor, size);
        for n in [1, 3, 8] {
            let got = pool(key, factor, size, n);
            assert_eq!(got.len(), expected.len(), "{key} x{factor} N = {n}");
            assert!(got == expected, "{key} x{factor} N = {n}: pixels differ");
        }
    }
}

/// Criterion 17 on worker 0: a missing or doubled tile is refused; the kept tiles are forgotten
/// after each assembly, so a second assembly does not reuse the first Run's tiles, and a new
/// input forgets tiles kept before it.
#[test]
fn c17_worker_zero_refuses_a_wrong_set_and_forgets_its_tiles() {
    load_nets();
    let (w, h) = (300, 230);
    ai_glue::set_tile_input(rgba(w, h), (w, h));
    let tiles: Vec<TileOutput> = (0..tile_count(w, h))
        .map(|i| ai_glue::run_tile("denoise", 1, i).unwrap())
        .collect();
    assert_eq!(tiles.len(), 4);

    for t in &tiles[1..] {
        ai_glue::add_tile(t.clone());
    }
    assert!(
        ai_glue::assemble_tiles(w, h).is_err(),
        "tile 0 missing is refused"
    );
    // The refused set is gone: a full set alone now assembles.
    for t in &tiles {
        ai_glue::add_tile(t.clone());
    }
    let first = ai_glue::assemble_tiles(w, h).unwrap();
    assert_eq!(first.len(), (w * h * 4) as usize);
    assert!(
        ai_glue::assemble_tiles(w, h).is_err(),
        "nothing kept after an assembly"
    );

    for t in &tiles {
        ai_glue::add_tile(t.clone());
    }
    ai_glue::add_tile(tiles[2].clone());
    assert!(ai_glue::assemble_tiles(w, h).is_err(), "a doubled tile");

    for t in &tiles[..2] {
        ai_glue::add_tile(t.clone());
    }
    ai_glue::set_tile_input(rgba(w, h), (w, h));
    for t in &tiles {
        ai_glue::add_tile(t.clone());
    }
    assert_eq!(
        ai_glue::assemble_tiles(w, h).unwrap(),
        first,
        "a new input forgot the tiles kept before it"
    );
}

/// A worker refuses a tile it cannot compute: no model loaded, no input, an index past the end,
/// and Remove background (not tiled).
#[test]
fn c3_c17_a_worker_refuses_a_tile_it_cannot_compute() {
    std::thread::spawn(|| {
        ai_glue::set_tile_input(rgba(10, 10), (10, 10));
        assert!(
            ai_glue::run_tile("denoise", 1, 0).is_err(),
            "no model loaded"
        );
    })
    .join()
    .unwrap();
    std::thread::spawn(|| {
        load_nets();
        assert!(ai_glue::run_tile("denoise", 1, 0).is_err(), "no input");
        ai_glue::set_tile_input(rgba(10, 10), (10, 10));
        assert!(ai_glue::run_tile("denoise", 1, 0).is_ok());
        assert!(ai_glue::run_tile("denoise", 1, 1).is_err(), "past the end");
        assert!(ai_glue::run_tile("background", 1, 0).is_err());
        assert!(ai_glue::run_tile("nonsense", 1, 0).is_err());
    })
    .join()
    .unwrap();
}
