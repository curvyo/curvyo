//! Finds the large test assets (the OCR weights and the customer's reference images) that the
//! crate itself does not carry: `VECTRACE_ASSETS` names a directory with
//! `models/paddleocr-latin/` and `trace-references/`; else `assets/` in the crate directory (the
//! standalone repository) and then the files next to the crate in the Curvyo workspace are used. Without either the test skips with a message, unless
//! `VECTRACE_REQUIRE_ASSETS` is set, then it fails. Tests only.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The directory with `det.onnx`, `rec.onnx` and `dict.txt`, or `None` when the test must skip.
pub fn models() -> Option<PathBuf> {
    find(
        "models/paddleocr-latin",
        "../frontend/public/models/paddleocr-latin",
    )
}

/// The directory of the customer's reference images, or `None` when the test must skip.
pub fn references() -> Option<PathBuf> {
    find("trace-references", "../tests/assets/trace-references")
}

/// The reference image a sidecar's `file` (`tests/assets/trace-references/<name>`) names.
pub fn reference_file(sidecar_file: &str) -> Option<PathBuf> {
    let name = sidecar_file.strip_prefix("tests/assets/trace-references/")?;
    references().map(|dir| dir.join(name))
}

fn find(in_assets: &str, in_workspace: &str) -> Option<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let found = match std::env::var_os("VECTRACE_ASSETS") {
        Some(root) => Some(Path::new(&root).join(in_assets)).filter(|dir| dir.is_dir()),
        None => [manifest.join("assets").join(in_assets), manifest.join(in_workspace)]
            .into_iter()
            .find(|dir| dir.is_dir()),
    };
    if found.is_none() {
        let message =
            format!("{in_assets} not found (set VECTRACE_ASSETS to a directory holding it)");
        assert!(
            std::env::var_os("VECTRACE_REQUIRE_ASSETS").is_none(),
            "{message}"
        );
        eprintln!("skipped: {message}");
    }
    found
}
