//! Step 0 of ADR 0075 §7, local only: the three converted models loaded on the pinned `rten`, run
//! on a 1 Mpx picture with their times, and one tile's output written for the comparison against
//! ONNX Runtime (`tools/models/compare_ort.py`). The model files are not in the repository:
//!
//! ```text
//! CURVYO_MODELS_DIR=<folder with u2netp-1/, realesr-animevideov3-1/, ffdnet-color-1/>
//! CURVYO_MODELS_OUT=<folder for the comparison files>
//! cargo test --release -p curvyo-imageai-core --test real_models -- --ignored --nocapture
//! ```

#![allow(
    clippy::unwrap_used,
    clippy::missing_panics_doc,
    clippy::cast_possible_wrap,
    missing_docs
)]

use std::path::{Path, PathBuf};
use std::time::Instant;

use curvyo_image_core::WorkingImage;
use curvyo_imageai_core::{
    Factor, MASK_SIDE, Model, ModelFile, ModelKind, TILE, denoise, remove_background, upscale,
};
use sha2::{Digest, Sha256};

/// The one `.onnx` file in `dir/<name>-1/`.
fn model_file(dir: &Path, name: &str) -> Vec<u8> {
    let folder = dir.join(format!("{name}-1"));
    let file = std::fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|e| e == "onnx"))
        .unwrap();
    std::fs::read(file).unwrap()
}

fn load(kind: ModelKind, bytes: &[u8]) -> Model {
    let pin = ModelFile {
        kind,
        name: "local",
        version: "1",
        sha256: Sha256::digest(bytes).into(),
        bytes: bytes.len() as u64,
        placeholder: false,
        licence: "",
        copyright: "",
        upstream: "",
        training_data: "",
    };
    Model::load(&pin, bytes).unwrap()
}

/// A picture with soft shapes, edges and grain.
fn picture(width: u32, height: u32) -> WorkingImage {
    let mut seed = 7_u32;
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let n = (seed >> 27) as i32 - 16;
            let (dx, dy) = (x as i32 - width as i32 / 2, y as i32 - height as i32 / 2);
            let disc = if dx * dx + dy * dy < (width as i32 / 4).pow(2) {
                60
            } else {
                200
            };
            let g = |o: i32| u8::try_from((disc + o + n).clamp(0, 255)).unwrap();
            pixels.extend([g(x as i32 % 40), g(10), g(y as i32 % 30), 255]);
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

/// The model folder name from `var`, else `default` (the rejected models can be measured again).
fn env_or(var: &str, default: &str) -> String {
    std::env::var(var).unwrap_or_else(|_| default.to_owned())
}

fn out_dir() -> Option<PathBuf> {
    std::env::var_os("CURVYO_MODELS_OUT").map(PathBuf::from)
}

fn write(name: &str, bytes: &[u8]) {
    if let Some(dir) = out_dir() {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(name), bytes).unwrap();
    }
}

fn timed<T>(what: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let out = f();
    println!("{what}: {:.2} s", start.elapsed().as_secs_f64());
    out
}

#[test]
#[ignore = "needs the converted model files (CURVYO_MODELS_DIR)"]
fn the_three_models_run_on_rten_with_their_times() {
    let Some(dir) = std::env::var_os("CURVYO_MODELS_DIR").map(PathBuf::from) else {
        return;
    };
    let mpx = picture(1_000, 1_000);
    let tile = picture(TILE, TILE);
    let mask_in = picture(MASK_SIDE, MASK_SIDE);
    let go = &mut |_, _| true;

    let bytes = model_file(&dir, "u2netp");
    let m = timed("u2netp load", || load(ModelKind::Background, &bytes));
    let mask = timed("u2netp 1 Mpx", || remove_background(&m, &mpx, go).unwrap());
    assert_eq!((mask.width, mask.height), (MASK_SIDE, MASK_SIDE));
    write("mask_in.rgba", &mask_in.pixels);
    write(
        "mask_out.grey",
        &remove_background(&m, &mask_in, go).unwrap().grey,
    );

    let bytes = model_file(
        &dir,
        &env_or("CURVYO_UPSCALE_MODEL", "realesr-animevideov3"),
    );
    let m = timed("upscale load", || load(ModelKind::Upscale, &bytes));
    let up = timed("upscale x4 1 Mpx (25 tiles of 256)", || {
        upscale(&m, &mpx, Factor::Four, go).unwrap()
    });
    assert_eq!((up.width, up.height), (4_000, 4_000));
    write("tile_in.rgba", &tile.pixels);
    // One tile of 224 px core: the 256 px input with 16 px of edge repeated around it.
    let one = picture(224, 224);
    write("up_in.rgba", &one.pixels);
    write(
        "up_out.rgba",
        &upscale(&m, &one, Factor::Four, go).unwrap().pixels,
    );

    let bytes = model_file(&dir, &env_or("CURVYO_DENOISE_MODEL", "ffdnet-color"));
    let m = timed("denoise load", || load(ModelKind::Denoise, &bytes));
    let dn = timed("denoise 1 Mpx (25 tiles of 256)", || {
        denoise(&m, &mpx, go).unwrap()
    });
    assert_eq!((dn.width, dn.height), (1_000, 1_000));
    write("dn_in.rgba", &one.pixels);
    write("dn_out.rgba", &denoise(&m, &one, go).unwrap().pixels);
}
