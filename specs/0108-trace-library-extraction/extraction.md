# 0108 slice 5: extraction rehearsal (2026-11-10)

Rehearsal: `vectrace/` copied to a scratch directory, its `Cargo.toml` made standalone (explicit
versions instead of `workspace = true`, the lints inline, the repo's `rust-toolchain.toml`, an own
`[workspace]`), the `curvyo-*` dev-dependencies removed.

| Check | Result |
|---|---|
| `cargo build --lib` (native) | passes |
| `cargo build --lib --target wasm32-unknown-unknown` | passes |
| `cargo clippy --lib -D warnings`, native and wasm32 | passes |
| `cargo test --lib` | did not compile (2 blockers below); done since, see "Done" |
| integration tests (`tests/*.rs`) | not run then; done since, see "Done" |

So the library itself has no Curvyo dependency left. What kept the test suite from standing alone (blockers 1 to 4 are done, 2026-11-11):

1. `src/units.rs` test compares `PixelMap` bit for bit with `curvyo-document-core::Affine`.
   Fix before the move: freeze the expected bit patterns of the five fixed matrices as constants
   (generated once from `Affine`) and delete the dev-dependency.
2. `src/ocr/read.rs` test and `tests/{reference,ocr_reference,colour,quality,padding,turned,...}`
   decode PNG/JPEG with `curvyo-image-core::decode_working`. Fix: depend on `image =0.25.10`
   (same pin, `png`/`jpeg`, no default features) in dev-dependencies and call a 15-line local
   decode helper that does what `decode_working` does (check the reference tables stay identical
   first; a decoder difference can change pixels).
3. `tests/quality.rs` and `tests/common/quality/real.rs` read Inkscape baseline SVGs with
   `curvyo-svg-core`. Fix: parse the baselines in the helper with a small path reader, or move
   the quality harness to the new repo's `tools/` and keep the golden tables in the crate.
4. `tests/model_record.rs` reads `frontend/public/models/paddleocr-latin/model.toml`. Fix: copy
   `model.toml` into `vectrace/tests/fixtures/` and keep the weights in Curvyo (bytes are passed in);
   the record test then checks the file it is given.
5. Comments naming `curvyo-render-core` (`shapes/join.rs`) and Curvyo specs: reword when the repo
   exists; not a build matter.

## Done (2026-11-11, on `story/trace-library-extraction`)

`vectrace/Cargo.toml` has no `curvyo-*` dependency of any kind; `cargo tree -p vectrace --edges all`
shows none.

1. `src/units.rs`: the expected bits of the five matrices (compose, determinant, inverse, point and
   vector) are constants, generated once from `Affine`; `curvyo-document-core` is gone.
2. `tests/common/decode.rs` (`image =0.25.10`, `png`, `jpeg`, no default features): orientation
   applied, gray and RGB widened to RGBA, 16-bit rounded to 8 bits as `decode_working` does; the
   OCR unit test in `src/ocr/read.rs` includes the same file. Checked once against
   `decode_working` on all 61 PNG and JPEG fixtures: identical size and bytes, so no reference
   table or golden changed. The helper does not downscale (nothing in the fixtures needs it).
3. `tests/common/baseline_svg.rs` is a small reader (about 120 lines and a test) of the Inkscape form (root `width` and
   `viewBox`, nested `g` transforms, `path d`; the rest ignored). It matches `curvyo-svg-core` on
   three hand-written samples (viewBox, matrix and scale, inches); the real baselines are not in
   the repository (`TRACE_BASELINE_DIR`, the harness is `#[ignore]`), so it was not compared on
   them. A full SVG reader was not reasonable.
4. `tests/fixtures/model.toml` is the record the test reads; the test also checks it equals the
   original next to the weights when that exists.

Still open for the standalone repository: the OCR tests (`tests/{ocr_reference,turned,padding,
model_record}.rs`, `acceptance_0091_tester.rs`, the unit test in `src/ocr/read.rs`, `src/ocr/model.rs`)
still read the weights from `../frontend/public/models/paddleocr-latin/`. The new repository needs
its own copy of the weights at a path (or an environment variable) the tests accept; decide when
the repository is created. Review findings of ADR 0048 (units in `add_shapes`, `wire::Text`,
`Preset`, `Diagnostics`) are done as well.

## Steps for the move (next task, not done here)
1. Fixes 1 to 4 are done (see above); the full gate stays green.
2. `git subtree split -P vectrace -b vectrace-export`; push to the new repository's `main`
   (history of the crate is kept).
3. In the new repo: workspace `Cargo.toml` with `[workspace.lints]` as above, `rust-toolchain.toml`,
   `deny.toml` (MIT, Apache-2.0, BSD, Unicode; ban `rayon` outside `rten*`), `LICENSE-*`.
4. CI (ubuntu): `fmt --check`, `clippy -D warnings`, `nextest run`, wasm32 `build` and `clippy`,
   `deny check`, `doc -D warnings`, native vs wasm result check; nightly Windows and macOS test
   run; release by tag (`vX.Y.Z`) publishing to crates.io after the licence and name check.
5. In this repo: `vectrace = { version = "0.1", ... }` (git or registry) replaces the path
   dependency in `curvyo-ui-core` and `curvyo-editor-wasm`; delete `vectrace/`; ADR 0048 (Accepted
   by then) gets a note naming the repository.
6. Weights: `model.toml` and the ONNX files stay in Curvyo's `frontend/public/models/`; the
   standalone app and the SaaS ship their own copy.
